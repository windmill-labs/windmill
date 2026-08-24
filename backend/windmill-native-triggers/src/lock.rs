//! Serializing the operations that mutate a trigger's external registration.

use std::sync::Arc;

use sqlx::{Connection, PgConnection};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use windmill_common::{
    error::{Error, Result},
    DB,
};

use crate::ServiceName;

/// How long to wait for whoever holds the lock before giving up.
///
/// A waiter parks on its own connection, and that connection is outside the pool and so outside
/// its limits — enough of them queued on one trigger would eat into what the server has for
/// everything else. Waiting is bounded instead: the holder is doing a network call with its own
/// timeout, so anything longer than this is not worth a connection, and the caller gets a plain
/// "busy, try again" rather than an open-ended stall.
const LOCK_WAIT: &str = "45s";

/// Ceiling on how many of these connections can exist at once.
///
/// Each is opened outside the pool, so nothing else caps them: a burst of renames or repeated
/// requests against one trigger would otherwise eat into what the server has for every other
/// workload. Waiting for a permit costs nothing — the operation was going to queue on the lock
/// anyway — and it is generous enough that ordinary use never reaches it.
const MAX_CONCURRENT_LOCKS: usize = 32;

lazy_static::lazy_static! {
    static ref LOCK_SLOTS: Arc<Semaphore> = Arc::new(Semaphore::new(MAX_CONCURRENT_LOCKS));
}

/// Held for the whole of a read → register → record cycle on one trigger.
///
/// Registering a webhook is a read-modify-write spanning a network round-trip, against state held
/// both here and on the external service. Two of them interleaving desynchronises the two: whoever
/// writes the row last wins in the database, whoever calls the service last wins there, and they
/// need not be the same operation — which is how a trigger ends up pointing somewhere the service
/// is not calling, or a deleted trigger keeps firing.
///
/// A Postgres advisory lock makes them take turns. It is deliberately not a row lock: a rename's
/// own `UPDATE native_trigger` must not block behind a re-registration's network call, and only
/// code that takes the same key here waits.
///
/// The lock lives on its own connection, opened outside the pool. Holders keep it for as long as
/// the external call takes and go on to need pool connections of their own to finish and release
/// it, so taking it from the pool would let a handful of concurrent renames hold every connection
/// while each waits for one more. Being off-pool also makes the lock impossible to strand: it is
/// session-scoped, and a dropped or crashed connection releases it, whereas a pooled connection
/// would carry it back into the pool.
pub(crate) struct TriggerLock {
    conn: Option<PgConnection>,
    _slot: OwnedSemaphorePermit,
}

impl TriggerLock {
    pub(crate) async fn acquire(
        db: &DB,
        w_id: &str,
        service_name: ServiceName,
        external_id: &str,
    ) -> Result<Self> {
        let (mut conn, slot) = Self::open(db).await?;
        sqlx::query_scalar!(
            "SELECT pg_advisory_lock(hashtextextended($1, 0))",
            Self::key(w_id, service_name, external_id)
        )
        .fetch_one(&mut conn)
        .await
        .map_err(|e| {
            Error::BadRequest(format!(
                "Another operation on {external_id} is still running after {LOCK_WAIT}; try again \
                 shortly ({e})"
            ))
        })?;
        Ok(Self { conn: Some(conn), _slot: slot })
    }

    /// Take the lock only if it is free, for callers that would rather come back later than wait
    /// out someone else's network call — the background renewal sweep, which runs again shortly.
    pub(crate) async fn try_acquire(
        db: &DB,
        w_id: &str,
        service_name: ServiceName,
        external_id: &str,
    ) -> Result<Option<Self>> {
        let (mut conn, slot) = Self::open(db).await?;
        let acquired = sqlx::query_scalar!(
            r#"SELECT pg_try_advisory_lock(hashtextextended($1, 0)) AS "acquired!""#,
            Self::key(w_id, service_name, external_id)
        )
        .fetch_one(&mut conn)
        .await?;
        Ok(acquired.then_some(Self { conn: Some(conn), _slot: slot }))
    }

    /// The lock's own connection, lent to the work it protects.
    ///
    /// Callers need a connection to hand the external service adapter, and it is held for the
    /// whole network call. Taking that from the pool is what starves it: a handful of concurrent
    /// renames would each pin one for as long as the remote takes to answer. This one is already
    /// dedicated and idle for exactly that window.
    pub(crate) fn conn(&mut self) -> &mut PgConnection {
        self.conn
            .as_mut()
            .expect("lock connection is only taken on release")
    }

    pub(crate) async fn release(mut self) -> Result<()> {
        if let Some(conn) = self.conn.take() {
            // Closing the connection is what ends the session and its locks; unlocking first only
            // makes that explicit at the call site.
            conn.close().await?;
        }
        Ok(())
    }

    async fn open(db: &DB) -> Result<(PgConnection, OwnedSemaphorePermit)> {
        let slot = LOCK_SLOTS
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| Error::internal_err(format!("trigger lock slots closed: {e}")))?;
        let mut conn = PgConnection::connect_with(&db.connect_options()).await?;
        sqlx::query(&format!("SET lock_timeout = '{LOCK_WAIT}'"))
            .execute(&mut conn)
            .await?;
        Ok((conn, slot))
    }

    fn key(w_id: &str, service_name: ServiceName, external_id: &str) -> String {
        format!("native_trigger:{w_id}:{service_name}:{external_id}")
    }
}
