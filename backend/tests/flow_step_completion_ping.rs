//! A flow step's completion is the progress that keeps its flow from being reaped as a zombie: it
//! must refresh the parent's ping, unless the parent is being canceled.

use serde_json::json;
use sqlx::{types::Json, Pool, Postgres};
use uuid::Uuid;
use windmill_queue::{add_completed_job, get_mini_completed_job};

const W_ID: &str = "test-workspace";

async fn insert_running(
    db: &Pool<Postgres>,
    kind: &str,
    parent: Option<Uuid>,
    step_id: Option<&str>,
) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, created_at, permissioned_as, \
            permissioned_as_email, kind, runnable_path, script_lang, tag, visible_to_owner, \
            parent_job, flow_step_id) \
         VALUES ($1, $2, 'test-user', now(), 'u/test-user', 'test@windmill.dev', \
            $3::job_kind, 'u/test-user/f', 'bash', 'bash', true, $4, $5)",
    )
    .bind(id)
    .bind(W_ID)
    .bind(kind)
    .bind(parent)
    .bind(step_id)
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
    sqlx::query("INSERT INTO v2_job_runtime (id, ping) VALUES ($1, now() - interval '1 hour')")
        .bind(id)
        .execute(db)
        .await?;
    Ok(id)
}

async fn complete_step_and_read_parent_ping_age(
    db: &Pool<Postgres>,
    parent_canceled: bool,
) -> anyhow::Result<f64> {
    let parent = insert_running(db, "flow", None, None).await?;
    if parent_canceled {
        sqlx::query("UPDATE v2_job_queue SET canceled_by = 'test-user' WHERE id = $1")
            .bind(parent)
            .execute(db)
            .await?;
    }
    let step = insert_running(db, "script", Some(parent), Some("a")).await?;
    let job = get_mini_completed_job(&step, W_ID, db).await?.unwrap();
    add_completed_job(
        db,
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
    .await?;

    let (status, queued): (String, bool) = sqlx::query_as(
        "SELECT status::text, EXISTS (SELECT 1 FROM v2_job_queue WHERE id = $1) \
         FROM v2_job_completed WHERE id = $1",
    )
    .bind(step)
    .fetch_one(db)
    .await?;
    assert_eq!(status, "success");
    assert!(!queued, "the step left the queue");

    Ok(sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM now() - ping)::float8 FROM v2_job_runtime WHERE id = $1",
    )
    .bind(parent)
    .fetch_one(db)
    .await?)
}

#[sqlx::test(fixtures("base"))]
async fn a_step_completion_refreshes_its_running_flow_ping(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let age = complete_step_and_read_parent_ping_age(&db, false).await?;
    assert!(age < 60.0, "the parent's ping was refreshed: {age}s old");

    let age = complete_step_and_read_parent_ping_age(&db, true).await?;
    assert!(
        age > 3000.0,
        "a canceled parent's ping is left alone: {age}s old"
    );
    Ok(())
}

async fn ping_age(db: &Pool<Postgres>, id: Uuid) -> anyhow::Result<f64> {
    Ok(sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM now() - ping)::float8 FROM v2_job_runtime WHERE id = $1",
    )
    .bind(id)
    .fetch_one(db)
    .await?)
}

/// Waits for the completion to be stuck on the queue row of `id`.
async fn wait_until_blocked_on(db: &Pool<Postgres>, id: Uuid) -> anyhow::Result<()> {
    for _ in 0..100 {
        let blocked: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM pg_locks l JOIN v2_job_queue q \
               ON q.ctid = ('(' || l.page || ',' || l.tuple || ')')::tid \
             WHERE NOT l.granted AND l.locktype = 'tuple' AND q.id = $1 \
               AND l.database = (SELECT oid FROM pg_database WHERE datname = current_database())) \
             OR EXISTS (SELECT 1 FROM pg_stat_activity \
             WHERE datname = current_database() AND wait_event_type = 'Lock' \
               AND query LIKE '%DELETE FROM v2_job_queue WHERE id = $1%')",
        )
        .bind(id)
        .fetch_one(db)
        .await?;
        if blocked {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    anyhow::bail!("the completion never blocked on the step's queue row")
}

/// A cancel of the flow marks the flow and then its steps in one transaction. A step completing
/// while that is uncommitted waits on its own row, and must then leave the canceled flow's ping
/// alone although it began before the cancel was visible.
#[sqlx::test(fixtures("base"))]
async fn a_flow_cancel_landing_during_a_step_completion_keeps_the_flow_ping(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let parent = insert_running(&db, "flow", None, None).await?;
    let step = insert_running(&db, "script", Some(parent), Some("a")).await?;
    let job = get_mini_completed_job(&step, W_ID, &db).await?.unwrap();

    let mut cancel = db.begin().await?;
    sqlx::query(
        "UPDATE v2_job_queue SET canceled_by = 'test-user', canceled_reason = 'stop' \
         WHERE id = ANY($1)",
    )
    .bind(vec![parent, step])
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
    wait_until_blocked_on(&db, step).await?;
    cancel.commit().await?;
    completing.await??;

    let status: String =
        sqlx::query_scalar("SELECT status::text FROM v2_job_completed WHERE id = $1")
            .bind(step)
            .fetch_one(&db)
            .await?;
    assert_eq!(status, "canceled");
    let age = ping_age(&db, parent).await?;
    assert!(
        age > 3000.0,
        "the canceled flow's ping is left alone: {age}s old"
    );
    Ok(())
}
