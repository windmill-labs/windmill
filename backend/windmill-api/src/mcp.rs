use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use axum::body::to_bytes;
use axum::Router;
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use rmcp::{
    handler::server::ServerHandler,
    model::*,
    schemars::{self, JsonSchema},
    service::{RequestContext, RoleServer},
    Error,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use sqlx::Row;
use tokio_util::sync::CancellationToken;
use windmill_common::db::UserDB;
use windmill_common::worker::to_raw_value;
use windmill_common::DB;

use crate::db::ApiAuthed;
use crate::jobs::{run_wait_result_script_by_path_internal, RunJobQuery};
use windmill_common::utils::StripPath;

const BIND_ADDRESS: &str = "127.0.0.1:8008"; // This address is only used when running standalone

#[derive(Clone)]
pub struct Runner {}

#[derive(Deserialize, Serialize, JsonSchema)]
struct GetScriptSchemaByPathParams {
    #[schemars(description = "The script path to get the schema for")]
    path: String,
}

#[derive(Deserialize, Serialize, JsonSchema)]
struct RunScriptParams {
    #[schemars(description = "The script path to run")]
    script: String,
    #[schemars(description = "The script arguments")]
    args: String,
}

#[derive(Serialize, FromRow)]
struct ScriptInfo {
    path: String,
    summary: Option<String>,
    description: Option<String>,
    schema: Option<Value>,
}

impl Runner {
    pub fn new() -> Self {
        Self {}
    }

    fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
    }

    async fn inner_get_scripts(
        &self,
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: String,
    ) -> Result<Vec<ScriptInfo>, Error> {
        let mut tx = user_db.clone().begin(authed).await.map_err(|e| {
            Error::internal_error(format!("Failed to begin transaction: {}", e), None)
        })?;

        // Skip the sqlx::query_as! macro to avoid conversion issues
        // Use sqlx::query directly and convert manually
        let rows = sqlx::query(
            "SELECT path, summary, description, schema FROM script WHERE workspace_id = $1 AND archived = false ORDER BY created_at DESC LIMIT 100"
        )
        .bind(workspace_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| Error::internal_error(format!("Failed to fetch scripts: {}", e), None))?;

        let mut scripts = Vec::with_capacity(rows.len());
        for row in rows {
            let path: String = row
                .try_get("path")
                .map_err(|e| Error::internal_error(format!("Failed to get path: {}", e), None))?;
            let summary: Option<String> = row.try_get("summary").map_err(|e| {
                Error::internal_error(format!("Failed to get summary: {}", e), None)
            })?;
            let description: Option<String> = row.try_get("description").map_err(|e| {
                Error::internal_error(format!("Failed to get description: {}", e), None)
            })?;
            let schema_json: Option<Value> = row
                .try_get("schema")
                .map_err(|e| Error::internal_error(format!("Failed to get schema: {}", e), None))?;

            let schema = schema_json;

            scripts.push(ScriptInfo { path, summary, description, schema });
        }

        tx.commit().await.map_err(|e| {
            Error::internal_error(format!("Failed to commit transaction: {}", e), None)
        })?;

        Ok(scripts)
    }
}

impl ServerHandler for Runner {
    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, Error> {
        tracing::debug!("Handling call_tool request: {}", request.name);

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
        tracing::info!("Handling list_tools request");
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
            .inner_get_scripts(user_db, authed, workspace_id)
            .await?;
        let mut last_path = scripts.first().map(|script| script.path.clone());
        let tools = scripts
            .into_iter()
            .map(|script| {
                // if summary exist and is not empty, use it
                let name = match script.summary {
                    Some(summary) if !summary.is_empty() => {
                        let parts: Vec<&str> = summary.split_whitespace().collect();
                        parts.join("_")
                    }
                    _ => {
                        // Determine the name based on whether the path is duplicated
                        let calculated_name = if last_path == Some(script.path.clone()) {
                            script.path.replace('/', "_")
                        } else {
                            // get last part of script after last /
                            script
                                .path
                                .split('/')
                                .last()
                                .unwrap_or(&script.path)
                                .to_string()
                        };
                        // Update last_path *after* determining the name
                        last_path = Some(script.path.clone());
                        // Return the calculated name
                        calculated_name
                    }
                };
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

        Ok(ListToolsResult { tools, next_cursor: None })
    }

    fn get_info(&self) -> ServerInfo {
        tracing::debug!("Handling get_info request");
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
        tracing::info!("initialize called");
        tracing::info!(
            "Context extensions in initialize: {:?}",
            _context.extensions.get::<String>()
        );
        Ok(self.get_info())
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, Error> {
        tracing::warn!("list_resources called but not implemented");
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

    async fn on_cancelled(&self, _params: CancelledNotificationParam) {
        tracing::debug!("on_cancelled notification received");
    }

    async fn on_progress(&self, _params: ProgressNotificationParam) {
        tracing::debug!("on_progress notification received");
    }

    async fn on_initialized(&self) {
        tracing::debug!("on_initialized notification received");
    }

    async fn on_roots_list_changed(&self) {
        tracing::debug!("on_roots_list_changed notification received");
    }
}

pub fn setup_mcp_server(path: &str) -> anyhow::Result<(SseServer, Router)> {
    let config = SseServerConfig {
        bind: BIND_ADDRESS.parse()?,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        full_message_path: path.to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: None,
    };

    Ok(SseServer::new(config))
}
