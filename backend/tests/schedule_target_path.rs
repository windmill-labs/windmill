use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

#[sqlx::test(fixtures("base"))]
async fn create_stores_the_parsed_target(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!(
        "http://localhost:{}/api/w/test-workspace/schedules",
        server.addr.port()
    );
    let client = reqwest::Client::new();
    let create = |path: &str, script_path: &str| {
        client
            .post(format!("{base}/create"))
            .bearer_auth("SECRET_TOKEN")
            .json(&json!({
                "path": path,
                "schedule": "0 0 0 * * *",
                "timezone": "UTC",
                "script_path": script_path,
                "is_flow": false,
                "args": {},
                "enabled": false,
            }))
            .send()
    };

    let resp = create("u/test-user/one_slash", "/u/test-user/x").await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let stored: String = sqlx::query_scalar(
        "SELECT script_path FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'u/test-user/one_slash'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(stored, "u/test-user/x");

    let resp = create("u/test-user/two_slashes", "//u/test-user/x").await?;
    assert_eq!(resp.status(), 400, "{}", resp.text().await?);
    Ok(())
}
