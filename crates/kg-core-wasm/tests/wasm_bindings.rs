#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

use js_sys::Array;
use kg_core_wasm::{prop_int, prop_string, WasmUow};
use wasm_bindgen::JsValue;

fn props(entries: Vec<(&str, JsValue)>) -> Array {
    let arr = Array::new();
    for (k, v) in entries {
        let pair = Array::new();
        pair.push(&JsValue::from_str(k));
        pair.push(&v);
        arr.push(&pair);
    }
    arr
}

#[wasm_bindgen_test]
fn create_node_returns_local_id_and_emits_data() {
    let mut uow = WasmUow::new();
    let alice = uow
        .create_node(vec!["Person".into()], props(vec![("name", prop_string("Alice".into()).unwrap())]))
        .unwrap();
    let bob = uow
        .create_node(vec!["Person".into()], props(vec![("name", prop_string("Bob".into()).unwrap())]))
        .unwrap();
    assert_ne!(alice, bob);

    let out_js = uow.emit().unwrap();
    let s = js_sys::JSON::stringify(&out_js).unwrap().as_string().unwrap();
    assert!(s.contains("CREATE (n_1:`Person`"), "missing n_1 Person CREATE, got: {s}");
    assert!(s.contains("CREATE (n_2:`Person`"), "missing n_2 Person CREATE, got: {s}");
    assert!(s.contains("WITH *"), "missing WITH * between clauses, got: {s}");
}

#[wasm_bindgen_test]
fn create_rel_using_local_refs() {
    let mut uow = WasmUow::new();
    let a = uow.create_node(vec!["P".into()], Array::new()).unwrap();
    let b = uow.create_node(vec!["P".into()], Array::new()).unwrap();

    let start_js = serde_wasm_bindgen::to_value(&serde_json::json!({"kind":"local","id":a})).unwrap();
    let end_js   = serde_wasm_bindgen::to_value(&serde_json::json!({"kind":"local","id":b})).unwrap();

    let _r = uow
        .create_rel(start_js, end_js, "KNOWS".into(), props(vec![("since", prop_int(2020).unwrap())]))
        .unwrap();
    let s = js_sys::JSON::stringify(&uow.emit().unwrap()).unwrap().as_string().unwrap();
    assert!(s.contains("CREATE (n_1)-[r_3:`KNOWS`"), "missing rel CREATE, got: {s}");
    assert!(s.contains("->(n_2)"), "missing rel endpoint, got: {s}");
}
