//! `max_token_expiration_days`: the instance-wide ceiling on how far ahead a token minted
//! through `POST /users/tokens/create` may expire, and the service-account exemption that
//! keeps unattended automation able to hold longer-lived credentials.

use serde_json::json;
use sqlx::types::chrono::Utc;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

async fn set_max_days(db: &Pool<Postgres>, days: i64) {
    sqlx::query(
        "INSERT INTO global_settings (name, value) VALUES ('max_token_expiration_days', $1)
         ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(json!(days))
    .execute(db)
    .await
    .unwrap();
}

/// Mints as `test2@windmill.dev`, a plain (non service account) member of `test-workspace`.
async fn create_token(port: u16, body: serde_json::Value) -> reqwest::Response {
    client()
        .post(format!("http://localhost:{port}/api/users/tokens/create"))
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .json(&body)
        .send()
        .await
        .unwrap()
}

fn in_days(days: u64) -> String {
    (Utc::now() + std::time::Duration::from_secs(days * 24 * 60 * 60)).to_rfc3339()
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_max_token_expiration_days_caps_user_tokens(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let resp = create_token(port, json!({ "label": "unset" })).await;
    assert_eq!(resp.status(), 201, "no setting must leave tokens uncapped");

    set_max_days(&db, 7).await;

    let resp = create_token(port, json!({ "label": "no expiry" })).await;
    assert_eq!(
        resp.status(),
        400,
        "a token with no expiration must be refused"
    );

    let resp = create_token(
        port,
        json!({ "label": "too far", "expiration": in_days(8) }),
    )
    .await;
    assert_eq!(
        resp.status(),
        400,
        "an expiration past the cap must be refused"
    );

    let resp = create_token(port, json!({ "label": "within", "expiration": in_days(6) })).await;
    assert_eq!(
        resp.status(),
        201,
        "an expiration within the cap must be accepted"
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_service_accounts_are_exempt(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    set_max_days(&db, 7).await;
    sqlx::query(
        "UPDATE usr SET is_service_account = true
         WHERE email = 'test2@windmill.dev' AND workspace_id = 'test-workspace'",
    )
    .execute(&db)
    .await?;

    let resp = create_token(port, json!({ "label": "global" })).await;
    assert_eq!(
        resp.status(),
        201,
        "a service account may mint a global token with no expiration"
    );

    let resp = create_token(
        port,
        json!({ "label": "own workspace", "workspace_id": "test-workspace" }),
    )
    .await;
    assert_eq!(
        resp.status(),
        201,
        "and one scoped to the workspace it is a service account in"
    );

    // The exemption follows the service-account membership, so it must not carry to a
    // workspace where this email is an ordinary user.
    let resp = create_token(
        port,
        json!({ "label": "other workspace", "workspace_id": "other-workspace" }),
    )
    .await;
    assert_eq!(resp.status(), 400, "but not to another workspace");

    Ok(())
}
