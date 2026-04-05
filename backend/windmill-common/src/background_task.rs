//! Persistent, leased state for long-running background tasks coordinated
//! across multiple server replicas.
//!
//! Prior to this module, task state like "is log cleanup running?" lived in a
//! per-process `lazy_static` RwLock, which silently broke in multi-server
//! deployments: the server that claimed the lock was rarely the one the
//! frontend's GET hit next. This module stores state in the dedicated
//! `background_task_state` table and exposes a lease-based API:
//!
//! * [`try_claim`] atomically claims the named task, failing if another owner
//!   currently holds it with a fresh heartbeat.
//! * [`update_state`] writes progress and refreshes the heartbeat (the caller
//!   must be the owner; stale owners are ignored).
//! * [`release`] marks the task finished.
//! * [`get`] returns the current state. If the owner's heartbeat is stale,
//!   the read returns `running = false` so other servers can reclaim.
//!
//! The stale-heartbeat threshold is [`STALE_HEARTBEAT_SECS`]. Tasks should
//! call [`update_state`] at least once every threshold/2 seconds.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Pool, Postgres};

use crate::error::{self, Result};

/// After this many seconds without a heartbeat, a "running" lease is treated
/// as stale and can be reclaimed by another server.
pub const STALE_HEARTBEAT_SECS: i64 = 120;

#[derive(Debug, Clone, Serialize)]
pub struct BackgroundTaskRow {
    pub name: String,
    pub value: serde_json::Value,
    pub running: bool,
    pub owner: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

/// Try to atomically claim the named task for this server.
///
/// Succeeds if:
///  * no row exists yet, or
///  * the existing row is not running, or
///  * the existing row's heartbeat (`updated_at`) is older than
///    [`STALE_HEARTBEAT_SECS`].
///
/// Returns `Ok(true)` if the caller now holds the lease, `Ok(false)` if
/// someone else is actively running it.
pub async fn try_claim<T: Serialize>(
    db: &Pool<Postgres>,
    name: &str,
    owner: &str,
    initial_value: &T,
) -> Result<bool> {
    let value = serde_json::to_value(initial_value).map_err(|e| {
        error::Error::internal_err(format!("serialize background_task initial value: {e}"))
    })?;
    let stale_cutoff = format!("{STALE_HEARTBEAT_SECS} seconds");
    let claimed = sqlx::query_scalar!(
        "INSERT INTO background_task_state
             (name, value, running, owner, started_at, finished_at, updated_at)
         VALUES ($1, $2, true, $3, NOW(), NULL, NOW())
         ON CONFLICT (name) DO UPDATE SET
             value = EXCLUDED.value,
             running = true,
             owner = EXCLUDED.owner,
             started_at = NOW(),
             finished_at = NULL,
             updated_at = NOW()
         WHERE background_task_state.running = false
            OR background_task_state.updated_at < NOW() - ($4::text)::interval
         RETURNING 1",
        name,
        value,
        owner,
        stale_cutoff,
    )
    .fetch_optional(db)
    .await?;
    Ok(claimed.is_some())
}

/// Update the task's stored state. Also refreshes the heartbeat.
///
/// No-ops if another owner has since claimed the lease (e.g. because our
/// heartbeat went stale and another server took over); callers should not
/// treat this as an error.
pub async fn update_state<T: Serialize>(
    db: &Pool<Postgres>,
    name: &str,
    owner: &str,
    value: &T,
) -> Result<()> {
    let value = serde_json::to_value(value)
        .map_err(|e| error::Error::internal_err(format!("serialize background_task value: {e}")))?;
    sqlx::query!(
        "UPDATE background_task_state
         SET value = $1, updated_at = NOW()
         WHERE name = $2 AND owner = $3 AND running = true",
        value,
        name,
        owner
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Mark the task as finished. Only the current owner can release.
pub async fn release<T: Serialize>(
    db: &Pool<Postgres>,
    name: &str,
    owner: &str,
    final_value: &T,
) -> Result<()> {
    let value = serde_json::to_value(final_value).map_err(|e| {
        error::Error::internal_err(format!("serialize background_task final value: {e}"))
    })?;
    sqlx::query!(
        "UPDATE background_task_state
         SET value = $1, running = false, finished_at = NOW(), updated_at = NOW()
         WHERE name = $2 AND owner = $3",
        value,
        name,
        owner
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Read the current state of the named task.
///
/// If the row claims to be running but its heartbeat is stale (exceeds
/// [`STALE_HEARTBEAT_SECS`]), `running` is reported as `false` so the caller
/// / UI does not believe a dead server is still working.
pub async fn get(db: &Pool<Postgres>, name: &str) -> Result<Option<BackgroundTaskRow>> {
    let row = sqlx::query_as!(
        BackgroundTaskRow,
        r#"SELECT name, value as "value!: serde_json::Value", running as "running!",
                  owner, started_at, finished_at, updated_at as "updated_at!"
           FROM background_task_state WHERE name = $1"#,
        name
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(|mut r| {
        if r.running && r.updated_at < Utc::now() - chrono::Duration::seconds(STALE_HEARTBEAT_SECS)
        {
            r.running = false;
        }
        r
    }))
}
