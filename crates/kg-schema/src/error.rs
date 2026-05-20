//! Schema parsing errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("schema invalid: {0}")]
    Invalid(String),
}
