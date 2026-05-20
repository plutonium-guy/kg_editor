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
    Transport(#[from] TransportError),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("conversion {from} -> {to}: {reason}")]
    Conversion { from: &'static str, to: &'static str, reason: String },
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
