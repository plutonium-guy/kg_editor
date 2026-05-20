//! Public client + builder.

use kg_core::cypher::CypherEmitter;
use kg_core::schema::SchemaRegistry;
use kg_core::uow::UnitOfWork;
use std::collections::HashMap;

use crate::auth::Auth;
use crate::error::Neo4jError;
use crate::transport::Transport;

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
        let out = CypherEmitter::emit(&uow)?;
        // DDL each runs as its own auto-commit (Neo4j forbids mixing schema + data in one tx).
        for ddl_stmt in &out.ddl {
            self.transport.run_autocommit(ddl_stmt).await?;
        }
        // The chained data statement runs in its own transaction.
        if let Some(data_stmt) = out.data {
            let _ = self.transport.run_tx(std::slice::from_ref(&data_stmt)).await?;
        }
        Ok(CommitResult { id_map: HashMap::new() })
    }
}

/// Outcome of a successful `Client::commit`. Phase-0: `id_map` is currently empty; populated id mapping is tracked for a later release.
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

impl Client {
    pub async fn query<T: crate::from_row::FromRow>(
        &self,
        cypher: &str,
        params: impl IntoIterator<Item = (impl Into<String>, kg_core::value::PropValue)>,
    ) -> Result<Vec<T>, Neo4jError> {
        use kg_core::cypher::Statement;
        let stmt = Statement::new(cypher, params);
        let res = self.transport.run_autocommit(&stmt).await?;
        let mut out = Vec::with_capacity(res.rows.len());
        for row in res.rows {
            out.push(T::from_row(&row).map_err(|e| Neo4jError::Core(kg_core::error::CoreError::InvalidPatch {
                field: "row".into(), reason: e.to_string(),
            }))?);
        }
        Ok(out)
    }

    pub async fn fetch_node(
        &self,
        id: kg_core::node::NodeId,
    ) -> Result<Option<kg_core::node::Node>, Neo4jError> {
        use kg_core::value::PropValue;
        let res: Vec<std::collections::BTreeMap<String, PropValue>> = self.query(
            "MATCH (n) WHERE id(n) = $id RETURN labels(n) AS labels, properties(n) AS props",
            [("id", PropValue::Int(id.0))],
        ).await?;
        let Some(row) = res.into_iter().next() else { return Ok(None); };
        let labels = match row.get("labels") {
            Some(PropValue::List(xs)) => xs.iter().filter_map(|x| match x {
                PropValue::String(s) => Some(s.clone()), _ => None,
            }).collect(),
            _ => smallvec::smallvec![],
        };
        let props = match row.get("props") {
            Some(PropValue::Map(m)) => m.clone(),
            _ => Default::default(),
        };
        Ok(Some(kg_core::node::Node { id: Some(id), labels, props }))
    }

    pub async fn fetch_rel(
        &self,
        id: kg_core::rel::RelId,
    ) -> Result<Option<kg_core::rel::Rel>, Neo4jError> {
        use kg_core::value::PropValue;
        let res: Vec<std::collections::BTreeMap<String, PropValue>> = self.query(
            "MATCH (s)-[r]->(e) WHERE id(r) = $id \
             RETURN type(r) AS type, id(s) AS start_id, id(e) AS end_id, properties(r) AS props",
            [("id", PropValue::Int(id.0))],
        ).await?;
        let Some(row) = res.into_iter().next() else { return Ok(None); };
        let ty = match row.get("type") {
            Some(PropValue::String(s)) => s.clone(),
            _ => return Ok(None),
        };
        let start_id = match row.get("start_id") {
            Some(PropValue::Int(i)) => *i,
            _ => return Ok(None),
        };
        let end_id = match row.get("end_id") {
            Some(PropValue::Int(i)) => *i,
            _ => return Ok(None),
        };
        let props = match row.get("props") {
            Some(PropValue::Map(m)) => m.clone(),
            _ => Default::default(),
        };
        Ok(Some(kg_core::rel::Rel {
            id: Some(id),
            r#type: ty,
            start: kg_core::node::NodeRef::Server(kg_core::node::NodeId(start_id)),
            end:   kg_core::node::NodeRef::Server(kg_core::node::NodeId(end_id)),
            props,
        }))
    }

    pub async fn materialize_schema(&self) -> Result<(), Neo4jError> {
        let Some(reg) = &self.registry else { return Ok(()); };
        let mut uow = UnitOfWork::new();
        for ns in reg.nodes() {
            for unique in &ns.uniqueness {
                uow.ensure_constraint(kg_core::uow::NodeConstraint::Unique {
                    label: ns.label.clone(), props: unique.clone(),
                });
            }
            for ix in &ns.indexes {
                uow.ensure_index(kg_core::uow::IndexSpec {
                    label: ns.label.clone(), props: ix.clone(),
                });
            }
            for p in &ns.props {
                if p.required {
                    uow.ensure_constraint(kg_core::uow::NodeConstraint::Exists {
                        label: ns.label.clone(), prop: p.name.clone(),
                    });
                }
            }
        }
        self.commit_unchecked(uow).await?;
        Ok(())
    }
}

impl Client {
    /// Run a single DDL statement as an auto-commit.
    pub async fn run_autocommit_for_ddl(&self, stmt: &kg_core::cypher::Statement) -> Result<(), Neo4jError> {
        self.transport.run_autocommit(stmt).await?;
        Ok(())
    }

    /// Run a single chained data statement inside a transaction.
    pub async fn run_tx_for_data(&self, stmt: &kg_core::cypher::Statement) -> Result<(), Neo4jError> {
        let _ = self.transport.run_tx(std::slice::from_ref(stmt)).await?;
        Ok(())
    }
}

/// Fluent builder for [`Client`]. Configure URI, auth, optional database name, and optional schema registry, then call `build()` (native) or `build_with_http(client)` (wasm).
pub struct ClientBuilder {
    #[allow(dead_code)]
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

#[cfg(feature = "wasm")]
impl ClientBuilder {
    /// Build a client backed by an HTTP transport using the supplied client.
    /// Use `WebSysHttpClient` for browser fetch, or any custom impl.
    pub fn build_with_http<C>(self, client: C) -> Result<Client, Neo4jError>
    where
        C: crate::transport::http::HttpClient + 'static,
    {
        use std::sync::Arc;
        let auth = self.auth.ok_or_else(|| Neo4jError::Auth("missing auth".into()))?;
        let database = self.database.unwrap_or_else(|| "neo4j".into());
        let tr = crate::transport::http::HttpTransport::new(
            self.uri, database, &auth, Arc::new(client),
        );
        Ok(Client { transport: Box::new(tr), registry: self.registry })
    }
}
