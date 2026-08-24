//! Guards the `sqlx` entries in `[patch.crates-io]`.
//!
//! Upstream `Pool::begin` raises the transaction depth its rollback-on-drop guard keys on
//! only *after* the `BEGIN` round trip, so a cancelled caller leaves the session inside a
//! transaction nothing will end — and the pool hands that connection to the next borrower,
//! whose statements run inside it until an error turns every later query on it into `25P02`.
//! Dropping the patch (an sqlx bump that forgets to rebase it, or a deleted `Cargo.toml`
//! line) still compiles, so this is what notices.

use sqlx::{Connection, PgConnection, Pool, Postgres};
use std::time::Duration;

#[sqlx::test(migrations = "../migrations")]
async fn begin_cancelled_mid_round_trip_leaves_no_open_transaction(db: Pool<Postgres>) {
    // One connection, so the session inspected below is the one the cancelled begin used.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .connect_with((*db.connect_options()).clone())
        .await
        .expect("failed to build pool");
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&pool)
        .await
        .unwrap();

    // A plain `BEGIN` answers in well under a millisecond, which is too narrow to cancel
    // reliably; appending a sleep widens the round trip the way a degraded database does,
    // and it runs through the same `PgTransactionManager::begin` the patch fixes.
    let cancelled = tokio::time::timeout(
        Duration::from_millis(300),
        pool.begin_with("BEGIN; SELECT pg_sleep(2);"),
    )
    .await;
    assert!(cancelled.is_err(), "the begin must not have completed");

    // Outlast the sleep: sqlx only flushes the queued ROLLBACK once the abandoned statement
    // has answered and the connection is on its way back to the pool.
    tokio::time::sleep(Duration::from_millis(3500)).await;

    let mut admin = PgConnection::connect_with(&(*db.connect_options()).clone())
        .await
        .expect("failed to open an observing connection");
    let state: Option<String> =
        sqlx::query_scalar("SELECT state FROM pg_stat_activity WHERE pid = $1")
            .bind(pid)
            .fetch_optional(&mut admin)
            .await
            .unwrap()
            .flatten();
    assert_eq!(
        state.as_deref(),
        Some("idle"),
        "connection returned to the pool still inside a transaction — is the sqlx patch in \
         backend/Cargo.toml still applied?"
    );

    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("pool must still serve queries");
}
