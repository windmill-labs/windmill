//! Guards the `sqlx` entries in `[patch.crates-io]` — `backend/Cargo.toml` carries the why.
//! Dropping the patch still compiles, so a test is what notices.
//!
//! Ignored by default: it only has something to say when the sqlx dependency moves, and it
//! spends a couple of seconds waiting on a deliberately slow round trip. Run it whenever you
//! touch sqlx — a version bump, a change to the patch entries, a fork rebase:
//!
//! ```text
//! cargo test -p windmill-common --test sqlx_begin_cancel_safe -- --ignored
//! ```

use sqlx::{Connection, PgConnection, Pool, Postgres};
use std::time::{Duration, Instant};

#[sqlx::test]
#[ignore = "run with --ignored after any sqlx bump or change to [patch.crates-io]"]
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
    // reliably; appending a sleep widens the round trip and runs through the same
    // `PgTransactionManager::begin` the patch fixes.
    let cancelled = tokio::time::timeout(
        Duration::from_millis(300),
        pool.begin_with("BEGIN; SELECT pg_sleep(2);"),
    )
    .await;
    assert!(cancelled.is_err(), "the begin must not have completed");

    let mut admin = PgConnection::connect_with(&(*db.connect_options()).clone())
        .await
        .expect("failed to open an observing connection");

    // sqlx only flushes the queued ROLLBACK once the abandoned statement has answered, so
    // wait for the session to stop running rather than sleeping a fixed time a loaded runner
    // could overshoot.
    let deadline = Instant::now() + Duration::from_secs(30);
    let state = loop {
        let state: String = sqlx::query_scalar("SELECT state FROM pg_stat_activity WHERE pid = $1")
            .bind(pid)
            .fetch_optional(&mut admin)
            .await
            .unwrap()
            .flatten()
            .unwrap_or_default();
        if state != "active" || Instant::now() >= deadline {
            break state;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    assert!(
        !state.starts_with("idle in transaction"),
        "connection returned to the pool still inside a transaction (state {state:?}) — is \
         the sqlx patch in backend/Cargo.toml still applied?"
    );

    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("pool must still serve queries");
}
