use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn assert_2xx(status: u16, body: &str, endpoint: &str) {
    assert!(
        (200..300).contains(&status),
        "{endpoint} returned {status}: {body}",
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_worker_group_crud(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/configs");

    // Create a worker group
    let resp = authed(client().post(format!("{base}/update/worker__test_group")))
        .json(&json!({"worker_tags": ["test_tag"]}))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /configs/update/worker__test_group",
    );

    // Verify it exists via get
    let resp = authed(client().get(format!("{base}/get/worker__test_group")))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "GET /configs/get/worker__test_group");
    let config: serde_json::Value = serde_json::from_str(&body)?;
    assert_eq!(config["worker_tags"], json!(["test_tag"]));

    // Verify it appears in list_worker_groups with prefix stripped
    let resp = authed(client().get(format!("{base}/list_worker_groups")))
        .send()
        .await?;
    let body = resp.text().await?;
    let groups: Vec<serde_json::Value> = serde_json::from_str(&body)?;
    let test_group = groups.iter().find(|g| g["name"] == "test_group");
    assert!(
        test_group.is_some(),
        "test_group should appear in list_worker_groups with prefix stripped"
    );

    // Delete the worker group
    let resp = authed(client().delete(format!("{base}/update/worker__test_group")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "DELETE /configs/update/worker__test_group",
    );

    // Verify it's gone from list_worker_groups
    let resp = authed(client().get(format!("{base}/list_worker_groups")))
        .send()
        .await?;
    let body = resp.text().await?;
    let groups: Vec<serde_json::Value> = serde_json::from_str(&body)?;
    let test_group = groups.iter().find(|g| g["name"] == "test_group");
    assert!(
        test_group.is_none(),
        "test_group should be gone after deletion"
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_list_worker_groups_excludes_non_worker_entries(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/configs");

    // Insert a non-worker entry directly into the config table
    sqlx::query("INSERT INTO config (name, config) VALUES ($1, $2) ON CONFLICT (name) DO UPDATE SET config = EXCLUDED.config")
        .bind("test__other_entry")
        .bind(json!({"some_key": "some_value"}))
        .execute(&db)
        .await?;

    // list_worker_groups should NOT return it
    let resp = authed(client().get(format!("{base}/list_worker_groups")))
        .send()
        .await?;
    let body = resp.text().await?;
    let groups: Vec<serde_json::Value> = serde_json::from_str(&body)?;
    let other_entry = groups
        .iter()
        .find(|g| g["name"] == "test__other_entry" || g["name"] == "other_entry");
    assert!(
        other_entry.is_none(),
        "non-worker entries should not appear in list_worker_groups"
    );

    // Clean up
    sqlx::query("DELETE FROM config WHERE name = $1")
        .bind("test__other_entry")
        .execute(&db)
        .await?;

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_no_alert_entries_in_config_table_after_migration(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    // After migration, no alert__* entries should remain in the config table
    let rows: Vec<(String,)> = sqlx::query_as("SELECT name FROM config WHERE name LIKE 'alert__%'")
        .fetch_all(&db)
        .await?;
    assert!(
        rows.is_empty(),
        "No alert entries should remain in config table after migration, found: {:?}",
        rows.iter().map(|(n,)| n.as_str()).collect::<Vec<_>>()
    );

    Ok(())
}
