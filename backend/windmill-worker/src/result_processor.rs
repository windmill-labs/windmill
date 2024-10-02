use serde::Serialize;
use sqlx::{types::Json, Pool, Postgres};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU16, Ordering},
        Arc,
    },
};

use uuid::Uuid;

use windmill_common::{
    add_time,
    error::{self, Error},
    jobs::{JobKind, QueuedJob},
    worker::{to_raw_value, WORKER_GROUP},
    DB,
};

#[cfg(feature = "benchmark")]
use windmill_common::bench::{BenchmarkInfo, BenchmarkIter};

use windmill_queue::{append_logs, get_queued_job, CanceledBy, WrappedError};

#[cfg(feature = "prometheus")]
use windmill_queue::register_metric;

use serde_json::{json, value::RawValue};

use tokio::{
    sync::{
        self,
        mpsc::{Receiver, Sender},
    },
    task::JoinHandle,
};

use windmill_queue::{add_completed_job, add_completed_job_error};

use crate::{
    bash_executor::ANSI_ESCAPE_RE,
    common::{read_result, save_in_cache},
    worker_flow::update_flow_status_after_job_completion,
    AuthedClient, JobCompleted, JobCompletedSender, SameWorkerSender, SendResult, INIT_SCRIPT_TAG,
};

pub fn start_background_processor<R>(
    mut job_completed_rx: Receiver<SendResult>,
    job_completed_sender: Sender<SendResult>,
    same_worker_queue_size: Arc<AtomicU16>,
    job_completed_processor_is_done: Arc<AtomicBool>,
    base_internal_url: String,
    db: DB,
    worker_dir: String,
    same_worker_tx: SameWorkerSender,
    rsmq: Option<R>,
    worker_name: String,
    killpill_tx: sync::broadcast::Sender<()>,
    is_dedicated_worker: bool,
) -> JoinHandle<()>
where
    R: rsmq_async::RsmqConnection + Send + Sync + Clone + 'static,
{
    tokio::spawn(async move {
        let mut has_been_killed = false;

        #[cfg(feature = "benchmark")]
        let mut infos = BenchmarkInfo::new();

        //if we have been killed, we want to drain the queue of jobs
        while let Some(sr) = {
            if has_been_killed && same_worker_queue_size.load(Ordering::SeqCst) == 0 {
                job_completed_rx.try_recv().ok()
            } else {
                job_completed_rx.recv().await
            }
        } {
            #[cfg(feature = "benchmark")]
            let mut bench = BenchmarkIter::new();

            match sr {
                SendResult::JobCompleted(jc) => {
                    let rsmq = rsmq.clone();

                    let is_init_script_and_failure =
                        !jc.success && jc.job.tag.as_str() == INIT_SCRIPT_TAG;
                    let is_dependency_job = matches!(
                        jc.job.job_kind,
                        JobKind::Dependencies | JobKind::FlowDependencies
                    );

                    handle_receive_completed_job(
                        jc,
                        &base_internal_url,
                        &db,
                        &worker_dir,
                        &same_worker_tx,
                        rsmq,
                        &worker_name,
                        job_completed_sender.clone(),
                        #[cfg(feature = "benchmark")]
                        &mut bench,
                    )
                    .await;

                    if is_init_script_and_failure {
                        tracing::error!("init script errored, exiting");
                        killpill_tx.send(()).unwrap_or_default();
                        break;
                    }
                    if is_dependency_job && is_dedicated_worker {
                        tracing::error!("Dedicated worker executed a dependency job, a new script has been deployed. Exiting expecting to be restarted.");
                        sqlx::query!(
                                "UPDATE config SET config = config WHERE name = $1",
                                format!("worker__{}", *WORKER_GROUP)
                            )
                            .execute(&db)
                            .await
                            .expect("update config to trigger restart of all dedicated workers at that config");
                        killpill_tx.send(()).unwrap_or_default();
                    }
                    add_time!(bench, "job completed processed");

                    #[cfg(feature = "benchmark")]
                    {
                        infos.add_iter(bench, true);
                    }
                }
                SendResult::UpdateFlow {
                    flow,
                    w_id,
                    success,
                    result,
                    worker_dir,
                    stop_early_override,
                    token,
                } => {
                    // let r;
                    tracing::info!(parent_flow = %flow, "updating flow status");
                    if let Err(e) = update_flow_status_after_job_completion(
                        &db,
                        &AuthedClient {
                            base_internal_url: base_internal_url.to_string(),
                            workspace: w_id.clone(),
                            token: token.clone(),
                            force_client: None,
                        },
                        flow,
                        &Uuid::nil(),
                        &w_id,
                        success,
                        Arc::new(result),
                        true,
                        same_worker_tx.clone(),
                        &worker_dir,
                        stop_early_override,
                        rsmq.clone(),
                        &worker_name,
                        job_completed_sender.clone(),
                        #[cfg(feature = "benchmark")]
                        &mut bench,
                    )
                    .await
                    {
                        tracing::error!("Error updating flow status after job completion for {flow} on {worker_name}: {e:#}");
                    }
                }
                SendResult::Kill => {
                    has_been_killed = true;
                }
            }
        }

        job_completed_processor_is_done.store(true, Ordering::SeqCst);

        tracing::info!("finished processing all completed jobs");

        #[cfg(feature = "benchmark")]
        {
            infos
                .write_to_file("profiling_result_processor.json")
                .expect("write to file profiling");
        }
    })
}

async fn send_job_completed(
    job_completed_tx: JobCompletedSender,
    job: Arc<QueuedJob>,
    result: Arc<Box<RawValue>>,
    mem_peak: i32,
    canceled_by: Option<CanceledBy>,
    success: bool,
    cached_res_path: Option<String>,
    token: String,
) {
    let jc = JobCompleted { job, result, mem_peak, canceled_by, success, cached_res_path, token };
    job_completed_tx.send(jc).await.expect("send job completed")
}

pub async fn process_result(
    job: Arc<QueuedJob>,
    result: error::Result<Arc<Box<RawValue>>>,
    job_dir: &str,
    job_completed_tx: JobCompletedSender,
    mem_peak: i32,
    canceled_by: Option<CanceledBy>,
    cached_res_path: Option<String>,
    token: String,
    column_order: Option<Vec<String>>,
    new_args: Option<HashMap<String, Box<RawValue>>>,
    db: &DB,
) -> error::Result<bool> {
    match result {
        Ok(r) => {
            let job = if column_order.is_some() || new_args.is_some() {
                let mut updated_job = (*job).clone();
                if let Some(column_order) = column_order {
                    match updated_job.flow_status {
                        Some(_) => {
                            tracing::warn!("flow_status was expected to be none");
                        }
                        None => {
                            updated_job.flow_status =
                                Some(sqlx::types::Json(to_raw_value(&serde_json::json!({
                                    "_metadata": {
                                        "column_order": column_order
                                    }
                                }))));
                        }
                    }
                }
                if let Some(new_args) = new_args {
                    match updated_job.flow_status {
                        Some(_) => {
                            tracing::warn!("flow_status was expected to be none");
                        }
                        None => {
                            // TODO save original args somewhere
                            // if let Some(args) = updated_job.args.as_mut() {
                            //     args.0.remove(ENTRYPOINT_OVERRIDE);
                            // }
                            updated_job.flow_status =
                                Some(sqlx::types::Json(to_raw_value(&serde_json::json!({
                                    "_metadata": {
                                        "preprocessed_args": true
                                    }
                                }))));
                        }
                    }
                    updated_job.args = Some(Json(new_args));
                }
                Arc::new(updated_job)
            } else {
                job
            };

            send_job_completed(
                job_completed_tx,
                job,
                r,
                mem_peak,
                canceled_by,
                true,
                cached_res_path,
                token,
            )
            .await;
            Ok(true)
        }
        Err(e) => {
            let error_value = match e {
                Error::ExitStatus(i) => {
                    let res = read_result(job_dir).await.ok();

                    if res.as_ref().is_some_and(|x| !x.get().is_empty()) {
                        res.unwrap()
                    } else {
                        let last_10_log_lines = sqlx::query_scalar!(
                            "SELECT right(logs, 600) FROM job_logs WHERE job_id = $1 AND workspace_id = $2 ORDER BY created_at DESC LIMIT 1",
                            &job.id,
                            &job.workspace_id
                        ).fetch_one(db).await.ok().flatten().unwrap_or("".to_string());

                        let log_lines = last_10_log_lines
                            .split("CODE EXECUTION ---")
                            .last()
                            .unwrap_or(&last_10_log_lines);

                        extract_error_value(log_lines, i, job.flow_step_id.clone())
                    }
                }
                err @ _ => to_raw_value(&SerializedError {
                    message: format!("error during execution of the script:\n{}", err),
                    name: "ExecutionErr".to_string(),
                    step_id: job.flow_step_id.clone(),
                }),
            };

            send_job_completed(
                job_completed_tx,
                job,
                Arc::new(to_raw_value(&error_value)),
                mem_peak,
                canceled_by,
                false,
                cached_res_path,
                token,
            )
            .await;
            Ok(false)
        }
    }
}

pub async fn handle_receive_completed_job<
    R: rsmq_async::RsmqConnection + Send + Sync + Clone + 'static,
>(
    jc: JobCompleted,
    base_internal_url: &str,
    db: &DB,
    worker_dir: &str,
    same_worker_tx: &SameWorkerSender,
    rsmq: Option<R>,
    worker_name: &str,
    job_completed_tx: Sender<SendResult>,
    #[cfg(feature = "benchmark")] bench: &mut BenchmarkIter,
) {
    let token = jc.token.clone();
    let workspace = jc.job.workspace_id.clone();
    let client = AuthedClient {
        base_internal_url: base_internal_url.to_string(),
        workspace,
        token,
        force_client: None,
    };
    let job = jc.job.clone();
    let mem_peak = jc.mem_peak.clone();
    let canceled_by = jc.canceled_by.clone();
    if let Err(err) = process_completed_job(
        jc,
        &client,
        db,
        &worker_dir,
        same_worker_tx.clone(),
        rsmq.clone(),
        worker_name,
        job_completed_tx.clone(),
        #[cfg(feature = "benchmark")]
        bench,
    )
    .await
    {
        handle_job_error(
            db,
            &client,
            job.as_ref(),
            mem_peak,
            canceled_by,
            err,
            false,
            same_worker_tx.clone(),
            &worker_dir,
            rsmq.clone(),
            worker_name,
            job_completed_tx,
            #[cfg(feature = "benchmark")]
            bench,
        )
        .await;
    }
}

#[tracing::instrument(name = "completed_job", level = "info", skip_all, fields(job_id = %job.id))]
pub async fn process_completed_job<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    JobCompleted { job, result, mem_peak, success, cached_res_path, canceled_by, .. }: JobCompleted,
    client: &AuthedClient,
    db: &DB,
    worker_dir: &str,
    same_worker_tx: SameWorkerSender,
    rsmq: Option<R>,
    worker_name: &str,
    job_completed_tx: Sender<SendResult>,
    #[cfg(feature = "benchmark")] bench: &mut BenchmarkIter,
) -> windmill_common::error::Result<()> {
    if success {
        // println!("bef completed job{:?}",  SystemTime::now());
        if let Some(cached_path) = cached_res_path {
            save_in_cache(db, client, &job, cached_path.to_string(), &result).await;
        }

        let is_flow_step = job.is_flow_step;
        let parent_job = job.parent_job.clone();
        let job_id = job.id.clone();
        let workspace_id = job.workspace_id.clone();

        add_completed_job(
            db,
            &job,
            true,
            false,
            Json(&result),
            mem_peak.to_owned(),
            canceled_by,
            rsmq.clone(),
            false,
            #[cfg(feature = "benchmark")]
            bench,
        )
        .await?;
        drop(job);

        add_time!(bench, "add_completed_job END");

        if is_flow_step {
            if let Some(parent_job) = parent_job {
                tracing::info!(parent_flow = %parent_job, subflow = %job_id, "updating flow status (2)");
                update_flow_status_after_job_completion(
                    db,
                    client,
                    parent_job,
                    &job_id,
                    &workspace_id,
                    true,
                    result,
                    false,
                    same_worker_tx.clone(),
                    &worker_dir,
                    None,
                    rsmq.clone(),
                    worker_name,
                    job_completed_tx,
                    #[cfg(feature = "benchmark")]
                    bench,
                )
                .await?;
            }
        }
        add_time!(bench, "updated flow status END");
    } else {
        let result = add_completed_job_error(
            db,
            &job,
            mem_peak.to_owned(),
            canceled_by,
            serde_json::from_str(result.get()).unwrap_or_else(
                |_| json!({ "message": format!("Non serializable error: {}", result.get()) }),
            ),
            rsmq.clone(),
            worker_name,
            false,
            #[cfg(feature = "benchmark")]
            bench,
        )
        .await?;
        if job.is_flow_step {
            if let Some(parent_job) = job.parent_job {
                tracing::error!(parent_flow = %parent_job, subflow = %job.id, "process completed job error, updating flow status");
                update_flow_status_after_job_completion(
                    db,
                    client,
                    parent_job,
                    &job.id,
                    &job.workspace_id,
                    false,
                    Arc::new(serde_json::value::to_raw_value(&result).unwrap()),
                    false,
                    same_worker_tx,
                    &worker_dir,
                    None,
                    rsmq,
                    worker_name,
                    job_completed_tx,
                    #[cfg(feature = "benchmark")]
                    bench,
                )
                .await?;
            }
        }
    }
    Ok(())
}

#[tracing::instrument(name = "job_error", level = "info", skip_all, fields(job_id = %job.id))]
pub async fn handle_job_error<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    db: &Pool<Postgres>,
    client: &AuthedClient,
    job: &QueuedJob,
    mem_peak: i32,
    canceled_by: Option<CanceledBy>,
    err: Error,
    unrecoverable: bool,
    same_worker_tx: SameWorkerSender,
    worker_dir: &str,
    rsmq: Option<R>,
    worker_name: &str,
    job_completed_tx: Sender<SendResult>,
    #[cfg(feature = "benchmark")] bench: &mut BenchmarkIter,
) {
    let err = match err {
        Error::JsonErr(err) => err,
        _ => json!({"message": err.to_string(), "name": "InternalErr"}),
    };

    let rsmq_2 = rsmq.clone();
    let update_job_future = || async {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("Unexpected error during job execution:\n{err:#?}"),
            db,
        )
        .await;
        add_completed_job_error(
            db,
            job,
            mem_peak,
            canceled_by.clone(),
            err.clone(),
            rsmq_2,
            worker_name,
            false,
            #[cfg(feature = "benchmark")]
            bench,
        )
        .await
    };

    let update_job_future = if job.is_flow_step || job.is_flow() {
        let (flow, job_status_to_update) = if let Some(parent_job_id) = job.parent_job {
            if let Err(e) = update_job_future().await {
                tracing::error!(
                    "error updating job future for job {} for handle_job_error: {e:#}",
                    job.id
                );
            }
            (parent_job_id, job.id)
        } else {
            (job.id, Uuid::nil())
        };

        let wrapped_error = WrappedError { error: err.clone() };
        tracing::error!(parent_flow = %flow, subflow = %job_status_to_update, "handle job error, updating flow status: {err:?}");
        let updated_flow = update_flow_status_after_job_completion(
            db,
            client,
            flow,
            &job_status_to_update,
            &job.workspace_id,
            false,
            Arc::new(serde_json::value::to_raw_value(&wrapped_error).unwrap()),
            unrecoverable,
            same_worker_tx,
            worker_dir,
            None,
            rsmq.clone(),
            worker_name,
            job_completed_tx.clone(),
            #[cfg(feature = "benchmark")]
            bench,
        )
        .await;

        if let Err(err) = updated_flow {
            if let Some(parent_job_id) = job.parent_job {
                if let Ok(Some(parent_job)) =
                    get_queued_job(&parent_job_id, &job.workspace_id, &db).await
                {
                    let e = json!({"message": err.to_string(), "name": "InternalErr"});
                    append_logs(
                        &parent_job.id,
                        &job.workspace_id,
                        format!("Unexpected error during flow job error handling:\n{err}"),
                        db,
                    )
                    .await;
                    let _ = add_completed_job_error(
                        db,
                        &parent_job,
                        mem_peak,
                        canceled_by.clone(),
                        e,
                        rsmq,
                        worker_name,
                        false,
                        #[cfg(feature = "benchmark")]
                        bench,
                    )
                    .await;
                }
            }
        }

        None
    } else {
        Some(update_job_future)
    };
    if let Some(f) = update_job_future {
        let _ = f().await;
    }
    tracing::error!(job_id = %job.id, "error handling job: {err:?} {} {} {}", job.id, job.workspace_id, job.created_by);
}

#[derive(Debug, Serialize)]
pub struct SerializedError {
    pub message: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,
}
pub fn extract_error_value(log_lines: &str, i: i32, step_id: Option<String>) -> Box<RawValue> {
    return to_raw_value(&SerializedError {
        message: format!(
            "ExitCode: {i}, last log lines:\n{}",
            ANSI_ESCAPE_RE.replace_all(log_lines.trim(), "").to_string()
        ),
        name: "ExecutionErr".to_string(),
        step_id,
    });
}
