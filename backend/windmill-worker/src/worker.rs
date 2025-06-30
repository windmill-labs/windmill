/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

// #[cfg(feature = "otel")]
// use opentelemetry::{global,  KeyValue};

use anyhow::anyhow;
use futures::TryFutureExt;
use windmill_common::client::AuthedClient;
use windmill_common::{
    agent_workers::DECODED_AGENT_TOKEN,
    apps::AppScriptId,
    cache::{future::FutureCachedExt, ScriptData, ScriptMetadata},
    schema::{should_validate_schema, SchemaValidator},
    scripts::PREVIEW_IS_TAR_CODEBASE_HASH,
    utils::{create_directory_async, WarnAfterExt},
    worker::{
        make_pull_query, write_file, Connection, HttpClient, MAX_TIMEOUT, ROOT_CACHE_DIR,
        ROOT_CACHE_NOMOUNT_DIR, TMP_DIR,
    },
    KillpillSender,
};

#[cfg(feature = "enterprise")]
use windmill_common::ee_oss::LICENSE_KEY_VALID;

use anyhow::Result;
use const_format::concatcp;
#[cfg(feature = "prometheus")]
use prometheus::IntCounter;

use tracing::{field, Instrument, Span};
#[cfg(feature = "prometheus")]
use windmill_common::METRICS_DEBUG_ENABLED;
#[cfg(feature = "prometheus")]
use windmill_common::METRICS_ENABLED;

use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use std::{
    collections::HashMap,
    fmt::Display,
    sync::{
        atomic::{AtomicBool, AtomicU16, Ordering},
        Arc,
    },
    time::Duration,
};
use windmill_parser::MainArgSignature;

use uuid::Uuid;

use windmill_common::{
    cache::{self, RawData},
    error::{self, to_anyhow, Error},
    flows::FlowNodeId,
    jobs::JobKind,
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang, PREVIEW_IS_CODEBASE_HASH},
    utils::StripPath,
    worker::{CLOUD_HOSTED, NO_LOGS, WORKER_CONFIG, WORKER_GROUP},
    DB, IS_READY,
};

use windmill_queue::{
    append_logs, canceled_job_to_result, empty_result, get_same_worker_job, pull, push_init_job,
    CanceledBy, JobAndPerms, JobCompleted, MiniPulledJob, PrecomputedAgentInfo, PulledJob,
    SameWorkerPayload, HTTP_CLIENT,
};

#[cfg(feature = "prometheus")]
use windmill_queue::register_metric;

use serde_json::value::RawValue;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use tokio::fs::symlink;

#[cfg(target_os = "windows")]
use tokio::fs::symlink_file as symlink;

use tokio::{
    sync::{
        broadcast,
        mpsc::{self, Receiver, Sender},
        RwLock,
    },
    task::JoinHandle,
    time::Instant,
};

use rand::Rng;

use crate::{
    agent_workers::queue_init_job,
    bash_executor::{handle_bash_job, handle_powershell_job},
    bun_executor::handle_bun_job,
    common::{
        build_args_map, cached_result_path, error_to_value, get_cached_resource_value_if_valid,
        get_reserved_variables, update_worker_ping_for_failed_init_script, OccupancyMetrics,
    },
    csharp_executor::handle_csharp_job,
    deno_executor::handle_deno_job,
    go_executor::handle_go_job,
    graphql_executor::do_graphql,
    handle_child::SLOW_LOGS,
    handle_job_error,
    job_logger::NO_LOGS_AT_ALL,
    js_eval::{eval_fetch_timeout, transpile_ts},
    pg_executor::do_postgresql,
    result_processor::{process_result, start_background_processor},
    schema::schema_validator_from_main_arg_sig,
    worker_flow::handle_flow,
    worker_lockfiles::{
        handle_app_dependency_job, handle_dependency_job, handle_flow_dependency_job,
    },
    worker_utils::{insert_ping, queue_vacuum, update_worker_ping_full},
};

#[cfg(feature = "rust")]
use crate::rust_executor::handle_rust_job;

#[cfg(feature = "nu")]
use crate::nu_executor::{handle_nu_job, JobHandlerInput as JobHandlerInputNu};

#[cfg(feature = "java")]
use crate::java_executor::{handle_java_job, JobHandlerInput as JobHandlerInputJava};

#[cfg(feature = "php")]
use crate::php_executor::handle_php_job;

#[cfg(feature = "python")]
use crate::{
    python_executor::handle_python_job,
    python_versions::{PyV, PyVAlias},
};

#[cfg(feature = "python")]
use crate::ansible_executor::handle_ansible_job;

#[cfg(feature = "mysql")]
use crate::mysql_executor::do_mysql;

#[cfg(feature = "duckdb")]
use crate::duckdb_executor::do_duckdb;

#[cfg(feature = "oracledb")]
use crate::oracledb_executor::do_oracledb;

#[cfg(feature = "enterprise")]
use crate::dedicated_worker::create_dedicated_worker_map;

#[cfg(feature = "enterprise")]
use crate::snowflake_executor::do_snowflake;

#[cfg(all(feature = "enterprise", feature = "mssql"))]
use crate::mssql_executor::do_mssql;

#[cfg(all(feature = "enterprise", feature = "bigquery"))]
use crate::bigquery_executor::do_bigquery;

#[cfg(feature = "benchmark")]
use windmill_common::bench::{benchmark_init, BenchmarkInfo, BenchmarkIter};

use windmill_common::add_time;

pub const PY310_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "python_3_10");
pub const PY311_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "python_3_11");
pub const PY312_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "python_3_12");
pub const PY313_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "python_3_13");

pub const TAR_JAVA_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "tar/java");

pub const UV_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "uv");
pub const PY_INSTALL_DIR: &str = concatcp!(ROOT_CACHE_DIR, "py_runtime");
pub const TAR_PYBASE_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "tar");
pub const DENO_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "deno");
pub const DENO_CACHE_DIR_DEPS: &str = concatcp!(ROOT_CACHE_DIR, "deno/deps");
pub const DENO_CACHE_DIR_NPM: &str = concatcp!(ROOT_CACHE_DIR, "deno/npm");

pub const GO_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "go");
pub const RUST_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "rust");
pub const NU_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "nu");
pub const CSHARP_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "csharp");

// JAVA
pub const JAVA_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "java");
pub const COURSIER_CACHE_DIR: &str = concatcp!(JAVA_CACHE_DIR, "/coursier-cache");
pub const JAVA_REPOSITORY_DIR: &str = concatcp!(JAVA_CACHE_DIR, "/repository");
// for related places search: ADD_NEW_LANG
pub const BUN_CACHE_DIR: &str = concatcp!(ROOT_CACHE_NOMOUNT_DIR, "bun");
pub const BUN_BUNDLE_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "bun");
pub const BUN_CODEBASE_BUNDLE_CACHE_DIR: &str = concatcp!(ROOT_CACHE_NOMOUNT_DIR, "script_bundle");

pub const GO_BIN_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "gobin");
pub const POWERSHELL_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "powershell");
pub const COMPOSER_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "composer");

const NUM_SECS_PING: u64 = 5;
const NUM_SECS_READINGS: u64 = 60;

const INCLUDE_DEPS_PY_SH_CONTENT: &str = include_str!("../nsjail/download_deps.py.sh");

const WORKER_SHELL_NAP_TIME_DURATION: u64 = 15;
const TIMEOUT_TO_RESET_WORKER_SHELL_NAP_TIME_DURATION: u64 = 2 * 60;

pub const DEFAULT_SLEEP_QUEUE: u64 = 50;

// only 1 native job so that we don't have to worry about concurrency issues on non dedicated native jobs workers
pub const DEFAULT_NATIVE_JOBS: usize = 1;

const VACUUM_PERIOD: u32 = 50000;

// #[cfg(any(target_os = "linux"))]
// const DROP_CACHE_PERIOD: u32 = 1000;

pub const MAX_BUFFERED_DEDICATED_JOBS: usize = 3;

#[cfg(feature = "prometheus")]
lazy_static::lazy_static! {

    static ref WORKER_STARTED: Option<prometheus::IntGauge> = if METRICS_ENABLED.load(Ordering::Relaxed) { Some(prometheus::register_int_gauge!(
        "worker_started",
        "Total number of workers started."
    )
    .unwrap()) } else { None };

    static ref WORKER_UPTIME_OPTS: prometheus::Opts = prometheus::opts!(
        "worker_uptime",
        "Total number of seconds since the worker has started"
    );


    pub static ref WORKER_EXECUTION_COUNT: Arc<RwLock<HashMap<String, IntCounter>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref WORKER_EXECUTION_DURATION_COUNTER: Arc<RwLock<HashMap<String, prometheus::Counter>>> = Arc::new(RwLock::new(HashMap::new()));

    pub static ref WORKER_EXECUTION_DURATION: Arc<RwLock<HashMap<String, prometheus::Histogram>>> = Arc::new(RwLock::new(HashMap::new()));
}

#[cfg(windows)]
const DOTNET_DEFAULT_PATH: &str = "C:\\Program Files\\dotnet\\dotnet.exe";
#[cfg(unix)]
const DOTNET_DEFAULT_PATH: &str = "/usr/bin/dotnet";
pub const SAME_WORKER_REQUIREMENTS: &'static str =
    "SameWorkerSender is required because this job may be part of a flow";

lazy_static::lazy_static! {

    pub static ref SLEEP_QUEUE: u64 = std::env::var("SLEEP_QUEUE")
    .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or_else(|| {
            if std::env::var("MODE").unwrap_or_default() == "agent" {
                1000
            } else {
                DEFAULT_SLEEP_QUEUE * std::env::var("NUM_WORKERS")
                    .ok()
                    .map(|x| x.parse().ok())
                    .flatten()
                    .unwrap_or(2) / 2
            }
        });


    pub static ref DISABLE_NUSER: bool = std::env::var("DISABLE_NUSER")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(false);

    // pub static ref DISABLE_NSJAIL: bool = false;
    pub static ref DISABLE_NSJAIL: bool = std::env::var("DISABLE_NSJAIL")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(true);

    pub static ref KEEP_JOB_DIR: AtomicBool = AtomicBool::new(std::env::var("KEEP_JOB_DIR")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false));

    pub static ref NO_PROXY: Option<String> = std::env::var("no_proxy").ok().or(std::env::var("NO_PROXY").ok());
    pub static ref HTTP_PROXY: Option<String> = std::env::var("http_proxy").ok().or(std::env::var("HTTP_PROXY").ok());
    pub static ref HTTPS_PROXY: Option<String> = std::env::var("https_proxy").ok().or(std::env::var("HTTPS_PROXY").ok());

    pub static ref PROXY_ENVS: Vec<(&'static str, String)> = {
        let mut proxy_env = Vec::new();
        if let Some(no_proxy) = NO_PROXY.as_ref() {
            proxy_env.push(("NO_PROXY", no_proxy.to_string()));
        } else if HTTPS_PROXY.is_some() || HTTP_PROXY.is_some() {
            proxy_env.push(("NO_PROXY", "localhost,127.0.0.1".to_string()));
        }
        if let Some(http_proxy) = HTTP_PROXY.as_ref() {
            proxy_env.push(("HTTP_PROXY", http_proxy.to_string()));
        }
        if let Some(https_proxy) = HTTPS_PROXY.as_ref() {
            proxy_env.push(("HTTPS_PROXY", https_proxy.to_string()));
        }
        proxy_env
    };
    pub static ref DENO_PATH: String = std::env::var("DENO_PATH").unwrap_or_else(|_| "/usr/bin/deno".to_string());
    pub static ref BUN_PATH: String = std::env::var("BUN_PATH").unwrap_or_else(|_| "/usr/bin/bun".to_string());
    pub static ref NPM_PATH: String = std::env::var("NPM_PATH").unwrap_or_else(|_| "/usr/bin/npm".to_string());
    pub static ref NODE_BIN_PATH: String = std::env::var("NODE_BIN_PATH").unwrap_or_else(|_| "/usr/bin/node".to_string());
    pub static ref POWERSHELL_PATH: String = std::env::var("POWERSHELL_PATH").unwrap_or_else(|_| "/usr/bin/pwsh".to_string());
    pub static ref PHP_PATH: String = std::env::var("PHP_PATH").unwrap_or_else(|_| "/usr/bin/php".to_string());
    pub static ref COMPOSER_PATH: String = std::env::var("COMPOSER_PATH").unwrap_or_else(|_| "/usr/bin/composer".to_string());
    pub static ref DOTNET_PATH: String = std::env::var("DOTNET_PATH").unwrap_or_else(|_| DOTNET_DEFAULT_PATH.to_string());
    pub static ref NSJAIL_PATH: String = std::env::var("NSJAIL_PATH").unwrap_or_else(|_| "nsjail".to_string());
    pub static ref PATH_ENV: String = std::env::var("PATH").unwrap_or_else(|_| String::new());
    pub static ref HOME_ENV: String = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    pub static ref GIT_PATH: String = std::env::var("GIT_PATH").unwrap_or_else(|_| "/usr/bin/git".to_string());

    pub static ref NODE_PATH: Option<String> = std::env::var("NODE_PATH").ok();

    pub static ref TZ_ENV: String = std::env::var("TZ").unwrap_or_else(|_| String::new());
    pub static ref GOPRIVATE: Option<String> = std::env::var("GOPRIVATE").ok();
    pub static ref GOPROXY: Option<String> = std::env::var("GOPROXY").ok();
    pub static ref NETRC: Option<String> = std::env::var("NETRC").ok();


    pub static ref NPM_CONFIG_REGISTRY: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref BUNFIG_INSTALL_SCOPES: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref NUGET_CONFIG: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref MAVEN_REPOS: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref NO_DEFAULT_MAVEN: AtomicBool = AtomicBool::new(std::env::var("NO_DEFAULT_MAVEN")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false));

    pub static ref PIP_EXTRA_INDEX_URL: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref PIP_INDEX_URL: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref INSTANCE_PYTHON_VERSION: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref JOB_DEFAULT_TIMEOUT: Arc<RwLock<Option<i32>>> = Arc::new(RwLock::new(None));



    pub static ref MAX_WAIT_FOR_SIGINT: u64 = std::env::var("MAX_WAIT_FOR_SIGINT")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or_else(|| 0);

    pub static ref MAX_WAIT_FOR_SIGTERM: u64 = std::env::var("MAX_WAIT_FOR_SIGTERM")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or_else(|| 5);

    pub static ref MAX_TIMEOUT_DURATION: Duration = Duration::from_secs(*MAX_TIMEOUT);


    pub static ref GLOBAL_CACHE_INTERVAL: u64 = std::env::var("GLOBAL_CACHE_INTERVAL")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or(60 * 10);


    pub static ref EXIT_AFTER_NO_JOB_FOR_SECS: Option<u64> = std::env::var("EXIT_AFTER_NO_JOB_FOR_SECS")
        .ok()
        .and_then(|x| x.parse::<u64>().ok());


    pub static ref REFRESH_CGROUP_READINGS: bool = std::env::var("REFRESH_CGROUP_READINGS")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(false);

    pub static ref OUTSTANDING_WAIT_TIME_THRESHOLD_MS: i64 = std::env::var("OUTSTANDING_WAIT_TIME_THRESHOLD_MS")
        .ok()
        .and_then(|x| x.parse::<i64>().ok())
        .unwrap_or(1000);
}

type Envs = Vec<(String, String)>;

#[cfg(windows)]
lazy_static::lazy_static! {
    pub static ref SYSTEM_ROOT: String = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
    pub static ref USERPROFILE_ENV: String = std::env::var("USERPROFILE").unwrap_or_else(|_| "/tmp".to_string());
    static ref TMP: String = std::env::var("TMP").unwrap_or_else(|_| "/tmp".to_string());
    static ref LOCALAPPDATA: String = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| format!("{}\\AppData\\Local", HOME_ENV.as_str()));
    pub static ref WIN_ENVS: Envs = vec![
        ("SystemRoot".into(), SYSTEM_ROOT.clone()),
        ("USERPROFILE".into(), USERPROFILE_ENV.clone()),
        ("TMP".into(), TMP.clone()),
        ("LOCALAPPDATA".into(), LOCALAPPDATA.clone())
    ];

}

#[cfg(not(windows))]
lazy_static::lazy_static! {
    pub static ref WIN_ENVS: Envs = vec![];
}

#[derive(Debug)]
pub enum NextJob {
    Sql(PulledJob),
    Http(JobAndPerms),
}

impl NextJob {
    pub fn job(self) -> MiniPulledJob {
        match self {
            NextJob::Sql(job) => job.job,
            NextJob::Http(job) => job.job,
        }
    }
}

impl std::ops::Deref for NextJob {
    type Target = MiniPulledJob;
    fn deref(&self) -> &Self::Target {
        match self {
            NextJob::Sql(job) => &job.job,
            NextJob::Http(job) => &job.job,
        }
    }
}

//only matter if CLOUD_HOSTED
pub const MAX_RESULT_SIZE: usize = 1024 * 1024 * 2; // 2MB

pub const INIT_SCRIPT_TAG: &str = "init_script";

#[derive(Clone)]
pub struct SameWorkerSender(pub Sender<SameWorkerPayload>, pub Arc<AtomicU16>);

#[allow(dead_code)]
#[derive(Clone)]
pub enum JobCompletedSender {
    Sql(SqlJobCompletedSender),
    Http(HttpClient),
    NeverUsed,
}

#[derive(Clone)]
pub struct SqlJobCompletedSender {
    sender: flume::Sender<SendResult>,
    unbounded_sender: flume::Sender<SendResult>,
    killpill_tx: broadcast::Sender<()>,
}

pub struct JobCompletedReceiver {
    pub bounded_rx: flume::Receiver<SendResult>,
    pub killpill_rx: broadcast::Receiver<()>,
    pub unbounded_rx: flume::Receiver<SendResult>,
}

impl JobCompletedReceiver {
    pub fn clone(&self) -> Self {
        Self {
            bounded_rx: self.bounded_rx.clone(),
            killpill_rx: self.killpill_rx.resubscribe(),
            unbounded_rx: self.unbounded_rx.clone(),
        }
    }
}

impl JobCompletedSender {
    pub fn new_job_completed_sender_sql(buffer_size: u8) -> (Self, JobCompletedReceiver) {
        let (sender, receiver) = flume::bounded::<SendResult>(buffer_size as usize);
        let (unbounded_sender, unbounded_rx) = flume::unbounded::<SendResult>();
        let (killpill_tx, killpill_rx) = broadcast::channel::<()>(10);
        (
            Self::Sql(SqlJobCompletedSender { sender, unbounded_sender, killpill_tx }),
            JobCompletedReceiver { bounded_rx: receiver, killpill_rx, unbounded_rx },
        )
    }

    pub fn new(conn: &Connection, buffer_size: u8) -> (Self, Option<JobCompletedReceiver>) {
        match conn {
            Connection::Sql(_) => {
                let result = Self::new_job_completed_sender_sql(buffer_size);
                (result.0, Some(result.1))
            }
            Connection::Http(client) => (Self::Http(client.clone()), None),
        }
    }

    pub fn new_never_used() -> (Self, Option<Receiver<SendResult>>) {
        (Self::NeverUsed, None)
    }

    pub async fn send_job(&self, jc: JobCompleted, wait_for_capacity: bool) -> anyhow::Result<()> {
        match self {
            Self::Sql(SqlJobCompletedSender { sender, unbounded_sender, .. }) => {
                if wait_for_capacity {
                    sender
                } else {
                    unbounded_sender
                }
                .send_async(SendResult::JobCompleted(jc))
                .await
                .map_err(|_e| {
                    anyhow::anyhow!("Failed to send job completed to background processor")
                })
            }
            Self::Http(client) => {
                crate::agent_workers::send_result(client, jc).await?;
                Ok(())
            }
            Self::NeverUsed => {
                tracing::error!(
                    "Sending job completed to NeverUsed JobCompletedSender, this should not happen"
                );
                Ok(())
            }
        }
    }

    pub async fn send(
        &self,
        send_result: SendResult,
        wait_for_capacity: bool,
    ) -> Result<(), flume::SendError<SendResult>> {
        match self {
            Self::Sql(SqlJobCompletedSender { sender, unbounded_sender, .. }) => {
                if wait_for_capacity {
                    sender.send_async(send_result).await
                } else {
                    unbounded_sender.send_async(send_result).await
                }
            }
            Self::Http(_) => {
                tracing::error!("Sending job completed to http client, this should not happen");
                Ok(())
            }
            Self::NeverUsed => {
                tracing::error!(
                    "Sending job completed to NeverUsed JobCompletedSender, this should not happen"
                );
                Ok(())
            }
        }
    }

    pub async fn kill(&self) -> Result<(), broadcast::error::SendError<()>> {
        match self {
            Self::Sql(SqlJobCompletedSender { killpill_tx, .. }) => {
                tracing::info!("Sending killpill to bg processors");
                killpill_tx.send(())?;
                Ok(())
            }
            Self::Http(_) => {
                tracing::error!("Sending kill to http client, this should not happen");
                Ok(())
            }
            Self::NeverUsed => {
                tracing::error!(
                    "Sending kill to NeverUsed JobCompletedSender, this should not happen"
                );
                Ok(())
            }
        }
    }
}

impl SameWorkerSender {
    pub async fn send(
        &self,
        payload: SameWorkerPayload,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<SameWorkerPayload>> {
        self.1.fetch_add(1, Ordering::Relaxed);
        self.0.send(payload).await
    }
}

// on linux, we drop caches every DROP_CACHE_PERIOD to avoid OOM killer believing we are using too much memory just because we create lots of files when executing jobs
#[cfg(any(target_os = "linux"))]
pub async fn drop_cache() {
    tracing::info!("Syncing and dropping linux file caches to reduce memory usage");
    // Run the sync command
    if let Err(e) = tokio::process::Command::new("sync").status().await {
        tracing::error!("Failed to run sync command: {}", e);
        return;
    }

    // Open /proc/sys/vm/drop_caches for writing asynchronously
    match tokio::fs::File::create("/proc/sys/vm/drop_caches").await {
        Ok(mut file) => {
            // Write '3' to the file to drop caches
            if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut file, b"3").await {
                tracing::warn!("Failed to write to /proc/sys/vm/drop_caches (expected to work only in privileged mode, only required to forcefully drop the cache to avoid spurrious oom killer): {}", e);
            }
        }
        Err(e) => {
            tracing::warn!("Failed to open /proc/sys/vm/drop_caches (expected to work only in privileged mode, only required to forcefully drop the cache to avoid spurrious oom killer):: {}", e);
        }
    }
}

async fn insert_wait_time(
    job_id: Uuid,
    root_job_id: Option<Uuid>,
    db: &DB,
    wait_time: i64,
) -> sqlx::error::Result<()> {
    sqlx::query!(
                "INSERT INTO outstanding_wait_time(job_id, self_wait_time_ms) VALUES ($1, $2)
                    ON CONFLICT (job_id) DO UPDATE SET self_wait_time_ms = EXCLUDED.self_wait_time_ms",
                job_id,
                wait_time
            )
            .execute(db)
            .await?;

    if let Some(root_id) = root_job_id {
        // TODO: queued_job.root_job is not guaranteed to be the true root job (e.g. parallel flow
        // subflows). So this is currently incorrect for those cases
        sqlx::query!(
            "INSERT INTO outstanding_wait_time(job_id, aggregate_wait_time_ms) VALUES ($1, $2)
                ON CONFLICT (job_id) DO UPDATE SET aggregate_wait_time_ms =
                COALESCE(outstanding_wait_time.aggregate_wait_time_ms, 0) + EXCLUDED.aggregate_wait_time_ms",
            root_id,
            wait_time
                )
                .execute(db)
                .await?;
    }
    Ok(())
}

fn add_outstanding_wait_time(
    conn: &Connection,
    queued_job: &MiniPulledJob,
    waiting_threshold: i64,
) -> () {
    let wait_time;

    if let Some(started_time) = queued_job.started_at {
        wait_time = (started_time - queued_job.scheduled_for).num_milliseconds();
    } else {
        return;
    }

    if wait_time < waiting_threshold {
        return;
    }

    let job_id = queued_job.id;
    let root_job_id = queued_job.flow_innermost_root_job;
    let conn = conn.clone();

    if let Some(db) = conn.as_sql() {
        let db = db.clone();
        tokio::spawn(async move {
            match insert_wait_time(job_id, root_job_id, &db, wait_time).await {
                Ok(()) => tracing::warn!("job {job_id} waited for an executor for a significant amount of time. Recording value wait_time={}ms", wait_time),
                Err(e) => tracing::error!("Failed to insert outstanding wait time: {}", e),
            }
        }.in_current_span());
    }
}

async fn extract_job_and_perms(job: NextJob, conn: &Connection) -> JobAndPerms {
    match (job, conn) {
        (NextJob::Sql(job), Connection::Sql(db)) => job.get_job_and_perms(db).await,
        (NextJob::Sql(_), Connection::Http(_)) => panic!("sql job on http connection"),
        (NextJob::Http(job), _) => job,
    }
}

fn create_span(arc_job: &Arc<MiniPulledJob>, worker_name: &str, hostname: &str) -> Span {
    let span = tracing::span!(tracing::Level::INFO, "job",
        job_id = %arc_job.id, root_job = field::Empty, workspace_id = %arc_job.workspace_id,  worker = %worker_name, hostname = %hostname, tag = %arc_job.tag,
        language = field::Empty,
        script_path = field::Empty, flow_step_id = field::Empty, parent_job = field::Empty,
        otel.name = field::Empty);

    let rj = arc_job.flow_innermost_root_job.unwrap_or(arc_job.id);

    if let Some(lg) = arc_job.script_lang.as_ref() {
        span.record("language", lg.as_str());
    }
    if let Some(step_id) = arc_job.flow_step_id.as_ref() {
        span.record("otel.name", format!("job {}", step_id).as_str());
        span.record("flow_step_id", step_id.as_str());
    } else {
        span.record("otel.name", "job");
    }
    if let Some(parent_job) = arc_job.parent_job.as_ref() {
        span.record("parent_job", parent_job.to_string().as_str());
    }
    if let Some(script_path) = arc_job.runnable_path.as_ref() {
        span.record("script_path", script_path.as_str());
    }
    if let Some(root_job) = arc_job.flow_innermost_root_job.as_ref() {
        span.record("root_job", root_job.to_string().as_str());
    }

    windmill_common::otel_oss::set_span_parent(&span, &rj);
    span
}

pub async fn handle_all_job_kind_error(
    conn: &Connection,
    authed_client: &AuthedClient,
    job: Arc<MiniPulledJob>,
    err: Error,
    same_worker_tx: Option<&SameWorkerSender>,
    worker_dir: &str,
    worker_name: &str,
    job_completed_tx: JobCompletedSender,
    #[cfg(feature = "benchmark")] bench: &mut BenchmarkIter,
) {
    match conn {
        Connection::Sql(db) => {
            handle_job_error(
                db,
                authed_client,
                job.as_ref(),
                0,
                None,
                err,
                false,
                same_worker_tx,
                &worker_dir,
                &worker_name,
                job_completed_tx.clone(),
                #[cfg(feature = "benchmark")]
                bench,
            )
            .await;
        }
        Connection::Http(_) => {
            job_completed_tx
                .send_job(
                    JobCompleted {
                        preprocessed_args: None,
                        job: job.clone(),
                        result: Arc::new(windmill_common::worker::to_raw_value(&error_to_value(
                            err,
                        ))),
                        result_columns: None,
                        mem_peak: 0,
                        canceled_by: None,
                        success: false,
                        cached_res_path: None,
                        token: authed_client.token.clone(),
                        duration: None,
                    },
                    false,
                )
                .await
                .expect("send job completed");
        }
    }
}

pub fn start_interactive_worker_shell(
    conn: Connection,
    hostname: String,
    worker_name: String,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
    job_completed_tx: JobCompletedSender,
    base_internal_url: String,
    worker_dir: String,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut occupancy_metrics = OccupancyMetrics::new(Instant::now());

        let mut last_executed_job: Option<Instant> =
            Instant::now().checked_sub(Duration::from_millis(2500));

        loop {
            if let Ok(_) = killpill_rx.try_recv() {
                break;
            } else {
                let pulled_job = match &conn {
                    Connection::Sql(db) => {
                        let query = ("".to_string(), make_pull_query(&[hostname.to_owned()]));

                        #[cfg(feature = "benchmark")]
                        let mut bench = windmill_common::bench::BenchmarkIter::new();
                        let job = pull(
                            &db,
                            false,
                            &worker_name,
                            Some(&query),
                            #[cfg(feature = "benchmark")]
                            &mut bench,
                        )
                        .await;

                        job.map(|x| x.job.map(NextJob::Sql))
                    }
                    Connection::Http(client) => {
                        crate::agent_workers::pull_job(&client, None, Some(true))
                            .await
                            .map_err(|e| error::Error::InternalErr(e.to_string()))
                            .map(|x| x.map(|y| NextJob::Http(y)))
                    }
                };

                match pulled_job {
                    Ok(Some(job)) => {
                        tracing::debug!(worker = %worker_name, hostname = %hostname, "started handling of job {}", job.id);

                        let job_dir = create_job_dir(&worker_dir, job.id).await;
                        #[cfg(feature = "benchmark")]
                        let mut bench = windmill_common::bench::BenchmarkIter::new();

                        let JobAndPerms {
                            job,
                            raw_code,
                            raw_lock,
                            raw_flow,
                            parent_runnable_path,
                            token,
                            precomputed_agent_info: precomputed_bundle,
                        } = extract_job_and_perms(job, &conn).await;

                        let authed_client = AuthedClient::new(
                            base_internal_url.to_owned(),
                            job.workspace_id.clone(),
                            token,
                            None,
                        );

                        let arc_job = Arc::new(job);

                        let _ = handle_queued_job(
                            arc_job.clone(),
                            raw_code,
                            raw_lock,
                            raw_flow,
                            parent_runnable_path,
                            &conn,
                            &authed_client,
                            &hostname,
                            &worker_name,
                            &worker_dir,
                            &job_dir,
                            None,
                            &base_internal_url,
                            job_completed_tx.clone(),
                            &mut occupancy_metrics,
                            &mut killpill_rx,
                            precomputed_bundle,
                            #[cfg(feature = "benchmark")]
                            &mut bench,
                        )
                        .await;

                        last_executed_job = Some(Instant::now());
                    }
                    Ok(None) => {
                        let now = Instant::now();
                        match last_executed_job {
                            Some(last)
                                if now.duration_since(last).as_secs()
                                    > TIMEOUT_TO_RESET_WORKER_SHELL_NAP_TIME_DURATION =>
                            {
                                tokio::time::sleep(Duration::from_secs(
                                    WORKER_SHELL_NAP_TIME_DURATION,
                                ))
                                .await;
                            }
                            _ => {
                                tokio::time::sleep(Duration::from_millis(*SLEEP_QUEUE * 10)).await;
                            }
                        }
                    }

                    Err(err) => {
                        tracing::error!(worker = %worker_name, hostname = %hostname, "Failed to pull jobs: {}", err);
                        tokio::time::sleep(Duration::from_millis(*SLEEP_QUEUE * 20)).await;
                    }
                };
            }
        }
    })
}

async fn create_job_dir(worker_directory: &str, job_id: impl Display) -> String {
    let job_dir_path = format!("{}/{}", worker_directory, job_id);

    create_directory_async(&job_dir_path).await;

    job_dir_path
}

pub async fn run_worker(
    conn: &Connection,
    hostname: &str,
    worker_name: String,
    i_worker: u64,
    _num_workers: u32,
    ip: &str,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
    killpill_tx: KillpillSender,
    base_internal_url: &str,
) {
    #[cfg(not(feature = "enterprise"))]
    if !*DISABLE_NSJAIL {
        tracing::warn!(
            worker = %worker_name, hostname = %hostname,
            "NSJAIL to sandbox process in untrusted environments is an enterprise feature but allowed to be used for testing purposes"
        );
    }

    let start_time = Instant::now();

    let worker_dir = format!("{TMP_DIR}/{worker_name}");
    tracing::debug!(worker = %worker_name, hostname = %hostname, worker_dir = %worker_dir, "Creating worker dir");

    #[cfg(feature = "python")]
    {
        let (conn, worker_name, hostname, worker_dir) = (
            conn.clone(),
            worker_name.clone(),
            hostname.to_owned(),
            worker_dir.clone(),
        );
        tokio::spawn(async move {
            if let Err(e) = PyV::gravitational_version(&Uuid::nil(), "", Some(conn.clone()))
                .await
                .try_get_python(&Uuid::nil(), &mut 0, &conn, &worker_name, "", &mut None)
                .await
            {
                tracing::error!(
                    worker = %worker_name,
                    hostname = %hostname,
                    worker_dir = %worker_dir,
                    "Cannot preinstall or find Instance Python version to worker: {e}"//
                );
            }
            if let Err(e) = PyV::from(PyVAlias::Py311)
                .try_get_python(&Uuid::nil(), &mut 0, &conn, &worker_name, "", &mut None)
                .await
            {
                tracing::error!(
                    worker = %worker_name,
                    hostname = %hostname,
                    worker_dir = %worker_dir,
                    "Cannot preinstall or find default 311 version to worker: {e}"//
                );
            }
        });
    }

    if let Some(ref netrc) = *NETRC {
        tracing::info!(worker = %worker_name, hostname = %hostname, "Writing netrc at {}/.netrc", HOME_ENV.as_str());
        write_file(&HOME_ENV, ".netrc", netrc).expect("could not write netrc");
    }

    create_directory_async(&worker_dir).await;

    if !*DISABLE_NSJAIL {
        let _ = write_file(
            &worker_dir,
            "download_deps.py.sh",
            INCLUDE_DEPS_PY_SH_CONTENT,
        );
    }

    let mut last_ping = Instant::now() - Duration::from_secs(NUM_SECS_PING + 1);

    insert_ping(hostname, &worker_name, ip, conn)
        .await
        .expect("initial ping could be sent");

    #[cfg(feature = "prometheus")]
    let uptime_metric = if METRICS_ENABLED.load(Ordering::Relaxed) {
        Some(
            prometheus::register_counter!(WORKER_UPTIME_OPTS
                .clone()
                .const_label("name", &worker_name))
            .unwrap(),
        )
    } else {
        None
    };

    #[cfg(feature = "prometheus")]
    let worker_sleep_duration_counter = if METRICS_ENABLED.load(Ordering::Relaxed) {
        Some(
            prometheus::register_counter!(prometheus::opts!(
                "worker_sleep_duration_counter",
                "Total number of seconds spent sleeping between pulling jobs from the queue"
            )
            .const_label("name", &worker_name))
            .expect("register prometheus metric"),
        )
    } else {
        None
    };

    #[cfg(feature = "prometheus")]
    let worker_pull_duration = if METRICS_ENABLED.load(Ordering::Relaxed) {
        Some(
            prometheus::register_histogram!(prometheus::HistogramOpts::new(
                "worker_pull_duration",
                "Duration pulling next job",
            )
            .const_label("name", &worker_name)
            .const_label("has_job", "true"),)
            .expect("register prometheus metric"),
        )
    } else {
        None
    };

    #[cfg(feature = "prometheus")]
    let worker_pull_duration_empty = if METRICS_ENABLED.load(Ordering::Relaxed) {
        Some(
            prometheus::register_histogram!(prometheus::HistogramOpts::new(
                "worker_pull_duration",
                "Duration pulling next job",
            )
            .const_label("name", &worker_name)
            .const_label("has_job", "false"),)
            .expect("register prometheus metric"),
        )
    } else {
        None
    };

    #[cfg(feature = "prometheus")]
    let _worker_save_completed_job_duration = if METRICS_DEBUG_ENABLED.load(Ordering::Relaxed)
        && METRICS_ENABLED.load(Ordering::Relaxed)
    {
        Some(Arc::new(
            prometheus::register_histogram!(prometheus::HistogramOpts::new(
                "worker_save_duration",
                "Duration sending job to completed job channel",
            )
            .const_label("name", &worker_name),)
            .expect("register prometheus metric"),
        ))
    } else {
        None
    };

    #[cfg(feature = "prometheus")]
    let worker_pull_duration_counter_empty =
        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            Some(
                prometheus::register_counter!(prometheus::opts!(
        "worker_pull_duration_counter",
        "Total number of seconds spent pulling jobs (if growing large the db is undersized)"
    )
                .const_label("name", &worker_name)
                .const_label("has_job", "false"))
                .expect("register prometheus metric"),
            )
        } else {
            None
        };

    #[cfg(feature = "prometheus")]
    let worker_pull_duration_counter = if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed)
    {
        Some(
            prometheus::register_counter!(prometheus::opts!(
        "worker_pull_duration_counter",
        "Total number of seconds spent pulling jobs (if growing large the db is undersized)"
    )
            .const_label("name", &worker_name)
            .const_label("has_job", "true"))
            .expect("register prometheus metric"),
        )
    } else {
        None
    };

    #[cfg(feature = "prometheus")]
    let worker_pull_over_500_counter_empty =
        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            Some(
                prometheus::register_counter!(prometheus::opts!(
                    "worker_pull_slow_counter",
                    "Total number of pull being too slow (if growing large the db is undersized)"
                )
                .const_label("name", &worker_name)
                .const_label("over", "500")
                .const_label("has_job", "false"))
                .expect("register prometheus metric"),
            )
        } else {
            None
        };

    #[cfg(feature = "prometheus")]
    let worker_pull_over_500_counter = if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed)
    {
        Some(
            prometheus::register_counter!(prometheus::opts!(
                "worker_pull_slow_counter",
                "Total number of pull being too slow (if growing large the db is undersized)"
            )
            .const_label("name", &worker_name)
            .const_label("over", "500")
            .const_label("has_job", "true"))
            .expect("register prometheus metric"),
        )
    } else {
        None
    };

    #[cfg(feature = "prometheus")]
    let worker_pull_over_100_counter_empty =
        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            Some(
                prometheus::register_counter!(prometheus::opts!(
                    "worker_pull_slow_counter",
                    "Total number of pull being too slow (if growing large the db is undersized)"
                )
                .const_label("name", &worker_name)
                .const_label("over", "100")
                .const_label("has_job", "false"))
                .expect("register prometheus metric"),
            )
        } else {
            None
        };

    #[cfg(feature = "prometheus")]
    let worker_pull_over_100_counter = if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed)
    {
        Some(
            prometheus::register_counter!(prometheus::opts!(
                "worker_pull_slow_counter",
                "Total number of pull being too slow (if growing large the db is undersized)"
            )
            .const_label("name", &worker_name)
            .const_label("over", "100")
            .const_label("has_job", "true"))
            .expect("register prometheus metric"),
        )
    } else {
        None
    };

    #[cfg(feature = "prometheus")]
    let worker_busy: Option<prometheus::IntGauge> =
        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            Some(
                prometheus::register_int_gauge!(prometheus::Opts::new(
                    "worker_busy",
                    "Is the worker busy executing a job?",
                )
                .const_label("name", &worker_name))
                .unwrap(),
            )
        } else {
            None
        };

    // let worker_resource = &[
    //     KeyValue::new("hostname", hostname.to_string()),
    //     KeyValue::new("worker", worker_name.to_string()),
    // ];
    // // Create a meter from the above MeterProvider.
    // let meter = global::meter("windmill");
    // let counter = meter.u64_counter("jobs.execution").build();

    let mut occupancy_metrics = OccupancyMetrics::new(start_time);
    let mut jobs_executed = 0;

    let is_dedicated_worker: bool = WORKER_CONFIG.read().await.dedicated_worker.is_some();

    #[cfg(feature = "benchmark")]
    let benchmark_jobs: i32 = std::env::var("BENCHMARK_JOBS")
        .unwrap_or("5000".to_string())
        .parse::<i32>()
        .unwrap();

    #[cfg(feature = "benchmark")]
    {
        if let Some(db) = conn.as_sql() {
            benchmark_init(benchmark_jobs, db).await;
        }
    }

    #[cfg(feature = "prometheus")]
    if let Some(ws) = WORKER_STARTED.as_ref() {
        ws.inc();
    }

    let (same_worker_tx, mut same_worker_rx) = mpsc::channel::<SameWorkerPayload>(5);

    let (job_completed_tx, job_completed_rx) = JobCompletedSender::new(&conn, 10);

    let same_worker_queue_size = Arc::new(AtomicU16::new(0));
    let same_worker_tx = SameWorkerSender(same_worker_tx, same_worker_queue_size.clone());
    let job_completed_processor_is_done =
        Arc::new(AtomicBool::new(matches!(conn, Connection::Http(_))));

    let send_result = match (conn, job_completed_rx) {
        (Connection::Sql(db), Some(job_completed_receiver)) => Some(start_background_processor(
            job_completed_receiver,
            job_completed_tx.clone(),
            same_worker_queue_size.clone(),
            job_completed_processor_is_done.clone(),
            base_internal_url.to_string(),
            db.clone(),
            worker_dir.clone(),
            same_worker_tx.clone(),
            worker_name.clone(),
            killpill_tx.clone(),
            is_dedicated_worker,
        )),
        _ => None,
    };
    // If we're the first worker to run, we start another background process that listens for a specific tag.
    // This tag is associated only with jobs using Bash as the script language.
    // For agent workers, the expected tag format is the worker name suffixed with "-ssh".
    // For regular workers, the tag is simply the machine's hostname and if not found the randomly generated hostname.
    let interactive_shell = if i_worker == 1 {
        let it_shell = start_interactive_worker_shell(
            conn.clone(),
            hostname.to_owned(),
            worker_name.clone(),
            killpill_rx.resubscribe(),
            job_completed_tx.clone(),
            base_internal_url.to_owned(),
            worker_dir.clone(),
        );

        Some(it_shell)
    } else {
        None
    };

    let mut last_executed_job: Option<Instant> = None;

    #[cfg(feature = "benchmark")]
    let mut started = false;

    #[cfg(feature = "benchmark")]
    let mut infos = BenchmarkInfo::new();

    let vacuum_shift = rand::rng().random_range(0..VACUUM_PERIOD);

    IS_READY.store(true, Ordering::Relaxed);
    if let Some(token) = DECODED_AGENT_TOKEN.as_ref() {
        tracing::info!(
            worker = %worker_name, hostname = %hostname,
            "listening for jobs, agent mode, tags: {:?}",
            token.tags
        );
    } else {
        tracing::info!(
            worker = %worker_name, hostname = %hostname,
            "listening for jobs, WORKER_GROUP: {}, config: {:?}",
            *WORKER_GROUP,
            WORKER_CONFIG.read().await
        );
    }

    // (dedi_path, dedicated_worker_tx, dedicated_worker_handle)
    // Option<Sender<Arc<QueuedJob>>>,
    // Option<JoinHandle<()>>,

    #[cfg(feature = "enterprise")]
    let (dedicated_workers, is_flow_worker, dedicated_handles): (
        HashMap<String, Sender<Arc<MiniPulledJob>>>,
        bool,
        Vec<JoinHandle<()>>,
    ) = match conn {
        Connection::Sql(pool) => {
            create_dedicated_worker_map(
                &killpill_tx,
                &killpill_rx,
                pool,
                &worker_dir,
                base_internal_url,
                &worker_name,
                &job_completed_tx,
            )
            .await
        }
        Connection::Http(_) => (HashMap::new(), false, vec![]),
    };

    #[cfg(not(feature = "enterprise"))]
    let (dedicated_workers, is_flow_worker, dedicated_handles): (
        HashMap<String, Sender<Arc<MiniPulledJob>>>,
        bool,
        Vec<JoinHandle<()>>,
    ) = (HashMap::new(), false, vec![]);

    if i_worker == 1 {
        if let Err(e) = queue_init_bash_maybe(conn, same_worker_tx.clone(), &worker_name).await {
            killpill_tx.send();
            tracing::error!(worker = %worker_name, hostname = %hostname, "Error queuing init bash script for worker {worker_name}: {e:#}");
            return;
        }
    }

    #[cfg(feature = "prometheus")]
    let _worker_dedicated_channel_queue_send_duration = {
        if is_dedicated_worker
            && METRICS_DEBUG_ENABLED.load(Ordering::Relaxed)
            && METRICS_ENABLED.load(Ordering::Relaxed)
        {
            Some(Arc::new(
                prometheus::register_histogram!(prometheus::HistogramOpts::new(
                    "worker_dedicated_worker_channel_send_duration",
                    "Duration sending job to dedicated worker channel",
                )
                .const_label("name", &worker_name),)
                .expect("register prometheus metric"),
            ))
        } else {
            None
        }
    };
    let mut suspend_first_success = false;
    let mut last_reading = Instant::now() - Duration::from_secs(NUM_SECS_READINGS + 1);
    let mut last_30jobs_suspended = 0;
    let mut last_suspend_first = Instant::now();
    let mut killed_but_draining_same_worker_jobs = false;

    let mut killpill_rx2 = killpill_rx.resubscribe();
    loop {
        #[cfg(feature = "enterprise")]
        {
            if let Ok(_) = killpill_rx.try_recv() {
                tracing::info!(worker = %worker_name, hostname = %hostname, "killpill received on worker waiting for valid key");
                if send_result.is_some() {
                    job_completed_tx
                        .kill()
                        .await
                        .expect("send kill to job completed tx");
                }
                break;
            }
            let valid_key = *LICENSE_KEY_VALID.read().await;

            if !valid_key {
                tracing::error!(
                    worker = %worker_name, hostname = %hostname,
                    "Invalid license key, workers require a valid license key, sleeping for 10s waiting for valid key to be set"
                );
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        }

        #[cfg(feature = "benchmark")]
        let mut bench = BenchmarkIter::new();

        #[cfg(feature = "prometheus")]
        if let Some(wk) = worker_busy.as_ref() {
            wk.set(0);
            tracing::debug!(worker = %worker_name, hostname = %hostname, "set worker busy to 0");
        }

        occupancy_metrics.running_job_started_at = None;

        #[cfg(feature = "prometheus")]
        if let Some(ref um) = uptime_metric {
            um.inc_by(
                ((start_time.elapsed().as_millis() as f64) / 1000.0 - um.get())
                    .try_into()
                    .unwrap(),
            );
            tracing::debug!(worker = %worker_name, hostname = %hostname, "set uptime metric");
        }

        if last_ping.elapsed().as_secs() > NUM_SECS_PING {
            let read_cgroups =
                *REFRESH_CGROUP_READINGS && last_reading.elapsed().as_secs() > NUM_SECS_READINGS;
            update_worker_ping_full(
                &conn,
                read_cgroups,
                jobs_executed,
                &worker_name,
                &hostname,
                &mut occupancy_metrics,
                &killpill_tx,
            )
            .await;

            if read_cgroups {
                last_reading = Instant::now();
            }
            last_ping = Instant::now();
        }

        if (jobs_executed as u32 + vacuum_shift) % VACUUM_PERIOD == 0 {
            queue_vacuum(&conn, &worker_name, &hostname).await;
            jobs_executed += 1;
        }

        // #[cfg(any(target_os = "linux"))]
        // if (jobs_executed as u32 + 1) % DROP_CACHE_PERIOD == 0 {
        //     drop_cache().await;
        //     jobs_executed += 1;
        // }

        #[cfg(feature = "benchmark")]
        if benchmark_jobs > 0 && infos.iters == benchmark_jobs as u64 {
            tracing::info!("benchmark finished, exiting");
            job_completed_tx
                .kill()
                .await
                .expect("send kill to job completed tx");
            break;
        } else {
            tracing::info!("benchmark not finished, still pulling jobs {}", infos.iters);
        }

        let next_job = {
            // println!("2: {:?}",  instant.elapsed());
            #[cfg(feature = "benchmark")]
            if !started {
                started = true
            }

            if let Ok(same_worker_job) = same_worker_rx.try_recv() {
                same_worker_queue_size.fetch_sub(1, Ordering::SeqCst);
                tracing::debug!(
                    worker = %worker_name, hostname = %hostname,
                    "received {} from same worker channel",
                    same_worker_job.job_id
                );

                match &conn {
                    Connection::Sql(db) => {
                        let job = get_same_worker_job(db, &same_worker_job).await;
                        // tracing::error!("r: {:?}", r);
                        if job.is_err() && !same_worker_job.recoverable {
                            tracing::error!(
                                worker = %worker_name, hostname = %hostname,
                                "failed to fetch same_worker job on a non recoverable job, exiting: {job:?}",
                            );
                            job_completed_tx
                                .kill()
                                .await
                                .expect("send kill to job completed tx");
                            break;
                        } else {
                            job.map(|x| x.map(NextJob::Sql))
                        }
                    }
                    Connection::Http(client) => client
                        .post(
                            &format!(
                                "/api/agent_workers/same_worker_job/{}",
                                same_worker_job.job_id
                            ),
                            None,
                            &same_worker_job,
                        )
                        .await
                        .map_err(|e| error::Error::InternalErr(e.to_string()))
                        .map(|x: Option<JobAndPerms>| x.map(|y| NextJob::Http(y))),
                }
            } else if let Ok(_) = killpill_rx.try_recv() {
                if !killed_but_draining_same_worker_jobs {
                    killed_but_draining_same_worker_jobs = true;
                    job_completed_tx
                        .kill()
                        .await
                        .expect("send kill to job completed tx");
                }
                continue;
            } else if killed_but_draining_same_worker_jobs {
                if job_completed_processor_is_done.load(Ordering::SeqCst) {
                    tracing::info!(worker = %worker_name, hostname = %hostname, "all running jobs have completed and all completed jobs have been fully processed, exiting");
                    break;
                } else {
                    tracing::info!(worker = %worker_name, hostname = %hostname, "there may be same_worker jobs to process later, waiting for job_completed_processor to finish progressing all remaining flows before exiting");
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    continue;
                }
            } else {
                match &conn {
                    Connection::Sql(db) => {
                        let pull_time = Instant::now();
                        let likelihood_of_suspend = last_30jobs_suspended as f64 / 30.0;
                        let suspend_first = suspend_first_success
                            || rand::random::<f64>() < likelihood_of_suspend
                            || last_suspend_first.elapsed().as_secs_f64() > 5.0;

                        if suspend_first {
                            last_suspend_first = Instant::now();
                        }

                        let job = pull(
                            &db,
                            suspend_first,
                            &worker_name,
                            None,
                            #[cfg(feature = "benchmark")]
                            &mut bench,
                        )
                        .await;

                        add_time!(bench, "job pulled from DB");
                        let duration_pull_s = pull_time.elapsed().as_secs_f64();
                        let err_pull = job.is_ok();
                        // let empty = job.as_ref().is_ok_and(|x| x.is_none());

                        if duration_pull_s > 0.5 {
                            let empty = job.as_ref().is_ok_and(|x| x.job.is_none());
                            tracing::warn!(worker = %worker_name, hostname = %hostname, "pull took more than 0.5s ({duration_pull_s}), this is a sign that the database is VERY undersized for this load. empty: {empty}, err: {err_pull}");
                            #[cfg(feature = "prometheus")]
                            if empty {
                                if let Some(wp) = worker_pull_over_500_counter_empty.as_ref() {
                                    wp.inc();
                                }
                            } else if let Some(wp) = worker_pull_over_500_counter.as_ref() {
                                wp.inc();
                            }
                        } else if duration_pull_s > 0.1 {
                            let empty = job.as_ref().is_ok_and(|x| x.job.is_none());
                            tracing::warn!(worker = %worker_name, hostname = %hostname, "pull took more than 0.1s ({duration_pull_s}) this is a sign that the database is undersized for this load. empty: {empty}, err: {err_pull}");
                            #[cfg(feature = "prometheus")]
                            if empty {
                                if let Some(wp) = worker_pull_over_100_counter_empty.as_ref() {
                                    wp.inc();
                                }
                            } else if let Some(wp) = worker_pull_over_100_counter.as_ref() {
                                wp.inc();
                            }
                        }

                        if let Ok(j) = job.as_ref() {
                            let suspend_success = j.suspended;
                            if suspend_first {
                                if last_30jobs_suspended < 30 {
                                    last_30jobs_suspended += 1;
                                }
                            } else {
                                last_30jobs_suspended -= 1;
                            }
                            suspend_first_success = suspend_first && suspend_success;
                            #[cfg(feature = "prometheus")]
                            if j.job.is_some() {
                                if let Some(wp) = worker_pull_duration_counter.as_ref() {
                                    wp.inc_by(duration_pull_s);
                                }
                                if let Some(wp) = worker_pull_duration.as_ref() {
                                    wp.observe(duration_pull_s);
                                }
                            } else {
                                if let Some(wp) = worker_pull_duration_counter_empty.as_ref() {
                                    wp.inc_by(duration_pull_s);
                                }
                                if let Some(wp) = worker_pull_duration_empty.as_ref() {
                                    wp.observe(duration_pull_s);
                                }
                            }
                        }
                        job.map(|x| x.job.map(NextJob::Sql))
                    }
                    Connection::Http(client) => crate::agent_workers::pull_job(&client, None, None)
                        .await
                        .map_err(|e| error::Error::InternalErr(e.to_string()))
                        .map(|x| x.map(|y| NextJob::Http(y))),
                }
            }
        };

        match next_job {
            Ok(Some(job)) => {
                #[cfg(feature = "prometheus")]
                if let Some(wb) = worker_busy.as_ref() {
                    wb.set(1);
                    tracing::debug!("set worker busy to 1");
                }

                occupancy_metrics.running_job_started_at = Some(Instant::now());

                last_executed_job = None;
                jobs_executed += 1;

                tracing::debug!(worker = %worker_name, hostname = %hostname, "started handling of job {}", job.id);

                if matches!(job.kind, JobKind::Script | JobKind::Preview) {
                    if !dedicated_workers.is_empty() {
                        let key_o = if is_flow_worker {
                            job.flow_step_id.as_ref().map(|x| x.to_string())
                        } else {
                            job.runnable_path.as_ref().map(|x| x.to_string())
                        };
                        if let Some(key) = key_o {
                            if let Some(dedicated_worker_tx) = dedicated_workers.get(&key) {
                                if let Err(e) = dedicated_worker_tx.send(Arc::new(job.job())).await
                                {
                                    tracing::info!("failed to send jobs to dedicated workers. Likely dedicated worker has been shut down. This is normal: {e:?}");
                                }

                                #[cfg(feature = "benchmark")]
                                {
                                    add_time!(bench, "sent to dedicated worker");
                                    infos.add_iter(bench, true);
                                }

                                continue;
                            }
                        }
                    }
                }

                if matches!(job.kind, JobKind::Noop) {
                    add_time!(bench, "send job completed START");
                    job_completed_tx
                        .send_job(
                            JobCompleted {
                                preprocessed_args: None,
                                job: Arc::new(job.job()),
                                success: true,
                                result: Arc::new(empty_result()),
                                result_columns: None,
                                mem_peak: 0,
                                cached_res_path: None,
                                token: "".to_string(),
                                canceled_by: None,
                                duration: None,
                            },
                            true,
                        )
                        .await
                        .expect("send job completed END");
                    add_time!(bench, "sent job completed");
                } else {
                    add_outstanding_wait_time(&conn, &job, *OUTSTANDING_WAIT_TIME_THRESHOLD_MS);

                    #[cfg(feature = "prometheus")]
                    register_metric(
                        &WORKER_EXECUTION_COUNT,
                        &job.tag,
                        |s| {
                            let counter = prometheus::register_int_counter!(prometheus::Opts::new(
                                "worker_execution_count",
                                "Number of executed jobs"
                            )
                            .const_label("name", &worker_name)
                            .const_label("tag", s))
                            .expect("register prometheus metric");
                            counter.inc();
                            (counter, ())
                        },
                        |c| c.inc(),
                    )
                    .await;

                    // counter.add(
                    //     1,
                    //     worker_resource
                    // );

                    #[cfg(feature = "prometheus")]
                    let _timer = register_metric(
                        &WORKER_EXECUTION_DURATION,
                        &job.tag,
                        |s| {
                            let counter =
                                prometheus::register_histogram!(prometheus::HistogramOpts::new(
                                    "worker_execution_duration",
                                    "Duration between receiving a job and completing it",
                                )
                                .const_label("name", &worker_name)
                                .const_label("tag", s))
                                .expect("register prometheus metric");
                            let t = counter.start_timer();
                            (counter, t)
                        },
                        |c| c.start_timer(),
                    )
                    .await;

                    let job_root = job
                        .flow_innermost_root_job
                        .map(|x| x.to_string())
                        .unwrap_or_else(|| "none".to_string());

                    if job.id == Uuid::nil() {
                        tracing::info!("running warmup job");
                    } else {
                        tracing::info!(workspace_id = %job.workspace_id, job_id = %job.id, root_id = %job_root, "fetched job {} (root job: {}, scheduled for: {})", job.id, job_root, job.scheduled_for);
                    } // Here we can't remove the job id, but maybe with the
                      // fields macro we can make a job id that only appears when
                      // the job is defined?

                    let job_dir = create_job_dir(&worker_dir, job.id).await;

                    let same_worker = job.same_worker;

                    let folder = if job.script_lang == Some(ScriptLang::Go) {
                        create_directory_async(&format!("{job_dir}/go")).await;
                        "/go"
                    } else {
                        ""
                    };

                    let target = &format!("{job_dir}{folder}/shared");

                    if same_worker && job.parent_job.is_some() {
                        if tokio::fs::metadata(target).await.is_err() {
                            let parent_flow = job.parent_job.unwrap();
                            let parent_shared_dir = format!("{worker_dir}/{parent_flow}/shared");
                            create_directory_async(&parent_shared_dir).await;
                            symlink(&parent_shared_dir, target)
                                .await
                                .expect("could not symlink target");
                        }
                    } else {
                        create_directory_async(target).await;
                    }

                    #[cfg(feature = "prometheus")]
                    let tag = job.tag.clone();

                    let is_init_script: bool = job.tag.as_str() == INIT_SCRIPT_TAG;
                    let is_flow = job.is_flow();
                    let job_id = job.id;

                    let JobAndPerms {
                        job,
                        raw_code,
                        raw_lock,
                        raw_flow,
                        parent_runnable_path,
                        token,
                        precomputed_agent_info: precomputed_bundle,
                    } = extract_job_and_perms(job, &conn).await;

                    let authed_client = AuthedClient::new(
                        base_internal_url.to_owned(),
                        job.workspace_id.clone(),
                        token,
                        None,
                    );

                    let arc_job = Arc::new(job);

                    let span = create_span(&arc_job, &worker_name, hostname);

                    let job_result = handle_queued_job(
                        arc_job.clone(),
                        raw_code,
                        raw_lock,
                        raw_flow,
                        parent_runnable_path,
                        &conn,
                        &authed_client,
                        hostname,
                        &worker_name,
                        &worker_dir,
                        &job_dir,
                        Some(same_worker_tx.clone()),
                        base_internal_url,
                        job_completed_tx.clone(),
                        &mut occupancy_metrics,
                        &mut killpill_rx2,
                        precomputed_bundle,
                        #[cfg(feature = "benchmark")]
                        &mut bench,
                    )
                    .instrument(span)
                    .await;

                    match job_result {
                        Ok(false) if is_init_script => {
                            tracing::error!("init script job failed, exiting");
                            update_worker_ping_for_failed_init_script(conn, &worker_name, job_id)
                                .await;
                            break;
                        }
                        Err(err) => {
                            handle_all_job_kind_error(
                                &conn,
                                &authed_client,
                                arc_job.clone(),
                                err,
                                Some(&same_worker_tx),
                                &worker_dir,
                                &worker_name,
                                job_completed_tx.clone(),
                                #[cfg(feature = "benchmark")]
                                &mut bench,
                            )
                            .await;
                            if is_init_script {
                                tracing::error!("init script job failed (in handler), exiting");
                                update_worker_ping_for_failed_init_script(
                                    conn,
                                    &worker_name,
                                    job_id,
                                )
                                .await;
                                break;
                            }
                        }
                        _ => {}
                    }

                    #[cfg(feature = "prometheus")]
                    if let Some(duration) = _timer.map(|x| x.stop_and_record()) {
                        register_metric(
                            &WORKER_EXECUTION_DURATION_COUNTER,
                            &tag,
                            |s| {
                                let counter = prometheus::register_counter!(prometheus::Opts::new(
                                    "worker_execution_duration_counter",
                                    "Total number of seconds spent executing jobs"
                                )
                                .const_label("name", &worker_name)
                                .const_label("tag", s))
                                .expect("register prometheus metric");
                                counter.inc_by(duration);
                                (counter, ())
                            },
                            |c| c.inc_by(duration),
                        )
                        .await;
                    }

                    if !KEEP_JOB_DIR.load(Ordering::Relaxed) && !(is_flow && same_worker) {
                        let _ = tokio::fs::remove_dir_all(job_dir).await;
                    }
                }

                #[cfg(feature = "benchmark")]
                {
                    if started {
                        add_time!(bench, "job processed");
                        infos.add_iter(bench, true);
                    }
                }
            }
            Ok(None) => {
                if let Some(secs) = *EXIT_AFTER_NO_JOB_FOR_SECS {
                    if let Some(lj) = last_executed_job {
                        if lj.elapsed().as_secs() > secs {
                            tracing::info!(worker = %worker_name, hostname = %hostname, "no job for {} seconds, exiting", secs);
                            break;
                        }
                    } else {
                        last_executed_job = Some(Instant::now());
                    }
                }

                #[cfg(feature = "prometheus")]
                let _timer = if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                    Some(Instant::now())
                } else {
                    None
                };

                tokio::time::sleep(Duration::from_millis(*SLEEP_QUEUE)).await;

                #[cfg(feature = "benchmark")]
                {
                    add_time!(bench, "sleep because empty job queue");
                    infos.add_iter(bench, false);
                }
                #[cfg(feature = "prometheus")]
                _timer.map(|timer| {
                    let duration = timer.elapsed().as_secs_f64();
                    if let Some(ws) = worker_sleep_duration_counter.as_ref() {
                        ws.inc_by(duration);
                    }
                });
            }
            Err(err) => {
                tracing::error!(worker = %worker_name, hostname = %hostname, "Failed to pull jobs: {}", err);
                tokio::time::sleep(Duration::from_millis(*SLEEP_QUEUE * 5)).await;
            }
        };
    }

    tracing::info!(worker = %worker_name, hostname = %hostname, "worker {} exiting", worker_name);

    #[cfg(feature = "benchmark")]
    {
        infos
            .write_to_file("profiling_main.json")
            .expect("write to file profiling");
    }

    drop(dedicated_workers);

    let has_dedicated_workers = !dedicated_handles.is_empty();
    if has_dedicated_workers {
        for handle in dedicated_handles {
            if let Err(e) = handle.await {
                tracing::error!(worker = %worker_name, hostname = %hostname, "error in dedicated worker waiting for it to end: {:?}", e)
            }
        }
        tracing::info!(worker = %worker_name, hostname = %hostname, "all dedicated workers have exited");
    }

    drop(job_completed_tx);

    tracing::info!(worker = %worker_name, hostname = %hostname, "waiting for job_completed_processor to finish processing remaining jobs");
    if let Some(send_result) = send_result {
        if let Err(e) = send_result.await {
            tracing::error!("error in awaiting send_result process: {e:?}")
        }
    }
    if let Some(interactive_shell) = interactive_shell {
        if let Err(e) = interactive_shell.await {
            tracing::error!("error in awaiting interactive_shell process: {e:?}")
        }
    }
    tracing::info!(worker = %worker_name, hostname = %hostname, "worker {} exited", worker_name);
    tracing::info!(worker = %worker_name, hostname = %hostname, "number of jobs executed: {}", jobs_executed);
}

async fn queue_init_bash_maybe<'c>(
    conn: &Connection,
    same_worker_tx: SameWorkerSender,
    worker_name: &str,
) -> anyhow::Result<bool> {
    let uuid_content = if let Some(content) = WORKER_CONFIG.read().await.init_bash.clone() {
        let uuid = match conn {
            Connection::Sql(db) => push_init_job(db, content.clone(), worker_name).await?,
            Connection::Http(client) => queue_init_job(client, &content).await?,
        };
        Some((uuid, content))
    } else {
        None
    };
    if let Some((uuid, content)) = uuid_content {
        same_worker_tx
            .send(SameWorkerPayload { job_id: uuid, recoverable: false })
            .await
            .map_err(to_anyhow)?;
        tracing::info!("Creating initial job {uuid} from initial script script: {content}");
        Ok(true)
    } else {
        Ok(false)
    }
}

pub enum SendResult {
    JobCompleted(JobCompleted),
    UpdateFlow(UpdateFlow),
}

pub struct UpdateFlow {
    pub flow: Uuid,
    pub w_id: String,
    pub success: bool,
    pub result: Box<RawValue>,
    pub worker_dir: String,
    pub stop_early_override: Option<bool>,
    pub token: String,
}

async fn do_nativets(
    job: &MiniPulledJob,
    client: &AuthedClient,
    env_code: String,
    code: String,
    conn: &Connection,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    occupancy_metrics: &mut OccupancyMetrics,
) -> windmill_common::error::Result<Box<RawValue>> {
    let args = build_args_map(job, client, conn).await?.map(Json);
    let job_args = if args.is_some() {
        args.as_ref()
    } else {
        job.args.as_ref()
    };

    Ok(eval_fetch_timeout(
        env_code,
        code.clone(),
        transpile_ts(code)?,
        job_args,
        None,
        job.id,
        job.timeout,
        conn,
        mem_peak,
        canceled_by,
        worker_name,
        &job.workspace_id,
        true,
        occupancy_metrics,
    )
    .await?)
}

#[derive(Deserialize, Serialize, Default)]
pub struct PreviousResult<'a> {
    #[serde(borrow)]
    pub previous_result: Option<&'a RawValue>,
}

pub async fn handle_queued_job(
    job: Arc<MiniPulledJob>,
    raw_code: Option<String>,
    raw_lock: Option<String>,
    raw_flow: Option<Json<Box<RawValue>>>,
    parent_runnable_path: Option<String>,
    conn: &Connection,
    client: &AuthedClient,
    hostname: &str,
    worker_name: &str,
    worker_dir: &str,
    job_dir: &str,
    same_worker_tx: Option<SameWorkerSender>,
    base_internal_url: &str,
    job_completed_tx: JobCompletedSender,
    occupancy_metrics: &mut OccupancyMetrics,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
    precomputed_agent_info: Option<PrecomputedAgentInfo>,
    #[cfg(feature = "benchmark")] _bench: &mut BenchmarkIter,
) -> windmill_common::error::Result<bool> {
    // Extract the active span from the context

    if job.canceled_by.is_some() {
        return Err(Error::JsonErr(canceled_job_to_result(&job)));
    }
    if let Some(e) = &job.pre_run_error {
        return Err(Error::ExecutionErr(e.to_string()));
    }

    #[cfg(any(not(feature = "enterprise"), feature = "sqlx"))]
    match conn {
        Connection::Sql(db) => {
            if job.parent_job.is_none() && job.created_by.starts_with("email-") {
                let daily_count = sqlx::query!(
            "SELECT value FROM metrics WHERE id = 'email_trigger_usage' AND created_at > NOW() - INTERVAL '1 day' ORDER BY created_at DESC LIMIT 1"
        ).fetch_optional(db)
        .warn_after_seconds(5)
        .await?.map(|x| serde_json::from_value::<i64>(x.value).unwrap_or(1));

                if let Some(count) = daily_count {
                    if count >= 100 {
                        return Err(error::Error::QuotaExceeded(format!(
                            "Email trigger usage limit of 100 per day has been reached."
                        )));
                    } else {
                        sqlx::query!(
                    "UPDATE metrics SET value = $1 WHERE id = 'email_trigger_usage' AND created_at > NOW() - INTERVAL '1 day'",
                    serde_json::json!(count + 1)
                )
                .execute(db)
                .warn_after_seconds(5)
                .await?;
                    }
                } else {
                    sqlx::query!(
                "INSERT INTO metrics (id, value) VALUES ('email_trigger_usage', to_jsonb(1))"
            )
                    .execute(db)
                    .warn_after_seconds(5)
                    .await?;
                }
            }
        }
        Connection::Http(_) => {
            return Err(Error::internal_err(format!(
                "Could not check email trigger usage for job with agent worker {}",
                job.id
            )))
        }
    }

    // no need to mark job as started if http conn, it's done by the server when pulled
    if let Connection::Sql(db) = conn {
        job.mark_as_started_if_step(db).await?;
    }

    let started = Instant::now();
    // Pre-fetch preview jobs raw values if necessary.
    // The `raw_*` values passed to this function are the original raw values from `queue` tables,
    // they are kept for backward compatibility as they have been moved to the `job` table.
    let preview_data = match (job.kind, job.runnable_id) {
        (
            JobKind::Preview
            | JobKind::Dependencies
            | JobKind::FlowPreview
            | JobKind::Flow
            | JobKind::FlowDependencies,
            x,
        ) => match x.map(|x| x.0) {
            None | Some(PREVIEW_IS_CODEBASE_HASH) | Some(PREVIEW_IS_TAR_CODEBASE_HASH) => {
                Some(cache::job::fetch_preview(conn, &job.id, raw_lock, raw_code, raw_flow).await?)
            }
            _ => None,
        },
        _ => None,
    };

    let cached_res_path = if job.cache_ttl.is_some() {
        match conn {
            Connection::Sql(db) => {
                Some(cached_result_path(db, &client, &job, preview_data.as_ref()).await)
            }
            Connection::Http(_) => None,
        }
    } else {
        None
    };

    if let Some(db) = conn.as_sql() {
        if let Some(cached_res_path) = cached_res_path.as_ref() {
            let cached_result_maybe = get_cached_resource_value_if_valid(
                db,
                &client,
                &job.id,
                &job.workspace_id,
                &cached_res_path,
            )
            .warn_after_seconds(5)
            .await;
            if let Some(result) = cached_result_maybe {
                {
                    let logs = "Job skipped because args & path found in cache and not expired"
                        .to_string();
                    append_logs(&job.id, &job.workspace_id, logs, conn).await;
                }
                let result = job_completed_tx
                    .send_job(
                        JobCompleted {
                            preprocessed_args: None,
                            job,
                            result,
                            result_columns: None,
                            mem_peak: 0,
                            canceled_by: None,
                            success: true,
                            cached_res_path: None,
                            token: client.token.clone(),
                            duration: None,
                        },
                        true,
                    )
                    .await;

                match result {
                    Ok(_) => {
                        tracing::debug!("Send job completed")
                    }
                    Err(err) => {
                        tracing::error!("An error occurred while sending job completed: {:#?}", err)
                    }
                }

                return Ok(true);
            }
        };
    }
    if job.is_flow() {
        if let Some(db) = conn.as_sql() {
            let flow_data = match preview_data {
                Some(RawData::Flow(data)) => data,
                // Not a preview: fetch from the cache or the database.
                _ => cache::job::fetch_flow(db, job.kind, job.runnable_id).await?,
            };
            handle_flow(
                job,
                &flow_data,
                db,
                &client,
                None,
                &same_worker_tx.expect(SAME_WORKER_REQUIREMENTS),
                worker_dir,
                job_completed_tx.clone(),
                worker_name,
            )
            .warn_after_seconds(10)
            .await?;
            Ok(true)
        } else {
            return Err(Error::internal_err(
                "Could not handle flow job with agent worker".to_string(),
            ));
        }
    } else {
        let mut logs = "".to_string();
        let mut mem_peak: i32 = 0;
        let mut canceled_by: Option<CanceledBy> = None;
        // println!("handle queue {:?}",  SystemTime::now());

        logs.push_str(&format!(
            "job={} tag={} worker={} hostname={}\n",
            &job.id, &job.tag, &worker_name, &hostname
        ));

        if *NO_LOGS_AT_ALL {
            logs.push_str("Logs are fully disabled for this worker\n");
        }

        if *NO_LOGS {
            logs.push_str("Logs are disabled for this worker\n");
        }

        if *SLOW_LOGS {
            logs.push_str("Logs are 10x less frequent for this worker\n");
        }

        #[cfg(not(feature = "enterprise"))]
        if job.concurrent_limit.is_some() {
            logs.push_str("---\n");
            logs.push_str("WARNING: This job has concurrency limits enabled. Concurrency limits are an EE feature and the setting is ignored.\n");
            logs.push_str("---\n");
        }

        tracing::debug!(
            workspace_id = %job.workspace_id,
            "handling job {}",
            job.id
        );
        append_logs(&job.id, &job.workspace_id, logs, conn).await;

        let mut column_order: Option<Vec<String>> = None;
        let mut new_args: Option<HashMap<String, Box<RawValue>>> = None;
        let result = match job.kind {
            JobKind::Dependencies => match conn {
                Connection::Sql(db) => {
                    handle_dependency_job(
                        &job,
                        preview_data.as_ref(),
                        &mut mem_peak,
                        &mut canceled_by,
                        job_dir,
                        db,
                        worker_name,
                        worker_dir,
                        base_internal_url,
                        &client.token,
                        occupancy_metrics,
                    )
                    .await
                }
                Connection::Http(_) => {
                    return Err(Error::internal_err(
                        "Could not handle dependency job with agent worker".to_string(),
                    ));
                }
            },
            JobKind::FlowDependencies => match conn {
                Connection::Sql(db) => {
                    handle_flow_dependency_job(
                        &job,
                        preview_data.as_ref(),
                        &mut mem_peak,
                        &mut canceled_by,
                        job_dir,
                        db,
                        worker_name,
                        worker_dir,
                        base_internal_url,
                        &client.token,
                        occupancy_metrics,
                    )
                    .await
                }
                Connection::Http(_) => {
                    return Err(Error::internal_err(
                        "Could not handle flow dependency job with agent worker".to_string(),
                    ));
                }
            },
            JobKind::AppDependencies => match conn {
                Connection::Sql(db) => handle_app_dependency_job(
                    &job,
                    &mut mem_peak,
                    &mut canceled_by,
                    job_dir,
                    db,
                    worker_name,
                    worker_dir,
                    base_internal_url,
                    &client.token,
                    occupancy_metrics,
                )
                .await
                .map(|()| serde_json::from_str("{}").unwrap()),
                Connection::Http(_) => {
                    return Err(Error::internal_err(
                        "Could not handle app dependency job with agent worker".to_string(),
                    ));
                }
            },
            JobKind::Identity => Ok(job
                .args
                .as_ref()
                .map(|x| x.get("previous_result"))
                .flatten()
                .map(|x| x.to_owned())
                .unwrap_or_else(|| serde_json::from_str("{}").unwrap())),
            _ => {
                let metric_timer = Instant::now();
                let preview_data = preview_data.and_then(|data| match data {
                    RawData::Script(data) => Some(data),
                    _ => None,
                });
                let r = handle_code_execution_job(
                    job.as_ref(),
                    preview_data,
                    conn,
                    client,
                    parent_runnable_path,
                    job_dir,
                    worker_dir,
                    &mut mem_peak,
                    &mut canceled_by,
                    base_internal_url,
                    worker_name,
                    &mut column_order,
                    &mut new_args,
                    occupancy_metrics,
                    killpill_rx,
                    precomputed_agent_info,
                )
                .await;
                occupancy_metrics.total_duration_of_running_jobs +=
                    metric_timer.elapsed().as_secs_f32();
                r
            }
        };

        //it's a test job, no need to update the db
        if job.as_ref().workspace_id == "" {
            return Ok(true);
        }

        if result
            .as_ref()
            .is_err_and(|err| matches!(err, &Error::AlreadyCompleted(_)))
        {
            return Ok(false);
        }
        process_result(
            job,
            result.map(|x| Arc::new(x)),
            job_dir,
            job_completed_tx,
            mem_peak,
            canceled_by,
            cached_res_path,
            &client.token,
            column_order,
            new_args,
            conn,
            Some(started.elapsed().as_millis() as i64),
        )
        .await
    }
}

pub fn build_envs(
    envs: Option<&Vec<String>>,
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

    for (k, v) in PROXY_ENVS.iter() {
        envs.insert(k.to_string(), v.to_string());
    }

    Ok(envs)
}

pub struct ContentReqLangEnvs {
    pub content: String,
    pub lockfile: Option<String>,
    pub language: Option<ScriptLang>,
    pub envs: Option<Vec<String>>,
    pub codebase: Option<String>,
    pub schema: Option<String>,
}

pub async fn get_hub_script_content_and_requirements(
    script_path: Option<&String>,
    db: Option<&DB>,
) -> error::Result<ContentReqLangEnvs> {
    let script_path = script_path
        .clone()
        .ok_or_else(|| Error::internal_err(format!("expected script path for hub script")))?;

    let script =
        get_full_hub_script_by_path(StripPath(script_path.to_string()), &HTTP_CLIENT, db).await?;
    Ok(ContentReqLangEnvs {
        content: script.content,
        lockfile: script.lockfile,
        language: Some(script.language),
        envs: None,
        codebase: None,
        schema: Some(script.schema.get().to_string()),
    })
}

pub async fn get_script_content_by_hash(
    script_hash: &ScriptHash,
    _w_id: &str,
    conn: &Connection,
) -> error::Result<ContentReqLangEnvs> {
    let (data, metadata) = cache::script::fetch(conn, *script_hash).await?;
    Ok(ContentReqLangEnvs {
        content: data.code.clone(),
        lockfile: data.lock.clone(),
        language: metadata.language,
        envs: metadata.envs.clone(),
        codebase: match metadata.codebase.as_ref() {
            None => None,
            Some(x) if x.ends_with(".tar") => Some(format!("{}.tar", script_hash)),
            Some(_) => Some(script_hash.to_string()),
        },
        schema: None,
    })
}

async fn try_validate_schema(
    job: &MiniPulledJob,
    conn: &Connection,
    schema_validator: Option<&SchemaValidator>,
    code: &str,
    language: Option<&ScriptLang>,
    schema: Option<&String>,
) -> Result<(), Error> {
    if let Some(args) = job.args.as_ref() {
        if let Some(sv) = schema_validator {
            sv.validate(args)?;
        } else {
            let validators_cache = cache::anon!({ (u8, ScriptHash) => Arc<Option<SchemaValidator>> } in "schemavalidators" <= 1000);

            let sv_fut = async move {
                if language.map(|l| should_validate_schema(code, l)).unwrap_or(false) {
                    if let Some(schema) = schema {
                        Ok(Some(SchemaValidator::from_schema(schema)?))
                    } else {
                        if let Some(sig) = parse_sig_of_lang(
                            code,
                            language,
                            job.script_entrypoint_override.clone(),
                        )? {
                            Ok(Some(schema_validator_from_main_arg_sig(&sig)))
                        } else {
                            Err(anyhow!("Job was expected to validate the arguments schema, but no schema was provided and couldn't be inferred from the script for language `{language:?}`. Try removing schema validation for this job").into())
                        }
                    }
                } else { Ok(None) }
            }
            .map_ok(Arc::new);

            let sub_key: u8 = match job.kind {
                JobKind::Script => 0,
                JobKind::FlowScript => 1,
                JobKind::AppScript => 2,
                JobKind::Script_Hub => 3,
                JobKind::Preview => 4,
                JobKind::DeploymentCallback => 5,
                JobKind::SingleScriptFlow => 6,
                JobKind::Dependencies => 7,
                JobKind::Flow => 8,
                JobKind::FlowPreview => 9,
                JobKind::Identity => 10,
                JobKind::FlowDependencies => 11,
                JobKind::AppDependencies => 12,
                JobKind::Noop => 13,
                JobKind::FlowNode => 14,
            };

            let sv = match job.runnable_id {
                Some(hash) if job.kind != JobKind::Preview && job.kind != JobKind::FlowPreview => {
                    sv_fut.cached(validators_cache, (sub_key, hash)).await?
                }
                _ => sv_fut.await?,
            };

            if sv.is_some() && job.kind == JobKind::Preview {
                append_logs(
                    &job.id,
                    &job.workspace_id,
                    "\n--- ARGS VALIDATION ---\nScript contains `schema_validation` annotation, running schema validation for the script arguments...\n",
                    conn,
                )
                .await;
            }

            sv.as_ref()
                .as_ref()
                .map(|sv| sv.validate(args))
                .transpose()?;

            if sv.is_some() {
                append_logs(
                    &job.id,
                    &job.workspace_id,
                    "Script arguments were validated!\n\n",
                    conn,
                )
                .await;
            }
        }
    }

    Ok(())
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_code_execution_job(
    job: &MiniPulledJob,
    preview: Option<Arc<ScriptData>>,
    conn: &Connection,
    client: &AuthedClient,
    parent_runnable_path: Option<String>,
    job_dir: &str,
    #[allow(unused_variables)] worker_dir: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    base_internal_url: &str,
    worker_name: &str,
    column_order: &mut Option<Vec<String>>,
    new_args: &mut Option<HashMap<String, Box<RawValue>>>,
    occupancy_metrics: &mut OccupancyMetrics,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
    precomputed_agent_info: Option<PrecomputedAgentInfo>,
) -> error::Result<Box<RawValue>> {
    let script_hash = || {
        job.runnable_id
            .ok_or_else(|| Error::internal_err("expected script hash"))
    };
    let (arc_data, arc_metadata, data, metadata): (
        Arc<ScriptData>,
        Arc<ScriptMetadata>,
        ScriptData,
        ScriptMetadata,
    );
    let (
        ScriptData { code, lock },
        ScriptMetadata { language, envs, codebase, schema_validator, schema },
    ) = match job.kind {
        JobKind::Preview => {
            let codebase = match job.runnable_id.map(|x| x.0) {
                Some(PREVIEW_IS_CODEBASE_HASH) => Some(job.id.to_string()),
                Some(PREVIEW_IS_TAR_CODEBASE_HASH) => Some(format!("{}.tar", job.id)),
                _ => None,
            };

            if codebase.is_none() && job.runnable_id.is_some() {
                (arc_data, arc_metadata) =
                    cache::script::fetch(conn, job.runnable_id.unwrap()).await?;
                (arc_data.as_ref(), arc_metadata.as_ref())
            } else {
                arc_data =
                    preview.ok_or_else(|| Error::internal_err("expected preview".to_string()))?;
                metadata = ScriptMetadata {
                    language: job.script_lang,
                    codebase,
                    envs: None,
                    schema: None,
                    schema_validator: None,
                };
                (arc_data.as_ref(), &metadata)
            }
        }
        JobKind::Script_Hub => {
            let ContentReqLangEnvs { content, lockfile, language, envs, codebase, schema } =
                get_hub_script_content_and_requirements(job.runnable_path.as_ref(), conn.as_sql())
                    .await?;

            data = ScriptData { code: content, lock: lockfile };
            metadata = ScriptMetadata { language, envs, codebase, schema, schema_validator: None };
            (&data, &metadata)
        }
        JobKind::Script => {
            (arc_data, arc_metadata) = cache::script::fetch(conn, script_hash()?).await?;
            (arc_data.as_ref(), arc_metadata.as_ref())
        }
        JobKind::FlowScript => {
            arc_data = cache::flow::fetch_script(conn, FlowNodeId(script_hash()?.0)).await?;
            metadata = ScriptMetadata {
                language: job.script_lang,
                envs: None,
                codebase: None,
                schema: None,
                schema_validator: None,
            };
            (arc_data.as_ref(), &metadata)
        }
        JobKind::AppScript => {
            arc_data = cache::app::fetch_script(conn, AppScriptId(script_hash()?.0)).await?;
            metadata = ScriptMetadata {
                language: job.script_lang,
                envs: None,
                codebase: None,
                schema: None,
                schema_validator: None,
            };
            (arc_data.as_ref(), &metadata)
        }
        JobKind::DeploymentCallback => match conn {
            Connection::Sql(db) => {
                let script_path = job
                    .runnable_path
                    .as_ref()
                    .ok_or_else(|| Error::internal_err("expected script path".to_string()))?;
                if script_path.starts_with("hub/") {
                    let ContentReqLangEnvs { content, lockfile, language, envs, codebase, schema } =
                        get_hub_script_content_and_requirements(Some(script_path), conn.as_sql())
                            .await?;
                    data = ScriptData { code: content, lock: lockfile };
                    metadata =
                        ScriptMetadata { language, envs, codebase, schema, schema_validator: None };
                    (&data, &metadata)
                } else {
                    let hash = sqlx::query_scalar!(
                        "SELECT hash FROM script WHERE path = $1 AND workspace_id = $2 AND
                    deleted = false AND lock IS not NULL AND lock_error_logs IS NULL",
                        script_path,
                        &job.workspace_id
                    )
                    .fetch_optional(db)
                    .await?
                    .ok_or_else(|| Error::internal_err("expected script hash".to_string()))?;

                    (arc_data, arc_metadata) = cache::script::fetch(conn, ScriptHash(hash)).await?;
                    (arc_data.as_ref(), arc_metadata.as_ref())
                }
            }
            Connection::Http(_) => {
                return Err(Error::internal_err(
                    "Could not handle deployment callback with agent worker".to_string(),
                ));
            }
        },
        _ => unreachable!(
            "handle_code_execution_job should never be reachable with a non-code execution job"
        ),
    };

    try_validate_schema(
        job,
        conn,
        schema_validator.as_ref(),
        code,
        language.as_ref(),
        schema.as_ref(),
    )
    .await?;

    let language = language.clone();
    if language == Some(ScriptLang::Postgresql) {
        return do_postgresql(
            job,
            &client,
            &code,
            conn,
            mem_peak,
            canceled_by,
            worker_name,
            column_order,
            occupancy_metrics,
        )
        .await;
    } else if language == Some(ScriptLang::Mysql) {
        #[cfg(not(feature = "mysql"))]
        return Err(Error::internal_err(
            "MySQL requires the mysql feature to be enabled".to_string(),
        ));

        #[cfg(feature = "mysql")]
        return do_mysql(
            job,
            &client,
            &code,
            conn,
            mem_peak,
            canceled_by,
            worker_name,
            column_order,
            occupancy_metrics,
        )
        .await;
    } else if language == Some(ScriptLang::Bigquery) {
        #[cfg(not(feature = "enterprise"))]
        {
            return Err(Error::ExecutionErr(
                "Bigquery is only available with an enterprise license".to_string(),
            ));
        }

        #[allow(unreachable_code)]
        #[cfg(not(feature = "bigquery"))]
        {
            return Err(Error::internal_err(
                "Bigquery requires the bigquery feature to be enabled".to_string(),
            ));
        }

        #[cfg(all(feature = "enterprise", feature = "bigquery"))]
        {
            return do_bigquery(
                job,
                &client,
                &code,
                conn,
                mem_peak,
                canceled_by,
                worker_name,
                column_order,
                occupancy_metrics,
            )
            .await;
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
            return do_snowflake(
                job,
                &client,
                &code,
                conn,
                mem_peak,
                canceled_by,
                worker_name,
                column_order,
                occupancy_metrics,
            )
            .await;
        }
    } else if language == Some(ScriptLang::Mssql) {
        #[cfg(not(feature = "enterprise"))]
        {
            return Err(Error::ExecutionErr(
                "Microsoft SQL server is only available with an enterprise license".to_string(),
            ));
        }

        #[allow(unreachable_code)]
        #[cfg(not(feature = "mssql"))]
        {
            return Err(Error::internal_err(
                "Microsoft SQL server requires the mssql feature to be enabled".to_string(),
            ));
        }

        #[cfg(all(feature = "enterprise", feature = "mssql"))]
        {
            return do_mssql(
                job,
                &client,
                &code,
                conn,
                mem_peak,
                canceled_by,
                worker_name,
                occupancy_metrics,
                job_dir,
            )
            .await;
        }
    } else if language == Some(ScriptLang::OracleDB) {
        #[cfg(not(feature = "enterprise"))]
        {
            return Err(Error::ExecutionErr(
                "Oracle DB is only available with an enterprise license".to_string(),
            ));
        }

        #[allow(unreachable_code)]
        #[cfg(not(feature = "oracledb"))]
        {
            return Err(Error::internal_err(
                "Oracle DB requires the oracledb feature to be enabled".to_string(),
            ));
        }

        #[cfg(all(feature = "enterprise", feature = "oracledb"))]
        {
            return do_oracledb(
                job,
                &client,
                &code,
                conn,
                mem_peak,
                canceled_by,
                worker_name,
                column_order,
                occupancy_metrics,
            )
            .await;
        }
    } else if language == Some(ScriptLang::DuckDb) {
        #[allow(unreachable_code)]
        #[cfg(not(feature = "duckdb"))]
        {
            return Err(Error::internal_err(
                "Duck DB requires the duckdb feature to be enabled".to_string(),
            ));
        }

        #[cfg(feature = "duckdb")]
        {
            return do_duckdb(
                job,
                &client,
                &code,
                conn,
                mem_peak,
                canceled_by,
                worker_name,
                column_order,
                occupancy_metrics,
            )
            .await;
        }
    } else if language == Some(ScriptLang::Graphql) {
        return do_graphql(
            job,
            &client,
            &code,
            conn,
            mem_peak,
            canceled_by,
            worker_name,
            occupancy_metrics,
        )
        .await;
    } else if language == Some(ScriptLang::Nativets) {
        append_logs(
            &job.id,
            &job.workspace_id,
            "\n--- FETCH TS EXECUTION ---\n",
            conn,
        )
        .await;

        let reserved_variables =
            get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;

        let env_code = format!(
            "const process = {{ env: {{}} }};\nconst BASE_URL = '{base_internal_url}';\nconst BASE_INTERNAL_URL = '{base_internal_url}';\nprocess.env['BASE_URL'] = BASE_URL;process.env['BASE_INTERNAL_URL'] = BASE_INTERNAL_URL;\n{}",
            reserved_variables
                .iter()
                .map(|(k, v)| format!("const {} = '{}';\nprocess.env['{}'] = '{}';\n", k, v, k, v))
                .collect::<Vec<String>>()
                .join("\n"));

        let result = do_nativets(
            job,
            &client,
            env_code,
            code.clone(),
            conn,
            mem_peak,
            canceled_by,
            worker_name,
            occupancy_metrics,
        )
        .await?;
        return Ok(result);
    }

    let lang_str = job
        .script_lang
        .as_ref()
        .map(|x| format!("{x:?}"))
        .unwrap_or_else(|| "NO_LANG".to_string());

    tracing::debug!(
        workspace_id = %job.workspace_id,
        "started {} job {}",
        &lang_str,
        job.id
    );

    let shared_mount = if job.same_worker && job.script_lang != Some(ScriptLang::Deno) {
        let folder = if job.script_lang == Some(ScriptLang::Go) {
            "/go"
        } else {
            ""
        };
        format!(
            r#"
mount {{
    src: "{job_dir}{folder}/shared"
    dst: "/tmp{folder}/shared"
    is_bind: true
    rw: true
}}
        "#
        )
    } else {
        "".to_string()
    };

    // println!("handle lang job {:?}",  SystemTime::now());

    let envs = build_envs(envs.as_ref())?;

    let result: error::Result<Box<RawValue>> = match language {
        None => {
            return Err(Error::ExecutionErr(
                "Require language to be not null".to_string(),
            ))?;
        }
        Some(ScriptLang::Python3) => {
            #[cfg(not(feature = "python"))]
            return Err(Error::internal_err(
                "Python requires the python feature to be enabled".to_string(),
            ));

            #[cfg(feature = "python")]
            handle_python_job(
                lock.as_ref(),
                job_dir,
                worker_dir,
                worker_name,
                job,
                mem_peak,
                canceled_by,
                conn,
                client,
                parent_runnable_path,
                &code,
                &shared_mount,
                base_internal_url,
                envs,
                new_args,
                occupancy_metrics,
                precomputed_agent_info,
            )
            .await
        }
        Some(ScriptLang::Deno) => {
            handle_deno_job(
                lock.as_ref(),
                mem_peak,
                canceled_by,
                job,
                conn,
                client,
                parent_runnable_path,
                job_dir,
                &code,
                base_internal_url,
                worker_name,
                envs,
                new_args,
                occupancy_metrics,
            )
            .await
        }
        Some(ScriptLang::Bun) | Some(ScriptLang::Bunnative) => {
            handle_bun_job(
                lock.as_ref(),
                codebase.as_ref(),
                mem_peak,
                canceled_by,
                job,
                conn,
                client,
                parent_runnable_path,
                job_dir,
                &code,
                base_internal_url,
                worker_name,
                envs,
                &shared_mount,
                new_args,
                occupancy_metrics,
                precomputed_agent_info,
            )
            .await
        }
        Some(ScriptLang::Go) => {
            handle_go_job(
                mem_peak,
                canceled_by,
                job,
                conn,
                client,
                parent_runnable_path,
                &code,
                job_dir,
                lock.as_ref(),
                &shared_mount,
                base_internal_url,
                worker_name,
                envs,
                occupancy_metrics,
            )
            .await
        }
        Some(ScriptLang::Bash) => {
            handle_bash_job(
                mem_peak,
                canceled_by,
                job,
                conn,
                client,
                parent_runnable_path,
                &code,
                job_dir,
                &shared_mount,
                base_internal_url,
                worker_name,
                envs,
                occupancy_metrics,
                killpill_rx,
            )
            .await
        }
        Some(ScriptLang::Powershell) => {
            handle_powershell_job(
                mem_peak,
                canceled_by,
                job,
                conn,
                client,
                parent_runnable_path,
                &code,
                job_dir,
                &shared_mount,
                base_internal_url,
                worker_name,
                envs,
                occupancy_metrics,
            )
            .await
        }
        Some(ScriptLang::Php) => {
            #[cfg(not(feature = "php"))]
            return Err(Error::internal_err(
                "PHP requires the php feature to be enabled".to_string(),
            ));

            #[cfg(feature = "php")]
            handle_php_job(
                lock.as_ref(),
                mem_peak,
                canceled_by,
                job,
                conn,
                client,
                parent_runnable_path,
                job_dir,
                &code,
                base_internal_url,
                worker_name,
                envs,
                &shared_mount,
                occupancy_metrics,
            )
            .await
        }
        Some(ScriptLang::Rust) => {
            #[cfg(not(feature = "rust"))]
            return Err(Error::internal_err(
                "Rust requires the rust feature to be enabled".to_string(),
            ));

            #[cfg(feature = "rust")]
            handle_rust_job(
                mem_peak,
                canceled_by,
                job,
                conn,
                client,
                parent_runnable_path,
                &code,
                job_dir,
                lock.as_ref(),
                &shared_mount,
                base_internal_url,
                worker_name,
                envs,
                occupancy_metrics,
            )
            .await
        }
        Some(ScriptLang::Ansible) => {
            #[cfg(not(feature = "python"))]
            return Err(Error::internal_err(
                "Ansible requires the python feature to be enabled".to_string(),
            ));

            #[cfg(feature = "python")]
            handle_ansible_job(
                lock.as_ref(),
                job_dir,
                worker_dir,
                worker_name,
                job,
                mem_peak,
                canceled_by,
                conn,
                client,
                parent_runnable_path,
                &code,
                &shared_mount,
                base_internal_url,
                envs,
                occupancy_metrics,
            )
            .await
        }
        Some(ScriptLang::CSharp) => {
            handle_csharp_job(
                mem_peak,
                canceled_by,
                job,
                conn,
                client,
                parent_runnable_path,
                &code,
                job_dir,
                lock.as_ref(),
                &shared_mount,
                base_internal_url,
                worker_name,
                envs,
                occupancy_metrics,
            )
            .await
        }
        Some(ScriptLang::Nu) => {
            #[cfg(not(feature = "nu"))]
            return Err(
                anyhow::anyhow!("Nu is not available because the feature is not enabled").into(),
            );

            #[cfg(feature = "nu")]
            handle_nu_job(JobHandlerInputNu {
                mem_peak,
                canceled_by,
                job,
                conn,
                client,
                parent_runnable_path,
                inner_content: &code,
                job_dir,
                requirements_o: lock.as_ref(),
                shared_mount: &shared_mount,
                base_internal_url,
                worker_name,
                envs,
                occupancy_metrics,
            })
            .await
        }
        Some(ScriptLang::Java) => {
            #[cfg(not(feature = "java"))]
            return Err(anyhow::anyhow!(
                "Java is not available because the feature is not enabled"
            )
            .into());

            #[cfg(feature = "java")]
            handle_java_job(JobHandlerInputJava {
                mem_peak,
                canceled_by,
                job,
                conn,
                client,
                parent_runnable_path,
                inner_content: &code,
                job_dir,
                requirements_o: lock.as_ref(),
                shared_mount: &shared_mount,
                base_internal_url,
                worker_name,
                envs,
                occupancy_metrics,
            })
            .await
        }
        _ => panic!("unreachable, language is not supported: {language:#?}"),
    };
    tracing::info!(
        workspace_id = %job.workspace_id,
        is_ok = result.is_ok(),
        "finished {} job {}",
        &lang_str,
        job.id
    );
    // println!("handled job: {:?}",  SystemTime::now());

    result
}

fn parse_sig_of_lang(
    code: &str,
    language: Option<&ScriptLang>,
    main_override: Option<String>,
) -> Result<Option<MainArgSignature>> {
    Ok(if let Some(lang) = language {
        match lang {
            ScriptLang::Nativets | ScriptLang::Deno | ScriptLang::Bun | ScriptLang::Bunnative => {
                Some(windmill_parser_ts::parse_deno_signature(
                    code,
                    true,
                    false,
                    main_override,
                )?)
            }
            #[cfg(feature = "python")]
            ScriptLang::Python3 => Some(windmill_parser_py::parse_python_signature(
                code,
                main_override,
                false,
            )?),
            #[cfg(not(feature = "python"))]
            ScriptLang::Python3 => None,
            ScriptLang::Go => Some(windmill_parser_go::parse_go_sig(code)?),
            ScriptLang::Bash => Some(windmill_parser_bash::parse_bash_sig(code)?),
            ScriptLang::Powershell => Some(windmill_parser_bash::parse_powershell_sig(code)?),
            ScriptLang::Postgresql => Some(windmill_parser_sql::parse_pgsql_sig(code)?),
            ScriptLang::Mysql => Some(windmill_parser_sql::parse_mysql_sig(code)?),
            ScriptLang::Bigquery => Some(windmill_parser_sql::parse_bigquery_sig(code)?),
            ScriptLang::Snowflake => Some(windmill_parser_sql::parse_snowflake_sig(code)?),
            ScriptLang::Graphql => None,
            ScriptLang::Mssql => Some(windmill_parser_sql::parse_mssql_sig(code)?),
            ScriptLang::DuckDb => Some(windmill_parser_sql::parse_duckdb_sig(code)?),
            ScriptLang::OracleDB => Some(windmill_parser_sql::parse_oracledb_sig(code)?),
            #[cfg(feature = "php")]
            ScriptLang::Php => Some(windmill_parser_php::parse_php_signature(
                code,
                main_override,
            )?),
            #[cfg(not(feature = "php"))]
            ScriptLang::Php => None,
            #[cfg(feature = "rust")]
            ScriptLang::Rust => Some(windmill_parser_rust::parse_rust_signature(code)?),
            #[cfg(not(feature = "rust"))]
            ScriptLang::Rust => None,
            ScriptLang::Ansible => Some(windmill_parser_yaml::parse_ansible_sig(code)?),
            #[cfg(feature = "csharp")]
            ScriptLang::CSharp => Some(windmill_parser_csharp::parse_csharp_signature(code)?),
            #[cfg(not(feature = "csharp"))]
            ScriptLang::CSharp => None,
            #[cfg(feature = "nu")]
            ScriptLang::Nu => Some(windmill_parser_nu::parse_nu_signature(code)?),
            #[cfg(not(feature = "nu"))]
            ScriptLang::Nu => None,
            #[cfg(feature = "java")]
            ScriptLang::Java => Some(windmill_parser_java::parse_java_signature(code)?),
            #[cfg(not(feature = "java"))]
            ScriptLang::Java => None,
            // for related places search: ADD_NEW_LANG
        }
    } else {
        None
    })
}
