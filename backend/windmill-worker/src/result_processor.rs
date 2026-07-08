#[cfg(feature = "otel")]
use opentelemetry::trace::FutureExt;

use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU16, Ordering},
        Arc,
    },
};
use tracing::{field, Instrument};
#[cfg(not(feature = "otel"))]
use windmill_common::otel_oss::FutureExt;

use uuid::Uuid;

/// Set by the result processor when a WAC child completion makes suspend reach 0,
/// signaling the worker main loop to check for suspended jobs immediately.
pub static WAC_SUSPEND_READY: AtomicBool = AtomicBool::new(false);

use windmill_common::{
    add_time,
    error::{self, Error},
    flow_status::FlowJobDuration,
    jobs::JobKind,
    utils::WarnAfterExt,
    worker::{error_to_value, to_raw_value, Connection, WORKER_GROUP},
    worker_group_job_stats::{accumulate_job_stats, flush_stats_to_db, JobStatsMap},
    KillpillSender, DB,
};

#[cfg(feature = "benchmark")]
use windmill_common::bench::{BenchmarkInfo, BenchmarkIter};

use windmill_queue::{
    append_logs, asset_dispatch, get_mini_completed_job, is_pre_shaped_wm_failure_result,
    CanceledBy, FlowRunners, JobCompleted, MiniCompletedJob, MiniPulledJob, ValidableJson,
    WrappedError, INIT_SCRIPT_TAG, MANUAL_FAILURE_ERROR_NAME,
};

use serde_json::{json, value::RawValue, Value};

use tokio::{sync::Notify, task::JoinHandle};

use windmill_queue::{add_completed_job, add_completed_job_error};

use crate::{
    bash_executor::ANSI_ESCAPE_RE,
    common::{read_result, save_in_cache},
    otel_oss::add_root_flow_job_to_otlp,
    worker_flow::update_flow_status_after_job_completion,
    JobCompletedReceiver, JobCompletedSender, SameWorkerSender, SendResult, SendResultPayload,
    UpdateFlow, SAME_WORKER_REQUIREMENTS,
};
use windmill_common::client::AuthedClient;

#[derive(Debug, Deserialize)]
struct ErrorMessage {
    message: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct NestedErrorMessage {
    error: ErrorMessage,
}

/// Extract `{ name, message }` from a result. Accepts both the standard
/// top-level shape (regular runtime errors) and the nested `{ error: { name,
/// message }, ... }` shape produced by the wm_failure injection.
///
/// For wm_failure-injected results, we prefer the nested error: a successful
/// run may legitimately contain top-level `name`/`message` fields (user data
/// named `name`/`message`), and we want OTel to record the ManualFailure
/// rather than the user's sibling fields.
fn extract_error_message(raw: &str) -> Option<ErrorMessage> {
    let nested = serde_json::from_str::<NestedErrorMessage>(raw)
        .ok()
        .map(|n| n.error);
    if matches!(&nested, Some(em) if em.name == MANUAL_FAILURE_ERROR_NAME) {
        return nested;
    }
    if let Ok(em) = serde_json::from_str::<ErrorMessage>(raw) {
        return Some(em);
    }
    nested
}

/// Returns the post-processing `success` value (after any `wm_failure`
/// override). Callers use this to make worker-loop decisions that depend on
/// whether the job ultimately succeeded — e.g. the init-script killpill.
async fn process_jc(
    mut jc: JobCompleted,
    worker_name: &str,
    base_internal_url: &str,
    db: &DB,
    worker_dir: &str,
    same_worker_tx: Option<&SameWorkerSender>,
    job_completed_sender: &JobCompletedSender,
    stats_map: &JobStatsMap,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    #[cfg(feature = "benchmark")] bench: &mut BenchmarkIter,
    #[cfg(feature = "benchmark")] bench_infos: &mut BenchmarkInfo,
) -> bool {
    // Parse `wm_labels` and `wm_failure` together (single `from_str`)
    // so we don't deserialize the whole result twice on every job.
    let metadata = jc.result.result_metadata();

    // If the script returned a `wm_failure: <string>` field in its
    // result, tag the run as a failure. Inject an `error: { name, message }`
    // at the top level so error handlers / UI / OTel see the standard error
    // shape, while preserving sibling fields (`windmill_status_code`,
    // `windmill_content_type`, `windmill_headers`, the user's data) at the
    // top level so sync webhook responses still honor them.
    if jc.success {
        if let Some(failure_msg) = metadata.wm_failure.as_ref() {
            if let Ok(Value::Object(mut map)) = serde_json::from_str::<Value>(jc.result.get()) {
                map.insert(
                    "error".to_string(),
                    json!({ "name": MANUAL_FAILURE_ERROR_NAME, "message": failure_msg }),
                );
                if let Ok(raw) = serde_json::value::to_raw_value(&Value::Object(map)) {
                    jc.result = Arc::new(raw);
                }
            }
            jc.success = false;
        }
    }

    let success: bool = jc.success;

    let span = if success {
        tracing::span!(
            tracing::Level::INFO,
            "job_postprocessing",
            job_id = %jc.job.id, root_job = field::Empty, workspace_id = %jc.job.workspace_id,  worker = %worker_name,tag = %jc.job.tag,
            // hostname = %hostname,
            language = field::Empty,
            script_path = field::Empty,
            flow_step_id = field::Empty,
            parent_job = field::Empty,
            job_kind = %jc.job.kind.as_str(),
            created_by = %jc.job.created_by,
            trigger_kind = field::Empty,
            trigger = field::Empty,
            script_hash = field::Empty,
            otel.name = field::Empty,
            success = %success,
            labels = field::Empty,
        )
    } else {
        tracing::span!(
            tracing::Level::INFO,
            "job_postprocessing",
            job_id = %jc.job.id, root_job = field::Empty, workspace_id = %jc.job.workspace_id,  worker = %worker_name,tag = %jc.job.tag,
            // hostname = %hostname,
            language = field::Empty,
            script_path = field::Empty,
            flow_step_id = field::Empty,
            parent_job = field::Empty,
            job_kind = %jc.job.kind.as_str(),
            created_by = %jc.job.created_by,
            trigger_kind = field::Empty,
            trigger = field::Empty,
            script_hash = field::Empty,
            otel.name = field::Empty,
            otel.status_code = "ERROR",
            otel.status_message = field::Empty,
            success = %success,
            error.message = field::Empty,
            error.name = field::Empty,
            labels = field::Empty,
        )
    };
    let rj = if let Some(root_job) = jc.job.flow_innermost_root_job {
        root_job
    } else {
        jc.job.id
    };

    if let Some(labels) = metadata.wm_labels.as_ref() {
        if !labels.is_empty() {
            span.record("labels", labels.join(","));
        }
    }
    // The secondary `job_postprocessing` span stays on the UUID-derived context
    // (MiniCompletedJob carries no args, so the inbound traceparent isn't
    // available here); the primary job span is relocated in `create_span_with_name`.
    windmill_common::otel_oss::set_span_parent(&span, &rj);

    if let Some(lg) = jc.job.script_lang.as_ref() {
        span.record("language", lg.as_str());
    }
    if let Some(step_id) = jc.job.flow_step_id.as_ref() {
        span.record(
            "otel.name",
            format!("job_postprocessing {}", step_id).as_str(),
        );
        span.record("flow_step_id", step_id.as_str());
    } else {
        span.record("otel.name", "job postprocessing");
    }
    if let Some(parent_job) = jc.job.parent_job.as_ref() {
        span.record("parent_job", parent_job.to_string().as_str());
    }
    if let Some(script_path) = jc.job.runnable_path.as_ref() {
        span.record("script_path", script_path.as_str());
    }
    if let Some(root_job) = jc.job.flow_innermost_root_job.as_ref() {
        span.record("root_job", root_job.to_string().as_str());
    }
    if let Some(trigger_kind) = jc.job.trigger_kind.as_ref() {
        span.record("trigger_kind", trigger_kind.to_string().as_str());
    }
    if let Some(trigger) = jc.job.trigger.as_ref() {
        span.record("trigger", trigger.as_str());
    }
    if let Some(script_hash) = jc.job.runnable_id.as_ref() {
        span.record("script_hash", script_hash.to_string().as_str());
    }
    if !success {
        if let Some(result_error) = extract_error_message(jc.result.get()) {
            span.record("error.message", result_error.message.as_str());
            span.record("error.name", result_error.name.as_str());
            span.record(
                "otel.status_message",
                crate::worker::truncate_description(&result_error.message).as_str(),
            );
        } else {
            span.record("otel.status_message", "Job failed");
        }
    }

    // Extract stats info before moving jc
    let duration_ms = jc.duration.clone();
    let script_lang = jc.job.script_lang.clone();
    let workspace_id = jc.job.workspace_id.clone();

    let root_job = handle_receive_completed_job(
        jc,
        &base_internal_url,
        &db,
        worker_dir,
        same_worker_tx,
        &worker_name,
        job_completed_sender.clone(),
        killpill_rx,
        #[cfg(feature = "benchmark")]
        bench,
    )
    .instrument(span)
    .warn_after_seconds(10)
    .await;

    if let Some(root_job) = root_job {
        add_root_flow_job_to_otlp(&root_job, success);

        #[cfg(feature = "benchmark")]
        if bench_infos.count_top_level(root_job.id) {
            bench_infos
                .shared_iters
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    // Accumulate job stats if duration is available
    if let Some(duration_ms) = duration_ms {
        accumulate_job_stats(
            stats_map,
            &*WORKER_GROUP,
            script_lang,
            &workspace_id,
            duration_ms,
        )
        .await;
    }

    success
}

enum JobCompletedRx {
    JobCompleted(SendResult),
    Killpill,
    WakeUp,
}

pub fn start_background_processor(
    job_completed_rx: JobCompletedReceiver,
    job_completed_sender: JobCompletedSender,
    same_worker_queue_size: Arc<AtomicU16>,
    job_completed_processor_is_done: Arc<AtomicBool>,
    wake_up_notify: Arc<Notify>,
    last_processing_duration: Arc<AtomicU16>,
    base_internal_url: String,
    db: DB,
    worker_dir: String,
    same_worker_tx: SameWorkerSender,
    worker_name: String,
    killpill_tx: KillpillSender,
    is_dedicated_worker: bool,
    stats_map: JobStatsMap,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut has_been_killed = false;

        let JobCompletedReceiver { bounded_rx, mut killpill_rx, unbounded_rx } = job_completed_rx;

        #[cfg(feature = "benchmark")]
        let mut infos = BenchmarkInfo::new(windmill_common::bench::shared_bench_iters());

        // Start periodic stats flush task
        let db_clone = db.clone();
        let stats_map_clone = stats_map.clone();
        let mut killpill_rx_clone = killpill_rx.resubscribe();
        let flush_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(900)); // Flush every 15 min

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = flush_stats_to_db(&db_clone, &stats_map_clone).await {
                            tracing::error!("Failed to flush worker group job stats: {}", e);
                        }
                    }
                    _ = killpill_rx_clone.recv() => {
                        tracing::info!("bg processor received killpill signal, flushing remaining stats");
                        break;
                    }
                }
            }
        });

        //if we have been killed, we want to drain the queue of jobs
        while let Some(sr) = {
            if has_been_killed {
                tracing::info!("bg processor is killed, draining. same_worker_queue_size: {}, unbounded_rx: {}, bounded_rx: {}", same_worker_queue_size.load(Ordering::SeqCst), unbounded_rx.len(), bounded_rx.len())
            }
            if has_been_killed && same_worker_queue_size.load(Ordering::SeqCst) == 0 {
                unbounded_rx
                    .try_recv()
                    .ok()
                    .map(JobCompletedRx::JobCompleted)
                    .or_else(|| bounded_rx.try_recv().ok().map(JobCompletedRx::JobCompleted))
            } else {
                tokio::select! {
                    biased;
                    result = unbounded_rx.recv_async()  => {
                        result.ok().map(JobCompletedRx::JobCompleted)
                    }
                    result = bounded_rx.recv_async() => {
                        result.ok().map(JobCompletedRx::JobCompleted)
                    },
                    _ = wake_up_notify.notified() => {
                        tracing::info!("bg processor received wake up signal, checking if same worker queue is empty");
                        Some(JobCompletedRx::WakeUp)
                    },
                    _ = killpill_rx.recv() => {
                        tracing::info!("bg processor received killpill signal, queuing killpill job");
                        Some(JobCompletedRx::Killpill)
                    }
                }
            }
        } {
            #[cfg(feature = "benchmark")]
            let mut bench = BenchmarkIter::new();

            match sr {
                JobCompletedRx::JobCompleted(SendResult {
                    result: SendResultPayload::JobCompleted(jc),
                    time,
                }) => {
                    let is_init_script = jc.job.tag.as_str() == INIT_SCRIPT_TAG;
                    let is_dependency_job = matches!(
                        jc.job.kind,
                        JobKind::Dependencies | JobKind::FlowDependencies
                    );
                    #[cfg(feature = "benchmark")]
                    let bench_job_id = jc.job.id;
                    #[cfg(feature = "benchmark")]
                    let is_top_level_job = jc.job.parent_job.is_none();

                    // process_jc returns the post-override success value so a
                    // job that flipped to failure via `wm_failure` still
                    // triggers the init-script killpill.
                    let final_success = process_jc(
                        jc,
                        &worker_name,
                        &base_internal_url,
                        &db,
                        &worker_dir,
                        Some(&same_worker_tx),
                        &job_completed_sender,
                        &stats_map,
                        &killpill_rx,
                        #[cfg(feature = "benchmark")]
                        &mut bench,
                        #[cfg(feature = "benchmark")]
                        &mut infos,
                    )
                    .warn_after_seconds(10)
                    .await;

                    if is_init_script && !final_success {
                        tracing::error!("init script errored, exiting");
                        killpill_tx.send();
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
                        killpill_tx.send();
                    }
                    add_time!(bench, "job completed processed");

                    #[cfg(feature = "benchmark")]
                    {
                        if infos.add_iter(bench, bench_job_id, is_top_level_job) {
                            infos.shared_iters.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    last_processing_duration
                        .store(time.elapsed().as_secs() as u16, Ordering::SeqCst);
                }
                JobCompletedRx::JobCompleted(SendResult {
                    result:
                        SendResultPayload::UpdateFlow(UpdateFlow {
                            flow,
                            w_id,
                            success,
                            result,
                            worker_dir,
                            stop_early_override,
                            token,
                        }),
                    time,
                }) => {
                    // let r;
                    tracing::info!(parent_flow = %flow, "updating flow status after job completion");
                    if let Err(e) = update_flow_status_after_job_completion(
                        &db,
                        &AuthedClient::new(
                            base_internal_url.to_string(),
                            w_id.clone(),
                            token.clone(),
                            None,
                        ),
                        flow,
                        &Uuid::nil(),
                        &w_id,
                        success,
                        None,
                        Arc::new(result),
                        None,
                        true,
                        &same_worker_tx,
                        &worker_dir,
                        stop_early_override,
                        &worker_name,
                        job_completed_sender.clone(),
                        None,
                        &killpill_rx,
                        #[cfg(feature = "benchmark")]
                        &mut bench,
                    )
                    .await
                    {
                        tracing::error!("Error updating flow status after job completion for {flow} on {worker_name}: {e:#}");
                    }
                    #[cfg(feature = "benchmark")]
                    {
                        if infos.add_iter(bench, flow, true) {
                            infos.shared_iters.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    last_processing_duration
                        .store(time.elapsed().as_secs() as u16, Ordering::SeqCst);
                }
                JobCompletedRx::Killpill => {
                    tracing::info!("killpill job received, processing only same worker jobs");
                    has_been_killed = true;
                }
                JobCompletedRx::WakeUp => {}
            }
        }

        // Flush any remaining stats before shutting down
        tracing::info!("flushing remaining stats before shutting down");
        let flush_result =
            tokio::time::timeout(std::time::Duration::from_secs(10), flush_handle).await;
        match flush_result {
            Ok(Ok(())) => tracing::info!("Stats flushed successfully"),
            Ok(Err(join_err)) => tracing::error!("Stats flush task failed: {}", join_err),
            Err(_) => tracing::error!("Stats flush timed out after 10 seconds"),
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

async fn send_job_completed(job_completed_tx: JobCompletedSender, jc: JobCompleted) {
    if let Err(e) = job_completed_tx
        .send_job(jc, true)
        .with_context(windmill_common::otel_oss::otel_ctx())
        .await
    {
        tracing::error!("send job completed failed, triggering worker shutdown: {e:#}");
        job_completed_tx.send_worker_killpill();
    }
}

pub async fn process_result(
    job: MiniCompletedJob,
    result: error::Result<Arc<Box<RawValue>>>,
    job_dir: &str,
    job_completed_tx: JobCompletedSender,
    mem_peak: i32,
    canceled_by: Option<CanceledBy>,
    cached_res_path: Option<String>,
    token: &str,
    result_columns: Option<Vec<String>>,
    preprocessed_args: Option<HashMap<String, Box<RawValue>>>,
    conn: &Connection,
    duration: Option<i64>,
    has_stream: bool,
    flow_runners: Option<Arc<FlowRunners>>,
) -> error::Result<crate::worker::JobOutcome> {
    match result {
        Ok(result) => {
            send_job_completed(
                job_completed_tx,
                JobCompleted {
                    job,
                    preprocessed_args,
                    result,
                    result_columns,
                    mem_peak,
                    canceled_by,
                    success: true,
                    cached_res_path,
                    token: token.to_string(),
                    duration,
                    has_stream: Some(has_stream),
                    from_cache: None,
                    flow_runners,
                    done_tx: None,
                },
            )
            .with_context(windmill_common::otel_oss::otel_ctx())
            .await;
            Ok(crate::worker::JobOutcome::Completed)
        }
        Err(e) => {
            let error_value = match e {
                Error::ExitStatus(program, i) => {
                    let res = read_result(job_dir, None).await.ok();

                    if res.as_ref().is_some_and(|x| !x.get().is_empty()) {
                        res.unwrap()
                    } else {
                        match conn {
                            Connection::Sql(db) => {
                                let last_10_log_lines = sqlx::query_scalar!(
                            "SELECT right(logs, 600) FROM job_logs WHERE job_id = $1 AND workspace_id = $2 ORDER BY created_at DESC LIMIT 1",
                            &job.id,
                            &job.workspace_id
                        ).fetch_one(db).await.ok().flatten().unwrap_or("".to_string());

                                let log_lines = last_10_log_lines
                                    .split("CODE EXECUTION ---")
                                    .last()
                                    .unwrap_or(&last_10_log_lines);

                                extract_error_value(
                                    &program,
                                    log_lines,
                                    i,
                                    job.flow_step_id.clone(),
                                )
                            }
                            Connection::Http(_) => {
                                to_raw_value(&"See logs for more details".to_string())
                            }
                        }
                    }
                }
                Error::ExecutionRawError(e) => to_raw_value(&e),
                err @ _ => to_raw_value(&SerializedError {
                    message: format!("execution error:\n{err:#}",),
                    name: "ExecutionErr".to_string(),
                    step_id: job.flow_step_id.clone(),
                    exit_code: None,
                }),
            };

            // Use the structured error message that was just extracted (the
            // user-facing script error) rather than the generic Error string.
            // Pull `.message` out of the JSON object if present, otherwise
            // accept a bare string (e.g. agent-worker "See logs for more
            // details"), and only fall back to "Job failed" when the value
            // carries no readable description.
            let description = serde_json::from_str::<serde_json::Value>(error_value.get())
                .ok()
                .and_then(|value| {
                    value
                        .get("message")
                        .and_then(|m| m.as_str())
                        .or_else(|| value.as_str())
                        .map(|m| crate::worker::truncate_description(m))
                })
                .unwrap_or_else(|| "Job failed".to_string());

            send_job_completed(
                job_completed_tx,
                JobCompleted {
                    job,
                    result: Arc::new(to_raw_value(&error_value)),
                    result_columns: None,
                    preprocessed_args: None,
                    mem_peak,
                    canceled_by,
                    success: false,
                    cached_res_path,
                    token: token.to_string(),
                    duration,
                    has_stream: Some(has_stream),
                    from_cache: None,
                    flow_runners,
                    done_tx: None,
                },
            )
            .with_context(windmill_common::otel_oss::otel_ctx())
            .await;
            Ok(crate::worker::JobOutcome::Failed { description })
        }
    }
}

pub async fn handle_receive_completed_job(
    jc: JobCompleted,
    base_internal_url: &str,
    db: &DB,
    worker_dir: &str,
    same_worker_tx: Option<&SameWorkerSender>,
    worker_name: &str,
    job_completed_tx: JobCompletedSender,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    #[cfg(feature = "benchmark")] bench: &mut BenchmarkIter,
) -> Option<Arc<MiniPulledJob>> {
    let token = jc.token.clone();
    let workspace = jc.job.workspace_id.clone();
    let client = AuthedClient::new(base_internal_url.to_string(), workspace, token, None);
    let job = jc.job.clone();
    let mem_peak = jc.mem_peak.clone();
    let canceled_by = jc.canceled_by.clone();

    let processed_completed_job = process_completed_job(
        jc,
        &client,
        db,
        &worker_dir,
        same_worker_tx.clone(),
        worker_name,
        job_completed_tx.clone(),
        killpill_rx,
        #[cfg(feature = "benchmark")]
        bench,
    )
    .warn_after_seconds(10)
    .await;

    match processed_completed_job {
        // The job was already completed by another worker (e.g. this worker was
        // declared a zombie and the job restarted+finished elsewhere, then this
        // worker caught up). The job genuinely succeeded; routing this through
        // `handle_job_error` would propagate a spurious "AlreadyCompleted"
        // failure up the parent flow. Drop it instead, mirroring the
        // `JobOutcome::AlreadyCompleted` guard on the execution path.
        Err(err @ Error::AlreadyCompleted(_)) => {
            tracing::info!(
                job_id = %job.id,
                "job already completed by another worker, skipping result processing: {err:#}"
            );
            None
        }
        Err(err) => {
            handle_job_error(
                db,
                &client,
                &job,
                mem_peak,
                canceled_by,
                err,
                false,
                same_worker_tx.clone(),
                &worker_dir,
                worker_name,
                job_completed_tx,
                killpill_rx,
                #[cfg(feature = "benchmark")]
                bench,
            )
            .await;
            None
        }
        Ok(r) => r,
    }
}

/// A git-sync check run threaded through a pull job: the PR diff preview (phase 4)
/// or the live deploy status (phase 6). Both markers carry the same shape.
#[cfg(all(feature = "enterprise", feature = "private"))]
#[derive(serde::Deserialize)]
struct GitSyncCheck {
    check_run_id: i64,
    repo_url: String,
    #[serde(default)]
    pr_number: Option<i64>,
    #[serde(default)]
    head_sha: Option<String>,
    /// Whether the PR itself modifies wmill.yaml (None = undetermined); picks
    /// the wording for a settings difference in the diff summary.
    #[serde(default)]
    wmill_yaml_changed: Option<bool>,
}

/// Parsed diff summary from a (dry-run or real) pull result. `None` when the
/// result can't be parsed into the expected shape.
#[cfg(all(feature = "enterprise", feature = "private"))]
fn parse_git_sync_changes(result_raw: &str) -> Option<(Vec<(String, String)>, bool)> {
    use serde::Deserialize;
    #[derive(Deserialize)]
    struct Change {
        #[serde(rename = "type")]
        change_type: String,
        path: String,
    }
    #[derive(Deserialize)]
    struct SettingsDiff {
        #[serde(rename = "hasChanges", default)]
        has_changes: bool,
    }
    #[derive(Deserialize)]
    struct SyncResponse {
        changes: Option<Vec<Change>>,
        #[serde(rename = "settingsDiffResult")]
        settings_diff_result: Option<SettingsDiff>,
    }
    let resp = serde_json::from_str::<SyncResponse>(result_raw).ok()?;
    // A result carrying neither field isn't a recognizable diff; return None so the
    // caller falls back to the unsummarized path instead of a false "in sync".
    if resp.changes.is_none() && resp.settings_diff_result.is_none() {
        return None;
    }
    let settings_changed = resp
        .settings_diff_result
        .map(|s| s.has_changes)
        .unwrap_or(false);
    Some((
        resp.changes
            .unwrap_or_default()
            .into_iter()
            .map(|c| (c.change_type, c.path))
            .collect(),
        settings_changed,
    ))
}

#[cfg(all(feature = "enterprise", feature = "private"))]
fn format_change_list(changes: &[(String, String)]) -> Vec<String> {
    let mut lines = Vec::new();
    for (change_type, path) in changes.iter().take(100) {
        lines.push(format!("- `{}` {}", change_type, path));
    }
    if changes.len() > 100 {
        lines.push(format!("- ... and {} more", changes.len() - 100));
    }
    lines
}

#[cfg(all(test, feature = "enterprise", feature = "private"))]
mod git_sync_check_tests {
    use super::{format_change_list, parse_git_sync_changes};

    #[test]
    fn parse_empty_changes_is_in_sync() {
        // Present-but-empty diff → a real "in sync" result, not None.
        let (changes, settings) = parse_git_sync_changes(r#"{"changes":[]}"#).unwrap();
        assert!(changes.is_empty());
        assert!(!settings);
    }

    #[test]
    fn parse_missing_fields_is_none() {
        // Neither field present → unrecognizable, falls back to the caller's path.
        assert!(parse_git_sync_changes("{}").is_none());
    }

    #[test]
    fn parse_unparseable_is_none() {
        assert!(parse_git_sync_changes("not json").is_none());
    }

    #[test]
    fn parse_changes_and_settings() {
        let (changes, settings) = parse_git_sync_changes(
            r#"{"changes":[{"type":"edited","path":"f/a"}],"settingsDiffResult":{"hasChanges":true}}"#,
        )
        .unwrap();
        assert_eq!(changes, vec![("edited".to_string(), "f/a".to_string())]);
        assert!(settings);
    }

    #[test]
    fn format_truncates_over_100() {
        let changes: Vec<(String, String)> = (0..150)
            .map(|i| ("edited".to_string(), format!("f/{i}")))
            .collect();
        let lines = format_change_list(&changes);
        assert_eq!(lines.len(), 101);
        assert_eq!(lines.last().unwrap(), "- ... and 50 more");
    }
}

/// When an auto-pull job (carrying `__git_sync_auto_pull`) fails, roll the
/// optimistic `last_synced_sha` advance back to the pre-pull value so the commit
/// is retried instead of being silently treated as synced, and record the failure.
#[cfg(all(feature = "enterprise", feature = "private"))]
async fn maybe_reconcile_git_sync_auto_pull(
    db: &DB,
    job_id: &uuid::Uuid,
    workspace_id: &str,
    success: bool,
) {
    if success {
        return; // the optimistic synced state is already correct
    }
    let marker: Option<serde_json::Value> = match sqlx::query_scalar!(
        "SELECT args->'__git_sync_auto_pull' FROM v2_job WHERE id = $1",
        job_id
    )
    .fetch_optional(db)
    .await
    {
        Ok(v) => v.flatten(),
        Err(e) => {
            tracing::error!("git auto-pull: failed to read job args: {e:#}");
            return;
        }
    };
    let Some(marker) = marker else {
        return;
    };
    #[derive(serde::Deserialize)]
    struct AutoPullMarker {
        repo_resource_path: String,
        #[serde(default)]
        prev_synced: std::collections::HashMap<String, String>,
    }
    let Ok(m) = serde_json::from_value::<AutoPullMarker>(marker) else {
        return;
    };
    windmill_git_sync::record_auto_pull_failure(
        db,
        workspace_id,
        &m.repo_resource_path,
        &m.prev_synced,
        "auto-pull job failed".to_string(),
    )
    .await;
}

/// Branch a git-sync push job deployed to, mirroring the hub script's
/// derivation: a dev workspace deploys to its environment-label branch
/// (`dev`/`staging`), other fork workspaces to `wm-fork/<base>/<id-suffix>`,
/// else the promotion `wm_deploy/**` formula (per-folder or per-item form).
/// `None` when the deploy stays on the base branch (workspace-wide mode) and
/// has no PR to open.
#[cfg(all(feature = "enterprise", feature = "private"))]
fn git_sync_deploy_pr_head_branch(
    workspace_id: &str,
    parent_workspace_id: Option<&str>,
    dev_workspace_label: Option<&str>,
    base: &str,
    use_individual_branch: bool,
    group_by_folder: bool,
    item_path: &str,
    item_parent_path: &str,
    path_type: &str,
) -> Option<String> {
    if dev_workspace_label.is_some() {
        return Some(windmill_common::workspaces::dev_workspace_branch(
            dev_workspace_label,
        ));
    }
    let is_fork = parent_workspace_id.is_some()
        || workspace_id.starts_with(windmill_common::workspaces::WM_FORK_PREFIX);
    if is_fork {
        let suffix = workspace_id
            .strip_prefix(windmill_common::workspaces::WM_FORK_PREFIX)
            .unwrap_or(workspace_id);
        return Some(format!("wm-fork/{base}/{suffix}"));
    }
    if !use_individual_branch {
        return None;
    }
    let git_ref = if !item_path.is_empty() {
        item_path
    } else {
        item_parent_path
    };
    if git_ref.is_empty() {
        return None;
    }
    Some(if group_by_folder {
        format!(
            "wm_deploy/{workspace_id}/{}",
            git_ref.split('/').take(2).collect::<Vec<_>>().join("__")
        )
    } else {
        format!(
            "wm_deploy/{workspace_id}/{}/{}",
            path_type,
            git_ref.replace('/', "__")
        )
    })
}

/// When a git-sync push job carrying `__git_sync_open_pr` succeeds, open (or
/// reopen) the PR for the branch it pushed: `wm-fork/<base>/<id>` for a fork
/// deploy, `wm_deploy/**` for a promotion deploy. Runs outbound with the
/// installation token, so it works regardless of webhook reachability.
/// Best-effort: failures are logged, never propagated.
#[cfg(all(feature = "enterprise", feature = "private"))]
async fn maybe_open_git_sync_deploy_pr(db: &DB, job_id: &uuid::Uuid, workspace_id: &str) {
    let row = match sqlx::query!(
        r#"SELECT
            args->'__git_sync_open_pr' as "marker",
            args->>'repo_url_resource_path' as "repo_path",
            args->>'parent_workspace_id' as "parent_workspace_id",
            args->>'dev_workspace_label' as "dev_workspace_label",
            args->>'parent_dev_workspace_label' as "parent_dev_workspace_label",
            COALESCE((args->'use_individual_branch')::bool, false) as "use_individual_branch!",
            COALESCE((args->'group_by_folder')::bool, false) as "group_by_folder!",
            COALESCE(args->'items'->0->>'path', args->>'path', '') as "item_path!",
            COALESCE(args->'items'->0->>'parent_path', args->>'parent_path', '') as "item_parent_path!",
            COALESCE(args->'items'->0->>'path_type', args->>'path_type', '') as "path_type!",
            COALESCE(args->'items'->0->>'commit_msg', args->>'commit_msg', '') as "commit_msg!"
        FROM v2_job WHERE id = $1"#,
        job_id
    )
    .fetch_optional(db)
    .await
    {
        Ok(Some(r)) => r,
        Ok(None) => return,
        Err(e) => {
            tracing::error!("git sync PR: failed to read job args: {e:#}");
            return;
        }
    };
    if row.marker.is_none() {
        return;
    }
    let Some(repo_path) = row.repo_path else {
        return;
    };

    // Base = the tracked branch (resource branch, else the repo default). Also
    // acts as the app-backed gate: PR creation needs the installation token.
    let base = match windmill_common::git_sync_ee::get_app_repo_head_for_autopull(
        db,
        workspace_id,
        &repo_path,
    )
    .await
    {
        Ok(Some((branch, _))) => branch,
        Ok(None) => {
            tracing::warn!(
                "git sync PR: repo {repo_path} in {workspace_id} has a PR-on-deploy toggle set but is not GitHub-App-backed; skipping (connect the repo through the GitHub App, or use the open-pr-on-commit workflow)"
            );
            return;
        }
        Err(e) => {
            tracing::warn!("git sync PR: could not resolve base branch for {repo_path}: {e:#}");
            return;
        }
    };

    let Some(head) = git_sync_deploy_pr_head_branch(
        workspace_id,
        row.parent_workspace_id.as_deref(),
        row.dev_workspace_label.as_deref(),
        &base,
        row.use_individual_branch,
        row.group_by_folder,
        &row.item_path,
        &row.item_parent_path,
        &row.path_type,
    ) else {
        return;
    };

    let repo_url =
        match windmill_common::git_sync_ee::resolve_repo_url(db, workspace_id, &repo_path).await {
            Ok(url) => url,
            Err(e) => {
                tracing::warn!("git sync PR: could not resolve repo url for {repo_path}: {e:#}");
                return;
            }
        };
    // A fork of a dev workspace diverged from the dev's label branch, so its PR
    // merges back there; everything else targets the tracked branch.
    let pr_base = row.parent_dev_workspace_label.as_deref().unwrap_or(&base);
    if let Err(e) = windmill_common::git_sync_ee::ensure_pull_request(
        db,
        workspace_id,
        &repo_url,
        &head,
        pr_base,
        &row.commit_msg,
    )
    .await
    {
        tracing::warn!("git sync PR: failed to open PR {head} -> {pr_base} for {repo_path}: {e:#}");
    }
}

/// When a git-sync pull job carrying a check marker completes, post the outcome
/// to its GitHub check run: the PR diff preview (`__git_sync_pr_check`, phase 4)
/// or the live deploy status (`__git_sync_deploy_check`, phase 6).
#[cfg(all(feature = "enterprise", feature = "private"))]
async fn maybe_post_git_sync_check(
    db: &DB,
    job_id: &uuid::Uuid,
    workspace_id: &str,
    success: bool,
    result_raw: &str,
) {
    // Only git-sync pull jobs carry one of these markers; everything else no-ops.
    let row = match sqlx::query!(
        r#"SELECT args->'__git_sync_pr_check' AS "pr", args->'__git_sync_deploy_check' AS "deploy"
           FROM v2_job WHERE id = $1"#,
        job_id
    )
    .fetch_optional(db)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("git sync-check: failed to read job args: {e:#}");
            return;
        }
    };
    let Some(row) = row else {
        return;
    };
    // A PR dry-run and a deploy pull are mutually exclusive markers.
    let (is_deploy, marker) = match (row.pr, row.deploy) {
        (Some(pr), _) => (false, pr),
        (None, Some(deploy)) => (true, deploy),
        (None, None) => return,
    };
    let Ok(check) = serde_json::from_value::<GitSyncCheck>(marker) else {
        return;
    };

    let (conclusion, title, summary): (&str, String, String) = if is_deploy {
        // Phase 6: real deploy pull -> "Deployed N changes" / "In sync" / failure.
        if !success {
            (
                "failure",
                format!("Deploy to {} failed", workspace_id),
                "Deploying the latest commit failed. See the job in Windmill for details."
                    .to_string(),
            )
        } else {
            match parse_git_sync_changes(result_raw) {
                Some((changes, settings_changed)) if changes.is_empty() && !settings_changed => (
                    "success",
                    format!("In sync with {}", workspace_id),
                    format!(
                        "No changes to deploy to `{}` from this commit.",
                        workspace_id
                    ),
                ),
                Some((changes, settings_changed)) => {
                    let mut lines = vec![format!(
                        "Deployed {} change(s) to `{}`:\n",
                        changes.len(),
                        workspace_id
                    )];
                    lines.extend(format_change_list(&changes));
                    if settings_changed {
                        lines.push("\nWorkspace settings also changed.".to_string());
                    }
                    (
                        "success",
                        format!("Deployed {} change(s) to {}", changes.len(), workspace_id),
                        lines.join("\n"),
                    )
                }
                None => (
                    "success",
                    format!("Deployed to {}", workspace_id),
                    format!("Windmill deployed the latest commit to `{}`.", workspace_id),
                ),
            }
        }
    } else {
        // Phase 4: dry-run diff preview for a PR.
        if !success {
            (
                "failure",
                "Windmill diff failed".to_string(),
                "The dry-run pull to compute the diff failed. See the job in Windmill for details."
                    .to_string(),
            )
        } else {
            match parse_git_sync_changes(result_raw) {
                Some((changes, settings_changed)) if changes.is_empty() && !settings_changed => (
                    "success",
                    "In sync".to_string(),
                    "Merging this PR would make no changes to the workspace.".to_string(),
                ),
                Some((changes, settings_changed)) => {
                    let mut lines = vec![format!(
                        "Merging this PR would apply {} change(s) to the workspace:\n",
                        changes.len()
                    )];
                    lines.extend(format_change_list(&changes));
                    if settings_changed {
                        lines.push(match check.wmill_yaml_changed {
                            Some(true) => "\nThis PR changes wmill.yaml: pulling also applies the updated workspace settings.".to_string(),
                            Some(false) => "\nIndependent of this PR, the workspace's git-sync settings differ from the repo's wmill.yaml and a pull updates them to match.".to_string(),
                            None => "\nA pull also updates the workspace's git-sync settings to match the repo's wmill.yaml.".to_string(),
                        });
                    }
                    (
                        "neutral",
                        format!("{} change(s) to deploy", changes.len()),
                        lines.join("\n"),
                    )
                }
                None => (
                    "neutral",
                    "Diff computed".to_string(),
                    "Windmill computed a diff but could not summarize it.".to_string(),
                ),
            }
        }
    };

    if let Err(e) = windmill_common::git_sync_ee::update_check_run(
        db,
        workspace_id,
        &check.repo_url,
        check.check_run_id,
        conclusion,
        &title,
        &summary,
    )
    .await
    {
        tracing::error!("git sync-check: failed to update check run: {e:#}");
    }

    // Phase 4 also maintains ONE managed comment on the PR (Cloudflare
    // deploy-preview style): upserted on every synchronize, so reviewers see the
    // current diff without opening the Checks tab.
    if !is_deploy {
        if let Some(pr_number) = check.pr_number {
            let marker = "<!-- windmill-diff -->";
            let head = check
                .head_sha
                .as_deref()
                .map(|s| &s[..s.len().min(7)])
                .unwrap_or("latest");
            let body = format!(
                "{marker}\n### Windmill deploy preview\n\n                 | | |\n|---|---|\n                 | **Workspace** | `{workspace_id}` |\n                 | **Status** | {title} |\n                 | **Commit** | `{head}` |\n\n                 <details><summary>Details</summary>\n\n{summary}\n\n</details>"
            );
            if let Err(e) = windmill_common::git_sync_ee::upsert_pr_comment(
                db,
                workspace_id,
                &check.repo_url,
                pr_number,
                marker,
                &body,
            )
            .await
            {
                tracing::warn!("git sync-check: failed to upsert PR diff comment: {e:#}");
            }
        }
    }
}

pub async fn process_completed_job(
    JobCompleted {
        job,
        result,
        mem_peak,
        success,
        cached_res_path,
        canceled_by,
        duration,
        result_columns,
        preprocessed_args,
        from_cache,
        flow_runners,
        done_tx,
        ..
    }: JobCompleted,
    client: &AuthedClient,
    db: &DB,
    worker_dir: &str,
    same_worker_tx: Option<&SameWorkerSender>,
    worker_name: &str,
    job_completed_tx: JobCompletedSender,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    #[cfg(feature = "benchmark")] bench: &mut BenchmarkIter,
) -> error::Result<Option<Arc<MiniPulledJob>>> {
    if success {
        // println!("bef completed job{:?}",  SystemTime::now());
        if let Some(cached_path) = cached_res_path {
            save_in_cache(db, client, &job, cached_path, result.clone()).await;
        }

        let is_flow_step = job.is_flow_step();
        let parent_job = job.parent_job.clone();
        let job_id = job.id.clone();
        let workspace_id = job.workspace_id.clone();
        let started_at = job.started_at.clone();

        if job.flow_step_id.as_deref() == Some("preprocessor") {
            // Do this before inserting to `v2_job_completed` for backwards compatibility
            // when we set `flow_status->_metadata->preprocessed_args` to true.

            sqlx::query!(
                r#"UPDATE v2_job SET
                    args = '{"reason":"PREPROCESSOR_ARGS_ARE_DISCARDED"}'::jsonb,
                    preprocessed = TRUE
                WHERE id = $1 AND preprocessed = FALSE"#,
                job.id
            )
            .execute(db)
            .await
            .map_err(|e| {
                Error::InternalErr(format!(
                    "error while deleting args of preprocessing step: {e:#}"
                ))
            })?;
        } else if let Some(preprocessed_args) = preprocessed_args {
            // Update script args to preprocessed args, but preserve a
            // resolved pipeline `partition` (injected before the body ran
            // by resolve_partition_for_job). Run identity is immutable —
            // the preprocessor must not change or drop it, or the asset
            // cascade would read no partition for this producer.
            windmill_common::partition::merge_args_preserving_partition(
                db,
                job.id,
                preprocessed_args,
            )
            .await?;
        }

        add_time!(bench, "pre add_completed_job");

        let (_, duration, wac_job_ids) = add_completed_job(
            db,
            &job,
            true,
            false,
            Json(&result),
            result_columns,
            mem_peak.to_owned(),
            canceled_by.clone(),
            false,
            duration,
            from_cache.unwrap_or(false),
        )
        .await?;
        #[cfg(all(feature = "enterprise", feature = "private"))]
        if job.kind == JobKind::DeploymentCallback {
            maybe_post_git_sync_check(db, &job_id, &workspace_id, true, result.get()).await;
            maybe_open_git_sync_deploy_pr(db, &job_id, &workspace_id).await;
        }

        // Asset-trigger fan-out: best-effort, never propagates errors.
        // Internal eligibility checks gate to top-level Script/Preview runs;
        // see windmill_queue::asset_dispatch.
        asset_dispatch::dispatch_asset_triggers(db, &job).await;

        drop(job);

        add_time!(bench, "add_completed_job END");

        if is_flow_step {
            if let Some(parent_job) = parent_job {
                // tracing::info!(parent_flow = %parent_job, subflow = %job_id, "updating flow status (2)");
                let r = update_flow_status_after_job_completion(
                    db,
                    client,
                    parent_job,
                    &job_id,
                    &workspace_id,
                    true,
                    canceled_by,
                    result,
                    started_at.map(|x| FlowJobDuration { started_at: x, duration_ms: duration }),
                    false,
                    &same_worker_tx.expect(SAME_WORKER_REQUIREMENTS).to_owned(),
                    &worker_dir,
                    None,
                    worker_name,
                    job_completed_tx,
                    flow_runners,
                    killpill_rx,
                    #[cfg(feature = "benchmark")]
                    bench,
                )
                .warn_after_seconds(10)
                .await?;
                add_time!(bench, "updated flow status END");
                if let Some(done_tx) = done_tx {
                    done_tx
                        .send(())
                        .expect("done receiver should still be alive");
                }
                return Ok(r);
            }
        } else if let Some(parent_job) = parent_job {
            // wac_job_ids is piggybacked from the duration write in
            // add_completed_job — no extra query needed.
            if let Some(job_ids) = wac_job_ids {
                if let Ok(Some(_)) = handle_wac_child_completion(
                    db,
                    &job_id,
                    parent_job,
                    &workspace_id,
                    result,
                    true,
                    job_ids,
                )
                .await
                {
                    if let Some(done_tx) = done_tx {
                        done_tx
                            .send(())
                            .expect("done receiver should still be alive");
                    }
                    return Ok(None);
                }
            }
        }
    } else {
        // The result already carries our injected
        // `error: { name: "ManualFailure", ... }` marker when process_jc
        // retagged a successful run as a failure — store it as-is to preserve
        // sibling fields like `windmill_status_code`. We check for the
        // injected marker specifically (not just the presence of a
        // `wm_failure` field) so a real runtime failure whose raw
        // result happens to contain a `wm_failure` field still goes
        // through the standard `WrappedError { error: ... }` wrap path.
        let downstream_result: Arc<Box<RawValue>> = if is_pre_shaped_wm_failure_result(result.get())
        {
            windmill_queue::add_completed_job_pre_shaped_failure(
                db,
                &job,
                mem_peak.to_owned(),
                canceled_by.clone(),
                Json(&*result),
                worker_name,
                false,
                None,
            )
            .await?;
            result.clone()
        } else {
            let wrapped = add_completed_job_error(
                db,
                &job,
                mem_peak.to_owned(),
                canceled_by.clone(),
                serde_json::from_str(result.get()).unwrap_or_else(
                    |_| json!({ "message": format!("Non serializable error: {}", result.get()) }),
                ),
                worker_name,
                false,
                None,
            )
            .await?;
            Arc::new(serde_json::value::to_raw_value(&wrapped).unwrap())
        };
        #[cfg(all(feature = "enterprise", feature = "private"))]
        if job.kind == JobKind::DeploymentCallback {
            maybe_post_git_sync_check(db, &job.id, &job.workspace_id, false, result.get()).await;
            maybe_reconcile_git_sync_auto_pull(db, &job.id, &job.workspace_id, false).await;
        }
        if job.is_flow_step() {
            if let Some(parent_job) = job.parent_job {
                tracing::error!(parent_flow = %parent_job, subflow = %job.id, "process completed job error, updating flow status");
                let r = update_flow_status_after_job_completion(
                    db,
                    client,
                    parent_job,
                    &job.id,
                    &job.workspace_id,
                    false,
                    canceled_by,
                    downstream_result,
                    duration.and_then(|d| {
                        job.started_at.map(|started_at| FlowJobDuration {
                            started_at: started_at,
                            duration_ms: d,
                        })
                    }),
                    false,
                    &same_worker_tx.expect(SAME_WORKER_REQUIREMENTS).to_owned(),
                    &worker_dir,
                    None,
                    worker_name,
                    job_completed_tx,
                    flow_runners,
                    killpill_rx,
                    #[cfg(feature = "benchmark")]
                    bench,
                )
                .warn_after_seconds(10)
                .await?;
                if let Some(done_tx) = done_tx {
                    done_tx
                        .send(())
                        .expect("done receiver should still be alive");
                }
                return Ok(r);
            }
        } else if let Some(parent_job) = job.parent_job {
            // WAC child failed — query job_ids from parent (errors are rare,
            // so the extra read is acceptable here).
            let job_ids_json: Option<Option<Value>> = sqlx::query_scalar(
                "SELECT workflow_as_code_status->'_checkpoint'->'pending_steps'->'job_ids' \
                 FROM v2_job_status WHERE id = $1",
            )
            .bind(&parent_job)
            .fetch_optional(db)
            .await?;
            if let Some(Some(job_ids)) = job_ids_json {
                if let Ok(Some(_)) = handle_wac_child_completion(
                    db,
                    &job.id,
                    parent_job,
                    &job.workspace_id,
                    downstream_result,
                    false,
                    job_ids,
                )
                .await
                {
                    if let Some(done_tx) = done_tx {
                        done_tx
                            .send(())
                            .expect("done receiver should still be alive");
                    }
                    return Ok(None);
                }
            }
        }
    }
    return Ok(None);
}

/// Handle a WAC v2 child job completion.
/// Returns Ok(Some(())) if the parent was a WAC job and was handled,
/// Ok(None) if the parent is not a WAC job (caller should fall through).
///
/// CONCURRENCY: Multiple parallel children may complete simultaneously on
/// different workers.  We use atomic SQL operations throughout:
///   - `completed_steps` is merged via `jsonb_set(... || jsonb_build_object(...))`
///     — PostgreSQL serialises concurrent UPDATEs on the same row, so each
///     worker sees the previous worker's writes.
///   - The suspend counter (set to N at dispatch time) is decremented atomically
///     with `RETURNING` to determine the "all done" condition.
pub(crate) async fn handle_wac_child_completion(
    db: &DB,
    child_job_id: &Uuid,
    parent_job_id: Uuid,
    workspace_id: &str,
    result: Arc<Box<RawValue>>,
    success: bool,
    job_ids_value: Value,
) -> error::Result<Option<()>> {
    let job_ids = match job_ids_value {
        Value::Object(m) => m,
        _ => return Ok(None), // Not a WAC parent or no pending steps
    };

    let child_id_str = child_job_id.to_string();
    let step_key = job_ids.iter().find_map(|(key, val)| {
        if val.as_str() == Some(&child_id_str) {
            Some(key.clone())
        } else {
            None
        }
    });

    let step_key = match step_key {
        Some(k) => k,
        None => {
            if !success {
                // No step key and failed — can't store error, fail parent immediately
                tracing::error!(
                    parent_job = %parent_job_id,
                    child_job = %child_job_id,
                    "WAC v2 child job failed but no step key found, failing parent"
                );
                sqlx::query!(
                    "UPDATE v2_job_queue SET suspend = 0, suspend_until = NULL WHERE id = $1",
                    parent_job_id,
                )
                .execute(db)
                .await?;
                let parent_mini = get_mini_completed_job(&parent_job_id, workspace_id, db).await?;
                if let Some(parent_mini) = parent_mini {
                    let child_err: Value =
                        serde_json::from_str(result.get()).unwrap_or(Value::Null);
                    let err_value = json!({
                        "message": format!("WAC child job {} failed (no step key)", child_job_id),
                        "error": child_err,
                    });
                    let _ = windmill_queue::add_completed_job_error(
                        db,
                        &parent_mini,
                        0,
                        None,
                        err_value,
                        "wac_child_handler",
                        false,
                        None,
                    )
                    .await;
                }
                return Ok(Some(()));
            }
            tracing::warn!(
                parent_job = %parent_job_id,
                child_job = %child_job_id,
                "WAC v2 child completed but no matching step key found in checkpoint, decrementing suspend to avoid parent hang"
            );
            // Still decrement suspend so the parent doesn't hang indefinitely
            let _ = sqlx::query_scalar!(
                "UPDATE v2_job_queue \
                 SET suspend = GREATEST(suspend - 1, 0) \
                 WHERE id = $1 \
                 RETURNING suspend",
                parent_job_id,
            )
            .fetch_optional(db)
            .await?;
            return Ok(Some(()));
        }
    };

    // Build result — wrap errors with _error marker so workflow try/catch can handle them
    let result_value: Value = if success {
        serde_json::from_str(result.get()).unwrap_or(Value::Null)
    } else {
        let child_err: Value = serde_json::from_str(result.get()).unwrap_or(Value::Null);
        tracing::info!(
            parent_job = %parent_job_id,
            child_job = %child_job_id,
            step_key = %step_key,
            "WAC v2 child job failed, storing error for workflow try/catch"
        );
        json!({
            "__wmill_error": true,
            "message": format!("WAC task '{}' failed (child job {})", step_key, child_job_id),
            "child_job_id": child_job_id.to_string(),
            "step_key": step_key,
            "result": child_err,
        })
    };

    tracing::info!(
        parent_job = %parent_job_id,
        child_job = %child_job_id,
        step_key = %step_key,
        success = success,
        "WAC v2 child job completed"
    );

    // Use a transaction to ensure completed_steps merge + suspend decrement
    // are atomic.  Without this, a crash between the two could strand the parent.
    let result_json = serde_json::to_value(&result_value)
        .map_err(|e| error::Error::InternalErr(format!("Failed to serialize step result: {e}")))?;

    let mut tx = db.begin().await?;

    // Merge the completed step into the checkpoint.
    // Uses `|| jsonb_build_object(key, value)` so concurrent children on
    // different workers don't overwrite each other — PostgreSQL serialises
    // concurrent UPDATEs on the same row and each sees the previous write.
    sqlx::query(
        "UPDATE v2_job_status SET workflow_as_code_status = jsonb_set(
            workflow_as_code_status,
            '{_checkpoint,completed_steps}',
            COALESCE(workflow_as_code_status->'_checkpoint'->'completed_steps', '{}'::jsonb)
            || jsonb_build_object($2::text, $3::jsonb)
        ) WHERE id = $1",
    )
    .bind(&parent_job_id)
    .bind(&step_key)
    .bind(&result_json)
    .execute(&mut *tx)
    .await
    .map_err(|e| error::Error::InternalErr(format!("Failed to add WAC completed step: {e}")))?;

    // Decrement the suspend counter.  The counter was set to N (number of
    // children) at dispatch time.  When it reaches 0 all children are done.
    // Keep suspend_until non-null so the suspended pull query
    // (`WHERE suspend_until IS NOT NULL AND suspend <= 0`) picks up the parent.
    let new_suspend: Option<i32> = sqlx::query_scalar!(
        "UPDATE v2_job_queue \
         SET suspend = GREATEST(suspend - 1, 0) \
         WHERE id = $1 \
         RETURNING suspend",
        parent_job_id,
    )
    .fetch_optional(&mut *tx)
    .await?;

    let all_done = new_suspend == Some(0);

    if all_done {
        // Clear pending_steps from checkpoint since all children are complete.
        // This is cosmetic — the next replay will overwrite it anyway — but
        // keeps the checkpoint clean for frontend display.
        let _ = sqlx::query(
            "UPDATE v2_job_status SET workflow_as_code_status = \
             workflow_as_code_status #- '{_checkpoint,pending_steps}' \
             WHERE id = $1",
        )
        .bind(&parent_job_id)
        .execute(&mut *tx)
        .await;
    }

    tx.commit().await?;

    if all_done {
        tracing::info!(
            parent_job = %parent_job_id,
            "WAC v2 all child jobs completed, unsuspending parent"
        );
        WAC_SUSPEND_READY.store(true, Ordering::Relaxed);
    }

    Ok(Some(()))
}

pub async fn handle_non_flow_job_error(
    db: &DB,
    job: &MiniCompletedJob,
    mem_peak: i32,
    canceled_by: Option<CanceledBy>,
    err_string: String,
    err_json: Value,
    worker_name: &str,
) -> Result<WrappedError, Error> {
    append_logs(
        &job.id,
        &job.workspace_id,
        format!("Unexpected error during job execution:\n{err_string}"),
        &db.into(),
    )
    .await;
    add_completed_job_error(
        db,
        job,
        mem_peak,
        canceled_by,
        err_json,
        worker_name,
        false,
        None,
    )
    .await
}

#[tracing::instrument(name = "job_error", level = "info", skip_all, fields(job_id = %job.id))]
pub async fn handle_job_error(
    db: &DB,
    client: &AuthedClient,
    job: &MiniCompletedJob,
    mem_peak: i32,
    canceled_by: Option<CanceledBy>,
    err: Error,
    unrecoverable: bool,
    same_worker_tx: Option<&SameWorkerSender>,
    worker_dir: &str,
    worker_name: &str,
    job_completed_tx: JobCompletedSender,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    #[cfg(feature = "benchmark")] bench: &mut BenchmarkIter,
) {
    let err_string = format!("{}: {}", err.name(), err.to_string());
    let err_json = error_to_value(&err);

    let update_job_future = || async {
        handle_non_flow_job_error(
            db,
            job,
            mem_peak,
            canceled_by.clone(),
            err_string,
            err_json.clone(),
            worker_name,
        )
        .warn_after_seconds(10)
        .await
    };

    let update_job_future = if job.is_flow_step() || job.is_flow() {
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

        let wrapped_error = WrappedError { error: err_json.clone() };
        tracing::error!(parent_flow = %flow, subflow = %job_status_to_update, "handle job error, updating flow status: {err_json:?}");
        let updated_flow = update_flow_status_after_job_completion(
            db,
            client,
            flow,
            &job_status_to_update,
            &job.workspace_id,
            false,
            canceled_by.clone(),
            Arc::new(serde_json::value::to_raw_value(&wrapped_error).unwrap()),
            None,
            unrecoverable,
            &same_worker_tx.expect(SAME_WORKER_REQUIREMENTS).clone(),
            worker_dir,
            None,
            worker_name,
            job_completed_tx.clone(),
            None,
            killpill_rx,
            #[cfg(feature = "benchmark")]
            bench,
        )
        .await;

        if let Err(err) = updated_flow {
            if let Some(parent_job_id) = job.parent_job {
                if let Ok(Some(parent_job)) =
                    get_mini_completed_job(&parent_job_id, &job.workspace_id, db)
                        .warn_after_seconds(10)
                        .await
                {
                    let e = json!({"message": err.to_string(), "name": "InternalErr"});
                    append_logs(
                        &parent_job.id,
                        &job.workspace_id,
                        format!("Unexpected error during flow job error handling:\n{err}"),
                        &db.into(),
                    )
                    .await;
                    let _ = add_completed_job_error(
                        db,
                        &parent_job,
                        mem_peak,
                        canceled_by.clone(),
                        e,
                        worker_name,
                        false,
                        None,
                    )
                    .warn_after_seconds(10)
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
}

#[derive(Debug, Serialize)]
pub struct SerializedError {
    pub message: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}
pub fn extract_error_value(
    program: &str,
    log_lines: &str,
    i: i32,
    step_id: Option<String>,
) -> Box<RawValue> {
    return to_raw_value(&SerializedError {
        message: format!(
            "exit code for \"{program}\": {i}, last log lines:\n{}",
            ANSI_ESCAPE_RE.replace_all(log_lines.trim(), "").to_string()
        ),
        name: "ExecutionErr".to_string(),
        step_id,
        exit_code: Some(i),
    });
}

#[cfg(all(test, feature = "enterprise", feature = "private"))]
mod git_sync_pr_tests {
    use super::git_sync_deploy_pr_head_branch;

    #[test]
    fn fork_branch_wins_and_strips_the_id_prefix() {
        // Generated fork id: branch suffix drops the wm-fork- prefix.
        assert_eq!(
            git_sync_deploy_pr_head_branch(
                "wm-fork-abc",
                Some("prod"),
                None,
                "main",
                false,
                false,
                "",
                "",
                ""
            ),
            Some("wm-fork/main/abc".to_string())
        );
        // Dev workspace (prefix-less id, detected via parent): verbatim suffix.
        assert_eq!(
            git_sync_deploy_pr_head_branch(
                "staging",
                Some("prod"),
                None,
                "main",
                false,
                false,
                "",
                "",
                ""
            ),
            Some("wm-fork/main/staging".to_string())
        );
        // Orphaned fork (parent deleted): the id prefix still identifies it.
        assert_eq!(
            git_sync_deploy_pr_head_branch(
                "wm-fork-abc",
                None,
                None,
                "main",
                true,
                false,
                "f/x/y",
                "",
                "script"
            ),
            Some("wm-fork/main/abc".to_string())
        );
    }

    #[test]
    fn promotion_branch_matches_the_hub_script_formula() {
        // Per-item form: wm_deploy/<ws>/<path_type>/<path with / -> __>.
        assert_eq!(
            git_sync_deploy_pr_head_branch(
                "dev",
                None,
                None,
                "main",
                true,
                false,
                "f/folder/my_script",
                "",
                "script"
            ),
            Some("wm_deploy/dev/script/f__folder__my_script".to_string())
        );
        // Grouped-by-folder form: first two path segments joined by __.
        assert_eq!(
            git_sync_deploy_pr_head_branch(
                "dev",
                None,
                None,
                "main",
                true,
                true,
                "f/folder/my_script",
                "",
                "script"
            ),
            Some("wm_deploy/dev/f__folder".to_string())
        );
        // Renamed object: falls back to the parent path.
        assert_eq!(
            git_sync_deploy_pr_head_branch(
                "dev",
                None,
                None,
                "main",
                true,
                false,
                "",
                "f/folder/old",
                "script"
            ),
            Some("wm_deploy/dev/script/f__folder__old".to_string())
        );
    }

    #[test]
    fn no_branch_when_deploy_stays_on_base() {
        // Workspace-wide mode commits straight to the tracked branch.
        assert_eq!(
            git_sync_deploy_pr_head_branch(
                "dev", None, None, "main", false, false, "f/x/y", "", "script"
            ),
            None
        );
        // Promotion mode but no per-item ref (e.g. user/group objects).
        assert_eq!(
            git_sync_deploy_pr_head_branch("dev", None, None, "main", true, false, "", "", "user"),
            None
        );
    }

    #[test]
    fn dev_workspace_label_branch_wins() {
        // Dev workspaces deploy to their environment-label branch verbatim.
        assert_eq!(
            git_sync_deploy_pr_head_branch(
                "staging-ws",
                Some("prod"),
                Some("staging"),
                "main",
                false,
                false,
                "",
                "",
                ""
            ),
            Some("staging".to_string())
        );
        // Label present even on a wm-fork-prefixed id: label still wins.
        assert_eq!(
            git_sync_deploy_pr_head_branch(
                "wm-fork-x",
                Some("prod"),
                Some("dev"),
                "main",
                false,
                false,
                "",
                "",
                ""
            ),
            Some("dev".to_string())
        );
    }
}
