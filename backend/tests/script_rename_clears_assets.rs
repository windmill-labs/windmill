//! Regression test: renaming a script clears the OLD path's static asset
//! usage rows.
//!
//! A script deploy persists its producer/consumer asset lineage as `asset`
//! rows keyed by `usage_path = <script path>`, `usage_kind = 'script'`. A
//! rename is a deploy with a new `path` and a `parent_hash` pointing at the
//! version being renamed. The new path's rows are (re)written by the deploy,
//! but the old path's rows must be cleared — otherwise the renamed script
//! keeps lingering in the asset graph at a path where it no longer exists.
//!
//! The clear used to be attempted with the NEW (not-yet-inserted) script hash,
//! which resolved to no path and cleared nothing.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

fn new_script_with_asset(path: &str, content: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": "",
        "description": "",
        "content": content,
        "language": "bash",
        "assets": [{
            "path": "u/test-user/my_db",
            "kind": "resource",
            "access_type": "rw"
        }]
    })
}

async fn asset_usage_paths(db: &Pool<Postgres>) -> anyhow::Result<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT usage_path FROM asset \
         WHERE usage_kind = 'script' AND path = $1 AND workspace_id = $2 \
         ORDER BY usage_path",
    )
    .bind("u/test-user/my_db")
    .bind("test-workspace")
    .fetch_all(db)
    .await?)
}

#[sqlx::test(fixtures("base"))]
async fn test_rename_clears_old_path_asset_usage(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    let original_path = "u/test-user/asset_rename_orig";
    let renamed_path = "u/test-user/asset_rename_renamed";

    // 1. Deploy a script that produces (rw) a resource asset.
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&new_script_with_asset(original_path, "# v1"))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "create should succeed");
    let original_hash: String = resp.text().await?;

    assert_eq!(
        asset_usage_paths(&db).await?,
        vec![original_path.to_string()],
        "asset usage should be recorded at the original path"
    );

    // 2. Rename: deploy at a new path with parent_hash = v1.
    let mut rename = new_script_with_asset(renamed_path, "# v2");
    rename["parent_hash"] = json!(original_hash);
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&rename)
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "rename should succeed: {}",
        resp.text().await?
    );

    // 3. Only the new path retains the asset usage; the old path is cleared.
    assert_eq!(
        asset_usage_paths(&db).await?,
        vec![renamed_path.to_string()],
        "after rename the old path's asset usage must be cleared and only the new path kept"
    );

    Ok(())
}
