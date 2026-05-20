//! Schema registry bindings (minimal phase-1 surface).

use kg_core::schema::{NodeSchema, PropType, SchemaRegistry};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmSchemaRegistry {
    inner: SchemaRegistry,
}

#[wasm_bindgen]
impl WasmSchemaRegistry {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self { inner: SchemaRegistry::new() } }

    /// Phase-1 convenience: declare a node label with a list of (name, "String"|"Int"|"Float"|"Bool")
    /// required props. More expressive bindings come in phase 2.
    pub fn add_node_simple(&mut self, label: String, props: js_sys::Array) -> Result<(), JsValue> {
        let mut builder = NodeSchema::builder(&label);
        for entry in props.iter() {
            let pair: js_sys::Array = entry.dyn_into().map_err(|_| JsValue::from_str("each prop must be [name, type]"))?;
            if pair.length() != 2 {
                return Err(JsValue::from_str("prop must be [name, type]"));
            }
            let name = pair.get(0).as_string().ok_or_else(|| JsValue::from_str("name must be string"))?;
            let ty_str = pair.get(1).as_string().ok_or_else(|| JsValue::from_str("type must be string"))?;
            let ty = match ty_str.as_str() {
                "String" => PropType::String,
                "Int"    => PropType::Int,
                "Float"  => PropType::Float,
                "Bool"   => PropType::Bool,
                other    => return Err(JsValue::from_str(&format!("unsupported type `{other}`"))),
            };
            builder = builder.prop(name, ty).required();
        }
        self.inner.add_node(builder.build());
        Ok(())
    }
}

impl Default for WasmSchemaRegistry {
    fn default() -> Self { Self::new() }
}
