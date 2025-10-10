//! MCP (Model Context Protocol) client for AI agents
//!
//! This module provides integration with MCP servers, allowing AI agents to use
//! external tools through the Model Context Protocol.

pub mod client;
pub mod converter;
pub mod types;

pub use client::McpClient;
pub use converter::{mcp_tool_to_tooldef, openai_args_to_mcp_args};
pub use types::{McpResource, McpToolSource};
