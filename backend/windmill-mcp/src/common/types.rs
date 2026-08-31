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

/// One entry of the `?include_header=` allowlist: the exact HTTP header to read,
/// and the runnable parameter that carries its value.
///
/// The two are kept apart deliberately. Matching an inbound header by its
/// *normalised* name would make `x-user-id` and the distinct, equally valid
/// header `x_user_id` interchangeable, so a caller could supply the alias to
/// stand in for one a trusted proxy injected — and which of the two won would
/// depend on header iteration order.
#[derive(Clone, Debug)]
pub struct McpIncludeHeader {
    /// Lowercased HTTP header name, compared verbatim against the request.
    pub header_name: String,
    /// The runnable parameter fed from it (`x-user-id` -> `x_user_id`).
    pub param_name: String,
}

/// The request headers a tool call is allowed to forward into the script or flow
/// it runs, from `?include_header=` on the MCP connection URL.
///
/// The parameter names are transport-owned for the life of the connection: they
/// are removed from every published tool schema and dropped from model-supplied
/// arguments, so a value reaching a runnable under one of them came from the
/// request and not from the model. That is the whole point of the feature — an
/// identity the model can set is an identity prompt injection can forge.
#[derive(Clone, Debug, Default)]
pub struct McpIncludeHeaders(pub Vec<McpIncludeHeader>);

/// Ceilings on `?include_header=`, generous next to any real allowlist — which
/// names a handful of headers — and small enough that parsing one stays trivial
/// work. See [`McpIncludeHeaders::parse`] for why that matters.
const MAX_INCLUDE_HEADER_LEN: usize = 1024;
const MAX_INCLUDE_HEADER_ENTRIES: usize = 32;

impl McpIncludeHeaders {
    /// Parse the comma-separated `?include_header=` value.
    ///
    /// Two entries that differ only in `-` versus `_` name the same parameter;
    /// the first wins, so the mapping stays one-to-one and a runnable cannot be
    /// fed from whichever of them happened to be visited last.
    /// Rejects an entry that is not a valid HTTP header name rather than keeping
    /// it: one that can never match a header would also never be stripped from a
    /// tool schema, leaving the parameter model-settable on a connection the
    /// operator believes is locked down. `x-user-id;x-tenant`, or a space where a
    /// comma belongs, is a typo worth reporting, not worth half-honouring.
    ///
    /// Both ceilings are load-bearing, not cosmetic: the middleware that calls
    /// this sits outside the auth extractor, so an unauthenticated request
    /// reaches it, and a query string is otherwise bounded only by the server's
    /// request-head limit. With the entry count capped, the linear scan below
    /// costs at most [`MAX_INCLUDE_HEADER_ENTRIES`] comparisons per entry.
    pub fn parse(value: &str) -> Result<Self, String> {
        if value.len() > MAX_INCLUDE_HEADER_LEN {
            return Err(format!(
                "include_header is limited to {} characters",
                MAX_INCLUDE_HEADER_LEN
            ));
        }
        let mut entries: Vec<McpIncludeHeader> = Vec::new();
        for name in value.split(',') {
            let header_name = name.trim().to_lowercase();
            if header_name.is_empty() {
                continue;
            }
            if !is_valid_header_name(&header_name) {
                return Err(format!("'{}' is not a valid header name", header_name));
            }
            let param_name = normalize_header_name(&header_name);
            if entries.iter().any(|e| e.param_name == param_name) {
                continue;
            }
            if entries.len() >= MAX_INCLUDE_HEADER_ENTRIES {
                return Err(format!(
                    "include_header is limited to {} headers",
                    MAX_INCLUDE_HEADER_ENTRIES
                ));
            }
            entries.push(McpIncludeHeader { header_name, param_name });
        }
        Ok(Self(entries))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether `name` is a runnable parameter this connection fills from the
    /// request, and so must never be taken from the model.
    pub fn owns_param(&self, name: &str) -> bool {
        self.0.iter().any(|entry| entry.param_name == name)
    }

    /// Whether `key` is the *published* spelling of a parameter this connection
    /// fills from the request. It differs from [`Self::owns_param`] only when a
    /// header name carries punctuation that MCP's key transformation drops
    /// (`$user` publishes as `user`).
    ///
    /// Both the schema strip and the argument strip consult this, so a parameter
    /// hidden from the model can never be settable by it under the other
    /// spelling — the two halves agree by construction rather than by matching
    /// each other's arithmetic.
    pub fn owns_published_key(&self, key: &str) -> bool {
        self.0
            .iter()
            .any(|entry| super::transform::apply_key_transformation(&entry.param_name) == key)
    }

    pub fn iter(&self) -> impl Iterator<Item = &McpIncludeHeader> {
        self.0.iter()
    }
}

/// Render a header name as the runnable parameter that carries it.
pub fn normalize_header_name(name: &str) -> String {
    name.to_lowercase().replace('-', "_")
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
