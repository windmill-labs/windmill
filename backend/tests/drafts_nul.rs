//! Regression test for NUL bytes in draft values.
//!
//! `draft.value` is a `json` column (not `jsonb`), so a U+0000 escape can be
//! stored and then make any `->>`/`to_jsonb` extraction raise `22P05` — one
//! poisoned draft 500'd `GET /drafts/list` (silently hiding the home-page
//! "This workspace has N drafts" banner). The fix sanitizes the value on write
//! (`update_draft` -> `strip_json_nul`) so a NUL never reaches the column; this
//! drives the real endpoint and asserts the stored + listed value is NUL-free.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(b: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    b.header("Authorization", "Bearer DNUL_ADMIN_TOKEN")
}

#[sqlx::test(fixtures("drafts_nul"))]
async fn test_draft_write_strips_nul(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/dnul-ws");

    // Save a draft whose summary and content carry a real NUL.
    let resp = authed(client().post(format!("{base}/drafts/update/script/u/dnul-admin/poison")))
        .json(&json!({
            "value": {
                "summary": "hi\u{0}there",
                "path": "u/dnul-admin/poison",
                "content": "x\u{0}y"
            }
        }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        200,
        "save should succeed: {}",
        resp.text().await.unwrap_or_default()
    );

    // The stored value must be NUL-free (sanitized on write).
    let stored: Value =
        authed(client().get(format!("{base}/drafts/get_own/script/u/dnul-admin/poison")))
            .send()
            .await?
            .json()
            .await?;
    let value = stored.get("value").expect("draft should exist");
    assert_eq!(value["summary"], "hithere");
    assert_eq!(value["content"], "xy");
    assert!(
        !serde_json::to_string(value).unwrap().contains("\\u0000"),
        "stored value still contains a NUL escape: {value}"
    );

    // The list endpoint uses raw `->>`; it works (200, no 500) because the
    // stored data is clean, and the summary comes back stripped.
    let items: Vec<Value> = authed(client().get(format!("{base}/drafts/list")))
        .send()
        .await?
        .json()
        .await?;
    let item = items
        .iter()
        .find(|d| d["path"] == "u/dnul-admin/poison")
        .expect("saved draft should be listed");
    assert_eq!(item["summary"], "hithere");

    Ok(())
}
