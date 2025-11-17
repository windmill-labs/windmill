//! MCP Server implementation
//!
//! Contains the core MCP server handler that implements the Model Context Protocol
//! specification. This is a thin orchestration layer that delegates to the appropriate
//! modules for tool management, database operations, and schema transformation.

use std::collections::HashMap;
use std::sync::Arc;
use std::{borrow::Cow, time::Duration};

use axum::body::to_bytes;
use rmcp::{
    handler::server::ServerHandler,
    model::*,
    service::{RequestContext, RoleServer},
    transport::StreamableHttpServerConfig,
    ErrorData,
};
use serde_json::Value;
use tokio::try_join;
use windmill_common::db::UserDB;
use windmill_common::worker::to_raw_value;
use windmill_common::{utils::StripPath, DB};

use crate::db::ApiAuthed;
use crate::jobs::{
    run_wait_result_flow_by_path_internal, run_wait_result_script_by_path_internal, RunJobQuery,
};

use super::tools::endpoint_tools::{
    all_endpoint_tools, call_endpoint_tool, endpoint_tools_to_mcp_tools, EndpointTool,
};
use super::utils::{
    database::{
        check_scopes, get_hub_script_schema, get_item_schema, get_items, get_resources_types,
        get_scripts_from_hub,
    },
    models::{
        FlowInfo, ResourceInfo, ResourceType, SchemaType, ScriptInfo, ToolableItem, WorkspaceId,
    },
    schema::transform_schema_for_resources,
    scope_matcher::{is_resource_allowed, parse_mcp_scopes},
    transform::{reverse_transform, reverse_transform_key},
};

use axum::{
    extract::Path, http::Request, middleware::Next, response::Response, routing::get, Json, Router,
};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, SessionManager, StreamableHttpService,
};
use windmill_common::error::JsonResult;

/// MCP Server Runner - implements the core MCP protocol handlers
#[derive(Clone)]
pub struct Runner {}

impl Runner {
    pub fn new() -> Self {
        Self {}
    }

    /// Creates a Tool from a ToolableItem
    async fn create_tool_from_item<T: ToolableItem>(
        item: &T,
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: &str,
        resources_cache: &mut HashMap<String, Vec<ResourceInfo>>,
        resources_types: &Vec<ResourceType>,
    ) -> Result<Tool, ErrorData> {
        let is_hub = item.is_hub();
        let path = item.get_path_or_id();
        let item_type = item.item_type();
        let description = format!(
            "This is a {} named `{}` with the following description: `{}`.{}",
            item_type,
            item.get_summary(),
            item.get_description(),
            if is_hub {
                format!(
                    " It is a tool used for the following app: {}",
                    item.get_integration_type()
                        .unwrap_or("No integration type".to_string())
                )
            } else {
                "".to_string()
            }
        );
        let schema_obj = transform_schema_for_resources(
            &item.get_schema(),
            user_db,
            authed,
            &workspace_id,
            resources_cache,
            &resources_types,
        )
        .await?;
        let input_schema_map = match serde_json::to_value(schema_obj) {
            Ok(Value::Object(map)) => map,
            Ok(_) => {
                tracing::warn!("Schema object for tool '{}' did not serialize to a JSON object, using empty schema.", path);
                serde_json::Map::new()
            }
            Err(e) => {
                tracing::error!(
                    "Failed to serialize schema object for tool '{}': {}. Using empty schema.",
                    path,
                    e
                );
                serde_json::Map::new()
            }
        };

        Ok(Tool {
            name: Cow::Owned(path),
            description: Some(Cow::Owned(description)),
            input_schema: Arc::new(input_schema_map),
            title: Some(item.get_summary().to_string()),
            output_schema: None,
            icons: None,
            annotations: Some(ToolAnnotations {
                title: Some(item.get_summary().to_string()),
                read_only_hint: Some(false),  // Can modify environment
                destructive_hint: Some(true), // Can potentially be destructive
                idempotent_hint: Some(false), // Are not guaranteed to be idempotent
                open_world_hint: Some(true),  // Can interact with external services
            }),
        })
    }
}

impl ServerHandler for Runner {
    /// Handles the `CallTool` request from the MCP client
    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let http_parts = context
            .extensions
            .get::<axum::http::request::Parts>()
            .ok_or_else(|| {
                tracing::error!("http::request::Parts not found");
                ErrorData::internal_error("http::request::Parts not found", None)
            })?;

        let authed = http_parts.extensions.get::<ApiAuthed>().ok_or_else(|| {
            tracing::error!("ApiAuthed Axum extension not found");
            ErrorData::internal_error("ApiAuthed Axum extension not found", None)
        })?;

        check_scopes(authed)?;

        // Parse MCP scopes for authorization
        let scopes = authed.scopes.as_ref().map(|s| s.as_slice()).unwrap_or(&[]);
        let scope_config = parse_mcp_scopes(scopes)?;

        if request.name.ends_with("_TRUNC") {
            return Ok(CallToolResult::error(
                vec![
                    Annotated::new(
                        RawContent::Text(RawTextContent {
                            text:
                                "Tool path is too long. Consider shortening it to make it compatible with MCP."
                                    .to_string(),
                            meta: None,
                        }),
                        None
                    ),
                ]
            ));
        }

        let db = http_parts.extensions.get::<DB>().ok_or_else(|| {
            tracing::error!("DB Axum extension not found");
            ErrorData::internal_error("DB Axum extension not found", None)
        })?;

        let user_db = http_parts.extensions.get::<UserDB>().ok_or_else(|| {
            tracing::error!("UserDB Axum extension not found");
            ErrorData::internal_error("UserDB Axum extension not found", None)
        })?;

        let args = request.arguments.map(Value::Object).ok_or_else(|| {
            ErrorData::invalid_params(
                "Missing arguments for tool",
                Some(request.name.clone().into()),
            )
        })?;

        let workspace_id = http_parts
            .extensions
            .get::<WorkspaceId>()
            .ok_or_else(|| {
                tracing::error!("WorkspaceId not found");
                ErrorData::internal_error("WorkspaceId not found", None)
            })
            .map(|w_id| w_id.0.clone())?;

        // Check if this is a generated endpoint tool
        let endpoint_tools = all_endpoint_tools();
        for endpoint_tool in endpoint_tools {
            if endpoint_tool.name.as_ref() == request.name {
                // Validate endpoint scope
                if scope_config.granular
                    && !is_resource_allowed(&endpoint_tool.name, &scope_config.endpoints)
                {
                    return Err(ErrorData::internal_error(
                        format!(
                            "Access denied: endpoint '{}' not in token scope",
                            endpoint_tool.name
                        ),
                        None,
                    ));
                }

                // This is an endpoint tool, forward to the actual HTTP endpoint
                let result =
                    call_endpoint_tool(&endpoint_tool, args.clone(), &workspace_id, &authed)
                        .await?;
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()),
                )]));
            }
        }

        // Continue with script/flow logic
        let (tool_type, path, is_hub) = reverse_transform(&request.name).map_err(|e| {
            ErrorData::internal_error(format!("Failed to reverse transform path: {}", e), None)
        })?;

        // Validate script/flow scope
        if !is_hub && scope_config.granular {
            if tool_type == "script" && !is_resource_allowed(&path, &scope_config.scripts) {
                return Err(ErrorData::internal_error(
                    format!("Access denied: script '{}' not in token scope", path),
                    None,
                ));
            } else if tool_type == "flow" && !is_resource_allowed(&path, &scope_config.flows) {
                return Err(ErrorData::internal_error(
                    format!("Access denied: flow '{}' not in token scope", path),
                    None,
                ));
            }
        }

        let item_schema = if is_hub {
            get_hub_script_schema(&format!("hub/{}", path), db).await?
        } else {
            get_item_schema(&path, user_db, authed, &workspace_id, &tool_type).await?
        };

        let schema_obj = if let Some(ref s) = item_schema {
            match serde_json::from_str::<SchemaType>(s.0.get()) {
                Ok(val) => Some(val),
                Err(e) => {
                    tracing::warn!("Failed to parse schema: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let push_args = if let Value::Object(map) = args.clone() {
            let mut args_hash = HashMap::new();
            for (k, v) in map {
                // need to transform back the key without invalid characters to the original key
                let original_key = reverse_transform_key(&k, &schema_obj);
                args_hash.insert(original_key, to_raw_value(&v));
            }
            windmill_queue::PushArgsOwned { extra: None, args: args_hash }
        } else {
            windmill_queue::PushArgsOwned::default()
        };
        let script_or_flow_path = if is_hub {
            StripPath(format!("hub/{}", path))
        } else {
            StripPath(path)
        };
        let run_query = RunJobQuery::default();

        let result = if tool_type == "script" {
            run_wait_result_script_by_path_internal(
                db.clone(),
                run_query,
                script_or_flow_path,
                authed.clone(),
                user_db.clone(),
                workspace_id.clone(),
                push_args,
            )
            .await
        } else {
            run_wait_result_flow_by_path_internal(
                db.clone(),
                run_query,
                script_or_flow_path,
                authed.clone(),
                user_db.clone(),
                push_args,
                workspace_id.clone(),
            )
            .await
        };

        match result {
            Ok(response) => {
                let body_bytes = to_bytes(response.into_body(), usize::MAX)
                    .await
                    .map_err(|e| {
                        ErrorData::internal_error(
                            format!("Failed to read response body: {}", e),
                            None,
                        )
                    })?;
                let body_str = String::from_utf8(body_bytes.to_vec()).map_err(|e| {
                    ErrorData::internal_error(
                        format!("Failed to decode response body: {}", e),
                        None,
                    )
                })?;
                Ok(CallToolResult::success(vec![Content::text(body_str)]))
            }
            Err(e) => Err(ErrorData::internal_error(
                format!("Failed to run script: {}", e),
                None,
            )),
        }
    }

    /// Fetches available tools (scripts, flows, hub scripts) based on the user's scope
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        mut _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let http_parts = _context
            .extensions
            .get::<axum::http::request::Parts>()
            .ok_or_else(|| {
                tracing::error!("http::request::Parts not found");
                ErrorData::internal_error("http::request::Parts not found", None)
            })?;

        let authed = http_parts.extensions.get::<ApiAuthed>().ok_or_else(|| {
            tracing::error!("ApiAuthed Axum extension not found");
            ErrorData::internal_error("ApiAuthed Axum extension not found", None)
        })?;

        check_scopes(authed)?;

        let db = http_parts.extensions.get::<DB>().ok_or_else(|| {
            tracing::error!("DB Axum extension not found");
            ErrorData::internal_error("DB Axum extension not found", None)
        })?;

        let user_db = http_parts.extensions.get::<UserDB>().ok_or_else(|| {
            tracing::error!("UserDB Axum extension not found");
            ErrorData::internal_error("UserDB Axum extension not found", None)
        })?;

        let workspace_id = http_parts
            .extensions
            .get::<WorkspaceId>()
            .ok_or_else(|| {
                tracing::error!("WorkspaceId not found");
                ErrorData::internal_error("WorkspaceId not found", None)
            })
            .map(|w_id| w_id.0.clone())?;

        // Parse MCP scopes to determine what to expose
        let scopes = authed.scopes.as_ref().map(|s| s.as_slice()).unwrap_or(&[]);
        let scope_config = parse_mcp_scopes(scopes)?;

        let scope_type = if scope_config.favorites {
            "favorites"
        } else {
            // Fetch all items if either all or granular scope set (we filter later for granular scopes)
            "all"
        };

        let scripts_fn =
            get_items::<ScriptInfo>(user_db, authed, &workspace_id, scope_type, "script");
        let flows_fn = get_items::<FlowInfo>(user_db, authed, &workspace_id, scope_type, "flow");
        let resources_types_fn = get_resources_types(user_db, authed, &workspace_id);
        let hub_scripts_fn = get_scripts_from_hub(db, scope_config.hub_apps.as_deref());
        let (scripts, flows, resources_types, hub_scripts) = if scope_config.hub_apps.is_some() {
            let (scripts, flows, resources_types, hub_scripts) =
                try_join!(scripts_fn, flows_fn, resources_types_fn, hub_scripts_fn)?;
            (scripts, flows, resources_types, hub_scripts)
        } else {
            let (scripts, flows, resources_types) =
                try_join!(scripts_fn, flows_fn, resources_types_fn)?;
            (scripts, flows, resources_types, vec![])
        };

        let mut resources_cache: HashMap<String, Vec<ResourceInfo>> = HashMap::new();
        let mut tools: Vec<Tool> = Vec::new();

        // Filter and add scripts based on scope
        for script in scripts {
            // For granular scopes, filter by path
            if scope_config.granular && !is_resource_allowed(&script.path, &scope_config.scripts) {
                continue;
            }

            tools.push(
                Runner::create_tool_from_item(
                    &script,
                    user_db,
                    authed,
                    &workspace_id,
                    &mut resources_cache,
                    &resources_types,
                )
                .await?,
            );
        }

        // Filter and add flows based on scope
        for flow in flows {
            // For granular scopes, filter by path
            if scope_config.granular && !is_resource_allowed(&flow.path, &scope_config.flows) {
                continue;
            }

            tools.push(
                Runner::create_tool_from_item(
                    &flow,
                    user_db,
                    authed,
                    &workspace_id,
                    &mut resources_cache,
                    &resources_types,
                )
                .await?,
            );
        }

        for hub_script in hub_scripts {
            tools.push(
                Runner::create_tool_from_item(
                    &hub_script,
                    user_db,
                    authed,
                    &workspace_id,
                    &mut resources_cache,
                    &resources_types,
                )
                .await?,
            );
        }

        // Add endpoint tools from the generated MCP tools, filtered by scope
        let endpoint_tools = all_endpoint_tools();
        for endpoint_tool in endpoint_tools {
            // For granular scopes, filter by endpoint name
            if scope_config.granular
                && !is_resource_allowed(&endpoint_tool.name, &scope_config.endpoints)
            {
                continue;
            }

            tools.push(
                endpoint_tools_to_mcp_tools(vec![endpoint_tool])
                    .into_iter()
                    .next()
                    .unwrap(),
            );
        }

        Ok(ListToolsResult { tools, next_cursor: None })
    }

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides a list of scripts and flows the user can run on Windmill. Each flow and script is a tool callable with their respective arguments.".to_string()),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        Ok(self.get_info())
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult { resources: vec![], next_cursor: None })
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

/// Setup the MCP server with HTTP transport
pub async fn setup_mcp_server() -> anyhow::Result<(Router, Arc<LocalSessionManager>)> {
    let session_manager = Arc::new(LocalSessionManager::default());
    let service_config = StreamableHttpServerConfig {
        sse_keep_alive: Some(Duration::from_secs(15)),
        stateful_mode: false,
    };
    let service = StreamableHttpService::new(
        || Ok(Runner::new()),
        session_manager.clone(),
        service_config,
    );

    let router = axum::Router::new().nest_service("/", service);
    Ok((router, session_manager))
}

/// Shutdown the MCP server gracefully by closing all active sessions
pub async fn shutdown_mcp_server(session_manager: Arc<LocalSessionManager>) {
    let session_ids_to_close = {
        let sessions_map = session_manager.sessions.read().await;
        sessions_map.keys().cloned().collect::<Vec<_>>()
    };

    if !session_ids_to_close.is_empty() {
        tracing::info!(
            "Closing {} active MCP session(s)...",
            session_ids_to_close.len()
        );
        let close_futures = session_ids_to_close
            .iter()
            .map(|session_id| {
                let manager_clone = session_manager.clone();
                async move {
                    if let Err(_) = manager_clone.close_session(session_id).await {
                        tracing::warn!("Error closing MCP session");
                    }
                }
            })
            .collect::<Vec<_>>();
        futures::future::join_all(close_futures).await;
    }
}

/// HTTP handler to list MCP tools as JSON
async fn list_mcp_tools_handler() -> JsonResult<Vec<EndpointTool>> {
    let endpoint_tools = all_endpoint_tools();
    Ok(Json(endpoint_tools))
}

/// Creates a router service for listing MCP tools
pub fn list_tools_service() -> Router {
    Router::new().route("/", get(list_mcp_tools_handler))
}
