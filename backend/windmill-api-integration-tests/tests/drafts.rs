use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_draft_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/drafts");

    // create a script first so the draft has a valid path
    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/scripts/create"
    )))
    .json(&json!({
        "path": "u/test-user/draft_script",
        "summary": "Script for draft test",
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

    // --- create draft ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": "u/test-user/draft_script",
            "typ": "script",
            "value": {
                "content": "export async function main() { return 2; }",
                "language": "deno"
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create draft: {}", resp.text().await?);

    // verify draft exists via script get/draft endpoint
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/scripts/get/draft/u/test-user/draft_script"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert!(body["draft"].is_object(), "expected draft to be present");

    // --- update draft (create with same path overwrites) ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": "u/test-user/draft_script",
            "typ": "script",
            "value": {
                "content": "export async function main() { return 3; }",
                "language": "deno"
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // --- delete draft ---
    let resp = authed(client().delete(format!(
        "{base}/delete/script/u/test-user/draft_script"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    // verify draft is gone
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/scripts/get/draft/u/test-user/draft_script"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert!(body["draft"].is_null(), "expected draft to be deleted");

    Ok(())
}
