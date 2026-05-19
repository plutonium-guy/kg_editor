//! Unit of Work staging.

pub mod plan;

use crate::node::{LocalId, NodeId, NodeRef};
use crate::rel::RelId;
use crate::value::PropValue;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::BTreeMap;

/// Sparse property patch. `Some` = set, `None` = remove.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PropPatch(BTreeMap<String, Option<PropValue>>);

impl PropPatch {
    pub fn new() -> Self { Self::default() }
    pub fn set(mut self, name: impl Into<String>, v: PropValue) -> Self {
        self.0.insert(name.into(), Some(v)); self
    }
    pub fn unset(mut self, name: impl Into<String>) -> Self {
        self.0.insert(name.into(), None); self
    }
    pub fn entries(&self) -> &BTreeMap<String, Option<PropValue>> { &self.0 }
}

/// Behavior for deleting a node that still has incident relationships.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CascadeRule { Strict, Detach }

/// DDL specification for an index.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IndexSpec { pub label: String, pub props: Vec<String> }

/// DDL specification for a constraint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NodeConstraint {
    Unique  { label: String, props: Vec<String> },
    Exists  { label: String, prop: String },
}

/// A single staged operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StagedOp {
    CreateNode { local: LocalId, labels: SmallVec<[String; 2]>, props: BTreeMap<String, PropValue> },
    MergeNode  { local: LocalId, labels: SmallVec<[String; 2]>, key_props: BTreeMap<String, PropValue>, set_props: BTreeMap<String, PropValue> },
    UpdateNode { id: NodeId, patch: PropPatch },
    DeleteNode { id: NodeId, cascade: CascadeRule },
    CreateRel  { local: LocalId, r#type: String, start: NodeRef, end: NodeRef, props: BTreeMap<String, PropValue> },
    MergeRel   { local: LocalId, r#type: String, start: NodeRef, end: NodeRef, key_props: BTreeMap<String, PropValue>, set_props: BTreeMap<String, PropValue> },
    UpdateRel  { id: RelId, patch: PropPatch },
    DeleteRel  { id: RelId },
    EnsureConstraint(NodeConstraint),
    EnsureIndex(IndexSpec),
}

/// Staged mutations against a graph; commit emits one Cypher transaction.
#[derive(Clone, Debug, Default)]
pub struct UnitOfWork {
    ops: Vec<StagedOp>,
    next_local: u64,
}

impl UnitOfWork {
    pub fn new() -> Self { Self::default() }
    pub fn ops(&self) -> &[StagedOp] { &self.ops }

    fn fresh_local(&mut self) -> LocalId {
        self.next_local += 1;
        LocalId(self.next_local)
    }

    pub fn create_node<L, S, I, K>(&mut self, labels: L, props: I) -> LocalId
    where
        L: IntoIterator<Item = S>, S: Into<String>,
        I: IntoIterator<Item = (K, PropValue)>, K: Into<String>,
    {
        let local = self.fresh_local();
        self.ops.push(StagedOp::CreateNode {
            local,
            labels: labels.into_iter().map(Into::into).collect(),
            props: props.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        });
        local
    }

    pub fn merge_node<L, S, I, J, K1, K2>(&mut self, labels: L, key_props: I, set_props: J) -> LocalId
    where
        L: IntoIterator<Item = S>, S: Into<String>,
        I: IntoIterator<Item = (K1, PropValue)>, K1: Into<String>,
        J: IntoIterator<Item = (K2, PropValue)>, K2: Into<String>,
    {
        let local = self.fresh_local();
        self.ops.push(StagedOp::MergeNode {
            local,
            labels: labels.into_iter().map(Into::into).collect(),
            key_props: key_props.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            set_props: set_props.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        });
        local
    }

    pub fn update_node(&mut self, id: NodeId, patch: PropPatch) {
        self.ops.push(StagedOp::UpdateNode { id, patch });
    }
    pub fn delete_node(&mut self, id: NodeId, cascade: CascadeRule) {
        self.ops.push(StagedOp::DeleteNode { id, cascade });
    }

    pub fn create_rel<T, I, K>(&mut self, start: impl Into<NodeRef>, end: impl Into<NodeRef>, r#type: T, props: I) -> LocalId
    where
        T: Into<String>,
        I: IntoIterator<Item = (K, PropValue)>, K: Into<String>,
    {
        let local = self.fresh_local();
        self.ops.push(StagedOp::CreateRel {
            local,
            r#type: r#type.into(),
            start: start.into(),
            end: end.into(),
            props: props.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        });
        local
    }

    pub fn merge_rel<T, I, J, K1, K2>(
        &mut self,
        start: impl Into<NodeRef>,
        end: impl Into<NodeRef>,
        r#type: T,
        key_props: I,
        set_props: J,
    ) -> LocalId
    where
        T: Into<String>,
        I: IntoIterator<Item = (K1, PropValue)>, K1: Into<String>,
        J: IntoIterator<Item = (K2, PropValue)>, K2: Into<String>,
    {
        let local = self.fresh_local();
        self.ops.push(StagedOp::MergeRel {
            local,
            r#type: r#type.into(),
            start: start.into(),
            end: end.into(),
            key_props: key_props.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            set_props: set_props.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        });
        local
    }

    pub fn update_rel(&mut self, id: RelId, patch: PropPatch) {
        self.ops.push(StagedOp::UpdateRel { id, patch });
    }
    pub fn delete_rel(&mut self, id: RelId) {
        self.ops.push(StagedOp::DeleteRel { id });
    }

    pub fn ensure_constraint(&mut self, c: NodeConstraint) {
        self.ops.push(StagedOp::EnsureConstraint(c));
    }
    pub fn ensure_index(&mut self, ix: IndexSpec) {
        self.ops.push(StagedOp::EnsureIndex(ix));
    }
}

// Ergonomics: bare LocalId / NodeId convert into NodeRef.
impl From<LocalId> for NodeRef { fn from(v: LocalId) -> Self { NodeRef::Local(v) } }
impl From<NodeId>  for NodeRef { fn from(v: NodeId)  -> Self { NodeRef::Server(v) } }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{NodeId, NodeRef};
    use crate::value::PropValue;

    #[test]
    fn create_node_returns_increasing_local_ids() {
        let mut uow = UnitOfWork::new();
        let a = uow.create_node(["Person"], [("name", PropValue::from("A"))]);
        let b = uow.create_node(["Person"], [("name", PropValue::from("B"))]);
        assert_ne!(a, b);
    }

    #[test]
    fn stages_are_recorded() {
        let mut uow = UnitOfWork::new();
        uow.create_node(["X"], [] as [(String, PropValue); 0]);
        uow.update_node(NodeId(7), PropPatch::new().set("n", PropValue::Int(1)));
        uow.delete_node(NodeId(8), CascadeRule::Detach);
        assert_eq!(uow.ops().len(), 3);
    }

    #[test]
    fn create_rel_accepts_local_or_server_endpoints() {
        let mut uow = UnitOfWork::new();
        let a = uow.create_node(["P"], [] as [(String, PropValue); 0]);
        uow.create_rel(NodeRef::Local(a), NodeRef::Server(NodeId(9)), "R", [] as [(String, PropValue); 0]);
        assert_eq!(uow.ops().len(), 2);
    }

    #[test]
    fn merge_rel_stages_correctly() {
        let mut uow = UnitOfWork::new();
        let a = uow.create_node(["P"], [] as [(String, PropValue); 0]);
        let b = uow.create_node(["P"], [] as [(String, PropValue); 0]);
        let r = uow.merge_rel(
            a, b, "KNOWS",
            [("since", PropValue::Int(2020))],
            [("strength", PropValue::Float(0.8))],
        );
        assert_eq!(uow.ops().len(), 3);
        match uow.ops().last() {
            Some(StagedOp::MergeRel { r#type, key_props, set_props, .. }) => {
                assert_eq!(r#type, "KNOWS");
                assert_eq!(key_props.len(), 1);
                assert_eq!(set_props.len(), 1);
            }
            _ => panic!("expected MergeRel"),
        }
        let _ = r;
    }

    #[test]
    fn prop_patch_sparse() {
        let p = PropPatch::new()
            .set("a", PropValue::Int(1))
            .unset("b");
        assert_eq!(p.entries().len(), 2);
    }
}
