//! Application definitions shared by the native executable and Python bindings.
use crate::{
    json, Action, Application, Event, EventMode, Expr, Node, Parameter, Program, Schema, ValueType,
};

fn set(field: &str, value: Expr) -> Action {
    Action::Set {
        field: field.into(),
        path: vec![],
        value,
    }
}
fn event(
    schema: &mut Schema,
    name: &str,
    parameters: Vec<(&str, ValueType)>,
    actions: Vec<Action>,
    mode: EventMode,
) -> Result<(), String> {
    schema.event(
        &format!("RuntimeState.{name}"),
        Program {
            parameters: parameters
                .into_iter()
                .map(|(name, kind)| Parameter {
                    name: name.into(),
                    kind,
                })
                .collect(),
            actions,
            mode,
        },
    )
}
pub fn benchmark_application(rows: usize) -> Result<Application, String> {
    if rows > 100_000 {
        return Err("Demo row limit exceeded".into());
    }
    let mut schema = Schema::default();
    schema.field("count", ValueType::Int, json!(0))?;
    schema.field("name", ValueType::String, json!("Nano"))?;
    schema.field(
        "items",
        ValueType::List,
        json!((0..rows)
            .map(|i| json!({"id":i,"label":format!("Item {i}"),"done":false}))
            .collect::<Vec<_>>()),
    )?;
    schema.field("progress", ValueType::Int, json!(0))?;
    schema.computed("doubled", Expr::state("count").binary("mul", 2i64))?;
    schema.computed(
        "remaining",
        Expr::state("items").count_where("item", Expr::local("item").at("done").negate()),
    )?;
    let increment = vec![set(
        "count",
        Expr::state("count").plus(Expr::local("amount")),
    )];
    event(
        &mut schema,
        "increment",
        vec![("amount", ValueType::Int)],
        increment.clone(),
        EventMode::Immediate,
    )?;
    event(
        &mut schema,
        "increment_async",
        vec![("amount", ValueType::Int)],
        increment,
        EventMode::Deferred { milliseconds: 0 },
    )?;
    event(
        &mut schema,
        "rename",
        vec![("name", ValueType::String)],
        vec![set("name", Expr::local("name"))],
        EventMode::Immediate,
    )?;
    event(
        &mut schema,
        "submit",
        vec![("data", ValueType::Object)],
        vec![set("name", Expr::local("data").at("name").trim())],
        EventMode::Immediate,
    )?;
    event(
        &mut schema,
        "toggle",
        vec![("index", ValueType::Int)],
        vec![Action::Set {
            field: "items".into(),
            path: vec![Expr::local("index"), "done".into()],
            value: Expr::state("items")
                .at(Expr::local("index"))
                .at("done")
                .negate(),
        }],
        EventMode::Immediate,
    )?;
    event(
        &mut schema,
        "stream",
        vec![("steps", ValueType::Int)],
        vec![Action::Repeat {
            times: Expr::local("steps"),
            actions: vec![
                set("count", Expr::state("count").plus(1i64)),
                Action::Publish,
            ],
        }],
        EventMode::Streaming,
    )?;
    event(
        &mut schema,
        "background",
        vec![("steps", ValueType::Int)],
        vec![set("progress", Expr::state("progress").plus(1i64))],
        EventMode::Background {
            milliseconds: 20,
            times: Expr::local("steps"),
        },
    )?;
    event(
        &mut schema,
        "chain",
        vec![],
        vec![
            Action::Call {
                name: "RuntimeState.increment".into(),
                args: vec![2i64.into()],
            },
            Action::Call {
                name: "RuntimeState.increment".into(),
                args: vec![3i64.into()],
            },
        ],
        EventMode::Immediate,
    )?;
    let mut app = Application::new(schema);
    app.page(
        "/",
        "Runtime benchmark",
        Node::el(
            "main",
            [
                Node::text(Expr::state("count")),
                Node::text(Expr::state("doubled")),
                Node::text(Expr::state("remaining")),
                Node::text(Expr::state("name")),
                Node::text(Expr::state("progress")),
                Node::each(
                    Expr::state("items"),
                    "item",
                    "index",
                    Node::el("p", [Node::text(Expr::local("item").at("label"))])
                        .key(Expr::local("item").at("id")),
                ),
            ],
        ),
    );
    Ok(app)
}
fn paragraph(id: &str, value: Expr) -> Node {
    Node::el("p", [Node::text(value)]).attr("id", id)
}
fn button(label: &str, name: &str, args: Vec<Expr>) -> Node {
    let mut event = Event::new(format!("RuntimeState.{name}"));
    event.args = args;
    Node::el("button", [Node::text(label)])
        .attr("type", "button")
        .on("click", event)
}
fn card(title: &str, children: Vec<Node>) -> Node {
    Node::el(
        "section",
        std::iter::once(Node::el("h2", [Node::text(title)])).chain(children),
    )
    .style("padding", "24px")
    .style("border", "1px solid #dbe4dd")
    .style("border-radius", "16px")
    .style("background", "white")
    .style("display", "grid")
    .style("gap", "14px")
}
pub fn dashboard_application() -> Result<Application, String> {
    let mut app = benchmark_application(3)?;
    app.pages.clear();
    app.schema.fields.get_mut("items").unwrap().initial = json!([
        {"id":1,"label":"Build the Rust runtime","done":false},
        {"id":2,"label":"Expose native Python bindings","done":false},
        {"id":3,"label":"Measure real WebSocket events","done":false},
    ]);
    app.schema.field("next_id", ValueType::Int, json!(4))?;
    app.schema
        .field("priority", ValueType::String, json!("Medium"))?;
    event(
        &mut app.schema,
        "priority",
        vec![("value", ValueType::String)],
        vec![set("priority", Expr::local("value"))],
        EventMode::Immediate,
    )?;
    event(
        &mut app.schema,
        "add_task",
        vec![("data", ValueType::Object)],
        vec![
            Action::Require {
                condition: Expr::local("data").at("task").trim().length().gt(0i64),
                message: "Write a task first".into(),
            },
            Action::Append {
                field: "items".into(),
                value: Expr::object([
                    ("id".into(), Expr::state("next_id")),
                    ("label".into(), Expr::local("data").at("task").trim()),
                    ("done".into(), Expr::literal(false)),
                ]),
            },
            set("next_id", Expr::state("next_id").plus(1i64)),
        ],
        EventMode::Immediate,
    )?;
    event(
        &mut app.schema,
        "remove",
        vec![("index", ValueType::Int)],
        vec![Action::Remove {
            field: "items".into(),
            index: Expr::local("index"),
        }],
        EventMode::Immediate,
    )?;
    let task = Expr::local("item");
    let index = Expr::local("index");
    let row = Node::el(
        "div",
        [
            Node::el("input", [])
                .attr("type", "checkbox")
                .attr("checked", task.clone().at("done"))
                .attr(
                    "aria-label",
                    Expr::concat(["Complete ".into(), task.clone().at("label")]),
                )
                .on(
                    "input",
                    Event::new("RuntimeState.toggle").arg(index.clone()),
                ),
            Node::el("a", [Node::text(task.clone().at("label"))])
                .attr(
                    "href",
                    Expr::concat(["/item/".into(), task.clone().at("id")]),
                )
                .style("flex", "1"),
            button("Remove", "remove", vec![index]).attr(
                "aria-label",
                Expr::concat(["Remove ".into(), task.clone().at("label")]),
            ),
        ],
    )
    .key(task.at("id"))
    .style("display", "flex")
    .style("align-items", "center")
    .style("gap", "12px");
    let content=Node::el("main",[
        paragraph("eyebrow","REFLEX NANO / NATIVE RUST RUNTIME".into()).style("font-size","12px").style("letter-spacing","2px"),
        Node::el("h1",[Node::text("Small framework. Rust all the way.")]).style("font-size","40px"),
        Node::el("p",[Node::text("State, events, computed values and background work run in Rust. This same app runs through the Rust executable or thin Python bindings.")]),
        card("Reactive state",vec![
            paragraph("count",Expr::state("count")).style("font-size","64px").style("margin","0"),
            paragraph("doubled",Expr::concat(["Computed double: ".into(),Expr::state("doubled")])),
            Node::when(Expr::state("count").gt(0i64),paragraph("positive","Above zero".into()),Node::text("Ready")),
            Node::el("div",[button("Increment","increment",vec![1i64.into()]),button("Async +1","increment_async",vec![1i64.into()])]).style("display","flex").style("gap","12px"),
        ]),
        card("Controlled inputs",vec![
            Node::el("label",[Node::text("Your name")]).attr("for","name"),
            Node::el("input",[]).attr("id","name").attr("value",Expr::state("name")).on("input",Event::new("RuntimeState.rename").arg(Expr::local("event").at("value"))),
            paragraph("greeting",Expr::concat(["Hello, ".into(),Expr::state("name"),".".into()])),
            Node::el("select",["Low","Medium","High"].map(|p|Node::el("option",[Node::text(p)]).attr("value",p)))
                .attr("id","priority").attr("value",Expr::state("priority")).on("change",Event::new("RuntimeState.priority").arg(Expr::local("event").at("value"))),
            paragraph("priority-label",Expr::state("priority").eq_to("High").choose("High priority","Normal priority")),
        ]),
        card("Forms and keyed lists",vec![
            Node::el("form",[
                Node::el("input",[]).attr("name","task").attr("id","task-input").attr("placeholder","Your next task"),
                Node::el("button",[Node::text("Add task")]).attr("type","submit"),
            ]).on("submit",Event::new("RuntimeState.add_task").arg(Expr::local("event").at("value")).prevent_default()).attr("data-nano-reset","true")
                .style("display","flex").style("gap","12px").style("flex-wrap","wrap"),
            Node::el("div",[Node::each(Expr::state("items"),"item","index",row)]).attr("id","tasks").style("display","grid").style("gap","12px"),
            paragraph("remaining",Expr::concat([Expr::state("remaining")," remaining".into()])),
        ]),
        card("Live WebSocket updates",vec![
            paragraph("progress",Expr::concat(["Progress: ".into(),Expr::state("progress")])),
            Node::el("div",[button("Start background task","background",vec![5i64.into()]),button("Stream +3","stream",vec![3i64.into()]),button("Chain +5","chain",vec![])])
                .style("display","flex").style("gap","12px").style("flex-wrap","wrap"),
            Node::text("Open another tab to see shared session updates arrive automatically."),
        ]),
        Node::el("a",[Node::text("About")]).attr("href","/about"),
    ]).style("max-width","860px").style("margin","36px auto").style("padding","0 20px").style("display","grid").style("gap","24px");
    app.page("/", "Reflex Nano · Rust runtime", content);
    app.page(
        "/about",
        "About · Nano",
        card(
            "Rust owns the runtime",
            vec![
                paragraph(
                    "about-count",
                    Expr::concat(["Current counter: ".into(), Expr::state("count")]),
                ),
                Node::el("a", [Node::text("Back")]).attr("href", "/"),
            ],
        ),
    );
    app.page(
        "/item/[id]",
        "Task · Nano",
        card(
            "Task details",
            vec![
                paragraph(
                    "task-id",
                    Expr::concat([
                        "Task ID: ".into(),
                        Expr::state("_route").at("params").at("id"),
                    ]),
                ),
                Node::el("a", [Node::text("Back")]).attr("href", "/"),
            ],
        ),
    );
    Ok(app)
}
