use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use kg_neo4j::Neo4jError;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.code.as_str() {
            c if c.starts_with("400.") => StatusCode::BAD_REQUEST,
            c if c.starts_with("404.") => StatusCode::NOT_FOUND,
            c if c.starts_with("422.") => StatusCode::UNPROCESSABLE_ENTITY,
            c if c.starts_with("502.") => StatusCode::BAD_GATEWAY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(serde_json::json!({"error": self}))).into_response()
    }
}

impl From<Neo4jError> for ApiError {
    fn from(e: Neo4jError) -> ApiError {
        match e {
            Neo4jError::Core(_) => ApiError {
                code: "400.bad_request".into(), message: e.to_string(),
            },
            Neo4jError::Tx { code, message } => ApiError {
                code: format!("422.{code}"),
                message,
            },
            Neo4jError::Transport(_) => ApiError {
                code: "502.transport".into(), message: e.to_string(),
            },
            Neo4jError::Auth(_) => ApiError {
                code: "500.auth_misconfig".into(), message: e.to_string(),
            },
            Neo4jError::Conversion { .. } => ApiError {
                code: "500.conversion".into(), message: e.to_string(),
            },
        }
    }
}
