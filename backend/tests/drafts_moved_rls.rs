//! `POST /drafts/update` must never name a destination the caller cannot see.
//!
//! When an item moves, the saver's editor is still bound to the old path and its
//! next save is answered with `status: "moved"` plus the new path and the mover's
//! username. That answer is resolved twice — once before the write, once after it
//! on the write's own connection — and both must read under RLS. A post-write
//! re-assert on a raw pool connection reads rows the pre-check cannot see, so it
//! discloses the destination and permanently refuses a save that should land.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

/// Hex form of script hash 7001, the way a script draft stores `parent_hash`.
const OLD_HASH: &str = "0000000000001b59";

async fn save_draft(port: u16, token: &str) -> anyhow::Result<Value> {
    Ok(reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/drafts/update/script/f/mvrls_visible/s1"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "value": {
                "path": "f/mvrls_visible/s1",
                "parent_hash": OLD_HASH,
                "content": "export function main() { return 2 }",
                "language": "deno",
                "summary": "S1",
                "description": "",
                "schema": {}
            }
        }))
        .send()
        .await?
        .json()
        .await?)
}

#[sqlx::test(fixtures("base", "drafts_moved_rls"))]
async fn test_moved_answer_is_rls_scoped(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // test-user-2 writes in `mvrls_visible` but has nothing on `mvrls_secret`.
    // They must be told the save landed, not where the item went.
    let res = save_draft(port, "SECRET_TOKEN_2").await?;
    assert_eq!(res["status"], "saved", "non-admin save was refused: {res}");
    assert_eq!(res["moved_to"], Value::Null, "destination disclosed: {res}");
    assert_eq!(res["moved_by"], Value::Null, "mover disclosed: {res}");

    // The admin sees the destination, so they get the real answer.
    let res = save_draft(port, "SECRET_TOKEN").await?;
    assert_eq!(res["status"], "moved", "admin was not told it moved: {res}");
    assert_eq!(res["moved_to"], "f/mvrls_secret/s1");
    assert_eq!(res["moved_by"], "test-user");

    Ok(())
}
