/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::future::Future;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc, vec};

use anyhow::Context;
use async_recursion::async_recursion;
use chrono::{DateTime, Utc};
use futures::future::TryFutureExt;
use itertools::Itertools;
#[cfg(feature = "prometheus")]
use prometheus::IntCounter;
use quick_cache::sync::Cache;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde::{ser::SerializeMap, Serialize};
use serde_json::{json, value::RawValue};
use sqlx::{types::Json, Acquire, Pool, Postgres, Transaction};
use sqlx::{Encode, PgExecutor};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio::{sync::RwLock, time::sleep};
use ulid::Ulid;
use uuid::Uuid;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
#[cfg(feature = "benchmark")]
use windmill_common::add_time;
use windmill_common::audit::AuditAuthor;
use windmill_common::auth::JobPerms;
#[cfg(feature = "benchmark")]
use windmill_common::bench::BenchmarkIter;
use windmill_common::jobs::{JobTriggerKind, EMAIL_ERROR_HANDLER_USER_EMAIL};
use windmill_common::min_version::{
    MIN_VERSION_SUPPORTS_DEBOUNCING, MIN_VERSION_SUPPORTS_DEBOUNCING_V2,
};
use windmill_common::runnable_settings::{
    ConcurrencySettings, ConcurrencySettingsWithCustom, DebouncingSettings, RunnableSettings,
    RunnableSettingsTrait,
};
use windmill_common::triggers::TriggerMetadata;
use windmill_common::utils::{calculate_hash, configure_client, now_from_db};
use windmill_common::worker::{Connection, SCRIPT_TOKEN_EXPIRY};

use windmill_common::{
    auth::permissioned_as_to_username,
    cache::{self, FlowData},
    db::{Authed, UserDB},
    error::{self, Error},
    flow_status::{
        BranchAllStatus, FlowCleanupModule, FlowStatus, FlowStatusModule, FlowStatusModuleWParent,
        Iterator as FlowIterator, JobResult, RestartedFrom, RetryStatus, MAX_RETRY_ATTEMPTS,
        MAX_RETRY_INTERVAL,
    },
    flows::{
        add_virtual_items_if_necessary, FlowModule, FlowModuleValue, FlowValue, InputTransform,
        StopAfterIf,
    },
    jobs::{get_payload_tag_from_prefixed_path, JobKind, JobPayload, QueuedJob, RawCode},
    min_version::{MIN_VERSION_IS_AT_LEAST_1_432, MIN_VERSION_IS_AT_LEAST_1_440},
    schedule::Schedule,
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang},
    users::{SUPERADMIN_NOTIFICATION_EMAIL, SUPERADMIN_SECRET_EMAIL},
    utils::{not_found_if_none, report_critical_error, StripPath, WarnAfterExt},
    worker::{
        to_raw_value, CLOUD_HOSTED, DISABLE_FLOW_SCRIPT, NO_LOGS, WORKER_PULL_QUERIES,
        WORKER_SUSPENDED_PULL_QUERY,
    },
    DB, METRICS_ENABLED,
};

use backon::ConstantBuilder;
use backon::{BackoffBuilder, Retryable};

use crate::flow_status::{update_flow_status_in_progress, update_workflow_as_code_status};
use crate::schedule::{get_schedule_opt, push_scheduled_job};
use crate::tags::per_workspace_tag;
#[cfg(feature = "cloud")]
use windmill_common::users::SUPERADMIN_SYNC_EMAIL;

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

    pub static ref QUEUE_PULL_COUNT: prometheus::IntCounter = prometheus::register_int_counter!(
        "queue_pull_count",
        "Total number of jobs pulled from the queue."
    )
    .unwrap();

    pub static ref WORKER_EXECUTION_FAILED: Arc<RwLock<HashMap<String, IntCounter>>> = Arc::new(RwLock::new(HashMap::new()));

}

#[cfg(feature = "failpoints")]
pub mod schedule_failpoints {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ScheduleFailPoint {
        SavepointCreate,
        Push,
        PushQuotaExceeded,
        SavepointCommit,
        ScheduleDisable,
    }

    tokio::task_local! {
        pub static ACTIVE: ScheduleFailPoint;
    }

    pub fn is_active(point: ScheduleFailPoint) -> bool {
        ACTIVE.try_with(|fp| *fp == point).unwrap_or(false)
    }
}

lazy_static::lazy_static! {
    pub static ref HTTP_CLIENT: Client = configure_client(reqwest::ClientBuilder::new()
        .user_agent("windmill/beta")
        .timeout(std::time::Duration::from_secs(20))
        .connect_timeout(std::time::Duration::from_secs(10)))
        .build().unwrap();

    pub static ref WMDEBUG_NO_DEBOUNCING: bool = std::env::var("WMDEBUG_NO_DEBOUNCING").is_ok();
    pub static ref WMDEBUG_FORCE_NO_LEGACY_DEBOUNCING_COMPAT: bool = std::env::var("WMDEBUG_FORCE_NO_LEGACY_DEBOUNCING_COMPAT").is_ok();

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
const GLOBAL_ERROR_HANDLER_USERNAME: &str = "global";
const SUCCESS_HANDLER_USERNAME: &str = "success_handler";

pub const ERROR_HANDLER_USER_GROUP: &str = "g/error_handler";
pub const ERROR_HANDLER_USER_EMAIL: &str = "error_handler@windmill.dev";
pub const SCHEDULE_ERROR_HANDLER_USER_EMAIL: &str = "schedule_error_handler@windmill.dev";
pub const SUCCESS_HANDLER_USER_GROUP: &str = "g/success_handler";
pub const SUCCESS_HANDLER_USER_EMAIL: &str = "success_handler@windmill.dev";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanceledBy {
    pub username: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobCompleted {
    pub job: MiniCompletedJob,
    pub preprocessed_args: Option<HashMap<String, Box<RawValue>>>,
    pub result: Arc<Box<RawValue>>,
    pub result_columns: Option<Vec<String>>,
    pub mem_peak: i32,
    pub success: bool,
    pub cached_res_path: Option<String>,
    pub token: String,
    pub canceled_by: Option<CanceledBy>,
    pub duration: Option<i64>,
    pub has_stream: Option<bool>,
    pub from_cache: Option<bool>,
    #[serde(skip)]
    pub flow_runners: Option<Arc<FlowRunners>>,
    #[serde(skip)]
    pub done_tx: Option<oneshot::Sender<()>>,
}

pub async fn cancel_single_job<'c>(
    username: &str,
    reason: Option<String>,
    job_running: QueuedJobV2,
    w_id: &str,
    mut tx: Transaction<'c, Postgres>,
    db: &Pool<Postgres>,
    force_cancel: bool,
) -> error::Result<(Transaction<'c, Postgres>, Option<Uuid>)> {
    let id = job_running.id;
    if force_cancel || (job_running.parent_job.is_none() && !job_running.running) {
        let username = username.to_string();
        let w_id = w_id.to_string();
        let db = db.clone();
        tracing::info!("cancelling job {:?}", job_running.id);
        tokio::task::spawn(async move {
            let reason: String = reason
                .clone()
                .unwrap_or_else(|| "unexplicited reasons".to_string());
            let e = serde_json::json!({"message": format!("Job canceled: {reason} by {username}"), "name": "Canceled", "reason": reason, "canceler": username});

            append_logs(
                &job_running.id,
                w_id.to_string(),
                format!("canceled by {username}: (force cancel: {force_cancel})"),
                &Connection::from(db.clone()),
            )
            .await;
            let memory_peak = job_running.memory_peak.unwrap_or(0);
            let add_job = add_completed_job_error(
                &db,
                &MiniCompletedJob::from(job_running),
                memory_peak,
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

    Ok((tx, Some(id)))
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
    //TODO fetch mini completed job instead of QueuedJob
    let job = get_queued_job_v2(&mut *tx, &id).await?;

    if job.is_none() {
        return Ok((tx, None));
    }

    let mut job = job.unwrap();

    if require_anonymous && job.created_by != "anonymous" {
        return Err(Error::BadRequest(
            "You are not logged in and this job was not created by an anonymous user like you so you cannot cancel it".to_string(),
        ));
    }

    if job.workspace_id != w_id {
        return Err(Error::BadRequest(
            "You are not authorized to cancel this job belonging to another workspace".to_string(),
        ));
    }

    if force_cancel {
        // if force canceling a flow step, make sure we force cancel from the highest parent
        loop {
            if job.parent_job.is_none() {
                break;
            }
            match get_queued_job_v2(&mut *tx, &job.parent_job.unwrap()).await? {
                Some(j) => {
                    job = j;
                }
                None => break,
            }
        }
    }

    // prevent cancelling a future tick of a schedule
    if let Some(schedule_path) = job.schedule_path().as_ref() {
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

    let job = job;

    // get all children using recursive CTE
    let mut jobs_to_cancel = sqlx::query!(
        r#"
WITH RECURSIVE job_tree AS (
    -- Base case: direct children of the given parent job
    SELECT id, parent_job, 1 AS depth
    FROM v2_job_queue 
    INNER JOIN v2_job USING (id)
    WHERE parent_job = $1 AND v2_job.workspace_id = $2

    UNION ALL

    -- Recursive case: fetch children of previously found jobs
    SELECT q.id, j.parent_job, t.depth + 1
    FROM v2_job_queue q
    INNER JOIN v2_job j USING (id)
    INNER JOIN job_tree t ON t.id = j.parent_job
    WHERE j.workspace_id = $2 AND t.depth < 500  -- Limit recursion depth to 500
)
SELECT id AS id, depth
FROM job_tree
ORDER BY depth, id
        "#,
        job.id,
        w_id
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .filter_map(|r| r.id.clone())
    .collect_vec();

    jobs_to_cancel.reverse();
    if !jobs_to_cancel.is_empty() {
        tracing::info!("Found {} child jobs to cancel", jobs_to_cancel.len());
    }

    let (ntx, _) =
        cancel_single_job(username, reason.clone(), job, w_id, tx, db, force_cancel).await?;
    tx = ntx;

    if !force_cancel {
        // cancel children in batch first
        if !jobs_to_cancel.is_empty() {
            let updated = sqlx::query_scalar!(
            "UPDATE v2_job_queue SET canceled_by = $1, canceled_reason = $2, scheduled_for = now(), suspend = 0 WHERE id = ANY($3) AND workspace_id = $4 AND (canceled_by IS NULL OR canceled_reason != $2) RETURNING id",
            username,
            reason,
            jobs_to_cancel.as_slice(),
            w_id
        )
        .fetch_all(&mut *tx)
        .await?;

            // Remove any jobs that were successfully updated
            jobs_to_cancel.retain(|id| !updated.contains(&id));
        }
    }
    for job_id in jobs_to_cancel {
        let job = get_queued_job_v2(&mut *tx, &job_id).await?;

        if let Some(job) = job {
            let (ntx, _) =
                cancel_single_job(username, reason.clone(), job, w_id, tx, db, force_cancel)
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
    conn: &Connection,
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
    match conn {
        Connection::Sql(pool) => {
            if let Err(err) = sqlx::query!(
                "INSERT INTO job_logs (logs, job_id, workspace_id) VALUES ($1, $2, $3) ON CONFLICT (job_id) DO UPDATE SET logs = concat(job_logs.logs, EXCLUDED.logs)",
                logs.as_ref(),
                job_id,
                workspace.as_ref(),
            )
            .execute(pool)
            .warn_after_seconds(1)
            .await
            {
                tracing::error!(%job_id, %err, "error updating logs for job {job_id}: {err}");
            }
        }
        Connection::Http(client) => {
            if let Err(e) = client
            .post::<_, String>(
                &format!("/api/w/{}/agent_workers/push_logs/{}", workspace.as_ref(), job_id),
                None,
                &logs.as_ref(),
            )
            .await {
                tracing::error!(%job_id, %e, "error sending logs for  job {job_id}: {e}");
            };
        }
    }
}

pub const PERIODIC_SCRIPT_TAG: &str = "periodic_bash_script";
pub const INIT_SCRIPT_TAG: &str = "init_script";
pub const INIT_SCRIPT_PATH_PREFIX: &str = "init_script_";
pub const PERIODIC_SCRIPT_PATH_PREFIX: &str = "periodic_script_";

pub async fn push_init_job<'c>(
    db: &Pool<Postgres>,
    content: String,
    worker_name: &str,
) -> error::Result<Uuid> {
    let tx = PushIsolationLevel::IsolatedRoot(db.clone());
    let ehm = HashMap::new();
    let (uuid, inner_tx) = push(
        &db,
        tx,
        "admins",
        windmill_common::jobs::JobPayload::Code(windmill_common::jobs::RawCode {
            hash: None,
            content,
            path: Some(format!("{INIT_SCRIPT_PATH_PREFIX}{worker_name}")),
            language: ScriptLang::Bash,
            lock: None,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            concurrency_settings: ConcurrencySettingsWithCustom::default(),
            debouncing_settings: DebouncingSettings::default(),
        }),
        PushArgs::from(&ehm),
        worker_name,
        "worker@windmill.dev",
        SUPERADMIN_SECRET_EMAIL.to_string(),
        Some("worker_init_job"),
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        true,
        None,
        true,
        Some(INIT_SCRIPT_TAG.to_string()),
        None,
        None,
        None,
        None,
        false,
        None,
        None,
        None,
    )
    .await?;
    inner_tx.commit().await?;
    Ok(uuid)
}

pub async fn push_periodic_bash_job<'c>(
    db: &Pool<Postgres>,
    content: String,
    worker_name: &str,
) -> error::Result<Uuid> {
    let tx = PushIsolationLevel::IsolatedRoot(db.clone());
    let ehm = HashMap::new();
    let timestamp = chrono::Utc::now().timestamp();
    let (uuid, inner_tx) = push(
        &db,
        tx,
        "admins",
        windmill_common::jobs::JobPayload::Code(windmill_common::jobs::RawCode {
            hash: None,
            content,
            path: Some(format!(
                "{PERIODIC_SCRIPT_PATH_PREFIX}{}_{}",
                worker_name, timestamp
            )),
            language: ScriptLang::Bash,
            lock: None,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            concurrency_settings: ConcurrencySettingsWithCustom::default(),
            debouncing_settings: DebouncingSettings::default(),
        }),
        PushArgs::from(&ehm),
        worker_name,
        "worker@windmill.dev",
        SUPERADMIN_SECRET_EMAIL.to_string(),
        Some("worker_periodic_script_job"),
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        true,
        None,
        true,
        Some(PERIODIC_SCRIPT_TAG.to_string()),
        None,
        None,
        None,
        None,
        false,
        None,
        None,
        None,
    )
    .await?;
    inner_tx.commit().await?;
    Ok(uuid)
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
        "SELECT j.id FROM v2_job_queue q JOIN v2_job j USING (id) WHERE j.workspace_id = $1 AND j.runnable_path = $2 AND q.canceled_by IS NULL",
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
    fn size(&self) -> usize;
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

    fn size(&self) -> usize {
        0
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

    fn size(&self) -> usize {
        self.get().len()
    }
}

impl<T: ValidableJson> ValidableJson for Arc<T> {
    fn is_valid_json(&self) -> bool {
        T::is_valid_json(&self)
    }

    fn wm_labels(&self) -> Option<Vec<String>> {
        T::wm_labels(&self)
    }

    fn size(&self) -> usize {
        T::size(&self)
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

    fn size(&self) -> usize {
        self.size_hint()
    }
}

impl<T: ValidableJson> ValidableJson for Json<T> {
    fn is_valid_json(&self) -> bool {
        self.0.is_valid_json()
    }

    fn wm_labels(&self) -> Option<Vec<String>> {
        self.0.wm_labels()
    }

    fn size(&self) -> usize {
        self.0.size()
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
    completed_job: &MiniCompletedJob,
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
        &completed_job.tag,
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
        completed_job.id,
        completed_job.workspace_id,
        serde_json::to_string(&result).unwrap_or_else(|_| "".to_string())
    );
    let _ = add_completed_job(
        db,
        &completed_job,
        false,
        false,
        Json(&result),
        None,
        mem_peak,
        canceled_by,
        flow_is_done,
        duration,
        false,
    )
    .warn_after_seconds(10)
    .await?;
    Ok(result)
}

lazy_static::lazy_static! {
    pub static ref GLOBAL_ERROR_HANDLER_PATH_IN_ADMINS_WORKSPACE: Option<String> = std::env::var("GLOBAL_ERROR_HANDLER_PATH_IN_ADMINS_WORKSPACE").ok();
    pub static ref MAX_RESULT_SIZE_MB: usize = std::env::var("MAX_RESULT_SIZE_MB").unwrap_or("500".to_string()).parse().unwrap_or(500);

    // Cache for restart_unless_cancelled flag - keyed by (hash, workspace_id)
    static ref RESTART_UNLESS_CANCELLED_CACHE: Cache<(i64, String), bool> = Cache::new(10000);

    // Cache for workspace error handler settings with 60s TTL
    // Key: workspace_id, Value: (error_handler, error_handler_extra_args, error_handler_muted_on_cancel, error_handler_muted_on_user_path, expiry_timestamp)
    static ref WORKSPACE_ERROR_HANDLER_CACHE: Cache<String, (Option<String>, Option<Json<Box<RawValue>>>, bool, bool, i64)> = Cache::new(1000);

    // Cache for workspace success handler settings with 60s TTL
    // Key: workspace_id, Value: (success_handler, success_handler_extra_args, expiry_timestamp)
    static ref WORKSPACE_SUCCESS_HANDLER_CACHE: Cache<String, (Option<String>, Option<Json<Box<RawValue>>>, i64)> = Cache::new(1000);
}

const WORKSPACE_HANDLER_CACHE_TTL_SECONDS: i64 = 60;

pub async fn add_completed_job<T: Serialize + Send + Sync + ValidableJson>(
    db: &Pool<Postgres>,
    completed_job: &MiniCompletedJob,
    success: bool,
    skipped: bool,
    result: Json<&T>,
    result_columns: Option<Vec<String>>,
    mem_peak: i32,
    canceled_by: Option<CanceledBy>,
    flow_is_done: bool,
    duration: Option<i64>,
    from_cache: bool,
) -> Result<(Uuid, i64), Error> {
    // tracing::error!("Start");
    // let start = tokio::time::Instant::now();

    // add_time!(bench, "add_completed_job start");
    if !result.is_valid_json() {
        return Err(Error::internal_err(
            "Result of job is invalid json (empty)".to_string(),
        ));
    }

    let result_columns = result_columns.as_ref();
    let (opt_uuid, duration, _skip_downstream_error_handlers) = (|| {
        commit_completed_job(
            db,
            completed_job,
            success,
            skipped,
            result,
            result_columns,
            mem_peak,
            &canceled_by,
            flow_is_done,
            duration,
            from_cache,
        )
        .warn_after_seconds(10)
    })
    .retry(
        ConstantBuilder::default()
            .with_delay(std::time::Duration::from_secs(3))
            .with_max_times(10)
            .build(),
    )
    .when(|err| {
        !matches!(err, Error::QuotaExceeded(_))
            && !matches!(err, Error::ResultTooLarge(_))
            && !matches!(err, Error::AlreadyCompleted(_))
            && !matches!(err, Error::NotFound(_))
    })
    .notify(|err, dur| {
        tracing::error!("Could not insert completed job, retrying in {dur:#?}, err: {err:#?}");
    })
    .sleep(tokio::time::sleep)
    .await?;

    // if scheduling next job failed, return the job_id early to ensure the job get retried after a timeout
    if let Some(job_id) = opt_uuid {
        return Ok((job_id, duration));
    }

    #[cfg(feature = "cloud")]
    apply_completed_job_cloud_usage(db, completed_job, duration);

    #[cfg(all(feature = "enterprise", feature = "private"))]
    crate::jobs_ee::apply_completed_job_error_handlers(
        db,
        completed_job,
        success,
        result,
        &canceled_by,
        _skip_downstream_error_handlers,
    )
    .await;

    restart_job_if_perpetual(db, completed_job, &canceled_by).await?;

    // tracing::error!("4 {:?}", start.elapsed());

    Ok((completed_job.id, duration))
}

async fn commit_completed_job<T: Serialize + Send + Sync + ValidableJson>(
    db: &Pool<Postgres>,
    completed_job: &MiniCompletedJob,
    success: bool,
    skipped: bool,
    result: Json<&T>,
    result_columns: Option<&Vec<String>>,
    mem_peak: i32,
    canceled_by: &Option<CanceledBy>,
    flow_is_done: bool,
    duration: Option<i64>,
    from_cache: bool,
) -> windmill_common::error::Result<(Option<Uuid>, i64, bool)> {
    // let start = std::time::Instant::now();

    let mut tx = db.begin().warn_after_seconds(10).await?;

    let job_id = completed_job.id;
    // tracing::error!("1 {:?}", start.elapsed());

    // tracing::debug!(
    //     "completed job {} {}",
    //     queued_job.id,
    //     serde_json::to_string(&result).unwrap_or_else(|_| "".to_string())
    // );

    let mem_peak = mem_peak;
    // add_time!(bench, "add_completed_job query START");

    if let Some(value) = check_result_size(db, completed_job, result).await {
        return value;
    }

    let duration =  sqlx::query_scalar!(
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
            /* $1 */ completed_job.id,
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
        .fetch_optional(&mut *tx)
        .warn_after_seconds(10)
        .await
        .map_err(|e| Error::internal_err(format!("Could not add completed job {job_id}: {e:#}")))?;

    let duration = if let Some(duration) = duration {
        duration
    } else {
        let already_inserted = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM v2_job_completed WHERE id = $1)",
            job_id
        )
        .fetch_one(&mut *tx)
        .warn_after_seconds(10)
        .await
        .map_err(|e| Error::internal_err(format!("Could not add completed job {job_id}: {e:#}")))?
        .unwrap_or(false);

        if already_inserted {
            return Err(Error::AlreadyCompleted(format!(
                "The queued job {job_id} is already completed."
            )));
        } else {
            return Err(Error::AlreadyCompleted(format!(
                "There is no queued job anymore for {job_id} but there is no completed job either."
            )));
        }
    };

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
        .warn_after_seconds(10)
        .await
        .map_err(|e| Error::InternalErr(format!("Could not update job labels: {e:#}")))?;
    }

    if !completed_job.is_flow_step() {
        if let Some(parent_job) = completed_job.parent_job {
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
                &completed_job.id.to_string(),
                duration,
                parent_job
            )
            .execute(&mut *tx)
            .warn_after_seconds(10)
            .await
            .inspect_err(|e| {
                tracing::error!(
                    "Could not update parent job `duration_ms` in workflow as code status: {}",
                    e,
                )
            });
        }
    }
    // tracing::error!("Added completed job {:#?}", queued_job);

    let mut _skip_downstream_error_handlers = false;
    tx = delete_job(tx, &job_id).warn_after_seconds(10).await?;
    // tracing::error!("3 {:?}", start.elapsed());

    if completed_job.is_flow_step() {
        if let Some(parent_job) = completed_job.parent_job {
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
                &completed_job.workspace_id
            )
            .execute(&mut *tx)
            .warn_after_seconds(10)
            .await?;
            if flow_is_done {
                let r = sqlx::query_scalar!(
                    "UPDATE parallel_monitor_lock SET last_ping = now() WHERE parent_flow_id = $1 and job_id = $2 RETURNING 1",
                    parent_job,
                    &completed_job.id
                ).fetch_optional(&mut *tx).warn_after_seconds(10).await?;
                if r.is_some() {
                    tracing::info!(
                            "parallel flow iteration is done, setting parallel monitor last ping lock for job {}",
                            &completed_job.id
                        );
                }
            }
        }
    } else {
        if completed_job.schedule_path().is_some() && completed_job.runnable_path.is_some() {
            let schedule_path = completed_job.schedule_path().unwrap();
            let script_path = completed_job.runnable_path.as_ref().unwrap();

            let schedule = get_schedule_opt(&mut *tx, &completed_job.workspace_id, &schedule_path)
                .warn_after_seconds(10)
                .await?;

            if let Some(schedule) = schedule {
                #[cfg(feature = "enterprise")]
                {
                    _skip_downstream_error_handlers = schedule.ws_error_handler_muted;
                }

                // for scripts, always try to schedule next tick
                // for flows, only try to schedule next tick here if flow failed and because first handle_flow failed (step = 0, modules[0] = {type: 'Failure', 'job': uuid::nil()})
                // or job was cancelled before first handle_flow was called (step = 0, modules = [] OR modules[0].type == 'WaitingForPriorSteps')
                // otherwise flow rescheduling is done inside handle_flow
                let schedule_next_tick = !completed_job.is_flow()
                    || from_cache
                    || !success
                        && sqlx::query_scalar!(
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
                            &completed_job.id,
                            &completed_job.workspace_id
                        )
                        .fetch_optional(&mut *tx)
                        .warn_after_seconds(10)
                        .await?
                        .flatten()
                        .unwrap_or(false);

                if schedule_next_tick {
                    let (returned_tx, schedule_push_err) =
                        try_schedule_next_job(db, tx, completed_job, &schedule, &script_path).await;
                    tx = returned_tx;
                    if let Some(err) = schedule_push_err {
                        return Err(err);
                    }
                }

                #[cfg(all(feature = "enterprise", feature = "private"))]
                if let Err(err) = crate::jobs_ee::apply_schedule_handlers(
                    db,
                    &schedule,
                    &script_path,
                    &completed_job.workspace_id,
                    success,
                    result,
                    job_id,
                    completed_job.started_at.unwrap_or(chrono::Utc::now()),
                    completed_job.priority,
                )
                .warn_after_seconds(10)
                .await
                {
                    if !success {
                        tracing::error!("Could not apply schedule error handler: {}", err);
                        let base_url = windmill_common::BASE_URL.read().await;
                        let w_id: &String = &completed_job.workspace_id;
                        if !matches!(err, Error::QuotaExceeded(_)) {
                            report_error_to_workspace_handler_or_critical_side_channel(
                                    &completed_job,
                                    db,
                                    format!(
                                        "Failed to push schedule error handler job to handle failed job ({base_url}/run/{}?workspace={w_id}): {}",
                                        completed_job.id,
                                        err
                                    ),
                                )
                                .warn_after_seconds(10)
                                .await;
                        }
                    } else {
                        tracing::error!("Could not apply schedule recovery handler: {}", err);
                    }
                };
            } else {
                tracing::error!(
                        "Schedule {schedule_path} in {} not found. Impossible to schedule again and apply schedule handlers",
                        &completed_job.workspace_id
                    );
            }
        }
    }

    if completed_job.concurrent_limit.is_some()
        || windmill_common::runnable_settings::prefetch_cached_from_handle(
            completed_job.runnable_settings_handle,
            db,
        )
        .await?
        .1
        .concurrent_limit
        .is_some()
    {
        let concurrency_key = sqlx::query_scalar!(
            "SELECT key FROM concurrency_key WHERE job_id = $1",
            &completed_job.id
        )
        .fetch_optional(&mut *tx)
        .warn_after_seconds(10)
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Could not get concurrency key for job {}: {e:#}",
                completed_job.id
            ))
        })?;
        if *DISABLE_CONCURRENCY_LIMIT || concurrency_key.is_none() {
            tracing::warn!("Concurrency limit is disabled, skipping");
        } else {
            let concurrency_key = concurrency_key.unwrap();
            sqlx::query_scalar!(
                "UPDATE concurrency_counter SET job_uuids = job_uuids - $2 WHERE concurrency_id = $1",
                concurrency_key,
                completed_job.id.hyphenated().to_string(),
            )
            .execute(&mut *tx)
            .warn_after_seconds(10)
            .await
            .map_err(|e| {
                Error::internal_err(format!(
                    "Could not decrement concurrency counter for job_id={}: {e:#}",
                    completed_job.id
                ))
            })?;
        }

        if let Err(e) = sqlx::query_scalar!(
            "UPDATE concurrency_key SET ended_at = now() WHERE job_id = $1",
            completed_job.id,
        )
        .execute(&mut *tx)
        .warn_after_seconds(10)
        .await
        {
            tracing::error!(
                "Could not update concurrency_key ended_at for job_id={}: {e:#}",
                completed_job.id,
            );
        }
        tracing::debug!("decremented concurrency counter");
    }

    tx.commit().warn_after_seconds(10).await?;

    tracing::info!(
        %job_id,
        root_job = ?completed_job.flow_innermost_root_job.map(|x| x.to_string()).unwrap_or_else(|| String::new()),
        path = &completed_job.runnable_path,
        job_kind = ?completed_job.kind,
        started_at = ?completed_job.started_at.map(|x| x.to_string()).unwrap_or_else(|| String::new()),
        duration = ?duration,
        permissioned_as = ?completed_job.permissioned_as,
        email = ?completed_job.permissioned_as_email,
        created_by = completed_job.created_by,
        is_flow_step = completed_job.is_flow_step(),
        language = completed_job.script_lang.map(|x| x.as_str()).unwrap_or_default(),
        scheduled_for = ?completed_job.scheduled_for,
        workspace_id = ?completed_job.workspace_id,
        success,
        "inserted completed job: {} (success: {success})",
        completed_job.id
    );
    // tracing::info!("completed job: {:?}", start.elapsed().as_micros());
    Ok((None, duration, _skip_downstream_error_handlers))
}

async fn check_result_size<T: ValidableJson>(
    db: &Pool<Postgres>,
    queued_job: &MiniCompletedJob,
    result: Json<&T>,
) -> Option<Result<(Option<Uuid>, i64, bool), Error>> {
    let result_size = result.size() / 1024 / 1024;
    if result_size > 2 {
        if result_size > *MAX_RESULT_SIZE_MB {
            tracing::error!(
                "Result of job {} is too large: {}MB > MAX_RESULT_SIZE_MB={}MB",
                queued_job.id,
                result_size,
                *MAX_RESULT_SIZE_MB
            );
            return Some(Err(Error::ResultTooLarge(format!("Result of job {} is too large: {}MB > MAX_RESULT_SIZE_MB={}MB.\nUse external storages such as the Windmill Object Storage to store large results: https://www.windmill.dev/docs/core_concepts/object_storage_in_windmill", queued_job.id, result_size, *MAX_RESULT_SIZE_MB))));
        }
        append_logs(
            &queued_job.id,
            &queued_job.workspace_id,
            format!("Warning: Result of job {} is large: {}MB.\nRecommended max size is 2MB.\nPrefer using external storages such as the Windmill Object Storage to store large results: https://www.windmill.dev/docs/core_concepts/object_storage_in_windmill", queued_job.id, result_size),
            &db.into(),
        )
        .await;
        if *CLOUD_HOSTED {
            return Some(Err(Error::ResultTooLarge(format!("Result of job {} is too large for multi-tenant cloud: {}MB (max 2MB).\nUse external storages such as the Windmill Object Storage to store large results: https://www.windmill.dev/docs/core_concepts/object_storage_in_windmill", queued_job.id, result_size))));
        } else {
            tracing::warn!(
                "Result of job {} is larger than 2MB: {}MB. Not recommended.",
                queued_job.id,
                result_size
            );
        }
    }
    None
}

async fn restart_job_if_perpetual(
    db: &Pool<Postgres>,
    queued_job: &MiniCompletedJob,
    canceled_by: &Option<CanceledBy>,
) -> Result<(), Error> {
    if !queued_job.is_flow_step() && queued_job.kind == JobKind::Script && canceled_by.is_none() {
        if let Some(hash) = queued_job.runnable_id {
            (|| restart_job_if_perpetual_inner(db, queued_job, hash))
                .retry(
                    ConstantBuilder::default()
                        .with_delay(std::time::Duration::from_secs(3))
                        .with_max_times(5)
                        .build(),
                )
                .notify(|err, dur| {
                    tracing::error!(
                    "Could not apply perpetual job restart, retrying in {dur:#?}, err: {err:#?}"
                );
                })
                .sleep(tokio::time::sleep)
                .await?;
        }
    }
    Ok(())
}

async fn restart_job_if_perpetual_inner(
    db: &Pool<Postgres>,
    queued_job: &MiniCompletedJob,
    hash: ScriptHash,
) -> Result<(), Error> {
    let cache_key = (hash.0, queued_job.workspace_id.clone());

    let restart = if let Some(cached) = RESTART_UNLESS_CANCELLED_CACHE.get(&cache_key) {
        cached
    } else {
        let restart = sqlx::query_scalar!(
            "SELECT restart_unless_cancelled FROM script WHERE hash = $1 AND workspace_id = $2",
            hash.0,
            &queued_job.workspace_id
        )
        .fetch_optional(db)
        .await?
        .flatten()
        .unwrap_or(false);

        RESTART_UNLESS_CANCELLED_CACHE.insert(cache_key, restart);
        restart
    };

    if restart {
        let tx = PushIsolationLevel::IsolatedRoot(db.clone());

        // perpetual jobs can run one job per 10s max. If the job was faster than 10s, schedule the next one with the appropriate delay
        let now = now_from_db(db).await?;
        let scheduled_for = if now
            .signed_duration_since(queued_job.started_at.unwrap_or(now))
            .num_seconds()
            < 10
        {
            let next_run =
                queued_job.started_at.unwrap_or(now) + chrono::Duration::try_seconds(10).unwrap();
            tracing::warn!("Perpetual script {:?} is running too fast, only 1 job per 10s it supported. Scheduling next run for {:?}", queued_job.runnable_path, next_run);
            Some(next_run)
        } else {
            None
        };

        let args = sqlx::query_scalar!(
            "SELECT args as \"args: sqlx::types::Json<HashMap<String, Box<RawValue>>>\" FROM v2_job WHERE id = $1 AND workspace_id = $2",
            queued_job.id,
            queued_job.workspace_id
        )
        .fetch_optional(db)
        .await?
        .flatten()
        .unwrap_or_default();
        let (_uuid, tx) = push(
            db,
            tx,
            &queued_job.workspace_id,
            JobPayload::ScriptHash {
                hash,
                path: queued_job.runnable_path.clone().unwrap_or_default(),
                cache_ttl: queued_job.cache_ttl,
                cache_ignore_s3_path: queued_job.cache_ignore_s3_path,
                dedicated_worker: None,
                language: queued_job
                    .script_lang
                    .clone()
                    .unwrap_or_else(|| ScriptLang::Deno),
                priority: queued_job.priority,
                apply_preprocessor: false,
                concurrency_settings: ConcurrencySettings {
                    concurrency_key: custom_concurrency_key(db, &queued_job.id).await?,
                    concurrent_limit: None,
                    concurrency_time_window_s: None,
                },
                // TODO(debouncing): handle properly
                debouncing_settings: DebouncingSettings::default(),
            },
            PushArgs::from(&args.0),
            &queued_job.created_by,
            &queued_job.permissioned_as_email,
            queued_job.permissioned_as.clone(),
            Some(&format!("add.completed.job{}", queued_job.id)),
            scheduled_for,
            queued_job.schedule_path(),
            None,
            None,
            None,
            None,
            false,
            false,
            None,
            true,
            Some(queued_job.tag.clone()),
            None,
            None,
            queued_job.priority,
            None,
            false,
            None,
            None,
            None,
        )
        .await?;
        tx.commit().await?;
    }
    Ok(())
}

#[cfg(feature = "cloud")]
fn apply_completed_job_cloud_usage(
    db: &Pool<Postgres>,
    queued_job: &MiniCompletedJob,
    _duration: i64,
) {
    if *CLOUD_HOSTED && !queued_job.is_flow() && _duration > 1000 {
        let db = db.clone();
        let w_id = queued_job.workspace_id.clone();
        let email = queued_job.permissioned_as_email.clone();
        let w_id2 = w_id.clone();
        let email2 = email.clone();
        tokio::task::spawn(async move {
            let additional_usage = _duration / 1000;
            let result = tokio::time::timeout(std::time::Duration::from_secs(10), async move {
                // Update workspace usage
                let workspace_result = sqlx::query!(
                    "INSERT INTO usage (id, is_workspace, month_, usage)
                    VALUES ($1, TRUE, EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date), $2)
                    ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + EXCLUDED.usage",
                    &w_id,
                    additional_usage as i32
                )
                .execute(&db)
                .await;

                if let Err(e) = workspace_result {
                    tracing::error!("Failed to update workspace usage for {}: {:#}", w_id, e);
                }

                match windmill_common::workspaces::get_team_plan_status(&db, &w_id).await {
                    Ok(team_plan_status) => {
                        // Update user usage for non-premium workspaces
                        if !team_plan_status.premium {
                            let user_result = sqlx::query!(
                                "INSERT INTO usage (id, is_workspace, month_, usage)
                                VALUES ($1, FALSE, EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date), $2)
                                ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + EXCLUDED.usage",
                                &email,
                                additional_usage as i32
                            )
                            .execute(&db)
                            .await;

                            if let Err(e) = user_result {
                                tracing::error!("Failed to update user usage for {}: {:#}", email, e);
                            }
                        }
                    },
                    Err(err) => {
                        tracing::error!("Failed to get team plan status to update usage for workspace {w_id}: {err:#}");
                    }
                };
            }).await;

            if let Err(_) = result {
                tracing::error!(
                    "Could not update usage for workspace {} and permissioned as {}, stopped after 10s",
                    w_id2,
                    email2
                );
            }
        });
    }
}

pub async fn send_error_to_global_handler<'a, T: Serialize + Send + Sync>(
    queued_job: &MiniCompletedJob,
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
            queued_job.schedule_path(),
            queued_job.runnable_path.clone(),
            queued_job.is_flow(),
            &queued_job.workspace_id,
            &prefixed_global_error_handler_path,
            result,
            None,
            queued_job.started_at,
            None,
            &queued_job.permissioned_as_email,
            false,
            true,
            None,
        )
        .await?;
    }

    Ok(())
}

pub async fn report_error_to_workspace_handler_or_critical_side_channel(
    queued_job: &MiniCompletedJob,
    db: &Pool<Postgres>,
    error_message: String,
) -> () {
    let w_id = &queued_job.workspace_id;
    let row_result = sqlx::query_as::<_, (Option<String>, Option<Json<Box<RawValue>>>)>(
        r#"
                SELECT
                    error_handler->>'path',
                    (error_handler->'extra_args')::text::json
                FROM
                    workspace_settings
                WHERE
                    workspace_id = $1
                "#,
    )
    .bind(&w_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .unwrap_or((None, None));

    let (error_handler, error_handler_extra_args) = row_result;

    if let Some(error_handler) = error_handler {
        if let Err(err) = push_error_handler(
            db,
            queued_job.id,
            queued_job.schedule_path(),
            queued_job.runnable_path.clone(),
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
            &queued_job.permissioned_as_email,
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

async fn fetch_error_handler_from_db(
    db: &Pool<Postgres>,
    w_id: &str,
) -> Result<(Option<String>, Option<Json<Box<RawValue>>>, bool, bool), Error> {
    sqlx::query_as::<
        _,
        (
            Option<String>,
            Option<Json<Box<RawValue>>>,
            Option<bool>,
            Option<bool>,
        ),
    >(
        r#"
        SELECT
            error_handler->>'path',
            (error_handler->'extra_args')::text::json,
            (error_handler->>'muted_on_cancel')::boolean,
            (error_handler->>'muted_on_user_path')::boolean
        FROM workspace_settings
        WHERE workspace_id = $1
        "#,
    )
    .bind(w_id)
    .fetch_optional(db)
    .await
    .context("fetching error handler info from workspace_settings")?
    .map(|(path, extra_args, muted_on_cancel, muted_on_user_path)| {
        (
            path,
            extra_args,
            muted_on_cancel.unwrap_or(false),
            muted_on_user_path.unwrap_or(false),
        )
    })
    .ok_or_else(|| Error::internal_err(format!("no workspace settings for id {w_id}")))
}

pub async fn send_error_to_workspace_handler<'a, 'c, T: Serialize + Send + Sync>(
    queued_job: &MiniCompletedJob,
    is_canceled: bool,
    db: &Pool<Postgres>,
    result: Json<&'a T>,
) -> Result<(), Error> {
    let w_id = &queued_job.workspace_id;

    let now = chrono::Utc::now().timestamp();
    let (
        error_handler,
        error_handler_extra_args,
        error_handler_muted_on_cancel,
        error_handler_muted_on_user_path,
    ) = if let Some(cached) = WORKSPACE_ERROR_HANDLER_CACHE.get(w_id) {
        if cached.4 > now {
            (cached.0.clone(), cached.1.clone(), cached.2, cached.3)
        } else {
            let row = fetch_error_handler_from_db(db, w_id).await?;
            let expiry = now + WORKSPACE_HANDLER_CACHE_TTL_SECONDS;
            WORKSPACE_ERROR_HANDLER_CACHE.insert(
                w_id.clone(),
                (row.0.clone(), row.1.clone(), row.2, row.3, expiry),
            );
            row
        }
    } else {
        let row = fetch_error_handler_from_db(db, w_id).await?;
        let expiry = now + WORKSPACE_HANDLER_CACHE_TTL_SECONDS;
        WORKSPACE_ERROR_HANDLER_CACHE.insert(
            w_id.clone(),
            (row.0.clone(), row.1.clone(), row.2, row.3, expiry),
        );
        row
    };

    if is_canceled && error_handler_muted_on_cancel {
        return Ok(());
    }

    // Skip error handler for scripts/flows starting with u/
    if error_handler_muted_on_user_path {
        if let Some(ref path) = queued_job.runnable_path {
            if path.starts_with("u/") {
                return Ok(());
            }
        }
    }

    if let Some(error_handler) = error_handler {
        let ws_error_handler_muted: Option<bool> = match queued_job.kind {
            JobKind::Script => {
                sqlx::query_scalar!(
                "SELECT ws_error_handler_muted FROM script WHERE workspace_id = $1 AND hash = $2",
                queued_job.workspace_id,
                queued_job.runnable_id.map(|x| x.0),
            )
                .fetch_optional(db)
                .await?
            }
            JobKind::Flow => {
                sqlx::query_scalar!(
                    "SELECT ws_error_handler_muted FROM flow WHERE workspace_id = $1 AND path = $2",
                    queued_job.workspace_id,
                    queued_job.runnable_path.clone(),
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
                queued_job.schedule_path(),
                queued_job.runnable_path.clone(),
                queued_job.is_flow(),
                &queued_job.workspace_id,
                &error_handler,
                result,
                None,
                queued_job.started_at,
                error_handler_extra_args,
                &queued_job.permissioned_as_email,
                false,
                false,
                None,
            )
            .await?;
        }
    }
    Ok(())
}

async fn fetch_success_handler_from_db(
    db: &Pool<Postgres>,
    w_id: &str,
) -> Result<(Option<String>, Option<Json<Box<RawValue>>>), Error> {
    sqlx::query_as::<_, (Option<String>, Option<Json<Box<RawValue>>>)>(
        r#"
        SELECT
            success_handler->>'path',
            (success_handler->'extra_args')::text::json
        FROM workspace_settings
        WHERE workspace_id = $1
        "#,
    )
    .bind(w_id)
    .fetch_optional(db)
    .await
    .context("fetching success handler info from workspace_settings")?
    .ok_or_else(|| Error::internal_err(format!("no workspace settings for id {w_id}")))
}

pub async fn send_success_to_workspace_handler<'a, 'c, T: Serialize + Send + Sync>(
    queued_job: &MiniCompletedJob,
    db: &Pool<Postgres>,
    result: Json<&'a T>,
) -> Result<(), Error> {
    let w_id = &queued_job.workspace_id;

    let now = chrono::Utc::now().timestamp();
    let (success_handler, success_handler_extra_args) =
        if let Some(cached) = WORKSPACE_SUCCESS_HANDLER_CACHE.get(w_id) {
            if cached.2 > now {
                (cached.0.clone(), cached.1.clone())
            } else {
                let row = fetch_success_handler_from_db(db, w_id).await?;
                let expiry = now + WORKSPACE_HANDLER_CACHE_TTL_SECONDS;
                WORKSPACE_SUCCESS_HANDLER_CACHE
                    .insert(w_id.clone(), (row.0.clone(), row.1.clone(), expiry));
                row
            }
        } else {
            let row = fetch_success_handler_from_db(db, w_id).await?;
            let expiry = now + WORKSPACE_HANDLER_CACHE_TTL_SECONDS;
            WORKSPACE_SUCCESS_HANDLER_CACHE
                .insert(w_id.clone(), (row.0.clone(), row.1.clone(), expiry));
            row
        };

    if let Some(success_handler) = success_handler {
        tracing::info!("workspace success handler for job {}", &queued_job.id);

        push_success_handler(
            db,
            queued_job.id,
            queued_job.schedule_path(),
            queued_job.runnable_path.clone(),
            queued_job.is_flow(),
            &queued_job.workspace_id,
            &success_handler,
            result,
            queued_job.started_at,
            success_handler_extra_args,
            &queued_job.permissioned_as_email,
            None,
        )
        .await?;
    }
    Ok(())
}

pub async fn try_schedule_next_job<'c>(
    db: &Pool<Postgres>,
    mut tx: Transaction<'c, Postgres>,
    job: &MiniCompletedJob,
    schedule: &Schedule,
    script_path: &str,
) -> (Transaction<'c, Postgres>, Option<Error>) {
    if !schedule.enabled {
        tracing::info!(
            "Schedule {} in {} is disabled. Not scheduling again.",
            schedule.path,
            &job.workspace_id
        );
        return (tx, None);
    }

    if script_path != schedule.script_path {
        tracing::warn!(
            "Schedule {} in {} has a different script path than the job. Not scheduling again",
            schedule.path,
            &job.workspace_id
        );
        return (tx, None);
    }

    tracing::info!(
        "Schedule {} scheduling next job for {} in {}",
        schedule.path,
        schedule.script_path,
        &job.workspace_id
    );

    let schedule_authed = windmill_common::auth::fetch_authed_from_permissioned_as(
        &windmill_common::users::username_to_permissioned_as(&schedule.edited_by),
        &schedule.email,
        &job.workspace_id,
        &mut *tx,
    )
    .await
    .ok();

    let mut push_err = None;

    #[cfg(feature = "failpoints")]
    if schedule_failpoints::is_active(schedule_failpoints::ScheduleFailPoint::SavepointCreate) {
        push_err = Some(Error::internal_err(
            "failpoint: savepoint create".to_string(),
        ));
    }

    if push_err.is_none() {
        let savepoint_result = tx.begin().await;
        match savepoint_result {
            Ok(savepoint) => {
                let push_result = match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    push_scheduled_job(
                        db,
                        savepoint,
                        schedule,
                        schedule_authed.as_ref(),
                        Some(job.scheduled_for),
                    ),
                )
                .await
                {
                    Ok(result) => result,
                    Err(_elapsed) => Err(Error::internal_err(
                        "push_scheduled_job timed out after 5s".to_string(),
                    )),
                };
                #[cfg(feature = "failpoints")]
                let push_result =
                    if schedule_failpoints::is_active(schedule_failpoints::ScheduleFailPoint::Push)
                    {
                        if let Ok(sp) = push_result {
                            sp.rollback().await.ok();
                        }
                        Err(Error::internal_err("failpoint: push".to_string()))
                    } else if schedule_failpoints::is_active(
                        schedule_failpoints::ScheduleFailPoint::PushQuotaExceeded,
                    ) {
                        if let Ok(sp) = push_result {
                            sp.rollback().await.ok();
                        }
                        Err(Error::QuotaExceeded(
                            "failpoint: push quota exceeded".to_string(),
                        ))
                    } else {
                        push_result
                    };
                match push_result {
                    Ok(savepoint) => {
                        #[cfg(feature = "failpoints")]
                        let savepoint_commit_fail = schedule_failpoints::is_active(
                            schedule_failpoints::ScheduleFailPoint::SavepointCommit,
                        );
                        #[cfg(not(feature = "failpoints"))]
                        let savepoint_commit_fail = false;

                        if savepoint_commit_fail {
                            savepoint.rollback().await.ok();
                            push_err = Some(Error::internal_err(
                                "failpoint: savepoint commit".to_string(),
                            ));
                        } else {
                            match savepoint.commit().await {
                                Ok(()) => {}
                                Err(e) => {
                                    push_err = Some(Error::internal_err(format!(
                                        "Could not commit savepoint: {e:#}"
                                    )));
                                }
                            }
                        }
                    }
                    Err(err) if matches!(err, Error::QuotaExceeded(_)) => {
                        push_err = Some(err);
                    }
                    Err(err) => {
                        tracing::warn!(
                            "Could not push next scheduled job for {}: {err}",
                            schedule.path,
                        );
                        push_err = Some(err);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Could not create savepoint for schedule push: {e:#}",);
                push_err = Some(Error::internal_err(format!(
                    "Could not create savepoint: {e:#}"
                )));
            }
        }
    }

    if let Some(ref err) = push_err {
        if matches!(err, Error::QuotaExceeded(_) | Error::NotFound(_)) {
            tracing::error!(
                "Could not push next scheduled job for {}: {err}. Disabling schedule.",
                schedule.path
            );
            let disable_result = sqlx::query!(
                "UPDATE schedule SET enabled = false, error = $1 WHERE workspace_id = $2 AND path = $3",
                err.to_string(),
                &schedule.workspace_id,
                &schedule.path
            )
            .execute(&mut *tx)
            .await;
            #[cfg(feature = "failpoints")]
            let disable_result = if schedule_failpoints::is_active(
                schedule_failpoints::ScheduleFailPoint::ScheduleDisable,
            ) {
                Err(sqlx::Error::Protocol(
                    "failpoint: schedule disable".to_string(),
                ))
            } else {
                disable_result
            };
            if let Err(disable_err) = disable_result {
                report_error_to_workspace_handler_or_critical_side_channel(
                    job,
                    db,
                    format!(
                        "Could not push next scheduled job for {} and could not disable schedule: {disable_err}",
                        schedule.path,
                    ),
                )
                .await;
            } else {
                push_err = None;
            }
        }
    }

    (tx, push_err)
}

pub const ERROR_HANDLER_PATH_TEAMS: &str = "/workspace-or-schedule-error-handler-teams";
pub const ERROR_HANDLER_PATH_SLACK: &str = "/workspace-or-schedule-error-handler-slack";
pub const ERROR_HANDLER_PATH_EMAIL: &str = "/workspace-or-error-handler-email";

enum ErrorHandlerType {
    Custom,
    Teams,
    Slack,
    Email,
}

impl ErrorHandlerType {
    fn from_error_handler_path(error_handler_path: &str) -> Option<ErrorHandlerType> {
        let error_handler_path = if error_handler_path.starts_with("script/") {
            error_handler_path.strip_prefix("script/").unwrap()
        } else if error_handler_path.starts_with("flow/") {
            error_handler_path.strip_prefix("flow/").unwrap()
        } else {
            error_handler_path
        };

        if let Some(from_hub) = error_handler_path.strip_prefix("hub/") {
            let handler_type = if from_hub.ends_with(ERROR_HANDLER_PATH_TEAMS) {
                ErrorHandlerType::Teams
            } else if from_hub.ends_with(ERROR_HANDLER_PATH_SLACK) {
                ErrorHandlerType::Slack
            } else if from_hub.ends_with(ERROR_HANDLER_PATH_EMAIL) {
                ErrorHandlerType::Email
            } else {
                return None;
            };

            return Some(handler_type);
        }

        Some(ErrorHandlerType::Custom)
    }
}

fn get_email_and_permissioned_as(
    error_handler_path: &str,
    is_global_error_handler: bool,
    is_schedule_error_handler: bool,
) -> (&'static str, String) {
    let res = if is_global_error_handler {
        (SUPERADMIN_SECRET_EMAIL, SUPERADMIN_SECRET_EMAIL.to_string())
    } else if is_schedule_error_handler {
        (
            SCHEDULE_ERROR_HANDLER_USER_EMAIL,
            ERROR_HANDLER_USER_GROUP.to_string(),
        )
    } else {
        let handler_type = ErrorHandlerType::from_error_handler_path(error_handler_path);

        let email = match handler_type {
            Some(ErrorHandlerType::Email) => EMAIL_ERROR_HANDLER_USER_EMAIL,
            _ => ERROR_HANDLER_USER_EMAIL,
        };

        (email, ERROR_HANDLER_USER_GROUP.to_string())
    };

    res
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
    } else {
        get_email_and_permissioned_as(
            on_failure_path,
            is_global_error_handler,
            is_schedule_error_handler,
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
            GLOBAL_ERROR_HANDLER_USERNAME
        } else if is_schedule_error_handler {
            SCHEDULE_ERROR_HANDLER_USERNAME
        } else {
            ERROR_HANDLER_USERNAME
        },
        email,
        permissioned_as,
        Some(&format!("error.handler.{job_id}")),
        None,
        None,
        Some(job_id),
        None,
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
        false,
        None,
        None,
        None,
    )
    .await?;
    tx.commit().await?;
    return Ok(uuid);
}

pub async fn push_success_handler<'a, 'c, T: Serialize + Send + Sync>(
    db: &Pool<Postgres>,
    job_id: Uuid,
    schedule_path: Option<String>,
    script_path: Option<String>,
    is_flow: bool,
    w_id: &str,
    on_success_path: &str,
    result: Json<&'a T>,
    started_at: Option<DateTime<Utc>>,
    extra_args: Option<Json<Box<RawValue>>>,
    email: &str,
    priority: Option<i16>,
) -> windmill_common::error::Result<Uuid> {
    let (payload, tag, on_behalf_of) =
        get_payload_tag_from_prefixed_path(on_success_path, db, w_id).await?;

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
    } else {
        (
            SUCCESS_HANDLER_USER_EMAIL,
            SUCCESS_HANDLER_USER_GROUP.to_string(),
        )
    };

    let tx = PushIsolationLevel::IsolatedRoot(db.clone());
    let (uuid, tx) = push(
        &db,
        tx,
        w_id,
        payload,
        PushArgs { extra: Some(extra), args: &result },
        SUCCESS_HANDLER_USERNAME,
        email,
        permissioned_as,
        Some(&format!("success.handler.{job_id}")),
        None,
        None,
        Some(job_id), // parent_job
        Some(job_id), // root_job
        Some(job_id), // flow_innermost_root_job
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
        false,
        None,
        None,
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

#[derive(sqlx::FromRow, Debug, Clone, Serialize, Deserialize)]
pub struct MiniPulledJob {
    pub workspace_id: String,
    pub id: Uuid,
    pub args: Option<Json<HashMap<String, Box<RawValue>>>>,
    pub parent_job: Option<Uuid>,
    pub created_by: String,
    pub scheduled_for: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub runnable_path: Option<String>,
    pub kind: JobKind,
    pub runnable_id: Option<ScriptHash>,
    pub canceled_reason: Option<String>,
    pub canceled_by: Option<String>,
    pub permissioned_as: String,
    pub permissioned_as_email: String,
    pub flow_status: Option<Json<Box<RawValue>>>,
    pub tag: String,
    pub script_lang: Option<ScriptLang>,
    pub same_worker: bool,
    pub pre_run_error: Option<String>,
    pub concurrent_limit: Option<i32>,
    pub concurrency_time_window_s: Option<i32>,
    pub flow_innermost_root_job: Option<Uuid>,
    pub root_job: Option<Uuid>,
    pub timeout: Option<i32>,
    pub flow_step_id: Option<String>,
    pub cache_ttl: Option<i32>,
    pub cache_ignore_s3_path: Option<bool>,
    pub priority: Option<i16>,
    pub preprocessed: Option<bool>,
    pub script_entrypoint_override: Option<String>,
    pub trigger: Option<String>,
    pub trigger_kind: Option<JobTriggerKind>,
    pub visible_to_owner: bool,
    pub permissioned_as_end_user_email: Option<String>,
    pub runnable_settings_handle: Option<i64>,
}

impl MiniPulledJob {
    pub fn new_inline(
        workspace_id: String,
        args: Option<HashMap<String, Box<RawValue>>>,
        created_by: String,
        permissioned_as: String,
        permissioned_as_email: String,
        runnable_path: Option<String>,
        kind: JobKind,
        runnable_id: Option<ScriptHash>,
        tag: String,
        script_lang: Option<ScriptLang>,
    ) -> Self {
        Self {
            workspace_id,
            id: Uuid::new_v4(),
            args: args.map(Json),
            parent_job: None,
            created_by,
            scheduled_for: chrono::Utc::now(),
            started_at: None,
            runnable_path,
            kind,
            runnable_id,
            canceled_reason: None,
            canceled_by: None,
            permissioned_as,
            permissioned_as_email,
            flow_status: None,
            tag,
            script_lang,
            same_worker: true,
            pre_run_error: None,
            flow_innermost_root_job: None,
            root_job: None,
            timeout: None,
            flow_step_id: None,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            priority: None,
            preprocessed: None,
            script_entrypoint_override: None,
            trigger: None,
            trigger_kind: None,
            visible_to_owner: false,
            permissioned_as_end_user_email: None,
            runnable_settings_handle: None,
            concurrent_limit: None,
            concurrency_time_window_s: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniCompletedJob {
    pub id: Uuid,
    pub workspace_id: String,
    pub runnable_id: Option<ScriptHash>,
    pub scheduled_for: chrono::DateTime<chrono::Utc>,
    pub parent_job: Option<Uuid>,
    // pub root_job: Option<Uuid>,
    pub flow_innermost_root_job: Option<Uuid>,
    pub runnable_path: Option<String>,
    pub kind: JobKind,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub permissioned_as: String,
    pub created_by: String,
    pub script_lang: Option<ScriptLang>,
    pub permissioned_as_email: String,
    pub flow_step_id: Option<String>,
    pub trigger_kind: Option<JobTriggerKind>,
    pub trigger: Option<String>,
    pub priority: Option<i16>,
    pub concurrent_limit: Option<i32>,
    pub tag: String,
    pub cache_ttl: Option<i32>,
    pub cache_ignore_s3_path: Option<bool>,
    pub runnable_settings_handle: Option<i64>,
}

impl From<QueuedJobV2> for MiniCompletedJob {
    fn from(job: QueuedJobV2) -> Self {
        MiniCompletedJob {
            id: job.id,
            workspace_id: job.workspace_id,
            runnable_id: job.runnable_id,
            scheduled_for: job.scheduled_for,
            parent_job: job.parent_job,
            flow_innermost_root_job: job.flow_innermost_root_job,
            runnable_path: job.runnable_path,
            kind: job.kind,
            started_at: job.started_at,
            permissioned_as: job.permissioned_as,
            created_by: job.created_by,
            script_lang: job.script_lang,
            permissioned_as_email: job.permissioned_as_email,
            flow_step_id: job.flow_step_id,
            trigger_kind: job.trigger_kind,
            trigger: job.trigger,
            priority: job.priority,
            concurrent_limit: job.concurrent_limit,
            tag: job.tag,
            cache_ttl: job.cache_ttl,
            cache_ignore_s3_path: job.cache_ignore_s3_path,
            runnable_settings_handle: job.runnable_settings_handle,
        }
    }
}

impl From<MiniPulledJob> for MiniCompletedJob {
    fn from(job: MiniPulledJob) -> Self {
        MiniCompletedJob {
            id: job.id,
            workspace_id: job.workspace_id,
            runnable_id: job.runnable_id,
            scheduled_for: job.scheduled_for,
            parent_job: job.parent_job,
            // root_job: job.root_job,,
            flow_innermost_root_job: job.flow_innermost_root_job,
            runnable_path: job.runnable_path,
            kind: job.kind,
            started_at: job.started_at,
            permissioned_as: job.permissioned_as,
            created_by: job.created_by,
            script_lang: job.script_lang,
            permissioned_as_email: job.permissioned_as_email,
            flow_step_id: job.flow_step_id,
            trigger_kind: job.trigger_kind,
            trigger: job.trigger,
            priority: job.priority,
            concurrent_limit: job.concurrent_limit,
            tag: job.tag,
            cache_ttl: job.cache_ttl,
            cache_ignore_s3_path: job.cache_ignore_s3_path,
            runnable_settings_handle: job.runnable_settings_handle,
        }
    }
}

impl From<Arc<MiniPulledJob>> for MiniCompletedJob {
    fn from(job: Arc<MiniPulledJob>) -> Self {
        MiniCompletedJob {
            id: job.id,
            workspace_id: job.workspace_id.clone(),
            runnable_id: job.runnable_id,
            scheduled_for: job.scheduled_for,
            parent_job: job.parent_job,
            flow_innermost_root_job: job.flow_innermost_root_job,
            runnable_path: job.runnable_path.clone(),
            kind: job.kind,
            started_at: job.started_at,
            permissioned_as: job.permissioned_as.clone(),
            created_by: job.created_by.clone(),
            script_lang: job.script_lang,
            permissioned_as_email: job.permissioned_as_email.clone(),
            flow_step_id: job.flow_step_id.clone(),
            trigger_kind: job.trigger_kind.clone(),
            trigger: job.trigger.clone(),
            priority: job.priority,
            concurrent_limit: job.concurrent_limit,
            tag: job.tag.clone(),
            cache_ttl: job.cache_ttl,
            cache_ignore_s3_path: job.cache_ignore_s3_path,
            runnable_settings_handle: job.runnable_settings_handle,
        }
    }
}

impl MiniCompletedJob {
    pub fn is_flow_step(&self) -> bool {
        self.flow_step_id.is_some()
    }
    pub fn schedule_path(&self) -> Option<String> {
        schedule_path(&self.trigger_kind, &self.trigger)
    }

    pub fn is_flow(&self) -> bool {
        self.kind.is_flow()
    }

    pub fn is_dependency(&self) -> bool {
        self.kind.is_dependency()
    }
}

fn schedule_path(
    trigger_kind: &Option<JobTriggerKind>,
    trigger: &Option<String>,
) -> Option<String> {
    if trigger_kind
        .as_ref()
        .is_some_and(|t| matches!(t, JobTriggerKind::Schedule))
    {
        trigger.clone()
    } else {
        None
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FlowStatusChatInputEnabled {
    chat_input_enabled: Option<bool>,
}

impl MiniPulledJob {
    pub fn runnable_path(&self) -> &str {
        self.runnable_path
            .as_ref()
            .map(String::as_str)
            .unwrap_or("tmp/main")
    }

    pub fn is_flow_step(&self) -> bool {
        self.flow_step_id.is_some()
    }

    pub fn is_canceled(&self) -> bool {
        self.canceled_by.is_some()
    }

    pub fn parse_flow_status(&self) -> Option<FlowStatus> {
        // tracing::error!("parse_flow_status: {:?}", self.flow_status);

        self.flow_status
            .as_ref()
            .and_then(|v| serde_json::from_str::<FlowStatus>((**v).get()).ok())
    }

    pub fn parse_chat_input_enabled(&self) -> Option<bool> {
        self.flow_status
            .as_ref()
            .and_then(|v| serde_json::from_str::<FlowStatusChatInputEnabled>((**v).get()).ok())
            .and_then(|f| f.chat_input_enabled)
    }

    pub fn from(job: &QueuedJob) -> MiniPulledJob {
        MiniPulledJob {
            workspace_id: job.workspace_id.clone(),
            id: job.id,
            args: job.args.clone(),
            parent_job: job.parent_job.clone(),
            created_by: job.created_by.clone(),
            started_at: job.started_at.clone(),
            scheduled_for: job.scheduled_for,
            runnable_path: job.script_path.clone(),
            kind: job.job_kind,
            runnable_id: job.script_hash.clone(),
            canceled_reason: job.canceled_reason.clone(),
            canceled_by: job.canceled_by.clone(),
            permissioned_as: job.permissioned_as.clone(),
            permissioned_as_email: job.email.clone(),
            flow_status: job.flow_status.clone(),
            tag: job.tag.clone(),
            script_lang: job.language.clone(),
            same_worker: job.same_worker,
            pre_run_error: job.pre_run_error.clone(),
            concurrent_limit: job.concurrent_limit.clone(),
            concurrency_time_window_s: job.concurrency_time_window_s.clone(),
            runnable_settings_handle: job.runnable_settings_handle,
            flow_innermost_root_job: job.root_job.clone(), // QueuedJob is taken from v2_as_queue, where root_job corresponds to flow_innermost_root_job in v2_job
            root_job: None,
            timeout: job.timeout.clone(),
            flow_step_id: job.flow_step_id.clone(),
            cache_ttl: job.cache_ttl.clone(),
            cache_ignore_s3_path: job.cache_ignore_s3_path.clone(),
            priority: job.priority.clone(),
            preprocessed: job.preprocessed.clone(),
            script_entrypoint_override: job.script_entrypoint_override.clone(),
            trigger: job.schedule_path.clone(),
            trigger_kind: if job.schedule_path.is_some() {
                Some(JobTriggerKind::Schedule)
            } else {
                None
            },
            visible_to_owner: job.visible_to_owner.clone(),
            permissioned_as_end_user_email: None,
        }
    }
    pub fn is_flow(&self) -> bool {
        self.kind.is_flow()
    }

    pub fn is_dependency(&self) -> bool {
        self.kind.is_dependency()
    }

    pub fn schedule_path(&self) -> Option<String> {
        schedule_path(&self.trigger_kind, &self.trigger)
    }

    pub async fn mark_as_started_if_step(&self, db: &DB) -> Result<(), Error> {
        if self.is_flow_step() {
            let _ = update_flow_status_in_progress(
                db,
                &self.workspace_id,
                self.parent_job
                    .ok_or_else(|| Error::internal_err(format!("expected parent job")))?,
                self.id,
            )
            .warn_after_seconds(5)
            .await?;
        } else if let Some(parent_job) = self.parent_job {
            let _ = update_workflow_as_code_status(db, &self.id, &parent_job).await?;
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct PulledJob {
    #[sqlx(flatten)]
    pub job: MiniPulledJob,
    pub raw_code: Option<String>,
    pub raw_lock: Option<String>,
    pub raw_flow: Option<Json<Box<RawValue>>>,
    pub parent_runnable_path: Option<String>,
    pub permissioned_as_email: Option<String>,
    pub permissioned_as_username: Option<String>,
    pub permissioned_as_is_admin: Option<bool>,
    pub permissioned_as_is_operator: Option<bool>,
    pub permissioned_as_groups: Option<Vec<String>>,
    pub permissioned_as_folders: Option<Vec<serde_json::Value>>,
}

// NOTE:
// Precomputed by the server
// Used to offload work from agent workers to server
#[derive(Debug, Serialize, Deserialize)]
pub enum PrecomputedAgentInfo {
    Bun {
        local: String,
        remote: String,
    },
    Python {
        // V1, not used anymore. Exists for compat.
        // TODO: Needs to be removed eventually
        py_version: Option<u32>,
        py_version_v2: Option<String>,
        requirements: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobAndPerms {
    pub job: MiniPulledJob,
    pub raw_code: Option<String>,
    pub raw_flow: Option<Json<Box<RawValue>>>,
    pub raw_lock: Option<String>,
    pub parent_runnable_path: Option<String>,
    pub token: String,
    pub precomputed_agent_info: Option<PrecomputedAgentInfo>,
    #[serde(skip)]
    pub flow_runners: Option<Arc<FlowRunners>>,
}
impl PulledJob {
    pub async fn get_job_and_perms(self, db: &DB) -> JobAndPerms {
        let job_perms = match (
            self.permissioned_as_email,
            self.permissioned_as_username,
            self.permissioned_as_is_admin,
            self.permissioned_as_is_operator,
            self.permissioned_as_groups,
            self.permissioned_as_folders,
        ) {
            (
                Some(email),
                Some(username),
                Some(is_admin),
                Some(is_operator),
                Some(groups),
                Some(folders),
            ) => Some(JobPerms { email, username, is_admin, is_operator, groups, folders }),
            _ => None,
        };

        let token = create_token(&db, &self.job, job_perms).await;
        JobAndPerms {
            job: self.job,
            raw_code: self.raw_code,
            raw_flow: self.raw_flow,
            raw_lock: self.raw_lock,
            parent_runnable_path: self.parent_runnable_path,
            token,
            precomputed_agent_info: None,
            flow_runners: None,
        }
    }
}

// struct Permission
pub async fn create_token(db: &DB, job: &MiniPulledJob, perms: Option<JobPerms>) -> String {
    // skipping test runs
    if job.workspace_id != "" {
        let label = if job.permissioned_as != format!("u/{}", job.created_by)
            && job.permissioned_as != job.created_by
        {
            format!("ephemeral-script-end-user-{}", job.created_by)
        } else {
            "ephemeral-script".to_string()
        };
        windmill_common::auth::create_token_for_owner(
            db,
            &job.workspace_id,
            &job.permissioned_as,
            &label,
            *SCRIPT_TOKEN_EXPIRY,
            &job.permissioned_as_email,
            &job.id,
            perms,
            Some(format!(
                "job-span-{}",
                job.flow_innermost_root_job.unwrap_or(job.id)
            )),
        )
        .warn_after_seconds(5)
        .await
        .expect("could not create job token")
    } else {
        return "".to_string();
    }
}

impl std::ops::Deref for PulledJob {
    type Target = MiniPulledJob;
    fn deref(&self) -> &Self::Target {
        &self.job
    }
}

impl std::ops::DerefMut for PulledJob {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.job
    }
}

lazy_static::lazy_static! {
    pub static ref DISABLE_CONCURRENCY_LIMIT: bool = std::env::var("DISABLE_CONCURRENCY_LIMIT").is_ok_and(|s| s == "true");
}

pub async fn get_mini_pulled_job<'c>(
    e: impl PgExecutor<'c>,
    job_id: &Uuid,
) -> windmill_common::error::Result<Option<MiniPulledJob>> {
    let job = sqlx::query_as!(
        MiniPulledJob,
        "SELECT 
        v2_job_queue.workspace_id,
        v2_job_queue.id,
        v2_job.args as \"args: sqlx::types::Json<HashMap<String, Box<RawValue>>>\",
        v2_job.parent_job,
        v2_job.created_by,
        v2_job_queue.started_at,
        v2_job_queue.runnable_settings_handle,
        scheduled_for,
        runnable_path,
        kind as \"kind: JobKind\",
        runnable_id as \"runnable_id: ScriptHash\",
        canceled_reason,
        canceled_by,
        permissioned_as,
        permissioned_as_email,
        flow_status as \"flow_status: sqlx::types::Json<Box<RawValue>>\",
        v2_job.tag,
        script_lang as \"script_lang: ScriptLang\",
        same_worker,
        pre_run_error,
        concurrent_limit,
        concurrency_time_window_s,
        flow_innermost_root_job,
        root_job,
        timeout,
        flow_step_id,
        cache_ttl,
        cache_ignore_s3_path,
        v2_job_queue.priority,
        preprocessed,
        script_entrypoint_override,
        trigger,
        trigger_kind as \"trigger_kind: JobTriggerKind\",
        visible_to_owner,
        NULL as permissioned_as_end_user_email
        FROM v2_job_queue INNER JOIN v2_job ON v2_job.id = v2_job_queue.id LEFT JOIN v2_job_status ON v2_job_status.id = v2_job_queue.id WHERE v2_job_queue.id = $1",
        job_id,
    )
    .fetch_optional(e)
    .await?;
    Ok(job)
}

pub struct QueuedJobV2 {
    pub id: Uuid,
    pub workspace_id: String,
    pub runnable_id: Option<ScriptHash>,
    pub scheduled_for: chrono::DateTime<chrono::Utc>,
    pub parent_job: Option<Uuid>,
    // pub root_job: Option<Uuid>,
    pub flow_innermost_root_job: Option<Uuid>,
    pub runnable_path: Option<String>,
    pub kind: JobKind,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub permissioned_as: String,
    pub created_by: String,
    pub script_lang: Option<ScriptLang>,
    pub permissioned_as_email: String,
    pub flow_step_id: Option<String>,
    pub trigger_kind: Option<JobTriggerKind>,
    pub trigger: Option<String>,
    pub priority: Option<i16>,
    pub concurrent_limit: Option<i32>,
    pub tag: String,
    pub cache_ttl: Option<i32>,
    pub cache_ignore_s3_path: Option<bool>,
    pub runnable_settings_handle: Option<i64>,
    pub last_ping: Option<chrono::DateTime<chrono::Utc>>,
    pub worker: Option<String>,
    pub memory_peak: Option<i32>,
    pub running: bool,
}

impl QueuedJobV2 {
    pub fn schedule_path(&self) -> Option<String> {
        schedule_path(&self.trigger_kind, &self.trigger)
    }
}

pub async fn get_queued_job_v2<'c>(
    e: impl PgExecutor<'c>,
    job_id: &Uuid,
) -> error::Result<Option<QueuedJobV2>> {
    let job = sqlx::query_as!(
        QueuedJobV2,
        r#"SELECT
                id,
                q.runnable_settings_handle,
                q.workspace_id,
                j.runnable_id as "runnable_id: ScriptHash",
                scheduled_for,
                parent_job,
                flow_innermost_root_job,
                runnable_path,
                kind as "kind: JobKind",
                started_at,
                permissioned_as,
                created_by,
                script_lang as "script_lang: ScriptLang",
                permissioned_as_email,
                flow_step_id,
                trigger_kind as "trigger_kind: JobTriggerKind",
                trigger,
                q.priority,
                concurrent_limit,
                q.tag,
                cache_ttl,
                cache_ignore_s3_path,
                r.ping as last_ping,
                worker,
                memory_peak,
                running
            FROM v2_job_queue q
                JOIN v2_job j USING (id)
                LEFT JOIN v2_job_runtime r USING (id)
                LEFT JOIN v2_job_status s USING (id)
            WHERE j.id = $1"#,
        job_id,
    )
    .fetch_optional(e)
    .await?;
    Ok(job)
}

#[derive(Debug)]
pub struct PulledJobResult {
    pub job: Option<PulledJob>,
    pub suspended: bool,
    pub missing_concurrency_key: bool,
    pub error_while_preprocessing: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum PulledJobResultToJobErr {
    #[error("missing concurrency key")]
    MissingConcurrencyKey(JobCompleted),
    #[error("pulled job preprocessor error: {}", .0.result)]
    ErrorWhilePreprocessing(JobCompleted),
}

impl PulledJobResult {
    pub fn to_pulled_job(self) -> Result<Option<PulledJob>, PulledJobResultToJobErr> {
        match self {
            PulledJobResult { job: Some(job), missing_concurrency_key: true, .. } => Err(
                PulledJobResultToJobErr::MissingConcurrencyKey(JobCompleted {
                    preprocessed_args: None,
                    job: MiniCompletedJob::from(job.job),
                    success: false,
                    result: Arc::new(windmill_common::worker::to_raw_value(&json!({
                        "name": "InternalErr",
                        "message": "The job has a concurrency limit but concurrency key couldn't be found. This is an unexpected behavior that should never happen. Please report this to support."}
                    ))),
                    result_columns: None,
                    mem_peak: 0,
                    cached_res_path: None,
                    token: "".to_string(),
                    canceled_by: None,
                    duration: None,
                    has_stream: Some(false),
                    from_cache: None,
                    flow_runners: None,
                    done_tx: None,
                }),
            ),
            PulledJobResult { job: Some(job), error_while_preprocessing: Some(e), .. } => Err(
                PulledJobResultToJobErr::ErrorWhilePreprocessing(JobCompleted {
                    preprocessed_args: None,
                    job: MiniCompletedJob::from(job.job),
                    success: false,
                    result: Arc::new(windmill_common::worker::to_raw_value(&json!({
                        "name": "Pulled job preprocessing error",
                        "message": e
                    }))),
                    result_columns: None,
                    mem_peak: 0,
                    cached_res_path: None,
                    token: "".to_string(),
                    canceled_by: None,
                    duration: None,
                    has_stream: Some(false),
                    from_cache: None,
                    flow_runners: None,
                    done_tx: None,
                }),
            ),
            PulledJobResult { job, .. } => Ok(job),
        }
    }

    /// Generic preprocess function
    /// Can be used for any kind of preprocessing
    pub async fn maybe_apply_debouncing(&mut self, db: &DB) -> error::Result<()> {
        let PulledJobResult { job: Some(ref mut j), .. } = self else {
            return Ok(());
        };

        let DebouncingSettings { debounce_delay_s, debounce_args_to_accumulate, .. } =
            windmill_common::runnable_settings::prefetch_cached_from_handle(
                j.runnable_settings_handle,
                db,
            )
            .await?
            .0;

        let (kind, j_id) = (j.kind, j.id);
        let is_djob_to_debounce = kind.is_dependency()
            && j.args
                .as_ref()
                .and_then(|x| x.get("triggered_by_relative_import"))
                .is_some();

        if (is_djob_to_debounce || debounce_delay_s.filter(|x| *x > 0).is_some())
            && MIN_VERSION_SUPPORTS_DEBOUNCING.met().await
            && !*WMDEBUG_NO_DEBOUNCING
        {
            let needs_debounce = sqlx::query_scalar!(
                "WITH _ AS (
                    DELETE FROM debounce_key WHERE job_id = $1
                ) SELECT status = 'skipped' FROM v2_job_completed WHERE id = $1",
                j_id,
            )
            .fetch_optional(db)
            .await?
            .flatten()
            .unwrap_or_default();

            if needs_debounce {
                tracing::info!(
                    job_id = %j_id,
                    "Late debounce: job was already debounced by another job, skipping execution"
                );
                self.job = None;
                return Ok(());
            }

            if matches!(kind, JobKind::FlowDependencies | JobKind::AppDependencies)
                && !MIN_VERSION_SUPPORTS_DEBOUNCING_V2.met().await
                && !*WMDEBUG_FORCE_NO_LEGACY_DEBOUNCING_COMPAT
            {
                // Simply disable optimization for apps and flows if min version doesn't support debouncing v2
                let field_name = match kind {
                    JobKind::FlowDependencies => "nodes_to_relock",
                    JobKind::AppDependencies => "components_to_relock",
                    _ => unreachable!(),
                };

                tracing::debug!(
                    job_id = %j_id,
                    job_kind = ?kind,
                    field_removed = field_name,
                    "Removing optimization field: workers behind v2 debouncing version, disabling relock optimization"
                );

                if let Some(args) = &mut j.args {
                    args.remove(field_name);
                }
            } else if let Some(arg_name_to_accumulate) =
                // TODO: Maybe support multiple arguments in future
                debounce_args_to_accumulate.as_ref().and_then(|v| v.get(0))
            {
                tracing::debug!(
                    job_id = %j_id,
                    job_kind = ?kind,
                    arg_name = arg_name_to_accumulate,
                    "Accumulating debounced arguments from batch"
                );
                let mut accumulated_arg: Vec<Box<RawValue>> = vec![];
                for str_o in sqlx::query_scalar!(
                    "WITH ids AS (
                        SELECT id as job_id FROM v2_job_debounce_batch WHERE debounce_batch = (
                            SELECT debounce_batch FROM v2_job_debounce_batch WHERE id = $1
                        )
                    ) SELECT args->>$2 FROM ids LEFT JOIN v2_job ON v2_job.id = ids.job_id
                    ",
                    j_id,
                    arg_name_to_accumulate,
                )
                .fetch_all(db)
                .await?
                .into_iter()
                {
                    if let Some(s) = str_o.as_ref() {
                        match serde_json::from_str::<Vec<Box<RawValue>>>(s) {
                            Ok(ref mut vec) => accumulated_arg.append(vec),
                            Err(e) => {
                                return Err(error::Error::ArgumentErr(format!("cannot consolidate arguments of non-list type. Type provided for argument `{arg_name_to_accumulate}` is not a list\nUnwrapped Error: {e}")));
                            }
                        }
                    }
                }

                tracing::debug!(
                    job_id = %j_id,
                    arg_name = arg_name_to_accumulate,
                    accumulated_count = accumulated_arg.len(),
                    "Accumulated arguments from debounced jobs in batch"
                );

                let new_value = to_raw_value(&accumulated_arg);

                append_logs(
                    &j_id,
                    &j.workspace_id,
                    format!(
                        "Substituting `{arg_name_to_accumulate}` with: {}\n\n",
                        &new_value
                    ),
                    &(db.into()),
                )
                .await;

                j.args
                    .get_or_insert(Json(Default::default()))
                    .as_mut()
                    .insert(arg_name_to_accumulate.to_owned(), new_value);
            }

            // Handle dependency job debouncing cleanup when a job is pulled for execution
            if is_djob_to_debounce {
                clone_runnable(j, db).await?;
            }
        }

        Ok(())
    }
}

async fn clone_runnable(j: &mut PulledJob, db: &DB) -> error::Result<()> {
    let Some(base_hash) = j.runnable_id else {
        return Err(Error::InternalErr(
            "Missing runnable_id for dependency job triggered by relative import".to_string(),
        ));
    };

    tracing::debug!(
        job_id = %j.id,
        base_hash = %base_hash,
        job_kind = ?j.kind,
        runnable_path = ?j.runnable_path,
        "Creating new version for dependency job triggered by relative import"
    );

    {
        let maybe_new_id = match j.kind {
            JobKind::Dependencies => {
                let deployment_message = j
                    .args
                    .clone()
                    .map(|hashmap| {
                        hashmap
                            .get("deployment_message")
                            .map(|map_value| serde_json::from_str::<String>(map_value.get()).ok())
                            .flatten()
                    })
                    .flatten();

                // This way we tell downstream which script we should archive when the resolution is finished.
                // (not used at the moment)
                j.args
                    .as_mut()
                    .map(|args| args.insert("base_hash".to_owned(), to_raw_value(&*base_hash)));

                windmill_common::scripts::clone_script(
                    j.runnable_path(),
                    &j.workspace_id,
                    deployment_message,
                    db,
                )
                .await?
                .new_hash
            }
            JobKind::FlowDependencies => {
                sqlx::query_scalar!(
                    "INSERT INTO flow_version
                    (workspace_id, path, value, schema, created_by)

                    SELECT workspace_id, path, value, schema, created_by
                    FROM flow_version WHERE path = $1 AND workspace_id = $2 AND id = $3

                    RETURNING id
                    ",
                    j.runnable_path(),
                    j.workspace_id,
                    *base_hash,
                )
                .fetch_one(db)
                .await?
            }
            JobKind::AppDependencies => {
                sqlx::query_scalar!(
                    "INSERT INTO app_version
                        (app_id, value, created_by, raw_app)
                    SELECT app_id, value, created_by, raw_app
                    FROM app_version WHERE id = $1
                    RETURNING id",
                    *base_hash
                )
                .fetch_one(db)
                .await?
            }
            _ => *base_hash,
        };

        j.runnable_id.replace(maybe_new_id.into());
    }

    Ok(())
}

// TODO: Factorize
/// Pull the job from queue
pub async fn pull(
    db: &Pool<Postgres>,
    // Whether or not try to pull from suspended jobs first
    suspend_first: bool,
    worker_name: &str,
    // Execute queries supplied by caller instead of generic one
    query_o: Option<&(String, String)>,
    #[cfg(feature = "benchmark")] bench: &mut BenchmarkIter,
) -> windmill_common::error::Result<PulledJobResult> {
    let mut pull_loop_count = 0;
    loop {
        pull_loop_count += 1;
        if pull_loop_count % 10 == 0 {
            tracing::warn!("Pull job loop count: {}", pull_loop_count);
            tokio::task::yield_now().await;
        }
        if pull_loop_count > 1000 {
            tracing::error!("Pull job loop count exceeded 1000, breaking");
            return Ok(PulledJobResult {
                job: None,
                suspended: false,
                missing_concurrency_key: false,
                error_while_preprocessing: None,
            });
        }

        if let Some((query_suspended, query_no_suspend)) = query_o {
            let njob = {
                let job = if query_suspended.is_empty() {
                    None
                } else {
                    timeout(
                        Duration::from_secs(15),
                        sqlx::query_as::<_, PulledJob>(query_suspended)
                            .bind(worker_name)
                            .fetch_optional(db),
                    )
                    .await??
                };

                let (job, suspended) = if let Some(job) = job {
                    (Some(job), true)
                } else {
                    let job = timeout(
                        Duration::from_secs(15),
                        sqlx::query_as::<_, PulledJob>(query_no_suspend)
                            .bind(worker_name)
                            .fetch_optional(db),
                    )
                    .await??;

                    (job, false)
                };

                if let Some(job) = job.as_ref() {
                    if job.is_flow() || job.is_dependency() {
                        let per_workspace = per_workspace_tag(&job.workspace_id).await;
                        let base_tag = if job.is_flow() {
                            "flow".to_string()
                        } else {
                            "dependency".to_string()
                        };
                        let tag = if per_workspace {
                            format!("{}-{}", base_tag, job.workspace_id)
                        } else {
                            base_tag
                        };
                        sqlx::query!(
                            "UPDATE v2_job_queue SET tag = $1, running = false WHERE id = $2",
                            tag,
                            job.id
                        )
                        .execute(db)
                        .await?;
                        continue;
                    }
                }

                #[cfg(feature = "private")]
                let concurrency_settings = if let Some(ref j) = job {
                    windmill_common::runnable_settings::prefetch_cached_from_handle(
                        j.runnable_settings_handle,
                        db,
                    )
                    .await?
                    .1
                    .maybe_fallback(
                        None,
                        j.concurrent_limit,
                        j.concurrency_time_window_s,
                    )
                } else {
                    Default::default()
                };

                let pulled_job_result = match job {
                    #[cfg(feature = "private")]
                    Some(job)
                        if concurrency_settings.concurrent_limit.is_some()
                            // Concurrency limit is available for either enterprise job or dependency job
                            && (cfg!(feature = "enterprise") || (job.is_dependency() && !*WMDEBUG_NO_DEBOUNCING)) =>
                    {
                        timeout(
                            Duration::from_secs(15),
                            crate::jobs_ee::apply_concurrency_limit(
                                db,
                                pull_loop_count,
                                suspended,
                                job,
                                &concurrency_settings,
                            ),
                        )
                        .await??
                        .unwrap_or(PulledJobResult {
                            job: None,
                            suspended,
                            missing_concurrency_key: false,
                            error_while_preprocessing: None,
                        })
                    }
                    _ => PulledJobResult {
                        job,
                        suspended,
                        missing_concurrency_key: false,
                        error_while_preprocessing: None,
                    },
                };

                Ok::<_, Error>(pulled_job_result)
            }?;

            return Ok(njob);
        };

        let (job, suspended) = timeout(
            Duration::from_secs(15),
            pull_single_job_and_mark_as_running_no_concurrency_limit(
                db,
                suspend_first,
                worker_name,
                #[cfg(feature = "benchmark")]
                bench,
            ),
        )
        .await??;
        let Some(job) = job else {
            return Ok(PulledJobResult {
                job: None,
                suspended,
                missing_concurrency_key: false,
                error_while_preprocessing: None,
            });
        };

        let concurrency_settings = windmill_common::runnable_settings::prefetch_cached_from_handle(
            job.runnable_settings_handle,
            db,
        )
        .await?
        .1
        .maybe_fallback(None, job.concurrent_limit, job.concurrency_time_window_s);

        let has_concurent_limit = concurrency_settings.concurrent_limit.is_some();

        #[cfg(not(feature = "enterprise"))]
        if has_concurent_limit && !job.is_dependency() {
            tracing::error!("Concurrent limits are an EE feature only, ignoring constraints")
        }

        #[cfg(not(feature = "enterprise"))]
        let has_concurent_limit = job.is_dependency()
            && job.concurrent_limit.is_some()
            && cfg!(feature = "private")
            && !*WMDEBUG_NO_DEBOUNCING;
        // if we don't have private flag, we don't have concurrency limit

        // concurrency check. If more than X jobs for this path are already running, we re-queue and pull another job from the queue
        let pulled_job = job;
        if pulled_job.runnable_path.is_none()
            || !has_concurent_limit
            || pulled_job.canceled_by.is_some()
        {
            #[cfg(feature = "prometheus")]
            if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                QUEUE_PULL_COUNT.inc();
            }
            return Ok(PulledJobResult {
                job: Some(pulled_job),
                suspended,
                missing_concurrency_key: false,
                error_while_preprocessing: None,
            });
        }

        #[cfg(feature = "private")]
        if cfg!(feature = "enterprise") || (pulled_job.is_dependency() && !*WMDEBUG_NO_DEBOUNCING) {
            if let Some(pulled_job_res) = timeout(
                Duration::from_secs(15),
                crate::jobs_ee::apply_concurrency_limit(
                    db,
                    pull_loop_count,
                    suspended,
                    pulled_job,
                    &concurrency_settings,
                ),
            )
            .await??
            {
                return Ok(pulled_job_res);
            }
        }
    }
}

async fn pull_single_job_and_mark_as_running_no_concurrency_limit<'c>(
    db: &Pool<Postgres>,
    suspend_first: bool,
    worker_name: &str,
    #[cfg(feature = "benchmark")] bench: &mut BenchmarkIter,
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
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
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
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                return Ok((None, false));
            }

            for query in queries.iter() {
                // tracing::info!("Pulling job with query: {}", query);
                // let instant = std::time::Instant::now();

                #[cfg(feature = "benchmark")]
                add_time!(bench, "pre pull");

                let r = sqlx::query_as::<_, PulledJob>(query)
                    .bind(worker_name)
                    .fetch_optional(db)
                    .await?;

                #[cfg(feature = "benchmark")]
                add_time!(bench, "post pull");

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
    job_id: &Uuid,
) -> Result<Option<String>, sqlx::Error> {
    let fut = async || {
        sqlx::query_scalar!("SELECT key FROM concurrency_key WHERE job_id = $1", job_id)
            .fetch_optional(db) // this should no longer be fetch optional
            .await
    };
    fut.retry(
        ConstantBuilder::default()
            .with_delay(std::time::Duration::from_secs(3))
            .with_max_times(5)
            .build(),
    )
    .notify(|err, dur| {
        tracing::error!(
            "Could not get concurrency key for job {job_id}, retrying in {dur:#?}, err: {err:#?}"
        );
    })
    .await
}

pub async fn concurrency_key(
    db: &Pool<Postgres>,
    id: &Uuid,
) -> windmill_common::error::Result<Option<String>> {
    custom_concurrency_key(db, id)
        .await
        .map(|x| {
            if x.is_none() {
                tracing::info!("No concurrency key found for job {id}");
            }
            return x;
        })
        .map_err(|e| {
            Error::internal_err(format!("Could not get concurrency key for job {id}: {e:#}"))
        })
}

pub async fn custom_debounce_key(
    db: &Pool<Postgres>,
    job_id: &Uuid,
) -> Result<Option<String>, sqlx::Error> {
    let fut = async || {
        sqlx::query_scalar!("SELECT key FROM debounce_key WHERE job_id = $1", job_id)
            .fetch_optional(db) // this should no longer be fetch optional
            .await
    };
    fut.retry(
        ConstantBuilder::default()
            .with_delay(std::time::Duration::from_secs(3))
            .with_max_times(5)
            .build(),
    )
    .notify(|err, dur| {
        tracing::error!(
            "Could not get debounce key for job {job_id}, retrying in {dur:#?}, err: {err:#?}"
        );
    })
    .await
}

pub fn resolve_debounce_key<'b>(
    unresolved_debounce_key: Option<String>,
    runnable_path: &Option<String>,
    workspace_id: &str,
    job_kind: JobKind,
    args: &PushArgs<'b>,
    args_to_ignore_if_default: Option<&String>,
) -> String {
    let original_debounce_key = unresolved_debounce_key
        .map(|x| crate::interpolate_args(x, &args, workspace_id))
        .unwrap_or(format!(
            "{}#args:{}",
            crate::fullpath_with_workspace(workspace_id, runnable_path.as_ref(), &job_kind),
            args.args
                .iter()
                .filter_map(|(k, v)| {
                    if args_to_ignore_if_default
                        .map(|name| name == k)
                        .unwrap_or_default()
                    {
                        None
                    } else {
                        Some(v.to_string())
                    }
                })
                // TODO: disable sorted?
                .sorted()
                .collect_vec()
                .join(":"),
        ));

    tracing::debug!("Original debounce key: {}", original_debounce_key);

    // If debounce_key is not too long (< 255 chars), keep it as is, otherwise hash it
    const MAX_DEBOUNCE_KEY_LENGTH: usize = 255;
    let resolved = if original_debounce_key.len() <= MAX_DEBOUNCE_KEY_LENGTH {
        original_debounce_key
    } else {
        let hash = calculate_hash(&original_debounce_key);
        tracing::debug!(
            "Debounce key too long ({}), using hash: {}",
            original_debounce_key.len(),
            hash
        );
        hash
    };

    #[cfg(feature = "cloud")]
    let resolved = format!("{workspace_id}:{resolved}");

    tracing::debug!("Final debounce key: {}", resolved);
    resolved
}

#[derive(Debug)]
pub enum DebouncingLimitsReport {
    MaxCountExceeded,
    TimeExceeded,
    CanDebounce,
}

pub async fn check_debouncing_within_limits(
    (current_time, current_amount): (DateTime<Utc>, i32),
    (allowed_time, allowed_amount): (Option<i32>, Option<i32>),
    job_id: &Uuid,
    runnable_path: &Option<String>,
) -> DebouncingLimitsReport {
    use DebouncingLimitsReport::*;

    let no_legacy_compat = MIN_VERSION_SUPPORTS_DEBOUNCING_V2.met().await
        || *WMDEBUG_FORCE_NO_LEGACY_DEBOUNCING_COMPAT;

    if !no_legacy_compat {
        tracing::warn!(
            job_id = %job_id,
            runnable_path = ?runnable_path,
            allowed_time = ?allowed_time,
            allowed_amount = ?allowed_amount,
            "Debouncing limits not enforced: workers are behind minimum version for v2 debouncing (require >= 1.597.0)"
        );
    }

    let elapsed_seconds = (Utc::now() - current_time).num_seconds() as i32;

    tracing::debug!(
        job_id = %job_id,
        runnable_path = ?runnable_path,
        current_amount = current_amount,
        allowed_amount = ?allowed_amount,
        elapsed_seconds = elapsed_seconds,
        allowed_time = ?allowed_time,
        first_started_at = %current_time,
        no_legacy_compat = no_legacy_compat,
        "Checking debouncing limits"
    );

    if allowed_amount
        .map(|allowed_amount| current_amount > allowed_amount)
        .unwrap_or_default()
        && no_legacy_compat
    {
        tracing::info!(
            job_id = %job_id,
            runnable_path = ?runnable_path,
            current_amount = current_amount,
            allowed_amount = allowed_amount,
            "Debouncing limit exceeded: max debounce count reached"
        );
        MaxCountExceeded
    } else if allowed_time
        .map(|allowed_time| elapsed_seconds > allowed_time)
        .unwrap_or_default()
        && no_legacy_compat
    {
        tracing::info!(
            job_id = %job_id,
            runnable_path = ?runnable_path,
            elapsed_seconds = elapsed_seconds,
            allowed_time = allowed_time,
            first_started_at = %current_time,
            "Debouncing limit exceeded: max debounce time window reached"
        );
        TimeExceeded
    } else {
        tracing::debug!(
            job_id = %job_id,
            runnable_path = ?runnable_path,
            current_amount = current_amount,
            elapsed_seconds = elapsed_seconds,
            "Debouncing within limits: can continue debouncing"
        );
        CanDebounce
    }
}

pub fn interpolate_args(x: String, args: &PushArgs, workspace_id: &str) -> String {
    // Save this value to avoid parsing twice
    let workspaced = x.as_str().replace("$workspace", workspace_id).to_string();
    if RE_ARG_TAG.is_match(&workspaced) {
        let mut interpolated = workspaced.clone();
        for cap in RE_ARG_TAG.captures_iter(&workspaced) {
            let arg_name = cap.get(1).unwrap().as_str();
            let arg_value = if arg_name.contains('.') {
                let parts: Vec<&str> = arg_name.split('.').collect();
                let root = parts[0];
                let mut value = args
                    .args
                    .get(root)
                    .or(args.extra.as_ref().and_then(|x| x.get(root)))
                    .map(|x| x.get())
                    .unwrap_or_default()
                    .to_string();

                for part in parts.iter().skip(1) {
                    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&value) {
                        value = obj
                            .get(part)
                            .and_then(|v| Some(v.to_string()))
                            .unwrap_or_default()
                            .as_str()
                            .to_string();
                    } else {
                        value = "".to_string(); // Invalid JSON or missing field
                        break;
                    }
                }
                value.trim_matches('"').to_string()
            } else {
                args.args
                    .get(arg_name)
                    .or(args.extra.as_ref().and_then(|x| x.get(arg_name)))
                    .map(|x| x.get())
                    .unwrap_or_default()
                    .trim_matches('"')
                    .to_string()
            };
            interpolated =
                interpolated.replace(format!("$args[{}]", arg_name).as_str(), &arg_value);
        }
        interpolated
    } else {
        workspaced
    }
}

pub fn fullpath_with_workspace(
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
        Err(e) => {
            let root = sqlx::query!(
                "SELECT
                    id As \"id!\",
                    flow_status->'restarted_from'->'flow_job_id' AS \"restarted_from: Json<Uuid>\"
                FROM v2_job_status
                WHERE COALESCE((SELECT flow_innermost_root_job FROM v2_job WHERE id = $1), $1) = id",
                flow_id
            )
            .fetch_optional(&db)
            .await?;
            match root {
                Some(root) => {
                    let restarted_from_id = not_found_if_none(
                        root.restarted_from,
                        "Id not found in the result's mapping of the root job and root job had no restarted from information",
                        format!("parent: {}, root: {}, id: {}, error: {e:#}", flow_id, root.id, node_id),
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
                "SELECT status = 'success' OR status = 'skipped' AS \"success!\"
                FROM v2_job_completed WHERE id = $1 AND workspace_id = $2",
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
                    WHERE id = $1 
                )
                SELECT module->>'type' = 'Success'
                FROM modules
                WHERE module->>'id' = $2"#,
                if completed {
                    "v2_job_completed"
                } else {
                    "v2_job_status"
                }
            );
            sqlx::query_scalar(&query)
                .bind(flow_id)
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
        "SELECT flow_leaf_jobs->$1::text AS \"leaf_jobs: Json<Box<RawValue>>\", v2_job.parent_job
        FROM v2_job_status
        LEFT JOIN v2_job ON v2_job.id = v2_job_status.id AND v2_job.workspace_id = $3
        WHERE COALESCE((SELECT flow_innermost_root_job FROM v2_job WHERE id = $2), $2) = v2_job_status.id",
        node_id,
        flow_id,
        w_id,
    )
    .fetch_optional(db)
    .await?;

    // tracing::error!("flow_job_result: {:?} {:?}", flow_job_result, flow_id);
    let flow_job_result = windmill_common::utils::not_found_if_none(
        flow_job_result,
        "Root job of parent runnnig flow",
        format!("parent: {}, id: {}", flow_id, node_id),
    )?;
    // tracing::error!("flow_job_result: {:?}, {:?}", flow_job_result.leaf_jobs, flow_job_result.parent_job);

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
    created_at: DateTime<Utc>,
    subflows: Vec<(Uuid, FlowStatus)>,
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
                "SELECT j.id, jc.flow_status AS \"flow_status!: Json<FlowStatus>\"
                FROM v2_job j
                JOIN v2_job_completed jc ON j.id = jc.id
                WHERE j.parent_job = $1 AND j.workspace_id = $2 AND j.created_at >= $3 AND jc.flow_status IS NOT NULL",
                id,
                w_id,
                created_at
            )
            .map(|record| (record.id, record.flow_status.0))
            .fetch_all(db)
            .await?;
            match Box::pin(get_completed_flow_node_result_rec(
                db, w_id, created_at, subflows, node_id,
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
        "SELECT jc.id, jc.flow_status AS \"flow_status!: Json<FlowStatus>\", j.created_at
        FROM v2_job_completed jc
        JOIN v2_job j ON j.id = jc.id
        WHERE jc.id = $1 AND jc.workspace_id = $2 AND jc.flow_status IS NOT NULL",
        completed_flow_id,
        w_id
    )
    .map(|record| (record.id, record.flow_status.0, record.created_at))
    .fetch_optional(db)
    .await?;

    let (id, flow_status, created_at) = not_found_if_none(
        flow_job,
        "Root completed job",
        format!("root: {}, id: {}", completed_flow_id, node_id),
    )?;

    match get_completed_flow_node_result_rec(db, w_id, created_at, vec![(id, flow_status)], node_id)
        .await?
    {
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

pub fn get_mini_completed_job<'a, 'e, A: sqlx::Acquire<'e, Database = Postgres> + Send + 'a>(
    id: &'a Uuid,
    w_id: &'a str,
    db: A,
) -> impl Future<Output = error::Result<Option<MiniCompletedJob>>> + Send + 'a {
    async move {
        let mut conn = db.acquire().await?;
        sqlx::query_as!(
            MiniCompletedJob,
            "SELECT 
            j.id, j.workspace_id, j.runnable_id AS \"runnable_id: ScriptHash\", q.scheduled_for, q.started_at, j.parent_job, j.flow_innermost_root_job, j.runnable_path, j.kind as \"kind!: JobKind\", j.permissioned_as, 
            j.created_by, j.script_lang AS \"script_lang: ScriptLang\", j.permissioned_as_email, j.flow_step_id, j.trigger_kind AS \"trigger_kind: JobTriggerKind\", j.trigger, j.priority, j.concurrent_limit, j.tag, j.cache_ttl, q.cache_ignore_s3_path, q.runnable_settings_handle
            FROM v2_job j LEFT JOIN v2_job_queue q ON j.id = q.id
            WHERE j.id = $1 AND j.workspace_id = $2",
            id,
            w_id
        )
        .fetch_optional(&mut *conn)
        .await
        .map_err(Into::into)
    }
}

pub enum PushIsolationLevel<'c> {
    IsolatedRoot(DB),
    Isolated(UserDB, Authed),
    Transaction(Transaction<'c, Postgres>),
}

impl<'c> PushIsolationLevel<'c> {
    pub async fn into_tx(self) -> error::Result<Transaction<'c, Postgres>> {
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

#[derive(Debug, Clone, Default)]
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

impl<'c> From<&PushArgs<'c>> for HashMap<String, Box<RawValue>> {
    fn from(args: &PushArgs<'c>) -> Self {
        let mut map = HashMap::new();
        map.extend(args.args.clone().into_iter());
        if let Some(ref extra) = args.extra {
            map.extend(extra.clone().into_iter());
        }
        map
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
    pub static ref RE_ARG_TAG: Regex = Regex::new(r#"\$args\[((?:\w+\.)*\w+)\]"#).unwrap();
}

#[cfg(feature = "cloud")]
lazy_static::lazy_static! {
    // Cache for superadmin status: email -> (is_super_admin, expiry_timestamp)
    static ref SUPERADMIN_CACHE: Arc<RwLock<HashMap<String, (bool, std::time::Instant)>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

#[cfg(feature = "cloud")]
const SUPERADMIN_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(60);

#[cfg(feature = "cloud")]
async fn is_superadmin_cached(db: &Pool<Postgres>, email: &str) -> Result<bool, Error> {
    let now = std::time::Instant::now();

    // Try to get from cache first
    {
        let cache = SUPERADMIN_CACHE.read().await;
        if let Some((is_super_admin, expiry)) = cache.get(email) {
            if *expiry > now {
                return Ok(*is_super_admin);
            }
        }
    }

    // Cache miss or expired, fetch from database
    let is_super_admin =
        sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
            .fetch_optional(db)
            .await?
            .unwrap_or(false);

    // Update cache
    {
        let mut cache = SUPERADMIN_CACHE.write().await;
        cache.insert(
            email.to_string(),
            (is_super_admin, now + SUPERADMIN_CACHE_TTL),
        );
    }

    Ok(is_super_admin)
}

#[cfg(feature = "cloud")]
async fn check_usage_limits(
    db: &Pool<Postgres>,
    workspace_id: &str,
    email: &str,
    check_user_usage: bool,
) -> Result<(i32, Option<i32>), Error> {
    // Get current workspace usage with a simple SELECT (no row lock)
    let workspace_usage = sqlx::query_scalar!(
        "SELECT usage FROM usage
        WHERE id = $1
        AND is_workspace = TRUE
        AND month_ = EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date)",
        workspace_id
    )
    .fetch_optional(db)
    .await
    .map_err(|e| Error::internal_err(format!("fetching workspace usage: {e:#}")))?
    .unwrap_or(0);

    // Get current user usage (only for non-premium workspaces)
    let user_usage = if check_user_usage {
        sqlx::query_scalar!(
            "SELECT usage FROM usage
            WHERE id = $1
            AND is_workspace = FALSE
            AND month_ = EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date)",
            email
        )
        .fetch_optional(db)
        .await
        .map_err(|e| Error::internal_err(format!("fetching user usage: {e:#}")))?
    } else {
        None
    };

    Ok((workspace_usage, user_usage))
}

#[cfg(feature = "cloud")]
use crate::cloud_usage::increment_usage_async;

// Thin wrapper that boxes the future to reduce async state machine sizes in callers.
// Without this, the ~13KB future of push_inner is inlined into every caller's state machine,
// causing stack overflows in deeply nested async call chains (e.g. flow execution).
pub async fn push<'c, 'd>(
    _db: &Pool<Postgres>,
    tx: PushIsolationLevel<'c>,
    workspace_id: &str,
    job_payload: JobPayload,
    args: PushArgs<'d>,
    user: &str,
    email: &str,
    permissioned_as: String,
    token_prefix: Option<&str>,
    scheduled_for_o: Option<chrono::DateTime<chrono::Utc>>,
    schedule_path: Option<String>,
    parent_job: Option<Uuid>,
    root_job: Option<Uuid>,
    flow_innermost_root_job: Option<Uuid>,
    job_id: Option<Uuid>,
    _is_flow_step: bool,
    same_worker: bool,
    pre_run_error: Option<&windmill_common::error::Error>,
    visible_to_owner: bool,
    tag: Option<String>,
    custom_timeout: Option<i32>,
    flow_step_id: Option<String>,
    _priority_override: Option<i16>,
    authed: Option<&Authed>,
    running: bool,
    end_user_email: Option<String>,
    trigger: Option<TriggerMetadata>,
    suspended_mode: Option<bool>,
) -> Result<(Uuid, Transaction<'c, Postgres>), Error> {
    Box::pin(push_inner(
        _db,
        tx,
        workspace_id,
        job_payload,
        args,
        user,
        email,
        permissioned_as,
        token_prefix,
        scheduled_for_o,
        schedule_path,
        parent_job,
        root_job,
        flow_innermost_root_job,
        job_id,
        _is_flow_step,
        same_worker,
        pre_run_error,
        visible_to_owner,
        tag,
        custom_timeout,
        flow_step_id,
        _priority_override,
        authed,
        running,
        end_user_email,
        trigger,
        suspended_mode,
    ))
    .await
}

// #[instrument(level = "trace", skip_all)]
async fn push_inner<'c, 'd>(
    _db: &Pool<Postgres>,
    mut tx: PushIsolationLevel<'c>,
    workspace_id: &str,
    job_payload: JobPayload,
    args: PushArgs<'d>,
    user: &str,
    mut email: &str,
    mut permissioned_as: String,
    token_prefix: Option<&str>,
    #[allow(unused_mut)] mut scheduled_for_o: Option<chrono::DateTime<chrono::Utc>>,
    schedule_path: Option<String>, //should be removed in favor of the trigger param below
    parent_job: Option<Uuid>,
    root_job: Option<Uuid>,
    flow_innermost_root_job: Option<Uuid>,
    job_id: Option<Uuid>,
    _is_flow_step: bool,
    mut same_worker: bool, // whether the job will be executed on the same worker: if true, the job will be set to running but started_at will not be set.
    pre_run_error: Option<&windmill_common::error::Error>,
    visible_to_owner: bool,
    mut tag: Option<String>,
    custom_timeout: Option<i32>,
    flow_step_id: Option<String>,
    _priority_override: Option<i16>,
    authed: Option<&Authed>,
    running: bool, // whether the job is already running: only set this to true if you don't want the job to be picked up by a worker from the queue. It will also set started_at to now.
    end_user_email: Option<String>,
    trigger: Option<TriggerMetadata>,
    suspended_mode: Option<bool>,
) -> Result<(Uuid, Transaction<'c, Postgres>), Error> {
    #[cfg(feature = "cloud")]
    if *CLOUD_HOSTED {
        let team_plan_status =
            windmill_common::workspaces::get_team_plan_status(_db, workspace_id).await?;
        // we track only non flow steps
        let (workspace_usage, user_usage) = if !matches!(
            job_payload,
            JobPayload::Flow { .. } | JobPayload::RawFlow { .. }
        ) {
            // Check current usage with SELECT (fast, no row locks)
            // Only check user usage for non-premium workspaces
            let (current_workspace_usage, current_user_usage) =
                check_usage_limits(_db, workspace_id, email, !team_plan_status.premium).await?;

            // Spawn async task to update usage counters in the background
            increment_usage_async(
                _db.clone(),
                workspace_id.to_string(),
                if !team_plan_status.premium {
                    Some(email.to_string())
                } else {
                    None
                },
            );

            // Return the current usage + 1 to account for this job
            let workspace_usage_with_new_job = current_workspace_usage + 1;
            let user_usage_with_new_job = if !team_plan_status.premium {
                Some(current_user_usage.unwrap_or(0) + 1)
            } else {
                None
            };

            (Some(workspace_usage_with_new_job), user_usage_with_new_job)
        } else {
            (None, None)
        };

        if !team_plan_status.premium || team_plan_status.is_past_due {
            let is_super_admin = is_superadmin_cached(_db, email).await?;

            #[cfg(feature = "private")]
            let recovery_email = crate::jobs_ee::SCHEDULE_RECOVERY_HANDLER_USER_EMAIL;
            #[cfg(not(feature = "private"))]
            let recovery_email = "recovery@windmill.dev";

            if !is_super_admin {
                if !team_plan_status.premium
                    && email != ERROR_HANDLER_USER_EMAIL
                    && email != SCHEDULE_ERROR_HANDLER_USER_EMAIL
                    && email != recovery_email
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
                        "SELECT COUNT(j.id) FROM v2_job_queue q JOIN v2_job j USING (id) WHERE j.permissioned_as_email = $1",
                        email
                    )
                    .fetch_one(_db)
                    .await?
                    .unwrap_or(0);

                    if in_queue > MAX_FREE_EXECS as i64 {
                        return Err(error::Error::QuotaExceeded(format!(
                            "User {email} has exceeded the jobs in queue limit of {MAX_FREE_EXECS} that applies outside of premium workspaces."
                        )));
                    }

                    let concurrent_runs = sqlx::query_scalar!(
                        "SELECT COUNT(j.id) FROM v2_job_queue q JOIN v2_job j USING (id) WHERE q.running = true AND j.permissioned_as_email = $1",
                        email
                    )
                    .fetch_one(_db)
                    .await?
                    .unwrap_or(0);

                    if concurrent_runs > MAX_FREE_CONCURRENT_RUNS as i64 {
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
                    if team_plan_status.premium {
                        // team plan is premium but past due, we check if the workspace has exceeded the max tolerated executions
                        if team_plan_status.max_tolerated_executions.is_none()
                            || workspace_usage > team_plan_status.max_tolerated_executions.unwrap()
                        {
                            return Err(error::Error::QuotaExceeded(format!(
                                "Workspace {workspace_id} team plan is past due and isn't allowed to run any more jobs. Please fix your payment method in the workspace settings."
                            )));
                        }
                    } else {
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

                        if in_queue_workspace > MAX_FREE_EXECS as i64 {
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

                        if concurrent_runs_workspace > MAX_FREE_CONCURRENT_RUNS as i64 {
                            return Err(error::Error::QuotaExceeded(format!(
                                "Workspace {workspace_id} has exceeded the concurrent runs limit of {MAX_FREE_CONCURRENT_RUNS} that applies outside of premium workspaces."
                            )));
                        }
                    }
                }
            }
        }
    }

    #[derive(Default)]
    struct JobPayloadUntagged {
        runnable_id: Option<i64>,
        runnable_path: Option<String>,
        raw_code_tuple: Option<(String, Option<String>)>, // (content, lock)
        job_kind: JobKind,
        raw_flow: Option<FlowValue>,
        flow_status: Option<FlowStatus>,
        language: Option<ScriptLang>,
        cache_ttl: Option<i32>,
        cache_ignore_s3_path: Option<bool>,
        dedicated_worker: Option<bool>,
        _low_level_priority: Option<i16>,
        concurrency_settings: ConcurrencySettings,
        debouncing_settings: DebouncingSettings,
    }
    let mut preprocessed = None;
    #[allow(unused)]
    let JobPayloadUntagged {
        runnable_id,
        runnable_path,
        raw_code_tuple,
        mut job_kind,
        raw_flow,
        flow_status,
        language,
        cache_ttl,
        cache_ignore_s3_path,
        dedicated_worker,
        _low_level_priority,
        mut concurrency_settings,
        debouncing_settings,
    } = match job_payload {
        JobPayload::ScriptHash {
            hash,
            path,
            cache_ttl,
            cache_ignore_s3_path,
            language,
            dedicated_worker,
            priority,
            apply_preprocessor,
            concurrency_settings,
            debouncing_settings,
        } => {
            if apply_preprocessor {
                preprocessed = Some(false);
            }

            JobPayloadUntagged {
                runnable_id: Some(hash.0),
                runnable_path: Some(path),
                job_kind: JobKind::Script,
                language: Some(language),
                concurrency_settings,
                debouncing_settings,
                cache_ttl,
                cache_ignore_s3_path,
                dedicated_worker,
                _low_level_priority: priority,
                ..Default::default()
            }
        }
        JobPayload::FlowScript {
            id, // flow_node(id).
            language,
            cache_ttl,
            cache_ignore_s3_path,
            dedicated_worker,
            path,
            concurrency_settings,
        } => JobPayloadUntagged {
            runnable_id: Some(id.0),
            runnable_path: Some(path),
            job_kind: JobKind::FlowScript,
            language: Some(language),
            concurrency_settings,
            cache_ttl,
            cache_ignore_s3_path,
            dedicated_worker,
            ..Default::default()
        },
        JobPayload::FlowNode { id, path } => {
            let data = cache::flow::fetch_flow(_db, id).await?;
            let value = data.value();
            let status = Some(FlowStatus::new(value));
            // Keep inserting `value` if not all workers are updated.
            // Starting at `v1.440`, the value is fetched on pull from the flow node id.
            let value_o = if !MIN_VERSION_IS_AT_LEAST_1_440.met().await {
                Some(value.clone())
            } else {
                // `raw_flow` is fetched on pull.
                None
            };
            JobPayloadUntagged {
                runnable_id: Some(id.0),
                runnable_path: Some(path),
                job_kind: JobKind::FlowNode,
                raw_flow: value_o,
                flow_status: status,
                ..Default::default()
            }
        }
        JobPayload::AppScript {
            id, // app_script(id).
            path,
            language,
            cache_ttl,
        } => JobPayloadUntagged {
            runnable_id: Some(id.0),
            runnable_path: path,
            job_kind: JobKind::AppScript,
            language: Some(language),
            cache_ttl,
            ..Default::default()
        },
        JobPayload::ScriptHub { path, apply_preprocessor } => {
            if path == "hub/7771/slack" || path == "hub/7836/slack" || path == "hub/9084/slack" {
                // these scripts send app reports to slack
                // they use the slack bot token and should therefore be run with permissions to access it
                permissioned_as = SUPERADMIN_NOTIFICATION_EMAIL.to_string();
                email = SUPERADMIN_NOTIFICATION_EMAIL;
            }

            if apply_preprocessor {
                preprocessed = Some(false);
            }

            let hub_script =
                get_full_hub_script_by_path(StripPath(path.clone()), &HTTP_CLIENT, Some(_db))
                    .await?;

            JobPayloadUntagged {
                runnable_path: Some(path),
                job_kind: JobKind::Script_Hub,
                language: Some(hub_script.language),
                ..Default::default()
            }
        }
        JobPayload::Code(RawCode {
            content,
            path,
            hash,
            language,
            lock,
            cache_ttl,
            cache_ignore_s3_path,
            dedicated_worker,
            concurrency_settings,
            debouncing_settings,
        }) => JobPayloadUntagged {
            runnable_id: hash,
            runnable_path: path,
            raw_code_tuple: Some((content, lock)),
            job_kind: JobKind::Preview,
            language: Some(language),
            concurrency_settings: concurrency_settings.into(),
            debouncing_settings,
            cache_ttl,
            cache_ignore_s3_path,
            dedicated_worker,
            ..Default::default()
        },
        JobPayload::Dependencies {
            hash,
            language,
            path,
            dedicated_worker,
            debouncing_settings,
        } => JobPayloadUntagged {
            runnable_id: Some(hash.0),
            runnable_path: Some(path),
            job_kind: JobKind::Dependencies,
            language: Some(language),
            dedicated_worker,
            debouncing_settings,
            ..Default::default()
        },

        // CLI usage, is not modifying db, no need for debouncing.
        JobPayload::RawScriptDependencies { script_path, content, language } => {
            JobPayloadUntagged {
                runnable_path: Some(script_path),
                raw_code_tuple: Some((content, None)),
                job_kind: JobKind::Dependencies,
                language: Some(language),
                ..Default::default()
            }
        }

        // CLI usage, is not modifying db, no need for debouncing.
        JobPayload::RawFlowDependencies { path, flow_value } => JobPayloadUntagged {
            runnable_path: Some(path),
            job_kind: JobKind::FlowDependencies,
            raw_flow: Some(flow_value),
            ..Default::default()
        },
        JobPayload::FlowDependencies { path, dedicated_worker, version, debouncing_settings } => {
            #[cfg(test)]
            let skip_compat = args
                .args
                .contains_key("dbg_create_job_for_unexistant_flow_version");

            #[cfg(not(test))]
            let skip_compat = false;

            // Keep inserting `value` if not all workers are updated.
            // Starting at `v1.440`, the value is fetched on pull from the version id.
            let value_o = if !MIN_VERSION_IS_AT_LEAST_1_440.met().await && !skip_compat {
                let mut ntx = tx.into_tx().await?;
                // The version has been inserted only within the transaction.
                let data = cache::flow::fetch_version(&mut *ntx, version).await?;
                tx = PushIsolationLevel::Transaction(ntx);
                Some(data.value().clone())
            } else {
                // `raw_flow` is fetched on pull.
                None
            };
            JobPayloadUntagged {
                runnable_id: Some(version),
                runnable_path: Some(path.clone()),
                job_kind: JobKind::FlowDependencies,
                raw_flow: value_o,
                dedicated_worker,
                debouncing_settings,
                ..Default::default()
            }
        }
        JobPayload::AppDependencies { path, version, debouncing_settings } => JobPayloadUntagged {
            runnable_id: Some(version),
            runnable_path: Some(path.clone()),
            job_kind: JobKind::AppDependencies,
            debouncing_settings,
            ..Default::default()
        },
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
                            restarted_from_val.flow_version,
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
                            flow_version: restarted_from_val.flow_version,
                        }),
                        user_states,
                        preprocessor_module: None,
                        stream_job: None,
                        chat_input_enabled: None,
                        memory_id: None,
                    }
                }
                _ => {
                    value.preprocessor_module = None;
                    FlowStatus::new(&value)
                } // this is a new flow being pushed, flow_status is set to flow_value
            };
            let cache_ttl = value.cache_ttl.map(|x| x as i32);
            let priority = value.priority;
            JobPayloadUntagged {
                runnable_path: path,
                job_kind: JobKind::FlowPreview,
                flow_status: Some(flow_status),
                cache_ttl,
                cache_ignore_s3_path: value.cache_ignore_s3_path,
                _low_level_priority: priority,
                concurrency_settings: value.concurrency_settings.clone(),
                debouncing_settings: value.debouncing_settings.clone(),
                raw_flow: Some(value),
                ..Default::default()
            }
        }
        JobPayload::SingleStepFlow {
            path,
            hash,
            flow_version,
            retry,
            error_handler_path,
            error_handler_args,
            skip_handler,
            args,
            cache_ttl,
            cache_ignore_s3_path,
            priority,
            tag_override,
            trigger_path,
            apply_preprocessor,
            debouncing_settings,
            concurrency_settings,
        } => {
            // Determine if this is a flow or a script
            let is_flow = flow_version.is_some();

            // Build modules list
            let mut modules = vec![];

            // Add skip validation module if provided
            if let Some(skip_handler) = skip_handler {
                let mut skip_input_transforms = HashMap::<String, InputTransform>::new();
                for (arg_name, arg_value) in skip_handler.args {
                    skip_input_transforms
                        .insert(arg_name, InputTransform::Static { value: arg_value });
                }

                modules.push(FlowModule {
                    id: "skip_validation".to_string(),
                    value: to_raw_value(&FlowModuleValue::Script {
                        input_transforms: skip_input_transforms,
                        path: skip_handler.path,
                        hash: None,
                        tag_override: None,
                        is_trigger: None,
                        pass_flow_input_directly: None,
                    }),
                    stop_after_if: Some(StopAfterIf {
                        expr: skip_handler.stop_condition,
                        skip_if_stopped: true,
                        error_message: Some(skip_handler.stop_message),
                    }),
                    ..Default::default()
                });
            }

            // Add main module (script or flow)
            let mut main_input_transforms = HashMap::<String, InputTransform>::new();
            for (arg_name, arg_value) in args {
                main_input_transforms.insert(arg_name, InputTransform::Static { value: arg_value });
            }

            let main_module = if is_flow {
                FlowModule {
                    id: "a".to_string(),
                    value: to_raw_value(&FlowModuleValue::Flow {
                        path: path.clone(),
                        input_transforms: main_input_transforms,
                        pass_flow_input_directly: None,
                    }),
                    retry,
                    pass_flow_input_directly: Some(true),
                    ..Default::default()
                }
            } else {
                FlowModule {
                    id: "a".to_string(),
                    value: to_raw_value(&FlowModuleValue::Script {
                        input_transforms: main_input_transforms,
                        path: path.clone(),
                        hash,
                        tag_override,
                        is_trigger: None,
                        pass_flow_input_directly: None,
                    }),
                    retry,
                    apply_preprocessor: Some(apply_preprocessor),
                    ..Default::default()
                }
            };
            modules.push(main_module);

            // Build failure module if error handler is provided
            let failure_module = if let Some(error_handler_path) = error_handler_path {
                let mut input_transforms = HashMap::<String, InputTransform>::new();
                input_transforms.insert(
                    "error".to_string(),
                    InputTransform::Javascript { expr: "error".to_string() },
                );
                input_transforms.insert(
                    "path".to_string(),
                    InputTransform::Static { value: to_raw_value(&path) },
                );
                input_transforms.insert(
                    "is_flow".to_string(),
                    InputTransform::Static { value: to_raw_value(&is_flow) },
                );
                input_transforms.insert(
                    "trigger_path".to_string(),
                    InputTransform::Static { value: to_raw_value(&trigger_path) },
                );
                input_transforms.insert(
                    "workspace_id".to_string(),
                    InputTransform::Static { value: to_raw_value(&workspace_id) },
                );
                input_transforms.insert(
                    "email".to_string(),
                    InputTransform::Static { value: to_raw_value(&email) },
                );
                // for the below transforms to work, make sure that flow_job_id and started_at are added to the eval context when pusing the error handler job
                input_transforms.insert(
                    "job_id".to_string(),
                    InputTransform::Javascript { expr: "flow_job_id".to_string() },
                );
                input_transforms.insert(
                    "started_at".to_string(),
                    InputTransform::Javascript { expr: "started_at".to_string() },
                );

                if let Some(error_handler_args) = error_handler_args {
                    for (arg_name, arg_value) in error_handler_args {
                        input_transforms
                            .insert(arg_name, InputTransform::Static { value: arg_value });
                    }
                }

                Some(Box::new(FlowModule {
                    id: "failure".to_string(),
                    value: to_raw_value(&FlowModuleValue::Script {
                        input_transforms,
                        path: error_handler_path,
                        hash: None,
                        tag_override: None,
                        is_trigger: None,
                        pass_flow_input_directly: None,
                    }),
                    ..Default::default()
                }))
            } else {
                None
            };

            let flow_value = FlowValue {
                modules,
                failure_module,
                concurrency_settings: concurrency_settings.clone(),
                debouncing_settings: debouncing_settings.clone(),
                priority,
                cache_ttl: cache_ttl.map(|val| val as u32),
                cache_ignore_s3_path: cache_ignore_s3_path,
                same_worker: false,
                early_return: None,
                skip_expr: None,
                preprocessor_module: None,
                chat_input_enabled: None,
                flow_env: None,
            };
            // this is a new flow being pushed, flow_status is set to flow_value:
            let flow_status: FlowStatus = FlowStatus::new(&flow_value);
            JobPayloadUntagged {
                runnable_path: Some(path),
                job_kind: JobKind::SingleStepFlow,
                raw_flow: Some(flow_value),
                flow_status: Some(flow_status),
                cache_ttl,
                cache_ignore_s3_path,
                _low_level_priority: priority,
                concurrency_settings,
                debouncing_settings,
                ..Default::default()
            }
        }
        JobPayload::Flow { path, dedicated_worker, apply_preprocessor, version } => {
            let mut ntx = tx.into_tx().await?;
            // Do not use the lite version unless all workers are updated.
            let data = if *DISABLE_FLOW_SCRIPT
                || (!MIN_VERSION_IS_AT_LEAST_1_432.met().await && !*CLOUD_HOSTED)
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

            let mut value = data.value().clone();
            let priority = value.priority;
            let cache_ttl = value.cache_ttl.map(|x| x as i32);
            let cache_ignore_s3_path = value.cache_ignore_s3_path;
            let mut concurrency_settings = value.concurrency_settings.clone();
            let mut debouncing_settings = value.debouncing_settings.clone();

            if !apply_preprocessor {
                value.preprocessor_module = None;
            } else {
                tag = None;

                concurrency_settings.concurrent_limit = None;
                // TODO: May be re-enable?
                debouncing_settings.debounce_delay_s = None;

                preprocessed = Some(false);
            }

            // this is a new flow being pushed, status is set to `value`.
            let status = FlowStatus::new(&value);

            // Keep inserting `value` if not all workers are updated.
            // Starting at `v1.440`, the value is fetched on pull from the version id.
            let value_o = if !MIN_VERSION_IS_AT_LEAST_1_440.met().await {
                let mut value = value;
                add_virtual_items_if_necessary(&mut value.modules);
                if same_worker {
                    value.same_worker = true;
                }
                Some(value)
            } else {
                // `raw_flow` is fetched on pull, the mutations from the other branch are replaced
                // by additional checks when handling the flow.
                None
            };
            JobPayloadUntagged {
                runnable_id: Some(version), // Starting from `v1.436`, the version id is used to fetch the value on pull.
                runnable_path: Some(path),
                job_kind: JobKind::Flow,
                raw_flow: value_o,
                flow_status: Some(status),
                cache_ttl,
                cache_ignore_s3_path,
                dedicated_worker,
                _low_level_priority: priority,
                concurrency_settings,
                debouncing_settings,
                ..Default::default()
            }
        }
        JobPayload::RestartedFlow {
            completed_job_id,
            step_id,
            branch_or_iteration_n,
            flow_version,
        } => {
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
                flow_version,
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
                    flow_version,
                }),
                user_states,
                preprocessor_module: None,
                stream_job: None,
                chat_input_enabled: None,
                memory_id: None,
            };
            let value = flow_data.value();
            let priority = value.priority;
            let concurrency_settings = value.concurrency_settings.clone();
            let debouncing_settings = value.debouncing_settings.clone();
            let cache_ttl = value.cache_ttl.map(|x| x as i32);
            // Keep inserting `value` if not all workers are updated.
            // Starting at `v1.440`, the value is fetched on pull from the version id.
            let value_o = if version.is_none() || !MIN_VERSION_IS_AT_LEAST_1_440.met().await {
                Some(value.clone())
            } else {
                // `raw_flow` is fetched on pull.
                None
            };
            JobPayloadUntagged {
                runnable_id: version,
                runnable_path: flow_path,
                job_kind: JobKind::Flow,
                raw_flow: value_o,
                flow_status: Some(restarted_flow_status),
                cache_ttl,
                cache_ignore_s3_path: value.cache_ignore_s3_path,
                _low_level_priority: priority,
                concurrency_settings,
                debouncing_settings,
                ..Default::default()
            }
        }
        JobPayload::DeploymentCallback { path, debouncing_settings } => JobPayloadUntagged {
            runnable_path: Some(path.clone()),
            job_kind: JobKind::DeploymentCallback,
            concurrency_settings: ConcurrencySettings {
                concurrency_key: Some(format!("{workspace_id}:git_sync")),
                concurrent_limit: Some(1),
                concurrency_time_window_s: Some(0),
            },
            debouncing_settings,
            ..Default::default()
        },
        JobPayload::Identity => {
            JobPayloadUntagged { job_kind: JobKind::Identity, ..Default::default() }
        }
        JobPayload::Noop => JobPayloadUntagged { job_kind: JobKind::Noop, ..Default::default() },
        JobPayload::AIAgent { path } => JobPayloadUntagged {
            runnable_path: Some(path),
            job_kind: JobKind::AIAgent,
            ..Default::default()
        },
    };

    // Enforce concurrency limit on all dependency jobs.
    // TODO: We can ignore this for scripts djobs. The main reason we need all djobs to be sequential is because we have
    // nodes_to_relock and we need all locks whose corresponding steps aren't in nodes_to_relock be already present.
    //
    // This is not the case for scripts, so we can potentially have multiple djobs for scripts at the same time.
    if let (Some(path), true) = (
        &runnable_path,
        cfg!(feature = "private")
            && job_kind.is_dependency()
            && !*WMDEBUG_NO_DEBOUNCING
            && MIN_VERSION_SUPPORTS_DEBOUNCING.met().await,
    ) {
        concurrency_settings.concurrency_key = Some(format!("dependency:{workspace_id}/{path}"));
        concurrency_settings.concurrent_limit = Some(1);
    }

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
    } else if job_kind == JobKind::Dependencies {
        Some(0)
    } else {
        final_priority
    };

    let is_running = same_worker || running;
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

    let tag = if dedicated_worker.is_some_and(|x| x) {
        let flow_prefix = if job_kind == JobKind::Flow || job_kind == JobKind::FlowDependencies {
            "flow/"
        } else {
            ""
        };
        let full_path = format!(
            "{}{}",
            flow_prefix,
            runnable_path.clone().expect("dedicated script has a path")
        );
        windmill_common::worker::dedicated_worker_tag(workspace_id, &full_path)
    } else {
        if tag == Some("".to_string()) {
            tag = None;
        }

        let interpolated_tag = tag.map(|x| interpolate_args(x, &args, workspace_id));
        let per_workspace = per_workspace_tag(&workspace_id).await;

        let default = || {
            let ntag = if job_kind.is_flow()
                || job_kind == JobKind::Identity
                || job_kind == JobKind::AIAgent
            {
                "flow".to_string()
            } else if job_kind == JobKind::Dependencies
                || job_kind == JobKind::FlowDependencies
                || job_kind == JobKind::DeploymentCallback
                || job_kind == JobKind::AppDependencies
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
                        if job_kind == JobKind::Dependencies {
                            ScriptLang::Bun.as_str()
                        } else {
                            ScriptLang::Nativets.as_str()
                        }
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

    #[cfg(feature = "private")]
    if schedule_path.is_none() && flow_step_id.is_none() {
        crate::jobs_ee::maybe_debounce(
            &debouncing_settings,
            &mut scheduled_for_o,
            &runnable_path,
            workspace_id,
            job_kind,
            job_id,
            &args,
            &mut tx,
        )
        .await?
    }

    let (trigger_path, trigger_kind) = trigger
        .map_or_else(
            || schedule_path.map(|path| (Some(path), JobTriggerKind::Schedule)),
            |trigger| Some((trigger.trigger_path, trigger.trigger_kind)),
        )
        .unzip();

    if concurrency_settings.concurrent_limit.is_some() {
        insert_concurrency_key(
            workspace_id,
            &args,
            &runnable_path,
            job_kind,
            concurrency_settings.concurrency_key.clone(),
            &mut *tx,
            job_id,
        )
        .await?;
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
            windmill_common::auth::fetch_authed_from_permissioned_as(
                &permissioned_as,
                email,
                workspace_id,
                &mut *tx,
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

    // if let Err(err) = sqlx::query!("INSERT INTO job_perms (job_id, email, username, is_admin, is_operator, folders, groups, workspace_id)
    //     values ($1, $2, $3, $4, $5, $6, $7, $8)
    //     ON CONFLICT (job_id) DO UPDATE SET email = $2, username = $3, is_admin = $4, is_operator = $5, folders = $6, groups = $7, workspace_id = $8",
    //     job_id,
    //     job_authed.email,
    //     job_authed.username,
    //     job_authed.is_admin,
    //     job_authed.is_operator,
    //     folders.as_slice(),
    //     job_authed.groups.as_slice(),
    //     workspace_id,
    // ).execute(&mut *tx).await {
    //     tracing::error!("Could not insert job_perms for job {job_id}: {err:#}");
    // }

    let root_job = if root_job.is_some()
        && (root_job == flow_innermost_root_job.or(parent_job).or(Some(job_id)))
    {
        // We only save the root job if it's not the innermost root job, parent job, or the job itself as an optimization
        // Reference: see [`windmill_worker::common::get_root_job_id`] for logic on determining the root job.
        None
    } else {
        root_job
    };

    let (job_kind, scheduled_for_o) = if suspended_mode.unwrap_or(false) {
        let unassigned_job_kind = match job_kind {
            JobKind::Script => JobKind::UnassignedScript,
            JobKind::Flow => JobKind::UnassignedFlow,
            JobKind::SingleStepFlow => JobKind::UnassignedSinglestepFlow,
            _ => return Err(Error::internal_err(format!("Cannot suspend job of kind {job_kind:?} as it is not a script, flow or single step flow"))),
        };
        (
            unassigned_job_kind,
            Some(Utc::now() + chrono::Duration::days(30)),
        )
    } else {
        (job_kind, scheduled_for_o)
    };

    let runnable_settings_handle = windmill_common::runnable_settings::insert_rs(
        RunnableSettings {
            debouncing_settings: debouncing_settings.insert_cached(_db).await?,
            concurrency_settings: concurrency_settings.insert_cached(_db).await?,
        },
        _db,
    )
    .await?;

    let (guarded_concurrent_limit, guarded_concurrency_time_window_s) =
        if windmill_common::runnable_settings::min_version_supports_runnable_settings_v0().await {
            (None, None)
        } else {
            (
                concurrency_settings.concurrent_limit,
                concurrency_settings.concurrency_time_window_s,
            )
        };
    sqlx::query!(
        "WITH inserted_job AS (
            INSERT INTO v2_job (
                id, -- 1
                workspace_id, -- 2
                raw_code, -- 3
                raw_lock, -- 4
                raw_flow, -- 5
                tag, -- 6
                parent_job, -- 7
                created_by, -- 8
                permissioned_as, -- 9
                runnable_id, -- 10
                runnable_path, -- 11
                args, -- 12
                kind, -- 13
                trigger, -- 14
                script_lang, -- 15
                same_worker, -- 16
                pre_run_error, -- 17 
                permissioned_as_email, -- 18
                visible_to_owner, -- 19
                flow_innermost_root_job, -- 20
                root_job, -- 38
                concurrent_limit, -- 21
                concurrency_time_window_s, -- 22
                timeout, -- 23
                flow_step_id, -- 24
                cache_ttl, -- 25
                priority, -- 26
                trigger_kind, -- 39
                script_entrypoint_override, -- 12
                preprocessed -- 27,
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18,
            $19, $20, $38, $21, $22, $23, $24, $25, $26, $39::job_trigger_kind,
            ($12::JSONB)->>'_ENTRYPOINT_OVERRIDE', $27)
        ),
        inserted_runtime AS (
            INSERT INTO v2_job_runtime (id, ping) VALUES ($1, null)
        ),
        inserted_job_perms AS (
            INSERT INTO job_perms (job_id, email, username, is_admin, is_operator, folders, groups, workspace_id, end_user_email) 
            values ($1, $32, $33, $34, $35, $36, $37, $2, $41) 
            ON CONFLICT (job_id) DO UPDATE SET email = EXCLUDED.email, username = EXCLUDED.username, is_admin = EXCLUDED.is_admin, is_operator = EXCLUDED.is_operator, folders = EXCLUDED.folders, groups = EXCLUDED.groups, workspace_id = EXCLUDED.workspace_id, end_user_email = EXCLUDED.end_user_email
        )
        INSERT INTO v2_job_queue
            (workspace_id, id, running, scheduled_for, started_at, tag, priority, cache_ignore_s3_path, runnable_settings_handle)
            VALUES ($2, $1, $28, COALESCE($29, now()), CASE WHEN $27 OR $40 THEN now() END, $30, $31, $42, $43)",
        job_id,
        workspace_id,
        raw_code,
        raw_lock,
        raw_flow as Option<Json<FlowValue>>,
        tag,
        parent_job,
        user,
        permissioned_as,
        runnable_id,
        runnable_path.clone(),
        Json(args) as Json<PushArgs>,
        job_kind.clone() as JobKind,
        trigger_path.flatten(),
        language as Option<ScriptLang>,
        same_worker,
        pre_run_error.map(|e| e.to_string()),
        email,
        visible_to_owner,
        flow_innermost_root_job,
        guarded_concurrent_limit,
        guarded_concurrency_time_window_s,
        custom_timeout,
        flow_step_id,
        cache_ttl,
        final_priority,
        preprocessed,
        is_running,
        scheduled_for_o,
        tag,
        final_priority,
        job_authed.email,
        job_authed.username,
        job_authed.is_admin,
        job_authed.is_operator,
        folders.as_slice(),
        job_authed.groups.as_slice(),
        root_job,
        trigger_kind as Option<JobTriggerKind>,
        running,
        end_user_email,
        cache_ignore_s3_path,
        runnable_settings_handle,
    )
    .execute(&mut *tx)
    .warn_after_seconds(1)
    .await?;

    // RunnableSettings::insert(RunnableType::Job)

    //     tracing::debug!("Pushing job {job_id} with tag {tag}, schedule_path {schedule_path:?}, script_path: {script_path:?}, email {email}, workspace_id {workspace_id}");
    //     let uuid = sqlx::query_scalar!(
    //         "INSERT INTO v2_job_queue
    //             (workspace_id, id, running, scheduled_for, started_at, tag, priority)
    //             VALUES ($1, $2, $3, COALESCE($4, now()), CASE WHEN $3 THEN now() END, $5, $6) \
    //          RETURNING id AS \"id!\"",
    //         workspace_id,
    //         job_id,
    // ,
    //     )
    //     .fetch_one(&mut *tx)
    //     .warn_after_seconds(1)
    //     .await
    //     .map_err(|e| Error::internal_err(format!("Could not insert into queue {job_id} with tag {tag}, schedule_path {schedule_path:?}, script_path: {script_path:?}, email {email}, workspace_id {workspace_id}: {e:#}")))?;

    // sqlx::query!(
    //     "INSERT INTO v2_job_runtime (id, ping) VALUES ($1, null)",
    //     job_id
    // )
    // .execute(&mut *tx)
    // .await?;
    if let Some(flow_status) = flow_status {
        sqlx::query!(
            "INSERT INTO v2_job_status (id, flow_status) VALUES ($1, $2)",
            job_id,
            Json(flow_status) as Json<FlowStatus>,
        )
        .execute(&mut *tx)
        .warn_after_seconds(1)
        .await?;
    }

    tracing::debug!("Pushed {job_id}");
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
                s = ScriptHash(runnable_id.unwrap()).to_string();
                hm.insert("hash", s.as_str());
                "jobs.run.script"
            }
            JobKind::Flow => "jobs.run.flow",
            JobKind::FlowPreview => "jobs.run.flow_preview",
            JobKind::SingleStepFlow => "jobs.run.single_step_flow",
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
            JobKind::AIAgent => "jobs.run.ai_agent",
            JobKind::UnassignedScript => "jobs.run.unassigned_script",
            JobKind::UnassignedFlow => "jobs.run.unassigned_flow",
            JobKind::UnassignedSinglestepFlow => "jobs.run.unassigned_singlestepflow",
        };

        let audit_author = if format!("u/{user}") != permissioned_as && user != permissioned_as {
            AuditAuthor {
                email: email.to_string(),
                username: permissioned_as.trim_start_matches("u/").to_string(),
                username_override: Some(user.to_string()),
                token_prefix: token_prefix.map(|s| s.to_string()),
            }
        } else {
            AuditAuthor {
                email: email.to_string(),
                username: user.to_string(),
                username_override: None,
                token_prefix: token_prefix.map(|s| s.to_string()),
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
            runnable_path.as_ref().map(|x| x.as_str()),
            Some(hm),
        )
        .warn_after_seconds(1)
        .await?;
    }

    Ok((job_id, tx))
}

pub async fn insert_concurrency_key<'d, 'c>(
    workspace_id: &str,
    args: &PushArgs<'d>,
    script_path: &Option<String>,
    job_kind: JobKind,
    custom_concurrency_key: Option<String>,
    db: impl PgExecutor<'c>,
    job_id: Uuid,
) -> Result<(), Error> {
    let concurrency_key = custom_concurrency_key
        .map(|x| {
            let interpolated = interpolate_args(x.clone(), args, workspace_id);
            // In cloud mode, enforce workspace isolation by prefixing with workspace
            // if the custom key doesn't already specify $workspace
            #[cfg(feature = "cloud")]
            {
                if !x.contains("$workspace") {
                    format!("{}/{}", workspace_id, interpolated)
                } else {
                    interpolated
                }
            }
            #[cfg(not(feature = "cloud"))]
            {
                interpolated
            }
        })
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
    .execute(db)
    .warn_after_seconds(3)
    .await
    .map_err(|e| Error::internal_err(format!("Could not insert concurrency_key={concurrency_key} for job_id={job_id} script_path={script_path:?} workspace_id={workspace_id}: {e:#}")))?;
    Ok(())
}

// pub async fn insert_debounce_key<'d, 'c>(
//     workspace_id: &str,
//     args: &PushArgs<'d>,
//     script_path: &Option<String>,
//     job_kind: JobKind,
//     custom_concurrency_key: Option<String>,
//     tx: &mut Transaction<'c, Postgres>,
//     job_id: Uuid,
// ) -> Result<(), Error> {
//     let concurrency_key = custom_concurrency_key
//         .map(|x| interpolate_args(x, args, workspace_id))
//         .unwrap_or(fullpath_with_workspace(
//             workspace_id,
//             script_path.as_ref(),
//             &job_kind,
//         ));
//     sqlx::query!(
//         "WITH inserted_concurrency_counter AS (
//                 INSERT INTO concurrency_counter (concurrency_id, job_uuids)
//                 VALUES ($1, '{}'::jsonb)
//                 ON CONFLICT DO NOTHING
//             )
//             INSERT INTO concurrency_key(key, job_id) VALUES ($1, $2)",
//         concurrency_key,
//         job_id,
//     )
//     .execute(&mut **tx)
//     .warn_after_seconds(3)
//     .await
//     .map_err(|e| Error::internal_err(format!("Could not insert concurrency_key={concurrency_key} for job_id={job_id} script_path={script_path:?} workspace_id={workspace_id}: {e:#}")))?;
//     Ok(())
// }

pub fn canceled_job_to_result(job: &MiniPulledJob) -> serde_json::Value {
    let reason = job
        .canceled_reason
        .as_deref()
        .unwrap_or_else(|| "no reason given");
    let canceler = job.canceled_by.as_deref().unwrap_or_else(|| "unknown");
    serde_json::json!({"message": format!("Job canceled: {reason} by {canceler}"), "name": "Canceled", "reason": reason, "canceler": canceler})
}

/// Helper function to create a restarted module for branch/iteration restart
fn create_restarted_module(
    module: &FlowStatusModule,
    module_definition: &FlowModule,
    branch_or_iteration_n: usize,
    restart_step_id: &str,
) -> Result<FlowStatusModule, Error> {
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
                    restart_step_id, total_branch_number, branch_or_iteration_n,
                )));
            }
            let mut new_flow_jobs = module.flow_jobs().unwrap_or_default();
            new_flow_jobs.truncate(branch_or_iteration_n);
            let mut new_flow_jobs_success = module.flow_jobs_success();
            if let Some(new_flow_jobs_success) = new_flow_jobs_success.as_mut() {
                new_flow_jobs_success.truncate(branch_or_iteration_n);
            }
            let mut new_flow_jobs_timeline = module.flow_jobs_duration();
            if let Some(new_flow_jobs_timeline) = new_flow_jobs_timeline.as_mut() {
                new_flow_jobs_timeline.truncate(branch_or_iteration_n);
            }
            Ok(FlowStatusModule::InProgress {
                id: module.id(),
                job: new_flow_jobs[new_flow_jobs.len() - 1],
                iterator: None,
                flow_jobs: Some(new_flow_jobs),
                flow_jobs_success: new_flow_jobs_success,
                flow_jobs_duration: new_flow_jobs_timeline,
                branch_chosen: None,
                branchall: Some(BranchAllStatus {
                    branch: branch_or_iteration_n - 1,
                    len: branches.len(),
                }),
                parallel,
                while_loop: false,
                progress: None,
                agent_actions: None,
                agent_actions_success: None,
            })
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
            let mut new_flow_jobs_timeline = module.flow_jobs_duration();
            if let Some(new_flow_jobs_timeline) = new_flow_jobs_timeline.as_mut() {
                new_flow_jobs_timeline.truncate(branch_or_iteration_n);
            }
            Ok(FlowStatusModule::InProgress {
                id: module.id(),
                job: new_flow_jobs[new_flow_jobs.len() - 1],
                iterator: Some(FlowIterator {
                    index: branch_or_iteration_n - 1,
                    itered: None,
                    itered_len: None,
                }),
                flow_jobs: Some(new_flow_jobs),
                flow_jobs_success: new_flow_jobs_success,
                flow_jobs_duration: new_flow_jobs_timeline,
                branch_chosen: None,
                branchall: None,
                parallel,
                while_loop: false,
                progress: None,
                agent_actions: None,
                agent_actions_success: None,
            })
        }
        _ => Err(Error::internal_err(format!(
            "Module {} is not a branchall or forloop, unable to restart it at step {:?}",
            restart_step_id, branch_or_iteration_n
        ))),
    }
}

async fn restarted_flows_resolution(
    db: &Pool<Postgres>,
    workspace_id: &str,
    completed_flow_id: Uuid,
    restart_step_id: &str,
    branch_or_iteration_n: Option<usize>,
    flow_version: Option<i64>,
    // parents: Vec<RestartedParent>,
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
            j.runnable_path as script_path, j.runnable_id AS \"script_hash: ScriptHash\",
            j.kind AS \"job_kind!: JobKind\",
            COALESCE(c.flow_status, c.workflow_as_code_status) AS \"flow_status: Json<Box<RawValue>>\",
            j.raw_flow AS \"raw_flow: Json<Box<RawValue>>\"
        FROM v2_job_completed c JOIN v2_job j USING (id) WHERE j.id = $1 and j.workspace_id = $2",
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

    let current_flow_version = row.script_hash.map(|x| x.0);
    let is_version_change = flow_version.is_some()
        && current_flow_version.is_some()
        && flow_version != current_flow_version
        && row.job_kind == JobKind::Flow;

    let flow_data = if is_version_change {
        // Fetch the new flow version
        let new_version = flow_version.unwrap();
        cache::flow::fetch_version(db, new_version).await?
    } else {
        cache::job::fetch_flow(db, &row.job_kind, row.script_hash)
            .or_else(|_| {
                cache::job::fetch_preview_flow(db.into(), &completed_flow_id, row.raw_flow)
            })
            .await?
    };

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

    if is_version_change {
        // When the flow version has changed, create flow status from scratch
        // based on the new flow, but match modules from the old flow status where possible
        for module_definition in &flow_value.modules {
            let module_id = &module_definition.id;

            if module_id == restart_step_id {
                // Mark this and all following modules as WaitingForPriorSteps
                if branch_or_iteration_n.is_none() || branch_or_iteration_n.unwrap() == 0 {
                    truncated_modules
                        .push(FlowStatusModule::WaitingForPriorSteps { id: module_id.clone() });
                } else {
                    // Handle branch/iteration restart for version changes
                    let branch_n = branch_or_iteration_n.unwrap();
                    // Try to find matching module in old flow status
                    if let Some(old_module) =
                        flow_status.modules.iter().find(|m| &m.id() == module_id)
                    {
                        truncated_modules.push(create_restarted_module(
                            old_module,
                            module_definition,
                            branch_n,
                            restart_step_id,
                        )?);
                    } else {
                        // Module not found in old flow, mark as waiting
                        truncated_modules
                            .push(FlowStatusModule::WaitingForPriorSteps { id: module_id.clone() });
                    }
                }
                dependent_module = true;
            } else if dependent_module {
                truncated_modules
                    .push(FlowStatusModule::WaitingForPriorSteps { id: module_id.clone() });
            } else {
                // Before the restart step, try to match with old flow status
                if let Some(old_module) = flow_status.modules.iter().find(|m| &m.id() == module_id)
                {
                    truncated_modules.push(old_module.clone());
                } else {
                    truncated_modules
                        .push(FlowStatusModule::WaitingForPriorSteps { id: module_id.clone() });
                }
                step_n += 1;
            }
        }
    } else {
        // Original logic for same version
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
                    truncated_modules
                        .push(FlowStatusModule::WaitingForPriorSteps { id: module.id() });
                } else {
                    // expect a module to be either a branchall (resp. loop), and resume the flow from this branch (resp. iteration)
                    truncated_modules.push(create_restarted_module(
                        &module,
                        module_definition,
                        branch_or_iteration_n.unwrap(),
                        restart_step_id,
                    )?);
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
    }

    if !dependent_module {
        // step not found in flow.
        return Err(Error::internal_err(format!(
            "Flow cannot be restarted from step {} as it could not be found.",
            restart_step_id
        )));
    }

    Ok((
        if is_version_change {
            flow_version
        } else {
            row.script_hash.map(|x| x.0)
        },
        row.script_path,
        flow_data,
        step_n,
        truncated_modules,
        flow_status.user_states,
        flow_status.cleanup_module,
    ))
}

// Wrapper struct to send both job and optional flow_runners to dedicated workers
pub struct DedicatedWorkerJob {
    pub job: Arc<MiniPulledJob>,
    pub flow_runners: Option<Arc<FlowRunners>>,
    pub done_tx: Option<oneshot::Sender<()>>,
}

#[derive(Debug)]
pub struct FlowRunners {
    pub runners: HashMap<String, Sender<DedicatedWorkerJob>>,
    pub handles: Vec<JoinHandle<()>>,
    pub job_id: Uuid,
}

impl Drop for FlowRunners {
    fn drop(&mut self) {
        let total_runners = self.handles.len();
        tracing::info!(
            "dropping {} flow runners for job {}",
            total_runners,
            self.job_id
        );

        // First, drop all senders to signal workers to stop gracefully
        self.runners.clear();

        // Spawn a background task to wait with timeout and abort if needed
        let handles = std::mem::take(&mut self.handles);
        let job_id = self.job_id;

        tokio::spawn(async move {
            // Extract abort handles before consuming the join handles
            let abort_handles: Vec<_> = handles.iter().map(|h| h.abort_handle()).collect();

            // Wait up to 5 seconds for natural termination
            let timeout_result = tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                futures::future::join_all(handles),
            )
            .await;

            match timeout_result {
                Ok(_) => {
                    tracing::info!(
                        "all {} flow runners for job {} terminated gracefully",
                        total_runners,
                        job_id
                    );
                }
                Err(_) => {
                    // Timeout reached, abort only the handles that haven't finished
                    let mut aborted = 0;
                    for abort_handle in abort_handles {
                        if !abort_handle.is_finished() {
                            abort_handle.abort();
                            aborted += 1;
                        }
                    }
                    let graceful = total_runners - aborted;
                    tracing::warn!(
                        "flow runners for job {}: {} terminated gracefully, {} aborted after 5s timeout",
                        job_id, graceful, aborted
                    );
                }
            }
        });
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SameWorkerPayload {
    pub job_id: Uuid,
    pub recoverable: bool,
    #[serde(skip)]
    pub flow_runners: Option<Arc<FlowRunners>>,
}

pub async fn get_same_worker_job(
    db: &DB,
    same_worker_job: &SameWorkerPayload,
) -> windmill_common::error::Result<Option<PulledJob>> {
    sqlx::query_as::<_, PulledJob>(
        "WITH ping AS (
                        UPDATE v2_job_runtime SET ping = NOW() WHERE id = $1
                    ),
                    started_at AS (
                        UPDATE v2_job_queue SET started_at = NOW() WHERE id = $1
                    )
                    SELECT
                    v2_job_queue.workspace_id,
                    v2_job_queue.id,
                    v2_job.args,
                    v2_job.parent_job,
                    v2_job.created_by,
                    v2_job_queue.started_at,
                    scheduled_for,
                    v2_job.runnable_path,
                    v2_job.kind,
                    v2_job.runnable_id,
                    v2_job_queue.canceled_reason,
                    v2_job_queue.canceled_by,
                    v2_job.permissioned_as,
                    v2_job.permissioned_as_email,
                    v2_job_status.flow_status,
                    v2_job.tag,
                    v2_job.script_lang,
                    v2_job.same_worker,
                    v2_job.pre_run_error,
                    v2_job.concurrent_limit,
                    v2_job.concurrency_time_window_s,
                    v2_job.flow_innermost_root_job,
                    v2_job.root_job,
                    v2_job.timeout,
                    v2_job.flow_step_id,
                    v2_job.cache_ttl,
                    v2_job_queue.cache_ignore_s3_path,
                    v2_job_queue.runnable_settings_handle,
                    v2_job_queue.priority,
                    v2_job.preprocessed,
                    v2_job.script_entrypoint_override,
                    v2_job.trigger,
                    v2_job.trigger_kind,
                    v2_job.visible_to_owner,
                    v2_job.raw_code,
                    v2_job.raw_lock,
                    v2_job.raw_flow,
                    COALESCE(pj.runnable_path, v2_job.args->>'_FLOW_PATH') as parent_runnable_path,
                    p.email as permissioned_as_email, p.username as permissioned_as_username, p.is_admin as permissioned_as_is_admin,
                    p.is_operator as permissioned_as_is_operator, p.groups as permissioned_as_groups, p.folders as permissioned_as_folders, p.end_user_email as permissioned_as_end_user_email
                    FROM v2_job_queue
                    INNER JOIN v2_job ON v2_job.id = v2_job_queue.id
                    LEFT JOIN v2_job_status ON v2_job_status.id = v2_job_queue.id
                    LEFT JOIN job_perms p ON p.job_id = v2_job.id
                    LEFT JOIN v2_job pj ON v2_job.parent_job = pj.id
                    WHERE v2_job_queue.id = $1
",
    )
    .bind(same_worker_job.job_id)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        Error::internal_err(format!(
            "Impossible to fetch same_worker job {}: {}",
            same_worker_job.job_id, e
        ))
    })
}
