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
use windmill_common::{JOB_RETENTION_SECS_OVERRIDES, JOB_RETENTION_SECS_OVERRIDES_LOADED};

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
    pub pg_superuser_reserved_connections: i64,
    pub status: HealthLevel,
    pub message: String,
    /// Connection sizing guidance derived from the live Windmill fleet.
    pub sizing: ConnectionSizingInfo,
}

#[derive(Serialize)]
pub struct ConnectionSizingInfo {
    /// Live DB-connected worker processes (distinct worker_instance pinged recently).
    pub live_worker_instances: i64,
    /// Live individual DB-connected workers across all instances.
    pub live_workers: i64,
    /// Live agent workers (HTTP-only, hold no postgres connections; excluded from the estimate).
    pub live_agent_workers: i64,
    /// Effective per-server pool ceiling: DATABASE_CONNECTIONS if set, else DEFAULT_MAX_CONNECTIONS_SERVER.
    pub server_pool_size: i64,
    /// Effective per-worker-instance pool ceiling (single-worker baseline; grows +1 per extra worker unless DATABASE_CONNECTIONS is set).
    pub worker_pool_size: i64,
    /// The DATABASE_CONNECTIONS override, if this server has one set (caps every process's pool).
    pub database_connections_override: Option<i64>,
    /// Estimated peak connections opened by all live worker instances.
    pub estimated_worker_connections: i64,
    /// Recommended max_connections floor (workers + one server + headroom).
    pub recommended_max_connections: i64,
    /// Per-additional-server increment to add to the recommendation.
    pub per_server_increment: i64,
    /// Human-readable sizing explanation.
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
    // Per-workspace retention overrides (EE) make "oldest completed job vs the instance retention"
    // wrong as a single global signal: each override workspace has its own effective window, so its
    // intentionally-retained jobs must be judged against that window — not the instance one. We
    // therefore compute the ratio per scope and report the worst:
    //  - global scope: oldest job across all non-override workspaces vs the instance retention;
    //  - each override workspace with a *positive* window: its own oldest job vs its own window;
    //  - keep-forever (0) overrides: excluded entirely — their jobs are retained forever by design,
    //    so there is no window to fall behind on.
    // The majority (no-override) path keeps the original index-driven `MIN(completed_at)` with no
    // performance change; the override paths use the completed_at / (workspace_id, completed_at)
    // indexes and only run when overrides exist.
    let overrides = JOB_RETENTION_SECS_OVERRIDES.load_full();
    let overrides_active = !overrides.is_empty()
        && JOB_RETENTION_SECS_OVERRIDES_LOADED.load(std::sync::atomic::Ordering::Relaxed);

    // `true_oldest` is the real table minimum across every workspace — reported verbatim in the
    // public `oldest_completed_at` field so the UI's "Oldest job" label stays honest. `global_oldest`
    // excludes override workspaces and drives only the global health ratio (override workspaces are
    // judged against their own window below).
    type OptTs = Option<chrono::DateTime<chrono::Utc>>;
    let (true_oldest, global_oldest, total): (OptTs, OptTs, i64) = if !overrides_active {
        let r = sqlx::query!(
            "SELECT MIN(completed_at) as oldest, COUNT(*) as total FROM v2_job_completed"
        )
        .fetch_one(db)
        .await?;
        (r.oldest, r.oldest, r.total.unwrap_or(0))
    } else {
        // `MIN(...) WHERE workspace_id <> ALL(...)` is still driven by the completed_at index
        // (ascending scan, early-stop at the first non-override row); the plain `MIN(...)` is an
        // index-only scan. Both are cheap.
        let override_ids: Vec<String> = overrides.keys().cloned().collect();
        let r = sqlx::query!(
            "SELECT
                (SELECT MIN(completed_at) FROM v2_job_completed) as true_oldest,
                (SELECT MIN(completed_at) FROM v2_job_completed
                 WHERE workspace_id <> ALL($1::text[])) as global_oldest,
                (SELECT COUNT(*) FROM v2_job_completed) as total",
            &override_ids,
        )
        .fetch_one(db)
        .await?;
        (r.true_oldest, r.global_oldest, r.total.unwrap_or(0))
    };

    let retention_row =
        sqlx::query!("SELECT value FROM global_settings WHERE name = 'retention_period_secs'")
            .fetch_optional(db)
            .await?;

    let retention_period_secs: Option<i64> =
        retention_row.map(|r| r.value).and_then(|v| v.as_i64());

    // Oldest job per positive-window override workspace (one grouped seek on the
    // `(workspace_id, completed_at)` index; only workspaces that actually have rows come back).
    let positive_override_ids: Vec<String> = if overrides_active {
        overrides
            .iter()
            .filter(|(_, &secs)| secs > 0)
            .map(|(ws, _)| ws.clone())
            .collect()
    } else {
        Vec::new()
    };
    let mut per_workspace_oldest: Vec<(String, chrono::DateTime<chrono::Utc>)> = Vec::new();
    if !positive_override_ids.is_empty() {
        let rows = sqlx::query!(
            "SELECT workspace_id as \"workspace_id!\", MIN(completed_at) as oldest
             FROM v2_job_completed
             WHERE workspace_id = ANY($1::text[])
             GROUP BY workspace_id",
            &positive_override_ids,
        )
        .fetch_all(db)
        .await?;
        for r in rows {
            if let Some(oldest) = r.oldest {
                per_workspace_oldest.push((r.workspace_id, oldest));
            }
        }
    }

    // Evaluate each scope independently and report the WORST. Each scope contributes a candidate
    // (severity, level, message); the max-severity candidate wins. This keeps the global scope's
    // "no retention configured" warning visible even when a healthy override would otherwise mask it.
    let now = chrono::Utc::now();
    let ratio_status = |scope: String, ratio: f64| -> (u8, HealthLevel, String) {
        if ratio <= 2.0 {
            (
                0,
                HealthLevel::Green,
                format!("{scope} is {ratio:.1}x the retention period. Cleanup is keeping up."),
            )
        } else if ratio <= 5.0 {
            (
                1,
                HealthLevel::Yellow,
                format!(
                    "{scope} is {ratio:.1}x the retention period. Cleanup may be falling behind."
                ),
            )
        } else {
            (2, HealthLevel::Red, format!("{scope} is {ratio:.1}x the retention period. Consider reducing retention or investigating cleanup."))
        }
    };

    let mut candidates: Vec<(u8, HealthLevel, String)> = Vec::new();
    // Global scope: judged against the instance retention, or flagged when non-override jobs exist
    // (`global_oldest` is `Some`) but no positive instance retention is configured. A `0` instance
    // retention means keep-forever globally, so it contributes no candidate.
    match (global_oldest, retention_period_secs) {
        (Some(oldest_ts), Some(retention_secs)) if retention_secs > 0 => {
            let ratio = (now - oldest_ts).num_seconds() as f64 / retention_secs as f64;
            candidates.push(ratio_status("Oldest job".to_string(), ratio));
        }
        (Some(_), None) => candidates.push((
            1,
            HealthLevel::Yellow,
            "No retention_period_secs configured. Old jobs will accumulate.".to_string(),
        )),
        _ => {}
    }
    // Each positive-window override, judged against its own window.
    for (ws, oldest_ts) in &per_workspace_oldest {
        if let Some(&window) = overrides.get(ws) {
            if window > 0 {
                let ratio = (now - *oldest_ts).num_seconds() as f64 / window as f64;
                candidates.push(ratio_status(format!("Workspace {ws} oldest job"), ratio));
            }
        }
    }

    let (status, message) = if let Some((_, level, message)) = candidates
        .into_iter()
        .max_by_key(|(severity, _, _)| *severity)
    {
        (level, message)
    } else if total > 0 {
        // Jobs exist but none produced a candidate: every completed job lives in a keep-forever
        // scope (instance or override), so it is retained by design rather than overdue.
        (
            HealthLevel::Green,
            "Completed jobs are within their configured retention windows.".to_string(),
        )
    } else {
        (HealthLevel::Green, "No completed jobs found.".to_string())
    };

    Ok(JobRetentionInfo {
        oldest_completed_at: true_oldest,
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

    let reserved = sqlx::query_scalar!(
        r#"SELECT setting::bigint as "v!" FROM pg_settings WHERE name = 'superuser_reserved_connections'"#
    )
    .fetch_one(db)
    .await
    .unwrap_or(0);

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

    // Live Windmill worker fleet: each worker pings worker_ping every ~5s; a
    // window of 30s tolerates a missed ping without counting dead workers.
    // Agent workers (name prefix "ag-") talk to the API over HTTP and hold no
    // postgres pool, so they're excluded from the connection estimate and only
    // reported for context; regular DB-connected workers use the "wk-" prefix.
    let db_worker_pattern = format!("{}-%", windmill_common::utils::WORKER_NAME_PREFIX);
    let agent_worker_pattern = format!("{}-%", windmill_common::utils::AGENT_WORKER_NAME_PREFIX);
    let fleet = sqlx::query!(
        r#"SELECT
            COUNT(*) FILTER (WHERE worker LIKE $1) as "live_workers!",
            COUNT(DISTINCT worker_instance) FILTER (WHERE worker LIKE $1) as "live_instances!",
            COUNT(*) FILTER (WHERE worker LIKE $2) as "live_agent_workers!"
        FROM worker_ping
        WHERE ping_at > now() - interval '30 seconds'"#,
        db_worker_pattern,
        agent_worker_pattern,
    )
    .fetch_one(db)
    .await?;

    // Matches how db_connect.rs reads it: when DATABASE_CONNECTIONS is set it caps
    // every process's pool (server, indexer, worker) regardless of worker count.
    let database_connections_override = std::env::var("DATABASE_CONNECTIONS")
        .ok()
        .and_then(|n| n.parse::<i64>().ok())
        .filter(|n| *n > 0);

    let sizing = compute_connection_sizing(
        fleet.live_workers,
        fleet.live_instances,
        fleet.live_agent_workers,
        reserved,
        database_connections_override,
    );

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
        pg_superuser_reserved_connections: reserved,
        status,
        message,
        sizing,
    })
}

/// Estimate how many postgres connections the live Windmill fleet can open and
/// derive a recommended `max_connections` floor.
///
/// Pool sizing mirrors `db_connect.rs`: when `DATABASE_CONNECTIONS` is set it
/// caps *every* process's pool (server, indexer, worker) at that value, so each
/// worker instance opens up to that many connections. Otherwise each worker
/// instance shares a pool of `DEFAULT_MAX_CONNECTIONS_WORKER + (workers - 1)`
/// (fleet ceiling `(worker_pool - 1) * instances + workers`) and each server
/// opens up to `DEFAULT_MAX_CONNECTIONS_SERVER`.
///
/// `database_connections_override` is this server's `DATABASE_CONNECTIONS`, our
/// best proxy for the fleet's config. Servers do not ping `worker_ping`, so we
/// can't count them — the recommendation assumes one server and exposes the
/// per-server increment so the operator can add capacity for the rest.
///
/// `live_agent_workers` is reported for context only: agent workers reach the
/// API over HTTP and open no postgres connections, so they never contribute to
/// the estimate.
fn compute_connection_sizing(
    live_workers: i64,
    live_instances: i64,
    live_agent_workers: i64,
    reserved: i64,
    database_connections_override: Option<i64>,
) -> ConnectionSizingInfo {
    // Never recommend below this floor: postgres defaults to 100 and headroom
    // for growth/bursts/psql is cheap, so 200 is a safe baseline for any fleet.
    const MIN_RECOMMENDED_MAX_CONNECTIONS: i64 = 200;

    let default_server_pool = windmill_common::DEFAULT_MAX_CONNECTIONS_SERVER as i64;
    let default_worker_pool = windmill_common::DEFAULT_MAX_CONNECTIONS_WORKER as i64;

    let (server_pool, worker_pool_size, estimated_worker_connections) =
        match database_connections_override {
            // Override caps every process identically; per-instance pool is the override.
            Some(n) => (n, n, n * live_instances),
            None => (
                default_server_pool,
                default_worker_pool,
                (default_worker_pool - 1) * live_instances + live_workers,
            ),
        };

    // Workers + one server, plus 20% headroom and the superuser reserve, so the
    // recommendation leaves room for psql/monitoring sessions and bursts, then
    // floored at MIN_RECOMMENDED_MAX_CONNECTIONS.
    let base = estimated_worker_connections + server_pool;
    let recommended = ((((base as f64) * 1.20).ceil() as i64) + reserved.max(3))
        .max(MIN_RECOMMENDED_MAX_CONNECTIONS);

    let pool_source = if database_connections_override.is_some() {
        format!("DATABASE_CONNECTIONS={server_pool}")
    } else {
        "defaults, configurable via DATABASE_CONNECTIONS".to_string()
    };

    let message = if live_instances == 0 {
        format!(
            "No live workers detected. Each Windmill server and worker instance opens up to {server_pool} connections ({pool_source}). Size max_connections as (servers + worker instances) × {server_pool} + ~20% headroom, and at least {MIN_RECOMMENDED_MAX_CONNECTIONS}."
        )
    } else {
        let per_instance = if database_connections_override.is_some() {
            format!("each instance up to {worker_pool_size}")
        } else {
            format!("each instance up to {worker_pool_size}, +1 per extra worker")
        };
        let agent_note = if live_agent_workers > 0 {
            format!(
                " ({live_agent_workers} agent worker(s) excluded — they use HTTP, not postgres connections.)"
            )
        } else {
            String::new()
        };
        format!(
            "{live_workers} live worker(s) across {live_instances} instance(s) can open up to ~{estimated_worker_connections} connections ({per_instance}; {pool_source}). Each Windmill server adds up to {server_pool}. Recommended max_connections ≥ {recommended} for a single server; add {server_pool} per additional server.{agent_note}"
        )
    };

    ConnectionSizingInfo {
        live_worker_instances: live_instances,
        live_workers,
        live_agent_workers,
        server_pool_size: server_pool,
        worker_pool_size,
        database_connections_override,
        estimated_worker_connections,
        recommended_max_connections: recommended,
        per_server_increment: server_pool,
        message,
    }
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

#[cfg(test)]
mod tests {
    use super::compute_connection_sizing;

    #[test]
    fn no_workers_reports_defaults_and_floors_at_200() {
        let s = compute_connection_sizing(0, 0, 0, 3, None);
        assert_eq!(s.live_workers, 0);
        assert_eq!(s.live_worker_instances, 0);
        assert_eq!(s.live_agent_workers, 0);
        assert_eq!(s.estimated_worker_connections, 0);
        assert_eq!(s.server_pool_size, 50);
        assert_eq!(s.worker_pool_size, 5);
        assert_eq!(s.database_connections_override, None);
        assert_eq!(s.per_server_increment, 50);
        // ceil((0 + 50) * 1.20) + 3 = 63, floored up to 200.
        assert_eq!(s.recommended_max_connections, 200);
        assert!(s.message.contains("No live workers"));
    }

    #[test]
    fn single_worker_single_instance_floors_at_200() {
        let s = compute_connection_sizing(1, 1, 0, 3, None);
        // (5 - 1) * 1 instance + 1 worker = 5
        assert_eq!(s.estimated_worker_connections, 5);
        // ceil((5 + 50) * 1.20) + 3 = 66 + 3 = 69, floored up to 200.
        assert_eq!(s.recommended_max_connections, 200);
    }

    #[test]
    fn multi_worker_instances_sum_per_instance_pools() {
        // Two instances, 5 workers total: pools are (4 + w_i) summed = 4*2 + 5 = 13.
        let s = compute_connection_sizing(5, 2, 0, 3, None);
        assert_eq!(s.estimated_worker_connections, 13);
    }

    #[test]
    fn large_fleet_exceeds_floor_with_margin_and_reserve() {
        // 300 workers across 10 instances: (5-1)*10 + 300 = 340 worker connections.
        let s = compute_connection_sizing(300, 10, 0, 3, None);
        assert_eq!(s.estimated_worker_connections, 340);
        // ceil((340 + 50) * 1.20) + 3 = ceil(468.0) + 3 = 471, above the 200 floor.
        assert_eq!(s.recommended_max_connections, 471);
    }

    #[test]
    fn reserved_above_default_widens_recommendation() {
        // Big enough fleet that the floor doesn't mask the reserved contribution.
        let base = compute_connection_sizing(300, 10, 0, 3, None);
        let high = compute_connection_sizing(300, 10, 0, 20, None);
        assert_eq!(
            high.recommended_max_connections - base.recommended_max_connections,
            17
        );
    }

    #[test]
    fn database_connections_override_caps_every_process() {
        // DATABASE_CONNECTIONS=100 means each instance opens up to 100, not the
        // default 5-based estimate. 5 instances -> 500 worker connections.
        let s = compute_connection_sizing(20, 5, 0, 3, Some(100));
        assert_eq!(s.server_pool_size, 100);
        assert_eq!(s.worker_pool_size, 100);
        assert_eq!(s.database_connections_override, Some(100));
        assert_eq!(s.estimated_worker_connections, 500);
        assert_eq!(s.per_server_increment, 100);
        // ceil((500 + 100) * 1.20) + 3 = 720 + 3 = 723.
        assert_eq!(s.recommended_max_connections, 723);
        assert!(s.message.contains("DATABASE_CONNECTIONS=100"));
    }

    #[test]
    fn agent_workers_are_excluded_from_the_estimate() {
        // 50 agent workers alongside 2 DB workers/2 instances: only the DB
        // workers count toward connections; the agent count is reported.
        let s = compute_connection_sizing(2, 2, 50, 3, None);
        assert_eq!(s.live_agent_workers, 50);
        // (5 - 1) * 2 + 2 = 10, agent workers contribute nothing.
        assert_eq!(s.estimated_worker_connections, 10);
        let without_agents = compute_connection_sizing(2, 2, 0, 3, None);
        assert_eq!(
            s.recommended_max_connections,
            without_agents.recommended_max_connections
        );
        assert!(s.message.contains("50 agent worker(s) excluded"));
    }
}
