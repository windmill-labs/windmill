#![cfg(all(feature = "enterprise", feature = "private"))]

use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_queue::wake_deferred_concurrency_job;

const KEY: &str = "wake-key";
const LIMIT: i32 = 2;

/// A job of `KEY`, limited to `LIMIT`, scheduled an hour out; `deferred_secs_ago` marks it
/// as deferred by the limit after becoming due that long ago, as `apply_concurrency_limit`
/// records it.
async fn waiting_job(
    db: &Pool<Postgres>,
    deferred_secs_ago: Option<i64>,
    time_window_s: Option<i32>,
) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, concurrent_limit, concurrency_time_window_s)
         VALUES ($1, 'test-workspace', $2, $3)",
    )
    .bind(id)
    .bind(LIMIT)
    .bind(time_window_s)
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for)
         VALUES ($1, 'test-workspace', now() + interval '1 hour')",
    )
    .bind(id)
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO concurrency_key (key, job_id, original_scheduled_for)
         VALUES ($1, $2, now() - make_interval(secs => $3))",
    )
    .bind(KEY)
    .bind(id)
    .bind(deferred_secs_ago.map(|s| s as f64))
    .execute(db)
    .await?;
    Ok(id)
}

async fn due(db: &Pool<Postgres>, id: Uuid) -> anyhow::Result<bool> {
    Ok(
        sqlx::query_scalar("SELECT scheduled_for <= now() FROM v2_job_queue WHERE id = $1")
            .bind(id)
            .fetch_one(db)
            .await?,
    )
}

/// Completions wake the jobs the limit deferred first, as many as the free slots, never
/// a job scheduled for later or one limited per time window.
#[sqlx::test(fixtures("base"))]
async fn completions_wake_the_first_deferred_jobs(db: Pool<Postgres>) -> anyhow::Result<()> {
    let windowed = waiting_job(&db, Some(40), Some(10)).await?;
    let first = waiting_job(&db, Some(30), None).await?;
    let second = waiting_job(&db, Some(20), None).await?;
    let third = waiting_job(&db, Some(10), None).await?;
    let scheduled_later = waiting_job(&db, None, None).await?;

    for _ in 0..3 {
        wake_deferred_concurrency_job(&db, KEY).await?;
    }

    assert!(
        due(&db, first).await?,
        "the first deferred job was not woken"
    );
    assert!(
        due(&db, second).await?,
        "a free slot was left to the deferral estimate"
    );
    assert!(
        !due(&db, third).await?,
        "more jobs were woken than slots are free"
    );
    assert!(
        !due(&db, windowed).await?,
        "a job limited per time window was woken"
    );
    assert!(
        !due(&db, scheduled_later).await?,
        "a job scheduled for later was woken"
    );
    Ok(())
}
