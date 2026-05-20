//! Relationship types and identifiers.

use crate::node::NodeRef;
use crate::value::PropValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Server-assigned relationship identifier.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelId(pub i64);

/// A relationship with typed endpoints (server or local refs).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rel {
    pub id: Option<RelId>,
    pub r#type: String,
    pub start: NodeRef,
    pub end: NodeRef,
    pub props: BTreeMap<String, PropValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{LocalId, NodeRef};

    #[test]
    fn rel_construct() {
        let r = Rel {
            id: None,
            r#type: "KNOWS".into(),
            start: NodeRef::Local(LocalId(1)),
            end: NodeRef::Local(LocalId(2)),
            props: Default::default(),
        };
        assert_eq!(r.r#type, "KNOWS");
    }
}
