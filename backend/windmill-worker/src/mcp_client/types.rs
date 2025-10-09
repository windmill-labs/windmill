//! Types for MCP client configuration and metadata

use serde::{Deserialize, Serialize};

/// Configuration for MCP tools in an AI agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolConfig {
    /// Path to the MCP resource in the workspace (e.g., "f/myteam/mcp_server")
    pub mcp_resource_path: String,
    /// Optional filter for which tools to include (None = all tools)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_tools: Option<Vec<String>>,
}

/// MCP server resource configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    /// HTTP URL for the MCP server endpoint
    pub url: String,
    /// Optional API key for authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

/// Metadata for tracking MCP tool sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolSource {
    /// Resource path where this tool comes from
    pub resource_path: String,
    /// Original tool name in the MCP server
    pub original_tool_name: String,
}
