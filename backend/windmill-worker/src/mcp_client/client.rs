//! Core MCP client implementation

use crate::mcp_client::types::{McpResource, McpToolSource};
use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use rmcp::model::InitializeRequestParam;
use rmcp::service::RunningService;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::RoleClient;
use rmcp::{
    model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation, Tool},
    transport::StreamableHttpClientTransport,
    ServiceExt,
};
use std::sync::Arc;

/// MCP client for communicating with external MCP servers
pub struct McpClient {
    /// The underlying rmcp client
    client: Arc<RunningService<RoleClient, InitializeRequestParam>>,
    /// Cached list of available tools from the server
    available_tools: Vec<Tool>,
    /// Name of the MCP resource for tracking tool sources
    name: String,
}

impl McpClient {
    /// Create a new MCP client from a resource configuration
    pub async fn from_resource(resource: McpResource) -> Result<Self> {
        tracing::debug!("Initializing MCP client for {}", resource.url);

        // Build custom reqwest client with headers if provided
        let mut headers = HeaderMap::new();
        if let Some(resource_headers) = &resource.headers {
            for (key, value) in resource_headers {
                match (
                    HeaderName::from_bytes(key.as_bytes()),
                    HeaderValue::from_str(value),
                ) {
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

        tracing::info!("Connected to MCP server: {:?}", client.peer_info());

        // Immediately fetch available tools
        let available_tools = client
            .list_tools(Default::default())
            .await
            .context("Failed to list tools from MCP server")?
            .tools;

        tracing::debug!("Discovered {} tools from MCP server", available_tools.len());

        Ok(Self { client: Arc::new(client), available_tools, name: resource.name })
    }

    /// Get the list of available tools from the MCP server
    pub fn available_tools(&self) -> &[Tool] {
        &self.available_tools
    }

    /// Call a tool on the MCP server
    ///
    /// # Arguments
    /// * `name` - The name of the tool to call
    /// * `arguments` - JSON arguments to pass to the tool
    ///
    /// # Returns
    /// The tool's result as a JSON value
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<serde_json::Value> {
        tracing::debug!("Calling MCP tool: {} with args: {:?}", name, arguments);

        let result = self
            .client
            .call_tool(CallToolRequestParam { name: name.to_string().into(), arguments })
            .await
            .context(format!("Failed to call MCP tool: {}", name))?;

        tracing::debug!("MCP tool {} returned: {:?}", name, result);

        // Convert the result to a JSON value
        // MCP tools return ToolResult which contains content array
        let result_json =
            serde_json::to_value(&result).context("Failed to serialize MCP tool result")?;

        Ok(result_json)
    }

    /// Create metadata for tracking this tool's source
    pub fn create_tool_source(&self, tool_name: &str) -> McpToolSource {
        McpToolSource { name: self.name.clone(), original_tool_name: tool_name.to_string() }
    }

    /// Cleanup and close the connection
    /// Note: This consumes self, so Drop won't be called
    pub async fn shutdown(self) -> Result<()> {
        // Clone the Arc to avoid moving out of self
        let client_clone = self.client.clone();
        // Drop self to release one reference
        drop(self);

        // Try to unwrap the Arc, if we're the only owner we can cancel
        if let Ok(client) = Arc::try_unwrap(client_clone) {
            client
                .cancel()
                .await
                .context("Failed to cancel MCP client")?;
        }
        Ok(())
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        tracing::debug!("Dropping MCP client for {}", self.name);
    }
}
