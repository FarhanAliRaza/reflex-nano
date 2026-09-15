use crate::model::{escape_into, safe_url};
use crate::{Expr, NativeState, Node, RenderPlan};
use axum::{
    extract::{DefaultBodyLimit, Query, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
    serve::ListenerExt,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{
    broadcast, Mutex as AsyncMutex, OwnedMutexGuard, OwnedSemaphorePermit, Semaphore,
};
use uuid::Uuid;

pub type EventResult = Result<Vec<Effect>, String>;
type Handler = Arc<dyn Fn(&mut NativeState, &[Value]) -> EventResult + Send + Sync>;
type Computed = Arc<dyn Fn(&NativeState) -> Result<Arc<Value>, String> + Send + Sync>;
type ContextHandler = Arc<dyn Fn(EventContext, &[Value]) -> EventResult + Send + Sync>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Effect {
    Redirect { path: String },
    Alert { message: String },
    Event { name: String, args: Vec<Value> },
}
#[derive(Clone)]
struct Page {
    renderer: crate::Renderer,
    title: String,
    tree: Node,
    plan: Arc<RenderPlan>,
}

#[derive(Clone)]
pub struct App {
    pub renderer: crate::Renderer,
    initial: NativeState,
    pages: BTreeMap<String, Page>,
    handlers: HashMap<String, Handler>,
    context_handlers: HashMap<String, (bool, ContextHandler)>,
    computed: Vec<(String, Computed)>,
    pub session_limit: usize,
    pub session_ttl_secs: u64,
}
impl App {
    pub fn new(initial: Value) -> Self {
        assert!(initial.is_object(), "initial state must be an object");
        Self {
            renderer: crate::Renderer::Html,
            initial: NativeState::from_value(initial).expect("initial state must be an object"),
            pages: BTreeMap::new(),
            handlers: HashMap::new(),
            context_handlers: HashMap::new(),
            computed: vec![],
            session_limit: 5000,
            session_ttl_secs: 3600,
        }
    }
    pub fn add_page(&mut self, route: &str, title: &str, tree: Node) -> Result<(), String> {
        self.add_page_with_renderer(route, title, tree, self.renderer)
    }
    pub fn add_page_with_renderer(
        &mut self,
        route: &str,
        title: &str,
        tree: Node,
        renderer: crate::Renderer,
    ) -> Result<(), String> {
        if !route.starts_with('/')
            || route.starts_with("/__nano")
            || route.contains(['?', '#'])
            || self.pages.contains_key(route)
        {
            return Err(format!("Invalid or duplicate route: {route}"));
        }
        tree.validate()?;
        self.pages.insert(
            route.into(),
            Page {
                renderer,
                title: title.into(),
                plan: Arc::new(RenderPlan::for_renderer(&tree, renderer)?),
                tree,
            },
        );
        Ok(())
    }
    pub fn on(
        &mut self,
        name: &str,
        handler: impl Fn(&mut Value, &[Value]) -> EventResult + Send + Sync + 'static,
    ) -> Result<(), String> {
        self.on_native(name, move |state, args| {
            let mut value = state.to_value();
            let effects = handler(&mut value, args)?;
            *state = NativeState::from_value(value)?;
            Ok(effects)
        })
    }
    pub fn on_native(
        &mut self,
        name: &str,
        handler: impl Fn(&mut NativeState, &[Value]) -> EventResult + Send + Sync + 'static,
    ) -> Result<(), String> {
        if name.is_empty()
            || self.handlers.contains_key(name)
            || self.context_handlers.contains_key(name)
        {
            return Err(format!("Invalid or duplicate event: {name}"));
        }
        self.handlers.insert(name.into(), Arc::new(handler));
        Ok(())
    }
    /// Foreground streaming callbacks may publish intermediate state through a lease.
    pub fn on_stream(
        &mut self,
        name: &str,
        handler: impl Fn(EventContext, &[Value]) -> EventResult + Send + Sync + 'static,
    ) -> Result<(), String> {
        self.on_context(name, false, handler)
    }
    /// Background callbacks schedule work and return immediately; context updates broadcast.
    pub fn on_background(
        &mut self,
        name: &str,
        handler: impl Fn(EventContext, &[Value]) -> EventResult + Send + Sync + 'static,
    ) -> Result<(), String> {
        self.on_context(name, true, handler)
    }
    fn on_context(
        &mut self,
        name: &str,
        background: bool,
        handler: impl Fn(EventContext, &[Value]) -> EventResult + Send + Sync + 'static,
    ) -> Result<(), String> {
        if name.is_empty()
            || self.handlers.contains_key(name)
            || self.context_handlers.contains_key(name)
        {
            return Err(format!("Invalid or duplicate event: {name}"));
        }
        self.context_handlers
            .insert(name.into(), (background, Arc::new(handler)));
        Ok(())
    }
    pub fn computed(
        &mut self,
        name: &str,
        compute: impl Fn(&Value) -> Result<Value, String> + Send + Sync + 'static,
    ) -> Result<(), String> {
        self.computed_native(name, move |state| compute(&state.to_value()))
    }
    fn computed_native(
        &mut self,
        name: &str,
        compute: impl Fn(&NativeState) -> Result<Value, String> + Send + Sync + 'static,
    ) -> Result<(), String> {
        self.computed_shared(name, move |state| compute(state).map(Arc::new))
    }
    fn computed_shared(
        &mut self,
        name: &str,
        compute: impl Fn(&NativeState) -> Result<Arc<Value>, String> + Send + Sync + 'static,
    ) -> Result<(), String> {
        if self.initial.get(name).is_some()
            || name.starts_with('_')
            || self.computed.iter().any(|(n, _)| n == name)
        {
            return Err(format!("Computed name collision: {name}"));
        }
        self.computed.push((name.into(), Arc::new(compute)));
        Ok(())
    }
    /// Pure native expressions can reuse results when every referenced field is equal.
    pub fn computed_expression(&mut self, name: &str, expression: Expr) -> Result<(), String> {
        let dependencies = expression.dependencies();
        let cache = Mutex::new(None::<(Vec<(String, Option<Arc<Value>>)>, Arc<Value>)>);
        self.computed_shared(name, move |state| {
            let mut cache = cache.lock().map_err(|_| "Computed cache unavailable")?;
            if let Some((inputs, value)) = &*cache {
                if inputs.iter().all(
                    |(name, value)| match (state.get_arc(name), value.as_ref()) {
                        (Some(a), Some(b)) => Arc::ptr_eq(a, b) || a == b,
                        (None, None) => true,
                        _ => false,
                    },
                ) {
                    return Ok(value.clone());
                }
            }
            let value = Arc::new(expression.eval(state, &serde_json::Map::new()));
            if !crate::runtime::safe_json(&value) {
                return Err("Computed value exceeds the JSON number range".into());
            }
            let inputs = dependencies
                .iter()
                .map(|name| (name.clone(), state.get_arc(name).cloned()))
                .collect();
            *cache = Some((inputs, value.clone()));
            Ok(value)
        })
    }
    fn resolve(&self, path: &str) -> Option<(&Page, Value)> {
        let clean = path.split('?').next().unwrap_or(path);
        if let Some(page) = self.pages.get(clean) {
            return Some((page, json!({"path":clean,"params":{}})));
        }
        let parts: Vec<_> = clean.trim_matches('/').split('/').collect();
        for (pattern, page) in &self.pages {
            let pattern: Vec<_> = pattern.trim_matches('/').split('/').collect();
            if pattern.len() != parts.len() {
                continue;
            }
            let mut params = serde_json::Map::new();
            let mut matches = true;
            for (a, b) in pattern.iter().zip(&parts) {
                if a.starts_with('[') && a.ends_with(']') && a.len() > 2 {
                    params.insert(a[1..a.len() - 1].into(), json!(b));
                } else if a != b {
                    matches = false;
                    break;
                }
            }
            if matches {
                return Some((page, json!({"path":clean,"params":params})));
            }
        }
        None
    }
    pub fn view(&self, state: &Value, path: &str) -> Result<Value, String> {
        Ok(self
            .native_view(&NativeState::from_value(state.clone())?, path)?
            .to_value())
    }
    pub fn native_view(&self, state: &NativeState, path: &str) -> Result<NativeState, String> {
        let (_, route) = self.resolve(path).ok_or("Page not found")?;
        let mut view = state.fork();
        view["_route"] = route;
        for (name, compute) in &self.computed {
            let value = compute(&view)?;
            view.insert_arc(name, value);
        }
        Ok(view)
    }
    pub fn initial_state(&self) -> Value {
        self.initial.to_value()
    }
    pub fn render(&self, path: &str) -> Result<String, String> {
        let (page, _) = self.resolve(path).ok_or("Page not found")?;
        Ok(page.plan.render(&self.native_view(&self.initial, path)?))
    }
    pub fn tree_json(&self, path: &str) -> Result<String, String> {
        serde_json::to_string(&self.resolve(path).ok_or("Page not found")?.0.tree)
            .map_err(|e| e.to_string())
    }
    /// Apply to a copy; an error never commits partially mutated state.
    pub fn dispatch(
        &self,
        state: &Value,
        name: &str,
        args: &[Value],
        path: &str,
    ) -> Result<(Value, Value, Vec<Effect>), String> {
        let (state, view, effects) =
            self.dispatch_native(&NativeState::from_value(state.clone())?, name, args, path)?;
        Ok((state.to_value(), view.to_value(), effects))
    }
    fn dispatch_native(
        &self,
        state: &NativeState,
        name: &str,
        args: &[Value],
        path: &str,
    ) -> Result<(NativeState, NativeState, Vec<Effect>), String> {
        let handler = self.handlers.get(name).ok_or("Unknown event")?;
        let mut next = state.fork();
        next["_route"] = self.resolve(path).ok_or("Page not found")?.1;
        let effects = handler(&mut next, args)?;
        next.remove("_route");
        for (name, _) in &self.computed {
            next.remove(name);
        }
        for effect in &effects {
            if let Effect::Redirect { path } = effect {
                if !safe_url(path) {
                    return Err("Unsafe redirect URL".into());
                }
            }
        }
        // Computed failures also roll back the transaction.
        let view = self.native_view(&next, path)?;
        Ok((next, view, effects))
    }
    pub fn render_plan_json(&self, path: &str) -> Result<String, String> {
        Ok(self.resolve(path).ok_or("Page not found")?.0.plan.to_json())
    }
    pub fn router(self) -> Router {
        if self
            .pages
            .values()
            .any(|page| page.renderer == crate::Renderer::React)
        {
            crate::assets::react_assets();
        }
        crate::assets::client();
        crate::assets::css();
        let state = Arc::new(Server {
            app: self,
            sessions: Mutex::new(HashMap::new()),
        });
        Router::new()
            .route("/__nano/client.js", get(client_js))
            .route("/__nano/style.css", get(style_css))
            .route("/__nano/asset/{hash}/{name}", get(asset))
            .route("/__nano/react/{hash}/{*name}", get(react_asset))
            .route("/__nano/page", get(page_json))
            .route("/__nano/event", post(event))
            .route("/__nano/ws", get(crate::websocket::upgrade))
            .fallback(get(document))
            .layer(DefaultBodyLimit::max(64 * 1024))
            .with_state(state)
    }
    pub async fn serve(self, host: &str, port: u16) -> Result<(), String> {
        let address = format!("{host}:{port}");
        let listener = tokio::net::TcpListener::bind(&address)
            .await
            .map_err(|e| e.to_string())?;
        println!("Reflex Nano → http://{address}");
        axum::serve(
            listener.tap_io(|stream| {
                let _ = stream.set_nodelay(true);
            }),
            self.router(),
        )
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|e| e.to_string())
    }
}

pub(crate) struct SessionData {
    state: NativeState,
    version: u64,
    recent: HashMap<String, (String, Reply)>,
    order: VecDeque<String>,
}
pub(crate) struct Session {
    pub(crate) csrf: String,
    data: Arc<AsyncMutex<SessionData>>,
    pub(crate) seen: AtomicU64,
    pub(crate) updates: broadcast::Sender<Arc<Snapshot>>,
    pub(crate) notices: broadcast::Sender<Value>,
    pub(crate) connections: Arc<Semaphore>,
    tasks: Arc<Semaphore>,
    gate: AsyncMutex<()>,
}
pub(crate) struct Server {
    pub(crate) app: App,
    sessions: Mutex<HashMap<String, Arc<Session>>>,
}
pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn cookie(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("cookie")?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| part.trim().strip_prefix("nano_session="))
}
impl Server {
    pub(crate) fn session(
        &self,
        headers: &HeaderMap,
        create: bool,
    ) -> Result<(Arc<Session>, Option<String>), (StatusCode, String)> {
        let mut sessions = self.sessions.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Session store unavailable".into(),
            )
        })?;
        let time = now();
        if let Some(session) = cookie(headers).and_then(|id| sessions.get(id)) {
            if time.saturating_sub(session.seen.load(Ordering::Relaxed)) < self.app.session_ttl_secs
            {
                session.seen.store(time, Ordering::Relaxed);
                return Ok((session.clone(), None));
            }
        }
        if !create {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Session expired; reload the page".into(),
            ));
        }
        sessions.retain(|_, v| {
            time.saturating_sub(v.seen.load(Ordering::Relaxed)) < self.app.session_ttl_secs
        });
        if sessions.len() >= self.app.session_limit {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "Session capacity reached".into(),
            ));
        }
        let id = Uuid::new_v4().to_string();
        let session = Arc::new(Session {
            csrf: Uuid::new_v4().to_string(),
            data: Arc::new(AsyncMutex::new(SessionData {
                state: self.app.initial.clone(),
                version: 0,
                recent: HashMap::new(),
                order: VecDeque::new(),
            })),
            seen: AtomicU64::new(time),
            updates: broadcast::channel(128).0,
            notices: broadcast::channel(32).0,
            connections: Arc::new(Semaphore::new(8)),
            tasks: Arc::new(Semaphore::new(16)),
            gate: AsyncMutex::new(()),
        });
        sessions.insert(id.clone(), session.clone());
        Ok((
            session,
            Some(format!(
                "nano_session={id}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
                self.app.session_ttl_secs
            )),
        ))
    }
    async fn bootstrap(
        &self,
        headers: &HeaderMap,
        path: &str,
        render_html: bool,
    ) -> Result<(Bootstrap, String, Option<String>), (StatusCode, String)> {
        let (page, _) = self
            .app
            .resolve(path)
            .ok_or((StatusCode::NOT_FOUND, "Page not found".into()))?;
        let (session, cookie) = self.session(headers, true)?;
        let data = session.data.lock().await;
        let state = self
            .app
            .native_view(&data.state, path)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        let version = data.version;
        drop(data);
        let html = if render_html {
            page.plan.render(&state)
        } else {
            String::new()
        };
        Ok((
            Bootstrap {
                renderer: page.renderer,
                tree: page.plan.wire.clone(),
                state,
                title: page.title.clone(),
                csrf: session.csrf.clone(),
                version,
            },
            html,
            cookie,
        ))
    }
}
#[derive(Serialize)]
struct Bootstrap {
    renderer: crate::Renderer,
    tree: Arc<Value>,
    state: NativeState,
    title: String,
    csrf: String,
    version: u64,
}
fn response_headers(response: &mut Response, cookie: Option<String>) {
    let headers = response.headers_mut();
    headers.insert("cache-control", "no-store".parse().unwrap());
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("referrer-policy", "same-origin".parse().unwrap());
    headers.insert("content-security-policy","default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' https: http:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'".parse().unwrap());
    if let Some(cookie) = cookie {
        headers.insert("set-cookie", cookie.parse().unwrap());
    }
}
async fn document(State(server): State<Arc<Server>>, headers: HeaderMap, uri: Uri) -> Response {
    match server.bootstrap(&headers, uri.path(), true).await {
        Ok((boot, body, cookie)) => {
            let encoded = serde_json::to_string(&boot)
                .expect("native bootstrap")
                .replace('&', "\\u0026")
                .replace('<', "\\u003c")
                .replace('>', "\\u003e");
            let mut title = String::new();
            escape_into(&boot.title, &mut title);
            let react = boot.renderer == crate::Renderer::React;
            let client = if react {
                &crate::assets::react_entry().path
            } else {
                &crate::assets::client().path
            };
            let css = &crate::assets::css().path;
            let preload = if react { "modulepreload" } else { "preload" };
            let script_type = if react { "type=\"module\"" } else { "defer" };
            let html=format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{title}</title><link rel=\"{preload}\" href=\"{client}\" as=\"script\"><link rel=\"stylesheet\" href=\"{css}\"></head><body><div id=\"nano-root\">{body}</div><div id=\"nano-status\" role=\"status\" aria-live=\"polite\"></div><script id=\"nano-data\" type=\"application/json\">{encoded}</script><script src=\"{client}\" {script_type}></script></body></html>");
            let mut response = crate::assets::document(html, &headers);
            response_headers(&mut response, cookie);
            response
        }
        Err((status, message)) => (status, message).into_response(),
    }
}
#[derive(Deserialize)]
struct PageQuery {
    path: String,
}
async fn page_json(
    State(server): State<Arc<Server>>,
    headers: HeaderMap,
    Query(query): Query<PageQuery>,
) -> Response {
    match server.bootstrap(&headers, &query.path, false).await {
        Ok((boot, _, cookie)) => {
            let mut response = Json(boot).into_response();
            response_headers(&mut response, cookie);
            response
        }
        Err((status, message)) => (status, Json(json!({"error":message}))).into_response(),
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct EventRequest {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) args: Vec<Value>,
    pub(crate) path: String,
}

#[derive(Clone)]
pub(crate) struct Snapshot {
    pub(crate) state: NativeState,
    pub(crate) view: NativeState,
    pub(crate) path: String,
    pub(crate) version: u64,
}
#[derive(Clone)]
pub(crate) struct Reply {
    pub(crate) snapshot: Option<Arc<Snapshot>>,
    pub(crate) effects: Vec<Effect>,
    pub(crate) error: Option<String>,
}
impl Reply {
    fn error(error: impl Into<String>) -> Self {
        Self {
            snapshot: None,
            effects: vec![],
            error: Some(error.into()),
        }
    }
}

/// Session-bound state access for streaming and background handlers.
#[derive(Clone)]
pub struct EventContext {
    server: Arc<Server>,
    session: Arc<Session>,
    path: String,
    permit: Arc<Mutex<Option<OwnedSemaphorePermit>>>,
}
pub struct StateLease {
    context: EventContext,
    guard: Option<OwnedMutexGuard<SessionData>>,
}
impl EventContext {
    /// Release this background task's capacity slot when it completes.
    pub fn finish(&self) {
        if let Ok(mut permit) = self.permit.lock() {
            permit.take();
        }
    }
    /// Blocking transaction acquisition; call from a blocking worker.
    pub fn begin(&self) -> StateLease {
        StateLease {
            context: self.clone(),
            guard: Some(self.session.data.clone().blocking_lock_owned()),
        }
    }
    pub fn read(&self) -> Value {
        self.read_native().to_value()
    }
    pub fn read_native(&self) -> NativeState {
        let mut state = self.session.data.blocking_lock().state.fork();
        state["_route"] = self
            .server
            .app
            .resolve(&self.path)
            .map(|(_, r)| r)
            .unwrap_or(Value::Null);
        state
    }
    pub fn report_error(&self, message: &str) {
        let _ = self
            .session
            .notices
            .send(json!({"type":"background_error","error":message}));
    }
    pub async fn update<F>(&self, change: F) -> Result<(), String>
    where
        F: FnOnce(&mut Value) -> Result<(), String> + Send + 'static,
    {
        self.update_native(move |state| {
            let mut value = state.to_value();
            change(&mut value)?;
            *state = NativeState::from_value(value)?;
            Ok(())
        })
        .await
    }
    pub async fn update_native<F>(&self, change: F) -> Result<(), String>
    where
        F: FnOnce(&mut NativeState) -> Result<(), String> + Send + 'static,
    {
        let context = self.clone();
        tokio::task::spawn_blocking(move || {
            let mut lease = context.begin();
            let mut state = lease.native_state();
            change(&mut state)?;
            lease.publish_native(state)?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
impl StateLease {
    pub fn state(&self) -> Value {
        self.native_state().to_value()
    }
    pub fn native_state(&self) -> NativeState {
        let mut state = self.guard.as_ref().expect("closed lease").state.fork();
        state["_route"] = self
            .context
            .server
            .app
            .resolve(&self.context.path)
            .map(|(_, r)| r)
            .unwrap_or(Value::Null);
        state
    }
    /// Publish a committed checkpoint while keeping the session lock for a streaming event.
    pub fn publish(&mut self, next: Value) -> Result<u64, String> {
        self.publish_native(NativeState::from_value(next)?)
    }
    pub fn publish_native(&mut self, mut next: NativeState) -> Result<u64, String> {
        next.remove("_route");
        for (name, _) in &self.context.server.app.computed {
            next.remove(name);
        }
        let view = self
            .context
            .server
            .app
            .native_view(&next, &self.context.path)?;
        let data = self.guard.as_mut().ok_or("Lease is closed")?;
        data.state = next.clone();
        data.version += 1;
        let snapshot = Arc::new(Snapshot {
            state: next,
            view,
            path: self.context.path.clone(),
            version: data.version,
        });
        let _ = self.context.session.updates.send(snapshot);
        self.context.session.seen.store(now(), Ordering::Relaxed);
        Ok(data.version)
    }
    pub fn close(&mut self) {
        self.guard.take();
    }
}
impl Server {
    pub(crate) async fn snapshot(
        self: &Arc<Self>,
        session: &Arc<Session>,
        path: &str,
    ) -> Result<Arc<Snapshot>, String> {
        let server = self.clone();
        let session = session.clone();
        let path = path.to_owned();
        tokio::task::spawn_blocking(move || {
            let data = session.data.blocking_lock();
            let view = server.app.native_view(&data.state, &path)?;
            Ok(Arc::new(Snapshot {
                state: data.state.clone(),
                view,
                path,
                version: data.version,
            }))
        })
        .await
        .map_err(|e| e.to_string())?
    }
    pub(crate) async fn apply(
        self: &Arc<Self>,
        session: Arc<Session>,
        request: EventRequest,
        id: String,
    ) -> Reply {
        let server = self.clone();
        tokio::task::spawn_blocking(move || {
            let _serial = session.gate.blocking_lock();
            let fingerprint = serde_json::to_string(&request).unwrap();
            {
                let data = session.data.blocking_lock();
                if let Some((previous, receipt)) = data.recent.get(&id) {
                    if previous != &fingerprint {
                        return Reply::error("Request ID reused with different payload");
                    }
                    let mut reply = receipt.clone();
                    if let Ok(view) = server.app.native_view(&data.state, &request.path) {
                        reply.snapshot = Some(Arc::new(Snapshot {
                            state: data.state.clone(),
                            view,
                            path: request.path.clone(),
                            version: data.version,
                        }));
                    }
                    return reply;
                }
            }
            let result = (|| -> Result<(Arc<Snapshot>, Vec<Effect>), String> {
                if let Some((background, handler)) = server.app.context_handlers.get(&request.name)
                {
                    if server.app.resolve(&request.path).is_none() {
                        return Err("Page not found".into());
                    }
                    let permit = if *background {
                        Some(
                            session
                                .tasks
                                .clone()
                                .try_acquire_owned()
                                .map_err(|_| "Background task limit reached")?,
                        )
                    } else {
                        None
                    };
                    let context = EventContext {
                        server: server.clone(),
                        session: session.clone(),
                        path: request.path.clone(),
                        permit: Arc::new(Mutex::new(permit)),
                    };
                    let effects = handler(context, &request.args)?;
                    for effect in &effects {
                        if let Effect::Redirect { path } = effect {
                            if !safe_url(path) {
                                return Err("Unsafe redirect URL".into());
                            }
                        }
                    }
                    let data = session.data.blocking_lock();
                    let view = server.app.native_view(&data.state, &request.path)?;
                    return Ok((
                        Arc::new(Snapshot {
                            state: data.state.clone(),
                            view,
                            path: request.path.clone(),
                            version: data.version,
                        }),
                        effects,
                    ));
                }
                let mut data = session.data.blocking_lock();
                let (next, view, effects) = server.app.dispatch_native(
                    &data.state,
                    &request.name,
                    &request.args,
                    &request.path,
                )?;
                data.state = next.clone();
                data.version += 1;
                let snapshot = Arc::new(Snapshot {
                    state: next,
                    view,
                    path: request.path.clone(),
                    version: data.version,
                });
                let _ = session.updates.send(snapshot.clone());
                Ok((snapshot, effects))
            })();
            let reply = match result {
                Ok((snapshot, effects)) => Reply {
                    snapshot: Some(snapshot),
                    effects,
                    error: None,
                },
                Err(e) => Reply::error(e),
            };
            let mut data = session.data.blocking_lock();
            if data.order.len() >= 256 {
                if let Some(old) = data.order.pop_front() {
                    data.recent.remove(&old);
                }
            }
            data.order.push_back(id.clone());
            let mut receipt = reply.clone();
            receipt.snapshot = None;
            data.recent.insert(id, (fingerprint, receipt));
            session.seen.store(now(), Ordering::Relaxed);
            reply
        })
        .await
        .unwrap_or_else(|_| Reply::error("Event handler failed"))
    }
}
async fn event(
    State(server): State<Arc<Server>>,
    headers: HeaderMap,
    Json(request): Json<EventRequest>,
) -> Response {
    let session = match server.session(&headers, false) {
        Ok((s, _)) => s,
        Err((s, m)) => return (s, Json(json!({"error":m}))).into_response(),
    };
    if headers.get("x-nano-csrf").and_then(|v| v.to_str().ok()) != Some(session.csrf.as_str()) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error":"Invalid event token"})),
        )
            .into_response();
    }
    let reply = server
        .apply(session, request, Uuid::new_v4().to_string())
        .await;
    let mut response = if let Some(error) = reply.error {
        (StatusCode::BAD_REQUEST, Json(json!({"error":error}))).into_response()
    } else {
        let snapshot = reply.snapshot.unwrap();
        Json(json!({"state":snapshot.view,"version":snapshot.version,"effects":reply.effects}))
            .into_response()
    };
    response_headers(&mut response, None);
    response
}
async fn client_js(headers: HeaderMap) -> Response {
    crate::assets::client().response(&headers, false)
}
async fn style_css(headers: HeaderMap) -> Response {
    crate::assets::css().response(&headers, false)
}
async fn asset(headers: HeaderMap, uri: Uri) -> Response {
    for asset in [crate::assets::client(), crate::assets::css()] {
        if uri.path() == asset.path {
            return asset.response(&headers, true);
        }
    }
    StatusCode::NOT_FOUND.into_response()
}
async fn react_asset(headers: HeaderMap, uri: Uri) -> Response {
    match crate::assets::react_assets()
        .iter()
        .find(|asset| asset.path == uri.path())
    {
        Some(asset) => asset.response(&headers, true),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
