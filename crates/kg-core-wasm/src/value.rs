//! Property value bindings.
//!
//! `prop_*` functions return a JS value that is the serde-serialized form of
//! `kg_core::PropValue` (tagged enum: `{kind:"int",value:42}` etc). These can
//! be nested in JS — e.g. `prop_list([prop_int(1), prop_int(2)])`. UoW methods
//! deserialize them back to `PropValue` via serde-wasm-bindgen.

use kg_core::value::PropValue;
use wasm_bindgen::prelude::*;

/// Type alias for clarity in TS bindings — `WasmPropValue` is just the JS-side
/// JSON form of a `PropValue`.
pub type WasmPropValue = JsValue;

fn to_js(v: PropValue) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&v).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Convert a JS-side PropValue (object with {kind, value}) back to Rust.
pub(crate) fn from_js(v: JsValue) -> Result<PropValue, JsValue> {
    serde_wasm_bindgen::from_value(v).map_err(|e| JsValue::from_str(&format!("not a PropValue: {e}")))
}

#[wasm_bindgen]
pub fn prop_null() -> Result<JsValue, JsValue> { to_js(PropValue::Null) }

#[wasm_bindgen]
pub fn prop_bool(v: bool) -> Result<JsValue, JsValue> { to_js(PropValue::Bool(v)) }

#[wasm_bindgen]
pub fn prop_int(v: i64) -> Result<JsValue, JsValue> { to_js(PropValue::Int(v)) }

#[wasm_bindgen]
pub fn prop_float(v: f64) -> Result<JsValue, JsValue> { to_js(PropValue::Float(v)) }

#[wasm_bindgen]
pub fn prop_string(v: String) -> Result<JsValue, JsValue> { to_js(PropValue::String(v)) }

/// Construct a List from a JS array of nested PropValue JS objects.
#[wasm_bindgen]
pub fn prop_list(items: js_sys::Array) -> Result<JsValue, JsValue> {
    let mut out = Vec::with_capacity(items.length() as usize);
    for entry in items.iter() {
        out.push(from_js(entry)?);
    }
    to_js(PropValue::List(out))
}

/// Construct a Map from a JS array of [string, PropValue] tuples.
#[wasm_bindgen]
pub fn prop_map(entries: js_sys::Array) -> Result<JsValue, JsValue> {
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
        out.insert(key, from_js(tuple.get(1))?);
    }
    to_js(PropValue::Map(out))
}
