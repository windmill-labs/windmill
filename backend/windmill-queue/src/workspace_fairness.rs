//! Per-workspace fairness for the shared worker pool (Enterprise feature).
//!
//! On multi-tenant deployments (notably `app.windmill.dev`, but any EE
//! cluster with a single shared worker group) a single workspace flooding
//! the queue with jobs can degrade quality of service for everyone else.
//! This module computes the set of "overloaded" workspaces whose share of
//! the worker pool must be capped, and the dispatch in `jobs.rs` uses a
//! **stochastic admission rule** at pull time: each pull flips a coin and
//! admits capped workspaces' jobs with probability `(cap_percent + ε)/100`.
//! Over many pulls the noisy workspace is held at a steady share around the
//! cap — there is no on/off oscillation, so victim latency stays flat
//! instead of breathing in/out with each refresh cycle.
//!
//! ## Detection signal
//!
//! A workspace is overloaded when, over the last `WORKSPACE_FAIRNESS_DURATION_SECS`
//! seconds, it has accounted for at least `WORKSPACE_FAIRNESS_MAX_PERCENT`% of
//! cluster activity. "Cluster activity" counts both currently-running jobs and
//! jobs completed within the window — this captures workspaces hogging slots
//! with long-running jobs **and** workspaces spamming many small short-lived
//! jobs (where no individual job's `started_at` is old, but the aggregate
//! throughput share dominates).
//!
//! ## Coordinated refresh
//!
//! The aggregation runs **at most once every `refresh_interval` seconds
//! cluster-wide**, regardless of fleet size. A single `UPDATE` statement on
//! `background_task_state` does double duty:
//!   1. The `WHERE updated_at < now() - $interval` predicate, combined with
//!      row-level locking, ensures only the first process to commit per cycle
//!      actually recomputes the value. Other processes that race in see the
//!      `WHERE` re-evaluated against the now-fresh row and update zero rows.
//!   2. The same round trip falls through to a plain `SELECT` (via
//!      `UNION ALL ... LIMIT 1`) so every caller reads the current value.
//!
//! Each process mirrors the result into [`WORKSPACE_FAIRNESS_OVERLOADED`]
//! which the pull path reads at near-zero cost.
//!
//! ## Enterprise gating
//!
//! The cap is an Enterprise feature. The `windmill-api-settings` setter
//! rejects writes from non-EE builds, and on a self-hosted single-tenant
//! deployment the operator simply leaves `workspace_fairness_enabled = false`
//! (the default). At runtime the dispatch checks `WORKSPACE_FAIRNESS_ENABLED`
//! only — when it is off the pull path is bit-identical to today's.

use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use rand::Rng;
use sqlx::{Pool, Postgres};

use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::audit::AuditAuthor;
use windmill_common::error::Result;
use windmill_common::worker::{
    WORKSPACE_FAIRNESS_DURATION_SECS, WORKSPACE_FAIRNESS_ENABLED,
    WORKSPACE_FAIRNESS_LAST_REFRESH_MICROS, WORKSPACE_FAIRNESS_MAX_PERCENT,
    WORKSPACE_FAIRNESS_MIN_TOTAL, WORKSPACE_FAIRNESS_OVERLOADED,
};

pub const TASK_STATE_NAME: &str = "workspace_fairness";

/// Refresh interval when no workspace is currently capped. Slower cadence to
/// keep DB load minimal during normal operation.
const IDLE_REFRESH_SECS: u32 = 5;

/// Refresh interval when at least one workspace is capped. Faster cadence so
/// the cap lifts promptly once load drops below threshold.
const ACTIVE_REFRESH_SECS: u32 = 2;

/// Hard cap on the size of the overloaded list bound into the pull query.
const MAX_OVERLOADED_RETURNED: i64 = 64;

/// Hysteresis margin added to `MAX_PERCENT` when computing the admission
/// probability. The noisy workspace ends up holding a share of pulls roughly
/// equal to `(MAX_PERCENT + ε)/100`, which keeps its measured share strictly
/// above the cap threshold so the cap doesn't auto-lift on each refresh
/// cycle. Set to 0 for an exact-cap policy (allowed but more flap-prone).
pub const ADMISSION_EPSILON_PERCENT: u32 = 5;

/// Whether the feature can be active in this process. EE is enforced at
/// settings-set time (see `windmill-api-settings`); at runtime we only check
/// the toggle so the hot path stays a single atomic load.
fn fairness_active() -> bool {
    WORKSPACE_FAIRNESS_ENABLED.load(Ordering::Relaxed)
}

/// Pull-time decision: should this pull admit capped workspaces?
///
/// Returns true with probability `(MAX_PERCENT + ADMISSION_EPSILON_PERCENT)/100`.
/// The dispatch in `jobs.rs` uses this to decide between the standard pull
/// (no fairness filter) and the fairness pull (excludes capped workspaces).
/// Over many pulls this converges to a steady-state share around the cap
/// without the on/off oscillation that a binary cap produces.
///
/// Fast path: a single atomic load + one f64 RNG draw. The RNG is the
/// thread-local default `rand::rng()` (Xoshiro256++ in current `rand`),
/// which is a few nanoseconds per call.
pub fn should_admit_capped() -> bool {
    let cap = WORKSPACE_FAIRNESS_MAX_PERCENT
        .load(Ordering::Relaxed)
        .clamp(1, 100) as f64;
    let p = (cap + ADMISSION_EPSILON_PERCENT as f64).min(100.0) / 100.0;
    rand::rng().random::<f64>() < p
}

#[derive(serde::Deserialize)]
struct FairnessState {
    #[serde(default)]
    overloaded: Vec<String>,
}

/// Lazy, non-blocking refresh entry point called from the pull path.
///
/// Cost on the hot path: one atomic load, optionally one compare-exchange. If
/// this process wins the per-interval CAS, the actual refresh is spawned as a
/// `tokio` task — the caller does not wait on it.
pub fn maybe_refresh_overloaded(db: &Pool<Postgres>) {
    if !fairness_active() {
        // Drain the cached list so the dispatch in jobs.rs falls back to the
        // unmodified pull queries within at most one pull cycle.
        if !WORKSPACE_FAIRNESS_OVERLOADED.load().is_empty() {
            WORKSPACE_FAIRNESS_OVERLOADED.store(Arc::new(vec![]));
        }
        return;
    }

    let interval_us = current_refresh_interval_micros();
    let now_us = chrono::Utc::now().timestamp_micros();
    let last = WORKSPACE_FAIRNESS_LAST_REFRESH_MICROS.load(Ordering::Relaxed);
    if now_us.saturating_sub(last) < interval_us {
        return;
    }
    // Single in-flight refresh per process per cycle. If someone beat us, give up.
    if WORKSPACE_FAIRNESS_LAST_REFRESH_MICROS
        .compare_exchange(last, now_us, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        return;
    }

    let db = db.clone();
    tokio::spawn(async move {
        match tokio::time::timeout(Duration::from_secs(5), refresh_overloaded(&db)).await {
            Ok(Ok(())) => {}
            // On failure, leave `LAST_REFRESH_MICROS` set to `now_us` (already done by the CAS
            // above). The next attempt therefore has to wait a full `current_refresh_interval`
            // — exactly the same cooldown as a successful refresh. Previously we wrote `0`
            // here, which removed the rate limit entirely and let every subsequent pull spawn
            // a fresh refresh task while the DB was under pressure (precisely the moment we
            // most need to back off).
            Ok(Err(e)) => {
                tracing::warn!("workspace fairness refresh failed: {e:#}");
            }
            Err(_) => {
                tracing::warn!("workspace fairness refresh timed out after 5s");
            }
        }
    });
}

fn current_refresh_interval_micros() -> i64 {
    let secs = if WORKSPACE_FAIRNESS_OVERLOADED.load().is_empty() {
        IDLE_REFRESH_SECS
    } else {
        ACTIVE_REFRESH_SECS
    };
    (secs as i64) * 1_000_000
}

/// Run the coordinated refresh.
///
/// The previous implementation used a single `INSERT ... ON CONFLICT DO UPDATE
/// WHERE updated_at < ...` statement, which had a fatal flaw: Postgres evaluates
/// the `VALUES` clause (including the expensive `v2_job_queue ∪ v2_job_completed`
/// aggregation inlined there) **for every contender** to build the proposed row,
/// before the conflict-row check decides whether to actually apply the update.
/// So every worker process re-ran the heavy aggregation each cycle, and the
/// claimed "one heavy aggregation per cycle cluster-wide" property did not hold.
///
/// This version splits the refresh into three small statements:
///   1. Claim: a cheap upsert with only constant `VALUES`. Returns `Some(...)`
///      iff this process won the right to refresh (row was either missing or
///      had a stale `updated_at`).
///   2. Winner-only: an `UPDATE ... SET value = ...` whose `SET` expression
///      contains the heavy aggregation. Postgres evaluates `SET` per row
///      matching `WHERE`; we only issue it when `won`, so the aggregation runs
///      exactly once per refresh cycle cluster-wide.
///   3. Read: every caller reads the current value (winner sees its own fresh
///      write; losers see whatever the winner-from-this-or-the-prior-cycle
///      wrote).
pub async fn refresh_overloaded(db: &Pool<Postgres>) -> Result<()> {
    let duration_secs = WORKSPACE_FAIRNESS_DURATION_SECS
        .load(Ordering::Relaxed)
        .clamp(1, i32::MAX as u32) as i32;
    let max_percent = WORKSPACE_FAIRNESS_MAX_PERCENT
        .load(Ordering::Relaxed)
        .clamp(1, 100) as i64;
    let min_total = WORKSPACE_FAIRNESS_MIN_TOTAL.load(Ordering::Relaxed) as i64;
    // Use the tighter of the two intervals as the cluster-wide guard. The
    // slower idle cadence is enforced by the per-process CAS gate in
    // `maybe_refresh_overloaded`; the DB-side guard only needs to prevent
    // two processes from racing into a refresh at the same time.
    let refresh_secs = ACTIVE_REFRESH_SECS as i32;

    // Step 1: claim. The VALUES clause is all constants — Postgres has no
    // expensive work to do for either the insert-side or the conflict-side.
    // Returns Some(true) for the unique winner per cycle, None for losers.
    let won = sqlx::query_scalar::<_, bool>(
        r#"
        INSERT INTO background_task_state (name, value, running, owner, updated_at)
        VALUES ($1, '{"overloaded":[]}'::jsonb, false, NULL, NOW())
        ON CONFLICT (name) DO UPDATE
            SET updated_at = NOW()
            WHERE background_task_state.updated_at
                < NOW() - make_interval(secs => $2::int)
        RETURNING true
        "#,
    )
    .bind(TASK_STATE_NAME)
    .bind(refresh_secs)
    .fetch_optional(db)
    .await?
    .is_some();

    // Winner-only: read the value the cluster currently holds, so we can emit
    // audit-log entries for workspaces that just entered or left the capped set.
    // We compare against the DB-stored prior list (not the per-process local
    // cache) so a freshly-restarted worker that happens to win the claim does
    // not emit spurious "newly capped" entries for the workspaces that were
    // already capped before it started.
    let prev_list_from_db: Vec<String> = if won {
        let prev_val: Option<serde_json::Value> =
            sqlx::query_scalar("SELECT value FROM background_task_state WHERE name = $1")
                .bind(TASK_STATE_NAME)
                .fetch_optional(db)
                .await?;
        match prev_val {
            Some(v) => serde_json::from_value::<FairnessState>(v)
                .map(|s| s.overloaded)
                .unwrap_or_default(),
            None => vec![],
        }
    } else {
        vec![]
    };

    // Step 2: winner-only aggregation + value write. `SET` is evaluated per
    // updated row, so issuing this statement only when `won` guarantees the
    // expensive aggregation never runs for a loser.
    if won {
        sqlx::query(
            r#"
            UPDATE background_task_state
            SET value = jsonb_build_object('overloaded', (
                WITH active AS (
                    SELECT workspace_id FROM v2_job_queue WHERE running = true
                    UNION ALL
                    SELECT workspace_id FROM v2_job_completed
                     WHERE completed_at > NOW() - make_interval(secs => $2::int)
                ),
                per_ws AS (
                    SELECT workspace_id, COUNT(*)::int8 AS c FROM active GROUP BY 1
                ),
                total AS (SELECT SUM(c)::int8 AS t FROM per_ws)
                SELECT COALESCE(jsonb_agg(workspace_id ORDER BY c DESC), '[]'::jsonb)
                FROM (
                    SELECT workspace_id, c FROM per_ws, total
                    WHERE total.t >= $3
                      AND per_ws.c * 100 >= $4 * total.t
                    ORDER BY c DESC
                    LIMIT $5
                ) capped
            ))
            WHERE name = $1
            "#,
        )
        .bind(TASK_STATE_NAME)
        .bind(duration_secs)
        .bind(min_total)
        .bind(max_percent)
        .bind(MAX_OVERLOADED_RETURNED)
        .execute(db)
        .await?;
    }

    // Step 3: read current state (winner reads its own fresh write).
    let row: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT value FROM background_task_state WHERE name = $1")
            .bind(TASK_STATE_NAME)
            .fetch_optional(db)
            .await?;

    let new_list: Vec<String> = match row {
        Some(value) => match serde_json::from_value::<FairnessState>(value) {
            Ok(s) => s.overloaded,
            Err(e) => {
                tracing::warn!("workspace fairness state parse error: {e:#}");
                vec![]
            }
        },
        None => vec![],
    };

    // Audit-log transitions (winner only). Skipping losers prevents N duplicate
    // entries per transition on a fleet of N worker processes.
    if won && prev_list_from_db != new_list {
        let prev_set: HashSet<&str> = prev_list_from_db.iter().map(String::as_str).collect();
        let new_set: HashSet<&str> = new_list.iter().map(String::as_str).collect();
        let newly_capped: Vec<&str> = new_list
            .iter()
            .map(String::as_str)
            .filter(|w| !prev_set.contains(w))
            .collect();
        let newly_uncapped: Vec<&str> = prev_list_from_db
            .iter()
            .map(String::as_str)
            .filter(|w| !new_set.contains(w))
            .collect();

        emit_transition_audit(
            db,
            &newly_capped,
            &newly_uncapped,
            max_percent,
            duration_secs,
            new_list.len(),
        )
        .await;
    }

    let prev = WORKSPACE_FAIRNESS_OVERLOADED.load();
    if **prev != new_list {
        tracing::info!(
            "workspace fairness overloaded set changed: {} -> {} ({:?})",
            prev.len(),
            new_list.len(),
            &new_list,
        );
        WORKSPACE_FAIRNESS_OVERLOADED.store(Arc::new(new_list));
    }

    Ok(())
}

/// Best-effort audit-log emission for workspaces entering or leaving the
/// capped set. Failures are logged but never propagated — the refresh cycle
/// must not abort just because an audit insert fails.
///
/// Entries are scoped to the `admins` workspace and carry the affected
/// workspace as the `resource` field, so the queue-metrics drawer (cloud-only)
/// can fetch a single time-ordered stream of cap transitions without merging
/// across workspaces. Cluster admins also see them in the standard `admins`
/// audit listing (no `all_workspaces=true` toggle required).
async fn emit_transition_audit(
    db: &Pool<Postgres>,
    newly_capped: &[&str],
    newly_uncapped: &[&str],
    max_percent: i64,
    duration_secs: i32,
    total_overloaded: usize,
) {
    if newly_capped.is_empty() && newly_uncapped.is_empty() {
        return;
    }

    let author = AuditAuthor {
        username: "system".to_string(),
        email: "system@windmill.dev".to_string(),
        username_override: None,
        token_prefix: None,
    };

    let max_percent_s = max_percent.to_string();
    let window_secs_s = duration_secs.to_string();
    let total_s = total_overloaded.to_string();

    for w in newly_capped {
        let mut params = std::collections::HashMap::<&str, &str>::new();
        params.insert("max_percent", &max_percent_s);
        params.insert("window_secs", &window_secs_s);
        params.insert("total_overloaded", &total_s);
        if let Err(e) = audit_log(
            db,
            &author,
            "workspace_fairness.capped",
            ActionKind::Update,
            "admins",
            Some(w),
            Some(params),
        )
        .await
        {
            tracing::warn!("failed to write workspace_fairness.capped audit for {w}: {e:#}");
        }
    }

    for w in newly_uncapped {
        if let Err(e) = audit_log(
            db,
            &author,
            "workspace_fairness.uncapped",
            ActionKind::Update,
            "admins",
            Some(w),
            None,
        )
        .await
        {
            tracing::warn!("failed to write workspace_fairness.uncapped audit for {w}: {e:#}");
        }
    }
}
