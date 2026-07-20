//! Regression guard for `concurrency_key_queue_depth`, which backs the cloud-only cap on how
//! many jobs may queue behind a single concurrency key.
//!
//! Run with:
//!   cargo test -p windmill-queue --test concurrency_key_queue_depth_test

use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_queue::jobs::concurrency_key_queue_depth;

/// Queues `count` jobs on `key`, all scheduled `offset_secs` from now.
async fn seed_queued(db: &Pool<Postgres>, key: &str, count: usize, offset_secs: i64) {
    for _ in 0..count {
        let id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO v2_job (id, workspace_id, tag) VALUES ($1, 'test-workspace', 'other')",
            id,
        )
        .execute(db)
        .await
        .expect("seed v2_job");
        sqlx::query!(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag)
             VALUES ($1, 'test-workspace', now() + ($2::bigint::text || ' s')::interval, 'other')",
            id,
            offset_secs,
        )
        .execute(db)
        .await
        .expect("seed v2_job_queue");
        sqlx::query!(
            "INSERT INTO concurrency_key (key, job_id) VALUES ($1, $2)",
            key,
            id,
        )
        .execute(db)
        .await
        .expect("seed concurrency_key");
    }
}

/// The property the cap depends on: jobs scheduled into the future count toward the depth.
///
/// The concurrency limiter re-queues a blocked job by pushing `scheduled_for` forward one full
/// window at a time, so a backlog gated by a concurrency limit is almost entirely future-dated.
/// A depth query narrowed to `scheduled_for <= now()` would report only the trickle the limiter
/// has released and the cap would never fire on the runaway it exists to stop.
#[sqlx::test(migrations = "../migrations")]
async fn counts_future_scheduled_jobs(db: Pool<Postgres>) {
    let key = "future-scheduled-count";
    seed_queued(&db, key, 3, -10).await; // due now
    seed_queued(&db, key, 7, 3600).await; // parked an hour out by the limiter

    let depth = concurrency_key_queue_depth(&db, key, 1000)
        .await
        .expect("count depth");

    assert_eq!(
        depth, 10,
        "depth must include future-scheduled jobs; counting only due jobs would report 3"
    );
}

/// A job that left the queue without going through `add_completed_job` leaves its
/// `concurrency_key` row with `ended_at IS NULL` forever. Those must not count, otherwise a mass
/// cancel or manual purge leaves the key permanently wedged above the cap with an empty queue.
#[sqlx::test(migrations = "../migrations")]
async fn ignores_rows_whose_job_left_the_queue(db: Pool<Postgres>) {
    let key = "orphaned-rows";
    seed_queued(&db, key, 4, 3600).await;
    sqlx::query!("DELETE FROM v2_job_queue")
        .execute(&db)
        .await
        .expect("purge queue");

    let depth = concurrency_key_queue_depth(&db, key, 1000)
        .await
        .expect("count depth");

    assert_eq!(depth, 0, "orphaned concurrency_key rows must not count");
}

/// The count is bounded by `limit` so a saturated key cannot make every push scan its whole
/// backlog.
#[sqlx::test(migrations = "../migrations")]
async fn stops_counting_at_limit(db: Pool<Postgres>) {
    let key = "bounded-count";
    seed_queued(&db, key, 12, 3600).await;

    let depth = concurrency_key_queue_depth(&db, key, 5)
        .await
        .expect("count depth");

    assert_eq!(depth, 5, "count must saturate at the limit");
}
