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
    static ref HEALTH_STATUS_GAUGE: Option<prometheus::Gauge> =
        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            Some(prometheus::register_gauge!(
                "health_status",
                "Health status: 1=healthy, 0.5=degraded, 0=unhealthy"
            ).unwrap())
        } else {
            None
        };

    static ref HEALTH_DATABASE_HEALTHY: Option<prometheus::Gauge> =
        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            Some(prometheus::register_gauge!(
                "health_database_healthy",
                "Database health: 1=healthy, 0=unhealthy"
            ).unwrap())
        } else {
            None
        };

    static ref HEALTH_WORKERS_ALIVE: Option<prometheus::Gauge> =
        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            Some(prometheus::register_gauge!(
                "health_workers_alive",
                "Number of workers alive"
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

async fn check_database_healthy(db: &DB) -> bool {
    tokio::time::timeout(
        HEALTH_CHECK_TIMEOUT,
        sqlx::query_scalar!("SELECT 1").fetch_one(db),
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false)
}

async fn check_database_detailed(db: &DB) -> DatabaseHealth {
    let start = std::time::Instant::now();
    let healthy = tokio::time::timeout(
        HEALTH_CHECK_TIMEOUT,
        sqlx::query_scalar!("SELECT 1").fetch_one(db),
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false);
    let latency_ms = start.elapsed().as_millis() as i64;

    let pool = PoolStats {
        size: db.size(),
        idle: db.num_idle() as u32,
        max_connections: db.options().get_max_connections(),
    };

    DatabaseHealth { healthy, latency_ms, pool }
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
                checked_at = %status.checked_at,
                "Health check completed"
            );
        }
        HealthStatus::Degraded => {
            tracing::warn!(
                status = "degraded",
                database_healthy = status.database_healthy,
                workers_alive = status.workers_alive,
                checked_at = %status.checked_at,
                "Health check: degraded status (no workers alive)"
            );
        }
        HealthStatus::Unhealthy => {
            tracing::error!(
                status = "unhealthy",
                database_healthy = status.database_healthy,
                workers_alive = status.workers_alive,
                checked_at = %status.checked_at,
                "Health check: unhealthy status"
            );
        }
    }
}

/// Update prometheus metrics for health status
#[cfg(feature = "prometheus")]
fn update_health_metrics(status: &HealthStatusResponse) {
    if let Some(gauge) = HEALTH_STATUS_GAUGE.as_ref() {
        let value = match status.status {
            HealthStatus::Healthy => 1.0,
            HealthStatus::Degraded => 0.5,
            HealthStatus::Unhealthy => 0.0,
        };
        gauge.set(value);
    }

    if let Some(gauge) = HEALTH_DATABASE_HEALTHY.as_ref() {
        gauge.set(if status.database_healthy { 1.0 } else { 0.0 });
    }

    if let Some(gauge) = HEALTH_WORKERS_ALIVE.as_ref() {
        gauge.set(status.workers_alive as f64);
    }
}

/// Perform fresh health check
async fn perform_health_check(db: &DB) -> HealthStatusResponse {
    let checked_at = Utc::now();
    let database_healthy = check_database_healthy(db).await;

    let workers_alive = if database_healthy {
        check_worker_count(db).await
    } else {
        0
    };

    let status = if !database_healthy {
        HealthStatus::Unhealthy
    } else if workers_alive == 0 {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    };

    HealthStatusResponse {
        status,
        checked_at,
        database_healthy,
        workers_alive,
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
    let health_status = perform_health_check(&db).await;

    // Log and update metrics on fresh check
    log_health_status(&health_status);
    #[cfg(feature = "prometheus")]
    update_health_metrics(&health_status);

    // Update cache (clone before acquiring lock to minimize lock duration)
    let cached = CachedHealthStatus {
        status: health_status.clone(),
        cached_at: std::time::Instant::now(),
    };
    {
        let mut cache = STATUS_CACHE.write().await;
        *cache = Some(cached);
    }

    let status_code = if health_status.status == HealthStatus::Unhealthy {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    (status_code, Json(health_status))
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
