//! MCP Server implementation
//!
//! Contains the MCP server setup and routing. The core MCP protocol handlers
//! are implemented in windmill-mcp crate using the WindmillBackend.

use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use windmill_common::db::UserDB;
use windmill_common::DB;
use windmill_mcp::server::{
    LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use windmill_mcp::WorkspaceId;

use super::auto_generated_endpoints::{all_tools, EndpointTool};
use super::backend_impl::WindmillBackend;

use axum::{
    extract::Path, http::Request, middleware::Next, response::Response, routing::get, Json, Router,
};
use windmill_common::error::JsonResult;

/// Extract workspace ID from path and store it in request extensions
pub async fn extract_and_store_workspace_id(
    Path(params): Path<String>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let workspace_id = params;
    request.extensions_mut().insert(WorkspaceId(workspace_id));
    next.run(request).await
}

/// Setup the MCP server with HTTP transport
pub async fn setup_mcp_server(db: DB, user_db: UserDB) -> anyhow::Result<(Router, CancellationToken)> {
    let cancellation_token = CancellationToken::new();
    let session_manager = Arc::new(LocalSessionManager::default());
    let service_config = StreamableHttpServerConfig {
        sse_keep_alive: Some(Duration::from_secs(15)),
        stateful_mode: false,
        cancellation_token: cancellation_token.clone(),
    };

    let backend = WindmillBackend::new(db, user_db);

    let service = StreamableHttpService::new(
        move || Ok(windmill_mcp::server::Runner::new(backend.clone())),
        session_manager.clone(),
        service_config,
    );

    let router = axum::Router::new().nest_service("/", service);
    Ok((router, cancellation_token))
}

/// HTTP handler to list MCP tools as JSON
async fn list_mcp_tools_handler() -> JsonResult<Vec<EndpointTool>> {
    let endpoint_tools = all_tools();
    Ok(Json(endpoint_tools))
}

/// Creates a router service for listing MCP tools
pub fn list_tools_service() -> Router {
    Router::new().route("/", get(list_mcp_tools_handler))
}
