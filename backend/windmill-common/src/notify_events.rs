/*
 * Author: Windmill Labs
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Polling-based event notification system.
//!
//! This module provides a table-based alternative to PostgreSQL LISTEN/NOTIFY
//! for propagating cache invalidation and setting change events across
//! workers and servers.

use sqlx::{FromRow, Pool, Postgres};

use crate::error::Error;

#[derive(Debug, Clone, FromRow)]
pub struct NotifyEvent {
    pub id: i64,
    pub channel: String,
    pub payload: String,
}

/// Fetch all events with id greater than `last_event_id`.
/// Returns events ordered by id ascending.
pub async fn poll_notify_events(
    db: &Pool<Postgres>,
    last_event_id: i64,
) -> Result<Vec<NotifyEvent>, Error> {
    let events = sqlx::query_as::<_, NotifyEvent>(
        "SELECT id, channel, payload FROM notify_event WHERE id > $1 ORDER BY id LIMIT 1000",
    )
    .bind(last_event_id)
    .fetch_all(db)
    .await?;

    Ok(events)
}

/// Get the current maximum event id.
/// Used to initialize last_event_id on startup to avoid processing old events.
pub async fn get_latest_event_id(db: &Pool<Postgres>) -> Result<i64, Error> {
    let result: (i64,) = sqlx::query_as("SELECT COALESCE(MAX(id), 0) FROM notify_event")
        .fetch_one(db)
        .await?;

    Ok(result.0)
}

/// Delete events older than the specified number of minutes.
/// Returns the number of deleted rows.
pub async fn cleanup_old_events(
    db: &Pool<Postgres>,
    older_than_minutes: i32,
) -> Result<u64, Error> {
    let result = sqlx::query(
        "DELETE FROM notify_event WHERE created_at < now() - make_interval(mins => $1)",
    )
    .bind(older_than_minutes)
    .execute(db)
    .await?;

    Ok(result.rows_affected())
}
