//! Pending jobs stuck on a tag no worker serves.
//!
//! Such a job waits forever, and every idle pull of every worker group walks past it in the
//! shared `queue_sort_v2` index. This pass alerts superadmins about them and, when
//! `cancel_stranded_jobs_after_days` is set, cancels them.

use std::collections::{BTreeMap, HashSet};

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

/// Upper bound on cancellations per pass; a larger backlog drains over the next passes.
const MAX_CANCELS_PER_PASS: usize = 500;

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

    if let Err(e) = report_stranded_jobs(db, cancel_after_days).await {
        tracing::error!("stranded jobs: report failed: {e:#}");
    }
    if let Some(days) = cancel_after_days {
        if let Err(e) = cancel_stranded_jobs(db, days).await {
            tracing::error!("stranded jobs: cancellation failed: {e:#}");
        }
    }

    let _ = lock_tx.rollback().await;
}

async fn cancel_after_days(db: &Pool<Postgres>) -> error::Result<Option<i64>> {
    Ok(
        load_value_from_global_settings(db, CANCEL_STRANDED_JOBS_AFTER_DAYS_SETTING)
            .await?
            .and_then(|v| v.as_i64())
            .filter(|d| *d > 0),
    )
}

struct StrandedJob {
    id: Uuid,
    workspace_id: String,
    tag: String,
    scheduled_for: DateTime<Utc>,
}

/// Top-level pending jobs due for at least `window_secs` whose tag no worker that pinged within
/// the same window serves, oldest first. Children are covered by their root.
///
/// Jobs carry their resolved tag (per-workspace suffix, `$workspace`, dedicated-worker tag) and
/// the pull matches it exactly, so exact comparison is the pull's own criterion. Agent workers
/// ping `worker_ping` through the API, so they count too.
async fn find_stranded(db: &Pool<Postgres>, window_secs: i64) -> error::Result<Vec<StrandedJob>> {
    // MATERIALIZED keeps the queue driving the plan: joined freely, the planner may instead walk
    // every root job of `v2_job`, which holds the whole job history.
    Ok(sqlx::query_as!(
        StrandedJob,
        r#"WITH served AS (
            SELECT unnest(custom_tags) AS tag FROM worker_ping
                WHERE ping_at > now() - $1::bigint * interval '1 second'
            UNION SELECT unnest(dedicated_workers) FROM worker_ping
                WHERE ping_at > now() - $1::bigint * interval '1 second'
            UNION SELECT dedicated_worker FROM worker_ping
                WHERE ping_at > now() - $1::bigint * interval '1 second'
        ),
        pending AS MATERIALIZED (
            SELECT q.id, q.workspace_id, q.tag, q.scheduled_for FROM v2_job_queue q
            WHERE q.running = false
                AND q.canceled_by IS NULL
                AND q.scheduled_for <= now() - $1::bigint * interval '1 second'
                AND NOT EXISTS (SELECT 1 FROM served s WHERE s.tag = q.tag)
        )
        SELECT p.id AS "id!", p.workspace_id AS "workspace_id!", p.tag AS "tag!",
            p.scheduled_for AS "scheduled_for!"
        FROM pending p JOIN v2_job j ON j.id = p.id
        WHERE j.parent_job IS NULL
        ORDER BY p.scheduled_for"#,
        window_secs,
    )
    .fetch_all(db)
    .await?)
}

async fn report_stranded_jobs(
    db: &Pool<Postgres>,
    cancel_after_days: Option<i64>,
) -> error::Result<()> {
    let jobs = find_stranded(db, ALERT_WINDOW_SECS).await?;
    if jobs.is_empty() {
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

    // (workspace, tag) -> (count, oldest scheduled_for); jobs arrive oldest first.
    let mut groups: BTreeMap<(String, String), (usize, DateTime<Utc>)> = BTreeMap::new();
    for job in jobs {
        groups
            .entry((job.workspace_id, job.tag))
            .or_insert((0, job.scheduled_for))
            .0 += 1;
    }

    let now = Utc::now();
    for ((workspace_id, tag), (count, oldest)) in groups {
        let resource = format!("{ALERT_RESOURCE_PREFIX}{tag}");
        if recently_alerted.contains(&(workspace_id.clone(), resource.clone())) {
            continue;
        }
        let cleanup = match cancel_after_days {
            Some(days) => format!(
                " Jobs whose tag no worker serves for {} are canceled automatically.",
                plural(days, "day")
            ),
            None => " Add the tag to a worker group, or cancel these jobs.".to_string(),
        };
        let message = format!(
            "Workspace {workspace_id} has {} with tag '{tag}', which no worker has served in the last {}. The oldest has waited {}.{cleanup}",
            plural(count as i64, "pending job"),
            fmt_duration(ALERT_WINDOW_SECS),
            fmt_duration((now - oldest).num_seconds()),
        );
        tracing::warn!(workspace_id, tag, count, "stranded jobs: {message}");
        report_critical_error(message, db.clone(), Some(&workspace_id), Some(&resource)).await;
    }
    Ok(())
}

async fn cancel_stranded_jobs(db: &Pool<Postgres>, days: i64) -> error::Result<()> {
    let window_secs = days.saturating_mul(24 * 3600);

    // With no worker pinging at all over the window this is a fleet outage, not a tag nobody
    // serves: cancelling would empty the whole queue.
    let any_worker = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM worker_ping
            WHERE ping_at > now() - $1::bigint * interval '1 second') AS "exists!""#,
        window_secs,
    )
    .fetch_one(db)
    .await?;
    if !any_worker {
        tracing::warn!(
            "stranded jobs: no worker pinged in the last {days} day(s), not cancelling anything"
        );
        return Ok(());
    }

    let mut jobs = find_stranded(db, window_secs).await?;
    if jobs.is_empty() {
        return Ok(());
    }
    jobs.truncate(MAX_CANCELS_PER_PASS);
    tracing::warn!(
        "stranded jobs: cancelling {} pending job(s) on tags no worker served in the last {days} day(s)",
        jobs.len()
    );
    for job in jobs {
        let reason = format!(
            "no worker has served tag '{}' for {}",
            job.tag,
            plural(days, "day")
        );
        if let Err(e) = cancel_one(db, job.id, &job.workspace_id, reason).await {
            tracing::error!("stranded jobs: could not cancel job {}: {e:#}", job.id);
        }
    }
    Ok(())
}

async fn cancel_one(
    db: &Pool<Postgres>,
    id: Uuid,
    workspace_id: &str,
    reason: String,
) -> error::Result<()> {
    let tx = db.begin().await?;
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
        sqlx::query("INSERT INTO v2_job (id, workspace_id, tag, parent_job) VALUES ($1, 'admins', $2, $3)")
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

    async fn ping(db: &Pool<Postgres>, worker: &str, age: &str, tags: &[&str], dws: &[&str]) {
        sqlx::query(
            "INSERT INTO worker_ping (worker, worker_instance, ping_at, custom_tags, dedicated_workers)
            VALUES ($1, 'test', now() - $2::interval, $3, $4)",
        )
        .bind(worker)
        .bind(age)
        .bind(tags)
        .bind(dws)
        .execute(db)
        .await
        .unwrap();
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn find_stranded_selects_unserved_root_jobs(db: Pool<Postgres>) {
        ping(&db, "live", "1 minute", &["live"], &[]).await;
        // A group scaled to zero within the window still serves its tags.
        ping(&db, "night", "10 hours", &["night"], &[]).await;
        ping(&db, "gone", "2 days", &["gone"], &[]).await;
        ping(&db, "dedicated", "1 minute", &["other"], &["admins:f/dedicated"]).await;

        let nobody = plant(&db, "nobody", "3 days", None).await;
        let gone = plant(&db, "gone", "3 days", None).await;
        let live = plant(&db, "live", "3 days", None).await;
        plant(&db, "night", "3 days", None).await;
        plant(&db, "admins:f/dedicated", "3 days", None).await;
        plant(&db, "nobody", "1 hour", None).await;
        plant(&db, "nobody", "-1 day", None).await;
        plant(&db, "nobody", "3 days", Some(live)).await;
        let running = plant(&db, "nobody", "3 days", None).await;
        sqlx::query("UPDATE v2_job_queue SET running = true WHERE id = $1")
            .bind(running)
            .execute(&db)
            .await
            .unwrap();

        let mut found: Vec<Uuid> = find_stranded(&db, ALERT_WINDOW_SECS)
            .await
            .unwrap()
            .into_iter()
            .map(|j| j.id)
            .collect();
        found.sort();
        let mut expected = vec![nobody, gone];
        expected.sort();
        assert_eq!(found, expected);
    }
}
