use futures::Future;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use nix::sys::signal::{self, Signal};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use nix::unistd::Pid;
use process_wrap::tokio::TokioChildWrapper;
use windmill_common::agent_workers::PingJobStatusResponse;
use windmill_common::jobs::LARGE_LOG_THRESHOLD_SIZE;

#[cfg(windows)]
use std::process::Stdio;
use tokio::fs::File;
#[cfg(windows)]
use tokio::process::Command;
use windmill_common::error::to_anyhow;

use windmill_common::error::{self, Error};

use windmill_common::worker::{
    get_windmill_memory_usage, get_worker_memory_usage, set_job_cancelled_query, Connection,
    JobCancelled, CLOUD_HOSTED,
};

use windmill_queue::{append_logs, CanceledBy};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::os::unix::process::ExitStatusExt;

use std::process::ExitStatus;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::{io, panic, time::Duration};

use tracing::{trace_span, Instrument};
use uuid::Uuid;

#[cfg(feature = "enterprise")]
use windmill_common::job_metrics;

#[cfg(target_os = "linux")]
use tokio::io::AsyncWriteExt;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::{broadcast, watch},
    time::{interval, sleep, Instant, MissedTickBehavior},
};

use futures::{
    future::{self, ready, FutureExt},
    stream, StreamExt,
};

use crate::common::{resolve_job_timeout, OccupancyMetrics};
use crate::job_logger::{append_job_logs, append_with_limit};
use crate::job_logger_oss::process_streaming_log_lines;
use crate::worker_utils::{ping_job_status, update_worker_ping_from_job};
use crate::{MAX_RESULT_SIZE, MAX_WAIT_FOR_SIGINT, MAX_WAIT_FOR_SIGTERM};

lazy_static::lazy_static! {
    pub static ref SLOW_LOGS: bool = std::env::var("SLOW_LOGS").ok().is_some_and(|x| x == "1" || x == "true");
}

//  - kill windows process along with all child processes
#[cfg(windows)]
async fn kill_process_tree(pid: Option<u32>) -> Result<(), String> {
    let pid = match pid {
        Some(pid) => pid,
        None => return Err("No PID provided to kill.".to_string()),
    };

    let output = Command::new("cmd")
        .args(&["/C", "taskkill", "/PID", &pid.to_string(), "/T", "/F"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to execute taskkill: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Failed to kill process tree. Error: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// - wait until child exits and return with exit status
/// - read lines from stdout and stderr and append them to the "queue"."logs"
///   quitting early if output exceedes MAX_LOG_SIZE characters (not bytes)
/// - update the `last_line` and `logs` strings with the program output
/// - update "queue"."last_ping" every five seconds
/// - kill process if we exceed timeout or "queue"."canceled" is set
#[tracing::instrument(name="run_subprocess", level = "info", skip_all, fields(otel.name = %child_name))]
pub async fn handle_child(
    job_id: &Uuid,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by_ref: &mut Option<CanceledBy>,
    mut child: Box<dyn TokioChildWrapper>,
    nsjail: bool,
    worker: &str,
    w_id: &str,
    child_name: &str,
    custom_timeout: Option<i32>,
    sigterm: bool,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    // Do not print logs to output, but instead save to string.
    pipe_stdout: Option<&mut String>,
) -> error::Result<()> {
    let start = Instant::now();

    let pid = child.id();
    #[cfg(target_os = "linux")]
    if let Some(pid) = pid {
        //set the highest oom priority
        if let Some(mut file) = File::create(format!("/proc/{pid}/oom_score_adj"))
            .await
            .map_err(|e| {
                tracing::error!("Could not create oom_score_file to pid {pid}: {e:#}");
                e
            })
            .ok()
        {
            let _ = file.write_all(b"1000").await;
            let _ = file.sync_all().await;
        }
    } else {
        tracing::info!("could not get child pid");
    }
    let (mut set_too_many_logs, mut too_many_logs) = watch::channel::<bool>(false);
    let (tx, rx) = broadcast::channel::<()>(3);
    let mut rx2: broadcast::Receiver<()> = tx.subscribe();

    let output = child_joined_output_stream(&mut child, job_id.clone(), w_id.to_string());

    let job_id: Uuid = job_id.clone();

    /* the cancellation future is polled on by `wait_on_child` while
     * waiting for the child to exit normally */
    let update_job = update_job_poller(
        job_id,
        conn,
        mem_peak,
        canceled_by_ref,
        Box::pin(stream::unfold((), move |_| async move {
            Some((get_mem_peak(pid, nsjail).await, ()))
        })),
        worker,
        w_id,
        rx,
        occupancy_metrics,
    );

    enum KillReason {
        TooManyLogs,
        Timeout { is_job_specific: bool },
        Cancelled(Option<CanceledBy>),
        AlreadyCompleted,
    }

    impl std::fmt::Debug for KillReason {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                KillReason::TooManyLogs => f.write_str("too many logs (max size: 2MB)"),
                KillReason::Timeout { is_job_specific } => f.write_str(if *is_job_specific {
                    "timeout after exceeding job-specific duration limit"
                } else {
                    "timeout after exceeding instance-wide job duration limit"
                }),
                KillReason::Cancelled(canceled_by) => {
                    let mut reason = "cancelled".to_string();
                    if let Some(canceled_by) = canceled_by {
                        if let Some(by) = canceled_by.username.as_ref() {
                            reason.push_str(&format!(" by {}", by));
                        }
                        if let Some(rsn) = canceled_by.reason.as_ref() {
                            reason.push_str(&format!(" (reason: {})", rsn));
                        }
                    }
                    f.write_str(&reason)
                }
                KillReason::AlreadyCompleted => f.write_str("already completed"),
            }
        }
    }

    let (timeout_duration, timeout_warn_msg, is_job_specific) =
        resolve_job_timeout(&conn, w_id, job_id, custom_timeout).await;
    if let Some(msg) = timeout_warn_msg {
        append_logs(&job_id, w_id, msg.as_str(), conn).await;
    }

    /* a future that completes when the child process exits */
    let wait_on_child = async {
        let kill_reason = tokio::select! {
            biased;
            result = Box::into_pin(child.wait()) => return result.map(Ok),
            Ok(()) = too_many_logs.changed() => KillReason::TooManyLogs,
            _ = sleep(timeout_duration) => KillReason::Timeout { is_job_specific },
            ex = update_job, if job_id != Uuid::nil() => match ex {
                UpdateJobPollingExit::Done(canceled_by) => KillReason::Cancelled(canceled_by),
                UpdateJobPollingExit::AlreadyCompleted => KillReason::AlreadyCompleted,
            },
        };
        tx.send(()).expect("rx should never be dropped");
        drop(tx);

        let set_reason = async {
            if matches!(kill_reason, KillReason::Timeout { .. }) {
                match conn {
                    Connection::Sql(db) => {
                        if let Err(err) = set_job_cancelled_query(
                            job_id,
                            db,
                            "timeout",
                            &format!("duration > {}", timeout_duration.as_secs()),
                        )
                        .await
                        {
                            tracing::error!(%job_id, %err, "error setting cancelation reason for job {job_id}: {err}");
                        }
                    }
                    Connection::Http(client) => {
                        if let Err(err) = client
                            .post::<_, ()>(
                                &format!("/api/agent_workers/set_job_cancelled/{}", job_id),
                                None,
                                &JobCancelled {
                                    canceled_by: "timeout".to_string(),
                                    reason: format!("duration > {}", timeout_duration.as_secs()),
                                },
                            )
                            .await
                        {
                            tracing::error!(%job_id, %err, "error setting cancelation reason for job using http {job_id}: {err}");
                        }
                    }
                }
            }
        };

        if let Some(id) = child.id() {
            if *MAX_WAIT_FOR_SIGINT > 0 {
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                signal::kill(Pid::from_raw(id as i32), Signal::SIGINT).unwrap();

                for _ in 0..*MAX_WAIT_FOR_SIGINT {
                    if child.try_wait().is_ok_and(|x| x.is_some()) {
                        break;
                    }
                    sleep(Duration::from_secs(1)).await;
                }
                if child.try_wait().is_ok_and(|x| x.is_some()) {
                    set_reason.await;
                    return Ok(Err(kill_reason));
                }
            }
            if sigterm {
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                signal::kill(Pid::from_raw(id as i32), Signal::SIGTERM).unwrap();

                for _ in 0..*MAX_WAIT_FOR_SIGTERM {
                    if child.try_wait().is_ok_and(|x| x.is_some()) {
                        break;
                    }
                    sleep(Duration::from_secs(1)).await;
                }
                if child.try_wait().is_ok_and(|x| x.is_some()) {
                    set_reason.await;
                    return Ok(Err(kill_reason));
                }
            }
        }
        #[cfg(windows)]
        {
            let pid_to_kill = child.id();
            match kill_process_tree(pid_to_kill).await {
                Ok(_) => tracing::debug!(
                    "successfully killed process tree with PID: {:?}",
                    pid_to_kill
                ),
                Err(e) => tracing::error!("failed to kill process tree: {:?}", e),
            };
            set_reason.await;
            return Ok(Err(kill_reason));
        }

        #[cfg(unix)]
        {
            /* send SIGKILL and reap child process */
            let (_, kill) = future::join(set_reason, Box::into_pin(child.kill())).await;
            kill.map(|()| Err(kill_reason))
        }
    };

    /* a future that reads output from the child and appends to the database */
    let lines = write_lines(
        output,
        &job_id,
        w_id,
        worker,
        conn,
        &mut set_too_many_logs,
        start,
        pipe_stdout,
        &mut rx2,
        child_name,
    )
    .instrument(trace_span!("child_lines"));

    let (wait_result, _) = tokio::join!(wait_on_child, lines);

    let success = wait_result.is_ok()
        && wait_result.as_ref().unwrap().is_ok()
        && wait_result.as_ref().unwrap().as_ref().unwrap().success();
    tracing::info!(%job_id, %success, %mem_peak, %worker, "child process '{child_name}' took {}ms", start.elapsed().as_millis());

    match wait_result {
        _ if *too_many_logs.borrow() => Err(Error::ExecutionErr(format!(
            "logs or result reached limit. (current max size: {MAX_RESULT_SIZE} characters)"
        ))),
        Ok(Ok(status)) => process_status(&child_name, status),
        Ok(Err(kill_reason)) => match kill_reason {
            KillReason::AlreadyCompleted => {
                Err(Error::AlreadyCompleted("Job already completed".to_string()))
            }
            _ => Err(Error::ExecutionErr(format!(
                "job process terminated due to {kill_reason:#?}"
            ))),
        },
        Err(err) => Err(Error::ExecutionErr(format!("job process io error: {err}"))),
    }
}

pub async fn write_lines(
    output: impl stream::Stream<Item = io::Result<String>> + Send,
    job_id: &Uuid,
    w_id: &str,
    worker: &str,
    conn: &Connection,
    set_too_many_logs: &mut watch::Sender<bool>,
    start: Instant,
    pipe_stdout: Option<&mut String>,
    rx2: &mut broadcast::Receiver<()>,
    child_name: &str,
) {
    let max_log_size = if *CLOUD_HOSTED {
        MAX_RESULT_SIZE
    } else {
        usize::MAX
    };

    /* log_remaining is zero when output limit was reached */
    let mut log_remaining = if *CLOUD_HOSTED {
        max_log_size
    } else {
        usize::MAX
    };
    let mut result = io::Result::Ok(());
    let mut output = output
        .take_until(async {
            let _ = rx2.recv().await;
            //wait at most 50ms after end of a script for output stream to end
            tokio::time::sleep(Duration::from_millis(50)).await;
        })
        .boxed();
    /* `do_write` resolves the task, but does not contain the Result.
     * It's useful to know if the task completed. */
    let (mut do_write, mut write_result) = tokio::spawn(ready(())).remote_handle();

    let mut log_total_size: u64 = 0;
    let pg_log_total_size = Arc::new(AtomicU32::new(0));

    let mut pipe_stdout = pipe_stdout;

    while let Some(line) = output.by_ref().next().await {
        let do_write_ = do_write.shared();

        let delay = if start.elapsed() < Duration::from_secs(10) {
            Duration::from_millis(500)
        } else if start.elapsed() < Duration::from_secs(60) {
            Duration::from_millis(2500)
        } else {
            Duration::from_millis(5000)
        };

        let delay = if *SLOW_LOGS { delay * 10 } else { delay };

        let mut read_lines = stream::once(async { line })
            .chain(output.by_ref())
            /* after receiving a line, continue until some delay has passed
             * _and_ the previous database write is complete */
            .take_until(future::join(sleep(delay), do_write_.clone()))
            .boxed();

        /* Read up until an error is encountered,
         * handle log lines first and then the error... */
        let mut joined = String::new();

        let job_id = job_id.clone();
        while let Some(line) = read_lines.next().await {
            match line {
                Ok(line) => {
                    if line.is_empty() {
                        continue;
                    }
                    append_with_limit(&mut joined, &line, &mut log_remaining);
                    if log_remaining == 0 {
                        tracing::info!(%job_id, "Too many logs lines for job {job_id}");
                        let _ = set_too_many_logs.send(true);
                        joined.push_str(&format!(
                                "Job logs or result reached character limit of {MAX_RESULT_SIZE}; killing job."
                            ));
                        /* stop reading and drop our streams fairly quickly */
                        break;
                    }
                }
                Err(err) => {
                    result = Err(err);
                    break;
                }
            }
        }

        /* Ensure the last flush completed before starting a new one.
         *
         * This shouldn't pause since `take_until()` reads lines until `do_write`
         * resolves. We only stop reading lines before `take_until()` resolves if we reach
         * EOF or a read error.  In those cases, waiting on a database query to complete is
         * fine because we're done. */

        if let Some(Ok(p)) = do_write_
            .then(|()| write_result)
            .await
            .err()
            .map(|err| err.try_into_panic())
        {
            panic::resume_unwind(p);
        }

        let joined_len = joined.len() as u64;
        log_total_size += joined_len;
        let compact_logs = log_total_size > LARGE_LOG_THRESHOLD_SIZE as u64;
        if compact_logs {
            log_total_size = 0;
        }

        let worker_name = worker.to_string();

        if let Some(buf) = &mut pipe_stdout {
            buf.push_str(&joined);
            (do_write, write_result) = tokio::spawn(async {}).remote_handle();
        } else {
            let conn = conn.clone();
            let worker_name = worker_name.to_string();
            let w_id = w_id.to_string();
            let job_id = job_id.clone();
            let pg_log_total_size = pg_log_total_size.clone();
            (do_write, write_result) = tokio::spawn(async move {
                append_job_logs(
                    &job_id,
                    &w_id,
                    &joined,
                    &conn,
                    compact_logs,
                    pg_log_total_size,
                    &worker_name,
                )
                .await;
            })
            .remote_handle();
        }

        if let Err(err) = result {
            tracing::error!(%job_id, %err, "error reading output for job {job_id} '{child_name}': {err}");
            break;
        }

        if *set_too_many_logs.borrow() {
            break;
        }
    }

    /* drop our end of the pipe */
    drop(output);

    if let Some(Ok(p)) = do_write
        .then(|()| write_result)
        .await
        .err()
        .map(|err| err.try_into_panic())
    {
        panic::resume_unwind(p);
    }
}

pub(crate) async fn get_mem_peak(pid: Option<u32>, nsjail: bool) -> i32 {
    if pid.is_none() {
        return -1;
    }
    let pid = if nsjail {
        // Read /proc/<nsjail_pid>/task/<nsjail_pid>/children and extract pid
        let nsjail_pid = pid.unwrap();
        let children_path = format!("/proc/{}/task/{}/children", nsjail_pid, nsjail_pid);
        if let Ok(mut file) = File::open(children_path).await {
            let mut contents = String::new();
            if tokio::io::AsyncReadExt::read_to_string(&mut file, &mut contents)
                .await
                .is_ok()
            {
                if let Some(child_pid) = contents.split_whitespace().next() {
                    if let Ok(child_pid) = child_pid.parse::<u32>() {
                        child_pid
                    } else {
                        return -1;
                    }
                } else {
                    return -1;
                }
            } else {
                return -1;
            }
        } else {
            return -1;
        }
    } else {
        pid.unwrap()
    };

    if let Ok(file) = File::open(format!("/proc/{}/status", pid)).await {
        let mut lines = BufReader::new(file).lines();
        while let Some(line) = lines.next_line().await.unwrap_or(None) {
            if line.starts_with("VmHWM:") {
                return line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(-1);
            };
        }
        -2
    } else {
        // rand::random::<i32>() % 100 // to remove - used to fake memory data on MacOS
        -3
    }
}

pub async fn run_future_with_polling_update_job_poller<Fut, T, S>(
    job_id: Uuid,
    timeout: Option<i32>,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by_ref: &mut Option<CanceledBy>,
    result_f: Fut,
    worker_name: &str,
    w_id: &str,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
    get_mem: S,
) -> error::Result<T>
where
    Fut: Future<Output = windmill_common::error::Result<T>>,
    S: stream::Stream<Item = i32> + Unpin,
{
    let (tx, rx) = broadcast::channel::<()>(3);

    let update_job = update_job_poller(
        job_id,
        conn,
        mem_peak,
        canceled_by_ref,
        get_mem,
        worker_name,
        w_id,
        rx,
        occupancy_metrics,
    );

    let timeout_ms = u64::try_from(
        resolve_job_timeout(&conn, &w_id, job_id, timeout)
            .await
            .0
            .as_millis(),
    )
    .unwrap_or(200000);

    let rows = tokio::select! {
        biased;
        result = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), result_f) => result
        .map_err(|e| {
            tracing::error!("Query timeout: {}", e);
            Error::ExecutionErr(format!("Query timeout after (>{}s)", timeout_ms/1000))
        })?,
        ex = update_job, if job_id != Uuid::nil() => {
            match ex {
                UpdateJobPollingExit::Done(canceled_by) => {
                    let (by, reason) = canceled_by.as_ref().map_or(("unknown".to_string(), "unknown".to_string()), |x| (x.username.clone().unwrap_or("".to_string()), x.reason.clone().unwrap_or("".to_string())));
                    Err(Error::ExecutionErr(format!("Job cancelled by {by} (reason: {reason})",))).map_err(to_anyhow)?
                },
                UpdateJobPollingExit::AlreadyCompleted => Err(Error::AlreadyCompleted("Job already completed".to_string())).map_err(to_anyhow)?,
            }
        }
    }?;
    drop(tx);
    Ok(rows)
}

pub enum UpdateJobPollingExit {
    Done(Option<CanceledBy>),
    AlreadyCompleted,
}

pub async fn update_job_poller<S>(
    job_id: Uuid,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by_ref: &mut Option<CanceledBy>,
    mut get_mem: S,
    worker_name: &str,
    w_id: &str,
    mut rx: broadcast::Receiver<()>,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
) -> UpdateJobPollingExit
where
    S: stream::Stream<Item = i32> + Unpin,
{
    let update_job_interval = Duration::from_millis(500);

    let conn = conn.clone();
    let mut interval = interval(update_job_interval);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut i = 0;

    #[cfg(feature = "enterprise")]
    let mut memory_metric_id: Result<String, Error> =
        Err(Error::NotFound("not yet initialized".to_string()));

    loop {
        tokio::select!(
            _ = rx.recv() => break,
            _ = interval.tick() => {
                // update the last_ping column every 5 seconds
                i+=1;
                if i == 1 || i % 10 == 0 {
                    let memory_usage = get_worker_memory_usage();
                    let wm_memory_usage = get_windmill_memory_usage();
                    tracing::info!("job {job_id} on {worker_name} in {w_id} worker memory snapshot {}kB/{}kB", memory_usage.unwrap_or_default()/1024, wm_memory_usage.unwrap_or_default()/1024);
                    let occupancy = occupancy_metrics.as_mut().map(|x| x.update_occupancy_metrics());
                    if job_id != Uuid::nil() {
                        if let Err(err) = update_worker_ping_from_job(&conn, &job_id, w_id, worker_name, memory_usage, wm_memory_usage, occupancy).await {
                            tracing::error!("Unable to update worker ping for job {} in workspace {}. Error was: {:?}", job_id, w_id, err);
                        }
                    }
                }
                let current_mem = get_mem.next().await.unwrap_or(0);
                if current_mem > *mem_peak {
                    *mem_peak = current_mem
                }
                tracing::info!("job {job_id} on {worker_name} in {w_id} still running.  mem: {current_mem}kB, peak mem: {mem_peak}kB");


                let update_job_row = i == 2 || (!*SLOW_LOGS && (i < 20 || (i < 120 && i % 5 == 0) || i % 10 == 0)) || i % 20 == 0;
                if update_job_row {
                #[cfg(feature = "enterprise")]
                {
                    if job_id != Uuid::nil() {
                        if let Connection::Sql(ref db) = conn {
                            // tracking metric starting at i >= 2 b/c first point it useless and we don't want to track metric for super fast jobs
                            if i == 2 {
                                        memory_metric_id = job_metrics::register_metric_for_job(
                                    &db,
                                    w_id.to_string(),
                                    job_id,
                                    "memory_kb".to_string(),
                                    job_metrics::MetricKind::TimeseriesInt,
                                    Some("Job Memory Footprint (kB)".to_string()),
                                )
                                .await;
                            }
                            if let Ok(ref metric_id) = memory_metric_id {
                                if let Err(err) = job_metrics::record_metric(&db, w_id.to_string(), job_id, metric_id.to_owned(), job_metrics::MetricNumericValue::Integer(current_mem)).await {
                                    tracing::error!("Unable to save memory stat for job {} in workspace {}. Error was: {:?}", job_id, w_id, err);
                                }
                            }
                        }
                    }
                }
                if job_id != Uuid::nil() {
                    if matches!(conn, Connection::Http(_)) {
                        if i % 4 != 0 {
                            // only ping every 4th time (2s) on http agent mode
                            continue;
                        }
                    }
                    let ping_job_status = ping_job_status(&conn, &job_id, Some(*mem_peak), if current_mem > 0 { Some(current_mem) } else { None }).await.unwrap_or_else(|e| {
                            tracing::error!("Unable to ping job status for job {job_id}. Error was: {:?}", e);
                            PingJobStatusResponse {
                            canceled_by: None,
                            canceled_reason: None,
                            already_completed: false,
                        }
                    });
                    if ping_job_status.already_completed {
                        return UpdateJobPollingExit::AlreadyCompleted
                    }
                    if ping_job_status.canceled_by.is_some() {
                        canceled_by_ref.replace(CanceledBy {
                            username: ping_job_status.canceled_by.clone(),
                            reason: ping_job_status.canceled_reason.clone(),
                        });
                        break
                    }
                }
            }
            },
        );
    }
    tracing::info!("job {job_id} finished");

    UpdateJobPollingExit::Done(canceled_by_ref.clone())
}

/// takes stdout and stderr from Child, panics if either are not present
///
/// builds a stream joining both stdout and stderr each read line by line
fn child_joined_output_stream(
    child: &mut Box<dyn TokioChildWrapper>,
    job_id: Uuid,
    w_id: String,
) -> impl stream::FusedStream<Item = io::Result<String>> {
    let stderr = child
        .stderr()
        .take()
        .expect("child did not have a handle to stderr");

    let stdout = child
        .stdout()
        .take()
        .expect("child did not have a handle to stdout");

    let stdout = BufReader::new(stdout).lines();
    let stderr = BufReader::new(stderr).lines();
    stream::select(
        lines_to_stream(stderr, true, job_id.clone(), w_id.clone()),
        lines_to_stream(stdout, false, job_id, w_id),
    )
}

pub fn lines_to_stream<R: tokio::io::AsyncBufRead + Unpin>(
    mut lines: tokio::io::Lines<R>,
    stderr: bool,
    job_id: Uuid,
    w_id: String,
) -> impl futures::Stream<Item = io::Result<String>> {
    stream::poll_fn(move |cx| {
        std::pin::Pin::new(&mut lines)
            .poll_next_line(cx)
            .map(|result| process_streaming_log_lines(result, stderr, &job_id, &w_id))
    })
}

pub fn process_status(program: &str, status: ExitStatus) -> error::Result<()> {
    if status.success() {
        Ok(())
    } else if let Some(code) = status.code() {
        Err(error::Error::ExitStatus(program.to_string(), code))
    } else {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        return Err(error::Error::ExecutionErr(format!(
            "process terminated by signal: {:#?}, stopped_signal: {:#?}, core_dumped: {}",
            status.signal(),
            status.stopped_signal(),
            status.core_dumped()
        )));

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        return Err(error::Error::ExecutionErr(String::from(
            "process terminated by signal",
        )));
    }
}
