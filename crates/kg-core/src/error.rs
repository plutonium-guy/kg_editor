//! Error and validation-violation types.

use crate::node::LocalId;
use thiserror::Error;

/// Top-level error from pure-logic operations.
#[derive(Debug, Error, PartialEq)]
pub enum CoreError {
    #[error("schema violations: {0:?}")]
    SchemaViolation(Vec<SchemaViolation>),
    #[error("unresolved {0:?}; rel referenced a local node not staged")]
    UnresolvedLocalId(LocalId),
    #[error("invalid patch on field `{field}`: {reason}")]
    InvalidPatch { field: String, reason: String },
    #[error("cycle detected in commit plan")]
    CycleInPlan,
}

/// A single schema validation violation.
#[derive(Debug, Error, PartialEq, Clone)]
pub enum SchemaViolation {
    #[error("`{label_or_type}` missing required prop `{prop}`")]
    MissingRequiredProp { label_or_type: String, prop: String },
    #[error("`{label_or_type}`.`{prop}` expected {expected:?}, got {got:?}")]
    WrongPropType {
        label_or_type: String,
        prop: String,
        expected: String,
        got: String,
    },
    #[error("rel `{r#type}` endpoints `{start}`->`{end}` not in allowed set")]
    DisallowedEndpoints {
        r#type: String,
        start: String,
        end: String,
    },
    #[error("unknown label `{label}` not registered in schema")]
    UnknownLabel { label: String },
    #[error("unknown rel type `{r#type}` not registered in schema")]
    UnknownRelType { r#type: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::LocalId;

    #[test]
    fn display_messages() {
        let e = CoreError::UnresolvedLocalId(LocalId(7));
        assert!(format!("{e}").contains("LocalId(7)"));
        let e = CoreError::CycleInPlan;
        assert!(format!("{e}").contains("cycle"));
    }

    #[test]
    fn schema_violation_variants() {
        let v = SchemaViolation::MissingRequiredProp {
            label_or_type: "Person".into(),
            prop: "name".into(),
        };
        assert!(format!("{v}").contains("name"));
    }
}
