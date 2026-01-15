//! Model Context Protocol (MCP) implementation for Windmill
//!
//! This module provides the MCP server implementation that exposes Windmill scripts,
//! flows, and API endpoints as MCP tools for AI assistants to interact with.

pub mod oauth_server;
pub mod server;
pub mod tools;
pub mod utils;

// Re-export main components
pub use server::{extract_and_store_workspace_id, list_tools_service, setup_mcp_server};
