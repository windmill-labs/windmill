/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::{Path, Query},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult},
};
use windmill_sandbox::{ExecRequest, ExecResult, SandboxConfig, SandboxInfo, SandboxStatus};

use windmill_api_auth::ApiAuthed;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create_sandbox))
        .route("/list", get(list_sandboxes))
        .route("/:sandbox_id", get(get_sandbox))
        .route("/:sandbox_id", delete(delete_sandbox))
        .route("/:sandbox_id/exec", post(exec_sandbox))
        .route("/:sandbox_id/suspend", post(suspend_sandbox))
        .route("/:sandbox_id/resume", post(resume_sandbox))
        .route("/:sandbox_id/terminate", post(terminate_sandbox))
        .route("/:sandbox_id/write_file", post(write_file))
        .route("/:sandbox_id/read_file", get(read_file))
        .route("/:sandbox_id/execs", get(list_execs))
}

#[derive(sqlx::FromRow)]
struct SandboxRow {
    id: Uuid,
    workspace_id: String,
    status: SandboxStatus,
    image: Option<String>,
    labels: serde_json::Value,
    mode: String,
    created_by: String,
    created_at: chrono::DateTime<Utc>,
    started_at: Option<chrono::DateTime<Utc>>,
    last_activity_at: Option<chrono::DateTime<Utc>>,
    suspended_at: Option<chrono::DateTime<Utc>>,
    stopped_at: Option<chrono::DateTime<Utc>>,
    error_message: Option<String>,
    ephemeral: bool,
    cpu_limit: i32,
    memory_limit_mb: i32,
    network_enabled: bool,
}

impl From<SandboxRow> for SandboxInfo {
    fn from(r: SandboxRow) -> Self {
        SandboxInfo {
            id: r.id,
            workspace_id: r.workspace_id,
            status: r.status,
            image: r.image,
            labels: r.labels,
            mode: r.mode,
            created_by: r.created_by,
            created_at: r.created_at,
            started_at: r.started_at,
            last_activity_at: r.last_activity_at,
            suspended_at: r.suspended_at,
            stopped_at: r.stopped_at,
            error_message: r.error_message,
            ephemeral: r.ephemeral,
            cpu_limit: r.cpu_limit,
            memory_limit_mb: r.memory_limit_mb,
            network_enabled: r.network_enabled,
        }
    }
}

#[derive(sqlx::FromRow)]
struct SandboxStatusRow {
    status: SandboxStatus,
    host_id: Option<String>,
    mode: String,
}

async fn create_sandbox(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
    Json(config): Json<SandboxConfig>,
) -> JsonResult<SandboxInfo> {
    let mut tx = user_db.begin(&authed).await?;

    let mode = config.mode.to_string();
    let ephemeral = config.ephemeral || config.mode == windmill_sandbox::SandboxMode::Embedded;
    let expires_at = config
        .timeout_secs
        .map(|t| Utc::now() + chrono::Duration::seconds(t as i64));

    let row = sqlx::query_as::<_, (Uuid, chrono::DateTime<Utc>)>(
        "INSERT INTO sandbox (
            workspace_id, created_by, image, timeout_secs, idle_timeout_secs,
            cpu_limit, memory_limit_mb, disk_limit_mb, env_vars, labels,
            mounts, network_enabled, mode, status, ephemeral,
            auto_stop_after_secs, expires_at, started_at, last_activity_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
            $11, $12, $13, 'creating', $14, $15, $16, now(), now()
        ) RETURNING id, created_at",
    )
    .bind(&w_id)
    .bind(&authed.username)
    .bind(config.image.as_deref())
    .bind(config.timeout_secs)
    .bind(config.idle_timeout_secs)
    .bind(config.cpu_limit)
    .bind(config.memory_limit_mb)
    .bind(config.disk_limit_mb)
    .bind(&config.env_vars)
    .bind(&config.labels)
    .bind(&config.mounts)
    .bind(config.network_enabled)
    .bind(&mode)
    .bind(ephemeral)
    .bind(config.auto_stop_after_secs)
    .bind(expires_at)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(SandboxInfo {
        id: row.0,
        workspace_id: w_id,
        status: SandboxStatus::Creating,
        image: config.image,
        labels: config.labels,
        mode,
        created_by: authed.username,
        created_at: row.1,
        started_at: None,
        last_activity_at: None,
        suspended_at: None,
        stopped_at: None,
        error_message: None,
        ephemeral,
        cpu_limit: config.cpu_limit,
        memory_limit_mb: config.memory_limit_mb,
        network_enabled: config.network_enabled,
    }))
}

#[derive(Deserialize)]
struct ListSandboxesQuery {
    status: Option<String>,
    label_key: Option<String>,
    label_value: Option<String>,
}

const SANDBOX_SELECT_COLS: &str = "id, workspace_id, status, image, labels, mode, created_by, created_at, started_at, last_activity_at, suspended_at, stopped_at, error_message, ephemeral, cpu_limit, memory_limit_mb, network_enabled";

async fn list_sandboxes(
    authed: ApiAuthed,
    Path(w_id): Path<String>,
    Extension(user_db): Extension<UserDB>,
    Query(query): Query<ListSandboxesQuery>,
) -> JsonResult<Vec<SandboxInfo>> {
    let mut tx = user_db.begin(&authed).await?;

    let statuses: Option<Vec<String>> = query
        .status
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect());

    let rows = sqlx::query_as::<_, SandboxRow>(&format!(
        "SELECT {SANDBOX_SELECT_COLS}
        FROM sandbox
        WHERE workspace_id = $1
            AND ($2::text[] IS NULL OR status::text = ANY($2))
            AND ($3::text IS NULL OR $4::text IS NULL OR labels->>$3 = $4)
        ORDER BY created_at DESC
        LIMIT 100"
    ))
    .bind(&w_id)
    .bind(statuses.as_deref())
    .bind(query.label_key.as_deref())
    .bind(query.label_value.as_deref())
    .fetch_all(&mut *tx)
    .await?;

    let sandboxes = rows.into_iter().map(SandboxInfo::from).collect();
    Ok(Json(sandboxes))
}

async fn get_sandbox(
    authed: ApiAuthed,
    Path((w_id, sandbox_id)): Path<(String, Uuid)>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<SandboxInfo> {
    let mut tx = user_db.begin(&authed).await?;

    let r = sqlx::query_as::<_, SandboxRow>(&format!(
        "SELECT {SANDBOX_SELECT_COLS} FROM sandbox WHERE id = $1 AND workspace_id = $2"
    ))
    .bind(sandbox_id)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;

    Ok(Json(r.into()))
}

async fn delete_sandbox(
    authed: ApiAuthed,
    Path((w_id, sandbox_id)): Path<(String, Uuid)>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<String> {
    let mut tx = user_db.begin(&authed).await?;

    let status = sqlx::query_scalar::<_, SandboxStatus>(
        "SELECT status FROM sandbox WHERE id = $1 AND workspace_id = $2",
    )
    .bind(sandbox_id)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;

    if status != SandboxStatus::Stopped && status != SandboxStatus::Error {
        return Err(Error::BadRequest(
            "Sandbox must be stopped or in error state before deletion".to_string(),
        ));
    }

    sqlx::query("DELETE FROM sandbox WHERE id = $1")
        .bind(sandbox_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(format!("Sandbox {sandbox_id} deleted")))
}

async fn exec_sandbox(
    authed: ApiAuthed,
    Path((w_id, sandbox_id)): Path<(String, Uuid)>,
    Extension(user_db): Extension<UserDB>,
    Json(request): Json<ExecRequest>,
) -> JsonResult<ExecResult> {
    let mut tx = user_db.begin(&authed).await?;

    let sandbox = sqlx::query_as::<_, SandboxStatusRow>(
        "SELECT status, host_id, mode FROM sandbox WHERE id = $1 AND workspace_id = $2",
    )
    .bind(sandbox_id)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;

    if sandbox.status != SandboxStatus::Running {
        return Err(Error::BadRequest(format!(
            "Sandbox is not running (status: {})",
            sandbox.status
        )));
    }

    let host_base_url = if sandbox.mode == "remote" {
        let host_id = sandbox.host_id.as_ref().ok_or_else(|| {
            Error::InternalErr("Remote sandbox has no host_id".to_string())
        })?;
        let host = sqlx::query_scalar::<_, String>(
            "SELECT base_url FROM sandbox_host WHERE id = $1",
        )
        .bind(host_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Sandbox host {host_id} not found")))?;
        Some(host)
    } else {
        None
    };

    let started_at = Utc::now();

    let result = if let Some(base_url) = host_base_url {
        proxy_exec(&base_url, sandbox_id, &request).await?
    } else {
        return Err(Error::BadRequest(
            "Embedded sandbox exec must be performed directly via WM_SANDBOX_URL".to_string(),
        ));
    };

    let completed_at = Utc::now();
    let duration_ms = (completed_at - started_at).num_milliseconds();

    sqlx::query(
        "INSERT INTO sandbox_exec (
            sandbox_id, workspace_id, command, cwd, env_vars,
            started_at, completed_at, exit_code, stdout, stderr,
            duration_ms, created_by
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(sandbox_id)
    .bind(&w_id)
    .bind(&request.command)
    .bind(request.cwd.as_deref())
    .bind(&request.env)
    .bind(started_at)
    .bind(completed_at)
    .bind(result.exit_code)
    .bind(&result.stdout)
    .bind(&result.stderr)
    .bind(duration_ms)
    .bind(&authed.username)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE sandbox SET last_activity_at = now() WHERE id = $1")
        .bind(sandbox_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(result))
}

async fn proxy_exec(
    base_url: &str,
    sandbox_id: Uuid,
    request: &ExecRequest,
) -> windmill_common::error::Result<ExecResult> {
    let client = reqwest::Client::new();
    let url = format!("{}/sandbox/{}/exec", base_url, sandbox_id);
    let resp = client
        .post(&url)
        .json(request)
        .timeout(std::time::Duration::from_secs(
            request.timeout_secs.unwrap_or(300) as u64 + 5,
        ))
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to proxy exec to sandbox host: {e}")))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::InternalErr(format!(
            "Sandbox host returned error: {body}"
        )));
    }

    resp.json::<ExecResult>()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to parse exec result: {e}")))
}

async fn proxy_action(
    base_url: &str,
    sandbox_id: Uuid,
    action: &str,
) -> windmill_common::error::Result<SandboxStatus> {
    let client = reqwest::Client::new();
    let url = format!("{}/sandbox/{}/{}", base_url, sandbox_id, action);
    let resp = client
        .post(&url)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| {
            Error::InternalErr(format!("Failed to proxy {action} to sandbox host: {e}"))
        })?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(Error::InternalErr(format!(
            "Sandbox host returned error: {body}"
        )));
    }

    #[derive(Deserialize)]
    struct StatusResponse {
        status: SandboxStatus,
    }

    let status_resp = resp.json::<StatusResponse>().await.map_err(|e| {
        Error::InternalErr(format!("Failed to parse action response: {e}"))
    })?;

    Ok(status_resp.status)
}

async fn get_host_base_url(
    tx: &mut sqlx::PgConnection,
    host_id: &str,
) -> windmill_common::error::Result<String> {
    sqlx::query_scalar::<_, String>("SELECT base_url FROM sandbox_host WHERE id = $1")
        .bind(host_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| Error::NotFound(format!("Sandbox host {host_id} not found")))
}

async fn suspend_sandbox(
    authed: ApiAuthed,
    Path((w_id, sandbox_id)): Path<(String, Uuid)>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<SandboxInfo> {
    let mut tx = user_db.clone().begin(&authed).await?;

    let sandbox = sqlx::query_as::<_, SandboxStatusRow>(
        "SELECT status, host_id, mode FROM sandbox WHERE id = $1 AND workspace_id = $2",
    )
    .bind(sandbox_id)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;

    if sandbox.status != SandboxStatus::Running {
        return Err(Error::BadRequest(format!(
            "Can only suspend a running sandbox (status: {})",
            sandbox.status
        )));
    }

    if sandbox.mode == "remote" {
        let host_id = sandbox.host_id.as_ref().ok_or_else(|| {
            Error::InternalErr("Remote sandbox has no host_id".to_string())
        })?;
        let base_url = get_host_base_url(&mut *tx, host_id).await?;
        proxy_action(&base_url, sandbox_id, "suspend").await?;
    }

    sqlx::query(
        "UPDATE sandbox SET status = 'suspended', suspended_at = now() WHERE id = $1",
    )
    .bind(sandbox_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    get_sandbox_info(user_db, &authed, &w_id, sandbox_id).await
}

async fn resume_sandbox(
    authed: ApiAuthed,
    Path((w_id, sandbox_id)): Path<(String, Uuid)>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<SandboxInfo> {
    let mut tx = user_db.clone().begin(&authed).await?;

    let sandbox = sqlx::query_as::<_, SandboxStatusRow>(
        "SELECT status, host_id, mode FROM sandbox WHERE id = $1 AND workspace_id = $2",
    )
    .bind(sandbox_id)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;

    if sandbox.status != SandboxStatus::Suspended {
        return Err(Error::BadRequest(format!(
            "Can only resume a suspended sandbox (status: {})",
            sandbox.status
        )));
    }

    if sandbox.mode == "remote" {
        let host_id = sandbox.host_id.as_ref().ok_or_else(|| {
            Error::InternalErr("Remote sandbox has no host_id".to_string())
        })?;
        let base_url = get_host_base_url(&mut *tx, host_id).await?;
        proxy_action(&base_url, sandbox_id, "resume").await?;
    }

    sqlx::query(
        "UPDATE sandbox SET status = 'running', suspended_at = NULL, last_activity_at = now() WHERE id = $1",
    )
    .bind(sandbox_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    get_sandbox_info(user_db, &authed, &w_id, sandbox_id).await
}

async fn terminate_sandbox(
    authed: ApiAuthed,
    Path((w_id, sandbox_id)): Path<(String, Uuid)>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<SandboxInfo> {
    let mut tx = user_db.clone().begin(&authed).await?;

    let sandbox = sqlx::query_as::<_, SandboxStatusRow>(
        "SELECT status, host_id, mode FROM sandbox WHERE id = $1 AND workspace_id = $2",
    )
    .bind(sandbox_id)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;

    if sandbox.status == SandboxStatus::Stopped {
        return get_sandbox_info(user_db, &authed, &w_id, sandbox_id).await;
    }

    if sandbox.mode == "remote" {
        if let Some(host_id) = sandbox.host_id.as_ref() {
            if let Ok(base_url) = get_host_base_url(&mut *tx, host_id).await {
                let _ = proxy_action(&base_url, sandbox_id, "terminate").await;
            }
        }
    }

    sqlx::query(
        "UPDATE sandbox SET status = 'stopped', stopped_at = now() WHERE id = $1",
    )
    .bind(sandbox_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    get_sandbox_info(user_db, &authed, &w_id, sandbox_id).await
}

#[derive(Deserialize, Serialize)]
struct WriteFileBody {
    path: String,
    content: String,
}

async fn write_file(
    authed: ApiAuthed,
    Path((w_id, sandbox_id)): Path<(String, Uuid)>,
    Extension(user_db): Extension<UserDB>,
    Json(body): Json<WriteFileBody>,
) -> JsonResult<String> {
    let mut tx = user_db.begin(&authed).await?;

    let sandbox = sqlx::query_as::<_, SandboxStatusRow>(
        "SELECT status, host_id, mode FROM sandbox WHERE id = $1 AND workspace_id = $2",
    )
    .bind(sandbox_id)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;

    if sandbox.status == SandboxStatus::Stopped {
        return Err(Error::BadRequest("Sandbox is stopped".to_string()));
    }

    if sandbox.mode == "remote" {
        let host_id = sandbox.host_id.as_ref().ok_or_else(|| {
            Error::InternalErr("Remote sandbox has no host_id".to_string())
        })?;
        let base_url = get_host_base_url(&mut *tx, host_id).await?;
        let client = reqwest::Client::new();
        let url = format!("{}/sandbox/{}/write_file", base_url, sandbox_id);
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::InternalErr(format!("Failed to proxy write_file: {e}")))?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(Error::InternalErr(format!("Write file failed: {err}")));
        }
    }

    sqlx::query("UPDATE sandbox SET last_activity_at = now() WHERE id = $1")
        .bind(sandbox_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json("ok".to_string()))
}

#[derive(Deserialize)]
struct ReadFileQuery {
    path: String,
}

async fn read_file(
    authed: ApiAuthed,
    Path((w_id, sandbox_id)): Path<(String, Uuid)>,
    Extension(user_db): Extension<UserDB>,
    Query(query): Query<ReadFileQuery>,
) -> JsonResult<String> {
    let mut tx = user_db.begin(&authed).await?;

    let sandbox = sqlx::query_as::<_, SandboxStatusRow>(
        "SELECT status, host_id, mode FROM sandbox WHERE id = $1 AND workspace_id = $2",
    )
    .bind(sandbox_id)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Sandbox {sandbox_id} not found")))?;

    if sandbox.status == SandboxStatus::Stopped {
        return Err(Error::BadRequest("Sandbox is stopped".to_string()));
    }

    if sandbox.mode == "remote" {
        let host_id = sandbox.host_id.as_ref().ok_or_else(|| {
            Error::InternalErr("Remote sandbox has no host_id".to_string())
        })?;
        let base_url = get_host_base_url(&mut *tx, host_id).await?;
        let client = reqwest::Client::new();
        let url = format!(
            "{}/sandbox/{}/read_file?path={}",
            base_url, sandbox_id, query.path
        );
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::InternalErr(format!("Failed to proxy read_file: {e}")))?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(Error::InternalErr(format!("Read file failed: {err}")));
        }
        let content = resp.text().await.map_err(|e| {
            Error::InternalErr(format!("Failed to read response body: {e}"))
        })?;
        return Ok(Json(content));
    }

    Err(Error::BadRequest(
        "Embedded sandbox file ops must be performed directly via WM_SANDBOX_URL".to_string(),
    ))
}

#[derive(Serialize, sqlx::FromRow)]
struct ExecRecord {
    id: Uuid,
    sandbox_id: Uuid,
    command: String,
    started_at: chrono::DateTime<Utc>,
    completed_at: Option<chrono::DateTime<Utc>>,
    exit_code: Option<i32>,
    stdout: Option<String>,
    stderr: Option<String>,
    duration_ms: Option<i64>,
    created_by: String,
}

async fn list_execs(
    authed: ApiAuthed,
    Path((w_id, sandbox_id)): Path<(String, Uuid)>,
    Extension(user_db): Extension<UserDB>,
) -> JsonResult<Vec<ExecRecord>> {
    let mut tx = user_db.begin(&authed).await?;

    let rows = sqlx::query_as::<_, ExecRecord>(
        "SELECT id, sandbox_id, command, started_at, completed_at,
            exit_code, stdout, stderr, duration_ms, created_by
        FROM sandbox_exec
        WHERE sandbox_id = $1 AND workspace_id = $2
        ORDER BY started_at DESC
        LIMIT 100",
    )
    .bind(sandbox_id)
    .bind(&w_id)
    .fetch_all(&mut *tx)
    .await?;

    Ok(Json(rows))
}

async fn get_sandbox_info(
    user_db: UserDB,
    authed: &ApiAuthed,
    w_id: &str,
    sandbox_id: Uuid,
) -> JsonResult<SandboxInfo> {
    let mut tx = user_db.begin(authed).await?;

    let r = sqlx::query_as::<_, SandboxRow>(&format!(
        "SELECT {SANDBOX_SELECT_COLS} FROM sandbox WHERE id = $1 AND workspace_id = $2"
    ))
    .bind(sandbox_id)
    .bind(w_id)
    .fetch_one(&mut *tx)
    .await?;

    Ok(Json(r.into()))
}
