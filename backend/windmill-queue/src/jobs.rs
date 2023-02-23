/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{collections::HashMap, str::FromStr};

use anyhow::Context;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Transaction};
use tracing::{instrument, Instrument};
use ulid::Ulid;
use uuid::Uuid;
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    error::{self, Error},
    flow_status::{FlowStatus, JobResult, MAX_RETRY_ATTEMPTS, MAX_RETRY_INTERVAL},
    flows::{FlowModule, FlowModuleValue, FlowValue},
    scripts::{get_full_hub_script_by_path, HubScript, ScriptHash, ScriptLang},
    utils::StripPath,
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
    pub static ref CLOUD_HOSTED: bool = std::env::var("CLOUD_HOSTED").is_ok();

}

const MAX_FREE_EXECS: i32 = 1000;
const MAX_FREE_CONCURRENT_RUNS: i32 = 15;

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

pub async fn pull(
    db: &Pool<Postgres>,
    whitelist_workspaces: Option<Vec<String>>,
    blacklist_workspaces: Option<Vec<String>>,
) -> windmill_common::error::Result<Option<QueuedJob>> {
    let mut workspaces_filter = String::new();
    if let Some(whitelist) = whitelist_workspaces {
        workspaces_filter.push_str(&format!(
            " AND workspace_id IN ({})",
            whitelist
                .into_iter()
                .map(|x| format!("'{x}'"))
                .collect::<Vec<String>>()
                .join(",")
        ));
    }
    if let Some(blacklist) = blacklist_workspaces {
        workspaces_filter.push_str(&format!(
            " AND workspace_id NOT IN ({})",
            blacklist
                .into_iter()
                .map(|x| format!("'{x}'"))
                .collect::<Vec<String>>()
                .join(",")
        ));
    }
    /* Jobs can be started if they:
     * - haven't been started before,
     *   running = false
     * - are flows with a step that needed resume,
     *   suspend_until is non-null
     *   and suspend = 0 when the resume messages are received
     *   or suspend_until <= now() if it has timed out */
    let job: Option<QueuedJob> = sqlx::query_as::<_, QueuedJob>(&format!(
        "UPDATE queue
            SET running = true
              , started_at = coalesce(started_at, now())
              , last_ping = now()
              , suspend_until = null
            WHERE id = (
                SELECT id
                FROM queue
                WHERE ((running = false
                       AND scheduled_for <= now())
                   OR (suspend_until IS NOT NULL
                       AND (   suspend <= 0
                            OR suspend_until <= now()))) {workspaces_filter}
                ORDER BY scheduled_for
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            RETURNING *"
    ))
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
    email: &str,
    permissioned_as: String,
    scheduled_for_o: Option<chrono::DateTime<chrono::Utc>>,
    schedule_path: Option<String>,
    parent_job: Option<Uuid>,
    is_flow_step: bool,
    mut same_worker: bool,
    pre_run_error: Option<&windmill_common::error::Error>,
    visible_to_owner: bool,
) -> Result<(Uuid, Transaction<'c, Postgres>), Error> {
    let scheduled_for = scheduled_for_o.unwrap_or_else(chrono::Utc::now);
    let args_json = serde_json::Value::Object(args);
    let job_id: Uuid = Ulid::new().into();

    if cfg!(feature = "enterprise") {
        let premium_workspace =
            sqlx::query_scalar!("SELECT premium FROM workspace WHERE id = $1", workspace_id)
                .fetch_one(&mut tx)
                .await
                .map_err(|e| {
                    Error::InternalErr(format!("fetching if {workspace_id} is premium: {e}"))
                })?;

        // we track only non flow steps
        let usage = if !matches!(
            job_payload,
            JobPayload::Flow(_) | JobPayload::RawFlow { .. }
        ) {
            if !premium_workspace {
                sqlx::query_scalar!(
                    "INSERT INTO usage (id, is_workspace, month_, usage) 
                    VALUES ($1, false, EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date), 0) 
                    ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + 1 
                    RETURNING usage.usage",
                    email)
                .fetch_one(&mut tx)
                .await
                .map_err(|e| Error::InternalErr(format!("updating usage: {e}")))?
            } else {
                sqlx::query_scalar!(
                    "INSERT INTO usage (id, is_workspace, month_, usage) 
                    VALUES ($1, true, EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date), 0) 
                    ON CONFLICT (id, is_workspace, month_) DO UPDATE SET usage = usage.usage + 1 
                    RETURNING usage.usage",
                    workspace_id)
                .fetch_one(&mut tx)
                .await
                .map_err(|e| Error::InternalErr(format!("updating usage: {e}")))?
            }
        } else if *CLOUD_HOSTED && !premium_workspace {
            sqlx::query_scalar!(
                "
        SELECT usage.usage + 1 FROM usage 
        WHERE is_workspace = false AND
     month_ = EXTRACT(YEAR FROM current_date) * 12 + EXTRACT(MONTH FROM current_date)
     AND id = $1",
                email
            )
            .fetch_optional(&mut tx)
            .await?
            .flatten()
            .unwrap_or(0)
        } else {
            0
        };

        if *CLOUD_HOSTED && !premium_workspace {
            let is_super_admin =
                sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
                    .fetch_optional(&mut tx)
                    .await?
                    .unwrap_or(false);

            if !is_super_admin {
                if usage > MAX_FREE_EXECS {
                    return Err(error::Error::BadRequest(format!(
                    "User {email} has exceeded the free usage limit of {MAX_FREE_EXECS} that applies outside of premium workspaces."
                )));
                }
                let in_queue =
                    sqlx::query_scalar!("SELECT COUNT(id) FROM queue WHERE email = $1", email)
                        .fetch_one(&mut tx)
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
                .fetch_one(&mut tx)
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

    let (script_hash, script_path, raw_code_tuple, job_kind, mut raw_flow, language) =
        match job_payload {
            JobPayload::ScriptHash { hash, path } => {
                let language = sqlx::query_scalar!(
                    "SELECT language as \"language: ScriptLang\" FROM script WHERE hash = $1 AND workspace_id = $2",
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
                let script = get_hub_script(&HTTP_CLIENT, path.clone(), email)
                    .await
                    .context("error fetching hub script")?;
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
                    "SELECT value FROM flow WHERE path = $1 AND workspace_id = $2",
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
                "SELECT value FROM flow WHERE path = $1 AND workspace_id = $2",
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
                id: format!("{}-v", flow.modules[flow.modules.len() - 1].id),
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
         flow_status, is_flow_step, language, started_at, same_worker, pre_run_error, email, visible_to_owner)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, CASE WHEN $3 THEN now() END, $19, $20, $21, $22) \
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
        pre_run_error.map(|e| e.to_string()),
        email,
        visible_to_owner
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
        .instrument(tracing::info_span!("job_run", email = &email))
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

pub async fn get_hub_script(
    client: &reqwest::Client,
    path: String,
    email: &str,
) -> error::Result<HubScript> {
    get_full_hub_script_by_path(email, StripPath(path), client)
        .await
        .map(|e| e)
}

#[derive(Debug, sqlx::FromRow, Serialize, Clone)]
pub struct QueuedJob {
    pub workspace_id: String,
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_job: Option<Uuid>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub scheduled_for: chrono::DateTime<chrono::Utc>,
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_hash: Option<ScriptHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_path: Option<String>,
    pub args: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_lock: Option<String>,
    pub canceled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canceled_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canceled_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_ping: Option<chrono::DateTime<chrono::Utc>>,
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
    pub same_worker: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_run_error: Option<String>,
    pub email: String,
    pub visible_to_owner: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspend: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem_peak: Option<i32>,
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
