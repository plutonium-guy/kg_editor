use axum::{extract::State, Json};
use kg_schema::SchemaFile;
use crate::state::AppState;

pub async fn run(State(s): State<AppState>) -> Json<SchemaFile> {
    Json((*s.schema).clone())
}
