#![cfg(feature = "native")]

use async_trait::async_trait;
use kg_core::cypher::Statement;
use std::collections::BTreeMap;

use crate::auth::Auth;
use crate::convert::bolt::{from_bolt, to_bolt};
use crate::error::{Neo4jError, TransportError};
use super::{Counters, StatementResult, Transport, TxOutcome};

pub(crate) struct BoltTransport {
    graph: neo4rs::Graph,
}

impl BoltTransport {
    #[allow(dead_code)]
    pub async fn connect(
        uri: &str,
        auth: &Auth,
        database: Option<&str>,
    ) -> Result<Self, Neo4jError> {
        let Auth::Basic { user, password } = auth;
        let mut b = neo4rs::ConfigBuilder::new()
            .uri(uri)
            .user(user)
            .password(password);
        if let Some(db) = database {
            b = b.db(db);
        }
        let cfg = b
            .build()
            .map_err(|e| TransportError::Connect(e.to_string()))?;
        let graph = neo4rs::Graph::connect(cfg)
            .await
            .map_err(|e| TransportError::Connect(e.to_string()))?;
        Ok(Self { graph })
    }

    fn build_query(stmt: &Statement) -> Result<neo4rs::Query, Neo4jError> {
        let mut q = neo4rs::query(&stmt.cypher);
        for (k, v) in &stmt.params {
            q = q.param(k.as_str(), to_bolt(v)?);
        }
        Ok(q)
    }
}

/// Map a neo4rs error to a TransportError, extracting the server-side Neo4j error code/message
/// when available so the caller can distinguish syntax/client errors from transport failures.
fn map_neo4rs_to_transport(e: neo4rs::Error) -> TransportError {
    match e {
        neo4rs::Error::Neo4j(ref neo4j_err) => TransportError::ServerError {
            code: neo4j_err.code().to_string(),
            message: neo4j_err.message().to_string(),
        },
        other => TransportError::Protocol(other.to_string()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl Transport for BoltTransport {
    async fn run_tx(&self, stmts: &[Statement]) -> Result<TxOutcome, TransportError> {
        let mut txn = self
            .graph
            .start_txn()
            .await
            .map_err(map_neo4rs_to_transport)?;

        let mut statements = Vec::with_capacity(stmts.len());
        for stmt in stmts {
            let q = Self::build_query(stmt)
                .map_err(|e| TransportError::Protocol(e.to_string()))?;
            let result = run_one(&mut txn, q)
                .await
                .map_err(|e| match e {
                    // Neo4jError::Tx was already mapped via TransportError::ServerError; re-wrap.
                    Neo4jError::Tx { code, message } => {
                        TransportError::ServerError { code, message }
                    }
                    other => TransportError::Protocol(other.to_string()),
                })?;
            statements.push(result);
        }

        txn.commit()
            .await
            .map_err(map_neo4rs_to_transport)?;

        Ok(TxOutcome {
            statements,
            counters: Counters::default(),
        })
    }

    async fn run_autocommit(
        &self,
        stmt: &Statement,
    ) -> Result<StatementResult, TransportError> {
        let outcome = self.run_tx(std::slice::from_ref(stmt)).await?;
        Ok(outcome.statements.into_iter().next().unwrap_or_default())
    }
}

/// Execute a single query inside an open transaction and collect all rows into a
/// [`StatementResult`].
///
/// # How row decoding works (neo4rs 0.8)
///
/// `txn.execute(q)` returns a `RowStream`.  Each call to `stream.next(txn.handle())` yields a
/// `neo4rs::Row`.  A `Row` is a thin wrapper over a `BoltMap` (the server's field→value map).
/// Deserializing the row *as* a `BoltMap` via `row.to::<neo4rs::BoltMap>()` gives us direct
/// access to the underlying `HashMap<BoltString, BoltType>`, which we then convert to our own
/// `PropValue` type using the pre-existing `from_bolt` helper from T19.
async fn run_one(
    txn: &mut neo4rs::Txn,
    q: neo4rs::Query,
) -> Result<StatementResult, Neo4jError> {
    let mut stream = txn
        .execute(q)
        .await
        .map_err(|e| Neo4jError::from(map_neo4rs_to_transport(e)))?;

    let mut rows: Vec<BTreeMap<String, kg_core::value::PropValue>> = Vec::new();

    while let Some(row) = stream
        .next(txn.handle())
        .await
        .map_err(|e| Neo4jError::from(map_neo4rs_to_transport(e)))?
    {
        // Deserialize the whole row as a BoltMap so we can iterate all columns.
        let bolt_map: neo4rs::BoltMap = row
            .to_strict::<neo4rs::BoltMap>()
            .map_err(|e| Neo4jError::Conversion {
                from: "Row",
                to: "BoltMap",
                reason: e.to_string(),
            })?;

        let mut map: BTreeMap<String, kg_core::value::PropValue> =
            BTreeMap::new();
        for (k, v) in &bolt_map.value {
            let prop = from_bolt(v)?;
            map.insert(k.value.clone(), prop);
        }
        rows.push(map);
    }

    Ok(StatementResult {
        rows,
        counters: Counters::default(),
    })
}
