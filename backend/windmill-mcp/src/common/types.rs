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

/// The headers named by `?include_header=` on the MCP connection URL.
///
/// Only credential headers are withheld from a runnable's preprocessor by
/// default, so this list exists to name one of those back in — an operator who
/// genuinely wants the caller's token in `event.headers` writes
/// `?include_header=authorization`. Naming anything else is harmless and does
/// nothing: every other header already reaches the preprocessor.
///
/// The list is fixed by whoever configures the MCP client, out of reach of the
/// model driving the session.
#[derive(Clone, Debug, Default)]
pub struct McpIncludeHeaders(pub Vec<String>);

/// Ceilings on `?include_header=`, generous next to any real list — which names
/// a header or two — and small enough that parsing one stays trivial work. See
/// [`McpIncludeHeaders::parse`] for why that matters.
const MAX_INCLUDE_HEADER_LEN: usize = 1024;
const MAX_INCLUDE_HEADER_ENTRIES: usize = 32;

impl McpIncludeHeaders {
    /// Parse the comma-separated `?include_header=` value.
    ///
    /// Rejects an entry that is not a valid HTTP header name rather than keeping
    /// it: one that can never match a header is a typo worth reporting, not worth
    /// half-honouring, and silently ignoring it would leave an operator believing
    /// a credential was being forwarded when it was not.
    ///
    /// Both ceilings are load-bearing, not cosmetic: the middleware that calls
    /// this sits outside the auth extractor, so an unauthenticated request
    /// reaches it, and a query string is otherwise bounded only by the server's
    /// request-head limit.
    pub fn parse(value: &str) -> Result<Self, String> {
        if value.len() > MAX_INCLUDE_HEADER_LEN {
            return Err(format!(
                "include_header is limited to {} characters",
                MAX_INCLUDE_HEADER_LEN
            ));
        }
        let mut names: Vec<String> = Vec::new();
        for name in value.split(',') {
            let name = name.trim().to_lowercase();
            if name.is_empty() {
                continue;
            }
            if !is_valid_header_name(&name) {
                return Err(format!("'{}' is not a valid header name", name));
            }
            if names.contains(&name) {
                continue;
            }
            if names.len() >= MAX_INCLUDE_HEADER_ENTRIES {
                return Err(format!(
                    "include_header is limited to {} headers",
                    MAX_INCLUDE_HEADER_ENTRIES
                ));
            }
            names.push(name);
        }
        Ok(Self(names))
    }

    /// Whether the operator named this header. Compared case-insensitively
    /// because a `HeaderMap` key may arrive in any casing.
    pub fn names(&self, header: &str) -> bool {
        self.0.iter().any(|n| n.eq_ignore_ascii_case(header))
    }
}

/// The RFC 9110 token set a field name may draw from. Spelled out here rather
/// than deferring to `http::HeaderName` because this module also compiles for the
/// MCP client, which does not pull in `http`.
fn is_valid_header_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
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
