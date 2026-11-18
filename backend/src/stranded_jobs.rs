//! Pending jobs stuck on a tag no worker serves.
//!
//! Such a job waits forever, and every idle pull of every worker group walks past it in the
//! shared `queue_sort_v2` index. This pass alerts superadmins about them and, when
//! `cancel_stranded_jobs_after_days` is set, cancels them.

use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::{
    error,
    global_settings::{
        load_value_from_global_settings, CANCEL_STRANDED_JOBS_AFTER_DAYS_SETTING,
        CRITICAL_ALERT_MUTE_STRANDED_JOBS_SETTING,
    },
    utils::report_critical_error,
};
use windmill_queue::cancel_job;

/// How long a tag must go without a worker serving it before its pending jobs are reported.
/// Long enough that a group autoscaled to zero, or only started at night, is not reported.
const ALERT_WINDOW_SECS: i64 = 24 * 3600;

/// At most one alert per this interval, however many tags are stranded.
const ALERT_COOLDOWN_SECS: i64 = 24 * 3600;

/// Tags listed in one alert; the rest are counted.
const MAX_LISTED_GROUPS: usize = 20;

/// Upper bound on cancellations per pass; a larger backlog drains over the next passes.
const MAX_CANCELS_PER_PASS: i64 = 500;

const MAX_CANCEL_AFTER_DAYS: i64 = 3650;

/// Next to the other monitor pass locks (737_483_920..=737_483_923).
const STRANDED_JOBS_LOCK_ID: i64 = 737_483_924;

const ALERT_RESOURCE: &str = "stranded_jobs";

/// Every cancel reason starts with it, which is how `finish_lost_cancels` recognises them.
const CANCEL_REASON_PREFIX: &str = "no worker has served tag '";

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
    finish_lost_cancels(db).await?;

    // With no worker pinging at all over a window, every tag looks unserved: that is a fleet
    // outage, which would otherwise raise one alert per tag or cancel the whole queue. Each
    // window gets its own check: a fleet quiet for a day may still show which tags went
    // unserved over a longer cancel window.
    let groups = if any_worker_pinged(db, ALERT_WINDOW_SECS).await? {
        find_stranded(db, ALERT_WINDOW_SECS).await?
    } else {
        tracing::warn!("stranded jobs: no worker pinged in the last 24 hours, not alerting");
        vec![]
    };
    report_stranded_jobs(db, &groups, cancel_after_days).await?;

    if let Some(days) = cancel_after_days {
        let window_secs = days * 24 * 3600;
        let groups = if window_secs == ALERT_WINDOW_SECS {
            groups
        } else if any_worker_pinged(db, window_secs).await? {
            find_stranded(db, window_secs).await?
        } else {
            tracing::warn!(
                "stranded jobs: no worker pinged in the last {days} days, not cancelling"
            );
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

/// Pending jobs due for at least `window_secs` whose tag no worker that pinged within the same
/// window serves, grouped by workspace and tag. A job whose parent is still queued is covered
/// by that parent; one whose parent is gone (a native script retry points at the completed
/// original) stands on its own.
///
/// A worker pulls its `custom_tags`, which hold resolved tags (dedicated-worker tag included),
/// the positive-priority `priority_tags` of its group config (which may be absent from
/// `custom_tags`), and its own name prefix. Jobs carry their resolved tag (per-workspace suffix, `$workspace`)
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
            UNION SELECT p.key
                FROM worker_ping w JOIN config c ON c.name = 'worker__' || w.worker_group,
                    jsonb_each(c.config->'priority_tags') p
                WHERE w.ping_at > now() - $1::bigint * interval '1 second'
                    AND jsonb_typeof(c.config->'priority_tags') = 'object'
                    AND jsonb_typeof(p.value) = 'number' AND p.value::text::numeric > 0
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
        WHERE (j.parent_job IS NULL
            OR NOT EXISTS (SELECT 1 FROM v2_job_queue pq WHERE pq.id = j.parent_job))
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
    let muted = load_value_from_global_settings(db, CRITICAL_ALERT_MUTE_STRANDED_JOBS_SETTING)
        .await?
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if muted {
        return Ok(());
    }
    // Read from `alerts` rather than kept in memory so the limit holds across servers.
    let alerted_recently = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM alerts
            WHERE alert_type = 'critical_error' AND resource = $1
                AND created_at > now() - $2::bigint * interval '1 second') AS "exists!""#,
        ALERT_RESOURCE,
        ALERT_COOLDOWN_SECS,
    )
    .fetch_one(db)
    .await?;
    if alerted_recently {
        return Ok(());
    }

    let now = Utc::now();
    let total: i64 = groups.iter().map(|g| g.count).sum();
    let mut message = format!(
        "{} waiting on tags that no worker has served in the last {}:",
        plural(total, "pending job"),
        fmt_duration(ALERT_WINDOW_SECS),
    );
    for g in groups.iter().take(MAX_LISTED_GROUPS) {
        message.push_str(&format!(
            "\n- workspace {}, tag '{}': {}, oldest waiting {}",
            g.workspace_id,
            g.tag,
            plural(g.count, "job"),
            fmt_duration((now - g.oldest).num_seconds()),
        ));
    }
    if groups.len() > MAX_LISTED_GROUPS {
        message.push_str(&format!(
            "\n- and {} more",
            plural((groups.len() - MAX_LISTED_GROUPS) as i64, "tag")
        ));
    }
    match cancel_after_days {
        Some(days) => message.push_str(&format!(
            "\nJobs whose tag no worker serves for {} are canceled automatically.",
            plural(days, "day")
        )),
        None => message.push_str(
            "\nAdd these tags to a worker group or cancel the jobs. To cancel such jobs automatically, set 'Cancel jobs on unserved tags after (days)' in Instance settings > Jobs.",
        ),
    }
    message.push_str(
        "\nThis alert is sent at most once a day. To turn it off, enable 'Mute stranded job alerts' in Instance settings > Alerts.",
    );

    tracing::warn!("stranded jobs: {message}");
    report_critical_error(message, db.clone(), None, Some(ALERT_RESOURCE)).await;
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
            WHERE (j.parent_job IS NULL
                OR NOT EXISTS (SELECT 1 FROM v2_job_queue pq WHERE pq.id = j.parent_job))
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
            "{CANCEL_REASON_PREFIX}{}' for {}",
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
    // completes it as canceled instead of running it. That completion is lost if the server
    // stops first; `finish_lost_cancels` picks those rows up.
    let still_stranded = sqlx::query_scalar!(
        r#"UPDATE v2_job_queue q SET canceled_by = 'monitor', canceled_reason = $3
        WHERE q.id = $1 AND q.running = false AND q.canceled_by IS NULL
            AND NOT EXISTS (SELECT 1 FROM worker_ping w
                LEFT JOIN config c ON c.name = 'worker__' || w.worker_group
                WHERE w.ping_at > now() - $2::bigint * interval '1 second'
                    AND (q.tag = ANY(w.custom_tags)
                        OR q.tag = regexp_replace(w.worker, '-[^-]*$', '')
                        OR (jsonb_typeof(c.config->'priority_tags'->q.tag) = 'number'
                            AND (c.config->'priority_tags'->>q.tag)::numeric > 0)))
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
    complete_canceled(tx, db, id, workspace_id, reason).await
}

/// Forced: the non-forced path leaves a job that has a parent to its worker, and no worker
/// serves this one. A job whose parent is still queued never gets here.
async fn complete_canceled(
    tx: sqlx::Transaction<'_, Postgres>,
    db: &Pool<Postgres>,
    id: Uuid,
    workspace_id: &str,
    reason: String,
) -> error::Result<()> {
    let (tx, _) = cancel_job(
        "monitor",
        Some(reason),
        id,
        workspace_id,
        tx,
        db,
        true,
        false,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Completes jobs an earlier pass marked canceled whose asynchronous completion was lost (the
/// server stopped first). Runs whatever the cancel setting now says: the decision was made.
async fn finish_lost_cancels(db: &Pool<Postgres>) -> error::Result<()> {
    let rows = sqlx::query!(
        r#"SELECT id, workspace_id, canceled_reason AS "reason!" FROM v2_job_queue
        WHERE running = false AND canceled_by = 'monitor' AND canceled_reason LIKE $1 || '%'
        LIMIT $2"#,
        CANCEL_REASON_PREFIX,
        MAX_CANCELS_PER_PASS,
    )
    .fetch_all(db)
    .await?;
    for row in rows {
        let mut tx = db.begin().await?;
        let still_marked = sqlx::query_scalar!(
            "SELECT id FROM v2_job_queue WHERE id = $1 AND running = false FOR UPDATE",
            row.id
        )
        .fetch_optional(&mut *tx)
        .await?
        .is_some();
        if still_marked {
            tracing::warn!("stranded jobs: completing canceled job {}", row.id);
            if let Err(e) = complete_canceled(tx, db, row.id, &row.workspace_id, row.reason).await {
                tracing::error!("stranded jobs: could not complete job {}: {e:#}", row.id);
            }
        }
    }
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

    async fn ping(db: &Pool<Postgres>, worker: &str, group: &str, age: &str, tags: &[&str]) {
        sqlx::query(
            "INSERT INTO worker_ping (worker, worker_instance, worker_group, ping_at, custom_tags)
            VALUES ($1, 'test', $2, now() - $3::interval, $4)",
        )
        .bind(worker)
        .bind(group)
        .bind(age)
        .bind(tags)
        .execute(db)
        .await
        .unwrap();
    }

    async fn set_canceled_by(db: &Pool<Postgres>, id: Uuid, by: Option<&str>) {
        sqlx::query("UPDATE v2_job_queue SET canceled_by = $2 WHERE id = $1")
            .bind(id)
            .bind(by)
            .execute(db)
            .await
            .unwrap();
    }

    async fn canceled_by(db: &Pool<Postgres>, id: Uuid) -> Option<String> {
        sqlx::query_scalar("SELECT canceled_by FROM v2_job_queue WHERE id = $1")
            .bind(id)
            .fetch_one(db)
            .await
            .unwrap()
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn find_stranded_selects_unserved_root_jobs(db: Pool<Postgres>) {
        ping(&db, "wk-live-a1b2", "live", "1 minute", &["live"]).await;
        // A group scaled to zero within the window still serves its tags.
        ping(&db, "wk-night-c3d4", "night", "10 hours", &["night"]).await;
        ping(&db, "wk-gone-e5f6", "gone", "2 days", &["gone"]).await;
        // A positive-priority tag is pulled even when absent from the worker's tags.
        ping(&db, "wk-prio-g7h8", "prio", "1 minute", &["other"]).await;
        sqlx::query(
            r#"INSERT INTO config (name, config) VALUES ('worker__prio', '{"priority_tags": {"prio": 2, "zero": 0}}')"#,
        )
        .execute(&db)
        .await
        .unwrap();

        let nobody = plant(&db, "nobody", "3 days", None).await;
        plant(&db, "gone", "3 days", None).await;
        let live = plant(&db, "live", "3 days", None).await;
        plant(&db, "night", "3 days", None).await;
        plant(&db, "prio", "3 days", None).await;
        // Priority 0 is only pulled when also in the worker's tags.
        plant(&db, "zero", "3 days", None).await;
        // A worker also pulls its own name prefix.
        plant(&db, "wk-live", "3 days", None).await;
        plant(&db, "nobody", "1 hour", None).await;
        plant(&db, "nobody", "-1 day", None).await;
        plant(&db, "nobody", "3 days", Some(live)).await;
        // A native retry points at the completed original, which has left the queue.
        let original = Uuid::new_v4();
        sqlx::query("INSERT INTO v2_job (id, workspace_id, tag) VALUES ($1, 'admins', 'retry')")
            .bind(original)
            .execute(&db)
            .await
            .unwrap();
        plant(&db, "retry", "3 days", Some(original)).await;
        let marked = plant(&db, "nobody", "3 days", None).await;
        set_canceled_by(&db, marked, Some("monitor")).await;
        let by_user = plant(&db, "nobody", "3 days", None).await;
        set_canceled_by(&db, by_user, Some("alice")).await;
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
            vec![
                ("gone".to_string(), 1),
                ("nobody".to_string(), 1),
                ("retry".to_string(), 1),
                ("zero".to_string(), 1)
            ]
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
        assert_eq!(canceled_by(&db, running).await, None);

        // Marked before its asynchronous completion, so a worker pulling it meanwhile skips it.
        cancel_one(&db, nobody, "admins", "test".to_string(), ALERT_WINDOW_SECS)
            .await
            .unwrap();
        assert_eq!(cancel_state(&db, nobody).await, Some("monitor".to_string()));
    }

    /// `canceled_by` of a job, whether still queued or already completed.
    async fn cancel_state(db: &Pool<Postgres>, id: Uuid) -> Option<String> {
        // Queue first: the completion moves the row to `v2_job_completed` in one transaction.
        let queued: Option<Option<String>> =
            sqlx::query_scalar("SELECT canceled_by FROM v2_job_queue WHERE id = $1")
                .bind(id)
                .fetch_optional(db)
                .await
                .unwrap();
        match queued {
            Some(by) => by,
            None => sqlx::query_scalar("SELECT canceled_by FROM v2_job_completed WHERE id = $1")
                .bind(id)
                .fetch_optional(db)
                .await
                .unwrap()
                .flatten(),
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn lost_cancels_are_completed(db: Pool<Postgres>) {
        let job = plant(&db, "nobody", "3 days", None).await;
        sqlx::query(
            "UPDATE v2_job_queue SET canceled_by = 'monitor', canceled_reason = $2 WHERE id = $1",
        )
        .bind(job)
        .bind(format!("{CANCEL_REASON_PREFIX}nobody' for 1 day"))
        .execute(&db)
        .await
        .unwrap();

        finish_lost_cancels(&db).await.unwrap();

        for _ in 0..50 {
            let done: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM v2_job_completed WHERE id = $1 AND status = 'canceled')",
            )
            .bind(job)
            .fetch_one(&db)
            .await
            .unwrap();
            if done {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        panic!("job was not completed as canceled");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn fleet_outage_neither_alerts_nor_cancels(db: Pool<Postgres>) {
        ping(&db, "wk-all-a1b2", "all", "2 days", &["gone"]).await;
        let job = plant(&db, "nobody", "3 days", None).await;

        run_pass(&db, Some(1)).await.unwrap();

        let alerts: i64 = sqlx::query_scalar("SELECT count(*) FROM alerts")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(alerts, 0);
        assert_eq!(canceled_by(&db, job).await, None);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn alerts_at_most_daily_and_can_be_muted(db: Pool<Postgres>) {
        let groups = vec![StrandedGroup {
            workspace_id: "admins".to_string(),
            tag: "nobody".to_string(),
            count: 3,
            oldest: Utc::now() - chrono::Duration::days(3),
        }];
        let alerts = || async {
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM alerts")
                .fetch_one(&db)
                .await
                .unwrap()
        };

        sqlx::query(
            "INSERT INTO global_settings (name, value) VALUES ('critical_alert_mute_stranded_jobs', 'true')",
        )
        .execute(&db)
        .await
        .unwrap();
        report_stranded_jobs(&db, &groups, None).await.unwrap();
        assert_eq!(alerts().await, 0);

        sqlx::query("DELETE FROM global_settings WHERE name = 'critical_alert_mute_stranded_jobs'")
            .execute(&db)
            .await
            .unwrap();
        report_stranded_jobs(&db, &groups, None).await.unwrap();
        report_stranded_jobs(&db, &groups, None).await.unwrap();
        assert_eq!(alerts().await, 1);
    }
}
