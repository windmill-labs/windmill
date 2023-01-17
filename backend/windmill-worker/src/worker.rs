/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use const_format::concatcp;
use git_version::git_version;
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use sqlx::{Pool, Postgres, Transaction};
use std::{borrow::Borrow, collections::HashMap, io, panic, process::Stdio, time::Duration};
use tracing::{trace_span, Instrument};
use uuid::Uuid;

use windmill_common::{
    error::{self, to_anyhow, Error},
    flows::{FlowModuleValue, FlowValue},
    scripts::{ScriptHash, ScriptLang},
    utils::rd_string,
    variables,
};
use windmill_queue::{canceled_job_to_result, get_queued_job, pull, JobKind, QueuedJob};

use serde_json::{json, Value};

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
    stream, StreamExt,
};

use async_recursion::async_recursion;

use crate::{
    jobs::{add_completed_job, add_completed_job_error, error_to_result},
    worker_flow::{
        handle_flow, update_flow_status_after_job_completion, update_flow_status_in_progress,
    },
};

#[cfg(feature = "enterprise")]
use rand::Rng;

#[cfg(feature = "enterprise")]
const TAR_CACHE_FILENAME: &str = "entirecache.tar";

#[cfg(feature = "enterprise")]
async fn copy_cache_from_bucket(bucket: &str) {
    tracing::info!("Copying cache from bucket {bucket}");
    let elapsed = Instant::now();

    match Command::new("rclone")
        .arg("copy")
        .arg(format!(":s3,env_auth=true:{bucket}"))
        .arg(ROOT_CACHE_DIR)
        .arg("--size-only")
        .arg("--fast-list")
        .arg("--exclude")
        .arg(format!("\"{TAR_CACHE_FILENAME}\""))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            h.wait().await.unwrap();
        }
        Err(e) => tracing::warn!("Failed to run periodic job pull. Error: {:?}", e),
    }
    tracing::info!(
        "Finished copying cache from bucket {bucket}, took {:?}s",
        elapsed.elapsed().as_secs()
    );

    for x in [PIP_CACHE_DIR, DENO_CACHE_DIR, GO_CACHE_DIR] {
        DirBuilder::new()
            .recursive(true)
            .create(x)
            .await
            .expect("could not create initial worker dir");
    }
}

#[cfg(feature = "enterprise")]
async fn copy_cache_to_bucket(bucket: &str) {
    tracing::info!("Copying cache to bucket {bucket}");
    let elapsed = Instant::now();
    match Command::new("rclone")
        .arg("copy")
        .arg(ROOT_CACHE_DIR)
        .arg(format!(":s3,env_auth=true:{bucket}"))
        .arg("--size-only")
        .arg("--fast-list")
        .arg("--exclude")
        .arg(format!("\"{TAR_CACHE_FILENAME}\""))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            h.wait().await.unwrap();
        }
        Err(e) => tracing::warn!("Failed to run periodic job push. Error: {:?}", e),
    }
    tracing::info!(
        "Finished copying cache to bucket {bucket}, took: {:?}s",
        elapsed.elapsed().as_secs()
    );
}

#[cfg(feature = "enterprise")]
async fn copy_cache_to_bucket_as_tar(bucket: &str) {
    tracing::info!("Copying cache to bucket {bucket} as tar");
    let elapsed = Instant::now();

    match Command::new("tar")
        .current_dir(ROOT_CACHE_DIR)
        .arg("-c")
        .arg("-f")
        .arg(format!("{ROOT_CACHE_DIR}/{TAR_CACHE_FILENAME}"))
        .args(&["pip", "go", "deno"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            if !h.wait().await.unwrap().success() {
                tracing::warn!("Failed to tar cache");
                return;
            }
        }
        Err(e) => {
            tracing::warn!("Failed tar cache. Error: {e:?}");
            return;
        }
    }

    match Command::new("rclone")
        .current_dir(ROOT_CACHE_DIR)
        .arg("copyto")
        .arg(TAR_CACHE_FILENAME)
        .arg(format!(":s3,env_auth=true:{bucket}/{TAR_CACHE_FILENAME}"))
        .arg("--size-only")
        .arg("--fast-list")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            h.wait().await.unwrap();
        }
        Err(e) => tracing::warn!("Failed to copying tar cache to bucket. Error: {:?}", e),
    }
    tracing::info!(
        "Finished copying cache to bucket {bucket} as tar, took: {:?}s",
        elapsed.elapsed().as_secs()
    );
}

#[cfg(feature = "enterprise")]
async fn copy_cache_from_bucket_as_tar(bucket: &str) -> bool {
    tracing::info!("Copying cache from bucket {bucket} as tar");
    let elapsed = Instant::now();

    match Command::new("rclone")
        .arg("copyto")
        .arg(format!(":s3,env_auth=true:{bucket}/{TAR_CACHE_FILENAME}"))
        .arg(format!("{ROOT_CACHE_DIR}/{TAR_CACHE_FILENAME}"))
        .arg("--size-only")
        .arg("--fast-list")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            if !h.wait().await.unwrap().success() {
                tracing::warn!("Failed to download tar cache");
                return false;
            }
        }
        Err(e) => {
            tracing::warn!("Failed to download tar cache. Error: {e:?}");
            return false;
        }
    }

    match Command::new("tar")
        .current_dir(ROOT_CACHE_DIR)
        .arg("-xpvf")
        .arg(format!("{ROOT_CACHE_DIR}/{TAR_CACHE_FILENAME}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            if !h.wait().await.unwrap().success() {
                tracing::warn!("Failed to untar cache");
                return false;
            }
        }
        Err(e) => {
            tracing::warn!("Failed untar cache. Error: {e:?}");
            return false;
        }
    }

    tracing::info!(
        "Finished copying cache from bucket {bucket} as tar, took: {:?}s",
        elapsed.elapsed().as_secs()
    );
    return true;
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn create_token_for_owner<'c>(
    mut tx: Transaction<'c, Postgres>,
    w_id: &str,
    owner: &str,
    label: &str,
    expires_in: i32,
) -> error::Result<(Transaction<'c, Postgres>, String)> {
    // TODO: Bad implementation. We should not have access to this DB here.
    let token: String = rd_string(30);
    let is_super_admin = owner.contains('@')
        && sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", owner)
            .fetch_optional(&mut tx)
            .await?
            .unwrap_or(false);

    sqlx::query_scalar!(
        "INSERT INTO token
            (workspace_id, token, owner, label, expiration, super_admin)
            VALUES ($1, $2, $3, $4, now() + ($5 || ' seconds')::interval, $6)",
        &w_id,
        token,
        owner,
        label,
        expires_in.to_string(),
        is_super_admin
    )
    .execute(&mut tx)
    .await?;
    Ok((tx, token))
}

const TMP_DIR: &str = "/tmp/windmill";
const ROOT_CACHE_DIR: &str = "/tmp/windmill/cache/";
const PIP_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "pip");
const DENO_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "deno");
const GO_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "go");
const NUM_SECS_ENV_CHECK: u64 = 15;
const NUM_SECS_SYNC: u64 = 60 * 10;

const INCLUDE_DEPS_PY_SH_CONTENT: &str = include_str!("../nsjail/download_deps.py.sh");
const NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT: &str = include_str!("../nsjail/download.py.config.proto");
const NSJAIL_CONFIG_RUN_PYTHON3_CONTENT: &str = include_str!("../nsjail/run.python3.config.proto");
const NSJAIL_CONFIG_RUN_GO_CONTENT: &str = include_str!("../nsjail/run.go.config.proto");
const NSJAIL_CONFIG_RUN_BASH_CONTENT: &str = include_str!("../nsjail/run.bash.config.proto");
const NSJAIL_CONFIG_RUN_DENO_CONTENT: &str = include_str!("../nsjail/run.deno.config.proto");

const RELATIVE_PYTHON_LOADER: &str = include_str!("../loader.py");

const GO_REQ_SPLITTER: &str = "//go.sum";
const GIT_VERSION: &str = git_version!(args = ["--tag", "--always"], fallback = "unknown-version");

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
    sync_bucket: Option<String>,
    mut rx: tokio::sync::broadcast::Receiver<()>,
) {
    tracing::info!("Starting worker {worker_instance} {worker_name}, version: {GIT_VERSION}");
    let start_time = Instant::now();

    let worker_dir = format!("{TMP_DIR}/{worker_name}");
    tracing::debug!(worker_dir = %worker_dir, worker_name = %worker_name, "Creating worker dir");

    for x in [&worker_dir, PIP_CACHE_DIR, DENO_CACHE_DIR, GO_CACHE_DIR] {
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

    let mut last_ping = Instant::now() - Duration::from_secs(NUM_SECS_SYNC + 1);

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

    let worker_sleep_duration = prometheus::register_histogram!(prometheus::HistogramOpts::new(
        "worker_sleep_duration",
        "Duration sleeping waiting for job",
    )
    .const_label("name", &worker_name),)
    .expect("register prometheus metric");

    let worker_pull_duration = prometheus::register_histogram!(prometheus::HistogramOpts::new(
        "worker_pull_duration",
        "Duration pulling next job",
    )
    .const_label("name", &worker_name),)
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
    let nsjail_path = std::env::var("NSJAIL_PATH").unwrap_or_else(|_| "nsjail".to_string());
    let path_env = std::env::var("PATH").unwrap_or_else(|_| String::new());
    let home_env = std::env::var("HOME").unwrap_or_else(|_| String::new());
    let pip_index_url = std::env::var("PIP_INDEX_URL").ok();
    let pip_extra_index_url = std::env::var("PIP_EXTRA_INDEX_URL").ok();
    let pip_trusted_host = std::env::var("PIP_TRUSTED_HOST").ok();
    let max_log_size = std::env::var("MAX_LOG_SIZE")
        .ok()
        .and_then(|x| x.parse::<i64>().ok())
        .unwrap_or(500000);

    #[cfg(feature = "enterprise")]
    let tar_cache_rate = std::env::var("TAR_CACHE_RATE")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(100);

    let envs = Envs {
        deno_path,
        go_path,
        python_path,
        nsjail_path,
        path_env,
        home_env,
        pip_index_url,
        pip_extra_index_url,
        pip_trusted_host,
        max_log_size,
    };
    WORKER_STARTED.inc();

    #[cfg(feature = "enterprise")]
    if let Some(ref s) = sync_bucket.clone() {
        // We try to download the entire cache as a tar, it is much faster over S3
        if !copy_cache_from_bucket_as_tar(&s).await {
            // We revert to copying the cache from the bucket
            copy_cache_from_bucket(&s).await;
        }
    }

    #[cfg(feature = "enterprise")]
    let mut last_sync =
        Instant::now() + Duration::from_secs(rand::thread_rng().gen_range(0..NUM_SECS_SYNC));

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

            #[cfg(feature = "enterprise")]
            if last_sync.elapsed().as_secs() > NUM_SECS_SYNC {
                if let Some(ref s) = sync_bucket.clone() {
                    copy_cache_from_bucket(&s).await;
                    copy_cache_to_bucket(&s).await;
                    if rand::thread_rng().gen_range(0..tar_cache_rate) == 1 {
                        copy_cache_to_bucket_as_tar(&s).await;
                    }
                }
                last_sync = Instant::now();
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
                    (job, timer) = {let timer = worker_pull_duration.start_timer(); pull(&db).map(|x| (x, timer)) } => {
                        drop(timer);
                        (false, job)
                    },
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

                    let target = &format!("{job_dir}/shared");

                    if same_worker && job.parent_job.is_some() {
                        let parent_flow = job.parent_job.unwrap();
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
                    let tx = db.begin().await.expect("could not start token transaction");
                    let (tx, token) = create_token_for_owner(
                        tx,
                        &job.workspace_id,
                        &job.permissioned_as,
                        "ephemeral-script",
                        timeout * 2,
                    )
                    .await.expect("could not create job token");
                    tx.commit().await.expect("could not commit job token");
                    let job_client = windmill_api_client::create_client(&worker_config.base_url, token.clone());
                    let is_flow = job.job_kind == JobKind::Flow || job.job_kind == JobKind::FlowPreview || job.job_kind == JobKind::FlowDependencies;

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

                    let _timer = worker_sleep_duration
                        .start_timer();

                    tokio::time::sleep(Duration::from_millis(sleep_queue * num_workers)).await;

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
    let _ = add_completed_job_error(
        db,
        &job,
        format!("Unexpected error during job execution:\n{err}"),
        json!({"message": err.to_string(), "name": "InternalErr"}),
        metrics.clone(),
    )
    .await;

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
                            &parent_job,
                            format!("Unexpected error during flow job error handling:\n{err}"),
                            json!({"message": err.to_string(), "name": "InternalErr"}),
                            metrics.clone(),
                        )
                        .await;
                    }
                }
            }
        }
    }
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
    nsjail_path: String,
    path_env: String,
    home_env: String,
    pip_index_url: Option<String>,
    pip_extra_index_url: Option<String>,
    pip_trusted_host: Option<String>,
    max_log_size: i64,
}

fn extract_error_value(log_lines: &str) -> serde_json::Value {
    return json!({"message": log_lines.to_string().trim().to_string(), "name": "ExecutionErr"});
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
        return Err(Error::ExecutionErr(
            canceled_job_to_result(&job).to_string(),
        ))?;
    }
    if let Some(e) = job.pre_run_error {
        return Err(Error::ExecutionErr(e));
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

            tracing::debug!(
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
                JobKind::FlowDependencies => {
                    handle_flow_dependency_job(&job, &mut logs, job_dir, db, timeout, &envs)
                        .await
                        .map(|()| Value::Null)
                }
                JobKind::Identity => match job.args.clone() {
                    Some(Value::Object(args))
                        if args.len() == 1 && args.contains_key("previous_result") =>
                    {
                        Ok(args.get("previous_result").unwrap().clone())
                    }
                    args @ _ => Ok(args.unwrap_or_else(|| Value::Null)),
                },
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
                    add_completed_job(db, &job, true, false, r.clone(), logs).await?;
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
                    let error_value = match e {
                        Error::ExitStatus(_) => {
                            let res = read_result(job_dir).await.ok();

                            if res.is_some() && res.clone().unwrap().is_object() {
                                res.unwrap()
                            } else {
                                let last_10_log_lines = logs
                                    .lines()
                                    .skip(logs.lines().count().max(13) - 13)
                                    .join("\n")
                                    .to_string()
                                    .replace("\n\n", "\n");

                                let log_lines = last_10_log_lines
                                    .split("CODE EXECUTION ---")
                                    .last()
                                    .unwrap_or(&logs);

                                extract_error_value(log_lines)
                            }
                        }
                        err @ _ => {
                            json!({"message": format!("error before termination: {err:#?}"), "name": "ExecutionErr"})
                        }
                    };

                    let result =
                        add_completed_job_error(db, &job, logs, error_value, Some(metrics.clone()))
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
                                result,
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
                .map_err(|_| Error::NotFound(format!("Variable {path} not found")))
                .map(|v| v.into_inner())?
                .value
                .unwrap_or_else(|| String::new());
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
                .map_err(|_| Error::NotFound(format!("Resource {path} not found")))?
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
    let (inner_content, requirements_o, language) = match job.job_kind {
        JobKind::Preview | JobKind::Script_Hub => (
            job.raw_code
                .clone()
                .unwrap_or_else(|| "no raw code".to_owned()),
            job.raw_lock.clone(),
            job.language.to_owned(),
        ),
        JobKind::Script => sqlx::query_as::<_, (String, Option<String>, Option<ScriptLang>)>(
            "SELECT content, lock, language FROM script WHERE hash = $1 AND (workspace_id = $2 OR \
                                    workspace_id = 'starter')",
        )
        .bind(&job.script_hash.unwrap_or(ScriptHash(0)).0)
        .bind(&job.workspace_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| Error::InternalErr(format!("expected content and lock")))?,
        _ => unreachable!(
            "handle_code_execution_job should never be reachable with a non-code execution job"
        ),
    };
    let worker_name = worker_dir.split("/").last().unwrap_or("unknown");
    let lang_str = job
        .language
        .as_ref()
        .map(|x| format!("{x:?}"))
        .unwrap_or_else(|| "NO_LANG".to_string());

    tracing::debug!(
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
    src: "{job_dir}/shared"
    dst: "/tmp/shared"
    is_bind: true
    rw: true
}}
        "#
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
        Some(ScriptLang::Bash) => {
            handle_bash_job(
                worker_config,
                envs,
                logs,
                job,
                db,
                token,
                &inner_content,
                timeout,
                job_dir,
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
    Envs { nsjail_path, go_path, path_env, home_env, max_log_size, .. }: &Envs,
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
    let skip_go_mod = if let Some(requirements) = requirements_o {
        gen_go_mymod(inner_content, job_dir).await?;

        // TODO: remove after some time in favor of just requirements
        // this is just migration code from a time we also stored go.sum
        let md = requirements
            .split_once(GO_REQ_SPLITTER)
            .map(|x| x.0)
            .unwrap_or(&requirements);
        write_file(job_dir, "go.mod", &md).await?;
        true
    } else {
        false
    };
    logs.push_str("\n\n--- GO DEPENDENCIES SETUP ---\n");
    set_logs(logs, &job.id, db).await;

    install_go_dependencies(
        &job.id,
        inner_content,
        logs,
        job_dir,
        db,
        timeout,
        *max_log_size,
        go_path,
        true,
        skip_go_mod,
    )
    .await?;

    logs.push_str("\n\n--- GO CODE EXECUTION ---\n");
    set_logs(logs, &job.id, db).await;
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
        let build_go = Command::new(go_path)
            .current_dir(job_dir)
            .env_clear()
            .env("PATH", path_env)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("GOPATH", GO_CACHE_DIR)
            .env("HOME", home_env)
            .args(vec!["build", "main.go"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        handle_child(&job.id, db, logs, timeout, *max_log_size, build_go).await?;

        Command::new(nsjail_path)
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", path_env)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec!["--config", "run.config.proto", "--", "/tmp/go/main"])
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
            .env("GOPATH", GO_CACHE_DIR)
            .env("HOME", home_env)
            .args(vec!["run", "main.go"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };
    handle_child(&job.id, db, logs, timeout, *max_log_size, child).await?;
    read_result(job_dir).await
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_bash_job(
    WorkerConfig { base_internal_url, disable_nuser, disable_nsjail, base_url, .. }: &WorkerConfig,
    Envs { nsjail_path, path_env, home_env, max_log_size, .. }: &Envs,
    logs: &mut String,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    token: String,
    content: &str,
    timeout: i32,
    job_dir: &str,
    shared_mount: &str,
) -> Result<serde_json::Value, Error> {
    logs.push_str("\n\n--- BASH CODE EXECUTION ---\n");
    set_logs(logs, &job.id, db).await;
    write_file(job_dir, "main.sh", content).await?;

    let mut reserved_variables = get_reserved_variables(job, &token, &base_url, db).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

    let hm = match job.args {
        Some(Value::Object(ref hm)) => hm.clone(),
        _ => serde_json::Map::new(),
    };
    let args_owned = windmill_parser_bash::parse_bash_sig(&content)?
        .args
        .iter()
        .map(|arg| {
            hm.get(&arg.name)
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => serde_json::to_string(v).ok(),
                })
                .unwrap_or_else(String::new)
        })
        .collect::<Vec<String>>();
    let args = args_owned.iter().map(|s| &s[..]).collect::<Vec<&str>>();

    let child = if !disable_nsjail {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_BASH_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!disable_nuser).to_string())
                .replace("{SHARED_MOUNT}", shared_mount),
        )
        .await?;
        let mut cmd_args = vec!["--config", "run.config.proto", "--", "/bin/sh", "main.sh"];
        cmd_args.extend(args);
        Command::new(nsjail_path)
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", path_env)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        let mut cmd_args = vec!["main.sh"];
        cmd_args.extend(&args);
        Command::new("/bin/sh")
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", path_env)
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", home_env)
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };
    handle_child(&job.id, db, logs, timeout, *max_log_size, child).await?;
    //for now bash jobs have an empty result object
    Ok(serde_json::json!(logs
        .lines()
        .last()
        .map(|x| x.to_string())
        .unwrap_or_else(String::new)))
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
    Envs { nsjail_path, deno_path, path_env, max_log_size, .. }: &Envs,
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
    set_logs(logs, &job.id, db).await;
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
run().catch(async (e) => {{
    await Deno.writeTextFile("result.json", JSON.stringify({{ message: e.message, name: e.name, stack: e.stack }}));
    Deno.exit(1);
}});
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
            let script_path = format!("{job_dir}/main.ts");
            args.push("run");
            if lockfile.is_some() {
                args.push("--lock=/tmp/lock.json");
            }
            args.push("--unstable");
            args.push("--v8-flags=--max-heap-size=2048");
            args.push("-A");
            args.push(&script_path);
            Command::new(deno_path)
                .current_dir(job_dir)
                .env_clear()
                .envs(reserved_variables)
                .env("PATH", path_env)
                .env("DENO_DIR", DENO_CACHE_DIR)
                .env("DENO_AUTH_TOKENS", deno_auth_tokens)
                .env("BASE_INTERNAL_URL", base_internal_url)
                .env("NO_COLOR", "true")
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        }) as error::Result<_>
    }
    .instrument(trace_span!("create_deno_jail"))
    .await?;
    handle_child(&job.id, db, logs, timeout, *max_log_size, child).await?;
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

lazy_static! {
    static ref RELATIVE_IMPORT_REGEX: Regex = Regex::new(r#"(import|from)\s(u|f)\."#).unwrap();
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_python_job(
    WorkerConfig { base_internal_url, base_url, disable_nuser, disable_nsjail, .. }: &WorkerConfig,
    envs @ Envs { nsjail_path, python_path, path_env, max_log_size, .. }: &Envs,
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
                pip_compile(&job.id, &requirements, logs, job_dir, envs, db, timeout)
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
                    .replace("{WORKER_DIR}", &worker_dir)
                    .replace("{CACHE_DIR}", PIP_CACHE_DIR)
                    .replace("{CLONE_NEWUSER}", &(!disable_nuser).to_string()),
            )
            .await?;
        }

        additional_python_paths = handle_python_reqs(
            requirements
                .split("\n")
                .filter(|x| !x.starts_with("--"))
                .collect(),
            envs,
            job,
            logs,
            db,
            timeout,
            disable_nsjail.clone(),
            worker_name,
            job_dir,
        )
        .await?;
    }
    logs.push_str("\n\n--- PYTHON CODE EXECUTION ---\n");

    set_logs(logs, &job.id, db).await;

    let relative_imports = RELATIVE_IMPORT_REGEX.is_match(&inner_content);

    let _ = write_file(job_dir, "inner.py", inner_content).await?;
    if relative_imports {
        let _ = write_file(job_dir, "loader.py", RELATIVE_PYTHON_LOADER).await?;
    }

    let sig = windmill_parser_py::parse_python_signature(inner_content)?;
    let transforms = sig
        .args
        .iter()
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

    let import_loader = if relative_imports {
        "import loader"
    } else {
        ""
    };
    let import_base64 = if sig
        .args
        .iter()
        .any(|x| x.typ == windmill_parser::Typ::Bytes)
    {
        "import base64"
    } else {
        ""
    };
    let import_datetime = if sig
        .args
        .iter()
        .any(|x| x.typ == windmill_parser::Typ::Datetime)
    {
        "from datetime import datetime"
    } else {
        ""
    };
    let wrapper_content: String = format!(
        r#"
import json
{import_loader}
{import_base64}
{import_datetime}
import traceback
import sys

inner_script = __import__("inner")

with open("args.json") as f:
    kwargs = json.load(f, strict=False)
for k, v in list(kwargs.items()):
    if v == '<function call>':
        del kwargs[k]
{transforms}
try:
    res = inner_script.main(**kwargs)
    res_json = json.dumps(res, separators=(',', ':'), default=str).replace('\n', '')
    with open("result.json", 'w') as f:
        f.write(res_json)
except Exception as e:
    exc_type, exc_value, exc_traceback = sys.exc_info()
    tb = traceback.format_tb(exc_traceback)
    with open("result.json", 'w') as f:
        err_json = json.dumps({{ "message": str(e), "name": e.__class__.__name__, "stack": '\n'.join(tb[1:])  }}, separators=(',', ':'), default=str).replace('\n', '')
        f.write(err_json)
        sys.exit(1)
"#,
    );
    write_file(job_dir, "main.py", &wrapper_content).await?;

    let mut reserved_variables = get_reserved_variables(job, &token, &base_url, db).await?;
    let additional_python_paths_folders = additional_python_paths.iter().join(":");
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
        reserved_variables.insert("PYTHONPATH".to_string(), additional_python_paths_folders);
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

    handle_child(&job.id, db, logs, timeout, *max_log_size, child).await?;
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
    let content = capture_dependency_job(
        &job.id,
        job.language.as_ref().map(|v| Ok(v)).unwrap_or_else(|| {
            Err(Error::InternalErr(
                "Job Language required for dependency jobs".to_owned(),
            ))
        })?,
        job.raw_code
            .as_ref()
            .map(|a| a.as_str())
            .unwrap_or_else(|| "no raw code"),
        logs,
        job_dir,
        db,
        timeout,
        envs,
    )
    .await;
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

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_flow_dependency_job(
    job: &QueuedJob,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    timeout: i32,
    envs: &Envs,
) -> error::Result<()> {
    let path = job.script_path.clone().ok_or_else(|| {
        error::Error::InternalErr(
            "Cannot resolve flow dependencies for flow without path".to_string(),
        )
    })?;
    let raw_flow = job.raw_flow.clone().map(|v| Ok(v)).unwrap_or_else(|| {
        Err(Error::InternalErr(
            "Flow Dependency requires raw flow".to_owned(),
        ))
    })?;
    let mut flow = serde_json::from_value::<FlowValue>(raw_flow).map_err(to_anyhow)?;
    let mut new_flow_modules = Vec::new();
    for mut e in flow.modules.into_iter() {
        let FlowModuleValue::RawScript { lock: _, path, content, language, input_transforms} = e.value else {
            new_flow_modules.push(e);
            continue;
        };
        // sync with windmill-api/scripts
        let dependencies = match language {
            ScriptLang::Python3 => windmill_parser_py::parse_python_imports(&content)?.join("\n"),
            _ => content.clone(),
        };
        let new_lock = capture_dependency_job(
            &job.id,
            &language,
            &dependencies,
            logs,
            job_dir,
            db,
            timeout,
            envs,
        )
        .await;
        match new_lock {
            Ok(new_lock) => {
                e.value = FlowModuleValue::RawScript {
                    lock: Some(new_lock),
                    path: path,
                    input_transforms,
                    content,
                    language,
                };
                new_flow_modules.push(e);
                continue;
            }
            Err(error) => {
                // TODO: Record flow raw script error lock logs
                tracing::warn!(
                    path = path,
                    language = ?language,
                    error = ?error,
                    logs = ?logs,
                    "Failed to generate flow lock for raw script"
                );
                e.value = FlowModuleValue::RawScript {
                    lock: None,
                    path: path,
                    input_transforms,
                    content,
                    language,
                };
                new_flow_modules.push(e);
                continue;
            }
        }
    }
    flow.modules = new_flow_modules;
    let new_flow_value = serde_json::to_value(flow).map_err(to_anyhow)?;

    // Re-check cancelation to ensure we don't accidentially override a flow.
    if sqlx::query_scalar!("SELECT canceled FROM queue WHERE id = $1", job.id)
        .fetch_optional(db)
        .await
        .map(|v| Some(true) == v)
        .unwrap_or_else(|err| {
            tracing::error!(%job.id, %err, "error checking cancelation for job {0}: {err}", job.id);
            false
        })
    {
        return Ok(());
    }

    sqlx::query!(
        "UPDATE flow SET value = $1 WHERE path = $2 AND workspace_id = $3",
        new_flow_value,
        path,
        job.workspace_id
    )
    .execute(db)
    .await?;
    Ok(())
}

#[cfg(not(feature = "deno-lock"))]
async fn generate_deno_lock(
    _job_id: &Uuid,
    _code: &str,
    _logs: &mut String,
    _job_dir: &str,
    _db: &sqlx::Pool<sqlx::Postgres>,
    _timeout: i32,
    _envs: &Envs,
) -> error::Result<String> {
    Ok(String::new())
}

#[cfg(feature = "deno-lock")]
async fn generate_deno_lock(
    job_id: &Uuid,
    code: &str,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    timeout: i32,
    Envs { deno_path, max_log_size, .. }: &Envs,
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

    handle_child(job_id, db, logs, timeout, *max_log_size, child).await?;

    let path_lock = format!("{job_dir}/lock.json");
    let mut file = File::open(path_lock).await?;
    let mut req_content = "".to_string();
    file.read_to_string(&mut req_content).await?;
    Ok(req_content)
}
async fn capture_dependency_job(
    job_id: &Uuid,
    job_language: &ScriptLang,
    job_raw_code: &str,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    timeout: i32,
    envs: &Envs,
) -> error::Result<String> {
    match job_language {
        ScriptLang::Python3 => {
            create_dependencies_dir(job_dir).await;
            pip_compile(job_id, job_raw_code, logs, job_dir, envs, db, timeout).await
        }
        ScriptLang::Go => {
            install_go_dependencies(
                job_id,
                job_raw_code,
                logs,
                job_dir,
                db,
                timeout,
                envs.max_log_size,
                &envs.go_path,
                false,
                false,
            )
            .await
        }
        ScriptLang::Deno => {
            generate_deno_lock(job_id, job_raw_code, logs, job_dir, db, timeout, &envs).await
        }
        ScriptLang::Bash => Ok("".to_owned()),
    }
}

async fn pip_compile(
    job_id: &Uuid,
    requirements: &str,
    logs: &mut String,
    job_dir: &str,
    Envs { pip_extra_index_url, pip_index_url, pip_trusted_host, max_log_size, .. }: &Envs,
    db: &Pool<Postgres>,
    timeout: i32,
) -> error::Result<String> {
    logs.push_str(&format!("\nresolving dependencies..."));
    set_logs(logs, job_id, db).await;
    logs.push_str(&format!("\ncontent of requirements:\n{}", requirements));
    let file = "requirements.in";
    write_file(job_dir, file, &requirements).await?;

    let mut args = vec!["-q", "--no-header", file, "--resolver=backtracking"];
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
    handle_child(job_id, db, logs, timeout, *max_log_size, child)
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
    max_log_size: i64,
    go_path: &str,
    preview: bool,
    skip_go_mod: bool,
) -> error::Result<String> {
    if !skip_go_mod {
        gen_go_mymod(code, job_dir).await?;
        let child = Command::new("go")
            .current_dir(job_dir)
            .args(vec!["mod", "init", "mymod"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        handle_child(job_id, db, logs, timeout, max_log_size, child).await?;
    }
    let child = Command::new(go_path)
        .current_dir(job_dir)
        .args(vec!["mod", "tidy"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    handle_child(job_id, db, logs, timeout, max_log_size, child)
        .await
        .map_err(|e| Error::ExecutionErr(format!("Lock file generation failed: {e:?}")))?;

    if preview {
        Ok(String::new())
    } else {
        let mut req_content = "".to_string();

        let mut file = File::open(format!("{job_dir}/go.mod")).await?;
        file.read_to_string(&mut req_content).await?;

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
        &job.email,
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
    max_log_size: i64,
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
                            , canceled_by = 'timeout'
                            , canceled_reason = $1
                        WHERE id = $2
                    "#,
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
        let mut log_remaining = (max_log_size as usize).saturating_sub(logs.chars().count());
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
                                "Job logs or result reached character limit of {max_log_size}; killing job."
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
        _ if *too_many_logs.borrow() => Err(Error::ExecutionErr(format!(
            "logs or result reached limit. Set MAX_LOG_SIZE higher (current: {max_log_size})"
        ))),
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
async fn set_logs(logs: &str, id: &uuid::Uuid, db: &Pool<Postgres>) {
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

async fn handle_python_reqs(
    requirements: Vec<&str>,
    Envs {
        python_path,
        path_env,
        pip_index_url,
        pip_extra_index_url,
        pip_trusted_host,
        nsjail_path,
        max_log_size,
        ..
    }: &Envs,
    job: &QueuedJob,
    logs: &mut String,
    db: &sqlx::Pool<sqlx::Postgres>,
    timeout: i32,
    disable_nsjail: bool,
    worker_name: &str,
    job_dir: &str,
) -> error::Result<Vec<String>> {
    let mut req_paths: Vec<String> = vec![];
    let mut vars = vec![("PATH", path_env)];
    if !disable_nsjail {
        if let Some(url) = pip_extra_index_url {
            vars.push(("EXTRA_INDEX_URL", url));
        }
        if let Some(url) = pip_index_url {
            vars.push(("INDEX_URL", url));
        }
        if let Some(host) = pip_trusted_host {
            vars.push(("TRUSTED_HOST", host));
        }
    };

    for req in requirements {
        // todo: handle many reqs
        let venv_p = format!("{PIP_CACHE_DIR}/{req}");
        if metadata(&venv_p).await.is_ok() {
            req_paths.push(venv_p);
            continue;
        }

        logs.push_str("\n--- PIP INSTALL ---\n");
        logs.push_str(&format!("\n{req} is being installed for the first time.\n It will be cached for all ulterior uses."));

        tracing::info!(
            worker_name = %worker_name,
            job_id = %job.id,
            workspace_id = %job.workspace_id,
            "started setup python dependencies"
        );

        let child = if !disable_nsjail {
            tracing::info!(
                worker_name = %worker_name,
                job_id = %job.id,
                workspace_id = %job.workspace_id,
                "starting nsjail"
            );
            let mut vars = vars.clone();
            let req = req.to_string();
            vars.push(("REQ", &req));
            vars.push(("TARGET", &venv_p));
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
                &req,
                "-I",
                "--no-deps",
                "--no-color",
                "--isolated",
                "--no-warn-conflicts",
                "--disable-pip-version-check",
                "-t",
                venv_p.as_str(),
            ];
            if let Some(url) = pip_extra_index_url {
                args.extend(["--extra-index-url", &url]);
            }
            if let Some(url) = pip_index_url {
                args.extend(["--index-url", &url]);
            }
            if let Some(host) = pip_trusted_host {
                args.extend(["--trusted-host", &host]);
            }
            Command::new(python_path)
                .env_clear()
                .env("PATH", path_env)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        };

        let child = handle_child(&job.id, db, logs, timeout, *max_log_size, child).await;
        tracing::info!(
            worker_name = %worker_name,
            job_id = %job.id,
            workspace_id = %job.workspace_id,
            is_ok = child.is_ok(),
            "finished setting up python dependencies {}",
            job.id
        );
        child?;

        req_paths.push(venv_p);
    }
    Ok(req_paths)
}
