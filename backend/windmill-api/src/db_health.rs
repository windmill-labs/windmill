/*
 * Author: Windmill Labs
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{routing::get, Extension, Json, Router};
use serde::Serialize;

use windmill_common::error::JsonResult;

use crate::db::{ApiAuthed, DB};
use crate::health::get_pool_stats;
use crate::health::PoolStats;
use crate::utils::require_super_admin;

pub fn global_service() -> Router {
    Router::new().route("/", get(get_db_health))
}

// --- Response types ---

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthLevel {
    Green,
    Yellow,
    Red,
}

#[derive(Serialize)]
pub struct DbHealthResponse {
    pub database_size: DatabaseSizeInfo,
    pub job_retention: JobRetentionInfo,
    pub large_results: LargeResultsInfo,
    pub connection_pool: ConnectionPoolInfo,
    pub table_maintenance: Vec<TableMaintenanceInfo>,
    pub slow_queries: Option<SlowQueriesInfo>,
    pub datatables: Vec<DatatableInfo>,
}

#[derive(Serialize)]
pub struct DatabaseSizeInfo {
    pub total_size_bytes: i64,
    pub total_size_pretty: String,
    pub top_tables: Vec<TableSizeInfo>,
}

#[derive(Serialize)]
pub struct TableSizeInfo {
    pub table_name: String,
    pub total_size_bytes: i64,
    pub total_size_pretty: String,
}

#[derive(Serialize)]
pub struct JobRetentionInfo {
    pub oldest_completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub total_completed_jobs: i64,
    pub retention_period_secs: Option<i64>,
    pub status: HealthLevel,
    pub message: String,
}

#[derive(Serialize)]
pub struct LargeResultsInfo {
    pub top_large_results: Vec<LargeResultRow>,
    pub avg_result_size_bytes: Option<i64>,
}

#[derive(Serialize)]
pub struct LargeResultRow {
    pub id: uuid::Uuid,
    pub workspace_id: String,
    pub runnable_path: Option<String>,
    pub result_size_bytes: i64,
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct ConnectionPoolInfo {
    pub pool: PoolStats,
    pub pg_active_connections: i64,
    pub status: HealthLevel,
    pub message: String,
}

#[derive(Serialize)]
pub struct TableMaintenanceInfo {
    pub table_name: String,
    pub live_tuples: i64,
    pub dead_tuples: i64,
    pub dead_ratio: f64,
    pub last_autovacuum: Option<chrono::NaiveDateTime>,
    pub last_autoanalyze: Option<chrono::NaiveDateTime>,
    pub status: HealthLevel,
}

#[derive(Serialize)]
pub struct SlowQueriesInfo {
    pub queries: Vec<SlowQueryRow>,
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct SlowQueryRow {
    pub query: String,
    pub calls: i64,
    pub total_exec_time_ms: f64,
    pub mean_exec_time_ms: f64,
}

#[derive(Serialize)]
pub struct DatatableInfo {
    pub workspace_id: String,
    pub name: String,
    pub table_name: String,
    pub size_bytes: i64,
    pub size_pretty: String,
    pub estimated_rows: f64,
}

// --- Handler ---

async fn get_db_health(
    ApiAuthed { email, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<DbHealthResponse> {
    require_super_admin(&db, &email).await?;

    let (
        database_size,
        job_retention,
        large_results,
        connection_pool,
        table_maintenance,
        slow_queries,
        datatables,
    ) = tokio::try_join!(
        fetch_database_size(&db),
        fetch_job_retention(&db),
        fetch_large_results(&db),
        fetch_connection_pool(&db),
        fetch_table_maintenance(&db),
        fetch_slow_queries(&db),
        fetch_datatables(&db),
    )?;

    Ok(Json(DbHealthResponse {
        database_size,
        job_retention,
        large_results,
        connection_pool,
        table_maintenance,
        slow_queries,
        datatables,
    }))
}

// --- Diagnostic queries ---

async fn fetch_database_size(db: &DB) -> windmill_common::error::Result<DatabaseSizeInfo> {
    let row = sqlx::query!(
        "SELECT pg_database_size(current_database()) as size_bytes, pg_size_pretty(pg_database_size(current_database())) as size_pretty"
    )
    .fetch_one(db)
    .await?;

    let top_tables = sqlx::query_as!(
        TableSizeInfo,
        r#"SELECT
            schemaname || '.' || relname as "table_name!",
            pg_total_relation_size(relid) as "total_size_bytes!",
            pg_size_pretty(pg_total_relation_size(relid)) as "total_size_pretty!"
        FROM pg_catalog.pg_statio_user_tables
        ORDER BY pg_total_relation_size(relid) DESC
        LIMIT 15"#
    )
    .fetch_all(db)
    .await?;

    Ok(DatabaseSizeInfo {
        total_size_bytes: row.size_bytes.unwrap_or(0),
        total_size_pretty: row.size_pretty.unwrap_or_default(),
        top_tables,
    })
}

async fn fetch_job_retention(db: &DB) -> windmill_common::error::Result<JobRetentionInfo> {
    let job_row =
        sqlx::query!("SELECT MIN(completed_at) as oldest, COUNT(*) as total FROM v2_job_completed")
            .fetch_one(db)
            .await?;

    let retention_row =
        sqlx::query!("SELECT value FROM global_settings WHERE name = 'retention_period_secs'")
            .fetch_optional(db)
            .await?;

    let retention_period_secs: Option<i64> =
        retention_row.map(|r| r.value).and_then(|v| v.as_i64());

    let oldest = job_row.oldest;
    let total = job_row.total.unwrap_or(0);

    let (status, message) = if let (Some(oldest_ts), Some(retention_secs)) =
        (oldest, retention_period_secs)
    {
        let age_secs: i64 = (chrono::Utc::now() - oldest_ts).num_seconds();
        let ratio = if retention_secs > 0 {
            age_secs as f64 / retention_secs as f64
        } else {
            0.0
        };
        if ratio <= 2.0 {
            (
                HealthLevel::Green,
                format!(
                    "Oldest job is {:.1}x the retention period. Cleanup is keeping up.",
                    ratio
                ),
            )
        } else if ratio <= 5.0 {
            (
                HealthLevel::Yellow,
                format!(
                    "Oldest job is {:.1}x the retention period. Cleanup may be falling behind.",
                    ratio
                ),
            )
        } else {
            (HealthLevel::Red, format!("Oldest job is {:.1}x the retention period. Consider reducing retention or investigating cleanup.", ratio))
        }
    } else if oldest.is_some() && retention_period_secs.is_none() {
        (
            HealthLevel::Yellow,
            "No retention_period_secs configured. Old jobs will accumulate.".to_string(),
        )
    } else {
        (HealthLevel::Green, "No completed jobs found.".to_string())
    };

    Ok(JobRetentionInfo {
        oldest_completed_at: oldest,
        total_completed_jobs: total,
        retention_period_secs,
        status,
        message,
    })
}

async fn fetch_large_results(db: &DB) -> windmill_common::error::Result<LargeResultsInfo> {
    let top_large_results = sqlx::query_as!(
        LargeResultRow,
        r#"SELECT
            c.id as "id!",
            c.workspace_id as "workspace_id!",
            j.runnable_path as "runnable_path",
            pg_column_size(c.result) as "result_size_bytes!",
            c.completed_at as "completed_at!"
        FROM v2_job_completed c
        LEFT JOIN v2_job j ON j.id = c.id
        WHERE c.completed_at > now() - interval '30 days'
          AND c.result IS NOT NULL
        ORDER BY pg_column_size(c.result) DESC
        LIMIT 10"#
    )
    .fetch_all(db)
    .await?;

    let avg_row = sqlx::query!(
        r#"SELECT AVG(pg_column_size(result))::bigint as "avg_size"
        FROM (
            SELECT result FROM v2_job_completed
            WHERE completed_at > now() - interval '30 days'
              AND result IS NOT NULL
            ORDER BY completed_at DESC
            LIMIT 10000
        ) sub"#
    )
    .fetch_one(db)
    .await?;

    Ok(LargeResultsInfo { top_large_results, avg_result_size_bytes: avg_row.avg_size })
}

async fn fetch_connection_pool(db: &DB) -> windmill_common::error::Result<ConnectionPoolInfo> {
    let pool = get_pool_stats(db);

    let active_row =
        sqlx::query!("SELECT COUNT(*) as cnt FROM pg_stat_activity WHERE state = 'active'")
            .fetch_one(db)
            .await?;

    let pg_active = active_row.cnt.unwrap_or(0);
    let utilization = if pool.max_connections > 0 {
        pool.size as f64 / pool.max_connections as f64
    } else {
        0.0
    };

    let (status, message) = if utilization < 0.8 {
        (
            HealthLevel::Green,
            format!(
                "Pool utilization: {:.0}% ({}/{})",
                utilization * 100.0,
                pool.size,
                pool.max_connections
            ),
        )
    } else if utilization < 0.95 {
        (
            HealthLevel::Yellow,
            format!(
                "Pool utilization is high: {:.0}% ({}/{}). Consider increasing max_connections.",
                utilization * 100.0,
                pool.size,
                pool.max_connections
            ),
        )
    } else {
        (
            HealthLevel::Red,
            format!(
                "Pool near exhaustion: {:.0}% ({}/{}). Increase max_connections urgently.",
                utilization * 100.0,
                pool.size,
                pool.max_connections
            ),
        )
    };

    Ok(ConnectionPoolInfo { pool, pg_active_connections: pg_active, status, message })
}

async fn fetch_table_maintenance(
    db: &DB,
) -> windmill_common::error::Result<Vec<TableMaintenanceInfo>> {
    let rows = sqlx::query!(
        r#"SELECT
            schemaname || '.' || relname as "table_name!",
            COALESCE(n_live_tup, 0) as "live_tuples!",
            COALESCE(n_dead_tup, 0) as "dead_tuples!",
            last_autovacuum as "last_autovacuum",
            last_autoanalyze as "last_autoanalyze"
        FROM pg_stat_user_tables
        ORDER BY n_dead_tup DESC
        LIMIT 15"#
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let total = r.live_tuples + r.dead_tuples;
            let dead_ratio = if total > 0 {
                r.dead_tuples as f64 / total as f64
            } else {
                0.0
            };
            let status = if dead_ratio < 0.1 {
                HealthLevel::Green
            } else if dead_ratio < 0.3 {
                HealthLevel::Yellow
            } else {
                HealthLevel::Red
            };
            TableMaintenanceInfo {
                table_name: r.table_name,
                live_tuples: r.live_tuples,
                dead_tuples: r.dead_tuples,
                dead_ratio,
                last_autovacuum: r.last_autovacuum.map(|t| t.naive_utc()),
                last_autoanalyze: r.last_autoanalyze.map(|t| t.naive_utc()),
                status,
            }
        })
        .collect())
}

async fn fetch_slow_queries(db: &DB) -> windmill_common::error::Result<Option<SlowQueriesInfo>> {
    let ext_exists: bool = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'pg_stat_statements') as "exists!""#
    )
    .fetch_one(db)
    .await?;

    if !ext_exists {
        return Ok(Some(SlowQueriesInfo {
            queries: vec![],
            message: Some(
                "pg_stat_statements extension is not installed. Enable it for slow query insights."
                    .to_string(),
            ),
        }));
    }

    // Use raw query since pg_stat_statements may not exist at compile time
    let rows: Vec<SlowQueryRow> = sqlx::query_as::<_, (String, i64, f64, f64)>(
        r#"SELECT
            LEFT(query, 200),
            calls::bigint,
            total_exec_time::float8,
            mean_exec_time::float8
        FROM pg_stat_statements
        WHERE query NOT LIKE '%pg_stat_statements%'
        ORDER BY mean_exec_time DESC
        LIMIT 10"#,
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(
        |(query, calls, total_exec_time_ms, mean_exec_time_ms)| SlowQueryRow {
            query,
            calls,
            total_exec_time_ms,
            mean_exec_time_ms,
        },
    )
    .collect();

    Ok(Some(SlowQueriesInfo { queries: rows, message: None }))
}

async fn fetch_datatables(db: &DB) -> windmill_common::error::Result<Vec<DatatableInfo>> {
    // Find instance-type datatables from workspace_settings
    let rows = sqlx::query!(
        r#"SELECT
            ws.workspace_id as "workspace_id!",
            dt.key as "name!",
            dt.value->>'table_name' as "table_name"
        FROM workspace_settings ws,
             jsonb_each(ws.datatable) dt
        WHERE dt.value->>'resource_type' = 'instance'
          AND dt.value->>'table_name' IS NOT NULL"#
    )
    .fetch_all(db)
    .await?;

    let mut result = Vec::new();
    for row in rows {
        let table_name = match &row.table_name {
            Some(t) => t.clone(),
            None => continue,
        };
        // Get size for each datatable
        let size_row = sqlx::query!(
            r#"SELECT
                pg_total_relation_size(c.oid) as "size_bytes!",
                pg_size_pretty(pg_total_relation_size(c.oid)) as "size_pretty!",
                COALESCE(c.reltuples, 0) as "estimated_rows!"
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE n.nspname = 'public' AND c.relname = $1"#,
            table_name
        )
        .fetch_optional(db)
        .await?;

        if let Some(s) = size_row {
            result.push(DatatableInfo {
                workspace_id: row.workspace_id,
                name: row.name,
                table_name,
                size_bytes: s.size_bytes,
                size_pretty: s.size_pretty,
                estimated_rows: s.estimated_rows as f64,
            });
        }
    }

    // Sort by size descending
    result.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    Ok(result)
}
