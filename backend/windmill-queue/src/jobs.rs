/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{borrow::Borrow, collections::HashMap, sync::Arc, vec};

use anyhow::Context;
use async_recursion::async_recursion;
use chrono::{DateTime, Duration, Utc};
use futures::future::TryFutureExt;
use itertools::Itertools;
#[cfg(feature = "prometheus")]
use prometheus::IntCounter;
use regex::Regex;
use reqwest::Client;
use serde::{ser::SerializeMap, Serialize};
use serde_json::{json, value::RawValue};
use sqlx::{types::Json, FromRow, Pool, Postgres, Transaction};
use tokio::{sync::RwLock, time::sleep};
use ulid::Ulid;
use uuid::Uuid;
use windmill_audit::audit_ee::{audit_log, AuditAuthor};
use windmill_audit::ActionKind;

use windmill_common::utils::now_from_db;
use windmill_common::{
    auth::{fetch_authed_from_permissioned_as, permissioned_as_to_username},
    cache::{self, FlowData},
    db::{Authed, UserDB},
    error::{self, to_anyhow, Error},
    flow_status::{
        BranchAllStatus, FlowCleanupModule, FlowStatus, FlowStatusModule, FlowStatusModuleWParent,
        Iterator as FlowIterator, JobResult, RestartedFrom, RetryStatus, MAX_RETRY_ATTEMPTS,
        MAX_RETRY_INTERVAL,
    },
    flows::{
        add_virtual_items_if_necessary, FlowModule, FlowModuleValue, FlowValue, InputTransform,
    },
    jobs::{get_payload_tag_from_prefixed_path, JobKind, JobPayload, QueuedJob, RawCode},
    schedule::Schedule,
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang},
    users::{SUPERADMIN_NOTIFICATION_EMAIL, SUPERADMIN_SECRET_EMAIL},
    utils::{not_found_if_none, report_critical_error, StripPath, WarnAfterExt},
    worker::{
        to_raw_value, CLOUD_HOSTED, DEFAULT_TAGS_PER_WORKSPACE, DEFAULT_TAGS_WORKSPACES,
        DISABLE_FLOW_SCRIPT, MIN_VERSION_IS_AT_LEAST_1_432, MIN_VERSION_IS_AT_LEAST_1_440, NO_LOGS,
        WORKER_PULL_QUERIES, WORKER_SUSPENDED_PULL_QUERY,
    },
    DB, METRICS_ENABLED,
};

use backon::ConstantBuilder;
use backon::{BackoffBuilder, Retryable};

#[cfg(feature = "enterprise")]
use windmill_common::BASE_URL;

#[cfg(feature = "cloud")]
use windmill_common::users::SUPERADMIN_SYNC_EMAIL;

use crate::jobs_ee::update_concurrency_counter;
use crate::schedule::{get_schedule_opt, push_scheduled_job};

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


    pub static ref JOB_TOKEN: Option<String> = std::env::var("JOB_TOKEN").ok();

    static ref JOB_ARGS_AUDIT_LOGS: bool = std::env::var("JOB_ARGS_AUDIT_LOGS")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(false);
}

#[cfg(feature = "cloud")]
const MAX_FREE_EXECS: i32 = 1000;
#[cfg(feature = "cloud")]
const MAX_FREE_CONCURRENT_RUNS: i32 = 30;

const ERROR_HANDLER_USERNAME: &str = "error_handler";
const SCHEDULE_ERROR_HANDLER_USERNAME: &str = "schedule_error_handler";
#[cfg(feature = "enterprise")]
const SCHEDULE_RECOVERY_HANDLER_USERNAME: &str = "schedule_recovery_handler";
const ERROR_HANDLER_USER_GROUP: &str = "g/error_handler";
const ERROR_HANDLER_USER_EMAIL: &str = "error_handler@windmill.dev";
const SCHEDULE_ERROR_HANDLER_USER_EMAIL: &str = "schedule_error_handler@windmill.dev";
#[cfg(any(feature = "enterprise", feature = "cloud"))]
const SCHEDULE_RECOVERY_HANDLER_USER_EMAIL: &str = "schedule_recovery_handler@windmill.dev";

#[derive(Clone, Debug)]
pub struct CanceledBy {
    pub username: Option<String>,
    pub reason: Option<String>,
}

pub async fn cancel_single_job<'c>(
    username: &str,
    reason: Option<String>,
    job_running: Arc<QueuedJob>,
    w_id: &str,
    mut tx: Transaction<'c, Postgres>,
    db: &Pool<Postgres>,
    force_cancel: bool,
) -> error::Result<(Transaction<'c, Postgres>, Option<Uuid>)> {
    if force_cancel || (job_running.parent_job.is_none() && !job_running.running) {
        let username = username.to_string();
        let w_id = w_id.to_string();
        let db = db.clone();
        let job_running = job_running.clone();
        tokio::task::spawn(async move {
            let reason: String = reason
                .clone()
                .unwrap_or_else(|| "unexplicited reasons".to_string());
            let e = serde_json::json!({"message": format!("Job canceled: {reason} by {username}"), "name": "Canceled", "reason": reason, "canceler": username});
            append_logs(
                &job_running.id,
                w_id.to_string(),
                format!("canceled by {username}: (force cancel: {force_cancel})"),
                &db,
            )
            .await;
            let add_job = add_completed_job_error(
                &db,
                &job_running,
                job_running.mem_peak.unwrap_or(0),
                Some(CanceledBy { username: Some(username.to_string()), reason: Some(reason) }),
                e,
                "server",
                false,
                None,
            )
            .await;

            if let Err(e) = add_job {
                tracing::error!("Failed to add canceled job: {}", e);
            }
        });
    } else {
        let id: Option<Uuid> = sqlx::query_scalar!(
            "UPDATE v2_job_queue SET canceled_by = $1, canceled_reason = $2, scheduled_for = now(), suspend = 0 WHERE id = $3 AND workspace_id = $4 AND (canceled_by IS NULL OR canceled_reason != $2) RETURNING id",
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
    force_cancel: bool,
    require_anonymous: bool,
) -> error::Result<(Transaction<'c, Postgres>, Option<Uuid>)> {
    let job = get_queued_job_tx(id, &w_id, &mut tx).await?;

    if job.is_none() {
        return Ok((tx, None));
    }

    if require_anonymous && job.as_ref().unwrap().created_by != "anonymous" {
        return Err(Error::BadRequest(
            "You are not logged in and this job was not created by an anonymous user like you so you cannot cancel it".to_string(),
        ));
    }

    let mut job = job.unwrap();
    if force_cancel {
        // if force canceling a flow step, make sure we force cancel from the highest parent
        loop {
            if job.parent_job.is_none() {
                break;
            }
            match get_queued_job_tx(job.parent_job.unwrap(), &w_id, &mut tx).await? {
                Some(j) => {
                    job = j;
                }
                None => break,
            }
        }
    }

    // prevent cancelling a future tick of a schedule
    if let Some(schedule_path) = job.schedule_path.as_ref() {
        let now = now_from_db(&mut *tx).await?;
        if job.scheduled_for > now {
            return Err(Error::BadRequest(
                format!(
                    "Cannot cancel a future tick of a schedule, cancel the schedule direcly ({})",
                    schedule_path
                )
                .to_string(),
            ));
        }
    }

    let job = Arc::new(job);

    // get all children
    let mut jobs = vec![job.id];
    let mut jobs_to_cancel = vec![];
    while !jobs.is_empty() {
        let p_job = jobs.pop();
        let new_jobs = sqlx::query_scalar!(
            "SELECT id AS \"id!\" FROM v2_job_queue INNER JOIN v2_job USING (id) WHERE parent_job = $1 AND v2_job.workspace_id = $2",
            p_job,
            w_id
        )
        .fetch_all(&mut *tx)
        .await?;
        jobs.extend(new_jobs.clone());
        jobs_to_cancel.extend(new_jobs);
    }
    jobs.reverse();

    let (ntx, _) = cancel_single_job(
        username,
        reason.clone(),
        job.clone(),
        w_id,
        tx,
        db,
        force_cancel,
    )
    .await?;
    tx = ntx;

    // cancel children
    for job_id in jobs_to_cancel {
        let job = get_queued_job_tx(job_id, &w_id, &mut tx).await?;

        if let Some(job) = job {
            let (ntx, _) = cancel_single_job(
                username,
                reason.clone(),
                Arc::new(job),
                w_id,
                tx,
                db,
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
    job_id: &uuid::Uuid,
    workspace: impl AsRef<str>,
    logs: impl AsRef<str>,
    db: impl Borrow<Pool<Postgres>>,
) {
    if logs.as_ref().is_empty() {
        return;
    }

    if job_id.is_nil() {
        tracing::info!("local job: {}", logs.as_ref());
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
    .warn_after_seconds(1)
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
) -> error::Result<Vec<Uuid>> {
    // There can be only one perpetual script run per 10 seconds window. We execute the cancel twice with 5s interval
    // to avoid the case of a job _about to be restarted_ when the first cancel is run.
    let cancelled_job_ids_first_batch =
        cancel_persistent_script_jobs_internal(username, reason.clone(), script_path, w_id, db)
            .await?;
    sleep(std::time::Duration::from_secs(5)).await;
    let cancelled_job_ids_second_batch =
        cancel_persistent_script_jobs_internal(username, reason.clone(), script_path, w_id, db)
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
) -> error::Result<Vec<Uuid>> {
    let mut tx = db.begin().await?;

    // we could have retrieved the job IDs in the first query where we retrieve the hashes, but just in case a job was inserted in the queue right in-between the two above query, we re-do the fetch here
    let jobs_to_cancel = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM v2_as_queue WHERE workspace_id = $1 AND script_path = $2 AND canceled = false",
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
            false,
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
    fn wm_labels(&self) -> Option<Vec<String>>;
}

#[derive(serde::Deserialize)]
struct ResultLabels {
    wm_labels: Vec<String>,
}

impl ValidableJson for WrappedError {
    fn is_valid_json(&self) -> bool {
        true
    }

    fn wm_labels(&self) -> Option<Vec<String>> {
        None
    }
}

impl ValidableJson for Box<RawValue> {
    fn is_valid_json(&self) -> bool {
        !self.get().is_empty()
    }

    fn wm_labels(&self) -> Option<Vec<String>> {
        serde_json::from_str::<ResultLabels>(self.get())
            .ok()
            .map(|r| r.wm_labels)
    }
}

impl<T: ValidableJson> ValidableJson for Arc<T> {
    fn is_valid_json(&self) -> bool {
        T::is_valid_json(&self)
    }

    fn wm_labels(&self) -> Option<Vec<String>> {
        T::wm_labels(&self)
    }
}

impl ValidableJson for serde_json::Value {
    fn is_valid_json(&self) -> bool {
        true
    }

    fn wm_labels(&self) -> Option<Vec<String>> {
        serde_json::from_value::<ResultLabels>(self.clone())
            .ok()
            .map(|r| r.wm_labels)
    }
}

impl<T: ValidableJson> ValidableJson for Json<T> {
    fn is_valid_json(&self) -> bool {
        self.0.is_valid_json()
    }

    fn wm_labels(&self) -> Option<Vec<String>> {
        self.0.wm_labels()
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

pub async fn add_completed_job_error(
    db: &Pool<Postgres>,
    queued_job: &QueuedJob,
    mem_peak: i32,
    canceled_by: Option<CanceledBy>,
    e: serde_json::Value,
    _worker_name: &str,
    flow_is_done: bool,
    duration: Option<i64>,
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
        "job {} in {} did not succeed: {}",
        queued_job.id,
        queued_job.workspace_id,
        serde_json::to_string(&result).unwrap_or_else(|_| "".to_string())
    );
    let _ = add_completed_job(
        db,
        &queued_job,
        false,
        false,
        Json(&result),
        None,
        mem_peak,
        canceled_by,
        flow_is_done,
        duration,
    )
    .await?;
    Ok(result)
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_ERROR_HANDLER_PATH_IN_ADMINS_WORKSPACE: Option<String> = std::env::var("GLOBAL_ERROR_HANDLER_PATH_IN_ADMINS_WORKSPACE").ok();
}

pub async fn add_completed_job<T: Serialize + Send + Sync + ValidableJson>(
    db: &Pool<Postgres>,
    queued_job: &QueuedJob,
    success: bool,
    skipped: bool,
    result: Json<&T>,
    result_columns: Option<Vec<String>>,
    mem_peak: i32,
    canceled_by: Option<CanceledBy>,
    flow_is_done: bool,
    duration: Option<i64>,
) -> Result<Uuid, Error> {
    // tracing::error!("Start");
    // let start = tokio::time::Instant::now();

    // add_time!(bench, "add_completed_job start");
    if !result.is_valid_json() {
        return Err(Error::internal_err(
            "Result of job is invalid json (empty)".to_string(),
        ));
    }

    let result_columns = result_columns.as_ref();
    let _job_id = queued_job.id;
    let (opt_uuid, _duration, _skip_downstream_error_handlers) = (|| async {

        // let start = std::time::Instant::now();

        let mut tx = db.begin().await?;

        let job_id = queued_job.id;
        // tracing::error!("1 {:?}", start.elapsed());

        tracing::debug!(
            "completed job {} {}",
            queued_job.id,
            serde_json::to_string(&result).unwrap_or_else(|_| "".to_string())
        );

        let mem_peak = mem_peak.max(queued_job.mem_peak.unwrap_or(0));
        // add_time!(bench, "add_completed_job query START");

        let _duration =  sqlx::query_scalar!(
            "INSERT INTO v2_job_completed AS cj
                    ( workspace_id
                    , id
                    , started_at
                    , duration_ms
                    , result
                    , result_columns
                    , canceled_by
                    , canceled_reason
                    , flow_status
                    , workflow_as_code_status
                    , memory_peak
                    , status
                    , worker
                    )
                SELECT q.workspace_id, q.id, started_at, COALESCE($9::bigint, (EXTRACT('epoch' FROM (now())) - EXTRACT('epoch' FROM (COALESCE(started_at, now()))))*1000), $3, $10, $5, $6,
                        flow_status, workflow_as_code_status,
                        $8, CASE WHEN $4::BOOL THEN 'canceled'::job_status
                        WHEN $7::BOOL THEN 'skipped'::job_status
                        WHEN $2::BOOL THEN 'success'::job_status
                        ELSE 'failure'::job_status END AS status,
                        q.worker
                FROM v2_job_queue q LEFT JOIN v2_job_status USING (id) WHERE q.id = $1
            ON CONFLICT (id) DO UPDATE SET status = EXCLUDED.status, result = $3 RETURNING duration_ms AS \"duration_ms!\"",
            /* $1 */ queued_job.id,
            /* $2 */ success,
            /* $3 */ result as Json<&T>,
            /* $4 */ canceled_by.is_some(),
            /* $5 */ canceled_by.clone().map(|cb| cb.username).flatten(),
            /* $6 */ canceled_by.clone().map(|cb| cb.reason).flatten(),
            /* $7 */ skipped,
            /* $8 */ if mem_peak > 0 { Some(mem_peak) } else { None },
            /* $9 */ duration,
            /* $10 */ result_columns as Option<&Vec<String>>,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| Error::internal_err(format!("Could not add completed job {job_id}: {e:#}")))?;

        if let Some(labels) = result.wm_labels() {
            sqlx::query!(
                "UPDATE v2_job SET labels = (
                    SELECT array_agg(DISTINCT all_labels)
                    FROM unnest(coalesce(labels, ARRAY[]::TEXT[]) || $2) all_labels
                ) WHERE id = $1",
                job_id,
                labels as Vec<String>
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| Error::InternalErr(format!("Could not update job labels: {e:#}")))?;
        }

        if !queued_job.is_flow_step {
            if let Some(parent_job) = queued_job.parent_job {
                let _ = sqlx::query_scalar!(
                    "UPDATE v2_job_status SET
                        workflow_as_code_status = jsonb_set(
                            jsonb_set(
                                COALESCE(workflow_as_code_status, '{}'::jsonb),
                                array[$1],
                                COALESCE(workflow_as_code_status->$1, '{}'::jsonb)
                            ),
                            array[$1, 'duration_ms'],
                            to_jsonb($2::bigint)
                        )
                    WHERE id = $3",
                    &queued_job.id.to_string(),
                    _duration,
                    parent_job
                )
                .execute(&mut *tx)
                .await
                .inspect_err(|e| tracing::error!(
                    "Could not update parent job `duration_ms` in workflow as code status: {}",
                    e,
                ));
            }
        }
        // tracing::error!("Added completed job {:#?}", queued_job);

        let mut _skip_downstream_error_handlers = false;
        tx = delete_job(tx, &job_id).await?;
        // tracing::error!("3 {:?}", start.elapsed());

        if queued_job.is_flow_step {
            if let Some(parent_job) = queued_job.parent_job {
                // persist the flow last progress timestamp to avoid zombie flow jobs
                tracing::debug!(
                    "Persisting flow last progress timestamp to flow job: {:?}",
                    parent_job
                );
                sqlx::query!(
                    "UPDATE v2_job_runtime r SET
                        ping = now()
                    FROM v2_job_queue q
                    WHERE r.id = $1 AND q.id = r.id
                        AND q.workspace_id = $2
                        AND canceled_by IS NULL",
                    parent_job,
                    &queued_job.workspace_id
                )
                .execute(&mut *tx)
                .await?;
                if flow_is_done {
                    let r = sqlx::query_scalar!(
                    "UPDATE parallel_monitor_lock SET last_ping = now() WHERE parent_flow_id = $1 and job_id = $2 RETURNING 1",
                    parent_job,
                    &queued_job.id
                ).fetch_optional(&mut *tx).await?;
                    if r.is_some() {
                        tracing::info!(
                            "parallel flow iteration is done, setting parallel monitor last ping lock for job {}",
                            &queued_job.id
                        );
                    }
                }
            }
        } else {
            if queued_job.schedule_path.is_some() && queued_job.script_path.is_some() {
                let schedule_path = queued_job.schedule_path.as_ref().unwrap();
                let script_path = queued_job.script_path.as_ref().unwrap();

                let schedule =
                    get_schedule_opt(&mut *tx, &queued_job.workspace_id, schedule_path).await?;

                if let Some(schedule) = schedule {
                    #[cfg(feature = "enterprise")]
                    {
                        _skip_downstream_error_handlers = schedule.ws_error_handler_muted;
                    }

                    // for scripts, always try to schedule next tick
                    // for flows, only try to schedule next tick here if flow failed and because first handle_flow failed (step = 0, modules[0] = {type: 'Failure', 'job': uuid::nil()}) or job was cancelled before first handle_flow was called (step = 0, modules = [] OR modules[0].type == 'WaitingForPriorSteps')
                    // otherwise flow rescheduling is done inside handle_flow
                    let schedule_next_tick = !queued_job.is_flow()
                        || !success && sqlx::query_scalar!(
                            "SELECT 
                                flow_status->>'step' = '0' 
                                AND (
                                    jsonb_array_length(flow_status->'modules') = 0 
                                    OR flow_status->'modules'->0->>'type' = 'WaitingForPriorSteps' 
                                    OR (
                                        flow_status->'modules'->0->>'type' = 'Failure' 
                                        AND flow_status->'modules'->0->>'job' = $1
                                    )
                                )
                            FROM v2_job_completed WHERE id = $2 AND workspace_id = $3",
                            Uuid::nil().to_string(),
                            &queued_job.id,
                            &queued_job.workspace_id
                        ).fetch_optional(&mut *tx).await?.flatten().unwrap_or(false);

                    if schedule_next_tick {
                        if let Err(err) = handle_maybe_scheduled_job(
                            db,
                            queued_job,
                            &schedule,
                            script_path,
                            &queued_job.workspace_id,
                        )
                        .await
                        {
                            match err {
                                Error::QuotaExceeded(_) => (),
                                // scheduling next job failed and could not disable schedule => make zombie job to retry
                                _ => return Ok((Some(job_id), 0, true)),
                            }
                        };
                    }

                    #[cfg(feature = "enterprise")]
                    if let Err(err) = apply_schedule_handlers(
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
            if *DISABLE_CONCURRENCY_LIMIT {
                tracing::warn!("Concurrency limit is disabled, skipping");
            } else {
                if let Err(e) = sqlx::query_scalar!(
                    "UPDATE concurrency_counter SET job_uuids = job_uuids - $2 WHERE concurrency_id = $1",
                    concurrency_key,
                    queued_job.id.hyphenated().to_string(),
                )
                .execute(&mut *tx)
                .await
                {
                    tracing::error!("Could not decrement concurrency counter: {}", e);
                }
            }

            if let Err(e) = sqlx::query_scalar!(
                "UPDATE concurrency_key SET ended_at = now() WHERE job_id = $1",
                queued_job.id,
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                Error::internal_err(format!(
                    "Error updating to add ended_at timestamp concurrency_key={concurrency_key}: {e:#}"
                ))
            }) {
                tracing::error!("Could not update concurrency_key: {}", e);
            }
            tracing::debug!("decremented concurrency counter");
        }

        if JOB_TOKEN.is_none() {
            sqlx::query!("DELETE FROM job_perms WHERE job_id = $1", job_id)
                .execute(&mut *tx)
                .await?;
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
        // tracing::info!("completed job: {:?}", start.elapsed().as_micros());
        Ok((None, _duration, _skip_downstream_error_handlers)) as windmill_common::error::Result<(Option<Uuid>, i64, bool)>
    })
    .retry(
        ConstantBuilder::default()
            .with_delay(std::time::Duration::from_secs(3))
            .with_max_times(5)
            .build(),
    )
    .when(|err| !matches!(err, Error::QuotaExceeded(_)))
    .notify(|err, dur| {
        tracing::error!(
            "Could not insert completed job, retrying in {dur:#?}, err: {err:#?}"
        );
    })
    .sleep(tokio::time::sleep)
    .await?;

    // if scheduling next job failed, return the job_id early to ensure the job get retried after a timeout
    if let Some(job_id) = opt_uuid {
        return Ok(job_id);
    }

    #[cfg(feature = "cloud")]
    if *CLOUD_HOSTED && !queued_job.is_flow() && _duration > 1000 {
        let additional_usage = _duration / 1000;
        let w_id = &queued_job.workspace_id;
        let premium_workspace =
            sqlx::query_scalar!("SELECT premium FROM workspace WHERE id = $1", w_id)
                .fetch_one(db)
                .await
                .map_err(|e| {
                    Error::internal_err(format!("fetching if {w_id} is premium: {e:#}"))
                })?;
        let _ = sqlx::query!(
            "INSERT INTO usage (id, is_workspace, month_, usage) 
            VALUES ($1, TRUE, EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date), $2) 
            ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + $2",
            w_id,
            additional_usage as i32
        )
        .execute(db)
        .await
        .map_err(|e| Error::internal_err(format!("updating usage: {e:#}")));

        if !premium_workspace {
            let _ = sqlx::query!(
                "INSERT INTO usage (id, is_workspace, month_, usage) 
                VALUES ($1, FALSE, EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date), $2) 
                ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + $2",
                queued_job.email,
                additional_usage as i32
            )
            .execute(db)
            .await
            .map_err(|e| Error::internal_err(format!("updating usage: {e:#}")));
        }
    }

    #[cfg(feature = "enterprise")]
    if !success {
        async fn has_failure_module(db: &Pool<Postgres>, job: &QueuedJob) -> bool {
            if let Ok(flow) = cache::job::fetch_flow(db, job.job_kind, job.script_hash).await {
                return flow.value().failure_module.is_some();
            }
            sqlx::query_scalar!(
                "SELECT raw_flow->'failure_module' != 'null'::jsonb FROM v2_job WHERE id = $1",
                job.id
            )
            .fetch_one(db)
            .await
            .unwrap_or(Some(false))
            .unwrap_or(false)
        }

        if queued_job.email == ERROR_HANDLER_USER_EMAIL {
            let base_url = BASE_URL.read().await;
            let w_id = &queued_job.workspace_id;
            report_critical_error(
                format!(
                    "Workspace error handler job failed ({base_url}/run/{}?workspace={w_id}){}",
                    queued_job.id,
                    queued_job
                        .parent_job
                        .map(|id| format!(
                            " trying to handle failed job ({base_url}/run/{id}?workspace={w_id})"
                        ))
                        .unwrap_or("".to_string()),
                ),
                db.clone(),
                Some(&w_id),
                None,
            )
            .await;
        } else if queued_job.email == SCHEDULE_ERROR_HANDLER_USER_EMAIL {
            let base_url = BASE_URL.read().await;
            let w_id = &queued_job.workspace_id;
            report_error_to_workspace_handler_or_critical_side_channel(
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
        } else if !_skip_downstream_error_handlers
            && (matches!(queued_job.job_kind, JobKind::Script)
                || matches!(queued_job.job_kind, JobKind::Flow)
                    && !has_failure_module(db, queued_job).await)
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
            if let Err(e) = send_error_to_global_handler(&queued_job, db, Json(&result)).await {
                tracing::error!(
                    "Could not run global error handler for job {}: {}",
                    &queued_job.id,
                    e
                );
            }

            if let Err(err) = send_error_to_workspace_handler(
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
                        ), db.clone(), Some(&w_id), None)
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
                let tx = PushIsolationLevel::IsolatedRoot(db.clone());

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

                let ehm = HashMap::new();
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
                        apply_preprocessor: false,
                    },
                    queued_job
                        .args
                        .as_ref()
                        .map(|x| PushArgs::from(&x.0))
                        .unwrap_or_else(|| PushArgs::from(&ehm)),
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
                    None,
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

pub async fn send_error_to_global_handler<'a, T: Serialize + Send + Sync>(
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

pub async fn report_error_to_workspace_handler_or_critical_side_channel(
    queued_job: &QueuedJob,
    db: &Pool<Postgres>,
    error_message: String,
) -> () {
    let w_id = &queued_job.workspace_id;
    let (error_handler, error_handler_extra_args) = sqlx::query_as::<_, (Option<String>, Option<Json<Box<RawValue>>>)>(
        "SELECT error_handler, error_handler_extra_args FROM workspace_settings WHERE workspace_id = $1",
    )
    .bind(&w_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .unwrap_or((None, None));

    if let Some(error_handler) = error_handler {
        if let Err(err) = push_error_handler(
            db,
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
            report_critical_error(error_message, db.clone(), Some(&w_id), None).await;
        }
    } else {
        report_critical_error(error_message, db.clone(), Some(&w_id), None).await;
    }
}

pub async fn send_error_to_workspace_handler<'a, 'c, T: Serialize + Send + Sync>(
    queued_job: &QueuedJob,
    is_canceled: bool,
    db: &Pool<Postgres>,
    result: Json<&'a T>,
) -> Result<(), Error> {
    let w_id = &queued_job.workspace_id;
    let (error_handler, error_handler_extra_args, error_handler_muted_on_cancel) = sqlx::query_as::<_, (Option<String>, Option<Json<Box<RawValue>>>, bool)>(
        "SELECT error_handler, error_handler_extra_args, error_handler_muted_on_cancel FROM workspace_settings WHERE workspace_id = $1",
    )
    .bind(&w_id)
    .fetch_optional(db)
    .await
    .context("fetching error handler info from workspace_settings")?
    .ok_or_else(|| Error::internal_err(format!("no workspace settings for id {w_id}")))?;

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

pub async fn handle_maybe_scheduled_job<'c>(
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
        let push_next_job_future = (|| {
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                let mut tx = db.begin().await?;
                tx = push_scheduled_job(db, tx, &schedule, None).await?;
                tx.commit().await?;
                Ok::<(), Error>(())
            })
            .map_err(|e| Error::internal_err(format!("Pushing next scheduled job timedout: {e:#}")))
            .unwrap_or_else(|e| Err(e))
        })
        .retry(
            ConstantBuilder::default()
                .with_delay(std::time::Duration::from_secs(5))
                .with_max_times(10)
                .build(),
        )
        .when(|err| !matches!(err, Error::QuotaExceeded(_)))
        .notify(|err, dur| {
            tracing::error!(
                "Could not push next scheduled job, retrying in {dur:#?}, err: {err:#?}"
            );
        })
        .sleep(tokio::time::sleep);
        match push_next_job_future.await {
            Ok(()) => Ok(()),
            Err(err) => {
                let update_schedule = sqlx::query!(
                    "UPDATE schedule SET enabled = false, error = $1 WHERE workspace_id = $2 AND path = $3",
                    err.to_string(),
                    &schedule.workspace_id,
                    &schedule.path
                )
                .execute(db)
                .await;
                match update_schedule {
                    Ok(_) => {
                        match err {
                            Error::QuotaExceeded(_) => {}
                            _ => {
                                report_error_to_workspace_handler_or_critical_side_channel(job, db,
                                    format!("Could not schedule next job for {} with err {}. Schedule disabled", schedule.path, err.to_string()),
                                ).await;
                            }
                        }
                        Ok(())
                    }
                    Err(disable_err) => match err {
                        Error::QuotaExceeded(_) => Err(err),
                        _ => {
                            report_error_to_workspace_handler_or_critical_side_channel(job, db,
                                format!("Could not schedule next job for {} and could not disable schedule with err {}.", schedule.path, disable_err),
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

#[cfg(feature = "enterprise")]
async fn apply_schedule_handlers<'a, 'c, T: Serialize + Send + Sync>(
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
        if let Some(on_failure_path) = schedule.on_failure.clone() {
            let times = schedule.on_failure_times.unwrap_or(1).max(1);
            let exact = schedule.on_failure_exact.unwrap_or(false);
            if times > 1 || exact {
                let past_jobs = sqlx::query!(
                    // Query plan:
                    // - use of the  `ix_v2_job_root_by_path` index;
                    //   hence the `parent_job IS NULL` clause.
                    // - select from `v2_job` first, then join with `v2_job_completed` to avoid a full
                    //   table scan.
                    "SELECT status = 'success' AS \"success!\"
                    FROM v2_job j JOIN v2_job_completed USING (id)
                    WHERE j.workspace_id = $1 AND trigger_kind = 'schedule' AND trigger = $2
                        AND parent_job IS NULL
                        AND runnable_path = $3
                        AND j.id != $4
                    ORDER BY created_at DESC
                    LIMIT $5",
                    &schedule.workspace_id,
                    &schedule.path,
                    script_path,
                    job_id,
                    if exact { times } else { times - 1 } as i64
                )
                .fetch_all(db)
                .await?;

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
        if let Some(ref on_success_path) = schedule.on_success {
            handle_successful_schedule(
                db,
                job_id,
                &schedule.path,
                script_path,
                schedule.is_flow,
                w_id,
                on_success_path,
                result,
                started_at,
                schedule.on_success_extra_args.clone(),
            )
            .await?;
        }

        if let Some(ref on_recovery_path) = schedule.on_recovery.clone() {
            let tx = db.begin().await?;
            let times = schedule.on_recovery_times.unwrap_or(1).max(1);
            let past_jobs = sqlx::query!(
                // Query plan:
                // - use of the  `ix_v2_job_root_by_path` index;
                //   hence the `parent_job IS NULL` clause.
                // - select from `v2_job` first, then join with `v2_job_completed` to avoid a full
                //   table scan.
                "SELECT status = 'success' AS \"success!\",
                    result AS \"result: Json<Box<RawValue>>\",
                    started_at AS \"started_at!\"\
                FROM v2_job j JOIN v2_job_completed USING (id)
                WHERE j.workspace_id = $1 AND trigger_kind = 'schedule' AND trigger = $2
                    AND parent_job IS NULL
                    AND runnable_path = $3
                    AND j.id != $4
                ORDER BY created_at DESC
                LIMIT $5",
                &schedule.workspace_id,
                &schedule.path,
                script_path,
                job_id,
                times as i64
            )
            .fetch_all(db)
            .await?;

            if past_jobs.len() < times as usize {
                return Ok(());
            }

            let n_times_successful = past_jobs[..(times - 1) as usize].iter().all(|j| j.success);

            if !n_times_successful {
                return Ok(());
            }

            let failed_job = &past_jobs[past_jobs.len() - 1];

            if !failed_job.success {
                handle_recovered_schedule(
                    db,
                    tx,
                    job_id,
                    &schedule.path,
                    script_path,
                    schedule.is_flow,
                    w_id,
                    &on_recovery_path,
                    failed_job.result.as_ref().map(AsRef::as_ref),
                    failed_job.started_at,
                    result,
                    times,
                    started_at,
                    schedule.on_recovery_extra_args.clone(),
                )
                .await?;
            } else {
                tx.commit().await?;
            }
        }
    }

    Ok(())
}

pub async fn push_error_handler<'a, 'c, T: Serialize + Send + Sync>(
    db: &Pool<Postgres>,
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
    let (payload, tag, on_behalf_of) =
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

    let result = sanitize_result(result);

    let (email, permissioned_as) = if let Some(on_behalf_of) = on_behalf_of.as_ref() {
        (
            on_behalf_of.email.as_str(),
            on_behalf_of.permissioned_as.clone(),
        )
    } else if is_global_error_handler {
        (SUPERADMIN_SECRET_EMAIL, SUPERADMIN_SECRET_EMAIL.to_string())
    } else if is_schedule_error_handler {
        (
            SCHEDULE_ERROR_HANDLER_USER_EMAIL,
            ERROR_HANDLER_USER_GROUP.to_string(),
        )
    } else {
        (
            ERROR_HANDLER_USER_EMAIL,
            ERROR_HANDLER_USER_GROUP.to_string(),
        )
    };

    let tx = PushIsolationLevel::IsolatedRoot(db.clone());
    let (uuid, tx) = push(
        &db,
        tx,
        handler_w_id,
        payload,
        PushArgs { extra: Some(extra), args: &result },
        if is_global_error_handler {
            "global"
        } else if is_schedule_error_handler {
            SCHEDULE_ERROR_HANDLER_USERNAME
        } else {
            ERROR_HANDLER_USERNAME
        },
        email,
        permissioned_as,
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
        None,
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
#[cfg(feature = "enterprise")]
async fn handle_recovered_schedule<'a, 'c, T: Serialize + Send + Sync>(
    db: &Pool<Postgres>,
    tx: Transaction<'c, Postgres>,
    job_id: Uuid,
    schedule_path: &str,
    script_path: &str,
    is_flow: bool,
    w_id: &str,
    on_recovery_path: &str,
    result: Option<&Box<RawValue>>,
    started_at: DateTime<Utc>,
    successful_job_result: Json<&'a T>,
    successful_times: i32,
    successful_job_started_at: DateTime<Utc>,
    extra_args: Option<Json<Box<RawValue>>>,
) -> windmill_common::error::Result<()> {
    let (payload, tag, on_behalf_of) =
        get_payload_tag_from_prefixed_path(on_recovery_path, db, w_id).await?;

    let mut extra = HashMap::new();
    extra.insert("error_started_at".to_string(), to_raw_value(&started_at));
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

    let args = result
        .and_then(|x| serde_json::from_str::<HashMap<String, Box<RawValue>>>(x.get()).ok())
        .unwrap_or_else(HashMap::new);

    let (email, permissioned_as) = if let Some(on_behalf_of) = on_behalf_of.as_ref() {
        (
            on_behalf_of.email.as_str(),
            on_behalf_of.permissioned_as.clone(),
        )
    } else {
        (
            SCHEDULE_RECOVERY_HANDLER_USER_EMAIL,
            ERROR_HANDLER_USER_GROUP.to_string(),
        )
    };

    let tx = PushIsolationLevel::Transaction(tx);
    let (uuid, tx) = push(
        &db,
        tx,
        w_id,
        payload,
        PushArgs { extra: Some(extra), args: &args },
        SCHEDULE_RECOVERY_HANDLER_USERNAME,
        email,
        permissioned_as,
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
        None,
    )
    .await?;
    tracing::info!(
        "Pushed on_recovery job {} for {} to queue",
        uuid,
        schedule_path
    );
    tx.commit().await?;
    Ok(())
}

#[cfg(feature = "enterprise")]
async fn handle_successful_schedule<'a, 'c, T: Serialize + Send + Sync>(
    db: &Pool<Postgres>,
    job_id: Uuid,
    schedule_path: &str,
    script_path: &str,
    is_flow: bool,
    w_id: &str,
    on_success_path: &str,
    successful_job_result: Json<&'a T>,
    successful_job_started_at: DateTime<Utc>,
    extra_args: Option<Json<Box<RawValue>>>,
) -> windmill_common::error::Result<()> {
    let (payload, tag, on_behalf_of) =
        get_payload_tag_from_prefixed_path(on_success_path, db, w_id).await?;

    let mut extra = HashMap::new();
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

    let (email, permissioned_as) = if let Some(on_behalf_of) = on_behalf_of.as_ref() {
        (
            on_behalf_of.email.as_str(),
            on_behalf_of.permissioned_as.clone(),
        )
    } else {
        (
            SCHEDULE_RECOVERY_HANDLER_USER_EMAIL,
            ERROR_HANDLER_USER_GROUP.to_string(),
        )
    };

    let tx = PushIsolationLevel::IsolatedRoot(db.clone());
    let (uuid, tx) = push(
        &db,
        tx,
        w_id,
        payload,
        PushArgs { extra: Some(extra), args: &HashMap::new() },
        SCHEDULE_RECOVERY_HANDLER_USERNAME,
        email,
        permissioned_as,
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
        None,
    )
    .await?;
    tracing::info!(
        "Pushed on_success job {} for {} to queue",
        uuid,
        schedule_path
    );
    tx.commit().await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
pub struct PulledJob {
    #[sqlx(flatten)]
    pub job: QueuedJob,
    pub raw_code: Option<String>,
    pub raw_lock: Option<String>,
    pub raw_flow: Option<Json<Box<RawValue>>>,
}

impl std::ops::Deref for PulledJob {
    type Target = QueuedJob;
    fn deref(&self) -> &Self::Target {
        &self.job
    }
}

lazy_static::lazy_static! {
    static ref DISABLE_CONCURRENCY_LIMIT: bool = std::env::var("DISABLE_CONCURRENCY_LIMIT").is_ok_and(|s| s == "true");
}

pub async fn pull(
    db: &Pool<Postgres>,
    suspend_first: bool,
    worker_name: &str,
) -> windmill_common::error::Result<(Option<PulledJob>, bool)> {
    loop {
        let (job, suspended) = pull_single_job_and_mark_as_running_no_concurrency_limit(
            db,
            suspend_first,
            worker_name,
        )
        .await?;

        let Some(job) = job else {
            return Ok((None, suspended));
        };

        let has_concurent_limit = job.concurrent_limit.is_some();

        #[cfg(not(feature = "enterprise"))]
        if has_concurent_limit {
            tracing::error!("Concurrent limits are an EE feature only, ignoring constraints")
        }

        #[cfg(not(feature = "enterprise"))]
        let has_concurent_limit = false;

        // concurrency check. If more than X jobs for this path are already running, we re-queue and pull another job from the queue
        let pulled_job = job;
        if pulled_job.script_path.is_none() || !has_concurent_limit || pulled_job.canceled {
            #[cfg(feature = "prometheus")]
            if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                QUEUE_PULL_COUNT.inc();
            }
            return Ok((Option::Some(pulled_job), suspended));
        }

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

        let jobs_uuids_init_json_value = serde_json::from_str::<serde_json::Value>(
            format!("{{\"{}\": {{}}}}", pulled_job.id.hyphenated().to_string()).as_str(),
        )
        .expect("Unable to serialize job_uuids column to proper JSON");

        let (within_limit, max_ended_at) = if *DISABLE_CONCURRENCY_LIMIT {
            tracing::warn!("Concurrency limit is disabled, skipping");
            (true, None)
        } else {
            update_concurrency_counter(
                db,
                &pulled_job.id,
                job_concurrency_key.clone(),
                jobs_uuids_init_json_value,
                pulled_job.id.hyphenated().to_string(),
                job_custom_concurrency_time_window_s,
                job_custom_concurrent_limit,
            )
            .await?
        };
        if within_limit {
            #[cfg(feature = "prometheus")]
            if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                QUEUE_PULL_COUNT.inc();
            }
            return Ok((Option::Some(pulled_job), suspended));
        }

        let job_script_path = pulled_job.script_path.clone().unwrap_or_default();

        let min_started_at = sqlx::query!(
            "SELECT COALESCE((SELECT MIN(started_at) as min_started_at
            FROM v2_job_queue INNER JOIN v2_job ON v2_job.id = v2_job_queue.id
            WHERE v2_job.runnable_path = $1 AND v2_job.kind != 'dependencies'  AND v2_job_queue.running = true AND v2_job_queue.workspace_id = $2 AND v2_job_queue.canceled_by IS NULL AND v2_job.concurrent_limit > 0), $3) as min_started_at, now() AS now",
            job_script_path,
            &pulled_job.workspace_id,
            max_ended_at
        )
        .fetch_one(db)
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Error getting min started at for script path {job_script_path}: {e:#}"
            ))
        })?;

        let job_uuid: Uuid = pulled_job.id;
        let avg_script_duration: Option<i64> = sqlx::query_scalar!(
            "SELECT CAST(ROUND(AVG(duration_ms), 0) AS BIGINT) AS avg_duration_s FROM
                (SELECT duration_ms FROM concurrency_key LEFT JOIN v2_job_completed ON v2_job_completed.id = concurrency_key.job_id WHERE key = $1 AND ended_at IS NOT NULL
                ORDER BY ended_at
                DESC LIMIT 10) AS t",
            job_concurrency_key
        )
        .fetch_one(db)
        .await?;
        tracing::debug!(
            "avg script duration computed: {}",
            avg_script_duration.unwrap_or(0)
        );

        // optimal scheduling is: 'older_job_in_concurrency_time_window_started_timestamp + script_avg_duration + concurrency_time_window_s'
        let inc = Duration::try_milliseconds(
            avg_script_duration.map(|x| i64::from(x + 100)).unwrap_or(0),
        )
        .unwrap_or_default()
        .max(Duration::try_seconds(1).unwrap_or_default())
            + Duration::try_seconds(i64::from(job_custom_concurrency_time_window_s))
                .unwrap_or_default();

        let now = min_started_at.now.unwrap();
        let min_started_at_or_now = min_started_at.min_started_at.unwrap_or(now);
        let min_started_p_inc =
            (min_started_at_or_now + inc).max(now + Duration::try_seconds(3).unwrap_or_default());

        let mut estimated_next_schedule_timestamp = min_started_p_inc;
        let all_jobs = sqlx::query_scalar!(
            "SELECT scheduled_for FROM v2_job_queue  INNER JOIN concurrency_key ON concurrency_key.job_id = v2_job_queue.id
             WHERE key = $1 AND running = false AND canceled_by IS NULL AND scheduled_for >= $2",
            job_concurrency_key,
            estimated_next_schedule_timestamp - inc
        ).fetch_all(db).await?;

        tracing::debug!(
            "all_jobs: {:?}, estimated_next_schedule_timestamp: {:?}, inc: {:?}",
            all_jobs,
            estimated_next_schedule_timestamp,
            inc
        );
        let mut i = 0;
        loop {
            let jobs_in_window = all_jobs
                .iter()
                .filter(|&scheduled_for| scheduled_for <= &estimated_next_schedule_timestamp)
                .count() as i32
                - (job_custom_concurrent_limit * i);

            tracing::debug!("estimated_next_schedule_timestamp: {:?}, jobs_in_window: {jobs_in_window}, inc: {inc}", estimated_next_schedule_timestamp);

            if jobs_in_window < job_custom_concurrent_limit || *DISABLE_CONCURRENCY_LIMIT {
                break;
            } else {
                i += 1;
                estimated_next_schedule_timestamp = estimated_next_schedule_timestamp + inc;
            }
        }

        tracing::info!("Job '{}' from path '{}' with concurrency key '{}' has reached its concurrency limit of {} jobs run in the last {} seconds. This job will be re-queued for next execution at {} (min_started_at: {min_started_at_or_now}, avg script duration: {:?}, number of time windows full: {})", 
            job_uuid, job_script_path,  job_concurrency_key, job_custom_concurrent_limit, job_custom_concurrency_time_window_s, estimated_next_schedule_timestamp, avg_script_duration, i);

        let job_log_event = format!(
            "\nRe-scheduled job to {estimated_next_schedule_timestamp} due to concurrency limits with key {job_concurrency_key} and limit {job_custom_concurrent_limit} in the last {job_custom_concurrency_time_window_s} seconds (min_started_at: {min_started_at_or_now}, avg script duration: {:?}, number of time windows full: {})\n",
            avg_script_duration, i
        );
        let _ = append_logs(&job_uuid, &pulled_job.workspace_id, job_log_event, db).await;

        sqlx::query!(
            "
            WITH ping AS (
                UPDATE v2_job_runtime SET ping = null WHERE id = $2
            )
            UPDATE v2_job_queue SET
                running = false,
                started_at = null,
                scheduled_for = $1
            WHERE id = $2",
            estimated_next_schedule_timestamp,
            job_uuid,
        )
        .execute(db)
        .await
        .map_err(|e| Error::internal_err(format!("Could not update and re-queue job {job_uuid}. The job will be marked as running but it is not running: {e:#}")))?;
    }
}

async fn pull_single_job_and_mark_as_running_no_concurrency_limit<'c>(
    db: &Pool<Postgres>,
    suspend_first: bool,
    worker_name: &str,
) -> windmill_common::error::Result<(Option<PulledJob>, bool)> {
    let job_and_suspended: (Option<PulledJob>, bool) = {
        /* Jobs can be started if they:
         * - haven't been started before,
         *   running = false
         * - are flows with a step that needed resume,
         *   suspend_until is non-null
         *   and suspend = 0 when the resume messages are received
         *   or suspend_until <= now() if it has timed out */
        let query = WORKER_SUSPENDED_PULL_QUERY.read().await;

        if query.is_empty() {
            tracing::warn!("No suspended pull queries available");
            return Ok((None, false));
        }

        let r = if suspend_first {
            // tracing::info!("Pulling job with query: {}", query);
            sqlx::query_as::<_, PulledJob>(&query)
                .bind(worker_name)
                .fetch_optional(db)
                .await?
        } else {
            None
        };
        if r.is_none() {
            // #[cfg(feature = "benchmark")]
            // let instant = Instant::now();
            let mut highest_priority_job: Option<PulledJob> = None;

            let queries = WORKER_PULL_QUERIES.read().await;

            if queries.is_empty() {
                tracing::warn!("No pull queries available");
                return Ok((None, false));
            }

            for query in queries.iter() {
                // tracing::info!("Pulling job with query: {}", query);
                // let instant = std::time::Instant::now();
                let r = sqlx::query_as::<_, PulledJob>(query)
                    .bind(worker_name)
                    .fetch_optional(db)
                    .await?;

                if let Some(pulled_job) = r {
                    // tracing::info!("pulled job: {:?}", instant.elapsed().as_micros());

                    highest_priority_job = Some(pulled_job);
                    break;
                }
                // else continue pulling for lower priority tags
            }

            // #[cfg(feature = "benchmark")]
            // println!("pull query: {:?}", instant.elapsed());
            (highest_priority_job, false)
        } else {
            (r, true)
        }
    };
    Ok(job_and_suspended)
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
            "SELECT flow_version.value->>'concurrency_key'
            FROM flow 
            LEFT JOIN flow_version
                ON flow_version.id = flow.versions[array_upper(flow.versions, 1)]
            WHERE flow.path = $1 AND flow.workspace_id = $2",
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

    let ehm = HashMap::new();
    let push_args = queued_job
        .args
        .as_ref()
        .map(|x| PushArgs::from(&x.0))
        .unwrap_or_else(|| PushArgs::from(&ehm));
    r.map(|x| interpolate_args(x, &push_args, &queued_job.workspace_id))
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
                .or(args.extra.as_ref().and_then(|x| x.get(arg_name)))
                .map(|x| x.get())
                .unwrap_or_default()
                .trim_matches('"');
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
    let is_flow = job_kind.is_flow();
    format!(
        "{}/{}/{}",
        workspace_id,
        if is_flow { "flow" } else { "script" },
        path,
    )
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
            let root = sqlx::query!(
                "SELECT
                    id As \"id!\",
                    flow_status->'restarted_from'->'flow_job_id' AS \"restarted_from: Json<Uuid>\"
                FROM v2_as_queue
                WHERE COALESCE((SELECT flow_innermost_root_job FROM v2_job WHERE id = $1), $1) = id AND workspace_id = $2",
                flow_id,
                &w_id
            )
            .fetch_optional(&db)
            .await?;
            match root {
                Some(root) => {
                    let restarted_from_id = not_found_if_none(
                        root.restarted_from,
                        "Id not found in the result's mapping of the root job and root job had no restarted from information",
                        format!("parent: {}, root: {}, id: {}", flow_id, root.id, node_id),
                    )?;

                    get_result_by_id_from_original_flow(
                        &db,
                        w_id.as_str(),
                        &restarted_from_id,
                        node_id.as_str(),
                        json_path.clone(),
                    )
                    .await
                }
                None => {
                    get_result_by_id_from_original_flow(
                        &db,
                        w_id.as_str(),
                        &flow_id,
                        node_id.as_str(),
                        json_path.clone(),
                    )
                    .await
                }
            }
        }
    }
}

pub async fn get_result_and_success_by_id_from_flow(
    db: &Pool<Postgres>,
    w_id: &str,
    flow_id: &Uuid,
    node_id: &str,
    json_path: Option<String>,
) -> error::Result<(Box<RawValue>, bool)> {
    // used exclusively for sync webhook/routes, do not handle restarted_from like get_result_by_id
    let mut job_result =
        match get_result_by_id_from_running_flow_inner(db, w_id, flow_id, node_id).await {
            Ok(res) => Some(res),
            Err(err) => match err {
                Error::NotFound(_) => None,
                _ => return Err(err),
            },
        };

    let mut completed = false;

    if job_result.is_none() {
        job_result =
            match get_result_by_id_from_original_flow_inner(db, w_id, flow_id, node_id).await {
                Ok(res) => Some(res),
                Err(err) => match err {
                    Error::NotFound(_) => None,
                    _ => return Err(err),
                },
            };
        completed = job_result.is_some();
    }

    let job_result = not_found_if_none(
        job_result,
        "Node result",
        format!("flow: {}, node: {}", flow_id, node_id),
    )?;

    let success = match &job_result {
        JobResult::SingleJob(job_id) => {
            sqlx::query_scalar!(
                "SELECT success AS \"success!\"
                FROM v2_as_completed_job WHERE id = $1 AND workspace_id = $2",
                job_id,
                w_id
            )
            .fetch_one(db)
            .await?
        }
        JobResult::ListJob(_) => {
            let query = format!(
                r#"WITH modules AS (
                    SELECT jsonb_array_elements(flow_status->'modules') AS module
                    FROM {}
                    WHERE id = $1 AND workspace_id = $2
                )
                SELECT module->>'type' = 'Success'
                FROM modules
                WHERE module->>'id' = $3"#,
                if completed {
                    "v2_as_completed_job"
                } else {
                    "v2_as_queue"
                }
            );
            sqlx::query_scalar(&query)
                .bind(flow_id)
                .bind(w_id)
                .bind(node_id)
                .fetch_optional(db)
                .await?
                .ok_or_else(|| {
                    error::Error::internal_err(format!(
                        "Could not get success from flow job status"
                    ))
                })?
        }
    };

    let result = extract_result_from_job_result(db, w_id, job_result, json_path).await?;

    Ok((result, success))
}

#[async_recursion]
pub async fn get_result_by_id_from_running_flow(
    db: &Pool<Postgres>,
    w_id: &str,
    flow_id: &Uuid,
    node_id: &str,
    json_path: Option<String>,
) -> error::Result<Box<RawValue>> {
    let result_id = get_result_by_id_from_running_flow_inner(db, w_id, flow_id, node_id).await?;

    extract_result_from_job_result(db, w_id, result_id, json_path).await
}

#[async_recursion]
pub async fn get_result_by_id_from_running_flow_inner(
    db: &Pool<Postgres>,
    w_id: &str,
    flow_id: &Uuid,
    node_id: &str,
) -> error::Result<JobResult> {
    let flow_job_result = sqlx::query!(
        "SELECT leaf_jobs->$1::text AS \"leaf_jobs: Json<Box<RawValue>>\", parent_job
        FROM v2_as_queue
        WHERE COALESCE((SELECT flow_innermost_root_job FROM v2_job WHERE id = $2), $2) = id AND workspace_id = $3",
        node_id,
        flow_id,
        w_id,
    )
    .fetch_optional(db)
    .await?;

    let flow_job_result = windmill_common::utils::not_found_if_none(
        flow_job_result,
        "Root job of parent runnnig flow",
        format!("parent: {}, id: {}", flow_id, node_id),
    )?;

    let job_result = flow_job_result
        .leaf_jobs
        .map(|x| serde_json::from_str(x.get()).ok())
        .flatten();

    if job_result.is_none() && flow_job_result.parent_job.is_some() {
        let parent_job = flow_job_result.parent_job.unwrap();
        let root_job = sqlx::query_scalar!(
            "SELECT flow_innermost_root_job FROM v2_job WHERE id = $1",
            parent_job
        )
        .fetch_optional(db)
        .await?
        .flatten()
        .unwrap_or(parent_job);
        return get_result_by_id_from_running_flow_inner(db, w_id, &root_job, node_id).await;
    }

    let result_id = windmill_common::utils::not_found_if_none(
        job_result,
        "Flow result by id",
        format!("parent: {}, id: {}", flow_id, node_id),
    )?;

    Ok(result_id)
}

async fn get_completed_flow_node_result_rec(
    db: &Pool<Postgres>,
    w_id: &str,
    subflows: impl std::iter::Iterator<Item = (Uuid, FlowStatus)>,
    node_id: &str,
) -> error::Result<Option<JobResult>> {
    for (id, flow_status) in subflows {
        if let Some(node_status) = flow_status
            .modules
            .iter()
            .find(|module| module.id() == node_id)
        {
            return match (node_status.job(), node_status.flow_jobs()) {
                (Some(leaf_job_uuid), None) => Ok(Some(JobResult::SingleJob(leaf_job_uuid))),
                (Some(_), Some(jobs)) => Ok(Some(JobResult::ListJob(jobs))),
                _ => Err(error::Error::NotFound(format!(
                    "Flow result by id not found going top-down in subflows (currently: {}), (id: {})",
                    id,
                    node_id,
                ))),
            };
        } else {
            let subflows = sqlx::query!(
                "SELECT id AS \"id!\", flow_status AS \"flow_status!: Json<FlowStatus>\"
                FROM v2_as_completed_job
                WHERE parent_job = $1 AND workspace_id = $2 AND flow_status IS NOT NULL",
                id,
                w_id
            )
            .map(|record| (record.id, record.flow_status.0))
            .fetch_all(db)
            .await?
            .into_iter();
            match Box::pin(get_completed_flow_node_result_rec(
                db, w_id, subflows, node_id,
            ))
            .await?
            {
                Some(res) => return Ok(Some(res)),
                None => continue,
            };
        }
    }

    Ok(None)
}

async fn get_result_by_id_from_original_flow_inner(
    db: &Pool<Postgres>,
    w_id: &str,
    completed_flow_id: &Uuid,
    node_id: &str,
) -> error::Result<JobResult> {
    let flow_job = sqlx::query!(
        "SELECT id, flow_status AS \"flow_status!: Json<FlowStatus>\"
        FROM v2_job_completed WHERE id = $1 AND workspace_id = $2",
        completed_flow_id,
        w_id
    )
    .map(|record| (record.id, record.flow_status.0))
    .fetch_optional(db)
    .await?;

    let flow_job = not_found_if_none(
        flow_job,
        "Root completed job",
        format!("root: {}, id: {}", completed_flow_id, node_id),
    )?;

    match get_completed_flow_node_result_rec(db, w_id, [flow_job].into_iter(), node_id).await? {
        Some(res) => Ok(res),
        None => Err(Error::NotFound(format!(
            "Flow result by id not found going top-down from {}, (id: {})",
            completed_flow_id, node_id
        ))),
    }
}

async fn get_result_by_id_from_original_flow(
    db: &Pool<Postgres>,
    w_id: &str,
    completed_flow_id: &Uuid,
    node_id: &str,
    json_path: Option<String>,
) -> error::Result<Box<RawValue>> {
    let job_result =
        get_result_by_id_from_original_flow_inner(db, w_id, completed_flow_id, node_id).await?;
    extract_result_from_job_result(db, w_id, job_result, json_path).await
}

async fn extract_result_from_job_result(
    db: &Pool<Postgres>,
    w_id: &str,
    job_result: JobResult,
    json_path: Option<String>,
) -> error::Result<Box<RawValue>> {
    match job_result {
        JobResult::ListJob(job_ids) => match json_path {
            Some(json_path) => {
                let mut parts = json_path.split(".");

                let Some(idx) = parts.next().map(|x| x.parse::<usize>().ok()).flatten() else {
                    return Ok(to_raw_value(&serde_json::Value::Null));
                };
                let Some(job_id) = job_ids.get(idx).cloned() else {
                    return Ok(to_raw_value(&serde_json::Value::Null));
                };
                Ok(sqlx::query_scalar!(
                    "SELECT result #> $3 AS \"result: Json<Box<RawValue>>\"
                    FROM v2_job_completed WHERE id = $1 AND workspace_id = $2",
                    job_id,
                    w_id,
                    parts.collect::<Vec<_>>() as Vec<&str>
                )
                .fetch_optional(db)
                .await?
                .flatten()
                .map(|x| x.0)
                .unwrap_or_else(|| to_raw_value(&serde_json::Value::Null)))
            }
            None => {
                let rows = sqlx::query!(
                    "SELECT id, result  AS \"result: Json<Box<RawValue>>\"
                    FROM v2_job_completed WHERE id = ANY($1) AND workspace_id = $2",
                    job_ids.as_slice(),
                    w_id
                )
                .fetch_all(db)
                .await?
                .into_iter()
                .filter_map(|x| x.result.map(|y| (x.id, y)))
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
        },
        JobResult::SingleJob(x) => Ok(sqlx::query!(
            "SELECT result #> $3 AS \"result: Json<Box<RawValue>>\"
            FROM v2_job_completed WHERE id = $1 AND workspace_id = $2",
            x,
            w_id,
            json_path
                .as_ref()
                .map(|x| x.split(".").collect::<Vec<_>>())
                .unwrap_or_default() as Vec<&str>,
        )
        .fetch_optional(db)
        .await?
        .map(|r| r.result.map(|x| x.0))
        .flatten()
        .unwrap_or_else(|| to_raw_value(&serde_json::Value::Null))),
    }
}

pub async fn delete_job<'c>(
    mut tx: Transaction<'c, Postgres>,
    job_id: &Uuid,
) -> windmill_common::error::Result<Transaction<'c, Postgres>> {
    #[cfg(feature = "prometheus")]
    if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        QUEUE_DELETE_COUNT.inc();
    }

    let job_removed =
        sqlx::query_scalar!("DELETE FROM v2_job_queue WHERE id = $1 RETURNING 1", job_id,)
            .fetch_optional(&mut *tx)
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
        "SELECT EXISTS(SELECT 1 FROM v2_job_completed WHERE id = $1 AND workspace_id = $2)",
        id,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false))
}

async fn get_queued_job_tx<'c>(
    id: Uuid,
    w_id: &str,
    tx: &mut Transaction<'c, Postgres>,
) -> error::Result<Option<QueuedJob>> {
    sqlx::query_as::<_, QueuedJob>(
        "SELECT *
            FROM v2_as_queue WHERE id = $1 AND workspace_id = $2",
    )
    .bind(id)
    .bind(w_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(Into::into)
}

pub async fn get_queued_job(id: &Uuid, w_id: &str, db: &DB) -> error::Result<Option<QueuedJob>> {
    sqlx::query_as::<_, QueuedJob>(
        "SELECT *
            FROM v2_as_queue WHERE id = $1 AND workspace_id = $2",
    )
    .bind(id)
    .bind(w_id)
    .fetch_optional(db)
    .await
    .map_err(Into::into)
}

pub enum PushIsolationLevel<'c> {
    IsolatedRoot(DB),
    Isolated(UserDB, Authed),
    Transaction(Transaction<'c, Postgres>),
}

impl<'c> PushIsolationLevel<'c> {
    async fn into_tx(self) -> error::Result<Transaction<'c, Postgres>> {
        match self {
            PushIsolationLevel::Isolated(db, authed) => Ok((db.begin(&authed).await?).into()),
            PushIsolationLevel::IsolatedRoot(db) => Ok(db.begin().await?),
            PushIsolationLevel::Transaction(tx) => Ok(tx),
        }
    }
}

#[macro_export]
macro_rules! fetch_scalar_isolated {
    ( $query:expr, $tx:expr) => {
        match $tx {
            PushIsolationLevel::IsolatedRoot(db) => {
                let r = $query.fetch_optional(&db).await;
                $tx = PushIsolationLevel::IsolatedRoot(db);
                r
            }
            PushIsolationLevel::Isolated(db, user) => {
                let mut ntx = db.clone().begin(&user).await?;
                let r = $query.fetch_optional(&mut *ntx).await;
                $tx = PushIsolationLevel::Isolated(db, user);
                r
            }
            PushIsolationLevel::Transaction(mut tx) => {
                let r = $query.fetch_optional(&mut *tx).await;
                $tx = PushIsolationLevel::Transaction(tx);
                r
            }
        }
    };
}

use sqlx::types::JsonRawValue;

#[derive(Debug, Default)]
pub struct PushArgsOwned {
    pub extra: Option<HashMap<String, Box<RawValue>>>,
    pub args: HashMap<String, Box<RawValue>>,
}

#[derive(Debug)]
pub struct PushArgs<'c> {
    pub extra: Option<HashMap<String, Box<RawValue>>>,
    pub args: &'c HashMap<String, Box<RawValue>>,
}

impl<'c> From<&'c HashMap<String, Box<RawValue>>> for PushArgs<'c> {
    fn from(args: &'c HashMap<String, Box<RawValue>>) -> Self {
        PushArgs { extra: None, args }
    }
}

impl<'c> Serialize for PushArgs<'c> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(
            self.args.len() + self.extra.as_ref().map(|x| x.len()).unwrap_or_default(),
        ))?;
        let mut in_extra = vec![];
        if let Some(extra) = &self.extra {
            for (k, v) in extra {
                map.serialize_entry(k, v)?;
                in_extra.push(k);
            }
        }
        for (k, v) in self.args {
            if !in_extra.contains(&k) {
                map.serialize_entry(k, v)?;
            }
        }
        map.end()
    }
}

impl PushArgsOwned {
    pub fn empty() -> Self {
        PushArgsOwned { extra: None, args: HashMap::new() }
    }
}

pub fn empty_result() -> Box<RawValue> {
    return JsonRawValue::from_string("{}".to_string()).unwrap();
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
pub async fn push<'c, 'd>(
    _db: &Pool<Postgres>,
    mut tx: PushIsolationLevel<'c>,
    workspace_id: &str,
    job_payload: JobPayload,
    mut args: PushArgs<'d>,
    user: &str,
    mut email: &str,
    mut permissioned_as: String,
    scheduled_for_o: Option<chrono::DateTime<chrono::Utc>>,
    schedule_path: Option<String>,
    parent_job: Option<Uuid>,
    root_job: Option<Uuid>,
    job_id: Option<Uuid>,
    _is_flow_step: bool,
    mut same_worker: bool,
    pre_run_error: Option<&windmill_common::error::Error>,
    visible_to_owner: bool,
    mut tag: Option<String>,
    custom_timeout: Option<i32>,
    flow_step_id: Option<String>,
    _priority_override: Option<i16>,
    authed: Option<&Authed>,
) -> Result<(Uuid, Transaction<'c, Postgres>), Error> {
    #[cfg(feature = "cloud")]
    if *CLOUD_HOSTED {
        let premium_workspace =
            sqlx::query_scalar!("SELECT premium FROM workspace WHERE id = $1", workspace_id)
                .fetch_one(_db)
                .await
                .map_err(|e| {
                    Error::internal_err(format!(
                        "fetching if {workspace_id} is premium and overquota: {e:#}"
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
                .map_err(|e| Error::internal_err(format!("updating usage: {e:#}")))?;

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
                .map_err(|e| Error::internal_err(format!("updating usage: {e:#}")))?)
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
                    && permissioned_as != SUPERADMIN_SYNC_EMAIL
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

                    let in_queue = sqlx::query_scalar!(
                        "SELECT COUNT(id) FROM v2_as_queue WHERE email = $1",
                        email
                    )
                    .fetch_one(_db)
                    .await?
                    .unwrap_or(0);

                    if in_queue > MAX_FREE_EXECS.into() {
                        return Err(error::Error::QuotaExceeded(format!(
                            "User {email} has exceeded the jobs in queue limit of {MAX_FREE_EXECS} that applies outside of premium workspaces."
                        )));
                    }

                    let concurrent_runs = sqlx::query_scalar!(
                        "SELECT COUNT(id) FROM v2_as_queue WHERE running = true AND email = $1",
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
                        "SELECT COUNT(id) FROM v2_job_queue WHERE workspace_id = $1",
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
                        "SELECT COUNT(id) FROM v2_job_queue WHERE running = true AND workspace_id = $1",
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

    let mut preprocessed = None;
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
            apply_preprocessor,
        } => {
            let extra = args.extra.get_or_insert_with(HashMap::new);
            if apply_preprocessor {
                preprocessed = Some(false);
                extra.entry("wm_trigger".to_string()).or_insert_with(|| {
                    to_raw_value(&serde_json::json!({
                        "kind": "webhook",
                    }))
                });
            } else {
                extra.remove("wm_trigger");
            }
            (
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
            )
        }
        JobPayload::FlowScript {
            id, // flow_node(id).
            language,
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            dedicated_worker,
            path,
        } => (
            Some(id.0),
            Some(path),
            None,
            JobKind::FlowScript,
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
        JobPayload::FlowNode { id, path } => {
            let data = cache::flow::fetch_flow(_db, id).await?;
            let value = data.value();
            let status = Some(FlowStatus::new(value));
            // Keep inserting `value` if not all workers are updated.
            // Starting at `v1.440`, the value is fetched on pull from the flow node id.
            let value_o = if !*MIN_VERSION_IS_AT_LEAST_1_440.read().await {
                Some(value.clone())
            } else {
                // `raw_flow` is fetched on pull.
                None
            };
            (
                Some(id.0),
                Some(path),
                None,
                JobKind::FlowNode,
                value_o,
                status,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
        }
        JobPayload::AppScript {
            id, // app_script(id).
            path,
            language,
            cache_ttl,
        } => (
            Some(id.0),
            path,
            None,
            JobKind::AppScript,
            None,
            None,
            Some(language),
            None,
            None,
            None,
            cache_ttl,
            None,
            None,
        ),
        JobPayload::ScriptHub { path } => {
            if path == "hub/7771/slack" || path == "hub/7836/slack" {
                permissioned_as = SUPERADMIN_NOTIFICATION_EMAIL.to_string();
                email = SUPERADMIN_NOTIFICATION_EMAIL;
            }

            let hub_script =
                get_full_hub_script_by_path(StripPath(path.clone()), &HTTP_CLIENT, Some(_db))
                    .await?;

            (
                None,
                Some(path),
                None,
                // Some((script.content, script.lockfile)),
                JobKind::Script_Hub,
                None,
                None,
                Some(hub_script.language),
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
        JobPayload::RawFlowDependencies { path, flow_value } => (
            None,
            Some(path),
            None,
            JobKind::FlowDependencies,
            Some(flow_value),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ),
        JobPayload::FlowDependencies { path, dedicated_worker, version } => {
            // Keep inserting `value` if not all workers are updated.
            // Starting at `v1.440`, the value is fetched on pull from the version id.
            let value_o = if !*MIN_VERSION_IS_AT_LEAST_1_440.read().await {
                let mut ntx = tx.into_tx().await?;
                // The version has been inserted only within the transaction.
                let data = cache::flow::fetch_version(&mut *ntx, version).await?;
                tx = PushIsolationLevel::Transaction(ntx);
                Some(data.value().clone())
            } else {
                // `raw_flow` is fetched on pull.
                None
            };
            (
                Some(version),
                Some(path),
                None,
                JobKind::FlowDependencies,
                value_o,
                None,
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
                    let (_, _, _, step_n, truncated_modules, user_states, cleanup_module) =
                        restarted_flows_resolution(
                            _db,
                            workspace_id,
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
                        preprocessor_module: None,
                    }
                }
                _ => {
                    value.preprocessor_module = None;
                    FlowStatus::new(&value)
                } // this is a new flow being pushed, flow_status is set to flow_value
            };
            let concurrency_key = value.concurrency_key.clone();
            let concurrent_limit = value.concurrent_limit;
            let concurrency_time_window_s = value.concurrency_time_window_s;
            let cache_ttl = value.cache_ttl.map(|x| x as i32);
            let priority = value.priority;
            (
                None,
                path,
                None,
                JobKind::FlowPreview,
                Some(value),
                Some(flow_status),
                None,
                concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                cache_ttl,
                None,
                priority,
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
                    value: to_raw_value(&FlowModuleValue::Script {
                        input_transforms,
                        path: path.clone(),
                        hash: Some(hash),
                        tag_override,
                        is_trigger: None,
                    }),
                    stop_after_if: None,
                    stop_after_all_iters_if: None,
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
                    skip_if: None,
                }],
                same_worker: false,
                failure_module: None,
                concurrency_time_window_s,
                concurrent_limit,
                skip_expr: None,
                cache_ttl: cache_ttl.map(|val| val as u32),
                early_return: None,
                concurrency_key: custom_concurrency_key.clone(),
                priority,
                preprocessor_module: None,
            };
            // this is a new flow being pushed, flow_status is set to flow_value:
            let flow_status: FlowStatus = FlowStatus::new(&flow_value);
            (
                None,
                Some(path),
                None,
                JobKind::Flow,
                Some(flow_value),
                Some(flow_status),
                None,
                custom_concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                cache_ttl,
                None,
                priority,
            )
        }
        JobPayload::Flow { path, dedicated_worker, apply_preprocessor } => {
            let mut ntx = tx.into_tx().await?;
            // Fetch the latest version of the flow.
            let version = sqlx::query_scalar!(
                "SELECT flow.versions[array_upper(flow.versions, 1)] AS \"version!: i64\"
                FROM flow WHERE path = $1 AND workspace_id = $2",
                &path,
                &workspace_id
            )
            .fetch_optional(&mut *ntx)
            .await?
            .ok_or_else(|| Error::internal_err(format!("not found flow at path {:?}", path)))?;

            // Do not use the lite version unless all workers are updated.
            let data = if *DISABLE_FLOW_SCRIPT
                || (!*MIN_VERSION_IS_AT_LEAST_1_432.read().await && !*CLOUD_HOSTED)
            {
                cache::flow::fetch_version(&mut *ntx, version).await
            } else {
                // Fallback to the original version if the lite version is not found.
                // This also prevent a race condition where the flow is run just after deploy and
                // the lite version is still being created.
                match cache::flow::fetch_version_lite(&mut *ntx, version).await {
                    Ok(data) => Ok(data),
                    Err(_) => cache::flow::fetch_version(&mut *ntx, version).await,
                }
            }?;
            tx = PushIsolationLevel::Transaction(ntx);

            let value = data.value().clone();
            let priority = value.priority;
            let cache_ttl = value.cache_ttl.map(|x| x as i32);
            let custom_concurrency_key = value.concurrency_key.clone();
            let concurrency_time_window_s = value.concurrency_time_window_s;
            let concurrent_limit = value.concurrent_limit;

            // this is a new flow being pushed, status is set to `value`.
            let mut status = FlowStatus::new(&value);
            let extra = args.extra.get_or_insert_with(HashMap::new);
            if !apply_preprocessor {
                status.preprocessor_module = None;
                extra.remove("wm_trigger");
            } else {
                preprocessed = Some(false);
                extra.entry("wm_trigger".to_string()).or_insert_with(|| {
                    to_raw_value(&serde_json::json!({
                        "kind": "webhook",
                    }))
                });
            }
            // Keep inserting `value` if not all workers are updated.
            // Starting at `v1.440`, the value is fetched on pull from the version id.
            let value_o = if !*MIN_VERSION_IS_AT_LEAST_1_440.read().await {
                let mut value = value;
                add_virtual_items_if_necessary(&mut value.modules);
                if same_worker {
                    value.same_worker = true;
                }
                if !apply_preprocessor {
                    value.preprocessor_module = None;
                }
                Some(value)
            } else {
                // `raw_flow` is fetched on pull, the mutations from the other branch are replaced
                // by additional checks when handling the flow.
                None
            };
            (
                Some(version), // Starting from `v1.436`, the version id is used to fetch the value on pull.
                Some(path),
                None,
                JobKind::Flow,
                value_o,
                Some(status),
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
                version,
                flow_path,
                flow_data,
                step_n,
                truncated_modules,
                user_states,
                cleanup_module,
            ) = restarted_flows_resolution(
                _db,
                workspace_id,
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
                preprocessor_module: None,
            };
            let value = flow_data.value();
            let priority = value.priority;
            let concurrency_key = value.concurrency_key.clone();
            let concurrent_limit = value.concurrent_limit;
            let concurrency_time_window_s = value.concurrency_time_window_s;
            let cache_ttl = value.cache_ttl.map(|x| x as i32);
            // Keep inserting `value` if not all workers are updated.
            // Starting at `v1.440`, the value is fetched on pull from the version id.
            let value_o = if version.is_none() || !*MIN_VERSION_IS_AT_LEAST_1_440.read().await {
                Some(value.clone())
            } else {
                // `raw_flow` is fetched on pull.
                None
            };
            (
                version,
                flow_path,
                None,
                JobKind::Flow,
                value_o,
                Some(restarted_flow_status),
                None,
                concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                cache_ttl,
                None,
                priority,
            )
        }
        JobPayload::DeploymentCallback { path } => (
            None,
            Some(path.clone()),
            None,
            JobKind::DeploymentCallback,
            None,
            None,
            None,
            Some(format!("{workspace_id}:git_sync")),
            Some(1),
            Some(0),
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
    // prioritize flow steps to drain the queue faster
    let final_priority = if flow_step_id.is_some() && final_priority.is_none() {
        Some(0)
    } else {
        final_priority
    };

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

    let per_workspace_workspaces = DEFAULT_TAGS_WORKSPACES.read().await;
    let per_workspace = DEFAULT_TAGS_PER_WORKSPACE.load(std::sync::atomic::Ordering::Relaxed)
        && (per_workspace_workspaces.is_none()
            || per_workspace_workspaces
                .as_ref()
                .unwrap()
                .contains(&workspace_id.to_string()));

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
    } else {
        if tag == Some("".to_string()) {
            tag = None;
        }

        let interpolated_tag = tag.map(|x| interpolate_args(x, &args, workspace_id));

        let default = || {
            let ntag = if job_kind.is_flow() || job_kind == JobKind::Identity {
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
                    let tag_lang = if x == &ScriptLang::Bunnative {
                        ScriptLang::Nativets.as_str()
                    } else {
                        x.as_str()
                    };
                    if per_workspace {
                        format!("{}-{}", tag_lang, workspace_id)
                    } else {
                        tag_lang.to_string()
                    }
                })
                .unwrap_or_else(default)
        })
    };

    let mut tx = tx.into_tx().await?;

    let job_id: Uuid = if let Some(job_id) = job_id {
        let conflicting_id = sqlx::query_scalar!("SELECT 1 FROM v2_job WHERE id = $1", job_id)
            .fetch_optional(&mut *tx)
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
            "WITH inserted_concurrency_counter AS (
                INSERT INTO concurrency_counter (concurrency_id, job_uuids) 
                VALUES ($1, '{}'::jsonb)
                ON CONFLICT DO NOTHING
            )
            INSERT INTO concurrency_key(key, job_id) VALUES ($1, $2)",
            concurrency_key,
            job_id,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::internal_err(format!("Could not insert concurrency_key={concurrency_key} for job_id={job_id} script_path={script_path:?} workspace_id={workspace_id}: {e:#}")))?;
    }

    let stringified_args = if *JOB_ARGS_AUDIT_LOGS {
        Some(serde_json::to_string(&args).map_err(|e| {
            Error::internal_err(format!(
                "Could not serialize args for audit log of job {job_id}: {e:#}"
            ))
        })?)
    } else {
        None
    };

    let raw_flow = raw_flow.map(Json);
    let preprocessed = preprocessed.or_else(|| match flow_step_id.as_deref() {
        Some("preprocessor") => Some(false),
        _ => None,
    });

    sqlx::query!(
        "INSERT INTO v2_job (id, workspace_id, raw_code, raw_lock, raw_flow, tag, parent_job,
            created_by, permissioned_as, runnable_id, runnable_path, args, kind, trigger,
            script_lang, same_worker, pre_run_error, permissioned_as_email, visible_to_owner,
            flow_innermost_root_job, concurrent_limit, concurrency_time_window_s, timeout, flow_step_id,
            cache_ttl, priority, trigger_kind, script_entrypoint_override, preprocessed)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18,
            $19, $20, $21, $22, $23, $24, $25, $26,
            CASE WHEN $14::VARCHAR IS NOT NULL THEN 'schedule'::job_trigger_kind END,
            ($12::JSONB)->>'_ENTRYPOINT_OVERRIDE', $27)",
        job_id,
        workspace_id,
        raw_code,
        raw_lock,
        raw_flow as Option<Json<FlowValue>>,
        tag,
        parent_job,
        user,
        permissioned_as,
        script_hash,
        script_path.clone(),
        Json(args) as Json<PushArgs>,
        job_kind.clone() as JobKind,
        schedule_path,
        language as Option<ScriptLang>,
        same_worker,
        pre_run_error.map(|e| e.to_string()),
        email,
        visible_to_owner,
        root_job,
        concurrent_limit,
        if concurrent_limit.is_some() {
            concurrency_time_window_s
        } else {
            None
        },
        custom_timeout,
        flow_step_id,
        cache_ttl,
        final_priority,
        preprocessed,
    )
    .execute(&mut *tx)
    .warn_after_seconds(1)
    .await?;

    tracing::debug!("Pushing job {job_id} with tag {tag}, schedule_path {schedule_path:?}, script_path: {script_path:?}, email {email}, workspace_id {workspace_id}");
    let uuid = sqlx::query_scalar!(
        "INSERT INTO v2_job_queue
            (workspace_id, id, running, scheduled_for, started_at, tag, priority)
            VALUES ($1, $2, $3, COALESCE($4, now()), CASE WHEN $3 THEN now() END, $5, $6) \
         RETURNING id AS \"id!\"",
        workspace_id,
        job_id,
        is_running,
        scheduled_for_o,
        tag,
        final_priority,
    )
    .fetch_one(&mut *tx)
    .warn_after_seconds(1)
    .await
    .map_err(|e| Error::internal_err(format!("Could not insert into queue {job_id} with tag {tag}, schedule_path {schedule_path:?}, script_path: {script_path:?}, email {email}, workspace_id {workspace_id}: {e:#}")))?;

    sqlx::query!(
        "INSERT INTO v2_job_runtime (id, ping) VALUES ($1, null)",
        job_id
    )
    .execute(&mut *tx)
    .await?;
    if let Some(flow_status) = flow_status {
        sqlx::query!(
            "INSERT INTO v2_job_status (id, flow_status) VALUES ($1, $2)",
            job_id,
            Json(flow_status) as Json<FlowStatus>,
        )
        .execute(&mut *tx)
        .await?;
    }

    tracing::debug!("Pushed {job_id}");
    // TODO: technically the job isn't queued yet, as the transaction can be rolled back. Should be solved when moving these metrics to the queue abstraction.
    #[cfg(feature = "prometheus")]
    if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        QUEUE_PUSH_COUNT.inc();
    }

    if JOB_TOKEN.is_none() {
        let job_authed = match authed {
            Some(authed)
                if authed.email == email
                    && authed.username == permissioned_as_to_username(&permissioned_as) =>
            {
                authed.clone()
            }
            _ => {
                if authed.is_some() {
                    tracing::warn!("Authed passed to push is not the same as permissioned_as, refetching direclty permissions for job {job_id}...")
                }
                fetch_authed_from_permissioned_as(
                    permissioned_as.clone(),
                    email.to_string(),
                    workspace_id,
                    _db,
                )
                .await
                .map_err(|e| {
                    Error::internal_err(format!(
                        "Could not get permissions directly for job {job_id}: {e:#}"
                    ))
                })?
            }
        };

        let folders = job_authed
            .folders
            .iter()
            .filter_map(|x| serde_json::to_value(x).ok())
            .collect::<Vec<_>>();

        if let Err(err) = sqlx::query!("INSERT INTO job_perms (job_id, email, username, is_admin, is_operator, folders, groups, workspace_id) 
            values ($1, $2, $3, $4, $5, $6, $7, $8) 
            ON CONFLICT (job_id) DO UPDATE SET email = $2, username = $3, is_admin = $4, is_operator = $5, folders = $6, groups = $7, workspace_id = $8",
            job_id,
            job_authed.email,
            job_authed.username,
            job_authed.is_admin,
            job_authed.is_operator,
            folders.as_slice(),
            job_authed.groups.as_slice(),
            workspace_id,
        ).execute(&mut *tx).await {
            tracing::error!("Could not insert job_perms for job {job_id}: {err:#}");
        }
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
            JobKind::FlowScript => "jobs.run.flow_script",
            JobKind::FlowNode => "jobs.run.flow_node",
            JobKind::AppScript => "jobs.run.app_script",
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

        if let Some(ref stringified_args) = stringified_args {
            hm.insert("args", stringified_args);
        }

        audit_log(
            &mut *tx,
            &audit_author,
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
    branch_or_iteration_n: Option<usize>,
) -> Result<
    (
        Option<i64>,
        Option<String>,
        Arc<FlowData>,
        i32,
        Vec<FlowStatusModule>,
        HashMap<String, serde_json::Value>,
        FlowCleanupModule,
    ),
    Error,
> {
    let row = sqlx::query!(
        "SELECT
            script_path, script_hash AS \"script_hash: ScriptHash\",
            job_kind AS \"job_kind!: JobKind\",
            flow_status AS \"flow_status: Json<Box<RawValue>>\",
            raw_flow AS \"raw_flow: Json<Box<RawValue>>\"
        FROM v2_as_completed_job WHERE id = $1 and workspace_id = $2",
        completed_flow_id,
        workspace_id,
    )
    .fetch_one(db) // TODO: should we try to use the passed-in `tx` here?
    .await
    .map_err(|err| {
        Error::internal_err(format!(
            "completed job not found for UUID {} in workspace {}: {}",
            completed_flow_id, workspace_id, err
        ))
    })?;

    let flow_data = cache::job::fetch_flow(db, row.job_kind, row.script_hash)
        .or_else(|_| cache::job::fetch_preview_flow(db, &completed_flow_id, row.raw_flow))
        .await?;
    let flow_value = flow_data.value();
    let flow_status = row
        .flow_status
        .as_ref()
        .and_then(|v| serde_json::from_str::<FlowStatus>(v.get()).ok())
        .ok_or(Error::internal_err(format!(
            "Unable to parse flow status for job {} in workspace {}",
            completed_flow_id, workspace_id,
        )))?;

    let mut step_n = 0;
    let mut dependent_module = false;
    let mut truncated_modules: Vec<FlowStatusModule> = vec![];
    for module in flow_status.modules {
        let Some(module_definition) = flow_value
            .modules
            .iter()
            .find(|flow_value_module| flow_value_module.id == module.id())
        else {
            // skip module as it doesn't appear in the flow_value anymore
            continue;
        };
        if module.id() == restart_step_id {
            // if the module ID is the one we want to restart the flow at, or if it's past it in the flow,
            // set the module as WaitingForPriorSteps as it needs to be re-run
            if branch_or_iteration_n.is_none() || branch_or_iteration_n.unwrap() == 0 {
                // The module as WaitingForPriorSteps as the entire module (i.e. all the branches) need to be re-run
                truncated_modules.push(FlowStatusModule::WaitingForPriorSteps { id: module.id() });
            } else {
                // expect a module to be either a branchall (resp. loop), and resume the flow from this branch (resp. iteration)
                let branch_or_iteration_n = branch_or_iteration_n.unwrap();

                match module_definition.get_value() {
                    Ok(FlowModuleValue::BranchAll { branches, parallel, .. }) => {
                        if parallel {
                            return Err(Error::internal_err(format!(
                                "Module {} is a parallel branchall. It can only be restarted at a given branch if it's sequential",
                                restart_step_id,
                            )));
                        }
                        let total_branch_number = module.flow_jobs().map(|v| v.len()).unwrap_or(0);
                        if total_branch_number <= branch_or_iteration_n {
                            return Err(Error::internal_err(format!(
                                "Branch-all module {} has only {} branches. It can't be restarted on branch {}",
                                restart_step_id,
                                total_branch_number,
                                branch_or_iteration_n,
                            )));
                        }
                        let mut new_flow_jobs = module.flow_jobs().unwrap_or_default();
                        new_flow_jobs.truncate(branch_or_iteration_n);
                        let mut new_flow_jobs_success = module.flow_jobs_success();
                        if let Some(new_flow_jobs_success) = new_flow_jobs_success.as_mut() {
                            new_flow_jobs_success.truncate(branch_or_iteration_n);
                        }
                        truncated_modules.push(FlowStatusModule::InProgress {
                            id: module.id(),
                            job: new_flow_jobs[new_flow_jobs.len() - 1], // set to last finished job from completed flow
                            iterator: None,
                            flow_jobs: Some(new_flow_jobs),
                            flow_jobs_success: new_flow_jobs_success,
                            branch_chosen: None,
                            branchall: Some(BranchAllStatus {
                                branch: branch_or_iteration_n - 1, // Doing minus one here as this variable reflects the latest finished job in the iteration
                                len: branches.len(),
                            }),
                            parallel,
                            while_loop: false,
                            progress: None,
                        });
                    }
                    Ok(FlowModuleValue::ForloopFlow { parallel, .. }) => {
                        if parallel {
                            return Err(Error::internal_err(format!(
                                "Module {} is not parallel loop. It can only be restarted at a given iteration if it's sequential",
                                restart_step_id,
                            )));
                        }
                        let total_iterations = module.flow_jobs().map(|v| v.len()).unwrap_or(0);
                        if total_iterations <= branch_or_iteration_n {
                            return Err(Error::internal_err(format!(
                                "For-loop module {} doesn't cannot be restarted on iteration number {} as it has only {} iterations",
                                restart_step_id,
                                branch_or_iteration_n,
                                total_iterations,
                            )));
                        }
                        let mut new_flow_jobs = module.flow_jobs().unwrap_or_default();
                        new_flow_jobs.truncate(branch_or_iteration_n);
                        let mut new_flow_jobs_success = module.flow_jobs_success();
                        if let Some(new_flow_jobs_success) = new_flow_jobs_success.as_mut() {
                            new_flow_jobs_success.truncate(branch_or_iteration_n);
                        }
                        truncated_modules.push(FlowStatusModule::InProgress {
                            id: module.id(),
                            job: new_flow_jobs[new_flow_jobs.len() - 1], // set to last finished job from completed flow
                            iterator: Some(FlowIterator {
                                index: branch_or_iteration_n - 1, // same deal as above, this refers to the last finished job
                                itered: vec![], // Setting itered to empty array here, such that input transforms will be re-computed by worker_flows
                            }),
                            flow_jobs: Some(new_flow_jobs),
                            flow_jobs_success: new_flow_jobs_success,
                            branch_chosen: None,
                            branchall: None,
                            parallel,
                            while_loop: false,
                            progress: None,
                        });
                    }
                    _ => {
                        return Err(Error::internal_err(format!(
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
                FlowStatusModule::Success { .. } => Ok(truncated_modules.push(module)),
                _ => Err(Error::internal_err(format!(
                    "Flow cannot be restarted from a non successful module",
                ))),
            }?;
        }
    }

    if !dependent_module {
        // step not found in flow.
        return Err(Error::internal_err(format!(
            "Flow cannot be restarted from step {} as it could not be found.",
            restart_step_id
        )));
    }

    Ok((
        row.script_hash.map(|x| x.0),
        row.script_path,
        flow_data,
        step_n,
        truncated_modules,
        flow_status.user_states,
        flow_status.cleanup_module,
    ))
}
