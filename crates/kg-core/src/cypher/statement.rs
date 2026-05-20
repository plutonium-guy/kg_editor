//! Statement and parameter representation.

use crate::value::PropValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Parameter map. Keys are bare names (no leading `$`).
pub type ParamMap = BTreeMap<String, PropValue>;

/// A single parameterized Cypher statement.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Statement {
    pub cypher: String,
    pub params: ParamMap,
}

impl Statement {
    pub fn new<I, K>(cypher: impl Into<String>, params: I) -> Self
    where
        I: IntoIterator<Item = (K, PropValue)>,
        K: Into<String>,
    {
        Statement {
            cypher: cypher.into(),
            params: params.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        }
    }
}

/// Reject anything that would let an attacker break out of an
/// identifier context (label, rel type, prop name).
pub fn sanitize_ident(s: &str) -> Result<&str, IdentError> {
    if s.is_empty() { return Err(IdentError::Empty); }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(IdentError::BadStart(first));
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return Err(IdentError::BadChar(c));
        }
    }
    Ok(s)
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum IdentError {
    #[error("identifier is empty")]
    Empty,
    #[error("identifier cannot start with `{0}`")]
    BadStart(char),
    #[error("identifier contains illegal char `{0}`")]
    BadChar(char),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::PropValue;

    #[test]
    fn statement_holds_cypher_and_params() {
        let s = Statement::new("RETURN $x", [("x", PropValue::Int(1))]);
        assert_eq!(s.cypher, "RETURN $x");
        assert_eq!(s.params.get("x"), Some(&PropValue::Int(1)));
    }

    #[test]
    fn sanitize_ident_rejects_injection() {
        assert!(sanitize_ident("Person").is_ok());
        assert!(sanitize_ident("Per_son1").is_ok());
        assert!(sanitize_ident("1bad").is_err());
        assert!(sanitize_ident("a-b").is_err());
        assert!(sanitize_ident("a`b").is_err());
        assert!(sanitize_ident("").is_err());
    }
}
