//! Regression tests for resource value history.
//!
//! Four properties that a later refactor could plausibly undo, each with a cost
//! that is invisible until it bites:
//!  - `state` and `cache` resources are excluded. They are rewritten by every job
//!    that calls `setState` or caches a result, so versioning them would grow the
//!    table without bound and without anyone asking for it.
//!  - an unchanged value mints nothing, which is what keeps no-op saves, renames and
//!    description edits out of the history.
//!  - restore appends the old value as a new version rather than rewinding, so the
//!    history stays append-only and the restore is itself attributable.
//!  - writes that never touch the resource handlers are still recorded. Variable
//!    renames, workspace forks and native integrations all write `resource` directly,
//!    and a rename that changes path and value in one statement used to leave the
//!    newest version holding the pre-rename value while the UI labelled it "Current".
//!    This is why recording lives in a database trigger rather than the handlers.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(b: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    b.header("Authorization", "Bearer RVER_ADMIN_TOKEN")
}

async fn history(base: &str, path: &str) -> anyhow::Result<Vec<Value>> {
    let body: Value = authed(client().get(format!("{base}/resources/history/p/{path}")))
        .send()
        .await?
        .json()
        .await?;
    Ok(body["versions"].as_array().cloned().unwrap_or_default())
}

#[sqlx::test(fixtures("resource_versioning"))]
async fn test_resource_version_history(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api/w/rver-ws", server.addr.port());
    let path = "u/rver-admin/db";

    let resp = authed(client().post(format!("{base}/resources/create")))
        .json(&json!({
            "path": path,
            "value": {"host": "one"},
            "resource_type": "postgresql"
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "create should succeed");
    assert_eq!(history(&base, path).await?.len(), 1, "create mints v1");

    authed(client().post(format!("{base}/resources/update_value/{path}")))
        .json(&json!({"value": {"host": "two"}}))
        .send()
        .await?;
    let versions = history(&base, path).await?;
    assert_eq!(versions.len(), 2, "a changed value mints a version");

    authed(client().post(format!("{base}/resources/update_value/{path}")))
        .json(&json!({"value": {"host": "two"}}))
        .send()
        .await?;
    assert_eq!(
        history(&base, path).await?.len(),
        2,
        "re-saving an identical value must not mint a version"
    );

    // Restore the oldest version: the value goes back, the history grows.
    let oldest = versions.last().unwrap()["id"].as_i64().unwrap();
    let resp = authed(client().post(format!("{base}/resources/history/restore/v/{oldest}")))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "restore should succeed");

    let restored: Value = authed(client().get(format!("{base}/resources/get_value/{path}")))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(restored["host"], "one", "restore brings the old value back");
    assert_eq!(
        history(&base, path).await?.len(),
        3,
        "restore appends rather than rewinding"
    );

    // Machine-written resource types stay out of the history entirely.
    for internal in ["state", "cache"] {
        let internal_path = format!("u/rver-admin/{internal}_item");
        authed(client().post(format!("{base}/resources/create")))
            .json(&json!({
                "path": internal_path,
                "value": {"a": 1},
                "resource_type": internal
            }))
            .send()
            .await?;
        assert_eq!(
            history(&base, &internal_path).await?.len(),
            0,
            "{internal} resources must not be versioned"
        );
    }

    Ok(())
}

/// A version's reported dangling references. Pins that `$jsonvar:` is matched on its own prefix
/// rather than colliding with `$var:` — the two share a suffix, so a check written with a
/// substring test instead of a prefix test would report every `$jsonvar:` as missing.
#[sqlx::test(fixtures("resource_versioning"))]
async fn test_missing_references(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api/w/rver-ws", server.addr.port());
    let path = "u/rver-admin/refs";

    authed(client().post(format!("{base}/variables/create")))
        .json(&json!({
            "path": "u/rver-admin/present",
            "value": "v",
            "is_secret": false,
            "description": ""
        }))
        .send()
        .await?;

    authed(client().post(format!("{base}/resources/create")))
        .json(&json!({
            "path": path,
            "value": {
                "a": "$jsonvar:u/rver-admin/present",
                "b": "$jsonvar:u/rver-admin/absent",
                "c": "$var:u/rver-admin/present",
                "d": "$res:u/rver-admin/absent"
            },
            "resource_type": "postgresql"
        }))
        .send()
        .await?;

    let id = history(&base, path).await?[0]["id"].as_i64().unwrap();
    let version: Value = authed(client().get(format!("{base}/resources/history/v/{id}")))
        .send()
        .await?
        .json()
        .await?;
    let mut missing: Vec<&str> = version["missing_references"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    missing.sort();

    assert_eq!(
        missing,
        vec!["$jsonvar:u/rver-admin/absent", "$res:u/rver-admin/absent"],
        "only the references that do not resolve should be reported"
    );

    Ok(())
}

/// Writes that bypass the resource handlers entirely. A variable rename changes a linked
/// resource's path and value in one statement; the cascading FK moves the history to the new
/// path, so without trigger-level recording the newest row would keep the pre-rename value and
/// disagree with the resource it describes.
#[sqlx::test(fixtures("resource_versioning"))]
async fn test_direct_writes_are_recorded(db: Pool<Postgres>) -> anyhow::Result<()> {
    let path = "u/rver-admin/direct";
    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type, created_by, edited_at)
         VALUES ('rver-ws', $1, '{\"h\":\"one\"}', 'postgresql', 'rver-admin', now())",
    )
    .bind(path)
    .execute(&db)
    .await?;

    let renamed = "u/rver-admin/direct_renamed";
    sqlx::query(
        "UPDATE resource SET path = $1, value = '{\"h\":\"two\"}', edited_at = now()
         WHERE workspace_id = 'rver-ws' AND path = $2",
    )
    .bind(renamed)
    .bind(path)
    .execute(&db)
    .await?;

    let (count, newest): (i64, Option<String>) = sqlx::query_as(
        "SELECT count(*), (SELECT value->>'h' FROM resource_version
                            WHERE workspace_id = 'rver-ws' AND path = $1
                            ORDER BY id DESC LIMIT 1)
           FROM resource_version WHERE workspace_id = 'rver-ws' AND path = $1",
    )
    .bind(renamed)
    .fetch_one(&db)
    .await?;

    assert_eq!(count, 2, "the direct insert and the rename both record");
    assert_eq!(
        newest.as_deref(),
        Some("two"),
        "newest version must match the live value after a rename that also changed it"
    );

    Ok(())
}
