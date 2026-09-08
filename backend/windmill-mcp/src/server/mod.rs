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
pub use crate::common::types::{McpToken, MultiWorkspaceMcp, WorkspaceInfo};
pub use backend::{BackendResult, McpAuth, McpBackend, McpRequest, PathFilter};
pub use endpoints::{
    endpoint_tool_to_mcp_tool, endpoint_tool_to_mcp_tool_multi, is_endpoint_read_only,
    list_workspaces_tool, non_empty_body_fields, EndpointTool,
};
pub use runner::{has_endpoint_path_policy, Runner};
pub use tools::create_tool_from_item;

// Re-export rmcp types for convenience
pub use rmcp::handler::server::ServerHandler;
pub use rmcp::model::{
    CallToolRequestParams, CallToolResult, ContentBlock, Implementation, InitializeRequestParams,
    InitializeResult, ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult,
    ListToolsResult, PaginatedRequestParams, ProtocolVersion, ServerCapabilities, ServerInfo, Tool,
    ToolAnnotations,
};
pub use rmcp::service::{RequestContext, RoleServer};
pub use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpService,
};
pub use rmcp::transport::StreamableHttpServerConfig;
pub use rmcp::ErrorData;
