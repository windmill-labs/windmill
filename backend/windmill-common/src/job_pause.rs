/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Workspace job pause: while `workspace_job_pause.paused_until` is in the future, new top-level
//! jobs of the workspace are held as `unassigned_*` (recorded in `workspace_held_job`) or, when
//! their caller waits for the result, refused with [`Error::JobsPaused`].

use chrono::{DateTime, Utc};
use quick_cache::sync::Cache;
use sqlx::{Postgres, Transaction};

use crate::{
    error::{Error, Result},
    jobs::{JobKind, JobTriggerKind},
    DB,
};

/// Longest pause accepted. Held jobs are queued 30 days out, and a worker fails an `unassigned_*`
/// job it reaches, so a pause must always end, and its jobs be released, well before that.
pub const MAX_PAUSE_DAYS: i64 = 7;

/// Short because agent workers push jobs and do not see `notify_event` evictions.
const CACHE_TTL_SECS: i64 = 10;

lazy_static::lazy_static! {
    // Value: (paused_until, cached_at timestamp)
    pub static ref JOB_PAUSE_CACHE: Cache<String, (Option<DateTime<Utc>>, i64)> = Cache::new(5000);
}

pub const JOB_PAUSE_CHANGE_CHANNEL: &str = "notify_workspace_job_pause_change";

/// The deadline of the workspace's pause, `None` when it is not paused. A pause past its deadline
/// reads as over even before the monitor deletes it. Takes the caller's connection: `push` reads it
/// inside its own transaction, where asking the pool for a second connection can starve it, and
/// where a failed read has aborted the transaction anyway, so the error is returned.
pub async fn jobs_paused_until<'c, E: sqlx::PgExecutor<'c>>(
    db: E,
    w_id: &str,
) -> Result<Option<DateTime<Utc>>> {
    let now = Utc::now();
    let cached = JOB_PAUSE_CACHE
        .get(w_id)
        .filter(|(_, cached_at)| now.timestamp() - cached_at < CACHE_TTL_SECS);
    let until = match cached {
        Some((until, _)) => until,
        None => {
            let until = sqlx::query_scalar!(
                "SELECT paused_until FROM workspace_job_pause WHERE workspace_id = $1",
                w_id
            )
            .fetch_optional(db)
            .await?;
            JOB_PAUSE_CACHE.insert(w_id.to_string(), (until, now.timestamp()));
            until
        }
    };
    Ok(until.filter(|until| *until > now))
}

pub fn evict(w_id: &str) {
    JOB_PAUSE_CACHE.remove(w_id);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseDecision {
    Run,
    Hold,
    Reject,
}

/// What a pause does to a job pushed with no parent. A job with a parent (flow step, a running
/// job's child, an error handler) always runs: the work it belongs to already started.
///
/// A direct run carries no trigger path: `trigger_or_fallback` stamps nothing for a session and
/// `webhook` for any token, while every trigger and schedule names itself. An admin's direct run
/// is not held, so an admin can run migrations and smoke tests during a deploy. An admin token
/// used by an external webhook looks the same and is not held either.
pub fn pause_decision(
    kind: JobKind,
    trigger_kind: Option<&JobTriggerKind>,
    trigger_path: Option<&str>,
    is_admin: bool,
) -> PauseDecision {
    match kind {
        JobKind::Dependencies
        | JobKind::FlowDependencies
        | JobKind::AppDependencies
        | JobKind::DeploymentCallback
        | JobKind::Identity
        | JobKind::Noop
        | JobKind::FlowScript
        | JobKind::FlowNode
        | JobKind::AIAgent
        | JobKind::UnassignedScript
        | JobKind::UnassignedFlow
        | JobKind::UnassignedSinglestepFlow => return PauseDecision::Run,
        _ => (),
    }
    let direct =
        trigger_path.is_none() && matches!(trigger_kind, None | Some(JobTriggerKind::Webhook));
    if direct && is_admin {
        return PauseDecision::Run;
    }
    match kind {
        JobKind::Script | JobKind::Flow | JobKind::SingleStepFlow => PauseDecision::Hold,
        _ => PauseDecision::Reject,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PausePolicy {
    /// Part of a deploy or a migration: runs while paused.
    Exempt,
    /// The caller waits for the result, so a job that would be held is refused instead.
    Refuse,
}

tokio::task_local! {
    static PAUSE_POLICY: PausePolicy;
}

/// Run `f` with every job it pushes under `policy`. Task-locals do not cross `tokio::spawn`: the
/// push must happen on the task that entered the scope.
pub async fn with_policy<F: std::future::Future>(policy: PausePolicy, f: F) -> F::Output {
    PAUSE_POLICY.scope(policy, f).await
}

/// Runs `f` under [`PausePolicy::Exempt`] when `exempt`, as is.
pub async fn exempt_if<F: std::future::Future>(exempt: bool, f: F) -> F::Output {
    if exempt {
        with_policy(PausePolicy::Exempt, f).await
    } else {
        f.await
    }
}

pub fn current_policy() -> Option<PausePolicy> {
    PAUSE_POLICY.try_with(|p| *p).ok()
}

/// Applies the pause to a top-level job about to be pushed: `Ok(None)` runs it, `Ok(Some(until))`
/// holds it, an error refuses it.
pub async fn check_push<'c, E: sqlx::PgExecutor<'c>>(
    db: E,
    w_id: &str,
    kind: JobKind,
    trigger_kind: Option<&JobTriggerKind>,
    trigger_path: Option<&str>,
    is_admin: bool,
) -> Result<Option<DateTime<Utc>>> {
    let policy = current_policy();
    if policy == Some(PausePolicy::Exempt) {
        return Ok(None);
    }
    let Some(until) = jobs_paused_until(db, w_id).await? else {
        return Ok(None);
    };
    match pause_decision(kind, trigger_kind, trigger_path, is_admin) {
        PauseDecision::Run => Ok(None),
        PauseDecision::Hold if policy != Some(PausePolicy::Refuse) => Ok(Some(until)),
        PauseDecision::Hold | PauseDecision::Reject => Err(Error::JobsPaused { until }),
    }
}

/// Holds the workspace's queued jobs that have not started and that the pause would have held had
/// they been pushed now, keeping when each was due. A worker that already claimed a job keeps it.
///
/// Like [`release_held_jobs`] and [`resume`], it checks no permission: the caller must be a
/// workspace admin's request or the monitor.
pub async fn hold_queued_jobs(tx: &mut Transaction<'_, Postgres>, w_id: &str) -> Result<i64> {
    // Extending a pause moves the jobs it already holds out of reach again, so a pause renewed
    // past 30 days never lets a worker reach (and fail) one of them.
    sqlx::query!(
        "UPDATE v2_job_queue q SET scheduled_for = now() + interval '30 days'
        FROM workspace_held_job h
        WHERE h.job_id = q.id AND h.workspace_id = $1 AND NOT q.running",
        w_id
    )
    .execute(&mut **tx)
    .await?;
    sweep_queued_jobs(tx, w_id).await
}

/// The sweep half of [`hold_queued_jobs`], also run by the monitor while a pause lasts: a server
/// or worker whose cached read still says "not paused" queues jobs for up to the cache TTL.
///
/// Only jobs pushed before the pause reached every cache are judged here. Later ones were judged
/// by `check_push`, which knows who started them (an admin's on-behalf-of run is let through)
/// where these columns do not, so the sweep must not undo its decision.
async fn sweep_queued_jobs(tx: &mut Transaction<'_, Postgres>, w_id: &str) -> Result<i64> {
    let held = sqlx::query_scalar!(
        r#"WITH candidates AS (
            SELECT q.id, q.scheduled_for
            FROM v2_job_queue q
            JOIN v2_job j ON j.id = q.id
            JOIN workspace_job_pause wp ON wp.workspace_id = q.workspace_id
            LEFT JOIN job_perms p ON p.job_id = q.id
            WHERE q.workspace_id = $1
              AND j.created_at < wp.paused_at + make_interval(secs => $2)
              AND NOT q.running
              -- a flow requeued after its preprocessor is not running but has started
              AND q.started_at IS NULL
              AND j.parent_job IS NULL
              AND j.kind IN ('script', 'flow', 'singlestepflow')
              -- an admin's direct run, judged by who started it as `check_push` does: for an
              -- on-behalf-of run the job's permissions are someone else's
              AND NOT (
                    j.trigger IS NULL
                    AND (j.trigger_kind IS NULL OR j.trigger_kind = 'webhook')
                    AND (
                        (COALESCE(p.is_admin, false) AND j.permissioned_as = 'u/' || j.created_by)
                        OR EXISTS (SELECT 1 FROM usr u WHERE u.workspace_id = q.workspace_id
                            AND u.username = j.created_by AND u.is_admin)
                    )
              )
            FOR UPDATE OF q SKIP LOCKED
        ),
        held AS (
            INSERT INTO workspace_held_job (job_id, workspace_id, scheduled_for)
            SELECT id, $1, scheduled_for FROM candidates
            ON CONFLICT (job_id) DO NOTHING
        ),
        queued AS (
            UPDATE v2_job_queue q SET scheduled_for = now() + interval '30 days'
            FROM candidates c WHERE q.id = c.id
        ),
        kinds AS (
            UPDATE v2_job j SET kind = CASE j.kind
                    WHEN 'script' THEN 'unassigned_script'::JOB_KIND
                    WHEN 'flow' THEN 'unassigned_flow'::JOB_KIND
                    ELSE 'unassigned_singlestepflow'::JOB_KIND
                END
            FROM candidates c WHERE j.id = c.id
        )
        SELECT count(*) AS "count!" FROM candidates"#,
        w_id,
        (CACHE_TTL_SECS + 5) as f64,
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(held)
}

/// Releases every job the workspace's pause holds, each due when it was before being held, or now.
/// Jobs held by a suspended trigger are not in `workspace_held_job` and stay held.
async fn release_held_jobs(tx: &mut Transaction<'_, Postgres>, w_id: &str) -> Result<i64> {
    let released = sqlx::query_scalar!(
        r#"WITH held AS (
            DELETE FROM workspace_held_job WHERE workspace_id = $1
            RETURNING job_id, scheduled_for
        ),
        queued AS (
            UPDATE v2_job_queue q SET scheduled_for = GREATEST(h.scheduled_for, now())
            FROM held h
            WHERE q.id = h.job_id AND NOT q.running
            RETURNING q.id
        ),
        kinds AS (
            UPDATE v2_job j SET kind = CASE j.kind
                    WHEN 'unassigned_script' THEN 'script'::JOB_KIND
                    WHEN 'unassigned_flow' THEN 'flow'::JOB_KIND
                    WHEN 'unassigned_singlestepflow' THEN 'singlestepflow'::JOB_KIND
                    ELSE j.kind
                END
            FROM queued q WHERE j.id = q.id
        )
        SELECT count(*) AS "count!" FROM queued"#,
        w_id
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(released)
}

/// Hands an `unassigned_*` job a suspended trigger held over to the workspace pause, which then
/// releases it with the jobs it holds itself. Used when the trigger releases its jobs mid-pause;
/// should the pause end first, the monitor releases the row as one left without a pause.
pub async fn hold_released_trigger_job(
    tx: &mut Transaction<'_, Postgres>,
    w_id: &str,
    job_id: uuid::Uuid,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO workspace_held_job (job_id, workspace_id, scheduled_for) VALUES ($1, $2, now())
        ON CONFLICT (job_id) DO NOTHING",
        job_id,
        w_id,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Serializes pausing, resuming and the monitor's release for one workspace until `tx` ends, so a
/// release never runs on the strength of a "not paused" read that a new pause just made stale.
pub async fn lock_workspace(tx: &mut Transaction<'_, Postgres>, w_id: &str) -> Result<()> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtextextended('workspace_job_pause:' || $1, 0))",
        w_id
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Ends the pause of `w_id` and releases its jobs. Checks no permission: the caller must be a
/// workspace admin's request, holding [`lock_workspace`].
pub async fn resume(tx: &mut Transaction<'_, Postgres>, w_id: &str) -> Result<i64> {
    sqlx::query!(
        "DELETE FROM workspace_job_pause WHERE workspace_id = $1",
        w_id
    )
    .execute(&mut **tx)
    .await?;
    release_held_jobs(tx, w_id).await
}

/// Holds what slipped through since the last pass in workspaces still paused, ends every pause past
/// its deadline, and releases the jobs of workspaces that are no longer paused. Safe to run on
/// every server at once: the sweep skips locked rows and each release deletes the rows it acts on.
pub async fn maintain_pauses(db: &DB) -> Result<()> {
    let paused = sqlx::query_scalar!(
        "SELECT workspace_id FROM workspace_job_pause WHERE paused_until > now()"
    )
    .fetch_all(db)
    .await?;
    for w_id in paused {
        let mut tx = db.begin().await?;
        let held = sweep_queued_jobs(&mut tx, &w_id).await?;
        tx.commit().await?;
        if held > 0 {
            tracing::info!("job pause of workspace {w_id} held {held} more queued jobs");
        }
    }

    let workspaces = sqlx::query_scalar!(
        r#"SELECT workspace_id AS "workspace_id!" FROM workspace_job_pause WHERE paused_until <= now()
        UNION
        SELECT DISTINCT workspace_id FROM workspace_held_job h
        WHERE NOT EXISTS (SELECT 1 FROM workspace_job_pause p WHERE p.workspace_id = h.workspace_id)"#
    )
    .fetch_all(db)
    .await?;
    for w_id in workspaces {
        let mut tx = db.begin().await?;
        lock_workspace(&mut tx, &w_id).await?;
        let ended = sqlx::query!(
            "DELETE FROM workspace_job_pause WHERE workspace_id = $1 AND paused_until <= now()",
            &w_id
        )
        .execute(&mut *tx)
        .await?
        .rows_affected()
            > 0;
        let still_paused = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM workspace_job_pause WHERE workspace_id = $1) AS "exists!""#,
            &w_id
        )
        .fetch_one(&mut *tx)
        .await?;
        if still_paused {
            continue;
        }
        let released = release_held_jobs(&mut tx, &w_id).await?;
        tx.commit().await?;
        evict(&w_id);
        if ended {
            tracing::info!(
                "job pause of workspace {w_id} reached its deadline, released {released} jobs"
            );
            crate::feature_usage::log_feature_usage("workspace_job_pause", "resumed", "deadline");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decisions() {
        use PauseDecision::*;
        let http = Some(&JobTriggerKind::Http);
        let webhook = Some(&JobTriggerKind::Webhook);
        let schedule = Some(&JobTriggerKind::Schedule);
        assert_eq!(
            pause_decision(JobKind::Dependencies, None, None, false),
            Run
        );
        assert_eq!(
            pause_decision(JobKind::DeploymentCallback, None, None, false),
            Run
        );
        assert_eq!(pause_decision(JobKind::Script, None, None, true), Run);
        assert_eq!(pause_decision(JobKind::Flow, webhook, None, true), Run);
        assert_eq!(pause_decision(JobKind::Preview, None, None, true), Run);
        // Triggers and schedules run as their owner, who may be an admin.
        assert_eq!(
            pause_decision(JobKind::Script, http, Some("f/h"), true),
            Hold
        );
        assert_eq!(
            pause_decision(JobKind::Flow, schedule, Some("f/s"), true),
            Hold
        );
        assert_eq!(pause_decision(JobKind::Script, webhook, None, false), Hold);
        assert_eq!(
            pause_decision(JobKind::SingleStepFlow, http, Some("f/h"), false),
            Hold
        );
        assert_eq!(pause_decision(JobKind::Preview, None, None, false), Reject);
        assert_eq!(
            pause_decision(
                JobKind::AppScript,
                Some(&JobTriggerKind::App),
                Some("f/a"),
                true
            ),
            Reject
        );
    }
}
