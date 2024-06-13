/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use windmill_common::{
    auth::{fetch_authed_from_permissioned_as, JWTAuthClaims, JobPerms, JWT_SECRET},
    db::Authed,
    worker::{get_windmill_memory_usage, get_worker_memory_usage, TMP_DIR},
};

use anyhow::Result;
use const_format::concatcp;
#[cfg(feature = "prometheus")]
use prometheus::{
    core::{AtomicI64, GenericGauge},
    IntCounter,
};
use tracing::Instrument;
#[cfg(feature = "prometheus")]
use windmill_common::METRICS_DEBUG_ENABLED;
#[cfg(feature = "prometheus")]
use windmill_common::METRICS_ENABLED;

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

use uuid::Uuid;

use windmill_common::{
    error::{self, to_anyhow, Error},
    get_latest_deployed_hash_for_path,
    jobs::{JobKind, QueuedJob},
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang, PREVIEW_IS_CODEBASE_HASH},
    users::SUPERADMIN_SECRET_EMAIL,
    utils::StripPath,
    worker::{to_raw_value, update_ping, CLOUD_HOSTED, NO_LOGS, WORKER_CONFIG, WORKER_GROUP},
    DB, IS_READY,
};

use windmill_queue::{
    append_logs, canceled_job_to_result, empty_result, get_queued_job, pull, push, CanceledBy,
    PushArgs, PushIsolationLevel, WrappedError, HTTP_CLIENT,
};

#[cfg(feature = "prometheus")]
use windmill_queue::register_metric;

use serde_json::{json, value::RawValue};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use tokio::fs::symlink;

#[cfg(target_os = "windows")]
use tokio::fs::symlink_file as symlink;

use tokio::{
    fs::DirBuilder,
    sync::{
        mpsc::{self, Sender},
        RwLock,
    },
    task::JoinHandle,
    time::Instant,
};

use futures::future::FutureExt;

use rand::Rng;

use windmill_queue::{add_completed_job, add_completed_job_error};

use crate::{
    bash_executor::{handle_bash_job, handle_powershell_job, ANSI_ESCAPE_RE},
    bun_executor::handle_bun_job,
    common::{
        build_args_map, get_cached_resource_value_if_valid, get_reserved_variables, hash_args,
        read_result, save_in_cache, write_file, NO_LOGS_AT_ALL, SLOW_LOGS,
    },
    deno_executor::handle_deno_job,
    go_executor::handle_go_job,
    graphql_executor::do_graphql,
    js_eval::{eval_fetch_timeout, transpile_ts},
    mysql_executor::do_mysql,
    pg_executor::do_postgresql,
    php_executor::handle_php_job,
    python_executor::handle_python_job,
    worker_flow::{
        handle_flow, update_flow_status_after_job_completion, update_flow_status_in_progress,
    },
    worker_lockfiles::{
        handle_app_dependency_job, handle_dependency_job, handle_flow_dependency_job,
    },
};

#[cfg(feature = "enterprise")]
use crate::dedicated_worker::create_dedicated_worker_map;

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

        let label = if job.permissioned_as != format!("u/{}", job.created_by)
            && job.permissioned_as != job.created_by
        {
            format!("ephemeral-script-end-user-{}", job.created_by)
        } else {
            "ephemeral-script".to_string()
        };
        tokio::spawn(async move {
            let token = create_token_for_owner(
                &db.clone(),
                &w_id,
                &owner,
                &label,
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

    let jwt_secret = JWT_SECRET.read().await;

    if jwt_secret.is_empty() {
        return Err(Error::InternalErr("No JWT secret found".to_string()));
    }

    let job_authed = match sqlx::query_as!(
        JobPerms,
        "SELECT * FROM job_perms WHERE job_id = $1 AND workspace_id = $2",
        job_id,
        w_id
    )
    .fetch_optional(db)
    .await
    {
        Ok(Some(jp)) => jp.into(),
        _ => {
            tracing::warn!("Could not get permissions for job {job_id} from job_perms table, getting permissions directly...");
            fetch_authed_from_permissioned_as(owner.to_string(), email.to_string(), w_id, db)
                .await
                .map_err(|e| {
                    Error::InternalErr(format!(
                        "Could not get permissions directly for job {job_id}: {e:#}"
                    ))
                })?
        }
    };

    let payload = JWTAuthClaims {
        email: job_authed.email,
        username: job_authed.username,
        is_admin: job_authed.is_admin,
        is_operator: job_authed.is_operator,
        groups: job_authed.groups,
        folders: job_authed.folders,
        label: Some(label.to_string()),
        workspace_id: w_id.to_string(),
        exp: (chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64)).timestamp()
            as usize,
    };

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &payload,
        &jsonwebtoken::EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|err| {
        Error::InternalErr(format!(
            "Could not encode JWT token for job {job_id}: {:?}",
            err
        ))
    })?;
    Ok(format!("jwt:{}", token))
}

pub const TMP_LOGS_DIR: &str = concatcp!(TMP_DIR, "/logs");

pub const ROOT_CACHE_DIR: &str = concatcp!(TMP_DIR, "/cache/");
pub const LOCK_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "lock");
pub const PIP_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "pip");
pub const TAR_PIP_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "tar/pip");
pub const DENO_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "deno");
pub const DENO_CACHE_DIR_DEPS: &str = concatcp!(ROOT_CACHE_DIR, "deno/deps");
pub const DENO_CACHE_DIR_NPM: &str = concatcp!(ROOT_CACHE_DIR, "deno/npm");

pub const GO_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "go");
pub const BUN_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "bun");

pub const HUB_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "hub");
pub const GO_BIN_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "gobin");
pub const POWERSHELL_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "powershell");
pub const COMPOSER_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "composer");

const NUM_SECS_PING: u64 = 5;

const INCLUDE_DEPS_PY_SH_CONTENT: &str = include_str!("../nsjail/download_deps.py.sh");

pub const DEFAULT_CLOUD_TIMEOUT: u64 = 900;
pub const DEFAULT_SELFHOSTED_TIMEOUT: u64 = 604800; // 7 days
pub const DEFAULT_SLEEP_QUEUE: u64 = 50;

// only 1 native job so that we don't have to worry about concurrency issues on non dedicated native jobs workers
pub const DEFAULT_NATIVE_JOBS: usize = 1;

const VACUUM_PERIOD: u32 = 50000;

const DROP_CACHE_PERIOD: u32 = 1000;

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

lazy_static::lazy_static! {

    pub static ref JOB_TOKEN: Option<String> = std::env::var("JOB_TOKEN").ok();

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
    pub static ref NPM_PATH: String = std::env::var("NPM_PATH").unwrap_or_else(|_| "/usr/bin/npm".to_string());
    pub static ref NODE_PATH: String = std::env::var("NODE_PATH").unwrap_or_else(|_| "/usr/bin/node".to_string());
    pub static ref POWERSHELL_PATH: String = std::env::var("POWERSHELL_PATH").unwrap_or_else(|_| "/usr/bin/pwsh".to_string());
    pub static ref PHP_PATH: String = std::env::var("PHP_PATH").unwrap_or_else(|_| "/usr/bin/php".to_string());
    pub static ref COMPOSER_PATH: String = std::env::var("COMPOSER_PATH").unwrap_or_else(|_| "/usr/bin/composer".to_string());
    pub static ref NSJAIL_PATH: String = std::env::var("NSJAIL_PATH").unwrap_or_else(|_| "nsjail".to_string());
    pub static ref PATH_ENV: String = std::env::var("PATH").unwrap_or_else(|_| String::new());
    pub static ref HOME_ENV: String = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    pub static ref TZ_ENV: String = std::env::var("TZ").unwrap_or_else(|_| String::new());
    pub static ref GOPRIVATE: Option<String> = std::env::var("GOPRIVATE").ok();
    pub static ref GOPROXY: Option<String> = std::env::var("GOPROXY").ok();
    pub static ref NETRC: Option<String> = std::env::var("NETRC").ok();


    pub static ref NPM_CONFIG_REGISTRY: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref BUNFIG_INSTALL_SCOPES: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));

    pub static ref PIP_EXTRA_INDEX_URL: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref PIP_INDEX_URL: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref JOB_DEFAULT_TIMEOUT: Arc<RwLock<Option<i32>>> = Arc::new(RwLock::new(None));


    static ref MAX_TIMEOUT: u64 = std::env::var("TIMEOUT")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or_else(|| if *CLOUD_HOSTED { DEFAULT_CLOUD_TIMEOUT } else { DEFAULT_SELFHOSTED_TIMEOUT });

    pub static ref MAX_WAIT_FOR_SIGINT: u64 = std::env::var("MAX_WAIT_FOR_SIGINT")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or_else(|| 0);

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


    pub static ref EXIT_AFTER_NO_JOB_FOR_SECS: Option<u64> = std::env::var("EXIT_AFTER_NO_JOB_FOR_SECS")
        .ok()
        .and_then(|x| x.parse::<u64>().ok());


}
//only matter if CLOUD_HOSTED
pub const MAX_RESULT_SIZE: usize = 1024 * 1024 * 2; // 2MB

pub const INIT_SCRIPT_TAG: &str = "init_script";

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

    pub async fn get_id_token(&self, audience: &str) -> anyhow::Result<String> {
        let url = format!(
            "{}/api/w/{}/oidc/token/{}",
            self.base_internal_url, self.workspace, audience
        );
        let response = self.get(&url, vec![]).await?;
        match response.status().as_u16() {
            200u16 => Ok(response.json::<String>().await?),
            _ => Err(anyhow::anyhow!(response.text().await.unwrap_or_default())),
        }
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

#[cfg(feature = "prometheus")]
type Histo = Arc<prometheus::Histogram>;
#[cfg(feature = "prometheus")]
type GGauge = Arc<GenericGauge<AtomicI64>>;

#[cfg(not(feature = "prometheus"))]
type Histo = ();
#[cfg(not(feature = "prometheus"))]
type GGauge = ();

async fn handle_receive_completed_job<
    R: rsmq_async::RsmqConnection + Send + Sync + Clone + 'static,
>(
    jc: JobCompleted,
    base_internal_url: String,
    db: Pool<Postgres>,
    worker_dir: String,
    same_worker_tx: Sender<SameWorkerPayload>,
    rsmq: Option<R>,
    worker_name: &str,
    worker_save_completed_job_duration: Option<Histo>,
    worker_flow_transition_duration: Option<Histo>,
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

#[allow(dead_code)]
#[derive(Clone)]
pub struct JobCompletedSender(Sender<SendResult>, Option<GGauge>, Option<Histo>);

pub struct SameWorkerPayload {
    pub job_id: Uuid,
    pub recoverable: bool,
}

impl JobCompletedSender {
    pub async fn send(
        &self,
        jc: JobCompleted,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<SendResult>> {
        #[cfg(feature = "prometheus")]
        if let Some(wj) = self.1.as_ref() {
            wj.inc()
        }
        #[cfg(feature = "prometheus")]
        let timer = self.2.as_ref().map(|x| x.start_timer());
        let r = self.0.send(SendResult::JobCompleted(jc)).await;
        #[cfg(feature = "prometheus")]
        timer.map(|x| x.stop_and_record());
        r
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
                tracing::warn!("Failed to write to /proc/sys/vm/drop_caches (expected to not work in not in privileged mode, only required to forcefully drop the cache to avoid spurrious oom killer): {}", e);
            }
        }
        Err(e) => {
            tracing::warn!("Failed to open /proc/sys/vm/drop_caches (expected to not work in not in privileged mode, only required to forcefully drop the cache to avoid spurrious oom killer):: {}", e);
        }
    }
}

const OUTSTANDING_WAIT_TIME_THRESHOLD_MS: i64 = 1000;

async fn insert_wait_time(
    job_id: Uuid,
    root_job_id: Option<Uuid>,
    db: &Pool<Postgres>,
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
    queued_job: &QueuedJob,
    db: &Pool<Postgres>,
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
    let root_job_id = queued_job.root_job;
    let db = db.clone();

    tokio::spawn(async move {
            match insert_wait_time(job_id, root_job_id, &db, wait_time).await {
                Ok(()) => tracing::warn!("This job waited for an executor for a significant amount of time. Recording value wait_time={}ms", wait_time),
                Err(e) => tracing::error!("Failed to insert outstanding wait time: {}", e),
            }
    }.in_current_span());
}

#[tracing::instrument(name = "worker", level = "info", skip_all, fields(worker = %worker_name, hostname = %hostname))]
pub async fn run_worker<R: rsmq_async::RsmqConnection + Send + Sync + Clone + 'static>(
    db: &Pool<Postgres>,
    hostname: &str,
    worker_name: String,
    i_worker: u64,
    _num_workers: u32,
    ip: &str,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
    killpill_tx: tokio::sync::broadcast::Sender<()>,
    base_internal_url: &str,
    rsmq: Option<R>,
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
    tracing::debug!(worker_dir = %worker_dir, "Creating worker dir");

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

    update_ping(hostname, &worker_name, ip, db).await;

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

    let worker_job_completed_channel_queue = {
        #[cfg(feature = "prometheus")]
        if METRICS_DEBUG_ENABLED.load(Ordering::Relaxed) && METRICS_ENABLED.load(Ordering::Relaxed)
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
        }

        #[cfg(not(feature = "prometheus"))]
        None
    };

    let worker_completed_channel_queue_send_duration = {
        #[cfg(feature = "prometheus")]
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
        }
        #[cfg(not(feature = "prometheus"))]
        None
    };

    #[cfg(feature = "prometheus")]
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

    let worker_code_execution_duration = {
        #[cfg(feature = "prometheus")]
        if METRICS_DEBUG_ENABLED.load(Ordering::Relaxed) && METRICS_ENABLED.load(Ordering::Relaxed)
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
        }

        #[cfg(not(feature = "prometheus"))]
        None
    };

    let worker_flow_initial_transition_duration = {
        #[cfg(feature = "prometheus")]
        if METRICS_DEBUG_ENABLED.load(Ordering::Relaxed) && METRICS_ENABLED.load(Ordering::Relaxed)
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
        }

        #[cfg(not(feature = "prometheus"))]
        None
    };

    #[cfg(feature = "prometheus")]
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

    let mut worker_code_execution_metric: f32 = 0.0;

    let mut jobs_executed = 0;

    #[cfg(feature = "prometheus")]
    if let Some(ws) = WORKER_STARTED.as_ref() {
        ws.inc();
    }

    let (same_worker_tx, mut same_worker_rx) = mpsc::channel::<SameWorkerPayload>(5);

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

    #[cfg(feature = "prometheus")]
    let worker_job_completed_channel_queue2 = worker_job_completed_channel_queue.clone();
    #[cfg(feature = "prometheus")]
    let worker_save_completed_job_duration2 = worker_save_completed_job_duration.clone();
    #[cfg(feature = "prometheus")]
    let worker_flow_transition_duration2 = worker_flow_transition_duration.clone();

    #[cfg(not(feature = "prometheus"))]
    let worker_save_completed_job_duration2 = None;
    #[cfg(not(feature = "prometheus"))]
    let worker_flow_transition_duration2 = None;

    let worker_name2 = worker_name.clone();
    let killpill_tx2 = killpill_tx.clone();
    let job_completed_sender = job_completed_tx.0.clone();
    let send_result = tokio::spawn((async move {
        while let Some(sr) = job_completed_rx.recv().await {
            match sr {
                SendResult::JobCompleted(jc) => {
                    #[cfg(feature = "prometheus")]
                    if let Some(wj) = worker_job_completed_channel_queue2.as_ref() {
                        wj.dec();
                    }
                    let base_internal_url2 = base_internal_url2.clone();
                    let worker_dir2 = worker_dir2.clone();
                    let db2 = db2.clone();
                    let same_worker_tx2 = same_worker_tx2.clone();
                    let rsmq2 = rsmq2.clone();
                    let worker_name = worker_name2.clone();
                    if matches!(jc.job.job_kind, JobKind::Noop) {
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

                        #[cfg(feature = "prometheus")]
                        let worker_save_completed_job_duration2 =
                            worker_save_completed_job_duration.clone();
                        #[cfg(feature = "prometheus")]
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
                        let is_init_script_and_failure =
                            !jc.success && jc.job.tag.as_str() == INIT_SCRIPT_TAG;
                        let is_dependency_job = matches!(
                            jc.job.job_kind,
                            JobKind::Dependencies | JobKind::FlowDependencies);
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
                        if is_init_script_and_failure {
                            tracing::error!("init script errored, exiting");
                            killpill_tx2.send(()).unwrap_or_default();
                        }
                        if is_dependency_job && is_dedicated_worker {
                            tracing::error!("Dedicated worker executed a dependency job, a new script has been deployed. Exiting expecting to be restarted.");
                            sqlx::query!("UPDATE config SET config = config WHERE name = $1", format!("worker__{}", *WORKER_GROUP))
                        .execute(&db2)
                        .await
                        .expect("update config to trigger restart of all dedicated workers at that config");
                            killpill_tx2.send(()).unwrap_or_default();

                        }
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
                    tracing::info!(parent_flow = %flow, "updating flow status");
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
                        tracing::error!("Error updating flow status after job completion for {flow} on {worker_name2}: {e:#}");
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
    }).instrument(tracing::Span::current()));

    let mut last_executed_job: Option<Instant> = None;
    let mut last_checked_suspended = Instant::now();

    #[cfg(feature = "benchmark")]
    let mut started = false;

    #[cfg(feature = "benchmark")]
    let mut infos = BenchmarkInfo { iters: 0, timings: vec![] };

    let vacuum_shift = rand::thread_rng().gen_range(0..VACUUM_PERIOD);

    IS_READY.store(true, Ordering::Relaxed);
    tracing::info!(
        "listening for jobs, WORKER_GROUP: {}, config: {:?}",
        *WORKER_GROUP,
        WORKER_CONFIG.read().await
    );

    // (dedi_path, dedicated_worker_tx, dedicated_worker_handle)
    // Option<Sender<Arc<QueuedJob>>>,
    // Option<JoinHandle<()>>,

    #[cfg(feature = "enterprise")]
    let (dedicated_workers, is_flow_worker, dedicated_handles): (
        HashMap<String, Sender<Arc<QueuedJob>>>,
        bool,
        Vec<JoinHandle<()>>,
    ) = create_dedicated_worker_map(
        &killpill_tx,
        &killpill_rx,
        db,
        &worker_dir,
        base_internal_url,
        &worker_name,
        &job_completed_tx,
    )
    .await;

    #[cfg(not(feature = "enterprise"))]
    let (dedicated_workers, is_flow_worker, dedicated_handles): (
        HashMap<String, Sender<Arc<QueuedJob>>>,
        bool,
        Vec<JoinHandle<()>>,
    ) = (HashMap::new(), false, vec![]);

    #[cfg(feature = "benchmark")]
    tracing::info!("pre loop time {}s", start.elapsed().as_secs_f64());

    if i_worker == 1 {
        if let Err(e) =
            queue_init_bash_maybe(db, same_worker_tx.clone(), &worker_name, rsmq.clone()).await
        {
            killpill_tx.send(()).unwrap_or_default();
            tracing::error!("Error queuing init bash script for worker {worker_name}: {e:#}");
            return;
        }
    }

    #[cfg(feature = "prometheus")]
    let worker_dedicated_channel_queue_send_duration = {
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

    loop {
        #[cfg(feature = "benchmark")]
        let loop_start = Instant::now();

        #[cfg(feature = "benchmark")]
        let mut timing = vec![];

        #[cfg(feature = "prometheus")]
        if let Some(wk) = worker_busy.as_ref() {
            wk.set(0);
            tracing::debug!("set worker busy to 0");
        }

        #[cfg(feature = "prometheus")]
        if let Some(ref um) = uptime_metric {
            um.inc_by(
                ((start_time.elapsed().as_millis() as f64) / 1000.0 - um.get())
                    .try_into()
                    .unwrap(),
            );
            tracing::debug!("set uptime metric");
        }

        if last_ping.elapsed().as_secs() > NUM_SECS_PING {
            let tags = WORKER_CONFIG.read().await.worker_tags.clone();

            let memory_usage = get_worker_memory_usage();
            let wm_memory_usage = get_windmill_memory_usage();

            if let Err(e) = sqlx::query!(
                "UPDATE worker_ping SET ping_at = now(), jobs_executed = $1, custom_tags = $2, occupancy_rate = $3, memory_usage = $4, wm_memory_usage = $5, current_job_id = NULL, current_job_workspace_id = NULL WHERE worker = $6",
                jobs_executed,
                tags.as_slice(),
                worker_code_execution_metric / start_time.elapsed().as_secs_f32(),
                memory_usage,
                wm_memory_usage,
                &worker_name
            ).execute(db).await {
                tracing::error!("failed to update worker ping, exiting: {}", e);
                killpill_tx.send(()).unwrap_or_default();
            }
            tracing::info!(
                "ping update, memory: container={}MB, windmill={}MB",
                memory_usage.unwrap_or_default() / (1024 * 1024),
                wm_memory_usage.unwrap_or_default() / (1024 * 1024)
            );

            last_ping = Instant::now();
        }

        if (jobs_executed as u32 + vacuum_shift) % VACUUM_PERIOD == 0 {
            let db2 = db.clone();
            let current_span = tracing::Span::current();
            tokio::task::spawn(
                (async move {
                    tracing::info!("vacuuming queue and completed_job");
                    if let Err(e) = sqlx::query!("VACUUM (skip_locked) queue")
                        .execute(&db2)
                        .await
                    {
                        tracing::error!("failed to vacuum queue: {}", e);
                    }
                    tracing::info!("vacuumed queue and completed_job");
                })
                .instrument(current_span),
            );
            jobs_executed += 1;
        }

        #[cfg(any(target_os = "linux"))]
        if (jobs_executed as u32 + 1) % DROP_CACHE_PERIOD == 0 {
            drop_cache().await;
            jobs_executed += 1;
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
                Some(same_worker_job) = same_worker_rx.recv() => {
                    tracing::debug!("received {} from same worker channel", same_worker_job.job_id);
                    let r = sqlx::query_as::<_, QueuedJob>("SELECT * FROM queue WHERE id = $1")
                        .bind(same_worker_job.job_id)
                        .fetch_optional(db)
                        .await
                        .map_err(|_| Error::InternalErr("Impossible to fetch same_worker job".to_string()));
                    if r.is_err() && !same_worker_job.recoverable {
                        tracing::error!("failed to fetch same_worker job on a non recoverable job, exiting");
                        job_completed_tx.0.send(SendResult::Kill).await.unwrap();
                        break;
                    } else {
                        r
                    }
                },
                (job, timer, suspend_first) = async {
                    let pull_time = Instant::now();
                    let suspend_first = if suspend_first_success || last_checked_suspended.elapsed().as_secs() > 3 {
                        last_checked_suspended = Instant::now();
                        true
                    } else { false };
                    pull(&db, rsmq.clone(), suspend_first).map(|x| (x, pull_time, suspend_first)).await
                } => {
                    add_time!(timing, loop_start, "post pull");
                    // tracing::debug!("pulled job: {:?}", job.as_ref().ok().and_then(|x| x.as_ref().map(|y| y.id)));
                    let duration_pull_s = timer.elapsed().as_secs_f64();
                    let err_pull = job.is_ok();
                    let empty = job.as_ref().is_ok_and(|x| x.is_none());
                    suspend_first_success = suspend_first && !empty;
                    if !agent_mode && duration_pull_s > 0.5 {
                        tracing::warn!("pull took more than 0.5s ({duration_pull_s}), this is a sign that the database is VERY undersized for this load. empty: {empty}, err: {err_pull}");
                        #[cfg(feature = "prometheus")]
                        if empty {
                            if let Some(wp) = worker_pull_over_500_counter_empty.as_ref() {
                                wp.inc();
                            }
                        } else if let Some(wp) = worker_pull_over_500_counter.as_ref() {
                            wp.inc();
                        }

                    } else if !agent_mode && duration_pull_s > 0.1 {
                        tracing::warn!("pull took more than 0.1s ({duration_pull_s}) this is a sign that the database is undersized for this load. empty: {empty}, err: {err_pull}");
                        #[cfg(feature = "prometheus")]
                        if empty {
                            if let Some(wp) = worker_pull_over_100_counter_empty.as_ref() {
                                wp.inc();
                            }
                        } else if let Some(wp) = worker_pull_over_100_counter.as_ref() {
                            wp.inc();
                        }
                    }

                    #[cfg(feature = "prometheus")]
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

        #[cfg(feature = "prometheus")]
        if let Some(wb) = worker_busy.as_ref() {
            wb.set(1);
            tracing::debug!("set worker busy to 1");
        }

        match next_job {
            Ok(Some(job)) => {
                last_executed_job = None;
                jobs_executed += 1;

                tracing::debug!("started handling of job {}", job.id);

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

                                #[cfg(feature = "prometheus")]
                                let timer = worker_dedicated_channel_queue_send_duration
                                    .as_ref()
                                    .map(|x| x.start_timer());

                                if let Err(e) = dedicated_worker_tx.send(Arc::new(job)).await {
                                    tracing::info!("failed to send jobs to dedicated workers. Likely dedicated worker has been shut down. This is normal: {e:?}");
                                }

                                #[cfg(feature = "prometheus")]
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
                    add_outstanding_wait_time(&job, db, OUTSTANDING_WAIT_TIME_THRESHOLD_MS);

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
                        .root_job
                        .map(|x| x.to_string())
                        .unwrap_or_else(|| "none".to_string());

                    if job.id == Uuid::nil() {
                        tracing::info!("running warmup job");
                    } else {
                        tracing::info!(workspace_id = %job.workspace_id, job_id = %job.id, root_id = %job_root, "fetched job {}, root job: {}", job.id, job_root);
                    } // Here we can't remove the job id, but maybe with the
                      // fields macro we can make a job id that only appears when
                      // the job is defined?

                    let job_dir = format!("{worker_dir}/{}", job.id);

                    DirBuilder::new()
                        .recursive(true)
                        .create(&job_dir)
                        .await
                        .expect("could not create job dir");

                    let same_worker = job.same_worker;

                    let target = &format!("{job_dir}/shared");

                    if same_worker && job.parent_job.is_some() {
                        if tokio::fs::metadata(target).await.is_err() {
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
                        }
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

                    #[cfg(feature = "prometheus")]
                    let tag = job.tag.clone();

                    let arc_job = Arc::new(job);
                    if let Err(err) = handle_queued_job(
                        arc_job.clone(),
                        db,
                        &authed_client,
                        &hostname,
                        &worker_name,
                        &worker_dir,
                        &job_dir,
                        same_worker_tx.clone(),
                        base_internal_url,
                        rsmq.clone(),
                        job_completed_tx.clone(),
                        &mut worker_code_execution_metric,
                        worker_flow_initial_transition_duration.clone(),
                        worker_code_execution_duration.clone(),
                    )
                    .await
                    {
                        let is_init_script = arc_job.tag.as_str() == INIT_SCRIPT_TAG;
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
                        if is_init_script {
                            tracing::error!("failed to execute init_script, exiting");
                            killpill_tx.send(()).unwrap();
                            break;
                        }
                    };

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
                            tracing::info!("no job for {} seconds, exiting", secs);
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

                #[cfg(feature = "benchmark")]
                tracing::info!("no job found");

                tokio::time::sleep(Duration::from_millis(*SLEEP_QUEUE)).await;

                #[cfg(feature = "prometheus")]
                _timer.map(|timer| {
                    let duration = timer.elapsed().as_secs_f64();
                    if let Some(ws) = worker_sleep_duration_counter.as_ref() {
                        ws.inc_by(duration);
                    }
                });
            }
            Err(err) => {
                tracing::error!("Failed to pull jobs: {}", err);
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
    tracing::info!("worker {} exited", worker_name);
}

async fn queue_init_bash_maybe<'c, R: rsmq_async::RsmqConnection + Send + 'c>(
    db: &Pool<Postgres>,
    same_worker_tx: Sender<SameWorkerPayload>,
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
                hash: None,
                content: content.clone(),
                path: Some(format!("init_script_{worker_name}")),
                language: ScriptLang::Bash,
                lock: None,
                custom_concurrency_key: None,
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
            Some("init_script".to_string()),
            None,
            None,
            None,
            None,
        )
        .await?;
        inner_tx.commit().await?;
        same_worker_tx
            .send(SameWorkerPayload { job_id: uuid, recoverable: false })
            .await
            .map_err(to_anyhow)?;
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

#[tracing::instrument(name = "completed_job", level = "info", skip_all, fields(job_id = %job.id))]
pub async fn process_completed_job<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    JobCompleted { job, result, mem_peak, success, cached_res_path, canceled_by, .. }: JobCompleted,
    client: &AuthedClient,
    db: &DB,
    worker_dir: &str,
    same_worker_tx: Sender<SameWorkerPayload>,
    rsmq: Option<R>,
    worker_name: &str,
    _worker_save_completed_job_duration: Option<Histo>,
    _worker_flow_transition_duration: Option<Histo>,
    job_completed_tx: Sender<SendResult>,
) -> windmill_common::error::Result<()> {
    if success {
        // println!("bef completed job{:?}",  SystemTime::now());
        if let Some(cached_path) = cached_res_path {
            save_in_cache(db, client, &job, cached_path.to_string(), &result).await;
        }

        #[cfg(feature = "prometheus")]
        let timer = _worker_save_completed_job_duration
            .as_ref()
            .map(|x| x.start_timer());
        add_completed_job(
            db,
            &job,
            true,
            false,
            Json(&result),
            mem_peak.to_owned(),
            canceled_by,
            rsmq.clone(),
            false,
        )
        .await?;

        #[cfg(feature = "prometheus")]
        timer.map(|x| x.stop_and_record());

        if job.is_flow_step {
            if let Some(parent_job) = job.parent_job {
                #[cfg(feature = "prometheus")]
                let timer = _worker_flow_transition_duration
                    .as_ref()
                    .map(|x| x.start_timer());
                tracing::info!(parent_flow = %parent_job, subflow = %job.id, "updating flow status (2)");
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
                #[cfg(feature = "prometheus")]
                timer.map(|x| x.stop_and_record());
            }
        }
    } else {
        let result = add_completed_job_error(
            db,
            &job,
            mem_peak.to_owned(),
            canceled_by,
            serde_json::from_str(result.get()).unwrap_or_else(
                |_| json!({ "message": format!("Non serializable error: {}", result.get()) }),
            ),
            rsmq.clone(),
            worker_name,
            false,
        )
        .await?;
        if job.is_flow_step {
            if let Some(parent_job) = job.parent_job {
                tracing::error!(parent_flow = %parent_job, subflow = %job.id, "process completed job error, updating flow status");
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

#[tracing::instrument(name = "job_error", level = "info", skip_all, fields(job_id = %job.id))]
pub async fn handle_job_error<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    db: &Pool<Postgres>,
    client: &AuthedClient,
    job: &QueuedJob,
    mem_peak: i32,
    canceled_by: Option<CanceledBy>,
    err: Error,
    unrecoverable: bool,
    same_worker_tx: Sender<SameWorkerPayload>,
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
    let update_job_future = || async {
        append_logs(
            &job.id,
            &job.workspace_id,
            format!("Unexpected error during job execution:\n{err:#?}"),
            db,
        )
        .await;
        add_completed_job_error(
            db,
            job,
            mem_peak,
            canceled_by.clone(),
            err.clone(),
            rsmq_2,
            worker_name,
            false,
        )
        .await
    };

    let update_job_future = if job.is_flow_step || job.is_flow() {
        let (flow, job_status_to_update) = if let Some(parent_job_id) = job.parent_job {
            if let Err(e) = update_job_future().await {
                tracing::error!(
                    "error updating job future for job {} for handle_job_error: {e:#}",
                    job.id
                );
            }
            (parent_job_id, job.id)
        } else {
            (job.id, Uuid::nil())
        };

        let wrapped_error = WrappedError { error: err.clone() };
        tracing::error!(parent_flow = %flow, subflow = %job_status_to_update, "handle job error, updating flow status: {err:?}");
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
                if let Ok(Some(parent_job)) =
                    get_queued_job(&parent_job_id, &job.workspace_id, &db).await
                {
                    let e = json!({"message": err.to_string(), "name": "InternalErr"});
                    append_logs(
                        &parent_job.id,
                        &job.workspace_id,
                        format!("Unexpected error during flow job error handling:\n{err}"),
                        db,
                    )
                    .await;
                    let _ = add_completed_job_error(
                        db,
                        &parent_job,
                        mem_peak,
                        canceled_by.clone(),
                        e,
                        rsmq,
                        worker_name,
                        false,
                    )
                    .await;
                }
            }
        }

        None
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
    pub mem_peak: i32,
    pub success: bool,
    pub cached_res_path: Option<String>,
    pub token: String,
    pub canceled_by: Option<CanceledBy>,
}

async fn do_nativets(
    job: &QueuedJob,
    client: &AuthedClientBackgroundTask,
    env_code: String,
    code: String,
    db: &Pool<Postgres>,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
) -> windmill_common::error::Result<(Box<RawValue>, String)> {
    let args = build_args_map(job, client, db).await?.map(Json);
    let job_args = if args.is_some() {
        args.as_ref()
    } else {
        job.args.as_ref()
    };

    let result = eval_fetch_timeout(
        env_code,
        code.clone(),
        transpile_ts(code)?,
        job_args,
        job.id,
        job.timeout,
        db,
        mem_peak,
        canceled_by,
        worker_name,
        &job.workspace_id,
    )
    .await?;
    Ok((result.0, result.1))
}

#[derive(Deserialize, Serialize, Default)]
pub struct PreviousResult<'a> {
    #[serde(borrow)]
    pub previous_result: Option<&'a RawValue>,
}

#[tracing::instrument(name = "job", level = "info", skip_all, fields(job_id = %job.id))]
async fn handle_queued_job<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    job: Arc<QueuedJob>,
    db: &DB,
    client: &AuthedClientBackgroundTask,
    hostname: &str,
    worker_name: &str,
    worker_dir: &str,
    job_dir: &str,
    same_worker_tx: Sender<SameWorkerPayload>,
    base_internal_url: &str,
    rsmq: Option<R>,
    job_completed_tx: JobCompletedSender,
    worker_code_execution_metric: &mut f32,
    _worker_flow_initial_transition_duration: Option<Histo>,
    _worker_code_execution_duration: Option<Histo>,
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
        if let Some(parent_job) = job.parent_job {
            if let Err(e) = sqlx::query_scalar!(
                "UPDATE queue SET flow_status = jsonb_set(jsonb_set(COALESCE(flow_status, '{}'::jsonb), array[$1], COALESCE(flow_status->$1, '{}'::jsonb)), array[$1, 'started_at'], to_jsonb(now()::text)) WHERE id = $2 AND workspace_id = $3",
                &job.id.to_string(),
                parent_job,
                &job.workspace_id
            )
            .execute(db)
            .await {
                tracing::error!("Could not update parent job started_at flow_status: {}", e);
            }
        }
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
            .map_err(|e| Error::InternalErr(format!("fetching step flow status: {e:#}")))?
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
            {
                let logs =
                    "Job skipped because args & path found in cache and not expired".to_string();
                append_logs(&job.id, &job.workspace_id, logs, db).await;
            }
            job_completed_tx
                .send(JobCompleted {
                    job: job,
                    result: cached_resource_value,
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
    if job.is_flow() {
        #[cfg(feature = "prometheus")]
        let timer = _worker_flow_initial_transition_duration.map(|x| x.start_timer());
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
        #[cfg(feature = "prometheus")]
        timer.map(|x| x.stop_and_record());
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
            logs.push_str("WARNING: This job has concurrency limits enabled. Concurrency limits are going to become an Enterprise Edition feature in the near future.\n");
            logs.push_str("---\n");
        }

        tracing::debug!(
            workspace_id = %job.workspace_id,
            "handling job {}",
            job.id
        );
        append_logs(&job.id, &job.workspace_id, logs, db).await;

        let mut column_order: Option<Vec<String>> = None;
        let result = match job.job_kind {
            JobKind::Dependencies => {
                handle_dependency_job(
                    &job,
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
            JobKind::FlowDependencies => {
                handle_flow_dependency_job(
                    &job,
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
            JobKind::AppDependencies => handle_app_dependency_job(
                &job,
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
                #[cfg(feature = "prometheus")]
                let timer = _worker_code_execution_duration.map(|x| x.start_timer());
                let metric_timer = Instant::now();
                let r = handle_code_execution_job(
                    job.as_ref(),
                    db,
                    client,
                    job_dir,
                    worker_dir,
                    &mut mem_peak,
                    &mut canceled_by,
                    base_internal_url,
                    worker_name,
                    &mut column_order,
                )
                .await;
                *worker_code_execution_metric += metric_timer.elapsed().as_secs_f32();
                #[cfg(feature = "prometheus")]
                timer.map(|x| x.stop_and_record());
                r
            }
        };

        //it's a test job, no need to update the db
        if job.as_ref().workspace_id == "" {
            return Ok(());
        }

        if result
            .as_ref()
            .is_err_and(|err| matches!(err, &Error::AlreadyCompleted(_)))
        {
            return Ok(());
        }

        process_result(
            job,
            result,
            job_dir,
            job_completed_tx,
            mem_peak,
            canceled_by,
            cached_res_path,
            client.get_token().await,
            column_order,
            db,
        )
        .await?;
    };
    Ok(())
}

async fn process_result(
    job: Arc<QueuedJob>,
    result: error::Result<Box<RawValue>>,
    job_dir: &str,
    job_completed_tx: JobCompletedSender,
    mem_peak: i32,
    canceled_by: Option<CanceledBy>,
    cached_res_path: Option<String>,
    token: String,
    column_order: Option<Vec<String>>,
    db: &DB,
) -> error::Result<()> {
    match result {
        Ok(r) => {
            let job = if let Some(column_order) = column_order {
                let mut job_with_column_order = (*job).clone();
                match job_with_column_order.flow_status {
                    Some(_) => {
                        tracing::warn!("flow_status was expected to be none");
                    }
                    None => {
                        job_with_column_order.flow_status =
                            Some(sqlx::types::Json(to_raw_value(&serde_json::json!({
                                "_metadata": {
                                    "column_order": column_order
                                }
                            }))));
                    }
                }
                Arc::new(job_with_column_order)
            } else {
                job
            };
            job_completed_tx
                .send(JobCompleted {
                    job,
                    result: r,
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
                        let last_10_log_lines = sqlx::query_scalar!(
                            "SELECT right(logs, 600) FROM job_logs WHERE job_id = $1 AND workspace_id = $2 ORDER BY created_at DESC LIMIT 1",
                            &job.id,
                            &job.workspace_id
                        ).fetch_one(db).await.ok().flatten().unwrap_or("".to_string());

                        let log_lines = last_10_log_lines
                            .split("CODE EXECUTION ---")
                            .last()
                            .unwrap_or(&last_10_log_lines);

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

pub fn build_envs(
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

pub struct ContentReqLangEnvs {
    pub content: String,
    pub lockfile: Option<String>,
    pub language: Option<ScriptLang>,
    pub envs: Option<Vec<String>>,
    pub codebase: Option<String>,
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
        codebase: None,
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

pub async fn get_script_content_by_hash(
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
            bool,
        ),
    >(
        "SELECT content, lock, language, envs, codebase IS NOT NULL FROM script WHERE hash = $1 AND workspace_id = $2",
    )
    .bind(script_hash.0)
    .bind(w_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::InternalErr(format!("expected content and lock")))?;
    Ok(ContentReqLangEnvs {
        content: r.0,
        lockfile: r.1,
        language: r.2,
        envs: r.3,
        codebase: if r.4 {
            Some(script_hash.to_string())
        } else {
            None
        },
    })
}

#[tracing::instrument(level = "trace", skip_all)]
async fn handle_code_execution_job(
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    job_dir: &str,
    worker_dir: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    base_internal_url: &str,
    worker_name: &str,
    column_order: &mut Option<Vec<String>>,
) -> error::Result<Box<RawValue>> {
    let ContentReqLangEnvs {
        content: inner_content,
        lockfile: requirements_o,
        language,
        envs,
        codebase,
    } = match job.job_kind {
        JobKind::Preview => ContentReqLangEnvs {
            content: job
                .raw_code
                .clone()
                .unwrap_or_else(|| "no raw code".to_owned()),
            lockfile: job.raw_lock.clone(),
            language: job.language.to_owned(),
            envs: None,
            codebase: if job
                .script_hash
                .is_some_and(|y| y.0 == PREVIEW_IS_CODEBASE_HASH)
            {
                Some(job.id.to_string())
            } else {
                None
            },
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
        return do_postgresql(
            job,
            &client,
            &inner_content,
            db,
            mem_peak,
            canceled_by,
            worker_name,
            column_order,
        )
        .await;
    } else if language == Some(ScriptLang::Mysql) {
        return do_mysql(
            job,
            &client,
            &inner_content,
            db,
            mem_peak,
            canceled_by,
            worker_name,
            column_order,
        )
        .await;
    } else if language == Some(ScriptLang::Bigquery) {
        #[cfg(not(feature = "enterprise"))]
        {
            return Err(Error::ExecutionErr(
                "Bigquery is only available with an enterprise license".to_string(),
            ));
        }

        #[cfg(feature = "enterprise")]
        {
            return do_bigquery(
                job,
                &client,
                &inner_content,
                db,
                mem_peak,
                canceled_by,
                worker_name,
                column_order,
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
                &inner_content,
                db,
                mem_peak,
                canceled_by,
                worker_name,
                column_order,
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

        #[cfg(feature = "enterprise")]
        {
            return do_mssql(
                job,
                &client,
                &inner_content,
                db,
                mem_peak,
                canceled_by,
                worker_name,
            )
            .await;
        }
    } else if language == Some(ScriptLang::Graphql) {
        return do_graphql(
            job,
            &client,
            &inner_content,
            db,
            mem_peak,
            canceled_by,
            worker_name,
        )
        .await;
    } else if language == Some(ScriptLang::Nativets) {
        append_logs(
            &job.id,
            &job.workspace_id,
            "\n--- FETCH TS EXECUTION ---\n",
            db,
        )
        .await;

        let reserved_variables = get_reserved_variables(job, &client.get_token().await, db).await?;

        let env_code = format!(
            "const process = {{ env: {{}} }};\nconst BASE_URL = '{base_internal_url}';\nconst BASE_INTERNAL_URL = '{base_internal_url}';\nprocess.env['BASE_URL'] = BASE_URL;process.env['BASE_INTERNAL_URL'] = BASE_INTERNAL_URL;\n{}",
            reserved_variables
                .iter()
                .map(|(k, v)| format!("const {} = '{}';\nprocess.env['{}'] = '{}';\n", k, v, k, v))
                .collect::<Vec<String>>()
                .join("\n"));
        let (result, ts_logs) = do_nativets(
            job,
            &client,
            env_code,
            inner_content,
            db,
            mem_peak,
            canceled_by,
            worker_name,
        )
        .await?;
        append_logs(&job.id, &job.workspace_id, ts_logs, db).await;
        return Ok(result);
    }

    let lang_str = job
        .language
        .as_ref()
        .map(|x| format!("{x:?}"))
        .unwrap_or_else(|| "NO_LANG".to_string());

    tracing::debug!(
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
                codebase,
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
        Some(ScriptLang::Php) => {
            handle_php_job(
                requirements_o,
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
