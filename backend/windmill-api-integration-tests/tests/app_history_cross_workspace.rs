//! `app_version.id` is a global sequence, so an app-history deployment message
//! addressed by `(app_id, app_version)` alone crosses workspaces. This pins both
//! halves: the write refuses a version that is not the workspace's app's, and the
//! read only joins deployment metadata owned by the workspace.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn new_app(path: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": "Test app",
        "value": { "type": "rawapp", "inline_script": null },
        "policy": { "execution_mode": "anonymous", "triggerables": {} }
    })
}

/// Creates the app and returns its `(app_id, version_id)`.
async fn create_app(port: u16, workspace: &str, path: &str) -> anyhow::Result<(i64, i64)> {
    let base = format!("http://localhost:{port}/api/w/{workspace}/apps");
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_app(path))
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(status, 201, "create {path} in {workspace}: {body}");

    let latest = authed(client().get(format!("{base}/get_latest_version/{path}")))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    Ok((
        latest["app_id"].as_i64().unwrap(),
        latest["version"].as_i64().unwrap(),
    ))
}

async fn update_history(
    port: u16,
    workspace: &str,
    app_id: i64,
    version: i64,
    msg: &str,
) -> anyhow::Result<(u16, String)> {
    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/w/{workspace}/apps/history_update/a/{app_id}/v/{version}"
    )))
    .json(&json!({ "deployment_msg": msg }))
    .send()
    .await?;
    let status = resp.status().as_u16();
    Ok((status, resp.text().await?))
}

async fn history(port: u16, workspace: &str, path: &str) -> anyhow::Result<serde_json::Value> {
    Ok(authed(client().get(format!(
        "http://localhost:{port}/api/w/{workspace}/apps/history/p/{path}"
    )))
    .send()
    .await?
    .json()
    .await?)
}

#[sqlx::test(
    migrations = "../migrations",
    fixtures("base", "app_history_second_workspace")
)]
async fn test_app_history_is_workspace_scoped(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let victim_path = "u/test-user/victim_app";
    let attacker_path = "u/test-user/attacker_app";
    let (victim_app, victim_version) = create_app(port, "test-workspace", victim_path).await?;
    let (attacker_app, attacker_version) =
        create_app(port, "test-workspace-2", attacker_path).await?;

    // Own app, another workspace's version: the version must belong to the app.
    let (status, body) = update_history(
        port,
        "test-workspace-2",
        attacker_app,
        victim_version,
        "injected",
    )
    .await?;
    assert_eq!(status, 404, "foreign version must be rejected: {body}");

    // Another workspace's app entirely: the app must belong to the workspace.
    let (status, body) = update_history(
        port,
        "test-workspace-2",
        victim_app,
        victim_version,
        "injected",
    )
    .await?;
    assert_eq!(status, 404, "foreign app must be rejected: {body}");

    // The legitimate update still lands, and only in its own workspace.
    let (status, body) = update_history(
        port,
        "test-workspace-2",
        attacker_app,
        attacker_version,
        "deployed by owner",
    )
    .await?;
    assert_eq!(status, 200, "own app and version must be accepted: {body}");
    assert_eq!(
        history(port, "test-workspace-2", attacker_path).await?[0]["deployment_msg"],
        "deployed by owner"
    );

    // A deployment_metadata row owned by another workspace but pointing at this
    // app's version must not surface — the read joins on workspace too.
    sqlx::query(
        "INSERT INTO deployment_metadata (workspace_id, path, app_version, deployment_msg)
         VALUES ('test-workspace-2', $1, $2, 'injected')",
    )
    .bind(victim_path)
    .bind(victim_version)
    .execute(&db)
    .await?;

    assert_eq!(
        history(port, "test-workspace", victim_path).await?[0]["deployment_msg"],
        serde_json::Value::Null,
        "another workspace's deployment metadata must not surface"
    );

    Ok(())
}
