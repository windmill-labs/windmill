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
    summary: Option<String>,
    description: Option<String>,
    schema: Option<Schema>,
}

#[derive(Serialize, FromRow)]
struct ItemSchema {
    schema: Option<Schema>,
}

#[derive(Serialize, FromRow, Debug)]
struct FlowInfo {
    path: String,
    summary: Option<String>,
    description: Option<String>,
    schema: Option<Schema>,
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

impl Runner {
    pub fn new() -> Self {
        Self {}
    }

    async fn get_item_schema(
        path: &str,
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: &str,
        item_type: &str,
    ) -> Result<ItemSchema, Error> {
        let mut sqlb = SqlBuilder::select_from(&format!("{} as o", item_type));
        sqlb.fields(&["o.schema"]);
        sqlb.and_where("o.path = ?".bind(&path));
        sqlb.and_where("o.workspace_id = ?".bind(&workspace_id));
        sqlb.and_where("o.archived = false");
        sqlb.and_where("o.draft_only IS NOT TRUE");
        let sql = sqlb.sql().map_err(|_e| {
            tracing::error!("failed to build sql: {}", _e);
            Error::internal_error("failed to build sql", None)
        })?;
        let mut tx = user_db
            .clone()
            .begin(authed)
            .await
            .map_err(|_e| Error::internal_error("failed to begin transaction", None))?;
        let rows = sqlx::query_as::<_, ItemSchema>(&sql)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_e| {
                tracing::error!("failed to fetch item schema: {}", _e);
                Error::internal_error("failed to fetch item schema", None)
            })?;
        tx.commit()
            .await
            .map_err(|_e| Error::internal_error("failed to commit transaction", None))?;
        Ok(rows)
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

    async fn inner_get_resources_types(
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: &str,
    ) -> Result<Vec<ResourceType>, Error> {
        let mut sqlb = SqlBuilder::select_from("resource_type as o");
        sqlb.fields(&["o.name", "o.description"]);
        sqlb.and_where("o.workspace_id = ?".bind(&workspace_id));
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
            .fetch_all(&mut *tx)
            .await
            .map_err(|_e| {
                tracing::error!("Failed to fetch resource types: {}", _e);
                Error::internal_error("failed to fetch resource types", None)
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

    async fn inner_get_items<T: for<'a> sqlx::FromRow<'a, sqlx::postgres::PgRow> + Send + Unpin>(
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: &str,
        scope_type: &str,
        item_type: &str,
    ) -> Result<Vec<T>, Error> {
        let mut sqlb = SqlBuilder::select_from(&format!("{} as o", item_type));
        sqlb.fields(&["o.path", "o.summary", "o.description", "o.schema"]);
        if scope_type == "favorites" {
            sqlb.join("favorite")
                .on("favorite.favorite_kind = ? AND favorite.workspace_id = o.workspace_id AND favorite.path = o.path AND favorite.usr = ?".bind(&item_type)
                    .bind(&authed.username));
        }
        sqlb.and_where("o.workspace_id = ?".bind(&workspace_id))
            .and_where("o.archived = false")
            .and_where("o.draft_only IS NOT TRUE")
            .order_by(
                if item_type == "flow" {
                    "o.edited_at"
                } else {
                    "o.created_at"
                },
                false,
            )
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
        let rows = sqlx::query_as::<_, T>(&sql)
            .fetch_all(&mut *tx)
            .await
            .map_err(|_e| {
                tracing::error!("Failed to fetch {}: {}", item_type, _e);
                Error::internal_error(format!("failed to fetch {}", item_type), None)
            })?;
        tx.commit()
            .await
            .map_err(|_e| Error::internal_error("failed to commit transaction", None))?;
        Ok(rows)
    }

    fn transform_value_if_object(key: &str, value: &Value, schema: &Option<Schema>) -> Value {
        if value.is_string() && value.as_str().unwrap().starts_with("$res:") {
            return value.clone();
        }

        let schema = match schema {
            Some(s) => s,
            None => return value.clone(),
        };

        // Parse schema
        let schema_obj: serde_json::Value = match serde_json::from_str(schema.0.get()) {
            Ok(val) => val,
            Err(_) => return value.clone(),
        };

        // Check if property is defined in schema and is an object type
        let is_obj_type = match schema_obj.get("properties") {
            Some(properties) => match properties.get(key) {
                Some(property) => {
                    let prop_type = property.get("type").and_then(|t| t.as_str());
                    prop_type == Some("object")
                }
                None => false,
            },
            None => false,
        };

        // If it's an object type and we received a string, try to parse it
        if is_obj_type && value.is_string() {
            if let Some(str_val) = value.as_str() {
                if let Ok(obj_val) = serde_json::from_str::<serde_json::Value>(str_val) {
                    return obj_val;
                }
            }
        }

        value.clone()
    }

    async fn transform_schema_for_resources(
        schema: &Schema,
        user_db: &UserDB,
        authed: &ApiAuthed,
        w_id: &str,
        resources_cache: &mut HashMap<String, Vec<ResourceInfo>>,
        resources_types: &Vec<ResourceType>,
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
                    // transform object properties to string because some client does not support object, might change in the future
                    if let serde_json::Value::Object(prop_map) = prop_value {
                        if let Some(type_value) = prop_map.get("type") {
                            if let serde_json::Value::String(type_str) = type_value {
                                if type_str == "object" {
                                    prop_map.insert(
                                        "type".to_string(),
                                        serde_json::Value::String("string".to_string()),
                                    );
                                }
                            }
                        }
                        if let Some(format_value) = prop_map.get("format") {
                            if let serde_json::Value::String(format_str) = format_value {
                                // if property is a resource, fetch the resource type infos, and add each available resource to the description
                                if format_str.starts_with("resource-") {
                                    let resource_type_key = format_str
                                        .split("-")
                                        .last()
                                        .unwrap_or_default()
                                        .to_string();
                                    let resource_type = resources_types
                                        .iter()
                                        .find(|rt| rt.name == resource_type_key);
                                    let resource_type = match resource_type {
                                        Some(resource_type) => resource_type,
                                        None => {
                                            tracing::info!(
                                                "Resource type not found: {}",
                                                resource_type_key
                                            );
                                            continue;
                                        }
                                    };

                                    if !resources_cache.contains_key(&resource_type_key) {
                                        let available_resources = Runner::inner_get_resources(
                                            user_db,
                                            authed,
                                            &w_id,
                                            &resource_type_key,
                                        )
                                        .await;

                                        match available_resources {
                                            Ok(cache_data) => {
                                                resources_cache
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
                                        resources_cache.get(&resource_type_key)
                                    {
                                        let resources_count = resource_cache.len();
                                        let description = format!(
                                            "This is a resource named {} with the following description: {}.\nThe path of the resource should be used to specify the resource.\n{}",
                                            resource_type.name,
                                            resource_type.description.as_deref().unwrap_or("No description"),
                                            if resources_count == 0 {
                                                "This resource does not have any available instances, you should create one from your windmill workspace."
                                            } else if resources_count > 1 {
                                                "This resource has multiple available instances, you should precisely select the one you want to use."
                                            } else {
                                                "There is 1 resource available."
                                            }
                                        );
                                        prop_map.insert(
                                            "type".to_string(),
                                            serde_json::Value::String("string".to_string()),
                                        );
                                        prop_map.insert(
                                            "description".to_string(),
                                            serde_json::Value::String(description),
                                        );
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
                                                .join("\n");

                                            prop_map.insert(
                                                "description".to_string(),
                                                serde_json::Value::String(format!(
                                                    "{}\nHere are the available resources, in the format title:path. Title can be empty. Path should be used to specify the resource:\n{}",
                                                    prop_map.get("description").unwrap_or(&serde_json::Value::String("No description".to_string())),
                                                    resources_description
                                                )),
                                            );
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

        let item_info =
            Runner::get_item_schema(&path, user_db, authed, &context.workspace_id, &tool_type)
                .await?;
        let schema = item_info.schema;
        let push_args = if let Value::Object(map) = args.clone() {
            let mut args_hash = HashMap::new();
            for (k, v) in map {
                // object properties are transformed to string because some client does not support object, might change in the future
                let transformed_v = Runner::transform_value_if_object(&k, &v, &schema);
                args_hash.insert(k, to_raw_value(&transformed_v));
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

        let scripts_fn = Runner::inner_get_items::<ScriptInfo>(
            user_db,
            authed,
            &workspace_id,
            scope_type,
            "script",
        );
        let flows_fn =
            Runner::inner_get_items::<FlowInfo>(user_db, authed, &workspace_id, scope_type, "flow");
        let resources_types_fn = Runner::inner_get_resources_types(user_db, authed, &workspace_id);
        let (scripts, flows, resources_types) =
            try_join!(scripts_fn, flows_fn, resources_types_fn)?;

        let mut resources_cache: HashMap<String, Vec<ResourceInfo>> = HashMap::new();

        let mut script_tools: Vec<Tool> = Vec::with_capacity(scripts.len());
        for script in scripts {
            let name = Runner::transform_path(&script.path, "script").unwrap_or_default();
            let description = format!(
                "This is a script named {} with the following description: {}.",
                script.summary.as_deref().unwrap_or("No summary"),
                script.description.as_deref().unwrap_or("No description")
            );
            let schema_obj = if let Some(schema) = script.schema {
                Runner::transform_schema_for_resources(
                    &schema,
                    user_db,
                    authed,
                    &workspace_id,
                    &mut resources_cache,
                    &resources_types,
                )
                .await?
            } else {
                serde_json::Value::Object(serde_json::Map::new())
            };
            script_tools.push(Tool {
                name: Cow::Owned(name),
                description: Some(Cow::Owned(description)),
                input_schema: {
                    if let serde_json::Value::Object(map) = schema_obj {
                        Arc::new(map)
                    } else {
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
                flow.summary.as_deref().unwrap_or("No summary"),
                flow.description.as_deref().unwrap_or("No description")
            );
            let schema_obj = if let Some(schema) = flow.schema {
                Runner::transform_schema_for_resources(
                    &schema,
                    user_db,
                    authed,
                    &workspace_id,
                    &mut resources_cache,
                    &resources_types,
                )
                .await?
            } else {
                serde_json::Value::Object(serde_json::Map::new())
            };
            flow_tools.push(Tool {
                name: Cow::Owned(name),
                description: Some(Cow::Owned(description)),
                input_schema: {
                    if let serde_json::Value::Object(map) = schema_obj {
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
            instructions: Some("This server provides a list of scripts and flows the user can run on Windmill. Each flow and script is a tool callable with their respective arguments.".to_string()),
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
