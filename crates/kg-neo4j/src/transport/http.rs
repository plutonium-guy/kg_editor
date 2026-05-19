#![cfg(feature = "wasm")]

use async_trait::async_trait;
use kg_core::cypher::Statement;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::auth::Auth;
use crate::convert::json::{from_json, to_json};
use crate::error::{Neo4jError, TransportError};
use super::{Counters, StatementResult, Transport, TxOutcome};

/// Transport-agnostic HTTP client (injectable for tests).
#[async_trait(?Send)]
pub trait HttpClient {
    async fn post(&self, url: &str, headers: &[(String, String)], body: &str)
        -> Result<HttpResponse, TransportError>;
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

pub(crate) struct HttpTransport<C: HttpClient> {
    base: String,
    database: String,
    auth_header: String,
    client: Arc<C>,
}

#[allow(dead_code)]
impl<C: HttpClient> HttpTransport<C> {
    pub fn new(base: impl Into<String>, database: impl Into<String>, auth: &Auth, client: Arc<C>) -> Self {
        let Auth::Basic { user, password } = auth;
        use base64::Engine;
        let token = base64::engine::general_purpose::STANDARD
            .encode(format!("{user}:{password}"));
        Self {
            base: base.into(),
            database: database.into(),
            auth_header: format!("Basic {token}"),
            client,
        }
    }

    fn url(&self) -> String { format!("{}/db/{}/query/v2", self.base, self.database) }
    fn tx_url(&self) -> String { format!("{}/db/{}/query/v2/tx", self.base, self.database) }

    fn headers(&self) -> Vec<(String, String)> {
        vec![
            ("Content-Type".into(), "application/json".into()),
            ("Authorization".into(), self.auth_header.clone()),
        ]
    }

    fn statement_body(stmt: &Statement) -> Result<Value, Neo4jError> {
        let mut params = serde_json::Map::new();
        for (k, v) in &stmt.params { params.insert(k.clone(), to_json(v)?); }
        Ok(json!({
            "statement": stmt.cypher,
            "parameters": params,
            "includeCounters": true,
        }))
    }

    fn batch_body(stmts: &[Statement]) -> Result<String, Neo4jError> {
        let mut arr = Vec::with_capacity(stmts.len());
        for s in stmts { arr.push(Self::statement_body(s)?); }
        Ok(serde_json::to_string(&json!({ "statements": arr })).unwrap())
    }

    fn parse_result(body: &str) -> Result<TxOutcome, Neo4jError> {
        let v: Value = serde_json::from_str(body).map_err(|e| Neo4jError::Transport(
            TransportError::Protocol(format!("invalid json: {e}"))
        ))?;
        if let Some(errs) = v.get("errors").and_then(|e| e.as_array()) {
            if let Some(first) = errs.first() {
                return Err(Neo4jError::Tx {
                    code: first.get("code").and_then(|c| c.as_str()).unwrap_or("Unknown").into(),
                    message: first.get("message").and_then(|m| m.as_str()).unwrap_or("").into(),
                });
            }
        }
        let mut statements = vec![];
        if let Some(results) = v.get("results").and_then(|r| r.as_array()) {
            for r in results {
                let keys = r.get("columns").and_then(|c| c.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect::<Vec<_>>())
                    .unwrap_or_default();
                let mut rows = vec![];
                if let Some(data) = r.get("data").and_then(|d| d.as_array()) {
                    for row in data {
                        if let Some(values) = row.get("row").and_then(|x| x.as_array()) {
                            let mut map = std::collections::BTreeMap::new();
                            for (k, vv) in keys.iter().zip(values.iter()) {
                                map.insert(k.clone(), from_json(vv)?);
                            }
                            rows.push(map);
                        }
                    }
                }
                statements.push(StatementResult { rows, counters: Counters::default() });
            }
        }
        Ok(TxOutcome { statements, counters: Counters::default() })
    }
}

#[async_trait(?Send)]
impl<C: HttpClient> Transport for HttpTransport<C> {
    async fn run_tx(&self, stmts: &[Statement]) -> Result<TxOutcome, TransportError> {
        let body = Self::batch_body(stmts).map_err(|e| TransportError::Protocol(e.to_string()))?;
        let res = self.client.post(&self.tx_url(), &self.headers(), &body).await?;
        if !(200..300).contains(&res.status) {
            return Err(TransportError::Http { status: res.status, body: res.body });
        }
        Self::parse_result(&res.body).map_err(|e| TransportError::Protocol(e.to_string()))
    }

    async fn run_autocommit(&self, stmt: &Statement) -> Result<StatementResult, TransportError> {
        let body = serde_json::to_string(&Self::statement_body(stmt).map_err(|e| TransportError::Protocol(e.to_string()))?).unwrap();
        let res = self.client.post(&self.url(), &self.headers(), &body).await?;
        if !(200..300).contains(&res.status) {
            return Err(TransportError::Http { status: res.status, body: res.body });
        }
        let outcome = Self::parse_result(&res.body).map_err(|e| TransportError::Protocol(e.to_string()))?;
        Ok(outcome.statements.into_iter().next().unwrap_or_default())
    }
}
