use std::borrow::Cow;

use axum::Router;
use chrono::{DateTime, Utc};
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
use tokio_util::sync::CancellationToken;
use windmill_common::db::UserDB;

use crate::db::ApiAuthed;

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

#[derive(Deserialize)]
struct ScriptSchemaResponse {
    schema: Value,
}

#[derive(Serialize, FromRow)]
struct ScriptInfo {
    path: String,
    summary: Option<String>,
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
        let scripts: Vec<ScriptInfo> = sqlx::query_as!(
            ScriptInfo,
            "SELECT path, summary FROM script WHERE workspace_id = $1 AND archived = false ORDER BY created_at DESC LIMIT 100",
            workspace_id
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| Error::internal_error(format!("Failed to fetch scripts: {}", e), None))?;

        tx.commit().await.map_err(|e| {
            Error::internal_error(format!("Failed to commit transaction: {}", e), None)
        })?;

        Ok(scripts)
    }

    async fn get_scripts(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, Error> {
        let ct = context.ct;

        tokio::select! {
             _ = ct.cancelled() => {
                tracing::info!("get_scripts cancelled.");
                return Err(Error::internal_error("Operation cancelled", None));
            }
            result = async {

                let workspace_id = "admins".to_string();
                let user_db = context.req_extensions.get::<UserDB>().unwrap();
                let authed = context.req_extensions.get::<ApiAuthed>().unwrap();
                let scripts = self.inner_get_scripts(user_db, authed, workspace_id).await?;

                let content = Content::json(scripts)?;
                Ok(CallToolResult::success(vec![content]))

            } => { result }
        }
    }

    async fn get_script_schema_by_path(
        &self,
        context: RequestContext<RoleServer>,
        path: String,
    ) -> Result<CallToolResult, Error> {
        let ct = context.ct;

        tokio::select! {
             _ = ct.cancelled() => {
                tracing::info!("get_script_schema_by_path cancelled.");
                return Err(Error::internal_error("Operation cancelled", None));
            }
            result = async {
                Ok(CallToolResult::success(vec![Content::text(
                    "Hello, world!".to_string(),
                )]))
            } => { result }
        }
    }

    async fn run_script(
        &self,
        context: RequestContext<RoleServer>,
        script: String,
        args: String,
    ) -> Result<CallToolResult, Error> {
        let ct = context.ct;

        tokio::select! {
             _ = ct.cancelled() => {
                tracing::info!("run_script cancelled.");
                return Err(Error::internal_error("Operation cancelled", None));
            }
            result = async {
                Ok(CallToolResult::success(vec![Content::text(
                    "Hello, world!".to_string(),
                )]))
            } => { result }
        }
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

        match request.name.as_ref() {
            "get_scripts" => {
                if request.arguments.is_some() && !request.arguments.as_ref().unwrap().is_empty() {
                    return Err(Error::invalid_params(
                        "get_scripts takes no arguments",
                        None,
                    ));
                }
                self.get_scripts(context).await
            }
            "get_script_schema_by_path" => {
                let args_val = parse_args(request.arguments)?;
                let params: GetScriptSchemaByPathParams = serde_json::from_value(args_val)
                    .map_err(|e| {
                        Error::invalid_params(
                            format!("Invalid arguments for get_script_schema_by_path: {}", e),
                            None,
                        )
                    })?;
                self.get_script_schema_by_path(context, params.path).await
            }
            "run_script" => {
                let args_val = parse_args(request.arguments)?;
                let params: RunScriptParams = serde_json::from_value(args_val).map_err(|e| {
                    Error::invalid_params(format!("Invalid arguments for run_script: {}", e), None)
                })?;
                self.run_script(context, params.script, params.args).await
            }
            _ => {
                tracing::warn!("Received call for unknown tool: {}", request.name);
                Err(Error::invalid_params(
                    format!("Unknown tool: {}", request.name),
                    None,
                ))
            }
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, Error> {
        tracing::debug!("Handling list_tools request");
        let workspace_id = "admins".to_string();
        let user_db = _context.req_extensions.get::<UserDB>().unwrap();
        let authed = _context.req_extensions.get::<ApiAuthed>().unwrap();
        let scripts = self
            .inner_get_scripts(user_db, authed, workspace_id)
            .await?;
        let tools = scripts
            .iter()
            .map(|script| Tool {
                name: Cow::Owned(script.path.clone()),
                description: Some(Cow::Owned(script.summary.clone().unwrap_or_default())),
                input_schema: rmcp::handler::server::tool::cached_schema_for_type::<EmptyObject>(),
                annotations: None,
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

pub fn setup_mcp_server() -> anyhow::Result<(SseServer, Router)> {
    let config = SseServerConfig {
        bind: BIND_ADDRESS.parse()?,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: None,
    };

    Ok(SseServer::new(config))
}
