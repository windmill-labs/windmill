#![cfg(all(feature = "enterprise", feature = "private"))]

use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

const ERROR_HANDLER_EMAIL: &str = "error_handler@windmill.dev";
const SUCCESS_HANDLER_EMAIL: &str = "success_handler@windmill.dev";

/// Every test seeds `f/test/error_handler` or `f/test/success_handler` with a different hash,
/// each in its own database, while the deployed-script caches are process-global and keyed by
/// workspace and path. With them on, one test resolves the handler to another's hash and the
/// handler push 404s whenever that hash's info is not cached yet.
fn disable_deployed_script_cache() {
    windmill_common::DEPLOYED_SCRIPT_CACHE_DISABLED
        .store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Polls so the assertion does not depend on the handler being pushed before
/// `run_until_complete` returns, which only holds while the worker drains its completions on exit.
async fn wait_for_handler_job(
    db: &Pool<Postgres>,
    handler_email: &str,
) -> anyhow::Result<Option<(Option<String>, Option<uuid::Uuid>)>> {
    for _ in 0..100 {
        let job = sqlx::query_as::<_, (Option<String>, Option<uuid::Uuid>)>(
            "SELECT runnable_path, parent_job FROM v2_job
             WHERE workspace_id = 'test-workspace' AND permissioned_as_email = $1",
        )
        .bind(handler_email)
        .fetch_optional(db)
        .await?;
        if job.is_some() {
            return Ok(job);
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    Ok(None)
}

#[cfg(feature = "deno_core")]
async fn insert_bun_script(
    db: &Pool<Postgres>,
    hash: i64,
    path: &str,
    content: &str,
    ws_error_handler_muted: bool,
) -> anyhow::Result<()> {
    // The lock must be non-null for the script to count as deployed.
    sqlx::query(
        r#"INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, ws_error_handler_muted)
           VALUES ('test-workspace', $1, $2, $3, 'bun', 'script', 'test-user', '{}', '', '', E'{}\n//bun.lock\n<empty>', $4)"#,
    )
    .bind(hash)
    .bind(path)
    .bind(content)
    .bind(ws_error_handler_muted)
    .execute(db)
    .await?;
    Ok(())
}

/// Configures `f/test/error_handler` and `f/test/success_handler` as the workspace handlers.
/// Both settings are cached per workspace id across the tests of this binary, and any test's
/// top-level job can fill either cache, so every test configures both, with the same paths.
async fn configure_workspace_handlers(db: &Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        r#"UPDATE workspace_settings
           SET error_handler = '{"path": "script/f/test/error_handler"}'::jsonb,
               success_handler = '{"path": "script/f/test/success_handler"}'::jsonb
           WHERE workspace_id = 'test-workspace'"#,
    )
    .execute(db)
    .await?;
    sqlx::query(
        r#"INSERT INTO group_ (workspace_id, name, summary, extra_perms)
           VALUES ('test-workspace', 'error_handler', '', '{"u/test-user": true}'),
                  ('test-workspace', 'success_handler', '', '{"u/test-user": true}')
           ON CONFLICT DO NOTHING"#,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Runs a deployed script by hash, which makes a `script` job rather than a preview.
#[cfg(feature = "deno_core")]
async fn run_script_hash(db: &Pool<Postgres>, port: u16, hash: i64, path: &str) -> CompletedJob {
    use windmill_common::jobs::JobPayload;
    use windmill_common::runnable_settings::{ConcurrencySettings, DebouncingSettings};
    use windmill_common::scripts::{ScriptHash, ScriptLang};

    RunJob::from(JobPayload::ScriptHash {
        hash: ScriptHash(hash),
        path: path.to_string(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        language: ScriptLang::Bun,
        priority: None,
        apply_preprocessor: false,
        concurrency_settings: ConcurrencySettings::default(),
        debouncing_settings: DebouncingSettings::default(),
        labels: None,
        job_token_scopes: None,
    })
    .run_until_complete(db, false, port)
    .await
}

/// A job's handler, if any, is pushed before the worker that ran it exits, so once a control
/// job has been handled, every handler job that the jobs run before it would produce exists.
#[cfg(feature = "deno_core")]
async fn assert_only_handled(
    db: &Pool<Postgres>,
    control_job: uuid::Uuid,
    message: &str,
) -> anyhow::Result<()> {
    assert!(
        wait_for_handler_job(db, ERROR_HANDLER_EMAIL)
            .await?
            .is_some(),
        "Error handler should have handled the control failure"
    );
    let handled = sqlx::query_scalar::<_, Option<uuid::Uuid>>(
        "SELECT parent_job FROM v2_job
         WHERE workspace_id = 'test-workspace' AND permissioned_as_email = $1",
    )
    .bind(ERROR_HANDLER_EMAIL)
    .fetch_all(db)
    .await?;
    assert_eq!(handled, vec![Some(control_job)], "{message}");
    Ok(())
}

/// Test that a failed job triggers the workspace error handler
#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_error_handler_triggered_on_failure(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    disable_deployed_script_cache();

    let server = ApiServer::start(db.clone()).await?;

    insert_bun_script(
        &db,
        1111111111,
        "f/test/error_handler",
        r#"
export async function main(path: string, email: string, job_id: string, is_flow: boolean, workspace_id: string, error: any) {
    console.log("Error handler called for job:", job_id);
    return { handled: true, original_path: path };
}
"#,
        false,
    )
    .await?;
    insert_bun_script(
        &db,
        2222222222,
        "f/test/failing_script",
        "export function main() { throw new Error('intentional failure'); }",
        false,
    )
    .await?;
    configure_workspace_handlers(&db).await?;

    let completed_job =
        run_script_hash(&db, server.addr.port(), 2222222222, "f/test/failing_script").await;
    assert!(!completed_job.success, "Job should have failed");

    assert_eq!(
        wait_for_handler_job(&db, ERROR_HANDLER_EMAIL).await?,
        Some((
            Some("f/test/error_handler".to_string()),
            Some(completed_job.id)
        )),
        "Error handler should run the configured script with the failed job as parent"
    );

    Ok(())
}

/// Test that error handler is NOT triggered when ws_error_handler_muted is set on the script
#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_error_handler_muted_on_script(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    disable_deployed_script_cache();

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    insert_bun_script(
        &db,
        3333333333,
        "f/test/error_handler",
        r#"export function main() { return "handled"; }"#,
        false,
    )
    .await?;
    let failing = r#"export function main() { throw new Error("fail"); }"#;
    insert_bun_script(
        &db,
        4444444444,
        "f/test/muted_failing_script",
        failing,
        true,
    )
    .await?;
    insert_bun_script(&db, 4444444445, "f/test/failing_script", failing, false).await?;
    configure_workspace_handlers(&db).await?;

    let muted_job = run_script_hash(&db, port, 4444444444, "f/test/muted_failing_script").await;
    assert!(!muted_job.success, "Job should have failed");

    let control_job = run_script_hash(&db, port, 4444444445, "f/test/failing_script").await;
    assert!(!control_job.success, "Job should have failed");

    assert_only_handled(
        &db,
        control_job.id,
        "Error handler should NOT have been triggered for a muted script",
    )
    .await
}

/// Test that error handler is NOT triggered on successful job completion
#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_error_handler_not_triggered_on_success(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    disable_deployed_script_cache();

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    insert_bun_script(
        &db,
        5555555555,
        "f/test/error_handler",
        r#"export function main() { return "handled"; }"#,
        false,
    )
    .await?;
    insert_bun_script(
        &db,
        6666666666,
        "f/test/success_script",
        r#"export function main() { return "ok"; }"#,
        false,
    )
    .await?;
    insert_bun_script(
        &db,
        6666666667,
        "f/test/failing_script",
        r#"export function main() { throw new Error("fail"); }"#,
        false,
    )
    .await?;
    configure_workspace_handlers(&db).await?;

    let success_job = run_script_hash(&db, port, 6666666666, "f/test/success_script").await;
    assert!(success_job.success, "Job should have succeeded");

    let control_job = run_script_hash(&db, port, 6666666667, "f/test/failing_script").await;
    assert!(!control_job.success, "Job should have failed");

    assert_only_handled(
        &db,
        control_job.id,
        "Error handler should NOT have been triggered for a successful job",
    )
    .await
}

/// A hub script run at the top level (a schedule, a trigger, the API) is a `script_hub` job,
/// and its failure must reach the workspace error handler like a workspace script's.
#[sqlx::test(fixtures("base"))]
async fn test_error_handler_triggered_on_hub_script_failure(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    use windmill_common::jobs::JobPayload;

    initialize_tracing().await;
    disable_deployed_script_cache();

    let server = ApiServer::start(db.clone()).await?;

    let version = "990000002";
    let hub_dir = &*windmill_common::worker::HUB_CACHE_DIR;
    tokio::fs::create_dir_all(hub_dir).await?;
    tokio::fs::write(
        format!("{hub_dir}/{version}"),
        r#"{"content":"exit 1","lockfile":null,"language":"bash","schema":{},"summary":null}"#,
    )
    .await?;

    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock)
         VALUES ('test-workspace', 1111111112, 'f/test/error_handler', 'echo handled', 'bash', 'script', 'test-user', '{}', '', '', '')",
    )
    .execute(&db)
    .await?;
    configure_workspace_handlers(&db).await?;

    let completed_job = RunJob::from(JobPayload::ScriptHub {
        path: format!("hub/{version}/test/fail"),
        apply_preprocessor: false,
    })
    .run_until_complete(&db, false, server.addr.port())
    .await;
    assert!(!completed_job.success, "Hub script should have failed");

    let handler_job = wait_for_handler_job(&db, ERROR_HANDLER_EMAIL).await?;
    assert_eq!(
        handler_job.map(|(_, parent)| parent),
        Some(Some(completed_job.id)),
        "Workspace error handler should handle the failed hub script"
    );

    Ok(())
}

/// Test that a successful job completion triggers the success handler
#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_success_handler_triggered_on_success(db: Pool<Postgres>) -> anyhow::Result<()> {
    use serde_json::json;

    initialize_tracing().await;
    disable_deployed_script_cache();

    let server = ApiServer::start(db.clone()).await?;

    insert_bun_script(
        &db,
        1234567890,
        "f/test/success_handler",
        r#"
export async function main(path: string, email: string, job_id: string, is_flow: boolean, workspace_id: string, result: any) {
    console.log("Success handler called for job:", job_id);
    return { handled: true, original_path: path };
}
"#,
        false,
    )
    .await?;
    insert_bun_script(
        &db,
        9876543210,
        "f/test/simple_script",
        "export function main() { return 'success'; }",
        false,
    )
    .await?;
    configure_workspace_handlers(&db).await?;

    let completed_job =
        run_script_hash(&db, server.addr.port(), 9876543210, "f/test/simple_script").await;
    assert_eq!(completed_job.json_result().unwrap(), json!("success"));

    assert_eq!(
        wait_for_handler_job(&db, SUCCESS_HANDLER_EMAIL).await?,
        Some((
            Some("f/test/success_handler".to_string()),
            Some(completed_job.id)
        )),
        "Success handler should run the configured script with the main job as parent"
    );

    Ok(())
}

/// A hub script run at the top level (a schedule, a trigger, the API) is a `script_hub` job,
/// and its success must reach the workspace success handler like a workspace script's.
#[sqlx::test(fixtures("base"))]
async fn test_success_handler_triggered_on_hub_script_success(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    use windmill_common::jobs::JobPayload;

    initialize_tracing().await;
    disable_deployed_script_cache();

    let server = ApiServer::start(db.clone()).await?;

    let version = "990000003";
    let hub_dir = &*windmill_common::worker::HUB_CACHE_DIR;
    tokio::fs::create_dir_all(hub_dir).await?;
    tokio::fs::write(
        format!("{hub_dir}/{version}"),
        r#"{"content":"echo ok","lockfile":null,"language":"bash","schema":{},"summary":null}"#,
    )
    .await?;

    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock)
         VALUES ('test-workspace', 1234567891, 'f/test/success_handler', 'echo handled', 'bash', 'script', 'test-user', '{}', '', '', '')",
    )
    .execute(&db)
    .await?;
    configure_workspace_handlers(&db).await?;

    let completed_job = RunJob::from(JobPayload::ScriptHub {
        path: format!("hub/{version}/test/ok"),
        apply_preprocessor: false,
    })
    .run_until_complete(&db, false, server.addr.port())
    .await;
    assert!(completed_job.success, "Hub script should have succeeded");

    let handler_job = wait_for_handler_job(&db, SUCCESS_HANDLER_EMAIL).await?;
    assert_eq!(
        handler_job.map(|(_, parent)| parent),
        Some(Some(completed_job.id)),
        "Workspace success handler should handle the successful hub script"
    );

    Ok(())
}
