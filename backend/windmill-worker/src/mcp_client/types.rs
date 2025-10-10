//! Types for MCP client configuration and metadata

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// MCP server resource configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    /// Name of the MCP resource (used for prefixing tools)
    pub name: String,
    /// HTTP URL for the MCP server endpoint
    pub url: String,
    /// Optional API key for authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

/// Metadata for tracking MCP tool sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolSource {
    /// Name of the MCP resource this tool comes from
    pub name: String,
    /// Original tool name in the MCP server
    pub original_tool_name: String,
}
