use axum::{extract::State, Json};
use kg_core::value::PropValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct QueryRequest {
    pub cypher: String,
    #[serde(default)]
    pub params: BTreeMap<String, PropValue>,
}

#[derive(Serialize)]
pub struct QueryResponse {
    pub rows: Vec<BTreeMap<String, PropValue>>,
}

pub async fn run(
    State(s): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, ApiError> {
    let rows = s.client.query::<BTreeMap<String, PropValue>>(
        &req.cypher,
        req.params.into_iter().collect::<Vec<_>>(),
    ).await.map_err(ApiError::from)?;
    Ok(Json(QueryResponse { rows }))
}
