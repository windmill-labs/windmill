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
use sqlx::types::Json;
use sqlx::{PgExecutor, Postgres, Transaction};
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

/// One column of a captured asset output schema: its name and substrate type
/// (e.g. `{"name": "order_id", "type": "BIGINT"}`). `type` is the substrate's
/// own type spelling (DuckDB for ducklake) — kept verbatim so #2b can compare
/// declared vs. captured without a lossy normalization step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaColumn {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
}

/// The materialization outcome an agent worker (`Connection::Http`, no direct
/// DB) sends to the API to be recorded. Mirrors the `record_materialization`
/// args; the API handler unpacks it and calls that function with its own DB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordMaterializationRequest {
    pub asset_kind: AssetKind,
    pub asset_path: String,
    pub partition: String,
    pub status: MaterializationStatus,
    pub snapshot_id: Option<i64>,
    pub row_count: Option<i64>,
    pub job_id: Option<Uuid>,
    pub error: Option<String>,
    /// Captured output schema of the materialized asset (`None` when the
    /// substrate/run produced no schema, e.g. a failed run or a polyglot helper
    /// that doesn't DESCRIBE). When present, the recorder also upserts a
    /// `materialized_asset_schema` version. Defaults to `None` so older agents
    /// stay wire-compatible.
    #[serde(default)]
    pub schema: Option<Vec<SchemaColumn>>,
}

/// Upsert the latest materialization state for one (asset, partition) slice.
/// The worker records the terminal outcome once the write finishes:
/// `Materialized` (with the DuckLake `snapshot_id` + `row_count`) or `Failed`
/// (with `error`). `Running` mirrors the pg enum but has no writer in this flow.
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
    sqlx::query!(
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
        workspace_id,
        asset_kind as AssetKind,
        asset_path,
        partition,
        status as MaterializationStatus,
        snapshot_id,
        row_count,
        job_id,
        error,
    )
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
    let rows = sqlx::query_as!(
        MaterializedPartition,
        r#"SELECT asset_kind AS "asset_kind: AssetKind", asset_path, partition,
                  status AS "status: MaterializationStatus", snapshot_id,
                  row_count, job_id, materialized_at, error
             FROM materialized_partition
            WHERE workspace_id = $1 AND asset_kind = $2 AND asset_path = $3
            ORDER BY partition DESC"#,
        workspace_id,
        asset_kind as AssetKind,
        asset_path,
    )
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// One captured schema version of an asset, newest first — the schema-evolution
/// history surfaced on the asset node and read by #2b contract enforcement.
#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct AssetSchemaVersion {
    pub version: i64,
    pub columns: Json<Vec<SchemaColumn>>,
    pub snapshot_id: Option<i64>,
    pub job_id: Option<Uuid>,
    pub captured_at: DateTime<Utc>,
}

/// Record the captured output schema of a freshly-materialized asset.
///
/// **Authorization:** like the sibling `record_materialization`, this performs
/// no access control of its own — it writes the row for whatever `workspace_id`
/// it is given. Callers MUST pass a workspace-authorized executor and a
/// `workspace_id` the caller is allowed to write: an RLS-scoped `user_db`
/// transaction for API / agent-worker entry points, or the trusted worker DB
/// pool for the in-worker recorder. Do not expose it to an unauthenticated path.
///
/// Versioning across re-materializations: a new `version` row is inserted only
/// when `columns` differs from the latest stored version; an unchanged
/// re-materialize re-affirms the latest row in place (updates its
/// `snapshot_id`/`job_id`/`captured_at`). The result is a compact
/// schema-evolution history where `MAX(version)` is the current contract.
///
/// Runs in a transaction guarded by a per-asset advisory lock so two concurrent
/// materializations of the same asset can't both insert the same next version
/// or interleave a stale comparison. Returns `true` if a new version was
/// inserted (the schema changed), `false` if the latest was re-affirmed.
pub async fn record_asset_schema(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    asset_kind: AssetKind,
    asset_path: &str,
    columns: &[SchemaColumn],
    snapshot_id: Option<i64>,
    job_id: Option<Uuid>,
) -> Result<bool> {
    // Serialize concurrent captures of the *same* asset; the lock auto-releases
    // at tx end. Hash the identity into the bigint advisory-lock key space.
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0::int8))",
        format!("materialized_asset_schema:{workspace_id}:{asset_kind:?}:{asset_path}"),
    )
    .fetch_one(&mut **tx)
    .await?;

    let latest = sqlx::query!(
        r#"SELECT version, columns AS "columns: Json<Vec<SchemaColumn>>"
             FROM materialized_asset_schema
            WHERE workspace_id = $1 AND asset_kind = $2 AND asset_path = $3
            ORDER BY version DESC
            LIMIT 1"#,
        workspace_id,
        asset_kind as AssetKind,
        asset_path,
    )
    .fetch_optional(&mut **tx)
    .await?;

    let columns_json = Json(columns.to_vec());
    let next_version = match latest {
        Some(latest) if latest.columns.0.as_slice() == columns => {
            // Unchanged schema — re-affirm the latest version in place.
            sqlx::query!(
                "UPDATE materialized_asset_schema
                    SET snapshot_id = $5, job_id = $6, captured_at = now()
                  WHERE workspace_id = $1 AND asset_kind = $2 AND asset_path = $3
                    AND version = $4",
                workspace_id,
                asset_kind as AssetKind,
                asset_path,
                latest.version,
                snapshot_id,
                job_id,
            )
            .execute(&mut **tx)
            .await?;
            return Ok(false);
        }
        Some(latest) => latest.version + 1,
        None => 1,
    };
    sqlx::query!(
        "INSERT INTO materialized_asset_schema
           (workspace_id, asset_kind, asset_path, version, columns,
            snapshot_id, job_id, captured_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, now())",
        workspace_id,
        asset_kind as AssetKind,
        asset_path,
        next_version,
        columns_json as Json<Vec<SchemaColumn>>,
        snapshot_id,
        job_id,
    )
    .execute(&mut **tx)
    .await?;
    Ok(true)
}

/// One table a fork workspace should read from its parent through a defer view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkDeferTable {
    /// Lake-internal table name: the `asset_path` minus its `<lake>/` prefix (may itself be
    /// `schema.table`).
    pub table: String,
    /// The parent asset's latest captured schema carries the SCD2 marker column
    /// (`is_current`), so the parent lake also holds a managed `<table>_current` companion
    /// view that consumers read — defer it alongside the table.
    #[serde(default)]
    pub with_current_view: bool,
}

/// Tables of lake `lake_name` materialized in the parent workspace but not (yet) in the fork —
/// the fork's read-defer set. Only `Materialized` rows count on either side: a deferred parent
/// table must physically exist (defer views bind at CREATE and would otherwise fail the whole
/// job), and any successful fork materialization makes the fork's own table authoritative.
///
/// **Authorization:** performs no access control; trusted server-side callers only. It reads
/// the *parent* workspace's rows on behalf of a fork — acceptable because defer itself exposes
/// the parent's table contents to fork members.
pub async fn list_fork_defer_tables<'e>(
    executor: impl PgExecutor<'e>,
    parent_workspace_id: &str,
    fork_workspace_id: &str,
    lake_name: &str,
) -> Result<Vec<ForkDeferTable>> {
    let rows = sqlx::query!(
        r#"
        WITH parent_mat AS (
            SELECT DISTINCT asset_path FROM materialized_partition
            WHERE workspace_id = $1 AND asset_kind = 'ducklake' AND status = 'materialized'
              AND split_part(asset_path, '/', 1) = $3 AND asset_path LIKE '%/%'
        ), fork_mat AS (
            SELECT DISTINCT asset_path FROM materialized_partition
            WHERE workspace_id = $2 AND asset_kind = 'ducklake' AND status = 'materialized'
        ), latest_schema AS (
            SELECT DISTINCT ON (asset_path) asset_path, columns
            FROM materialized_asset_schema
            WHERE workspace_id = $1 AND asset_kind = 'ducklake'
            ORDER BY asset_path, version DESC
        )
        SELECT p.asset_path AS "asset_path!",
               COALESCE(EXISTS (
                   SELECT 1 FROM jsonb_array_elements(ls.columns) e
                   WHERE e->>'name' = 'is_current'
               ), false) AS "has_current!"
        FROM parent_mat p
        LEFT JOIN latest_schema ls ON ls.asset_path = p.asset_path
        WHERE p.asset_path NOT IN (SELECT asset_path FROM fork_mat)
        ORDER BY p.asset_path
        "#,
        parent_workspace_id,
        fork_workspace_id,
        lake_name,
    )
    .fetch_all(executor)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| ForkDeferTable {
            table: r.asset_path[lake_name.len() + 1..].to_string(),
            with_current_view: r.has_current,
        })
        .collect())
}

/// All captured schema versions for one asset, newest version first.
///
/// **Authorization:** performs no access control (mirrors
/// `list_materialized_partitions`); the caller must pass a workspace-authorized
/// executor (an RLS-scoped `user_db` transaction on the API read path) and a
/// `workspace_id` it is allowed to read.
pub async fn list_asset_schemas<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    asset_kind: AssetKind,
    asset_path: &str,
) -> Result<Vec<AssetSchemaVersion>> {
    let rows = sqlx::query_as!(
        AssetSchemaVersion,
        r#"SELECT version, columns AS "columns: Json<Vec<SchemaColumn>>",
                  snapshot_id, job_id, captured_at
             FROM materialized_asset_schema
            WHERE workspace_id = $1 AND asset_kind = $2 AND asset_path = $3
            ORDER BY version DESC"#,
        workspace_id,
        asset_kind as AssetKind,
        asset_path,
    )
    .fetch_all(executor)
    .await?;
    Ok(rows)
}
