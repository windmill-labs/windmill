/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;
use std::time::Duration;

use crate::common::{hash_args, save_in_cache};
use crate::js_eval::{eval_timeout, IdContext};
use crate::{AuthedClient, KEEP_JOB_DIR};
use anyhow::Context;
use async_recursion::async_recursion;
use serde_json::{json, Map, Value};
use tokio::sync::mpsc::Sender;
use tracing::instrument;
use uuid::Uuid;
use windmill_common::flow_status::{FlowStatusModuleWParent, Iterator, JobResult};
use windmill_common::jobs::{
    script_hash_to_tag_and_limits, script_path_to_payload, JobPayload, Metrics, QueuedJob, RawCode,
};
use windmill_common::{
    error::{self, to_anyhow, Error},
    flow_status::{
        Approval, BranchAllStatus, BranchChosen, FlowStatus, FlowStatusModule, RetryStatus,
        MAX_RETRY_ATTEMPTS, MAX_RETRY_INTERVAL,
    },
    flows::{FlowModule, FlowModuleValue, FlowValue, InputTransform, Retry, Suspend},
};
use windmill_queue::{
    add_completed_job, add_completed_job_error, handle_maybe_scheduled_job, PushIsolationLevel,
};

type DB = sqlx::Pool<sqlx::Postgres>;

use windmill_queue::{canceled_job_to_result, get_queued_job, push, QueueTransaction};

// #[instrument(level = "trace", skip_all)]
pub async fn update_flow_status_after_job_completion<
    R: rsmq_async::RsmqConnection + Send + Sync + Clone,
>(
    db: &DB,
    client: &AuthedClient,
    flow: uuid::Uuid,
    job_id_for_status: &Uuid,
    w_id: &str,
    success: bool,
    result: serde_json::Value,
    metrics: Option<Metrics>,
    unrecoverable: bool,
    same_worker_tx: Sender<Uuid>,
    worker_dir: &str,
    stop_early_override: Option<bool>,
    rsmq: Option<R>,
) -> error::Result<()> {
    // this is manual tailrecursion because async_recursion blows up the stack
    let mut rec = update_flow_status_after_job_completion_internal(
        db,
        client,
        flow,
        job_id_for_status,
        w_id,
        success,
        result,
        metrics.clone(),
        unrecoverable,
        same_worker_tx.clone(),
        worker_dir,
        stop_early_override,
        false,
        rsmq.clone(),
    )
    .await?;
    while let Some(nrec) = rec {
        rec = update_flow_status_after_job_completion_internal(
            db,
            client,
            nrec.flow,
            &nrec.job_id_for_status,
            w_id,
            nrec.success,
            nrec.result,
            metrics.clone(),
            false,
            same_worker_tx.clone(),
            worker_dir,
            nrec.stop_early_override,
            nrec.skip_error_handler,
            rsmq.clone(),
        )
        .await?;
    }
    Ok(())
}
pub struct RecUpdateFlowStatusAfterJobCompletion {
    flow: uuid::Uuid,
    job_id_for_status: Uuid,
    success: bool,
    result: serde_json::Value,
    stop_early_override: Option<bool>,
    skip_error_handler: bool,
}
// #[instrument(level = "trace", skip_all)]
pub async fn update_flow_status_after_job_completion_internal<
    R: rsmq_async::RsmqConnection + Send + Sync + Clone,
>(
    db: &DB,
    client: &AuthedClient,
    flow: uuid::Uuid,
    job_id_for_status: &Uuid,
    w_id: &str,
    mut success: bool,
    result: serde_json::Value,
    metrics: Option<Metrics>,
    unrecoverable: bool,
    same_worker_tx: Sender<Uuid>,
    worker_dir: &str,
    stop_early_override: Option<bool>,
    skip_error_handler: bool,
    rsmq: Option<R>,
) -> error::Result<Option<RecUpdateFlowStatusAfterJobCompletion>> {
    let (should_continue_flow, flow_job, stop_early, skip_if_stop_early, nresult, is_failure_step) = {
        // tracing::debug!("UPDATE FLOW STATUS: {flow:?} {success} {result:?} {w_id} {depth}");

        let old_status_json = sqlx::query_scalar!(
            "SELECT flow_status FROM queue WHERE id = $1 AND workspace_id = $2",
            flow,
            w_id
        )
        .fetch_one(db)
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

        // tracing::debug!(
        //     "UPDATE FLOW STATUS 2: {module_index:#?} {module_status:#?} {old_status:#?} "
        // );

        let (skip_loop_failures, parallelism) = if matches!(
            module_status,
            FlowStatusModule::InProgress { iterator: Some(_), .. }
        ) {
            let (loop_failures, parallelism) =
                compute_skip_loop_failures_and_parallelism(flow, old_status.step, db).await?;
            (loop_failures.unwrap_or(false), parallelism)
        } else {
            (false, None)
        };

        // 0 length flows are not failure steps
        let is_failure_step =
            old_status.step >= old_status.modules.len() as i32 && old_status.modules.len() > 0;

        let (mut stop_early, skip_if_stop_early) = if let Some(se) = stop_early_override {
            //do not stop early if module is a flow step
            let mut tx = db.begin().await?;
            let flow_job = get_queued_job(flow, w_id, &mut tx)
                .await?
                .ok_or_else(|| Error::InternalErr(format!("requiring flow to be in the queue")))?;
            tx.commit().await?;
            let module = get_module(&flow_job, module_index);
            if module.is_some_and(|x| matches!(x.value, FlowModuleValue::Flow { .. })) {
                (false, false)
            } else {
                (true, se)
            }
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
        .fetch_one(db)
        .await
        .map_err(|e| Error::InternalErr(format!("retrieval of stop_early_expr from state: {e}")))?;

            let stop_early = success
                && if let Some(expr) = r.stop_early_expr.clone() {
                    compute_bool_from_expr(expr, &r.args, result.clone(), None, Some(client), None)
                        .await?
                } else {
                    false
                };
            (stop_early, r.skip_if_stopped.unwrap_or(false))
        };

        let skip_branch_failure = match module_status {
            FlowStatusModule::InProgress {
                branchall: Some(BranchAllStatus { branch, .. }),
                ..
            } => compute_skip_branchall_failure(flow, old_status.step, *branch, db)
                .await?
                .unwrap_or(false),
            _ => false,
        };

        let mut tx: QueueTransaction<'_, _> = (rsmq.clone(), db.begin().await?).into();

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
                        success = true;
                        FlowStatusModule::Success {
                            id: module_status.id(),
                            job: job_id_for_status.clone(),
                            flow_jobs: Some(jobs.clone()),
                            branch_chosen: None,
                            approvers: vec![],
                        }
                    } else {
                        success = false;

                        FlowStatusModule::Failure {
                            id: module_status.id(),
                            job: job_id_for_status.clone(),
                            flow_jobs: Some(jobs.clone()),
                            branch_chosen: None,
                        }
                    };
                    (true, Some(new_status))
                } else {
                    if parallelism.is_some() {
                        sqlx::query!(
                            "UPDATE queue SET suspend = suspend - 1 WHERE parent_job = $1",
                            flow
                        )
                        .execute(&mut tx)
                        .await?;
                    }
                    tx.commit().await?;
                    return Ok(None);
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
                    success = true;
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
                    json!(FlowStatusModuleWParent {
                        parent_module,
                        module_status: new_status.clone()
                    }),
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

                if let Some(job_result) = new_status.job_result() {
                    sqlx::query!(
                        "
            UPDATE queue
               SET leaf_jobs = JSONB_SET(coalesce(leaf_jobs, '{}'::jsonb), ARRAY[$1::TEXT], $2)
             WHERE COALESCE((SELECT root_job FROM queue WHERE id = $3), $3) = id
            ",
                        new_status.id(),
                        json!(job_result),
                        flow
                    )
                    .execute(&mut tx)
                    .await?;
                }
            }
        }

        let nresult = match &new_status {
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
                        results.get(j).ok_or_else(|| {
                            Error::InternalErr(format!("missing job result for {}", j))
                        })
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

        let flow_job = get_queued_job(flow, w_id, tx.transaction_mut())
            .await?
            .ok_or_else(|| Error::InternalErr(format!("requiring flow to be in the queue")))?;

        let job_root = flow_job
            .root_job
            .map(|x| x.to_string())
            .unwrap_or_else(|| "none".to_string());
        tracing::info!(id = %flow_job.id, root_id = %job_root, "update flow status");

        let module = get_module(&flow_job, module_index);

        // tracing::error!(
        //     "UPDATE FLOW STATUS 3: {module:#?} {skip_failure} {is_last_step} {success}"
        // );
        let should_continue_flow = match success {
            _ if stop_early => false,
            _ if flow_job.canceled => false,
            true => !is_last_step,
            false if unrecoverable => false,
            false if skip_branch_failure || skip_loop_failures => !is_last_step,
            false
                if next_retry(
                    &module.and_then(|m| m.retry.clone()).unwrap_or_default(),
                    &old_status.retry,
                )
                .is_some() =>
            {
                true
            }
            false
                if !is_failure_step
                    && !skip_error_handler
                    && has_failure_module(flow, tx.transaction_mut()).await? =>
            {
                true
            }
            false => false,
        };

        if old_status.step == 0
            && !flow_job.is_flow_step
            && flow_job.schedule_path.is_some()
            && flow_job.script_path.is_some()
        {
            tx = handle_maybe_scheduled_job(
                tx,
                db,
                flow_job.schedule_path.as_ref().unwrap(),
                flow_job.script_path.as_ref().unwrap(),
                &w_id,
            )
            .await?;
        }

        tx.commit().await?;
        (
            should_continue_flow,
            flow_job,
            stop_early,
            skip_if_stop_early,
            nresult,
            is_failure_step,
        )
    };

    let done = if !should_continue_flow {
        let logs = if flow_job.canceled {
            "Flow job canceled".to_string()
        } else if stop_early {
            format!("Flow job stopped early because of a stop early predicate returning true")
        } else if success {
            "Flow job completed with success".to_string()
        } else {
            "Flow job completed with error".to_string()
        };
        if flow_job.canceled {
            add_completed_job_error(
                db,
                &flow_job,
                logs,
                &canceled_job_to_result(&flow_job),
                metrics.clone(),
                rsmq.clone(),
            )
            .await?;
        } else {
            if flow_job.cache_ttl.is_some() {
                let cached_res_path = {
                    let args_hash = hash_args(&flow_job.args.clone().unwrap_or_else(|| json!({})));
                    let flow_path = flow_job.script_path();
                    format!("{flow_path}/flow/cache/{args_hash}")
                };

                save_in_cache(&client, &flow_job, cached_res_path, &nresult).await;
            }
            add_completed_job(
                db,
                &flow_job,
                success && !is_failure_step && !skip_error_handler,
                stop_early && skip_if_stop_early,
                &nresult,
                logs,
                rsmq.clone(),
            )
            .await?;
        }
        true
    } else {
        match handle_flow(
            &flow_job,
            db,
            client,
            nresult.clone(),
            same_worker_tx.clone(),
            worker_dir,
            rsmq.clone(),
        )
        .await
        {
            Err(err) => {
                let e = json!({"message": err.to_string(), "name": "InternalError"});
                let _ = add_completed_job_error(
                    db,
                    &flow_job,
                    "Unexpected error during flow chaining:\n".to_string(),
                    &e,
                    metrics.clone(),
                    rsmq.clone(),
                )
                .await;
                true
            }
            Ok(_) => false,
        }
    };

    if done {
        if flow_job.same_worker && !*KEEP_JOB_DIR {
            let _ = tokio::fs::remove_dir_all(format!("{worker_dir}/{}", flow_job.id)).await;
        }

        if let Some(parent_job) = flow_job.parent_job {
            return Ok(Some(RecUpdateFlowStatusAfterJobCompletion {
                flow: parent_job,
                job_id_for_status: flow,
                success: success && !is_failure_step,
                result: nresult,
                stop_early_override: if stop_early {
                    Some(skip_if_stop_early)
                } else {
                    None
                },
                skip_error_handler: skip_error_handler || is_failure_step,
            }));
        }
        Ok(None)
    } else {
        Ok(None)
    }
}

fn get_module(flow_job: &QueuedJob, module_index: Option<usize>) -> Option<FlowModule> {
    let raw_flow = flow_job.parse_raw_flow();
    if let Some(raw_flow) = raw_flow {
        if let Some(i) = module_index {
            if let Some(module) = raw_flow.modules.get(i) {
                Some(module.clone())
            } else {
                raw_flow.failure_module
            }
        } else {
            None
        }
    } else {
        None
    }
}

async fn compute_skip_loop_failures_and_parallelism(
    flow: Uuid,
    step: i32,
    db: &DB,
) -> Result<(Option<bool>, Option<i32>), Error> {
    sqlx::query_as(
        "
    SELECT (raw_flow->'modules'->$1->'value'->>'skip_failures')::bool, (raw_flow->'modules'->$1->'value'->>'parallelism')::int
      FROM queue
     WHERE id = $2
        ",
    )
    .bind(step)
    .bind(flow)
    .fetch_one(db)
    .await
    .map(|(v, n)| (v,n))
    .map_err(|e| Error::InternalErr(format!("error during retrieval of skip_loop_failures: {e}")))
}

async fn compute_skip_branchall_failure<'c>(
    flow: Uuid,
    step: i32,
    branch: usize,
    db: &DB,
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
    .fetch_one(db)
    .await
    .map(|(v,)| v)
    .map_err(|e| Error::InternalErr(format!("error during retrieval of skip_loop_failures: {e}")))
}

async fn has_failure_module<'c>(
    flow: Uuid,
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
) -> Result<bool, Error> {
    sqlx::query_scalar::<_, Option<bool>>(
        "
    SELECT raw_flow->'failure_module' != 'null'::jsonb
      FROM queue
     WHERE id = $1
        ",
    )
    .bind(flow)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| Error::InternalErr(format!("error during retrieval of has_failure_module: {e}")))
    .map(|v| v.unwrap_or(false))
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
    by_id: Option<IdContext>,
    client: Option<&AuthedClient>,
    resumes: Option<(&[Value], Vec<String>)>,
) -> error::Result<bool> {
    let flow_input = flow_args.clone().unwrap_or_else(|| json!({}));
    let mut env = vec![
        ("flow_input".to_string(), flow_input),
        ("result".to_string(), result.clone()),
        ("previous_result".to_string(), result),
    ];

    if let Some(resumes) = resumes {
        env.push((
            "resume".to_string(),
            resumes.0.last().map(|v| json!(v)).unwrap_or_default(),
        ));
        env.push(("resumes".to_string(), resumes.0.clone().into()));
        env.push(("approvers".to_string(), json!(resumes.1.clone())));
    }

    match eval_timeout(expr, env.into(), client, by_id).await? {
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
) -> error::Result<Option<i32>> {
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
        Ok(Some(step))
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
        Ok(None)
    }
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
    resumes: &[Value],
    approvers: Vec<String>,
    by_id: &IdContext,
    client: &AuthedClient,
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

                let v = eval_timeout(expr.to_string(), context, Some(client), Some(by_id.clone()))
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
pub async fn handle_flow<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    flow_job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClient,
    last_result: serde_json::Value,
    same_worker_tx: Sender<Uuid>,
    worker_dir: &str,
    rsmq: Option<R>,
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
        worker_dir,
        rsmq,
    )
    .await?;
    Ok(())
}

#[async_recursion]
// #[instrument(level = "trace", skip_all)]
async fn push_next_flow_job<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    flow_job: &QueuedJob,
    mut status: FlowStatus,
    flow: FlowValue,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClient,
    mut last_result: serde_json::Value,
    same_worker_tx: Sender<Uuid>,
    worker_dir: &str,
    rsmq: Option<R>,
) -> error::Result<()> {
    let job_root = flow_job
        .root_job
        .map(|x| x.to_string())
        .unwrap_or_else(|| "none".to_string());
    tracing::info!(id = %flow_job.id, root_id = %job_root, "pushing next flow job");

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
                flow_job.args.clone().unwrap_or_default()
            } else {
                json!([])
            },
            None,
            true,
            same_worker_tx,
            worker_dir,
            None,
            rsmq,
        )
        .await;
    }

    if let Some(skip_expr) = &flow.skip_expr {
        let skip = compute_bool_from_expr(
            skip_expr.to_string(),
            &flow_job.args,
            last_result.clone(),
            None,
            Some(client),
            None,
        )
        .await?;
        if skip {
            return update_flow_status_after_job_completion(
                db,
                client,
                flow_job.id,
                &Uuid::nil(),
                flow_job.workspace_id.as_str(),
                true,
                json!([]),
                None,
                true,
                same_worker_tx,
                worker_dir,
                Some(true),
                rsmq,
            )
            .await;
        }
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
            .fetch_one(&mut *tx)
            .await
            .context("lock flow in queue")?;

            let resumes = sqlx::query!(
                "SELECT value, approver, resume_id FROM resume_job WHERE job = $1 ORDER BY created_at ASC",
                last
            )
            .fetch_all(&mut *tx)
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
                .execute(&mut *tx)
                .await?;

                /* If we are woken up after suspending, last_result will be the flow args, but we
                 * should use the result from the last job */
                if let FlowStatusModule::WaitingForEvents { .. } = &status_module {
                    last_result =
                        sqlx::query_scalar!("SELECT result FROM completed_job WHERE id = $1", last)
                            .fetch_one(&mut *tx)
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
                .execute(&mut *tx)
                .await?;

                tx.commit().await?;
                return Ok(());

            /* cancelled or we're WaitingForEvents but we don't have enough messages (timed out) */
            } else {
                tx.commit().await?;

                let success = false;
                let skipped = false;
                let logs = "Timed out waiting to be resumed".to_string();
                let result = json!({ "error": {"message": logs, "name": "SuspendedTimeout"}});
                let _uuid =
                    add_completed_job(db, &flow_job, success, skipped, &result, logs, rsmq).await?;

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

    let mut transform_context: Option<IdContext> = None;

    let args: windmill_common::error::Result<_> = if module.mock.is_some()
        && module.mock.as_ref().unwrap().enabled
    {
        let mut m = Map::new();
        let v = module
            .mock
            .as_ref()
            .unwrap()
            .return_value
            .clone()
            .ok_or_else(|| {
                Error::BadRequest(format!(
                    "mock enabled but no return_value specified for module {}",
                    module.id
                ))
            })?;
        m.insert("previous_result".to_string(), v);
        Ok(m)
    } else {
        match &module.value {
            FlowModuleValue::Script { input_transforms, .. }
            | FlowModuleValue::RawScript { input_transforms, .. }
            | FlowModuleValue::Flow { input_transforms, .. } => {
                let ctx = get_transform_context(&flow_job, &previous_id, &status).await?;
                transform_context = Some(ctx);
                let by_id = transform_context.as_ref().unwrap();
                transform_input(
                    &flow_job.args,
                    last_result.clone(),
                    input_transforms,
                    resume_messages.as_slice(),
                    approvers.clone(),
                    by_id,
                    client,
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
        }
    };

    let next_flow_transform = compute_next_flow_transform(
        flow_job,
        &flow,
        transform_context,
        db,
        &module,
        &status,
        &status_module,
        last_result.clone(),
        &previous_id,
        client,
        resume_messages.as_slice(),
        approvers.clone(),
    )
    .await?;
    tracing::info!(id = %flow_job.id, root_id = %job_root, "next flow transform computed");

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

    /* Finally, push the job into the queue */
    let mut uuids = vec![];

    let len = match &job_payloads {
        ContinuePayload::SingleJob(_) => 1,
        ContinuePayload::BranchAllJobs(payloads) => payloads.len(),
        ContinuePayload::ForloopJobs { n, .. } => *n,
    };

    let mut tx: QueueTransaction<'_, R> = (rsmq.clone(), db.begin().await?).into();

    for i in (0..len).into_iter() {
        let payload_tag = match &job_payloads {
            ContinuePayload::SingleJob(payload) => payload.clone(),
            ContinuePayload::BranchAllJobs(payloads) => payloads[i].clone(),
            ContinuePayload::ForloopJobs { payload, .. } => payload.clone(),
        };

        let transform_inp;
        let args = match &next_status {
            NextStatus::AllFlowJobs {
                branchall: Some(BranchAllStatus { .. }),
                iterator: None,
                ..
            } => args.as_ref().map(|args| args.clone()),
            NextStatus::NextLoopIteration(NextIteration { new_args, .. }) => {
                args.as_ref().map(|args| {
                    let mut args = args.clone();
                    args.extend(new_args.clone());
                    args
                })
            }
            NextStatus::AllFlowJobs {
                branchall: None,
                iterator: Some(Iterator { itered, .. }),
                simple_input_transforms,
            } => {
                if let Ok(args) = args.as_ref() {
                    let mut new_args = args.clone();
                    new_args.insert(
                        "iter".to_string(),
                        json!({ "index": i, "value": itered[i] }),
                    );
                    if let Some(input_transforms) = simple_input_transforms {
                        let ctx = get_transform_context(&flow_job, &previous_id, &status).await?;
                        transform_inp = transform_input(
                            &Some(serde_json::Value::Object(new_args)),
                            last_result.clone(),
                            input_transforms,
                            resume_messages.as_slice(),
                            approvers.clone(),
                            &ctx,
                            client,
                        )
                        .await;
                        transform_inp.as_ref().map(|args| args.clone())
                    } else {
                        Ok(new_args)
                    }
                } else {
                    args.as_ref().map(|args| args.clone())
                }
            }
            _ => args.as_ref().map(|args| args.clone()),
        };
        let (ok, err) = match args {
            Ok(v) => (Some(v), None),
            Err(e) => (None, Some(e)),
        };
        let root_job = if matches!(
            module.value,
            FlowModuleValue::Flow { .. } | FlowModuleValue::ForloopFlow { parallel: true, .. }
        ) {
            None
        } else {
            flow_job.root_job.or_else(|| Some(flow_job.id))
        };

        let tx2 = PushIsolationLevel::Transaction(tx);
        let (uuid, mut inner_tx) = push(
            &db,
            tx2,
            &flow_job.workspace_id,
            payload_tag.payload,
            ok.unwrap_or_else(|| Map::new()),
            &flow_job.created_by,
            &flow_job.email,
            flow_job.permissioned_as.to_owned(),
            scheduled_for_o,
            flow_job.schedule_path.clone(),
            Some(flow_job.id),
            root_job,
            None,
            true,
            continue_on_same_worker,
            err,
            flow_job.visible_to_owner,
            if flow_job.tag == "flow" {
                payload_tag.tag
            } else {
                Some(flow_job.tag.clone())
            },
            module.timeout,
            Some(module.id.clone()),
        )
        .await?;

        if let FlowModuleValue::ForloopFlow { parallelism: Some(p), .. } = &module.value {
            if i as u16 >= *p {
                sqlx::query!(
                    "
                UPDATE queue
                   SET suspend = $1, suspend_until = now() + interval '14 day', running = true
                 WHERE id = $2
                ",
                    (i as u16 - p + 1) as i32,
                    uuid,
                )
                .execute(&mut inner_tx)
                .await?;
            }
        }
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
        NextStatus::AllFlowJobs { iterator, branchall, .. } => FlowStatusModule::InProgress {
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
        .execute(db)
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
        .execute(db)
        .await?;
    };

    tx.commit().await?;
    tracing::info!(id = %flow_job.id, root_id = %job_root, "all next flow jobs pushed");

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
        simple_input_transforms: Option<HashMap<String, InputTransform>>,
    },
}

#[derive(Clone)]
struct JobPayloadWithTag {
    payload: JobPayload,
    tag: Option<String>,
}
enum ContinuePayload {
    SingleJob(JobPayloadWithTag),
    ForloopJobs { n: usize, payload: JobPayloadWithTag },
    BranchAllJobs(Vec<JobPayloadWithTag>),
}

enum NextFlowTransform {
    EmptyInnerFlows,
    Continue(ContinuePayload, NextStatus),
}

async fn compute_next_flow_transform(
    flow_job: &QueuedJob,
    flow: &FlowValue,
    by_id: Option<IdContext>,
    db: &DB,
    module: &FlowModule,
    status: &FlowStatus,
    status_module: &FlowStatusModule,
    last_result: serde_json::Value,
    previous_id: &str,
    client: &AuthedClient,
    resumes: &[Value],
    approvers: Vec<String>,
) -> error::Result<NextFlowTransform> {
    if module.mock.is_some() && module.mock.as_ref().unwrap().enabled {
        return Ok(NextFlowTransform::Continue(
            ContinuePayload::SingleJob(JobPayloadWithTag {
                payload: JobPayload::Identity,
                tag: None,
            }),
            NextStatus::NextStep,
        ));
    }
    let trivial_next_job = |payload| {
        Ok(NextFlowTransform::Continue(
            ContinuePayload::SingleJob(JobPayloadWithTag { payload, tag: None }),
            NextStatus::NextStep,
        ))
    };
    match &module.value {
        FlowModuleValue::Identity => trivial_next_job(JobPayload::Identity),
        FlowModuleValue::Flow { path, .. } => {
            let payload = flow_to_payload(path);
            Ok(NextFlowTransform::Continue(
                ContinuePayload::SingleJob(payload),
                NextStatus::NextStep,
            ))
        }
        FlowModuleValue::Script { path: script_path, hash: script_hash, .. } => {
            let payload = script_to_payload(script_hash, script_path, db, flow_job, module).await?;
            Ok(NextFlowTransform::Continue(
                ContinuePayload::SingleJob(payload),
                NextStatus::NextStep,
            ))
        }
        FlowModuleValue::RawScript {
            path,
            content,
            language,
            lock,
            tag,
            concurrent_limit,
            concurrency_time_window_s,
            ..
        } => {
            let path = path
                .clone()
                .or_else(|| Some(format!("{}/step-{}", flow_job.script_path(), status.step)));
            let payload = raw_script_to_payload(
                path,
                content,
                language,
                lock,
                concurrent_limit,
                concurrency_time_window_s,
                module,
                tag,
            );
            Ok(NextFlowTransform::Continue(
                ContinuePayload::SingleJob(payload),
                NextStatus::NextStep,
            ))
        }
        /* forloop modules are expected set `iter: { value: Value, index: usize }` as job arguments */
        FlowModuleValue::ForloopFlow { modules, iterator, parallel, .. } => {
            let new_args: &mut Map<String, serde_json::Value> = &mut Map::new();
            // if it's a simple single step flow, we will collapse it as an optimization and need to pass flow_input as an arg
            let is_simple =
                modules.len() == 1 && modules[0].value.is_simple() && flow.failure_module.is_none();

            let next_loop_status = match status_module {
                FlowStatusModule::WaitingForPriorSteps { .. }
                | FlowStatusModule::WaitingForEvents { .. }
                | FlowStatusModule::WaitingForExecutor { .. } => {
                    let by_id = if let Some(x) = by_id {
                        x
                    } else {
                        get_transform_context(&flow_job, previous_id, &status).await?
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
                                (
                                    "resume".to_string(),
                                    resumes.last().map(|v| json!(v)).unwrap_or_default(),
                                ),
                                ("resumes".to_string(), resumes.clone().into()),
                                ("approvers".to_string(), json!(approvers.clone())),
                            ]
                        },
                        Some(client),
                        Some(by_id),
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
                LoopStatus::EmptyIterator => Ok(NextFlowTransform::EmptyInnerFlows),
                LoopStatus::NextIteration(ns) => {
                    let mut fm = flow.failure_module.clone();
                    if let Some(mut failure_module) = flow.failure_module.clone() {
                        failure_module.id_append(&format!("{}/{}", status.step, ns.index));
                        fm = Some(failure_module);
                    }
                    let modules = (*modules).clone();
                    let inner_path = Some(format!("{}/loop-{}", flow_job.script_path(), ns.index));
                    let continue_payload = ContinuePayload::SingleJob(JobPayloadWithTag {
                        payload: JobPayload::RawFlow {
                            value: FlowValue {
                                modules,
                                failure_module: fm,
                                same_worker: flow.same_worker,
                                concurrent_limit: None,
                                concurrency_time_window_s: None,
                                skip_expr: None,
                                cache_ttl: None,
                            },
                            path: inner_path,
                        },
                        tag: None,
                    });
                    Ok(NextFlowTransform::Continue(
                        continue_payload,
                        NextStatus::NextLoopIteration(ns),
                    ))
                }
                LoopStatus::ParallelIteration { itered, .. } => {
                    let inner_path = Some(format!("{}/loop-parrallel", flow_job.script_path(),));
                    let continue_payload = if is_simple {
                        let payload = match &modules[0].value {
                            FlowModuleValue::Flow { path, .. } => flow_to_payload(path),
                            FlowModuleValue::Script {
                                path: script_path,
                                hash: script_hash,
                                ..
                            } => {
                                script_to_payload(script_hash, script_path, db, flow_job, module)
                                    .await?
                            }
                            FlowModuleValue::RawScript {
                                path,
                                content,
                                language,
                                lock,
                                tag,
                                concurrent_limit,
                                concurrency_time_window_s,
                                ..
                            } => raw_script_to_payload(
                                path.clone().or(inner_path),
                                content,
                                language,
                                lock,
                                concurrent_limit,
                                concurrency_time_window_s,
                                module,
                                tag,
                            ),
                            _ => unreachable!("is simple flow"),
                        };
                        ContinuePayload::ForloopJobs { n: itered.len(), payload: payload }
                    } else {
                        let payload = {
                            JobPayloadWithTag {
                                payload: JobPayload::RawFlow {
                                    value: FlowValue {
                                        modules: (*modules).clone(),
                                        failure_module: flow.failure_module.clone(),
                                        same_worker: flow.same_worker,
                                        concurrent_limit: None,
                                        concurrency_time_window_s: None,
                                        skip_expr: None,
                                        cache_ttl: None,
                                    },
                                    path: Some(format!("{}/forloop", flow_job.script_path())),
                                },
                                tag: None,
                            }
                        };
                        ContinuePayload::ForloopJobs { n: itered.len(), payload }
                    };
                    Ok(NextFlowTransform::Continue(
                        continue_payload,
                        NextStatus::AllFlowJobs {
                            branchall: None,
                            iterator: Some(windmill_common::flow_status::Iterator {
                                index: 0,
                                itered,
                            }),
                            simple_input_transforms: if is_simple {
                                match &modules[0].value {
                                    FlowModuleValue::Script { input_transforms, .. }
                                    | FlowModuleValue::RawScript { input_transforms, .. }
                                    | FlowModuleValue::Flow { input_transforms, .. } => {
                                        Some(input_transforms.clone())
                                    }
                                    _ => None,
                                }
                            } else {
                                None
                            },
                        },
                    ))
                }
            }
        }
        FlowModuleValue::BranchOne { branches, default, .. } => {
            let branch = match status_module {
                FlowStatusModule::WaitingForPriorSteps { .. }
                | FlowStatusModule::WaitingForEvents { .. }
                | FlowStatusModule::WaitingForExecutor { .. } => {
                    let mut branch_chosen = BranchChosen::Default;
                    let idcontext = get_transform_context(&flow_job, previous_id, &status).await?;
                    for (i, b) in branches.iter().enumerate() {
                        let pred = compute_bool_from_expr(
                            b.expr.to_string(),
                            &flow_job.args,
                            last_result.clone(),
                            Some(idcontext.clone()),
                            Some(client),
                            Some((resumes, approvers.clone())),
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
            Ok(NextFlowTransform::Continue(
                ContinuePayload::SingleJob(JobPayloadWithTag {
                    payload: JobPayload::RawFlow {
                        value: FlowValue {
                            modules,
                            failure_module: fm,
                            same_worker: flow.same_worker,
                            concurrent_limit: None,
                            concurrency_time_window_s: None,
                            skip_expr: None,
                            cache_ttl: None,
                        },
                        path: Some(format!(
                            "{}/branchone-{}",
                            flow_job.script_path(),
                            status.step
                        )),
                    },
                    tag: None,
                }),
                NextStatus::BranchChosen(branch),
            ))
        }
        FlowModuleValue::BranchAll { branches, parallel, .. } => {
            let (branch_status, flow_jobs) = match status_module {
                FlowStatusModule::WaitingForPriorSteps { .. }
                | FlowStatusModule::WaitingForEvents { .. }
                | FlowStatusModule::WaitingForExecutor { .. } => {
                    if branches.is_empty() {
                        return Ok(NextFlowTransform::EmptyInnerFlows);
                    } else if *parallel {
                        return Ok(NextFlowTransform::Continue(
                            ContinuePayload::BranchAllJobs(
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
                                        JobPayloadWithTag {
                                            payload: JobPayload::RawFlow {
                                                value: FlowValue {
                                                    modules: b.modules.clone(),
                                                    failure_module: fm.clone(),
                                                    same_worker: flow.same_worker,
                                                    concurrent_limit: None,
                                                    concurrency_time_window_s: None,
                                                    skip_expr: None,
                                                    cache_ttl: None,
                                                },
                                                path: Some(format!(
                                                    "{}/branchall-{}",
                                                    flow_job.script_path(),
                                                    i
                                                )),
                                            },
                                            tag: None,
                                        }
                                    })
                                    .collect(),
                            ),
                            NextStatus::AllFlowJobs {
                                branchall: Some(BranchAllStatus {
                                    branch: 0,
                                    previous_result: last_result,
                                    len: branches.len(),
                                }),
                                iterator: None,
                                simple_input_transforms: None,
                            },
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
            Ok(NextFlowTransform::Continue(
                ContinuePayload::SingleJob(JobPayloadWithTag {
                    payload: JobPayload::RawFlow {
                        value: FlowValue {
                            modules,
                            failure_module: fm.clone(),
                            same_worker: flow.same_worker,
                            concurrent_limit: None,
                            concurrency_time_window_s: None,
                            skip_expr: None,
                            cache_ttl: None,
                        },
                        path: Some(format!(
                            "{}/branchall-{}",
                            flow_job.script_path(),
                            branch_status.branch
                        )),
                    },
                    tag: None,
                }),
                NextStatus::NextBranchStep(NextBranch { status: branch_status, flow_jobs }),
            ))
        }
    }
}

fn raw_script_to_payload(
    path: Option<String>,
    content: &String,
    language: &windmill_common::scripts::ScriptLang,
    lock: &Option<String>,
    concurrent_limit: &Option<i32>,
    concurrency_time_window_s: &Option<i32>,
    module: &FlowModule,
    tag: &Option<String>,
) -> JobPayloadWithTag {
    JobPayloadWithTag {
        payload: JobPayload::Code(RawCode {
            path,
            content: content.clone(),
            language: language.clone(),
            lock: lock.clone(),
            concurrent_limit: *concurrent_limit,
            concurrency_time_window_s: *concurrency_time_window_s,
            cache_ttl: module.cache_ttl.map(|x| x as i32),
        }),
        tag: tag.clone(),
    }
}

fn flow_to_payload(path: &str) -> JobPayloadWithTag {
    let payload = JobPayload::Flow(path.to_string());
    JobPayloadWithTag { payload, tag: None }
}

async fn script_to_payload(
    script_hash: &Option<windmill_common::scripts::ScriptHash>,
    script_path: &String,
    db: &sqlx::Pool<sqlx::Postgres>,
    flow_job: &QueuedJob,
    module: &FlowModule,
) -> Result<JobPayloadWithTag, Error> {
    let (payload, tag) = if script_hash.is_none() {
        script_path_to_payload(script_path, &db, &flow_job.workspace_id).await?
    } else {
        let hash = script_hash.clone().unwrap();
        let mut tx: sqlx::Transaction<'_, sqlx::Postgres> = db.begin().await?;
        let (
            tag,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            language,
            dedicated_worker,
        ) = script_hash_to_tag_and_limits(&hash, &mut tx, &flow_job.workspace_id).await?;
        (
            JobPayload::ScriptHash {
                hash,
                path: script_path.to_owned(),
                concurrent_limit,
                concurrency_time_window_s,
                cache_ttl: module.cache_ttl.map(|x| x as i32).ok_or(cache_ttl).ok(),
                language,
                dedicated_worker,
            },
            tag,
        )
    };
    Ok(JobPayloadWithTag { payload, tag })
}

async fn get_transform_context(
    flow_job: &QueuedJob,
    previous_id: &str,
    status: &FlowStatus,
) -> error::Result<IdContext> {
    let steps_results: HashMap<String, JobResult> = status
        .modules
        .iter()
        .filter_map(|x| x.job_result().map(|y| (x.id(), y)))
        .collect();

    Ok(IdContext { flow_job: flow_job.id, steps_results, previous_id: previous_id.to_string() })
}

async fn evaluate_with<F>(
    transform: InputTransform,
    vars: F,
    client: Option<&AuthedClient>,
    by_id: Option<IdContext>,
) -> anyhow::Result<serde_json::Value>
where
    F: FnOnce() -> Vec<(String, serde_json::Value)>,
{
    match transform {
        InputTransform::Static { value } => Ok(value),
        InputTransform::Javascript { expr } => eval_timeout(expr, vars(), client, by_id).await,
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
