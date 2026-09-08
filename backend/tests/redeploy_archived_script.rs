//! Pins that a path holding no live version can be redeployed with the content it
//! already holds in its history.
//!
//! A script hash is derived from (path, parent, content, metadata), so a deploy that
//! resolves to no parent re-derives the hash of the version that started the lineage
//! and lands on that row. `wmill sync push` walks straight into it: a locally deleted
//! file archives the path, and the push then redeploys it unchanged.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn new_script(path: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": "",
        "description": "",
        "content": "export async function main() { return 1; }",
        "language": "deno",
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        }
    })
}

async fn create(port: u16, script: serde_json::Value) -> anyhow::Result<String> {
    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/scripts/create?skip_if_noop=true"
    )))
    .json(&script)
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(status, 201, "script create should succeed: {body}");
    Ok(body)
}

async fn post(port: u16, route: &str) -> anyhow::Result<()> {
    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/{route}"
    )))
    .send()
    .await?;
    assert!(resp.status().is_success(), "{route} should succeed");
    Ok(())
}

/// The live version at `path`, or `None` when the path holds none.
async fn live_hash(port: u16, path: &str) -> anyhow::Result<Option<String>> {
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/scripts/get/p/{path}"
    )))
    .send()
    .await?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let script: serde_json::Value = resp.json().await?;
    Ok(Some(script["hash"].as_str().unwrap().to_string()))
}

#[sqlx::test(fixtures("base"))]
async fn test_redeploy_unchanged_content_onto_retired_path(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // The CLI's own shape: it names the version it read and asks the server to re-resolve
    // the parent, which finds nothing live once the archive landed.
    let archived_path = "u/test-user/retired_auto_parent";
    let v1 = create(port, new_script(archived_path)).await?;
    post(port, &format!("scripts/archive/p/{archived_path}")).await?;
    let mut redeploy = new_script(archived_path);
    redeploy["parent_hash"] = json!(v1);
    redeploy["auto_parent"] = json!(true);
    let v2 = create(port, redeploy).await?;
    assert_ne!(
        v1, v2,
        "the redeploy must be a new version, not the archived one"
    );
    assert_eq!(
        live_hash(port, archived_path).await?,
        Some(v2),
        "the redeployed script must be the live version at its path"
    );

    // A retried push sends no parent at all: the archived path is absent from the
    // listing it diffed against, so it deploys as if the path were new.
    let parentless_path = "u/test-user/retired_parentless";
    create(port, new_script(parentless_path)).await?;
    post(port, &format!("scripts/archive/p/{parentless_path}")).await?;
    create(port, new_script(parentless_path)).await?;

    // A deleted version keeps its row, and its hash with it.
    let deleted_path = "u/test-user/retired_deleted";
    let deleted = create(port, new_script(deleted_path)).await?;
    post(port, &format!("scripts/delete/h/{deleted}")).await?;
    create(port, new_script(deleted_path)).await?;

    Ok(())
}
