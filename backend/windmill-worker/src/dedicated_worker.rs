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
#[cfg(feature = "enterprise")]
use windmill_common::flows::FlowNodeId;
use windmill_common::flows::FlowValue;
use windmill_common::worker::WORKER_CONFIG;
use windmill_common::KillpillSender;
use windmill_common::{
    cache, error,
    flows::{FlowModule, FlowModuleValue},
    scripts::{ScriptHash, ScriptLang},
    variables,
    worker::to_raw_value,
    DB,
};
use windmill_queue::append_logs;
use windmill_queue::MiniPulledJob;
#[cfg(feature = "enterprise")]
use windmill_queue::{DedicatedWorkerJob, FlowRunners};

use anyhow::Context;

use crate::{common::start_child_process, JobCompletedSender, MAX_BUFFERED_DEDICATED_JOBS};

use futures::{future, Future};
use std::{collections::HashMap, task::Poll};

use tokio::sync::{mpsc::Receiver, RwLock};

// Global registry of dynamic runner pools, keyed by flow root job ID
lazy_static::lazy_static! {
    static ref FLOW_RUNNERS: Arc<RwLock<HashMap<String, DedicatedWorker>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

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
    mut jobs_rx: Receiver<DedicatedWorkerJob>,
    worker_name: &str,
    db: &DB,
    script_path: &str,
    mode: &str,
    client: windmill_common::client::AuthedClient,
) -> std::result::Result<(), error::Error> {
    //do not cache local dependencies

    use tokio::sync::oneshot;
    use windmill_queue::{JobCompleted, MiniCompletedJob};

    use crate::{handle_child::process_status, PROXY_ENVS};
    let cmd_name = format!("dedicated {command_path}");
    let mut child = {
        let mut cmd = Command::new(command_path);
        cmd.current_dir(job_dir)
            .env_clear()
            .envs(context_envs)
            .envs(envs)
            .envs(PROXY_ENVS.clone())
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
        start_child_process(cmd, command_path, false).await?
    };

    let stdout = child
        .stdout()
        .take()
        .expect("child did not have a handle to stdout");

    let stderr = child
        .stderr()
        .take()
        .expect("child did not have a handle to stderr");

    let mut reader = BufReader::new(stdout).lines();

    let mut err_reader = BufReader::new(stderr).lines();

    let mut stdin = child
        .stdin()
        .take()
        .expect("child did not have a handle to stdin");

    // Ensure the child process is spawned in the runtime so it can
    // make progress on its own while we await for any output.
    let child = tokio::spawn(async move {
        let status = Box::into_pin(child.wait())
            .await
            .expect("child process encountered an error");
        if let Err(e) = process_status(&cmd_name, status, vec![]) {
            tracing::error!("child exit status was not success: {e:#}");
        } else {
            tracing::info!("child exit status was success");
        }
    });

    let mut jobs: VecDeque<(
        MiniCompletedJob,
        Option<Arc<FlowRunners>>,
        Option<oneshot::Sender<()>>,
    )> = VecDeque::with_capacity(MAX_BUFFERED_DEDICATED_JOBS);
    // let mut i = 0;
    // let mut j = 0;
    let mut alive = true;

    let init_log = format!("dedicated worker {mode}: {worker_name}\n\n");
    let mut logs = init_log.clone();
    loop {
        use crate::common::transform_json;

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
                        let (job, flow_runners, done_tx) = jobs.pop_front().expect("pop");
                        tracing::info!("job completed on dedicated worker {script_path}: {}", job.id);
                        match serde_json::from_str::<Box<serde_json::value::RawValue>>(&line.replace("wm_res[success]:", "").replace("wm_res[error]:", "")) {
                            Ok(result) => {
                                let result = Arc::new(result);
                                append_logs(&job.id, &job.workspace_id,  logs.clone(), &db.into()).await;
                                if line.starts_with("wm_res[success]:") {
                                    job_completed_tx.send_job(JobCompleted { job , result, result_columns: None, mem_peak: 0, canceled_by: None, success: true, cached_res_path: None, token: token.to_string(), duration: None, preprocessed_args: None, has_stream: Some(false), from_cache: None, flow_runners, done_tx }, true).await.unwrap()
                                } else {
                                    job_completed_tx.send_job(JobCompleted { job , result, result_columns: None, mem_peak: 0, canceled_by: None, success: false, cached_res_path: None, token: token.to_string(), duration: None, preprocessed_args: None, has_stream: Some(false), from_cache: None, flow_runners, done_tx }, true).await.unwrap()
                                }
                            },
                            Err(e) => {
                                tracing::error!("Could not deserialize job result `{line}`: {e:?}");
                                job_completed_tx.send_job(JobCompleted { job , result: Arc::new(to_raw_value(&serde_json::json!({"error": format!("Could not deserialize job result `{line}`: {e:?}")}))), result_columns: None, mem_peak: 0, canceled_by: None, success: false, cached_res_path: None, token: token.to_string(), duration: None, preprocessed_args: None, has_stream: Some(false), from_cache: None, flow_runners, done_tx }, true).await.unwrap();
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
            dedicated_job = conditional_polling(jobs_rx.recv(), alive && jobs.len() < MAX_BUFFERED_DEDICATED_JOBS) => {
                // i += 1;
                if let Some(DedicatedWorkerJob { job, flow_runners, done_tx }) = dedicated_job {
                    let id = job.id;
                    let args = if let Some(args) = job.args.as_ref() {
                        if let Some(x) = transform_json(&client, &job.workspace_id, &args.0, &job, &db.into()).await? {
                            serde_json::to_string(&x).unwrap_or_else(|_| "{}".to_string())
                        } else {
                            serde_json::to_string(&args).unwrap_or_else(|_| "{}".to_string())
                        }
                    } else {
                        "{}".to_string()
                    };
                    jobs.push_back((MiniCompletedJob::from(job), flow_runners, done_tx));
                    tracing::info!("received job and adding to queue on dedicated worker for {script_path}: {} (queue_size: {})", id, jobs.len());

                    // write_stdin(&mut stdin, &serde_json::to_string(&job.args.unwrap_or_else(|| serde_json::json!({"x": job.id}))).expect("serialize")).await?;
                    write_stdin(&mut stdin, &args).await?;
                    stdin.flush().await.context("stdin flush")?;
                    // tracing::info!("wrote job to stdin for {script_path}: {} (queue_size: {})", id, jobs.len());
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

type DedicatedWorker = (String, Sender<DedicatedWorkerJob>, Option<JoinHandle<()>>);

// spawn one dedicated worker per compatible steps of the flow, associating the node id to the dedicated worker channel send
#[async_recursion]
#[cfg(feature = "enterprise")]
async fn spawn_dedicated_workers_for_flow(
    mut modules: Vec<FlowModule>,
    failure_module: Option<&Box<FlowModule>>,
    modules_node: Option<FlowNodeId>,
    w_id: &str,
    path: &str,
    killpill_tx: Option<KillpillSender>,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    db: &DB,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    job_completed_tx: &JobCompletedSender,
    flow_job_id: Option<&str>,
    parent_module_id: Option<String>,
) -> error::Result<Vec<DedicatedWorker>> {
    use crate::worker_flow::is_simple_modules;

    let mut workers = vec![];
    let mut script_path_to_worker: HashMap<String, Sender<DedicatedWorkerJob>> = HashMap::new();

    if let Some(modules_node) = modules_node {
        modules = cache::flow::fetch_flow(db, modules_node)
            .await
            .map(|data| data.value().modules.clone())?
    }

    let simple_parent_module_id =
        if parent_module_id.is_some() && is_simple_modules(&modules, failure_module) {
            parent_module_id
        } else {
            None
        };

    for module in modules.iter() {
        let value = module.get_value();
        if let Ok(value) = value {
            match value {
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
                            Some(
                                simple_parent_module_id
                                    .clone()
                                    .unwrap_or_else(|| module.id.clone()),
                            ),
                            flow_job_id,
                        )
                        .await?
                        {
                            script_path_to_worker.insert(key, dedi_w.1.clone());
                            workers.push(dedi_w);
                        }
                    }
                }
                FlowModuleValue::ForloopFlow { modules, modules_node, .. } => {
                    let w = spawn_dedicated_workers_for_flow(
                        modules,
                        failure_module,
                        modules_node,
                        w_id,
                        path,
                        killpill_tx.clone(),
                        killpill_rx,
                        db,
                        worker_dir,
                        base_internal_url,
                        worker_name,
                        job_completed_tx,
                        flow_job_id,
                        Some(module.id.clone()),
                    )
                    .await?;
                    workers.extend(w);
                }
                FlowModuleValue::WhileloopFlow { modules, modules_node, .. } => {
                    let w = spawn_dedicated_workers_for_flow(
                        modules,
                        failure_module,
                        modules_node,
                        w_id,
                        path,
                        killpill_tx.clone(),
                        killpill_rx,
                        db,
                        worker_dir,
                        base_internal_url,
                        worker_name,
                        job_completed_tx,
                        flow_job_id,
                        Some(module.id.clone()),
                    )
                    .await?;
                    workers.extend(w);
                }
                FlowModuleValue::BranchOne { branches, default, default_node, .. } => {
                    for (modules, modules_node) in branches
                        .into_iter()
                        .map(|x| (x.modules, x.modules_node))
                        .chain(std::iter::once((default, default_node)))
                    {
                        let w = spawn_dedicated_workers_for_flow(
                            modules,
                            failure_module,
                            modules_node,
                            w_id,
                            path,
                            killpill_tx.clone(),
                            killpill_rx,
                            db,
                            worker_dir,
                            base_internal_url,
                            worker_name,
                            job_completed_tx,
                            flow_job_id,
                            None,
                        )
                        .await?;
                        workers.extend(w);
                    }
                }
                FlowModuleValue::BranchAll { branches, .. } => {
                    for branch in branches {
                        let w = spawn_dedicated_workers_for_flow(
                            branch.modules,
                            failure_module,
                            branch.modules_node,
                            w_id,
                            path,
                            killpill_tx.clone(),
                            killpill_rx,
                            db,
                            worker_dir,
                            base_internal_url,
                            worker_name,
                            job_completed_tx,
                            flow_job_id,
                            None,
                        )
                        .await?;
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
                        Some(
                            simple_parent_module_id
                                .clone()
                                .unwrap_or_else(|| module.id.clone()),
                        ),
                        flow_job_id,
                    )
                    .await?
                    {
                        workers.push(dedi_w);
                    }
                }
                FlowModuleValue::FlowScript { id, language, .. } => {
                    let spawn = cache::flow::fetch_script(
                        &windmill_common::worker::Connection::Sql(db.clone()),
                        id,
                    )
                    .await
                    .map(|data| SpawnWorker::RawScript {
                        path: "".to_string(),
                        content: data.code.clone(),
                        lock: data.lock.clone(),
                        lang: language,
                    });
                    tracing::info!("spawn: {:?}", spawn);
                    match spawn {
                        Ok(spawn) => {
                            if let Some(dedi_w) = spawn_dedicated_worker(
                                spawn,
                                w_id,
                                killpill_tx.clone(),
                                killpill_rx,
                                db,
                                worker_dir,
                                base_internal_url,
                                worker_name,
                                job_completed_tx,
                                Some(
                                    simple_parent_module_id
                                        .clone()
                                        .unwrap_or_else(|| module.id.clone()),
                                ),
                                flow_job_id,
                            )
                            .await?
                            {
                                workers.push(dedi_w);
                            }
                        }
                        Err(err) => tracing::error!(
                            "failed to get script for module: {:?}, err: {:?}",
                            module,
                            err
                        ),
                    }
                }
                FlowModuleValue::Flow { .. } => (),
                FlowModuleValue::Identity => (),
                FlowModuleValue::AIAgent { .. } => (),
            }
        } else {
            tracing::error!("failed to get value for module: {:?}", module);
        }
    }
    Ok(workers)
}

pub async fn spawn_flow_module_runners(
    job: &MiniPulledJob,
    module: &FlowModule,
    failure_module: Option<&Box<FlowModule>>,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    db: &DB,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    job_completed_tx: &JobCompletedSender,
) -> error::Result<(
    HashMap<String, Sender<DedicatedWorkerJob>>,
    Vec<JoinHandle<()>>,
)> {
    let workers = spawn_dedicated_workers_for_flow(
        vec![module.clone()],
        failure_module,
        None,
        &job.workspace_id,
        job.runnable_path(),
        None,
        &killpill_rx,
        db,
        &worker_dir,
        base_internal_url,
        &worker_name,
        &job_completed_tx,
        Some(job.id.to_string().as_str()),
        None,
    )
    .await?;

    tracing::info!("spawned dedicated workers for flow: {:?}", workers);

    let mut dedicated_handles = vec![];
    let mut hm = HashMap::new();
    workers.into_iter().for_each(|(path, sender, handle)| {
        tracing::info!("spawned dedicated worker for flow: {}", path.as_str());
        if let Some(h) = handle {
            dedicated_handles.push(h);
        }
        hm.insert(path, sender);
    });
    Ok((hm, dedicated_handles))
}

pub async fn create_dedicated_worker_map(
    killpill_tx: &KillpillSender,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    db: &DB,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    job_completed_tx: &JobCompletedSender,
) -> (
    HashMap<String, Sender<DedicatedWorkerJob>>,
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
                "SELECT flow_version.value AS \"value!: sqlx::types::Json<Box<sqlx::types::JsonRawValue>>\" 
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
                    let value = serde_json::from_str::<FlowValue>(v.get()).map_err(|err| {
                        Error::internal_err(format!(
                            "could not convert json to flow for {flow_path}: {err:?}"
                        ))
                    });
                    if let Ok(flow) = value {
                        match spawn_dedicated_workers_for_flow(
                            flow.modules,
                            flow.failure_module.as_ref(),
                            None,
                            &_wp.workspace_id,
                            &_wp.path,
                            Some(killpill_tx.clone()),
                            &killpill_rx,
                            db,
                            &worker_dir,
                            base_internal_url,
                            &worker_name,
                            &job_completed_tx,
                            None,
                            None,
                        )
                        .await
                        {
                            Ok(workers) => {
                                workers.into_iter().for_each(|(path, sender, handle)| {
                                    tracing::info!(
                                        "spawned dedicated worker for flow: {}",
                                        path.as_str()
                                    );
                                    if let Some(h) = handle {
                                        dedicated_handles.push(h);
                                    }
                                    hm.insert(path, sender);
                                });
                            }
                            Err(e) => tracing::error!(
                                "failed to spawn dedicated workers for flow {}: {}",
                                flow_path,
                                e
                            ),
                        };
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
            match spawn_dedicated_worker(
                SpawnWorker::Script { path: _wp.path.clone(), hash: None },
                &_wp.workspace_id,
                Some(killpill_tx.clone()),
                &killpill_rx,
                db,
                &worker_dir,
                base_internal_url,
                &worker_name,
                &job_completed_tx,
                None,
                None,
            )
            .await
            {
                Ok(Some((path, sender, handle))) => {
                    if let Some(h) = handle {
                        dedicated_handles.push(h);
                    }
                    hm.insert(path, sender);
                }
                Ok(None) => {
                    tracing::error!(
                        "Language of script {} not supported for dedicated worker",
                        _wp.path
                    );
                    killpill_tx.send();
                }
                Err(e) => {
                    tracing::error!("failed to spawn dedicated worker for {}: {}", _wp.path, e);
                    killpill_tx.send();
                }
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
    killpill_tx: Option<KillpillSender>,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    db: &DB,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    job_completed_tx: &JobCompletedSender,
    node_id: Option<String>,
    flow_job_id: Option<&str>,
) -> error::Result<Option<DedicatedWorker>> {
    use windmill_common::{
        error::Error,
        scripts::{ScriptHash, ScriptLang},
        utils::rd_string,
    };

    use crate::{build_envs, get_script_content_by_hash, ContentReqLangEnvs};

    let (dedicated_worker_tx, dedicated_worker_rx) =
        tokio::sync::mpsc::channel::<DedicatedWorkerJob>(MAX_BUFFERED_DEDICATED_JOBS);
    let killpill_rx = killpill_rx.resubscribe();
    let db2 = db.clone();
    let base_internal_url = base_internal_url.to_string();
    let worker_name = worker_name.to_string();
    let job_completed_tx = job_completed_tx.clone();
    let job_dir = format!(
        "{}/dedicated{}{}",
        worker_dir,
        flow_job_id
            .as_ref()
            .map(|x| format!("-{x}"))
            .unwrap_or_else(|| "".to_string()),
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
                get_script_content_by_hash(&hash, &w_id, &db2.into())
                    .await
                    .map(|r: ContentReqLangEnvs| {
                        Some((r.content, r.lockfile, r.language, r.envs, r.codebase))
                    })
            } else {
                sqlx::query_as::<_, (String, Option<String>, Option<ScriptLang>, Option<Vec<String>>, bool, Option<ScriptHash>)>(
                        "SELECT content, lock, language, envs, codebase IS NOT NULL, hash FROM script WHERE path = $1 AND workspace_id = $2 AND
                            archived = false AND lock IS not NULL AND lock_error_logs IS NULL ORDER BY created_at DESC LIMIT 1",
                    )
                    .bind(&path)
                    .bind(&w_id)
                    .fetch_optional(&db2)
                    .await
                    .map_err(|e| Error::internal_err(format!("expected content and lock: {e:#}")))
                    .map(|x| x.map(|y| (y.0, y.1, y.2, y.3, if y.4 { y.5.map(|z| z.to_string()) } else { None })))
            };
            if let Ok(q) = q {
                if let Some(wp) = q {
                    wp
                } else {
                    return Err(Error::internal_err(format!(
                        "Failed to fetch script `{}` in workspace {} for dedicated worker.",
                        path, w_id
                    )));
                }
            } else {
                return Err(Error::internal_err(format!(
                    "Failed to fetch script for dedicated worker"
                )));
            }
        }
        SpawnWorker::RawScript { content, lock, lang, .. } => {
            (content, lock, Some(lang), None, None)
        }
    };

    match language {
        Some(ScriptLang::Python3) | Some(ScriptLang::Bun) | Some(ScriptLang::Deno) => {}
        _ => return Ok(None),
    }

    let db = db.clone();
    let handle = tokio::spawn(async move {
        let token = {
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
                if let Some(killpill_tx) = killpill_tx.as_ref() {
                    killpill_tx.send();
                }
                return;
            };
            token
        };

        let worker_envs = match build_envs(envs.as_ref()) {
            Ok(envs) => envs,
            Err(e) => {
                tracing::error!("failed to build envs for dedicated worker: {:?}", e);
                if let Some(killpill_tx) = killpill_tx {
                    killpill_tx.send();
                }
                return;
            }
        };

        let client = windmill_common::client::AuthedClient {
            base_internal_url: base_internal_url.to_string(),
            workspace: w_id.to_string(),
            token: token.to_string(),
            force_client: None,
        };

        if let Err(e) = match language {
            Some(ScriptLang::Python3) => {
                #[cfg(not(feature = "python"))]
                {
                    tracing::error!("Python requires the python feature to be enabled");
                    if let Some(killpill_tx) = killpill_tx {
                        killpill_tx.send();
                    }
                    return;
                }

                #[cfg(feature = "python")]
                crate::python_executor::start_worker(
                    lock.as_ref(),
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
                    client,
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
                    client,
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
                    client,
                )
                .await
            }
            _ => unreachable!("Non supported language for dedicated worker"),
        } {
            tracing::error!("error in dedicated worker for {sw:#?}: {:?}", e);
        };
        if let Some(killpill_tx) = killpill_tx {
            killpill_tx.clone().send();
        }
    });
    return Ok(Some((
        node_id.unwrap_or(path2),
        dedicated_worker_tx,
        Some(handle),
    )));
}
