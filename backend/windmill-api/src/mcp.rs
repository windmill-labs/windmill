use std::borrow::Cow;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::to_bytes;
use axum::Router;
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use rmcp::{
    handler::server::ServerHandler,
    model::*,
    service::{RequestContext, RoleServer},
    Error,
};
use serde::Serialize;
use serde_json::Value;
use sql_builder::prelude::*;
use sqlx::FromRow;
use tokio::try_join;
use tokio_util::sync::CancellationToken;
use windmill_common::db::UserDB;
use windmill_common::scripts::Schema;
use windmill_common::worker::to_raw_value;
use windmill_common::DB;

use crate::db::ApiAuthed;
use crate::jobs::{
    run_wait_result_flow_by_path_internal, run_wait_result_script_by_path_internal, RunJobQuery,
};
use windmill_common::utils::StripPath;

#[derive(Clone)]
pub struct Runner {}

#[derive(Serialize, FromRow)]
struct ScriptInfo {
    path: String,
    summary: String,
    description: String,
    schema: Schema,
}

#[derive(Serialize, FromRow, Debug)]
struct FlowInfo {
    path: String,
    summary: String,
    description: String,
    schema: Schema,
}

#[derive(Serialize, FromRow, Debug)]
struct ResourceInfo {
    path: String,
    description: Option<String>,
    resource_type: String,
}

#[derive(Serialize, FromRow, Debug)]
struct ResourceType {
    name: String,
    description: Option<String>,
}

#[derive(Serialize, FromRow, Debug)]
struct ResourceCache {
    resource_type: ResourceType,
    resources: Vec<ResourceInfo>,
}

impl Runner {
    pub fn new() -> Self {
        Self {}
    }

    fn transform_path(path: &str, type_str: &str) -> Result<String, String> {
        if type_str != "script" && type_str != "flow" {
            return Err(format!("Invalid type: {}", type_str));
        }

        // Only apply special underscore escaping for paths starting with "f/"
        let transformed = if path.starts_with("f/") {
            let escaped_path = path.replace('_', "__");
            escaped_path.replace('/', "_")
        } else {
            path.replace('/', "_")
        };

        Ok(format!("{}-{}", type_str, transformed))
    }

    fn reverse_transform(transformed_path: &str) -> Result<(&str, String), String> {
        let prefix = if transformed_path.starts_with("script-") {
            "script-"
        } else if transformed_path.starts_with("flow-") {
            "flow-"
        } else {
            return Err(format!(
                "Invalid prefix in transformed path: {}",
                transformed_path
            ));
        };

        let type_str = &prefix[..prefix.len() - 1]; // "script" or "flow"
        let mangled_path = &transformed_path[prefix.len()..];

        // Check if this path was previously transformed with special underscore handling
        let is_special_path = mangled_path.starts_with("f_");

        let original_path = if is_special_path {
            const TEMP_PLACEHOLDER: &str = "@@UNDERSCORE@@";
            let path_with_placeholder = mangled_path.replace("__", TEMP_PLACEHOLDER);
            let path_with_slashes = path_with_placeholder.replace('_', "/");
            path_with_slashes.replace(TEMP_PLACEHOLDER, "_")
        } else {
            mangled_path.replacen('_', "/", 2)
        };

        Ok((type_str, original_path))
    }

    async fn inner_get_resource_type_info(
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: &str,
        resource_type: &str,
    ) -> Result<ResourceType, Error> {
        let mut sqlb = SqlBuilder::select_from("resource_type as o");
        sqlb.fields(&["o.name", "o.description"]);
        sqlb.and_where("o.workspace_id = ?".bind(&workspace_id));
        sqlb.and_where("o.name = ?".bind(&resource_type));
        let sql = sqlb.sql().map_err(|_e| {
            tracing::error!("failed to build sql: {}", _e);
            Error::internal_error("failed to build sql", None)
        })?;
        let mut tx = user_db
            .clone()
            .begin(authed)
            .await
            .map_err(|_e| Error::internal_error("failed to begin transaction", None))?;
        let rows = sqlx::query_as::<_, ResourceType>(&sql)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_e| {
                tracing::error!("Failed to fetch resource info: {}", _e);
                Error::internal_error("failed to fetch resource info", None)
            })?;
        tx.commit()
            .await
            .map_err(|_e| Error::internal_error("failed to commit transaction", None))?;
        Ok(rows)
    }

    async fn inner_get_resources(
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: &str,
        resource_type: &str,
    ) -> Result<Vec<ResourceInfo>, Error> {
        let mut sqlb = SqlBuilder::select_from("resource as o");
        sqlb.fields(&["o.path", "o.description", "o.resource_type"]);
        sqlb.and_where("o.workspace_id = ?".bind(&workspace_id));
        sqlb.and_where("o.resource_type = ?".bind(&resource_type));
        let sql = sqlb.sql().map_err(|_e| {
            tracing::error!("failed to build sql: {}", _e);
            Error::internal_error("failed to build sql", None)
        })?;
        let mut tx = user_db
            .clone()
            .begin(authed)
            .await
            .map_err(|_e| Error::internal_error("failed to begin transaction", None))?;
        let rows = sqlx::query_as::<_, ResourceInfo>(&sql)
            .fetch_all(&mut *tx)
            .await
            .map_err(|_e| {
                tracing::error!("Failed to fetch resources: {}", _e);
                Error::internal_error("failed to fetch resources", None)
            })?;
        tx.commit()
            .await
            .map_err(|_e| Error::internal_error("failed to commit transaction", None))?;

        Ok(rows)
    }

    async fn inner_get_flows(
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: &str,
        scope_type: &str,
    ) -> Result<Vec<FlowInfo>, Error> {
        let mut sqlb = SqlBuilder::select_from("flow as o");
        sqlb.fields(&["o.path", "o.summary", "o.description", "o.schema"]);
        if scope_type == "favorites" {
            sqlb.join("favorite")
                .on("favorite.favorite_kind = 'flow' AND favorite.workspace_id = o.workspace_id AND favorite.path = o.path AND favorite.usr = ?"
                    .bind(&authed.username));
        }
        sqlb.and_where("o.workspace_id = ?".bind(&workspace_id))
            .and_where("o.archived = false")
            .and_where("o.draft_only IS NOT TRUE")
            .order_by("o.edited_at", false)
            .limit(100);
        let sql = sqlb.sql().map_err(|_e| {
            tracing::error!("failed to build sql: {}", _e);
            Error::internal_error("failed to build sql", None)
        })?;
        let mut tx = user_db
            .clone()
            .begin(authed)
            .await
            .map_err(|_e| Error::internal_error("failed to begin transaction", None))?;
        let rows = sqlx::query_as::<_, FlowInfo>(&sql)
            .fetch_all(&mut *tx)
            .await
            .map_err(|_e| {
                tracing::error!("Failed to fetch flows: {}", _e);
                Error::internal_error("failed to fetch flows", None)
            })?;
        tx.commit()
            .await
            .map_err(|_e| Error::internal_error("failed to commit transaction", None))?;
        Ok(rows)
    }

    async fn inner_get_scripts(
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: &str,
        scope_type: &str,
    ) -> Result<Vec<ScriptInfo>, Error> {
        let mut sqlb = SqlBuilder::select_from("script as o");
        sqlb.fields(&["o.path", "o.summary", "o.description", "o.schema"]);
        if scope_type == "favorites" {
            sqlb.join("favorite")
                .on("favorite.favorite_kind = 'script' AND favorite.workspace_id = o.workspace_id AND favorite.path = o.path AND favorite.usr = ?"
                    .bind(&authed.username));
        }
        sqlb.and_where("o.workspace_id = ?".bind(&workspace_id))
            .and_where("o.archived = false")
            .and_where("o.draft_only IS NOT TRUE")
            .order_by("o.created_at", false)
            .limit(100);
        let sql = sqlb.sql().map_err(|_e| {
            tracing::error!("failed to build sql: {}", _e);
            Error::internal_error("failed to build sql", None)
        })?;
        let mut tx = user_db
            .clone()
            .begin(authed)
            .await
            .map_err(|_e| Error::internal_error("failed to begin transaction", None))?;
        let rows = sqlx::query_as::<_, ScriptInfo>(&sql)
            .fetch_all(&mut *tx)
            .await
            .map_err(|_e| {
                tracing::error!("Failed to fetch scripts: {}", _e);
                Error::internal_error("failed to fetch scripts", None)
            })?;
        tx.commit()
            .await
            .map_err(|_e| Error::internal_error("failed to commit transaction", None))?;
        Ok(rows)
    }

    async fn transform_schema_for_resources(
        schema: &Schema,
        user_db: &UserDB,
        authed: &ApiAuthed,
        w_id: &str,
        resources_info: &mut HashMap<String, ResourceCache>,
    ) -> Result<serde_json::Value, Error> {
        let mut schema_obj: serde_json::Value = match serde_json::from_str(schema.0.get()) {
            Ok(val) => val,
            Err(_) => serde_json::Value::Object(serde_json::Map::new()), // Default if JSON is empty/invalid
        };

        if let serde_json::Value::Object(schema_map) = &mut schema_obj {
            if let Some(serde_json::Value::Object(properties_map)) =
                schema_map.get_mut("properties")
            {
                for (_key, prop_value) in properties_map.iter_mut() {
                    if let serde_json::Value::Object(prop_map) = prop_value {
                        if let Some(format_value) = prop_map.get("format") {
                            if let serde_json::Value::String(format_str) = format_value {
                                if format_str.contains("resource") {
                                    let resource_type_key = format_str
                                        .split("-")
                                        .last()
                                        .unwrap_or_default()
                                        .to_string();

                                    if !resources_info.contains_key(&resource_type_key) {
                                        let fetch_result = async {
                                            let resource_type_info_future =
                                                Runner::inner_get_resource_type_info(
                                                    user_db,
                                                    authed,
                                                    &w_id,
                                                    &resource_type_key,
                                                );
                                            let resources_data_future = Runner::inner_get_resources(
                                                user_db,
                                                authed,
                                                &w_id,
                                                &resource_type_key,
                                            );
                                            let (resource_type_info, resources_data) = try_join!(
                                                resource_type_info_future,
                                                resources_data_future
                                            )?;
                                            Ok::<_, Error>(ResourceCache {
                                                resource_type: resource_type_info,
                                                resources: resources_data,
                                            })
                                        }
                                        .await;

                                        match fetch_result {
                                            Ok(cache_data) => {
                                                resources_info
                                                    .insert(resource_type_key.clone(), cache_data);
                                            }
                                            Err(e) => {
                                                tracing::error!(
                                                    "Failed to fetch resource cache data: {}",
                                                    e
                                                );
                                                continue; // Skip this property if fetching failed
                                            }
                                        }
                                    }

                                    if let Some(resource_cache) =
                                        resources_info.get(&resource_type_key)
                                    {
                                        let resources_count = resource_cache.resources.len();

                                        // Update the property map directly
                                        prop_map.insert(
                                            "type".to_string(),
                                            serde_json::Value::String("string".to_string()),
                                        );
                                        let description = format!(
                                            "This is a resource named {} with the following description: {}.\nThe path of the resource should be used to specify the resource.\n{}",
                                            resource_cache.resource_type.name,
                                            resource_cache.resource_type.description.as_deref().unwrap_or("No description"),
                                            if resources_count == 0 {
                                                "This resource does not have any available instances, you should create one from your windmill workspace"
                                            } else if resources_count > 1 {
                                                "This resource has multiple available instances, you should precisely select the one you want to use"
                                            } else {
                                                "There is 1 resource available"
                                            }
                                        );
                                        prop_map.insert(
                                            "description".to_string(),
                                            serde_json::Value::String(description),
                                        );

                                        if resources_count > 0 {
                                            let one_of_values: Vec<Value> = resource_cache
                                                .resources
                                                .iter()
                                                .map(|resource| {
                                                    serde_json::Value::Object(
                                                        serde_json::Map::from_iter(
                                                            [
                                                                (
                                                                    "const".to_string(),
                                                                    serde_json::Value::String(
                                                                        format!(
                                                                            "$res:{}",
                                                                            resource.path
                                                                        ),
                                                                    ),
                                                                ),
                                                                (
                                                                    "title".to_string(),
                                                                    serde_json::Value::String(
                                                                        resource
                                                                            .description
                                                                            .as_deref()
                                                                            .unwrap_or(
                                                                                "No description",
                                                                            )
                                                                            .to_string(),
                                                                    ),
                                                                ),
                                                            ]
                                                            .into_iter(),
                                                        ),
                                                    )
                                                })
                                                .collect();
                                            prop_map.insert(
                                                "oneOf".to_string(),
                                                serde_json::Value::Array(one_of_values),
                                            );
                                        } else {
                                            prop_map.remove("oneOf"); // Remove oneOf if it exists and count is 0
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        tracing::warn!(
                            "Schema property value is not a JSON object: {:?}",
                            prop_value
                        );
                    }
                }
            } else {
                tracing::info!(
                    "Schema does not contain a 'properties' object or it's not an object."
                );
            }
        } else {
            tracing::warn!("Top-level schema value is not a JSON object.");
        }

        Ok(schema_obj)
    }
}

impl ServerHandler for Runner {
    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, Error> {
        let parse_args = |args_opt: Option<JsonObject>| -> Result<Value, Error> {
            args_opt.map(Value::Object).ok_or_else(|| {
                Error::invalid_params(
                    "Missing arguments for tool",
                    Some(request.name.clone().into()),
                )
            })
        };

        let authed = context
            .req_extensions
            .get::<ApiAuthed>()
            .ok_or_else(|| Error::internal_error("ApiAuthed not found", None))?;
        let db = context
            .req_extensions
            .get::<DB>()
            .ok_or_else(|| Error::internal_error("DB not found", None))?;
        let user_db = context
            .req_extensions
            .get::<UserDB>()
            .ok_or_else(|| Error::internal_error("UserDB not found", None))?;
        let args = parse_args(request.arguments)?;

        let (tool_type, path) = Runner::reverse_transform(&request.name).unwrap_or_default();

        // Convert Value to PushArgsOwned
        let push_args = if let Value::Object(map) = args.clone() {
            let mut args_hash = HashMap::new();
            for (k, v) in map {
                args_hash.insert(k, to_raw_value(&v));
            }
            windmill_queue::PushArgsOwned { extra: None, args: args_hash }
        } else {
            windmill_queue::PushArgsOwned::default()
        };

        let w_id = context.workspace_id.clone();
        let script_or_flow_path = StripPath(path);
        let run_query = RunJobQuery::default();

        let result = if tool_type == "script" {
            run_wait_result_script_by_path_internal(
                db.clone(),
                run_query,
                script_or_flow_path,
                authed.clone(),
                user_db.clone(),
                w_id.clone(),
                push_args,
                None,
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
                w_id.clone(),
                None,
            )
            .await
        };

        match result {
            Ok(response) => {
                // Extract the response body as bytes, then convert to a string
                let body_bytes = to_bytes(response.into_body(), usize::MAX)
                    .await
                    .map_err(|e| {
                        Error::internal_error(format!("Failed to read response body: {}", e), None)
                    })?;
                let body_str = String::from_utf8(body_bytes.to_vec()).map_err(|e| {
                    Error::internal_error(format!("Failed to decode response body: {}", e), None)
                })?;
                Ok(CallToolResult::success(vec![Content::text(body_str)]))
            }
            Err(e) => Err(Error::internal_error(
                format!("Failed to run script: {}", e),
                None,
            )),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        mut _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, Error> {
        let workspace_id = _context.workspace_id.clone();
        let user_db = _context
            .req_extensions
            .get::<UserDB>()
            .ok_or_else(|| Error::internal_error("UserDB not found", None))?;
        let authed = _context
            .req_extensions
            .get::<ApiAuthed>()
            .ok_or_else(|| Error::internal_error("ApiAuthed not found", None))?;
        let scope = authed
            .scopes
            .as_ref()
            .and_then(|scopes| scopes.iter().find(|scope| scope.starts_with("mcp:")));
        let scope_type = scope.map_or("all", |scope| scope.split(":").last().unwrap_or("all"));
        let mut resources_info: HashMap<String, ResourceCache> = HashMap::new();

        let scripts_fn = Runner::inner_get_scripts(user_db, authed, &workspace_id, scope_type);
        let flows_fn = Runner::inner_get_flows(user_db, authed, &workspace_id, scope_type);
        let (scripts, flows) = try_join!(scripts_fn, flows_fn)?;

        let mut script_tools: Vec<Tool> = Vec::with_capacity(scripts.len());
        for script in scripts {
            let name = Runner::transform_path(&script.path, "script").unwrap_or_default();
            let description = format!(
                "This is a script named {} with the following description: {}.",
                script.summary, script.description
            );
            let schema: Schema = script.schema;
            let schema_obj = Runner::transform_schema_for_resources(
                &schema,
                user_db,
                authed,
                &workspace_id,
                &mut resources_info,
            )
            .await?;
            script_tools.push(Tool {
                name: Cow::Owned(name),
                description: Some(Cow::Owned(description)),
                input_schema: {
                    // Directly use the returned Value
                    if let serde_json::Value::Object(map) = schema_obj {
                        Arc::new(map)
                    } else {
                         tracing::warn!("Schema for script {} was not a valid JSON object after transformation, using empty schema.", script.path);
                         Arc::new(serde_json::Map::new())
                    }
                },
                annotations: None,
            });
        }

        let mut flow_tools: Vec<Tool> = Vec::with_capacity(flows.len());
        for flow in flows {
            let name = Runner::transform_path(&flow.path, "flow").unwrap_or_default();
            let description = format!(
                "This is a flow named {} with the following description: {}.",
                flow.summary, flow.description
            );
            let mut schema: Schema = flow.schema;
            Runner::transform_schema_for_resources(
                &mut schema,
                user_db,
                authed,
                &workspace_id,
                &mut resources_info,
            )
            .await?;
            flow_tools.push(Tool {
                name: Cow::Owned(name),
                description: Some(Cow::Owned(description)),
                input_schema: {
                    let value = serde_json::to_value(schema).unwrap_or_default();
                    if let serde_json::Value::Object(map) = value {
                        Arc::new(map)
                    } else {
                        Arc::new(serde_json::Map::new())
                    }
                },
                annotations: None,
            });
        }

        let tools = [script_tools, flow_tools].concat();
        Ok(ListToolsResult { tools, next_cursor: None })
    }

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides a runner tool that can run scripts. Use 'get_scripts' to get the list of scripts.".to_string()),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, Error> {
        Ok(self.get_info())
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, Error> {
        Ok(ListResourcesResult { resources: vec![], next_cursor: None })
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, Error> {
        Ok(ListPromptsResult::default())
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, Error> {
        Ok(ListResourceTemplatesResult::default())
    }
}

pub fn setup_mcp_server(addr: SocketAddr, path: &str) -> anyhow::Result<(SseServer, Router)> {
    let config = SseServerConfig {
        bind: addr,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        full_message_path: path.to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: None,
    };

    Ok(SseServer::new(config))
}
