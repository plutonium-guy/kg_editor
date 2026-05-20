//! Property value bindings.

use kg_core::value::PropValue;
use wasm_bindgen::prelude::*;

/// Opaque handle to a `PropValue`. JS code constructs via the `prop_*`
/// free functions and passes handles into UoW methods.
#[wasm_bindgen]
pub struct WasmPropValue {
    pub(crate) inner: PropValue,
}

impl From<PropValue> for WasmPropValue {
    fn from(v: PropValue) -> Self { WasmPropValue { inner: v } }
}

#[wasm_bindgen]
pub fn prop_null() -> WasmPropValue { PropValue::Null.into() }

#[wasm_bindgen]
pub fn prop_bool(v: bool) -> WasmPropValue { PropValue::Bool(v).into() }

#[wasm_bindgen]
pub fn prop_int(v: i64) -> WasmPropValue { PropValue::Int(v).into() }

#[wasm_bindgen]
pub fn prop_float(v: f64) -> WasmPropValue { PropValue::Float(v).into() }

#[wasm_bindgen]
pub fn prop_string(v: String) -> WasmPropValue { PropValue::String(v).into() }

/// Construct a List from a JS array of PropValue objects (deserialized from JSON-compatible JS values).
#[wasm_bindgen]
pub fn prop_list(items: js_sys::Array) -> Result<WasmPropValue, JsValue> {
    let mut out = Vec::with_capacity(items.length() as usize);
    for entry in items.iter() {
        let v: PropValue = serde_wasm_bindgen::from_value(entry)
            .map_err(|e| JsValue::from_str(&format!("prop_list item deserialization failed: {e}")))?;
        out.push(v);
    }
    Ok(PropValue::List(out).into())
}

/// Construct a Map from a JS array of [string, PropValue] tuples.
#[wasm_bindgen]
pub fn prop_map(entries: js_sys::Array) -> Result<WasmPropValue, JsValue> {
    use std::collections::BTreeMap;
    let mut out = BTreeMap::new();
    for entry in entries.iter() {
        let tuple: js_sys::Array = entry.dyn_into()
            .map_err(|_| JsValue::from_str("each entry must be [string, PropValue]"))?;
        if tuple.length() != 2 {
            return Err(JsValue::from_str("each entry must have exactly 2 elements"));
        }
        let key = tuple.get(0).as_string()
            .ok_or_else(|| JsValue::from_str("entry[0] must be a string"))?;
        let val: PropValue = serde_wasm_bindgen::from_value(tuple.get(1))
            .map_err(|e| JsValue::from_str(&format!("prop_map value deserialization failed: {e}")))?;
        out.insert(key, val);
    }
    Ok(PropValue::Map(out).into())
}
