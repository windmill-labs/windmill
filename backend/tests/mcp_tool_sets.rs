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
