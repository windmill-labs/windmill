use futures::Future;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use nix::sys::signal::{self, Signal};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use nix::unistd::Pid;

use sqlx::{Pool, Postgres};
#[cfg(windows)]
use std::process::Stdio;
use tokio::fs::File;
#[cfg(windows)]
use tokio::process::Command;
use windmill_common::error::to_anyhow;

use windmill_common::error::{self, Error};

use windmill_common::worker::{get_windmill_memory_usage, get_worker_memory_usage, CLOUD_HOSTED};

use windmill_queue::{append_logs, CanceledBy};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::os::unix::process::ExitStatusExt;

use std::process::ExitStatus;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::{io, panic, time::Duration};

use tracing::{trace_span, Instrument};
use uuid::Uuid;
use windmill_common::DB;

#[cfg(feature = "enterprise")]
use windmill_common::job_metrics;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Child,
    sync::{broadcast, watch},
    time::{interval, sleep, Instant, MissedTickBehavior},
};

use futures::{
    future::{self, ready, FutureExt},
    stream, StreamExt,
};

use crate::common::{resolve_job_timeout, OccupancyMetrics};
use crate::job_logger::{append_job_logs, append_with_limit, LARGE_LOG_THRESHOLD_SIZE};
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
#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_child(
    job_id: &Uuid,
    db: &Pool<Postgres>,
    mem_peak: &mut i32,
    canceled_by_ref: &mut Option<CanceledBy>,
    mut child: Child,
    nsjail: bool,
    worker: &str,
    w_id: &str,
    child_name: &str,
    custom_timeout: Option<i32>,
    sigterm: bool,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
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
    let (set_too_many_logs, mut too_many_logs) = watch::channel::<bool>(false);
    let (tx, rx) = broadcast::channel::<()>(3);
    let mut rx2 = tx.subscribe();

    let output = child_joined_output_stream(&mut child);

    let job_id = job_id.clone();

    /* the cancellation future is polled on by `wait_on_child` while
     * waiting for the child to exit normally */
    let update_job = update_job_poller(
        job_id,
        db,
        mem_peak,
        canceled_by_ref,
        || get_mem_peak(pid, nsjail),
        worker,
        w_id,
        rx,
        occupancy_metrics,
    );

    #[derive(PartialEq, Debug)]
    enum KillReason {
        TooManyLogs,
        Timeout,
        Cancelled,
        AlreadyCompleted,
    }

    let (timeout_duration, timeout_warn_msg) =
        resolve_job_timeout(&db, w_id, job_id, custom_timeout).await;
    if let Some(msg) = timeout_warn_msg {
        append_logs(&job_id, w_id, msg.as_str(), db).await;
    }

    /* a future that completes when the child process exits */
    let wait_on_child = async {
        let db = db.clone();

        let kill_reason = tokio::select! {
            biased;
            result = child.wait() => return result.map(Ok),
            Ok(()) = too_many_logs.changed() => KillReason::TooManyLogs,
            _ = sleep(timeout_duration) => KillReason::Timeout,
            ex = update_job, if job_id != Uuid::nil() => match ex {
                UpdateJobPollingExit::Done => KillReason::Cancelled,
                UpdateJobPollingExit::AlreadyCompleted => KillReason::AlreadyCompleted,
            },
        };
        tx.send(()).expect("rx should never be dropped");
        drop(tx);

        let set_reason = async {
            if kill_reason == KillReason::Timeout {
                if let Err(err) = sqlx::query(
                    r#"
                       UPDATE queue
                          SET canceled = true
                            , canceled_by = 'timeout'
                            , canceled_reason = $1
                        WHERE id = $2
                    "#,
                )
                .bind(format!("duration > {}", timeout_duration.as_secs()))
                .bind(job_id)
                .execute(&db)
                .await
                {
                    tracing::error!(%job_id, %err, "error setting cancelation reason for job {job_id}: {err}");
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
            let (_, kill) = future::join(set_reason, child.kill()).await;
            kill.map(|()| Err(kill_reason))
        }
    };

    /* a future that reads output from the child and appends to the database */
    let lines = async move {

        let max_log_size = if *CLOUD_HOSTED {
            MAX_RESULT_SIZE
        } else {
            usize::MAX
        };

        /* log_remaining is zero when output limit was reached */
        let mut log_remaining =  if *CLOUD_HOSTED {
            max_log_size
        } else {
            usize::MAX
        };
        let mut result = io::Result::Ok(());
        let mut output = output.take_until(async {
            let _ = rx2.recv().await;
            //wait at most 50ms after end of a script for output stream to end
            tokio::time::sleep(Duration::from_millis(50)).await;
         }).boxed();
        /* `do_write` resolves the task, but does not contain the Result.
         * It's useful to know if the task completed. */
        let (mut do_write, mut write_result) = tokio::spawn(ready(())).remote_handle();

        let mut log_total_size: u64 = 0;
        let pg_log_total_size = Arc::new(AtomicU32::new(0));

        while let Some(line) =  output.by_ref().next().await {

            let do_write_ = do_write.shared();

            let delay = if start.elapsed() < Duration::from_secs(10) {
                Duration::from_millis(500)
            } else if start.elapsed() < Duration::from_secs(60){
                Duration::from_millis(2500)
            } else {
                Duration::from_millis(5000)
            };

            let delay = if *SLOW_LOGS {
                delay * 10
            } else {
                delay
            };

            let mut read_lines = stream::once(async { line })
                .chain(output.by_ref())
                /* after receiving a line, continue until some delay has passed
                 * _and_ the previous database write is complete */
                .take_until(future::join(sleep(delay), do_write_.clone()))
                .boxed();

            /* Read up until an error is encountered,
             * handle log lines first and then the error... */
            let mut joined = String::new();

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
            let w_id2 = w_id.to_string();
            (do_write, write_result) = tokio::spawn(append_job_logs(job_id, w_id2, joined, db.clone(), compact_logs, pg_log_total_size.clone(), worker_name)).remote_handle();



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
    }.instrument(trace_span!("child_lines"));

    let (wait_result, _) = tokio::join!(wait_on_child, lines);

    let success = wait_result.is_ok()
        && wait_result.as_ref().unwrap().is_ok()
        && wait_result.as_ref().unwrap().as_ref().unwrap().success();
    tracing::info!(%job_id, %success, %mem_peak, %worker, "child process '{child_name}' took {}ms", start.elapsed().as_millis());

    match wait_result {
        _ if *too_many_logs.borrow() => Err(Error::ExecutionErr(format!(
            "logs or result reached limit. (current max size: {MAX_RESULT_SIZE} characters)"
        ))),
        Ok(Ok(status)) => process_status(status),
        Ok(Err(kill_reason)) => match kill_reason {
            KillReason::AlreadyCompleted => {
                Err(Error::AlreadyCompleted("Job already completed".to_string()))
            }
            _ => Err(Error::ExecutionErr(format!(
                "job process killed because {kill_reason:#?}"
            ))),
        },
        Err(err) => Err(Error::ExecutionErr(format!("job process io error: {err}"))),
    }
}

async fn get_mem_peak(pid: Option<u32>, nsjail: bool) -> i32 {
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

pub async fn run_future_with_polling_update_job_poller<Fut, T>(
    job_id: Uuid,
    timeout: Option<i32>,
    db: &DB,
    mem_peak: &mut i32,
    canceled_by_ref: &mut Option<CanceledBy>,
    result_f: Fut,
    worker_name: &str,
    w_id: &str,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
) -> error::Result<T>
where
    Fut: Future<Output = anyhow::Result<T>>,
{
    let (tx, rx) = broadcast::channel::<()>(3);

    let update_job = update_job_poller(
        job_id,
        db,
        mem_peak,
        canceled_by_ref,
        || async { 0 },
        worker_name,
        w_id,
        rx,
        occupancy_metrics,
    );

    let timeout_ms = u64::try_from(
        resolve_job_timeout(&db, &w_id, job_id, timeout)
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
                UpdateJobPollingExit::Done => Err(Error::ExecutionErr("Job cancelled".to_string())).map_err(to_anyhow)?,
                UpdateJobPollingExit::AlreadyCompleted => Err(Error::AlreadyCompleted("Job already completed".to_string())).map_err(to_anyhow)?,
            }
        }
    }?;
    drop(tx);
    Ok(rows)
}

pub enum UpdateJobPollingExit {
    Done,
    AlreadyCompleted,
}

pub async fn update_job_poller<F, Fut>(
    job_id: Uuid,
    db: &DB,
    mem_peak: &mut i32,
    canceled_by_ref: &mut Option<CanceledBy>,
    get_mem: F,
    worker_name: &str,
    w_id: &str,
    mut rx: broadcast::Receiver<()>,
    occupancy_metrics: &mut Option<&mut OccupancyMetrics>,
) -> UpdateJobPollingExit
where
    F: Fn() -> Fut,
    Fut: Future<Output = i32>,
{
    let update_job_interval = Duration::from_millis(500);

    let db = db.clone();

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
                        sqlx::query!(
                            "UPDATE worker_ping SET ping_at = now(), current_job_id = $1, current_job_workspace_id = $2, memory_usage = $3, wm_memory_usage = $4,
                            occupancy_rate = $6, occupancy_rate_15s = $7, occupancy_rate_5m = $8, occupancy_rate_30m = $9 WHERE worker = $5",
                            &job_id,
                            &w_id,
                            memory_usage,
                            wm_memory_usage,
                            &worker_name,
                            occupancy.map(|x| x.0),
                            occupancy.and_then(|x| x.1),
                            occupancy.and_then(|x| x.2),
                            occupancy.and_then(|x| x.3),
                        )
                        .execute(&db)
                        .await
                        .expect("update worker ping");
                    }
                }
                let current_mem = get_mem().await;
                if current_mem > *mem_peak {
                    *mem_peak = current_mem
                }
                tracing::info!("job {job_id} on {worker_name} in {w_id} still running.  mem: {current_mem}kB, peak mem: {mem_peak}kB");


                let update_job_row = i == 2 || (!*SLOW_LOGS && (i < 20 || (i < 120 && i % 5 == 0) || i % 10 == 0)) || i % 20 == 0;
                if update_job_row {
                #[cfg(feature = "enterprise")]
                {
                    if job_id != Uuid::nil() {

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
                if job_id != Uuid::nil() {
                    let (canceled, canceled_by, canceled_reason, already_completed) = sqlx::query_as::<_, (bool, Option<String>, Option<String>, bool)>("UPDATE queue SET mem_peak = $1, last_ping = now() WHERE id = $2 RETURNING canceled, canceled_by, canceled_reason, false")
                        .bind(*mem_peak)
                        .bind(job_id)
                        .fetch_optional(&db)
                        .await
                        .unwrap_or_else(|e| {
                            tracing::error!(%e, "error updating job {job_id}: {e:#}");
                            Some((false, None, None, false))
                        })
                        .unwrap_or_else(|| {
                            // if the job is not in queue, it can only be in the completed_job so it is already complete
                            (false, None, None, true)
                        });
                    if already_completed {
                        return UpdateJobPollingExit::AlreadyCompleted
                    }
                    if canceled {
                        canceled_by_ref.replace(CanceledBy {
                            username: canceled_by.clone(),
                            reason: canceled_reason.clone(),
                        });
                        break
                    }
                }
            }
            },
        );
    }
    tracing::info!("job {job_id} finished");

    UpdateJobPollingExit::Done
}

/// takes stdout and stderr from Child, panics if either are not present
///
/// builds a stream joining both stdout and stderr each read line by line
fn child_joined_output_stream(
    child: &mut Child,
) -> impl stream::FusedStream<Item = io::Result<String>> {
    let stderr = child
        .stderr
        .take()
        .expect("child did not have a handle to stdout");

    let stdout = child
        .stdout
        .take()
        .expect("child did not have a handle to stdout");

    let stdout = BufReader::new(stdout).lines();
    let stderr = BufReader::new(stderr).lines();
    stream::select(lines_to_stream(stderr), lines_to_stream(stdout))
}

pub fn lines_to_stream<R: tokio::io::AsyncBufRead + Unpin>(
    mut lines: tokio::io::Lines<R>,
) -> impl futures::Stream<Item = io::Result<String>> {
    stream::poll_fn(move |cx| {
        std::pin::Pin::new(&mut lines)
            .poll_next_line(cx)
            .map(|result| result.transpose())
    })
}

pub fn process_status(status: ExitStatus) -> error::Result<()> {
    if status.success() {
        Ok(())
    } else if let Some(code) = status.code() {
        Err(error::Error::ExitStatus(code))
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
