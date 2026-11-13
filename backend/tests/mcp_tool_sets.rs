//! A named tool set narrows and never grants: a member signed in to it gets the set's
//! runnables they can see, the set is read on every request, and its URL refuses any other
//! token so the client signs in to the set.
#![cfg(feature = "mcp")]

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

async fn list_tools(port: u16, path: &str, token: &str) -> anyhow::Result<(u16, Value)> {
    let response = reqwest::Client::new()
        .post(format!("http://localhost:{port}{path}"))
        .bearer_auth(token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-06-18")
        .json(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}))
        .send()
        .await?;
    let status = response.status().as_u16();
    let body = response.text().await?;
    let data = body
        .lines()
        .find_map(|l| l.strip_prefix("data: "))
        .unwrap_or(&body);
    Ok((status, serde_json::from_str(data).unwrap_or(Value::Null)))
}

fn names(list: &Value) -> Vec<String> {
    list["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("no tools in {list}"))
        .iter()
        .map(|t| t["name"].as_str().unwrap_or_default().to_string())
        .collect()
}

#[sqlx::test(fixtures("base"))]
async fn tool_set_narrows_to_what_the_member_can_see(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    sqlx::raw_sql(
        r#"
        INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms) VALUES
            ('test-workspace', 'ops', 'ops', ARRAY['u/test-user'], '{"u/test-user-3": false}'),
            ('test-workspace', 'internal', 'internal', ARRAY['u/test-user'], '{}');
        INSERT INTO script (workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES
            ('test-workspace', 'test-user', 'export function main() { return 1 }', '{}', '', '', 'f/ops/check_order', 7788301, 'bun', ''),
            ('test-workspace', 'test-user', 'export function main() { return 1 }', '{}', '', '', 'f/internal/secret', 7788302, 'bun', '');
        UPDATE workspace_settings
           SET mcp_tool_sets = '{"support": {"scripts": ["f/ops/*", "f/internal/secret"]}}'
         WHERE workspace_id = 'test-workspace';
        INSERT INTO token (token_hash, token_prefix, email, label, super_admin, scopes, workspace_id) VALUES
            (encode(sha256('SET_TOKEN'::bytea), 'hex'), 'SET_TOKEN_', 'test3@windmill.dev', 'mcp', false,
             ARRAY['mcp:set:support', 'mcp:runnables_as_tools'], 'test-workspace'),
            (encode(sha256('FULL_TOKEN'::bytea), 'hex'), 'FULL_TOKEN', 'test3@windmill.dev', 'mcp', false,
             ARRAY['mcp:all', 'mcp:session'], 'test-workspace');
        "#,
    )
    .execute(&db)
    .await?;

    let server = ApiServer::start_mcp(db.clone()).await?;
    let port = server.addr.port();
    const SET_URL: &str = "/api/mcp/w/test-workspace/s/support/mcp";

    let (_, list) = list_tools(port, SET_URL, "SET_TOKEN").await?;
    assert_eq!(names(&list), ["check_order"], "{list}");

    let (status, _) = list_tools(port, SET_URL, "FULL_TOKEN").await?;
    assert_eq!(status, 401, "a token not signed in to the set");

    sqlx::query(
        "UPDATE workspace_settings SET mcp_tool_sets = NULL WHERE workspace_id = 'test-workspace'",
    )
    .execute(&db)
    .await?;
    let (_, list) = list_tools(port, SET_URL, "SET_TOKEN").await?;
    assert!(
        names(&list).is_empty(),
        "a deleted set leaves no tool: {list}"
    );
    Ok(())
}

async fn call(
    port: u16,
    path: &str,
    token: &str,
    name: &str,
    args: Value,
) -> anyhow::Result<String> {
    let body = reqwest::Client::new()
        .post(format!("http://localhost:{port}{path}"))
        .bearer_auth(token)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-06-18")
        .json(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                      "params": {"name": name, "arguments": args}}))
        .send()
        .await?
        .text()
        .await?;
    Ok(body)
}

/// The set's run tools read only the set's runs, though the member may see others.
#[sqlx::test(fixtures("base"))]
async fn run_history_stays_in_the_set(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    sqlx::raw_sql(
        r#"
        INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms) VALUES
            ('test-workspace', 'ops', 'ops', ARRAY['u/test-user'], '{"u/test-user-3": false}'),
            ('test-workspace', 'internal', 'internal', ARRAY['u/test-user'], '{"u/test-user-3": false}');
        INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, kind, runnable_path) VALUES
            ('11111111-0000-0000-0000-000000000001', 'test-workspace', 'test-user', 'u/test-user', 'script', 'f/ops/check_order'),
            ('11111111-0000-0000-0000-000000000002', 'test-workspace', 'test-user', 'u/test-user', 'script', 'f/internal/secret');
        INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status) VALUES
            ('11111111-0000-0000-0000-000000000001', 'test-workspace', 1, 'success'),
            ('11111111-0000-0000-0000-000000000002', 'test-workspace', 1, 'success');
        UPDATE workspace_settings
           SET mcp_tool_sets = '{"support": {"scripts": ["f/ops/*"], "run_history": true}}'
         WHERE workspace_id = 'test-workspace';
        INSERT INTO token (token_hash, token_prefix, email, label, super_admin, scopes, workspace_id) VALUES
            (encode(sha256('RUN_SET_TOKEN'::bytea), 'hex'), 'RUN_SET_TO', 'test3@windmill.dev', 'mcp', false,
             ARRAY['mcp:set:support', 'mcp:runnables_as_tools'], 'test-workspace');
        "#,
    )
    .execute(&db)
    .await?;

    let server = ApiServer::start_mcp(db.clone()).await?;
    let port = server.addr.port();
    const SET_URL: &str = "/api/mcp/w/test-workspace/s/support/mcp";

    let listed = call(port, SET_URL, "RUN_SET_TOKEN", "list_runs", json!({})).await?;
    assert!(listed.contains("f/ops/check_order"), "{listed}");
    assert!(!listed.contains("f/internal/secret"), "{listed}");

    let outside = call(
        port,
        SET_URL,
        "RUN_SET_TOKEN",
        "get_run",
        json!({"id": "11111111-0000-0000-0000-000000000002"}),
    )
    .await?;
    assert!(
        outside.contains("not in this connection's tool set"),
        "{outside}"
    );
    Ok(())
}
