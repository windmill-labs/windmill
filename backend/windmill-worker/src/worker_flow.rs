/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::common::{cached_result_path, save_in_cache};
use crate::js_eval::{eval_timeout, IdContext};
use crate::worker_utils::get_tag_and_concurrency;
use crate::{
    JobCompletedSender, PreviousResult, SameWorkerSender, SendResultPayload, UpdateFlow,
    KEEP_JOB_DIR,
};

use anyhow::Context;
use futures::TryFutureExt;
use mappable_rc::Marc;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::{json, Value};
use sqlx::types::Json;
use sqlx::{FromRow, Postgres, Transaction};
use tracing::instrument;
use uuid::Uuid;
use windmill_common::auth::JobPerms;
#[cfg(feature = "benchmark")]
use windmill_common::bench::BenchmarkIter;
use windmill_common::cache::{self, RawData};
use windmill_common::client::AuthedClient;
use windmill_common::db::Authed;
use windmill_common::flow_status::{
    ApprovalConditions, FlowStatusModuleWParent, Iterator as FlowIterator, JobResult,
};
use windmill_common::flows::{add_virtual_items_if_necessary, Branch, FlowNodeId, StopAfterIf};
use windmill_common::jobs::{
    script_path_to_payload, JobKind, JobPayload, OnBehalfOf, RawCode, ENTRYPOINT_OVERRIDE,
};
use windmill_common::scripts::ScriptHash;
use windmill_common::users::username_to_permissioned_as;
use windmill_common::utils::WarnAfterExt;
use windmill_common::worker::to_raw_value;
use windmill_common::{
    add_time, get_latest_flow_version_info_for_path, get_script_info_for_hash, FlowVersionInfo,
    ScriptHashInfo,
};
use windmill_common::{
    error::{self, to_anyhow, Error},
    flow_status::{
        Approval, BranchAllStatus, BranchChosen, FlowStatus, FlowStatusModule, RetryStatus,
        MAX_RETRY_ATTEMPTS, MAX_RETRY_INTERVAL,
    },
    flows::{FlowModule, FlowModuleValue, FlowValue, InputTransform, Retry, Suspend},
};
use windmill_queue::flow_status::Step;
use windmill_queue::schedule::get_schedule_opt;
use windmill_queue::{
    add_completed_job, add_completed_job_error, append_logs, get_mini_pulled_job,
    handle_maybe_scheduled_job, insert_concurrency_key, interpolate_args, CanceledBy,
    MiniPulledJob, PushArgs, PushIsolationLevel, SameWorkerPayload, WrappedError,
};

type DB = sqlx::Pool<sqlx::Postgres>;

use windmill_audit::audit_oss::{audit_log, AuditAuthor};
use windmill_audit::ActionKind;
use windmill_queue::{canceled_job_to_result, push};

// #[instrument(level = "trace", skip_all)]
pub async fn update_flow_status_after_job_completion(
    db: &DB,
    client: &AuthedClient,
    flow: uuid::Uuid,
    job_id_for_status: &Uuid,
    w_id: &str,
    success: bool,
    result: Arc<Box<RawValue>>,
    unrecoverable: bool,
    same_worker_tx: &SameWorkerSender,
    worker_dir: &str,
    stop_early_override: Option<bool>,
    worker_name: &str,
    job_completed_tx: JobCompletedSender,
    #[cfg(feature = "benchmark")] bench: &mut BenchmarkIter,
) -> error::Result<Option<Arc<MiniPulledJob>>> {
    // this is manual tailrecursion because async_recursion blows up the stack
    potentially_crash_for_testing();

    let mut rec = RecUpdateFlowStatusAfterJobCompletion {
        flow,
        job_id_for_status: job_id_for_status.clone(),
        success,
        result,
        stop_early_override,
        skip_error_handler: false,
    };
    let mut unrecoverable = unrecoverable;
    loop {
        potentially_crash_for_testing();
        let nrec = match update_flow_status_after_job_completion_internal(
            db,
            client,
            rec.flow,
            &rec.job_id_for_status,
            w_id,
            rec.success,
            rec.result,
            unrecoverable,
            same_worker_tx,
            worker_dir,
            rec.stop_early_override,
            rec.skip_error_handler,
            worker_name,
            job_completed_tx.clone(),
            #[cfg(feature = "benchmark")]
            bench,
        )
        .await
        {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("Error while updating flow status of {} after  completion of {}, updating flow status again with error: {e:#}", rec.flow, &rec.job_id_for_status);
                update_flow_status_after_job_completion_internal(
                    db,
                    client,
                    rec.flow,
                    &rec.job_id_for_status,
                    w_id,
                    false,
                    Arc::new(to_raw_value(&Json(&WrappedError {
                        error: json!(e.to_string()),
                    }))),
                    true,
                    same_worker_tx,
                    worker_dir,
                    rec.stop_early_override,
                    rec.skip_error_handler,
                    worker_name,
                    job_completed_tx.clone(),
                    #[cfg(feature = "benchmark")]
                    bench,
                )
                .await?
            }
        };
        unrecoverable = false;

        match nrec {
            UpdateFlowStatusAfterJobCompletion::Done(job) => {
                add_time!(bench, "update flow status internal END");
                return Ok(Some(job));
            }
            UpdateFlowStatusAfterJobCompletion::Rec(nrec) => {
                rec = nrec;
            }
            UpdateFlowStatusAfterJobCompletion::NonLastParallelBranch => {
                add_time!(bench, "update flow status internal END");
                return Ok(None);
            }
            UpdateFlowStatusAfterJobCompletion::NotDone => {
                add_time!(bench, "update flow status internal END");
                return Ok(None);
            }
            UpdateFlowStatusAfterJobCompletion::PreprocessingStep => {
                add_time!(bench, "update flow status preprocessing step END");
                return Ok(None);
            }
        }
    }
}

pub enum UpdateFlowStatusAfterJobCompletion {
    Rec(RecUpdateFlowStatusAfterJobCompletion),
    Done(Arc<MiniPulledJob>),
    NotDone,
    NonLastParallelBranch,
    PreprocessingStep,
}
pub struct RecUpdateFlowStatusAfterJobCompletion {
    flow: uuid::Uuid,
    job_id_for_status: Uuid,
    success: bool,
    result: Arc<Box<RawValue>>,
    stop_early_override: Option<bool>,
    skip_error_handler: bool,
}

#[derive(Deserialize)]
struct RecoveryObject {
    recover: Option<bool>,
}

fn get_stop_after_if_data(
    stop_early: bool,
    stop_after_if: Option<&StopAfterIf>,
) -> (bool, Option<String>) {
    if let Some(stop_after_if) = stop_after_if {
        let err_msg = stop_early
            .then(|| {
                let err_msg = stop_after_if.error_message.as_ref().and_then(|message| {
                    let err_start_msg = "Flow early stop";
                    let s = if message.is_empty() {
                        format!("{}: {}", err_start_msg, &stop_after_if.expr)
                    } else {
                        format!("{}: {}", err_start_msg, message)
                    };
                    Some(s)
                });
                err_msg
            })
            .flatten();
        return (stop_after_if.skip_if_stopped, err_msg);
    }
    return (false, None);
}

// #[instrument(level = "trace", skip_all)]
pub async fn update_flow_status_after_job_completion_internal(
    db: &DB,
    client: &AuthedClient,
    flow: uuid::Uuid,
    job_id_for_status: &Uuid,
    w_id: &str,
    mut success: bool,
    result: Arc<Box<RawValue>>,
    unrecoverable: bool,
    same_worker_tx: &SameWorkerSender,
    worker_dir: &str,
    stop_early_override: Option<bool>,
    skip_error_handler: bool,
    worker_name: &str,
    job_completed_tx: JobCompletedSender,
    #[cfg(feature = "benchmark")] bench: &mut BenchmarkIter,
) -> error::Result<UpdateFlowStatusAfterJobCompletion> {
    add_time!(bench, "update flow status internal START");
    let (
        should_continue_flow,
        flow_job,
        flow_data,
        stop_early,
        stop_early_err_msg,
        skip_if_stop_early,
        nresult,
        is_failure_step,
        _cleanup_module,
    ) = {
        // tracing::debug!("UPDATE FLOW STATUS: {flow:?} {success} {result:?} {w_id} {depth}");

        let (job_kind, script_hash, old_status, raw_flow) = sqlx::query!(
             "SELECT
                 kind AS \"job_kind!: JobKind\",
                 runnable_id AS \"script_hash: ScriptHash\",
                 flow_status AS \"flow_status!: Json<Box<RawValue>>\",
                 raw_flow AS \"raw_flow: Json<Box<RawValue>>\"
             FROM v2_job INNER JOIN v2_job_status ON v2_job.id = v2_job_status.id WHERE v2_job.id = $1 AND v2_job.workspace_id = $2 LIMIT 1",
             flow,
             w_id
         )
         .fetch_one(db)
         .await
         .map_err(|e| {
             Error::internal_err(format!(
                 "fetching flow status {flow} while reporting {success} {result:?}: {e:#}"
             ))
         })
         .and_then(|record| {
             Ok((
                 record.job_kind,
                 record.script_hash,
                 serde_json::from_str::<FlowStatus>(record.flow_status.0.get()).map_err(|e| {
                     Error::internal_err(format!(
                         "requiring current module to be parsable as FlowStatus: {e:?}"
                     ))
                 })?,
                 record.raw_flow,
             ))
         })?;

        let flow_data = cache::job::fetch_flow(db, job_kind, script_hash)
            .or_else(|_| cache::job::fetch_preview_flow(db, &flow, raw_flow))
            .await?;
        let flow_value = flow_data.value();

        let module_step = Step::from_i32_and_len(old_status.step, old_status.modules.len());
        let current_module = match module_step {
            Step::Step(i) => flow_value.modules.get(i),
            _ => None,
        };
        let module_status = match module_step {
            Step::PreprocessorStep => old_status
                .preprocessor_module
                .as_ref()
                .ok_or_else(|| Error::internal_err(format!("preprocessor module not found")))?,
            Step::FailureStep => &old_status.failure_module.module_status,
            Step::Step(i) => old_status
                .modules
                .get(i as usize)
                .ok_or_else(|| Error::internal_err(format!("module {i} not found")))?,
        };

        // tracing::debug!(
        //     "UPDATE FLOW STATUS 2: {module_step:#?} {module_status:#?} {old_status:#?} "
        // );

        let (is_loop, skip_loop_failures, parallelism) = if matches!(
            module_status,
            FlowStatusModule::InProgress { iterator: Some(_), .. }
        ) {
            let value = current_module
                .as_ref()
                .and_then(|x| x.get_value_with_skip_failures().ok());
            (
                true,
                value
                    .as_ref()
                    .and_then(|x| x.skip_failures)
                    .unwrap_or(false),
                value.as_ref().and_then(|x| x.parallelism),
            )
        } else {
            (false, false, None)
        };

        let is_branch_all = matches!(
            module_status,
            FlowStatusModule::InProgress { branchall: Some(_), .. }
        );

        // 0 length flows are not failure steps
        let is_failure_step =
            old_status.step >= old_status.modules.len() as i32 && old_status.modules.len() > 0;

        let (mut stop_early, mut stop_early_err_msg, mut skip_if_stop_early, continue_on_error) =
            if let Some(se) = stop_early_override {
                //do not stop early if module is a flow step
                let step = match module_step {
                    Step::PreprocessorStep => None,
                    Step::FailureStep => None,
                    Step::Step(i) => Some(i),
                };

                let is_flow = if let Some(_) = step {
                    #[derive(Deserialize)]
                    struct GetType<'j> {
                        r#type: &'j str,
                    }

                    current_module
                        .map(|module| {
                            serde_json::from_str::<GetType>(module.value.get())
                                .map(|v| v.r#type == "flow")
                        })
                        .unwrap_or(Ok(false))
                        .unwrap_or(false)
                } else {
                    false
                };

                if is_flow {
                    (false, None, false, false)
                } else {
                    (true, None, se, false)
                }
            } else if is_failure_step || matches!(module_step, Step::PreprocessorStep) {
                (false, None, false, false)
            } else if let Some(current_module) = current_module {
                let stop_early = success
                    && !is_branch_all
                    && if let Some(expr) = current_module
                        .stop_after_if
                        .as_ref()
                        .map(|x| x.expr.as_str())
                    {
                        let all_iters =
                            match &module_status {
                                FlowStatusModule::InProgress {
                                    flow_jobs: Some(flow_jobs), ..
                                } if expr.contains("all_iters") => Some(Arc::new(
                                    retrieve_flow_jobs_results(db, w_id, flow_jobs).await?,
                                )),
                                _ => None,
                            };
                        let args = sqlx::query_scalar!(
                            "SELECT
                                 args AS \"args: Json<HashMap<String, Box<RawValue>>>\"
                             FROM v2_job
                             WHERE id = $1",
                            flow
                        )
                        .fetch_one(db)
                        .await
                        .map_err(|e| {
                            Error::internal_err(format!("retrieval of args from state: {e:#}"))
                        })?;
                        compute_bool_from_expr(
                            &expr,
                            Marc::new(args.unwrap_or_default().0),
                            result.clone(),
                            all_iters,
                            None,
                            Some(client),
                            None,
                            None,
                        )
                        .await?
                    } else {
                        false
                    };
                let (skip_if_stopped, stop_early_err_msg) =
                    get_stop_after_if_data(stop_early, current_module.stop_after_if.as_ref());
                (
                    stop_early,
                    stop_early_err_msg.filter(|_| !(is_loop || is_branch_all)),
                    skip_if_stopped,
                    current_module.continue_on_error.unwrap_or(false),
                )
            } else {
                (false, None, false, false)
            };

        let skip_seq_branch_failure = match module_status {
            FlowStatusModule::InProgress {
                branchall: Some(BranchAllStatus { branch, .. }),
                parallel: false,
                ..
            } => {
                compute_skip_branchall_failure(branch.to_owned(), false, current_module, &None)
                    .await?
            }
            _ => false,
        };

        let mut tx = db.begin().await?;

        add_time!(bench, "process module status START");

        let (inc_step_counter, new_status) = match module_status {
            FlowStatusModule::InProgress {
                iterator,
                branchall,
                parallel,
                flow_jobs: Some(jobs),
                flow_jobs_success,
                ..
            } if *parallel => {
                let (nindex, len) = match (iterator, branchall) {
                    (Some(FlowIterator { itered, .. }), _) => {
                        let position = if flow_jobs_success.is_some() {
                            find_flow_job_index(jobs, job_id_for_status)
                        } else {
                            None
                        };

                        let nindex = if let Some(position) = position {
                             sqlx::query_scalar!(
                                 "UPDATE v2_job_status SET
                                     flow_status = JSONB_SET(
                                         JSONB_SET(flow_status, ARRAY['modules', $1::TEXT, 'flow_jobs_success', $3::TEXT], $4),
                                         ARRAY['modules', $1::TEXT, 'iterator', 'index'],
                                         ((flow_status->'modules'->$1::int->'iterator'->>'index')::int + 1)::text::jsonb
                                     )
                                 WHERE id = $2
                                 RETURNING (flow_status->'modules'->$1::int->'iterator'->>'index')::int",
                                 old_status.step,
                                 flow,
                                 position as i32,
                                 json!(success)
                             )
                         } else {
                             sqlx::query_scalar!(
                                 "UPDATE v2_job_status SET
                                     flow_status = JSONB_SET(
                                         flow_status,
                                         ARRAY['modules', $1::TEXT, 'iterator', 'index'],
                                         ((flow_status->'modules'->$1::int->'iterator'->>'index')::int + 1)::text::jsonb
                                     )
                                 WHERE id = $2
                                 RETURNING (flow_status->'modules'->$1::int->'iterator'->>'index')::int",
                                 old_status.step,
                                 flow
                             )
                         }
                         .fetch_one(&mut *tx)
                         .await.map_err(|e| {
                             Error::internal_err(format!(
                                 "error while fetching iterator index: {e:#}"
                             ))
                         })?
                         .ok_or_else(|| Error::internal_err(format!("requiring an index in InProgress")))?;
                        tracing::info!(
                             "parallel iteration {job_id_for_status} of flow {flow} update nindex: {nindex} len: {len}",
                             nindex = nindex,
                             len = itered.len()
                         );
                        (nindex, itered.len() as i32)
                    }
                    (_, Some(BranchAllStatus { len, .. })) => {
                        let position = if flow_jobs_success.is_some() {
                            find_flow_job_index(jobs, job_id_for_status)
                        } else {
                            None
                        };

                        let nindex = if let Some(position) = position {
                             sqlx::query_scalar!(
                                 "UPDATE v2_job_status SET
                                     flow_status = JSONB_SET(
                                         JSONB_SET(flow_status, ARRAY['modules', $1::TEXT, 'flow_jobs_success', $3::TEXT], $4),
                                         ARRAY['modules', $1::TEXT, 'branchall', 'branch'],
                                         ((flow_status->'modules'->$1::int->'branchall'->>'branch')::int + 1)::text::jsonb
                                     )
                                 WHERE id = $2
                                 RETURNING (flow_status->'modules'->$1::int->'branchall'->>'branch')::int",
                                 old_status.step,
                                 flow,
                                 position as i32,
                                 json!(success)
                             )
                         } else {
                             sqlx::query_scalar!(
                                 "UPDATE v2_job_status SET
                                     flow_status = JSONB_SET(
                                         flow_status,
                                         ARRAY['modules', $1::TEXT, 'branchall', 'branch'],
                                         ((flow_status->'modules'->$1::int->'branchall'->>'branch')::int + 1)::text::jsonb
                                     )
                                 WHERE id = $2
                                 RETURNING (flow_status->'modules'->$1::int->'branchall'->>'branch')::int",
                                 old_status.step,
                                 flow
                             )
                         }
                         .fetch_one(&mut *tx)
                         .await
                         .map_err(|e| {
                             Error::internal_err(format!(
                                 "error while fetching branchall index: {e:#}"
                             ))
                         })?
                         .ok_or_else(|| Error::internal_err(format!("requiring an index in InProgress")))?;
                        (nindex, *len as i32)
                    }
                    _ => Err(Error::internal_err(format!(
                        "unexpected status for parallel module"
                    )))?,
                };
                // reset ping after updating flow status:
                let _ = sqlx::query!("UPDATE v2_job_runtime SET ping = NULL WHERE id = $1", flow)
                    .execute(&mut *tx)
                    .await?;
                if nindex == len {
                    let mut flow_jobs_success = flow_jobs_success.clone();
                    if let Some(flow_job_success) = flow_jobs_success.as_mut() {
                        let position = jobs.iter().position(|x| x == job_id_for_status);
                        if let Some(position) = position {
                            if position < flow_job_success.len() {
                                flow_job_success[position] = Some(success);
                            }
                        }
                    }

                    let new_status = if skip_loop_failures
                         || sqlx::query_scalar!(
                             "SELECT status = 'success' OR status = 'skipped' AS \"success!\" FROM v2_job_completed WHERE id = ANY($1)",
                             jobs.as_slice()
                         )
                         .fetch_all(&mut *tx)
                         .await
                         .map_err(|e| {
                             Error::internal_err(format!(
                                 "error while fetching sucess from completed_jobs: {e:#}"
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
                             flow_jobs_success: flow_jobs_success.clone(),
                             branch_chosen: None,
                             approvers: vec![],
                             failed_retries: vec![],
                             skipped: false,
                         }
                     } else {
                         success = false;
                         FlowStatusModule::Failure {
                             id: module_status.id(),
                             job: job_id_for_status.clone(),
                             flow_jobs: Some(jobs.clone()),
                             flow_jobs_success: flow_jobs_success.clone(),
                             branch_chosen: None,
                             failed_retries: vec![],
                         }
                     };
                    let r = sqlx::query_scalar!(
                         "DELETE FROM parallel_monitor_lock WHERE parent_flow_id = $1 RETURNING last_ping",
                         flow,
                     ).fetch_optional(db).await.map_err(|e| {
                         Error::internal_err(format!(
                             "error while deleting parallel_monitor_lock: {e:#}"
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
                    add_time!(bench, "handle parallel flow start");
                    tx.commit().await?;

                    if parallelism.is_some() {
                        sqlx::query!(
                            "UPDATE v2_job_queue q SET suspend = 0
                             FROM v2_job j, v2_job_status f
                             WHERE q.workspace_id = $1 AND q.suspend = $3 AND j.parent_job = $2
                                 AND f.id = j.id AND q.id = j.id
                                 AND (f.flow_status->'step')::int = 0",
                            w_id,
                            flow,
                            nindex
                        )
                        .execute(db)
                        .await
                        .map_err(|e| {
                            Error::internal_err(format!(
                                "error resuming job at suspend {nindex} and parent {flow}: {e:#}"
                            ))
                        })?;
                    }

                    let r = sqlx::query_scalar!(
                         "DELETE FROM parallel_monitor_lock WHERE parent_flow_id = $1 and job_id = $2 RETURNING last_ping",
                         flow,
                         job_id_for_status
                     ).fetch_optional(db).await.map_err(|e| {
                         Error::internal_err(format!("error while removing parallel_monitor_lock: {e:#}"))
                     })?;
                    if r.is_some() {
                        tracing::info!(
                            "parallel flow has removed lock on its parent, last ping was {:?}",
                            r.unwrap()
                        );
                    }
                    add_time!(bench, "non final parallel flow finished");
                    return Ok(UpdateFlowStatusAfterJobCompletion::NonLastParallelBranch);
                }
            }
            FlowStatusModule::InProgress {
                iterator: Some(FlowIterator { index, itered, .. }),
                flow_jobs_success,
                flow_jobs,
                while_loop,
                ..
            } if (*while_loop
                || (*index + 1 < itered.len()) && (success || skip_loop_failures))
                && !stop_early =>
            {
                if let Some(jobs) = flow_jobs {
                    set_success_in_flow_job_success(
                        flow_jobs_success,
                        jobs,
                        job_id_for_status,
                        &old_status,
                        flow,
                        success,
                        &mut tx,
                    )
                    .await?;
                }

                (false, None)
            }
            FlowStatusModule::InProgress {
                branchall: Some(BranchAllStatus { branch, len, .. }),
                flow_jobs_success,
                flow_jobs,
                ..
            } if branch.to_owned() < len - 1 && (success || skip_seq_branch_failure) => {
                if let Some(jobs) = flow_jobs {
                    set_success_in_flow_job_success(
                        flow_jobs_success,
                        jobs,
                        job_id_for_status,
                        &old_status,
                        flow,
                        success,
                        &mut tx,
                    )
                    .await?;
                }
                (false, None)
            }
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

                let flow_jobs = module_status.flow_jobs();
                let branch_chosen = module_status.branch_chosen();
                let mut flow_jobs_success = module_status.flow_jobs_success();

                if let (Some(flow_job_success), Some(flow_jobs)) =
                    (flow_jobs_success.as_mut(), flow_jobs.as_ref())
                {
                    let position = flow_jobs.iter().position(|x| x == job_id_for_status);
                    if let Some(position) = position {
                        if position < flow_job_success.len() {
                            flow_job_success[position] = Some(success);
                        }
                    }
                }
                if success
                    || (flow_jobs.is_some() && (skip_loop_failures || skip_seq_branch_failure))
                {
                    let is_skipped = if current_module.as_ref().is_some_and(|m| m.skip_if.is_some())
                    {
                        sqlx::query_scalar!(
                            "SELECT kind = 'identity' FROM v2_job WHERE id = $1",
                            job_id_for_status
                        )
                        .fetch_one(db)
                        .await
                        .map_err(|e| {
                            Error::internal_err(format!("error during skip check: {e:#}"))
                        })?
                        .unwrap_or(false)
                    } else {
                        false
                    };
                    success = true;
                    (
                        true,
                        Some(FlowStatusModule::Success {
                            id: module_status.id(),
                            job: job_id_for_status.clone(),
                            flow_jobs,
                            flow_jobs_success,
                            branch_chosen,
                            approvers: vec![],
                            failed_retries: old_status.retry.failed_jobs.clone(),
                            skipped: is_skipped,
                        }),
                    )
                } else {
                    let inc = if continue_on_error {
                        let retry = current_module
                            .as_ref()
                            .and_then(|x| x.retry.clone())
                            .unwrap_or_default();

                        tracing::info!("update flow status on rety: {retry:#?} ");
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
                            flow_jobs_success,
                            branch_chosen,
                            failed_retries: old_status.retry.failed_jobs.clone(),
                        }),
                    )
                }
            }
        };

        let skip_parallel_branchall_failure = match (module_status, new_status.as_ref()) {
            (
                FlowStatusModule::InProgress { branchall: Some(_), parallel: true, .. },
                Some(FlowStatusModule::Success { flow_jobs_success, .. }),
            ) => compute_skip_branchall_failure(0, true, current_module, flow_jobs_success).await?,
            (
                FlowStatusModule::InProgress { branchall: Some(_), parallel: true, .. },
                Some(FlowStatusModule::Failure { flow_jobs_success, .. }),
            ) => compute_skip_branchall_failure(0, true, current_module, flow_jobs_success).await?,
            _ => false,
        };
        let step_counter = if inc_step_counter {
            sqlx::query!(
                "UPDATE v2_job_status
                 SET flow_status = JSONB_SET(flow_status, ARRAY['step'], $1)
                 WHERE id = $2",
                json!(old_status.step + 1),
                flow
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                Error::internal_err(format!("error while setting flow index for {flow}: {e:#}"))
            })?;
            old_status.step + 1
        } else {
            old_status.step
        };

        // tracing::error!(
        //     "step_counter: {:?} {} {inc_step_counter} {flow}",
        //     step_counter,
        //     old_status.step,
        // );
        // panic!("stop");

        /* is_last_step is true when the step_counter (the next step index) is an invalid index */
        let is_last_step = usize::try_from(step_counter)
            .map(|i| !(..old_status.modules.len()).contains(&i))
            .unwrap_or(true);

        if let Some(new_status) = new_status.as_ref() {
            if is_failure_step {
                let parent_module = sqlx::query_scalar!(
                     "SELECT flow_status->'failure_module'->>'parent_module' FROM v2_job_status WHERE id = $1",
                     flow
                 )
                 .fetch_one(&mut *tx)
                 .await.map_err(|e| {
                     Error::internal_err(format!(
                         "error while fetching failure module: {e:#}"
                     ))
                 })?;

                sqlx::query!(
                    "UPDATE v2_job_status
                     SET flow_status = JSONB_SET(flow_status, ARRAY['failure_module'], $1)
                     WHERE id = $2",
                    json!(FlowStatusModuleWParent {
                        parent_module,
                        module_status: new_status.clone()
                    }),
                    flow
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    Error::internal_err(format!(
                        "error while setting flow status in failure step: {e:#}"
                    ))
                })?;
            } else if matches!(module_step, Step::PreprocessorStep) {
                sqlx::query!(
                    "UPDATE v2_job_status
                     SET flow_status = JSONB_SET(flow_status, ARRAY['preprocessor_module'], $1)
                     WHERE id = $2",
                    json!(new_status),
                    flow
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    Error::internal_err(format!(
                        "error while setting flow status in preprocessing step: {e:#}"
                    ))
                })?;
            } else {
                sqlx::query!(
                    "UPDATE v2_job_status
                     SET flow_status = JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2)
                     WHERE id = $3",
                    old_status.step.to_string(),
                    json!(new_status),
                    flow
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    Error::internal_err(format!("error while setting new flow status: {e:#}"))
                })?;

                if let Some(job_result) = new_status.job_result() {
                    sqlx::query!(
                         "UPDATE v2_job_status
                         SET flow_leaf_jobs = JSONB_SET(coalesce(flow_leaf_jobs, '{}'::jsonb), ARRAY[$1::TEXT], $2)
                         WHERE COALESCE((SELECT flow_innermost_root_job FROM v2_job WHERE id = $3), $3) = id",
                         new_status.id(),
                         json!(job_result),
                         flow
                     )
                     .execute(&mut *tx)
                     .await.map_err(|e| {
                         Error::internal_err(format!(
                             "error while setting leaf jobs: {e:#}"
                         ))
                     })?;
                }
            }
        }

        let mut nresult = if let Some(stop_early_err_msg) = stop_early_err_msg.as_ref() {
            Arc::new(to_raw_value(stop_early_err_msg))
        } else {
            match &new_status {
                Some(FlowStatusModule::Success { flow_jobs: Some(jobs), .. })
                | Some(FlowStatusModule::Failure { flow_jobs: Some(jobs), .. }) => {
                    Arc::new(retrieve_flow_jobs_results(db, w_id, jobs).await?)
                }
                _ => result.clone(),
            }
        };

        match &new_status {
            Some(FlowStatusModule::Success { .. }) if is_loop || is_branch_all => {
                if let Some(stop_after_all_iters_if) = current_module
                    .as_ref()
                    .and_then(|m| m.stop_after_all_iters_if.as_ref())
                {
                    let args = sqlx::query_scalar!(
                        "SELECT args AS \"args: Json<HashMap<String, Box<RawValue>>>\"
                         FROM v2_job WHERE id = $1",
                        flow
                    )
                    .fetch_one(db)
                    .await
                    .map_err(|e| {
                        Error::internal_err(format!("retrieval of args from state: {e:#}"))
                    })?;

                    let should_stop = compute_bool_from_expr(
                        &stop_after_all_iters_if.expr,
                        Marc::new(args.unwrap_or_default().0),
                        nresult.clone(),
                        None,
                        None,
                        Some(client),
                        None,
                        None,
                    )
                    .await?;

                    if should_stop {
                        stop_early = true;
                        let (skip_if_stopped, err_msg_internal) =
                            get_stop_after_if_data(should_stop, Some(stop_after_all_iters_if));
                        skip_if_stop_early = skip_if_stopped;
                        if err_msg_internal.is_some() {
                            stop_early_err_msg = err_msg_internal;
                            nresult = Arc::new(to_raw_value(&stop_early_err_msg));
                        }
                    }
                }
            }
            _ => {}
        }

        if old_status.retry.fail_count > 0
            && matches!(&new_status, Some(FlowStatusModule::Success { .. }))
        {
            sqlx::query!(
                "UPDATE v2_job_status
                 SET flow_status = flow_status - 'retry'
                 WHERE id = $1",
                flow
            )
            .execute(&mut *tx)
            .await
            .context("remove flow status retry")?;
        }

        let flow_job = get_mini_pulled_job(&mut *tx, &flow)
            .await?
            .ok_or_else(|| Error::internal_err(format!("requiring flow to be in the queue")))?;
        tx.commit().await?;

        if matches!(module_step, Step::PreprocessorStep) {
            let tag_and_concurrency_key = get_tag_and_concurrency(&flow, db).await;
            let require_args = tag_and_concurrency_key.as_ref().is_some_and(|x| {
                x.tag.as_ref().is_some_and(|t| t.contains("$args"))
                    || x.concurrency_key
                        .as_ref()
                        .is_some_and(|ck| ck.contains("$args"))
            });
            let mut tag = tag_and_concurrency_key
                .as_ref()
                .map(|x| x.tag.clone())
                .flatten();
            let concurrency_key = tag_and_concurrency_key
                .as_ref()
                .map(|x| x.concurrency_key.clone())
                .flatten();
            let concurrent_limit = tag_and_concurrency_key
                .as_ref()
                .map(|x| x.concurrent_limit)
                .flatten();
            let concurrency_time_window_s = tag_and_concurrency_key
                .as_ref()
                .map(|x| x.concurrency_time_window_s)
                .flatten();
            if require_args {
                let args = sqlx::query_scalar!(
                    "SELECT result  as \"result: Json<HashMap<String, Box<RawValue>>>\"
                 FROM v2_job_completed 
                 WHERE id = $1",
                    job_id_for_status
                )
                .fetch_one(db)
                .await
                .map_err(|e| {
                    Error::internal_err(format!("error while fetching preprocessing args: {e:#}"))
                })?;
                let args_hm = args.unwrap_or_default().0;
                let args = PushArgs::from(&args_hm);
                if let Some(ck) = concurrency_key {
                    let mut tx = db.begin().await?;
                    insert_concurrency_key(
                        &flow_job.workspace_id,
                        &args,
                        &flow_job.runnable_path,
                        JobKind::Flow,
                        Some(ck),
                        &mut tx,
                        flow,
                    )
                    .await?;
                    tx.commit().await?;
                }
                if let Some(t) = tag {
                    tag = Some(interpolate_args(t, &args, &flow_job.workspace_id));
                }
            } else if let Some(ck) = concurrency_key {
                let mut tx = db.begin().await?;
                insert_concurrency_key(
                    &flow_job.workspace_id,
                    &PushArgs::from(&HashMap::new()),
                    &flow_job.runnable_path,
                    JobKind::Flow,
                    Some(ck),
                    &mut tx,
                    flow,
                )
                .await?;
                tx.commit().await?;
            }

            // let tag = tag_and_concurrency_key.and_then(|tc| tc.tag.map(|t| interpolate_args(t.clone(), &args, &workspace_id)));
            // let concurrency_key = tag_and_concurrency_key.and_then(|tc| tc.concurrency_key.map(|ck| interpolate_args(&ck, &args, &workspace_id)));
            sqlx::query!(
                "WITH job_result AS (
                 SELECT result 
                 FROM v2_job_completed 
                 WHERE id = $1
             ),
             updated_queue AS (
                UPDATE v2_job_queue
                SET running = false,
                tag = COALESCE($3, tag)
                WHERE id = $2
             )
             UPDATE v2_job 
             SET 
                tag = COALESCE($3, tag),
                concurrent_limit = COALESCE($4, concurrent_limit),
                concurrency_time_window_s = COALESCE($5, concurrency_time_window_s),
                args = COALESCE(
                     CASE 
                         WHEN job_result.result IS NULL THEN NULL
                         WHEN jsonb_typeof(job_result.result) = 'object' 
                         THEN job_result.result
                         WHEN jsonb_typeof(job_result.result) = 'null'
                         THEN NULL
                         ELSE jsonb_build_object('value', job_result.result)
                     END, 
                     '{}'::jsonb
                 ),
                 preprocessed = TRUE
             FROM job_result
             WHERE v2_job.id = $2;
             ",
                job_id_for_status,
                flow,
                tag,
                concurrent_limit,
                concurrency_time_window_s,
            )
            .execute(db)
            .await
            .map_err(|e| {
                Error::internal_err(format!(
                    "error while updating args in preprocessing step: {e:#}"
                ))
            })?;
            if success {
                return Ok(UpdateFlowStatusAfterJobCompletion::PreprocessingStep);
            }
        }

        let job_root = flow_job
            .flow_innermost_root_job
            .map(|x| x.to_string())
            .unwrap_or_else(|| "none".to_string());
        tracing::info!(id = %flow_job.id, root_id = %job_root, "update flow status");

        let should_continue_flow = match success {
            _ if stop_early => false,
            _ if flow_job.is_canceled() => false,
            true => !is_last_step,
            false if unrecoverable => false,
            false
                if skip_seq_branch_failure
                    || skip_parallel_branchall_failure
                    || skip_loop_failures
                    || continue_on_error =>
            {
                !is_last_step
            }
            false
                if next_retry(
                    match module_step {
                        Step::PreprocessorStep => flow_value
                            .preprocessor_module
                            .as_ref()
                            .and_then(|m| m.retry.as_ref()),
                        Step::Step(i) => flow_value
                            .modules
                            .get(i)
                            .as_ref()
                            .and_then(|m| m.retry.as_ref()),
                        Step::FailureStep => flow_value
                            .failure_module
                            .as_ref()
                            .and_then(|m| m.retry.as_ref()),
                    }
                    .unwrap_or(&Retry::default()),
                    &old_status.retry,
                )
                .is_some() =>
            {
                true
            }
            false
                if !is_failure_step
                    && !skip_error_handler
                    && flow_value.failure_module.is_some() =>
            {
                true
            }
            false => false,
        };

        tracing::debug!(id = %flow_job.id, root_id = %job_root, "flow status updated");

        (
            should_continue_flow,
            flow_job,
            flow_data,
            stop_early,
            stop_early_err_msg,
            skip_if_stop_early,
            nresult,
            is_failure_step,
            old_status.cleanup_module,
        )
    };

    let flow_job = Arc::new(flow_job);

    let done = if !should_continue_flow {
        {
            let logs = if flow_job.is_canceled() {
                "Flow job canceled\n".to_string()
            } else if stop_early {
                format!("Flow job stopped early because of a stop early predicate returning true\n")
            } else if success {
                "Flow job completed with success\n".to_string()
            } else {
                "Flow job completed with error\n".to_string()
            };
            append_logs(&flow_job.id, w_id, logs, &db.into()).await;
        }
        #[cfg(feature = "enterprise")]
        if let Some(parent_job) = flow_job.parent_job {
            // if has parent job, append flow cleanup modules to parent job
            if !_cleanup_module.flow_jobs_to_clean.is_empty() {
                let uuids_json = serde_json::to_value(&_cleanup_module.flow_jobs_to_clean)
                    .map_err(|e| {
                        error::Error::internal_err(format!("Unable to serialize uuids: {e:#}"))
                    })?;
                sqlx::query!(
                    "UPDATE v2_job_status
                    SET flow_status = JSONB_SET(flow_status, ARRAY['cleanup_module', 'flow_jobs_to_clean'], COALESCE(flow_status->'cleanup_module'->'flow_jobs_to_clean', '[]'::jsonb) || $1)
                    WHERE id = $2",
                    uuids_json,
                    parent_job
                )
                .execute(db)
                .warn_after_seconds(3)
                .await?;
            }
        } else {
            // run the cleanup step only when the root job is complete
            if !_cleanup_module.flow_jobs_to_clean.is_empty() {
                tracing::debug!(
                     "Cleaning up jobs arguments, result and logs as they were marked as delete_after_use {:?}",
                     _cleanup_module.flow_jobs_to_clean
                 );
                sqlx::query!(
                    "UPDATE v2_job SET args = '{}'::jsonb WHERE id = ANY($1)",
                    &_cleanup_module.flow_jobs_to_clean,
                )
                .execute(db)
                .await
                .map_err(|e| {
                    Error::InternalErr(format!("error while cleaning up completed job: {e:#}"))
                })?;
                sqlx::query!(
                    "UPDATE v2_job_completed SET result = '{}'::jsonb WHERE id = ANY($1)",
                    &_cleanup_module.flow_jobs_to_clean,
                )
                .execute(db)
                .await
                .map_err(|e| {
                    Error::internal_err(format!("error while cleaning up completed job: {e:#}"))
                })?;
            }
        }
        if flow_job.is_canceled() {
            add_completed_job_error(
                db,
                &flow_job,
                0,
                Some(CanceledBy {
                    username: flow_job.canceled_by.clone(),
                    reason: flow_job.canceled_reason.clone(),
                }),
                canceled_job_to_result(&flow_job),
                worker_name,
                true,
                None,
            )
            .await?;
        } else {
            if flow_job.cache_ttl.is_some() && success {
                let flow = RawData::Flow(flow_data.clone());
                let cached_res_path = cached_result_path(db, client, &flow_job, Some(&flow)).await;

                save_in_cache(db, client, &flow_job, cached_res_path, nresult.clone()).await;
            }
            fn result_has_recover_true(nresult: Arc<Box<RawValue>>) -> bool {
                let recover = serde_json::from_str::<RecoveryObject>(nresult.get());
                return recover.map(|r| r.recover.unwrap_or(false)).unwrap_or(false);
            }
            let success = success
                && (!is_failure_step || result_has_recover_true(nresult.clone()))
                && !skip_error_handler
                && stop_early_err_msg.is_none();

            add_time!(bench, "flow status update 1");
            if success {
                add_completed_job(
                    db,
                    &flow_job,
                    true,
                    stop_early && skip_if_stop_early,
                    Json(&nresult),
                    None,
                    0,
                    None,
                    true,
                    None,
                )
                .await?;
            } else {
                add_completed_job(
                    db,
                    &flow_job,
                    false,
                    stop_early && skip_if_stop_early,
                    Json(
                        &serde_json::from_str::<Value>(nresult.get()).unwrap_or_else(
                            |e| json!({"error": format!("Impossible to serialize error: {e:#}")}),
                        ),
                    ),
                    None,
                    0,
                    None,
                    true,
                    None,
                )
                .await?;
            }
        }
        true
    } else {
        tracing::debug!(id = %flow_job.id,  "start handle flow");
        match handle_flow(
            flow_job.clone(),
            &flow_data,
            db,
            client,
            Some(nresult.clone()),
            same_worker_tx,
            worker_dir,
            job_completed_tx,
            worker_name,
        )
        .warn_after_seconds(10)
        .await
        {
            Err(err) => {
                let e = json!({"message": err.to_string(), "name": "InternalError"});
                append_logs(
                    &flow_job.id,
                    w_id,
                    format!("Unexpected error during flow chaining:\n{:#?}", e),
                    &db.into(),
                )
                .await;
                let _ = add_completed_job_error(db, &flow_job, 0, None, e, worker_name, true, None)
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

        if flow_job.is_flow_step() {
            if let Some(parent_job) = flow_job.parent_job {
                tracing::info!(subflow_id = %flow_job.id, parent_id = %parent_job, "subflow is finished, updating parent flow status");

                return Ok(UpdateFlowStatusAfterJobCompletion::Rec(
                    RecUpdateFlowStatusAfterJobCompletion {
                        flow: parent_job,
                        job_id_for_status: flow,
                        success: success && !is_failure_step,
                        result: nresult.clone(),
                        stop_early_override: if stop_early {
                            Some(skip_if_stop_early)
                        } else {
                            None
                        },
                        skip_error_handler: skip_error_handler || is_failure_step,
                    },
                ));
            }
        }
        Ok(UpdateFlowStatusAfterJobCompletion::Done(flow_job))
    } else {
        Ok(UpdateFlowStatusAfterJobCompletion::NotDone)
    }
}

fn find_flow_job_index(flow_jobs: &Vec<Uuid>, job_id_for_status: &Uuid) -> Option<usize> {
    flow_jobs.iter().position(|x| x == job_id_for_status)
}

async fn set_success_in_flow_job_success<'c>(
    flow_jobs_success: &Option<Vec<Option<bool>>>,
    flow_jobs: &Vec<Uuid>,
    job_id_for_status: &Uuid,
    old_status: &FlowStatus,
    flow: Uuid,
    success: bool,
    tx: &mut Transaction<'c, Postgres>,
) -> error::Result<()> {
    if flow_jobs_success.is_some() {
        let position = find_flow_job_index(flow_jobs, job_id_for_status);
        if let Some(position) = position {
            sqlx::query!(
                "UPDATE v2_job_status SET
                     flow_status = JSONB_SET(
                         flow_status,
                         ARRAY['modules', $1::TEXT, 'flow_jobs_success', $3::TEXT],
                         $4
                     )
                 WHERE id = $2",
                old_status.step as i32,
                flow,
                position as i32,
                json!(success)
            )
            .execute(&mut **tx)
            .await
            .map_err(|e| {
                Error::internal_err(format!("error while setting flow_jobs_success: {e:#}"))
            })?;
        }
    }
    Ok(())
}

async fn retrieve_flow_jobs_results(
    db: &DB,
    w_id: &str,
    job_uuids: &Vec<Uuid>,
) -> error::Result<Box<RawValue>> {
    let results = sqlx::query!(
        "SELECT result, id
         FROM v2_job_completed
         WHERE id = ANY($1) AND workspace_id = $2",
        job_uuids.as_slice(),
        w_id
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|br| (br.id, br.result))
    .collect::<HashMap<_, _>>();

    let results = job_uuids
        .iter()
        .map(|j| {
            results
                .get(j)
                .ok_or_else(|| Error::internal_err(format!("missing job result for {}", j)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    tracing::debug!("Retrieved results for flow jobs {:?}", results);
    Ok(to_raw_value(&results))
}

async fn compute_skip_branchall_failure<'c>(
    branch: usize,
    parallel: bool,
    flow_module: Option<&FlowModule>,
    successes: &Option<Vec<Option<bool>>>,
) -> windmill_common::error::Result<bool> {
    let branches = flow_module
        .and_then(|x| x.get_branches_skip_failures().ok())
        .map(|x| {
            x.branches
                .iter()
                .map(|b| b.skip_failure.unwrap_or(false))
                .collect::<Vec<_>>()
        });
    if parallel {
        if let Some(successes) = successes {
            for (i, success) in successes.iter().enumerate() {
                if branches
                    .as_ref()
                    .and_then(|x| x.get(i))
                    .unwrap_or(&false)
                    .to_owned()
                {
                    continue;
                }
                if !(success.unwrap_or(false)) {
                    return Ok(false);
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    } else {
        Ok(branches
            .and_then(|x| x.get(branch as usize).map(|b| b.to_owned()))
            .unwrap_or(false))
    }
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
//     .map_err(|e| Error::internal_err(format!("error during retrieval of cleanup module: {e:#}")))?;

//     raw_value
//         .clone()
//         .and_then(|rv| serde_json::from_value::<FlowCleanupModule>(rv).ok())
//         .ok_or(Error::internal_err(format!(
//             "Unable to parse flow cleanup module {:?}",
//             raw_value
//         )))
// }

fn next_retry(retry: &Retry, status: &RetryStatus) -> Option<(u32, Duration)> {
    (status.fail_count <= MAX_RETRY_ATTEMPTS)
        .then(|| &retry)
        .and_then(|retry| retry.interval(status.fail_count, false))
        .map(|d| (status.fail_count + 1, std::cmp::min(d, MAX_RETRY_INTERVAL)))
}

async fn compute_bool_from_expr(
    expr: &str,
    flow_args: Marc<HashMap<String, Box<RawValue>>>,
    result: Arc<Box<RawValue>>,
    all_iters: Option<Arc<Box<RawValue>>>,
    by_id: Option<&IdContext>,
    client: Option<&AuthedClient>,
    resumes: Option<(Arc<Box<RawValue>>, Arc<Box<RawValue>>, Arc<Box<RawValue>>)>,
    ctx: Option<Vec<(String, String)>>,
) -> error::Result<bool> {
    let mut context = HashMap::with_capacity(if resumes.is_some() { 7 } else { 3 });
    context.insert("result".to_string(), result.clone());
    if let Some(all_iters) = all_iters {
        context.insert("all_iters".to_string(), all_iters);
    }
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
        ctx,
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

struct FailureContext {
    started_at: Arc<Box<RawValue>>,
    flow_job_id: Uuid,
}

/// resumes should be in order of timestamp ascending, so that more recent are at the end
#[instrument(level = "trace", skip_all)]
async fn transform_input(
    flow_args: Marc<HashMap<String, Box<RawValue>>>,
    last_result: Arc<Box<RawValue>>,
    input_transforms: &HashMap<String, InputTransform>,
    resumes: Arc<Box<RawValue>>,
    resume: Arc<Box<RawValue>>,
    approvers: Arc<Box<RawValue>>,
    failure_context: Option<FailureContext>,
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
        if let Some(FailureContext { started_at, flow_job_id }) = failure_context {
            env.insert("started_at".to_string(), started_at);
            env.insert(
                "flow_job_id".to_string(),
                Arc::new(to_raw_value(&flow_job_id)),
            );
        }
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
                    Some(by_id),
                    None,
                )
                .await
                .map_err(|e| {
                    Error::ExecutionErr(format!(
                        "Error during isolated evaluation of expression `{expr}`:\n{e:#}"
                    ))
                })?;
                mapped.insert(key.to_string(), v);
            }
        }
    }

    Ok(mapped)
}

#[instrument(level = "trace", skip_all)]
pub async fn handle_flow(
    flow_job: Arc<MiniPulledJob>,
    flow_data: &cache::FlowData,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClient,
    last_result: Option<Arc<Box<RawValue>>>,
    same_worker_tx: &SameWorkerSender,
    worker_dir: &str,
    job_completed_tx: JobCompletedSender,
    worker_name: &str,
) -> anyhow::Result<()> {
    let flow = flow_data.value();
    let status = flow_job
        .parse_flow_status()
        .with_context(|| "Unable to parse flow status")?;

    let schedule_path = flow_job.schedule_path();
    if !flow_job.is_flow_step()
        && status.retry.fail_count == 0
        && schedule_path.is_some()
        && flow_job.runnable_path.is_some()
        && status.step == 0
    {
        let schedule_path = schedule_path.as_ref().unwrap();

        let schedule = get_schedule_opt(db, &flow_job.workspace_id, schedule_path)
            .warn_after_seconds(5)
            .await?;

        if let Some(schedule) = schedule {
            if let Err(err) = handle_maybe_scheduled_job(
                db,
                &flow_job,
                &schedule,
                flow_job.runnable_path.as_ref().unwrap(),
                &flow_job.workspace_id,
            )
            .warn_after_seconds(5)
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
    let mut rec = PushNextFlowJobRec { flow_job: flow_job, status: status };
    loop {
        let PushNextFlowJobRec { flow_job, status } = rec;
        let next = push_next_flow_job(
            flow_job,
            status,
            flow,
            db,
            client,
            last_result.clone(),
            same_worker_tx,
            worker_dir,
            worker_name,
        )
        .warn_after_seconds(10)
        .await?;
        match next {
            PushNextFlowJob::Rec(nrec) => {
                tracing::info!("recursively pushing next flow job {}", nrec.flow_job.id);
                rec = nrec;
            }
            PushNextFlowJob::Done(update_flow) => {
                if let Some(update_flow) = update_flow {
                    tracing::info!(
                        "sending flow status update {} with success {} to job completed channel",
                        update_flow.flow,
                        update_flow.success
                    );
                    job_completed_tx
                        .send(SendResultPayload::UpdateFlow(update_flow), false)
                        .warn_after_seconds(3)
                        .await
                        .map_err(|e| {
                            Error::internal_err(format!(
                                "error sending update flow message to job completed channel: {e:#}"
                            ))
                        })?;
                }
                break;
            }
        }
    }

    Ok(())
}

#[derive(Serialize, Debug)]
pub struct Iter {
    index: i32,
    value: Box<RawValue>,
}

#[derive(FromRow)]
pub struct ResumeRow {
    pub value: Json<Box<RawValue>>,
    pub approver: Option<String>,
    pub approved: bool,
    pub resume_id: i32,
}

lazy_static::lazy_static! {
    static ref CRASH_FORCEFULLY_AT_STEP: Option<usize> = std::env::var("CRASH_FORCEFULLY_AT_STEP")
        .ok()
        .and_then(|x| x.parse::<usize>().ok());

    static ref CRASH_STEP_COUNTER: AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    static ref BRANCHALL_INDEX_RE: regex::Regex = regex::Regex::new(r"/branchall-(\d+)$").unwrap();
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

// static
lazy_static::lazy_static! {
    pub static ref EHM: HashMap<String, Box<RawValue>> = HashMap::new();
}

enum PushNextFlowJob {
    Rec(PushNextFlowJobRec),
    Done(Option<UpdateFlow>),
}
struct PushNextFlowJobRec {
    flow_job: Arc<MiniPulledJob>,
    status: FlowStatus,
}
// #[async_recursion]
// #[instrument(level = "trace", skip_all)]
async fn push_next_flow_job(
    flow_job: Arc<MiniPulledJob>,
    mut status: FlowStatus,
    flow: &FlowValue,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClient,
    last_job_result: Option<Arc<Box<RawValue>>>,
    same_worker_tx: &SameWorkerSender,
    worker_dir: &str,
    worker_name: &str,
) -> error::Result<PushNextFlowJob> {
    let job_root = flow_job
        .flow_innermost_root_job
        .map(|x| x.to_string())
        .unwrap_or_else(|| "none".to_string());
    tracing::info!(id = %flow_job.id, root_id = %job_root, "pushing next flow job");

    let mut step = Step::from_i32_and_len(status.step, flow.modules.len());

    let mut status_module = match step {
        Step::Step(i) => status
            .modules
            .get(i)
            .cloned()
            .unwrap_or_else(|| status.failure_module.module_status.clone()),
        Step::PreprocessorStep => status
            .preprocessor_module
            .clone()
            .unwrap_or_else(|| status.failure_module.module_status.clone()),
        Step::FailureStep => status.failure_module.module_status.clone(),
    };

    let fj: mappable_rc::Marc<MiniPulledJob> = flow_job.clone().into();
    let arc_flow_job_args: Marc<HashMap<String, Box<RawValue>>> = Marc::map(fj, |x| {
        if let Some(args) = &x.args {
            &args.0
        } else {
            &EHM
        }
    });

    // if this is an empty module of if the module has already been completed, successfully, update the parent flow
    if flow.modules.is_empty() || matches!(status_module, FlowStatusModule::Success { .. }) {
        return Ok(PushNextFlowJob::Done(Some(UpdateFlow {
            flow: flow_job.id,
            success: true,
            result: if flow.modules.is_empty() {
                to_raw_value(arc_flow_job_args.as_ref())
            } else {
                // it has to be an empty for loop event
                serde_json::from_str("[]").unwrap()
            },
            stop_early_override: None,
            w_id: flow_job.workspace_id.clone(),
            worker_dir: worker_dir.to_string(),
            token: client.token.clone(),
        })));
    }

    if matches!(step, Step::Step(0)) {
        if !flow_job.is_flow_step() && flow_job.schedule_path().is_some() {
            let schedule_path = flow_job.schedule_path();
            let no_flow_overlap = sqlx::query_scalar!(
                "SELECT no_flow_overlap FROM schedule WHERE path = $1 AND workspace_id = $2",
                schedule_path.as_ref().unwrap(),
                flow_job.workspace_id.as_str()
            )
            .fetch_one(db)
            .warn_after_seconds(3)
            .await?;
            if no_flow_overlap {
                let overlapping = sqlx::query_scalar!(
                     // Query plan:
                     // - use of the `ix_v2_job_root_by_path` index; hence the `parent_job IS NULL`
                     //   clause.
                     // - select from `v2_job` first, then join with `v2_job_queue` to avoid a full
                     //   table scan on `running = true`.
                     "SELECT id
                     FROM v2_job j JOIN v2_job_queue USING (id)
                     WHERE j.workspace_id = $2 AND trigger_kind = 'schedule' AND trigger = $1 AND runnable_path = $4
                         AND parent_job IS NULL
                         AND j.id != $3
                         AND running = true",
                     schedule_path.as_ref().unwrap(),
                     flow_job.workspace_id.as_str(),
                     flow_job.id,
                     flow_job.runnable_path()
                 )
                 .fetch_all(db)
                 .warn_after_seconds(3)
                 .await?;
                if overlapping.len() > 0 {
                    let overlapping_str = overlapping
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>()
                        .join(", ");

                    return Ok(PushNextFlowJob::Done(Some(
                        UpdateFlow {
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
                        }
                    )));
                }
            }
        }
        if let Some(skip_expr) = &flow.skip_expr {
            let skip = compute_bool_from_expr(
                &skip_expr,
                arc_flow_job_args.clone(),
                Arc::new(to_raw_value(&json!("{}"))),
                None,
                None,
                Some(client),
                None,
                Some(vec![(
                    windmill_common::variables::WM_SCHEDULED_FOR.to_string(),
                    flow_job.scheduled_for.to_string(),
                )]),
            )
            .warn_after_seconds(3)
            .await?;
            if skip {
                return Ok(PushNextFlowJob::Done(Some(UpdateFlow {
                    flow: flow_job.id,
                    success: true,
                    result: serde_json::from_str("\"stopped early\"").unwrap(),
                    stop_early_override: Some(true),
                    w_id: flow_job.workspace_id.clone(),
                    worker_dir: worker_dir.to_string(),
                    token: client.token.clone(),
                })));
            }
        }
    }

    // Compute and initialize last_job_result
    let arc_last_job_result = if status_module.is_failure() {
        // if job is being retried, pass the result of its previous failure
        last_job_result.unwrap_or_else(|| Arc::new(to_raw_value(&json!("{}"))))
    } else if matches!(step, Step::Step(0)) || matches!(step, Step::PreprocessorStep) {
        // if it's the first job executed in the flow, pass the flow args
        Arc::new(to_raw_value(&flow_job.args))
    } else {
        // else pass the last job result. Either from the function arg if it's set, or manually fetch it from the previous job
        // having last_job_result empty can happen either when the job was suspended and is being restarted, or if it's a
        // flow restart from a specific step
        if last_job_result.is_some() {
            last_job_result.unwrap()
        } else {
            match get_previous_job_result(db, flow_job.workspace_id.as_str(), &status)
                .warn_after_seconds(3)
                .await?
            {
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
            let mut tx = db.begin().warn_after_seconds(3).await?;

            /* Lock this row to prevent the suspend column getting out out of sync
             * if a resume message arrives after we fetch and count them here.
             *
             * This only works because jobs::resume_job does the same thing. */
            sqlx::query_scalar!(
                "SELECT null FROM v2_job_queue WHERE id = $1 FOR UPDATE",
                flow_job.id
            )
            .fetch_one(&mut *tx)
            .warn_after_seconds(3)
            .await
            .context("lock flow in queue")?;

            let resumes = sqlx::query_as::<_, ResumeRow>(
                 "SELECT value, approver, resume_id, approved FROM resume_job WHERE job = $1 ORDER BY created_at ASC",
             )
             .bind(last)
             .fetch_all(&mut *tx)
             .warn_after_seconds(3)
             .await
             ?
             .into_iter()
             .collect::<Vec<_>>();

            resume_messages.extend(resumes.iter().map(|r| to_raw_value(&r.value)));
            approvers.extend(resumes.iter().map(|r| {
                r.approver
                    .clone()
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
                                     None
                                 )
                                 .warn_after_seconds(3)
                                 .await
                                 .map_err(|e| {
                                     Error::ExecutionErr(format!(
                                         "Error during isolated evaluation of expression `{expr}`:\n{e:#}"
                                     ))
                                 })?
                                 .get(),
                             );
                            if eval_result.is_ok() {
                                user_groups_required = eval_result.ok().unwrap_or(Vec::new())
                            } else {
                                let e = eval_result.err().unwrap();
                                return Err(Error::ExecutionErr(format!(
                                    "Result returned by input transform invalid `{e:#}`"
                                )));
                            }
                        }
                    }
                }
                let approval_conditions = ApprovalConditions {
                    user_auth_required,
                    user_groups_required,
                    self_approval_disabled,
                };
                sqlx::query!(
                    "UPDATE v2_job_status
                     SET flow_status = JSONB_SET(flow_status, ARRAY['approval_conditions'], $1)
                     WHERE id = $2",
                    json!(approval_conditions),
                    flow_job.id
                )
                .execute(&mut *tx)
                .warn_after_seconds(3)
                .await?;
            }

            let is_disapproved = resumes.iter().find(|x| !x.approved);
            let can_be_resumed =
                is_disapproved.is_none() && resume_messages.len() >= required_events as usize;
            let disapproved_or_timeout_but_continue = !can_be_resumed
                && (is_disapproved.is_some()
                    || !matches!(
                        &status_module,
                        FlowStatusModule::WaitingForPriorSteps { .. }
                    ))
                && suspend.continue_on_disapprove_timeout.unwrap_or(false);

            let audit_author = AuditAuthor {
                username: flow_job
                    .permissioned_as
                    .trim_start_matches("u/")
                    .to_string(),
                email: flow_job.permissioned_as_email.clone(),
                username_override: None,
                token_prefix: Some(format!("psh.nxt.flowjob-{}", client.token.to_string())),
            };

            if can_be_resumed || disapproved_or_timeout_but_continue {
                if disapproved_or_timeout_but_continue {
                    let js = if let Some(disapproved) = is_disapproved.as_ref() {
                        json!({"error": {"message": format!("Disapproved by {}", disapproved.approver.clone().unwrap_or_else( || "unknown".to_string())), "name": "SuspendedDisapproved"}})
                    } else {
                        json!({"error": {"message": "Timed out waiting to be resumed", "name": "SuspendedTimedOut"}})
                    };

                    resume_messages.push(to_raw_value(&js));
                    audit_log(
                         &mut *tx,
                         &audit_author,
                         "jobs.suspend_resume",
                         ActionKind::Update,
                         &flow_job.workspace_id,
                         Some(&serde_json::json!({"approved": false, "job_id": flow_job.id, "details": "Suspend timed out without approval but can continue".to_string()}).to_string()),
                         None,
                     )
                     .warn_after_seconds(3)
                     .await?;
                }

                sqlx::query!(
                     "UPDATE v2_job_status
                     SET flow_status = JSONB_SET(flow_status, ARRAY['modules', $1::TEXT, 'approvers'], $2)
                     WHERE id = $3",
                     (status.step - 1).to_string(),
                     json!(resumes
                         .into_iter()
                         .map(|r| Approval {
                             resume_id: r.resume_id as u16,
                             approver: r
                                 .approver.clone()
                                 .unwrap_or_else(|| "unknown".to_string())
                         })
                         .collect::<Vec<_>>()
                     ),
                     flow_job.id
                 )
                 .execute(&mut *tx)
                 .warn_after_seconds(3)
                 .await?;

                // Remove the approval conditions from the flow status
                sqlx::query!(
                    "UPDATE v2_job_status
                     SET flow_status = flow_status - 'approval_conditions'
                     WHERE id = $1",
                    flow_job.id
                )
                .execute(&mut *tx)
                .warn_after_seconds(3)
                .await?;

                /* continue on and run this job! */
                tx.commit().warn_after_seconds(3).await?;

            /* not enough messages to do this job, "park"/suspend until there are */
            } else if matches!(
                &status_module,
                FlowStatusModule::WaitingForPriorSteps { .. }
            ) && is_disapproved.is_none()
            {
                sqlx::query!(
                    "WITH suspend AS (
                         UPDATE v2_job_queue SET suspend = $2, suspend_until = now() + $3
                         WHERE id = $4
                         RETURNING id
                     ) UPDATE v2_job_status SET flow_status = JSONB_SET(
                         flow_status,
                         ARRAY['modules', flow_status->>'step'::TEXT],
                         $1
                     ) WHERE id = (SELECT id FROM suspend)",
                    json!(FlowStatusModule::WaitingForEvents {
                        id: status_module.id(),
                        count: required_events,
                        job: last
                    }),
                    (required_events - resume_messages.len() as u16) as i32,
                    Duration::from_secs(
                        suspend.timeout.map(|t| t.into()).unwrap_or_else(|| 30 * 60)
                    ) as Duration,
                    flow_job.id,
                )
                .execute(&mut *tx)
                .warn_after_seconds(3)
                .await?;

                sqlx::query!(
                    "UPDATE v2_job_runtime SET ping = NULL
                     WHERE id = $1",
                    flow_job.id,
                )
                .execute(&mut *tx)
                .warn_after_seconds(3)
                .await?;

                tx.commit().warn_after_seconds(3).await?;
                return Ok(PushNextFlowJob::Done(None));

            /* cancelled or we're WaitingForEvents but we don't have enough messages (timed out) */
            } else {
                if is_disapproved.is_none() {
                    audit_log(
                         &mut *tx,
                         &audit_author,
                         "jobs.suspend_resume",
                         ActionKind::Update,
                         &flow_job.workspace_id,
                         Some(&serde_json::json!({"approved": false, "job_id": flow_job.id, "details": "Suspend timed out without approval and is cancelled".to_string()}).to_string()),
                         None,
                     )
                     .warn_after_seconds(3)
                     .await?;
                }
                tx.commit().warn_after_seconds(3).await?;

                let (logs, error_name) = if let Some(disapprover) = is_disapproved {
                    (
                        format!(
                            "Disapproved by {}",
                            disapprover
                                .approver
                                .clone()
                                .unwrap_or_else(|| "unknown".to_string())
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

                append_logs(
                    &flow_job.id,
                    &flow_job.workspace_id,
                    logs.clone(),
                    &db.into(),
                )
                .warn_after_seconds(3)
                .await;

                return Ok(PushNextFlowJob::Done(Some(UpdateFlow {
                    flow: flow_job.id,
                    success: false,
                    result: to_raw_value(&result),
                    stop_early_override: None,
                    w_id: flow_job.workspace_id.clone(),
                    worker_dir: worker_dir.to_string(),
                    token: client.token.clone(),
                })));
            }
        }
    }

    let mut module = match step {
        Step::Step(i) => flow
            .modules
            .get(i)
            .with_context(|| format!("no module at index {}", i))?,
        Step::PreprocessorStep => flow
            .preprocessor_module
            .as_ref()
            .with_context(|| format!("no preprocessor module"))?,
        Step::FailureStep => flow
            .failure_module
            .as_deref()
            .with_context(|| format!("no failure module"))?,
    };

    let current_id = &module.id;
    let mut previous_id = match step {
        Step::Step(i) if i >= 1 => flow.modules.get(i - 1).map(|m| m.id.clone()).unwrap(),
        _ => String::new(),
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
            let sleep_input_transform = if let Step::Step(i) = step {
                i.checked_sub(1)
                    .and_then(|i| flow.modules.get(i))
                    .and_then(|m| m.sleep.clone())
            } else {
                None
            };

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
                                 None,
                             )
                             .warn_after_seconds(3)
                             .await
                             .map_err(|e| {
                                 Error::ExecutionErr(format!(
                                     "Error during isolated evaluation of expression `{expr}`:\n{e:#}"
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
                sqlx::query!(
                     "UPDATE v2_job_status
                     SET flow_status = JSONB_SET(JSONB_SET(flow_status, ARRAY['retry'], $1), ARRAY['modules', $3::TEXT, 'failed_retries'], $4)
                     WHERE id = $2",
                     json!(RetryStatus { fail_count, ..status.retry.clone() }),
                     flow_job.id,
                     status.step.to_string(),
                     json!(status.retry.failed_jobs)
                 )
                 .execute(db)
                 .warn_after_seconds(2)
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
                step = Step::FailureStep;
                previous_id = current_id.clone();
                module = flow
                    .failure_module
                    .as_ref()
                    /* If this fails, it's a update_flow_status_after_job_completion shouldn't have called
                     * handle_flow to get here. */
                    .context("missing failure module")?;
                status_module = status.failure_module.module_status.clone();

                if module.retry.as_ref().is_some_and(|x| x.has_attempts()) {
                    sqlx::query!(
                        "UPDATE v2_job_status
                         SET flow_status = JSONB_SET(flow_status, ARRAY['retry'], $1)
                         WHERE id = $2",
                        json!(RetryStatus { fail_count: 0, failed_jobs: vec![] }),
                        flow_job.id
                    )
                    .execute(db)
                    .warn_after_seconds(3)
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

    let is_skipped = if let Some(skip_if) = &module.skip_if {
        let idcontext = get_transform_context(&flow_job, previous_id.as_str(), &status)
            .warn_after_seconds(3)
            .await?;
        compute_bool_from_expr(
            &skip_if.expr,
            arc_flow_job_args.clone(),
            arc_last_job_result.clone(),
            None,
            Some(&idcontext),
            Some(client),
            Some((resumes.clone(), resume.clone(), approvers.clone())),
            None,
        )
        .warn_after_seconds(3)
        .await?
    } else {
        false
    };

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
        Ok(Marc::new(hm))
    } else if let Some(id) = get_args_from_id {
        let args = sqlx::query_scalar!(
            "SELECT args AS \"args: Json<HashMap<String, Box<RawValue>>>\"
                 FROM v2_job WHERE id = $1 AND workspace_id = $2",
            id,
            &flow_job.workspace_id
        )
        .fetch_optional(db)
        .warn_after_seconds(3)
        .await?;
        if let Some(args) = args {
            Ok(Marc::new(args.map(|x| x.0).unwrap_or_else(HashMap::new)))
        } else {
            Ok(Marc::new(HashMap::new()))
        }
    } else if matches!(step, Step::PreprocessorStep) {
        let mut hm = (*arc_flow_job_args).clone();
        hm.insert(
            ENTRYPOINT_OVERRIDE.to_string(),
            to_raw_value(&"preprocessor"),
        );
        Ok(Marc::new(hm))
    } else {
        let value = module.get_value();
        match &value {
            Ok(_) if matches!(value, Ok(FlowModuleValue::Identity)) || is_skipped => {
                serde_json::from_str(
                    &serde_json::to_string(&PreviousResult {
                        previous_result: Some(&arc_last_job_result),
                    })
                    .unwrap(),
                )
                .map(Marc::new)
                .map_err(|e| error::Error::internal_err(format!("identity: {e:#}")))
            }
            Ok(
                FlowModuleValue::Script { input_transforms, .. }
                | FlowModuleValue::RawScript { input_transforms, .. }
                | FlowModuleValue::FlowScript { input_transforms, .. }
                | FlowModuleValue::Flow { input_transforms, .. },
            ) => {
                let ctx = get_transform_context(&flow_job, &previous_id, &status)
                    .warn_after_seconds(3)
                    .await?;
                transform_context = Some(ctx);
                let by_id = transform_context.as_ref().unwrap();
                // if a failure step, we add flow job id and started_at to the context. This is for error handling of triggers where we wrap scripts into single step flows
                // It's to make its arguments consistent with global/workspace/schedule error handlers.
                let failure_context = match step {
                    Step::FailureStep if flow_job.started_at.is_some() => Some(FailureContext {
                        started_at: Arc::new(to_raw_value(&flow_job.started_at.unwrap())),
                        flow_job_id: flow_job.id,
                    }),
                    _ => None,
                };
                transform_input(
                    arc_flow_job_args.clone(),
                    arc_last_job_result.clone(),
                    input_transforms,
                    resumes.clone(),
                    resume.clone(),
                    approvers.clone(),
                    failure_context,
                    by_id,
                    client,
                )
                .warn_after_seconds(3)
                .await
                .map(Marc::new)
            }
            Ok(_) => Ok(arc_flow_job_args.clone()),
            Err(e) => {
                return Err(error::Error::internal_err(format!(
                    "module was not convertible to acceptable value {e:?}"
                )))
            }
        }
    };
    tracing::debug!(id = %flow_job.id, root_id = %job_root, "flow job args computed");

    let next_flow_transform = compute_next_flow_transform(
        arc_flow_job_args.clone(),
        arc_last_job_result.clone(),
        &flow_job,
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
        is_skipped,
    )
    .warn_after_seconds(3)
    .await?;
    tracing::info!(id = %flow_job.id, root_id = %job_root, "next flow transform computed");

    let (job_payloads, next_status) = match next_flow_transform {
        NextFlowTransform::Continue(job_payload, next_state) => (job_payload, next_state),
        NextFlowTransform::EmptyInnerFlows { branch_chosen } => {
            let raw_status = sqlx::query_scalar!(
                "UPDATE v2_job_status
                 SET flow_status = JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2)
                 WHERE id = $3
                 RETURNING flow_status AS \"flow_status: Json<Box<RawValue>>\"",
                status.step.to_string(),
                json!(FlowStatusModule::Success {
                    id: status_module.id(),
                    job: Uuid::nil(),
                    flow_jobs: Some(vec![]),
                    flow_jobs_success: Some(vec![]),
                    branch_chosen: branch_chosen,
                    approvers: vec![],
                    failed_retries: vec![],
                    skipped: false,
                }),
                flow_job.id
            )
            .fetch_optional(db)
            .warn_after_seconds(3)
            .await?
            .flatten();

            let status = raw_status
                .as_ref()
                .and_then(|v| serde_json::from_str::<FlowStatus>((**v).get()).ok());

            if let Some(status) = status {
                // // flow is reprocessed by the worker in a state where the module has completed successfully.
                return Ok(PushNextFlowJob::Rec(PushNextFlowJobRec {
                    flow_job: flow_job,
                    status: status,
                }));
            } else {
                return Err(Error::BadRequest(
                    "impossible to parse new flow status after applying inner flows".to_string(),
                ));
            }
        }
    };

    // Also check `flow_job.same_worker` for [`JobKind::Flow`] jobs as it's no
    // more reflected to the flow value on push.
    let job_same_worker = flow_job.same_worker
        && matches!(flow_job.kind, JobKind::Flow)
        && flow_job.runnable_id.is_some();
    let continue_on_same_worker =
        (flow.same_worker || job_same_worker) && module.suspend.is_none() && module.sleep.is_none();

    /* Finally, push the job into the queue */
    let mut uuids = vec![];

    let job_payloads = match job_payloads {
        ContinuePayload::SingleJob(payload) => vec![payload],
        ContinuePayload::ParallelJobs(payloads) => payloads,
    };
    let len = job_payloads.len();

    let mut tx = db.begin().warn_after_seconds(3).await?;
    let nargs = args.as_ref();
    for (i, payload_tag) in job_payloads.into_iter().enumerate() {
        if i % 100 == 0 && i != 0 {
            tracing::info!(id = %flow_job.id, root_id = %job_root, "pushed (non-commited yet) first {i} subflows of {len}");
            sqlx::query!(
                "UPDATE v2_job_runtime SET ping = now() WHERE id = $1 AND ping < now()",
                flow_job.id,
            )
            .execute(db)
            .warn_after_seconds(3)
            .await?;
        }
        tracing::debug!(id = %flow_job.id, root_id = %job_root, "pushing job {i} of {len}");

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

        let marc;
        let me;
        let args = match &next_status {
            NextStatus::AllFlowJobs {
                branchall: Some(BranchAllStatus { .. }),
                iterator: None,
                ..
            } => nargs,
            NextStatus::NextLoopIteration {
                next: ForloopNextIteration { new_args, .. },
                simple_input_transforms,
            } => {
                let mut args = if let Ok(args) = nargs {
                    args.as_ref().clone()
                } else {
                    HashMap::new()
                };
                insert_iter_arg(&mut args, "iter".to_string(), to_raw_value(new_args));

                args.insert("iter".to_string(), to_raw_value(new_args));
                if let Some(input_transforms) = simple_input_transforms {
                    //previous id is none because we do not want to use previous id if we are in a for loop
                    let ctx = get_transform_context(&flow_job, "", &status)
                        .warn_after_seconds(3)
                        .await?;
                    let ti = transform_input(
                        Marc::new(args),
                        arc_last_job_result.clone(),
                        input_transforms,
                        resumes.clone(),
                        resume.clone(),
                        approvers.clone(),
                        None,
                        &ctx,
                        client,
                    )
                    .warn_after_seconds(3)
                    .await
                    .map_err(|e| {
                        Error::ExecutionErr(
                            format!("could not transform input using an expr: {e}",),
                        )
                    })
                    .map(Marc::new);
                    if let Ok(ti) = ti {
                        marc = ti;
                        Ok(&marc)
                    } else {
                        me = ti.unwrap_err();
                        Err(&me)
                    }
                } else {
                    marc = Marc::new(args);
                    Ok(&marc)
                }
            }
            NextStatus::AllFlowJobs {
                branchall: None,
                iterator: Some(FlowIterator { itered, .. }),
                simple_input_transforms,
            } => {
                if let Ok(args) = args.as_ref() {
                    let mut hm = HashMap::new();
                    for (k, v) in args.iter() {
                        hm.insert(k.to_string(), v.to_owned());
                    }
                    insert_iter_arg(
                        &mut hm,
                        "iter".to_string(),
                        to_raw_value(&json!({ "index": i as i32, "value": itered[i]})),
                    );
                    if let Some(input_transforms) = simple_input_transforms {
                        let ctx = get_transform_context(&flow_job, &previous_id, &status)
                            .warn_after_seconds(3)
                            .await?;
                        let ti = transform_input(
                            Marc::new(hm),
                            arc_last_job_result.clone(),
                            input_transforms,
                            resumes.clone(),
                            resume.clone(),
                            approvers.clone(),
                            None,
                            &ctx,
                            client,
                        )
                        .warn_after_seconds(3)
                        .await
                        .map_err(|e| {
                            Error::ExecutionErr(format!(
                                "could not transform input using an expr: {e}"
                            ))
                        })
                        .map(Marc::new);
                        if let Ok(ti) = ti {
                            marc = ti;
                            Ok(&marc)
                        } else {
                            me = ti.unwrap_err();
                            Err(&me)
                        }
                    } else {
                        marc = Marc::new(hm);
                        Ok(&marc)
                    }
                } else {
                    nargs
                }
            }
            _ => nargs,
        };

        let push_args;
        let err;
        let ov;

        match args {
            Ok(v) => {
                ov = v;
                push_args = PushArgs::from(ov.as_ref());
                err = None;
            }
            Err(e) => {
                push_args = PushArgs::from(&*EHM);
                err = Some(e);
            }
        };

        tracing::debug!(id = %flow_job.id, root_id = %job_root, "computed args for job {i} of {len}");

        let value_with_parallel = module.get_value_with_parallel()?;

        let root_job = if {
            value_with_parallel.type_ == "flow"
                || (value_with_parallel.type_ == "forloopflow"
                    && value_with_parallel.parallel.is_some_and(|x| x))
        } {
            None
        } else {
            flow_job
                .flow_innermost_root_job
                .or_else(|| Some(flow_job.id))
        };

        // forward root job permissions to the new job
        let job_perms: Option<Authed> = {
            if let Some(root_job) = &flow_job
                .flow_innermost_root_job
                .or_else(|| Some(flow_job.id))
            {
                sqlx::query_as!(
                     JobPerms,
                     "SELECT email, username, is_admin, is_operator, groups, folders FROM job_perms WHERE job_id = $1 AND workspace_id = $2",
                     root_job,
                     flow_job.workspace_id,
                 )
                 .fetch_optional(&mut *tx)
                 .warn_after_seconds(3)
                 .await?
                 .map(|x| x.into())
            } else {
                None
            }
        };

        tracing::debug!(id = %flow_job.id, root_id = %job_root, "computed perms for job {i} of {len}");
        let tag = if !matches!(step, Step::PreprocessorStep)
            && (flow_job.tag == "flow" || flow_job.tag == format!("flow-{}", flow_job.workspace_id))
        {
            payload_tag.tag.clone()
        } else {
            Some(flow_job.tag.clone())
        };

        let (email, permissioned_as) = if let Some(on_behalf_of) = payload_tag.on_behalf_of.as_ref()
        {
            (&on_behalf_of.email, on_behalf_of.permissioned_as.clone())
        } else {
            (
                &flow_job.permissioned_as_email,
                flow_job.permissioned_as.to_owned(),
            )
        };
        let tx2 = PushIsolationLevel::Transaction(tx);
        let (uuid, mut inner_tx) = push(
            &db,
            tx2,
            &flow_job.workspace_id,
            payload_tag.payload.clone(),
            push_args,
            &flow_job.created_by,
            email,
            permissioned_as,
            Some(&format!(
                "job-span-{}",
                flow_job.flow_innermost_root_job.unwrap_or(flow_job.id)
            )),
            scheduled_for_o,
            flow_job.schedule_path(),
            Some(flow_job.id),
            root_job,
            None,
            true,
            continue_on_same_worker,
            err,
            flow_job.visible_to_owner,
            tag,
            payload_tag.timeout,
            Some(module.id.clone()),
            new_job_priority_override,
            job_perms.as_ref(),
            None
        )
        .warn_after_seconds(2)
        .await?;

        if continue_on_same_worker {
            let _ = sqlx::query!(
                "UPDATE v2_job_queue SET worker = $2 WHERE id = $1",
                uuid,
                worker_name
            )
            .execute(&mut *inner_tx)
            .warn_after_seconds(3)
            .await;
        }

        tracing::debug!(id = %flow_job.id, root_id = %job_root, "pushed next flow job: {uuid}");

        if value_with_parallel.type_ == "forloopflow" {
            if let Some(p) = value_with_parallel.parallelism {
                tracing::debug!(id = %flow_job.id, root_id = %job_root, "updating suspend for forloopflow job {uuid}");

                if i as u16 >= p {
                    sqlx::query!(
                        "UPDATE v2_job_queue SET
                             suspend = $1,
                             suspend_until = now() + interval '14 day',
                             running = true
                         WHERE id = $2",
                        (i as u16 - p + 1) as i32,
                        uuid,
                    )
                    .execute(&mut *inner_tx)
                    .warn_after_seconds(3)
                    .await?;
                }
                tracing::debug!(id = %flow_job.id, root_id = %job_root, "updated suspend for {uuid}");
            }
        }

        if payload_tag.delete_after_use {
            let uuid_singleton_json = serde_json::to_value(&[uuid]).map_err(|e| {
                error::Error::internal_err(format!("Unable to serialize uuid: {e:#}"))
            })?;

            sqlx::query!(
                 "UPDATE v2_job_status
                 SET flow_status = JSONB_SET(flow_status, ARRAY['cleanup_module', 'flow_jobs_to_clean'], COALESCE(flow_status->'cleanup_module'->'flow_jobs_to_clean', '[]'::jsonb) || $1)
                 WHERE id = $2",
                 uuid_singleton_json,
                 root_job.unwrap_or(flow_job.id)
             )
             .execute(&mut *inner_tx)
             .warn_after_seconds(3)
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
            .execute(&mut *tx)
            .warn_after_seconds(3)
            .await?;
            tracing::debug!(id = %flow_job.id, root_id = %job_root, "updated parallel monitor lock for {uuid}");
        }
    }

    let first_uuid = uuids[0];
    let new_status = match next_status {
        NextStatus::NextLoopIteration {
            next:
                ForloopNextIteration {
                    index,
                    itered,
                    mut flow_jobs,
                    while_loop,
                    mut flow_jobs_success,
                    ..
                },
            ..
        } => {
            let uuid = one_uuid?;

            flow_jobs.push(uuid);

            if let Some(flow_jobs_success) = &mut flow_jobs_success {
                flow_jobs_success.push(None);
            }
            FlowStatusModule::InProgress {
                job: uuid,
                iterator: Some(FlowIterator { index, itered }),
                flow_jobs: Some(flow_jobs),
                flow_jobs_success,
                branch_chosen: None,
                branchall: None,
                id: status_module.id(),
                parallel: false,
                while_loop,
                progress: None,
            }
        }
        NextStatus::AllFlowJobs { iterator, branchall, .. } => FlowStatusModule::InProgress {
            job: flow_job.id,
            iterator,
            flow_jobs_success: Some(vec![None; uuids.len()]),
            flow_jobs: Some(uuids.clone()),
            branch_chosen: None,
            branchall,
            id: status_module.id(),
            parallel: true,
            while_loop: false,
            progress: None,
        },
        NextStatus::NextBranchStep(NextBranch {
            mut flow_jobs,
            status,
            mut flow_jobs_success,
            ..
        }) => {
            let uuid = one_uuid?;
            flow_jobs.push(uuid);
            if let Some(flow_jobs_success) = &mut flow_jobs_success {
                flow_jobs_success.push(None);
            }
            FlowStatusModule::InProgress {
                job: uuid,
                iterator: None,
                flow_jobs: Some(flow_jobs),
                flow_jobs_success,
                branch_chosen: None,
                branchall: Some(status),
                id: status_module.id(),
                parallel: false,
                while_loop: false,
                progress: None,
            }
        }

        NextStatus::BranchChosen(branch) => FlowStatusModule::InProgress {
            job: one_uuid?,
            iterator: None,
            flow_jobs: None,
            flow_jobs_success: None,
            branch_chosen: Some(branch),
            branchall: None,
            id: status_module.id(),
            parallel: false,
            while_loop: false,
            progress: None,
        },
        NextStatus::NextStep => {
            FlowStatusModule::WaitingForExecutor { id: status_module.id(), job: one_uuid? }
        }
    };

    tracing::debug!("STATUS STEP: {:?} {step:?} {:#?}", status.step, new_status);

    match step {
        Step::FailureStep => {
            sqlx::query!(
                "UPDATE v2_job_status SET
                     flow_status = JSONB_SET(
                         JSONB_SET(flow_status, ARRAY['failure_module'], $1),
                         ARRAY['step'],
                         $2
                     )
                 WHERE id = $3",
                json!(FlowStatusModuleWParent {
                    parent_module: Some(current_id.clone()),
                    module_status: new_status
                }),
                json!(flow.modules.len()),
                flow_job.id
            )
            .execute(&mut *tx)
            .warn_after_seconds(3)
            .await?;
        }
        Step::PreprocessorStep => {
            sqlx::query!(
                "UPDATE v2_job_status SET
                     flow_status = JSONB_SET(
                         JSONB_SET(flow_status, ARRAY['preprocessor_module'], $1),
                         ARRAY['step'],
                         $2
                     )
                 WHERE id = $3",
                json!(new_status),
                json!(-1),
                flow_job.id
            )
            .execute(&mut *tx)
            .warn_after_seconds(3)
            .await?;
        }
        Step::Step(i) => {
            sqlx::query!(
                "UPDATE v2_job_status SET
                     flow_status = JSONB_SET(
                         JSONB_SET(flow_status, ARRAY['modules', $1::TEXT], $2),
                         ARRAY['step'],
                         $3
                     )
                 WHERE id = $4",
                i as i32,
                json!(new_status),
                json!(i),
                flow_job.id
            )
            .execute(&mut *tx)
            .warn_after_seconds(3)
            .await?;
        }
    };

    potentially_crash_for_testing();

    sqlx::query!(
        "UPDATE v2_job_runtime SET ping = null WHERE id = $1",
        flow_job.id
    )
    .execute(&mut *tx)
    .warn_after_seconds(3)
    .await?;

    if continue_on_same_worker {
        if !is_one_uuid {
            return Err(Error::BadRequest(
                 "Cannot continue on same worker with multiple jobs, parallel cannot be used in conjunction with same_worker".to_string(),
             ));
        }
    }
    tx.commit().warn_after_seconds(3).await?;
    tracing::info!(id = %flow_job.id, root_id = %job_root, "all next flow jobs pushed: {uuids:?}");

    if continue_on_same_worker {
        same_worker_tx
            .send(SameWorkerPayload { job_id: first_uuid, recoverable: true })
            .warn_after_seconds(3)
            .await
            .map_err(to_anyhow)?;
    }
    return Ok(PushNextFlowJob::Done(None));
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
//     .fetch_one(&mut *tx)
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
    flow_jobs_success: Option<Vec<Option<bool>>>,
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
    flow_jobs_success: Option<Vec<Option<bool>>>,
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
        iterator: Option<FlowIterator>,
        simple_input_transforms: Option<HashMap<String, InputTransform>>,
    },
}

#[derive(Clone)]
struct JobPayloadWithTag {
    payload: JobPayload,
    tag: Option<String>,
    delete_after_use: bool,
    timeout: Option<i32>,
    on_behalf_of: Option<OnBehalfOf>,
}
enum ContinuePayload {
    SingleJob(JobPayloadWithTag),
    ParallelJobs(Vec<JobPayloadWithTag>),
}

enum NextFlowTransform {
    EmptyInnerFlows { branch_chosen: Option<BranchChosen> },
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

fn payload_from_modules<'a>(
    mut modules: Vec<FlowModule>,
    modules_node: Option<FlowNodeId>,
    failure_module: Option<&Box<FlowModule>>,
    same_worker: bool,
    id: impl FnOnce() -> String,
    path: impl FnOnce() -> String,
    opt_empty_inner_flows: bool,
) -> Option<JobPayload> {
    if opt_empty_inner_flows && modules.is_empty() && modules_node.is_none() {
        return None;
    }

    if let Some(id) = modules_node {
        return Some(JobPayload::FlowNode { id, path: path() });
    }

    add_virtual_items_if_necessary(&mut modules);

    let mut failure_module = failure_module.cloned();
    if let Some(failure_module) = failure_module.as_mut() {
        failure_module.id_append(&id());
    }

    Some(JobPayload::RawFlow {
        value: FlowValue { modules, failure_module, same_worker, ..Default::default() },
        path: Some(path()),
        restarted_from: None,
    })
}

fn get_path(flow_job: &MiniPulledJob, status: &FlowStatus, module: &FlowModule) -> String {
    if status
        .preprocessor_module
        .as_ref()
        .is_some_and(|x| x.id() == module.id)
    {
        format!("{}/preprocessor", flow_job.runnable_path())
    } else {
        format!("{}/{}", flow_job.runnable_path(), module.id)
    }
}

async fn compute_next_flow_transform(
    arc_flow_job_args: Marc<HashMap<String, Box<RawValue>>>,
    arc_last_job_result: Arc<Box<RawValue>>,
    flow_job: &MiniPulledJob,
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
    is_skipped: bool,
) -> error::Result<NextFlowTransform> {
    if module.mock.is_some() && module.mock.as_ref().unwrap().enabled {
        return Ok(NextFlowTransform::Continue(
            ContinuePayload::SingleJob(JobPayloadWithTag {
                payload: JobPayload::Identity,
                tag: None,
                delete_after_use: false,
                timeout: None,
                on_behalf_of: None,
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
                on_behalf_of: None,
            }),
            NextStatus::NextStep,
        ))
    };
    let delete_after_use = module.delete_after_use.unwrap_or(false);

    tracing::debug!(id = %flow_job.id, "computing next flow transform for {:?}", &module.value);
    if is_skipped {
        return trivial_next_job(JobPayload::Identity);
    }

    match module.get_value()? {
        FlowModuleValue::Identity => trivial_next_job(JobPayload::Identity),
        FlowModuleValue::Flow { path, .. } => {
            let payload =
                flow_to_payload(path, delete_after_use, &flow_job.workspace_id, db).await?;
            Ok(NextFlowTransform::Continue(
                ContinuePayload::SingleJob(payload),
                NextStatus::NextStep,
            ))
        }
        FlowModuleValue::Script { path: script_path, hash: script_hash, tag_override, .. } => {
            let payload = script_to_payload(
                script_hash,
                script_path,
                db,
                flow_job,
                module,
                tag_override,
                module.apply_preprocessor,
            )
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
            let path = path.unwrap_or_else(|| get_path(flow_job, status, module));

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
                delete_after_use,
            );
            Ok(NextFlowTransform::Continue(
                ContinuePayload::SingleJob(payload),
                NextStatus::NextStep,
            ))
        }
        FlowModuleValue::FlowScript {
            id, // flow_node(id).
            tag,
            language,
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            ..
        } => {
            let path = get_path(flow_job, status, module);

            let payload = JobPayloadWithTag {
                payload: JobPayload::FlowScript {
                    id,
                    language,
                    custom_concurrency_key: custom_concurrency_key.clone(),
                    concurrent_limit,
                    concurrency_time_window_s,
                    cache_ttl: module.cache_ttl.map(|x| x as i32),
                    dedicated_worker: None,
                    path,
                },
                tag: tag.clone(),
                delete_after_use,
                timeout: module.timeout,
                on_behalf_of: None,
            };
            Ok(NextFlowTransform::Continue(
                ContinuePayload::SingleJob(payload),
                NextStatus::NextStep,
            ))
        }
        FlowModuleValue::WhileloopFlow { modules, modules_node, .. } => {
            // if it's a simple single step flow, we will collapse it as an optimization and need to pass flow_input as an arg
            let is_simple = is_simple_modules(&modules, flow.failure_module.as_ref());
            let (flow_jobs, flow_jobs_success) = match status_module {
                FlowStatusModule::InProgress {
                    flow_jobs: Some(flow_jobs),
                    flow_jobs_success,
                    ..
                } => (flow_jobs.clone(), flow_jobs_success.clone()),
                _ => (vec![], Some(vec![])),
            };
            let next_loop_idx = flow_jobs.len();
            next_loop_iteration(
                flow,
                status,
                ForloopNextIteration {
                    index: next_loop_idx,
                    itered: vec![],
                    flow_jobs: flow_jobs,
                    flow_jobs_success: flow_jobs_success,
                    new_args: Iter {
                        index: next_loop_idx as i32,
                        value: windmill_common::worker::to_raw_value(&next_loop_idx),
                    },
                    while_loop: true,
                },
                modules,
                modules_node,
                flow_job,
                is_simple,
                db,
                module,
                delete_after_use,
            )
            .await
        }
        /* forloop modules are expected set `iter: { value: Value, index: usize }` as job arguments */
        FlowModuleValue::ForloopFlow { modules, modules_node, iterator, parallel, .. } => {
            // if it's a simple single step flow, we will collapse it as an optimization and need to pass flow_input as an arg
            let is_simple = !matches!(flow_job.kind, JobKind::FlowPreview)
                && !parallel
                && is_simple_modules(&modules, flow.failure_module.as_ref());

            // if is_simple {
            //     match value {
            //         FlowModuleValue::Script { input_transforms, .. }
            //         | FlowModuleValue::RawScript { input_transforms, .. }
            //         | FlowModuleValue::FlowScript { input_transforms, .. }
            //         | FlowModuleValue::Flow { input_transforms, .. } => {
            //             Some(input_transforms.clone())
            //         }
            //         _ => None,
            //     }
            let next_loop_status = next_forloop_status(
                status_module,
                by_id,
                flow_job,
                previous_id,
                status,
                &iterator,
                arc_last_job_result,
                resumes,
                resume,
                approvers,
                arc_flow_job_args,
                client,
                &parallel,
            )
            .await?;

            match next_loop_status {
                ForLoopStatus::EmptyIterator => {
                    Ok(NextFlowTransform::EmptyInnerFlows { branch_chosen: None })
                }
                ForLoopStatus::NextIteration(ns) => {
                    next_loop_iteration(
                        flow,
                        status,
                        ns,
                        modules,
                        modules_node,
                        flow_job,
                        is_simple,
                        db,
                        module,
                        delete_after_use,
                    )
                    .await
                }
                ForLoopStatus::ParallelIteration { itered, .. } => {
                    // let inner_path = Some(format!("{}/loop-parallel", flow_job.script_path(),));
                    // let value = &modules[0].get_value()?;

                    // we removed the is_simple_case
                    // if is_simple {
                    //     let payload =
                    //         payload_from_simple_module(value, db, flow_job, module, inner_path)
                    //             .await?;
                    //     ContinuePayload::ForloopJobs { n: itered.len(), payload: payload }
                    // } else {

                    let payloads = (0..itered.len())
                        .into_iter()
                        .filter_map(|i| {
                            let Some(payload) = payload_from_modules(
                                modules.clone(),
                                modules_node,
                                flow.failure_module.as_ref(),
                                flow.same_worker,
                                || format!("{}-{i}", status.step),
                                || format!("{}/forloop-{i}", flow_job.runnable_path()),
                                true,
                            ) else {
                                return None;
                            };
                            Some(JobPayloadWithTag {
                                payload,
                                tag: None,
                                delete_after_use,
                                timeout: None,
                                on_behalf_of: None,
                            })
                        })
                        .collect::<Vec<_>>();
                    if payloads.is_empty() {
                        return Ok(NextFlowTransform::EmptyInnerFlows { branch_chosen: None });
                    }
                    Ok(NextFlowTransform::Continue(
                        ContinuePayload::ParallelJobs(payloads),
                        NextStatus::AllFlowJobs {
                            branchall: None,
                            iterator: Some(FlowIterator { index: 0, itered }),
                            // we removed the is_simple_case for simple_input_transforms
                            // if is_simple {
                            //     match value {
                            //         FlowModuleValue::Script { input_transforms, .. }
                            //         | FlowModuleValue::RawScript { input_transforms, .. }
                            //         | FlowModuleValue::FlowScript { input_transforms, .. }
                            //         | FlowModuleValue::Flow { input_transforms, .. } => {
                            //             Some(input_transforms.clone())
                            //         }
                            //         _ => None,
                            //     }
                            simple_input_transforms: None,
                        },
                    ))
                }
            }
        }
        FlowModuleValue::BranchOne { branches, default, default_node } => {
            let branch = match status_module {
                FlowStatusModule::WaitingForPriorSteps { .. }
                | FlowStatusModule::WaitingForEvents { .. }
                | FlowStatusModule::WaitingForExecutor { .. } => {
                    let mut branch_chosen = BranchChosen::Default;
                    let idcontext = get_transform_context(&flow_job, previous_id, &status).await?;
                    for (i, b) in branches.iter().enumerate() {
                        let pred = compute_bool_from_expr(
                            &b.expr,
                            arc_flow_job_args.clone(),
                            arc_last_job_result.clone(),
                            None,
                            Some(&idcontext),
                            Some(client),
                            Some((resumes.clone(), resume.clone(), approvers.clone())),
                            None,
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

            let (modules, modules_node, branch_idx) = match branch {
                BranchChosen::Default => (default, default_node, 0),
                BranchChosen::Branch { branch } => branches
                    .into_iter()
                    .nth(branch)
                    .map(|Branch { modules, modules_node, .. }| (modules, modules_node, branch + 1))
                    .ok_or_else(|| {
                        Error::BadRequest(format!(
                            "Unrecognized branch for BranchOne {status_module:?}"
                        ))
                    })?,
            };

            let Some(payload) = payload_from_modules(
                modules,
                modules_node,
                flow.failure_module.as_ref(),
                flow.same_worker,
                || status.step.to_string(),
                || format!("{}/branchone-{}", flow_job.runnable_path(), branch_idx),
                true,
            ) else {
                return Ok(NextFlowTransform::EmptyInnerFlows { branch_chosen: Some(branch) });
            };

            Ok(NextFlowTransform::Continue(
                ContinuePayload::SingleJob(JobPayloadWithTag {
                    payload,
                    tag: None,
                    delete_after_use,
                    timeout: None,
                    on_behalf_of: None,
                }),
                NextStatus::BranchChosen(branch),
            ))
        }
        FlowModuleValue::BranchAll { branches, parallel, .. } => {
            let (branch_status, flow_jobs, flow_jobs_success) = match status_module {
                FlowStatusModule::WaitingForPriorSteps { .. }
                | FlowStatusModule::WaitingForEvents { .. }
                | FlowStatusModule::WaitingForExecutor { .. } => {
                    if branches.is_empty() {
                        return Ok(NextFlowTransform::EmptyInnerFlows { branch_chosen: None });
                    } else if parallel {
                        let len = branches.len();
                        let payloads: Vec<JobPayloadWithTag> = branches
                            .into_iter()
                            .enumerate()
                            .filter_map(|(i, Branch { modules, modules_node, .. })| {
                                let Some(payload) = payload_from_modules(
                                    modules,
                                    modules_node,
                                    flow.failure_module.as_ref(),
                                    flow.same_worker,
                                    || format!("{}-{i}", status.step),
                                    || format!("{}/branchall-{}", flow_job.runnable_path(), i),
                                    false,
                                ) else {
                                    return None;
                                };
                                Some(JobPayloadWithTag {
                                    payload,
                                    tag: None,
                                    delete_after_use,
                                    timeout: None,
                                    on_behalf_of: None,
                                })
                            })
                            .collect::<Vec<_>>();
                        if payloads.is_empty() {
                            return Ok(NextFlowTransform::EmptyInnerFlows { branch_chosen: None });
                        }
                        return Ok(NextFlowTransform::Continue(
                            ContinuePayload::ParallelJobs(payloads),
                            NextStatus::AllFlowJobs {
                                branchall: Some(BranchAllStatus { branch: 0, len }),
                                iterator: None,
                                simple_input_transforms: None,
                            },
                        ));
                    } else {
                        (
                            BranchAllStatus { branch: 0, len: branches.len() },
                            vec![],
                            Some(vec![]),
                        )
                    }
                }
                FlowStatusModule::InProgress {
                    branchall: Some(BranchAllStatus { branch, len }),
                    flow_jobs: Some(flow_jobs),
                    flow_jobs_success,
                    ..
                } if !parallel => (
                    BranchAllStatus { branch: branch + 1, len: len.clone() },
                    flow_jobs.clone(),
                    flow_jobs_success.clone(),
                ),

                _ => Err(Error::BadRequest(format!(
                    "Unrecognized module status for BranchAll {status_module:?}"
                )))?,
            };

            let Branch { modules, modules_node, .. } = branches
                .into_iter()
                .nth(branch_status.branch)
                .ok_or_else(|| {
                    Error::BadRequest(format!(
                        "Unrecognized branch for BranchAll {status_module:?}"
                    ))
                })?;

            let Some(payload) = payload_from_modules(
                modules,
                modules_node,
                flow.failure_module.as_ref(),
                flow.same_worker,
                || format!("{}-{}", status.step, branch_status.branch),
                || {
                    format!(
                        "{}/branchall-{}",
                        flow_job.runnable_path(),
                        branch_status.branch
                    )
                },
                false,
            ) else {
                return Ok(NextFlowTransform::EmptyInnerFlows {
                    branch_chosen: Some(BranchChosen::Default),
                });
            };

            Ok(NextFlowTransform::Continue(
                ContinuePayload::SingleJob(JobPayloadWithTag {
                    payload,
                    tag: None,
                    delete_after_use,
                    timeout: None,
                    on_behalf_of: None,
                }),
                NextStatus::NextBranchStep(NextBranch {
                    status: branch_status,
                    flow_jobs,
                    flow_jobs_success,
                }),
            ))
        }
    }
}

async fn next_loop_iteration(
    flow: &FlowValue,
    status: &FlowStatus,
    ns: ForloopNextIteration,
    modules: Vec<FlowModule>,
    modules_node: Option<FlowNodeId>,
    flow_job: &MiniPulledJob,
    is_simple: bool,
    db: &sqlx::Pool<sqlx::Postgres>,
    module: &FlowModule,
    delete_after_use: bool,
) -> Result<NextFlowTransform, Error> {
    let inner_path = || format!("{}/loop-{}", flow_job.runnable_path(), ns.index);
    if is_simple {
        let mut value = modules[0].get_value()?;
        let simple_input_transforms = match &mut value {
            FlowModuleValue::Script { input_transforms, .. }
            | FlowModuleValue::RawScript { input_transforms, .. }
            | FlowModuleValue::FlowScript { input_transforms, .. }
            | FlowModuleValue::Flow { input_transforms, .. } => {
                Some(std::mem::take(input_transforms))
            }
            _ => None,
        };
        return Ok(NextFlowTransform::Continue(
            ContinuePayload::SingleJob(
                payload_from_simple_module(value, db, flow_job, module, inner_path()).await?,
            ),
            NextStatus::NextLoopIteration { next: ns, simple_input_transforms },
        ));
    }

    let Some(payload) = payload_from_modules(
        modules,
        modules_node,
        flow.failure_module.as_ref(),
        flow.same_worker,
        || format!("{}-{}", status.step, ns.index),
        inner_path,
        true,
    ) else {
        return Ok(NextFlowTransform::EmptyInnerFlows { branch_chosen: None });
    };

    Ok(NextFlowTransform::Continue(
        ContinuePayload::SingleJob(JobPayloadWithTag {
            payload,
            tag: None,
            delete_after_use,
            timeout: None,
            on_behalf_of: None,
        }),
        NextStatus::NextLoopIteration { next: ns, simple_input_transforms: None },
    ))
}

pub(super) fn is_simple_modules(
    modules: &Vec<FlowModule>,
    failure_module: Option<&Box<FlowModule>>,
) -> bool {
    let is_simple = modules.len() == 1
        && modules[0].is_simple()
        && modules[0].sleep.is_none()
        && modules[0].suspend.is_none()
        && modules[0].cache_ttl.is_none()
        && modules[0].retry.is_none()
        && modules[0].stop_after_if.is_none()
        && modules[0].stop_after_all_iters_if.is_none()
        && modules[0].skip_if.is_none()
        && (modules[0].mock.is_none() || modules[0].mock.as_ref().is_some_and(|m| !m.enabled))
        && failure_module.is_none();
    is_simple
}

async fn next_forloop_status(
    status_module: &FlowStatusModule,
    by_id: Option<IdContext>,
    flow_job: &MiniPulledJob,
    previous_id: &str,
    status: &FlowStatus,
    iterator: &InputTransform,
    arc_last_job_result: Arc<Box<RawValue>>,
    resumes: Arc<Box<RawValue>>,
    resume: Arc<Box<RawValue>>,
    approvers: Arc<Box<RawValue>>,
    arc_flow_job_args: Marc<HashMap<String, Box<RawValue>>>,
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
                        Some(&by_id),
                        None,
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
                    flow_jobs_success: Some(vec![]),
                    new_args: iter,
                    while_loop: false,
                })
            } else {
                panic!("itered cannot be empty")
            }
        }

        FlowStatusModule::InProgress {
            iterator: Some(FlowIterator { itered, index }),
            flow_jobs: Some(flow_jobs),
            flow_jobs_success,
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
                            Some(&by_id),
                            None,
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
                flow_jobs_success: flow_jobs_success.clone(),
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
    value: FlowModuleValue,
    db: &sqlx::Pool<sqlx::Postgres>,
    flow_job: &MiniPulledJob,
    module: &FlowModule,
    inner_path: String,
) -> Result<JobPayloadWithTag, Error> {
    let delete_after_use = module.delete_after_use.unwrap_or(false);
    Ok(match value {
        FlowModuleValue::Flow { path, .. } => {
            flow_to_payload(path, delete_after_use, &flow_job.workspace_id, db).await?
        }
        FlowModuleValue::Script { path: script_path, hash: script_hash, tag_override, .. } => {
            script_to_payload(
                script_hash,
                script_path,
                db,
                flow_job,
                module,
                tag_override,
                module.apply_preprocessor,
            )
            .await?
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
            path.unwrap_or_else(|| inner_path),
            content,
            language,
            lock,
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            module,
            tag,
            delete_after_use,
        ),
        FlowModuleValue::FlowScript {
            id, // flow_node(id).
            tag,
            language,
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            ..
        } => JobPayloadWithTag {
            payload: JobPayload::FlowScript {
                id,
                language,
                custom_concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                cache_ttl: module.cache_ttl.map(|x| x as i32),
                dedicated_worker: None,
                path: inner_path,
            },
            tag,
            delete_after_use,
            timeout: module.timeout,
            on_behalf_of: None,
        },
        _ => unreachable!("is simple flow"),
    })
}

fn raw_script_to_payload(
    path: String,
    content: String,
    language: windmill_common::scripts::ScriptLang,
    lock: Option<String>,
    custom_concurrency_key: Option<String>,
    concurrent_limit: Option<i32>,
    concurrency_time_window_s: Option<i32>,
    module: &FlowModule,
    tag: Option<String>,
    delete_after_use: bool,
) -> JobPayloadWithTag {
    JobPayloadWithTag {
        payload: JobPayload::Code(RawCode {
            hash: None,
            path: Some(path),
            content,
            language,
            lock,
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl: module.cache_ttl.map(|x| x as i32),
            dedicated_worker: None,
        }),
        tag,
        delete_after_use,
        timeout: module.timeout,
        on_behalf_of: None,
    }
}

async fn flow_to_payload(
    path: String,
    delete_after_use: bool,
    w_id: &str,
    db: &DB,
) -> Result<JobPayloadWithTag, Error> {
    let FlowVersionInfo { version, on_behalf_of_email, edited_by, tag, .. } =
        get_latest_flow_version_info_for_path(db, w_id, &path, true).await?;
    let on_behalf_of = if let Some(email) = on_behalf_of_email {
        Some(OnBehalfOf { email, permissioned_as: username_to_permissioned_as(&edited_by) })
    } else {
        None
    };
    let payload =
        JobPayload::Flow { path, dedicated_worker: None, apply_preprocessor: false, version };
    Ok(JobPayloadWithTag { payload, tag, delete_after_use, timeout: None, on_behalf_of })
}

async fn script_to_payload(
    script_hash: Option<windmill_common::scripts::ScriptHash>,
    script_path: String,
    db: &sqlx::Pool<sqlx::Postgres>,
    flow_job: &MiniPulledJob,
    module: &FlowModule,
    tag_override: Option<String>,
    apply_preprocessor: Option<bool>,
) -> Result<JobPayloadWithTag, Error> {
    let tag_override = if tag_override.as_ref().is_some_and(|x| x.trim().is_empty()) {
        None
    } else {
        tag_override
    };
    let (payload, tag, delete_after_use, script_timeout, on_behalf_of) = if script_hash.is_none() {
        let (jp, tag, delete_after_use, script_timeout, on_behalf_of) =
            script_path_to_payload(&script_path, db, &flow_job.workspace_id, Some(true)).await?;
        (
            jp,
            tag_override.to_owned().or(tag),
            delete_after_use,
            script_timeout,
            on_behalf_of,
        )
    } else {
        let hash = script_hash.unwrap();
        let mut tx: sqlx::Transaction<'_, sqlx::Postgres> = db.begin().await?;
        let ScriptHashInfo {
            tag,
            concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            language,
            dedicated_worker,
            priority,
            delete_after_use,
            timeout,
            on_behalf_of_email,
            created_by,
            ..
        } = get_script_info_for_hash(&mut *tx, &flow_job.workspace_id, hash.0).await?;
        let on_behalf_of = if let Some(email) = on_behalf_of_email {
            Some(OnBehalfOf { email, permissioned_as: username_to_permissioned_as(&created_by) })
        } else {
            None
        };
        (
            // We only apply the preprocessor if it's explicitly set to true in the module,
            // which can only happen if the the flow is a SingleScriptFlow triggered by a trigger with retries or error handling.
            // In that case, apply_preprocessor is still only set to true if the script has a preprocesor.
            // We only check for script hash because SingleScriptFlow triggers specifies the script hash
            JobPayload::ScriptHash {
                hash,
                path: script_path,
                custom_concurrency_key: concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                cache_ttl: module.cache_ttl.map(|x| x as i32).ok_or(cache_ttl).ok(),
                language,
                dedicated_worker,
                priority,
                apply_preprocessor: apply_preprocessor.unwrap_or(false),
            },
            tag_override.to_owned().or(tag),
            delete_after_use,
            timeout,
            on_behalf_of,
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
        on_behalf_of,
    })
}

async fn get_transform_context(
    flow_job: &MiniPulledJob,
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
            sqlx::query_scalar!(
                "SELECT result AS \"result!: Json<Box<RawValue>>\"
                 FROM v2_job_completed WHERE id = $1 AND workspace_id = $2",
                job,
                w_id
            )
            .fetch_one(db)
            .await?
            .0,
        )),
        _ => Ok(None),
    }
}
