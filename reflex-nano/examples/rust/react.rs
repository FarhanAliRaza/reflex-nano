//! Same native schema/program/component API, with a direct HTML fallback.
use reflex_nano::{Action, Application, Event, Expr, Node, Program, Renderer, Schema, ValueType};
#[tokio::main]
async fn main() -> Result<(), String> {
    let mut schema = Schema::default();
    schema.field("count", ValueType::Int, 0.into())?;
    schema.event(
        "increment",
        Program {
            parameters: vec![],
            mode: Default::default(),
            actions: vec![Action::Set {
                field: "count".into(),
                path: vec![],
                value: Expr::state("count").plus(1i64),
            }],
        },
    )?;
    let fallback = Node::el(
        "main",
        [
            Node::text(Expr::state("count")),
            Node::el("button", [Node::text("Increment")]).on("click", Event::new("increment")),
        ],
    );
    let tree = Node::component(
        "@radix-ui/themes",
        "Theme",
        [Node::component(
            "@radix-ui/themes",
            "Button",
            [Node::text(Expr::state("count"))],
        )
        .on("click", Event::new("increment"))],
    )
    .fallback(fallback);
    let mut app = Application::new(schema);
    app.renderer = std::env::var("NANO_RENDERER")
        .unwrap_or("react".into())
        .parse::<Renderer>()?;
    app.page("/", "Rust + React", tree);
    app.build()?.serve("127.0.0.1", 3002).await
}
