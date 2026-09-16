//! `max_token_expiration_days`: the instance-wide ceiling on how far ahead a token a caller
//! picks the lifetime of may expire.

use serde_json::json;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const DAY: u64 = 24 * 60 * 60;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn from_now(secs: u64) -> DateTime<Utc> {
    Utc::now() + std::time::Duration::from_secs(secs)
}

async fn set_max(db: &Pool<Postgres>, value: serde_json::Value) {
    sqlx::query(
        "INSERT INTO global_settings (name, value) VALUES ('max_token_expiration_days', $1)
         ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(value)
    .execute(db)
    .await
    .unwrap();
}

/// Mints as `test2@windmill.dev`, a plain member of `test-workspace`.
async fn create_token(port: u16, body: serde_json::Value) -> reqwest::Response {
    client()
        .post(format!("http://localhost:{port}/api/users/tokens/create"))
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .json(&body)
        .send()
        .await
        .unwrap()
}

async fn stored_expiration(db: &Pool<Postgres>, label: &str) -> Option<DateTime<Utc>> {
    sqlx::query_scalar::<_, Option<DateTime<Utc>>>("SELECT expiration FROM token WHERE label = $1")
        .bind(label)
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_max_token_expiration_days_shortens_user_tokens(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let resp = create_token(port, json!({ "label": "unset" })).await;
    assert_eq!(resp.status(), 201);
    assert_eq!(
        stored_expiration(&db, "unset").await,
        None,
        "with no setting a token may still have no expiration"
    );

    set_max(&db, json!(7)).await;

    // The token form reads the ceiling as whoever is creating the token, usually not a
    // superadmin, so it can offer only expirations the server would keep.
    let resp = client()
        .get(format!(
            "http://localhost:{port}/api/settings/global/max_token_expiration_days"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await?, "7");

    let resp = create_token(port, json!({ "label": "none asked" })).await;
    assert_eq!(resp.status(), 201);
    let expiration = stored_expiration(&db, "none asked")
        .await
        .expect("a token asking for no expiration gets the ceiling");
    assert!(
        expiration > from_now(6 * DAY) && expiration <= from_now(7 * DAY),
        "expected the 7 day ceiling, got {expiration}"
    );

    let resp = create_token(
        port,
        json!({ "label": "past the ceiling", "expiration": from_now(30 * DAY) }),
    )
    .await;
    assert_eq!(resp.status(), 201);
    let expiration = stored_expiration(&db, "past the ceiling").await.unwrap();
    assert!(
        expiration > from_now(6 * DAY) && expiration <= from_now(7 * DAY),
        "expected an expiration past the ceiling to be shortened to it, got {expiration}"
    );

    let resp = create_token(
        port,
        json!({ "label": "within", "expiration": from_now(3 * DAY) }),
    )
    .await;
    assert_eq!(resp.status(), 201);
    let expiration = stored_expiration(&db, "within").await.unwrap();
    assert!(
        expiration <= from_now(3 * DAY),
        "an expiration within the ceiling must be kept, got {expiration}"
    );

    // The settings UI stores an integer, but the YAML instance config and config sync can write
    // the same whole number as a string or as `5.0`. Reading either as "unset" would silently
    // drop the ceiling, while the token form (`parseMaxTokenExpirationDays`) still showed it.
    for (stored, label) in [
        (json!("5"), "string setting"),
        (json!(5.0), "float setting"),
    ] {
        set_max(&db, stored).await;
        let resp = create_token(port, json!({ "label": label })).await;
        assert_eq!(resp.status(), 201);
        let expiration = stored_expiration(&db, label).await;
        assert!(
            expiration.is_some_and(|e| e > from_now(4 * DAY) && e <= from_now(5 * DAY)),
            "{label}: expected the 5 day ceiling, got {expiration:?}"
        );
    }

    sqlx::query(
        "UPDATE usr SET is_service_account = true
         WHERE email = 'test2@windmill.dev' AND workspace_id = 'test-workspace'",
    )
    .execute(&db)
    .await?;
    let resp = create_token(
        port,
        json!({ "label": "service account", "workspace_id": "test-workspace" }),
    )
    .await;
    assert_eq!(resp.status(), 201);
    assert!(
        stored_expiration(&db, "service account").await.is_some(),
        "a service account gets the ceiling like anyone else"
    );

    // A superadmin impersonating a user picks the lifetime too, so the ceiling applies there;
    // left out, it would be the one way to mint a token that never expires.
    let resp = client()
        .post(format!(
            "http://localhost:{port}/api/users/tokens/impersonate"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({ "label": "impersonated", "impersonate_email": "test3@windmill.dev" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201);
    assert!(
        stored_expiration(&db, "impersonated").await.is_some(),
        "an impersonation token asking for no expiration gets the ceiling"
    );

    Ok(())
}
