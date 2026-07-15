//! Windmill MCP Backend implementation
//!
//! This module provides the concrete implementation of the McpBackend trait
//! for the Windmill platform.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use windmill_common::{db::UserDB, utils::StripPath, DB};
use windmill_mcp::common::schema::enrich_resource_schemas;
use windmill_mcp::common::transform::apply_key_transformation;
use windmill_mcp::common::types::{
    FlowInfo, HubScriptInfo, ResourceInfo, ResourceType, SchemaType, ScriptInfo, WorkspaceInfo,
};
use windmill_mcp::server::{BackendResult, EndpointTool, ErrorData, McpBackend, PathFilter};

use crate::auth::AuthCache;
use crate::db::ApiAuthed;
use crate::jobs::{
    run_wait_result_flow_by_path_internal, run_wait_result_script_by_path_internal, RunJobQuery,
};

use super::auto_generated_endpoints::all_tools;
use super::utils::{
    build_query_string, build_request_body, create_http_request, get_hub_script_schema,
    get_item_schema, get_items, get_resources, get_resources_types, get_scripts_from_hub,
    parse_response_body, prepare_push_args, substitute_path_params,
};

use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use windmill_mcp::server::{
    LocalSessionManager, McpToken, MultiWorkspaceMcp, Runner, StreamableHttpServerConfig,
    StreamableHttpService,
};
use windmill_mcp::WorkspaceId;

use axum::{
    extract::{Extension, Path},
    http::Request,
    middleware::Next,
    response::Response,
    routing::get,
    Json, Router,
};
use windmill_common::{auth::hash_token, db::GatewayWorkspaceId, error::JsonResult};

// McpAuth impl for ApiAuthed is in windmill-api-auth (same crate as the type)

/// Windmill's MCP backend implementation
#[derive(Clone)]
pub struct WindmillBackend {
    pub db: DB,
    pub user_db: UserDB,
    pub base_internal_url: String,
    pub auth_cache: Arc<AuthCache>,
}

impl WindmillBackend {
    pub fn new(
        db: DB,
        user_db: UserDB,
        base_internal_url: String,
        auth_cache: Arc<AuthCache>,
    ) -> Self {
        Self { db, user_db, base_internal_url, auth_cache }
    }
}

#[async_trait]
impl McpBackend for WindmillBackend {
    type Auth = ApiAuthed;

    async fn list_scripts(
        &self,
        auth: &ApiAuthed,
        workspace_id: &str,
        favorites_only: bool,
        path_filter: Option<PathFilter<'_>>,
    ) -> BackendResult<Vec<ScriptInfo>> {
        let scope_type = if favorites_only { "favorites" } else { "all" };
        get_items::<ScriptInfo>(
            &self.user_db,
            auth,
            workspace_id,
            scope_type,
            "script",
            path_filter,
        )
        .await
        .map_err(|e| ErrorData::internal_error(e.message, None))
    }

    async fn list_flows(
        &self,
        auth: &ApiAuthed,
        workspace_id: &str,
        favorites_only: bool,
        path_filter: Option<PathFilter<'_>>,
    ) -> BackendResult<Vec<FlowInfo>> {
        let scope_type = if favorites_only { "favorites" } else { "all" };
        get_items::<FlowInfo>(
            &self.user_db,
            auth,
            workspace_id,
            scope_type,
            "flow",
            path_filter,
        )
        .await
        .map_err(|e| ErrorData::internal_error(e.message, None))
    }

    async fn list_resource_types(
        &self,
        auth: &ApiAuthed,
        workspace_id: &str,
    ) -> BackendResult<Vec<ResourceType>> {
        get_resources_types(&self.user_db, auth, workspace_id)
            .await
            .map_err(|e| ErrorData::internal_error(e.message, None))
    }

    async fn list_resources(
        &self,
        auth: &ApiAuthed,
        workspace_id: &str,
        resource_type: &str,
    ) -> BackendResult<Vec<ResourceInfo>> {
        get_resources(&self.user_db, auth, workspace_id, resource_type)
            .await
            .map_err(|e| ErrorData::internal_error(e.message, None))
    }

    async fn list_hub_scripts(
        &self,
        app_filter: Option<&str>,
    ) -> BackendResult<Vec<HubScriptInfo>> {
        get_scripts_from_hub(&self.db, app_filter)
            .await
            .map_err(|e| ErrorData::internal_error(e.message, None))
    }

    async fn get_item_schema(
        &self,
        auth: &ApiAuthed,
        workspace_id: &str,
        path: &str,
        item_type: &str,
    ) -> BackendResult<Option<SchemaType>> {
        let schema = get_item_schema(path, &self.user_db, auth, workspace_id, item_type)
            .await
            .map_err(|e| ErrorData::internal_error(e.message, None))?;

        if let Some(ref s) = schema {
            match serde_json::from_str::<SchemaType>(s.0.get()) {
                Ok(val) => Ok(Some(val)),
                Err(e) => {
                    tracing::warn!("Failed to parse schema: {}", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    async fn get_hub_script_schema(&self, path: &str) -> BackendResult<Option<SchemaType>> {
        let schema = get_hub_script_schema(path, &self.db)
            .await
            .map_err(|e| ErrorData::internal_error(e.message, None))?;

        if let Some(ref s) = schema {
            match serde_json::from_str::<SchemaType>(s.0.get()) {
                Ok(val) => Ok(Some(val)),
                Err(e) => {
                    tracing::warn!("Failed to parse hub schema: {}", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    fn transform_schema_for_resources(
        &self,
        schema: &SchemaType,
        resources_cache: &HashMap<String, Vec<ResourceInfo>>,
        resources_types: &[ResourceType],
    ) -> SchemaType {
        let mut schema_obj = schema.clone();

        // Replace invalid char in property key with underscore
        let replacements: Vec<(String, String, Value)> = schema_obj
            .properties
            .iter()
            .filter_map(|(key, value)| {
                if key.chars().any(|c| !c.is_alphanumeric() && c != '_') {
                    let new_key = apply_key_transformation(key);
                    Some((key.clone(), new_key, value.clone()))
                } else {
                    None
                }
            })
            .collect();

        for (old_key, new_key, value) in replacements {
            schema_obj.properties.remove(&old_key);
            schema_obj.properties.insert(new_key, value);
        }

        // Enrich every resource reference in the schema — including those
        // inside `items`, nested `properties`, etc. — with a description
        // listing the available resources. Both shapes are handled:
        //   { type: "object", format: "resource-<name>" }   (top-level scalar)
        //   { type: "resource", resourceType: "<name>" }    (inside list items)
        for prop_value in schema_obj.properties.values_mut() {
            enrich_resource_schemas(prop_value, resources_cache, resources_types);
        }

        schema_obj
    }

    async fn run_script(
        &self,
        auth: &ApiAuthed,
        workspace_id: &str,
        path: &str,
        args: Value,
    ) -> BackendResult<Value> {
        let push_args = prepare_push_args(args);

        let result = run_wait_result_script_by_path_internal(
            self.db.clone(),
            RunJobQuery::default(),
            StripPath(path.to_string()),
            auth.clone(),
            self.user_db.clone(),
            workspace_id.to_string(),
            push_args,
        )
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        parse_response_body(result).await
    }

    async fn run_flow(
        &self,
        auth: &ApiAuthed,
        workspace_id: &str,
        path: &str,
        args: Value,
    ) -> BackendResult<Value> {
        let push_args = prepare_push_args(args);

        let result = run_wait_result_flow_by_path_internal(
            self.db.clone(),
            RunJobQuery::default(),
            StripPath(path.to_string()),
            auth.clone(),
            self.user_db.clone(),
            push_args,
            workspace_id.to_string(),
        )
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        parse_response_body(result).await
    }

    async fn call_endpoint(
        &self,
        auth: &ApiAuthed,
        workspace_id: &str,
        endpoint_tool: &EndpointTool,
        args: Value,
    ) -> BackendResult<Value> {
        let args_map = match &args {
            Value::Object(map) => map,
            _ => {
                return Err(ErrorData::invalid_params(
                    "Arguments must be an object",
                    None,
                ));
            }
        };

        // Build URL with path substitutions
        let path_template = substitute_path_params(
            &endpoint_tool.path,
            workspace_id,
            args_map,
            &endpoint_tool.path_params_schema,
            &endpoint_tool.path_field_renames,
        )?;
        let query_string = build_query_string(
            args_map,
            &endpoint_tool.query_params_schema,
            &endpoint_tool.query_field_renames,
        );
        let full_url = format!(
            "{}/api{}{}",
            self.base_internal_url, path_template, query_string
        );

        // Prepare request body
        let body_json = build_request_body(
            &endpoint_tool.method,
            args_map,
            &endpoint_tool.body_schema,
            &endpoint_tool.body_field_renames,
            &endpoint_tool.path_params_schema,
            &endpoint_tool.query_params_schema,
        );

        // Create and execute request
        let response = create_http_request(
            &endpoint_tool.method,
            &full_url,
            workspace_id,
            auth,
            body_json,
        )
        .await?;

        let status = response.status();
        let response_text = response.text().await.map_err(|e| {
            ErrorData::internal_error(format!("Failed to read response text: {}", e), None)
        })?;

        if status.is_success() {
            Ok(serde_json::from_str(&response_text)
                .unwrap_or_else(|_| Value::String(response_text)))
        } else {
            Err(ErrorData::internal_error(
                format!(
                    "HTTP {} {}: {}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or(""),
                    response_text
                ),
                None,
            ))
        }
    }

    async fn list_accessible_workspaces(
        &self,
        auth: &ApiAuthed,
    ) -> BackendResult<Vec<WorkspaceInfo>> {
        // A superadmin can act in every workspace and often has no explicit `usr`
        // membership row (matching resolve_workspace_auth, which authorizes any
        // workspace for a superadmin), so list them all. Everyone else is limited
        // to the workspaces they are a member of.
        let workspaces = if auth.is_admin {
            sqlx::query_as!(
                WorkspaceInfo,
                "SELECT id, name FROM workspace WHERE deleted = false ORDER BY name",
            )
            .fetch_all(&self.db)
            .await
        } else {
            sqlx::query_as!(
                WorkspaceInfo,
                "SELECT workspace.id, workspace.name
                 FROM workspace
                 JOIN usr ON usr.workspace_id = workspace.id
                 WHERE usr.email = $1 AND usr.disabled = false AND workspace.deleted = false
                 ORDER BY workspace.name",
                auth.email,
            )
            .fetch_all(&self.db)
            .await
        };

        workspaces.map_err(|e| ErrorData::internal_error(e.to_string(), None))
    }

    async fn resolve_workspace_auth(
        &self,
        token: &str,
        workspace_id: &str,
    ) -> BackendResult<ApiAuthed> {
        self.auth_cache
            .get_authed(Some(workspace_id.to_string()), token)
            .await
            .ok_or_else(|| {
                ErrorData::invalid_params(
                    format!(
                        "Access denied: token owner is not a member of workspace '{}'",
                        workspace_id
                    ),
                    None,
                )
            })
    }

    fn all_endpoint_tools(&self) -> Vec<EndpointTool> {
        all_tools()
    }
}

/// Extract workspace ID from path and store it in request extensions
pub async fn extract_and_store_workspace_id(
    Path(params): Path<String>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let workspace_id = params;
    request.extensions_mut().insert(WorkspaceId(workspace_id));
    next.run(request).await
}

/// Middleware that adds WWW-Authenticate header to 401 responses
/// This helps MCP clients discover the OAuth authorization server (RFC 9728)
pub async fn add_www_authenticate_header(
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    use axum::http::StatusCode;
    use windmill_common::BASE_URL;

    // Extract workspace_id before consuming the request
    let Some(workspace_id) = request
        .extensions()
        .get::<WorkspaceId>()
        .map(|w| w.0.clone())
    else {
        return Response::builder()
            .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body(axum::body::Body::from("Missing workspace_id in request"))
            .unwrap();
    };

    let response = next.run(request).await;

    // Only add header to 401 Unauthorized responses
    if response.status() == StatusCode::UNAUTHORIZED {
        let base_url = BASE_URL.load();

        // RFC 9728: The resource parameter contains the protected resource URL.
        // Clients derive the metadata URL by inserting /.well-known/oauth-protected-resource
        // after the host, e.g., http://host/.well-known/oauth-protected-resource/api/mcp/w/test/mcp
        let resource_url = format!("{}/api/mcp/w/{}/mcp", base_url, workspace_id);
        let www_authenticate = format!("Bearer resource=\"{}\"", resource_url);

        // Reconstruct response with the new header
        let (mut parts, body) = response.into_parts();
        parts.headers.insert(
            axum::http::header::WWW_AUTHENTICATE,
            www_authenticate
                .parse()
                .unwrap_or_else(|_| "Bearer".parse().unwrap()),
        );
        Response::from_parts(parts, body)
    } else {
        response
    }
}

/// Extract the bearer token from either the `Authorization` header or the
/// `?token=` query parameter (MCP clients commonly pass it in the URL).
fn extract_gateway_token(request: &Request<axum::body::Body>) -> Option<String> {
    if let Some(token) = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
    {
        return Some(token.to_string());
    }
    request.uri().query().and_then(|q| {
        url::form_urlencoded::parse(q.as_bytes())
            .find(|(k, _)| k == "token")
            .map(|(_, v)| v.into_owned())
    })
}

/// Middleware for gateway: resolve the MCP session mode from the Bearer token in
/// the DB. A token bound to a workspace injects `WorkspaceId` (single-workspace
/// mode). A workspace-less MCP token (`workspace_id IS NULL` with an `mcp:` scope)
/// injects `MultiWorkspaceMcp` + `McpToken`, putting the runner in
/// multi-workspace mode where tools take an explicit `workspace_id` argument.
pub async fn extract_workspace_from_token(
    Extension(db): Extension<DB>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    if let Some(token) = extract_gateway_token(&request) {
        let t_hash = hash_token(&token);
        match sqlx::query!(
            "SELECT workspace_id, scopes FROM token WHERE token_hash = $1 AND (expiration > NOW() OR expiration IS NULL)",
            t_hash
        )
        .fetch_optional(&db)
        .await
        {
            Ok(Some(row)) => match row.workspace_id {
                Some(workspace_id) => {
                    request
                        .extensions_mut()
                        .insert(GatewayWorkspaceId(workspace_id.clone()));
                    request.extensions_mut().insert(WorkspaceId(workspace_id));
                }
                None => {
                    // Only enter multi-workspace mode for genuine MCP tokens; a
                    // full-privilege global token without mcp scope is rejected
                    // by the runner's mcp-scope check anyway.
                    let is_mcp = row
                        .scopes
                        .as_deref()
                        .is_some_and(|s| s.iter().any(|scope| scope.starts_with("mcp:")));
                    if is_mcp {
                        request.extensions_mut().insert(MultiWorkspaceMcp);
                        request.extensions_mut().insert(McpToken(token));
                    }
                }
            },
            Ok(None) => {}
            Err(e) => {
                tracing::error!("Gateway token workspace lookup failed: {}", e);
            }
        }
    }

    next.run(request).await
}

/// Middleware that adds WWW-Authenticate header for gateway 401 responses
pub async fn add_www_authenticate_header_gateway(
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    use axum::http::StatusCode;
    use windmill_common::BASE_URL;

    let response = next.run(request).await;

    if response.status() == StatusCode::UNAUTHORIZED {
        let base_url = BASE_URL.load();
        let resource_url = format!("{}/api/mcp/gateway", base_url);
        let www_authenticate = format!("Bearer resource=\"{}\"", resource_url);

        let (mut parts, body) = response.into_parts();
        parts.headers.insert(
            axum::http::header::WWW_AUTHENTICATE,
            www_authenticate
                .parse()
                .unwrap_or_else(|_| "Bearer".parse().unwrap()),
        );
        Response::from_parts(parts, body)
    } else {
        response
    }
}

/// Setup the MCP server with HTTP transport
pub async fn setup_mcp_server(
    db: DB,
    user_db: UserDB,
    base_internal_url: String,
    auth_cache: Arc<AuthCache>,
) -> anyhow::Result<(Router, CancellationToken)> {
    let cancellation_token = CancellationToken::new();
    let session_manager = Arc::new(LocalSessionManager::default());

    let backend = WindmillBackend::new(db, user_db, base_internal_url, auth_cache);
    let runner = Runner::new(backend);

    let service_config = StreamableHttpServerConfig {
        sse_keep_alive: Some(Duration::from_secs(15)),
        stateful_mode: false,
        cancellation_token: cancellation_token.clone(),
        sse_retry: Some(Duration::from_secs(15)),
    };

    let service =
        StreamableHttpService::new(move || Ok(runner.clone()), session_manager, service_config);

    let router = Router::new().route_service("/", service);
    Ok((router, cancellation_token))
}

/// HTTP handler to list MCP tools as JSON
async fn list_mcp_tools_handler() -> JsonResult<Vec<EndpointTool>> {
    let endpoint_tools = all_tools();
    Ok(Json(endpoint_tools))
}

/// Creates a router service for listing MCP tools
pub fn list_tools_service() -> Router {
    Router::new().route("/", get(list_mcp_tools_handler))
}
