//! Trait for decoding a single Cypher row into a user struct.

use kg_core::value::PropValue;
use std::collections::BTreeMap;

/// Decoder for a single Cypher row into a user struct. The blanket impl on `BTreeMap<String, PropValue>` returns the raw row map.
pub trait FromRow: Sized {
    fn from_row(row: &BTreeMap<String, PropValue>) -> Result<Self, RowError>;
}

/// Errors that may occur while decoding a row into a user struct.
#[derive(Debug, thiserror::Error)]
pub enum RowError {
    #[error("missing column `{0}`")]
    Missing(String),
    #[error("wrong type for `{col}`: expected {expected}, got {got}")]
    WrongType { col: String, expected: &'static str, got: &'static str },
}

impl FromRow for BTreeMap<String, PropValue> {
    fn from_row(row: &BTreeMap<String, PropValue>) -> Result<Self, RowError> {
        Ok(row.clone())
    }
}
