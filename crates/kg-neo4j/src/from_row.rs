//! Trait for decoding a single Cypher row into a user struct.

use kg_core::value::PropValue;
use std::collections::BTreeMap;

pub trait FromRow: Sized {
    fn from_row(row: &BTreeMap<String, PropValue>) -> Result<Self, RowError>;
}

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
