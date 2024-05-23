/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::common::{hash_args, save_in_cache};
use crate::js_eval::{eval_timeout, IdContext};
use crate::{AuthedClient, PreviousResult, SameWorkerPayload, SendResult, KEEP_JOB_DIR};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::{json, Value};
use sqlx::types::Json;
use sqlx::FromRow;
use tokio::sync::mpsc::Sender;
use tracing::instrument;
use uuid::Uuid;
use windmill_common::flow_status::{
    ApprovalConditions, FlowStatusModuleWParent, Iterator, JobResult,
};
use windmill_common::flows::add_virtual_items_if_necessary;
use windmill_common::jobs::{
    script_hash_to_tag_and_limits, script_path_to_payload, BranchResults, JobPayload, QueuedJob,
    RawCode,
};
use windmill_common::worker::to_raw_value;
use windmill_common::{
    error::{self, to_anyhow, Error},
    flow_status::{
        Approval, BranchAllStatus, BranchChosen, FlowStatus, FlowStatusModule, RetryStatus,
        MAX_RETRY_ATTEMPTS, MAX_RETRY_INTERVAL,
    },
    flows::{FlowModule, FlowModuleValue, FlowValue, InputTransform, Retry, Suspend},
};
use windmill_queue::schedule::get_schedule_opt;
use windmill_queue::{
    add_completed_job, add_completed_job_error, append_logs, get_queued_job,
    handle_maybe_scheduled_job, CanceledBy, PushIsolationLevel, WrappedError,
};

type DB = sqlx::Pool<sqlx::Postgres>;

use windmill_queue::{canceled_job_to_result, get_queued_job_tx, push, QueueTransaction};

// #[instrument(level = "trace", skip_all)]
pub async fn update_flow_status_after_job_completion<
    'a,
    R: rsmq_async::RsmqConnection + Send + Sync + Clone,
>(
    db: &DB,
    client: &AuthedClient,
    flow: uuid::Uuid,
    job_id_for_status: &Uuid,
    w_id: &str,
    success: bool,
    result: &'a RawValue,
    unrecoverable: bool,
    same_worker_tx: Sender<SameWorkerPayload>,
    worker_dir: &str,
    stop_early_override: Option<bool>,
    rsmq: Option<R>,
    worker_name: &str,
    job_completed_tx: Sender<SendResult>,
) -> error::Result<()> {
    // this is manual tailrecursion because async_recursion blows up the stack
    // todo!();
    potentially_crash_for_testing();

    let mut rec = update_flow_status_after_job_completion_internal(
        db,
        client,
        flow,
        job_id_for_status,
        w_id,
        success,
        result,
        unrecoverable,
        same_worker_tx.clone(),
        worker_dir,
        stop_early_override,
        false,
        rsmq.clone(),
        worker_name,
        job_completed_tx.clone(),
    )
    .await?;
    while let Some(nrec) = rec {
        potentially_crash_for_testing();
        rec = match update_flow_status_after_job_completion_internal(
            db,
            client,
            nrec.flow,
            &nrec.job_id_for_status,
            w_id,
            nrec.success,
            nrec.result.as_ref(),
            false,
            same_worker_tx.clone(),
            worker_dir,
            nrec.stop_early_override,
            nrec.skip_error_handler,
            rsmq.clone(),
            worker_name,
            job_completed_tx.clone(),
        )
        .await
        {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("Error while updating flow status of {} after  completion of {}, updating flow status again with error: {e}", nrec.flow,&nrec.job_id_for_status);
                update_flow_status_after_job_completion_internal(
                    db,
                    client,
                    nrec.flow,
                    &nrec.job_id_for_status,
                    w_id,
                    false,
                    &to_raw_value(&Json(&WrappedError { error: json!(e.to_string()) })),
                    true,
                    same_worker_tx.clone(),
                    worker_dir,
                    nrec.stop_early_override,
                    nrec.skip_error_handler,
                    rsmq.clone(),
                    worker_name,
                    job_completed_tx.clone(),
                )
                .await?
            }
        }
    }
    Ok(())
}
pub struct RecUpdateFlowStatusAfterJobCompletion {
    flow: uuid::Uuid,
    job_id_for_status: Uuid,
    success: bool,
    result: Box<RawValue>,
    stop_early_override: Option<bool>,
    skip_error_handler: bool,
}

#[derive(FromRow)]
pub struct SkipIfStopped {
    pub skip_if_stopped: Option<bool>,
    pub stop_early_expr: Option<String>,
    pub continue_on_error: Option<bool>,
    pub args: Option<Json<HashMap<String, Box<RawValue>>>>,
}

#[derive(sqlx::FromRow, Deserialize)]
pub struct RowFlowStatus {
    pub flow_status: sqlx::types::Json<Box<serde_json::value::RawValue>>,
}
// #[instrument(level = "trace", skip_all)]
pub async fn update_flow_status_after_job_completion_internal<
    'a,
    R: rsmq_async::RsmqConnection + Send + Sync + Clone,
>(
    db: &DB,
    client: &AuthedClient,
    flow: uuid::Uuid,
    job_id_for_status: &Uuid,
    w_id: &str,
    mut success: bool,
    result: &'a RawValue,
    unrecoverable: bool,
    same_worker_tx: Sender<SameWorkerPayload>,
    worker_dir: &str,
    stop_early_override: Option<bool>,
    skip_error_handler: bool,
    rsmq: Option<R>,
    worker_name: &str,
    job_completed_tx: Sender<SendResult>,
) -> error::Result<Option<RecUpdateFlowStatusAfterJobCompletion>> {
    let (
        should_continue_flow,
        flow_job,
        stop_early,
        skip_if_stop_early,
        nresult,
        is_failure_step,
        _cleanup_module,
    ) = {
        // tracing::debug!("UPDATE FLOW STATUS: {flow:?} {success} {result:?} {w_id} {depth}");

        let old_status_json = sqlx::query_as::<_, RowFlowStatus>(
            "SELECT flow_status FROM queue WHERE id = $1 AND workspace_id = $2",
        )
        .bind(flow)
        .bind(w_id)
        .fetch_one(db)
        .await
        .map_err(|e| {
            Error::InternalErr(format!(
                "fetching flow status {flow} while reporting {success} {result:?}: {e}"
            ))
        })?;

        let old_status = serde_json::from_str::<FlowStatus>(old_status_json.flow_status.get())
            .or_else(|e| {
                Err(Error::InternalErr(format!(
                    "requiring status to be parsable as FlowStatus: {e:?}"
                )))
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

        let (mut stop_early, skip_if_stop_early, continue_on_error) = if let Some(se) =
            stop_early_override
        {
            //do not stop early if module is a flow step
            let flow_job = get_queued_job(&flow, w_id, db)
                .await?
                .ok_or_else(|| Error::InternalErr(format!("requiring flow to be in the queue")))?;
            let module = get_module(&flow_job, module_index);

            if module.is_some_and(|x| x.is_flow()) {
                (false, false, false)
            } else {
                (true, se, false)
            }
        } else if is_failure_step {
            (false, false, false)
        } else {
            let row = sqlx::query(
            "SELECT 
                    raw_flow->'modules'->$1::int->'stop_after_if'->>'expr' as stop_early_expr,
                    (raw_flow->'modules'->$1::int->'stop_after_if'->>'skip_if_stopped')::bool as skip_if_stopped,
                    (raw_flow->'modules'->$1::int->'continue_on_error')::bool as continue_on_error,
                    args 
                FROM queue
                WHERE id = $2"
            )
            .bind(old_status.step)
            .bind(flow)
            .fetch_one(db)
            .await
            .map_err(|e| Error::InternalErr(format!("retrieval of stop_early_expr from state: {e}")))?;
            let r = SkipIfStopped::from_row(&row)?;

            let stop_early = success
                && if let Some(expr) = r.stop_early_expr.clone() {
                    compute_bool_from_expr(
                        expr,
                        Arc::new(
                            r.args
                                .map(|x| x.0)
                                .unwrap_or_else(|| serde_json::from_str("{}").unwrap())
                                .to_owned(),
                        ),
                        Arc::new(result.to_owned()),
                        None,
                        Some(client),
                        None,
                    )
                    .await?
                } else {
                    false
                };
            (
                stop_early,
                r.skip_if_stopped.unwrap_or(false),
                r.continue_on_error.unwrap_or(false),
            )
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
                            "UPDATE queue
                            SET flow_status = JSONB_SET(flow_status, ARRAY['modules', $1::TEXT, 'iterator', 'index'], ((flow_status->'modules'->$1::int->'iterator'->>'index')::int + 1)::text::jsonb)
                            WHERE id = $2
                            RETURNING (flow_status->'modules'->$1::int->'iterator'->>'index')::int",
                            old_status.step,
                            flow
                        )
                        .fetch_one(&mut tx)
                        .await.map_err(|e| {
                            Error::InternalErr(format!(
                                "error while fetching iterator index: {e}"
                            ))
                        })?
                        .ok_or_else(|| Error::InternalErr(format!("requiring an index in InProgress")))?;
                        tracing::info!(
                            "parallel iteration {job_id_for_status} of flow {flow} update nindex: {nindex} len: {len}",
                            nindex = nindex,
                            len = itered.len()
                        );
                        (nindex, itered.len() as i32)
                    }
                    (_, Some(BranchAllStatus { len, .. })) => {
                        let nindex = sqlx::query_scalar!(
                            "UPDATE queue
                            SET flow_status = JSONB_SET(flow_status, ARRAY['modules', $1::TEXT, 'branchall', 'branch'], ((flow_status->'modules'->$1::int->'branchall'->>'branch')::int + 1)::text::jsonb)
                            WHERE id = $2
                            RETURNING (flow_status->'modules'->$1::int->'branchall'->>'branch')::int",
                            old_status.step,
                            flow
                        )
                        .fetch_one(&mut tx)
                        .await
                        .map_err(|e| {
                            Error::InternalErr(format!(
                                "error while fetching branchall index: {e}"
                            ))
                        })?
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
                            "SELECT success FROM completed_job WHERE id = ANY($1)",
                            jobs.as_slice()
                        )
                        .fetch_all(&mut tx)
                        .await
                        .map_err(|e| {
                            Error::InternalErr(format!(
                                "error while fetching sucess from completed_jobs: {e}"
                            ))
                        })?
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
                            failed_retries: vec![],
                        }
                    } else {
                        success = false;
                        FlowStatusModule::Failure {
                            id: module_status.id(),
                            job: job_id_for_status.clone(),
                            flow_jobs: Some(jobs.clone()),
                            branch_chosen: None,
                            failed_retries: vec![],
                        }
                    };
                    let r = sqlx::query_scalar!(
                        "DELETE FROM parallel_monitor_lock WHERE parent_flow_id = $1 RETURNING last_ping",
                        flow,
                    ).fetch_optional(db).await.map_err(|e| {
                        Error::InternalErr(format!(
                            "error while deleting parallel_monitor_lock: {e}"
                        ))
                    })?;
                    if r.is_some() {
                        tracing::info!(
                            "parallel flow has removed lock on its parent, last ping was {:?}",
                            r.unwrap()
                        );
                    }
                    tracing::info!(
                        "parallel iteration {job_id_for_status} of flow {flow} has finished",
                    );

                    (true, Some(new_status))
                } else {
                    tx.commit().await?;

                    if parallelism.is_some() {
                        // this ensure that the lock is taken in the same order and thus avoid deadlocks
                        let ids = sqlx::query_scalar!(
                            "SELECT id FROM queue WHERE parent_job = $1 AND suspend > 0 ORDER by suspend",
                            flow
                        )
                        .fetch_all(db)
                        .await
                        .map_err(|e| {
                            Error::InternalErr(format!("error while locking jobs to decrease parallelism of: {e}"))
                        })?;
                        for id in ids {
                            sqlx::query!(
                                "UPDATE queue SET suspend = suspend - 1 WHERE id = $1 AND suspend > 0",
                                id
                            )
                            .execute(db)
                            .await
                            .map_err(|e| {
                                Error::InternalErr(format!(
                                    "error decreasing suspend for {id}: {e}"
                                ))
                            })?;
                        }
                    }

                    sqlx::query!(
                        "UPDATE queue
                        SET last_ping = null
                        WHERE id = $1",
                        flow
                    )
                    .execute(db)
                    .await
                    .map_err(|e| {
                        Error::InternalErr(format!("error while setting last ping to null: {e}"))
                    })?;

                    let r = sqlx::query_scalar!(
                        "DELETE FROM parallel_monitor_lock WHERE parent_flow_id = $1 and job_id = $2 RETURNING last_ping",
                        flow,
                        job_id_for_status
                    ).fetch_optional(db).await.map_err(|e| {
                        Error::InternalErr(format!("error while removing parallel_monitor_lock: {e}"))
                    })?;
                    if r.is_some() {
                        tracing::info!(
                            "parallel flow has removed lock on its parent, last ping was {:?}",
                            r.unwrap()
                        );
                    }

                    return Ok(None);
                }
            }
            FlowStatusModule::InProgress {
                iterator: Some(windmill_common::flow_status::Iterator { index, itered, .. }),
                while_loop,
                ..
            } if (*while_loop
                || (*index + 1 < itered.len()) && (success || skip_loop_failures))
                && !stop_early =>
            {
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
                    FlowStatusModule::Success { flow_jobs, branch_chosen, .. } => {
                        (flow_jobs.clone(), branch_chosen.clone())
                    }
                    FlowStatusModule::Failure { flow_jobs, branch_chosen, .. } => {
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
                            failed_retries: old_status.retry.failed_jobs.clone(),
                        }),
                    )
                } else {
                    let inc = if continue_on_error {
                        let retry = sqlx::query_scalar!(
                            "SELECT raw_flow->'modules'->$2::int->'retry' FROM queue WHERE id = $1",
                            flow,
                            old_status.step
                        )
                        .fetch_optional(&mut tx)
                        .await
                        .map_err(|e| {
                            Error::InternalErr(format!("error while getting retry from step: {e}"))
                        })?
                        .flatten();

                        let retry = retry
                            .map(|x| serde_json::from_value::<Retry>(x).ok())
                            .flatten()
                            .unwrap_or_default();
                        tracing::error!("UPDATE FLOW STATUS 2: {retry:#?} ");
                        next_retry(&retry, &old_status.retry).is_none()
                    } else {
                        false
                    };
                    (
                        inc,
                        Some(FlowStatusModule::Failure {
                            id: module_status.id(),
                            job: job_id_for_status.clone(),
                            flow_jobs,
                            branch_chosen,
                            failed_retries: old_status.retry.failed_jobs.clone(),
                        }),
                    )
                }
            }
        };

        let step_counter = if inc_step_counter {
            sqlx::query!(
                "UPDATE queue
                SET flow_status = JSONB_SET(flow_status, ARRAY['step'], $1)
                WHERE id = $2",
                json!(old_status.step + 1),
                flow
            )
            .execute(&mut tx)
            .await
            .map_err(|e| {
                Error::InternalErr(format!("error while setting flow index for {flow}: {e}"))
            })?;
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
                .await.map_err(|e| {
                    Error::InternalErr(format!(
                        "error while fetching failure module: {e}"
                    ))
                })?;

                sqlx::query!(
                    "UPDATE queue
                    SET flow_status = JSONB_SET(flow_status, ARRAY['failure_module'], $1)
                    WHERE id = $2",
                    json!(FlowStatusModuleWParent {
                        parent_module,
                        module_status: new_status.clone()
                    }),
                    flow
                )
                .execute(&mut tx)
                .await
                .map_err(|e| {
                    Error::InternalErr(format!(
                        "error while setting flow status in failure step: {e}"
                    ))
                })?;
            } else {
                sqlx::query!(
                    "UPDATE queue
                    SET flow_status = JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2)
                    WHERE id = $3",
                    old_status.step.to_string(),
                    json!(new_status),
                    flow
                )
                .execute(&mut tx)
                .await
                .map_err(|e| {
                    Error::InternalErr(format!("error while setting new flow status: {e}"))
                })?;

                if let Some(job_result) = new_status.job_result() {
                    sqlx::query!(
                        "UPDATE queue
                        SET leaf_jobs = JSONB_SET(coalesce(leaf_jobs, '{}'::jsonb), ARRAY[$1::TEXT], $2)
                        WHERE COALESCE((SELECT root_job FROM queue WHERE id = $3), $3) = id",
                        new_status.id(),
                        json!(job_result),
                        flow
                    )
                    .execute(&mut tx)
                    .await.map_err(|e| {
                        Error::InternalErr(format!(
                            "error while setting leaf jobs: {e}"
                        ))
                    })?;
                }
            }
        }

        let nresult = match &new_status {
            Some(FlowStatusModule::Success { flow_jobs: Some(jobs), .. })
            | Some(FlowStatusModule::Failure { flow_jobs: Some(jobs), .. }) => {
                retrieve_flow_jobs_results(db, w_id, jobs).await?
            }
            _ => result.to_owned(),
        };

        if old_status.retry.fail_count > 0
            && matches!(&new_status, Some(FlowStatusModule::Success { .. }))
        {
            sqlx::query(
                "UPDATE queue
                SET flow_status = flow_status - 'retry'
                WHERE id = $1
                RETURNING flow_status",
            )
            .bind(flow)
            .execute(&mut tx)
            .await
            .context("remove flow status retry")?;
        }

        let flow_job = get_queued_job_tx(flow, w_id, tx.transaction_mut())
            .await?
            .ok_or_else(|| Error::InternalErr(format!("requiring flow to be in the queue")))?;

        let job_root = flow_job
            .root_job
            .map(|x| x.to_string())
            .unwrap_or_else(|| "none".to_string());
        tracing::info!(id = %flow_job.id, root_id = %job_root, "update flow status");

        let module = get_module(&flow_job, module_index);

        // tracing::error!(
        //     "UPDATE FLOW STATUS 3: {module:#?} {unrecoverable} {} {is_last_step} {success} {skip_error_handler}", flow_job.canceled
        // );
        let should_continue_flow = match success {
            _ if stop_early => false,
            _ if flow_job.canceled => false,
            true => !is_last_step,
            false if unrecoverable => false,
            false if skip_branch_failure || skip_loop_failures || continue_on_error => {
                !is_last_step
            }
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

        tx.commit().await?;
        tracing::debug!(id = %flow_job.id, root_id = %job_root, "flow status updated");

        (
            should_continue_flow,
            flow_job,
            stop_early,
            skip_if_stop_early,
            nresult,
            is_failure_step,
            old_status.cleanup_module,
        )
    };

    let done = if !should_continue_flow {
        {
            let logs = if flow_job.canceled {
                "Flow job canceled\n".to_string()
            } else if stop_early {
                format!("Flow job stopped early because of a stop early predicate returning true\n")
            } else if success {
                "Flow job completed with success\n".to_string()
            } else {
                "Flow job completed with error\n".to_string()
            };
            append_logs(flow_job.id, w_id.to_string(), logs, db).await;
        }
        #[cfg(feature = "enterprise")]
        if flow_job.parent_job.is_none() {
            // run the cleanup step only when the root job is complete
            if !_cleanup_module.flow_jobs_to_clean.is_empty() {
                tracing::debug!(
                    "Cleaning up jobs arguments, result and logs as they were marked as delete_after_use {:?}",
                    _cleanup_module.flow_jobs_to_clean
                );
                sqlx::query!(
                    "UPDATE completed_job
                    SET logs = '##DELETED##', args = '{}'::jsonb, result = '{}'::jsonb
                    WHERE id = ANY($1)",
                    &_cleanup_module.flow_jobs_to_clean,
                )
                .execute(db)
                .await
                .map_err(|e| {
                    Error::InternalErr(format!("error while cleaning up completed_job: {e}"))
                })?;
            }
        }
        if flow_job.canceled {
            add_completed_job_error(
                db,
                &flow_job,
                0,
                Some(CanceledBy {
                    username: flow_job.canceled_by.clone(),
                    reason: flow_job.canceled_reason.clone(),
                }),
                canceled_job_to_result(&flow_job),
                rsmq.clone(),
                worker_name,
                true,
            )
            .await?;
        } else {
            if flow_job.cache_ttl.is_some() && success {
                let cached_res_path = {
                    let args_hash =
                        hash_args(db, client, w_id, job_id_for_status, &flow_job.args).await;
                    let flow_path = flow_job.script_path();
                    let version_hash = if let Some(rc) = flow_job.raw_flow.as_ref() {
                        use std::hash::Hasher;
                        let mut s = DefaultHasher::new();
                        serde_json::to_string(&rc.0)
                            .unwrap_or_default()
                            .hash(&mut s);
                        format!("flow_{}", hex::encode(s.finish().to_be_bytes()))
                    } else {
                        "flow_unknown".to_string()
                    };
                    format!("{flow_path}/cache/{version_hash}/{args_hash}")
                };

                save_in_cache(db, client, &flow_job, cached_res_path, &nresult).await;
            }
            let success = success && !is_failure_step && !skip_error_handler;
            if success {
                add_completed_job(
                    db,
                    &flow_job,
                    success,
                    stop_early && skip_if_stop_early,
                    Json(&nresult),
                    0,
                    None,
                    rsmq.clone(),
                    true,
                )
                .await?;
            } else {
                add_completed_job(
                    db,
                    &flow_job,
                    success,
                    stop_early && skip_if_stop_early,
                    Json(
                        &serde_json::from_str::<Value>(nresult.get()).unwrap_or_else(
                            |e| json!({"error": format!("Impossible to serialize error: {e}")}),
                        ),
                    ),
                    0,
                    None,
                    rsmq.clone(),
                    true,
                )
                .await?;
            }
        }
        true
    } else {
        tracing::debug!(id = %flow_job.id,  "start handle flow");
        match handle_flow(
            &flow_job,
            db,
            client,
            Some(nresult.to_owned()),
            same_worker_tx.clone(),
            worker_dir,
            rsmq.clone(),
            job_completed_tx,
        )
        .await
        {
            Err(err) => {
                let e = json!({"message": err.to_string(), "name": "InternalError"});
                append_logs(
                    flow_job.id,
                    w_id.to_string(),
                    format!("Unexpected error during flow chaining:\n{:#?}", e),
                    db,
                )
                .await;
                let _ = add_completed_job_error(
                    db,
                    &flow_job,
                    0,
                    None,
                    e,
                    rsmq.clone(),
                    worker_name,
                    true,
                )
                .await;
                true
            }
            Ok(_) => false,
        }
    };

    if done {
        if flow_job.same_worker && !KEEP_JOB_DIR.load(Ordering::Relaxed) {
            let _ = tokio::fs::remove_dir_all(format!("{worker_dir}/{}", flow_job.id)).await;
        }

        if let Some(parent_job) = flow_job.parent_job {
            tracing::info!(subflow_id = %flow_job.id, parent_id = %parent_job, "subflow is finished, updating parent flow status");

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

async fn retrieve_flow_jobs_results(
    db: &DB,
    w_id: &str,
    job_uuids: &Vec<Uuid>,
) -> error::Result<Box<RawValue>> {
    let results = sqlx::query(
        "SELECT result, id
        FROM completed_job
        WHERE id = ANY($1) AND workspace_id = $2",
    )
    .bind(job_uuids.as_slice())
    .bind(w_id)
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|r| {
        let br = BranchResults::from_row(&r).unwrap();
        (br.id, br.result.to_owned())
    })
    .collect::<HashMap<_, _>>();

    let results = job_uuids
        .iter()
        .map(|j| {
            results
                .get(j)
                .ok_or_else(|| Error::InternalErr(format!("missing job result for {}", j)))
        })
        .collect::<Result<Vec<_>, _>>()?;

    tracing::debug!("Retrieved results for flow jobs {:?}", results);
    Ok(to_raw_value(&results))
}

fn get_module(flow_job: &QueuedJob, module_index: Option<usize>) -> Option<FlowModule> {
    let raw_flow = flow_job.parse_raw_flow();
    if let Some(raw_flow) = raw_flow {
        if let Some(i) = module_index {
            if let Some(module) = raw_flow.modules.get(i) {
                Some(module.clone())
            } else {
                raw_flow.failure_module.map(|x| *x.clone())
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
        "SELECT (raw_flow->'modules'->$1->'value'->>'skip_failures')::bool, (raw_flow->'modules'->$1->'value'->>'parallelism')::int
        FROM queue
        WHERE id = $2",
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
        "SELECT (raw_flow->'modules'->$1->'value'->'branches'->$2->>'skip_failure')::bool
        FROM queue
        WHERE id = $3",
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
        "SELECT raw_flow->'failure_module' != 'null'::jsonb
        FROM queue
        WHERE id = $1",
    )
    .bind(flow)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| Error::InternalErr(format!("error during retrieval of has_failure_module: {e}")))
    .map(|v| v.unwrap_or(false))
}

// async fn retrieve_cleanup_module<'c>(flow_uuid: Uuid, db: &DB) -> Result<FlowCleanupModule, Error> {
//     tracing::warn!("Retrieving cleanup module of flow {}", flow_uuid);
//     let raw_value = sqlx::query_scalar!(
//         "SELECT flow_status->'cleanup_module' as cleanup_module
//         FROM queue
//         WHERE id = $1",
//         flow_uuid,
//     )
//     .fetch_one(db)
//     .await
//     .map_err(|e| Error::InternalErr(format!("error during retrieval of cleanup module: {e}")))?;

//     raw_value
//         .clone()
//         .and_then(|rv| serde_json::from_value::<FlowCleanupModule>(rv).ok())
//         .ok_or(Error::InternalErr(format!(
//             "Unable to parse flow cleanup module {:?}",
//             raw_value
//         )))
// }

fn next_retry(retry: &Retry, status: &RetryStatus) -> Option<(u16, Duration)> {
    (status.fail_count <= MAX_RETRY_ATTEMPTS)
        .then(|| &retry)
        .and_then(|retry| retry.interval(status.fail_count))
        .map(|d| (status.fail_count + 1, std::cmp::min(d, MAX_RETRY_INTERVAL)))
}

async fn compute_bool_from_expr(
    expr: String,
    flow_args: Arc<HashMap<String, Box<RawValue>>>,
    result: Arc<Box<RawValue>>,
    by_id: Option<IdContext>,
    client: Option<&AuthedClient>,
    resumes: Option<(Arc<Box<RawValue>>, Arc<Box<RawValue>>, Arc<Box<RawValue>>)>,
) -> error::Result<bool> {
    let mut context = HashMap::with_capacity(if resumes.is_some() { 7 } else { 3 });
    context.insert("result".to_string(), result.clone());
    context.insert("previous_result".to_string(), result.clone());

    if let Some(resumes) = resumes {
        context.insert("resume".to_string(), resumes.1);
        context.insert("resumes".to_string(), resumes.0);
        context.insert("approvers".to_string(), resumes.2);
    }

    match eval_timeout(
        format!("Boolean({expr})"),
        context,
        Some(flow_args),
        client,
        by_id,
    )
    .await?
    .get()
    {
        "true" => Ok(true),
        "false" => Ok(false),
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

#[derive(serde::Deserialize)]
pub struct ErrorValue<'a> {
    #[serde(borrow)]
    pub error: Option<&'a RawValue>,
}

/// resumes should be in order of timestamp ascending, so that more recent are at the end
#[instrument(level = "trace", skip_all)]
async fn transform_input(
    flow_args: Arc<HashMap<String, Box<RawValue>>>,
    last_result: Arc<Box<RawValue>>,
    input_transforms: &HashMap<String, InputTransform>,
    resumes: Arc<Box<RawValue>>,
    resume: Arc<Box<RawValue>>,
    approvers: Arc<Box<RawValue>>,
    by_id: &IdContext,
    client: &AuthedClient,
) -> windmill_common::error::Result<HashMap<String, Box<RawValue>>> {
    let mut mapped = HashMap::new();

    let mut env = HashMap::new();

    if input_transforms
        .iter()
        .any(|x| matches!(x.1, InputTransform::Javascript { .. }))
    {
        env.insert("params".to_string(), Arc::new(to_raw_value(&mapped)));
        env.insert("previous_result".to_string(), last_result);
        env.insert("resume".to_string(), resume);
        env.insert("resumes".to_string(), resumes);
        env.insert("approvers".to_string(), approvers);
    }

    for (key, val) in input_transforms.into_iter() {
        match val {
            InputTransform::Static { value } => {
                mapped.insert(key.to_string(), to_raw_value(&value));
            }
            InputTransform::Javascript { expr } => {
                let v = eval_timeout(
                    expr.to_string(),
                    env.clone(),
                    Some(flow_args.clone()),
                    Some(client),
                    Some(by_id.clone()),
                )
                .await
                .map_err(|e| {
                    Error::ExecutionErr(format!(
                        "Error during isolated evaluation of expression `{expr}`:\n{e}"
                    ))
                })?;
                mapped.insert(key.to_string(), v);
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
    last_result: Option<Box<RawValue>>,
    same_worker_tx: Sender<SameWorkerPayload>,
    worker_dir: &str,
    rsmq: Option<R>,
    job_completed_tx: Sender<SendResult>,
) -> anyhow::Result<()> {
    let flow = flow_job
        .parse_raw_flow()
        .with_context(|| "Unable to parse flow definition")?;
    let status = flow_job
        .parse_flow_status()
        .with_context(|| "Unable to parse flow status")?;

    if !flow_job.is_flow_step
        && status.retry.fail_count == 0
        && flow_job.schedule_path.is_some()
        && flow_job.script_path.is_some()
        && status.step == 0
    {
        let mut tx: QueueTransaction<'_, R> = (rsmq.clone(), db.begin().await?).into();

        let schedule_path = flow_job.schedule_path.as_ref().unwrap();

        let schedule =
            get_schedule_opt(tx.transaction_mut(), &flow_job.workspace_id, schedule_path).await?;

        tx.commit().await?;

        if let Some(schedule) = schedule {
            if let Err(err) = handle_maybe_scheduled_job(
                rsmq.clone(),
                db,
                flow_job,
                &schedule,
                flow_job.script_path.as_ref().unwrap(),
                &flow_job.workspace_id,
            )
            .await
            {
                match err {
                    Error::QuotaExceeded(_) => return Err(err.into()),
                    // scheduling next job failed and could not disable schedule => make zombie job to retry
                    _ => return Ok(()),
                }
            };
        } else {
            tracing::error!(
                "Schedule {schedule_path} in {} not found. Impossible to schedule again",
                &flow_job.workspace_id
            );
        }
    }

    push_next_flow_job(
        flow_job,
        status,
        flow,
        db,
        client,
        last_result.to_owned(),
        same_worker_tx,
        worker_dir,
        rsmq,
        job_completed_tx,
    )
    .await?;
    Ok(())
}

#[derive(Serialize, Debug)]
pub struct Iter {
    index: i32,
    value: Box<RawValue>,
}

#[derive(Serialize)]
pub struct MergeArgs<'a> {
    #[serde(flatten)]
    b: &'a Iter,

    #[serde(flatten)]
    a: HashMap<String, Box<RawValue>>,
}

#[derive(FromRow)]
pub struct ResumeRow {
    pub value: Json<Box<RawValue>>,
    pub approver: Option<String>,
    pub approved: bool,
    pub resume_id: i32,
}

#[derive(FromRow)]
pub struct RawArgs {
    pub args: Option<Json<HashMap<String, Box<RawValue>>>>,
}

lazy_static::lazy_static! {
    static ref CRASH_FORCEFULLY_AT_STEP: Option<usize> = std::env::var("CRASH_FORCEFULLY_AT_STEP")
        .ok()
        .and_then(|x| x.parse::<usize>().ok());

    static ref CRASH_STEP_COUNTER: AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
}

#[inline(always)]
fn potentially_crash_for_testing() {
    #[cfg(feature = "flow_testing")]
    if let Some(crash_at) = CRASH_FORCEFULLY_AT_STEP.as_ref() {
        let counter = CRASH_STEP_COUNTER.fetch_add(1, Ordering::SeqCst);
        if &counter == crash_at {
            panic!("CRASH#1 - expected crash for testing at step {}", crash_at);
        }
    }
}

// #[async_recursion]
// #[instrument(level = "trace", skip_all)]
async fn push_next_flow_job<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    flow_job: &QueuedJob,
    mut status: FlowStatus,
    flow: FlowValue,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClient,
    last_job_result: Option<Box<RawValue>>,
    same_worker_tx: Sender<SameWorkerPayload>,
    worker_dir: &str,
    rsmq: Option<R>,
    job_completed_tx: Sender<SendResult>,
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

    let flow_job_args = flow_job.get_args();

    // if this is an empty module of if the module has already been completed, successfully, update the parent flow
    if flow.modules.is_empty() || matches!(status_module, FlowStatusModule::Success { .. }) {
        job_completed_tx
            .send(SendResult::UpdateFlow {
                flow: flow_job.id,
                success: true,
                result: if flow.modules.is_empty() {
                    to_raw_value(&flow_job_args)
                } else {
                    // it has to be an empty for loop event
                    serde_json::from_str("[]").unwrap()
                },
                stop_early_override: None,
                w_id: flow_job.workspace_id.clone(),
                worker_dir: worker_dir.to_string(),
                token: client.token.clone(),
            })
            .await
            .map_err(|e| {
                Error::InternalErr(format!(
                    "error sending update flow message to job completed channel: {e}"
                ))
            })?;

        return Ok(());
    }

    let arc_flow_job_args = Arc::new(flow_job_args.clone());

    if i == 0 {
        if !flow_job.is_flow_step && flow_job.schedule_path.is_some() {
            let no_flow_overlap = sqlx::query_scalar!(
                "SELECT no_flow_overlap FROM schedule WHERE path = $1 AND workspace_id = $2",
                flow_job.schedule_path.as_ref().unwrap(),
                flow_job.workspace_id.as_str()
            )
            .fetch_one(db)
            .await?;
            if no_flow_overlap {
                let overlapping = sqlx::query_scalar!(
                    "SELECT id FROM queue WHERE schedule_path = $1 AND workspace_id = $2 AND id != $3 AND running = true",
                    flow_job.schedule_path.as_ref().unwrap(),
                    flow_job.workspace_id.as_str(),
                    flow_job.id
                ).fetch_all(db).await?;
                if overlapping.len() > 0 {
                    let overlapping_str = overlapping
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>()
                        .join(", ");
                    job_completed_tx
                        .send(SendResult::UpdateFlow {
                            flow: flow_job.id,
                            success: true,
                            result: serde_json::from_str(
                                &format!("\"not allowed to overlap with {overlapping_str}, scheduling next iteration\""),
                            )
                            .unwrap(),
                            stop_early_override: Some(true),
                            w_id: flow_job.workspace_id.clone(),
                            worker_dir: worker_dir.to_string(),
                            token: client.token.clone(),
                        })
                        .await
                        .map_err(|e| {
                            Error::InternalErr(format!(
                                "error sending update flow message to job completed channel: {e}"
                            ))
                        })?;

                    return Ok(());
                }
            }
        }
        if let Some(skip_expr) = &flow.skip_expr {
            let skip = compute_bool_from_expr(
                skip_expr.to_string(),
                arc_flow_job_args.clone(),
                Arc::new(to_raw_value(&json!("{}"))),
                None,
                Some(client),
                None,
            )
            .await?;
            if skip {
                job_completed_tx
                    .send(SendResult::UpdateFlow {
                        flow: flow_job.id,
                        success: true,
                        result: serde_json::from_str("\"stopped early\"").unwrap(),
                        stop_early_override: Some(true),
                        w_id: flow_job.workspace_id.clone(),
                        worker_dir: worker_dir.to_string(),
                        token: client.token.clone(),
                    })
                    .await
                    .map_err(|e| {
                        Error::InternalErr(format!(
                            "error sending update flow message to job completed channel: {e}"
                        ))
                    })?;

                return Ok(());
            }
        }
    }

    // Compute and initialize last_job_result
    let arc_last_job_result = if status_module.is_failure() {
        // if job is being retried, pass the result of its previous failure
        Arc::new(last_job_result.unwrap_or(to_raw_value(&json!("{}"))))
    } else if i == 0 {
        // if it's the first job executed in the flow, pass the flow args
        Arc::new(to_raw_value(&flow_job.args))
    } else {
        // else pass the last job result. Either from the function arg if it's set, or manually fetch it from the previous job
        // having last_job_result empty can happen either when the job was suspended and is being restarted, or if it's a
        // flow restart from a specific step
        if last_job_result.is_some() {
            Arc::new(last_job_result.unwrap())
        } else {
            match get_previous_job_result(db, flow_job.workspace_id.as_str(), &status).await? {
                None => Arc::new(to_raw_value(&json!("{}"))),
                Some(previous_job_result) => Arc::new(previous_job_result),
            }
        }
    };

    let mut resume_messages: Vec<Box<RawValue>> = vec![];
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

            let resumes = sqlx::query(
                "SELECT value, approver, resume_id, approved FROM resume_job WHERE job = $1 ORDER BY created_at ASC",
            )
            .bind(last)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|x| ResumeRow::from_row(&x))
            .collect::<Vec<_>>();

            resume_messages.extend(
                resumes
                    .iter()
                    .map(|r| to_raw_value(&r.as_ref().map(|x| x.value.clone()).ok())),
            );
            approvers.extend(resumes.iter().map(|r| {
                r.as_ref()
                    .ok()
                    .and_then(|x| x.approver.clone())
                    .as_deref()
                    .unwrap_or_else(|| "anonymous")
                    .to_string()
            }));

            // Persist approval user groups conditions, if any. Requires runnning the InputTransform
            let required_events = suspend.required_events.unwrap() as u16;
            let user_auth_required = suspend.user_auth_required.unwrap_or(false);
            if user_auth_required {
                let self_approval_disabled = suspend.self_approval_disabled.unwrap_or(false);
                let mut user_groups_required: Vec<String> = Vec::new();
                if suspend.user_groups_required.is_some() {
                    match suspend.user_groups_required.unwrap() {
                        InputTransform::Static { value } => {
                            user_groups_required = serde_json::from_str::<Vec<String>>(value.get())
                                .expect("Unable to deserialize group names");
                        }
                        InputTransform::Javascript { expr } => {
                            let mut context = HashMap::with_capacity(2);
                            context.insert("result".to_string(), arc_last_job_result.clone());
                            context
                                .insert("previous_result".to_string(), arc_last_job_result.clone());

                            let eval_result = serde_json::from_str::<Vec<String>>(
                                eval_timeout(
                                    expr.to_string(),
                                    context,
                                    Some(arc_flow_job_args.clone()),
                                    None,
                                    None,
                                )
                                .await
                                .map_err(|e| {
                                    Error::ExecutionErr(format!(
                                        "Error during isolated evaluation of expression `{expr}`:\n{e}"
                                    ))
                                })?
                                .get(),
                            );
                            if eval_result.is_ok() {
                                user_groups_required = eval_result.ok().unwrap_or(Vec::new())
                            } else {
                                let e = eval_result.err().unwrap();
                                return Err(Error::ExecutionErr(format!(
                                    "Result returned by input transform invalid `{e}`"
                                )));
                            }
                        }
                    }
                }
                let approval_conditions = ApprovalConditions {
                    user_auth_required: user_auth_required,
                    user_groups_required: user_groups_required,
                    self_approval_disabled: self_approval_disabled,
                };
                sqlx::query(
                    "UPDATE queue
                    SET flow_status = JSONB_SET(flow_status, ARRAY['approval_conditions'], $1)
                    WHERE id = $2",
                )
                .bind(json!(approval_conditions))
                .bind(flow_job.id)
                .execute(&mut *tx)
                .await?;
            }

            let is_disapproved = resumes
                .iter()
                .find(|x| x.as_ref().is_ok_and(|x| !x.approved));
            if is_disapproved.is_none() && resume_messages.len() >= required_events as usize {
                sqlx::query(
                    "UPDATE queue
                    SET flow_status = JSONB_SET(flow_status, ARRAY['modules', $1::TEXT, 'approvers'], $2)
                    WHERE id = $3",
                )
                .bind(status.step - 1)
                .bind(json!(resumes
                    .into_iter()
                    .map(|r| Approval {
                        resume_id: r.as_ref().map(|x| x.resume_id).unwrap_or_default() as u16,
                        approver: r
                            .as_ref()
                            .ok()
                            .and_then(|x| x.approver.clone())
                            .unwrap_or_else(|| "unknown".to_string())
                    })
                    .collect::<Vec<_>>()))
                .bind(flow_job.id)
                .execute(&mut *tx)
                .await?;

                // Remove the approval conditions from the flow status
                sqlx::query(
                    "UPDATE queue
                    SET flow_status = flow_status - 'approval_conditions'
                    WHERE id = $1",
                )
                .bind(flow_job.id)
                .execute(&mut *tx)
                .await?;

                /* continue on and run this job! */
                tx.commit().await?;

            /* not enough messages to do this job, "park"/suspend until there are */
            } else if matches!(
                &status_module,
                FlowStatusModule::WaitingForPriorSteps { .. }
            ) && is_disapproved.is_none()
            {
                sqlx::query(
                    "UPDATE queue SET 
                        flow_status = JSONB_SET(flow_status, ARRAY['modules', flow_status->>'step'::text], $1), 
                        suspend = $2,
                        suspend_until = now() + $3
                    WHERE id = $4",
                )
                .bind(json!(FlowStatusModule::WaitingForEvents { id: status_module.id(), count: required_events, job: last }))
                .bind((required_events - resume_messages.len() as u16) as i32)
                .bind(Duration::from_secs(suspend.timeout.map(|t| t.into()).unwrap_or_else(|| 30 * 60)))
                .bind(flow_job.id)
                .execute(&mut *tx)
                .await?;

                sqlx::query!(
                    "UPDATE queue
                    SET last_ping = null
                    WHERE id = $1 AND last_ping = $2",
                    flow_job.id,
                    flow_job.last_ping
                )
                .execute(&mut *tx)
                .await?;

                tx.commit().await?;
                return Ok(());

            /* cancelled or we're WaitingForEvents but we don't have enough messages (timed out) */
            } else {
                tx.commit().await?;

                let (logs, error_name) = if let Some(disapprover) = is_disapproved {
                    (
                        format!(
                            "Disapproved by {:?}",
                            disapprover.as_ref().unwrap().approver
                        ),
                        "SuspendedDisapproved",
                    )
                } else {
                    (
                        "Timed out waiting to be resumed".to_string(),
                        "SuspendedTimedOut",
                    )
                };

                let result: Value = json!({ "error": {"message": logs, "name": error_name}});

                append_logs(flow_job.id, flow_job.workspace_id.clone(), logs.clone(), db).await;

                job_completed_tx
                    .send(SendResult::UpdateFlow {
                        flow: flow_job.id,
                        success: false,
                        result: to_raw_value(&result),
                        stop_early_override: None,
                        w_id: flow_job.workspace_id.clone(),
                        worker_dir: worker_dir.to_string(),
                        token: client.token.clone(),
                    })
                    .await
                    .map_err(|e| {
                        Error::InternalErr(format!(
                            "error sending update flow message to job completed channel: {e}"
                        ))
                    })?;

                return Ok(());
            }
        }
    }

    let mut module: &FlowModule = flow
        .modules
        .get(i)
        .or_else(|| flow.failure_module.as_deref())
        .with_context(|| format!("no module at index {}", status.step))?;

    let current_id = &module.id;
    let previous_id = if i >= 1 {
        flow.modules.get(i - 1).map(|m| m.id.clone()).unwrap()
    } else {
        String::new()
    };

    // calculate sleep if any
    let mut scheduled_for_o = {
        // avoid branchall sleeping on every iteration if sleep is on module prior
        if !matches!(
            &status_module,
            FlowStatusModule::WaitingForPriorSteps { .. }
                | FlowStatusModule::WaitingForEvents { .. }
        ) {
            None
        } else {
            let sleep_input_transform = i
                .checked_sub(1)
                .and_then(|i| flow.modules.get(i))
                .and_then(|m| m.sleep.clone());

            if let Some(it) = sleep_input_transform {
                let json_value = match it {
                    InputTransform::Static { value } => Ok(value),
                    InputTransform::Javascript { expr } => {
                        let mut context = HashMap::with_capacity(2);
                        context.insert("result".to_string(), arc_last_job_result.clone());
                        context.insert("previous_result".to_string(), arc_last_job_result.clone());

                        serde_json::from_str(
                            eval_timeout(
                                expr.to_string(),
                                context,
                                Some(arc_flow_job_args.clone()),
                                None,
                                None,
                            )
                            .await
                            .map_err(|e| {
                                Error::ExecutionErr(format!(
                                    "Error during isolated evaluation of expression `{expr}`:\n{e}"
                                ))
                            })?
                            .get(),
                        )
                    }
                };
                match json_value.and_then(|x| serde_json::from_str::<serde_json::Value>(x.get())) {
                    Ok(serde_json::Value::Number(n)) => {
                        if n.is_f64() {
                            n.as_f64()
                                .map(|x: f64| from_now(Duration::from_millis((x * 1000.0) as u64)))
                        } else if n.is_u64() {
                            n.as_u64().map(|x: u64| from_now(Duration::from_secs(x)))
                        } else {
                            return Err(Error::ExecutionErr(format!(
                                "Expected an integer, found: {n}"
                            )));
                        }
                    }
                    Ok(x @ _) => Err(Error::ExecutionErr(format!(
                        "Expected an integer, found: {x:?}"
                    )))?,
                    Err(e) => Err(Error::ExecutionErr(format!(
                        "Expected a number value, had error instead: {e:?}",
                    )))?,
                }
            } else {
                None
            }
        }
    };

    let retry = if matches!(&status_module, FlowStatusModule::Failure { .. },) {
        let retry = &module.retry.clone().unwrap_or_default();
        next_retry(retry, &status.retry)
    } else {
        None
    };
    let get_args_from_id = match &status_module {
        FlowStatusModule::Failure { job, .. }
            if retry.as_ref().is_some() || !module.continue_on_error.is_some_and(|x| x) =>
        {
            if let Some((fail_count, retry_in)) = retry {
                tracing::debug!(
                    retry_in_seconds = retry_in.as_secs(),
                    fail_count = fail_count,
                    "retrying"
                );

                scheduled_for_o = Some(from_now(retry_in));
                status.retry.failed_jobs.push(job.clone());
                sqlx::query(
                    "UPDATE queue
                    SET flow_status = JSONB_SET(JSONB_SET(flow_status, ARRAY['retry'], $1), ARRAY['modules', $3::TEXT, 'failed_retries'], $4)
                    WHERE id = $2",
                )
                .bind(json!(RetryStatus { fail_count, ..status.retry.clone() }))
                .bind(flow_job.id)
                .bind(status.step)
                .bind(json!(status.retry.failed_jobs))
                .execute(db)
                .await
                .context("update flow retry")?;

                status_module = FlowStatusModule::WaitingForPriorSteps { id: status_module.id() };
                // we get the args from the last failed job
                status.retry.failed_jobs.last()
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

                if module.retry.as_ref().is_some_and(|x| x.has_attempts()) {
                    sqlx::query(
                        "UPDATE queue
                        SET flow_status = JSONB_SET(flow_status, ARRAY['retry'], $1)
                        WHERE id = $2",
                    )
                    .bind(json!(RetryStatus { fail_count: 0, failed_jobs: vec![] }))
                    .bind(flow_job.id)
                    .execute(db)
                    .await
                    .context("update flow retry")?;
                };
                None
            }
        }
        _ => None,
    };

    let mut transform_context: Option<IdContext> = None;

    let approvers = Arc::new(to_raw_value(&approvers));
    let resume = Arc::new(to_raw_value(&resume_messages.last()));
    let resumes = Arc::new(to_raw_value(&resume_messages));

    drop(resume_messages);

    let args: windmill_common::error::Result<_> = if module.mock.is_some()
        && module.mock.as_ref().unwrap().enabled
    {
        let mut hm = HashMap::new();
        hm.insert(
            "previous_result".to_string(),
            to_raw_value(
                &module
                    .mock
                    .as_ref()
                    .unwrap()
                    .return_value
                    .clone()
                    .unwrap_or_else(|| serde_json::from_str("null").unwrap()),
            ),
        );
        Ok(hm)
    } else if let Some(id) = get_args_from_id {
        let row = sqlx::query("SELECT args FROM completed_job WHERE id = $1 AND workspace_id = $2")
            .bind(id)
            .bind(&flow_job.workspace_id)
            .fetch_optional(db)
            .await?;
        if let Some(row) = row {
            RawArgs::from_row(&row)
                .map(|x| x.args.map(|x| x.0).unwrap_or_else(HashMap::new))
                .map_err(|e| error::Error::InternalErr(format!("Impossible to build args: {e}")))
        } else {
            Ok(HashMap::new())
        }
    } else {
        match &module.get_value() {
            Ok(
                FlowModuleValue::Script { input_transforms, .. }
                | FlowModuleValue::RawScript { input_transforms, .. }
                | FlowModuleValue::Flow { input_transforms, .. },
            ) => {
                let ctx = get_transform_context(&flow_job, &previous_id, &status).await?;
                transform_context = Some(ctx);
                let by_id = transform_context.as_ref().unwrap();
                transform_input(
                    arc_flow_job_args.clone(),
                    arc_last_job_result.clone(),
                    input_transforms,
                    resumes.clone(),
                    resume.clone(),
                    approvers.clone(),
                    by_id,
                    client,
                )
                .await
            }
            Ok(FlowModuleValue::Identity) => serde_json::from_str(
                &serde_json::to_string(&PreviousResult {
                    previous_result: Some(&arc_last_job_result),
                })
                .unwrap(),
            )
            .map_err(|e| error::Error::InternalErr(format!("identity: {e}"))),

            Ok(_) => Ok(flow_job_args),
            Err(e) => {
                return Err(error::Error::InternalErr(format!(
                    "module was not convertible to acceptable value {e:?}"
                )))
            }
        }
    };
    tracing::debug!(id = %flow_job.id, root_id = %job_root, "flow job args computed");

    let next_flow_transform = compute_next_flow_transform(
        arc_flow_job_args.clone(),
        arc_last_job_result.clone(),
        flow_job,
        &flow,
        transform_context,
        db,
        &module,
        &status,
        &status_module,
        &previous_id,
        client,
        resumes.clone(),
        resume.clone(),
        approvers.clone(),
    )
    .await?;
    tracing::info!(id = %flow_job.id, root_id = %job_root, "next flow transform computed");

    let (job_payloads, next_status) = match next_flow_transform {
        NextFlowTransform::Continue(job_payload, next_state) => (job_payload, next_state),
        NextFlowTransform::EmptyInnerFlows => {
            sqlx::query(
                "UPDATE queue
                SET flow_status = JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2)
                WHERE id = $3",
            )
            .bind(status.step)
            .bind(json!(FlowStatusModule::Success {
                id: status_module.id(),
                job: Uuid::nil(),
                flow_jobs: Some(vec![]),
                branch_chosen: None,
                approvers: vec![],
                failed_retries: vec![],
            }))
            .bind(flow_job.id)
            .execute(db)
            .await?;
            // flow is reprocessed by the worker in a state where the module has completed successfully.
            // The next steps are pull -> handle flow -> push next flow job -> update flow status since module status is success
            same_worker_tx
                .send(SameWorkerPayload { job_id: flow_job.id, recoverable: true })
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

        // compute job-to-be-pushed priority
        // The job definition itself might have its own priority, but as we're running
        // it from a flow here, it inherits first the flow step priority and second the
        // flow priority.
        let new_job_priority_override = if module.priority.is_some() {
            module.priority
        } else if flow_job.priority.is_some() {
            flow_job.priority
        } else {
            None
        };

        let transform_inp;
        let args = match &next_status {
            NextStatus::AllFlowJobs {
                branchall: Some(BranchAllStatus { .. }),
                iterator: None,
                ..
            } => args.as_ref().map(|args| args.clone()),
            NextStatus::NextLoopIteration {
                next: ForloopNextIteration { new_args, .. },
                simple_input_transforms,
            } => {
                let mut args = if let Ok(args) = args.as_ref() {
                    args.clone()
                } else {
                    HashMap::new()
                };
                insert_iter_arg(&mut args, "iter".to_string(), to_raw_value(new_args));

                args.insert("iter".to_string(), to_raw_value(new_args));
                if let Some(input_transforms) = simple_input_transforms {
                    let ctx = get_transform_context(&flow_job, &previous_id, &status).await?;
                    transform_inp = transform_input(
                        Arc::new(args),
                        arc_last_job_result.clone(),
                        input_transforms,
                        resumes.clone(),
                        resume.clone(),
                        approvers.clone(),
                        &ctx,
                        client,
                    )
                    .await;
                    transform_inp.as_ref().map(|args| args.clone())
                } else {
                    Ok(args)
                }
            }
            NextStatus::AllFlowJobs {
                branchall: None,
                iterator: Some(Iterator { itered, .. }),
                simple_input_transforms,
            } => {
                if let Ok(args) = args.as_ref() {
                    let mut hm = HashMap::new();
                    for (k, v) in args {
                        hm.insert(k.to_string(), v.to_owned());
                    }
                    insert_iter_arg(
                        &mut hm,
                        "iter".to_string(),
                        to_raw_value(&json!({ "index": i as i32, "value": itered[i]})),
                    );
                    if let Some(input_transforms) = simple_input_transforms {
                        let ctx = get_transform_context(&flow_job, &previous_id, &status).await?;
                        transform_inp = transform_input(
                            Arc::new(hm),
                            arc_last_job_result.clone(),
                            input_transforms,
                            resumes.clone(),
                            resume.clone(),
                            approvers.clone(),
                            &ctx,
                            client,
                        )
                        .await;
                        transform_inp.as_ref().map(|args| args.clone())
                    } else {
                        Ok(hm)
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

        let value_with_parallel = module.get_value_with_parallel()?;

        let root_job = if {
            value_with_parallel.type_ == "flow"
                || (value_with_parallel.type_ == "forloopflow"
                    && value_with_parallel.parallel.is_some_and(|x| x))
        } {
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
            windmill_queue::PushArgs {
                args: ok.unwrap_or_else(|| serde_json::from_str("{}").unwrap()),
                extra: HashMap::new(),
            },
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
            payload_tag.timeout,
            Some(module.id.clone()),
            new_job_priority_override,
        )
        .await?;

        tracing::debug!(id = %flow_job.id, root_id = %job_root, "pushed next flow job: {uuid}");

        if value_with_parallel.type_ == "forloopflow" {
            if let Some(p) = value_with_parallel.parallelism {
                if i as u16 >= p {
                    sqlx::query!(
                        "UPDATE queue
                    SET suspend = $1, suspend_until = now() + interval '14 day', running = true
                    WHERE id = $2",
                        (i as u16 - p + 1) as i32,
                        uuid,
                    )
                    .execute(&mut inner_tx)
                    .await?;
                }
            }
        }

        if payload_tag.delete_after_use {
            let uuid_singleton_json = serde_json::to_value(&[uuid])
                .map_err(|e| error::Error::InternalErr(format!("Unable to serialize uuid: {e}")))?;

            sqlx::query(
                "UPDATE queue
                SET flow_status = JSONB_SET(flow_status, ARRAY['cleanup_module', 'flow_jobs_to_clean'], COALESCE(flow_status->'cleanup_module'->'flow_jobs_to_clean', '[]'::jsonb) || $1)
                WHERE id = $2",
            )
            .bind(uuid_singleton_json)
            .bind(root_job.unwrap_or(flow_job.id))
            .execute(&mut inner_tx)
            .await?;
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

    if !is_one_uuid {
        for uuid in &uuids {
            sqlx::query!(
                "INSERT INTO parallel_monitor_lock (parent_flow_id, job_id)
                VALUES ($1, $2)",
                flow_job.id,
                uuid
            )
            .execute(&mut tx)
            .await?;
        }
    }
    let first_uuid = uuids[0];
    let new_status = match next_status {
        NextStatus::NextLoopIteration {
            next: ForloopNextIteration { index, itered, mut flow_jobs, while_loop, .. },
            ..
        } => {
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
                while_loop,
            }
        }
        NextStatus::AllFlowJobs { iterator, branchall, .. } => FlowStatusModule::InProgress {
            job: flow_job.id,
            iterator,
            flow_jobs: Some(uuids.clone()),
            branch_chosen: None,
            branchall,
            id: status_module.id(),
            parallel: true,
            while_loop: false,
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
                while_loop: false,
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
            while_loop: false,
        },
        NextStatus::NextStep => {
            FlowStatusModule::WaitingForExecutor { id: status_module.id(), job: one_uuid? }
        }
    };

    tracing::debug!("STATUS STEP: {:?} {i} {:#?}", status.step, new_status);

    if i >= flow.modules.len() {
        sqlx::query!(
            "UPDATE queue
            SET flow_status = JSONB_SET(
                JSONB_SET(flow_status, ARRAY['failure_module'], $1), ARRAY['step'], $2)
            WHERE id = $3",
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
            "UPDATE queue
            SET flow_status = JSONB_SET(
                JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2), ARRAY['step'], $3)
            WHERE id = $4",
            i as i32,
            json!(new_status),
            json!(i),
            flow_job.id
        )
        .execute(&mut tx)
        .await?;
    };

    potentially_crash_for_testing();

    sqlx::query!(
        "UPDATE queue
        SET last_ping = null
        WHERE id = $1",
        flow_job.id
    )
    .execute(&mut tx)
    .await?;

    tx.commit().await?;
    tracing::info!(id = %flow_job.id, root_id = %job_root, "all next flow jobs pushed: {uuids:?}");

    if continue_on_same_worker {
        if !is_one_uuid {
            return Err(Error::BadRequest(
                "Cannot continue on same worker with multiple jobs, parallel cannot be used in conjunction with same_worker".to_string(),
            ));
        }
        same_worker_tx
            .send(SameWorkerPayload { job_id: first_uuid, recoverable: true })
            .await
            .map_err(to_anyhow)?;
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
struct ForloopNextIteration {
    index: usize,
    itered: Vec<Box<RawValue>>,
    flow_jobs: Vec<Uuid>,
    new_args: Iter,
    while_loop: bool,
}

enum ForLoopStatus {
    ParallelIteration { itered: Vec<Box<RawValue>> },
    NextIteration(ForloopNextIteration),
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
    NextLoopIteration {
        next: ForloopNextIteration,
        simple_input_transforms: Option<HashMap<String, InputTransform>>,
    },
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
    delete_after_use: bool,
    timeout: Option<i32>,
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

fn insert_iter_arg(
    args: &mut HashMap<String, Box<RawValue>>,
    desired_key: String,
    value: Box<RawValue>,
) {
    let conflicting_parent_maybe = args.get(desired_key.as_str());
    if let Some(conflicting_parent) = conflicting_parent_maybe {
        // we change the key of the parent to {desired_key}_parent.
        // we do it recursively b/c it's possible there's another conflict
        let conflicting_parent_new_key = format!("{}_parent", desired_key.as_str());
        insert_iter_arg(
            args,
            conflicting_parent_new_key,
            conflicting_parent.to_owned(),
        );
    }
    args.insert(desired_key, value);
}

async fn compute_next_flow_transform(
    arc_flow_job_args: Arc<HashMap<String, Box<RawValue>>>,
    arc_last_job_result: Arc<Box<RawValue>>,
    flow_job: &QueuedJob,
    flow: &FlowValue,
    by_id: Option<IdContext>,
    db: &DB,
    module: &FlowModule,
    status: &FlowStatus,
    status_module: &FlowStatusModule,
    previous_id: &str,
    client: &AuthedClient,
    resumes: Arc<Box<RawValue>>,
    resume: Arc<Box<RawValue>>,
    approvers: Arc<Box<RawValue>>,
) -> error::Result<NextFlowTransform> {
    if module.mock.is_some() && module.mock.as_ref().unwrap().enabled {
        return Ok(NextFlowTransform::Continue(
            ContinuePayload::SingleJob(JobPayloadWithTag {
                payload: JobPayload::Identity,
                tag: None,
                delete_after_use: false,
                timeout: None,
            }),
            NextStatus::NextStep,
        ));
    }
    let trivial_next_job = |payload| {
        Ok(NextFlowTransform::Continue(
            ContinuePayload::SingleJob(JobPayloadWithTag {
                payload,
                tag: None,
                delete_after_use: false,
                timeout: None,
            }),
            NextStatus::NextStep,
        ))
    };
    let delete_after_use = module.delete_after_use.unwrap_or(false);

    tracing::debug!(id = %flow_job.id, "computing next flow transform for {:?}", &module.value);
    match &module.get_value()? {
        FlowModuleValue::Identity => trivial_next_job(JobPayload::Identity),
        FlowModuleValue::Flow { path, .. } => {
            let payload = flow_to_payload(path, &delete_after_use);
            Ok(NextFlowTransform::Continue(
                ContinuePayload::SingleJob(payload),
                NextStatus::NextStep,
            ))
        }
        FlowModuleValue::Script { path: script_path, hash: script_hash, tag_override, .. } => {
            let payload =
                script_to_payload(script_hash, script_path, db, flow_job, module, tag_override)
                    .await?;
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
            custom_concurrency_key,
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
                custom_concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                module,
                tag,
                &delete_after_use,
            );
            Ok(NextFlowTransform::Continue(
                ContinuePayload::SingleJob(payload),
                NextStatus::NextStep,
            ))
        }
        FlowModuleValue::WhileloopFlow { modules, .. } => {
            // if it's a simple single step flow, we will collapse it as an optimization and need to pass flow_input as an arg
            let is_simple = is_simple_modules(modules, flow);
            let flow_jobs = match status_module {
                FlowStatusModule::InProgress { flow_jobs: Some(flow_jobs), .. } => {
                    flow_jobs.clone()
                }
                _ => vec![],
            };
            let next_loop_idx = flow_jobs.len();
            next_loop_iteration(
                flow,
                status,
                ForloopNextIteration {
                    index: next_loop_idx,
                    itered: vec![],
                    flow_jobs: flow_jobs.clone(),
                    new_args: Iter {
                        index: next_loop_idx as i32,
                        value: windmill_common::worker::to_raw_value(&next_loop_idx),
                    },
                    while_loop: true,
                },
                modules,
                flow_job,
                is_simple,
                db,
                module,
                delete_after_use,
            )
            .await
        }
        /* forloop modules are expected set `iter: { value: Value, index: usize }` as job arguments */
        FlowModuleValue::ForloopFlow { modules, iterator, parallel, .. } => {
            // if it's a simple single step flow, we will collapse it as an optimization and need to pass flow_input as an arg
            let is_simple = is_simple_modules(modules, flow);

            let next_loop_status = next_forloop_status(
                status_module,
                by_id,
                flow_job,
                previous_id,
                status,
                iterator,
                arc_last_job_result,
                resumes,
                resume,
                approvers,
                arc_flow_job_args,
                client,
                parallel,
            )
            .await?;

            match next_loop_status {
                ForLoopStatus::EmptyIterator => Ok(NextFlowTransform::EmptyInnerFlows),
                ForLoopStatus::NextIteration(ns) => {
                    next_loop_iteration(
                        flow,
                        status,
                        ns,
                        modules,
                        flow_job,
                        is_simple,
                        db,
                        module,
                        delete_after_use,
                    )
                    .await
                }
                ForLoopStatus::ParallelIteration { itered, .. } => {
                    if modules.is_empty() {
                        return Ok(NextFlowTransform::EmptyInnerFlows);
                    } else {
                        let inner_path =
                            Some(format!("{}/loop-parrallel", flow_job.script_path(),));
                        let value = &modules[0].get_value()?;
                        let continue_payload = if is_simple {
                            let payload =
                                payload_from_simple_module(value, db, flow_job, module, inner_path)
                                    .await?;
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
                                            priority: None,
                                            early_return: None,
                                            concurrency_key: None,
                                        },
                                        path: Some(format!("{}/forloop", flow_job.script_path())),
                                        restarted_from: None,
                                    },
                                    tag: None,
                                    delete_after_use: delete_after_use,
                                    timeout: None,
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
                                    match value {
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
                            arc_flow_job_args.clone(),
                            arc_last_job_result.clone(),
                            Some(idcontext.clone()),
                            Some(client),
                            Some((resumes.clone(), resume.clone(), approvers.clone())),
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

            let mut modules = if let BranchChosen::Branch { branch } = branch {
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
            add_virtual_items_if_necessary(&mut modules);

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
                            priority: None,
                            early_return: None,
                            concurrency_key: None,
                        },
                        path: Some(format!(
                            "{}/branchone-{}",
                            flow_job.script_path(),
                            status.step
                        )),
                        restarted_from: None,
                    },
                    tag: None,
                    delete_after_use: delete_after_use,
                    timeout: None,
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
                                        let mut modules = b.modules.clone();
                                        add_virtual_items_if_necessary(&mut modules);
                                        JobPayloadWithTag {
                                            payload: JobPayload::RawFlow {
                                                value: FlowValue {
                                                    modules,
                                                    failure_module: fm.clone(),
                                                    same_worker: flow.same_worker,
                                                    concurrent_limit: None,
                                                    concurrency_time_window_s: None,
                                                    skip_expr: None,
                                                    cache_ttl: None,
                                                    priority: None,
                                                    early_return: None,
                                                    concurrency_key: None,
                                                },
                                                path: Some(format!(
                                                    "{}/branchall-{}",
                                                    flow_job.script_path(),
                                                    i
                                                )),
                                                restarted_from: None,
                                            },
                                            tag: None,
                                            delete_after_use: delete_after_use,
                                            timeout: None,
                                        }
                                    })
                                    .collect(),
                            ),
                            NextStatus::AllFlowJobs {
                                branchall: Some(BranchAllStatus { branch: 0, len: branches.len() }),
                                iterator: None,
                                simple_input_transforms: None,
                            },
                        ));
                    } else {
                        (BranchAllStatus { branch: 0, len: branches.len() }, vec![])
                    }
                }
                FlowStatusModule::InProgress {
                    branchall: Some(BranchAllStatus { branch, len }),
                    flow_jobs: Some(flow_jobs),
                    ..
                } if !*parallel => (
                    BranchAllStatus { branch: branch + 1, len: len.clone() },
                    flow_jobs.clone(),
                ),

                _ => Err(Error::BadRequest(format!(
                    "Unrecognized module status for BranchAll {status_module:?}"
                )))?,
            };

            let mut modules = branches
                .get(branch_status.branch)
                .map(|b| b.modules.clone())
                .ok_or_else(|| {
                    Error::BadRequest(format!(
                        "Unrecognized branch for BranchAll {status_module:?}"
                    ))
                })?;

            add_virtual_items_if_necessary(&mut modules);
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
                            priority: None,
                            early_return: None,
                            concurrency_key: None,
                        },
                        path: Some(format!(
                            "{}/branchall-{}",
                            flow_job.script_path(),
                            branch_status.branch
                        )),
                        restarted_from: None,
                    },
                    tag: None,
                    delete_after_use: delete_after_use,
                    timeout: None,
                }),
                NextStatus::NextBranchStep(NextBranch { status: branch_status, flow_jobs }),
            ))
        }
    }
}

async fn next_loop_iteration(
    flow: &FlowValue,
    status: &FlowStatus,
    ns: ForloopNextIteration,
    modules: &Vec<FlowModule>,
    flow_job: &QueuedJob,
    is_simple: bool,
    db: &sqlx::Pool<sqlx::Postgres>,
    module: &FlowModule,
    delete_after_use: bool,
) -> Result<NextFlowTransform, Error> {
    let mut fm = flow.failure_module.clone();
    if let Some(mut failure_module) = flow.failure_module.clone() {
        failure_module.id_append(&format!("{}/{}", status.step, ns.index));
        fm = Some(failure_module);
    }
    let mut modules = (*modules).clone();
    add_virtual_items_if_necessary(&mut modules);
    let inner_path = Some(format!("{}/loop-{}", flow_job.script_path(), ns.index));
    if is_simple {
        let value = &modules[0].get_value()?;
        let payload = payload_from_simple_module(value, db, flow_job, module, inner_path).await?;
        Ok(NextFlowTransform::Continue(
            ContinuePayload::SingleJob(payload),
            NextStatus::NextLoopIteration {
                next: ns,
                simple_input_transforms: if is_simple {
                    match value {
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
    } else {
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
                        priority: None,
                        early_return: None,
                        concurrency_key: None,
                    },
                    path: inner_path,
                    restarted_from: None,
                },
                tag: None,
                delete_after_use,
                timeout: None,
            }),
            NextStatus::NextLoopIteration { next: ns, simple_input_transforms: None },
        ))
    }
}

fn is_simple_modules(modules: &Vec<FlowModule>, flow: &FlowValue) -> bool {
    let is_simple = modules.len() == 1
        && modules[0].is_simple()
        && modules[0].sleep.is_none()
        && modules[0].suspend.is_none()
        && modules[0].cache_ttl.is_none()
        && modules[0].stop_after_if.is_none()
        && (modules[0].mock.is_none() || modules[0].mock.as_ref().is_some_and(|m| !m.enabled))
        && flow.failure_module.is_none();
    is_simple
}

async fn next_forloop_status(
    status_module: &FlowStatusModule,
    by_id: Option<IdContext>,
    flow_job: &QueuedJob,
    previous_id: &str,
    status: &FlowStatus,
    iterator: &InputTransform,
    arc_last_job_result: Arc<Box<RawValue>>,
    resumes: Arc<Box<RawValue>>,
    resume: Arc<Box<RawValue>>,
    approvers: Arc<Box<RawValue>>,
    arc_flow_job_args: Arc<HashMap<String, Box<RawValue>>>,
    client: &AuthedClient,
    parallel: &bool,
) -> Result<ForLoopStatus, Error> {
    let next_loop_status = match status_module {
        FlowStatusModule::WaitingForPriorSteps { .. }
        | FlowStatusModule::WaitingForEvents { .. }
        | FlowStatusModule::WaitingForExecutor { .. } => {
            let by_id = if let Some(x) = by_id {
                x
            } else {
                get_transform_context(&flow_job, previous_id, &status).await?
            };
            /* Iterator is an InputTransform, evaluate it into an array. */
            let itered_raw = match iterator {
                InputTransform::Static { value } => to_raw_value(value),
                InputTransform::Javascript { expr } => {
                    let mut context = HashMap::with_capacity(5);
                    context.insert("result".to_string(), arc_last_job_result.clone());
                    context.insert("previous_result".to_string(), arc_last_job_result);
                    context.insert("resumes".to_string(), resumes);
                    context.insert("resume".to_string(), resume);
                    context.insert("approvers".to_string(), approvers);

                    eval_timeout(
                        expr.to_string(),
                        context,
                        Some(arc_flow_job_args),
                        Some(client),
                        Some(by_id),
                    )
                    .await?
                }
            };
            let itered = serde_json::from_str::<Vec<Box<RawValue>>>(itered_raw.get()).map_err(
                |not_array| {
                    Error::ExecutionErr(format!(
                        "Expected an array value in the iterator expression, found: {not_array}"
                    ))
                },
            )?;

            if itered.is_empty() {
                ForLoopStatus::EmptyIterator
            } else if *parallel {
                ForLoopStatus::ParallelIteration { itered }
            } else if let Some(first) = itered.first() {
                let iter = Iter { index: 0 as i32, value: first.to_owned() };
                ForLoopStatus::NextIteration(ForloopNextIteration {
                    index: 0,
                    itered,
                    flow_jobs: vec![],
                    new_args: iter,
                    while_loop: false,
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
            let itered_new = if itered.is_empty() {
                // it's possible we need to re-compute the iterator Input Transforms here, in particular if the flow is being restarted inside the loop
                let by_id = if let Some(x) = by_id {
                    x
                } else {
                    get_transform_context(&flow_job, previous_id, &status).await?
                };
                let itered_raw = match iterator {
                    InputTransform::Static { value } => to_raw_value(value),
                    InputTransform::Javascript { expr } => {
                        let mut context = HashMap::with_capacity(5);
                        context.insert("result".to_string(), arc_last_job_result.clone());
                        context.insert("previous_result".to_string(), arc_last_job_result);
                        context.insert("resumes".to_string(), resumes);
                        context.insert("resume".to_string(), resume);
                        context.insert("approvers".to_string(), approvers);

                        eval_timeout(
                            expr.to_string(),
                            context,
                            Some(arc_flow_job_args),
                            Some(client),
                            Some(by_id),
                        )
                        .await?
                    }
                };
                serde_json::from_str::<Vec<Box<RawValue>>>(itered_raw.get()).map_err(
                    |not_array| {
                        Error::ExecutionErr(format!("Expected an array value, found: {not_array}"))
                    },
                )?
            } else {
                itered.clone()
            };
            let (index, next) = index
                .checked_add(1)
                .and_then(|i| itered_new.get(i).map(|next| (i, next)))
                .with_context(|| {
                    format!("Could not find iteration number {index} restarting inside the for-loop flow. It's possible the itered-array has changed and this value isn't available anymore.")
                })?;

            ForLoopStatus::NextIteration(ForloopNextIteration {
                index,
                itered: itered_new.clone(),
                flow_jobs: flow_jobs.clone(),
                new_args: Iter { index: index as i32, value: next.to_owned() },
                while_loop: false,
            })
        }

        _ => Err(Error::BadRequest(format!(
            "Unrecognized module status for ForloopFlow {status_module:?}"
        )))?,
    };
    Ok(next_loop_status)
}

async fn payload_from_simple_module(
    value: &FlowModuleValue,
    db: &sqlx::Pool<sqlx::Postgres>,
    flow_job: &QueuedJob,
    module: &FlowModule,
    inner_path: Option<String>,
) -> Result<JobPayloadWithTag, Error> {
    let delete_after_use = module.delete_after_use.unwrap_or(false);
    Ok(match value {
        FlowModuleValue::Flow { path, .. } => flow_to_payload(path, &delete_after_use),
        FlowModuleValue::Script { path: script_path, hash: script_hash, tag_override, .. } => {
            script_to_payload(script_hash, script_path, db, flow_job, module, tag_override).await?
        }
        FlowModuleValue::RawScript {
            path,
            content,
            language,
            lock,
            tag,
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            ..
        } => raw_script_to_payload(
            path.clone().or(inner_path),
            content,
            language,
            lock,
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            module,
            tag,
            &delete_after_use,
        ),
        _ => unreachable!("is simple flow"),
    })
}

fn raw_script_to_payload(
    path: Option<String>,
    content: &String,
    language: &windmill_common::scripts::ScriptLang,
    lock: &Option<String>,
    custom_concurrency_key: &Option<String>,
    concurrent_limit: &Option<i32>,
    concurrency_time_window_s: &Option<i32>,
    module: &FlowModule,
    tag: &Option<String>,
    delete_after_use: &bool,
) -> JobPayloadWithTag {
    JobPayloadWithTag {
        payload: JobPayload::Code(RawCode {
            hash: None,
            path,
            content: content.clone(),
            language: language.clone(),
            lock: lock.clone(),
            custom_concurrency_key: custom_concurrency_key.clone(),
            concurrent_limit: *concurrent_limit,
            concurrency_time_window_s: *concurrency_time_window_s,
            cache_ttl: module.cache_ttl.map(|x| x as i32),
            dedicated_worker: None,
        }),
        tag: tag.clone(),
        delete_after_use: *delete_after_use,
        timeout: module.timeout,
    }
}

fn flow_to_payload(path: &str, delete_after_use: &bool) -> JobPayloadWithTag {
    let payload = JobPayload::Flow { path: path.to_string(), dedicated_worker: None };
    JobPayloadWithTag { payload, tag: None, delete_after_use: *delete_after_use, timeout: None }
}

async fn script_to_payload(
    script_hash: &Option<windmill_common::scripts::ScriptHash>,
    script_path: &String,
    db: &sqlx::Pool<sqlx::Postgres>,
    flow_job: &QueuedJob,
    module: &FlowModule,
    tag_override: &Option<String>,
) -> Result<JobPayloadWithTag, Error> {
    let (payload, tag, delete_after_use, script_timeout) = if script_hash.is_none() {
        script_path_to_payload(script_path, &db, &flow_job.workspace_id).await?
    } else {
        let hash = script_hash.clone().unwrap();
        let mut tx: sqlx::Transaction<'_, sqlx::Postgres> = db.begin().await?;
        let (
            tag,
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            language,
            dedicated_worker,
            priority,
            delete_after_use,
            script_timeout,
        ) = script_hash_to_tag_and_limits(&hash, &mut tx, &flow_job.workspace_id).await?;
        (
            JobPayload::ScriptHash {
                hash,
                path: script_path.to_owned(),
                custom_concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                cache_ttl: module.cache_ttl.map(|x| x as i32).ok_or(cache_ttl).ok(),
                language,
                dedicated_worker,
                priority,
            },
            tag_override.to_owned().or(tag),
            delete_after_use,
            script_timeout,
        )
    };
    // the module value overrides the value set at the script level. Defaults to false if both are unset.
    let final_delete_after_user =
        module.delete_after_use.unwrap_or(false) || delete_after_use.unwrap_or(false);
    let flow_step_timeout = module.timeout.or(script_timeout);
    Ok(JobPayloadWithTag {
        payload,
        tag,
        delete_after_use: final_delete_after_user,
        timeout: flow_step_timeout,
    })
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

// trait IntoArray: Sized {
//     fn into_array(self) -> Result<Vec<Value>, Self>;
// }

// impl IntoArray for Value {
//     fn into_array(self) -> Result<Vec<Value>, Self> {
//         match self {
//             Value::Array(array) => Ok(array),
//             not_array => Err(not_array),
//         }
//     }
// }

fn from_now(duration: Duration) -> chrono::DateTime<chrono::Utc> {
    // "This function errors when original duration is larger than
    // the maximum value supported for this type."
    chrono::Duration::from_std(duration)
        .ok()
        .and_then(|d| chrono::Utc::now().checked_add_signed(d))
        .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC)
}

/// returns previous module non-zero suspend count and job, if relevant
fn needs_resume(flow: &FlowValue, status: &FlowStatus) -> Option<(Suspend, Uuid)> {
    // for a restarted job, if the restarted step is just after a suspend, don't run the suspend
    if status.restarted_from.is_some() {
        let current_step_id = flow.modules.get(status.step as usize)?.id.clone();
        if status.restarted_from.as_ref().unwrap().step_id == current_step_id {
            return None;
        }
    }
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

// returns the result of the previous step of a running flow (if the job was successful)
async fn get_previous_job_result(
    db: &sqlx::Pool<sqlx::Postgres>,
    w_id: &str,
    flow_status: &FlowStatus,
) -> error::Result<Option<Box<RawValue>>> {
    let prev = usize::try_from(flow_status.step)
        .ok()
        .and_then(|s| s.checked_sub(1))
        .with_context(|| "No step preceding the current one")?;

    match flow_status.modules.get(prev) {
        Some(FlowStatusModule::Success { flow_jobs: Some(flow_jobs), .. }) => {
            Ok(Some(retrieve_flow_jobs_results(db, w_id, flow_jobs).await?))
        }
        Some(FlowStatusModule::Success { job, .. }) => Ok(Some(
            sqlx::query_scalar::<_, Json<Box<RawValue>>>(
                "SELECT result FROM completed_job WHERE id = $1 AND workspace_id = $2",
            )
            .bind(job)
            .bind(w_id)
            .fetch_one(db)
            .await?
            .0,
        )),
        _ => Ok(None),
    }
}
