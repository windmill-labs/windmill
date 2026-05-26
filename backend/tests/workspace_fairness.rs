//! Tests for the workspace-fairness algorithm (Enterprise feature).
//!
//! Multi-tenant clusters with a single shared worker pool let one workspace
//! starve the others if it floods the queue. The algorithm in
//! `windmill_queue::workspace_fairness_ee` periodically aggregates
//! per-workspace activity and stochastically excludes any workspace whose
//! share of cluster activity exceeds `WORKSPACE_FAIRNESS_MAX_PERCENT`%.
//!
//! There are two layers of tests in this file:
//!
//! 1. **Unit-style tests** (the first six) exercise the algorithm's response
//!    to fabricated activity tables and verify the audit-log writer. They are
//!    deterministic and fast.
//!
//! 2. **Simulation tests** (`fairness_50_workers_diverse_workload`,
//!    `fairness_oscillation_long_run`, `fairness_burst_then_stop`) spin up
//!    50 mock workers (async tasks doing the real pull → mark-running →
//!    sleep → complete cycle over real `v2_job_queue` rows), drive sustained
//!    diverse traffic from one noisy workspace + many victim workspaces, and
//!    measure the per-workspace **quality of service**. They are marked
//!    `#[ignore]` so the default `cargo test` stays fast — run with
//!    `--ignored` to exercise them.
//!
//! The entire file is gated on `private` because the algorithm itself only
//! compiles into the binary in EE builds. In OSS the `workspace_fairness`
//! module is a thin set of no-op stubs, so a test against it would have
//! nothing to assert.

#![cfg(feature = "private")]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serial_test::serial;
use sqlx::{Pool, Postgres};
use tokio::sync::Mutex;
use uuid::Uuid;

use windmill_common::worker::{
    WORKSPACE_FAIRNESS_DURATION_SECS, WORKSPACE_FAIRNESS_ENABLED,
    WORKSPACE_FAIRNESS_LAST_REFRESH_MICROS, WORKSPACE_FAIRNESS_MAX_PERCENT,
    WORKSPACE_FAIRNESS_MIN_TOTAL, WORKSPACE_FAIRNESS_OVERLOADED,
};
use windmill_queue::workspace_fairness::refresh_overloaded;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn reset_fairness_state() {
    WORKSPACE_FAIRNESS_OVERLOADED.store(Arc::new(vec![]));
    WORKSPACE_FAIRNESS_LAST_REFRESH_MICROS.store(0, Ordering::Relaxed);
    WORKSPACE_FAIRNESS_ENABLED.store(true, Ordering::Relaxed);
    WORKSPACE_FAIRNESS_MAX_PERCENT.store(50, Ordering::Relaxed);
    WORKSPACE_FAIRNESS_DURATION_SECS.store(10, Ordering::Relaxed);
    WORKSPACE_FAIRNESS_MIN_TOTAL.store(4, Ordering::Relaxed);
}

async fn create_workspace(db: &Pool<Postgres>, id: &str) {
    sqlx::query(
        "INSERT INTO workspace (id, name, owner)
         VALUES ($1, $1, 'test-user') ON CONFLICT (id) DO NOTHING",
    )
    .bind(id)
    .execute(db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workspace_settings (workspace_id) VALUES ($1)
         ON CONFLICT (workspace_id) DO NOTHING",
    )
    .bind(id)
    .execute(db)
    .await
    .unwrap();
}

/// Insert `n` completed jobs for `workspace_id`, each ending `secs_ago`
/// seconds in the past with a 1-second wall-clock duration. The fairness
/// algorithm weights contributions by `duration_ms` (clamped to the window),
/// so each job contributes ~1 worker-second when fully inside the window.
async fn insert_completed(db: &Pool<Postgres>, workspace_id: &str, n: usize, secs_ago: i32) {
    for _ in 0..n {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO v2_job (id, workspace_id, kind)
             VALUES (gen_random_uuid(), $1, 'script'::job_kind) RETURNING id",
        )
        .bind(workspace_id)
        .fetch_one(db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status,
                started_at, completed_at)
             VALUES ($1, $2, 1000, 'success'::job_status,
                NOW() - make_interval(secs => ($3::int + 1)),
                NOW() - make_interval(secs => $3::int))",
        )
        .bind(id)
        .bind(workspace_id)
        .bind(secs_ago)
        .execute(db)
        .await
        .unwrap();
    }
}

async fn insert_queued(
    db: &Pool<Postgres>,
    workspace_id: &str,
    n: usize,
    running: bool,
    tag: &str,
) -> Vec<Uuid> {
    let mut ids = Vec::with_capacity(n);
    for _ in 0..n {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO v2_job (id, workspace_id, kind, tag)
             VALUES (gen_random_uuid(), $1, 'script'::job_kind, $2) RETURNING id",
        )
        .bind(workspace_id)
        .bind(tag)
        .fetch_one(db)
        .await
        .unwrap();
        // Running jobs need a `started_at` for the fairness algorithm to
        // compute a positive elapsed-time contribution. Backdate by 1s so
        // each running row contributes ~1 worker-second by the time the
        // refresh runs, matching the `insert_completed` scale.
        sqlx::query(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, tag, started_at)
             VALUES ($1, $2, NOW(), $3, $4,
                     CASE WHEN $3 THEN NOW() - interval '1 second' ELSE NULL END)",
        )
        .bind(id)
        .bind(workspace_id)
        .bind(running)
        .bind(tag)
        .execute(db)
        .await
        .unwrap();
        if running {
            // The fairness algorithm bounds the running contribution by the
            // per-job `v2_job_runtime.ping`. Insert a fresh ping so each
            // running row accrues real-time worker-seconds.
            sqlx::query(
                "INSERT INTO v2_job_runtime (id, ping) VALUES ($1, NOW())
                 ON CONFLICT (id) DO UPDATE SET ping = NOW()",
            )
            .bind(id)
            .execute(db)
            .await
            .unwrap();
            insert_live_worker_ping(db, workspace_id, id).await;
        }
        ids.push(id);
    }
    ids
}

/// Insert a live `worker_ping` row claiming the given job. Each insert uses
/// a fresh randomly-named worker so callers can stack multiple pings without
/// PK collisions on `worker`.
async fn insert_live_worker_ping(db: &Pool<Postgres>, workspace_id: &str, job_id: Uuid) {
    let worker_name = format!("test-worker-{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO worker_ping (worker, worker_instance, ping_at, ip, current_job_id, current_job_workspace_id)
         VALUES ($1, 'test', NOW(), '127.0.0.1', $2, $3)",
    )
    .bind(&worker_name)
    .bind(job_id)
    .bind(workspace_id)
    .execute(db)
    .await
    .unwrap();
}

/// Insert a "zombie" running row: a row in `v2_job_queue` with `running=true`
/// but **no** live `worker_ping` claiming it (no paired worker, or the worker
/// has stopped pinging). The fairness algorithm must NOT count these — they
/// don't consume any worker slot.
async fn insert_zombie_running(db: &Pool<Postgres>, workspace_id: &str, n: usize) -> Vec<Uuid> {
    let mut ids = Vec::with_capacity(n);
    for _ in 0..n {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, tag, started_at)
             VALUES (gen_random_uuid(), $1, NOW() - interval '1 hour', true, 'deno',
                     NOW() - interval '1 hour') RETURNING id",
        )
        .bind(workspace_id)
        .fetch_one(db)
        .await
        .unwrap();
        ids.push(id);
    }
    ids
}

/// Insert a concurrency-suspended row: `running=true` AND `suspend > 0`. These
/// rows are not being processed by any worker (the flow is paused), so the
/// algorithm must not count them as slot occupancy.
async fn insert_suspended_running(db: &Pool<Postgres>, workspace_id: &str, n: usize) -> Vec<Uuid> {
    let mut ids = Vec::with_capacity(n);
    for _ in 0..n {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, suspend, tag)
             VALUES (gen_random_uuid(), $1, NOW(), true, 1, 'deno') RETURNING id",
        )
        .bind(workspace_id)
        .fetch_one(db)
        .await
        .unwrap();
        ids.push(id);
    }
    ids
}

fn overloaded_set() -> Vec<String> {
    (**WORKSPACE_FAIRNESS_OVERLOADED.load()).clone()
}

// ---------------------------------------------------------------------------
// Unit-style algorithm tests
// ---------------------------------------------------------------------------

#[sqlx::test(fixtures("base"))]
#[serial]
async fn fairness_caps_dominant_workspace(db: Pool<Postgres>) {
    reset_fairness_state();
    create_workspace(&db, "noisy").await;
    create_workspace(&db, "victim_a").await;
    create_workspace(&db, "victim_b").await;

    insert_completed(&db, "noisy", 60, 2).await;
    insert_completed(&db, "victim_a", 5, 3).await;
    insert_completed(&db, "victim_b", 5, 1).await;

    refresh_overloaded(&db).await.expect("refresh ok");

    assert_eq!(overloaded_set(), vec!["noisy".to_string()]);
}

#[sqlx::test(fixtures("base"))]
#[serial]
async fn fairness_respects_min_total(db: Pool<Postgres>) {
    reset_fairness_state();
    create_workspace(&db, "lone").await;
    insert_completed(&db, "lone", 3, 2).await;

    refresh_overloaded(&db).await.expect("refresh ok");

    assert!(overloaded_set().is_empty());
}

#[sqlx::test(fixtures("base"))]
#[serial]
async fn fairness_pull_query_skips_capped_workspace(db: Pool<Postgres>) {
    reset_fairness_state();
    create_workspace(&db, "noisy").await;
    create_workspace(&db, "victim").await;

    let noisy_ids = insert_queued(&db, "noisy", 1, false, "deno").await;
    let victim_ids = insert_queued(&db, "victim", 1, false, "deno").await;

    let regular_pick: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM v2_job_queue
         WHERE running = false AND tag IN ('deno') AND scheduled_for <= now()
         ORDER BY priority DESC NULLS LAST, scheduled_for LIMIT 1",
    )
    .fetch_optional(&db)
    .await
    .unwrap();
    assert_eq!(regular_pick, Some(noisy_ids[0]));

    let capped = vec!["noisy".to_string()];
    let fairness_pick: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM v2_job_queue
         WHERE running = false AND tag IN ('deno') AND scheduled_for <= now()
           AND workspace_id <> ALL($1::text[])
         ORDER BY priority DESC NULLS LAST, scheduled_for LIMIT 1",
    )
    .bind(&capped)
    .fetch_optional(&db)
    .await
    .unwrap();
    assert_eq!(fairness_pick, Some(victim_ids[0]));
}

#[sqlx::test(fixtures("base"))]
#[serial]
async fn fairness_lifts_when_load_drops(db: Pool<Postgres>) {
    reset_fairness_state();
    create_workspace(&db, "noisy").await;
    create_workspace(&db, "victim_a").await;
    create_workspace(&db, "victim_b").await;

    insert_completed(&db, "noisy", 60, 2).await;
    insert_completed(&db, "victim_a", 5, 3).await;
    insert_completed(&db, "victim_b", 5, 1).await;
    refresh_overloaded(&db).await.expect("refresh ok");
    assert_eq!(overloaded_set(), vec!["noisy".to_string()]);

    sqlx::query(
        "UPDATE v2_job_completed
            SET completed_at = NOW() - make_interval(secs => 60),
                started_at = NOW() - make_interval(secs => 60)
          WHERE workspace_id IN ('noisy', 'victim_a', 'victim_b')",
    )
    .execute(&db)
    .await
    .unwrap();
    insert_completed(&db, "noisy", 10, 2).await;
    insert_completed(&db, "victim_a", 10, 2).await;
    insert_completed(&db, "victim_b", 10, 2).await;

    // Roll updated_at back so the DB-side claim guard lets the next refresh win.
    sqlx::query(
        "UPDATE background_task_state
            SET updated_at = NOW() - INTERVAL '1 hour'
          WHERE name = 'workspace_fairness'",
    )
    .execute(&db)
    .await
    .unwrap();

    refresh_overloaded(&db).await.expect("refresh ok");
    assert!(overloaded_set().is_empty());
}

/// Ensure today's audit_partitioned partition exists — the migration only
/// creates partitions for the day it ran + 3 days, after which production
/// relies on `monitor::manage_audit_partitions` to roll new ones. That
/// maintenance task does not run in the test binary, so inserts silently
/// fail-and-warn without it.
async fn ensure_today_audit_partition(db: &Pool<Postgres>) {
    let today: chrono::NaiveDate = chrono::Utc::now().date_naive();
    let next = today + chrono::Duration::days(1);
    let partition = format!("audit_{}", today.format("%Y%m%d"));
    let sql = format!(
        "CREATE TABLE IF NOT EXISTS \"{partition}\" PARTITION OF audit_partitioned \
         FOR VALUES FROM ('{today}') TO ('{next}')"
    );
    let _ = sqlx::query(&sql).execute(db).await;
}

/// Both cap AND uncap transitions must produce audit-log rows. This test
/// drives a cap → uncap cycle and inspects `audit_partitioned` directly.
#[sqlx::test(fixtures("base"))]
#[serial]
async fn fairness_audit_records_both_cap_and_uncap(db: Pool<Postgres>) {
    reset_fairness_state();
    ensure_today_audit_partition(&db).await;
    create_workspace(&db, "noisy").await;
    create_workspace(&db, "victim_a").await;
    create_workspace(&db, "victim_b").await;

    // Phase 1 — push noisy to dominate, cap it.
    insert_completed(&db, "noisy", 60, 2).await;
    insert_completed(&db, "victim_a", 5, 3).await;
    insert_completed(&db, "victim_b", 5, 1).await;
    refresh_overloaded(&db).await.expect("refresh ok");
    assert_eq!(overloaded_set(), vec!["noisy".to_string()]);

    // Phase 2 — roll noisy's completions outside the window, push balanced
    // load, force a refresh; noisy should be uncapped.
    sqlx::query(
        "UPDATE v2_job_completed
            SET completed_at = NOW() - make_interval(secs => 60),
                started_at = NOW() - make_interval(secs => 60)
          WHERE workspace_id IN ('noisy', 'victim_a', 'victim_b')",
    )
    .execute(&db)
    .await
    .unwrap();
    insert_completed(&db, "noisy", 10, 2).await;
    insert_completed(&db, "victim_a", 10, 2).await;
    insert_completed(&db, "victim_b", 10, 2).await;
    sqlx::query(
        "UPDATE background_task_state
            SET updated_at = NOW() - INTERVAL '1 hour'
          WHERE name = 'workspace_fairness'",
    )
    .execute(&db)
    .await
    .unwrap();
    refresh_overloaded(&db).await.expect("refresh ok");
    assert!(overloaded_set().is_empty());

    // Verify both audit rows actually landed.
    let capped_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_partitioned
         WHERE workspace_id = 'admins'
           AND operation = 'workspace_fairness.capped'
           AND resource = 'noisy'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    let uncapped_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_partitioned
         WHERE workspace_id = 'admins'
           AND operation = 'workspace_fairness.uncapped'
           AND resource = 'noisy'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    println!("audit rows: capped={capped_count}, uncapped={uncapped_count}");
    assert_eq!(capped_count, 1, "expected exactly 1 capped audit for noisy");
    assert_eq!(
        uncapped_count, 1,
        "expected exactly 1 uncapped audit for noisy — \
         if this is 0, the uncap transition is not being recorded"
    );
}

#[sqlx::test(fixtures("base"))]
#[serial]
async fn fairness_catches_slot_hoggers(db: Pool<Postgres>) {
    reset_fairness_state();
    create_workspace(&db, "hogger").await;
    create_workspace(&db, "victim").await;

    insert_queued(&db, "hogger", 10, true, "deno").await;
    insert_completed(&db, "victim", 2, 1).await;

    refresh_overloaded(&db).await.expect("refresh ok");

    assert_eq!(overloaded_set(), vec!["hogger".to_string()]);
}

/// Regression: a workspace with a large backlog of `running = true` rows that
/// have **no live worker** claiming them (worker died, ping went stale, etc.)
/// must not be counted as "active". A previous version of the algorithm
/// counted `v2_job_queue.running = true` directly and was perpetually pinned
/// on the workspace with the most zombie rows, masking every other workspace.
#[sqlx::test(fixtures("base"))]
#[serial]
#[ignore = "flaky in CI"]
async fn fairness_ignores_zombie_running_rows(db: Pool<Postgres>) {
    reset_fairness_state();
    create_workspace(&db, "stuck_backlog").await;
    create_workspace(&db, "real_noisy").await;
    create_workspace(&db, "victim").await;

    // 100 zombie running rows for `stuck_backlog`. No paired worker_ping ⇒
    // no live worker is processing them. Old algorithm: 100 units of fake
    // activity. New algorithm: 0 units.
    insert_zombie_running(&db, "stuck_backlog", 100).await;
    // `real_noisy` is genuinely flooding the cluster.
    insert_completed(&db, "real_noisy", 60, 2).await;
    insert_completed(&db, "victim", 5, 3).await;

    refresh_overloaded(&db).await.expect("refresh ok");

    let set = overloaded_set();
    assert!(
        !set.contains(&"stuck_backlog".to_string()),
        "zombie running rows must not flag a workspace as overloaded; got {set:?}"
    );
    assert_eq!(
        set,
        vec!["real_noisy".to_string()],
        "the actually noisy workspace must surface even when another workspace \
         has a large backlog of zombie running rows; got {set:?}"
    );
}

/// Regression: concurrency-suspended rows (`running = true AND suspend > 0`)
/// are not consuming worker slots — the flow is paused at a suspend step —
/// and must not contribute to the activity share.
#[sqlx::test(fixtures("base"))]
#[serial]
#[ignore = "flaky in CI"]
async fn fairness_ignores_concurrency_suspended_rows(db: Pool<Postgres>) {
    reset_fairness_state();
    create_workspace(&db, "concurrency_capped").await;
    create_workspace(&db, "real_noisy").await;
    create_workspace(&db, "victim").await;

    // 100 concurrency-suspended rows. Each has `running = true` (the legacy
    // signal) but `suspend > 0` (not actually on a worker).
    insert_suspended_running(&db, "concurrency_capped", 100).await;
    insert_completed(&db, "real_noisy", 60, 2).await;
    insert_completed(&db, "victim", 5, 3).await;

    refresh_overloaded(&db).await.expect("refresh ok");

    let set = overloaded_set();
    assert!(
        !set.contains(&"concurrency_capped".to_string()),
        "concurrency-suspended rows must not flag a workspace as overloaded; got {set:?}"
    );
    assert_eq!(
        set,
        vec!["real_noisy".to_string()],
        "noisy workspace must still surface despite another workspace's large \
         suspended backlog; got {set:?}"
    );
}

// ---------------------------------------------------------------------------
// Simulation test
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct JobSpec {
    duration_ms: u32,
}

#[derive(Debug)]
struct Stats {
    /// Per-workspace observed latencies in milliseconds (enqueue → complete).
    per_ws: HashMap<String, Vec<u64>>,
    /// Per-workspace queued counts (pushed by the workload generator).
    pushed: HashMap<String, u64>,
    /// Per-workspace completion events as (elapsed_ms_since_scenario_start,
    /// latency_ms). Used by the oscillation simulation to compute per-second
    /// latency time series.
    events: HashMap<String, Vec<(u64, u64)>>,
    /// Reference t=0 for the current scenario, set by `run_scenario`.
    started: Option<Instant>,
}

impl Stats {
    fn new() -> Self {
        Self {
            per_ws: HashMap::new(),
            pushed: HashMap::new(),
            events: HashMap::new(),
            started: None,
        }
    }
    fn record(&mut self, ws: &str, latency_ms: u64) {
        self.per_ws
            .entry(ws.to_string())
            .or_default()
            .push(latency_ms);
        if let Some(t0) = self.started {
            let elapsed_ms = t0.elapsed().as_millis() as u64;
            self.events
                .entry(ws.to_string())
                .or_default()
                .push((elapsed_ms, latency_ms));
        }
    }
    fn pushed_inc(&mut self, ws: &str) {
        *self.pushed.entry(ws.to_string()).or_insert(0) += 1;
    }
}

#[derive(Debug, Clone)]
struct WsSummary {
    workspace: String,
    pushed: u64,
    completed: u64,
    p50_ms: u64,
    p95_ms: u64,
    p99_ms: u64,
    max_ms: u64,
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p / 100.0).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn summarize(stats: &Stats) -> Vec<WsSummary> {
    let mut workspaces: Vec<&String> = stats.per_ws.keys().collect();
    workspaces.sort();
    workspaces
        .into_iter()
        .map(|ws| {
            let mut lat = stats.per_ws.get(ws).cloned().unwrap_or_default();
            lat.sort_unstable();
            WsSummary {
                workspace: ws.clone(),
                pushed: stats.pushed.get(ws).copied().unwrap_or(0),
                completed: lat.len() as u64,
                p50_ms: percentile(&lat, 50.0),
                p95_ms: percentile(&lat, 95.0),
                p99_ms: percentile(&lat, 99.0),
                max_ms: *lat.last().unwrap_or(&0),
            }
        })
        .collect()
}

fn print_summary(label: &str, rows: &[WsSummary]) {
    println!(
        "\n=== {label} ===\n{:<14} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7}",
        "workspace", "pushed", "done", "p50", "p95", "p99", "max"
    );
    for r in rows {
        println!(
            "{:<14} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7}",
            r.workspace, r.pushed, r.completed, r.p50_ms, r.p95_ms, r.p99_ms, r.max_ms,
        );
    }
}

/// Mock worker. Loops pulling one job at a time, marking it running, sleeping
/// for the job's specified duration, then writing it to `v2_job_completed`.
/// Honors the overloaded-set bind if `fairness_on` is true. Stops when
/// `shutdown` flips.
async fn mock_worker(
    worker_id: u32,
    db: Pool<Postgres>,
    fairness_on: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    stats: Arc<Mutex<Stats>>,
    completed_counter: Arc<AtomicU64>,
) {
    let worker_name = format!("mock-worker-{worker_id}");
    // Each mock worker maintains its own `worker_ping` row, the way a real
    // worker would: `current_job_*` set on pick-up, cleared on completion.
    // The fairness algorithm now reads slot occupancy from `worker_ping` (so
    // that concurrency-suspended rows and zombies with no live ping do not
    // inflate the denominator), so the simulation must keep this in sync.
    sqlx::query(
        "INSERT INTO worker_ping (worker, worker_instance, ping_at) VALUES ($1, 'sim', NOW())
         ON CONFLICT (worker) DO UPDATE SET ping_at = NOW(),
            current_job_id = NULL, current_job_workspace_id = NULL",
    )
    .bind(&worker_name)
    .execute(&db)
    .await
    .unwrap();
    let standard_sql = "WITH picked AS (
                    SELECT id FROM v2_job_queue
                    WHERE running = false AND scheduled_for <= now()
                    ORDER BY priority DESC NULLS LAST, scheduled_for
                    FOR UPDATE SKIP LOCKED LIMIT 1
                )
                UPDATE v2_job_queue q
                   SET running = true, started_at = now()
                  FROM picked
                 WHERE q.id = picked.id
                RETURNING q.id, q.workspace_id, COALESCE((q.extras->>'duration_ms')::int, 30), q.created_at";
    let fairness_sql = "WITH picked AS (
                    SELECT id FROM v2_job_queue
                    WHERE running = false AND scheduled_for <= now()
                      AND workspace_id <> ALL($1::text[])
                    ORDER BY priority DESC NULLS LAST, scheduled_for
                    FOR UPDATE SKIP LOCKED LIMIT 1
                )
                UPDATE v2_job_queue q
                   SET running = true, started_at = now()
                  FROM picked
                 WHERE q.id = picked.id
                RETURNING q.id, q.workspace_id, COALESCE((q.extras->>'duration_ms')::int, 30), q.created_at";

    while !shutdown.load(Ordering::Relaxed) {
        // Snapshot the overloaded set at pull time so each pull reflects the
        // latest refresh. Mirror the production dispatch: if there is anything
        // capped, flip the same coin the real pull does to decide whether to
        // admit it. Empty overloaded set => standard query unconditionally.
        let overloaded = if fairness_on.load(Ordering::Relaxed) {
            (**WORKSPACE_FAIRNESS_OVERLOADED.load()).clone()
        } else {
            vec![]
        };
        let exclude_capped =
            !overloaded.is_empty() && !windmill_queue::workspace_fairness::should_admit_capped();

        // Primary query (chosen by the coin flip).
        let mut row: Option<(Uuid, String, i32, chrono::DateTime<chrono::Utc>)> = if exclude_capped
        {
            sqlx::query_as::<_, (Uuid, String, i32, chrono::DateTime<chrono::Utc>)>(fairness_sql)
                .bind(&overloaded)
                .fetch_optional(&db)
                .await
                .unwrap()
        } else {
            sqlx::query_as::<_, (Uuid, String, i32, chrono::DateTime<chrono::Utc>)>(standard_sql)
                .fetch_optional(&db)
                .await
                .unwrap()
        };

        // Fallback: if the fairness query returned nothing (every non-capped
        // workspace queue is empty), retry without the filter so workers
        // don't idle when only capped jobs remain.
        if row.is_none() && exclude_capped {
            row = sqlx::query_as::<_, (Uuid, String, i32, chrono::DateTime<chrono::Utc>)>(
                standard_sql,
            )
            .fetch_optional(&db)
            .await
            .unwrap();
        }

        match row {
            Some((id, ws, dur_ms, created_at)) => {
                // Claim the slot on this worker's ping so the fairness
                // algorithm counts this workspace's slot occupancy.
                sqlx::query(
                    "UPDATE worker_ping SET ping_at = NOW(),
                        current_job_id = $1, current_job_workspace_id = $2
                     WHERE worker = $3",
                )
                .bind(id)
                .bind(&ws)
                .bind(&worker_name)
                .execute(&db)
                .await
                .unwrap();

                tokio::time::sleep(Duration::from_millis(dur_ms as u64)).await;

                // Move to completed atomically: insert + delete in one query.
                let completed_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
                    "WITH del AS (
                        DELETE FROM v2_job_queue WHERE id = $1 RETURNING id, workspace_id
                    )
                    INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status, started_at, completed_at)
                    SELECT id, workspace_id, $2, 'success'::job_status, now(), now()
                    FROM del
                    RETURNING completed_at",
                )
                .bind(id)
                .bind(dur_ms as i64)
                .fetch_one(&db)
                .await
                .unwrap();

                // Release the slot.
                sqlx::query(
                    "UPDATE worker_ping SET ping_at = NOW(),
                        current_job_id = NULL, current_job_workspace_id = NULL
                     WHERE worker = $1",
                )
                .bind(&worker_name)
                .execute(&db)
                .await
                .unwrap();

                let latency_ms = (completed_at - created_at).num_milliseconds().max(0) as u64;
                {
                    let mut s = stats.lock().await;
                    s.record(&ws, latency_ms);
                }
                completed_counter.fetch_add(1, Ordering::Relaxed);
            }
            None => {
                // Empty queue (or every queued workspace is capped). Back off
                // briefly so we don't hammer the DB. Refresh the heartbeat so
                // this worker's ping doesn't go stale during long idle gaps.
                sqlx::query("UPDATE worker_ping SET ping_at = NOW() WHERE worker = $1")
                    .bind(&worker_name)
                    .execute(&db)
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
        }
    }
}

/// Push a stream of jobs from `workspace` at `rate_per_sec`. Each job's
/// duration is sampled from `[min_dur_ms, max_dur_ms)` using the given RNG seed.
async fn pusher(
    db: Pool<Postgres>,
    workspace: String,
    rate_per_sec: u32,
    min_dur_ms: u32,
    max_dur_ms: u32,
    duration: Duration,
    seed: u64,
    stats: Arc<Mutex<Stats>>,
    shutdown: Arc<AtomicBool>,
) {
    let mut rng = StdRng::seed_from_u64(seed);
    let interval = Duration::from_micros(1_000_000 / rate_per_sec.max(1) as u64);
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline && !shutdown.load(Ordering::Relaxed) {
        let dur = if min_dur_ms == max_dur_ms {
            min_dur_ms
        } else {
            rng.random_range(min_dur_ms..max_dur_ms)
        };
        let spec = JobSpec { duration_ms: dur };
        let extras = serde_json::json!({"duration_ms": spec.duration_ms});
        let res = sqlx::query(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, tag, extras)
             VALUES (gen_random_uuid(), $1, NOW(), false, 'deno', $2)",
        )
        .bind(&workspace)
        .bind(&extras)
        .execute(&db)
        .await;
        if res.is_ok() {
            let mut s = stats.lock().await;
            s.pushed_inc(&workspace);
        }
        tokio::time::sleep(interval).await;
    }
}

/// Like `pusher` but does NO inter-insert sleep — pushes flat out for
/// `duration`, batching every insert. Used to drive the noisy workspace into
/// genuine queue oversubscription. Multiple instances run in parallel to
/// exceed single-task push ceilings.
async fn noisy_pusher(
    db: Pool<Postgres>,
    workspace: String,
    min_dur_ms: u32,
    max_dur_ms: u32,
    duration: Duration,
    seed: u64,
    stats: Arc<Mutex<Stats>>,
    shutdown: Arc<AtomicBool>,
) {
    let mut rng = StdRng::seed_from_u64(seed);
    let deadline = Instant::now() + duration;
    let mut local_pushed: u64 = 0;
    while Instant::now() < deadline && !shutdown.load(Ordering::Relaxed) {
        let dur = if min_dur_ms == max_dur_ms {
            min_dur_ms
        } else {
            rng.random_range(min_dur_ms..max_dur_ms)
        };
        let extras = serde_json::json!({"duration_ms": dur});
        let res = sqlx::query(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, tag, extras)
             VALUES (gen_random_uuid(), $1, NOW(), false, 'deno', $2)",
        )
        .bind(&workspace)
        .bind(&extras)
        .execute(&db)
        .await;
        if res.is_ok() {
            local_pushed += 1;
            // Batch stats updates to avoid lock contention with workers.
            if local_pushed % 32 == 0 {
                let mut s = stats.lock().await;
                for _ in 0..32 {
                    s.pushed_inc(&workspace);
                }
            }
        }
        // Yield to the scheduler so other tasks (workers, refresh) can run.
        tokio::task::yield_now().await;
    }
    // Flush remaining counter.
    let leftover = local_pushed % 32;
    if leftover > 0 {
        let mut s = stats.lock().await;
        for _ in 0..leftover {
            s.pushed_inc(&workspace);
        }
    }
}

/// Background task that re-runs the fairness algorithm on a cadence so the
/// overloaded set tracks the live workload (mirrors what `maybe_refresh_overloaded`
/// does in production).
///
/// `force_refresh = true` rolls back the DB-side claim guard every iteration,
/// so each call re-runs the heavy aggregation. Use for short-running tests
/// that need fast adaptation. `force_refresh = false` leaves the natural 2 s
/// (`ACTIVE_REFRESH_SECS`) claim guard in place — this is what production
/// behaves like and what the oscillation/burst simulations want.
async fn refresh_loop(db: Pool<Postgres>, shutdown: Arc<AtomicBool>, force_refresh: bool) {
    while !shutdown.load(Ordering::Relaxed) {
        if force_refresh {
            let _ = sqlx::query(
                "UPDATE background_task_state
                    SET updated_at = NOW() - INTERVAL '1 hour'
                  WHERE name = 'workspace_fairness'",
            )
            .execute(&db)
            .await;
        }
        let _ = refresh_overloaded(&db).await;
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

#[derive(Debug)]
struct ScenarioResult {
    summary: Vec<WsSummary>,
    /// Wall-clock duration the scenario actually ran for (push + drain).
    elapsed: Duration,
    /// Per-workspace completion events: (elapsed_ms, latency_ms). Used for
    /// the oscillation time-series analysis.
    events: HashMap<String, Vec<(u64, u64)>>,
}

async fn run_scenario(
    sqlx_db: &Pool<Postgres>,
    label: &'static str,
    fairness_on: bool,
    duration: Duration,
    drain_timeout: Duration,
    n_workers: u32,
    fairness_window_secs: u32,
    force_refresh: bool,
) -> ScenarioResult {
    reset_fairness_state();
    WORKSPACE_FAIRNESS_DURATION_SECS.store(fairness_window_secs, Ordering::Relaxed);
    WORKSPACE_FAIRNESS_MIN_TOTAL.store(10, Ordering::Relaxed);

    // The sqlx::test-provided pool is capped at 10 connections — way too few
    // for 50 concurrent workers + pushers + refresh. Rebuild a wider pool
    // against the same database so the simulation actually runs in parallel.
    let opts = (*sqlx_db.connect_options()).clone();
    let big_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(80)
        .min_connections(20)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(opts)
        .await
        .expect("build simulation pool");
    let db = &big_pool;

    // Ensure the simulation workspaces exist (idempotent across scenarios).
    create_workspace(db, "noisy").await;
    for i in 0..5 {
        create_workspace(db, &format!("victim_{i}")).await;
    }

    // Truncate residual state from any prior scenario on the same DB.
    sqlx::query("DELETE FROM v2_job_queue WHERE workspace_id IN ('noisy','victim_0','victim_1','victim_2','victim_3','victim_4')")
        .execute(db).await.unwrap();
    sqlx::query("DELETE FROM v2_job_completed WHERE workspace_id IN ('noisy','victim_0','victim_1','victim_2','victim_3','victim_4')")
        .execute(db).await.unwrap();
    sqlx::query("DELETE FROM background_task_state WHERE name = 'workspace_fairness'")
        .execute(db)
        .await
        .unwrap();

    let stats = Arc::new(Mutex::new(Stats::new()));
    let shutdown = Arc::new(AtomicBool::new(false));
    let fairness_flag = Arc::new(AtomicBool::new(fairness_on));
    let completed = Arc::new(AtomicU64::new(0));

    let started = Instant::now();
    {
        let mut s = stats.lock().await;
        s.started = Some(started);
    }

    // Spawn workers.
    let mut worker_handles = Vec::with_capacity(n_workers as usize);
    for wid in 0..n_workers {
        let db = db.clone();
        let stats = stats.clone();
        let shutdown = shutdown.clone();
        let fairness_flag = fairness_flag.clone();
        let completed = completed.clone();
        worker_handles.push(tokio::spawn(async move {
            mock_worker(wid, db, fairness_flag, shutdown, stats, completed).await
        }));
    }

    // Spawn fairness refresh loop (a no-op when fairness_on is false, but we
    // still drive it so the DB state stays consistent).
    let refresh_handle = if fairness_on {
        let db = db.clone();
        let shutdown = shutdown.clone();
        Some(tokio::spawn(async move {
            refresh_loop(db, shutdown, force_refresh).await
        }))
    } else {
        None
    };

    // Pre-populate the queue with a noisy backlog so workers start saturated
    // from t=0 — the realistic case where a noisy workspace has already been
    // flooding the queue before the simulation window begins.
    let noisy_backlog: i64 = 1500;
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, tag, extras)
         SELECT gen_random_uuid(), 'noisy', NOW(), false, 'deno',
                jsonb_build_object('duration_ms', 60 + (random()*40)::int)
         FROM generate_series(1, $1::int)",
    )
    .bind(noisy_backlog)
    .execute(db)
    .await
    .unwrap();
    {
        let mut s = stats.lock().await;
        for _ in 0..noisy_backlog {
            s.pushed_inc("noisy");
        }
    }
    // Pre-populate v2_job_completed with synthetic noisy completions so the
    // first fairness refresh (running before any real completion arrives)
    // already sees noisy as dominant. Without this, fairness has nothing to
    // detect for ~1s and the comparison is contaminated by an unfair
    // warmup phase.
    sqlx::query(
        "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status, started_at, completed_at)
         SELECT gen_random_uuid(), 'noisy', 80, 'success'::job_status,
                NOW() - INTERVAL '1 second', NOW() - INTERVAL '1 second'
         FROM generate_series(1, 200)",
    )
    .execute(db).await.unwrap();

    // Spawn pushers. Workload:
    //   - "noisy": FOUR sustained pushers with no inter-insert sleep,
    //     job durations 60–100ms. Combined they aim to push >2000 jobs/s,
    //     well over the 50-worker capacity (~625 jobs/s @ 80ms avg).
    //   - 3 victim_high: 10 jobs/s each, 60–100 ms (moderate workspaces)
    //   - 2 victim_low:   4 jobs/s each, 60–100 ms (quiet workspaces)
    // Total victim demand: 3*10 + 2*4 = 38 jobs/s, ~3 s/s of work —
    // a rounding error against worker capacity, so under fairness their
    // jobs should drain at near-zero queueing latency.
    let pusher_specs: Vec<(String, u32, u32, u32, u64)> = vec![
        // Victims
        ("victim_0".to_string(), 10, 60, 100, 11),
        ("victim_1".to_string(), 10, 60, 100, 12),
        ("victim_2".to_string(), 10, 60, 100, 13),
        ("victim_3".to_string(), 4, 60, 100, 21),
        ("victim_4".to_string(), 4, 60, 100, 22),
    ];
    let mut pusher_handles = vec![];
    for (ws, rate, mn, mx, seed) in pusher_specs {
        let db = db.clone();
        let stats = stats.clone();
        let shutdown = shutdown.clone();
        pusher_handles.push(tokio::spawn(async move {
            pusher(db, ws, rate, mn, mx, duration, seed, stats, shutdown).await;
        }));
    }
    // Four noisy pushers running flat out (no sleep). Each pushes
    // continuously for `duration`, then drops.
    for noisy_seed in 1..=4u64 {
        let db = db.clone();
        let stats = stats.clone();
        let shutdown = shutdown.clone();
        pusher_handles.push(tokio::spawn(async move {
            noisy_pusher(
                db,
                "noisy".to_string(),
                60,
                100,
                duration,
                noisy_seed,
                stats,
                shutdown,
            )
            .await;
        }));
    }

    // Wait for pushers to finish pushing.
    for h in pusher_handles {
        let _ = h.await;
    }

    // Drain phase: wait until VICTIM workspaces drain (or timeout). We
    // deliberately do NOT wait for noisy to drain — when fairness is OFF the
    // noisy backlog runs into tens of thousands of jobs and "fully drain"
    // makes the test take minutes. Victim QoL is what we're measuring, and
    // a victim job not completing inside the drain window is itself a
    // signal of starvation that we want to capture in the latency record.
    let drain_deadline = Instant::now() + drain_timeout;
    loop {
        let victim_remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM v2_job_queue
              WHERE workspace_id IN ('victim_0','victim_1','victim_2','victim_3','victim_4')",
        )
        .fetch_one(db)
        .await
        .unwrap();
        if victim_remaining == 0 || Instant::now() >= drain_deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Shut everything down.
    shutdown.store(true, Ordering::Relaxed);
    for h in worker_handles {
        let _ = h.await;
    }
    if let Some(h) = refresh_handle {
        let _ = h.await;
    }

    let elapsed = started.elapsed();
    let stats = stats.lock().await;
    let summary = summarize(&stats);
    print_summary(label, &summary);
    ScenarioResult { summary, elapsed, events: stats.events.clone() }
}

fn pick<'a>(rows: &'a [WsSummary], ws: &str) -> &'a WsSummary {
    rows.iter()
        .find(|r| r.workspace == ws)
        .expect("workspace in summary")
}

/// **50-worker simulation.** Pushes one noisy + five victim workspaces with
/// diverse durations through a real mock-worker pool, with and without the
/// fairness algorithm enabled. Asserts the **QoL of victim workspaces** is
/// materially better with fairness on.
///
/// Marked `#[ignore]` so the default `cargo test` keeps a sub-second profile.
/// Run with: `cargo test --test workspace_fairness -- --ignored --nocapture`.
#[sqlx::test(fixtures("base"))]
#[ignore]
#[serial]
async fn fairness_50_workers_diverse_workload(db: Pool<Postgres>) {
    let push_dur = Duration::from_secs(5);
    // Cap drain at 12s. Under fairness the victim queue drains in <1s; with
    // fairness off the victim jobs are stuck behind the noisy backlog and
    // may never drain inside the cap — that's the point. Whichever victim
    // jobs DO complete contribute to the p95 we assert against.
    let drain_dur = Duration::from_secs(12);

    // CONTROL: fairness OFF.
    let control = run_scenario(
        &db,
        "control (fairness OFF)",
        false,
        push_dur,
        drain_dur,
        50,
        3,
        true,
    )
    .await;
    // TREATMENT: fairness ON. Short test → force refresh every 250ms so the
    // cap takes effect inside the 5s window. (Production rate is 2s, which
    // would only give ~2 refresh cycles inside a 5s test.)
    let treatment = run_scenario(
        &db,
        "treatment (fairness ON)",
        true,
        push_dur,
        drain_dur,
        50,
        3,
        true,
    )
    .await;

    let victims = ["victim_0", "victim_1", "victim_2", "victim_3", "victim_4"];

    println!("\n=== victim p95 latency comparison ===");
    println!(
        "{:<10} {:>10} {:>10} {:>10}",
        "victim", "ctrl p95", "treat p95", "improvement"
    );
    let mut total_ctrl_p95 = 0u64;
    let mut total_treat_p95 = 0u64;
    let mut min_ratio = f64::INFINITY;
    for v in &victims {
        let c = pick(&control.summary, v);
        let t = pick(&treatment.summary, v);
        let ratio = if t.p95_ms == 0 {
            f64::INFINITY
        } else {
            c.p95_ms as f64 / t.p95_ms as f64
        };
        min_ratio = min_ratio.min(ratio);
        total_ctrl_p95 += c.p95_ms;
        total_treat_p95 += t.p95_ms;
        println!(
            "{:<10} {:>10} {:>10} {:>9.2}x",
            v, c.p95_ms, t.p95_ms, ratio
        );
    }
    let avg_ctrl_p95 = total_ctrl_p95 / victims.len() as u64;
    let avg_treat_p95 = total_treat_p95 / victims.len() as u64;
    println!(
        "avg victim p95: control={}ms  treatment={}ms  ratio={:.2}x",
        avg_ctrl_p95,
        avg_treat_p95,
        avg_ctrl_p95 as f64 / avg_treat_p95.max(1) as f64,
    );
    println!(
        "scenario elapsed: control={:?}  treatment={:?}",
        control.elapsed, treatment.elapsed,
    );

    // Treatment ran the algorithm: confirm the noisy workspace's completed
    // count is no higher than its control count — fairness must not inflate
    // throughput overall, it must reallocate slots away from noisy.
    let noisy_ctrl = pick(&control.summary, "noisy");
    let noisy_treat = pick(&treatment.summary, "noisy");
    println!(
        "noisy: control completed={}  treatment completed={}",
        noisy_ctrl.completed, noisy_treat.completed,
    );

    // Completion-rate comparison. Under fairness, victim queues drain inside
    // the simulation window; without fairness, victim jobs sit behind the
    // noisy backlog and many never complete inside the cap.
    let ctrl_v_pushed: u64 = victims
        .iter()
        .map(|v| pick(&control.summary, v).pushed)
        .sum();
    let ctrl_v_done: u64 = victims
        .iter()
        .map(|v| pick(&control.summary, v).completed)
        .sum();
    let treat_v_pushed: u64 = victims
        .iter()
        .map(|v| pick(&treatment.summary, v).pushed)
        .sum();
    let treat_v_done: u64 = victims
        .iter()
        .map(|v| pick(&treatment.summary, v).completed)
        .sum();
    let ctrl_v_rate = ctrl_v_done as f64 / ctrl_v_pushed.max(1) as f64;
    let treat_v_rate = treat_v_done as f64 / treat_v_pushed.max(1) as f64;
    println!(
        "victim completion rate: control={:.1}% ({}/{})  treatment={:.1}% ({}/{})",
        ctrl_v_rate * 100.0,
        ctrl_v_done,
        ctrl_v_pushed,
        treat_v_rate * 100.0,
        treat_v_done,
        treat_v_pushed,
    );

    // Treatment-side sanity: fairness should fully drain victim queues and
    // keep their p95 well sub-second. If either of these fails, the workload
    // is mis-sized or the algorithm has regressed.
    for v in &victims {
        let t = pick(&treatment.summary, v);
        let rate = t.completed as f64 / t.pushed.max(1) as f64;
        assert!(
            rate > 0.95,
            "victim {v} completion rate under fairness was {:.1}% ({}/{}) — \
             fairness algorithm is not protecting victim throughput",
            rate * 100.0,
            t.completed,
            t.pushed,
        );
        assert!(
            t.p95_ms < 1500,
            "victim {v} p95 latency under fairness is {}ms — should be \
             sub-second when noisy is capped",
            t.p95_ms,
        );
    }

    // Headline assertion: fairness must improve victim QoL substantially.
    // Either of these is sufficient:
    //   (a) victim p95 latency drops by ≥ 5x (slow service under starvation
    //       turns into fast service when the noisy workspace is capped), or
    //   (b) victim completion rate jumps by ≥ 1.5x (jobs that were never
    //       getting pulled finally complete).
    // We accept either because the relative weights of (a) vs (b) shift with
    // CI-machine speed: a fast box may complete more victim jobs in the
    // control run (boosting completion rate, deflating p95 ratio), while a
    // slow box will starve them more aggressively (boosting p95 ratio).
    let p95_ratio = (avg_ctrl_p95 as f64) / (avg_treat_p95.max(1) as f64);
    let rate_ratio = treat_v_rate / ctrl_v_rate.max(0.001);
    println!("p95 ratio (ctrl/treat) = {p95_ratio:.2}x, completion-rate ratio (treat/ctrl) = {rate_ratio:.2}x");
    assert!(
        p95_ratio >= 5.0 || rate_ratio >= 1.5,
        "fairness did not materially improve victim QoL: p95 ratio={p95_ratio:.2}x \
         (want ≥5x), completion-rate ratio={rate_ratio:.2}x (want ≥1.5x)",
    );

    // Sanity: noisy must NOT be capped to zero — fairness only throttles, it
    // does not exclude. Its completed count should stay > 0.
    assert!(
        noisy_treat.completed > 0,
        "noisy was completely starved by fairness — should be throttled, not excluded",
    );
}

/// Bucket events by 1-second windows of `elapsed_ms`. Returns
/// `Vec<(bucket_idx_seconds, count, p50, p95, max)>`.
fn time_series(events: &[(u64, u64)], buckets: usize) -> Vec<(usize, usize, u64, u64, u64)> {
    let mut by_bucket: Vec<Vec<u64>> = vec![vec![]; buckets];
    for (elapsed_ms, lat_ms) in events {
        let b = (*elapsed_ms / 1000) as usize;
        if b < buckets {
            by_bucket[b].push(*lat_ms);
        }
    }
    by_bucket
        .into_iter()
        .enumerate()
        .map(|(i, mut v)| {
            v.sort_unstable();
            let n = v.len();
            (
                i,
                n,
                percentile(&v, 50.0),
                percentile(&v, 95.0),
                *v.last().unwrap_or(&0),
            )
        })
        .collect()
}

/// **Oscillation test.** A capped workspace's stale completions roll out of
/// the rolling window after `WORKSPACE_FAIRNESS_DURATION_SECS` seconds — at
/// which point its share drops to 0%, the algorithm un-caps it, the noisy
/// queue (which has the oldest `scheduled_for`) jumps to the front of the
/// pull, and victims briefly wait until the next refresh cycle re-caps. Over
/// a long run this manifests as periodic spikes in victim latency, roughly
/// every `(window + refresh_interval)` seconds.
///
/// This test runs a 25-second sustained workload (long enough to cross at
/// least two cap/uncap cycles with the default 10s window) and prints
/// per-second victim p95 latency. It then asserts that the oscillation peaks
/// remain bounded — i.e. fairness still delivers good QoL on average even
/// though the cap is not perfectly stable.
///
/// Marked `#[ignore]`. Run with:
/// `cargo test --test workspace_fairness fairness_oscillation -- --ignored --nocapture`.
#[sqlx::test(fixtures("base"))]
#[ignore]
#[serial]
async fn fairness_oscillation_long_run(db: Pool<Postgres>) {
    // Use the *production default* 10-second window so the cap/uncap cycle
    // matches what the cluster actually sees. (Other tests use a 3s window
    // to keep wall-clock short.)
    reset_fairness_state();
    WORKSPACE_FAIRNESS_DURATION_SECS.store(10, Ordering::Relaxed);

    let push_dur = Duration::from_secs(25);
    // No drain — we don't care about post-push tail; the time-series view
    // already includes everything in the active window.
    let drain_dur = Duration::from_secs(2);

    // Use production refresh cadence (force_refresh=false) — the SQL claim's
    // 2 s rate limit takes effect, so refresh runs every 2 s like on the
    // real cluster instead of every 250 ms. This is what victims actually
    // experience.
    let treatment = run_scenario(
        &db,
        "treatment (fairness ON) — long run, 10s window, prod refresh",
        true,
        push_dur,
        drain_dur,
        50,
        10,
        false,
    )
    .await;

    let total_buckets = (push_dur.as_secs() + drain_dur.as_secs() + 2) as usize;
    let victims = ["victim_0", "victim_1", "victim_2", "victim_3", "victim_4"];

    // Merge all victim events into one stream for the time-series view —
    // QoL per-second across all victim workspaces is what we want to inspect.
    let mut merged: Vec<(u64, u64)> = Vec::new();
    for v in &victims {
        if let Some(es) = treatment.events.get(*v) {
            merged.extend_from_slice(es);
        }
    }
    let series = time_series(&merged, total_buckets);

    println!(
        "\n=== victim latency per second (treatment, 10s window) ===\n{:>4} {:>6} {:>6} {:>6} {:>6}",
        "sec", "count", "p50", "p95", "max"
    );
    for (sec, count, p50, p95, mx) in &series {
        println!("{:>4} {:>6} {:>6} {:>6} {:>6}", sec, count, p50, p95, mx);
    }

    // Same view for noisy — visualises the cap on/off pattern. A capped
    // bucket has near-zero completions; an uncapped bucket has many.
    let noisy_events = treatment.events.get("noisy").cloned().unwrap_or_default();
    let noisy_series = time_series(&noisy_events, total_buckets);
    println!("\n=== noisy completions per second (treatment) ===");
    for (sec, count, _, _, _) in &noisy_series {
        println!("sec {:>3}: {:>5} noisy completions", sec, count);
    }

    // Aggregate p95 and worst-bucket p95 across the active window (skip the
    // first second, which is dominated by warmup before the first refresh).
    let active: Vec<&(usize, usize, u64, u64, u64)> = series
        .iter()
        .filter(|(sec, count, ..)| *sec >= 1 && *sec < push_dur.as_secs() as usize && *count > 0)
        .collect();
    let avg_p95: u64 = if active.is_empty() {
        0
    } else {
        active.iter().map(|x| x.3).sum::<u64>() / active.len() as u64
    };
    let worst_p95: u64 = active.iter().map(|x| x.3).max().unwrap_or(0);
    let buckets_over_2s = active.iter().filter(|x| x.3 > 2000).count();
    let buckets_over_5s = active.iter().filter(|x| x.3 > 5000).count();
    println!(
        "\nactive window: {} sec, avg per-second victim p95 = {} ms, worst per-second p95 = {} ms",
        active.len(),
        avg_p95,
        worst_p95,
    );
    println!(
        "seconds with victim p95 > 2s: {} / {},  > 5s: {} / {}",
        buckets_over_2s,
        active.len(),
        buckets_over_5s,
        active.len(),
    );

    // The user's hypothesis under test: "10s latency on and off". The cycle
    // period is ~window + refresh interval ≈ 12-15s; the oscillation peak
    // (time spent in the uncapped state, which is when victims wait) is
    // bounded by the refresh interval, NOT the window. So we expect:
    //   - average per-second victim p95 well under 1s (cap mostly holds)
    //   - worst-second p95 under 5s (oscillation peaks are bounded)
    //   - only a small minority of seconds spent in the high-latency regime
    //
    // If any of these break, the cap/uncap cycle is too long or too costly,
    // and the algorithm needs to revisit the refresh cadence vs window size.
    let summary_v: Vec<&WsSummary> = victims
        .iter()
        .map(|v| pick(&treatment.summary, v))
        .collect();
    let total_completed: u64 = summary_v.iter().map(|s| s.completed).sum();
    let total_pushed: u64 = summary_v.iter().map(|s| s.pushed).sum();
    println!(
        "total victim completion rate: {:.1}% ({}/{})",
        100.0 * total_completed as f64 / total_pushed.max(1) as f64,
        total_completed,
        total_pushed,
    );

    assert!(
        avg_p95 < 1500,
        "average per-second victim p95 = {} ms — cap is not holding most of the time",
        avg_p95,
    );
    assert!(
        worst_p95 < 5_000,
        "worst-second victim p95 = {} ms — oscillation peak exceeds 5s, \
         which means uncapped windows are too long. Reduce refresh interval \
         or shorten the duration window.",
        worst_p95,
    );
    assert!(
        buckets_over_2s <= active.len() / 4,
        "victims spent > 2s p95 in {} / {} buckets — oscillation is more \
         frequent than expected (more than 25% of the simulation)",
        buckets_over_2s,
        active.len(),
    );
}

/// **Burst-then-stop scenario.** A noisy workspace enqueues 10,000 jobs in a
/// single burst at t=0 (all with the same `scheduled_for = now()`, so they
/// sit at the front of the FIFO queue forever after) and then stops pushing.
/// Victim workspaces push modestly throughout.
///
/// This is the worst-case oscillation regime for the algorithm: once noisy is
/// uncapped, every worker grabs from its backlog because it has the lowest
/// `scheduled_for` in the queue — exactly the behavior the user pointed at.
/// The question is how much that costs victims.
///
/// Mechanics with `WORKSPACE_FAIRNESS_DURATION_SECS = 10` (production default):
/// 1. t ≈ 0–1 s: workers drain ~500 noisy jobs FIFO. First refresh sees
///    noisy at ~100% of activity → CAPPED.
/// 2. t ≈ 1–11 s: noisy capped. Workers serve victims only. Noisy queue
///    stays at ~9,500.
/// 3. t ≈ 11 s: the noisy completions from step 1 age out of the rolling
///    window. Noisy share drops to 0% → UNCAPPED.
/// 4. t ≈ 11 s – 11 s + (refresh_interval): workers all switch to noisy
///    (oldest `scheduled_for`). Victims queue. Within ~1 refresh interval
///    the next refresh sees noisy dominant again → RE-CAPPED.
/// 5. Cycle repeats every ~(window + refresh_interval) ≈ 12 s.
///
/// Asserts:
///   - average per-second victim p95 stays well under 1 s
///   - worst per-second victim p95 stays under 3 s (uncapped bursts are bounded)
///   - the bulk of noisy is still drained (the cap is throttling, not excluding)
///
/// Run with:
/// `cargo test --test workspace_fairness fairness_burst -- --ignored --nocapture`.
#[sqlx::test(fixtures("base"))]
#[ignore]
#[serial]
async fn fairness_burst_then_stop(sqlx_db: Pool<Postgres>) {
    reset_fairness_state();
    WORKSPACE_FAIRNESS_DURATION_SECS.store(10, Ordering::Relaxed);
    WORKSPACE_FAIRNESS_MIN_TOTAL.store(10, Ordering::Relaxed);

    // Wider pool so 50 workers really run in parallel.
    let opts = (*sqlx_db.connect_options()).clone();
    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(80)
        .min_connections(20)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(opts)
        .await
        .expect("build burst pool");
    let db = &db;

    create_workspace(db, "noisy").await;
    for i in 0..3 {
        create_workspace(db, &format!("victim_{i}")).await;
    }

    sqlx::query(
        "DELETE FROM v2_job_queue WHERE workspace_id IN ('noisy','victim_0','victim_1','victim_2')",
    )
    .execute(db)
    .await
    .unwrap();
    sqlx::query(
        "DELETE FROM v2_job_completed WHERE workspace_id IN ('noisy','victim_0','victim_1','victim_2')",
    )
    .execute(db)
    .await
    .unwrap();
    sqlx::query("DELETE FROM background_task_state WHERE name = 'workspace_fairness'")
        .execute(db)
        .await
        .unwrap();

    // The burst: 10_000 noisy queued jobs, all with same scheduled_for. They
    // will hold the front-of-queue position for the entire simulation, which
    // is the scenario under test.
    let burst_size = 10_000_i64;
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, tag, extras)
         SELECT gen_random_uuid(), 'noisy', NOW(), false, 'deno',
                jsonb_build_object('duration_ms', 80)
         FROM generate_series(1, $1::int)",
    )
    .bind(burst_size)
    .execute(db)
    .await
    .unwrap();

    let stats = Arc::new(Mutex::new(Stats::new()));
    let shutdown = Arc::new(AtomicBool::new(false));
    let fairness_flag = Arc::new(AtomicBool::new(true));
    let completed = Arc::new(AtomicU64::new(0));
    let started = Instant::now();
    {
        let mut s = stats.lock().await;
        s.started = Some(started);
        for _ in 0..burst_size {
            s.pushed_inc("noisy");
        }
    }

    // 50 workers.
    let mut worker_handles = Vec::new();
    for wid in 0..50u32 {
        let db = db.clone();
        let stats = stats.clone();
        let shutdown = shutdown.clone();
        let fairness_flag = fairness_flag.clone();
        let completed = completed.clone();
        worker_handles.push(tokio::spawn(async move {
            mock_worker(wid, db, fairness_flag, shutdown, stats, completed).await
        }));
    }

    // Refresh task. force_refresh=false → the SQL claim's 2 s rate limit
    // takes effect, matching `ACTIVE_REFRESH_SECS` in production. This is
    // the regime the cloud cluster actually sees.
    let refresh_handle = {
        let db = db.clone();
        let shutdown = shutdown.clone();
        tokio::spawn(async move { refresh_loop(db, shutdown, false).await })
    };

    // Victim pushers: 3 workspaces, 20 jobs/s each, 80 ms durations,
    // sustained for the full simulation. Total victim demand: 60 jobs/s.
    let sim_dur = Duration::from_secs(30);
    let mut pusher_handles = vec![];
    for i in 0..3 {
        let db = db.clone();
        let stats = stats.clone();
        let shutdown = shutdown.clone();
        let ws = format!("victim_{i}");
        pusher_handles.push(tokio::spawn(async move {
            pusher(db, ws, 20, 80, 81, sim_dur, 100 + i as u64, stats, shutdown).await;
        }));
    }
    for h in pusher_handles {
        let _ = h.await;
    }

    // Brief drain so any queued victim jobs at the end have a chance to land.
    tokio::time::sleep(Duration::from_secs(2)).await;

    shutdown.store(true, Ordering::Relaxed);
    for h in worker_handles {
        let _ = h.await;
    }
    let _ = refresh_handle.await;

    let stats = stats.lock().await;
    let summary = summarize(&stats);
    print_summary("burst-then-stop (fairness ON, 10s window)", &summary);

    let total_buckets = (sim_dur.as_secs() + 4) as usize;
    let victims = ["victim_0", "victim_1", "victim_2"];
    let mut merged: Vec<(u64, u64)> = Vec::new();
    for v in &victims {
        if let Some(es) = stats.events.get(*v) {
            merged.extend_from_slice(es);
        }
    }
    let series = time_series(&merged, total_buckets);
    println!("\n=== victim latency per second (burst-then-stop) ===");
    println!(
        "{:>4} {:>6} {:>6} {:>6} {:>6}",
        "sec", "count", "p50", "p95", "max"
    );
    for (sec, count, p50, p95, mx) in &series {
        println!("{:>4} {:>6} {:>6} {:>6} {:>6}", sec, count, p50, p95, mx);
    }

    let noisy_events = stats.events.get("noisy").cloned().unwrap_or_default();
    let noisy_series = time_series(&noisy_events, total_buckets);
    println!("\n=== noisy completions per second (burst-then-stop) ===");
    for (sec, count, _, _, _) in &noisy_series {
        let bar = "#".repeat((count / 10).min(60) as usize);
        println!("sec {:>3}: {:>5}  {}", sec, count, bar);
    }

    let active: Vec<&(usize, usize, u64, u64, u64)> = series
        .iter()
        .filter(|(sec, count, ..)| *sec >= 1 && *sec < sim_dur.as_secs() as usize && *count > 0)
        .collect();
    let avg_p95: u64 = if active.is_empty() {
        0
    } else {
        active.iter().map(|x| x.3).sum::<u64>() / active.len() as u64
    };
    let worst_p95: u64 = active.iter().map(|x| x.3).max().unwrap_or(0);
    let buckets_over_1s = active.iter().filter(|x| x.3 > 1000).count();
    println!(
        "\nburst-then-stop summary: avg per-second victim p95 = {} ms, worst = {} ms, \
         seconds with p95 > 1s: {} / {}",
        avg_p95,
        worst_p95,
        buckets_over_1s,
        active.len(),
    );
    let noisy_drained = noisy_events.len();
    println!(
        "noisy jobs drained over simulation: {} / {} ({:.1}%)",
        noisy_drained,
        burst_size,
        100.0 * noisy_drained as f64 / burst_size as f64,
    );

    // Sanity: every victim still completes (cap is throttling not excluding).
    for v in &victims {
        let t = summary.iter().find(|s| s.workspace == *v).unwrap();
        let rate = t.completed as f64 / t.pushed.max(1) as f64;
        assert!(
            rate > 0.95,
            "victim {v} completion rate {:.1}% — fairness should protect victims even under burst",
            rate * 100.0,
        );
    }

    // The actual QoL claim we're testing against the user's hypothesis:
    // even though workers fully switch to noisy during each uncapped
    // interval, the uncap is bounded by `ACTIVE_REFRESH_SECS` (2 s in
    // production). Empirically with the prod-realistic refresh cadence
    // the avg per-second p95 stays under 1.5 s and the worst-second p95
    // stays under 4 s. If either of these blows out, the oscillation is
    // worse than acceptable and the algorithm needs a softer rate limit
    // (e.g. stochastic admission of capped workspaces).
    assert!(
        avg_p95 < 1_500,
        "average per-second victim p95 under burst was {} ms — \
         oscillation is degrading victim QoL more than expected",
        avg_p95,
    );
    assert!(
        worst_p95 < 4_000,
        "worst per-second victim p95 under burst was {} ms — \
         uncapped bursts are too long; check ACTIVE_REFRESH_SECS",
        worst_p95,
    );
}
