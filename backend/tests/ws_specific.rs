//! Integration tests for the workspace-specific (ws_specific) feature.
//!
//! Covers three regression-prone areas:
//!
//! 1. **Linked-delete cleanup** — deleting a resource (or variable) must also
//!    drop the cross-kind ws_specific row that was auto-inserted by
//!    `mark_linked_variables_ws_specific` so a later item recreated at the
//!    same path doesn't inherit a stale flag.
//! 2. **list_ws_specific authorization filtering** — the endpoint must hide
//!    paths the caller cannot see via the underlying resource/variable RLS.
//! 3. **Resource upsert with `ws_specific: false`** — `create_resource` with
//!    `update_if_exists=true` and `ws_specific: false` must clear an
//!    existing flag (was previously a silent no-op).

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

/// Helper: count rows in ws_specific for (workspace, kind, path).
async fn ws_specific_row_count(
    db: &Pool<Postgres>,
    workspace: &str,
    kind: &str,
    path: &str,
) -> anyhow::Result<i64> {
    let n: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ws_specific
         WHERE workspace_id = $1 AND item_kind = $2 AND path = $3",
    )
    .bind(workspace)
    .bind(kind)
    .bind(path)
    .fetch_one(db)
    .await?;
    Ok(n.unwrap_or(0))
}

#[sqlx::test(fixtures("ws_specific"))]
async fn test_linked_delete_cleanup(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // Create a referenced variable.
    let resp = authed(
        client().post(format!("{base}/variables/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "u/test-user/db_pwd",
        "value": "hunter2",
        "is_secret": false,
        "description": ""
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "create var: {}", resp.text().await?);

    // Create a ws_specific resource that references the variable via $var:.
    let resp = authed(
        client().post(format!("{base}/resources/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "u/test-user/db",
        "value": { "user": "admin", "password": "$var:u/test-user/db_pwd" },
        "description": "",
        "resource_type": "object",
        "ws_specific": true
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "create res: {}", resp.text().await?);

    // The auto-mark on save inserts a ws_specific 'variable' row for the
    // linked variable.
    assert_eq!(
        ws_specific_row_count(&db, "test-workspace", "variable", "u/test-user/db_pwd").await?,
        1,
        "linked variable should be auto-marked ws_specific"
    );
    assert_eq!(
        ws_specific_row_count(&db, "test-workspace", "resource", "u/test-user/db").await?,
        1
    );

    // Delete the resource — it should cascade to the linked variable AND
    // the ws_specific row for that variable.
    let resp = authed(
        client().delete(format!("{base}/resources/delete/u/test-user/db")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "delete res: {}", resp.text().await?);

    assert_eq!(
        ws_specific_row_count(&db, "test-workspace", "resource", "u/test-user/db").await?,
        0,
        "resource ws_specific row should be gone"
    );
    assert_eq!(
        ws_specific_row_count(&db, "test-workspace", "variable", "u/test-user/db_pwd").await?,
        0,
        "orphaned linked-variable ws_specific row should also be gone"
    );

    // The same fix applies in reverse: delete_variable must clean the
    // ws_specific 'resource' row at the same path.
    let resp = authed(
        client().post(format!("{base}/variables/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "u/test-user/twin",
        "value": "v",
        "is_secret": false,
        "description": ""
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201);

    let resp = authed(
        client().post(format!("{base}/resources/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "u/test-user/twin",
        "value": { "x": 1 },
        "resource_type": "object",
        "ws_specific": true
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201);

    // ws_specific row exists for resource at u/test-user/twin
    assert_eq!(
        ws_specific_row_count(&db, "test-workspace", "resource", "u/test-user/twin").await?,
        1
    );

    // Delete the variable at the same path: cascades to the resource AND
    // its ws_specific row.
    let resp = authed(
        client().delete(format!("{base}/variables/delete/u/test-user/twin")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "delete var: {}", resp.text().await?);

    assert_eq!(
        ws_specific_row_count(&db, "test-workspace", "resource", "u/test-user/twin").await?,
        0,
        "ws_specific resource row should be cleaned by variable delete"
    );

    Ok(())
}

#[sqlx::test(fixtures("ws_specific"))]
async fn test_list_ws_specific_filters_by_rls(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // Admin creates two ws_specific items: one in u/test-user/* (private to
    // test-user) and one in u/test-user-2/* (private to test-user-2).
    for path in ["u/test-user/admin_only", "u/test-user-2/user2_only"] {
        let resp = authed(
            client().post(format!("{base}/variables/create")),
            "SECRET_TOKEN",
        )
        .json(&json!({
            "path": path,
            "value": "v",
            "is_secret": false,
            "description": "",
            "ws_specific": true
        }))
        .send()
        .await?;
        assert_eq!(
            resp.status(),
            201,
            "create var {path}: {}",
            resp.text().await?
        );
    }

    // Admin sees both via list_ws_specific.
    let resp = authed(
        client().get(format!("{base}/workspaces/list_ws_specific")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    let admin_items: Vec<serde_json::Value> = resp.json().await?;
    let admin_paths: Vec<&str> = admin_items
        .iter()
        .filter_map(|i| i.get("path").and_then(|p| p.as_str()))
        .collect();
    assert!(admin_paths.contains(&"u/test-user/admin_only"));
    assert!(admin_paths.contains(&"u/test-user-2/user2_only"));

    // Non-admin (test-user-2) only sees their own u/test-user-2/* path —
    // u/test-user/admin_only is filtered by RLS see_own (path requires
    // SPLIT_PART(path,'/',2) = session.user).
    let resp = authed(
        client().get(format!("{base}/workspaces/list_ws_specific")),
        "SECRET_TOKEN_2",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    let user2_items: Vec<serde_json::Value> = resp.json().await?;
    let user2_paths: Vec<&str> = user2_items
        .iter()
        .filter_map(|i| i.get("path").and_then(|p| p.as_str()))
        .collect();
    assert!(
        user2_paths.contains(&"u/test-user-2/user2_only"),
        "user2 should see their own ws_specific item, got: {user2_paths:?}"
    );
    assert!(
        !user2_paths.contains(&"u/test-user/admin_only"),
        "user2 should NOT see admin's ws_specific item, got: {user2_paths:?}"
    );

    Ok(())
}

#[sqlx::test(fixtures("ws_specific"))]
async fn test_create_resource_upsert_clears_ws_specific(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // Step 1: create resource with ws_specific=true.
    let resp = authed(
        client().post(format!("{base}/resources/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "u/test-user/upsert_target",
        "value": { "host": "h" },
        "resource_type": "object",
        "ws_specific": true
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    assert_eq!(
        ws_specific_row_count(
            &db,
            "test-workspace",
            "resource",
            "u/test-user/upsert_target"
        )
        .await?,
        1
    );

    // Step 2: upsert (update_if_exists=true) with ws_specific=false — must
    // CLEAR the existing row.
    let resp = authed(
        client().post(format!("{base}/resources/create?update_if_exists=true")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "u/test-user/upsert_target",
        "value": { "host": "h2" },
        "resource_type": "object",
        "ws_specific": false
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    assert_eq!(
        ws_specific_row_count(
            &db,
            "test-workspace",
            "resource",
            "u/test-user/upsert_target"
        )
        .await?,
        0,
        "ws_specific=false on upsert must clear the existing row"
    );

    // Step 3: upsert without ws_specific (None) leaves whatever's there
    // alone — re-flag it true, then upsert with no field, expect row stays.
    let resp = authed(
        client().post(format!("{base}/resources/create?update_if_exists=true")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "u/test-user/upsert_target",
        "value": { "host": "h3" },
        "resource_type": "object",
        "ws_specific": true
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201);
    assert_eq!(
        ws_specific_row_count(
            &db,
            "test-workspace",
            "resource",
            "u/test-user/upsert_target"
        )
        .await?,
        1
    );

    let resp = authed(
        client().post(format!("{base}/resources/create?update_if_exists=true")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "path": "u/test-user/upsert_target",
        "value": { "host": "h4" },
        "resource_type": "object"
        // no ws_specific field
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201);
    assert_eq!(
        ws_specific_row_count(
            &db,
            "test-workspace",
            "resource",
            "u/test-user/upsert_target"
        )
        .await?,
        1,
        "absent ws_specific field must leave the existing flag alone"
    );

    Ok(())
}
