/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use anyhow::Context;
use axum::{
    extract::{FromRequest, Path, Query},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use hmac::Mac;
use hyper::StatusCode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sql_builder::{prelude::*, quote, SqlBuilder};
use sqlx::{query_scalar, types::Uuid, Postgres, Transaction};
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    error::{self, to_anyhow, Error},
    flow_status::{Approval, FlowStatus, FlowStatusModule},
    flows::FlowValue,
    oauth2::HmacSha256,
    scripts::{ScriptHash, ScriptLang},
    users::owner_to_token_owner,
    utils::{not_found_if_none, now_from_db, paginate, require_admin, Pagination, StripPath},
};
use windmill_queue::{get_queued_job, push, JobKind, JobPayload, QueuedJob, RawCode};

use crate::{
    db::{UserDB, DB},
    users::Authed,
    variables::get_workspace_key,
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/run/f/*script_path", post(run_flow_by_path))
        .route("/run/p/*script_path", post(run_job_by_path))
        .route(
            "/run_wait_result/p/*script_path",
            post(run_wait_result_job_by_path),
        )
        .route(
            "/run_wait_result/h/:hash",
            post(run_wait_result_job_by_hash),
        )
        .route("/run/h/:hash", post(run_job_by_hash))
        .route("/run/preview", post(run_preview_job))
        .route("/run/preview_flow", post(run_preview_flow_job))
        .route("/list", get(list_jobs))
        .route("/queue/list", get(list_queue_jobs))
        .route("/queue/cancel/:id", post(cancel_job_api))
        .route("/completed/list", get(list_completed_jobs))
        .route("/completed/get/:id", get(get_completed_job))
        .route("/completed/get_result/:id", get(get_completed_job_result))
        .route("/completed/delete/:id", post(delete_completed_job))
        .route("/get/:id", get(get_job))
        .route("/getupdate/:id", get(get_job_update))
        .route(
            "/job_signature/:job_id/:resume_id",
            get(create_job_signature),
        )
        .route("/result_by_id/:job_id/:node_id", get(get_result_by_id))
}

pub fn global_service() -> Router {
    Router::new()
        .route(
            "/resume/:job_id/:resume_id/:secret",
            get(resume_suspended_job),
        )
        .route(
            "/resume/:job_id/:resume_id/:secret",
            post(resume_suspended_job),
        )
        .route(
            "/cancel/:job_id/:resume_id/:secret",
            get(cancel_suspended_job),
        )
        .route(
            "/cancel/:job_id/:resume_id/:secret",
            post(cancel_suspended_job),
        )
        .route(
            "/get_flow/:job_id/:resume_id/:secret",
            get(get_suspended_job_flow),
        )
}

async fn get_result_by_id(
    Extension(db): Extension<DB>,
    Query(ResultByIdQuery { skip_direct }): Query<ResultByIdQuery>,
    Path((w_id, flow_id, node_id)): Path<(String, String, String)>,
) -> windmill_common::error::JsonResult<serde_json::Value> {
    let res = windmill_queue::get_result_by_id(db, skip_direct, w_id, flow_id, node_id).await?;
    Ok(Json(res))
}

async fn cancel_job_api(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Json(CancelJob { reason }): Json<CancelJob>,
) -> error::Result<String> {
    let tx = user_db.begin(&authed).await?;

    let (mut tx, job_option) =
        windmill_queue::cancel_job(&authed.username, reason, id, &w_id, tx).await?;

    if let Some(id) = job_option {
        audit_log(
            &mut tx,
            &authed.username,
            "jobs.cancel",
            ActionKind::Delete,
            &w_id,
            Some(&id.to_string()),
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(id.to_string())
    } else {
        let (job_o, tx) = get_job_by_id(tx, &w_id, id).await?;
        tx.commit().await?;
        let err = match job_o {
            Some(Job::CompletedJob(_)) => error::Error::BadRequest(format!(
                "queued job id {} exists but is already completed and cannot be canceled",
                id
            )),
            _ => error::Error::NotFound(format!("queued job id {} does not exist", id)),
        };
        Err(err)
    }
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

async fn get_job(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::JsonResult<Job> {
    let tx = db.begin().await?;
    let (job_o, tx) = get_job_by_id(tx, &w_id, id).await?;
    let job = not_found_if_none(job_o, "Job", id.to_string())?;
    tx.commit().await?;
    Ok(Json(job))
}

#[derive(Deserialize)]
pub struct ResultByIdQuery {
    pub skip_direct: bool,
}

pub async fn get_job_by_id<'c>(
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
    if job_option.is_some() {
        Ok((job_option, tx))
    } else {
        // check if a job had been moved in-between queries
        let cjob_option = sqlx::query_as::<_, CompletedJob>(
            "SELECT * FROM completed_job WHERE id = $1 AND workspace_id = $2",
        )
        .bind(id)
        .bind(w_id)
        .fetch_optional(&mut tx)
        .await?;
        Ok((cjob_option.map(Job::CompletedJob), tx))
    }
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct CompletedJob {
    pub workspace_id: String,
    pub id: Uuid,
    pub parent_job: Option<Uuid>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub duration_ms: i32,
    pub success: bool,
    pub script_hash: Option<ScriptHash>,
    pub script_path: Option<String>,
    pub args: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub logs: Option<String>,
    pub deleted: bool,
    pub raw_code: Option<String>,
    pub canceled: bool,
    pub canceled_by: Option<String>,
    pub canceled_reason: Option<String>,
    pub job_kind: JobKind,
    pub schedule_path: Option<String>,
    pub permissioned_as: String,
    pub flow_status: Option<serde_json::Value>,
    pub raw_flow: Option<serde_json::Value>,
    pub is_flow_step: bool,
    pub language: Option<ScriptLang>,
    pub is_skipped: bool,
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
    // TODO: todo!("rewrite this to just run list_queue_jobs and list_completed_jobs separately and return as one");
    let (per_page, offset) = paginate(pagination);
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

pub async fn resume_suspended_job(
    /* unauthed */
    Extension(db): Extension<DB>,
    Path((w_id, job_id, resume_id, secret)): Path<(String, Uuid, u32, String)>,
    QueryOrBody(value): QueryOrBody<serde_json::Value>,
    Query(approver): Query<QueryApprover>,
) -> error::Result<StatusCode> {
    let value = value.unwrap_or(serde_json::Value::Null);
    let mut tx = db.begin().await?;
    let key = get_workspace_key(&w_id, &mut tx).await?;
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).map_err(to_anyhow)?;
    mac.update(job_id.as_bytes());
    mac.update(resume_id.to_be_bytes().as_ref());
    if let Some(approver) = approver.approver.clone() {
        mac.update(approver.as_bytes());
    }
    mac.verify_slice(hex::decode(secret)?.as_ref())
        .map_err(|_| anyhow::anyhow!("Invalid signature"))?;
    let flow = sqlx::query!(
        r#"
        SELECT id, flow_status, suspend
        FROM queue
        WHERE id = ( SELECT parent_job FROM queue WHERE id = $1 UNION ALL SELECT parent_job FROM completed_job WHERE id = $1)
        FOR UPDATE
        "#,
        job_id,
    )
    .fetch_optional(&mut tx)
    .await?
    .ok_or_else(|| anyhow::anyhow!("parent flow job not found"))?;

    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (SELECT 1 FROM resume_job WHERE id = $1)
        "#,
        Uuid::from_u128(job_id.as_u128() ^ resume_id as u128),
    )
    .fetch_one(&mut tx)
    .await?
    .unwrap_or(false);

    if exists {
        return Err(anyhow::anyhow!("resume request already sent").into());
    }

    sqlx::query!(
        r#"
        INSERT INTO resume_job
                    (id, resume_id, job, flow, value, approver)
             VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        Uuid::from_u128(job_id.as_u128() ^ resume_id as u128),
        resume_id as i32,
        job_id,
        flow.id,
        value,
        approver.approver
    )
    .execute(&mut tx)
    .await?;

    /* If the flow is currently waiting to be resumed (`FlowStatusModule::WaitingForEvents`)
     * the suspend column must be set to the number of resume messages waited on.
     *
     * The flow's queue row is locked in this transaction because to avoid race conditions around
     * the suspend column.
     * That is, a job needs one event but it hasn't arrived, a worker counts zero events before
     * entering WaitingForEvents.  Then this message arrives but the job isn't in WaitingForEvents
     * yet so the suspend counter isn't updated.  Then the job enters WaitingForEvents expecting
     * one event to arrive based on the count that is no longer correct. */
    if let Some(suspend) = (0 < flow.suspend).then(|| flow.suspend - 1) {
        let status =
            serde_json::from_value::<FlowStatus>(flow.flow_status.context("no flow status")?)
                .context("deserialize flow status")?;
        if matches!(status.current_step(), Some(FlowStatusModule::WaitingForEvents { job, .. }) if job == &job_id)
        {
            sqlx::query!(
                "UPDATE queue SET suspend = $1 WHERE id = $2",
                suspend,
                flow.id,
            )
            .execute(&mut tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(StatusCode::CREATED)
}

pub async fn cancel_suspended_job(
    /* unauthed */
    Extension(db): Extension<DB>,
    Path((w_id, job, resume_id, secret)): Path<(String, Uuid, u32, String)>,
    Query(approver): Query<QueryApprover>,
) -> error::Result<String> {
    let mut tx = db.begin().await?;
    let key = get_workspace_key(&w_id, &mut tx).await?;
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).map_err(to_anyhow)?;
    mac.update(job.as_bytes());
    mac.update(resume_id.to_be_bytes().as_ref());
    if let Some(approver) = approver.approver.clone() {
        mac.update(approver.as_bytes());
    }
    mac.verify_slice(hex::decode(secret)?.as_ref())
        .map_err(|_| anyhow::anyhow!("Invalid signature"))?;

    let whom = approver.approver.unwrap_or_else(|| "unknown".to_string());
    let parent_flow = get_root_job(db, &w_id, job).await?;
    let (mut tx, job) = windmill_queue::cancel_job(
        &whom,
        Some("approval request disapproved".to_string()),
        parent_flow,
        &w_id,
        tx,
    )
    .await?;
    if job.is_some() {
        audit_log(
            &mut tx,
            &whom,
            "jobs.disapproval",
            ActionKind::Delete,
            &w_id,
            Some(&parent_flow.to_string()),
            None,
        )
        .await?;
    }
    tx.commit().await?;
    Ok("Flow of job cancelled".to_string())
}

pub async fn get_root_job(db: DB, w_id: &str, job: Uuid) -> error::Result<Uuid> {
    let mut tx = db.begin().await?;
    let mut job_id = job;
    loop {
        let (job, ntx) = get_job_by_id(tx, w_id, job_id).await?;
        tx = ntx;
        let p_job = job.and_then(|x| match x {
            Job::QueuedJob(job) => job.parent_job,
            Job::CompletedJob(job) => job.parent_job,
        });
        if let Some(p_job) = p_job {
            job_id = p_job;
        } else {
            return Ok(job_id);
        }
    }
}

#[derive(Serialize)]
pub struct SuspendedJobFlow {
    pub job: Job,
    pub approvers: Vec<Approval>,
}

#[derive(Deserialize)]
pub struct QueryApprover {
    pub approver: Option<String>,
}

pub async fn get_suspended_job_flow(
    /* unauthed */
    Extension(db): Extension<DB>,
    Path((w_id, job, resume_id, secret)): Path<(String, Uuid, u32, String)>,
    Query(approver): Query<QueryApprover>,
) -> error::JsonResult<SuspendedJobFlow> {
    let mut tx = db.begin().await?;
    let key = get_workspace_key(&w_id, &mut tx).await?;
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).map_err(to_anyhow)?;
    mac.update(job.as_bytes());
    mac.update(resume_id.to_be_bytes().as_ref());
    if let Some(approver) = approver.approver {
        mac.update(approver.as_bytes());
    }
    mac.verify_slice(hex::decode(secret)?.as_ref())
        .map_err(|_| anyhow::anyhow!("Invalid signature"))?;
    let flow_id = sqlx::query_scalar!(
        r#"
        SELECT parent_job
        FROM queue
        WHERE id = $1 AND workspace_id = $2
        UNION ALL
        SELECT parent_job
        FROM completed_job
        WHERE id = $1 AND workspace_id = $2
        "#,
        job,
        w_id
    )
    .fetch_optional(&mut tx)
    .await?
    .flatten()
    .ok_or_else(|| anyhow::anyhow!("parent flow job not found"))?;
    let (flow_o, mut tx) = get_job_by_id(tx, &w_id, flow_id).await?;
    let flow = not_found_if_none(flow_o, "Parent Flow", job.to_string())?;

    let flow_status = flow
        .flow_status()
        .ok_or_else(|| anyhow::anyhow!("unable to deserialize the flow"))?;
    let flow_module_status = flow_status
        .modules
        .iter()
        .find(|p| p.job() == Some(job))
        .ok_or_else(|| anyhow::anyhow!("unable to find the module"))?;

    let approvers_from_status = match flow_module_status {
        FlowStatusModule::Success { approvers, .. } => approvers.to_owned(),
        _ => vec![],
    };
    let approvers = if approvers_from_status.is_empty() {
        sqlx::query!(
            r#"
            SELECT resume_id, approver
            FROM resume_job
            WHERE job = $1
            "#,
            job,
        )
        .fetch_all(&mut tx)
        .await?
        .into_iter()
        .map(|x| Approval {
            resume_id: x.resume_id as u16,
            approver: x.approver.unwrap_or_else(|| "anonymous".to_string()),
        })
        .collect()
    } else {
        approvers_from_status
    };

    Ok(Json(SuspendedJobFlow { job: flow, approvers }))
}

pub async fn create_job_signature(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, job_id, resume_id)): Path<(String, Uuid, u32)>,
    Query(approver): Query<QueryApprover>,
) -> error::Result<String> {
    let key = get_workspace_key(&w_id, &mut user_db.begin(&authed).await?).await?;
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).map_err(to_anyhow)?;
    mac.update(job_id.as_bytes());
    mac.update(resume_id.to_be_bytes().as_ref());
    tracing::info!("approver: {:?}", approver.approver);
    if let Some(approver) = approver.approver {
        mac.update(approver.as_bytes());
    }
    Ok(hex::encode(mac.finalize().into_bytes()))
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum Job {
    QueuedJob(QueuedJob),
    CompletedJob(CompletedJob),
}

impl Job {
    pub fn raw_flow(&self) -> Option<FlowValue> {
        let value = match self {
            Job::QueuedJob(job) => job.raw_flow.clone(),
            Job::CompletedJob(job) => job.raw_flow.clone(),
        };
        value.map(|v| serde_json::from_value(v).ok()).flatten()
    }
    pub fn flow_status(&self) -> Option<FlowStatus> {
        let value = match self {
            Job::QueuedJob(job) => job.flow_status.clone(),
            Job::CompletedJob(job) => job.flow_status.clone(),
        };
        value.map(|v| serde_json::from_value(v).ok()).flatten()
    }
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
                raw_lock: None,
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
                same_worker: false,
            }),
            t => panic!("job type {} not valid", t),
        }
    }
}
#[derive(Deserialize)]
struct CancelJob {
    reason: Option<String>,
}

#[derive(Deserialize)]
struct Preview {
    content: String,
    path: Option<String>,
    args: Option<serde_json::Map<String, serde_json::Value>>,
    language: ScriptLang,
}

#[derive(Deserialize)]
struct PreviewFlow {
    value: FlowValue,
    path: Option<String>,
    args: Option<serde_json::Map<String, serde_json::Value>>,
}

pub struct QueryOrBody<D>(pub Option<D>);

#[axum::async_trait]
impl<D, B> FromRequest<B> for QueryOrBody<D>
where
    D: DeserializeOwned,
    B: Send + axum::body::HttpBody,
    <B as axum::body::HttpBody>::Data: Send,
    <B as axum::body::HttpBody>::Error: Into<axum::BoxError>,
{
    type Rejection = Response;

    async fn from_request(
        req: &mut axum::extract::RequestParts<B>,
    ) -> std::result::Result<Self, Self::Rejection> {
        return if req.method() == axum::http::Method::GET {
            let Query(InPayload { payload }) = Query::from_request(req)
                .await
                .map_err(IntoResponse::into_response)?;
            payload
                .map(|p| {
                    decode_payload(p)
                        .map(QueryOrBody)
                        .map_err(|err| (StatusCode::BAD_REQUEST, format!("{err:#?}")))
                        .map_err(IntoResponse::into_response)
                })
                .unwrap_or(Ok(QueryOrBody(None)))
        } else {
            Json::from_request(req)
                .await
                .map(|Json(v)| QueryOrBody(Some(v)))
                .map_err(IntoResponse::into_response)
        };

        #[derive(Deserialize)]
        struct InPayload {
            payload: Option<String>,
        }

        fn decode_payload<D: DeserializeOwned, T: AsRef<[u8]>>(t: T) -> anyhow::Result<D> {
            let vec = base64::decode_config(&t, base64::URL_SAFE).context("invalid base64")?;
            serde_json::from_slice(vec.as_slice()).context("invalid json")
        }
    }
}
pub async fn run_flow_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    axum::Json(args): axum::Json<Option<serde_json::Map<String, serde_json::Value>>>,
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
    axum::Json(args): axum::Json<Option<serde_json::Map<String, serde_json::Value>>>,
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
        false,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

async fn run_wait_result<T>(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    uuid: Uuid,
    Path((w_id, _)): Path<(String, T)>,
) -> error::JsonResult<serde_json::Value> {
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

pub async fn run_wait_result_job_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    axum::Json(args): axum::Json<Option<serde_json::Map<String, serde_json::Value>>>,
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
        false,
    )
    .await?;
    tx.commit().await?;

    run_wait_result(authed, Extension(user_db), uuid, Path((w_id, script_path))).await
}

pub async fn run_wait_result_job_by_hash(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_hash)): Path<(String, ScriptHash)>,
    axum::Json(args): axum::Json<Option<serde_json::Map<String, serde_json::Value>>>,
    Query(run_query): Query<RunJobQuery>,
) -> error::JsonResult<serde_json::Value> {
    let hash = script_hash.0;
    let mut tx = user_db.clone().begin(&authed).await?;
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
        false,
    )
    .await?;
    tx.commit().await?;

    run_wait_result(authed, Extension(user_db), uuid, Path((w_id, script_hash))).await
}

// a similar function exists on the worker
pub async fn script_path_to_payload<'c>(
    script_path: &str,
    db: &mut Transaction<'c, Postgres>,
    w_id: &String,
) -> std::result::Result<JobPayload, Error> {
    let job_payload = if script_path.starts_with("hub/") {
        JobPayload::ScriptHub { path: script_path.to_owned() }
    } else {
        let script_hash = windmill_common::get_latest_hash_for_path(db, w_id, script_path).await?;
        JobPayload::ScriptHash { hash: script_hash, path: script_path.to_owned() }
    };
    Ok(job_payload)
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
            lock: None,
        }),
        preview.args,
        &authed.username,
        owner_to_token_owner(&authed.username, false),
        scheduled_for,
        None,
        None,
        false,
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
        false,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

pub async fn run_job_by_hash(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_hash)): Path<(String, ScriptHash)>,
    axum::Json(args): axum::Json<Option<serde_json::Map<String, serde_json::Value>>>,
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
        false,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
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
        let logs = not_found_if_none(logs, "Job", id.to_string())?;
        tx.commit().await?;
        Ok(Json(JobUpdate {
            running: Some(false),
            completed: Some(true),
            new_logs: logs,
        }))
    }
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
async fn list_completed_jobs(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListCompletedQuery>,
) -> error::JsonResult<Vec<CompletedJob>> {
    let (per_page, offset) = paginate(pagination);

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

    let job = not_found_if_none(job_o, "Completed Job", id.to_string())?;
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

    let result = not_found_if_none(result_o, "Completed Job", id.to_string())?;
    Ok(Json(result))
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

    let job = not_found_if_none(job_o, "Completed Job", id.to_string())?;

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
