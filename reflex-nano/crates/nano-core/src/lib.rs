//! Reflex Nano: Rust-owned UI trees, reactive expressions and session state.
mod assets;
mod demo;
mod frontend;
mod model;
mod render;
mod runtime;
mod server;
mod state;
mod websocket;
pub use demo::{benchmark_application, dashboard_application};
pub use frontend::Renderer;
pub use model::{Event, Expr, Node, NodeKind};
pub use render::RenderPlan;
pub use runtime::{
    Action, Application, EventMode, Field, PageDefinition, Parameter, Program, Schema, ValueType,
};
pub use serde_json::{json, Value};
pub use server::{App, Effect, EventContext, EventResult, StateLease};
pub use state::{NativeState, StateView};
#[cfg(test)]
mod tests;
