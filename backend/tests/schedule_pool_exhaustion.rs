use sqlx::{Pool, Postgres};
use windmill_common::DEFAULT_MAX_CONNECTIONS_WORKER;

mod common;
use common::*;

// =============================================================================
// Permanent regression test — uses the real default worker pool size
// =============================================================================

/// Regression test: schedule push must succeed with the default worker pool size
/// (DEFAULT_MAX_CONNECTIONS_WORKER = 5, NUM_WORKERS = 1) while 1 connection is
/// held for the outer transaction in commit_completed_job — the normal scenario
/// every time a scheduled job finishes.
#[sqlx::test(fixtures("base"))]
async fn test_schedule_push_default_worker_pool(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    // Default worker pool: DEFAULT_MAX_CONNECTIONS_WORKER + NUM_WORKERS - 1 = 5
    let num_workers: u32 = 1;
    let pool_size = DEFAULT_MAX_CONNECTIONS_WORKER + num_workers - 1;
    let pool = create_constrained_pool(&db, pool_size).await?;
    setup_schedule_infrastructure(&db).await?;

    let schedule = get_test_schedule(&db).await?;

    // Hold 1 connection: the outer tx from commit_completed_job (always present)
    let _outer_tx = pool.acquire().await?;

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        windmill_queue::handle_maybe_scheduled_job(
            &pool,
            &make_completed_job(),
            &schedule,
            "f/test/schedule_script",
            "test-workspace",
        ),
    )
    .await
    .expect("must not timeout with default worker pool");

    assert!(result.is_ok(), "push must succeed: {result:?}");
    assert_schedule_enabled(&db).await;

    Ok(())
}

// =============================================================================
// Experiments: finding the breaking point for pool = 5
// =============================================================================

/// Pool = 5, 2 held (outer tx + 1 background op) → 3 available → should succeed.
#[sqlx::test(fixtures("base"))]
async fn test_pool5_2held_succeeds(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let pool = create_constrained_pool(&db, 5).await?;
    setup_schedule_infrastructure(&db).await?;
    let schedule = get_test_schedule(&db).await?;

    let _held = acquire_n(&pool, 2).await;

    let result = try_schedule_push(&pool, &schedule).await;
    assert!(result.is_ok(), "pool=5, 2 held: should succeed");
    assert_schedule_enabled(&db).await;
    Ok(())
}

/// Pool = 5, 3 held (outer tx + 2 background ops) → 2 available → should succeed.
/// handle_maybe_scheduled_job needs exactly 2 (inner tx + 1 pool query at a time).
#[sqlx::test(fixtures("base"))]
async fn test_pool5_3held_succeeds(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let pool = create_constrained_pool(&db, 5).await?;
    setup_schedule_infrastructure(&db).await?;
    let schedule = get_test_schedule(&db).await?;

    let _held = acquire_n(&pool, 3).await;

    let result = try_schedule_push(&pool, &schedule).await;
    assert!(result.is_ok(), "pool=5, 3 held: should succeed");
    assert_schedule_enabled(&db).await;
    Ok(())
}

/// Pool = 5, 4 held → only 1 available → should still succeed.
/// Pre-computing Authed before the inner tx means push() only needs
/// 1 connection (the inner tx) during the transaction scope.
#[sqlx::test(fixtures("base"))]
async fn test_pool5_4held_succeeds(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let pool = create_constrained_pool(&db, 5).await?;
    setup_schedule_infrastructure(&db).await?;
    let schedule = get_test_schedule(&db).await?;

    let _held = acquire_n(&pool, 4).await;

    let result = try_schedule_push(&pool, &schedule).await;
    assert!(result.is_ok(), "pool=5, 4 held: should succeed after fix");
    assert_schedule_enabled(&db).await;
    Ok(())
}

/// Pool = 5, two concurrent schedule pushes each holding an outer tx.
/// Push A: outer tx (1) + inner tx (1) + pool query (1) = 3 at peak.
/// Push B: outer tx (1) + inner tx (1) + pool query (1) = 3 at peak.
/// Worst-case overlap: 6 connections, pool has 5 → contention.
#[sqlx::test(fixtures("base"))]
async fn test_pool5_two_concurrent_pushes(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let pool = create_constrained_pool(&db, 5).await?;
    setup_schedule_infrastructure(&db).await?;
    // Insert a second schedule
    setup_second_schedule(&db).await?;

    let schedule_a = get_test_schedule(&db).await?;
    let schedule_b = windmill_queue::schedule::get_schedule_opt(
        &db,
        "test-workspace",
        "f/test/test_schedule_2",
    )
    .await?
    .expect("schedule_b should exist");

    // Each push also has an outer tx from commit_completed_job
    let _outer_a = pool.acquire().await?;
    let _outer_b = pool.acquire().await?;
    // 3 connections remaining for 2 concurrent pushes that each need 2

    let job_a = make_completed_job();
    let job_b = make_completed_job_for("f/test/test_schedule_2");

    let push_a = windmill_queue::handle_maybe_scheduled_job(
        &pool,
        &job_a,
        &schedule_a,
        "f/test/schedule_script",
        "test-workspace",
    );
    let push_b = windmill_queue::handle_maybe_scheduled_job(
        &pool,
        &job_b,
        &schedule_b,
        "f/test/schedule_script",
        "test-workspace",
    );

    let timeout = std::time::Duration::from_secs(15);
    let (res_a, res_b) = tokio::join!(
        tokio::time::timeout(timeout, push_a),
        tokio::time::timeout(timeout, push_b),
    );

    let a_ok = matches!(res_a, Ok(Ok(())));
    let b_ok = matches!(res_b, Ok(Ok(())));

    let sched_a = windmill_queue::schedule::get_schedule_opt(
        &db,
        "test-workspace",
        "f/test/test_schedule",
    )
    .await?
    .unwrap();
    let sched_b = windmill_queue::schedule::get_schedule_opt(
        &db,
        "test-workspace",
        "f/test/test_schedule_2",
    )
    .await?
    .unwrap();

    eprintln!("=== Two concurrent pushes on pool=5 (2 outer tx held) ===");
    eprintln!("Push A completed successfully: {a_ok}");
    eprintln!("Push B completed successfully: {b_ok}");
    eprintln!("Schedule A enabled: {}, error: {:?}", sched_a.enabled, sched_a.error);
    eprintln!("Schedule B enabled: {}, error: {:?}", sched_b.enabled, sched_b.error);

    if !a_ok || !b_ok || !sched_a.enabled || !sched_b.enabled {
        eprintln!("FINDING: pool=5 is too small for 2 concurrent schedule pushes + their outer txs");
    } else {
        eprintln!("Both pushes succeeded — pool=5 can handle 2 concurrent pushes with 2 outer txs");
    }

    Ok(())
}

// =============================================================================
// Pool = 1: proves authed is fully pre-computed outside tx
// =============================================================================

/// Pool = 1, 0 held → only 1 connection available. Pre-compute authed uses it
/// and releases, then tx uses it. push() needs 0 extra. Must succeed.
#[sqlx::test(fixtures("base"))]
async fn test_pool1_normal_case_succeeds(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let pool = create_constrained_pool(&db, 1).await?;
    setup_schedule_infrastructure(&db).await?;
    let schedule = get_test_schedule(&db).await?;

    let result = try_schedule_push(&pool, &schedule).await;
    assert!(result.is_ok(), "pool=1, 0 held: must succeed with pre-computed authed: {result:?}");
    assert_schedule_enabled(&db).await;
    Ok(())
}

// =============================================================================
// on_behalf_of_email tests
// =============================================================================

/// on_behalf_of_email, pool=5, 0 held → should succeed.
#[sqlx::test(fixtures("base"))]
async fn test_on_behalf_of_email_no_pressure(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let pool = create_constrained_pool(&db, 5).await?;
    setup_schedule_infrastructure(&db).await?;
    setup_on_behalf_of_email_infrastructure(&db).await?;
    let schedule = get_obo_schedule(&db).await?;

    let result = try_schedule_push_obo(&pool, &schedule).await;
    assert!(result.is_ok(), "on_behalf_of_email, pool=5, 0 held: {result:?}");
    assert_schedule_enabled_at(&db, "f/test/obo_schedule").await;
    Ok(())
}

/// on_behalf_of_email, pool=5, 2 held → 3 available, needs 2 peak → should succeed.
#[sqlx::test(fixtures("base"))]
async fn test_on_behalf_of_email_pool5_2held(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let pool = create_constrained_pool(&db, 5).await?;
    setup_schedule_infrastructure(&db).await?;
    setup_on_behalf_of_email_infrastructure(&db).await?;
    let schedule = get_obo_schedule(&db).await?;

    let _held = acquire_n(&pool, 2).await;

    let result = try_schedule_push_obo(&pool, &schedule).await;
    assert!(result.is_ok(), "on_behalf_of_email, pool=5, 2 held: {result:?}");
    assert_schedule_enabled_at(&db, "f/test/obo_schedule").await;
    Ok(())
}

/// on_behalf_of_email, pool=5, 3 held → 2 available, needs 2 peak → should succeed (tight).
#[sqlx::test(fixtures("base"))]
async fn test_on_behalf_of_email_pool5_3held(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let pool = create_constrained_pool(&db, 5).await?;
    setup_schedule_infrastructure(&db).await?;
    setup_on_behalf_of_email_infrastructure(&db).await?;
    let schedule = get_obo_schedule(&db).await?;

    let _held = acquire_n(&pool, 3).await;

    let result = try_schedule_push_obo(&pool, &schedule).await;
    assert!(result.is_ok(), "on_behalf_of_email, pool=5, 3 held: {result:?}");
    assert_schedule_enabled_at(&db, "f/test/obo_schedule").await;
    Ok(())
}

/// on_behalf_of_email, pool=1, 0 held → proves authed is fully pre-computed
/// outside the tx for on_behalf_of_email too. Only 1 connection needed (the tx).
#[sqlx::test(fixtures("base"))]
async fn test_on_behalf_of_email_pool1_succeeds(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let pool = create_constrained_pool(&db, 1).await?;
    setup_schedule_infrastructure(&db).await?;
    setup_on_behalf_of_email_infrastructure(&db).await?;
    let schedule = get_obo_schedule(&db).await?;

    let result = try_schedule_push_obo(&pool, &schedule).await;
    assert!(result.is_ok(), "on_behalf_of_email, pool=1, 0 held: {result:?}");
    assert_schedule_enabled_at(&db, "f/test/obo_schedule").await;
    Ok(())
}

// =============================================================================
// Edge cases: schedule disabled / path mismatch
// =============================================================================

/// Schedule disabled → handle_maybe_scheduled_job should not push any job.
#[sqlx::test(fixtures("base"))]
async fn test_schedule_disabled_no_push(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let pool = create_constrained_pool(&db, 5).await?;
    setup_schedule_infrastructure(&db).await?;

    // Disable the schedule
    sqlx::query("UPDATE schedule SET enabled = false WHERE workspace_id = 'test-workspace' AND path = 'f/test/test_schedule'")
        .execute(&db)
        .await?;

    let schedule = get_test_schedule(&db).await?;
    assert!(!schedule.enabled);

    let jobs_before = count_queued_jobs(&db).await;

    let result = windmill_queue::handle_maybe_scheduled_job(
        &pool,
        &make_completed_job(),
        &schedule,
        "f/test/schedule_script",
        "test-workspace",
    )
    .await;
    assert!(result.is_ok(), "disabled schedule should return Ok: {result:?}");

    let jobs_after = count_queued_jobs(&db).await;
    assert_eq!(jobs_before, jobs_after, "no job should be pushed for disabled schedule");
    Ok(())
}

/// Script path doesn't match schedule's script_path → should not push any job.
#[sqlx::test(fixtures("base"))]
async fn test_schedule_path_mismatch_no_push(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let pool = create_constrained_pool(&db, 5).await?;
    setup_schedule_infrastructure(&db).await?;
    let schedule = get_test_schedule(&db).await?;

    let jobs_before = count_queued_jobs(&db).await;

    // Pass a mismatched script_path
    let result = windmill_queue::handle_maybe_scheduled_job(
        &pool,
        &make_completed_job(),
        &schedule,
        "f/test/some_other_script",
        "test-workspace",
    )
    .await;
    assert!(result.is_ok(), "path mismatch should return Ok: {result:?}");

    let jobs_after = count_queued_jobs(&db).await;
    assert_eq!(jobs_before, jobs_after, "no job should be pushed for mismatched path");
    Ok(())
}

/// Verify the pushed job has correct trigger metadata.
#[sqlx::test(fixtures("base"))]
async fn test_pushed_job_has_schedule_trigger(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let pool = create_constrained_pool(&db, 5).await?;
    setup_schedule_infrastructure(&db).await?;
    let schedule = get_test_schedule(&db).await?;

    let jobs_before = count_queued_jobs(&db).await;

    let result = try_schedule_push(&pool, &schedule).await;
    assert!(result.is_ok(), "push should succeed: {result:?}");

    let jobs_after = count_queued_jobs(&db).await;
    assert_eq!(jobs_after, jobs_before + 1, "exactly one job should be pushed");

    // Verify trigger metadata
    let trigger: Option<String> = sqlx::query_scalar(
        "SELECT trigger FROM v2_job WHERE workspace_id = 'test-workspace' \
         AND trigger = 'f/test/test_schedule' ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(trigger.as_deref(), Some("f/test/test_schedule"));
    Ok(())
}

// =============================================================================
// Original reproduction tests (pool = 4)
// =============================================================================

/// Baseline: pool=4, no concurrent pressure → should succeed.
#[sqlx::test(fixtures("base"))]
async fn test_schedule_push_succeeds_with_4_connections_no_pressure(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let pool = create_constrained_pool(&db, 4).await?;
    setup_schedule_infrastructure(&db).await?;
    let schedule = get_test_schedule(&db).await?;

    let result = try_schedule_push(&pool, &schedule).await;
    assert!(result.is_ok(), "push should succeed with pool=4, no pressure");
    assert_schedule_enabled(&db).await;
    Ok(())
}

/// Pool=4, 3 held → 1 available → should now succeed after fix.
/// Previously reproduced the v1.614.0 regression where DATABASE_CONNECTIONS=4 (5-1) caused
/// schedules to disable themselves. Pre-computing Authed before the inner tx resolves this.
#[sqlx::test(fixtures("base"))]
async fn test_schedule_push_succeeds_with_4_connections_under_pressure(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let pool = create_constrained_pool(&db, 4).await?;
    setup_schedule_infrastructure(&db).await?;
    let schedule = get_test_schedule(&db).await?;

    // Hold 3: outer tx + PgListener (v1.614.0) + background op
    let _held = acquire_n(&pool, 3).await;

    let result = try_schedule_push(&pool, &schedule).await;
    assert!(result.is_ok(), "pool=4, 3 held: should succeed after fix");
    assert_schedule_enabled(&db).await;
    Ok(())
}

/// Pool=5, 3 held (same pressure as above) → 2 available → should succeed.
/// Proves the extra connection from pool=5 prevents the issue.
#[sqlx::test(fixtures("base"))]
async fn test_schedule_push_succeeds_with_5_connections_under_same_pressure(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let pool = create_constrained_pool(&db, 5).await?;
    setup_schedule_infrastructure(&db).await?;
    let schedule = get_test_schedule(&db).await?;

    let _held = acquire_n(&pool, 3).await;

    let result = try_schedule_push(&pool, &schedule).await;
    assert!(result.is_ok(), "push should succeed with pool=5, 3 held");
    assert_schedule_enabled(&db).await;
    Ok(())
}

// =============================================================================
// Helpers
// =============================================================================

async fn create_constrained_pool(
    db: &Pool<Postgres>,
    max_connections: u32,
) -> anyhow::Result<Pool<Postgres>> {
    let options: sqlx::postgres::PgConnectOptions = (*db.connect_options()).clone();
    Ok(sqlx::postgres::PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(0)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect_with(options)
        .await?)
}

async fn setup_schedule_infrastructure(db: &Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, \
         created_by, schema, language, kind, lock, created_at, archived, deleted, \
         is_template, extra_perms, draft_only, ws_error_handler_muted, no_main_func) \
         VALUES ('test-workspace', 12345, 'f/test/schedule_script', '', '', 'print(1)', \
         'test-user', '{}', 'python3', 'script', '', now(), false, false, \
         false, '{}', false, false, false)",
    )
    .execute(db)
    .await?;

    sqlx::query(
        "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, \
         timezone, enabled, script_path, is_flow, args, extra_perms, email, \
         no_flow_overlap, ws_error_handler_muted) \
         VALUES ('test-workspace', 'f/test/test_schedule', 'test-user', now(), \
         '0 */5 * * * *', 'UTC', true, 'f/test/schedule_script', false, \
         '{}', '{}', 'test@windmill.dev', false, false)",
    )
    .execute(db)
    .await?;

    Ok(())
}

async fn setup_second_schedule(db: &Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, \
         timezone, enabled, script_path, is_flow, args, extra_perms, email, \
         no_flow_overlap, ws_error_handler_muted) \
         VALUES ('test-workspace', 'f/test/test_schedule_2', 'test-user', now(), \
         '0 */10 * * * *', 'UTC', true, 'f/test/schedule_script', false, \
         '{}', '{}', 'test@windmill.dev', false, false)",
    )
    .execute(db)
    .await?;
    Ok(())
}

fn make_completed_job() -> windmill_queue::MiniCompletedJob {
    make_completed_job_for("f/test/test_schedule")
}

fn make_completed_job_for(schedule_path: &str) -> windmill_queue::MiniCompletedJob {
    use windmill_common::jobs::{JobKind, JobTriggerKind};
    use windmill_common::scripts::{ScriptHash, ScriptLang};

    windmill_queue::MiniCompletedJob {
        id: uuid::Uuid::new_v4(),
        workspace_id: "test-workspace".to_string(),
        runnable_id: Some(ScriptHash(12345)),
        scheduled_for: chrono::Utc::now(),
        parent_job: None,
        flow_innermost_root_job: None,
        runnable_path: Some("f/test/schedule_script".to_string()),
        kind: JobKind::Script,
        started_at: Some(chrono::Utc::now()),
        permissioned_as: "u/test-user".to_string(),
        created_by: "test-user".to_string(),
        script_lang: Some(ScriptLang::Python3),
        permissioned_as_email: "test@windmill.dev".to_string(),
        flow_step_id: None,
        trigger_kind: Some(JobTriggerKind::Schedule),
        trigger: Some(schedule_path.to_string()),
        priority: None,
        concurrent_limit: None,
        tag: "".to_string(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        runnable_settings_handle: None,
    }
}

async fn get_test_schedule(
    db: &Pool<Postgres>,
) -> anyhow::Result<windmill_common::schedule::Schedule> {
    Ok(windmill_queue::schedule::get_schedule_opt(db, "test-workspace", "f/test/test_schedule")
        .await?
        .expect("schedule should exist"))
}

/// Acquire N connections from the pool, returning them in a Vec to keep them held.
async fn acquire_n(
    pool: &Pool<Postgres>,
    n: u32,
) -> Vec<sqlx::pool::PoolConnection<Postgres>> {
    let mut conns = Vec::with_capacity(n as usize);
    for _ in 0..n {
        conns.push(pool.acquire().await.expect("should acquire connection"));
    }
    conns
}

/// Attempt a schedule push with a 30s timeout.
async fn try_schedule_push(
    pool: &Pool<Postgres>,
    schedule: &windmill_common::schedule::Schedule,
) -> Result<(), String> {
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        windmill_queue::handle_maybe_scheduled_job(
            pool,
            &make_completed_job(),
            schedule,
            "f/test/schedule_script",
            "test-workspace",
        ),
    )
    .await
    .map_err(|_| "timeout".to_string())?
    .map_err(|e| e.to_string())
}

async fn assert_schedule_enabled(db: &Pool<Postgres>) {
    assert_schedule_enabled_at(db, "f/test/test_schedule").await;
}

async fn assert_schedule_enabled_at(db: &Pool<Postgres>, path: &str) {
    let s = windmill_queue::schedule::get_schedule_opt(db, "test-workspace", path)
        .await
        .expect("query")
        .expect("schedule exists");
    assert!(s.enabled, "schedule {path} should remain enabled, error: {:?}", s.error);
}

async fn setup_on_behalf_of_email_infrastructure(db: &Pool<Postgres>) -> anyhow::Result<()> {
    // Add a delegate user
    sqlx::query(
        "INSERT INTO usr (workspace_id, email, username, is_admin, role) \
         VALUES ('test-workspace', 'delegate@windmill.dev', 'delegate-user', false, 'Admin')",
    )
    .execute(db)
    .await?;

    // Create a script with on_behalf_of_email set
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, \
         created_by, schema, language, kind, lock, created_at, archived, deleted, \
         is_template, extra_perms, draft_only, ws_error_handler_muted, no_main_func, \
         on_behalf_of_email) \
         VALUES ('test-workspace', 12346, 'f/test/obo_script', '', '', 'print(1)', \
         'delegate-user', '{}', 'python3', 'script', '', now(), false, false, \
         false, '{}', false, false, false, 'delegate@windmill.dev')",
    )
    .execute(db)
    .await?;

    // Schedule pointing to the on_behalf_of_email script
    sqlx::query(
        "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, \
         timezone, enabled, script_path, is_flow, args, extra_perms, email, \
         no_flow_overlap, ws_error_handler_muted) \
         VALUES ('test-workspace', 'f/test/obo_schedule', 'test-user', now(), \
         '0 */5 * * * *', 'UTC', true, 'f/test/obo_script', false, \
         '{}', '{}', 'test@windmill.dev', false, false)",
    )
    .execute(db)
    .await?;

    Ok(())
}

async fn get_obo_schedule(
    db: &Pool<Postgres>,
) -> anyhow::Result<windmill_common::schedule::Schedule> {
    Ok(
        windmill_queue::schedule::get_schedule_opt(db, "test-workspace", "f/test/obo_schedule")
            .await?
            .expect("obo schedule should exist"),
    )
}

fn make_completed_job_obo() -> windmill_queue::MiniCompletedJob {
    use windmill_common::scripts::ScriptHash;

    let mut job = make_completed_job_for("f/test/obo_schedule");
    job.runnable_id = Some(ScriptHash(12346));
    job.runnable_path = Some("f/test/obo_script".to_string());
    job
}

async fn try_schedule_push_obo(
    pool: &Pool<Postgres>,
    schedule: &windmill_common::schedule::Schedule,
) -> Result<(), String> {
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        windmill_queue::handle_maybe_scheduled_job(
            pool,
            &make_completed_job_obo(),
            schedule,
            "f/test/obo_script",
            "test-workspace",
        ),
    )
    .await
    .map_err(|_| "timeout".to_string())?
    .map_err(|e| e.to_string())
}

async fn count_queued_jobs(db: &Pool<Postgres>) -> i64 {
    let count: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM v2_job WHERE workspace_id = 'test-workspace'",
    )
    .fetch_one(db)
    .await
    .expect("count query should succeed");
    count.0
}
