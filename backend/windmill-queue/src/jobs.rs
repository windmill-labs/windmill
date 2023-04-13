/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use anyhow::Context;
use reqwest::Client;
use sqlx::{Pool, Postgres, Transaction};
use tracing::{instrument, Instrument};
use ulid::Ulid;
use uuid::Uuid;
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    error::{self, Error},
    flow_status::{FlowStatus, JobResult, MAX_RETRY_ATTEMPTS, MAX_RETRY_INTERVAL},
    flows::{FlowModule, FlowModuleValue, FlowValue},
    jobs::{JobKind, JobPayload, QueuedJob, RawCode},
    scripts::{get_full_hub_script_by_path, HubScript, ScriptHash, ScriptLang},
    utils::StripPath,
    METRICS_ENABLED,
};

use crate::QueueTransaction;

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
const RSMQ_MAIN_QUEUE: &'static str = "main_queue";

pub async fn cancel_job<'c, R: rsmq_async::RsmqConnection + Clone>(
    username: &str,
    reason: Option<String>,
    id: Uuid,
    w_id: &str,
    mut tx: Transaction<'c, Postgres>,
    rsmq: Option<R>,
    force_rerun: bool,
) -> error::Result<(Transaction<'c, Postgres>, Option<Uuid>)> {
    let job_option = sqlx::query_scalar!(
        "UPDATE queue SET  canceled = true, canceled_by = $1, canceled_reason = $2, scheduled_for = now(), suspend = 0, running = CASE WHEN $3 THEN false ELSE running END  WHERE id = $4 \
         AND workspace_id = $5 RETURNING id",
        username,
        reason,
        force_rerun,
        id,
        w_id
    )
    .fetch_optional(&mut tx)
    .await?;
    if let Some(mut rsmq) = rsmq {
        rsmq.change_message_visibility(RSMQ_MAIN_QUEUE, &id.to_string(), 0)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
    }

    let mut jobs = job_option.map(|j| vec![j]).unwrap_or_default();
    while !jobs.is_empty() {
        let p_job = jobs.pop();
        let new_jobs = sqlx::query_scalar!(
            "UPDATE queue SET canceled = true, canceled_by = $1, canceled_reason = $2, running = CASE WHEN $3 THEN false ELSE running END WHERE parent_job = $4 \
             AND workspace_id = $5 RETURNING id",
            username,
            reason,
            force_rerun,
            p_job,
            w_id
        )
        .fetch_all(&mut tx)
        .await?;
        jobs.extend(new_jobs);
    }
    Ok((tx, job_option))
}

pub async fn pull<R: rsmq_async::RsmqConnection + Clone>(
    db: &Pool<Postgres>,
    whitelist_workspaces: Option<Vec<String>>,
    blacklist_workspaces: Option<Vec<String>>,
    rsmq: Option<R>,
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
        if let Some(_rsmq) = rsmq {
            todo!("REDIS: Implement workspace filters for redis");
        }
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
        if let Some(_rsmq) = rsmq {
            todo!("REDIS: Implement workspace filters for redis");
        }
    }

    let job: Option<QueuedJob> = if let Some(mut rsmq) = rsmq {
        // TODO: REDIS: Race conditions / replace last_ping
        let msg = rsmq
            .pop_message::<Vec<u8>>(RSMQ_MAIN_QUEUE)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        if let Some(msg) = msg {
            let uuid = Uuid::from_bytes_le(
                msg.message
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Failed to parsed Redis message"))?,
            );

            sqlx::query_as::<_, QueuedJob>(
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
            .await?
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
        sqlx::query_as::<_, QueuedJob>(&format!(
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
        .await?
    };

    if job.is_some() && *METRICS_ENABLED {
        QUEUE_PULL_COUNT.inc();
    }

    Ok(job)
}

pub async fn get_result_by_id(
    db: Pool<Postgres>,
    w_id: String,
    flow_id: Uuid,
    node_id: String,
) -> error::Result<serde_json::Value> {
    let job_result: Option<JobResult> = sqlx::query_scalar!(
        "SELECT leaf_jobs->$1::text FROM queue WHERE COALESCE((SELECT root_job FROM queue WHERE id = $2), $2) = id AND workspace_id = $3",
        node_id,
        flow_id,
        w_id,
    )
    .fetch_optional(&db)
    .await?
    .flatten()
    .map(|x| serde_json::from_value(x).ok())
    .flatten();

    let result_id = windmill_common::utils::not_found_if_none(
        job_result,
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
    .fetch_one(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("Error during deletion of job {job_id}: {e}")))?
    .unwrap_or(0)
        == 1;
    tracing::debug!("Job {job_id} deleted: {job_removed}");
    Ok(tx)
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

// #[instrument(level = "trace", skip_all)]
pub async fn push<'c, R: rsmq_async::RsmqConnection + Send + 'c>(
    mut tx: QueueTransaction<'c, R>,
    workspace_id: &str,
    job_payload: JobPayload,
    args: serde_json::Map<String, serde_json::Value>,
    user: &str,
    email: &str,
    permissioned_as: String,
    scheduled_for_o: Option<chrono::DateTime<chrono::Utc>>,
    schedule_path: Option<String>,
    parent_job: Option<Uuid>,
    root_job: Option<Uuid>,
    is_flow_step: bool,
    mut same_worker: bool,
    pre_run_error: Option<&windmill_common::error::Error>,
    visible_to_owner: bool,
) -> Result<(Uuid, QueueTransaction<'c, R>), Error> {
    let args_json = serde_json::Value::Object(args);
    let job_id: Uuid = Ulid::new().into();

    if cfg!(feature = "enterprise") {
        let premium_workspace = *CLOUD_HOSTED
            && sqlx::query_scalar!("SELECT premium FROM workspace WHERE id = $1", workspace_id)
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
                if usage > MAX_FREE_EXECS
                    && !matches!(job_payload, JobPayload::Dependencies { .. })
                    && !matches!(job_payload, JobPayload::FlowDependencies { .. })
                {
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
         flow_status, is_flow_step, language, started_at, same_worker, pre_run_error, email, visible_to_owner, root_job)
            VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7, now()), $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, CASE WHEN $3 THEN now() END, $19, $20, $21, $22, $23) \
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
        visible_to_owner,
        root_job
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
    if let Some(ref mut rsmq) = tx.rsmq {
        rsmq.send_message(job_id.to_bytes_le().to_vec(), scheduled_for_o);
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
