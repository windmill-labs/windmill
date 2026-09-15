//! A rename onto a path that already holds a draft is refused.
//!
//! Nothing deployed can sit at a rename's destination (the deploy conflicts on
//! that), but a draft can: a never-deployed item, or a draft left on an archived
//! script. Moving onto it would merge two items or strand a row, so the rename
//! itself fails, in its own transaction, and the source stays deployed. The
//! destination draft here is the deployer's own, which is the same collision.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

/// Hex form of script hash 7010, the way the API takes a parent hash.
const HEAD_HASH: &str = "0000000000001b62";

#[sqlx::test(fixtures("base", "drafts_move_taken"))]
async fn test_rename_onto_a_draft_is_refused(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = reqwest::Client::new();

    let resp = client
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/scripts/create"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({
            "path": "u/test-user/mvtaken_b",
            "parent_hash": HEAD_HASH,
            "summary": "A",
            "description": "",
            "content": "export function main() { return 1 }",
            "language": "deno",
            "schema": {}
        }))
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(status, 400, "rename onto a draft was not refused: {body}");
    assert!(
        body.contains("already has a draft"),
        "unexpected refusal: {body}"
    );

    // The whole deploy rolled back: the source is still the live head, and the
    // draft at the destination is untouched.
    let head: Value = client
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/scripts/get/p/u/test-user/mvtaken_a"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(head["hash"], HEAD_HASH, "source was replaced: {head}");
    assert_eq!(head["archived"], false, "source was archived: {head}");

    let draft: Value = client
        .get(format!("http://localhost:{port}/api/w/test-workspace/drafts/get_own/script/u/test-user/mvtaken_b"))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(
        draft["value"]["summary"], "B",
        "destination draft changed: {draft}"
    );

    Ok(())
}

/// A legacy (ownerless) draft occupies its path too: a deploy there deletes it together
/// with the caller's own row, so a move that parks a second draft beside it would discard
/// edits the caller never saw. Only an admin can clear it, so the refusal says so.
#[sqlx::test(fixtures("base", "drafts_move_taken"))]
async fn test_draft_move_refuses_a_legacy_destination(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/move/script/u/test-user/mvtaken_b"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({ "new_path": "u/test-user/mvtaken_legacy" }))
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(status, 400, "move onto a legacy draft was allowed: {body}");
    assert!(
        body.contains("legacy workspace draft") && body.contains("workspace admin"),
        "the refusal did not point at the one remedy: {body}"
    );

    // Both rows stayed where they were: the caller's own, and the legacy one the list
    // synthesizes under the caller's name.
    let list: Vec<Value> = reqwest::Client::new()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/list"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .send()
        .await?
        .json()
        .await?;
    let mut at = list
        .iter()
        .filter_map(|d| Some((d["kind"].as_str()?, d["path"].as_str()?)))
        .filter(|(_, p)| p.starts_with("u/test-user/mvtaken_b") || p.ends_with("mvtaken_legacy"))
        .collect::<Vec<_>>();
    at.sort();
    assert_eq!(
        at,
        vec![
            ("script", "u/test-user/mvtaken_b"),
            ("script", "u/test-user/mvtaken_legacy")
        ],
        "{list:?}"
    );
    Ok(())
}

/// A classic app and a raw app deploy into the same table, so a draft-only move onto
/// the other kind's draft must be refused: deploying either path afterwards deletes
/// the caller's drafts of both kinds, taking the loser's item with it.
#[sqlx::test(fixtures("base", "drafts_move_taken"))]
async fn test_draft_move_refuses_the_other_app_kind(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/move/raw_app/u/test-user/mvtaken_raw"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({ "new_path": "u/test-user/mvtaken_app" }))
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 400,
        "move onto a classic app draft was allowed: {body}"
    );
    assert!(
        body.contains("already have a draft at 'u/test-user/mvtaken_app' (app)"),
        "the refusal did not name the occupying kind: {body}"
    );

    // Both drafts are untouched.
    let list: Vec<Value> = reqwest::Client::new()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/list"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .send()
        .await?
        .json()
        .await?;
    let mut at = list
        .iter()
        .filter(|d| matches!(d["kind"].as_str(), Some("app") | Some("raw_app")))
        .filter_map(|d| Some((d["kind"].as_str()?, d["path"].as_str()?)))
        .collect::<Vec<_>>();
    at.sort();
    assert_eq!(
        at,
        vec![
            ("app", "u/test-user/mvtaken_app"),
            ("raw_app", "u/test-user/mvtaken_raw")
        ],
        "{list:?}"
    );
    Ok(())
}

/// Teammates' drafts of one item share its path by design, so another user's row is no
/// obstacle — except across the app pair, where the two kinds are different items on one
/// deployed path: deploying either strands the other, and deleting the app takes both.
#[sqlx::test(fixtures("base", "drafts_move_taken"))]
async fn test_draft_move_refuses_another_users_other_app_kind(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let move_to = |kind: &'static str, from: &'static str, to: &'static str| async move {
        let resp = reqwest::Client::new()
            .post(format!(
                "http://localhost:{port}/api/w/test-workspace/drafts/move/{kind}/{from}"
            ))
            .header("Authorization", "Bearer SECRET_TOKEN")
            .json(&json!({ "new_path": to }))
            .send()
            .await?;
        Ok::<_, anyhow::Error>((resp.status(), resp.text().await?))
    };

    let (status, body) = move_to(
        "app",
        "u/test-user/mvtaken_app",
        "u/test-user/mvtaken_theirs",
    )
    .await?;
    assert_eq!(
        status, 400,
        "a classic app was moved onto another user's raw app: {body}"
    );
    assert!(
        body.contains("holds another user's raw app draft"),
        "the refusal did not name the occupant: {body}"
    );

    // And the other direction, where the occupant reads as the classic kind.
    let (status, body) = move_to(
        "raw_app",
        "u/test-user/mvtaken_raw",
        "u/test-user/mvtaken_app_theirs",
    )
    .await?;
    assert_eq!(
        status, 400,
        "a raw app was moved onto another user's classic app: {body}"
    );
    assert!(
        body.contains("holds another user's app draft"),
        "the refusal did not name the occupant: {body}"
    );

    // The same-kind case is the ordinary one: two users' drafts of one raw app.
    let (status, body) = move_to(
        "raw_app",
        "u/test-user/mvtaken_raw",
        "u/test-user/mvtaken_theirs",
    )
    .await?;
    assert!(
        status.is_success(),
        "a raw app was refused beside another user's raw-app draft: {body}"
    );
    Ok(())
}
