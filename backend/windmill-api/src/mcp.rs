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
use tokio_util::sync::CancellationToken;
use windmill_common::db::UserDB;
use windmill_common::worker::to_raw_value;
use windmill_common::DB;

use crate::ai::AIConfig;
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
    schema: Option<Value>,
}

#[derive(Serialize, FromRow, Debug)]
struct FlowInfo {
    path: String,
    summary: Option<String>,
    description: Option<String>,
    schema: Option<Value>,
}

#[derive(Serialize, FromRow, Debug)]
struct WorkspaceSettings {
    ai_config: Option<sqlx::types::Json<AIConfig>>,
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

    fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
    }

    async fn inner_get_resource_info(
        &self,
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: String,
        resource_type: String,
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
        &self,
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: String,
        resource_types: Vec<String>,
    ) -> Result<Vec<ResourceInfo>, Error> {
        let mut sqlb = SqlBuilder::select_from("resource as o");
        sqlb.fields(&["o.path", "o.description", "o.resource_type"]);
        sqlb.and_where("o.workspace_id = ?".bind(&workspace_id));
        let prepared_resource_types: Vec<String> = resource_types
        .iter()
        .map(|s| quote(s))
        .collect();
    
    if !prepared_resource_types.is_empty() {
        sqlb.and_where_in("o.resource_type", &prepared_resource_types);
    } else {
        println!("Warning: resource_types is empty, IN clause will not be added.");
    }
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
        &self,
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: String,
    ) -> Result<Vec<FlowInfo>, Error> {
        let mut sqlb = SqlBuilder::select_from("flow as o");
        sqlb.fields(&["o.path", "o.summary", "o.description", "o.schema"]).join("favorite")
                .on("favorite.favorite_kind = 'flow' AND favorite.workspace_id = o.workspace_id AND favorite.path = o.path AND favorite.usr = ?"
                    .bind(&authed.username));
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
        &self,
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: String,
    ) -> Result<Vec<ScriptInfo>, Error> {
        let mut sqlb = SqlBuilder::select_from("script as o");
        sqlb.fields(&["o.path", "o.summary", "o.description", "o.schema"]).join("favorite")
                .on("favorite.favorite_kind = 'script' AND favorite.workspace_id = o.workspace_id AND favorite.path = o.path AND favorite.usr = ?"
                    .bind(&authed.username));
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

    fn generate_tool_name(&self, summary: Option<String>, path: &str, last_path: Option<String>) -> String {
        match summary {
            // if summary exist and is not empty, use it
            Some(summary) if !summary.is_empty() => {
                let parts: Vec<&str> = summary.split_whitespace().collect();
                parts.join("_")
            }
            _ => {
                // if path is duplicated, use the full path
                if last_path == Some(path.to_string()) {
                    path.replace('/', "_")
                } else {
                    // if path is not duplicated, use the last part of the path
                    path.split('/').last().unwrap_or(path).to_string()
                }
            }
        }
    }
    
    async fn transform_schema_for_resources(&self, schema: &mut serde_json::Value, user_db: &UserDB, authed: &ApiAuthed, w_id: String) -> Result<(), Error> {
        if let serde_json::Value::Object(schema_obj) = schema {
            if let Some(serde_json::Value::Object(properties)) = schema_obj.get_mut("properties") {
                for (_key, prop) in properties.iter_mut() {
                    if let serde_json::Value::Object(prop_obj) = prop {
                        if let Some(serde_json::Value::String(format)) = prop_obj.get("format") {
                            if format.contains("resource") {
                                let resource_type = format.split("-").last().unwrap_or_default();
                                let resource_info = self.inner_get_resource_info(&user_db, &authed, w_id.clone(), resource_type.to_string()).await?;
                                let resources = self.inner_get_resources(&user_db, &authed, w_id.clone(), vec![resource_type.to_string()]).await?;
                                prop_obj.insert("description".to_string(), serde_json::Value::String(format!("This is a resource named {} with the following description: {}. It contains a predefined set of resources. The path of the resource should be used to specify the resource. If no resource is found, the LLM should tell the user that no resources are available and he should create one from his windmill workspace.", resource_info.name, resource_info.description.as_deref().unwrap_or("No description"))));
                                prop_obj.insert("oneOf".to_string(), serde_json::Value::Array(resources.iter().map(|resource| serde_json::Value::Object(serde_json::Map::from_iter([("const".to_string(), serde_json::Value::String(resource.path.clone())), ("title".to_string(), serde_json::Value::String(resource.description.as_deref().unwrap_or("No description").to_string()))].into_iter()))).collect()));
                                prop_obj.insert("type".to_string(), serde_json::Value::String("string".to_string()));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
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

        let authed = context.req_extensions.get::<ApiAuthed>().unwrap().clone();
        let db = context.req_extensions.get::<DB>().unwrap().clone();
        let user_db = context.req_extensions.get::<UserDB>().unwrap().clone();
        let args = parse_args(request.arguments)?;

        // find path from list of tools, by checking annotations.title
        let tools = self.list_tools(None, context.clone()).await?; // Clone context for reuse
        let path_and_schema = tools.tools.iter().find_map(|tool| {
            // Check if annotations exist and then if the title matches
            if tool.name.as_ref() == &request.name {
                if let Some(annotations) = &tool.annotations {
                    if let Some(title) = &annotations.title {
                        Some((title.clone(), tool.input_schema.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        });

        let (path, schema) = match path_and_schema {
            Some((path, schema)) => (Some(path), Some(schema)),
            None => (None, None),
        };

        let (tool_type, path) = match path {
            Some(path) => {
                let split = path.split(":").collect::<Vec<&str>>();
                if split.len() == 2 {
                    (split[0].to_string(), split[1].to_string())
                } else {
                    return Err(Error::invalid_params(
                        format!(
                            "Tool with name '{}' not found or title mismatch",
                            request.name
                        ),
                        Some(request.name.into()),
                    ));
                }
            }
            None => {
                return Err(Error::invalid_params(
                    format!(
                        "Tool with name '{}' not found or title mismatch",
                        request.name
                    ),
                    Some(request.name.into()),
                ));
            }
        };

        // Convert Value to PushArgsOwned
        let push_args = if let Value::Object(map) = args.clone() {
            let mut args_hash = HashMap::new();
            for (k, v) in map {
                // Check if we need to transform this argument based on schema
                let mut value_to_use = v.clone();
                if let Some(ref schema) = schema {
                    if let Some(properties) = schema.get("properties") {
                        if let Some(property) = properties.get(&k) {
                            if let Some(format) = property.get("format") {
                                if let Value::String(format_str) = format {
                                    if format_str.starts_with("resource") {
                                        // Modify string values to have "$res:" prefix for resource types
                                        if let Value::String(s) = &v {
                                            value_to_use = Value::String(format!("$res:{}", s));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                args_hash.insert(k, to_raw_value(&value_to_use.clone()));
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
                db,
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
                db,
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
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, Error> {
        let workspace_id = _context.workspace_id;
        let user_db = _context
            .req_extensions
            .get::<UserDB>()
            .ok_or_else(|| Error::internal_error("UserDB not found", None))?;
        let authed = _context
            .req_extensions
            .get::<ApiAuthed>()
            .ok_or_else(|| Error::internal_error("ApiAuthed not found", None))?;
        let scripts = self
            .inner_get_scripts(user_db, authed, workspace_id.clone())
            .await?;
        let mut last_path = scripts.first().map(|script| script.path.clone());
        let mut script_tools: Vec<Tool> = Vec::with_capacity(scripts.len());
        for script in scripts {
            let name = self.generate_tool_name(script.summary, &script.path, last_path.clone());
            last_path = Some(script.path.clone());
            let mut schema = script.schema.unwrap_or_default();
            self.transform_schema_for_resources(&mut schema, user_db, authed, workspace_id.clone()).await?;
            script_tools.push(Tool {
                name: Cow::Owned(name),
                description: Some(Cow::Owned(script.description.unwrap_or_default())),
                input_schema: if let serde_json::Value::Object(map) = schema {
                    Arc::new(map)
                } else {
                    Arc::new(serde_json::Map::new())
                },
                annotations: Some(ToolAnnotations::with_title(format!("script:{}", script.path))),
            });
        }

        let flows = self.inner_get_flows(user_db, authed, workspace_id.clone()).await?;
        let mut last_path = flows.first().map(|flow| flow.path.clone());
        let mut flow_tools: Vec<Tool> = Vec::with_capacity(flows.len());
        for flow in flows {
            let name = self.generate_tool_name(flow.summary, &flow.path, last_path.clone());
            last_path = Some(flow.path.clone());
            let mut schema = flow.schema.unwrap_or_default();
            self.transform_schema_for_resources(&mut schema, user_db, authed, workspace_id.clone()).await?;
            flow_tools.push(Tool {
                name: Cow::Owned(name),
                description: Some(Cow::Owned(flow.description.unwrap_or_default())),
                input_schema: if let serde_json::Value::Object(map) = schema {
                    Arc::new(map)
                } else {
                    Arc::new(serde_json::Map::new())
                },
                annotations: Some(ToolAnnotations::with_title(format!("flow:{}", flow.path))),
            });
        }

        Ok(ListToolsResult { tools: [script_tools, flow_tools].concat(), next_cursor: None })
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
