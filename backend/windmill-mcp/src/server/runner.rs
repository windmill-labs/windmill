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
use crate::common::types::{
    McpToken, MultiWorkspaceMcp, ResourceInfo, SchemaType, ToolableItem, WorkspaceId,
};
use crate::server::backend::{McpAuth, McpBackend, McpRequest, PathFilter};
use crate::server::endpoints::{
    endpoint_tool_to_mcp_tool, endpoint_tool_to_mcp_tool_multi, list_workspaces_tool, EndpointTool,
};
use crate::server::tools::create_tool_from_item;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CacheScope, CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock,
    Implementation, InitializeResult, ListPromptsResult, ListResourceTemplatesResult,
    ListResourcesResult, ListToolsResult, PaginatedRequestParams, ProtocolVersion,
    ServerCapabilities, ServerInfo,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ErrorData;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Protocol revisions this server is willing to speak. `2026-07-28` is served
/// statelessly with per-request metadata; the older revisions keep the
/// `initialize` handshake, so both eras are answered on the same endpoint.
const SUPPORTED_PROTOCOL_VERSIONS: &[ProtocolVersion] = &[
    ProtocolVersion::V_2024_11_05,
    ProtocolVersion::V_2025_03_26,
    ProtocolVersion::V_2025_06_18,
    ProtocolVersion::V_2025_11_25,
    ProtocolVersion::V_2026_07_28,
];

/// SEP-2549 cache hints, required on every list result at `2026-07-28` — rmcp
/// leaves them unset, and a strict client rejects the response without them.
///
/// Zero because nothing here is cacheable: the listing is rebuilt from the
/// workspace's scripts and flows, which change at any time, and this server
/// advertises no `listChanged` capability, so a client that cached a stale list
/// would have no way to learn it had gone stale.
const LIST_TTL_MS: u64 = 0;
/// Every listing is filtered by the caller's token scopes and workspace
/// membership, so no two callers necessarily see the same tools — a shared
/// cache entry would leak one token's view to another.
const LIST_CACHE_SCOPE: CacheScope = CacheScope::Private;

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

/// Everything a request carries besides its MCP payload.
struct McpContext<A> {
    auth: A,
    mode: McpMode,
    headers: http::HeaderMap,
}

impl<B: McpBackend> Runner<B> {
    /// Create a new Runner with the given backend
    pub fn new(backend: B) -> Self {
        Self { backend: Arc::new(backend) }
    }

    /// Extract authentication, the workspace mode and the HTTP request itself
    /// from the request context
    fn extract_context(
        context: &RequestContext<RoleServer>,
    ) -> Result<McpContext<B::Auth>, ErrorData> {
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

        Ok(McpContext { auth: auth.clone(), mode, headers: http_parts.headers.clone() })
    }
}

/// How an endpoint tool interacts with the token's `mcp:scripts:` / `mcp:flows:`
/// path scopes. Mirrored in
/// `frontend/src/lib/components/mcp/endpointScopePolicy.ts` so the scope picker
/// only advertises tools this policy will actually list — keep the two in sync.
enum EndpointPathPolicy {
    /// Executes the script/flow named by the `path` argument. Gated by the
    /// script/flow scope alone (an endpoint scope is not enough to run things):
    /// in multi-workspace mode these are the only way to run scripts/flows, so a
    /// granular token must not run items outside its allowed paths.
    RunByPath(&'static str),
    /// Reads/writes the script/flow named by the listed path arguments. The
    /// endpoint scope grants the capability; when the token also carries path
    /// patterns for `kind`, every listed argument must match them.
    ///
    /// `optional_fields` is for a destination path the tool lets the caller omit, where
    /// absent means "leave it where it is": requiring it would refuse every call that
    /// does not move anything. Absent or empty, it is filled in from the argument it
    /// pairs with before the request is built, so the item stays where it is without
    /// the endpoint having to accept a body that omits its own path. Names are the ones
    /// the tool exposes, so a body path colliding with the URL's is `path__body` — the
    /// URL path keeps the plain name (see `generate_mcp_tools.py`).
    PathArgs {
        kind: &'static str,
        fields: &'static [&'static str],
        /// `(destination, source)` pairs: a destination path the tool lets the caller
        /// omit, and the argument it means when omitted.
        optional_fields: &'static [(&'static str, &'static str)],
    },
    /// Affects scripts without taking a checkable path (delete-by-hash) or
    /// executes arbitrary code (preview). Unavailable to path-confined tokens —
    /// allowing these would bypass the path patterns entirely.
    Unconfinable(&'static str),
}

/// Whether `endpoint_path_policy` confines this tool's paths. Public so the crate that
/// owns the generated endpoint catalogue can assert every path-addressed script/flow
/// tool is covered: one that is not falls through to no confinement at all.
pub fn has_endpoint_path_policy(endpoint_name: &str) -> bool {
    endpoint_path_policy(endpoint_name).is_some()
}

fn endpoint_path_policy(endpoint_name: &str) -> Option<EndpointPathPolicy> {
    use EndpointPathPolicy::*;
    match endpoint_name {
        "runScriptByPath" => Some(RunByPath("script")),
        "runFlowByPath" => Some(RunByPath("flow")),
        "getScriptByPath" | "deleteScriptByPath" | "createScript" => {
            Some(PathArgs { kind: "script", fields: &["path"], optional_fields: &[] })
        }
        "getFlowByPath" | "deleteFlowByPath" | "createFlow" => {
            Some(PathArgs { kind: "flow", fields: &["path"], optional_fields: &[] })
        }
        // These address the item via the URL path and can move it to the path given
        // in the body — both must stay within scope, and the body one only binds
        // when supplied, since omitting it is how a caller updates in place.
        "updateScript" => Some(PathArgs {
            kind: "script",
            fields: &["path"],
            optional_fields: &[("path__body", "path")],
        }),
        "updateFlow" => Some(PathArgs {
            kind: "flow",
            fields: &["path"],
            optional_fields: &[("path__body", "path")],
        }),
        "deleteScriptByHash" | "runScriptPreviewAndWaitResult" => Some(Unconfinable("script")),
        _ => None,
    }
}

/// Whether the token restricts `kind` ("script"/"flow") to specific paths. A
/// `*` pattern grants every path (see `is_resource_allowed`), so it does not
/// count as confinement.
fn path_confined(scope_config: &crate::common::scope::McpScopeConfig, kind: &str) -> bool {
    if scope_config.all {
        return false;
    }
    let patterns = match kind {
        "script" => &scope_config.scripts,
        "flow" => &scope_config.flows,
        _ => return false,
    };
    !patterns.is_empty() && !patterns.iter().any(|p| p == "*")
}

fn require_path_arg<'a>(
    endpoint_tool: &EndpointTool,
    args: &'a Value,
    field: &str,
) -> Result<&'a str, ErrorData> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ErrorData::invalid_params(
                format!(
                    "Missing required '{}' argument for tool '{}'.",
                    field, endpoint_tool.name
                ),
                None,
            )
        })
}

/// Whether an endpoint tool is in scope for *listing*. Run-by-path endpoints are
/// gated by the script/flow scope (`has_any` — the token can run at least one
/// path of that kind); unconfinable endpoints are hidden from path-confined
/// tokens (their calls would always be denied); every other endpoint by the
/// endpoint-name scope.
fn endpoint_tool_in_scope(
    scope_config: &crate::common::scope::McpScopeConfig,
    endpoint_tool: &EndpointTool,
) -> bool {
    let endpoint_allowed =
        !scope_config.granular || scope_config.is_allowed("endpoint", &endpoint_tool.name);
    match endpoint_path_policy(&endpoint_tool.name) {
        Some(EndpointPathPolicy::RunByPath(kind)) => scope_config.has_any(kind),
        Some(EndpointPathPolicy::Unconfinable(kind)) => {
            endpoint_allowed && !path_confined(scope_config, kind)
        }
        _ => endpoint_allowed,
    }
}

/// Authorize an endpoint-tool *call* against the token's MCP scopes and
/// read-only flag. Shared by single- and multi-workspace modes so both enforce
/// the same rules — otherwise a granular token could run items outside its
/// allowed paths through the single-workspace path. Run-by-path endpoints
/// (runScriptByPath/runFlowByPath) are gated by the script/flow scope for the
/// requested `path` (the endpoint-name scope alone is insufficient); other
/// endpoints by the endpoint-name scope, with script/flow path arguments
/// additionally confined to the token's path patterns when it has any; and
/// non-GET endpoints are refused for read-only tokens.
fn authorize_endpoint_call(
    scope_config: &crate::common::scope::McpScopeConfig,
    endpoint_tool: &EndpointTool,
    args: &mut Value,
    read_only: bool,
) -> Result<(), ErrorData> {
    // A destination the caller omitted, or left empty because it had to fill in every
    // argument, means "leave the item where it is". Resolved to the path the item is
    // already at, so the endpoint receives a body naming its own path rather than one
    // that omits it, and so `update_flow` cannot read the empty string as the empty
    // path and move the flow there. The confinement below then sees a destination
    // equal to the source, which it already has to allow.
    if let Some(EndpointPathPolicy::PathArgs { optional_fields, .. }) =
        endpoint_path_policy(&endpoint_tool.name)
    {
        for (destination, source) in optional_fields {
            let unset = args
                .get(destination)
                .is_none_or(|v| !v.as_str().is_some_and(|s| !s.is_empty()));
            let Some(current) = args
                .get(source)
                .and_then(|v| v.as_str())
                .map(str::to_string)
            else {
                continue;
            };
            if unset {
                if let Some(obj) = args.as_object_mut() {
                    obj.insert(destination.to_string(), Value::String(current));
                }
            }
        }
    }

    match endpoint_path_policy(&endpoint_tool.name) {
        Some(EndpointPathPolicy::RunByPath(kind)) => {
            let path = require_path_arg(endpoint_tool, args, "path")?;
            // No `granular` gate: is_allowed already encodes every mode — true for
            // mcp:all, pattern-matched for granular scopes, and false for
            // mcp:favorites (a favorites token can't run an arbitrary path).
            if !scope_config.is_allowed(kind, path) {
                return Err(ErrorData::internal_error(
                    format!("Access denied: {} '{}' not in token scope", kind, path),
                    None,
                ));
            }
        }
        policy => {
            if scope_config.granular && !scope_config.is_allowed("endpoint", &endpoint_tool.name) {
                return Err(ErrorData::internal_error(
                    format!(
                        "Access denied: endpoint '{}' not in token scope",
                        endpoint_tool.name
                    ),
                    None,
                ));
            }
            match policy {
                Some(EndpointPathPolicy::PathArgs { kind, fields, optional_fields })
                    if path_confined(scope_config, kind) =>
                {
                    // Filled in above when omitted, so each one names a real path.
                    let supplied = optional_fields
                        .iter()
                        .filter_map(|(field, _)| args.get(field).and_then(|v| v.as_str()).map(Ok));
                    for path in fields
                        .iter()
                        .map(|field| require_path_arg(endpoint_tool, args, field))
                        .chain(supplied)
                    {
                        let path = path?;
                        if !scope_config.is_allowed(kind, path) {
                            return Err(ErrorData::internal_error(
                                format!("Access denied: {} '{}' not in token scope", kind, path),
                                None,
                            ));
                        }
                    }
                }
                Some(EndpointPathPolicy::Unconfinable(kind))
                    if path_confined(scope_config, kind) =>
                {
                    return Err(ErrorData::internal_error(
                        format!(
                            "Access denied: endpoint '{}' is not available to a token restricted to specific {} paths",
                            endpoint_tool.name, kind
                        ),
                        None,
                    ));
                }
                _ => {}
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
    Ok(())
}

/// Map the model's argument keys back to the runnable's original parameter names.
fn transform_call_args(args: Value, item_schema: &Option<SchemaType>) -> Value {
    let Value::Object(map) = args else {
        return args;
    };
    let mut args_hash = HashMap::new();
    for (k, v) in map {
        args_hash.insert(reverse_transform_key(&k, item_schema), v);
    }
    Value::Object(args_hash.into_iter().collect())
}

fn find_matching_path<T: ToolableItem>(candidates: Vec<T>, request_name: &str) -> Option<String> {
    candidates
        .into_iter()
        .find(|item| item.get_transformed_path() == request_name)
        .map(|item| item.get_full_path().to_string())
}

impl<B: McpBackend> ServerHandler for Runner<B> {
    fn get_info(&self) -> ServerInfo {
        // Not `Implementation::from_build_env()`: its `env!` expands inside rmcp, so it
        // would name the SDK crate rather than this server.
        let server_info = Implementation::new("windmill", env!("CARGO_PKG_VERSION"))
            .with_title("Windmill")
            .with_website_url("https://windmill.dev");

        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(server_info)
            .with_instructions(
                "This server provides a list of scripts and flows the user can run on Windmill. \
                 Each flow and script is a tool callable with their respective arguments.",
            )
    }

    /// Pinned rather than left to rmcp's default (every version the SDK knows), so a
    /// future SDK revision cannot start advertising a version this server has not been
    /// exercised against. Bounds `initialize` negotiation, `server/discover`, and
    /// per-request version validation alike.
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(SUPPORTED_PROTOCOL_VERSIONS)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let McpContext { auth, mode, .. } = Self::extract_context(&context)?;

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
    ) -> Result<CallToolResponse, ErrorData> {
        let McpContext { auth, mode, headers } = Self::extract_context(&context)?;

        // Parse MCP scopes for authorization
        let scopes = auth.scopes().unwrap_or(&[]);
        let scope_config =
            parse_mcp_scopes(scopes).map_err(|e| ErrorData::internal_error(e, None))?;
        let read_only = auth.read_only();

        let args = request.arguments.map(Value::Object).unwrap_or(Value::Null);
        let mcp_request = McpRequest { headers: &headers, tool_name: request.name.as_ref() };

        // Every tool here runs to completion in one round trip: none of them ask the
        // client for input, so the MRTR variants of `CallToolResponse` are never built.
        let result = match mode {
            McpMode::Single(workspace_id) => {
                self.call_tool_single(
                    &auth,
                    &workspace_id,
                    &scope_config,
                    read_only,
                    request.name.clone(),
                    args,
                    &mcp_request,
                )
                .await
            }
            McpMode::Multi(token) => {
                self.call_tool_multi(
                    &auth,
                    &token,
                    &scope_config,
                    read_only,
                    request.name.clone(),
                    args,
                )
                .await
            }
        }?;
        Ok(result.into())
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult::with_all_items(vec![])
            .with_ttl_ms(LIST_TTL_MS)
            .with_cache_scope(LIST_CACHE_SCOPE))
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        Ok(ListPromptsResult::default()
            .with_ttl_ms(LIST_TTL_MS)
            .with_cache_scope(LIST_CACHE_SCOPE))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        Ok(ListResourceTemplatesResult::default()
            .with_ttl_ms(LIST_TTL_MS)
            .with_cache_scope(LIST_CACHE_SCOPE))
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

        // Add endpoint tools from the generated MCP tools, filtered by scope.
        // Uses the same run-by-path-aware gate as multi-workspace mode so a
        // granular token only sees runScriptByPath / runFlowByPath when it can
        // actually run scripts / flows.
        let endpoint_tools = self.backend.all_endpoint_tools();
        for endpoint_tool in endpoint_tools {
            if !endpoint_tool_in_scope(scope_config, &endpoint_tool) {
                continue;
            }
            if read_only && !crate::server::is_endpoint_read_only(&endpoint_tool) {
                continue;
            }

            tools.push(endpoint_tool_to_mcp_tool(&endpoint_tool));
        }

        Ok(ListToolsResult::with_all_items(tools)
            .with_ttl_ms(LIST_TTL_MS)
            .with_cache_scope(LIST_CACHE_SCOPE))
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
        request: &McpRequest<'_>,
    ) -> Result<CallToolResult, ErrorData> {
        // Check if this is an endpoint tool
        let endpoint_tools = self.backend.all_endpoint_tools();
        for endpoint_tool in &endpoint_tools {
            if endpoint_tool.name.as_ref() == name.as_ref() {
                // Authorize against the token's MCP scopes and read-only flag,
                // including the run-by-path path check (shared with multi mode).
                let mut args = args;
                authorize_endpoint_call(scope_config, endpoint_tool, &mut args, read_only)?;

                // This is an endpoint tool, call via backend. The backend's own error
                // code is kept: a client that retries an internal error would loop on
                // arguments the endpoint rejected.
                let result = self
                    .backend
                    .call_endpoint(auth, workspace_id, endpoint_tool, args)
                    .await?;

                return Ok(CallToolResult::success(vec![ContentBlock::text(
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

        let transformed_args = transform_call_args(args, &item_schema);

        let script_or_flow_path = if is_hub {
            format!("hub/{}", path)
        } else {
            path
        };

        // Execute script or flow
        let result = if tool_type == "script" {
            self.backend
                .run_script(
                    auth,
                    workspace_id,
                    &script_or_flow_path,
                    transformed_args,
                    request,
                )
                .await
        } else {
            self.backend
                .run_flow(
                    auth,
                    workspace_id,
                    &script_or_flow_path,
                    transformed_args,
                    request,
                )
                .await
        };

        match result {
            Ok(value) => Ok(CallToolResult::success(vec![ContentBlock::text(
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
            if !endpoint_tool_in_scope(scope_config, &endpoint_tool) {
                continue;
            }
            if read_only && !crate::server::is_endpoint_read_only(&endpoint_tool) {
                continue;
            }

            tools.push(endpoint_tool_to_mcp_tool_multi(&endpoint_tool));
        }

        ListToolsResult::with_all_items(tools)
            .with_ttl_ms(LIST_TTL_MS)
            .with_cache_scope(LIST_CACHE_SCOPE)
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
        mut args: Value,
    ) -> Result<CallToolResult, ErrorData> {
        if name.as_ref() == "list_workspaces" {
            let workspaces = self
                .backend
                .list_accessible_workspaces(base_auth)
                .await
                .map_err(|e| ErrorData::internal_error(e.message, None))?;
            return Ok(CallToolResult::success(vec![ContentBlock::text(
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

        // Authorize the tool against the token's MCP scopes and read-only flag
        // (shared with single-workspace mode). Run-by-path endpoints
        // (runScriptByPath / runFlowByPath) run an arbitrary `path` and are
        // checked against the script/flow scope for that path — the endpoint
        // scope alone would let a granular token run items outside its allowed
        // paths.
        authorize_endpoint_call(scope_config, endpoint_tool, &mut args, read_only)?;

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
            .await?;

        Ok(CallToolResult::success(vec![ContentBlock::text(
            truncate_tool_result(
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()),
            ),
        )]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::scope::{parse_mcp_scopes, McpScopeConfig};
    use serde_json::json;
    use std::borrow::Cow;

    fn cfg(scopes: &[&str]) -> McpScopeConfig {
        parse_mcp_scopes(&scopes.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
    }

    fn ep(name: &'static str, method: &'static str) -> EndpointTool {
        EndpointTool {
            name: Cow::Borrowed(name),
            description: Cow::Borrowed(""),
            instructions: Cow::Borrowed(""),
            path: Cow::Borrowed("/w/{workspace}/jobs/run/p/{path}"),
            method: Cow::Borrowed(method),
            path_params_schema: None,
            query_params_schema: None,
            body_schema: None,
            query_field_renames: None,
            body_field_renames: None,
        }
    }

    // The core invariant: a folder-scoped token (mcp:endpoints:* +
    // mcp:scripts:f/team/*, as the folder-scope UI emits) must not run a script
    // outside its allowed folders via runScriptByPath — mcp:endpoints:* alone
    // must never authorize an arbitrary path.
    #[test]
    fn run_by_path_call_enforces_script_scope() {
        let config = cfg(&[
            "mcp:scripts:f/team/*",
            "mcp:flows:f/team/*",
            "mcp:endpoints:*",
        ]);
        let tool = ep("runScriptByPath", "POST");

        assert!(authorize_endpoint_call(
            &config,
            &tool,
            &mut json!({"path": "f/team/deploy"}),
            false
        )
        .is_ok());
        assert!(authorize_endpoint_call(
            &config,
            &tool,
            &mut json!({"path": "f/secret/admin"}),
            false
        )
        .is_err());
    }

    #[test]
    fn run_by_path_call_requires_path_arg() {
        let tool = ep("runFlowByPath", "POST");
        assert!(authorize_endpoint_call(&cfg(&["mcp:all"]), &tool, &mut json!({}), false).is_err());
    }

    #[test]
    fn run_by_path_flow_scope_independent_of_script_scope() {
        // A flow-only token can run flows by path but not scripts by path.
        let config = cfg(&["mcp:flows:f/team/*", "mcp:endpoints:*"]);
        assert!(authorize_endpoint_call(
            &config,
            &ep("runFlowByPath", "POST"),
            &mut json!({"path": "f/team/x"}),
            false
        )
        .is_ok());
        assert!(authorize_endpoint_call(
            &config,
            &ep("runScriptByPath", "POST"),
            &mut json!({"path": "f/team/x"}),
            false
        )
        .is_err());
    }

    #[test]
    fn non_run_by_path_gated_by_endpoint_scope() {
        let get_var = ep("getVariable", "GET");
        assert!(authorize_endpoint_call(
            &cfg(&["mcp:endpoints:getVariable"]),
            &get_var,
            &mut json!({"path": "u/a/b"}),
            false
        )
        .is_ok());
        // A granular token without the endpoint scope is denied.
        assert!(authorize_endpoint_call(
            &cfg(&["mcp:scripts:f/team/*"]),
            &get_var,
            &mut json!({"path": "u/a/b"}),
            false
        )
        .is_err());
        // mcp:all (non-granular) allows any endpoint.
        assert!(
            authorize_endpoint_call(&cfg(&["mcp:all"]), &get_var, &mut json!({}), false).is_ok()
        );
    }

    #[test]
    fn read_only_refuses_non_get_endpoint() {
        // Reaches the read-only check via mcp:all so scope isn't the blocker.
        assert!(authorize_endpoint_call(
            &cfg(&["mcp:all"]),
            &ep("createResource", "POST"),
            &mut json!({}),
            true
        )
        .is_err());
        assert!(authorize_endpoint_call(
            &cfg(&["mcp:all"]),
            &ep("getVariable", "GET"),
            &mut json!({"path": "u/a/b"}),
            true
        )
        .is_ok());
    }

    #[test]
    fn listing_run_by_path_needs_runnable_scope() {
        let tool = ep("runScriptByPath", "POST");
        // An endpoint-only token cannot run any script, so the tool isn't listed.
        assert!(!endpoint_tool_in_scope(&cfg(&["mcp:endpoints:*"]), &tool));
        // A script-scoped token can, so it is listed.
        assert!(endpoint_tool_in_scope(
            &cfg(&["mcp:scripts:f/team/*"]),
            &tool
        ));
        assert!(endpoint_tool_in_scope(&cfg(&["mcp:all"]), &tool));
        // Non-run-by-path endpoints are governed by the endpoint scope.
        let get_var = ep("getVariable", "GET");
        assert!(endpoint_tool_in_scope(
            &cfg(&["mcp:endpoints:getVariable"]),
            &get_var
        ));
        assert!(!endpoint_tool_in_scope(
            &cfg(&["mcp:scripts:f/team/*"]),
            &get_var
        ));
    }

    // A folder-scoped token must not read/write/delete scripts or flows outside
    // its allowed paths through the non-run endpoint tools either.
    #[test]
    fn path_arg_tools_confined_by_path_patterns() {
        let config = cfg(&[
            "mcp:scripts:f/team/*",
            "mcp:flows:f/team/*",
            "mcp:endpoints:*",
        ]);
        for name in ["getScriptByPath", "deleteScriptByPath", "createScript"] {
            let tool = ep(name, "POST");
            assert!(
                authorize_endpoint_call(&config, &tool, &mut json!({"path": "f/team/x"}), false)
                    .is_ok(),
                "{name} should allow in-scope path"
            );
            assert!(
                authorize_endpoint_call(&config, &tool, &mut json!({"path": "f/secret/x"}), false)
                    .is_err(),
                "{name} should deny out-of-scope path"
            );
            // Confinement can't be verified without the path argument.
            assert!(
                authorize_endpoint_call(&config, &tool, &mut json!({}), false).is_err(),
                "{name} should require the path argument when confined"
            );
        }
        for name in ["getFlowByPath", "deleteFlowByPath", "createFlow"] {
            let tool = ep(name, "POST");
            assert!(
                authorize_endpoint_call(&config, &tool, &mut json!({"path": "f/team/x"}), false)
                    .is_ok(),
                "{name} should allow in-scope path"
            );
            assert!(
                authorize_endpoint_call(&config, &tool, &mut json!({"path": "f/secret/x"}), false)
                    .is_err(),
                "{name} should deny out-of-scope path"
            );
        }
    }

    // A token that never expressed path patterns (endpoints-only) is not
    // confined: the endpoint scope alone authorizes any path.
    #[test]
    fn path_arg_tools_unconfined_without_path_patterns() {
        let config = cfg(&["mcp:endpoints:*"]);
        for name in ["getScriptByPath", "createScript", "deleteFlowByPath"] {
            assert!(authorize_endpoint_call(
                &config,
                &ep(name, "POST"),
                &mut json!({"path": "f/anywhere/x"}),
                false
            )
            .is_ok());
        }
        // A `*` pattern grants every path, so it doesn't confine either.
        let star = cfg(&["mcp:scripts:*", "mcp:endpoints:*"]);
        assert!(authorize_endpoint_call(
            &star,
            &ep("createScript", "POST"),
            &mut json!({"path": "f/anywhere/x"}),
            false
        )
        .is_ok());
    }

    // updateFlow both addresses a flow (URL path) and can move it (body path):
    // a confined token must have both within scope.
    #[test]
    fn update_flow_checks_target_and_destination_paths() {
        let config = cfg(&["mcp:flows:f/team/*", "mcp:endpoints:*"]);
        let tool = ep("updateFlow", "POST");
        assert!(authorize_endpoint_call(
            &config,
            &tool,
            &mut json!({"path": "f/team/a", "path__body": "f/team/b"}),
            false
        )
        .is_ok());
        // Moving a flow out of the allowed folder is denied.
        assert!(authorize_endpoint_call(
            &config,
            &tool,
            &mut json!({"path": "f/team/a", "path__body": "f/secret/a"}),
            false
        )
        .is_err());
        // Touching a flow outside the allowed folder is denied.
        assert!(authorize_endpoint_call(
            &config,
            &tool,
            &mut json!({"path": "f/secret/a", "path__body": "f/team/a"}),
            false
        )
        .is_err());
    }

    // updateScript has updateFlow's shape, and the destination it can move a script
    // to is optional: omitting it updates in place, so requiring it would refuse
    // every call that moves nothing.
    #[test]
    fn update_script_confines_target_and_optional_destination() {
        let config = cfg(&["mcp:scripts:f/team/*", "mcp:endpoints:*"]);
        let tool = ep("updateScript", "POST");
        for mut args in [
            json!({"path": "f/team/a"}),
            json!({"path": "f/team/a", "path__body": "f/team/b"}),
            // Empty destination reads as absent, the same way the handler reads it.
            json!({"path": "f/team/a", "path__body": ""}),
        ] {
            assert!(
                authorize_endpoint_call(&config, &tool, &mut args, false).is_ok(),
                "in-scope update should be allowed: {args}"
            );
        }
        for mut args in [
            // Deploying over a script outside the allowed folder.
            json!({"path": "f/secret/a"}),
            // Moving one out of it.
            json!({"path": "f/team/a", "path__body": "f/secret/a"}),
        ] {
            assert!(
                authorize_endpoint_call(&config, &tool, &mut args, false).is_err(),
                "out-of-scope update should be denied: {args}"
            );
        }
    }

    /// A destination the caller omitted, or left empty because it had to fill in every
    /// argument, is filled in with the path the item is already at. The endpoint then
    /// never receives a body that omits its own path, and `update_flow` never receives
    /// the empty string, which it would read as the empty path and move the flow there.
    #[test]
    fn an_unset_optional_destination_becomes_the_current_path() {
        for tool in ["updateScript", "updateFlow"] {
            let kind_scope = if tool == "updateScript" {
                "mcp:scripts:f/team/*"
            } else {
                "mcp:flows:f/team/*"
            };
            for mut args in [
                json!({"path": "f/team/a", "summary": "s"}),
                json!({"path": "f/team/a", "path__body": "", "summary": "s"}),
                json!({"path": "f/team/a", "path__body": null, "summary": "s"}),
            ] {
                authorize_endpoint_call(
                    &cfg(&[kind_scope, "mcp:endpoints:*"]),
                    &ep(tool, "POST"),
                    &mut args,
                    false,
                )
                .unwrap_or_else(|e| panic!("{tool} should allow an unset destination: {e:?}"));
                assert_eq!(
                    args,
                    json!({"path": "f/team/a", "path__body": "f/team/a", "summary": "s"}),
                    "{tool} should fill the destination in with the current path"
                );
            }
        }

        // A destination that names somewhere else is left for the confinement to check.
        let mut args = json!({"path": "f/team/a", "path__body": "f/team/b"});
        authorize_endpoint_call(
            &cfg(&["mcp:scripts:f/team/*", "mcp:endpoints:*"]),
            &ep("updateScript", "POST"),
            &mut args,
            false,
        )
        .unwrap();
        assert_eq!(args["path__body"], "f/team/b");
    }

    // Tools that can't be path-checked (delete-by-hash) or execute arbitrary
    // code (preview) would bypass path confinement, so a path-confined token is
    // denied them entirely — and doesn't see them listed.
    #[test]
    fn unconfinable_tools_denied_for_path_confined_token() {
        let confined = cfg(&["mcp:scripts:f/team/*", "mcp:endpoints:*"]);
        for name in ["deleteScriptByHash", "runScriptPreviewAndWaitResult"] {
            let tool = ep(name, "POST");
            assert!(authorize_endpoint_call(&confined, &tool, &mut json!({}), false).is_err());
            assert!(!endpoint_tool_in_scope(&confined, &tool));
            // Without script path patterns the tools stay available.
            assert!(authorize_endpoint_call(
                &cfg(&["mcp:endpoints:*"]),
                &tool,
                &mut json!({}),
                false
            )
            .is_ok());
            assert!(
                authorize_endpoint_call(&cfg(&["mcp:all"]), &tool, &mut json!({}), false).is_ok()
            );
            assert!(endpoint_tool_in_scope(&cfg(&["mcp:endpoints:*"]), &tool));
        }
        // Flow-only confinement doesn't affect script-kind unconfinable tools.
        let flow_confined = cfg(&["mcp:flows:f/team/*", "mcp:endpoints:*"]);
        assert!(authorize_endpoint_call(
            &flow_confined,
            &ep("deleteScriptByHash", "POST"),
            &mut json!({}),
            false
        )
        .is_ok());
    }
}
