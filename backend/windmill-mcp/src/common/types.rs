/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Data models for MCP server
//!
//! Contains all the data structures used throughout the MCP implementation,
//! including database models, API response models, and utility types.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use windmill_common::scripts::Schema;

#[cfg(feature = "server")]
use sqlx::FromRow;

/// Workspace ID wrapper for Axum extensions
#[derive(Clone, Debug)]
pub struct WorkspaceId(pub String);

/// Hub API response structure
#[derive(Serialize, Deserialize, Debug)]
pub struct HubResponse {
    pub asks: Vec<HubScriptInfo>,
}

/// Hub script information
#[derive(Serialize, Deserialize, Debug)]
pub struct HubScriptInfo {
    pub version_id: u64,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub schema: Option<Value>,
    pub app: Option<String>,
}

/// Schema type structure for JSON schemas
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct SchemaType {
    pub r#type: String,
    pub properties: HashMap<String, Value>,
    pub required: Vec<String>,
}

impl Default for SchemaType {
    fn default() -> Self {
        Self {
            r#type: "object".to_string(),
            properties: HashMap::new(),
            required: vec![],
        }
    }
}

/// Script information from database
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct ScriptInfo {
    pub path: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub schema: Option<Schema>,
}

/// Flow information from database
#[derive(Serialize, Debug)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct FlowInfo {
    pub path: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub schema: Option<Schema>,
}

/// Resource information from database
#[derive(Serialize, Debug, Clone)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct ResourceInfo {
    pub path: String,
    pub description: Option<String>,
    pub resource_type: String,
}

/// Resource type information from database
#[derive(Serialize, Debug, Clone)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct ResourceType {
    pub name: String,
    pub description: Option<String>,
}

/// Schema holder for database queries
#[derive(Serialize)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct ItemSchema {
    pub schema: Option<Schema>,
}

/// Trait for objects that can be converted to MCP tools
pub trait ToolableItem {
    /// Get the path or identifier for this item (transformed for MCP compatibility)
    fn get_path_or_id(&self) -> String;
    /// Get the summary/title of this item
    fn get_summary(&self) -> &str;
    /// Get the description of this item
    fn get_description(&self) -> &str;
    /// Get the JSON schema for this item's parameters
    fn get_schema(&self) -> SchemaType;
    /// Whether this item is from the Hub
    fn is_hub(&self) -> bool;
    /// Get the type of this item ("script" or "flow")
    fn item_type(&self) -> &'static str;
    /// Get the integration type (for hub scripts)
    fn get_integration_type(&self) -> Option<String>;
}
