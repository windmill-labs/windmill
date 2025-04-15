use axum::Router;
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use tokio_util::sync::CancellationToken;

pub mod runner; // Make runner public

const BIND_ADDRESS: &str = "127.0.0.1:8008"; // This address is only used when running standalone

pub fn setup_mcp_server() -> anyhow::Result<(SseServer, Router)> {
    let config = SseServerConfig {
        // The bind address here is for the MCP server *if run standalone*.
        // It's ignored when the router is nested within another Axum server.
        bind: BIND_ADDRESS.parse()?,
        sse_path: "/sse".to_string(), // Relative path handled by SseServer's router
        post_path: "/message".to_string(), // Relative path handled by SseServer's router
        ct: CancellationToken::new(), // Independent cancellation for this MCP instance
        sse_keep_alive: None,
    };

    // SseServer::new conveniently returns the server instance and the router together.
    Ok(SseServer::new(config))
}
