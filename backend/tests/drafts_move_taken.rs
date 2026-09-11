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
