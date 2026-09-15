use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
};
use reflex_nano::{
    Action, Application, Event, EventMode, Expr, Node, Parameter, Program, Schema, Value, ValueType,
};

fn invalid(e: impl ToString) -> PyErr {
    PyValueError::new_err(e.to_string())
}
fn parse(s: &str) -> PyResult<Value> {
    serde_json::from_str(s).map_err(invalid)
}

#[pyclass(name = "Expr", module = "reflex_nano._native", frozen)]
#[derive(Clone)]
struct NativeExpr {
    inner: Expr,
}
#[pymethods]
impl NativeExpr {
    #[staticmethod]
    fn from_json(source: &str) -> PyResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(source).map_err(invalid)?,
        })
    }
    fn trim(&self) -> Self {
        Self {
            inner: self.inner.clone().trim(),
        }
    }
    fn count_where(&self, name: &str, predicate: PyRef<'_, Self>) -> Self {
        Self {
            inner: self
                .inner
                .clone()
                .count_where(name, predicate.inner.clone()),
        }
    }
    #[staticmethod]
    fn literal(json: &str) -> PyResult<Self> {
        Ok(Self {
            inner: Expr::literal(parse(json)?),
        })
    }
    #[staticmethod]
    fn state(name: String) -> Self {
        Self {
            inner: Expr::state(name),
        }
    }
    #[staticmethod]
    fn local(name: String) -> Self {
        Self {
            inner: Expr::local(name),
        }
    }
    fn get(&self, key: PyRef<'_, Self>) -> Self {
        Self {
            inner: self.inner.clone().at(key.inner.clone()),
        }
    }
    fn binary(&self, operator: &str, right: PyRef<'_, Self>) -> PyResult<Self> {
        if ![
            "add", "sub", "mul", "div", "mod", "eq", "ne", "gt", "ge", "lt", "le", "and", "or",
        ]
        .contains(&operator)
        {
            return Err(invalid("Unknown operator"));
        }
        Ok(Self {
            inner: self.inner.clone().binary(operator, right.inner.clone()),
        })
    }
    fn unary(&self, operator: &str) -> PyResult<Self> {
        Ok(Self {
            inner: match operator {
                "not" => self.inner.clone().negate(),
                "length" => self.inner.clone().length(),
                _ => return Err(invalid("Unknown operator")),
            },
        })
    }
    fn choose(&self, yes: PyRef<'_, Self>, no: PyRef<'_, Self>) -> Self {
        Self {
            inner: self
                .inner
                .clone()
                .choose(yes.inner.clone(), no.inner.clone()),
        }
    }
    #[staticmethod]
    fn concat(py: Python<'_>, parts: Vec<Py<NativeExpr>>) -> Self {
        Self {
            inner: Expr::concat(parts.iter().map(|p| p.borrow(py).inner.clone())),
        }
    }
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(invalid)
    }
    fn evaluate(&self, state_json: &str) -> PyResult<String> {
        Ok(self
            .inner
            .eval(&parse(state_json)?, &serde_json::Map::new())
            .to_string())
    }
}

#[pyclass(name = "Node", module = "reflex_nano._native", frozen)]
#[derive(Clone)]
struct NativeNode {
    inner: Node,
}
#[pymethods]
impl NativeNode {
    #[staticmethod]
    fn component(
        py: Python<'_>,
        library: &str,
        export_name: &str,
        children: Vec<Py<NativeNode>>,
    ) -> Self {
        Self {
            inner: Node::component(
                library,
                export_name,
                children.iter().map(|n| n.borrow(py).inner.clone()),
            ),
        }
    }
    fn fallback(&self, node: PyRef<'_, NativeNode>) -> Self {
        Self {
            inner: self.inner.clone().fallback(node.inner.clone()),
        }
    }
    #[staticmethod]
    fn text(value: PyRef<'_, NativeExpr>) -> Self {
        Self {
            inner: Node::text(value.inner.clone()),
        }
    }
    #[staticmethod]
    fn element(py: Python<'_>, tag: String, children: Vec<Py<NativeNode>>) -> Self {
        Self {
            inner: Node::el(tag, children.iter().map(|n| n.borrow(py).inner.clone())),
        }
    }
    #[staticmethod]
    fn fragment(py: Python<'_>, children: Vec<Py<NativeNode>>) -> Self {
        Self {
            inner: Node::fragment(children.iter().map(|n| n.borrow(py).inner.clone())),
        }
    }
    #[staticmethod]
    fn when(
        condition: PyRef<'_, NativeExpr>,
        yes: PyRef<'_, NativeNode>,
        no: PyRef<'_, NativeNode>,
    ) -> Self {
        Self {
            inner: Node::when(condition.inner.clone(), yes.inner.clone(), no.inner.clone()),
        }
    }
    #[staticmethod]
    fn each(
        items: PyRef<'_, NativeExpr>,
        name: &str,
        index: &str,
        body: PyRef<'_, NativeNode>,
    ) -> Self {
        Self {
            inner: Node::each(items.inner.clone(), name, index, body.inner.clone()),
        }
    }
    fn attr(&self, name: &str, value: PyRef<'_, NativeExpr>) -> Self {
        Self {
            inner: self.inner.clone().attr(name, value.inner.clone()),
        }
    }
    fn style(&self, name: &str, value: PyRef<'_, NativeExpr>) -> Self {
        Self {
            inner: self.inner.clone().style(name, value.inner.clone()),
        }
    }
    fn key(&self, value: PyRef<'_, NativeExpr>) -> Self {
        Self {
            inner: self.inner.clone().key(value.inner.clone()),
        }
    }
    #[pyo3(signature=(event,name,args,prevent_default=false,*,debounce_ms=0,throttle_ms=0,temporal=false,stop_propagation=false))]
    fn on(
        &self,
        py: Python<'_>,
        event: &str,
        name: &str,
        args: Vec<Py<NativeExpr>>,
        prevent_default: bool,
        debounce_ms: u32,
        throttle_ms: u32,
        temporal: bool,
        stop_propagation: bool,
    ) -> Self {
        Self {
            inner: self.inner.clone().on(
                event,
                Event {
                    name: name.into(),
                    args: args.iter().map(|a| a.borrow(py).inner.clone()).collect(),
                    prevent_default,
                    debounce_ms,
                    throttle_ms,
                    temporal,
                    stop_propagation,
                },
            ),
        }
    }
    fn render(&self, state_json: &str) -> PyResult<String> {
        self.inner.validate().map_err(invalid)?;
        Ok(self.inner.render(&parse(state_json)?))
    }
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(invalid)
    }
}

#[pyclass(name = "Action", module = "reflex_nano._native", frozen)]
#[derive(Clone)]
struct NativeAction {
    inner: Action,
}
#[pymethods]
impl NativeAction {
    #[staticmethod]
    fn from_json(source: &str) -> PyResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(source).map_err(invalid)?,
        })
    }
    #[staticmethod]
    #[pyo3(signature=(field,value,path=Vec::new()))]
    fn set(
        py: Python<'_>,
        field: String,
        value: PyRef<'_, NativeExpr>,
        path: Vec<Py<NativeExpr>>,
    ) -> Self {
        Self {
            inner: Action::Set {
                field,
                path: path.iter().map(|p| p.borrow(py).inner.clone()).collect(),
                value: value.inner.clone(),
            },
        }
    }
    #[staticmethod]
    fn append(field: String, value: PyRef<'_, NativeExpr>) -> Self {
        Self {
            inner: Action::Append {
                field,
                value: value.inner.clone(),
            },
        }
    }
    #[staticmethod]
    fn remove(field: String, index: PyRef<'_, NativeExpr>) -> Self {
        Self {
            inner: Action::Remove {
                field,
                index: index.inner.clone(),
            },
        }
    }
    #[staticmethod]
    fn require(condition: PyRef<'_, NativeExpr>, message: String) -> Self {
        Self {
            inner: Action::Require {
                condition: condition.inner.clone(),
                message,
            },
        }
    }
    #[staticmethod]
    #[pyo3(signature=(name,args=Vec::new()))]
    fn call(py: Python<'_>, name: String, args: Vec<Py<NativeExpr>>) -> Self {
        Self {
            inner: Action::Call {
                name,
                args: args.iter().map(|p| p.borrow(py).inner.clone()).collect(),
            },
        }
    }
    #[staticmethod]
    fn repeat(
        py: Python<'_>,
        times: PyRef<'_, NativeExpr>,
        actions: Vec<Py<NativeAction>>,
    ) -> Self {
        Self {
            inner: Action::Repeat {
                times: times.inner.clone(),
                actions: actions.iter().map(|p| p.borrow(py).inner.clone()).collect(),
            },
        }
    }
    #[staticmethod]
    fn publish() -> Self {
        Self {
            inner: Action::Publish,
        }
    }
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(invalid)
    }
}
#[pyclass(name = "Program", module = "reflex_nano._native", frozen)]
#[derive(Clone)]
struct NativeProgram {
    inner: Program,
}
#[pymethods]
impl NativeProgram {
    #[new]
    #[pyo3(signature=(actions,parameters_json="[]",mode_json="{\"kind\":\"immediate\"}"))]
    fn new(
        py: Python<'_>,
        actions: Vec<Py<NativeAction>>,
        parameters_json: &str,
        mode_json: &str,
    ) -> PyResult<Self> {
        let parameters: Vec<Parameter> = serde_json::from_str(parameters_json).map_err(invalid)?;
        let mode: EventMode = serde_json::from_str(mode_json).map_err(invalid)?;
        Ok(Self {
            inner: Program {
                actions: actions.iter().map(|a| a.borrow(py).inner.clone()).collect(),
                parameters,
                mode,
            },
        })
    }
    #[staticmethod]
    fn from_json(source: &str) -> PyResult<Self> {
        Ok(Self {
            inner: serde_json::from_str(source).map_err(invalid)?,
        })
    }
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(invalid)
    }
}
#[pyclass(name = "Schema", module = "reflex_nano._native")]
#[derive(Clone)]
struct NativeSchema {
    inner: Schema,
}
#[pymethods]
impl NativeSchema {
    #[new]
    fn new() -> Self {
        Self {
            inner: Schema::default(),
        }
    }
    fn field(&mut self, name: &str, kind: &str, initial_json: &str) -> PyResult<()> {
        let kind: ValueType =
            serde_json::from_value(Value::String(kind.into())).map_err(invalid)?;
        self.inner
            .field(name, kind, parse(initial_json)?)
            .map_err(invalid)
    }
    fn computed(&mut self, name: &str, value: PyRef<'_, NativeExpr>) -> PyResult<()> {
        self.inner
            .computed(name, value.inner.clone())
            .map_err(invalid)
    }
    fn event(&mut self, name: &str, program: PyRef<'_, NativeProgram>) -> PyResult<()> {
        self.inner
            .event(name, program.inner.clone())
            .map_err(invalid)
    }
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(invalid)
    }
}
#[pyclass(name = "App", module = "reflex_nano._native")]
struct NativeApp {
    inner: Application,
}
#[pymethods]
impl NativeApp {
    #[new]
    #[pyo3(signature=(schema=None,*,renderer="html"))]
    fn new(schema: Option<PyRef<'_, NativeSchema>>, renderer: &str) -> PyResult<Self> {
        let mut inner = Application::new(schema.map(|s| s.inner.clone()).unwrap_or_default());
        inner.renderer = renderer.parse().map_err(invalid)?;
        Ok(Self { inner })
    }
    #[getter]
    fn renderer(&self) -> &'static str {
        self.inner.renderer.as_str()
    }
    fn set_renderer(&mut self, renderer: &str) -> PyResult<()> {
        let mut next = self.inner.clone();
        next.renderer = renderer.parse().map_err(invalid)?;
        next.build().map_err(invalid)?;
        self.inner = next;
        Ok(())
    }
    #[staticmethod]
    #[pyo3(signature=(*,renderer="html"))]
    fn dashboard(renderer: &str) -> PyResult<Self> {
        let mut inner = reflex_nano::dashboard_application().map_err(invalid)?;
        inner.renderer = renderer.parse().map_err(invalid)?;
        Ok(Self { inner })
    }
    #[staticmethod]
    #[pyo3(signature=(rows=100,*,renderer="html"))]
    fn benchmark(rows: usize, renderer: &str) -> PyResult<Self> {
        let mut inner = reflex_nano::benchmark_application(rows).map_err(invalid)?;
        inner.renderer = renderer.parse().map_err(invalid)?;
        Ok(Self { inner })
    }
    #[staticmethod]
    fn from_json(source: &str) -> PyResult<Self> {
        let inner = Application::from_json(source).map_err(invalid)?;
        inner.build().map_err(invalid)?;
        Ok(Self { inner })
    }
    fn to_json(&self) -> PyResult<String> {
        self.inner.to_json().map_err(invalid)
    }
    #[pyo3(signature=(route,title,node,*,renderer=None))]
    fn add_page(
        &mut self,
        route: &str,
        title: &str,
        node: PyRef<'_, NativeNode>,
        renderer: Option<&str>,
    ) -> PyResult<()> {
        let mut next = self.inner.clone();
        if let Some(renderer) = renderer {
            next.page_with_renderer(
                route,
                title,
                node.inner.clone(),
                renderer.parse().map_err(invalid)?,
            );
        } else {
            next.page(route, title, node.inner.clone());
        }
        next.build().map_err(invalid)?;
        self.inner = next;
        Ok(())
    }
    #[pyo3(signature=(path="/"))]
    fn render(&self, py: Python<'_>, path: &str) -> PyResult<String> {
        py.detach(|| self.inner.build()?.render(path))
            .map_err(invalid)
    }
    #[pyo3(signature=(path="/"))]
    fn tree_json(&self, path: &str) -> PyResult<String> {
        self.inner
            .build()
            .and_then(|app| app.tree_json(path))
            .map_err(invalid)
    }
    #[pyo3(signature=(path="/"))]
    fn render_plan_json(&self, path: &str) -> PyResult<String> {
        self.inner
            .build()
            .and_then(|app| app.render_plan_json(path))
            .map_err(invalid)
    }
    fn initial_state_json(&self) -> PyResult<String> {
        Ok(self
            .inner
            .build()
            .map_err(invalid)?
            .initial_state()
            .to_string())
    }
    #[pyo3(signature=(name,args_json="[]",state_json=None,path="/"))]
    fn dispatch(
        &self,
        py: Python<'_>,
        name: &str,
        args_json: &str,
        state_json: Option<&str>,
        path: &str,
    ) -> PyResult<String> {
        let app = self.inner.build().map_err(invalid)?;
        let state = state_json
            .map(parse)
            .transpose()?
            .unwrap_or_else(|| app.initial_state());
        let args: Vec<Value> = serde_json::from_str(args_json).map_err(invalid)?;
        let (state, view, effects) = py
            .detach(|| app.dispatch(&state, name, &args, path))
            .map_err(invalid)?;
        Ok(serde_json::json!({"state":state,"view":view,"effects":effects}).to_string())
    }
    #[pyo3(signature=(host="127.0.0.1",port=3000,*,renderer=None))]
    fn run(&self, py: Python<'_>, host: &str, port: u16, renderer: Option<&str>) -> PyResult<()> {
        // Drop all binding references before serving; the GIL stays released.
        let mut definition = self.inner.clone();
        if let Some(renderer) = renderer {
            definition.renderer = renderer.parse().map_err(invalid)?;
        }
        let app = definition.build().map_err(invalid)?;
        py.detach(move || {
            let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
            runtime.block_on(app.serve(host, port))
        })
        .map_err(PyRuntimeError::new_err)
    }
}
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<NativeExpr>()?;
    m.add_class::<NativeNode>()?;
    m.add_class::<NativeAction>()?;
    m.add_class::<NativeProgram>()?;
    m.add_class::<NativeSchema>()?;
    m.add_class::<NativeApp>()?;
    m.add("__version__", "0.4.0")?;
    Ok(())
}
