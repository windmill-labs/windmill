use std::{
    collections::{BTreeMap, HashMap},
    sync::OnceLock,
};

use itertools::Itertools;
use serde_json::value::RawValue;
use sqlx::{types::Json, Pool, Postgres, QueryBuilder};
use tokio::sync::mpsc;
use windmill_parser::asset_parser::parse_asset_syntax;

use crate::{
    assets::{
        merge_asset_columns, merge_asset_usage_access_types, AssetKind, AssetUsageAccessType,
        AssetUsageKind,
    },
    error,
};

/// Represents a runtime-detected asset from job arguments
#[derive(Debug, Clone)]
pub struct RuntimeAsset {
    pub path: String,
    pub kind: AssetKind,
}

/// Extract assets from job arguments by analyzing the JSON values
/// We only detect assets that are commonly used outside of our APIs
/// (Only resources at the time of writing)
pub fn extract_runtime_assets_from_args(
    args: &HashMap<String, Box<RawValue>>,
) -> Vec<RuntimeAsset> {
    let mut assets = Vec::new();
    for (_key, value) in args.iter() {
        extract_assets_from_raw_value(value, &mut assets).unwrap_or(());
    }
    assets
}

fn extract_assets_from_raw_value(
    value: &Box<RawValue>,
    assets: &mut Vec<RuntimeAsset>,
) -> Option<()> {
    let json = value.get().trim_start();
    if json.starts_with('"') && json.len() < 256 && json.len() > 5 {
        // Ensure the string starts with an asset scheme before parsing
        // We only include the schemes that are NOT used in our APIs here, because
        // we track those assets on usage.
        let prefix = ["$res:", "res://"]
            .iter()
            .any(|prefix| json[1..].starts_with(prefix));
        if prefix {
            let s = serde_json::from_str::<String>(value.get()).ok()?;
            let (kind, path) = parse_asset_syntax(&s, false)?;
            assets.push(RuntimeAsset {
                path: path.to_string(),
                kind: crate::assets::asset_kind_from_parser(kind),
            });
        }
    }
    None
}
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct InsertRuntimeAssetParams {
    pub workspace_id: String,
    pub asset_path: String,
    pub asset_kind: AssetKind,
    pub job_id: uuid::Uuid,
    pub access_type: Option<AssetUsageAccessType>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub columns: Option<BTreeMap<String, AssetUsageAccessType>>,
}

async fn insert_runtime_assets(
    executor: &Pool<Postgres>,
    assets: &[InsertRuntimeAssetParams],
) -> error::Result<()> {
    for chunk in assets.chunks(1000) {
        let mut query_builder = QueryBuilder::new("INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, columns, usage_kind, created_at) ");
        query_builder.push_values(chunk, |mut b, asset| {
            b.push_bind(&asset.workspace_id)
                .push_bind(&asset.asset_path)
                .push_bind(&asset.asset_kind)
                .push_bind(&asset.access_type)
                .push_bind(asset.job_id.to_string())
                .push_bind(Json(&asset.columns))
                .push_bind(&AssetUsageKind::Job)
                .push_bind(&asset.created_at);
        });
        query_builder.push(" ON CONFLICT DO NOTHING");
        query_builder.build().execute(executor).await?;
    }
    Ok(())
}

// Makes sure that for each (workspace_id, path, kind) combination, only the latest `max_n` assets are kept.
// This does two things:
//  - Dedup the input vector
//  - Delete old assets from the database so that only `max_n` latest assets remain per (workspace_id, path, kind)
//    (taking into account the new assets being inserted)
async fn prune_runtime_assets(
    executor: &Pool<Postgres>,
    assets: Vec<InsertRuntimeAssetParams>,
    max_n: usize,
) -> error::Result<Vec<InsertRuntimeAssetParams>> {
    let mut unique_assets: HashMap<_, Vec<InsertRuntimeAssetParams>> = HashMap::new();
    for asset in assets.into_iter().rev() {
        let key = (
            asset.workspace_id.clone(),
            asset.asset_path.clone(),
            asset.asset_kind.clone(),
        );
        let v = unique_assets.entry(key).or_default();
        if let Some(last_same_job) = v.last_mut().filter(|a| a.job_id == asset.job_id) {
            // Same job used the same asset multiple times
            last_same_job.access_type =
                merge_asset_usage_access_types(last_same_job.access_type, asset.access_type);
            last_same_job.columns = merge_asset_columns(&last_same_job.columns, &asset.columns);
        } else if v.len() < max_n {
            v.push(asset);
        }
    }

    let (workspace_ids, asset_paths, asset_kinds, max_n_per_key): (Vec<_>, Vec<_>, Vec<_>, Vec<_>) =
        unique_assets
            .iter()
            .map(|((w, p, k), v)| (w.clone(), p.clone(), k.clone(), (max_n - v.len()) as i32))
            .multiunzip();

    let delete_result = sqlx::query!(
        r#"
        DELETE FROM asset
        WHERE (workspace_id, path, kind) IN (
            SELECT workspace_id, path, kind FROM (
                SELECT a.workspace_id, a.path, a.kind, a.usage_kind, ROW_NUMBER() OVER (
                    PARTITION BY a.workspace_id, a.path, a.kind
                    ORDER BY a.created_at DESC
                ) as rn,
                limits.max_n
                FROM asset a
                INNER JOIN (
                    SELECT * FROM UNNEST(
                        $1::varchar[], 
                        $2::varchar[], 
                        $3::asset_kind[],
                        $4::int[]
                    ) AS t(workspace_id, path, kind, max_n)
                ) limits
                ON a.workspace_id = limits.workspace_id 
                AND a.path = limits.path 
                AND a.kind = limits.kind
                WHERE a.usage_kind = 'job'
            ) ranked
            WHERE rn > max_n
        )"#,
        &workspace_ids,
        &asset_paths,
        &asset_kinds as &Vec<AssetKind>,
        &max_n_per_key
    )
    .execute(executor)
    .await
    .map_err(|error| error::Error::SqlErr {
        error,
        location: "prune_runtime_assets".to_string(),
    })?;

    tracing::debug!("Pruned {} runtime assets", delete_result.rows_affected());

    Ok(unique_assets.into_values().flatten().collect_vec())
}

pub const RUNTIME_ASSET_CHANNEL_CAPACITY: usize = 10_000;
// To avoid workers having to insert lots of runtime asset rows when detecting them,
// we use a channel to batch insert them periodically.
pub fn init_runtime_asset_loop(
    executor: Pool<Postgres>,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let (tx, mut rx) = mpsc::channel(RUNTIME_ASSET_CHANNEL_CAPACITY);
    match RUNTIME_ASSET_SENDER.set(tx) {
        Ok(_) => {}
        Err(_) => {
            tracing::debug!("RUNTIME_ASSET_SENDER was already set, skipping");
            return;
        }
    }
    tokio::spawn(async move {
        let flush_interval = tokio::time::Duration::from_secs(120);
        let mut flush_timer = tokio::time::interval(flush_interval);
        loop {
            tokio::select! {
                _ = flush_timer.tick() => {
                    if rx.is_closed() { break; }
                    let mut buffer = Vec::with_capacity(RUNTIME_ASSET_CHANNEL_CAPACITY);
                    while let Ok(asset) = rx.try_recv() {
                        buffer.push(asset);
                        if buffer.len() >= RUNTIME_ASSET_CHANNEL_CAPACITY { break; }
                    }
                    if buffer.is_empty() { continue; }
                    let buffer = match prune_runtime_assets(&executor, buffer, 10).await {
                        Ok(buf) => buf,
                        Err(e) => { tracing::error!("Failed to prune runtime assets: {e}"); continue; }
                    };
                    if let Err(e) = insert_runtime_assets(&executor, &buffer).await {
                        tracing::error!("Failed to insert runtime assets batch: {e}");
                    }
                }
                _ = killpill_rx.recv() => {
                    tracing::info!("Shutting down runtime asset inserter loop");
                    break;
                }
            }
        }
    });
}

pub fn register_runtime_asset(mut asset: InsertRuntimeAssetParams) {
    // Capture timestamp at registration time to preserve ordering when batch-inserted later
    asset.created_at = Some(chrono::Utc::now());

    let job_id = asset.job_id;
    let Some(runtime_asset_tx) = RUNTIME_ASSET_SENDER.get() else {
        tracing::error!("Failed to send runtime asset to channel for job {job_id}: RUNTIME_ASSET_SENDER.get() returned None");
        return;
    };
    if let Err(e) = runtime_asset_tx.try_send(asset) {
        // Log the error but do not fail the job execution
        tracing::error!("Failed to send runtime asset to channel for job {job_id}: {e}",);
    }
}

static RUNTIME_ASSET_SENDER: OnceLock<mpsc::Sender<InsertRuntimeAssetParams>> = OnceLock::new();
