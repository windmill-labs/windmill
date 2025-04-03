use rmcp::{
    Error as McpError, RoleServer, ServerHandler, const_string, model::*, schemars,
    service::RequestContext, tool,
};
use serde_json::json;
use windmill_common::initial_connection;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct StructRequest {
    pub a: i32,
    pub b: i32,
}

#[derive(Clone)]
pub struct Runner {}
#[tool(tool_box)]
impl Runner {
    pub fn new() -> Self {
        Self {}
    }

    fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
    }

    #[tool(description = "Get list of scripts")]
    async fn get_scripts(&self) -> Result<CallToolResult, McpError> {
        let db = initial_connection().await.unwrap();
        let scripts = sqlx::query!("SELECT path FROM script").fetch_all(&db).await;
        let scripts = scripts.unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(
            scripts
                .iter()
                .map(|s| s.path.clone())
                .collect::<Vec<String>>()
                .join("\n"),
        )]))
    }

    #[tool(description = "Run a script by path name")]
    async fn run_script(
        &self,
        #[tool(param)]
        #[schemars(description = "The script path to run")]
        script: String,
    ) -> Result<CallToolResult, McpError> {
        let db = initial_connection().await.unwrap();
        let script = sqlx::query!("SELECT path FROM script WHERE path = $1", script)
            .fetch_one(&db)
            .await;
        let script = match script {
            Ok(s) => s,
            Err(_) => {
                return Ok(CallToolResult::success(vec![Content::text(
                    "Script not found".to_string(),
                )]));
            }
        };
        Ok(CallToolResult::success(vec![Content::text(script.path)]))
    }

    #[tool(description = "Calculate the sum of two numbers")]
    fn sum(
        &self,
        #[tool(aggr)] StructRequest { a, b }: StructRequest,
    ) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(
            (a + b).to_string(),
        )]))
    }
}

const_string!(Echo = "echo");
#[tool(tool_box)]
impl ServerHandler for Runner {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides a runner tool that can run scripts. Use 'get_scripts' to get the list of scripts.".to_string()),
        }
    }

    async fn list_resources(
        &self,
        _request: PaginatedRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                self._create_resource_text("str:////Users/to/some/path/", "cwd"),
                self._create_resource_text("memo://insights", "memo-name"),
            ],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        match uri.as_str() {
            "str:////Users/to/some/path/" => {
                let cwd = "/Users/to/some/path/";
                Ok(ReadResourceResult { contents: vec![ResourceContents::text(cwd, uri)] })
            }
            "memo://insights" => {
                let memo = "Business Intelligence Memo\n\nAnalysis has revealed 5 key insights ...";
                Ok(ReadResourceResult { contents: vec![ResourceContents::text(memo, uri)] })
            }
            _ => Err(McpError::resource_not_found(
                "resource_not_found",
                Some(json!({
                    "uri": uri
                })),
            )),
        }
    }

    async fn list_prompts(
        &self,
        _request: PaginatedRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            next_cursor: None,
            prompts: vec![Prompt::new(
                "example_prompt",
                Some("This is an example prompt that takes one required argument, message"),
                Some(vec![PromptArgument {
                    name: "message".to_string(),
                    description: Some("A message to put in the prompt".to_string()),
                    required: Some(true),
                }]),
            )],
        })
    }

    async fn get_prompt(
        &self,
        GetPromptRequestParam { name, arguments }: GetPromptRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        match name.as_str() {
            "example_prompt" => {
                let message = arguments
                    .and_then(|json| json.get("message")?.as_str().map(|s| s.to_string()))
                    .ok_or_else(|| {
                        McpError::invalid_params("No message provided to example_prompt", None)
                    })?;

                let prompt =
                    format!("This is an example prompt with your message here: '{message}'");
                Ok(GetPromptResult {
                    description: None,
                    messages: vec![PromptMessage {
                        role: PromptMessageRole::User,
                        content: PromptMessageContent::text(prompt),
                    }],
                })
            }
            _ => Err(McpError::invalid_params("prompt not found", None)),
        }
    }

    async fn list_resource_templates(
        &self,
        _request: PaginatedRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult { next_cursor: None, resource_templates: Vec::new() })
    }
}
