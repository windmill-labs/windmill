use hyper::Client;
use hyper::Request;
use hyper::body::Body;
use rmcp::{
    Error,
    handler::server::ServerHandler,
    model::*,
    schemars::{self, JsonSchema},
    service::{Peer, RequestContext, RoleServer},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Runner {
    token: Option<String>,
}

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

impl Runner {
    pub fn new() -> Self {
        Self { token: None }
    }

    pub fn update_user_token(&mut self, token: String) -> Result<(), Error> {
        tracing::info!("Updating user token: {}", token);
        self.token = Some(token);
        Ok(())
    }

    fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
    }

    async fn get_scripts(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, Error> {
        // tracing::info!(
        //     "get_scripts called via manual handler. Context extensions: {:?}",
        //     context.extensions
        // );
        // tracing::info!(
        //     "get_scripts called via manual handler. Context : {:?}",
        //     context
        // );
        // tracing::info!(
        //     "get_scripts called via manual handler. Context peer: {:?}",
        //     context.peer.peer_info()
        // );
        let ct = context.ct;

        tokio::select! {
             _ = ct.cancelled() => {
                tracing::info!("get_scripts cancelled.");
                return Err(Error::internal_error("Operation cancelled", None));
            }
            result = async {
                let client = Client::new();
                let req = Request::builder()
                    .method("GET")
                    .uri("http://localhost:8000/api/w/admins/scripts/list")
                    .header(
                        "Authorization",
                        format!(
                            "Bearer {}",
                            self.token.clone().unwrap_or_default()
                        ),
                    )
                    .body(Body::empty())
                    .map_err(|e| Error::internal_error(e.to_string(), None))?;
                let response = client
                    .request(req)
                    .await
                    .map_err(|e| Error::internal_error(e.to_string(), None))?;
                let body = hyper::body::to_bytes(response.into_body())
                    .await
                    .map_err(|e| Error::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![Content::text(
                    String::from_utf8_lossy(&body).into_owned(),
                )]))
            } => { result }
        }
    }

    async fn get_script_schema_by_path(
        &self,
        context: RequestContext<RoleServer>,
        path: String,
    ) -> Result<CallToolResult, Error> {
        tracing::info!(
            "get_script_schema_by_path called via manual handler. Path: {}, Context ID: {:?}",
            path,
            context.id
        );
        let ct = context.ct;

        tokio::select! {
             _ = ct.cancelled() => {
                tracing::info!("get_script_schema_by_path cancelled.");
                return Err(Error::internal_error("Operation cancelled", None));
            }
            result = async {
                let client = Client::new();
                let req = Request::builder()
                    .method("GET")
                    .uri(format!(
                        "http://localhost:8000/api/w/admins/scripts/get/p/{}",
                        path
                    ))
                    .header(
                        "Authorization",
                        format!(
                            "Bearer {}",
                            String::from("zfg8ZyUDwf2sGwNUw1aEIR1gqfY1ywZ9")
                        ),
                    )
                    .body(Body::empty())
                    .map_err(|e| Error::internal_error(e.to_string(), None))?;
                let response = client
                    .request(req)
                    .await
                    .map_err(|e| Error::internal_error(e.to_string(), None))?;
                let body = hyper::body::to_bytes(response.into_body())
                    .await
                    .map_err(|e| Error::internal_error(e.to_string(), None))?;

                let response_data: ScriptSchemaResponse = serde_json::from_slice(&body).map_err(|e| {
                    Error::internal_error(format!("Failed to parse JSON response: {}", e), None)
                })?;

                let schema_str = serde_json::to_string_pretty(&response_data.schema).map_err(|e| {
                    Error::internal_error(format!("Failed to serialize schema field: {}", e), None)
                })?;

                Ok(CallToolResult::success(vec![Content::text(schema_str)]))
            } => { result }
        }
    }

    async fn run_script(
        &self,
        context: RequestContext<RoleServer>,
        script: String,
        args: String,
    ) -> Result<CallToolResult, Error> {
        tracing::info!(
            "run_script called via manual handler. Script: {}, Context ID: {:?}",
            script,
            context.id
        );
        let ct = context.ct;

        tokio::select! {
             _ = ct.cancelled() => {
                tracing::info!("run_script cancelled.");
                return Err(Error::internal_error("Operation cancelled", None));
            }
            result = async {
                let client = Client::new();
                let req = Request::builder()
                    .method("POST")
                    .uri(format!("http://localhost:8000/api/w/admins/jobs/run_wait_result/p/{}?skip_preprocessor=true", script))
                    .header(
                        "Authorization",
                        format!(
                            "Bearer {}",
                            String::from("zfg8ZyUDwf2sGwNUw1aEIR1gqfY1ywZ9")
                        ),
                    )
                    .header("Content-Type", "application/json")
                    .body(Body::from(args))
                    .map_err(|e| Error::internal_error(e.to_string(), None))?;
                let response = client
                    .request(req)
                    .await
                    .map_err(|e| Error::internal_error(e.to_string(), None))?;
                let body = hyper::body::to_bytes(response.into_body())
                    .await
                    .map_err(|e| Error::internal_error(e.to_string(), None))?;

                Ok(CallToolResult::success(vec![Content::text(
                    String::from_utf8_lossy(&body).into_owned(),
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
        let tools = vec![
            Tool {
                name: "get_scripts".into(),
                description: Some("Get list of scripts".into()),
                input_schema: rmcp::handler::server::tool::cached_schema_for_type::<EmptyObject>(),
                annotations: None,
            },
            Tool {
                name: "get_script_schema_by_path".into(),
                description: Some("Get script schema by path".into()),
                input_schema: rmcp::handler::server::tool::cached_schema_for_type::<
                    GetScriptSchemaByPathParams,
                >(),
                annotations: None,
            },
            Tool {
                name: "run_script".into(),
                description: Some("Run a script by path name".into()),
                input_schema: rmcp::handler::server::tool::cached_schema_for_type::<RunScriptParams>(
                ),
                annotations: None,
            },
        ];

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
