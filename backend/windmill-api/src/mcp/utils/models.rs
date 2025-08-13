//! Data models for MCP server
//!
//! Contains all the data structures used throughout the MCP implementation,
//! including database models, API response models, and utility types.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use std::collections::HashMap;
use windmill_common::scripts::Schema;

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
#[derive(Serialize, FromRow, Deserialize, Debug, Clone)]
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
#[derive(Serialize, FromRow, Debug)]
pub struct ScriptInfo {
    pub path: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub schema: Option<Schema>,
}

/// Flow information from database
#[derive(Serialize, FromRow, Debug)]
pub struct FlowInfo {
    pub path: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub schema: Option<Schema>,
}

/// Resource information from database
#[derive(Serialize, FromRow, Debug, Clone)]
pub struct ResourceInfo {
    pub path: String,
    pub description: Option<String>,
    pub resource_type: String,
}

/// Resource type information from database
#[derive(Serialize, FromRow, Debug, Clone)]
pub struct ResourceType {
    pub name: String,
    pub description: Option<String>,
}

/// Schema holder for database queries
#[derive(Serialize, FromRow)]
pub struct ItemSchema {
    pub schema: Option<Schema>,
}

/// Trait for objects that can be converted to MCP tools
pub trait ToolableItem {
    fn get_path_or_id(&self) -> String;
    fn get_summary(&self) -> &str;
    fn get_description(&self) -> &str;
    fn get_schema(&self) -> SchemaType;
    fn is_hub(&self) -> bool;
    fn item_type(&self) -> &'static str;
    fn get_integration_type(&self) -> Option<String>;
}