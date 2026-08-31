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

/// Marker extension inserted by the gateway middleware when an MCP token has no
/// bound workspace (`workspace_id IS NULL`). Signals the runner to operate in
/// multi-workspace mode: tools take an explicit `workspace_id` argument and the
/// per-workspace auth is resolved on demand from the raw token.
#[derive(Clone, Debug)]
pub struct MultiWorkspaceMcp;

/// Raw bearer token wrapper for Axum extensions. In multi-workspace mode the
/// runner needs the raw token to re-resolve auth for each requested workspace.
#[derive(Clone, Debug)]
pub struct McpToken(pub String);

/// The request headers a tool call is allowed to forward into the script or flow
/// it runs, named the way a runnable parameter is (lowercased, `-` → `_`), from
/// `?include_header=` on the MCP connection URL.
///
/// These names are transport-owned for the life of the connection: they are
/// removed from every published tool schema and dropped from model-supplied
/// arguments, so a value reaching a runnable under one of them came from the
/// request and not from the model. That is the whole point of the feature — an
/// identity the model can set is an identity prompt injection can forge.
#[derive(Clone, Debug, Default)]
pub struct McpIncludeHeaders(pub Vec<String>);

impl McpIncludeHeaders {
    /// Parse the comma-separated `?include_header=` value into runnable-parameter
    /// names. Mirrors the normalisation webhook headers already get in
    /// `build_headers`, so `X-User-Id` reaches a script as `x_user_id` whichever
    /// entrypoint it came through.
    pub fn parse(value: &str) -> Self {
        Self(
            value
                .split(',')
                .map(|name| normalize_header_name(name.trim()))
                .filter(|name| !name.is_empty())
                .collect(),
        )
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.0.iter().any(|allowed| allowed == name)
    }
}

/// Render a header name as the runnable parameter that carries it.
pub fn normalize_header_name(name: &str) -> String {
    name.to_lowercase().replace('-', "_")
}

/// Summary of a workspace the caller can access, returned by the
/// `list_workspaces` tool in multi-workspace mode.
#[derive(Serialize, Debug, Clone)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct WorkspaceInfo {
    pub id: String,
    pub name: String,
}

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
///
/// The `#[serde(default)]` attributes are load-bearing: flow input schemas
/// legitimately omit `required` (they carry an `order` key instead) and can omit
/// `type`. Without the defaults, `serde_json::from_str::<SchemaType>` errors on
/// those schemas and callers fall back to an empty schema, dropping every input.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "server", derive(FromRow))]
pub struct SchemaType {
    #[serde(default = "default_schema_type")]
    pub r#type: String,
    #[serde(default)]
    pub properties: HashMap<String, Value>,
    #[serde(default)]
    pub required: Vec<String>,
}

fn default_schema_type() -> String {
    "object".to_string()
}

impl Default for SchemaType {
    fn default() -> Self {
        Self { r#type: "object".to_string(), properties: HashMap::new(), required: vec![] }
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
    /// Get the MCP-compatible tool name (path transformed with escaping/hashing)
    fn get_transformed_path(&self) -> String;
    /// Get the original full path of this item (for display in tool title)
    fn get_full_path(&self) -> &str;
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
