//! commit endpoint stub (real impl in T11).
use axum::Json;
pub async fn run(Json(_v): Json<serde_json::Value>) -> &'static str { "stub" }
