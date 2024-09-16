use serde::Serialize;
use sqlx::{types::Json, Pool, Postgres};
use std::sync::Arc;

use uuid::Uuid;

use windmill_common::{
    error::{self, Error},
    jobs::QueuedJob,
    worker::to_raw_value,
    DB,
};

use windmill_queue::{append_logs, get_queued_job, CanceledBy, WrappedError};

#[cfg(feature = "prometheus")]
use windmill_queue::register_metric;

use serde_json::{json, value::RawValue};

use tokio::sync::mpsc::Sender;

use windmill_queue::{add_completed_job, add_completed_job_error};

use crate::{
    bash_executor::ANSI_ESCAPE_RE,
    common::{read_result, save_in_cache},
    worker_flow::update_flow_status_after_job_completion,
    AuthedClient, Histo, JobCompleted, JobCompletedSender, SameWorkerSender, SendResult,
};

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
    db: &DB,
) -> error::Result<()> {
    match result {
        Ok(r) => {
            let job = if let Some(column_order) = column_order {
                let mut job_with_column_order = (*job).clone();
                match job_with_column_order.flow_status {
                    Some(_) => {
                        tracing::warn!("flow_status was expected to be none");
                    }
                    None => {
                        job_with_column_order.flow_status =
                            Some(sqlx::types::Json(to_raw_value(&serde_json::json!({
                                "_metadata": {
                                    "column_order": column_order
                                }
                            }))));
                    }
                }
                Arc::new(job_with_column_order)
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
        }
    };
    Ok(())
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
    worker_save_completed_job_duration: Option<Histo>,
    worker_flow_transition_duration: Option<Histo>,
    job_completed_tx: Sender<SendResult>,
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
        worker_save_completed_job_duration,
        worker_flow_transition_duration,
        job_completed_tx.clone(),
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
    _worker_save_completed_job_duration: Option<Histo>,
    _worker_flow_transition_duration: Option<Histo>,
    job_completed_tx: Sender<SendResult>,
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
        #[cfg(feature = "prometheus")]
        let timer = _worker_save_completed_job_duration
            .as_ref()
            .map(|x| x.start_timer());
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
        )
        .await?;
        drop(job);

        #[cfg(feature = "prometheus")]
        timer.map(|x| x.stop_and_record());

        if is_flow_step {
            if let Some(parent_job) = parent_job {
                #[cfg(feature = "prometheus")]
                let timer = _worker_flow_transition_duration
                    .as_ref()
                    .map(|x| x.start_timer());
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
                )
                .await?;
                #[cfg(feature = "prometheus")]
                timer.map(|x| x.stop_and_record());
            }
        }
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
