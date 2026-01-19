//! Windmill MCP Backend implementation
//!
//! This module provides the concrete implementation of the McpBackend trait
//! for the Windmill platform.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use windmill_common::{db::UserDB, utils::StripPath, DB};
use windmill_mcp::common::transform::apply_key_transformation;
use windmill_mcp::common::types::{
    FlowInfo, HubScriptInfo, ResourceInfo, ResourceType, SchemaType, ScriptInfo,
};
use windmill_mcp::server::{BackendResult, EndpointTool, ErrorData, McpAuth, McpBackend};

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
    LocalSessionManager, Runner, StreamableHttpServerConfig, StreamableHttpService,
};
use windmill_mcp::WorkspaceId;

use axum::{
    extract::Path, http::Request, middleware::Next, response::Response, routing::get, Json, Router,
};
use windmill_common::error::JsonResult;

/// Implement McpAuth for ApiAuthed
impl McpAuth for ApiAuthed {
    fn username(&self) -> &str {
        &self.username
    }

    fn email(&self) -> &str {
        &self.email
    }

    fn is_admin(&self) -> bool {
        self.is_admin
    }

    fn is_operator(&self) -> bool {
        self.is_operator
    }

    fn groups(&self) -> &[String] {
        &self.groups
    }

    fn folders(&self) -> &[(String, bool, bool)] {
        &self.folders
    }

    fn scopes(&self) -> Option<&[String]> {
        self.scopes.as_deref()
    }
}

/// Windmill's MCP backend implementation
#[derive(Clone)]
pub struct WindmillBackend {
    pub db: DB,
    pub user_db: UserDB,
}

impl WindmillBackend {
    pub fn new(db: DB, user_db: UserDB) -> Self {
        Self { db, user_db }
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
    ) -> BackendResult<Vec<ScriptInfo>> {
        let scope_type = if favorites_only { "favorites" } else { "all" };
        get_items::<ScriptInfo>(&self.user_db, auth, workspace_id, scope_type, "script")
            .await
            .map_err(|e| ErrorData::internal_error(e.message, None))
    }

    async fn list_flows(
        &self,
        auth: &ApiAuthed,
        workspace_id: &str,
        favorites_only: bool,
    ) -> BackendResult<Vec<FlowInfo>> {
        let scope_type = if favorites_only { "favorites" } else { "all" };
        get_items::<FlowInfo>(&self.user_db, auth, workspace_id, scope_type, "flow")
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

        for (_key, prop_value) in schema_obj.properties.iter_mut() {
            if let Value::Object(prop_map) = prop_value {
                if let Some(format_value) = prop_map.get("format") {
                    if let Value::String(format_str) = format_value {
                        if format_str.starts_with("resource-") {
                            let resource_type_key =
                                format_str.split("-").last().unwrap_or_default().to_string();
                            let resource_type = resources_types
                                .iter()
                                .find(|rt| rt.name == resource_type_key);
                            let resource_type_obj = resource_type.cloned();

                            if let Some(resource_cache) = resources_cache.get(&resource_type_key) {
                                let resources_count = resource_cache.len();
                                let description = match resource_type_obj {
                                    Some(resource_type_obj) => format!(
                                        "This is a resource named `{}` with the following description: `{}`.\\nThe path of the resource should be used to specify the resource.\\n{}",
                                        resource_type_obj.name,
                                        resource_type_obj.description.as_deref().unwrap_or("No description"),
                                        if resources_count == 0 {
                                            "This resource does not have any available instances, you should create one from your windmill workspace."
                                        } else if resources_count > 1 {
                                            "This resource has multiple available instances, you should precisely select the one you want to use."
                                        } else {
                                            "There is 1 resource available."
                                        }
                                    ),
                                    None => "An object parameter.".to_string(),
                                };
                                prop_map.insert(
                                    "type".to_string(),
                                    Value::String("string".to_string()),
                                );
                                prop_map
                                    .insert("description".to_string(), Value::String(description));
                                if resources_count > 0 {
                                    let resources_description = resource_cache
                                        .iter()
                                        .map(|resource| {
                                            format!(
                                                "{}: $res:{}",
                                                resource
                                                    .description
                                                    .as_deref()
                                                    .unwrap_or("No title"),
                                                resource.path
                                            )
                                        })
                                        .collect::<Vec<String>>()
                                        .join("\\n");

                                    prop_map.insert(
                                        "description".to_string(),
                                        Value::String(format!(
                                            "{}\\nHere are the available resources, in the format title:path. Title can be empty. Path should be used to specify the resource:\\n{}",
                                            prop_map.get("description").unwrap_or(&Value::String("No description".to_string())),
                                            resources_description
                                        )),
                                    );
                                }
                            }
                        }
                    }
                }
            }
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
        )?;
        let query_string = build_query_string(args_map, &endpoint_tool.query_params_schema);
        let full_url = format!(
            "{}/api{}{}",
            windmill_common::BASE_INTERNAL_URL.as_str(),
            path_template,
            query_string
        );

        // Prepare request body
        let body_json =
            build_request_body(&endpoint_tool.method, args_map, &endpoint_tool.body_schema);

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

/// Setup the MCP server with HTTP transport
pub async fn setup_mcp_server(
    db: DB,
    user_db: UserDB,
) -> anyhow::Result<(Router, CancellationToken)> {
    let cancellation_token = CancellationToken::new();
    let session_manager = Arc::new(LocalSessionManager::default());

    let backend = WindmillBackend::new(db, user_db);
    let runner = Runner::new(backend);

    let service_config = StreamableHttpServerConfig {
        sse_keep_alive: Some(Duration::from_secs(15)),
        stateful_mode: false,
        cancellation_token: cancellation_token.clone(),
        sse_retry: Some(Duration::from_secs(15)),
    };

    let service =
        StreamableHttpService::new(move || Ok(runner.clone()), session_manager, service_config);

    let router = Router::new().nest_service("/", service);
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
