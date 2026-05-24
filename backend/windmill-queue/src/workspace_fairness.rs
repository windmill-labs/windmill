//! Per-workspace fairness for the shared worker pool (cloud-only).
//!
//! On `app.windmill.dev` the cluster runs a single default worker group, so a
//! single workspace flooding the queue with jobs can degrade quality of service
//! for everyone else. This module computes the set of "overloaded" workspaces
//! that should be temporarily excluded from the pull query.
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
//! ## Cloud gating
//!
//! The feature is hard-gated to `CLOUD_HOSTED=true` **and** `BASE_URL` matching
//! `app.windmill.dev` (belt-and-suspenders against an on-prem instance importing
//! cloud's `global_settings` row). When either check fails, [`maybe_refresh_overloaded`]
//! and the pull-side dispatch both treat the feature as disabled.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use sqlx::{Pool, Postgres};

use windmill_common::error::Result;
use windmill_common::worker::{
    CLOUD_HOSTED, WORKSPACE_FAIRNESS_DURATION_SECS, WORKSPACE_FAIRNESS_ENABLED,
    WORKSPACE_FAIRNESS_LAST_REFRESH_MICROS, WORKSPACE_FAIRNESS_MAX_PERCENT,
    WORKSPACE_FAIRNESS_MIN_TOTAL, WORKSPACE_FAIRNESS_OVERLOADED,
};
use windmill_common::BASE_URL;

pub const TASK_STATE_NAME: &str = "workspace_fairness";

const APP_WINDMILL_DEV_HOST: &str = "app.windmill.dev";

/// Refresh interval when no workspace is currently capped. Slower cadence to
/// keep DB load minimal during normal operation.
const IDLE_REFRESH_SECS: u32 = 5;

/// Refresh interval when at least one workspace is capped. Faster cadence so
/// the cap lifts promptly once load drops below threshold.
const ACTIVE_REFRESH_SECS: u32 = 2;

/// Hard cap on the size of the overloaded list bound into the pull query.
const MAX_OVERLOADED_RETURNED: i64 = 64;

/// Read `BASE_URL` and check whether the cluster's base URL points at
/// `app.windmill.dev`. The setting is configured by the admin via `/base_url`
/// and is empty until the first settings load completes.
fn is_app_windmill_dev() -> bool {
    let base = BASE_URL.load();
    let s = base.as_str();
    if s.is_empty() {
        return false;
    }
    // Strip scheme then check the leading host segment. Avoids a new dependency
    // for what is a fixed-string match against one production host.
    let after_scheme = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
        .unwrap_or(s);
    let host = after_scheme.split('/').next().unwrap_or("");
    let host = host.split(':').next().unwrap_or("");
    host == APP_WINDMILL_DEV_HOST
}

/// Whether the feature can be active in this process. Combined gate:
///   - `WORKSPACE_FAIRNESS_ENABLED` setting toggled on, AND
///   - `CLOUD_HOSTED=true`, AND
///   - `BASE_URL` host is `app.windmill.dev`.
pub fn fairness_active() -> bool {
    WORKSPACE_FAIRNESS_ENABLED.load(Ordering::Relaxed) && *CLOUD_HOSTED && is_app_windmill_dev()
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
            Ok(Err(e)) => {
                tracing::warn!("workspace fairness refresh failed: {e:#}");
                // Reset the gate so the next pull can retry, but with the
                // current "active" interval as a floor to avoid stampeding.
                WORKSPACE_FAIRNESS_LAST_REFRESH_MICROS.store(0, Ordering::Relaxed);
            }
            Err(_) => {
                tracing::warn!("workspace fairness refresh timed out after 5s");
                WORKSPACE_FAIRNESS_LAST_REFRESH_MICROS.store(0, Ordering::Relaxed);
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

/// Run the coordinated refresh. Idempotent across processes: only one update
/// per cycle commits real work, everyone else does a cheap read.
async fn refresh_overloaded(db: &Pool<Postgres>) -> Result<()> {
    let duration_secs = WORKSPACE_FAIRNESS_DURATION_SECS
        .load(Ordering::Relaxed)
        .max(1) as i32;
    let max_percent = WORKSPACE_FAIRNESS_MAX_PERCENT
        .load(Ordering::Relaxed)
        .clamp(1, 100) as i64;
    let min_total = WORKSPACE_FAIRNESS_MIN_TOTAL.load(Ordering::Relaxed) as i64;
    // Use the tighter of the two intervals as the refresh predicate; the slower
    // idle cadence is enforced client-side by the CAS gate, while the SQL guard
    // protects against multiple in-flight refreshes from different processes.
    let refresh_secs = ACTIVE_REFRESH_SECS as i32;

    // Single statement: either we win the row-lock and recompute, or we read
    // whatever was just written by the winner.
    let row: Option<serde_json::Value> = sqlx::query_scalar(
        r#"
        WITH refreshed AS (
            INSERT INTO background_task_state (name, value, running, owner, updated_at)
            VALUES (
                $1,
                jsonb_build_object('overloaded', (
                    WITH active AS (
                        SELECT workspace_id FROM v2_job_queue WHERE running = true
                        UNION ALL
                        SELECT workspace_id FROM v2_job_completed
                         WHERE completed_at > now() - make_interval(secs => $2)
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
                )),
                false,
                NULL,
                NOW()
            )
            ON CONFLICT (name) DO UPDATE
                SET value = EXCLUDED.value,
                    updated_at = NOW()
                WHERE background_task_state.updated_at
                    < NOW() - make_interval(secs => $6)
            RETURNING value
        )
        SELECT value FROM refreshed
        UNION ALL
        SELECT value FROM background_task_state WHERE name = $1
        LIMIT 1
        "#,
    )
    .bind(TASK_STATE_NAME)
    .bind(duration_secs)
    .bind(min_total)
    .bind(max_percent)
    .bind(MAX_OVERLOADED_RETURNED)
    .bind(refresh_secs)
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
