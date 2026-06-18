use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn flow_url(port: u16, endpoint: &str, path: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/flows/{endpoint}/{path}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

async fn authed_get(port: u16, endpoint: &str, path: &str) -> reqwest::Response {
    authed(client().get(flow_url(port, endpoint, path)))
        .send()
        .await
        .unwrap()
}

fn new_flow(path: &str, summary: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": summary,
        "description": "",
        "value": {
            "modules": []
        },
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        }
    })
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_flow_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/flows");

    // --- create ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_flow("u/test-user/test_flow", "Test flow"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create: {}", resp.text().await?);

    // create second flow
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_flow("u/test-user/another_flow", "Another flow"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create another: {}", resp.text().await?);

    // --- exists ---
    let resp = authed_get(port, "exists", "u/test-user/test_flow").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed_get(port, "exists", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- get by path ---
    let resp = authed_get(port, "get", "u/test-user/test_flow").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_flow");
    assert_eq!(body["summary"], "Test flow");

    // get not found
    let resp = authed_get(port, "get", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 404);

    // --- list ---
    let resp = authed(client().get(format!("{base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(
        list.len() >= 2,
        "expected at least 2 flows, got {}",
        list.len()
    );
    assert!(list.iter().any(|f| f["path"] == "u/test-user/test_flow"));

    // --- list_search ---
    let resp = authed(client().get(format!("{base}/list_search")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!list.is_empty());

    // --- list_paths ---
    let resp = authed(client().get(format!("{base}/list_paths")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let paths = resp.json::<Vec<String>>().await?;
    assert!(paths.contains(&"u/test-user/test_flow".to_string()));

    // --- history ---
    let resp = authed_get(port, "history/p", "u/test-user/test_flow").await;
    assert_eq!(resp.status(), 200);
    let history = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!history.is_empty());

    // --- get_latest_version ---
    let resp = authed_get(port, "get_latest_version", "u/test-user/test_flow").await;
    assert_eq!(resp.status(), 200);

    // --- deployment_status ---
    let resp = authed_get(port, "deployment_status/p", "u/test-user/test_flow").await;
    assert_eq!(resp.status(), 200);

    // --- list_tokens ---
    let resp = authed_get(port, "list_tokens", "u/test-user/test_flow").await;
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // --- list_paths_from_workspace_runnable ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/flows/list_paths_from_workspace_runnable/flow/u/test-user/test_flow"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<String>>().await?;

    // --- get by version ---
    let version = &history[0]["id"];
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/flows/get/v/{version}"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_flow");

    // --- update ---
    let resp = authed(client().post(flow_url(port, "update", "u/test-user/test_flow")))
        .json(&new_flow("u/test-user/test_flow", "Updated flow"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "update: {}", resp.text().await?);

    // verify update
    let resp = authed_get(port, "get", "u/test-user/test_flow").await;
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["summary"], "Updated flow");

    // history should have 2 entries
    let resp = authed_get(port, "history/p", "u/test-user/test_flow").await;
    let history = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(
        history.len() >= 2,
        "expected at least 2 history entries, got {}",
        history.len()
    );
    let latest_version = &history[0]["id"];

    // --- get by version + path ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/flows/get/v/{latest_version}/p/u/test-user/test_flow"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_flow");

    // --- history_update ---
    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/flows/history_update/v/{latest_version}"
    )))
    .json(&json!({"deployment_msg": "deployed v2"}))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200, "history_update: {}", resp.text().await?);

    // --- get_triggers_count ---
    let resp = authed(client().get(flow_url(
        port,
        "get_triggers_count",
        "u/test-user/test_flow",
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    // --- toggle_workspace_error_handler (EE-gated, expect 400 in OSS) ---
    let resp = authed(client().post(flow_url(
        port,
        "toggle_workspace_error_handler",
        "u/test-user/test_flow",
    )))
    .json(&json!({}))
    .send()
    .await
    .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "toggle_workspace_error_handler: unexpected status {}",
        resp.status()
    );

    // --- archive ---
    let resp = authed(client().post(flow_url(port, "archive", "u/test-user/another_flow")))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // archived flow should still be gettable
    let resp = authed_get(port, "get", "u/test-user/another_flow").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["archived"], true);

    // --- delete ---
    let resp = authed(client().delete(flow_url(port, "delete", "u/test-user/another_flow")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed_get(port, "exists", "u/test-user/another_flow").await;
    assert_eq!(resp.json::<bool>().await?, false);

    // ===== Hub endpoints (require external network, expect 500 or 200) =====

    // --- hub/list ---
    let resp = authed(client().get(format!("http://localhost:{port}/api/flows/hub/list")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "hub/list: unexpected status {}",
        resp.status()
    );

    // --- hub/get ---
    let resp = authed(client().get(format!("http://localhost:{port}/api/flows/hub/get/1")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "hub/get: unexpected status {}",
        resp.status()
    );

    Ok(())
}

/// Regression test for GHSA-2ppx-66jv-wpw5: a path-scoped token must only see
/// the flows within its scope when listing, even though the route-level scope
/// check only validates `domain:action`. Before the fix, `list_search` returned
/// `path` + the full flow `value` for every flow the underlying user could see,
/// leaking out-of-scope flow definitions to narrowly-scoped tokens.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_list_search_scope_filtering(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/flows");

    // Create two folders and one flow in each, as the (super-admin) test user.
    for folder in ["allowed", "private"] {
        let resp = authed(client().post(format!(
            "http://localhost:{port}/api/w/test-workspace/folders/create"
        )))
        .json(&json!({ "name": folder }))
        .send()
        .await
        .unwrap();
        assert_eq!(resp.status(), 200, "create folder: {}", resp.text().await?);
    }

    for path in ["f/allowed/foo", "f/private/bar"] {
        let resp = authed(client().post(format!("{base}/create")))
            .json(&new_flow(path, "summary"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 201, "create {path}: {}", resp.text().await?);
    }

    // Helper: GET /list_search with an arbitrary bearer token, returning the set
    // of flow paths visible to that token.
    async fn list_search_paths(port: u16, token: &str) -> Vec<String> {
        let resp = client()
            .get(format!(
                "http://localhost:{port}/api/w/test-workspace/flows/list_search"
            ))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        resp.json::<Vec<serde_json::Value>>()
            .await
            .unwrap()
            .into_iter()
            .map(|s| s["path"].as_str().unwrap().to_string())
            .collect()
    }

    // Insert three tokens for the same super-admin user, differing only by scope.
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin, scopes) VALUES
         (encode(sha256('SCOPED_TOKEN'::bytea), 'hex'), 'SCOPED_TOK', 'SCOPED_TOKEN', 'test@windmill.dev', 'scoped', true, ARRAY['flows:read:f/allowed/*']),
         (encode(sha256('BROAD_TOKEN'::bytea), 'hex'), 'BROAD_TOK', 'BROAD_TOKEN', 'test@windmill.dev', 'broad', true, ARRAY['flows:read']),
         (encode(sha256('TAG_TOKEN'::bytea), 'hex'), 'TAG_TOK', 'TAG_TOKEN', 'test@windmill.dev', 'tag-only', true, ARRAY['if_jobs:filter_tags:default'])",
    )
    .execute(&db)
    .await?;

    // Path-scoped token: only sees flows within `f/allowed/*`.
    let scoped = list_search_paths(port, "SCOPED_TOKEN").await;
    assert!(
        scoped.contains(&"f/allowed/foo".to_string()),
        "scoped token should see f/allowed/foo, got: {scoped:?}"
    );
    assert!(
        !scoped.contains(&"f/private/bar".to_string()),
        "scoped token must NOT see f/private/bar, got: {scoped:?}"
    );

    // Broad `flows:read` token: still sees every RLS-visible flow.
    let broad = list_search_paths(port, "BROAD_TOKEN").await;
    assert!(broad.contains(&"f/allowed/foo".to_string()));
    assert!(
        broad.contains(&"f/private/bar".to_string()),
        "broad flows:read token should see all flows, got: {broad:?}"
    );

    // Tag-filter-only token is not scope-restricted: unchanged, sees all.
    let tag_only = list_search_paths(port, "TAG_TOKEN").await;
    assert!(tag_only.contains(&"f/allowed/foo".to_string()));
    assert!(tag_only.contains(&"f/private/bar".to_string()));

    // Unscoped token (no scopes column set): unchanged, sees all.
    let unscoped = list_search_paths(port, "SECRET_TOKEN").await;
    assert!(unscoped.contains(&"f/allowed/foo".to_string()));
    assert!(unscoped.contains(&"f/private/bar".to_string()));

    Ok(())
}
