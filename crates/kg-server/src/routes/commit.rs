use axum::{extract::State, Json};
use kg_core::cypher::Statement;
use kg_core::value::PropValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CommitRequest {
    pub ddl: Vec<StatementJson>,
    pub data: Option<StatementJson>,
}

#[derive(Deserialize)]
pub struct StatementJson {
    pub cypher: String,
    #[serde(default)]
    pub params: BTreeMap<String, PropValue>,
}

impl From<StatementJson> for Statement {
    fn from(s: StatementJson) -> Statement {
        Statement { cypher: s.cypher, params: s.params }
    }
}

#[derive(Serialize, Default)]
pub struct CommitResponse {
    pub ok: bool,
}

pub async fn run(
    State(s): State<AppState>,
    Json(req): Json<CommitRequest>,
) -> Result<Json<CommitResponse>, ApiError> {
    for ddl in req.ddl {
        let stmt = Statement::from(ddl);
        s.client.run_autocommit_for_ddl(&stmt).await.map_err(ApiError::from)?;
    }
    if let Some(data) = req.data {
        let stmt = Statement::from(data);
        s.client.run_tx_for_data(&stmt).await.map_err(ApiError::from)?;
    }
    Ok(Json(CommitResponse { ok: true }))
}
