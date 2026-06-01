//! Regression test for `auto_parent` when all versions at a script path are
//! archived (e.g. after a rename).
//!
//! The CLI's `wmill sync push` sends `parent_hash` together with
//! `auto_parent: true`, delegating parent resolution to the backend. When every
//! version at the target path is archived, there is no active head, so the
//! stale `parent_hash` (an archived ancestor) used to leak into the lineage
//! check and produce a spurious
//! `lineage must be linear: no 2 scripts can have the same parent` error
//! whenever that archived hash already had a child from the prior rename.
//!
//! The fix clears `parent_hash` to `None` in that case so the push starts a
//! fresh lineage instead of failing.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

fn new_script(path: &str, content: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": "",
        "description": "",
        "content": content,
        "language": "deno",
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        }
    })
}

#[sqlx::test(fixtures("base"))]
async fn test_auto_parent_starts_fresh_lineage_when_all_versions_archived(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    let original_path = "u/test-user/script_archived_parent";
    let renamed_path = "u/test-user/script_renamed";

    // 1. Create the initial version at the original path.
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&new_script(
        original_path,
        "export async function main() { return 1; }",
    ))
    .send()
    .await?;
    assert_eq!(resp.status(), 201);
    let original_hash: String = resp.text().await?;

    // 2. Rename the script (new path, parent_hash pointing at v1). This
    //    archives the original hash and gives the new version a
    //    `parent_hashes[1]` equal to `original_hash`, so the original path now
    //    has only archived versions.
    let mut rename = new_script(renamed_path, "export async function main() { return 2; }");
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

    // Sanity: the original path has no active (non-archived) version.
    let active_at_original: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM script WHERE path = $1 AND archived = false AND workspace_id = $2)",
    )
    .bind(original_path)
    .bind("test-workspace")
    .fetch_one(&db)
    .await?;
    assert!(
        !active_at_original,
        "all versions at the original path should be archived after rename"
    );

    // 3. Reproduce `wmill sync push`: push back to the original path with the
    //    stale archived `parent_hash` AND `auto_parent: true`. Before the fix
    //    this returned 400 "lineage must be linear" because the archived hash
    //    already had a child (the renamed version) sharing the same parent.
    let mut push = new_script(original_path, "export async function main() { return 3; }");
    push["parent_hash"] = json!(original_hash);
    push["auto_parent"] = json!(true);
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&push)
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 201,
        "auto_parent push to a path with only archived versions should start a \
         fresh lineage, got {status}: {body}"
    );

    // 4. There is now exactly one active version at the original path and it is
    //    a fresh lineage with no parent (rather than attaching to the archived
    //    ancestor).
    let active: Vec<Option<Vec<i64>>> = sqlx::query_scalar(
        "SELECT parent_hashes FROM script \
         WHERE path = $1 AND archived = false AND workspace_id = $2",
    )
    .bind(original_path)
    .bind("test-workspace")
    .fetch_all(&db)
    .await?;
    assert_eq!(
        active.len(),
        1,
        "exactly one active version expected at the original path"
    );
    assert!(
        active[0].is_none(),
        "fresh lineage should have no parent_hashes, got {:?}",
        active[0]
    );

    Ok(())
}
