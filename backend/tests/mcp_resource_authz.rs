//! Regression test for the MCP resource-authorization bypass
//! (WIN-2041, GHSA-7qg3-pr4g-cq5x).
//!
//! Invariant: the AI Agent worker (`load_mcp_tools`) must load an MCP resource
//! only when the job identity is allowed to read it — the same
//! `resources:read:{path}` + RLS gate the regular MCP tools API (`get_mcp_tools`)
//! enforces. It loads the resource through the job's permissioned client
//! (`AuthedClient::get_resource_value`, the `resources/get_value` path), not the
//! raw DB pool. Otherwise a low-privileged user who can edit a flow could make
//! the agent load and use an MCP resource (URL + inline headers + token) they are
//! not allowed to read: a confused-deputy bypass.
//!
//! This test pins, against the `mcp_resource_authz` fixture (an MCP resource in
//! a folder only the admin can read):
//!   - a plain developer (test-user-3) is DENIED loading the private resource,
//!     before any MCP connection is attempted, and the resource never leaks;
//!   - an admin (test-user) clears the gate, the resource resolves, and loading
//!     only fails later at the connect step — proving no over-blocking.
#![cfg(feature = "mcp")]

use sqlx::{Pool, Postgres};
use windmill_common::client::AuthedClient;
use windmill_test_utils::*;
use windmill_worker::{load_mcp_tools, McpResourceConfig};

fn config() -> Vec<McpResourceConfig> {
    vec![McpResourceConfig {
        resource_path: "$res:f/private/admin_mcp".to_string(),
        include_tools: None,
        exclude_tools: None,
    }]
}

// The Ok variant ((HashMap<_, Arc<McpClient>>, Vec<Tool>)) does not implement
// Debug, so `expect_err` is unavailable — extract the error explicitly.
async fn load_err(db: &Pool<Postgres>, client: &AuthedClient, ctx: &str) -> String {
    match load_mcp_tools(db, "test-workspace", config(), client).await {
        Ok(_) => panic!("{ctx}"),
        Err(e) => e.to_string(),
    }
}

#[sqlx::test(fixtures("base", "mcp_resource_authz"))]
async fn test_mcp_resource_not_loaded_without_authorization(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let base_internal_url = format!("http://localhost:{}", server.addr.port());

    // ---- CORE REGRESSION: the developer cannot read the private MCP resource,
    //      so the worker must refuse to load it — before any MCP connection is
    //      attempted, and without the resource ever leaking.
    let dev_client = AuthedClient::new(
        base_internal_url.clone(),
        "test-workspace".to_string(),
        "SECRET_TOKEN_3".to_string(),
        None,
    );
    let msg = load_err(
        &db,
        &dev_client,
        "developer must be denied loading a resource they can't read",
    )
    .await;
    assert!(
        msg.contains("don't have access"),
        "denial should come from the resource-RLS gate, not a connection error: {msg}"
    );
    // An unauthorized caller must be blocked before MCP client creation, so no
    // resource material (URL, headers, token) is ever loaded for them.
    assert!(
        !msg.contains("Failed to create MCP client"),
        "developer must be blocked before the connection step (would mean the resource was loaded): {msg}"
    );

    // ---- NO OVER-BLOCKING: an admin clears the resource-RLS gate, so the
    //      resource resolves and loading only fails later at the connect step.
    let admin_client = AuthedClient::new(
        base_internal_url,
        "test-workspace".to_string(),
        "SECRET_TOKEN".to_string(),
        None,
    );
    let msg = load_err(
        &db,
        &admin_client,
        "the fixture URL is non-resolvable, so the connect step must fail",
    )
    .await;
    assert!(
        !msg.contains("don't have access"),
        "admin must clear the resource-RLS gate, not be denied: {msg}"
    );
    assert!(
        msg.contains("Failed to create MCP client"),
        "admin should resolve the resource and only fail at the connect step: {msg}"
    );

    Ok(())
}
