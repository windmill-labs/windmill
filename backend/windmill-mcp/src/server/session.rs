//! The "Windmill workspace" MCP server: an AI session's toolset and guidance for an
//! external client, served beside the tools server (`Runner`) on its own URL so a
//! client that only wants to run a script does not load builder guidance.

use crate::common::scope::parse_mcp_scopes;
use crate::server::backend::{McpAuth, McpBackend};
use crate::server::runner::{
    truncate_tool_result, McpContext, McpMode, Runner, LIST_CACHE_SCOPE, LIST_TTL_MS,
    SUPPORTED_PROTOCOL_VERSIONS,
};
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, ContentBlock, DiscoverResult, Implementation,
    InitializeRequestParams, InitializeResult, ListToolsResult, PaginatedRequestParams,
    ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ErrorData;
use serde_json::Value;
use std::borrow::Cow;
use std::sync::Arc;

pub struct SessionRunner<B: McpBackend> {
    backend: Arc<B>,
}

impl<B: McpBackend> Clone for SessionRunner<B> {
    fn clone(&self) -> Self {
        Self { backend: self.backend.clone() }
    }
}

impl<B: McpBackend> SessionRunner<B> {
    pub fn new(backend: B) -> Self {
        Self { backend: Arc::new(backend) }
    }

    /// The caller and its workspace. This server writes drafts and deploys, so it takes
    /// the scope that already reaches the create/update endpoint tools and nothing narrower:
    /// a token scoped to a few scripts was never consented for authoring.
    fn caller(context: &RequestContext<RoleServer>) -> Result<(B::Auth, String), ErrorData> {
        let McpContext { auth, mode, .. } = Runner::<B>::extract_context(context)?;
        let scope_config = parse_mcp_scopes(auth.scopes().unwrap_or(&[]))
            .map_err(|e| ErrorData::internal_error(e, None))?;
        if !scope_config.all {
            return Err(ErrorData::invalid_request(
                "This server needs a token with the `mcp:all` scope",
                None,
            ));
        }
        match mode {
            McpMode::Single(workspace_id) => Ok((auth, workspace_id)),
            McpMode::Multi(_) => Err(ErrorData::invalid_request(
                "This server needs a workspace in its URL",
                None,
            )),
        }
    }

    async fn info_for(
        &self,
        context: &RequestContext<RoleServer>,
    ) -> Result<ServerInfo, ErrorData> {
        let (auth, workspace_id) = Self::caller(context)?;
        let instructions = self
            .backend
            .session_instructions(&auth, &workspace_id)
            .await?;
        Ok(self.get_info().with_instructions(instructions))
    }
}

impl<B: McpBackend> ServerHandler for SessionRunner<B> {
    /// Without instructions: those are built per caller, in `initialize` and `discover`.
    fn get_info(&self) -> ServerInfo {
        let server_info = Implementation::new("windmill-workspace", env!("CARGO_PKG_VERSION"))
            .with_title("Windmill workspace")
            .with_website_url("https://windmill.dev");
        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(server_info)
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(SUPPORTED_PROTOCOL_VERSIONS)
    }

    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        context.peer.set_peer_info(request.clone());
        let mut info = self.info_for(&context).await?;
        if SUPPORTED_PROTOCOL_VERSIONS.contains(&request.protocol_version) {
            info.protocol_version = request.protocol_version;
        }
        Ok(info)
    }

    async fn discover(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<DiscoverResult, ErrorData> {
        Ok(DiscoverResult::from_server_info(
            SUPPORTED_PROTOCOL_VERSIONS.to_vec(),
            self.info_for(&context).await?,
        ))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let (auth, _) = Self::caller(&context)?;
        // Without these a `2026-07-28` client rejects the whole listing.
        Ok(
            ListToolsResult::with_all_items(self.backend.session_tools(auth.read_only()))
                .with_ttl_ms(LIST_TTL_MS)
                .with_cache_scope(LIST_CACHE_SCOPE),
        )
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let (auth, workspace_id) = Self::caller(&context)?;
        let args = request.arguments.map(Value::Object).unwrap_or(Value::Null);
        let mut result = self
            .backend
            .call_session_tool(
                &auth,
                &workspace_id,
                request.name.as_ref(),
                args,
                auth.read_only(),
            )
            .await?;
        for block in result.content.iter_mut() {
            if let Some(text) = block.as_text() {
                *block = ContentBlock::text(truncate_tool_result(text.text.clone()));
            }
        }
        Ok(result.into())
    }
}
