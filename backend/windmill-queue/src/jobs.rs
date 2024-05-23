/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    sync::Arc,
    vec,
};

use anyhow::Context;
use async_recursion::async_recursion;
use axum::{
    body::Bytes,
    extract::{FromRequest, FromRequestParts, Query},
    http::{request::Parts, Request, Uri},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Duration, Utc};
use itertools::Itertools;
#[cfg(feature = "prometheus")]
use prometheus::IntCounter;
use regex::Regex;
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
use tokio::{sync::RwLock, time::sleep};
use tracing::{instrument, Instrument};
use ulid::Ulid;
use uuid::Uuid;
use windmill_audit::audit_ee::{audit_log, AuditAuthor};
use windmill_audit::ActionKind;
#[cfg(not(feature = "enterprise"))]
use windmill_common::worker::PriorityTags;
use windmill_common::{
    db::{Authed, UserDB},
    error::{self, to_anyhow, Error},
    flow_status::{
        BranchAllStatus, FlowCleanupModule, FlowStatus, FlowStatusModule, FlowStatusModuleWParent,
        Iterator, JobResult, RestartedFrom, RetryStatus, MAX_RETRY_ATTEMPTS, MAX_RETRY_INTERVAL,
    },
    flows::{
        add_virtual_items_if_necessary, FlowModule, FlowModuleValue, FlowValue, InputTransform,
    },
    jobs::{
        get_payload_tag_from_prefixed_path, CompletedJob, JobKind, JobPayload, QueuedJob, RawCode,
    },
    oauth2::WORKSPACE_SLACK_BOT_TOKEN_PATH,
    schedule::Schedule,
    scripts::{ScriptHash, ScriptLang},
    users::{SUPERADMIN_NOTIFICATION_EMAIL, SUPERADMIN_SECRET_EMAIL},
    utils::{not_found_if_none, report_critical_error},
    worker::{to_raw_value, DEFAULT_TAGS_PER_WORKSPACE, NO_LOGS, WORKER_CONFIG},
    BASE_URL, DB, METRICS_ENABLED,
};

#[cfg(feature = "cloud")]
use windmill_common::users::SUPERADMIN_SYNC_EMAIL;

#[cfg(feature = "enterprise")]
use windmill_common::worker::CLOUD_HOSTED;

use crate::{
    schedule::{get_schedule_opt, push_scheduled_job},
    QueueTransaction,
};

#[cfg(feature = "prometheus")]
lazy_static::lazy_static! {

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

    pub static ref WORKER_EXECUTION_FAILED: Arc<RwLock<HashMap<String, IntCounter>>> = Arc::new(RwLock::new(HashMap::new()));

}

lazy_static::lazy_static! {
    pub static ref HTTP_CLIENT: Client = reqwest::ClientBuilder::new()
        .user_agent("windmill/beta")
        .timeout(std::time::Duration::from_secs(20))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build().unwrap();


}

#[cfg(feature = "cloud")]
const MAX_FREE_EXECS: i32 = 1000;
#[cfg(feature = "cloud")]
const MAX_FREE_CONCURRENT_RUNS: i32 = 30;

const ERROR_HANDLER_USERNAME: &str = "error_handler";
const SCHEDULE_ERROR_HANDLER_USERNAME: &str = "schedule_error_handler";
const SCHEDULE_RECOVERY_HANDLER_USERNAME: &str = "schedule_recovery_handler";
const ERROR_HANDLER_USER_GROUP: &str = "g/error_handler";
const ERROR_HANDLER_USER_EMAIL: &str = "error_handler@windmill.dev";
const SCHEDULE_ERROR_HANDLER_USER_EMAIL: &str = "schedule_error_handler@windmill.dev";
const SCHEDULE_RECOVERY_HANDLER_USER_EMAIL: &str = "schedule_recovery_handler@windmill.dev";

#[derive(Clone, Debug)]
pub struct CanceledBy {
    pub username: Option<String>,
    pub reason: Option<String>,
}

pub async fn cancel_single_job<'c>(
    username: &str,
    reason: Option<String>,
    job_running: &QueuedJob,
    w_id: &str,
    mut tx: Transaction<'c, Postgres>,
    db: &Pool<Postgres>,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    force_cancel: bool,
) -> error::Result<(Transaction<'c, Postgres>, Option<Uuid>)> {
    if ((job_running.running || job_running.root_job.is_some()) || (job_running.is_flow()))
        && !force_cancel
    {
        let id = sqlx::query_scalar!(
            "UPDATE queue SET canceled = true, canceled_by = $1, canceled_reason = $2, scheduled_for = now(), suspend = 0 WHERE id = $3 AND workspace_id = $4 RETURNING id",
            username,
            reason,
            job_running.id,
            w_id
        )
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(id) = id {
            tracing::info!("Soft cancelling job {}", id);
        }
    } else {
        let reason: String = reason
            .clone()
            .unwrap_or_else(|| "unexplicited reasons".to_string());
        let e = serde_json::json!({"message": format!("Job canceled: {reason} by {username}"), "name": "Canceled", "reason": reason, "canceler": username});
        append_logs(
            job_running.id,
            w_id.to_string(),
            format!("canceled by {username}: (force cancel: {force_cancel})"),
            db,
        )
        .await;
        let add_job = add_completed_job_error(
            &db,
            job_running,
            job_running.mem_peak.unwrap_or(0),
            Some(CanceledBy { username: Some(username.to_string()), reason: Some(reason) }),
            e,
            rsmq.clone(),
            "server",
            false,
        )
        .await;

        if let Err(e) = add_job {
            tracing::error!("Failed to add canceled job: {}", e);
        }
    }
    if let Some(mut rsmq) = rsmq.clone() {
        rsmq.change_message_visibility(&job_running.tag, &job_running.id.to_string(), 0)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
    }

    Ok((tx, Some(job_running.id)))
}

pub async fn cancel_job<'c>(
    username: &str,
    reason: Option<String>,
    id: Uuid,
    w_id: &str,
    mut tx: Transaction<'c, Postgres>,
    db: &Pool<Postgres>,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
    force_cancel: bool,
) -> error::Result<(Transaction<'c, Postgres>, Option<Uuid>)> {
    let job = get_queued_job_tx(id, &w_id, &mut tx).await?;

    if job.is_none() {
        return Ok((tx, None));
    }

    let job = job.unwrap();

    // get all children
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

    let (ntx, _) = cancel_single_job(
        username,
        reason.clone(),
        &job,
        w_id,
        tx,
        db,
        rsmq.clone(),
        force_cancel,
    )
    .await?;
    tx = ntx;

    // soft cancel parent if force cancel and job is a flow step
    if force_cancel && job.is_flow_step {
        if let Some(parent_job_id) = job.parent_job {
            let job = get_queued_job_tx(parent_job_id, &w_id, &mut tx).await?;
            if let Some(job) = job {
                let (ntx, _) = cancel_single_job(
                    username,
                    reason.clone(),
                    &job,
                    w_id,
                    tx,
                    db,
                    rsmq.clone(),
                    false,
                )
                .await?;
                tx = ntx;
            }
        }
    }

    // cancel children
    for job_id in jobs_to_cancel {
        let job = get_queued_job_tx(job_id, &w_id, &mut tx).await?;

        if let Some(job) = job {
            let (ntx, _) = cancel_single_job(
                username,
                reason.clone(),
                &job,
                w_id,
                tx,
                db,
                rsmq.clone(),
                force_cancel,
            )
            .await?;
            tx = ntx;
        }
    }
    Ok((tx, Some(id)))
}

/* TODO retry this? */
#[tracing::instrument(level = "trace", skip_all)]
pub async fn append_logs(
    job_id: uuid::Uuid,
    workspace: impl AsRef<str>,
    logs: impl AsRef<str>,
    db: impl Borrow<Pool<Postgres>>,
) {
    if logs.as_ref().is_empty() {
        return;
    }

    if *NO_LOGS {
        tracing::info!("NO LOGS [{job_id}]: {}", logs.as_ref());
        return;
    }
    if let Err(err) = sqlx::query!(
        "INSERT INTO job_logs (logs, job_id, workspace_id) VALUES ($1, $2, $3) ON CONFLICT (job_id) DO UPDATE SET logs = concat(job_logs.logs, $1::text)",
        logs.as_ref(),
        job_id,
        workspace.as_ref(),
    )
    .execute(db.borrow())
    .await
    {
        tracing::error!(%job_id, %err, "error updating logs for large_log job {job_id}: {err}");
    }
}

pub async fn cancel_persistent_script_jobs<'c>(
    username: &str,
    reason: Option<String>,
    script_path: &str,
    w_id: &str,
    db: &Pool<Postgres>,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
) -> error::Result<Vec<Uuid>> {
    // There can be only one perpetual script run per 10 seconds window. We execute the cancel twice with 5s interval
    // to avoid the case of a job _about to be restarted_ when the first cancel is run.
    let cancelled_job_ids_first_batch = cancel_persistent_script_jobs_internal(
        username,
        reason.clone(),
        script_path,
        w_id,
        db,
        rsmq.clone(),
    )
    .await?;
    sleep(std::time::Duration::from_secs(5)).await;
    let cancelled_job_ids_second_batch = cancel_persistent_script_jobs_internal(
        username,
        reason.clone(),
        script_path,
        w_id,
        db,
        rsmq.clone(),
    )
    .await?;
    return Ok(cancelled_job_ids_first_batch
        .into_iter()
        .chain(cancelled_job_ids_second_batch)
        .collect_vec());
}

async fn cancel_persistent_script_jobs_internal<'c>(
    username: &str,
    reason: Option<String>,
    script_path: &str,
    w_id: &str,
    db: &Pool<Postgres>,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
) -> error::Result<Vec<Uuid>> {
    let mut tx = db.begin().await?;

    // we could have retrieved the job IDs in the first query where we retrieve the hashes, but just in case a job was inserted in the queue right in-between the two above query, we re-do the fetch here
    let jobs_to_cancel = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM queue WHERE workspace_id = $1 AND script_path = $2 AND canceled = false",
    )
    .bind(w_id)
    .bind(script_path)
    .fetch_all(&mut *tx)
    .await?;

    // Then we cancel all the jobs currently in the queue, one by one
    for queued_job_id in jobs_to_cancel.clone() {
        let (new_tx, _) = cancel_job(
            username,
            reason.clone(),
            queued_job_id,
            w_id,
            tx,
            db,
            rsmq.clone(),
            false,
        )
        .await?;
        tx = new_tx;
    }
    tx.commit().await?;

    return Ok(jobs_to_cancel);
}

#[derive(Serialize, Debug)]
pub struct WrappedError {
    pub error: serde_json::Value,
}

pub trait ValidableJson {
    fn is_valid_json(&self) -> bool;
}

impl ValidableJson for WrappedError {
    fn is_valid_json(&self) -> bool {
        true
    }
}

impl ValidableJson for Box<RawValue> {
    fn is_valid_json(&self) -> bool {
        !self.get().is_empty()
    }
}

impl ValidableJson for serde_json::Value {
    fn is_valid_json(&self) -> bool {
        true
    }
}

impl<T: ValidableJson> ValidableJson for Json<T> {
    fn is_valid_json(&self) -> bool {
        self.0.is_valid_json()
    }
}

pub async fn register_metric<T, F, F2, R>(
    l: &Arc<RwLock<HashMap<String, T>>>,
    s: &str,
    fnew: F2,
    f: F,
) -> Option<R>
where
    F: FnOnce(&T) -> R,
    F2: FnOnce(&str) -> (T, R),
{
    if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        let lock = l.read().await;

        let counter = lock.get(s);
        if let Some(counter) = counter {
            let r = f(counter);
            drop(lock);
            Some(r)
        } else {
            drop(lock);
            let (metric, r) = fnew(s);
            let mut m = l.write().await;
            (*m).insert(s.to_string(), metric);
            Some(r)
        }
    } else {
        None
    }
}

#[instrument(level = "trace", skip_all)]
pub async fn add_completed_job_error<R: rsmq_async::RsmqConnection + Clone + Send>(
    db: &Pool<Postgres>,
    queued_job: &QueuedJob,
    mem_peak: i32,
    canceled_by: Option<CanceledBy>,
    e: serde_json::Value,
    rsmq: Option<R>,
    _worker_name: &str,
    flow_is_done: bool,
) -> Result<WrappedError, Error> {
    #[cfg(feature = "prometheus")]
    register_metric(
        &WORKER_EXECUTION_FAILED,
        &queued_job.tag,
        |s| {
            let counter = prometheus::register_int_counter!(prometheus::Opts::new(
                "worker_execution_failed",
                "Number of jobs having failed"
            )
            .const_label("name", _worker_name)
            .const_label("tag", s))
            .expect("register prometheus metric");
            counter.inc();
            (counter, ())
        },
        |c| c.inc(),
    )
    .await;

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
        mem_peak,
        canceled_by,
        rsmq,
        flow_is_done,
    )
    .await?;
    Ok(result)
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_ERROR_HANDLER_PATH_IN_ADMINS_WORKSPACE: Option<String> = std::env::var("GLOBAL_ERROR_HANDLER_PATH_IN_ADMINS_WORKSPACE").ok();
}

#[instrument(level = "trace", skip_all, name = "add_completed_job")]
pub async fn add_completed_job<
    T: Serialize + Send + Sync + ValidableJson,
    R: rsmq_async::RsmqConnection + Clone + Send,
>(
    db: &Pool<Postgres>,
    queued_job: &QueuedJob,
    success: bool,
    skipped: bool,
    result: Json<&T>,
    mem_peak: i32,
    canceled_by: Option<CanceledBy>,
    rsmq: Option<R>,
    flow_is_done: bool,
) -> Result<Uuid, Error> {
    // tracing::error!("Start");
    // let start = tokio::time::Instant::now();

    if !result.is_valid_json() {
        return Err(Error::InternalErr(
            "Result of job is invalid json (empty)".to_string(),
        ));
    }

    let mut tx: QueueTransaction<'_, R> = (rsmq.clone(), db.begin().await?).into();
    let job_id = queued_job.id;
    // tracing::error!("1 {:?}", start.elapsed());

    tracing::debug!(
        "completed job {} {}",
        queued_job.id,
        serde_json::to_string(&result).unwrap_or_else(|_| "".to_string())
    );

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
            VALUES ($1, $2, $3, $4, $5, COALESCE($6, now()), (EXTRACT('epoch' FROM (now())) - EXTRACT('epoch' FROM (COALESCE($6, now()))))*1000, $7, $8, $9,\
                    $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29)
         ON CONFLICT (id) DO UPDATE SET success = $7, result = $11 RETURNING duration_ms",
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
        queued_job.raw_code,
        queued_job.raw_lock,
        canceled_by.is_some(),
        canceled_by.clone().map(|cb| cb.username).flatten(),
        canceled_by.clone().map(|cb| cb.reason).flatten(),
        queued_job.job_kind.clone() as JobKind,
        queued_job.schedule_path,
        queued_job.permissioned_as,
        &queued_job.flow_status as &Option<Json<Box<RawValue>>>,
        &queued_job.raw_flow as &Option<Json<Box<RawValue>>>,
        queued_job.is_flow_step,
        skipped,
        queued_job.language.clone() as Option<ScriptLang>,
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

    if !queued_job.is_flow_step {
        if _duration > 500
            && (queued_job.job_kind == JobKind::Script || queued_job.job_kind == JobKind::Preview)
        {
            if let Err(e) = sqlx::query!(
                "UPDATE completed_job SET flow_status = q.flow_status FROM queue q WHERE completed_job.id = $1 AND q.id = $1 AND q.workspace_id = $2 AND completed_job.workspace_id = $2 AND q.flow_status IS NOT NULL",
                &queued_job.id,
                &queued_job.workspace_id
            )
            .execute(&mut tx)
            .await {
                tracing::error!("Could not update job duration: {}", e);
            }
        }
        if let Some(parent_job) = queued_job.parent_job {
            if let Err(e) = sqlx::query_scalar!(
                "UPDATE queue SET flow_status = jsonb_set(jsonb_set(COALESCE(flow_status, '{}'::jsonb), array[$1],  COALESCE(flow_status->$1, '{}'::jsonb)), array[$1, 'duration_ms'], to_jsonb($2::bigint)) WHERE id = $3 AND workspace_id = $4",
                &queued_job.id.to_string(),
                _duration,
                parent_job,
                &queued_job.workspace_id
            )
            .execute(&mut tx)
            .await {
                tracing::error!("Could not update parent job flow_status: {}", e);
            }
        }
    }
    // tracing::error!("Added completed job {:#?}", queued_job);
    let mut skip_downstream_error_handlers = false;
    tx = delete_job(tx, &queued_job.workspace_id, job_id).await?;
    // tracing::error!("3 {:?}", start.elapsed());

    if queued_job.is_flow_step {
        if let Some(parent_job) = queued_job.parent_job {
            // persist the flow last progress timestamp to avoid zombie flow jobs
            tracing::debug!(
                "Persisting flow last progress timestamp to flow job: {:?}",
                parent_job
            );
            sqlx::query!(
                "UPDATE queue SET last_ping = now() WHERE id = $1 AND workspace_id = $2",
                parent_job,
                &queued_job.workspace_id
            )
            .execute(&mut tx)
            .await?;
            if flow_is_done {
                let r = sqlx::query_scalar!(
                    "UPDATE parallel_monitor_lock SET last_ping = now() WHERE parent_flow_id = $1 and job_id = $2 RETURNING 1",
                    parent_job,
                    &queued_job.id
                ).fetch_optional(&mut tx).await?;
                if r.is_some() {
                    tracing::info!(
                        "parallel flow is done, setting parallel monitor last ping lock for job {}",
                        &queued_job.id
                    );
                }
            }
        }
    } else {
        if queued_job.schedule_path.is_some() && queued_job.script_path.is_some() {
            let schedule_path = queued_job.schedule_path.as_ref().unwrap();
            let script_path = queued_job.script_path.as_ref().unwrap();

            let schedule = get_schedule_opt(
                tx.transaction_mut(),
                &queued_job.workspace_id,
                schedule_path,
            )
            .await?;

            if let Some(schedule) = schedule {
                skip_downstream_error_handlers = schedule.ws_error_handler_muted;

                // script or flow that failed on start and might not have been rescheduled
                let schedule_next_tick = !queued_job.is_flow()
                    || {
                        let flow_status = queued_job.parse_flow_status();
                        flow_status.is_some_and(|fs| {
                            fs.step == 0
                            && fs.modules.get(0).is_some_and(|m| {
                                matches!(m, FlowStatusModule::WaitingForPriorSteps { .. }) || matches!(m, FlowStatusModule::Failure { job, ..} if job == &Uuid::nil())
                            })
                        })
                    };

                if schedule_next_tick {
                    if let Err(err) = handle_maybe_scheduled_job(
                        rsmq.clone(),
                        db,
                        queued_job,
                        &schedule,
                        script_path,
                        &queued_job.workspace_id,
                    )
                    .await
                    {
                        match err {
                            Error::QuotaExceeded(_) => return Err(err.into()),
                            // scheduling next job failed and could not disable schedule => make zombie job to retry
                            _ => return Ok(job_id),
                        }
                    };
                }

                if let Err(err) = apply_schedule_handlers(
                    rsmq.clone(),
                    db,
                    &schedule,
                    script_path,
                    &queued_job.workspace_id,
                    success,
                    result,
                    job_id,
                    queued_job.started_at.unwrap_or(chrono::Utc::now()),
                    queued_job.priority,
                )
                .await
                {
                    if !success {
                        tracing::error!("Could not apply schedule error handler: {}", err);
                        let base_url = BASE_URL.read().await;
                        let w_id: &String = &queued_job.workspace_id;
                        if !matches!(err, Error::QuotaExceeded(_)) {
                            report_error_to_workspace_handler_or_critical_side_channel(
                                    rsmq.clone(),
                                    &queued_job,
                                    db,
                                    format!(
                                        "Failed to push schedule error handler job to handle failed job ({base_url}/run/{}?workspace={w_id}): {}",
                                        queued_job.id,
                                        err
                                    ),
                                )
                                .await;
                        }
                    } else {
                        tracing::error!("Could not apply schedule recovery handler: {}", err);
                    }
                };
            } else {
                tracing::error!(
                    "Schedule {schedule_path} in {} not found. Impossible to schedule again and apply schedule handlers",
                    &queued_job.workspace_id
                );
            }
        }
    }
    if queued_job.concurrent_limit.is_some() {
        let concurrency_key = match concurrency_key(db, queued_job).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(
                    "Could not get concurrency key for job {} defaulting to default key: {e:?}",
                    queued_job.id
                );
                legacy_concurrency_key(db, queued_job)
                    .await
                    .unwrap_or_else(|| queued_job.full_path_with_workspace())
            }
        };
        if let Err(e) = sqlx::query_scalar!(
            "UPDATE concurrency_counter SET job_uuids = job_uuids - $2 WHERE concurrency_id = $1",
            concurrency_key,
            queued_job.id.hyphenated().to_string(),
        )
        .execute(&mut tx)
        .await
        {
            tracing::error!("Could not decrement concurrency counter: {}", e);
        }

        if let Err(e) = sqlx::query_scalar!(
            "UPDATE concurrency_key SET ended_at = now() WHERE job_id = $1",
            queued_job.id,
        )
        .execute(&mut tx)
        .await
        .map_err(|e| {
            Error::InternalErr(format!(
                "Error updating to add ended_at timestamp concurrency_key={concurrency_key}: {e}"
            ))
        }) {
            tracing::error!("Could not update concurrency_key: {}", e);
        }
        tracing::debug!("decremented concurrency counter");
    }

    tx.commit().await?;
    tracing::info!(
        %job_id,
        root_job = ?queued_job.root_job.map(|x| x.to_string()).unwrap_or_else(|| String::new()),
        path = &queued_job.script_path(),
        job_kind = ?queued_job.job_kind,
        started_at = ?queued_job.started_at.map(|x| x.to_string()).unwrap_or_else(|| String::new()),
        duration = ?_duration,
        permissioned_as = ?queued_job.permissioned_as,
        email = ?queued_job.email,
        created_by = queued_job.created_by,
        is_flow_step = queued_job.is_flow_step,
        language = ?queued_job.language,
        success,
        "inserted completed job: {} (success: {success})",
        queued_job.id
    );

    #[cfg(feature = "cloud")]
    if *CLOUD_HOSTED && !queued_job.is_flow() && _duration > 1000 {
        let additional_usage = _duration / 1000;
        let w_id = &queued_job.workspace_id;
        let premium_workspace =
            sqlx::query_scalar!("SELECT premium FROM workspace WHERE id = $1", w_id)
                .fetch_one(db)
                .await
                .map_err(|e| Error::InternalErr(format!("fetching if {w_id} is premium: {e}")))?;
        let _ = sqlx::query!(
                "INSERT INTO usage (id, is_workspace, month_, usage) 
                VALUES ($1, TRUE, EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date), $2) 
                ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + $2",
                w_id,
                additional_usage as i32)
                .execute(db)
                .await
                .map_err(|e| Error::InternalErr(format!("updating usage: {e}")));

        if !premium_workspace {
            let _ = sqlx::query!(
                "INSERT INTO usage (id, is_workspace, month_, usage) 
                VALUES ($1, FALSE, EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date), $2) 
                ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + $2",
                queued_job.email,
                additional_usage as i32)
                .execute(db)
                .await
                .map_err(|e| Error::InternalErr(format!("updating usage: {e}")));
        }
    }

    #[cfg(feature = "enterprise")]
    if !success {
        if queued_job.email == ERROR_HANDLER_USER_EMAIL {
            let base_url = BASE_URL.read().await;
            let w_id = &queued_job.workspace_id;
            report_critical_error(format!(
                "Workspace error handler job failed ({base_url}/run/{}?workspace={w_id}){}",
                queued_job.id,
                queued_job
                    .parent_job
                    .map(|id| format!(
                        " trying to handle failed job ({base_url}/run/{id}?workspace={w_id})"
                    ))
                    .unwrap_or("".to_string()),
            ))
            .await;
        } else if queued_job.email == SCHEDULE_ERROR_HANDLER_USER_EMAIL {
            let base_url = BASE_URL.read().await;
            let w_id = &queued_job.workspace_id;
            report_error_to_workspace_handler_or_critical_side_channel(
                rsmq.clone(),
                &queued_job,
                db,
                format!(
                    "Schedule error handler job failed ({base_url}/run/{}?workspace={w_id}){}",
                    queued_job.id,
                    queued_job
                        .parent_job
                        .map(|id| format!(
                            " trying to handle failed job: {base_url}/run/{id}?workspace={w_id}"
                        ))
                        .unwrap_or("".to_string()),
                ),
            )
            .await;
        } else if !skip_downstream_error_handlers
            && matches!(queued_job.job_kind, JobKind::Flow | JobKind::Script)
            && queued_job.parent_job.is_none()
        {
            let result = serde_json::from_str(
                &serde_json::to_string(result.0).unwrap_or_else(|_| "{}".to_string()),
            )
            .unwrap_or_else(|_| json!({}));
            let result = if result.is_object() || result.is_null() {
                result
            } else {
                json!({ "error": result })
            };
            tracing::info!(
                "Sending error of job {} to error handlers (if any)",
                queued_job.id
            );
            if let Err(e) =
                send_error_to_global_handler(rsmq.clone(), &queued_job, db, Json(&result)).await
            {
                tracing::error!(
                    "Could not run global error handler for job {}: {}",
                    &queued_job.id,
                    e
                );
            }

            if let Err(err) = send_error_to_workspace_handler(
                rsmq.clone(),
                &queued_job,
                canceled_by.is_some(),
                db,
                Json(&result),
            )
            .await
            {
                match err {
                    Error::QuotaExceeded(_) => {}
                    _ => {
                        let base_url = BASE_URL.read().await;
                        let w_id: &String = &queued_job.workspace_id;
                        report_critical_error(format!(
                            "Could not push workspace error handler for failed job ({base_url}/run/{}?workspace={w_id}): {}",
                            queued_job.id,
                            err
                        ))
                        .await;
                    }
                }
            }
        }
    }

    if !queued_job.is_flow_step && queued_job.job_kind == JobKind::Script && canceled_by.is_none() {
        if let Some(hash) = queued_job.script_hash {
            let p = sqlx::query_scalar!(
                "SELECT restart_unless_cancelled FROM script WHERE hash = $1 AND workspace_id = $2",
                hash.0,
                &queued_job.workspace_id
            )
            .fetch_optional(db)
            .await?
            .flatten()
            .unwrap_or(false);

            if p {
                let tx = PushIsolationLevel::IsolatedRoot(db.clone(), rsmq);

                // perpetual jobs can run one job per 10s max. If the job was faster than 10s, schedule the next one with the appropriate delay
                let now = chrono::Utc::now();
                let scheduled_for = if now
                    .signed_duration_since(queued_job.started_at.unwrap_or(now))
                    .num_seconds()
                    < 10
                {
                    let next_run = queued_job.started_at.unwrap_or(now)
                        + chrono::Duration::try_seconds(10).unwrap();
                    tracing::warn!("Perpetual script {:?} is running too fast, only 1 job per 10s it supported. Scheduling next run for {:?}", queued_job.script_path, next_run);
                    Some(next_run)
                } else {
                    None
                };

                let (_uuid, tx) = push(
                    db,
                    tx,
                    &queued_job.workspace_id,
                    JobPayload::ScriptHash {
                        hash,
                        path: queued_job.script_path().to_string(),
                        custom_concurrency_key: custom_concurrency_key(db, queued_job.id).await?,
                        concurrent_limit: queued_job.concurrent_limit,
                        concurrency_time_window_s: queued_job.concurrency_time_window_s,
                        cache_ttl: queued_job.cache_ttl,
                        dedicated_worker: None,
                        language: queued_job
                            .language
                            .clone()
                            .unwrap_or_else(|| ScriptLang::Deno),
                        priority: queued_job.priority,
                    },
                    queued_job
                        .args
                        .as_ref()
                        .map(|x| PushArgs { args: x.0.clone(), extra: HashMap::new() })
                        .unwrap_or_else(PushArgs::empty),
                    &queued_job.created_by,
                    &queued_job.email,
                    queued_job.permissioned_as.clone(),
                    scheduled_for,
                    queued_job.schedule_path.clone(),
                    None,
                    None,
                    None,
                    false,
                    false,
                    None,
                    queued_job.visible_to_owner,
                    Some(queued_job.tag.clone()),
                    queued_job.timeout,
                    None,
                    queued_job.priority,
                )
                .await?;
                if let Err(e) = tx.commit().await {
                    tracing::error!("Could not restart job {}: {}", queued_job.id, e);
                }
            }
        }
    }
    // tracing::error!("4 {:?}", start.elapsed());

    Ok(queued_job.id)
}

pub async fn send_error_to_global_handler<
    'a,
    T: Serialize + Send + Sync,
    R: rsmq_async::RsmqConnection + Clone + Send,
>(
    rsmq: Option<R>,
    queued_job: &QueuedJob,
    db: &Pool<Postgres>,
    result: Json<&T>,
) -> Result<(), Error> {
    if let Some(ref global_error_handler) = *GLOBAL_ERROR_HANDLER_PATH_IN_ADMINS_WORKSPACE {
        let prefixed_global_error_handler_path = if global_error_handler.starts_with("script/")
            || global_error_handler.starts_with("flow/")
        {
            global_error_handler.clone()
        } else {
            format!("script/{}", global_error_handler)
        };
        push_error_handler(
            db,
            rsmq,
            queued_job.id,
            queued_job.schedule_path.clone(),
            queued_job.script_path.clone(),
            queued_job.is_flow(),
            &queued_job.workspace_id,
            &prefixed_global_error_handler_path,
            result,
            None,
            queued_job.started_at,
            None,
            &queued_job.email,
            false,
            true,
            None,
        )
        .await?;
    }

    Ok(())
}

pub async fn report_error_to_workspace_handler_or_critical_side_channel<
    R: rsmq_async::RsmqConnection + Clone + Send,
>(
    rsmq: Option<R>,
    queued_job: &QueuedJob,
    db: &Pool<Postgres>,
    error_message: String,
) -> () {
    let w_id = &queued_job.workspace_id;
    let (error_handler, error_handler_extra_args) = sqlx::query_as::<_, (Option<String>, Option<Json<Box<RawValue>>>)>(
        "SELECT error_handler, error_handler_extra_args FROM workspace_settings WHERE workspace_id = $1",
    ).bind(&w_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .unwrap_or((None, None));

    if let Some(error_handler) = error_handler {
        if let Err(err) = push_error_handler(
            db,
            rsmq,
            queued_job.id,
            queued_job.schedule_path.clone(),
            queued_job.script_path.clone(),
            queued_job.is_flow(),
            w_id,
            &error_handler,
            Json(&json!({
                "error": {
                    "message": error_message
                }
            })),
            None,
            queued_job.started_at,
            error_handler_extra_args,
            &queued_job.email,
            false,
            false,
            None,
        )
        .await
        {
            tracing::error!(
                "Could not push workspace error handler trying to handle critical error ({error_message}) of failed job ({}): {}",
                queued_job.id,
                err
            );
            report_critical_error(error_message).await;
        }
    } else {
        report_critical_error(error_message).await;
    }
}

pub async fn send_error_to_workspace_handler<
    'a,
    'c,
    T: Serialize + Send + Sync,
    R: rsmq_async::RsmqConnection + Clone + Send,
>(
    rsmq: Option<R>,
    queued_job: &QueuedJob,
    is_canceled: bool,
    db: &Pool<Postgres>,
    result: Json<&'a T>,
) -> Result<(), Error> {
    let w_id = &queued_job.workspace_id;
    let (error_handler, error_handler_extra_args, error_handler_muted_on_cancel) = sqlx::query_as::<_, (Option<String>, Option<Json<Box<RawValue>>>, bool)>(
        "SELECT error_handler, error_handler_extra_args, error_handler_muted_on_cancel FROM workspace_settings WHERE workspace_id = $1",
    ).bind(&w_id)
    .fetch_optional(db)
    .await
    .context("fetching error handler info from workspace_settings")?
    .ok_or_else(|| Error::InternalErr(format!("no workspace settings for id {w_id}")))?;

    if is_canceled && error_handler_muted_on_cancel {
        return Ok(());
    }

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

        let muted = ws_error_handler_muted.unwrap_or(false);
        if !muted {
            tracing::info!("workspace error handled for job {}", &queued_job.id);

            push_error_handler(
                db,
                rsmq,
                queued_job.id,
                queued_job.schedule_path.clone(),
                queued_job.script_path.clone(),
                queued_job.is_flow(),
                &queued_job.workspace_id,
                &error_handler,
                result,
                None,
                queued_job.started_at,
                error_handler_extra_args,
                &queued_job.email,
                false,
                false,
                None,
            )
            .await?;
        }
    }

    Ok(())
}

#[instrument(level = "trace", skip_all)]
pub async fn handle_maybe_scheduled_job<'c, R: rsmq_async::RsmqConnection + Clone + Send + 'c>(
    rsmq: Option<R>,
    db: &Pool<Postgres>,
    job: &QueuedJob,
    schedule: &Schedule,
    script_path: &str,
    w_id: &str,
) -> windmill_common::error::Result<()> {
    tracing::info!(
        "Schedule {} scheduling next job for {} in {w_id}",
        schedule.path,
        schedule.script_path
    );

    if schedule.enabled && script_path == schedule.script_path {
        let push_next_job_future = async {
            let mut tx: QueueTransaction<'_, _> = (rsmq.clone(), db.begin().await?).into();
            tx = push_scheduled_job(db, tx, &schedule).await?;
            tx.commit().await?;
            Ok::<(), Error>(())
        };
        match push_next_job_future.await {
            Ok(_) => Ok(()),
            Err(err) => {
                match sqlx::query!(
                    "UPDATE schedule SET enabled = false, error = $1 WHERE workspace_id = $2 AND path = $3",
                    err.to_string(),
                    &schedule.workspace_id,
                    &schedule.path
                )
                .execute(db).await {
                    Ok(_) => {
                        match err {
                            Error::QuotaExceeded(_) => {}
                            _ => {
                                report_error_to_workspace_handler_or_critical_side_channel(rsmq, job, db,
                                    format!("Could not schedule next job for {} with err {}. Schedule disabled", schedule.path, err)
                                ).await;
                            }
                        }
                        Ok(())
                    }
                    Err(disable_err) => match err {
                        Error::QuotaExceeded(_) => Err(err),
                        _ => {
                            report_error_to_workspace_handler_or_critical_side_channel(rsmq, job, db,
                                    format!("Could not schedule next job for {} and could not disable schedule with err {}. Will retry", schedule.path, disable_err)
                                ).await;
                            Err(to_anyhow(disable_err).into())
                        }
                    },
                }
            }
        }
    } else {
        if script_path != schedule.script_path {
            tracing::warn!(
                "Schedule {} in {w_id} has a different script path than the job. Not scheduling again", schedule.path
            );
        } else {
            tracing::info!(
                "Schedule {} in {w_id} is disabled. Not scheduling again.",
                schedule.path
            );
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, FromRow)]
struct CompletedJobSubset {
    success: bool,
    result: Option<sqlx::types::Json<Box<RawValue>>>,
    started_at: chrono::DateTime<chrono::Utc>,
}
async fn apply_schedule_handlers<
    'a,
    'c,
    T: Serialize + Send + Sync,
    R: rsmq_async::RsmqConnection + Clone + Send + 'c,
>(
    rsmq: Option<R>,
    db: &Pool<Postgres>,
    schedule: &Schedule,
    script_path: &str,
    w_id: &str,
    success: bool,
    result: Json<&'a T>,
    job_id: Uuid,
    started_at: DateTime<Utc>,
    job_priority: Option<i16>,
) -> windmill_common::error::Result<()> {
    if !success {
        #[cfg(feature = "enterprise")]
        if let Some(on_failure_path) = schedule.on_failure.clone() {
            let times = schedule.on_failure_times.unwrap_or(1).max(1);
            let exact = schedule.on_failure_exact.unwrap_or(false);
            if times > 1 || exact {
                let past_jobs = sqlx::query_as::<_, CompletedJobSubset>(
                    "SELECT success, result, started_at FROM completed_job WHERE workspace_id = $1 AND schedule_path = $2 AND script_path = $3 AND id != $4 ORDER BY created_at DESC LIMIT $5",
                )
                .bind(&schedule.workspace_id)
                .bind(&schedule.path)
                .bind(script_path)
                .bind(job_id)
                .bind(if exact { times } else { times - 1 } as i64,)
                .fetch_all(db).await?;

                let match_times = if exact {
                    past_jobs.len() == times as usize
                        && past_jobs[..(times - 1) as usize].iter().all(|j| !j.success)
                        && past_jobs[(times - 1) as usize].success
                } else {
                    past_jobs.len() == ((times - 1) as usize)
                        && past_jobs.iter().all(|j| !j.success)
                };

                if !match_times {
                    return Ok(());
                }
            }

            push_error_handler(
                db,
                rsmq,
                job_id,
                Some(schedule.path.to_string()),
                Some(script_path.to_string()),
                schedule.is_flow,
                w_id,
                &on_failure_path,
                result,
                Some(times),
                Some(started_at),
                schedule.on_failure_extra_args.clone(),
                &schedule.email,
                true,
                false,
                job_priority,
            )
            .await?;
        }
    } else {
        #[cfg(feature = "enterprise")]
        if let Some(on_recovery_path) = schedule.on_recovery.clone() {
            let mut tx: QueueTransaction<'_, R> = (rsmq.clone(), db.begin().await?).into();
            let times = schedule.on_recovery_times.unwrap_or(1).max(1);
            let past_jobs = sqlx::query_as::<_, CompletedJobSubset>(
                "SELECT success, result, started_at FROM completed_job WHERE workspace_id = $1 AND schedule_path = $2 AND script_path = $3 AND id != $4 ORDER BY created_at DESC LIMIT $5",
            )
            .bind(&schedule.workspace_id)
            .bind(&schedule.path)
            .bind(script_path)
            .bind(job_id)
            .bind(times as i64)
            .fetch_all(db).await?;

            if past_jobs.len() < times as usize {
                return Ok(());
            }

            let n_times_successful = past_jobs[..(times - 1) as usize].iter().all(|j| j.success);

            if !n_times_successful {
                return Ok(());
            }

            let failed_job = past_jobs[past_jobs.len() - 1].clone();

            if !failed_job.success {
                tx = handle_recovered_schedule(
                    db,
                    tx,
                    job_id,
                    &schedule.path,
                    script_path,
                    schedule.is_flow,
                    w_id,
                    &on_recovery_path,
                    failed_job,
                    result,
                    times,
                    started_at,
                    schedule.on_recovery_extra_args.clone(),
                )
                .await?;
            }

            tx.commit().await?;
        }
    }

    Ok(())
}

pub async fn push_error_handler<
    'a,
    'c,
    T: Serialize + Send + Sync,
    R: rsmq_async::RsmqConnection + Clone + Send + 'c,
>(
    db: &Pool<Postgres>,
    rsmq: Option<R>,
    job_id: Uuid,
    schedule_path: Option<String>,
    script_path: Option<String>,
    is_flow: bool,
    w_id: &str,
    on_failure_path: &str,
    result: Json<&'a T>,
    failed_times: Option<i32>,
    started_at: Option<DateTime<Utc>>,
    extra_args: Option<Json<Box<RawValue>>>,
    email: &str,
    is_schedule_error_handler: bool,
    is_global_error_handler: bool,
    priority: Option<i16>,
) -> windmill_common::error::Result<Uuid> {
    let handler_w_id = if is_global_error_handler {
        "admins"
    } else {
        w_id
    };
    let (payload, tag) =
        get_payload_tag_from_prefixed_path(on_failure_path, db, handler_w_id).await?;

    let mut extra = HashMap::new();
    if let Some(schedule_path) = schedule_path {
        extra.insert("schedule_path".to_string(), to_raw_value(&schedule_path));
    }
    extra.insert("workspace_id".to_string(), to_raw_value(&w_id));
    extra.insert("job_id".to_string(), to_raw_value(&job_id));
    extra.insert("path".to_string(), to_raw_value(&script_path));
    extra.insert("is_flow".to_string(), to_raw_value(&is_flow));
    extra.insert("started_at".to_string(), to_raw_value(&started_at));
    extra.insert("email".to_string(), to_raw_value(&email));
    if let Some(failed_times) = failed_times {
        extra.insert("failed_times".to_string(), to_raw_value(&failed_times));
    }

    if let Some(args_v) = extra_args {
        if let Ok(args_m) = serde_json::from_str::<HashMap<String, Box<RawValue>>>(args_v.get()) {
            extra.extend(args_m);
        } else {
            return Err(error::Error::ExecutionErr(
                "args of scripts needs to be dict".to_string(),
            ));
        }
    }

    // TODO(gbouv): REMOVE THIS after December 1st 2023 and ping users to re-save their error handlers
    if on_failure_path
        .to_string()
        .eq("script/hub/5792/workspace-or-schedule-error-handler-slack")
    {
        // default slack error handler being used -> we need to inject the slack token
        let slack_resource = format!("$res:{WORKSPACE_SLACK_BOT_TOKEN_PATH}");
        extra.insert("slack".to_string(), to_raw_value(&slack_resource));
    }

    let result = sanitize_result(result);

    let tx = PushIsolationLevel::IsolatedRoot(db.clone(), rsmq);
    let (uuid, tx) = push(
        &db,
        tx,
        handler_w_id,
        payload,
        PushArgs { extra, args: result },
        if is_global_error_handler {
            "global"
        } else if is_schedule_error_handler {
            SCHEDULE_ERROR_HANDLER_USERNAME
        } else {
            ERROR_HANDLER_USERNAME
        },
        if is_global_error_handler {
            SUPERADMIN_SECRET_EMAIL
        } else if is_schedule_error_handler {
            SCHEDULE_ERROR_HANDLER_USER_EMAIL
        } else {
            ERROR_HANDLER_USER_EMAIL
        },
        if is_global_error_handler {
            SUPERADMIN_SECRET_EMAIL.to_string()
        } else {
            ERROR_HANDLER_USER_GROUP.to_string()
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
        priority,
    )
    .await?;
    tx.commit().await?;
    return Ok(uuid);
}

fn sanitize_result<T: Serialize + Send + Sync>(result: Json<&T>) -> HashMap<String, Box<RawValue>> {
    let as_str = serde_json::to_string(result.0).unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str::<HashMap<String, Box<RawValue>>>(&as_str)
        .unwrap_or_else(|_| [("error".to_string(), RawValue::from_string(as_str).unwrap())].into())
}

// #[derive(Serialize)]
// pub struct RecoveryValue<T> {
//     error_started_at: chrono::DateTime<Utc>,
//     schedule_path: String,
//     path: String,
//     is_flow: boolean,
//     extra_args: serde_json::Value
// }
async fn handle_recovered_schedule<
    'a,
    'c,
    T: Serialize + Send + Sync,
    R: rsmq_async::RsmqConnection + Clone + Send + 'c,
>(
    db: &Pool<Postgres>,
    tx: QueueTransaction<'c, R>,
    job_id: Uuid,
    schedule_path: &str,
    script_path: &str,
    is_flow: bool,
    w_id: &str,
    on_recovery_path: &str,
    error_job: CompletedJobSubset,
    successful_job_result: Json<&'a T>,
    successful_times: i32,
    successful_job_started_at: DateTime<Utc>,
    extra_args: Option<Json<Box<RawValue>>>,
) -> windmill_common::error::Result<QueueTransaction<'c, R>> {
    let (payload, tag) = get_payload_tag_from_prefixed_path(on_recovery_path, db, w_id).await?;

    let mut extra = HashMap::new();
    extra.insert(
        "error_started_at".to_string(),
        to_raw_value(&error_job.started_at),
    );
    extra.insert("schedule_path".to_string(), to_raw_value(&schedule_path));
    extra.insert("path".to_string(), to_raw_value(&script_path));
    extra.insert("is_flow".to_string(), to_raw_value(&is_flow));
    extra.insert(
        "success_result".to_string(),
        serde_json::from_str::<Box<RawValue>>(
            &serde_json::to_string(&successful_job_result).unwrap(),
        )
        .unwrap_or_else(|_| serde_json::value::RawValue::from_string("{}".to_string()).unwrap()),
    );
    extra.insert("success_times".to_string(), to_raw_value(&successful_times));
    extra.insert(
        "success_started_at".to_string(),
        to_raw_value(&successful_job_started_at),
    );
    if let Some(args_v) = extra_args {
        if let Ok(args_m) = serde_json::from_str::<HashMap<String, Box<RawValue>>>(args_v.get()) {
            extra.extend(args_m);
        } else {
            return Err(error::Error::ExecutionErr(
                "args of scripts needs to be dict".to_string(),
            ));
        }
    }
    // TODO(gbouv): REMOVE THIS after December 1st 2023 and ping users to re-save their error handlers
    if on_recovery_path
        .to_string()
        .eq("script/hub/2430/slack/schedule-recovery-handler-slack")
    {
        // default slack error handler being used -> we need to inject the slack token
        let slack_resource = format!("$res:{WORKSPACE_SLACK_BOT_TOKEN_PATH}");
        extra.insert("slack".to_string(), to_raw_value(&slack_resource));
    }

    let args = error_job
        .result
        .and_then(|x| serde_json::from_str::<HashMap<String, Box<RawValue>>>(x.0.get()).ok())
        .unwrap_or_else(HashMap::new);

    let tx = PushIsolationLevel::Transaction(tx);
    let (uuid, tx) = push(
        &db,
        tx,
        w_id,
        payload,
        PushArgs { extra: extra, args: args },
        SCHEDULE_RECOVERY_HANDLER_USERNAME,
        SCHEDULE_RECOVERY_HANDLER_USER_EMAIL,
        ERROR_HANDLER_USER_GROUP.to_string(),
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

        let has_concurent_limit = job.as_ref().unwrap().concurrent_limit.is_some();

        #[cfg(not(feature = "enterprise"))]
        if has_concurent_limit {
            tracing::error!("Concurrent limits are an EE feature only, ignoring constraints")
        }

        #[cfg(not(feature = "enterprise"))]
        let has_concurent_limit = false;

        // concurrency check. If more than X jobs for this path are already running, we re-queue and pull another job from the queue
        let pulled_job = job.unwrap();
        if pulled_job.script_path.is_none() || !has_concurent_limit || pulled_job.canceled {
            #[cfg(feature = "prometheus")]
            if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                QUEUE_PULL_COUNT.inc();
            }
            return Ok(Option::Some(pulled_job));
        }

        let itx = db.begin().await?;

        let mut tx: QueueTransaction<'_, _> = (rsmq.clone(), itx).into();

        // Else the job is subject to concurrency limits
        let job_script_path = pulled_job.script_path.clone().unwrap();

        let job_concurrency_key = match concurrency_key(db, &pulled_job).await {
            Ok(key) => key,
            Err(e) => {
                tracing::error!(
                    "Could not get concurrency key for job {} defaulting to default key: {e:?}",
                    pulled_job.id
                );
                legacy_concurrency_key(db, &pulled_job)
                    .await
                    .unwrap_or_else(|| pulled_job.full_path_with_workspace())
            }
        };
        tracing::debug!("Concurrency key is '{}'", job_concurrency_key);
        let job_custom_concurrent_limit = pulled_job.concurrent_limit.unwrap();
        // setting concurrency_time_window to 0 will count only the currently running jobs
        let job_custom_concurrency_time_window_s =
            pulled_job.concurrency_time_window_s.unwrap_or(0);
        tracing::debug!(
            "Job concurrency limit is {} per {}s",
            job_custom_concurrent_limit,
            job_custom_concurrency_time_window_s
        );

        sqlx::query_scalar!(
            "SELECT null FROM queue WHERE id = $1 FOR UPDATE",
            pulled_job.id
        )
        .fetch_one(&mut tx)
        .await
        .context("lock job in queue")?;

        let jobs_uuids_init_json_value = serde_json::from_str::<serde_json::Value>(
            format!("{{\"{}\": {{}}}}", pulled_job.id.hyphenated().to_string()).as_str(),
        )
        .expect("Unable to serialize job_uuids column to proper JSON");
        let running_job = sqlx::query_scalar!(
            "INSERT INTO concurrency_counter(concurrency_id, job_uuids) VALUES ($1, $2)
        ON CONFLICT (concurrency_id) 
        DO UPDATE SET job_uuids = jsonb_set(concurrency_counter.job_uuids, array[$3], '{}')
        RETURNING (SELECT COUNT(*) FROM jsonb_object_keys(job_uuids))",
            job_concurrency_key,
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

        let completed_count = sqlx::query!(
            "SELECT COUNT(*) as count, COALESCE(MAX(ended_at), now() - INTERVAL '1 second' * $2)  as max_ended_at FROM concurrency_key WHERE key = $1 AND ended_at >=  (now() - INTERVAL '1 second' * $2)",
            job_concurrency_key,
            f64::from(job_custom_concurrency_time_window_s),
        ).fetch_one(&mut tx).await.map_err(|e| {
            Error::InternalErr(format!(
                "Error getting completed count for key {job_concurrency_key}: {e}"
            ))
        })?;

        let min_started_at = sqlx::query!(
            "SELECT COALESCE((SELECT MIN(started_at) as min_started_at
            FROM queue
            WHERE script_path = $1 AND job_kind != 'dependencies'  AND running = true AND workspace_id = $2 AND canceled = false
            GROUP BY script_path), $3) as min_started_at, now() AS now",
            job_script_path,
            &pulled_job.workspace_id,
            completed_count.max_ended_at
        )
        .fetch_one(&mut tx)
        .await
        .map_err(|e| {
            Error::InternalErr(format!(
                "Error getting concurrency count for script path {job_script_path}: {e}"
            ))
        })?;

        let concurrent_jobs_for_this_script =
            completed_count.count.unwrap_or_default() as i32 + running_job.unwrap_or(0) as i32;
        tracing::debug!(
            "Current concurrent jobs for this script: {}",
            concurrent_jobs_for_this_script
        );
        if concurrent_jobs_for_this_script <= job_custom_concurrent_limit {
            #[cfg(feature = "prometheus")]
            if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                QUEUE_PULL_COUNT.inc();
            }
            tx.commit().await?;
            return Ok(Option::Some(pulled_job));
        }
        let x = sqlx::query_scalar!(
            "UPDATE concurrency_counter SET job_uuids = job_uuids - $2 WHERE concurrency_id = $1 RETURNING (SELECT COUNT(*) FROM jsonb_object_keys(job_uuids))",
            job_concurrency_key,
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
        let estimated_next_schedule_timestamp = ((min_started_at
            .min_started_at
            .unwrap_or(min_started_at.now.unwrap()))
            + Duration::try_seconds(avg_script_duration.map(i64::from).unwrap_or(0))
                .unwrap_or_default()
                .max(Duration::try_seconds(5).unwrap_or_default())
            + Duration::try_seconds(i64::from(job_custom_concurrency_time_window_s))
                .unwrap_or_default()
                .max(Duration::try_seconds(5).unwrap_or_default()))
        .max(min_started_at.now.unwrap() + Duration::try_seconds(10).unwrap_or_default());

        tracing::info!("Job '{}' from path '{}' with concurrency key '{}' has reached its concurrency limit of {} jobs run in the last {} seconds. This job will be re-queued for next execution at {}", 
            job_uuid, job_script_path,  job_concurrency_key, job_custom_concurrent_limit, job_custom_concurrency_time_window_s, estimated_next_schedule_timestamp);

        let job_log_event = format!(
            "\nRe-scheduled job to {estimated_next_schedule_timestamp} due to concurrency limits with key {job_concurrency_key} and limit {job_custom_concurrent_limit} in the last {job_custom_concurrency_time_window_s} seconds",
        );
        let _ = append_logs(job_uuid, pulled_job.workspace_id, job_log_event, db).await;
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
            RETURNING  id,  workspace_id,  parent_job,  created_by,  created_at,  started_at,  scheduled_for,
                running,  script_hash,  script_path,  args,   right(logs, 900000) as logs,  raw_code,  canceled,  canceled_by,  
                canceled_reason,  last_ping,  job_kind,  env_id,  schedule_path,  permissioned_as, 
                flow_status,  raw_flow,  is_flow_step,  language,  suspend,  suspend_until,  
                same_worker,  raw_lock,  pre_run_error,  email,  visible_to_owner,  mem_peak, 
                 root_job,  leaf_jobs,  tag,  concurrent_limit,  concurrency_time_window_s,  
                 timeout,  flow_step_id,  cache_ttl, priority",
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
            RETURNING  id,  workspace_id,  parent_job,  created_by,  created_at,  started_at,  scheduled_for,
            running,  script_hash,  script_path,  args,   null as logs,  raw_code,  canceled,  canceled_by,  
            canceled_reason,  last_ping,  job_kind,  env_id,  schedule_path,  permissioned_as, 
            flow_status,  raw_flow,  is_flow_step,  language,  suspend,  suspend_until,  
            same_worker,  raw_lock,  pre_run_error,  email,  visible_to_owner,  mem_peak, 
             root_job,  leaf_jobs,  tag,  concurrent_limit,  concurrency_time_window_s,  
             timeout,  flow_step_id,  cache_ttl, priority")
                .bind(tags)
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
                    RETURNING  id,  workspace_id,  parent_job,  created_by,  created_at,  started_at,  scheduled_for,
                    running,  script_hash,  script_path,  args,  null as logs,  raw_code,  canceled,  canceled_by,  
                    canceled_reason,  last_ping,  job_kind,  env_id,  schedule_path,  permissioned_as, 
                    flow_status,  raw_flow,  is_flow_step,  language,  suspend,  suspend_until,  
                    same_worker,  raw_lock,  pre_run_error,  email,  visible_to_owner,  mem_peak, 
                     root_job,  leaf_jobs,  tag,  concurrent_limit,  concurrency_time_window_s,  
                     timeout,  flow_step_id,  cache_ttl, priority",
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

pub async fn custom_concurrency_key(
    db: &Pool<Postgres>,
    job_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!("SELECT key FROM concurrency_key WHERE job_id = $1", job_id)
        .fetch_optional(db) // this should no longer be fetch optional
        .await
}

async fn legacy_concurrency_key(db: &Pool<Postgres>, queued_job: &QueuedJob) -> Option<String> {
    let r = if queued_job.is_flow() {
        sqlx::query_scalar!(
            "SELECT value->>'concurrency_key' FROM flow WHERE path = $1 AND workspace_id = $2",
            queued_job.script_path,
            queued_job.workspace_id
        )
        .fetch_optional(db)
        .await
    } else {
        sqlx::query_scalar!(
            "SELECT concurrency_key FROM script WHERE hash = $1 AND workspace_id = $2",
            queued_job.script_hash.unwrap_or(ScriptHash(0)).0,
            queued_job.workspace_id
        )
        .fetch_optional(db)
        .await
    }
    .ok()
    .flatten()
    .flatten();

    r.map(|x| {
        interpolate_args(
            x,
            &queued_job
                .args
                .clone()
                .map(|x| x.0)
                .unwrap_or_default()
                .into(),
            &queued_job.workspace_id,
        )
    })
}

async fn concurrency_key(
    db: &Pool<Postgres>,
    queued_job: &QueuedJob,
) -> windmill_common::error::Result<String> {
    not_found_if_none(
        custom_concurrency_key(db, queued_job.id).await?,
        "ConcurrencyKey",
        queued_job.id.to_string(),
    )
}

fn interpolate_args(x: String, args: &PushArgs, workspace_id: &str) -> String {
    // Save this value to avoid parsing twice
    let workspaced = x.as_str().replace("$workspace", workspace_id).to_string();
    if RE_ARG_TAG.is_match(&workspaced) {
        let mut interpolated = workspaced.clone();
        for cap in RE_ARG_TAG.captures_iter(&workspaced) {
            let arg_name = cap.get(1).unwrap().as_str();
            let arg_value = args
                .args
                .get(arg_name)
                .or(args.extra.get(arg_name))
                .map(|x| x.get())
                .unwrap_or_default();
            interpolated =
                interpolated.replace(format!("$args[{}]", arg_name).as_str(), &arg_value);
        }
        interpolated
    } else {
        workspaced
    }
}

fn fullpath_with_workspace(
    workspace_id: &str,
    script_path: Option<&String>,
    job_kind: &JobKind,
) -> String {
    let path = script_path.map(String::as_str).unwrap_or("tmp/main");
    let is_flow = matches!(
        job_kind,
        &JobKind::Flow | &JobKind::FlowPreview | &JobKind::SingleScriptFlow
    );
    format!(
        "{}/{}/{}",
        workspace_id,
        if is_flow { "flow" } else { "script" },
        path,
    )
}

#[derive(FromRow)]
pub struct ResultR {
    result: Option<Json<Box<RawValue>>>,
}

#[derive(FromRow)]
pub struct ResultWithId {
    result: Option<Json<Box<RawValue>>>,
    id: Uuid,
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

    extract_result_from_job_result(db, w_id, result_id, json_path).await
}

#[async_recursion]
async fn get_result_by_id_from_original_flow(
    db: &Pool<Postgres>,
    w_id: &str,
    completed_flow_id: &Uuid,
    node_id: &str,
    json_path: Option<String>,
) -> error::Result<Box<RawValue>> {
    let flow_job = sqlx::query_as::<_, CompletedJob>(
        "SELECT *, null as labels FROM completed_job WHERE id = $1 AND workspace_id = $2",
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

    let mut leaf_jobs_for_flow = HashMap::<String, JobResult>::new();
    compute_leaf_jobs_for_completed_flow(
        db,
        w_id,
        flow_job.id,
        Some(flow_job.clone()),
        &mut leaf_jobs_for_flow,
    )
    .await?;

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
    let leaf_job_uuid = leaf_jobs_for_flow.get(&node_id.to_string()).unwrap();
    extract_result_from_job_result(db, w_id, leaf_job_uuid.to_owned(), json_path).await
}

#[async_recursion]
async fn compute_leaf_jobs_for_completed_flow(
    db: &Pool<Postgres>,
    w_id: &str,
    completed_flow_id: Uuid,
    completed_flow_row: Option<CompletedJob>, // if provided, will be used, otherwise completed_flow_id must be set and the job definition will be pulled from DB
    recursive_result: &mut HashMap<String, JobResult>,
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
                        recursive_result.insert(module.id(), JobResult::SingleJob(child_job_id));
                    }
                }
            }
            JobKind::Flow | JobKind::FlowPreview | JobKind::SingleScriptFlow => {
                // Extract the leaf job for this flow and add them to the result map
                for module in &flow_job_status_modules {
                    // we add the module as an element of ListJob for this step ID and recursiively extract leaf job of the sub-flow
                    match module {
                        FlowStatusModule::Success { flow_jobs: Some(jobs), .. } => {
                            let jobs_set: HashSet<Uuid> = HashSet::from_iter(jobs.iter().cloned());
                            if jobs_set.contains(&child_job_id) {
                                let new_list_job = match recursive_result.get(&module.id()) {
                                    Some(JobResult::ListJob(jobs_list)) => {
                                        let mut jobs_list_c = jobs_list.clone();
                                        jobs_list_c.push(child_job_id);
                                        jobs_list_c
                                    }
                                    _ => vec![child_job_id],
                                };
                                recursive_result
                                    .insert(module.id(), JobResult::ListJob(new_list_job));
                            }
                        }
                        _ => {}
                    }
                }
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

async fn extract_result_from_job_result(
    db: &Pool<Postgres>,
    w_id: &str,
    job_result: JobResult,
    json_path: Option<String>,
) -> error::Result<Box<RawValue>> {
    match job_result {
        JobResult::ListJob(job_ids) => {
            let rows = sqlx::query(
                "SELECT id, result FROM completed_job WHERE id = ANY($1) AND workspace_id = $2",
            )
            .bind(job_ids.as_slice())
            .bind(w_id)
            .fetch_all(db)
            .await?
            .into_iter()
            .filter_map(|x| {
                ResultWithId::from_row(&x)
                    .ok()
                    .and_then(|x| x.result.map(|y| (x.id, y)))
            })
            .collect::<HashMap<Uuid, Json<Box<RawValue>>>>();
            let result = job_ids
                .into_iter()
                .map(|id| {
                    rows.get(&id)
                        .map(|x| x.0.clone())
                        .unwrap_or_else(|| to_raw_value(&serde_json::Value::Null))
                })
                .collect::<Vec<_>>();
            Ok(to_raw_value(&result))
        }
        JobResult::SingleJob(x) => Ok(sqlx::query(
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
        .unwrap_or_else(|| to_raw_value(&serde_json::Value::Null))),
    }
}

#[instrument(level = "trace", skip_all)]
pub async fn delete_job<'c, R: rsmq_async::RsmqConnection + Clone + Send>(
    mut tx: QueueTransaction<'c, R>,
    w_id: &str,
    job_id: Uuid,
) -> windmill_common::error::Result<QueueTransaction<'c, R>> {
    #[cfg(feature = "prometheus")]
    if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
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

pub async fn get_queued_job_tx<'c>(
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

pub async fn get_queued_job(id: &Uuid, w_id: &str, db: &DB) -> error::Result<Option<QueuedJob>> {
    let r = sqlx::query(
        "SELECT *
            FROM queue WHERE id = $1 AND workspace_id = $2",
    )
    .bind(id)
    .bind(w_id)
    .fetch_optional(db)
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

#[derive(Serialize, Debug)]
pub struct PushArgs {
    #[serde(flatten)]
    pub extra: HashMap<String, Box<RawValue>>,
    #[serde(flatten)]
    pub args: HashMap<String, Box<RawValue>>,
}

#[derive(Deserialize)]
pub struct DecodeQuery {
    pub include_query: Option<String>,
}

#[derive(Deserialize)]
pub struct IncludeQuery {
    pub include_query: Option<String>,
}

pub struct DecodeQueries(pub HashMap<String, Box<RawValue>>);

#[axum::async_trait]
impl<S> FromRequestParts<S> for DecodeQueries
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(DecodeQueries::from_uri(&parts.uri).unwrap_or_else(|| DecodeQueries(HashMap::new())))
    }
}

impl DecodeQueries {
    fn from_uri(uri: &Uri) -> Option<Self> {
        let query = uri.query();
        if query.is_none() {
            return None;
        }
        let query = query.unwrap();
        let include_query = serde_urlencoded::from_str::<IncludeQuery>(query)
            .map(|x| x.include_query)
            .ok()
            .flatten()
            .unwrap_or_default();
        let parse_query_args = include_query
            .split(",")
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let mut args = HashMap::new();
        if !parse_query_args.is_empty() {
            let queries =
                serde_urlencoded::from_str::<HashMap<String, String>>(query).unwrap_or_default();
            parse_query_args.iter().for_each(|h| {
                if let Some(v) = queries.get(h) {
                    args.insert(h.to_string(), to_raw_value(v));
                }
            });
        }
        Some(DecodeQueries(args))
    }
}

impl PushArgs {
    pub fn insert<K: Into<String>, V: Into<Box<RawValue>>>(&mut self, k: K, v: V) {
        self.extra.insert(k.into(), v.into());
    }
}

#[derive(Deserialize)]
pub struct RequestQuery {
    pub raw: Option<bool>,
    pub include_header: Option<String>,
}

#[axum::async_trait]
impl<S> FromRequest<S, axum::body::Body> for PushArgs
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
            let uri = req.uri();
            let query = Query::<RequestQuery>::try_from_uri(uri).unwrap().0;
            let mut extra = build_extra(&headers_map, query.include_header);
            let query_decode = DecodeQueries::from_uri(uri);
            if let Some(DecodeQueries(queries)) = query_decode {
                extra.extend(queries);
            }
            let raw = query.raw.as_ref().is_some_and(|x| *x);
            (content_type, extra, raw)
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
                Ok(PushArgs { extra, args: hm })
            } else {
                let hm = serde_json::from_str::<Option<HashMap<String, Box<JsonRawValue>>>>(&str)
                    .map_err(|e| Error::BadRequest(format!("invalid json: {}", e)).into_response())?
                    .unwrap_or_else(HashMap::new);
                Ok(PushArgs { extra, args: hm })
            }
        } else if content_type
            .unwrap()
            .starts_with("application/x-www-form-urlencoded")
        {
            let bytes = Bytes::from_request(req, _state)
                .await
                .map_err(IntoResponse::into_response)?;

            if use_raw {
                let raw_string = String::from_utf8(bytes.to_vec()).map_err(|e| {
                    Error::BadRequest(format!("invalid utf8: {}", e)).into_response()
                })?;
                extra.insert("raw_string".to_string(), to_raw_value(&raw_string));
            }

            let payload: HashMap<String, Option<String>> = serde_urlencoded::from_bytes(&bytes)
                .map_err(|e| {
                    Error::BadRequest(format!("invalid urlencoded data: {}", e)).into_response()
                })?;
            let payload = payload
                .into_iter()
                .map(|(k, v)| (k, to_raw_value(&v)))
                .collect::<HashMap<_, _>>();

            return Ok(PushArgs { extra, args: payload });
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

impl PushArgs {
    pub fn empty() -> Self {
        PushArgs { extra: HashMap::new(), args: HashMap::new() }
    }
}

pub fn empty_result() -> Box<RawValue> {
    return JsonRawValue::from_string("{}".to_string()).unwrap();
}

impl From<HashMap<String, Box<JsonRawValue>>> for PushArgs {
    fn from(value: HashMap<String, Box<JsonRawValue>>) -> Self {
        PushArgs { extra: HashMap::new(), args: value }
    }
}

// impl<T> From<PushArgsInner<T>> for PushArgs<T> {
//     fn from(value: PushArgsInner<T>) -> Self {
//         PushArgs::Unwrapped(value)
//     }
// }

lazy_static::lazy_static! {
    pub static ref RE_ARG_TAG: Regex = Regex::new(r#"\$args\[(\w+)\]"#).unwrap();
}

// #[instrument(level = "trace", skip_all)]
pub async fn push<'c, R: rsmq_async::RsmqConnection + Send + 'c>(
    _db: &Pool<Postgres>,
    mut tx: PushIsolationLevel<'c, R>,
    workspace_id: &str,
    job_payload: JobPayload,
    args: PushArgs,
    user: &str,
    mut email: &str,
    mut permissioned_as: String,
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
    _priority_override: Option<i16>,
) -> Result<(Uuid, QueueTransaction<'c, R>), Error> {
    #[cfg(feature = "cloud")]
    if *CLOUD_HOSTED {
        let premium_workspace =
            sqlx::query_scalar!("SELECT premium FROM workspace WHERE id = $1", workspace_id)
                .fetch_one(_db)
                .await
                .map_err(|e| {
                    Error::InternalErr(format!(
                        "fetching if {workspace_id} is premium and overquota: {e}"
                    ))
                })?;

        // we track only non flow steps
        let (workspace_usage, user_usage) = if !matches!(
            job_payload,
            JobPayload::Flow { .. } | JobPayload::RawFlow { .. }
        ) {
            let workspace_usage = sqlx::query_scalar!(
                    "INSERT INTO usage (id, is_workspace, month_, usage)
                    VALUES ($1, TRUE, EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date), 1)
                    ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + 1 
                    RETURNING usage.usage",
                    workspace_id
                )
                .fetch_one(_db)
                .await
                .map_err(|e| Error::InternalErr(format!("updating usage: {e}")))?;

            let user_usage = if !premium_workspace {
                Some(sqlx::query_scalar!(
                    "INSERT INTO usage (id, is_workspace, month_, usage)
                    VALUES ($1, FALSE, EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date), 1)
                    ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + 1 
                    RETURNING usage.usage",
                    email
                )
                .fetch_one(_db)
                .await
                .map_err(|e| Error::InternalErr(format!("updating usage: {e}")))?)
            } else {
                None
            };
            (Some(workspace_usage), user_usage)
        } else {
            (None, None)
        };

        if !premium_workspace {
            let is_super_admin =
                sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
                    .fetch_optional(_db)
                    .await?
                    .unwrap_or(false);

            if !is_super_admin {
                if email != ERROR_HANDLER_USER_EMAIL
                    && email != SCHEDULE_ERROR_HANDLER_USER_EMAIL
                    && email != SCHEDULE_RECOVERY_HANDLER_USER_EMAIL
                    && email != "worker@windmill.dev"
                    && email != SUPERADMIN_SECRET_EMAIL
                    && email != SUPERADMIN_SYNC_EMAIL
                    && email != SUPERADMIN_NOTIFICATION_EMAIL
                {
                    let user_usage = if let Some(user_usage) = user_usage {
                        user_usage
                    } else {
                        sqlx::query_scalar!(
                            "SELECT usage.usage + 1 FROM usage 
                            WHERE is_workspace IS FALSE AND
                            month_ = EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date)
                            AND id = $1",
                            email
                        )
                        .fetch_optional(_db)
                        .await?
                        .flatten()
                        .unwrap_or(1)
                    };

                    if user_usage > MAX_FREE_EXECS
                        && !matches!(job_payload, JobPayload::Dependencies { .. })
                        && !matches!(job_payload, JobPayload::FlowDependencies { .. })
                        && !matches!(job_payload, JobPayload::AppDependencies { .. })
                    {
                        return Err(error::Error::QuotaExceeded(format!(
                            "User {email} has exceeded the free usage limit of {MAX_FREE_EXECS} that applies outside of premium workspaces."
                        )));
                    }

                    let in_queue =
                        sqlx::query_scalar!("SELECT COUNT(id) FROM queue WHERE email = $1", email)
                            .fetch_one(_db)
                            .await?
                            .unwrap_or(0);

                    if in_queue > MAX_FREE_EXECS.into() {
                        return Err(error::Error::QuotaExceeded(format!(
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
                        return Err(error::Error::QuotaExceeded(format!(
                            "User {email} has exceeded the concurrent runs limit of {MAX_FREE_CONCURRENT_RUNS} that applies outside of premium workspaces."
                        )));
                    }
                }

                if workspace_id != "demo" {
                    let workspace_usage = if let Some(workspace_usage) = workspace_usage {
                        workspace_usage
                    } else {
                        sqlx::query_scalar!(
                        "SELECT usage.usage + 1 FROM usage 
                        WHERE is_workspace IS TRUE AND
                        month_ = EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date)
                        AND id = $1",
                        workspace_id
                    )
                    .fetch_optional(_db)
                    .await?
                    .flatten()
                    .unwrap_or(1)
                    };

                    if workspace_usage > MAX_FREE_EXECS
                        && !matches!(job_payload, JobPayload::Dependencies { .. })
                        && !matches!(job_payload, JobPayload::FlowDependencies { .. })
                        && !matches!(job_payload, JobPayload::AppDependencies { .. })
                    {
                        return Err(error::Error::QuotaExceeded(format!(
                            "Workspace {workspace_id} has exceeded the free usage limit of {MAX_FREE_EXECS} that applies outside of premium workspaces."
                        )));
                    }

                    let in_queue_workspace = sqlx::query_scalar!(
                        "SELECT COUNT(id) FROM queue WHERE workspace_id = $1",
                        workspace_id
                    )
                    .fetch_one(_db)
                    .await?
                    .unwrap_or(0);

                    if in_queue_workspace > MAX_FREE_EXECS.into() {
                        return Err(error::Error::QuotaExceeded(format!(
                            "Workspace {workspace_id} has exceeded the jobs in queue limit of {MAX_FREE_EXECS} that applies outside of premium workspaces."
                        )));
                    }

                    let concurrent_runs_workspace = sqlx::query_scalar!(
                        "SELECT COUNT(id) FROM queue WHERE running = true AND workspace_id = $1",
                        workspace_id
                    )
                    .fetch_one(_db)
                    .await?
                    .unwrap_or(0);

                    if concurrent_runs_workspace > MAX_FREE_CONCURRENT_RUNS.into() {
                        return Err(error::Error::QuotaExceeded(format!(
                            "Workspace {workspace_id} has exceeded the concurrent runs limit of {MAX_FREE_CONCURRENT_RUNS} that applies outside of premium workspaces."
                        )));
                    }
                }
            }
        }
    }

    let (
        script_hash,
        script_path,
        raw_code_tuple,
        job_kind,
        raw_flow,
        flow_status,
        language,
        custom_concurrency_key,
        concurrent_limit,
        concurrency_time_window_s,
        cache_ttl,
        dedicated_worker,
        _low_level_priority,
    ) = match job_payload {
        JobPayload::ScriptHash {
            hash,
            path,
            custom_concurrency_key,
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
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            dedicated_worker,
            priority,
        ),
        JobPayload::ScriptHub { path } => {
            if path == "hub/7771/slack" || path == "hub/7836/slack" {
                permissioned_as = SUPERADMIN_NOTIFICATION_EMAIL.to_string();
                email = SUPERADMIN_NOTIFICATION_EMAIL;
            }
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
                None,
            )
        }
        JobPayload::Code(RawCode {
            content,
            path,
            hash,
            language,
            lock,
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            dedicated_worker,
        }) => (
            hash,
            path,
            Some((content, lock)),
            JobKind::Preview,
            None,
            None,
            Some(language),
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            dedicated_worker,
            None,
        ),
        JobPayload::Dependencies { hash, language, path, dedicated_worker } => (
            Some(hash.0),
            Some(path),
            None,
            JobKind::Dependencies,
            None,
            None,
            Some(language),
            None,
            None,
            None,
            None,
            dedicated_worker,
            None,
        ),
        JobPayload::RawScriptDependencies { script_path, content, language } => (
            None,
            Some(script_path),
            Some((content, None)),
            JobKind::Dependencies,
            None,
            None,
            Some(language),
            None,
            None,
            None,
            None,
            None,
            None,
        ),
        JobPayload::FlowDependencies { path, dedicated_worker } => {
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
                dedicated_worker,
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
            None,
        ),
        JobPayload::RawFlow { mut value, path, restarted_from } => {
            add_virtual_items_if_necessary(&mut value.modules);

            let flow_status: FlowStatus = match restarted_from {
                Some(restarted_from_val) => {
                    let (_, _, step_n, truncated_modules, _, user_states, cleanup_module) =
                        restarted_flows_resolution(
                            _db,
                            workspace_id,
                            Some(value.clone()),
                            restarted_from_val.flow_job_id,
                            restarted_from_val.step_id.as_str(),
                            restarted_from_val.branch_or_iteration_n,
                        )
                        .await?;
                    FlowStatus {
                        step: step_n,
                        modules: truncated_modules,
                        // failure_module is reset
                        failure_module: Box::new(FlowStatusModuleWParent {
                            parent_module: None,
                            module_status: FlowStatusModule::WaitingForPriorSteps {
                                id: "failure".to_string(),
                            },
                        }),
                        cleanup_module,
                        // retry status is reset
                        retry: RetryStatus { fail_count: 0, failed_jobs: vec![] },
                        // TODO: for now, flows with approval conditions aren't supported for restart
                        approval_conditions: None,
                        restarted_from: Some(RestartedFrom {
                            flow_job_id: restarted_from_val.flow_job_id,
                            step_id: restarted_from_val.step_id,
                            branch_or_iteration_n: restarted_from_val.branch_or_iteration_n,
                        }),
                        user_states,
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
                value.concurrency_key.clone(),
                value.concurrent_limit.clone(),
                value.concurrency_time_window_s,
                value.cache_ttl.map(|x| x as i32),
                None,
                value.priority,
            )
        }
        JobPayload::SingleScriptFlow {
            path,
            hash,
            retry,
            args,
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            priority,
            tag_override,
        } => {
            let mut input_transforms = HashMap::<String, InputTransform>::new();
            for (arg_name, arg_value) in args {
                input_transforms.insert(arg_name, InputTransform::Static { value: arg_value });
            }
            let flow_value = FlowValue {
                modules: vec![FlowModule {
                    id: "a".to_string(),
                    value: windmill_common::worker::to_raw_value(
                        &windmill_common::flows::FlowModuleValue::Script {
                            input_transforms: input_transforms,
                            path: path.clone(),
                            hash: Some(hash),
                            tag_override: tag_override,
                        },
                    ),
                    stop_after_if: None,
                    summary: None,
                    suspend: None,
                    mock: None,
                    retry: Some(retry),
                    sleep: None,
                    cache_ttl: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                    continue_on_error: None,
                }],
                same_worker: false,
                failure_module: None,
                concurrency_time_window_s: concurrency_time_window_s,
                concurrent_limit: concurrent_limit,
                skip_expr: None,
                cache_ttl: cache_ttl.map(|val| val as u32),
                early_return: None,
                concurrency_key: custom_concurrency_key.clone(),
                priority: priority,
            };
            (
                None,
                Some(path),
                None,
                JobKind::Flow,
                Some(flow_value.clone()),
                Some(FlowStatus::new(&flow_value)), // this is a new flow being pushed, flow_status is set to flow_value
                None,
                custom_concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                cache_ttl,
                None,
                priority,
            )
        }
        JobPayload::Flow { path, dedicated_worker } => {
            let value_json = fetch_scalar_isolated!(
                sqlx::query_scalar!(
                    "SELECT value FROM flow WHERE path = $1 AND workspace_id = $2",
                    path,
                    workspace_id
                ),
                tx
            )?
            .ok_or_else(|| Error::InternalErr(format!("not found flow at path {:?}", path)))?;
            let mut value = serde_json::from_value::<FlowValue>(value_json).map_err(|err| {
                Error::InternalErr(format!(
                    "could not convert json to flow for {path}: {err:?}"
                ))
            })?;
            let priority = value.priority;
            add_virtual_items_if_necessary(&mut value.modules);
            let cache_ttl = value.cache_ttl.map(|x| x as i32).clone();
            let custom_concurrency_key = value.concurrency_key.clone();
            let concurrency_time_window_s = value.concurrency_time_window_s.clone();
            let concurrent_limit = value.concurrent_limit.clone();
            let status = Some(FlowStatus::new(&value));
            (
                None,
                Some(path),
                None,
                JobKind::Flow,
                Some(value),
                status, // this is a new flow being pushed, flow_status is set to flow_value
                None,
                custom_concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                cache_ttl,
                dedicated_worker,
                priority,
            )
        }
        JobPayload::RestartedFlow { completed_job_id, step_id, branch_or_iteration_n } => {
            let (
                flow_path,
                raw_flow,
                step_n,
                truncated_modules,
                priority,
                user_states,
                cleanup_module,
            ) = restarted_flows_resolution(
                _db,
                workspace_id,
                None,
                completed_job_id,
                step_id.as_str(),
                branch_or_iteration_n,
            )
            .await?;
            let restarted_flow_status = FlowStatus {
                step: step_n,
                modules: truncated_modules,
                // failure_module is reset
                failure_module: Box::new(FlowStatusModuleWParent {
                    parent_module: None,
                    module_status: FlowStatusModule::WaitingForPriorSteps {
                        id: "failure".to_string(),
                    },
                }),
                cleanup_module,
                // retry status is reset
                retry: RetryStatus { fail_count: 0, failed_jobs: vec![] },
                // TODO: for now, flows with approval conditions aren't supported for restart
                approval_conditions: None,
                restarted_from: Some(RestartedFrom {
                    flow_job_id: completed_job_id,
                    step_id,
                    branch_or_iteration_n,
                }),
                user_states,
            };
            (
                None,
                flow_path,
                None,
                JobKind::Flow,
                Some(raw_flow.clone()),
                Some(restarted_flow_status),
                None,
                raw_flow.concurrency_key,
                raw_flow.concurrent_limit,
                raw_flow.concurrency_time_window_s,
                raw_flow.cache_ttl.map(|x| x as i32),
                None,
                priority,
            )
        }
        JobPayload::DeploymentCallback { path } => (
            None,
            Some(path),
            None,
            JobKind::DeploymentCallback,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ),
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
        } else if _priority_override.is_some() {
            _priority_override
        } else {
            // else it takes the priority defined at the script/flow level, if it's a script or flow
            _low_level_priority
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
    }

    let (raw_code, raw_lock) = raw_code_tuple
        .map(|e| (Some(e.0), e.1))
        .unwrap_or_else(|| (None, None));

    let per_workspace: bool = DEFAULT_TAGS_PER_WORKSPACE.load(std::sync::atomic::Ordering::Relaxed);

    let tag = if dedicated_worker.is_some_and(|x| x) {
        format!(
            "{}:{}{}",
            workspace_id,
            if job_kind == JobKind::Flow || job_kind == JobKind::FlowDependencies {
                "flow/"
            } else {
                ""
            },
            script_path.clone().expect("dedicated script has a path")
        )
    } else if job_kind == JobKind::Script_Hub {
        if per_workspace {
            format!("hub-{}", workspace_id)
        } else {
            "hub".to_string()
        }
    } else {
        if tag == Some("".to_string()) {
            tag = None;
        }

        let interpolated_tag = tag.map(|x| interpolate_args(x, &args, workspace_id));

        let default = || {
            let ntag = if job_kind == JobKind::Flow
                || job_kind == JobKind::FlowPreview
                || job_kind == JobKind::SingleScriptFlow
                || job_kind == JobKind::Identity
            {
                "flow".to_string()
            } else if job_kind == JobKind::Dependencies
                || job_kind == JobKind::FlowDependencies
                || job_kind == JobKind::DeploymentCallback
            {
                // using the dependency tag for deployment callback for now. We can create a separate tag when we need
                "dependency".to_string()
            } else {
                "deno".to_string()
            };
            if per_workspace {
                format!("{}-{}", ntag, workspace_id)
            } else {
                ntag
            }
        };

        interpolated_tag.unwrap_or_else(|| {
            language
                .as_ref()
                .map(|x| {
                    if per_workspace {
                        format!("{}-{}", x.as_str(), workspace_id)
                    } else {
                        x.as_str().to_string()
                    }
                })
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

    if concurrent_limit.is_some() {
        let concurrency_key = custom_concurrency_key
            .map(|x| interpolate_args(x, &args, workspace_id))
            .unwrap_or(fullpath_with_workspace(
                workspace_id,
                script_path.as_ref(),
                &job_kind,
            ));
        sqlx::query!(
            "INSERT INTO concurrency_key(key, job_id) VALUES ($1, $2)",
            concurrency_key,
            job_id,
        )
        .execute(&mut tx)
        .await
        .map_err(|e| Error::InternalErr(format!("Could not insert concurrency_key={concurrency_key} for job_id={job_id} script_path={script_path:?} workspace_id={workspace_id}: {e}")))?;
    }

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
        Json(args) as Json<PushArgs>,
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
        if concurrent_limit.is_some() {  concurrency_time_window_s } else { None },
        custom_timeout,
        flow_step_id,
        cache_ttl,
        final_priority,
    )
    .fetch_one(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("Could not insert into queue {job_id} with tag {tag}, schedule_path {schedule_path:?}, script_path: {script_path:?}, email {email}, workspace_id {workspace_id}: {e}")))?;

    // TODO: technically the job isn't queued yet, as the transaction can be rolled back. Should be solved when moving these metrics to the queue abstraction.
    #[cfg(feature = "prometheus")]
    if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
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
            JobKind::SingleScriptFlow => "jobs.run.single_script_flow",
            JobKind::Script_Hub => "jobs.run.script_hub",
            JobKind::Dependencies => "jobs.run.dependencies",
            JobKind::Identity => "jobs.run.identity",
            JobKind::Noop => "jobs.run.noop",
            JobKind::FlowDependencies => "jobs.run.flow_dependencies",
            JobKind::AppDependencies => "jobs.run.app_dependencies",
            JobKind::DeploymentCallback => "jobs.run.deployment_callback",
        };

        let audit_author = if format!("u/{user}") != permissioned_as && user != permissioned_as {
            AuditAuthor {
                email: email.to_string(),
                username: permissioned_as.trim_start_matches("u/").to_string(),
                username_override: Some(user.to_string()),
            }
        } else {
            AuditAuthor {
                email: email.to_string(),
                username: user.to_string(),
                username_override: None,
            }
        };

        audit_log(
            &mut tx,
            &audit_author,
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
    flow_value_if_any: Option<FlowValue>,
    completed_flow_id: Uuid,
    restart_step_id: &str,
    branch_or_iteration_n: Option<usize>,
) -> Result<
    (
        Option<String>,
        FlowValue,
        i32,
        Vec<FlowStatusModule>,
        Option<i16>,
        HashMap<String, serde_json::Value>,
        FlowCleanupModule,
    ),
    Error,
> {
    let completed_job = sqlx::query_as::<_, CompletedJob>(
        "SELECT *, null as labels FROM completed_job WHERE id = $1 and workspace_id = $2",
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
        if flow_value_if_any
            .clone()
            .map(|fv| {
                fv.modules
                    .iter()
                    .find(|flow_value_module| flow_value_module.id == module.id())
                    .is_none()
            })
            .unwrap_or(false)
        {
            // skip module as it doesn't appear in the flow_value anymore
            continue;
        }
        if module.id() == restart_step_id {
            // if the module ID is the one we want to restart the flow at, or if it's past it in the flow,
            // set the module as WaitingForPriorSteps as it needs to be re-run
            if branch_or_iteration_n.is_none() || branch_or_iteration_n.unwrap() == 0 {
                // The module as WaitingForPriorSteps as the entire module (i.e. all the branches) need to be re-run
                truncated_modules.push(FlowStatusModule::WaitingForPriorSteps { id: module.id() });
            } else {
                // expect a module to be either a branchall (resp. loop), and resume the flow from this branch (resp. iteration)
                let branch_or_iteration_n = branch_or_iteration_n.unwrap();
                let module_definition = raw_flow
                    .modules
                    .iter()
                    .find(|flow_value_module| flow_value_module.id == restart_step_id)
                    .ok_or(Error::InternalErr(format!(
                        "Module {} not found in flow definition",
                        module.id()
                    )))?;

                match module_definition.get_value() {
                    Ok(FlowModuleValue::BranchAll { branches, parallel, .. }) => {
                        if parallel {
                            return Err(Error::InternalErr(format!(
                                "Module {} is a parallel branchall. It can only be restarted at a given branch if it's sequential",
                                restart_step_id,
                            )));
                        }
                        let total_branch_number = module.flow_jobs().map(|v| v.len()).unwrap_or(0);
                        if total_branch_number <= branch_or_iteration_n {
                            return Err(Error::InternalErr(format!(
                                "Branch-all module {} has only {} branches. It can't be restarted on branch {}",
                                restart_step_id,
                                total_branch_number,
                                branch_or_iteration_n,
                            )));
                        }
                        let mut new_flow_jobs = module.flow_jobs().unwrap_or_default();
                        new_flow_jobs.truncate(branch_or_iteration_n);
                        truncated_modules.push(FlowStatusModule::InProgress {
                            id: module.id(),
                            job: new_flow_jobs[new_flow_jobs.len() - 1], // set to last finished job from completed flow
                            iterator: None,
                            flow_jobs: Some(new_flow_jobs),
                            branch_chosen: None,
                            branchall: Some(BranchAllStatus {
                                branch: branch_or_iteration_n - 1, // Doing minus one here as this variable reflects the latest finished job in the iteration
                                len: branches.len(),
                            }),
                            parallel: parallel,
                            while_loop: false,
                        });
                    }
                    Ok(FlowModuleValue::ForloopFlow { parallel, .. }) => {
                        if parallel {
                            return Err(Error::InternalErr(format!(
                                "Module {} is not parallel loop. It can only be restarted at a given iteration if it's sequential",
                                restart_step_id,
                            )));
                        }
                        let total_iterations = module.flow_jobs().map(|v| v.len()).unwrap_or(0);
                        if total_iterations <= branch_or_iteration_n {
                            return Err(Error::InternalErr(format!(
                                "For-loop module {} doesn't cannot be restarted on iteration number {} as it has only {} iterations",
                                restart_step_id,
                                branch_or_iteration_n,
                                total_iterations,
                            )));
                        }
                        let mut new_flow_jobs = module.flow_jobs().unwrap_or_default();
                        new_flow_jobs.truncate(branch_or_iteration_n);

                        truncated_modules.push(FlowStatusModule::InProgress {
                            id: module.id(),
                            job: new_flow_jobs[new_flow_jobs.len() - 1], // set to last finished job from completed flow
                            iterator: Some(Iterator {
                                index: branch_or_iteration_n - 1, // same deal as above, this refers to the last finished job
                                itered: vec![], // Setting itered to empty array here, such that input transforms will be re-computed by worker_flows
                            }),
                            flow_jobs: Some(new_flow_jobs),
                            branch_chosen: None,
                            branchall: None,
                            parallel: parallel,
                            while_loop: false,
                        });
                    }
                    _ => {
                        return Err(Error::InternalErr(format!(
                            "Module {} is not a branchall or forloop, unable to restart it at step {:?}",
                            restart_step_id,
                            branch_or_iteration_n
                        )));
                    }
                }
            }
            dependent_module = true;
        } else if dependent_module {
            truncated_modules.push(FlowStatusModule::WaitingForPriorSteps { id: module.id() });
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
                    failed_retries: _,
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
        flow_status.user_states,
        flow_status.cleanup_module,
    ));
}
