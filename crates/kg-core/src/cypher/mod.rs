//! Cypher emitter (UoW -> parameterized statements).

pub mod ddl;
pub mod nodes;
pub mod rels;
pub mod statement;

pub use statement::{ParamMap, Statement};

use crate::error::CoreError;
use crate::uow::{plan::CommitPlan, StagedOp, UnitOfWork};

/// Output of [`CypherEmitter::emit`]: zero-or-more DDL statements (each ran
/// as its own auto-commit transaction by the driver) plus an optional data
/// statement that chains every node/rel mutation via `WITH *` so variable
/// bindings persist across clauses.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EmitOutput {
    pub ddl: Vec<Statement>,
    pub data: Option<Statement>,
}

/// Lowers a [`UnitOfWork`] into ordered DDL + a single chained data statement.
pub struct CypherEmitter;

impl CypherEmitter {
    pub fn emit(uow: &UnitOfWork) -> Result<EmitOutput, CoreError> {
        let plan = CommitPlan::from_uow(uow)?;
        let mut ddl = vec![];
        let mut data_clauses: Vec<String> = vec![];
        let mut data_params: std::collections::BTreeMap<String, crate::value::PropValue> =
            Default::default();

        for op in plan.ordered {
            let stmt = match op {
                StagedOp::EnsureConstraint(c) => {
                    ddl.push(ddl::emit_constraint(&c).map_err(map_id)?);
                    continue;
                }
                StagedOp::EnsureIndex(ix) => {
                    ddl.push(ddl::emit_index(&ix).map_err(map_id)?);
                    continue;
                }
                StagedOp::CreateNode { local, labels, props } => {
                    nodes::emit_create_node(local, &labels, &props).map_err(map_id)?
                }
                StagedOp::MergeNode { local, labels, key_props, set_props } => {
                    nodes::emit_merge_node(local, &labels, &key_props, &set_props)
                        .map_err(map_id)?
                }
                StagedOp::UpdateNode { id, patch } => {
                    nodes::emit_update_node(id, &patch).map_err(map_id)?
                }
                StagedOp::DeleteNode { id, cascade } => {
                    nodes::emit_delete_node(id, cascade).map_err(map_id)?
                }
                StagedOp::CreateRel { local, start, end, r#type, props } => {
                    rels::emit_create_rel(local, start, end, &r#type, &props).map_err(map_id)?
                }
                StagedOp::MergeRel { local, start, end, r#type, key_props, set_props } => {
                    rels::emit_merge_rel(local, start, end, &r#type, &key_props, &set_props)
                        .map_err(map_id)?
                }
                StagedOp::UpdateRel { id, patch } => {
                    rels::emit_update_rel(id, &patch).map_err(map_id)?
                }
                StagedOp::DeleteRel { id } => rels::emit_delete_rel(id).map_err(map_id)?,
            };
            data_clauses.push(stmt.cypher);
            data_params.extend(stmt.params);
        }

        let data = if data_clauses.is_empty() {
            None
        } else {
            // Chain clauses with `WITH *` so previously-bound vars stay in scope.
            let joined = data_clauses.join("\nWITH *\n");
            Some(Statement { cypher: joined, params: data_params })
        };

        Ok(EmitOutput { ddl, data })
    }
}

fn map_id(e: statement::IdentError) -> CoreError {
    CoreError::InvalidPatch { field: "identifier".into(), reason: e.to_string() }
}
