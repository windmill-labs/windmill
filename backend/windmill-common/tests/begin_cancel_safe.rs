//! Pins that a caller cancelled while `begin_cancel_safe` is in flight leaves no transaction
//! open on the session. sqlx's own `Pool::begin` does not hold this: its rollback-on-drop
//! guard keys on a transaction depth it only raises after the `BEGIN` round trip returns.

use futures::FutureExt;
use sqlx::{Connection, PgConnection, Pool, Postgres};
use std::time::Duration;
use windmill_common::db::BeginCancelSafe;

#[sqlx::test(migrations = "../migrations")]
async fn cancelled_begin_leaves_no_open_transaction(db: Pool<Postgres>) {
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

    // Polls exactly once — enough to start the work, never enough to finish it — and then
    // drops the future, so the cancellation lands while the transaction is being opened. A
    // short timeout would not do: tokio's timer granularity lets the begin win the race.
    let cancelled = pool.begin_cancel_safe().now_or_never();
    assert!(cancelled.is_none(), "the begin must not have completed");

    // The detached task still finishes and drops its transaction, which rolls it back.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut admin = PgConnection::connect_with(&(*db.connect_options()).clone())
        .await
        .expect("failed to open an observing connection");
    let (state, last_query): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT state, query FROM pg_stat_activity WHERE pid = $1")
            .bind(pid)
            .fetch_one(&mut admin)
            .await
            .unwrap();
    assert_eq!(
        state.as_deref(),
        Some("idle"),
        "session left in a transaction"
    );
    // Proves the task outlived its caller rather than being aborted with it: it opened the
    // transaction and rolled it back. Cancelling `Pool::begin` directly leaves the session
    // still showing whatever ran before it.
    assert_eq!(
        last_query.as_deref(),
        Some("ROLLBACK"),
        "the cancelled begin should still have opened and rolled back a transaction"
    );

    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("pool must still serve queries");
}
