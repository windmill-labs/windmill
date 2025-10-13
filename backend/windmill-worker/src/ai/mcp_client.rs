use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::{json, value::RawValue, Value};
use std::str::FromStr;
use windmill_common::variables::get_secret_value_as_admin;
use windmill_common::worker::to_raw_value;
use windmill_common::DB;

use crate::ai::types::{Tool, ToolDef, ToolDefFunction};

use rmcp::model::Tool as McpTool;
use rmcp::{
    model::{
        CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation,
        InitializeRequestParam,
    },
    service::RunningService,
    transport::{
        streamable_http_client::StreamableHttpClientTransportConfig, StreamableHttpClientTransport,
    },
    RoleClient, ServiceExt,
};

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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

/// MCP client for communicating with external MCP servers
pub struct McpClient {
    /// The underlying rmcp client
    client: RunningService<RoleClient, InitializeRequestParam>,
    /// Cached list of available tools from the server, already converted to Windmill tools
    available_tools: Vec<Tool>,
}

impl McpClient {
    /// Create a new MCP client from a resource configuration
    pub async fn from_resource(
        resource: McpResource,
        db: &DB,
        w_id: &str,
        resource_path: &str,
    ) -> Result<Self> {
        // Build custom reqwest client with headers if provided
        let mut headers = HeaderMap::new();
        if let Some(token_path) = &resource.token {
            let value =
                get_secret_value_as_admin(db, w_id, token_path.trim_start_matches("$var:")).await?;
            headers.insert(
                HeaderName::from_static("authorization"),
                HeaderValue::from_str(format!("Bearer {}", value).as_str())?,
            );
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
                website_url: None,
                icons: None,
            },
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

        // Convert MCP tools to Windmill tools
        let available_tools = available_tools
            .into_iter()
            .map(|mcp_tool| {
                let tool_def = Self::mcp_tool_to_tooldef(&mcp_tool, &resource.name)?;
                Ok(Tool {
                    def: tool_def,
                    module: None,
                    mcp_source: Some(McpToolSource {
                        name: resource.name.clone(),
                        tool_name: mcp_tool.name.to_string(),
                        resource_path: resource_path.to_string(),
                    }),
                })
            })
            .collect::<Result<Vec<Tool>>>()?;

        Ok(Self { client, available_tools })
    }

    /// Get the list of available tools from the MCP server
    pub fn available_tools(&self) -> &[Tool] {
        &self.available_tools
    }

    /// Call a tool on the MCP server, with openai-style arguments
    pub async fn call_tool(&self, name: &str, arguments: &str) -> Result<serde_json::Value> {
        // Convert OpenAI-style arguments to MCP format
        let mcp_args =
            Self::openai_args_to_mcp_args(arguments).context("Failed to parse tool arguments")?;

        let result = self
            .client
            .call_tool(CallToolRequestParam { name: name.to_string().into(), arguments: mcp_args })
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
    fn fix_array_schemas(schema: Value) -> Value {
        match schema {
            Value::Object(mut obj) => {
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
                if let Some(properties) = obj.get_mut("properties") {
                    if let Value::Object(props) = properties {
                        for (_key, value) in props.iter_mut() {
                            *value = Self::fix_array_schemas(value.clone());
                        }
                    }
                }

                // Fix items if present (for nested arrays)
                if let Some(items) = obj.get_mut("items") {
                    *items = Self::fix_array_schemas(items.clone());
                }

                // Fix oneOf, anyOf, allOf schemas
                for key in &["oneOf", "anyOf", "allOf"] {
                    if let Some(Value::Array(schemas)) = obj.get_mut(*key) {
                        for schema in schemas.iter_mut() {
                            *schema = Self::fix_array_schemas(schema.clone());
                        }
                    }
                }

                // Fix additionalProperties if it's a schema
                if let Some(additional) = obj.get_mut("additionalProperties") {
                    if additional.is_object() {
                        *additional = Self::fix_array_schemas(additional.clone());
                    }
                }

                Value::Object(obj)
            }
            other => other,
        }
    }

    /// Convert an MCP tool to Windmill's ToolDef format
    fn mcp_tool_to_tooldef(mcp_tool: &McpTool, name: &str) -> Result<ToolDef> {
        let tool_name = format!("mcp_{}_{}", name, mcp_tool.name);

        let schema_value = serde_json::to_value(&*mcp_tool.input_schema)
            .context("Failed to convert MCP schema to JSON value")?;
        let fixed_schema = Self::fix_array_schemas(schema_value);
        let parameters: Box<RawValue> = to_raw_value(&fixed_schema);

        // Build the description from title and description
        let description = if let Some(title) = &mcp_tool.title {
            if let Some(desc) = &mcp_tool.description {
                Some(format!("{}: {}", title, desc))
            } else {
                Some(title.to_string())
            }
        } else {
            mcp_tool.description.as_ref().map(|d| d.to_string())
        };

        let tool_def_function =
            ToolDefFunction { name: tool_name.clone(), description, parameters };

        Ok(ToolDef { r#type: "function".to_string(), function: tool_def_function })
    }

    /// Convert OpenAI-style tool call arguments to MCP format
    /// OpenAI sends arguments as a JSON string, MCP expects a Map
    fn openai_args_to_mcp_args(
        args_str: &str,
    ) -> Result<Option<serde_json::Map<String, serde_json::Value>>> {
        if args_str.trim().is_empty() {
            return Ok(None);
        }

        let args_value: serde_json::Value =
            serde_json::from_str(args_str).context("Failed to parse tool call arguments")?;

        match args_value {
            serde_json::Value::Object(map) => Ok(Some(map)),
            serde_json::Value::Null => Ok(None),
            _ => Ok(Some(
                vec![("value".to_string(), args_value)]
                    .into_iter()
                    .collect(),
            )),
        }
    }
}
