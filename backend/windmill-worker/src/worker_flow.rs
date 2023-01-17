/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;
use std::iter;
use std::time::Duration;

use crate::jobs::{add_completed_job, add_completed_job_error, schedule_again_if_scheduled};
use crate::js_eval::{eval_timeout, EvalCreds, IdContext};
use crate::worker;
use anyhow::Context;
use async_recursion::async_recursion;
use dyn_iter::DynIter;
use serde_json::{json, Map, Value};
use tokio::sync::mpsc::Sender;
use tracing::instrument;
use uuid::Uuid;
use windmill_common::flow_status::{FlowStatusModuleWParent, Iterator, JobResult};
use windmill_common::{
    error::{self, to_anyhow, Error},
    flow_status::{
        Approval, BranchAllStatus, BranchChosen, FlowStatus, FlowStatusModule, RetryStatus,
        MAX_RETRY_ATTEMPTS, MAX_RETRY_INTERVAL,
    },
    flows::{FlowModule, FlowModuleValue, FlowValue, InputTransform, Retry, Suspend},
};

type DB = sqlx::Pool<sqlx::Postgres>;

use windmill_queue::{
    canceled_job_to_result, get_queued_job, push, JobPayload, QueuedJob, RawCode,
};

#[async_recursion]
// #[instrument(level = "trace", skip_all)]
pub async fn update_flow_status_after_job_completion(
    db: &DB,
    client: &windmill_api_client::Client,
    flow: uuid::Uuid,
    job_id_for_status: &Uuid,
    w_id: &str,
    success: bool,
    result: serde_json::Value,
    metrics: Option<worker::Metrics>,
    unrecoverable: bool,
    same_worker_tx: Sender<Uuid>,
    worker_dir: &str,
    keep_job_dir: bool,
    base_internal_url: &str,
    stop_early_override: Option<bool>,
) -> error::Result<()> {
    tracing::debug!("UPDATE FLOW STATUS: {flow:?} {success} {result:?} {w_id}");

    let mut tx = db.begin().await?;

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

    let module_index = usize::try_from(old_status.step).ok();

    let module_status = module_index
        .and_then(|i| old_status.modules.get(i))
        .unwrap_or(&old_status.failure_module.module_status);

    tracing::debug!("UPDATE FLOW STATUS 2: {module_index:#?} {module_status:#?} {old_status:#?} ");

    let skip_loop_failures = if matches!(
        module_status,
        FlowStatusModule::InProgress { iterator: Some(_), .. }
    ) {
        compute_skip_loop_failures(flow, old_status.step, &mut tx)
            .await?
            .unwrap_or(false)
    } else {
        false
    };

    let is_failure_step = old_status.step >= old_status.modules.len() as i32;

    let (mut stop_early, skip_if_stop_early) = if let Some(se) = stop_early_override {
        (true, se)
    } else if is_failure_step {
        (false, false)
    } else {
        let r = sqlx::query!(
            "
            SELECT raw_flow->'modules'->$1::int->'stop_after_if'->>'expr' as stop_early_expr,
            (raw_flow->'modules'->$1::int->'stop_after_if'->>'skip_if_stopped')::bool as skip_if_stopped,
            args 
            FROM queue
             WHERE id = $2
            ",
            old_status.step,
            flow
        )
        .fetch_one(&mut tx)
        .await
        .map_err(|e| Error::InternalErr(format!("retrieval of stop_early_expr from state: {e}")))?;

        let stop_early = success
            && if let Some(expr) = r.stop_early_expr.clone() {
                compute_bool_from_expr(expr, &r.args, result.clone(), base_internal_url, None, None)
                    .await?
            } else {
                false
            };
        (stop_early, r.skip_if_stopped.unwrap_or(false))
    };

    let skip_branch_failure = match module_status {
        FlowStatusModule::InProgress {
            branchall: Some(BranchAllStatus { branch, .. }), ..
        } => compute_skip_branchall_failure(flow, old_status.step, *branch, &mut tx)
            .await?
            .unwrap_or(false),
        _ => false,
    };

    let skip_failure = skip_branch_failure || skip_loop_failures;

    let (inc_step_counter, new_status) = match module_status {
        FlowStatusModule::InProgress {
            iterator,
            branchall,
            parallel,
            flow_jobs: Some(jobs),
            ..
        } if *parallel => {
            let (nindex, len) = match (iterator, branchall) {
                (Some(Iterator { itered, .. }), _) => {
                    let nindex = sqlx::query_scalar!(
                "
            UPDATE queue
               SET flow_status = JSONB_SET(flow_status, ARRAY['modules', $1::TEXT, 'iterator', 'index'], ((flow_status->'modules'->$1::int->'iterator'->>'index')::int + 1)::text::jsonb)
             WHERE id = $2
             RETURNING (flow_status->'modules'->$1::int->'iterator'->>'index')::int
            ",
            old_status.step,
            flow
            )
            .fetch_one(&mut tx)
            .await?
            .ok_or_else(|| Error::InternalErr(format!("requiring an index in InProgress")))?;
                    (nindex, itered.len() as i32)
                }
                (_, Some(BranchAllStatus { len, .. })) => {
                    let nindex = sqlx::query_scalar!(
                "
            UPDATE queue
               SET flow_status = JSONB_SET(flow_status, ARRAY['modules', $1::TEXT, 'branchall', 'branch'], ((flow_status->'modules'->$1::int->'branchall'->>'branch')::int + 1)::text::jsonb)
             WHERE id = $2
             RETURNING (flow_status->'modules'->$1::int->'branchall'->>'branch')::int
            ",
            old_status.step,
            flow
            )
            .fetch_one(&mut tx)
            .await?
            .ok_or_else(|| Error::InternalErr(format!("requiring an index in InProgress")))?;
                    (nindex, *len as i32)
                }
                _ => Err(Error::InternalErr(format!(
                    "unexpected status for parallel module"
                )))?,
            };
            if nindex == len {
                let new_status = if skip_loop_failures
                    || sqlx::query_scalar!(
                        "
                      SELECT success
                        FROM completed_job
                       WHERE id = ANY($1)
                        ",
                        jobs.as_slice(),
                    )
                    .fetch_all(&mut tx)
                    .await?
                    .into_iter()
                    .all(|x| x)
                {
                    FlowStatusModule::Success {
                        id: module_status.id(),
                        job: job_id_for_status.clone(),
                        flow_jobs: Some(jobs.clone()),
                        branch_chosen: None,
                        approvers: vec![],
                    }
                } else {
                    FlowStatusModule::Failure {
                        id: module_status.id(),
                        job: job_id_for_status.clone(),
                        flow_jobs: Some(jobs.clone()),
                        branch_chosen: None,
                    }
                };
                (true, Some(new_status))
            } else {
                tx.commit().await?;
                return Ok(());
            }
        }
        FlowStatusModule::InProgress {
            iterator: Some(windmill_common::flow_status::Iterator { index, itered, .. }),
            ..
        } if (*index + 1 < itered.len() && (success || skip_loop_failures)) && !stop_early => {
            (false, None)
        }
        FlowStatusModule::InProgress {
            branchall: Some(BranchAllStatus { branch, len, .. }),
            ..
        } if branch.to_owned() < len - 1 && (success || skip_branch_failure) => (false, None),
        _ => {
            if stop_early
                && matches!(
                    module_status,
                    FlowStatusModule::InProgress { iterator: Some(_), .. }
                )
            {
                // if we're stopping early inside a loop, we just want to break the loop instead
                stop_early = false;
            }
            let (flow_jobs, branch_chosen) = match module_status {
                FlowStatusModule::InProgress { flow_jobs, branch_chosen, .. } => {
                    (flow_jobs.clone(), branch_chosen.clone())
                }
                _ => (None, None),
            };
            if success || (flow_jobs.is_some() && (skip_loop_failures || skip_branch_failure)) {
                (
                    true,
                    Some(FlowStatusModule::Success {
                        id: module_status.id(),
                        job: job_id_for_status.clone(),
                        flow_jobs,
                        branch_chosen,
                        approvers: vec![],
                    }),
                )
            } else {
                (
                    false,
                    Some(FlowStatusModule::Failure {
                        id: module_status.id(),
                        job: job_id_for_status.clone(),
                        flow_jobs,
                        branch_chosen,
                    }),
                )
            }
        }
    };

    let step_counter = if inc_step_counter {
        sqlx::query!(
            "
            UPDATE queue
               SET flow_status = JSONB_SET(flow_status, ARRAY['step'], $1)
             WHERE id = $2
            ",
            json!(old_status.step + 1),
            flow
        )
        .execute(&mut tx)
        .await?;
        old_status.step + 1
    } else {
        old_status.step
    };

    /* is_last_step is true when the step_counter (the next step index) is an invalid index */
    let is_last_step = usize::try_from(step_counter)
        .map(|i| !(..old_status.modules.len()).contains(&i))
        .unwrap_or(true);

    if let Some(new_status) = new_status.as_ref() {
        if is_failure_step {
            let parent_module = sqlx::query_scalar!(
                "SELECT flow_status->'failure_module'->>'parent_module' FROM queue WHERE id = $1",
                flow
            )
            .fetch_one(&mut tx)
            .await?;

            sqlx::query!(
                "
            UPDATE queue
               SET flow_status = JSONB_SET(flow_status, ARRAY['failure_module'], $1)
             WHERE id = $2
            ",
                json!(FlowStatusModuleWParent { parent_module, module_status: new_status.clone() }),
                flow
            )
            .execute(&mut tx)
            .await?;
        } else {
            sqlx::query!(
                "
            UPDATE queue
               SET flow_status = JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2)
             WHERE id = $3
            ",
                old_status.step.to_string(),
                json!(new_status),
                flow
            )
            .execute(&mut tx)
            .await?;
        }
    }

    let result = match &new_status {
        Some(FlowStatusModule::Success { flow_jobs: Some(jobs), .. })
        | Some(FlowStatusModule::Failure { flow_jobs: Some(jobs), .. }) => {
            let results = sqlx::query!(
                "
                  SELECT result, id
                    FROM completed_job
                   WHERE id = ANY($1)
                     AND workspace_id = $2
                    ",
                jobs.as_slice(),
                w_id
            )
            .fetch_all(&mut tx)
            .await?
            .into_iter()
            .map(|r| (r.id, r.result))
            .collect::<HashMap<_, _>>();

            let results = jobs
                .iter()
                .map(|j| {
                    results
                        .get(j)
                        .ok_or_else(|| Error::InternalErr(format!("missing job result for {}", j)))
                })
                .collect::<Result<Vec<_>, _>>()?;

            json!(results)
        }
        _ => result,
    };

    if matches!(&new_status, Some(FlowStatusModule::Success { .. })) {
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

    let raw_flow = flow_job.parse_raw_flow();
    let module = raw_flow.as_ref().and_then(|module| {
        module_index.and_then(|i| module.modules.get(i).or(module.failure_module.as_ref()))
    });

    let should_continue_flow = match success {
        _ if stop_early => false,
        _ if flow_job.canceled => false,
        true => !is_last_step,
        false if unrecoverable => false,
        false if skip_failure => !is_last_step,
        false
            if next_retry(
                &module.and_then(|m| m.retry.clone()).unwrap_or_default(),
                &old_status.retry,
            )
            .is_some() =>
        {
            true
        }
        false if has_failure_module(flow, &mut tx).await? && !is_failure_step => true,
        false => false,
    };

    if old_status.step == 0
        && !flow_job.is_flow_step
        && flow_job.schedule_path.is_some()
        && flow_job.script_path.is_some()
    {
        tx = schedule_again_if_scheduled(
            tx,
            db,
            flow_job.schedule_path.as_ref().unwrap(),
            flow_job.script_path.as_ref().unwrap(),
            &w_id,
        )
        .await?;
    }
    tx.commit().await?;

    let done = if !should_continue_flow {
        let logs = if flow_job.canceled {
            "Flow job canceled".to_string()
        } else if stop_early {
            format!("Flow job stopped early because of a stop early predicate returning true")
        } else {
            "Flow job completed".to_string()
        };
        if flow_job.canceled {
            add_completed_job_error(
                db,
                &flow_job,
                logs,
                canceled_job_to_result(&flow_job),
                metrics.clone(),
            )
            .await?;
        } else {
            add_completed_job(
                db,
                &flow_job,
                success,
                stop_early && skip_if_stop_early,
                result.clone(),
                logs,
            )
            .await?;
        }
        true
    } else {
        match handle_flow(
            &flow_job,
            db,
            client,
            result.clone(),
            same_worker_tx.clone(),
            worker_dir,
            base_internal_url,
        )
        .await
        {
            Err(err) => {
                let _ = add_completed_job_error(
                    db,
                    &flow_job,
                    "Unexpected error during flow chaining:\n".to_string(),
                    json!({"message": err.to_string(), "name": "InternalError"}),
                    metrics.clone(),
                )
                .await;
                true
            }
            Ok(_) => false,
        }
    };

    if done {
        if flow_job.same_worker && !keep_job_dir {
            let _ = tokio::fs::remove_dir_all(format!("{worker_dir}/{}", flow_job.id)).await;
        }

        if let Some(parent_job) = flow_job.parent_job {
            return Ok(update_flow_status_after_job_completion(
                db,
                client,
                parent_job,
                &flow,
                w_id,
                success,
                result,
                metrics,
                false,
                same_worker_tx.clone(),
                worker_dir,
                keep_job_dir,
                base_internal_url,
                if stop_early {
                    Some(skip_if_stop_early)
                } else {
                    None
                },
            )
            .await?);
        }
    }

    Ok(())
}

async fn compute_skip_loop_failures<'c>(
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

async fn compute_skip_branchall_failure<'c>(
    flow: Uuid,
    step: i32,
    branch: usize,
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
) -> Result<Option<bool>, Error> {
    sqlx::query_as(
        "
    SELECT (raw_flow->'modules'->$1->'value'->'branches'->$2->>'skip_failure')::bool
      FROM queue
     WHERE id = $3
        ",
    )
    .bind(step)
    .bind(branch as i32)
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

async fn compute_bool_from_expr(
    expr: String,
    flow_args: &Option<serde_json::Value>,
    result: serde_json::Value,
    base_internal_url: &str,
    by_id: Option<IdContext>,
    creds: Option<EvalCreds>,
) -> error::Result<bool> {
    let flow_input = flow_args.clone().unwrap_or_else(|| json!({}));
    match eval_timeout(
        expr,
        [
            ("flow_input".to_string(), flow_input),
            ("result".to_string(), result.clone()),
            ("previous_result".to_string(), result),
        ]
        .into(),
        creds,
        by_id,
        base_internal_url.to_string(),
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

pub async fn update_flow_status_in_progress(
    db: &DB,
    w_id: &str,
    flow: Uuid,
    job_in_progress: Uuid,
) -> error::Result<()> {
    let step = get_step_of_flow_status(db, flow).await?;
    if let Step::Step(step) = step {
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
    } else {
        sqlx::query(&format!(
            "UPDATE queue
                SET flow_status = jsonb_set(jsonb_set(flow_status, '{{failure_module, job}}', $1), '{{failure_module, type}}', $2)
                WHERE id = $3 AND workspace_id = $4",
        ))
        .bind(json!(job_in_progress.to_string()))
        .bind(json!("InProgress"))
        .bind(flow)
        .bind(w_id)
        .execute(db)
        .await?;
    }
    Ok(())
}

pub enum Step {
    Step(i32),
    FailureStep,
}
#[instrument(level = "trace", skip_all)]
pub async fn get_step_of_flow_status(db: &DB, id: Uuid) -> error::Result<Step> {
    let r = sqlx::query!(
        "SELECT (flow_status->'step')::integer as step, jsonb_array_length(flow_status->'modules') as len  FROM queue WHERE id = $1",
        id
    )
    .fetch_one(db)
    .await
    .map_err(|e| Error::InternalErr(format!("fetching step flow status: {e}")))?;
    if r.step < r.len {
        Ok(Step::Step(r.step.ok_or_else(|| {
            Error::InternalErr("step is null".to_string())
        })?))
    } else {
        Ok(Step::FailureStep)
    }
}

/// resumes should be in order of timestamp ascending, so that more recent are at the end
#[instrument(level = "trace", skip_all)]
async fn transform_input(
    flow_args: &Option<serde_json::Value>,
    last_result: serde_json::Value,
    input_transforms: &HashMap<String, InputTransform>,
    workspace: &str,
    token: &str,
    resumes: &[Value],
    approvers: Vec<String>,
    by_id: &IdContext,
    base_internal_url: &str,
) -> windmill_common::error::Result<Map<String, serde_json::Value>> {
    let mut mapped = serde_json::Map::new();

    for (key, val) in input_transforms.into_iter() {
        if let InputTransform::Static { value } = val {
            mapped.insert(key.to_string(), value.to_owned());
        }
    }

    let lresult = last_result.clone();
    let error = if let Some(error) = lresult.as_object() {
        error.get("error")
    } else {
        None
    };

    for (key, val) in input_transforms.into_iter() {
        match val {
            InputTransform::Static { value: _ } => (),
            InputTransform::Javascript { expr } => {
                let flow_input = flow_args.clone().unwrap_or_else(|| json!({}));
                let previous_result = last_result.clone();
                let mut context = vec![
                    ("params".to_string(), json!(mapped)),
                    ("previous_result".to_string(), previous_result),
                    ("flow_input".to_string(), flow_input),
                    (
                        "resume".to_string(),
                        resumes.last().map(|v| json!(v)).unwrap_or_default(),
                    ),
                    ("resumes".to_string(), resumes.clone().into()),
                    ("approvers".to_string(), json!(approvers.clone())),
                ];

                if error.is_some() {
                    context.push(("error".to_string(), error.unwrap().clone()));
                }

                let v = eval_timeout(
                    expr.to_string(),
                    context,
                    Some(EvalCreds { workspace: workspace.to_string(), token: token.to_string() }),
                    Some(by_id.clone()),
                    base_internal_url.to_string(),
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
    client: &windmill_api_client::Client,
    last_result: serde_json::Value,
    same_worker_tx: Sender<Uuid>,
    worker_dir: &str,
    base_internal_url: &str,
) -> anyhow::Result<()> {
    let value = flow_job
        .raw_flow
        .as_ref()
        .ok_or_else(|| Error::InternalErr(format!("requiring a raw flow value")))?
        .to_owned();
    let flow = serde_json::from_value::<FlowValue>(value)?;

    let status: FlowStatus =
        serde_json::from_value::<FlowStatus>(flow_job.flow_status.clone().unwrap_or_default())
            .with_context(|| format!("parse flow status {}", flow_job.id))?;

    tracing::debug!("handle_flow: {:#?}", flow_job.flow_status);
    push_next_flow_job(
        flow_job,
        status,
        flow,
        db,
        client,
        last_result,
        same_worker_tx,
        base_internal_url,
        worker_dir,
    )
    .await?;
    Ok(())
}

#[async_recursion]
// #[instrument(level = "trace", skip_all)]
async fn push_next_flow_job(
    flow_job: &QueuedJob,
    mut status: FlowStatus,
    flow: FlowValue,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &windmill_api_client::Client,
    mut last_result: serde_json::Value,
    same_worker_tx: Sender<Uuid>,
    base_internal_url: &str,
    worker_dir: &str,
) -> error::Result<()> {
    let mut i = usize::try_from(status.step)
        .with_context(|| format!("invalid module index {}", status.step))?;

    let mut status_module: FlowStatusModule = status
        .modules
        .get(i)
        .cloned()
        .unwrap_or_else(|| status.failure_module.module_status.clone());

    // if this is an empty module of if the module has aleady been completed, successfully, update the parent flow
    if flow.modules.is_empty() || matches!(status_module, FlowStatusModule::Success { .. }) {
        return update_flow_status_after_job_completion(
            db,
            client,
            flow_job.id,
            &Uuid::nil(),
            flow_job.workspace_id.as_str(),
            true,
            if flow.modules.is_empty() {
                json!({})
            } else {
                json!([])
            },
            None,
            true,
            same_worker_tx,
            worker_dir,
            false,
            base_internal_url,
            None,
        )
        .await;
    }

    let mut module: &FlowModule = flow
        .modules
        .get(i)
        .or_else(|| flow.failure_module.as_ref())
        .with_context(|| format!("no module at index {}", status.step))?;

    let current_id = &module.id;
    let previous_id = if i >= 1 {
        flow.modules.get(i - 1).map(|m| m.id.clone()).unwrap()
    } else {
        String::new()
    };

    // calculate sleep if any
    let mut scheduled_for_o = {
        let sleep_input_transform = i
            .checked_sub(1)
            .and_then(|i| flow.modules.get(i))
            .and_then(|m| m.sleep.clone());

        if let Some(it) = sleep_input_transform {
            let json_value = match it {
                InputTransform::Static { value } => value,
                InputTransform::Javascript { expr } => {
                    let flow_input = flow_job.args.clone().unwrap_or_else(|| json!({}));

                    eval_timeout(
                        expr.to_string(),
                        [
                            ("result".to_string(), last_result.clone()),
                            ("flow_input".to_string(), flow_input),
                        ]
                        .into(),
                        None,
                        None,
                        "".to_string(),
                    )
                    .await
                    .map_err(|e| {
                        Error::ExecutionErr(format!(
                            "Error during isolated evaluation of expression `{expr}`:\n{e}"
                        ))
                    })?
                }
            };
            match json_value {
                serde_json::Value::Number(n) => {
                    n.as_u64().map(|x| from_now(Duration::from_secs(x)))
                }
                _ => Err(Error::ExecutionErr(format!(
                    "Expected a number value, found: {json_value}"
                )))?,
            }
        } else {
            None
        }
    };

    let mut resume_messages: Vec<Value> = vec![];
    let mut approvers: Vec<String> = vec![];

    /* (suspend / resume), when starting a module, if previous module has a
     * non-zero `suspend` value, collect `resume_job`s for the previous module job.
     *
     * If there aren't enough, try again later. */
    if matches!(
        &status_module,
        FlowStatusModule::WaitingForPriorSteps { .. } | FlowStatusModule::WaitingForEvents { .. }
    ) {
        if let Some((suspend, last)) = needs_resume(&flow, &status) {
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
                "SELECT value, approver, resume_id FROM resume_job WHERE job = $1 ORDER BY created_at ASC",
                last
            )
            .fetch_all(&mut tx)
            .await?;

            resume_messages.extend(resumes.iter().map(|r| r.value.clone()));
            approvers.extend(resumes.iter().map(|r| {
                r.approver
                    .as_deref()
                    .unwrap_or_else(|| "anonymous")
                    .to_string()
            }));

            let required_events = suspend.required_events.unwrap() as u16;
            if resume_messages.len() >= required_events as usize {
                sqlx::query(
                    "
                    UPDATE queue
                       SET flow_status = 
                            JSONB_SET(flow_status, ARRAY['modules', $1::TEXT, 'approvers'], $2)
                       WHERE id = $3
                      ",
                )
                .bind(status.step - 1)
                .bind(json!(resumes
                    .into_iter()
                    .map(|r| Approval {
                        resume_id: r.resume_id as u16,
                        approver: r.approver.unwrap_or_else(|| "unknown".to_string())
                    })
                    .collect::<Vec<_>>()))
                .bind(flow_job.id)
                .execute(&mut tx)
                .await?;

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
            } else if matches!(
                &status_module,
                FlowStatusModule::WaitingForPriorSteps { .. }
            ) {
                sqlx::query(
                    "
                    UPDATE queue
                       SET flow_status = JSONB_SET(flow_status, ARRAY['modules', flow_status->>'step'::text], $1)
                         , suspend = $2
                         , suspend_until = now() + $3
                     WHERE id = $4
                    ",
                )
                .bind(json!(FlowStatusModule::WaitingForEvents { id: status_module.id(), count: required_events, job: last }))
                .bind((required_events - resume_messages.len() as u16) as i32)
                .bind(Duration::from_secs(suspend.timeout.map(|t| t.into()).unwrap_or_else(|| 30 * 60)))
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
                let logs = "Timed out waiting to be resumed".to_string();
                let result = json!({ "error": logs });
                let _uuid =
                    add_completed_job(db, &flow_job, success, skipped, result, logs).await?;

                return Ok(());
            }
        }
    }

    match &status_module {
        FlowStatusModule::Failure { job, .. } => {
            let retry = &module.retry.clone().unwrap_or_default();
            if let Some((fail_count, retry_in)) = next_retry(retry, &status.retry) {
                tracing::debug!(
                    retry_in_seconds = retry_in.as_secs(),
                    fail_count = fail_count,
                    "retrying"
                );

                scheduled_for_o = Some(from_now(retry_in));
                status.retry.failed_jobs.push(job.clone());
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
                status_module = FlowStatusModule::WaitingForPriorSteps { id: status_module.id() };

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
                status_module = status.failure_module.module_status.clone();

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
                        failed_jobs: vec![],
                    }))
                    .bind(flow_job.id)
                    .execute(db)
                    .await
                    .context("update flow retry")?;
                };
            }

            /* (retry feature) save the previous_result the first time this step is run */
        }
        FlowStatusModule::WaitingForPriorSteps { .. }
            if module
                .retry
                .as_ref()
                .map(|x| x.has_attempts())
                .unwrap_or(false)
                && status.retry.fail_count == 0 =>
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
                failed_jobs: vec![],
            }))
            .bind(flow_job.id)
            .execute(db)
            .await
            .context("update flow retry")?;
        }
        _ => (),
    }

    let mut transform_context: Option<TransformContext> = None;

    let args: windmill_common::error::Result<_> = match &module.value {
        FlowModuleValue::Script { input_transforms, .. }
        | FlowModuleValue::RawScript { input_transforms, .. }
        | FlowModuleValue::Flow { input_transforms, .. } => {
            let ctx = get_transform_context(db, &flow_job, previous_id.clone(), &status).await?;
            transform_context = Some(ctx);
            let (token, by_id) = transform_context.as_ref().unwrap();
            transform_input(
                &flow_job.args,
                last_result.clone(),
                if !input_transforms.is_empty() {
                    input_transforms
                } else {
                    &module.input_transforms
                },
                &flow_job.workspace_id,
                &token,
                resume_messages.as_slice(),
                approvers,
                by_id,
                base_internal_url,
            )
            .await
        }
        FlowModuleValue::Identity => match last_result.clone() {
            Value::Object(m) => Ok(m),
            v @ _ => {
                let mut m = Map::new();
                m.insert("previous_result".to_string(), v);
                Ok(m)
            }
        },
        _ => {
            /* embedded flow input is augmented with embedding flow input */
            if let Some(value) = &flow_job.args {
                Ok(value
                    .as_object()
                    .ok_or_else(|| {
                        Error::BadRequest(format!("Expected an object value, found: {value:?}"))
                    })?
                    .clone())
            } else {
                Ok(Map::new())
            }
        }
    };

    let tx = db.begin().await?;

    let (tx, next_flow_transform) = compute_next_flow_transform(
        flow_job,
        &flow,
        transform_context,
        db,
        tx,
        &module,
        &status,
        &status_module,
        last_result.clone(),
        base_internal_url,
        previous_id,
    )
    .await?;
    tx.commit().await?;

    let (job_payloads, next_status) = match next_flow_transform {
        NextFlowTransform::Continue(job_payload, next_state) => (job_payload, next_state),
        NextFlowTransform::EmptyInnerFlows => {
            sqlx::query(
                        r#"
                                UPDATE queue
                                    SET flow_status = JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2)
                                    WHERE id = $3
                                "#,
                    )
                    .bind(status.step)
                    .bind(json!(FlowStatusModule::Success { id: status_module.id(), job: Uuid::nil(), flow_jobs: None, branch_chosen: None, approvers: vec![] }))
                    .bind(flow_job.id)
                    .execute(db)
                    .await?;
            // flow is reprocessed by the worker in a state where the module has completed succesfully.
            // The next steps are pull -> handle flow -> push next flow job -> update flow status since module status is success
            same_worker_tx
                .send(flow_job.id)
                .await
                .expect("send to same worker");
            return Ok(());
        }
    };

    let continue_on_same_worker =
        flow.same_worker && module.suspend.is_none() && module.sleep.is_none();

    let zipped = {
        let all_args = match &next_status {
            NextStatus::NextLoopIteration(NextIteration { new_args, .. }) => {
                let args = args.as_ref().map(|args| {
                    let mut args = args.clone();
                    args.extend(new_args.clone());
                    args
                });
                DynIter::new(iter::once(args))
            }

            NextStatus::AllFlowJobs {
                branchall: Some(BranchAllStatus { len, .. }),
                iterator: None,
                ..
            } => DynIter::new((0..*len).map(|_| args.as_ref().map(|args| args.clone()))),
            NextStatus::AllFlowJobs {
                branchall: None,
                iterator: Some(Iterator { itered, .. }),
                ..
            } => DynIter::new(itered.into_iter().enumerate().map(|(i, v)| {
                args.as_ref().map(|args| {
                    let mut new_args = args.clone();
                    new_args.insert("iter".to_string(), json!({ "index": i, "value": v }));
                    new_args
                })
            })),

            _ => DynIter::new(iter::once(args.as_ref().map(|m| m.clone()))),
        };

        job_payloads.into_iter().zip(all_args).collect::<Vec<_>>()
    };

    /* Finally, push the job into the queue */
    let mut tx = db.begin().await?;
    let mut uuids = vec![];

    for (payload, args) in zipped {
        let (ok, err) = match args {
            Ok(v) => (Some(v), None),
            Err(e) => (None, Some(e)),
        };
        let (uuid, inner_tx) = push(
            tx,
            &flow_job.workspace_id,
            payload,
            ok.unwrap_or_else(|| Map::new()),
            &flow_job.created_by,
            &flow_job.email,
            flow_job.permissioned_as.to_owned(),
            scheduled_for_o,
            flow_job.schedule_path.clone(),
            Some(flow_job.id),
            true,
            continue_on_same_worker,
            err,
            flow_job.visible_to_owner,
        )
        .await?;
        tx = inner_tx;
        uuids.push(uuid);
    }

    let is_one_uuid = uuids.len() == 1;

    let one_uuid = if is_one_uuid {
        Ok(uuids[0].clone())
    } else {
        Err(Error::BadRequest("Expected only one uuid".to_string()))
    };
    let first_uuid = uuids[0];
    let new_status = match next_status {
        NextStatus::NextLoopIteration(NextIteration { index, itered, mut flow_jobs, .. }) => {
            let uuid = one_uuid?;

            flow_jobs.push(uuid);

            FlowStatusModule::InProgress {
                job: uuid,
                iterator: Some(windmill_common::flow_status::Iterator { index, itered }),
                flow_jobs: Some(flow_jobs),
                branch_chosen: None,
                branchall: None,
                id: status_module.id(),
                parallel: false,
            }
        }
        NextStatus::AllFlowJobs { iterator, branchall } => FlowStatusModule::InProgress {
            job: flow_job.id,
            iterator,
            flow_jobs: Some(uuids),
            branch_chosen: None,
            branchall,
            id: status_module.id(),
            parallel: true,
        },
        NextStatus::NextBranchStep(NextBranch { mut flow_jobs, status, .. }) => {
            let uuid = one_uuid?;
            flow_jobs.push(uuid);

            FlowStatusModule::InProgress {
                job: uuid,
                iterator: None,
                flow_jobs: Some(flow_jobs),
                branch_chosen: None,
                branchall: Some(status),
                id: status_module.id(),
                parallel: false,
            }
        }

        NextStatus::BranchChosen(branch) => FlowStatusModule::InProgress {
            job: one_uuid?,
            iterator: None,
            flow_jobs: None,
            branch_chosen: Some(branch),
            branchall: None,
            id: status_module.id(),
            parallel: false,
        },
        NextStatus::NextStep => {
            FlowStatusModule::WaitingForExecutor { id: status_module.id(), job: one_uuid? }
        }
    };

    tracing::debug!("STATUS STEP: {:?} {i} {:#?}", status.step, new_status);

    if i >= flow.modules.len() {
        sqlx::query!(
            "
                UPDATE queue
                   SET flow_status = JSONB_SET(
                                     JSONB_SET(flow_status, ARRAY['failure_module'], $1),
                                                            ARRAY['step'], $2)
                 WHERE id = $3
                  ",
            json!(FlowStatusModuleWParent {
                parent_module: Some(current_id.clone()),
                module_status: new_status.clone()
            }),
            json!(i),
            flow_job.id
        )
        .execute(&mut tx)
        .await?;
    } else {
        sqlx::query!(
            "
                UPDATE queue
                   SET flow_status = JSONB_SET(
                                     JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2),
                                                            ARRAY['step'], $3)
                 WHERE id = $4
                  ",
            i as i32,
            json!(new_status),
            json!(i),
            flow_job.id
        )
        .execute(&mut tx)
        .await?;
    };

    tx.commit().await?;

    if continue_on_same_worker {
        if !is_one_uuid {
            return Err(Error::BadRequest(
                "Cannot continue on same worker with multiple jobs, parallel cannot be used in conjunction with same_worker".to_string(),
            ));
        }
        same_worker_tx.send(first_uuid).await.map_err(to_anyhow)?;
    }
    return Ok(());
}

// async fn jump_to_next_step(
//     status_step: i32,
//     i: usize,
//     job_id: &Uuid,
//     flow: FlowValue,
//     db: &DB,
//     client: &windmill_api_client::Client,
//     status_module: FlowStatusModule,
//     last_result: serde_json::Value,
//     same_worker_tx: Sender<Uuid>,
//     base_internal_url: &str,
// ) -> error::Result<()> {
//     let mut tx = db.begin().await?;

//     let next_step = i
//         .checked_add(1)
//         .filter(|i| (..flow.modules.len()).contains(i));

//     let new_job = sqlx::query_as::<_, QueuedJob>(
//         r#"
//                 UPDATE queue
//                     SET flow_status = JSONB_SET(
//                                       JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2),
//                                                                ARRAY['step'], $3)
//                     WHERE id = $4
//                 RETURNING *
//                 "#,
//     )
//     .bind(status_step)
//     .bind(json!(status_module))
//     .bind(json!(next_step.unwrap_or(i)))
//     .bind(job_id)
//     .fetch_one(&mut tx)
//     .await?;

//     tx.commit().await?;

//     let new_status = new_job.parse_flow_status().ok_or_else(|| {
//         Error::ExecutionErr("Impossible to parse new status after jump".to_string())
//     })?;

//     if next_step.is_some() {
//         tracing::debug!("Jumping to next step with flow {flow:#?}");
//         return push_next_flow_job(
//             &new_job,
//             new_status,
//             flow,
//             db,
//             client,
//             last_result,
//             same_worker_tx,
//             base_internal_url,
//         )
//         .await;
//     } else {
//         let success = true;
//         let skipped = false;
//         let logs = "Forloop completed without iteration".to_string();
//         let _uuid =
//             add_completed_job(db, client, &new_job, success, skipped, json!([]), logs).await?;
//         return Ok(());
//     }
// }

/// Some state about the current/last forloop FlowStatusModule used to initialized the next
/// iteration's FlowStatusModule after pushing a job
#[derive(Debug)]
struct NextIteration {
    index: usize,
    itered: Vec<Value>,
    flow_jobs: Vec<Uuid>,
    new_args: Map<String, serde_json::Value>,
}

enum LoopStatus {
    ParallelIteration { itered: Vec<Value> },
    NextIteration(NextIteration),
    EmptyIterator,
}

#[derive(Debug)]
struct NextBranch {
    status: BranchAllStatus,
    flow_jobs: Vec<Uuid>,
}

#[derive(Debug)]
enum NextStatus {
    NextStep,
    BranchChosen(BranchChosen),
    NextBranchStep(NextBranch),
    NextLoopIteration(NextIteration),
    AllFlowJobs {
        branchall: Option<BranchAllStatus>,
        iterator: Option<windmill_common::flow_status::Iterator>,
    },
}

enum NextFlowTransform {
    EmptyInnerFlows,
    Continue(Vec<JobPayload>, NextStatus),
}

// a similar function exists on the backend
// TODO: rewrite this to use an endpoint in the backend directly, instead of checking for hub itself, and then using the API
async fn script_path_to_payload<'c>(
    script_path: &str,
    db: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    w_id: &String,
) -> Result<JobPayload, Error> {
    let job_payload = if script_path.starts_with("hub/") {
        JobPayload::ScriptHub { path: script_path.to_owned() }
    } else {
        let script_hash = windmill_common::get_latest_hash_for_path(db, w_id, script_path).await?;
        JobPayload::ScriptHash { hash: script_hash, path: script_path.to_owned() }
    };
    Ok(job_payload)
}

type TransformContext = (String, IdContext);

async fn compute_next_flow_transform<'c>(
    flow_job: &QueuedJob,
    flow: &FlowValue,
    transform_context: Option<TransformContext>,
    db: &DB,
    mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
    module: &FlowModule,
    status: &FlowStatus,
    status_module: &FlowStatusModule,
    last_result: serde_json::Value,
    base_internal_url: &str,
    previous_id: String,
) -> error::Result<(sqlx::Transaction<'c, sqlx::Postgres>, NextFlowTransform)> {
    match &module.value {
        FlowModuleValue::Identity => Ok((
            tx,
            NextFlowTransform::Continue(vec![JobPayload::Identity], NextStatus::NextStep),
        )),
        FlowModuleValue::Flow { path, .. } => {
            let payload = JobPayload::Flow(path.to_string());
            Ok((
                tx,
                NextFlowTransform::Continue(vec![payload], NextStatus::NextStep),
            ))
        }
        FlowModuleValue::Script { path: script_path, hash: script_hash, .. } => {
            let payload = if script_hash.is_none() {
                script_path_to_payload(script_path, &mut tx, &flow_job.workspace_id).await?
            } else {
                JobPayload::ScriptHash {
                    hash: script_hash.clone().unwrap(),
                    path: script_path.to_owned(),
                }
            };
            Ok((
                tx,
                NextFlowTransform::Continue(vec![payload], NextStatus::NextStep),
            ))
        }
        FlowModuleValue::RawScript { path, content, language, lock, .. } => {
            let path = path
                .clone()
                .or_else(|| Some(format!("{}/{}", flow_job.script_path(), status.step)));
            Ok((
                tx,
                NextFlowTransform::Continue(
                    vec![JobPayload::Code(RawCode {
                        path,
                        content: content.clone(),
                        language: language.clone(),
                        lock: lock.clone(),
                    })],
                    NextStatus::NextStep,
                ),
            ))
        }
        /* forloop modules are expected set `iter: { value: Value, index: usize }` as job arguments */
        FlowModuleValue::ForloopFlow { modules, iterator, parallel, .. } => {
            let new_args: &mut Map<String, serde_json::Value> = &mut Map::new();

            let next_loop_status = match status_module {
                FlowStatusModule::WaitingForPriorSteps { .. }
                | FlowStatusModule::WaitingForEvents { .. }
                | FlowStatusModule::WaitingForExecutor { .. } => {
                    let (token, by_id) = if let Some(x) = transform_context {
                        x
                    } else {
                        get_transform_context(db, &flow_job, previous_id, &status).await?
                    };
                    let flow_input = flow_job.args.clone().unwrap_or_else(|| json!({}));
                    /* Iterator is an InputTransform, evaluate it into an array. */
                    let itered = evaluate_with(
                        iterator.clone(),
                        || {
                            vec![
                                ("flow_input".to_string(), flow_input),
                                ("result".to_string(), last_result.clone()),
                                ("previous_result".to_string(), last_result.clone()),
                            ]
                        },
                        token,
                        flow_job.workspace_id.clone(),
                        Some(by_id),
                        base_internal_url,
                    )
                    .await?
                    .into_array()
                    .map_err(|not_array| {
                        Error::ExecutionErr(format!("Expected an array value, found: {not_array}"))
                    })?;

                    if itered.is_empty() {
                        LoopStatus::EmptyIterator
                    } else if *parallel {
                        LoopStatus::ParallelIteration { itered }
                    } else if let Some(first) = itered.first() {
                        new_args.insert("iter".to_string(), json!({ "index": 0, "value": first }));

                        LoopStatus::NextIteration(NextIteration {
                            index: 0,
                            itered,
                            flow_jobs: vec![],
                            new_args: new_args.clone(),
                        })
                    } else {
                        panic!("itered cannot be empty")
                    }
                }

                FlowStatusModule::InProgress {
                    iterator: Some(windmill_common::flow_status::Iterator { itered, index }),
                    flow_jobs: Some(flow_jobs),
                    ..
                } if !*parallel => {
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

                    new_args.insert("iter".to_string(), json!({ "index": index, "value": next }));

                    LoopStatus::NextIteration(NextIteration {
                        index,
                        itered: itered.clone(),
                        flow_jobs: flow_jobs.clone(),
                        new_args: new_args.clone(),
                    })
                }

                _ => Err(Error::BadRequest(format!(
                    "Unrecognized module status for ForloopFlow {status_module:?}"
                )))?,
            };

            match next_loop_status {
                LoopStatus::EmptyIterator => Ok((tx, NextFlowTransform::EmptyInnerFlows)),
                LoopStatus::NextIteration(ns) => {
                    let mut fm = flow.failure_module.clone();
                    if let Some(mut failure_module) = flow.failure_module.clone() {
                        failure_module.id_append(&format!("{}/{}", status.step, ns.index));
                        fm = Some(failure_module);
                    }
                    Ok((
                        tx,
                        NextFlowTransform::Continue(
                            vec![JobPayload::RawFlow {
                                value: FlowValue {
                                    modules: (*modules).clone(),
                                    failure_module: fm,
                                    same_worker: flow.same_worker,
                                },
                                path: Some(format!("{}/loop-{}", flow_job.script_path(), ns.index)),
                            }],
                            NextStatus::NextLoopIteration(ns),
                        ),
                    ))
                }
                LoopStatus::ParallelIteration { itered } => Ok((
                    tx,
                    NextFlowTransform::Continue(
                        (0..itered.len())
                            .map(|i| {
                                let mut fm = flow.failure_module.clone();
                                if let Some(mut failure_module) = flow.failure_module.clone() {
                                    failure_module.id_append(&format!("{}/{}", status.step, i));
                                    fm = Some(failure_module);
                                }
                                JobPayload::RawFlow {
                                    value: FlowValue {
                                        modules: (*modules).clone(),
                                        failure_module: fm.clone(),
                                        same_worker: flow.same_worker,
                                    },
                                    path: Some(format!("{}/loop-{}", flow_job.script_path(), i)),
                                }
                            })
                            .collect(),
                        NextStatus::AllFlowJobs {
                            branchall: None,
                            iterator: Some(windmill_common::flow_status::Iterator {
                                index: 0,
                                itered,
                            }),
                        },
                    ),
                )),
            }
        }
        FlowModuleValue::BranchOne { branches, default, .. } => {
            let branch = match status_module {
                FlowStatusModule::WaitingForPriorSteps { .. }
                | FlowStatusModule::WaitingForEvents { .. }
                | FlowStatusModule::WaitingForExecutor { .. } => {
                    let mut branch_chosen = BranchChosen::Default;
                    let (token, idcontext) =
                        get_transform_context(db, &flow_job, previous_id, &status).await?;
                    for (i, b) in branches.iter().enumerate() {
                        let pred = compute_bool_from_expr(
                            b.expr.to_string(),
                            &flow_job.args,
                            last_result.clone(),
                            base_internal_url,
                            Some(idcontext.clone()),
                            Some(EvalCreds {
                                workspace: flow_job.workspace_id.clone(),
                                token: token.to_string(),
                            }),
                        )
                        .await?;

                        if pred {
                            branch_chosen = BranchChosen::Branch { branch: i };
                            break;
                        }
                    }
                    branch_chosen
                }
                _ => Err(Error::BadRequest(format!(
                    "Unrecognized module status for BranchOne {status_module:?}"
                )))?,
            };

            let modules = if let BranchChosen::Branch { branch } = branch {
                branches
                    .get(branch)
                    .map(|b| b.modules.clone())
                    .ok_or_else(|| {
                        Error::BadRequest(format!(
                            "Unrecognized branch for BranchAll {status_module:?}"
                        ))
                    })?
            } else {
                default.clone()
            };
            let mut fm = flow.failure_module.clone();
            if let Some(mut failure_module) = flow.failure_module.clone() {
                failure_module.id_append(&status.step.to_string());
                fm = Some(failure_module);
            }
            Ok((
                tx,
                NextFlowTransform::Continue(
                    vec![JobPayload::RawFlow {
                        value: FlowValue {
                            modules,
                            failure_module: fm,
                            same_worker: flow.same_worker,
                        },
                        path: Some(format!(
                            "{}/branchone-{}",
                            flow_job.script_path(),
                            status.step
                        )),
                    }],
                    NextStatus::BranchChosen(branch),
                ),
            ))
        }
        FlowModuleValue::BranchAll { branches, parallel, .. } => {
            let (branch_status, flow_jobs) = match status_module {
                FlowStatusModule::WaitingForPriorSteps { .. }
                | FlowStatusModule::WaitingForEvents { .. }
                | FlowStatusModule::WaitingForExecutor { .. } => {
                    if branches.is_empty() {
                        return Ok((tx, NextFlowTransform::EmptyInnerFlows));
                    } else if *parallel {
                        return Ok((
                            tx,
                            NextFlowTransform::Continue(
                                branches
                                    .iter()
                                    .enumerate()
                                    .map(|(i, b)| {
                                        let mut fm = flow.failure_module.clone();
                                        if let Some(mut failure_module) =
                                            flow.failure_module.clone()
                                        {
                                            failure_module
                                                .id_append(&format!("{}/{i}", status.step));
                                            fm = Some(failure_module);
                                        }
                                        JobPayload::RawFlow {
                                            value: FlowValue {
                                                modules: b.modules.clone(),
                                                failure_module: fm.clone(),
                                                same_worker: flow.same_worker,
                                            },
                                            path: Some(format!(
                                                "{}/branchall-{}",
                                                flow_job.script_path(),
                                                i
                                            )),
                                        }
                                    })
                                    .collect(),
                                NextStatus::AllFlowJobs {
                                    branchall: Some(BranchAllStatus {
                                        branch: 0,
                                        previous_result: last_result,
                                        len: branches.len(),
                                    }),
                                    iterator: None,
                                },
                            ),
                        ));
                    } else {
                        (
                            BranchAllStatus {
                                branch: 0,
                                previous_result: last_result,
                                len: branches.len(),
                            },
                            vec![],
                        )
                    }
                }
                FlowStatusModule::InProgress {
                    branchall: Some(BranchAllStatus { branch, previous_result, len }),
                    flow_jobs: Some(flow_jobs),
                    ..
                } if !*parallel => (
                    BranchAllStatus {
                        branch: branch + 1,
                        previous_result: previous_result.clone(),
                        len: len.clone(),
                    },
                    flow_jobs.clone(),
                ),

                _ => Err(Error::BadRequest(format!(
                    "Unrecognized module status for BranchAll {status_module:?}"
                )))?,
            };

            let modules = branches
                .get(branch_status.branch)
                .map(|b| b.modules.clone())
                .ok_or_else(|| {
                    Error::BadRequest(format!(
                        "Unrecognized branch for BranchAll {status_module:?}"
                    ))
                })?;

            let mut fm = flow.failure_module.clone();
            if let Some(mut failure_module) = flow.failure_module.clone() {
                failure_module.id_append(&format!("{}/{}", status.step, branch_status.branch));
                fm = Some(failure_module);
            }
            Ok((
                tx,
                NextFlowTransform::Continue(
                    vec![JobPayload::RawFlow {
                        value: FlowValue {
                            modules,
                            failure_module: fm.clone(),
                            same_worker: flow.same_worker,
                        },
                        path: Some(format!(
                            "{}/branchall-{}",
                            flow_job.script_path(),
                            branch_status.branch
                        )),
                    }],
                    NextStatus::NextBranchStep(NextBranch { status: branch_status, flow_jobs }),
                ),
            ))
        }
    }
}

async fn get_transform_context(
    db: &DB,
    flow_job: &QueuedJob,
    previous_id: String,
    status: &FlowStatus,
) -> error::Result<TransformContext> {
    let tx = db.begin().await?;
    let (tx, new_token) = crate::create_token_for_owner(
        tx,
        &flow_job.workspace_id,
        &flow_job.permissioned_as,
        "transform-input",
        10,
    )
    .await?;
    //we need to commit asap otherwise the token won't be valid for auth to check outside of this transaction
    //which will happen with client http calls
    tx.commit().await?;
    let steps_results: HashMap<String, JobResult> = status
        .modules
        .iter()
        .filter_map(|x| x.job_result().map(|y| (x.id(), y)))
        .collect();

    Ok((
        new_token,
        IdContext { flow_job: flow_job.id, steps_results, previous_id },
    ))
}

async fn evaluate_with<F>(
    transform: InputTransform,
    vars: F,
    token: String,
    workspace: String,
    by_id: Option<IdContext>,
    base_internal_url: &str,
) -> anyhow::Result<serde_json::Value>
where
    F: FnOnce() -> Vec<(String, serde_json::Value)>,
{
    match transform {
        InputTransform::Static { value } => Ok(value),
        InputTransform::Javascript { expr } => {
            eval_timeout(
                expr,
                vars(),
                Some(EvalCreds { workspace, token }),
                by_id,
                base_internal_url.to_string(),
            )
            .await
        }
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
fn needs_resume(flow: &FlowValue, status: &FlowStatus) -> Option<(Suspend, Uuid)> {
    let prev = usize::try_from(status.step)
        .ok()
        .and_then(|s| s.checked_sub(1))?;

    let suspend = flow.modules.get(prev)?.suspend.clone();
    if suspend
        .as_ref()
        .and_then(|s| s.required_events)
        .unwrap_or(0)
        == 0
    {
        return None;
    }

    if let &FlowStatusModule::Success { job, .. } = status.modules.get(prev)? {
        Some((suspend.unwrap(), job))
    } else {
        None
    }
}
