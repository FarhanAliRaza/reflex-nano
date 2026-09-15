use crate::server::{now, EventRequest, Reply, Server, Session, Snapshot};
use crate::NativeState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use tokio::sync::{broadcast, mpsc, OwnedSemaphorePermit};

pub(crate) async fn upgrade(
    State(server): State<Arc<Server>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let origin = headers
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if host.is_empty()
        || !(origin == format!("http://{host}") || origin == format!("https://{host}"))
    {
        return (StatusCode::FORBIDDEN, "WebSocket origin rejected").into_response();
    }
    let session = match server.session(&headers, false) {
        Ok((s, _)) => s,
        Err((s, m)) => return (s, m).into_response(),
    };
    let permit = match session.connections.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => {
            return (StatusCode::TOO_MANY_REQUESTS, "Connection limit reached").into_response()
        }
    };
    ws.protocols(["nano.v1"])
        .max_message_size(64 * 1024)
        .max_frame_size(64 * 1024)
        .on_upgrade(move |socket| connection(socket, server, session, permit))
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Incoming {
    Hello {
        csrf: String,
        path: String,
    },
    Event {
        id: String,
        name: String,
        #[serde(default)]
        args: Vec<Value>,
        path: String,
    },
    Navigate {
        path: String,
    },
    Sync,
}
async fn send<T: Serialize>(socket: &mut WebSocket, value: T) -> Result<(), ()> {
    tokio::time::timeout(
        Duration::from_secs(10),
        socket.send(Message::Text(
            serde_json::to_string(&value).map_err(|_| ())?.into(),
        )),
    )
    .await
    .map_err(|_| ())?
    .map_err(|_| ())
}
struct Cursor {
    view: NativeState,
    version: Option<u64>,
    path: String,
}
async fn update(
    socket: &mut WebSocket,
    server: &Arc<Server>,
    cursor: &mut Cursor,
    snapshot: &Snapshot,
    full: bool,
) -> Result<(), ()> {
    if !full && cursor.version.is_some_and(|v| snapshot.version <= v) {
        return Ok(());
    }
    let view = if snapshot.path == cursor.path {
        snapshot.view.clone()
    } else {
        let app = server.app.clone();
        let state = snapshot.state.clone();
        let path = cursor.path.clone();
        tokio::task::spawn_blocking(move || app.native_view(&state, &path))
            .await
            .map_err(|_| ())?
            .map_err(|_| ())?
    };
    #[derive(Serialize)]
    struct Full<'a> {
        #[serde(rename = "type")]
        kind: &'static str,
        state: &'a NativeState,
        version: u64,
        path: &'a str,
    }
    #[derive(Serialize)]
    struct Delta<'a> {
        #[serde(rename = "type")]
        kind: &'static str,
        delta: std::collections::BTreeMap<&'a str, &'a Value>,
        removed: Vec<&'a str>,
        version: u64,
        path: &'a str,
    }
    if full {
        send(
            socket,
            Full {
                kind: "snapshot",
                state: &view,
                version: snapshot.version,
                path: &cursor.path,
            },
        )
        .await?;
    } else {
        let fields = view.fields();
        let delta = fields
            .iter()
            .filter(|(key, _)| !cursor.view.same_field(key, &view))
            .map(|(key, value)| (key.as_str(), value.as_ref()))
            .collect();
        let removed = cursor
            .view
            .fields()
            .keys()
            .filter(|key| !fields.contains_key(*key))
            .map(String::as_str)
            .collect();
        send(
            socket,
            Delta {
                kind: "update",
                delta,
                removed,
                version: snapshot.version,
                path: &cursor.path,
            },
        )
        .await?;
    }
    cursor.view = view;
    cursor.version = Some(snapshot.version);
    Ok(())
}
async fn connection(
    mut socket: WebSocket,
    server: Arc<Server>,
    session: Arc<Session>,
    _permit: OwnedSemaphorePermit,
) {
    let hello = tokio::time::timeout(Duration::from_secs(5), socket.recv()).await;
    let path = match hello {
        Ok(Some(Ok(Message::Text(text)))) => match serde_json::from_str::<Incoming>(&text) {
            Ok(Incoming::Hello { csrf, path }) if csrf == session.csrf => path,
            _ => {
                let _ = send(
                    &mut socket,
                    json!({"type":"error","error":"Authentication failed"}),
                )
                .await;
                return;
            }
        },
        _ => return,
    };
    // Subscribe before taking the snapshot so no intervening commit can be missed.
    let mut updates = session.updates.subscribe();
    let mut notices = session.notices.subscribe();
    let mut cursor = Cursor {
        view: NativeState::default(),
        version: None,
        path,
    };
    let initial = match server.snapshot(&session, &cursor.path).await {
        Ok(s) => s,
        Err(_) => return,
    };
    if update(&mut socket, &server, &mut cursor, &initial, true)
        .await
        .is_err()
    {
        return;
    }
    let (jobs, mut work) = mpsc::channel::<(String, EventRequest)>(64);
    let (replies, mut results) = mpsc::channel::<(String, Reply)>(64);
    let processor = server.clone();
    let event_session = session.clone();
    tokio::spawn(async move {
        while let Some((id, request)) = work.recv().await {
            let reply = processor
                .apply(event_session.clone(), request, id.clone())
                .await;
            // Accepted events finish even if their connection closes; receipts prevent retries duplicating work.
            let _ = replies.send((id, reply)).await;
        }
    });
    let mut heartbeat = tokio::time::interval(Duration::from_secs(20));
    heartbeat.tick().await;
    let mut last_pong = tokio::time::Instant::now();
    loop {
        tokio::select! {
            message=socket.recv()=>match message {
                Some(Ok(Message::Text(text)))=> {
                    session.seen.store(now(),Ordering::Relaxed);
                    match serde_json::from_str::<Incoming>(&text) {
                        Ok(Incoming::Event{id,name,args,path}) if !id.is_empty() && id.len()<=100=> {
                            if jobs.try_send((id.clone(),EventRequest{name,args,path})).is_err() && send(&mut socket,json!({"type":"ack","id":id,"ok":false,"error":"Event queue full"})).await.is_err(){break;}
                        }
                        Ok(Incoming::Navigate{path})=> {
                            match server.snapshot(&session,&path).await {
                                Ok(snapshot)=>{cursor.path=path;if update(&mut socket,&server,&mut cursor,&snapshot,true).await.is_err(){break;}}
                                Err(e)=>{if send(&mut socket,json!({"type":"error","error":e})).await.is_err(){break;}}
                            }
                        }
                        Ok(Incoming::Sync)=>match server.snapshot(&session,&cursor.path).await {
                            Ok(snapshot)=>if update(&mut socket,&server,&mut cursor,&snapshot,true).await.is_err(){break;},Err(_)=>break,
                        }
                        _=>if send(&mut socket,json!({"type":"error","error":"Invalid WebSocket message"})).await.is_err(){break;},
                    }
                }
                Some(Ok(Message::Pong(_)))=>{last_pong=tokio::time::Instant::now();session.seen.store(now(),Ordering::Relaxed);}
                Some(Ok(Message::Ping(data)))=>if socket.send(Message::Pong(data)).await.is_err(){break;},
                Some(Ok(Message::Close(_)))|None|Some(Err(_))=>break,
                _=>break,
            },
            Some((id,reply))=results.recv()=> {
                // Preserve streaming checkpoints queued before completion.
                while let Ok(snapshot)=updates.try_recv() {
                    if update(&mut socket,&server,&mut cursor,&snapshot,false).await.is_err(){return;}
                }
                if let Some(snapshot)=&reply.snapshot {
                    if update(&mut socket,&server,&mut cursor,snapshot,false).await.is_err(){break;}
                }
                let ack=json!({"type":"ack","id":id,"ok":reply.error.is_none(),"error":reply.error,"version":cursor.version,"effects":reply.effects});
                if send(&mut socket,ack).await.is_err(){break;}
            }
            change=updates.recv()=>match change {
                Ok(snapshot)=>if update(&mut socket,&server,&mut cursor,&snapshot,false).await.is_err(){break;},
                Err(broadcast::error::RecvError::Lagged(_))=>match server.snapshot(&session,&cursor.path).await {
                    Ok(snapshot)=>if update(&mut socket,&server,&mut cursor,&snapshot,true).await.is_err(){break;},Err(_)=>break,
                },Err(_)=>break,
            },
            notice=notices.recv()=>if let Ok(value)=notice {if send(&mut socket,value).await.is_err(){break;}},
            _=heartbeat.tick()=> {
                if last_pong.elapsed()>Duration::from_secs(60) || socket.send(Message::Ping(vec![].into())).await.is_err(){break;}
            }
        }
    }
}
