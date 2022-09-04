/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::extract::Host;

use sql_builder::prelude::*;
use sqlx::{query_scalar, Postgres, Transaction};
use std::collections::HashMap;
use tracing::instrument;

use crate::{
    audit::{audit_log, ActionKind},
    db::{UserDB, DB},
    error,
    error::{to_anyhow, Error},
    flows::FlowValue,
    schedule::get_schedule_opt,
    scripts::{get_hub_script_by_path, ScriptHash, ScriptLang},
    users::{owner_to_token_owner, Authed},
    utils::{require_admin, Pagination, StripPath, now_from_db},
    worker,
    worker_flow::{init_flow_status, FlowStatus},
};
use axum::{
    extract::{Extension, Path, Query},
    routing::{get, post},
    Json, Router,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sql_builder::SqlBuilder;

use ulid::Ulid;
use uuid::Uuid;

const MAX_NB_OF_JOBS_IN_Q_PER_USER: i64 = 10;
const MAX_DURATION_LAST_1200: std::time::Duration = std::time::Duration::from_secs(900);

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/run/f/*script_path", post(run_flow_by_path))
        .route("/run/p/*script_path", post(run_job_by_path))
        .route(
            "/run_wait_result/p/*script_path",
            post(run_wait_result_job_by_path),
        )
        .route("/run/h/:hash", post(run_job_by_hash))
        .route("/run/preview", post(run_preview_job))
        .route("/run/preview_flow", post(run_preview_flow_job))
        .route("/list", get(list_jobs))
        .route("/queue/list", get(list_queue_jobs))
        .route("/queue/cancel/:id", post(cancel_job))
        .route("/completed/list", get(list_completed_jobs))
        .route("/completed/get/:id", get(get_completed_job))
        .route("/completed/get_result/:id", get(get_completed_job_result))
        .route("/completed/delete/:id", post(delete_completed_job))
        .route("/get/:id", get(get_job))
        .route("/getupdate/:id", get(get_job_update))
}

pub fn global_service() -> Router {
    Router::new().route("/resume/:id", post(resume_job))
}

#[derive(Debug, sqlx::FromRow, Serialize, Clone)]
pub struct QueuedJob {
    pub workspace_id: String,
    pub id: Uuid,
    pub parent_job: Option<Uuid>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub scheduled_for: chrono::DateTime<chrono::Utc>,
    pub running: bool,
    pub script_hash: Option<ScriptHash>,
    pub script_path: Option<String>,
    pub args: Option<serde_json::Value>,
    pub logs: Option<String>,
    pub raw_code: Option<String>,
    pub canceled: bool,
    pub canceled_by: Option<String>,
    pub canceled_reason: Option<String>,
    pub last_ping: Option<chrono::DateTime<chrono::Utc>>,
    pub job_kind: JobKind,
    pub schedule_path: Option<String>,
    pub permissioned_as: String,
    pub flow_status: Option<serde_json::Value>,
    pub raw_flow: Option<serde_json::Value>,
    pub is_flow_step: bool,
    pub language: Option<ScriptLang>,
}

impl QueuedJob {
    pub fn script_path(&self) -> &str {
        self.script_path
            .as_ref()
            .map(String::as_str)
            .unwrap_or("NO_FLOW_PATH")
    }

    pub fn parse_raw_flow(&self) -> Option<FlowValue> {
        self.raw_flow
            .as_ref()
            .and_then(|v| serde_json::from_value::<FlowValue>(v.clone()).ok())
    }

    pub fn parse_flow_status(&self) -> Option<FlowStatus> {
        self.flow_status
            .as_ref()
            .and_then(|v| serde_json::from_value::<FlowStatus>(v.clone()).ok())
    }
}

#[derive(Debug, sqlx::FromRow, Serialize)]
struct CompletedJob {
    workspace_id: String,
    id: Uuid,
    parent_job: Option<Uuid>,
    created_by: String,
    created_at: chrono::DateTime<chrono::Utc>,
    started_at: chrono::DateTime<chrono::Utc>,
    duration_ms: i32,
    success: bool,
    script_hash: Option<ScriptHash>,
    script_path: Option<String>,
    args: Option<serde_json::Value>,
    result: Option<serde_json::Value>,
    logs: Option<String>,
    deleted: bool,
    raw_code: Option<String>,
    canceled: bool,
    canceled_by: Option<String>,
    canceled_reason: Option<String>,
    job_kind: JobKind,
    schedule_path: Option<String>,
    permissioned_as: String,
    flow_status: Option<serde_json::Value>,
    raw_flow: Option<serde_json::Value>,
    is_flow_step: bool,
    language: Option<ScriptLang>,
    is_skipped: bool,
}

#[derive(Deserialize, Clone, Copy)]
pub struct RunJobQuery {
    scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    scheduled_in_secs: Option<i64>,
    parent_job: Option<Uuid>,
}

impl RunJobQuery {
    async fn get_scheduled_for<'c>(
        self,
        db: &mut Transaction<'c, Postgres>,
    ) -> error::Result<Option<chrono::DateTime<chrono::Utc>>> {
        if let Some(scheduled_for) = self.scheduled_for {
            Ok(Some(scheduled_for))
        } else if let Some(scheduled_in_secs) = self.scheduled_in_secs {
            let now = now_from_db(db).await?;
            Ok(Some(now + chrono::Duration::seconds(scheduled_in_secs)))
        } else {
            Ok(None)
        }
    }
}

pub async fn run_flow_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    axum::Json(args): axum::Json<Option<Map<String, Value>>>,
    Query(run_query): Query<RunJobQuery>,
) -> error::Result<(StatusCode, String)> {
    let flow_path = flow_path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    let scheduled_for = run_query.get_scheduled_for(&mut tx).await?;
    let (uuid, tx) = push(
        tx,
        &w_id,
        JobPayload::Flow(flow_path.to_string()),
        args,
        &authed.username,
        owner_to_token_owner(&authed.username, false),
        scheduled_for,
        None,
        run_query.parent_job,
        false,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

pub async fn run_job_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    axum::Json(args): axum::Json<Option<Map<String, Value>>>,
    Query(run_query): Query<RunJobQuery>,
) -> error::Result<(StatusCode, String)> {
    let script_path = script_path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    let job_payload = script_path_to_payload(script_path, &mut tx, &w_id).await?;
    let scheduled_for = run_query.get_scheduled_for(&mut tx).await?;

    let (uuid, tx) = push(
        tx,
        &w_id,
        job_payload,
        args,
        &authed.username,
        owner_to_token_owner(&authed.username, false),
        scheduled_for,
        None,
        run_query.parent_job,
        false,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

pub async fn run_wait_result_job_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    axum::Json(args): axum::Json<Option<Map<String, Value>>>,
    Query(run_query): Query<RunJobQuery>,
) -> error::JsonResult<serde_json::Value> {
    let script_path = script_path.to_path();
    let mut tx = user_db.clone().begin(&authed).await?;
    let job_payload = script_path_to_payload(script_path, &mut tx, &w_id).await?;
        let scheduled_for = run_query.get_scheduled_for(&mut tx).await?;

    let (uuid, tx) = push(
        tx,
        &w_id,
        job_payload,
        args,
        &authed.username,
        owner_to_token_owner(&authed.username, false),
        scheduled_for,
        None,
        run_query.parent_job,
        false,
    )
    .await?;
    tx.commit().await?;

    //wait 5 secs top
    let mut result = None;
    for i in 0..48 {
        let mut tx = user_db.clone().begin(&authed).await?;

        result = sqlx::query_scalar!(
            "SELECT result FROM completed_job WHERE id = $1 AND workspace_id = $2",
            uuid,
            &w_id
        )
        .fetch_optional(&mut tx)
        .await?
        .flatten();

        if result.is_some() {
            break;
        }
        let delay = if i < 10 { 100 } else { 500 };
        tokio::time::sleep(core::time::Duration::from_millis(delay)).await;
    }
    if let Some(result) = result {
        Ok(Json(result))
    } else {
        Err(Error::ExecutionErr("timeout after 20s".to_string()))
    }
}

pub async fn script_path_to_payload<'c>(
    script_path: &str,
    db: &mut Transaction<'c, Postgres>,
    w_id: &String,
) -> Result<JobPayload, Error> {
    let job_payload = if script_path.starts_with("hub/") {
        JobPayload::ScriptHub { path: script_path.to_owned() }
    } else {
        let script_hash = get_latest_hash_for_path(db, w_id, script_path).await?;
        JobPayload::ScriptHash { hash: script_hash, path: script_path.to_owned() }
    };
    Ok(job_payload)
}

pub async fn get_latest_hash_for_path<'c>(
    db: &mut Transaction<'c, Postgres>,
    w_id: &str,
    script_path: &str,
) -> error::Result<ScriptHash> {
    let script_hash_o = sqlx::query_scalar!(
        "select hash from script where path = $1 AND (workspace_id = $2 OR workspace_id = \
         'starter') AND
    created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND (workspace_id = $2 OR \
         workspace_id = 'starter')) AND
    deleted = false",
        script_path,
        w_id
    )
    .fetch_optional(db)
    .await?;

    let script_hash = crate::utils::not_found_if_none(script_hash_o, "ScriptHash", script_path)?;

    Ok(ScriptHash(script_hash))
}

pub async fn run_job_by_hash(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_hash)): Path<(String, ScriptHash)>,
    axum::Json(args): axum::Json<Option<Map<String, Value>>>,
    Query(run_query): Query<RunJobQuery>,
) -> error::Result<(StatusCode, String)> {
    let hash = script_hash.0;
    let mut tx = user_db.begin(&authed).await?;
    let path = get_path_for_hash(&mut tx, &w_id, hash).await?;
    let scheduled_for = run_query.get_scheduled_for(&mut tx).await?;

    let (uuid, tx) = push(
        tx,
        &w_id,
        JobPayload::ScriptHash { hash: ScriptHash(hash), path },
        args,
        &authed.username,
        owner_to_token_owner(&authed.username, false),
        scheduled_for,
        None,
        run_query.parent_job,
        false,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

pub async fn get_path_for_hash<'c>(
    db: &mut Transaction<'c, Postgres>,
    w_id: &str,
    hash: i64,
) -> error::Result<String> {
    let path = sqlx::query_scalar!(
        "select path from script where hash = $1 AND (workspace_id = $2 OR workspace_id = \
         'starter')",
        hash,
        w_id
    )
    .fetch_one(db)
    .await
    .map_err(|e| {
        Error::InternalErr(format!(
            "querying getting path for hash {hash} in {w_id}: {e}"
        ))
    })?;
    Ok(path)
}

async fn run_preview_job(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(preview): Json<Preview>,
    Query(sch_query): Query<RunJobQuery>,
) -> error::Result<(StatusCode, String)> {
    let mut tx = user_db.begin(&authed).await?;
    let scheduled_for = sch_query.get_scheduled_for(&mut tx).await?;

    let (uuid, tx) = push(
        tx,
        &w_id,
        JobPayload::Code(RawCode {
            content: preview.content,
            path: preview.path,
            language: preview.language,
        }),
        preview.args,
        &authed.username,
        owner_to_token_owner(&authed.username, false),
        scheduled_for,
        None,
        None,
        false,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

async fn run_preview_flow_job(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(raw_flow): Json<PreviewFlow>,
    Query(sch_query): Query<RunJobQuery>,
) -> error::Result<(StatusCode, String)> {
    let mut tx = user_db.begin(&authed).await?;
    let scheduled_for = sch_query.get_scheduled_for(&mut tx).await?;
    let (uuid, tx) = push(
        tx,
        &w_id,
        JobPayload::RawFlow { value: raw_flow.value, path: raw_flow.path },
        raw_flow.args,
        &authed.username,
        owner_to_token_owner(&authed.username, false),
        scheduled_for,
        None,
        None,
        false,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}
#[derive(Deserialize)]
pub struct ListQueueQuery {
    pub script_path_start: Option<String>,
    pub script_path_exact: Option<String>,
    pub script_hash: Option<String>,
    pub created_by: Option<String>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub running: Option<bool>,
    pub parent_job: Option<String>,
    pub order_desc: Option<bool>,
    pub job_kinds: Option<String>,
}

fn list_queue_jobs_query(w_id: &str, lq: &ListQueueQuery, fields: &[&str]) -> SqlBuilder {
    let mut sqlb = SqlBuilder::select_from("queue")
        .fields(fields)
        .order_by("created_at", lq.order_desc.unwrap_or(true))
        .limit(1000)
        .and_where_eq("workspace_id", "?".bind(&w_id))
        .clone();

    if let Some(ps) = &lq.script_path_start {
        sqlb.and_where_like_left("script_path", "?".bind(ps));
    }
    if let Some(p) = &lq.script_path_exact {
        sqlb.and_where_eq("script_path", "?".bind(p));
    }
    if let Some(h) = &lq.script_hash {
        sqlb.and_where_eq("script_hash", "?".bind(h));
    }
    if let Some(cb) = &lq.created_by {
        sqlb.and_where_eq("created_by", "?".bind(cb));
    }
    if let Some(r) = &lq.running {
        sqlb.and_where_eq("running", &r);
    }
    if let Some(pj) = &lq.parent_job {
        sqlb.and_where_eq("parent_job", "?".bind(pj));
    }
    if let Some(dt) = &lq.created_before {
        sqlb.and_where_lt("created_at", format!("to_timestamp({})", dt.timestamp()));
    }
    if let Some(dt) = &lq.created_after {
        sqlb.and_where_gt("created_at", format!("to_timestamp({})", dt.timestamp()));
    }
    if let Some(jk) = &lq.job_kinds {
        sqlb.and_where_in(
            "job_kind",
            &jk.split(',').into_iter().map(quote).collect::<Vec<_>>(),
        );
    }

    sqlb
}

async fn list_queue_jobs(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(lq): Query<ListQueueQuery>,
) -> error::JsonResult<Vec<QueuedJob>> {
    let sql = list_queue_jobs_query(&w_id, &lq, &["*"]).sql()?;
    let jobs = sqlx::query_as::<_, QueuedJob>(&sql).fetch_all(&db).await?;
    Ok(Json(jobs))
}

async fn list_jobs(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListCompletedQuery>,
) -> error::JsonResult<Vec<Job>> {
    let (per_page, offset) = crate::utils::paginate(pagination);
    let lqc = lq.clone();
    let sqlq = list_queue_jobs_query(
        &w_id,
        &ListQueueQuery {
            script_path_start: lq.script_path_start,
            script_path_exact: lq.script_path_exact,
            script_hash: lq.script_hash,
            created_by: lq.created_by,
            created_before: lq.created_before,
            created_after: lq.created_after,
            running: None,
            parent_job: lq.parent_job,
            order_desc: Some(true),
            job_kinds: lq.job_kinds,
        },
        &[
            "'QueuedJob' as typ",
            "id",
            "workspace_id",
            "parent_job",
            "created_by",
            "created_at",
            "started_at",
            "scheduled_for",
            "running",
            "script_hash",
            "script_path",
            "args",
            "null as duration_ms",
            "null as success",
            "false as deleted",
            "canceled",
            "canceled_by",
            "job_kind",
            "schedule_path",
            "permissioned_as",
            "flow_status",
            "is_flow_step",
            "language",
            "false as is_skipped",
        ],
    );
    let sqlc = list_completed_jobs_query(
        &w_id,
        per_page + offset,
        0,
        &ListCompletedQuery { order_desc: Some(true), ..lqc },
        &[
            "'CompletedJob' as typ",
            "id",
            "workspace_id",
            "parent_job",
            "created_by",
            "created_at",
            "started_at",
            "null as scheduled_for",
            "null as running",
            "script_hash",
            "script_path",
            "args",
            "duration_ms",
            "success",
            "deleted",
            "canceled",
            "canceled_by",
            "job_kind",
            "schedule_path",
            "permissioned_as",
            "flow_status",
            "is_flow_step",
            "language",
            "is_skipped",
        ],
    );
    let sql = format!(
        "{} UNION ALL {} ORDER BY created_at DESC LIMIT {} OFFSET {};",
        &sqlq.subquery()?,
        &sqlc.subquery()?,
        per_page,
        offset
    );
    let mut tx = user_db.begin(&authed).await?;
    let jobs: Vec<UnifiedJob> = sqlx::query_as(&sql).fetch_all(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(jobs.into_iter().map(From::from).collect()))
}
#[derive(Deserialize, Clone)]
pub struct ListCompletedQuery {
    pub script_path_start: Option<String>,
    pub script_path_exact: Option<String>,
    pub script_hash: Option<String>,
    pub created_by: Option<String>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub success: Option<bool>,
    pub parent_job: Option<String>,
    pub order_desc: Option<bool>,
    pub job_kinds: Option<String>,
    pub is_skipped: Option<bool>,
    pub is_flow_step: Option<bool>,
}
fn list_completed_jobs_query(
    w_id: &str,
    per_page: usize,
    offset: usize,
    lq: &ListCompletedQuery,
    fields: &[&str],
) -> SqlBuilder {
    let mut sqlb = SqlBuilder::select_from("completed_job")
        .fields(fields)
        .order_by("created_at", lq.order_desc.unwrap_or(true))
        .and_where_eq("workspace_id", "?".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();

    if let Some(ps) = &lq.script_path_start {
        sqlb.and_where_like_left("script_path", "?".bind(ps));
    }
    if let Some(p) = &lq.script_path_exact {
        sqlb.and_where_eq("script_path", "?".bind(p));
    }
    if let Some(h) = &lq.script_hash {
        sqlb.and_where_eq("script_hash", "?".bind(h));
    }
    if let Some(cb) = &lq.created_by {
        sqlb.and_where_eq("created_by", "?".bind(cb));
    }
    if let Some(r) = &lq.success {
        sqlb.and_where_eq("success", r);
    }
    if let Some(pj) = &lq.parent_job {
        sqlb.and_where_eq("parent_job", "?".bind(pj));
    }
    if let Some(dt) = &lq.created_before {
        sqlb.and_where_lt("created_at", format!("to_timestamp({})", dt.timestamp()));
    }
    if let Some(dt) = &lq.created_after {
        sqlb.and_where_gt("created_at", format!("to_timestamp({})", dt.timestamp()));
    }
    if let Some(sk) = &lq.is_skipped {
        sqlb.and_where_eq("is_skipped", sk);
    }
        if let Some(fs) = &lq.is_flow_step {
        sqlb.and_where_eq("is_flow_step", fs);
    }
    if let Some(jk) = &lq.job_kinds {
        sqlb.and_where_in(
            "job_kind",
            &jk.split(',').into_iter().map(quote).collect::<Vec<_>>(),
        );
    }

    sqlb
}

async fn list_completed_jobs(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListCompletedQuery>,
) -> error::JsonResult<Vec<CompletedJob>> {
    let (per_page, offset) = crate::utils::paginate(pagination);

    let sql = list_completed_jobs_query(
        &w_id,
        per_page,
        offset,
        &lq,
        &[
            "id",
            "workspace_id",
            "parent_job",
            "created_by",
            "created_at",
            "started_at",
            "duration_ms",
            "success",
            "script_hash",
            "script_path",
            "args",
            "result",
            "null as logs",
            "deleted",
            "canceled",
            "canceled_by",
            "canceled_reason",
            "job_kind",
            "schedule_path",
            "permissioned_as",
            "null as raw_code",
            "null as flow_status",
            "null as raw_flow",
            "is_flow_step",
            "language",
            "is_skipped",
        ],
    )
    .sql()?;
    let jobs = sqlx::query_as::<_, CompletedJob>(&sql)
        .fetch_all(&db)
        .await?;
    Ok(Json(jobs))
}

async fn get_completed_job(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::JsonResult<CompletedJob> {
    let job_o = sqlx::query_as::<_, CompletedJob>(
        "SELECT * FROM completed_job WHERE id = $1 AND workspace_id = $2",
    )
    .bind(id)
    .bind(w_id)
    .fetch_optional(&db)
    .await?;

    let job = crate::utils::not_found_if_none(job_o, "Completed Job", id.to_string())?;
    Ok(Json(job))
}

async fn get_completed_job_result(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::JsonResult<Option<serde_json::Value>> {
    let result_o = sqlx::query_scalar!(
        "SELECT result FROM completed_job WHERE id = $1 AND workspace_id = $2",
        id,
        w_id,
    )
    .fetch_optional(&db)
    .await?;

    let result = crate::utils::not_found_if_none(result_o, "Completed Job", id.to_string())?;
    Ok(Json(result))
}

async fn cancel_job(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Json(CancelJob { reason }): Json<CancelJob>,
) -> error::Result<String> {
    let mut tx = user_db.begin(&authed).await?;

    let job_option = sqlx::query_scalar!(
        "UPDATE queue SET canceled = true, canceled_by = $1, canceled_reason = $2 WHERE id = $3 \
         AND schedule_path IS NULL AND workspace_id = $4RETURNING id",
        &authed.username,
        reason,
        id,
        w_id
    )
    .fetch_optional(&mut tx)
    .await?;

    if let Some(id) = job_option {
        audit_log(
            &mut tx,
            &authed.username,
            "jobs.delete",
            ActionKind::Delete,
            &w_id,
            Some(&id.to_string()),
            None,
        )
        .await?;
        Ok(id.to_string())
    } else {
        let (job_o, tx) = get_job_by_id(tx, &w_id, id).await?;
        tx.commit().await?;
        let err = match job_o {
            Some(Job::CompletedJob(_)) => error::Error::BadRequest(format!(
                "queued job id {} exists but is already completed and cannot be canceled",
                id
            )),
            Some(Job::QueuedJob(job)) if job.schedule_path.is_some() => {
                error::Error::BadRequest(format!(
                    "queued job id {} exists but has been created by a scheduler 
                and can only be only canceled by disabling the parent scheduler",
                    id
                ))
            }
            _ => error::Error::NotFound(format!("queued job id {} does not exist", id)),
        };
        Err(err)
    }
}

async fn delete_completed_job(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::JsonResult<CompletedJob> {
    let mut tx = user_db.begin(&authed).await?;

    require_admin(authed.is_admin, &authed.username)?;
    let job_o = sqlx::query_as::<_, CompletedJob>(
        "UPDATE completed_job SET logs = '', deleted = true WHERE id = $1 AND workspace_id = $2 \
         RETURNING *",
    )
    .bind(id)
    .bind(&w_id)
    .fetch_optional(&mut tx)
    .await?;

    let job = crate::utils::not_found_if_none(job_o, "Completed Job", id.to_string())?;

    audit_log(
        &mut tx,
        &authed.username,
        "jobs.delete",
        ActionKind::Delete,
        &w_id,
        Some(&id.to_string()),
        None,
    )
    .await?;

    tx.commit().await?;
    Ok(Json(job))
}

#[derive(Deserialize)]
pub struct JobUpdateQuery {
    pub running: bool,
    pub log_offset: i32,
}

#[derive(Serialize)]
pub struct JobUpdate {
    pub running: Option<bool>,
    pub completed: Option<bool>,
    pub new_logs: Option<String>,
}

async fn get_job_update(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Query(JobUpdateQuery { running, log_offset }): Query<JobUpdateQuery>,
) -> error::JsonResult<JobUpdate> {
    let mut tx = db.begin().await?;

    let logs = query_scalar!(
        "SELECT substr(logs, $1) as logs FROM queue WHERE workspace_id = $2 AND id = $3",
        log_offset,
        &w_id,
        &id
    )
    .fetch_optional(&mut tx)
    .await?;

    if let Some(logs) = logs {
        tx.commit().await?;
        Ok(Json(JobUpdate {
            running: if !running { Some(true) } else { None },
            completed: None,
            new_logs: logs,
        }))
    } else {
        let logs = query_scalar!(
            "SELECT substr(logs, $1) as logs FROM completed_job WHERE workspace_id = $2 AND id = \
             $3",
            log_offset,
            &w_id,
            &id
        )
        .fetch_optional(&mut tx)
        .await?;
        let logs = crate::utils::not_found_if_none(logs, "Job", id.to_string())?;
        tx.commit().await?;
        Ok(Json(JobUpdate {
            running: Some(false),
            completed: Some(true),
            new_logs: logs,
        }))
    }
}

async fn get_job(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::JsonResult<Job> {
    let tx = db.begin().await?;
    let (job_o, tx) = get_job_by_id(tx, &w_id, id).await?;
    let job = crate::utils::not_found_if_none(job_o, "Completed Job", id.to_string())?;
    tx.commit().await?;
    Ok(Json(job))
}

async fn get_job_by_id<'c>(
    mut tx: Transaction<'c, Postgres>,
    w_id: &str,
    id: Uuid,
) -> error::Result<(Option<Job>, Transaction<'c, Postgres>)> {
    let cjob_option = sqlx::query_as::<_, CompletedJob>(
        "SELECT * FROM completed_job WHERE id = $1 AND workspace_id = $2",
    )
    .bind(id)
    .bind(w_id)
    .fetch_optional(&mut tx)
    .await?;
    let job_option = match cjob_option {
        Some(job) => Some(Job::CompletedJob(job)),
        None => get_queued_job(id, w_id, &mut tx).await?.map(Job::QueuedJob),
    };
    Ok((job_option, tx))
}

pub async fn get_queued_job<'c>(
    id: Uuid,
    w_id: &str,
    tx: &mut Transaction<'c, Postgres>,
) -> error::Result<Option<QueuedJob>> {
    let r = sqlx::query_as::<_, QueuedJob>(
        "SELECT *
            FROM queue WHERE id = $1 AND workspace_id = $2",
    )
    .bind(id)
    .bind(w_id)
    .fetch_optional(tx)
    .await?;
    Ok(r)
}

pub async fn resume_job(
    /* unauthed */
    Extension(db): Extension<DB>,
    Path((_, id)): Path<(String, Uuid)>,
    Json(value): Json<serde_json::Value>,
    ) -> error::Result<StatusCode>
{
    let mut tx = db.begin().await?;

    sqlx::query!(
        r#"
        INSERT INTO resume_job
                    (id, job, flow, value)
             VALUES ($1, $2, (SELECT parent_job FROM queue WHERE id = $2), $3);
        "#,
        Uuid::from(Ulid::new()),
        id,
        value,
    ).execute(&mut tx).await?;

    tx.commit().await?;

    Ok(StatusCode::CREATED)
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum Job {
    QueuedJob(QueuedJob),
    CompletedJob(CompletedJob),
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[sqlx(type_name = "JOB_KIND", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase"))]
pub enum JobKind {
    Script,
    #[allow(non_camel_case_types)]
    Script_Hub,
    Preview,
    Dependencies,
    Flow,
    FlowPreview,
}

#[derive(sqlx::FromRow)]
struct UnifiedJob {
    workspace_id: String,
    typ: String,
    id: Uuid,
    parent_job: Option<Uuid>,
    created_by: String,
    created_at: chrono::DateTime<chrono::Utc>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    running: Option<bool>,
    script_hash: Option<ScriptHash>,
    script_path: Option<String>,
    args: Option<serde_json::Value>,
    duration_ms: Option<i32>,
    success: Option<bool>,
    deleted: bool,
    canceled: bool,
    canceled_by: Option<String>,
    job_kind: JobKind,
    schedule_path: Option<String>,
    permissioned_as: String,
    flow_status: Option<serde_json::Value>,
    is_flow_step: bool,
    language: Option<ScriptLang>,
    is_skipped: bool,
}

impl From<UnifiedJob> for Job {
    fn from(uj: UnifiedJob) -> Self {
        match uj.typ.as_ref() {
            "CompletedJob" => Job::CompletedJob(CompletedJob {
                workspace_id: uj.workspace_id,
                id: uj.id,
                parent_job: uj.parent_job,
                created_by: uj.created_by,
                created_at: uj.created_at,
                started_at: uj.started_at.unwrap_or(uj.created_at),
                duration_ms: uj.duration_ms.unwrap(),
                success: uj.success.unwrap(),
                script_hash: uj.script_hash,
                script_path: uj.script_path,
                args: uj.args,
                result: None,
                logs: None,
                deleted: uj.deleted,
                canceled: uj.canceled,
                canceled_by: uj.canceled_by,
                raw_code: None,
                canceled_reason: None,
                job_kind: uj.job_kind,
                schedule_path: uj.schedule_path,
                permissioned_as: uj.permissioned_as,
                flow_status: uj.flow_status,
                raw_flow: None,
                is_flow_step: uj.is_flow_step,
                language: uj.language,
                is_skipped: uj.is_skipped,
            }),
            "QueuedJob" => Job::QueuedJob(QueuedJob {
                workspace_id: uj.workspace_id,
                id: uj.id,
                parent_job: uj.parent_job,
                created_by: uj.created_by,
                created_at: uj.created_at,
                started_at: uj.started_at,
                script_hash: uj.script_hash,
                script_path: uj.script_path,
                args: uj.args,
                running: uj.running.unwrap(),
                scheduled_for: uj.scheduled_for.unwrap(),
                logs: None,
                raw_code: None,
                canceled: uj.canceled,
                canceled_by: uj.canceled_by,
                canceled_reason: None,
                last_ping: None,
                job_kind: uj.job_kind,
                schedule_path: uj.schedule_path,
                permissioned_as: uj.permissioned_as,
                flow_status: uj.flow_status,
                raw_flow: None,
                is_flow_step: uj.is_flow_step,
                language: uj.language,
            }),
            t => panic!("job type {} not valid", t),
        }
    }
}
#[derive(Deserialize)]
struct CancelJob {
    reason: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RawCode {
    pub content: String,
    pub path: Option<String>,
    pub language: ScriptLang,
}

#[derive(Deserialize)]
struct Preview {
    content: String,
    path: Option<String>,
    args: Option<Map<String, Value>>,
    language: ScriptLang,
}

#[derive(Deserialize)]
struct PreviewFlow {
    value: FlowValue,
    path: Option<String>,
    args: Option<Map<String, Value>>,
}

#[derive(Debug, Clone)]
pub enum JobPayload {
    ScriptHub { path: String },
    ScriptHash { hash: ScriptHash, path: String },
    Code(RawCode),
    Dependencies { hash: ScriptHash, dependencies: Vec<String> },
    Flow(String),
    RawFlow { value: FlowValue, path: Option<String> },
}

#[instrument(level = "trace", skip_all)]
pub async fn push<'c>(
    mut tx: Transaction<'c, Postgres>,
    workspace_id: &str,
    job_payload: JobPayload,
    args: Option<Map<String, Value>>,
    user: &str,
    permissioned_as: String,
    scheduled_for_o: Option<chrono::DateTime<chrono::Utc>>,
    schedule_path: Option<String>,
    parent_job: Option<Uuid>,
    is_flow_step: bool,
) -> Result<(Uuid, Transaction<'c, Postgres>), Error> {
    let scheduled_for = scheduled_for_o.unwrap_or_else(chrono::Utc::now);
    let args_json = args.map(serde_json::Value::Object);
    let job_id: Uuid = Ulid::new().into();

    let premium_workspace =
        sqlx::query_scalar!("SELECT premium FROM workspace WHERE id = $1", workspace_id)
            .fetch_one(&mut tx)
            .await
            .map_err(|e| {
                Error::InternalErr(format!("fetching if {workspace_id} is premium: {e}"))
            })?;

    if !premium_workspace && std::env::var("CLOUD_HOSTED").is_ok() {
        let rate_limiting_queue = sqlx::query_scalar!(
            "SELECT COUNT(id) FROM queue WHERE permissioned_as = $1 AND workspace_id = $2",
            permissioned_as,
            workspace_id
        )
        .fetch_one(&mut tx)
        .await?;

        if let Some(nb_jobs) = rate_limiting_queue {
            if nb_jobs > MAX_NB_OF_JOBS_IN_Q_PER_USER {
                return Err(error::Error::ExecutionErr(format!(
                    "You have exceeded the number of authorized elements of queue at any given \
                     time: {}",
                    MAX_NB_OF_JOBS_IN_Q_PER_USER
                )));
            }
        }

        let rate_limiting_duration_ms = sqlx::query_scalar!(
            "
           SELECT SUM(duration_ms)
             FROM completed_job
            WHERE permissioned_as = $1
              AND created_at > NOW() - INTERVAL '1200 seconds'
              AND workspace_id = $2",
            permissioned_as,
            workspace_id
        )
        .fetch_one(&mut tx)
        .await?;

        if let Some(sum_duration_ms) = rate_limiting_duration_ms {
            if sum_duration_ms as u128 > MAX_DURATION_LAST_1200.as_millis() {
                return Err(error::Error::ExecutionErr(format!(
                    "You have exceeded the scripts cumulative duration limit over the last 20m \
                     which is: {} seconds",
                    MAX_DURATION_LAST_1200.as_secs()
                )));
            }
        }
    }

    let (script_hash, script_path, raw_code, job_kind, raw_flow, language) = match job_payload {
        JobPayload::ScriptHash { hash, path } => {
            let language = sqlx::query_scalar!(
                "SELECT language as \"language: ScriptLang\" FROM script WHERE hash = $1 AND \
                 (workspace_id = $2 OR workspace_id = 'starter')",
                hash.0,
                workspace_id
            )
            .fetch_one(&mut tx)
            .await
            .map_err(|e| {
                Error::InternalErr(format!(
                    "fetching language for hash {hash} in {workspace_id}: {e}"
                ))
            })?;
            (
                Some(hash.0),
                Some(path),
                None,
                JobKind::Script,
                None,
                Some(language),
            )
        }
        JobPayload::ScriptHub { path } => {
            let email = sqlx::query_scalar!(
                "SELECT email FROM usr WHERE username = $1 AND workspace_id = $2",
                user,
                workspace_id
            )
            .fetch_optional(&mut tx)
            .await?;
            (
                None,
                Some(path.clone()),
                Some(
                    get_hub_script_by_path(
                        Authed {
                            email,
                            username: user.to_string(),
                            is_admin: false,
                            groups: vec![],
                        },
                        Path(StripPath(path)),
                        Extension(
                            reqwest::ClientBuilder::new()
                                .user_agent("windmill/beta")
                                .build()
                                .map_err(to_anyhow)?,
                        ),
                        Host(std::env::var("BASE_URL").unwrap_or_else(|_| "".to_string())),
                    )
                    .await?,
                ),
                JobKind::Script_Hub,
                None,
                Some(ScriptLang::Deno),
            )
        }
        JobPayload::Code(RawCode { content, path, language }) => (
            None,
            path,
            Some(content),
            JobKind::Preview,
            None,
            Some(language),
        ),
        JobPayload::Dependencies { hash, dependencies } => (
            Some(hash.0),
            None,
            Some(dependencies.join("\n")),
            JobKind::Dependencies,
            None,
            Some(ScriptLang::Python3),
        ),
        JobPayload::RawFlow { value, path } => {
            (None, path, None, JobKind::FlowPreview, Some(value), None)
        }
        JobPayload::Flow(flow) => {
            let value_json = sqlx::query_scalar!(
                "SELECT value FROM flow WHERE path = $1 AND (workspace_id = $2 OR workspace_id = \
                 'starter')",
                flow,
                workspace_id
            )
            .fetch_optional(&mut tx)
            .await?
            .ok_or_else(|| Error::InternalErr(format!("not found flow at path {:?}", flow)))?;
            let value = serde_json::from_value::<FlowValue>(value_json).map_err(|err| {
                Error::InternalErr(format!(
                    "could not convert json to flow for {flow}: {err:?}"
                ))
            })?;
            (None, Some(flow), None, JobKind::Flow, Some(value), None)
        }
    };

    let flow_status = raw_flow.as_ref().map(init_flow_status);
    let uuid = sqlx::query_scalar!(
        "INSERT INTO queue
            (workspace_id, id, parent_job, created_by, permissioned_as, scheduled_for, 
                script_hash, script_path, raw_code, args, job_kind, schedule_path, raw_flow, \
         flow_status, is_flow_step, language)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16) \
         RETURNING id",
        workspace_id,
        job_id,
        parent_job,
        user,
        permissioned_as,
        scheduled_for,
        script_hash,
        script_path.clone(),
        raw_code,
        args_json,
        job_kind: JobKind,
        schedule_path,
        raw_flow.map(|f| serde_json::json!(f)),
        flow_status.map(|f| serde_json::json!(f)),
        is_flow_step,
        language: ScriptLang
    )
    .fetch_one(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("Could not insert into queue {job_id}: {e}")))?;

    {
        let uuid_string = job_id.to_string();
        let uuid_str = uuid_string.as_str();
        let mut hm = HashMap::from([("uuid", uuid_str), ("permissioned_as", &permissioned_as)]);

        let s: String;
        let operation_name = match job_kind {
            JobKind::Preview => "jobs.run.preview",
            JobKind::Script => {
                s = ScriptHash(script_hash.unwrap()).to_string();
                hm.insert("hash", s.as_str());
                "jobs.run.script"
            }
            JobKind::Flow => "jobs.run.flow",
            JobKind::FlowPreview => "jobs.run.flow_preview",
            JobKind::Script_Hub => "jobs.run.script_hub",
            JobKind::Dependencies => "jobs.run.dependencies",
        };

        audit_log(
            &mut tx,
            &user,
            operation_name,
            ActionKind::Execute,
            workspace_id,
            script_path.as_ref().map(|x| x.as_str()),
            Some(hm),
        )
        .await?;
    }
    Ok((uuid, tx))
}

#[instrument(level = "trace", skip_all)]
pub async fn add_completed_job_error<E: ToString + std::fmt::Debug>(
    db: &DB,
    queued_job: &QueuedJob,
    logs: String,
    e: E,
    metrics: &worker::Metrics,
) -> Result<(Uuid, Map<String, Value>), Error> {
    metrics.jobs_failed.inc();
    let mut output_map = serde_json::Map::new();
    output_map.insert(
        "error".to_string(),
        serde_json::Value::String(e.to_string()),
    );
    let a = add_completed_job(
        db,
        &queued_job,
        false,
        false,
        serde_json::Value::Object(output_map.clone()),
        format!("\n{}\n{}", logs, e.to_string()),
    )
    .await?;
    Ok((a, output_map))
}

#[instrument(level = "trace", skip_all)]
pub async fn add_completed_job(
    db: &DB,
    queued_job: &QueuedJob,
    success: bool,
    skipped: bool,
    result: serde_json::Value,
    logs: String,
) -> Result<Uuid, Error> {
    let job_id = queued_job.id.clone();
    sqlx::query!(
        "INSERT INTO completed_job as cj
            (workspace_id, id, parent_job, created_by, created_at, started_at, duration_ms, success, \
            script_hash, script_path, args, result, logs, \
            raw_code, canceled, canceled_by, canceled_reason, job_kind, schedule_path, \
            permissioned_as, flow_status, raw_flow, is_flow_step, is_skipped, language)
            VALUES ($1, $2, $3, $4, $5, $6, EXTRACT(milliseconds FROM (now() - $6)), $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, \
                    $18, $19, $20, $21, $22, $23, $24)
         ON CONFLICT (id) DO UPDATE SET success = $7, result = $11, logs = concat(cj.logs, $12)",
        queued_job.workspace_id,
        queued_job.id,
        queued_job.parent_job,
        queued_job.created_by,
        queued_job.created_at,
        queued_job.started_at,        
        success,
        queued_job.script_hash.map(|x| x.0),
        queued_job.script_path,
        queued_job.args,
        result,
        logs,
        queued_job.raw_code,
        queued_job.canceled,
        queued_job.canceled_by,
        queued_job.canceled_reason,
        queued_job.job_kind: JobKind,
        queued_job.schedule_path,
        queued_job.permissioned_as,
        queued_job.flow_status,
        queued_job.raw_flow,
        queued_job.is_flow_step,
        skipped,
        queued_job.language: ScriptLang,
    )
    .execute(db)
    .await
    .map_err(|e| Error::InternalErr(format!("Could not add completed job {job_id}: {e}")))?;
    tracing::debug!("Added completed job {}", queued_job.id);
    Ok(queued_job.id)
}

#[instrument(level = "trace", skip_all)]
pub async fn postprocess_queued_job(
    is_flow_step: bool,
    schedule_path: Option<String>,
    script_path: Option<String>,
    w_id: &str,
    job_id: Uuid,
    db: &DB,
) -> crate::error::Result<()> {
    let _ = delete_job(db, w_id, job_id).await?;
    if !is_flow_step {
        schedule_again_if_scheduled(schedule_path, script_path, &w_id, db).await?;
    }
    Ok(())
}

#[instrument(level = "trace", skip_all)]
pub async fn schedule_again_if_scheduled(
    schedule_path: Option<String>,
    script_path: Option<String>,
    w_id: &str,
    db: &DB,
) -> crate::error::Result<()> {
    if let Some(schedule_path) = schedule_path {
        let mut tx = db.begin().await?;
        let schedule = get_schedule_opt(&mut tx, &w_id, &schedule_path)
            .await?
            .ok_or_else(|| {
                Error::InternalErr(format!(
                    "Could not find schedule {:?} for workspace {}",
                    schedule_path, w_id
                ))
            })?;
        if schedule.enabled && script_path.is_some() && script_path.unwrap() == schedule.script_path
        {
            tx = crate::schedule::push_scheduled_job(tx, schedule).await?;
        }
        tx.commit().await?;
    }
    Ok(())
}

pub async fn pull(db: &DB) -> Result<Option<QueuedJob>, crate::Error> {
    let job: Option<QueuedJob> = sqlx::query_as::<_, QueuedJob>(
        "UPDATE queue
            SET running = true, started_at = now(), last_ping = now()
            WHERE id IN (
                SELECT id
                FROM queue
                WHERE running = false
                  AND scheduled_for <= now()
                  AND (   suspend = ( SELECT count(*)
                                        FROM resume_job r
                                       WHERE queue.id = r.job )
                       OR suspend_until <= now())
                ORDER BY scheduled_for
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            RETURNING *",
    )
    .fetch_optional(db)
    .await?;

    Ok(job)
}

#[instrument(level = "trace", skip_all)]
pub async fn delete_job(db: &DB, w_id: &str, job_id: Uuid) -> Result<(), crate::Error> {
    let job_removed = sqlx::query_scalar!(
        "DELETE FROM queue WHERE workspace_id = $1 AND id = $2 RETURNING 1",
        w_id,
        job_id
    )
    .fetch_one(db)
    .await
    .map_err(|e| Error::InternalErr(format!("Error during deletion of job {job_id}: {e}")))?
    .unwrap_or(0)
        == 1;
    tracing::debug!("Job {job_id} deletion was achieved with success: {job_removed}");
    Ok(())
}
