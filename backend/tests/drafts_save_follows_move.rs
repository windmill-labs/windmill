//! A draft save addressed to a path its item moved away from lands on the moved draft.
//!
//! A move carries every draft on the item to the new path and records where they
//! went. An editor left open across it still saves to the old path; the server puts
//! the save on the moved draft, keeps the path keys the move gave it, and names the
//! new path so the editor can follow. Without the record the save would plant a
//! phantom draft-only item at the old location.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

/// Hex form of script hash 7030, the way the API takes a parent hash.
const HEAD_HASH: &str = "0000000000001b76";

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
        .filter(|p| p.starts_with("u/test-user/follow_"))
        .collect())
}

#[sqlx::test(fixtures("base", "drafts_save_follows_move"))]
async fn test_save_follows_a_rename(db: Pool<Postgres>) -> anyhow::Result<()> {
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
            "path": "u/test-user/follow_b",
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
    assert_eq!(own_draft_paths(port).await?, vec!["u/test-user/follow_b"]);
    // A script draft's `path` is where deploying it lands, so it moves with the row.
    assert_eq!(
        own_draft_value(port, "u/test-user/follow_b").await?["path"],
        "u/test-user/follow_b"
    );

    // The editor is still on the old path and writes that path back into the value.
    let saved: Value = client
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/update/script/u/test-user/follow_a"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({
            "value": {
                "path": "u/test-user/follow_a",
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
    assert_eq!(
        saved["path"], "u/test-user/follow_b",
        "save did not follow the row: {saved}"
    );

    // The write landed on the carried row; nothing reappeared at the old path.
    assert_eq!(own_draft_paths(port).await?, vec!["u/test-user/follow_b"]);
    let draft = own_draft_value(port, "u/test-user/follow_b").await?;
    assert_eq!(draft["content"], "edited after the move", "{draft}");
    assert_eq!(draft["path"], "u/test-user/follow_b", "{draft}");

    Ok(())
}

/// A draft-only move rewrites both path keys. The owner's open editor still carries
/// the typed path it had, which names neither the old nor the new path; the moved
/// draft's own keys have to win, or the save walks the item back.
#[sqlx::test(fixtures("base", "drafts_save_follows_move"))]
async fn test_save_follows_a_draft_only_move(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = reqwest::Client::new();

    let resp = client
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/move/script/u/test-user/draft_store"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({ "new_path": "u/test-user/moved" }))
        .send()
        .await?;
    assert!(resp.status().is_success(), "move failed: {}", resp.text().await?);

    let saved: Value = client
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/update/script/u/test-user/draft_store"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({
            "value": {
                "path": "u/test-user/friendly",
                "draft_path": "u/test-user/friendly",
                "summary": "D",
                "content": "edited after the move"
            }
        }))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(saved["path"], "u/test-user/moved", "{saved}");

    let draft = own_draft_value(port, "u/test-user/moved").await?;
    assert_eq!(draft["content"], "edited after the move", "{draft}");
    assert_eq!(draft["path"], "u/test-user/moved", "{draft}");
    assert_eq!(draft["draft_path"], "u/test-user/moved", "{draft}");
    Ok(())
}
