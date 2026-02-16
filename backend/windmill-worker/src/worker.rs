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
use tokio::sync::Mutex;
use tokio::time::timeout;
use windmill_common::client::AuthedClient;
use windmill_common::jobs::WorkerInternalServerInlineUtils;
use windmill_common::jobs::WORKER_INTERNAL_SERVER_INLINE_UTILS;
use windmill_common::runtime_assets::init_runtime_asset_loop;
use windmill_common::runtime_assets::register_runtime_asset;
use windmill_common::scripts::hash_to_codebase_id;
use windmill_common::scripts::is_special_codebase_hash;
use windmill_common::utils::report_critical_error;
use windmill_common::utils::retrieve_common_worker_prefix;
use windmill_common::worker::error_to_value;
use windmill_common::workspace_dependencies::RawWorkspaceDependencies;
use windmill_common::workspace_dependencies::WorkspaceDependenciesPrefetched;
use windmill_common::{
    agent_workers::DECODED_AGENT_TOKEN,
    apps::AppScriptId,
    cache::{future::FutureCachedExt, ScriptData, ScriptMetadata},
    schema::{should_validate_schema, SchemaValidator},
    utils::{create_directory_async, WarnAfterExt},
    worker::{
        make_pull_query, write_file, Connection, HttpClient, MAX_TIMEOUT,
        MIN_PERIODIC_SCRIPT_INTERVAL_SECONDS, ROOT_CACHE_DIR, ROOT_CACHE_NOMOUNT_DIR, TMP_DIR,
    },
    worker_group_job_stats::JobStatsMap,
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
    collections::{HashMap, HashSet},
    fmt::Display,
    sync::{
        atomic::{AtomicBool, AtomicU16, AtomicU8, Ordering},
        Arc,
    },
    time::Duration,
};
use windmill_parser::MainArgSignature;
use windmill_queue::DedicatedWorkerJob;
use windmill_queue::FlowRunners;
use windmill_queue::MiniCompletedJob;
use windmill_queue::PulledJobResultToJobErr;

use uuid::Uuid;

use windmill_common::{
    cache::{self, RawData},
    error::{self, to_anyhow, Error},
    flows::FlowNodeId,
    jobs::JobKind,
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang},
    tracing_init::{QUIET_MODE, VERBOSE_TARGET},
    utils::StripPath,
    worker::{CLOUD_HOSTED, NATIVE_MODE_RESOLVED, NO_LOGS, WORKER_CONFIG, WORKER_GROUP},
    DB, IS_READY,
};

use windmill_queue::{
    append_logs, canceled_job_to_result, empty_result, get_same_worker_job, pull, push_init_job,
    push_periodic_bash_job, CanceledBy, JobAndPerms, JobCompleted, MiniPulledJob,
    PrecomputedAgentInfo, PulledJob, SameWorkerPayload, HTTP_CLIENT, INIT_SCRIPT_TAG,
    PERIODIC_SCRIPT_TAG,
};

#[cfg(feature = "prometheus")]
use windmill_queue::register_metric;

use serde_json::value::RawValue;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use tokio::fs::symlink;

#[cfg(target_os = "windows")]
use tokio::fs::symlink_dir;

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

use crate::ai_executor::handle_ai_agent_job;
use crate::common::MaybeLock;
use crate::common::StreamNotifier;
use crate::{
    agent_workers::{queue_init_job, queue_periodic_job},
    bash_executor::handle_bash_job,
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
    job_logger::NO_LOGS_AT_ALL,
    js_eval::{eval_fetch_timeout, transpile_ts},
    pg_executor::do_postgresql,
    pwsh_executor::handle_powershell_job,
    result_processor::{handle_job_error, process_result, start_background_processor},
    schema::schema_validator_from_main_arg_sig,
    worker_flow::{handle_flow, SchedulePushZombieError},
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

#[cfg(feature = "ruby")]
use crate::ruby_executor::{handle_ruby_job, JobHandlerInput as JobHandlerInputRuby};

#[cfg(feature = "php")]
use crate::php_executor::handle_php_job;

#[cfg(feature = "python")]
use crate::{python_executor::handle_python_job, python_versions::PyV};
#[cfg(feature = "python")]
use windmill_common::worker::PyVAlias;

#[cfg(feature = "python")]
use crate::ansible_executor::handle_ansible_job;

#[cfg(feature = "mysql")]
use crate::mysql_executor::do_mysql;

#[cfg(feature = "duckdb")]
use crate::duckdb_executor::do_duckdb;

#[cfg(all(feature = "enterprise", feature = "oracledb"))]
use crate::oracledb_executor::do_oracledb;

#[cfg(all(feature = "private", feature = "enterprise"))]
use crate::dedicated_worker_oss::create_dedicated_worker_map;

#[cfg(feature = "enterprise")]
use crate::snowflake_executor::do_snowflake;

#[cfg(all(feature = "enterprise", feature = "mssql"))]
use crate::mssql_executor::do_mssql;

#[cfg(all(feature = "enterprise", feature = "bigquery"))]
use crate::bigquery_executor::do_bigquery;

#[cfg(feature = "benchmark")]
use windmill_common::bench::{benchmark_init, benchmark_verify, BenchmarkInfo, BenchmarkIter};

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

// Java
pub const JAVA_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "java");
pub const COURSIER_CACHE_DIR: &str = concatcp!(JAVA_CACHE_DIR, "/coursier-cache");
pub const JAVA_REPOSITORY_DIR: &str = concatcp!(JAVA_CACHE_DIR, "/repository");
pub const JAVA_HOME_DIR: &str = concatcp!(JAVA_CACHE_DIR, "/home");

// Ruby
pub const RUBY_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "ruby");

// for related places search: ADD_NEW_LANG
pub const BUN_CACHE_DIR: &str = concatcp!(ROOT_CACHE_NOMOUNT_DIR, "bun");
pub const BUN_BUNDLE_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "bun");
pub const BUN_CODEBASE_BUNDLE_CACHE_DIR: &str = concatcp!(ROOT_CACHE_NOMOUNT_DIR, "script_bundle");

pub const GO_BIN_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "gobin");
pub const POWERSHELL_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "powershell");
pub const COMPOSER_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "composer");

pub const TRACING_PROXY_CA_CERT_PATH: &str =
    concatcp!(ROOT_CACHE_NOMOUNT_DIR, "tracing_proxy_ca.pem");

const NUM_SECS_PING: u64 = 5;
const NUM_SECS_READINGS: u64 = 60;

const INCLUDE_DEPS_PY_SH_CONTENT: &str = include_str!("../nsjail/download_deps.py.sh");

const WORKER_SHELL_NAP_TIME_DURATION: u64 = 15;
const TIMEOUT_TO_RESET_WORKER_SHELL_NAP_TIME_DURATION: u64 = 2 * 60;

pub const DEFAULT_SLEEP_QUEUE: u64 = 50;

// only 1 native job so that we don't have to worry about concurrency issues on non dedicated native jobs workers
pub const DEFAULT_NATIVE_JOBS: usize = 1;

const VACUUM_PERIOD: u32 = 10000;

// #[cfg(any(target_os = "linux"))]
// const DROP_CACHE_PERIOD: u32 = 1000;

pub const MAX_BUFFERED_DEDICATED_JOBS: usize = 3;

/// Per-language OTEL tracing proxy configuration.
/// Default languages are configured in frontend instanceSettings.ts
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OtelTracingProxySettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub enabled_languages: HashSet<ScriptLang>,
}

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

#[derive(Deserialize, Clone)]
pub struct PowershellRepo {
    pub url: String,
    pub pat: String,
}

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

    /// Global setting for job isolation mode. 0=undefined (use env vars), 1=none, 2=unshare, 3=nsjail
    pub static ref JOB_ISOLATION: AtomicU8 = AtomicU8::new(JobIsolationLevel::Undefined as u8);

    pub static ref ENABLE_UNSHARE_PID: bool = std::env::var("ENABLE_UNSHARE_PID")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false);

    pub static ref FAVOR_UNSHARE_PID: bool = std::env::var("FAVOR_UNSHARE_PID")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false);

    pub static ref UNSHARE_TINI_PATH: String = {
        std::env::var("UNSHARE_TINI_PATH").unwrap_or_else(|_| "tini".to_string())
    };

    // --fork is required for unshare to work with --pid --mount-proc.
    // When tini is available, it runs as PID 1 inside the forked namespace for proper signal handling.
    pub static ref UNSHARE_ISOLATION_FLAGS: String = {
        std::env::var("UNSHARE_ISOLATION_FLAGS")
            .unwrap_or_else(|_| "--user --map-root-user --pid --fork --mount-proc".to_string())
    };

    // Check if tini is available for proper PID 1 handling in unshare namespaces.
    // tini handles OOM signals correctly, returning exit code 137 instead of sigprocmask errors.
    pub static ref TINI_AVAILABLE: Option<String> = {
        let tini_path = UNSHARE_TINI_PATH.as_str();
        let test_result = std::process::Command::new(tini_path)
            .args(["-s", "--", "true"])
            .output();

        match test_result {
            Ok(output) if output.status.success() => {
                tracing::info!("tini available at: {}", tini_path);
                Some(tini_path.to_string())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!(
                    "tini test failed: {}. Proceeding without tini (OOM exit codes may be incorrect).",
                    stderr.trim()
                );
                None
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    tracing::warn!(
                        "tini not found at '{}'. Install tini for correct OOM exit codes, or set UNSHARE_TINI_PATH.",
                        tini_path
                    );
                } else {
                    tracing::warn!(
                        "Failed to test tini: {}. Proceeding without tini.",
                        e
                    );
                }
                None
            }
        }
    };

    pub static ref UNSHARE_PATH: Option<String> = {
        let flags = UNSHARE_ISOLATION_FLAGS.as_str();
        let mut test_cmd_args: Vec<&str> = flags.split_whitespace().collect();

        // Build the test command based on whether tini is available
        // Note: --fork should already be in the flags for proper namespace setup
        if let Some(tini_path) = TINI_AVAILABLE.as_ref() {
            // Test with tini: unshare <flags> -- tini -s -- true
            test_cmd_args.push("--");
            test_cmd_args.push(tini_path.as_str());
            test_cmd_args.push("-s");
            test_cmd_args.push("--");
            test_cmd_args.push("true");
        } else {
            // Fallback without tini: unshare <flags> -- true
            test_cmd_args.push("--");
            test_cmd_args.push("true");
        }

        let test_result = std::process::Command::new("unshare")
            .args(&test_cmd_args)
            .output();

        match test_result {
            Ok(output) if output.status.success() => {
                if TINI_AVAILABLE.is_some() {
                    tracing::info!("PID namespace isolation enabled with tini. Flags: {}", flags);
                } else {
                    tracing::info!("PID namespace isolation enabled. Flags: {}", flags);
                }
                Some("unshare".to_string())
            },
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);

                if *ENABLE_UNSHARE_PID {
                    panic!(
                        "ENABLE_UNSHARE_PID is set but unshare test failed.\n\
                        Error: {}\n\
                        Flags: {}\n\
                        \n\
                        Solutions:\n\
                        • Check if user namespaces are enabled: 'sysctl kernel.unprivileged_userns_clone'\n\
                        • Check max user namespaces limit: 'cat /proc/sys/user/max_user_namespaces'\n\
                          (Some AMIs like Bottlerocket have max_user_namespaces=0 which disables user namespaces entirely)\n\
                        • For Docker: Requires 'privileged: true' in docker-compose for --mount-proc flag\n\
                        • For Kubernetes: Requires 'privileged: true' in securityContext for --mount-proc flag\n\
                        • Try different flags via UNSHARE_ISOLATION_FLAGS env var (remove --mount-proc if privileged mode not possible)\n\
                        • Alternative: Use NSJAIL instead\n\
                        • Disable: Set ENABLE_UNSHARE_PID=false (or disableUnsharePid=true in Helm chart)",
                        stderr.trim(),
                        flags
                    );
                }

                tracing::warn!(
                    "unshare test failed: {}. Flags: {}. Set ENABLE_UNSHARE_PID=true to fail on error.",
                    stderr.trim(),
                    flags
                );
                None
            },
            Err(e) => {
                if *ENABLE_UNSHARE_PID {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        panic!(
                            "ENABLE_UNSHARE_PID is set but unshare binary not found.\n\
                            Install util-linux package or set ENABLE_UNSHARE_PID=false"
                        );
                    } else {
                        panic!(
                            "ENABLE_UNSHARE_PID is set but failed to test unshare: {}",
                            e
                        );
                    }
                }

                if e.kind() == std::io::ErrorKind::NotFound {
                    tracing::debug!("unshare binary not found");
                } else {
                    tracing::warn!("Failed to test unshare: {}", e);
                }
                None
            }
        }
    };

    pub static ref NSJAIL_AVAILABLE: Option<String> = {
        let nsjail_path = NSJAIL_PATH.as_str();

        let test_result = std::process::Command::new(nsjail_path)
            .arg("--help")
            .output();

        match test_result {
            Ok(output) if output.status.success() => {
                tracing::info!("nsjail available at: {}", nsjail_path);
                Some(nsjail_path.to_string())
            },
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!(
                    "nsjail test failed: {}. \
                    nsjail should be included in all standard windmill images. \
                    Check that the nsjail binary is installed and working correctly.",
                    stderr.trim()
                );
                None
            },
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    tracing::info!(
                        "nsjail not found at '{}'. Sandboxing will not be available.",
                        nsjail_path
                    );
                } else {
                    tracing::warn!(
                        "Failed to test nsjail at '{}': {}.",
                        nsjail_path,
                        e
                    );
                }
                None
            }
        }
    };

    pub static ref KEEP_JOB_DIR: AtomicBool = AtomicBool::new(std::env::var("KEEP_JOB_DIR")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false));

    pub static ref NO_PROXY: Option<String> = std::env::var("no_proxy").ok().or(std::env::var("NO_PROXY").ok());
    pub static ref HTTP_PROXY: Option<String> = std::env::var("http_proxy").ok().or(std::env::var("HTTP_PROXY").ok());
    pub static ref HTTPS_PROXY: Option<String> = std::env::var("https_proxy").ok().or(std::env::var("HTTPS_PROXY").ok());

    /// Static proxy environment variables from env vars (for languages not using dynamic OTEL tracing proxy config)
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

    /// Per-language OTEL tracing proxy settings (configured via instance settings)
    pub static ref OTEL_TRACING_PROXY_SETTINGS: Arc<RwLock<OtelTracingProxySettings>> = Arc::new(RwLock::new(OtelTracingProxySettings::default()));
    pub static ref WHITELIST_ENVS: HashMap<String, String> = {
        windmill_common::worker::load_env_vars(
            windmill_common::worker::load_whitelist_env_vars_from_env(),
            &HashMap::new(),
        )
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
    pub static ref BUN_NO_CACHE: bool = std::env::var("BUN_NO_CACHE")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false);
    pub static ref NUGET_CONFIG: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref POWERSHELL_REPO_URL: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref POWERSHELL_REPO_PAT: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref MAVEN_REPOS: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref MAVEN_SETTINGS_XML: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref NO_DEFAULT_MAVEN: AtomicBool = AtomicBool::new(std::env::var("NO_DEFAULT_MAVEN")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false));
    pub static ref RUBY_REPOS: Arc<RwLock<Option<Vec<url::Url>>>> = Arc::new(RwLock::new(None));
    pub static ref CARGO_REGISTRIES: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));

    pub static ref PIP_EXTRA_INDEX_URL: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref PIP_INDEX_URL: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref UV_INDEX_STRATEGY: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
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

    pub static ref FLOW_RUNNER_RUNNING: Mutex<bool> = Mutex::new(false);
}

type Envs = Vec<(String, String)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum JobIsolationLevel {
    /// Not set via global setting; fall back to env vars (DISABLE_NSJAIL, FAVOR_UNSHARE_PID)
    Undefined = 0,
    /// No isolation
    None = 1,
    /// PID namespace isolation via unshare
    Unshare = 2,
    /// Full nsjail sandboxing
    NsjailSandboxing = 3,
}

impl JobIsolationLevel {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::None,
            2 => Self::Unshare,
            3 => Self::NsjailSandboxing,
            _ => Self::Undefined,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "none" => Self::None,
            "unshare" => Self::Unshare,
            "nsjail_sandboxing" => Self::NsjailSandboxing,
            _ => Self::Undefined,
        }
    }
}

pub fn get_job_isolation() -> JobIsolationLevel {
    JobIsolationLevel::from_u8(JOB_ISOLATION.load(Ordering::Relaxed))
}

/// Returns true if nsjail sandboxing should be used for job execution.
/// DISABLE_NSJAIL=false forces nsjail regardless of the global setting.
pub async fn read_ee_registry<T>(
    value: Option<T>,
    name: &str,
    job_id: &uuid::Uuid,
    w_id: &str,
    conn: &Connection,
) -> Option<T> {
    if !cfg!(feature = "enterprise") && value.is_some() {
        append_logs(
            job_id,
            w_id,
            format!("Private registry ({name}) configuration ignored: this feature requires Windmill Enterprise Edition\n"),
            conn,
        )
        .await;
        return None;
    }
    value
}

pub fn is_sandboxing_enabled() -> bool {
    if !*DISABLE_NSJAIL {
        return true;
    }
    match get_job_isolation() {
        JobIsolationLevel::NsjailSandboxing => true,
        _ => false,
    }
}

/// Returns true if unshare PID isolation should be used (when not using nsjail).
/// ENABLE_UNSHARE_PID forces unshare regardless of the global setting.
/// FAVOR_UNSHARE_PID uses unshare only when the global setting is not set.
pub fn is_unshare_enabled() -> bool {
    if *ENABLE_UNSHARE_PID {
        return true;
    }
    match get_job_isolation() {
        JobIsolationLevel::Unshare => true,
        JobIsolationLevel::Undefined => *FAVOR_UNSHARE_PID,
        _ => false,
    }
}

/// Check if OTEL tracing proxy is enabled for a specific language (EE only)
pub async fn is_otel_tracing_proxy_enabled_for_lang(lang: &ScriptLang) -> bool {
    cfg!(all(feature = "private", feature = "enterprise")) && {
        let settings = OTEL_TRACING_PROXY_SETTINGS.read().await;
        settings.enabled && settings.enabled_languages.contains(lang)
    }
}

/// Get proxy environment variables for job execution for a specific language.
/// When OTEL tracing proxy is enabled for this language, routes all traffic through the proxy.
/// Otherwise, uses the standard HTTP_PROXY/HTTPS_PROXY from environment.
pub async fn get_proxy_envs_for_lang(
    lang: &ScriptLang,
) -> anyhow::Result<Vec<(&'static str, String)>> {
    #[cfg(all(feature = "private", feature = "enterprise"))]
    if is_otel_tracing_proxy_enabled_for_lang(lang).await {
        return get_otel_tracing_proxy_envs().await;
    }
    let _ = lang;
    Ok(PROXY_ENVS.clone())
}

#[cfg(all(feature = "private", feature = "enterprise"))]
async fn get_otel_tracing_proxy_envs() -> anyhow::Result<Vec<(&'static str, String)>> {
    let port = crate::otel_tracing_proxy_ee::TRACING_PROXY_PORT
        .read()
        .await
        .ok_or_else(|| anyhow::anyhow!("OTEL tracing proxy port not initialized"))?;
    let proxy_url = format!("http://127.0.0.1:{}", port);
    Ok(vec![
        ("HTTP_PROXY", proxy_url.clone()),
        ("HTTPS_PROXY", proxy_url.clone()),
        // Lowercase variants for Ruby and other runtimes that check lowercase first
        ("http_proxy", proxy_url.clone()),
        ("https_proxy", proxy_url),
        ("NO_PROXY", "".to_string()),
        ("no_proxy", "".to_string()),
        // CA cert for various runtimes to trust the tracing proxy
        ("SSL_CERT_FILE", TRACING_PROXY_CA_CERT_PATH.to_string()),
        ("REQUESTS_CA_BUNDLE", TRACING_PROXY_CA_CERT_PATH.to_string()),
        (
            "NODE_EXTRA_CA_CERTS",
            TRACING_PROXY_CA_CERT_PATH.to_string(),
        ),
        ("CURL_CA_BUNDLE", TRACING_PROXY_CA_CERT_PATH.to_string()),
        ("DENO_CERT", TRACING_PROXY_CA_CERT_PATH.to_string()),
    ])
}

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
    Sql { job: PulledJob, flow_runners: Option<Arc<FlowRunners>> },
    Http(JobAndPerms),
}

impl NextJob {
    pub fn job(self) -> MiniPulledJob {
        match self {
            NextJob::Sql { job, .. } => job.job,
            NextJob::Http(job) => job.job,
        }
    }
}

impl std::ops::Deref for NextJob {
    type Target = MiniPulledJob;
    fn deref(&self) -> &Self::Target {
        match self {
            NextJob::Sql { job, .. } => &job.job,
            NextJob::Http(job) => &job.job,
        }
    }
}

//only matter if CLOUD_HOSTED
pub const MAX_RESULT_SIZE: usize = 1024 * 1024 * 2; // 2MB

#[derive(Clone)]
pub struct SameWorkerSender(pub Sender<SameWorkerPayload>, pub Arc<AtomicU16>);

#[allow(dead_code)]
#[derive(Clone)]
pub enum JobCompletedSender {
    Sql(SqlJobCompletedSender),
    Http(HttpClient),
    NeverUsed,
}

impl JobCompletedSender {
    pub fn is_sql(&self) -> bool {
        matches!(self, Self::Sql(_))
    }
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
                .send_async(SendResult {
                    result: SendResultPayload::JobCompleted(jc),
                    time: Instant::now(),
                })
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
        send_result: SendResultPayload,
        wait_for_capacity: bool,
    ) -> Result<(), flume::SendError<SendResult>> {
        match self {
            Self::Sql(SqlJobCompletedSender { sender, unbounded_sender, .. }) => {
                if wait_for_capacity {
                    sender
                        .send_async(SendResult { result: send_result, time: Instant::now() })
                        .await
                } else {
                    unbounded_sender
                        .send_async(SendResult { result: send_result, time: Instant::now() })
                        .await
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
        message: SameWorkerPayload,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<SameWorkerPayload>> {
        self.1.fetch_add(1, Ordering::Relaxed);
        self.0.send(message).await
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
        (NextJob::Sql { job, flow_runners, .. }, Connection::Sql(db)) => {
            JobAndPerms { flow_runners, ..job.get_job_and_perms(db).await }
        }
        (NextJob::Sql { .. }, Connection::Http(_)) => panic!("sql job on http connection"),
        (NextJob::Http(job), _) => job,
    }
}

pub fn create_span_with_name(
    arc_job: &MiniPulledJob,
    worker_name: &str,
    hostname: Option<&str>,
    span_name: &str,
) -> Span {
    // The span macro requires a literal, so we use a fixed name and set otel.name dynamically
    let span = tracing::span!(
        tracing::Level::INFO,
        "job",
        job_id = %arc_job.id,
        root_job = field::Empty,
        workspace_id = %arc_job.workspace_id,
        worker = %worker_name,
        hostname = field::Empty,
        tag = %arc_job.tag,
        language = field::Empty,
        script_path = field::Empty,
        flow_step_id = field::Empty,
        parent_job = field::Empty,
        otel.name = field::Empty
    );

    let rj = arc_job.flow_innermost_root_job.unwrap_or(arc_job.id);

    if let Some(lg) = arc_job.script_lang.as_ref() {
        span.record("language", lg.as_str());
    }
    if let Some(step_id) = arc_job.flow_step_id.as_ref() {
        span.record("otel.name", format!("{} {}", span_name, step_id).as_str());
        span.record("flow_step_id", step_id.as_str());
    } else {
        span.record("otel.name", span_name);
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
    if let Some(hostname) = hostname {
        span.record("hostname", hostname);
    }

    windmill_common::otel_oss::set_span_parent(&span, &rj);
    span
}

pub async fn handle_all_job_kind_error(
    conn: &Connection,
    authed_client: &AuthedClient,
    job: MiniCompletedJob,
    err: Error,
    same_worker_tx: Option<&SameWorkerSender>,
    worker_dir: &str,
    worker_name: &str,
    job_completed_tx: JobCompletedSender,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
    #[cfg(feature = "benchmark")] bench: &mut BenchmarkIter,
) {
    match conn {
        Connection::Sql(db) => {
            handle_job_error(
                db,
                authed_client,
                &job,
                0,
                None,
                err,
                false,
                same_worker_tx,
                &worker_dir,
                &worker_name,
                job_completed_tx.clone(),
                &killpill_rx,
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
                        job: job,
                        result: Arc::new(windmill_common::worker::to_raw_value(&error_to_value(
                            &err,
                        ))),
                        result_columns: None,
                        mem_peak: 0,
                        canceled_by: None,
                        success: false,
                        cached_res_path: None,
                        token: authed_client.token.clone(),
                        duration: None,
                        has_stream: Some(false),
                        from_cache: None,
                        flow_runners: None,
                        done_tx: None,
                    },
                    false,
                )
                .await
                .expect("send job completed");
        }
    }
}

fn start_interactive_worker_shell(
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
                tracing::info!("Received killpill, exiting worker shell");
                break;
            }

            let pulled_job = tokio::select! {
                _ = killpill_rx.recv() => {
                    tracing::info!("Received killpill during pull, exiting worker shell");
                    break;
                }
                result = async {
                    match &conn {
                        Connection::Sql(db) => {
                            let common_worker_prefix = retrieve_common_worker_prefix(&worker_name);
                            let query = ("".to_string(), make_pull_query(&[common_worker_prefix]));
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

                            use PulledJobResultToJobErr::*;
                            match job {
                                Ok(j) => match j.to_pulled_job() {
                                    Ok(j) => Ok(j
                                        .clone()
                                        .map(|job| NextJob::Sql { flow_runners: None, job })),
                                    Err(MissingConcurrencyKey(jc))
                                    | Err(ErrorWhilePreprocessing(jc)) => {
                                        if let Err(err) = job_completed_tx.send_job(jc, true).await {
                                            tracing::error!(
                                                "An error occurred while sending job completed: {:#?}",
                                                err
                                            )
                                        }
                                        Ok(None)
                                    }
                                },
                                Err(err) => Err(err),
                            }
                        }
                        Connection::Http(client) => {
                            crate::agent_workers::pull_job(&client, None, Some(true))
                                .await
                                .map_err(|e| error::Error::InternalErr(e.to_string()))
                                .map(|x| x.map(|y| NextJob::Http(y)))
                        }
                    }
                } => result,
            };

            match pulled_job {
                Ok(Some(job)) => {
                    tracing::debug!(target: VERBOSE_TARGET, worker = %worker_name, hostname = %hostname, "started handling of job {}", job.id);
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
                        flow_runners,
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
                        flow_runners,
                        #[cfg(feature = "benchmark")]
                        &mut bench,
                    )
                    .await;

                    last_executed_job = Some(Instant::now());
                }
                Ok(None) => {
                    let now = Instant::now();
                    let nap_time = match last_executed_job {
                        Some(last)
                            if now.duration_since(last).as_secs()
                                > TIMEOUT_TO_RESET_WORKER_SHELL_NAP_TIME_DURATION =>
                        {
                            Duration::from_secs(WORKER_SHELL_NAP_TIME_DURATION)
                        }
                        _ => Duration::from_millis(*SLEEP_QUEUE * 10),
                    };
                    tokio::select! {
                        _ = tokio::time::sleep(nap_time) => {
                        }
                        _ = killpill_rx.recv() => {
                            break;
                        }
                    }
                }

                Err(err) => {
                    tracing::error!(worker = %worker_name, hostname = %hostname, "Failed to pull jobs: {}", err);
                    tokio::time::sleep(Duration::from_millis(*SLEEP_QUEUE * 20)).await;
                }
            };
        }
    })
}

pub async fn create_job_dir(worker_directory: &str, job_id: impl Display) -> String {
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
    if is_sandboxing_enabled() {
        tracing::warn!(
            worker = %worker_name, hostname = %hostname,
            "NSJAIL to sandbox process in untrusted environments is an enterprise feature but allowed to be used for testing purposes"
        );
    }

    // Force UNSHARE_PATH initialization now to fail-fast if unshare doesn't work
    // This ensures we panic at startup rather than lazily when first accessed during job execution
    if is_unshare_enabled() || *ENABLE_UNSHARE_PID || *FAVOR_UNSHARE_PID {
        let _ = &*UNSHARE_PATH;
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
            if let Err(e) = PyV::from(PyVAlias::default())
                .try_get_python(&Uuid::nil(), &mut 0, &conn, &worker_name, "", &mut None)
                .await
            {
                tracing::error!(
                    worker = %worker_name,
                    hostname = %hostname,
                    worker_dir = %worker_dir,
                    "Cannot preinstall or find default version to worker: {e}"//
                );
            }
        });
    }

    if let Some(ref netrc) = *NETRC {
        tracing::info!(worker = %worker_name, hostname = %hostname, "Writing netrc at {}/.netrc", HOME_ENV.as_str());
        write_file(&HOME_ENV, ".netrc", netrc).expect("could not write netrc");
    }

    create_directory_async(&worker_dir).await;

    if is_sandboxing_enabled() {
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

    let is_dedicated_worker: bool = {
        let config = WORKER_CONFIG.read().await;
        config.dedicated_worker.is_some()
            || config
                .dedicated_workers
                .as_ref()
                .is_some_and(|dws| !dws.is_empty())
    };

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
    let last_processing_duration = Arc::new(AtomicU16::new(0));
    let job_completed_processor_is_done =
        Arc::new(AtomicBool::new(matches!(conn, Connection::Http(_))));

    // This is used to wake up the background processor when main loop is done and just waiting for new same workers jobs, and that bg processor is also not processing any jobs, bg processing can exit if no more same worker jobs
    let wake_up_notify = Arc::new(tokio::sync::Notify::new());
    let stats_map = JobStatsMap::default();

    let send_result = match (conn, job_completed_rx) {
        (Connection::Sql(db), Some(job_completed_receiver)) => Some(start_background_processor(
            job_completed_receiver,
            job_completed_tx.clone(),
            same_worker_queue_size.clone(),
            job_completed_processor_is_done.clone(),
            wake_up_notify.clone(),
            last_processing_duration.clone(),
            base_internal_url.to_string(),
            db.clone(),
            worker_dir.clone(),
            same_worker_tx.clone(),
            worker_name.clone(),
            killpill_tx.clone(),
            is_dedicated_worker,
            stats_map,
        )),
        _ => None,
    };

    // If we're the first worker to run, we start another background process that listens for a specific tag.
    // The tag itself is simply the worker’s common name (for example, wk-{worker_group}-{instance_name}).
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
    let mut infos = BenchmarkInfo::new(windmill_common::bench::shared_bench_iters());

    #[cfg(feature = "benchmark")]
    let mut bench_empty_queue_count: u64 = 0;

    #[cfg(feature = "benchmark")]
    if let Some(db) = conn.as_sql() {
        infos.init_pool_stats(db.size());
    }

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

    #[cfg(all(feature = "private", feature = "enterprise"))]
    let (dedicated_workers, dedicated_flow_paths, dedicated_handles): (
        HashMap<String, Sender<DedicatedWorkerJob>>,
        HashSet<String>,
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
        Connection::Http(_) => (HashMap::new(), HashSet::new(), vec![]),
    };

    #[cfg(any(not(feature = "private"), not(feature = "enterprise")))]
    let (dedicated_workers, dedicated_flow_paths, dedicated_handles): (
        HashMap<String, Sender<DedicatedWorkerJob>>,
        HashSet<String>,
        Vec<JoinHandle<()>>,
    ) = (HashMap::new(), HashSet::new(), vec![]);

    if i_worker == 1 {
        // Initialize runtime asset inserter for batched database inserts
        if let Connection::Sql(db) = conn {
            init_runtime_asset_loop(db.clone(), killpill_rx.resubscribe());
        }
        if let Err(e) = queue_init_bash_maybe(conn, same_worker_tx.clone(), &worker_name).await {
            killpill_tx.send();
            tracing::error!(worker = %worker_name, hostname = %hostname, "Error queuing init bash script for worker {worker_name}: {e:#}");
            return;
        }
        spawn_periodic_script_task(
            worker_name.clone(),
            conn.clone(),
            same_worker_tx.clone(),
            killpill_rx.resubscribe(),
        );
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
        let last_processing_duration_secs = last_processing_duration.load(Ordering::SeqCst);
        if last_processing_duration_secs > 5 {
            let sleep_duration = if last_processing_duration_secs > 10 {
                10
            } else {
                5
            };
            tracing::warn!(worker = %worker_name, hostname = %hostname, "last bg processor processing duration > {sleep_duration}s: {last_processing_duration_secs}s, throttling next job pull by {sleep_duration}s");
            last_processing_duration.store(0, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_secs(sleep_duration)).await;
            continue;
        }
        #[cfg(feature = "enterprise")]
        {
            let valid_key = *LICENSE_KEY_VALID.read().await;

            if !valid_key {
                tracing::error!(
                    worker = %worker_name, hostname = %hostname,
                    "Invalid license key, workers require a valid license key, sleeping for 10s waiting for valid key to be set"
                );
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(10)) => {
                        continue;
                    }
                    _ = killpill_rx.recv() => {
                        job_completed_tx
                            .kill()
                            .await
                            .expect("send kill to job completed tx");
                        tracing::info!(worker = %worker_name, hostname = %hostname, "killpill received while waiting for valid key, exiting");
                        break;
                    }
                }
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
        {
            let total_iters = infos
                .shared_iters
                .load(std::sync::atomic::Ordering::Relaxed);
            if benchmark_jobs > 0 && total_iters >= benchmark_jobs as u64 {
                tracing::info!(
                    "benchmark finished, exiting (total iters: {}, worker iters: {})",
                    total_iters,
                    infos.iters
                );
                job_completed_tx
                    .kill()
                    .await
                    .expect("send kill to job completed tx");
                killpill_tx.send();
                break;
            } else if benchmark_jobs > 0 && bench_empty_queue_count > 2000 {
                tracing::warn!(
                    "benchmark stalled: no jobs in queue for 2000 polls, exiting (total iters: {}, worker iters: {}/{})",
                    total_iters,
                    infos.iters,
                    benchmark_jobs
                );
                job_completed_tx
                    .kill()
                    .await
                    .expect("send kill to job completed tx");
                killpill_tx.send();
                break;
            } else if bench_empty_queue_count % 100 == 0 {
                if let Some(db) = conn.as_sql() {
                    let remaining = sqlx::query_as::<
                        _,
                        (uuid::Uuid, String, bool, Option<String>, Option<uuid::Uuid>),
                    >(
                        "SELECT q.id, q.tag, q.running, j.kind::text, j.parent_job
                         FROM v2_job_queue q JOIN v2_job j ON q.id = j.id
                         WHERE q.workspace_id = 'admins' LIMIT 10",
                    )
                    .fetch_all(db)
                    .await;
                    match remaining {
                        Ok(rows) => {
                            let total_remaining = sqlx::query_scalar::<_, i64>(
                                "SELECT COUNT(*) FROM v2_job_queue WHERE workspace_id = 'admins'",
                            )
                            .fetch_one(db)
                            .await
                            .unwrap_or(0);
                            for (id, tag, running, kind, parent) in &rows {
                                tracing::info!(
                                    "  pending job: id={id}, tag={tag}, running={running}, kind={}, parent={:?}",
                                    kind.as_deref().unwrap_or("?"), parent
                                );
                            }
                            tracing::info!(
                                "benchmark not finished (total: {}, worker: {}, queue: {})",
                                total_iters,
                                infos.iters,
                                total_remaining
                            );
                        }
                        Err(e) => {
                            tracing::info!("benchmark not finished (total: {}, worker: {}), queue query err: {e}", total_iters, infos.iters);
                        }
                    }
                }
            }
        }

        let next_job = {
            // println!("2: {:?}",  instant.elapsed());
            #[cfg(feature = "benchmark")]
            if !started {
                started = true
            }

            if let Ok(same_worker_job) = same_worker_rx.try_recv() {
                same_worker_queue_size.fetch_sub(1, Ordering::SeqCst);
                tracing::info!(
                    worker = %worker_name, hostname = %hostname,
                    "received {} from same worker channel",
                    same_worker_job.job_id
                );

                match &conn {
                    Connection::Sql(db) => {
                        let job = get_same_worker_job(db, &same_worker_job).await;
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
                            job.map(|x| {
                                x.map(|job| NextJob::Sql {
                                    flow_runners: same_worker_job.flow_runners,
                                    job,
                                })
                            })
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
            } else if match killpill_rx.try_recv() {
                Ok(_) | Err(broadcast::error::TryRecvError::Closed) => true,
                _ => false,
            } {
                if !killed_but_draining_same_worker_jobs {
                    killed_but_draining_same_worker_jobs = true;
                    if job_completed_tx.is_sql() {
                        tracing::info!(worker = %worker_name, hostname = %hostname, "killpill received in worker main loop, sending killpill job");
                        job_completed_tx
                            .kill()
                            .await
                            .expect("send kill to job completed tx");
                    }
                }
                continue;
            } else if killed_but_draining_same_worker_jobs {
                if job_completed_processor_is_done.load(Ordering::SeqCst) {
                    tracing::info!(worker = %worker_name, hostname = %hostname, "all running jobs have completed and all completed jobs have been fully processed, exiting");
                    break;
                } else {
                    wake_up_notify.notify_one();
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
                        let mut job = match timeout(
                            Duration::from_secs(30),
                            pull(
                                &db,
                                suspend_first,
                                &worker_name,
                                None,
                                #[cfg(feature = "benchmark")]
                                &mut bench,
                            )
                            .warn_after_seconds(2),
                        )
                        .await
                        {
                            Ok(job) => job,
                            Err(e) => {
                                tracing::error!(worker = %worker_name, hostname = %hostname, "pull timed out after 20s, sleeping for 30s: {e:?}");
                                tokio::time::sleep(Duration::from_secs(30)).await;
                                continue;
                            }
                        };

                        // Preprocess pulled job result
                        if let Ok(ref mut pulled_job_res) = job {
                            if let Err(e) = timeout(
                                // Will fail if longer than 10 seconds
                                core::time::Duration::from_secs(10),
                                pulled_job_res.maybe_apply_debouncing(db),
                            )
                            .warn_after_seconds(2)
                            .await
                            // Flatten result
                            .map_err(error::Error::from)
                            .and_then(|r| r)
                            {
                                pulled_job_res.error_while_preprocessing = Some(e.to_string());
                            }
                        }

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
                        match job {
                            Ok(pulled_job_result) => match pulled_job_result.to_pulled_job() {
                                Ok(j) => Ok(j.map(|job| NextJob::Sql { flow_runners: None, job })),
                                Err(PulledJobResultToJobErr::MissingConcurrencyKey(jc))
                                | Err(PulledJobResultToJobErr::ErrorWhilePreprocessing(jc)) => {
                                    if let Err(err) = job_completed_tx.send_job(jc, true).await {
                                        tracing::error!(
                                            "An error occurred while sending job completed: {:#?}",
                                            err
                                        )
                                    }
                                    Ok(None)
                                }
                            },
                            Err(err) => Err(err),
                        }
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
                #[cfg(feature = "benchmark")]
                {
                    bench_empty_queue_count = 0;
                }
                #[cfg(feature = "benchmark")]
                let is_top_level_job = job.parent_job.is_none() && !job.kind.is_flow();
                #[cfg(feature = "benchmark")]
                let bench_job_id = job.id;

                #[cfg(feature = "prometheus")]
                if let Some(wb) = worker_busy.as_ref() {
                    wb.set(1);
                    tracing::debug!("set worker busy to 1");
                }

                occupancy_metrics.running_job_started_at = Some(Instant::now());

                last_executed_job = None;
                jobs_executed += 1;

                tracing::debug!(target: VERBOSE_TARGET, worker = %worker_name, hostname = %hostname, "started handling of job {}", job.id);

                if matches!(
                    job.kind,
                    JobKind::Script | JobKind::Preview | JobKind::FlowScript
                ) {
                    if !dedicated_workers.is_empty() {
                        // Try flow path + step_id combinations for flow jobs, otherwise use runnable_path
                        let dedicated_worker_tx = if let Some(step_id) = job.flow_step_id.as_ref() {
                            dedicated_flow_paths.iter().find_map(|flow_path| {
                                let key = format!("{}:{}", flow_path, step_id);
                                dedicated_workers.get(&key)
                            })
                        } else {
                            job.runnable_path
                                .as_ref()
                                .and_then(|path| dedicated_workers.get(path))
                        };
                        if let Some(dedicated_worker_tx) = dedicated_worker_tx {
                            let dedicated_job = DedicatedWorkerJob {
                                job: Arc::new(job.job()),
                                flow_runners: None,
                                done_tx: None,
                            };
                            if let Err(e) = dedicated_worker_tx.send(dedicated_job).await {
                                tracing::info!("failed to send jobs to dedicated workers. Likely dedicated worker has been shut down. This is normal: {e:?}");
                            }

                            #[cfg(feature = "benchmark")]
                            {
                                add_time!(bench, "sent to dedicated worker");
                                if let Some(db) = conn.as_sql() {
                                    infos.sample_pool(db.size(), db.num_idle() as u32);
                                }
                                infos.add_iter(bench, bench_job_id, is_top_level_job);
                            }

                            continue;
                        }
                    }

                    // Extract flow_runners early to use in both dedicated workers and flow runners
                    let flow_runners = match &job {
                        NextJob::Sql { flow_runners, .. } => flow_runners.clone(),
                        NextJob::Http(_) => None,
                    };

                    if let Some(flow_runners) = flow_runners {
                        let key_o = job.flow_step_id.as_ref().map(|x| x.to_string());
                        if let Some(key) = key_o {
                            if let Some(flow_runner_tx) = flow_runners.runners.get(&key) {
                                tracing::info!(
                                    "sending job {} to flow runner step {}",
                                    job.id,
                                    key
                                );
                                let (done_tx, done_rx) = tokio::sync::oneshot::channel::<()>();
                                let flow_runners = flow_runners.clone();

                                let job = job.job();

                                let dedicated_job = DedicatedWorkerJob {
                                    job: Arc::new(job.clone()),
                                    flow_runners: Some(flow_runners),
                                    done_tx: Some(done_tx),
                                };

                                if let Err(e) = flow_runner_tx.send(dedicated_job).await {
                                    let token = match &conn {
                                        Connection::Sql(db) => {
                                            windmill_queue::jobs::create_token(db, &job, None).await
                                        }
                                        _ => "".to_string(),
                                    };
                                    handle_all_job_kind_error(
                                        &conn,
                                        &AuthedClient::new(
                                            base_internal_url.to_owned(),
                                            job.workspace_id.clone(),
                                            token,
                                            None,
                                        ),
                                        MiniCompletedJob::from(job),
                                        error::Error::InternalErr(format!(
                                            "failed to send jobs to flow runners: {e:?}"
                                        )),
                                        Some(&same_worker_tx),
                                        &worker_dir,
                                        &worker_name,
                                        job_completed_tx.clone(),
                                        &killpill_rx,
                                        #[cfg(feature = "benchmark")]
                                        &mut bench,
                                    )
                                    .await;
                                } else {
                                    if let Err(err) = done_rx.await {
                                        tracing::error!("Flow runner done channel has been dropped without being received: {err:?}");
                                    }
                                }

                                #[cfg(feature = "benchmark")]
                                {
                                    add_time!(bench, "sent to flow runner");
                                    if let Some(db) = conn.as_sql() {
                                        infos.sample_pool(db.size(), db.num_idle() as u32);
                                    }
                                    infos.add_iter(bench, bench_job_id, is_top_level_job);
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
                                job: MiniCompletedJob::from(job.job()),
                                success: true,
                                result: Arc::new(empty_result()),
                                result_columns: None,
                                mem_peak: 0,
                                cached_res_path: None,
                                token: "".to_string(),
                                canceled_by: None,
                                duration: None,
                                has_stream: Some(false),
                                from_cache: None,
                                flow_runners: None,
                                done_tx: None,
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

                            #[cfg(windows)]
                            {
                                // On Windows, try symlink_dir
                                let windows_target = target.replace("/", "\\");
                                let windows_parent = parent_shared_dir.replace("/", "\\");

                                match symlink_dir(&windows_parent, &windows_target).await {
                                    Ok(_) => {
                                        tracing::info!(
                                            "Successfully created directory symlink on Windows"
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to create symlink_dir on Windows (likely needs admin privileges or Developer Mode): {}", e);
                                        create_directory_async(&target).await;
                                    }
                                }
                            }

                            #[cfg(not(windows))]
                            {
                                symlink(&parent_shared_dir, &target)
                                    .await
                                    .expect("could not symlink target");
                            }
                        }
                    } else {
                        create_directory_async(target).await;
                    }

                    #[cfg(feature = "prometheus")]
                    let tag = job.tag.clone();

                    let is_init_script: bool = job.tag.as_str() == INIT_SCRIPT_TAG;
                    let is_periodic_bash_script: bool = job.tag.as_str() == PERIODIC_SCRIPT_TAG;
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
                        flow_runners,
                    } = extract_job_and_perms(job, &conn).await;

                    let authed_client = AuthedClient::new(
                        base_internal_url.to_owned(),
                        job.workspace_id.clone(),
                        token,
                        None,
                    );

                    let arc_job = Arc::new(job);

                    let span = create_span_with_name(&arc_job, &worker_name, Some(hostname), "job");

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
                        flow_runners,
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
                        Ok(false) if is_periodic_bash_script => {
                            tracing::error!(
                                "periodic script job failed. Check logs for job ID {} for details.",
                                job_id
                            );

                            if let Connection::Sql(db) = conn {
                                report_critical_error(
                                    format!(
                                        "Periodic script job {} returned false (failed). Check logs for job ID {} for details.",
                                        job_id, job_id
                                    ),
                                    db.clone(),
                                    Some(&arc_job.workspace_id),
                                    Some("periodic_script_job_failed"),
                                )
                                .await;
                            }
                        }
                        Err(err) => {
                            if is_periodic_bash_script {
                                tracing::error!("periodic script job failed");

                                // Report critical error for periodic script failures
                                if let Connection::Sql(db) = conn {
                                    report_critical_error(
                                        format!(
                                            "Periodic script job {} failed in worker {}: {}",
                                            job_id, worker_name, &err
                                        ),
                                        db.clone(),
                                        Some(&arc_job.workspace_id),
                                        Some("periodic_script_job_failed"),
                                    )
                                    .await;
                                }
                            }
                            handle_all_job_kind_error(
                                &conn,
                                &authed_client,
                                MiniCompletedJob::from(arc_job),
                                err,
                                Some(&same_worker_tx),
                                &worker_dir,
                                &worker_name,
                                job_completed_tx.clone(),
                                &killpill_rx,
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
                        if let Some(db) = conn.as_sql() {
                            infos.sample_pool(db.size(), db.num_idle() as u32);
                        }
                        infos.add_iter(bench, bench_job_id, is_top_level_job);
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
                    bench_empty_queue_count += 1;
                    add_time!(bench, "sleep because empty job queue");
                    if let Some(db) = conn.as_sql() {
                        infos.sample_pool(db.size(), db.num_idle() as u32);
                    }
                    infos.add_iter(bench, uuid::Uuid::nil(), false);
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

    #[cfg(feature = "enterprise")]
    {
        let valid_key = *LICENSE_KEY_VALID.read().await;

        if !valid_key {
            tracing::info!(worker = %worker_name, hostname = %hostname, "Invalid license key, exiting immediately");
            return;
        }
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

    #[cfg(feature = "benchmark")]
    {
        infos
            .write_to_file("profiling_main.json")
            .expect("write to file profiling");

        if let Some(db) = conn.as_sql() {
            benchmark_verify(benchmark_jobs, db).await;
        }
    }
    tracing::info!(worker = %worker_name, hostname = %hostname, "waiting for interactive_shell to finish");
    if let Some(interactive_shell) = interactive_shell {
        match tokio::time::timeout(Duration::from_secs(10), interactive_shell).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                tracing::error!("error in interactive_shell process: {e:?}")
            }
            Err(_) => {
                tracing::error!("timed out awaiting interactive_shell process")
            }
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
            .send(SameWorkerPayload { job_id: uuid, recoverable: false, flow_runners: None })
            .await
            .map_err(to_anyhow)?;
        tracing::info!("Creating initial job {uuid} from initial script script: {content}");
        Ok(true)
    } else {
        Ok(false)
    }
}

fn spawn_periodic_script_task(
    worker_name: String,
    conn: Connection,
    same_worker_tx: SameWorkerSender,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    tokio::spawn(async move {
        let config = WORKER_CONFIG.read().await;

        match (
            &config.periodic_script_bash,
            &config.periodic_script_interval_seconds,
        ) {
            (Some(_), None) => {
                tracing::error!(
                    worker = %worker_name,
                    "periodic_script_bash is set but periodic_script_interval_seconds is not set. Both must be configured together."
                );
                return;
            }
            (None, Some(_)) => {
                tracing::error!(
                    worker = %worker_name,
                    "periodic_script_interval_seconds is set but periodic_script_bash is not set. Both must be configured together."
                );
                return;
            }
            (Some(content), Some(interval_seconds)) => {
                let interval_seconds = *interval_seconds;
                if interval_seconds < MIN_PERIODIC_SCRIPT_INTERVAL_SECONDS {
                    tracing::error!(
                        worker = %worker_name,
                        "Periodic script interval {} seconds is below minimum of {} seconds. Periodic script task will not start.",
                        interval_seconds,
                        MIN_PERIODIC_SCRIPT_INTERVAL_SECONDS
                    );
                    return;
                }

                let content = content.clone();
                let interval_duration = Duration::from_secs(interval_seconds);

                tracing::info!(
                    worker = %worker_name,
                    "Starting periodic script task (interval: {}s)",
                    interval_seconds
                );

                loop {
                    tracing::info!(
                        worker = %worker_name,
                        "Triggering periodic script execution"
                    );

                    match queue_periodic_script_bash_maybe(
                        &conn,
                        same_worker_tx.clone(),
                        &worker_name,
                        &content,
                    )
                    .await
                    {
                        Ok(_) => {
                            tracing::debug!(
                                worker = %worker_name,
                                "Successfully queued periodic script"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                worker = %worker_name,
                                "Error queuing periodic script: {e:#}"
                            );
                        }
                    }

                    tokio::select! {
                        _ = killpill_rx.recv() => {
                            tracing::info!("Periodic init script task shutting down for worker {}", worker_name);
                            break;
                        }
                        _ = tokio::time::sleep(interval_duration) => {
                        }
                    }
                }
            }
            (None, None) => {
                tracing::debug!(
                    worker = %worker_name,
                    "No periodic script configured"
                );
            }
        }
    });
}

async fn queue_periodic_script_bash_maybe<'c>(
    conn: &Connection,
    same_worker_tx: SameWorkerSender,
    worker_name: &str,
    content: &str,
) -> anyhow::Result<()> {
    let uuid = match conn {
        Connection::Sql(db) => push_periodic_bash_job(db, content.to_owned(), worker_name).await?,
        Connection::Http(client) => queue_periodic_job(client, &content).await?,
    };

    same_worker_tx
        .send(SameWorkerPayload { job_id: uuid, recoverable: false, flow_runners: None })
        .await
        .map_err(to_anyhow)?;
    tracing::info!("Creating periodic script job {uuid} from periodic script: {content}");
    Ok(())
}

pub struct SendResult {
    pub result: SendResultPayload,
    pub time: Instant,
}

pub enum SendResultPayload {
    JobCompleted(JobCompleted),
    UpdateFlow(UpdateFlow),
}

#[derive(Debug, Clone)]
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
    has_stream: &mut bool,
) -> windmill_common::error::Result<Box<RawValue>> {
    let args = build_args_map(job, client, conn).await?.map(Json);
    let job_args = if args.is_some() {
        args.as_ref()
    } else {
        job.args.as_ref()
    };

    let stream_notifier = StreamNotifier::new(conn, job);

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
        stream_notifier,
        has_stream,
    )
    .await?)
}

lazy_static::lazy_static! {
    static ref LOG_TAG_NAME: String = std::env::var("LOG_TAG_NAME").unwrap_or("tag".to_string());
}

#[derive(Deserialize, Serialize, Default)]
pub struct PreviousResult<'a> {
    #[serde(borrow)]
    pub previous_result: Option<&'a RawValue>,
}

/// Detects and stores runtime assets from job arguments.
/// This function is called when a job starts executing to track which assets
/// are passed as inputs to the job at runtime.
async fn detect_and_store_runtime_assets_from_job_args(
    workspace_id: &str,
    job_id: &Uuid,
    Json(args_map): &Json<HashMap<String, Box<RawValue>>>,
    job_kind: &JobKind,
) {
    match job_kind {
        JobKind::Script_Hub | JobKind::Script | JobKind::Flow => {}
        _ => return,
    }

    let runtime_assets =
        windmill_common::runtime_assets::extract_runtime_assets_from_args(args_map);
    if runtime_assets.is_empty() {
        return;
    }

    // Store each detected runtime asset
    for asset in runtime_assets {
        let asset = windmill_common::runtime_assets::InsertRuntimeAssetParams {
            workspace_id: workspace_id.to_string(),
            job_id: *job_id,
            asset_path: asset.path,
            asset_kind: asset.kind,
            access_type: None,
            created_at: None,
            columns: None,
        };
        register_runtime_asset(asset);
    }
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
    flow_runners: Option<Arc<FlowRunners>>,
    #[cfg(feature = "benchmark")] _bench: &mut BenchmarkIter,
) -> windmill_common::error::Result<bool> {
    if job.canceled_by.is_some() {
        return Err(Error::JsonErr(canceled_job_to_result(&job)));
    }
    if let Some(e) = &job.pre_run_error {
        return Err(Error::ExecutionErr(e.to_string()));
    }

    match job.kind {
        JobKind::UnassignedScript | JobKind::UnassignedFlow | JobKind::UnassignedSinglestepFlow => {
            return Err(Error::ExecutionErr("Suspended job was not handled by the user within 30 days, job will not be executed.".to_string()));
        }
        _ => {}
    }

    if NATIVE_MODE_RESOLVED.load(std::sync::atomic::Ordering::Relaxed) {
        if let Some(lang) = &job.script_lang {
            if !lang.is_native() {
                return Err(Error::ExecutionErr(format!(
                    "Worker is in native mode and cannot execute non-native job with language '{}'",
                    lang.as_str(),
                )));
            }
        }
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
            | JobKind::FlowDependencies
            | JobKind::SingleStepFlow,
            x,
        ) => {
            if x.map(|x| x.0).is_none_or(|x| is_special_codebase_hash(x)) {
                Some(
                    cache::job::fetch_preview(conn, &job.id, raw_lock, raw_code, raw_flow.clone())
                        .await?,
                )
            } else {
                None
            }
        }
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
                            job: MiniCompletedJob::from(job),
                            result,
                            result_columns: None,
                            mem_peak: 0,
                            canceled_by: None,
                            success: true,
                            cached_res_path: None,
                            token: client.token.clone(),
                            duration: None,
                            has_stream: Some(false),
                            from_cache: Some(true),
                            flow_runners: None,
                            done_tx: None,
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
                _ => cache::job::fetch_flow(db, &job.kind, job.runnable_id).await?,
            };
            match Box::pin(handle_flow(
                job,
                &flow_data,
                db,
                &client,
                None,
                &same_worker_tx.expect(SAME_WORKER_REQUIREMENTS),
                worker_dir,
                job_completed_tx.clone(),
                worker_name,
                flow_runners,
                &killpill_rx,
            ))
            .warn_after_seconds(10)
            .await
            {
                Err(err) if err.downcast_ref::<SchedulePushZombieError>().is_some() => {
                    tracing::error!(
                        "Schedule push zombie: {err}. Leaving flow job in queue for zombie detection to restart."
                    );
                    Ok(true)
                }
                other => {
                    other?;
                    Ok(true)
                }
            }
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

        let isolation_label = if is_sandboxing_enabled() {
            "nsjail"
        } else if is_unshare_enabled() {
            "unshare"
        } else {
            "none"
        };

        logs.push_str(&format!(
            "job={} {}={} worker={} hostname={} isolation={}\n",
            &job.id, *LOG_TAG_NAME, &job.tag, &worker_name, &hostname, isolation_label
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

        if *QUIET_MODE {
            logs.push_str("Quiet mode enabled: verbose service logs are suppressed\n");
        }

        #[cfg(not(feature = "enterprise"))]
        if let Connection::Sql(db) = conn {
            if (job.concurrent_limit.is_some()
                || windmill_common::runnable_settings::prefetch_cached_from_handle(
                    job.runnable_settings_handle,
                    db,
                )
                .await?
                .1
                .concurrent_limit
                .is_some())
                && !job.kind.is_dependency()
            {
                logs.push_str("---\n");
                logs.push_str("WARNING: This job has concurrency limits enabled. Concurrency limits are an EE feature and the setting is ignored.\n");
                logs.push_str("---\n");
            }
        }

        // Only used for testing in tests/relative_imports.rs
        // Give us some space to work with.
        #[cfg(debug_assertions)]
        if let Some(dbg_djob_sleep) = job
            .args
            .as_ref()
            .map(|x| {
                x.get("dbg_djob_sleep")
                    .map(|v| serde_json::from_str::<u32>(v.get()).ok())
                    .flatten()
            })
            .flatten()
        {
            tracing::debug!("Debug: {} going to sleep for {}", job.id, dbg_djob_sleep);
            tokio::time::sleep(std::time::Duration::from_secs(dbg_djob_sleep as u64)).await;
        }

        tracing::debug!(
            workspace_id = %job.workspace_id,
            "handling job {}",
            job.id
        );
        append_logs(&job.id, &job.workspace_id, logs, conn).await;

        // Extract and store runtime assets from job arguments
        if let (Connection::Sql(_), Some(args_json)) = (conn, &job.args) {
            detect_and_store_runtime_assets_from_job_args(
                &job.workspace_id,
                &job.id,
                args_json,
                &job.kind,
            )
            .await;
        }

        let mut column_order: Option<Vec<String>> = None;
        let mut new_args: Option<HashMap<String, Box<RawValue>>> = None;
        let mut has_stream = false;

        let raw_workspace_dependencies_o = if job.kind.is_dependency() {
            job.args
                .as_ref()
                .and_then(|x| x.get("raw_workspace_dependencies"))
                .map(|v| v.get())
                .and_then(|v| serde_json::from_str::<RawWorkspaceDependencies>(v).ok())
        } else {
            None
        };
        // Box::pin all async branches to prevent large match enum on stack
        let result = match job.kind {
            JobKind::Dependencies => match conn {
                Connection::Sql(db) => {
                    Box::pin(handle_dependency_job(
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
                        raw_workspace_dependencies_o,
                    ))
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
                    Box::pin(handle_flow_dependency_job(
                        (*job).clone(),
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
                        raw_workspace_dependencies_o,
                    ))
                    .await
                }
                Connection::Http(_) => {
                    return Err(Error::internal_err(
                        "Could not handle flow dependency job with agent worker".to_string(),
                    ));
                }
            },
            JobKind::AppDependencies => match conn {
                Connection::Sql(db) => Box::pin(handle_app_dependency_job(
                    (*job).clone(),
                    &mut mem_peak,
                    &mut canceled_by,
                    job_dir,
                    db,
                    worker_name,
                    worker_dir,
                    base_internal_url,
                    &client.token,
                    occupancy_metrics,
                    raw_workspace_dependencies_o,
                ))
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
            JobKind::AIAgent => match conn {
                Connection::Sql(db) => {
                    Box::pin(handle_ai_agent_job(
                        conn,
                        db,
                        job.as_ref(),
                        &client,
                        &mut canceled_by,
                        &mut mem_peak,
                        &mut *occupancy_metrics,
                        &job_completed_tx,
                        worker_dir,
                        base_internal_url,
                        worker_name,
                        hostname,
                        killpill_rx,
                        &mut has_stream,
                    ))
                    .await
                }
                Connection::Http(_) => {
                    return Err(Error::internal_err(
                        "Agent worker does not support ai agent jobs".to_string(),
                    ));
                }
            },
            _ => {
                let metric_timer = Instant::now();
                let preview_data = preview_data.and_then(|data| match data {
                    RawData::Script(data) => Some(data),
                    _ => None,
                });

                // Set job context for OTEL tracing before entering handle_code_execution_job's span
                #[cfg(all(feature = "private", feature = "enterprise"))]
                if matches!(
                    job.script_lang,
                    Some(ScriptLang::Nativets) | Some(ScriptLang::Bunnative)
                ) && is_otel_tracing_proxy_enabled_for_lang(&ScriptLang::Nativets).await
                {
                    crate::otel_tracing_proxy_ee::set_current_job_context(job.id).await;
                }

                // Box::pin to move large future to heap
                let r = Box::pin(handle_code_execution_job(
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
                    &mut has_stream,
                ))
                .await;

                occupancy_metrics.total_duration_of_running_jobs +=
                    metric_timer.elapsed().as_secs_f32();
                r
            }
        };

        let cjob = MiniCompletedJob::from(job.to_owned());
        drop(job);
        //it's a test job, no need to update the db
        if cjob.workspace_id == "" {
            return Ok(true);
        }

        if result
            .as_ref()
            .is_err_and(|err| matches!(err, &Error::AlreadyCompleted(_)))
        {
            return Ok(false);
        }
        process_result(
            cjob,
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
            has_stream,
            flow_runners,
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
        envs.insert(k.to_string(), v.clone());
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
                JobKind::SingleStepFlow => 6,
                JobKind::Dependencies => 7,
                JobKind::Flow => 8,
                JobKind::FlowPreview => 9,
                JobKind::Identity => 10,
                JobKind::FlowDependencies => 11,
                JobKind::AppDependencies => 12,
                JobKind::Noop => 13,
                JobKind::FlowNode => 14,
                JobKind::AIAgent => 15,
                JobKind::UnassignedScript => 16,
                JobKind::UnassignedFlow => 17,
                JobKind::UnassignedSinglestepFlow => 18,
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
    has_stream: &mut bool,
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

    // Box::pin the script fetching match to prevent large enum on stack
    let (
        ScriptData { code, lock },
        ScriptMetadata { language, envs, codebase, schema_validator, schema },
    ) = match job.kind {
        JobKind::Preview => {
            let codebase = job
                .runnable_id
                .and_then(|x| hash_to_codebase_id(&job.id.to_string(), x.0));
            if codebase.is_none() && job.runnable_id.is_some() {
                (arc_data, arc_metadata) =
                    Box::pin(cache::script::fetch(conn, job.runnable_id.unwrap())).await?;
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
                Box::pin(get_hub_script_content_and_requirements(
                    job.runnable_path.as_ref(),
                    conn.as_sql(),
                ))
                .await?;

            data = ScriptData { code: content, lock: lockfile };
            metadata = ScriptMetadata { language, envs, codebase, schema, schema_validator: None };
            (&data, &metadata)
        }
        JobKind::Script => {
            (arc_data, arc_metadata) = Box::pin(cache::script::fetch(conn, script_hash()?)).await?;
            (arc_data.as_ref(), arc_metadata.as_ref())
        }
        JobKind::FlowScript => {
            arc_data = Box::pin(cache::flow::fetch_script(
                conn,
                FlowNodeId(script_hash()?.0),
            ))
            .await?;
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
            arc_data = Box::pin(cache::app::fetch_script(
                conn,
                AppScriptId(script_hash()?.0),
            ))
            .await?;
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
                        Box::pin(get_hub_script_content_and_requirements(
                            Some(script_path),
                            conn.as_sql(),
                        ))
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

                    (arc_data, arc_metadata) =
                        Box::pin(cache::script::fetch(conn, ScriptHash(hash))).await?;
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
    run_language_executor(
        job,
        conn,
        client,
        parent_runnable_path,
        job_dir,
        worker_dir,
        mem_peak,
        canceled_by,
        base_internal_url,
        worker_name,
        column_order,
        new_args,
        occupancy_metrics,
        killpill_rx,
        precomputed_agent_info,
        has_stream,
        language,
        code,
        envs,
        codebase,
        lock,
        false,
    )
    .await
}

pub async fn run_language_executor(
    job: &MiniPulledJob,
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
    has_stream: &mut bool,
    language: Option<ScriptLang>,
    code: &String,
    envs: &Option<Vec<String>>,
    codebase: &Option<String>,
    lock: &Option<String>,
    run_inline: bool,
) -> error::Result<Box<RawValue>> {
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
            parent_runnable_path,
            run_inline,
        )
        .await;
    } else if language == Some(ScriptLang::Mysql) {
        #[cfg(not(feature = "mysql"))]
        return Err(Error::internal_err(
            "MySQL requires the mysql feature to be enabled".to_string(),
        ));

        #[cfg(feature = "mysql")]
        {
            if run_inline {
                return Err(Error::internal_err(
                    "Inline execution is not yet supported for this language".to_string(),
                ));
            }
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
                parent_runnable_path,
            )
            .await;
        }
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
            if run_inline {
                return Err(Error::internal_err(
                    "Inline execution is not yet supported for this language".to_string(),
                ));
            }
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
                parent_runnable_path,
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
            if run_inline {
                return Err(Error::internal_err(
                    "Inline execution is not yet supported for this language".to_string(),
                ));
            }
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
                parent_runnable_path,
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
            if run_inline {
                return Err(Error::internal_err(
                    "Inline execution is not yet supported for this language".to_string(),
                ));
            }
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
                parent_runnable_path,
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
            if run_inline {
                return Err(Error::internal_err(
                    "Inline execution is not yet supported for this language".to_string(),
                ));
            }
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
                parent_runnable_path,
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
                parent_runnable_path,
                run_inline,
            )
            .await;
        }
    } else if language == Some(ScriptLang::Graphql) {
        if run_inline {
            return Err(Error::internal_err(
                "Inline execution is not yet supported for this language".to_string(),
            ));
        }
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
        if run_inline {
            return Err(Error::internal_err(
                "Inline execution is not yet supported for this language".to_string(),
            ));
        }
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
            has_stream,
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

    let Some(language) = language else {
        return Err(Error::ExecutionErr(
            "Require language to be not null".to_string(),
        ))?;
    };

    /// Resolves MaybeLock for languages that need workspace dependencies prefetching.
    /// Only call this for Bun, Bunnative, Go, and Php.
    async fn resolve_maybe_lock(
        lock: &Option<String>,
        code: &str,
        language: ScriptLang,
        workspace_id: &str,
        runnable_path: &str,
        conn: Connection,
    ) -> error::Result<MaybeLock> {
        if let Some(lock) = lock.clone() {
            Ok(MaybeLock::Resolved { lock })
        } else {
            Ok(MaybeLock::Unresolved {
                workspace_dependencies: WorkspaceDependenciesPrefetched::extract(
                    code,
                    language,
                    workspace_id,
                    // TODO: implement
                    &None,
                    runnable_path,
                    conn,
                )
                .await?,
            })
        }
    }

    // Box::pin all language handlers to prevent large match enum on stack
    let result: error::Result<Box<RawValue>> = match language {
        ScriptLang::Python3 => {
            #[cfg(not(feature = "python"))]
            return Err(Error::internal_err(
                "Python requires the python feature to be enabled".to_string(),
            ));

            #[cfg(feature = "python")]
            {
                if run_inline {
                    return Err(Error::internal_err(
                        "Inline execution is not yet supported for this language".to_string(),
                    ));
                }
                Box::pin(handle_python_job(
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
                    has_stream,
                ))
                .await
            }
        }
        ScriptLang::Deno => {
            if run_inline {
                return Err(Error::internal_err(
                    "Inline execution is not yet supported for this language".to_string(),
                ));
            }
            Box::pin(handle_deno_job(
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
                has_stream,
            ))
            .await
        }
        ScriptLang::Bun | ScriptLang::Bunnative => {
            if run_inline {
                return Err(Error::internal_err(
                    "Inline execution is not yet supported for this language".to_string(),
                ));
            }
            let maybe_lock = resolve_maybe_lock(
                &lock,
                &code,
                language,
                &job.workspace_id,
                job.runnable_path(),
                conn.clone(),
            )
            .await?;
            Box::pin(handle_bun_job(
                maybe_lock,
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
                has_stream,
            ))
            .await
        }
        ScriptLang::Go => {
            if run_inline {
                return Err(Error::internal_err(
                    "Inline execution is not yet supported for this language".to_string(),
                ));
            }
            let maybe_lock = resolve_maybe_lock(
                &lock,
                &code,
                language,
                &job.workspace_id,
                job.runnable_path(),
                conn.clone(),
            )
            .await?;
            Box::pin(handle_go_job(
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
                maybe_lock,
            ))
            .await
        }
        ScriptLang::Bash => {
            if run_inline {
                return Err(Error::internal_err(
                    "Inline execution is not yet supported for this language".to_string(),
                ));
            }
            Box::pin(handle_bash_job(
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
            ))
            .await
        }
        ScriptLang::Powershell => {
            if run_inline {
                return Err(Error::internal_err(
                    "Inline execution is not yet supported for this language".to_string(),
                ));
            }
            Box::pin(handle_powershell_job(
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
            ))
            .await
        }
        ScriptLang::Php => {
            #[cfg(not(feature = "php"))]
            return Err(Error::internal_err(
                "PHP requires the php feature to be enabled".to_string(),
            ));

            #[cfg(feature = "php")]
            {
                if run_inline {
                    return Err(Error::internal_err(
                        "Inline execution is not yet supported for this language".to_string(),
                    ));
                }
                let maybe_lock = resolve_maybe_lock(
                    &lock,
                    &code,
                    language,
                    &job.workspace_id,
                    job.runnable_path(),
                    conn.clone(),
                )
                .await?;
                Box::pin(handle_php_job(
                    maybe_lock,
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
                ))
                .await
            }
        }
        ScriptLang::Rust => {
            #[cfg(not(feature = "rust"))]
            return Err(Error::internal_err(
                "Rust requires the rust feature to be enabled".to_string(),
            ));

            #[cfg(feature = "rust")]
            {
                if run_inline {
                    return Err(Error::internal_err(
                        "Inline execution is not yet supported for this language".to_string(),
                    ));
                }
                Box::pin(handle_rust_job(
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
                ))
                .await
            }
        }
        ScriptLang::Ansible => {
            #[cfg(not(feature = "python"))]
            return Err(Error::internal_err(
                "Ansible requires the python feature to be enabled".to_string(),
            ));

            #[cfg(feature = "python")]
            {
                if run_inline {
                    return Err(Error::internal_err(
                        "Inline execution is not yet supported for this language".to_string(),
                    ));
                }
                Box::pin(handle_ansible_job(
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
                ))
                .await
            }
        }
        ScriptLang::CSharp => {
            if run_inline {
                return Err(Error::internal_err(
                    "Inline execution is not yet supported for this language".to_string(),
                ));
            }
            Box::pin(handle_csharp_job(
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
            ))
            .await
        }
        ScriptLang::Nu => {
            #[cfg(not(feature = "nu"))]
            return Err(
                anyhow::anyhow!("Nu is not available because the feature is not enabled").into(),
            );

            #[cfg(feature = "nu")]
            {
                if run_inline {
                    return Err(Error::internal_err(
                        "Inline execution is not yet supported for this language".to_string(),
                    ));
                }
                Box::pin(handle_nu_job(JobHandlerInputNu {
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
                }))
                .await
            }
        }
        ScriptLang::Java => {
            #[cfg(not(feature = "java"))]
            return Err(anyhow::anyhow!(
                "Java is not available because the feature is not enabled"
            )
            .into());

            #[cfg(feature = "java")]
            {
                if run_inline {
                    return Err(Error::internal_err(
                        "Inline execution is not yet supported for this language".to_string(),
                    ));
                }
                Box::pin(handle_java_job(JobHandlerInputJava {
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
                }))
                .await
            }
        }
        ScriptLang::Ruby => {
            #[cfg(not(feature = "ruby"))]
            return Err(anyhow::anyhow!(
                "Ruby is not available because the feature is not enabled"
            )
            .into());

            #[cfg(feature = "ruby")]
            {
                if run_inline {
                    return Err(Error::internal_err(
                        "Inline execution is not yet supported for this language".to_string(),
                    ));
                }
                Box::pin(handle_ruby_job(JobHandlerInputRuby {
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
                }))
                .await
            }
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

pub fn parse_sig_of_lang(
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

pub fn init_worker_internal_server_inline_utils(
    killpill_rx: tokio::sync::broadcast::Receiver<()>,
    base_internal_url: String,
) -> windmill_common::error::Result<()> {
    let utils = WorkerInternalServerInlineUtils {
        base_internal_url,
        killpill_rx: Arc::new(killpill_rx),
        run_inline_preview_script: Arc::new(|params| {
            let job = MiniPulledJob {
                workspace_id: params.workspace_id,
                id: Uuid::new_v4(),
                args: params.args.map(Json),
                parent_job: None,
                created_by: params.created_by,
                scheduled_for: chrono::Utc::now(),
                started_at: None,
                runnable_path: None,
                kind: JobKind::Preview,
                runnable_id: None,
                canceled_reason: None,
                canceled_by: None,
                permissioned_as: params.permissioned_as,
                permissioned_as_email: params.permissioned_as_email,
                flow_status: None,
                tag: "inline_preview".to_string(),
                script_lang: Some(params.lang),
                same_worker: true,
                pre_run_error: None,
                flow_innermost_root_job: None,
                root_job: None,
                timeout: None,
                flow_step_id: None,
                cache_ttl: None,
                cache_ignore_s3_path: None,
                priority: None,
                preprocessed: None,
                script_entrypoint_override: None,
                trigger: None,
                trigger_kind: None,
                visible_to_owner: false,
                permissioned_as_end_user_email: None,
                runnable_settings_handle: None,
                concurrent_limit: None,
                concurrency_time_window_s: None,
            };
            Box::pin(async move {
                let mut mem_peak: i32 = -1;
                let mut canceled_by: Option<CanceledBy> = None;
                let mut column_order: Option<Vec<String>> = None;
                let mut new_args: Option<HashMap<String, Box<RawValue>>> = None;
                let mut occupancy_metrics = OccupancyMetrics::new(Instant::now());
                let mut has_stream: bool = false;
                let mut killpill_rx = params.killpill_rx;

                run_language_executor(
                    &job,
                    &params.conn,
                    &params.client,
                    None,
                    &params.job_dir,
                    &params.worker_dir,
                    &mut mem_peak,
                    &mut canceled_by,
                    &params.base_internal_url,
                    &params.worker_name,
                    &mut column_order,
                    &mut new_args,
                    &mut occupancy_metrics,
                    &mut killpill_rx,
                    None,
                    &mut has_stream,
                    Some(params.lang),
                    &params.content,
                    &None,
                    &None,
                    &None,
                    true,
                )
                .await
            })
        }),
    };
    WORKER_INTERNAL_SERVER_INLINE_UTILS
        .set(utils)
        .map_err(|_| {
            error::Error::InternalErr(
                "Couldn't set WorkerInternalServerInlineUtils OnceCell".to_string(),
            )
        })?;
    Ok(())
}

pub fn get_worker_internal_server_inline_utils(
) -> windmill_common::error::Result<&'static WorkerInternalServerInlineUtils> {
    match WORKER_INTERNAL_SERVER_INLINE_UTILS.get() {
        Some(utils) => Ok(utils),
        None => Err(error::Error::internal_err(
            "worker inline functions are meant to be called from a worker's internal server",
        )),
    }
}
