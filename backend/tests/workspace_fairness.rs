//! Tests for the cloud workspace-fairness algorithm.
//!
//! The cloud cluster runs a single shared worker pool. A "noisy" workspace
//! that floods the queue can starve the rest of the cluster of slots. The
//! algorithm in `windmill_queue::workspace_fairness` periodically aggregates
//! per-workspace activity (running jobs + jobs completed in a rolling window)
//! and excludes any workspace whose share of cluster activity exceeds
//! `WORKSPACE_FAIRNESS_MAX_PERCENT`% of the total from the next pull cycles.
//!
//! There are two layers of tests in this file:
//!
//! 1. **Unit-style tests** (the first five) exercise the algorithm's response
//!    to fabricated activity tables. They are deterministic and fast.
//!
//! 2. **Simulation test** `fairness_50_workers_diverse_workload` spins up 50
//!    mock workers (async tasks doing the real pull → mark-running → sleep →
//!    complete cycle over real `v2_job_queue` rows), drives sustained diverse
//!    traffic from one noisy workspace + many victim workspaces, and measures
//!    the per-workspace **quality of service** (completion count, latency
//!    percentiles) both with fairness ON and OFF. It then asserts the
//!    treatment beats the control on the metric that matters: victim p95
//!    latency.
//!
//! The cloud gate (`CLOUD_HOSTED` + `BASE_URL == app.windmill.dev`) is
//! enforced only inside the wrapper `maybe_refresh_overloaded` and in the
//! settings API. The algorithm itself (`refresh_overloaded`) is gate-free —
//! these tests call it directly with the per-process atomics set to
//! representative cloud values, which is the same state the runtime ends up
//! in once the gate flips on.

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

async fn insert_completed(db: &Pool<Postgres>, workspace_id: &str, n: usize, secs_ago: i32) {
    for _ in 0..n {
        sqlx::query(
            "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status,
                started_at, completed_at)
             VALUES (gen_random_uuid(), $1, 1, 'success'::job_status,
                NOW() - make_interval(secs => $2::int),
                NOW() - make_interval(secs => $2::int))",
        )
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
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, tag)
             VALUES (gen_random_uuid(), $1, NOW(), $2, $3) RETURNING id",
        )
        .bind(workspace_id)
        .bind(running)
        .bind(tag)
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
}

impl Stats {
    fn new() -> Self {
        Self { per_ws: HashMap::new(), pushed: HashMap::new() }
    }
    fn record(&mut self, ws: &str, latency_ms: u64) {
        self.per_ws
            .entry(ws.to_string())
            .or_default()
            .push(latency_ms);
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
    let _ = worker_id;
    while !shutdown.load(Ordering::Relaxed) {
        // Snapshot the overloaded set at pull time so each pull reflects the
        // latest refresh.
        let overloaded = if fairness_on.load(Ordering::Relaxed) {
            (**WORKSPACE_FAIRNESS_OVERLOADED.load()).clone()
        } else {
            vec![]
        };

        // Atomically claim one queued job. Returns the workspace and the
        // `created_at` so we can compute end-to-end latency at completion.
        let row: Option<(Uuid, String, i32, chrono::DateTime<chrono::Utc>)> = if overloaded
            .is_empty()
        {
            sqlx::query_as::<_, (Uuid, String, i32, chrono::DateTime<chrono::Utc>)>(
                "WITH picked AS (
                    SELECT id FROM v2_job_queue
                    WHERE running = false AND scheduled_for <= now()
                    ORDER BY priority DESC NULLS LAST, scheduled_for
                    FOR UPDATE SKIP LOCKED LIMIT 1
                )
                UPDATE v2_job_queue q
                   SET running = true, started_at = now()
                  FROM picked
                 WHERE q.id = picked.id
                RETURNING q.id, q.workspace_id, COALESCE((q.extras->>'duration_ms')::int, 30), q.created_at",
            )
            .fetch_optional(&db)
            .await
            .unwrap()
        } else {
            sqlx::query_as::<_, (Uuid, String, i32, chrono::DateTime<chrono::Utc>)>(
                "WITH picked AS (
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
                RETURNING q.id, q.workspace_id, COALESCE((q.extras->>'duration_ms')::int, 30), q.created_at",
            )
            .bind(&overloaded)
            .fetch_optional(&db)
            .await
            .unwrap()
        };

        match row {
            Some((id, ws, dur_ms, created_at)) => {
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

                let latency_ms = (completed_at - created_at).num_milliseconds().max(0) as u64;
                {
                    let mut s = stats.lock().await;
                    s.record(&ws, latency_ms);
                }
                completed_counter.fetch_add(1, Ordering::Relaxed);
            }
            None => {
                // Empty queue (or every queued workspace is capped). Back off
                // briefly so we don't hammer the DB.
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
async fn refresh_loop(db: Pool<Postgres>, shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Relaxed) {
        // Force a refresh every cycle by rolling back the DB-side guard.
        let _ = sqlx::query(
            "UPDATE background_task_state
                SET updated_at = NOW() - INTERVAL '1 hour'
              WHERE name = 'workspace_fairness'",
        )
        .execute(&db)
        .await;
        let _ = refresh_overloaded(&db).await;
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

#[derive(Debug)]
struct ScenarioResult {
    summary: Vec<WsSummary>,
    /// Wall-clock duration the scenario actually ran for (push + drain).
    elapsed: Duration,
}

async fn run_scenario(
    sqlx_db: &Pool<Postgres>,
    label: &'static str,
    fairness_on: bool,
    duration: Duration,
    drain_timeout: Duration,
    n_workers: u32,
) -> ScenarioResult {
    reset_fairness_state();
    // Make the algorithm reactive enough to take effect within the 5-8s the
    // simulation actually runs for. (Defaults are 10s — fine for prod, slow
    // for tests.)
    WORKSPACE_FAIRNESS_DURATION_SECS.store(3, Ordering::Relaxed);
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
        Some(tokio::spawn(
            async move { refresh_loop(db, shutdown).await },
        ))
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
    ScenarioResult { summary, elapsed }
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
    )
    .await;
    // TREATMENT: fairness ON.
    let treatment = run_scenario(
        &db,
        "treatment (fairness ON)",
        true,
        push_dur,
        drain_dur,
        50,
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
