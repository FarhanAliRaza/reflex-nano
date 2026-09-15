//! Rust compiler for memo boundaries, constant expressions and reusable SSR ops.
//! This plan is application data consumed by a small browser DOM adapter.
use crate::model::{
    display, escape_into, is_bool_attr, is_url_attr, safe_url, truthy, Locals, Scope,
};
use crate::{Expr, Node, NodeKind, Renderer, StateView};
use serde_json::{json, Map, Value};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Default)]
struct Dependencies {
    state: BTreeSet<String>,
    locals: BTreeSet<String>,
}
impl Dependencies {
    fn add(&mut self, other: Self) {
        self.state.extend(other.state);
        self.locals.extend(other.locals);
    }
    fn expression(&mut self, value: &mut Value) {
        let expr: Expr = serde_json::from_value(value.clone()).expect("native expression");
        let state = expr.dependencies();
        let locals = expr.locals();
        if state.is_empty() && locals.is_empty() {
            *value =
                serde_json::to_value(Expr::literal(expr.eval(&Value::Null, &Map::new()))).unwrap();
        }
        self.state.extend(state);
        self.locals.extend(locals);
    }
}
fn annotate(node: &mut Value) -> Dependencies {
    let mut deps = Dependencies::default();
    match node["kind"].as_str().unwrap() {
        "text" => deps.expression(&mut node["value"]),
        "element" | "component" => {
            if !node["fallback"].is_null() {
                deps.add(annotate(&mut node["fallback"]));
            }
            for field in ["attrs", "styles"] {
                for value in node[field].as_object_mut().unwrap().values_mut() {
                    deps.expression(value);
                }
            }
            if !node["key"].is_null() {
                deps.expression(&mut node["key"]);
            }
            for event in node["events"].as_object_mut().unwrap().values_mut() {
                for value in event["args"].as_array_mut().unwrap() {
                    deps.expression(value);
                }
            }
            for child in node["children"].as_array_mut().unwrap() {
                deps.add(annotate(child));
            }
        }
        "fragment" => {
            for child in node["children"].as_array_mut().unwrap() {
                deps.add(annotate(child));
            }
        }
        "when" => {
            deps.expression(&mut node["condition"]);
            deps.add(annotate(&mut node["yes"]));
            deps.add(annotate(&mut node["no"]));
        }
        "each" => {
            deps.expression(&mut node["items"]);
            let mut body = annotate(&mut node["body"]);
            body.locals.remove(node["name"].as_str().unwrap());
            body.locals.remove(node["index"].as_str().unwrap());
            deps.add(body);
        }
        _ => unreachable!(),
    }
    node["_s"] = json!(deps.state);
    node["_l"] = json!(deps.locals);
    deps
}

enum Op {
    Html(String),
    Text(Expr),
    Attribute(String, Expr),
    Style(String, Expr),
    When(Expr, Vec<Op>, Vec<Op>),
    Each(Expr, String, String, Vec<Op>),
}
fn html(ops: &mut Vec<Op>, text: impl AsRef<str>) {
    if let Some(Op::Html(previous)) = ops.last_mut() {
        previous.push_str(text.as_ref());
    } else {
        ops.push(Op::Html(text.as_ref().into()));
    }
}
fn static_value(expr: &Expr) -> Option<Value> {
    (expr.dependencies().is_empty() && expr.locals().is_empty())
        .then(|| expr.eval(&Value::Null, &Map::new()))
}
fn text(ops: &mut Vec<Op>, expr: &Expr, react: bool) {
    if react {
        ops.push(Op::Text(
            static_value(expr)
                .map(Expr::literal)
                .unwrap_or_else(|| expr.clone()),
        ));
        return;
    }
    if let Some(value) = static_value(expr) {
        let mut encoded = String::new();
        escape_into(&display(&value), &mut encoded);
        html(ops, encoded);
    } else {
        ops.push(Op::Text(expr.clone()));
    }
}
fn attribute(name: &str, value: &Value, out: &mut String) {
    if value.is_null()
        || (is_bool_attr(name) && !truthy(value))
        || (is_url_attr(name) && !safe_url(&display(value)))
    {
        return;
    }
    out.push(' ');
    out.push_str(name);
    out.push_str("=\"");
    escape_into(&display(value), out);
    out.push('"');
}
fn style(name: &str, value: &Value, out: &mut String) {
    if value.is_null() {
        return;
    }
    out.push_str(name);
    out.push(':');
    escape_into(&display(value), out);
    out.push(';');
}
fn compile(node: &Node, ops: &mut Vec<Op>, react: bool) {
    match node.0.as_ref() {
        NodeKind::Text { value } => text(ops, value, react),
        NodeKind::Component { fallback, .. } => {
            if let Some(fallback) = fallback {
                compile(fallback, ops, react);
            }
        }
        NodeKind::Fragment { children } => {
            for child in children {
                compile(child, ops, react);
            }
        }
        NodeKind::When { condition, yes, no } => {
            if let Some(value) = static_value(condition) {
                compile(if truthy(&value) { yes } else { no }, ops, react);
            } else {
                let mut a = vec![];
                let mut b = vec![];
                compile(yes, &mut a, react);
                compile(no, &mut b, react);
                ops.push(Op::When(condition.clone(), a, b));
            }
        }
        NodeKind::Each {
            items,
            name,
            index,
            body,
        } => {
            let mut body_ops = vec![];
            compile(body, &mut body_ops, react);
            ops.push(Op::Each(
                items.clone(),
                name.clone(),
                index.clone(),
                body_ops,
            ));
        }
        NodeKind::Element {
            tag,
            attrs,
            styles,
            children,
            ..
        } => {
            html(ops, format!("<{tag}"));
            for (name, value) in attrs {
                if tag == "textarea" && name == "value" {
                    continue;
                }
                if let Some(value) = static_value(value) {
                    let mut encoded = String::new();
                    attribute(name, &value, &mut encoded);
                    html(ops, encoded);
                } else {
                    ops.push(Op::Attribute(name.clone(), value.clone()));
                }
            }
            if !styles.is_empty() {
                html(ops, " style=\"");
                for (name, value) in styles {
                    if let Some(value) = static_value(value) {
                        let mut encoded = String::new();
                        style(name, &value, &mut encoded);
                        html(ops, encoded);
                    } else {
                        ops.push(Op::Style(name.clone(), value.clone()));
                    }
                }
                html(ops, "\"");
            }
            html(ops, ">");
            if !matches!(
                tag.as_str(),
                "area" | "br" | "col" | "hr" | "img" | "input" | "source" | "track" | "wbr"
            ) {
                if tag == "textarea" {
                    if let Some(value) = attrs.get("value") {
                        text(ops, value, false);
                    }
                }
                for child in children {
                    compile(child, ops, react);
                }
                html(ops, format!("</{tag}>"));
            }
        }
    }
}
fn write(
    ops: &[Op],
    state: &dyn StateView,
    scope: &dyn Locals,
    out: &mut String,
    react: bool,
    last_text: &mut bool,
) {
    for op in ops {
        match op {
            Op::Html(html) => {
                out.push_str(html);
                *last_text = false;
            }
            Op::Text(expr) => {
                let value = if let Some(value) = expr.borrowed(state, scope) {
                    display(value)
                } else {
                    display(&expr.eval_in(state, scope))
                };
                if !value.is_empty() {
                    if react && *last_text {
                        out.push_str("<!-- -->");
                    }
                    escape_into(&value, out);
                    *last_text = true;
                }
            }
            Op::Attribute(name, expr) => attribute(name, &expr.eval_in(state, scope), out),
            Op::Style(name, expr) => style(name, &expr.eval_in(state, scope), out),
            Op::When(condition, yes, no) => write(
                if truthy(&condition.eval_in(state, scope)) {
                    yes
                } else {
                    no
                },
                state,
                scope,
                out,
                react,
                last_text,
            ),
            Op::Each(items, name, index, body) => {
                let owned;
                let value = if let Some(value) = items.borrowed(state, scope) {
                    value
                } else {
                    owned = items.eval_in(state, scope);
                    &owned
                };
                if let Some(items) = value.as_array() {
                    for (i, item) in items.iter().enumerate() {
                        let position = json!(i);
                        let child = Scope {
                            parent: scope,
                            bindings: &[(name, item), (index, &position)],
                        };
                        write(body, state, &child, out, react, last_text);
                    }
                }
            }
        }
    }
}

pub struct RenderPlan {
    pub(crate) wire: Arc<Value>,
    ops: Vec<Op>,
    renderer: Renderer,
}
impl RenderPlan {
    pub fn new(node: &Node) -> Result<Self, String> {
        Self::for_renderer(node, Renderer::Html)
    }
    pub fn for_renderer(node: &Node, renderer: Renderer) -> Result<Self, String> {
        node.validate()?;
        let resolved;
        let node = if renderer == Renderer::Html {
            resolved = crate::frontend::html_tree(node)?;
            &resolved
        } else {
            node
        };
        let mut wire = serde_json::to_value(node).map_err(|e| e.to_string())?;
        annotate(&mut wire);
        let mut ops = vec![];
        compile(node, &mut ops, renderer == Renderer::React);
        if renderer == Renderer::React {
            crate::frontend::translate(&mut wire)?;
        }
        Ok(Self {
            wire: Arc::new(wire),
            ops,
            renderer,
        })
    }
    pub fn to_json(&self) -> String {
        self.wire.to_string()
    }
    pub fn render(&self, state: &dyn StateView) -> String {
        let mut output = String::with_capacity(4096);
        write(
            &self.ops,
            state,
            &Map::new(),
            &mut output,
            self.renderer == Renderer::React,
            &mut false,
        );
        output
    }
}
