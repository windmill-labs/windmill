use std::collections::HashMap;
use std::time::Duration;

use crate::{
    db::DB,
    error::{self, Error},
    flows::{FlowModule, FlowModuleValue, FlowValue, InputTransform, Retry},
    jobs::{
        add_completed_job, add_completed_job_error, get_queued_job, postprocess_queued_job, push,
        script_path_to_payload, JobPayload, QueuedJob,
    },
    js_eval::{eval_timeout, EvalCreds},
    more_serde::is_default,
    users::create_token_for_owner,
    worker,
};
use anyhow::Context;
use async_recursion::async_recursion;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tokio::sync::mpsc::Sender;
use tracing::instrument;
use uuid::Uuid;

const MINUTES: Duration = Duration::from_secs(60);
const HOURS: Duration = MINUTES.saturating_mul(60);

pub const MAX_RETRY_ATTEMPTS: u16 = 1000;
pub const MAX_RETRY_INTERVAL: Duration = HOURS.saturating_mul(6);

#[derive(Serialize, Deserialize, Debug)]
pub struct FlowStatus {
    pub step: i32,
    pub modules: Vec<FlowStatusModule>,
    pub failure_module: FlowStatusModule,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default")]
    pub retry: RetryStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(default)]
pub struct RetryStatus {
    pub fail_count: u16,
    pub previous_result: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Iterator {
    pub index: usize,
    pub itered: Vec<Value>,
    pub args: Map<String, Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BranchChosen {
    Default,
    Branch(usize),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum FlowStatusModule {
    WaitingForPriorSteps,
    WaitingForEvents {
        count: u16,
        job: Uuid,
    },
    WaitingForExecutor {
        job: Uuid,
    },
    InProgress {
        job: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        iterator: Option<Iterator>,
        #[serde(skip_serializing_if = "Option::is_none")]
        forloop_jobs: Option<Vec<Uuid>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch_chosen: Option<BranchChosen>,
    },
    Success {
        job: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        forloop_jobs: Option<Vec<Uuid>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch_chosen: Option<BranchChosen>,
    },
    Failure {
        job: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        forloop_jobs: Option<Vec<Uuid>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch_chosen: Option<BranchChosen>,
    },
}

impl FlowStatus {
    pub fn new(f: &FlowValue) -> Self {
        Self {
            step: 0,
            modules: vec![FlowStatusModule::WaitingForPriorSteps; f.modules.len()],
            failure_module: FlowStatusModule::WaitingForPriorSteps,
            retry: RetryStatus { fail_count: 0, previous_result: None },
        }
    }

    /// current module status ... excluding failure_module
    pub fn current_step(&self) -> Option<&FlowStatusModule> {
        let i = usize::try_from(self.step).ok()?;
        self.modules.get(i)
    }
}

#[async_recursion]
#[instrument(level = "trace", skip_all)]
pub async fn update_flow_status_after_job_completion(
    db: &DB,
    job: &QueuedJob,
    success: bool,
    result: serde_json::Value,
    metrics: Option<worker::Metrics>,
    unrecoverable: bool,
    same_worker_tx: Sender<Uuid>,
    worker_dir: &str,
    keep_job_dir: bool,
) -> error::Result<()> {
    tracing::debug!("UPDATE FLOW STATUS: {job:?} {success} {result:?}");

    let mut tx = db.begin().await?;

    let w_id = &job.workspace_id;

    let flow = job
        .parent_job
        .ok_or_else(|| Error::InternalErr(format!("expected parent job")))?;

    let old_status_json = sqlx::query_scalar!(
        "SELECT flow_status FROM queue WHERE id = $1 AND workspace_id = $2",
        flow,
        w_id
    )
    .fetch_one(&mut tx)
    .await
    .map_err(|e| {
        Error::InternalErr(format!(
            "fetching flow status {flow} while reporting {success} {result:?}: {e}"
        ))
    })?
    .ok_or_else(|| Error::InternalErr(format!("requiring a previous status")))?;

    let old_status = serde_json::from_value::<FlowStatus>(old_status_json)
        .ok()
        .ok_or_else(|| {
            Error::InternalErr(format!("requiring status to be parsabled as FlowStatus"))
        })?;

    let skip_loop_failures = skip_loop_failures(flow, old_status.step, &mut tx)
        .await?
        .unwrap_or(false);

    let module_index = usize::try_from(old_status.step).ok();
    let module = module_index
        .and_then(|i| old_status.modules.get(i))
        .unwrap_or(&old_status.failure_module);

    let (step_counter, new_status) = match module {
        FlowStatusModule::InProgress { iterator: Some(Iterator { index, itered, .. }), .. }
            if *index + 1 < itered.len() && (success || skip_loop_failures) =>
        {
            (old_status.step, module.clone())
        }
        _ => {
            let forloop_jobs = match module {
                FlowStatusModule::InProgress { forloop_jobs, .. } => forloop_jobs.clone(),
                _ => None,
            };
            if success || (forloop_jobs.is_some() && skip_loop_failures) {
                (
                    old_status.step + 1,
                    FlowStatusModule::Success { job: job.id, forloop_jobs, branch_chosen: None },
                )
            } else {
                (
                    old_status.step,
                    FlowStatusModule::Failure { job: job.id, forloop_jobs, branch_chosen: None },
                )
            }
        }
    };

    /* is_last_step is true when the step_counter (the next step index) is an invalid index */
    let is_last_step = usize::try_from(step_counter)
        .map(|i| !(..old_status.modules.len()).contains(&i))
        .unwrap_or(true);

    let (stop_early_expr, skip_if_stop_early) =
        sqlx::query_as::<_, (Option<String>, Option<bool>)>(
            "
            UPDATE queue
               SET flow_status = JSONB_SET(
                                 JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2),
                                                        ARRAY['step'], $3)
             WHERE id = $4
            RETURNING
                (raw_flow->'modules'->$1->'stop_after_if'->>'expr'),
                (raw_flow->'modules'->$1->'stop_after_if'->>'skip_if_stopped')::bool
            ",
        )
        .bind(old_status.step)
        .bind(serde_json::json!(new_status))
        .bind(serde_json::json!(step_counter))
        .bind(flow)
        .fetch_one(&mut tx)
        .await
        .map_err(|e| Error::InternalErr(format!("retrieval of stop_early_expr from state: {e}")))?;

    let stop_early = success
        && if let Some(expr) = stop_early_expr.clone() {
            compute_bool_from_expr(expr, result.clone()).await?
        } else {
            false
        };

    let result = match &new_status {
        FlowStatusModule::Success { forloop_jobs: Some(jobs), .. } => {
            let results = sqlx::query_as(
                "
                  SELECT result
                    FROM completed_job
                   WHERE id = ANY($1)
                     AND workspace_id = $2
                     AND success = true
                ORDER BY args->'iter'->'index'
                    ",
            )
            .bind(jobs.as_slice())
            .bind(w_id)
            .fetch(&mut tx)
            .map_ok(|(v,)| v)
            .try_collect::<Vec<Value>>()
            .await?;
            json!(results)
        }
        _ => result,
    };

    if matches!(&new_status, FlowStatusModule::Success { .. }) {
        sqlx::query(
            "
            UPDATE queue
               SET flow_status = flow_status - 'retry'
             WHERE id = $1
             RETURNING flow_status
            ",
        )
        .bind(flow)
        .execute(&mut tx)
        .await
        .context("remove flow status retry")?;
    }

    let flow_job = get_queued_job(flow, w_id, &mut tx)
        .await?
        .ok_or_else(|| Error::InternalErr(format!("requiring flow to be in the queue")))?;

    let should_continue_flow = match success {
        _ if stop_early => false,
        _ if flow_job.canceled => false,
        true => !is_last_step,
        false if unrecoverable => false,
        false if skip_loop_failures => !is_last_step,
        false
            if next_retry(
                &flow_job
                    .parse_raw_flow_retry(module_index)
                    .unwrap_or_default(),
                &flow_job.parse_flow_status_retry().unwrap_or_default(),
            )
            .is_some() =>
        {
            true
        }
        false if has_failure_module(flow, &mut tx).await? => true,
        false => false,
    };
    tx.commit().await?;

    let done = if !should_continue_flow {
        let logs = if flow_job.canceled {
            "Flow job canceled".to_string()
        } else if stop_early {
            let stop_early_expr = stop_early_expr.unwrap();
            format!("Flow job stopped early based on the stop_early_expr predicate: {stop_early_expr} returning true")
        } else {
            "Flow job completed".to_string()
        };
        tracing::debug!("{skip_if_stop_early:?}");
        add_completed_job(
            db,
            &flow_job,
            success,
            stop_early && skip_if_stop_early.unwrap_or(false),
            result.clone(),
            logs,
        )
        .await?;
        true
    } else {
        match handle_flow(&flow_job, db, result.clone(), same_worker_tx.clone()).await {
            Err(err) => {
                let _ = add_completed_job_error(
                    db,
                    &flow_job,
                    "Unexpected error during flow chaining:\n".to_string(),
                    err,
                    metrics.clone(),
                )
                .await;
                true
            }
            Ok(_) => false,
        }
    };

    if done {
        postprocess_queued_job(
            flow_job.is_flow_step,
            flow_job.schedule_path.clone(),
            flow_job.script_path.clone(),
            &w_id,
            flow,
            db,
        )
        .await?;

        if flow_job.same_worker && !keep_job_dir {
            let _ = tokio::fs::remove_dir_all(format!("{worker_dir}/{}", flow_job.id)).await;
        }

        if flow_job.parent_job.is_some() {
            return Ok(update_flow_status_after_job_completion(
                db,
                &flow_job,
                success,
                result,
                metrics,
                false,
                same_worker_tx.clone(),
                worker_dir,
                keep_job_dir,
            )
            .await?);
        }
    }

    Ok(())
}

async fn skip_loop_failures<'c>(
    flow: Uuid,
    step: i32,
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
) -> Result<Option<bool>, Error> {
    sqlx::query_as(
        "
    SELECT (raw_flow->'modules'->$1->'value'->>'skip_failures')::bool
      FROM queue
     WHERE id = $2
        ",
    )
    .bind(step)
    .bind(flow)
    .fetch_one(tx)
    .await
    .map(|(v,)| v)
    .map_err(|e| Error::InternalErr(format!("error during retrieval of skip_loop_failures: {e}")))
}

async fn has_failure_module<'c>(
    flow: Uuid,
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
) -> Result<bool, Error> {
    sqlx::query_scalar(
        "
    SELECT raw_flow->'failure_module' != 'null'::jsonb
      FROM queue
     WHERE id = $1
        ",
    )
    .bind(flow)
    .fetch_one(tx)
    .await
    .map_err(|e| Error::InternalErr(format!("error during retrieval of has_failure_module: {e}")))
}

fn next_retry(retry: &Retry, status: &RetryStatus) -> Option<(u16, Duration)> {
    (status.fail_count <= MAX_RETRY_ATTEMPTS)
        .then(|| &retry)
        .and_then(|retry| retry.interval(status.fail_count))
        .map(|d| (status.fail_count + 1, std::cmp::min(d, MAX_RETRY_INTERVAL)))
}

async fn compute_bool_from_expr(expr: String, result: serde_json::Value) -> error::Result<bool> {
    match eval_timeout(
        expr,
        [
            ("result".to_string(), result.clone()),
            ("previous_result".to_string(), result),
        ]
        .into(),
        None,
        vec![],
    )
    .await?
    {
        serde_json::Value::Bool(true) => Ok(true),
        serde_json::Value::Bool(false) => Ok(false),
        a @ _ => Err(Error::ExecutionErr(format!(
            "Expected a boolean value, found: {a:?}"
        ))),
    }
}

pub fn init_flow_status(f: &FlowValue) -> FlowStatus {
    FlowStatus::new(f)
}

pub async fn update_flow_status_in_progress(
    db: &DB,
    w_id: &str,
    flow: Uuid,
    job_in_progress: Uuid,
) -> error::Result<()> {
    let step = get_step_of_flow_status(db, flow).await?;
    sqlx::query(&format!(
        "UPDATE queue
            SET flow_status = jsonb_set(jsonb_set(flow_status, '{{modules, {step}, job}}', $1), '{{modules, {step}, type}}', $2)
            WHERE id = $3 AND workspace_id = $4",
    ))
    .bind(json!(job_in_progress.to_string()))
    .bind(json!("InProgress"))
    .bind(flow)
    .bind(w_id)
    .execute(db)
    .await?;
    Ok(())
}

#[instrument(level = "trace", skip_all)]
pub async fn get_step_of_flow_status(db: &DB, id: Uuid) -> error::Result<i32> {
    let r = sqlx::query_scalar!(
        "SELECT (flow_status->'step')::integer FROM queue WHERE id = $1",
        id
    )
    .fetch_one(db)
    .await
    .map_err(|e| Error::InternalErr(format!("fetching step flow status: {e}")))?
    .ok_or_else(|| Error::InternalErr(format!("not found step")))?;
    Ok(r)
}

/// resumes should be in order of timestamp ascending, so that more recent are at the end
#[instrument(level = "trace", skip_all)]
async fn transform_input(
    flow_args: &Option<serde_json::Value>,
    last_result: serde_json::Value,
    input_transforms: &HashMap<String, InputTransform>,
    workspace: &str,
    token: &str,
    steps: Vec<String>,
    resumes: &[Value],
) -> anyhow::Result<Map<String, serde_json::Value>> {
    let mut mapped = serde_json::Map::new();

    for (key, val) in input_transforms.into_iter() {
        if let InputTransform::Static { value } = val {
            mapped.insert(key.to_string(), value.to_owned());
        }
    }

    for (key, val) in input_transforms.into_iter() {
        match val {
            InputTransform::Static { value: _ } => (),
            InputTransform::Javascript { expr } => {
                let previous_result = last_result.clone();
                let flow_input = flow_args.clone().unwrap_or_else(|| json!({}));
                let context = vec![
                    ("params".to_string(), serde_json::json!(mapped)),
                    ("previous_result".to_string(), previous_result),
                    ("flow_input".to_string(), flow_input),
                    (
                        "resume".to_string(),
                        resumes.last().map(|v| json!(v)).unwrap_or_default(),
                    ),
                    ("resumes".to_string(), resumes.clone().into()),
                ];

                let v = eval_timeout(
                    expr.to_string(),
                    context,
                    Some(EvalCreds { workspace: workspace.to_string(), token: token.to_string() }),
                    steps.clone(),
                )
                .await
                .map_err(|e| {
                    Error::ExecutionErr(format!(
                        "Error during isolated evaluation of expression `{expr}`:\n{e}"
                    ))
                })?;
                mapped.insert(key.to_string(), v);
                ()
            }
        }
    }

    Ok(mapped)
}

#[instrument(level = "trace", skip_all)]
pub async fn handle_flow(
    flow_job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    last_result: serde_json::Value,
    same_worker_tx: Sender<Uuid>,
) -> anyhow::Result<()> {
    let value = flow_job
        .raw_flow
        .as_ref()
        .ok_or_else(|| Error::InternalErr(format!("requiring a raw flow value")))?
        .to_owned();
    let flow = serde_json::from_value::<FlowValue>(value.to_owned())?;

    let status: FlowStatus =
        serde_json::from_value::<FlowStatus>(flow_job.flow_status.clone().unwrap_or_default())
            .with_context(|| format!("parse flow status {}", flow_job.id))?;

    tracing::debug!("handle_flow: {:#?} {:#?}", status, flow);
    push_next_flow_job(flow_job, status, flow, db, last_result, same_worker_tx).await?;
    Ok(())
}

#[async_recursion]
#[instrument(level = "trace", skip_all)]
async fn push_next_flow_job(
    flow_job: &QueuedJob,
    mut status: FlowStatus,
    mut flow: FlowValue,
    db: &sqlx::Pool<sqlx::Postgres>,
    mut last_result: serde_json::Value,
    same_worker_tx: Sender<Uuid>,
) -> anyhow::Result<()> {
    /* `mut` because reassigned on FlowStatusModule::Failure when failure_module is Some */
    let mut i = usize::try_from(status.step)
        .with_context(|| format!("invalid module index {}", status.step))?;

    let mut module: &FlowModule = flow
        .modules
        .get(i)
        .or_else(|| flow.failure_module.as_ref())
        .with_context(|| format!("no module at index {}", status.step))?;

    // calculate sleep if any
    let mut scheduled_for_o = {
        let sleep_input_transform = i
            .checked_sub(1)
            .and_then(|i| flow.modules.get(i))
            .and_then(|m| m.sleep.clone());

        if let Some(it) = sleep_input_transform {
            let json_value = match it {
                InputTransform::Static { value } => value,
                InputTransform::Javascript { expr } => eval_timeout(
                    expr.to_string(),
                    [("result".to_string(), last_result.clone())].into(),
                    None,
                    vec![],
                )
                .await
                .map_err(|e| {
                    Error::ExecutionErr(format!(
                        "Error during isolated evaluation of expression `{expr}`:\n{e}"
                    ))
                })?,
            };
            match json_value {
                serde_json::Value::Number(n) => {
                    n.as_u64().map(|x| from_now(Duration::from_secs(x)))
                }
                _ => Err(Error::ExecutionErr(format!(
                    "Expected an array value, found: {json_value}"
                )))?,
            }
        } else {
            None
        }
    };

    let mut status_module: FlowStatusModule = status
        .modules
        .get(i)
        .cloned()
        .unwrap_or_else(|| status.failure_module.clone());

    tracing::debug!(
        "push_next_flow_job: module: {:#?}, status: {:#?}",
        module.value,
        status_module
    );

    let mut resume_messages: Vec<Value> = vec![];

    /* (suspend / resume), when starting a module, if previous module has a
     * non-zero `suspend` value, collect `resume_job`s for the previous module job.
     *
     * If there aren't enough, try again later. */
    if matches!(
        &status_module,
        FlowStatusModule::WaitingForPriorSteps | FlowStatusModule::WaitingForEvents { .. }
    ) {
        if let Some((count, last)) = needs_resume(&flow, &status) {
            let mut tx = db.begin().await?;

            /* Lock this row to prevent the suspend column getting out out of sync
             * if a resume message arrives after we fetch and count them here.
             *
             * This only works because jobs::resume_job does the same thing. */
            sqlx::query_scalar!(
                "SELECT null FROM queue WHERE id = $1 FOR UPDATE",
                flow_job.id
            )
            .fetch_one(&mut tx)
            .await
            .context("lock flow in queue")?;

            let resumes = sqlx::query!(
                "SELECT value, is_cancel FROM resume_job WHERE job = $1 ORDER BY created_at ASC",
                last
            )
            .fetch_all(&mut tx)
            .await?;

            let is_cancelled = resumes
                .iter()
                .find(|r| r.is_cancel)
                .map(|r| r.value.clone());

            resume_messages.extend(resumes.into_iter().map(|r| r.value));

            if is_cancelled.is_none() && resume_messages.len() >= count as usize {
                /* If we are woken up after suspending, last_result will be the flow args, but we
                 * should use the result from the last job */
                if let FlowStatusModule::WaitingForEvents { .. } = &status_module {
                    last_result =
                        sqlx::query_scalar!("SELECT result FROM completed_job WHERE id = $1", last)
                            .fetch_one(&mut tx)
                            .await?
                            .context("previous job result")?;
                }

                /* continue on and run this job! */
                tx.commit().await?;

            /* not enough messages to do this job, "park"/suspend until there are */
            } else if is_cancelled.is_none()
                && matches!(&status_module, FlowStatusModule::WaitingForPriorSteps)
            {
                sqlx::query(
                    "
                    UPDATE queue
                       SET flow_status = JSONB_SET(flow_status, ARRAY['modules', flow_status->>'step'::text], $1)
                         , suspend = $2
                         , suspend_until = now() + $3
                     WHERE id = $4
                    ",
                )
                .bind(json!(FlowStatusModule::WaitingForEvents { count, job: last }))
                .bind(count as i32)
                .bind(30 * MINUTES)
                .bind(flow_job.id)
                .execute(&mut tx)
                .await?;

                tx.commit().await?;
                return Ok(());

            /* cancelled or we're WaitingForEvents but we don't have enough messages (timed out) */
            } else {
                tx.commit().await?;

                let success = false;
                let skipped = false;
                let logs = if is_cancelled.is_some() {
                    "Cancelled while waiting to be resumed"
                } else {
                    "Timed out waiting to be resumed"
                }
                .to_string();
                let result = is_cancelled.unwrap_or(json!({ "error": logs }));
                let _uuid =
                    add_completed_job(db, &flow_job, success, skipped, result, logs).await?;
                postprocess_queued_job(
                    false,
                    flow_job.schedule_path.clone(),
                    flow_job.script_path.clone(),
                    &flow_job.workspace_id,
                    flow_job.id,
                    db,
                )
                .await?;

                return Ok(());
            }
        }
    }

    if matches!(&status_module, FlowStatusModule::Failure { .. }) {
        let retry = &module.retry.clone().unwrap_or_default();
        if let Some((fail_count, retry_in)) = next_retry(retry, &status.retry) {
            tracing::debug!(
                retry_in_seconds = retry_in.as_secs(),
                fail_count = fail_count,
                "retrying"
            );

            scheduled_for_o = Some(from_now(retry_in));

            sqlx::query(
                "
                UPDATE queue
                   SET flow_status = JSONB_SET(flow_status, ARRAY['retry'], $1)
                 WHERE id = $2
                ",
            )
            .bind(json!(RetryStatus { fail_count, ..status.retry.clone() }))
            .bind(flow_job.id)
            .execute(db)
            .await
            .context("update flow retry")?;

            /* it might be better to retry the job using the previous args instead of determining
             * them again from the last result, but that seemed to not play well with the forloop
             * logic and I couldn't figure out why. */
            if let Some(v) = &status.retry.previous_result {
                last_result = v.clone();
            }
            status_module = FlowStatusModule::WaitingForPriorSteps;

        /* Start the failure module ... */
        } else {
            /* push_next_flow_job is called with the current step on FlowStatusModule::Failure.
             * This must update the step index to the end so that no subsequent steps are run after
             * the failure module.
             *
             * The failure module may also run again if it fails and the retry feature is used.
             * In that case, `i` will index past `flow.modules`.  The above should handle that and
             * re-run the failure module. */
            i = flow.modules.len();
            module = flow
                .failure_module
                .as_ref()
                /* If this fails, it's a update_flow_status_after_job_completion shouldn't have called
                 * handle_flow to get here. */
                .context("missing failure module")?;
            status_module = status.failure_module.clone();

            /* (retry feature) save the previous_result the first time this step is run */
            let retry = &module.retry.clone().unwrap_or_default();
            if retry.has_attempts() {
                sqlx::query(
                    "
                UPDATE queue
                   SET flow_status = JSONB_SET(flow_status, ARRAY['retry'], $1)
                 WHERE id = $2
                ",
                )
                .bind(json!(RetryStatus {
                    previous_result: Some(last_result.clone()),
                    fail_count: 0,
                }))
                .bind(flow_job.id)
                .execute(db)
                .await
                .context("update flow retry")?;
            };
        }

    /* (retry feature) save the previous_result the first time this step is run */
    } else if matches!(&status_module, FlowStatusModule::WaitingForPriorSteps)
        && module
            .retry
            .as_ref()
            .map(|x| x.has_attempts())
            .unwrap_or(false)
        && status.retry.fail_count == 0
    {
        sqlx::query(
            "
            UPDATE queue
               SET flow_status = JSONB_SET(flow_status, ARRAY['retry'], $1)
             WHERE id = $2
            ",
        )
        .bind(json!(RetryStatus {
            previous_result: Some(last_result.clone()),
            fail_count: 0,
        }))
        .bind(flow_job.id)
        .execute(db)
        .await
        .context("update flow retry")?;
    }

    /* Don't evaluate `module.input_transforms` after iteration has begun.  Instead, args are
     * carried through the Iterator by the InProgress variant. */

    #[rustfmt::skip]
    let compute_input_transform = !(   matches!(&module.value, FlowModuleValue::ForloopFlow { .. })
                                    && matches!(&status_module, FlowStatusModule::InProgress { .. }));

    let mut transform_context: Option<(String, Vec<String>)> = None;
    let mut args = if compute_input_transform {
        transform_context = Some(get_transform_context(&db, &flow_job, &status).await?);
        let (token, steps) = transform_context.as_ref().unwrap();
        transform_input(
            &flow_job.args,
            last_result.clone(),
            &module.input_transforms,
            &flow_job.workspace_id,
            &token,
            steps.to_vec(),
            resume_messages.as_slice(),
        )
        .await?
    } else {
        Map::new()
    };

    let next_flow_transform = compute_next_flow_transform(
        flow_job,
        &flow,
        transform_context,
        &db,
        &module,
        &status,
        &status_module,
        last_result.clone(),
        &mut args,
    )
    .await?;

    let (job_payload, next_status) = match next_flow_transform {
        NextFlowTransform::Continue(job_payload, next_state) => (job_payload, next_state),
        NextFlowTransform::BranchChosen(branch, modules) => {
            let added_flow_statuses = vec![FlowStatusModule::WaitingForPriorSteps; modules.len()];

            let index = (status.step + 1) as usize;
            status.modules.splice(index..index, added_flow_statuses);
            flow.modules.splice(index..index, modules);

            let mut tx = db.begin().await?;
            sqlx::query!(
                "
                UPDATE queue
                   SET flow_status = JSONB_SET(flow_status, ARRAY['modules'], $1),
                       raw_flow = JSONB_SET(raw_flow, ARRAY['modules'], $2)
                 WHERE id = $3
                  ",
                json!(status.modules),
                json!(flow.modules),
                flow_job.id
            )
            .execute(&mut tx)
            .await?;

            return jump_to_next_step(
                status.step,
                i,
                &flow_job.id,
                flow,
                tx,
                &db,
                FlowStatusModule::Success {
                    job: flow_job.id,
                    forloop_jobs: None,
                    branch_chosen: Some(branch),
                },
                last_result,
                same_worker_tx,
            )
            .await;
        }
        NextFlowTransform::EmptyIterator => {
            let tx = db.begin().await?;

            return jump_to_next_step(
                status.step,
                i,
                &flow_job.id,
                flow.clone(),
                tx,
                &db,
                FlowStatusModule::Success {
                    job: flow_job.id,
                    forloop_jobs: Some(vec![]),
                    branch_chosen: None,
                },
                json!([]),
                same_worker_tx,
            )
            .await;
        }
    };

    let continue_on_same_worker =
        flow.same_worker && !matches!(job_payload, JobPayload::RawFlow { .. });

    /* Finally, push the job into the queue */
    let tx = db.begin().await?;

    let (uuid, mut tx) = push(
        tx,
        &flow_job.workspace_id,
        job_payload,
        Some(args.clone()),
        &flow_job.created_by,
        flow_job.permissioned_as.to_owned(),
        scheduled_for_o,
        flow_job.schedule_path.clone(),
        Some(flow_job.id),
        true,
        continue_on_same_worker,
    )
    .await?;

    let new_status = match next_status {
        NextStatus::NextLoopIteration(NextIteration { index, itered, mut forloop_jobs }) => {
            forloop_jobs.push(uuid);

            FlowStatusModule::InProgress {
                job: uuid,
                iterator: Some(Iterator { index, itered, args }),
                forloop_jobs: Some(forloop_jobs),
                branch_chosen: None,
            }
        }
        _ => FlowStatusModule::WaitingForExecutor { job: uuid },
    };

    sqlx::query(
        "
        UPDATE queue
           SET flow_status = JSONB_SET(
                             JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2),
                                                    ARRAY['step'], $3)
         WHERE id = $4
          ",
    )
    .bind(status.step)
    .bind(json!(new_status))
    .bind(json!(i))
    .bind(flow_job.id)
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    if continue_on_same_worker {
        same_worker_tx.send(uuid).await?;
    }
    return Ok(());
}

async fn jump_to_next_step<'c>(
    status_step: i32,
    i: usize,
    job_id: &Uuid,
    flow: FlowValue,
    mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
    db: &DB,
    status_module: FlowStatusModule,
    last_result: serde_json::Value,
    same_worker_tx: Sender<Uuid>,
) -> anyhow::Result<()> {
    let next_step = i
        .checked_add(1)
        .filter(|i| (..flow.modules.len()).contains(i));

    let new_job = sqlx::query_as::<_, QueuedJob>(
        r#"
                UPDATE queue
                    SET flow_status = JSONB_SET(
                                      JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2),
                                                               ARRAY['step'], $3)
                    WHERE id = $4
                RETURNING *
                "#,
    )
    .bind(status_step)
    .bind(json!(status_module))
    .bind(json!(next_step.unwrap_or(i)))
    .bind(job_id)
    .fetch_one(&mut tx)
    .await?;

    tx.commit().await?;

    let new_status = new_job.parse_flow_status().ok_or_else(|| {
        Error::ExecutionErr("Impossible to parse new status after jump".to_string())
    })?;

    if next_step.is_some() {
        tracing::debug!("Jumping to next step with flow {flow:#?}");
        return push_next_flow_job(&new_job, new_status, flow, db, last_result, same_worker_tx)
            .await;
    } else {
        let success = true;
        let skipped = false;
        let logs = "Forloop completed without iteration".to_string();
        let _uuid = add_completed_job(db, &new_job, success, skipped, json!([]), logs).await?;
        postprocess_queued_job(
            false,
            new_job.schedule_path,
            new_job.script_path,
            &new_job.workspace_id,
            new_job.id,
            db,
        )
        .await?;
        return Ok(());
    }
}

/// Some state about the current/last forloop FlowStatusModule used to initialized the next
/// iteration's FlowStatusModule after pushing a job
struct NextIteration {
    index: usize,
    itered: Vec<Value>,
    forloop_jobs: Vec<Uuid>,
}

enum LoopStatus {
    NextIteration(NextIteration),
    EmptyIterator,
}

enum NextStatus {
    NextStep,
    NextLoopIteration(NextIteration),
}

enum NextFlowTransform {
    EmptyIterator,
    Continue(JobPayload, NextStatus),
    BranchChosen(BranchChosen, Vec<FlowModule>),
}

async fn compute_next_flow_transform(
    flow_job: &QueuedJob,
    flow: &FlowValue,
    transform_context: Option<(String, Vec<String>)>,
    db: &DB,
    module: &FlowModule,
    status: &FlowStatus,
    status_module: &FlowStatusModule,
    last_result: serde_json::Value,
    args: &mut Map<String, serde_json::Value>,
) -> error::Result<NextFlowTransform> {
    match &module.value {
        FlowModuleValue::Script { path: script_path } => Ok(NextFlowTransform::Continue(
            script_path_to_payload(script_path, &mut db.begin().await?, &flow_job.workspace_id)
                .await?,
            NextStatus::NextStep,
        )),
        FlowModuleValue::RawScript(raw_code) => {
            let mut raw_code = raw_code.clone();
            if raw_code.path.is_none() {
                raw_code.path = Some(format!("{}/{}", flow_job.script_path(), status.step));
            }
            Ok(NextFlowTransform::Continue(
                JobPayload::Code(raw_code),
                NextStatus::NextStep,
            ))
        }
        /* forloop modules are expected set `iter: { value: Value, index: usize }` as job arguments */
        FlowModuleValue::ForloopFlow { modules, iterator, .. } => {
            let next_loop_status = match status_module {
                FlowStatusModule::WaitingForPriorSteps => {
                    let (token, steps) = if let Some(x) = transform_context {
                        x
                    } else {
                        get_transform_context(&db, &flow_job, &status).await?
                    };
                    /* Iterator is an InputTransform, evaluate it into an array. */
                    let itered = iterator
                        .clone()
                        .evaluate_with(
                            || {
                                vec![
                                    ("result".to_string(), last_result.clone()),
                                    ("previous_result".to_string(), last_result.clone()),
                                ]
                            },
                            token,
                            flow_job.workspace_id.clone(),
                            steps,
                        )
                        .await?
                        .into_array()
                        .map_err(|not_array| {
                            Error::ExecutionErr(format!(
                                "Expected an array value, found: {not_array}"
                            ))
                        })?;

                    if let Some(first) = itered.first() {
                        args.insert("iter".to_string(), json!({ "index": 0, "value": first }));

                        LoopStatus::NextIteration(NextIteration {
                            index: 0,
                            itered,
                            forloop_jobs: vec![],
                        })
                    } else {
                        LoopStatus::EmptyIterator
                    }
                }

                FlowStatusModule::InProgress {
                    iterator: Some(Iterator { itered, index, args: iterator_args }),
                    forloop_jobs: Some(forloop_jobs),
                    ..
                } => {
                    let (index, next) = index
                        .checked_add(1)
                        .and_then(|i| itered.get(i).map(|next| (i, next)))
                        /* we shouldn't get here because update_flow_status_after_job_completion
                         * should leave this state if there iteration is complete, but also it should
                         * be reasonable to just enter a completed state instead of failing, similar to
                         * iterating an empty list above */
                        .with_context(|| {
                            format!("could not iterate index {index} of {itered:?}")
                        })?;

                    args.extend(iterator_args.clone());
                    args.insert("iter".to_string(), json!({ "index": index, "value": next }));

                    LoopStatus::NextIteration(NextIteration {
                        index,
                        itered: itered.clone(),
                        forloop_jobs: forloop_jobs.clone(),
                    })
                }

                _ => Err(Error::BadRequest(format!(
                    "Unrecognized module status for ForloopFlow {status_module:?}"
                )))?,
            };

            match next_loop_status {
                LoopStatus::EmptyIterator => Ok(NextFlowTransform::EmptyIterator),
                LoopStatus::NextIteration(ns) => {
                    /* embedded flow input is augmented with embedding flow input */
                    if let Some(value) = &flow_job.args {
                        value
                            .as_object()
                            .ok_or_else(|| {
                                Error::BadRequest(format!(
                                    "Expected an object value, found: {value:?}"
                                ))
                            })
                            .map(|map| args.extend(map.clone()))?;
                    }

                    Ok(NextFlowTransform::Continue(
                        JobPayload::RawFlow {
                            value: FlowValue {
                                modules: (*modules).clone(),
                                failure_module: flow.failure_module.clone(),
                                same_worker: flow.same_worker,
                            },
                            path: Some(format!("{}/loop-{}", flow_job.script_path(), status.step)),
                        },
                        NextStatus::NextLoopIteration(ns),
                    ))
                }
            }
        }
        FlowModuleValue::Branches { branches, default, .. } => {
            let branch = match status_module {
                FlowStatusModule::WaitingForPriorSteps => {
                    let mut branch_chosen = BranchChosen::Default;
                    for (i, b) in branches.iter().enumerate() {
                        let pred =
                            compute_bool_from_expr(b.expr.to_string(), last_result.clone()).await?;

                        if pred {
                            branch_chosen = BranchChosen::Branch(i);
                            break;
                        }
                    }
                    branch_chosen
                }
                _ => Err(Error::BadRequest(format!(
                    "Unrecognized module status for Branches {status_module:?}"
                )))?,
            };

            let modules = if let BranchChosen::Branch(index) = branch {
                &branches[index].modules
            } else {
                &default
            };

            // match inner_flow_transform {}

            Ok(NextFlowTransform::BranchChosen(branch, modules.clone()))
        }
    }
}

async fn get_transform_context(
    db: &DB,
    flow_job: &QueuedJob,
    status: &FlowStatus,
) -> error::Result<(String, Vec<String>)> {
    let new_token = create_token_for_owner(
        db,
        &flow_job.workspace_id,
        &flow_job.permissioned_as,
        "transform-input",
        10,
        &flow_job.created_by,
    )
    .await?;
    let new_steps = status
        .modules
        .iter()
        .map(|x| match x {
            FlowStatusModule::Success { job, .. } => job.to_string(),
            _ => "invalid step status".to_string(),
        })
        .collect();

    Ok((new_token, new_steps))
}

impl InputTransform {
    async fn evaluate_with<F>(
        self,
        vars: F,
        token: String,
        workspace: String,
        steps: Vec<String>,
    ) -> anyhow::Result<Value>
    where
        F: FnOnce() -> Vec<(String, Value)>,
    {
        match self {
            InputTransform::Static { value } => Ok(value),
            InputTransform::Javascript { expr } => {
                eval_timeout(expr, vars(), Some(EvalCreds { workspace, token }), steps).await
            }
        }
    }
}

impl QueuedJob {
    pub fn parse_raw_flow(&self) -> Option<FlowValue> {
        self.raw_flow
            .as_ref()
            .and_then(|v| serde_json::from_value::<FlowValue>(v.clone()).ok())
    }

    pub fn parse_raw_flow_retry(&self, mod_index: Option<usize>) -> Option<Retry> {
        self.parse_raw_flow().and_then(|module| {
            mod_index.and_then(|i| {
                module
                    .modules
                    .get(i)
                    .or(module.failure_module.as_ref())
                    .and_then(|m| m.retry.clone())
            })
        })
    }

    pub fn parse_flow_status(&self) -> Option<FlowStatus> {
        self.flow_status
            .as_ref()
            .and_then(|v| serde_json::from_value::<FlowStatus>(v.clone()).ok())
    }

    pub fn parse_flow_status_retry(&self) -> Option<RetryStatus> {
        self.parse_flow_status().map(|status| status.retry)
    }
}

trait IntoArray: Sized {
    fn into_array(self) -> Result<Vec<Value>, Self>;
}

impl IntoArray for Value {
    fn into_array(self) -> Result<Vec<Value>, Self> {
        match self {
            Value::Array(array) => Ok(array),
            not_array => Err(not_array),
        }
    }
}

fn from_now(duration: Duration) -> chrono::DateTime<chrono::Utc> {
    // "This function errors when original duration is larger than
    // the maximum value supported for this type."
    chrono::Duration::from_std(duration)
        .ok()
        .and_then(|d| chrono::Utc::now().checked_add_signed(d))
        .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC)
}

/// returns previous module non-zero suspend count and job
fn needs_resume(flow: &FlowValue, status: &FlowStatus) -> Option<(u16, Uuid)> {
    let prev = usize::try_from(status.step)
        .ok()
        .and_then(|s| s.checked_sub(1))?;

    let suspend = flow.modules.get(prev)?.suspend;
    if suspend == 0 {
        return None;
    }

    if let &FlowStatusModule::Success { job, .. } = status.modules.get(prev)? {
        Some((suspend, job))
    } else {
        None
    }
}
