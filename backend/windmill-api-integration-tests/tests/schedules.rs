use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn schedule_url(port: u16, endpoint: &str, path: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/schedules/{endpoint}/{path}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

async fn authed_get(port: u16, endpoint: &str, path: &str) -> reqwest::Response {
    authed(client().get(schedule_url(port, endpoint, path)))
        .send()
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_schedule_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/schedules");

    // create a script for the schedule to reference
    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/scripts/create"
    )))
    .json(&json!({
        "path": "u/test-user/scheduled_script",
        "summary": "Scheduled script",
        "description": "",
        "content": "export async function main() { return 1; }",
        "language": "deno",
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        }
    }))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 201, "create script: {}", resp.text().await?);

    // --- create ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": "u/test-user/test_schedule",
            "schedule": "0 0 */6 * * *",
            "timezone": "UTC",
            "script_path": "u/test-user/scheduled_script",
            "is_flow": false,
            "enabled": false
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "create: {}", resp.text().await?);

    // create second schedule
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": "u/test-user/another_schedule",
            "schedule": "0 0 0 * * *",
            "timezone": "America/New_York",
            "script_path": "u/test-user/scheduled_script",
            "is_flow": false,
            "enabled": false
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "create another: {}", resp.text().await?);

    // --- exists ---
    let resp = authed_get(port, "exists", "u/test-user/test_schedule").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed_get(port, "exists", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- get ---
    let resp = authed_get(port, "get", "u/test-user/test_schedule").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_schedule");
    assert_eq!(body["schedule"], "0 0 */6 * * *");
    assert_eq!(body["timezone"], "UTC");
    assert_eq!(body["script_path"], "u/test-user/scheduled_script");
    assert_eq!(body["is_flow"], false);
    assert_eq!(body["enabled"], false);

    // --- list ---
    let resp = authed(client().get(format!("{base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(
        list.len() >= 2,
        "expected at least 2 schedules, got {}",
        list.len()
    );
    assert!(list.iter().any(|s| s["path"] == "u/test-user/test_schedule"));

    // --- list_with_jobs ---
    let resp = authed(client().get(format!("{base}/list_with_jobs")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!list.is_empty());

    // --- update ---
    let resp = authed(client().post(schedule_url(
        port,
        "update",
        "u/test-user/test_schedule",
    )))
    .json(&json!({
        "schedule": "0 0 */12 * * *",
        "timezone": "Europe/Paris"
    }))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200, "update: {}", resp.text().await?);

    // verify update
    let resp = authed_get(port, "get", "u/test-user/test_schedule").await;
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["schedule"], "0 0 */12 * * *");
    assert_eq!(body["timezone"], "Europe/Paris");

    // --- setenabled ---
    let resp = authed(client().post(schedule_url(
        port,
        "setenabled",
        "u/test-user/test_schedule",
    )))
    .json(&json!({"enabled": true}))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed_get(port, "get", "u/test-user/test_schedule").await;
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["enabled"], true);

    // disable it back
    let resp = authed(client().post(schedule_url(
        port,
        "setenabled",
        "u/test-user/test_schedule",
    )))
    .json(&json!({"enabled": false}))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    // --- setdefaulthandler ---
    let resp = authed(client().post(format!("{base}/setdefaulthandler")))
        .json(&json!({
            "handler_type": "error",
            "override_existing": false,
            "path": "u/test-user/scheduled_script",
            "number_of_occurence": 1,
            "number_of_occurence_exact": false
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "setdefaulthandler: {}",
        resp.text().await?
    );

    // clear default handler
    let resp = authed(client().post(format!("{base}/setdefaulthandler")))
        .json(&json!({
            "handler_type": "error",
            "override_existing": false
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- delete ---
    let resp = authed(client().delete(schedule_url(
        port,
        "delete",
        "u/test-user/another_schedule",
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed_get(port, "exists", "u/test-user/another_schedule").await;
    assert_eq!(resp.json::<bool>().await?, false);

    // ===== Global endpoints =====

    // --- preview ---
    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/schedules/preview"
    )))
    .json(&json!({
        "schedule": "0 0 */6 * * *",
        "timezone": "UTC"
    }))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200, "preview: {}", resp.text().await?);

    Ok(())
}
