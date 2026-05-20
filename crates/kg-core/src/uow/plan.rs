//! Commit planning (topological sort).

use crate::error::CoreError;
use crate::node::{LocalId, NodeRef};
use crate::uow::{StagedOp, UnitOfWork};
use std::collections::HashSet;

/// Ordered staged ops ready for Cypher emission.
#[derive(Clone, Debug)]
pub struct CommitPlan {
    pub ordered: Vec<StagedOp>,
}

impl CommitPlan {
    /// Sort `uow.ops()` into the canonical commit order and verify that every
    /// rel referencing a `LocalId` refers to a node staged earlier.
    pub fn from_uow(uow: &UnitOfWork) -> Result<Self, CoreError> {
        let phase = |op: &StagedOp| -> u8 {
            use StagedOp::*;
            match op {
                EnsureConstraint(_) | EnsureIndex(_) => 0,
                CreateNode { .. } | MergeNode { .. } => 1,
                CreateRel { .. } | MergeRel { .. } => 2,
                UpdateNode { .. } => 3,
                UpdateRel { .. } => 4,
                DeleteRel { .. } => 5,
                DeleteNode { .. } => 6,
            }
        };
        let mut ordered: Vec<StagedOp> = uow.ops().to_vec();
        // stable sort preserves relative order within a phase
        ordered.sort_by_key(phase);

        // verify LocalId resolution
        let mut known: HashSet<LocalId> = HashSet::new();
        for op in &ordered {
            match op {
                StagedOp::CreateNode { local, .. }
                | StagedOp::MergeNode { local, .. }  => { known.insert(*local); }
                StagedOp::CreateRel { start, end, .. }
                | StagedOp::MergeRel { start, end, .. } => {
                    check_ref(start, &known)?;
                    check_ref(end, &known)?;
                }
                _ => {}
            }
        }
        Ok(CommitPlan { ordered })
    }
}

fn check_ref(r: &NodeRef, known: &HashSet<LocalId>) -> Result<(), CoreError> {
    if let NodeRef::Local(id) = r {
        if !known.contains(id) {
            return Err(CoreError::UnresolvedLocalId(*id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{LocalId, NodeId, NodeRef};
    use crate::uow::{CascadeRule, PropPatch, UnitOfWork};
    use crate::value::PropValue;

    #[test]
    fn ordering_constraints_before_creates() {
        let mut uow = UnitOfWork::new();
        let a = uow.create_node(["P"], [] as [(String, PropValue); 0]);
        let _b = uow.create_node(["P"], [] as [(String, PropValue); 0]);
        uow.update_node(NodeId(1), PropPatch::new().set("x", PropValue::Int(1)));
        uow.delete_node(NodeId(2), CascadeRule::Detach);
        uow.ensure_index(crate::uow::IndexSpec { label: "P".into(), props: vec!["name".into()] });
        let plan = CommitPlan::from_uow(&uow).unwrap();
        let kinds: Vec<&str> = plan.ordered.iter().map(kind_of).collect();
        assert_eq!(kinds[0], "EnsureIndex");
        assert!(kinds[1] == "CreateNode" && kinds[2] == "CreateNode");
        assert!(kinds.iter().position(|&k| k == "UpdateNode").unwrap()
              > kinds.iter().rposition(|&k| k == "CreateNode").unwrap());
        let _ = a;
    }

    #[test]
    fn rel_referencing_local_after_node_create() {
        let mut uow = UnitOfWork::new();
        let a = uow.create_node(["P"], [] as [(String, PropValue); 0]);
        let b = uow.create_node(["P"], [] as [(String, PropValue); 0]);
        uow.create_rel(NodeRef::Local(a), NodeRef::Local(b), "R", [] as [(String, PropValue); 0]);
        let plan = CommitPlan::from_uow(&uow).unwrap();
        let kinds: Vec<&str> = plan.ordered.iter().map(kind_of).collect();
        assert_eq!(kinds, vec!["CreateNode", "CreateNode", "CreateRel"]);
    }

    #[test]
    fn rel_referencing_unstaged_local_id_errors() {
        let mut uow = UnitOfWork::new();
        let phantom = LocalId(99);
        uow.create_rel(NodeRef::Local(phantom), NodeRef::Server(NodeId(1)), "R", [] as [(String, PropValue); 0]);
        let err = CommitPlan::from_uow(&uow).unwrap_err();
        assert!(matches!(err, crate::error::CoreError::UnresolvedLocalId(_)));
    }

    fn kind_of(op: &crate::uow::StagedOp) -> &'static str {
        use crate::uow::StagedOp::*;
        match op {
            EnsureConstraint(_) => "EnsureConstraint",
            EnsureIndex(_) => "EnsureIndex",
            CreateNode { .. } => "CreateNode",
            MergeNode { .. } => "MergeNode",
            CreateRel { .. } => "CreateRel",
            MergeRel { .. } => "MergeRel",
            UpdateNode { .. } => "UpdateNode",
            UpdateRel { .. } => "UpdateRel",
            DeleteRel { .. } => "DeleteRel",
            DeleteNode { .. } => "DeleteNode",
        }
    }
}
