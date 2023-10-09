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
use once_cell::sync::OnceCell;
use prometheus::core::{AtomicU64, GenericCounter};
#[cfg(feature = "benchmark")]
use serde::Serialize;
use sqlx::{Pool, Postgres};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use windmill_api_client::Client;

use uuid::Uuid;
use windmill_common::{
    error::{self, to_anyhow, Error},
    flows::{FlowModule, FlowModuleValue, FlowValue},
    jobs::{JobKind, Metrics, QueuedJob},
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang},
    users::SUPERADMIN_SECRET_EMAIL,
    utils::{rd_string, StripPath},
    worker::{update_ping, CLOUD_HOSTED, WORKER_CONFIG},
    DB, IS_READY, METRICS_ENABLED,
};
use windmill_queue::{
    canceled_job_to_result, get_queued_job, pull, push, PushIsolationLevel, HTTP_CLIENT,
};

use serde_json::{json, Value};

use tokio::{
    fs::{symlink, DirBuilder},
    sync::{
        mpsc::{self, Sender},
        Barrier, RwLock,
    },
    task::JoinHandle,
    time::Instant,
};

use futures::future::FutureExt;

use async_recursion::async_recursion;

use rand::Rng;

#[cfg(feature = "enterprise")]
use crate::global_cache::{
    cache_global, copy_all_piptars_from_bucket, copy_cache_to_tmp_cache,
    copy_denogo_cache_from_bucket_as_tar, copy_tmp_cache_to_cache,
};

#[cfg(feature = "enterprise")]
use crate::bun_executor::start_worker;

use windmill_queue::{add_completed_job, add_completed_job_error};

use crate::{
    bash_executor::{handle_bash_job, handle_powershell_job, ANSI_ESCAPE_RE},
    bun_executor::{gen_lockfile, get_trusted_deps, handle_bun_job},
    common::{hash_args, read_result, save_in_cache, transform_json_value, write_file},
    deno_executor::{generate_deno_lock, handle_deno_job},
    go_executor::{handle_go_job, install_go_dependencies},
    graphql_executor::do_graphql,
    js_eval::{eval_fetch_timeout, transpile_ts},
    mysql_executor::do_mysql,
    pg_executor::do_postgresql,
    python_executor::{
        create_dependencies_dir, handle_python_job, handle_python_reqs, pip_compile,
    },
    worker_flow::{
        handle_flow, update_flow_status_after_job_completion, update_flow_status_in_progress,
    },
};

#[cfg(feature = "enterprise")]
use crate::{bigquery_executor::do_bigquery, snowflake_executor::do_snowflake};

pub async fn create_token_for_owner_in_bg(
    db: &Pool<Postgres>,
    job: &QueuedJob,
) -> Arc<RwLock<String>> {
    let rw_lock = Arc::new(RwLock::new(String::new()));
    // skipping test runs
    if job.workspace_id != "" {
        let mut locked = rw_lock.clone().write_owned().await;
        let db = db.clone();
        let job = job.clone();
        tokio::spawn(async move {
            let job = job.clone();
            let token = create_token_for_owner(
                &db.clone(),
                &job.workspace_id,
                &job.permissioned_as,
                "ephemeral-script",
                *SCRIPT_TOKEN_EXPIRY,
                &job.email,
            )
            .await
            .expect("could not create job token");
            *locked = token;
        });
    };
    return rw_lock;
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
    let is_super_admin =
        sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
            .fetch_optional(db)
            .await?
            .unwrap_or(false)
            || email == SUPERADMIN_SECRET_EMAIL;

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

pub const TMP_DIR: &str = "/tmp/windmill";
pub const ROOT_CACHE_DIR: &str = "/tmp/windmill/cache/";
pub const ROOT_TMP_CACHE_DIR: &str = "/tmp/windmill/tmpcache/";
pub const LOCK_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "lock");
pub const PIP_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "pip");
pub const DENO_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "deno");
pub const DENO_CACHE_DIR_DEPS: &str = concatcp!(ROOT_CACHE_DIR, "deno/deps");
pub const DENO_CACHE_DIR_NPM: &str = concatcp!(ROOT_CACHE_DIR, "deno/npm");

pub const GO_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "go");
pub const BUN_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "bun");
pub const HUB_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "hub");
pub const GO_BIN_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "gobin");

pub const TAR_PIP_TMP_CACHE_DIR: &str = concatcp!(ROOT_TMP_CACHE_DIR, "tar/pip");
pub const DENO_TMP_CACHE_DIR: &str = concatcp!(ROOT_TMP_CACHE_DIR, "deno");
pub const BUN_TMP_CACHE_DIR: &str = concatcp!(ROOT_TMP_CACHE_DIR, "bun");
pub const GO_TMP_CACHE_DIR: &str = concatcp!(ROOT_TMP_CACHE_DIR, "go");
pub const HUB_TMP_CACHE_DIR: &str = concatcp!(ROOT_TMP_CACHE_DIR, "hub");
pub const DENO_TMP_CACHE_DIR_DEPS: &str = concatcp!(ROOT_TMP_CACHE_DIR, "deno/deps");
pub const DENO_TMP_CACHE_DIR_NPM: &str = concatcp!(ROOT_TMP_CACHE_DIR, "deno/npm");
const NUM_SECS_PING: u64 = 5;

const INCLUDE_DEPS_PY_SH_CONTENT: &str = include_str!("../nsjail/download_deps.py.sh");

pub const DEFAULT_CLOUD_TIMEOUT: u64 = 900;
pub const DEFAULT_SELFHOSTED_TIMEOUT: u64 = 604800; // 7 days
pub const DEFAULT_SLEEP_QUEUE: u64 = 50;

// only 1 native job so that we don't have to worry about concurrency issues on non dedicated native jobs workers
pub const DEFAULT_NATIVE_JOBS: usize = 1;

const VACUUM_PERIOD: u32 = 10000;

pub const MAX_BUFFERED_DEDICATED_JOBS: usize = 3;

lazy_static::lazy_static! {

    static ref SLEEP_QUEUE: u64 = std::env::var("SLEEP_QUEUE")
    .ok()
    .and_then(|x| x.parse::<u64>().ok())
    .unwrap_or(DEFAULT_SLEEP_QUEUE);


    pub static ref DISABLE_NUSER: bool = std::env::var("DISABLE_NUSER")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(false);

    pub static ref DISABLE_NSJAIL: bool = std::env::var("DISABLE_NSJAIL")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(true);

    pub static ref KEEP_JOB_DIR: bool = std::env::var("KEEP_JOB_DIR")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(false);

    pub static ref NO_PROXY: Option<String> = std::env::var("no_proxy").ok().or(std::env::var("NO_PROXY").ok());
    pub static ref HTTP_PROXY: Option<String> = std::env::var("http_proxy").ok().or(std::env::var("HTTP_PROXY").ok());
    pub static ref HTTPS_PROXY: Option<String> = std::env::var("https_proxy").ok().or(std::env::var("HTTPS_PROXY").ok());
    pub static ref DENO_PATH: String = std::env::var("DENO_PATH").unwrap_or_else(|_| "/usr/bin/deno".to_string());
    pub static ref BUN_PATH: String = std::env::var("BUN_PATH").unwrap_or_else(|_| "/usr/bin/bun".to_string());
    pub static ref NSJAIL_PATH: String = std::env::var("NSJAIL_PATH").unwrap_or_else(|_| "nsjail".to_string());
    pub static ref PATH_ENV: String = std::env::var("PATH").unwrap_or_else(|_| String::new());
    pub static ref HOME_ENV: String = std::env::var("HOME").unwrap_or_else(|_| String::new());
    pub static ref TZ_ENV: String = std::env::var("TZ").unwrap_or_else(|_| String::new());
    pub static ref GOPRIVATE: Option<String> = std::env::var("GOPRIVATE").ok();
    pub static ref GOPROXY: Option<String> = std::env::var("GOPROXY").ok();
    pub static ref NETRC: Option<String> = std::env::var("NETRC").ok();


    pub static ref NPM_CONFIG_REGISTRY: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref PIP_EXTRA_INDEX_URL: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));



    pub static ref WHITELIST_ENVS: Option<Vec<(String, String)>> = std::env::var("WHITELIST_ENVS")
        .ok()
        .map(|x| x.split(',').map(|x| (x.to_string(), std::env::var(x).unwrap_or("".to_string()))).collect());

    pub static ref TAR_CACHE_RATE: i32 = std::env::var("TAR_CACHE_RATE")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(100);

    static ref WORKER_STARTED: prometheus::IntGauge = prometheus::register_int_gauge!(
        "worker_started",
        "Total number of workers started."
    )
    .unwrap();

    static ref WORKER_UPTIME_OPTS: prometheus::Opts = prometheus::opts!(
        "worker_uptime",
        "Total number of seconds since the worker has started"
    );

    static ref TIMEOUT: u64 = std::env::var("TIMEOUT")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or_else(|| if *CLOUD_HOSTED { DEFAULT_CLOUD_TIMEOUT } else { DEFAULT_SELFHOSTED_TIMEOUT });

    pub static ref MAX_WAIT_FOR_SIGTERM: u64 = std::env::var("MAX_WAIT_FOR_SIGTERM")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or_else(|| 5);

    pub static ref TIMEOUT_DURATION: Duration = Duration::from_secs(*TIMEOUT);

    pub static ref SCRIPT_TOKEN_EXPIRY: i32 = std::env::var("SCRIPT_TOKEN_EXPIRY")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(900);

    pub static ref GLOBAL_CACHE_INTERVAL: u64 = std::env::var("GLOBAL_CACHE_INTERVAL")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or(60 * 10);

    pub static ref S3_CACHE_BUCKET: Option<String> = std::env::var("S3_CACHE_BUCKET")
        .ok()
        .map(|e| Some(e))
        .unwrap_or(None);


    pub static ref EXIT_AFTER_NO_JOB_FOR_SECS: Option<u64> = std::env::var("EXIT_AFTER_NO_JOB_FOR_SECS")
        .ok()
        .and_then(|x| x.parse::<u64>().ok());

    pub static ref CAN_PULL: Arc<RwLock<()>> = Arc::new(RwLock::new(()));


}
//only matter if CLOUD_HOSTED
pub const MAX_RESULT_SIZE: usize = 1024 * 1024 * 2; // 2MB

pub struct AuthedClientBackgroundTask {
    pub base_internal_url: String,
    pub workspace: String,
    pub token: Arc<RwLock<String>>,
    pub client: OnceCell<Client>,
}

impl AuthedClientBackgroundTask {
    pub async fn get_authed(&self) -> AuthedClient {
        return AuthedClient {
            base_internal_url: self.base_internal_url.clone(),
            workspace: self.workspace.clone(),
            token: self.get_token().await,
            client: self.client.clone(),
        };
    }
    pub async fn get_token(&self) -> String {
        return self.token.read().await.clone();
    }
}
#[derive(Clone)]
pub struct AuthedClient {
    pub base_internal_url: String,
    pub workspace: String,
    pub token: String,
    pub client: OnceCell<Client>,
}

impl AuthedClient {
    pub fn get_client(&self) -> &Client {
        return self.client.get_or_init(|| {
            windmill_api_client::create_client(&self.base_internal_url, self.token.clone())
        });
    }
}

#[cfg(feature = "benchmark")]
#[derive(Serialize)]
struct BenchmarkInfo {
    iters: u64,
    timings: Vec<Vec<u32>>,
}

#[macro_export]
macro_rules! add_time {
    ($x:expr, $y:expr, $z:expr) => {
        #[cfg(feature = "benchmark")]
        {
            $x.push($y.elapsed().as_nanos() as u32);
            // println!("{}: {:?}", $z, $y.elapsed());
        }
    };
}

async fn handle_receive_completed_job<
    R: rsmq_async::RsmqConnection + Send + Sync + Clone + 'static,
>(
    jc: JobCompleted,
    worker_execution_failed: HashMap<Option<ScriptLang>, GenericCounter<AtomicU64>>,
    base_internal_url: String,
    db: Pool<Postgres>,
    worker_dir: String,
    same_worker_tx: Sender<Uuid>,
    rsmq: Option<R>,
) {
    let metrics = build_language_metrics(&worker_execution_failed.clone(), &jc.job.language);
    let token = jc.token.clone();
    let workspace = jc.job.workspace_id.clone();
    let client = AuthedClient {
        base_internal_url: base_internal_url.to_string(),
        workspace,
        token,
        client: OnceCell::new(),
    };
    if let Err(err) = process_completed_job(
        &jc,
        &client,
        &db,
        &worker_dir,
        metrics.clone(),
        same_worker_tx.clone(),
        rsmq.clone(),
    )
    .await
    {
        handle_job_error(
            &db,
            &client,
            &jc.job,
            err,
            metrics,
            false,
            same_worker_tx.clone(),
            &worker_dir,
            rsmq.clone(),
        )
        .await;
    }
}

pub async fn run_worker<R: rsmq_async::RsmqConnection + Send + Sync + Clone + 'static>(
    db: &Pool<Postgres>,
    worker_instance: &str,
    worker_name: String,
    i_worker: u64,
    _num_workers: u32,
    ip: &str,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
    killpill_tx: tokio::sync::broadcast::Sender<()>,
    base_internal_url: &str,
    rsmq: Option<R>,
    _sync_barrier: Arc<RwLock<Option<Barrier>>>,
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

    if let Some(ref netrc) = *NETRC {
        tracing::info!("Writing netrc at {}/.netrc", HOME_ENV.as_str());
        write_file(&HOME_ENV, ".netrc", netrc)
            .await
            .expect("could not write netrc");
    }

    DirBuilder::new()
        .recursive(true)
        .create(&worker_dir)
        .await
        .expect("could not create initial worker dir");

    let _ = write_file(
        &worker_dir,
        "download_deps.py.sh",
        INCLUDE_DEPS_PY_SH_CONTENT,
    )
    .await;

    let mut last_ping = Instant::now() - Duration::from_secs(NUM_SECS_PING + 1);

    update_ping(worker_instance, &worker_name, ip, db).await;

    let uptime_metric =
        prometheus::register_counter!(WORKER_UPTIME_OPTS.clone().const_label("name", &worker_name))
            .unwrap();

    let all_langs = [
        None,
        Some(ScriptLang::Python3),
        Some(ScriptLang::Deno),
        Some(ScriptLang::Go),
        Some(ScriptLang::Bash),
        Some(ScriptLang::Powershell),
        Some(ScriptLang::Nativets),
        Some(ScriptLang::Postgresql),
        Some(ScriptLang::Mysql),
        Some(ScriptLang::Bigquery),
        Some(ScriptLang::Snowflake),
        Some(ScriptLang::Graphql),
        Some(ScriptLang::Bun),
    ];

    let worker_execution_duration: HashMap<_, _> = all_langs
        .clone()
        .into_iter()
        .map(|x| {
            (
                x.clone(),
                prometheus::register_histogram!(prometheus::HistogramOpts::new(
                    "worker_execution_duration",
                    "Duration between receiving a job and completing it",
                )
                .const_label("name", &worker_name)
                .const_label("language", x.map(|x| x.as_str()).unwrap_or("none")))
                .expect("register prometheus metric"),
            )
        })
        .collect();

    let worker_execution_duration_counter: HashMap<_, _> = all_langs
        .clone()
        .into_iter()
        .map(|x| {
            (
                x.clone(),
                prometheus::register_counter!(prometheus::Opts::new(
                    "worker_execution_duration_counter",
                    "Total number of seconds spent executing jobs"
                )
                .const_label("name", &worker_name)
                .const_label("language", x.map(|x| x.as_str()).unwrap_or("none")))
                .expect("register prometheus metric"),
            )
        })
        .collect();

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

    let worker_execution_failed: HashMap<_, _> = all_langs
        .clone()
        .into_iter()
        .map(|x| {
            (
                x.clone(),
                prometheus::register_int_counter!(prometheus::Opts::new(
                    "worker_execution_failed",
                    "Number of failed jobs",
                )
                .const_label("name", &worker_name)
                .const_label("language", x.map(|x| x.as_str()).unwrap_or("none")))
                .expect("register prometheus metric"),
            )
        })
        .collect();

    let worker_execution_count: HashMap<_, _> = all_langs
        .into_iter()
        .map(|x| {
            (
                x.clone(),
                prometheus::register_int_counter!(prometheus::Opts::new(
                    "worker_execution_count",
                    "Number of executed jobs"
                )
                .const_label("name", &worker_name)
                .const_label("language", x.map(|x| x.as_str()).unwrap_or("none")))
                .expect("register prometheus metric"),
            )
        })
        .collect();

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

    let (_copy_to_bucket_tx, mut copy_to_bucket_rx) = mpsc::channel::<()>(2);

    #[cfg(feature = "enterprise")]
    let mut copy_cache_from_bucket_handle: Option<tokio::task::JoinHandle<()>> = None;

    #[cfg(feature = "enterprise")]
    let mut last_sync = Instant::now()
        + Duration::from_secs(rand::thread_rng().gen_range(0..*GLOBAL_CACHE_INTERVAL));

    #[cfg(feature = "enterprise")]
    let mut handles = Vec::with_capacity(2);

    #[cfg(feature = "enterprise")]
    if i_worker == 1 {
        if let Some(ref s) = S3_CACHE_BUCKET.clone() {
            if crate::global_cache::worker_s3_bucket_sync_enabled(&db).await {
                let bucket = s.to_string();
                let worker_name2 = worker_name.clone();

                //piptars can be fetched in background
                handles.push(tokio::task::spawn(async move {
                    tracing::info!(worker = %worker_name2, "Started initial piptar sync in background");
                    copy_all_piptars_from_bucket(&bucket).await;
                }));
                //denogocache.tar need to be fetched in foreground, block workers until they fetched it
                copy_denogo_cache_from_bucket_as_tar(s).await;
            }
        }
    }

    let (same_worker_tx, mut same_worker_rx) = mpsc::channel::<Uuid>(5);

    let (job_completed_tx, mut job_completed_rx) = mpsc::channel::<JobCompleted>(100);

    let db2 = db.clone();
    let base_internal_url2 = base_internal_url.to_string();
    let same_worker_tx2 = same_worker_tx.clone();
    let rsmq2 = rsmq.clone();
    let worker_dir2 = worker_dir.clone();
    let worker_execution_failed2 = worker_execution_failed.clone();
    let thread_count = Arc::new(AtomicUsize::new(0));

    let is_dedicated_worker = WORKER_CONFIG.read().await.dedicated_worker.is_some();

    #[cfg(feature = "benchmark")]
    let jobs = 25000;

    #[cfg(feature = "benchmark")]
    {
        if is_dedicated_worker {
            // you need to create the script first, check https://github.com/windmill-labs/windmill/blob/b76a92cfe454c686f005c65f534e29e039f3c706/benchmarks/lib.ts#L47
            let hash = sqlx::query_scalar!(
                "SELECT hash FROM script WHERE path = $1 AND workspace_id = $2",
                "f/benchmarks/dedicated",
                "admins"
            )
            .fetch_one(db)
            .await
            .unwrap_or_else(|_e| panic!("failed to insert dedicated jobs"));
            sqlx::query!("INSERT INTO queue (id, script_hash, script_path, job_kind, language, tag, created_by, permissioned_as, email, scheduled_for, workspace_id) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10 FROM generate_series(1, $11))",
                    hash,
                    "f/benchmarks/dedicated",
                    JobKind::Script as JobKind,
                    ScriptLang::Bun as ScriptLang,
                    "admins:f/benchmarks/dedicated",
                    "admin",
                    "u/admin",
                    "admin@windmill.dev",
                    chrono::Utc::now(),
                    "admins",
                    jobs
                )
                .execute(db)
                .await.unwrap_or_else(|_e| panic!("failed to insert dedicated jobs"));
        } else {
            sqlx::query!("INSERT INTO queue (id, script_hash, script_path, job_kind, language, tag, created_by, permissioned_as, email, scheduled_for, workspace_id) (SELECT gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10 FROM generate_series(1, $11))",
            None::<i64>,
            None::<String>,
            JobKind::Noop as JobKind,
            ScriptLang::Deno as ScriptLang,
            "deno",
            "admin",
            "u/admin",
            "admin@windmill.dev",
            chrono::Utc::now(),
            "admins",
            jobs
        )
        .execute(db)
        .await.unwrap_or_else(|_e| panic!("failed to insert noop jobs"));
        }
    }

    #[cfg(feature = "benchmark")]
    let completed_jobs = Arc::new(AtomicUsize::new(0));
    #[cfg(feature = "benchmark")]
    let start = Instant::now();
    #[cfg(feature = "benchmark")]
    let main_duration = Arc::new(AtomicUsize::new(0));
    #[cfg(feature = "benchmark")]
    let send_duration = Arc::new(AtomicUsize::new(0));
    #[cfg(feature = "benchmark")]
    let process_duration = Arc::new(AtomicUsize::new(0));

    #[cfg(feature = "benchmark")]
    let main_duration2 = main_duration.clone();
    #[cfg(feature = "benchmark")]
    let send_duration2 = send_duration.clone();

    let send_result = tokio::spawn(async move {
        while let Some(jc) = job_completed_rx.recv().await {
            let base_internal_url2 = base_internal_url2.clone();
            let worker_execution_failed2 = worker_execution_failed2.clone();
            let worker_dir2 = worker_dir2.clone();
            let db2 = db2.clone();
            let same_worker_tx2 = same_worker_tx2.clone();
            let rsmq2 = rsmq2.clone();

            if matches!(jc.job.job_kind, JobKind::Noop) || is_dedicated_worker {
                thread_count.fetch_add(1, Ordering::SeqCst);
                let thread_count = thread_count.clone();

                #[cfg(feature = "benchmark")]
                let send_duration = send_duration2.clone();
                #[cfg(feature = "benchmark")]
                let process_duration = process_duration.clone();
                #[cfg(feature = "benchmark")]
                let completed_jobs = completed_jobs.clone();
                #[cfg(feature = "benchmark")]
                let main_duration = main_duration2.clone();

                tokio::spawn(async move {
                    #[cfg(feature = "benchmark")]
                    let process_start = Instant::now();

                    handle_receive_completed_job(
                        jc,
                        worker_execution_failed2,
                        base_internal_url2,
                        db2,
                        worker_dir2,
                        same_worker_tx2,
                        rsmq2,
                    )
                    .await;
                    #[cfg(feature = "benchmark")]
                    {
                        let n = completed_jobs.fetch_add(1, Ordering::SeqCst);
                        if (n + 1) % 1000 == 0 || n == (jobs - 1) as usize {
                            let duration_s = start.elapsed().as_secs_f64();
                            let jobs_per_sec = n as f64 / duration_s;
                            tracing::info!(
                                "completed {} jobs in {}s, {} jobs/s",
                                n + 1,
                                duration_s,
                                jobs_per_sec
                            );

                            tracing::info!(
                                "main loop without send {}s",
                                main_duration.load(Ordering::SeqCst) as f64 / 1000.0
                            );

                            tracing::info!(
                                "send job completed / send dedicated job duration {}s",
                                send_duration.load(Ordering::SeqCst) as f64 / 1000.0
                            );

                            tracing::info!(
                                "job completed process duration {}s",
                                process_duration.load(Ordering::SeqCst) as f64 / 1000.0
                            );
                        }

                        process_duration.fetch_add(
                            process_start.elapsed().as_millis() as usize,
                            Ordering::SeqCst,
                        );
                    }

                    thread_count.fetch_sub(1, Ordering::SeqCst);
                });
            } else {
                handle_receive_completed_job(
                    jc,
                    worker_execution_failed2,
                    base_internal_url2,
                    db2,
                    worker_dir2,
                    same_worker_tx2,
                    rsmq2,
                )
                .await;
            }
        }

        tracing::info!("stopped processing new completed jobs");
        while thread_count.load(Ordering::SeqCst) > 0 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        tracing::info!("finished processing all completed jobs");

        // if let Err(e) =
        //     add_completed_job(&db2, &job, success, false, result, logs, rsmq2.clone()).await
        // {
        //     tracing::error!(worker = %worker_name2, "failed to add completed job: {}", e);
        // }
    });

    let mut last_executed_job: Option<Instant> = None;
    let mut last_checked_suspended = Instant::now();

    #[cfg(feature = "benchmark")]
    let mut started = false;

    #[cfg(feature = "benchmark")]
    let mut infos = BenchmarkInfo { iters: 0, timings: vec![] };

    let vacuum_shift = rand::thread_rng().gen_range(0..VACUUM_PERIOD);

    IS_READY.store(true, Ordering::Relaxed);
    tracing::info!(worker = %worker_name, "listening for jobs, config: {:?}", WORKER_CONFIG.read().await);

    let (dedicated_worker_tx, dedicated_worker_handle) = if let Some(_wp) =
        WORKER_CONFIG.read().await.dedicated_worker.clone()
    {
        #[cfg(not(feature = "enterprise"))]
        {
            tracing::error!("Dedicated worker is an enterprise feature");
            killpill_tx.send(()).expect("send");
            return;
        }

        #[cfg(feature = "enterprise")]
        {
            let (dedicated_worker_tx, dedicated_worker_rx) =
                mpsc::channel::<QueuedJob>(MAX_BUFFERED_DEDICATED_JOBS);
            let mut killpill_rx = killpill_rx.resubscribe();
            let db = db.clone();
            let worker_dir = worker_dir.clone();
            let base_internal_url = base_internal_url.to_string();
            let worker_name = worker_name.clone();
            let job_completed_tx = job_completed_tx.clone();
            let job_dir = format!("{}/dedicated", worker_dir);
            tokio::fs::create_dir_all(&job_dir)
                .await
                .expect("create dir");
            let handle = tokio::spawn(async move {
                let token = rd_string(30);
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
                    killpill_tx.send(()).expect("send");
                };

                let (content, lock, _language, envs) = {
                    let r;
                    loop {
                        let q = sqlx::query_as::<_, (String, Option<String>, Option<ScriptLang>, Option<Vec<String>>)>(
                    "SELECT content, lock, language, envs FROM script WHERE path = $1 AND workspace_id = $2 AND
                    created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND workspace_id = $2 AND
                    deleted = false AND lock IS not NULL AND lock_error_logs IS NULL)",
                )
                .bind(&_wp.path)
                .bind(&_wp.workspace_id)
                .fetch_optional(&db)
                .await;
                        if let Ok(q) = q {
                            if let Some(wp) = q {
                                r = wp;
                                break;
                            } else {
                                tracing::error!(
                                "Failed to fetch script `{}` in workspace {} for dedicated worker. Retrying in 10s.",
                                _wp.path,
                                _wp.workspace_id
                            );
                                tokio::select! {
                                    biased;
                                    _ = killpill_rx.recv() => {
                                        tracing::info!("Killing dedicated worker while it was attempting to fetch script");
                                        return;
                                    }
                                    _ = tokio::time::sleep(Duration::from_secs(10)) => {
                                        continue;
                                    }

                                }
                            }
                        } else {
                            tracing::error!("Failed to fetch script for dedicated worker");
                            killpill_tx.send(()).expect("send");
                            return;
                        }
                    }
                    r
                };

                let worker_envs = build_envs(envs).expect("failed to build envs");
                if let Err(e) = start_worker(
                    lock,
                    &db,
                    &content,
                    &base_internal_url,
                    &job_dir,
                    &worker_name,
                    worker_envs,
                    &_wp.workspace_id,
                    &_wp.path,
                    &token,
                    job_completed_tx,
                    dedicated_worker_rx,
                    killpill_rx,
                )
                .await
                {
                    tracing::error!("error in dedicated worker: {:?}", e)
                }
            });
            (Some(dedicated_worker_tx), Some(handle))
        }
    } else {
        (None, None) as (Option<Sender<QueuedJob>>, Option<JoinHandle<()>>)
    };

    #[cfg(feature = "benchmark")]
    tracing::info!("pre loop time {}s", start.elapsed().as_secs_f64());

    if i_worker == 1 {
        if let Err(e) =
            queue_init_bash_maybe(db, same_worker_tx.clone(), &worker_name, rsmq.clone()).await
        {
            tracing::error!("Error queuing init bash script for worker {worker_name}: {e}");
        }
    }
    loop {
        #[cfg(feature = "benchmark")]
        let loop_start = Instant::now();

        #[cfg(feature = "benchmark")]
        let mut timing = vec![];

        if *METRICS_ENABLED {
            worker_busy.set(0);
            uptime_metric.inc_by(
                ((start_time.elapsed().as_millis() as f64) / 1000.0 - uptime_metric.get())
                    .try_into()
                    .unwrap(),
            );
        }

        #[cfg(feature = "enterprise")]
        let copy_tx = _copy_to_bucket_tx.clone();

        if last_ping.elapsed().as_secs() > NUM_SECS_PING {
            let tags = WORKER_CONFIG.read().await.worker_tags.clone();

            sqlx::query!(
                "UPDATE worker_ping SET ping_at = now(), jobs_executed = $1, custom_tags = $2 WHERE worker = $3",
                jobs_executed,
                tags.as_slice(),
                &worker_name
            )
            .execute(db)
            .await
            .expect("update worker ping");

            last_ping = Instant::now();
        }

        if (jobs_executed as u32 + vacuum_shift) % VACUUM_PERIOD == 0 {
            if let Err(e) = sqlx::query!("VACUUM queue").execute(db).await {
                tracing::error!(worker = %worker_name, "failed to vacuum queue: {}", e);
            }
            tracing::info!(worker = %worker_name, "vacuumed queue and completed_job");
        }

        #[cfg(feature = "enterprise")]
        if i_worker == 1 && S3_CACHE_BUCKET.is_some() {
            if last_sync.elapsed().as_secs() > *GLOBAL_CACHE_INTERVAL
                && (copy_cache_from_bucket_handle.is_none()
                    || copy_cache_from_bucket_handle
                        .as_ref()
                        .unwrap()
                        .is_finished())
            {
                last_sync = Instant::now();

                if crate::global_cache::worker_s3_bucket_sync_enabled(&db).await {
                    tracing::debug!("CAN PULL LOCK START");
                    let _lock = CAN_PULL.write().await;

                    tracing::info!("Started syncing cache");
                    // if num_workers > 1 {
                    //     create_barrier_for_all_workers(num_workers, sync_barrier.clone()).await;
                    // }
                    if let Err(e) = copy_cache_to_tmp_cache().await {
                        tracing::error!("failed to copy cache to tmp cache: {}", e);
                    } else {
                        copy_cache_from_bucket_handle = Some(tokio::task::spawn(async move {
                            if let Some(ref s) = S3_CACHE_BUCKET.clone() {
                                if let Err(e) = cache_global(s, copy_tx).await {
                                    tracing::error!("failed to sync cache: {}", e);
                                }
                            }
                        }));
                    }
                    tracing::info!("Ended syncing cache sync part");
                }
            }
        }

        // // The barrier is to avoid the sync to bucket syncing partial folders
        // #[cfg(feature = "enterprise")]
        // if num_workers > 1 && S3_CACHE_BUCKET.is_some()  {
        //     let read_barrier = sync_barrier.read().await;
        //     if let Some(b) = read_barrier.as_ref() {
        //         tracing::debug!("worker #{i_worker} waiting for barrier");
        //         b.wait().await;
        //         tracing::debug!("worker #{i_worker} done waiting for barrier");
        //         drop(read_barrier);
        //         // wait for barrier to be reset
        //         let _ = CAN_PULL.read().await;
        //         tracing::debug!("worker #{i_worker} done waiting for lock");
        //     } else {
        //         tracing::debug!("worker #{i_worker} no barrier");
        //     };
        // }

        let next_job = {
            // println!("2: {:?}",  instant.elapsed());
            #[cfg(feature = "benchmark")]
            if !started {
                started = true
            }

            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    #[cfg(feature = "enterprise")]
                    if let Some(copy_cache_from_bucket_handle) = copy_cache_from_bucket_handle.as_ref() {
                        if !copy_cache_from_bucket_handle.is_finished() {
                            copy_cache_from_bucket_handle.abort();
                        }
                    }
                    #[cfg(feature = "enterprise")]
                    for handle in &handles {
                        if !handle.is_finished() {
                            handle.abort();
                        }
                    }
                    println!("received killpill for worker {}", i_worker);
                    break
                },
                _ = copy_to_bucket_rx.recv() => {
                    tracing::debug!("can_pull lock start");
                    let _lock = CAN_PULL.write().await;
                    // if num_workers > 1 {
                    //     create_barrier_for_all_workers(num_workers, sync_barrier.clone()).await;
                    // }
                    //Arc::new(tokio::sync::Barrier::new(num_workers as usize + 1));
                    #[cfg(feature = "enterprise")]
                    if let Err(e) = copy_tmp_cache_to_cache().await {
                        tracing::error!(worker = %worker_name, "failed to sync tmp cache to cache: {}", e);
                    }
                    tracing::debug!("can_pull lock end");
                    Ok(None)
                },
                Some(job_id) = same_worker_rx.recv() => {
                    sqlx::query_as::<_, QueuedJob>("SELECT * FROM queue WHERE id = $1")
                    .bind(job_id)
                    .fetch_optional(db)
                    .await
                    .map_err(|_| Error::InternalErr("Impossible to fetch same_worker job".to_string()))
                },
                (job, timer) = {
                    let timer = if *METRICS_ENABLED { Some(worker_pull_duration.start_timer()) } else { None };
                    let suspend_first = if last_checked_suspended.elapsed().as_secs() > 3 {
                        last_checked_suspended = Instant::now();
                        true
                    } else { false };
                    pull(&db, rsmq.clone(), suspend_first).map(|x| (x, timer))
                } => {
                    add_time!(timing, loop_start, "post pull");

                    timer.map(|timer| {
                        let duration_pull_s = timer.stop_and_record();
                        worker_pull_duration_counter.inc_by(duration_pull_s);
                    });
                    job

                },
            }
        };

        if *METRICS_ENABLED {
            worker_busy.set(1);
        }

        match next_job {
            Ok(Some(job)) => {
                last_executed_job = None;
                jobs_executed += 1;

                if let Some(dedicated_worker_tx) = dedicated_worker_tx.clone() {
                    #[cfg(feature = "benchmark")]
                    main_duration
                        .fetch_add(loop_start.elapsed().as_millis() as usize, Ordering::SeqCst);
                    #[cfg(feature = "benchmark")]
                    let send_start = Instant::now();

                    if let Err(e) = dedicated_worker_tx.send(job.clone()).await {
                        tracing::info!("failed to send jobs to dedicated workers. Likely dedicated worker has been shut down. This is normal: {e:?}");
                    }

                    #[cfg(feature = "benchmark")]
                    send_duration
                        .fetch_add(send_start.elapsed().as_millis() as usize, Ordering::SeqCst);
                    continue;
                } else if matches!(job.job_kind, JobKind::Noop) {
                    #[cfg(feature = "benchmark")]
                    main_duration
                        .fetch_add(loop_start.elapsed().as_millis() as usize, Ordering::SeqCst);
                    #[cfg(feature = "benchmark")]
                    let send_start = Instant::now();

                    job_completed_tx
                        .send(JobCompleted {
                            job,
                            success: true,
                            result: json!({}),
                            logs: String::new(),
                            cached_res_path: None,
                            token: "".to_string(),
                        })
                        .await
                        .expect("send job completed");

                    #[cfg(feature = "benchmark")]
                    send_duration
                        .fetch_add(send_start.elapsed().as_millis() as usize, Ordering::SeqCst);
                } else {
                    let token = create_token_for_owner_in_bg(&db, &job).await;

                    let language = job.language.clone();
                    let _timer = worker_execution_duration
                        .get(&language)
                        .expect("no timer found")
                        .start_timer();

                    if *METRICS_ENABLED {
                        worker_execution_count
                            .get(&language)
                            .expect("no timer found")
                            .inc();
                    }

                    let job_root = job
                        .root_job
                        .map(|x| x.to_string())
                        .unwrap_or_else(|| "none".to_string());

                    if job.id == Uuid::nil() {
                        tracing::info!(worker = %worker_name, "running warmup job");
                    } else {
                        tracing::info!(worker = %worker_name, workspace_id = %job.workspace_id, id = %job.id, root_id = %job_root, "fetched job {}, root job: {}", job.id, job_root);
                    }

                    let job_dir = format!("{worker_dir}/{}", job.id);

                    DirBuilder::new()
                        .recursive(true)
                        .create(&job_dir)
                        .await
                        .expect("could not create job dir");

                    let same_worker = job.same_worker;

                    let target = &format!("{job_dir}/shared");

                    if same_worker && job.parent_job.is_some() {
                        let parent_flow = job.parent_job.unwrap();
                        let parent_shared_dir = format!("{worker_dir}/{parent_flow}/shared");
                        DirBuilder::new()
                            .recursive(true)
                            .create(&parent_shared_dir)
                            .await
                            .expect("could not create parent shared dir");

                        symlink(&parent_shared_dir, target)
                            .await
                            .expect("could not symlink target");
                    } else {
                        DirBuilder::new()
                            .recursive(true)
                            .create(target)
                            .await
                            .expect("could not create shared dir");
                    }

                    let authed_client = AuthedClientBackgroundTask {
                        base_internal_url: base_internal_url.to_string(),
                        token,
                        workspace: job.workspace_id.to_string(),
                        client: OnceCell::new(),
                    };

                    if let Some(err) = handle_queued_job(
                        job.clone(),
                        db,
                        &authed_client,
                        &worker_name,
                        &worker_dir,
                        &job_dir,
                        same_worker_tx.clone(),
                        base_internal_url,
                        rsmq.clone(),
                        job_completed_tx.clone(),
                    )
                    .await
                    .err()
                    {
                        let metrics = build_language_metrics(&worker_execution_failed, &language);

                        handle_job_error(
                            db,
                            &authed_client.get_authed().await,
                            &job,
                            err,
                            metrics,
                            false,
                            same_worker_tx.clone(),
                            &worker_dir,
                            rsmq.clone(),
                        )
                        .await;
                    };

                    let duration = _timer.stop_and_record();
                    worker_execution_duration_counter
                        .get(&language)
                        .expect("no timer found")
                        .inc_by(duration);

                    if !*KEEP_JOB_DIR && !(job.is_flow() && same_worker) {
                        let _ = tokio::fs::remove_dir_all(job_dir).await;
                    }
                }
                #[cfg(feature = "benchmark")]
                {
                    if started {
                        add_time!(timing, loop_start, format!("post iter: {}", infos.iters));
                        infos.iters += 1;
                        infos.timings.push(timing);
                    }
                }
            }
            Ok(None) => {
                if let Some(secs) = *EXIT_AFTER_NO_JOB_FOR_SECS {
                    if let Some(lj) = last_executed_job {
                        if lj.elapsed().as_secs() > secs {
                            tracing::info!(worker = %worker_name, "no job for {} seconds, exiting", secs);
                            break;
                        }
                    } else {
                        last_executed_job = Some(Instant::now());
                    }
                }
                let _timer = if *METRICS_ENABLED {
                    Some(Instant::now())
                } else {
                    None
                };

                #[cfg(feature = "benchmark")]
                tracing::info!("no job found");

                tokio::time::sleep(Duration::from_millis(*SLEEP_QUEUE)).await;
                _timer.map(|timer| {
                    let duration = timer.elapsed().as_secs_f64();
                    worker_sleep_duration_counter.inc_by(duration);
                });
            }
            Err(err) => {
                tracing::error!(worker = %worker_name, "Failed to pull jobs: {}", err);
            }
        };
    }

    // #[cfg(feature = "benchmark")]
    // {
    //     println!("Writing benchmark file");
    //     write_file(
    //         TMP_DIR,
    //         "/profiling.json",
    //         &serde_json::to_string(&infos).unwrap(),
    //     )
    //     .await
    //     .expect("write profiling");
    // }

    drop(dedicated_worker_tx);

    if let Some(handle) = dedicated_worker_handle {
        handle.await.expect("dedicated worker failed");
    }

    drop(job_completed_tx);

    send_result.await.expect("send result failed");
    println!("worker {} exited", i_worker);
}

async fn queue_init_bash_maybe<'c, R: rsmq_async::RsmqConnection + Send + 'c>(
    db: &Pool<Postgres>,
    same_worker_tx: Sender<Uuid>,
    worker_name: &str,
    rsmq: Option<R>,
) -> error::Result<()> {
    if let Some(content) = WORKER_CONFIG.read().await.init_bash.clone() {
        let tx = PushIsolationLevel::IsolatedRoot(db.clone(), rsmq);
        let (uuid, inner_tx) = push(
            &db,
            tx,
            "admins",
            windmill_common::jobs::JobPayload::Code(windmill_common::jobs::RawCode {
                content: content.clone(),
                path: Some(format!("init_script_{worker_name}")),
                language: ScriptLang::Bash,
                lock: None,
                concurrent_limit: None,
                concurrency_time_window_s: None,
                cache_ttl: None,
            }),
            serde_json::Map::new(),
            worker_name,
            "worker@windmill.dev",
            SUPERADMIN_SECRET_EMAIL.to_string(),
            None,
            None,
            None,
            None,
            None,
            false,
            true,
            None,
            true,
            None,
            None,
            None,
        )
        .await?;
        inner_tx.commit().await?;
        same_worker_tx.send(uuid).await.map_err(to_anyhow)?;
        tracing::info!("Creating initial job {uuid} from initial script script: {content}");
    }
    Ok(())
}

// async fn process_result<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
//     client: AuthedClient,
//     job: QueuedJob,
//     result: error::Result<serde_json::Value>,
//     cached_res_path: Option<String>,
//     db: &DB,
//     worker_dir: &str,
//     job_dir: &str,
//     metrics: Option<Metrics>,
//     same_worker_tx: Sender<Uuid>,
//     base_internal_url: &str,
//     rsmq: Option<R>,
//     job_completed_tx: Sender<JobCompleted>,
//     logs: String,
// ) -> error::Result<()> {

pub async fn process_completed_job<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    JobCompleted { job, result, logs, success, cached_res_path, .. }: &JobCompleted,
    client: &AuthedClient,
    db: &DB,
    worker_dir: &str,
    metrics: Option<Metrics>,
    same_worker_tx: Sender<Uuid>,
    rsmq: Option<R>,
) -> windmill_common::error::Result<()> {
    if *success {
        // println!("bef completed job{:?}",  SystemTime::now());
        if let Some(cached_path) = cached_res_path {
            save_in_cache(&client, &job, cached_path.to_string(), &result).await;
        }
        add_completed_job(
            db,
            &job,
            true,
            false,
            result,
            logs.to_string(),
            rsmq.clone(),
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
                    true,
                    result.clone(),
                    metrics.clone(),
                    false,
                    same_worker_tx.clone(),
                    &worker_dir,
                    None,
                    rsmq.clone(),
                )
                .await?;
            }
        }
    } else {
        let result = add_completed_job_error(
            db,
            &job,
            logs.to_string(),
            &result,
            metrics.clone(),
            rsmq.clone(),
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
                    result,
                    metrics,
                    false,
                    same_worker_tx,
                    &worker_dir,
                    None,
                    rsmq,
                )
                .await?;
            }
        }
    }
    Ok(())
}

fn build_language_metrics(
    worker_execution_failed: &HashMap<
        Option<ScriptLang>,
        prometheus::core::GenericCounter<prometheus::core::AtomicU64>,
    >,
    language: &Option<ScriptLang>,
) -> Option<Metrics> {
    let metrics = if *METRICS_ENABLED {
        Some(Metrics {
            worker_execution_failed: worker_execution_failed
                .get(language)
                .expect("no timer found")
                .clone(),
        })
    } else {
        None
    };
    metrics
}

// pub async fn create_barrier_for_all_workers(num_workers: u32, sync_barrier: Arc<RwLock<Option<tokio::sync::Barrier>>>) {
//     tracing::debug!("acquiring write lock");
//     let mut barrier = sync_barrier.write().await;
//     *barrier = Some(tokio::sync::Barrier::new(num_workers as usize));
//     drop(barrier);
//     tracing::debug!("dropped write lock");
//     if let Some(b) = sync_barrier.read().await.as_ref() {
//         tracing::debug!("leader worker waiting for barrier");
//         b.wait().await;
//         tracing::debug!("leader worker done waiting for barrier");
//     };
//     let mut barrier = sync_barrier.write().await;
//     *barrier = None;
//     tracing::debug!("leader worker done waiting for");
// }
pub async fn handle_job_error<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    db: &Pool<Postgres>,
    client: &AuthedClient,
    job: &QueuedJob,
    err: Error,
    metrics: Option<Metrics>,
    unrecoverable: bool,
    same_worker_tx: Sender<Uuid>,
    worker_dir: &str,
    rsmq: Option<R>,
) {
    let err = match err {
        Error::JsonErr(err) => err,
        _ => json!({"message": err.to_string(), "name": "InternalErr"}),
    };

    let rsmq_2 = rsmq.clone();
    let update_job_future = || {
        add_completed_job_error(
            db,
            job,
            format!("Unexpected error during job execution:\n{err}"),
            &err,
            metrics.clone(),
            rsmq_2,
        )
    };

    let update_job_future = if job.is_flow_step
        || job.job_kind == JobKind::FlowPreview
        || job.job_kind == JobKind::Flow
    {
        let (flow, job_status_to_update, update_job_future) =
            if let Some(parent_job_id) = job.parent_job {
                let _ = update_job_future().await;
                (parent_job_id, job.id, None)
            } else {
                (job.id, Uuid::nil(), Some(update_job_future))
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
            rsmq.clone(),
        )
        .await;

        if let Err(err) = updated_flow {
            if let Some(parent_job_id) = job.parent_job {
                if let Ok(mut tx) = db.begin().await {
                    if let Ok(Some(parent_job)) =
                        get_queued_job(parent_job_id, &job.workspace_id, &mut tx).await
                    {
                        let e = json!({"message": err.to_string(), "name": "InternalErr"});
                        let _ = add_completed_job_error(
                            db,
                            &parent_job,
                            format!("Unexpected error during flow job error handling:\n{err}"),
                            &e,
                            metrics.clone(),
                            rsmq,
                        )
                        .await;
                    }
                }
            }
        }

        update_job_future
    } else {
        Some(update_job_future)
    };
    if let Some(f) = update_job_future {
        let _ = f().await;
    }
    tracing::error!(job_id = %job.id, "error handling job: {err:?} {} {} {}", job.id, job.workspace_id, job.created_by);
}

fn extract_error_value(log_lines: &str, i: i32) -> serde_json::Value {
    return json!({"message": format!("ExitCode: {i}, last log lines:\n{}", ANSI_ESCAPE_RE.replace_all(log_lines.trim(), "").to_string()), "name": "ExecutionErr"});
}

#[derive(Debug, Clone)]
pub struct JobCompleted {
    pub job: QueuedJob,
    pub result: serde_json::Value,
    pub logs: String,
    pub success: bool,
    pub cached_res_path: Option<String>,
    pub token: String,
}

pub async fn get_content(job: &QueuedJob, db: &Pool<Postgres>) -> Result<String, Error> {
    let query = match job.job_kind {
        JobKind::Preview => job
            .raw_code
            .clone()
            .ok_or_else(|| Error::ExecutionErr("Missing code".to_string()))?,
        JobKind::Script => {
            sqlx::query_scalar("SELECT content FROM script WHERE hash = $1 AND workspace_id = $2")
                .bind(&job.script_hash.unwrap_or(ScriptHash(0)).0)
                .bind(&job.workspace_id)
                .fetch_optional(db)
                .await?
                .ok_or_else(|| Error::InternalErr(format!("expected content")))?
        }
        _ => unreachable!(
            "get_content called for non-script job kind: {:#?}",
            job.job_kind
        ),
    };
    Ok(query)
}

async fn do_nativets(
    job: QueuedJob,
    logs: String,
    client: &AuthedClient,
    code: String,
    db: &Pool<Postgres>,
) -> windmill_common::error::Result<(serde_json::Value, String)> {
    let args = if let Some(args) = &job.args {
        Some(transform_json_value("args", client, &job.workspace_id, args.clone(), &job, db).await?)
    } else {
        None
    };

    let args = args
        .as_ref()
        .map(|x| x.clone())
        .unwrap_or_else(|| json!({}))
        .as_object()
        .unwrap()
        .clone();
    let result = eval_fetch_timeout(code.clone(), transpile_ts(code)?, args).await?;
    Ok((result.0, [logs, result.1].join("\n\n")))
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_queued_job<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    job: QueuedJob,
    db: &DB,
    client: &AuthedClientBackgroundTask,
    worker_name: &str,
    worker_dir: &str,
    job_dir: &str,
    same_worker_tx: Sender<Uuid>,
    base_internal_url: &str,
    rsmq: Option<R>,
    job_completed_tx: Sender<JobCompleted>,
) -> windmill_common::error::Result<()> {
    if job.canceled {
        return Err(Error::JsonErr(canceled_job_to_result(&job)))?;
    }
    if let Some(e) = job.pre_run_error {
        return Err(Error::ExecutionErr(e));
    }

    let step = if job.is_flow_step {
        let r = update_flow_status_in_progress(
            db,
            &job.workspace_id,
            job.parent_job
                .ok_or_else(|| Error::InternalErr(format!("expected parent job")))?,
            job.id,
        )
        .await?;

        r
    } else {
        None
    };

    let cached_res_path = if job.cache_ttl.is_some() {
        let args_hash = hash_args(&job.args.clone().unwrap_or_else(|| json!({})));
        if job.is_flow_step {
            let flow_path = sqlx::query_scalar!(
                "SELECT script_path FROM queue WHERE id = $1",
                &job.parent_job.unwrap()
            )
            .fetch_one(db)
            .await
            .map_err(|e| Error::InternalErr(format!("fetching step flow status: {e}")))?
            .ok_or_else(|| Error::InternalErr(format!("Expected script_path")))?;
            let step = step.unwrap_or(-1);
            Some(format!("{flow_path}/cache/{step}/{args_hash}"))
        } else if let Some(script_path) = &job.script_path {
            let is_flow = if job.is_flow() { "flow/" } else { "" };
            Some(format!("{script_path}/{is_flow}cache/{args_hash}"))
        } else {
            None
        }
    } else {
        None
    };

    if let Some(cached_res_path) = cached_res_path.clone() {
        let authed_client = client.get_authed().await;
        let client: &Client = authed_client.get_client();
        let resource = client
            .get_resource_value(&job.workspace_id, &cached_res_path)
            .await;

        if let Ok(resource) = resource {
            let v = resource.into_inner();
            if let Some(o) = v.as_object() {
                let expire = o.get("expire");
                if expire.is_some()
                    && expire
                        .unwrap()
                        .as_i64()
                        .map(|x| x > chrono::Utc::now().timestamp())
                        .unwrap_or(false)
                {
                    let result = v
                        .get("value")
                        .map(|x| x.to_owned())
                        .unwrap_or_else(|| json!({}));
                    let logs = "Job skipped because args & path found in cache and not expired"
                        .to_string();

                    job_completed_tx
                        .send(JobCompleted {
                            job,
                            result,
                            logs,
                            success: true,
                            cached_res_path: None,
                            token: authed_client.token,
                        })
                        .await
                        .expect("send job completed");

                    return Ok(());
                }
            }
        }
    };
    match job.job_kind {
        JobKind::FlowPreview | JobKind::Flow => {
            let args = job.args.clone().unwrap_or(Value::Null);
            handle_flow(
                &job,
                db,
                &client.get_authed().await,
                args,
                same_worker_tx,
                worker_dir,
                rsmq,
            )
            .await?;
        }
        _ => {
            let mut logs = "".to_string();
            // println!("handle queue {:?}",  SystemTime::now());
            if let Some(log_str) = &job.logs {
                logs.push_str(&log_str);
                logs.push_str("\n");
            }

            logs.push_str(&format!(
                "job {} on worker {} (tag: {})\n",
                &job.id, &worker_name, &job.tag
            ));

            tracing::debug!(
                worker = %worker_name,
                job_id = %job.id,
                workspace_id = %job.workspace_id,
                "handling job {}",
                job.id
            );

            let result = match job.job_kind {
                JobKind::Dependencies => {
                    handle_dependency_job(
                        &job,
                        &mut logs,
                        job_dir,
                        db,
                        worker_name,
                        worker_dir,
                        base_internal_url,
                        &client.get_token().await,
                    )
                    .await
                }
                JobKind::FlowDependencies => handle_flow_dependency_job(
                    &job,
                    &mut logs,
                    job_dir,
                    db,
                    worker_name,
                    worker_dir,
                    base_internal_url,
                    &client.get_token().await,
                )
                .await
                .map(|()| Value::Null),
                JobKind::AppDependencies => handle_app_dependency_job(
                    &job,
                    &mut logs,
                    job_dir,
                    db,
                    worker_name,
                    worker_dir,
                    base_internal_url,
                    &client.get_token().await,
                )
                .await
                .map(|()| Value::Null),
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
                        job_dir,
                        worker_dir,
                        &mut logs,
                        base_internal_url,
                        worker_name,
                    )
                    .await
                }
            };

            //it's a test job, no need to update the db
            if job.workspace_id == "" {
                return Ok(());
            }
            process_result(
                job,
                result,
                job_dir,
                job_completed_tx,
                logs,
                cached_res_path,
                client.get_token().await,
            )
            .await?;
        }
    }
    Ok(())
}

async fn process_result(
    job: QueuedJob,
    result: error::Result<serde_json::Value>,
    job_dir: &str,
    job_completed_tx: Sender<JobCompleted>,
    logs: String,
    cached_res_path: Option<String>,
    token: String,
) -> error::Result<()> {
    match result {
        Ok(r) => {
            job_completed_tx
                .send(JobCompleted {
                    job,
                    result: r,
                    logs,
                    success: true,
                    cached_res_path,
                    token: token,
                })
                .await
                .expect("send job completed");
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

            // in the happy path and if job not a flow step, we can delegate updating the completed job in the background
            job_completed_tx
                .send(JobCompleted {
                    job,
                    result: error_value,
                    logs: logs,
                    success: false,
                    cached_res_path,
                    token: token,
                })
                .await
                .expect("send job completed");
        }
    };
    Ok(())
}

fn build_envs(
    envs: Option<Vec<String>>,
) -> windmill_common::error::Result<HashMap<String, String>> {
    let mut envs = if *CLOUD_HOSTED || envs.is_none() {
        HashMap::new()
    } else {
        let mut hm = HashMap::new();
        for s in envs.unwrap() {
            let (k, v) = s.split_once('=').ok_or_else(|| {
                Error::BadRequest(format!(
                    "Invalid env var: {}. Must be in the form of KEY=VALUE",
                    s
                ))
            })?;
            hm.insert(k.to_string(), v.to_string());
        }
        hm
    };

    if let Some(ref env) = *HTTPS_PROXY {
        envs.insert("HTTPS_PROXY".to_string(), env.to_string());
    }
    if let Some(ref env) = *HTTP_PROXY {
        envs.insert("HTTP_PROXY".to_string(), env.to_string());
    }
    if let Some(ref env) = *NO_PROXY {
        envs.insert("NO_PROXY".to_string(), env.to_string());
    }
    Ok(envs)
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_code_execution_job(
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    job_dir: &str,
    worker_dir: &str,
    logs: &mut String,
    base_internal_url: &str,
    worker_name: &str,
) -> error::Result<serde_json::Value> {
    let (inner_content, requirements_o, language, envs) = match job.job_kind {
        JobKind::Preview  => (
            job.raw_code
                .clone()
                .unwrap_or_else(|| "no raw code".to_owned()),
            job.raw_lock.clone(),
            job.language.to_owned(),
            None,
        ),
        JobKind::Script_Hub => {
            let script_path = job.script_path.clone().ok_or_else(|| Error::InternalErr(format!("expected script path for hub script")))?;
            let mut script_path_iterator = script_path.split("/");
            script_path_iterator.next();
            let version = script_path_iterator.next().ok_or_else(|| Error::InternalErr(format!("expected hub path to have version number")))?;
            let cache_path = format!("{HUB_CACHE_DIR}/{version}");
            let script;
            if tokio::fs::metadata(&cache_path).await.is_err() {
                script =  get_full_hub_script_by_path(&job.email, StripPath(script_path.clone()), &HTTP_CLIENT).await?;
                write_file(HUB_CACHE_DIR, &version, &serde_json::to_string(&script).map_err(to_anyhow)?).await?;
                tracing::info!("wrote hub script {script_path} to cache");
            } else {
                let cache_content = tokio::fs::read_to_string(cache_path).await?;
                script = serde_json::from_str(&cache_content).unwrap();
                tracing::info!("read hub script {script_path} from cache");
            }
            (
            script.content,
            script.lockfile,
            Some(script.language),
            None
        )},
        JobKind::Script => sqlx::query_as::<_, (String, Option<String>, Option<ScriptLang>, Option<Vec<String>>)>(
            "SELECT content, lock, language, envs FROM script WHERE hash = $1 AND workspace_id = $2",
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

    if language == Some(ScriptLang::Postgresql) {
        return do_postgresql(job.clone(), &client.get_authed().await, &inner_content, db).await;
    } else if language == Some(ScriptLang::Mysql) {
        return do_mysql(job.clone(), &client.get_authed().await, &inner_content, db).await;
    } else if language == Some(ScriptLang::Bigquery) {
        #[cfg(not(feature = "enterprise"))]
        {
            return Err(Error::ExecutionErr(
                "Bigquery is only available with an enterprise license".to_string(),
            ));
        }

        #[cfg(feature = "enterprise")]
        {
            return do_bigquery(job.clone(), &client.get_authed().await, &inner_content, db).await;
        }
    } else if language == Some(ScriptLang::Snowflake) {
        #[cfg(not(feature = "enterprise"))]
        {
            return Err(Error::ExecutionErr(
                "Snowflake is only available with an enterprise license".to_string(),
            ));
        }

        #[cfg(feature = "enterprise")]
        {
            return do_snowflake(job.clone(), &client.get_authed().await, &inner_content, db).await;
        }
    } else if language == Some(ScriptLang::Graphql) {
        return do_graphql(job.clone(), &client.get_authed().await, &inner_content, db).await;
    } else if language == Some(ScriptLang::Nativets) {
        logs.push_str("\n--- FETCH TS EXECUTION ---\n");
        let code = format!(
            "const BASE_URL = '{base_internal_url}';\nconst WM_TOKEN = '{}';\n{}",
            &client.get_token().await,
            inner_content
        );
        let (result, ts_logs) = do_nativets(
            job.clone(),
            logs.clone(),
            &client.get_authed().await,
            code,
            db,
        )
        .await?;
        *logs = ts_logs;
        return Ok(result);
    }

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

    // println!("handle lang job {:?}",  SystemTime::now());

    let envs = build_envs(envs)?;

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
                &inner_content,
                &shared_mount,
                base_internal_url,
                envs,
            )
            .await
        }
        Some(ScriptLang::Deno) => {
            handle_deno_job(
                requirements_o,
                logs,
                job,
                db,
                client,
                job_dir,
                &inner_content,
                base_internal_url,
                worker_name,
                envs,
            )
            .await
        }
        Some(ScriptLang::Bun) => {
            handle_bun_job(
                requirements_o,
                logs,
                job,
                db,
                client,
                job_dir,
                &inner_content,
                base_internal_url,
                worker_name,
                envs,
                &shared_mount,
            )
            .await
        }
        Some(ScriptLang::Go) => {
            handle_go_job(
                logs,
                job,
                db,
                client,
                &inner_content,
                job_dir,
                requirements_o,
                &shared_mount,
                base_internal_url,
                worker_name,
                envs,
            )
            .await
        }
        Some(ScriptLang::Bash) => {
            handle_bash_job(
                logs,
                job,
                db,
                client,
                &inner_content,
                job_dir,
                &shared_mount,
                base_internal_url,
                worker_name,
                envs,
            )
            .await
        }
        Some(ScriptLang::Powershell) => {
            handle_powershell_job(
                logs,
                job,
                db,
                client,
                &inner_content,
                job_dir,
                &shared_mount,
                base_internal_url,
                worker_name,
                envs,
            )
            .await
        }
        _ => panic!("unreachable, language is not supported: {language:#?}"),
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
    // println!("handled job: {:?}",  SystemTime::now());

    result
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_dependency_job(
    job: &QueuedJob,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    base_internal_url: &str,
    token: &str,
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
        worker_dir,
        base_internal_url,
        token,
        job.script_path(),
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

async fn handle_flow_dependency_job(
    job: &QueuedJob,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    base_internal_url: &str,
    token: &str,
) -> error::Result<()> {
    let job_path = job.script_path.clone().ok_or_else(|| {
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

    flow.modules = lock_modules(
        flow.modules,
        job,
        logs,
        job_dir,
        db,
        worker_name,
        worker_dir,
        &job_path,
        base_internal_url,
        token,
    )
    .await?;
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
        job_path,
        job.workspace_id
    )
    .execute(db)
    .await?;
    Ok(())
}

#[async_recursion]
async fn lock_modules(
    modules: Vec<FlowModule>,
    job: &QueuedJob,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    job_path: &str,
    base_internal_url: &str,
    token: &str,
) -> Result<Vec<FlowModule>> {
    let mut new_flow_modules = Vec::new();
    for mut e in modules.into_iter() {
        let FlowModuleValue::RawScript {
            lock: _,
            path,
            content,
            language,
            input_transforms,
            tag,
            concurrent_limit,
            concurrency_time_window_s,
        } = e.value
        else {
            match e.value {
                FlowModuleValue::ForloopFlow {
                    iterator,
                    modules,
                    skip_failures,
                    parallel,
                    parallelism,
                } => {
                    e.value = FlowModuleValue::ForloopFlow {
                        iterator,
                        modules: lock_modules(
                            modules,
                            job,
                            logs,
                            job_dir,
                            db,
                            worker_name,
                            worker_dir,
                            job_path,
                            base_internal_url,
                            token,
                        )
                        .await?,
                        skip_failures,
                        parallel,
                        parallelism,
                    }
                }
                FlowModuleValue::BranchAll { branches, parallel } => {
                    let mut nbranches = vec![];
                    for mut b in branches {
                        b.modules = lock_modules(
                            b.modules,
                            job,
                            logs,
                            job_dir,
                            db,
                            worker_name,
                            worker_dir,
                            job_path,
                            base_internal_url,
                            token,
                        )
                        .await?;
                        nbranches.push(b)
                    }
                    e.value = FlowModuleValue::BranchAll { branches: nbranches, parallel }
                }
                FlowModuleValue::BranchOne { branches, default } => {
                    let mut nbranches = vec![];
                    for mut b in branches {
                        b.modules = lock_modules(
                            b.modules,
                            job,
                            logs,
                            job_dir,
                            db,
                            worker_name,
                            worker_dir,
                            job_path,
                            base_internal_url,
                            token,
                        )
                        .await?;
                        nbranches.push(b)
                    }
                    let default = lock_modules(
                        default,
                        job,
                        logs,
                        job_dir,
                        db,
                        worker_name,
                        worker_dir,
                        job_path,
                        base_internal_url,
                        token,
                    )
                    .await?;
                    e.value = FlowModuleValue::BranchOne { branches: nbranches, default };
                }
                _ => (),
            };
            new_flow_modules.push(e);
            continue;
        };
        // sync with windmill-api/scripts
        let dependencies = match language {
            ScriptLang::Python3 => windmill_parser_py_imports::parse_python_imports(
                &content,
                &job.workspace_id,
                &path.clone().unwrap_or_else(|| job_path.to_string()),
                &db,
            )
            .await?
            .join("\n"),
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
            worker_dir,
            base_internal_url,
            token,
            job.script_path(),
        )
        .await;
        match new_lock {
            Ok(new_lock) => {
                e.value = FlowModuleValue::RawScript {
                    lock: Some(new_lock),
                    path,
                    input_transforms,
                    content,
                    language,
                    tag,
                    concurrent_limit,
                    concurrency_time_window_s,
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
                    path,
                    input_transforms,
                    content,
                    language,
                    tag,
                    concurrent_limit,
                    concurrency_time_window_s,
                };
                new_flow_modules.push(e);
                continue;
            }
        }
    }
    Ok(new_flow_modules)
}

#[async_recursion]
async fn lock_modules_app(
    value: Value,
    job: &QueuedJob,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    job_path: &str,
    base_internal_url: &str,
    token: &str,
) -> Result<Value> {
    match value {
        Value::Object(mut m) => {
            if m.contains_key("inlineScript") {
                let v = m.get_mut("inlineScript").unwrap();
                if let Some(v) = v.as_object_mut() {
                    if v.contains_key("content") && v.contains_key("language") {
                        if let Ok(language) =
                            serde_json::from_value::<ScriptLang>(v.get("language").unwrap().clone())
                        {
                            let content = v
                                .get("content")
                                .unwrap()
                                .as_str()
                                .unwrap_or_default()
                                .to_string();
                            let dependencies = match language {
                                ScriptLang::Python3 => {
                                    windmill_parser_py_imports::parse_python_imports(
                                        &content,
                                        &job.workspace_id,
                                        job_path,
                                        &db,
                                    )
                                    .await?
                                    .join("\n")
                                }
                                _ => content.clone(),
                            };
                            logs.push_str("Found lockable inline script. Generating lock...\n");
                            let new_lock = capture_dependency_job(
                                &job.id,
                                &language,
                                &dependencies,
                                logs,
                                job_dir,
                                db,
                                worker_name,
                                &job.workspace_id,
                                worker_dir,
                                base_internal_url,
                                token,
                                job.script_path(),
                            )
                            .await;
                            match new_lock {
                                Ok(new_lock) => {
                                    v.insert(
                                        "lock".to_string(),
                                        serde_json::Value::String(new_lock),
                                    );
                                    return Ok(Value::Object(m.clone()));
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        language = ?language,
                                        error = ?e,
                                        logs = ?logs,
                                        "Failed to generate flow lock for inline script"
                                    );
                                    ()
                                }
                            }
                        }
                    }
                }
            }
            for (a, b) in m.clone().into_iter() {
                m.insert(
                    a.clone(),
                    lock_modules_app(
                        b,
                        job,
                        logs,
                        job_dir,
                        db,
                        worker_name,
                        worker_dir,
                        job_path,
                        base_internal_url,
                        token,
                    )
                    .await?,
                );
            }
            Ok(Value::Object(m))
        }
        Value::Array(a) => {
            let mut nv = vec![];
            for b in a.clone().into_iter() {
                nv.push(
                    lock_modules_app(
                        b,
                        job,
                        logs,
                        job_dir,
                        db,
                        worker_name,
                        worker_dir,
                        job_path,
                        base_internal_url,
                        token,
                    )
                    .await?,
                );
            }
            Ok(Value::Array(nv))
        }
        a @ _ => Ok(a),
    }
}

async fn handle_app_dependency_job(
    job: &QueuedJob,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    base_internal_url: &str,
    token: &str,
) -> error::Result<()> {
    let job_path = job.script_path.clone().ok_or_else(|| {
        error::Error::InternalErr(
            "Cannot resolve flow dependencies for flow without path".to_string(),
        )
    })?;

    let id = job
        .script_hash
        .clone()
        .ok_or_else(|| Error::InternalErr("Flow Dependency requires script hash".to_owned()))?
        .0;
    let value = sqlx::query_scalar!("SELECT value FROM app_version WHERE id = $1", id)
        .fetch_optional(db)
        .await?;

    if let Some(value) = value {
        let value = lock_modules_app(
            value,
            job,
            logs,
            job_dir,
            db,
            worker_name,
            worker_dir,
            &job_path,
            base_internal_url,
            token,
        )
        .await?;

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

        sqlx::query!("UPDATE app_version SET value = $1 WHERE id = $2", value, id,)
            .execute(db)
            .await?;
        Ok(())
    } else {
        Ok(())
    }
}

async fn capture_dependency_job(
    job_id: &Uuid,
    job_language: &ScriptLang,
    job_raw_code: &str,
    logs: &mut String,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    w_id: &str,
    worker_dir: &str,
    base_internal_url: &str,
    token: &str,
    script_path: &str,
) -> error::Result<String> {
    match job_language {
        ScriptLang::Python3 => {
            create_dependencies_dir(job_dir).await;
            let req: std::result::Result<String, Error> =
                pip_compile(job_id, job_raw_code, logs, job_dir, db, worker_name, w_id).await;
            // install the dependencies to pre-fill the cache
            if let Ok(req) = req.as_ref() {
                let r = handle_python_reqs(
                    req.split("\n").filter(|x| !x.starts_with("--")).collect(),
                    job_id,
                    w_id,
                    logs,
                    db,
                    worker_name,
                    job_dir,
                    worker_dir,
                )
                .await;

                if let Err(e) = r {
                    tracing::error!(
                        "Failed to install python dependencies to prefill the cache: {:?} \n{}",
                        e,
                        logs
                    );
                }
            }
            req
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
                w_id,
            )
            .await
        }
        ScriptLang::Deno => {
            generate_deno_lock(
                job_id,
                job_raw_code,
                logs,
                job_dir,
                db,
                w_id,
                worker_name,
                base_internal_url,
            )
            .await
        }
        ScriptLang::Bun => {
            let _ = write_file(job_dir, "main.ts", job_raw_code).await?;
            //TODO: remove once bun provides sane default fot it
            let trusted_deps = get_trusted_deps(job_raw_code);
            let req = gen_lockfile(
                logs,
                job_id,
                w_id,
                db,
                token,
                script_path,
                job_dir,
                base_internal_url,
                worker_name,
                true,
                trusted_deps,
            )
            .await?;
            Ok(req.unwrap_or_else(String::new))
        }
        ScriptLang::Postgresql => Ok("".to_owned()),
        ScriptLang::Mysql => Ok("".to_owned()),
        ScriptLang::Bigquery => Ok("".to_owned()),
        ScriptLang::Snowflake => Ok("".to_owned()),
        ScriptLang::Graphql => Ok("".to_owned()),
        ScriptLang::Bash => Ok("".to_owned()),
        ScriptLang::Powershell => Ok("".to_owned()),
        ScriptLang::Nativets => Ok("".to_owned()),
    }
}
