use std::future::IntoFuture;
use std::sync::Arc;

use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use tokio::net::TcpListener;
use uuid::Uuid;
use windmill_sandbox::{ExecRequest, ExecResult, SandboxConfig, SandboxManager};

struct EmbeddedSandboxState {
    manager: SandboxManager,
}

pub struct EmbeddedSandboxHandle {
    port: u16,
    state: Arc<EmbeddedSandboxState>,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl EmbeddedSandboxHandle {
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub async fn shutdown(self) {
        self.state.manager.cleanup_all().await;
        let _ = self.shutdown_tx.send(());
    }
}

pub async fn start_embedded_sandbox_server(
    base_dir: String,
) -> windmill_common::error::Result<EmbeddedSandboxHandle> {
    let manager = SandboxManager::new(base_dir);
    let state = Arc::new(EmbeddedSandboxState { manager });

    let app = Router::new()
        .route("/sandbox/create", post(handle_create))
        .route("/sandbox/:id/exec", post(handle_exec))
        .route("/sandbox/:id/suspend", post(handle_suspend))
        .route("/sandbox/:id/resume", post(handle_resume))
        .route("/sandbox/:id/terminate", post(handle_terminate))
        .route("/sandbox/:id/status", get(handle_status))
        .route("/sandbox/:id/write_file", post(handle_write_file))
        .route("/sandbox/:id/read_file", get(handle_read_file))
        .route("/sandboxes", get(handle_list))
        .layer(Extension(state.clone()));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| {
            windmill_common::error::Error::InternalErr(format!(
                "Failed to bind embedded sandbox server: {e}"
            ))
        })?;
    let port = listener.local_addr().unwrap().port();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        let server = axum::serve(listener, app);
        tokio::select! {
            result = server.into_future() => {
                if let Err(e) = result {
                    tracing::error!("Embedded sandbox server error: {e}");
                }
            }
            _ = shutdown_rx => {
                tracing::debug!("Embedded sandbox server shutting down");
            }
        }
    });

    tracing::info!("Embedded sandbox server started on port {port}");

    Ok(EmbeddedSandboxHandle {
        port,
        state,
        shutdown_tx,
    })
}

async fn handle_create(
    Extension(state): Extension<Arc<EmbeddedSandboxState>>,
    Json(config): Json<SandboxConfig>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    match state.manager.create_sandbox(config).await {
        Ok((id, status)) => Ok(Json(serde_json::json!({
            "id": id,
            "status": status,
        }))),
        Err(e) => {
            tracing::error!("Failed to create sandbox: {e}");
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn handle_exec(
    Extension(state): Extension<Arc<EmbeddedSandboxState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<ExecRequest>,
) -> Result<Json<ExecResult>, axum::http::StatusCode> {
    match state.manager.exec(id, request).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => {
            tracing::error!("Failed to exec in sandbox {id}: {e}");
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn handle_suspend(
    Extension(state): Extension<Arc<EmbeddedSandboxState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    match state.manager.suspend(id).await {
        Ok(status) => Ok(Json(serde_json::json!({ "status": status }))),
        Err(e) => {
            tracing::error!("Failed to suspend sandbox {id}: {e}");
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn handle_resume(
    Extension(state): Extension<Arc<EmbeddedSandboxState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    match state.manager.resume(id).await {
        Ok(status) => Ok(Json(serde_json::json!({ "status": status }))),
        Err(e) => {
            tracing::error!("Failed to resume sandbox {id}: {e}");
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn handle_terminate(
    Extension(state): Extension<Arc<EmbeddedSandboxState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    match state.manager.terminate(id).await {
        Ok(status) => Ok(Json(serde_json::json!({ "status": status }))),
        Err(e) => {
            tracing::error!("Failed to terminate sandbox {id}: {e}");
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn handle_status(
    Extension(state): Extension<Arc<EmbeddedSandboxState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    match state.manager.status(id).await {
        Ok(status) => Ok(Json(serde_json::json!({ "id": id, "status": status }))),
        Err(e) => {
            tracing::error!("Failed to get sandbox {id} status: {e}");
            Err(axum::http::StatusCode::NOT_FOUND)
        }
    }
}

#[derive(Deserialize)]
struct WriteFileBody {
    path: String,
    content: String,
}

async fn handle_write_file(
    Extension(state): Extension<Arc<EmbeddedSandboxState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<WriteFileBody>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    match state
        .manager
        .write_file(id, &body.path, body.content.as_bytes())
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({ "ok": true }))),
        Err(e) => {
            tracing::error!("Failed to write file to sandbox {id}: {e}");
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Deserialize)]
struct ReadFileQuery {
    path: String,
}

async fn handle_read_file(
    Extension(state): Extension<Arc<EmbeddedSandboxState>>,
    Path(id): Path<Uuid>,
    Query(query): Query<ReadFileQuery>,
) -> Result<String, axum::http::StatusCode> {
    match state.manager.read_file(id, &query.path).await {
        Ok(content) => Ok(content),
        Err(e) => {
            tracing::error!("Failed to read file from sandbox {id}: {e}");
            Err(axum::http::StatusCode::NOT_FOUND)
        }
    }
}

async fn handle_list(
    Extension(state): Extension<Arc<EmbeddedSandboxState>>,
) -> Json<serde_json::Value> {
    let sandboxes = state.manager.list_sandboxes().await;
    let list: Vec<serde_json::Value> = sandboxes
        .into_iter()
        .map(|(id, status)| serde_json::json!({ "id": id, "status": status }))
        .collect();
    Json(serde_json::json!(list))
}
