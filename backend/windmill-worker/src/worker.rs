/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use itertools::Itertools;
use sqlx::{Pool, Postgres, Transaction};
use std::{borrow::Borrow, collections::HashMap, io, panic, process::Stdio, time::Duration};
use tracing::{trace_span, Instrument};
use uuid::Uuid;
use windmill_common::{
    error::{self, to_anyhow, Error},
    scripts::{ScriptHash, ScriptLang},
    utils::rd_string,
    variables,
};
use windmill_queue::{
    canceled_job_to_result, get_hub_script, get_queued_job, pull, JobKind, QueuedJob,
};

use serde_json::{json, Map, Value};

use tokio::{
    fs::{metadata, symlink, DirBuilder, File},
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::{
        mpsc::{self, Sender},
        oneshot, watch,
    },
    time::{interval, sleep, Instant, MissedTickBehavior},
};

use futures::{
    future::{self, ready, FutureExt},
    stream::{self, StreamExt},
};

use async_recursion::async_recursion;

use crate::{
    jobs::{add_completed_job, add_completed_job_error, error_to_result},
    worker_flow::{
        handle_flow, update_flow_status_after_job_completion, update_flow_status_in_progress,
    },
};

#[tracing::instrument(level = "trace", skip_all)]
pub async fn create_token_for_owner<'c>(
    mut tx: Transaction<'c, Postgres>,
    w_id: &str,
    owner: &str,
    label: &str,
    expires_in: i32,
    username: &str,
) -> error::Result<(Transaction<'c, Postgres>, String)> {
    // TODO: Bad implementation. We should not have access to this DB here.
    let token: String = rd_string(30);
    let is_super_admin = username.contains('@')
        && sqlx::query_scalar!(
            "SELECT super_admin FROM password WHERE email = $1",
            owner.split_once('/').map(|x| x.1).unwrap_or("")
        )
        .fetch_optional(&mut tx)
        .await?
        .unwrap_or(false);

    let expiration = sqlx::query_scalar!(
        "INSERT INTO token
            (workspace_id, token, owner, label, expiration, super_admin)
            VALUES ($1, $2, $3, $4, now() + ($5 || ' seconds')::interval, $6) RETURNING expiration",
        &w_id,
        token,
        owner,
        label,
        expires_in.to_string(),
        is_super_admin
    )
    .fetch_one(&mut tx)
    .await?;

    let mut truncated_token = token[..10].to_owned();
    truncated_token.push_str("*****");

    windmill_audit::audit_log(
        &mut tx,
        &username,
        "users.token.create",
        windmill_audit::ActionKind::Create,
        w_id,
        Some(&truncated_token),
        Some(
            [
                Some(("label", label)),
                expiration
                    .map(|x| x.to_string())
                    .as_ref()
                    .map(|exp| ("expiration", &exp[..])),
            ]
            .into_iter()
            .flatten()
            .collect(),
        ),
    )
    .await?;
    Ok((tx, token))
}

const TMP_DIR: &str = "/tmp/windmill";
const PIP_SUPERCACHE_DIR: &str = "/tmp/windmill/cache/pip_permanent";
const PIP_CACHE_DIR: &str = "/tmp/windmill/cache/pip";
const DENO_CACHE_DIR: &str = "/tmp/windmill/cache/deno";
const GO_CACHE_DIR: &str = "/tmp/windmill/cache/go";
const NUM_SECS_ENV_CHECK: u64 = 15;
const DEFAULT_HEAVY_DEPS: [&str; 18] = [
    "numpy",
    "pandas",
    "anyio",
    "attrs",
    "certifi",
    "h11",
    "httpcore",
    "httpx",
    "idna",
    "python-dateutil",
    "rfc3986",
    "six",
    "sniffio",
    "windmill-api",
    "wmill",
    "psycopg2-binary",
    "matplotlib",
    "seaborn",
];

const INCLUDE_DEPS_PY_SH_CONTENT: &str = include_str!("../nsjail/download_deps.py.sh");
const NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT: &str = include_str!("../nsjail/download.py.config.proto");
const NSJAIL_CONFIG_RUN_PYTHON3_CONTENT: &str = include_str!("../nsjail/run.python3.config.proto");

const NSJAIL_CONFIG_RUN_GO_CONTENT: &str = include_str!("../nsjail/run.go.config.proto");

const NSJAIL_CONFIG_RUN_DENO_CONTENT: &str = include_str!("../nsjail/run.deno.config.proto");
const MAX_LOG_SIZE: u32 = 200000;
const GO_REQ_SPLITTER: &str = "//go.sum";

#[derive(Clone)]
pub struct Metrics {
    pub worker_execution_failed: prometheus::IntCounter,
}

#[derive(Clone, Debug)]
pub struct WorkerConfig {
    pub base_internal_url: String,
    pub base_url: String,
    pub disable_nuser: bool,
    pub disable_nsjail: bool,
    pub keep_job_dir: bool,
}

lazy_static::lazy_static! {
    static ref WORKER_STARTED: prometheus::IntGauge = prometheus::register_int_gauge!(
        "worker_started",
        "Total number of workers started."
    )
    .unwrap();
    static ref QUEUE_ZOMBIE_RESTART_COUNT: prometheus::IntCounter = prometheus::register_int_counter!(
        "queue_zombie_restart_count",
        "Total number of jobs restarted due to ping timeout."
    )
    .unwrap();
    static ref QUEUE_ZOMBIE_DELETE_COUNT: prometheus::IntCounter = prometheus::register_int_counter!(
        "queue_zombie_delete_count",
        "Total number of jobs deleted due to their ping timing out in an unrecoverable state."
    )
    .unwrap();
    static ref WORKER_UPTIME_OPTS: prometheus::Opts = prometheus::opts!(
        "worker_uptime",
        "Total number of milliseconds since the worker has started"
    );
}

#[tracing::instrument(level = "trace")]
pub async fn run_worker(
    db: &Pool<Postgres>,
    timeout: i32,
    worker_instance: &str,
    worker_name: String,
    i_worker: u64,
    num_workers: u64,
    ip: &str,
    sleep_queue: u64,
    worker_config: WorkerConfig,
    mut rx: tokio::sync::broadcast::Receiver<()>,
) {
    let start_time = Instant::now();

    let worker_dir = format!("{TMP_DIR}/{worker_name}");
    tracing::debug!(worker_dir = %worker_dir, worker_name = %worker_name, "Creating worker dir");

    for x in [
        &worker_dir,
        PIP_SUPERCACHE_DIR,
        PIP_CACHE_DIR,
        DENO_CACHE_DIR,
        GO_CACHE_DIR,
    ] {
        DirBuilder::new()
            .recursive(true)
            .create(x)
            .await
            .expect("could not create initial worker dir");
    }

    let _ = write_file(
        &worker_dir,
        "download_deps.py.sh",
        INCLUDE_DEPS_PY_SH_CONTENT,
    )
    .await;

    let mut last_ping = Instant::now() - Duration::from_secs(NUM_SECS_ENV_CHECK + 1);

    insert_initial_ping(worker_instance, &worker_name, ip, db).await;

    let uptime_metric = prometheus::register_int_counter!(WORKER_UPTIME_OPTS
        .clone()
        .const_label("name", &worker_name))
    .unwrap();
    uptime_metric.inc_by(
        ((Instant::now() - start_time).as_millis() - uptime_metric.get() as u128)
            .try_into()
            .unwrap(),
    );

    let worker_execution_duration = prometheus::register_histogram_vec!(
        prometheus::HistogramOpts::new(
            "worker_execution_duration",
            "Duration between receiving a job and completing it",
        )
        .const_label("name", &worker_name),
        &["workspace_id", "language"],
    )
    .expect("register prometheus metric");

    let worker_execution_failed = prometheus::register_int_counter_vec!(
        prometheus::Opts::new("worker_execution_failed", "Number of failed jobs",)
            .const_label("name", &worker_name),
        &["workspace_id", "language"],
    )
    .expect("register prometheus metric");

    let worker_execution_count = prometheus::register_int_counter_vec!(
        prometheus::Opts::new("worker_execution_count", "Number of executed jobs",)
            .const_label("name", &worker_name),
        &["workspace_id", "language"],
    )
    .expect("register prometheus metric");

    let mut jobs_executed = 0;

    let deno_path = std::env::var("DENO_PATH").unwrap_or_else(|_| "/usr/bin/deno".to_string());
    let go_path = std::env::var("GO_PATH").unwrap_or_else(|_| "/usr/bin/go".to_string());
    let python_path =
        std::env::var("PYTHON_PATH").unwrap_or_else(|_| "/usr/local/bin/python3".to_string());
    let python_heavy_deps = std::env::var("PYTHON_HEAVY_DEPS")
        .map(|x| x.split(',').map(|x| x.to_string()).collect::<Vec<_>>())
        .unwrap_or_else(|_| vec![]);
    let nsjail_path = std::env::var("NSJAIL_PATH").unwrap_or_else(|_| "nsjail".to_string());
    let path_env = std::env::var("PATH").unwrap_or_else(|_| String::new());
    let gopath_env = std::env::var("GOPATH").unwrap_or_else(|_| String::new());
    let home_env = std::env::var("HOME").unwrap_or_else(|_| String::new());
    let pip_index_url = std::env::var("PIP_INDEX_URL").ok();
    let pip_extra_index_url = std::env::var("PIP_EXTRA_INDEX_URL").ok();
    let pip_trusted_host = std::env::var("PIP_TRUSTED_HOST").ok();
    let envs = Envs {
        deno_path,
        go_path,
        python_path,
        python_heavy_deps,
        nsjail_path,
        path_env,
        gopath_env,
        home_env,
        pip_index_url,
        pip_extra_index_url,
        pip_trusted_host,
    };
    WORKER_STARTED.inc();

    let (same_worker_tx, mut same_worker_rx) = mpsc::channel::<Uuid>(5);

    loop {
        uptime_metric.inc_by(
            ((Instant::now() - start_time).as_millis() - uptime_metric.get() as u128)
                .try_into()
                .unwrap(),
        );
        let do_break = async {
            if last_ping.elapsed().as_secs() > NUM_SECS_ENV_CHECK {
                sqlx::query!(
                    "UPDATE worker_ping SET ping_at = now(), jobs_executed = $1 WHERE worker = $2",
                    jobs_executed,
                    &worker_name
                )
                .execute(db)
                .await
                .expect("update worker ping");

                last_ping = Instant::now();
            }

            let (do_break, next_job) = async {
                tokio::select! {
                    biased;
                    _ = rx.recv() => {
                        println!("received killpill for worker {}", i_worker);
                        (true, Ok(None))
                    },
                    Some(job_id) = same_worker_rx.recv() => {
                        (false, sqlx::query_as::<_, QueuedJob>("SELECT * FROM queue WHERE id = $1")
                        .bind(job_id)
                        .fetch_optional(db)
                        .await
                        .map_err(|_| Error::InternalErr("Impossible to fetch same_worker job".to_string())))
                    },
                    job = pull(&db) => (false, job),
                }
            }.instrument(trace_span!("worker_get_next_job")).await;
            if do_break {
                return true;
            }

            match next_job {
                Ok(Some(job)) => {
                    let label_values = [
                        &job.workspace_id,
                        job.language.as_ref().map(|l| l.as_str()).unwrap_or(""),
                    ];

                    let _timer = worker_execution_duration
                        .with_label_values(label_values.as_slice())
                        .start_timer();

                    jobs_executed += 1;
                    worker_execution_count
                        .with_label_values(label_values.as_slice())
                        .inc();

                    let metrics = Metrics {
                        worker_execution_failed: worker_execution_failed
                            .with_label_values(label_values.as_slice()),
                    };

                    tracing::info!(worker = %worker_name, id = %job.id, "fetched job {}", job.id);

                    let job_dir = format!("{worker_dir}/{}", job.id);

                    DirBuilder::new()
                        .create(&job_dir)
                        .await
                        .expect("could not create job dir");

                    let same_worker = job.same_worker;
                    let is_flow = job.job_kind == JobKind::Flow || job.job_kind == JobKind::FlowPreview;

                    if is_flow && same_worker {
                        let target = &format!("{job_dir}/shared");
                        if let Some(parent_flow) = job.parent_job {
                            let parent_shared_dir = format!("{worker_dir}/{parent_flow}/shared");
                            if metadata(&parent_shared_dir).await.is_err() {
                                DirBuilder::new()
                                    .recursive(true)
                                    .create(&parent_shared_dir)
                                    .await
                                    .expect("could not create parent shared dir");
                            }
                            symlink(&parent_shared_dir, target)
                                .await
                                .expect("could not symlink target");
                        } else {
                            DirBuilder::new()
                                .create(target)
                                .await
                                .expect("could not create shared dir");
                        }
                    }
                    let tx = db.begin().await.expect("could not start token transaction");
                    let (tx, token) = create_token_for_owner(
                        tx,
                        &job.workspace_id,
                        &job.permissioned_as,
                        "ephemeral-script",
                        timeout * 2,
                        &job.created_by,
                    )
                    .await.expect("could not create job token");
                    tx.commit().await.expect("could not commit job token");
                    let job_client = windmill_api_client::create_client(&worker_config.base_url, token.clone());

                    if let Some(err) = handle_queued_job(
                        job.clone(),
                        db,
                        &job_client,
                        token,
                        timeout,
                        &worker_name,
                        &worker_dir,
                        &job_dir,
                        &worker_config,
                        metrics.clone(),
                        &envs,
                        same_worker_tx.clone(),
                        &worker_config.base_internal_url,
                    )
                    .await
                    .err()
                    {
                        handle_job_error(
                            db,
                            &job_client,
                            job,
                            err,
                            Some(metrics),
                            false,
                            same_worker_tx.clone(),
                            &worker_dir,
                            !worker_config.keep_job_dir,
                            &worker_config.base_internal_url,
                        )
                        .await;
                    };

                    if !worker_config.keep_job_dir && !(is_flow && same_worker) {
                        let _ = tokio::fs::remove_dir_all(job_dir).await;
                    }
                }
                Ok(None) => {
                    tokio::time::sleep(Duration::from_millis(sleep_queue * num_workers)).await
                }
                Err(err) => {
                    tracing::error!(worker = %worker_name, "run_worker: pulling jobs: {}", err);
                }
            };

            false
        }
        .instrument(trace_span!("worker_loop_iteration"))
        .await;
        if do_break {
            break;
        }
    }
}

async fn handle_job_error(
    db: &Pool<Postgres>,
    client: &windmill_api_client::Client,
    job: QueuedJob,
    err: Error,
    metrics: Option<Metrics>,
    unrecoverable: bool,
    same_worker_tx: Sender<Uuid>,
    worker_dir: &str,
    keep_job_dir: bool,
    base_internal_url: &str,
) {
    if job.is_flow_step || job.job_kind == JobKind::FlowPreview || job.job_kind == JobKind::Flow {
        let (flow, job_status_to_update) = if let Some(parent_job_id) = job.parent_job {
            (parent_job_id, job.id)
        } else {
            (job.id, Uuid::nil())
        };

        let mut output_map = serde_json::Map::new();
        error_to_result(&mut output_map, &err);
        let updated_flow = update_flow_status_after_job_completion(
            db,
            client,
            flow,
            &job_status_to_update,
            &job.workspace_id,
            false,
            serde_json::Value::Object(output_map),
            metrics.clone(),
            unrecoverable,
            same_worker_tx,
            worker_dir,
            keep_job_dir,
            base_internal_url,
            None,
        )
        .await;
        if let Err(err) = updated_flow {
            println!("error updating flow status: {}", err);

            if let Some(parent_job_id) = job.parent_job {
                if let Ok(mut tx) = db.begin().await {
                    if let Ok(Some(parent_job)) =
                        get_queued_job(parent_job_id, &job.workspace_id, &mut tx).await
                    {
                        let _ = add_completed_job_error(
                            db,
                            client,
                            &parent_job,
                            format!("Unexpected error during flow job error handling:\n{err}"),
                            err,
                            metrics.clone(),
                        )
                        .await;
                    }
                }
            }
        }
    }
    add_completed_job_error(
        db,
        client,
        &job,
        format!("Unexpected error during job execution:\n{err}"),
        &err,
        metrics,
    )
    .await
    .map(|(_, m)| m)
    .unwrap_or_else(|_| Map::new());

    tracing::error!(job_id = %job.id, err = err.alt(), "error handling job: {} {} {}", job.id, job.workspace_id, job.created_by);
}

async fn insert_initial_ping(
    worker_instance: &str,
    worker_name: &str,
    ip: &str,
    db: &Pool<Postgres>,
) {
    sqlx::query!(
        "INSERT INTO worker_ping (worker_instance, worker, ip) VALUES ($1, $2, $3)",
        worker_instance,
        worker_name,
        ip
    )
    .execute(db)
    .await
    .expect("insert worker_ping initial value");
}

struct Envs {
    deno_path: String,
    go_path: String,
    python_path: String,
    python_heavy_deps: Vec<String>,
    nsjail_path: String,
    path_env: String,
    gopath_env: String,
    home_env: String,
    pip_index_url: Option<String>,
    pip_extra_index_url: Option<String>,
    pip_trusted_host: Option<String>,
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_queued_job(
    job: QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &windmill_api_client::Client,
    token: String,
    timeout: i32,
    worker_name: &str,
    worker_dir: &str,
    job_dir: &str,
    worker_config: &WorkerConfig,
    metrics: Metrics,
    envs: &Envs,
    same_worker_tx: Sender<Uuid>,
    base_internal_url: &str,
) -> windmill_common::error::Result<()> {
    if job.canceled {
        return Err(Error::ExecutionErr(canceled_job_to_result(&job)))?;
    }
    match job.job_kind {
        JobKind::FlowPreview | JobKind::Flow => {
            let args = job.args.clone().unwrap_or(Value::Null);
            handle_flow(
                &job,
                db,
                client,
                args,
                same_worker_tx,
                worker_dir,
                base_internal_url,
            )
            .await?;
        }
        _ => {
            let mut logs = "".to_string();

            if job.is_flow_step {
                update_flow_status_in_progress(
                    db,
                    &job.workspace_id,
                    job.parent_job
                        .ok_or_else(|| Error::InternalErr(format!("expected parent job")))?,
                    job.id,
                )
                .await?;
            }

            tracing::info!(
                worker = %worker_name,
                job_id = %job.id,
                workspace_id = %job.workspace_id,
                "handling job {}",
                job.id
            );

            logs.push_str(&format!("job {} on worker {}\n", &job.id, &worker_name));
            let result = match job.job_kind {
                JobKind::Dependencies => {
                    handle_dependency_job(&job, &mut logs, job_dir, db, timeout, &envs).await
                }
                JobKind::Identity => Ok(job.args.clone().unwrap_or_else(|| Value::Null)),
                _ => {
                    handle_code_execution_job(
                        &job,
                        db,
                        client,
                        token,
                        job_dir,
                        worker_dir,
                        &mut logs,
                        timeout,
                        worker_config,
                        envs,
                    )
                    .await
                }
            };

            match result {
                Ok(r) => {
                    add_completed_job(db, client, &job, true, false, r.clone(), logs).await?;
                    if job.is_flow_step {
                        if let Some(parent_job) = job.parent_job {
                            update_flow_status_after_job_completion(
                                db,
                                client,
                                parent_job,
                                &job.id,
                                &job.workspace_id,
                                true,
                                r,
                                Some(metrics.clone()),
                                false,
                                same_worker_tx.clone(),
                                worker_dir,
                                worker_config.keep_job_dir,
                                &worker_config.base_internal_url,
                                None,
                            )
                            .await?;
                        }
                    }
                }
                Err(e) => {
                    let error_message = match e {
                        Error::ExitStatus(_) => {
                            let last_10_log_lines = logs
                                .lines()
                                .skip(logs.lines().count().max(10) - 10)
                                .join("\n")
                                .to_string()
                                .replace("\n\n", "\n");

                            let log_lines = last_10_log_lines
                                .split("CODE EXECUTION ---")
                                .last()
                                .unwrap_or(&logs);
                            format!("Error during execution of the script:\n{}", log_lines)
                        }
                        err @ _ => format!("error before termination: {err:#?}"),
                    };

                    let (_, output_map) = add_completed_job_error(
                        db,
                        client,
                        &job,
                        logs,
                        error_message,
                        Some(metrics.clone()),
                    )
                    .await?;
                    if job.is_flow_step {
                        if let Some(parent_job) = job.parent_job {
                            update_flow_status_after_job_completion(
                                db,
                                client,
                                parent_job,
                                &job.id,
                                &job.workspace_id,
                                false,
                                serde_json::Value::Object(output_map),
                                Some(metrics),
                                false,
                                same_worker_tx,
                                worker_dir,
                                worker_config.keep_job_dir,
                                &worker_config.base_internal_url,
                                None,
                            )
                            .await?;
                        }
                    }
                }
            };
        }
    }
    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
async fn write_file(dir: &str, path: &str, content: &str) -> error::Result<File> {
    let path = format!("{}/{}", dir, path);
    let mut file = File::create(&path).await?;
    file.write_all(content.as_bytes()).await?;
    file.flush().await?;
    Ok(file)
}

#[async_recursion]
async fn transform_json_value(
    client: &windmill_api_client::Client,
    workspace: &str,
    v: Value,
) -> error::Result<Value> {
    match v {
        Value::String(y) if y.starts_with("$var:") => {
            let path = y.strip_prefix("$var:").unwrap();
            let v = client
                .get_variable(workspace, path, Some(true))
                .await
                .map_err(to_anyhow)
                .map(|v| v.into_inner())?
                .value
                .map_or_else(
                    || Err(Error::NotFound(format!("Variable not found at {path}"))),
                    |e| Ok(e),
                )?;
            Ok(Value::String(v))
        }
        Value::String(y) if y.starts_with("$res:") => {
            let path = y.strip_prefix("$res:").unwrap();
            if path.split("/").count() < 2 {
                return Err(Error::InternalErr(
                    format!("invalid resource path: {path}",),
                ));
            }
            let v = client
                .get_resource_value(workspace, path)
                .await
                .map_err(to_anyhow)?
                .into_inner();
            transform_json_value(client, workspace, v).await
        }
        Value::Object(mut m) => {
            for (a, b) in m.clone().into_iter() {
                m.insert(a, transform_json_value(client, workspace, b).await?);
            }
            Ok(Value::Object(m))
        }
        a @ _ => Ok(a),
    }
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_code_execution_job(
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &windmill_api_client::Client,
    token: String,
    job_dir: &str,
    worker_dir: &str,
    logs: &mut String,
    timeout: i32,
    worker_config: &WorkerConfig,
    envs: &Envs,
) -> error::Result<serde_json::Value> {
    let (inner_content, requirements_o, language) = if matches!(job.job_kind, JobKind::Preview) {
        let code = (job.raw_code.as_ref().unwrap_or(&"no raw code".to_owned())).to_owned();
        (code, None, job.language.to_owned())
    } else if matches!(job.job_kind, JobKind::Script_Hub) {
        let code = (job.raw_code.as_ref().unwrap_or(&"no raw code".to_owned())).to_owned();
        let script = get_hub_script(
            job.script_path
                .clone()
                .unwrap_or_else(|| "missing script path".to_string()),
            None,
            &job.created_by,
        )
        .await?;
        (code, script.lockfile, job.language.to_owned())
    } else {
        sqlx::query_as::<_, (String, Option<String>, Option<ScriptLang>)>(
            "SELECT content, lock, language FROM script WHERE hash = $1 AND (workspace_id = $2 OR \
             workspace_id = 'starter')",
        )
        .bind(&job.script_hash.unwrap_or(ScriptHash(0)).0)
        .bind(&job.workspace_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| Error::InternalErr(format!("expected content and lock")))?
    };
    let worker_name = worker_dir.split("/").last().unwrap_or("unknown");
    let lang_str = job
        .language
        .as_ref()
        .map(|x| format!("{x:?}"))
        .unwrap_or_else(|| "NO_LANG".to_string());

    tracing::info!(
        worker_name = %worker_name,
        job_id = %job.id,
        workspace_id = %job.workspace_id,
        "started {} job {}",
        &lang_str,
        job.id
    );

    let shared_mount = if job.same_worker {
        format!(
            r#"
mount {{
    src: "{worker_dir}/{}/shared"
    dst: "/shared"
    is_bind: true
    rw: true
}}
        "#,
            job.parent_job.ok_or(Error::ExecutionErr(
                "no parent job, required for same worker job".to_string()
            ))?,
        )
    } else {
        "".to_string()
    };
    let result: error::Result<serde_json::Value> = match language {
        None => {
            return Err(Error::ExecutionErr(
                "Require language to be not null".to_string(),
            ))?;
        }
        Some(ScriptLang::Python3) => {
            handle_python_job(
                worker_config,
                envs,
                requirements_o,
                job_dir,
                worker_dir,
                worker_name,
                job,
                logs,
                db,
                client,
                token,
                timeout,
                &inner_content,
                &shared_mount,
            )
            .await
        }
        Some(ScriptLang::Deno) => {
            handle_deno_job(
                worker_config,
                envs,
                logs,
                job,
                db,
                client,
                token,
                job_dir,
                &inner_content,
                timeout,
                &shared_mount,
                requirements_o,
            )
            .await
        }
        Some(ScriptLang::Go) => {
            handle_go_job(
                worker_config,
                envs,
                logs,
                job,
                db,
                client,
                token,
                &inner_content,
                timeout,
                job_dir,
                requirements_o,
                &shared_mount,
            )
            .await
        }
    };
    tracing::info!(
        worker_name = %worker_name,
        job_id = %job.id,
        workspace_id = %job.workspace_id,
        is_ok = result.is_ok(),
        "finished {} job {}",
        &lang_str,
        job.id
    );
    result
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_go_job(
    WorkerConfig { base_internal_url, disable_nuser, disable_nsjail, base_url, .. }: &WorkerConfig,
    Envs { nsjail_path, go_path, path_env, gopath_env, home_env, .. }: &Envs,
    logs: &mut String,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &windmill_api_client::Client,
    token: String,
    inner_content: &str,
    timeout: i32,
    job_dir: &str,
    requirements_o: Option<String>,
    shared_mount: &str,
) -> Result<serde_json::Value, Error> {
    //go does not like executing modules at temp root
    let job_dir = &format!("{job_dir}/go");
    if let Some(requirements) = requirements_o {
        gen_go_mymod(inner_content, job_dir).await?;
        let (md, sum) = requirements
            .split_once(GO_REQ_SPLITTER)
            .ok_or(Error::ExecutionErr(
                "Invalid requirement file, missing splitter".to_string(),
            ))?;
        write_file(job_dir, "go.mod", md).await?;
        write_file(job_dir, "go.sum", sum).await?;
    } else {
        logs.push_str("\n\n--- GO DEPENDENCIES SETUP ---\n");
        set_logs(logs, job.id, db).await;

        install_go_dependencies(
            &job.id,
            inner_content,
            logs,
            job_dir,
            db,
            timeout,
            go_path,
            true,
        )
        .await?;
    }

    logs.push_str("\n\n--- GO CODE EXECUTION ---\n");
    set_logs(logs, job.id, db).await;
    create_args_and_out_file(client, job, job_dir).await?;
    {
        let sig = windmill_parser_go::parse_go_sig(&inner_content)?;
        drop(inner_content);

        const WRAPPER_CONTENT: &str = r#"package main

import (
    "encoding/json"
    "os"
    "fmt"
    "mymod/inner"
)

func main() {{

    dat, err := os.ReadFile("args.json")
    if err != nil {{
        fmt.Println(err)
        os.Exit(1)
    }}

    var req inner.Req

    if err := json.Unmarshal(dat, &req); err != nil {{
        fmt.Println(err)
        os.Exit(1)
    }}

    res, err := inner.Run(req)
    if err != nil {{
        fmt.Println(err)
        os.Exit(1)
    }}
    res_json, err := json.Marshal(res)
    if err != nil {{
        fmt.Println(err)
        os.Exit(1)
    }}
    f, err := os.OpenFile("result.json", os.O_APPEND|os.O_WRONLY, os.ModeAppend)
    if err != nil {{
        fmt.Println(err)
        os.Exit(1)
    }}
    _, err = f.WriteString(string(res_json))
    if err != nil {{
        fmt.Println(err)
        os.Exit(1)
    }}
}}"#;

        write_file(job_dir, "main.go", WRAPPER_CONTENT).await?;

        {
            let spread = &sig
                .args
                .clone()
                .into_iter()
                .map(|x| format!("req.{}", capitalize(&x.name)))
                .join(", ");
            let req_body = &sig
                .args
                .into_iter()
                .map(|x| {
                    format!(
                        "{} {} `json:\"{}\"`",
                        capitalize(&x.name),
                        windmill_parser_go::otyp_to_string(x.otyp),
                        x.name
                    )
                })
                .join("\n");
            let runner_content: String = format!(
                r#"package inner
type Req struct {{
    {req_body}
}}

func Run(req Req) (interface{{}}, error){{
    return main({spread})
}}

"#,
            );
            write_file(&format!("{job_dir}/inner"), "runner.go", &runner_content).await?;
        }
    }
    let mut reserved_variables = get_reserved_variables(job, &token, &base_url, db).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

    let child = if !disable_nsjail {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_GO_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", GO_CACHE_DIR)
                .replace("{CLONE_NEWUSER}", &(!disable_nuser).to_string())
                .replace("{SHARED_MOUNT}", shared_mount),
        )
        .await?;

        Command::new(nsjail_path)
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", path_env)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("GOMEMLIMIT", "2000MiB")
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                go_path,
                "run",
                "main.go",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        Command::new(go_path)
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", path_env)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("GOPATH", gopath_env)
            .env("HOME", home_env)
            .args(vec!["run", "main.go"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };
    handle_child(&job.id, db, logs, timeout, child).await?;
    read_result(job_dir).await
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_deno_job(
    WorkerConfig { base_internal_url, base_url, disable_nuser, disable_nsjail, .. }: &WorkerConfig,
    Envs { nsjail_path, deno_path, path_env, .. }: &Envs,
    logs: &mut String,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &windmill_api_client::Client,
    token: String,
    job_dir: &str,
    inner_content: &String,
    timeout: i32,
    shared_mount: &str,
    lockfile: Option<String>,
) -> error::Result<serde_json::Value> {
    logs.push_str("\n\n--- DENO CODE EXECUTION ---\n");
    set_logs(logs, job.id, db).await;
    let lockfile = lockfile.and_then(|e| if e.starts_with("{") { Some(e) } else { None });
    let _ = write_file(
        job_dir,
        "lock.json",
        &lockfile.clone().unwrap_or("".to_string()),
    )
    .await?;
    // TODO: Separately cache dependencies here using `deno cache --reload --lock=lock.json src/deps.ts` (https://deno.land/manual@v1.27.0/linking_to_external_code/integrity_checking)
    // Then require caching below using --cached-only. This makes it so we require zero network interaction when running the process below

    let _ = write_file(job_dir, "inner.ts", inner_content).await?;
    let sig = trace_span!("parse_deno_signature")
        .in_scope(|| windmill_parser_ts::parse_deno_signature(inner_content))?;
    create_args_and_out_file(client, job, job_dir).await?;
    let spread = sig.args.into_iter().map(|x| x.name).join(",");
    let wrapper_content: String = format!(
        r#"
import {{ main }} from "./inner.ts";

const args = await Deno.readTextFile("args.json")
    .then(JSON.parse)
    .then(({{ {spread} }}) => [ {spread} ])

async function run() {{
    let res: any = await main(...args);
    const res_json = JSON.stringify(res ?? null, (key, value) => typeof value === 'undefined' ? null : value);
    await Deno.writeTextFile("result.json", res_json);
    Deno.exit(0);
}}
run();
"#,
    );
    write_file(job_dir, "main.ts", &wrapper_content).await?;
    let mut reserved_variables = get_reserved_variables(job, &token, &base_url, db).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

    let hostname_base = base_url.split("://").last().unwrap_or("localhost");
    let hostname_internal = base_internal_url.split("://").last().unwrap_or("localhost");
    let deno_auth_tokens = format!("{token}@{hostname_base};{token}@{hostname_internal}");
    let child = async {
        Ok(if !disable_nsjail {
            let _ = write_file(
                job_dir,
                "run.config.proto",
                &NSJAIL_CONFIG_RUN_DENO_CONTENT
                    .replace("{JOB_DIR}", job_dir)
                    .replace("{CACHE_DIR}", DENO_CACHE_DIR)
                    .replace("{CLONE_NEWUSER}", &(!disable_nuser).to_string())
                    .replace("{SHARED_MOUNT}", shared_mount),
            )
            .await?;
            let mut args = Vec::new();
            args.push("--config");
            args.push("run.config.proto");
            args.push("--");
            args.push(deno_path);
            args.push("run");
            if lockfile.is_some() {
                args.push("--lock=/tmp/lock.json");
            }
            args.push("--unstable");
            args.push("--v8-flags=--max-heap-size=2048");
            args.push("-A");
            args.push("/tmp/main.ts");

            Command::new(nsjail_path)
                .current_dir(job_dir)
                .env_clear()
                .envs(reserved_variables)
                .env("PATH", path_env)
                .env("DENO_AUTH_TOKENS", deno_auth_tokens)
                .env("BASE_INTERNAL_URL", base_internal_url)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        } else {
            let mut args = Vec::new();
            args.push("run");
            if lockfile.is_some() {
                args.push("--lock=/tmp/lock.json");
            }
            args.push("--unstable");
            args.push("--v8-flags=--max-heap-size=2048");
            args.push("-A");
            args.push("/tmp/main.ts");
            Command::new(deno_path)
                .current_dir(job_dir)
                .env_clear()
                .envs(reserved_variables)
                .env("PATH", path_env)
                .env("DENO_AUTH_TOKENS", deno_auth_tokens)
                .env("BASE_INTERNAL_URL", base_internal_url)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        }) as error::Result<_>
    }
    .instrument(trace_span!("create_deno_jail"))
    .await?;
    handle_child(&job.id, db, logs, timeout, child).await?;
    read_result(job_dir).await
}

#[tracing::instrument(level = "trace", skip_all)]
async fn create_args_and_out_file(
    client: &windmill_api_client::Client,
    job: &QueuedJob,
    job_dir: &str,
) -> Result<(), Error> {
    let args = if let Some(args) = &job.args {
        Some(transform_json_value(client, &job.workspace_id, args.clone()).await?)
    } else {
        None
    };
    let ser_args = serde_json::to_string(&args).map_err(|e| Error::ExecutionErr(e.to_string()))?;
    write_file(job_dir, "args.json", &ser_args).await?;
    write_file(job_dir, "result.json", "").await?;
    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_python_job(
    WorkerConfig { base_internal_url, base_url, disable_nuser, disable_nsjail, .. }: &WorkerConfig,
    envs @ Envs {
        nsjail_path,
        python_path,
        python_heavy_deps,
        path_env,
        pip_extra_index_url,
        pip_index_url,
        pip_trusted_host,
        ..
    }: &Envs,
    requirements_o: Option<String>,
    job_dir: &str,
    worker_dir: &str,
    worker_name: &str,
    job: &QueuedJob,
    logs: &mut String,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &windmill_api_client::Client,
    token: String,
    timeout: i32,
    inner_content: &String,
    shared_mount: &str,
) -> error::Result<serde_json::Value> {
    create_dependencies_dir(job_dir).await;

    let mut additional_python_paths: Vec<String> = vec![];

    let requirements = match requirements_o {
        Some(r) => r,
        None => {
            let requirements = windmill_parser_py::parse_python_imports(&inner_content)?.join("\n");
            if requirements.is_empty() {
                "".to_string()
            } else {
                pip_compile(job, &requirements, logs, job_dir, envs, db, timeout)
                    .await
                    .map_err(|e| {
                        Error::ExecutionErr(format!("pip compile failed: {}", e.to_string()))
                    })?
            }
        }
    };

    if requirements.len() > 0 {
        if !disable_nsjail {
            let _ = write_file(
                job_dir,
                "download.config.proto",
                &NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT
                    .replace("{JOB_DIR}", job_dir)
                    .replace("{WORKER_DIR}", &worker_dir)
                    .replace("{CACHE_DIR}", PIP_CACHE_DIR)
                    .replace("{CLONE_NEWUSER}", &(!disable_nuser).to_string()),
            )
            .await?;
        }

        let mut heavy_deps = DEFAULT_HEAVY_DEPS
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        heavy_deps.extend(python_heavy_deps.into_iter().map(|s| s.to_string()));

        let (heavy, regular): (Vec<&str>, Vec<&str>) = requirements
            .split("\n")
            .partition(|d| heavy_deps.iter().any(|hd| d.starts_with(hd)));

        let _ = write_file(job_dir, "requirements.txt", &regular.join("\n")).await?;

        let mut vars = vec![];
        if let Some(url) = pip_extra_index_url {
            vars.push(("EXTRA_INDEX_URL", url));
        }
        if let Some(url) = pip_index_url {
            vars.push(("INDEX_URL", url));
        }
        if let Some(host) = pip_trusted_host {
            vars.push(("TRUSTED_HOST", host));
        }

        if heavy.len() > 0 {
            logs.push_str(&format!(
                "\nheavy deps detected, using supercache for: {heavy:?}"
            ));
            additional_python_paths =
                handle_python_heavy_reqs(python_path, heavy, vars.clone(), job, logs, db, timeout)
                    .await?;
        }

        if regular.len() > 0 {
            tracing::info!(
                worker_name = %worker_name,
                job_id = %job.id,
                workspace_id = %job.workspace_id,
                "started setup python dependencies"
            );

            let child = if !disable_nsjail {
                Command::new(nsjail_path)
                    .current_dir(job_dir)
                    .env_clear()
                    .envs(vars)
                    .args(vec!["--config", "download.config.proto"])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?
            } else {
                let mut args = vec![
                    "-m",
                    "pip",
                    "install",
                    "--no-deps",
                    "--no-color",
                    "--isolated",
                    "--no-warn-conflicts",
                    "--disable-pip-version-check",
                    "-t",
                    "./dependencies",
                    "-r",
                    "./requirements.txt",
                ];
                if let Some(url) = pip_extra_index_url {
                    args.extend(["--extra-index-url", url]);
                }
                if let Some(url) = pip_index_url {
                    args.extend(["--index-url", url]);
                }
                if let Some(host) = pip_trusted_host {
                    args.extend(["--trusted-host", host]);
                }
                Command::new(python_path)
                    .current_dir(job_dir)
                    .env_clear()
                    .args(args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?
            };

            logs.push_str("\n--- PIP DEPENDENCIES INSTALL ---\n");
            let child = handle_child(&job.id, db, logs, timeout, child).await;
            tracing::info!(
                worker_name = %worker_name,
                job_id = %job.id,
                workspace_id = %job.workspace_id,
                is_ok = child.is_ok(),
                "finished setting up python dependencies {}",
                job.id
            );
            child?;
        } else {
            logs.push_str("\nskipping pip install since not needed");
        };
    }
    logs.push_str("\n\n--- PYTHON CODE EXECUTION ---\n");

    set_logs(logs, job.id, db).await;

    let _ = write_file(job_dir, "inner.py", inner_content).await?;

    let sig = windmill_parser_py::parse_python_signature(inner_content)?;
    let transforms = sig
        .args
        .into_iter()
        .map(|x| match x.typ {
            windmill_parser::Typ::Bytes => {
                format!(
                    "if \"{}\" in kwargs and kwargs[\"{}\"] is not None:\n    \
                                     kwargs[\"{}\"] = base64.b64decode(kwargs[\"{}\"])\n",
                    x.name, x.name, x.name, x.name
                )
            }
            windmill_parser::Typ::Datetime => {
                format!(
                    "if \"{}\" in kwargs and kwargs[\"{}\"] is not None:\n    \
                                     kwargs[\"{}\"] = datetime.strptime(kwargs[\"{}\"], \
                                     '%Y-%m-%dT%H:%M')\n",
                    x.name, x.name, x.name, x.name
                )
            }
            _ => "".to_string(),
        })
        .collect::<Vec<String>>()
        .join("");
    create_args_and_out_file(client, job, job_dir).await?;

    let wrapper_content: String = format!(
        r#"
import json
import base64
from datetime import datetime

inner_script = __import__("inner")

with open("args.json") as f:
    kwargs = json.load(f, strict=False)
for k, v in list(kwargs.items()):
    if v == '<function call>':
        del kwargs[k]
{transforms}
res = inner_script.main(**kwargs)
res_json = json.dumps(res, separators=(',', ':'), default=str).replace('\n', '')
with open("result.json", 'w') as f:
    f.write(res_json)
"#,
    );
    write_file(job_dir, "main.py", &wrapper_content).await?;

    let mut reserved_variables = get_reserved_variables(job, &token, &base_url, db).await?;
    let additional_python_paths_folders = additional_python_paths
        .iter()
        .map(|x| format!(":{x}"))
        .join("");
    if !disable_nsjail {
        let shared_deps = additional_python_paths
            .into_iter()
            .map(|pp| {
                format!(
                    r#"
mount {{
    src: "{pp}"
    dst: "{pp}"
    is_bind: true
    rw: false
}}
        "#
                )
            })
            .join("\n");
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_PYTHON3_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!disable_nuser).to_string())
                .replace("{SHARED_MOUNT}", shared_mount)
                .replace("{SHARED_DEPENDENCIES}", shared_deps.as_str())
                .replace(
                    "{ADDITIONAL_PYTHON_PATHS}",
                    additional_python_paths_folders.as_str(),
                ),
        )
        .await?;
    } else {
        reserved_variables.insert(
            "PYTHONPATH".to_string(),
            format!("{job_dir}/dependencies{additional_python_paths_folders}"),
        );
    }

    tracing::info!(
        worker_name = %worker_name,
        job_id = %job.id,
        workspace_id = %job.workspace_id,
        "started python code execution {}",
        job.id
    );
    let child = if !disable_nsjail {
        Command::new(nsjail_path)
            .current_dir(job_dir)
            .env_clear()
            // inject PYTHONPATH here - for some reason I had to do it in nsjail conf
            .envs(reserved_variables)
            .env("PATH", path_env)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                python_path,
                "-u",
                "/tmp/main.py",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        Command::new(python_path)
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", path_env)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec!["-u", "main.py"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };

    handle_child(&job.id, db, logs, timeout, child).await?;
    read_result(job_dir).await
}

async fn create_dependencies_dir(job_dir: &str) {
    DirBuilder::new()
        .recursive(true)
        .create(&format!("{job_dir}/dependencies"))
        .await
        .expect("could not create dependencies dir");
}

async fn read_result(job_dir: &str) -> error::Result<serde_json::Value> {
    let mut file = File::open(format!("{job_dir}/result.json")).await?;
    let mut content = "".to_string();
    file.read_to_string(&mut content).await?;
    serde_json::from_str(&content)
        .map_err(|e| Error::ExecutionErr(format!("Error parsing result: {e}")))
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_dependency_job(
    job: &QueuedJob,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    timeout: i32,
    envs: &Envs,
) -> error::Result<serde_json::Value> {
    let content = capture_dependency_job(job, logs, job_dir, db, timeout, envs).await;
    match content {
        Ok(content) => {
            sqlx::query!(
                "UPDATE script SET lock = $1 WHERE hash = $2 AND workspace_id = $3",
                &content,
                &job.script_hash.unwrap_or(ScriptHash(0)).0,
                &job.workspace_id
            )
            .execute(db)
            .await?;
            Ok(json!({ "success": "Successful lock file generation", "lock": content }))
        }
        Err(error) => {
            sqlx::query!(
                "UPDATE script SET lock_error_logs = $1 WHERE hash = $2 AND workspace_id = $3",
                &format!("{logs}\n{error}"),
                &job.script_hash.unwrap_or(ScriptHash(0)).0,
                &job.workspace_id
            )
            .execute(db)
            .await?;
            Err(Error::ExecutionErr(format!("Error locking file: {error}")))?
        }
    }
}

async fn generate_deno_lock(
    job_id: &Uuid,
    code: &str,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    timeout: i32,
    Envs { deno_path, .. }: &Envs,
) -> error::Result<String> {
    let _ = write_file(job_dir, "main.ts", code).await?;

    let child = Command::new(deno_path)
        .current_dir(job_dir)
        .args(vec![
            "cache",
            "--unstable",
            "--lock=lock.json",
            "--lock-write",
            "main.ts",
        ])
        .env("NO_COLOR", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    handle_child(job_id, db, logs, timeout, child).await?;

    let path_lock = format!("{job_dir}/lock.json");
    let mut file = File::open(path_lock).await?;
    let mut req_content = "".to_string();
    file.read_to_string(&mut req_content).await?;
    Ok(req_content)
}
async fn capture_dependency_job(
    job: &QueuedJob,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    timeout: i32,
    envs: &Envs,
) -> error::Result<String> {
    match job.language {
        Some(ScriptLang::Python3) => {
            create_dependencies_dir(job_dir).await;
            let requirements = &job
                .raw_code
                .as_ref()
                .ok_or_else(|| Error::ExecutionErr("missing requirements".to_string()))?
                .clone();
            pip_compile(job, requirements, logs, job_dir, envs, db, timeout).await
        }
        Some(ScriptLang::Go) => {
            let requirements = job
                .raw_code
                .as_ref()
                .ok_or_else(|| Error::ExecutionErr("missing requirements".to_string()))?;
            install_go_dependencies(
                &job.id,
                &requirements,
                logs,
                job_dir,
                db,
                timeout,
                &envs.go_path,
                false,
            )
            .await
        }
        Some(ScriptLang::Deno) => {
            let requirements = job
                .raw_code
                .as_ref()
                .ok_or_else(|| Error::ExecutionErr("missing requirements".to_string()))?;
            generate_deno_lock(&job.id, &requirements, logs, job_dir, db, timeout, &envs).await
        }
        _ => Err(error::Error::InternalErr(
            "Language incompatible with dep job".to_string(),
        )),
    }
}

async fn pip_compile(
    job: &QueuedJob,
    requirements: &str,
    logs: &mut String,
    job_dir: &str,
    Envs { pip_extra_index_url, pip_index_url, pip_trusted_host, .. }: &Envs,
    db: &Pool<Postgres>,
    timeout: i32,
) -> error::Result<String> {
    logs.push_str(&format!("content of requirements:\n{}\n", requirements));
    let file = "requirements.in";
    write_file(job_dir, file, &requirements).await?;
    let mut args = vec!["-q", "--no-header", file];
    if let Some(url) = pip_extra_index_url {
        args.extend(["--extra-index-url", url]);
    }
    if let Some(url) = pip_index_url {
        args.extend(["--index-url", url]);
    }
    if let Some(host) = pip_trusted_host {
        args.extend(["--trusted-host", host]);
    }
    let child = Command::new("pip-compile")
        .current_dir(job_dir)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    handle_child(&job.id, db, logs, timeout, child)
        .await
        .map_err(|e| Error::ExecutionErr(format!("Lock file generation failed: {e:?}")))?;
    let path_lock = format!("{job_dir}/requirements.txt");
    let mut file = File::open(path_lock).await?;
    let mut req_content = "".to_string();
    file.read_to_string(&mut req_content).await?;
    Ok(req_content
        .lines()
        .filter(|x| !x.trim_start().starts_with('#'))
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join("\n"))
}

async fn install_go_dependencies(
    job_id: &Uuid,
    code: &str,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    timeout: i32,
    go_path: &str,
    preview: bool,
) -> error::Result<String> {
    gen_go_mymod(code, job_dir).await?;
    let child = Command::new("go")
        .current_dir(job_dir)
        .args(vec!["mod", "init", "mymod"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    handle_child(job_id, db, logs, timeout, child).await?;

    let child = Command::new(go_path)
        .current_dir(job_dir)
        .env("GOMEMLIMIT", "2000MiB")
        .args(vec!["mod", "tidy"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    handle_child(job_id, db, logs, timeout, child)
        .await
        .map_err(|e| Error::ExecutionErr(format!("Lock file generation failed: {e:?}")))?;

    if preview {
        Ok(String::new())
    } else {
        let mut req_content = "".to_string();

        let mut file = File::open(format!("{job_dir}/go.mod")).await?;
        file.read_to_string(&mut req_content).await?;

        req_content.push_str(&format!("\n{GO_REQ_SPLITTER}\n"));

        if let Ok(mut file) = File::open(format!("{job_dir}/go.sum")).await {
            file.read_to_string(&mut req_content).await?;
        }

        Ok(req_content)
    }
}

async fn gen_go_mymod(code: &str, job_dir: &str) -> error::Result<()> {
    let code = if code.trim_start().starts_with("package") {
        code.to_string()
    } else {
        format!("package inner; {code}")
    };

    let mymod_dir = format!("{job_dir}/inner");
    DirBuilder::new()
        .recursive(true)
        .create(&mymod_dir)
        .await
        .expect("could not create go's mymod dir");

    write_file(&mymod_dir, "inner_main.go", &code).await?;

    Ok(())
}

// TODO: this really shouldn't be here
pub async fn get_email_from_username(
    username: &String,
    db: &Pool<Postgres>,
) -> error::Result<Option<String>> {
    let email = sqlx::query_scalar!("SELECT email FROM usr WHERE username = $1", username)
        .fetch_optional(db)
        .await?;
    Ok(email)
}

#[tracing::instrument(level = "trace", skip_all)]
async fn get_reserved_variables(
    job: &QueuedJob,
    token: &str,
    base_url: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> Result<HashMap<String, String>, Error> {
    let flow_path = if let Some(uuid) = job.parent_job {
        sqlx::query_scalar!("SELECT script_path FROM queue WHERE id = $1", uuid)
            .fetch_optional(db)
            .await?
            .flatten()
    } else {
        None
    };

    let variables = variables::get_reserved_variables(
        &job.workspace_id,
        token,
        &get_email_from_username(&job.created_by, db)
            .await?
            .unwrap_or_else(|| "nosuitable@email.xyz".to_string()),
        &job.created_by,
        &job.id.to_string(),
        &job.permissioned_as,
        base_url,
        job.script_path.clone(),
        job.parent_job.map(|x| x.to_string()),
        flow_path,
        job.schedule_path.clone(),
    );
    Ok(variables
        .into_iter()
        .map(|rv| (rv.name, rv.value))
        .collect())
}

/// - wait until child exits and return with exit status
/// - read lines from stdout and stderr and append them to the "queue"."logs"
///   quitting early if output exceedes MAX_LOG_SIZE characters (not bytes)
/// - update the `last_line` and `logs` strings with the program output
/// - update "queue"."last_ping" every five seconds
/// - kill process if we exceed timeout or "queue"."canceled" is set
#[tracing::instrument(level = "trace", skip_all)]
async fn handle_child(
    job_id: &Uuid,
    db: &Pool<Postgres>,
    logs: &mut String,
    timeout: i32,
    mut child: Child,
) -> error::Result<()> {
    let timeout = Duration::from_secs(u64::try_from(timeout).expect("invalid timeout"));
    let ping_interval = Duration::from_secs(5);
    let cancel_check_interval = Duration::from_millis(500);
    let write_logs_delay = Duration::from_millis(500);

    let (set_too_many_logs, mut too_many_logs) = watch::channel::<bool>(false);

    let output = child_joined_output_stream(&mut child);
    let job_id = job_id.clone();

    let (tx, mut rx) = mpsc::channel::<()>(1);

    /* the cancellation future is polled on by `wait_on_child` while
     * waiting for the child to exit normally */
    let cancel_check = async {
        let db = db.clone();

        let mut interval = interval(cancel_check_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select!(
                _ = rx.recv() => break,
                _ = interval.tick() => {
                    if sqlx::query_scalar!("SELECT canceled FROM queue WHERE id = $1", job_id)
                        .fetch_optional(&db)
                        .await
                        .map(|v| Some(true) == v)
                        .unwrap_or_else(|err| {
                            tracing::error!(%job_id, %err, "error checking cancelation for job {job_id}: {err}");
                            false
                        })
                    {
                        break;
                    }
                },
            );
        }
    };

    #[derive(PartialEq, Debug)]
    enum KillReason {
        TooManyLogs,
        Timeout,
        Cancelled,
    }
    /* a future that completes when the child process exits */
    let wait_on_child = async {
        let db = db.clone();

        let kill_reason = tokio::select! {
            biased;
            result = child.wait() => return result.map(Ok),
            Ok(()) = too_many_logs.changed() => KillReason::TooManyLogs,
            _ = cancel_check => KillReason::Cancelled,
            _ = sleep(timeout) => KillReason::Timeout,
        };
        tx.send(()).await.expect("rx should never be dropped");
        drop(tx);

        let set_reason = async {
            if kill_reason == KillReason::Timeout {
                if let Err(err) = sqlx::query(
                    r#"
                       UPDATE queue
                          SET canceled = true
                            , canceled_by = 'timeout',
                            , canceled_reason = $1
                        WHERE id = $2
                    r"#,
                )
                .bind(format!("duration > {}", timeout.as_secs()))
                .bind(job_id)
                .execute(&db)
                .await
                {
                    tracing::error!(%job_id, %err, "error setting cancelation reason for job {job_id}: {err}");
                }
            }
        };

        /* send SIGKILL and reap child process */
        let (_, kill) = future::join(set_reason, child.kill()).await;
        kill.map(|()| Err(kill_reason))
    };

    /* a future that reads output from the child and appends to the database */
    let lines = async move {
        /* log_remaining is zero when output limit was reached */
        let mut log_remaining = (MAX_LOG_SIZE as usize).saturating_sub(logs.chars().count());
        let mut result = io::Result::Ok(());
        let mut output = output;
        /* `do_write` resolves the task, but does not contain the Result.
         * It's useful to know if the task completed. */
        let (mut do_write, mut write_result) = tokio::spawn(ready(())).remote_handle();

        while let Some(line) = output.by_ref().next().await {
            let do_write_ = do_write.shared();

            let mut read_lines = stream::once(async { line })
                .chain(output.by_ref())
                /* after receiving a line, continue until some delay has passed
                 * _and_ the previous database write is complete */
                .take_until(future::join(sleep(write_logs_delay), do_write_.clone()))
                .boxed();

            /* Read up until an error is encountered,
             * handle log lines first and then the error... */
            let mut joined = String::new();

            while let Some(line) = read_lines.next().await {
                match line {
                    Ok(_) if log_remaining == 0 => (),
                    Ok(line) => {
                        append_with_limit(&mut joined, &line, &mut log_remaining);

                        if log_remaining == 0 {
                            tracing::info!(%job_id, "Too many logs lines for job {job_id}");
                            let _ = set_too_many_logs.send(true);
                            joined.push_str(&format!(
                                "Job logs or result reached character limit of {MAX_LOG_SIZE}; killing job."
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

            logs.push_str(&joined);

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

            (do_write, write_result) =
                tokio::spawn(append_logs(job_id, joined, db.clone())).remote_handle();

            if let Err(err) = result {
                tracing::error!(%job_id, %err, "error reading output for job {job_id}: {err}");
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

    /* a stream updating "queue"."last_ping" at an interval */

    let (kill_tx, mut kill_rx) = oneshot::channel::<()>();

    let mut interval = interval(ping_interval);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let db1 = db.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(err) = sqlx::query!("UPDATE queue SET last_ping = now() WHERE id = $1", job_id)
                        .execute(&db1)
                    .await
            {
                tracing::error!(%job_id, %err, "error setting last ping for job {job_id}: {err}");
                    };
                },
                _ = (&mut kill_rx) => return,
            }
        }
    });
    let (wait_result, _) = tokio::join!(wait_on_child, lines);
    kill_tx.send(()).expect("send should always work");

    match wait_result {
        _ if *too_many_logs.borrow() => Err(Error::ExecutionErr(
            "logs or result reached limit".to_string(),
        )),
        Ok(Ok(status)) => {
            if status.success() {
                Ok(())
            } else if let Some(code) = status.code() {
                Err(error::Error::ExitStatus(code))
            } else {
                Err(error::Error::ExecutionErr(
                    "process terminated by signal".to_string(),
                ))
            }
        }
        Ok(Err(kill_reason)) => Err(Error::ExecutionErr(format!(
            "job process killed because {kill_reason:#?}"
        ))),
        Err(err) => Err(Error::ExecutionErr(format!("job process io error: {err}"))),
    }
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

fn lines_to_stream<R: tokio::io::AsyncBufRead + Unpin>(
    mut lines: tokio::io::Lines<R>,
) -> impl futures::Stream<Item = io::Result<String>> {
    stream::poll_fn(move |cx| {
        std::pin::Pin::new(&mut lines)
            .poll_next_line(cx)
            .map(|result| result.transpose())
    })
}

// as a detail, `BufReader::lines()` removes \n and \r\n from the strings it yields,
// so this pushes \n to thd destination string in each call
fn append_with_limit(dst: &mut String, src: &str, limit: &mut usize) {
    if *limit > 0 {
        dst.push('\n');
    }
    *limit -= 1;

    let src_len = src.chars().count();
    if src_len <= *limit {
        dst.push_str(&src);
        *limit -= src_len;
    } else {
        let byte_pos = src
            .char_indices()
            .skip(*limit)
            .next()
            .map(|(byte_pos, _)| byte_pos)
            .unwrap_or(0);
        dst.push_str(&src[0..byte_pos]);
        *limit = 0;
    }
}

#[tracing::instrument(level = "trace", skip_all)]
async fn set_logs(logs: &str, id: uuid::Uuid, db: &Pool<Postgres>) {
    if sqlx::query!(
        "UPDATE queue SET logs = $1 WHERE id = $2",
        logs.to_owned(),
        id
    )
    .execute(db)
    .await
    .is_err()
    {
        tracing::error!(%id, "error updating logs for id {id}")
    };
}

/* TODO retry this? */
#[tracing::instrument(level = "trace", skip_all)]
async fn append_logs(job_id: uuid::Uuid, logs: impl AsRef<str>, db: impl Borrow<Pool<Postgres>>) {
    if logs.as_ref().is_empty() {
        return;
    }

    if let Err(err) = sqlx::query!(
        "UPDATE queue SET logs = concat(logs, $1::text) WHERE id = $2",
        logs.as_ref(),
        job_id,
    )
    .execute(db.borrow())
    .await
    {
        tracing::error!(%job_id, %err, "error updating logs for job {job_id}: {err}");
    }
}

pub async fn handle_zombie_jobs_periodically(
    db: &Pool<Postgres>,
    timeout: i32,
    base_url: &str,
    mut rx: tokio::sync::broadcast::Receiver<()>,
) {
    loop {
        handle_zombie_jobs(db, timeout, base_url).await;

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(60))    => (),
            _ = rx.recv() => {
                    println!("received killpill for monitor job");
                    break;
            }
        }
    }
}

async fn handle_zombie_jobs(db: &Pool<Postgres>, timeout: i32, base_url: &str) {
    let restarted = sqlx::query!(
            "UPDATE queue SET running = false WHERE last_ping < now() - ($1 || ' seconds')::interval AND running = true AND job_kind != $2 AND same_worker = false RETURNING id, workspace_id, last_ping",
            (timeout * 5).to_string(),
            JobKind::Flow: JobKind,
        )
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![]);

    QUEUE_ZOMBIE_RESTART_COUNT.inc_by(restarted.len() as _);
    for r in restarted {
        tracing::info!(
            "restarted zombie job {} {} {}",
            r.id,
            r.workspace_id,
            r.last_ping
        );
    }

    let timeouts = sqlx::query_as::<_, QueuedJob>(
            "SELECT * FROM queue WHERE last_ping < now() - ($1 || ' seconds')::interval AND running = true AND job_kind != $2 AND same_worker = true",
        )
        .bind((timeout * 5).to_string())
        .bind(JobKind::Flow)
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![]);

    QUEUE_ZOMBIE_DELETE_COUNT.inc_by(timeouts.len() as _);
    for job in timeouts {
        tracing::info!(
            "timedouts zombie same_worker job {} {}",
            job.id,
            job.workspace_id,
        );

        // since the job is unrecoverable, the same worker queue should never be sent anything
        let (same_worker_tx_never_used, _same_worker_rx_never_used) = mpsc::channel::<Uuid>(1);

        let tx = db.begin().await.expect("could not start token transaction");
        let (tx, token) = create_token_for_owner(
            tx,
            &job.workspace_id,
            &job.permissioned_as,
            "ephemeral-zombie-jobs",
            timeout * 2,
            &job.created_by,
        )
        .await
        .expect("could not create job token");
        tx.commit().await.expect("could not commit job token");
        let client = windmill_api_client::create_client(base_url, token.clone());

        let _ = handle_job_error(
            db,
            &client,
            job,
            error::Error::ExecutionErr("Same worker job timed out".to_string()),
            None,
            true,
            same_worker_tx_never_used,
            "",
            true,
            &std::env::var("BASE_INTERNAL_URL")
                .unwrap_or_else(|_| "http://localhost:8000".to_string()),
        )
        .await;
    }
}

async fn handle_python_heavy_reqs(
    python_path: &String,
    heavy_requirements: Vec<&str>,
    vars: Vec<(&str, &String)>,
    job: &QueuedJob,
    logs: &mut String,
    db: &sqlx::Pool<sqlx::Postgres>,
    timeout: i32,
) -> error::Result<Vec<String>> {
    let mut req_paths: Vec<String> = vec![];
    for req in heavy_requirements {
        // todo: handle many reqs
        let venv_p = format!("{PIP_SUPERCACHE_DIR}/{req}");
        if metadata(&venv_p).await.is_ok() {
            tracing::info!("already exists: {:?}", &venv_p);
            req_paths.push(venv_p);
            continue;
        }

        logs.push_str("\n--- PIP SUPERCACHE INSTALL ---\n");
        logs.push_str(&format!("\nthe heavy dependency {req} is being installed for the first time.\nIt will take a bit longer but further execution will be much faster!"));

        logs.push_str("pip install\n");
        let child = Command::new(python_path)
            .env_clear()
            .envs(vars.clone())
            .args(vec![
                "-m",
                "pip",
                "install",
                &req,
                "-I",
                "--no-deps",
                "--no-color",
                "--isolated",
                "--no-warn-conflicts",
                "--disable-pip-version-check",
                "-t",
                venv_p.as_str(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        handle_child(&job.id, db, logs, timeout, child).await?;

        req_paths.push(venv_p);
    }
    Ok(req_paths)
}
