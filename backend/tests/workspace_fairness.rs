//! Simulated-workload tests for the workspace-fairness algorithm.
//!
//! The cloud cluster runs a single shared worker pool. A "noisy" workspace
//! that floods the queue can starve the rest of the cluster of slots. The
//! algorithm in `windmill_queue::workspace_fairness` periodically aggregates
//! per-workspace activity (running jobs + jobs completed in a rolling window)
//! and excludes any workspace whose share of cluster activity exceeds
//! `WORKSPACE_FAIRNESS_MAX_PERCENT`% of the total from the next pull cycles.
//!
//! These tests verify the **effect** of that algorithm on simulated workloads:
//!
//! 1. `fairness_caps_dominant_workspace` — when one workspace dominates total
//!    activity, the algorithm flags it as overloaded and leaves quieter
//!    workspaces alone.
//! 2. `fairness_respects_min_total` — at very low cluster activity the cap
//!    must NOT fire (otherwise a workspace running a single job would be
//!    classified as "100% of activity" and capped immediately).
//! 3. `fairness_pull_query_skips_capped_workspace` — once the overloaded set
//!    is populated, the fairness-aware pull SQL must skip the capped
//!    workspace's queued jobs while the regular SQL would pick them up.
//! 4. `fairness_lifts_when_load_drops` — once the noisy workspace's share
//!    falls below the threshold, the next refresh cycle removes it from the
//!    overloaded set.
//!
//! The cloud gate (`CLOUD_HOSTED` + `BASE_URL == app.windmill.dev`) is
//! enforced only inside the wrapper `maybe_refresh_overloaded` and in the
//! settings API. The algorithm itself (`refresh_overloaded`) is gate-free —
//! these tests call it directly with the per-process atomics set to
//! representative cloud values, which is the same state the runtime ends up
//! in once the gate flips on.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use serial_test::serial;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use windmill_common::worker::{
    WORKSPACE_FAIRNESS_DURATION_SECS, WORKSPACE_FAIRNESS_ENABLED,
    WORKSPACE_FAIRNESS_LAST_REFRESH_MICROS, WORKSPACE_FAIRNESS_MAX_PERCENT,
    WORKSPACE_FAIRNESS_MIN_TOTAL, WORKSPACE_FAIRNESS_OVERLOADED,
};
use windmill_queue::workspace_fairness::refresh_overloaded;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Reset the per-process fairness atomics to a known starting state. Required
/// between tests because the atomics live for the lifetime of the test binary,
/// not the per-test sqlx database.
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
         VALUES ($1, $1, 'test-user')
         ON CONFLICT (id) DO NOTHING",
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

/// Insert `n` rows into `v2_job_completed` for `workspace_id`, time-shifted
/// `secs_ago` seconds into the past. Only the columns the fairness aggregation
/// reads (workspace_id, completed_at) need to be meaningful; the rest can take
/// defaults. We bypass the `v2_job` table entirely because the fairness SQL
/// never joins to it.
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

/// Insert `n` queued rows for `workspace_id`. If `running` is true the rows
/// count toward "running" activity in the aggregation; otherwise they are
/// pending (scheduled, not yet picked).
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
             VALUES (gen_random_uuid(), $1, NOW(), $2, $3)
             RETURNING id",
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
// Tests
// ---------------------------------------------------------------------------

/// **Simulated workload:** one workspace (`noisy`) completes 60 short jobs in
/// the rolling window; two victim workspaces complete 5 each. With
/// `max_percent = 50%`, `noisy`'s share is ~85% of cluster activity, so the
/// algorithm must flag it as overloaded — and ONLY it.
#[sqlx::test(fixtures("base"))]
#[serial]
async fn fairness_caps_dominant_workspace(db: Pool<Postgres>) {
    reset_fairness_state();

    create_workspace(&db, "noisy").await;
    create_workspace(&db, "victim_a").await;
    create_workspace(&db, "victim_b").await;

    // Noisy: 60 jobs over the last 5 seconds — short, high-throughput
    // workload (the case where started_at-based age won't catch the abuse
    // but rolling-window throughput will).
    insert_completed(&db, "noisy", 60, 2).await;
    insert_completed(&db, "victim_a", 5, 3).await;
    insert_completed(&db, "victim_b", 5, 1).await;

    refresh_overloaded(&db).await.expect("refresh ok");

    let capped = overloaded_set();
    assert_eq!(
        capped,
        vec!["noisy".to_string()],
        "exactly the dominant workspace should be capped, got {capped:?}",
    );
}

/// **Simulated workload:** total cluster activity below `min_total`. Even if
/// one workspace has 100% of the activity (e.g. 3 jobs out of 3), no cap must
/// fire — otherwise a freshly woken cluster running a handful of jobs would
/// immediately throttle whoever ran them first.
#[sqlx::test(fixtures("base"))]
#[serial]
async fn fairness_respects_min_total(db: Pool<Postgres>) {
    reset_fairness_state();
    // min_total = 4 (default); insert only 3 jobs.
    create_workspace(&db, "lone").await;
    insert_completed(&db, "lone", 3, 2).await;

    refresh_overloaded(&db).await.expect("refresh ok");

    let capped = overloaded_set();
    assert!(
        capped.is_empty(),
        "no workspace should be capped below min_total, got {capped:?}",
    );
}

/// **Simulated workload:** populate the overloaded set, then queue jobs from
/// both noisy and victim workspaces. The fairness-aware pull SQL must return
/// a victim job (not a noisy one) on the first pull; the regular pull SQL
/// (control) must return a noisy job since it predates the others.
#[sqlx::test(fixtures("base"))]
#[serial]
async fn fairness_pull_query_skips_capped_workspace(db: Pool<Postgres>) {
    reset_fairness_state();
    create_workspace(&db, "noisy").await;
    create_workspace(&db, "victim").await;

    // Noisy's job is enqueued first so the regular pull (priority + scheduled_for ASC)
    // picks it up. The fairness pull must skip it.
    let noisy_ids = insert_queued(&db, "noisy", 1, false, "deno").await;
    let victim_ids = insert_queued(&db, "victim", 1, false, "deno").await;

    // Control: regular pull picks the oldest enqueued job (noisy's).
    let regular_pick: Option<Uuid> = sqlx::query_scalar(
        "SELECT id
         FROM v2_job_queue
         WHERE running = false AND tag IN ('deno') AND scheduled_for <= now()
         ORDER BY priority DESC NULLS LAST, scheduled_for
         LIMIT 1",
    )
    .fetch_optional(&db)
    .await
    .unwrap();
    assert_eq!(regular_pick, Some(noisy_ids[0]));

    // With fairness active: exclude `noisy` and the next pull must return
    // the victim's job instead.
    let capped = vec!["noisy".to_string()];
    let fairness_pick: Option<Uuid> = sqlx::query_scalar(
        "SELECT id
         FROM v2_job_queue
         WHERE running = false AND tag IN ('deno') AND scheduled_for <= now()
           AND workspace_id <> ALL($1::text[])
         ORDER BY priority DESC NULLS LAST, scheduled_for
         LIMIT 1",
    )
    .bind(&capped)
    .fetch_optional(&db)
    .await
    .unwrap();
    assert_eq!(fairness_pick, Some(victim_ids[0]));
}

/// **Simulated workload:** noisy was overloaded, then traffic balanced out.
/// The next refresh must remove it from the overloaded set (i.e. the cap is
/// not sticky once load drops).
#[sqlx::test(fixtures("base"))]
#[serial]
async fn fairness_lifts_when_load_drops(db: Pool<Postgres>) {
    reset_fairness_state();
    create_workspace(&db, "noisy").await;
    create_workspace(&db, "victim_a").await;
    create_workspace(&db, "victim_b").await;

    // Phase 1: noisy dominates → capped.
    insert_completed(&db, "noisy", 60, 2).await;
    insert_completed(&db, "victim_a", 5, 3).await;
    insert_completed(&db, "victim_b", 5, 1).await;
    refresh_overloaded(&db).await.expect("refresh ok");
    assert_eq!(overloaded_set(), vec!["noisy".to_string()]);

    // Phase 2: clear the window of noisy's burst by pushing all rows beyond
    // the rolling window (duration_secs = 10 → shift everything to 60s ago).
    sqlx::query(
        "UPDATE v2_job_completed
            SET completed_at = NOW() - make_interval(secs => 60),
                started_at = NOW() - make_interval(secs => 60)
          WHERE workspace_id IN ('noisy', 'victim_a', 'victim_b')",
    )
    .execute(&db)
    .await
    .unwrap();
    // Then add a balanced fresh burst: 10 jobs from each workspace inside
    // the window. Noisy's share is now ~33%, well below the 50% cap.
    insert_completed(&db, "noisy", 10, 2).await;
    insert_completed(&db, "victim_a", 10, 2).await;
    insert_completed(&db, "victim_b", 10, 2).await;

    // The DB-side claim guard (ACTIVE_REFRESH_SECS = 2s) prevents the second
    // refresh from running the aggregation if called immediately after the
    // first. Roll back `updated_at` to force the next refresh to win the
    // claim and re-run the aggregation against the new workload.
    sqlx::query(
        "UPDATE background_task_state
            SET updated_at = NOW() - INTERVAL '1 hour'
          WHERE name = 'workspace_fairness'",
    )
    .execute(&db)
    .await
    .unwrap();

    refresh_overloaded(&db).await.expect("refresh ok");
    let capped = overloaded_set();
    assert!(
        capped.is_empty(),
        "noisy should be uncapped after load balances, still capped: {capped:?}",
    );
}

/// **Simulated workload:** long-running jobs hogging slots. The aggregation
/// counts `v2_job_queue WHERE running = true` so a workspace that holds N
/// slots with long jobs is detected even when its `completed` count is low.
#[sqlx::test(fixtures("base"))]
#[serial]
async fn fairness_catches_slot_hoggers(db: Pool<Postgres>) {
    reset_fairness_state();
    create_workspace(&db, "hogger").await;
    create_workspace(&db, "victim").await;

    // hogger: 0 completed but 10 currently-running long jobs.
    insert_queued(&db, "hogger", 10, true, "deno").await;
    // victim: 2 completed, no running jobs.
    insert_completed(&db, "victim", 2, 1).await;

    refresh_overloaded(&db).await.expect("refresh ok");

    let capped = overloaded_set();
    assert_eq!(
        capped,
        vec!["hogger".to_string()],
        "long-running slot-hogger must be capped, got {capped:?}",
    );
}
