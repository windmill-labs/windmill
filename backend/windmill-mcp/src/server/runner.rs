//! MCP Server Runner implementation
//!
//! Contains the generic Runner that implements the MCP ServerHandler trait
//! and delegates to a McpBackend for actual functionality.

use crate::common::schema::extract_resource_types_from_schema;
use crate::common::scope::parse_mcp_scopes;
use crate::common::transform::{
    extract_hub_version_id_from_hashed, extract_path_prefix_from_hashed, parse_tool_prefix,
    reverse_transform, reverse_transform_key,
};
use crate::common::types::{McpToken, MultiWorkspaceMcp, ResourceInfo, ToolableItem, WorkspaceId};
use crate::server::backend::{McpAuth, McpBackend, PathFilter};
use crate::server::endpoints::{
    endpoint_tool_to_mcp_tool, endpoint_tool_to_mcp_tool_multi, list_workspaces_tool,
};
use crate::server::tools::create_tool_from_item;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, InitializeRequestParams,
    InitializeResult, ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult,
    ListToolsResult, PaginatedRequestParams, ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ErrorData;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// Re-export from http crate for extracting request parts
use http::request::Parts as HttpParts;

/// Maximum size in bytes for tool result text content.
/// Anthropic enforces a 25,000 token limit per tool result.
/// At ~3.5 bytes/token, 87,500 bytes is a safe upper bound.
const MAX_TOOL_RESULT_BYTES: usize = 87_500;

fn truncate_tool_result(text: String) -> String {
    if text.len() <= MAX_TOOL_RESULT_BYTES {
        return text;
    }
    // Truncate at a char boundary
    let mut end = MAX_TOOL_RESULT_BYTES;
    while !text.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    let mut truncated = text[..end].to_string();
    truncated.push_str("\n\n[truncated — result exceeded 25,000 token limit]");
    truncated
}

/// MCP Server Runner - generic over the backend implementation
///
/// This struct implements the MCP ServerHandler trait and uses a McpBackend
/// to perform the actual operations (database queries, job execution, etc.)
pub struct Runner<B: McpBackend> {
    backend: Arc<B>,
}

impl<B: McpBackend> Clone for Runner<B> {
    fn clone(&self) -> Self {
        Self { backend: self.backend.clone() }
    }
}

/// Whether the request targets one bound workspace or spans every workspace the
/// token can access.
enum McpMode {
    /// A single workspace, resolved from the URL path or the token's bound
    /// workspace. Tools operate against this workspace implicitly.
    Single(String),
    /// The token has no bound workspace (`workspace_id IS NULL`). Tools take an
    /// explicit `workspace_id` argument; the wrapped value is the raw bearer
    /// token, used to re-resolve auth per requested workspace.
    Multi(String),
}

impl<B: McpBackend> Runner<B> {
    /// Create a new Runner with the given backend
    pub fn new(backend: B) -> Self {
        Self { backend: Arc::new(backend) }
    }

    /// Extract authentication and the workspace mode from request context
    fn extract_context(
        context: &RequestContext<RoleServer>,
    ) -> Result<(B::Auth, McpMode), ErrorData> {
        let http_parts = context.extensions.get::<HttpParts>().ok_or_else(|| {
            tracing::error!("http::request::Parts not found");
            ErrorData::internal_error("http::request::Parts not found", None)
        })?;

        let auth = http_parts.extensions.get::<B::Auth>().ok_or_else(|| {
            tracing::error!("Auth extension not found");
            ErrorData::internal_error("Auth extension not found", None)
        })?;

        // Validate MCP scope
        if !auth.has_mcp_scope() {
            tracing::error!("Unauthorized: missing mcp scope");
            return Err(ErrorData::internal_error(
                "Unauthorized: missing mcp scope",
                None,
            ));
        }

        let mode = if http_parts.extensions.get::<MultiWorkspaceMcp>().is_some() {
            let token = http_parts.extensions.get::<McpToken>().ok_or_else(|| {
                tracing::error!("MultiWorkspaceMcp set but McpToken missing");
                ErrorData::internal_error("MCP token not found for multi-workspace session", None)
            })?;
            McpMode::Multi(token.0.clone())
        } else {
            let workspace_id = http_parts
                .extensions
                .get::<WorkspaceId>()
                .ok_or_else(|| {
                    tracing::error!("WorkspaceId not found");
                    ErrorData::internal_error("WorkspaceId not found", None)
                })
                .map(|w_id| w_id.0.clone())?;
            McpMode::Single(workspace_id)
        };

        Ok((auth.clone(), mode))
    }
}

/// The run-by-path endpoint tools execute an arbitrary script/flow named by a
/// `path` argument. In multi-workspace mode they are the only way to run
/// scripts/flows, so their authorization must honor the `mcp:scripts:` /
/// `mcp:flows:` path scopes (not the generic endpoint scope) — otherwise a
/// granular token could run items outside its allowed paths. Returns the scope
/// resource type ("script"/"flow") for these endpoints, `None` otherwise.
fn run_by_path_scope_kind(endpoint_name: &str) -> Option<&'static str> {
    match endpoint_name {
        "runScriptByPath" => Some("script"),
        "runFlowByPath" => Some("flow"),
        _ => None,
    }
}

fn find_matching_path<T: ToolableItem>(candidates: Vec<T>, request_name: &str) -> Option<String> {
    candidates
        .into_iter()
        .find(|item| item.get_transformed_path() == request_name)
        .map(|item| item.get_full_path().to_string())
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
        _request: InitializeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        Ok(self.get_info())
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let (auth, mode) = Self::extract_context(&context)?;

        // Parse MCP scopes to determine what to expose
        let scopes = auth.scopes().unwrap_or(&[]);
        let scope_config =
            parse_mcp_scopes(scopes).map_err(|e| ErrorData::internal_error(e, None))?;

        let read_only = auth.read_only();

        match mode {
            McpMode::Single(workspace_id) => {
                self.list_tools_single(&auth, &workspace_id, &scope_config, read_only)
                    .await
            }
            // Multi-workspace: expose the generic endpoint tools (each taking an
            // explicit workspace_id) plus list_workspaces. Per-workspace scripts
            // and flows are intentionally not enumerated here — doing so across
            // every workspace would overload the tool list; callers run them via
            // runScriptByPath / runFlowByPath with a workspace_id instead.
            McpMode::Multi(_) => Ok(self.list_tools_multi(&scope_config, read_only)),
        }
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let (auth, mode) = Self::extract_context(&context)?;

        // Parse MCP scopes for authorization
        let scopes = auth.scopes().unwrap_or(&[]);
        let scope_config =
            parse_mcp_scopes(scopes).map_err(|e| ErrorData::internal_error(e, None))?;
        let read_only = auth.read_only();

        let args = request.arguments.map(Value::Object).unwrap_or(Value::Null);

        match mode {
            McpMode::Single(workspace_id) => {
                self.call_tool_single(
                    &auth,
                    &workspace_id,
                    &scope_config,
                    read_only,
                    request.name,
                    args,
                )
                .await
            }
            McpMode::Multi(token) => {
                self.call_tool_multi(&auth, &token, &scope_config, read_only, request.name, args)
                    .await
            }
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult { resources: vec![], next_cursor: None, meta: None })
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        Ok(ListPromptsResult::default())
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        Ok(ListResourceTemplatesResult::default())
    }
}

impl<B: McpBackend> Runner<B> {
    /// List tools for a single, bound workspace (URL-path or token-bound).
    async fn list_tools_single(
        &self,
        auth: &B::Auth,
        workspace_id: &str,
        scope_config: &crate::common::scope::McpScopeConfig,
        read_only: bool,
    ) -> Result<ListToolsResult, ErrorData> {
        let favorites_only = scope_config.favorites;

        let mut tools = Vec::new();

        // Read-only tokens cannot run scripts/flows/hub-scripts (running is a
        // mutating action), so skip the script/flow/hub/resource fetches
        // entirely — they would only be discarded below.
        if !read_only {
            // For granular tokens, push the scope patterns into the SQL query so
            // in-scope items survive the fetch cap (see `PathFilter`). The Rust
            // filter below still runs as a defense-in-depth check. Non-granular
            // (favorites/all) tokens are already narrowed by the favorites join
            // or intentionally unfiltered.
            let script_filter = scope_config
                .granular
                .then(|| PathFilter::Patterns(scope_config.scripts.as_slice()));
            let flow_filter = scope_config
                .granular
                .then(|| PathFilter::Patterns(scope_config.flows.as_slice()));

            let (scripts, flows, resource_types, hub_scripts) = tokio::try_join!(
                self.backend
                    .list_scripts(auth, workspace_id, favorites_only, script_filter),
                self.backend
                    .list_flows(auth, workspace_id, favorites_only, flow_filter),
                self.backend.list_resource_types(auth, workspace_id),
                async {
                    if let Some(ref apps) = scope_config.hub_apps {
                        self.backend.list_hub_scripts(Some(apps)).await
                    } else {
                        Ok(vec![])
                    }
                }
            )?;

            let filtered_scripts: Vec<_> = scripts
                .into_iter()
                .filter(|s| !scope_config.granular || scope_config.is_allowed("script", &s.path))
                .collect();

            let filtered_flows: Vec<_> = flows
                .into_iter()
                .filter(|f| !scope_config.granular || scope_config.is_allowed("flow", &f.path))
                .collect();

            // Collect all needed resource types from all schemas
            let mut needed_resource_types: HashSet<String> = HashSet::new();
            for script in &filtered_scripts {
                needed_resource_types
                    .extend(extract_resource_types_from_schema(&script.get_schema()));
            }
            for flow in &filtered_flows {
                needed_resource_types
                    .extend(extract_resource_types_from_schema(&flow.get_schema()));
            }
            for hub_script in &hub_scripts {
                needed_resource_types
                    .extend(extract_resource_types_from_schema(&hub_script.get_schema()));
            }

            // Pre-fetch all resources
            let resource_futures: Vec<_> = needed_resource_types
                .into_iter()
                .map(|rt| {
                    let backend = self.backend.clone();
                    let auth = auth.clone();
                    let workspace_id = workspace_id.to_string();
                    async move {
                        backend
                            .list_resources(&auth, &workspace_id, &rt)
                            .await
                            .map(|resources| (rt, resources))
                    }
                })
                .collect();

            let resource_results = futures::future::try_join_all(resource_futures).await?;
            let resources_cache: HashMap<String, Vec<ResourceInfo>> =
                resource_results.into_iter().collect();

            for script in &filtered_scripts {
                tools.push(create_tool_from_item(
                    script,
                    self.backend.as_ref(),
                    &resources_cache,
                    &resource_types,
                ));
            }

            for flow in &filtered_flows {
                tools.push(create_tool_from_item(
                    flow,
                    self.backend.as_ref(),
                    &resources_cache,
                    &resource_types,
                ));
            }

            for hub_script in &hub_scripts {
                tools.push(create_tool_from_item(
                    hub_script,
                    self.backend.as_ref(),
                    &resources_cache,
                    &resource_types,
                ));
            }
        }

        // Add endpoint tools from the generated MCP tools, filtered by scope
        let endpoint_tools = self.backend.all_endpoint_tools();
        for endpoint_tool in endpoint_tools {
            if scope_config.granular && !scope_config.is_allowed("endpoint", &endpoint_tool.name) {
                continue;
            }
            if read_only && !crate::server::is_endpoint_read_only(&endpoint_tool) {
                continue;
            }

            tools.push(endpoint_tool_to_mcp_tool(&endpoint_tool));
        }

        Ok(ListToolsResult { tools, next_cursor: None, meta: None })
    }

    /// Handle a tool call for a single, bound workspace.
    async fn call_tool_single(
        &self,
        auth: &B::Auth,
        workspace_id: &str,
        scope_config: &crate::common::scope::McpScopeConfig,
        read_only: bool,
        name: std::borrow::Cow<'static, str>,
        args: Value,
    ) -> Result<CallToolResult, ErrorData> {
        // Check if this is an endpoint tool
        let endpoint_tools = self.backend.all_endpoint_tools();
        for endpoint_tool in &endpoint_tools {
            if endpoint_tool.name.as_ref() == name.as_ref() {
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
                if read_only && !crate::server::is_endpoint_read_only(endpoint_tool) {
                    return Err(ErrorData::internal_error(
                        format!(
                            "Access denied: endpoint '{}' is not read-only and this token is restricted to read-only operations",
                            endpoint_tool.name
                        ),
                        None,
                    ));
                }

                // This is an endpoint tool, call via backend
                let result = self
                    .backend
                    .call_endpoint(auth, workspace_id, endpoint_tool, args)
                    .await
                    .map_err(|e| ErrorData::internal_error(e.message, None))?;

                return Ok(CallToolResult::success(vec![Content::text(
                    truncate_tool_result(
                        serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()),
                    ),
                )]));
            }
        }

        // Anything below this point runs a script or flow, which is a mutating
        // action and must be denied for read-only tokens.
        if read_only {
            return Err(ErrorData::internal_error(
                format!(
                    "Access denied: tool '{}' runs a script/flow and this token is restricted to read-only operations",
                    name
                ),
                None,
            ));
        }

        // Resolve the tool name to (type, path, is_hub)
        let (type_str, is_hub, is_hashed) = parse_tool_prefix(name.as_ref()).map_err(|e| {
            ErrorData::internal_error(format!("Failed to parse tool name: {}", e), None)
        })?;

        let (tool_type, path, is_hub) = if !is_hashed {
            reverse_transform(name.as_ref()).map_err(|e| {
                ErrorData::internal_error(format!("Failed to parse tool name: {}", e), None)
            })?
        } else if is_hub {
            let version_id = extract_hub_version_id_from_hashed(name.as_ref()).map_err(|e| {
                ErrorData::internal_error(format!("Failed to extract hub version_id: {}", e), None)
            })?;
            (type_str, version_id, true)
        } else {
            let path_prefix = extract_path_prefix_from_hashed(name.as_ref());
            let path_filter = path_prefix.as_deref().map(PathFilter::Prefix);
            let favorites_only = scope_config.favorites;
            let matched_path = if type_str == "script" {
                find_matching_path(
                    self.backend
                        .list_scripts(auth, workspace_id, favorites_only, path_filter)
                        .await
                        .map_err(|e| ErrorData::internal_error(e.message, None))?,
                    name.as_ref(),
                )
            } else {
                find_matching_path(
                    self.backend
                        .list_flows(auth, workspace_id, favorites_only, path_filter)
                        .await
                        .map_err(|e| ErrorData::internal_error(e.message, None))?,
                    name.as_ref(),
                )
            };

            let matched_path = matched_path.ok_or_else(|| {
                ErrorData::internal_error(
                    format!("No {} found matching hashed tool name '{}'", type_str, name),
                    None,
                )
            })?;

            (type_str, matched_path, false)
        };

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
                .get_item_schema(auth, workspace_id, &path, tool_type)
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
                .run_script(auth, workspace_id, &script_or_flow_path, transformed_args)
                .await
        } else {
            self.backend
                .run_flow(auth, workspace_id, &script_or_flow_path, transformed_args)
                .await
        };

        match result {
            Ok(value) => Ok(CallToolResult::success(vec![Content::text(
                truncate_tool_result(
                    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string()),
                ),
            )])),
            Err(e) => Err(ErrorData::internal_error(
                format!("Failed to run {}: {}", tool_type, e.message),
                None,
            )),
        }
    }

    /// List tools for a multi-workspace session: the synthetic `list_workspaces`
    /// tool plus every generic endpoint tool, each taking an explicit
    /// `workspace_id` argument.
    fn list_tools_multi(
        &self,
        scope_config: &crate::common::scope::McpScopeConfig,
        read_only: bool,
    ) -> ListToolsResult {
        let mut tools = vec![list_workspaces_tool()];

        let endpoint_tools = self.backend.all_endpoint_tools();
        for endpoint_tool in endpoint_tools {
            // Run-by-path tools are gated by script/flow scope (they run an
            // arbitrary path); every other endpoint by the endpoint scope.
            let allowed = match run_by_path_scope_kind(&endpoint_tool.name) {
                Some(kind) => scope_config.has_any(kind),
                None => {
                    !scope_config.granular
                        || scope_config.is_allowed("endpoint", &endpoint_tool.name)
                }
            };
            if !allowed {
                continue;
            }
            if read_only && !crate::server::is_endpoint_read_only(&endpoint_tool) {
                continue;
            }

            tools.push(endpoint_tool_to_mcp_tool_multi(&endpoint_tool));
        }

        ListToolsResult { tools, next_cursor: None, meta: None }
    }

    /// Handle a tool call for a multi-workspace session. `base_auth` is the
    /// workspace-less identity derived from the token; per-workspace auth is
    /// resolved on demand from `token` for the workspace named in the args.
    async fn call_tool_multi(
        &self,
        base_auth: &B::Auth,
        token: &str,
        scope_config: &crate::common::scope::McpScopeConfig,
        read_only: bool,
        name: std::borrow::Cow<'static, str>,
        args: Value,
    ) -> Result<CallToolResult, ErrorData> {
        if name.as_ref() == "list_workspaces" {
            let workspaces = self
                .backend
                .list_accessible_workspaces(base_auth)
                .await
                .map_err(|e| ErrorData::internal_error(e.message, None))?;
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&workspaces).unwrap_or_else(|_| "[]".to_string()),
            )]));
        }

        // Only endpoint tools are exposed in multi-workspace mode; scripts and
        // flows are run through the runScriptByPath / runFlowByPath endpoints.
        let endpoint_tools = self.backend.all_endpoint_tools();
        let endpoint_tool = endpoint_tools
            .iter()
            .find(|t| t.name.as_ref() == name.as_ref())
            .ok_or_else(|| {
                ErrorData::invalid_params(
                    format!(
                        "Unknown tool '{}' in multi-workspace mode. Available tools are list_workspaces and the generic API endpoint tools (run scripts/flows via runScriptByPath / runFlowByPath).",
                        name
                    ),
                    None,
                )
            })?;

        // Authorize the tool. Run-by-path endpoints (runScriptByPath /
        // runFlowByPath) run an arbitrary `path` and must be checked against the
        // script/flow scope for that path — the endpoint scope alone would let a
        // granular token run items outside its allowed paths.
        match run_by_path_scope_kind(&endpoint_tool.name) {
            Some(kind) => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| {
                        ErrorData::invalid_params(
                            format!(
                                "Missing required 'path' argument for tool '{}'.",
                                endpoint_tool.name
                            ),
                            None,
                        )
                    })?;
                // No `granular` gate: is_allowed already encodes every mode —
                // true for mcp:all, pattern-matched for granular scopes, and
                // false for mcp:favorites (a favorites token can't run an
                // arbitrary path, only its enumerated favorites).
                if !scope_config.is_allowed(kind, path) {
                    return Err(ErrorData::internal_error(
                        format!("Access denied: {} '{}' not in token scope", kind, path),
                        None,
                    ));
                }
            }
            None => {
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
            }
        }
        if read_only && !crate::server::is_endpoint_read_only(endpoint_tool) {
            return Err(ErrorData::internal_error(
                format!(
                    "Access denied: endpoint '{}' is not read-only and this token is restricted to read-only operations",
                    endpoint_tool.name
                ),
                None,
            ));
        }

        // Workspace-scoped endpoints need an explicit target workspace and a
        // per-workspace auth; global endpoints (e.g. docs) use the base identity.
        let needs_workspace = endpoint_tool.path.contains("{workspace}");
        let (workspace_id, resolved_auth) = if needs_workspace {
            let workspace_id = args
                .get("workspace_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    ErrorData::invalid_params(
                        format!(
                            "Missing required 'workspace_id' argument for tool '{}'. Call list_workspaces to see the workspaces you can access.",
                            endpoint_tool.name
                        ),
                        None,
                    )
                })?
                .to_string();

            let resolved = self
                .backend
                .resolve_workspace_auth(token, &workspace_id)
                .await
                .map_err(|e| ErrorData::internal_error(e.message, None))?;
            (workspace_id, resolved)
        } else {
            (String::new(), base_auth.clone())
        };

        // `workspace_id` is a synthetic argument only this layer understands; the
        // target workspace is passed to call_endpoint separately. Strip it so it
        // can't leak into a pass-through request body (e.g. runScriptByPath, whose
        // body forwards all remaining args as the script's arguments).
        let mut args = args;
        if let Value::Object(map) = &mut args {
            map.remove("workspace_id");
        }

        let result = self
            .backend
            .call_endpoint(&resolved_auth, &workspace_id, endpoint_tool, args)
            .await
            .map_err(|e| ErrorData::internal_error(e.message, None))?;

        Ok(CallToolResult::success(vec![Content::text(
            truncate_tool_result(
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()),
            ),
        )]))
    }
}
