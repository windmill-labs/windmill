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

use crate::common::{cached_result_path, get_root_job_id, save_in_cache};
use crate::js_eval::{eval_timeout, IdContext};
use crate::worker_utils::get_tag_and_concurrency;
use crate::{
    JobCompletedSender, PreviousResult, SameWorkerSender, SendResultPayload, UpdateFlow,
    KEEP_JOB_DIR,
};

use anyhow::Context;
use async_once_cell::Lazy;
use backon::{BackoffBuilder, ConstantBuilder, Retryable};
use futures::TryFutureExt;
use mappable_rc::Marc;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::{json, Value};
use sqlx::types::Json;
use sqlx::{FromRow, Postgres, Transaction};
use tracing::instrument;
use uuid::Uuid;
use windmill_common::auth::get_job_perms;
#[cfg(feature = "benchmark")]
use windmill_common::bench::BenchmarkIter;
use windmill_common::cache::{self, RawData};
use windmill_common::client::AuthedClient;
use windmill_common::db::Authed;
use windmill_common::flow_conversations::{add_message_to_conversation_tx, MessageType};
use windmill_common::flow_status::{
    ApprovalConditions, FlowJobDuration, FlowJobsDuration, FlowStatusModuleWParent,
    Iterator as FlowIterator, JobResult,
};
use windmill_common::flows::{add_virtual_items_if_necessary, Branch, FlowNodeId, StopAfterIf};
use windmill_common::jobs::{
    check_tag_available_for_workspace_internal, script_path_to_payload, JobKind, JobPayload,
    OnBehalfOf, RawCode, ENTRYPOINT_OVERRIDE,
};
use windmill_common::runnable_settings::{ConcurrencySettingsWithCustom, DebouncingSettings};
use windmill_common::scripts::{ScriptHash, ScriptRunnableSettingsInline};
use windmill_common::users::username_to_permissioned_as;
use windmill_common::utils::WarnAfterExt;
use windmill_common::worker::to_raw_value;
use windmill_common::{
    add_time, get_latest_flow_version_info_for_path, get_script_info_for_hash, FlowVersionInfo,
    ScriptHashInfo, DB,
};
use windmill_common::{
    error::{self, to_anyhow, Error},
    flow_status::{
        Approval, BranchAllStatus, BranchChosen, FlowStatus, FlowStatusModule, RetryStatus,
        MAX_RETRY_ATTEMPTS, MAX_RETRY_INTERVAL,
    },
    flows::{FlowModule, FlowModuleValue, FlowValue, InputTransform, Retry, Step, Suspend},
    min_version::MIN_VERSION_IS_AT_LEAST_1_595,
};
use windmill_queue::schedule::get_schedule_opt;
use windmill_queue::{
    add_completed_job, add_completed_job_error, append_logs, get_mini_pulled_job,
    insert_concurrency_key, interpolate_args,
    report_error_to_workspace_handler_or_critical_side_channel, try_schedule_next_job, CanceledBy,
    FlowRunners, MiniCompletedJob, MiniPulledJob, PushArgs, PushIsolationLevel, SameWorkerPayload,
    WrappedError,
};

use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::audit::AuditAuthor;
use windmill_queue::{canceled_job_to_result, push};

#[derive(Debug)]
pub struct SchedulePushZombieError(pub String);

impl std::fmt::Display for SchedulePushZombieError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SchedulePushZombieError {}

/// Helper function to write itered data to separate table
/// Returns None if data was written to separate table, Some(itered) if it should be stored in JSONB
async fn write_itered_to_db(
    db: &DB,
    job_id: Uuid,
    itered: &Vec<Box<RawValue>>,
) -> error::Result<Option<Vec<Box<RawValue>>>> {
    if MIN_VERSION_IS_AT_LEAST_1_595.met().await {
        // Write to separate table
        sqlx::query!(
            "INSERT INTO flow_iterator_data (job_id, itered) VALUES ($1, $2)
             ON CONFLICT (job_id) DO UPDATE SET itered = $2",
            job_id,
            Json(itered) as Json<&Vec<Box<RawValue>>>,
        )
        .execute(db)
        .await?;
        // Return None to indicate itered should not be stored in JSONB
        Ok(None)
    } else {
        // Return Some(itered) to indicate it should be stored in JSONB for backwards compatibility
        Ok(Some(itered.clone()))
    }
}

/// Helper function to read itered data from separate table
/// Falls back to reading from JSONB flow_status if not found in separate table or version too old
async fn read_itered_from_db(
    db: &DB,
    job_id: Uuid,
    itered_from_status: &Option<Vec<Box<RawValue>>>,
) -> error::Result<Vec<Box<RawValue>>> {
    // Only try to read from separate table if version supports it
    if MIN_VERSION_IS_AT_LEAST_1_595.met().await {
        let result = sqlx::query_scalar!(
            "SELECT itered as \"itered: Json<Vec<Box<RawValue>>>\" FROM flow_iterator_data WHERE job_id = $1",
            job_id,
        )
        .fetch_optional(db)
        .await?;

        if let Some(Json(itered)) = result {
            // Found in separate table
            return Ok(itered);
        }
    }

    // Fall back to reading from JSONB flow_status (backwards compatibility or not found in table)
    Ok(itered_from_status.clone().unwrap_or_default()) // can be none for restarted flows, in which case we return an empty vector
}

// #[instrument(level = "trace", skip_all)]
pub async fn update_flow_status_after_job_completion(
    db: &DB,
    client: &AuthedClient,
    flow: uuid::Uuid,
    job_id_for_status: &Uuid,
    w_id: &str,
    success: bool,
    canceled_by: Option<CanceledBy>,
    result: Arc<Box<RawValue>>,
    flow_job_duration: Option<FlowJobDuration>,
    unrecoverable: bool,
    same_worker_tx: &SameWorkerSender,
    worker_dir: &str,
    stop_early_override: Option<bool>,
    worker_name: &str,
    job_completed_tx: JobCompletedSender,
    flow_runners: Option<Arc<FlowRunners>>,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    #[cfg(feature = "benchmark")] bench: &mut BenchmarkIter,
) -> error::Result<Option<Arc<MiniPulledJob>>> {
    // this is manual tailrecursion because async_recursion blows up the stack
    potentially_crash_for_testing();

    let mut rec = RecUpdateFlowStatusAfterJobCompletion {
        flow,
        job_id_for_status: job_id_for_status.clone(),
        success,
        canceled_by,
        result,
        flow_job_duration,
        stop_early_override,
        has_triggered_error_handler: false,
    };
    let mut unrecoverable = unrecoverable;
    loop {
        potentially_crash_for_testing();
        let nrec = match Box::pin(update_flow_status_after_job_completion_internal(
            db,
            client,
            rec.flow,
            &rec.job_id_for_status,
            w_id,
            rec.success,
            rec.canceled_by,
            rec.flow_job_duration.clone(),
            rec.result,
            unrecoverable,
            same_worker_tx,
            worker_dir,
            rec.stop_early_override,
            rec.has_triggered_error_handler,
            worker_name,
            job_completed_tx.clone(),
            flow_runners.clone(),
            killpill_rx,
            #[cfg(feature = "benchmark")]
            bench,
        ))
        .await
        {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("Error while updating flow status of {} after  completion of {}, updating flow status again with error: {e:#}", rec.flow, &rec.job_id_for_status);
                Box::pin(update_flow_status_after_job_completion_internal(
                    db,
                    client,
                    rec.flow,
                    &rec.job_id_for_status,
                    w_id,
                    false,
                    None,
                    rec.flow_job_duration,
                    Arc::new(to_raw_value(&Json(&WrappedError {
                        error: json!(e.to_string()),
                    }))),
                    true,
                    same_worker_tx,
                    worker_dir,
                    rec.stop_early_override,
                    rec.has_triggered_error_handler,
                    worker_name,
                    job_completed_tx.clone(),
                    flow_runners.clone(),
                    killpill_rx,
                    #[cfg(feature = "benchmark")]
                    bench,
                ))
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
    canceled_by: Option<CanceledBy>,
    flow_job_duration: Option<FlowJobDuration>,
    stop_early_override: Option<bool>,
    has_triggered_error_handler: bool,
}

#[derive(Deserialize)]
struct RecoveryObject {
    recover: Option<bool>,
}

fn get_stop_after_if_data(stop_after_if: Option<&StopAfterIf>) -> (bool, Option<String>) {
    if let Some(stop_after_if) = stop_after_if {
        let err_msg = stop_after_if.error_message.as_ref().and_then(|message| {
            let s = if message.is_empty() {
                format!("stop after if: {}", stop_after_if.expr)
            } else {
                message.clone()
            };
            Some(s)
        });
        return (stop_after_if.skip_if_stopped, err_msg);
    }
    return (false, None);
}

async fn get_id_ctx_for_expr(
    expr: &str,
    flow: uuid::Uuid,
    db: &DB,
    status: &FlowStatus,
) -> error::Result<Option<IdContext>> {
    if expr.contains("results.") || expr.contains("results[") || expr.contains("results?.") {
        let flow_job = get_mini_pulled_job(db, &flow).await?;
        if let Some(flow_job) = flow_job {
            Ok(Some(get_transform_context(&flow_job, "", &status)))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

async fn evaluate_stop_after_all_iters_if(
    db: &DB,
    stop_after_all_iters_if: &StopAfterIf,
    module_status: &FlowStatusModule,
    w_id: &str,
    client: &AuthedClient,
    stop_early: &mut bool,
    skip_if_stop_early: &mut bool,
    stop_early_err_msg: &mut Option<String>,
    nresult: &mut Option<Arc<Box<RawValue>>>,
    args: HashMap<String, Box<RawValue>>,
    flow: uuid::Uuid,
    status: &FlowStatus,
) -> error::Result<()> {
    let iters_result = match &module_status {
        FlowStatusModule::InProgress { flow_jobs: Some(flow_jobs), .. } => {
            Arc::new(retrieve_flow_jobs_results(db, w_id, flow_jobs).await?)
        }
        _ => {
            return Err(Error::internal_err(format!(
                "A branchall or loop should have flow_jobs"
            )))
        }
    };

    *nresult = Some(iters_result.clone()); // as an optimization, we store the result of all jobs as when stop_early_after_all_iters evaluates to false, it would have to be computed (finished loop/branchall)

    let id_ctx = get_id_ctx_for_expr(&stop_after_all_iters_if.expr, flow, db, status).await?;

    let stop_early_after_all_iters = compute_bool_from_expr(
        &stop_after_all_iters_if.expr,
        Marc::new(args),
        None,
        iters_result.clone(),
        None,
        id_ctx.as_ref(),
        Some(client),
        None,
        None,
    )
    .await?;

    if stop_early_after_all_iters {
        *stop_early = true;
        (*skip_if_stop_early, *stop_early_err_msg) =
            get_stop_after_if_data(Some(stop_after_all_iters_if));
    }
    Ok(())
}

fn result_has_recover_true(nresult: Arc<Box<RawValue>>) -> bool {
    let recover = serde_json::from_str::<RecoveryObject>(nresult.get());
    return recover.map(|r| r.recover.unwrap_or(false)).unwrap_or(false);
}

// #[instrument(level = "trace", skip_all)]
pub async fn update_flow_status_after_job_completion_internal(
    db: &DB,
    client: &AuthedClient,
    flow: uuid::Uuid,
    job_id_for_status: &Uuid,
    w_id: &str,
    mut success: bool,
    canceled_by: Option<CanceledBy>,
    mut flow_job_duration: Option<FlowJobDuration>,
    result: Arc<Box<RawValue>>,
    unrecoverable: bool,
    same_worker_tx: &SameWorkerSender,
    worker_dir: &str,
    stop_early_override: Option<bool>,
    has_triggered_error_handler: bool,
    worker_name: &str,
    job_completed_tx: JobCompletedSender,
    flow_runners: Option<Arc<FlowRunners>>,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    #[cfg(feature = "benchmark")] bench: &mut BenchmarkIter,
) -> error::Result<UpdateFlowStatusAfterJobCompletion> {
    let mut has_triggered_error_handler = has_triggered_error_handler;
    add_time!(bench, "update flow status internal START");
    struct ChatAiInfo {
        chat_input_enabled: bool,
        conversation_id: Option<Uuid>,
        is_ai_agent_step: bool,
    }
    let (
        should_continue_flow,
        flow_job,
        flow_data,
        stop_early,
        skip_if_stop_early,
        nresult,
        is_failure_step,
        _cleanup_module,
        chat_ai_info,
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

        let flow_data = cache::job::fetch_flow(db, &job_kind, script_hash)
            .or_else(|_| cache::job::fetch_preview_flow(db, &flow, raw_flow))
            .await?;
        let flow_value = flow_data.value();

        let module_step = Step::from_i32_and_len(old_status.step, old_status.modules.len());
        let current_module = match module_step {
            Step::Step { idx: i, .. } => flow_value.modules.get(i),
            _ => None,
        };

        if current_module.is_some_and(|x| x.is_flow()) {
            has_triggered_error_handler = false;
        }

        let module_status = match module_step {
            Step::PreprocessorStep => old_status
                .preprocessor_module
                .as_ref()
                .ok_or_else(|| Error::internal_err(format!("preprocessor module not found")))?,
            Step::FailureStep => &old_status.failure_module.module_status,
            Step::Step { idx: i, .. } => old_status
                .modules
                .get(i as usize)
                .ok_or_else(|| Error::internal_err(format!("module {i} not found")))?,
        };

        // tracing::debug!(
        //     "UPDATE FLOW STATUS 2: {module_step:#?} {module_status:#?} {old_status:#?} "
        // );

        let (is_loop, skip_loop_failures, parallelism, parallel_loop) =
            if let FlowStatusModule::InProgress { iterator: Some(_), parallel, .. } = module_status
            {
                let value = current_module
                    .as_ref()
                    .and_then(|x| x.get_value_with_skip_failures().ok());
                (
                    true,
                    value
                        .as_ref()
                        .and_then(|x| x.skip_failures)
                        .unwrap_or(false),
                    value.and_then(|x| x.parallelism),
                    *parallel,
                )
            } else {
                (false, false, None, false)
            };

        let (is_branch_all, parallel_branchall) = match module_status {
            FlowStatusModule::InProgress { branchall: Some(_), parallel, .. } => (true, *parallel),
            _ => (false, false),
        };

        // 0 length flows are not failure steps
        let is_failure_step =
            old_status.step >= old_status.modules.len() as i32 && old_status.modules.len() > 0;

        let is_flow_stop_early_override = stop_early_override.is_some() && {
            let step = module_step.get_step_index();

            if let Some(_) = step {
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
            }
        };

        let args = Arc::pin(Lazy::new(async move {
            let args = sqlx::query_scalar!(
                r#"
            SELECT args AS "args: Json<HashMap<String, Box<RawValue>>>"
            FROM v2_job
            WHERE id = $1
            "#,
                flow
            )
            .fetch_one(db)
            .await;
            let args =
                args.map(|flow_args| flow_args.map(|flow_args| flow_args.0).unwrap_or_default());

            args
        }));

        let from_result_to_args = |args: &Result<HashMap<String, Box<RawValue>>, sqlx::Error>| {
            let args = args
                .as_ref()
                .map_err(|e| Error::internal_err(format!("retrieval of args from state: {e:#}")))?;

            Ok::<_, Error>(args.clone())
        };

        let (mut stop_early, mut stop_early_err_msg, mut skip_if_stop_early, continue_on_error) =
            if stop_early_override.is_some()
                && !is_flow_stop_early_override
                && !parallel_loop
                && !parallel_branchall
            {
                // we ignore stop_early_override (stop_early in children) if module is parallel or is a flow step
                let se = stop_early_override.as_ref().unwrap();
                (true, None, *se, false)
            } else if is_failure_step || module_step.is_preprocessor_step() {
                (false, None, false, false)
            } else if let Some(current_module) = current_module {
                let stop_early = success
                    && !is_branch_all // we don't support stop_early per branch
                    && !parallel_loop // we don't support anymore stop_early per iteration when parallel for loop (removed from frontend)
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
                        let args = from_result_to_args(args.as_ref().await.get_ref())?;

                        let id_ctx = get_id_ctx_for_expr(expr, flow, db, &old_status).await?;

                        compute_bool_from_expr(
                            &expr,
                            Marc::new(args),
                            None,
                            result.clone(),
                            all_iters,
                            id_ctx.as_ref(),
                            Some(client),
                            None,
                            None,
                        )
                        .await?
                    } else {
                        false
                    };
                let (skip_if_stopped, stop_early_err_msg) = if stop_early {
                    get_stop_after_if_data(current_module.stop_after_if.as_ref())
                } else {
                    (false, None)
                };

                (
                    stop_early,
                    stop_early_err_msg,
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

        let mut nresult = None;
        let (inc_step_counter, new_status) = match module_status {
            FlowStatusModule::InProgress {
                iterator,
                branchall,
                parallel,
                flow_jobs: Some(jobs),
                flow_jobs_success,
                flow_jobs_duration,
                ..
            } if *parallel => {
                let (nindex, len) = match (iterator, branchall) {
                    (Some(FlowIterator { itered, itered_len, .. }), _) => {
                        let position = if flow_jobs_success.is_some() {
                            find_flow_job_index(jobs, job_id_for_status)
                        } else {
                            None
                        };

                        let nindex = if let Some(position) = position {
                             sqlx::query_scalar!(
                                 "
                                 UPDATE v2_job_status SET flow_status = 
                                    JSONB_SET(JSONB_SET(JSONB_SET(JSONB_SET(
                                            flow_status, 
                                            ARRAY['modules', $1::TEXT, 'flow_jobs_success', $3::TEXT], $4), 
                                            ARRAY['modules', $1::TEXT, 'iterator', 'index'], ((flow_status->'modules'->$1::int->'iterator'->>'index')::int + 1)::text::jsonb),
                                            ARRAY['modules', $1::TEXT, 'flow_jobs_duration', 'started_at', $3::TEXT], $5),
                                            ARRAY['modules', $1::TEXT, 'flow_jobs_duration', 'duration_ms', $3::TEXT], $6)
                                 WHERE id = $2
                                 RETURNING (flow_status->'modules'->$1::int->'iterator'->>'index')::int",
                                 old_status.step,
                                 flow,
                                 position as i32,
                                 json!(success),
                                 flow_job_duration.as_ref().map(|x| json!(x.started_at)).unwrap_or_default(),
                                 flow_job_duration.as_ref().map(|x| json!(x.duration_ms)).unwrap_or_default(),
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
                         })?.ok_or_else(|| Error::internal_err(format!("requiring an index in InProgress for flow {flow} at step {}", old_status.step)))?;

                        // let status_for_debug = sqlx::query!(
                        //     "SELECT flow_status FROM v2_job_status WHERE id = $1",
                        //     flow
                        // )
                        // .fetch_one(&mut *tx)
                        // .await
                        // .map_err(|e| {
                        //     Error::internal_err(format!("error while fetching flow status: {e:#}"))
                        // })?;

                        // tracing::error!("status_for_debug: {:?}", status_for_debug.flow_status);

                        let itered_len = if let Some(itered_len) = itered_len {
                            *itered_len
                        } else {
                            itered.as_ref().map(|itered| itered.len()).unwrap_or(0)
                        };

                        tracing::info!(
                             "parallel iteration {job_id_for_status} of flow {flow} update nindex: {nindex} len: {len}",
                             nindex = nindex,
                             len = itered_len
                         );
                        (nindex, itered_len as i32)
                    }
                    (_, Some(BranchAllStatus { len, .. })) => {
                        let position = if flow_jobs_success.is_some() {
                            find_flow_job_index(jobs, job_id_for_status)
                        } else {
                            None
                        };

                        let nindex = if let Some(position) = position {
                             sqlx::query_scalar!(
                                 "UPDATE v2_job_status SET flow_status = 
                                 CASE 
                                 WHEN flow_status->'modules'->$1::int->'flow_jobs_duration' IS NOT NULL THEN
                                    JSONB_SET(
                                         JSONB_SET(JSONB_SET(JSONB_SET(
                                            flow_status, 
                                            ARRAY['modules', $1::TEXT, 'flow_jobs_success', $3::TEXT], $4), 
                                            ARRAY['modules', $1::TEXT, 'flow_jobs_duration', 'started_at', $3::TEXT], $5), 
                                            ARRAY['modules', $1::TEXT, 'flow_jobs_duration', 'duration_ms', $3::TEXT], $6),
                                            ARRAY['modules', $1::TEXT, 'branchall', 'branch'],
                                            ((flow_status->'modules'->$1::int->'branchall'->>'branch')::int + 1)::text::jsonb
                                     )
                                 ELSE
                                    JSONB_SET(JSONB_SET(
                                    flow_status, ARRAY['modules', $1::TEXT, 'flow_jobs_success', $3::TEXT], $4),
                                         ARRAY['modules', $1::TEXT, 'branchall', 'branch'],
                                         ((flow_status->'modules'->$1::int->'branchall'->>'branch')::int + 1)::text::jsonb
                                     )
                                 END
                                 WHERE id = $2
                                 RETURNING (flow_status->'modules'->$1::int->'branchall'->>'branch')::int",
                                 old_status.step,
                                 flow,
                                 position as i32,
                                 json!(success),
                                 flow_job_duration.as_ref().map(|x| json!(x.started_at)).unwrap_or_default(),
                                 flow_job_duration.as_ref().map(|x| json!(x.duration_ms)).unwrap_or_default(),
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
                    let success_and_durations =  match sqlx::query!(
                        "SELECT flow_status->'modules'->$2::int->'flow_jobs_success' as \"flow_jobs_success: Json<Vec<Option<bool>>>\", flow_status->'modules'->$2::int->'flow_jobs_duration' as \"flow_jobs_duration: Json<FlowJobsDuration>\"
                        FROM v2_job_status WHERE id = $1",
                        flow,
                        old_status.step
                    )
                    .fetch_one(&mut *tx)
                    .await {
                        Err(e) => {
                            tracing::error!("error while fetching success and durations: {e:#}");
                            (flow_jobs_success.clone(), flow_jobs_duration.clone())
                        }
                        Ok(x) => (x.flow_jobs_success.map(|x| x.0), x.flow_jobs_duration.map(|x| x.0)),
                    };

                    let mut flow_jobs_success = success_and_durations.0;
                    if let Some(flow_job_success) = flow_jobs_success.as_mut() {
                        let position = jobs.iter().position(|x| x == job_id_for_status);
                        if let Some(position) = position {
                            if position < flow_job_success.len() {
                                flow_job_success[position] = Some(success);
                            }
                        }
                    }
                    let mut flow_jobs_duration = success_and_durations.1;
                    if let Some(flow_jobs_duration) = flow_jobs_duration.as_mut() {
                        let position = jobs.iter().position(|x| x == job_id_for_status);
                        flow_jobs_duration.set(position, &flow_job_duration);
                    }

                    let branches = current_module
                        .and_then(|x| x.get_branches_skip_failures().ok())
                        .map(|x| {
                            x.branches
                                .iter()
                                .map(|b| b.skip_failure.unwrap_or(false))
                                .collect::<Vec<_>>()
                        });

                    let mut njobs = Vec::new();

                    let jobs_filtered = if branchall.is_some() {
                        if let Some(branches) = branches {
                            for (branch, job) in branches.iter().zip(jobs.iter()) {
                                if !branch {
                                    njobs.push(job.clone());
                                }
                            }
                            njobs.as_slice()
                        } else {
                            jobs.as_slice()
                        }
                    } else {
                        jobs.as_slice()
                    };

                    // evaluate stop_after_all_iters_if for parallel loops/branchall
                    if let Some(stop_after_all_iters_if) = current_module
                        .as_ref()
                        .and_then(|x| x.stop_after_all_iters_if.as_ref())
                    {
                        let args = from_result_to_args(args.as_ref().await.get_ref())?;
                        evaluate_stop_after_all_iters_if(
                            db,
                            stop_after_all_iters_if,
                            module_status,
                            w_id,
                            client,
                            &mut stop_early,
                            &mut skip_if_stop_early,
                            &mut stop_early_err_msg,
                            &mut nresult,
                            args,
                            flow,
                            &old_status,
                        )
                        .await?;
                    }

                    let new_status = if
                        !(stop_early && stop_early_err_msg.is_some() && !skip_if_stop_early) // if stop_early with error and NOT skip_if_stopped, mark as failure
                        && (
                                skip_loop_failures
                                || sqlx::query_scalar!(
                                    "SELECT status = 'success' OR status = 'skipped' AS \"success!\" FROM v2_job_completed WHERE id = ANY($1)",
                                    jobs_filtered
                                )
                                .fetch_all(&mut *tx)
                                .await
                                .map_err(|e| {
                                    Error::internal_err(format!(
                                        "error while fetching success from completed_jobs: {e:#}"
                                    ))
                                })?
                                .into_iter()
                                .all(|x| x)
                            )
                     {
                         success = true;
                         FlowStatusModule::Success {
                             id: module_status.id(),
                             job: job_id_for_status.clone(),
                             flow_jobs: Some(jobs.clone()),
                             flow_jobs_success: flow_jobs_success.clone(),
                             flow_jobs_duration: flow_jobs_duration.clone(),
                             branch_chosen: None,
                             approvers: vec![],
                             failed_retries: vec![],
                             skipped: stop_early && skip_if_stop_early,
                             agent_actions: None,
                             agent_actions_success: None,
                         }
                     } else {
                         success = false;
                         FlowStatusModule::Failure {
                             id: module_status.id(),
                             job: job_id_for_status.clone(),
                             flow_jobs: Some(jobs.clone()),
                             flow_jobs_success: flow_jobs_success.clone(),
                             flow_jobs_duration: flow_jobs_duration.clone(),
                             branch_chosen: None,
                             failed_retries: vec![],
                             agent_actions: None,
                             agent_actions_success: None,
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

                    // for parallel branchall and forloop, we do not want to trigger the error handler again at the forloop/branchall node since it was already triggered at the leaf level
                    // so we want to ignore the has_triggered_error_handler flag at the forloop/branchall node and reset it based on if the node is a success or failure
                    if !success && flow_value.failure_module.is_some() {
                        has_triggered_error_handler = true;
                    } else {
                        has_triggered_error_handler = false;
                    }
                    (success, Some(new_status))
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
                iterator: Some(FlowIterator { index, itered_len, itered }),
                flow_jobs_success,
                flow_jobs,
                while_loop,
                ..
            } if {
                let itered_len = if let Some(itered_len) = itered_len {
                    *itered_len
                } else {
                    // backwards compatibility
                    itered.as_ref().map(|itered| itered.len()).unwrap_or(0)
                };
                (*while_loop || (*index + 1 < itered_len) && (success || skip_loop_failures))
                    && !stop_early
            } =>
            {
                if let Some(jobs) = flow_jobs {
                    set_success_and_duration_in_flow_job_success(
                        flow_jobs_success,
                        jobs,
                        job_id_for_status,
                        old_status.step,
                        flow,
                        success,
                        flow_job_duration.clone(),
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
            } if branch.to_owned() < len - 1
                && (success || skip_seq_branch_failure)
                && !stop_early =>
            {
                if let Some(jobs) = flow_jobs {
                    set_success_and_duration_in_flow_job_success(
                        flow_jobs_success,
                        jobs,
                        job_id_for_status,
                        old_status.step,
                        flow,
                        success,
                        flow_job_duration.clone(),
                        &mut tx,
                    )
                    .await?;
                }
                (false, None)
            }
            _ => {
                // this case is when when not a parallel loops/branchall and not an in progress loop/branchall

                if stop_early && is_loop {
                    // if we're stopping early inside a (non-parallel) loop, we don't want to bubble up the stop_early to the parent => we only want to break the loop (see conditions in match above)
                    stop_early = false;
                    stop_early_err_msg = None;
                    skip_if_stop_early = false;
                }

                if is_loop || (is_branch_all && !stop_early) {
                    // when we finish a loop or branchall, we only want to evaluate stop_after_all_iters_if if:
                    //  -  we're in a loop (non-parallel)
                    //  -  we're in a branchall and it wasn't stopped early from inside (non-parallel branchall stopped inside => stop flow => no need to evaluate stop_after_all_iters_if)
                    if let Some(stop_after_all_iters_if) = current_module
                        .as_ref()
                        .and_then(|x| x.stop_after_all_iters_if.as_ref())
                    {
                        let args = from_result_to_args(args.as_ref().await.get_ref())?;

                        evaluate_stop_after_all_iters_if(
                            db,
                            stop_after_all_iters_if,
                            module_status,
                            w_id,
                            client,
                            &mut stop_early,
                            &mut skip_if_stop_early,
                            &mut stop_early_err_msg,
                            &mut nresult,
                            args,
                            flow,
                            &old_status,
                        )
                        .await?;
                    }
                }

                let flow_jobs = module_status.flow_jobs();
                let branch_chosen = module_status.branch_chosen();
                let mut flow_jobs_success = module_status.flow_jobs_success();
                let mut flow_jobs_duration = module_status.flow_jobs_duration();

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

                if let (Some(flow_jobs_duration), Some(flow_jobs)) =
                    (flow_jobs_duration.as_mut(), flow_jobs.as_ref())
                {
                    let position = flow_jobs.iter().position(|x| x == job_id_for_status);
                    flow_jobs_duration.set(position, &flow_job_duration);
                }

                // if stop_early with error message and NOT skip_if_stopped, mark as failure
                // if skip_if_stopped=true, we want to mark as success (skipped), not failure
                if (success
                    || (flow_jobs.is_some() && (skip_loop_failures || skip_seq_branch_failure)))
                    && !(stop_early && stop_early_err_msg.is_some() && !skip_if_stop_early)
                {
                    let is_skipped = (stop_early && skip_if_stop_early)
                        || if current_module.as_ref().is_some_and(|m| m.skip_if.is_some()) {
                            sqlx::query_scalar!(
                                "SELECT kind = 'identity' FROM v2_job WHERE id = $1",
                                job_id_for_status
                            )
                            .fetch_optional(db)
                            .await
                            .map_err(|e| {
                                Error::internal_err(format!("error during skip check: {e:#}"))
                            })?
                            .flatten()
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
                            flow_jobs_duration,
                            branch_chosen,
                            approvers: vec![],
                            failed_retries: old_status.retry.failed_jobs.clone(),
                            skipped: is_skipped,
                            agent_actions: module_status.agent_actions(),
                            agent_actions_success: module_status.agent_actions_success(),
                        }),
                    )
                } else {
                    let inc = if continue_on_error {
                        let retry = current_module
                            .as_ref()
                            .and_then(|x| x.retry.clone())
                            .unwrap_or_default();

                        tracing::info!("update flow status on retry: {retry:#?} ");
                        let args = from_result_to_args(args.as_ref().await.get_ref())?;

                        evaluate_retry(
                            &retry,
                            &old_status.retry,
                            result.clone(),
                            Marc::new(args),
                            None,
                            Some(client),
                        )
                        .await?
                        .is_none()
                    } else {
                        false
                    };
                    if stop_early && stop_early_err_msg.is_some() {
                        // if stop_early with error message, we need to set explictely success to false as this function was called with success = true as the job succeeded
                        success = false;
                    }
                    (
                        inc,
                        Some(FlowStatusModule::Failure {
                            id: module_status.id(),
                            job: job_id_for_status.clone(),
                            flow_jobs,
                            flow_jobs_success,
                            flow_jobs_duration,
                            branch_chosen,
                            failed_retries: old_status.retry.failed_jobs.clone(),
                            agent_actions: module_status.agent_actions(),
                            agent_actions_success: module_status.agent_actions_success(),
                        }),
                    )
                }
            }
        };

        if stop_early && stop_early_err_msg.is_some() {
            nresult = Some(Arc::new(to_raw_value(&serde_json::json! ({
                "error": {
                    "name": "EarlyStopError",
                    "message": stop_early_err_msg.as_ref().unwrap(),
                }
            }))));
        }

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
            } else if module_step.is_preprocessor_step() {
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

        let nresult = if let Some(nresult) = nresult {
            // can be some either with early stop error or with the flow jobs results (was fetched to evaluate stop_early_after_all_iters but evaluated to false)
            nresult
        } else {
            match &new_status {
                Some(FlowStatusModule::Success { flow_jobs: Some(jobs), .. })
                | Some(FlowStatusModule::Failure { flow_jobs: Some(jobs), .. }) => {
                    Arc::new(retrieve_flow_jobs_results(db, w_id, jobs).await?)
                }
                _ => result.clone(),
            }
        };

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

        if module_step.is_preprocessor_step() && success {
            let tag_and_concurrency_key = get_tag_and_concurrency(&flow, db).await;
            let has_debouncing = flow_value
                .debouncing_settings
                .debounce_delay_s
                .filter(|x| *x > 0)
                .is_some();
            let concurrency_requires_args = tag_and_concurrency_key.as_ref().is_some_and(|x| {
                x.tag.as_ref().is_some_and(|t| t.contains("$args"))
                    || x.concurrency_key
                        .as_ref()
                        .is_some_and(|ck| ck.contains("$args"))
            });
            let require_args = concurrency_requires_args || has_debouncing;
            let mut tag = tag_and_concurrency_key.as_ref().and_then(|x| x.tag.clone());
            let concurrency_key = tag_and_concurrency_key
                .as_ref()
                .and_then(|x| x.concurrency_key.clone());
            let concurrent_limit = tag_and_concurrency_key
                .as_ref()
                .and_then(|x| x.concurrent_limit);
            let concurrency_time_window_s = tag_and_concurrency_key
                .as_ref()
                .and_then(|x| x.concurrency_time_window_s);

            let fetched_args = if require_args {
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
                Some(args.unwrap_or_default().0)
            } else {
                None
            };

            if concurrency_requires_args {
                let args = PushArgs::from(fetched_args.as_ref().unwrap());
                if let Some(ck) = concurrency_key {
                    insert_concurrency_key(
                        &flow_job.workspace_id,
                        &args,
                        &flow_job.runnable_path,
                        JobKind::Flow,
                        Some(ck),
                        db,
                        flow,
                    )
                    .await?;
                }
                if let Some(t) = tag {
                    tag = Some(interpolate_args(t, &args, &flow_job.workspace_id));
                }
            } else if concurrent_limit.is_some() {
                insert_concurrency_key(
                    &flow_job.workspace_id,
                    &PushArgs::from(&HashMap::new()),
                    &flow_job.runnable_path,
                    JobKind::Flow,
                    concurrency_key,
                    db,
                    flow,
                )
                .await?;
            }

            let scheduled_for: Option<chrono::DateTime<chrono::Utc>> = {
                #[cfg(feature = "private")]
                {
                    if has_debouncing {
                        let empty_hm = HashMap::new();
                        let args = PushArgs::from(fetched_args.as_ref().unwrap_or(&empty_hm));
                        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                            &flow_value.debouncing_settings,
                            &flow_job.runnable_path,
                            &flow_job.workspace_id,
                            flow,
                            &args,
                            db,
                        )
                        .await?
                    } else {
                        None
                    }
                }
                #[cfg(not(feature = "private"))]
                {
                    None
                }
            };

            sqlx::query!(
                "WITH job_result AS (
                 SELECT result
                 FROM v2_job_completed
                 WHERE id = $1
             ),
             updated_queue AS (
                UPDATE v2_job_queue
                SET running = false,
                tag = COALESCE($3, tag),
                scheduled_for = COALESCE($6, scheduled_for)
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
                scheduled_for,
            )
            .execute(db)
            .await
            .map_err(|e| {
                Error::internal_err(format!(
                    "error while updating args in preprocessing step: {e:#}"
                ))
            })?;
            return Ok(UpdateFlowStatusAfterJobCompletion::PreprocessingStep);
        }

        let job_root = flow_job
            .flow_innermost_root_job
            .map(|x| x.to_string())
            .unwrap_or_else(|| "none".to_string());

        let should_retry = async move || -> error::Result<bool> {
            let default_retry = Retry::default();
            let retry_config = flow_value
                .get_flow_module_at_step(module_step)
                .ok()
                .and_then(|flow_module| flow_module.retry.as_ref())
                .unwrap_or(&default_retry);
            let args = from_result_to_args(args.as_ref().await.get_ref())?;

            let should_retry = evaluate_retry(
                retry_config,
                &old_status.retry,
                result.clone(),
                Marc::new(args),
                None,
                Some(client),
            )
            .await?
            .is_some();

            Ok(should_retry)
        };

        let should_continue_flow = match success {
            _ if stop_early => stop_early_err_msg.is_some() && flow_value.failure_module.is_some(), // if stop_early_err_msg some, we want to trigger the error handler before stopping the flow, if any
            _ if flow_job.is_canceled() => false,
            true => !is_last_step,
            false if unrecoverable => false,
            false if skip_seq_branch_failure || skip_loop_failures || continue_on_error => {
                !is_last_step
            }
            false if should_retry().await? => true,
            false
                if !is_failure_step
                    && !has_triggered_error_handler
                    && flow_value.failure_module.is_some() =>
            {
                true
            }
            false => false,
        };

        tracing::info!(id = %flow_job.id, root_id = %job_root, success = %success, stop_early = %stop_early, is_last_step = %is_last_step, unrecoverable = %unrecoverable,
             skip_seq_branch_failure = %skip_seq_branch_failure, skip_loop_failures = %skip_loop_failures,
             current_module_id = %current_module.map(|x| x.id.clone()).unwrap_or_default(),
            continue_on_error = %continue_on_error, should_continue_flow = %should_continue_flow, "computed if flow should continue");

        let chat_ai_info = ChatAiInfo {
            chat_input_enabled: old_status.chat_input_enabled.unwrap_or(false),
            conversation_id: old_status.memory_id,
            is_ai_agent_step: current_module.is_some_and(|m| m.is_ai_agent()),
        };
        (
            should_continue_flow,
            flow_job,
            flow_data,
            stop_early,
            skip_if_stop_early,
            nresult,
            is_failure_step,
            old_status.cleanup_module,
            chat_ai_info,
        )
    };

    let flow_job = Arc::new(flow_job);

    let done = if !should_continue_flow {
        {
            let logs = if flow_job.is_canceled() {
                "Flow job canceled\n".to_string()
            } else if stop_early {
                format!("Flow job stopped early because of a stop early predicate returning true\n")
            } else if is_failure_step {
                format!("Flow job completed with error, and error handler was triggered.\nIt completed with {}, and with recover: {}\n", if success { "success" } else { "error" }, result_has_recover_true(nresult.clone()))
            } else {
                format!(
                    "Flow job completed with {}\n",
                    if success { "success" } else { "error" }
                )
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
            let canceled_by = CanceledBy {
                username: flow_job.canceled_by.clone(),
                reason: flow_job.canceled_reason.clone(),
            };
            let error = canceled_job_to_result(&flow_job);
            add_completed_job_error(
                db,
                &MiniCompletedJob::from(flow_job.clone()),
                0,
                Some(canceled_by),
                error,
                worker_name,
                true,
                None,
            )
            .await?;
        } else {
            let cflow_job: MiniCompletedJob = MiniCompletedJob::from(flow_job.clone());

            if flow_job.cache_ttl.is_some() && success {
                let flow = RawData::Flow(flow_data.clone());
                let cached_res_path = cached_result_path(db, client, &flow_job, Some(&flow)).await;

                save_in_cache(
                    db,
                    client,
                    &MiniCompletedJob::from(cflow_job.clone()),
                    cached_res_path,
                    nresult.clone(),
                )
                .await;
            }

            let success = success && (!is_failure_step || result_has_recover_true(nresult.clone()));

            add_time!(bench, "flow status update 1");

            let skipped = stop_early && skip_if_stop_early;
            add_tool_message_to_conversation(
                db,
                &job_id_for_status,
                success,
                skipped,
                chat_ai_info.is_ai_agent_step,
                &nresult,
                chat_ai_info.chat_input_enabled,
                chat_ai_info.conversation_id,
            )
            .await?;
            let duration = if success {
                let (_, duration) = add_completed_job(
                    db,
                    &cflow_job,
                    true,
                    skipped,
                    Json(&nresult),
                    None,
                    0,
                    None,
                    true,
                    None,
                    false,
                )
                .await?;
                duration
            } else {
                let (_, duration) = add_completed_job(
                    db,
                    &cflow_job,
                    false,
                    skipped,
                    Json(
                        &serde_json::from_str::<Value>(nresult.get()).unwrap_or_else(
                            |e| json!({"error": format!("Impossible to serialize error: {e:#}")}),
                        ),
                    ),
                    None,
                    0,
                    canceled_by.clone(),
                    true,
                    None,
                    false,
                )
                .await?;
                duration
            };
            flow_job_duration = flow_job
                .started_at
                .map(|x| FlowJobDuration { started_at: x, duration_ms: duration });
        }
        true
    } else {
        tracing::debug!(id = %flow_job.id,  "start handle flow");
        match Box::pin(handle_flow(
            flow_job.clone(),
            &flow_data,
            db,
            client,
            Some(nresult.clone()),
            same_worker_tx,
            worker_dir,
            job_completed_tx,
            worker_name,
            flow_runners,
            &killpill_rx,
        ))
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
                let _ = add_completed_job_error(
                    db,
                    &MiniCompletedJob::from(flow_job.clone()),
                    0,
                    None,
                    e,
                    worker_name,
                    true,
                    None,
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

        if flow_job.is_flow_step() {
            if let Some(parent_job) = flow_job.parent_job {
                tracing::info!(subflow_id = %flow_job.id, parent_id = %parent_job, "subflow is finished, updating parent flow status");

                return Ok(UpdateFlowStatusAfterJobCompletion::Rec(
                    RecUpdateFlowStatusAfterJobCompletion {
                        flow: parent_job,
                        job_id_for_status: flow,
                        success: success && !is_failure_step,
                        canceled_by: if !success { canceled_by.clone() } else { None },
                        flow_job_duration: flow_job_duration.clone(),
                        result: nresult.clone(),
                        stop_early_override: if stop_early {
                            Some(skip_if_stop_early)
                        } else {
                            None
                        },
                        has_triggered_error_handler: has_triggered_error_handler || is_failure_step,
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

async fn add_tool_message_to_conversation(
    db: &DB,
    job_id: &Uuid,
    success: bool,
    skipped: bool,
    is_ai_agent_step: bool,
    result: &Box<RawValue>,
    chat_input_enabled: bool,
    conversation_id: Option<Uuid>,
) -> error::Result<()> {
    // Create assistant message if it's a flow and it's done, but only if last module is not an AI agent
    if !skipped && chat_input_enabled {
        // Get conversation_id from flow_status.memory_id

        if let Some(conversation_id) = conversation_id {
            // Only create assistant message if last module is NOT an AI agent, or there was an error
            if !is_ai_agent_step || success == false {
                let value = serde_json::to_value(result.get())
                    .map_err(|e| Error::internal_err(format!("Failed to serialize result: {e}")))?;

                let content = match value {
                    // If it's an Object with "output" key AND the output is a String, return it
                    serde_json::Value::Object(mut map)
                        if map.contains_key("output")
                            && matches!(map.get("output"), Some(serde_json::Value::String(_))) =>
                    {
                        if let Some(serde_json::Value::String(s)) = map.remove("output") {
                            s
                        } else {
                            // prettify the whole result
                            serde_json::to_string_pretty(&map)
                                .unwrap_or_else(|e| format!("Failed to serialize result: {e}"))
                        }
                    }
                    // Otherwise, if the whole value is a String, return it
                    serde_json::Value::String(s) => s,
                    // Otherwise, prettify the whole result
                    v => serde_json::to_string_pretty(&v)
                        .unwrap_or_else(|e| format!("Failed to serialize result: {e}")),
                };

                // Insert new assistant message
                let mut tx = db.begin().await?;
                add_message_to_conversation_tx(
                    &mut tx,
                    conversation_id,
                    Some(job_id.clone()),
                    &content,
                    MessageType::Assistant,
                    None,
                    success,
                )
                .await?;
                tx.commit().await?;
            }
        }
    }

    Ok(())
}

async fn set_success_and_duration_in_flow_job_success<'c>(
    flow_jobs_success: &Option<Vec<Option<bool>>>,
    flow_jobs: &Vec<Uuid>,
    job_id_for_status: &Uuid,
    old_status_step: i32,
    flow: Uuid,
    success: bool,
    flow_job_duration: Option<FlowJobDuration>,
    tx: &mut Transaction<'c, Postgres>,
) -> error::Result<()> {
    if flow_jobs_success.is_some() {
        let position = find_flow_job_index(flow_jobs, job_id_for_status);
        if let Some(position) = position {
            sqlx::query!(
                "UPDATE v2_job_status SET flow_status = 
            CASE WHEN flow_status->'modules'->$1::int->'flow_jobs_duration' IS NOT NULL THEN
                     JSONB_SET(JSONB_SET(JSONB_SET(
                         flow_status,
                         ARRAY['modules', $1::TEXT, 'flow_jobs_success', $3::TEXT],
                         $4
                     ),
                     ARRAY['modules', $1::TEXT, 'flow_jobs_duration', 'duration_ms', $3::TEXT], $5),
                     ARRAY['modules', $1::TEXT, 'flow_jobs_duration', 'started_at', $3::TEXT], $6)
                 ELSE
                    JSONB_SET(flow_status, ARRAY['modules', $1::TEXT, 'flow_jobs_success', $3::TEXT], $4)
            END
            WHERE id = $2",
                old_status_step as i32,
                flow,
                position as i32,
                json!(success),
                flow_job_duration.as_ref().map(|x| json!(x.duration_ms)).unwrap_or_default(),
                flow_job_duration.as_ref().map(|x| json!(x.started_at)).unwrap_or_default()
            )
            .execute(&mut **tx)
            .await
            .map_err(|e| {
                Error::internal_err(format!(
                    "error while setting flow_jobs_success/timeline: {e:#}"
                ))
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

async fn evaluate_retry(
    retry: &Retry,
    status: &RetryStatus,
    result: Arc<Box<RawValue>>,
    flow_args: Marc<HashMap<String, Box<RawValue>>>,
    flow_env: Option<&HashMap<String, Box<RawValue>>>,
    client: Option<&AuthedClient>,
) -> anyhow::Result<Option<(u32, Duration)>> {
    if status.fail_count > MAX_RETRY_ATTEMPTS {
        return Ok(None);
    }

    if let Some(retry_if) = &retry.retry_if {
        let should_retry = compute_bool_from_expr(
            &retry_if.expr,
            flow_args,
            flow_env,
            result,
            None,
            None,
            client,
            None,
            None,
        )
        .await?;

        if !should_retry {
            tracing::debug!("Retry condition evaluated to false, not retrying");
            return Ok(None);
        }
    }

    Ok(retry
        .interval(status.fail_count, false)
        .map(|d| (status.fail_count + 1, std::cmp::min(d, MAX_RETRY_INTERVAL))))
}

async fn compute_bool_from_expr(
    expr: &str,
    flow_args: Marc<HashMap<String, Box<RawValue>>>,
    flow_env: Option<&HashMap<String, Box<RawValue>>>,
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
        flow_env,
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

#[instrument(level = "trace", skip_all)]
pub async fn evaluate_input_transform<T>(
    transform: &InputTransform,
    last_result: Arc<Box<RawValue>>,
    flow_args: Option<Marc<HashMap<String, Box<RawValue>>>>,
    flow_env: Option<&HashMap<String, Box<RawValue>>>,
    authed_client: Option<&AuthedClient>,
    by_id: Option<&IdContext>,
) -> error::Result<T>
where
    T: for<'de> serde::Deserialize<'de> + Send + Default,
{
    match transform {
        InputTransform::Static { value } => serde_json::from_str(value.get()).map_err(|e| {
            Error::ExecutionErr(format!(
                "Error parsing static value as {}: {e:#}",
                std::any::type_name::<T>()
            ))
        }),
        InputTransform::Javascript { expr } => {
            let mut context = HashMap::with_capacity(2);
            context.insert("result".to_string(), last_result.clone());
            context.insert("previous_result".to_string(), last_result.clone());
            let result = eval_timeout(
                expr.to_string(),
                context,
                flow_args,
                flow_env,
                authed_client,
                by_id,
                None,
            )
            .warn_after_seconds(3)
            .await
            .map_err(|e| {
                Error::ExecutionErr(format!(
                    "Error during evaluation of expression `{expr}`:\n{e:#}"
                ))
            })?;

            serde_json::from_str(result.get()).map_err(|e| {
                Error::ExecutionErr(format!(
                    "Error parsing result as {}: {e:#}. Value was: {}",
                    std::any::type_name::<T>(),
                    result.get()
                ))
            })
        }
        InputTransform::Ai => Ok(T::default()),
    }
}

/// resumes should be in order of timestamp ascending, so that more recent are at the end
#[instrument(level = "trace", skip_all)]
async fn transform_input(
    flow_args: Marc<HashMap<String, Box<RawValue>>>,
    flow_env: Option<&HashMap<String, Box<RawValue>>>,
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
                    flow_env,
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
            InputTransform::Ai => (),
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
    flow_runners: Option<Arc<FlowRunners>>,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
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
            let mini_job = MiniCompletedJob::from(flow_job.clone());
            let runnable_path = flow_job.runnable_path.as_ref().unwrap().clone();
            let schedule_push_result = (|| async {
                let tx = db.begin().warn_after_seconds(5).await
                    .map_err(|e| Error::internal_err(format!("begin tx for schedule push: {e:#}")))?;
                let (tx, schedule_push_err) = try_schedule_next_job(
                    db,
                    tx,
                    &mini_job,
                    &schedule,
                    &runnable_path,
                )
                .await;
                if let Some(err) = schedule_push_err {
                    return Err(err);
                }
                tx.commit().warn_after_seconds(5).await
                    .map_err(|e| Error::internal_err(format!("commit schedule push: {e:#}")))?;
                Ok::<(), Error>(())
            })
            .retry(
                ConstantBuilder::default()
                    .with_delay(std::time::Duration::from_secs(3))
                    .with_max_times(10)
                    .build(),
            )
            .when(|err: &Error| !matches!(err, Error::QuotaExceeded(_) | Error::NotFound(_)))
            .notify(|err: &Error, dur: std::time::Duration| {
                tracing::error!(
                    "Could not push next scheduled job for flow schedule {}, retrying in {dur:#?}: {err:#?}",
                    schedule.path
                );
            })
            .sleep(tokio::time::sleep)
            .await;

            // Non-retryable errors (QuotaExceeded, NotFound) are handled inside
            // try_schedule_next_job (schedule disabled, returns None), so they never
            // reach here. This handles only transient errors after retry exhaustion.
            if let Err(err) = schedule_push_result {
                tracing::error!(
                    "Could not push next scheduled job for {} after retries: {err}. Disabling schedule.",
                    schedule.path
                );
                if let Err(disable_err) = sqlx::query!(
                    "UPDATE schedule SET enabled = false, error = $1 WHERE workspace_id = $2 AND path = $3",
                    err.to_string(),
                    &flow_job.workspace_id,
                    &schedule.path
                )
                .execute(db)
                .await
                {
                    report_error_to_workspace_handler_or_critical_side_channel(
                        &mini_job,
                        db,
                        format!(
                            "Could not push next scheduled job for {} and could not disable schedule: {disable_err}",
                            schedule.path,
                        ),
                    )
                    .await;
                    return Err(SchedulePushZombieError(
                        format!(
                            "Could not push or disable schedule {} after retries",
                            schedule.path
                        ),
                    ).into());
                }
            }
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
        let next = Box::pin(push_next_flow_job(
            flow_job,
            status,
            flow,
            db,
            client,
            last_result.clone(),
            same_worker_tx,
            worker_dir,
            worker_name,
            flow_runners.clone(),
            job_completed_tx.clone(),
            &killpill_rx,
        ))
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

#[derive(Debug)]
enum PushNextFlowJob {
    Rec(PushNextFlowJobRec),
    Done(Option<UpdateFlow>),
}

#[derive(Debug)]
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
    flow_runners: Option<Arc<FlowRunners>>,
    job_completed_tx: JobCompletedSender,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
) -> error::Result<PushNextFlowJob> {
    let job_root = flow_job
        .flow_innermost_root_job
        .map(|x| x.to_string())
        .unwrap_or_else(|| "none".to_string());

    let mut step = Step::from_i32_and_len(status.step, flow.modules.len());

    tracing::info!(id = %flow_job.id, root_id = %job_root, step = ?step, "pushing next flow job");

    let mut status_module = match step {
        Step::Step { idx: i, .. } => status
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

    // tracing::error!("status_module: {status_module:#?}");

    let fj: mappable_rc::Marc<MiniPulledJob> = flow_job.clone().into();
    let arc_flow_job_args: Marc<HashMap<String, Box<RawValue>>> =
        Marc::map(fj, |x: &MiniPulledJob| {
            if let Some(args) = &x.args {
                &args.0
            } else {
                &EHM
            }
        });

    // if this is an empty module without preprocessor or if the module has already been completed, successfully, update the parent flow
    if (flow.modules.is_empty() && !step.is_preprocessor_step())
        || matches!(status_module, FlowStatusModule::Success { .. })
    {
        return Ok(PushNextFlowJob::Done(Some(UpdateFlow {
            flow: flow_job.id,
            success: true,
            result: if flow.modules.is_empty() {
                to_raw_value(arc_flow_job_args.as_ref())
            } else if matches!(
                status_module,
                FlowStatusModule::Success { branch_chosen: Some(_), .. }
            ) {
                last_job_result
                    .as_ref()
                    .map(|x| x.as_ref().clone())
                    .unwrap_or_else(|| to_raw_value(&json!("{}")))
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

    if matches!(step, Step::Step { idx: 0, .. }) {
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
                flow.flow_env.as_ref(),
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
    } else if matches!(step, Step::Step { idx: 0, .. }) || step.is_preprocessor_step() {
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

            // Query for both step-level resumes (job = step_id) and flow-level resumes (job = flow_id)
            // Flow-level resumes allow pre-approvals that can be consumed by any suspend step
            let resumes = sqlx::query_as::<_, ResumeRow>(
                 "SELECT value, approver, resume_id, approved FROM resume_job WHERE job = $1 OR job = $2 ORDER BY created_at ASC",
             )
             .bind(last)
             .bind(flow_job.id)
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
                let user_groups_required: Vec<String>;
                if let Some(user_groups_required_as_input_transform) = suspend.user_groups_required
                {
                    match user_groups_required_as_input_transform {
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
                                     flow.flow_env.as_ref(),
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
                        InputTransform::Ai => {
                            user_groups_required = Vec::new();
                        }
                    }
                } else {
                    user_groups_required = Vec::new();
                };

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

    let mut module = flow.get_flow_module_at_step(step)?;

    let current_id = &module.id;
    let mut previous_id = match step {
        Step::Step { idx: i, .. } if i >= 1 => {
            flow.modules.get(i - 1).map(|m| m.id.clone()).unwrap()
        }
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
            let sleep_input_transform = if let Step::Step { idx: i, .. } = step {
                i.checked_sub(1)
                    .and_then(|i| flow.modules.get(i))
                    .and_then(|m| m.sleep.clone())
            } else {
                None
            };

            if let Some(input_transform) = sleep_input_transform {
                let timeout_value = evaluate_input_transform::<serde_json::Value>(
                    &input_transform,
                    arc_last_job_result.clone(),
                    Some(arc_flow_job_args.clone()),
                    flow.flow_env.as_ref(),
                    Some(client),
                    None,
                )
                .await;
                match timeout_value {
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
        evaluate_retry(
            retry,
            &status.retry,
            arc_last_job_result.clone(),
            arc_flow_job_args.clone(),
            flow.flow_env.as_ref(),
            Some(client),
        )
        .await?
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
        let idcontext = get_transform_context(&flow_job, previous_id.as_str(), &status);
        compute_bool_from_expr(
            &skip_if.expr,
            arc_flow_job_args.clone(),
            flow.flow_env.as_ref(),
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
    } else if step.is_preprocessor_step() {
        let mut hm = (*arc_flow_job_args).clone();
        hm.insert(
            ENTRYPOINT_OVERRIDE.to_string(),
            to_raw_value(&"preprocessor"),
        );
        Ok(Marc::new(hm))
    } else if module.pass_flow_input_directly.unwrap_or(false) {
        // If pass_flow_input_directly is set, use flow args directly
        Ok(arc_flow_job_args.clone())
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
                | FlowModuleValue::Flow { input_transforms, .. }
                | FlowModuleValue::AIAgent { input_transforms, .. },
            ) => {
                let ctx = get_transform_context(&flow_job, &previous_id, &status);
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
                    flow.flow_env.as_ref(),
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
        flow.flow_env.as_ref(),
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
                    flow_jobs: if branch_chosen.is_some() {
                        None
                    } else {
                        Some(vec![])
                    },
                    flow_jobs_success: if branch_chosen.is_some() {
                        None
                    } else {
                        Some(vec![])
                    },
                    flow_jobs_duration: if branch_chosen.is_some() {
                        None
                    } else {
                        Some(FlowJobsDuration { started_at: vec![], duration_ms: vec![] })
                    },
                    branch_chosen: branch_chosen,
                    approvers: vec![],
                    failed_retries: vec![],
                    skipped: false,
                    agent_actions: None,
                    agent_actions_success: None,
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

    // only start runners if we're not already in a squash for loop
    let start_runners = flow_runners.is_none()
        && matches!(
            next_status,
            NextStatus::NextLoopIteration { start_runners: true, .. }
        );

    let do_not_pass_runners = matches!(next_status, NextStatus::NextStep { .. })
        && flow_runners
            .as_ref()
            .is_some_and(|fr| fr.job_id == flow_job.id);

    let continue_with_runners = (start_runners || (flow_runners.is_some() && !do_not_pass_runners))
        && module.suspend.is_none()
        && module.sleep.is_none();

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
            .execute(&mut *tx)
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
                ..
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
                    let ctx = get_transform_context(&flow_job, "", &status);
                    let ti = transform_input(
                        Marc::new(args),
                        flow.flow_env.as_ref(),
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
                    let Some(itered) = itered else {
                        return Err(Error::ExecutionErr(format!(
                            "iterator itered should never be none for parallel for loop jobs"
                        )));
                    };

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
                        let ctx = get_transform_context(&flow_job, &previous_id, &status);
                        let ti = transform_input(
                            Marc::new(hm),
                            flow.flow_env.as_ref(),
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

        let flow_innermost_root_job = if {
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

        let flow_root_job = get_root_job_id(&flow_job);

        // forward root job permissions to the new job
        let job_perms: Option<Authed> =
            get_job_perms(&mut *tx, &flow_root_job, &flow_job.workspace_id)
                .await?
                .map(|x| x.into());

        tracing::debug!(id = %flow_job.id, root_id = %job_root, "computed perms for job {i} of {len}");
        let tag = if step.is_preprocessor_step()
            || (flow_job.tag == "flow" || flow_job.tag == format!("flow-{}", flow_job.workspace_id))
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

        // Check tag availability for flow substeps to prevent abuse
        if let Some(tag_str) = tag.as_deref().filter(|t| !t.is_empty()) {
            check_tag_available_for_workspace_internal(
                &db,
                &flow_job.workspace_id,
                tag_str,
                email,
                None, // no token for flow substeps so no scopes so no scope_tags
            )
            .warn_after_seconds_with_sql(
                1,
                "check_tag_available_for_workspace_internal".to_string(),
            )
            .await?;
        }

        let evaluated_timeout = if let Some(timeout_transform) = &module.timeout {
            let ctx = get_transform_context(&flow_job, &previous_id, &status);

            let timeout_value = evaluate_input_transform::<i32>(
                timeout_transform,
                arc_last_job_result.clone(),
                Some(arc_flow_job_args.clone()),
                flow.flow_env.as_ref(),
                Some(client),
                Some(&ctx),
            )
            .await?;

            if timeout_value < 0 {
                return Err(Error::ExecutionErr(
                    "Timeout value cannot be negative".to_string(),
                ));
            }

            Some(timeout_value)
        } else {
            payload_tag.timeout
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
            Some(flow_root_job),
            flow_innermost_root_job,
            None,
            true,
            continue_on_same_worker,
            err,
            flow_job.visible_to_owner,
            tag,
            evaluated_timeout,
            Some(module.id.clone()),
            new_job_priority_override,
            job_perms.as_ref(),
            continue_with_runners,
            None,
            None,
            None,
        )
        .warn_after_seconds(2)
        .await?;

        if continue_on_same_worker || continue_with_runners {
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

        if value_with_parallel.type_ == "forloopflow"
            && value_with_parallel.parallel.unwrap_or(false)
        {
            if let Some(parallelism_transform) = &value_with_parallel.parallelism {
                tracing::debug!(id = %flow_job.id, root_id = %job_root, "evaluating parallelism expression for forloopflow job {uuid}");

                let ctx = get_transform_context(&flow_job, &previous_id, &status);

                let evaluated_parallelism = evaluate_input_transform::<u16>(
                    parallelism_transform,
                    arc_last_job_result.clone(),
                    Some(arc_flow_job_args.clone()),
                    flow.flow_env.as_ref(),
                    Some(client),
                    Some(&ctx),
                )
                .await?;

                tracing::debug!(id = %flow_job.id, root_id = %job_root, "updating suspend for forloopflow job {uuid} with parallelism {evaluated_parallelism}");

                if i as u16 >= evaluated_parallelism {
                    sqlx::query!(
                        "UPDATE v2_job_queue SET
                             suspend = $1,
                             suspend_until = now() + interval '14 day',
                             running = true
                         WHERE id = $2",
                        (i as u16 - evaluated_parallelism + 1) as i32,
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
                 flow_innermost_root_job.unwrap_or(flow_job.id)
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
                    itered_len,
                    mut flow_jobs,
                    while_loop,
                    mut flow_jobs_success,
                    mut flow_jobs_duration,
                    ..
                },
            ..
        } => {
            let uuid = one_uuid?;

            flow_jobs.push(uuid);

            if let Some(flow_jobs_success) = &mut flow_jobs_success {
                flow_jobs_success.push(None);
            }
            if let Some(flow_jobs_duration) = &mut flow_jobs_duration {
                flow_jobs_duration.push(&None);
            }

            // Conditionally write itered to separate table if all workers support it
            // Returns None if written to table, Some(itered) if should be in JSONB
            let itered_for_status = write_itered_to_db(db, flow_job.id, &itered).await?;

            FlowStatusModule::InProgress {
                job: uuid,
                iterator: Some(FlowIterator {
                    index,
                    itered: itered_for_status,
                    itered_len: Some(itered_len),
                }),
                flow_jobs: Some(flow_jobs),
                flow_jobs_success,
                flow_jobs_duration,
                branch_chosen: None,
                branchall: None,
                id: status_module.id(),
                parallel: false,
                while_loop,
                progress: None,
                agent_actions: None,
                agent_actions_success: None,
            }
        }
        NextStatus::AllFlowJobs { iterator, branchall, .. } => {
            let iterator_for_status = if let Some(mut iter) = iterator {
                iter.itered = None; // in parallel for loops, itered is only useful when starting the jobs, no need to store it in status/or the dedicated table
                Some(iter)
            } else {
                None
            };

            FlowStatusModule::InProgress {
                job: flow_job.id,
                iterator: iterator_for_status,
                flow_jobs_success: Some(vec![None; uuids.len()]),
                flow_jobs: Some(uuids.clone()),
                flow_jobs_duration: Some(FlowJobsDuration::new(uuids.len())),
                branch_chosen: None,
                branchall,
                id: status_module.id(),
                parallel: true,
                while_loop: false,
                progress: None,
                agent_actions: None,
                agent_actions_success: None,
            }
        }
        NextStatus::NextBranchStep(NextBranch {
            mut flow_jobs,
            status,
            mut flow_jobs_success,
            mut flow_jobs_duration,
            ..
        }) => {
            let uuid = one_uuid?;
            flow_jobs.push(uuid);
            if let Some(flow_jobs_success) = &mut flow_jobs_success {
                flow_jobs_success.push(None);
            }
            if let Some(flow_jobs_duration) = &mut flow_jobs_duration {
                flow_jobs_duration.push(&None);
            }
            FlowStatusModule::InProgress {
                job: uuid,
                iterator: None,
                flow_jobs: Some(flow_jobs),
                flow_jobs_success,
                flow_jobs_duration,
                branch_chosen: None,
                branchall: Some(status),
                id: status_module.id(),
                parallel: false,
                while_loop: false,
                progress: None,
                agent_actions: None,
                agent_actions_success: None,
            }
        }

        NextStatus::BranchChosen(branch) => FlowStatusModule::InProgress {
            job: one_uuid?,
            iterator: None,
            flow_jobs: None,
            flow_jobs_success: None,
            flow_jobs_duration: None,
            branch_chosen: Some(branch),
            branchall: None,
            id: status_module.id(),
            parallel: false,
            while_loop: false,
            progress: None,
            agent_actions: None,
            agent_actions_success: None,
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
        Step::Step { idx: i, .. } => {
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

    if continue_on_same_worker || continue_with_runners {
        if !is_one_uuid {
            return Err(Error::BadRequest(
                 "Cannot continue on same worker with multiple jobs, parallel cannot be used in conjunction with same_worker".to_string(),
             ));
        }
    }
    tx.commit().warn_after_seconds(3).await?;
    tracing::info!(id = %flow_job.id, root_id = %job_root, "all next flow jobs pushed: {uuids:?}");

    if continue_on_same_worker || continue_with_runners {
        let flow_runners = if start_runners {
            tracing::info!(id = %flow_job.id, "starting flow runners for module {}", module.id);
            let (new_flow_runners, new_flow_runner_handles) =
                crate::dedicated_worker_oss::spawn_flow_module_runners(
                    &flow_job,
                    module,
                    flow.failure_module.as_ref(),
                    &killpill_rx,
                    db,
                    worker_dir,
                    &client.base_internal_url,
                    worker_name,
                    &job_completed_tx,
                )
                .await
                .with_context(|| {
                    format!(
                        "failed to spawn flow module runners for job: {}",
                        flow_job.id
                    )
                })?;

            let flow_runners = FlowRunners {
                runners: new_flow_runners,
                handles: new_flow_runner_handles,
                job_id: flow_job.id,
            };
            Some(Arc::new(flow_runners))
        } else if !do_not_pass_runners {
            flow_runners
        } else {
            None
        };

        same_worker_tx
            .send(SameWorkerPayload { job_id: first_uuid, recoverable: true, flow_runners })
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
    itered_len: usize,
    flow_jobs: Vec<Uuid>,
    flow_jobs_success: Option<Vec<Option<bool>>>,
    flow_jobs_duration: Option<FlowJobsDuration>,
    new_args: Iter,
    while_loop: bool,
}

enum ForLoopStatus {
    ParallelIteration { itered: Vec<Box<RawValue>>, itered_len: usize },
    NextIteration(ForloopNextIteration),
    EmptyIterator,
}

#[derive(Debug)]
struct NextBranch {
    status: BranchAllStatus,
    flow_jobs: Vec<Uuid>,
    flow_jobs_success: Option<Vec<Option<bool>>>,
    flow_jobs_duration: Option<FlowJobsDuration>,
}

#[derive(Debug)]
enum NextStatus {
    NextStep,
    BranchChosen(BranchChosen),
    NextBranchStep(NextBranch),
    NextLoopIteration {
        next: ForloopNextIteration,
        simple_input_transforms: Option<HashMap<String, InputTransform>>,
        start_runners: bool,
    },
    AllFlowJobs {
        branchall: Option<BranchAllStatus>,
        iterator: Option<FlowIterator>,
        simple_input_transforms: Option<HashMap<String, InputTransform>>,
    },
}

#[derive(Debug, Clone)]
pub struct JobPayloadWithTag {
    pub payload: JobPayload,
    pub tag: Option<String>,
    pub delete_after_use: bool,
    pub timeout: Option<i32>,
    pub on_behalf_of: Option<OnBehalfOf>,
}
#[derive(Debug)]
enum ContinuePayload {
    SingleJob(JobPayloadWithTag),
    ParallelJobs(Vec<JobPayloadWithTag>),
}

#[derive(Debug)]
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

pub fn get_path(flow_job: &MiniPulledJob, status: &FlowStatus, module: &FlowModule) -> String {
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
    flow_env: Option<&HashMap<String, Box<RawValue>>>,
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
        FlowModuleValue::AIAgent { .. } => {
            let path = get_path(flow_job, status, module);
            let payload = JobPayload::AIAgent { path };
            Ok(NextFlowTransform::Continue(
                ContinuePayload::SingleJob(JobPayloadWithTag {
                    payload,
                    tag: None,
                    delete_after_use,
                    timeout: None,
                    on_behalf_of: None,
                }),
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
            concurrency_settings,
            ..
        } => {
            let path = path.unwrap_or_else(|| get_path(flow_job, status, module));

            let payload = raw_script_to_payload(
                path,
                content,
                language,
                lock,
                concurrency_settings,
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
            concurrency_settings,
            ..
        } => {
            let path = get_path(flow_job, status, module);

            let payload = JobPayloadWithTag {
                payload: JobPayload::FlowScript {
                    id,
                    language,
                    concurrency_settings: concurrency_settings.into(),
                    cache_ttl: module.cache_ttl.map(|x| x as i32),
                    cache_ignore_s3_path: module.cache_ignore_s3_path.clone(),
                    dedicated_worker: None,
                    path,
                },
                tag: tag.clone(),
                delete_after_use,
                timeout: None,
                on_behalf_of: None,
            };
            Ok(NextFlowTransform::Continue(
                ContinuePayload::SingleJob(payload),
                NextStatus::NextStep,
            ))
        }
        FlowModuleValue::WhileloopFlow { modules, modules_node, squash, .. } => {
            // if it's a simple single step flow, we will collapse it as an optimization and need to pass flow_input as an arg
            let is_simple = is_simple_modules(&modules, flow.failure_module.as_ref());
            let (flow_jobs, flow_jobs_success, flow_jobs_duration) = match status_module {
                FlowStatusModule::InProgress {
                    flow_jobs: Some(flow_jobs),
                    flow_jobs_success,
                    flow_jobs_duration,
                    ..
                } => (
                    flow_jobs.clone(),
                    flow_jobs_success.clone(),
                    flow_jobs_duration.clone(),
                ),
                _ => (vec![], Some(vec![]), Some(FlowJobsDuration::new(0))),
            };
            let next_loop_idx = flow_jobs.len();

            let start_runners = next_loop_idx == 0 && squash.unwrap_or(false);
            next_loop_iteration(
                flow,
                status,
                ForloopNextIteration {
                    index: next_loop_idx,
                    itered: vec![],
                    itered_len: 0,
                    flow_jobs: flow_jobs,
                    flow_jobs_success: flow_jobs_success,
                    flow_jobs_duration: flow_jobs_duration,
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
                start_runners,
            )
            .await
        }
        /* forloop modules are expected set `iter: { value: Value, index: usize }` as job arguments */
        FlowModuleValue::ForloopFlow {
            modules, modules_node, iterator, parallel, squash, ..
        } => {
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
                flow_env,
                client,
                &parallel,
                db,
            )
            .await?;

            match next_loop_status {
                ForLoopStatus::EmptyIterator => {
                    Ok(NextFlowTransform::EmptyInnerFlows { branch_chosen: None })
                }
                ForLoopStatus::NextIteration(ns) => {
                    let start_runners = ns.index == 0 && squash.unwrap_or(false);
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
                        start_runners,
                    )
                    .await
                }
                ForLoopStatus::ParallelIteration { itered, itered_len } => {
                    // let inner_path = Some(format!("{}/loop-parallel", flow_job.script_path(),));
                    // let value = &modules[0].get_value()?;

                    // we removed the is_simple_case
                    // if is_simple {
                    //     let payload =
                    //         payload_from_simple_module(value, db, flow_job, module, inner_path)
                    //             .await?;
                    //     ContinuePayload::ForloopJobs { n: itered.len(), payload: payload }
                    // } else {

                    let payloads = (0..itered_len)
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
                            iterator: Some(FlowIterator {
                                index: 0,
                                itered_len: Some(itered_len),
                                itered: Some(itered),
                            }),
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
                    let idcontext = get_transform_context(&flow_job, previous_id, &status);
                    for (i, b) in branches.iter().enumerate() {
                        let pred = compute_bool_from_expr(
                            &b.expr,
                            arc_flow_job_args.clone(),
                            flow.flow_env.as_ref(),
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
            let (branch_status, flow_jobs, flow_jobs_success, flow_jobs_duration) =
                match status_module {
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
                                return Ok(NextFlowTransform::EmptyInnerFlows {
                                    branch_chosen: None,
                                });
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
                                Some(FlowJobsDuration::new(0)),
                            )
                        }
                    }
                    FlowStatusModule::InProgress {
                        branchall: Some(BranchAllStatus { branch, len }),
                        flow_jobs: Some(flow_jobs),
                        flow_jobs_success,
                        flow_jobs_duration,
                        ..
                    } if !parallel => (
                        BranchAllStatus { branch: branch + 1, len: len.clone() },
                        flow_jobs.clone(),
                        flow_jobs_success.clone(),
                        flow_jobs_duration.clone(),
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
                    flow_jobs_duration,
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
    start_runners: bool,
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
            NextStatus::NextLoopIteration { next: ns, simple_input_transforms, start_runners },
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
        NextStatus::NextLoopIteration { next: ns, simple_input_transforms: None, start_runners },
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
    flow_env: Option<&HashMap<String, Box<RawValue>>>,
    client: &AuthedClient,
    parallel: &bool,
    db: &DB,
) -> Result<ForLoopStatus, Error> {
    let next_loop_status = match status_module {
        FlowStatusModule::WaitingForPriorSteps { .. }
        | FlowStatusModule::WaitingForEvents { .. }
        | FlowStatusModule::WaitingForExecutor { .. } => {
            let by_id = if let Some(x) = by_id {
                x
            } else {
                get_transform_context(&flow_job, previous_id, &status)
            };
            /* Iterator is an InputTransform, evaluate it into an array. */
            let itered_raw = match iterator {
                InputTransform::Static { value } => to_raw_value(value),
                InputTransform::Ai => {
                    return Err(Error::ExecutionErr(format!(
                        "AI input transform not supported for iterator"
                    )))?
                }
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
                        flow_env,
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
                ForLoopStatus::ParallelIteration { itered_len: itered.len(), itered }
            } else if let Some(first) = itered.first() {
                let iter = Iter { index: 0 as i32, value: first.to_owned() };
                ForLoopStatus::NextIteration(ForloopNextIteration {
                    index: 0,
                    itered_len: itered.len(),
                    itered,
                    flow_jobs: vec![],
                    flow_jobs_success: Some(vec![]),
                    flow_jobs_duration: Some(FlowJobsDuration::new(0)),
                    new_args: iter,
                    while_loop: false,
                })
            } else {
                panic!("itered cannot be empty")
            }
        }

        FlowStatusModule::InProgress {
            iterator: Some(FlowIterator { itered, index, .. }),
            flow_jobs: Some(flow_jobs),
            flow_jobs_success,
            flow_jobs_duration,
            ..
        } if !*parallel => {
            // Read itered from separate table or fallback to JSONB
            let itered = read_itered_from_db(db, flow_job.id, itered).await?;

            let itered_new = if itered.is_empty() {
                // it's possible we need to re-compute the iterator Input Transforms here, in particular if the flow is being restarted inside the loop
                let by_id = if let Some(x) = by_id {
                    x
                } else {
                    get_transform_context(&flow_job, previous_id, &status)
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
                            flow_env,
                            Some(client),
                            Some(&by_id),
                            None,
                        )
                        .await?
                    }
                    InputTransform::Ai => {
                        return Err(Error::ExecutionErr(format!(
                            "AI input transform not supported for iterator"
                        )))?
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
                itered_len: itered_new.len(),
                flow_jobs: flow_jobs.clone(),
                flow_jobs_success: flow_jobs_success.clone(),
                flow_jobs_duration: flow_jobs_duration.clone(),
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
            concurrency_settings,
            ..
        } => raw_script_to_payload(
            path.unwrap_or_else(|| inner_path),
            content,
            language,
            lock,
            concurrency_settings,
            module,
            tag,
            delete_after_use,
        ),
        FlowModuleValue::FlowScript {
            id, // flow_node(id).
            tag,
            language,
            concurrency_settings,
            ..
        } => JobPayloadWithTag {
            payload: JobPayload::FlowScript {
                id,
                language,
                cache_ttl: module.cache_ttl.map(|x| x as i32),
                cache_ignore_s3_path: module.cache_ignore_s3_path,
                dedicated_worker: None,
                path: inner_path,
                concurrency_settings: concurrency_settings.into(),
            },
            tag,
            delete_after_use,
            timeout: None, // timeout evaluation handled at higher level
            on_behalf_of: None,
        },
        _ => unreachable!("is simple flow"),
    })
}

pub fn raw_script_to_payload(
    path: String,
    content: String,
    language: windmill_common::scripts::ScriptLang,
    lock: Option<String>,
    concurrency_settings: ConcurrencySettingsWithCustom,
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
            cache_ttl: module.cache_ttl.map(|x| x as i32),
            cache_ignore_s3_path: module.cache_ignore_s3_path,
            dedicated_worker: None,
            concurrency_settings,
            // TODO: Should this have debouncing?
            debouncing_settings: DebouncingSettings::default(),
        }),
        tag,
        delete_after_use,
        timeout: None, // timeout evaluation handled at higher level
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
        get_latest_flow_version_info_for_path(None, &db, w_id, &path, true).await?;
    let on_behalf_of = if let Some(email) = on_behalf_of_email {
        Some(OnBehalfOf { email, permissioned_as: username_to_permissioned_as(&edited_by) })
    } else {
        None
    };
    let payload =
        JobPayload::Flow { path, dedicated_worker: None, apply_preprocessor: false, version };
    Ok(JobPayloadWithTag { payload, tag, delete_after_use, timeout: None, on_behalf_of })
}

pub async fn script_to_payload(
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
        let (jp, tag, delete_after_use, script_timeout, on_behalf_of) = script_path_to_payload(
            &script_path,
            None,
            db.clone(),
            &flow_job.workspace_id,
            Some(true),
        )
        .await?;
        (
            jp,
            tag_override.to_owned().or(tag),
            delete_after_use,
            script_timeout,
            on_behalf_of,
        )
    } else {
        let hash = script_hash.unwrap();

        let ScriptHashInfo {
            tag,
            cache_ttl,
            language,
            dedicated_worker,
            priority,
            delete_after_use,
            timeout,
            on_behalf_of_email,
            created_by,
            runnable_settings:
                ScriptRunnableSettingsInline { concurrency_settings, debouncing_settings },
            ..
        } = get_script_info_for_hash(None, db, &flow_job.workspace_id, hash.0)
            .await?
            .prefetch_cached(&db)
            .await?;

        let on_behalf_of = if let Some(email) = on_behalf_of_email {
            Some(OnBehalfOf { email, permissioned_as: username_to_permissioned_as(&created_by) })
        } else {
            None
        };
        (
            // We only apply the preprocessor if it's explicitly set to true in the module,
            // which can only happen if the the flow is a SingleStepFlow triggered by a trigger with retries or error handling.
            // In that case, apply_preprocessor is still only set to true if the script has a preprocesor.
            // We only check for script hash because SingleStepFlow triggers specifies the script hash
            JobPayload::ScriptHash {
                hash,
                path: script_path,
                concurrency_settings,
                debouncing_settings,
                cache_ttl: module.cache_ttl.map(|x| x as i32).ok_or(cache_ttl).ok(),
                cache_ignore_s3_path: module.cache_ignore_s3_path,
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

    let flow_step_timeout = if module.timeout.is_some() {
        None
    } else {
        script_timeout
    };
    Ok(JobPayloadWithTag {
        payload,
        tag,
        delete_after_use: final_delete_after_user,
        timeout: flow_step_timeout,
        on_behalf_of,
    })
}

pub fn get_transform_context(
    flow_job: &MiniPulledJob,
    previous_id: &str,
    status: &FlowStatus,
) -> IdContext {
    let steps_results: HashMap<String, JobResult> = status
        .modules
        .iter()
        .filter_map(|x| x.job_result().map(|y| (x.id(), y)))
        .collect();

    IdContext { flow_job: flow_job.id, steps_results, previous_id: previous_id.to_string() }
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
pub async fn get_previous_job_result(
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
        Some(FlowStatusModule::Success { job, .. }) if *job == Uuid::nil() => {
            // Empty branch — no real job was executed, return empty object
            Ok(None)
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
