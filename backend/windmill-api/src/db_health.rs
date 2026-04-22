/*
 * Author: Windmill Labs
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::Query,
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};

use windmill_common::error::JsonResult;

use crate::db::{ApiAuthed, DB};
use crate::utils::require_super_admin;

pub fn global_service() -> Router {
    Router::new()
        .route("/", get(get_db_health))
        .route("/jobs", get(get_db_health_jobs))
        .route("/slow_queries", get(get_slow_queries))
        .route("/slow_queries/reset", post(reset_slow_queries))
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
    pub connection_pool: ConnectionPoolInfo,
    pub table_maintenance: Vec<TableMaintenanceInfo>,
    pub slow_queries: Option<SlowQueriesInfo>,
    pub datatables: Vec<DatatableInfo>,
}

#[derive(Serialize)]
pub struct DbHealthJobsResponse {
    pub job_retention: JobRetentionInfo,
    pub large_results: LargeResultsInfo,
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
    pub pg_max_connections: i64,
    pub pg_total_connections: i64,
    pub pg_active_connections: i64,
    pub pg_idle_connections: i64,
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
    /// When stats were last reset (from pg_stat_statements_info.stats_reset, PG 14+)
    pub stats_reset: Option<chrono::DateTime<chrono::Utc>>,
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

#[derive(Deserialize)]
struct DbHealthQuery {
    /// Max number of recent completed jobs to scan for large results (default 10000)
    scan_limit: Option<i64>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum SlowQuerySort {
    Total,
    Mean,
    Calls,
}

impl SlowQuerySort {
    fn order_by(&self) -> &'static str {
        match self {
            SlowQuerySort::Total => "total_exec_time DESC",
            SlowQuerySort::Mean => "mean_exec_time DESC",
            SlowQuerySort::Calls => "calls DESC",
        }
    }
}

#[derive(Deserialize)]
struct SlowQueriesQuery {
    sort: Option<SlowQuerySort>,
}

async fn get_db_health(
    ApiAuthed { email, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<DbHealthResponse> {
    require_super_admin(&db, &email).await?;

    let (database_size, connection_pool, table_maintenance, slow_queries, datatables) = tokio::try_join!(
        fetch_database_size(&db),
        fetch_connection_pool(&db),
        fetch_table_maintenance(&db),
        fetch_slow_queries(&db, SlowQuerySort::Total),
        fetch_datatables(&db),
    )?;

    Ok(Json(DbHealthResponse {
        database_size,
        connection_pool,
        table_maintenance,
        slow_queries,
        datatables,
    }))
}

async fn get_db_health_jobs(
    ApiAuthed { email, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Query(query): Query<DbHealthQuery>,
) -> JsonResult<DbHealthJobsResponse> {
    require_super_admin(&db, &email).await?;

    let scan_limit = query.scan_limit.unwrap_or(10_000).clamp(1_000, 1_000_000);

    let (job_retention, large_results) = tokio::try_join!(
        fetch_job_retention(&db),
        fetch_large_results(&db, scan_limit),
    )?;

    Ok(Json(DbHealthJobsResponse { job_retention, large_results }))
}

async fn get_slow_queries(
    ApiAuthed { email, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Query(query): Query<SlowQueriesQuery>,
) -> JsonResult<Option<SlowQueriesInfo>> {
    require_super_admin(&db, &email).await?;
    let sort = query.sort.unwrap_or(SlowQuerySort::Total);
    Ok(Json(fetch_slow_queries(&db, sort).await?))
}

async fn reset_slow_queries(
    ApiAuthed { email, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
) -> windmill_common::error::Result<StatusCode> {
    require_super_admin(&db, &email).await?;
    sqlx::query("SELECT pg_stat_statements_reset()")
        .execute(&db)
        .await
        .map_err(|e| {
            windmill_common::error::Error::InternalErr(format!(
                "Failed to reset pg_stat_statements (is the extension enabled?): {e}"
            ))
        })?;
    Ok(StatusCode::NO_CONTENT)
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

async fn fetch_large_results(
    db: &DB,
    scan_limit: i64,
) -> windmill_common::error::Result<LargeResultsInfo> {
    let top_large_results = sqlx::query_as!(
        LargeResultRow,
        r#"SELECT
            c.id as "id!",
            c.workspace_id as "workspace_id!",
            j.runnable_path as "runnable_path",
            pg_column_size(c.result) as "result_size_bytes!",
            c.completed_at as "completed_at!"
        FROM (
            SELECT id, workspace_id, result, completed_at
            FROM v2_job_completed
            WHERE completed_at > now() - interval '30 days'
              AND result IS NOT NULL
            ORDER BY completed_at DESC
            LIMIT $1
        ) c
        LEFT JOIN v2_job j ON j.id = c.id
        WHERE pg_column_size(c.result) > 1024
        ORDER BY pg_column_size(c.result) DESC
        LIMIT 10"#,
        scan_limit
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
            LIMIT $1
        ) sub"#,
        scan_limit
    )
    .fetch_one(db)
    .await?;

    Ok(LargeResultsInfo { top_large_results, avg_result_size_bytes: avg_row.avg_size })
}

async fn fetch_connection_pool(db: &DB) -> windmill_common::error::Result<ConnectionPoolInfo> {
    let max_row = sqlx::query_scalar!(
        r#"SELECT setting::bigint as "max!" FROM pg_settings WHERE name = 'max_connections'"#
    )
    .fetch_one(db)
    .await?;

    let stats_row = sqlx::query!(
        r#"SELECT
            COUNT(*) as "total!",
            COUNT(*) FILTER (WHERE state = 'active') as "active!",
            COUNT(*) FILTER (WHERE state = 'idle') as "idle!"
        FROM pg_stat_activity
        WHERE backend_type = 'client backend'"#
    )
    .fetch_one(db)
    .await?;

    let pg_max = max_row;
    let pg_total = stats_row.total;
    let pg_active = stats_row.active;
    let pg_idle = stats_row.idle;

    let utilization = if pg_max > 0 {
        pg_total as f64 / pg_max as f64
    } else {
        0.0
    };

    let (status, message) = if utilization < 0.8 {
        (
            HealthLevel::Green,
            format!(
                "Connection utilization: {:.0}% ({}/{})",
                utilization * 100.0,
                pg_total,
                pg_max
            ),
        )
    } else if utilization < 0.95 {
        (
            HealthLevel::Yellow,
            format!(
                "Connection utilization is high: {:.0}% ({}/{}). Consider increasing max_connections.",
                utilization * 100.0,
                pg_total,
                pg_max
            ),
        )
    } else {
        (
            HealthLevel::Red,
            format!(
                "Connections near exhaustion: {:.0}% ({}/{}). Increase max_connections urgently.",
                utilization * 100.0,
                pg_total,
                pg_max
            ),
        )
    };

    Ok(ConnectionPoolInfo {
        pg_max_connections: pg_max,
        pg_total_connections: pg_total,
        pg_active_connections: pg_active,
        pg_idle_connections: pg_idle,
        status,
        message,
    })
}

async fn fetch_table_maintenance(
    db: &DB,
) -> windmill_common::error::Result<Vec<TableMaintenanceInfo>> {
    // Aggregate partitioned tables (e.g. audit_YYYYMMDD -> audit_partitioned)
    // while keeping non-partitioned tables as-is
    let rows = sqlx::query!(
        r#"SELECT
            table_name as "table_name!",
            SUM(live_tuples)::bigint as "live_tuples!",
            SUM(dead_tuples)::bigint as "dead_tuples!",
            MAX(last_autovacuum) as "last_autovacuum",
            MAX(last_autoanalyze) as "last_autoanalyze"
        FROM (
            SELECT
                CASE
                    WHEN i.inhparent IS NOT NULL THEN schemaname || '.' || p.relname
                    ELSE schemaname || '.' || s.relname
                END as table_name,
                COALESCE(n_live_tup, 0) as live_tuples,
                COALESCE(n_dead_tup, 0) as dead_tuples,
                last_autovacuum,
                last_autoanalyze
            FROM pg_stat_user_tables s
            LEFT JOIN pg_class c ON c.relname = s.relname AND c.relnamespace = (
                SELECT oid FROM pg_namespace WHERE nspname = s.schemaname
            )
            LEFT JOIN pg_inherits i ON i.inhrelid = c.oid
            LEFT JOIN pg_class p ON p.oid = i.inhparent
        ) sub
        GROUP BY table_name
        HAVING SUM(live_tuples) + SUM(dead_tuples) >= 1000
        ORDER BY SUM(dead_tuples) DESC"#
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

async fn fetch_slow_queries(
    db: &DB,
    sort: SlowQuerySort,
) -> windmill_common::error::Result<Option<SlowQueriesInfo>> {
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
            stats_reset: None,
        }));
    }

    // Use raw query since pg_stat_statements may not exist at compile time.
    // ORDER BY column is controlled via an enum (SlowQuerySort) so only safe
    // whitelisted column names reach the query — no SQL injection risk.
    let query = format!(
        r#"SELECT
            LEFT(query, 500),
            calls::bigint,
            total_exec_time::float8,
            mean_exec_time::float8
        FROM pg_stat_statements
        WHERE query NOT LIKE '%pg_stat_statements%'
        ORDER BY {}
        LIMIT 50"#,
        sort.order_by()
    );
    let rows: Vec<SlowQueryRow> = sqlx::query_as::<_, (String, i64, f64, f64)>(&query)
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

    // pg_stat_statements_info exists in PG 14+; tolerate its absence
    let stats_reset: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
            "SELECT stats_reset FROM pg_stat_statements_info",
        )
        .fetch_one(db)
        .await
        .ok()
        .flatten();

    Ok(Some(SlowQueriesInfo {
        queries: rows,
        message: None,
        stats_reset,
    }))
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

    let table_names: Vec<String> = rows.iter().filter_map(|r| r.table_name.clone()).collect();

    if table_names.is_empty() {
        return Ok(vec![]);
    }

    // Batch lookup: single query for all table sizes
    let size_rows = sqlx::query!(
        r#"SELECT
            c.relname as "table_name!",
            pg_total_relation_size(c.oid) as "size_bytes!",
            pg_size_pretty(pg_total_relation_size(c.oid)) as "size_pretty!",
            COALESCE(c.reltuples, 0) as "estimated_rows!"
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public' AND c.relname = ANY($1)"#,
        &table_names
    )
    .fetch_all(db)
    .await?;

    let size_map: std::collections::HashMap<String, _> = size_rows
        .into_iter()
        .map(|s| (s.table_name.clone(), s))
        .collect();

    let mut result = Vec::new();
    for row in rows {
        let table_name = match &row.table_name {
            Some(t) => t.clone(),
            None => continue,
        };
        if let Some(s) = size_map.get(&table_name) {
            result.push(DatatableInfo {
                workspace_id: row.workspace_id,
                name: row.name,
                table_name,
                size_bytes: s.size_bytes,
                size_pretty: s.size_pretty.clone(),
                estimated_rows: s.estimated_rows as f64,
            });
        }
    }

    // Sort by size descending
    result.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    Ok(result)
}
