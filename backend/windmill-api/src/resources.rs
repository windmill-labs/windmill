#[cfg(feature = "mcp")]
use axum::routing::{get, post};
use axum::Router;

/// Wraps the subcrate's workspaced_service with the mcp_tools routes
/// that depend on windmill-api internals.
pub fn workspaced_service() -> Router {
    let router = windmill_store::resources::workspaced_service();

    #[cfg(feature = "mcp")]
    use crate::mcp_tools::{call_mcp_tool, get_mcp_tools};
    #[cfg(feature = "mcp")]
    let router = router
        .route("/mcp_tools/{*path}", get(get_mcp_tools))
        .route("/mcp_call_tool/{*path}", post(call_mcp_tool));

    router
}
