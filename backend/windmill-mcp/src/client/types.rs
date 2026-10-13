//! Client-specific types for MCP
//!
//! Contains configuration and metadata types used for MCP client connections.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP server resource configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    /// Name of the MCP resource (used for prefixing tools)
    pub name: String,
    /// HTTP URL for the MCP server endpoint
    pub url: String,
    /// Optional token for authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Optional headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

impl McpResource {
    /// Replaces the url and each header written as `$var:<path>` with what
    /// `read` returns for that path. `read` MUST go through the caller's
    /// permissioned (RLS + scopes) variable path. The token is not handled
    /// here: its variable path is needed first, to refresh an OAuth token.
    pub async fn resolve_variables<F, Fut, E>(&mut self, mut read: F) -> Result<(), E>
    where
        F: FnMut(String) -> Fut,
        Fut: std::future::Future<Output = Result<String, E>>,
    {
        let fields =
            std::iter::once(&mut self.url).chain(self.headers.iter_mut().flatten().map(|(_, v)| v));
        for value in fields {
            if let Some(var_path) = value.strip_prefix("$var:") {
                *value = read(var_path.to_string()).await?;
            }
        }
        Ok(())
    }
}

/// Metadata for tracking MCP tool sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolSource {
    /// Name of the MCP resource this tool comes from
    pub name: String,
    /// Original tool name in the MCP server
    pub tool_name: String,
    /// Path of the MCP resource
    pub resource_path: String,
}
