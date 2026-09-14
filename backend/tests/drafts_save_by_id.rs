//! A draft saved by row id lands where the row is, not where the editor was.
//!
//! A rename carries every draft on the item to the new path. An editor left open
//! across it is still bound to the old path; saving by the row's id writes at the
//! item's current path and the response names it, so the editor can follow. A
//! save by path would instead have planted a phantom draft at the old location.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

/// Hex form of script hash 7030, the way the API takes a parent hash.
const HEAD_HASH: &str = "0000000000001b76";
const DRAFT_ID: i64 = 9001;

async fn own_draft_value(port: u16, path: &str) -> anyhow::Result<Value> {
    let draft: Value = reqwest::Client::new()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/get_own/script/{path}"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .send()
        .await?
        .json()
        .await?;
    Ok(draft["value"].clone())
}

async fn own_draft_paths(port: u16) -> anyhow::Result<Vec<String>> {
    let list: Vec<Value> = reqwest::Client::new()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/list"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .send()
        .await?
        .json()
        .await?;
    Ok(list
        .iter()
        .filter(|d| d["kind"] == "script")
        .filter_map(|d| d["path"].as_str().map(String::from))
        .filter(|p| p.starts_with("u/test-user/byid_"))
        .collect())
}

#[sqlx::test(fixtures("base", "drafts_save_by_id"))]
async fn test_save_by_id_follows_a_rename(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = reqwest::Client::new();

    // Rename from Home: a redeploy of the deployed content at the new path that
    // keeps the deployer's own draft, so the draft is carried rather than consumed.
    let resp = client
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/scripts/create"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({
            "path": "u/test-user/byid_b",
            "parent_hash": HEAD_HASH,
            "summary": "A",
            "description": "",
            "content": "export function main() { return 1 }",
            "language": "deno",
            "schema": {},
            "skip_draft_deletion": true
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "rename failed: {}", resp.text().await?);
    assert_eq!(own_draft_paths(port).await?, vec!["u/test-user/byid_b"]);
    // A script draft's `path` is where deploying it lands, so it moves with the row.
    assert_eq!(
        own_draft_value(port, "u/test-user/byid_b").await?["path"],
        "u/test-user/byid_b"
    );

    // The editor is still on the old path but saves by id, writing that path back.
    let saved: Value = client
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/update/script/u/test-user/byid_a"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({
            "id": DRAFT_ID,
            "value": {
                "path": "u/test-user/byid_a",
                "parent_hash": HEAD_HASH,
                "summary": "A",
                "content": "edited after the move"
            }
        }))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(saved["status"], "saved", "save refused: {saved}");
    assert_eq!(saved["id"], DRAFT_ID);
    assert_eq!(
        saved["path"], "u/test-user/byid_b",
        "save did not follow the row: {saved}"
    );

    // The write landed on the carried row; nothing reappeared at the old path.
    assert_eq!(own_draft_paths(port).await?, vec!["u/test-user/byid_b"]);
    let draft = own_draft_value(port, "u/test-user/byid_b").await?;
    assert_eq!(draft["content"], "edited after the move", "{draft}");
    assert_eq!(draft["path"], "u/test-user/byid_b", "{draft}");

    Ok(())
}
