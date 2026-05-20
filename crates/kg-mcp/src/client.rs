//! Thin wrapper around kg-server REST API.
//!
//! `KgClient` is constructed once in `main`, shared across tool handlers via
//! `Arc<KgClient>`, and calls the kg-server HTTP endpoints.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

pub struct KgClient {
    pub base: String,
    pub http: reqwest::Client,
}

impl KgClient {
    /// Build a client from the `KG_SERVER_URL` environment variable,
    /// defaulting to `http://localhost:9000`.
    pub fn from_env() -> Self {
        let base = std::env::var("KG_SERVER_URL")
            .unwrap_or_else(|_| "http://localhost:9000".into());
        Self {
            base,
            http: reqwest::Client::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct CreateEntityArgs {
    pub label: String,
    #[serde(default)]
    pub props: serde_json::Map<String, JsonValue>,
}

#[derive(Debug, Deserialize)]
pub struct EntityResponse {
    pub id: i64,
    pub label: String,
    pub props: JsonValue,
}

// ---------------------------------------------------------------------------
// REST methods
// ---------------------------------------------------------------------------

impl KgClient {
    /// Fetch the knowledge-graph schema from kg-server.
    pub async fn get_schema(&self) -> anyhow::Result<JsonValue> {
        Ok(self
            .http
            .get(format!("{}/schema", self.base))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    /// Create a new entity (node) in the graph.
    pub async fn create_entity(
        &self,
        args: &CreateEntityArgs,
    ) -> anyhow::Result<EntityResponse> {
        Ok(self
            .http
            .post(format!("{}/entities", self.base))
            .json(args)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    /// Update properties on an existing entity.
    pub async fn update_entity(
        &self,
        id: i64,
        set: &serde_json::Map<String, JsonValue>,
        unset: &[String],
    ) -> anyhow::Result<()> {
        self.http
            .put(format!("{}/entities/{}", self.base, id))
            .json(&serde_json::json!({"set": set, "unset": unset}))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    /// Delete an entity, optionally cascading to connected relationships.
    pub async fn delete_entity(&self, id: i64, cascade: bool) -> anyhow::Result<()> {
        self.http
            .delete(format!(
                "{}/entities/{}?cascade={}",
                self.base, id, cascade
            ))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    /// Create a relationship between two entities; returns the new relation id.
    pub async fn link_entities(
        &self,
        ty: &str,
        start_id: i64,
        end_id: i64,
        props: &serde_json::Map<String, JsonValue>,
    ) -> anyhow::Result<i64> {
        let v: JsonValue = self
            .http
            .post(format!("{}/links", self.base))
            .json(&serde_json::json!({
                "type": ty,
                "start_id": start_id,
                "end_id": end_id,
                "props": props,
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        v["id"]
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("link_entities: response missing 'id'"))
    }

    /// Delete a relationship by its id.
    pub async fn unlink_entities(&self, rel_id: i64) -> anyhow::Result<()> {
        self.http
            .delete(format!("{}/links/{}", self.base, rel_id))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    /// Search / list entities with optional label filter, full-text query, and limit.
    pub async fn find_entities(
        &self,
        label: Option<&str>,
        q: Option<&str>,
        limit: i64,
    ) -> anyhow::Result<JsonValue> {
        let mut url = reqwest::Url::parse(&format!("{}/entities", self.base))?;
        {
            let mut pairs = url.query_pairs_mut();
            if let Some(l) = label {
                pairs.append_pair("label", l);
            }
            if let Some(qq) = q {
                pairs.append_pair("q", qq);
            }
            pairs.append_pair("limit", &limit.to_string());
        }
        Ok(self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}
