use hyper::Client;
use hyper::Request;
use hyper::body::Body;
use rmcp::{
    Error as McpError, RoleServer, ServerHandler, model::*, schemars, service::RequestContext, tool,
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone)]
pub struct Runner {
    token: Option<String>,
}

#[derive(Deserialize)]
struct ScriptSchemaResponse {
    schema: Value,
}

#[tool(tool_box)]
impl Runner {
    pub fn new() -> Self {
        Self { token: None }
    }

    pub fn update_user_token(&mut self, token: String) -> Result<CallToolResult, McpError> {
        tracing::info!("Updating user token: {}", token);
        self.token = Some(token.clone());
        Ok(CallToolResult::success(vec![Content::text(token)]))
    }

    fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        RawResource::new(uri, name.to_string()).no_annotation()
    }

    #[tool(description = "Get list of scripts")]
    async fn get_scripts(&self) -> Result<CallToolResult, McpError> {
        let client = Client::new();
        let req = Request::builder()
            .method("GET")
            .uri("http://localhost:8000/api/w/admins/scripts/list")
            .header(
                "Authorization",
                format!("Bearer {}", self.token.as_ref().unwrap()),
            )
            .body(Body::empty())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let response = client
            .request(req)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let body = hyper::body::to_bytes(response.into_body())
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            String::from_utf8_lossy(&body).into_owned(),
        )]))
    }

    #[tool(description = "Get script schema by path")]
    async fn get_script_schema_by_path(
        &self,
        #[tool(param)]
        #[schemars(description = "The script path to get the schema for")]
        path: String,
    ) -> Result<CallToolResult, McpError> {
        let client = Client::new();
        let req = Request::builder()
            .method("GET")
            .uri(format!(
                "http://localhost:8000/api/w/admins/scripts/get/p/{}",
                path
            ))
            .header(
                "Authorization",
                format!("Bearer {}", self.token.as_ref().unwrap()),
            )
            .body(Body::empty())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let response = client
            .request(req)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let body = hyper::body::to_bytes(response.into_body())
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        // Deserialize the body into the defined struct
        let response_data: ScriptSchemaResponse = serde_json::from_slice(&body).map_err(|e| {
            McpError::internal_error(format!("Failed to parse JSON response: {}", e), None)
        })?;

        // Convert the schema field back to a string (pretty-printed)
        let schema_str = serde_json::to_string_pretty(&response_data.schema).map_err(|e| {
            McpError::internal_error(format!("Failed to serialize schema field: {}", e), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(
            schema_str, // Use the extracted and serialized schema string
        )]))
    }

    #[tool(description = "Run a script by path name")]
    async fn run_script(
        &self,
        #[tool(param)]
        #[schemars(description = "The script path to run")]
        script: String,
        #[tool(param)]
        #[schemars(description = "The script arguments")]
        args: String,
    ) -> Result<CallToolResult, McpError> {
        let client = Client::new();
        let req = Request::builder()
            .method("POST")
            .uri(format!("http://localhost:8000/api/w/admins/jobs/run_wait_result/p/{}?skip_preprocessor=true", script))
            .header(
                "Authorization",
                format!("Bearer {}", self.token.as_ref().unwrap()),
            )
            .body(Body::from(args))
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let response = client
            .request(req)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let body = hyper::body::to_bytes(response.into_body())
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            String::from_utf8_lossy(&body).into_owned(),
        )]))
    }
}

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
        Ok(ListResourcesResult { resources: vec![], next_cursor: None })
    }
}
