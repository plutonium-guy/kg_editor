use kg_core::value::PropValue;
use kg_neo4j::{auth::basic, ClientBuilder, WebSysHttpClient};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub async fn run() -> Result<(), JsValue> {
    let client = ClientBuilder::new("http://localhost:7474")
        .auth(basic("neo4j", "test"))
        .database("neo4j")
        .build_with_http(WebSysHttpClient)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut uow = client.unit_of_work();
    let a = uow.create_node(["Person"], [("name", PropValue::from("Alice"))]);
    let b = uow.create_node(["Person"], [("name", PropValue::from("Bob"))]);
    uow.create_rel(a, b, "KNOWS", [] as [(String, PropValue); 0]);
    client.commit(uow).await.map_err(|e| JsValue::from_str(&e.to_string()))?;
    web_sys::console::log_1(&"committed".into());
    Ok(())
}
