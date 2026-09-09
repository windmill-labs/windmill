//! Moving a draft must carry both of its path keys.
//!
//! A draft value holds a typed path and a mirror the editors keep beside it while
//! it differs from the row's path (`path`/`draft_path`; which is which depends on
//! the kind). The loaders prefer the mirror, so a move that rewrote only the typed
//! key left the mirror naming the old location: reopening the item restored the
//! old path, and the next autosave wrote it back — undoing the move silently.
//!
//! The rule is spread over three sites that have to agree (`move_draft`, the
//! passive carry in `move_drafts_for_path`, and `resolve_moved_to_in`'s patch);
//! this pins the endpoint, including that a draft with no mirror never gains one.

use serde_json::Value;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

async fn move_to(port: u16, from: &str, to: &str) -> anyhow::Result<String> {
    Ok(reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/move/script/{from}"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&serde_json::json!({ "new_path": to }))
        .send()
        .await?
        .text()
        .await?)
}

/// The stored draft value at `path`, read back through the API so this test needs
/// no `sqlx::query!` (which would want an offline cache entry of its own).
async fn value_at(port: u16, path: &str) -> anyhow::Result<Value> {
    let body: Value = reqwest::Client::new()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/get_own/script/{path}"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .send()
        .await?
        .json()
        .await?;
    Ok(body
        .get("value")
        .cloned()
        .unwrap_or_else(|| panic!("no draft at {path}: {body}")))
}

#[sqlx::test(fixtures("base", "drafts_move_mirror"))]
async fn test_move_carries_both_path_keys(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    move_to(port, "u/test-user/draft_mirror", "u/test-user/renamed").await?;
    move_to(port, "u/test-user/draft_plain", "u/test-user/plain2").await?;

    // The mirror follows: left at `u/test-user/friendly` it would win at load and
    // walk the item back there.
    let moved = value_at(port, "u/test-user/renamed").await?;
    assert_eq!(moved["path"], "u/test-user/renamed");
    assert_eq!(moved["draft_path"], "u/test-user/renamed");

    // A draft that never had a mirror must not be given one.
    let plain = value_at(port, "u/test-user/plain2").await?;
    assert_eq!(plain["path"], "u/test-user/plain2");
    assert_eq!(plain.get("draft_path"), None, "mirror injected: {plain}");

    Ok(())
}
