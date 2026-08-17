//! Regression test for the cross-workspace custom_path conflict.
//!
//! When custom paths are instance-global (CLOUD_HOSTED unset and
//! `app_workspaced_route` off — the default for dedicated instances), a
//! custom_path is a single global route slot. The uniqueness check correctly
//! blocks two apps from claiming it, including the same logical app deployed
//! to two workspaces (staging/prod, git-sync). The bug was that the error
//! ("App with custom path <x> already exists") gave the operator no idea
//! where the conflicting copy lived. This test pins down:
//!   - a single-workspace edit keeping its own custom_path still succeeds
//!     (the app's own row is excluded),
//!   - a real conflict is still rejected, and
//!   - the error now names the conflicting app's path and workspace.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

fn new_app(path: &str, custom_path: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": "Test app",
        "value": { "type": "rawapp", "inline_script": null },
        "policy": { "execution_mode": "anonymous", "triggerables": {} },
        "custom_path": custom_path
    })
}

#[sqlx::test(fixtures("app_custom_path_cross_workspace"))]
async fn test_custom_path_cross_workspace_deploy(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws_a = format!("http://localhost:{port}/api/w/test-workspace");
    let ws_b = format!("http://localhost:{port}/api/w/test-workspace-2");

    let app_path = "f/Newsletter/newsletter_composer";
    let custom_path = "newsletter";

    // 1. Create the app with a custom path in workspace A.
    let resp = authed(client().post(format!("{ws_a}/apps/create")), "SECRET_TOKEN")
        .json(&new_app(app_path, custom_path))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        201,
        "create app in ws A should succeed: {}",
        resp.text().await?
    );

    // 2. Editing the app in its own workspace, keeping the same custom path,
    //    must still succeed — the app's own row is excluded from the check.
    //    (This is the common single-workspace deploy; it must not regress.)
    let resp = authed(
        client().post(format!("{ws_a}/apps/update/{app_path}")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "summary": "Test app (edited)",
        "value": { "type": "rawapp", "inline_script": null },
        "policy": { "execution_mode": "anonymous", "triggerables": {} },
        "custom_path": custom_path
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "editing an app in its own workspace keeping its custom path must succeed: {}",
        resp.text().await?
    );

    // 3. Deploying the same app (same path) to a second workspace is a real
    //    conflict in global mode (one global route slot). It must be rejected,
    //    and the error must name the conflicting workspace + app so the
    //    operator knows what to resolve.
    let resp = authed(client().post(format!("{ws_b}/apps/create")), "SECRET_TOKEN")
        .json(&new_app(app_path, custom_path))
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 400,
        "same custom path in another workspace is a global conflict: {body}"
    );
    assert!(
        body.contains("test-workspace") && body.contains(app_path),
        "error must name the conflicting workspace and app, got: {body}"
    );

    // 4. A genuinely different app claiming the in-use custom path is still
    //    rejected, with the same actionable message.
    let resp = authed(client().post(format!("{ws_a}/apps/create")), "SECRET_TOKEN")
        .json(&new_app("f/Other/other_app", custom_path))
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 400,
        "a different app must not steal an in-use custom path: {body}"
    );
    assert!(
        body.contains(app_path),
        "error must name the app already using the custom path, got: {body}"
    );

    Ok(())
}
