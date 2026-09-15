use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[test]
fn dual_renderer_translation_and_ssr_text_boundaries() {
    let tree = Node::el(
        "label",
        [
            Node::text("A"),
            Node::text(Expr::state("name")),
            Node::text("Z"),
        ],
    )
    .attr("class", "label")
    .attr("for", "input")
    .style("background-color", "red")
    .on("click", Event::new("increment"));
    let html = RenderPlan::new(&tree).unwrap();
    let react = RenderPlan::for_renderer(&tree, Renderer::React).unwrap();
    let state = json!({"name":"Nano"});
    assert!(html.render(&state).contains(">ANanoZ</label>"));
    assert!(react
        .render(&state)
        .contains(">A<!-- -->Nano<!-- -->Z</label>"));
    let wire: Value = serde_json::from_str(&react.to_json()).unwrap();
    assert_eq!(wire["attrs"]["className"]["value"], "label");
    assert_eq!(wire["attrs"]["htmlFor"]["value"], "input");
    assert_eq!(wire["styles"]["backgroundColor"]["value"], "red");
    assert_eq!(wire["events"]["onClick"]["name"], "increment");
    let component =
        Node::component("@radix-ui/themes", "Button", [Node::text("React")]).fallback(tree);
    assert_eq!(
        RenderPlan::new(&component).unwrap().render(&state),
        html.render(&state)
    );
    assert_eq!(
        RenderPlan::for_renderer(&component, Renderer::React)
            .unwrap()
            .render(&state),
        react.render(&state)
    );
    assert!(RenderPlan::new(&Node::component("@radix-ui/themes", "Button", [])).is_err());
    assert!(
        RenderPlan::for_renderer(&Node::component("missing", "Unknown", []), Renderer::React)
            .is_err()
    );
}

#[test]
fn compiled_render_preserves_scopes_escaping_and_dependencies() {
    let row = Expr::local("row");
    let tree = Node::el(
        "main",
        [
            Node::text(Expr::literal(2).plus(3i64)),
            Node::each(
                Expr::state("groups"),
                "row",
                "index",
                Node::el(
                    "section",
                    [
                        Node::text(row.clone().at("label")),
                        Node::each(
                            row.clone().at("children"),
                            "row",
                            "index",
                            Node::el("b", [Node::text(row.clone().at("label"))]),
                        ),
                        Node::text(row.clone().at("label")),
                    ],
                )
                .attr("title", row.at("label")),
            ),
            Node::el("textarea", []).attr("value", Expr::state("name")),
            Node::el("a", [Node::text("safe")]).attr("href", "javascript:alert(1)"),
        ],
    );
    let state =
        json!({"groups":[{"label":"<outer>","children":[{"label":"inner &"}]}],"name":"<typed>"});
    let plan = RenderPlan::new(&tree).unwrap();
    assert_eq!(plan.render(&state), tree.render(&state));
    let wire: Value = serde_json::from_str(&plan.to_json()).unwrap();
    assert_eq!(wire["_s"], json!(["groups", "name"]));
    assert_eq!(wire["_l"], json!([]));
    assert_eq!(
        wire["children"][0]["value"],
        json!({"op":"literal","value":5})
    );
    assert_eq!(
        wire["children"][1]["body"]["children"][1]["_l"],
        json!(["row"])
    );
}

#[test]
fn computed_collections_share_cached_outputs_and_invalidate_transitively() {
    let mut app = App::new(json!({"items":[{"label":"first"}], "count":0}));
    app.add_page("/", "Cache", Node::text("")).unwrap();
    app.computed_expression("copy", Expr::state("items"))
        .unwrap();
    app.computed_expression("first", Expr::state("copy").at(0i64).at("label"))
        .unwrap();
    let state = NativeState::from_value(app.initial_state()).unwrap();
    let a = app.native_view(&state, "/").unwrap();
    let mut next = state.fork();
    next["count"] = json!(1);
    let b = app.native_view(&next, "/").unwrap();
    assert!(std::sync::Arc::ptr_eq(
        a.get_arc("copy").unwrap(),
        b.get_arc("copy").unwrap()
    ));
    next["items"][0]["label"] = json!("changed");
    let c = app.native_view(&next, "/").unwrap();
    assert!(!std::sync::Arc::ptr_eq(
        a.get_arc("copy").unwrap(),
        c.get_arc("copy").unwrap()
    ));
    assert_eq!(c["first"], "changed");
    assert_eq!(a["first"], "first");
}

#[tokio::test]
async fn compressed_documents_and_immutable_assets_preserve_http_contract() {
    use std::io::Read;
    let mut application = app();
    application
        .add_page(
            "/compressed",
            "Compression",
            Node::fragment([
                Node::el("main", [Node::text("0")]),
                Node::text("large document ".repeat(100)),
            ]),
        )
        .unwrap();
    let router = application.router();
    let request = |uri: &str, encoding: &str| {
        Request::builder()
            .uri(uri)
            .header("accept-encoding", encoding)
            .body(Body::empty())
            .unwrap()
    };
    let response = router
        .clone()
        .oneshot(request("/compressed", "gzip"))
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["content-encoding"], "gzip");
    assert!(response.headers().contains_key("set-cookie"));
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let mut html = String::new();
    flate2::read::GzDecoder::new(bytes.as_ref())
        .read_to_string(&mut html)
        .unwrap();
    assert!(html.contains("<main>0</main>"));
    assert!(html.contains(&crate::assets::client().path));
    let path = &crate::assets::client().path;
    let response = router.clone().oneshot(request(path, "gzip")).await.unwrap();
    assert_eq!(response.headers()["content-encoding"], "gzip");
    assert!(response.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("immutable"));
    let etag = response.headers()["etag"].clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let mut js = String::new();
    flate2::read::GzDecoder::new(bytes.as_ref())
        .read_to_string(&mut js)
        .unwrap();
    assert!(js.contains("__NANO__"));
    let plain = router
        .clone()
        .oneshot(request(path, "gzip;q=0, *;q=1"))
        .await
        .unwrap();
    assert!(!plain.headers().contains_key("content-encoding"));
    assert_eq!(plain.into_body().collect().await.unwrap().to_bytes(), js);
    let cached = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .header("if-none-match", etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cached.status(), StatusCode::NOT_MODIFIED);
    assert!(cached
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .is_empty());
    let legacy = router
        .oneshot(request("/__nano/client.js", "identity"))
        .await
        .unwrap();
    assert_eq!(legacy.headers()["cache-control"], "no-cache");
}

#[test]
fn native_state_shares_untouched_fields_and_rolls_back_nested_writes() {
    let state = NativeState::from_value(json!({"count":0,"items":[{"done":false}]})).unwrap();
    let mut next = state.fork();
    next["count"] = json!(1);
    assert!(std::sync::Arc::ptr_eq(
        state.get_arc("items").unwrap(),
        next.get_arc("items").unwrap()
    ));
    assert_eq!(state["count"], 0);
    next["items"][0]["done"] = json!(true);
    assert!(!std::sync::Arc::ptr_eq(
        state.get_arc("items").unwrap(),
        next.get_arc("items").unwrap()
    ));
    assert_eq!(state["items"][0]["done"], false);
    assert_eq!(next.dirty_fields().len(), 2);
}

#[test]
fn native_manifest_chain_and_dependency_order() {
    let definition = benchmark_application(3).unwrap();
    let mut restored = Application::from_json(&definition.to_json().unwrap()).unwrap();
    restored
        .schema
        .computed("after_double", Expr::state("doubled").plus(1i64))
        .unwrap();
    let app = restored.build().unwrap();
    let (state, view, effects) = app
        .dispatch(&app.initial_state(), "RuntimeState.chain", &[], "/")
        .unwrap();
    assert_eq!(state["count"], 5);
    assert_eq!(view["after_double"], 11);
    assert!(effects.is_empty());
    let (_, view, _) = app
        .dispatch(&state, "RuntimeState.toggle", &[json!(1)], "/")
        .unwrap();
    assert_eq!(view["remaining"], 2);
    restored
        .schema
        .computed("cycle", Expr::state("cycle"))
        .unwrap();
    assert!(restored.build().is_err());
}

fn app() -> App {
    let mut app = App::new(json!({"count":0}));
    app.add_page(
        "/",
        "Test",
        Node::el("main", [Node::text(Expr::state("count"))]),
    )
    .unwrap();
    app.add_page(
        "/item/[id]",
        "Item",
        Node::text(Expr::state("_route").at("params").at("id")),
    )
    .unwrap();
    app.on("increment", |s, _| {
        s["count"] = json!(s["count"].as_i64().unwrap() + 1);
        Ok(vec![])
    })
    .unwrap();
    app.on("fail", |s, _| {
        s["count"] = json!(999);
        Err("rollback".into())
    })
    .unwrap();
    app.computed("double", |s| Ok(json!(s["count"].as_i64().unwrap() * 2)))
        .unwrap();
    app
}

#[test]
fn expressions_and_dynamic_rendering() {
    let app = app();
    assert_eq!(app.render("/").unwrap(), "<main>0</main>");
    assert_eq!(app.render("/item/42").unwrap(), "42");
    assert!(app.render("/missing").is_err());
    let list = Node::each(
        Expr::state("items"),
        "row",
        "i",
        Node::text(Expr::local("row").at("label")),
    );
    assert_eq!(
        list.render(&json!({"items":[{"label":"<one>"},{"label":"two"}]})),
        "&lt;one&gt;two"
    );
    assert_eq!(
        Expr::literal(1)
            .plus(1i64)
            .eq_to(2i64)
            .eval(&json!({}), &Default::default()),
        json!(true)
    );
}

#[test]
fn transaction_rolls_back_and_tree_clones_are_immutable() {
    let app = app();
    let state = app.initial_state();
    assert!(app.dispatch(&state, "fail", &[], "/").is_err());
    assert_eq!(state["count"], 0);
    let (next, view, _) = app.dispatch(&state, "increment", &[], "/").unwrap();
    assert_eq!(view["double"], 2);
    assert_eq!(app.view(&next, "/").unwrap()["double"], 2);
    let node = Node::el("p", [Node::text("text")]);
    let modified = node.clone().attr("id", "new");
    assert_eq!(node.render(&state), "<p>text</p>");
    assert!(modified.render(&state).contains("id=\"new\""));
}

#[test]
fn validates_tags_and_escapes_values() {
    let mut app = App::new(json!({}));
    assert!(app.add_page("/", "", Node::el("script", [])).is_err());
    assert!(app
        .add_page("/", "", Node::el("div", []).attr("onload", "bad"))
        .is_err());
    let link = Node::el("a", [Node::text("<bad>")]).attr("href", "javascript:alert(1)");
    assert_eq!(link.render(&json!({})), "<a>&lt;bad&gt;</a>");
}

#[tokio::test]
async fn http_sessions_csrf_transactions_and_routes() {
    let router = app().router();
    let page = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/__nano/page?path=/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    let cookie = page.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let boot: Value =
        serde_json::from_slice(&page.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let csrf = boot["csrf"].as_str().unwrap();
    for (name, token, status, count) in [
        ("increment", "wrong", StatusCode::FORBIDDEN, None),
        ("increment", csrf, StatusCode::OK, Some(1)),
        ("fail", csrf, StatusCode::BAD_REQUEST, None),
        ("increment", csrf, StatusCode::OK, Some(2)),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri("/__nano/event")
            .header("cookie", &cookie)
            .header("x-nano-csrf", token)
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"name":name,"args":[],"path":"/"}).to_string(),
            ))
            .unwrap();
        let response = router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), status);
        if let Some(count) = count {
            let result: Value =
                serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                    .unwrap();
            assert_eq!(result["state"]["count"], count);
            assert_eq!(result["state"]["double"], count * 2);
        }
    }
    let second = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/__nano/page?path=/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let second: Value =
        serde_json::from_slice(&second.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(second["state"]["count"], 0);
    assert_ne!(second["csrf"], csrf);
    let missing = router
        .oneshot(
            Request::builder()
                .uri("/missing")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}
