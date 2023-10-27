/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{collections::HashMap, vec};

use anyhow::Context;
use async_recursion::async_recursion;
use axum::{
    body::Bytes,
    extract::{FromRequest, Query},
    http::Request,
    response::{IntoResponse, Response},
    Form, RequestExt,
};
use bigdecimal::ToPrimitive;
use chrono::{DateTime, Duration, Utc};
use reqwest::{
    header::{HeaderMap, CONTENT_TYPE},
    Client, StatusCode,
};
use rsmq_async::RsmqConnection;
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue};
use sqlx::{types::Json, FromRow, Pool, Postgres, Transaction};
#[cfg(feature = "benchmark")]
use std::time::Instant;
use tracing::{instrument, Instrument};
use ulid::Ulid;
use uuid::Uuid;
use windmill_audit::{audit_log, ActionKind};
#[cfg(not(feature = "enterprise"))]
use windmill_common::worker::PriorityTags;
use windmill_common::{
    db::{Authed, UserDB},
    error::{self, Error},
    flow_status::{
        FlowStatus, FlowStatusModule, FlowStatusModuleWParent, JobResult, RestartedFrom,
        RetryStatus, MAX_RETRY_ATTEMPTS, MAX_RETRY_INTERVAL,
    },
    flows::{FlowModule, FlowModuleValue, FlowValue},
    jobs::{
        get_payload_tag_from_prefixed_path, script_path_to_payload, CompletedJob, JobKind,
        JobPayload, Metrics, QueuedJob, RawCode,
    },
    schedule::{schedule_to_user, Schedule},
    scripts::{ScriptHash, ScriptLang},
    users::{username_to_permissioned_as, SUPERADMIN_SECRET_EMAIL},
    worker::{to_raw_value, WORKER_CONFIG},
    DB, METRICS_ENABLED,
};

#[cfg(feature = "enterprise")]
use windmill_common::worker::CLOUD_HOSTED;

use crate::{
    schedule::{get_schedule_opt, push_scheduled_job},
    QueueTransaction,
};

lazy_static::lazy_static! {
    pub static ref HTTP_CLIENT: Client = reqwest::ClientBuilder::new()
        .user_agent("windmill/beta")
        .build().unwrap();

    // TODO: these aren't synced, they should be moved into the queue abstraction once/if that happens.
    static ref QUEUE_PUSH_COUNT: prometheus::IntCounter = prometheus::register_int_counter!(
        "queue_push_count",
        "Total number of jobs pushed to the queue."
    )
    .unwrap();
    static ref QUEUE_DELETE_COUNT: prometheus::IntCounter = prometheus::register_int_counter!(
        "queue_delete_count",
        "Total number of jobs deleted from the queue."
    )
    .unwrap();
    static ref QUEUE_PULL_COUNT: prometheus::IntCounter = prometheus::register_int_counter!(
        "queue_pull_count",
        "Total number of jobs pulled from the queue."
    )
    .unwrap();

}

#[cfg(feature = "enterprise")]
const MAX_FREE_EXECS: i32 = 1000;
#[cfg(feature = "enterprise")]
const MAX_FREE_CONCURRENT_RUNS: i32 = 15;

#[async_recursion]
pub async fn cancel_job<'c: 'async_recursion>(
    username: &str,
    reason: Option<String>,
    id: Uuid,
    w_id: &str,
    mut tx: Transaction<'c, Postgres>,
    db: &Pool<Postgres>,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    force_cancel: bool,
) -> error::Result<(Transaction<'c, Postgres>, Option<Uuid>)> {
    let job_running = get_queued_job(id, &w_id, &mut tx).await?;

    if job_running.is_none() {
        return Ok((tx, None));
    }
    let job_running = job_running.unwrap();

    if ((job_running.running || job_running.root_job.is_some())
        || (job_running.job_kind == JobKind::Flow || job_running.job_kind == JobKind::FlowPreview))
        && !force_cancel
    {
        let id = sqlx::query_scalar!(
        "UPDATE queue SET  canceled = true, canceled_by = $1, canceled_reason = $2, scheduled_for = now(), suspend = 0 WHERE id = $3 AND workspace_id = $4 RETURNING id",
        username,
        reason,
        id,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
        if let Some(id) = id {
            tracing::info!("Soft cancelling job {}", id);
        }
    } else {
        let reason = reason
            .clone()
            .unwrap_or_else(|| "No reason provided".to_string());
        let e = serde_json::json!({"message": format!("Job canceled: {reason} by {username}"), "name": "Canceled", "reason": reason, "canceler": username});
        let add_job = add_completed_job_error(
            &db,
            &job_running,
            format!("canceled by {username}: (force cancel: {force_cancel})"),
            job_running.mem_peak.unwrap_or(0),
            e,
            None,
            rsmq.clone(),
        )
        .await;
        if let Err(e) = add_job {
            tracing::error!("Failed to add canceled job: {}", e);
        }
    }
    if let Some(mut rsmq) = rsmq.clone() {
        rsmq.change_message_visibility(&job_running.tag, &id.to_string(), 0)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
    }

    let mut jobs = vec![id];
    let mut jobs_to_cancel = vec![];
    while !jobs.is_empty() {
        let p_job = jobs.pop();
        let new_jobs = sqlx::query_scalar!(
            "SELECT id FROM queue WHERE parent_job = $1 AND workspace_id = $2",
            p_job,
            w_id
        )
        .fetch_all(&mut *tx)
        .await?;
        jobs.extend(new_jobs.clone());
        jobs_to_cancel.extend(new_jobs);
    }
    for job in jobs_to_cancel {
        let (ntx, _) = cancel_job(
            username,
            reason.clone(),
            job,
            w_id,
            tx,
            db,
            rsmq.clone(),
            force_cancel,
        )
        .await?;
        tx = ntx;
    }
    Ok((tx, Some(id)))
}

#[derive(Serialize, Debug)]
pub struct WrappedError {
    pub error: serde_json::Value,
}

#[instrument(level = "trace", skip_all)]
pub async fn add_completed_job_error<R: rsmq_async::RsmqConnection + Clone + Send>(
    db: &Pool<Postgres>,
    queued_job: &QueuedJob,
    logs: String,
    mem_peak: i32,
    e: serde_json::Value,
    metrics: Option<Metrics>,
    rsmq: Option<R>,
) -> Result<WrappedError, Error> {
    if *METRICS_ENABLED {
        metrics.map(|m| m.worker_execution_failed.inc());
    }
    let result = WrappedError { error: e };
    tracing::error!(
        "job {} did not succeed: {}",
        queued_job.id,
        serde_json::to_string(&result).unwrap_or_else(|_| "".to_string())
    );
    let _ = add_completed_job(
        db,
        &queued_job,
        false,
        false,
        Json(&result),
        logs,
        mem_peak,
        rsmq,
    )
    .await?;
    Ok(result)
}

fn flatten_jobs(modules: Vec<FlowStatusModule>) -> Vec<Uuid> {
    modules
        .into_iter()
        .filter_map(|m| match m {
            FlowStatusModule::Success { job, flow_jobs, .. }
            | FlowStatusModule::Failure { job, flow_jobs, .. } => {
                if let Some(flow_jobs) = flow_jobs {
                    Some(flow_jobs)
                } else {
                    Some(vec![job])
                }
            }
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>()
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_ERROR_HANDLER_PATH_IN_ADMINS_WORKSPACE: Option<String> = std::env::var("GLOBAL_ERROR_HANDLER_PATH_IN_ADMINS_WORKSPACE").ok();
}

#[instrument(level = "trace", skip_all, name = "add_completed_job")]
pub async fn add_completed_job<
    T: Serialize + Send + Sync,
    R: rsmq_async::RsmqConnection + Clone + Send,
>(
    db: &Pool<Postgres>,
    queued_job: &QueuedJob,
    success: bool,
    skipped: bool,
    result: Json<&T>,
    logs: String,
    mem_peak: i32,
    rsmq: Option<R>,
) -> Result<Uuid, Error> {
    // tracing::error!("Start");
    // let start = tokio::time::Instant::now();

    let is_flow =
        queued_job.job_kind == JobKind::Flow || queued_job.job_kind == JobKind::FlowPreview;
    let duration = if is_flow {
        let jobs = queued_job.parse_flow_status().map(|s| {
            let mut modules = s.modules;
            modules.extend([s.failure_module.module_status]);
            flatten_jobs(modules)
        });
        if let Some(jobs) = jobs {
            sqlx::query_scalar!(
                "SELECT SUM(duration_ms) as duration FROM completed_job WHERE id = ANY($1)",
                jobs.as_slice()
            )
            .fetch_one(db)
            .await
            .ok()
            .flatten()
            .map(|x| x.to_i64())
            .flatten()
        } else {
            tracing::warn!("Could not parse flow status");
            None
        }
    } else {
        None
    };

    let mut tx: QueueTransaction<'_, R> = (rsmq.clone(), db.begin().await?).into();
    let job_id = queued_job.id;
    // tracing::error!("1 {:?}", start.elapsed());

    let mem_peak = mem_peak.max(queued_job.mem_peak.unwrap_or(0));
    let _duration: i64 = sqlx::query_scalar!(
        "INSERT INTO completed_job AS cj
                   ( workspace_id
                   , id
                   , parent_job
                   , created_by
                   , created_at
                   , started_at
                   , duration_ms
                   , success
                   , script_hash
                   , script_path
                   , args
                   , result
                   , logs
                   , raw_code
                   , raw_lock
                   , canceled
                   , canceled_by
                   , canceled_reason
                   , job_kind
                   , schedule_path
                   , permissioned_as
                   , flow_status
                   , raw_flow
                   , is_flow_step
                   , is_skipped
                   , language
                   , email
                   , visible_to_owner
                   , mem_peak
                   , tag
                   , priority
                )
            VALUES ($1, $2, $3, $4, $5, COALESCE($6, now()), COALESCE($26, (EXTRACT('epoch' FROM (now())) - EXTRACT('epoch' FROM (COALESCE($6, now()))))*1000), $7, $8, $9,\
                    $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $27, $28, $29, $30, $31)
         ON CONFLICT (id) DO UPDATE SET success = $7, result = $11, logs = concat(cj.logs, $12) RETURNING duration_ms",
        queued_job.workspace_id,
        queued_job.id,
        queued_job.parent_job,
        queued_job.created_by,
        queued_job.created_at,
        queued_job.started_at,
        success,
        queued_job.script_hash.map(|x| x.0),
        queued_job.script_path,
        &queued_job.args as &Option<Json<HashMap<String, Box<RawValue>>>>,
        result as Json<&T>,
        logs,
        queued_job.raw_code,
        queued_job.raw_lock,
        queued_job.canceled,
        queued_job.canceled_by,
        queued_job.canceled_reason,
        queued_job.job_kind.clone() as JobKind,
        queued_job.schedule_path,
        queued_job.permissioned_as,
        &queued_job.flow_status as &Option<Json<Box<RawValue>>>,
        &queued_job.raw_flow as &Option<Json<Box<RawValue>>>,
        queued_job.is_flow_step,
        skipped,
        queued_job.language.clone() as Option<ScriptLang>,
        duration as Option<i64>,
        queued_job.email,
        queued_job.visible_to_owner,
        if mem_peak > 0 { Some(mem_peak) } else { None },
        queued_job.tag,
        queued_job.priority,
    )
    .fetch_one(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("Could not add completed job {job_id}: {e}")))?;
    // tracing::error!("2 {:?}", start.elapsed());

    // tracing::error!("Added completed job {:#?}", queued_job);
    tx = delete_job(tx, &queued_job.workspace_id, job_id).await?;
    // tracing::error!("3 {:?}", start.elapsed());

    if !queued_job.is_flow_step
        && queued_job.schedule_path.is_some()
        && queued_job.script_path.is_some()
    {
        tx = apply_schedule_handlers(
            tx,
            db,
            queued_job.schedule_path.as_ref().unwrap(),
            queued_job.script_path.as_ref().unwrap(),
            &queued_job.workspace_id,
            success,
            result,
            job_id,
            queued_job.started_at.unwrap_or(chrono::Utc::now()),
            queued_job.priority,
        )
        .await?;
    }
    if !queued_job.is_flow_step
        && queued_job.job_kind != JobKind::Flow
        && queued_job.job_kind != JobKind::FlowPreview
        && queued_job.schedule_path.is_some()
        && queued_job.script_path.is_some()
    {
        // script only
        tx = handle_maybe_scheduled_job(
            tx,
            db,
            queued_job.schedule_path.as_ref().unwrap(),
            queued_job.script_path.as_ref().unwrap(),
            &queued_job.workspace_id,
        )
        .await?;
    }
    if queued_job.concurrent_limit.is_some() {
        if let Err(e) = sqlx::query_scalar!(
            "UPDATE concurrency_counter SET job_uuids = job_uuids - $2 WHERE concurrency_id = $1",
            queued_job.full_path(),
            queued_job.id.hyphenated().to_string(),
        )
        .execute(&mut tx)
        .await
        {
            tracing::error!("Could not decrement concurrency counter: {}", e);
        }
        tracing::debug!("decremented concurrency counter");
    }

    tx.commit().await?;

    #[cfg(feature = "enterprise")]
    if *CLOUD_HOSTED && !is_flow && _duration > 1000 {
        let additional_usage = _duration / 1000;
        let w_id = &queued_job.workspace_id;
        let premium_workspace = *windmill_common::worker::CLOUD_HOSTED
            && sqlx::query_scalar!("SELECT premium FROM workspace WHERE id = $1", w_id)
                .fetch_one(db)
                .await
                .map_err(|e| Error::InternalErr(format!("fetching if {w_id} is premium: {e}")))?;
        let _ = sqlx::query!(
                "INSERT INTO usage (id, is_workspace, month_, usage) 
                VALUES ($1, $2, EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date), 0) 
                ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + $3",
                if premium_workspace { w_id } else { &queued_job.email },
                premium_workspace,
                additional_usage as i32)
                .execute(db)
                .await
                .map_err(|e| Error::InternalErr(format!("updating usage: {e}")));
    }

    if matches!(queued_job.job_kind, JobKind::Flow | JobKind::Script)
        && queued_job.parent_job.is_none()
        && !success
    {
        if let Err(e) = send_error_to_global_handler(rsmq.clone(), &queued_job, db, result).await {
            tracing::error!(
                "Could not run global error handler for job {}: {}",
                &queued_job.id,
                e
            );
        }

        if let Err(e) = send_error_to_workspace_handler(rsmq.clone(), &queued_job, db, result).await
        {
            tracing::error!(
                "Could not run workspace error handler for job {}: {}",
                &queued_job.id,
                e
            );
        }
    }

    tracing::debug!("Added completed job {}", queued_job.id);
    // tracing::error!("4 {:?}", start.elapsed());

    Ok(queued_job.id)
}

pub async fn run_error_handler<
    'a,
    T: Serialize + Send + Sync,
    R: rsmq_async::RsmqConnection + Clone + Send,
>(
    rsmq: Option<R>,
    queued_job: &QueuedJob,
    db: &Pool<Postgres>,
    result: Json<&'a T>,
    error_handler_path: &str,
    error_handler_extra_args: Option<serde_json::Value>,
    is_global: bool,
) -> Result<(), Error> {
    let w_id = &queued_job.workspace_id;
    let script_w_id = if is_global { "admins" } else { w_id }; // script workspace id
    let job_id = queued_job.id;
    let (job_payload, tag) = script_path_to_payload(&error_handler_path, db, script_w_id).await?;

    let mut extra = HashMap::new();
    extra.insert("workspace_id".to_string(), to_raw_value(&w_id));
    extra.insert("job_id".to_string(), to_raw_value(&job_id));
    extra.insert("path".to_string(), to_raw_value(&queued_job.script_path));
    extra.insert(
        "is_flow".to_string(),
        to_raw_value(&queued_job.raw_flow.is_some()),
    );
    extra.insert("email".to_string(), to_raw_value(&queued_job.email));

    if let Some(extra_args) = error_handler_extra_args {
        if let serde_json::Value::Object(args_m) = extra_args {
            for (k, v) in args_m {
                extra.insert(k, to_raw_value(&v));
            }
        } else {
            return Err(error::Error::ExecutionErr(
                "args of scripts needs to be dict".to_string(),
            ));
        }
    }

    let tx = PushIsolationLevel::IsolatedRoot(db.clone(), rsmq);

    let (uuid, tx) = push(
        &db,
        tx,
        script_w_id,
        job_payload,
        PushArgs { extra, args: result.to_owned() },
        if is_global { "global" } else { "error_handler" },
        if is_global {
            SUPERADMIN_SECRET_EMAIL
        } else {
            "error_handler@windmill.dev"
        },
        if is_global {
            SUPERADMIN_SECRET_EMAIL.to_string()
        } else {
            "g/error_handler".to_string()
        },
        None,
        None,
        Some(job_id),
        Some(job_id),
        None,
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

    let error_handler_type = if is_global { "global" } else { "workspace" };
    tracing::info!(
        "Sent error of job {job_id} to {error_handler_type} error handler under uuid {uuid}"
    );

    Ok(())
}

pub async fn send_error_to_global_handler<
    'a,
    T: Serialize + Send + Sync,
    R: rsmq_async::RsmqConnection + Clone + Send,
>(
    rsmq: Option<R>,
    queued_job: &QueuedJob,
    db: &Pool<Postgres>,
    result: Json<&'a T>,
) -> Result<(), Error> {
    if let Some(ref global_error_handler) = *GLOBAL_ERROR_HANDLER_PATH_IN_ADMINS_WORKSPACE {
        run_error_handler(
            rsmq,
            queued_job,
            db,
            result,
            global_error_handler,
            None,
            true,
        )
        .await?
    }

    Ok(())
}

pub async fn send_error_to_workspace_handler<
    'a,
    'c,
    T: Serialize + Send + Sync,
    R: rsmq_async::RsmqConnection + Clone + Send,
>(
    rsmq: Option<R>,
    queued_job: &QueuedJob,
    db: &Pool<Postgres>,
    result: Json<&'a T>,
) -> Result<(), Error> {
    let w_id = &queued_job.workspace_id;
    let mut tx = db.begin().await?;
    let (error_handler, error_handler_extra_args) = sqlx::query_as::<_, (Option<String>, Option<serde_json::Value>)>(
        "SELECT error_handler, error_handler_extra_args FROM workspace_settings WHERE workspace_id = $1",
    ).bind(&w_id)
    .fetch_optional(&mut *tx)
    .await
    .context("sending error to global handler")?
    .ok_or_else(|| Error::InternalErr(format!("no workspace settings for id {w_id}")))?;

    if let Some(error_handler) = error_handler {
        let ws_error_handler_muted: Option<bool> = match queued_job.job_kind {
            JobKind::Script => {
                sqlx::query_scalar!(
                "SELECT ws_error_handler_muted FROM script WHERE workspace_id = $1 AND hash = $2",
                queued_job.workspace_id,
                queued_job.script_hash.unwrap().0,
            )
                .fetch_optional(db)
                .await?
            }
            JobKind::Flow => {
                sqlx::query_scalar!(
                    "SELECT ws_error_handler_muted FROM flow WHERE workspace_id = $1 AND path = $2",
                    queued_job.workspace_id,
                    queued_job.script_path.as_ref().unwrap(),
                )
                .fetch_optional(db)
                .await?
            }
            _ => None,
        };

        if !ws_error_handler_muted.unwrap_or(false) {
            run_error_handler(
                rsmq,
                queued_job,
                db,
                result,
                &error_handler.strip_prefix("script/").unwrap(),
                error_handler_extra_args,
                false,
            )
            .await?
        }
    }

    Ok(())
}

#[instrument(level = "trace", skip_all)]
pub async fn handle_maybe_scheduled_job<'c, R: rsmq_async::RsmqConnection + Clone + Send + 'c>(
    mut tx: QueueTransaction<'c, R>,
    db: &Pool<Postgres>,
    schedule_path: &str,
    script_path: &str,
    w_id: &str,
) -> windmill_common::error::Result<QueueTransaction<'c, R>> {
    let schedule = get_schedule_opt(tx.transaction_mut(), w_id, schedule_path).await?;

    if schedule.is_none() {
        tracing::error!(
            "Schedule {schedule_path} in {w_id} not found. Impossible to schedule again"
        );
        return Ok(tx);
    }

    let schedule = schedule.unwrap();

    if schedule.enabled && script_path == schedule.script_path {
        let res = push_scheduled_job(
            db,
            tx,
            Schedule {
                workspace_id: w_id.to_owned(),
                path: schedule.path.clone(),
                edited_by: schedule.edited_by,
                edited_at: schedule.edited_at,
                schedule: schedule.schedule,
                timezone: schedule.timezone,
                enabled: schedule.enabled,
                script_path: schedule.script_path,
                is_flow: schedule.is_flow,
                args: schedule
                    .args
                    .and_then(|e| serde_json::to_value(e).map_or(None, |v| Some(v))),
                extra_perms: serde_json::to_value(schedule.extra_perms).expect("hashmap -> json"),
                email: schedule.email,
                error: None,
                on_failure: schedule.on_failure,
                on_failure_times: schedule.on_failure_times,
                on_failure_exact: schedule.on_failure_exact,
                on_failure_extra_args: schedule.on_failure_extra_args,
                on_recovery: schedule.on_recovery,
                on_recovery_times: schedule.on_recovery_times,
                on_recovery_extra_args: schedule.on_recovery_extra_args,
            },
        )
        .await;
        match res {
            Ok(tx) => Ok(tx),
            Err(err) => {
                sqlx::query!(
                    "UPDATE schedule SET enabled = false, error = $1 WHERE workspace_id = $2 AND path = $3",
                    err.to_string(),
                    &schedule.workspace_id,
                    &schedule.path
                )
                .execute(db)
                .await?;
                tracing::warn!("Could not schedule job for {}: {}", schedule_path, err);
                Err(err)
            }
        }
    } else {
        Ok(tx)
    }
}

#[derive(Clone, Serialize)]
struct CompletedJobSubset {
    success: bool,
    result: Option<serde_json::Value>,
    started_at: chrono::DateTime<chrono::Utc>,
}
async fn apply_schedule_handlers<
    'a,
    'c,
    T: Serialize + Send + Sync,
    R: rsmq_async::RsmqConnection + Clone + Send + 'c,
>(
    mut tx: QueueTransaction<'c, R>,
    db: &Pool<Postgres>,
    schedule_path: &str,
    script_path: &str,
    w_id: &str,
    success: bool,
    result: Json<&'a T>,
    job_id: Uuid,
    started_at: DateTime<Utc>,
    job_priority: Option<i16>,
) -> windmill_common::error::Result<QueueTransaction<'c, R>> {
    let schedule = get_schedule_opt(tx.transaction_mut(), w_id, schedule_path).await?;

    if schedule.is_none() {
        tracing::error!(
            "Schedule {schedule_path} in {w_id} not found. Impossible to apply schedule handlers"
        );
        return Ok(tx);
    }

    let schedule = schedule.unwrap();

    if !success {
        if let Some(on_failure_path) = schedule.on_failure.clone() {
            let times = schedule.on_failure_times.unwrap_or(1).max(1);
            let exact = schedule.on_failure_exact.unwrap_or(false);
            if times > 1 || exact {
                let past_jobs = sqlx::query_as!(
                    CompletedJobSubset,
                    "SELECT success, result, started_at FROM completed_job WHERE workspace_id = $1 AND schedule_path = $2 AND script_path = $3 AND id != $4 ORDER BY created_at DESC LIMIT $5",
                    &schedule.workspace_id,
                    &schedule.path,
                    &schedule.script_path,
                    job_id,
                    if exact { times } else { times - 1 } as i64,
                ).fetch_all(&mut tx).await?;

                let match_times = if exact {
                    past_jobs.len() == times as usize
                        && past_jobs[..(times - 1) as usize].iter().all(|j| !j.success)
                        && past_jobs[(times - 1) as usize].success
                } else {
                    past_jobs.len() == ((times - 1) as usize)
                        && past_jobs.iter().all(|j| !j.success)
                };

                if !match_times {
                    return Ok(tx);
                }
            }

            let on_failure_result = handle_on_failure(
                db,
                tx,
                schedule_path,
                script_path,
                schedule.is_flow,
                w_id,
                &on_failure_path,
                result,
                times,
                started_at,
                schedule.on_failure_extra_args,
                &schedule.email,
                &schedule_to_user(&schedule.path),
                username_to_permissioned_as(&schedule.edited_by),
                job_priority,
            )
            .await;

            match on_failure_result {
                Ok((_, ntx)) => {
                    tx = ntx;
                }
                Err(err) => {
                    sqlx::query!(
                    "UPDATE schedule SET enabled = false, error = $1 WHERE workspace_id = $2 AND path = $3",
                    format!("Could not trigger error handler: {err}"),
                    &schedule.workspace_id,
                    &schedule.path
                )
                .execute(db)
                .await?;
                    tracing::warn!(
                        "Could not trigger error handler for {}: {}",
                        schedule_path,
                        err
                    );
                    return Err(err);
                }
            }
        }
    } else {
        if let Some(on_recovery_path) = schedule.on_recovery.clone() {
            let times = schedule.on_recovery_times.unwrap_or(1).max(1);
            let past_jobs = sqlx::query_as!(
                CompletedJobSubset,
                "SELECT success, result, started_at FROM completed_job WHERE workspace_id = $1 AND schedule_path = $2 AND script_path = $3 AND id != $4 ORDER BY created_at DESC LIMIT $5",
                &schedule.workspace_id,
                &schedule.path,
                &schedule.script_path,
                job_id,
                times as i64,
            ).fetch_all(&mut tx).await?;

            if past_jobs.len() < times as usize {
                return Ok(tx);
            }

            let n_times_successful = past_jobs[..(times - 1) as usize].iter().all(|j| j.success);

            if !n_times_successful {
                return Ok(tx);
            }

            let failed_job = past_jobs[past_jobs.len() - 1].clone();

            if !failed_job.success {
                let on_recovery_result = handle_on_recovery(
                    db,
                    tx,
                    schedule_path,
                    script_path,
                    schedule.is_flow,
                    w_id,
                    &on_recovery_path,
                    failed_job,
                    result,
                    times,
                    started_at,
                    schedule.on_recovery_extra_args,
                    &schedule.email,
                    &schedule_to_user(&schedule.path),
                    username_to_permissioned_as(&schedule.edited_by),
                )
                .await;

                match on_recovery_result {
                    Ok(ntx) => {
                        tx = ntx;
                    }
                    Err(err) => {
                        sqlx::query!(
                            "UPDATE schedule SET enabled = false, error = $1 WHERE workspace_id = $2 AND path = $3",
                            format!("Could not trigger recovery handler: {err}"),
                            &schedule.workspace_id,
                            &schedule.path
                        )
                        .execute(db)
                        .await?;
                        tracing::warn!(
                            "Could not trigger recovery handler for {}: {}",
                            schedule_path,
                            err
                        );
                        return Err(err);
                    }
                }
            }
        }
    }

    Ok(tx)
}

pub async fn handle_on_failure<
    'a,
    'c,
    T: Serialize + Send + Sync,
    R: rsmq_async::RsmqConnection + Clone + Send + 'c,
>(
    db: &Pool<Postgres>,
    tx: QueueTransaction<'c, R>,
    schedule_path: &str,
    script_path: &str,
    is_flow: bool,
    w_id: &str,
    on_failure_path: &str,
    result: Json<&'a T>,
    failed_times: i32,
    started_at: DateTime<Utc>,
    extra_args: Option<serde_json::Value>,
    username: &str,
    email: &str,
    permissioned_as: String,
    priority: Option<i16>,
) -> windmill_common::error::Result<(Uuid, QueueTransaction<'c, R>)> {
    let (payload, tag) = get_payload_tag_from_prefixed_path(on_failure_path, db, w_id).await?;

    let mut extra = HashMap::new();
    extra.insert("schedule_path".to_string(), to_raw_value(&schedule_path));
    extra.insert("path".to_string(), to_raw_value(&script_path));
    extra.insert("is_flow".to_string(), to_raw_value(&is_flow));
    extra.insert("started_at".to_string(), to_raw_value(&started_at));
    extra.insert("failed_times".to_string(), to_raw_value(&failed_times));

    if let Some(args_v) = extra_args {
        if let serde_json::Value::Object(args_m) = args_v {
            for (k, v) in args_m {
                extra.insert(k, to_raw_value(&v));
            }
        } else {
            return Err(error::Error::ExecutionErr(
                "args of scripts needs to be dict".to_string(),
            ));
        }
    }

    let tx = PushIsolationLevel::Transaction(tx);
    let (uuid, tx) = push(
        &db,
        tx,
        w_id,
        payload,
        PushArgs { extra, args: result.to_owned() },
        username,
        email,
        permissioned_as,
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        true,
        tag,
        None,
        None,
        priority,
    )
    .await?;
    tracing::info!(
        "Pushed on_failure job {} for {} to queue",
        uuid,
        schedule_path
    );
    return Ok((uuid, tx));
}

// #[derive(Serialize)]
// pub struct RecoveryValue<T> {
//     error_started_at: chrono::DateTime<Utc>,
//     schedule_path: String,
//     path: String,
//     is_flow: boolean,
//     extra_args: serde_json::Value
// }
async fn handle_on_recovery<
    'a,
    'c,
    T: Serialize + Send + Sync,
    R: rsmq_async::RsmqConnection + Clone + Send + 'c,
>(
    db: &Pool<Postgres>,
    tx: QueueTransaction<'c, R>,
    schedule_path: &str,
    script_path: &str,
    is_flow: bool,
    w_id: &str,
    on_recovery_path: &str,
    error_job: CompletedJobSubset,
    successful_job_result: Json<&'a T>,
    successful_times: i32,
    successful_job_started_at: DateTime<Utc>,
    extra_args: Option<serde_json::Value>,
    username: &str,
    email: &str,
    permissioned_as: String,
) -> windmill_common::error::Result<QueueTransaction<'c, R>> {
    let (payload, tag) = get_payload_tag_from_prefixed_path(on_recovery_path, db, w_id).await?;

    let mut args = error_job
        .result
        .unwrap_or(json!({}))
        .as_object()
        .unwrap()
        .clone();
    args.insert("error_started_at".to_string(), json!(error_job.started_at));
    args.insert("schedule_path".to_string(), json!(schedule_path));
    args.insert("path".to_string(), json!(script_path));
    args.insert("is_flow".to_string(), json!(is_flow));
    args.insert(
        "success_result".to_string(),
        serde_json::from_str(&serde_json::to_string(&successful_job_result).unwrap())
            .unwrap_or_else(|_| json!("{}")),
    );
    args.insert("success_times".to_string(), json!(successful_times));
    args.insert(
        "success_started_at".to_string(),
        json!(successful_job_started_at),
    );
    if let Some(args_v) = extra_args {
        if let serde_json::Value::Object(args_m) = args_v {
            args.extend(args_m);
        } else {
            return Err(error::Error::ExecutionErr(
                "args of scripts needs to be dict".to_string(),
            ));
        }
    }
    let tx = PushIsolationLevel::Transaction(tx);
    let (uuid, tx) = push(
        &db,
        tx,
        w_id,
        payload,
        args,
        username,
        email,
        permissioned_as,
        None,
        None,
        None,
        None,
        None,
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
    tracing::info!(
        "Pushed on_recovery job {} for {} to queue",
        uuid,
        schedule_path
    );
    return Ok(tx);
}

pub async fn pull<R: rsmq_async::RsmqConnection + Send + Clone>(
    db: &Pool<Postgres>,
    rsmq: Option<R>,
    suspend_first: bool,
) -> windmill_common::error::Result<Option<QueuedJob>> {
    loop {
        let job = pull_single_job_and_mark_as_running_no_concurrency_limit(
            db,
            rsmq.clone(),
            suspend_first,
        )
        .await?;

        if job.is_none() {
            return Ok(None);
        }

        // concurrency check. If more than X jobs for this path are already running, we re-queue and pull another job from the queue
        let pulled_job = job.unwrap();
        if pulled_job.script_path.is_none()
            || pulled_job.concurrent_limit.is_none()
            || pulled_job.canceled
        {
            if *METRICS_ENABLED {
                QUEUE_PULL_COUNT.inc();
            }
            return Ok(Option::Some(pulled_job));
        }

        let itx = db.begin().await?;

        let mut tx: QueueTransaction<'_, _> = (rsmq.clone(), itx).into();

        // Else the job is subject to concurrency limits
        let job_script_path = pulled_job.script_path.clone().unwrap();

        let job_custom_concurrent_limit = pulled_job.concurrent_limit.unwrap();
        // setting concurrency_time_window to 0 will count only the currently running jobs
        let job_custom_concurrency_time_window_s =
            pulled_job.concurrency_time_window_s.unwrap_or(0);
        tracing::debug!(
            "Job concurrency limit is {} per {}s",
            job_custom_concurrent_limit,
            job_custom_concurrency_time_window_s
        );

        let jobs_uuids_init_json_value = serde_json::from_str::<serde_json::Value>(
            format!("{{\"{}\": {{}}}}", pulled_job.id.hyphenated().to_string()).as_str(),
        )
        .expect("Unable to serialize job_uuids column to proper JSON");
        let running_job = sqlx::query_scalar!(
            "INSERT INTO concurrency_counter(concurrency_id, job_uuids) VALUES ($1, $2)
        ON CONFLICT (concurrency_id) 
        DO UPDATE SET job_uuids = jsonb_set(concurrency_counter.job_uuids, array[$3], '{}')
        RETURNING (SELECT COUNT(*) FROM jsonb_object_keys(job_uuids))",
            pulled_job.full_path(),
            jobs_uuids_init_json_value,
            pulled_job.id.hyphenated().to_string(),
        )
        .fetch_one(&mut tx)
        .await
        .map_err(|e| {
            Error::InternalErr(format!(
                "Error getting concurrency count for script path {job_script_path}: {e}"
            ))
        })?;
        tracing::debug!("running_job: {}", running_job.unwrap_or(0));

        let script_path_live_stats = sqlx::query!(
            "SELECT COALESCE(j.min_started_at, q.min_started_at) AS min_started_at, COALESCE(completed_count, 0) AS completed_count
            FROM
                (SELECT script_path, MIN(started_at) as min_started_at, COUNT(*) as completed_count
                FROM completed_job
                WHERE script_path = $1 AND job_kind != 'dependencies' AND started_at + INTERVAL '1 MILLISECOND' * duration_ms > (now() - INTERVAL '1 second' * $2) AND workspace_id = $3 AND canceled = false
                GROUP BY script_path) as j
            FULL OUTER JOIN
                (SELECT script_path, MIN(started_at) as min_started_at
                FROM queue
                WHERE script_path = $1 AND job_kind != 'dependencies'  AND running = true AND workspace_id = $3 AND canceled = false
                GROUP BY script_path) as q
            ON q.script_path = j.script_path",
            job_script_path,
            f64::from(job_custom_concurrency_time_window_s),
            &pulled_job.workspace_id
        )
        .fetch_one(&mut tx)
        .await
        .map_err(|e| {
            Error::InternalErr(format!(
                "Error getting concurrency count for script path {job_script_path}: {e}"
            ))
        })?;

        let concurrent_jobs_for_this_script =
            script_path_live_stats.completed_count.unwrap_or_default() as i32
                + running_job.unwrap_or(0) as i32;
        tracing::debug!(
            "Current concurrent jobs for this script: {}",
            concurrent_jobs_for_this_script
        );
        if concurrent_jobs_for_this_script <= job_custom_concurrent_limit {
            if *METRICS_ENABLED {
                QUEUE_PULL_COUNT.inc();
            }
            tx.commit().await?;
            return Ok(Option::Some(pulled_job));
        }
        let x = sqlx::query_scalar!(
            "UPDATE concurrency_counter SET job_uuids = job_uuids - $2 WHERE concurrency_id = $1 RETURNING (SELECT COUNT(*) FROM jsonb_object_keys(job_uuids))",
            pulled_job.full_path(),
            pulled_job.id.hyphenated().to_string(),

        )
        .fetch_one(&mut tx)
        .await
        .map_err(|e| {
            Error::InternalErr(format!(
                "Error decreasing concurrency count for script path {job_script_path}: {e}"
            ))
        })?;
        tracing::debug!("running_job after decrease: {}", x.unwrap_or(0));

        let job_uuid: Uuid = pulled_job.id;
        let min_started_at: Option<DateTime<Utc>> = script_path_live_stats.min_started_at;
        let avg_script_duration: Option<i64> = sqlx::query_scalar!(
            "SELECT CAST(ROUND(AVG(duration_ms) / 1000, 0) AS BIGINT) AS avg_duration_s FROM
                (SELECT duration_ms FROM completed_job WHERE script_path = $1
                ORDER BY started_at
                DESC LIMIT 10) AS t",
            job_script_path
        )
        .fetch_one(&mut tx)
        .await?;

        // optimal scheduling is: 'older_job_in_concurrency_time_window_started_timestamp + script_avg_duration + concurrency_time_window_s'
        let estimated_next_schedule_timestamp = min_started_at.unwrap_or(pulled_job.scheduled_for)
            + Duration::seconds(avg_script_duration.map(i64::from).unwrap_or(0))
            + Duration::seconds(i64::from(job_custom_concurrency_time_window_s));
        tracing::info!("Job '{}' from path '{}' has reached its concurrency limit of {} jobs run in the last {} seconds. This job will be re-queued for next execution at {}", 
            job_uuid, job_script_path, job_custom_concurrent_limit, job_custom_concurrency_time_window_s, estimated_next_schedule_timestamp);

        let job_log_line_break = '\n';
        let job_log_event = format!(
            "Re-scheduled job to {estimated_next_schedule_timestamp} due to concurrency limits"
        );
        if rsmq.is_some() {
            // if let Some(ref mut rsmq) = tx.rsmq {
            // if using redis, only one message at a time can be poped from the queue. Process only this message and move to the next elligible job
            // In this case, the job might be a job from the same script path, but we can't optimise this further
            // if using posgtres, then we're able to re-queue the entire batch of scheduled job for this script_path, so we do it
            let requeued_job_tag = sqlx::query_scalar::<_, String>(&format!(
                "UPDATE queue
                SET running = false
                , started_at = null
                , scheduled_for = '{estimated_next_schedule_timestamp}'
                , logs = CASE WHEN logs IS NULL OR logs = '' THEN '{job_log_event}'::text WHEN logs LIKE '%{job_log_event}' THEN logs ELSE concat(logs, '{job_log_line_break}{job_log_event}'::text) END
                WHERE id = '{job_uuid}'
                RETURNING tag"
            ))
            .fetch_one(&mut tx)
            .await
            .map_err(|e| Error::InternalErr(format!("Could not update and re-queue job {job_uuid}. The job will be marked as running but it is not running: {e}")))?;

            if let Some(ref mut rsmq) = tx.rsmq {
                rsmq.send_message(
                    job_uuid.to_bytes_le().to_vec(),
                    Option::Some(estimated_next_schedule_timestamp),
                    requeued_job_tag,
                );
            }
            tx.commit().await?;
        } else {
            // if using posgtres, then we're able to re-queue the entire batch of scheduled job for this script_path, so we do it
            sqlx::query(&format!(
                "UPDATE queue
                SET running = false
                , started_at = null
                , scheduled_for = '{estimated_next_schedule_timestamp}'
                , logs = CASE WHEN logs IS NULL OR logs = '' THEN '{job_log_event}'::text WHEN logs LIKE '%{job_log_event}' THEN logs ELSE concat(logs, '{job_log_line_break}{job_log_event}'::text) END
                WHERE (id = '{job_uuid}') OR (script_path = '{job_script_path}' AND running = false AND scheduled_for <= now())"
            ))
            .fetch_all(&mut tx)
            .await
            .map_err(|e| Error::InternalErr(format!("Could not update and re-queue job {job_uuid}. The job will be marked as running but it is not running: {e}")))?;
            tx.commit().await?
        }
    }
}

async fn pull_single_job_and_mark_as_running_no_concurrency_limit<
    'c,
    R: rsmq_async::RsmqConnection + Send + Clone,
>(
    db: &Pool<Postgres>,
    rsmq: Option<R>,
    suspend_first: bool,
) -> windmill_common::error::Result<Option<QueuedJob>> {
    let job: Option<QueuedJob> = if let Some(mut rsmq) = rsmq {
        #[cfg(feature = "benchmark")]
        let instant = Instant::now();

        // TODO: REDIS: Race conditions / replace last_ping

        // TODO: shuffle this list to have fairness
        let mut all_tags = WORKER_CONFIG.read().await.worker_tags.clone();

        let mut msg: Option<_> = None;
        let mut tag = None;

        while msg.is_none() && !all_tags.is_empty() {
            let ntag = all_tags.pop().unwrap();
            tag = Some(ntag.clone());
            msg = rsmq
                .receive_message::<Vec<u8>>(&ntag, Some(10))
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        // #[cfg(feature = "benchmark")]
        // println!("rsmq 1: {:?}", instant.elapsed());

        // println!("3.1: {:?} {rs}", instant.elapsed());
        if let Some(msg) = msg {
            let uuid = Uuid::from_bytes_le(
                msg.message
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Failed to parsed Redis message"))?,
            );

            let m2r = sqlx::query(
                "UPDATE queue
            SET running = true
            , started_at = coalesce(started_at, now())
            , last_ping = now()
            , suspend_until = null
            WHERE id = $1
            RETURNING *",
            )
            .bind(uuid)
            .fetch_optional(db)
            .await?;
            let m2 = if let Some(row) = m2r {
                Some(QueuedJob::from_row(&row)?)
            } else {
                None
            };

            rsmq.delete_message(&tag.unwrap(), &msg.id)
                .await
                .map_err(|e| anyhow::anyhow!(e))?;

            #[cfg(feature = "benchmark")]
            println!("rsmq 2: {:?}", instant.elapsed());

            m2
        } else {
            None
        }
    } else {
        /* Jobs can be started if they:
         * - haven't been started before,
         *   running = false
         * - are flows with a step that needed resume,
         *   suspend_until is non-null
         *   and suspend = 0 when the resume messages are received
         *   or suspend_until <= now() if it has timed out */
        let config = WORKER_CONFIG.read().await.clone();
        let tags = config.worker_tags.clone();
        #[cfg(not(feature = "enterprise"))]
        let priority_tags_sorted = vec![PriorityTags { priority: 0, tags: tags.clone() }];
        #[cfg(feature = "enterprise")]
        let priority_tags_sorted = config.priority_tags_sorted.clone();
        drop(config);
        let r = if suspend_first {
            sqlx::query("UPDATE queue
            SET running = true
              , started_at = coalesce(started_at, now())
              , last_ping = now()
              , suspend_until = null
            WHERE id = (
                SELECT id
                FROM queue
                WHERE suspend_until IS NOT NULL AND (suspend <= 0 OR suspend_until <= now()) AND tag = ANY($1)
                ORDER BY priority DESC NULLS LAST, created_at
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            RETURNING *")
                .bind(tags.clone())
                .fetch_optional(db)
                .await?
        } else {
            None
        };
        let r = if let Some(row) = r {
            Some(QueuedJob::from_row(&row)?)
        } else {
            None
        };
        if r.is_none() {
            // #[cfg(feature = "benchmark")]
            // let instant = Instant::now();
            let mut highest_priority_job: Option<QueuedJob> = None;

            for priority_tags in priority_tags_sorted {
                let r = sqlx::query(
                    "UPDATE queue
                    SET running = true
                    , started_at = coalesce(started_at, now())
                    , last_ping = now()
                    , suspend_until = null
                    WHERE id = (
                        SELECT id
                        FROM queue
                        WHERE running = false AND scheduled_for <= now() AND tag = ANY($1)
                        ORDER BY priority DESC NULLS LAST, scheduled_for, created_at
                        FOR UPDATE SKIP LOCKED
                        LIMIT 1
                    )
                    RETURNING *",
                )
                .bind(priority_tags.tags.clone())
                .fetch_optional(db)
                .await?;

                if let Some(pulled_row) = r {
                    let pulled_job = QueuedJob::from_row(&pulled_row)?;
                    highest_priority_job = Some(pulled_job.clone());
                    tracing::debug!(
                        "Pulling for job {} with tags {:?} with priority {}",
                        pulled_job.id,
                        priority_tags.tags,
                        priority_tags.priority
                    );
                    break;
                }
                // else continue pulling for lower priority tags
            }

            // #[cfg(feature = "benchmark")]
            // println!("pull query: {:?}", instant.elapsed());
            highest_priority_job
        } else {
            r
        }
    };
    Ok(job)
}

#[derive(FromRow)]
struct ResultR {
    result: Option<Json<Box<RawValue>>>,
}

pub async fn get_result_by_id(
    db: Pool<Postgres>,
    w_id: String,
    flow_id: Uuid,
    node_id: String,
    json_path: Option<String>,
) -> error::Result<Box<RawValue>> {
    match get_result_by_id_from_running_flow(
        &db,
        w_id.as_str(),
        &flow_id,
        node_id.as_str(),
        json_path.clone(),
    )
    .await
    {
        Ok(res) => Ok(res),
        Err(_) => {
            let running_flow_job = sqlx::query_as::<_, QueuedJob>(
                "SELECT * FROM queue WHERE COALESCE((SELECT root_job FROM queue WHERE id = $1), $1) = id AND workspace_id = $2"
            ).bind(flow_id)
            .bind(&w_id)
            .fetch_optional(&db)
            .await?;

            let restarted_from = windmill_common::utils::not_found_if_none(
                running_flow_job
                    .map(|fj| fj.parse_flow_status())
                    .flatten()
                    .map(|status| status.restarted_from)
                    .flatten(),
                "Flow result by id in leaf jobs",
                format!("{}, {}", flow_id, node_id),
            )?;

            get_result_by_id_from_original_flow(
                &db,
                w_id.as_str(),
                &restarted_from.flow_job_id,
                node_id.as_str(),
                json_path.clone(),
            )
            .await
        }
    }
}

#[async_recursion]
pub async fn get_result_by_id_from_running_flow(
    db: &Pool<Postgres>,
    w_id: &str,
    flow_id: &Uuid,
    node_id: &str,
    json_path: Option<String>,
) -> error::Result<Box<RawValue>> {
    let flow_job_result = sqlx::query!(
        "SELECT leaf_jobs->$1::text as leaf_jobs, parent_job FROM queue WHERE COALESCE((SELECT root_job FROM queue WHERE id = $2), $2) = id AND workspace_id = $3",
        node_id,
        flow_id,
        w_id,
    )
    .fetch_optional(db)
    .await?;

    let flow_job_result = windmill_common::utils::not_found_if_none(
        flow_job_result,
        "Flow result by id in leaf jobs",
        format!("{}, {}", flow_id, node_id),
    )?;

    let job_result = flow_job_result
        .leaf_jobs
        .map(|x| serde_json::from_value(x).ok())
        .flatten();

    if job_result.is_none() && flow_job_result.parent_job.is_some() {
        let parent_job = flow_job_result.parent_job.unwrap();
        let root_job = sqlx::query_scalar!("SELECT root_job FROM queue WHERE id = $1", parent_job)
            .fetch_optional(db)
            .await?
            .flatten()
            .unwrap_or(parent_job);
        return get_result_by_id_from_running_flow(db, w_id, &root_job, node_id, json_path).await;
    }

    let result_id = windmill_common::utils::not_found_if_none(
        job_result,
        "Flow result by id",
        format!("{}, {}", flow_id, node_id),
    )?;

    let value = match result_id {
        JobResult::ListJob(x) => {
            let rows = sqlx::query(
                "SELECT result FROM completed_job WHERE id = ANY($1) AND workspace_id = $2",
            )
            .bind(x.as_slice())
            .bind(w_id)
            .fetch_all(db)
            .await?
            .into_iter()
            .filter_map(|x| ResultR::from_row(&x).ok().and_then(|x| x.result))
            .collect::<Vec<Json<Box<RawValue>>>>();
            to_raw_value(&rows)
        }
        JobResult::SingleJob(x) => sqlx::query(
            "SELECT result #> $3 as result FROM completed_job WHERE id = $1 AND workspace_id = $2",
        )
        .bind(x)
        .bind(w_id)
        .bind(
            json_path
                .map(|x| x.split(".").map(|x| x.to_string()).collect::<Vec<_>>())
                .unwrap_or_default(),
        )
        .fetch_optional(db)
        .await?
        .map(|r| {
            ResultR::from_row(&r)
                .ok()
                .and_then(|x| x.result.map(|x| x.0))
        })
        .flatten()
        .unwrap_or_else(|| to_raw_value(&serde_json::Value::Null)),
    };

    Ok(value)
}

#[async_recursion]
pub async fn get_result_by_id_from_original_flow(
    db: &Pool<Postgres>,
    w_id: &str,
    completed_flow_id: &Uuid,
    node_id: &str,
    json_path: Option<String>,
) -> error::Result<Box<RawValue>> {
    let flow_job = sqlx::query_as::<_, CompletedJob>(
        "SELECT * FROM completed_job WHERE id = $1 AND workspace_id = $2",
    )
    .bind(completed_flow_id)
    .bind(w_id)
    .fetch_optional(db)
    .await?;

    let flow_job = windmill_common::utils::not_found_if_none(
        flow_job,
        "Flow result by id in leaf jobs",
        format!("{}", completed_flow_id),
    )?;

    let mut leaf_jobs_for_flow = HashMap::<String, Uuid>::new();
    compute_leaf_jobs_for_completed_flow(
        db,
        w_id,
        flow_job.id,
        Some(flow_job.clone()),
        &mut leaf_jobs_for_flow,
    )
    .await?;

    tracing::debug!(
        "Fetching leaf jobs for flow {} : {:?}",
        flow_job.id,
        leaf_jobs_for_flow
    );
    if !leaf_jobs_for_flow.contains_key(&node_id.to_string()) {
        // if the flow is itself a restart flow, the step job might be from the upstream flow
        let restarted_from = windmill_common::utils::not_found_if_none(
            flow_job
                .parse_flow_status()
                .map(|status| status.restarted_from)
                .flatten(),
            "Flow result by id in leaf jobs",
            format!("{}", completed_flow_id),
        )?;
        return get_result_by_id_from_original_flow(
            db,
            w_id,
            &restarted_from.flow_job_id,
            node_id,
            json_path,
        )
        .await;
    }

    // if the job is in the leaf_jobs map, then fetch its result and return
    let leaf_job_uuid = leaf_jobs_for_flow.get(&node_id.to_string());
    let leaf_job_result = sqlx::query(
        "SELECT result #> $3 as result FROM completed_job WHERE id = $1 AND workspace_id = $2",
    )
    .bind(leaf_job_uuid)
    .bind(w_id)
    .bind(
        json_path
            .map(|x| x.split(".").map(|x| x.to_string()).collect::<Vec<_>>())
            .unwrap_or_default(),
    )
    .fetch_optional(db)
    .await?
    .map(|r| {
        ResultR::from_row(&r)
            .ok()
            .and_then(|x| x.result.map(|x| x.0))
    })
    .flatten()
    .unwrap_or_else(|| to_raw_value(&serde_json::Value::Null));
    return Ok(leaf_job_result);
}

#[async_recursion]
async fn compute_leaf_jobs_for_completed_flow(
    db: &Pool<Postgres>,
    w_id: &str,
    completed_flow_id: Uuid,
    completed_flow_row: Option<CompletedJob>, // if provided, will be used, otherwise completed_flow_id must be set and the job definition will be pulled from DB
    recursive_result: &mut HashMap<String, Uuid>,
) -> error::Result<()> {
    let flow_status = match completed_flow_row {
        Some(job) => job.parse_flow_status(),
        None => {
            let job_status_raw = sqlx::query_scalar!(
                "SELECT flow_status FROM completed_job WHERE id = $1 AND workspace_id = $2",
                completed_flow_id,
                w_id,
            )
            .fetch_one(db)
            .await?;
            job_status_raw
                .map(|raw| serde_json::from_value::<FlowStatus>(raw).ok())
                .flatten()
        }
    };

    let flow_job_status_modules = windmill_common::utils::not_found_if_none(
        flow_status.map(|fs| fs.modules),
        "Flow result by id in leaf jobs",
        format!("{}", completed_flow_id),
    )?;

    let children_jobs = sqlx::query_as::<_, (Uuid, JobKind)>(
        "SELECT id, job_kind FROM completed_job WHERE parent_job = $1 AND workspace_id = $2",
    )
    .bind(completed_flow_id)
    .bind(w_id)
    .fetch_all(db)
    .await?;

    for child_job in children_jobs {
        let child_job_id = child_job.0;
        let child_job_kind = child_job.1;
        match child_job_kind {
            JobKind::Script | JobKind::Preview | JobKind::Script_Hub => {
                // if is potentially a leaf job. Get its step_id from the initial flow definition and add it to the result map
                for module in &flow_job_status_modules {
                    if module.job().map(|id| id == child_job_id).unwrap_or(false) {
                        recursive_result.insert(module.id(), child_job_id);
                    }
                }
            }
            JobKind::Flow | JobKind::FlowPreview => {
                // Extract the leaf job for this flow and add them to the result map
                // TODO: we should exclude forloops nested flow here as it's non-sense to reference a result of a module inside a forloop from outside this forloop
                compute_leaf_jobs_for_completed_flow(
                    db,
                    w_id,
                    child_job_id,
                    None,
                    recursive_result,
                )
                .await?;
            }
            _ => {} // do nothing
        }
    }
    Ok(())
}

#[instrument(level = "trace", skip_all)]
pub async fn delete_job<'c, R: rsmq_async::RsmqConnection + Clone + Send>(
    mut tx: QueueTransaction<'c, R>,
    w_id: &str,
    job_id: Uuid,
) -> windmill_common::error::Result<QueueTransaction<'c, R>> {
    if *METRICS_ENABLED {
        QUEUE_DELETE_COUNT.inc();
    }
    let job_removed = sqlx::query_scalar!(
        "DELETE FROM queue WHERE workspace_id = $1 AND id = $2 RETURNING 1",
        w_id,
        job_id
    )
    .fetch_optional(&mut tx)
    .await;

    if let Err(job_removed) = job_removed {
        tracing::error!(
            "Job {job_id} could not be deleted: {job_removed}. This is not necessarily an error, as the job might have been deleted by another process such as in the case of cancelling"
        );
    } else {
        let job_removed = job_removed.unwrap().flatten().unwrap_or(0);
        if job_removed != 1 {
            tracing::error!("Job {job_id} could not be deleted, returned not 1: {job_removed}. This is not necessarily an error, as the job might have been deleted by another process such as in the case of cancelling");
        }
    }

    tracing::debug!("Job {job_id} deleted");
    Ok(tx)
}

pub async fn job_is_complete(db: &DB, id: Uuid, w_id: &str) -> error::Result<bool> {
    Ok(sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM completed_job WHERE id = $1 AND workspace_id = $2)",
        id,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false))
}

pub async fn get_queued_job<'c>(
    id: Uuid,
    w_id: &str,
    tx: &mut Transaction<'c, Postgres>,
) -> error::Result<Option<QueuedJob>> {
    let r = sqlx::query(
        "SELECT *
            FROM queue WHERE id = $1 AND workspace_id = $2",
    )
    .bind(id)
    .bind(w_id)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(row) = r {
        Ok(Some(QueuedJob::from_row(&row)?.to_owned()))
    } else {
        Ok(None)
    }
}

pub enum PushIsolationLevel<'c, R: rsmq_async::RsmqConnection + Send + 'c> {
    IsolatedRoot(DB, Option<R>),
    Isolated(UserDB, Authed, Option<R>),
    Transaction(QueueTransaction<'c, R>),
}

#[macro_export]
macro_rules! fetch_scalar_isolated {
    ( $query:expr, $tx:expr) => {
        match $tx {
            PushIsolationLevel::IsolatedRoot(db, rmsq) => {
                let r = $query.fetch_optional(&db).await;
                $tx = PushIsolationLevel::IsolatedRoot(db, rmsq);
                r
            }
            PushIsolationLevel::Isolated(db, user, rsmq) => {
                let mut ntx = db.clone().begin(&user).await?;
                let r = $query.fetch_optional(&mut *ntx).await;
                $tx = PushIsolationLevel::Isolated(db, user, rsmq);
                r
            }
            PushIsolationLevel::Transaction(mut tx) => {
                let r = $query.fetch_optional(&mut tx).await;
                $tx = PushIsolationLevel::Transaction(tx);
                r
            }
        }
    };
}

use sqlx::types::JsonRawValue;

#[derive(Serialize)]
pub struct PushArgs<T> {
    #[serde(flatten)]
    pub extra: HashMap<String, Box<RawValue>>,
    #[serde(flatten)]
    pub args: Json<T>,
}

#[derive(Deserialize)]
pub struct RequestQuery {
    pub raw: Option<bool>,
    pub include_header: Option<String>,
}

#[axum::async_trait]
impl<S> FromRequest<S, axum::body::Body> for PushArgs<HashMap<String, Box<RawValue>>>
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(
        req: Request<axum::body::Body>,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let (content_type, mut extra, use_raw) = {
            let headers_map = req.headers();
            let content_type_header = headers_map.get(CONTENT_TYPE);
            let content_type = content_type_header.and_then(|value| value.to_str().ok());
            let query = Query::<RequestQuery>::try_from_uri(req.uri()).unwrap().0;
            let raw = query.raw.as_ref().is_some_and(|x| *x);
            (
                content_type,
                build_extra(&headers_map, query.include_header),
                raw,
            )
        };

        if content_type.is_none() || content_type.unwrap().starts_with("application/json") {
            let bytes = Bytes::from_request(req, _state)
                .await
                .map_err(IntoResponse::into_response)?;
            let str = String::from_utf8(bytes.to_vec())
                .map_err(|e| Error::BadRequest(format!("invalid utf8: {}", e)).into_response())?;

            if use_raw {
                extra.insert("raw_string".to_string(), to_raw_value(&str));
            }

            let wrap_body = str.len() > 0 && str.chars().next().unwrap() != '{';

            if wrap_body {
                let args = serde_json::from_str::<Option<Box<RawValue>>>(&str)
                    .map_err(|e| Error::BadRequest(format!("invalid json: {}", e)).into_response())?
                    .unwrap_or_else(|| to_raw_value(&serde_json::Value::Null));
                let mut hm = HashMap::new();
                hm.insert("body".to_string(), args);
                Ok(PushArgs { extra, args: Json(hm) })
            } else {
                let hm = serde_json::from_str::<Option<HashMap<String, Box<JsonRawValue>>>>(&str)
                    .map_err(|e| Error::BadRequest(format!("invalid json: {}", e)).into_response())?
                    .unwrap_or_else(HashMap::new);
                Ok(PushArgs { extra, args: Json(hm) })
            }
        } else if content_type
            .unwrap()
            .starts_with("application/x-www-form-urlencoded")
        {
            let Form(payload): Form<Option<HashMap<String, Box<RawValue>>>> =
                req.extract().await.map_err(IntoResponse::into_response)?;
            return Ok(PushArgs {
                extra: HashMap::new(),
                args: Json(payload.unwrap_or_else(HashMap::new)),
            });
        } else {
            Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response())
        }
    }
}

lazy_static::lazy_static! {
    static ref INCLUDE_HEADERS: Vec<String> = std::env::var("INCLUDE_HEADERS")
        .ok().map(|x| x
        .split(',')
        .map(|s| s.to_string())
        .collect()).unwrap_or_default();
}

pub fn build_extra(
    headers: &HeaderMap,
    include_header: Option<String>,
) -> HashMap<String, Box<RawValue>> {
    let mut args = HashMap::new();
    let whitelist = include_header
        .map(|s| s.split(",").map(|s| s.to_string()).collect::<Vec<_>>())
        .unwrap_or_default();

    whitelist
        .iter()
        .chain(INCLUDE_HEADERS.iter())
        .for_each(|h| {
            if let Some(v) = headers.get(h) {
                args.insert(
                    h.to_string().to_lowercase().replace('-', "_"),
                    to_raw_value(&v.to_str().unwrap().to_string()),
                );
            }
        });
    args
}

impl PushArgs<HashMap<String, Box<RawValue>>> {
    pub fn empty() -> Self {
        PushArgs { extra: HashMap::new(), args: Json(HashMap::new()) }
    }
}

pub fn empty_args() -> Box<RawValue> {
    return JsonRawValue::from_string("{}".to_string()).unwrap();
}

impl From<HashMap<String, Box<JsonRawValue>>> for PushArgs<HashMap<String, Box<JsonRawValue>>> {
    fn from(value: HashMap<String, Box<JsonRawValue>>) -> Self {
        PushArgs { extra: HashMap::new(), args: Json(value) }
    }
}

// impl<T> From<PushArgsInner<T>> for PushArgs<T> {
//     fn from(value: PushArgsInner<T>) -> Self {
//         PushArgs::Unwrapped(value)
//     }
// }

// #[instrument(level = "trace", skip_all)]
pub async fn push<'c, T: Serialize + Send + Sync, R: rsmq_async::RsmqConnection + Send + 'c>(
    _db: &Pool<Postgres>,
    mut tx: PushIsolationLevel<'c, R>,
    workspace_id: &str,
    job_payload: JobPayload,
    args: T,
    user: &str,
    email: &str,
    permissioned_as: String,
    scheduled_for_o: Option<chrono::DateTime<chrono::Utc>>,
    schedule_path: Option<String>,
    parent_job: Option<Uuid>,
    root_job: Option<Uuid>,
    job_id: Option<Uuid>,
    is_flow_step: bool,
    mut same_worker: bool,
    pre_run_error: Option<&windmill_common::error::Error>,
    visible_to_owner: bool,
    mut tag: Option<String>,
    custom_timeout: Option<i32>,
    flow_step_id: Option<String>,
    priority_override: Option<i16>,
) -> Result<(Uuid, QueueTransaction<'c, R>), Error> {
    #[cfg(feature = "enterprise")]
    if *CLOUD_HOSTED {
        let premium_workspace = *CLOUD_HOSTED
            && sqlx::query_scalar!("SELECT premium FROM workspace WHERE id = $1", workspace_id)
                .fetch_one(_db)
                .await
                .map_err(|e| {
                    Error::InternalErr(format!("fetching if {workspace_id} is premium: {e}"))
                })?;

        // we track only non flow steps
        let usage = if !matches!(
            job_payload,
            JobPayload::Flow(_) | JobPayload::RawFlow { .. }
        ) {
            sqlx::query_scalar!(
                    "INSERT INTO usage (id, is_workspace, month_, usage) 
                    VALUES ($1, $2, EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date), 0) 
                    ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + 1 
                    RETURNING usage.usage",
                    if premium_workspace { workspace_id } else { email },
                    premium_workspace
                )
                .fetch_one(_db)
                .await
                .map_err(|e| Error::InternalErr(format!("updating usage: {e}")))?
        } else if *CLOUD_HOSTED && !premium_workspace {
            sqlx::query_scalar!(
                "
        SELECT usage.usage + 1 FROM usage 
        WHERE is_workspace = false AND
     month_ = EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date)
     AND id = $1",
                email
            )
            .fetch_optional(_db)
            .await?
            .flatten()
            .unwrap_or(0)
        } else {
            0
        };

        if *CLOUD_HOSTED && !premium_workspace {
            let is_super_admin =
                sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
                    .fetch_optional(_db)
                    .await?
                    .unwrap_or(false);

            if !is_super_admin {
                if usage > MAX_FREE_EXECS
                    && !matches!(job_payload, JobPayload::Dependencies { .. })
                    && !matches!(job_payload, JobPayload::FlowDependencies { .. })
                    && !matches!(job_payload, JobPayload::AppDependencies { .. })
                {
                    return Err(error::Error::BadRequest(format!(
                    "User {email} has exceeded the free usage limit of {MAX_FREE_EXECS} that applies outside of premium workspaces."
                )));
                }
                let in_queue =
                    sqlx::query_scalar!("SELECT COUNT(id) FROM queue WHERE email = $1", email)
                        .fetch_one(_db)
                        .await?
                        .unwrap_or(0);

                if in_queue > MAX_FREE_EXECS.into() {
                    return Err(error::Error::BadRequest(format!(
                    "User {email} has exceeded the jobs in queue limit of {MAX_FREE_EXECS} that applies outside of premium workspaces."
                )));
                }

                let concurrent_runs = sqlx::query_scalar!(
                    "SELECT COUNT(id) FROM queue WHERE running = true AND email = $1",
                    email
                )
                .fetch_one(_db)
                .await?
                .unwrap_or(0);

                if concurrent_runs > MAX_FREE_CONCURRENT_RUNS.into() {
                    return Err(error::Error::BadRequest(format!(
                    "User {email} has exceeded the concurrent runs limit of {MAX_FREE_CONCURRENT_RUNS} that applies outside of premium workspaces."
                )));
                }
            }
        }
    }

    let (
        script_hash,
        script_path,
        raw_code_tuple,
        job_kind,
        mut raw_flow,
        flow_status,
        language,
        concurrent_limit,
        concurrency_time_window_s,
        cache_ttl,
        dedicated_worker,
        low_level_priority,
    ) = match job_payload {
        JobPayload::ScriptHash {
            hash,
            path,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            language,
            dedicated_worker,
            priority,
        } => (
            Some(hash.0),
            Some(path),
            None,
            JobKind::Script,
            None,
            None,
            Some(language),
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            dedicated_worker,
            priority,
        ),
        JobPayload::ScriptHub { path } => {
            (
                None,
                Some(path),
                None,
                // Some((script.content, script.lockfile)),
                JobKind::Script_Hub,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
        }
        JobPayload::Code(RawCode {
            content,
            path,
            language,
            lock,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
        }) => (
            None,
            path,
            Some((content, lock)),
            JobKind::Preview,
            None,
            None,
            Some(language),
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            None,
            None,
        ),
        JobPayload::Dependencies { hash, dependencies, language, path } => (
            Some(hash.0),
            Some(path),
            Some((dependencies, None)),
            JobKind::Dependencies,
            None,
            None,
            Some(language),
            None,
            None,
            None,
            None,
            None,
        ),
        JobPayload::FlowDependencies { path } => {
            let value_json = fetch_scalar_isolated!(
                sqlx::query_scalar!(
                    "SELECT value FROM flow WHERE path = $1 AND workspace_id = $2",
                    path,
                    workspace_id
                ),
                tx
            )?
            .ok_or_else(|| Error::InternalErr(format!("not found flow at path {:?}", path)))?;
            let value = serde_json::from_value::<FlowValue>(value_json).map_err(|err| {
                Error::InternalErr(format!(
                    "could not convert json to flow for {path}: {err:?}"
                ))
            })?;
            (
                None,
                Some(path),
                None,
                JobKind::FlowDependencies,
                Some(value.clone()),
                Some(FlowStatus::new(&value)), // this is a new flow being pushed, flow_status is set to flow_value
                None,
                None,
                None,
                None,
                None,
                None,
            )
        }
        JobPayload::AppDependencies { path, version } => (
            Some(version),
            Some(path),
            None,
            JobKind::AppDependencies,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ),
        JobPayload::RawFlow { value, path, restarted_from } => {
            let flow_status: FlowStatus = match restarted_from {
                Some(restarted_from_val) => {
                    let (_, _, step_n, truncated_modules, _) = restarted_flows_resolution(
                        _db,
                        workspace_id,
                        restarted_from_val.flow_job_id,
                        restarted_from_val.step_id.as_str(),
                    )
                    .await?;
                    FlowStatus {
                        step: step_n,
                        modules: truncated_modules,
                        // failure_module is reset
                        failure_module: FlowStatusModuleWParent {
                            parent_module: None,
                            module_status: FlowStatusModule::WaitingForPriorSteps {
                                id: "failure".to_string(),
                            },
                        },
                        // retry status is reset
                        retry: RetryStatus { fail_count: 0, failed_jobs: vec![] },
                        // TODO: for now, flows with approval conditions aren't supported for restart
                        approval_conditions: None,
                        restarted_from: Some(RestartedFrom {
                            flow_job_id: restarted_from_val.flow_job_id,
                            step_id: restarted_from_val.step_id,
                        }),
                    }
                }
                _ => FlowStatus::new(&value), // this is a new flow being pushed, flow_status is set to flow_value
            };
            (
                None,
                path,
                None,
                JobKind::FlowPreview,
                Some(value.clone()),
                Some(flow_status),
                None,
                value.concurrent_limit.clone(),
                value.concurrency_time_window_s,
                value.cache_ttl.map(|x| x as i32),
                None,
                value.priority,
            )
        }
        JobPayload::Flow(flow) => {
            let value_json = fetch_scalar_isolated!(
                sqlx::query_scalar!(
                    "SELECT value FROM flow WHERE path = $1 AND workspace_id = $2",
                    flow,
                    workspace_id
                ),
                tx
            )?
            .ok_or_else(|| Error::InternalErr(format!("not found flow at path {:?}", flow)))?;
            let value = serde_json::from_value::<FlowValue>(value_json).map_err(|err| {
                Error::InternalErr(format!(
                    "could not convert json to flow for {flow}: {err:?}"
                ))
            })?;
            (
                None,
                Some(flow),
                None,
                JobKind::Flow,
                Some(value.clone()),
                Some(FlowStatus::new(&value)), // this is a new flow being pushed, flow_status is set to flow_value
                None,
                value.concurrent_limit.clone(),
                value.concurrency_time_window_s,
                value.cache_ttl.map(|x| x as i32),
                None,
                value.priority,
            )
        }
        JobPayload::RestartedFlow { completed_job_id, step_id } => {
            let (flow_path, raw_flow, step_n, truncated_modules, priority) =
                restarted_flows_resolution(_db, workspace_id, completed_job_id, step_id.as_str())
                    .await?;
            let restarted_flow_status = FlowStatus {
                step: step_n,
                modules: truncated_modules,
                // failure_module is reset
                failure_module: FlowStatusModuleWParent {
                    parent_module: None,
                    module_status: FlowStatusModule::WaitingForPriorSteps {
                        id: "failure".to_string(),
                    },
                },
                // retry status is reset
                retry: RetryStatus { fail_count: 0, failed_jobs: vec![] },
                // TODO: for now, flows with approval conditions aren't supported for restart
                approval_conditions: None,
                restarted_from: Some(RestartedFrom { flow_job_id: completed_job_id, step_id }),
            };
            (
                None,
                flow_path,
                None,
                JobKind::Flow,
                Some(raw_flow.clone()),
                Some(restarted_flow_status),
                None,
                raw_flow.concurrent_limit,
                raw_flow.concurrency_time_window_s,
                raw_flow.cache_ttl.map(|x| x as i32),
                None,
                priority,
            )
        }
        JobPayload::Identity => (
            None,
            None,
            None,
            JobKind::Identity,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ),
        JobPayload::Noop => (
            None,
            None,
            None,
            JobKind::Noop,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ),
    };

    let final_priority: Option<i16>;
    #[cfg(not(feature = "enterprise"))]
    {
        // priority is only available on EE. Do not compute it on CE
        final_priority = None;
    }
    #[cfg(feature = "enterprise")]
    {
        final_priority = if *CLOUD_HOSTED {
            // for cloud hosted instance, priority queues is disabled
            None
        } else if priority_override.is_some() {
            priority_override
        } else {
            // else it takes the priority defined at the script/flow level, if it's a script or flow
            low_level_priority
        }; // else it remains empty, i.e. no priority
    }

    let is_running = same_worker;
    if let Some(flow) = raw_flow.as_ref() {
        same_worker = same_worker || flow.same_worker;

        for module in flow.modules.iter() {
            if let Some(retry) = &module.retry {
                if retry.max_attempts() > MAX_RETRY_ATTEMPTS {
                    Err(Error::BadRequest(format!(
                        "retry attempts exceeds the maximum of {MAX_RETRY_ATTEMPTS}"
                    )))?
                }

                if matches!(retry.max_interval(), Some(interval) if interval > MAX_RETRY_INTERVAL) {
                    let max = MAX_RETRY_INTERVAL.as_secs();
                    Err(Error::BadRequest(format!(
                        "retry interval exceeds the maximum of {max} seconds"
                    )))?
                }
            }
        }

        // If last module has a sleep or suspend, we insert a virtual identity module
        if flow.modules.len() > 0
            && (flow.modules[flow.modules.len() - 1].sleep.is_some()
                || flow.modules[flow.modules.len() - 1].suspend.is_some())
        {
            let mut modules = flow.modules.clone();
            modules.push(FlowModule {
                id: format!("{}-v", flow.modules[flow.modules.len() - 1].id),
                value: FlowModuleValue::Identity,
                stop_after_if: None,
                summary: Some(
                    "Virtual module needed for suspend/sleep when last module".to_string(),
                ),
                mock: None,
                retry: None,
                sleep: None,
                suspend: None,
                cache_ttl: None,
                timeout: None,
                priority: None,
            });
            raw_flow = Some(FlowValue { modules, ..flow.clone() });
        }
    }

    let (raw_code, raw_lock) = raw_code_tuple
        .map(|e| (Some(e.0), e.1))
        .unwrap_or_else(|| (None, None));

    let tag = if dedicated_worker.is_some_and(|x| x) {
        format!(
            "{}:{}",
            workspace_id,
            script_path.clone().expect("dedicated script has a path")
        )
    } else if job_kind == JobKind::Script_Hub {
        "hub".to_string()
    } else {
        if tag == Some("".to_string()) {
            tag = None;
        }
        let default = || {
            if job_kind == JobKind::Flow
                || job_kind == JobKind::FlowPreview
                || job_kind == JobKind::Identity
            {
                "flow".to_string()
            } else if job_kind == JobKind::Dependencies || job_kind == JobKind::FlowDependencies {
                "dependency".to_string()
            } else {
                "deno".to_string()
            }
        };
        tag.map(|x| x.as_str().replace("$workspace", workspace_id).to_string())
            .unwrap_or_else(|| {
                language
                    .as_ref()
                    .map(|x| x.as_str().to_string())
                    .unwrap_or_else(default)
            })
    };

    let mut tx = match tx {
        PushIsolationLevel::Isolated(user_db, authed, rsmq) => {
            (rsmq, user_db.begin(&authed).await?).into()
        }
        PushIsolationLevel::IsolatedRoot(db, rsmq) => (rsmq, db.begin().await?).into(),
        PushIsolationLevel::Transaction(tx) => tx,
    };

    let job_id: Uuid = if let Some(job_id) = job_id {
        let conflicting_id = sqlx::query_scalar!(
            "SELECT 1 FROM queue WHERE id = $1 UNION ALL select 1 FROM completed_job WHERE id = $1",
            job_id
        )
        .fetch_optional(&mut tx)
        .await?;

        if conflicting_id.is_some() {
            return Err(Error::BadRequest(format!(
                "Job with id {job_id} already exists"
            )));
        }

        job_id
    } else {
        Ulid::new().into()
    };

    let uuid = sqlx::query_scalar!(
        "INSERT INTO queue
            (workspace_id, id, running, parent_job, created_by, permissioned_as, scheduled_for, 
                script_hash, script_path, raw_code, raw_lock, args, job_kind, schedule_path, raw_flow, \
                flow_status, is_flow_step, language, started_at, same_worker, pre_run_error, email, \
                visible_to_owner, root_job, tag, concurrent_limit, concurrency_time_window_s, timeout, \
                flow_step_id, cache_ttl, priority)
            VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7, now()), $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, CASE WHEN $3 THEN now() END, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30) \
         RETURNING id",
        workspace_id,
        job_id,
        is_running,
        parent_job,
        user,
        permissioned_as,
        scheduled_for_o,
        script_hash,
        script_path.clone(),
        raw_code,
        raw_lock,
        Json(args) as Json<T>,
        job_kind.clone() as JobKind,
        schedule_path,
        raw_flow.map(|f| serde_json::json!(f)),
        flow_status.map(|f| serde_json::json!(f)),
        is_flow_step,
        language as Option<ScriptLang>,
        same_worker,
        pre_run_error.map(|e| e.to_string()),
        email,
        visible_to_owner,
        root_job,
        tag,
        concurrent_limit,
        concurrency_time_window_s,
        custom_timeout,
        flow_step_id,
        cache_ttl,
        final_priority,
    )
    .fetch_one(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("Could not insert into queue {job_id}: {e}")))?;

    // TODO: technically the job isn't queued yet, as the transaction can be rolled back. Should be solved when moving these metrics to the queue abstraction.
    if *METRICS_ENABLED {
        QUEUE_PUSH_COUNT.inc();
    }

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
            JobKind::Identity => "jobs.run.identity",
            JobKind::Noop => "jobs.run.noop",
            JobKind::FlowDependencies => "jobs.run.flow_dependencies",
            JobKind::AppDependencies => "jobs.run.app_dependencies",
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
        .instrument(tracing::info_span!("job_run", email = &email))
        .await?;
    }
    if let Some(ref mut rsmq) = tx.rsmq {
        rsmq.send_message(job_id.to_bytes_le().to_vec(), scheduled_for_o, tag);
    }

    Ok((uuid, tx))
}

pub fn canceled_job_to_result(job: &QueuedJob) -> serde_json::Value {
    let reason = job
        .canceled_reason
        .as_deref()
        .unwrap_or_else(|| "no reason given");
    let canceler = job.canceled_by.as_deref().unwrap_or_else(|| "unknown");
    serde_json::json!({"message": format!("Job canceled: {reason} by {canceler}"), "name": "Canceled", "reason": reason, "canceler": canceler})
}

async fn restarted_flows_resolution(
    db: &Pool<Postgres>,
    workspace_id: &str,
    completed_flow_id: Uuid,
    restart_step_id: &str,
) -> Result<
    (
        Option<String>,
        FlowValue,
        i32,
        Vec<FlowStatusModule>,
        Option<i16>,
    ),
    Error,
> {
    let completed_job = sqlx::query_as::<_, CompletedJob>(
        "SELECT * FROM completed_job WHERE id = $1 and workspace_id = $2",
    )
    .bind(completed_flow_id)
    .bind(workspace_id)
    .fetch_one(db) // TODO: should we try to use the passed-in `tx` here?
    .await
    .map_err(|err| {
        Error::InternalErr(format!(
            "completed job not found for UUID {} in workspace {}: {}",
            completed_flow_id, workspace_id, err
        ))
    })?;

    let raw_flow = completed_job
        .parse_raw_flow()
        .ok_or(Error::InternalErr(format!(
            "Unable to parse raw definition for job {} in workspace {}",
            completed_flow_id, workspace_id,
        )))?;
    let flow_status = completed_job
        .parse_flow_status()
        .ok_or(Error::InternalErr(format!(
            "Unable to parse flow status for job {} in workspace {}",
            completed_flow_id, workspace_id,
        )))?;

    let mut step_n = 0;
    let mut dependent_module = false;
    let mut truncated_modules: Vec<FlowStatusModule> = vec![];
    for module in flow_status.modules {
        if module.id() == restart_step_id || dependent_module {
            // if the module ID is the one we want to restart the flow at, or if it's past it in the flow,
            // set the module as WaitingForPriorSteps as it needs to be re-run
            truncated_modules.push(FlowStatusModule::WaitingForPriorSteps { id: module.id() });
            dependent_module = true;
        } else {
            // else we simply "transfer" the module from the completed flow to the new one if it's a success
            step_n = step_n + 1;
            match module.clone() {
                FlowStatusModule::Success {
                    id: _,
                    job: _,
                    flow_jobs: _,
                    branch_chosen: _,
                    approvers: _,
                } => Ok(truncated_modules.push(module)),
                _ => Err(Error::InternalErr(format!(
                    "Flow cannot be restarted from a non successful module",
                ))),
            }?;
        }
    }
    if !dependent_module {
        // step not found in flow.
        return Err(Error::InternalErr(format!(
            "Flow cannot be restarted from step {} as it could not be found.",
            restart_step_id
        )));
    }

    return Ok((
        completed_job.script_path,
        raw_flow,
        step_n,
        truncated_modules,
        completed_job.priority,
    ));
}
