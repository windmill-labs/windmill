// use tokio::sync::mpsc;

// pub fn create_dedicated_worker() {
//     let (job_completed_tx, mut new_job) = mpsc::channel::<JobCompleted>(100);
// }

use async_recursion::async_recursion;
use std::{collections::VecDeque, process::Stdio, sync::Arc};
use tokio::sync::mpsc::Sender;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    task::JoinHandle,
};
use windmill_common::error::Error;
use windmill_common::flows::FlowValue;
use windmill_common::worker::WORKER_CONFIG;
use windmill_common::{
    error,
    flows::{FlowModule, FlowModuleValue},
    jobs::QueuedJob,
    scripts::{ScriptHash, ScriptLang},
    variables,
    worker::to_raw_value,
    DB,
};
use windmill_queue::append_logs;

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

#[cfg(feature = "enterprise")]
pub async fn handle_dedicated_process(
    command_path: &String,
    job_dir: &str,
    context_envs: HashMap<String, String>,
    envs: HashMap<String, String>,
    reserved_variables: Vec<variables::ContextualVariable>,
    common_bun_proc_envs: HashMap<String, String>,
    args: Vec<&str>,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
    job_completed_tx: JobCompletedSender,
    token: &str,
    mut jobs_rx: Receiver<std::sync::Arc<QueuedJob>>,
    worker_name: &str,
    db: &DB,
    script_path: &str,
    mode: &str,
) -> std::result::Result<(), error::Error> {
    //do not cache local dependencies

    use crate::handle_child::process_status;

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
        if let Err(e) = process_status(status) {
            tracing::error!("child exit status was not success: {e:#}");
        } else {
            tracing::info!("child exit status was success");
        }
    });

    let mut jobs: VecDeque<Arc<QueuedJob>> = VecDeque::with_capacity(MAX_BUFFERED_DEDICATED_JOBS);
    // let mut i = 0;
    // let mut j = 0;
    let mut alive = true;

    let init_log = format!("dedicated worker {mode}: {worker_name}\n\n");
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
                stdin.flush().await.context("stdin flush")?;
            },
            line = err_reader.next_line() => {
                if let Some(line) = line.expect("line is ok") {
                    tracing::error!("stderr dedicated worker: {line}");
                    logs.push_str("[stderr] ");
                    logs.push_str(&line);
                    logs.push_str("\n");
                } else {
                    tracing::info!("dedicated worker process exited {script_path}");
                    let mut last_stdout = "".to_string();
                    while let Some(line) = reader.next_line().await.ok().flatten() {
                        last_stdout = line;
                        last_stdout.push_str("\n");
                    }
                    tracing::info!("Last stdout for {script_path}: {last_stdout}");
                    break;
                }
            },
            line = reader.next_line() => {
                // j += 1;

                if let Some(line) = line.expect("line is ok") {
                    if line == "start" {
                        tracing::info!("dedicated worker process started {script_path}");
                        continue;
                    }
                    tracing::debug!("processed job: |{line}|");
                    if line.starts_with("wm_res[") {
                        let job: Arc<QueuedJob> = jobs.pop_front().expect("pop");
                        tracing::info!("job completed on dedicated worker {script_path}: {}", job.id);
                        match serde_json::from_str::<Box<serde_json::value::RawValue>>(&line.replace("wm_res[success]:", "").replace("wm_res[error]:", "")) {
                            Ok(result) => {
                                let result = Arc::new(result);
                                append_logs(&job.id, &job.workspace_id,  logs.clone(), db).await;
                                if line.starts_with("wm_res[success]:") {
                                    job_completed_tx.send(JobCompleted { job , result, mem_peak: 0, canceled_by: None, success: true, cached_res_path: None, token: token.to_string() }).await.unwrap()
                                } else {
                                    job_completed_tx.send(JobCompleted { job , result, mem_peak: 0, canceled_by: None, success: false, cached_res_path: None, token: token.to_string() }).await.unwrap()
                                }
                            },
                            Err(e) => {
                                tracing::error!("Could not deserialize job result `{line}`: {e:?}");
                                job_completed_tx.send(JobCompleted { job , result: Arc::new(to_raw_value(&serde_json::json!({"error": format!("Could not deserialize job result `{line}`: {e:?}")}))),  mem_peak: 0, canceled_by: None, success: false, cached_res_path: None, token: token.to_string() }).await.unwrap();
                            },
                        };
                        logs = init_log.clone();
                    } else {
                        logs.push_str(&line);
                        logs.push_str("\n");
                    }
                } else {
                    tracing::info!("dedicated worker {script_path} process exited");
                    let mut last_stderr = "".to_string();
                    while let Some(line) = err_reader.next_line().await.ok().flatten() {
                        last_stderr = line;
                        last_stderr.push_str("\n");
                    }
                    tracing::info!("Last stderr for {script_path}: {last_stderr}");
                    break;
                }
            },
            job = conditional_polling(jobs_rx.recv(), alive && jobs.len() < MAX_BUFFERED_DEDICATED_JOBS) => {
                // i += 1;
                if let Some(job) = job {
                    jobs.push_back(job.clone());
                    tracing::info!("received job and adding to queue on dedicated worker for {script_path}: {} (queue_size: {})", job.id, jobs.len());

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
        .map_err(|e| anyhow::anyhow!("child process {script_path} encountered an error: {e:#}"))?;

    tracing::info!("dedicated worker {script_path} child process exited successfully");
    Ok(())
}

type DedicatedWorker = (
    String,
    Sender<std::sync::Arc<QueuedJob>>,
    Option<JoinHandle<()>>,
);

// spawn one dedicated worker per compatible steps of the flow, associating the node id to the dedicated worker channel send
#[async_recursion]
#[cfg(feature = "enterprise")]
async fn spawn_dedicated_workers_for_flow(
    modules: &Vec<FlowModule>,
    w_id: &str,
    path: &str,
    killpill_tx: tokio::sync::broadcast::Sender<()>,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    db: &DB,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    job_completed_tx: &JobCompletedSender,
) -> Vec<DedicatedWorker> {
    let mut workers = vec![];
    let mut script_path_to_worker: HashMap<String, Sender<std::sync::Arc<QueuedJob>>> =
        HashMap::new();
    for module in modules.iter() {
        let value = module.get_value();
        if let Ok(value) = value {
            match &value {
                FlowModuleValue::Script { path, hash, .. } => {
                    let key = format!(
                        "{}:{}",
                        path,
                        hash.clone()
                            .map(|x| x.to_string())
                            .unwrap_or_else(|| "".to_string())
                    );
                    if let Some(sender) = script_path_to_worker.get(&key) {
                        workers.push((module.id.clone(), sender.clone(), None));
                    } else {
                        if let Some(dedi_w) = spawn_dedicated_worker(
                            SpawnWorker::Script { path: path.to_string(), hash: hash.clone() },
                            w_id,
                            killpill_tx.clone(),
                            killpill_rx,
                            db,
                            worker_dir,
                            base_internal_url,
                            worker_name,
                            job_completed_tx,
                            Some(module.id.clone()),
                        )
                        .await
                        {
                            script_path_to_worker.insert(key, dedi_w.1.clone());
                            workers.push(dedi_w);
                        }
                    }
                }
                FlowModuleValue::ForloopFlow { modules, .. } => {
                    let w = spawn_dedicated_workers_for_flow(
                        &modules,
                        w_id,
                        path,
                        killpill_tx.clone(),
                        killpill_rx,
                        db,
                        worker_dir,
                        base_internal_url,
                        worker_name,
                        job_completed_tx,
                    )
                    .await;
                    workers.extend(w);
                }
                FlowModuleValue::WhileloopFlow { modules, .. } => {
                    let w = spawn_dedicated_workers_for_flow(
                        &modules,
                        w_id,
                        path,
                        killpill_tx.clone(),
                        killpill_rx,
                        db,
                        worker_dir,
                        base_internal_url,
                        worker_name,
                        job_completed_tx,
                    )
                    .await;
                    workers.extend(w);
                }
                FlowModuleValue::BranchOne { branches, default } => {
                    for modules in branches
                        .iter()
                        .map(|x| &x.modules)
                        .chain(std::iter::once(default))
                    {
                        let w = spawn_dedicated_workers_for_flow(
                            &modules,
                            w_id,
                            path,
                            killpill_tx.clone(),
                            killpill_rx,
                            db,
                            worker_dir,
                            base_internal_url,
                            worker_name,
                            job_completed_tx,
                        )
                        .await;
                        workers.extend(w);
                    }
                }
                FlowModuleValue::BranchAll { branches, .. } => {
                    for branch in branches {
                        let w = spawn_dedicated_workers_for_flow(
                            &branch.modules,
                            w_id,
                            path,
                            killpill_tx.clone(),
                            killpill_rx,
                            db,
                            worker_dir,
                            base_internal_url,
                            worker_name,
                            job_completed_tx,
                        )
                        .await;
                        workers.extend(w);
                    }
                }
                FlowModuleValue::RawScript { content, lock, path: spath, language, .. } => {
                    if let Some(dedi_w) = spawn_dedicated_worker(
                        SpawnWorker::RawScript {
                            path: spath.clone().unwrap_or(path.to_string()),
                            content: content.to_string(),
                            lock: lock.clone(),
                            lang: language.clone(),
                        },
                        w_id,
                        killpill_tx.clone(),
                        killpill_rx,
                        db,
                        worker_dir,
                        base_internal_url,
                        worker_name,
                        job_completed_tx,
                        Some(module.id.clone()),
                    )
                    .await
                    {
                        workers.push(dedi_w);
                    }
                }
                FlowModuleValue::Flow { .. } => (),
                FlowModuleValue::Identity => (),
            }
        } else {
            tracing::error!("failed to get value for module: {:?}", module);
        }
    }
    workers
}

pub async fn create_dedicated_worker_map(
    killpill_tx: &tokio::sync::broadcast::Sender<()>,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    db: &DB,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    job_completed_tx: &JobCompletedSender,
) -> (
    HashMap<String, Sender<std::sync::Arc<QueuedJob>>>,
    bool,
    Vec<JoinHandle<()>>,
) {
    let mut dedicated_handles = vec![];
    if let Some(_wp) = WORKER_CONFIG.read().await.dedicated_worker.clone() {
        let mut hm = HashMap::new();
        let is_flow_worker;
        if let Some(flow_path) = _wp.path.strip_prefix("flow/") {
            is_flow_worker = true;
            let value = sqlx::query_scalar!(
                "SELECT flow_version.value 
                FROM flow 
                LEFT JOIN flow_version 
                    ON flow_version.id = flow.versions[array_upper(flow.versions, 1)]
                WHERE flow.path = $1 AND flow.workspace_id = $2",
                flow_path,
                _wp.workspace_id
            )
            .fetch_optional(db)
            .await;
            if let Ok(v) = value {
                if let Some(v) = v {
                    let value = serde_json::from_value::<FlowValue>(v).map_err(|err| {
                        Error::InternalErr(format!(
                            "could not convert json to flow for {flow_path}: {err:?}"
                        ))
                    });
                    if let Ok(flow) = value {
                        let workers = spawn_dedicated_workers_for_flow(
                            &flow.modules,
                            &_wp.workspace_id,
                            &_wp.path,
                            killpill_tx.clone(),
                            &killpill_rx,
                            db,
                            &worker_dir,
                            base_internal_url,
                            &worker_name,
                            &job_completed_tx,
                        )
                        .await;
                        workers.into_iter().for_each(|(path, sender, handle)| {
                            tracing::info!("spawned dedicated worker for flow: {}", path.as_str());
                            if let Some(h) = handle {
                                dedicated_handles.push(h);
                            }
                            hm.insert(path, sender);
                        });
                    }
                } else {
                    tracing::error!(
                        "flow present but value not found for dedicated worker. {}",
                        flow_path
                    );
                }
            } else {
                tracing::error!("flow not found for dedicated worker: {}. Waiting for dependency job and expected to restart.", flow_path);
            }
        } else {
            is_flow_worker = false;
            if let Some((path, sender, handle)) = spawn_dedicated_worker(
                SpawnWorker::Script { path: _wp.path.clone(), hash: None },
                &_wp.workspace_id,
                killpill_tx.clone(),
                &killpill_rx,
                db,
                &worker_dir,
                base_internal_url,
                &worker_name,
                &job_completed_tx,
                None,
            )
            .await
            {
                if let Some(h) = handle {
                    dedicated_handles.push(h);
                }
                hm.insert(path, sender);
            } else {
                tracing::error!(
                    "failed to spawn dedicated worker for {}, script not found",
                    _wp.path
                );
            }
        }
        (hm, is_flow_worker, dedicated_handles)
    } else {
        (HashMap::new(), false, dedicated_handles)
    }
}

#[derive(Debug, Clone)]
pub enum SpawnWorker {
    Script { path: String, hash: Option<ScriptHash> },
    RawScript { path: String, content: String, lock: Option<String>, lang: ScriptLang },
}

// spawn one dedicated worker and return the key, the channel sender and the join handle
// note that for it will return none for language that do not support dedicated workers
// note that go using cache binary does not need dedicated workers so all languages are supported
#[cfg(feature = "enterprise")]
async fn spawn_dedicated_worker(
    sw: SpawnWorker,
    w_id: &str,
    killpill_tx: tokio::sync::broadcast::Sender<()>,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    db: &DB,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    job_completed_tx: &JobCompletedSender,
    node_id: Option<String>,
) -> Option<DedicatedWorker> {
    use windmill_common::{
        error::Error,
        scripts::{ScriptHash, ScriptLang},
        utils::rd_string,
    };

    use crate::{build_envs, get_script_content_by_hash, ContentReqLangEnvs, JOB_TOKEN};

    #[cfg(not(feature = "enterprise"))]
    {
        tracing::error!("Dedicated worker is an enterprise feature");
        killpill_tx.send(()).expect("send");
        return None;
    }

    #[cfg(feature = "enterprise")]
    {
        let (dedicated_worker_tx, dedicated_worker_rx) =
            tokio::sync::mpsc::channel::<std::sync::Arc<QueuedJob>>(MAX_BUFFERED_DEDICATED_JOBS);
        let killpill_rx = killpill_rx.resubscribe();
        let db = db.clone();
        let base_internal_url = base_internal_url.to_string();
        let worker_name = worker_name.to_string();
        let job_completed_tx = job_completed_tx.clone();
        let job_dir = format!(
            "{}/dedicated{}",
            worker_dir,
            node_id
                .as_ref()
                .map(|x| format!("-{x}"))
                .unwrap_or_else(|| "".to_string())
        );
        tokio::fs::create_dir_all(&job_dir)
            .await
            .expect("create dir");

        let path = match &sw {
            SpawnWorker::RawScript { path, .. } => path.to_string(),
            SpawnWorker::Script { path, .. } => path.to_string(),
        };

        let path2 = path.clone();
        let w_id = w_id.to_string();

        let (content, lock, language, envs, codebase) = match sw.clone() {
            SpawnWorker::Script { path, hash } => {
                let q = if let Some(hash) = hash {
                    get_script_content_by_hash(&hash, &w_id, &db).await.map(
                        |r: ContentReqLangEnvs| {
                            Some((r.content, r.lockfile, r.language, r.envs, r.codebase))
                        },
                    )
                } else {
                    sqlx::query_as::<_, (String, Option<String>, Option<ScriptLang>, Option<Vec<String>>, bool, Option<ScriptHash>)>(
                        "SELECT content, lock, language, envs, codebase IS NOT NULL, hash FROM script WHERE path = $1 AND workspace_id = $2 AND
                            created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND workspace_id = $2 AND
                            deleted = false AND lock IS not NULL AND lock_error_logs IS NULL)",
                    )
                    .bind(&path)
                    .bind(&w_id)
                    .fetch_optional(&db)
                    .await
                    .map_err(|e| Error::InternalErr(format!("expected content and lock: {e:#}")))
                    .map(|x| x.map(|y| (y.0, y.1, y.2, y.3, if y.4 { y.5.map(|z| z.to_string()) } else { None })))
                };
                if let Ok(q) = q {
                    if let Some(wp) = q {
                        wp
                    } else {
                        tracing::error!(
                            "Failed to fetch script `{}` in workspace {} for dedicated worker.",
                            path,
                            w_id
                        );
                        return None;
                    }
                } else {
                    tracing::error!("Failed to fetch script for dedicated worker");
                    killpill_tx.send(()).expect("send");
                    return None;
                }
            }
            SpawnWorker::RawScript { content, lock, lang, .. } => {
                (content, lock, Some(lang), None, None)
            }
        };

        match language {
            Some(ScriptLang::Python3) | Some(ScriptLang::Bun) | Some(ScriptLang::Deno) => {}
            _ => return None,
        }

        let handle = tokio::spawn(async move {
            let token = if let Some(token) = JOB_TOKEN.as_ref() {
                token.clone()
            } else {
                let token = rd_string(32);
                if let Err(e) = sqlx::query_scalar!(
                    "INSERT INTO token
                    (token, label, super_admin, email)
                    VALUES ($1, $2, $3, $4)",
                    token,
                    "dedicated_worker",
                    true,
                    "dedicated_worker@windmill.dev"
                )
                .execute(&db)
                .await
                {
                    tracing::error!("failed to create token for dedicated worker: {:?}", e);
                    killpill_tx.clone().send(()).expect("send");
                };
                token
            };

            let worker_envs = build_envs(envs).expect("failed to build envs");

            if let Err(e) = match language {
                Some(ScriptLang::Python3) => {
                    crate::python_executor::start_worker(
                        lock,
                        &db,
                        &content,
                        &base_internal_url,
                        &job_dir,
                        &worker_name,
                        worker_envs,
                        &w_id,
                        &path,
                        &token,
                        job_completed_tx,
                        dedicated_worker_rx,
                        killpill_rx,
                    )
                    .await
                }
                Some(ScriptLang::Bun) => {
                    crate::bun_executor::start_worker(
                        lock,
                        codebase,
                        &db,
                        &content,
                        &base_internal_url,
                        &job_dir,
                        &worker_name,
                        worker_envs,
                        &w_id,
                        &path,
                        &token,
                        job_completed_tx,
                        dedicated_worker_rx,
                        killpill_rx,
                    )
                    .await
                }
                Some(ScriptLang::Deno) => {
                    crate::deno_executor::start_worker(
                        &content,
                        &base_internal_url,
                        &job_dir,
                        &worker_name,
                        worker_envs,
                        &w_id,
                        &path,
                        &token,
                        job_completed_tx,
                        dedicated_worker_rx,
                        killpill_rx,
                        &db,
                    )
                    .await
                }
                _ => unreachable!("Non supported language for dedicated worker"),
            } {
                tracing::error!("error in dedicated worker for {sw:#?}: {:?}", e);
            };
            if let Err(e) = killpill_tx.clone().send(()) {
                tracing::error!("failed to send final killpill to dedicated worker: {:?}", e);
            }
        });
        return Some((node_id.unwrap_or(path2), dedicated_worker_tx, Some(handle)));
        // (Some(dedi_path), Some(dedicated_worker_tx), Some(handle))
    }
}
