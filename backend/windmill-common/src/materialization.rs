//! CE materialization state — the per-partition status recorded by the managed
//! `// materialize` write (in windmill-worker), read by the partition-status
//! grid and by the EE backfill worklist.
//!
//! The write engine and this state are CE; only automatic partition
//! *resolution* (`partition_ee`) and *backfill* orchestration
//! (`pipeline_advanced_ee`) are enterprise. This module is the shared seam:
//! the EE backfill enumerates the partitions in a range, diffs them against
//! these rows to find the missing/failed set, and pushes one CE materialization
//! job per gap (with an explicit `partition` arg — which runs idempotently and
//! upserts the row here). Nothing about that orchestration lives in this file;
//! it only needs the rows to exist, which is why recording is CE.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::assets::AssetKind;
use crate::error::Result;

/// Sentinel `partition` value for an unpartitioned (whole-table)
/// materialization — partition is part of the primary key and cannot be NULL.
pub const UNPARTITIONED: &str = "";

/// Mirrors the `MATERIALIZATION_STATUS` pg enum (see migration
/// `20260619170118_add_materialized_partition`).
#[derive(sqlx::Type, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[sqlx(type_name = "MATERIALIZATION_STATUS", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MaterializationStatus {
    Running,
    Materialized,
    Failed,
}

/// Upsert the latest materialization state for one (asset, partition) slice.
/// The worker records `Running` before the write, then `Materialized` (with the
/// DuckLake `snapshot_id` + `row_count`) or `Failed` (with `error`) after.
/// Idempotent: re-running the same partition overwrites the row — exactly the
/// backfill / failure-recovery contract.
#[allow(clippy::too_many_arguments)]
pub async fn record_materialization<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    asset_kind: AssetKind,
    asset_path: &str,
    partition: &str,
    status: MaterializationStatus,
    snapshot_id: Option<i64>,
    row_count: Option<i64>,
    job_id: Option<Uuid>,
    error: Option<&str>,
) -> Result<()> {
    // Unchecked query: keeps the offline sqlx cache untouched for a brand-new
    // table. Convert to `query!` + `cargo sqlx prepare` once the migration is
    // applied to the check DB (see the update-sqlx skill).
    sqlx::query(
        "INSERT INTO materialized_partition
           (workspace_id, asset_kind, asset_path, partition, status,
            snapshot_id, row_count, job_id, materialized_at, error)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now(), $9)
         ON CONFLICT (workspace_id, asset_kind, asset_path, partition)
         DO UPDATE SET status = EXCLUDED.status,
                       snapshot_id = EXCLUDED.snapshot_id,
                       row_count = EXCLUDED.row_count,
                       job_id = EXCLUDED.job_id,
                       materialized_at = now(),
                       error = EXCLUDED.error",
    )
    .bind(workspace_id)
    .bind(asset_kind)
    .bind(asset_path)
    .bind(partition)
    .bind(status)
    .bind(snapshot_id)
    .bind(row_count)
    .bind(job_id)
    .bind(error)
    .execute(executor)
    .await?;
    Ok(())
}

/// One materialized-partition row, for the status grid / backfill diff.
#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct MaterializedPartition {
    pub asset_kind: AssetKind,
    pub asset_path: String,
    pub partition: String,
    pub status: MaterializationStatus,
    pub snapshot_id: Option<i64>,
    pub row_count: Option<i64>,
    pub job_id: Option<Uuid>,
    pub materialized_at: DateTime<Utc>,
    pub error: Option<String>,
}

/// All recorded partitions for one asset, newest first — the grid's data and
/// the backfill worklist's "what already exists" set.
pub async fn list_materialized_partitions<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    asset_kind: AssetKind,
    asset_path: &str,
) -> Result<Vec<MaterializedPartition>> {
    let rows = sqlx::query_as::<_, MaterializedPartition>(
        "SELECT asset_kind, asset_path, partition, status, snapshot_id,
                row_count, job_id, materialized_at, error
           FROM materialized_partition
          WHERE workspace_id = $1 AND asset_kind = $2 AND asset_path = $3
          ORDER BY partition DESC",
    )
    .bind(workspace_id)
    .bind(asset_kind)
    .bind(asset_path)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}
