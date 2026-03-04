//! Tests for WM_END_USER_EMAIL environment variable.
//!
//! These tests verify that WM_END_USER_EMAIL is populated with the authenticated
//! user's email when executing app components.
//!
//! TODO: Add tests for scripts and flows once public execution endpoints are identified.
//! Currently only apps support non-workspace-member execution via OptAuthed + token lookup.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::worker::Connection;
use windmill_test_utils::*;

const SAME_WS_TOKEN: &str = "SECRET_TOKEN";
const OTHER_WS_TOKEN: &str = "OTHER_WS_TOKEN";
const NO_WS_TOKEN: &str = "NO_WS_TOKEN";

const SAME_WS_EMAIL: &str = "test@windmill.dev";
const OTHER_WS_EMAIL: &str = "other-ws@windmill.dev";
const NO_WS_EMAIL: &str = "no-ws@windmill.dev";

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

// TODO: Script tests - need to identify public execution endpoints for non-workspace-members
// async fn run_script(port: u16, token: &str) -> anyhow::Result<String> {
//     let url = format!(
//         "http://localhost:{}/api/w/test-workspace/jobs/run_wait_result/p/f/test/get_end_user_email",
//         port
//     );
//     let resp = authed(client().post(&url), token)
//         .json(&json!({}))
//         .send()
//         .await?;
//     if !resp.status().is_success() {
//         anyhow::bail!("script run failed: {} - {}", resp.status(), resp.text().await?);
//     }
//     Ok(resp.json::<serde_json::Value>().await?
//         .as_str().unwrap_or("").to_string())
// }

// TODO: Flow tests - need to identify public execution endpoints for non-workspace-members
// async fn run_flow(port: u16, token: &str) -> anyhow::Result<String> {
//     let url = format!(
//         "http://localhost:{}/api/w/test-workspace/jobs/run_wait_result/f/f/test/get_end_user_email_flow",
//         port
//     );
//     let resp = authed(client().post(&url), token)
//         .json(&json!({}))
//         .send()
//         .await?;
//     if !resp.status().is_success() {
//         anyhow::bail!("flow run failed: {} - {}", resp.status(), resp.text().await?);
//     }
//     Ok(resp.json::<serde_json::Value>().await?
//         .as_str().unwrap_or("").to_string())
// }

/// Create an app with inline script via API
async fn create_app_with_inline_script(port: u16, path: &str) -> anyhow::Result<()> {
    let url = format!(
        "http://localhost:{}/api/w/test-workspace/apps/create",
        port
    );
    let resp = authed(client().post(&url), SAME_WS_TOKEN)
        .json(&json!({
            "path": path,
            "summary": "Test app for WM_END_USER_EMAIL",
            "value": {
                "type": "app",
                "grid": [],
                "subgrids": {},
                "hiddenInlineScripts": [{
                    "name": "get_email",
                    "language": "deno",
                    "content": "export function main() { return Deno.env.get(\"WM_END_USER_EMAIL\") || \"\"; }",
                    "path": "f/test/email_app/get_email"
                }]
            },
            "policy": {
                "execution_mode": "anonymous",
                "on_behalf_of": null,
                "on_behalf_of_email": null,
                "triggerables_v2": {
                    "get_email": {
                        "static_inputs": {},
                        "one_of_inputs": {}
                    },
                    // SHA256 hash of raw_code content for anonymous execution
                    "rawscript/6428aba5aa2d3ea8e1215bfdccbedd3718b18da7a239e3778a9787bb9a0ea606": {
                        "static_inputs": {},
                        "one_of_inputs": {}
                    }
                }
            }
        }))
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("create app failed: {} - {}", resp.status(), resp.text().await?);
    }
    Ok(())
}

/// Create a raw app with inline script via API (uses regular app endpoint with rawapp type)
async fn create_raw_app_with_inline_script(port: u16, path: &str) -> anyhow::Result<()> {
    let url = format!(
        "http://localhost:{}/api/w/test-workspace/apps/create",
        port
    );
    let resp = authed(client().post(&url), SAME_WS_TOKEN)
        .json(&json!({
            "path": path,
            "summary": "Test raw app for WM_END_USER_EMAIL",
            "value": {
                "type": "rawapp",
                "css": "",
                "inlineScripts": [{
                    "name": "get_email",
                    "language": "deno",
                    "content": "export function main() { return Deno.env.get(\"WM_END_USER_EMAIL\") || \"\"; }"
                }]
            },
            "policy": {
                "execution_mode": "anonymous",
                "on_behalf_of": null,
                "on_behalf_of_email": null,
                "triggerables_v2": {
                    "get_email": {
                        "static_inputs": {},
                        "one_of_inputs": {}
                    },
                    // SHA256 hash of raw_code content for anonymous execution
                    "rawscript/6428aba5aa2d3ea8e1215bfdccbedd3718b18da7a239e3778a9787bb9a0ea606": {
                        "static_inputs": {},
                        "one_of_inputs": {}
                    }
                }
            }
        }))
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("create raw app failed: {} - {}", resp.status(), resp.text().await?);
    }
    Ok(())
}

async fn run_app_inline_script(port: u16, token: &str, app_path: &str, force_viewer: bool) -> anyhow::Result<String> {
    let url = format!(
        "http://localhost:{}/api/w/test-workspace/apps_u/execute_component/{}",
        port, app_path
    );
    let mut payload = json!({
        "args": {},
        "component": "get_email",
        "raw_code": {
            "language": "deno",
            "content": "export function main() { return Deno.env.get(\"WM_END_USER_EMAIL\") || \"\"; }",
            "path": format!("{}/get_email", app_path)
        }
    });
    if force_viewer {
        payload["force_viewer_static_fields"] = json!({});
    }
    let resp = authed(client().post(&url), token)
        .json(&payload)
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("app inline script run failed: {} - {}", resp.status(), resp.text().await?);
    }
    let job_id = resp.text().await?;
    wait_for_job_result(port, token, &job_id).await
}

async fn run_raw_app_inline_script(port: u16, token: &str, app_path: &str, force_viewer: bool) -> anyhow::Result<String> {
    let url = format!(
        "http://localhost:{}/api/w/test-workspace/apps_u/execute_component/{}",
        port, app_path
    );
    let mut payload = json!({
        "args": {},
        "component": "get_email",
        "raw_code": {
            "language": "deno",
            "content": "export function main() { return Deno.env.get(\"WM_END_USER_EMAIL\") || \"\"; }"
        }
    });
    if force_viewer {
        payload["force_viewer_static_fields"] = json!({});
    }
    let resp = authed(client().post(&url), token)
        .json(&payload)
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("raw app inline script run failed: {} - {}", resp.status(), resp.text().await?);
    }
    let job_id = resp.text().await?;
    wait_for_job_result(port, token, &job_id).await
}

async fn wait_for_job_result(port: u16, token: &str, job_id: &str) -> anyhow::Result<String> {
    let url = format!(
        "http://localhost:{}/api/w/test-workspace/jobs_u/completed/get_result/{}",
        port, job_id
    );
    for _ in 0..100 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let resp = authed(client().get(&url), token).send().await?;
        if resp.status().is_success() {
            return Ok(resp.json::<serde_json::Value>().await?
                .as_str().unwrap_or("").to_string());
        }
    }
    anyhow::bail!("timeout waiting for job result")
}

// TODO: Script tests - need to identify public execution endpoints for non-workspace-members
// #[cfg(feature = "deno_core")]
// #[sqlx::test(fixtures("base", "end_user_email"))]
// async fn test_script_wm_end_user_email(db: Pool<Postgres>) -> anyhow::Result<()> {
//     initialize_tracing().await;
//     set_jwt_secret().await;
//     let server = ApiServer::start(db.clone()).await?;
//     let port = server.addr.port();
//
//     in_test_worker(Connection::Sql(db.clone()), async move {
//         let result = run_script(port, SAME_WS_TOKEN).await?;
//         assert_eq!(result, SAME_WS_EMAIL, "same workspace user should get their email");
//         Ok::<(), anyhow::Error>(())
//     }, port).await?;
//
//     Ok(())
// }

// TODO: Flow tests - need to identify public execution endpoints for non-workspace-members
// #[cfg(feature = "deno_core")]
// #[sqlx::test(fixtures("base", "end_user_email"))]
// async fn test_flow_wm_end_user_email(db: Pool<Postgres>) -> anyhow::Result<()> {
//     initialize_tracing().await;
//     set_jwt_secret().await;
//     let server = ApiServer::start(db.clone()).await?;
//     let port = server.addr.port();
//
//     in_test_worker(Connection::Sql(db.clone()), async move {
//         let result = run_flow(port, SAME_WS_TOKEN).await?;
//         assert_eq!(result, SAME_WS_EMAIL, "same workspace user should get their email");
//         Ok::<(), anyhow::Error>(())
//     }, port).await?;
//
//     Ok(())
// }

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base", "end_user_email"))]
async fn test_app_wm_end_user_email(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let app_path = "f/test/email_app";

    in_test_worker(Connection::Sql(db.clone()), async move {
        // Create the app with inline script first
        create_app_with_inline_script(port, app_path).await?;

        // Same workspace user (force_viewer mode works for workspace members)
        let result = run_app_inline_script(port, SAME_WS_TOKEN, app_path, true).await?;
        assert_eq!(result, SAME_WS_EMAIL, "same workspace user should get their email");

        // Other workspace user (uses app's anonymous policy + token lookup)
        let result = run_app_inline_script(port, OTHER_WS_TOKEN, app_path, false).await?;
        assert_eq!(result, OTHER_WS_EMAIL, "other workspace user should get their email");

        // No workspace user (uses app's anonymous policy + token lookup)
        let result = run_app_inline_script(port, NO_WS_TOKEN, app_path, false).await?;
        assert_eq!(result, NO_WS_EMAIL, "no workspace user should get their email");

        Ok::<(), anyhow::Error>(())
    }, port).await?;

    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base", "end_user_email"))]
async fn test_raw_app_wm_end_user_email(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let app_path = "f/test/email_raw_app";

    in_test_worker(Connection::Sql(db.clone()), async move {
        // Create the raw app with inline script first
        create_raw_app_with_inline_script(port, app_path).await?;

        // Same workspace user (force_viewer mode works for workspace members)
        let result = run_raw_app_inline_script(port, SAME_WS_TOKEN, app_path, true).await?;
        assert_eq!(result, SAME_WS_EMAIL, "same workspace user should get their email");

        // Other workspace user (uses app's anonymous policy + token lookup)
        let result = run_raw_app_inline_script(port, OTHER_WS_TOKEN, app_path, false).await?;
        assert_eq!(result, OTHER_WS_EMAIL, "other workspace user should get their email");

        // No workspace user (uses app's anonymous policy + token lookup)
        let result = run_raw_app_inline_script(port, NO_WS_TOKEN, app_path, false).await?;
        assert_eq!(result, NO_WS_EMAIL, "no workspace user should get their email");

        Ok::<(), anyhow::Error>(())
    }, port).await?;

    Ok(())
}
