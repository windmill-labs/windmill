//! The Windmill MCP server's toolset: an AI session's tools and guidance for an external
//! client. `/api/mcp` serves it to tokens with `mcp:all`, and the tools server (`Runner`)
//! to tokens scoped to a selection of scripts and flows.
//!
//! Every tool takes the workspace it acts in, whether the token reaches one workspace or
//! several, so a client's tool schemas do not change with the token it signed in with.

use crate::common::scope::{parse_mcp_scopes, McpScopeConfig};
use crate::common::types::WorkspaceInfo;
use crate::server::backend::{McpAuth, McpBackend, SessionAccess};
use crate::server::endpoints::{list_workspaces_tool, with_workspace_id_arg};
use crate::server::runner::{
    truncate_tool_result, workspace_not_granted, McpContext, McpMode, Runner, LIST_CACHE_SCOPE,
    LIST_TTL_MS, SUPPORTED_PROTOCOL_VERSIONS,
};
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ClientCapabilities, ContentBlock,
    DiscoverResult, Implementation, InitializeRequestParams, InitializeResult, ListResourcesResult,
    ListToolsResult, MetaObject, PaginatedRequestParams, ProtocolVersion,
    ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult, Resource,
    ResourceContents, ServerCapabilities, ServerInfo, Tool, ToolAnnotations,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ErrorData;
use serde_json::Value;
use std::borrow::Cow;
use std::sync::Arc;

/// `mcp:all`, and no scope from outside `mcp:` that confines the token to some routes
/// or paths (`variables:read:f/x/*`): the tools call the API without a route scope, so
/// that confinement would be lost here. The job tag fence is carried through instead.
fn admits(scopes: &[String], all: bool) -> bool {
    all && scopes
        .iter()
        .all(|s| s.starts_with("mcp:") || s.starts_with("if_jobs:filter_tags:"))
}

fn access<A: McpAuth>(auth: &A, scope: &McpScopeConfig) -> SessionAccess {
    if auth.read_only() || scope.read_only {
        SessionAccess::ReadOnly
    } else if scope.read_run {
        SessionAccess::ReadAndRun
    } else {
        SessionAccess::Full
    }
}

const GET_WORKSPACE_CONTEXT: &str = "get_workspace_context";

/// The MIME type of an MCP Apps view (the `io.modelcontextprotocol/ui` extension).
const MCP_APP_MIME_TYPE: &str = "text/html;profile=mcp-app";
const MCP_APPS_EXTENSION: &str = "io.modelcontextprotocol/ui";

/// A tool that only shows a card does nothing for a client that declares it draws none.
/// Only a request's own capabilities count: a legacy client's are never seen (the
/// transport keeps no session), and it keeps the tools.
fn declines_cards(capabilities: Option<&ClientCapabilities>) -> bool {
    capabilities.is_some_and(|caps| {
        !caps
            .extensions
            .as_ref()
            .is_some_and(|e| e.contains_key(MCP_APPS_EXTENSION))
    })
}

/// The views load nothing from outside their own page and reach Windmill only through
/// tools, so they declare no domain; a host reviewing the declared policy holds them to it.
fn mcp_app_meta() -> MetaObject {
    let ui = serde_json::json!({
        "prefersBorder": true,
        "csp": { "connectDomains": [], "resourceDomains": [], "frameDomains": [] },
        // The card's copy controls; a host that grants nothing leaves its fallback.
        "permissions": { "clipboardWrite": {} },
    });
    MetaObject(serde_json::Map::from_iter([("ui".to_string(), ui)]))
}

/// Workspaces named in the instructions before the rest are left to `list_workspaces`.
const MAX_LISTED_WORKSPACES: usize = 40;

fn get_workspace_context_tool() -> Tool {
    let schema = serde_json::json!({ "type": "object", "properties": {} });
    with_workspace_id_arg(
        Tool::new(
            Cow::Borrowed(GET_WORKSPACE_CONTEXT),
            "Your username in a workspace, the folders you can write to there, and the \
             workspace's skills and instructions. Call it before your first action in a \
             workspace.",
            Arc::new(schema.as_object().unwrap().clone()),
        )
        .with_annotations(ToolAnnotations::new().read_only(true)),
    )
}

/// A request's caller, and the workspaces its token reaches.
struct Caller<A> {
    auth: A,
    mode: McpMode,
    scope: McpScopeConfig,
    /// The scheme and host the client reached, for links to Windmill pages.
    origin: Option<String>,
}

/// `x-forwarded-*` first, since a proxy in front rewrites the host the request names.
fn request_origin(headers: &http::HeaderMap) -> Option<String> {
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(str::trim)
            .filter(|v| !v.is_empty())
    };
    let host = header("x-forwarded-host").or_else(|| header("host"))?;
    let scheme = header("x-forwarded-proto").unwrap_or("http");
    Some(format!("{scheme}://{host}"))
}

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

    /// This server writes drafts and deploys, so it takes the scope that already reaches
    /// the create/update endpoint tools and nothing narrower: a token scoped to a few
    /// scripts was never consented for authoring.
    fn caller(context: &RequestContext<RoleServer>) -> Result<Caller<B::Auth>, ErrorData> {
        let McpContext { auth, mode, headers, .. } = Runner::<B>::extract_context(context)?;
        let scope = parse_mcp_scopes(auth.scopes().unwrap_or(&[]))
            .map_err(|e| ErrorData::internal_error(e, None))?;
        if !admits(auth.scopes().unwrap_or(&[]), scope.all) {
            return Err(ErrorData::invalid_request(
                "This server needs an `mcp:all` token with no other route or path scope",
                None,
            ));
        }
        Ok(Caller { auth, mode, scope, origin: request_origin(&headers) })
    }

    async fn workspaces(&self, caller: &Caller<B::Auth>) -> Result<Vec<WorkspaceInfo>, ErrorData> {
        let mut workspaces = self
            .backend
            .list_accessible_workspaces(&caller.auth)
            .await?;
        match &caller.mode {
            McpMode::Single(bound) => workspaces.retain(|w| &w.id == bound),
            McpMode::Multi(_) => workspaces.retain(|w| caller.scope.allows_workspace(&w.id)),
        }
        Ok(workspaces)
    }

    /// The caller as they are in `workspace_id`, if the token reaches it.
    async fn auth_in(
        &self,
        caller: &Caller<B::Auth>,
        workspace_id: &str,
    ) -> Result<B::Auth, ErrorData> {
        match &caller.mode {
            McpMode::Single(bound) if bound == workspace_id => Ok(caller.auth.clone()),
            McpMode::Multi(token) if caller.scope.allows_workspace(workspace_id) => {
                self.backend
                    .resolve_workspace_auth(token, workspace_id)
                    .await
            }
            _ => Err(workspace_not_granted(workspace_id)),
        }
    }

    async fn info_for(
        &self,
        context: &RequestContext<RoleServer>,
    ) -> Result<ServerInfo, ErrorData> {
        let caller = Self::caller(context)?;
        let workspaces = self.workspaces(&caller).await?;
        let instructions = match workspaces.as_slice() {
            [only] => {
                let auth = self.auth_in(&caller, &only.id).await?;
                format!(
                    "Every tool takes a `workspace_id`: pass `{}`, the workspace this connection \
                     works in.\n\n{}",
                    only.id,
                    self.backend.session_instructions(&auth, &only.id).await?
                )
            }
            _ => {
                let mut listed = workspaces
                    .iter()
                    .take(MAX_LISTED_WORKSPACES)
                    .map(|w| format!("`{}` ({})", w.id, w.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                if workspaces.len() > MAX_LISTED_WORKSPACES {
                    listed.push_str(&format!(
                        " (+{} more)",
                        workspaces.len() - MAX_LISTED_WORKSPACES
                    ));
                }
                format!(
                    "This connection works across several Windmill workspaces: {listed}. Every \
                     tool takes a `workspace_id` naming the one it acts in; list_workspaces lists \
                     them. Before your first action in a workspace, call get_workspace_context \
                     with its id: it gives your username there (the `<username>` in the paths \
                     below), the folders you can write to, and the workspace's skills and \
                     instructions. When the user has not said which workspace, ask rather than \
                     guess, and name the workspace whenever you change something.\n\n{}",
                    self.backend.generic_session_instructions()
                )
            }
        };
        Ok(self.get_info().with_instructions(instructions))
    }
}

impl<B: McpBackend> ServerHandler for SessionRunner<B> {
    /// Without instructions: those are built per caller, in `initialize` and `discover`.
    fn get_info(&self) -> ServerInfo {
        let server_info = Implementation::new("windmill", env!("CARGO_PKG_VERSION"))
            .with_title("Windmill")
            .with_website_url("https://windmill.dev");
        InitializeResult::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
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
        let caller = Self::caller(&context)?;
        let mut tools = vec![list_workspaces_tool()];
        // A bound token's one workspace is described in the instructions already.
        if let McpMode::Multi(_) = caller.mode {
            tools.push(get_workspace_context_tool());
        }
        tools.extend(
            self.backend
                .session_tools(access(&caller.auth, &caller.scope))
                .into_iter()
                .map(with_workspace_id_arg),
        );
        if declines_cards(context.meta.client_capabilities().as_ref()) {
            tools.retain(|tool| {
                !tool.meta.as_ref().is_some_and(|m| {
                    m.0.get("ui")
                        .is_some_and(|ui| ui.get("resourceUri").is_some())
                })
            });
        }
        // Without these a `2026-07-28` client rejects the whole listing.
        Ok(ListToolsResult::with_all_items(tools)
            .with_ttl_ms(LIST_TTL_MS)
            .with_cache_scope(LIST_CACHE_SCOPE))
    }

    /// The MCP Apps views a client draws tool results in. Static pages: what they show
    /// comes from tool results, which every request authorizes on its own.
    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let views = self
            .backend
            .session_views()
            .iter()
            .map(|(uri, _)| Resource::new(*uri, *uri).with_mime_type(MCP_APP_MIME_TYPE))
            .collect();
        Ok(ListResourcesResult::with_all_items(views)
            .with_ttl_ms(LIST_TTL_MS)
            .with_cache_scope(LIST_CACHE_SCOPE))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let Some((uri, html)) = self
            .backend
            .session_views()
            .iter()
            .find(|(uri, _)| *uri == request.uri)
        else {
            return Err(ErrorData::resource_not_found(
                format!("No resource at {}", request.uri),
                None,
            ));
        };
        Ok(
            ReadResourceResult::new(vec![ResourceContents::text(*html, *uri)
                .with_mime_type(MCP_APP_MIME_TYPE)
                .with_meta(mcp_app_meta())])
            .into(),
        )
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let caller = Self::caller(&context)?;
        if request.name == "list_workspaces" {
            let workspaces = self.workspaces(&caller).await?;
            return Ok(CallToolResult::success(vec![ContentBlock::text(
                serde_json::to_string_pretty(&workspaces).unwrap_or_default(),
            )])
            .into());
        }
        let mut args = request.arguments.unwrap_or_default();
        let workspace_id =
            match args.remove("workspace_id") {
                Some(Value::String(w)) if !w.is_empty() => w,
                _ => return Err(ErrorData::invalid_params(
                    "Missing required `workspace_id` argument. Call list_workspaces to see the \
                     workspaces you can act in.",
                    None,
                )),
            };
        let auth = self.auth_in(&caller, &workspace_id).await?;
        if request.name == GET_WORKSPACE_CONTEXT {
            let text = self.backend.workspace_context(&auth, &workspace_id).await;
            return Ok(CallToolResult::success(vec![ContentBlock::text(text)]).into());
        }
        let mut result = self
            .backend
            .call_session_tool(
                &auth,
                &workspace_id,
                request.name.as_ref(),
                Value::Object(args),
                access(&auth, &caller.scope),
                caller.origin.as_deref(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_client_declaring_no_cards_loses_the_card_tools() {
        let mut caps: ClientCapabilities = serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(declines_cards(Some(&caps)));
        assert!(!declines_cards(None));
        caps.extensions = Some([(MCP_APPS_EXTENSION.to_string(), Default::default())].into());
        assert!(!declines_cards(Some(&caps)));
    }

    #[test]
    fn origin_prefers_forwarded_headers() {
        let mut headers = http::HeaderMap::new();
        headers.insert("host", "backend:8000".parse().unwrap());
        assert_eq!(
            request_origin(&headers).as_deref(),
            Some("http://backend:8000")
        );
        headers.insert("x-forwarded-host", "wm.example.com, proxy".parse().unwrap());
        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        assert_eq!(
            request_origin(&headers).as_deref(),
            Some("https://wm.example.com")
        );
    }

    #[test]
    fn only_an_unnarrowed_mcp_all_token_is_admitted() {
        let admitted = |scopes: &[&str]| {
            let scopes: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();
            let all = parse_mcp_scopes(&scopes).is_ok_and(|c| c.all);
            admits(&scopes, all)
        };
        assert!(admitted(&["mcp:all"]));
        assert!(admitted(&["mcp:all", "if_jobs:filter_tags:gpu"]));
        assert!(admitted(&["mcp:all", "mcp:workspaces:dev"]));
        assert!(!admitted(&["mcp:all", "variables:read:u/a/safe/*"]));
        assert!(!admitted(&["mcp:all:f/x/*"]));
        assert!(!admitted(&["mcp:favorites"]));
        assert!(!admitted(&[]));
    }
}
