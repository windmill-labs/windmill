//! Which user tokens get an "expiring soon" warning queued when they are created.

use serde_json::json;
use sqlx::types::chrono::Utc;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const DAY: u64 = 24 * 60 * 60;

async fn warning_queued(db: &Pool<Postgres>, label: &str) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM token_expiry_notification n
            JOIN token t ON t.token_hash = n.token_hash WHERE t.label = $1)",
    )
    .bind(label)
    .fetch_one(db)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_short_lived_tokens_get_no_expiry_warning(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    for (label, lifetime) in [("short", DAY), ("long", 30 * DAY)] {
        let resp = reqwest::Client::new()
            .post(format!("http://localhost:{port}/api/users/tokens/create"))
            .header("Authorization", "Bearer SECRET_TOKEN_2")
            .json(&json!({
                "label": label,
                "expiration": Utc::now() + std::time::Duration::from_secs(lifetime),
            }))
            .send()
            .await?;
        assert_eq!(resp.status(), 201);
    }

    assert!(
        !warning_queued(&db, "short").await,
        "a token whose whole lifetime fits in the warning window must not be warned about"
    );
    assert!(
        warning_queued(&db, "long").await,
        "a longer-lived token still gets its warning"
    );
    Ok(())
}
