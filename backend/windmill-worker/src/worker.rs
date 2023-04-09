/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use anyhow::Result;
use const_format::concatcp;
use itertools::Itertools;
use lazy_static::lazy_static;
use once_cell::sync::OnceCell;
use regex::Regex;
use sqlx::{Pool, Postgres};
use windmill_api_client::Client;
use windmill_parser_go::parse_go_imports;
use std::{
    borrow::Borrow, collections::HashMap, io, os::unix::process::ExitStatusExt, panic,
    process::Stdio, time::Duration, sync::atomic::Ordering,
};
use tracing::{trace_span, Instrument};
use uuid::Uuid;

use windmill_common::{
    error::{self, to_anyhow, Error},
    flows::{FlowModuleValue, FlowValue},
    scripts::{ScriptHash, ScriptLang},
    utils::{rd_string, calculate_hash},
    variables, BASE_URL, users::SUPERADMIN_SECRET_EMAIL, IS_READY, METRICS_ENABLED,
};
use windmill_queue::{canceled_job_to_result, get_queued_job, pull, JobKind, QueuedJob, CLOUD_HOSTED};

use serde_json::{json, Value};

use tokio::{
    fs::{metadata, symlink, DirBuilder, File},
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::{
        mpsc::{self, Sender},  watch, broadcast
    },
    time::{interval, sleep, Instant, MissedTickBehavior},
};

use futures::{
    future::{self, ready, FutureExt},
    stream, StreamExt,
};

use async_recursion::async_recursion;

use crate::{
    jobs::{add_completed_job, add_completed_job_error},
    worker_flow::{
        handle_flow, update_flow_status_after_job_completion, update_flow_status_in_progress,
    },
};

#[cfg(feature = "enterprise")]
use rand::Rng;

#[cfg(feature = "enterprise")]
const TAR_CACHE_FILENAME: &str = "entirecache.tar";

#[cfg(feature = "enterprise")]
async fn copy_cache_from_bucket(bucket: &str, tx: Option<Sender<()>>) -> Option::<tokio::task::JoinHandle<()>> {
    tracing::info!("Copying cache from bucket in the background {bucket}");
    let bucket = bucket.to_string();
    let tx_is_some = tx.is_some();
    let f = async move { 
        let elapsed = Instant::now();

        match Command::new("rclone")
            .arg("copy")
            .arg(format!(":s3,env_auth=true:{bucket}"))
            .arg(if tx_is_some { ROOT_TMP_CACHE_DIR } else { ROOT_CACHE_DIR })
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

        for x in 
            if !tx_is_some 
                { [PIP_CACHE_DIR, DENO_CACHE_DIR, GO_CACHE_DIR] }
                else { [PIP_TMP_CACHE_DIR, DENO_TMP_CACHE_DIR, GO_TMP_CACHE_DIR] } {
            DirBuilder::new()
                .recursive(true)
                .create(x)
                .await
                .expect("could not create initial worker dir");
        }

        if let Some(tx) = tx {
            tx.send(()).await.expect("can send copy cache signal");
        }
    };
    if tx_is_some {
        return Some(tokio::spawn(f));
    } else {
        f.await;
        return None;
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
        Err(e) => tracing::info!("Failed to run periodic job push. Error: {:?}", e),
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
        .arg(format!("{ROOT_CACHE_DIR}{TAR_CACHE_FILENAME}"))
        .args(&["pip", "go", "deno"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            if !h.wait().await.unwrap().success() {
                tracing::info!("Failed to tar cache");
                return;
            }
        }
        Err(e) => {
            tracing::info!("Failed tar cache. Error: {e:?}");
            return;
        }
    }

    let tar_metadata = tokio::fs::metadata(format!("{ROOT_CACHE_DIR}{TAR_CACHE_FILENAME}"))
    .await;
    if tar_metadata.is_err() || tar_metadata.as_ref().unwrap().len() == 0 {
        tracing::info!("Failed to tar cache");
        return;
    }

    match Command::new("rclone")
        .current_dir(ROOT_CACHE_DIR)
        .arg("copyto")
        .arg(TAR_CACHE_FILENAME)
        .arg(format!(":s3,env_auth=true:{bucket}/{TAR_CACHE_FILENAME}"))
        .arg("-vv")
        .arg("--size-only")
        .arg("--fast-list")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            h.wait().await.unwrap();
        }
        Err(e) => tracing::info!("Failed to copy tar cache to bucket. Error: {:?}", e),
    }
    
    if let Err(e) = tokio::fs::remove_file(format!("{ROOT_CACHE_DIR}{TAR_CACHE_FILENAME}")).await {
        tracing::info!("Failed to remove tar cache. Error: {:?}", e);
    };

    tracing::info!(
        "Finished copying cache to bucket {bucket} as tar, took: {:?}s. Size of new tar: {}",
        elapsed.elapsed().as_secs(),
        tar_metadata.unwrap().len()
    );
}

#[cfg(feature = "enterprise")]
async fn copy_cache_from_bucket_as_tar(bucket: &str) -> bool {
    tracing::info!("Copying cache from bucket {bucket} as tar");
    let elapsed = Instant::now();

    match Command::new("rclone")
        .arg("copyto")
        .arg(format!(":s3,env_auth=true:{bucket}/{TAR_CACHE_FILENAME}"))
        .arg(format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}"))
        .arg("-vv")
        .arg("--size-only")
        .arg("--fast-list")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            if !h.wait().await.unwrap().success() {
                tracing::info!("Failed to download tar cache, continuing nonetheless");
                return false;
            }
        }
        Err(e) => {
            tracing::info!("Failed to download tar cache, continuing nonetheless. Error: {e:?}");
            return false;
        }
    }

    match Command::new("tar")
        .current_dir(ROOT_TMP_CACHE_DIR)
        .arg("-xpvf")
        .arg(format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(mut h) => {
            if !h.wait().await.unwrap().success() {
                tracing::info!("Failed to untar cache, continuing nonetheless");
                return false;
            }
        }
        Err(e) => {
            tracing::warn!("Failed to untar cache, continuing nonetheless. Error: {e:?}");
            return false;
        }
    }

    if let Err(e) = tokio::fs::remove_file(format!("{ROOT_TMP_CACHE_DIR}{TAR_CACHE_FILENAME}")).await {
        tracing::info!("Failed to remove tar cache. Error: {:?}", e);
    };

    let r = move_tmp_cache_to_cache().await.is_ok();

    tracing::info!(
        "Finished copying cache from bucket {bucket} as tar, took: {:?}s. copy success: {r}",
        elapsed.elapsed().as_secs()
    );
    return r;
}

// async fn check_if_bucket_syncable(bucket: &str) -> bool {
//     match Command::new("rclone")
//         .arg("lsf")
//         .arg(format!(":s3,env_auth=true:{bucket}/NOSYNC"))

//         .arg("-vv")
//         .arg("--fast-list")
//         .stdin(Stdio::null())
//         .stdout(Stdio::null())
//         .output()
//         .await;
//     return true;
// }

async fn move_tmp_cache_to_cache() -> Result<()> {
    tokio::fs::remove_dir_all(ROOT_CACHE_DIR).await?;
    tokio::fs::rename(ROOT_TMP_CACHE_DIR, ROOT_CACHE_DIR).await?;
    tracing::info!("Finished moving tmp cache to cache");
    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn create_token_for_owner(
    db: &Pool<Postgres>,
    w_id: &str,
    owner: &str,
    label: &str,
    expires_in: i32,
    email: &str,
) -> error::Result<String> {
    // TODO: Bad implementation. We should not have access to this DB here.
    let token: String = rd_string(30);
    let is_super_admin = sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
            .fetch_optional(db)
            .await?
            .unwrap_or(false) || email == SUPERADMIN_SECRET_EMAIL;

    sqlx::query_scalar!(
        "INSERT INTO token
            (workspace_id, token, owner, label, expiration, super_admin, email)
            VALUES ($1, $2, $3, $4, now() + ($5 || ' seconds')::interval, $6, $7)",
        &w_id,
        token,
        owner,
        label,
        expires_in.to_string(),
        is_super_admin,
        email
    )
    .execute(db)
    .await?;
    Ok(token)
}

const TMP_DIR: &str = "/tmp/windmill";
const ROOT_CACHE_DIR: &str = "/tmp/windmill/cache/";
const ROOT_TMP_CACHE_DIR: &str = "/tmp/windmill/tmpcache/";
const PIP_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "pip");
const DENO_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "deno");
const GO_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "go");
const PIP_TMP_CACHE_DIR: &str = concatcp!(ROOT_TMP_CACHE_DIR, "pip");
const DENO_TMP_CACHE_DIR: &str = concatcp!(ROOT_TMP_CACHE_DIR, "deno");
const GO_TMP_CACHE_DIR: &str = concatcp!(ROOT_TMP_CACHE_DIR, "go");
const NUM_SECS_PING: u64 = 5;
const NUM_SECS_SYNC: u64 = 60 * 10;

const INCLUDE_DEPS_PY_SH_CONTENT: &str = include_str!("../nsjail/download_deps.py.sh");
const NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT: &str = include_str!("../nsjail/download.py.config.proto");
const NSJAIL_CONFIG_RUN_PYTHON3_CONTENT: &str = include_str!("../nsjail/run.python3.config.proto");
const NSJAIL_CONFIG_RUN_GO_CONTENT: &str = include_str!("../nsjail/run.go.config.proto");
const NSJAIL_CONFIG_RUN_BASH_CONTENT: &str = include_str!("../nsjail/run.bash.config.proto");

const RELATIVE_PYTHON_LOADER: &str = include_str!("../loader.py");

const GO_REQ_SPLITTER: &str = "//go.sum\n";

#[derive(Clone)]
pub struct Metrics {
    pub worker_execution_failed: prometheus::IntCounter,
}


pub const DEFAULT_TIMEOUT: u16 = 300;
pub const DEFAULT_SLEEP_QUEUE: u64 = 50;

lazy_static::lazy_static! {
    


    static ref SLEEP_QUEUE: u64 = std::env::var("SLEEP_QUEUE")
    .ok()
    .and_then(|x| x.parse::<u64>().ok())
    .unwrap_or(DEFAULT_SLEEP_QUEUE);

    static ref DISABLE_NUSER: bool = std::env::var("DISABLE_NUSER")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(false);

    static ref DISABLE_NSJAIL: bool = std::env::var("DISABLE_NSJAIL")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(true);

    pub static ref KEEP_JOB_DIR: bool = std::env::var("KEEP_JOB_DIR")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(false);

    static ref S3_CACHE_BUCKET: Option<String> = std::env::var("S3_CACHE_BUCKET")
    .ok()
    .map(|e| Some(e))
    .unwrap_or(None);

    static ref DENO_PATH: String = std::env::var("DENO_PATH").unwrap_or_else(|_| "/usr/bin/deno".to_string());
    static ref GO_PATH: String = std::env::var("GO_PATH").unwrap_or_else(|_| "/usr/bin/go".to_string());
    static ref PYTHON_PATH: String =
        std::env::var("PYTHON_PATH").unwrap_or_else(|_| "/usr/local/bin/python3".to_string());
    static ref NSJAIL_PATH: String = std::env::var("NSJAIL_PATH").unwrap_or_else(|_| "nsjail".to_string());
    static ref PATH_ENV: String = std::env::var("PATH").unwrap_or_else(|_| String::new());
    static ref HOME_ENV: String = std::env::var("HOME").unwrap_or_else(|_| String::new());
    static ref PIP_INDEX_URL: Option<String> = std::env::var("PIP_INDEX_URL").ok();
    static ref PIP_EXTRA_INDEX_URL: Option<String> = std::env::var("PIP_EXTRA_INDEX_URL").ok();
    static ref PIP_TRUSTED_HOST: Option<String> = std::env::var("PIP_TRUSTED_HOST").ok();
    static ref DENO_AUTH_TOKENS: String = std::env::var("DENO_AUTH_TOKENS")
        .ok()
        .map(|x| format!(";{x}"))
        .unwrap_or_else(|| String::new());

    static ref NPM_CONFIG_REGISTRY: Option<String> = std::env::var("NPM_CONFIG_REGISTRY").ok();

    static ref DENO_FLAGS: Option<Vec<String>> = std::env::var("DENO_FLAGS")
        .ok()
        .map(|x| x.split(' ').map(|x| x.to_string()).collect());
        
    static ref PIP_LOCAL_DEPENDENCIES: Option<Vec<String>> = {
        let pip_local_dependencies = std::env::var("PIP_LOCAL_DEPENDENCIES")
            .ok()
            .map(|x| x.split(',').map(|x| x.to_string()).collect());
        if pip_local_dependencies == Some(vec!["".to_string()]) {
            None
        } else {
            pip_local_dependencies
        }
    
    };
    static ref WHITELIST_WORKSPACES: Option<Vec<String>> = std::env::var("WHITELIST_WORKSPACES")
        .ok()
        .map(|x| x.split(',').map(|x| x.to_string()).collect());
    static ref BLACKLIST_WORKSPACES: Option<Vec<String>>  = std::env::var("BLACKLIST_WORKSPACES")
        .ok()
        .map(|x| x.split(',').map(|x| x.to_string()).collect());

    static ref TAR_CACHE_RATE: i32 = std::env::var("TAR_CACHE_RATE")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(100);

    static ref ADDITIONAL_PYTHON_PATHS: Option<Vec<String>> = std::env::var("ADDITIONAL_PYTHON_PATHS")
        .ok()
        .map(|x| x.split(':').map(|x| x.to_string()).collect());


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
        "Total number of seconds since the worker has started"
    );

    static ref TIMEOUT: u16 = std::env::var("TIMEOUT")
        .ok()
        .and_then(|x| x.parse::<u16>().ok())
        .unwrap_or(DEFAULT_TIMEOUT as u16);

    static ref TIMEOUT_DURATION: Duration = Duration::from_secs(*TIMEOUT as u64);

    static ref ZOMBIE_JOB_TIMEOUT: String = std::env::var("ZOMBIE_JOB_TIMEOUT")
        .ok()
        .and_then(|x| x.parse::<String>().ok())
        .unwrap_or_else(|| "30".to_string());


    pub static ref RESTART_ZOMBIE_JOBS: bool = std::env::var("RESTART_ZOMBIE_JOBS")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(true);

    static ref SESSION_TOKEN_EXPIRY: i32 = (*TIMEOUT as i32) * 2;
}

//only matter if CLOUD_HOSTED
const MAX_RESULT_SIZE: usize = 1024 * 1024 * 2; // 2MB

#[derive(Clone)]
pub struct AuthedClient {
    pub base_internal_url: String,
    pub workspace: String,
    pub token: String,
    pub client: OnceCell<Client>
}

impl AuthedClient {
    pub fn get_client(&self) -> &Client {
        return self.client.get_or_init(|| {
            windmill_api_client::create_client(&self.base_internal_url, self.token.clone())
        });
    }
}


#[tracing::instrument(level = "trace")]
pub async fn run_worker(
    db: &Pool<Postgres>,
    worker_instance: &str,
    worker_name: String,
    i_worker: u64,
    ip: &str,
    mut rx: tokio::sync::broadcast::Receiver<()>,
    base_internal_url: &str,
) {

    #[cfg(not(feature = "enterprise"))]
    if !*DISABLE_NSJAIL {
        tracing::warn!(
            "NSJAIL to sandbox process in untrusted environments is an enterprise feature but allowed to be used for testing purposes"
        );
    }

    let start_time = Instant::now();

    let worker_dir = format!("{TMP_DIR}/{worker_name}");
    tracing::debug!(worker_dir = %worker_dir, worker_name = %worker_name, "Creating worker dir");

    for x in [&worker_dir, ROOT_TMP_CACHE_DIR, PIP_CACHE_DIR, DENO_CACHE_DIR, GO_CACHE_DIR] {
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

    let mut last_ping = Instant::now() - Duration::from_secs(NUM_SECS_PING + 1);

    insert_initial_ping(worker_instance, &worker_name, ip, db).await;

    let uptime_metric = prometheus::register_counter!(WORKER_UPTIME_OPTS
        .clone()
        .const_label("name", &worker_name))
    .unwrap();


    let worker_execution_duration = prometheus::register_histogram_vec!(
        prometheus::HistogramOpts::new(
            "worker_execution_duration",
            "Duration between receiving a job and completing it",
        )
        .const_label("name", &worker_name),
        &["workspace_id", "language"],
    )
    .expect("register prometheus metric");

    let worker_execution_duration_counter = prometheus::register_counter!(prometheus::opts!(
        "worker_execution_duration_counter",
        "Total number of seconds spent executing jobs"
    )
        .const_label("name", &worker_name))
        .expect("register prometheus metric");


    let worker_sleep_duration = prometheus::register_histogram!(prometheus::HistogramOpts::new(
        "worker_sleep_duration",
        "Duration sleeping waiting for job",
    )
    .const_label("name", &worker_name),)
    .expect("register prometheus metric");


    let worker_sleep_duration_counter = prometheus::register_counter!(prometheus::opts!(
        "worker_sleep_duration_counter",
        "Total number of seconds spent sleeping between pulling jobs from the queue"
    )
        .const_label("name", &worker_name))
        .expect("register prometheus metric");


    let worker_pull_duration = prometheus::register_histogram!(prometheus::HistogramOpts::new(
        "worker_pull_duration",
        "Duration pulling next job",
    )
    .const_label("name", &worker_name),)
    .expect("register prometheus metric");

    let worker_pull_duration_counter = prometheus::register_counter!(prometheus::opts!(
        "worker_pull_duration_counter",
        "Total number of seconds spent pulling jobs (if growing large the db is undersized)"
    )
        .const_label("name", &worker_name))
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

    let worker_busy: prometheus::IntGauge = prometheus::register_int_gauge!(prometheus::Opts::new(
        "worker_busy",
        "Is the worker busy executing a job?",
    )
    .const_label("name", &worker_name))
    .unwrap();

    let mut jobs_executed = 0;

    if *METRICS_ENABLED {
        WORKER_STARTED.inc();
    }

    let (_copy_bucket_tx, mut _copy_bucket_rx) = mpsc::channel::<()>(2);

    let mut copy_cache_from_bucket_handle: Option<tokio::task::JoinHandle<()>> = None;

    let mut initialized_cache = false;

    #[cfg(feature = "enterprise")]
    if let Some(ref s) = S3_CACHE_BUCKET.clone() {
        // We try to download the entire cache as a tar, it is much faster over S3
        if !copy_cache_from_bucket_as_tar(&s).await {
            // We revert to copying the cache from the bucket
            copy_cache_from_bucket_handle = copy_cache_from_bucket(&s, Some(_copy_bucket_tx.clone())).await;
        } else {
            initialized_cache = true;
        }
    }

    IS_READY.store(true, Ordering::Relaxed);

    tracing::info!(worker = %worker_name, "starting worker");

    #[cfg(feature = "enterprise")]
    let mut last_sync =
        Instant::now() + Duration::from_secs(rand::thread_rng().gen_range(0..NUM_SECS_SYNC));

    let (same_worker_tx, mut same_worker_rx) = mpsc::channel::<Uuid>(5);

    tracing::info!(worker = %worker_name, "listening for jobs");

    
    loop {
        if *METRICS_ENABLED {
            worker_busy.set(0);
            uptime_metric.inc_by(
                (((Instant::now() - start_time).as_millis() as f64)/1000.0 - uptime_metric.get())
                    .try_into()
                    .unwrap(),
            );
        }


        let do_break = async {
            if last_ping.elapsed().as_secs() > NUM_SECS_PING {
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
            if initialized_cache && last_sync.elapsed().as_secs() > NUM_SECS_SYNC {
                if let Some(ref s) = S3_CACHE_BUCKET.clone() {
                    copy_cache_from_bucket(&s, None).await;
                    copy_cache_to_bucket(&s).await;
                    
                    // this is to prevent excessive tar upload. 1/100*15min = each worker sync its tar once per day on average
                    if rand::thread_rng().gen_range(0..*TAR_CACHE_RATE) == 1 {
                        copy_cache_to_bucket_as_tar(&s).await;
                    }
                }
                last_sync = Instant::now();
            }

            let (do_break, next_job) = async {
                tokio::select! {
                    biased;
                    _ = rx.recv() => {
                        if let Some(copy_cache_from_bucket_handle) = copy_cache_from_bucket_handle.as_ref() {
                            copy_cache_from_bucket_handle.abort();
                        }
                        println!("received killpill for worker {}", i_worker);
                        (true, Ok(None))
                    },
                    _ = _copy_bucket_rx.recv() => {
                        if let Err(e) = move_tmp_cache_to_cache().await {
                            tracing::error!(worker = %worker_name, "failed to sync tmp cache to cache: {}", e);
                        }
                        copy_cache_from_bucket_handle = None;
                        initialized_cache = true;
                        (false, Ok(None))
                    },
                    Some(job_id) = same_worker_rx.recv() => {
                        (false, sqlx::query_as::<_, QueuedJob>("SELECT * FROM queue WHERE id = $1")
                        .bind(job_id)
                        .fetch_optional(db)
                        .await
                        .map_err(|_| Error::InternalErr("Impossible to fetch same_worker job".to_string())))
                    },
                    (job, timer) = {
                        let timer = if *METRICS_ENABLED { Some(worker_pull_duration.start_timer()) } else { None }; 
                        pull(&db, WHITELIST_WORKSPACES.clone(), BLACKLIST_WORKSPACES.clone()).map(|x| (x, timer)) } => {
                        timer.map(|timer| {
                            let duration_pull_s = timer.stop_and_record();
                            worker_pull_duration_counter.inc_by(duration_pull_s);
                        });
                        (false, job)
                    },
                }
            }.instrument(trace_span!("worker_get_next_job")).await;
            if do_break {
                return true;
            }
            if *METRICS_ENABLED {
                worker_busy.set(1);
            }
            match next_job {
                Ok(Some(job)) => {
                    let (token_tx, mut token_rx) = broadcast::channel::<()>(1);

                    let label_values = [
                        &job.workspace_id,
                        job.language.as_ref().map(|l| l.as_str()).unwrap_or(""),
                    ];

                    let _timer = worker_execution_duration
                        .with_label_values(label_values.as_slice())
                        .start_timer();

                    jobs_executed += 1;
                    if *METRICS_ENABLED {
                        worker_execution_count
                            .with_label_values(label_values.as_slice())
                            .inc();
                    }

                    let metrics = Metrics {
                        worker_execution_failed: worker_execution_failed
                            .with_label_values(label_values.as_slice()),
                    };

                    let job_root = job.root_job.map(|x| x.to_string()).unwrap_or_else(|| "none".to_string());
                    tracing::info!(worker = %worker_name, id = %job.id, root_id = %job_root, "fetched job {}, root job: {}", job.id, job_root);

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
                    let token = create_token_for_owner(
                        &db,
                        &job.workspace_id,
                        &job.permissioned_as,
                        "ephemeral-script",
                        *SESSION_TOKEN_EXPIRY,
                        &job.email,
                    )
                    .await.expect("could not create job token");
                    let authed_client = AuthedClient { base_internal_url: base_internal_url.to_string(), token: token.clone(), workspace: job.workspace_id.to_string(), client: OnceCell::new() };
                    let is_flow = job.job_kind == JobKind::Flow || job.job_kind == JobKind::FlowPreview || job.job_kind == JobKind::FlowDependencies;

                    if let Some(err) = handle_queued_job(
                        job.clone(),
                        db,
                        &authed_client,
                        token,
                        &worker_name,
                        &worker_dir,
                        &job_dir,
                        metrics.clone(),
                        same_worker_tx.clone(),
                        base_internal_url
                    )
                    .await
                    .err()
                    {
                        handle_job_error(
                            db,
                            &authed_client,
                            job,
                            err,
                            Some(metrics),
                            false,
                            same_worker_tx.clone(),
                            &worker_dir,
                            base_internal_url
                        )
                        .await;
                    };

                    let duration = _timer.stop_and_record();
                    worker_execution_duration_counter.inc_by(duration);

                    if !*KEEP_JOB_DIR && !(is_flow && same_worker) {
                        let _ = tokio::fs::remove_dir_all(job_dir).await;
                    }
                }
                Ok(None) => {
                    let _timer =  if *METRICS_ENABLED { Some(worker_sleep_duration.start_timer()) } else { None };
                    tokio::time::sleep(Duration::from_millis(*SLEEP_QUEUE)).await;
                    _timer.map(|timer| {
                        let duration = timer.stop_and_record();
                        worker_sleep_duration_counter.inc_by(duration);
                    });
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
    client: &AuthedClient,
    job: QueuedJob,
    err: Error,
    metrics: Option<Metrics>,
    unrecoverable: bool,
    same_worker_tx: Sender<Uuid>,
    worker_dir: &str,
    base_internal_url: &str,
) {
    let err = match err {
        Error::JsonErr(err) => err,
        _ => json!({"message": err.to_string(), "name": "InternalErr"}),
    };

    let update_job_future = || {
        add_completed_job_error(
            db,
            &job,
            format!("Unexpected error during job execution:\n{err}"),
            err.clone(),
            metrics.clone(),
        )
    };

    if job.is_flow_step || job.job_kind == JobKind::FlowPreview || job.job_kind == JobKind::Flow {
        let (flow, job_status_to_update) = if let Some(parent_job_id) = job.parent_job {
            let _ = update_job_future().await;
            (parent_job_id, job.id)
        } else {
            (job.id, Uuid::nil())
        };

        let updated_flow = update_flow_status_after_job_completion(
            db,
            client,
            flow,
            &job_status_to_update,
            &job.workspace_id,
            false,
            json!({ "error": err }),
            metrics.clone(),
            unrecoverable,
            same_worker_tx,
            worker_dir,
            None,
            base_internal_url,
            
        )
        .await;

        if let Err(err) = updated_flow {
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
    if job.parent_job.is_none() {
        let _ = update_job_future().await;
    }
    tracing::error!(job_id = %job.id, "error handling job: {err:#?} {} {} {}", job.id, job.workspace_id, job.created_by);
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


fn extract_error_value(log_lines: &str, i: i32) -> serde_json::Value {
    return json!({"message": format!("ExitCode: {i}, last log lines: {}", log_lines.to_string().trim().to_string()), "name": "ExecutionErr"});
}
#[tracing::instrument(level = "trace", skip_all)]
async fn handle_queued_job(
    job: QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClient,
    token: String,
    worker_name: &str,
    worker_dir: &str,
    job_dir: &str,
    metrics: Metrics,
    same_worker_tx: Sender<Uuid>,
    base_internal_url: &str,
) -> windmill_common::error::Result<()> {
    if job.canceled {
        return Err(Error::JsonErr(canceled_job_to_result(&job)))?;
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
                base_internal_url
            )
            .await?;
        }
        _ => {
            let mut logs = "".to_string();
            if let Some(log_str) = &job.logs {
                logs.push_str(&log_str);
            }

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
                    handle_dependency_job(&job, &mut logs, job_dir, db, worker_name).await
                }
                JobKind::FlowDependencies => {
                    handle_flow_dependency_job(&job, &mut logs, job_dir, db, worker_name)
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
                        base_internal_url,
                        worker_name
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
                                None,
                                base_internal_url,
                        
                            )
                            .await?;
                        }
                    }
                }
                Err(e) => {
                    let error_value = match e {
                        Error::ExitStatus(i) => {
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

                                extract_error_value(log_lines, i)
                            }
                        }
                        err @ _ => {
                            json!({"message": format!("error during execution of the script:\n{}", err), "name": "ExecutionErr"})
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
                                None,
                                base_internal_url,
                                
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
    name: &str,
    client: &AuthedClient,
    workspace: &str,
    v: Value,
) -> error::Result<Value> {
    match v {
        Value::String(y) if y.starts_with("$var:") => {
            let path = y.strip_prefix("$var:").unwrap();
            let v = client.get_client()
                .get_variable(workspace, path, Some(true))
                .await
                .map_err(|_| Error::NotFound(format!("Variable {path} not found for `{name}`")))
                .map(|v| v.into_inner())?
                .value
                .unwrap_or_else(|| String::new());
            Ok(Value::String(v))
        }
        Value::String(y) if y.starts_with("$res:") => {
            let path = y.strip_prefix("$res:").unwrap();
            if path.split("/").count() < 2 {
                return Err(Error::InternalErr(format!(
                    "Argument `{name}` is an invalid resource path: {path}",
                )));
            }
            let v = client.get_client()
                .get_resource_value(workspace, path)
                .await
                .map_err(|_| Error::NotFound(format!("Resource {path} not found for `{name}`")))?
                .into_inner();
            transform_json_value(name, client, workspace, v).await
        }
        Value::Object(mut m) => {
            for (a, b) in m.clone().into_iter() {
                m.insert(
                    a.clone(),
                    transform_json_value(&a, client, workspace, b).await?,
                );
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
    client: &AuthedClient,
    token: String,
    job_dir: &str,
    worker_dir: &str,
    logs: &mut String,
    base_internal_url: &str,
    worker_name: &str

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
            "SELECT content, lock, language FROM script WHERE hash = $1 AND workspace_id = $2",
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

    let shared_mount = if job.same_worker && job.language != Some(ScriptLang::Deno) {
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
                requirements_o,
                job_dir,
                worker_dir,
                worker_name,
                job,
                logs,
                db,
                client,
                token,
                &inner_content,
                &shared_mount,
                base_internal_url,
            )
            .await
        }
        Some(ScriptLang::Deno) => {
            handle_deno_job(
                logs,
                job,
                db,
                client,
                token,
                job_dir,
                &inner_content,
                base_internal_url,
                worker_name
            )
            .await
        }
        Some(ScriptLang::Go) => {
            handle_go_job(
                logs,
                job,
                db,
                client,
                token,
                &inner_content,
                job_dir,
                requirements_o,
                &shared_mount,
                base_internal_url,
                worker_name
            )
            .await
        }
        Some(ScriptLang::Bash) => {
            handle_bash_job(
                logs,
                job,
                db,
                token,
                &inner_content,
                job_dir,
                &shared_mount,
                base_internal_url,
                worker_name
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

async fn gen_go_mod(inner_content: &str, job_dir: &str, requirements: &str) -> error::Result<(bool, bool)> {
    gen_go_mymod(inner_content, job_dir).await?;

    let md = requirements
        .split_once(GO_REQ_SPLITTER);
    if let Some((req, sum)) = md {
        write_file(job_dir, "go.mod", &req).await?;
        write_file(job_dir, "go.sum", &sum).await?;
        Ok((true, true))
    } else {
        write_file(job_dir, "go.mod", &requirements).await?;
        Ok((true, false))
    }
}
#[tracing::instrument(level = "trace", skip_all)]
async fn handle_go_job(
    logs: &mut String,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClient,
    token: String,
    inner_content: &str,
    job_dir: &str,
    requirements_o: Option<String>,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
) -> Result<serde_json::Value, Error> {
    //go does not like executing modules at temp root
    let job_dir = &format!("{job_dir}/go");
    let (skip_go_mod, skip_tidy) = if let Some(requirements) = requirements_o {
        gen_go_mod(inner_content, job_dir, &requirements).await?
    } else {
        (false, false)
    };
    logs.push_str("\n\n--- GO DEPENDENCIES SETUP ---\n");
    set_logs(logs, &job.id, db).await;

    install_go_dependencies(
        &job.id,
        inner_content,
        logs,
        job_dir,
        db,
        true,
        skip_go_mod,
        skip_tidy,
        worker_name,
        &job.workspace_id,
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
    let mut reserved_variables = get_reserved_variables(job, &token, db).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

    let child = if !*DISABLE_NSJAIL {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_GO_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", GO_CACHE_DIR)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount),
        )
        .await?;
        let build_go = Command::new(GO_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("GOPATH", GO_CACHE_DIR)
            .env("HOME", HOME_ENV.as_str())
            .args(vec!["build", "main.go"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        handle_child(&job.id, db, logs,  build_go, false, worker_name, &job.workspace_id).await?;

        Command::new(NSJAIL_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec!["--config", "run.config.proto", "--", "/tmp/go/main"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        Command::new(GO_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("GOPATH", GO_CACHE_DIR)
            .env("HOME", HOME_ENV.as_str())
            .args(vec!["run", "main.go"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };
    handle_child(&job.id, db, logs, child, !*DISABLE_NSJAIL, worker_name, &job.workspace_id).await?;
    read_result(job_dir).await
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_bash_job(
    logs: &mut String,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    token: String,
    content: &str,
    job_dir: &str,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
) -> Result<serde_json::Value, Error> {
    logs.push_str("\n\n--- BASH CODE EXECUTION ---\n");
    set_logs(logs, &job.id, db).await;
    write_file(job_dir, "main.sh", &format!("{content}\necho \"\"\nsync\necho \"\"")).await?;

    let mut reserved_variables = get_reserved_variables(job, &token, db).await?;
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

    let child = if !*DISABLE_NSJAIL {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_BASH_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount),
        )
        .await?;
        let mut cmd_args = vec!["--config", "run.config.proto", "--", "/bin/bash", "main.sh"];
        cmd_args.extend(args);
        Command::new(NSJAIL_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        let mut cmd_args = vec!["main.sh"];
        cmd_args.extend(&args);
        Command::new("/bin/bash")
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };
    handle_child(&job.id, db, logs,  child, !*DISABLE_NSJAIL, worker_name, &job.workspace_id).await?;
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

fn get_common_deno_proc_envs(token: &str, base_internal_url: &str) -> HashMap<String, String> {
    let hostname_base = BASE_URL.split("://").last().unwrap_or("localhost");
    let hostname_internal = base_internal_url.split("://").last().unwrap_or("localhost");
    let deno_auth_tokens_base = DENO_AUTH_TOKENS.as_str();
    let deno_auth_tokens =
        format!("{token}@{hostname_base};{token}@{hostname_internal}{deno_auth_tokens_base}",);

    let mut deno_envs: HashMap<String, String> = HashMap::from([
        (String::from("PATH"), PATH_ENV.clone()),
        (String::from("DENO_AUTH_TOKENS"), deno_auth_tokens),
        (String::from("BASE_INTERNAL_URL"), base_internal_url.to_string()),
        (String::from("NO_COLOR"), String::from("true")),
    ]);

    if let Some(ref s) = *NPM_CONFIG_REGISTRY {
        deno_envs.insert(String::from("NPM_CONFIG_REGISTRY"), s.clone());
    }
    return deno_envs;
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_deno_job(
    logs: &mut String,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClient,
    token: String,
    job_dir: &str,
    inner_content: &String,
    base_internal_url: &str,
    worker_name: &str
) -> error::Result<serde_json::Value> {
    // let mut start = Instant::now();
    logs.push_str("\n\n--- DENO CODE EXECUTION ---\n");
    set_logs(logs, &job.id, db).await;
    
    // logs.push_str(format!("st: {:?}\n", start.elapsed().as_millis()).as_str());
    // start = Instant::now();
    let _ = write_file(job_dir, "main.ts", inner_content).await?;

    let sig = trace_span!("parse_deno_signature")
        .in_scope(|| windmill_parser_ts::parse_deno_signature(inner_content, true))?;

    create_args_and_out_file(client, job, job_dir).await?;

    let spread = sig.args.into_iter().map(|x| x.name).join(",");
    let wrapper_content: String = format!(
        r#"
import {{ main }} from "./main.ts";

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
    let w_id = job.workspace_id.clone();
    let script_path_split = job.script_path().split("/");
    let script_path_parts_len = script_path_split.clone().count();
    let mut relative_mounts = "".to_string();
    for c in 0..script_path_parts_len {
        relative_mounts += ",\n          ";
        relative_mounts += &format!("\"./{}\": \"{base_internal_url}/api/w/{w_id}/scripts/raw/p/{}{}\"",
            (0..c).map(|_| "../").join(""),
            &script_path_split.clone().take(script_path_parts_len - c - 1).join("/"),
            if c == script_path_parts_len - 1 { "" } else { "/" },
        );
    }
    write_file(job_dir, "wrapper.ts", &wrapper_content).await?;
    let import_map = format!(
        r#"{{
        "imports": {{
          "/": "{base_internal_url}/api/w/{w_id}/scripts/raw/p/",
          "./wrapper.ts": "./wrapper.ts",
          "./main.ts": "./main.ts"{relative_mounts}
        }}
      }}"#,
    );
    write_file(job_dir, "import_map.json", &import_map).await?;
    let mut reserved_variables = get_reserved_variables(job, &token, db).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

    let common_deno_proc_envs = get_common_deno_proc_envs(&token, base_internal_url);
    //do not cache local dependencies
    let reload = format!("--reload={base_internal_url}");
    let child = async {
            let mut args = Vec::new();
            let script_path = format!("{job_dir}/wrapper.ts");
            let import_map_path = format!("{job_dir}/import_map.json");
            args.push("run");
            args.push("--import-map");
            args.push(&import_map_path);
            args.push(&reload);
            args.push("--unstable");
            if let Some(deno_flags) = DENO_FLAGS.as_ref() {
                for flag in deno_flags {
                    args.push(flag);
                }
            } else if !*DISABLE_NSJAIL {
                args.push("--allow-net");
                args.push("--allow-read=./");
                args.push("--allow-write=./");
                args.push("--allow-env");
            } else {
                args.push("-A");
            }
            args.push(&script_path);
            Command::new(DENO_PATH.as_str())
                .current_dir(job_dir)
                .env_clear()
                .envs(reserved_variables)
                .envs(common_deno_proc_envs)
                .env("DENO_DIR", DENO_CACHE_DIR)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
    }
    .instrument(trace_span!("create_deno_jail"))
    .await?;
    // logs.push_str(format!("prepare: {:?}\n", start.elapsed().as_millis()).as_str());
    // start = Instant::now();
    handle_child(&job.id, db, logs, child, false, worker_name, &job.workspace_id).await?;
    // logs.push_str(format!("execute: {:?}\n", start.elapsed().as_millis()).as_str());
    // start = Instant::now();
    let r = read_result(job_dir).await;
    // logs.push_str(format!("rr: {:?}\n", start.elapsed().as_millis()).as_str());
    r
}

#[tracing::instrument(level = "trace", skip_all)]
async fn create_args_and_out_file(
    client: &AuthedClient,
    job: &QueuedJob,
    job_dir: &str,
) -> Result<(), Error> {
    let args = if let Some(args) = &job.args {
        Some(transform_json_value("args", client, &job.workspace_id, args.clone()).await?)
    } else {
        None
    };
    let ser_args = serde_json::to_string(&args).map_err(|e| Error::ExecutionErr(e.to_string()))?;
    write_file(job_dir, "args.json", &ser_args).await?;
    write_file(job_dir, "result.json", "").await?;
    Ok(())
}

lazy_static! {
    static ref RELATIVE_IMPORT_REGEX: Regex = Regex::new(r#"(import|from)\s(((u|f)\.)|\.)"#).unwrap();
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_python_job(
    requirements_o: Option<String>,
    job_dir: &str,
    worker_dir: &str,
    worker_name: &str,
    job: &QueuedJob,
    logs: &mut String,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClient,
    token: String,
    inner_content: &String,
    shared_mount: &str,
    base_internal_url: &str
) -> error::Result<serde_json::Value> {
    create_dependencies_dir(job_dir).await;

    let mut additional_python_paths: Vec<String> =
        ADDITIONAL_PYTHON_PATHS.to_owned().unwrap_or_else(|| vec![]);

    let requirements = match requirements_o {
        Some(r) => r,
        None => {
            let requirements = windmill_parser_py::parse_python_imports(&inner_content)?.join("\n");
            if requirements.is_empty() {
                "".to_string()
            } else {
                pip_compile(&job.id, &requirements, logs, job_dir, db, worker_name, &job.workspace_id)
                    .await
                    .map_err(|e| {
                        Error::ExecutionErr(format!("pip compile failed: {}", e.to_string()))
                    })?
            }
        }
    };

    if requirements.len() > 0 {
        if !*DISABLE_NSJAIL {
            let _ = write_file(
                job_dir,
                "download.config.proto",
                &NSJAIL_CONFIG_DOWNLOAD_PY_CONTENT
                    .replace("{WORKER_DIR}", &worker_dir)
                    .replace("{CACHE_DIR}", PIP_CACHE_DIR)
                    .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string()),
            )
            .await?;
        }

        additional_python_paths = handle_python_reqs(
            requirements
                .split("\n")
                .filter(|x| !x.starts_with("--"))
                .collect(),
            job,
            logs,
            db,
            worker_name,
            job_dir,
        )
        .await?;
    }
    logs.push_str("\n\n--- PYTHON CODE EXECUTION ---\n");

    set_logs(logs, &job.id, db).await;

    let relative_imports = RELATIVE_IMPORT_REGEX.is_match(&inner_content);

    let script_path_splitted = &job.script_path().split("/");
    let dirs_full = script_path_splitted.clone().take(script_path_splitted.clone().count() - 1).join("/").replace("-", "_");
    let dirs = if dirs_full.len() > 0 { dirs_full } else { "tmp".to_string() };
    let last = script_path_splitted.clone().last().unwrap().replace("-", "_").replace(" ", "_").to_lowercase();
    let module_dir = format!("{}/{}", job_dir, dirs );
    tokio::fs::create_dir_all(format!("{module_dir}/")).await?;
    let _ = write_file(&module_dir, &format!("{last}.py"), inner_content).await?;
    if relative_imports {
        let _ = write_file(&job_dir, "loader.py", RELATIVE_PYTHON_LOADER).await?;
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
    let spread = if sig.star_kwargs {
        "args = kwargs".to_string()
    } else {
        sig.args
            .into_iter()
            .map(|x| format!("args[\"{}\"] = kwargs.get(\"{}\")", x.name, x.name))
            .join("\n")
    };

    let module_dir_dot = dirs.replace("/", ".").replace("-", "_");
    let wrapper_content: String = format!(
        r#"
import json
{import_loader}
{import_base64}
{import_datetime}
import traceback
import sys
from {module_dir_dot} import {last} as inner_script


with open("args.json") as f:
    kwargs = json.load(f, strict=False)
args = {{}}
{spread}
{transforms}
for k, v in list(args.items()):
    if v == '<function call>':
        del args[k]

try:
    res = inner_script.main(**args)
    typ = type(res)
    if typ.__name__ == 'DataFrame':
        if typ.__module__ == 'pandas.core.frame':
            res = res.values.tolist()
        elif typ.__module__ == 'polars.dataframe.frame':
            res = res.rows()
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
    write_file(job_dir, "wrapper.py", &wrapper_content).await?;

    let mut reserved_variables = get_reserved_variables(job, &token, db).await?;
    let additional_python_paths_folders = additional_python_paths.iter().join(":");
    if !*DISABLE_NSJAIL {
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
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount)
                .replace("{SHARED_DEPENDENCIES}", shared_deps.as_str())
                .replace("{MAIN}", format!("{dirs}/{last}").as_str())
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
    let child = if !*DISABLE_NSJAIL {
        Command::new(NSJAIL_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            // inject PYTHONPATH here - for some reason I had to do it in nsjail conf
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec![
                "--config",
                "run.config.proto",
                "--",
                PYTHON_PATH.as_str(),
                "-u",
                "-m",
                "wrapper",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        Command::new(PYTHON_PATH.as_str())
            .current_dir(job_dir)
            .env_clear()
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec!["-u", "-m", "wrapper"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };

    handle_child(&job.id, db, logs, child, !*DISABLE_NSJAIL, worker_name, &job.workspace_id).await?;
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
    if *CLOUD_HOSTED && content.len() > MAX_RESULT_SIZE {
        return Err(Error::ExecutionErr("Result is too large for the cloud app (limit 2MB). 
        If using this script as part of the flow, use the shared folder to pass heavy data between steps.".to_owned()));
    }
    serde_json::from_str(&content)
        .map_err(|e| Error::ExecutionErr(format!("Error parsing result: {e}")))
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_dependency_job(
    job: &QueuedJob,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
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
        worker_name,
        &job.workspace_id,
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
    worker_name: &str,
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
            worker_name,
            &job.workspace_id,
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

// #[cfg(not(feature = "deno-lock"))]
// async fn generate_deno_lock(
//     _job_id: &Uuid,
//     _code: &str,
//     _logs: &mut String,
//     _job_dir: &str,
//     _db: &sqlx::Pool<sqlx::Postgres>,
//     _timeout: i32,
// ) -> error::Result<String> {
//     Ok(String::new())
// }

// #[cfg(feature = "deno-lock")]
// async fn generate_deno_lock(
//     job_id: &Uuid,
//     code: &str,
//     logs: &mut String,
//     job_dir: &str,
//     db: &sqlx::Pool<sqlx::Postgres>,
//     timeout: i32,
// ) -> error::Result<String> {
//     let _ = write_file(job_dir, "main.ts", code).await?;

//     let child = Command::new(deno_path)
//         .current_dir(job_dir)
//         .args(vec![
//             "cache",
//             "--unstable",
//             "--lock=lock.json",
//             "--lock-write",
//             "main.ts",
//         ])
//         .env("NO_COLOR", "1")
//         .stdout(Stdio::piped())
//         .stderr(Stdio::piped())
//         .spawn()?;

//     handle_child(job_id, db, logs, timeout, child).await?;

//     let path_lock = format!("{job_dir}/lock.json");
//     let mut file = File::open(path_lock).await?;
//     let mut req_content = "".to_string();
//     file.read_to_string(&mut req_content).await?;
//     Ok(req_content)
// }

async fn capture_dependency_job(
    job_id: &Uuid,
    job_language: &ScriptLang,
    job_raw_code: &str,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    w_id: &str,
) -> error::Result<String> {
    match job_language {
        ScriptLang::Python3 => {
            create_dependencies_dir(job_dir).await;
            pip_compile(job_id, job_raw_code, logs, job_dir, db, worker_name, w_id).await
        }
        ScriptLang::Go => {
            install_go_dependencies(
                job_id,
                job_raw_code,
                logs,
                job_dir,
                db,
                false,
                false,
                false,
                worker_name,
                w_id
            )
            .await
        }
        ScriptLang::Deno => {
            Ok(String::new())
            // generate_deno_lock(job_id, job_raw_code, logs, job_dir, db, timeout).await
        }
        ScriptLang::Bash => Ok("".to_owned()),
    }
}

async fn pip_compile(
    job_id: &Uuid,
    requirements: &str,
    logs: &mut String,
    job_dir: &str,
    db: &Pool<Postgres>,
    worker_name: &str,
    w_id: &str,
) -> error::Result<String> {
    logs.push_str(&format!("\nresolving dependencies..."));
    set_logs(logs, job_id, db).await;
    logs.push_str(&format!("\ncontent of requirements:\n{}", requirements));
    let req_hash = calculate_hash(&requirements) ;
    if let Some(cached) = sqlx::query_scalar!(
        "SELECT lockfile FROM pip_resolution_cache WHERE hash = $1",
        req_hash    
    ).fetch_optional(db).await? {
        logs.push_str(&format!("\nfound cached resolution"));
        return Ok(cached);
    }
    let file = "requirements.in";
    let requirements = if let Some(pip_local_dependencies) = PIP_LOCAL_DEPENDENCIES.as_ref() {
        let deps = pip_local_dependencies.clone();
        requirements
            .lines()
            .filter(|s| !deps.contains(&s.to_string()))
            .join("\n")
    } else {
        requirements.to_string()
    };
    write_file(job_dir, file, &requirements).await?;

    let mut args = vec!["-q", "--no-header", file, "--resolver=backtracking"];
    if let Some(url) = PIP_EXTRA_INDEX_URL.as_ref() {
        args.extend(["--extra-index-url", url]);
    }
    if let Some(url) = PIP_INDEX_URL.as_ref() {
        args.extend(["--index-url", url]);
    }
    if let Some(host) = PIP_TRUSTED_HOST.as_ref() {
        args.extend(["--trusted-host", host]);
    }
    let child = Command::new("pip-compile")
        .current_dir(job_dir)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    handle_child(job_id, db, logs,  child, false, worker_name, &w_id)
        .await
        .map_err(|e| Error::ExecutionErr(format!("Lock file generation failed: {e:?}")))?;
    let path_lock = format!("{job_dir}/requirements.txt");
    let mut file = File::open(path_lock).await?;
    let mut req_content = "".to_string();
    file.read_to_string(&mut req_content).await?;
    let lockfile = req_content
        .lines()
        .filter(|x| !x.trim_start().starts_with('#'))
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join("\n");
    sqlx::query!(
        "INSERT INTO pip_resolution_cache (hash, lockfile, expiration) VALUES ($1, $2, now() + ('3 days')::interval) ON CONFLICT (hash) DO UPDATE SET lockfile = $2",
        req_hash,
        lockfile
    ).fetch_optional(db).await?;
    Ok(lockfile)
}

async fn install_go_dependencies(
    job_id: &Uuid,
    code: &str,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    non_dep_job: bool,
    skip_go_mod: bool,
    has_sum: bool,
    worker_name: &str,
    w_id: &str
) -> error::Result<String> {
    if !skip_go_mod {
        gen_go_mymod(code, job_dir).await?;
        let child = Command::new("go")
            .current_dir(job_dir)
            .args(vec!["mod", "init", "mymod"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        handle_child(job_id, db, logs,   child, false, worker_name, w_id).await?;
    }

    let mut new_lockfile = false;

    let hash = if !has_sum {
        calculate_hash(parse_go_imports(&code)?.iter().join("\n").as_str())
    } else {
        "".to_string()
    };

    let mut skip_tidy = has_sum;

    if !has_sum {
        if let Some(cached) = sqlx::query_scalar!(
            "SELECT lockfile FROM pip_resolution_cache WHERE hash = $1",
            hash
        ).fetch_optional(db).await? {
            logs.push_str(&format!("\nfound cached resolution"));
            gen_go_mod(code, job_dir, &cached).await?;
            skip_tidy = true;
            new_lockfile = false;
        } else {
            new_lockfile = true;
        }
    }
    let mod_command = if skip_tidy {
        "download"
    } else {
        "tidy"
    };
    let child = Command::new(GO_PATH.as_str())
        .current_dir(job_dir)
        .env("GOPATH", GO_CACHE_DIR)
        .args(vec!["mod", mod_command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    handle_child(job_id, db, logs,  child, false, worker_name, &w_id)
        .await
        .map_err(|e| Error::ExecutionErr(format!("Lock file generation failed: {e:?}")))?;

    if (!new_lockfile || has_sum) && non_dep_job{
        return Ok("".to_string());
    }


    let mut req_content = "".to_string();

    let mut file = File::open(format!("{job_dir}/go.mod")).await?;
    file.read_to_string(&mut req_content).await?;
    req_content.push_str(GO_REQ_SPLITTER);
    let mut file = File::open(format!("{job_dir}/go.sum")).await?;
    file.read_to_string(&mut req_content).await?;
    
    if non_dep_job {
        sqlx::query!(
            "INSERT INTO pip_resolution_cache (hash, lockfile, expiration) VALUES ($1, $2, now() + ('3 days')::interval) ON CONFLICT (hash) DO UPDATE SET lockfile = $2",
            hash,
            req_content
        ).fetch_optional(db).await?;

        return Ok(String::new())
    } else {
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

async fn get_mem_peak(pid: Option<u32>, nsjail: bool) -> i32 {
    if pid.is_none() {
        return -1
    }
    let pid = if nsjail {
        // This is a bit hacky, but the process id of the nsjail process is the pid of nsjail + 1. 
        // Ideally, we would get the number from fork() itself. This works in MOST cases.
        pid.unwrap() + 1
    } else {
        pid.unwrap()
    };
    
    if let Ok(file) = File::open(format!("/proc/{}/status", pid)).await {
        let mut lines = BufReader::new(file).lines();
        while let Some(line) = lines.next_line().await.unwrap_or(None) {
            if line.starts_with("VmHWM:") {
                return line.split_whitespace().nth(1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(-1);
            };
        }
        -2
    } else {
        -3
    }
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
    mut child: Child,
    nsjail: bool,
    worker_name: &str,
    _w_id: &str,
) -> error::Result<()> {
    let update_job_interval = Duration::from_millis(500);
    let write_logs_delay = Duration::from_millis(500);

    let pid = child.id();
    #[cfg(target_os = "linux")]
    if let Some(pid) = pid {
        //set the highest oom priority
        let mut file = File::create(format!("/proc/{pid}/oom_score_adj")).await?;
        let _ = file.write_all(b"1000").await;
    } else {
        tracing::info!("could not get child pid");
    }
    let (set_too_many_logs, mut too_many_logs) = watch::channel::<bool>(false);
    let (tx, mut rx) = broadcast::channel::<()>(3);
    let mut rx2 = tx.subscribe();


    let output = child_joined_output_stream(&mut child);

    let job_id = job_id.clone();


    /* the cancellation future is polled on by `wait_on_child` while
     * waiting for the child to exit normally */
    let update_job = async {
        let db = db.clone();

        let mut interval = interval(update_job_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let mut i = 1;
        loop {
            tokio::select!(
                _ = rx.recv() => break,
                _ = interval.tick() => {
                    // update the last_ping column every 5 seconds
                    i+=1;
                    if i % 10 == 0 {
                        sqlx::query!(
                            "UPDATE worker_ping SET ping_at = now() WHERE worker = $1",
                            &worker_name
                        )
                        .execute(&db)
                        .await
                        .expect("update worker ping");
                    }
                    let mem_peak = get_mem_peak(pid, nsjail).await;
                    tracing::info!("{job_id} still running. mem peak: {}kB", mem_peak);
                    let mem_peak = if mem_peak > 0 { Some(mem_peak) } else { None };
                    if sqlx::query_scalar!("UPDATE queue SET mem_peak = GREATEST($1, mem_peak), last_ping = now() WHERE id = $2 RETURNING canceled", mem_peak, job_id)
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

        #[cfg(not(feature = "enterprise"))]
        let timeout_duration = *TIMEOUT_DURATION;

        #[cfg(feature = "enterprise")]
        let premium_workspace = *CLOUD_HOSTED && sqlx::query_scalar!("SELECT premium FROM workspace WHERE id = $1", _w_id)
            .fetch_one(&db)
            .await
            .map_err(|e| {
                tracing::error!(%e, "error getting premium workspace for job {job_id}: {e}");
            }).unwrap_or(false);
        
        #[cfg(feature = "enterprise")]
        let timeout_duration = if premium_workspace {
            *TIMEOUT_DURATION*6 //30mins
        } else {
            *TIMEOUT_DURATION
        };

        let kill_reason = tokio::select! {
            biased;
            result = child.wait() => return result.map(Ok),
            Ok(()) = too_many_logs.changed() => KillReason::TooManyLogs,
            _ = sleep(timeout_duration) => KillReason::Timeout,
            _ = update_job => KillReason::Cancelled,
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
                .bind(format!("duration > {}", TIMEOUT_DURATION.as_secs()))
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
        let max_log_size = if *CLOUD_HOSTED {
            MAX_RESULT_SIZE
        } else {
            usize::MAX
        };
        /* log_remaining is zero when output limit was reached */
        let mut log_remaining = max_log_size.saturating_sub(logs.chars().count());
        let mut result = io::Result::Ok(());
        let mut output = output.take_until(rx2.recv()).boxed();
        /* `do_write` resolves the task, but does not contain the Result.
         * It's useful to know if the task completed. */
        let (mut do_write, mut write_result) = tokio::spawn(ready(())).remote_handle();

        while let Some(line) =  output.by_ref().next().await {

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

            (do_write, write_result) = tokio::spawn(append_logs(job_id, joined, db.clone())).remote_handle();

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


    let (wait_result, _) = tokio::join!(wait_on_child, lines);

    match wait_result {
        _ if *too_many_logs.borrow() => Err(Error::ExecutionErr(format!(
            "logs or result reached limit. (current max size: {MAX_RESULT_SIZE} characters)"
        ))),
        Ok(Ok(status)) => {
            if status.success() {
                Ok(())
            } else if let Some(code) = status.code() {
                Err(error::Error::ExitStatus(code))
            } else {
                Err(error::Error::ExecutionErr(format!(
                    "process terminated by signal: {:#?}, stopped_signal: {:#?}, core_dumped: {}",
                    status.signal(),
                    status.stopped_signal(),
                    status.core_dumped()
                )))
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
    mut rx: tokio::sync::broadcast::Receiver<()>,
    base_internal_url: &str,
) {
    loop {
        handle_zombie_jobs(db, base_internal_url).await;

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(30))    => (),
            _ = rx.recv() => {
                    println!("received killpill for monitor job");
                    break;
            }
        }
    }
}

async fn handle_zombie_jobs(db: &Pool<Postgres>, base_internal_url: &str) {
    if *RESTART_ZOMBIE_JOBS {
        let restarted = sqlx::query!(
                "UPDATE queue SET running = false, started_at = null, logs = logs || '\nRestarted job after not receiving job''s ping for too long the ' || now() || '\n\n' WHERE last_ping < now() - ($1 || ' seconds')::interval AND running = true AND job_kind != $2 AND job_kind != $3 AND same_worker = false RETURNING id, workspace_id, last_ping",
                *ZOMBIE_JOB_TIMEOUT,
                JobKind::Flow: JobKind,
                JobKind::FlowPreview: JobKind,
            )
            .fetch_all(db)
            .await
            .ok()
            .unwrap_or_else(|| vec![]);

        if *METRICS_ENABLED {
            QUEUE_ZOMBIE_RESTART_COUNT.inc_by(restarted.len() as _);
        }
        for r in restarted {
            tracing::info!(
                "restarted zombie job {} {} {}",
                r.id,
                r.workspace_id,
                r.last_ping
            );
        }
    }

    let mut timeout_query = "SELECT * FROM queue WHERE last_ping < now() - ($1 || ' seconds')::interval AND running = true AND job_kind != $2".to_string();
    if *RESTART_ZOMBIE_JOBS  {
        timeout_query.push_str(" AND same_worker = true");
    };
    let timeouts = sqlx::query_as::<_, QueuedJob>(
            &timeout_query
        )
        .bind(ZOMBIE_JOB_TIMEOUT.as_str())
        .bind(JobKind::Flow)
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![]);

    if *METRICS_ENABLED {
        QUEUE_ZOMBIE_DELETE_COUNT.inc_by(timeouts.len() as _);
    }
    for job in timeouts {
        tracing::info!(
            "timedout zombie job {} {}",
            job.id,
            job.workspace_id,
        );

        // since the job is unrecoverable, the same worker queue should never be sent anything
        let (same_worker_tx_never_used, _same_worker_rx_never_used) = mpsc::channel::<Uuid>(1);

        let token = create_token_for_owner(
            &db,
            &job.workspace_id,
            &job.permissioned_as,
            "ephemeral-zombie-jobs",
            *SESSION_TOKEN_EXPIRY,
            &job.email,
        )
        .await
        .expect("could not create job token");

        let client = AuthedClient { base_internal_url: base_internal_url.to_string(), token: token.clone(), workspace: job.workspace_id.to_string(), client: OnceCell::new() };

        let last_ping = job.last_ping.clone();
        let _ = handle_job_error(
            db,
            &client,
            job,
            error::Error::ExecutionErr(format!("Job timed out after no ping from job since {} (ZOMBIE_JOB_TIMEOUT: {})",
                last_ping.map(|x| x.to_string()).unwrap_or_else(|| "no ping".to_string()), *ZOMBIE_JOB_TIMEOUT)),
            None,
            true,
            same_worker_tx_never_used,
            "",
            base_internal_url,
        )
        .await;
    }
}

async fn handle_python_reqs(
    requirements: Vec<&str>,
    job: &QueuedJob,
    logs: &mut String,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    job_dir: &str,
) -> error::Result<Vec<String>> {
    let mut req_paths: Vec<String> = vec![];
    let mut vars = vec![("PATH", PATH_ENV.as_str())];
    if !*DISABLE_NSJAIL {
        if let Some(url) = PIP_EXTRA_INDEX_URL.as_ref() {
            vars.push(("EXTRA_INDEX_URL", url));
        }
        if let Some(url) = PIP_INDEX_URL.as_ref() {
            vars.push(("INDEX_URL", url));
        }
        if let Some(host) = PIP_TRUSTED_HOST.as_ref() {
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

        let child = if !*DISABLE_NSJAIL {
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
            Command::new(NSJAIL_PATH.as_str())
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
            if let Some(url) = PIP_EXTRA_INDEX_URL.as_ref() {
                args.extend(["--extra-index-url", url]);
            }
            if let Some(url) = PIP_INDEX_URL.as_ref() {
                args.extend(["--index-url", url]);
            }
            if let Some(host) = PIP_TRUSTED_HOST.as_ref() {
                args.extend(["--trusted-host", &host]);
            }
            Command::new(PYTHON_PATH.as_str())
                .env_clear()
                .env("PATH", PATH_ENV.as_str())
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
        };

        let child = handle_child(&job.id, db, logs, child, false, worker_name, &job.workspace_id).await;
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
