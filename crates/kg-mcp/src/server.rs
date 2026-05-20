//! MCP stdio server entry-point.
//!
//! This module initialises the `rmcp` stdio transport and hands control to the
//! MCP runtime.  For now it runs a no-op service so the binary starts cleanly;
//! T12 replaces the stub with real tool registrations from `crate::tools`.
//!
//! # Transport choice
//! We use `rmcp`'s built-in `transport::stdio()` helper (`transport-io`
//! feature) which returns a `(tokio::io::Stdin, tokio::io::Stdout)` pair.
//! That tuple implements `IntoTransport` and is passed directly to `.serve()`.
//! stderr is left free for tracing output so Claude's MCP host never sees log
//! noise on stdout.

use anyhow::Result;
use rmcp::{
    ServerHandler, ServiceExt,
    model::{Implementation, ServerInfo},
    transport::stdio,
};

/// Minimal MCP service stub.
///
/// T12 will add `#[tool]`-annotated methods and replace this with the real
/// tool handler that wraps `KgClient`.
#[derive(Clone)]
pub struct KgMcpService;

impl ServerHandler for KgMcpService {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.server_info = Implementation::new("kg-mcp", env!("CARGO_PKG_VERSION"));
        info
    }
}

/// Start the MCP stdio server and block until the transport closes.
pub async fn run() -> Result<()> {
    tracing::info!("kg-mcp starting (stub; T12 wires tools)");

    let service = KgMcpService
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
