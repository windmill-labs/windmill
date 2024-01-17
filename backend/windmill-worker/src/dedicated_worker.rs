// use tokio::sync::mpsc;

// pub fn create_dedicated_worker() {
//     let (job_completed_tx, mut new_job) = mpsc::channel::<JobCompleted>(100);
// }

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};
use windmill_common::{error, jobs::QueuedJob, variables, worker::to_raw_value};

use std::{collections::VecDeque, process::Stdio, sync::Arc};

use anyhow::Context;

use crate::{
    common::start_child_process, JobCompleted, JobCompletedSender, MAX_BUFFERED_DEDICATED_JOBS,
};

use futures::{future, Future};
use std::{collections::HashMap, task::Poll};

use tokio::sync::mpsc::Receiver;

fn conditional_polling<T>(
    fut: impl Future<Output = T>,
    predicate: bool,
) -> impl Future<Output = T> {
    let mut fut = Box::pin(fut);
    future::poll_fn(move |cx| {
        if predicate {
            fut.as_mut().poll(cx)
        } else {
            Poll::Pending
        }
    })
}

async fn write_stdin(stdin: &mut tokio::process::ChildStdin, s: &str) -> error::Result<()> {
    let _ = &stdin.write_all(format!("{s}\n").as_bytes()).await?;
    stdin.flush().await.context("stdin flush")?;
    Ok(())
}

pub async fn handle_dedicated_process(
    command_path: &String,
    job_dir: &str,
    context_envs: HashMap<String, String>,
    envs: HashMap<String, String>,
    reserved_variables: [variables::ContextualVariable; 17],
    common_bun_proc_envs: HashMap<String, String>,
    args: Vec<&str>,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
    job_completed_tx: JobCompletedSender,
    token: &str,
    mut jobs_rx: Receiver<Arc<QueuedJob>>,
    worker_name: &str,
) -> std::result::Result<(), error::Error> {
    //do not cache local dependencies
    let mut child = {
        let mut cmd = Command::new(command_path);
        cmd.current_dir(job_dir)
            .env_clear()
            .envs(context_envs)
            .envs(envs)
            .envs(
                reserved_variables
                    .iter()
                    .map(|x| (x.name.clone(), x.value.clone()))
                    .collect::<Vec<_>>(),
            )
            .envs(common_bun_proc_envs)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(cmd, command_path).await?
    };

    let stdout = child
        .stdout
        .take()
        .expect("child did not have a handle to stdout");

    let stderr = child
        .stderr
        .take()
        .expect("child did not have a handle to stderr");

    let mut reader = BufReader::new(stdout).lines();

    let mut err_reader = BufReader::new(stderr).lines();

    let mut stdin = child
        .stdin
        .take()
        .expect("child did not have a handle to stdin");

    // Ensure the child process is spawned in the runtime so it can
    // make progress on its own while we await for any output.
    let child = tokio::spawn(async move {
        let status = child
            .wait()
            .await
            .expect("child process encountered an error");
        tracing::info!("child status was: {}", status);
    });

    let mut jobs = VecDeque::with_capacity(MAX_BUFFERED_DEDICATED_JOBS);
    // let mut i = 0;
    // let mut j = 0;
    let mut alive = true;

    let init_log = format!("dedicated worker: {worker_name}\n\n");
    let mut logs = init_log.clone();
    loop {
        tokio::select! {
            biased;
            _ = killpill_rx.recv(), if alive => {
                println!("received killpill for dedicated worker");
                alive = false;
                if let Err(e) = write_stdin(&mut stdin, "end").await {
                    tracing::info!("Could not write end message to stdin: {e:?}")
                }
            },
            line = err_reader.next_line() => {
                if let Some(line) = line.expect("line is ok") {
                    logs.push_str("[stderr] ");
                    logs.push_str(&line);
                    logs.push_str("\n");
                } else {
                    tracing::info!("dedicated worker process exited");
                    break;
                }
            },
            line = reader.next_line() => {
                // j += 1;

                if let Some(line) = line.expect("line is ok") {
                    if line == "start" {
                        tracing::info!("dedicated worker process started");
                        continue;
                    }
                    tracing::debug!("processed job: {line}");
                    if line.starts_with("wm_res[") {
                        let job: Arc<QueuedJob> = jobs.pop_front().expect("pop");
                        match serde_json::from_str::<Box<serde_json::value::RawValue>>(&line.replace("wm_res[success]:", "").replace("wm_res[error]:", "")) {
                            Ok(result) => {
                                if line.starts_with("wm_res[success]:") {
                                    job_completed_tx.send(JobCompleted { job , result, logs: logs, mem_peak: 0, canceled_by: None, success: true, cached_res_path: None, token: token.to_string() }).await.unwrap()
                                } else {
                                    job_completed_tx.send(JobCompleted { job , result, logs: logs, mem_peak: 0, canceled_by: None, success: false, cached_res_path: None, token: token.to_string() }).await.unwrap()
                                }
                            },
                            Err(e) => {
                                tracing::error!("Could not deserialize job result `{line}`: {e:?}");
                                job_completed_tx.send(JobCompleted { job , result: to_raw_value(&serde_json::json!({"error": format!("Could not deserialize job result `{line}`: {e:?}")})), logs: "".to_string(), mem_peak: 0, canceled_by: None, success: false, cached_res_path: None, token: token.to_string() }).await.unwrap();
                            },
                        };
                        logs = init_log.clone();
                    } else {
                        logs.push_str(&line);
                        logs.push_str("\n");
                    }
                } else {
                    tracing::info!("dedicated worker process exited");
                    break;
                }
            },
            job = conditional_polling(jobs_rx.recv(), alive && jobs.len() < MAX_BUFFERED_DEDICATED_JOBS) => {
                // i += 1;
                if let Some(job) = job {
                    tracing::debug!("received job");
                    jobs.push_back(job.clone());
                    // write_stdin(&mut stdin, &serde_json::to_string(&job.args.unwrap_or_else(|| serde_json::json!({"x": job.id}))).expect("serialize")).await?;
                    write_stdin(&mut stdin, &serde_json::to_string(&job.args).expect("serialize")).await?;
                    stdin.flush().await.context("stdin flush")?;
                } else {
                    tracing::debug!("job channel closed");
                    alive = false;
                    if let Err(e) = write_stdin(&mut stdin, "end").await {
                        tracing::error!("Could not write end message to stdin: {e:?}")
                    }
                }
            }
        }
    }

    child
        .await
        .map_err(|e| anyhow::anyhow!("child process encountered an error: {e}"))?;
    tracing::info!("dedicated worker child process exited successfully");
    Ok(())
}
