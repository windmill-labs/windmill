/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

pub use windmill_api_jobs::execution::*;
pub use windmill_api_jobs::query::*;
pub use windmill_api_jobs::types::*;
pub use windmill_api_sse::*;

use axum::body::Body;
use futures::{StreamExt, TryFutureExt};
use itertools::Itertools;
use quick_cache::sync::Cache;
use serde_json::value::RawValue;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tower::ServiceBuilder;
use url::Url;
#[cfg(all(feature = "enterprise", feature = "smtp"))]
use windmill_common::auth::is_super_admin_email;
use windmill_common::auth::TOKEN_PREFIX_LEN;
#[cfg(feature = "run_inline")]
use windmill_common::client::AuthedClient;
use windmill_common::db::UserDbWithAuthed;
use windmill_common::error::JsonResult;
use windmill_common::flow_status::{JobResult, RestartedFrom};
use windmill_common::jobs::{
    format_completed_job_result, format_result, DynamicInput, ENTRYPOINT_OVERRIDE,
};
#[cfg(feature = "run_inline")]
use windmill_common::jobs::{
    InlineScriptTarget, RunInlinePreviewScriptFnParams, RunInlineScriptFnParams,
};
use windmill_common::runnable_settings::{
    ConcurrencySettings, ConcurrencySettingsWithCustom, DebouncingSettings,
};
#[cfg(feature = "run_inline")]
use windmill_common::runtime_assets::{register_runtime_asset, InsertRuntimeAssetParams};
use windmill_common::scripts::ScriptRunnableSettingsInline;
use windmill_common::triggers::TriggerMetadata;
use windmill_common::utils::{RunnableKind, WarnAfterExt};
use windmill_common::worker::{Connection, CLOUD_HOSTED, TMP_DIR};
use windmill_common::workspace_dependencies::{
    RawWorkspaceDependencies, MIN_VERSION_WORKSPACE_DEPENDENCIES,
};
use windmill_common::DYNAMIC_INPUT_CACHE;
#[cfg(all(feature = "enterprise", feature = "smtp"))]
use windmill_common::{email_oss::send_email_html, server::load_smtp_config};
use windmill_object_store::upload_artifact_to_store;
#[cfg(feature = "run_inline")]
use windmill_parser::asset_parser::AssetKind;
use windmill_types::s3::BundleFormat;
#[cfg(feature = "run_inline")]
use windmill_worker::get_worker_internal_server_inline_utils;

use windmill_common::variables::get_workspace_key;

#[cfg(feature = "run_inline")]
use crate::db::OptJobAuthed;
use crate::triggers::trigger_helpers::{FlowId, ScriptId};
use crate::{
    add_webhook_allowed_origin,
    args::{self, RawWebhookArgs},
    auth::{OptTokened, Tokened},
    concurrency_groups::join_concurrency_key,
    db::{ApiAuthed, DB},
    triggers::trigger_helpers::RunnableId,
    users::{
        get_scope_tags, require_owner_of_path, require_path_read_access_for_preview, OptAuthed,
    },
    utils::{check_scopes, content_plain, require_super_admin},
};
use anyhow::Context;
use axum::{
    extract::{Json, Path, Query},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Router,
};
use chrono::Utc;
use hmac::Mac;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use sql_builder::prelude::*;
use sqlx::types::JsonRawValue;
use sqlx::{types::Uuid, FromRow, Postgres, Transaction};
use tower_http::cors::{Any, CorsLayer};
use urlencoding::encode;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::audit::AuditAuthor;
use windmill_common::worker::to_raw_value;

use windmill_common::{
    cache,
    db::UserDB,
    error::{self, to_anyhow, Error},
    flow_status::{Approval, FlowStatus, FlowStatusModule},
    flows::{add_virtual_items_if_necessary, resolve_maybe_value, FlowValue},
    jobs::{script_path_to_payload, CompletedJob, JobKind, JobPayload, QueuedJob, RawCode},
    oauth2::HmacSha256,
    scripts::{ScriptHash, ScriptLang},
    users::username_to_permissioned_as,
    utils::{not_found_if_none, now_from_db, paginate, require_admin, Pagination, StripPath},
};

use windmill_common::{
    get_flow_version_info_from_version, get_latest_deployed_hash_for_path,
    get_latest_flow_version_info_for_path, get_script_info_for_hash, utils::empty_as_none,
    ScriptHashInfo, BASE_URL,
};
use windmill_queue::{
    get_result_and_success_by_id_from_flow, job_is_complete, push, PushArgs, PushArgsOwned,
    PushIsolationLevel,
};

pub fn workspaced_service() -> Router {
    let cors = CorsLayer::new()
        .allow_methods([http::Method::GET, http::Method::POST])
        .allow_headers([http::header::CONTENT_TYPE, http::header::AUTHORIZATION])
        .allow_origin(Any);

    // Cloud events abuse control headers
    let ce_headers =
        ServiceBuilder::new().layer(axum::middleware::from_fn(add_webhook_allowed_origin));

    Router::new()
        .route(
            "/run/f/*script_path",
            post(run_flow_by_path)
                .head(|| async { "" })
                .layer(cors.clone())
                .layer(ce_headers.clone()),
        )
        .route(
            "/run/fv/:version",
            post(run_flow_by_version)
                .head(|| async { "" })
                .layer(cors.clone())
                .layer(ce_headers.clone()),
        )
        .route(
            "/run/batch_rerun_jobs",
            post(batch_rerun_jobs)
                .head(|| async { "" })
                .layer(cors.clone())
                .layer(ce_headers.clone()),
        )
        .route(
            "/run/workflow_as_code/:job_id/:entrypoint",
            post(run_workflow_as_code)
                .head(|| async { "" })
                .layer(cors.clone())
                .layer(ce_headers.clone()),
        )
        .route(
            "/restart/f/:job_id",
            post(restart_flow).head(|| async { "" }).layer(cors.clone()),
        )
        .route(
            "/run/p/*script_path",
            post(run_script_by_path)
                .head(|| async { "" })
                .layer(cors.clone())
                .layer(ce_headers.clone()),
        )
        .route(
            "/run_wait_result/p/*script_path",
            post(run_wait_result_script_by_path)
                .get(run_wait_result_job_by_path_get)
                .head(|| async { "" })
                .layer(cors.clone())
                .layer(ce_headers.clone()),
        )
        .route(
            "/run_wait_result/h/:hash",
            post(run_wait_result_script_by_hash)
                .head(|| async { "" })
                .layer(cors.clone())
                .layer(ce_headers.clone()),
        )
        .route(
            "/run_wait_result/f/*script_path",
            post(run_wait_result_flow_by_path)
                .get(run_wait_result_flow_by_path_get)
                .head(|| async { "" })
                .layer(cors.clone())
                .layer(ce_headers.clone()),
        )
        .route(
            "/run_wait_result/fv/:version",
            post(run_wait_result_flow_by_version)
                .get(run_wait_result_flow_by_version_get)
                .head(|| async { "" })
                .layer(cors.clone())
                .layer(ce_headers.clone()),
        )
        .route(
            "/run_and_stream/f/*script_path",
            get(stream_flow_by_path)
                .post(stream_flow_by_path)
                .head(|| async { "" })
                .layer(cors.clone())
                .layer(ce_headers.clone()),
        )
        .route(
            "/run_and_stream/fv/:version",
            get(stream_flow_by_version)
                .post(stream_flow_by_version)
                .head(|| async { "" })
                .layer(cors.clone())
                .layer(ce_headers.clone()),
        )
        .route(
            "/run_and_stream/p/*script_path",
            get(stream_script_by_path)
                .post(stream_script_by_path)
                .head(|| async { "" })
                .layer(cors.clone())
                .layer(ce_headers.clone()),
        )
        .route(
            "/run_and_stream/h/:hash",
            get(stream_script_by_hash)
                .post(stream_script_by_hash)
                .head(|| async { "" })
                .layer(cors.clone())
                .layer(ce_headers.clone()),
        )
        .route(
            "/run/h/:hash",
            post(run_job_by_hash)
                .head(|| async { "" })
                .layer(cors.clone())
                .layer(ce_headers.clone()),
        )
        .route("/run/preview", post(run_preview_script))
        .route("/run_inline/preview", post(run_inline_preview_script))
        .route(
            "/run_inline/p/*script_path",
            post(run_inline_script_by_path),
        )
        .route("/run_inline/h/:hash", post(run_inline_script_by_hash))
        .route(
            "/run_wait_result/preview",
            post(run_wait_result_preview_script),
        )
        .route(
            "/run/preview_bundle",
            post(run_bundle_preview_script).layer(axum::extract::DefaultBodyLimit::disable()),
        )
        .route("/add_batch_jobs/:n", post(add_batch_jobs))
        .route("/run/preview_flow", post(run_preview_flow_job))
        .route(
            "/run_wait_result/preview_flow",
            post(run_wait_result_preview_flow),
        )
        .route("/run/dynamic_select", post(run_dynamic_select))
        .route("/list", get(list_jobs))
        .route(
            "/list_selected_job_groups",
            // We use post because sending a huge array as a query param can produce
            // URLs that may be too long
            post(list_selected_job_groups),
        )
        .route("/list_filtered_uuids", get(list_filtered_job_uuids))
        .route("/queue/list", get(list_queue_jobs))
        .route("/queue/export", get(crate::jobs_export::export_queued_jobs))
        .route(
            "/queue/import",
            post(crate::jobs_export::import_queued_jobs),
        )
        .route("/queue/count", get(count_queue_jobs))
        .route("/queue/list_filtered_uuids", get(list_filtered_uuids))
        .route("/queue/position/:timestamp", get(get_queue_position))
        .route("/queue/scheduled_for/:id", get(get_scheduled_for))
        .route("/queue/cancel_selection", post(cancel_selection))
        .route("/completed/count", get(count_completed_jobs))
        .route("/completed/count_jobs", get(count_completed_jobs_detail))
        .route(
            "/completed/list",
            get(list_completed_jobs).layer(cors.clone()),
        )
        .route(
            "/completed/export",
            get(crate::jobs_export::export_completed_jobs).layer(cors.clone()),
        )
        .route(
            "/completed/import",
            post(crate::jobs_export::import_completed_jobs).layer(cors.clone()),
        )
        .route("/delete", post(crate::jobs_export::delete_jobs))
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
            "/completed/get_timing/:id",
            get(get_completed_job_timing).layer(cors.clone()),
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
        .route(
            "/flow_env_by_flow_job_id/:flow_job_id/:var_name",
            get(get_flow_env_by_flow_job_id).layer(cors.clone()),
        )
        .route("/run/dependencies", post(run_dependencies_job))
        .route("/run/flow_dependencies", post(run_flow_dependencies_job))
        .route(
            "/send_email_with_instance_smtp",
            post(send_email_with_instance_smtp),
        )
        .route("/get_otel_traces/:id", get(get_otel_traces))
}

pub fn workspace_unauthed_service() -> Router {
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
        .route(
            "/get_completed_logs_tail/:id",
            get(get_completed_job_logs_tail),
        )
        .route("/get_args/:id", get(get_args))
        .route("/queue/get_started_at_by_ids", post(get_started_at_by_ids))
        .route("/get_flow_debug_info/:id", get(get_flow_job_debug_info))
        .route("/completed/get/:id", get(get_completed_job))
        .route("/completed/get_result/:id", get(get_completed_job_result))
        .route(
            "/completed/get_result_maybe/:id",
            get(get_completed_job_result_maybe),
        )
        .route("/completed/get_timing/:id", get(get_completed_job_timing))
        .route("/getupdate/:id", get(get_job_update))
        .route("/getupdate_sse/:id", get(get_job_update_sse))
        .route("/get_log_file/*file_path", get(get_log_file))
        .route("/queue/cancel/:id", post(cancel_job_api))
        .route(
            "/queue/cancel_persistent/*script_path",
            post(cancel_persistent_script_api),
        )
        .route("/queue/force_cancel/:id", post(force_cancel))
}

pub fn global_root_service() -> Router {
    Router::new()
        .route("/db_clock", get(get_db_clock))
        .route("/completed/count_by_tag", get(count_by_tag))
}

#[derive(Deserialize)]
struct JsonPath {
    pub json_path: Option<String>,
    pub suspended_job: Option<Uuid>,
    pub resume_id: Option<u32>,
    pub secret: Option<String>,
    pub approver: Option<String>,
}
async fn get_result_by_id(
    authed: ApiAuthed,
    tokened: Tokened,
    Extension(db): Extension<DB>,
    Path((w_id, flow_id, node_id)): Path<(String, Uuid, String)>,
    Query(JsonPath { json_path, .. }): Query<JsonPath>,
) -> windmill_common::error::JsonResult<Box<JsonRawValue>> {
    let res =
        windmill_queue::get_result_by_id(db.clone(), w_id.clone(), flow_id, node_id, json_path)
            .await?;

    log_job_view(&db, Some(&authed), Some(&tokened.token), &w_id, &flow_id).await?;

    Ok(Json(res))
}

async fn get_root_job(
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> windmill_common::error::JsonResult<String> {
    let res = compute_root_job_for_flow(&db, &w_id, id).await?;
    Ok(Json(res))
}

async fn get_flow_env_by_flow_job_id(
    authed: ApiAuthed,
    tokened: Tokened,
    Extension(db): Extension<DB>,
    Path((w_id, flow_job_id, var_name)): Path<(String, Uuid, String)>,
    Query(JsonPath { json_path, .. }): Query<JsonPath>,
) -> windmill_common::error::JsonResult<Box<JsonRawValue>> {
    let flow_env = sqlx::query_scalar!(
            r#"
                SELECT
                    CASE
                        WHEN flow_version.id IS NOT NULL THEN
                            (flow_version.value -> 'flow_env' -> $3) #> $4
                        ELSE
                            (root_job.raw_flow -> 'flow_env' -> $3) #> $4
                    END AS "flow_env: sqlx::types::Json<Box<RawValue>>"
                FROM
                    v2_job current_job
                JOIN
                    v2_job root_job ON root_job.id = COALESCE(current_job.root_job, current_job.flow_innermost_root_job, current_job.parent_job, current_job.id)
                    AND root_job.workspace_id = current_job.workspace_id
                LEFT JOIN
                    flow_version ON flow_version.id = root_job.runnable_id
                    AND flow_version.path = root_job.runnable_path
                    AND flow_version.workspace_id = root_job.workspace_id
            WHERE
                    current_job.id = $1 AND
                    current_job.workspace_id = $2"#,
            flow_job_id,
            w_id,
            var_name,
            json_path
                .as_ref()
                .map(|x| x.split(".").collect::<Vec<_>>())
                .unwrap_or_default() as Vec<&str>,
        )
        .fetch_optional(&db)
        .await?
        .map(|r| r.map(|x| x.0))
        .flatten()
        .unwrap_or_else(|| to_raw_value(&serde_json::Value::Null));

    log_job_view(
        &db,
        Some(&authed),
        Some(&tokened.token),
        &w_id,
        &flow_job_id,
    )
    .await?;
    Ok(Json(flow_env))
}

async fn compute_root_job_for_flow(db: &DB, w_id: &str, job_id: Uuid) -> error::Result<String> {
    let root_job = sqlx::query_scalar!(
            r#"SELECT COALESCE(root_job, flow_innermost_root_job, parent_job, id) as "root_job!" FROM v2_job WHERE id = $1 AND workspace_id = $2"#,
            job_id,
            w_id
        )
        .fetch_one(db)
        .await?;

    Ok(root_job.to_string())
}

async fn get_db_clock(Extension(db): Extension<DB>) -> windmill_common::error::JsonResult<i64> {
    Ok(Json(now_from_db(&db).await?.timestamp_millis()))
}

async fn cancel_job_api(
    OptAuthed(opt_authed): OptAuthed,
    opt_tokened: OptTokened,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Json(CancelJob { reason }): Json<CancelJob>,
) -> error::Result<String> {
    let tx = db.begin().await?;

    let audit_author: AuditAuthor = match opt_authed.as_ref() {
        Some(authed) => (authed).into(),
        None => AuditAuthor {
            username: "anonymous".to_string(),
            username_override: None,
            email: "anonymous".to_string(),
            token_prefix: opt_tokened
                .token
                .map(|s| s[0..TOKEN_PREFIX_LEN].to_string()),
        },
    };
    let (mut tx, job_option) = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        windmill_queue::cancel_job(
            &audit_author.username,
            reason,
            id,
            &w_id,
            tx,
            &db,
            false,
            opt_authed.is_none(),
        ),
    )
    .await
    .map_err(|e| {
        Error::internal_err(format!(
            "timeout after 120s while cancelling job {id} in {w_id}: {e:#}"
        ))
    })??;

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
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    Json(CancelJob { reason }): Json<CancelJob>,
) -> error::Result<()> {
    let audit_author: AuditAuthor = match opt_authed {
        Some(authed) => (&authed).into(),
        None => {
            return Err(Error::BadRequest(format!(
                "Cancelling persistent script require to be logged in and member of {w_id}"
            )))
        }
    };

    let cancelled_job_ids = windmill_queue::cancel_persistent_script_jobs(
        &audit_author.username,
        reason,
        script_path.to_path(),
        &w_id,
        &db,
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
    OptAuthed(opt_authed): OptAuthed,
    tokened: OptTokened,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Json(CancelJob { reason }): Json<CancelJob>,
) -> error::Result<String> {
    let tx = db.begin().await?;

    let audit_author: AuditAuthor = match opt_authed.as_ref() {
        Some(authed) => (authed).into(),
        None => AuditAuthor {
            username: "anonymous".to_string(),
            username_override: None,
            email: "anonymous".to_string(),
            token_prefix: tokened.token.map(|t| t[0..TOKEN_PREFIX_LEN].to_string()),
        },
    };

    let (mut tx, job_option) = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        windmill_queue::cancel_job(
            &audit_author.username,
            reason,
            id,
            &w_id,
            tx,
            &db,
            true,
            opt_authed.is_none(),
        ),
    )
    .await
    .map_err(|e| {
        Error::internal_err(format!(
            "timeout after 120s while cancelling job {id} in {w_id}: {e:#}"
        ))
    })??;

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

#[derive(Serialize)]
struct QueuePosition {
    position: Option<i64>,
}

async fn get_queue_position(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((_w_id, scheduled_for)): Path<(String, i64)>,
) -> error::Result<Json<QueuePosition>> {
    // First check if the job exists and is in queue

    // Count jobs that are scheduled before this job and are not suspended
    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) as count
                FROM v2_job_queue
                WHERE scheduled_for < to_timestamp($1::bigint / 1000.0)
                AND running = false
                AND suspend_until IS NULL",
        scheduled_for,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);

    Ok(Json(QueuePosition { position: Some(count + 1) }))
}

async fn get_scheduled_for(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::Result<Json<i64>> {
    let scheduled_for = sqlx::query_scalar!(
        "SELECT scheduled_for FROM v2_job_queue WHERE id = $1 AND workspace_id = $2",
        id,
        w_id,
    )
    .fetch_optional(&db)
    .await?;

    let scheduled_for = not_found_if_none(scheduled_for, "QueuedJob", &id.to_string())?;
    Ok(Json(scheduled_for.timestamp_millis()))
}

async fn get_flow_job_debug_info(
    OptAuthed(opt_authed): OptAuthed,
    tokened_o: OptTokened,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::Result<Response> {
    let job = GetQuery::new()
        .fetch_queued((&db).into(), &id, &w_id)
        .await?;
    if let Some(job) = job {
        let is_flow = job.is_flow();
        if job.is_flow_step || !is_flow {
            return Err(error::Error::BadRequest(
                "This endpoint is only for root flow jobs".to_string(),
            ));
        }
        let leaf_jobs: HashMap<String, JobResult> = job
            .leaf_jobs
            .clone()
            .and_then(|x| serde_json::from_value(x).ok())
            .unwrap_or_else(HashMap::new);
        let mut jobs = HashMap::new();
        let id = job.id.clone();
        jobs.insert("root_job".to_string(), Job::QueuedJob(job));

        let mut job_ids = vec![];
        let jobs_with_root = sqlx::query_scalar!(
            "SELECT id FROM v2_job WHERE workspace_id = $1 and flow_innermost_root_job = $2",
            &w_id,
            &id,
        )
        .fetch_all(&db)
        .await?;

        for job in jobs_with_root {
            job_ids.push(job);
        }

        for job in leaf_jobs.iter() {
            match job.1 {
                JobResult::ListJob(jobs) => job_ids.extend(jobs.to_owned()),
                JobResult::SingleJob(job) => job_ids.push(job.clone()),
            }
        }
        for job_id in job_ids {
            let job = GetQuery::new()
                .with_auth(&opt_authed)
                .fetch(&db, &job_id, &w_id)
                .await;
            if let Ok(job) = job {
                jobs.insert(job.id().to_string(), job);
            }
        }

        log_job_view(
            &db,
            opt_authed.as_ref(),
            tokened_o.token.as_deref(),
            &w_id,
            &id,
        )
        .await?;

        Ok(Json(jobs).into_response())
    } else {
        Err(error::Error::NotFound(format!(
            "QueuedJob {} not found",
            id
        )))
    }
}

async fn list_selected_job_groups(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(uuids): Json<Vec<Uuid>>,
) -> error::Result<Response> {
    let mut tx = user_db.begin(&authed).await?;

    let results = sqlx::query_scalar!(
            r#"SELECT jsonb_build_object(
                'kind', jb.kind,
                'script_path', jb.runnable_path,
                'latest_schema', COALESCE(
                    (SELECT DISTINCT ON (s.path) s.schema FROM script s WHERE s.workspace_id = $1 AND s.path = jb.runnable_path AND jb.kind = 'script' ORDER BY s.path, s.created_at DESC),
                    (SELECT flow_version.schema FROM flow LEFT JOIN flow_version ON flow_version.id = flow.versions[array_upper(flow.versions, 1)] WHERE flow.workspace_id = $1 AND flow.path = jb.runnable_path AND jb.kind = 'flow')
                ),
                'schemas', ARRAY(
                    SELECT jsonb_build_object(
                        'script_hash', LPAD(TO_HEX(COALESCE(s.hash, f.id)), 16, '0'),
                        'job_ids', ARRAY_AGG(DISTINCT j.id),
                        'schema', (ARRAY_AGG(COALESCE(s.schema, f.schema)))[1]
                    ) FROM v2_job j
                    LEFT JOIN script s ON s.hash = j.runnable_id AND j.kind = 'script'
                    LEFT JOIN flow_version f ON f.id = j.runnable_id AND j.kind = 'flow'
                    WHERE j.id = ANY(ARRAY_AGG(jb.id))
                    GROUP BY COALESCE(s.hash, f.id)
                )
            ) FROM v2_job jb
            WHERE (jb.kind = 'flow' OR jb.kind = 'script')
                AND jb.workspace_id = $1 AND jb.id = ANY($2)
            GROUP BY jb.kind, jb.runnable_path"#,
            &w_id,
            &uuids
        )
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(results).into_response())
}

#[derive(Deserialize)]
struct GetJobQuery {
    pub no_logs: Option<bool>,
    pub no_code: Option<bool>,
}

async fn get_job(
    OptAuthed(opt_authed): OptAuthed,
    opt_tokened: OptTokened,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Query(GetJobQuery { no_logs, no_code }): Query<GetJobQuery>,
) -> error::Result<Response> {
    let tags = opt_authed
        .as_ref()
        .map(|authed| get_scope_tags(authed))
        .flatten();

    let mut get = GetQuery::new()
        .with_auth(&opt_authed)
        .with_in_tags(tags.as_ref());

    if no_code.unwrap_or(false) {
        get = get.without_code();
    }

    if no_logs.unwrap_or(false) {
        get = get.without_logs();
    }
    let mut job = get.fetch(&db, &id, &w_id).await?;
    job.fetch_outstanding_wait_time(&db).await?;

    log_job_view(
        &db,
        opt_authed.as_ref(),
        opt_tokened.token.as_deref(),
        &w_id,
        &id,
    )
    .await?;

    Ok(Json(job).into_response())
}

macro_rules! get_job_query {
    ("v2_job_completed", $($opts:tt)*) => {
        get_job_query!(
            @impl "v2_job_completed", ($($opts)*),
            "v2_job_completed.duration_ms, v2_job_completed.completed_at, CASE WHEN status = 'success' OR status = 'skipped' THEN true ELSE false END as success, result_columns, deleted, status = 'skipped' as is_skipped, result->'wm_labels' as labels, \
            CASE WHEN result is null or pg_column_size(result) < 90000 THEN result ELSE '\"WINDMILL_TOO_BIG\"'::jsonb END as result",
            "",
        )
    };
    ("v2_job_queue", $($opts:tt)*) => {
        get_job_query!(
            @impl "v2_job_queue", ($($opts)*),
            "scheduled_for, running, ping as last_ping, suspend, suspend_until, same_worker, pre_run_error, visible_to_owner, \
            flow_innermost_root_job  AS root_job, flow_leaf_jobs AS leaf_jobs, concurrent_limit, concurrency_time_window_s, timeout, flow_step_id, cache_ttl, cache_ignore_s3_path, runnable_settings_handle, \
            script_entrypoint_override",
            "LEFT JOIN v2_job_runtime ON v2_job_runtime.id = v2_job_queue.id LEFT JOIN v2_job_status ON v2_job_status.id = v2_job_queue.id",
        )
    };
    (@impl $table:literal, (with_logs: $with_logs:expr, $($rest:tt)*), $additional_fields:literal, $additional_joins:literal, $($args:tt)*) => {
        if $with_logs {
            get_job_query!(@impl $table, ($($rest)*), $additional_fields, $additional_joins, logs = "right(job_logs.logs, 20000)", $($args)*)
        } else {
            get_job_query!(@impl $table, ($($rest)*), $additional_fields, $additional_joins, logs = "null", $($args)*)
        }
    };
    (@impl $table:literal, (with_code: $with_code:expr, $($rest:tt)*), $additional_fields:literal, $additional_joins:literal, $($args:tt)*) => {
        if $with_code {
            get_job_query!(@impl $table, ($($rest)*), $additional_fields, $additional_joins, lock = "raw_lock", code = "raw_code", $($args)*)
        } else {
            get_job_query!(@impl $table, ($($rest)*), $additional_fields, $additional_joins, lock = "null", code = "null", $($args)*)
        }
    };
    (@impl $table:literal, (with_flow: $with_flow:expr, $($rest:tt)*), $additional_fields:literal, $additional_joins:literal, $($args:tt)*) => {
        if $with_flow {
            get_job_query!(@impl $table, ($($rest)*), $additional_fields, $additional_joins, flow = "raw_flow", $($args)*)
        } else {
            get_job_query!(@impl $table, ($($rest)*), $additional_fields, $additional_joins, flow = "null", $($args)*)
        }
    };
    (@impl $table:literal, (), $additional_fields:literal, $additional_joins:literal, $($args:tt)*) => {
        const_format::formatcp!(
            "SELECT \
            {table}.id, {table}.workspace_id, parent_job, v2_job.created_by, v2_job.created_at, started_at, v2_job.runnable_id as script_hash, v2_job.runnable_path as script_path, \
            CASE WHEN args is null THEN NULL
            WHEN pg_column_size(args) < 90000 THEN
                CASE WHEN jsonb_typeof(args) = 'object' THEN args
                ELSE jsonb_build_object('value', args)
                END
            ELSE '{{\"reason\": \"WINDMILL_TOO_BIG\"}}'::jsonb END as args, flow_status, workflow_as_code_status, \
            {logs} as logs, {code} as raw_code, canceled_by is not null as canceled, canceled_by, canceled_reason, kind as job_kind, \
            CASE WHEN trigger_kind = 'schedule'::job_trigger_kind THEN trigger END AS schedule_path, permissioned_as, \
            {flow} as raw_flow, flow_step_id IS NOT NULL AS is_flow_step, script_lang as language, \
            {lock} as raw_lock, permissioned_as_email as email, visible_to_owner, memory_peak as mem_peak, v2_job.tag, v2_job.priority, preprocessed, worker,\
            {additional_fields} \
            FROM {table}
            INNER JOIN v2_job ON v2_job.id = {table}.id \
            {additional_joins} \
            LEFT JOIN job_logs ON {table}.id = job_id \
            WHERE {table}.id = $1 AND {table}.workspace_id = $2 AND ($3::text[] IS NULL OR v2_job.tag = ANY($3))",
            table = $table,
            additional_fields = $additional_fields,
            additional_joins = $additional_joins,
            $($args)*
        )
    }
}

#[derive(Copy, Clone)]
struct GetQuery<'a> {
    with_logs: bool,
    with_code: bool,
    with_flow: bool,
    with_auth: Option<&'a Option<ApiAuthed>>,
    with_in_tags: Option<&'a Vec<&'a str>>,
}

impl<'a> GetQuery<'a> {
    fn new() -> Self {
        Self {
            with_logs: true,
            with_code: true,
            with_flow: true,
            with_auth: None,
            with_in_tags: None,
        }
    }

    fn without_logs(self) -> Self {
        Self { with_logs: false, ..self }
    }

    fn without_code(self) -> Self {
        Self { with_code: false, ..self }
    }

    fn without_flow(self) -> Self {
        Self { with_flow: false, ..self }
    }

    fn with_auth(self, auth: &'a Option<ApiAuthed>) -> Self {
        Self { with_auth: Some(auth), ..self }
    }

    fn with_in_tags(self, in_tags: Option<&'a Vec<&str>>) -> Self {
        Self { with_in_tags: in_tags, ..self }
    }

    fn check_auth(&self, email: Option<&str>) -> error::Result<()> {
        if let Some(email) = email {
            if self.with_auth.is_some_and(|x| x.is_none()) && email != "anonymous" {
                return Err(Error::BadRequest(
                    "As a non logged in user, you can only see jobs ran by anonymous users"
                        .to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Resolve job raw values.
    /// This fetch the raw values from the cache and update the job accordingly.
    ///
    /// # Details
    /// Most of the raw values (code, lock and flow) had been removed from the `job`, `queue` and
    /// `completed_job` tables. Only remains ones for "preview" jobs (i.e. [`JobKind::Preview`],
    /// [`JobKind::FlowPreview`] and [`JobKind::Dependencies`]). [`JobKind::Flow`] as well but only
    /// when pushed from an un-updated workers.
    /// This function is used to make the above change transparent for the API, as the returned jobs
    /// will have the raw values as if they were still in the tables.
    async fn resolve_raw_values<T: JobCommon>(
        &self,
        db: &DB,
        id: Uuid,
        hash: Option<ScriptHash>,
        job: &mut JobExtended<T>,
    ) {
        let (raw_code, raw_lock, raw_flow) = (
            job.raw_code.take(),
            job.raw_lock.take(),
            job.raw_flow.take(),
        );
        if self.with_flow {
            // Try to fetch the flow from the cache, fallback to the preview flow.
            // NOTE: This could check for the job kinds instead of the `or_else` but it's not
            // necessary as `fetch_flow` return early if the job kind is not a preview one.
            cache::job::fetch_flow(db, job.job_kind(), hash)
                .or_else(|_| cache::job::fetch_preview_flow(db, &id, raw_flow))
                .await
                .ok()
                .inspect(|data| job.raw_flow = Some(sqlx::types::Json(data.raw_flow.clone())));
        }
        if self.with_code && job.job_kind() == &JobKind::Preview {
            // Try to fetch the code from the cache, fallback to the preview code.
            // NOTE: This could check for the job kinds instead of the `or_else` but it's not
            // necessary as `fetch_script` return early if the job kind is not a preview one.
            let conn = Connection::from(db.clone());
            cache::job::fetch_script(db.clone(), job.job_kind(), hash)
                .or_else(|_| cache::job::fetch_preview_script(&conn, &id, raw_lock, raw_code))
                .await
                .ok()
                .inspect(|data| {
                    (job.raw_lock, job.raw_code) = (data.lock.clone(), Some(data.code.clone()))
                });
        }
    }

    async fn fetch_queued(
        self,
        db: &DB,
        job_id: &Uuid,
        workspace_id: &str,
    ) -> error::Result<Option<JobExtended<QueuedJob>>> {
        let query = get_job_query!("v2_job_queue",
            with_logs: self.with_logs,
            with_code: self.with_code,
            with_flow: self.with_flow,
        );
        let query = sqlx::query_as::<_, JobExtended<QueuedJob>>(query)
            .bind(job_id)
            .bind(workspace_id)
            .bind(self.with_in_tags);

        let mut job = query.fetch_optional(db).await?;

        self.check_auth(job.as_ref().map(|job| job.created_by.as_str()))?;
        if let Some(job) = job.as_mut() {
            self.resolve_raw_values(&db, job.id, job.script_hash, job)
                .await;
        }
        if self.with_flow {
            job = resolve_maybe_value(db, workspace_id, self.with_code, job, |job| {
                job.raw_flow.as_mut()
            })
            .await?;
        }
        Ok(job)
    }

    async fn fetch_completed(
        self,
        db: &DB,
        job_id: &Uuid,
        workspace_id: &str,
    ) -> error::Result<Option<JobExtended<CompletedJob>>> {
        let query = get_job_query!("v2_job_completed",
            with_logs: self.with_logs,
            with_code: self.with_code,
            with_flow: self.with_flow,
        );

        // tracing::info!("query: {}", query);
        let query = sqlx::query_as::<_, JobExtended<CompletedJob>>(query)
            .bind(job_id)
            .bind(workspace_id)
            .bind(self.with_in_tags);

        let mut cjob = query.fetch_optional(db).await?;

        self.check_auth(cjob.as_ref().map(|job| job.created_by.as_str()))?;
        if let Some(job) = cjob.as_mut() {
            self.resolve_raw_values(db, job.id, job.script_hash, job)
                .await;
        }

        if self.with_flow {
            cjob = resolve_maybe_value(db, workspace_id, self.with_code, cjob, |job| {
                job.raw_flow.as_mut()
            })
            .await?;
        }

        if let Some(mut cjob) = cjob {
            cjob.inner = format_completed_job_result(cjob.inner);
            return Ok(Some(cjob));
        }
        Ok(cjob)
    }

    async fn fetch(self, db: &DB, job_id: &Uuid, workspace_id: &str) -> error::Result<Job> {
        let cjob = self
            .fetch_completed(db.into(), job_id, workspace_id)
            .await?
            .map(Job::CompletedJob);

        match cjob {
            Some(cjob) => Ok(cjob),
            None => {
                let job_maybe = self
                    .fetch_queued(db.into(), job_id, workspace_id)
                    .await?
                    .map(Job::QueuedJob);
                // potential race condition here, if the job was in queue and completed right after the fetch completed, so we need to check one last time
                if let Some(job) = job_maybe {
                    return Ok(job);
                } else {
                    let cjob2 = self
                        .fetch_completed(db.into(), job_id, workspace_id)
                        .await?
                        .map(Job::CompletedJob);
                    not_found_if_none(cjob2, "Job", job_id.to_string())
                }
            }
        }
    }
}

#[cfg(all(feature = "smtp", feature = "enterprise"))]
async fn send_workspace_trigger_failure_email_notification(
    db: &DB,
    w_id: &str,
    job_id: &Uuid,
    trigger_path: Option<&str>,
    runnable_path: Option<&str>,
    email_recipients: &Vec<String>,
    error: &Value,
) -> Result<String, Error> {
    let smtp_config = match load_smtp_config(db).await? {
        Some(config) => config,
        None => {
            tracing::info!(
                "SMTP not configured, skipping workspace trigger failure email notification"
            );
            return Err(anyhow::anyhow!(
                "SMTP not configured, skipping workspace trigger failure email notification"
            )
            .into());
        }
    };

    let runnable_path = runnable_path.as_deref().unwrap_or("Unknown");

    let (trigger_kind, trigger_path) = if let Some(trigger_path) = trigger_path.as_deref() {
        match trigger_path.split_once('/') {
            Some((trigger_kind, trigger_path)) => {
                tracing::debug!(
                    "Workspace trigger job {} is a {:?} trigger",
                    &job_id,
                    trigger_kind
                );
                (Some(trigger_kind), Some(trigger_path))
            }
            _ => (None, None),
        }
    } else {
        (None, None)
    };

    let base_url = BASE_URL.read().await;
    let job_url = format!("{}/run/{}?workspace={}", base_url, &job_id, w_id);
    let trigger_kind_str = trigger_kind.unwrap_or("Unknown").to_string().to_uppercase();

    let subject = format!(
        "Windmill Job Failed: {} in workspace {}",
        runnable_path, w_id
    );

    let error_details = serde_json::to_string_pretty(&error)
        .unwrap_or_else(|_| format!("Unable to serialize error: {:?}", error));

    let trigger_info = if trigger_kind.is_some() && trigger_path.is_some() {
        format!(
            r#"
        <div class="section">
        <span class="label">Trigger path:</span> {}
        </div>

        <div class="section">
        <span class="label">Trigger Type:</span> {}
        </div>"#,
            trigger_path.unwrap(),
            trigger_kind_str
        )
    } else {
        String::new()
    };

    let email_title = if trigger_kind.is_some() {
        format!("Windmill Trigger Job {} Failed", &job_id)
    } else {
        format!("Windmill Job {} Failed", &job_id)
    };

    let content = format!(
        r#"
    <!DOCTYPE html>
    <html>
    <head>
        <meta charset="UTF-8">
        <style>
        body {{ font-family: Arial, sans-serif; color: #333; }}
        h1 {{ color: #c0392b; }}
        .section {{ margin-bottom: 20px; }}
        .label {{ font-weight: bold; }}
        pre {{ background-color: #f4f4f4; padding: 10px; border-radius: 4px; overflow-x: auto; }}
        a.button {{
            display: inline-block;
            padding: 10px 15px;
            margin-top: 10px;
            color: #fff;
            background-color: #2980b9;
            text-decoration: none;
            border-radius: 5px;
        }}
        </style>
    </head>
    <body>
        <h1>{}</h1>

        <div class="section">
        <span class="label">Workspace:</span> {}
        </div>
    {}
        <div class="section">
        <span class="label">Script/Flow Path:</span> {}
        </div>

        <div class="section">
        <span class="label">Job ID:</span> {}
        </div>

        <div class="section">
        <span class="label">Error Details:</span>
        <pre>{}</pre>
        </div>

        <a href="{}" class="button">View Job Details</a>
    </body>
    </html>
    "#,
        email_title, w_id, trigger_info, runnable_path, &job_id, error_details, job_url
    );

    if let Err(e) = send_email_html(
        &subject,
        &content,
        email_recipients.to_owned(),
        smtp_config,
        None,
    )
    .await
    {
        let err_msg = format!(
                "Failed to send workspace email notification for trigger failure (trigger kind: {}, job ID: {}): {}",
                trigger_kind_str,
                &job_id,
                e
            );
        return Err(Error::internal_err(err_msg));
    }
    let success_msg = format!(
        "Job ID '{}' failed. An email with error details has been sent to {:?}.",
        &job_id, email_recipients
    );
    tracing::info!("{}", &success_msg);
    Ok(success_msg)
}

#[derive(Debug, Serialize, Deserialize)]
struct SendEmail {
    job_id: Uuid,
    #[serde(default, deserialize_with = "empty_as_none")]
    trigger_path: Option<String>,
    #[serde(default, deserialize_with = "empty_as_none")]
    runnable_path: Option<String>,
    #[serde(default, deserialize_with = "empty_as_none")]
    email_recipients: Option<Vec<String>>,
    error: Value,
}

#[cfg(all(feature = "enterprise", feature = "smtp"))]
async fn send_email_with_instance_smtp(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(send_email): Json<SendEmail>,
) -> error::Result<Json<String>> {
    use windmill_common::jobs::EMAIL_ERROR_HANDLER_USER_EMAIL;

    if *CLOUD_HOSTED {
        tracing::warn!(
                "Workspace trigger failure email notification is not available for cloud hosted Windmill",
            );
        return Err(anyhow::anyhow!("Feature not supported in cloud hosted windmill").into());
    }

    if send_email.email_recipients.is_none() {
        use windmill_common::utils::report_critical_error;

        tracing::error!("No recipient to send the error");
        report_critical_error(
            "No recipient to send the error".to_string(),
            db.clone(),
            Some(&w_id),
            None,
        )
        .await;
        return Err(anyhow::anyhow!("No recipient to send the error").into());
    }

    if authed.email == EMAIL_ERROR_HANDLER_USER_EMAIL
        || is_super_admin_email(&db, &authed.email).await?
    {
        let resp = send_workspace_trigger_failure_email_notification(
            &db,
            &w_id,
            &send_email.job_id,
            send_email.trigger_path.as_deref(),
            send_email.runnable_path.as_deref(),
            &send_email.email_recipients.unwrap(),
            &send_email.error,
        )
        .await?;

        return Ok(Json(resp));
    }

    return Err(Error::NotAuthorized(
        "Only super admin or whitelisted token can access email workspace error handler feature"
            .to_string(),
    ));
}

#[cfg(not(all(feature = "enterprise", feature = "smtp")))]
async fn send_email_with_instance_smtp(
    _authed: ApiAuthed,
    Extension(_db): Extension<DB>,
    Path(_w_id): Path<String>,
    Json(_send_email): Json<SendEmail>,
) -> error::Result<Json<String>> {
    tracing::warn!("SMTP is not enabled, skipping workspace trigger failure email notification",);

    return Err(anyhow::anyhow!("SMTP is not enabled").into());
}

#[cfg(all(feature = "enterprise", feature = "parquet"))]
async fn get_logs_from_store(
    log_offset: i32,
    logs: &str,
    log_file_index: &Option<Vec<String>>,
) -> Option<error::Result<Body>> {
    use futures::StreamExt;
    let stream =
        windmill_object_store::get_logs_from_store(log_offset, logs, log_file_index).await?;
    let header = bytes::Bytes::from(
        r#"to remove ansi colors, use: | sed 's/\x1B\[[0-9;]\{1,\}[A-Za-z]//g'
"#
        .to_string(),
    );
    let prefixed_stream = futures::stream::once(async { Ok(header) }).chain(stream);
    Some(Ok(Body::from_stream(prefixed_stream)))
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
                yield Ok(bytes::Bytes::from(
                    r#"to remove ansi colors, use: | sed 's/\x1B\[[0-9;]\{1,\}[A-Za-z]//g'
            "#.to_string(),
                ));
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

async fn get_completed_job_logs_tail(
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::JsonResult<String> {
    let tags = opt_authed
        .as_ref()
        .map(|authed| get_scope_tags(authed).map(|v| v.iter().map(|s| s.to_string()).collect_vec()))
        .flatten();

    let record = sqlx::query!(
            "SELECT created_by AS \"created_by!\", coalesce(job_logs.logs, '') as logs
            FROM v2_job
            LEFT JOIN job_logs ON job_logs.job_id = v2_job.id
            WHERE v2_job.id = $1 AND v2_job.workspace_id = $2 AND ($3::text[] IS NULL OR v2_job.tag = ANY($3))
            ORDER BY job_logs.log_offset DESC
            LIMIT 100",
            id,
            w_id,
            tags.as_ref().map(|v| v.as_slice())
        )
        .fetch_optional(&db)
        .await?;

    if let Some(record) = record {
        if opt_authed.is_none() && record.created_by != "anonymous" {
            return Err(Error::BadRequest(
                "As a non logged in user, you can only see jobs ran by anonymous users".to_string(),
            ));
        }

        let logs = record.logs.unwrap_or_default();
        Ok(Json(logs))
    } else {
        Err(Error::NotFound("Job not found".to_string()).into())
    }
}
#[derive(Debug, Deserialize)]
struct QueryJobLogs {
    remove_ansi_warnings: Option<bool>,
}

async fn get_job_logs(
    OptAuthed(opt_authed): OptAuthed,
    opt_tokened: OptTokened,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Query(query_job_logs): Query<QueryJobLogs>,
) -> error::Result<Response> {
    // let audit_author: AuditAuthor = match opt_authed {
    //     Some(authed) => (&authed).into(),
    //     None => {
    //         return Err(Error::BadRequest(format!(
    //             "Cancelling script require to be logged in and member of {w_id}"
    //         )))
    //     }
    // };

    let tags = opt_authed
        .as_ref()
        .map(|authed| get_scope_tags(authed).map(|v| v.iter().map(|s| s.to_string()).collect_vec()))
        .flatten();

    let record = sqlx::query!(
            "SELECT j.created_by AS \"created_by\", coalesce(job_logs.logs, '') as logs, COALESCE(job_logs.log_offset, 0) as log_offset, job_logs.log_file_index
            FROM v2_job j
            LEFT JOIN job_logs ON job_logs.job_id = j.id
            WHERE j.id = $1 AND j.workspace_id = $2 AND ($3::text[] IS NULL OR j.tag = ANY($3))",
            id,
            w_id,
            tags.as_ref().map(|v| v.as_slice())
        )
        .fetch_optional(&db)
        .await?;

    if let Some(record) = record {
        if opt_authed.is_none() && record.created_by != "anonymous" {
            return Err(Error::BadRequest(
                "As a non logged in user, you can only see jobs ran by anonymous users".to_string(),
            ));
        }
        let logs = record.logs.unwrap_or_default();

        log_job_view(
            &db,
            opt_authed.as_ref(),
            opt_tokened.token.as_deref(),
            &w_id,
            &id,
        )
        .await?;

        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if let Some(r) = get_logs_from_store(
            record.log_offset.unwrap_or(0),
            &logs,
            &record.log_file_index,
        )
        .await
        {
            return r.map(content_plain);
        }
        if let Some(r) = get_logs_from_disk(
            record.log_offset.unwrap_or(0),
            &logs,
            &record.log_file_index,
        )
        .await
        {
            return r.map(content_plain);
        }
        let logs = if query_job_logs.remove_ansi_warnings.unwrap_or(false) {
            logs
        } else {
            format!(
                "to remove ansi colors, use: | sed 's/\\x1B\\[[0-9;]\\{{1,\\}}[A-Za-z]//g'\n{}",
                logs
            )
        };
        Ok(content_plain(Body::from(logs)))
    } else {
        let text = sqlx::query!(
                "SELECT j.created_by AS \"created_by!\", CONCAT(coalesce(job_logs.logs, '')) as logs, coalesce(job_logs.log_offset, 0) as log_offset, job_logs.log_file_index
                FROM v2_job j
                LEFT JOIN job_logs ON job_logs.job_id = j.id
                WHERE j.id = $1 AND j.workspace_id = $2 AND ($3::text[] IS NULL OR j.tag = ANY($3))",
                id,
                w_id,
                tags.as_ref().map(|v| v.as_slice())
            )
            .fetch_optional(&db)
            .await?;
        let text = not_found_if_none(text, "Job Logs", id.to_string())?;

        if opt_authed.is_none() && text.created_by != "anonymous" {
            return Err(Error::BadRequest(
                "As a non logged in user, you can only see jobs ran by anonymous users".to_string(),
            ));
        }
        let logs = text.logs.unwrap_or_default();

        log_job_view(
            &db,
            opt_authed.as_ref(),
            opt_tokened.token.as_deref(),
            &w_id,
            &id,
        )
        .await?;

        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if let Some(r) =
            get_logs_from_store(text.log_offset.unwrap_or(0), &logs, &text.log_file_index).await
        {
            return r.map(content_plain);
        }
        if let Some(r) =
            get_logs_from_disk(text.log_offset.unwrap_or(0), &logs, &text.log_file_index).await
        {
            return r.map(content_plain);
        }

        let logs = format!(
            "to remove ansi colors, use: | sed 's/\\x1B\\[[0-9;]\\{{1,\\}}[A-Za-z]//g'\n{}",
            logs
        );
        Ok(content_plain(Body::from(logs)))
    }
}

async fn get_args(
    OptAuthed(opt_authed): OptAuthed,
    opt_tokened: OptTokened,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> JsonResult<Box<RawValue>> {
    let tags = opt_authed
        .as_ref()
        .map(|authed| get_scope_tags(authed))
        .flatten();
    let record = sqlx::query!(
            "SELECT j.created_by AS \"created_by!\", j.args as \"args: sqlx::types::Json<Box<RawValue>>\"
            FROM v2_job j
            WHERE j.id = $1 AND j.workspace_id = $2 AND ($3::text[] IS NULL OR j.tag = ANY($3))",
            id,
            &w_id,
            tags.as_ref().map(|v| v.as_slice()) as Option<&[&str]>,
        )
        .fetch_optional(&db)
        .await?;

    if let Some(record) = record {
        if opt_authed.is_none() && record.created_by != "anonymous" {
            return Err(Error::BadRequest(
                "As a non logged in user, you can only see jobs ran by anonymous users".to_string(),
            ));
        }

        log_job_view(
            &db,
            opt_authed.as_ref(),
            opt_tokened.token.as_deref(),
            &w_id,
            &id,
        )
        .await?;

        Ok(Json(record.args.map(|x| x.0).unwrap_or_default()))
    } else {
        let record = sqlx::query!(
                "SELECT created_by AS \"created_by!\", args as \"args: sqlx::types::Json<Box<RawValue>>\"
                FROM v2_job
                WHERE id = $1 AND workspace_id = $2 AND ($3::text[] IS NULL OR tag = ANY($3))",
                id,
                &w_id,
                tags.as_ref().map(|v| v.as_slice()) as Option<&[&str]>,
            )
            .fetch_optional(&db)
            .await?;
        let record = not_found_if_none(record, "Job Args", id.to_string())?;
        if opt_authed.is_none() && record.created_by != "anonymous" {
            return Err(Error::BadRequest(
                "As a non logged in user, you can only see jobs ran by anonymous users".to_string(),
            ));
        }

        log_job_view(
            &db,
            opt_authed.as_ref(),
            opt_tokened.token.as_deref(),
            &w_id,
            &id,
        )
        .await?;

        Ok(Json(record.args.map(|x| x.0).unwrap_or_default()))
    }
}

async fn get_started_at_by_ids(
    Extension(db): Extension<DB>,
    Json(mut ids): Json<Vec<Uuid>>,
) -> JsonResult<Vec<Option<chrono::DateTime<chrono::Utc>>>> {
    ids.truncate(100);

    let started_at = sqlx::query!(
        "SELECT id, started_at FROM v2_job_queue WHERE id = ANY($1)",
        ids.as_slice()
    )
    .fetch_all(&db)
    .await?;

    let as_map = started_at
        .iter()
        .map(|x| (x.id, x.started_at))
        .collect::<HashMap<_, _>>();

    let mut r = Vec::new();
    for id in ids {
        r.push(as_map.get(&id).map(|x| x.clone()).unwrap_or_default());
    }

    Ok(Json(r))
}

#[derive(Serialize, FromRow)]
struct ListableQueuedJob {
    pub id: Uuid,
    pub running: bool,
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
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListQueueQuery>,
) -> error::JsonResult<Vec<ListableQueuedJob>> {
    let include_args = lq.include_args.unwrap_or(false);

    if include_args && *CLOUD_HOSTED {
        return Err(error::Error::BadRequest(
            "include_args is not supported on cloud hosted Windmill".to_string(),
        ));
    }

    let args_field = if include_args {
        "v2_job.args"
    } else {
        "null as args"
    };

    let sql = list_queue_jobs_query(
        &w_id,
        &lq,
        &[
            "v2_job.id",
            "v2_job_queue.running",
            "v2_job.created_by",
            "v2_job.created_at",
            "v2_job_queue.started_at",
            "v2_job_queue.scheduled_for",
            "v2_job.runnable_id as script_hash",
            "v2_job.runnable_path as script_path",
            args_field,
            "v2_job.kind as job_kind",
            "CASE WHEN v2_job.trigger_kind = 'schedule' THEN v2_job.trigger END as schedule_path",
            "v2_job.permissioned_as",
            "v2_job.flow_step_id IS NOT NULL as is_flow_step",
            "v2_job.script_lang as language",
            "v2_job.permissioned_as_email as email",
            "v2_job_queue.suspend",
            "v2_job.tag",
            "v2_job.priority",
            "v2_job.workspace_id",
        ],
        pagination,
        false,
        get_scope_tags(&authed),
    )
    .sql()?;
    let mut tx = user_db.begin(&authed).await?;
    let jobs = sqlx::query_as::<_, ListableQueuedJob>(&sql)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(jobs))
}

#[derive(Deserialize)]
pub struct CancelSelectionQuery {
    force_cancel: Option<bool>,
}

async fn cancel_selection(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<CancelSelectionQuery>,
    Json(jobs): Json<Vec<Uuid>>,
) -> error::JsonResult<Vec<Uuid>> {
    let mut tx = user_db.begin(&authed).await?;
    let tags = get_scope_tags(&authed).map(|v| v.iter().map(|s| s.to_string()).collect_vec());
    let jobs_to_cancel = sqlx::query_scalar!(
            "SELECT j.id AS \"id!\" FROM v2_job j LEFT JOIN v2_job_queue q USING (id) WHERE j.id = ANY($1) AND j.trigger_kind IS DISTINCT FROM 'schedule'::job_trigger_kind AND ($2::text[] IS NULL OR j.tag = ANY($2))",
            &jobs,
            tags.as_ref().map(|v| v.as_slice())
        )
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;

    cancel_jobs(
        jobs_to_cancel,
        &db,
        authed.username.as_str(),
        w_id.as_str(),
        query.force_cancel.unwrap_or(false),
    )
    .await
}

async fn list_filtered_job_uuids(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(lq): Query<ListCompletedQuery>,
) -> error::JsonResult<Vec<Uuid>> {
    require_admin(authed.is_admin, &authed.username)?;

    let mut sqlb = list_completed_jobs_query(
        w_id.as_str(),
        None,
        0,
        &lq,
        &["v2_job.id"],
        false,
        get_scope_tags(&authed),
    );
    let sqlb2 = list_queue_jobs_query(
        w_id.as_str(),
        &lq.into(),
        &["v2_job.id"],
        Pagination { page: None, per_page: None },
        false,
        get_scope_tags(&authed),
    );
    let query = sqlb.union_all(sqlb2.subquery()?).subquery()?;
    let ids = sqlx::query_scalar(query.as_str()).fetch_all(&db).await?;
    Ok(Json(ids))
}

async fn list_filtered_uuids(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,

    Path(w_id): Path<String>,
    Query(lq): Query<ListQueueQuery>,
) -> error::JsonResult<Vec<Uuid>> {
    require_admin(authed.is_admin, &authed.username)?;

    let mut sqlb = SqlBuilder::select_from("v2_job_queue")
        .fields(&["v2_job_queue.id"])
        .clone();

    sqlb = join_concurrency_key(lq.concurrency_key.as_ref(), sqlb);

    sqlb.and_where_ne("v2_job.trigger_kind", "'schedule'")
        .or_where_is_null("v2_job.trigger_kind");

    if let Some(tags) = get_scope_tags(&authed) {
        sqlb.and_where_in(
            "v2_job.tag",
            &tags.iter().map(|x| quote(x)).collect::<Vec<_>>(),
        );
    }

    sqlb = filter_list_queue_query(sqlb, &lq, w_id.as_str(), false);

    let sql = sqlb.query()?;
    let jobs = sqlx::query_scalar(sql.as_str()).fetch_all(&db).await?;

    Ok(Json(jobs))
}

#[derive(Serialize)]
struct QueueStats {
    database_length: i64,
    suspended: Option<i64>,
}

#[derive(Deserialize)]
pub struct CountQueueJobsQuery {
    all_workspaces: Option<bool>,
    tags: Option<String>,
}

async fn count_queue_jobs(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(cq): Query<CountQueueJobsQuery>,
) -> error::JsonResult<QueueStats> {
    let tags = cq
        .tags
        .map(|t| t.split(',').map(|s| s.to_string()).collect::<Vec<_>>());
    Ok(Json(
            sqlx::query_as!(
                QueueStats,
                "SELECT coalesce(COUNT(*) FILTER(WHERE q.suspend = 0 AND q.running = false), 0) as \"database_length!\", coalesce(COUNT(*) FILTER(WHERE q.suspend > 0), 0) as \"suspended!\" FROM v2_job_queue q JOIN v2_job j USING (id) WHERE (j.workspace_id = $1 OR $2) AND q.scheduled_for <= now() AND ($3::text[] IS NULL OR j.tag = ANY($3))",
                w_id,
                w_id == "admins" && cq.all_workspaces.unwrap_or(false),
                tags.as_ref().map(|v| v.as_slice())
            )
            .fetch_one(&db)
            .await?,
        ))
}

#[derive(Deserialize)]
pub struct CountCompletedJobsQuery {
    completed_after_s_ago: Option<i64>,
    success: Option<bool>,
    tags: Option<String>,
    all_workspaces: Option<bool>,
}

async fn count_completed_jobs_detail(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(query): Query<CountCompletedJobsQuery>,
) -> error::JsonResult<i64> {
    let mut sqlb = SqlBuilder::select_from("v2_job_completed");
    //FOR RLS
    sqlb.join("v2_job USING (id)");
    sqlb.field("COUNT(*) as count");

    if !query.all_workspaces.unwrap_or(false) {
        sqlb.and_where_eq("v2_job.workspace_id", "?".bind(&w_id));
    }

    if let Some(after_s_ago) = query.completed_after_s_ago {
        let after = Utc::now() - chrono::Duration::seconds(after_s_ago);
        sqlb.and_where_gt("completed_at", "?".bind(&after.to_rfc3339()));
    }

    if let Some(success) = query.success {
        if success {
            sqlb.and_where_eq("status", "'success'")
                .or_where_eq("status", "'skipped'");
        } else {
            sqlb.and_where_ne("status", "'success'")
                .and_where_ne("status", "'skipped'");
        }
    }

    if let Some(tags) = query.tags {
        sqlb.and_where_in(
            "v2_job.tag",
            &tags
                .split(",")
                .map(|t| format!("'{}'", t))
                .collect::<Vec<_>>(),
        );
    }

    let sql = sqlb.sql()?;
    let stats = sqlx::query_scalar::<_, i64>(&sql).fetch_one(&db).await?;

    Ok(Json(stats))
}

async fn count_completed_jobs(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> error::JsonResult<QueueStats> {
    Ok(Json(
            sqlx::query_as!(
                QueueStats,
                "SELECT coalesce(COUNT(*), 0) as \"database_length!\", null::bigint as suspended FROM v2_job_completed WHERE workspace_id = $1",
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
) -> error::JsonResult<Vec<Job>> {
    let include_args = lq.include_args.unwrap_or(false);

    if include_args && *CLOUD_HOSTED {
        return Err(error::Error::BadRequest(
            "include_args is not supported on cloud hosted Windmill".to_string(),
        ));
    }

    let (per_page, offset) = paginate(pagination);
    let lqc = lq.clone();

    if offset > 0 {
        tracing::warn!("offset is not 0, but is ignored for list_jobs. Use created_before or completed_before instead.");
    }

    if lq.success.is_some() && lq.running.is_some_and(|x| x) {
        return Err(error::Error::BadRequest(
            "cannot specify both success and running".to_string(),
        ));
    }

    // Create dynamic field arrays when include_args is true
    let cj_fields: Vec<&str>;
    let qj_fields: Vec<&str>;
    let cj_fields_ref: &[&str];
    let qj_fields_ref: &[&str];

    if include_args {
        cj_fields = UnifiedJob::completed_job_fields()
            .iter()
            .map(|f| {
                if *f == "null as args" {
                    "v2_job.args"
                } else {
                    *f
                }
            })
            .collect();
        qj_fields = UnifiedJob::queued_job_fields()
            .iter()
            .map(|f| {
                if *f == "null as args" {
                    "v2_job.args"
                } else {
                    *f
                }
            })
            .collect();
        cj_fields_ref = &cj_fields;
        qj_fields_ref = &qj_fields;
    } else {
        cj_fields_ref = UnifiedJob::completed_job_fields();
        qj_fields_ref = UnifiedJob::queued_job_fields();
    }

    let sqlc = if lq.running.is_none() {
        Some(list_completed_jobs_query(
            &w_id,
            Some(per_page),
            0,
            &ListCompletedQuery { order_desc: Some(true), ..lqc },
            cj_fields_ref,
            true,
            get_scope_tags(&authed),
        ))
    } else {
        None
    };

    let sql = if lq.success.is_none()
        && lq.label.is_none()
        && lq.created_before.is_none()
        && lq.started_before.is_none()
        && lq.created_or_started_before.is_none()
        && lq.completed_before.is_none()
    {
        let mut sqlq = list_queue_jobs_query(
            &w_id,
            &ListQueueQuery { order_desc: Some(true), ..lq.into() },
            qj_fields_ref,
            Pagination { per_page: None, page: None },
            true,
            get_scope_tags(&authed),
        );

        if let Some(sqlc) = sqlc {
            format!("{} UNION ALL {}", &sqlq.subquery()?, &sqlc.subquery()?,)
        } else {
            sqlq.limit(per_page).offset(offset).query()?
        }
    } else {
        if sqlc.is_none() {
            return Err(error::Error::BadRequest(
                "cannot specify success, label, created_or_started_before, or starte
                    d_before with running"
                    .to_string(),
            ));
        }
        sqlc.unwrap().limit(per_page).offset(offset).query()?
    };
    // tracing::info!("sql: {}", &sql);
    let mut tx: Transaction<'_, Postgres> = user_db.begin(&authed).await?;

    let jobs: Vec<UnifiedJob> = sqlx::query_as(&sql)
        .fetch_all(&mut *tx)
        .warn_after_seconds_with_sql(5, format!("list_jobs: {}", sql))
        .await?;
    tx.commit().await?;

    Ok(Json(jobs.into_iter().map(From::from).collect()))
}

pub async fn resume_suspended_flow_as_owner(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((_w_id, flow_id)): Path<(String, Uuid)>,
    QueryOrBody(value): QueryOrBody<serde_json::Value>,
) -> error::Result<StatusCode> {
    let mut tx = db.begin().await?;

    let (flow, job_id) = get_suspended_flow_info(flow_id, &mut tx).await?;

    let flow_path = flow.script_path.as_deref().unwrap_or_else(|| "");
    require_owner_of_path(&authed, flow_path)?;
    check_scopes(&authed, || format!("jobs:run:flows:{}", flow_path))?;

    // Check approval conditions (self-approval, required groups, etc.)
    if let Some(ref flow_status_value) = flow.flow_status {
        if let Ok(flow_status) = serde_json::from_value::<FlowStatus>(flow_status_value.clone()) {
            let trigger_email = flow.email.as_deref().unwrap_or("");
            conditionally_require_authed_user(Some(authed.clone()), flow_status, trigger_email)?;
        }
    }

    let value = value.unwrap_or(serde_json::Value::Null);

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
    opt_tokened: OptTokened,
    Extension(db): Extension<DB>,
    Path((w_id, job_id, resume_id, secret)): Path<(String, Uuid, u32, String)>,
    Query(approver): Query<QueryApprover>,
    QueryOrBody(value): QueryOrBody<serde_json::Value>,
) -> error::Result<StatusCode> {
    resume_suspended_job_internal(
        value,
        db,
        w_id,
        job_id,
        resume_id,
        approver,
        secret,
        authed,
        opt_tokened,
        true,
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
    opt_tokened: OptTokened,
    approved: bool,
) -> Result<StatusCode, Error> {
    let value = value.unwrap_or(serde_json::Value::Null);
    verify_suspended_secret(&w_id, &db, job_id, resume_id, &approver, secret).await?;

    // Get flow info - works for both step-level (job_id is a step) and flow-level (job_id is the flow)
    let (flow_info, is_flow_level) = get_flow_info_for_resume(job_id, &db).await?;

    // For step-level resumes, verify user auth and flow status
    // For flow-level resumes (pre-approvals), the flow might not be at a suspended step yet
    if !is_flow_level {
        let parent_flow = GetQuery::new()
            .without_logs()
            .without_code()
            .without_flow()
            .fetch(&db, &flow_info.id, &w_id)
            .await?;
        let flow_status = parent_flow
            .flow_status()
            .ok_or_else(|| anyhow::anyhow!("unable to find the flow status in the flow job"))?;

        let trigger_email = match &parent_flow {
            Job::CompletedJob(job) => &job.email,
            Job::QueuedJob(job) => &job.email,
        };
        conditionally_require_authed_user(authed.clone(), flow_status, trigger_email)?;
    }

    let exists = sqlx::query_scalar!(
        r#"
            SELECT EXISTS (SELECT 1 FROM resume_job WHERE id = $1)
            "#,
        Uuid::from_u128(job_id.as_u128() ^ resume_id as u128),
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    if exists {
        return Err(anyhow::anyhow!("resume request already sent").into());
    }

    let approver_value = if authed.as_ref().is_none()
        || (approver
            .approver
            .clone()
            .is_some_and(|x| x != "".to_string()))
    {
        approver.approver
    } else {
        authed.as_ref().map(|x| x.username.clone())
    };
    let mut tx: Transaction<'_, Postgres> = db.begin().await?;

    insert_resume_job(
        resume_id,
        job_id,
        &flow_info,
        value,
        approver_value.clone(),
        approved,
        &mut tx,
    )
    .await?;

    if !approved {
        sqlx::query!(
            "UPDATE v2_job_queue SET suspend = 0 WHERE id = $1",
            flow_info.id
        )
        .execute(&mut *tx)
        .await?;
    } else if is_flow_level {
        // For flow-level resumes, decrement the suspend counter if the flow is currently suspended
        // The approval will be matched when the worker checks for resumes (both step-level and flow-level)
        resume_immediately_for_flow_level(&flow_info, &mut tx).await?;
    } else {
        // For step-level resumes, try to resume immediately if the step is waiting
        resume_immediately_if_relevant(flow_info, job_id, &mut tx).await?;
    }

    let approver = approver_value.unwrap_or_else(|| "anonymous".to_string());

    let audit_author = match authed {
        Some(authed) => (&authed).into(),
        None => AuditAuthor {
            email: approver.clone(),
            username: approver.clone(),
            username_override: None,
            token_prefix: opt_tokened
                .token
                .map(|s| s[0..TOKEN_PREFIX_LEN].to_string()),
        },
    };
    audit_log(
        &mut *tx,
        &audit_author,
        "jobs.suspend_resume",
        ActionKind::Update,
        &w_id,
        Some(
            &serde_json::json!({
                "approved": approved,
                "job_id": job_id,
                "details": if approved {
                    format!("Approved by {}", &approver)
                } else {
                    format!("Cancelled by {}", &approver)
                }
            })
            .to_string(),
        ),
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::CREATED)
}

async fn verify_suspended_secret(
    w_id: &String,
    db: &DB,
    job_id: Uuid,
    resume_id: u32,
    approver: &QueryApprover,
    secret: String,
) -> Result<(), Error> {
    let key = get_workspace_key(w_id, db).await?;
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).map_err(to_anyhow)?;
    mac.update(job_id.as_bytes());
    mac.update(resume_id.to_be_bytes().as_ref());
    if let Some(approver) = approver.approver.clone() {
        mac.update(approver.as_bytes());
    }
    mac.verify_slice(hex::decode(secret)?.as_ref())
        .map_err(|_| anyhow::anyhow!("Invalid signature"))?;
    Ok(())
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
                    "UPDATE v2_job_queue SET suspend = $1 WHERE id = $2",
                    suspend,
                    flow.id,
                )
                .execute(&mut **tx)
                .await?;
            }
        },
    )
}

/// For flow-level resumes, decrement the suspend counter if the flow is currently suspended.
/// Unlike step-level resumes, we don't check if the job_id matches - we just need the flow
/// to be in a suspended state.
async fn resume_immediately_for_flow_level<'c>(
    flow: &FlowInfo,
    tx: &mut Transaction<'c, Postgres>,
) -> error::Result<()> {
    if flow.suspend > 0 {
        let new_suspend = flow.suspend - 1;
        sqlx::query!(
            "UPDATE v2_job_queue SET suspend = $1 WHERE id = $2",
            new_suspend,
            flow.id,
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
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
    email: Option<String>,
}

/// Get flow info from either a step job (by looking up its parent) or a flow job directly.
/// Returns (FlowInfo, is_flow_level) where is_flow_level indicates if job_id was a flow job.
async fn get_flow_info_for_resume(job_id: Uuid, db: &DB) -> error::Result<(FlowInfo, bool)> {
    // Single query that determines if job_id is a flow or step, and fetches the appropriate flow info
    let result = sqlx::query!(
        r#"
        WITH job_info AS (
            SELECT id, kind::text AS kind, parent_job
            FROM v2_job
            WHERE id = $1
        )
        SELECT
            q.id AS "id!",
            s.flow_status,
            q.suspend AS "suspend!",
            j.runnable_path AS script_path,
            j.permissioned_as_email AS email,
            (ji.kind IN ('flow', 'flowpreview')) AS "is_flow_level!"
        FROM job_info ji
        JOIN v2_job_queue q ON q.id = CASE
            WHEN ji.kind IN ('flow', 'flowpreview') THEN ji.id
            ELSE ji.parent_job
        END
        JOIN v2_job j ON j.id = q.id
        JOIN v2_job_status s ON s.id = q.id
        FOR UPDATE OF q
        "#,
        job_id,
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| anyhow::anyhow!("job not found or parent flow not in queue: {}", job_id))?;

    let flow_info = FlowInfo {
        id: result.id,
        flow_status: result.flow_status,
        suspend: result.suspend,
        script_path: result.script_path,
        email: Some(result.email),
    };

    Ok((flow_info, result.is_flow_level))
}

async fn get_suspended_flow_info<'c>(
    job_id: Uuid,
    tx: &mut Transaction<'c, Postgres>,
) -> error::Result<(FlowInfo, Uuid)> {
    let flow = sqlx::query_as!(
            FlowInfo,
            r#"
            SELECT j.id AS "id!", COALESCE(s.flow_status, s.workflow_as_code_status) as flow_status, q.suspend AS "suspend!", j.runnable_path as script_path, j.permissioned_as_email as email
            FROM v2_job_queue q JOIN v2_job j USING (id) LEFT JOIN v2_job_status s USING (id)
            WHERE j.id = $1
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
    opt_tokened: OptTokened,
    Extension(db): Extension<DB>,
    Path((w_id, job_id, resume_id, secret)): Path<(String, Uuid, u32, String)>,
    Query(approver): Query<QueryApprover>,
    QueryOrBody(value): QueryOrBody<serde_json::Value>,
) -> error::Result<StatusCode> {
    resume_suspended_job_internal(
        value,
        db,
        w_id,
        job_id,
        resume_id,
        approver,
        secret,
        authed,
        opt_tokened,
        false,
    )
    .await
}

#[derive(Serialize)]
pub struct SuspendedJobFlow {
    pub job: Job,
    pub approvers: Vec<Approval>,
}

pub async fn get_suspended_job_flow(
    authed: Option<ApiAuthed>,
    opt_tokened: OptTokened,
    Extension(db): Extension<DB>,
    Path((w_id, job, resume_id, secret)): Path<(String, Uuid, u32, String)>,
    Query(approver): Query<QueryApprover>,
) -> error::Result<Response> {
    verify_suspended_secret(&w_id, &db, job, resume_id, &approver, secret).await?;

    let flow_id = sqlx::query_scalar!(
        r#"
            SELECT parent_job
            FROM v2_job
            WHERE id = $1 AND workspace_id = $2
            "#,
        job,
        w_id
    )
    .fetch_optional(&db)
    .await?
    .flatten()
    .ok_or_else(|| anyhow::anyhow!("parent flow job not found"))?;

    let flow = GetQuery::new()
        .without_logs()
        .without_code()
        .fetch(&db, &flow_id, &w_id)
        .await?;

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
    };
    conditionally_require_authed_user(authed.clone(), flow_status.clone(), trigger_email)?;

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
        .fetch_all(&db)
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

    log_job_view(
        &db,
        authed.as_ref(),
        opt_tokened.token.as_deref(),
        &w_id,
        &job,
    )
    .await?;

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
                            let error_msg = format!(
                                    "Only users from one of the following groups are allowed to approve this workflow: {}",
                                    approval_conditions.user_groups_required.join(", ")
                                );
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
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, job_id, resume_id)): Path<(String, Uuid, u32)>,
    Query(approver): Query<QueryApprover>,
) -> error::Result<String> {
    let key = get_workspace_key(&w_id, &db).await?;
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
            SELECT COALESCE(s.flow_status, s.workflow_as_code_status)->'user_states'->$1
            FROM v2_job_queue q LEFT JOIN v2_job_status s USING (id)
            WHERE q.id = $2 AND q.workspace_id = $3
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
            UPDATE v2_job_status f SET flow_status = JSONB_SET(flow_status,  ARRAY['user_states'], JSONB_SET(COALESCE(flow_status->'user_states', '{}'::jsonb), ARRAY[$1], $2))
            FROM v2_job j
            WHERE f.id = $3 AND f.id = j.id AND j.workspace_id = $4 AND kind IN ('flow', 'flowpreview', 'flownode') RETURNING 1
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
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, job_id, resume_id)): Path<(String, Uuid, u32)>,
    Query(approver): Query<QueryApprover>,
) -> error::JsonResult<ResumeUrls> {
    get_resume_urls_internal(
        Extension(db),
        Path((w_id, job_id, resume_id)),
        Query(approver),
    )
    .await
}

pub async fn get_resume_urls_internal(
    Extension(db): Extension<DB>,
    Path((w_id, job_id, resume_id)): Path<(String, Uuid, u32)>,
    Query(approver): Query<QueryApprover>,
) -> error::JsonResult<ResumeUrls> {
    let key = get_workspace_key(&w_id, &db).await?;

    // If flow_level is true, use the parent flow ID for the signature and URLs
    // This allows pre-approvals that can be consumed by any later suspend step
    let target_job_id = if approver.flow_level.unwrap_or(false) {
        get_flow_id_for_job(&db, job_id).await?
    } else {
        job_id
    };

    let signature = create_signature(key, target_job_id, resume_id, approver.approver.clone())?;
    let approver_query = approver
        .approver
        .as_ref()
        .map(|x| format!("?approver={}", encode(x)))
        .unwrap_or_else(String::new);

    let base_url_str = BASE_URL.read().await.clone();
    let base_url = base_url_str.as_str();
    let res = ResumeUrls {
        approvalPage: format!(
            "{base_url}/approve/{w_id}/{target_job_id}/{resume_id}/{signature}{approver_query}"
        ),
        cancel: build_resume_url(
            "cancel",
            &w_id,
            &target_job_id,
            &resume_id,
            &signature,
            &approver_query,
            &base_url,
        ),
        resume: build_resume_url(
            "resume",
            &w_id,
            &target_job_id,
            &resume_id,
            &signature,
            &approver_query,
            &base_url,
        ),
    };

    Ok(Json(res))
}

/// Get the flow ID for a job. If the job is a flow, returns the job_id.
/// If the job is a step in a flow, returns the parent flow ID.
async fn get_flow_id_for_job(db: &DB, job_id: Uuid) -> error::Result<Uuid> {
    // First check if the job is a flow itself (kind = 'flow' or 'flowpreview')
    let job_info = sqlx::query!(
        r#"
        SELECT kind::text as "kind!", parent_job
        FROM v2_job
        WHERE id = $1
        "#,
        job_id
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| anyhow::anyhow!("job not found: {}", job_id))?;

    // If it's a flow job, return the job_id itself
    if job_info.kind == "flow" || job_info.kind == "flowpreview" {
        return Ok(job_id);
    }

    // Otherwise, return the parent flow ID
    job_info
        .parent_job
        .ok_or_else(|| anyhow::anyhow!("job {} has no parent flow", job_id).into())
}

#[derive(Deserialize)]
struct CancelJob {
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum PreviewKind {
    Code,
    Identity,
    Noop,
    Bundle,
    Tarbundle,
    ScriptHash,
}

#[derive(Debug, Deserialize)]
struct Preview {
    content: Option<String>,
    kind: Option<PreviewKind>,
    script_hash: Option<String>,
    path: Option<String>,
    args: Option<HashMap<String, Box<JsonRawValue>>>,
    language: Option<ScriptLang>,
    tag: Option<String>,
    dedicated_worker: Option<bool>,
    lock: Option<String>,
    format: Option<String>,
    flow_path: Option<String>,
}

#[cfg(feature = "run_inline")]
#[derive(Debug, Deserialize)]
struct PreviewInline {
    content: String,
    args: Option<HashMap<String, Box<JsonRawValue>>>,
    language: ScriptLang,
}

#[cfg(feature = "run_inline")]
#[derive(Debug, Deserialize)]
struct InlineScriptArgs {
    args: Option<HashMap<String, Box<JsonRawValue>>>,
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

#[derive(Debug, Deserialize)]
struct DynamicSelectRequest {
    pub entrypoint_function: String,
    pub args: Option<HashMap<String, Box<JsonRawValue>>>,
    pub runnable_ref: DynamicSelectRunnableRef,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "source")]
pub enum DynamicSelectRunnableRef {
    #[serde(rename = "deployed")]
    Deployed { path: String, runnable_kind: RunnableKind },
    #[serde(rename = "inline")]
    Inline { code: String, lang: Option<ScriptLang> },
}

use windmill_common::flows::InputTransform;

#[derive(Deserialize)]
struct BatchReRunJobsBodyArgs {
    job_ids: Vec<Uuid>,
    script_options_by_path: HashMap<String, BatchReRunOptions>,
    flow_options_by_path: HashMap<String, BatchReRunOptions>,
}

#[derive(Deserialize)]
struct BatchReRunOptions {
    input_transforms: Option<HashMap<String, InputTransform>>,
    use_latest_version: Option<bool>,
}

#[derive(sqlx::FromRow, Serialize, Clone)]
struct BatchReRunQueryReturnType {
    id: Uuid,
    kind: JobKind,
    script_path: String,
    script_hash: ScriptHash,
    input: serde_json::Value,
    scheduled_for: chrono::DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<serde_json::Value>,
}

async fn batch_rerun_compute_js_expression(
    expr: String,
    job: BatchReRunQueryReturnType,
) -> error::Result<Box<RawValue>> {
    let job_no_schema = BatchReRunQueryReturnType { schema: None, ..job };
    let job_value =
        serde_json::to_value(&job_no_schema).map_err(|e| Error::ExecutionErr(e.to_string()))?;
    let mut globals = std::collections::HashMap::new();
    globals.insert("job".to_string(), job_value);
    windmill_jseval::eval_simple_js(expr, globals)
        .await
        .map_err(|e| Error::ExecutionErr(e.to_string()))
}

async fn batch_rerun_jobs(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(body): Json<BatchReRunJobsBodyArgs>,
) -> error::Result<Response> {
    check_scopes(&authed, || format!("jobs:run"))?;
    let stream = batch_rerun_jobs_inner(authed, db, user_db, w_id, body);

    let body = axum::body::Body::from_stream(stream.map(Result::<_, std::convert::Infallible>::Ok));

    Ok(Response::builder()
        .status(201)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .body(body)
        .unwrap())
}

fn batch_rerun_jobs_inner(
    authed: ApiAuthed,
    db: DB,
    user_db: UserDB,
    w_id: String,
    body: BatchReRunJobsBodyArgs,
) -> impl futures::Stream<Item = String> {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    tokio::spawn(async move {
        let mut job_stream = sqlx::query_as!(
                BatchReRunQueryReturnType,
                r#"SELECT
                        j.id,
                        j.kind AS "kind: _",
                        COALESCE(s.path, f.path) AS "script_path!",
                        COALESCE(s.hash, f.id) AS "script_hash!: _",
                        COALESCE(jc.started_at, jq.scheduled_for, make_date(1970, 1, 1)) AS "scheduled_for!: _",
                        args AS input,
                        COALESCE(s.schema, f.schema) AS "schema: _"
                    FROM v2_job j
                    LEFT JOIN script s ON j.runnable_id = s.hash AND j.kind = 'script'
                    LEFT JOIN flow_version f ON j.runnable_id = f.id AND j.runnable_path = f.path AND j.kind = 'flow'
                    LEFT JOIN v2_job_completed jc ON jc.id = j.id
                    LEFT JOIN v2_job_queue jq ON jq.id = j.id
                    WHERE j.id = ANY($1)
                        AND j.workspace_id = $2
                        AND COALESCE(s.hash, f.id) IS NOT NULL
                        AND COALESCE(s.path, f.path) IS NOT NULL"#,
                &body.job_ids,
                w_id
            ).fetch(&db);
        while let Some(Ok(job)) = job_stream.next().await {
            let job_result =
                batch_rerun_handle_job(&job, &authed, &db, &user_db, &w_id, &body).await;
            let send_to_stream_result = tx
                .send(match job_result {
                    Ok(uuid) => format!("{}\n", uuid),
                    Err(err) => format!("Error: {}\n", err.to_string()),
                })
                .await;
            match send_to_stream_result {
                Ok(_) => {}
                Err(e) => tracing::error!("Couldn't re-run job {}: {}", job.id, e.to_string()),
            }
        }
    });
    tokio_stream::wrappers::ReceiverStream::new(rx)
}

async fn batch_rerun_handle_job(
    job: &BatchReRunQueryReturnType,
    authed: &ApiAuthed,
    db: &DB,
    user_db: &UserDB,
    w_id: &String,
    body: &BatchReRunJobsBodyArgs,
) -> error::Result<String> {
    let options = if matches!(job.kind, JobKind::Script) {
        &body.script_options_by_path
    } else {
        &body.flow_options_by_path
    }
    .get(&job.script_path);

    let mut args: HashMap<String, Box<RawValue>> = serde_json::from_value(job.input.clone())?;
    let use_latest_version = options.and_then(|o| o.use_latest_version).unwrap_or(false);
    let input_transforms = options
        .and_then(|o| o.input_transforms.as_ref())
        .map(|t| t.iter())
        .into_iter()
        .flatten();

    let latest_schema;
    let schema = if use_latest_version {
        latest_schema = sqlx::query_scalar!(
                r#"SELECT COALESCE(
                    (SELECT DISTINCT ON (s.path) s.schema FROM script s WHERE s.path = jb.runnable_path AND jb.kind = 'script' ORDER BY s.path, s.created_at DESC),
                    (SELECT flow_version.schema FROM flow LEFT JOIN flow_version ON flow_version.id = flow.versions[array_upper(flow.versions, 1)] WHERE flow.path = jb.runnable_path AND jb.kind = 'flow')
                ) FROM v2_job jb
                WHERE jb.id = $1 AND jb.workspace_id = $2
                GROUP BY jb.kind, jb.runnable_path"#,
                &job.id,
                &w_id
            ).fetch_optional(db).await?.flatten();
        latest_schema.as_ref()
    } else {
        job.schema.as_ref()
    };
    let schema = schema
        .and_then(serde_json::Value::as_object)
        .and_then(|s| s.get("properties"))
        .and_then(serde_json::Value::as_object);
    for (property_name, transform) in input_transforms {
        let schema_has_key = schema
            .map(|s| s.contains_key(property_name))
            .unwrap_or(false);
        if !schema_has_key {
            continue;
        }
        match transform {
            InputTransform::Static { value } => {
                args.insert(property_name.clone(), value.clone());
            }
            InputTransform::Javascript { expr } => {
                args.insert(
                    property_name.clone(),
                    batch_rerun_compute_js_expression(expr.clone(), job.clone()).await?,
                );
            }
            InputTransform::Ai => {}
        }
    }

    // Call appropriate function to push job to queue
    match job.kind {
        JobKind::Flow => {
            let result = push_flow_job_by_path_into_queue(
                authed.clone(),
                db.clone(),
                None,
                user_db.clone(),
                w_id.clone(),
                StripPath(job.script_path.clone()),
                RunJobQuery { skip_preprocessor: Some(true), ..Default::default() },
                PushArgsOwned { extra: None, args },
                None,
            )
            .await;
            if let Ok((uuid, _, _)) = result {
                return Ok(uuid.to_string());
            }
        }
        JobKind::Script => {
            let result = if use_latest_version {
                push_script_job_by_path_into_queue(
                    authed.clone(),
                    db.clone(),
                    None,
                    user_db.clone(),
                    w_id.clone(),
                    StripPath(job.script_path.clone()),
                    RunJobQuery { skip_preprocessor: Some(true), ..Default::default() },
                    PushArgsOwned { extra: None, args },
                    None,
                )
                .await
                .map(|r| r.0)
            } else {
                run_job_by_hash_inner(
                    authed.clone(),
                    db.clone(),
                    user_db.clone(),
                    w_id.clone(),
                    job.script_hash,
                    RunJobQuery { skip_preprocessor: Some(true), ..Default::default() },
                    PushArgsOwned { extra: None, args },
                    None,
                )
                .await
                .map(|r| r.0)
            };
            if let Ok(uuid) = result {
                return Ok(uuid.to_string());
            }
        }
        _ => {}
    }
    Err(error::Error::ExecutionErr(
        format!("Couldn't re-run job {}", job.id).to_string(),
    ))
}

pub async fn run_flow_by_path(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    args: RawWebhookArgs,
) -> error::Result<(StatusCode, String)> {
    let (args, trigger_metadata) = get_args_and_trigger_metadata(
        &db,
        &authed,
        RunnableId::from_flow_path(flow_path.to_path()),
        &run_query,
        &w_id,
        args,
    )
    .await?;

    let (uuid, _, _) = push_flow_job_by_path_into_queue(
        authed,
        db,
        None,
        user_db,
        w_id,
        flow_path,
        run_query,
        args,
        trigger_metadata,
    )
    .await?;

    Ok((StatusCode::CREATED, uuid.to_string()))
}

pub async fn run_flow_by_version(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, version)): Path<(String, i64)>,
    Query(run_query): Query<RunJobQuery>,
    args: RawWebhookArgs,
) -> error::Result<(StatusCode, String)> {
    let args = args
        .to_args_from_runnable(
            &authed,
            &db,
            &w_id,
            RunnableId::from_flow_version(version),
            run_query.skip_preprocessor,
        )
        .await?;

    let (uuid, _) =
        run_flow_by_version_inner(authed, db, user_db, w_id, version, run_query, args, None)
            .await?;

    Ok((StatusCode::CREATED, uuid.to_string()))
}

pub async fn run_flow_by_version_inner(
    authed: ApiAuthed,
    db: DB,
    user_db: UserDB,
    w_id: String,
    version: i64,
    run_query: RunJobQuery,
    args: PushArgsOwned,
    trigger: Option<TriggerMetadata>,
) -> error::Result<(Uuid, Option<String>)> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    let flow_path = sqlx::query_scalar!(
        r#"
            SELECT
                path
            FROM
                flow_version
            WHERE
                id = $1 AND
                workspace_id = $2
            "#,
        version,
        &w_id
    )
    .fetch_one(&db)
    .await?;

    check_scopes(&authed, || format!("jobs:run:flows:{flow_path}"))?;

    let flow_version_info =
        get_flow_version_info_from_version(&db, version, &w_id, &flow_path).await?;

    let (uuid, early_return, _) = run_flow(
        &authed,
        &db,
        None,
        user_db,
        &w_id,
        &flow_path,
        flow_version_info,
        run_query,
        args,
        trigger,
    )
    .await?;

    Ok((uuid, early_return))
}

#[cfg(not(feature = "enterprise"))]
pub async fn restart_flow(
    _authed: ApiAuthed,
    Extension(_db): Extension<DB>,
    Extension(_user_db): Extension<UserDB>,
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
#[derive(Deserialize)]
pub struct RestartFlowRequestBody {
    step_id: String,
    branch_or_iteration_n: Option<usize>,
    flow_version: Option<i64>,
}

#[cfg(feature = "enterprise")]
pub async fn restart_flow(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, job_id)): Path<(String, Uuid)>,
    Query(run_query): Query<RunJobQuery>,
    Json(RestartFlowRequestBody { step_id, branch_or_iteration_n, flow_version }): Json<
        RestartFlowRequestBody,
    >,
) -> error::Result<(StatusCode, String)> {
    check_license_key_valid().await?;

    let mut tx = user_db.clone().begin(&authed).await?;
    let completed_job = sqlx::query!(
            "SELECT
                j.runnable_path as script_path, j.args AS \"args: sqlx::types::Json<HashMap<String, Box<RawValue>>>\",
                j.tag AS \"tag!\", j.priority
            FROM v2_job j
            WHERE j.id = $1 and j.workspace_id = $2",
            job_id,
            &w_id,
        )
        .fetch_optional(&mut *tx)
        .await?
        .with_context(|| "Unable to find completed job with the given job UUID")?;
    drop(tx);

    let flow_path = completed_job
        .script_path
        .with_context(|| "No flow path set for completed flow job")?;
    check_scopes(&authed, || format!("jobs:run:flows:{flow_path}"))?;

    let ehm = HashMap::new();
    let push_args = completed_job
        .args
        .as_ref()
        .map(|json| PushArgs { args: &json.0, extra: None })
        .unwrap_or_else(|| PushArgs::from(&ehm));

    let scheduled_for = run_query.get_scheduled_for(&db).await?;

    let tx = PushIsolationLevel::Isolated(user_db, authed.clone().into());

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::RestartedFlow {
            completed_job_id: job_id,
            step_id,
            branch_or_iteration_n,
            flow_version,
        },
        push_args,
        &authed.username,
        &authed.email,
        username_to_permissioned_as(&authed.username),
        authed.token_prefix.as_deref(),
        scheduled_for,
        None,
        run_query.parent_job,
        None,
        run_query.root_job,
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        Some(completed_job.tag),
        None,
        None,
        completed_job.priority,
        Some(&authed.clone().into()),
        false,
        None,
        None,
        run_query.suspended_mode,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, uuid.to_string()))
}

pub async fn run_script_by_path(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    args: RawWebhookArgs,
) -> error::Result<(StatusCode, String)> {
    let (args, trigger_metadata) = get_args_and_trigger_metadata(
        &db,
        &authed,
        RunnableId::from_script_path(script_path.to_path()),
        &run_query,
        &w_id,
        args,
    )
    .await?;

    let (uuid, _, _) = push_script_job_by_path_into_queue(
        authed,
        db,
        None,
        user_db,
        w_id,
        script_path,
        run_query,
        args,
        trigger_metadata,
    )
    .await?;

    Ok((StatusCode::CREATED, uuid.to_string()))
}

#[allow(unused)]
pub async fn get_args_and_trigger_metadata(
    db: &DB,
    authed: &ApiAuthed,
    runnable_id: RunnableId,
    run_query: &RunJobQuery,
    w_id: &str,
    args: RawWebhookArgs,
) -> error::Result<(PushArgsOwned, Option<TriggerMetadata>)> {
    use windmill_common::triggers::TriggerMetadata;

    // Build trigger metadata if this is a native trigger request
    #[cfg(feature = "native_trigger")]
    let (trigger_metadata, native_args) = if let Some(service_name_str) = &run_query.service_name {
        use crate::native_triggers::{prepare_native_trigger_args, ServiceName};
        let service_name = ServiceName::try_from(service_name_str.to_owned())?;
        let metadata = Some(TriggerMetadata::new(
            run_query.trigger_external_id.clone(),
            service_name.as_job_trigger_kind(),
        ));
        let body = match &args.body {
            crate::args::RawBody::Json(s) => s.clone(),
            crate::args::RawBody::Text(s) => s.clone(),
            _ => String::new(),
        };
        let native =
            prepare_native_trigger_args(service_name, db, w_id, &args.metadata.headers, body)
                .await?;
        (metadata, native)
    } else {
        (None, None)
    };

    #[cfg(not(feature = "native_trigger"))]
    let trigger_metadata: Option<TriggerMetadata> = None;
    #[cfg(not(feature = "native_trigger"))]
    let native_args: Option<windmill_queue::PushArgsOwned> = None;

    let args = if let Some(prepared) = native_args {
        prepared
    } else {
        args.to_args_from_runnable(
            &authed,
            &db,
            &w_id,
            runnable_id,
            run_query.skip_preprocessor,
        )
        .await?
    };

    Ok((args, trigger_metadata))
}

#[derive(Deserialize)]
pub struct WorkflowAsCodeQuery {
    pub skip_update: Option<bool>,
}

pub async fn run_workflow_as_code(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, job_id, entrypoint)): Path<(String, Uuid, String)>,
    Query(run_query): Query<RunJobQuery>,
    Query(wkflow_query): Query<WorkflowAsCodeQuery>,
    Json(task): Json<WorkflowTask>,
) -> error::Result<(StatusCode, String)> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;
    check_tag_available_for_workspace(&db, &w_id, &run_query.tag, &authed).await?;
    check_scopes(&authed, || format!("jobs:run"))?;

    let mut i = 1;

    if *CLOUD_HOSTED {
        tracing::info!("workflow_as_code_tracing id {i} ");
        i += 1;
    }

    if *CLOUD_HOSTED {
        tracing::info!("workflow_as_code_tracing id {i} ");
        i += 1;
    }

    let job = GetQuery::new()
        .without_logs()
        .fetch_queued(&db, &job_id, &w_id)
        .await?;

    if *CLOUD_HOSTED {
        tracing::info!("workflow_as_code_tracing id {i} ");
        i += 1;
    }

    let job = not_found_if_none(job, "Queued Job", &job_id.to_string())?;
    let JobExtended { inner: job, raw_code, raw_lock, .. } = job;

    let (_debouncing_settings, concurrency_settings) =
        windmill_common::runnable_settings::prefetch_cached_from_handle(
            job.runnable_settings_handle,
            &db,
        )
        .await?;

    let (job_payload, tag, _delete_after_use, timeout, on_behalf_of) = match job.job_kind {
        JobKind::Preview => (
            JobPayload::Code(RawCode {
                hash: None,
                content: raw_code.unwrap_or_default(),
                path: job.script_path,
                language: job.language.unwrap_or_else(|| ScriptLang::Deno),
                lock: raw_lock,
                concurrency_settings: concurrency_settings
                    .maybe_fallback(
                        windmill_queue::custom_concurrency_key(&db, &job.id)
                            .await
                            .map_err(to_anyhow)?,
                        job.concurrent_limit,
                        job.concurrency_time_window_s,
                    )
                    .into(),
                cache_ttl: job.cache_ttl,
                cache_ignore_s3_path: job.cache_ignore_s3_path,
                dedicated_worker: None,
                // TODO(debouncing): enable for this mode
                debouncing_settings: DebouncingSettings::default(),
            }),
            Some(job.tag.clone()),
            None,
            run_query.timeout,
            None,
        ),
        JobKind::Script => {
            let userdb_authed =
                UserDbWithAuthed { db: user_db.clone(), authed: &authed.to_authed_ref() };
            script_path_to_payload(
                job.script_path(),
                Some(userdb_authed),
                db.clone(),
                &w_id,
                run_query.skip_preprocessor,
            )
            .await?
        }
        _ => return Err(anyhow::anyhow!("Not supported").into()),
    };

    if *CLOUD_HOSTED {
        tracing::info!("workflow_as_code_tracing id {i} ");
        i += 1;
    }

    let mut extra = HashMap::new();
    extra.insert(ENTRYPOINT_OVERRIDE.to_string(), to_raw_value(&entrypoint));

    let args = PushArgs { args: &task.args.unwrap_or_else(HashMap::new), extra: Some(extra) };
    let scheduled_for = run_query.get_scheduled_for(&db).await?;

    let tag = run_query.tag.clone().or(tag).or(Some(job.tag));

    if *CLOUD_HOSTED {
        tracing::info!("workflow_as_code_tracing id {i} ");
        i += 1;
    }

    if *CLOUD_HOSTED {
        tracing::info!("workflow_as_code_tracing id {i} ");
        i += 1;
    }

    let (email, permissioned_as, push_authed, tx) =
        if let Some(on_behalf_of) = on_behalf_of.as_ref() {
            (
                on_behalf_of.email.as_str(),
                on_behalf_of.permissioned_as.clone(),
                None,
                PushIsolationLevel::IsolatedRoot(db.clone()),
            )
        } else {
            (
                authed.email.as_str(),
                username_to_permissioned_as(&authed.username),
                Some(authed.clone().into()),
                PushIsolationLevel::Isolated(user_db, authed.clone().into()),
            )
        };

    let (uuid, mut tx) = push(
        &db,
        tx,
        &w_id,
        job_payload,
        PushArgs { args: &args.args, extra: args.extra },
        authed.display_username(),
        email,
        permissioned_as,
        authed.token_prefix.as_deref(),
        scheduled_for,
        None,
        Some(job_id),
        None,
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
        push_authed.as_ref(),
        false,
        None,
        None,
        None,
    )
    .await?;

    if *CLOUD_HOSTED {
        tracing::info!("workflow_as_code_tracing id {i} ");
        i += 1;
    }

    if !wkflow_query.skip_update.unwrap_or(false) {
        sqlx::query!(
            "INSERT INTO v2_job_status (id, workflow_as_code_status)
                VALUES ($1, JSONB_SET('{}'::JSONB, array[$2], $3))
                ON CONFLICT (id) DO UPDATE SET
                    workflow_as_code_status = JSONB_SET(
                        COALESCE(v2_job_status.workflow_as_code_status, '{}'::JSONB),
                        array[$2],
                        $3
                    )",
            job_id,
            uuid.to_string(),
            serde_json::json!({ "scheduled_for": Utc::now(), "name": entrypoint }),
        )
        .execute(&mut *tx)
        .await?;
    } else {
        tracing::info!("Skipping update of flow status for job {job_id} in workspace {w_id}");
        sqlx::query!(
            "INSERT INTO v2_job_status (id, workflow_as_code_status) VALUES ($1, '{}'::JSONB)
                ON CONFLICT (id) DO NOTHING",
            job_id,
        )
        .execute(&mut *tx)
        .await?;
    }

    if *CLOUD_HOSTED {
        tracing::info!("workflow_as_code_tracing id {i} ");
        i += 1;
    }

    tx.commit().await?;

    if *CLOUD_HOSTED {
        tracing::info!("workflow_as_code_tracing id {i} ");
    }

    Ok((StatusCode::CREATED, uuid.to_string()))
}

lazy_static::lazy_static! {
    static ref JOB_VIEW_AUDIT_LOGS: bool = std::env::var("JOB_VIEW_AUDIT_LOGS")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(false);

    static ref JOB_VIEW_CACHE: JobViewCache = JobViewCache::new(50000);
}

struct JobViewCache {
    cache: Cache<String, std::time::Instant>,
}

impl JobViewCache {
    fn new(items_capacity: usize) -> Self {
        Self { cache: Cache::new(items_capacity) }
    }
    fn get_or_insert(&self, key: &str) -> Option<std::time::Instant> {
        match self.cache.get(key) {
            Some(t) if t < std::time::Instant::now() => {
                self.cache.insert(
                    key.to_string(),
                    std::time::Instant::now() + std::time::Duration::from_secs(60),
                );
                None
            }
            v => {
                self.cache.insert(
                    key.to_string(),
                    std::time::Instant::now() + std::time::Duration::from_secs(60),
                );
                v
            }
        }
    }
}

async fn log_job_view(
    db: &DB,
    opt_authed: Option<&ApiAuthed>,
    opt_token: Option<&str>,
    w_id: &str,
    job_id: &Uuid,
) -> error::Result<()> {
    if *JOB_VIEW_AUDIT_LOGS {
        let audit_author = match opt_authed {
            Some(authed) => AuditAuthor::from(authed),
            None => AuditAuthor {
                username: "anonymous".to_string(),
                username_override: None,
                email: "anonymous".to_string(),
                token_prefix: opt_token.map(|t| t[0..TOKEN_PREFIX_LEN].to_string()),
            },
        };
        if JOB_VIEW_CACHE
            .get_or_insert(&format!("{}_{}", job_id, audit_author.email))
            .is_none()
        {
            audit_log(
                db,
                &audit_author,
                "jobs.view",
                ActionKind::Execute,
                w_id,
                Some(&job_id.to_string()),
                None,
            )
            .await?;
        };
    }

    Ok(())
}

pub async fn run_wait_result_job_by_path_get(
    method: hyper::http::Method,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    args: RawWebhookArgs,
) -> error::Result<Response> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    let script_path = script_path.to_path();
    check_scopes(&authed, || format!("jobs:run:scripts:{script_path}"))?;

    if method == http::Method::HEAD {
        return Ok(Json(serde_json::json!("")).into_response());
    }

    let payload_as_args = run_query.payload_as_args()?;

    let mut args = args.process_args(&authed, &db, &w_id, None).await?;
    args.body = args::Body::HashMap(payload_as_args);

    let args = args
        .to_args_from_runnable(
            &db,
            &w_id,
            RunnableId::from_script_path(script_path),
            run_query.skip_preprocessor,
        )
        .await?;

    check_queue_too_long(&db, QUEUE_LIMIT_WAIT_RESULT.or(run_query.queue_limit)).await?;

    let user_db_with_authed =
        UserDbWithAuthed { db: user_db.clone(), authed: &authed.to_authed_ref() };
    let (job_payload, tag, delete_after_use, timeout, on_behalf_authed) = script_path_to_payload(
        script_path,
        Some(user_db_with_authed),
        db.clone(),
        &w_id,
        run_query.skip_preprocessor,
    )
    .await?;

    let tag = run_query.tag.clone().or(tag);
    check_tag_available_for_workspace(&db, &w_id, &tag, &authed).await?;

    let (email, permissioned_as, push_authed, tx) =
        if let Some(on_behalf_of) = on_behalf_authed.as_ref() {
            (
                on_behalf_of.email.as_str(),
                on_behalf_of.permissioned_as.clone(),
                None,
                PushIsolationLevel::IsolatedRoot(db.clone()),
            )
        } else {
            (
                authed.email.as_str(),
                username_to_permissioned_as(&authed.username),
                Some(authed.clone().into()),
                PushIsolationLevel::Isolated(user_db, authed.clone().into()),
            )
        };

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        job_payload,
        PushArgs { args: &args.args, extra: args.extra },
        authed.display_username(),
        email,
        permissioned_as,
        authed.token_prefix.as_deref(),
        None,
        None,
        run_query.parent_job,
        None,
        run_query.root_job,
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
        timeout,
        None,
        None,
        push_authed.as_ref(),
        false,
        None,
        None,
        run_query.suspended_mode,
    )
    .await?;
    tx.commit().await?;

    let wait_result = run_wait_result(&db, uuid, &w_id, None, &authed.username).await;
    if delete_after_use.unwrap_or(false) {
        delete_job_metadata_after_use(&db, uuid).await?;
    }
    return wait_result;
}

pub async fn run_wait_result_flow_by_path_get(
    method: hyper::http::Method,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    args: RawWebhookArgs,
) -> error::Result<Response> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    let path = flow_path.to_path();
    check_scopes(&authed, || format!("jobs:run:flows:{path}"))?;

    if method == http::Method::HEAD {
        return Ok(Json(serde_json::json!("")).into_response());
    }
    let payload_as_args = run_query.payload_as_args()?;

    let mut args = args.process_args(&authed, &db, &w_id, None).await?;
    args.body = args::Body::HashMap(payload_as_args);

    let args = args
        .to_args_from_runnable(
            &db,
            &w_id,
            RunnableId::from_flow_path(path),
            run_query.skip_preprocessor,
        )
        .await?;

    run_wait_result_flow_by_path_internal(db, run_query, flow_path, authed, user_db, args, w_id)
        .await
}

pub async fn run_wait_result_script_by_path(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    args: RawWebhookArgs,
) -> error::Result<Response> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    let path = script_path.to_path();
    check_scopes(&authed, || format!("jobs:run:scripts:{path}"))?;

    let args = args
        .to_args_from_runnable(
            &authed,
            &db,
            &w_id,
            RunnableId::from_script_path(path),
            run_query.skip_preprocessor,
        )
        .await?;

    run_wait_result_script_by_path_internal(db, run_query, script_path, authed, user_db, w_id, args)
        .await
}

pub async fn run_wait_result_script_by_path_internal(
    db: sqlx::Pool<Postgres>,
    run_query: RunJobQuery,
    script_path: StripPath,
    authed: ApiAuthed,
    user_db: UserDB,
    w_id: String,
    args: PushArgsOwned,
) -> error::Result<Response> {
    check_queue_too_long(&db, QUEUE_LIMIT_WAIT_RESULT.or(run_query.queue_limit)).await?;

    let db_authed = UserDbWithAuthed { db: user_db.clone(), authed: &authed.to_authed_ref() };
    let (job_payload, tag, delete_after_use, timeout, on_behalf_of) = script_path_to_payload(
        script_path.to_path(),
        Some(db_authed),
        db.clone(),
        &w_id,
        run_query.skip_preprocessor,
    )
    .await?;

    let tag = run_query.tag.clone().or(tag);
    check_tag_available_for_workspace(&db, &w_id, &tag, &authed).await?;

    let (email, permissioned_as, push_authed, tx) =
        if let Some(on_behalf_of) = on_behalf_of.as_ref() {
            (
                on_behalf_of.email.as_str(),
                on_behalf_of.permissioned_as.clone(),
                None,
                PushIsolationLevel::IsolatedRoot(db.clone()),
            )
        } else {
            (
                authed.email.as_str(),
                username_to_permissioned_as(&authed.username),
                Some(authed.clone().into()),
                PushIsolationLevel::Isolated(user_db, authed.clone().into()),
            )
        };

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        job_payload,
        PushArgs { args: &args.args, extra: args.extra },
        authed.display_username(),
        email,
        permissioned_as,
        authed.token_prefix.as_deref(),
        None,
        None,
        run_query.parent_job,
        None,
        run_query.root_job,
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
        timeout,
        None,
        None,
        push_authed.as_ref(),
        false,
        None,
        None,
        run_query.suspended_mode,
    )
    .await?;
    tx.commit().await?;

    let wait_result = run_wait_result(&db, uuid, &w_id, None, &authed.username).await;
    if delete_after_use.unwrap_or(false) {
        delete_job_metadata_after_use(&db, uuid).await?;
    }
    return wait_result;
}

pub async fn run_wait_result_script_by_hash(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, script_hash)): Path<(String, ScriptHash)>,
    Query(run_query): Query<RunJobQuery>,
    args: RawWebhookArgs,
) -> error::Result<Response> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    let args = args
        .to_args_from_runnable(
            &authed,
            &db,
            &w_id,
            RunnableId::from_script_hash(script_hash),
            run_query.skip_preprocessor,
        )
        .await?;

    check_queue_too_long(&db, run_query.queue_limit).await?;

    let hash = script_hash.0;
    let userdb_authed = UserDbWithAuthed { db: user_db.clone(), authed: &authed.to_authed_ref() };
    let ScriptHashInfo {
        path,
        tag,
        mut cache_ttl,
        mut cache_ignore_s3_path,
        language,
        dedicated_worker,
        priority,
        delete_after_use,
        timeout,
        has_preprocessor,
        on_behalf_of_email,
        created_by,
        runnable_settings:
            ScriptRunnableSettingsInline { concurrency_settings, debouncing_settings },
        ..
    } = get_script_info_for_hash(Some(userdb_authed), &db, &w_id, hash)
        .await?
        .prefetch_cached(&db)
        .await?;

    if let Some(run_query_cache_ttl) = run_query.cache_ttl {
        cache_ttl = Some(run_query_cache_ttl);
        cache_ignore_s3_path = run_query.cache_ignore_s3_path;
    }
    check_scopes(&authed, || format!("jobs:run:scripts:{path}"))?;

    let tag = run_query.tag.clone().or(tag);
    check_tag_available_for_workspace(&db, &w_id, &tag, &authed).await?;

    let (email, permissioned_as, push_authed, tx) = if let Some(email) = on_behalf_of_email.as_ref()
    {
        (
            email,
            username_to_permissioned_as(created_by.as_str()),
            None,
            PushIsolationLevel::IsolatedRoot(db.clone()),
        )
    } else {
        (
            &authed.email,
            username_to_permissioned_as(&authed.username),
            Some(authed.clone().into()),
            PushIsolationLevel::Isolated(user_db, authed.clone().into()),
        )
    };

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::ScriptHash {
            hash: ScriptHash(hash),
            path: path,
            concurrency_settings,
            debouncing_settings,
            cache_ttl,
            cache_ignore_s3_path,
            language,
            dedicated_worker,
            priority,
            apply_preprocessor: !run_query.skip_preprocessor.unwrap_or(false)
                && has_preprocessor.unwrap_or(false),
        },
        PushArgs { args: &args.args, extra: args.extra },
        authed.display_username(),
        email,
        permissioned_as,
        authed.token_prefix.as_deref(),
        None,
        None,
        run_query.parent_job,
        None,
        run_query.root_job,
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
        timeout,
        None,
        None,
        push_authed.as_ref(),
        false,
        None,
        None,
        run_query.suspended_mode,
    )
    .await?;
    tx.commit().await?;

    let wait_result = run_wait_result(&db, uuid, &w_id, None, &authed.username).await;
    if delete_after_use.unwrap_or(false) {
        delete_job_metadata_after_use(&db, uuid).await?;
    }
    return wait_result;
}

pub async fn run_wait_result_flow_by_path(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    args: RawWebhookArgs,
) -> error::Result<Response> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    let path = flow_path.to_path();
    check_scopes(&authed, || format!("jobs:run:flows:{path}"))?;

    let args = args
        .to_args_from_runnable(
            &authed,
            &db,
            &w_id,
            RunnableId::from_flow_path(path),
            run_query.skip_preprocessor,
        )
        .await?;

    run_wait_result_flow_by_path_internal(db, run_query, flow_path, authed, user_db, args, w_id)
        .await
}

pub async fn stream_flow_by_path(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, flow_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    method: hyper::http::Method,
    args: RawWebhookArgs,
) -> error::Result<Response> {
    stream_job(
        authed,
        db,
        user_db,
        w_id,
        RunnableId::from_flow_path(flow_path.to_path()),
        args,
        run_query,
        method == http::Method::GET,
    )
    .await
}

pub async fn stream_flow_by_version(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, version)): Path<(String, i64)>,
    Query(run_query): Query<RunJobQuery>,
    method: hyper::http::Method,
    args: RawWebhookArgs,
) -> error::Result<Response> {
    stream_job(
        authed,
        db,
        user_db,
        w_id,
        RunnableId::from_flow_version(version),
        args,
        run_query,
        method == http::Method::GET,
    )
    .await
}

pub async fn stream_script_by_path(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    Query(run_query): Query<RunJobQuery>,
    method: hyper::http::Method,
    args: RawWebhookArgs,
) -> error::Result<Response> {
    stream_job(
        authed,
        db,
        user_db,
        w_id,
        RunnableId::from_script_path(script_path.to_path()),
        args,
        run_query,
        method == http::Method::GET,
    )
    .await
}

pub async fn stream_script_by_hash(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_hash)): Path<(String, ScriptHash)>,
    Query(run_query): Query<RunJobQuery>,
    method: hyper::http::Method,
    args: RawWebhookArgs,
) -> error::Result<Response> {
    stream_job(
        authed,
        db,
        user_db,
        w_id,
        RunnableId::from_script_hash(script_hash),
        args,
        run_query,
        method == http::Method::GET,
    )
    .await
}

pub async fn stream_job(
    authed: ApiAuthed,
    db: DB,
    user_db: UserDB,
    w_id: String,
    runnable_id: RunnableId,
    args: RawWebhookArgs,
    run_query: RunJobQuery,
    is_get: bool,
) -> error::Result<Response> {
    let args = if is_get {
        let payload_as_args = run_query.payload_as_args()?;

        let mut args = args.process_args(&authed, &db, &w_id, None).await?;
        args.body = args::Body::HashMap(payload_as_args);

        let args = args
            .to_args_from_runnable(&db, &w_id, runnable_id.clone(), run_query.skip_preprocessor)
            .await?;
        args
    } else {
        args.to_args_from_runnable(
            &authed,
            &db,
            &w_id,
            runnable_id.clone(),
            run_query.skip_preprocessor,
        )
        .await?
    };

    let poll_delay_ms = run_query.poll_delay_ms;
    let (uuid, early_return) = match runnable_id {
        RunnableId::ScriptId(ScriptId::ScriptPath(script_path))
        | RunnableId::HubScript(script_path) => {
            let (uuid, _, _) = push_script_job_by_path_into_queue(
                authed.clone(),
                db.clone(),
                None,
                user_db,
                w_id.clone(),
                StripPath(script_path),
                run_query,
                args,
                None,
            )
            .await?;
            (uuid, None)
        }
        RunnableId::ScriptId(ScriptId::ScriptHash(script_hash)) => {
            let (uuid, _) = run_job_by_hash_inner(
                authed.clone(),
                db.clone(),
                user_db,
                w_id.clone(),
                script_hash,
                run_query,
                args,
                None,
            )
            .await?;
            (uuid, None)
        }
        RunnableId::FlowId(FlowId::FlowPath(flow_path)) => {
            let (uuid, early_return, _) = push_flow_job_by_path_into_queue(
                authed.clone(),
                db.clone(),
                None,
                user_db,
                w_id.clone(),
                StripPath(flow_path),
                run_query,
                args,
                None,
            )
            .await?;
            (uuid, early_return)
        }
        RunnableId::FlowId(FlowId::FlowVersion(version)) => {
            let (uuid, early_return) = run_flow_by_version_inner(
                authed.clone(),
                db.clone(),
                user_db,
                w_id.clone(),
                version,
                run_query,
                args,
                None,
            )
            .await?;
            (uuid, early_return)
        }
    };

    let opt_authed = Some(authed.clone());
    let opt_tokened = OptTokened { token: None }; // ignored when authed is some
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(|x| {
        format!(
            "data: {}\n\n",
            serde_json::to_string(&x).unwrap_or_default()
        )
    });

    start_job_update_sse_stream(
        opt_authed,
        opt_tokened,
        db,
        w_id,
        uuid,
        None,
        None,
        None,
        None,
        Some(true),
        Some(true),
        None,
        None,
        tx,
        poll_delay_ms,
        early_return,
    );

    let body = axum::body::Body::from_stream(stream.map(Result::<_, std::convert::Infallible>::Ok));

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(body)
        .unwrap())
}

pub async fn run_wait_result_flow_by_path_internal(
    db: sqlx::Pool<Postgres>,
    run_query: RunJobQuery,
    flow_path: StripPath,
    authed: ApiAuthed,
    user_db: UserDB,
    args: PushArgsOwned,
    w_id: String,
) -> error::Result<Response> {
    check_queue_too_long(&db, run_query.queue_limit).await?;

    let flow_path = flow_path.to_path();

    let userdb_authed = UserDbWithAuthed { db: user_db.clone(), authed: &authed.to_authed_ref() };

    let flow_version_info =
        get_latest_flow_version_info_for_path(Some(userdb_authed), &db, &w_id, &flow_path, true)
            .await?;

    run_flow_and_wait_result(
        &authed,
        &db,
        user_db,
        &w_id,
        flow_path,
        flow_version_info,
        run_query,
        args,
        None,
    )
    .await
}

pub async fn run_wait_result_flow_by_version_get(
    method: hyper::http::Method,
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, version)): Path<(String, i64)>,
    Query(run_query): Query<RunJobQuery>,
    args: RawWebhookArgs,
) -> error::Result<Response> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    let flow_path = sqlx::query_scalar!(
        "SELECT path FROM flow_version WHERE id = $1 AND workspace_id = $2",
        version,
        &w_id
    )
    .fetch_one(&db)
    .await?;

    check_scopes(&authed, || format!("jobs:run:flows:{flow_path}"))?;

    if method == http::Method::HEAD {
        return Ok(Json(serde_json::json!("")).into_response());
    }

    let payload_as_args = run_query.payload_as_args()?;

    let mut args = args.process_args(&authed, &db, &w_id, None).await?;
    args.body = args::Body::HashMap(payload_as_args);

    let args = args
        .to_args_from_runnable(
            &db,
            &w_id,
            RunnableId::from_flow_version(version),
            run_query.skip_preprocessor,
        )
        .await?;

    let flow_version_info =
        get_flow_version_info_from_version(&db, version, &w_id, &flow_path).await?;

    run_flow_and_wait_result(
        &authed,
        &db,
        user_db,
        &w_id,
        &flow_path,
        flow_version_info,
        run_query,
        args,
        None,
    )
    .await
}

pub async fn run_wait_result_flow_by_version(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, version)): Path<(String, i64)>,
    Query(run_query): Query<RunJobQuery>,
    args: RawWebhookArgs,
) -> error::Result<Response> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    let flow_path = sqlx::query_scalar!(
        r#"
                SELECT
                    path
                FROM
                    flow_version
                WHERE
                    id = $1 AND
                    workspace_id = $2
            "#,
        version,
        &w_id
    )
    .fetch_one(&db)
    .await?;

    check_scopes(&authed, || format!("jobs:run:flows:{flow_path}"))?;

    let args = args
        .to_args_from_runnable(
            &authed,
            &db,
            &w_id,
            RunnableId::from_flow_version(version),
            run_query.skip_preprocessor,
        )
        .await?;

    let flow_version_info =
        get_flow_version_info_from_version(&db, version, &w_id, &flow_path).await?;

    run_flow_and_wait_result(
        &authed,
        &db,
        user_db,
        &w_id,
        &flow_path,
        flow_version_info,
        run_query,
        args,
        None,
    )
    .await
}

async fn run_preview_script(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(run_query): Query<RunJobQuery>,
    Json(preview): Json<Preview>,
) -> error::Result<(StatusCode, String)> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;
    if authed.is_operator {
        return Err(error::Error::NotAuthorized(
            "Operators cannot run preview jobs for security reasons".to_string(),
        ));
    }
    require_path_read_access_for_preview(&authed, &preview.path)?;
    let scheduled_for = run_query.get_scheduled_for(&db).await?;
    let tag = run_query.tag.clone().or(preview.tag.clone());
    check_tag_available_for_workspace(&db, &w_id, &tag, &authed).await?;
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into());

    let preview_args = preview.args.unwrap_or_default();
    let flow_path_extra = preview.flow_path.map(|fp| {
        let mut extra = HashMap::new();
        extra.insert("_FLOW_PATH".to_string(), to_raw_value(&fp));
        extra
    });
    let push_args = PushArgs { extra: flow_path_extra, args: &preview_args };

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        match preview.kind {
            Some(PreviewKind::Identity) => JobPayload::Identity,
            Some(PreviewKind::Noop) => JobPayload::Noop,
            _ => JobPayload::Code(RawCode {
                hash: preview
                    .script_hash
                    .as_ref()
                    .and_then(|s| windmill_common::scripts::to_i64(s).ok()),
                content: preview.content.unwrap_or_default(),
                path: preview.path,
                language: preview.language.unwrap_or(ScriptLang::Deno),
                lock: preview.lock,
                concurrency_settings: ConcurrencySettingsWithCustom::default(), // TODO(gbouv): once I find out how to store limits in the content of a script, should be easy to plug limits here
                debouncing_settings: DebouncingSettings::default(), // TODO(pyra): same as for concurrency limits.
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: preview.dedicated_worker,
            }),
        },
        push_args,
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        authed.token_prefix.as_deref(),
        scheduled_for,
        None,
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
        Some(&authed.clone().into()),
        false,
        None,
        None,
        None,
    )
    .await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, uuid.to_string()))
}

#[cfg(feature = "run_inline")]
async fn run_inline_preview_script(
    OptJobAuthed { authed, job_id }: OptJobAuthed,
    Tokened { token }: Tokened,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(preview): Json<PreviewInline>,
) -> error::Result<Response> {
    if let Some(job_id) = job_id {
        register_potential_assets_on_inline_execution(job_id, &w_id, &preview);
    }
    let utils = get_worker_internal_server_inline_utils()?;
    let result = utils.run_inline_preview_script.as_ref()(RunInlinePreviewScriptFnParams {
        content: preview.content,
        args: preview.args,
        workspace_id: w_id.clone(),
        base_internal_url: utils.base_internal_url.clone(),
        killpill_rx: utils.killpill_rx.resubscribe(),
        created_by: authed.display_username().to_string(),
        permissioned_as: username_to_permissioned_as(&authed.username),
        permissioned_as_email: authed.email.clone(),
        lang: preview.language,
        job_dir: "".to_string(),
        worker_name: "".to_string(),
        worker_dir: "".to_string(),
        client: AuthedClient {
            base_internal_url: utils.base_internal_url.clone(),
            force_client: None,
            token,
            workspace: w_id,
        },
        conn: windmill_common::worker::Connection::Sql(db),
    })
    .await?;
    Ok(Json(to_raw_value(&result)).into_response())
}

#[cfg(not(feature = "run_inline"))]
async fn run_inline_preview_script() -> error::Result<Response> {
    Err(error::Error::InternalErr(
        "inline preview requires the worker feature".to_string(),
    ))
}

#[cfg(feature = "run_inline")]
async fn run_inline_script_by_path(
    OptJobAuthed { authed, .. }: OptJobAuthed,
    Tokened { token }: Tokened,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_path)): Path<(String, StripPath)>,
    Json(body): Json<InlineScriptArgs>,
) -> error::Result<Response> {
    let script_path_str = script_path.to_path();
    check_scopes(&authed, || format!("jobs:run:scripts:{script_path_str}"))?;
    run_inline_script_inner(
        authed,
        token,
        db,
        w_id,
        InlineScriptTarget::Path(script_path.to_path().to_string()),
        body.args,
        Some(user_db),
    )
    .await
}

#[cfg(not(feature = "run_inline"))]
async fn run_inline_script_by_path() -> error::Result<Response> {
    Err(error::Error::InternalErr(
        "inline script by path requires the worker feature".to_string(),
    ))
}

#[cfg(feature = "run_inline")]
async fn run_inline_script_by_hash(
    OptJobAuthed { authed, .. }: OptJobAuthed,
    Tokened { token }: Tokened,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_hash)): Path<(String, ScriptHash)>,
    Json(body): Json<InlineScriptArgs>,
) -> error::Result<Response> {
    // Resolve the script path from the hash and check scopes properly
    let hash = script_hash.0;
    let userdb_authed = UserDbWithAuthed { db: user_db.clone(), authed: &authed.to_authed_ref() };
    let ScriptHashInfo { path, .. } =
        get_script_info_for_hash(Some(userdb_authed), &db, &w_id, hash)
            .await?
            .prefetch_cached(&db)
            .await?;

    check_scopes(&authed, || format!("jobs:run:scripts:{path}"))?;

    run_inline_script_inner(
        authed,
        token,
        db,
        w_id,
        InlineScriptTarget::Hash(hash),
        body.args,
        Some(user_db),
    )
    .await
}

#[cfg(not(feature = "run_inline"))]
async fn run_inline_script_by_hash() -> error::Result<Response> {
    Err(error::Error::InternalErr(
        "inline script by hash requires the worker feature".to_string(),
    ))
}

#[cfg(feature = "run_inline")]
async fn run_inline_script_inner(
    authed: ApiAuthed,
    token: String,
    db: DB,
    w_id: String,
    target: InlineScriptTarget,
    args: Option<HashMap<String, Box<JsonRawValue>>>,
    user_db: Option<UserDB>,
) -> error::Result<Response> {
    let utils = get_worker_internal_server_inline_utils()?;
    let authed_owned: windmill_common::db::Authed = authed.clone().into();
    let result = utils.run_inline_script.as_ref()(RunInlineScriptFnParams {
        target,
        args,
        workspace_id: w_id.clone(),
        base_internal_url: utils.base_internal_url.clone(),
        killpill_rx: utils.killpill_rx.resubscribe(),
        created_by: authed.display_username().to_string(),
        permissioned_as: username_to_permissioned_as(&authed.username),
        permissioned_as_email: authed.email.clone(),
        job_dir: "".to_string(),
        worker_name: "".to_string(),
        worker_dir: "".to_string(),
        client: AuthedClient {
            base_internal_url: utils.base_internal_url.clone(),
            force_client: None,
            token,
            workspace: w_id,
        },
        conn: windmill_common::worker::Connection::Sql(db),
        user_db: user_db.map(|udb| (udb, authed_owned)),
    })
    .await?;
    Ok(Json(to_raw_value(&result)).into_response())
}

#[cfg(feature = "run_inline")]
fn register_potential_assets_on_inline_execution(
    job_id: Uuid,
    w_id: &str,
    preview: &PreviewInline,
) {
    let assets = if preview.language == ScriptLang::DuckDb {
        Some(windmill_parser_sql::parse_assets(&preview.content).map(|a| a.assets))
    } else if preview.language == ScriptLang::Postgresql {
        let datatable = preview
            .args
            .as_ref()
            .and_then(|args| args.get("database"))
            .map(|v| v.get().trim_matches('"'))
            .and_then(|dt| dt.strip_prefix("datatable://"));
        if let Some(datatable) = datatable {
            let re = regex::Regex::new(r#"SET search_path TO "([^"]+)";"#).unwrap();
            let (schema, content) = if let Some(captures) = re.captures(&preview.content) {
                let schema = captures.get(1).map(|m| m.as_str().to_string());
                let content = Some(re.replace(&preview.content, "").to_string());
                (schema, content)
            } else {
                (None, None)
            };
            let content = content.as_deref().unwrap_or(&preview.content);
            windmill_parser_sql::parse_wmill_sdk_sql_assets(
                AssetKind::DataTable,
                datatable,
                schema.as_deref(),
                content,
            )
            .transpose()
        } else {
            None
        }
    } else {
        None
    };
    match assets {
        Some(Ok(assets)) => {
            for asset in assets {
                let columns = asset.columns.as_ref().map(|cols| {
                    cols.iter()
                        .map(|(col_name, col_access_type)| {
                            (
                                col_name.clone(),
                                windmill_common::assets::asset_access_type_from_parser(
                                    *col_access_type,
                                ),
                            )
                        })
                        .collect()
                });
                register_runtime_asset(InsertRuntimeAssetParams {
                    access_type: asset
                        .access_type
                        .map(windmill_common::assets::asset_access_type_from_parser),
                    asset_kind: windmill_common::assets::asset_kind_from_parser(asset.kind),
                    asset_path: asset.path,
                    columns,
                    job_id,
                    workspace_id: w_id.to_string(),
                    created_at: None,
                });
            }
        }
        _ => {}
    }
}

async fn run_wait_result_preview_script(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(run_query): Query<RunJobQuery>,
    Json(preview): Json<Preview>,
) -> error::Result<Response> {
    let (_status_code, uuid) = run_preview_script(
        authed.clone(),
        Extension(db.clone()),
        Extension(user_db.clone()),
        Path(w_id.clone()),
        Query(run_query.clone()),
        Json(preview),
    )
    .await?;
    let uuid = uuid
        .parse::<Uuid>()
        .map_err(|_| Error::BadRequest("Invalid UUID".to_string()))?;
    let result = run_wait_result(&db, uuid, &w_id, None, &authed.username).await;
    return result;
}

async fn run_bundle_preview_script(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(run_query): Query<RunJobQuery>,
    mut multipart: axum::extract::Multipart,
) -> error::Result<(StatusCode, String)> {
    if authed.is_operator {
        return Err(error::Error::NotAuthorized(
            "Operators cannot run preview jobs for security reasons".to_string(),
        ));
    }

    let mut job_id = None;
    let mut tx = None;
    let mut uploaded = false;
    let mut is_tar = false;
    let mut format = BundleFormat::Cjs;

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await;
        let data = data.map_err(to_anyhow)?;
        if name == "preview" {
            let preview: Preview = serde_json::from_slice(&data).map_err(to_anyhow)?;
            require_path_read_access_for_preview(&authed, &preview.path)?;
            format = preview
                .format
                .and_then(|s| BundleFormat::from_string(&s))
                .unwrap_or(BundleFormat::Cjs);

            let scheduled_for = run_query.get_scheduled_for(&db).await?;
            let tag = run_query.tag.clone().or(preview.tag.clone());
            check_tag_available_for_workspace(&db, &w_id, &tag, &authed).await?;
            let ltx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into());

            let args = preview.args.unwrap_or_default();

            is_tar = match preview.kind {
                Some(PreviewKind::Tarbundle) => true,
                _ => false,
            };

            // tracing::info!("is_tar 1: {is_tar}");
            // hmap.insert("")
            let (uuid, ntx) = push(
                &db,
                ltx,
                &w_id,
                JobPayload::Code(RawCode {
                    hash: Some(windmill_common::scripts::codebase_to_hash(
                        is_tar,
                        format == BundleFormat::Esm,
                    )),
                    content: preview.content.unwrap_or_default(),
                    path: preview.path,
                    language: preview.language.unwrap_or(ScriptLang::Deno),
                    lock: preview.lock,
                    cache_ttl: None,
                    cache_ignore_s3_path: None,
                    dedicated_worker: preview.dedicated_worker,
                    concurrency_settings: ConcurrencySettingsWithCustom::default(),
                    debouncing_settings: DebouncingSettings::default(),
                }),
                PushArgs::from(&args),
                authed.display_username(),
                &authed.email,
                username_to_permissioned_as(&authed.username),
                authed.token_prefix.as_deref(),
                scheduled_for,
                None,
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
                Some(&authed.clone().into()),
                false,
                None,
                None,
                None,
            )
            .await?;
            job_id = Some(uuid);
            tx = Some(ntx);
        }
        if name == "file" {
            let mut id = job_id
                .as_ref()
                .ok_or_else(|| {
                    Error::BadRequest(
                        "script need to be passed first in the multipart upload".to_string(),
                    )
                })?
                .to_string();

            // tracing::info!("is_tar 2: {is_tar}");

            if format == BundleFormat::Esm {
                id = format!("{}.esm", id);
            }
            if is_tar {
                id = format!("{}.tar", id);
            }

            uploaded = true;

            let path = windmill_object_store::bundle(&w_id, &id);
            upload_artifact_to_store(
                &path,
                data,
                &windmill_common::worker::ROOT_STANDALONE_BUNDLE_DIR,
            )
            .await?;
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

#[derive(Deserialize, Debug)]
pub struct RunDependenciesRequest {
    pub raw_scripts: Vec<RawScriptForDependencies>,
    pub entrypoint: String,
    #[serde(default)]
    pub raw_workspace_dependencies: Option<RawWorkspaceDependencies>,
    #[serde(default)]
    pub raw_deps: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RawScriptForDependencies {
    pub script_path: String,
    pub raw_code: Option<String>,
    pub language: ScriptLang,
}

#[derive(Serialize)]
pub struct RunDependenciesResponse {
    pub dependencies: String,
}

async fn run_dependencies_job(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<RunDependenciesRequest>,
) -> error::Result<Response> {
    check_scopes(&authed, || format!("jobs:run"))?;
    if authed.is_operator {
        return Err(error::Error::NotAuthorized(
            "Operators cannot run dependencies jobs for security reasons".to_string(),
        ));
    }

    if req.raw_deps.is_some() {
        return Err(error::Error::MigrationNeeded {
            feature: "cli is outdated".into(),
            version: MIN_VERSION_WORKSPACE_DEPENDENCIES.to_owned(),
            guide_url: Url::from_str(
                "https://www.windmill.dev/docs/core_concepts/workspace_dependencies/migration",
            )?,
        });
    }

    // Check if workers support workspace dependencies feature
    if req.raw_workspace_dependencies.is_some() {
        windmill_common::workspace_dependencies::min_version_supports_v0_workspace_dependencies()
            .await?;
    }

    if req.raw_scripts.len() != 1 || req.raw_scripts[0].script_path != req.entrypoint {
        return Err(error::Error::internal_err(
                "For now only a single raw script can be passed to this endpoint, and the entrypoint should be set to the script path".to_string(),
            ));
    }

    let RawScriptForDependencies {
        // unwrap
        script_path,
        raw_code,
        language,
    } = req.raw_scripts[0].clone();

    let mut hm = HashMap::new();
    req.raw_workspace_dependencies
        .map(|v| hm.insert("raw_workspace_dependencies".to_owned(), to_raw_value(&v)));

    let (uuid, tx) = push(
        &db,
        PushIsolationLevel::IsolatedRoot(db.clone()),
        &w_id,
        JobPayload::RawScriptDependencies {
            script_path,
            content: raw_code.unwrap_or_default(),
            language,
        },
        PushArgs { extra: Some(hm), args: &HashMap::new() },
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        authed.token_prefix.as_deref(),
        None,
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
        Some(&authed.clone().into()),
        false,
        None,
        None,
        None,
    )
    .await?;
    tx.commit().await?;

    let wait_result = run_wait_result(&db, uuid, &w_id, None, &authed.username).await;
    wait_result
}

#[derive(Deserialize)]
pub struct RunFlowDependenciesRequest {
    pub path: String,
    pub flow_value: FlowValue,
    #[serde(default)]
    pub raw_workspace_dependencies: Option<RawWorkspaceDependencies>,
    #[serde(default)]
    pub raw_deps: Option<HashMap<String, String>>,
}

#[derive(Serialize)]
pub struct RunFlowDependenciesResponse {
    pub dependencies: String,
}

async fn run_flow_dependencies_job(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<RunFlowDependenciesRequest>,
) -> error::Result<Response> {
    check_scopes(&authed, || format!("jobs:run"))?;
    if authed.is_operator {
        return Err(error::Error::NotAuthorized(
            "Operators cannot run dependencies jobs for security reasons".to_string(),
        ));
    }

    if req.raw_deps.is_some() {
        return Err(error::Error::MigrationNeeded {
            feature: "cli is outdated".into(),
            version: MIN_VERSION_WORKSPACE_DEPENDENCIES.to_owned(),
            guide_url: Url::from_str(
                "https://www.windmill.dev/docs/core_concepts/workspace_dependencies/migration",
            )?,
        });
    }

    // Check if workers support workspace dependencies feature
    if req.raw_workspace_dependencies.is_some() {
        windmill_common::workspace_dependencies::min_version_supports_v0_workspace_dependencies()
            .await?;
    }

    // Create args HashMap with skip_flow_update and raw_workspace_dependencies if present
    let mut args_map = HashMap::from([("skip_flow_update".to_string(), to_raw_value(&true))]);

    // Add raw_workspace_dependencies to args if present
    req.raw_workspace_dependencies
        .map(|v| args_map.insert("raw_workspace_dependencies".to_string(), to_raw_value(&v)));

    let (uuid, tx) = push(
        &db,
        PushIsolationLevel::IsolatedRoot(db.clone()),
        &w_id,
        JobPayload::RawFlowDependencies { path: req.path, flow_value: req.flow_value },
        PushArgs::from(&args_map),
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        authed.token_prefix.as_deref(),
        None,
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
        Some(&authed.clone().into()),
        false,
        None,
        None,
        None,
    )
    .await?;
    tx.commit().await?;

    let wait_result = run_wait_result(&db, uuid, &w_id, None, &authed.username).await;
    wait_result
}

#[derive(Deserialize)]
struct BatchRawScript {
    content: String,
    language: Option<ScriptLang>,
    lock: Option<String>,
}

#[derive(Deserialize)]
struct BatchInfo {
    kind: String,
    flow_value: Option<FlowValue>,
    path: Option<String>,
    rawscript: Option<BatchRawScript>,
    tag: Option<String>,
}

#[tracing::instrument(level = "trace", skip_all)]
async fn add_batch_jobs(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
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
        ScriptRunnableSettingsInline {
            concurrency_settings:
                ConcurrencySettings {
                    concurrent_limit,
                    concurrency_time_window_s,
                    concurrency_key: custom_concurrency_key,
                },
            ..
        },
        timeout,
        raw_code,
        raw_lock,
        raw_flow,
        flow_status,
    ) = match batch_info.kind.as_str() {
        "script" => {
            if let Some(path) = batch_info.path {
                let db_authed =
                    UserDbWithAuthed { db: user_db.clone(), authed: &authed.to_authed_ref() };
                let ScriptHashInfo {
                        hash: script_hash,
                        language,
                        dedicated_worker,
                        timeout,
                        runnable_settings,
                        .. // TODO: consider on_behalf_of_email and created_by for batch jobs
                    } = get_latest_deployed_hash_for_path(Some(db_authed), db.clone(), &w_id, &path)
                    .await?
                    .prefetch_cached(&db)
                    .await?;
                (
                    Some(script_hash),
                    Some(path),
                    JobKind::Script,
                    Some(language),
                    dedicated_worker,
                    runnable_settings,
                    timeout,
                    None,
                    None,
                    None,
                    None,
                )
            } else {
                Err(anyhow::anyhow!(
                    "Path is required if no value is not provided"
                ))?
            }
        }
        "rawscript" => {
            if let Some(rawscript) = batch_info.rawscript {
                (
                    None,
                    None,
                    JobKind::Preview,
                    rawscript.language,
                    None,
                    Default::default(),
                    None,
                    Some(rawscript.content),
                    rawscript.lock,
                    None,
                    None,
                )
            } else {
                Err(anyhow::anyhow!(
                    "rawscript is required for `rawscript` kind"
                ))?
            }
        }
        "flow" => {
            let (mut value, job_kind, path) = if let Some(value) = batch_info.flow_value {
                (value, JobKind::FlowPreview, None)
            } else if let Some(path) = batch_info.path {
                let mut tx = user_db.clone().begin(&authed).await?;
                let value_json = sqlx::query!(
                        "SELECT coalesce(flow_version_lite.value, flow_version.value) as \"value!: sqlx::types::Json<Box<RawValue>>\" FROM flow
                        LEFT JOIN flow_version
                            ON flow_version.id = flow.versions[array_upper(flow.versions, 1)]
                        LEFT JOIN flow_version_lite
                            ON flow_version_lite.id = flow_version.id
                        WHERE flow.path = $1 AND flow.workspace_id = $2 LIMIT 1",
                        &path, &w_id
                    )
                    .fetch_optional(&mut *tx)
                    .await?
                    .ok_or_else(|| Error::internal_err(format!("not found flow at path {:?}", path)))?;
                let value =
                    serde_json::from_str::<FlowValue>(value_json.value.get()).map_err(|err| {
                        Error::internal_err(format!(
                            "could not convert json to flow for {path}: {err:?}"
                        ))
                    })?;
                (value, JobKind::Flow, Some(path))
            } else {
                Err(anyhow::anyhow!(
                    "Path is required if no value is not provided"
                ))?
            };
            add_virtual_items_if_necessary(&mut value.modules);
            let flow_status = FlowStatus::new(&value);
            (
                None,     // script_hash
                path,     // script_path
                job_kind, // job_kind
                None,     // language
                None,     // dedicated_worker
                ScriptRunnableSettingsInline {
                    concurrency_settings: value.concurrency_settings.clone(),
                    ..Default::default()
                },
                None,              // timeout
                None,              // raw_code
                None,              // raw_lock
                Some(value),       // raw_flow
                Some(flow_status), // flow_status
            )
        }
        "noop" => (
            None,
            None,
            JobKind::Noop,
            None,
            None,
            Default::default(),
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
            windmill_common::worker::dedicated_worker_tag(&w_id, &path.clone().unwrap())
        } else {
            format!("{}", language.as_str())
        }
    } else if let Some(tag) = batch_info.tag {
        tag
    } else {
        format!("{}", language.as_str())
    };

    let mut tx = user_db.begin(&authed).await?;

    let uuids = sqlx::query_scalar!(
            r#"WITH uuid_table as (
                select gen_random_uuid() as uuid from generate_series(1, $16)
            )
            INSERT INTO v2_job
                (id, workspace_id, raw_code, raw_lock, raw_flow, tag, runnable_id, runnable_path, kind,
                script_lang, created_by, permissioned_as, permissioned_as_email, concurrent_limit,
                concurrency_time_window_s, timeout, args)
                (SELECT uuid, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                ('{ "uuid": "' || uuid || '" }')::jsonb FROM uuid_table)
            RETURNING id AS "id!""#,
            w_id,
            raw_code,
            raw_lock,
            raw_flow.map(sqlx::types::Json) as Option<sqlx::types::Json<FlowValue>>,
            tag,
            hash,
            path,
            job_kind.clone() as JobKind,
            language as ScriptLang,
            authed.username,
            username_to_permissioned_as(&authed.username),
            authed.email,
            concurrent_limit,
            concurrency_time_window_s,
            timeout,
            n,
        )
        .fetch_all(&mut *tx)
        .await?;

    let uuids = sqlx::query_scalar!(
        r#"WITH uuid_table as (
                select unnest($4::uuid[]) as uuid
            )
            INSERT INTO v2_job_queue
                (id, workspace_id, scheduled_for, tag)
                (SELECT uuid, $1, $2, $3 FROM uuid_table)
            RETURNING id"#,
        w_id,
        Utc::now(),
        tag,
        &uuids
    )
    .fetch_all(&mut *tx)
    .await?;

    sqlx::query!(
        "INSERT INTO v2_job_runtime (id, ping) SELECT unnest($1::uuid[]), null",
        &uuids,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
            "INSERT INTO job_perms (job_id, email, username, is_admin, is_operator, folders, groups, workspace_id)
            SELECT unnest($1::uuid[]), $2, $3, $4, $5, $6, $7, $8",
            &uuids,
            authed.email,
            authed.username,
            authed.is_admin,
            authed.is_operator,
            &[],
            &[],
            w_id,
        )
        .execute(&mut *tx)
        .await?;

    if let Some(flow_status) = flow_status {
        sqlx::query!(
            "INSERT INTO v2_job_status (id, flow_status)
                SELECT unnest($1::uuid[]), $2",
            &uuids,
            sqlx::types::Json(flow_status) as sqlx::types::Json<FlowStatus>
        )
        .execute(&mut *tx)
        .await?;
    }

    if let Some(custom_concurrency_key) = custom_concurrency_key {
        sqlx::query!(
            "INSERT INTO concurrency_counter(concurrency_id, job_uuids)
                VALUES ($1, '{}'::jsonb)",
            &custom_concurrency_key
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "INSERT INTO concurrency_key (job_id, key) SELECT id, $1 FROM unnest($2::uuid[]) as id",
            custom_concurrency_key,
            &uuids
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Json(uuids))
}

async fn run_preview_flow_job(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(run_query): Query<RunJobQuery>,
    Json(raw_flow): Json<PreviewFlow>,
) -> error::Result<(StatusCode, String)> {
    if authed.is_operator {
        return Err(error::Error::NotAuthorized(
            "Operators cannot run preview jobs for security reasons".to_string(),
        ));
    }
    require_path_read_access_for_preview(&authed, &raw_flow.path)?;
    let scheduled_for = run_query.get_scheduled_for(&db).await?;
    let tag = run_query.tag.clone().or(raw_flow.tag.clone());
    check_tag_available_for_workspace(&db, &w_id, &tag, &authed).await?;
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into());

    let chat_input_enabled = raw_flow.value.chat_input_enabled.unwrap_or(false);
    let flow_path = raw_flow.path.clone().unwrap_or_default();
    let user_message = raw_flow
        .args
        .as_ref()
        .and_then(|args| args.get("user_message"))
        .cloned();

    let (uuid, mut tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::RawFlow {
            value: raw_flow.value,
            path: raw_flow.path,
            restarted_from: raw_flow.restarted_from,
        },
        PushArgs::from(&raw_flow.args.unwrap_or_default()),
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        authed.token_prefix.as_deref(),
        scheduled_for,
        None,
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
        Some(&authed.clone().into()),
        false,
        None,
        None,
        None,
    )
    .await?;

    // Set memory_id if provided (for agent memory)
    if let Some(memory_id) = run_query.memory_id {
        set_flow_memory_id(&mut tx, uuid, memory_id).await?;
    }

    // Handle conversation messages for chat-enabled flows
    if chat_input_enabled {
        handle_chat_conversation_messages(
            &mut tx,
            &authed,
            &w_id,
            &flow_path,
            &run_query,
            user_message.as_ref(),
        )
        .await?;
    }

    tx.commit().await?;

    Ok((StatusCode::CREATED, uuid.to_string()))
}

async fn run_wait_result_preview_flow(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(run_query): Query<RunJobQuery>,
    Json(raw_flow): Json<PreviewFlow>,
) -> error::Result<Response> {
    let (_status_code, uuid) = run_preview_flow_job(
        authed.clone(),
        Extension(db.clone()),
        Extension(user_db.clone()),
        Path(w_id.clone()),
        Query(run_query.clone()),
        Json(raw_flow),
    )
    .await?;
    let uuid = uuid
        .parse::<Uuid>()
        .map_err(|_| Error::BadRequest("Invalid UUID".to_string()))?;
    let result = run_wait_result(&db, uuid, &w_id, None, &authed.username).await;
    return result;
}

async fn run_dynamic_select(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(run_query): Query<RunJobQuery>,
    Json(request): Json<DynamicSelectRequest>,
) -> error::Result<Response> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    if matches!(
        request.runnable_ref,
        DynamicSelectRunnableRef::Inline { .. }
    ) && authed.is_operator
    {
        return Err(error::Error::NotAuthorized(
            "Operators cannot run preview jobs for security reasons".to_string(),
        ));
    }

    let dynamic_input: DynamicInput;

    match request.runnable_ref {
        DynamicSelectRunnableRef::Deployed { path, runnable_kind } => match runnable_kind {
            RunnableKind::Script => {
                let mut script_args = request.args.unwrap_or_default();
                script_args.insert(
                    "_ENTRYPOINT_OVERRIDE".to_string(),
                    serde_json::value::to_raw_value(&request.entrypoint_function)?,
                );

                let push_args = PushArgsOwned { extra: None, args: script_args.clone() };

                let (uuid, _, _) = push_script_job_by_path_into_queue(
                    authed.clone(),
                    db.clone(),
                    None,
                    user_db.clone(),
                    w_id.clone(),
                    StripPath(path),
                    run_query.clone(),
                    push_args.clone(),
                    None,
                )
                .await?;

                return Ok((StatusCode::CREATED, uuid.to_string()).into_response());
            }
            RunnableKind::Flow => {
                let mut conn = user_db.clone().begin(&authed).await?;

                let dynamic_input_res = match DYNAMIC_INPUT_CACHE.get(&format!("{}:{}", w_id, path))
                {
                    Some(cached) => cached.as_ref().clone(),
                    None => {
                        let dynamic_input = sqlx::query_scalar!(
                            r#"
                            SELECT
                                schema
                            FROM
                                flow
                            WHERE
                                workspace_id = $1 AND
                                path = $2
                        "#,
                            &w_id,
                            &path
                        )
                        .fetch_one(&mut *conn)
                        .await?
                        .and_then(|dynamic_input| {
                            Some(serde_json::from_value::<DynamicInput>(dynamic_input))
                        })
                        .transpose()?;

                        let Some(dynamic_input) = dynamic_input else {
                            return Err(Error::BadRequest(format!(
                                "Flow at path {} does not have a dynamic select schema",
                                path
                            )));
                        };

                        let dynamic_input_key =
                            windmill_common::jobs::generate_dynamic_input_key(&w_id, &path);
                        DYNAMIC_INPUT_CACHE
                            .insert(dynamic_input_key, Arc::new(dynamic_input.clone()));
                        dynamic_input
                    }
                };

                conn.commit().await?;

                dynamic_input = dynamic_input_res;
            }
        },
        DynamicSelectRunnableRef::Inline { code, lang: language } => {
            dynamic_input = DynamicInput {
                x_windmill_dyn_select_code: code,
                x_windmill_dyn_select_lang: language.unwrap_or_default(),
            };
        }
    }

    let scheduled_for = run_query.get_scheduled_for(&db).await?;
    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into());

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::Code(RawCode {
            hash: None,
            content: dynamic_input.x_windmill_dyn_select_code,
            path: None,
            language: dynamic_input.x_windmill_dyn_select_lang,
            lock: None,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            concurrency_settings: ConcurrencySettings::default().into(),
            debouncing_settings: DebouncingSettings::default(),
        }),
        PushArgs::from(&request.args.unwrap_or_default()),
        authed.display_username(),
        &authed.email,
        username_to_permissioned_as(&authed.username),
        authed.token_prefix.as_deref(),
        scheduled_for,
        None,
        None,
        None,
        None,
        run_query.job_id,
        false,
        false,
        None,
        true,
        None,
        run_query.timeout,
        None,
        None,
        Some(&authed.clone().into()),
        false,
        None,
        None,
        None,
    )
    .await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, uuid.to_string()).into_response())
}

pub async fn run_job_by_hash(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, script_hash)): Path<(String, ScriptHash)>,
    Query(run_query): Query<RunJobQuery>,
    args: RawWebhookArgs,
) -> error::Result<(StatusCode, String)> {
    let args = args
        .to_args_from_runnable(
            &authed,
            &db,
            &w_id,
            RunnableId::from_script_hash(script_hash),
            run_query.skip_preprocessor,
        )
        .await?;

    let (uuid, _) = run_job_by_hash_inner(
        authed,
        db,
        user_db,
        w_id,
        script_hash,
        run_query,
        args,
        None,
    )
    .await?;

    Ok((StatusCode::CREATED, uuid.to_string()))
}

pub async fn run_job_by_hash_inner(
    authed: ApiAuthed,
    db: DB,
    user_db: UserDB,
    w_id: String,
    script_hash: ScriptHash,
    run_query: RunJobQuery,
    args: PushArgsOwned,
    trigger: Option<TriggerMetadata>,
) -> error::Result<(Uuid, Option<bool>)> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    let hash = script_hash.0;
    let userdb_authed = UserDbWithAuthed { db: user_db.clone(), authed: &authed.to_authed_ref() };
    let ScriptHashInfo {
        path,
        tag,
        runnable_settings:
            ScriptRunnableSettingsInline { concurrency_settings, debouncing_settings },
        mut cache_ttl,
        mut cache_ignore_s3_path,
        language,
        dedicated_worker,
        priority,
        timeout,
        has_preprocessor,
        on_behalf_of_email,
        created_by,
        delete_after_use,
        ..
    } = get_script_info_for_hash(Some(userdb_authed), &db, &w_id, hash)
        .await?
        .prefetch_cached(&db)
        .await?;

    check_scopes(&authed, || format!("jobs:run:scripts:{path}"))?;
    if let Some(run_query_cache_ttl) = run_query.cache_ttl {
        cache_ttl = Some(run_query_cache_ttl);
        cache_ignore_s3_path = run_query.cache_ignore_s3_path;
    }
    let scheduled_for = run_query.get_scheduled_for(&db).await?;
    let tag = run_query.tag.clone().or(tag);

    check_tag_available_for_workspace(&db, &w_id, &tag, &authed).await?;

    let (email, permissioned_as, push_authed, tx) = if let Some(email) = on_behalf_of_email.as_ref()
    {
        (
            email,
            username_to_permissioned_as(created_by.as_str()),
            None,
            PushIsolationLevel::IsolatedRoot(db.clone()),
        )
    } else {
        (
            &authed.email,
            username_to_permissioned_as(&authed.username),
            Some(authed.clone().into()),
            PushIsolationLevel::Isolated(user_db, authed.clone().into()),
        )
    };

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::ScriptHash {
            hash: ScriptHash(hash),
            path: path,
            concurrency_settings,
            debouncing_settings,
            cache_ttl,
            cache_ignore_s3_path,
            language,
            dedicated_worker,
            priority,
            apply_preprocessor: !run_query.skip_preprocessor.unwrap_or(false)
                && has_preprocessor.unwrap_or(false),
        },
        PushArgs { args: &args.args, extra: args.extra },
        authed.display_username(),
        email,
        permissioned_as,
        authed.token_prefix.as_deref(),
        scheduled_for,
        None,
        run_query.parent_job,
        None,
        run_query.root_job,
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
        timeout,
        None,
        None,
        push_authed.as_ref(),
        false,
        None,
        trigger,
        run_query.suspended_mode,
    )
    .await?;
    tx.commit().await?;

    Ok((uuid, delete_after_use))
}

async fn get_log_file(Path((_w_id, file_p)): Path<(String, String)>) -> error::Result<Response> {
    if file_p.contains("..") {
        return Err(error::Error::BadRequest("Invalid path".to_string()));
    }

    // Validate path format: must be exactly 2 parts, first is UUID, second ends with .txt
    let parts: Vec<&str> = file_p.split('/').collect();
    if parts.len() != 2 {
        return Err(error::Error::BadRequest(
            "Invalid path: must have exactly 2 components".to_string(),
        ));
    }
    if Uuid::parse_str(parts[0]).is_err() {
        return Err(error::Error::BadRequest(
            "Invalid path: first component must be a valid UUID".to_string(),
        ));
    }
    if !parts[1].ends_with(".txt") {
        return Err(error::Error::BadRequest(
            "Invalid path: file must end with .txt".to_string(),
        ));
    }

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
    if let Some(os) = windmill_object_store::get_object_store().await {
        let file = os
            .get(&windmill_object_store::object_store_reexports::Path::from(
                format!("logs/{file_p}"),
            ))
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
                return Err(error::Error::internal_err(format!(
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
        return Err(error::Error::internal_err(format!(
                "Object store client not present and file not found on server logs volume at {local_file}"
            )));
    }

    #[cfg(not(all(feature = "enterprise", feature = "parquet")))]
        return Err(error::Error::NotFound(format!(
            "File not found on server logs volume /tmp/windmill/logs and no distributed logs s3 storage for {}",
            file_p
        )));
}

async fn get_job_update(
    OptAuthed(opt_authed): OptAuthed,
    opt_tokened: OptTokened,
    Extension(db): Extension<DB>,
    Path((w_id, job_id)): Path<(String, Uuid)>,
    Query(JobUpdateQuery {
        log_offset,
        stream_offset,
        get_progress,
        running,
        only_result,
        no_logs,
        is_flow,
        ..
    }): Query<JobUpdateQuery>,
) -> JsonResult<JobUpdate> {
    Ok(Json(
        get_job_update_data(
            &opt_authed,
            &opt_tokened,
            &db,
            &w_id,
            &job_id,
            log_offset,
            stream_offset,
            get_progress.unwrap_or(false),
            running,
            true,
            false,
            only_result,
            no_logs,
            is_flow,
            None,
            None,
        )
        .await?,
    ))
}

async fn get_job_update_sse(
    OptAuthed(opt_authed): OptAuthed,
    opt_tokened: OptTokened,
    Extension(db): Extension<DB>,
    Path((w_id, job_id)): Path<(String, Uuid)>,
    Query(JobUpdateQuery {
        log_offset,
        stream_offset,
        get_progress,
        running,
        no_logs,
        only_result,
        fast,
        is_flow,
        poll_delay_ms,
    }): Query<JobUpdateQuery>,
) -> error::Result<Response> {
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    start_job_update_sse_stream(
        opt_authed,
        opt_tokened,
        db,
        w_id,
        job_id,
        log_offset,
        stream_offset,
        get_progress,
        running,
        only_result,
        fast,
        no_logs,
        is_flow,
        tx,
        poll_delay_ms,
        None,
    );

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(|x| {
        format!(
            "data: {}\n\n",
            serde_json::to_string(&x).unwrap_or_default()
        )
    });

    let body = axum::body::Body::from_stream(stream.map(Result::<_, std::convert::Infallible>::Ok));

    Ok(Response::builder()
        .status(200)
        .header("X-Accel-Buffering", "no")
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(body)
        .unwrap())
}

pub fn start_job_update_sse_stream(
    opt_authed: Option<ApiAuthed>,
    opt_tokened: OptTokened,
    db: DB,
    w_id: String,
    job_id: Uuid,
    initial_log_offset: Option<i32>,
    initial_stream_offset: Option<i32>,
    get_progress: Option<bool>,
    running: Option<bool>,
    only_result: Option<bool>,
    fast: Option<bool>,
    no_logs: Option<bool>,
    is_flow: Option<bool>,
    tx: tokio::sync::mpsc::Sender<JobUpdateSSEStream>,
    poll_delay_ms: Option<u64>,
    early_return: Option<String>,
) -> () {
    tokio::spawn(async move {
        let mut log_offset = initial_log_offset;
        let mut stream_offset = initial_stream_offset;
        let mut last_update_hash: Option<String> = None;
        let mut flow_stream_job_id = None;

        // Send initial update immediately
        let mut running = running;
        let mut mem_peak = 0;

        match get_job_update_data(
            &opt_authed,
            &opt_tokened,
            &db,
            &w_id,
            &job_id,
            log_offset,
            stream_offset,
            false,
            running,
            true,
            true,
            only_result,
            no_logs,
            is_flow,
            flow_stream_job_id,
            early_return.as_deref(),
        )
        .await
        {
            Ok(mut update) => {
                last_update_hash = Some(update.hash_str());
                let completion_sent = update.completed.unwrap_or(false);
                if running.is_some() && update.running.is_some_and(|x| x) {
                    running = Some(true);
                }
                if let Some(new_mem_peak) = update.mem_peak {
                    mem_peak = new_mem_peak;
                }
                if let Some(new_offset) = update.log_offset {
                    if new_offset != log_offset.unwrap_or(0) {
                        log_offset = Some(new_offset);
                    } else {
                        update.log_offset = None;
                    }
                }
                if let Some(new_stream_offset) = update.stream_offset {
                    if new_stream_offset != stream_offset.unwrap_or(0) {
                        stream_offset = Some(new_stream_offset);
                    } else {
                        update.stream_offset = None;
                    }
                }
                if update.flow_stream_job_id.is_some() {
                    flow_stream_job_id = update.flow_stream_job_id;
                }
                if tx.send(JobUpdateSSEStream::Update(update)).await.is_err() {
                    tracing::warn!("Failed to send initial job update for job {job_id}");
                    return;
                }
                if completion_sent {
                    return;
                }
            }
            Err(e) => {
                if tx
                    .send(JobUpdateSSEStream::Error { error: e.to_string() })
                    .await
                    .is_err()
                {
                    tracing::warn!("Failed to send initial job update for job {job_id}");
                    return;
                }
            }
        }

        let mut get_progress_m: bool = false;
        // Poll for updates every 1 second
        let mut i = 0;
        let start = Instant::now();
        let mut last_ping = Instant::now();
        let mut last_progress_check = Instant::now();
        loop {
            i += 1;

            #[allow(unused_mut)]
            let mut ms_duration = if i > 100 || !fast.unwrap_or(false) {
                3000
            } else if i > 10 {
                500
            } else {
                100
            };

            #[allow(unused_variables)]
            if let Some(poll_delay_ms) = poll_delay_ms {
                #[cfg(feature = "enterprise")]
                if poll_delay_ms < 50 {
                    tracing::warn!("Poll delay ms is less than 50, setting it to 50");
                    ms_duration = 50;
                } else {
                    ms_duration = poll_delay_ms;
                }

                #[cfg(not(feature = "enterprise"))]
                tracing::warn!("Settable poll delay requires EE");
            }

            if last_ping.elapsed().as_secs() > 5 {
                if tx.send(JobUpdateSSEStream::Ping).await.is_err() {
                    tracing::warn!("Failed to send job ping for job {job_id}");
                    return;
                }
                last_ping = Instant::now();
            }

            if start.elapsed().as_secs() > *TIMEOUT_SSE_STREAM {
                if tx.send(JobUpdateSSEStream::Timeout).await.is_err() {
                    tracing::warn!("Failed to send job timeout for job {job_id}");
                }
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(ms_duration)).await;

            // Check progress if the user requested it, and check periodically if the job has progress
            // Once it has progress, we always check progress
            let check_progress = get_progress.unwrap_or(false)
                && (get_progress_m || last_progress_check.elapsed().as_secs() > 5);
            match get_job_update_data(
                &opt_authed,
                &opt_tokened,
                &db,
                &w_id,
                &job_id,
                log_offset,
                stream_offset,
                check_progress,
                running,
                false,
                true,
                only_result,
                no_logs,
                is_flow,
                flow_stream_job_id,
                early_return.as_deref(),
            )
            .await
            {
                Ok(mut update) => {
                    if running.is_some() && update.running.is_some_and(|x| x) {
                        running = Some(true);
                    }
                    if update.completed.is_some_and(|x| !x) {
                        update.completed = None;
                    }
                    if update.new_logs.as_ref().is_some_and(|x| x.is_empty()) {
                        update.new_logs = None;
                    }
                    if check_progress {
                        if update.progress.is_some() {
                            get_progress_m = true;
                        } else {
                            last_progress_check = Instant::now();
                        }
                    }

                    // if !only_result.unwrap_or(false) {
                    //     tracing::error!("update {:?}", update);
                    // }
                    let update_last_status = update.hash_str();
                    // Only send if the update has changed
                    if last_update_hash.as_ref() != Some(&update_last_status) {
                        // Update log offset if available
                        if let Some(new_offset) = update.log_offset {
                            if new_offset != log_offset.unwrap_or(0) {
                                // let logs = update.new_logs.clone().unwrap_or_default();
                                // tracing::error!(
                                //     "new_offset: {new_offset}; log_offset: {log_offset:?}; {} {logs}",
                                //     logs.len()
                                // );
                                log_offset = Some(new_offset);
                            } else {
                                update.log_offset = None;
                            }
                        }

                        if let Some(new_stream_offset) = update.stream_offset {
                            if new_stream_offset != stream_offset.unwrap_or(0) {
                                stream_offset = Some(new_stream_offset);
                            } else {
                                update.stream_offset = None;
                            }
                        }
                        if update.flow_stream_job_id.is_some() {
                            if flow_stream_job_id.is_none() {
                                flow_stream_job_id = update.flow_stream_job_id;
                            } else {
                                update.flow_stream_job_id = None;
                            }
                        }
                        if let Some(new_mem_peak) = update.mem_peak {
                            if new_mem_peak != mem_peak {
                                mem_peak = new_mem_peak;
                            } else {
                                update.mem_peak = None;
                            }
                        }
                        let completed = update.completed.unwrap_or(false);
                        if tx.send(JobUpdateSSEStream::Update(update)).await.is_err() {
                            break;
                        }
                        if completed {
                            break;
                        }

                        last_update_hash = Some(update_last_status);
                    }
                }
                Err(e) => {
                    tracing::error!("Error getting job update: {:?}", e);
                    if tx.send(JobUpdateSSEStream::NotFound).await.is_err() {
                        tracing::warn!("Failed to send job not found for job {job_id}");
                    }
                    return;
                }
            }
        }
    });
}

async fn get_flow_stream_delta(
    db: &DB,
    flow_stream_job_id: Option<Uuid>,
    stream_offset: Option<i32>,
) -> error::Result<Option<(Option<String>, Option<i32>)>> {
    if let Some(job_id) = flow_stream_job_id {
        let record = sqlx::query!(
            "
                SELECT
                    string_agg(stream, '' order by idx asc) as stream,
                    max(idx) + 1 as offset
                FROM job_result_stream_v2
                WHERE job_id = $2 AND idx >= $1
                ",
            stream_offset.unwrap_or(0),
            job_id,
        )
        .fetch_optional(db)
        .await?;
        if let Some(record) = record {
            Ok(Some((record.stream, record.offset)))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

async fn get_job_update_data(
    opt_authed: &Option<ApiAuthed>,
    opt_tokened: &OptTokened,
    db: &DB,
    w_id: &str,
    job_id: &Uuid,
    log_offset: Option<i32>,
    stream_offset: Option<i32>,
    get_progress: bool,
    running: Option<bool>,
    log_view: bool,
    get_full_job_on_completion: bool,
    only_result: Option<bool>,
    no_logs: Option<bool>,
    is_flow: Option<bool>,
    flow_stream_job_id: Option<Uuid>,
    early_return: Option<&str>,
) -> error::Result<JobUpdate> {
    let tags = if log_view {
        log_job_view(
            db,
            opt_authed.as_ref(),
            opt_tokened.token.as_deref(),
            w_id,
            job_id,
        )
        .await?;
        opt_authed
            .as_ref()
            .map(|authed| get_scope_tags(authed))
            .flatten()
    } else {
        None
    };

    let ignore_flow_stream_job_id = is_flow.is_some_and(|x| !x) || flow_stream_job_id.is_some();

    if only_result.unwrap_or(false) {
        let (result, running, mut result_stream, mut new_stream_offset, new_flow_stream_job_id) =
            if let Some(tags) = tags {
                let r = sqlx::query!(
                    "
                    WITH result_stream AS (
                        SELECT
                            string_agg(stream, '' order by idx asc) as stream,
                            job_id,
                            max(idx) + 1 as offset
                        FROM job_result_stream_v2
                        WHERE job_id = $2 AND idx >= $3
                        GROUP BY job_id
                    )
                    SELECT
                        jc.result as \"result: sqlx::types::Json<Box<RawValue>>\",
                        v2_job.tag,
                        v2_job_queue.running as \"running: Option<bool>\",
                        rs.stream AS \"result_stream: Option<String>\",
                        rs.offset AS stream_offset,
                        CASE WHEN $4 THEN NULL ELSE (COALESCE(js.flow_status, jc.flow_status)->>'stream_job')::uuid END as stream_job
                    FROM v2_job
                    LEFT JOIN v2_job_queue USING (id)
                    LEFT JOIN v2_job_completed jc USING (id)
                    LEFT JOIN v2_job_status js USING (id)
                    LEFT JOIN result_stream rs ON rs.job_id = $2
                    WHERE v2_job.id = $2 AND v2_job.workspace_id = $1",
                    w_id,
                    job_id,
                    stream_offset.unwrap_or(0),
                    ignore_flow_stream_job_id,
                )
                .fetch_optional(db)
                .await?
                .ok_or_else(|| Error::NotFound(format!("Job not found: {}", job_id)))?;

                if !tags.contains(&r.tag.as_str()) {
                    return Err(Error::NotAuthorized(format!(
                        "Job tag {} is not in the scope tags: {}",
                        r.tag,
                        tags.join(", ")
                    )));
                }
                let running = r.running.as_ref().map(|x| *x);
                (
                    r.result.map(|x| x.0),
                    running,
                    r.result_stream.flatten(),
                    r.stream_offset,
                    r.stream_job,
                )
            } else {
                if running.is_some_and(|x| !x) {
                    let r = sqlx::query!(
                        "
                        WITH result_stream AS (
                            SELECT
                                string_agg(stream, '' order by idx asc) as stream,
                                job_id,
                                max(idx) + 1 as offset
                            FROM job_result_stream_v2
                            WHERE job_id = $1 AND idx >= $3
                            GROUP BY job_id
                        )
                        SELECT
                            COALESCE(jc.result, jc.result) as \"result: sqlx::types::Json<Box<RawValue>>\",
                            jq.running as \"running: Option<bool>\",
                            rs.stream AS \"result_stream: Option<String>\",
                            rs.offset AS stream_offset,
                            CASE WHEN $4 THEN NULL ELSE (COALESCE(js.flow_status, jc.flow_status)->>'stream_job')::uuid END as stream_job
                        FROM (
                            SELECT $1::uuid as job_id, $2::text as workspace_id
                        ) base
                        LEFT JOIN v2_job_completed jc ON jc.id = base.job_id AND jc.workspace_id = base.workspace_id
                        LEFT JOIN v2_job_queue jq ON jq.id = base.job_id AND jq.workspace_id = base.workspace_id
                        LEFT JOIN v2_job_status js ON js.id = base.job_id
                        LEFT JOIN result_stream rs ON rs.job_id = base.job_id
                        WHERE base.job_id = $1",
                        job_id,
                        w_id,
                        stream_offset.unwrap_or(0),
                        ignore_flow_stream_job_id,
                    ).fetch_optional(db).await?;
                    if let Some(r) = r {
                        let running = r.running.as_ref().map(|x| *x);
                        (
                            r.result.map(|x| x.0),
                            running,
                            r.result_stream.flatten(),
                            r.stream_offset,
                            r.stream_job,
                        )
                    } else {
                        (None, None, None, None, None)
                    }
                } else {
                    let q = sqlx::query!(
                        "
                        WITH result_stream AS (
                            SELECT
                                string_agg(stream, '' order by idx asc) as stream,
                                job_id,
                                max(idx) + 1 as offset
                            FROM job_result_stream_v2
                            WHERE job_id = $2 AND idx >= $3
                            GROUP BY job_id
                        )
                        SELECT
                            COALESCE(jc.result, NULL) as \"result: sqlx::types::Json<Box<RawValue>>\",
                            rs.stream AS \"result_stream: Option<String>\",
                            rs.offset AS stream_offset,
                            COALESCE(js.flow_status, jc.flow_status) as \"flow_status: sqlx::types::Json<Box<RawValue>>\",
                            CASE WHEN $4 THEN NULL ELSE (COALESCE(js.flow_status, jc.flow_status)->>'stream_job')::uuid END as stream_job
                        FROM (
                            SELECT $2::uuid as job_id, $1::text as workspace_id
                        ) base
                        LEFT JOIN v2_job_completed jc ON jc.id = base.job_id AND jc.workspace_id = base.workspace_id
                        LEFT JOIN v2_job_status js ON js.id = base.job_id
                        LEFT JOIN result_stream rs ON rs.job_id = base.job_id
                        WHERE base.job_id = $2",
                        w_id,
                        job_id,
                        stream_offset.unwrap_or(0),
                        ignore_flow_stream_job_id,
                    )
                    .fetch_optional(db)
                    .await?;
                    if let Some(r) = q {
                        (
                            r.result.map(|x| x.0),
                            running,
                            r.result_stream.flatten(),
                            r.stream_offset,
                            r.stream_job,
                        )
                    } else {
                        (None, None, None, None, None)
                    }
                }
            };

        let flow_stream_job_id = flow_stream_job_id.or(new_flow_stream_job_id);

        let result = if let Some(early_return) = early_return {
            match get_result_and_success_by_id_from_flow(db, w_id, job_id, early_return, None).await
            {
                Ok((early_result, _)) => Some(early_result),
                Err(_) => result,
            }
        } else {
            result
        };

        let flow_stream_delta =
            get_flow_stream_delta(db, flow_stream_job_id, stream_offset).await?;

        if let Some((flow_result_stream, flow_stream_offset)) = flow_stream_delta {
            result_stream = flow_result_stream;
            new_stream_offset = flow_stream_offset;
        }

        Ok(JobUpdate {
            running,
            completed: if result.is_some() { Some(true) } else { None },
            log_offset: None,
            new_logs: None,
            new_result_stream: result_stream,
            stream_offset: new_stream_offset,
            mem_peak: None,
            progress: None,
            job: None,
            flow_status: None,
            workflow_as_code_status: None,
            only_result: result,
            flow_stream_job_id,
        })
    } else {
        let mut record = sqlx::query!(
                "
                WITH result_stream AS (
                    SELECT
                        string_agg(stream, '' order by idx asc) as stream,
                        job_id,
                        max(idx) + 1 as offset
                    FROM job_result_stream_v2
                    WHERE job_id = $3 AND idx >= $8
                    GROUP BY job_id
                )
                SELECT
                    c.id IS NOT NULL AS completed,
                    CASE
                        WHEN q.id IS NOT NULL THEN (CASE WHEN NOT $5 AND q.running THEN true ELSE null END)
                        ELSE false
                    END AS running,
                    CASE WHEN $7::BOOLEAN THEN NULL ELSE SUBSTR(logs, GREATEST($1 - log_offset, 0)) END AS logs,
                    rs.stream AS new_result_stream,
                    COALESCE(r.memory_peak, c.memory_peak) AS mem_peak,
                    COALESCE(c.flow_status, f.flow_status) AS \"flow_status: sqlx::types::Json<Box<RawValue>>\",
                    (COALESCE(c.flow_status, f.flow_status)->>'stream_job')::uuid AS stream_job,
                    COALESCE(c.workflow_as_code_status, f.workflow_as_code_status) AS \"workflow_as_code_status: sqlx::types::Json<Box<RawValue>>\",
                    CASE WHEN $7::BOOLEAN THEN NULL ELSE job_logs.log_offset + CHAR_LENGTH(job_logs.logs) + 1 END AS log_offset,
                    rs.offset AS stream_offset,
                    created_by AS \"created_by!\",
                    CASE WHEN $4::BOOLEAN THEN (
                        SELECT scalar_int FROM job_stats WHERE job_id = $3 AND metric_id = 'progress_perc'
                    ) END AS progress,
                    rs.stream AS \"result_stream: Option<String>\"
                FROM v2_job j
                    LEFT JOIN v2_job_queue q USING (id)
                    LEFT JOIN v2_job_runtime r USING (id)
                    LEFT JOIN v2_job_status f USING (id)
                    LEFT JOIN v2_job_completed c USING (id)
                    LEFT JOIN result_stream rs ON rs.job_id = $3
                    LEFT JOIN job_logs ON job_logs.job_id =  $3
                WHERE j.workspace_id = $2 AND j.id = $3
                AND ($6::text[] IS NULL OR j.tag = ANY($6))",
                log_offset,
                w_id,
                job_id,
                get_progress,
                running,
                tags.as_ref().map(|v| v.as_slice()) as Option<&[&str]>,
                no_logs.unwrap_or(false),
                stream_offset.unwrap_or(0),
            )
            .fetch_optional(db)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Job not found: {}", job_id)))?;

        if opt_authed.is_none() && record.created_by != "anonymous" {
            return Err(Error::BadRequest(
                "As a non logged in user, you can only see jobs ran by anonymous users".to_string(),
            ));
        }

        let job = if record.completed.unwrap_or(false) && get_full_job_on_completion {
            let get = GetQuery::new()
                .with_auth(&opt_authed)
                .without_logs()
                .without_code();
            Some(get.fetch(&db, job_id, &w_id).await?)
        } else {
            None
        };

        let flow_stream_job_id = flow_stream_job_id.or(record.stream_job);

        let flow_stream_delta =
            get_flow_stream_delta(db, flow_stream_job_id, stream_offset).await?;

        if let Some((new_result_stream, stream_offset)) = flow_stream_delta {
            record.new_result_stream = new_result_stream;
            record.stream_offset = stream_offset;
        }

        Ok(JobUpdate {
            running: record.running,
            completed: record.completed,
            log_offset: record.log_offset,
            new_logs: record.logs,
            new_result_stream: record.new_result_stream,
            stream_offset: record.stream_offset,
            mem_peak: record.mem_peak,
            progress: record.progress,
            workflow_as_code_status: record
                .workflow_as_code_status
                .map(|x: sqlx::types::Json<Box<RawValue>>| x.0),
            job,
            flow_status: record
                .flow_status
                .map(|x: sqlx::types::Json<Box<RawValue>>| x.0),
            only_result: None,
            flow_stream_job_id,
        })
    }
}

async fn list_completed_jobs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListCompletedQuery>,
) -> error::JsonResult<Vec<ListableCompletedJob>> {
    let include_args = lq.include_args.unwrap_or(false);

    if include_args && *CLOUD_HOSTED {
        return Err(error::Error::BadRequest(
            "include_args is not supported on cloud hosted Windmill".to_string(),
        ));
    }

    let (per_page, offset) = paginate(pagination);

    let args_field = if include_args {
        "v2_job.args"
    } else {
        "null as args"
    };

    let sql = list_completed_jobs_query(
        &w_id,
        Some(per_page),
        offset,
        &lq,
        &[
            "v2_job_completed.id",
            "v2_job_completed.workspace_id",
            "v2_job.parent_job",
            "v2_job.created_by",
            "v2_job.created_at",
            "v2_job_completed.started_at",
            "v2_job_completed.duration_ms",
            "v2_job_completed.status = 'success' OR v2_job_completed.status = 'skipped' as success",
            "v2_job.runnable_id as script_hash",
            "v2_job.runnable_path as script_path",
            "false as deleted",
            "v2_job_completed.status = 'canceled' as canceled",
            "v2_job_completed.canceled_by",
            "v2_job_completed.canceled_reason",
            "v2_job.kind as job_kind",
            "CASE WHEN v2_job.trigger_kind = 'schedule' THEN v2_job.trigger END as schedule_path",
            "v2_job.permissioned_as",
            "null as raw_code",
            "null as flow_status",
            "null as raw_flow",
            "v2_job.flow_step_id IS NOT NULL as is_flow_step",
            "v2_job.script_lang as language",
            "v2_job_completed.status = 'skipped' as is_skipped",
            "v2_job.permissioned_as_email as email",
            "v2_job.visible_to_owner",
            "v2_job_completed.memory_peak as mem_peak",
            "v2_job.tag",
            "v2_job.priority",
            "v2_job_completed.result->'wm_labels' as labels",
            args_field,
            "'CompletedJob' as type",
        ],
        false,
        get_scope_tags(&authed),
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
    OptAuthed(opt_authed): OptAuthed,
    opt_tokened: OptTokened,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::Result<Response> {
    let tags = opt_authed
        .as_ref()
        .map(|authed| get_scope_tags(authed))
        .flatten();

    let job_o = GetQuery::new()
        .with_auth(&opt_authed)
        .with_in_tags(tags.as_ref())
        .fetch_completed(&db, &id, &w_id)
        .await?;

    let cj = not_found_if_none(job_o, "Completed Job", id.to_string())?;
    let response = Json(cj).into_response();
    // let extra_log = query_scalar!(
    //     "SELECT substr(logs, $1) as logs FROM large_logs WHERE workspace_id = $2 AND job_id = $3",
    //     log_offset - len,
    //     &w_id,
    //     &job_id
    // )
    // .fetch_optional(db)
    // .await.ok().flatten().flatten();

    log_job_view(
        &db,
        opt_authed.as_ref(),
        opt_tokened.token.as_deref(),
        &w_id,
        &id,
    )
    .await?;

    Ok(response)
}

#[derive(FromRow)]
pub struct RawResult {
    pub result: Option<sqlx::types::Json<Box<RawValue>>>,
    pub result_columns: Option<Vec<String>>,
    pub created_by: Option<String>,
}

async fn get_completed_job_result(
    OptAuthed(opt_authed): OptAuthed,
    opt_tokened: OptTokened,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Query(JsonPath { json_path, suspended_job, approver, resume_id, secret }): Query<JsonPath>,
) -> error::Result<Response> {
    let tags = opt_authed
        .as_ref()
        .map(|authed| get_scope_tags(authed))
        .flatten();
    let result_o = if let Some(json_path) = json_path {
        sqlx::query_as!(
            RawResult,
            "SELECT
                    result #> $3 AS \"result: sqlx::types::Json<Box<RawValue>>\",
                    result_columns,
                    created_by AS \"created_by!\"
                FROM v2_job_completed c
                    JOIN v2_job USING (id)
                WHERE c.id = $1 AND c.workspace_id = $2 AND ($4::text[] IS NULL OR tag = ANY($4))",
            id,
            &w_id,
            json_path.split(".").collect::<Vec<_>>() as Vec<&str>,
            tags.as_ref().map(|v| v.as_slice()) as Option<&[&str]>,
        )
        .fetch_optional(&db)
        .await?
    } else {
        sqlx::query_as!(
            RawResult,
            "SELECT
                    result AS \"result: sqlx::types::Json<Box<RawValue>>\",
                    result_columns,
                    created_by AS \"created_by!\"
                FROM v2_job_completed c
                    JOIN v2_job USING (id)
                WHERE c.id = $1 AND c.workspace_id = $2 AND ($3::text[] IS NULL OR tag = ANY($3))",
            id,
            &w_id,
            tags.as_ref().map(|v| v.as_slice()) as Option<&[&str]>,
        )
        .fetch_optional(&db)
        .await?
    };

    let mut raw_result = not_found_if_none(result_o, "Completed Job", id.to_string())?;

    if opt_authed.is_none() && raw_result.created_by.unwrap_or_default() != "anonymous" {
        match (suspended_job, resume_id, approver, secret) {
            (Some(suspended_job), Some(resume_id), approver, Some(secret)) => {
                let mut parent_job = id;
                while parent_job != suspended_job {
                    let p_job = sqlx::query_scalar!(
                        "SELECT parent_job FROM v2_job WHERE id = $1 AND workspace_id = $2",
                        parent_job,
                        &w_id
                    )
                    .fetch_optional(&db)
                    .await?
                    .flatten();
                    if let Some(p_job) = p_job {
                        parent_job = p_job;
                    } else {
                        return Err(Error::BadRequest("Approval secret of suspended job is not a parent of the job whose id's is being searched not found".to_string()));
                    }
                }
                verify_suspended_secret(
                    &w_id,
                    &db,
                    suspended_job,
                    resume_id,
                    &QueryApprover { approver, flow_level: None },
                    secret,
                )
                .await?
            }
            _ => {
                return Err(Error::BadRequest(
                    "As a non logged in user, you can only see jobs ran by anonymous users"
                        .to_string(),
                ))
            }
        }
    }

    format_result(
        raw_result.result_columns.as_ref(),
        raw_result.result.as_mut(),
    );

    log_job_view(
        &db,
        opt_authed.as_ref(),
        opt_tokened.token.as_deref(),
        &w_id,
        &id,
    )
    .await?;

    Ok(Json(raw_result.result).into_response())
}

#[derive(Deserialize)]
struct CountByTagQuery {
    horizon_secs: Option<i64>,
    workspace_id: Option<String>,
}

#[derive(Serialize)]
struct TagCount {
    tag: String,
    count: i64,
}

async fn count_by_tag(
    ApiAuthed { email, .. }: ApiAuthed,
    Extension(db): Extension<DB>,
    Query(query): Query<CountByTagQuery>,
) -> JsonResult<Vec<TagCount>> {
    require_super_admin(&db, &email).await?;
    let horizon = query.horizon_secs.unwrap_or(3600); // Default to 1 hour if not specified

    let counts = sqlx::query_as!(
            TagCount,
            r#"
            SELECT j.tag as "tag!", COUNT(*) as "count!"
            FROM v2_job_completed c JOIN v2_job j USING (id)
            WHERE c.started_at > NOW() - make_interval(secs => $1) AND ($2::text IS NULL OR j.workspace_id = $2)
            GROUP BY j.tag
            ORDER BY "count!" DESC
            "#,
            horizon as f64,
            query.workspace_id
        )
        .fetch_all(&db)
        .await?;

    Ok(Json(counts))
}

#[derive(Serialize)]
struct CompletedJobResult {
    started: Option<bool>,
    success: Option<bool>,
    completed: bool,
    result: Option<sqlx::types::Json<Box<RawValue>>>,
}

#[derive(Deserialize)]
struct GetCompletedJobQuery {
    get_started: Option<bool>,
}

async fn get_completed_job_result_maybe(
    OptAuthed(opt_authed): OptAuthed,
    opt_tokened: OptTokened,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Query(GetCompletedJobQuery { get_started }): Query<GetCompletedJobQuery>,
) -> error::Result<Response> {
    let tags = opt_authed
        .as_ref()
        .map(|authed| get_scope_tags(authed))
        .flatten();
    let result_o = sqlx::query!(
        "SELECT
                result AS \"result: sqlx::types::Json<Box<RawValue>>\",
                result_columns,
                status = 'success' AS \"success!\",
                created_by AS \"created_by!\"
            FROM v2_job_completed c
                JOIN v2_job j USING (id)
            WHERE c.id = $1 AND c.workspace_id = $2 AND ($3::text[] IS NULL OR tag = ANY($3))",
        id,
        &w_id,
        tags.as_ref().map(|v| v.as_slice()) as Option<&[&str]>,
    )
    .fetch_optional(&db)
    .await?;

    if let Some(mut res) = result_o {
        format_result(res.result_columns.as_ref(), res.result.as_mut());
        if opt_authed.is_none() && res.created_by != "anonymous" {
            return Err(Error::BadRequest(
                "As a non logged in user, you can only see jobs ran by anonymous users".to_string(),
            ));
        }

        log_job_view(
            &db,
            opt_authed.as_ref(),
            opt_tokened.token.as_deref(),
            &w_id,
            &id,
        )
        .await?;

        Ok(Json(CompletedJobResult {
            started: Some(true),
            success: Some(res.success),
            completed: true,
            result: res.result,
        })
        .into_response())
    } else if get_started.is_some_and(|x| x) {
        let started = sqlx::query_scalar!(
            "SELECT running AS \"running!\" FROM v2_job_queue WHERE id = $1 AND workspace_id = $2",
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

#[derive(Serialize)]
struct JobTiming {
    created_at: chrono::DateTime<Utc>,
    started_at: Option<chrono::DateTime<Utc>>,
    duration_ms: Option<i64>,
}

async fn get_completed_job_timing(
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::JsonResult<JobTiming> {
    let tags = opt_authed
        .as_ref()
        .map(|authed| get_scope_tags(authed))
        .flatten();

    let result = sqlx::query!(
        "SELECT
            j.created_at AS \"created_at!\",
            c.started_at,
            c.duration_ms,
            j.created_by AS \"created_by!\"
        FROM v2_job_completed c
            JOIN v2_job j USING (id)
        WHERE c.id = $1 AND c.workspace_id = $2 AND ($3::text[] IS NULL OR j.tag = ANY($3))",
        id,
        &w_id,
        tags.as_ref().map(|v| v.as_slice()) as Option<&[&str]>,
    )
    .fetch_optional(&db)
    .await?;

    let result = not_found_if_none(result, "Completed Job", id.to_string())?;

    if opt_authed.is_none() && result.created_by != "anonymous" {
        return Err(Error::BadRequest(
            "As a non logged in user, you can only see jobs ran by anonymous users".to_string(),
        ));
    }

    Ok(Json(JobTiming {
        created_at: result.created_at,
        started_at: result.started_at,
        duration_ms: Some(result.duration_ms),
    }))
}

async fn delete_completed_job<'a>(
    authed: ApiAuthed,
    Tokened { token }: Tokened,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::Result<Response> {
    let mut tx = user_db.begin(&authed).await?;

    require_admin(authed.is_admin, &authed.username)?;
    let tags = get_scope_tags(&authed);
    let job_o = sqlx::query_scalar!(
        "UPDATE v2_job_completed c SET
                    result = NULL,
                    deleted = TRUE
                FROM v2_job j
                WHERE c.id = $1
                    AND j.id = c.id
                    AND c.workspace_id = $2
                    AND ($3::TEXT[] IS NULL OR tag = ANY($3))
                RETURNING c.id
            ",
        id,
        &w_id,
        tags.as_ref().map(|v| v.as_slice()) as Option<&[&str]>,
    )
    .fetch_optional(&mut *tx)
    .await?;

    not_found_if_none(job_o, "Completed Job", id.to_string())?;

    sqlx::query!("UPDATE v2_job SET args = NULL WHERE id = $1", id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM job_logs WHERE job_id = $1", id)
        .execute(&mut *tx)
        .await?;

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
    return get_completed_job(
        OptAuthed(Some(authed)),
        OptTokened { token: Some(token) },
        Extension(db),
        Path((w_id, id)),
    )
    .await;
}

async fn get_otel_traces(
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> error::Result<Json<Vec<serde_json::Value>>> {
    // Check job exists and user has permission to view it
    let job = sqlx::query_scalar!(
        "SELECT created_by FROM v2_job WHERE id = $1 AND workspace_id = $2",
        id,
        w_id
    )
    .fetch_optional(&db)
    .await?;

    match job {
        Some(created_by) => {
            if opt_authed.is_none() && created_by != "anonymous" {
                return Err(Error::BadRequest(
                    "As a non logged in user, you can only see jobs ran by anonymous users"
                        .to_string(),
                ));
            }
        }
        None => {
            return Err(Error::NotFound(format!("Job {} not found", id)));
        }
    }

    let trace_id = id.as_bytes().as_slice();

    let traces = sqlx::query_scalar!(
        r#"SELECT json_build_object(
            'trace_id', encode(trace_id, 'hex'),           -- BYTEA to hex string
            'span_id', encode(span_id, 'hex'),             -- BYTEA to hex string
            'parent_span_id', encode(parent_span_id, 'hex'), -- BYTEA to hex string
            'trace_state', trace_state,
            'flags', flags,
            'name', name,
            'kind', kind,
            'start_time_unix_nano', start_time_unix_nano,
            'end_time_unix_nano', end_time_unix_nano,
            'attributes', attributes,
            'dropped_attributes_count', dropped_attributes_count,
            'events', events,
            'dropped_events_count', dropped_events_count,
            'links', links,
            'dropped_links_count', dropped_links_count,
            'status', status
        ) as "span!"
        FROM otel_traces
        WHERE trace_id = $1
        ORDER BY start_time_unix_nano"#,
        trace_id
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(traces))
}
