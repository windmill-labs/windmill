//! Same-tag wait deadlocks: every live worker serving a tag is executing a job that has a child
//! queued on that same tag, so none of those children can start until a parent gives up.
//!
//! A parent that merely queued a child and carries on (fire-and-forget) looks the same from the
//! queue, so the detection alone is only ever reported. Failing a wait is reserved for a parent
//! that is provably waiting: the one polling for its own child's result.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use uuid::Uuid;

use crate::{error, DB};

lazy_static::lazy_static! {
    /// How long a child must have been startable without starting before its tag can be called
    /// deadlocked. It absorbs a worker that is between jobs or a parent that is about to finish.
    pub static ref TAG_DEADLOCK_GRACE_SECS: i64 = std::env::var("TAG_DEADLOCK_GRACE_SECS")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(60);

    static ref CACHE: Mutex<HashMap<String, (Instant, Option<TagDeadlock>)>> =
        Mutex::new(HashMap::new());

    static ref LAST_FAIL_FAST: Mutex<HashMap<String, Instant>> = Mutex::new(HashMap::new());
}

const CACHE_TTL: Duration = Duration::from_secs(5);
const FAIL_FAST_COOLDOWN: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub struct TagDeadlock {
    pub tag: String,
    /// Live workers serving the tag, all of them held by a blocked parent.
    pub workers: i64,
    /// The jobs holding those workers.
    pub parents: Vec<Uuid>,
}

/// Every tag (or only `tag`) whose live workers are all held by a running job with a child
/// queued on that tag for longer than the grace period.
///
/// A worker's job is read from `v2_job_queue` (`running`, `worker`), not from
/// `worker_ping.current_job_id`, which keeps naming the last job after the worker went idle.
/// Flow jobs are excluded because they hold no worker while their steps run, and so are
/// parked jobs (a suspended workflow-as-code parent keeps `running` but released its slot).
/// Native-mode workers run several jobs at once, so a tag one of them serves is never reported.
pub async fn deadlocked_tags(db: &DB, tag: Option<&str>) -> error::Result<Vec<TagDeadlock>> {
    let rows = sqlx::query!(
        r#"
        WITH blocked AS (
            SELECT DISTINCT r.worker, c.tag, r.id AS parent
            FROM v2_job_queue r
                JOIN v2_job rj ON rj.id = r.id
                JOIN v2_job cj ON cj.parent_job = r.id
                JOIN v2_job_queue c ON c.id = cj.id
            WHERE r.running AND r.worker IS NOT NULL AND r.suspend = 0 AND r.started_at IS NOT NULL
                AND rj.kind NOT IN ('flow', 'flowpreview', 'singlestepflow', 'flownode',
                                    'unassigned_flow', 'unassigned_singlestepflow')
                AND NOT c.running AND c.suspend = 0
                AND c.scheduled_for <= now() - make_interval(secs => $2::bigint::double precision)
                AND ($1::text IS NULL OR c.tag = $1)
        ),
        live AS (
            SELECT worker, unnest(custom_tags) AS tag, native_mode
            FROM worker_ping
            WHERE ping_at > now() - interval '60 seconds' AND draining IS NOT TRUE
        )
        SELECT l.tag AS "tag!", count(*) AS "workers!",
            array_agg(b.parent) FILTER (WHERE b.parent IS NOT NULL) AS "parents!"
        FROM live l
            LEFT JOIN LATERAL (
                SELECT parent FROM blocked b WHERE b.worker = l.worker AND b.tag = l.tag LIMIT 1
            ) b ON true
        WHERE l.tag IN (SELECT tag FROM blocked)
        GROUP BY l.tag
        HAVING count(*) = count(b.parent) AND NOT bool_or(l.native_mode)
        "#,
        tag,
        *TAG_DEADLOCK_GRACE_SECS,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| TagDeadlock { tag: r.tag, workers: r.workers, parents: r.parents })
        .collect())
}

/// The deadlock that the parent waiting on `child` is part of, if any: `child` has not started
/// past the grace period, and every worker of its tag, the parent's included, is held by a job
/// with a child stuck on that tag. `waiter` is the job known to be waiting; `None` takes the
/// child's own `parent_job`, for a caller that created the child and is waiting on it.
async fn deadlock_for_waiting_parent(
    db: &DB,
    child: Uuid,
    waiter: Option<Uuid>,
) -> error::Result<Option<TagDeadlock>> {
    let stuck = sqlx::query!(
        r#"SELECT q.tag, j.parent_job AS "parent!"
        FROM v2_job_queue q JOIN v2_job j ON j.id = q.id
        WHERE q.id = $1 AND j.parent_job = COALESCE($2, j.parent_job)
            AND NOT q.running AND q.suspend = 0
            AND q.scheduled_for <= now() - make_interval(secs => $3::bigint::double precision)"#,
        child,
        waiter,
        *TAG_DEADLOCK_GRACE_SECS,
    )
    .fetch_optional(db)
    .await?;
    let Some((tag, parent)) = stuck.map(|r| (r.tag, r.parent)) else {
        return Ok(None);
    };

    let cached = CACHE
        .lock()
        .unwrap()
        .get(&tag)
        .filter(|(at, _)| at.elapsed() < CACHE_TTL)
        .map(|(_, d)| d.clone());
    let deadlock = match cached {
        Some(d) => d,
        None => {
            let d = deadlocked_tags(db, Some(&tag)).await?.into_iter().next();
            let mut cache = CACHE.lock().unwrap();
            cache.retain(|_, (at, _)| at.elapsed() < CACHE_TTL);
            cache.insert(tag, (Instant::now(), d.clone()));
            d
        }
    };
    Ok(deadlock.filter(|d| d.parents.contains(&parent)))
}

/// The error to fail a parent's wait on `child` with when it is part of a same-tag deadlock
/// (see [`deadlock_for_waiting_parent`]), after raising the tag's alert.
///
/// One failed parent is enough to start draining the tag: its worker takes a child, that child's
/// parent completes, and so on. Until the failed parent has exited the tag still reads as
/// deadlocked, so without the cooldown every parent polling in that window would be failed too.
/// The cooldown is per process: each API server may fail one parent per tag per window.
pub async fn fail_fast_waiting_parent(
    db: &DB,
    child: Uuid,
    waiter: Option<Uuid>,
) -> error::Result<Option<error::Error>> {
    let Some(d) = deadlock_for_waiting_parent(db, child, waiter).await? else {
        return Ok(None);
    };
    {
        let mut last = LAST_FAIL_FAST.lock().unwrap();
        if last
            .get(&d.tag)
            .is_some_and(|at| at.elapsed() < FAIL_FAST_COOLDOWN)
        {
            return Ok(None);
        }
        last.retain(|_, at| at.elapsed() < FAIL_FAST_COOLDOWN);
        last.insert(d.tag.clone(), Instant::now());
    }
    alert_on_fail_fast(db, &d).await;
    Ok(Some(deadlock_error(child, &d)))
}

fn deadlock_error(child: Uuid, d: &TagDeadlock) -> error::Error {
    error::Error::Generic(
        hyper::StatusCode::CONFLICT,
        format!(
            "job {child} cannot start: every worker serving tag `{}` ({} worker{}) is running a job \
             that waits on a child queued on that same tag, this one included. Run the child on \
             another tag, add workers to `{}`, or use workflow-as-code so the parent releases its \
             worker while it waits.",
            d.tag,
            d.workers,
            if d.workers == 1 { "" } else { "s" },
            d.tag,
        ),
    )
}

const ALERT_RESOURCE_PREFIX: &str = "tag_deadlock:";

/// Tag deadlock alerts still open: their latest row is the alert, not its recovery. Read from
/// `alerts` so every server agrees and a restart does not re-alert.
async fn open_alerts(db: &DB) -> error::Result<Vec<String>> {
    Ok(sqlx::query_scalar!(
        r#"SELECT resource AS "resource!" FROM (
            SELECT DISTINCT ON (resource) resource, alert_type
            FROM alerts
            WHERE resource LIKE 'tag_deadlock:%' AND created_at > now() - interval '7 days'
            ORDER BY resource, created_at DESC, id DESC
        ) latest WHERE alert_type = 'critical_error'"#
    )
    .fetch_all(db)
    .await?)
}

async fn alert_unless_open(db: &DB, d: &TagDeadlock, open: &[String]) {
    let resource = format!("{ALERT_RESOURCE_PREFIX}{}", d.tag);
    if open.contains(&resource) {
        return;
    }
    crate::utils::report_critical_error(
        format!(
            "Tag `{}` looks deadlocked: all {} of its workers are running a job whose child has \
             been queued on `{}` for over {}s, so none of those children can start while those \
             jobs wait on them. Jobs holding the workers: {}",
            d.tag,
            d.workers,
            d.tag,
            *TAG_DEADLOCK_GRACE_SECS,
            d.parents
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        ),
        db.clone(),
        None,
        Some(&resource),
    )
    .await;
}

/// Alerts on a deadlock a waiting parent is about to be failed for. That failure frees a worker
/// within seconds, usually before the monitor's next pass could see the deadlock, so without
/// this the alert would rarely fire for parents that wait through the SDK or `run_wait_result`.
async fn alert_on_fail_fast(db: &DB, d: &TagDeadlock) {
    match open_alerts(db).await {
        Ok(open) => alert_unless_open(db, d, &open).await,
        Err(e) => tracing::warn!("could not read open tag deadlock alerts: {e}"),
    }
}

/// Raises a critical alert for each tag that became deadlocked and a recovered alert for each
/// one that no longer is.
pub async fn alert_deadlocked_tags(db: &DB) -> error::Result<()> {
    let deadlocked = deadlocked_tags(db, None).await?;
    let open = open_alerts(db).await?;
    for d in &deadlocked {
        alert_unless_open(db, d, &open).await;
    }
    for resource in open {
        let tag = &resource[ALERT_RESOURCE_PREFIX.len()..];
        if !deadlocked.iter().any(|d| d.tag == tag) {
            crate::utils::report_recovered_critical_error(
                format!("Tag `{tag}` is no longer deadlocked"),
                db.clone(),
                None,
                Some(&resource),
            )
            .await;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn live_worker(db: &DB, worker: &str) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO worker_ping (worker, worker_instance, ping_at, custom_tags)
             VALUES ($1, 'test', now(), ARRAY['t1'])",
        )
        .bind(worker)
        .execute(db)
        .await?;
        Ok(())
    }

    /// A job of `kind` running on `worker`, with one child queued on `t1` for two minutes.
    async fn parent_with_stuck_child(
        db: &DB,
        worker: &str,
        kind: &str,
    ) -> anyhow::Result<(Uuid, Uuid)> {
        let (parent, child) = (Uuid::new_v4(), Uuid::new_v4());
        sqlx::query("INSERT INTO v2_job (id, workspace_id, kind, tag) VALUES ($1, 'admins', $2::job_kind, 't1')")
            .bind(parent)
            .bind(kind)
            .execute(db)
            .await?;
        sqlx::query(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, started_at, worker, tag)
             VALUES ($1, 'admins', now(), true, now(), $2, 't1')",
        )
        .bind(parent)
        .bind(worker)
        .execute(db)
        .await?;
        sqlx::query(
            "INSERT INTO v2_job (id, workspace_id, kind, tag, parent_job) VALUES ($1, 'admins', 'script', 't1', $2)",
        )
        .bind(child)
        .bind(parent)
        .execute(db)
        .await?;
        sqlx::query(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, tag)
             VALUES ($1, 'admins', now() - interval '2 minutes', false, 't1')",
        )
        .bind(child)
        .execute(db)
        .await?;
        Ok((parent, child))
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn a_tag_is_deadlocked_only_when_every_worker_holds_a_blocked_parent(
        db: DB,
    ) -> anyhow::Result<()> {
        live_worker(&db, "w1").await?;
        live_worker(&db, "w2").await?;
        let (p1, c1) = parent_with_stuck_child(&db, "w1", "script").await?;
        assert!(deadlocked_tags(&db, None).await?.is_empty(), "w2 is free");

        // A flow holds no worker while its step waits, nor does a parked workflow-as-code parent.
        let (flow, _) = parent_with_stuck_child(&db, "w2", "flow").await?;
        assert!(deadlocked_tags(&db, None).await?.is_empty());
        sqlx::query("UPDATE v2_job SET kind = 'script' WHERE id = $1")
            .bind(flow)
            .execute(&db)
            .await?;
        sqlx::query("UPDATE v2_job_queue SET suspend = 1, started_at = NULL WHERE id = $1")
            .bind(flow)
            .execute(&db)
            .await?;
        assert!(deadlocked_tags(&db, None).await?.is_empty());

        sqlx::query("UPDATE v2_job_queue SET suspend = 0, started_at = now() WHERE id = $1")
            .bind(flow)
            .execute(&db)
            .await?;
        let d = deadlocked_tags(&db, Some("t1")).await?;
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].workers, 2);
        assert!(d[0].parents.contains(&p1) && d[0].parents.contains(&flow));

        // Only a parent of the polled child, itself holding one of the workers, is failed.
        assert!(deadlock_for_waiting_parent(&db, c1, Some(p1))
            .await?
            .is_some());
        assert!(deadlock_for_waiting_parent(&db, c1, None).await?.is_some());
        assert!(deadlock_for_waiting_parent(&db, c1, Some(flow))
            .await?
            .is_none());
        Ok(())
    }
}
