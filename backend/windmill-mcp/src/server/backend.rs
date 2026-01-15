//! MCP Backend trait definitions
//!
//! This module defines the traits that must be implemented by the backend
//! (typically windmill-api) to provide the actual functionality for the MCP server.

use async_trait::async_trait;
use rmcp::ErrorData;
use serde_json::Value;
use std::collections::HashMap;

use crate::common::types::{
    FlowInfo, HubScriptInfo, ResourceInfo, ResourceType, SchemaType, ScriptInfo,
};
use crate::server::endpoints::EndpointTool;

/// Result type for backend operations using rmcp's ErrorData directly
pub type BackendResult<T> = Result<T, ErrorData>;

/// Authentication context required by the MCP server
pub trait McpAuth: Send + Sync + Clone + 'static {
    /// Get the username
    fn username(&self) -> &str;
    /// Get the email
    fn email(&self) -> &str;
    /// Check if user is admin
    fn is_admin(&self) -> bool;
    /// Check if user is operator
    fn is_operator(&self) -> bool;
    /// Get user's groups
    fn groups(&self) -> &[String];
    /// Get user's folders as (name, can_write, is_owner)
    fn folders(&self) -> &[(String, bool, bool)];
    /// Get token scopes
    fn scopes(&self) -> Option<&[String]>;

    /// Check if the user has an MCP scope
    fn has_mcp_scope(&self) -> bool {
        self.scopes()
            .map(|s| s.iter().any(|scope| scope.starts_with("mcp:")))
            .unwrap_or(false)
    }
}

/// The core backend trait that windmill-api implements
///
/// This trait abstracts the windmill-api specific operations needed by the MCP server.
/// By implementing this trait, windmill-api can inject its database access, job execution,
/// and other functionality without windmill-mcp needing to depend on windmill-api directly.
#[async_trait]
pub trait McpBackend: Send + Sync + Clone + 'static {
    /// The authentication context type
    type Auth: McpAuth;

    // ─────────────────────────────────────────────────────────────────
    // Listing Operations
    // ─────────────────────────────────────────────────────────────────

    /// List scripts, optionally filtered to favorites only
    async fn list_scripts(
        &self,
        auth: &Self::Auth,
        workspace_id: &str,
        favorites_only: bool,
    ) -> BackendResult<Vec<ScriptInfo>>;

    /// List flows, optionally filtered to favorites only
    async fn list_flows(
        &self,
        auth: &Self::Auth,
        workspace_id: &str,
        favorites_only: bool,
    ) -> BackendResult<Vec<FlowInfo>>;

    /// List resource types in workspace
    async fn list_resource_types(
        &self,
        auth: &Self::Auth,
        workspace_id: &str,
    ) -> BackendResult<Vec<ResourceType>>;

    /// List resources of a specific type
    async fn list_resources(
        &self,
        auth: &Self::Auth,
        workspace_id: &str,
        resource_type: &str,
    ) -> BackendResult<Vec<ResourceInfo>>;

    /// List hub scripts, optionally filtered by app integrations
    async fn list_hub_scripts(&self, app_filter: Option<&str>)
        -> BackendResult<Vec<HubScriptInfo>>;

    // ─────────────────────────────────────────────────────────────────
    // Schema Operations
    // ─────────────────────────────────────────────────────────────────

    /// Get schema for a script or flow
    async fn get_item_schema(
        &self,
        auth: &Self::Auth,
        workspace_id: &str,
        path: &str,
        item_type: &str,
    ) -> BackendResult<Option<SchemaType>>;

    /// Get schema for a hub script
    async fn get_hub_script_schema(&self, path: &str) -> BackendResult<Option<SchemaType>>;

    // ─────────────────────────────────────────────────────────────────
    // Schema Transformation (requires DB access for resources)
    // ─────────────────────────────────────────────────────────────────

    /// Transform schema for resources by enriching with available resource information
    async fn transform_schema_for_resources(
        &self,
        schema: &SchemaType,
        auth: &Self::Auth,
        workspace_id: &str,
        resources_cache: &mut HashMap<String, Vec<ResourceInfo>>,
        resources_types: &[ResourceType],
    ) -> BackendResult<SchemaType>;

    // ─────────────────────────────────────────────────────────────────
    // Execution Operations
    // ─────────────────────────────────────────────────────────────────

    /// Run a script and wait for result
    async fn run_script(
        &self,
        auth: &Self::Auth,
        workspace_id: &str,
        path: &str,
        args: Value,
    ) -> BackendResult<Value>;

    /// Run a flow and wait for result
    async fn run_flow(
        &self,
        auth: &Self::Auth,
        workspace_id: &str,
        path: &str,
        args: Value,
    ) -> BackendResult<Value>;

    /// Call an endpoint tool (generated API endpoint)
    async fn call_endpoint(
        &self,
        auth: &Self::Auth,
        workspace_id: &str,
        endpoint_tool: &EndpointTool,
        args: Value,
    ) -> BackendResult<Value>;

    // ─────────────────────────────────────────────────────────────────
    // Endpoint Tools
    // ─────────────────────────────────────────────────────────────────

    /// Get all available endpoint tools
    fn all_endpoint_tools(&self) -> Vec<EndpointTool>;
}
