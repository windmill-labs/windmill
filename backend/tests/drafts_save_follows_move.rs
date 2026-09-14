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

/// Rename `from` to `to` the way Home does: redeploy the deployed content at the new
/// path, keeping the deployer's own draft so it is carried rather than consumed.
/// Returns the new head's hash.
async fn rename(port: u16, from_hash: &str, to: &str) -> anyhow::Result<String> {
    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/scripts/create"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({
            "path": to,
            "parent_hash": from_hash,
            "summary": "A",
            "description": "",
            "content": "export function main() { return 1 }",
            "language": "deno",
            "schema": {},
            "skip_draft_deletion": true
        }))
        .send()
        .await?;
    let status = resp.status();
    let hash = resp.text().await?;
    assert_eq!(status, 201, "rename to {to} failed: {hash}");
    Ok(hash)
}

/// Save the draft as an editor still bound to `url_path` would. Returns the path the
/// save landed at.
async fn save_at(port: u16, url_path: &str, content: &str) -> anyhow::Result<String> {
    let saved: Value = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/update/script/{url_path}"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({
            "value": { "path": url_path, "summary": "A", "content": content, "language": "deno" }
        }))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(saved["status"], "saved", "save refused: {saved}");
    Ok(saved["path"].as_str().unwrap_or_default().to_string())
}

/// A record is kept to one hop, and a move back to the path it left ends it: both are
/// three statements whose order decides the answer.
#[sqlx::test(fixtures("base", "drafts_save_follows_move"))]
async fn test_move_records_stay_one_hop(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let b = rename(port, HEAD_HASH, "u/test-user/follow_b").await?;
    let _c = rename(port, &b, "u/test-user/follow_c").await?;
    assert_eq!(
        save_at(port, "u/test-user/follow_a", "after two moves").await?,
        "u/test-user/follow_c",
        "a save at the first path did not reach the last"
    );
    assert_eq!(own_draft_paths(port).await?, vec!["u/test-user/follow_c"]);
    Ok(())
}

#[sqlx::test(fixtures("base", "drafts_save_follows_move"))]
async fn test_move_back_ends_the_record(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let b = rename(port, HEAD_HASH, "u/test-user/follow_b").await?;
    let _a = rename(port, &b, "u/test-user/follow_a").await?;
    assert_eq!(
        save_at(port, "u/test-user/follow_a", "after moving back").await?,
        "u/test-user/follow_a",
        "a save was routed off the path the item moved back to"
    );
    assert_eq!(own_draft_paths(port).await?, vec!["u/test-user/follow_a"]);
    Ok(())
}

/// A rename carries a teammate's row too: both its path keys follow, and the version
/// it forked from does not move. A restamp there would clear their out-of-date prompt
/// and let them deploy over the mover's version believing they were current.
#[sqlx::test(fixtures("base", "drafts_save_follows_move"))]
async fn test_a_teammates_draft_follows_with_its_base(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    rename(port, HEAD_HASH, "u/test-user/follow_b").await?;

    // Read from the pool: the teammate's row is another user's, and this asserts on
    // `base`, which no endpoint exposes for someone else's draft.
    let row: (String, String, Option<String>) = sqlx::query_as(
        "SELECT value::jsonb ->> 'path', value::jsonb ->> 'draft_path', base
         FROM draft WHERE workspace_id = 'test-workspace' AND typ = 'script'
           AND email = 'test2@windmill.dev'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(row.0, "u/test-user/follow_b", "typed path did not follow");
    assert_eq!(row.1, "u/test-user/follow_b", "mirror did not follow");
    assert_eq!(
        row.2.as_deref(),
        Some(HEAD_HASH),
        "the teammate's base was restamped by someone else's rename"
    );
    Ok(())
}

/// An item move and then the owner's own move of what is left: the two records have
/// different scopes, so the owner's move has to extend the chain in its own scope or
/// a save addressed to the first path stops at the abandoned middle one.
#[sqlx::test(fixtures("base", "drafts_save_follows_move"))]
async fn test_an_owner_move_extends_an_item_move(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    rename(port, HEAD_HASH, "u/test-user/follow_b").await?;
    // Archiving the script at the new path leaves the carried draft as a draft-only
    // item, which its owner can move through `/drafts/move`.
    sqlx::query("UPDATE script SET archived = true WHERE path = 'u/test-user/follow_b'")
        .execute(&db)
        .await?;
    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/move/script/u/test-user/follow_b"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({ "new_path": "u/test-user/follow_c" }))
        .send()
        .await?;
    assert!(resp.status().is_success(), "move failed: {}", resp.text().await?);

    assert_eq!(
        save_at(port, "u/test-user/follow_a", "after both moves").await?,
        "u/test-user/follow_c",
        "a save at the first path stopped at the path the owner's move left"
    );
    assert_eq!(own_draft_paths(port).await?, vec!["u/test-user/follow_c"]);
    Ok(())
}

/// Redeploying at a path an owner's move routed away from ends that route: the live item
/// owns its path again, and its saves must not follow the draft that left.
#[sqlx::test(fixtures("base", "drafts_save_follows_move"))]
async fn test_redeploy_at_a_routed_path_ends_the_route(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // `move_draft` ignores archived rows, so an archived script's draft can be moved away.
    sqlx::query("UPDATE script SET archived = true WHERE path = 'u/test-user/follow_a'")
        .execute(&db)
        .await?;
    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/move/script/u/test-user/follow_a"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({ "new_path": "u/test-user/follow_b" }))
        .send()
        .await?;
    assert!(resp.status().is_success(), "move failed: {}", resp.text().await?);

    // Unarchiving redeploys at the same path, with the archived version as parent.
    rename(port, HEAD_HASH, "u/test-user/follow_a").await?;

    assert_eq!(
        save_at(port, "u/test-user/follow_a", "for the live script").await?,
        "u/test-user/follow_a",
        "a save for the redeployed script followed the draft that moved away"
    );
    Ok(())
}

/// A draft written before the NUL sanitizer still has to follow a move: its path keys are
/// what a deploy of it would land on, so the carry rewrites them, sanitizing the value it
/// could not otherwise parse.
#[sqlx::test(fixtures("base", "drafts_save_follows_move"))]
async fn test_a_poisoned_draft_follows_a_rename(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // The teammate's row, rewritten the way a pre-sanitizer client left one: a real NUL
    // escape in the content, both path keys naming the path the item is about to leave.
    sqlx::query(
        r#"UPDATE draft SET value = '{"path": "u/test-user/follow_a", "draft_path": "u/test-user/follow_a",
             "parent_hash": "0000000000001b76", "summary": "A", "content": "a\u0000b"}'
           WHERE email = 'test2@windmill.dev'"#,
    )
    .execute(&db)
    .await?;

    rename(port, HEAD_HASH, "u/test-user/follow_b").await?;

    let row: (String, String, String) = sqlx::query_as(
        "SELECT value::jsonb ->> 'path', value::jsonb ->> 'draft_path', value::jsonb ->> 'content'
         FROM draft WHERE email = 'test2@windmill.dev'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(row.0, "u/test-user/follow_b", "typed path did not follow");
    assert_eq!(row.1, "u/test-user/follow_b", "mirror did not follow");
    assert_eq!(row.2, "ab", "the NUL survived the rewrite");
    Ok(())
}
