#[cfg(feature = "mcp")]
use axum::routing::get;
use axum::Router;

/// Wraps the subcrate's workspaced_service with the mcp_tools route
/// that depends on windmill-api internals.
pub fn workspaced_service() -> Router {
    let router = windmill_store::resources::workspaced_service();

    #[cfg(feature = "mcp")]
    use crate::mcp_tools::get_mcp_tools;
    #[cfg(feature = "mcp")]
    let router = router.route("/mcp_tools/*path", get(get_mcp_tools));

    router
}
