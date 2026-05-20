use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::error::ApiError;
use crate::routes::entities::{list, ListParams};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
}
fn default_limit() -> i64 { 20 }

pub async fn run(
    s: State<AppState>,
    Query(p): Query<SearchParams>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    list(s, Query(ListParams { label: None, q: Some(p.q), limit: p.limit })).await
}
