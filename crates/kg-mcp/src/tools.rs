//! MCP tool implementations.
//!
//! Actual tool functions (entity CRUD, search, link traversal …) are wired in
//! T12.  Each tool will be an `async fn` on the `KgMcpService` struct in
//! `crate::server`, annotated with `#[rmcp::tool]` and documented with a
//! JSON Schema–driven description so Claude can discover and call them.
