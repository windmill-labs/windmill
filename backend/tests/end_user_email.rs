//! Tests for WM_END_USER_EMAIL environment variable.
//!
//! These tests verify that WM_END_USER_EMAIL is populated with the authenticated
//! user's email when executing app components and when running scripts and flows directly.
//! Only apps support non-workspace-member execution via OptAuthed + token lookup.

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

const GET_EMAIL_CONTENT: &str =
    "export function main() { return process.env.WM_END_USER_EMAIL || \"\"; }";

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

/// Create an app with inline script via API
async fn create_app_with_inline_script(port: u16, path: &str) -> anyhow::Result<()> {
    let url = format!("http://localhost:{}/api/w/test-workspace/apps/create", port);
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
                    "inlineScript": {
                        "language": "bun",
                        "content": GET_EMAIL_CONTENT,
                        "path": "f/test/email_app/get_email"
                    }
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
                    "rawscript/894d2f85ea2411e777e7d5787366c9fec4c1a7a5f83482748f1d129196c9f54f": {
                        "static_inputs": {},
                        "one_of_inputs": {}
                    }
                }
            }
        }))
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!(
            "create app failed: {} - {}",
            resp.status(),
            resp.text().await?
        );
    }
    Ok(())
}

/// Create a raw app with inline script via API (uses regular app endpoint with rawapp type)
async fn create_raw_app_with_inline_script(port: u16, path: &str) -> anyhow::Result<()> {
    let url = format!("http://localhost:{}/api/w/test-workspace/apps/create", port);
    let resp = authed(client().post(&url), SAME_WS_TOKEN)
        .json(&json!({
            "path": path,
            "summary": "Test raw app for WM_END_USER_EMAIL",
            "value": {
                "type": "rawapp",
                "css": "",
                "runnables": {
                    "get_email": {
                        "name": "get_email",
                        "type": "inline",
                        "inlineScript": { "language": "bun", "content": GET_EMAIL_CONTENT }
                    }
                }
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
                    "rawscript/894d2f85ea2411e777e7d5787366c9fec4c1a7a5f83482748f1d129196c9f54f": {
                        "static_inputs": {},
                        "one_of_inputs": {}
                    }
                }
            }
        }))
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!(
            "create raw app failed: {} - {}",
            resp.status(),
            resp.text().await?
        );
    }
    Ok(())
}

/// Create an app whose policy allows triggering the fixture flow
async fn create_app_triggering_flow(port: u16, path: &str, flow_path: &str) -> anyhow::Result<()> {
    let url = format!("http://localhost:{}/api/w/test-workspace/apps/create", port);
    let resp = authed(client().post(&url), SAME_WS_TOKEN)
        .json(&json!({
            "path": path,
            "summary": "Test app running a flow for WM_END_USER_EMAIL",
            "value": {
                "type": "app",
                "grid": [],
                "subgrids": {},
                "hiddenInlineScripts": []
            },
            "policy": {
                "execution_mode": "anonymous",
                "on_behalf_of": null,
                "on_behalf_of_email": null,
                "triggerables_v2": {
                    flow_path: {
                        "static_inputs": {},
                        "one_of_inputs": {}
                    }
                }
            }
        }))
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!(
            "create app failed: {} - {}",
            resp.status(),
            resp.text().await?
        );
    }
    Ok(())
}

async fn run_app_flow(
    port: u16,
    token: &str,
    app_path: &str,
    flow_path: &str,
) -> anyhow::Result<String> {
    let url = format!(
        "http://localhost:{}/api/w/test-workspace/apps_u/execute_component/{}",
        port, app_path
    );
    let resp = authed(client().post(&url), token)
        .json(&json!({
            "args": {},
            "component": "run_flow",
            "path": flow_path
        }))
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!(
            "app flow run failed: {} - {}",
            resp.status(),
            resp.text().await?
        );
    }
    let job_id = resp.text().await?;
    wait_for_job_result(port, token, &job_id).await
}

fn find_inline_script_id(value: &serde_json::Value) -> Option<i64> {
    match value {
        serde_json::Value::Object(m) => m
            .get("inlineScript")
            .and_then(|s| s.get("id"))
            .and_then(|id| id.as_i64())
            .or_else(|| m.values().find_map(find_inline_script_id)),
        serde_json::Value::Array(a) => a.iter().find_map(find_inline_script_id),
        _ => None,
    }
}

/// Once the app's dependency job writes the deployed lite value, inline code runs only through
/// the `app_script` rows it references, so the run must carry that row's id.
async fn deployed_inline_script_id(port: u16, app_path: &str) -> anyhow::Result<i64> {
    let url = format!(
        "http://localhost:{}/api/w/test-workspace/apps/get/lite/{}",
        port, app_path
    );
    for _ in 0..300 {
        let resp = authed(client().get(&url), SAME_WS_TOKEN).send().await?;
        if resp.status().is_success() {
            let app = resp.json::<serde_json::Value>().await?;
            if let Some(id) = find_inline_script_id(&app["value"]) {
                return Ok(id);
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    anyhow::bail!("timeout waiting for the deployed app_script id of {app_path}")
}

async fn run_inline_script(
    port: u16,
    token: &str,
    app_path: &str,
    force_viewer: bool,
) -> anyhow::Result<String> {
    let url = format!(
        "http://localhost:{}/api/w/test-workspace/apps_u/execute_component/{}",
        port, app_path
    );
    let mut payload = json!({
        "args": {},
        "component": "get_email",
        "id": deployed_inline_script_id(port, app_path).await?,
        "raw_code": {
            "language": "bun",
            "content": GET_EMAIL_CONTENT,
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
        anyhow::bail!(
            "app inline script run failed: {} - {}",
            resp.status(),
            resp.text().await?
        );
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
            return Ok(resp
                .json::<serde_json::Value>()
                .await?
                .as_str()
                .unwrap_or("")
                .to_string());
        }
    }
    anyhow::bail!("timeout waiting for job result")
}

async fn run_direct(port: u16, token: &str, route: &str) -> anyhow::Result<String> {
    let url = format!(
        "http://localhost:{}/api/w/test-workspace/jobs/run/{}",
        port, route
    );
    let resp = authed(client().post(&url), token)
        .json(&json!({}))
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!(
            "run {route} failed: {} - {}",
            resp.status(),
            resp.text().await?
        );
    }
    let job_id = resp.text().await?;
    wait_for_job_result(port, token, &job_id).await
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base", "end_user_email"))]
async fn test_direct_run_wm_end_user_email(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    in_test_worker(
        Connection::Sql(db.clone()),
        async move {
            let result = run_direct(port, SAME_WS_TOKEN, "p/f/test/get_end_user_email").await?;
            assert_eq!(result, SAME_WS_EMAIL, "a script run should see its caller");

            let result =
                run_direct(port, SAME_WS_TOKEN, "f/f/test/get_end_user_email_flow").await?;
            assert_eq!(
                result, SAME_WS_EMAIL,
                "every flow step should see the flow's caller"
            );

            Ok::<(), anyhow::Error>(())
        },
        port,
    )
    .await?;

    Ok(())
}

/// A job's `WM_TOKEN` names the identity the job runs as, which differs from its end user when
/// the job runs on behalf of another account. A run it starts must keep the job's end user.
#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base", "end_user_email"))]
async fn test_job_token_run_keeps_end_user_email(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let calling_job = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO job_perms (job_id, email, username, is_admin, is_operator, folders, groups, workspace_id, end_user_email)
         VALUES ($1, 'test2@windmill.dev', 'test-user-2', false, false, '{}', '{}', 'test-workspace', $2)",
    )
    .bind(calling_job)
    .bind(SAME_WS_EMAIL)
    .execute(&db)
    .await?;
    let wm_token = windmill_common::auth::create_jwt_token(
        windmill_common::db::Authed {
            email: "test2@windmill.dev".to_string(),
            username: "test-user-2".to_string(),
            is_admin: false,
            is_operator: false,
            groups: vec!["all".to_string()],
            folders: vec![],
            scopes: None,
            token_prefix: None,
        },
        "test-workspace",
        3600,
        Some(calling_job),
        None,
        None,
        None,
    )
    .await?;

    in_test_worker(
        Connection::Sql(db.clone()),
        async move {
            let result = run_direct(port, &wm_token, "p/f/test/get_end_user_email").await?;
            assert_eq!(
                result, SAME_WS_EMAIL,
                "the calling job's end user, not its run-as identity"
            );
            Ok::<(), anyhow::Error>(())
        },
        port,
    )
    .await?;

    Ok(())
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base", "end_user_email"))]
async fn test_app_wm_end_user_email(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let app_path = "f/test/email_app";

    in_test_worker(
        Connection::Sql(db.clone()),
        async move {
            // Create the app with inline script first
            create_app_with_inline_script(port, app_path).await?;

            // Same workspace user (force_viewer mode works for workspace members)
            let result = run_inline_script(port, SAME_WS_TOKEN, app_path, true).await?;
            assert_eq!(
                result, SAME_WS_EMAIL,
                "same workspace user should get their email"
            );

            // Other workspace user (uses app's anonymous policy + token lookup)
            let result = run_inline_script(port, OTHER_WS_TOKEN, app_path, false).await?;
            assert_eq!(
                result, OTHER_WS_EMAIL,
                "other workspace user should get their email"
            );

            // No workspace user (uses app's anonymous policy + token lookup)
            let result = run_inline_script(port, NO_WS_TOKEN, app_path, false).await?;
            assert_eq!(
                result, NO_WS_EMAIL,
                "no workspace user should get their email"
            );

            Ok::<(), anyhow::Error>(())
        },
        port,
    )
    .await?;

    Ok(())
}

/// Every flow step must see the same end user as the flow itself: `end_user_email` is only
/// stamped on the job the app pushes, so it has to be forwarded down to each step. The
/// fixture flow has two steps on purpose - the first one is pushed from the freshly pulled
/// flow job, every later one from a flow job re-fetched without its `job_perms`.
#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base", "end_user_email"))]
async fn test_app_flow_step_wm_end_user_email(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let app_path = "f/test/email_flow_app";
    let flow_path = "flow/f/test/get_end_user_email_flow";

    in_test_worker(
        Connection::Sql(db.clone()),
        async move {
            create_app_triggering_flow(port, app_path, flow_path).await?;

            // The flow result is its last step's result.
            let result = run_app_flow(port, OTHER_WS_TOKEN, app_path, flow_path).await?;
            assert_eq!(
                result, OTHER_WS_EMAIL,
                "every flow step should see the end user email, not the app publisher's"
            );

            Ok::<(), anyhow::Error>(())
        },
        port,
    )
    .await?;

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

    in_test_worker(
        Connection::Sql(db.clone()),
        async move {
            // Create the raw app with inline script first
            create_raw_app_with_inline_script(port, app_path).await?;

            // Same workspace user (force_viewer mode works for workspace members)
            let result = run_inline_script(port, SAME_WS_TOKEN, app_path, true).await?;
            assert_eq!(
                result, SAME_WS_EMAIL,
                "same workspace user should get their email"
            );

            // Other workspace user (uses app's anonymous policy + token lookup)
            let result = run_inline_script(port, OTHER_WS_TOKEN, app_path, false).await?;
            assert_eq!(
                result, OTHER_WS_EMAIL,
                "other workspace user should get their email"
            );

            // No workspace user (uses app's anonymous policy + token lookup)
            let result = run_inline_script(port, NO_WS_TOKEN, app_path, false).await?;
            assert_eq!(
                result, NO_WS_EMAIL,
                "no workspace user should get their email"
            );

            Ok::<(), anyhow::Error>(())
        },
        port,
    )
    .await?;

    Ok(())
}

/// Connecting to data tables as the viewer is honored on inline runnables only: a path runnable
/// is someone else's script, which must not read with the viewer's roles because the app's
/// policy entry for it says so.
#[sqlx::test(fixtures("base", "end_user_email"))]
async fn test_app_viewer_data_access_is_stamped_on_inline_runnables_only(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let app_path = "f/test/viewer_data_access_app";
    let script_path = "script/f/test/get_end_user_email";

    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/test-workspace/apps/create"
        )),
        SAME_WS_TOKEN,
    )
    .json(&json!({
        "path": app_path,
        "summary": "",
        "value": { "type": "app", "grid": [], "subgrids": {}, "hiddenInlineScripts": [] },
        "policy": {
            "execution_mode": "anonymous",
            "on_behalf_of": null,
            "on_behalf_of_email": null,
            "triggerables_v2": {
                "rawscript/894d2f85ea2411e777e7d5787366c9fec4c1a7a5f83482748f1d129196c9f54f": {
                    "static_inputs": {}, "one_of_inputs": {}, "viewer_data_access": true
                },
                (script_path): {
                    "static_inputs": {}, "one_of_inputs": {}, "viewer_data_access": true
                }
            }
        }
    }))
    .send()
    .await?;
    anyhow::ensure!(
        resp.status().is_success(),
        "create app: {}",
        resp.text().await?
    );

    let execute = |payload: serde_json::Value| async move {
        let resp = authed(
            client().post(format!(
                "http://localhost:{port}/api/w/test-workspace/apps_u/execute_component/{app_path}"
            )),
            SAME_WS_TOKEN,
        )
        .json(&payload)
        .send()
        .await?;
        anyhow::ensure!(
            resp.status().is_success(),
            "execute: {}",
            resp.text().await?
        );
        Ok::<_, anyhow::Error>(resp.text().await?.trim_matches('"').parse::<uuid::Uuid>()?)
    };
    let as_viewer = |job_id: uuid::Uuid| {
        let db = db.clone();
        async move {
            sqlx::query_scalar::<_, bool>(
                "SELECT viewer_data_access FROM job_perms WHERE job_id = $1",
            )
            .bind(job_id)
            .fetch_one(&db)
            .await
        }
    };

    let inline = execute(json!({
        "args": {},
        "component": "get_email",
        "raw_code": {
            "language": "bun",
            "content": "export function main() { return process.env.WM_END_USER_EMAIL || \"\"; }"
        }
    }))
    .await?;
    assert!(as_viewer(inline).await?);

    let path =
        execute(json!({ "args": {}, "component": "run_script", "path": script_path })).await?;
    assert!(
        !as_viewer(path).await?,
        "a path runnable connected as the viewer because its policy entry said so"
    );
    Ok(())
}
