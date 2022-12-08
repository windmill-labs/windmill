/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{collections::HashMap, str::FromStr};

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Transaction};
use tracing::instrument;
use ulid::Ulid;
use uuid::Uuid;
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    error::{self, to_anyhow, Error},
    flow_status::{FlowStatus, JobResult, MAX_RETRY_ATTEMPTS, MAX_RETRY_INTERVAL},
    flows::{FlowModule, FlowModuleValue, FlowValue},
    scripts::{get_full_hub_script_by_path, HubScript, ScriptHash, ScriptLang},
    utils::StripPath,
};

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
}

const MAX_NB_OF_JOBS_IN_Q_PER_USER: i64 = 10;
const MAX_DURATION_LAST_1200: std::time::Duration = std::time::Duration::from_secs(900);

pub async fn cancel_job<'c>(
    username: &str,
    reason: Option<String>,
    id: Uuid,
    w_id: &str,
    mut tx: Transaction<'c, Postgres>,
) -> error::Result<(Transaction<'c, Postgres>, Option<Uuid>)> {
    let job_option = sqlx::query_scalar!(
        "UPDATE queue SET canceled = true, canceled_by = $1, canceled_reason = $2, scheduled_for = now(), suspend = 0 WHERE id = $3 \
         AND workspace_id = $4 RETURNING id",
        username,
        reason,
        id,
        w_id
    )
    .fetch_optional(&mut tx)
    .await?;
    let mut jobs = job_option.map(|j| vec![j]).unwrap_or_default();
    while !jobs.is_empty() {
        let p_job = jobs.pop();
        let new_jobs = sqlx::query_scalar!(
            "UPDATE queue SET canceled = true, canceled_by = $1, canceled_reason = $2 WHERE parent_job = $3 \
             AND workspace_id = $4 RETURNING id",
            username,
            reason,
            p_job,
            w_id
        )
        .fetch_all(&mut tx)
        .await?;
        jobs.extend(new_jobs);
    }
    Ok((tx, job_option))
}

pub async fn pull(db: &Pool<Postgres>) -> windmill_common::error::Result<Option<QueuedJob>> {
    /* Jobs can be started if they:
     * - haven't been started before,
     *   running = false
     * - are flows with a step that needed resume,
     *   suspend_until is non-null
     *   and suspend = 0 when the resume messages are received
     *   or suspend_until <= now() if it has timed out */
    let job: Option<QueuedJob> = sqlx::query_as::<_, QueuedJob>(
        "UPDATE queue
            SET running = true
              , started_at = coalesce(started_at, now())
              , last_ping = now()
              , suspend_until = null
            WHERE id = (
                SELECT id
                FROM queue
                WHERE (    running = false
                       AND scheduled_for <= now())
                   OR (suspend_until IS NOT NULL
                       AND (   suspend <= 0
                            OR suspend_until <= now()))
                ORDER BY scheduled_for
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            RETURNING *",
    )
    .fetch_optional(db)
    .await?;

    if job.is_some() {
        QUEUE_PULL_COUNT.inc();
    }

    Ok(job)
}

pub async fn get_result_by_id(
    db: Pool<Postgres>,
    mut skip_direct: bool,
    w_id: String,
    flow_id: String,
    node_id: String,
) -> error::Result<serde_json::Value> {
    let mut result_id: Option<JobResult> = None;
    let mut parent_id = Uuid::from_str(&flow_id).ok();
    while result_id.is_none() && parent_id.is_some() {
        if !skip_direct {
            let r = sqlx::query!(
                "SELECT flow_status, parent_job FROM completed_job WHERE id = $1 AND workspace_id = $2 UNION ALL SELECT flow_status, parent_job FROM queue WHERE id = $1 AND workspace_id = $2 ",
                parent_id.unwrap(),
                w_id,
            )
            .fetch_optional(&db)
            .await?;
            if let Some(r) = r {
                let value = r
                    .flow_status
                    .as_ref()
                    .ok_or_else(|| Error::InternalErr(format!("requiring a flow status value")))?
                    .to_owned();
                parent_id = r.parent_job;
                let status_o = serde_json::from_value::<FlowStatus>(value).ok();
                result_id = status_o.and_then(|status| {
                    status
                        .modules
                        .iter()
                        .find(|m| m.id() == node_id)
                        .and_then(|m| m.job_result())
                });
            } else {
                parent_id = None;
            }
        } else {
            let q_parent = sqlx::query_scalar!(
                "SELECT parent_job FROM completed_job WHERE id = $1 AND workspace_id = $2 UNION ALL SELECT parent_job FROM queue WHERE id = $1 AND workspace_id = $2",
                parent_id.unwrap(),
                w_id,
            )
            .fetch_optional(&db)
            .await?
            .flatten();
            parent_id = q_parent;
            skip_direct = false
        }
    }
    let result_id = windmill_common::utils::not_found_if_none(
        result_id,
        "Flow result by id",
        format!("{}, {}", flow_id, node_id),
    )?;
    println!("result_id: {:#?}, {node_id}", result_id);

    let value = match result_id {
        JobResult::ListJob(x) => {
            let rows = sqlx::query_scalar!(
                "SELECT result FROM completed_job WHERE id = ANY($1) AND workspace_id = $2",
                x.as_slice(),
                w_id,
            )
            .fetch_all(&db)
            .await?
            .into_iter()
            .filter_map(|x| x)
            .collect::<Vec<serde_json::Value>>();
            serde_json::json!(rows)
        }
        JobResult::SingleJob(x) => sqlx::query_scalar!(
            "SELECT result FROM completed_job WHERE id = $1 AND workspace_id = $2",
            x,
            w_id,
        )
        .fetch_optional(&db)
        .await?
        .flatten()
        .unwrap_or(serde_json::Value::Null),
    };

    Ok(value)
}

#[instrument(level = "trace", skip_all)]
pub async fn delete_job(
    db: &Pool<Postgres>,
    w_id: &str,
    job_id: Uuid,
) -> windmill_common::error::Result<()> {
    QUEUE_DELETE_COUNT.inc();
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
    tracing::debug!("Job {job_id} deleted: {job_removed}");
    Ok(())
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

#[instrument(level = "trace", skip_all)]
pub async fn push<'c>(
    mut tx: Transaction<'c, Postgres>,
    workspace_id: &str,
    job_payload: JobPayload,
    args: serde_json::Map<String, serde_json::Value>,
    user: &str,
    permissioned_as: String,
    scheduled_for_o: Option<chrono::DateTime<chrono::Utc>>,
    schedule_path: Option<String>,
    parent_job: Option<Uuid>,
    is_flow_step: bool,
    mut same_worker: bool,
    pre_run_error: Option<&windmill_common::error::Error>,
) -> Result<(Uuid, Transaction<'c, Postgres>), Error> {
    let scheduled_for = scheduled_for_o.unwrap_or_else(chrono::Utc::now);
    let args_json = serde_json::Value::Object(args);
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

    let (script_hash, script_path, raw_code_tuple, job_kind, mut raw_flow, language) =
        match job_payload {
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
                let script = get_hub_script(path.clone(), email, user).await?;
                (
                    None,
                    Some(path),
                    Some((script.content, script.lockfile)),
                    JobKind::Script_Hub,
                    None,
                    Some(script.language),
                )
            }
            JobPayload::Code(RawCode { content, path, language, lock }) => (
                None,
                path,
                Some((content, lock)),
                JobKind::Preview,
                None,
                Some(language),
            ),
            JobPayload::Dependencies { hash, dependencies, language } => (
                Some(hash.0),
                None,
                Some((dependencies, None)),
                JobKind::Dependencies,
                None,
                Some(language),
            ),
            JobPayload::FlowDependencies { path } => {
                let value_json = sqlx::query_scalar!(
                "SELECT value FROM flow WHERE path = $1 AND (workspace_id = $2 OR workspace_id = \
                 'starter')",
                path,
                workspace_id
            )
                .fetch_optional(&mut tx)
                .await?
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
                    Some(value),
                    None,
                )
            }
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
            JobPayload::Identity => (None, None, None, JobKind::Identity, None, None),
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

        // If last module has a sleep or suspend, we insert a virtual identity module
        if flow.modules.len() > 0
            && (flow.modules[flow.modules.len() - 1].sleep.is_some()
                || flow.modules[flow.modules.len() - 1].suspend.is_some())
        {
            let mut modules = flow.modules.clone();
            modules.push(FlowModule {
                id: "".to_string(),
                value: FlowModuleValue::Identity,
                input_transforms: HashMap::new(),
                stop_after_if: None,
                summary: Some(
                    "Virtual module needed for suspend/sleep when last module".to_string(),
                ),
                retry: None,
                sleep: None,
                suspend: None,
            });
            raw_flow = Some(FlowValue { modules, ..flow.clone() });
        }
    }

    let (raw_code, raw_lock) = raw_code_tuple
        .map(|e| (Some(e.0), e.1))
        .unwrap_or_else(|| (None, None));

    let flow_status = raw_flow.as_ref().map(FlowStatus::new);
    let uuid = sqlx::query_scalar!(
        "INSERT INTO queue
            (workspace_id, id, running, parent_job, created_by, permissioned_as, scheduled_for, 
                script_hash, script_path, raw_code, raw_lock, args, job_kind, schedule_path, raw_flow, \
         flow_status, is_flow_step, language, started_at, same_worker, pre_run_error)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, CASE WHEN $3 THEN now() END, $19, $20) \
         RETURNING id",
        workspace_id,
        job_id,
        is_running,
        parent_job,
        user,
        permissioned_as,
        scheduled_for,
        script_hash,
        script_path.clone(),
        raw_code,
        raw_lock,
        args_json,
        job_kind: JobKind,
        schedule_path,
        raw_flow.map(|f| serde_json::json!(f)),
        flow_status.map(|f| serde_json::json!(f)),
        is_flow_step,
        language: ScriptLang,
        same_worker,
        pre_run_error.map(|e| e.to_string())
    )
    .fetch_one(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("Could not insert into queue {job_id}: {e}")))?;
    // TODO: technically the job isn't queued yet, as the transaction can be rolled back. Should be solved when moving these metrics to the queue abstraction.
    QUEUE_PUSH_COUNT.inc();

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
            JobKind::FlowDependencies => "jobs.run.flow_dependencies",
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

pub fn canceled_job_to_result(job: &QueuedJob) -> String {
    let reason = job
        .canceled_reason
        .as_deref()
        .unwrap_or_else(|| "no reason given");
    let canceler = job.canceled_by.as_deref().unwrap_or_else(|| "unknown");
    format!("Job canceled: {reason} by {canceler}")
}

pub async fn get_hub_script(
    path: String,
    email: Option<String>,
    user: &str,
) -> error::Result<HubScript> {
    get_full_hub_script_by_path(
        email,
        user.to_string(),
        StripPath(path),
        reqwest::ClientBuilder::new()
            .user_agent("windmill/beta")
            .build()
            .map_err(to_anyhow)?,
        std::env::var("BASE_URL").unwrap_or_else(|_| "".to_string()),
    )
    .await
    .map(|e| e)
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
    pub raw_lock: Option<String>,
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
    pub same_worker: bool,
    pub pre_run_error: Option<String>,
}

impl QueuedJob {
    pub fn script_path(&self) -> &str {
        self.script_path
            .as_ref()
            .map(String::as_str)
            .unwrap_or("NO_FLOW_PATH")
    }
}

impl QueuedJob {
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
    Identity,
    FlowDependencies,
}

#[derive(Debug, Clone)]
pub enum JobPayload {
    ScriptHub { path: String },
    ScriptHash { hash: ScriptHash, path: String },
    Code(RawCode),
    Dependencies { hash: ScriptHash, dependencies: String, language: ScriptLang },
    FlowDependencies { path: String },
    Flow(String),
    RawFlow { value: FlowValue, path: Option<String> },
    Identity,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RawCode {
    pub content: String,
    pub path: Option<String>,
    pub language: ScriptLang,
    pub lock: Option<String>,
}
