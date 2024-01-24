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
use prometheus::{
    core::{AtomicI64, GenericGauge},
    IntCounter,
};
use reqwest::Response;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::{types::Json, Pool, Postgres};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::Hash,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};
use windmill_parser_py_imports::parse_relative_imports;

use uuid::Uuid;
#[cfg(feature = "enterprise")]
use windmill_common::ee::{get_license_plan, LicensePlan};
use windmill_common::{
    error::{self, to_anyhow, Error},
    flows::{FlowModule, FlowModuleValue, FlowValue},
    get_latest_deployed_hash_for_path,
    jobs::{JobKind, JobPayload, QueuedJob},
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang},
    users::{SUPERADMIN_NOTIFICATION_EMAIL, SUPERADMIN_SECRET_EMAIL, SUPERADMIN_SYNC_EMAIL},
    utils::{rd_string, StripPath},
    worker::{
        to_raw_value, to_raw_value_owned, update_ping, CLOUD_HOSTED, WORKER_CONFIG, WORKER_GROUP,
    },
    DB, IS_READY, METRICS_DEBUG_ENABLED, METRICS_ENABLED,
};
use windmill_queue::{
    canceled_job_to_result, empty_result, get_queued_job, pull, push, register_metric, CanceledBy,
    PushArgs, PushIsolationLevel, WrappedError, HTTP_CLIENT,
};

use serde_json::{json, value::RawValue, Value};

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
use crate::global_cache::{copy_all_piptars_from_bucket, untar_all_piptars};

use windmill_queue::{add_completed_job, add_completed_job_error};

use crate::{
    bash_executor::{handle_bash_job, handle_powershell_job, ANSI_ESCAPE_RE},
    bun_executor::{gen_lockfile, get_trusted_deps, handle_bun_job},
    common::{
        build_args_map, get_cached_resource_value_if_valid, hash_args, read_result, save_in_cache,
        write_file,
    },
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
use crate::{
    bigquery_executor::do_bigquery, mssql_executor::do_mssql, snowflake_executor::do_snowflake,
};

pub async fn create_token_for_owner_in_bg(
    db: &Pool<Postgres>,
    job: &QueuedJob,
) -> Arc<RwLock<String>> {
    let rw_lock = Arc::new(RwLock::new(String::new()));
    // skipping test runs
    if job.workspace_id != "" {
        let mut locked = rw_lock.clone().write_owned().await;
        let db = db.clone();
        let w_id = job.workspace_id.clone();
        let owner = job.permissioned_as.clone();
        let email = job.email.clone();
        let job_id = job.id.clone();
        tokio::spawn(async move {
            let token = create_token_for_owner(
                &db.clone(),
                &w_id,
                &owner,
                "ephemeral-script",
                *SCRIPT_TOKEN_EXPIRY,
                &email,
                &job_id,
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
    expires_in: u64,
    email: &str,
    job_id: &Uuid,
) -> error::Result<String> {
    // TODO: Bad implementation. We should not have access to this DB here.
    if let Some(token) = JOB_TOKEN.as_ref() {
        return Ok(token.clone());
    }

    let token: String = rd_string(30);
    let is_super_admin =
        sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
            .fetch_optional(db)
            .await?
            .unwrap_or(false)
            || email == SUPERADMIN_SECRET_EMAIL
            || email == SUPERADMIN_NOTIFICATION_EMAIL
            || owner == SUPERADMIN_SYNC_EMAIL;

    sqlx::query_scalar!(
        "INSERT INTO token
            (workspace_id, token, owner, label, expiration, super_admin, email, job)
            VALUES ($1, $2, $3, $4, now() + ($5 || ' seconds')::interval, $6, $7, $8)",
        &w_id,
        token,
        owner,
        label,
        expires_in.to_string(),
        is_super_admin,
        email,
        job_id
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
pub const POWERSHELL_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "powershell");

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

    static ref JOB_TOKEN: Option<String> = std::env::var("JOB_TOKEN").ok();

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

    pub static ref KEEP_JOB_DIR: AtomicBool = AtomicBool::new(std::env::var("KEEP_JOB_DIR")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false));

    pub static ref NO_PROXY: Option<String> = std::env::var("no_proxy").ok().or(std::env::var("NO_PROXY").ok());
    pub static ref HTTP_PROXY: Option<String> = std::env::var("http_proxy").ok().or(std::env::var("HTTP_PROXY").ok());
    pub static ref HTTPS_PROXY: Option<String> = std::env::var("https_proxy").ok().or(std::env::var("HTTPS_PROXY").ok());
    pub static ref DENO_PATH: String = std::env::var("DENO_PATH").unwrap_or_else(|_| "/usr/bin/deno".to_string());
    pub static ref BUN_PATH: String = std::env::var("BUN_PATH").unwrap_or_else(|_| "/usr/bin/bun".to_string());
    pub static ref NODE_PATH: String = std::env::var("NODE_PATH").unwrap_or_else(|_| "/usr/bin/node".to_string());
    pub static ref POWERSHELL_PATH: String = std::env::var("POWERSHELL_PATH").unwrap_or_else(|_| "/usr/bin/pwsh".to_string());
    pub static ref NSJAIL_PATH: String = std::env::var("NSJAIL_PATH").unwrap_or_else(|_| "nsjail".to_string());
    pub static ref PATH_ENV: String = std::env::var("PATH").unwrap_or_else(|_| String::new());
    pub static ref HOME_ENV: String = std::env::var("HOME").unwrap_or_else(|_| String::new());
    pub static ref TZ_ENV: String = std::env::var("TZ").unwrap_or_else(|_| String::new());
    pub static ref GOPRIVATE: Option<String> = std::env::var("GOPRIVATE").ok();
    pub static ref GOPROXY: Option<String> = std::env::var("GOPROXY").ok();
    pub static ref NETRC: Option<String> = std::env::var("NETRC").ok();

    pub static ref NPM_CONFIG_REGISTRY: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref BUNFIG_INSTALL_SCOPES: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));

    pub static ref PIP_EXTRA_INDEX_URL: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref JOB_DEFAULT_TIMEOUT: Arc<RwLock<Option<i32>>> = Arc::new(RwLock::new(None));

    pub static ref TAR_CACHE_RATE: i32 = std::env::var("TAR_CACHE_RATE")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(100);

    static ref WORKER_STARTED: Option<prometheus::IntGauge> = if METRICS_ENABLED.load(Ordering::Relaxed) { Some(prometheus::register_int_gauge!(
        "worker_started",
        "Total number of workers started."
    )
    .unwrap()) } else { None };

    static ref WORKER_UPTIME_OPTS: prometheus::Opts = prometheus::opts!(
        "worker_uptime",
        "Total number of seconds since the worker has started"
    );

    static ref MAX_TIMEOUT: u64 = std::env::var("TIMEOUT")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or_else(|| if *CLOUD_HOSTED { DEFAULT_CLOUD_TIMEOUT } else { DEFAULT_SELFHOSTED_TIMEOUT });

    pub static ref MAX_WAIT_FOR_SIGTERM: u64 = std::env::var("MAX_WAIT_FOR_SIGTERM")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or_else(|| 5);

    pub static ref MAX_TIMEOUT_DURATION: Duration = Duration::from_secs(*MAX_TIMEOUT);

    pub static ref SCRIPT_TOKEN_EXPIRY: u64 = std::env::var("SCRIPT_TOKEN_EXPIRY")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or(*MAX_TIMEOUT);

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


    pub static ref WORKER_EXECUTION_COUNT: Arc<RwLock<HashMap<String, IntCounter>>> = Arc::new(RwLock::new(HashMap::new()));
    pub static ref WORKER_EXECUTION_DURATION_COUNTER: Arc<RwLock<HashMap<String, prometheus::Counter>>> = Arc::new(RwLock::new(HashMap::new()));

    pub static ref WORKER_EXECUTION_DURATION: Arc<RwLock<HashMap<String, prometheus::Histogram>>> = Arc::new(RwLock::new(HashMap::new()));

}
//only matter if CLOUD_HOSTED
pub const MAX_RESULT_SIZE: usize = 1024 * 1024 * 2; // 2MB

pub struct AuthedClientBackgroundTask {
    pub base_internal_url: String,
    pub workspace: String,
    pub token: Arc<RwLock<String>>,
}

impl AuthedClientBackgroundTask {
    pub async fn get_authed(&self) -> AuthedClient {
        return AuthedClient {
            base_internal_url: self.base_internal_url.clone(),
            workspace: self.workspace.clone(),
            token: self.get_token().await,
            force_client: None,
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
    pub force_client: Option<reqwest::Client>,
}

impl AuthedClient {
    pub async fn get(&self, url: &str, query: Vec<(&str, String)>) -> anyhow::Result<Response> {
        Ok(self
            .force_client
            .as_ref()
            .unwrap_or(&HTTP_CLIENT)
            .get(url)
            .query(&query)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .header(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.token))?,
            )
            .send()
            .await?)
    }

    pub async fn get_resource_value<T: DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let url = format!(
            "{}/api/w/{}/resources/get_value/{}",
            self.base_internal_url, self.workspace, path
        );
        let response = self.get(&url, vec![]).await?;
        match response.status().as_u16() {
            200u16 => Ok(response.json::<T>().await?),
            _ => Err(anyhow::anyhow!(response.text().await.unwrap_or_default())),
        }
    }

    pub async fn get_variable_value(&self, path: &str) -> anyhow::Result<String> {
        let url = format!(
            "{}/api/w/{}/variables/get_value/{}",
            self.base_internal_url, self.workspace, path
        );
        let response = self.get(&url, vec![]).await?;
        match response.status().as_u16() {
            200u16 => Ok(response.json::<String>().await?),
            _ => Err(anyhow::anyhow!(response.text().await.unwrap_or_default())),
        }
    }

    pub async fn get_resource_value_interpolated<T: DeserializeOwned>(
        &self,
        path: &str,
        job_id: Option<String>,
    ) -> anyhow::Result<T> {
        let url = format!(
            "{}/api/w/{}/resources/get_value_interpolated/{}",
            self.base_internal_url, self.workspace, path
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &job_id {
            query.push(("job_id", v.to_string()));
        }
        let response = self.get(&url, query).await?;
        match response.status().as_u16() {
            200u16 => Ok(response.json::<T>().await?),
            _ => Err(anyhow::anyhow!(response.text().await.unwrap_or_default())),
        }
    }

    pub async fn get_completed_job_result<T: DeserializeOwned>(
        &self,
        path: &str,
        json_path: Option<String>,
    ) -> anyhow::Result<T> {
        let url = format!(
            "{}/api/w/{}/jobs_u/completed/get_result/{}",
            self.base_internal_url, self.workspace, path
        );
        let query = if let Some(json_path) = json_path {
            vec![("json_path", json_path)]
        } else {
            vec![]
        };
        let response = self.get(&url, query).await?;
        match response.status().as_u16() {
            200u16 => Ok(response.json::<T>().await?),
            _ => Err(anyhow::anyhow!(response.text().await.unwrap_or_default())),
        }
    }

    pub async fn get_result_by_id<T: DeserializeOwned>(
        &self,
        flow_job_id: &str,
        node_id: &str,
        json_path: Option<String>,
    ) -> anyhow::Result<T> {
        let url = format!(
            "{}/api/w/{}/jobs/result_by_id/{}/{}",
            self.base_internal_url, self.workspace, flow_job_id, node_id
        );
        let query = if let Some(json_path) = json_path {
            vec![("json_path", json_path)]
        } else {
            vec![]
        };
        let response = self.get(&url, query).await?;
        match response.status().as_u16() {
            200u16 => Ok(response.json::<T>().await?),
            _ => Err(anyhow::anyhow!(response.text().await.unwrap_or_default())),
        }
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
    base_internal_url: String,
    db: Pool<Postgres>,
    worker_dir: String,
    same_worker_tx: Sender<Uuid>,
    rsmq: Option<R>,
    worker_name: &str,
    worker_save_completed_job_duration: Option<Arc<prometheus::Histogram>>,
    worker_flow_transition_duration: Option<Arc<prometheus::Histogram>>,
    job_completed_tx: Sender<SendResult>,
) {
    let token = jc.token.clone();
    let workspace = jc.job.workspace_id.clone();
    let client = AuthedClient {
        base_internal_url: base_internal_url.to_string(),
        workspace,
        token,
        force_client: None,
    };
    let job = jc.job.clone();
    let mem_peak = jc.mem_peak.clone();
    let canceled_by = jc.canceled_by.clone();
    if let Err(err) = process_completed_job(
        jc,
        &client,
        &db,
        &worker_dir,
        same_worker_tx.clone(),
        rsmq.clone(),
        worker_name,
        worker_save_completed_job_duration,
        worker_flow_transition_duration,
        job_completed_tx.clone(),
    )
    .await
    {
        handle_job_error(
            &db,
            &client,
            job.as_ref(),
            mem_peak,
            canceled_by,
            err,
            false,
            same_worker_tx.clone(),
            &worker_dir,
            rsmq.clone(),
            worker_name,
            job_completed_tx,
        )
        .await;
    }
}

#[derive(Clone)]
pub struct JobCompletedSender(
    Sender<SendResult>,
    Option<Arc<GenericGauge<AtomicI64>>>,
    Option<Arc<prometheus::Histogram>>,
);

impl JobCompletedSender {
    pub async fn send(
        &self,
        jc: JobCompleted,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<SendResult>> {
        if let Some(wj) = self.1.as_ref() {
            wj.inc()
        }
        let timer = self.2.as_ref().map(|x| x.start_timer());
        let r = self.0.send(SendResult::JobCompleted(jc)).await;
        timer.map(|x| x.stop_and_record());
        r
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
    agent_mode: bool,
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

    let worker_job_completed_channel_queue = if METRICS_DEBUG_ENABLED.load(Ordering::Relaxed)
        && METRICS_ENABLED.load(Ordering::Relaxed)
    {
        Some(Arc::new(
            prometheus::register_int_gauge!(prometheus::opts!(
                "worker_job_completed_channel_queue_length",
                "Queue length of the job completed channel queue",
            )
            .const_label("name", &worker_name),)
            .expect("register prometheus metric"),
        ))
    } else {
        None
    };

    let worker_completed_channel_queue_send_duration =
        if METRICS_DEBUG_ENABLED.load(Ordering::Relaxed) && METRICS_ENABLED.load(Ordering::Relaxed)
        {
            Some(Arc::new(
                prometheus::register_histogram!(prometheus::HistogramOpts::new(
                    "worker_completed_channel_queue_duration",
                    "Duration sending job to completed job channel",
                )
                .const_label("name", &worker_name),)
                .expect("register prometheus metric"),
            ))
        } else {
            None
        };

    let worker_save_completed_job_duration = if METRICS_DEBUG_ENABLED.load(Ordering::Relaxed)
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

    let worker_code_execution_duration = if METRICS_DEBUG_ENABLED.load(Ordering::Relaxed)
        && METRICS_ENABLED.load(Ordering::Relaxed)
    {
        Some(Arc::new(
            prometheus::register_histogram!(prometheus::HistogramOpts::new(
                "worker_code_execution_duration",
                "Duration of executing the job itself without the saving or flow transition",
            )
            .const_label("name", &worker_name),)
            .expect("register prometheus metric"),
        ))
    } else {
        None
    };

    let worker_flow_initial_transition_duration = if METRICS_DEBUG_ENABLED.load(Ordering::Relaxed)
        && METRICS_ENABLED.load(Ordering::Relaxed)
    {
        Some(Arc::new(
            prometheus::register_histogram!(prometheus::HistogramOpts::new(
                "worker_flow_initial_transition_duration",
                "Duration sending job to completed job channel",
            )
            .const_label("name", &worker_name),)
            .expect("register prometheus metric"),
        ))
    } else {
        None
    };

    let worker_flow_transition_duration = if METRICS_DEBUG_ENABLED.load(Ordering::Relaxed)
        && METRICS_ENABLED.load(Ordering::Relaxed)
    {
        Some(Arc::new(
            prometheus::register_histogram!(prometheus::HistogramOpts::new(
                "worker_flow_transition_duration",
                "Duration of doing a flow transition after the job is completed",
            )
            .const_label("name", &worker_name),)
            .expect("register prometheus metric"),
        ))
    } else {
        None
    };

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

    let mut jobs_executed = 0;

    if let Some(ws) = WORKER_STARTED.as_ref() {
        ws.inc();
    }

    #[cfg(feature = "enterprise")]
    if i_worker == 1 {
        if let Some(ref s) = S3_CACHE_BUCKET.clone() {
            if matches!(get_license_plan().await, LicensePlan::Pro) {
                tracing::warn!("S3 cache not available in the pro plan");
            } else if crate::global_cache::worker_s3_bucket_sync_enabled(&db).await {
                let bucket = s.to_string();

                copy_all_piptars_from_bucket(&bucket).await;
                if let Err(e) = untar_all_piptars().await {
                    tracing::error!("Failed to untar pip tarballs: {:?}", e);
                }
            }
        }
    }

    let (same_worker_tx, mut same_worker_rx) = mpsc::channel::<Uuid>(5);

    let (job_completed_tx, mut job_completed_rx) = mpsc::channel::<SendResult>(3);

    let job_completed_tx = JobCompletedSender(
        job_completed_tx,
        worker_job_completed_channel_queue.clone(),
        worker_completed_channel_queue_send_duration,
    );

    let db2 = db.clone();
    let base_internal_url2 = base_internal_url.to_string();
    let same_worker_tx2 = same_worker_tx.clone();
    let rsmq2 = rsmq.clone();
    let worker_dir2 = worker_dir.clone();
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

    let worker_job_completed_channel_queue2 = worker_job_completed_channel_queue.clone();
    let worker_save_completed_job_duration2 = worker_save_completed_job_duration.clone();
    let worker_flow_transition_duration2 = worker_flow_transition_duration.clone();

    let worker_name2 = worker_name.clone();
    let killpill_tx2 = killpill_tx.clone();
    let job_completed_sender = job_completed_tx.0.clone();
    let send_result = tokio::spawn(async move {
        while let Some(sr) = job_completed_rx.recv().await {
            match sr {
                SendResult::JobCompleted(jc) => {
                    if let Some(wj) = worker_job_completed_channel_queue2.as_ref() {
                        wj.dec();
                    }
                    let base_internal_url2 = base_internal_url2.clone();
                    let worker_dir2 = worker_dir2.clone();
                    let db2 = db2.clone();
                    let same_worker_tx2 = same_worker_tx2.clone();
                    let rsmq2 = rsmq2.clone();
                    let worker_name = worker_name2.clone();
                    if matches!(jc.job.job_kind, JobKind::Noop) || is_dedicated_worker {
                        thread_count.fetch_add(1, Ordering::SeqCst);
                        let thread_count = thread_count.clone();

                        loop {
                            if thread_count.load(Ordering::Relaxed) < 4 {
                                break;
                            }
                            tokio::time::sleep(Duration::from_millis(3)).await;
                        }

                        #[cfg(feature = "benchmark")]
                        let send_duration = send_duration2.clone();
                        #[cfg(feature = "benchmark")]
                        let process_duration = process_duration.clone();
                        #[cfg(feature = "benchmark")]
                        let completed_jobs = completed_jobs.clone();
                        #[cfg(feature = "benchmark")]
                        let main_duration = main_duration2.clone();

                        let worker_save_completed_job_duration2 =
                            worker_save_completed_job_duration.clone();
                        let worker_flow_transition_duration2 =
                            worker_flow_transition_duration.clone();
                        let killpill_tx = killpill_tx2.clone();
                        let job_completed_sender = job_completed_sender.clone();
                        tokio::spawn(async move {
                            #[cfg(feature = "benchmark")]
                            let process_start = Instant::now();

                            let is_dependency_job = matches!(
                                jc.job.job_kind,
                                JobKind::Dependencies | JobKind::FlowDependencies
                            );
                            handle_receive_completed_job(
                                jc,
                                base_internal_url2,
                                db2.clone(),
                                worker_dir2,
                                same_worker_tx2,
                                rsmq2,
                                &worker_name,
                                worker_save_completed_job_duration2.clone(),
                                worker_flow_transition_duration2.clone(),
                                job_completed_sender.clone(),
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
                            if is_dependency_job && is_dedicated_worker {
                                tracing::error!("Dedicated worker executed a dependency job, a new script has been deployed. Exiting expecting to be restarted.");
                                sqlx::query!("UPDATE config SET config = config WHERE name = $1", format!("worker__{}", *WORKER_GROUP))
                            .execute(&db2)
                            .await
                            .expect("update config to trigger restart of all dedicated workers at that config");
                                killpill_tx.send(()).unwrap_or_default();
                            }
                        });
                    } else {
                        handle_receive_completed_job(
                            jc,
                            base_internal_url2,
                            db2,
                            worker_dir2,
                            same_worker_tx2,
                            rsmq2,
                            &worker_name,
                            worker_save_completed_job_duration2.clone(),
                            worker_flow_transition_duration2.clone(),
                            job_completed_sender.clone(),
                        )
                        .await;
                    }
                }
                SendResult::UpdateFlow {
                    flow,
                    w_id,
                    success,
                    result,
                    worker_dir,
                    stop_early_override,
                    token,
                } => {
                    // let r;
                    if let Err(e) = update_flow_status_after_job_completion(
                        &db2,
                        &AuthedClient {
                            base_internal_url: base_internal_url2.to_string(),
                            workspace: w_id.clone(),
                            token: token.clone(),
                            force_client: None,
                        },
                        flow,
                        &Uuid::nil(),
                        &w_id,
                        success,
                        &result,
                        true,
                        same_worker_tx2.clone(),
                        &worker_dir,
                        stop_early_override,
                        rsmq2.clone(),
                        &worker_name2,
                        job_completed_sender.clone(),
                    )
                    .await
                    {
                        tracing::error!("Error updating flow status after job completion: {e}");
                    }
                }
                SendResult::Kill => {
                    break;
                }
            }
        }

        tracing::info!("stopped processing new completed jobs");
        while thread_count.load(Ordering::SeqCst) > 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        tracing::info!("finished processing all completed jobs");
    });

    let mut last_executed_job: Option<Instant> = None;
    let mut last_checked_suspended = Instant::now();

    #[cfg(feature = "benchmark")]
    let mut started = false;

    #[cfg(feature = "benchmark")]
    let mut infos = BenchmarkInfo { iters: 0, timings: vec![] };

    let vacuum_shift = rand::thread_rng().gen_range(0..VACUUM_PERIOD);

    IS_READY.store(true, Ordering::Relaxed);
    tracing::info!(worker = %worker_name, "listening for jobs, WORKER_GROUP: {}, config: {:?}", *WORKER_GROUP, WORKER_CONFIG.read().await);

    // (dedi_path, dedicated_worker_tx, dedicated_worker_handle)
    // Option<Sender<Arc<QueuedJob>>>,
    // Option<JoinHandle<()>>,

    let mut dedicated_handles: Vec<JoinHandle<()>> = vec![];
    let (dedicated_workers, is_flow_worker): (HashMap<String, Sender<Arc<QueuedJob>>>, bool) =
        if let Some(_wp) = WORKER_CONFIG.read().await.dedicated_worker.clone() {
            let mut hm = HashMap::new();
            let is_flow_worker;
            if let Some(flow_path) = _wp.path.strip_prefix("flow/") {
                is_flow_worker = true;
                let value = sqlx::query_scalar!(
                    "SELECT value FROM flow WHERE path = $1 AND workspace_id = $2",
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
            (hm, is_flow_worker)
        } else {
            (HashMap::new(), false)
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

    let worker_dedicated_channel_queue_send_duration = if is_dedicated_worker
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
    };

    loop {
        #[cfg(feature = "benchmark")]
        let loop_start = Instant::now();

        #[cfg(feature = "benchmark")]
        let mut timing = vec![];

        if let Some(wk) = worker_busy.as_ref() {
            wk.set(0);
        }
        if let Some(ref um) = uptime_metric {
            um.inc_by(
                ((start_time.elapsed().as_millis() as f64) / 1000.0 - um.get())
                    .try_into()
                    .unwrap(),
            );
        }

        if last_ping.elapsed().as_secs() > NUM_SECS_PING {
            let tags = WORKER_CONFIG.read().await.worker_tags.clone();

            if let Err(e) = sqlx::query!(
                "UPDATE worker_ping SET ping_at = now(), jobs_executed = $1, custom_tags = $2 WHERE worker = $3",
                jobs_executed,
                tags.as_slice(),
                &worker_name
            )
            .execute(db)
            .await
            {
                tracing::error!("failed to update worker ping, exiting: {}", e);
                killpill_tx.send(()).unwrap_or_default();
            }

            last_ping = Instant::now();
        }

        if (jobs_executed as u32 + vacuum_shift) % VACUUM_PERIOD == 0 {
            if let Err(e) = sqlx::query!("VACUUM queue").execute(db).await {
                jobs_executed += 1;
                tracing::error!(worker = %worker_name, "failed to vacuum queue: {}", e);
            }
            tracing::info!(worker = %worker_name, "vacuumed queue and completed_job");
        }

        let next_job = {
            // println!("2: {:?}",  instant.elapsed());
            #[cfg(feature = "benchmark")]
            if !started {
                started = true
            }

            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    println!("received killpill for worker {}", i_worker);
                    job_completed_tx.0.send(SendResult::Kill).await.unwrap();
                    break
                },
                Some(job_id) = same_worker_rx.recv() => {
                    sqlx::query_as::<_, QueuedJob>("SELECT * FROM queue WHERE id = $1")
                    .bind(job_id)
                    .fetch_optional(db)
                    .await
                    .map_err(|_| Error::InternalErr("Impossible to fetch same_worker job".to_string()))
                },
                (job, timer) = async {
                    let pull_time = Instant::now();
                    let suspend_first = if last_checked_suspended.elapsed().as_secs() > 3 {
                        last_checked_suspended = Instant::now();
                        true
                    } else { false };
                    pull(&db, rsmq.clone(), suspend_first).map(|x| (x, pull_time)).await
                } => {
                    add_time!(timing, loop_start, "post pull");
                    tracing::debug!("pulled job: {:?}", job.as_ref().ok().and_then(|x| x.as_ref().map(|y| y.id)));
                    let duration_pull_s = timer.elapsed().as_secs_f64();
                    let err_pull = job.is_ok();
                    let empty = job.as_ref().is_ok_and(|x| x.is_none());
                    if !agent_mode && duration_pull_s > 0.5 {
                        tracing::warn!("pull took more than 0.5s ({duration_pull_s}), this is a sign that the database is VERY undersized for this load. empty: {empty}, err: {err_pull}");
                        if empty {
                            if let Some(wp) = worker_pull_over_500_counter_empty.as_ref() {
                                wp.inc();
                            }
                        } else if let Some(wp) = worker_pull_over_500_counter.as_ref() {
                            wp.inc();
                        }

                    } else if !agent_mode && duration_pull_s > 0.1 {
                        tracing::warn!("pull took more than 0.1s ({duration_pull_s}) this is a sign that the database is undersized for this load. empty: {empty}, err: {err_pull}");
                        if empty {
                            if let Some(wp) = worker_pull_over_100_counter_empty.as_ref() {
                                wp.inc();
                            }
                        } else if let Some(wp) = worker_pull_over_100_counter.as_ref() {
                            wp.inc();
                        }
                    }
                    if let Ok(j) = job.as_ref() {
                        if j.is_some() {
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
                    job
                },
            }
        };

        if let Some(wb) = worker_busy.as_ref() {
            wb.set(1);
        }

        match next_job {
            Ok(Some(job)) => {
                last_executed_job = None;
                jobs_executed += 1;

                if matches!(job.job_kind, JobKind::Script | JobKind::Preview) {
                    if !dedicated_workers.is_empty() {
                        let key_o = if is_flow_worker {
                            job.flow_step_id.as_ref().map(|x| x.to_string())
                        } else {
                            job.script_path.as_ref().map(|x| x.to_string())
                        };
                        if let Some(key) = key_o {
                            if let Some(dedicated_worker_tx) = dedicated_workers.get(&key) {
                                #[cfg(feature = "benchmark")]
                                main_duration.fetch_add(
                                    loop_start.elapsed().as_millis() as usize,
                                    Ordering::SeqCst,
                                );
                                #[cfg(feature = "benchmark")]
                                let send_start = Instant::now();

                                let timer = worker_dedicated_channel_queue_send_duration
                                    .as_ref()
                                    .map(|x| x.start_timer());

                                if let Err(e) = dedicated_worker_tx.send(Arc::new(job)).await {
                                    tracing::info!("failed to send jobs to dedicated workers. Likely dedicated worker has been shut down. This is normal: {e:?}");
                                }

                                timer.map(|x| x.stop_and_record());

                                #[cfg(feature = "benchmark")]
                                send_duration.fetch_add(
                                    send_start.elapsed().as_millis() as usize,
                                    Ordering::SeqCst,
                                );
                                continue;
                            }
                        }
                    }
                }
                if matches!(job.job_kind, JobKind::Noop) {
                    #[cfg(feature = "benchmark")]
                    main_duration
                        .fetch_add(loop_start.elapsed().as_millis() as usize, Ordering::SeqCst);
                    #[cfg(feature = "benchmark")]
                    let send_start = Instant::now();

                    job_completed_tx
                        .send(JobCompleted {
                            job: Arc::new(job),
                            success: true,
                            result: empty_result(),
                            logs: String::new(),
                            mem_peak: 0,
                            cached_res_path: None,
                            token: "".to_string(),
                            canceled_by: None,
                        })
                        .await
                        .expect("send job completed");

                    #[cfg(feature = "benchmark")]
                    send_duration
                        .fetch_add(send_start.elapsed().as_millis() as usize, Ordering::SeqCst);
                } else {
                    let token = create_token_for_owner_in_bg(&db, &job).await;

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
                    };

                    let tag = job.tag.clone();
                    let arc_job = Arc::new(job);
                    if let Some(err) = handle_queued_job(
                        arc_job.clone(),
                        db,
                        &authed_client,
                        &worker_name,
                        &worker_dir,
                        &job_dir,
                        same_worker_tx.clone(),
                        base_internal_url,
                        rsmq.clone(),
                        job_completed_tx.clone(),
                        worker_flow_initial_transition_duration.clone(),
                        worker_code_execution_duration.clone(),
                    )
                    .await
                    .err()
                    {
                        handle_job_error(
                            db,
                            &authed_client.get_authed().await,
                            arc_job.as_ref(),
                            0,
                            None,
                            err,
                            false,
                            same_worker_tx.clone(),
                            &worker_dir,
                            rsmq.clone(),
                            &worker_name,
                            (&job_completed_tx.0).clone(),
                        )
                        .await;
                    };

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

                    if !KEEP_JOB_DIR.load(Ordering::Relaxed) && !(arc_job.is_flow() && same_worker)
                    {
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
                let _timer = if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                    Some(Instant::now())
                } else {
                    None
                };

                #[cfg(feature = "benchmark")]
                tracing::info!("no job found");

                tokio::time::sleep(Duration::from_millis(*SLEEP_QUEUE)).await;
                _timer.map(|timer| {
                    let duration = timer.elapsed().as_secs_f64();
                    if let Some(ws) = worker_sleep_duration_counter.as_ref() {
                        ws.inc_by(duration);
                    }
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

    drop(dedicated_workers);

    for handle in dedicated_handles {
        if let Err(e) = handle.await {
            tracing::error!("error in dedicated worker waiting for it to end: {:?}", e)
        }
    }

    drop(job_completed_tx);

    send_result.await.expect("send result failed");
    println!("worker {} exited", i_worker);
}

type DedicatedWorker = (String, Sender<Arc<QueuedJob>>, Option<JoinHandle<()>>);

// spawn one dedicated worker per compatible steps of the flow, associating the node id to the dedicated worker channel send
#[async_recursion]
async fn spawn_dedicated_workers_for_flow(
    modules: &Vec<FlowModule>,
    w_id: &str,
    path: &str,
    killpill_tx: tokio::sync::broadcast::Sender<()>,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    db: &Pool<Postgres>,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    job_completed_tx: &JobCompletedSender,
) -> Vec<DedicatedWorker> {
    let mut workers = vec![];
    let mut script_path_to_worker: HashMap<String, Sender<Arc<QueuedJob>>> = HashMap::new();
    for module in modules.iter() {
        match &module.value {
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
                    path,
                    w_id,
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
                        path,
                        w_id,
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
                        path,
                        w_id,
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
    }
    workers
}

enum SpawnWorker {
    Script { path: String, hash: Option<ScriptHash> },
    RawScript { path: String, content: String, lock: Option<String>, lang: ScriptLang },
}

// spawn one dedicated worker and return the key, the channel sender and the join handle
// note that for it will return none for language that do not support dedicated workers
// note that go using cache binary does not need dedicated workers so all languages are supported
async fn spawn_dedicated_worker(
    sw: SpawnWorker,
    w_id: &str,
    killpill_tx: tokio::sync::broadcast::Sender<()>,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    db: &Pool<Postgres>,
    worker_dir: &str,
    base_internal_url: &str,
    worker_name: &str,
    job_completed_tx: &JobCompletedSender,
    node_id: Option<String>,
) -> Option<DedicatedWorker> {
    #[cfg(not(feature = "enterprise"))]
    {
        tracing::error!("Dedicated worker is an enterprise feature");
        killpill_tx.send(()).expect("send");
        return None;
    }

    #[cfg(feature = "enterprise")]
    {
        let (dedicated_worker_tx, dedicated_worker_rx) =
            mpsc::channel::<Arc<QueuedJob>>(MAX_BUFFERED_DEDICATED_JOBS);
        let killpill_rx = killpill_rx.resubscribe();
        let db = db.clone();
        let base_internal_url = base_internal_url.to_string();
        let worker_name = worker_name.to_string();
        let job_completed_tx = job_completed_tx.clone();
        let job_dir = format!("{}/dedicated", worker_dir);
        tokio::fs::create_dir_all(&job_dir)
            .await
            .expect("create dir");

        let path = match &sw {
            SpawnWorker::RawScript { path, .. } => path.to_string(),
            SpawnWorker::Script { path, .. } => path.to_string(),
        };
        let path2 = path.clone();
        let w_id = w_id.to_string();

        let (content, lock, language, envs) = match sw {
            SpawnWorker::Script { path, hash } => {
                let q = if let Some(hash) = hash {
                    get_script_content_by_hash(&hash, &w_id, &db).await.map(
                        |r: ContentReqLangEnvs| Some((r.content, r.lockfile, r.language, r.envs)),
                    )
                } else {
                    sqlx::query_as::<_, (String, Option<String>, Option<ScriptLang>, Option<Vec<String>>)>(
                        "SELECT content, lock, language, envs FROM script WHERE path = $1 AND workspace_id = $2 AND
                            created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND workspace_id = $2 AND
                            deleted = false AND lock IS not NULL AND lock_error_logs IS NULL)",
                    )
                    .bind(&path)
                    .bind(&w_id)
                    .fetch_optional(&db)
                    .await
                    .map_err(|e| Error::InternalErr(format!("expected content and lock: {e}")))
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
            SpawnWorker::RawScript { content, lock, lang, .. } => (content, lock, Some(lang), None),
        };

        match language {
            Some(ScriptLang::Python3) | Some(ScriptLang::Bun) | Some(ScriptLang::Deno) => {}
            _ => return None,
        }

        let handle = tokio::spawn(async move {
            let token = if let Some(token) = JOB_TOKEN.as_ref() {
                token.clone()
            } else {
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
                    )
                    .await
                }
                _ => unreachable!("Non supported language for dedicated worker"),
            } {
                tracing::error!("error in dedicated worker: {:?}", e);
            };
        });
        return Some((node_id.unwrap_or(path2), dedicated_worker_tx, Some(handle)));
        // (Some(dedi_path), Some(dedicated_worker_tx), Some(handle))
    }
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
                dedicated_worker: None,
            }),
            PushArgs::empty(),
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
    JobCompleted {
        job,
        result,
        logs,
        mem_peak,
        success,
        cached_res_path,
        canceled_by,
        ..
    }: JobCompleted,
    client: &AuthedClient,
    db: &DB,
    worker_dir: &str,
    same_worker_tx: Sender<Uuid>,
    rsmq: Option<R>,
    worker_name: &str,
    worker_save_completed_job_duration: Option<Arc<prometheus::Histogram>>,
    worker_flow_transition_duration: Option<Arc<prometheus::Histogram>>,
    job_completed_tx: Sender<SendResult>,
) -> windmill_common::error::Result<()> {
    if success {
        // println!("bef completed job{:?}",  SystemTime::now());
        if let Some(cached_path) = cached_res_path {
            save_in_cache(db, client, &job, cached_path.to_string(), &result).await;
        }

        let timer = worker_save_completed_job_duration
            .as_ref()
            .map(|x| x.start_timer());
        add_completed_job(
            db,
            &job,
            true,
            false,
            Json(&result),
            logs,
            mem_peak.to_owned(),
            canceled_by,
            rsmq.clone(),
        )
        .await?;
        timer.map(|x| x.stop_and_record());

        if job.is_flow_step {
            if let Some(parent_job) = job.parent_job {
                let timer = worker_flow_transition_duration
                    .as_ref()
                    .map(|x| x.start_timer());
                update_flow_status_after_job_completion(
                    db,
                    client,
                    parent_job,
                    &job.id,
                    &job.workspace_id,
                    true,
                    &result,
                    false,
                    same_worker_tx.clone(),
                    &worker_dir,
                    None,
                    rsmq.clone(),
                    worker_name,
                    job_completed_tx,
                )
                .await?;
                timer.map(|x| x.stop_and_record());
            }
        }
    } else {
        let result = add_completed_job_error(
            db,
            &job,
            logs.to_string(),
            mem_peak.to_owned(),
            canceled_by,
            serde_json::from_str(result.get()).unwrap_or_else(
                |_| json!({ "message": format!("Non serializable error: {}", result.get()) }),
            ),
            rsmq.clone(),
            worker_name,
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
                    &serde_json::value::to_raw_value(&result).unwrap(),
                    false,
                    same_worker_tx,
                    &worker_dir,
                    None,
                    rsmq,
                    worker_name,
                    job_completed_tx,
                )
                .await?;
            }
        }
    }
    Ok(())
}

// fn build_language_metrics(
//     worker_execution_failed: &HashMap<
//         Option<ScriptLang>,
//         prometheus::core::GenericCounter<prometheus::core::AtomicU64>,
//     >,
//     language: &Option<ScriptLang>,
// ) -> Option<Metrics> {
//     let metrics = if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
//         Some(Metrics {
//             worker_execution_failed: worker_execution_failed
//                 .get(language)
//                 .expect("no timer found")
//                 .clone(),
//         })
//     } else {
//         None
//     };
//     metrics
// }

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
    mem_peak: i32,
    canceled_by: Option<CanceledBy>,
    err: Error,
    unrecoverable: bool,
    same_worker_tx: Sender<Uuid>,
    worker_dir: &str,
    rsmq: Option<R>,
    worker_name: &str,
    job_completed_tx: Sender<SendResult>,
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
            format!("Unexpected error during job execution:\n{err:#?}"),
            mem_peak,
            canceled_by.clone(),
            err.clone(),
            rsmq_2,
            worker_name,
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

        let wrapped_error = WrappedError { error: err.clone() };
        let updated_flow = update_flow_status_after_job_completion(
            db,
            client,
            flow,
            &job_status_to_update,
            &job.workspace_id,
            false,
            &serde_json::value::to_raw_value(&wrapped_error).unwrap(),
            unrecoverable,
            same_worker_tx,
            worker_dir,
            None,
            rsmq.clone(),
            worker_name,
            job_completed_tx.clone(),
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
                            mem_peak,
                            canceled_by.clone(),
                            e,
                            rsmq,
                            worker_name,
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

fn extract_error_value(log_lines: &str, i: i32) -> Box<RawValue> {
    return to_raw_value(
        &json!({"message": format!("ExitCode: {i}, last log lines:\n{}", ANSI_ESCAPE_RE.replace_all(log_lines.trim(), "").to_string()), "name": "ExecutionErr"}),
    );
}

pub enum SendResult {
    JobCompleted(JobCompleted),
    UpdateFlow {
        flow: Uuid,
        w_id: String,
        success: bool,
        result: Box<RawValue>,
        worker_dir: String,
        stop_early_override: Option<bool>,
        token: String,
    },
    Kill,
}

// db: &DB,
// client: &AuthedClient,
// flow: uuid::Uuid,
// job_id_for_status: &Uuid,
// w_id: &str,
// success: bool,
// result: &'a RawValue,
// unrecoverable: bool,
// same_worker_tx: Sender<Uuid>,
// worker_dir: &str,
// stop_early_override: Option<bool>,
// rsmq: Option<R>,
// worker_name: &str,

#[derive(Debug, Clone)]
pub struct JobCompleted {
    pub job: Arc<QueuedJob>,
    pub result: Box<RawValue>,
    pub logs: String,
    pub mem_peak: i32,
    pub success: bool,
    pub cached_res_path: Option<String>,
    pub token: String,
    pub canceled_by: Option<CanceledBy>,
}

async fn do_nativets(
    job: &QueuedJob,
    logs: String,
    client: &AuthedClientBackgroundTask,
    code: String,
    db: &Pool<Postgres>,
) -> windmill_common::error::Result<(Box<RawValue>, String)> {
    let args = build_args_map(job, client, db).await?.map(Json);
    let job_args = if args.is_some() {
        args.as_ref()
    } else {
        job.args.as_ref()
    };

    let result = eval_fetch_timeout(code.clone(), transpile_ts(code)?, job_args).await?;
    Ok((result.0, [logs, result.1].join("\n\n")))
}

#[derive(Deserialize, Serialize, Default)]
pub struct PreviousResult<'a> {
    #[serde(borrow)]
    pub previous_result: Option<&'a RawValue>,
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_queued_job<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    job: Arc<QueuedJob>,
    db: &DB,
    client: &AuthedClientBackgroundTask,
    worker_name: &str,
    worker_dir: &str,
    job_dir: &str,
    same_worker_tx: Sender<Uuid>,
    base_internal_url: &str,
    rsmq: Option<R>,
    job_completed_tx: JobCompletedSender,
    worker_flow_initial_transition_duration: Option<Arc<prometheus::Histogram>>,
    worker_code_execution_duration: Option<Arc<prometheus::Histogram>>,
) -> windmill_common::error::Result<()> {
    if job.canceled {
        return Err(Error::JsonErr(canceled_job_to_result(&job)));
    }
    if let Some(e) = &job.pre_run_error {
        return Err(Error::ExecutionErr(e.to_string()));
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
        let version_hash = if let Some(h) = job.script_hash {
            format!("script_{}", h.to_string())
        } else if let Some(rc) = job.raw_code.as_ref() {
            use std::hash::Hasher;
            let mut s = DefaultHasher::new();
            rc.hash(&mut s);
            format!("inline_{}", hex::encode(s.finish().to_be_bytes()))
        } else if let Some(rc) = job.raw_flow.as_ref() {
            use std::hash::Hasher;
            let mut s = DefaultHasher::new();
            serde_json::to_string(&rc.0)
                .unwrap_or_default()
                .hash(&mut s);
            format!("flow_{}", hex::encode(s.finish().to_be_bytes()))
        } else {
            "none".to_string()
        };
        let args_hash = hash_args(
            db,
            &client.get_authed().await,
            &job.workspace_id,
            &job.id,
            &job.args,
        )
        .await;
        if job.is_flow_step && !matches!(job.job_kind, JobKind::Flow) {
            let flow_path = sqlx::query_scalar!(
                "SELECT script_path FROM queue WHERE id = $1",
                &job.parent_job.unwrap()
            )
            .fetch_one(db)
            .await
            .map_err(|e| Error::InternalErr(format!("fetching step flow status: {e}")))?
            .ok_or_else(|| Error::InternalErr(format!("Expected script_path")))?;
            let step = step.unwrap_or(-1);
            Some(format!(
                "{flow_path}/cache/{version_hash}/{step}/{args_hash}"
            ))
        } else if let Some(script_path) = &job.script_path {
            Some(format!("{script_path}/cache/{version_hash}/{args_hash}"))
        } else {
            None
        }
    } else {
        None
    };

    if let Some(cached_res_path) = cached_res_path.clone() {
        let authed_client = client.get_authed().await;

        let cached_resource_value_maybe = get_cached_resource_value_if_valid(
            db,
            &authed_client,
            &job.id,
            &job.workspace_id,
            &cached_res_path,
        )
        .await;
        if let Some(cached_resource_value) = cached_resource_value_maybe {
            let logs = "Job skipped because args & path found in cache and not expired".to_string();
            job_completed_tx
                .send(JobCompleted {
                    job: job,
                    result: cached_resource_value,
                    logs,
                    mem_peak: 0,
                    canceled_by: None,
                    success: true,
                    cached_res_path: None,
                    token: authed_client.token,
                })
                .await
                .expect("send job completed");

            return Ok(());
        }
    };
    match job.job_kind {
        JobKind::FlowPreview | JobKind::Flow | JobKind::SingleScriptFlow => {
            let timer = worker_flow_initial_transition_duration.map(|x| x.start_timer());
            handle_flow(
                &job,
                db,
                &client.get_authed().await,
                None,
                same_worker_tx,
                worker_dir,
                rsmq,
                job_completed_tx.0.clone(),
            )
            .await?;
            timer.map(|x| x.stop_and_record());
        }
        _ => {
            let mut logs = "".to_string();
            let mut mem_peak: i32 = 0;
            let mut canceled_by: Option<CanceledBy> = None;
            // println!("handle queue {:?}",  SystemTime::now());
            if let Some(log_str) = &job.logs {
                logs.push_str(&log_str);
                logs.push_str("\n");
            }

            logs.push_str(&format!(
                "job {} on worker {} (tag: {})\n",
                &job.id, &worker_name, &job.tag
            ));

            #[cfg(not(feature = "enterprise"))]
            if job.concurrent_limit.is_some() {
                logs.push_str("---\n");
                logs.push_str("WARNING: This job has concurrency limits enabled. Concurrency limits are going to become an Enterprise Edition feature in the near future.\n");
                logs.push_str("---\n");
            }

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
                        &mut mem_peak,
                        &mut canceled_by,
                        job_dir,
                        db,
                        worker_name,
                        worker_dir,
                        base_internal_url,
                        &client.get_token().await,
                        rsmq.clone(),
                    )
                    .await
                }
                JobKind::FlowDependencies => handle_flow_dependency_job(
                    &job,
                    &mut logs,
                    &mut mem_peak,
                    &mut canceled_by,
                    job_dir,
                    db,
                    worker_name,
                    worker_dir,
                    base_internal_url,
                    &client.get_token().await,
                    rsmq.clone(),
                )
                .await
                .map(|()| serde_json::from_str("{}").unwrap()),
                JobKind::AppDependencies => handle_app_dependency_job(
                    &job,
                    &mut logs,
                    &mut mem_peak,
                    &mut canceled_by,
                    job_dir,
                    db,
                    worker_name,
                    worker_dir,
                    base_internal_url,
                    &client.get_token().await,
                    rsmq.clone(),
                )
                .await
                .map(|()| serde_json::from_str("{}").unwrap()),
                JobKind::Identity => Ok(job
                    .args
                    .as_ref()
                    .map(|x| x.get("previous_result"))
                    .flatten()
                    .map(|x| x.to_owned())
                    .unwrap_or_else(|| serde_json::from_str("{}").unwrap())),
                _ => {
                    let timer = worker_code_execution_duration.map(|x| x.start_timer());
                    let r = handle_code_execution_job(
                        job.as_ref(),
                        db,
                        client,
                        job_dir,
                        worker_dir,
                        &mut logs,
                        &mut mem_peak,
                        &mut canceled_by,
                        base_internal_url,
                        worker_name,
                    )
                    .await;
                    timer.map(|x| x.stop_and_record());
                    r
                }
            };

            //it's a test job, no need to update the db
            if job.as_ref().workspace_id == "" {
                return Ok(());
            }
            process_result(
                job,
                result,
                job_dir,
                job_completed_tx,
                logs,
                mem_peak,
                canceled_by,
                cached_res_path,
                client.get_token().await,
            )
            .await?;
        }
    }
    Ok(())
}

async fn process_result(
    job: Arc<QueuedJob>,
    result: error::Result<Box<RawValue>>,
    job_dir: &str,
    job_completed_tx: JobCompletedSender,
    logs: String,
    mem_peak: i32,
    canceled_by: Option<CanceledBy>,
    cached_res_path: Option<String>,
    token: String,
) -> error::Result<()> {
    match result {
        Ok(r) => {
            job_completed_tx
                .send(JobCompleted {
                    job: job,
                    result: r,
                    logs,
                    mem_peak,
                    canceled_by,
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

                    if res.as_ref().is_some_and(|x| !x.get().is_empty()) {
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
                err @ _ => to_raw_value(
                    &json!({"message": format!("error during execution of the script:\n{}", err), "name": "ExecutionErr"}),
                ),
            };

            // in the happy path and if job not a flow step, we can delegate updating the completed job in the background
            job_completed_tx
                .send(JobCompleted {
                    job: job,
                    result: to_raw_value(&error_value),
                    logs: logs,
                    mem_peak,
                    canceled_by,
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

struct ContentReqLangEnvs {
    content: String,
    lockfile: Option<String>,
    language: Option<ScriptLang>,
    envs: Option<Vec<String>>,
}

async fn get_hub_script_content_and_requirements(
    script_path: Option<String>,
    db: &DB,
) -> error::Result<ContentReqLangEnvs> {
    let script_path = script_path
        .clone()
        .ok_or_else(|| Error::InternalErr(format!("expected script path for hub script")))?;
    let mut script_path_iterator = script_path.split("/");
    script_path_iterator.next();
    let version = script_path_iterator
        .next()
        .ok_or_else(|| Error::InternalErr(format!("expected hub path to have version number")))?;
    let cache_path = format!("{HUB_CACHE_DIR}/{version}");
    let script;
    if tokio::fs::metadata(&cache_path).await.is_err() {
        script = get_full_hub_script_by_path(StripPath(script_path.to_string()), &HTTP_CLIENT, db)
            .await?;
        write_file(
            HUB_CACHE_DIR,
            &version,
            &serde_json::to_string(&script).map_err(to_anyhow)?,
        )
        .await?;
        tracing::info!("wrote hub script {script_path} to cache");
    } else {
        let cache_content = tokio::fs::read_to_string(cache_path).await?;
        script = serde_json::from_str(&cache_content).unwrap();
        tracing::info!("read hub script {script_path} from cache");
    }
    Ok(ContentReqLangEnvs {
        content: script.content,
        lockfile: script.lockfile,
        language: Some(script.language),
        envs: None,
    })
}

async fn get_script_content_by_path(
    script_path: Option<String>,
    w_id: &str,
    db: &DB,
) -> error::Result<ContentReqLangEnvs> {
    let script_path = script_path
        .clone()
        .ok_or_else(|| Error::InternalErr(format!("expected script path")))?;
    return if script_path.starts_with("hub/") {
        get_hub_script_content_and_requirements(Some(script_path), db).await
    } else {
        let (script_hash, ..) =
            get_latest_deployed_hash_for_path(db, w_id, script_path.as_str()).await?;
        get_script_content_by_hash(&script_hash, w_id, db).await
    };
}

async fn get_script_content_by_hash(
    script_hash: &ScriptHash,
    w_id: &str,
    db: &DB,
) -> error::Result<ContentReqLangEnvs> {
    let r = sqlx::query_as::<
        _,
        (
            String,
            Option<String>,
            Option<ScriptLang>,
            Option<Vec<String>>,
        ),
    >(
        "SELECT content, lock, language, envs FROM script WHERE hash = $1 AND workspace_id = $2",
    )
    .bind(script_hash.0)
    .bind(w_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::InternalErr(format!("expected content and lock")))?;
    Ok(ContentReqLangEnvs { content: r.0, lockfile: r.1, language: r.2, envs: r.3 })
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_code_execution_job(
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    job_dir: &str,
    worker_dir: &str,
    logs: &mut String,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    base_internal_url: &str,
    worker_name: &str,
) -> error::Result<Box<RawValue>> {
    let ContentReqLangEnvs { content: inner_content, lockfile: requirements_o, language, envs } =
        match job.job_kind {
            JobKind::Preview => ContentReqLangEnvs {
                content: job
                    .raw_code
                    .clone()
                    .unwrap_or_else(|| "no raw code".to_owned()),
                lockfile: job.raw_lock.clone(),
                language: job.language.to_owned(),
                envs: None,
            },
            JobKind::Script_Hub => {
                get_hub_script_content_and_requirements(job.script_path.clone(), db).await?
            }
            JobKind::Script => {
                get_script_content_by_hash(
                    &job.script_hash.unwrap_or(ScriptHash(0)),
                    &job.workspace_id,
                    db,
                )
                .await?
            }
            JobKind::DeploymentCallback => {
                get_script_content_by_path(job.script_path.clone(), &job.workspace_id, db).await?
            }
            _ => unreachable!(
                "handle_code_execution_job should never be reachable with a non-code execution job"
            ),
        };

    if language == Some(ScriptLang::Postgresql) {
        return do_postgresql(job, &client, &inner_content, db).await;
    } else if language == Some(ScriptLang::Mysql) {
        return do_mysql(job, &client, &inner_content, db).await;
    } else if language == Some(ScriptLang::Bigquery) {
        #[cfg(not(feature = "enterprise"))]
        {
            return Err(Error::ExecutionErr(
                "Bigquery is only available with an enterprise license".to_string(),
            ));
        }

        #[cfg(feature = "enterprise")]
        {
            return do_bigquery(job, &client, &inner_content, db).await;
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
            return do_snowflake(job, &client, &inner_content, db).await;
        }
    } else if language == Some(ScriptLang::Mssql) {
        #[cfg(not(feature = "enterprise"))]
        {
            return Err(Error::ExecutionErr(
                "Microsoft SQL server is only available with an enterprise license".to_string(),
            ));
        }

        #[cfg(feature = "enterprise")]
        {
            return do_mssql(job, &client, &inner_content, db).await;
        }
    } else if language == Some(ScriptLang::Graphql) {
        return do_graphql(job, &client, &inner_content, db).await;
    } else if language == Some(ScriptLang::Nativets) {
        logs.push_str("\n--- FETCH TS EXECUTION ---\n");
        let code = format!(
            "const BASE_URL = '{base_internal_url}';\nconst WM_TOKEN = '{}';\n{}",
            &client.get_token().await,
            inner_content
        );
        let (result, ts_logs) = do_nativets(job, logs.clone(), &client, code, db).await?;
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

    let result: error::Result<Box<RawValue>> = match language {
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
                mem_peak,
                canceled_by,
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
                mem_peak,
                canceled_by,
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
                mem_peak,
                canceled_by,
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
                mem_peak,
                canceled_by,
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
                mem_peak,
                canceled_by,
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
                mem_peak,
                canceled_by,
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
async fn handle_dependency_job<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    job: &QueuedJob,
    logs: &mut String,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    base_internal_url: &str,
    token: &str,
    rsmq: Option<R>,
) -> error::Result<Box<RawValue>> {
    let raw_code = match job.raw_code {
        Some(ref code) => code.to_owned(),
        None => sqlx::query_scalar!(
            "SELECT content FROM script WHERE hash = $1 AND workspace_id = $2",
            &job.script_hash.unwrap_or(ScriptHash(0)).0,
            &job.workspace_id
        )
        .fetch_optional(db)
        .await?
        .unwrap_or_else(|| "No script found at this hash".to_string()),
    };

    let script_path = job.script_path();
    let content = capture_dependency_job(
        &job.id,
        job.language.as_ref().map(|v| Ok(v)).unwrap_or_else(|| {
            Err(Error::InternalErr(
                "Job Language required for dependency jobs".to_owned(),
            ))
        })?,
        &raw_code,
        logs,
        mem_peak,
        canceled_by,
        job_dir,
        db,
        worker_name,
        &job.workspace_id,
        worker_dir,
        base_internal_url,
        token,
        script_path,
    )
    .await;
    match content {
        Ok(content) => {
            if job.script_hash.is_none() {
                // it a one-off raw script dependency job, no need to update the db
                return Ok(to_raw_value_owned(
                    json!({ "success": "Successful lock file generation", "lock": content }),
                ));
            }

            let hash = job.script_hash.unwrap_or(ScriptHash(0));
            let w_id = &job.workspace_id;
            sqlx::query!(
                "UPDATE script SET lock = $1 WHERE hash = $2 AND workspace_id = $3",
                &content,
                &hash.0,
                w_id
            )
            .execute(db)
            .await?;

            let (deployment_message, parent_path) =
                get_deployment_msg_and_parent_path_from_args(job.args.clone());

            if let Err(e) = handle_deployment_metadata(
                &job.email,
                &job.created_by,
                &db,
                &w_id,
                DeployedObject::Script {
                    hash,
                    path: script_path.to_string(),
                    parent_path: parent_path.clone(),
                },
                deployment_message.clone(),
                rsmq.clone(),
                false,
            )
            .await
            {
                tracing::error!(%e, "error handling deployment metadata");
            }

            if &job.language == &Some(ScriptLang::Python3) {
                if let Ok(relative_imports) = parse_relative_imports(&raw_code, script_path) {
                    logs.push_str("\n--- RELATIVE IMPORTS ---\n\n");
                    logs.push_str(&relative_imports.join("\n"));
                    if !relative_imports.is_empty() {
                        let mut tx = db.begin().await?;
                        sqlx::query!(
                            "DELETE FROM dependency_map
                             WHERE importer_path = $1 AND importer_kind = 'script'
                             AND workspace_id = $2",
                            script_path,
                            w_id
                        )
                        .execute(&mut *tx)
                        .await?;
                        for import in relative_imports {
                            sqlx::query!(
                                "INSERT INTO dependency_map (workspace_id, importer_path, importer_kind, imported_path)
                                 VALUES ($1, $2, 'script', $3)",
                                w_id,
                                script_path,
                                import
                            )
                            .execute(&mut *tx)
                            .await?;
                            logs.push_str(&format!("{}\n", import));
                        }
                        tx.commit().await?;
                    }
                    if let Err(e) = trigger_python_dependents_to_recompute_dependencies(
                        w_id,
                        script_path,
                        deployment_message,
                        parent_path,
                        &job.email,
                        &job.created_by,
                        &job.permissioned_as,
                        db,
                        rsmq,
                    )
                    .await
                    {
                        tracing::error!(%e, "error triggering python dependents to recompute dependencies");
                    }
                }
            }
            Ok(to_raw_value_owned(
                json!({ "success": "Successful lock file generation", "lock": content }),
            ))
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

async fn trigger_python_dependents_to_recompute_dependencies<
    R: rsmq_async::RsmqConnection + Send + Sync + Clone,
>(
    w_id: &str,
    script_path: &str,
    deployment_message: Option<String>,
    parent_path: Option<String>,
    email: &str,
    created_by: &str,
    permissioned_as: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    rsmq: Option<R>,
) -> error::Result<()> {
    let script_importers = sqlx::query_scalar!(
        "SELECT importer_path FROM dependency_map
         WHERE imported_path = $1 AND importer_kind = 'script'
         AND workspace_id = $2",
        script_path,
        w_id
    )
    .fetch_all(db)
    .await?;
    for s in script_importers.iter() {
        let tx: PushIsolationLevel<'_, R> =
            PushIsolationLevel::IsolatedRoot(db.clone(), rsmq.clone());
        let r = get_latest_deployed_hash_for_path(db, w_id, s.as_str()).await;
        if let Ok(r) = r {
            let mut args: HashMap<String, serde_json::Value> = HashMap::new();
            if let Some(ref dm) = deployment_message {
                args.insert("deployment_message".to_string(), json!(dm));
            }
            if let Some(ref p_path) = parent_path {
                args.insert("parent_path".to_string(), json!(p_path));
            }

            let (job_uuid, new_tx) = windmill_queue::push(
                db,
                tx,
                &w_id,
                JobPayload::Dependencies {
                    path: s.clone(),
                    hash: r.0,
                    language: r.5,
                    dedicated_worker: r.6,
                },
                args,
                &created_by,
                email,
                permissioned_as.to_string(),
                None,
                None,
                None,
                None,
                None,
                false,
                false,
                None,
                true,
                None,
                None,
                None,
                None,
            )
            .await?;
            tracing::info!(
                "pushed dependency job due to common python path: {job_uuid} for path {path} with hash {hash}",
                path = s,
                hash = r.0
            );
            new_tx.commit().await?;
        } else {
            tracing::error!(
                "error getting latest deployed hash for path {path}: {err}",
                path = s,
                err = r.unwrap_err()
            );
        }
    }
    Ok(())
}

async fn handle_flow_dependency_job<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    job: &QueuedJob,
    logs: &mut String,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    base_internal_url: &str,
    token: &str,
    rsmq: Option<R>,
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
    let mut flow = serde_json::from_str::<FlowValue>((*raw_flow.0).get()).map_err(to_anyhow)?;

    flow.modules = lock_modules(
        flow.modules,
        job,
        logs,
        mem_peak,
        canceled_by,
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

    let (deployment_message, parent_path) =
        get_deployment_msg_and_parent_path_from_args(job.args.clone());

    if let Err(e) = handle_deployment_metadata(
        &job.email,
        &job.created_by,
        &db,
        &job.workspace_id,
        DeployedObject::Flow { path: job_path, parent_path },
        deployment_message,
        rsmq.clone(),
        false,
    )
    .await
    {
        tracing::error!(%e, "error handling deployment metadata");
    }

    Ok(())
}

fn get_deployment_msg_and_parent_path_from_args(
    args: Option<Json<HashMap<String, Box<RawValue>>>>,
) -> (Option<String>, Option<String>) {
    let args_map = args.map(|json_hashmap| json_hashmap.0);
    let deployment_message = args_map
        .clone()
        .map(|hashmap| {
            hashmap
                .get("deployment_message")
                .map(|map_value| serde_json::from_str::<String>(map_value.get()).ok())
                .flatten()
        })
        .flatten();
    let parent_path = args_map
        .clone()
        .map(|hashmap| {
            hashmap
                .get("parent_path")
                .map(|map_value| serde_json::from_str::<String>(map_value.get()).ok())
                .flatten()
        })
        .flatten();
    (deployment_message, parent_path)
}

#[async_recursion]
async fn lock_modules(
    modules: Vec<FlowModule>,
    job: &QueuedJob,
    logs: &mut String,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
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
                            mem_peak,
                            canceled_by,
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
                            mem_peak,
                            canceled_by,
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
                            mem_peak,
                            canceled_by,
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
                        mem_peak,
                        canceled_by,
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

        let new_lock = capture_dependency_job(
            &job.id,
            &language,
            &content,
            logs,
            mem_peak,
            canceled_by,
            job_dir,
            db,
            worker_name,
            &job.workspace_id,
            worker_dir,
            base_internal_url,
            token,
            &path.clone().unwrap_or_else(|| job_path.to_string()),
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
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
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
                            logs.push_str("Found lockable inline script. Generating lock...\n");
                            let new_lock = capture_dependency_job(
                                &job.id,
                                &language,
                                &content,
                                logs,
                                mem_peak,
                                canceled_by,
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
                        mem_peak,
                        canceled_by,
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
                        mem_peak,
                        canceled_by,
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

async fn handle_app_dependency_job<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    job: &QueuedJob,
    logs: &mut String,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    worker_dir: &str,
    base_internal_url: &str,
    token: &str,
    rsmq: Option<R>,
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
            mem_peak,
            canceled_by,
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

        let (deployment_message, parent_path) =
            get_deployment_msg_and_parent_path_from_args(job.args.clone());

        if let Err(e) = handle_deployment_metadata(
            &job.email,
            &job.created_by,
            &db,
            &job.workspace_id,
            DeployedObject::App { path: job_path, version: id, parent_path },
            deployment_message,
            rsmq.clone(),
            false,
        )
        .await
        {
            tracing::error!(%e, "error handling deployment metadata");
        }

        // tx = PushIsolationLevel::Transaction(new_tx);
        // tx = handle_deployment_metadata(
        //     tx,
        //     &authed,
        //     &db,
        //     &w_id,
        //     DeployedObject::App { path: app.path.clone(), version: v_id },
        //     app.deployment_message,
        // )
        // .await?;

        // match tx {
        //     PushIsolationLevel::Transaction(tx) => tx.commit().await?,
        //     _ => {
        //         return Err(Error::InternalErr(
        //             "Expected a transaction here".to_string(),
        //         ));
        //     }
        // }

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
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
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
            let reqs = windmill_parser_py_imports::parse_python_imports(
                job_raw_code,
                &w_id,
                script_path,
                &db,
            )
            .await?
            .join("\n");
            create_dependencies_dir(job_dir).await;
            let req: std::result::Result<String, Error> = pip_compile(
                job_id,
                &reqs,
                logs,
                mem_peak,
                canceled_by,
                job_dir,
                db,
                worker_name,
                w_id,
            )
            .await;
            // install the dependencies to pre-fill the cache
            if let Ok(req) = req.as_ref() {
                let r = handle_python_reqs(
                    req.split("\n").filter(|x| !x.starts_with("--")).collect(),
                    job_id,
                    w_id,
                    logs,
                    mem_peak,
                    canceled_by,
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
                mem_peak,
                canceled_by,
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
                mem_peak,
                canceled_by,
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
                mem_peak,
                canceled_by,
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
        ScriptLang::Mssql => Ok("".to_owned()),
        ScriptLang::Graphql => Ok("".to_owned()),
        ScriptLang::Bash => Ok("".to_owned()),
        ScriptLang::Powershell => Ok("".to_owned()),
        ScriptLang::Nativets => Ok("".to_owned()),
    }
}
