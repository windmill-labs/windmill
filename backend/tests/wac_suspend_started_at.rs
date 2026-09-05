//! A WAC parent parked on a sleep, an approval or its children holds no worker,
//! so the queue row must not keep pointing at the segment that ran before it
//! parked: every path that completes a job without a worker-measured duration
//! falls back to `now() - started_at` and would record (and, on cloud, bill) the
//! whole wait as execution time.

use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_worker::wac_executor::suspend_wac_parent;

#[sqlx::test]
async fn wac_suspend_clears_started_at(db: Pool<Postgres>) -> anyhow::Result<()> {
    let job_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, started_at) \
         VALUES ($1, 'test-workspace', now(), true, now() - interval '4 days')",
    )
    .bind(job_id)
    .execute(&db)
    .await?;

    let mut tx = db.begin().await?;
    suspend_wac_parent(&mut tx, &job_id, "test-workspace", 1, 3600.0).await?;
    tx.commit().await?;

    let (started_at, running, suspend, suspend_until): (
        Option<chrono::DateTime<chrono::Utc>>,
        bool,
        i32,
        Option<chrono::DateTime<chrono::Utc>>,
    ) = sqlx::query_as(
        "SELECT started_at, running, suspend, suspend_until FROM v2_job_queue WHERE id = $1",
    )
    .bind(job_id)
    .fetch_one(&db)
    .await?;

    assert_eq!(
        started_at, None,
        "a parked parent must not carry the previous segment's started_at"
    );
    assert_eq!(suspend, 1);
    assert!(suspend_until.is_some());
    assert!(
        running,
        "running stays true so the normal pull query skips the parked row"
    );

    Ok(())
}
