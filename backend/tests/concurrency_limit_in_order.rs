#![cfg(all(feature = "enterprise", feature = "private"))]

use sqlx::{Pool, Postgres};
use std::sync::atomic::Ordering;
use uuid::Uuid;
use windmill_common::worker::CONCURRENCY_LIMIT_IN_ORDER;
use windmill_queue::jobs_ee::update_concurrency_counter;

const KEY: &str = "in-order-key";

async fn queue_job(db: &Pool<Postgres>, due_secs_ago: i64) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO v2_job (id, workspace_id) VALUES ($1, 'test-workspace')")
        .bind(id)
        .execute(db)
        .await?;
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for)
         VALUES ($1, 'test-workspace', now() - make_interval(secs => $2))",
    )
    .bind(id)
    .bind(due_secs_ago as f64)
    .execute(db)
    .await?;
    sqlx::query("INSERT INTO concurrency_key (key, job_id) VALUES ($1, $2)")
        .bind(KEY)
        .bind(id)
        .execute(db)
        .await?;
    Ok(id)
}

async fn admit(db: &Pool<Postgres>, id: Uuid) -> anyhow::Result<bool> {
    Ok(update_concurrency_counter(
        db,
        &id,
        KEY.to_string(),
        serde_json::json!({ id.to_string(): {} }),
        id.to_string(),
        0,
        1,
    )
    .await?
    .0)
}

/// A job deferred by the limit keeps its place: a free slot is refused to a job that
/// became due after it, and that refusal makes the deferred job due again.
#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn free_slot_goes_to_the_job_first_in_line(db: Pool<Postgres>) -> anyhow::Result<()> {
    CONCURRENCY_LIMIT_IN_ORDER.store(true, Ordering::Relaxed);
    let first = queue_job(&db, 10).await?;
    let second = queue_job(&db, 5).await?;
    // `first` was deferred an hour out by the limit, as `apply_concurrency_limit` does.
    sqlx::query(
        "WITH place AS (
            UPDATE concurrency_key SET original_scheduled_for =
                (SELECT scheduled_for FROM v2_job_queue WHERE id = $1)
            WHERE job_id = $1
        )
        UPDATE v2_job_queue SET scheduled_for = now() + interval '1 hour' WHERE id = $1",
    )
    .bind(first)
    .execute(&db)
    .await?;

    let second_admitted = admit(&db, second).await;
    let first_due: bool =
        sqlx::query_scalar("SELECT scheduled_for <= now() FROM v2_job_queue WHERE id = $1")
            .bind(first)
            .fetch_one(&db)
            .await?;
    let first_admitted = admit(&db, first).await;
    // A job ahead of one that already holds the slot must not make it give the slot up
    // when it comes back through admission, as a resumed flow does.
    let earlier = queue_job(&db, 20).await?;
    let first_readmitted = admit(&db, first).await;
    let earlier_admitted = admit(&db, earlier).await;
    CONCURRENCY_LIMIT_IN_ORDER.store(false, Ordering::Relaxed);

    assert!(!second_admitted?, "a later job took the slot");
    assert!(first_due, "the job first in line was not woken");
    assert!(
        first_admitted?,
        "the job first in line was refused a free slot"
    );
    assert!(first_readmitted?, "a job holding the slot was refused it");
    assert!(!earlier_admitted?, "a job was admitted past the limit");
    Ok(())
}
