use axum::{extract::State, Json};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub neo4j: &'static str,
}

pub async fn health(State(s): State<AppState>) -> Json<HealthResponse> {
    let neo4j = match s.client.query::<std::collections::BTreeMap<String, kg_core::value::PropValue>>(
        "RETURN 1 AS x",
        Vec::<(String, kg_core::value::PropValue)>::new(),
    ).await {
        Ok(_)  => "reachable",
        Err(_) => "down",
    };
    Json(HealthResponse { status: "ok", neo4j })
}
