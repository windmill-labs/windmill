//! A pooled connection whose session is left inside a transaction sqlx is not tracking stays
//! broken for every later borrower — `Rollback::drop` only fires while sqlx's own transaction
//! depth is non-zero, and the on-release `ping` is a bare `wait_until_ready` that reports such
//! a session as healthy. Before the reset hook this cost half an hour of unrelated `25P02`
//! failures across the whole process, until `max_lifetime` recycled the connection.

use sqlx::{Executor, Pool, Postgres};
use windmill_common::db::connection_reset;
use windmill_common::error::Error;

#[sqlx::test(migrations = "../migrations")]
async fn poisoned_connection_is_reset_before_being_handed_out_again(db: Pool<Postgres>) {
    // One connection, so every query below lands on the same session.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .after_release(|conn, _| Box::pin(connection_reset::reset_on_release(conn)))
        .connect_with((*db.connect_options()).clone())
        .await
        .expect("failed to build pool");

    // A batch that fails between BEGIN and COMMIT never reaches the COMMIT, leaving the
    // session `idle in transaction (aborted)` while sqlx still believes it is not in a
    // transaction — the same state a future cancelled during `begin()` leaves behind.
    pool.execute("BEGIN; SELECT 1/0; COMMIT;")
        .await
        .expect_err("the batch must fail");

    let err = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect_err("the next borrower inherits the aborted transaction");
    assert_eq!(
        err.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("25P02"),
    );
    // Converting the error is what arms the reset in production, where every `?` on a
    // sqlx result goes through this same `From`.
    let _ = Error::from(err);

    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("pool must hand out a usable connection again");
}
