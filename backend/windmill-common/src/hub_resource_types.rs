/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Keeps the `admins` workspace's resource types, which every workspace falls back to, in
//! step with the hub. Servers run [`spawn_daily_sync`] once a superadmin turns it on,
//! superadmins run [`sync`] on demand, and `windmill cache-rt` bakes the public hub's listing
//! into the image for instances that cannot reach a hub.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    error::{Error, Result},
    global_settings::{
        load_value_from_global_settings, DISABLE_HUB_SETTING, SYNC_HUB_RESOURCE_TYPES_DAILY_SETTING,
    },
    utils::{get_license_id_or_uid, HTTP_CLIENT_PERMISSIVE, HUB_API_SECRET},
    worker::HUB_RT_CACHE_DIR,
    DB, DEFAULT_HUB_BASE_URL, HUB_BASE_URL,
};

const SYNC_INTERVAL: chrono::TimeDelta = chrono::TimeDelta::hours(24);
const RETRY_INTERVAL: chrono::TimeDelta = chrono::TimeDelta::hours(1);
const CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(600);
const SYNC_LOCK_ID: i64 = 737_483_925;
/// `background_task_state.name` of the sync's last attempt and last hub sync.
const SYNC_STATE_TASK: &str = "hub_resource_type_sync";

pub fn cache_path() -> String {
    format!("{}/resource_types.json", *HUB_RT_CACHE_DIR)
}

/// A hub resource type with its schema parsed, as synced and as `cache-rt` stores it.
///
/// `format_extension` and `display_name` are doubly optional: no key (a cache written before
/// the field) leaves the stored value alone, while an explicit null clears it. Plain serde
/// folds both into `None`, hence the wrapping deserializer.
#[derive(Deserialize, Serialize, Clone)]
pub struct HubResourceType {
    pub name: String,
    pub schema: Option<serde_json::Value>,
    pub description: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::more_serde::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub format_extension: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "crate::more_serde::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub display_name: Option<Option<String>>,
    /// Absent from hubs that do not publish filesets, which leaves the stored flag alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_fileset: Option<bool>,
}

/// An entry of the hub's `/resource_types/list`, whose schema is a JSON string.
#[derive(Deserialize)]
struct HubListing {
    name: String,
    schema: Option<String>,
    description: Option<String>,
    /// Absent from hubs predating the column, which do not publish file types either, so
    /// the hub is authoritative for it whether or not it sends the key.
    #[serde(default)]
    format_extension: Option<String>,
    #[serde(default, deserialize_with = "crate::more_serde::double_option")]
    display_name: Option<Option<String>>,
    #[serde(default)]
    is_fileset: Option<bool>,
}

/// One hub's listing. A type whose schema does not parse is left out.
pub async fn fetch(
    hub_base_url: &str,
    api_secret: Option<&str>,
    uid: Option<&str>,
) -> Result<Vec<HubResourceType>> {
    // Permissive, so `ACCEPT_INVALID_CERTS` lets the sync reach a private hub behind a
    // self-signed certificate.
    let mut request = HTTP_CLIENT_PERMISSIVE
        .get(format!("{hub_base_url}/resource_types/list"))
        .header("Accept", "application/json");
    if let Some(uid) = uid {
        request = request.header("X-uid", uid);
    }
    if let Some(secret) = api_secret {
        request = request.header("X-api-secret", secret);
    }
    let response = request.send().await.map_err(|e| {
        Error::InternalErr(format!("Failed to reach the hub at {hub_base_url}: {e}"))
    })?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(Error::InternalErr(format!(
            "The hub at {hub_base_url} refused the request: check the Private Hub API secret"
        )));
    }
    if !status.is_success() {
        return Err(Error::InternalErr(format!(
            "The hub at {hub_base_url} answered {status}"
        )));
    }
    let listing: Vec<HubListing> = response.json().await.map_err(|e| {
        Error::InternalErr(format!(
            "Failed to parse the resource types listed by {hub_base_url}: {e}"
        ))
    })?;

    Ok(listing
        .into_iter()
        .filter_map(|rt| {
            let schema = match rt.schema.as_deref().map(serde_json::from_str).transpose() {
                Ok(schema) => schema,
                Err(e) => {
                    tracing::warn!(
                        "Skipping hub resource type {}: invalid schema: {e}",
                        rt.name
                    );
                    return None;
                }
            };
            Some(HubResourceType {
                name: rt.name,
                schema,
                description: rt.description,
                format_extension: Some(rt.format_extension),
                display_name: rt.display_name,
                is_fileset: rt.is_fileset,
            })
        })
        .collect())
}

/// The listing of the hub this instance is configured with.
async fn fetch_configured_hub(db: &DB) -> Result<Vec<HubResourceType>> {
    let hub = (**HUB_BASE_URL.load()).clone();
    let uid = get_license_id_or_uid(db).await.ok();
    if hub == DEFAULT_HUB_BASE_URL {
        return fetch(&hub, None, uid.as_deref()).await;
    }
    let secret = (**HUB_API_SECRET.load()).clone();
    let types = fetch(&hub, secret.as_deref(), uid.as_deref()).await?;
    if !types.is_empty() {
        return Ok(types);
    }
    // A private hub that publishes no resource types of its own defers to the public one,
    // which must not be sent the private hub's secret.
    tracing::info!("The hub at {hub} lists no resource types, syncing the public hub's");
    fetch(DEFAULT_HUB_BASE_URL, None, uid.as_deref()).await
}

#[derive(PartialEq, Debug)]
struct ResourceTypeRow {
    schema: Option<serde_json::Value>,
    description: Option<String>,
    format_extension: Option<String>,
    is_fileset: bool,
    display_name: Option<String>,
}

/// The row a synced type should end up as, given what `admins` already stores for it.
fn target_row(rt: &HubResourceType, stored: Option<&ResourceTypeRow>) -> ResourceTypeRow {
    let is_fileset = rt
        .is_fileset
        .unwrap_or_else(|| stored.is_some_and(|s| s.is_fileset));
    // A fileset is a set of files, so it cannot also be one file. Create and update reject
    // the pair; this writer bypasses both.
    let format_extension = if is_fileset {
        None
    } else {
        match &rt.format_extension {
            Some(extension) => extension.clone(),
            None => stored.and_then(|s| s.format_extension.clone()),
        }
    };
    // A name too long for the column counts as absent: one bad entry must not fail the
    // upsert and end the rest of the sync.
    let display_name = match &rt.display_name {
        Some(Some(name)) if name.chars().count() > 100 => {
            tracing::warn!(
                "Ignoring the display_name of resource type {}: longer than 100 characters",
                rt.name
            );
            stored.and_then(|s| s.display_name.clone())
        }
        Some(name) => name.clone(),
        None => stored.and_then(|s| s.display_name.clone()),
    };
    ResourceTypeRow {
        schema: rt.schema.clone(),
        description: rt.description.clone(),
        format_extension,
        is_fileset,
        display_name,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SyncSource {
    Hub,
    ImageCache,
}

pub struct SyncOutcome {
    pub source: SyncSource,
    /// Every type the source listed, changed or not.
    pub listed: Vec<String>,
    /// The types this sync created or updated.
    pub changed: Vec<String>,
}

impl SyncOutcome {
    pub fn summary(&self) -> String {
        let synced = self.changed.len();
        let unchanged = self.listed.len() - synced;
        match self.source {
            SyncSource::Hub => {
                format!("Synced {synced} resource types from the hub ({unchanged} unchanged)")
            }
            SyncSource::ImageCache => format!(
                "Synced {synced} resource types from the image's cache ({unchanged} unchanged)"
            ),
        }
    }
}

async fn apply(db: &DB, source: SyncSource, types: &[HubResourceType]) -> Result<SyncOutcome> {
    let stored: HashMap<String, ResourceTypeRow> = sqlx::query_as::<
        _,
        (
            String,
            Option<serde_json::Value>,
            Option<String>,
            Option<String>,
            bool,
            Option<String>,
        ),
    >(
        "SELECT name, schema, description, format_extension, is_fileset, display_name
         FROM resource_type WHERE workspace_id = 'admins'",
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(
        |(name, schema, description, format_extension, is_fileset, display_name)| {
            (
                name,
                ResourceTypeRow { schema, description, format_extension, is_fileset, display_name },
            )
        },
    )
    .collect();

    let mut changed = vec![];
    for rt in types {
        let current = stored.get(&rt.name);
        let target = target_row(rt, current);
        if current == Some(&target) {
            continue;
        }
        sqlx::query(
            "INSERT INTO resource_type
                (workspace_id, name, schema, description, format_extension, is_fileset, display_name, edited_at)
             VALUES ('admins', $1, $2, $3, $4, $5, $6, now())
             ON CONFLICT (workspace_id, name) DO UPDATE
             SET schema = EXCLUDED.schema, description = EXCLUDED.description,
                 format_extension = EXCLUDED.format_extension, is_fileset = EXCLUDED.is_fileset,
                 display_name = EXCLUDED.display_name, edited_at = now()",
        )
        .bind(&rt.name)
        .bind(&target.schema)
        .bind(&target.description)
        .bind(&target.format_extension)
        .bind(target.is_fileset)
        .bind(&target.display_name)
        .execute(db)
        .await?;
        changed.push(rt.name.clone());
    }

    Ok(SyncOutcome { source, listed: types.iter().map(|rt| rt.name.clone()).collect(), changed })
}

#[derive(Deserialize, Default)]
struct SyncState {
    last_attempt_at: Option<DateTime<Utc>>,
    last_hub_sync_at: Option<DateTime<Utc>>,
}

async fn load_state(db: &DB) -> Result<SyncState> {
    let value: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT value FROM background_task_state WHERE name = $1")
            .bind(SYNC_STATE_TASK)
            .fetch_optional(db)
            .await?;
    Ok(value
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default())
}

/// Merged into the stored state, so a writer only ever sets the keys it names.
async fn record_state(db: &DB, patch: serde_json::Value) -> Result<()> {
    sqlx::query(
        "INSERT INTO background_task_state (name, value) VALUES ($1, $2)
         ON CONFLICT (name) DO UPDATE
         SET value = background_task_state.value || EXCLUDED.value, updated_at = now()",
    )
    .bind(SYNC_STATE_TASK)
    .bind(patch)
    .execute(db)
    .await?;
    Ok(())
}

async fn setting_enabled(db: &DB, name: &str) -> Result<bool> {
    Ok(load_value_from_global_settings(db, name)
        .await?
        .and_then(|v| v.as_bool())
        .unwrap_or(false))
}

/// Syncs from the configured hub and records the attempt, successful or not.
async fn sync_from_hub(db: &DB) -> Result<SyncOutcome> {
    let attempted_at = Utc::now();
    let result = match fetch_configured_hub(db).await {
        Ok(types) => apply(db, SyncSource::Hub, &types).await,
        Err(e) => Err(e),
    };
    let state = match &result {
        Ok(_) => {
            json!({ "last_attempt_at": attempted_at, "last_hub_sync_at": attempted_at, "last_error": null })
        }
        Err(e) => json!({ "last_attempt_at": attempted_at, "last_error": e.to_string() }),
    };
    record_state(db, state).await?;
    result
}

/// Syncs from the configured hub, falling back to the image's cache when the hub is disabled
/// or unreachable and the instance has never synced from a hub.
pub async fn sync(db: &DB) -> Result<SyncOutcome> {
    let hub_err = if setting_enabled(db, DISABLE_HUB_SETTING).await? {
        Error::BadRequest("The hub is disabled on this instance".to_string())
    } else {
        match sync_from_hub(db).await {
            Ok(outcome) => return Ok(outcome),
            Err(e) => e,
        }
    };
    match fall_back_to_image_cache(db, &cache_path()).await? {
        Some(outcome) => {
            tracing::warn!("Hub resource type sync failed, used the image's cache: {hub_err}");
            Ok(outcome)
        }
        None => Err(hub_err),
    }
}

/// The cache is the listing as of the image's build, so standing in for the hub on an
/// instance that has synced from one would revert every type the hub has updated since.
async fn fall_back_to_image_cache(db: &DB, path: &str) -> Result<Option<SyncOutcome>> {
    if let Some(at) = load_state(db).await?.last_hub_sync_at {
        tracing::info!(
            "Not applying the image's cached resource types: synced from the hub at {at}"
        );
        return Ok(None);
    }
    sync_from_cache_file(db, path).await
}

/// Applies the listing baked into the image (`SYNC_CACHED_RT`), on every start and whether or
/// not the instance has synced from a hub: it is how an instance that cannot reach the public
/// hub, a private hub listing only its own types included, takes each new image's public types.
/// `None` when the image carries no cache.
pub async fn sync_from_image_cache(db: &DB) -> Result<Option<SyncOutcome>> {
    sync_from_cache_file(db, &cache_path()).await
}

async fn sync_from_cache_file(db: &DB, path: &str) -> Result<Option<SyncOutcome>> {
    let content = match tokio::fs::read_to_string(path).await {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::info!("No cached resource types at {path}");
            return Ok(None);
        }
        Err(e) => {
            return Err(Error::InternalErr(format!(
                "Failed to read cached resource types at {path}: {e}"
            )))
        }
    };
    let types: Vec<HubResourceType> = serde_json::from_str(&content).map_err(|e| {
        Error::InternalErr(format!(
            "Failed to parse cached resource types at {path}: {e}"
        ))
    })?;
    Ok(Some(apply(db, SyncSource::ImageCache, &types).await?))
}

/// While `sync_hub_resource_types_daily` is on, syncs once a day, from whichever server first
/// finds the sync due. Off by default: each sync overwrites local edits to the `admins` types
/// the hub also defines. The state it reads lives in the database, so servers starting or
/// restarting do not each sync again.
pub fn spawn_daily_sync(db: DB, mut killpill_rx: tokio::sync::broadcast::Receiver<()>) {
    tokio::spawn(async move {
        // Leaves time for the hub URL and secret settings to load, and spreads out servers
        // that start together.
        let mut wait = std::time::Duration::from_secs(rand::random_range(30..90));
        loop {
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = killpill_rx.recv() => return,
            }
            if let Err(e) = sync_if_due(&db).await {
                tracing::error!("Hub resource type sync: {e:#}");
            }
            wait = CHECK_INTERVAL;
        }
    });
}

async fn sync_if_due(db: &DB) -> Result<()> {
    if !setting_enabled(db, SYNC_HUB_RESOURCE_TYPES_DAILY_SETTING).await?
        || setting_enabled(db, DISABLE_HUB_SETTING).await?
    {
        return Ok(());
    }
    // A transaction-scoped lock: a dropped transaction rolls back and releases it, so a
    // pass cut short cannot leave it held on a pooled connection. The transaction only
    // owns the lock; the sync runs on other connections.
    let mut lock_tx = db.begin().await?;
    let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
        .bind(SYNC_LOCK_ID)
        .fetch_one(&mut *lock_tx)
        .await?;
    if !locked {
        return Ok(());
    }
    let state = load_state(db).await?;
    let now = Utc::now();
    let due = state
        .last_hub_sync_at
        .is_none_or(|at| now - at >= SYNC_INTERVAL)
        && state
            .last_attempt_at
            .is_none_or(|at| now - at >= RETRY_INTERVAL);
    // Hub only. Falling back to the image's cache here would re-apply the same listing every
    // hour on an instance that never reaches a hub, over any edit made since; the cache is
    // applied at boot (`SYNC_CACHED_RT`) or on demand instead.
    if due {
        match sync_from_hub(db).await {
            Ok(outcome) if outcome.changed.is_empty() => tracing::info!("{}", outcome.summary()),
            Ok(outcome) => {
                tracing::info!("{}: {}", outcome.summary(), outcome.changed.join(", "))
            }
            Err(e) => tracing::warn!("Hub resource type sync failed, retrying in an hour: {e}"),
        }
    }
    lock_tx.rollback().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hub_type(
        format_extension: Option<Option<&str>>,
        display_name: Option<Option<&str>>,
        is_fileset: Option<bool>,
    ) -> HubResourceType {
        HubResourceType {
            name: "t".to_string(),
            schema: None,
            description: None,
            format_extension: format_extension.map(|e| e.map(str::to_string)),
            display_name: display_name.map(|n| n.map(str::to_string)),
            is_fileset,
        }
    }

    fn stored(
        format_extension: Option<&str>,
        display_name: Option<&str>,
        is_fileset: bool,
    ) -> ResourceTypeRow {
        ResourceTypeRow {
            schema: None,
            description: None,
            format_extension: format_extension.map(str::to_string),
            is_fileset,
            display_name: display_name.map(str::to_string),
        }
    }

    #[test]
    fn absent_keys_keep_the_stored_values_and_nulls_clear_them() {
        let current = stored(Some("csv"), Some("T"), false);
        let kept = target_row(&hub_type(None, None, None), Some(&current));
        assert_eq!(kept, current);
        let cleared = target_row(&hub_type(Some(None), Some(None), None), Some(&current));
        assert_eq!(cleared, stored(None, None, false));
    }

    #[test]
    fn a_fileset_never_takes_a_format_extension() {
        let local_fileset = stored(None, None, true);
        let row = target_row(
            &hub_type(Some(Some("csv")), None, None),
            Some(&local_fileset),
        );
        assert_eq!(row, stored(None, None, true));
        let row = target_row(&hub_type(Some(Some("csv")), None, Some(true)), None);
        assert_eq!(row, stored(None, None, true));
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn only_sync_cached_rt_applies_the_image_cache_after_a_hub_sync(
        db: DB,
    ) -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("resource_types.json");
        std::fs::write(
            &path,
            json!([{ "name": "cached_type", "schema": null, "description": "image copy" }])
                .to_string(),
        )?;
        let path = path.to_str().unwrap();
        let description = || {
            sqlx::query_scalar::<_, Option<String>>(
                "SELECT description FROM resource_type WHERE workspace_id = 'admins' AND name = 'cached_type'",
            )
            .fetch_one(&db)
        };

        assert!(fall_back_to_image_cache(&db, path).await?.is_some());
        assert_eq!(description().await?.as_deref(), Some("image copy"));

        record_state(&db, json!({ "last_hub_sync_at": Utc::now() })).await?;
        sqlx::query(
            "UPDATE resource_type SET description = 'hub copy' WHERE workspace_id = 'admins' AND name = 'cached_type'",
        )
        .execute(&db)
        .await?;
        assert!(fall_back_to_image_cache(&db, path).await?.is_none());
        assert_eq!(description().await?.as_deref(), Some("hub copy"));

        assert!(sync_from_cache_file(&db, path).await?.is_some());
        assert_eq!(description().await?.as_deref(), Some("image copy"));
        Ok(())
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn no_daily_sync_unless_turned_on(db: DB) -> anyhow::Result<()> {
        sync_if_due(&db).await?;

        let state: Option<serde_json::Value> =
            sqlx::query_scalar("SELECT value FROM background_task_state WHERE name = $1")
                .bind(SYNC_STATE_TASK)
                .fetch_optional(&db)
                .await?;
        assert_eq!(state, None);
        Ok(())
    }
}
