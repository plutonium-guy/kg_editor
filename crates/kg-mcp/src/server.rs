//! MCP stdio server – `KgMcpService` with 7 kg-server tools.
//!
//! Tool registration uses the `#[tool_router]` + `#[tool_handler]` proc-macro
//! pair from rmcp 1.7.  Each async method annotated with `#[tool]` is
//! registered automatically; the doc-comment becomes the tool description that
//! Claude reads when deciding which tool to call.
//!
//! # Transport
//! `rmcp::transport::stdio()` returns a `(Stdin, Stdout)` pair.  Tracing is
//! wired to stderr so stdout stays clean for the MCP JSON-RPC framing.

use std::sync::Arc;

use anyhow::Result;
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerInfo},
    schemars,
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::client::KgClient;

// ---------------------------------------------------------------------------
// Parameter structs (JsonSchema derived for MCP input-schema generation)
// ---------------------------------------------------------------------------

/// Parameters for `create_entity`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateEntityParams {
    /// Label (type) for the new entity, e.g. "Person" or "Company".
    pub label: String,
    /// Optional property map to attach to the entity at creation time.
    #[serde(default)]
    pub props: serde_json::Map<String, JsonValue>,
}

/// Parameters for `update_entity`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateEntityParams {
    /// Numeric id of the entity to update.
    pub id: i64,
    /// Properties to set or overwrite on the entity.
    #[serde(default)]
    pub set: serde_json::Map<String, JsonValue>,
    /// Property keys to remove from the entity.
    #[serde(default)]
    pub unset: Vec<String>,
}

/// Parameters for `delete_entity`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteEntityParams {
    /// Numeric id of the entity to delete.
    pub id: i64,
    /// When true, also delete all relationships connected to this entity.
    #[serde(default)]
    pub cascade: bool,
}

/// Parameters for `link_entities`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LinkEntitiesParams {
    /// Relationship type, e.g. "KNOWS" or "OWNS".
    #[serde(rename = "type")]
    pub ty: String,
    /// Id of the source (start) entity.
    pub start_id: i64,
    /// Id of the target (end) entity.
    pub end_id: i64,
    /// Optional property map to attach to the relationship.
    #[serde(default)]
    pub props: serde_json::Map<String, JsonValue>,
}

/// Parameters for `unlink_entities`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UnlinkEntitiesParams {
    /// Numeric id of the relationship to delete.
    pub rel_id: i64,
}

/// Parameters for `find_entities`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindEntitiesParams {
    /// Restrict results to entities with this label.
    pub label: Option<String>,
    /// Full-text search query applied to entity properties.
    pub q: Option<String>,
    /// Maximum number of results to return (default 20).
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

// ---------------------------------------------------------------------------
// Service struct
// ---------------------------------------------------------------------------

/// MCP service that exposes kg-server entity/graph operations as tools.
#[derive(Clone)]
pub struct KgMcpService {
    kg: Arc<KgClient>,
    tool_router: ToolRouter<Self>,
}

impl KgMcpService {
    pub fn new(kg: KgClient) -> Self {
        Self {
            kg: Arc::new(kg),
            tool_router: Self::tool_router(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

#[tool_router]
impl KgMcpService {
    /// Return the full knowledge-graph schema (node labels, rel types, property defs).
    #[tool(description = "Return the full knowledge-graph schema (node labels, relationship types, property definitions).")]
    async fn get_schema(&self) -> String {
        match self.kg.get_schema().await {
            Ok(v) => v.to_string(),
            Err(e) => format!("{{\"error\": \"{e}\"}}"),
        }
    }

    /// Create a new entity (node) in the knowledge graph and return it.
    #[tool(description = "Create a new entity (node) in the knowledge graph; returns the created entity with its assigned id.")]
    async fn create_entity(&self, Parameters(p): Parameters<CreateEntityParams>) -> String {
        let args = crate::client::CreateEntityArgs {
            label: p.label,
            props: p.props,
        };
        match self.kg.create_entity(&args).await {
            Ok(e) => serde_json::json!({
                "id":    e.id,
                "label": e.label,
                "props": e.props,
            })
            .to_string(),
            Err(e) => format!("{{\"error\": \"{e}\"}}"),
        }
    }

    /// Update properties on an existing entity identified by id.
    #[tool(description = "Update properties on an existing entity: set new/changed keys and/or unset (remove) existing keys.")]
    async fn update_entity(&self, Parameters(p): Parameters<UpdateEntityParams>) -> String {
        match self.kg.update_entity(p.id, &p.set, &p.unset).await {
            Ok(()) => r#"{"ok":true}"#.to_string(),
            Err(e) => format!("{{\"error\": \"{e}\"}}"),
        }
    }

    /// Delete an entity by id, optionally cascading to its relationships.
    #[tool(description = "Delete an entity by id; set cascade=true to also remove all connected relationships.")]
    async fn delete_entity(&self, Parameters(p): Parameters<DeleteEntityParams>) -> String {
        match self.kg.delete_entity(p.id, p.cascade).await {
            Ok(()) => r#"{"ok":true}"#.to_string(),
            Err(e) => format!("{{\"error\": \"{e}\"}}"),
        }
    }

    /// Create a directed relationship between two entities.
    #[tool(description = "Create a directed relationship of the given type between start_id and end_id; returns the new relationship id.")]
    async fn link_entities(&self, Parameters(p): Parameters<LinkEntitiesParams>) -> String {
        match self
            .kg
            .link_entities(&p.ty, p.start_id, p.end_id, &p.props)
            .await
        {
            Ok(id) => serde_json::json!({"id": id}).to_string(),
            Err(e) => format!("{{\"error\": \"{e}\"}}"),
        }
    }

    /// Delete a relationship by its id.
    #[tool(description = "Delete a relationship (edge) from the graph by its numeric id.")]
    async fn unlink_entities(&self, Parameters(p): Parameters<UnlinkEntitiesParams>) -> String {
        match self.kg.unlink_entities(p.rel_id).await {
            Ok(()) => r#"{"ok":true}"#.to_string(),
            Err(e) => format!("{{\"error\": \"{e}\"}}"),
        }
    }

    /// Search and list entities, with optional label filter and full-text query.
    #[tool(description = "Search entities by optional label and/or full-text query; returns a list of matching entities up to `limit`.")]
    async fn find_entities(&self, Parameters(p): Parameters<FindEntitiesParams>) -> String {
        match self
            .kg
            .find_entities(p.label.as_deref(), p.q.as_deref(), p.limit)
            .await
        {
            Ok(v) => v.to_string(),
            Err(e) => format!("{{\"error\": \"{e}\"}}"),
        }
    }
}

// ---------------------------------------------------------------------------
// ServerHandler implementation
// ---------------------------------------------------------------------------

#[tool_handler(router = self.tool_router)]
impl ServerHandler for KgMcpService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("kg-mcp", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Knowledge-graph editor: create/update/delete entities and relationships, \
                 search nodes, inspect the schema.",
            )
    }
}

// ---------------------------------------------------------------------------
// Entry-point
// ---------------------------------------------------------------------------

/// Start the MCP stdio server and block until the transport closes.
pub async fn run() -> Result<()> {
    tracing::info!("kg-mcp starting");

    let kg = KgClient::from_env();
    let service = KgMcpService::new(kg)
        .serve(stdio())
        .await
        .map_err(|e| anyhow::anyhow!("MCP server init error: {e}"))?;

    service
        .waiting()
        .await
        .map_err(|e| anyhow::anyhow!("MCP server join error: {e}"))?;

    tracing::info!("kg-mcp shut down cleanly");
    Ok(())
}
