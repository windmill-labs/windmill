//! Regression tests for resource value history.
//!
//! Three properties that a later refactor could plausibly undo, each with a cost
//! that is invisible until it bites:
//!  - `state` and `cache` resources are excluded. They are rewritten by every job
//!    that calls `setState` or caches a result, so versioning them would grow the
//!    table without bound and without anyone asking for it.
//!  - an unchanged value mints nothing, which is what keeps no-op saves (and
//!    trashbin restores, which rewrite the value the last version already holds)
//!    out of the history.
//!  - restore appends the old value as a new version rather than rewinding, so the
//!    history stays append-only and the restore is itself attributable.

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
    Ok(
        authed(client().get(format!("{base}/resources/history/p/{path}")))
            .send()
            .await?
            .json()
            .await?,
    )
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
