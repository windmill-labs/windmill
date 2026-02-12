/*
 * Author: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::{Extension, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::db::{ApiAuthed, DB};
use windmill_common::min_version::MIN_KEEP_ALIVE_VERSION;
use windmill_common::utils::GIT_VERSION;
use windmill_common::IS_READY;

#[cfg(feature = "prometheus")]
use windmill_common::METRICS_ENABLED;

/// Fixed 5 second cache TTL for health status
const HEALTH_CACHE_TTL: Duration = Duration::from_secs(5);

lazy_static::lazy_static! {
    static ref STATUS_CACHE: Arc<RwLock<Option<CachedHealthStatus>>> = Arc::new(RwLock::new(None));

    /// Environment variable to silence health endpoint logs
    static ref SILENCE_HEALTH_LOGS: bool = std::env::var("SILENCE_HEALTH_LOGS")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);
}

#[cfg(feature = "prometheus")]
lazy_static::lazy_static! {
    /// Health status phase gauge with labels: healthy, degraded, unhealthy
    /// Only one label will be 1 at a time, others will be 0
    static ref HEALTH_STATUS_PHASE: Option<prometheus::IntGaugeVec> =
        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            Some(prometheus::register_int_gauge_vec!(
                "health_status_phase",
                "Health status phase (1 = current state, 0 = not current state)",
                &["phase"]
            ).unwrap())
        } else {
            None
        };

    /// Database latency in milliseconds
    static ref HEALTH_DATABASE_LATENCY: Option<prometheus::Gauge> =
        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            Some(prometheus::register_gauge!(
                "health_database_latency_ms",
                "Database query latency in milliseconds"
            ).unwrap())
        } else {
            None
        };

    /// Database unresponsive flag (1 = unresponsive, 0 = responsive)
    static ref HEALTH_DATABASE_UNRESPONSIVE: Option<prometheus::IntGauge> =
        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            Some(prometheus::register_int_gauge!(
                "health_database_unresponsive",
                "Database unresponsive flag (1 = unresponsive, 0 = responsive)"
            ).unwrap())
        } else {
            None
        };

    /// Database connection pool size
    static ref HEALTH_DATABASE_POOL_SIZE: Option<prometheus::IntGauge> =
        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            Some(prometheus::register_int_gauge!(
                "health_database_pool_size",
                "Current number of connections in the database pool"
            ).unwrap())
        } else {
            None
        };

    /// Database connection pool idle connections
    static ref HEALTH_DATABASE_POOL_IDLE: Option<prometheus::IntGauge> =
        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            Some(prometheus::register_int_gauge!(
                "health_database_pool_idle",
                "Number of idle connections in the database pool"
            ).unwrap())
        } else {
            None
        };

    /// Database connection pool max connections
    static ref HEALTH_DATABASE_POOL_MAX: Option<prometheus::IntGauge> =
        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            Some(prometheus::register_int_gauge!(
                "health_database_pool_max",
                "Maximum connections allowed in the database pool"
            ).unwrap())
        } else {
            None
        };
}

#[derive(Clone)]
struct CachedHealthStatus {
    status: HealthStatusResponse,
    cached_at: std::time::Instant,
}

/// Query parameters for status endpoint
#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    /// Force a fresh check, bypassing the cache
    #[serde(default)]
    force: bool,
}

/// Status endpoint - cached health status (unauthenticated)
pub fn status_service() -> Router {
    Router::new().route("/", get(health_status))
}

/// Detailed health endpoint - requires DB auth (always fresh)
pub fn detailed_service() -> Router {
    Router::new().route("/", get(health_detailed))
}

// ============ Response Types ============

#[derive(Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Serialize, Clone)]
pub struct HealthStatusResponse {
    pub status: HealthStatus,
    pub checked_at: DateTime<Utc>,
    pub database_healthy: bool,
    pub workers_alive: i64,
}

#[derive(Serialize)]
pub struct DetailedHealthResponse {
    pub status: HealthStatus,
    pub checked_at: DateTime<Utc>,
    pub version: String,
    pub checks: HealthChecks,
}

#[derive(Serialize)]
pub struct HealthChecks {
    pub database: DatabaseHealth,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workers: Option<WorkersHealth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue: Option<QueueHealth>,
    pub readiness: ReadinessHealth,
}

#[derive(Serialize)]
pub struct DatabaseHealth {
    pub healthy: bool,
    pub latency_ms: i64,
    pub pool: PoolStats,
}

#[derive(Serialize)]
pub struct PoolStats {
    pub size: u32,
    pub idle: u32,
    pub max_connections: u32,
}

#[derive(Serialize)]
pub struct WorkersHealth {
    pub healthy: bool,
    pub active_count: i64,
    pub worker_groups: Vec<String>,
    pub min_version: String,
    pub versions: Vec<String>,
}

#[derive(Serialize)]
pub struct QueueHealth {
    pub pending_jobs: u64,
    pub running_jobs: u64,
}

#[derive(Serialize)]
pub struct ReadinessHealth {
    pub healthy: bool,
}

// ============ Check Functions ============

const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Result of a database health check including latency
struct DatabaseCheckResult {
    healthy: bool,
    latency_ms: i64,
}

async fn check_database_with_latency(db: &DB) -> DatabaseCheckResult {
    let start = std::time::Instant::now();
    let healthy = tokio::time::timeout(
        HEALTH_CHECK_TIMEOUT,
        sqlx::query_scalar!("SELECT 1").fetch_one(db),
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false);
    let latency_ms = start.elapsed().as_millis() as i64;

    DatabaseCheckResult { healthy, latency_ms }
}

fn get_pool_stats(db: &DB) -> PoolStats {
    PoolStats {
        size: db.size(),
        idle: db.num_idle() as u32,
        max_connections: db.options().get_max_connections(),
    }
}

async fn check_database_detailed(db: &DB) -> DatabaseHealth {
    let check = check_database_with_latency(db).await;
    let pool = get_pool_stats(db);

    DatabaseHealth {
        healthy: check.healthy,
        latency_ms: check.latency_ms,
        pool,
    }
}

async fn check_worker_count(db: &DB) -> i64 {
    sqlx::query_scalar!(
        "SELECT COUNT(*) FROM worker_ping WHERE ping_at > now() - interval '5 minutes'"
    )
    .fetch_one(db)
    .await
    .unwrap_or(Some(0))
    .unwrap_or(0)
}

async fn check_workers_detailed(db: &DB) -> WorkersHealth {
    let workers = sqlx::query!(
        r#"
        SELECT
            worker_group,
            wm_version
        FROM worker_ping
        WHERE ping_at > now() - interval '5 minutes'
        "#
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let active_count = workers.len() as i64;

    let worker_groups: Vec<String> = workers
        .iter()
        .map(|w| w.worker_group.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let versions: Vec<String> = workers
        .iter()
        .map(|w| w.wm_version.clone())
        .filter(|v| !v.is_empty())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let min_version = format!(
        "v{}.{}.{}",
        MIN_KEEP_ALIVE_VERSION.0, MIN_KEEP_ALIVE_VERSION.1, MIN_KEEP_ALIVE_VERSION.2
    );

    let healthy = active_count > 0;

    WorkersHealth {
        healthy,
        active_count,
        worker_groups,
        min_version,
        versions,
    }
}

async fn check_queue(db: &DB) -> QueueHealth {
    let pending_counts = windmill_common::queue::get_queue_counts(db).await;
    let running_counts = windmill_common::queue::get_queue_running_counts(db).await;

    let pending_jobs: u64 = pending_counts.values().map(|&v| v as u64).sum();
    let running_jobs: u64 = running_counts.values().map(|&v| v as u64).sum();

    QueueHealth { pending_jobs, running_jobs }
}

fn check_readiness() -> ReadinessHealth {
    let healthy = IS_READY.load(std::sync::atomic::Ordering::Relaxed);
    ReadinessHealth { healthy }
}

#[cfg(feature = "enterprise")]
fn get_version() -> String {
    format!("EE {GIT_VERSION}")
}

#[cfg(not(feature = "enterprise"))]
fn get_version() -> String {
    format!("CE {GIT_VERSION}")
}

// ============ Background Loop ============

/// Spawn a background task that performs a health check every 10 seconds.
/// Updates the cache and prometheus metrics continuously.
pub fn start_health_check_loop(
    db: DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    tracing::info!("health check loop shutting down");
                    break;
                }
                _ = interval.tick() => {
                    let result = perform_health_check(&db).await;

                    log_health_status(&result.response);
                    #[cfg(feature = "prometheus")]
                    update_health_metrics(&result.metrics_data);

                    let cached = CachedHealthStatus {
                        status: result.response,
                        cached_at: std::time::Instant::now(),
                    };
                    *STATUS_CACHE.write().await = Some(cached);
                }
            }
        }
    });
}

// ============ Handlers ============

/// Log health status based on severity
fn log_health_status(status: &HealthStatusResponse) {
    if *SILENCE_HEALTH_LOGS {
        return;
    }

    match status.status {
        HealthStatus::Healthy => {
            tracing::info!(
                status = "healthy",
                database_healthy = status.database_healthy,
                workers_alive = status.workers_alive,
                "health check completed"
            );
        }
        HealthStatus::Degraded => {
            tracing::warn!(
                status = "degraded",
                database_healthy = status.database_healthy,
                workers_alive = status.workers_alive,
                "health check: degraded status (no workers alive)"
            );
        }
        HealthStatus::Unhealthy => {
            tracing::error!(
                status = "unhealthy",
                database_healthy = status.database_healthy,
                workers_alive = status.workers_alive,
                "health check: unhealthy status"
            );
        }
    }
}

/// Data needed for prometheus metrics (internal, not serialized)
#[cfg(feature = "prometheus")]
struct HealthMetricsData {
    status: HealthStatus,
    database_healthy: bool,
    database_latency_ms: i64,
    pool_size: u32,
    pool_idle: u32,
    pool_max: u32,
}

/// Update prometheus metrics for health status
#[cfg(feature = "prometheus")]
fn update_health_metrics(data: &HealthMetricsData) {
    // Update health status phase (only one label is 1, others are 0)
    if let Some(gauge_vec) = HEALTH_STATUS_PHASE.as_ref() {
        let (healthy, degraded, unhealthy) = match data.status {
            HealthStatus::Healthy => (1, 0, 0),
            HealthStatus::Degraded => (0, 1, 0),
            HealthStatus::Unhealthy => (0, 0, 1),
        };
        gauge_vec.with_label_values(&["healthy"]).set(healthy);
        gauge_vec.with_label_values(&["degraded"]).set(degraded);
        gauge_vec.with_label_values(&["unhealthy"]).set(unhealthy);
    }

    // Database latency
    if let Some(gauge) = HEALTH_DATABASE_LATENCY.as_ref() {
        gauge.set(data.database_latency_ms as f64);
    }

    // Database unresponsive flag
    if let Some(gauge) = HEALTH_DATABASE_UNRESPONSIVE.as_ref() {
        gauge.set(if data.database_healthy { 0 } else { 1 });
    }

    // Pool metrics
    if let Some(gauge) = HEALTH_DATABASE_POOL_SIZE.as_ref() {
        gauge.set(data.pool_size as i64);
    }
    if let Some(gauge) = HEALTH_DATABASE_POOL_IDLE.as_ref() {
        gauge.set(data.pool_idle as i64);
    }
    if let Some(gauge) = HEALTH_DATABASE_POOL_MAX.as_ref() {
        gauge.set(data.pool_max as i64);
    }
}

/// Result of perform_health_check including data needed for metrics
struct HealthCheckResult {
    response: HealthStatusResponse,
    #[cfg(feature = "prometheus")]
    metrics_data: HealthMetricsData,
}

/// Perform fresh health check
async fn perform_health_check(db: &DB) -> HealthCheckResult {
    let checked_at = Utc::now();
    let db_check = check_database_with_latency(db).await;

    let workers_alive = if db_check.healthy {
        check_worker_count(db).await
    } else {
        0
    };

    let status = if !db_check.healthy {
        HealthStatus::Unhealthy
    } else if workers_alive == 0 {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    };

    let response = HealthStatusResponse {
        status,
        checked_at,
        database_healthy: db_check.healthy,
        workers_alive,
    };

    #[cfg(feature = "prometheus")]
    let metrics_data = {
        let pool_stats = get_pool_stats(db);
        HealthMetricsData {
            status,
            database_healthy: db_check.healthy,
            database_latency_ms: db_check.latency_ms,
            pool_size: pool_stats.size,
            pool_idle: pool_stats.idle,
            pool_max: pool_stats.max_connections,
        }
    };

    HealthCheckResult {
        response,
        #[cfg(feature = "prometheus")]
        metrics_data,
    }
}

/// Status check - cached DB/worker status with optional force refresh
async fn health_status(
    Extension(db): Extension<DB>,
    Query(query): Query<StatusQuery>,
) -> impl IntoResponse {
    // Check cache (unless force=true)
    if !query.force {
        let cache = STATUS_CACHE.read().await;
        if let Some(cached) = cache.as_ref() {
            if cached.cached_at.elapsed() < HEALTH_CACHE_TTL {
                let status_code = if cached.status.status == HealthStatus::Unhealthy {
                    StatusCode::SERVICE_UNAVAILABLE
                } else {
                    StatusCode::OK
                };
                return (status_code, Json(cached.status.clone()));
            }
        }
    }

    // Cache miss, expired, or force=true - fetch fresh data
    let health_check_result = perform_health_check(&db).await;

    // Log and update metrics on fresh check
    log_health_status(&health_check_result.response);
    #[cfg(feature = "prometheus")]
    update_health_metrics(&health_check_result.metrics_data);

    // Update cache (clone before acquiring lock to minimize lock duration)
    let cached = CachedHealthStatus {
        status: health_check_result.response.clone(),
        cached_at: std::time::Instant::now(),
    };
    {
        let mut cache = STATUS_CACHE.write().await;
        *cache = Some(cached);
    }

    let status_code = if health_check_result.response.status == HealthStatus::Unhealthy {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    (status_code, Json(health_check_result.response))
}

/// Detailed health check - requires DB authentication (always fresh, no caching)
async fn health_detailed(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> impl IntoResponse {
    let checked_at = Utc::now();
    let database = check_database_detailed(&db).await;
    let readiness = check_readiness();

    // Short-circuit if database is down
    if !database.healthy {
        let response = DetailedHealthResponse {
            status: HealthStatus::Unhealthy,
            checked_at,
            version: get_version(),
            checks: HealthChecks {
                database,
                workers: None,
                queue: None,
                readiness,
            },
        };
        return (StatusCode::SERVICE_UNAVAILABLE, Json(response));
    }

    let workers = check_workers_detailed(&db).await;
    let queue = check_queue(&db).await;

    let status = if !workers.healthy {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    };

    let response = DetailedHealthResponse {
        status,
        checked_at,
        version: get_version(),
        checks: HealthChecks {
            database,
            workers: Some(workers),
            queue: Some(queue),
            readiness,
        },
    };

    let status_code = if status == HealthStatus::Unhealthy {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    (status_code, Json(response))
}
