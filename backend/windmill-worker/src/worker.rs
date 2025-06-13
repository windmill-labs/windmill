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
use windmill_common::{
    apps::AppScriptId,
    auth::{fetch_authed_from_permissioned_as, JWTAuthClaims, JobPerms},
    cache::{future::FutureCachedExt, ScriptData, ScriptMetadata},
    jwt,
    schema::{should_validate_schema, SchemaValidator},
    scripts::PREVIEW_IS_TAR_CODEBASE_HASH,
    utils::WarnAfterExt,
    worker::{
        get_memory, get_vcpus, get_windmill_memory_usage, get_worker_memory_usage, write_file,
        ROOT_CACHE_DIR, ROOT_CACHE_NOMOUNT_DIR, TMP_DIR,
    },
    KillpillSender,
};

#[cfg(feature = "enterprise")]
use windmill_common::ee::LICENSE_KEY_VALID;

use anyhow::{Context, Result};
use const_format::concatcp;
#[cfg(feature = "prometheus")]
use prometheus::IntCounter;

use tracing::{field, Instrument};
#[cfg(feature = "prometheus")]
use windmill_common::METRICS_DEBUG_ENABLED;
#[cfg(feature = "prometheus")]
use windmill_common::METRICS_ENABLED;

use reqwest::Response;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::{types::Json, Pool, Postgres};
use std::{
    collections::HashMap,
    fs::DirBuilder,
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
    users::SUPERADMIN_SECRET_EMAIL,
    utils::StripPath,
    worker::{update_ping, CLOUD_HOSTED, NO_LOGS, WORKER_CONFIG, WORKER_GROUP},
    DB, IS_READY,
};

use windmill_queue::{
    append_logs, canceled_job_to_result, empty_result, pull, push, CanceledBy, MiniPulledJob,
    PulledJob, PushArgs, PushIsolationLevel, HTTP_CLIENT,
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
        mpsc::{self, Sender},
        RwLock,
    },
    task::JoinHandle,
    time::Instant,
};

use rand::Rng;

use crate::{
    bash_executor::{handle_bash_job, handle_powershell_job},
    bun_executor::handle_bun_job,
    common::{
        build_args_map, cached_result_path, get_cached_resource_value_if_valid,
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
    worker_flow::{handle_flow, update_flow_status_in_progress},
    worker_lockfiles::{
        handle_app_dependency_job, handle_dependency_job, handle_flow_dependency_job,
    },
};

#[cfg(feature = "rust")]
use crate::rust_executor::handle_rust_job;

#[cfg(feature = "nu")]
use crate::nu_executor::{handle_nu_job, JobHandlerInput as JobHandlerInputNu};

#[cfg(feature = "java")]
use crate::java_executor::{handle_java_job, JobHandlerInput as JobHandlerInputJava};

#[cfg(feature = "ruby")]
use crate::ruby_executor::{handle_ruby_job, JobHandlerInput as JobHandlerInputRuby};

#[cfg(feature = "php")]
use crate::php_executor::handle_php_job;

#[cfg(feature = "python")]
use crate::python_executor::{handle_python_job, PyVersion};

#[cfg(feature = "python")]
use crate::ansible_executor::handle_ansible_job;

#[cfg(feature = "mysql")]
use crate::mysql_executor::do_mysql;

#[cfg(feature = "oracledb")]
use crate::oracledb_executor::do_oracledb;

use backon::ConstantBuilder;
use backon::{BackoffBuilder, Retryable};

#[cfg(feature = "enterprise")]
use crate::dedicated_worker::create_dedicated_worker_map;

#[cfg(feature = "enterprise")]
use crate::snowflake_executor::do_snowflake;

#[cfg(all(feature = "enterprise", feature = "mssql"))]
use crate::mssql_executor::do_mssql;

#[cfg(all(feature = "enterprise", feature = "bigquery"))]
use crate::bigquery_executor::do_bigquery;

#[cfg(feature = "benchmark")]
use crate::bench::{benchmark_init, BenchmarkInfo, BenchmarkIter};

use windmill_common::add_time;

// struct Permission
pub async fn create_token(db: &DB, job: &MiniPulledJob, perms: Option<JobPerms>) -> String {
    // skipping test runs
    if job.workspace_id != "" {
        let label = if job.permissioned_as != format!("u/{}", job.created_by)
            && job.permissioned_as != job.created_by
        {
            format!("ephemeral-script-end-user-{}", job.created_by)
        } else {
            "ephemeral-script".to_string()
        };
        create_token_for_owner(
            &db,
            &job.workspace_id,
            &job.permissioned_as,
            &label,
            *SCRIPT_TOKEN_EXPIRY,
            &job.permissioned_as_email,
            &job.id,
            perms,
        )
        .warn_after_seconds(5)
        .await
        .expect("could not create job token")
    } else {
        return "".to_string();
    }
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
    perms: Option<JobPerms>,
) -> error::Result<String> {
    // TODO: Bad implementation. We should not have access to this DB here.
    if let Some(token) = JOB_TOKEN.as_ref() {
        return Ok(token.clone());
    }

    let job_perms = if perms.is_some() {
        Ok(perms)
    } else {
        sqlx::query_as!(
            JobPerms,
            "SELECT email, username, is_admin, is_operator, groups, folders FROM job_perms WHERE job_id = $1 AND workspace_id = $2",
            job_id,
            w_id
        )
    .fetch_optional(db)
    .await
    };

    let job_authed = match job_perms {
        Ok(Some(jp)) => jp.into(),
        _ => {
            tracing::warn!("Could not get permissions for job {job_id} from job_perms table, getting permissions directly...");
            fetch_authed_from_permissioned_as(owner.to_string(), email.to_string(), w_id, db)
                .await
                .map_err(|e| {
                    Error::internal_err(format!(
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
        job_id: Some(job_id.to_string()),
        scopes: None,
    };

    let token = jwt::encode_with_internal_secret(&payload)
        .await
        .with_context(|| format!("Could not encode JWT token for job {job_id}"))?;

    Ok(format!("jwt_{}", token))
}

pub const PY310_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "python_310");
pub const PY311_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "python_311");
pub const PY312_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "python_312");
pub const PY313_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "python_313");

pub const TAR_PY310_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "tar/python_310");
pub const TAR_PY311_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "tar/python_311");
pub const TAR_PY312_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "tar/python_312");
pub const TAR_PY313_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "tar/python_313");

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

// Java
pub const JAVA_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "java");
pub const COURSIER_CACHE_DIR: &str = concatcp!(JAVA_CACHE_DIR, "/coursier-cache");
pub const JAVA_REPOSITORY_DIR: &str = concatcp!(JAVA_CACHE_DIR, "/repository");

// Ruby
pub const RUBY_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "ruby");

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

pub const DEFAULT_CLOUD_TIMEOUT: u64 = 900;
pub const DEFAULT_SELFHOSTED_TIMEOUT: u64 = 604800; // 7 days
pub const DEFAULT_SLEEP_QUEUE: u64 = 50;

// only 1 native job so that we don't have to worry about concurrency issues on non dedicated native jobs workers
pub const DEFAULT_NATIVE_JOBS: usize = 1;

const VACUUM_PERIOD: u32 = 50000;

#[cfg(any(target_os = "linux"))]
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

#[cfg(windows)]
const DOTNET_DEFAULT_PATH: &str = "C:\\Program Files\\dotnet\\dotnet.exe";
#[cfg(unix)]
const DOTNET_DEFAULT_PATH: &str = "/usr/bin/dotnet";

lazy_static::lazy_static! {

    pub static ref JOB_TOKEN: Option<String> = std::env::var("JOB_TOKEN").ok();

    pub static ref SLEEP_QUEUE: u64 = std::env::var("SLEEP_QUEUE")
    .ok()
    .and_then(|x| x.parse::<u64>().ok())
    .unwrap_or(DEFAULT_SLEEP_QUEUE * std::env::var("NUM_WORKERS")
    .ok()
    .map(|x| x.parse().ok())
    .flatten()
    .unwrap_or(2) / 2);


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


    pub static ref REFRESH_CGROUP_READINGS: bool = std::env::var("REFRESH_CGROUP_READINGS")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(false);

}

#[cfg(windows)]
lazy_static::lazy_static! {
    pub static ref SYSTEM_ROOT: String = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
    pub static ref USERPROFILE_ENV: String = std::env::var("USERPROFILE").unwrap_or_else(|_| "/tmp".to_string());
}

//only matter if CLOUD_HOSTED
pub const MAX_RESULT_SIZE: usize = 1024 * 1024 * 2; // 2MB

pub const INIT_SCRIPT_TAG: &str = "init_script";

#[derive(Clone)]
pub struct AuthedClient {
    pub base_internal_url: String,
    pub workspace: String,
    pub token: String,
    pub force_client: Option<reqwest::Client>,
}

impl AuthedClient {
    pub async fn get(&self, url: &str, query: Vec<(&str, String)>) -> anyhow::Result<Response> {
        self.force_client
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
            .await
            .context(format!(
                "Executing request from authed http client to {url} with query {query:?}",
            ))
    }

    pub async fn get_id_token(&self, audience: &str) -> anyhow::Result<String> {
        let url = format!(
            "{}/api/w/{}/oidc/token/{}",
            self.base_internal_url, self.workspace, audience
        );
        let response = self.get(&url, vec![]).await?;
        match response.status().as_u16() {
            200u16 => Ok(response
                .json::<String>()
                .await
                .context("decoding oidc token as json string")?),
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
            200u16 => Ok(response
                .json::<T>()
                .await
                .context("decoding resource value as json")?),
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
            200u16 => Ok(response
                .json::<String>()
                .await
                .context("decoding variable value as json")?),
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
            200u16 => Ok(response
                .json::<T>()
                .await
                .context("decoding interpolated resource value as json")?),
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
            200u16 => Ok(response
                .json::<T>()
                .await
                .context("decoding completed job result as json")?),
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
            200u16 => Ok(response
                .json::<T>()
                .await
                .context("decoding result by id as json")?),
            _ => Err(anyhow::anyhow!(response.text().await.unwrap_or_default())),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct JobCompletedSender(Sender<SendResult>);

#[derive(Clone)]
pub struct SameWorkerSender(pub Sender<SameWorkerPayload>, pub Arc<AtomicU16>);

pub struct SameWorkerPayload {
    pub job_id: Uuid,
    pub recoverable: bool,
}

impl JobCompletedSender {
    pub async fn send(
        &self,
        jc: JobCompleted,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<SendResult>> {
        self.0.send(SendResult::JobCompleted(jc)).await
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
    queued_job: &MiniPulledJob,
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
    let root_job_id = queued_job.flow_innermost_root_job;
    let db = db.clone();

    tokio::spawn(async move {
            match insert_wait_time(job_id, root_job_id, &db, wait_time).await {
                Ok(()) => tracing::warn!("job {job_id} waited for an executor for a significant amount of time. Recording value wait_time={}ms", wait_time),
                Err(e) => tracing::error!("Failed to insert outstanding wait time: {}", e),
            }
    }.in_current_span());
}

// struct WorkerMtrics {
//     job_
// }

pub async fn run_worker(
    db: &Pool<Postgres>,
    hostname: &str,
    worker_name: String,
    i_worker: u64,
    _num_workers: u32,
    ip: &str,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
    killpill_tx: KillpillSender,
    base_internal_url: &str,
    agent_mode: bool,
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
        let (db, worker_name, hostname, worker_dir) = (
            db.clone(),
            worker_name.clone(),
            hostname.to_owned(),
            worker_dir.clone(),
        );
        tokio::spawn(async move {
            if let Err(e) = PyVersion::from_instance_version(&Uuid::nil(), "", &db)
                .await
                .get_python(&Uuid::nil(), &mut 0, &db, &worker_name, "", &mut None)
                .await
            {
                tracing::error!(
                    worker = %worker_name,
                    hostname = %hostname,
                    worker_dir = %worker_dir,
                    "Cannot preinstall or find Instance Python version to worker: {e}"//
                );
            }
            if let Err(e) = PyVersion::Py311
                .get_python(&Uuid::nil(), &mut 0, &db, &worker_name, "", &mut None)
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

    DirBuilder::new()
        .recursive(true)
        .create(&worker_dir)
        .expect("could not create initial worker dir");

    if !*DISABLE_NSJAIL {
        let _ = write_file(
            &worker_dir,
            "download_deps.py.sh",
            INCLUDE_DEPS_PY_SH_CONTENT,
        );
    }

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
    benchmark_init(benchmark_jobs, &db).await;

    #[cfg(feature = "prometheus")]
    if let Some(ws) = WORKER_STARTED.as_ref() {
        ws.inc();
    }

    let (same_worker_tx, mut same_worker_rx) = mpsc::channel::<SameWorkerPayload>(5);

    let (job_completed_tx, job_completed_rx) = mpsc::channel::<SendResult>(3);

    let job_completed_tx = JobCompletedSender(job_completed_tx);

    let same_worker_queue_size = Arc::new(AtomicU16::new(0));
    let same_worker_tx = SameWorkerSender(same_worker_tx, same_worker_queue_size.clone());
    let job_completed_processor_is_done = Arc::new(AtomicBool::new(false));

    let send_result = start_background_processor(
        job_completed_rx,
        job_completed_tx.0.clone(),
        same_worker_queue_size.clone(),
        job_completed_processor_is_done.clone(),
        base_internal_url.to_string(),
        db.clone(),
        worker_dir.clone(),
        same_worker_tx.clone(),
        worker_name.clone(),
        killpill_tx.clone(),
        is_dedicated_worker,
    );

    let mut last_executed_job: Option<Instant> = None;

    #[cfg(feature = "benchmark")]
    let mut started = false;

    #[cfg(feature = "benchmark")]
    let mut infos = BenchmarkInfo::new();

    let vacuum_shift = rand::rng().random_range(0..VACUUM_PERIOD);

    IS_READY.store(true, Ordering::Relaxed);
    tracing::info!(
        worker = %worker_name, hostname = %hostname,
        "listening for jobs, WORKER_GROUP: {}, config: {:?}",
        *WORKER_GROUP,
        WORKER_CONFIG.read().await
    );

    // (dedi_path, dedicated_worker_tx, dedicated_worker_handle)
    // Option<Sender<Arc<QueuedJob>>>,
    // Option<JoinHandle<()>>,

    #[cfg(feature = "enterprise")]
    let (dedicated_workers, is_flow_worker, dedicated_handles): (
        HashMap<String, Sender<Arc<MiniPulledJob>>>,
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
        HashMap<String, Sender<Arc<MiniPulledJob>>>,
        bool,
        Vec<JoinHandle<()>>,
    ) = (HashMap::new(), false, vec![]);

    if i_worker == 1 {
        if let Err(e) = queue_init_bash_maybe(db, same_worker_tx.clone(), &worker_name).await {
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
    let mut last_30jobs_suspended: Vec<bool> = vec![false; 30];
    let mut last_suspend_first = Instant::now();
    let mut killed_but_draining_same_worker_jobs = false;

    let mut killpill_rx2 = killpill_rx.resubscribe();
    loop {
        #[cfg(feature = "enterprise")]
        {
            if let Ok(_) = killpill_rx.try_recv() {
                tracing::info!(worker = %worker_name, hostname = %hostname, "killpill received on worker waiting for valid key");
                job_completed_tx
                    .0
                    .send(SendResult::Kill)
                    .await
                    .expect("send kill to job completed tx");
                break;
            }
            let valid_key = *LICENSE_KEY_VALID.read().await;

            if !valid_key {
                tracing::error!(
                    worker = %worker_name, hostname = %hostname,
                    "Invalid license key, workers require a valid license key, sleeping for 30s waiting for valid key to be set"
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
            let tags = WORKER_CONFIG.read().await.worker_tags.clone();

            let memory_usage = get_worker_memory_usage();
            let wm_memory_usage = get_windmill_memory_usage();

            let (vcpus, memory) = if *REFRESH_CGROUP_READINGS
                && last_reading.elapsed().as_secs() > NUM_SECS_READINGS
            {
                last_reading = Instant::now();
                (get_vcpus(), get_memory())
            } else {
                (None, None)
            };

            let (occupancy_rate, occupancy_rate_15s, occupancy_rate_5m, occupancy_rate_30m) =
                occupancy_metrics.update_occupancy_metrics();

            if let Err(e) = (|| sqlx::query!(
                "UPDATE worker_ping SET ping_at = now(), jobs_executed = $1, custom_tags = $2,
                 occupancy_rate = $3, memory_usage = $4, wm_memory_usage = $5, vcpus = COALESCE($7, vcpus),
                 memory = COALESCE($8, memory), occupancy_rate_15s = $9, occupancy_rate_5m = $10, occupancy_rate_30m = $11 WHERE worker = $6",
                jobs_executed,
                tags.as_slice(),
                occupancy_rate,
                memory_usage,
                wm_memory_usage,
                &worker_name,
                vcpus,
                memory,
                occupancy_rate_15s,
                occupancy_rate_5m,
                occupancy_rate_30m
            ).execute(db)).retry(
                ConstantBuilder::default()
                    .with_delay(std::time::Duration::from_secs(2))
                    .with_max_times(10)
                    .build(),
            )
            .notify(|err, dur| {
                tracing::error!(
                    worker = %worker_name, hostname = %hostname,
                    "retrying updating worker ping in {dur:#?}, err: {err:#?}"
                );
            })
            .sleep(tokio::time::sleep)
            .await {
                tracing::error!(
                    worker = %worker_name, hostname = %hostname,
                    "failed to update worker ping, exiting: {}", e);
                killpill_tx.send();
            }
            tracing::info!(
                worker = %worker_name, hostname = %hostname,
                "ping update, memory: container={}MB, windmill={}MB",
                memory_usage.unwrap_or_default() / (1024 * 1024),
                wm_memory_usage.unwrap_or_default() / (1024 * 1024)
            );

            last_ping = Instant::now();
        }

        if (jobs_executed as u32 + vacuum_shift) % VACUUM_PERIOD == 0 {
            let db2 = db.clone();
            let current_span = tracing::Span::current();
            let worker_name = worker_name.clone();
            let hostname = hostname.to_string();
            tokio::task::spawn(
                (async move {
                    tracing::info!(worker = %worker_name, hostname = %hostname, "vacuuming queue");
                    if let Err(e) = sqlx::query!("VACUUM v2_job_queue, v2_job_runtime, v2_job_status")
                        .execute(&db2)
                        .await
                    {
                        tracing::error!(worker = %worker_name, hostname = %hostname, "failed to vacuum queue: {}", e);
                    }
                    tracing::info!(worker = %worker_name, hostname = %hostname, "vacuumed queue");
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

        #[cfg(feature = "benchmark")]
        if benchmark_jobs > 0 && infos.iters == benchmark_jobs as u64 {
            tracing::info!("benchmark finished, exiting");
            job_completed_tx
                .0
                .send(SendResult::Kill)
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
                let job = get_same_worker_job(db, &same_worker_job).await;
                // tracing::error!("r: {:?}", r);
                if job.is_err() && !same_worker_job.recoverable {
                    tracing::error!(
                        worker = %worker_name, hostname = %hostname,
                        "failed to fetch same_worker job on a non recoverable job, exiting"
                    );
                    job_completed_tx
                        .0
                        .send(SendResult::Kill)
                        .await
                        .expect("send kill to job completed tx");
                    break;
                } else {
                    job
                }
            } else if let Ok(_) = killpill_rx.try_recv() {
                if !killed_but_draining_same_worker_jobs {
                    tracing::info!(worker = %worker_name, hostname = %hostname, "received killpill for worker {}, jobs are not pulled anymore except same_worker jobs", i_worker);
                    killed_but_draining_same_worker_jobs = true;
                    job_completed_tx
                        .0
                        .send(SendResult::Kill)
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
                let pull_time = Instant::now();
                let likelihood_of_suspend =
                    (1.0 + last_30jobs_suspended.iter().filter(|&&x| x).count() as f64) / 31.0;
                let suspend_first = suspend_first_success
                    || rand::random::<f64>() < likelihood_of_suspend
                    || last_suspend_first.elapsed().as_secs_f64() > 5.0;

                if suspend_first {
                    last_suspend_first = Instant::now();
                }

                let job = pull(&db, suspend_first, &worker_name).await;

                add_time!(bench, "job pulled from DB");
                let duration_pull_s = pull_time.elapsed().as_secs_f64();
                let err_pull = job.is_ok();
                // let empty = job.as_ref().is_ok_and(|x| x.is_none());

                if !agent_mode && duration_pull_s > 0.5 {
                    let empty = job.as_ref().is_ok_and(|x| x.0.is_none());
                    tracing::warn!(worker = %worker_name, hostname = %hostname, "pull took more than 0.5s ({duration_pull_s}), this is a sign that the database is VERY undersized for this load. empty: {empty}, err: {err_pull}");
                    #[cfg(feature = "prometheus")]
                    if empty {
                        if let Some(wp) = worker_pull_over_500_counter_empty.as_ref() {
                            wp.inc();
                        }
                    } else if let Some(wp) = worker_pull_over_500_counter.as_ref() {
                        wp.inc();
                    }
                } else if !agent_mode && duration_pull_s > 0.1 {
                    let empty = job.as_ref().is_ok_and(|x| x.0.is_none());
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
                    let suspend_success = j.1;
                    if suspend_first {
                        last_30jobs_suspended.push(suspend_success);
                        if last_30jobs_suspended.len() > 30 {
                            last_30jobs_suspended.remove(0);
                        }
                    }
                    suspend_first_success = suspend_first && suspend_success;
                    #[cfg(feature = "prometheus")]
                    if j.0.is_some() {
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
                job.map(|x| x.0)
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
                                if let Err(e) = dedicated_worker_tx.send(Arc::new(job.job)).await {
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
                        .send(JobCompleted {
                            job: Arc::new(job.job),
                            success: true,
                            result: Arc::new(empty_result()),
                            result_columns: None,
                            mem_peak: 0,
                            cached_res_path: None,
                            token: "".to_string(),
                            canceled_by: None,
                            duration: None,
                        })
                        .await
                        .expect("send job completed END");
                    add_time!(bench, "sent job completed");
                } else {
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

                    let job_dir = format!("{worker_dir}/{}", job.id);

                    DirBuilder::new()
                        .recursive(true)
                        .create(&job_dir)
                        .expect("could not create job dir");

                    let same_worker = job.same_worker;

                    let folder = if job.script_lang == Some(ScriptLang::Go) {
                        DirBuilder::new()
                            .recursive(true)
                            .create(&format!("{job_dir}/go"))
                            .expect("could not create go dir");
                        "/go"
                    } else {
                        ""
                    };

                    let target = &format!("{job_dir}{folder}/shared");

                    if same_worker && job.parent_job.is_some() {
                        if tokio::fs::metadata(target).await.is_err() {
                            let parent_flow = job.parent_job.unwrap();
                            let parent_shared_dir = format!("{worker_dir}/{parent_flow}/shared");
                            DirBuilder::new()
                                .recursive(true)
                                .create(&parent_shared_dir)
                                .expect("could not create parent shared dir");

                            symlink(&parent_shared_dir, target)
                                .await
                                .expect("could not symlink target");
                        }
                    } else {
                        DirBuilder::new()
                            .recursive(true)
                            .create(target)
                            .expect("could not create shared dir");
                    }

                    #[cfg(feature = "prometheus")]
                    let tag = job.tag.clone();

                    let is_init_script: bool = job.tag.as_str() == INIT_SCRIPT_TAG;
                    let PulledJob {
                        job,
                        raw_code,
                        raw_lock,
                        raw_flow,
                        parent_runnable_path,
                        permissioned_as_email,
                        permissioned_as_username,
                        permissioned_as_is_admin,
                        permissioned_as_is_operator,
                        permissioned_as_groups,
                        permissioned_as_folders,
                    } = job;
                    let job_perms = match (
                        permissioned_as_email,
                        permissioned_as_username,
                        permissioned_as_is_admin,
                        permissioned_as_is_operator,
                        permissioned_as_groups,
                        permissioned_as_folders,
                    ) {
                        (
                            Some(email),
                            Some(username),
                            Some(is_admin),
                            Some(is_operator),
                            Some(groups),
                            Some(folders),
                        ) => Some(JobPerms {
                            email,
                            username,
                            is_admin,
                            is_operator,
                            groups,
                            folders,
                        }),
                        _ => None,
                    };

                    let token = create_token(&db, &job, job_perms).await;
                    let authed_client = AuthedClient {
                        base_internal_url: base_internal_url.to_string(),
                        token,
                        workspace: job.workspace_id.to_string(),
                        force_client: None,
                    };

                    let arc_job = Arc::new(job);
                    add_time!(bench, "handle_queued_job START");

                    let span = tracing::span!(tracing::Level::INFO, "job",
                            job_id = %arc_job.id, root_job = field::Empty, workspace_id = %arc_job.workspace_id,  worker = %worker_name, hostname = %hostname, tag = %arc_job.tag,
                            language = field::Empty,
                            script_path = field::Empty, flow_step_id = field::Empty, parent_job = field::Empty,
                            otel.name = field::Empty);
                    let rj = if let Some(root_job) = arc_job.flow_innermost_root_job {
                        root_job
                    } else {
                        arc_job.id
                    };
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

                    windmill_common::otel_ee::set_span_parent(&span, &rj);
                    // span.context().span().add_event_with_timestamp("job created".to_string(), arc_job.created_at.into(), vec![]);

                    match handle_queued_job(
                        arc_job.clone(),
                        raw_code,
                        raw_lock,
                        raw_flow,
                        parent_runnable_path,
                        db,
                        &authed_client,
                        &hostname,
                        &worker_name,
                        &worker_dir,
                        &job_dir,
                        same_worker_tx.clone(),
                        base_internal_url,
                        job_completed_tx.clone(),
                        &mut occupancy_metrics,
                        &mut killpill_rx2,
                        #[cfg(feature = "benchmark")]
                        &mut bench,
                    )
                    .instrument(span)
                    .await
                    {
                        Err(err) => {
                            handle_job_error(
                                db,
                                &authed_client,
                                arc_job.as_ref(),
                                0,
                                None,
                                err,
                                false,
                                same_worker_tx.clone(),
                                &worker_dir,
                                &worker_name,
                                (&job_completed_tx.0).clone(),
                                #[cfg(feature = "benchmark")]
                                &mut bench,
                            )
                            .await;
                            if is_init_script {
                                tracing::error!("init script job failed (in handler), exiting");
                                update_worker_ping_for_failed_init_script(
                                    db,
                                    &worker_name,
                                    arc_job.id,
                                )
                                .await;
                                break;
                            }
                        }
                        Ok(false) if is_init_script => {
                            tracing::error!("init script job failed, exiting");
                            update_worker_ping_for_failed_init_script(db, &worker_name, arc_job.id)
                                .await;
                            break;
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

                    if !KEEP_JOB_DIR.load(Ordering::Relaxed) && !(arc_job.is_flow() && same_worker)
                    {
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
    if let Err(e) = send_result.await {
        tracing::error!("error in awaiting send_result process: {e:?}")
    }
    tracing::info!(worker = %worker_name, hostname = %hostname, "worker {} exited", worker_name);
    tracing::info!(worker = %worker_name, hostname = %hostname, "number of jobs executed: {}", jobs_executed);
}

async fn get_same_worker_job(
    db: &Pool<Postgres>,
    same_worker_job: &SameWorkerPayload,
) -> windmill_common::error::Result<Option<PulledJob>> {
    sqlx::query_as::<_, PulledJob>(
        "WITH ping AS (
                        UPDATE v2_job_runtime SET ping = NOW() WHERE id = $1
                    ),
                    started_at AS (
                        UPDATE v2_job_queue SET started_at = NOW() WHERE id = $1
                    )
                    SELECT
                    v2_job_queue.workspace_id,
                    v2_job_queue.id,
                    v2_job.args,
                    v2_job.parent_job,
                    v2_job.created_by,
                    v2_job_queue.started_at,
                    scheduled_for,
                    v2_job.runnable_path,
                    v2_job.kind,
                    v2_job.runnable_id,
                    v2_job_queue.canceled_reason,
                    v2_job_queue.canceled_by,
                    v2_job.permissioned_as,
                    v2_job.permissioned_as_email,
                    v2_job_status.flow_status,
                    v2_job.tag,
                    v2_job.script_lang,
                    v2_job.same_worker,
                    v2_job.pre_run_error,
                    v2_job.concurrent_limit,
                    v2_job.concurrency_time_window_s,
                    v2_job.flow_innermost_root_job,
                    v2_job.timeout,
                    v2_job.flow_step_id,
                    v2_job.cache_ttl,
                    v2_job_queue.priority,
                    v2_job.preprocessed,
                    v2_job.script_entrypoint_override,
                    v2_job.trigger,
                    v2_job.trigger_kind,
                    v2_job.visible_to_owner,
                    v2_job.raw_code,
                    v2_job.raw_lock,
                    v2_job.raw_flow,
                    pj.runnable_path as parent_runnable_path,
                    p.email as permissioned_as_email, p.username as permissioned_as_username, p.is_admin as permissioned_as_is_admin,
                    p.is_operator as permissioned_as_is_operator, p.groups as permissioned_as_groups, p.folders as permissioned_as_folders
                    FROM v2_job_queue
                    INNER JOIN v2_job ON v2_job.id = v2_job_queue.id
                    LEFT JOIN v2_job_status ON v2_job_status.id = v2_job_queue.id
                    LEFT JOIN job_perms p ON p.job_id = v2_job.id
                    LEFT JOIN v2_job pj ON v2_job.parent_job = pj.id
                    WHERE v2_job_queue.id = $1
",
    )
    .bind(same_worker_job.job_id)
    .fetch_optional(db)
    .await
    .map_err(|e| {
        Error::internal_err(format!(
            "Impossible to fetch same_worker job {}: {}",
            same_worker_job.job_id, e
        ))
    })
}

async fn queue_init_bash_maybe<'c>(
    db: &Pool<Postgres>,
    same_worker_tx: SameWorkerSender,
    worker_name: &str,
) -> error::Result<bool> {
    if let Some(content) = WORKER_CONFIG.read().await.init_bash.clone() {
        let tx = PushIsolationLevel::IsolatedRoot(db.clone());
        let ehm = HashMap::new();
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
            PushArgs::from(&ehm),
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
        Ok(true)
    } else {
        Ok(false)
    }
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

#[derive(Debug, Clone)]
pub struct JobCompleted {
    pub job: Arc<MiniPulledJob>,
    pub result: Arc<Box<RawValue>>,
    pub result_columns: Option<Vec<String>>,
    pub mem_peak: i32,
    pub success: bool,
    pub cached_res_path: Option<String>,
    pub token: String,
    pub canceled_by: Option<CanceledBy>,
    pub duration: Option<i64>,
}

async fn do_nativets(
    job: &MiniPulledJob,
    client: &AuthedClient,
    env_code: String,
    code: String,
    db: &Pool<Postgres>,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    worker_name: &str,
    occupancy_metrics: &mut OccupancyMetrics,
) -> windmill_common::error::Result<Box<RawValue>> {
    let args = build_args_map(job, client, db).await?.map(Json);
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
        db,
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

async fn handle_queued_job(
    job: Arc<MiniPulledJob>,
    raw_code: Option<String>,
    raw_lock: Option<String>,
    raw_flow: Option<Json<Box<RawValue>>>,
    parent_runnable_path: Option<String>,
    db: &DB,
    client: &AuthedClient,
    hostname: &str,
    worker_name: &str,
    worker_dir: &str,
    job_dir: &str,
    same_worker_tx: SameWorkerSender,
    base_internal_url: &str,
    job_completed_tx: JobCompletedSender,
    occupancy_metrics: &mut OccupancyMetrics,
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
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

    if job.is_flow_step() {
        let _ = update_flow_status_in_progress(
            db,
            &job.workspace_id,
            job.parent_job
                .ok_or_else(|| Error::internal_err(format!("expected parent job")))?,
            job.id,
        )
        .warn_after_seconds(5)
        .await?;
    } else if let Some(parent_job) = job.parent_job {
        let _ = sqlx::query_scalar!(
            "UPDATE v2_job_status SET
                workflow_as_code_status = jsonb_set(
                    jsonb_set(
                        COALESCE(workflow_as_code_status, '{}'::jsonb),
                        array[$1],
                        COALESCE(workflow_as_code_status->$1, '{}'::jsonb)
                    ),
                    array[$1, 'started_at'],
                    to_jsonb(now()::text)
                )
            WHERE id = $2",
            &job.id.to_string(),
            parent_job
        )
        .execute(db)
        .warn_after_seconds(5)
        .await
        .inspect_err(|e| {
            tracing::error!(
                "Could not update parent job `started_at` in workflow as code status: {}",
                e
            )
        });
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
                Some(cache::job::fetch_preview(db, &job.id, raw_lock, raw_code, raw_flow).await?)
            }
            _ => None,
        },
        _ => None,
    };

    let cached_res_path = if job.cache_ttl.is_some() {
        Some(cached_result_path(db, &client, &job, preview_data.as_ref()).await)
    } else {
        None
    };

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
                let logs =
                    "Job skipped because args & path found in cache and not expired".to_string();
                append_logs(&job.id, &job.workspace_id, logs, db).await;
            }
            job_completed_tx
                .send(JobCompleted {
                    job,
                    result,
                    result_columns: None,
                    mem_peak: 0,
                    canceled_by: None,
                    success: true,
                    cached_res_path: None,
                    token: client.token.clone(),
                    duration: None,
                })
                .await
                .expect("send job completed");

            return Ok(true);
        }
    };
    if job.is_flow() {
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
            same_worker_tx,
            worker_dir,
            job_completed_tx.0.clone(),
            worker_name,
        )
        .warn_after_seconds(10)
        .await?;
        Ok(true)
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
        append_logs(&job.id, &job.workspace_id, logs, db).await;

        let mut column_order: Option<Vec<String>> = None;
        let mut new_args: Option<HashMap<String, Box<RawValue>>> = None;
        let result = match job.kind {
            JobKind::Dependencies => {
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
            JobKind::FlowDependencies => {
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
            JobKind::AppDependencies => handle_app_dependency_job(
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
                    db,
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
            db,
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
    db: &DB,
) -> error::Result<ContentReqLangEnvs> {
    let (data, metadata) = cache::script::fetch(db, *script_hash).await?;
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
    db: &Pool<Postgres>,
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
                    db,
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
                    db,
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
    db: &sqlx::Pool<sqlx::Postgres>,
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
        JobKind::Script_Hub => {
            let ContentReqLangEnvs { content, lockfile, language, envs, codebase, schema } =
                get_hub_script_content_and_requirements(job.runnable_path.as_ref(), Some(db))
                    .await?;

            data = ScriptData { code: content, lock: lockfile };
            metadata = ScriptMetadata { language, envs, codebase, schema, schema_validator: None };
            (&data, &metadata)
        }
        JobKind::Script => {
            (arc_data, arc_metadata) = cache::script::fetch(db, script_hash()?).await?;
            (arc_data.as_ref(), arc_metadata.as_ref())
        }
        JobKind::FlowScript => {
            arc_data = cache::flow::fetch_script(db, FlowNodeId(script_hash()?.0)).await?;
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
            arc_data = cache::app::fetch_script(db, AppScriptId(script_hash()?.0)).await?;
            metadata = ScriptMetadata {
                language: job.script_lang,
                envs: None,
                codebase: None,
                schema: None,
                schema_validator: None,
            };
            (arc_data.as_ref(), &metadata)
        }
        JobKind::DeploymentCallback => {
            let script_path = job
                .runnable_path
                .as_ref()
                .ok_or_else(|| Error::internal_err("expected script path".to_string()))?;
            if script_path.starts_with("hub/") {
                let ContentReqLangEnvs { content, lockfile, language, envs, codebase, schema } =
                    get_hub_script_content_and_requirements(Some(script_path), Some(db)).await?;
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

                (arc_data, arc_metadata) = cache::script::fetch(db, ScriptHash(hash)).await?;
                (arc_data.as_ref(), arc_metadata.as_ref())
            }
        }
        _ => unreachable!(
            "handle_code_execution_job should never be reachable with a non-code execution job"
        ),
    };

    try_validate_schema(
        job,
        db,
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
            db,
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
            db,
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
                db,
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
                db,
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
                db,
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
                db,
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
            db,
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
            db,
        )
        .await;

        let reserved_variables =
            get_reserved_variables(job, &client.token, db, parent_runnable_path).await?;

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
            db,
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
                db,
                client,
                parent_runnable_path,
                &code,
                &shared_mount,
                base_internal_url,
                envs,
                new_args,
                occupancy_metrics,
            )
            .await
        }
        Some(ScriptLang::Deno) => {
            handle_deno_job(
                lock.as_ref(),
                mem_peak,
                canceled_by,
                job,
                db,
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
                db,
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
                db,
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
                db,
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
                db,
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
                db,
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
                db,
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
                db,
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
                db,
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
        Some(ScriptLang::Ruby) => {
            #[cfg(not(feature = "ruby"))]
            return Err(anyhow::anyhow!(
                "Ruby is not available because the feature is not enabled"
            )
            .into());

            #[cfg(feature = "ruby")]
            handle_ruby_job(JobHandlerInputRuby {
                mem_peak,
                canceled_by,
                job,
                db,
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
        // for related places search: ADD_NEW_LANG
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
            #[cfg(feature = "ruby")]
            ScriptLang::Ruby => Some(windmill_parser_ruby::parse_ruby_signature(code)?),
            #[cfg(not(feature = "ruby"))]
            ScriptLang::Ruby => None,
            // for related places search: ADD_NEW_LANG
        }
    } else {
        None
    })
}
