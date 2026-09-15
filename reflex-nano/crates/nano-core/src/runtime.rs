//! Native state schemas and event programs. No Python objects or callbacks.
use crate::{App, Effect, EventContext, Expr, NativeState, Node, StateLease};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{collections::BTreeMap, sync::Arc, time::Duration};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    Json,
    Bool,
    Int,
    Float,
    String,
    List,
    Object,
}
impl ValueType {
    pub fn validate(&self, value: &Value) -> bool {
        match self {
            Self::Json => true,
            Self::Bool => value.is_boolean(),
            Self::Int => value.as_i64().is_some(),
            Self::Float => value.is_number(),
            Self::String => value.is_string(),
            Self::List => value.is_array(),
            Self::Object => value.is_object(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Field {
    pub kind: ValueType,
    pub initial: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub kind: ValueType,
}

/// Programs are native data interpreted by Rust; expression evaluation stays in Rust.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Action {
    Set {
        field: String,
        #[serde(default)]
        path: Vec<Expr>,
        value: Expr,
    },
    Append {
        field: String,
        value: Expr,
    },
    Remove {
        field: String,
        index: Expr,
    },
    Require {
        condition: Expr,
        message: String,
    },
    Call {
        name: String,
        #[serde(default)]
        args: Vec<Expr>,
    },
    Repeat {
        times: Expr,
        actions: Vec<Action>,
    },
    Publish,
    Alert {
        message: Expr,
    },
    Redirect {
        path: Expr,
    },
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventMode {
    #[default]
    Immediate,
    Streaming,
    Deferred {
        milliseconds: u64,
    },
    Background {
        milliseconds: u64,
        times: Expr,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Program {
    #[serde(default)]
    pub parameters: Vec<Parameter>,
    pub actions: Vec<Action>,
    #[serde(default)]
    pub mode: EventMode,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Schema {
    #[serde(default)]
    pub fields: BTreeMap<String, Field>,
    #[serde(default)]
    pub computed: BTreeMap<String, Expr>,
    #[serde(default)]
    pub events: BTreeMap<String, Program>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageDefinition {
    pub route: String,
    pub title: String,
    pub tree: Node,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renderer: Option<crate::Renderer>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Application {
    #[serde(default)]
    pub renderer: crate::Renderer,
    pub schema: Schema,
    pub pages: Vec<PageDefinition>,
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('_')
        && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}
pub(crate) fn safe_json(value: &Value) -> bool {
    match value {
        Value::Number(v) => v
            .as_f64()
            .is_some_and(|n| n.is_finite() && n.abs() <= 9_007_199_254_740_991.0),
        Value::Array(v) => v.iter().all(safe_json),
        Value::Object(v) => v.values().all(safe_json),
        _ => true,
    }
}
impl Schema {
    pub fn field(&mut self, name: &str, kind: ValueType, initial: Value) -> Result<(), String> {
        if !valid_name(name) || self.fields.contains_key(name) || self.computed.contains_key(name) {
            return Err(format!("Invalid or duplicate field: {name}"));
        }
        if !kind.validate(&initial) || !safe_json(&initial) {
            return Err(format!("Invalid initial value: {name}"));
        }
        self.fields.insert(name.into(), Field { kind, initial });
        Ok(())
    }
    pub fn computed(&mut self, name: &str, expression: Expr) -> Result<(), String> {
        if !valid_name(name) || self.fields.contains_key(name) || self.computed.contains_key(name) {
            return Err(format!("Invalid or duplicate computed field: {name}"));
        }
        self.computed.insert(name.into(), expression);
        Ok(())
    }
    pub fn event(&mut self, name: &str, program: Program) -> Result<(), String> {
        if name.is_empty() || self.events.contains_key(name) {
            return Err(format!("Invalid or duplicate event: {name}"));
        }
        let mut names = std::collections::BTreeSet::new();
        for parameter in &program.parameters {
            if !valid_name(&parameter.name) || !names.insert(&parameter.name) {
                return Err("Invalid event parameters".into());
            }
        }
        self.events.insert(name.into(), program);
        Ok(())
    }
    fn initial(&self) -> Value {
        Value::Object(
            self.fields
                .iter()
                .map(|(k, v)| (k.clone(), v.initial.clone()))
                .collect(),
        )
    }
    fn validate(&self, state: &NativeState) -> Result<(), String> {
        for name in state.dirty_fields() {
            let Some(field) = self.fields.get(name) else {
                continue;
            };
            if !state
                .get(name)
                .is_some_and(|v| field.kind.validate(v) && safe_json(v))
            {
                return Err(format!("Invalid state field: {name}"));
            }
        }
        Ok(())
    }
    fn locals(&self, name: &str, args: &[Value]) -> Result<Map<String, Value>, String> {
        let program = self
            .events
            .get(name)
            .ok_or_else(|| format!("Unknown event: {name}"))?;
        if args.len() != program.parameters.len() {
            return Err("Wrong event argument count".into());
        }
        program
            .parameters
            .iter()
            .zip(args)
            .map(|(p, v)| {
                if !p.kind.validate(v) || !safe_json(v) {
                    Err(format!("Invalid argument: {}", p.name))
                } else {
                    Ok((p.name.clone(), v.clone()))
                }
            })
            .collect()
    }
    fn execute(
        &self,
        actions: &[Action],
        state: &mut NativeState,
        locals: &Map<String, Value>,
        effects: &mut Vec<Effect>,
        lease: &mut Option<&mut StateLease>,
        fuel: &mut usize,
        depth: usize,
    ) -> Result<(), String> {
        if depth > 32 {
            return Err("Event nesting limit exceeded".into());
        }
        for action in actions {
            *fuel = fuel
                .checked_sub(1)
                .ok_or("Event instruction limit exceeded")?;
            match action {
                Action::Set { field, path, value } => {
                    if !self.fields.contains_key(field) {
                        return Err(format!("Unknown state field: {field}"));
                    }
                    let value = value.eval(state, locals);
                    let keys: Vec<_> = path.iter().map(|p| p.eval(state, locals)).collect();
                    let mut target = state.get_mut(field).ok_or("Missing state field")?;
                    for key in keys {
                        target = match key {
                            Value::String(key) => target.get_mut(&key),
                            Value::Number(key) => {
                                key.as_u64().and_then(|i| target.get_mut(i as usize))
                            }
                            _ => None,
                        }
                        .ok_or("State path does not exist")?;
                    }
                    *target = value;
                }
                Action::Append { field, value } => {
                    let value = value.eval(state, locals);
                    state
                        .get_mut(field)
                        .and_then(Value::as_array_mut)
                        .ok_or("Append requires a list field")?
                        .push(value);
                }
                Action::Remove { field, index } => {
                    let index = index
                        .eval(state, locals)
                        .as_u64()
                        .ok_or("Index must be a positive integer")?
                        as usize;
                    let values = state
                        .get_mut(field)
                        .and_then(Value::as_array_mut)
                        .ok_or("Remove requires a list field")?;
                    if index >= values.len() {
                        return Err("List index out of bounds".into());
                    }
                    values.remove(index);
                }
                Action::Require { condition, message } => {
                    if !crate::model::truthy(&condition.eval(state, locals)) {
                        return Err(message.clone());
                    }
                }
                Action::Call { name, args } => {
                    let args: Vec<_> = args.iter().map(|a| a.eval(state, locals)).collect();
                    let called = self.events.get(name).ok_or("Unknown chained event")?;
                    if !matches!(called.mode, EventMode::Immediate) {
                        return Err("Inline calls require immediate events".into());
                    }
                    let locals = self.locals(name, &args)?;
                    self.execute(
                        &called.actions,
                        state,
                        &locals,
                        effects,
                        lease,
                        fuel,
                        depth + 1,
                    )?;
                }
                Action::Repeat { times, actions } => {
                    let times = times
                        .eval(state, locals)
                        .as_u64()
                        .filter(|n| *n <= 10_000)
                        .ok_or("Repeat count must be 0–10000")?;
                    for _ in 0..times {
                        self.execute(actions, state, locals, effects, lease, fuel, depth + 1)?;
                    }
                }
                Action::Publish => {
                    self.validate(state)?;
                    lease
                        .as_deref_mut()
                        .ok_or("Publish requires a streaming event")?
                        .publish_native(state.clone())?;
                }
                Action::Alert { message } => effects.push(Effect::Alert {
                    message: crate::model::display(&message.eval(state, locals)),
                }),
                Action::Redirect { path } => {
                    let path = crate::model::display(&path.eval(state, locals));
                    if !crate::model::safe_url(&path) {
                        return Err("Unsafe redirect URL".into());
                    }
                    effects.push(Effect::Redirect { path });
                }
            }
        }
        Ok(())
    }
    fn run(
        &self,
        name: &str,
        state: &mut NativeState,
        args: &[Value],
        lease: Option<&mut StateLease>,
    ) -> Result<Vec<Effect>, String> {
        let locals = self.locals(name, args)?;
        let mut effects = vec![];
        self.execute(
            &self.events[name].actions,
            state,
            &locals,
            &mut effects,
            &mut { lease },
            &mut 100_000,
            0,
        )?;
        self.validate(state)?;
        Ok(effects)
    }
}

impl Application {
    pub fn new(schema: Schema) -> Self {
        Self {
            schema,
            pages: vec![],
            renderer: crate::Renderer::Html,
        }
    }
    pub fn page(&mut self, route: &str, title: &str, tree: Node) {
        self.pages.push(PageDefinition {
            route: route.into(),
            title: title.into(),
            tree,
            renderer: None,
        });
    }
    pub fn page_with_renderer(
        &mut self,
        route: &str,
        title: &str,
        tree: Node,
        renderer: crate::Renderer,
    ) {
        self.pages.push(PageDefinition {
            route: route.into(),
            title: title.into(),
            tree,
            renderer: Some(renderer),
        });
    }
    pub fn build(&self) -> Result<App, String> {
        // Revalidate deserialized definitions through the same registration path.
        let mut checked = Schema::default();
        for (name, field) in &self.schema.fields {
            checked.field(name, field.kind.clone(), field.initial.clone())?;
        }
        for (name, expr) in &self.schema.computed {
            checked.computed(name, expr.clone())?;
        }
        for (name, program) in &self.schema.events {
            checked.event(name, program.clone())?;
        }
        let schema = Arc::new(checked);
        let mut app = App::new(schema.initial());
        app.renderer = self.renderer;
        for page in &self.pages {
            app.add_page_with_renderer(
                &page.route,
                &page.title,
                page.tree.clone(),
                page.renderer.unwrap_or(self.renderer),
            )?;
        }
        // Dependency order is derived in Rust, including computed-on-computed expressions.
        let mut pending = schema.computed.clone();
        let mut registered = std::collections::BTreeSet::new();
        while !pending.is_empty() {
            let ready: Vec<_> = pending
                .iter()
                .filter(|(_, expr)| {
                    expr.dependencies()
                        .iter()
                        .all(|d| !schema.computed.contains_key(d) || registered.contains(d))
                })
                .map(|(n, _)| n.clone())
                .collect();
            if ready.is_empty() {
                return Err("Computed dependency cycle".into());
            }
            for name in ready {
                let expr = pending.remove(&name).unwrap();
                app.computed_expression(&name, expr)?;
                registered.insert(name);
            }
        }
        for (name, program) in &schema.events {
            let schema = schema.clone();
            let name_owned = name.clone();
            match &program.mode {
                EventMode::Immediate => app.on_native(name, move |state, args| {
                    schema.run(&name_owned, state, args, None)
                })?,
                EventMode::Streaming | EventMode::Deferred { .. } => {
                    let delay = if let EventMode::Deferred { milliseconds } = program.mode {
                        Some(milliseconds)
                    } else {
                        None
                    };
                    app.on_stream(name, move |context, args| {
                        schema.locals(&name_owned, args)?;
                        if let Some(delay) = delay {
                            tokio::runtime::Handle::current().block_on(async {
                                if delay == 0 {
                                    tokio::task::yield_now().await;
                                } else {
                                    tokio::time::sleep(Duration::from_millis(delay.min(60_000)))
                                        .await;
                                }
                            });
                        }
                        let mut lease = context.begin();
                        let mut state = lease.native_state();
                        let effects =
                            schema.run(&name_owned, &mut state, args, Some(&mut lease))?;
                        lease.publish_native(state)?;
                        Ok(effects)
                    })?;
                }
                EventMode::Background {
                    milliseconds,
                    times,
                } => {
                    let milliseconds = *milliseconds;
                    let times = times.clone();
                    app.on_background(name, move |context, args| {
                        let locals = schema.locals(&name_owned, args)?;
                        let times = times
                            .eval(&context.read_native(), &locals)
                            .as_u64()
                            .filter(|n| *n <= 10_000)
                            .ok_or("Background count must be 0–10000")?;
                        let schema = schema.clone();
                        let name = name_owned.clone();
                        let args = args.to_vec();
                        tokio::spawn(async move {
                            background(context, schema, name, args, times, milliseconds).await;
                        });
                        Ok(vec![])
                    })?;
                }
            }
        }
        Ok(app)
    }
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }
    pub fn from_json(source: &str) -> Result<Self, String> {
        serde_json::from_str(source).map_err(|e| e.to_string())
    }
}
async fn background(
    context: EventContext,
    schema: Arc<Schema>,
    name: String,
    args: Vec<Value>,
    times: u64,
    milliseconds: u64,
) {
    for _ in 0..times {
        tokio::time::sleep(Duration::from_millis(milliseconds.min(60_000))).await;
        let schema = schema.clone();
        let name = name.clone();
        let args = args.clone();
        let result = context
            .update_native(move |state| {
                let effects = schema.run(&name, state, &args, None)?;
                if !effects.is_empty() {
                    return Err(
                        "Background programs publish state; effects require a foreground event"
                            .into(),
                    );
                }
                Ok(())
            })
            .await;
        if let Err(error) = result {
            context.report_error(&error);
            break;
        }
    }
    context.finish();
}
