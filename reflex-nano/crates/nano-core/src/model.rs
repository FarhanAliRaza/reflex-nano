use crate::StateView;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::{collections::BTreeMap, sync::Arc};

/// Borrowed lexical scopes avoid cloning list items during native rendering.
pub(crate) trait Locals {
    fn get(&self, name: &str) -> Option<&Value>;
}
impl Locals for Map<String, Value> {
    fn get(&self, name: &str) -> Option<&Value> {
        Map::get(self, name)
    }
}
pub(crate) struct Scope<'a> {
    pub parent: &'a dyn Locals,
    pub bindings: &'a [(&'a str, &'a Value)],
}
impl Locals for Scope<'_> {
    fn get(&self, name: &str) -> Option<&Value> {
        self.bindings
            .iter()
            .rev()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| *value)
            .or_else(|| self.parent.get(name))
    }
}

/// Expressions are data, never executable JavaScript or Python source.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Expr {
    Literal {
        value: Value,
    },
    State {
        name: String,
    },
    Local {
        name: String,
    },
    Get {
        value: Arc<Expr>,
        key: Arc<Expr>,
    },
    Binary {
        operator: String,
        left: Arc<Expr>,
        right: Arc<Expr>,
    },
    Not {
        value: Arc<Expr>,
    },
    Length {
        value: Arc<Expr>,
    },
    Concat {
        parts: Vec<Expr>,
    },
    Select {
        condition: Arc<Expr>,
        yes: Arc<Expr>,
        no: Arc<Expr>,
    },
    Trim {
        value: Arc<Expr>,
    },
    Object {
        fields: BTreeMap<String, Expr>,
    },
    Count {
        items: Arc<Expr>,
        name: String,
        predicate: Arc<Expr>,
    },
}

impl Expr {
    pub fn trim(self) -> Self {
        Self::Trim {
            value: Arc::new(self),
        }
    }
    pub fn object(fields: impl IntoIterator<Item = (String, Expr)>) -> Self {
        Self::Object {
            fields: fields.into_iter().collect(),
        }
    }
    pub fn count_where(self, name: &str, predicate: Expr) -> Self {
        Self::Count {
            items: Arc::new(self),
            name: name.into(),
            predicate: Arc::new(predicate),
        }
    }
    pub fn dependencies(&self) -> std::collections::BTreeSet<String> {
        let mut result = std::collections::BTreeSet::new();
        match self {
            Self::State { name } => {
                result.insert(name.clone());
            }
            Self::Get { value, key } => {
                result.extend(value.dependencies());
                result.extend(key.dependencies());
            }
            Self::Binary { left, right, .. } => {
                result.extend(left.dependencies());
                result.extend(right.dependencies());
            }
            Self::Not { value } | Self::Length { value } | Self::Trim { value } => {
                result.extend(value.dependencies())
            }
            Self::Concat { parts } => {
                for p in parts {
                    result.extend(p.dependencies());
                }
            }
            Self::Select { condition, yes, no } => {
                result.extend(condition.dependencies());
                result.extend(yes.dependencies());
                result.extend(no.dependencies());
            }
            Self::Count {
                items, predicate, ..
            } => {
                result.extend(items.dependencies());
                result.extend(predicate.dependencies());
            }
            Self::Object { fields } => {
                for p in fields.values() {
                    result.extend(p.dependencies());
                }
            }
            _ => {}
        }
        result
    }
    /// Free lexical variables; count predicates bind their item variable.
    pub fn locals(&self) -> std::collections::BTreeSet<String> {
        use std::collections::BTreeSet;
        let mut out = BTreeSet::new();
        match self {
            Self::Local { name } => {
                out.insert(name.clone());
            }
            Self::Get { value, key } => {
                out.extend(value.locals());
                out.extend(key.locals());
            }
            Self::Binary { left, right, .. } => {
                out.extend(left.locals());
                out.extend(right.locals());
            }
            Self::Not { value } | Self::Length { value } | Self::Trim { value } => {
                out.extend(value.locals())
            }
            Self::Concat { parts } => {
                for p in parts {
                    out.extend(p.locals());
                }
            }
            Self::Select { condition, yes, no } => {
                out.extend(condition.locals());
                out.extend(yes.locals());
                out.extend(no.locals());
            }
            Self::Object { fields } => {
                for p in fields.values() {
                    out.extend(p.locals());
                }
            }
            Self::Count {
                items,
                name,
                predicate,
            } => {
                out.extend(items.locals());
                let mut bound = predicate.locals();
                bound.remove(name);
                out.extend(bound);
            }
            _ => {}
        }
        out
    }
    pub(crate) fn borrowed<'a>(
        &'a self,
        state: &'a dyn StateView,
        locals: &'a dyn Locals,
    ) -> Option<&'a Value> {
        match self {
            Self::Literal { value } => Some(value),
            Self::State { name } => state.field(name),
            Self::Local { name } => locals.get(name),
            Self::Get { value, key } => {
                let source = value.borrowed(state, locals)?;
                match key.eval_in(state, locals) {
                    Value::String(k) => source.get(&k),
                    Value::Number(k) => k.as_u64().and_then(|i| source.get(i as usize)),
                    _ => None,
                }
            }
            _ => None,
        }
    }
    pub fn state(name: impl Into<String>) -> Self {
        Self::State { name: name.into() }
    }
    pub fn local(name: impl Into<String>) -> Self {
        Self::Local { name: name.into() }
    }
    pub fn literal(value: impl Into<Value>) -> Self {
        Self::Literal {
            value: value.into(),
        }
    }
    pub fn at(self, key: impl Into<Expr>) -> Self {
        Self::Get {
            value: Arc::new(self),
            key: Arc::new(key.into()),
        }
    }
    pub fn binary(self, operator: &str, right: impl Into<Expr>) -> Self {
        Self::Binary {
            operator: operator.into(),
            left: Arc::new(self),
            right: Arc::new(right.into()),
        }
    }
    pub fn gt(self, rhs: impl Into<Expr>) -> Self {
        self.binary("gt", rhs)
    }
    pub fn eq_to(self, rhs: impl Into<Expr>) -> Self {
        self.binary("eq", rhs)
    }
    pub fn plus(self, rhs: impl Into<Expr>) -> Self {
        self.binary("add", rhs)
    }
    pub fn length(self) -> Self {
        Self::Length {
            value: Arc::new(self),
        }
    }
    pub fn negate(self) -> Self {
        Self::Not {
            value: Arc::new(self),
        }
    }
    pub fn concat(parts: impl IntoIterator<Item = Expr>) -> Self {
        Self::Concat {
            parts: parts.into_iter().collect(),
        }
    }
    pub fn choose(self, yes: impl Into<Expr>, no: impl Into<Expr>) -> Self {
        Self::Select {
            condition: Arc::new(self),
            yes: Arc::new(yes.into()),
            no: Arc::new(no.into()),
        }
    }
    pub fn eval(&self, state: &dyn StateView, locals: &Map<String, Value>) -> Value {
        self.eval_in(state, locals)
    }
    pub(crate) fn eval_in(&self, state: &dyn StateView, locals: &dyn Locals) -> Value {
        match self {
            Self::Literal { value } => value.clone(),
            Self::State { name } => state.field(name).cloned().unwrap_or(Value::Null),
            Self::Local { name } => locals.get(name).cloned().unwrap_or(Value::Null),
            Self::Get { value, key } => {
                if let Some(value) = self.borrowed(state, locals) {
                    return value.clone();
                }
                let value = value.eval_in(state, locals);
                match key.eval_in(state, locals) {
                    Value::String(k) => value.get(k).cloned().unwrap_or(Value::Null),
                    Value::Number(k) => k
                        .as_u64()
                        .and_then(|i| value.get(i as usize))
                        .cloned()
                        .unwrap_or(Value::Null),
                    _ => Value::Null,
                }
            }
            Self::Not { value } => json!(!truthy(&value.eval_in(state, locals))),
            Self::Trim { value } => json!(display(&value.eval_in(state, locals)).trim()),
            Self::Object { fields } => Value::Object(
                fields
                    .iter()
                    .map(|(k, v)| (k.clone(), v.eval_in(state, locals)))
                    .collect(),
            ),
            Self::Count {
                items,
                name,
                predicate,
            } => {
                let owned;
                let values = if let Some(value) = items.borrowed(state, locals) {
                    value
                } else {
                    owned = items.eval_in(state, locals);
                    &owned
                };
                let mut count = 0;
                if let Some(values) = values.as_array() {
                    for value in values {
                        let scope = Scope {
                            parent: locals,
                            bindings: &[(name, value)],
                        };
                        if truthy(&predicate.eval_in(state, &scope)) {
                            count += 1;
                        }
                    }
                }
                json!(count)
            }
            Self::Length { value } => {
                let owned;
                let value = if let Some(v) = value.borrowed(state, locals) {
                    v
                } else {
                    owned = value.eval_in(state, locals);
                    &owned
                };
                json!(match value {
                    Value::Array(v) => v.len(),
                    Value::Object(v) => v.len(),
                    Value::String(v) => v.chars().count(),
                    _ => 0,
                })
            }
            Self::Concat { parts } => Value::String(
                parts
                    .iter()
                    .map(|e| display(&e.eval_in(state, locals)))
                    .collect(),
            ),
            Self::Select { condition, yes, no } => if truthy(&condition.eval_in(state, locals)) {
                yes
            } else {
                no
            }
            .eval_in(state, locals),
            Self::Binary {
                operator,
                left,
                right,
            } => {
                let l = left.eval_in(state, locals);
                let r = right.eval_in(state, locals);
                if let (Some(a), Some(b)) = (l.as_i64(), r.as_i64()) {
                    let integer = match operator.as_str() {
                        "add" => a.checked_add(b),
                        "sub" => a.checked_sub(b),
                        "mul" => a.checked_mul(b),
                        "mod" => a.checked_rem(b),
                        _ => None,
                    };
                    if let Some(value) = integer {
                        return json!(value);
                    }
                }
                match operator.as_str() {
                    "eq" => json!(equal(&l, &r)),
                    "ne" => json!(!equal(&l, &r)),
                    "and" => json!(truthy(&l) && truthy(&r)),
                    "or" => json!(truthy(&l) || truthy(&r)),
                    "add" if l.is_string() || r.is_string() => json!(display(&l) + &display(&r)),
                    op => {
                        let (Some(a), Some(b)) = (l.as_f64(), r.as_f64()) else {
                            return Value::Null;
                        };
                        match op {
                            "add" => json!(a + b),
                            "sub" => json!(a - b),
                            "mul" => json!(a * b),
                            "div" if b != 0.0 => json!(a / b),
                            "mod" if b != 0.0 => json!(a % b),
                            "gt" => json!(a > b),
                            "ge" => json!(a >= b),
                            "lt" => json!(a < b),
                            "le" => json!(a <= b),
                            _ => Value::Null,
                        }
                    }
                }
            }
        }
    }
}
fn equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| equal(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len() && a.iter().all(|(k, v)| b.get(k).is_some_and(|b| equal(v, b)))
        }
        _ => a == b,
    }
}
impl From<&str> for Expr {
    fn from(v: &str) -> Self {
        Self::literal(v)
    }
}
impl From<String> for Expr {
    fn from(v: String) -> Self {
        Self::literal(v)
    }
}
impl From<i64> for Expr {
    fn from(v: i64) -> Self {
        Self::literal(v)
    }
}
impl From<bool> for Expr {
    fn from(v: bool) -> Self {
        Self::literal(v)
    }
}
impl From<Value> for Expr {
    fn from(v: Value) -> Self {
        Self::literal(v)
    }
}

pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(v) => *v,
        Value::Number(v) => v.as_f64() != Some(0.0),
        Value::String(v) => !v.is_empty(),
        Value::Array(v) => !v.is_empty(),
        Value::Object(v) => !v.is_empty(),
    }
}
pub fn display(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(v) => v.clone(),
        Value::Number(n) if n.is_f64() && n.as_f64().unwrap().fract() == 0.0 => {
            format!("{:.0}", n.as_f64().unwrap())
        }
        _ => v.to_string(),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    pub args: Vec<Expr>,
    pub prevent_default: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub debounce_ms: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub throttle_ms: u32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub temporal: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub stop_propagation: bool,
}
fn is_zero(value: &u32) -> bool {
    *value == 0
}
fn is_false(value: &bool) -> bool {
    !*value
}
impl Event {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args: vec![],
            prevent_default: false,
            debounce_ms: 0,
            throttle_ms: 0,
            temporal: false,
            stop_propagation: false,
        }
    }
    pub fn arg(mut self, value: impl Into<Expr>) -> Self {
        self.args.push(value.into());
        self
    }
    pub fn prevent_default(mut self) -> Self {
        self.prevent_default = true;
        self
    }
    pub fn debounce(mut self, milliseconds: u32) -> Self {
        self.debounce_ms = milliseconds;
        self
    }
    pub fn throttle(mut self, milliseconds: u32) -> Self {
        self.throttle_ms = milliseconds;
        self
    }
    pub fn temporal(mut self) -> Self {
        self.temporal = true;
        self
    }
    pub fn stop_propagation(mut self) -> Self {
        self.stop_propagation = true;
        self
    }
}

/// Cheap clones share immutable Rust-owned trees, including across Python calls.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Node(pub Arc<NodeKind>);
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NodeKind {
    Component {
        library: String,
        export_name: String,
        attrs: BTreeMap<String, Expr>,
        styles: BTreeMap<String, Expr>,
        events: BTreeMap<String, Event>,
        children: Vec<Node>,
        key: Option<Expr>,
        fallback: Option<Node>,
    },
    Text {
        value: Expr,
    },
    Element {
        tag: String,
        attrs: BTreeMap<String, Expr>,
        styles: BTreeMap<String, Expr>,
        events: BTreeMap<String, Event>,
        children: Vec<Node>,
        key: Option<Expr>,
    },
    When {
        condition: Expr,
        yes: Node,
        no: Node,
    },
    Each {
        items: Expr,
        name: String,
        index: String,
        body: Node,
    },
    Fragment {
        children: Vec<Node>,
    },
}
impl Node {
    pub fn component(
        library: impl Into<String>,
        export_name: impl Into<String>,
        children: impl IntoIterator<Item = Node>,
    ) -> Self {
        Self(Arc::new(NodeKind::Component {
            library: library.into(),
            export_name: export_name.into(),
            attrs: BTreeMap::new(),
            styles: BTreeMap::new(),
            events: BTreeMap::new(),
            children: children.into_iter().collect(),
            key: None,
            fallback: None,
        }))
    }
    pub fn fallback(mut self, node: Node) -> Self {
        if let NodeKind::Component { fallback, .. } = Arc::make_mut(&mut self.0) {
            *fallback = Some(node);
        }
        self
    }
    pub fn text(value: impl Into<Expr>) -> Self {
        Self(Arc::new(NodeKind::Text {
            value: value.into(),
        }))
    }
    pub fn el(tag: impl Into<String>, children: impl IntoIterator<Item = Node>) -> Self {
        Self(Arc::new(NodeKind::Element {
            tag: tag.into(),
            attrs: BTreeMap::new(),
            styles: BTreeMap::new(),
            events: BTreeMap::new(),
            children: children.into_iter().collect(),
            key: None,
        }))
    }
    pub fn fragment(children: impl IntoIterator<Item = Node>) -> Self {
        Self(Arc::new(NodeKind::Fragment {
            children: children.into_iter().collect(),
        }))
    }
    pub fn when(condition: Expr, yes: Node, no: Node) -> Self {
        Self(Arc::new(NodeKind::When { condition, yes, no }))
    }
    pub fn each(items: Expr, name: &str, index: &str, body: Node) -> Self {
        Self(Arc::new(NodeKind::Each {
            items,
            name: name.into(),
            index: index.into(),
            body,
        }))
    }
    pub fn attr(mut self, name: impl Into<String>, value: impl Into<Expr>) -> Self {
        if let NodeKind::Element { attrs, .. } | NodeKind::Component { attrs, .. } =
            Arc::make_mut(&mut self.0)
        {
            attrs.insert(name.into(), value.into());
        }
        self
    }
    pub fn style(mut self, name: impl Into<String>, value: impl Into<Expr>) -> Self {
        if let NodeKind::Element { styles, .. } | NodeKind::Component { styles, .. } =
            Arc::make_mut(&mut self.0)
        {
            styles.insert(name.into(), value.into());
        }
        self
    }
    pub fn on(mut self, name: impl Into<String>, event: Event) -> Self {
        if let NodeKind::Element { events, .. } | NodeKind::Component { events, .. } =
            Arc::make_mut(&mut self.0)
        {
            events.insert(name.into(), event);
        }
        self
    }
    pub fn key(mut self, value: impl Into<Expr>) -> Self {
        if let NodeKind::Element { key, .. } | NodeKind::Component { key, .. } =
            Arc::make_mut(&mut self.0)
        {
            *key = Some(value.into());
        }
        self
    }
    pub fn validate(&self) -> Result<(), String> {
        match self.0.as_ref() {
            NodeKind::Component {
                library,
                export_name,
                attrs,
                styles,
                events,
                children,
                fallback,
                ..
            } => {
                if library.is_empty() || export_name.is_empty() {
                    return Err("Component library/export must be nonempty".into());
                }
                for name in attrs.keys() {
                    if !safe_attr(name)
                        || matches!(
                            name.as_str(),
                            "ref" | "key" | "children" | "dangerouslySetInnerHTML"
                        )
                    {
                        return Err(format!("Unsupported component prop: {name}"));
                    }
                }
                for name in styles.keys() {
                    if !safe_name(name) {
                        return Err(format!("Invalid style name: {name}"));
                    }
                }
                for name in events.keys() {
                    if !safe_name(name)
                        || !name.is_ascii()
                        || !name.starts_with(|c: char| c.is_ascii_alphabetic())
                    {
                        return Err(format!("Invalid component event: {name}"));
                    }
                }
                for child in children {
                    child.validate()?;
                }
                if let Some(fallback) = fallback {
                    fallback.validate()?;
                }
            }
            NodeKind::Element {
                tag,
                attrs,
                styles,
                events,
                children,
                ..
            } => {
                if !safe_name(tag)
                    || tag != &tag.to_ascii_lowercase()
                    || matches!(
                        tag.as_str(),
                        "script"
                            | "style"
                            | "iframe"
                            | "object"
                            | "embed"
                            | "base"
                            | "meta"
                            | "link"
                    )
                {
                    return Err(format!("Unsupported HTML tag: {tag}"));
                }
                for name in attrs.keys() {
                    if !safe_attr(name) {
                        return Err(format!("Unsupported attribute: {name}"));
                    }
                }
                for name in styles.keys() {
                    if !safe_name(name) {
                        return Err(format!("Invalid style name: {name}"));
                    }
                }
                for name in events.keys() {
                    if !matches!(
                        name.as_str(),
                        "click"
                            | "input"
                            | "change"
                            | "submit"
                            | "keydown"
                            | "keyup"
                            | "focus"
                            | "blur"
                    ) {
                        return Err(format!("Unsupported event: {name}"));
                    }
                }
                for child in children {
                    child.validate()?;
                }
            }
            NodeKind::When { yes, no, .. } => {
                yes.validate()?;
                no.validate()?;
            }
            NodeKind::Each { body, .. } => body.validate()?,
            NodeKind::Fragment { children } => {
                for child in children {
                    child.validate()?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub fn render(&self, state: &dyn StateView) -> String {
        let mut output = String::new();
        self.write_html(state, &Map::new(), &mut output);
        output
    }
    fn write_html(&self, state: &dyn StateView, locals: &Map<String, Value>, out: &mut String) {
        match self.0.as_ref() {
            NodeKind::Component { fallback, .. } => {
                if let Some(fallback) = fallback {
                    fallback.write_html(state, locals, out);
                }
            }
            NodeKind::Text { value } => escape_into(&display(&value.eval(state, locals)), out),
            NodeKind::Fragment { children } => {
                for child in children {
                    child.write_html(state, locals, out);
                }
            }
            NodeKind::When { condition, yes, no } => if truthy(&condition.eval(state, locals)) {
                yes
            } else {
                no
            }
            .write_html(state, locals, out),
            NodeKind::Each {
                items,
                name,
                index,
                body,
            } => {
                if let Value::Array(values) = items.eval(state, locals) {
                    for (i, item) in values.into_iter().enumerate() {
                        let mut scope = locals.clone();
                        scope.insert(name.clone(), item);
                        scope.insert(index.clone(), json!(i));
                        body.write_html(state, &scope, out);
                    }
                }
            }
            NodeKind::Element {
                tag,
                attrs,
                styles,
                children,
                ..
            } => {
                out.push('<');
                out.push_str(tag);
                for (name, value) in attrs {
                    let value = value.eval(state, locals);
                    if value.is_null() || (is_bool_attr(name) && !truthy(&value)) {
                        continue;
                    }
                    if !safe_attr(name) || (is_url_attr(name) && !safe_url(&display(&value))) {
                        continue;
                    }
                    if tag == "textarea" && name == "value" {
                        continue;
                    }
                    out.push(' ');
                    out.push_str(name);
                    out.push_str("=\"");
                    escape_into(&display(&value), out);
                    out.push('"');
                }
                if !styles.is_empty() {
                    out.push_str(" style=\"");
                    for (name, value) in styles {
                        let value = value.eval(state, locals);
                        if value.is_null() {
                            continue;
                        }
                        out.push_str(name);
                        out.push(':');
                        escape_into(&display(&value), out);
                        out.push(';');
                    }
                    out.push('"');
                }
                out.push('>');
                if !matches!(
                    tag.as_str(),
                    "area" | "br" | "col" | "hr" | "img" | "input" | "source" | "track" | "wbr"
                ) {
                    if tag == "textarea" {
                        if let Some(value) = attrs.get("value") {
                            escape_into(&display(&value.eval(state, locals)), out);
                        }
                    }
                    for child in children {
                        child.write_html(state, locals, out);
                    }
                    out.push_str("</");
                    out.push_str(tag);
                    out.push('>');
                }
            }
        }
    }
}
pub fn safe_name(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b':'))
}
pub fn safe_attr(s: &str) -> bool {
    safe_name(s)
        && !s.to_ascii_lowercase().starts_with("on")
        && !matches!(
            s.to_ascii_lowercase().as_str(),
            "srcdoc" | "style" | "innerhtml" | "formaction"
        )
}
pub fn is_bool_attr(s: &str) -> bool {
    matches!(
        s,
        "disabled"
            | "checked"
            | "selected"
            | "multiple"
            | "required"
            | "readonly"
            | "autofocus"
            | "hidden"
            | "open"
    )
}
pub fn is_url_attr(s: &str) -> bool {
    matches!(
        s.to_ascii_lowercase().as_str(),
        "href" | "src" | "action" | "xlink:href" | "poster"
    )
}
pub fn safe_url(s: &str) -> bool {
    let s = s.trim();
    if s.chars().any(|c| c.is_control()) {
        return false;
    }
    match s.split_once(':') {
        Some((scheme, _))
            if !scheme.contains('/') && !scheme.contains('#') && !scheme.contains('?') =>
        {
            matches!(
                scheme.to_ascii_lowercase().as_str(),
                "http" | "https" | "mailto" | "tel"
            )
        }
        _ => true,
    }
}
pub fn escape_into(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
}
