//! Persistent fields: copying a state only copies Arc handles; writes copy one field.
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::{Index, IndexMut},
    sync::Arc,
};

pub trait StateView {
    fn field(&self, name: &str) -> Option<&Value>;
}
impl StateView for Value {
    fn field(&self, name: &str) -> Option<&Value> {
        self.get(name)
    }
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(transparent)]
pub struct NativeState {
    fields: BTreeMap<String, Arc<Value>>,
    #[serde(skip)]
    dirty: BTreeSet<String>,
}
impl NativeState {
    pub fn from_value(value: Value) -> Result<Self, String> {
        let Value::Object(fields) = value else {
            return Err("State must be an object".into());
        };
        let dirty = fields.keys().cloned().collect();
        Ok(Self {
            fields: fields.into_iter().map(|(k, v)| (k, Arc::new(v))).collect(),
            dirty,
        })
    }
    pub fn to_value(&self) -> Value {
        Value::Object(
            self.fields
                .iter()
                .map(|(k, v)| (k.clone(), v.as_ref().clone()))
                .collect(),
        )
    }
    pub fn fork(&self) -> Self {
        Self {
            fields: self.fields.clone(),
            dirty: BTreeSet::new(),
        }
    }
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.fields.get(name).map(Arc::as_ref)
    }
    pub fn get_arc(&self, name: &str) -> Option<&Arc<Value>> {
        self.fields.get(name)
    }
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.dirty.insert(name.into());
        self.fields.get_mut(name).map(Arc::make_mut)
    }
    pub fn insert(&mut self, name: &str, value: Value) {
        self.dirty.insert(name.into());
        self.fields.insert(name.into(), Arc::new(value));
    }
    pub(crate) fn insert_arc(&mut self, name: &str, value: Arc<Value>) {
        self.dirty.insert(name.into());
        self.fields.insert(name.into(), value);
    }
    pub fn remove(&mut self, name: &str) {
        self.dirty.insert(name.into());
        self.fields.remove(name);
    }
    pub fn fields(&self) -> &BTreeMap<String, Arc<Value>> {
        &self.fields
    }
    pub fn dirty_fields(&self) -> &BTreeSet<String> {
        &self.dirty
    }
    pub fn same_field(&self, name: &str, other: &Self) -> bool {
        match (self.get_arc(name), other.get_arc(name)) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b) || a == b,
            (None, None) => true,
            _ => false,
        }
    }
}
impl StateView for NativeState {
    fn field(&self, name: &str) -> Option<&Value> {
        self.get(name)
    }
}
impl Index<&str> for NativeState {
    type Output = Value;
    fn index(&self, name: &str) -> &Value {
        self.get(name).unwrap_or(&Value::Null)
    }
}
impl IndexMut<&str> for NativeState {
    fn index_mut(&mut self, name: &str) -> &mut Value {
        self.dirty.insert(name.into());
        Arc::make_mut(
            self.fields
                .entry(name.into())
                .or_insert_with(|| Arc::new(Value::Null)),
        )
    }
}
