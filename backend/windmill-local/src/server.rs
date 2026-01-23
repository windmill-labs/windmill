//! HTTP server for local mode
//!
//! Provides a minimal API compatible with Windmill's preview endpoints.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::db::LocalDb;
use crate::error::Result;
use crate::jobs::{
    get_completed_job, push_flow_preview, push_preview, FlowPreviewRequest,
    JobStatus, PreviewRequest, ScriptLang,
};
use crate::worker::Worker;

/// Application state shared across handlers
pub struct AppState {
    pub db: Arc<LocalDb>,
}

/// Local server that runs the API and embedded worker
pub struct LocalServer {
    db: Arc<LocalDb>,
    addr: SocketAddr,
}

impl LocalServer {
    /// Create a new local server
    pub async fn new(addr: SocketAddr) -> Result<Self> {
        let db = Arc::new(LocalDb::in_memory().await?);
        Ok(Self { db, addr })
    }

    /// Create a local server with a file-based database
    pub async fn with_file(addr: SocketAddr, db_path: &str) -> Result<Self> {
        let db = Arc::new(LocalDb::file(db_path).await?);
        Ok(Self { db, addr })
    }

    /// Create a local server connected to a remote Turso database
    pub async fn with_turso(addr: SocketAddr, url: &str, auth_token: &str) -> Result<Self> {
        let db = Arc::new(LocalDb::turso_remote(url, auth_token).await?);
        Ok(Self { db, addr })
    }

    /// Run the server
    pub async fn run(self) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Start the embedded worker
        let worker_db = self.db.clone();
        let worker_handle = tokio::spawn(async move {
            let mut worker = Worker::new(worker_db, shutdown_rx);
            if let Err(e) = worker.run().await {
                tracing::error!("Worker error: {}", e);
            }
        });

        // Build the router
        let state = Arc::new(AppState { db: self.db });
        let app = create_router(state);

        // Run the server
        tracing::info!("Local server listening on {}", self.addr);
        let listener = tokio::net::TcpListener::bind(self.addr).await.unwrap();

        // Handle graceful shutdown
        let shutdown_signal = async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install CTRL+C signal handler");
            tracing::info!("Shutdown signal received");
            shutdown_tx.send(true).ok();
        };

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal)
            .await
            .unwrap();

        // Wait for worker to finish
        worker_handle.await.ok();

        Ok(())
    }
}

/// Create the API router
fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Preview endpoints (mimics windmill-api)
        .route("/api/w/:workspace/jobs/run/preview", post(run_preview))
        .route(
            "/api/w/:workspace/jobs/run_wait_result/preview",
            post(run_wait_result_preview),
        )
        .route(
            "/api/w/:workspace/jobs/run/preview_flow",
            post(run_preview_flow),
        )
        .route(
            "/api/w/:workspace/jobs/run_wait_result/preview_flow",
            post(run_wait_result_preview_flow),
        )
        // Get job result
        .route(
            "/api/w/:workspace/jobs_u/completed/get_result/:job_id",
            get(get_job_result),
        )
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

// === Request/Response Types ===

#[derive(Debug, Deserialize)]
struct PreviewPayload {
    content: String,
    language: String,
    #[serde(default)]
    args: serde_json::Value,
    lock: Option<String>,
    tag: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FlowPreviewPayload {
    value: serde_json::Value,
    #[serde(default)]
    args: serde_json::Value,
    tag: Option<String>,
}

#[derive(Debug, Serialize)]
struct JobCreatedResponse {
    job_id: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

// === Handlers ===

async fn health_check() -> &'static str {
    "OK"
}

/// Run a preview job (async - returns job ID)
async fn run_preview(
    State(state): State<Arc<AppState>>,
    Path(_workspace): Path<String>,
    Json(payload): Json<PreviewPayload>,
) -> impl IntoResponse {
    let lang = match ScriptLang::from_str(&payload.language) {
        Some(l) => l,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Unknown language: {}", payload.language),
                }),
            )
                .into_response();
        }
    };

    let req = PreviewRequest {
        content: payload.content,
        language: lang,
        args: payload.args,
        lock: payload.lock,
        tag: payload.tag,
    };

    match push_preview(&state.db, req).await {
        Ok(job_id) => (
            StatusCode::CREATED,
            Json(JobCreatedResponse {
                job_id: job_id.to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Run a preview job and wait for result
async fn run_wait_result_preview(
    State(state): State<Arc<AppState>>,
    Path(_workspace): Path<String>,
    Json(payload): Json<PreviewPayload>,
) -> impl IntoResponse {
    let lang = match ScriptLang::from_str(&payload.language) {
        Some(l) => l,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Unknown language: {}", payload.language)})),
            )
                .into_response();
        }
    };

    let req = PreviewRequest {
        content: payload.content,
        language: lang,
        args: payload.args,
        lock: payload.lock,
        tag: payload.tag,
    };

    let job_id = match push_preview(&state.db, req).await {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    // Poll for result with timeout
    wait_for_result(&state.db, job_id, Duration::from_secs(60)).await
}

/// Run a flow preview job (async - returns job ID)
async fn run_preview_flow(
    State(state): State<Arc<AppState>>,
    Path(_workspace): Path<String>,
    Json(payload): Json<FlowPreviewPayload>,
) -> impl IntoResponse {
    let req = FlowPreviewRequest {
        value: payload.value,
        args: payload.args,
        tag: payload.tag,
    };

    match push_flow_preview(&state.db, req).await {
        Ok(job_id) => (
            StatusCode::CREATED,
            Json(JobCreatedResponse {
                job_id: job_id.to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Run a flow preview job and wait for result
async fn run_wait_result_preview_flow(
    State(state): State<Arc<AppState>>,
    Path(_workspace): Path<String>,
    Json(payload): Json<FlowPreviewPayload>,
) -> impl IntoResponse {
    let req = FlowPreviewRequest {
        value: payload.value,
        args: payload.args,
        tag: payload.tag,
    };

    let job_id = match push_flow_preview(&state.db, req).await {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };

    // Poll for result with timeout
    wait_for_result(&state.db, job_id, Duration::from_secs(120)).await
}

/// Get the result of a completed job
async fn get_job_result(
    State(state): State<Arc<AppState>>,
    Path((_workspace, job_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let job_id = match Uuid::parse_str(&job_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid job ID"})),
            )
                .into_response();
        }
    };

    match get_completed_job(&state.db, job_id).await {
        Ok(Some(job)) => {
            if job.status == JobStatus::Success {
                (StatusCode::OK, Json(job.result)).into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(job.result)).into_response()
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Job not found or not completed"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Poll for job completion with timeout
async fn wait_for_result(
    db: &LocalDb,
    job_id: Uuid,
    timeout: Duration,
) -> axum::response::Response {
    let start = std::time::Instant::now();
    let fast_poll_duration = Duration::from_secs(2);
    let fast_poll_interval = Duration::from_millis(50);
    let slow_poll_interval = Duration::from_millis(200);

    loop {
        if start.elapsed() > timeout {
            return (
                StatusCode::REQUEST_TIMEOUT,
                Json(serde_json::json!({"error": "Timeout waiting for job result"})),
            )
                .into_response();
        }

        match get_completed_job(db, job_id).await {
            Ok(Some(job)) => {
                if job.status == JobStatus::Success {
                    return (StatusCode::OK, Json(job.result)).into_response();
                } else {
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(job.result)).into_response();
                }
            }
            Ok(None) => {
                // Job not completed yet, keep polling
                let interval = if start.elapsed() < fast_poll_duration {
                    fast_poll_interval
                } else {
                    slow_poll_interval
                };
                tokio::time::sleep(interval).await;
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check() {
        let db = Arc::new(LocalDb::in_memory().await.unwrap());
        let state = Arc::new(AppState { db });
        let app = create_router(state);

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
