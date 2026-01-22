//! MCP Server module
//!
//! This module provides the MCP server implementation including:
//! - `McpBackend` trait for backend implementations
//! - `Runner` struct that implements the MCP protocol
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
