/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{
    db::{UserDB, DB},
    users::{check_scopes, require_owner_of_path, Authed, OptAuthed},
    utils::require_super_admin,
    variables::get_workspace_key,
    BASE_URL,
};
use anyhow::Context;
use axum::{
    extract::{FromRequest, Json, Path, Query},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Form, RequestExt, Router,
};
use base64::Engine;
use bytes::Bytes;
use hmac::Mac;
use hyper::{header::CONTENT_TYPE, http, HeaderMap, Request, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sql_builder::{prelude::*, quote, SqlBuilder};
use sqlx::{query_scalar, types::Uuid, FromRow, Postgres, Transaction};
use urlencoding::encode;
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    error::{self, to_anyhow, Error},
    flow_status::{Approval, FlowStatus, FlowStatusModule},
    flows::FlowValue,
    jobs::{script_path_to_payload, JobKind, JobPayload, QueuedJob, RawCode},
    oauth2::HmacSha256,
    scripts::{ScriptHash, ScriptLang},
    users::username_to_permissioned_as,
    utils::{not_found_if_none, now_from_db, paginate, require_admin, Pagination, StripPath},
};
use windmill_queue::{get_queued_job, push, QueueTransaction};

pub fn workspaced_service() -> Router {
    Router::new()
        .route(
            "/run/f/*script_path",
            post(run_flow_by_path).head(|| async { "" }),
        )
        .route(
            "/run/p/*script_path",
            post(run_job_by_path).head(|| async { "" }),
        )
        .route(
            "/run_wait_result/p/*script_path",
            post(run_wait_result_script_by_path).head(|| async { "" }),
        )
        .route(
            "/run_wait_result/p/*script_path",
            get(run_wait_result_job_by_path_get),
        )
        .route(
            "/run_wait_result/h/:hash",
            post(run_wait_result_script_by_hash).head(|| async { "" }),
        )
        .route(
            "/run_wait_result/f/*script_path",
            post(run_wait_result_flow_by_path).head(|| async { "" }),
        )
        .route(
            "/openai_sync/p/*script_path",
            post(openai_sync_script_by_path).head(|| async { "" }),
        )
        .route(
            "/openai_sync/f/*script_path",
            post(openai_sync_flow_by_path).head(|| async { "" }),
        )
        .route("/run/h/:hash", post(run_job_by_hash).head(|| async { "" }))
        .route("/run/preview", post(run_preview_job))
        .route("/add_noop_jobs/:n", post(add_noop_jobs))
        .route("/run/preview_flow", post(run_preview_flow_job))
        .route("/list", get(list_jobs))
        .route("/queue/list", get(list_queue_jobs))
        .route("/queue/count", get(count_queue_jobs))
        .route("/completed/list", get(list_completed_jobs))
        .route("/completed/get/:id", get(get_completed_job))
        .route("/completed/get_result/:id", get(get_completed_job_result))
        .route(
            "/completed/get_result_maybe/:id",
            get(get_completed_job_result_maybe),
        )
        .route("/completed/delete/:id", post(delete_completed_job))
        .route("/flow/resume/:id", post(resume_suspended_flow_as_owner))
        .route(
            "/job_signature/:job_id/:resume_id",
            get(create_job_signature),
        )
        .route("/resume_urls/:job_id/:resume_id", get(get_resume_urls))
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
        .route("/get/:id", get(get_job))
        .route("/get_logs/:id", get(get_job_logs))
        .route("/completed/get/:id", get(get_completed_job))
        .route("/completed/get_result/:id", get(get_completed_job_result))
        .route(
            "/completed/get_result_maybe/:id",
            get(get_completed_job_result_maybe),
        )
        .route("/getupdate/:id", get(get_job_update))
        .route("/queue/cancel/:id", post(cancel_job_api))
        .route("/queue/force_cancel/:id", post(force_cancel))
}

async fn get_result_by_id(
    Extension(db): Extension<DB>,
    Path((w_id, flow_id, node_id)): Path<(String, Uuid, String)>,
) -> windmill_common::error::JsonResult<serde_json::Value> {
    let res = windmill_queue::get_result_by_id(db, w_id, flow_id, node_id).await?;
    Ok(Json(res))
}

async fn cancel_job_api(
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Json(CancelJob { reason }): Json<CancelJob>,
) -> error::Result<String> {
    let tx = db.begin().await?;

    let username = match opt_authed {
        Some(authed) => authed.username,
        None => "anonymous".to_string(),
    };

    let (mut tx, job_option) =
        windmill_queue::cancel_job(&username, reason, id, &w_id, tx, &db, rsmq, false).await?;

    if let Some(id) = job_option {
        audit_log(
            &mut tx,
            &username,
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
            Some(Job::CompletedJob(_)) => {
                return Ok(format!("queued job id {} is already completed", id))
            }
            _ => error::Error::NotFound(format!("queued job id {} does not exist", id)),
        };
        Err(err)
    }
}

async fn force_cancel(
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Json(CancelJob { reason }): Json<CancelJob>,
) -> error::Result<String> {
    let tx = db.begin().await?;

    let username = match opt_authed {
        Some(authed) => authed.username,
        None => "anonymous".to_string(),
    };

    let (mut tx, job_option) =
        windmill_queue::cancel_job(&username, reason, id, &w_id, tx, &db, rsmq, true).await?;

    if let Some(id) = job_option {
        audit_log(
            &mut tx,
            &username,
            "jobs.force_cancel",
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
            Some(Job::CompletedJob(_)) => {
                return Ok(format!("queued job id {} is already completed", id))
            }
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
        "select path from script where hash = $1 AND workspace_id = $2",
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

pub async fn get_path_tag_and_limits_for_hash<'c>(
    db: &mut Transaction<'c, Postgres>,
    w_id: &str,
    hash: i64,
) -> error::Result<(String, Option<String>, Option<i32>, Option<i32>)> {
    let script = sqlx::query!(
        "select path, tag, concurrent_limit, concurrency_time_window_s from script where hash = $1 AND workspace_id = $2",
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
    Ok((script.path, script.tag, script.concurrent_limit, script.concurrency_time_window_s))
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

async fn get_job_logs(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::Result<String> {
    let text = sqlx::query_scalar!(
        "SELECT logs FROM completed_job WHERE id = $1 AND workspace_id = $2",
        id,
        w_id
    )
    .fetch_optional(&db)
    .await?
    .flatten();
    let text = not_found_if_none(text, "Job Logs", id.to_string())?;
    Ok(text)
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_job: Option<Uuid>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub duration_ms: i32,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_hash: Option<ScriptHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_path: Option<String>,
    pub args: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<String>,
    pub deleted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_code: Option<String>,
    pub canceled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canceled_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canceled_reason: Option<String>,
    pub job_kind: JobKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_path: Option<String>,
    pub permissioned_as: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_status: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_flow: Option<serde_json::Value>,
    pub is_flow_step: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<ScriptLang>,
    pub is_skipped: bool,
    pub email: String,
    pub visible_to_owner: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem_peak: Option<i32>,
    pub tag: String,
}

#[derive(Deserialize, Clone)]
pub struct RunJobQuery {
    scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    scheduled_in_secs: Option<i64>,
    parent_job: Option<Uuid>,
    include_header: Option<String>,
    invisible_to_owner: Option<bool>,
    queue_limit: Option<i64>,
    payload: Option<String>,
    job_id: Option<Uuid>,
}

lazy_static::lazy_static! {
    static ref INCLUDE_HEADERS: Vec<String> = std::env::var("INCLUDE_HEADERS")
        .ok().map(|x| x
        .split(',')
        .map(|s| s.to_string())
        .collect()).unwrap_or_default();
}

impl RunJobQuery {
    async fn get_scheduled_for<'c>(
        &self,
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

    fn add_include_headers(
        &self,
        headers: HeaderMap,
        args: serde_json::Map<String, serde_json::Value>,
    ) -> serde_json::Map<String, serde_json::Value> {
        return add_include_headers(&self.include_header, headers, args);
    }
}

pub fn add_include_headers(
    include_header: &Option<String>,
    headers: HeaderMap,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    let whitelist = include_header
        .as_ref()
        .map(|s| s.split(",").map(|s| s.to_string()).collect::<Vec<_>>())
        .unwrap_or_default();

    whitelist
        .iter()
        .chain(INCLUDE_HEADERS.iter())
        .for_each(|h| {
            if let Some(v) = headers.get(h) {
                args.insert(
                    h.to_string().to_lowercase().replace('-', "_"),
                    serde_json::Value::String(v.to_str().unwrap().to_string()),
                );
            }
        });
    args
}

#[derive(Deserialize)]
pub struct ListQueueQuery {
    pub script_path_start: Option<String>,
    pub script_path_exact: Option<String>,
    pub script_hash: Option<String>,
    pub created_by: Option<String>,
    pub started_before: Option<chrono::DateTime<chrono::Utc>>,
    pub started_after: Option<chrono::DateTime<chrono::Utc>>,
    pub running: Option<bool>,
    pub schedule_path: Option<String>,
    pub parent_job: Option<String>,
    pub order_desc: Option<bool>,
    pub job_kinds: Option<String>,
    pub suspended: Option<bool>,
    // filter by matching a subset of the args using base64 encoded json subset
    pub args: Option<String>,
    pub tag: Option<String>,
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
    if let Some(p) = &lq.schedule_path {
        sqlb.and_where_eq("schedule_path", "?".bind(p));
    }
    if let Some(h) = &lq.script_hash {
        sqlb.and_where_eq("script_hash", "?".bind(h));
    }
    if let Some(cb) = &lq.created_by {
        sqlb.and_where_eq("created_by", "?".bind(cb));
    }
    if let Some(t) = &lq.tag {
        sqlb.and_where_eq("tag", "?".bind(t));
    }
    if let Some(r) = &lq.running {
        sqlb.and_where_eq("running", &r);
    }
    if let Some(pj) = &lq.parent_job {
        sqlb.and_where_eq("parent_job", "?".bind(pj));
    }
    if let Some(dt) = &lq.started_before {
        sqlb.and_where_le("started_at", format!("to_timestamp({})", dt.timestamp()));
    }
    if let Some(dt) = &lq.started_after {
        sqlb.and_where_ge("started_at", format!("to_timestamp({})", dt.timestamp()));
    }

    if let Some(s) = &lq.suspended {
        if *s {
            sqlb.and_where_gt("suspend", 0);
        } else {
            sqlb.and_where_eq("suspend", 0);
        }
    }

    if let Some(jk) = &lq.job_kinds {
        sqlb.and_where_in(
            "job_kind",
            &jk.split(',').into_iter().map(quote).collect::<Vec<_>>(),
        );
    }

    if let Some(args) = &lq.args {
        sqlb.and_where("args @> ?".bind(&args.replace("'", "''")));
    }

    sqlb
}

#[derive(Serialize, FromRow)]
struct ListableQueuedJob {
    pub id: Uuid,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub scheduled_for: chrono::DateTime<chrono::Utc>,
    pub script_hash: Option<ScriptHash>,
    pub script_path: Option<String>,
    pub args: Option<serde_json::Value>,
    pub job_kind: JobKind,
    pub schedule_path: Option<String>,
    pub is_flow_step: bool,
    pub language: Option<ScriptLang>,
    pub email: String,
    pub suspend: Option<i32>,
}

async fn list_queue_jobs(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(lq): Query<ListQueueQuery>,
) -> error::JsonResult<Vec<ListableQueuedJob>> {
    let sql = list_queue_jobs_query(
        &w_id,
        &lq,
        &[
            "id",
            "created_by",
            "created_at",
            "started_at",
            "scheduled_for",
            "script_hash",
            "script_path",
            "args",
            "job_kind",
            "schedule_path",
            "permissioned_as",
            "is_flow_step",
            "language",
            "same_worker",
            "email",
            "suspend",
        ],
    )
    .sql()?;
    let jobs = sqlx::query_as::<_, ListableQueuedJob>(&sql)
        .fetch_all(&db)
        .await?;
    Ok(Json(jobs))
}

#[derive(Serialize, Debug, FromRow)]
struct QueueStats {
    database_length: i64,
}

async fn count_queue_jobs(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> error::JsonResult<QueueStats> {
    Ok(Json(
        sqlx::query_as!(
            QueueStats,
            "SELECT coalesce(COUNT(*), 0) as \"database_length!\" FROM queue WHERE workspace_id = $1",
            w_id
        )
        .fetch_one(&db)
        .await?,
    ))
}

async fn list_jobs(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListCompletedQuery>,
) -> error::JsonResult<Vec<Job>> {
    check_scopes(&authed, || format!("listjobs"))?;
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
            started_before: lq.started_before,
            started_after: lq.started_after,
            running: None,
            parent_job: lq.parent_job,
            order_desc: Some(true),
            job_kinds: lq.job_kinds,
            suspended: lq.suspended,
            args: lq.args,
            tag: lq.tag,
            schedule_path: lq.schedule_path,
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
            "null as args",
            "null as duration_ms",
            "null as success",
            "false as deleted",
            "canceled",
            "canceled_by",
            "job_kind",
            "schedule_path",
            "permissioned_as",
            "is_flow_step",
            "language",
            "false as is_skipped",
            "email",
            "visible_to_owner",
            "suspend",
            "mem_peak",
            "tag",
            "concurrent_limit",
            "concurrency_time_window_s",
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
            "null as args",
            "duration_ms",
            "success",
            "deleted",
            "canceled",
            "canceled_by",
            "job_kind",
            "schedule_path",
            "permissioned_as",
            "is_flow_step",
            "language",
            "is_skipped",
            "email",
            "visible_to_owner",
            "null as suspend",
            "mem_peak",
            "tag",
            "null as concurrent_limit",
            "null as concurrency_time_window_s",
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

pub async fn resume_suspended_flow_as_owner(
    authed: Authed,
    Extension(db): Extension<DB>,
    Path((_w_id, flow_id)): Path<(String, Uuid)>,
    QueryOrBody(value): QueryOrBody<serde_json::Value>,
) -> error::Result<StatusCode> {
    check_scopes(&authed, || format!("resumeflow"))?;
    let value = value.unwrap_or(serde_json::Value::Null);
    let mut tx = db.begin().await?;

    let (flow, job_id) = get_suspended_flow_info(flow_id, &mut tx).await?;

    require_owner_of_path(
        &authed,
        &flow.script_path.clone().unwrap_or_else(|| String::new()),
    )?;

    insert_resume_job(0, job_id, &flow, value, Some(authed.username), &mut tx).await?;

    resume_immediately_if_relevant(flow, job_id, &mut tx).await?;

    tx.commit().await?;
    Ok(StatusCode::CREATED)
}

pub async fn resume_suspended_job(
    /* unauthed */
    Extension(db): Extension<DB>,
    Path((w_id, job_id, resume_id, secret)): Path<(String, Uuid, u32, String)>,
    Query(approver): Query<QueryApprover>,
    QueryOrBody(value): QueryOrBody<serde_json::Value>,
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
    let flow = get_suspended_parent_flow_info(job_id, &mut tx).await?;

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

    insert_resume_job(resume_id, job_id, &flow, value, approver.approver, &mut tx).await?;

    resume_immediately_if_relevant(flow, job_id, &mut tx).await?;

    tx.commit().await?;
    Ok(StatusCode::CREATED)
}

/* If the flow is currently waiting to be resumed (`FlowStatusModule::WaitingForEvents`)
 * the suspend column must be set to the number of resume messages waited on.
 *
 * The flow's queue row is locked in this transaction because to avoid race conditions around
 * the suspend column.
 * That is, a job needs one event but it hasn't arrived, a worker counts zero events before
 * entering WaitingForEvents.  Then this message arrives but the job isn't in WaitingForEvents
 * yet so the suspend counter isn't updated.  Then the job enters WaitingForEvents expecting
 * one event to arrive based on the count that is no longer correct. */
async fn resume_immediately_if_relevant<'c>(
    flow: FlowInfo,
    job_id: Uuid,
    tx: &mut Transaction<'c, Postgres>,
) -> error::Result<()> {
    Ok(
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
                .execute(tx)
                .await?;
            }
        },
    )
}

async fn insert_resume_job<'c>(
    resume_id: u32,
    job_id: Uuid,
    flow: &FlowInfo,
    value: serde_json::Value,
    approver: Option<String>,
    tx: &mut Transaction<'c, Postgres>,
) -> error::Result<()> {
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
        approver
    )
    .execute(tx)
    .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct FlowInfo {
    id: Uuid,
    flow_status: Option<serde_json::Value>,
    suspend: i32,
    script_path: Option<String>,
}

async fn get_suspended_parent_flow_info<'c>(
    job_id: Uuid,
    tx: &mut Transaction<'c, Postgres>,
) -> error::Result<FlowInfo> {
    let flow = sqlx::query_as!(
        FlowInfo,
        r#"
        SELECT id, flow_status, suspend, script_path
        FROM queue
        WHERE id = ( SELECT parent_job FROM queue WHERE id = $1 UNION ALL SELECT parent_job FROM completed_job WHERE id = $1)
        FOR UPDATE
        "#,
        job_id,
    )
    .fetch_optional(tx)
    .await?
    .ok_or_else(|| anyhow::anyhow!("parent flow job not found"))?;
    Ok(flow)
}

async fn get_suspended_flow_info<'c>(
    job_id: Uuid,
    tx: &mut Transaction<'c, Postgres>,
) -> error::Result<(FlowInfo, Uuid)> {
    let flow = sqlx::query_as!(
        FlowInfo,
        r#"
        SELECT id, flow_status, suspend, script_path
        FROM queue
        WHERE id = $1
        "#,
        job_id,
    )
    .fetch_optional(tx)
    .await?
    .ok_or_else(|| anyhow::anyhow!("parent flow job not found"))?;
    let job_id = flow
        .flow_status
        .as_ref()
        .and_then(|v| serde_json::from_value::<FlowStatus>(v.clone()).ok())
        .and_then(|s| match s.modules.get(s.step as usize) {
            Some(FlowStatusModule::WaitingForEvents { job, .. }) => Some(job.to_owned()),
            _ => None,
        });

    if let Some(job_id) = job_id {
        Ok((flow, job_id))
    } else {
        Err(anyhow::anyhow!("the flow is not in a suspended state anymore").into())
    }
}

pub async fn cancel_suspended_job(
    /* unauthed */
    Extension(db): Extension<DB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
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
    let parent_flow = get_suspended_parent_flow_info(job, &mut tx).await?.id;

    let (mut tx, cjob) = windmill_queue::cancel_job(
        &whom,
        Some("approval request disapproved".to_string()),
        parent_flow,
        &w_id,
        tx,
        &db,
        rsmq,
        false,
    )
    .await?;
    if cjob.is_some() {
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
        tx.commit().await?;

        Ok(format!("Flow {parent_flow} of job {job} cancelled"))
    } else {
        Ok(format!(
            "Flow {parent_flow} of job {job} was not cancellable"
        ))
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
    create_signature(key, job_id, resume_id, approver.approver)
}

fn create_signature(
    key: String,
    job_id: Uuid,
    resume_id: u32,
    approver: Option<String>,
) -> Result<String, Error> {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).map_err(to_anyhow)?;
    mac.update(job_id.as_bytes());
    mac.update(resume_id.to_be_bytes().as_ref());
    if let Some(approver) = approver {
        mac.update(approver.as_bytes());
    }
    Ok(hex::encode(mac.finalize().into_bytes()))
}

#[allow(non_snake_case)]
#[derive(Serialize)]
pub struct ResumeUrls {
    approvalPage: String,
    cancel: String,
    resume: String,
}

fn build_resume_url(
    op: &str,
    w_id: &str,
    job_id: &Uuid,
    resume_id: &u32,
    signature: &str,
    approver: &str,
    base_url: &str,
) -> String {
    format!("{base_url}/api/w/{w_id}/jobs_u/{op}/{job_id}/{resume_id}/{signature}{approver}")
}

pub async fn get_resume_urls(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, job_id, resume_id)): Path<(String, Uuid, u32)>,
    Query(approver): Query<QueryApprover>,
) -> error::JsonResult<ResumeUrls> {
    let key = get_workspace_key(&w_id, &mut user_db.begin(&authed).await?).await?;
    let signature = create_signature(key, job_id, resume_id, approver.approver.clone())?;
    let approver = approver
        .approver
        .as_ref()
        .map(|x| format!("?approver={}", encode(x)))
        .unwrap_or_else(String::new);

    let base_url = BASE_URL.as_str();
    let res = ResumeUrls {
        approvalPage: format!(
            "{base_url}/approve/{w_id}/{job_id}/{resume_id}/{signature}{approver}"
        ),
        cancel: build_resume_url(
            "cancel", &w_id, &job_id, &resume_id, &signature, &approver, &base_url,
        ),
        resume: build_resume_url(
            "resume", &w_id, &job_id, &resume_id, &signature, &approver, &base_url,
        ),
    };

    Ok(Json(res))
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
    is_flow_step: bool,
    language: Option<ScriptLang>,
    is_skipped: bool,
    email: String,
    visible_to_owner: bool,
    suspend: Option<i32>,
    mem_peak: Option<i32>,
    tag: String,
    concurrent_limit: Option<i32>,
    concurrency_time_window_s: Option<i32>,
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
                flow_status: None,
                deleted: uj.deleted,
                canceled: uj.canceled,
                canceled_by: uj.canceled_by,
                raw_code: None,
                canceled_reason: None,
                job_kind: uj.job_kind,
                schedule_path: uj.schedule_path,
                permissioned_as: uj.permissioned_as,
                raw_flow: None,
                is_flow_step: uj.is_flow_step,
                language: uj.language,
                is_skipped: uj.is_skipped,
                email: uj.email,
                visible_to_owner: uj.visible_to_owner,
                mem_peak: uj.mem_peak,
                tag: uj.tag,
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
                flow_status: None,
                raw_code: None,
                raw_lock: None,
                canceled: uj.canceled,
                canceled_by: uj.canceled_by,
                canceled_reason: None,
                last_ping: None,
                job_kind: uj.job_kind,
                schedule_path: uj.schedule_path,
                permissioned_as: uj.permissioned_as,
                raw_flow: None,
                is_flow_step: uj.is_flow_step,
                language: uj.language,
                same_worker: false,
                pre_run_error: None,
                email: uj.email,
                visible_to_owner: uj.visible_to_owner,
                suspend: uj.suspend,
                mem_peak: uj.mem_peak,
                root_job: None,
                leaf_jobs: None,
                tag: uj.tag,
                concurrent_limit: uj.concurrent_limit,
                concurrency_time_window_s: uj.concurrency_time_window_s,
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
#[serde(rename_all = "lowercase")]
enum PreviewKind {
    Code,
    Identity,
    Http,
    Noop,
}
#[derive(Deserialize)]
struct Preview {
    content: Option<String>,
    kind: Option<PreviewKind>,
    path: Option<String>,
    args: Option<serde_json::Map<String, serde_json::Value>>,
    language: Option<ScriptLang>,
    tag: Option<String>,
}

#[derive(Deserialize)]
struct PreviewFlow {
    value: FlowValue,
    path: Option<String>,
    args: Option<serde_json::Map<String, serde_json::Value>>,
}

pub struct JsonOrForm(
    Option<serde_json::Map<String, serde_json::Value>>,
    Option<String>,
);

#[axum::async_trait]
impl<S> FromRequest<S, axum::body::Body> for JsonOrForm
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(
        req: Request<axum::body::Body>,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let content_type_header = req.headers().get(CONTENT_TYPE);
        let content_type = content_type_header.and_then(|value| value.to_str().ok());
        if let Some(content_type) = content_type {
            if content_type.starts_with("application/json") {
                if req
                    .uri()
                    .query()
                    .map(|x| x.contains("raw=true"))
                    .unwrap_or(false)
                {
                    let bytes = Bytes::from_request(req, _state)
                        .await
                        .map_err(IntoResponse::into_response)?;
                    let str = String::from_utf8(bytes.to_vec()).map_err(|e| {
                        Error::BadRequest(format!("invalid utf8: {}", e)).into_response()
                    })?;
                    let payload =
                        serde_json::from_str::<Option<serde_json::Value>>(&str).map_err(|e| {
                            Error::BadRequest(format!("invalid json: {}", e)).into_response()
                        })?;
                    return match payload {
                        Some(serde_json::Value::Object(map)) => Ok(Self(Some(map), Some(str))),
                        None => Ok(Self(None, Some(str))),
                        Some(x) => {
                            let mut map = serde_json::Map::new();
                            map.insert("body".to_string(), x);
                            Ok(Self(Some(map), Some(str)))
                        }
                    };
                } else {
                    let Json(payload): Json<Option<serde_json::Value>> =
                        req.extract().await.map_err(IntoResponse::into_response)?;
                    return match payload {
                        Some(serde_json::Value::Object(map)) => Ok(Self(Some(map), None)),
                        None => Ok(Self(None, None)),
                        Some(x) => {
                            let mut map = serde_json::Map::new();
                            map.insert("body".to_string(), x);
                            Ok(Self(Some(map), None))
                        }
                    };
                }
            }

            if content_type.starts_with("application/x-www-form-urlencoded") {
                let Form(payload) = req.extract().await.map_err(IntoResponse::into_response)?;
                return Ok(Self(Some(payload), None));
            }
        }

        Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response())
    }
}
pub struct QueryOrBody<D>(pub Option<D>);

#[axum::async_trait]
impl<S, D> FromRequest<S, axum::body::Body> for QueryOrBody<D>
where
    D: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(
        req: Request<axum::body::Body>,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        return if req.method() == axum::http::Method::GET {
            let Query(InPayload { payload }) = Query::from_request(req, state)
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
            Json::from_request(req, state)
                .await
                .map(|Json(v)| QueryOrBody(Some(v)))
                .map_err(IntoResponse::into_response)
        };

        #[derive(Deserialize)]
        struct InPayload {
            payload: Option<String>,
        }
    }
}

fn decode_payload<D: DeserializeOwned>(t: String) -> anyhow::Result<D> {
    let vec = base64::engine::general_purpose::URL_SAFE
        .decode(t)
        .context("invalid base64")?;
    serde_json::from_slice(vec.as_slice()).context("invalid json")
}

fn add_raw_string(
    raw_string: Option<String>,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    if let Some(raw_string) = raw_string {
        args.insert(
            "raw_string".to_string(),
            serde_json::Value::String(raw_string),
        );
    }
    return args;
}

pub async fn run_flow_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    headers: HeaderMap,
    JsonOrForm(args, raw_string): JsonOrForm,
) -> error::Result<(StatusCode, String)> {
    let flow_path = flow_path.to_path();
    check_scopes(&authed, || format!("run:flow/{flow_path}"))?;

    let mut tx: QueueTransaction<'_, _> = (rsmq, user_db.begin(&authed).await?).into();
    let scheduled_for = run_query.get_scheduled_for(tx.transaction_mut()).await?;
    let args = run_query.add_include_headers(headers, args.unwrap_or_default());
    let args = add_raw_string(raw_string, args);
    let (uuid, tx) = push(
        tx,
        &w_id,
        JobPayload::Flow(flow_path.to_string()),
        args,
        &authed.username,
        &authed.email,
        username_to_permissioned_as(&authed.username),
        scheduled_for,
        None,
        run_query.parent_job,
        run_query.parent_job,
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

pub async fn run_job_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    headers: HeaderMap,
    JsonOrForm(args, raw_string): JsonOrForm,
) -> error::Result<(StatusCode, String)> {
    let script_path = script_path.to_path();
    check_scopes(&authed, || format!("run:script/{script_path}"))?;

    let mut tx: QueueTransaction<'_, _> = (rsmq, user_db.begin(&authed).await?).into();
    let (job_payload, tag) =
        script_path_to_payload(script_path, tx.transaction_mut(), &w_id).await?;
    let scheduled_for = run_query.get_scheduled_for(tx.transaction_mut()).await?;
    let args = run_query.add_include_headers(headers, args.unwrap_or_default());
    let args = add_raw_string(raw_string, args);

    let (uuid, tx) = push(
        tx,
        &w_id,
        job_payload,
        args,
        &authed.username,
        &authed.email,
        username_to_permissioned_as(&authed.username),
        scheduled_for,
        None,
        run_query.parent_job,
        run_query.parent_job,
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

struct Guard {
    done: bool,
    id: Uuid,
    w_id: String,
    db: UserDB,
    authed: Authed,
}

impl Drop for Guard {
    fn drop(&mut self) {
        if !&self.done {
            let id = self.id;
            let username = self.authed.username.clone();
            let w_id = self.w_id.clone();
            let db = self.db.clone();
            let authed = self.authed.clone();

            tracing::info!("http connection broke, marking job {id} as canceled");
            tokio::spawn(async move {
                let tx = db.begin(&authed).await.ok();
                if let Some(mut tx) = tx {
                    let _ = sqlx::query!(
                "UPDATE queue SET canceled = true, canceled_reason = 'http connection broke', canceled_by = $1 WHERE id = $2 AND workspace_id = $3",
                username,
                id,
                w_id
            )
            .execute(&mut tx)
            .await;
                    let _ = tx.commit().await;
                }
            });
        }
    }
}

async fn run_wait_result<T>(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    timeout: i32,
    uuid: Uuid,
    Path((w_id, _)): Path<(String, T)>,
) -> error::JsonResult<serde_json::Value> {
    let mut result;
    let timeout_ms = if timeout <= 0 {
        2000
    } else {
        (timeout * 1000) as u64
    };

    let mut g = Guard {
        done: false,
        id: uuid,
        w_id: w_id.clone(),
        db: user_db.clone(),
        authed: authed.clone(),
    };

    let fast_poll_duration = *WAIT_RESULT_FAST_POLL_DURATION_SECS as u64 * 1000;
    let mut accumulated_delay = 0 as u64;
    loop {
        let mut tx = user_db.clone().begin(&authed).await?;
        result = sqlx::query_scalar!(
            "SELECT result FROM completed_job WHERE id = $1 AND workspace_id = $2",
            uuid,
            &w_id
        )
        .fetch_optional(&mut tx)
        .await?
        .flatten();
        drop(tx);

        if result.is_some() {
            break;
        }

        let delay = if accumulated_delay <= fast_poll_duration {
            *WAIT_RESULT_FAST_POLL_INTERVAL_MS
        } else {
            *WAIT_RESULT_SLOW_POLL_INTERVAL_MS
        };
        accumulated_delay += delay;
        if accumulated_delay > timeout_ms {
            break;
        };
        tokio::time::sleep(core::time::Duration::from_millis(delay)).await;
    }
    if let Some(result) = result {
        g.done = true;
        let status_code = result
            .get("windmill_status_code")
            .and_then(|x| x.as_i64())
            .and_then(|x| StatusCode::from_u16(x as u16).ok());
        if let Some(status_code) = status_code {
            return Err(Error::CustomStatusCode(status_code, result));
        }
        Ok(Json(result))
    } else {
        Err(Error::ExecutionErr(format!("timeout after {}s", timeout)))
    }
}

pub async fn check_queue_too_long(db: DB, queue_limit: Option<i64>) -> error::Result<()> {
    if let Some(limit) = queue_limit {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM queue WHERE  canceled = false AND (scheduled_for <= now()
        OR (suspend_until IS NOT NULL
            AND (   suspend <= 0
                 OR suspend_until <= now())))",
        )
        .fetch_one(&db)
        .await?
        .unwrap_or(0);

        if count > queue_limit.unwrap() {
            return Err(Error::InternalErr(format!(
                "Number of queued job is too high: {count} > {limit}"
            )));
        }
    }
    Ok(())
}

lazy_static::lazy_static! {
    pub static ref QUEUE_LIMIT_WAIT_RESULT: Option<i64> = std::env::var("QUEUE_LIMIT_WAIT_RESULT")
        .ok()
        .and_then(|x| x.parse().ok());
    pub static ref TIMEOUT_WAIT_RESULT: i32 = std::env::var("TIMEOUT_WAIT_RESULT")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(20);
    pub static ref WAIT_RESULT_FAST_POLL_INTERVAL_MS: u64 = std::env::var("WAIT_RESULT_FAST_POLL_INTERVAL_MS")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(50);
    pub static ref WAIT_RESULT_FAST_POLL_DURATION_SECS: u16 = std::env::var("WAIT_RESULT_FAST_POLL_DURATION_SECS")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(2);
    pub static ref WAIT_RESULT_SLOW_POLL_INTERVAL_MS: u64 = std::env::var("WAIT_RESULT_SLOW_POLL_INTERVAL_MS")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(200);
}

pub async fn run_wait_result_job_by_path_get(
    method: hyper::http::Method,
    authed: Authed,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
) -> error::JsonResult<serde_json::Value> {
    if method == http::Method::HEAD {
        return Ok(Json(serde_json::json!("")));
    }
    let payload_r = run_query
        .payload
        .map(decode_payload)
        .map(|x| x.map_err(|e| Error::InternalErr(e.to_string())));

    let args = if let Some(payload) = payload_r {
        payload?
    } else {
        serde_json::Map::new()
    };

    check_queue_too_long(db, QUEUE_LIMIT_WAIT_RESULT.or(run_query.queue_limit)).await?;
    let script_path = script_path.to_path();
    check_scopes(&authed, || format!("run:script/{script_path}"))?;

    let mut tx: QueueTransaction<'_, _> = (rsmq, user_db.clone().begin(&authed).await?).into();
    let (job_payload, tag) =
        script_path_to_payload(script_path, tx.transaction_mut(), &w_id).await?;

    let (uuid, tx) = push(
        tx,
        &w_id,
        job_payload,
        args,
        &authed.username,
        &authed.email,
        username_to_permissioned_as(&authed.username),
        None,
        None,
        run_query.parent_job,
        run_query.parent_job,
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
    )
    .await?;
    tx.commit().await?;

    run_wait_result(
        authed,
        Extension(user_db),
        *TIMEOUT_WAIT_RESULT,
        uuid,
        Path((w_id, script_path)),
    )
    .await
}

pub async fn run_wait_result_script_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(db): Extension<DB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    headers: HeaderMap,
    JsonOrForm(args, raw_string): JsonOrForm,
) -> error::JsonResult<serde_json::Value> {
    run_wait_result_script_by_path_internal(
        db,
        run_query,
        script_path,
        authed,
        rsmq,
        user_db,
        w_id,
        headers,
        args,
        raw_string,
    )
    .await
}

fn convert_from_openai_json(
    json: Option<serde_json::Map<String, serde_json::Value>>,
) -> error::Result<Option<serde_json::Map<String, serde_json::Value>>> {
    if let Some(m) = json {
        let mut new_json = serde_json::Map::new();
        let input_keys = m
            .get("inputKeys")
            .and_then(|x| x.as_array())
            .map(|x| x.to_owned())
            .unwrap_or_default();
        let input_values = m
            .get("inputValues")
            .and_then(|x| x.as_array())
            .map(|x| x.to_owned())
            .unwrap_or_default();
        for (k, v) in input_keys.into_iter().zip(input_values.into_iter()) {
            new_json.insert(k.as_str().unwrap_or_else(|| "invalid_key").to_string(), v);
        }
        Ok(Some(new_json))
    } else {
        Ok(None)
    }
}

pub async fn openai_sync_script_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(db): Extension<DB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    headers: HeaderMap,
    JsonOrForm(args, raw_string): JsonOrForm,
) -> error::JsonResult<serde_json::Value> {
    run_wait_result_script_by_path_internal(
        db,
        run_query,
        script_path,
        authed,
        rsmq,
        user_db,
        w_id,
        headers,
        convert_from_openai_json(args)?,
        raw_string,
    )
    .await
}

async fn run_wait_result_script_by_path_internal(
    db: sqlx::Pool<Postgres>,
    run_query: RunJobQuery,
    script_path: StripPath,
    authed: Authed,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    user_db: UserDB,
    w_id: String,
    headers: HeaderMap,
    args: Option<serde_json::Map<String, serde_json::Value>>,
    raw_string: Option<String>,
) -> Result<Json<serde_json::Value>, Error> {
    check_queue_too_long(db, QUEUE_LIMIT_WAIT_RESULT.or(run_query.queue_limit)).await?;
    let script_path = script_path.to_path();
    check_scopes(&authed, || format!("run:script/{script_path}"))?;

    let mut tx: QueueTransaction<'_, _> = (rsmq, user_db.clone().begin(&authed).await?).into();
    let (job_payload, tag) =
        script_path_to_payload(script_path, tx.transaction_mut(), &w_id).await?;

    let args = run_query.add_include_headers(headers, args.unwrap_or_default());
    let args = add_raw_string(raw_string, args);

    let (uuid, tx) = push(
        tx,
        &w_id,
        job_payload,
        args,
        &authed.username,
        &authed.email,
        username_to_permissioned_as(&authed.username),
        None,
        None,
        run_query.parent_job,
        run_query.parent_job,
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
    )
    .await?;
    tx.commit().await?;

    run_wait_result(
        authed,
        Extension(user_db),
        *TIMEOUT_WAIT_RESULT,
        uuid,
        Path((w_id, script_path)),
    )
    .await
}

pub async fn run_wait_result_script_by_hash(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(db): Extension<DB>,
    Path((w_id, script_hash)): Path<(String, ScriptHash)>,
    Query(run_query): Query<RunJobQuery>,
    headers: HeaderMap,
    JsonOrForm(args, raw_string): JsonOrForm,
) -> error::JsonResult<serde_json::Value> {
    check_queue_too_long(db, run_query.queue_limit).await?;

    let hash = script_hash.0;
    let mut tx: QueueTransaction<'_, _> = (rsmq, user_db.clone().begin(&authed).await?).into();
    let (path, tag, concurrent_limit, concurrency_time_window_s) = get_path_tag_and_limits_for_hash(tx.transaction_mut(), &w_id, hash).await?;
    check_scopes(&authed, || format!("run:script/{path}"))?;

    let args = run_query.add_include_headers(headers, args.unwrap_or_default());
    let args = add_raw_string(raw_string, args);

    let (uuid, tx) = push(
        tx,
        &w_id,
        JobPayload::ScriptHash { hash: ScriptHash(hash), path: path, concurrent_limit: concurrent_limit, concurrency_time_window_s: concurrency_time_window_s},
        args,
        &authed.username,
        &authed.email,
        username_to_permissioned_as(&authed.username),
        None,
        None,
        run_query.parent_job,
        run_query.parent_job,
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
    )
    .await?;
    tx.commit().await?;

    run_wait_result(
        authed,
        Extension(user_db),
        *TIMEOUT_WAIT_RESULT,
        uuid,
        Path((w_id, script_hash)),
    )
    .await
}

pub async fn openai_sync_flow_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(db): Extension<DB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    headers: HeaderMap,
    JsonOrForm(args, raw_string): JsonOrForm,
) -> error::JsonResult<serde_json::Value> {
    run_wait_result_flow_by_path_internal(
        db,
        run_query,
        flow_path,
        authed,
        rsmq,
        user_db,
        headers,
        convert_from_openai_json(args)?,
        raw_string,
        w_id,
    )
    .await
}

pub async fn run_wait_result_flow_by_path(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(db): Extension<DB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    headers: HeaderMap,
    JsonOrForm(args, raw_string): JsonOrForm,
) -> error::JsonResult<serde_json::Value> {
    run_wait_result_flow_by_path_internal(
        db, run_query, flow_path, authed, rsmq, user_db, headers, args, raw_string, w_id,
    )
    .await
}

async fn run_wait_result_flow_by_path_internal(
    db: sqlx::Pool<Postgres>,
    run_query: RunJobQuery,
    flow_path: StripPath,
    authed: Authed,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    user_db: UserDB,
    headers: HeaderMap,
    args: Option<serde_json::Map<String, serde_json::Value>>,
    raw_string: Option<String>,
    w_id: String,
) -> Result<Json<serde_json::Value>, Error> {
    check_queue_too_long(db, run_query.queue_limit).await?;

    let flow_path = flow_path.to_path();
    check_scopes(&authed, || format!("run:flow/{flow_path}"))?;

    let mut tx: QueueTransaction<'_, _> = (rsmq, user_db.clone().begin(&authed).await?).into();
    let scheduled_for = run_query.get_scheduled_for(tx.transaction_mut()).await?;
    let args = run_query.add_include_headers(headers, args.unwrap_or_default());
    let args = add_raw_string(raw_string, args);

    let (uuid, tx) = push(
        tx,
        &w_id,
        JobPayload::Flow(flow_path.to_string()),
        args,
        &authed.username,
        &authed.email,
        username_to_permissioned_as(&authed.username),
        scheduled_for,
        None,
        run_query.parent_job,
        run_query.parent_job,
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        None,
    )
    .await?;
    tx.commit().await?;

    run_wait_result(
        authed,
        Extension(user_db),
        *TIMEOUT_WAIT_RESULT,
        uuid,
        Path((w_id, flow_path)),
    )
    .await
}

async fn run_preview_job(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path(w_id): Path<String>,
    Query(run_query): Query<RunJobQuery>,
    headers: HeaderMap,
    Json(preview): Json<Preview>,
) -> error::Result<(StatusCode, String)> {
    check_scopes(&authed, || format!("runscript"))?;
    let mut tx: QueueTransaction<'_, _> = (rsmq, user_db.begin(&authed).await?).into();
    let scheduled_for = run_query.get_scheduled_for(tx.transaction_mut()).await?;
    let args = run_query.add_include_headers(headers, preview.args.unwrap_or_default());

    let (uuid, tx) = push(
        tx,
        &w_id,
        match preview.kind {
            Some(PreviewKind::Identity) => JobPayload::Identity,
            Some(PreviewKind::Http) => JobPayload::Http,
            Some(PreviewKind::Noop) => JobPayload::Noop,
            _ => JobPayload::Code(RawCode {
                content: preview.content.unwrap_or_default(),
                path: preview.path,
                language: preview.language.unwrap_or(ScriptLang::Deno),
                lock: None,
                concurrent_limit: None, // TODO(gbouv): once I find out how to store limits in the content of a script, should be easy to plug limits here
                concurrency_time_window_s: None, // TODO(gbouv): same as above
            }),
        },
        args,
        &authed.username,
        &authed.email,
        username_to_permissioned_as(&authed.username),
        scheduled_for,
        None,
        None,
        None,
        run_query.job_id,
        false,
        false,
        None,
        true,
        preview.tag,
    )
    .await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, uuid.to_string()))
}

async fn add_noop_jobs(
    authed: Authed,
    Extension(db): Extension<DB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path((w_id, n)): Path<(String, i32)>,
) -> error::JsonResult<Vec<String>> {
    require_super_admin(&mut db.begin().await?, &authed.email).await?;
    let mut tx: QueueTransaction<'_, _> = (rsmq, db.begin().await?).into();

    let mut uuids: Vec<String> = Vec::new();
    for _ in 0..n {
        let (uuid, ntx) = push(
            tx,
            &w_id,
            JobPayload::Noop,
            serde_json::Map::new(),
            &authed.username,
            &authed.email,
            username_to_permissioned_as(&authed.username),
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            None,
            true,
            None,
        )
        .await?;
        tx = ntx;
        uuids.push(uuid.to_string());
    }
    tx.commit().await?;

    Ok(Json(uuids))
}
async fn run_preview_flow_job(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path(w_id): Path<String>,
    Query(run_query): Query<RunJobQuery>,
    headers: HeaderMap,
    Json(raw_flow): Json<PreviewFlow>,
) -> error::Result<(StatusCode, String)> {
    check_scopes(&authed, || format!("runflow"))?;
    let mut tx: QueueTransaction<'_, _> = (rsmq, user_db.begin(&authed).await?).into();
    let scheduled_for = run_query.get_scheduled_for(tx.transaction_mut()).await?;
    let args = run_query.add_include_headers(headers, raw_flow.args.unwrap_or_default());

    let (uuid, tx) = push(
        tx,
        &w_id,
        JobPayload::RawFlow { value: raw_flow.value, path: raw_flow.path },
        args,
        &authed.username,
        &authed.email,
        username_to_permissioned_as(&authed.username),
        scheduled_for,
        None,
        None,
        None,
        run_query.job_id,
        false,
        false,
        None,
        true,
        None,
    )
    .await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, uuid.to_string()))
}

pub async fn run_job_by_hash(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path((w_id, script_hash)): Path<(String, ScriptHash)>,
    Query(run_query): Query<RunJobQuery>,
    headers: HeaderMap,
    JsonOrForm(args, raw_string): JsonOrForm,
) -> error::Result<(StatusCode, String)> {
    let hash = script_hash.0;
    let mut tx: QueueTransaction<'_, _> = (rsmq, user_db.begin(&authed).await?).into();
    let (path, tag, concurrent_limit, concurrency_time_window_s) = get_path_tag_and_limits_for_hash(tx.transaction_mut(), &w_id, hash).await?;
    check_scopes(&authed, || format!("run:script/{path}"))?;

    let scheduled_for = run_query.get_scheduled_for(tx.transaction_mut()).await?;
    let args = run_query.add_include_headers(headers, args.unwrap_or_default());
    let args = add_raw_string(raw_string, args);

    let (uuid, tx) = push(
        tx,
        &w_id,
        JobPayload::ScriptHash { hash: ScriptHash(hash), path: path, concurrent_limit: concurrent_limit, concurrency_time_window_s: concurrency_time_window_s },
        args,
        &authed.username,
        &authed.email,
        username_to_permissioned_as(&authed.username),
        scheduled_for,
        None,
        run_query.parent_job,
        run_query.parent_job,
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
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
    pub mem_peak: Option<i32>,
}

async fn get_job_update(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Query(JobUpdateQuery { running, log_offset }): Query<JobUpdateQuery>,
) -> error::JsonResult<JobUpdate> {
    let mut tx = db.begin().await?;

    let record = sqlx::query!(
        "SELECT substr(logs, $1) as logs, mem_peak FROM queue WHERE workspace_id = $2 AND id = $3",
        log_offset,
        &w_id,
        &id
    )
    .fetch_optional(&mut tx)
    .await?;

    if let Some(record) = record {
        tx.commit().await?;
        Ok(Json(JobUpdate {
            running: if !running { Some(true) } else { None },
            completed: None,
            new_logs: record.logs,
            mem_peak: record.mem_peak,
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
            mem_peak: record.map(|r| r.mem_peak).flatten(),
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

    if let Some(p) = &lq.schedule_path {
        sqlb.and_where_eq("schedule_path", "?".bind(p));
    }

    if let Some(ps) = &lq.script_path_start {
        sqlb.and_where_like_left("script_path", "?".bind(ps));
    }
    if let Some(p) = &lq.script_path_exact {
        sqlb.and_where_eq("script_path", "?".bind(p));
    }
    if let Some(h) = &lq.script_hash {
        sqlb.and_where_eq("script_hash", "?".bind(h));
    }
    if let Some(t) = &lq.tag {
        sqlb.and_where_eq("tag", "?".bind(t));
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
    if let Some(dt) = &lq.started_before {
        sqlb.and_where_le("started_at", format!("to_timestamp({})", dt.timestamp()));
    }
    if let Some(dt) = &lq.started_after {
        sqlb.and_where_ge("started_at", format!("to_timestamp({})", dt.timestamp()));
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

    if let Some(args) = &lq.args {
        sqlb.and_where("args @> ?".bind(&args.replace("'", "''")));
    }

    if let Some(result) = &lq.result {
        sqlb.and_where("result @> ?".bind(&result.replace("'", "''")));
    }

    sqlb
}
#[derive(Deserialize, Clone)]
pub struct ListCompletedQuery {
    pub script_path_start: Option<String>,
    pub script_path_exact: Option<String>,
    pub script_hash: Option<String>,
    pub created_by: Option<String>,
    pub started_before: Option<chrono::DateTime<chrono::Utc>>,
    pub started_after: Option<chrono::DateTime<chrono::Utc>>,
    pub success: Option<bool>,
    pub parent_job: Option<String>,
    pub order_desc: Option<bool>,
    pub job_kinds: Option<String>,
    pub is_skipped: Option<bool>,
    pub is_flow_step: Option<bool>,
    pub suspended: Option<bool>,
    pub schedule_path: Option<String>,
    // filter by matching a subset of the args using base64 encoded json subset
    pub args: Option<String>,
    // filter by matching a subset of the result using base64 encoded json subset
    pub result: Option<String>,
    pub tag: Option<String>,
}

async fn list_completed_jobs(
    authed: Authed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListCompletedQuery>,
) -> error::JsonResult<Vec<CompletedJob>> {
    check_scopes(&authed, || format!("listjobs"))?;

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
            "email",
            "visible_to_owner",
            "mem_peak",
            "tag",
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

    tracing::info!("job_o: {:?}", job_o);
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

#[derive(Serialize)]
struct CompletedJobResult {
    completed: bool,
    result: Option<serde_json::Value>,
}

async fn get_completed_job_result_maybe(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::JsonResult<CompletedJobResult> {
    let result_o = sqlx::query_scalar!(
        "SELECT result FROM completed_job WHERE id = $1 AND workspace_id = $2",
        id,
        w_id,
    )
    .fetch_optional(&db)
    .await?;

    if let Some(result) = result_o {
        Ok(Json(CompletedJobResult { completed: true, result }))
    } else {
        Ok(Json(CompletedJobResult { completed: false, result: None }))
    }
}

async fn delete_completed_job(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::JsonResult<CompletedJob> {
    check_scopes(&authed, || format!("deletejob"))?;

    let mut tx = user_db.begin(&authed).await?;

    require_admin(authed.is_admin, &authed.username)?;
    let job_o = sqlx::query_as::<_, CompletedJob>(
        "UPDATE completed_job SET logs = '', result = null, deleted = true WHERE id = $1 AND workspace_id = $2 \
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
