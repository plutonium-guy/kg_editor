//! Public client + builder.

use kg_core::cypher::CypherEmitter;
use kg_core::schema::SchemaRegistry;
use kg_core::uow::UnitOfWork;
use std::collections::HashMap;

use crate::auth::Auth;
use crate::error::Neo4jError;
use crate::transport::{Transport, TxOutcome};

#[cfg(feature = "native")]
use crate::transport::bolt::BoltTransport;

/// High-level client. One concrete struct; transport chosen at compile time.
pub struct Client {
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    transport: Box<dyn Transport>,
    registry: Option<SchemaRegistry>,
}

impl Client {
    pub fn unit_of_work(&self) -> UnitOfWork { UnitOfWork::new() }
    pub fn schema(&self) -> Option<&SchemaRegistry> { self.registry.as_ref() }

    /// Validate (if schema attached) then emit + run as single transaction.
    pub async fn commit(&self, uow: UnitOfWork) -> Result<CommitResult, Neo4jError> {
        if let Some(reg) = &self.registry {
            let violations = validate_uow(&uow, reg);
            if !violations.is_empty() {
                return Err(Neo4jError::Core(kg_core::error::CoreError::SchemaViolation(violations)));
            }
        }
        self.commit_unchecked(uow).await
    }

    pub async fn commit_unchecked(&self, uow: UnitOfWork) -> Result<CommitResult, Neo4jError> {
        let stmts = CypherEmitter::emit(&uow)?;
        let _outcome: TxOutcome = self.transport.run_tx(&stmts).await?;
        // Phase 0 returns an empty id_map; richer mapping deferred (requires
        // emitter RETURN clauses + driver result decoding pass — tracked).
        Ok(CommitResult { id_map: HashMap::new() })
    }
}

#[derive(Debug, Default)]
pub struct CommitResult {
    pub id_map: HashMap<kg_core::node::LocalId, i64>,
}

fn validate_uow(uow: &UnitOfWork, reg: &SchemaRegistry) -> Vec<kg_core::error::SchemaViolation> {
    use kg_core::uow::StagedOp::*;
    let mut out = vec![];
    for op in uow.ops() {
        match op {
            CreateNode { labels, props, .. } => {
                if let Some(label) = labels.first() {
                    out.extend(reg.validate_node_props(label, props));
                }
            }
            MergeNode { labels, set_props, .. } => {
                if let Some(label) = labels.first() {
                    out.extend(reg.validate_node_props(label, set_props));
                }
            }
            CreateRel { r#type, .. } | MergeRel { r#type, .. } => {
                out.extend(reg.validate_rel_endpoints(r#type, None, None));
            }
            _ => {}
        }
    }
    out
}

pub struct ClientBuilder {
    uri: String,
    auth: Option<Auth>,
    database: Option<String>,
    registry: Option<SchemaRegistry>,
}

impl ClientBuilder {
    pub fn new(uri: impl Into<String>) -> Self {
        ClientBuilder { uri: uri.into(), auth: None, database: None, registry: None }
    }
    pub fn auth(mut self, a: Auth) -> Self { self.auth = Some(a); self }
    pub fn database(mut self, d: impl Into<String>) -> Self { self.database = Some(d.into()); self }
    pub fn schema(mut self, r: SchemaRegistry) -> Self { self.registry = Some(r); self }

    #[cfg(feature = "native")]
    pub async fn build(self) -> Result<Client, Neo4jError> {
        let auth = self.auth.ok_or_else(|| Neo4jError::Auth("missing auth".into()))?;
        let bt = BoltTransport::connect(&self.uri, &auth, self.database.as_deref()).await?;
        Ok(Client { transport: Box::new(bt), registry: self.registry })
    }
}
