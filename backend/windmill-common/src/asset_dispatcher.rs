//! Reactive dispatcher for asset-change triggers (stage 4).
//!
//! Polls `asset_event` for new rows, matches them against
//! `asset_trigger.subscription_set`, and enqueues downstream runs.
//!
//! This module intentionally exposes **stateless** building blocks (match +
//! policy application + reserved-arg construction); the runtime wiring that
//! turns them into a long-running task with server-ownership and job push
//! lives in `windmill-worker` so that `windmill-common` can stay free of a
//! `windmill-queue` dependency.

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};

use crate::{assets::AssetKind, error::Result};

/// One pending event picked up from `asset_event`. The dispatcher hands
/// matched rows plus the triggering event down to the enqueue path.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PendingAssetEvent {
    pub id: i64,
    pub workspace_id: String,
    pub asset_kind: AssetKind,
    pub asset_path: String,
    pub partition_key: Option<String>,
    pub job_id: uuid::Uuid,
    pub at: chrono::DateTime<chrono::Utc>,
}

/// Row shape returned by the matching query — just what the enqueue path
/// needs, not the full trigger row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MatchedAssetTrigger {
    pub workspace_id: String,
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    pub fires: String,
    pub debounce_s: i32,
    pub cancel_on_new: bool,
    pub owner_script_hash: Option<i64>,
}

/// Fetch events with id greater than `last_id`, ordered by id ascending.
/// Bounded to a single batch so the dispatcher makes forward progress even
/// under burst.
pub async fn fetch_pending_events(
    db: &Pool<Postgres>,
    last_id: i64,
    batch_size: i32,
) -> Result<Vec<PendingAssetEvent>> {
    let events = sqlx::query_as::<_, PendingAssetEvent>(
        "SELECT id, workspace_id, asset_kind, asset_path, partition_key, job_id, at \
         FROM asset_event \
         WHERE id > $1 \
         ORDER BY id ASC \
         LIMIT $2",
    )
    .bind(last_id)
    .bind(batch_size)
    .fetch_all(db)
    .await?;
    Ok(events)
}

/// Select `asset_trigger` rows whose subscription set contains the
/// `(kind, path)` of the given event. Uses JSONB containment so the GIN
/// index on `subscription_set` kicks in.
pub async fn match_triggers_for_event(
    db: &Pool<Postgres>,
    event: &PendingAssetEvent,
) -> Result<Vec<MatchedAssetTrigger>> {
    let subscription_probe = serde_json::json!({
        "paths": [{ "kind": event.asset_kind, "path": event.asset_path }]
    });
    let rows = sqlx::query_as::<_, MatchedAssetTrigger>(
        "SELECT workspace_id, path, script_path, is_flow, fires, debounce_s, \
                cancel_on_new, owner_script_hash \
         FROM asset_trigger \
         WHERE workspace_id = $1 \
           AND mode = 'enabled' \
           AND subscription_set @> $2::jsonb",
    )
    .bind(&event.workspace_id)
    .bind(&subscription_probe)
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// Reserved argument payload passed to asset-triggered scripts. The worker
/// fills the corresponding `wm_asset_event` / `wm_partition` / `wm_backfill`
/// parameters at job materialization (stage 5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEventArg {
    pub id: i64,
    pub workspace_id: String,
    pub asset_kind: AssetKind,
    pub asset_path: String,
    pub partition_key: Option<String>,
    pub job_id: uuid::Uuid,
    pub at: chrono::DateTime<chrono::Utc>,
}

impl From<&PendingAssetEvent> for AssetEventArg {
    fn from(e: &PendingAssetEvent) -> Self {
        Self {
            id: e.id,
            workspace_id: e.workspace_id.clone(),
            asset_kind: e.asset_kind,
            asset_path: e.asset_path.clone(),
            partition_key: e.partition_key.clone(),
            job_id: e.job_id,
            at: e.at,
        }
    }
}

/// For `fires = 'all'`, the trigger should only fire when *every* path in
/// the subscription set has seen a newer event than the last fire. This
/// helper checks that condition against `asset_event`. Returns `true` if
/// the trigger is ready to fire.
pub async fn fires_all_ready(
    db: &Pool<Postgres>,
    workspace_id: &str,
    subscription_paths: &[(AssetKind, String)],
    last_fired_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<bool> {
    let last = last_fired_at.unwrap_or_else(|| {
        chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap_or_default()
    });
    for (kind, path) in subscription_paths {
        let max_at = sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
            "SELECT MAX(at) FROM asset_event \
             WHERE workspace_id = $1 AND asset_kind = $2 AND asset_path = $3",
        )
        .bind(workspace_id)
        .bind(kind)
        .bind(path)
        .fetch_one(db)
        .await?;
        match max_at {
            Some(t) if t > last => continue,
            _ => return Ok(false),
        }
    }
    Ok(true)
}
