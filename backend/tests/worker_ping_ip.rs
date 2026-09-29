use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::{
    external_ip::UNKNOWN_IP,
    worker::{insert_ping_query, update_worker_ping_main_loop_query},
};

async fn insert_ping(db: &Pool<Postgres>, worker: &str, ip: Option<&str>) -> anyhow::Result<()> {
    insert_ping_query(
        "test-instance",
        worker,
        "default",
        ip,
        &[],
        None,
        None,
        "test",
        None,
        None,
        None,
        false,
        db,
    )
    .await?;
    Ok(())
}

/// The external IP resolves in the background, so the initial ping often has none yet. That must
/// not blank the address a previous process wrote to the row this one reclaims — worker names are
/// stable across restarts under EXIT_AFTER_N_JOBS.
#[sqlx::test]
async fn unresolved_ip_keeps_the_reclaimed_rows_address(db: Pool<Postgres>) -> anyhow::Result<()> {
    insert_ping(&db, "wk-reclaimed", Some("1.2.3.4")).await?;
    insert_ping(&db, "wk-reclaimed", None).await?;
    let ip: String = sqlx::query_scalar("SELECT ip FROM worker_ping WHERE worker = $1")
        .bind("wk-reclaimed")
        .fetch_one(&db)
        .await?;
    assert_eq!(ip, "1.2.3.4");

    insert_ping(&db, "wk-fresh", None).await?;
    let ip: String = sqlx::query_scalar("SELECT ip FROM worker_ping WHERE worker = $1")
        .bind("wk-fresh")
        .fetch_one(&db)
        .await?;
    assert_eq!(ip, UNKNOWN_IP);
    Ok(())
}

async fn main_loop_ping(
    db: &Pool<Postgres>,
    worker: &str,
    last_job: Option<(Uuid, &str)>,
) -> anyhow::Result<()> {
    update_worker_ping_main_loop_query(
        worker,
        &[],
        None,
        None,
        Some(1),
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        None,
        last_job,
        db,
    )
    .await
}

/// The main-loop ping sends a job only once, so most pings carry none. Those must keep the last
/// job the row holds, whether this ping or the job poller wrote it.
#[sqlx::test]
async fn main_loop_ping_without_a_job_keeps_the_last_one(db: Pool<Postgres>) -> anyhow::Result<()> {
    insert_ping(&db, "wk-last-job", None).await?;
    let job_id = Uuid::new_v4();
    main_loop_ping(&db, "wk-last-job", Some((job_id, "admins"))).await?;
    main_loop_ping(&db, "wk-last-job", None).await?;
    let (current_job_id, workspace_id): (Option<Uuid>, Option<String>) = sqlx::query_as(
        "SELECT current_job_id, current_job_workspace_id FROM worker_ping WHERE worker = $1",
    )
    .bind("wk-last-job")
    .fetch_one(&db)
    .await?;
    assert_eq!(current_job_id, Some(job_id));
    assert_eq!(workspace_id.as_deref(), Some("admins"));
    Ok(())
}
