//! query endpoint stub (real impl in T10).
use axum::Json;
pub async fn run(Json(_v): Json<serde_json::Value>) -> &'static str { "stub" }
