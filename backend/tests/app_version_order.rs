//! The deployed order of an app's versions is the order they were appended to
//! `app.versions`, not the order of their `created_at`.
//!
//! `app_version.created_at` defaults to `now()`, which in Postgres is the
//! transaction's start time, while the append happens under the app row's lock.
//! Two deploys that overlap therefore land in one order and carry timestamps in
//! the other. The head the editor guards against, and the sequence the diff
//! picker numbers, both have to follow the array.
//!
//! Users from the `base` fixture: test-user (admin, token SECRET_TOKEN).

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const TOKEN: &str = "SECRET_TOKEN";

#[sqlx::test(fixtures("base"))]
async fn test_app_head_follows_the_append_order_not_the_timestamps(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let ws = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{ws}/apps/create"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .json(&json!({
            "path": "u/test-user/order_app",
            "summary": "ordered",
            "value": {},
            "policy": { "execution_mode": "publisher", "triggerables": {} }
        }))
        .send()
        .await?;
    assert!(res.status().is_success(), "{}", res.text().await?);

    let first: i64 = sqlx::query_scalar(
        "SELECT versions[array_upper(versions, 1)] FROM app
         WHERE workspace_id = 'test-workspace' AND path = 'u/test-user/order_app'",
    )
    .fetch_one(&db)
    .await?;

    // The overlapping deploy: appended after `first`, so it is the version that
    // landed, but stamped before it, so a timestamp sort puts it underneath.
    let second: i64 = sqlx::query_scalar(
        "INSERT INTO app_version (app_id, value, created_by, created_at, raw_app)
         SELECT app_id, value, 'racer', created_at - interval '1 hour', raw_app
         FROM app_version WHERE id = $1
         RETURNING id",
    )
    .bind(first)
    .fetch_one(&db)
    .await?;
    sqlx::query(
        "UPDATE app SET versions = array_append(versions, $1::bigint)
         WHERE workspace_id = 'test-workspace' AND path = 'u/test-user/order_app'",
    )
    .bind(second)
    .execute(&db)
    .await?;

    let head: serde_json::Value = client
        .get(format!(
            "{ws}/apps/get_latest_version/u/test-user/order_app"
        ))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(
        head["version"], second,
        "the head is the version appended last, not the newest timestamp: {head}"
    );

    let history: Vec<serde_json::Value> = client
        .get(format!("{ws}/apps/history/p/u/test-user/order_app"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .send()
        .await?
        .json()
        .await?;
    let listed: Vec<i64> = history
        .iter()
        .map(|v| v["version"].as_i64().unwrap())
        .collect();
    assert_eq!(
        listed,
        vec![second, first],
        "the picker numbers the history by deployed order, so it leads with the head"
    );

    Ok(())
}
