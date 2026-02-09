//! Windmill MCP (Model Context Protocol) implementation
//!
//! This crate provides:
//! - MCP client for connecting to external MCP servers (used by AI agents)
//! - Common types and utilities for MCP implementations
//! - MCP server types (when `server` feature is enabled)
//! - OAuth support (when `auth` feature is enabled)

// Common types and utilities module
pub mod common;

// Client module
pub mod client;

// Re-export common types at crate root for convenience
pub use common::{
    convert_schema_to_schema_type, is_resource_allowed, parse_mcp_scopes, transform_path, FlowInfo,
    HubResponse, HubScriptInfo, ItemSchema, McpScopeConfig, ResourceInfo, ResourceType, SchemaType,
    ScriptInfo, ToolableItem, WorkspaceId,
};

// Re-export client types at crate root for backward compatibility
pub use client::{McpClient, McpResource, McpToolSource};

// Re-export rmcp types for client usage
pub use rmcp::model::Tool as McpTool;

// Server module (when server feature is enabled)
#[cfg(feature = "server")]
pub mod server;

// MCP OAuth client registration (when auth feature is enabled)
#[cfg(feature = "auth")]
pub mod client_registration;

// Re-export rmcp auth types when auth feature is enabled
#[cfg(feature = "auth")]
pub mod oauth {
    //! Re-exports of rmcp auth and oauth2 types for MCP OAuth implementations

    pub use rmcp::transport::auth::AuthorizationManager;

    // Re-export oauth2 types needed for MCP OAuth flow
    pub use oauth2::{
        basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
        RedirectUrl, Scope, TokenUrl,
    };
}
