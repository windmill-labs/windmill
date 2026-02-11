//! MCP Client implementation
//!
//! This module provides functionality for connecting to external MCP servers
//! and executing tools on them.

mod types;

pub use types::{McpResource, McpToolSource};

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use rmcp::{
    model::{
        CallToolRequestParams, ClientCapabilities, ClientInfo, Implementation,
        InitializeRequestParams, Tool as McpTool,
    },
    service::RunningService,
    transport::{
        streamable_http_client::StreamableHttpClientTransportConfig, StreamableHttpClientTransport,
    },
    RoleClient, ServiceExt,
};
use serde_json::{json, Value};
use std::str::FromStr;
use windmill_common::variables::get_secret_value_as_admin;
use windmill_common::DB;

/// MCP client for communicating with external MCP servers
pub struct McpClient {
    /// The underlying rmcp client
    client: RunningService<RoleClient, InitializeRequestParams>,
    /// Cached list of available tools from the server
    available_tools: Vec<McpTool>,
}

impl McpClient {
    /// Create a new MCP client from a resource configuration
    pub async fn from_resource(resource: McpResource, db: &DB, w_id: &str) -> Result<Self> {
        // Build custom reqwest client with headers if provided
        let mut headers = HeaderMap::new();
        if let Some(token_path) = &resource.token {
            if !token_path.trim().is_empty() {
                let value =
                    get_secret_value_as_admin(db, w_id, token_path.trim_start_matches("$var:"))
                        .await?;
                headers.insert(
                    HeaderName::from_static("authorization"),
                    HeaderValue::from_str(format!("Bearer {}", value).as_str())?,
                );
            }
        }
        if let Some(resource_headers) = &resource.headers {
            for (key, value) in resource_headers {
                match (HeaderName::from_str(key), HeaderValue::from_str(value)) {
                    (Ok(name), Ok(value)) => {
                        headers.insert(name, value);
                    }
                    _ => {
                        tracing::warn!("Invalid header: {}={}", key, value);
                    }
                }
            }
        }

        let reqwest_client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to build HTTP client")?;

        // Create the HTTP transport with custom client
        let config = StreamableHttpClientTransportConfig::with_uri(resource.url.as_str());
        let transport = StreamableHttpClientTransport::with_client(reqwest_client, config);

        // Set up client info
        let client_info = ClientInfo {
            protocol_version: Default::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "windmill-ai-agent".to_string(),
                title: Some("Windmill AI Agent".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                description: None,
                website_url: None,
                icons: None,
            },
            meta: None,
        };

        // Initialize the connection
        let client = client_info
            .serve(transport)
            .await
            .context("Failed to connect to MCP server")?;

        // Immediately fetch available tools
        let available_tools = client
            .list_tools(Default::default())
            .await
            .context("Failed to list tools from MCP server")?
            .tools;

        Ok(Self { client, available_tools })
    }

    /// Get the list of available tools from the MCP server
    pub fn available_tools(&self) -> &[McpTool] {
        &self.available_tools
    }

    /// Call a tool on the MCP server, with openai-style arguments
    pub async fn call_tool(&self, name: &str, arguments: &str) -> Result<serde_json::Value> {
        // Convert OpenAI-style arguments to MCP format
        let mcp_args =
            Self::openai_args_to_mcp_args(arguments).context("Failed to parse tool arguments")?;

        let result = self
            .client
            .call_tool(CallToolRequestParams {
                name: name.to_string().into(),
                arguments: mcp_args,
                task: None,
                meta: None,
            })
            .await
            .context(format!("Failed to call MCP tool: {}", name))?;

        // Convert the result to a JSON value
        // MCP tools return ToolResult which contains content array
        let result_json =
            serde_json::to_value(&result).context("Failed to serialize MCP tool result")?;

        Ok(result_json)
    }

    /// Close the connection
    pub async fn shutdown(self) -> Result<()> {
        self.client.cancel().await?;
        Ok(())
    }

    /// Fix array schemas to ensure they have the required 'items' property
    /// OpenAI requires all array types to have an 'items' field. MCP servers may
    /// return schemas without this field, so we add a default.
    pub fn fix_array_schemas(schema: &mut Value) {
        if let Value::Object(obj) = schema {
            // Check if this is an array type
            if let Some(type_val) = obj.get("type") {
                let is_array = match type_val {
                    Value::String(s) => s == "array",
                    Value::Array(arr) => arr.iter().any(|v| v.as_str() == Some("array")),
                    _ => false,
                };

                // If it's an array and missing 'items', add a default
                if is_array && !obj.contains_key("items") {
                    obj.insert("items".to_string(), json!({}));
                }
            }

            // Recursively fix nested schemas
            if let Some(Value::Object(props)) = obj.get_mut("properties") {
                for value in props.values_mut() {
                    Self::fix_array_schemas(value);
                }
            }

            // Fix items if present (for nested arrays)
            if let Some(items) = obj.get_mut("items") {
                Self::fix_array_schemas(items);
            }

            // Fix oneOf, anyOf, allOf schemas
            for key in &["oneOf", "anyOf", "allOf"] {
                if let Some(Value::Array(schemas)) = obj.get_mut(*key) {
                    for schema in schemas {
                        Self::fix_array_schemas(schema);
                    }
                }
            }

            // Fix additionalProperties if it's a schema
            if let Some(additional) = obj.get_mut("additionalProperties") {
                if additional.is_object() {
                    Self::fix_array_schemas(additional);
                }
            }
        }
    }

    /// Convert OpenAI-style tool call arguments to MCP format
    /// OpenAI sends arguments as a JSON string, MCP expects a Map
    fn openai_args_to_mcp_args(
        args_str: &str,
    ) -> Result<Option<serde_json::Map<String, serde_json::Value>>> {
        if args_str.trim().is_empty() {
            return Ok(Some(serde_json::Map::new()));
        }

        let args_value: serde_json::Value =
            serde_json::from_str(args_str).context("Failed to parse tool call arguments")?;

        match args_value {
            serde_json::Value::Object(map) => Ok(Some(map)),
            serde_json::Value::Null => Ok(Some(serde_json::Map::new())),
            _ => Ok(Some(
                vec![("value".to_string(), args_value)]
                    .into_iter()
                    .collect(),
            )),
        }
    }
}
