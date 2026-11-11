//! A token scoped to a selection of scripts lists no per-runnable tool its role cannot run,
//! and calling one stays refused.
#![cfg(feature = "mcp")]

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

async fn mcp(port: u16, token: &str, method: &str, params: Value) -> anyhow::Result<Value> {
    let body = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/mcp/w/test-workspace/mcp"
        ))
        .bearer_auth(token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-06-18")
        .json(&json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}))
        .send()
        .await?
        .text()
        .await?;
    let data = body
        .lines()
        .find_map(|l| l.strip_prefix("data: "))
        .unwrap_or(&body);
    Ok(serde_json::from_str(data)?)
}

#[sqlx::test(fixtures("base"))]
async fn test_view_only_role_lists_no_runnable_tools(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    sqlx::raw_sql(
        "INSERT INTO script (workspace_id, created_by, content, schema, summary, description, path, hash, language, lock)
         VALUES ('test-workspace', 'test-user-3', 'export function main() { return 1 }', '{}', '', '', 'u/test-user-3/hello', 7788201, 'bun', '');
         INSERT INTO token (token_hash, token_prefix, email, label, super_admin, scopes, workspace_id)
         VALUES (encode(sha256('SCOPED_MCP_TOKEN'::bytea), 'hex'), 'SCOPED_MCP', 'test3@windmill.dev', 'mcp', false,
                 ARRAY['mcp:scripts:u/test-user-3/hello'], 'test-workspace'),
                (encode(sha256('ADMIN_MCP_TOKEN'::bytea), 'hex'), 'ADMIN_MCP_', 'test@windmill.dev', 'mcp', false,
                 ARRAY['mcp:scripts:u/test-user-3/hello'], 'test-workspace');
         UPDATE workspace_settings SET role_overrides = '{\"developer\": {\"capabilities\": {\"run\": false}}}'
         WHERE workspace_id = 'test-workspace';",
    )
    .execute(&db)
    .await?;
    windmill_common::roles::invalidate_workspace_roles_cache("test-workspace");

    let server = ApiServer::start_mcp(db.clone()).await?;
    let port = server.addr.port();

    let admin = mcp(port, "ADMIN_MCP_TOKEN", "tools/list", json!({})).await?;
    assert!(
        admin["result"]["tools"]
            .as_array()
            .is_some_and(|t| t.iter().any(|t| t["name"] == "hello")),
        "a role that can run must still list the tool: {admin}"
    );

    let list = mcp(port, "SCOPED_MCP_TOKEN", "tools/list", json!({})).await?;
    let tools = list["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("no tools in {list}"));
    assert!(
        tools.is_empty(),
        "view-only role must list no run tool: {list}"
    );

    let call = mcp(
        port,
        "SCOPED_MCP_TOKEN",
        "tools/call",
        json!({"name": "hello", "arguments": {}}),
    )
    .await?;
    let msg = call["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("Your role cannot run"),
        "call must stay refused: {call}"
    );
    Ok(())
}
