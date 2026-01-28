/*
 * Author: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::Extension,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::db::{ApiAuthed, DB};
use windmill_common::min_version::MIN_KEEP_ALIVE_VERSION;
use windmill_common::utils::GIT_VERSION;
use windmill_common::IS_READY;

const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

lazy_static::lazy_static! {
    pub static ref HEALTH_CHECK_SECRET: Arc<RwLock<Option<String>>> =
        Arc::new(RwLock::new(std::env::var("HEALTH_CHECK_SECRET").ok()));
}

/// Simple health endpoint - no DB auth, optional static secret
pub fn public_service() -> Router {
    Router::new().route("/", get(health_simple))
}

/// Detailed health endpoint - requires DB auth
pub fn authed_service() -> Router {
    Router::new().route("/", get(health_detailed))
}

// ============ Simple Health Response ============

#[derive(Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Serialize)]
pub struct SimpleHealthResponse {
    pub status: HealthStatus,
    pub timestamp: DateTime<Utc>,
    pub database_healthy: bool,
    pub workers_alive: i64,
}

// ============ Detailed Health Response ============

#[derive(Serialize)]
pub struct DetailedHealthResponse {
    pub status: HealthStatus,
    pub timestamp: DateTime<Utc>,
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

/// Returns count of alive workers (pinged within last 5 minutes)
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

/// Simple health check - no DB auth, optional static secret
async fn health_simple(
    headers: HeaderMap,
    Extension(db): Extension<DB>,
) -> impl IntoResponse {
    // Check static secret if configured
    let secret = HEALTH_CHECK_SECRET.read().await;
    if let Some(expected_secret) = secret.as_ref() {
        let provided_secret = headers
            .get("X-Health-Secret")
            .and_then(|v| v.to_str().ok());

        if provided_secret != Some(expected_secret.as_str()) {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
                "error": "Invalid or missing X-Health-Secret header"
            }))).into_response();
        }
    }
    drop(secret);

    let timestamp = Utc::now();
    let database_healthy = check_database_healthy(&db).await;

    let workers_alive = if database_healthy {
        check_worker_count(&db).await
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

    let response = SimpleHealthResponse {
        status,
        timestamp,
        database_healthy,
        workers_alive,
    };

    let status_code = if status == HealthStatus::Unhealthy {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    (status_code, Json(response)).into_response()
}

/// Detailed health check - requires DB authentication
async fn health_detailed(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> impl IntoResponse {
    let timestamp = Utc::now();
    let database = check_database_detailed(&db).await;
    let readiness = check_readiness();

    // Short-circuit if database is down
    if !database.healthy {
        let response = DetailedHealthResponse {
            status: HealthStatus::Unhealthy,
            timestamp,
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
        timestamp,
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
