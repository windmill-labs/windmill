use std::collections::HashMap;

use sqlx::{Pool, Postgres};

/// The priority a job is pushed with. Workers pull `ORDER BY priority DESC NULLS LAST`, so a
/// job without priority is stored as 0 rather than NULL, which leaves negative values (low
/// priority) below it. Flow steps and dependency jobs run at 1 or above, ahead of jobs without
/// priority, to drain the queue faster; a low flow keeps its steps low.
pub fn pushed_job_priority(priority: Option<i16>, flow_step: bool, dependency_job: bool) -> i16 {
    match priority {
        _ if dependency_job => 1,
        Some(p) if p < 0 => p,
        p if flow_step => p.unwrap_or(0).max(1),
        p => p.unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::pushed_job_priority;

    #[test]
    fn low_priority_sorts_after_jobs_without_priority() {
        // (priority, flow step, dependency job) -> stored priority
        let cases = [
            ((None, false, false), 0),
            ((Some(100), false, false), 100),
            ((Some(-100), false, false), -100),
            ((None, true, false), 1),
            ((Some(0), true, false), 1),
            ((Some(50), true, false), 50),
            ((Some(-100), true, false), -100),
            ((None, false, true), 1),
        ];
        for ((p, step, dep), expected) in cases {
            assert_eq!(
                pushed_job_priority(p, step, dep),
                expected,
                "{p:?} step={step} dep={dep}"
            );
        }
    }
}

pub async fn get_queue_counts(db: &Pool<Postgres>) -> HashMap<String, u32> {
    sqlx::query!(
        "SELECT tag AS \"tag!\", count(*) AS \"count!\" FROM v2_job_queue WHERE
            scheduled_for <= now() - ('3 seconds')::interval AND running = false
            GROUP BY tag",
    )
    .fetch_all(db)
    .await
    .ok()
    .map(|v| v.into_iter().map(|x| (x.tag, x.count as u32)).collect())
    .unwrap_or_else(|| HashMap::new())
}

/// Backlog of a single tag: jobs waiting more than 3 seconds past their `scheduled_for`.
pub struct QueueStat {
    pub count: u32,
    /// How long the job that would be picked up next has already been waiting, in seconds.
    pub delay: f64,
    /// When that job started waiting (its `scheduled_for`), in epoch seconds.
    pub head_since: f64,
}

/// Same backlog as [`get_queue_counts`], plus the delay of the job at the head of each
/// tag's queue. The head is picked with the same ordering the worker pull uses, so the
/// delay reported is the one a worker is about to observe.
///
/// Reads the queue of every workspace: a caller exposing the result MUST restrict it to
/// devops users, as `GET /workers/queue_counts` does. Unlike [`get_queue_counts`], a failed
/// read is an error rather than an empty map, which would read as every backlog draining.
pub async fn get_queue_stats(
    db: &Pool<Postgres>,
) -> crate::error::Result<HashMap<String, QueueStat>> {
    // Grouping by (tag, priority) first finds every head in the same single pass as the
    // count. A per-tag `ORDER BY ... LIMIT 1` walks `queue_sort_v2`, whose `tag` column comes
    // last, through every other tag's backlog queued ahead of it.
    let rows = sqlx::query!(
        "SELECT tag AS \"tag!\", count AS \"count!\",
            EXTRACT(EPOCH FROM now() - head)::double precision AS \"delay!\",
            EXTRACT(EPOCH FROM head)::double precision AS \"head_since!\"
        FROM (
            SELECT tag, sum(n)::bigint AS count,
                (array_agg(head ORDER BY priority DESC NULLS LAST))[1] AS head
            FROM (
                SELECT tag, priority, count(*) AS n, min(scheduled_for) AS head
                FROM v2_job_queue WHERE
                    scheduled_for <= now() - ('3 seconds')::interval AND running = false
                    GROUP BY tag, priority
            ) g
            GROUP BY tag
        ) t",
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|x| {
            (
                x.tag,
                QueueStat { count: x.count as u32, delay: x.delay, head_since: x.head_since },
            )
        })
        .collect())
}

pub async fn get_queue_running_counts(db: &Pool<Postgres>) -> HashMap<String, u32> {
    sqlx::query!(
        "SELECT tag AS \"tag!\", count(*) AS \"count!\" FROM v2_job_queue WHERE
            running = true
            GROUP BY tag",
    )
    .fetch_all(db)
    .await
    .ok()
    .map(|v| v.into_iter().map(|x| (x.tag, x.count as u32)).collect())
    .unwrap_or_else(|| HashMap::new())
}
