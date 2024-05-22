/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::body::Body;
use axum::http::HeaderValue;
use serde_json::value::RawValue;
use sqlx::Pool;
use std::collections::HashMap;
#[cfg(feature = "prometheus")]
use std::sync::atomic::Ordering;
use tokio::io::AsyncReadExt;
#[cfg(feature = "prometheus")]
use tokio::time::Instant;
use windmill_common::flow_status::{JobResult, RestartedFrom};
use windmill_common::jobs::{
    format_completed_job_result, format_result, CompletedJobWithFormattedResult, FormattedResult,
    ENTRYPOINT_OVERRIDE,
};
use windmill_common::worker::TMP_DIR;

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use windmill_common::scripts::PREVIEW_IS_CODEBASE_HASH;
use windmill_common::variables::get_workspace_key;

use crate::db::ApiAuthed;

use crate::{
    db::DB,
    users::{check_scopes, require_owner_of_path, OptAuthed},
    utils::require_super_admin,
};
use anyhow::Context;
use axum::{
    extract::{FromRequest, Json, Path, Query},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Router,
};
use base64::Engine;
use chrono::Utc;
use hmac::Mac;
use hyper::{Request, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sql_builder::prelude::*;
use sqlx::types::JsonRawValue;
use sqlx::{types::Uuid, FromRow, Postgres, Transaction};
use tower_http::cors::{Any, CorsLayer};
use urlencoding::encode;
use windmill_audit::audit_ee::{audit_log, AuditAuthor};
use windmill_audit::ActionKind;
use windmill_common::worker::{to_raw_value, CUSTOM_TAGS_PER_WORKSPACE, SERVER_CONFIG};
use windmill_common::{
    db::UserDB,
    error::{self, to_anyhow, Error},
    flow_status::{Approval, FlowStatus, FlowStatusModule},
    flows::FlowValue,
    jobs::{script_path_to_payload, CompletedJob, JobKind, JobPayload, QueuedJob, RawCode},
    oauth2::HmacSha256,
    scripts::{ScriptHash, ScriptLang},
    users::username_to_permissioned_as,
    utils::{not_found_if_none, now_from_db, paginate, require_admin, Pagination, StripPath},
};

#[cfg(all(feature = "enterprise", feature = "parquet"))]
use windmill_common::s3_helpers::OBJECT_STORE_CACHE_SETTINGS;
#[cfg(feature = "prometheus")]
use windmill_common::{METRICS_DEBUG_ENABLED, METRICS_ENABLED};

use windmill_common::{get_latest_deployed_hash_for_path, BASE_URL};
use windmill_queue::{
    add_completed_job_error, append_logs, get_queued_job, get_result_by_id_from_running_flow,
    job_is_complete, push, CanceledBy, DecodeQueries, PushArgs, PushIsolationLevel,
};

#[cfg(feature = "prometheus")]
type Histo = prometheus::Histogram;

#[cfg(not(feature = "prometheus"))]
type Histo = ();

#[cfg(feature = "prometheus")]
fn setup_list_jobs_debug_metrics() -> Option<Histo> {
    let api_list_jobs_query_duration = if METRICS_DEBUG_ENABLED.load(Ordering::Relaxed)
        && METRICS_ENABLED.load(Ordering::Relaxed)
    {
        Some(
            prometheus::register_histogram!(prometheus::HistogramOpts::new(
                "api_list_jobs_query_duration",
                "Duration of listing jobs (query)",
            ))
            .expect("register prometheus metric"),
        )
    } else {
        None
    };

    api_list_jobs_query_duration
}

#[cfg(not(feature = "prometheus"))]
fn setup_list_jobs_debug_metrics() -> Option<Histo> {
    None
}

pub fn workspaced_service() -> Router {
    let cors = CorsLayer::new()
        .allow_methods([http::Method::GET, http::Method::POST])
        .allow_headers([http::header::CONTENT_TYPE, http::header::AUTHORIZATION])
        .allow_origin(Any);

    let api_list_jobs_query_duration = setup_list_jobs_debug_metrics();

    Router::new()
        .route(
            "/run/f/*script_path",
            post(run_flow_by_path)
                .head(|| async { "" })
                .layer(cors.clone()),
        )
        .route(
            "/run/workflow_as_code/:job_id/:entrypoint",
            post(run_workflow_as_code)
                .head(|| async { "" })
                .layer(cors.clone()),
        )
        .route(
            "/restart/f/:job_id/from/:step_id",
            post(restart_flow).head(|| async { "" }).layer(cors.clone()),
        )
        .route(
            "/restart/f/:job_id/from/:step_id/:branch_of_iteration_n",
            post(restart_flow).head(|| async { "" }).layer(cors.clone()),
        )
        .route(
            "/run/p/*script_path",
            post(run_script_by_path)
                .head(|| async { "" })
                .layer(cors.clone()),
        )
        .route(
            "/run_wait_result/p/*script_path",
            post(run_wait_result_script_by_path)
                .get(run_wait_result_job_by_path_get)
                .head(|| async { "" })
                .layer(cors.clone()),
        )
        .route(
            "/run_wait_result/h/:hash",
            post(run_wait_result_script_by_hash)
                .head(|| async { "" })
                .layer(cors.clone()),
        )
        .route(
            "/run_wait_result/f/*script_path",
            post(run_wait_result_flow_by_path)
                .get(run_wait_result_flow_by_path_get)
                .head(|| async { "" })
                .layer(cors.clone()),
        )
        .route(
            "/run/h/:hash",
            post(run_job_by_hash)
                .head(|| async { "" })
                .layer(cors.clone()),
        )
        .route("/run/preview", post(run_preview_script))
        .route("/run/preview_bundle", post(run_bundle_preview_script))
        .route("/add_batch_jobs/:n", post(add_batch_jobs))
        .route("/run/preview_flow", post(run_preview_flow_job))
        .route(
            "/list",
            get(list_jobs).layer(Extension(api_list_jobs_query_duration)),
        )
        .route("/queue/list", get(list_queue_jobs))
        .route("/queue/count", get(count_queue_jobs))
        .route("/queue/cancel_all", post(cancel_all))
        .route("/completed/count", get(count_completed_jobs))
        .route(
            "/completed/list",
            get(list_completed_jobs).layer(cors.clone()),
        )
        .route(
            "/completed/get/:id",
            get(get_completed_job).layer(cors.clone()),
        )
        .route(
            "/completed/get_result/:id",
            get(get_completed_job_result).layer(cors.clone()),
        )
        .route(
            "/completed/get_result_maybe/:id",
            get(get_completed_job_result_maybe).layer(cors.clone()),
        )
        .route(
            "/completed/delete/:id",
            post(delete_completed_job).layer(cors.clone()),
        )
        .route(
            "/flow/resume/:id",
            post(resume_suspended_flow_as_owner).layer(cors.clone()),
        )
        .route(
            "/job_signature/:job_id/:resume_id",
            get(create_job_signature).layer(cors.clone()),
        )
        .route(
            "/flow/user_states/:job_id/:key",
            get(get_flow_user_state)
                .post(set_flow_user_state)
                .layer(cors.clone()),
        )
        .route(
            "/resume_urls/:job_id/:resume_id",
            get(get_resume_urls).layer(cors.clone()),
        )
        .route(
            "/result_by_id/:job_id/:node_id",
            get(get_result_by_id).layer(cors.clone()),
        )
        .route("/run/dependencies", post(run_dependencies_job))
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
        .route("/get_root_job_id/:id", get(get_root_job))
        .route("/get/:id", get(get_job))
        .route("/get_logs/:id", get(get_job_logs))
        .route("/get_flow_debug_info/:id", get(get_flow_job_debug_info))
        .route("/completed/get/:id", get(get_completed_job))
        .route("/completed/get_result/:id", get(get_completed_job_result))
        .route(
            "/completed/get_result_maybe/:id",
            get(get_completed_job_result_maybe),
        )
        .route("/getupdate/:id", get(get_job_update))
        .route("/get_log_file/*file_path", get(get_log_file))
        .route("/queue/cancel/:id", post(cancel_job_api))
        .route(
            "/queue/cancel_persistent/*script_path",
            post(cancel_persistent_script_api),
        )
        .route("/queue/force_cancel/:id", post(force_cancel))
}

pub fn global_root_service() -> Router {
    Router::new().route("/db_clock", get(get_db_clock))
}

#[derive(Deserialize)]
struct JsonPath {
    pub json_path: Option<String>,
}
async fn get_result_by_id(
    Extension(db): Extension<DB>,
    Path((w_id, flow_id, node_id)): Path<(String, Uuid, String)>,
    Query(JsonPath { json_path }): Query<JsonPath>,
) -> windmill_common::error::JsonResult<Box<JsonRawValue>> {
    let res = windmill_queue::get_result_by_id(db, w_id, flow_id, node_id, json_path).await?;
    Ok(Json(res))
}

async fn get_root_job(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> windmill_common::error::JsonResult<String> {
    let res = compute_root_job_for_flow(&db, &w_id, id).await?;
    Ok(Json(res))
}

async fn compute_root_job_for_flow(db: &DB, w_id: &str, job_id: Uuid) -> error::Result<String> {
    let mut job = get_queued_job(&job_id, w_id, db).await?;
    while let Some(j) = job {
        if let Some(uuid) = j.parent_job {
            job = get_queued_job(&uuid, w_id, db).await?;
        } else {
            return Ok(j.id.to_string());
        }
    }
    Ok(job_id.to_string())
}

async fn get_db_clock(Extension(db): Extension<DB>) -> windmill_common::error::JsonResult<i64> {
    Ok(Json(now_from_db(&db).await?.timestamp_millis()))
}

async fn cancel_job_api(
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Json(CancelJob { reason }): Json<CancelJob>,
) -> error::Result<String> {
    let tx = db.begin().await?;

    let audit_author = match opt_authed {
        Some(authed) => (&authed).into(),
        None => AuditAuthor {
            email: "anonymous".to_string(),
            username: "anonymous".to_string(),
            username_override: None,
        },
    };

    let (mut tx, job_option) = windmill_queue::cancel_job(
        &audit_author.username,
        reason,
        id,
        &w_id,
        tx,
        &db,
        rsmq,
        false,
    )
    .await?;

    if let Some(id) = job_option {
        audit_log(
            &mut *tx,
            &audit_author,
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
        tx.commit().await?;
        if job_is_complete(&db, id, &w_id).await.unwrap_or(false) {
            return Ok(format!("queued job id {} is already completed", id));
        } else {
            return Err(error::Error::NotFound(format!(
                "queued job id {} does not exist",
                id
            )));
        }
    }
}

async fn cancel_persistent_script_api(
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    Json(CancelJob { reason }): Json<CancelJob>,
) -> error::Result<()> {
    let audit_author = match opt_authed {
        Some(authed) => (&authed).into(),
        None => AuditAuthor {
            email: "anonymous".to_string(),
            username: "anonymous".to_string(),
            username_override: None,
        },
    };

    let cancelled_job_ids = windmill_queue::cancel_persistent_script_jobs(
        &audit_author.username,
        reason,
        script_path.to_path(),
        &w_id,
        &db,
        rsmq,
    )
    .await?;

    audit_log(
        &db,
        &audit_author,
        "jobs.cancel_persistent",
        ActionKind::Delete,
        &w_id,
        Some(script_path.to_path()),
        Some(
            [(
                "job_ids",
                cancelled_job_ids
                    .into_iter()
                    .map(|uuid: Uuid| uuid.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
                    .as_str(),
            )]
            .into(),
        ),
    )
    .await?;
    Ok(())
}

async fn force_cancel(
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Json(CancelJob { reason }): Json<CancelJob>,
) -> error::Result<String> {
    let tx = db.begin().await?;

    let audit_author = match opt_authed {
        Some(authed) => (&authed).into(),
        None => AuditAuthor {
            email: "anonymous".to_string(),
            username: "anonymous".to_string(),
            username_override: None,
        },
    };

    let (mut tx, job_option) = windmill_queue::cancel_job(
        &audit_author.username,
        reason,
        id,
        &w_id,
        tx,
        &db,
        rsmq,
        true,
    )
    .await?;

    if let Some(id) = job_option {
        audit_log(
            &mut *tx,
            &audit_author,
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
        tx.commit().await?;
        if job_is_complete(&db, id, &w_id).await.unwrap_or(false) {
            return Ok(format!("queued job id {} is already completed", id));
        } else {
            return Err(error::Error::NotFound(format!(
                "queued job id {} does not exist",
                id
            )));
        }
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
    .fetch_one(&mut **db)
    .await
    .map_err(|e| {
        Error::InternalErr(format!(
            "querying getting path for hash {hash} in {w_id}: {e}"
        ))
    })?;
    Ok(path)
}

pub async fn get_path_tag_limits_cache_for_hash(
    db: &DB,
    w_id: &str,
    hash: i64,
) -> error::Result<(
    String,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    ScriptLang,
    Option<bool>,
    Option<i16>,
    Option<bool>,
    Option<i32>,
)> {
    let script = sqlx::query!(
        "select path, tag, concurrency_key, concurrent_limit, concurrency_time_window_s, cache_ttl, language as \"language: ScriptLang\", dedicated_worker, priority, delete_after_use, timeout from script where hash = $1 AND workspace_id = $2",
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
    Ok((
        script.path,
        script.tag,
        script.concurrency_key,
        script.concurrent_limit,
        script.concurrency_time_window_s,
        script.cache_ttl,
        script.language,
        script.dedicated_worker,
        script.priority,
        script.delete_after_use,
        script.timeout,
    ))
}

async fn get_flow_job_debug_info(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::Result<Response> {
    let job = get_queued_job(&id, w_id.as_str(), &db).await?;
    if let Some(job) = job {
        let is_flow = job.is_flow();
        if job.is_flow_step || !is_flow {
            return Err(error::Error::BadRequest(
                "This endpoint is only for root flow jobs".to_string(),
            ));
        }
        let mut jobs = HashMap::new();
        jobs.insert("root_job".to_string(), Job::QueuedJob(job.clone()));

        let mut job_ids = vec![];
        let jobs_with_root = sqlx::query_scalar!(
            "SELECT id FROM queue WHERE workspace_id = $1 and root_job = $2",
            &w_id,
            &job.id,
        )
        .fetch_all(&db)
        .await?;

        for job in jobs_with_root {
            job_ids.push(job);
        }

        let leaf_jobs: HashMap<String, JobResult> = job
            .leaf_jobs
            .and_then(|x| serde_json::from_value(x).ok())
            .unwrap_or_else(HashMap::new);
        for job in leaf_jobs.iter() {
            match job.1 {
                JobResult::ListJob(jobs) => job_ids.extend(jobs.to_owned()),
                JobResult::SingleJob(job) => job_ids.push(job.clone()),
            }
        }
        for job_id in job_ids {
            let job = get_job_internal(&db, w_id.as_str(), job_id, false).await;
            if let Ok(job) = job {
                jobs.insert(job.id().to_string(), job);
            }
        }
        Ok(Json(jobs).into_response())
    } else {
        Err(error::Error::NotFound(format!(
            "QueuedJob {} not found",
            id
        )))
    }
}

#[derive(Deserialize)]
struct GetJobQuery {
    pub no_logs: Option<bool>,
}
async fn get_job(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Query(GetJobQuery { no_logs }): Query<GetJobQuery>,
) -> error::Result<Response> {
    let job = get_job_internal(&db, w_id.as_str(), id, no_logs.unwrap_or(false)).await?;
    Ok(Json(job).into_response())
}

lazy_static::lazy_static! {
    static ref GET_COMPLETED_JOB_QUERY_NO_LOGS: String = generate_get_job_query(true, "completed_job");
    static ref GET_COMPLETED_JOB_QUERY: String = generate_get_job_query(false, "completed_job");
    static ref GET_QUEUED_JOB_QUERY_NO_LOGS: String = generate_get_job_query(true, "queue");
    static ref GET_QUEUED_JOB_QUERY: String = generate_get_job_query(false, "queue");
}

fn generate_get_job_query(no_logs: bool, table: &str) -> String {
    let log_expr = if no_logs {
        "null".to_string()
    } else {
        format!("right(concat(coalesce({table}.logs, ''), job_logs.logs), 20000)")
    };
    let join = if no_logs {
        "".to_string()
    } else {
        format!("LEFT JOIN job_logs ON {table}.id = job_logs.job_id")
    };
    let additional_fields = if table == "completed_job" {
        "duration_ms,      
        success,        
        result,    
        deleted,    
        is_skipped,
        result->'wm_labels' as labels,
        CASE WHEN result is null or pg_column_size(result) < 90000 THEN result ELSE '\"WINDMILL_TOO_BIG\"'::jsonb END as result"
    } else {
        "scheduled_for,  
        running,       
        last_ping,        
        suspend,     
        suspend_until,
        same_worker,
        pre_run_error,           
        visible_to_owner,          
        root_job,         
        leaf_jobs,        
        tag,       
        concurrent_limit,      
        concurrency_time_window_s, 
        timeout,
        flow_step_id,
        cache_ttl
        "
    };
    return format!("SELECT 
    id, {table}.workspace_id, parent_job, created_by, {table}.created_at, started_at, script_hash, script_path, 
    CASE WHEN args is null or pg_column_size(args) < 90000 THEN args ELSE '{{\"reason\": \"WINDMILL_TOO_BIG\"}}'::jsonb END as args, 
    {log_expr} as logs, raw_code, canceled, canceled_by, canceled_reason, job_kind, env_id,
    schedule_path, permissioned_as, flow_status, raw_flow, is_flow_step, language,
    raw_lock, email, visible_to_owner, mem_peak, tag, priority, {additional_fields}
    FROM {table}
    {join}
    WHERE id = $1 AND {table}.workspace_id = $2");
}
pub async fn get_job_internal(
    db: &DB,
    workspace_id: &str,
    job_id: Uuid,
    no_logs: bool,
) -> error::Result<Job> {
    let cjob_maybe = sqlx::query_as::<_, CompletedJob>(if no_logs {
        &*GET_COMPLETED_JOB_QUERY_NO_LOGS
    } else {
        &*GET_COMPLETED_JOB_QUERY
    })
    .bind(job_id)
    .bind(workspace_id)
    .fetch_optional(db)
    .await?
    .map(Job::CompletedJob);

    if let Some(cjob) = cjob_maybe {
        Ok(match cjob {
            Job::CompletedJob(cjob) => {
                Job::CompletedJobWithFormattedResult(format_completed_job_result(cjob))
            }
            cjob => cjob,
        })
    } else {
        let job_o = sqlx::query_as::<_, QueuedJob>(if no_logs {
            &*GET_QUEUED_JOB_QUERY_NO_LOGS
        } else {
            &*GET_QUEUED_JOB_QUERY
        })
        .bind(job_id)
        .bind(workspace_id)
        .fetch_optional(db)
        .await?
        .map(Job::QueuedJob);
        let job: Job = not_found_if_none(job_o, "Job", job_id.to_string())?;
        Ok(job)
    }
}

#[cfg(all(feature = "enterprise", feature = "parquet"))]
async fn get_logs_from_store(
    log_offset: i32,
    logs: &str,
    log_file_index: &Option<Vec<String>>,
) -> Option<error::Result<Body>> {
    if log_offset > 0 {
        if let Some(file_index) = log_file_index.clone() {
            tracing::debug!("Getting logs from store: {file_index:?}");
            if let Some(os) = OBJECT_STORE_CACHE_SETTINGS.read().await.clone() {
                tracing::debug!("object store client present, streaming from there");

                let logs = logs.to_string();
                let stream = async_stream::stream! {
                    for file_p in file_index.clone() {
                        let file_p_2 = file_p.clone();
                        let file = os.get(&object_store::path::Path::from(file_p)).await;
                        if let Ok(file) = file {
                            if let Ok(bytes) = file.bytes().await {
                                yield Ok(bytes::Bytes::from(bytes)) as object_store::Result<bytes::Bytes>;
                            }
                        } else {
                            tracing::debug!("error getting file from store: {file_p_2}: {}", file.err().unwrap());
                        }
                    }

                    yield Ok(bytes::Bytes::from(logs))
                };
                return Some(Ok(Body::from_stream(stream)));
            } else {
                tracing::debug!("object store client not present, cannot stream logs from store");
            }
        }
    }
    return None;
}

async fn get_logs_from_disk(
    log_offset: i32,
    logs: &str,
    log_file_index: &Option<Vec<String>>,
) -> Option<error::Result<Body>> {
    if log_offset > 0 {
        if let Some(file_index) = log_file_index.clone() {
            for file_p in &file_index {
                if !tokio::fs::metadata(format!("{TMP_DIR}/{file_p}"))
                    .await
                    .is_ok()
                {
                    return None;
                }
            }

            let logs = logs.to_string();
            let stream = async_stream::stream! {
                for file_p in file_index.clone() {
                    let mut file = tokio::fs::File::open(format!("{TMP_DIR}/{file_p}")).await.map_err(to_anyhow)?;
                    let mut buffer = Vec::new();
                    file.read_to_end(&mut buffer).await.map_err(to_anyhow)?;
                    yield Ok(bytes::Bytes::from(buffer)) as anyhow::Result<bytes::Bytes>;
                }

                yield Ok(bytes::Bytes::from(logs))
            };
            return Some(Ok(Body::from_stream(stream)));
        }
    }
    return None;
}

fn content_plain(body: Body) -> Response {
    use axum::http::header;
    Response::builder()
        .header(header::CONTENT_TYPE, "text/plain")
        .body(body)
        .unwrap()
}

async fn get_job_logs(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::Result<Response> {
    let record = sqlx::query!(
        "SELECT CONCAT(coalesce(completed_job.logs, ''), coalesce(job_logs.logs, '')) as logs, job_logs.log_offset, job_logs.log_file_index
        FROM completed_job 
        LEFT JOIN job_logs ON job_logs.job_id = completed_job.id 
        WHERE completed_job.id = $1 AND completed_job.workspace_id = $2",
        id,
        w_id
    )
    .fetch_optional(&db)
    .await?;

    if let Some(record) = record {
        let logs = record.logs.unwrap_or_default();
        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if let Some(r) = get_logs_from_store(record.log_offset, &logs, &record.log_file_index).await
        {
            return r.map(content_plain);
        }
        if let Some(r) = get_logs_from_disk(record.log_offset, &logs, &record.log_file_index).await
        {
            return r.map(content_plain);
        }
        Ok(content_plain(Body::from(logs)))
    } else {
        let text = sqlx::query!(
            "SELECT CONCAT(coalesce(queue.logs, ''), coalesce(job_logs.logs, '')) as logs, job_logs.log_offset, job_logs.log_file_index
            FROM queue 
            LEFT JOIN job_logs ON job_logs.job_id = queue.id 
            WHERE queue.id = $1 AND queue.workspace_id = $2",
            id,
            w_id
        )
        .fetch_optional(&db)
        .await?;
        let text = not_found_if_none(text, "Job Logs", id.to_string())?;

        let logs = text.logs.unwrap_or_default();
        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if let Some(r) = get_logs_from_store(text.log_offset, &logs, &text.log_file_index).await {
            return r.map(content_plain);
        }
        if let Some(r) = get_logs_from_disk(text.log_offset, &logs, &text.log_file_index).await {
            return r.map(content_plain);
        }
        Ok(content_plain(Body::from(logs)))
    }
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct ListableCompletedJob {
    pub r#type: String,
    pub workspace_id: String,
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_job: Option<Uuid>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub duration_ms: i64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_hash: Option<ScriptHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_path: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<serde_json::Value>,
}

#[derive(Deserialize, Clone)]
pub struct RunJobQuery {
    scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    scheduled_in_secs: Option<i64>,
    parent_job: Option<Uuid>,
    root_job: Option<Uuid>,
    invisible_to_owner: Option<bool>,
    queue_limit: Option<i64>,
    payload: Option<String>,
    job_id: Option<Uuid>,
    tag: Option<String>,
    timeout: Option<i32>,
    cache_ttl: Option<i32>,
}

impl RunJobQuery {
    async fn get_scheduled_for<'c>(
        &self,
        db: &DB,
    ) -> error::Result<Option<chrono::DateTime<chrono::Utc>>> {
        if let Some(scheduled_for) = self.scheduled_for {
            Ok(Some(scheduled_for))
        } else if let Some(scheduled_in_secs) = self.scheduled_in_secs {
            let now = now_from_db(db).await?;
            Ok(Some(
                now + chrono::Duration::try_seconds(scheduled_in_secs).unwrap_or_default(),
            ))
        } else {
            Ok(None)
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct ListQueueQuery {
    pub script_path_start: Option<String>,
    pub script_path_exact: Option<String>,
    pub script_hash: Option<String>,
    pub created_by: Option<String>,
    pub started_before: Option<chrono::DateTime<chrono::Utc>>,
    pub started_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_or_started_before: Option<chrono::DateTime<chrono::Utc>>,
    pub created_or_started_after: Option<chrono::DateTime<chrono::Utc>>,
    pub running: Option<bool>,
    pub schedule_path: Option<String>,
    pub parent_job: Option<String>,
    pub order_desc: Option<bool>,
    pub job_kinds: Option<String>,
    pub suspended: Option<bool>,
    // filter by matching a subset of the args using base64 encoded json subset
    pub args: Option<String>,
    pub tag: Option<String>,
    pub scheduled_for_before_now: Option<bool>,
    pub all_workspaces: Option<bool>,
    pub is_flow_step: Option<bool>,
    pub has_null_parent: Option<bool>,
    pub is_not_schedule: Option<bool>,
}

impl From<ListCompletedQuery> for ListQueueQuery {
    fn from(lcq: ListCompletedQuery) -> Self {
        Self {
            script_path_start: lcq.script_path_start,
            script_path_exact: lcq.script_path_exact,
            script_hash: lcq.script_hash,
            created_by: lcq.created_by,
            started_before: lcq.started_before,
            started_after: lcq.started_after,
            created_before: lcq.created_before,
            created_after: lcq.created_after,
            created_or_started_before: lcq.created_or_started_before,
            created_or_started_after: lcq.created_or_started_after,
            running: lcq.running,
            parent_job: lcq.parent_job,
            order_desc: lcq.order_desc,
            job_kinds: lcq.job_kinds,
            suspended: lcq.suspended,
            args: lcq.args,
            tag: lcq.tag,
            schedule_path: lcq.schedule_path,
            scheduled_for_before_now: lcq.scheduled_for_before_now,
            all_workspaces: lcq.all_workspaces,
            is_flow_step: lcq.is_flow_step,
            has_null_parent: lcq.has_null_parent,
            is_not_schedule: lcq.is_not_schedule,
        }
    }
}

pub fn filter_list_queue_query(
    mut sqlb: SqlBuilder,
    lq: &ListQueueQuery,
    w_id: &str,
) -> SqlBuilder {
    if w_id != "admins" || !lq.all_workspaces.is_some_and(|x| x) {
        sqlb.and_where_eq("workspace_id", "?".bind(&w_id));
    }

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
        sqlb.and_where_le(
            "started_at",
            format!("to_timestamp({}  / 1000.0)", dt.timestamp_millis()),
        );
    }
    if let Some(dt) = &lq.started_after {
        sqlb.and_where_ge(
            "started_at",
            format!("to_timestamp({}  / 1000.0)", dt.timestamp_millis()),
        );
    }
    if let Some(fs) = &lq.is_flow_step {
        sqlb.and_where_eq("is_flow_step", fs);
    }
    if let Some(fs) = &lq.has_null_parent {
        if *fs {
            sqlb.and_where_is_null("parent_job");
        }
    }

    if let Some(dt) = &lq.created_before {
        sqlb.and_where_le(
            "created_at",
            format!("to_timestamp({}  / 1000.0)", dt.timestamp_millis()),
        );
    }
    if let Some(dt) = &lq.created_after {
        sqlb.and_where_ge(
            "created_at",
            format!("to_timestamp({}  / 1000.0)", dt.timestamp_millis()),
        );
    }

    if let Some(dt) = &lq.created_or_started_after {
        let ts = dt.timestamp_millis();
        sqlb.and_where(format!("(started_at IS NOT NULL AND started_at >= to_timestamp({}  / 1000.0)) OR (started_at IS NULL AND created_at >= to_timestamp({}  / 1000.0))", ts, ts));
    }

    if let Some(dt) = &lq.created_or_started_before {
        let ts = dt.timestamp_millis();
        sqlb.and_where(format!("(started_at IS NOT NULL AND started_at < to_timestamp({}  / 1000.0)) OR (started_at IS NULL AND created_at < to_timestamp({}  / 1000.0))", ts, ts));
    }

    if let Some(s) = &lq.suspended {
        if *s {
            sqlb.and_where_gt("suspend", 0);
        } else {
            sqlb.and_where_is_null("suspend_until");
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

    if lq.scheduled_for_before_now.is_some_and(|x| x) {
        sqlb.and_where_le("scheduled_for", "now()");
    }

    if lq.is_not_schedule.unwrap_or(false) {
        sqlb.and_where("schedule_path IS null");
    }

    sqlb
}

pub fn list_queue_jobs_query(w_id: &str, lq: &ListQueueQuery, fields: &[&str]) -> SqlBuilder {
    let sqlb = SqlBuilder::select_from("queue")
        .fields(fields)
        .order_by("created_at", lq.order_desc.unwrap_or(true))
        .limit(1000)
        .clone();

    filter_list_queue_query(sqlb, lq, w_id)
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
    pub tag: String,
    pub priority: Option<i16>,
    pub workspace_id: String,
}

async fn list_queue_jobs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
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
            "null as args",
            "job_kind",
            "schedule_path",
            "permissioned_as",
            "is_flow_step",
            "language",
            "same_worker",
            "email",
            "suspend",
            "tag",
            "priority",
            "workspace_id",
        ],
    )
    .sql()?;
    let mut tx = user_db.begin(&authed).await?;
    let jobs = sqlx::query_as::<_, ListableQueuedJob>(&sql)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(jobs))
}

async fn cancel_all(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,

    Path(w_id): Path<String>,
) -> error::JsonResult<Vec<Uuid>> {
    require_admin(authed.is_admin, &authed.username)?;

    let jobs = sqlx::query!(
        "SELECT id, running, is_flow_step FROM queue WHERE scheduled_for < now() AND workspace_id = $1 AND schedule_path IS NULL",
        w_id,
    )
    .fetch_all(&db)
    .await?;

    let username = authed.username;
    let mut uuids = vec![];
    for j in jobs.iter() {
        let r = sqlx::query!(
            "UPDATE queue SET canceled = true,  canceled_by = $1, scheduled_for = now(), suspend = 0 WHERE id = $2 RETURNING 1 as one",
            username,
            j.id,
        )
        .fetch_optional(&db)
        .await;

        if r.as_ref().is_ok_and(|x| x.is_some()) {
            uuids.push(j.id);

            if !j.running && !j.is_flow_step.unwrap_or(false) {
                let e = serde_json::json!({"message": format!("Job canceled: cancel_all by {username}"), "name": "Canceled", "reason": "cancel_all", "canceler": username});
                let job_running = get_queued_job(&j.id, &w_id, &db).await?;

                if let Some(job_running) = job_running {
                    append_logs(
                        j.id,
                        w_id.clone(),
                        format!("canceled by {username}: cancel_all"),
                        db.clone(),
                    )
                    .await;
                    let add_job = add_completed_job_error(
                        &db,
                        &job_running,
                        job_running.mem_peak.unwrap_or(0),
                        Some(CanceledBy {
                            username: Some(username.to_string()),
                            reason: Some("cancel_all".to_string()),
                        }),
                        e,
                        rsmq.clone(),
                        "server",
                        true,
                    )
                    .await;
                    if let Err(e) = add_job {
                        tracing::error!("Failed to add canceled job: {}", e);
                    }
                }
            }
        } else {
            tracing::error!("Failed to cancel job: {:?} {:?}", j.id, r.err());
        }
    }

    Ok(Json(uuids))
}

#[derive(Serialize, Debug, FromRow)]
struct QueueStats {
    database_length: i64,
}

#[derive(Deserialize)]
pub struct CountQueueJobsQuery {
    all_workspaces: Option<bool>,
}

async fn count_queue_jobs(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(cq): Query<CountQueueJobsQuery>,
) -> error::JsonResult<QueueStats> {
    Ok(Json(
        sqlx::query_as!(
            QueueStats,
            "SELECT coalesce(COUNT(*), 0) as \"database_length!\" FROM queue WHERE (workspace_id = $1 OR $2) AND scheduled_for <= now() AND running = false",
            w_id,
            w_id == "admins" && cq.all_workspaces.unwrap_or(false),
        )
        .fetch_one(&db)
        .await?,
    ))
}

async fn count_completed_jobs(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> error::JsonResult<QueueStats> {
    Ok(Json(
        sqlx::query_as!(
            QueueStats,
            "SELECT coalesce(COUNT(*), 0) as \"database_length!\" FROM completed_job WHERE workspace_id = $1",
            w_id
        )
        .fetch_one(&db)
        .await?,
    ))
}

async fn list_jobs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListCompletedQuery>,
    Extension(_api_list_jobs_query_duration): Extension<Option<Histo>>,
) -> error::JsonResult<Vec<Job>> {
    check_scopes(&authed, || format!("listjobs"))?;

    let (per_page, offset) = paginate(pagination);
    let lqc = lq.clone();

    if lq.success.is_some() && lq.running.is_some_and(|x| x) {
        return Err(error::Error::BadRequest(
            "cannot specify both success and running".to_string(),
        ));
    }
    let sqlc = if lq.running.is_none() {
        Some(list_completed_jobs_query(
            &w_id,
            per_page + offset,
            0,
            &ListCompletedQuery { order_desc: Some(true), ..lqc },
            UnifiedJob::completed_job_fields(),
        ))
    } else {
        None
    };

    let sql = if lq.success.is_none() && lq.label.is_none() {
        let sqlq = list_queue_jobs_query(
            &w_id,
            &ListQueueQuery { order_desc: Some(true), ..lq.into() },
            UnifiedJob::queued_job_fields(),
        );

        if let Some(sqlc) = sqlc {
            format!(
                "{} UNION ALL {} LIMIT {} OFFSET {};",
                &sqlq.subquery()?,
                &sqlc.subquery()?,
                per_page,
                offset
            )
        } else {
            sqlq.query()?
        }
    } else {
        sqlc.unwrap().query()?
    };
    let mut tx = user_db.begin(&authed).await?;

    #[cfg(feature = "prometheus")]
    let start = Instant::now();

    let jobs: Vec<UnifiedJob> = sqlx::query_as(&sql).fetch_all(&mut *tx).await?;
    tx.commit().await?;

    #[cfg(feature = "prometheus")]
    if let Some(api_list_jobs_query_duration) = _api_list_jobs_query_duration {
        let duration = start.elapsed().as_secs_f64();
        api_list_jobs_query_duration.observe(duration);
        tracing::info!("list_jobs query took {}s: {}", duration, sql);
    }

    Ok(Json(jobs.into_iter().map(From::from).collect()))
}

pub async fn resume_suspended_flow_as_owner(
    authed: ApiAuthed,
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

    insert_resume_job(
        0,
        job_id,
        &flow,
        value,
        Some(authed.username),
        true,
        &mut tx,
    )
    .await?;

    resume_immediately_if_relevant(flow, job_id, &mut tx).await?;

    tx.commit().await?;
    Ok(StatusCode::CREATED)
}

pub async fn resume_suspended_job(
    authed: Option<ApiAuthed>,
    Extension(db): Extension<DB>,
    Path((w_id, job_id, resume_id, secret)): Path<(String, Uuid, u32, String)>,
    Query(approver): Query<QueryApprover>,
    QueryOrBody(value): QueryOrBody<serde_json::Value>,
) -> error::Result<StatusCode> {
    resume_suspended_job_internal(
        value, db, w_id, job_id, resume_id, approver, secret, authed, true,
    )
    .await
}

async fn resume_suspended_job_internal(
    value: Option<serde_json::Value>,
    db: sqlx::Pool<Postgres>,
    w_id: String,
    job_id: Uuid,
    resume_id: u32,
    approver: QueryApprover,
    secret: String,
    authed: Option<ApiAuthed>,
    approved: bool,
) -> Result<StatusCode, Error> {
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
    let parent_flow_info = get_suspended_parent_flow_info(job_id, &mut tx).await?;
    let parent_flow = get_job_internal(&db, w_id.as_str(), parent_flow_info.id, false).await?;
    let flow_status = parent_flow
        .flow_status()
        .ok_or_else(|| anyhow::anyhow!("unable to find the flow status in the flow job"))?;

    let trigger_email = match &parent_flow {
        Job::CompletedJob(job) => &job.email,
        Job::QueuedJob(job) => &job.email,
        Job::CompletedJobWithFormattedResult(job) => &job.cj.email,
    };
    conditionally_require_authed_user(authed.clone(), flow_status, trigger_email)?;

    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (SELECT 1 FROM resume_job WHERE id = $1)
        "#,
        Uuid::from_u128(job_id.as_u128() ^ resume_id as u128),
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    if exists {
        return Err(anyhow::anyhow!("resume request already sent").into());
    }

    let approver = if authed.as_ref().is_none()
        || (approver
            .approver
            .clone()
            .is_some_and(|x| x != "".to_string()))
    {
        approver.approver
    } else {
        authed.as_ref().map(|x| x.username.clone())
    };
    insert_resume_job(
        resume_id,
        job_id,
        &parent_flow_info,
        value,
        approver.clone(),
        approved,
        &mut tx,
    )
    .await?;

    if !approved {
        sqlx::query!(
            "UPDATE queue SET suspend = 0 WHERE id = $1",
            parent_flow_info.id
        )
        .execute(&mut *tx)
        .await?;
    } else {
        resume_immediately_if_relevant(parent_flow_info, job_id, &mut tx).await?;
    }

    let audit_author = match authed {
        Some(authed) => (&authed).into(),
        None => {
            let approver = approver.unwrap_or_else(|| "anonymous".to_string());
            AuditAuthor { email: approver.clone(), username: approver, username_override: None }
        }
    };
    audit_log(
        &mut *tx,
        &audit_author,
        "jobs.approved",
        ActionKind::Update,
        &w_id,
        Some(&job_id.to_string()),
        None,
    )
    .await?;
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
                .execute(&mut **tx)
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
    approved: bool,
    tx: &mut Transaction<'c, Postgres>,
) -> error::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO resume_job
                    (id, resume_id, job, flow, value, approver, approved)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
        Uuid::from_u128(job_id.as_u128() ^ resume_id as u128),
        resume_id as i32,
        job_id,
        flow.id,
        value,
        approver,
        approved
    )
    .execute(&mut **tx)
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
    .fetch_optional(&mut **tx)
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
    .fetch_optional(&mut **tx)
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
    authed: Option<ApiAuthed>,
    Extension(db): Extension<DB>,
    Path((w_id, job_id, resume_id, secret)): Path<(String, Uuid, u32, String)>,
    Query(approver): Query<QueryApprover>,
    QueryOrBody(value): QueryOrBody<serde_json::Value>,
) -> error::Result<StatusCode> {
    resume_suspended_job_internal(
        value, db, w_id, job_id, resume_id, approver, secret, authed, false,
    )
    .await
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
    authed: Option<ApiAuthed>,
    Extension(db): Extension<DB>,
    Path((w_id, job, resume_id, secret)): Path<(String, Uuid, u32, String)>,
    Query(approver): Query<QueryApprover>,
) -> error::Result<Response> {
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
    .fetch_optional(&mut *tx)
    .await?
    .flatten()
    .ok_or_else(|| anyhow::anyhow!("parent flow job not found"))?;

    let flow = get_job_internal(&db, w_id.as_str(), flow_id, true).await?;

    let flow_status = flow
        .flow_status()
        .ok_or_else(|| anyhow::anyhow!("unable to find the flow status in the flow job"))?;
    let flow_module_status = flow_status
        .modules
        .iter()
        .find(|p| p.job() == Some(job))
        .ok_or_else(|| anyhow::anyhow!("unable to find the module"))?;

    let trigger_email = match &flow {
        Job::CompletedJob(job) => &job.email,
        Job::QueuedJob(job) => &job.email,
        Job::CompletedJobWithFormattedResult(job) => &job.cj.email,
    };
    conditionally_require_authed_user(authed, flow_status.clone(), trigger_email)?;

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
        .fetch_all(&mut *tx)
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

    Ok(Json(SuspendedJobFlow { job: flow, approvers }).into_response())
}

fn conditionally_require_authed_user(
    _authed: Option<ApiAuthed>,
    flow_status: FlowStatus,
    _trigger_email: &str,
) -> error::Result<()> {
    let approval_conditions_opt = flow_status.approval_conditions;

    if approval_conditions_opt.is_none() {
        return Ok(());
    }
    let approval_conditions = approval_conditions_opt.unwrap();

    if approval_conditions.user_auth_required {
        {
            #[cfg(not(feature = "enterprise"))]
            return Err(Error::BadRequest(
                "Approvals for logged in users is an enterprise only feature".to_string(),
            ));

            #[cfg(feature = "enterprise")]
            {
                if _authed.is_none() {
                    return Err(Error::NotAuthorized(
                        "Only logged in users can approve this flow step".to_string(),
                    ));
                }

                let authed = _authed.unwrap();
                if !authed.is_admin {
                    if approval_conditions.self_approval_disabled && authed.email.eq(_trigger_email)
                    {
                        return Err(Error::PermissionDenied(
                            "Self-approval is disabled for this flow step".to_string(),
                        ));
                    }

                    if !approval_conditions.user_groups_required.is_empty() {
                        #[cfg(feature = "enterprise")]
                        {
                            for required_group in approval_conditions.user_groups_required.iter() {
                                if authed.groups.contains(&required_group) {
                                    return Ok(());
                                }
                            }
                            let error_msg = format!("Only users from one of the following groups are allowed to approve this workflow: {}", 
                            approval_conditions.user_groups_required.join(", "));
                            return Err(Error::PermissionDenied(error_msg));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub async fn create_job_signature(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, job_id, resume_id)): Path<(String, Uuid, u32)>,
    Query(approver): Query<QueryApprover>,
) -> error::Result<String> {
    let key = get_workspace_key(&w_id, &mut user_db.begin(&authed).await?).await?;
    create_signature(key, job_id, resume_id, approver.approver)
}

pub async fn get_flow_user_state(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, job_id, key)): Path<(String, Uuid, String)>,
) -> error::JsonResult<Option<serde_json::Value>> {
    let mut tx = user_db.begin(&authed).await?;
    let r = sqlx::query_scalar!(
        r#"
        SELECT flow_status->'user_states'->$1
        FROM queue
        WHERE id = $2 AND workspace_id = $3
        "#,
        key,
        job_id,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();
    Ok(Json(r))
}

pub async fn set_flow_user_state(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, job_id, key)): Path<(String, Uuid, String)>,
    Json(value): Json<serde_json::Value>,
) -> error::Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    let r = sqlx::query_scalar!(
        r#"
        UPDATE queue SET flow_status = JSONB_SET(flow_status,  ARRAY['user_states'], JSONB_SET(COALESCE(flow_status->'user_states', '{}'::jsonb), ARRAY[$1], $2))
        WHERE id = $3 AND workspace_id = $4 AND job_kind IN ('flow', 'flowpreview') RETURNING 1
        "#,
        key,
        value,
        job_id,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();
    if r.is_none() {
        return Err(Error::NotFound("Flow job not found".to_string()));
    }
    tx.commit().await?;
    Ok("Flow job state updated".to_string())
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
    authed: ApiAuthed,
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

    let base_url_str = BASE_URL.read().await.clone();
    let base_url = base_url_str.as_str();
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
    #[serde(rename = "CompletedJob")]
    CompletedJobWithFormattedResult(CompletedJobWithFormattedResult),
}

impl Job {
    pub fn raw_flow(&self) -> Option<FlowValue> {
        match self {
            Job::QueuedJob(job) => job
                .raw_flow
                .as_ref()
                .map(|rf| serde_json::from_str(rf.0.get()).ok())
                .flatten(),
            Job::CompletedJob(job) => job
                .raw_flow
                .as_ref()
                .map(|rf| serde_json::from_str(rf.0.get()).ok())
                .flatten(),
            Job::CompletedJobWithFormattedResult(job) => job
                .cj
                .raw_flow
                .as_ref()
                .map(|rf| serde_json::from_str(rf.0.get()).ok())
                .flatten(),
        }
    }

    pub fn append_to_logs(&mut self, logs: &str) {
        match self {
            Job::QueuedJob(job) => {
                if let Some(ref mut l) = job.logs {
                    l.push_str(logs);
                } else {
                    job.logs = Some(logs.to_string());
                }
            }
            Job::CompletedJob(job) => {
                if let Some(ref mut l) = job.logs {
                    l.push_str(logs);
                } else {
                    job.logs = Some(logs.to_string());
                }
            }
            Job::CompletedJobWithFormattedResult(job) => {
                if let Some(ref mut l) = job.cj.logs {
                    l.push_str(logs);
                } else {
                    job.cj.logs = Some(logs.to_string());
                }
            }
        }
    }

    pub fn log_len(&self) -> Option<usize> {
        match self {
            Job::QueuedJob(job) => job.logs.as_ref().map(|l| l.len()),
            Job::CompletedJob(job) => job.logs.as_ref().map(|l| l.len()),
            Job::CompletedJobWithFormattedResult(job) => job.cj.logs.as_ref().map(|l| l.len()),
        }
    }

    pub fn logs(&self) -> Option<String> {
        match self {
            Job::QueuedJob(job) => job.logs.clone(),
            Job::CompletedJob(job) => job.logs.clone(),
            Job::CompletedJobWithFormattedResult(job) => job.cj.logs.clone(),
        }
    }
    pub fn flow_status(&self) -> Option<FlowStatus> {
        match self {
            Job::QueuedJob(job) => job
                .flow_status
                .as_ref()
                .map(|rf| serde_json::from_str(rf.0.get()).ok())
                .flatten(),
            Job::CompletedJob(job) => job
                .flow_status
                .as_ref()
                .map(|rf| serde_json::from_str(rf.0.get()).ok())
                .flatten(),
            Job::CompletedJobWithFormattedResult(job) => job
                .cj
                .flow_status
                .as_ref()
                .map(|rf| serde_json::from_str(rf.0.get()).ok())
                .flatten(),
        }
    }
    pub fn is_flow_step(&self) -> bool {
        match self {
            Job::QueuedJob(job) => job.is_flow_step,
            Job::CompletedJob(job) => job.is_flow_step,
            Job::CompletedJobWithFormattedResult(job) => job.cj.is_flow_step,
        }
    }

    pub fn is_flow(&self) -> bool {
        matches!(
            self.job_kind(),
            JobKind::Flow | JobKind::FlowPreview | JobKind::SingleScriptFlow
        )
    }

    pub fn job_kind(&self) -> &JobKind {
        match self {
            Job::QueuedJob(job) => &job.job_kind,
            Job::CompletedJob(job) => &job.job_kind,
            Job::CompletedJobWithFormattedResult(job) => &job.cj.job_kind,
        }
    }

    pub fn id(&self) -> Uuid {
        match self {
            Job::QueuedJob(job) => job.id,
            Job::CompletedJob(job) => job.id,
            Job::CompletedJobWithFormattedResult(job) => job.cj.id,
        }
    }

    pub fn workspace_id(&self) -> &String {
        match self {
            Job::QueuedJob(job) => &job.workspace_id,
            Job::CompletedJob(job) => &job.workspace_id,
            Job::CompletedJobWithFormattedResult(job) => &job.cj.workspace_id,
        }
    }

    pub fn script_path(&self) -> &str {
        match self {
            Job::QueuedJob(job) => job.script_path.as_ref(),
            Job::CompletedJob(job) => job.script_path.as_ref(),
            Job::CompletedJobWithFormattedResult(job) => job.cj.script_path.as_ref(),
        }
        .map(String::as_str)
        .unwrap_or("tmp/main")
    }

    pub fn args(&self) -> Option<&sqlx::types::Json<HashMap<String, Box<RawValue>>>> {
        match self {
            Job::QueuedJob(job) => job.args.as_ref(),
            Job::CompletedJob(job) => job.args.as_ref(),
            Job::CompletedJobWithFormattedResult(job) => job.cj.args.as_ref(),
        }
    }

    pub fn full_path_with_workspace(&self) -> String {
        format!(
            "{}/{}/{}",
            self.workspace_id(),
            if self.is_flow() { "flow" } else { "script" },
            self.script_path(),
        )
    }

    pub async fn concurrency_key(
        &self,
        db: &Pool<Postgres>,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar!(
            "SELECT key FROM concurrency_key WHERE job_id = $1",
            self.id()
        )
        .fetch_optional(db)
        .await
    }
}

#[derive(sqlx::FromRow)]
pub struct UnifiedJob {
    pub workspace_id: String,
    pub typ: String,
    pub id: Uuid,
    pub parent_job: Option<Uuid>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    pub running: Option<bool>,
    pub script_hash: Option<ScriptHash>,
    pub script_path: Option<String>,
    pub duration_ms: Option<i64>,
    pub success: Option<bool>,
    pub deleted: bool,
    pub canceled: bool,
    pub canceled_by: Option<String>,
    pub job_kind: JobKind,
    pub schedule_path: Option<String>,
    pub permissioned_as: String,
    pub is_flow_step: bool,
    pub language: Option<ScriptLang>,
    pub is_skipped: bool,
    pub email: String,
    pub visible_to_owner: bool,
    pub suspend: Option<i32>,
    pub mem_peak: Option<i32>,
    pub tag: String,
    pub concurrent_limit: Option<i32>,
    pub concurrency_time_window_s: Option<i32>,
    pub priority: Option<i16>,
    pub labels: Option<serde_json::Value>,
}

const CJ_FIELDS: &[&str] = &[
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
    "priority",
    "result->'wm_labels' as labels",
];
const QJ_FIELDS: &[&str] = &[
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
    "priority",
    "null as labels",
];

impl UnifiedJob {
    pub fn completed_job_fields() -> &'static [&'static str] {
        CJ_FIELDS
    }
    pub fn queued_job_fields() -> &'static [&'static str] {
        QJ_FIELDS
    }
}

impl<'a> From<UnifiedJob> for Job {
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
                args: None,
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
                priority: uj.priority,
                labels: uj.labels,
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
                args: None,
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
                timeout: None,
                flow_step_id: None,
                cache_ttl: None,
                priority: uj.priority,
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
    Bundle,
}

#[derive(Deserialize)]
struct Preview {
    content: Option<String>,
    kind: Option<PreviewKind>,
    path: Option<String>,
    args: Option<HashMap<String, Box<JsonRawValue>>>,
    language: Option<ScriptLang>,
    tag: Option<String>,
    dedicated_worker: Option<bool>,
    lock: Option<String>,
}

#[derive(Deserialize)]
pub struct WorkflowTask {
    pub args: Option<HashMap<String, Box<JsonRawValue>>>,
}

#[derive(Deserialize)]
struct PreviewFlow {
    value: FlowValue,
    path: Option<String>,
    args: Option<HashMap<String, Box<JsonRawValue>>>,
    tag: Option<String>,
    restarted_from: Option<RestartedFrom>,
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

pub fn add_raw_string(
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

async fn check_tag_available_for_workspace(w_id: &str, tag: &Option<String>) -> error::Result<()> {
    if let Some(tag) = tag {
        if tag == "" {
            return Ok(());
        }
        let custom_tags_per_w = CUSTOM_TAGS_PER_WORKSPACE.read().await;
        if custom_tags_per_w.0.contains(&tag.to_string()) {
            Ok(())
        } else if custom_tags_per_w.1.contains_key(tag)
            && custom_tags_per_w
                .1
                .get(tag)
                .unwrap()
                .contains(&w_id.to_string())
        {
            Ok(())
        } else {
            return Err(error::Error::BadRequest(format!(
                "Tag {tag} cannot be used on workspace {w_id}: (CUSTOM_TAGS: {:?})",
                custom_tags_per_w
            )));
        }
    } else {
        Ok(())
    }
}

#[cfg(feature = "enterprise")]
pub async fn check_license_key_valid() -> error::Result<()> {
    use windmill_common::ee::LICENSE_KEY_VALID;

    let valid = *LICENSE_KEY_VALID.read().await;
    if !valid {
        return Err(Error::BadRequest(
            "License key is not valid. Go to your superadmin settings to update your license key."
                .to_string(),
        ));
    }
    Ok(())
}

pub async fn run_flow_by_path(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    args: PushArgs,
) -> error::Result<(StatusCode, String)> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;
    let flow_path = flow_path.to_path();
    check_scopes(&authed, || format!("run:flow/{flow_path}"))?;

    let (tag, dedicated_worker) = sqlx::query!(
        "SELECT tag, dedicated_worker from flow WHERE path = $1 and workspace_id = $2",
        flow_path,
        w_id
    )
    .fetch_optional(&db)
    .await?
    .map(|x| (x.tag, x.dedicated_worker))
    .unwrap_or_else(|| (None, None));

    let tag = run_query.tag.clone().or(tag);

    check_tag_available_for_workspace(&w_id, &tag).await?;
    let scheduled_for = run_query.get_scheduled_for(&db).await?;
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into(), rsmq);
    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::Flow { path: flow_path.to_string(), dedicated_worker },
        args,
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        scheduled_for,
        None,
        run_query.parent_job,
        run_query.root_job.or(run_query.parent_job),
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
        None,
        None,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

#[cfg(not(feature = "enterprise"))]
pub async fn restart_flow(
    _authed: ApiAuthed,
    Extension(_db): Extension<DB>,
    Extension(_user_db): Extension<UserDB>,
    Extension(_rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path((_w_id, _job_id, _step_id, _branch_or_iteration_n)): Path<(
        String,
        Uuid,
        String,
        Option<usize>,
    )>,
    Query(_run_query): Query<RunJobQuery>,
) -> error::Result<(StatusCode, String)> {
    return Err(Error::BadRequest(
        "Restarting a flow is a feature only available in enterprise version".to_string(),
    ));
}

#[cfg(feature = "enterprise")]
pub async fn restart_flow(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path((w_id, job_id, step_id, branch_or_iteration_n)): Path<(
        String,
        Uuid,
        String,
        Option<usize>,
    )>,
    Query(run_query): Query<RunJobQuery>,
) -> error::Result<(StatusCode, String)> {
    check_license_key_valid().await?;

    let completed_job = sqlx::query_as::<_, CompletedJob>(
        "SELECT *, result->'wm_labels' as labels from completed_job WHERE id = $1 and workspace_id = $2",
    )
    .bind(job_id)
    .bind(&w_id)
    .fetch_optional(&db)
    .await?
    .with_context(|| "Unable to find completed job with the given job UUID")?;

    let flow_path = completed_job
        .script_path
        .with_context(|| "No flow path set for completed flow job")?;
    check_scopes(&authed, || format!("run:flow/{flow_path}"))?;

    let push_args = completed_job
        .args
        .map(|json| PushArgs { args: json.0, extra: HashMap::new() })
        .unwrap_or_else(PushArgs::empty);

    let scheduled_for = run_query.get_scheduled_for(&db).await?;
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into(), rsmq);

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::RestartedFlow {
            completed_job_id: job_id,
            step_id: step_id,
            branch_or_iteration_n: branch_or_iteration_n,
        },
        push_args,
        &authed.username,
        &authed.email,
        username_to_permissioned_as(&authed.username),
        scheduled_for,
        None,
        run_query.parent_job,
        run_query.root_job.or(run_query.parent_job),
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        Some(completed_job.tag),
        None,
        None,
        completed_job.priority,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

pub async fn run_script_by_path(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    args: PushArgs,
) -> error::Result<(StatusCode, String)> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    let script_path = script_path.to_path();

    check_scopes(&authed, || format!("run:script/{script_path}"))?;

    let (job_payload, tag, _delete_after_use, timeout) =
        script_path_to_payload(script_path, &db, &w_id).await?;
    let scheduled_for = run_query.get_scheduled_for(&db).await?;

    let tag = run_query.tag.clone().or(tag);
    check_tag_available_for_workspace(&w_id, &tag).await?;
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into(), rsmq);

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        job_payload,
        args,
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        scheduled_for,
        None,
        run_query.parent_job,
        run_query.root_job.or(run_query.parent_job),
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
        timeout,
        None,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

pub async fn run_workflow_as_code(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path((w_id, job_id, entrypoint)): Path<(String, Uuid, String)>,
    Query(run_query): Query<RunJobQuery>,
    Json(task): Json<WorkflowTask>,
) -> error::Result<(StatusCode, String)> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;
    check_tag_available_for_workspace(&w_id, &run_query.tag).await?;

    let job = get_queued_job(&job_id, &w_id, &db).await?;
    let job = not_found_if_none(job, "Queued Job", &job_id.to_string())?;
    let (job_payload, tag, _delete_after_use, timeout) = match job.job_kind {
        JobKind::Preview => (
            JobPayload::Code(RawCode {
                hash: None,
                content: job.raw_code.unwrap_or_default(),
                path: job.script_path,
                language: job.language.unwrap_or_else(|| ScriptLang::Deno),
                lock: job.raw_lock,
                custom_concurrency_key: windmill_queue::custom_concurrency_key(&db, job.id)
                    .await
                    .map_err(to_anyhow)?,
                concurrent_limit: job.concurrent_limit,
                concurrency_time_window_s: job.concurrency_time_window_s,
                cache_ttl: job.cache_ttl,
                dedicated_worker: None,
            }),
            Some(job.tag.clone()),
            None,
            run_query.timeout,
        ),
        JobKind::Script => script_path_to_payload(job.script_path(), &db, &w_id).await?,
        _ => return Err(anyhow::anyhow!("Not supported").into()),
    };

    let mut extra = HashMap::new();
    extra.insert(ENTRYPOINT_OVERRIDE.to_string(), to_raw_value(&entrypoint));

    let args = PushArgs { args: task.args.unwrap_or_else(HashMap::new), extra };
    let scheduled_for = run_query.get_scheduled_for(&db).await?;

    let tag = run_query.tag.clone().or(tag).or(Some(job.tag));
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into(), rsmq);

    let (uuid, mut tx) = push(
        &db,
        tx,
        &w_id,
        job_payload,
        args,
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        scheduled_for,
        None,
        Some(job_id),
        job.root_job.or(Some(job_id)),
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
        timeout,
        None,
        None,
    )
    .await?;
    sqlx::query!(
        "UPDATE queue SET flow_status = jsonb_set(COALESCE(flow_status, '{}'::jsonb), array[$1], jsonb_set(jsonb_set('{}'::jsonb, '{scheduled_for}', to_jsonb(now()::text)), '{name}', to_jsonb($4::text))) WHERE id = $2 AND workspace_id = $3",
        uuid.to_string(),
        job_id,
        w_id,
        entrypoint
    ).execute(&mut tx).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

struct Guard {
    done: bool,
    id: Uuid,
    w_id: String,
    db: DB,
}

impl Drop for Guard {
    fn drop(&mut self) {
        if !&self.done {
            let id = self.id;
            let w_id = self.w_id.clone();
            let db = self.db.clone();

            tracing::info!("http connection broke, marking job {id} as canceled");
            tokio::spawn(async move {
                let _ = sqlx::query!(
                "UPDATE queue SET canceled = true, canceled_reason = 'http connection broke', canceled_by = queue.created_by WHERE id = $1 AND workspace_id = $2",
                id,
                w_id
            )
            .execute(&db)
            .await;
            });
        }
    }
}

#[derive(Deserialize)]
pub struct WindmillCompositeResult {
    windmill_status_code: Option<u16>,
    windmill_content_type: Option<String>,
    result: Option<Box<RawValue>>,
}
async fn run_wait_result(
    db: &DB,
    uuid: Uuid,
    w_id: String,
    node_id_for_empty_return: Option<String>,
) -> error::Result<Response> {
    let mut result = None;
    let timeout = SERVER_CONFIG.read().await.timeout_wait_result.clone();
    let timeout_ms = if timeout <= 0 {
        2000
    } else {
        (timeout * 1000) as u64
    };

    let mut g = Guard { done: false, id: uuid, w_id: w_id.clone(), db: db.clone() };

    let fast_poll_duration = *WAIT_RESULT_FAST_POLL_DURATION_SECS as u64 * 1000;
    let mut accumulated_delay = 0 as u64;

    loop {
        if let Some(node_id_for_empty_return) = node_id_for_empty_return.as_ref() {
            result = get_result_by_id_from_running_flow(
                &db,
                &w_id,
                &uuid,
                node_id_for_empty_return,
                None,
            )
            .await
            .ok();
        }

        if result.is_none() {
            let row = sqlx::query(
                "SELECT result, language, flow_status FROM completed_job WHERE id = $1 AND workspace_id = $2",
            )
            .bind(uuid)
            .bind(&w_id)
            .fetch_optional(db)
            .await?;
            if let Some(row) = row {
                let raw_result = RawResult::from_row(&row)?;
                result = match format_result(
                    raw_result.language.as_ref(),
                    raw_result.flow_status.map(|x| x.0),
                    raw_result.result.map(|x| x.0),
                ) {
                    FormattedResult::RawValue(rv) => rv,
                    FormattedResult::Vec(v) => Some(to_raw_value(&v)),
                };
            }
        }

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

        let composite_result = serde_json::from_str::<WindmillCompositeResult>(result.get());
        match composite_result {
            Ok(WindmillCompositeResult {
                windmill_status_code,
                windmill_content_type,
                result: result_value,
            }) => {
                if windmill_content_type.is_none() && windmill_status_code.is_none() {
                    return Ok(Json(result).into_response());
                }

                let status_code_or_default = windmill_status_code
                    .map(|val| match StatusCode::from_u16(val) {
                        Ok(sc) => Ok(sc),
                        Err(_) => Err(Error::ExecutionErr("Invalid status code".to_string())),
                    })
                    .unwrap_or(if result_value.is_some() {
                        Ok(StatusCode::OK)
                    } else {
                        Ok(StatusCode::NO_CONTENT)
                    })?;

                if windmill_content_type.is_some() {
                    let serialized_json_result = result_value
                        .map(|val| val.get().to_owned())
                        .unwrap_or_else(String::new);
                    // if the `result` was just a single string, the below removes the surrounding quotes by parsing it as a string.
                    // it falls back to the original serialized JSON if it doesn't work.
                    let serialized_result =
                        serde_json::from_str::<String>(serialized_json_result.as_str())
                            .ok()
                            .unwrap_or(serialized_json_result);
                    return Ok((
                        status_code_or_default,
                        [(
                            http::header::CONTENT_TYPE,
                            HeaderValue::from_str(windmill_content_type.unwrap().as_str()).unwrap(),
                        )],
                        serialized_result,
                    )
                        .into_response());
                }
                return Ok((
                    status_code_or_default,
                    Json(result_value), // default to JSON result if no content type is provided
                )
                    .into_response());
            }
            _ => Ok(Json(result).into_response()),
        }
    } else {
        Err(Error::ExecutionErr(format!("timeout after {}s", timeout)))
    }
}

async fn delete_job_metadata_after_use(db: &DB, job_uuid: Uuid) -> Result<(), Error> {
    sqlx::query!(
        "UPDATE completed_job
        SET logs = '##DELETED##', args = '{}'::jsonb, result = '{}'::jsonb
        WHERE id = $1",
        job_uuid,
    )
    .execute(db)
    .await?;
    sqlx::query!(
        "UPDATE job_logs
        SET logs = '##DELETED##'
        WHERE job_id = $1",
        job_uuid,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn check_queue_too_long(db: &DB, queue_limit: Option<i64>) -> error::Result<()> {
    if let Some(limit) = queue_limit {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM queue WHERE  canceled = false AND (scheduled_for <= now()
        OR (suspend_until IS NOT NULL
            AND (   suspend <= 0
                 OR suspend_until <= now())))",
        )
        .fetch_one(db)
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
    authed: ApiAuthed,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    DecodeQueries(queries): DecodeQueries,
) -> error::Result<Response> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    if method == http::Method::HEAD {
        return Ok(Json(serde_json::json!("")).into_response());
    }
    let payload_r = run_query
        .payload
        .map(decode_payload)
        .map(|x| x.map_err(|e| Error::InternalErr(e.to_string())));

    let mut payload_args = if let Some(payload) = payload_r {
        payload?
    } else {
        HashMap::new()
    };
    queries.iter().for_each(|(k, v)| {
        payload_args.insert(k.to_string(), v.clone());
    });

    let inner_args: HashMap<String, Box<RawValue>> = HashMap::new();
    let args = PushArgs { extra: payload_args, args: inner_args };

    check_queue_too_long(&db, QUEUE_LIMIT_WAIT_RESULT.or(run_query.queue_limit)).await?;
    let script_path = script_path.to_path();
    check_scopes(&authed, || format!("run:script/{script_path}"))?;

    let (job_payload, tag, delete_after_use, timeout) =
        script_path_to_payload(script_path, &db, &w_id).await?;

    let tag = run_query.tag.clone().or(tag);
    check_tag_available_for_workspace(&w_id, &tag).await?;
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into(), rsmq);

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        job_payload,
        args,
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        None,
        None,
        run_query.parent_job,
        run_query.root_job.or(run_query.parent_job),
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
        timeout,
        None,
        None,
    )
    .await?;
    tx.commit().await?;

    let wait_result = run_wait_result(&db, uuid, w_id, None).await;
    if delete_after_use.unwrap_or(false) {
        delete_job_metadata_after_use(&db, uuid).await?;
    }
    return wait_result;
}

pub async fn run_wait_result_flow_by_path_get(
    method: hyper::http::Method,
    authed: ApiAuthed,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    DecodeQueries(queries): DecodeQueries,
) -> error::Result<Response> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    if method == http::Method::HEAD {
        return Ok(Json(serde_json::json!("")).into_response());
    }
    let payload_r = run_query
        .payload
        .clone()
        .map(decode_payload)
        .map(|x| x.map_err(|e| Error::InternalErr(e.to_string())));

    let mut payload_args = if let Some(payload) = payload_r {
        payload?
    } else {
        HashMap::new()
    };

    queries.iter().for_each(|(k, v)| {
        payload_args.insert(k.to_string(), v.clone());
    });

    let args = PushArgs { extra: payload_args, args: HashMap::new() };

    run_wait_result_flow_by_path_internal(
        db, run_query, flow_path, authed, rsmq, user_db, args, w_id,
    )
    .await
}

pub async fn run_wait_result_script_by_path(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(db): Extension<DB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    args: PushArgs,
) -> error::Result<Response> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    run_wait_result_script_by_path_internal(
        db,
        run_query,
        script_path,
        authed,
        rsmq,
        user_db,
        w_id,
        args,
    )
    .await
}

async fn run_wait_result_script_by_path_internal(
    db: sqlx::Pool<Postgres>,
    run_query: RunJobQuery,
    script_path: StripPath,
    authed: ApiAuthed,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    user_db: UserDB,
    w_id: String,
    args: PushArgs,
) -> error::Result<Response> {
    check_queue_too_long(&db, QUEUE_LIMIT_WAIT_RESULT.or(run_query.queue_limit)).await?;
    let script_path = script_path.to_path();
    check_scopes(&authed, || format!("run:script/{script_path}"))?;

    let (job_payload, tag, delete_after_use, timeout) =
        script_path_to_payload(script_path, &db, &w_id).await?;

    let tag = run_query.tag.clone().or(tag);
    check_tag_available_for_workspace(&w_id, &tag).await?;
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into(), rsmq);

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        job_payload,
        args,
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        None,
        None,
        run_query.parent_job,
        run_query.root_job.or(run_query.parent_job),
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
        timeout,
        None,
        None,
    )
    .await?;
    tx.commit().await?;

    let wait_result = run_wait_result(&db, uuid, w_id, None).await;
    if delete_after_use.unwrap_or(false) {
        delete_job_metadata_after_use(&db, uuid).await?;
    }
    return wait_result;
}

pub async fn run_wait_result_script_by_hash(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(db): Extension<DB>,
    Path((w_id, script_hash)): Path<(String, ScriptHash)>,
    Query(run_query): Query<RunJobQuery>,
    args: PushArgs,
) -> error::Result<Response> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    check_queue_too_long(&db, run_query.queue_limit).await?;

    let hash = script_hash.0;
    let (
        path,
        tag,
        custom_concurrency_key,
        concurrent_limit,
        concurrency_time_window_s,
        mut cache_ttl,
        language,
        dedicated_worker,
        priority,
        delete_after_use,
        timeout,
    ) = get_path_tag_limits_cache_for_hash(&db, &w_id, hash).await?;
    if let Some(run_query_cache_ttl) = run_query.cache_ttl {
        cache_ttl = Some(run_query_cache_ttl);
    }
    check_scopes(&authed, || format!("run:script/{path}"))?;

    let tag = run_query.tag.clone().or(tag);
    check_tag_available_for_workspace(&w_id, &tag).await?;
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into(), rsmq);

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::ScriptHash {
            hash: ScriptHash(hash),
            path: path,
            custom_concurrency_key,
            concurrent_limit: concurrent_limit,
            concurrency_time_window_s: concurrency_time_window_s,
            cache_ttl,
            language,
            dedicated_worker,
            priority,
        },
        args,
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        None,
        None,
        run_query.parent_job,
        run_query.root_job.or(run_query.parent_job),
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
        timeout,
        None,
        None,
    )
    .await?;
    tx.commit().await?;

    let wait_result = run_wait_result(&db, uuid, w_id, None).await;
    if delete_after_use.unwrap_or(false) {
        delete_job_metadata_after_use(&db, uuid).await?;
    }
    return wait_result;
}

pub async fn run_wait_result_flow_by_path(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Extension(db): Extension<DB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    args: PushArgs,
) -> error::Result<Response> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    run_wait_result_flow_by_path_internal(
        db, run_query, flow_path, authed, rsmq, user_db, args, w_id,
    )
    .await
}

async fn run_wait_result_flow_by_path_internal(
    db: sqlx::Pool<Postgres>,
    run_query: RunJobQuery,
    flow_path: StripPath,
    authed: ApiAuthed,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    user_db: UserDB,
    args: PushArgs,
    w_id: String,
) -> error::Result<Response> {
    check_queue_too_long(&db, run_query.queue_limit).await?;

    let flow_path = flow_path.to_path();
    check_scopes(&authed, || format!("run:flow/{flow_path}"))?;

    let scheduled_for = run_query.get_scheduled_for(&db).await?;

    let (tag, dedicated_worker, early_return) = sqlx::query!(
        "SELECT tag, dedicated_worker, value->>'early_return' as early_return from flow WHERE path = $1 and workspace_id = $2",
        flow_path,
        w_id
    )
    .fetch_optional(&db)
    .await?
    .map(|x| (x.tag, x.dedicated_worker, x.early_return))
    .unwrap_or_else(|| (None, None, None));

    let tag = run_query.tag.clone().or(tag);
    check_tag_available_for_workspace(&w_id, &tag).await?;
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into(), rsmq);

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::Flow { path: flow_path.to_string(), dedicated_worker },
        args,
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        scheduled_for,
        None,
        run_query.parent_job,
        run_query.root_job.or(run_query.parent_job),
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
        None,
        None,
        None,
    )
    .await?;
    tx.commit().await?;

    run_wait_result(&db, uuid, w_id, early_return).await
}

async fn run_preview_script(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path(w_id): Path<String>,
    Query(run_query): Query<RunJobQuery>,
    Json(preview): Json<Preview>,
) -> error::Result<(StatusCode, String)> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    check_scopes(&authed, || format!("runscript"))?;
    if authed.is_operator {
        return Err(error::Error::NotAuthorized(
            "Operators cannot run preview jobs for security reasons".to_string(),
        ));
    }
    let scheduled_for = run_query.get_scheduled_for(&db).await?;
    let tag = run_query.tag.clone().or(preview.tag.clone());
    check_tag_available_for_workspace(&w_id, &tag).await?;
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into(), rsmq);

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        match preview.kind {
            Some(PreviewKind::Identity) => JobPayload::Identity,
            Some(PreviewKind::Noop) => JobPayload::Noop,
            _ => JobPayload::Code(RawCode {
                hash: None,
                content: preview.content.unwrap_or_default(),
                path: preview.path,
                language: preview.language.unwrap_or(ScriptLang::Deno),
                lock: preview.lock,
                custom_concurrency_key: None,
                concurrent_limit: None, // TODO(gbouv): once I find out how to store limits in the content of a script, should be easy to plug limits here
                concurrency_time_window_s: None, // TODO(gbouv): same as above
                cache_ttl: None,
                dedicated_worker: preview.dedicated_worker,
            }),
        },
        preview.args.unwrap_or_default().into(),
        authed.display_username(),
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
        tag,
        run_query.timeout,
        None,
        None,
    )
    .await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, uuid.to_string()))
}

#[cfg(all(feature = "enterprise", feature = "parquet"))]
async fn run_bundle_preview_script(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path(w_id): Path<String>,
    Query(run_query): Query<RunJobQuery>,
    mut multipart: axum::extract::Multipart,
) -> error::Result<(StatusCode, String)> {
    check_license_key_valid().await?;

    check_scopes(&authed, || format!("runscript"))?;
    if authed.is_operator {
        return Err(error::Error::NotAuthorized(
            "Operators cannot run preview jobs for security reasons".to_string(),
        ));
    }

    let mut job_id = None;
    let mut tx = None;
    let mut uploaded = false;
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();
        if name == "preview" {
            let preview: Preview = serde_json::from_slice(&data).map_err(to_anyhow)?;

            let scheduled_for = run_query.get_scheduled_for(&db).await?;
            let tag = run_query.tag.clone().or(preview.tag.clone());
            check_tag_available_for_workspace(&w_id, &tag).await?;
            let ltx =
                PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into(), rsmq.clone());

            let args = preview.args.unwrap_or_default();

            // hmap.insert("")
            let (uuid, ntx) = push(
                &db,
                ltx,
                &w_id,
                match preview.kind {
                    Some(PreviewKind::Identity) => JobPayload::Identity,
                    Some(PreviewKind::Noop) => JobPayload::Noop,
                    _ => JobPayload::Code(RawCode {
                        hash: Some(PREVIEW_IS_CODEBASE_HASH),
                        content: preview.content.unwrap_or_default(),
                        path: preview.path,
                        language: preview.language.unwrap_or(ScriptLang::Deno),
                        lock: preview.lock,
                        concurrent_limit: None, // TODO(gbouv): once I find out how to store limits in the content of a script, should be easy to plug limits here
                        concurrency_time_window_s: None, // TODO(gbouv): same as above
                        cache_ttl: None,
                        dedicated_worker: preview.dedicated_worker,
                        custom_concurrency_key: None,
                    }),
                },
                args.into(),
                authed.display_username(),
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
                tag,
                run_query.timeout,
                None,
                None,
            )
            .await?;
            job_id = Some(uuid);
            tx = Some(ntx);
        }
        if name == "file" {
            let id = job_id
                .as_ref()
                .ok_or_else(|| {
                    Error::BadRequest(
                        "script need to be passed first in the multipart upload".to_string(),
                    )
                })?
                .to_string();

            uploaded = true;

            if let Some(os) = windmill_common::s3_helpers::OBJECT_STORE_CACHE_SETTINGS
                .read()
                .await
                .clone()
            {
                let path = windmill_common::s3_helpers::bundle(&w_id, &id);
                if let Err(e) = os
                    .put(&object_store::path::Path::from(path.clone()), data)
                    .await
                {
                    tracing::info!("Failed to put snapshot to s3 at {path}: {:?}", e);
                    return Err(Error::ExecutionErr(format!("Failed to put {path} to s3")));
                }
            } else {
                return Err(Error::BadConfig("Object store is required for snapshot script and is not configured for servers".to_string()));
            }
        }
        // println!("Length of `{}` is {} bytes", name, data.len());
    }
    if !uploaded {
        return Err(Error::BadRequest("No file uploaded".to_string()));
    }
    if job_id.is_none() {
        return Err(Error::BadRequest(
            "No script found in the uploaded file".to_string(),
        ));
    }

    tx.unwrap().commit().await?;

    Ok((StatusCode::CREATED, job_id.unwrap().to_string()))
}

#[cfg(not(all(feature = "enterprise", feature = "parquet")))]
async fn run_bundle_preview_script() -> error::Result<(StatusCode, String)> {
    return Err(Error::BadRequest(
        "bundle preview is an ee feature".to_string(),
    ));
}

#[derive(Deserialize)]
pub struct RunDependenciesRequest {
    pub raw_scripts: Vec<RawScriptForDependencies>,
    pub entrypoint: String,
    pub raw_deps: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct RawScriptForDependencies {
    pub script_path: String,
    pub raw_code: Option<String>,
    pub language: ScriptLang,
}

#[derive(Serialize)]
pub struct RunDependenciesResponse {
    pub dependencies: String,
}

pub async fn run_dependencies_job(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path(w_id): Path<String>,
    Json(req): Json<RunDependenciesRequest>,
) -> error::Result<Response> {
    check_scopes(&authed, || format!("runscript"))?;
    if authed.is_operator {
        return Err(error::Error::NotAuthorized(
            "Operators cannot run dependencies jobs for security reasons".to_string(),
        ));
    }

    if req.raw_scripts.len() != 1 || req.raw_scripts[0].script_path != req.entrypoint {
        return Err(error::Error::InternalErr(
            "For now only a single raw script can be passed to this endpoint, and the entrypoint should be set to the script path".to_string(),
        ));
    }
    let raw_script = req.raw_scripts[0].clone();
    let script_path = raw_script.script_path;
    let (args, raw_code) = if let Some(deps) = req.raw_deps {
        let mut hm = HashMap::new();
        hm.insert(
            "raw_deps".to_string(),
            JsonRawValue::from_string("true".to_string()).unwrap(),
        );
        (PushArgs { extra: hm, args: HashMap::new() }, deps)
    } else {
        (
            PushArgs::empty(),
            raw_script.raw_code.unwrap_or_else(|| "".to_string()),
        )
    };

    let language = raw_script.language;

    let (uuid, tx) = push(
        &db,
        PushIsolationLevel::IsolatedRoot(db.clone(), rsmq),
        &w_id,
        JobPayload::RawScriptDependencies {
            script_path: script_path,
            content: raw_code,
            language: language,
        },
        args,
        authed.display_username(),
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
        None,
        None,
        None,
    )
    .await?;
    tx.commit().await?;

    let wait_result = run_wait_result(&db, uuid, w_id, None).await;
    wait_result
}

#[derive(Deserialize)]
struct BatchInfo {
    kind: String,
    flow_value: Option<FlowValue>,
    path: Option<String>,
}

#[tracing::instrument(level = "trace", skip_all)]
async fn add_batch_jobs(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path((w_id, n)): Path<(String, i32)>,
    Json(batch_info): Json<BatchInfo>,
) -> error::JsonResult<Vec<Uuid>> {
    require_super_admin(&db, &authed.email).await?;

    let (
        hash,
        path,
        job_kind,
        language,
        dedicated_worker,
        _custom_concurrency_key,
        concurrent_limit,
        concurrent_time_window_s,
        timeout,
    ) = match batch_info.kind.as_str() {
        "script" => {
            if let Some(path) = batch_info.path {
                let (
                    script_hash,
                    _tag,
                    custom_concurrency_key,
                    concurrent_limit,
                    concurrency_time_window_s,
                    _cache_ttl,
                    language,
                    dedicated_worker,
                    _priority,
                    _delete_after_use,
                    timeout,
                ) = get_latest_deployed_hash_for_path(&db, &w_id, &path).await?;
                (
                    Some(script_hash),
                    Some(path),
                    JobKind::Script,
                    Some(language),
                    dedicated_worker,
                    custom_concurrency_key,
                    concurrent_limit,
                    concurrency_time_window_s,
                    timeout,
                )
            } else {
                Err(anyhow::anyhow!(
                    "Path is required if no value is not provided"
                ))?
            }
        }
        "flow" => {
            let mut tx = PushIsolationLevel::IsolatedRoot(db.clone(), rsmq);

            let mut uuids: Vec<Uuid> = Vec::new();
            let payload = if let Some(ref fv) = batch_info.flow_value {
                JobPayload::RawFlow { value: fv.clone(), path: None, restarted_from: None }
            } else {
                if let Some(path) = batch_info.path.as_ref() {
                    JobPayload::Flow { path: path.to_string(), dedicated_worker: None }
                } else {
                    Err(anyhow::anyhow!(
                        "Path is required if no value is not provided"
                    ))?
                }
            };
            for _ in 0..n {
                let (uuid, ntx) = push(
                    &db,
                    tx,
                    &w_id,
                    payload.clone(),
                    PushArgs::empty(),
                    authed.display_username(),
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
                    None,
                    None,
                    None,
                )
                .await?;
                tx = PushIsolationLevel::Transaction(ntx);
                uuids.push(uuid);
            }
            match tx {
                PushIsolationLevel::Transaction(tx) => {
                    tx.commit().await?;
                }
                _ => (),
            }
            return Ok(Json(uuids));
        }
        "noop" => (
            None,
            None,
            JobKind::Noop,
            None,
            None,
            None,
            None,
            None,
            None,
        ),
        _ => {
            return Err(error::Error::BadRequest(format!(
                "Invalid batch kind: {}",
                batch_info.kind
            )))
        }
    };

    let language = language.unwrap_or(ScriptLang::Deno);

    let tag = if let Some(dedicated_worker) = dedicated_worker {
        if dedicated_worker && path.is_some() {
            format!("{}:{}", w_id, path.clone().unwrap())
        } else {
            format!("{}", language.as_str())
        }
    } else {
        format!("{}", language.as_str())
    };

    let uuids = sqlx::query_scalar!(
        r#"WITH uuid_table as (
            select gen_random_uuid() as uuid from generate_series(1, $11)
        )
        INSERT INTO queue 
            (id, script_hash, script_path, job_kind, language, args, tag, created_by, permissioned_as, email, scheduled_for, workspace_id, concurrent_limit, concurrency_time_window_s, timeout)
            (SELECT uuid, $1, $2, $3, $4, ('{ "uuid": "' || uuid || '" }')::jsonb, $5, $6, $7, $8, $9, $10, $12, $13, $14 FROM uuid_table) 
        RETURNING id"#,
            hash.map(|h| h.0),
            path,
            job_kind.clone() as JobKind,
            language as ScriptLang,
            tag,
            authed.username,
            username_to_permissioned_as(&authed.username),
            authed.email,
            Utc::now(),
            w_id,
            n,
            concurrent_limit,
            concurrent_time_window_s,
            timeout
        )
        .fetch_all(&db)
        .await?;

    Ok(Json(uuids))
}

async fn run_preview_flow_job(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path(w_id): Path<String>,
    Query(run_query): Query<RunJobQuery>,
    Json(raw_flow): Json<PreviewFlow>,
) -> error::Result<(StatusCode, String)> {
    check_scopes(&authed, || format!("runflow"))?;
    if authed.is_operator {
        return Err(error::Error::NotAuthorized(
            "Operators cannot run preview jobs for security reasons".to_string(),
        ));
    }
    let scheduled_for = run_query.get_scheduled_for(&db).await?;
    let tag = run_query.tag.clone().or(raw_flow.tag.clone());
    check_tag_available_for_workspace(&w_id, &tag).await?;
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into(), rsmq);

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::RawFlow {
            value: raw_flow.value,
            path: raw_flow.path,
            restarted_from: raw_flow.restarted_from,
        },
        raw_flow.args.unwrap_or_default().into(),
        authed.display_username(),
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
        tag,
        None,
        None,
        None,
    )
    .await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, uuid.to_string()))
}

pub async fn run_job_by_hash(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,

    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path((w_id, script_hash)): Path<(String, ScriptHash)>,
    Query(run_query): Query<RunJobQuery>,
    args: PushArgs,
) -> error::Result<(StatusCode, String)> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    let hash = script_hash.0;
    let (
        path,
        tag,
        custom_concurrency_key,
        concurrent_limit,
        concurrency_time_window_s,
        mut cache_ttl,
        language,
        dedicated_worker,
        priority,
        _delete_after_use, // not taken into account in async endpoints
        timeout,
    ) = get_path_tag_limits_cache_for_hash(&db, &w_id, hash).await?;
    check_scopes(&authed, || format!("run:script/{path}"))?;
    if let Some(run_query_cache_ttl) = run_query.cache_ttl {
        cache_ttl = Some(run_query_cache_ttl);
    }
    let scheduled_for = run_query.get_scheduled_for(&db).await?;
    let tag = run_query.tag.clone().or(tag);

    check_tag_available_for_workspace(&w_id, &tag).await?;
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into(), rsmq);

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::ScriptHash {
            hash: ScriptHash(hash),
            path: path,
            custom_concurrency_key,
            concurrent_limit: concurrent_limit,
            concurrency_time_window_s: concurrency_time_window_s,
            cache_ttl,
            language,
            dedicated_worker,
            priority,
        },
        args,
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        scheduled_for,
        None,
        run_query.parent_job,
        run_query.root_job.or(run_query.parent_job),
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
        timeout,
        None,
        None,
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
    pub log_offset: Option<i32>,
    pub mem_peak: Option<i32>,
    pub flow_status: Option<Box<serde_json::value::RawValue>>,
}

async fn get_log_file(Path((_w_id, file_p)): Path<(String, String)>) -> error::Result<Response> {
    let local_file = format!("{TMP_DIR}/logs/{file_p}");
    if tokio::fs::metadata(&local_file).await.is_ok() {
        let mut file = tokio::fs::File::open(local_file).await.map_err(to_anyhow)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await.map_err(to_anyhow)?;
        let res = Response::builder()
            .header(http::header::CONTENT_TYPE, "text/plain")
            .body(Body::from(bytes::Bytes::from(buffer)))
            .unwrap();
        return Ok(res);
    }

    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    if let Some(os) = OBJECT_STORE_CACHE_SETTINGS.read().await.clone() {
        let file = os
            .get(&object_store::path::Path::from(format!("logs/{file_p}")))
            .await;
        if let Ok(file) = file {
            if let Ok(bytes) = file.bytes().await {
                use axum::http::header;
                let res = Response::builder()
                    .header(header::CONTENT_TYPE, "text/plain")
                    .body(Body::from(bytes::Bytes::from(bytes)))
                    .unwrap();
                return Ok(res);
            } else {
                return Err(error::Error::InternalErr(format!(
                    "Error getting bytes from file: {}",
                    file_p
                )));
            }
        } else {
            return Err(error::Error::NotFound(format!(
                "File not found: {}",
                file_p
            )));
        }
    } else {
        return Err(error::Error::InternalErr(
            "Object store client not present, cannot stream logs from store".to_string(),
        ));
    }

    return Err(error::Error::NotFound(format!(
        "File not found on server logs volume /tmp/windmill/logs and no distributed logs s3 storage for {}",
        file_p
    )));
}

#[derive(Deserialize, sqlx::FromRow)]
pub struct JobUpdateRow {
    pub running: bool,
    pub logs: Option<String>,
    pub mem_peak: Option<i32>,
    pub flow_status: Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
    pub log_offset: Option<i32>,
}
async fn get_job_update(
    Extension(db): Extension<DB>,
    Path((w_id, job_id)): Path<(String, Uuid)>,
    Query(JobUpdateQuery { running, log_offset }): Query<JobUpdateQuery>,
) -> error::JsonResult<JobUpdate> {
    let record = sqlx::query_as::<_, JobUpdateRow>(
        "SELECT running, substr(concat(coalesce(queue.logs, ''), job_logs.logs), greatest($1 - job_logs.log_offset, 0)) as logs, mem_peak, 
        CASE WHEN is_flow_step is true then NULL else flow_status END as flow_status,
        job_logs.log_offset + char_length(job_logs.logs) + 1 as log_offset
        FROM queue
        LEFT JOIN job_logs ON job_logs.job_id =  queue.id 
        WHERE queue.workspace_id = $2 AND queue.id = $3",
    )
    .bind(log_offset)
    .bind(&w_id)
    .bind(&job_id)
    .fetch_optional(&db)
    .await?;

    if let Some(record) = record {
        Ok(Json(JobUpdate {
            running: if !running && record.running {
                Some(true)
            } else {
                None
            },
            log_offset: record.log_offset,
            completed: None,
            new_logs: record.logs,
            mem_peak: record.mem_peak,
            flow_status: record
                .flow_status
                .map(|x: sqlx::types::Json<Box<RawValue>>| x.0),
        }))
    } else {
        let record = sqlx::query_as::<_, JobUpdateRow>(
            "SELECT false as running, substr(concat(coalesce(completed_job.logs, ''), job_logs.logs), greatest($1 - job_logs.log_offset, 0))  as logs, mem_peak, 
            CASE WHEN is_flow_step is true then NULL else flow_status END as flow_status,
            job_logs.log_offset + char_length(job_logs.logs) + 1 as log_offset
            FROM completed_job 
            LEFT JOIN job_logs ON job_logs.job_id = completed_job.id 
            WHERE completed_job.workspace_id = $2 AND id = $3",
        )
        .bind(log_offset)
        .bind(&w_id)
        .bind(&job_id)
        .fetch_optional(&db)
        .await?;
        if let Some(record) = record {
            Ok(Json(JobUpdate {
                running: Some(false),
                completed: Some(true),
                log_offset: record.log_offset,
                new_logs: record.logs,
                mem_peak: record.mem_peak,
                flow_status: record
                    .flow_status
                    .map(|x: sqlx::types::Json<Box<RawValue>>| x.0),
            }))
        } else {
            Err(error::Error::NotFound(format!("Job not found: {}", job_id)))
        }
    }
}

pub fn filter_list_completed_query(
    mut sqlb: SqlBuilder,
    lq: &ListCompletedQuery,
    w_id: &str,
) -> SqlBuilder {
    if w_id != "admins" || !lq.all_workspaces.is_some_and(|x| x) {
        sqlb.and_where_eq("workspace_id", "?".bind(&w_id));
    }

    if let Some(p) = &lq.schedule_path {
        sqlb.and_where_eq("schedule_path", "?".bind(p));
    }

    if let Some(ps) = &lq.script_path_start {
        sqlb.and_where_like_left("script_path", ps);
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
        sqlb.and_where_le(
            "started_at",
            format!("to_timestamp({}  / 1000.0)", dt.timestamp_millis()),
        );
    }
    if let Some(dt) = &lq.started_after {
        sqlb.and_where_ge(
            "started_at",
            format!("to_timestamp({}  / 1000.0)", dt.timestamp_millis()),
        );
    }

    if let Some(dt) = &lq.created_or_started_before {
        sqlb.and_where_le(
            "started_at",
            format!("to_timestamp({}  / 1000.0)", dt.timestamp_millis()),
        );
    }
    if let Some(dt) = &lq.created_or_started_after {
        sqlb.and_where_ge(
            "started_at",
            format!("to_timestamp({}  / 1000.0)", dt.timestamp_millis()),
        );
    }

    if let Some(sk) = &lq.is_skipped {
        sqlb.and_where_eq("is_skipped", sk);
    }
    if let Some(fs) = &lq.is_flow_step {
        sqlb.and_where_eq("is_flow_step", fs);
    }
    if let Some(fs) = &lq.has_null_parent {
        if *fs {
            sqlb.and_where_is_null("parent_job");
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

    if let Some(result) = &lq.result {
        sqlb.and_where("result @> ?".bind(&result.replace("'", "''")));
    }

    if let Some(label) = &lq.label {
        let mut wh = format!("result->'wm_labels' ? ");
        wh.push_str(&format!("'{}'", &label.replace("'", "''")));
        sqlb.and_where(&wh);
        sqlb.and_where("result ? 'wm_labels'");
    }

    if lq.is_not_schedule.unwrap_or(false) {
        sqlb.and_where("schedule_path IS null");
    }

    sqlb
}

pub fn list_completed_jobs_query(
    w_id: &str,
    per_page: usize,
    offset: usize,
    lq: &ListCompletedQuery,
    fields: &[&str],
) -> SqlBuilder {
    let sqlb = SqlBuilder::select_from("completed_job")
        .fields(fields)
        .order_by("created_at", lq.order_desc.unwrap_or(true))
        .offset(offset)
        .limit(per_page)
        .clone();

    filter_list_completed_query(sqlb, lq, w_id)
}
#[derive(Deserialize, Clone)]
pub struct ListCompletedQuery {
    pub script_path_start: Option<String>,
    pub script_path_exact: Option<String>,
    pub script_hash: Option<String>,
    pub created_by: Option<String>,
    pub started_before: Option<chrono::DateTime<chrono::Utc>>,
    pub started_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_or_started_before: Option<chrono::DateTime<chrono::Utc>>,
    pub created_or_started_after: Option<chrono::DateTime<chrono::Utc>>,
    pub success: Option<bool>,
    pub running: Option<bool>,
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
    pub scheduled_for_before_now: Option<bool>,
    pub all_workspaces: Option<bool>,
    pub has_null_parent: Option<bool>,
    pub label: Option<String>,
    pub is_not_schedule: Option<bool>,
}

async fn list_completed_jobs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListCompletedQuery>,
) -> error::JsonResult<Vec<ListableCompletedJob>> {
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
            "priority",
            "result->'wm_labels' as labels",
            "'CompletedJob' as type",
        ],
    )
    .sql()?;
    let mut tx = user_db.begin(&authed).await?;
    let jobs = sqlx::query_as::<_, ListableCompletedJob>(&sql)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(jobs))
}

async fn get_completed_job<'a>(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::Result<Response> {
    let job_o = sqlx::query("SELECT id, workspace_id, parent_job, created_by, created_at, duration_ms, success, script_hash, script_path, 
    CASE WHEN args is null or pg_column_size(args) < 90000 THEN args ELSE '\"WINDMILL_TOO_BIG\"'::jsonb END as args, CASE WHEN result is null or pg_column_size(result) < 90000 THEN result ELSE '\"WINDMILL_TOO_BIG\"'::jsonb END as result, logs, deleted, raw_code, canceled, canceled_by, canceled_reason, job_kind, env_id,
    schedule_path, permissioned_as, flow_status, raw_flow, is_flow_step, language, started_at, is_skipped,
    raw_lock, email, visible_to_owner, mem_peak, tag, priority, result->'wm_labels' as labels FROM completed_job WHERE id = $1 AND workspace_id = $2")
        .bind(id)
        .bind(w_id)
        .fetch_optional(&db)
        .await?;

    let job = not_found_if_none(job_o, "Completed Job", id.to_string())?;

    let cj = CompletedJob::from_row(&job)?;

    let cj = format_completed_job_result(cj);

    let response = Json(cj).into_response();
    // let extra_log = query_scalar!(
    //     "SELECT substr(logs, $1) as logs FROM large_logs WHERE workspace_id = $2 AND job_id = $3",
    //     log_offset - len,
    //     &w_id,
    //     &job_id
    // )
    // .fetch_optional(db)
    // .await.ok().flatten().flatten();
    Ok(response)
}

#[derive(FromRow)]
pub struct RawResult {
    pub result: Option<sqlx::types::Json<Box<RawValue>>>,
    pub flow_status: Option<sqlx::types::Json<Box<RawValue>>>,
    pub language: Option<ScriptLang>,
}

#[derive(FromRow)]
pub struct RawResultWithSuccess {
    pub result: Option<sqlx::types::Json<Box<RawValue>>>,
    pub flow_status: Option<sqlx::types::Json<Box<RawValue>>>,
    pub language: Option<ScriptLang>,
    pub success: bool,
}

async fn get_completed_job_result(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Query(JsonPath { json_path }): Query<JsonPath>,
) -> error::Result<Response> {
    let result_o = if let Some(json_path) = json_path {
        sqlx::query(
            "SELECT result #> $3 as result, flow_status, language FROM completed_job WHERE id = $1 AND workspace_id = $2",
        )
        .bind(id)
        .bind(w_id)
        .bind(
            json_path
                .split(".")
                .map(|x| x.to_string())
                .collect::<Vec<_>>(),
        )
        .fetch_optional(&db)
        .await?
    } else {
        sqlx::query("SELECT result, flow_status, language FROM completed_job WHERE id = $1 AND workspace_id = $2")
            .bind(id)
            .bind(w_id)
            .fetch_optional(&db)
            .await?
    };

    let result = not_found_if_none(result_o, "Completed Job", id.to_string())?;

    let raw_result = RawResult::from_row(&result)?;

    let result = format_result(
        raw_result.language.as_ref(),
        raw_result.flow_status.map(|x| x.0),
        raw_result.result.map(|x| x.0),
    );

    Ok(Json(result).into_response())
}

#[derive(Serialize)]
struct CompletedJobResult {
    started: Option<bool>,
    success: Option<bool>,
    completed: bool,
    result: Option<FormattedResult>,
}

#[derive(Deserialize)]
struct GetCompletedJobQuery {
    get_started: Option<bool>,
}

async fn get_completed_job_result_maybe(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Query(GetCompletedJobQuery { get_started }): Query<GetCompletedJobQuery>,
) -> error::Result<Response> {
    let result_o = sqlx::query(
        "SELECT result, success, language, flow_status FROM completed_job WHERE id = $1 AND workspace_id = $2",
    )
    .bind(id)
    .bind(&w_id)
    .fetch_optional(&db)
    .await?;

    if let Some(result) = result_o {
        let res = RawResultWithSuccess::from_row(&result)?;
        let result = format_result(
            res.language.as_ref(),
            res.flow_status.map(|x| x.0),
            res.result.map(|x| x.0),
        );
        Ok(Json(CompletedJobResult {
            started: Some(true),
            success: Some(res.success),
            completed: true,
            result: Some(result),
        })
        .into_response())
    } else if get_started.is_some_and(|x| x) {
        let started = sqlx::query_scalar!(
            "SELECT running FROM queue WHERE id = $1 AND workspace_id = $2",
            id,
            w_id
        )
        .fetch_optional(&db)
        .await?
        .unwrap_or(false);
        Ok(Json(CompletedJobResult {
            started: Some(started),
            completed: false,
            success: None,
            result: None,
        })
        .into_response())
    } else {
        Ok(Json(CompletedJobResult {
            started: None,
            completed: false,
            success: None,
            result: None,
        })
        .into_response())
    }
}

async fn delete_completed_job<'a>(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::Result<Response> {
    check_scopes(&authed, || format!("deletejob"))?;

    let mut tx = user_db.begin(&authed).await?;

    require_admin(authed.is_admin, &authed.username)?;
    let job_o = sqlx::query(
        "UPDATE completed_job SET args = null, logs = '', result = null, deleted = true WHERE id = $1 AND workspace_id = $2 \
         RETURNING *",
    )
    .bind(id)
    .bind(&w_id)
    .fetch_optional(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM job_logs WHERE job_id = $1", id)
        .execute(&mut *tx)
        .await?;

    let job = not_found_if_none(job_o, "Completed Job", id.to_string())?;

    audit_log(
        &mut *tx,
        &authed,
        "jobs.delete",
        ActionKind::Delete,
        &w_id,
        Some(&id.to_string()),
        None,
    )
    .await?;

    tx.commit().await?;
    let cj = CompletedJob::from_row(&job)?;

    let cj = format_completed_job_result(cj);

    let response = Json(cj).into_response();
    Ok(response)
}
