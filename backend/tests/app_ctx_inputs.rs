//! A context input a raw app runnable declares (`fields.<arg> = { type: "ctx", .. }`) is
//! filled by the server from the viewer's session. Whatever the request sends under that
//! name is discarded: a runnable filters rows on it, so a viewer who could set it would
//! read someone else's.
//!
//! Users from the `base` fixture:
//!   test-user   (admin,     token SECRET_TOKEN)
//!   test-user-2 (developer, token SECRET_TOKEN_2)

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const ADMIN_TOKEN: &str = "SECRET_TOKEN";
const VIEWER_TOKEN: &str = "SECRET_TOKEN_2";
const APP_PATH: &str = "u/test-user/ctx_app";
const CONTENT: &str = "export function main(who: string) { return who }";
// sha256 of CONTENT, the key an inline runnable is pinned under.
const CONTENT_SHA: &str = "6e198199621d57e4fed834510cb0c2692274aa4cbe976e1704fab2806be9d744";

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

#[sqlx::test(fixtures("base"))]
async fn a_declared_ctx_input_ignores_the_value_the_viewer_sends(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");
    let client = reqwest::Client::new();

    let resp = authed(client.post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": APP_PATH,
            "summary": "ctx inputs",
            "value": {
                "runnables": {
                    "whoami": {
                        "type": "inline",
                        "inlineScript": { "language": "bun", "content": CONTENT },
                        "fields": { "who": { "type": "ctx", "ctx": "username" } }
                    }
                }
            },
            // The policy says nothing of context inputs: the server reads the
            // declaration from the deployed runnable.
            "policy": {
                "execution_mode": "anonymous",
                "triggerables_v2": {
                    format!("whoami:rawscript/{CONTENT_SHA}"): {
                        "static_inputs": {},
                        "one_of_inputs": {}
                    }
                }
            }
        }))
        .send()
        .await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);

    // Both spellings a viewer could send: a forged literal, and no argument at all.
    for args in [json!({ "who": "test-user" }), json!({})] {
        let resp = authed(
            client.post(format!("{ws}/apps_u/execute_component/{APP_PATH}")),
            VIEWER_TOKEN,
        )
        .json(&json!({
            "component": "whoami",
            "args": args,
            "raw_code": { "language": "bun", "content": CONTENT }
        }))
        .send()
        .await?;
        assert!(resp.status().is_success(), "{}", resp.text().await?);
        let job_id: uuid::Uuid = resp.text().await?.parse()?;

        let queued: serde_json::Value = sqlx::query_scalar("SELECT args FROM v2_job WHERE id = $1")
            .bind(job_id)
            .fetch_one(&db)
            .await?;
        assert_eq!(queued["who"], json!("test-user-2"), "sent {args}");
    }

    Ok(())
}
