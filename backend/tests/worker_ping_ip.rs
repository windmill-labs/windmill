use sqlx::{Pool, Postgres};
use windmill_common::{external_ip::UNKNOWN_IP, worker::insert_ping_query};

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
