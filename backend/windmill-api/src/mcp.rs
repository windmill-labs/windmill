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
use crate::jobs::{run_wait_result_script_by_path_internal, RunJobQuery};
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

impl Runner {
    pub fn new() -> Self {
        Self {}
    }

    fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
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
}

fn generate_tool_name(summary: Option<String>, path: &str, last_path: Option<String>) -> String {
    match summary {
        Some(summary) if !summary.is_empty() => {
            let parts: Vec<&str> = summary.split_whitespace().collect();
            parts.join("_")
        }
        _ => {
            // Determine the name based on whether the path is duplicated
            if last_path == Some(path.to_string()) {
                path.replace('/', "_")
            } else {
                // get last part of script after last /
                path.split('/').last().unwrap_or(path).to_string()
            }
        }
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
        let path = tools.tools.iter().find_map(|tool| {
            // Check if annotations exist and then if the title matches
            if tool.name.as_ref() == &request.name {
                if let Some(annotations) = &tool.annotations {
                    if let Some(title) = &annotations.title {
                        Some(title.clone())
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

        // Handle the case where the tool was not found
        let owned_path = match path {
            Some(cow_path) => cow_path, // Convert Cow -> String
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
                args_hash.insert(k, to_raw_value(&v));
            }
            windmill_queue::PushArgsOwned { extra: None, args: args_hash }
        } else {
            windmill_queue::PushArgsOwned::default()
        };

        let w_id = context.workspace_id.clone();
        let script_path = StripPath(owned_path); // Use the owned String path
        let run_query = RunJobQuery::default(); // This assumes RunJobQuery has a default implementation

        let result = run_wait_result_script_by_path_internal(
            db,
            run_query,
            script_path,
            authed,
            user_db,
            w_id,
            push_args,
            None,
        )
        .await;

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
        let script_tools: Vec<Tool> = scripts
            .into_iter()
            .map(|script| {
                let name = generate_tool_name(script.summary, &script.path, last_path.clone());
                last_path = Some(script.path.clone());

                Tool {
                    name: Cow::Owned(name),
                    description: Some(Cow::Owned(script.description.unwrap_or_default())),
                    input_schema: script
                        .schema
                        .map(|schema| {
                            if let serde_json::Value::Object(map) = schema {
                                Arc::new(map)
                            } else {
                                Arc::new(serde_json::Map::new())
                            }
                        })
                        .unwrap_or_default(),
                    annotations: Some(ToolAnnotations::with_title(script.path)),
                }
            })
            .collect();

        let flows = self.inner_get_flows(user_db, authed, workspace_id).await?;
        let flow_tools = flows
            .into_iter()
            .map(|flow| {
                let name = generate_tool_name(flow.summary, &flow.path, last_path.clone());
                last_path = Some(flow.path.clone());

                Tool {
                    name: Cow::Owned(name),
                    description: Some(Cow::Owned(flow.description.unwrap_or_default())),
                    input_schema: flow
                        .schema
                        .map(|schema| {
                            if let serde_json::Value::Object(map) = schema {
                                Arc::new(map)
                            } else {
                                Arc::new(serde_json::Map::new())
                            }
                        })
                        .unwrap_or_default(),
                    annotations: Some(ToolAnnotations::with_title(flow.path)),
                }
            })
            .collect();
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
