use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::{
    external_ip::UNKNOWN_IP,
    worker::{insert_ping_query, set_worker_draining_query, update_worker_ping_main_loop_query},
    worker_job_stats::WorkerJobStats,
};

async fn insert_ping(db: &Pool<Postgres>, worker: &str, ip: Option<&str>) -> anyhow::Result<()> {
    register(db, worker, ip, None).await
}

async fn register(
    db: &Pool<Postgres>,
    worker: &str,
    ip: Option<&str>,
    draining: Option<bool>,
) -> anyhow::Result<()> {
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
        draining,
        db,
    )
    .await?;
    Ok(())
}

/// A worker re-registers mid-life once its external IP resolves. That registration must not
/// clear a draining flag the same process already published; only a worker starting up under
/// a reclaimed name resets it.
#[sqlx::test]
async fn only_a_starting_worker_resets_draining(db: Pool<Postgres>) -> anyhow::Result<()> {
    let draining = || async {
        sqlx::query_scalar::<_, bool>("SELECT draining FROM worker_ping WHERE worker = 'wk-drain'")
            .fetch_one(&db)
            .await
    };
    register(&db, "wk-drain", None, Some(false)).await?;
    set_worker_draining_query("wk-drain", &db).await?;
    register(&db, "wk-drain", Some("1.2.3.4"), None).await?;
    assert!(draining().await?);

    register(&db, "wk-drain", None, Some(false)).await?;
    assert!(!draining().await?);
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
    job_stats: Option<&WorkerJobStats>,
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
        job_stats,
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
    main_loop_ping(&db, "wk-last-job", Some((job_id, "admins")), None).await?;
    main_loop_ping(&db, "wk-last-job", None, None).await?;
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

/// Job outcome counters live in the worker's memory, so a worker restarting under the same name
/// gets them back only from the row it reclaims; a ping without counters must not erase them.
#[sqlx::test]
async fn reclaimed_row_returns_the_job_stats(db: Pool<Postgres>) -> anyhow::Result<()> {
    let reclaimed = |db: Pool<Postgres>| async move {
        insert_ping_query(
            "test-instance",
            "wk-stats",
            "default",
            None,
            &[],
            None,
            None,
            "test",
            None,
            None,
            None,
            false,
            Some(false),
            &db,
        )
        .await
    };
    assert!(reclaimed(db.clone()).await?.job_stats.is_none());

    let mut stats = WorkerJobStats::default();
    stats.record(true, 0);
    stats.record(false, 0);
    main_loop_ping(&db, "wk-stats", None, Some(&stats)).await?;
    main_loop_ping(&db, "wk-stats", None, None).await?;

    assert_eq!(reclaimed(db.clone()).await?.job_stats, Some(stats));
    Ok(())
}
