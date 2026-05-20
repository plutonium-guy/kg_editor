//! Schema error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("Schema error")]
    Unknown,
}
