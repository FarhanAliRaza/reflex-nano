//! Native frontend selection and translation of the shared UI IR to React props.
use crate::{Node, NodeKind};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Renderer {
    #[default]
    Html,
    React,
}
impl std::str::FromStr for Renderer {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "html" => Ok(Self::Html),
            "react" => Ok(Self::React),
            _ => Err("Renderer must be 'html' or 'react'".into()),
        }
    }
}
impl Renderer {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::React => "react",
        }
    }
}

pub(crate) fn html_tree(node: &Node) -> Result<Node, String> {
    let mut node = node.clone();
    match Arc::make_mut(&mut node.0) {
        NodeKind::Component {
            library,
            export_name,
            fallback,
            ..
        } => {
            return html_tree(fallback.as_ref().ok_or_else(|| {
                format!("{library}:{export_name} needs an explicit fallback for the HTML renderer")
            })?);
        }
        NodeKind::Element { children, .. } | NodeKind::Fragment { children } => {
            for child in children {
                *child = html_tree(child)?;
            }
        }
        NodeKind::Each { body, .. } => *body = html_tree(body)?,
        NodeKind::When { yes, no, .. } => {
            *yes = html_tree(yes)?;
            *no = html_tree(no)?;
        }
        _ => {}
    }
    Ok(node)
}
pub(crate) fn react_prop(name: &str) -> String {
    match name {
        "class" => "className".into(),
        "for" => "htmlFor".into(),
        "tabindex" => "tabIndex".into(),
        "readonly" => "readOnly".into(),
        "autofocus" => "autoFocus".into(),
        "maxlength" => "maxLength".into(),
        "colspan" => "colSpan".into(),
        "rowspan" => "rowSpan".into(),
        _ if name.starts_with("aria-") || name.starts_with("data-") || name.starts_with("--") => {
            name.into()
        }
        _ => camel(name),
    }
}
fn camel(name: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for c in name.chars() {
        if c == '_' || c == '-' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}
pub(crate) fn react_event(name: &str) -> String {
    match name {
        "keydown" => "onKeyDown".into(),
        "keyup" => "onKeyUp".into(),
        "dblclick" => "onDoubleClick".into(),
        _ if name.starts_with("on")
            && name.chars().nth(2).is_some_and(|c| c.is_ascii_uppercase()) =>
        {
            name.into()
        }
        _ => {
            let name = camel(name.strip_prefix("on_").unwrap_or(name));
            format!("on{}{}", name[..1].to_ascii_uppercase(), &name[1..])
        }
    }
}
/// Translate descriptors in Rust; the React adapter consumes these exact props.
pub(crate) fn translate(wire: &mut Value) -> Result<(), String> {
    match wire["kind"].as_str().unwrap() {
        "element" | "component" => {
            if wire["kind"] == "component" {
                let registry: Value =
                    serde_json::from_str(include_str!("../frontend-dist/registry.json")).unwrap();
                let library = wire["library"].as_str().unwrap();
                let name = wire["export_name"].as_str().unwrap();
                if !registry[library]
                    .as_array()
                    .is_some_and(|names| names.iter().any(|v| v == name))
                {
                    return Err(format!("React component {library}:{name} is not registered; rebuild the frontend registry"));
                }
                if !wire["fallback"].is_null() {
                    translate(&mut wire["fallback"])?;
                }
            }
            for field in ["attrs", "styles", "events"] {
                let old = wire[field].take();
                wire[field] = old
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| {
                        (
                            if field == "events" {
                                react_event(k)
                            } else {
                                react_prop(k)
                            },
                            v.clone(),
                        )
                    })
                    .collect();
            }
            for child in wire["children"].as_array_mut().unwrap() {
                translate(child)?;
            }
        }
        "fragment" => {
            for child in wire["children"].as_array_mut().unwrap() {
                translate(child)?;
            }
        }
        "when" => {
            translate(&mut wire["yes"])?;
            translate(&mut wire["no"])?;
        }
        "each" => translate(&mut wire["body"])?,
        _ => {}
    }
    Ok(())
}
