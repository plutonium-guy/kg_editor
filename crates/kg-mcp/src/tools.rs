//! MCP tool implementations.
//!
//! All 7 tools (get_schema, create_entity, update_entity, delete_entity,
//! link_entities, unlink_entities, find_entities) are defined as async methods
//! on `KgMcpService` in `crate::server`, registered via rmcp's `#[tool_router]`
//! and `#[tool_handler]` proc-macros.  This module is kept as a module
//! declaration placeholder so `main.rs` compiles unchanged.
