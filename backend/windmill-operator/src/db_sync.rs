use std::collections::BTreeMap;

use sqlx::{Pool, Postgres};
use windmill_common::instance_config::{
    apply_configs_diff, apply_settings_diff, diff_global_settings, diff_worker_configs, ApplyMode,
};

/// Perform a full declarative sync of global settings.
///
/// - Upserts every key in `desired` into the `global_settings` table.
/// - Deletes keys that exist in DB but are absent from `desired`
///   (except protected keys).
pub async fn sync_global_settings(
    db: &Pool<Postgres>,
    desired: &BTreeMap<String, serde_json::Value>,
) -> anyhow::Result<()> {
    // Fetch current settings from DB
    let current_rows: Vec<(String, serde_json::Value)> =
        sqlx::query_as("SELECT name, value FROM global_settings")
            .fetch_all(db)
            .await?;
    let current: BTreeMap<String, serde_json::Value> = current_rows.into_iter().collect();

    let diff = diff_global_settings(&current, desired, ApplyMode::Replace);
    apply_settings_diff(db, &diff).await
}

/// Perform a full declarative sync of worker configs.
///
/// - Upserts every key in `desired` into the `config` table with the
///   `worker__` prefix.
/// - Deletes `worker__*` rows that exist in DB but are absent from `desired`.
pub async fn sync_worker_configs(
    db: &Pool<Postgres>,
    desired: &BTreeMap<String, serde_json::Value>,
) -> anyhow::Result<()> {
    // Fetch current worker configs from DB (strip prefix for comparison)
    let current_rows: Vec<(String, serde_json::Value)> =
        sqlx::query_as("SELECT name, config FROM config WHERE name LIKE 'worker__%'")
            .fetch_all(db)
            .await?;
    let current: BTreeMap<String, serde_json::Value> = current_rows
        .into_iter()
        .map(|(name, config)| {
            let group = name.strip_prefix("worker__").unwrap_or(&name).to_string();
            (group, config)
        })
        .collect();

    let diff = diff_worker_configs(&current, desired, ApplyMode::Replace);
    apply_configs_diff(db, &diff).await
}
