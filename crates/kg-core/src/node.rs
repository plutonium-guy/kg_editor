//! Node types and identifiers.

use crate::value::PropValue;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::BTreeMap;

/// Server-assigned node identifier (Neo4j internal id).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub i64);

/// UoW-scoped handle issued before commit. Resolves to a `NodeId` on success.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalId(pub u64);

/// Reference to a node either by server id or local (pre-commit) id.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeRef {
    Server(NodeId),
    Local(LocalId),
}

/// A node with optional server id, labels, and properties.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: Option<NodeId>,
    pub labels: SmallVec<[String; 2]>,
    pub props: BTreeMap<String, PropValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::PropValue;

    #[test]
    fn local_id_distinct() {
        let a = LocalId(1);
        let b = LocalId(2);
        assert_ne!(a, b);
    }

    #[test]
    fn node_ref_variants() {
        let nr = NodeRef::Server(NodeId(7));
        assert!(matches!(nr, NodeRef::Server(_)));
        let nr2 = NodeRef::Local(LocalId(3));
        assert!(matches!(nr2, NodeRef::Local(_)));
    }

    #[test]
    fn node_construct() {
        let mut props = std::collections::BTreeMap::new();
        props.insert("name".into(), PropValue::String("Alice".into()));
        let n = Node { id: None, labels: smallvec::smallvec!["Person".into()], props };
        assert_eq!(n.labels.len(), 1);
    }
}
