//! Guests are unavailable on the shared cloud (`CLOUD_HOSTED`).
//!
//! One test in its own binary on purpose: `CLOUD_HOSTED` is read once into a
//! `lazy_static`, so it must be set before anything reads it and cannot be unset for a
//! sibling test in the same process.
//!
//! Users from the `base` fixture:
//!   test-user   (admin,     token SECRET_TOKEN)

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const ADMIN_TOKEN: &str = "SECRET_TOKEN";
const APP_PATH: &str = "u/test-user/guest_app";

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

#[sqlx::test(fixtures("base"))]
async fn the_cloud_admits_no_guest(db: Pool<Postgres>) -> anyhow::Result<()> {
    // Before the server starts, so the flag is what the whole process sees.
    unsafe { std::env::set_var("CLOUD_HOSTED", "true") };
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    // The workspace switch cannot be turned on, so no policy can lean on it.
    let resp = authed(
        client().post(format!("{ws}/workspaces/edit_guest_access")),
        ADMIN_TOKEN,
    )
    .json(&json!({ "guest_access_enabled": true }))
    .send()
    .await?;
    assert_eq!(resp.status(), 400);
    assert!(
        resp.text().await?.contains("self-hosted"),
        "the refusal must name what guests need"
    );

    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": APP_PATH,
            "summary": "Guest app",
            "value": {},
            "policy": { "execution_mode": "guest", "triggerables": {} }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 400, "an app cannot be deployed to guests");

    // An app already stored in guest mode — pushed by git-sync, or deployed before the
    // instance became a cloud one — advertises no entry either.
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": APP_PATH,
            "summary": "Guest app",
            "value": {},
            "policy": { "execution_mode": "publisher", "triggerables": {} }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    sqlx::query(
        "UPDATE app SET policy = jsonb_set(policy, '{execution_mode}', '\"guest\"')
         WHERE path = $1 AND workspace_id = 'test-workspace'",
    )
    .bind(APP_PATH)
    .execute(&db)
    .await?;
    sqlx::query("UPDATE workspace_settings SET guest_access_enabled = true WHERE workspace_id = 'test-workspace'")
        .execute(&db)
        .await?;

    let secret: String = authed(
        client().get(format!("{ws}/apps/secret_of/{APP_PATH}")),
        ADMIN_TOKEN,
    )
    .send()
    .await?
    .text()
    .await?;
    let resp = client()
        .get(format!("{ws}/apps_u/guest_entry/{secret}"))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        404,
        "a guest app must not advertise entry where guests are unavailable"
    );

    Ok(())
}
