//! Public error types.

use kg_core::error::CoreError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Neo4jError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("server error [{code}]: {message}")]
    Tx { code: String, message: String },
    #[error(transparent)]
    Transport(TransportError),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("conversion {from} -> {to}: {reason}")]
    Conversion { from: &'static str, to: &'static str, reason: String },
}

impl From<TransportError> for Neo4jError {
    fn from(e: TransportError) -> Self {
        match e {
            // Surface server-side Neo4j errors (syntax, semantic, etc.) as Tx errors
            // so the HTTP layer can map them to 422 instead of 502.
            TransportError::ServerError { code, message } => {
                Neo4jError::Tx { code, message }
            }
            other => Neo4jError::Transport(other),
        }
    }
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("connect: {0}")]
    Connect(String),
    #[error("timeout")]
    Timeout,
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("http {status}: {body}")]
    Http { status: u16, body: String },
    /// A well-formed server-side Neo4j error carrying the Neo4j error code and message.
    /// This is distinct from a transport/protocol failure and should be surfaced as a
    /// 422 Unprocessable Entity rather than a 502.
    #[error("server error [{code}]: {message}")]
    ServerError { code: String, message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn from_core() {
        let e: Neo4jError = CoreError::CycleInPlan.into();
        assert!(matches!(e, Neo4jError::Core(_)));
    }
    #[test]
    fn display() {
        let e = Neo4jError::Tx { code: "Neo.ClientError.X".into(), message: "boom".into() };
        assert!(format!("{e}").contains("Neo.ClientError.X"));
    }
}
