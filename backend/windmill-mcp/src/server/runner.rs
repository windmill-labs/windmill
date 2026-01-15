/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! MCP Server Runner implementation
//!
//! Contains the generic Runner that implements the MCP ServerHandler trait
//! and delegates to a McpBackend for actual functionality.

use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Content, Implementation, InitializeRequestParam,
    InitializeResult, ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult,
    ListToolsResult, PaginatedRequestParam, ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ErrorData;
use serde_json::Value;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use crate::common::scope::parse_mcp_scopes;
use crate::common::transform::{reverse_transform, reverse_transform_key};
use crate::common::types::{ResourceInfo, WorkspaceId};
use crate::server::backend::{BackendError, McpAuth, McpBackend};
use crate::server::endpoints::endpoint_tool_to_mcp_tool;
use crate::server::tools::create_tool_from_item;

/// MCP Server Runner - generic over the backend implementation
///
/// This struct implements the MCP ServerHandler trait and uses a McpBackend
/// to perform the actual operations (database queries, job execution, etc.)
pub struct Runner<B: McpBackend> {
    backend: Arc<B>,
    _marker: PhantomData<B>,
}

impl<B: McpBackend> Clone for Runner<B> {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            _marker: PhantomData,
        }
    }
}

impl<B: McpBackend> Runner<B> {
    /// Create a new Runner with the given backend
    pub fn new(backend: B) -> Self {
        Self {
            backend: Arc::new(backend),
            _marker: PhantomData,
        }
    }

    /// Extract authentication and workspace from request context
    fn extract_context(
        context: &RequestContext<RoleServer>,
    ) -> Result<(B::Auth, String), ErrorData> {
        let http_parts = context
            .extensions
            .get::<axum::http::request::Parts>()
            .ok_or_else(|| {
                tracing::error!("http::request::Parts not found");
                ErrorData::internal_error("http::request::Parts not found", None)
            })?;

        let auth = http_parts.extensions.get::<B::Auth>().ok_or_else(|| {
            tracing::error!("Auth extension not found");
            ErrorData::internal_error("Auth extension not found", None)
        })?;

        let workspace_id = http_parts
            .extensions
            .get::<WorkspaceId>()
            .ok_or_else(|| {
                tracing::error!("WorkspaceId not found");
                ErrorData::internal_error("WorkspaceId not found", None)
            })
            .map(|w_id| w_id.0.clone())?;

        // Validate MCP scope
        if !auth.has_mcp_scope() {
            tracing::error!("Unauthorized: missing mcp scope");
            return Err(ErrorData::internal_error(
                "Unauthorized: missing mcp scope",
                None,
            ));
        }

        Ok((auth.clone(), workspace_id))
    }
}

impl<B: McpBackend> ServerHandler for Runner<B> {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server provides a list of scripts and flows the user can run on Windmill. \
                 Each flow and script is a tool callable with their respective arguments."
                    .to_string(),
            ),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        Ok(self.get_info())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let (auth, workspace_id) = Self::extract_context(&context)?;

        // Parse MCP scopes to determine what to expose
        let scopes = auth.scopes().unwrap_or(&[]);
        let scope_config =
            parse_mcp_scopes(scopes).map_err(|e| ErrorData::internal_error(e, None))?;

        let favorites_only = scope_config.favorites;

        // Fetch all items concurrently
        let (scripts, flows, resource_types, hub_scripts) = tokio::try_join!(
            self.backend.list_scripts(&auth, &workspace_id, favorites_only),
            self.backend.list_flows(&auth, &workspace_id, favorites_only),
            self.backend.list_resource_types(&auth, &workspace_id),
            async {
                if let Some(ref apps) = scope_config.hub_apps {
                    self.backend.list_hub_scripts(Some(apps)).await
                } else {
                    Ok(vec![])
                }
            }
        )
        .map_err(|e: BackendError| ErrorData::internal_error(e.message, None))?;

        let mut resources_cache: HashMap<String, Vec<ResourceInfo>> = HashMap::new();
        let mut tools = Vec::new();

        // Filter and add scripts based on scope
        for script in scripts {
            if scope_config.granular && !scope_config.is_allowed("script", &script.path) {
                continue;
            }

            tools.push(
                create_tool_from_item(
                    &script,
                    self.backend.as_ref(),
                    &auth,
                    &workspace_id,
                    &mut resources_cache,
                    &resource_types,
                )
                .await
                .map_err(|e| ErrorData::internal_error(e.message, None))?,
            );
        }

        // Filter and add flows based on scope
        for flow in flows {
            if scope_config.granular && !scope_config.is_allowed("flow", &flow.path) {
                continue;
            }

            tools.push(
                create_tool_from_item(
                    &flow,
                    self.backend.as_ref(),
                    &auth,
                    &workspace_id,
                    &mut resources_cache,
                    &resource_types,
                )
                .await
                .map_err(|e| ErrorData::internal_error(e.message, None))?,
            );
        }

        // Add hub scripts
        for hub_script in hub_scripts {
            tools.push(
                create_tool_from_item(
                    &hub_script,
                    self.backend.as_ref(),
                    &auth,
                    &workspace_id,
                    &mut resources_cache,
                    &resource_types,
                )
                .await
                .map_err(|e| ErrorData::internal_error(e.message, None))?,
            );
        }

        // Add endpoint tools from the generated MCP tools, filtered by scope
        let endpoint_tools = self.backend.all_endpoint_tools();
        for endpoint_tool in endpoint_tools {
            if scope_config.granular && !scope_config.is_allowed("endpoint", &endpoint_tool.name) {
                continue;
            }

            tools.push(endpoint_tool_to_mcp_tool(&endpoint_tool));
        }

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let (auth, workspace_id) = Self::extract_context(&context)?;

        // Parse MCP scopes for authorization
        let scopes = auth.scopes().unwrap_or(&[]);
        let scope_config =
            parse_mcp_scopes(scopes).map_err(|e| ErrorData::internal_error(e, None))?;

        // Handle truncated tool names
        if request.name.ends_with("_TRUNC") {
            return Ok(CallToolResult::error(vec![rmcp::model::Annotated::new(
                rmcp::model::RawContent::Text(rmcp::model::RawTextContent {
                    text: "Tool path is too long. Consider shortening it to make it compatible with MCP.".to_string(),
                    meta: None,
                }),
                None,
            )]));
        }

        let args = request
            .arguments
            .map(Value::Object)
            .unwrap_or(Value::Null);

        // Check if this is an endpoint tool
        let endpoint_tools = self.backend.all_endpoint_tools();
        for endpoint_tool in &endpoint_tools {
            if endpoint_tool.name.as_ref() == request.name {
                // Validate endpoint scope
                if scope_config.granular
                    && !scope_config.is_allowed("endpoint", &endpoint_tool.name)
                {
                    return Err(ErrorData::internal_error(
                        format!(
                            "Access denied: endpoint '{}' not in token scope",
                            endpoint_tool.name
                        ),
                        None,
                    ));
                }

                // This is an endpoint tool, call via backend
                let result = self
                    .backend
                    .call_endpoint(&auth, &workspace_id, endpoint_tool, args)
                    .await
                    .map_err(|e| ErrorData::internal_error(e.message, None))?;

                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()),
                )]));
            }
        }

        // Not an endpoint tool - parse as script/flow
        let (tool_type, path, is_hub) = reverse_transform(&request.name)
            .map_err(|e| ErrorData::internal_error(format!("Failed to parse tool name: {}", e), None))?;

        // Validate script/flow scope
        if !is_hub && scope_config.granular {
            if tool_type == "script" && !scope_config.is_allowed("script", &path) {
                return Err(ErrorData::internal_error(
                    format!("Access denied: script '{}' not in token scope", path),
                    None,
                ));
            } else if tool_type == "flow" && !scope_config.is_allowed("flow", &path) {
                return Err(ErrorData::internal_error(
                    format!("Access denied: flow '{}' not in token scope", path),
                    None,
                ));
            }
        }

        // Get item schema for argument transformation
        let item_schema = if is_hub {
            self.backend
                .get_hub_script_schema(&format!("hub/{}", path))
                .await
                .map_err(|e| ErrorData::internal_error(e.message, None))?
        } else {
            self.backend
                .get_item_schema(&auth, &workspace_id, &path, tool_type)
                .await
                .map_err(|e| ErrorData::internal_error(e.message, None))?
        };

        // Transform arguments back to original key names
        let transformed_args = if let Value::Object(map) = args {
            let mut args_hash = HashMap::new();
            for (k, v) in map {
                let original_key = reverse_transform_key(&k, &item_schema);
                args_hash.insert(original_key, v);
            }
            Value::Object(args_hash.into_iter().collect())
        } else {
            args
        };

        let script_or_flow_path = if is_hub {
            format!("hub/{}", path)
        } else {
            path
        };

        // Execute script or flow
        let result = if tool_type == "script" {
            self.backend
                .run_script(&auth, &workspace_id, &script_or_flow_path, transformed_args)
                .await
        } else {
            self.backend
                .run_flow(&auth, &workspace_id, &script_or_flow_path, transformed_args)
                .await
        };

        match result {
            Ok(value) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string()),
            )])),
            Err(e) => Err(ErrorData::internal_error(
                format!("Failed to run {}: {}", tool_type, e.message),
                None,
            )),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
            meta: None,
        })
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        Ok(ListPromptsResult::default())
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        Ok(ListResourceTemplatesResult::default())
    }
}
