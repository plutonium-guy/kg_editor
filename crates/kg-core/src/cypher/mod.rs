//! Cypher emitter (UoW -> parameterized statements).

pub mod ddl;
pub mod nodes;
pub mod rels;
pub mod statement;

pub use statement::{ParamMap, Statement};

use crate::error::CoreError;
use crate::uow::{plan::CommitPlan, StagedOp, UnitOfWork};

/// Lowers a [`UnitOfWork`] into an ordered batch of parameterized Cypher statements.
pub struct CypherEmitter;

impl CypherEmitter {
    /// Lower the UoW into an ordered batch of parameterized statements.
    pub fn emit(uow: &UnitOfWork) -> Result<Vec<Statement>, CoreError> {
        let plan = CommitPlan::from_uow(uow)?;
        let mut out = vec![];
        for op in plan.ordered {
            let s = match op {
                StagedOp::EnsureConstraint(c) => ddl::emit_constraint(&c).map_err(map_id)?,
                StagedOp::EnsureIndex(ix)     => ddl::emit_index(&ix).map_err(map_id)?,
                StagedOp::CreateNode { local, labels, props } => {
                    nodes::emit_create_node(local, &labels, &props).map_err(map_id)?
                }
                StagedOp::MergeNode { local, labels, key_props, set_props } => {
                    nodes::emit_merge_node(local, &labels, &key_props, &set_props).map_err(map_id)?
                }
                StagedOp::UpdateNode { id, patch } => nodes::emit_update_node(id, &patch).map_err(map_id)?,
                StagedOp::DeleteNode { id, cascade } => nodes::emit_delete_node(id, cascade).map_err(map_id)?,
                StagedOp::CreateRel { local, start, end, r#type, props } => {
                    rels::emit_create_rel(local, start, end, &r#type, &props).map_err(map_id)?
                }
                StagedOp::MergeRel { local, start, end, r#type, key_props, set_props } => {
                    rels::emit_merge_rel(local, start, end, &r#type, &key_props, &set_props).map_err(map_id)?
                }
                StagedOp::UpdateRel { id, patch } => rels::emit_update_rel(id, &patch).map_err(map_id)?,
                StagedOp::DeleteRel { id } => rels::emit_delete_rel(id).map_err(map_id)?,
            };
            out.push(s);
        }
        Ok(out)
    }
}

fn map_id(e: statement::IdentError) -> CoreError {
    CoreError::InvalidPatch {
        field: "identifier".into(),
        reason: e.to_string(),
    }
}
