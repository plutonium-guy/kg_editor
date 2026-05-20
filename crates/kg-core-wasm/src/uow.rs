//! Unit of Work bindings.

use crate::value::from_js;
use js_sys::Array;
use kg_core::node::{LocalId, NodeId, NodeRef};
use kg_core::rel::RelId;
use kg_core::uow::{CascadeRule, PropPatch, UnitOfWork};
use kg_core::value::PropValue;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// JS-side discriminator for a NodeRef.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NodeRefJs {
    Server { id: i64 },
    Local  { id: u64 },
}

impl From<NodeRefJs> for NodeRef {
    fn from(v: NodeRefJs) -> NodeRef {
        match v {
            NodeRefJs::Server { id } => NodeRef::Server(NodeId(id)),
            NodeRefJs::Local  { id } => NodeRef::Local(LocalId(id)),
        }
    }
}

#[wasm_bindgen]
pub struct WasmUow {
    inner: UnitOfWork,
}

#[wasm_bindgen]
impl WasmUow {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmUow { WasmUow { inner: UnitOfWork::new() } }

    pub fn create_node(&mut self, labels: Vec<String>, props: Array) -> Result<u64, JsValue> {
        let p = props_from_array(props)?;
        Ok(self.inner.create_node(labels, p).0)
    }

    pub fn merge_node(&mut self, labels: Vec<String>, key_props: Array, set_props: Array) -> Result<u64, JsValue> {
        let key = props_from_array(key_props)?;
        let set = props_from_array(set_props)?;
        Ok(self.inner.merge_node(labels, key, set).0)
    }

    pub fn update_node(&mut self, server_id: i64, sets: Array, unsets: Vec<String>) -> Result<(), JsValue> {
        let mut patch = PropPatch::new();
        for (k, v) in props_from_array(sets)? {
            patch = patch.set(k, v);
        }
        for k in unsets {
            patch = patch.unset(k);
        }
        self.inner.update_node(NodeId(server_id), patch);
        Ok(())
    }

    pub fn delete_node(&mut self, server_id: i64, detach: bool) {
        let cascade = if detach { CascadeRule::Detach } else { CascadeRule::Strict };
        self.inner.delete_node(NodeId(server_id), cascade);
    }

    pub fn create_rel(&mut self, start: JsValue, end: JsValue, ty: String, props: Array) -> Result<u64, JsValue> {
        let s: NodeRefJs = serde_wasm_bindgen::from_value(start).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let e: NodeRefJs = serde_wasm_bindgen::from_value(end).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let p = props_from_array(props)?;
        Ok(self.inner.create_rel(NodeRef::from(s), NodeRef::from(e), ty, p).0)
    }

    pub fn merge_rel(&mut self, start: JsValue, end: JsValue, ty: String, key_props: Array, set_props: Array) -> Result<u64, JsValue> {
        let s: NodeRefJs = serde_wasm_bindgen::from_value(start).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let e: NodeRefJs = serde_wasm_bindgen::from_value(end).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let key = props_from_array(key_props)?;
        let set = props_from_array(set_props)?;
        Ok(self.inner.merge_rel(NodeRef::from(s), NodeRef::from(e), ty, key, set).0)
    }

    pub fn update_rel(&mut self, rel_id: i64, sets: Array, unsets: Vec<String>) -> Result<(), JsValue> {
        let mut patch = PropPatch::new();
        for (k, v) in props_from_array(sets)? {
            patch = patch.set(k, v);
        }
        for k in unsets {
            patch = patch.unset(k);
        }
        self.inner.update_rel(RelId(rel_id), patch);
        Ok(())
    }

    pub fn delete_rel(&mut self, rel_id: i64) {
        self.inner.delete_rel(RelId(rel_id));
    }

    /// Returns the EmitOutput { ddl, data } as a plain JS object.
    pub fn emit(&self) -> Result<JsValue, JsValue> {
        let out = kg_core::cypher::CypherEmitter::emit(&self.inner)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&out).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

impl Default for WasmUow {
    fn default() -> Self { Self::new() }
}

fn props_from_array(arr: Array) -> Result<Vec<(String, PropValue)>, JsValue> {
    let mut out = Vec::with_capacity(arr.length() as usize);
    for entry in arr.iter() {
        let tuple: Array = entry.dyn_into().map_err(|_| JsValue::from_str("each prop must be [string, PropValue]"))?;
        if tuple.length() != 2 {
            return Err(JsValue::from_str("each prop must have exactly 2 elements"));
        }
        let key = tuple.get(0).as_string().ok_or_else(|| JsValue::from_str("prop[0] must be string"))?;
        let val = from_js(tuple.get(1))?;
        out.push((key, val));
    }
    Ok(out)
}
