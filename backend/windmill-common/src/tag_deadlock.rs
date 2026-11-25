//! Same-tag wait deadlocks: every live worker serving a tag is executing a job that has a child
//! queued on that same tag, so none of those children can start until a parent gives up.
//!
//! A parent that merely queued a child and carries on (fire-and-forget), or that reads its
//! child's status between other work, looks the same from the queue, and its worker frees up
//! once it finishes. So a holder counts only while it is provably blocked on that very child:
//! the SDK wait helpers say so on each poll (`wait_for_child`), as does a sync `run_wait_result`,
//! and each such check marks the parent's queue row with the child (`extras.waiting_on`). A tag
//! is deadlocked only when every holder is freshly marked as waiting on one of its stuck
//! children; a parent waiting on a child of another tag is not one.
//!
//! Everything here reads the queue across workspaces and writes instance-level alerts: it is
//! for the server's own maintenance and wait paths, never a workspace-facing query.

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
}

const CACHE_TTL: Duration = Duration::from_secs(5);
/// How long a parent's last wait check on its stuck child counts as it still waiting. Checks
/// mark at most every 5s, so this spans several of them.
const WAITING_MARK_TTL_SECS: f64 = 30.0;

#[derive(Debug, Clone)]
pub struct TagDeadlock {
    pub tag: String,
    /// Live workers serving the tag, all of them held by a blocked parent.
    pub workers: i64,
    /// The jobs holding those workers.
    pub parents: Vec<Uuid>,
}

/// Every tag (or only `tag`) whose live workers are all held by a running job freshly marked as
/// waiting on its own child queued on that tag for longer than the grace period (see the module
/// doc). The child is reached through the mark rather than by walking the parent's children,
/// which on a backlog costs a lookup per queued child.
///
/// Instance-wide and unauthorized: the parents it returns belong to any workspace, so they may
/// reach superadmin alerts but not a workspace caller.
///
/// A worker's job is read from `v2_job_queue` (`running`, `worker`), not from
/// `worker_ping.current_job_id`, which keeps naming the last job after the worker went idle.
/// Flow jobs are excluded because they hold no worker while their steps run, and so are
/// parked jobs (a suspended workflow-as-code parent keeps `running` but released its slot).
/// Native-mode workers run several jobs at once, so a tag one of them serves is never reported.
pub async fn deadlocked_tags(
    db: impl sqlx::PgExecutor<'_>,
    tag: Option<&str>,
) -> error::Result<Vec<TagDeadlock>> {
    let rows = sqlx::query!(
        r#"
        WITH blocked AS (
            SELECT DISTINCT r.worker, c.tag, r.id AS parent
            FROM v2_job_queue r
                JOIN v2_job rj ON rj.id = r.id
                JOIN v2_job_queue c ON c.id = (r.extras->'waiting_on'->>'child')::uuid
                JOIN v2_job cj ON cj.id = c.id AND cj.parent_job = r.id
            WHERE r.running AND r.worker IS NOT NULL AND r.suspend = 0 AND r.started_at IS NOT NULL
                AND (r.extras->'waiting_on'->>'at')::timestamptz
                    > now() - make_interval(secs => $3)
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
        WAITING_MARK_TTL_SECS,
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
/// waiting on a child stuck on that tag. `waiter` is the job known to be waiting; `None` takes
/// the child's own `parent_job`, for a caller that created the child and is waiting on it.
/// Marks that parent as waiting on `child`, which is what lets the other holders' checks count it.
async fn deadlock_for_waiting_parent(
    db: &DB,
    child: Uuid,
    waiter: Option<Uuid>,
) -> error::Result<Option<(TagDeadlock, Uuid)>> {
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
    sqlx::query!(
        r#"UPDATE v2_job_queue
        SET extras = jsonb_set(COALESCE(extras, '{}'::jsonb), '{waiting_on}',
            jsonb_build_object('child', $2::uuid, 'at', now()))
        WHERE id = $1 AND running
            AND (extras->'waiting_on'->>'child' IS DISTINCT FROM $2::uuid::text
                OR (extras->'waiting_on'->>'at')::timestamptz < now() - interval '5 seconds')"#,
        parent,
        child,
    )
    .execute(db)
    .await?;

    let cached = CACHE
        .lock()
        .unwrap()
        .get(&tag)
        .filter(|(at, _)| at.elapsed() < CACHE_TTL)
        .map(|(_, d)| d.clone());
    let deadlock = match cached {
        Some(d) => d,
        None => {
            let d = deadlocked_tags(db, Some(&tag))
                .await?
                .into_iter()
                .next();
            let mut cache = CACHE.lock().unwrap();
            cache.retain(|_, (at, _)| at.elapsed() < CACHE_TTL);
            cache.insert(tag, (Instant::now(), d.clone()));
            d
        }
    };
    Ok(deadlock
        .filter(|d| d.parents.contains(&parent))
        .map(|d| (d, parent)))
}

/// The error to fail a parent's wait on `child` with when it is part of a same-tag deadlock
/// (see [`deadlock_for_waiting_parent`]), after raising the tag's alert.
///
/// One failed parent is enough to start draining the tag: its worker takes a child, that child's
/// parent completes, and so on. So failing it also clears its waiting mark: until it exits it
/// still holds its worker, and every other parent of the tag would otherwise be failed in that
/// window too. Under the tag's advisory lock the deadlock is read again rather than from the
/// cache, so a check that follows one failure sees that parent no longer waiting.
///
/// `waiter` must be the authenticated caller's own job, or `None` only when the caller created
/// `child` in this request: the check marks that parent as waiting and fails its wait.
pub async fn fail_fast_waiting_parent(
    db: &DB,
    child: Uuid,
    waiter: Option<Uuid>,
) -> error::Result<Option<error::Error>> {
    let Some((d, parent)) = deadlock_for_waiting_parent(db, child, waiter).await? else {
        return Ok(None);
    };
    let mut tx = db.begin().await?;
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtext('tag_deadlock:' || $1::text))",
        &d.tag
    )
    .fetch_one(&mut *tx)
    .await?;
    let Some(d) = deadlocked_tags(&mut *tx, Some(&d.tag))
        .await?
        .into_iter()
        .find(|d| d.parents.contains(&parent))
    else {
        return Ok(None);
    };
    sqlx::query!(
        "UPDATE v2_job_queue SET extras = extras - 'waiting_on' WHERE id = $1",
        parent
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
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
            "Tag `{}` is deadlocked: all {} of its workers are running a job waiting on a child \
             queued on `{}` for over {}s, which none of them can start. One waiting job is failed \
             to break it. Jobs holding the workers: {}",
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

/// Alerts on a deadlock a waiting parent is about to be failed for. This is the only place a tag
/// deadlock alert is raised: a deadlock counts only while its parents are polling with a fresh
/// waiting mark, and such a poll reaches this path within seconds.
async fn alert_on_fail_fast(db: &DB, d: &TagDeadlock) {
    match open_alerts(db).await {
        Ok(open) => alert_unless_open(db, d, &open).await,
        Err(e) => tracing::warn!("could not read open tag deadlock alerts: {e}"),
    }
}

/// Raises a recovered alert for each tag with an open deadlock alert that is no longer
/// deadlocked. Instance maintenance, run by the server monitor. The queue is only read while an
/// alert is open, so the monitor costs one read of `alerts` the rest of the time.
pub async fn recover_tag_deadlock_alerts(db: &DB) -> error::Result<()> {
    for resource in open_alerts(db).await? {
        let tag = &resource[ALERT_RESOURCE_PREFIX.len()..];
        if deadlocked_tags(db, Some(tag)).await?.is_empty() {
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

    async fn mark_waiting(db: &DB, parent: Uuid, child: Uuid) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE v2_job_queue SET extras = jsonb_build_object('waiting_on',
                jsonb_build_object('child', $2::uuid, 'at', now())) WHERE id = $1",
        )
        .bind(parent)
        .bind(child)
        .execute(db)
        .await?;
        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn a_tag_is_deadlocked_only_when_every_worker_holds_a_blocked_parent(
        db: DB,
    ) -> anyhow::Result<()> {
        live_worker(&db, "w1").await?;
        live_worker(&db, "w2").await?;
        let (p1, c1) = parent_with_stuck_child(&db, "w1", "script").await?;
        mark_waiting(&db, p1, c1).await?;
        assert!(deadlocked_tags(&db, None).await?.is_empty(), "w2 is free");

        // A flow holds no worker while its step waits, nor does a parked workflow-as-code parent.
        let (p2, c2) = parent_with_stuck_child(&db, "w2", "flow").await?;
        mark_waiting(&db, p2, c2).await?;
        assert!(deadlocked_tags(&db, None).await?.is_empty());
        sqlx::query("UPDATE v2_job SET kind = 'script' WHERE id = $1")
            .bind(p2)
            .execute(&db)
            .await?;
        sqlx::query("UPDATE v2_job_queue SET suspend = 1, started_at = NULL WHERE id = $1")
            .bind(p2)
            .execute(&db)
            .await?;
        assert!(deadlocked_tags(&db, None).await?.is_empty());

        sqlx::query("UPDATE v2_job_queue SET suspend = 0, started_at = now() WHERE id = $1")
            .bind(p2)
            .execute(&db)
            .await?;
        let d = deadlocked_tags(&db, Some("t1")).await?;
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].workers, 2);
        assert!(d[0].parents.contains(&p1) && d[0].parents.contains(&p2));
        sqlx::query("UPDATE v2_job_queue SET extras = extras - 'waiting_on'")
            .execute(&db)
            .await?;

        // p2 has not polled for its child, so it may be about to finish: no wait is failed yet.
        assert!(deadlock_for_waiting_parent(&db, c1, Some(p1))
            .await?
            .is_none());
        // Nor while p2 waits on a child of another tag: once that one runs, p2 frees its worker.
        let other = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO v2_job (id, workspace_id, kind, tag, parent_job)
             VALUES ($1, 'admins', 'script', 't2', $2)",
        )
        .bind(other)
        .bind(p2)
        .execute(&db)
        .await?;
        sqlx::query(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, tag)
             VALUES ($1, 'admins', now() - interval '2 minutes', false, 't2')",
        )
        .bind(other)
        .execute(&db)
        .await?;
        assert!(deadlock_for_waiting_parent(&db, other, Some(p2))
            .await?
            .is_none());
        CACHE.lock().unwrap().clear();
        assert!(deadlock_for_waiting_parent(&db, c1, Some(p1))
            .await?
            .is_none());
        CACHE.lock().unwrap().clear();
        // Once p2 waits too, the polling parent of a stuck child is part of the deadlock.
        assert!(deadlock_for_waiting_parent(&db, c2, Some(p2))
            .await?
            .is_some());
        assert!(deadlock_for_waiting_parent(&db, c1, None).await?.is_some());
        assert!(
            deadlock_for_waiting_parent(&db, c1, Some(p2))
                .await?
                .is_none(),
            "not its child"
        );

        // One failed parent drains the tag. It stops counting as waiting, so the other holder
        // is spared while the failed one still holds its worker, whichever process it polls.
        assert!(fail_fast_waiting_parent(&db, c1, Some(p1)).await?.is_some());
        assert!(fail_fast_waiting_parent(&db, c2, Some(p2)).await?.is_none());
        Ok(())
    }
}
