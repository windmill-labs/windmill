//! Model Context Protocol (MCP) implementation for Windmill
//!
//! This module provides the MCP server implementation that exposes Windmill scripts,
//! flows, and API endpoints as MCP tools for AI assistants to interact with.

mod auto_generated_endpoints;
mod core;
mod utils;

// Re-export only what's needed externally
pub use core::{extract_and_store_workspace_id, list_tools_service, setup_mcp_server};
