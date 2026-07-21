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
    seed(db, key, count, offset_secs, false).await
}

async fn seed(db: &Pool<Postgres>, key: &str, count: usize, offset_secs: i64, running: bool) {
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
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag, running)
             VALUES ($1, 'test-workspace', now() + ($2::bigint::text || ' s')::interval, 'other', $3)",
            id,
            offset_secs,
            running,
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

/// Running jobs are not backlog. They stay in `v2_job_queue` with `ended_at` still NULL, so
/// counting them would charge a key for the concurrency it is licensed to use: a key whose
/// `concurrent_limit` exceeds the cap would reject every push with nothing actually waiting.
#[sqlx::test(migrations = "../migrations")]
async fn ignores_running_jobs(db: Pool<Postgres>) {
    let key = "running-not-backlog";
    seed(&db, key, 6, -10, true).await; // executing right now
    seed(&db, key, 2, 3600, false).await; // actually waiting

    let depth = concurrency_key_queue_depth(&db, key, 1000)
        .await
        .expect("count depth");

    assert_eq!(
        depth, 2,
        "only waiting jobs count; the 6 running ones must not"
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
