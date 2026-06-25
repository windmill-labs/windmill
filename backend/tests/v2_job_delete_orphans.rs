//! v2_job no longer cascades to its sparse side tables `dispatch_event`,
//! `flow_conversation_message`, and `zombie_job_counter` — their `ON DELETE CASCADE`
//! foreign keys were dropped (migration `drop_v2_job_side_table_cascades`) to keep bulk
//! retention deletes cheap. Every path that deletes a v2_job must therefore clean those
//! tables explicitly. These tests assert no orphan side rows survive the three deletion
//! paths called out in the change: direct job deletion (`delete_jobs`), workspace deletion,
//! and schedule clearing.
//!
//! Uses runtime `sqlx::query` (not the compile-time macros) so no offline query cache is
//! needed, matching delete_after_secs.rs.

use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_test_utils::*;

const WS: &str = "test-workspace";

/// Insert one row in each side table that used to cascade from `job_id`.
async fn seed_side_rows(db: &Pool<Postgres>, job_id: Uuid) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO dispatch_event
            (workspace_id, producer_job_id, subscriber_path, asset_kind, asset_path, outcome)
         VALUES ($1, $2, 'f/sub', 'resource', 'f/res', 'dispatched')",
    )
    .bind(WS)
    .bind(job_id)
    .execute(db)
    .await?;

    let conv_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO flow_conversation (id, workspace_id, flow_path, created_by)
         VALUES ($1, $2, 'f/flow', 'test-user')",
    )
    .bind(conv_id)
    .bind(WS)
    .execute(db)
    .await?;
    // created_seq is assigned by a trigger; inserting a value is rejected.
    sqlx::query(
        "INSERT INTO flow_conversation_message (conversation_id, message_type, content, job_id)
         VALUES ($1, 'assistant', 'hi', $2)",
    )
    .bind(conv_id)
    .bind(job_id)
    .execute(db)
    .await?;

    sqlx::query("INSERT INTO zombie_job_counter (job_id, counter) VALUES ($1, 1)")
        .bind(job_id)
        .execute(db)
        .await?;
    Ok(())
}

async fn count(db: &Pool<Postgres>, sql: &str, job_id: Uuid) -> anyhow::Result<i64> {
    Ok(sqlx::query_scalar::<_, i64>(sql)
        .bind(job_id)
        .fetch_one(db)
        .await?)
}

/// (dispatch_event, flow_conversation_message, zombie_job_counter, v2_job) row counts for `job_id`.
async fn counts(db: &Pool<Postgres>, job_id: Uuid) -> anyhow::Result<(i64, i64, i64, i64)> {
    Ok((
        count(
            db,
            "SELECT count(*) FROM dispatch_event WHERE producer_job_id = $1",
            job_id,
        )
        .await?,
        count(
            db,
            "SELECT count(*) FROM flow_conversation_message WHERE job_id = $1",
            job_id,
        )
        .await?,
        count(
            db,
            "SELECT count(*) FROM zombie_job_counter WHERE job_id = $1",
            job_id,
        )
        .await?,
        count(db, "SELECT count(*) FROM v2_job WHERE id = $1", job_id).await?,
    ))
}

async fn insert_job(db: &Pool<Postgres>, job_id: Uuid) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO v2_job (id, workspace_id, kind) VALUES ($1, $2, 'script')")
        .bind(job_id)
        .bind(WS)
        .execute(db)
        .await?;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_delete_jobs_removes_side_rows(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let job_id = Uuid::new_v4();
    insert_job(&db, job_id).await?;
    seed_side_rows(&db, job_id).await?;
    assert_eq!(
        counts(&db, job_id).await?,
        (1, 1, 1, 1),
        "seed should create one row per table"
    );

    let mut conn = db.acquire().await?;
    windmill_common::jobs::delete_jobs(&mut conn, &[job_id]).await?;
    drop(conn);

    let (de, fcm, zombie, job) = counts(&db, job_id).await?;
    assert_eq!(
        (de, fcm, zombie, job),
        (0, 0, 0, 0),
        "delete_jobs left orphans: dispatch_event={de} flow_conversation_message={fcm} zombie_job_counter={zombie} v2_job={job}"
    );
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_clear_schedule_removes_side_rows(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    // clear_schedule deletes queued, non-running jobs whose v2_job is a schedule trigger.
    let job_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, kind, trigger_kind, trigger)
         VALUES ($1, $2, 'script', 'schedule', 'f/sched')",
    )
    .bind(job_id)
    .bind(WS)
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running)
         VALUES ($1, $2, now(), false)",
    )
    .bind(job_id)
    .bind(WS)
    .execute(&db)
    .await?;
    seed_side_rows(&db, job_id).await?;

    let mut tx = db.begin().await?;
    windmill_queue::schedule::clear_schedule(&mut tx, "f/sched", WS).await?;
    tx.commit().await?;

    let (de, fcm, zombie, job) = counts(&db, job_id).await?;
    assert_eq!(
        (de, fcm, zombie, job),
        (0, 0, 0, 0),
        "clear_schedule left orphans: dispatch_event={de} flow_conversation_message={fcm} zombie_job_counter={zombie} v2_job={job}"
    );
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_workspace_delete_removes_side_rows(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let job_id = Uuid::new_v4();
    insert_job(&db, job_id).await?;
    seed_side_rows(&db, job_id).await?;

    // SECRET_TOKEN is the base fixture's instance-superadmin token for test@windmill.dev.
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let resp = reqwest::Client::new()
        .delete(format!(
            "http://localhost:{port}/api/workspaces/delete/{WS}"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .send()
        .await?;
    let status = resp.status();
    assert!(
        status.is_success(),
        "delete workspace failed: {status} {}",
        resp.text().await?
    );

    let (de, fcm, zombie, job) = counts(&db, job_id).await?;
    assert_eq!(
        (de, fcm, zombie, job),
        (0, 0, 0, 0),
        "workspace deletion left orphans: dispatch_event={de} flow_conversation_message={fcm} zombie_job_counter={zombie} v2_job={job}"
    );
    Ok(())
}
