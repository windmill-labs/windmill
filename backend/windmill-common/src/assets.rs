use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, Pool, Postgres, QueryBuilder};
use tokio::sync::{mpsc, Mutex};

use crate::error;

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Hash, Eq, sqlx::Type)]
#[sqlx(type_name = "ASSET_KIND", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum AssetKind {
    S3Object,
    Resource,
    // Avoid unnexpected crashes when deserializing old assets
    Variable, // Deprecated
    Ducklake,
    DataTable,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Hash, Eq, sqlx::Type)]
#[sqlx(type_name = "ASSET_USAGE_KIND", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum AssetUsageKind {
    Script,
    Flow,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Hash, Eq, sqlx::Type)]
#[sqlx(type_name = "ASSET_DETECTION_KIND", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum AssetDetectionKind {
    Static,
    Runtime,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Hash, Eq, sqlx::Type)]
#[sqlx(type_name = "ASSET_ACCESS_TYPE", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum AssetUsageAccessType {
    R,
    W,
    RW,
}

pub struct Asset {
    pub path: String,
    pub kind: AssetKind,
}

pub struct AssetUsage {
    pub path: String,
    pub kind: AssetUsageKind,
    pub access_type: AssetUsageAccessType,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, sqlx::Type)]
pub struct AssetWithAltAccessType {
    pub path: String,
    pub kind: AssetKind,
    pub access_type: Option<AssetUsageAccessType>,
    pub alt_access_type: Option<AssetUsageAccessType>,
}

pub async fn insert_static_asset_usage<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    asset: &AssetWithAltAccessType,
    usage_path: &str,
    usage_kind: AssetUsageKind,
) -> error::Result<()> {
    sqlx::query!(
        r#"INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind, asset_detection_kind, job_id)
                VALUES ($1, $2, $3, $4, $5, $6, 'static', NULL) ON CONFLICT DO NOTHING"#,
        workspace_id,
        asset.path,
        asset.kind as AssetKind,
        (asset.access_type.or(asset.alt_access_type)) as Option<AssetUsageAccessType>,
        usage_path,
        usage_kind as AssetUsageKind
    )
    .execute(executor)
    .await?;

    Ok(())
}

pub struct InsertRuntimeAssetParams {
    pub workspace_id: String,
    pub job_id: uuid::Uuid,
    pub asset_path: String,
    pub asset_kind: AssetKind,
    pub usage_path: String,
    pub usage_kind: AssetUsageKind,
}

async fn insert_runtime_assets(
    executor: &Pool<Postgres>,
    assets: &[InsertRuntimeAssetParams],
) -> error::Result<()> {
    for chunk in assets.chunks(1000) {
        let mut query_builder = QueryBuilder::new("INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind, asset_detection_kind, job_id) ");
        query_builder.push_values(chunk, |mut b, asset| {
            b.push_bind(&asset.workspace_id)
                .push_bind(&asset.asset_path)
                .push_bind(&asset.asset_kind)
                .push_bind(Option::<AssetUsageAccessType>::None)
                .push_bind(&asset.usage_path)
                .push_bind(&asset.usage_kind)
                .push_bind(AssetDetectionKind::Runtime)
                .push_bind(&asset.job_id);
        });
        query_builder.build().execute(executor).await?;
    }
    Ok(())
}

// To avoid workers having to insert lots of runtime asset rows when detecting them,
// we use a channel to batch insert them periodically.
pub fn init_runtime_asset_inserter(executor: Pool<Postgres>) {
    tokio::spawn(async move {
        let _ = &*RUNTIME_ASSET_SENDER; // Force initialization
        let flush_interval = tokio::time::Duration::from_secs(120);
        let mut flush_timer = tokio::time::interval(flush_interval);

        let mut buffer = Vec::with_capacity(RUNTIME_ASSET_CHANNEL_CAPACITY);
        let mut rx = match RUNTIME_ASSET_RECEIVER.try_lock() {
            Ok(mut guard) => match guard.take() {
                Some(rx) => rx,
                None => {
                    tracing::error!("Runtime asset rx not initialized (Impossible state)");
                    return;
                }
            },
            Err(e) => {
                tracing::error!("Failed to acquire lock for runtime asset receiver: {e}");
                return;
            }
        };
        loop {
            tokio::select! {
                _ = flush_timer.tick() => {
                    if rx.is_closed() { break; }
                    rx.recv_many(&mut buffer, RUNTIME_ASSET_CHANNEL_CAPACITY).await;
                    if buffer.is_empty() { continue; }
                    if let Err(e) = insert_runtime_assets(&executor, &buffer).await {
                        tracing::error!("Failed to insert runtime assets batch: {e}");
                    }
                    buffer.clear();
                }
            }
        }
    });
}

const RUNTIME_ASSET_CHANNEL_CAPACITY: usize = 10_000;
lazy_static::lazy_static! {
    pub static ref RUNTIME_ASSET_SENDER: mpsc::Sender<InsertRuntimeAssetParams> = {
        let (tx, rx) = mpsc::channel(RUNTIME_ASSET_CHANNEL_CAPACITY);
        *RUNTIME_ASSET_RECEIVER.blocking_lock() = Some(rx);
        tx
    };
    static ref RUNTIME_ASSET_RECEIVER: Mutex<Option<mpsc::Receiver<InsertRuntimeAssetParams>>> = Mutex::new(None);
}

pub async fn clear_static_asset_usage<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    usage_path: &str,
    usage_kind: AssetUsageKind,
) -> error::Result<()> {
    sqlx::query!(
        r#"DELETE FROM asset WHERE workspace_id = $1 AND usage_path = $2 AND usage_kind = $3 AND asset_detection_kind = 'static'"#,
        workspace_id,
        usage_path,
        usage_kind as AssetUsageKind
    )
    .execute(executor)
    .await?;

    Ok(())
}
