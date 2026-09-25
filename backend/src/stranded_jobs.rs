//! Pending jobs stuck on a tag no worker serves.
//!
//! Such a job waits forever, and every idle pull of every worker group walks past it in the
//! shared `queue_sort_v2` index. This pass alerts superadmins about them and, when
//! `cancel_stranded_jobs_after_days` is set, cancels them.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::{
    error,
    global_settings::{load_value_from_global_settings, CANCEL_STRANDED_JOBS_AFTER_DAYS_SETTING},
    utils::report_critical_error,
};
use windmill_queue::cancel_job;

/// How long a tag must go without a worker serving it before its pending jobs are reported.
/// Long enough that a group autoscaled to zero, or only started at night, is not reported.
const ALERT_WINDOW_SECS: i64 = 24 * 3600;

/// Same tag in the same workspace is reported at most once per this interval.
const ALERT_COOLDOWN_SECS: i64 = 24 * 3600;

/// New alerts raised per pass; the rest are raised on the next passes.
const MAX_ALERTS_PER_PASS: usize = 10;

/// Upper bound on cancellations per pass; a larger backlog drains over the next passes.
const MAX_CANCELS_PER_PASS: i64 = 500;

const MAX_CANCEL_AFTER_DAYS: i64 = 3650;

/// Next to the other monitor pass locks (737_483_920..=737_483_923).
const STRANDED_JOBS_LOCK_ID: i64 = 737_483_924;

const ALERT_RESOURCE_PREFIX: &str = "stranded_tag:";

pub async fn check_stranded_jobs(db: &Pool<Postgres>) {
    // Transaction-scoped so a monitor timeout dropping this future cannot leave the lock held
    // on a pooled connection (see `reconcile_unarmed_schedules`).
    let mut lock_tx = match db.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("stranded jobs: failed to begin lock tx: {e:#}");
            return;
        }
    };
    match sqlx::query_scalar::<_, bool>("SELECT pg_try_advisory_xact_lock($1)")
        .bind(STRANDED_JOBS_LOCK_ID)
        .fetch_one(&mut *lock_tx)
        .await
    {
        Ok(true) => {}
        Ok(false) => return,
        Err(e) => {
            tracing::error!("stranded jobs: advisory lock failed: {e:#}");
            return;
        }
    }

    let cancel_after_days = match cancel_after_days(db).await {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(
                "stranded jobs: could not read {CANCEL_STRANDED_JOBS_AFTER_DAYS_SETTING}: {e:#}"
            );
            None
        }
    };

    if let Err(e) = run_pass(db, cancel_after_days).await {
        tracing::error!("stranded jobs: {e:#}");
    }

    let _ = lock_tx.rollback().await;
}

async fn cancel_after_days(db: &Pool<Postgres>) -> error::Result<Option<i64>> {
    Ok(
        load_value_from_global_settings(db, CANCEL_STRANDED_JOBS_AFTER_DAYS_SETTING)
            .await?
            .and_then(|v| v.as_i64())
            .filter(|d| *d > 0)
            .map(|d| d.min(MAX_CANCEL_AFTER_DAYS)),
    )
}

async fn run_pass(db: &Pool<Postgres>, cancel_after_days: Option<i64>) -> error::Result<()> {
    // With no worker pinging at all over a window, every tag looks unserved: that is a fleet
    // outage, which would otherwise raise one alert per tag and cancel the whole queue.
    if !any_worker_pinged(db, ALERT_WINDOW_SECS).await? {
        tracing::warn!("stranded jobs: no worker pinged in the last 24 hours, skipping");
        return Ok(());
    }

    let groups = find_stranded(db, ALERT_WINDOW_SECS).await?;
    report_stranded_jobs(db, &groups, cancel_after_days).await?;

    if let Some(days) = cancel_after_days {
        let window_secs = days * 24 * 3600;
        let groups = if window_secs == ALERT_WINDOW_SECS {
            groups
        } else if any_worker_pinged(db, window_secs).await? {
            find_stranded(db, window_secs).await?
        } else {
            vec![]
        };
        cancel_stranded_jobs(db, &groups, days).await?;
    }
    Ok(())
}

async fn any_worker_pinged(db: &Pool<Postgres>, window_secs: i64) -> error::Result<bool> {
    Ok(sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM worker_ping
            WHERE ping_at > now() - $1::bigint * interval '1 second') AS "exists!""#,
        window_secs,
    )
    .fetch_one(db)
    .await?)
}

/// Stranded jobs of one tag in one workspace.
struct StrandedGroup {
    workspace_id: String,
    tag: String,
    count: i64,
    oldest: DateTime<Utc>,
}

/// Top-level pending jobs due for at least `window_secs` whose tag no worker that pinged within
/// the same window serves, grouped by workspace and tag. Children are covered by their root.
///
/// A worker pulls its `custom_tags`, which hold resolved tags (dedicated-worker tag included),
/// plus its own name prefix. Jobs carry their resolved tag (per-workspace suffix, `$workspace`)
/// and the pull matches it exactly, so exact comparison is the pull's own criterion. Agent
/// workers ping `worker_ping` through the API, so they count too. `worker_ping` keeps a worker's
/// current tags only: a tag removed from a group's config stops counting at the next ping.
async fn find_stranded(db: &Pool<Postgres>, window_secs: i64) -> error::Result<Vec<StrandedGroup>> {
    // MATERIALIZED keeps the queue driving the plan: joined freely, the planner may instead walk
    // every root job of `v2_job`, which holds the whole job history.
    Ok(sqlx::query_as!(
        StrandedGroup,
        r#"WITH served AS (
            SELECT unnest(custom_tags) AS tag FROM worker_ping
                WHERE ping_at > now() - $1::bigint * interval '1 second'
            UNION SELECT regexp_replace(worker, '-[^-]*$', '') FROM worker_ping
                WHERE ping_at > now() - $1::bigint * interval '1 second'
        ),
        pending AS MATERIALIZED (
            SELECT q.id, q.workspace_id, q.tag, q.scheduled_for FROM v2_job_queue q
            WHERE q.running = false
                AND q.canceled_by IS NULL
                AND q.scheduled_for <= now() - $1::bigint * interval '1 second'
                AND NOT EXISTS (SELECT 1 FROM served s WHERE s.tag = q.tag)
        )
        SELECT p.workspace_id AS "workspace_id!", p.tag AS "tag!", count(*) AS "count!",
            min(p.scheduled_for) AS "oldest!"
        FROM pending p JOIN v2_job j ON j.id = p.id
        WHERE j.parent_job IS NULL
        GROUP BY p.workspace_id, p.tag
        ORDER BY min(p.scheduled_for)"#,
        window_secs,
    )
    .fetch_all(db)
    .await?)
}

async fn report_stranded_jobs(
    db: &Pool<Postgres>,
    groups: &[StrandedGroup],
    cancel_after_days: Option<i64>,
) -> error::Result<()> {
    if groups.is_empty() {
        return Ok(());
    }

    let recently_alerted: HashSet<(String, String)> = sqlx::query!(
        r#"SELECT workspace_id AS "workspace_id!", resource AS "resource!" FROM alerts
        WHERE alert_type = 'critical_error'
            AND resource LIKE $1 || '%'
            AND workspace_id IS NOT NULL
            AND created_at > now() - $2::bigint * interval '1 second'"#,
        ALERT_RESOURCE_PREFIX,
        ALERT_COOLDOWN_SECS,
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|r| (r.workspace_id, r.resource))
    .collect();

    let now = Utc::now();
    let to_alert = groups
        .iter()
        .map(|g| (g, format!("{ALERT_RESOURCE_PREFIX}{}", g.tag)))
        .filter(|(g, resource)| {
            !recently_alerted.contains(&(g.workspace_id.clone(), resource.clone()))
        })
        .take(MAX_ALERTS_PER_PASS);
    for (StrandedGroup { workspace_id, tag, count, oldest }, resource) in to_alert {
        let cleanup = match cancel_after_days {
            Some(days) => format!(
                " Jobs whose tag no worker serves for {} are canceled automatically.",
                plural(days, "day")
            ),
            None => " Add the tag to a worker group, or cancel these jobs.".to_string(),
        };
        let message = format!(
            "Workspace {workspace_id} has {} with tag '{tag}', which no worker has served in the last {}. The oldest has waited {}.{cleanup}",
            plural(*count, "pending job"),
            fmt_duration(ALERT_WINDOW_SECS),
            fmt_duration((now - *oldest).num_seconds()),
        );
        tracing::warn!(workspace_id, tag, count, "stranded jobs: {message}");
        report_critical_error(
            message,
            db.clone(),
            Some(workspace_id.as_str()),
            Some(&resource),
        )
        .await;
    }
    Ok(())
}

async fn cancel_stranded_jobs(
    db: &Pool<Postgres>,
    groups: &[StrandedGroup],
    days: i64,
) -> error::Result<()> {
    let window_secs = days * 24 * 3600;
    let mut budget = MAX_CANCELS_PER_PASS;
    for group in groups {
        if budget <= 0 {
            break;
        }
        let ids = sqlx::query_scalar!(
            r#"WITH pending AS MATERIALIZED (
                SELECT q.id, q.scheduled_for FROM v2_job_queue q
                WHERE q.workspace_id = $1 AND q.tag = $2
                    AND q.running = false
                    AND q.canceled_by IS NULL
                    AND q.scheduled_for <= now() - $3::bigint * interval '1 second'
            )
            SELECT p.id FROM pending p JOIN v2_job j ON j.id = p.id
            WHERE j.parent_job IS NULL
            ORDER BY p.scheduled_for
            LIMIT $4"#,
            group.workspace_id,
            group.tag,
            window_secs,
            budget,
        )
        .fetch_all(db)
        .await?;
        budget -= ids.len() as i64;
        tracing::warn!(
            workspace_id = group.workspace_id,
            tag = group.tag,
            "stranded jobs: cancelling {} of {} pending job(s) on a tag no worker served in the last {days} day(s)",
            ids.len(),
            group.count,
        );
        let reason = format!(
            "no worker has served tag '{}' for {}",
            group.tag,
            plural(days, "day")
        );
        for id in ids {
            if let Err(e) =
                cancel_one(db, id, &group.workspace_id, reason.clone(), window_secs).await
            {
                tracing::error!("stranded jobs: could not cancel job {id}: {e:#}");
            }
        }
    }
    Ok(())
}

async fn cancel_one(
    db: &Pool<Postgres>,
    id: Uuid,
    workspace_id: &str,
    reason: String,
    window_secs: i64,
) -> error::Result<()> {
    let mut tx = db.begin().await?;
    // A worker for the tag may have come back since the job was selected. The row lock keeps the
    // pull (which skips locked rows) off it, and `canceled_by` is set under that lock because
    // `cancel_job` completes a pending job asynchronously: a worker pulling it in between
    // completes it as canceled instead of running it.
    let still_stranded = sqlx::query_scalar!(
        r#"UPDATE v2_job_queue q SET canceled_by = 'monitor', canceled_reason = $3
        WHERE q.id = $1 AND q.running = false AND q.canceled_by IS NULL
            AND NOT EXISTS (SELECT 1 FROM worker_ping w
                WHERE w.ping_at > now() - $2::bigint * interval '1 second'
                    AND (q.tag = ANY(w.custom_tags)
                        OR q.tag = regexp_replace(w.worker, '-[^-]*$', '')))
        RETURNING q.id"#,
        id,
        window_secs,
        reason,
    )
    .fetch_optional(&mut *tx)
    .await?
    .is_some();
    if !still_stranded {
        return Ok(());
    }
    let (tx, _) = cancel_job(
        "monitor",
        Some(reason),
        id,
        workspace_id,
        tx,
        db,
        false,
        false,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

fn fmt_duration(secs: i64) -> String {
    let hours = secs / 3600;
    if hours >= 48 {
        plural(hours / 24, "day")
    } else if hours >= 1 {
        plural(hours, "hour")
    } else {
        plural(secs / 60, "minute")
    }
}

fn plural(n: i64, unit: &str) -> String {
    if n == 1 {
        format!("1 {unit}")
    } else {
        format!("{n} {unit}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn plant(db: &Pool<Postgres>, tag: &str, age: &str, parent: Option<Uuid>) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO v2_job (id, workspace_id, tag, parent_job) VALUES ($1, 'admins', $2, $3)",
        )
        .bind(id)
        .bind(tag)
        .bind(parent)
        .execute(db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO v2_job_queue (id, workspace_id, tag, scheduled_for)
            VALUES ($1, 'admins', $2, now() - $3::interval)",
        )
        .bind(id)
        .bind(tag)
        .bind(age)
        .execute(db)
        .await
        .unwrap();
        id
    }

    async fn ping(db: &Pool<Postgres>, worker: &str, age: &str, tags: &[&str]) {
        sqlx::query(
            "INSERT INTO worker_ping (worker, worker_instance, ping_at, custom_tags)
            VALUES ($1, 'test', now() - $2::interval, $3)",
        )
        .bind(worker)
        .bind(age)
        .bind(tags)
        .execute(db)
        .await
        .unwrap();
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn find_stranded_selects_unserved_root_jobs(db: Pool<Postgres>) {
        ping(&db, "wk-live-a1b2", "1 minute", &["live"]).await;
        // A group scaled to zero within the window still serves its tags.
        ping(&db, "wk-night-c3d4", "10 hours", &["night"]).await;
        ping(&db, "wk-gone-e5f6", "2 days", &["gone"]).await;

        plant(&db, "nobody", "3 days", None).await;
        plant(&db, "gone", "3 days", None).await;
        let live = plant(&db, "live", "3 days", None).await;
        plant(&db, "night", "3 days", None).await;
        // A worker also pulls its own name prefix.
        plant(&db, "wk-live", "3 days", None).await;
        plant(&db, "nobody", "1 hour", None).await;
        plant(&db, "nobody", "-1 day", None).await;
        plant(&db, "nobody", "3 days", Some(live)).await;
        let running = plant(&db, "nobody", "3 days", None).await;
        sqlx::query("UPDATE v2_job_queue SET running = true WHERE id = $1")
            .bind(running)
            .execute(&db)
            .await
            .unwrap();

        let mut found: Vec<(String, i64)> = find_stranded(&db, ALERT_WINDOW_SECS)
            .await
            .unwrap()
            .into_iter()
            .map(|g| (g.tag, g.count))
            .collect();
        found.sort();
        assert_eq!(
            found,
            vec![("gone".to_string(), 1), ("nobody".to_string(), 1)]
        );

        // Picked up by a worker after selection: must not be interrupted.
        cancel_one(
            &db,
            running,
            "admins",
            "test".to_string(),
            ALERT_WINDOW_SECS,
        )
        .await
        .unwrap();
        let canceled_by: Option<String> =
            sqlx::query_scalar("SELECT canceled_by FROM v2_job_queue WHERE id = $1")
                .bind(running)
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(canceled_by, None);
    }
}
