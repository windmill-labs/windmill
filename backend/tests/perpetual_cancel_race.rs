//! A cancel reaches the queue row first and the worker only later: one landing after the worker's
//! last read of that row completes the run carrying no `canceled_by` of its own, and a perpetual
//! script must still stop rather than restart as if nothing had been cancelled.

use serde_json::json;
use sqlx::{types::Json, Pool, Postgres};
use uuid::Uuid;
use windmill_queue::{add_completed_job, get_mini_completed_job};

const W_ID: &str = "test-workspace";
const PATH: &str = "u/test-user/loop";

async fn start_perpetual_run(db: &Pool<Postgres>) -> anyhow::Result<Uuid> {
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, \
            language, lock, restart_unless_cancelled) \
         VALUES ($1, 1, $2, '', '', 'echo', 'test-user', 'bash', '', true)",
    )
    .bind(W_ID)
    .bind(PATH)
    .execute(db)
    .await?;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, created_at, permissioned_as, \
            permissioned_as_email, kind, runnable_id, runnable_path, script_lang, tag, args, \
            visible_to_owner) \
         VALUES ($1, $2, 'test-user', now(), 'u/test-user', 'test@windmill.dev', 'script', 1, \
            $3, 'bash', 'bash', '{\"n\": 1}', true)",
    )
    .bind(id)
    .bind(W_ID)
    .bind(PATH)
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, started_at, tag) \
         VALUES ($1, $2, now(), true, now(), 'bash')",
    )
    .bind(id)
    .bind(W_ID)
    .execute(db)
    .await?;
    Ok(id)
}

/// The same cancel, one step earlier: still being written when the completion reads the queue row,
/// which reads it without a lock. Only the delete that follows waits for the writer.
#[sqlx::test(fixtures("base"))]
async fn a_cancel_still_being_written_is_kept(db: Pool<Postgres>) -> anyhow::Result<()> {
    let running = start_perpetual_run(&db).await?;
    let job = get_mini_completed_job(&running, W_ID, &db).await?.unwrap();

    let mut cancel = db.begin().await?;
    sqlx::query(
        "UPDATE v2_job_queue SET canceled_by = 'test-user', canceled_reason = 'stop' WHERE id = $1",
    )
    .bind(running)
    .execute(&mut *cancel)
    .await?;

    let completing = tokio::spawn({
        let db = db.clone();
        async move {
            add_completed_job(
                &db,
                &job,
                true,
                false,
                Json(&json!("done")),
                None,
                0,
                None,
                false,
                None,
                false,
            )
            .await
        }
    });
    // Long enough for the completion to have read the queue row and reached the delete, which is
    // where it waits for the cancel.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    cancel.commit().await?;
    completing.await??;

    let (status, canceled_by): (String, Option<String>) =
        sqlx::query_as("SELECT status::text, canceled_by FROM v2_job_completed WHERE id = $1")
            .bind(running)
            .fetch_one(&db)
            .await?;
    assert_eq!(status, "canceled", "the run is recorded as canceled");
    assert_eq!(canceled_by.as_deref(), Some("test-user"));

    let queued: Vec<Uuid> = sqlx::query_scalar(
        "SELECT q.id FROM v2_job_queue q JOIN v2_job j USING (id) \
         WHERE j.workspace_id = $1 AND j.runnable_path = $2",
    )
    .bind(W_ID)
    .bind(PATH)
    .fetch_all(&db)
    .await?;
    assert!(
        queued.is_empty(),
        "a canceled perpetual run queues no next one: {queued:?}"
    );
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn a_cancel_landing_after_the_worker_read_its_row_is_kept(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let running = start_perpetual_run(&db).await?;
    sqlx::query(
        "UPDATE v2_job_queue SET canceled_by = 'test-user', canceled_reason = 'stop' WHERE id = $1",
    )
    .bind(running)
    .execute(&db)
    .await?;

    let job = get_mini_completed_job(&running, W_ID, &db).await?.unwrap();
    add_completed_job(
        &db,
        &job,
        true,
        false,
        Json(&json!("done")),
        None,
        0,
        // What the worker carries when the cancel landed after it last read the row.
        None,
        false,
        None,
        false,
    )
    .await?;

    let (status, canceled_by): (String, Option<String>) =
        sqlx::query_as("SELECT status::text, canceled_by FROM v2_job_completed WHERE id = $1")
            .bind(running)
            .fetch_one(&db)
            .await?;
    assert_eq!(status, "canceled", "the run is recorded as canceled");
    assert_eq!(canceled_by.as_deref(), Some("test-user"));

    let queued: Vec<Uuid> = sqlx::query_scalar(
        "SELECT q.id FROM v2_job_queue q JOIN v2_job j USING (id) \
         WHERE j.workspace_id = $1 AND j.runnable_path = $2",
    )
    .bind(W_ID)
    .bind(PATH)
    .fetch_all(&db)
    .await?;
    assert!(
        queued.is_empty(),
        "a canceled perpetual run queues no next one: {queued:?}"
    );
    Ok(())
}
