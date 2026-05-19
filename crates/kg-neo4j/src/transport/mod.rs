//! Transport abstraction.

#[cfg(feature = "native")]
pub(crate) mod bolt;

#[cfg(feature = "wasm")]
pub(crate) mod http;

use async_trait::async_trait;
use kg_core::cypher::Statement;
use kg_core::value::PropValue;
use std::collections::BTreeMap;

use crate::error::TransportError;

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct StatementResult {
    pub rows: Vec<BTreeMap<String, PropValue>>,
    pub counters: Counters,
}

#[derive(Clone, Debug, Default, PartialEq)]
#[allow(dead_code)]
pub struct Counters {
    pub nodes_created: u32,
    pub nodes_deleted: u32,
    pub rels_created: u32,
    pub rels_deleted: u32,
    pub props_set: u32,
    pub labels_added: u32,
    pub labels_removed: u32,
    pub indexes_added: u32,
    pub constraints_added: u32,
}

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct TxOutcome {
    pub statements: Vec<StatementResult>,
    pub counters: Counters,
}

#[cfg(not(feature = "wasm"))]
#[async_trait]
#[allow(dead_code)]
pub(crate) trait Transport: Send + Sync {
    async fn run_tx(&self, stmts: &[Statement]) -> Result<TxOutcome, TransportError>;
    async fn run_autocommit(&self, stmt: &Statement) -> Result<StatementResult, TransportError>;
}

#[cfg(feature = "wasm")]
#[async_trait(?Send)]
#[allow(dead_code)]
pub(crate) trait Transport {
    async fn run_tx(&self, stmts: &[Statement]) -> Result<TxOutcome, TransportError>;
    async fn run_autocommit(&self, stmt: &Statement) -> Result<StatementResult, TransportError>;
}
