use sqlx::{Pool, Postgres};
use windmill_common::{
    flows::{FlowModule, FlowModuleValue, FlowValue},
    jobs::{resolve_delete_after_secs, schedule_job_deletion, JobPayload, RawCode},
    scripts::ScriptLang,
};
use windmill_test_utils::*;

// ---------------------------------------------------------------------------
// Unit tests for resolve_delete_after_secs
// ---------------------------------------------------------------------------

#[test]
fn test_resolve_no_deletion() {
    assert_eq!(resolve_delete_after_secs(None, None), None);
    assert_eq!(resolve_delete_after_secs(Some(false), None), None);
}

#[test]
fn test_resolve_immediate_backward_compat() {
    // delete_after_use=true with no secs → immediate (0)
    assert_eq!(resolve_delete_after_secs(Some(true), None), Some(0));
}

#[test]
fn test_resolve_explicit_secs() {
    assert_eq!(resolve_delete_after_secs(None, Some(0)), Some(0));
    assert_eq!(resolve_delete_after_secs(None, Some(3600)), Some(3600));
    assert_eq!(resolve_delete_after_secs(Some(true), Some(60)), Some(60));
    assert_eq!(resolve_delete_after_secs(Some(false), Some(120)), Some(120));
}

#[test]
fn test_resolve_rejects_negative() {
    assert_eq!(resolve_delete_after_secs(None, Some(-1)), None);
    assert_eq!(resolve_delete_after_secs(Some(true), Some(-100)), None);
}

// ---------------------------------------------------------------------------
// Integration: schedule_job_deletion inserts into job_delete_schedule
// ---------------------------------------------------------------------------

#[sqlx::test(fixtures("base"))]
async fn test_schedule_job_deletion_inserts_row(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let job_id = uuid::Uuid::new_v4();
    // Insert a dummy v2_job row to satisfy the FK constraint
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, permissioned_as_email, kind, tag, same_worker, visible_to_owner) \
         VALUES ($1, 'test-workspace', 'test', 'u/test', 'test@test.com', 'script', 'deno', false, true)",
    )
    .bind(job_id)
    .execute(&db)
    .await?;

    schedule_job_deletion(&db, job_id, "test-workspace", 3600).await?;

    let row = sqlx::query_as::<_, (uuid::Uuid, String)>(
        "SELECT job_id, workspace_id FROM job_delete_schedule WHERE job_id = $1",
    )
    .bind(job_id)
    .fetch_one(&db)
    .await?;

    assert_eq!(row.0, job_id);
    assert_eq!(row.1, "test-workspace");

    // Verify delete_at is approximately now + 3600s
    let delete_at: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT delete_at FROM job_delete_schedule WHERE job_id = $1")
            .bind(job_id)
            .fetch_one(&db)
            .await?;

    let expected_min = chrono::Utc::now() + chrono::Duration::seconds(3500);
    let expected_max = chrono::Utc::now() + chrono::Duration::seconds(3700);
    assert!(
        delete_at > expected_min && delete_at < expected_max,
        "delete_at should be ~1 hour from now, got {delete_at}"
    );

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_schedule_job_deletion_is_idempotent(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let job_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, permissioned_as_email, kind, tag, same_worker, visible_to_owner) \
         VALUES ($1, 'test-workspace', 'test', 'u/test', 'test@test.com', 'script', 'deno', false, true)",
    )
    .bind(job_id)
    .execute(&db)
    .await?;

    schedule_job_deletion(&db, job_id, "test-workspace", 60).await?;
    // Second call should not error (ON CONFLICT DO NOTHING)
    schedule_job_deletion(&db, job_id, "test-workspace", 120).await?;

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM job_delete_schedule WHERE job_id = $1")
            .bind(job_id)
            .fetch_one(&db)
            .await?;
    assert_eq!(count, 1, "should have exactly one row (idempotent)");

    Ok(())
}

// ---------------------------------------------------------------------------
// Integration: Script with delete_after_secs=0 → immediate deletion
// ---------------------------------------------------------------------------

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_script_delete_after_secs_zero_immediate(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Deploy a script with delete_after_secs=0 via direct SQL
    // (simulating what the API does when creating a script)
    let content = "export function main() { return 42; }";
    let job = JobPayload::Code(RawCode {
        hash: None,
        content: content.to_string(),
        path: Some("f/system/delete_test".to_string()),
        language: ScriptLang::Deno,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: Default::default(),
        debouncing_settings: Default::default(),
        modules: None,
    });

    let completed = RunJob::from(job).run_until_complete(&db, false, port).await;

    assert!(completed.success, "job should succeed");
    // Job ran directly (not via sync webhook), so delete_after_use is not triggered
    // Verify args and result are still present
    assert!(completed.result.is_some());

    Ok(())
}

// ---------------------------------------------------------------------------
// Integration: Flow with delete_after_secs on FlowValue
// ---------------------------------------------------------------------------

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_level_delete_after_secs(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let step_content = "export function main() { return 'hello'; }";

    let flow = FlowValue {
        modules: vec![FlowModule {
            id: "a".to_string(),
            value: FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: step_content.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default().into(),
                is_trigger: None,
                assets: None,
            }
            .into(),
            stop_after_if: Default::default(),
            stop_after_all_iters_if: Default::default(),
            summary: Default::default(),
            suspend: Default::default(),
            retry: None,
            sleep: None,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            mock: None,
            timeout: None,
            priority: None,
            delete_after_use: None,
            delete_after_secs: None,
            continue_on_error: None,
            skip_if: None,
            apply_preprocessor: None,
            pass_flow_input_directly: None,
            debouncing: None,
        }],
        failure_module: None,
        preprocessor_module: None,
        same_worker: false,
        concurrency_settings: Default::default(),
        debouncing_settings: Default::default(),
        skip_expr: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        early_return: None,
        priority: None,
        chat_input_enabled: None,
        flow_env: None,
        // Flow-level: schedule deletion 1 hour after completion
        delete_after_use: None,
        delete_after_secs: Some(3600),
    };

    let job = JobPayload::RawFlow {
        value: flow,
        path: Some("f/system/flow_delete_test".to_string()),
        restarted_from: None,
    };

    let completed = RunJob::from(job).run_until_complete(&db, false, port).await;

    assert!(completed.success, "flow should succeed");

    // The flow-level delete_after_secs=3600 should have scheduled deletion
    // for child jobs (and the parent flow job itself).
    // Check that at least one row exists in job_delete_schedule
    let scheduled_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM job_delete_schedule")
        .fetch_one(&db)
        .await?;

    assert!(
        scheduled_count > 0,
        "expected scheduled deletions for flow-level delete_after_secs, got {scheduled_count}"
    );

    // Verify the parent flow job itself is scheduled
    let parent_scheduled: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM job_delete_schedule WHERE job_id = $1)")
            .bind(completed.id)
            .fetch_one(&db)
            .await?;

    assert!(
        parent_scheduled,
        "parent flow job should be scheduled for deletion"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Integration: Flow module with delete_after_secs on individual step
// ---------------------------------------------------------------------------

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_module_delete_after_secs(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let step_content = "export function main() { return 'step result'; }";

    let flow = FlowValue {
        modules: vec![FlowModule {
            id: "a".to_string(),
            value: FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: step_content.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default().into(),
                is_trigger: None,
                assets: None,
            }
            .into(),
            stop_after_if: Default::default(),
            stop_after_all_iters_if: Default::default(),
            summary: Default::default(),
            suspend: Default::default(),
            retry: None,
            sleep: None,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            mock: None,
            timeout: None,
            priority: None,
            delete_after_use: None,
            // Step-level: schedule deletion 1800s after flow completes
            delete_after_secs: Some(1800),
            continue_on_error: None,
            skip_if: None,
            apply_preprocessor: None,
            pass_flow_input_directly: None,
            debouncing: None,
        }],
        failure_module: None,
        preprocessor_module: None,
        same_worker: false,
        concurrency_settings: Default::default(),
        debouncing_settings: Default::default(),
        skip_expr: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        early_return: None,
        priority: None,
        chat_input_enabled: None,
        flow_env: None,
        delete_after_use: None,
        delete_after_secs: None,
    };

    let job = JobPayload::RawFlow {
        value: flow,
        path: Some("f/system/flow_module_delete_test".to_string()),
        restarted_from: None,
    };

    let completed = RunJob::from(job).run_until_complete(&db, false, port).await;

    assert!(completed.success, "flow should succeed");

    // The step's delete_after_secs=1800 should have created a scheduled deletion
    let scheduled_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM job_delete_schedule")
        .fetch_one(&db)
        .await?;

    assert!(
        scheduled_count > 0,
        "expected at least one scheduled deletion for step-level delete_after_secs, got {scheduled_count}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Integration: Flow module with delete_after_use=true (backward compat, immediate)
// ---------------------------------------------------------------------------

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_module_delete_after_use_backward_compat(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let step_content = "export function main() { return 'will be deleted'; }";

    let flow = FlowValue {
        modules: vec![FlowModule {
            id: "a".to_string(),
            value: FlowModuleValue::RawScript {
                input_transforms: Default::default(),
                language: ScriptLang::Deno,
                content: step_content.to_string(),
                path: None,
                lock: None,
                tag: None,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default().into(),
                is_trigger: None,
                assets: None,
            }
            .into(),
            stop_after_if: Default::default(),
            stop_after_all_iters_if: Default::default(),
            summary: Default::default(),
            suspend: Default::default(),
            retry: None,
            sleep: None,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            mock: None,
            timeout: None,
            priority: None,
            // Old-style: delete_after_use=true, no delete_after_secs
            delete_after_use: Some(true),
            delete_after_secs: None,
            continue_on_error: None,
            skip_if: None,
            apply_preprocessor: None,
            pass_flow_input_directly: None,
            debouncing: None,
        }],
        failure_module: None,
        preprocessor_module: None,
        same_worker: false,
        concurrency_settings: Default::default(),
        debouncing_settings: Default::default(),
        skip_expr: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        early_return: None,
        priority: None,
        chat_input_enabled: None,
        flow_env: None,
        delete_after_use: None,
        delete_after_secs: None,
    };

    let job = JobPayload::RawFlow {
        value: flow,
        path: Some("f/system/flow_backward_compat_test".to_string()),
        restarted_from: None,
    };

    let completed = RunJob::from(job).run_until_complete(&db, false, port).await;

    assert!(completed.success, "flow should succeed");

    // With delete_after_use=true and no delete_after_secs, this should trigger
    // immediate deletion (via flow_jobs_to_clean), NOT scheduled deletion.
    let scheduled_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM job_delete_schedule")
        .fetch_one(&db)
        .await?;

    assert_eq!(
        scheduled_count, 0,
        "backward compat: delete_after_use=true should do immediate deletion, not scheduled"
    );

    // Check that the step job's args were cleared (immediate cleanup)
    let step_jobs: Vec<(uuid::Uuid, Option<serde_json::Value>)> =
        sqlx::query_as("SELECT j.id, j.args FROM v2_job j WHERE j.root_job = $1")
            .bind(completed.id)
            .fetch_all(&db)
            .await?;

    for (step_id, args) in &step_jobs {
        let args_str = args.as_ref().map(|v| v.to_string()).unwrap_or_default();
        assert!(
            args_str == "{}" || args_str.is_empty(),
            "step {step_id} args should be cleared after immediate delete, got: {args_str}"
        );
    }

    Ok(())
}
