use std::collections::{BTreeMap, HashSet};

use sqlx::{Pool, Postgres};

/// Settings that must never be deleted by the operator.
/// These are internal/system settings that could cause data loss if removed.
const PROTECTED_SETTINGS: &[&str] = &[
    "ducklake_user_pg_pwd",
    "ducklake_settings",
    "custom_instance_pg_databases",
];

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
    let current_rows: Vec<(String,)> = sqlx::query_as("SELECT name FROM global_settings")
        .fetch_all(db)
        .await?;
    let current_keys: HashSet<String> = current_rows.into_iter().map(|(name,)| name).collect();

    // Upsert desired settings
    for (key, value) in desired {
        sqlx::query(
            "INSERT INTO global_settings (name, value) VALUES ($1, $2) \
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
        )
        .bind(key)
        .bind(value)
        .execute(db)
        .await?;
        tracing::info!("Synced global setting: {key}");
    }

    // Delete settings not in desired (except protected keys)
    let desired_keys: HashSet<&str> = desired.keys().map(|s| s.as_str()).collect();
    for current_key in &current_keys {
        if !desired_keys.contains(current_key.as_str())
            && !PROTECTED_SETTINGS.contains(&current_key.as_str())
        {
            sqlx::query("DELETE FROM global_settings WHERE name = $1")
                .bind(current_key)
                .execute(db)
                .await?;
            tracing::info!("Deleted global setting not in CRD: {current_key}");
        }
    }

    Ok(())
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
    // Fetch current worker configs from DB
    let current_rows: Vec<(String,)> =
        sqlx::query_as("SELECT name FROM config WHERE name LIKE 'worker__%'")
            .fetch_all(db)
            .await?;
    let current_keys: HashSet<String> = current_rows.into_iter().map(|(name,)| name).collect();

    // Upsert desired configs (with worker__ prefix)
    for (group_name, config_value) in desired {
        let db_key = format!("worker__{group_name}");
        sqlx::query(
            "INSERT INTO config (name, config) VALUES ($1, $2) \
             ON CONFLICT (name) DO UPDATE SET config = EXCLUDED.config",
        )
        .bind(&db_key)
        .bind(config_value)
        .execute(db)
        .await?;
        tracing::info!("Synced worker config: {db_key}");
    }

    // Delete worker configs not in desired
    let desired_db_keys: HashSet<String> = desired.keys().map(|k| format!("worker__{k}")).collect();
    for current_key in &current_keys {
        if !desired_db_keys.contains(current_key) {
            sqlx::query("DELETE FROM config WHERE name = $1")
                .bind(current_key)
                .execute(db)
                .await?;
            tracing::info!("Deleted worker config not in CRD: {current_key}");
        }
    }

    Ok(())
}
