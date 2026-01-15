/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

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
