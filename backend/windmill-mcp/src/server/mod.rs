//! MCP Server module
//!
//! This module provides the MCP server implementation including:
//! - `McpBackend` trait for backend implementations
//! - `Runner` struct that implements the MCP protocol
//! - `setup_mcp_server` function for creating the server router
//! - Re-exports of rmcp types

pub mod backend;
pub mod endpoints;
pub mod runner;
pub mod tools;

// Re-export main types
pub use backend::{BackendResult, McpAuth, McpBackend};
pub use endpoints::{endpoint_tool_to_mcp_tool, EndpointTool};
pub use runner::Runner;
pub use tools::create_tool_from_item;

// Re-export rmcp types for convenience
pub use rmcp::handler::server::ServerHandler;
pub use rmcp::model::{
    Annotated, CallToolRequestParam, CallToolResult, Content, Implementation,
    InitializeRequestParam, InitializeResult, ListPromptsResult, ListResourceTemplatesResult,
    ListResourcesResult, ListToolsResult, PaginatedRequestParam, ProtocolVersion, RawContent,
    RawTextContent, ServerCapabilities, ServerInfo, Tool, ToolAnnotations,
};
pub use rmcp::service::{RequestContext, RoleServer};
pub use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpService,
};
pub use rmcp::transport::StreamableHttpServerConfig;
pub use rmcp::ErrorData;

use axum::Router;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Setup the MCP server with HTTP transport
///
/// This function creates a Router that can be nested into the main application router.
/// The backend parameter provides the actual implementation for database queries,
/// job execution, and other functionality.
///
/// # Type Parameters
/// - `B`: The backend type implementing `McpBackend`
///
/// # Returns
/// A tuple of (Router, CancellationToken) where the router handles MCP requests
/// and the cancellation token can be used for graceful shutdown.
pub async fn setup_mcp_server<B: McpBackend>(
    backend: B,
) -> anyhow::Result<(Router, CancellationToken)> {
    let cancellation_token = CancellationToken::new();
    let session_manager = Arc::new(LocalSessionManager::default());

    let runner = Runner::new(backend);

    let service_config = StreamableHttpServerConfig {
        sse_keep_alive: Some(Duration::from_secs(15)),
        stateful_mode: false,
        cancellation_token: cancellation_token.clone(),
    };

    let service =
        StreamableHttpService::new(move || Ok(runner.clone()), session_manager, service_config);

    let router = Router::new().nest_service("/", service);
    Ok((router, cancellation_token))
}
