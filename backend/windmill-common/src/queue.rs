use std::collections::HashMap;

use sqlx::{Pool, Postgres};

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
}

/// Same backlog as [`get_queue_counts`], plus the delay of the job at the head of each
/// tag's queue. The head is picked with the same ordering the worker pull uses, so the
/// delay reported is the one a worker is about to observe.
pub async fn get_queue_stats(db: &Pool<Postgres>) -> HashMap<String, QueueStat> {
    // The head of each queue is found through `queue_sort_v2` rather than with an ordered
    // aggregate over the group, which would sort every backlogged row on every call.
    sqlx::query!(
        "SELECT c.tag AS \"tag!\", c.count AS \"count!\",
            EXTRACT(EPOCH FROM now() - d.scheduled_for)::double precision AS \"delay!\"
        FROM (
            SELECT tag, count(*) AS count FROM v2_job_queue WHERE
                scheduled_for <= now() - ('3 seconds')::interval AND running = false
                GROUP BY tag
        ) c
        CROSS JOIN LATERAL (
            SELECT scheduled_for FROM v2_job_queue
            WHERE tag = c.tag AND running = false
                AND scheduled_for <= now() - ('3 seconds')::interval
            ORDER BY priority DESC NULLS LAST, scheduled_for LIMIT 1
        ) d",
    )
    .fetch_all(db)
    .await
    .ok()
    .map(|v| {
        v.into_iter()
            .map(|x| (x.tag, QueueStat { count: x.count as u32, delay: x.delay }))
            .collect()
    })
    .unwrap_or_else(|| HashMap::new())
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
