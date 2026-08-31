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
// Re-export proxy env-var snapshots so callers (including EE modules)
// can keep importing them via `crate::{NO_PROXY, HTTP_PROXY, HTTPS_PROXY}`.
use windmill_common::client::AuthedClient;
use windmill_common::db::UserDbWithAuthed;
use windmill_common::get_latest_deployed_hash_for_path;
use windmill_common::jobs::InlineScriptTarget;
use windmill_common::jobs::RunInlineScriptFnParams;
use windmill_common::jobs::WorkerInternalServerInlineUtils;
use windmill_common::jobs::WORKER_INTERNAL_SERVER_INLINE_UTILS;
use windmill_common::otel_oss::{
    otel_incr_worker_execution_count, otel_incr_worker_started,
    otel_record_worker_execution_duration, otel_record_worker_pull_duration, otel_set_worker_busy,
    otel_set_worker_uptime,
};
use windmill_common::runtime_assets::init_runtime_asset_loop;
use windmill_common::runtime_assets::register_runtime_asset;
use windmill_common::scripts::hash_to_codebase_id;
use windmill_common::scripts::is_special_codebase_hash;
use windmill_common::scripts::ScriptModule;
use windmill_common::utils::report_critical_error;
use windmill_common::utils::retrieve_common_worker_prefix;
use windmill_common::worker::error_to_value;
use windmill_common::workspace_dependencies::RawWorkspaceDependencies;
use windmill_common::workspace_dependencies::WorkspaceDependenciesPrefetched;
use windmill_common::{
    agent_workers::DECODED_AGENT_TOKEN,
    apps::AppScriptId,
    cache::{future::FutureCachedExt, ScriptData, ScriptMetadata},
    external_ip::cached_ip,
    schema::{should_validate_schema, SchemaValidator},
    utils::{create_directory_async, WarnAfterExt},
    worker::{
        is_allowed_file_location, make_pull_query, write_file, Connection, HttpClient,
        EXIT_AFTER_N_JOBS, MIN_PERIODIC_SCRIPT_INTERVAL_SECONDS, ROOT_CACHE_DIR,
        ROOT_CACHE_NOMOUNT_DIR, WINDMILL_DIR,
    },
    worker_group_job_stats::JobStatsMap,
    KillpillSender,
};
pub use windmill_common::{HTTPS_PROXY, HTTP_PROXY, NO_PROXY};

#[cfg(feature = "enterprise")]
use windmill_common::ee_oss::LICENSE_KEY_VALID;

use anyhow::Result;
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
    PrecomputedAgentInfo, PulledJob, SameWorkerPayload, HTTP_CLIENT, INIT_SCRIPT_PATH_PREFIX,
    INIT_SCRIPT_TAG, PERIODIC_SCRIPT_PATH_PREFIX, PERIODIC_SCRIPT_TAG,
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
        get_reserved_variables, get_root_job_id, update_worker_ping_for_failed_init_script,
        OccupancyMetrics,
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
    worker_flow::handle_flow,
    worker_lockfiles::{
        handle_app_dependency_job, handle_dependency_job, handle_flow_dependency_job,
        tally_unfinished_dependency_deploy,
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

#[cfg(feature = "rlang")]
use crate::r_executor::{handle_r_job, JobHandlerInput as JobHandlerInputRlang};

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

#[cfg(feature = "snowflake")]
use crate::snowflake_executor::do_snowflake;

#[cfg(all(feature = "enterprise", feature = "mssql"))]
use crate::mssql_executor::do_mssql;

#[cfg(feature = "bigquery")]
use crate::bigquery_executor::do_bigquery;

#[cfg(feature = "benchmark")]
use windmill_common::bench::{benchmark_init, benchmark_verify, BenchmarkInfo, BenchmarkIter};

use windmill_common::add_time;

lazy_static::lazy_static! {
    pub static ref PY310_CACHE_DIR: String = format!("{}python_3_10", *ROOT_CACHE_DIR);
    pub static ref PY311_CACHE_DIR: String = format!("{}python_3_11", *ROOT_CACHE_DIR);
    pub static ref PY312_CACHE_DIR: String = format!("{}python_3_12", *ROOT_CACHE_DIR);
    pub static ref PY313_CACHE_DIR: String = format!("{}python_3_13", *ROOT_CACHE_DIR);

    pub static ref TAR_JAVA_CACHE_DIR: String = format!("{}tar/java", *ROOT_CACHE_DIR);

    pub static ref UV_CACHE_DIR: String = format!("{}uv", *ROOT_CACHE_DIR);
    pub static ref PY_INSTALL_DIR: String = format!("{}py_runtime", *ROOT_CACHE_DIR);
    pub static ref TAR_PYBASE_CACHE_DIR: String = format!("{}tar", *ROOT_CACHE_DIR);
    pub static ref DENO_CACHE_DIR: String = format!("{}deno", *ROOT_CACHE_DIR);
    pub static ref DENO_CACHE_DIR_DEPS: String = format!("{}deno/deps", *ROOT_CACHE_DIR);
    pub static ref DENO_CACHE_DIR_NPM: String = format!("{}deno/npm", *ROOT_CACHE_DIR);

    pub static ref GO_CACHE_DIR: String = format!("{}go", *ROOT_CACHE_DIR);
    pub static ref RUST_CACHE_DIR: String = format!("{}rust", *ROOT_CACHE_DIR);
    pub static ref NU_CACHE_DIR: String = format!("{}nu", *ROOT_CACHE_DIR);
    pub static ref CSHARP_CACHE_DIR: String = format!("{}csharp", *ROOT_CACHE_DIR);

    // Java
    pub static ref JAVA_CACHE_DIR: String = format!("{}java", *ROOT_CACHE_DIR);
    pub static ref COURSIER_CACHE_DIR: String = format!("{}/coursier-cache", *JAVA_CACHE_DIR);
    pub static ref JAVA_REPOSITORY_DIR: String = format!("{}/repository", *JAVA_CACHE_DIR);
    pub static ref JAVA_HOME_DIR: String = format!("{}/home", *JAVA_CACHE_DIR);

    // Ruby
    pub static ref RUBY_CACHE_DIR: String = format!("{}ruby", *ROOT_CACHE_DIR);

    // R
    pub static ref R_CACHE_DIR: String = format!("{}rlang", *ROOT_CACHE_DIR);

    // for related places search: ADD_NEW_LANG
    pub static ref BUN_CACHE_DIR: String = format!("{}bun", *ROOT_CACHE_NOMOUNT_DIR);
    pub static ref BUN_BUNDLE_CACHE_DIR: String = format!("{}bun", *ROOT_CACHE_DIR);
    pub static ref BUN_CODEBASE_BUNDLE_CACHE_DIR: String = format!("{}script_bundle", *ROOT_CACHE_NOMOUNT_DIR);

    pub static ref GO_BIN_CACHE_DIR: String = format!("{}gobin", *ROOT_CACHE_DIR);
    pub static ref POWERSHELL_CACHE_DIR: String = format!("{}powershell", *ROOT_CACHE_DIR);
    pub static ref COMPOSER_CACHE_DIR: String = format!("{}composer", *ROOT_CACHE_DIR);

    pub static ref TRACING_PROXY_CA_CERT_PATH: String =
        format!("{}tracing_proxy_ca.pem", *ROOT_CACHE_NOMOUNT_DIR);
}

const NUM_SECS_PING: u64 = 5;
const NUM_SECS_READINGS: u64 = 60;

const INCLUDE_DEPS_PY_SH_CONTENT: &str = include_str!("../nsjail/download_deps.py.sh");

const WORKER_SHELL_NAP_TIME_DURATION: u64 = 15;
const WORKER_SHELL_INITIAL_NAP_TIME_DURATION: u64 = 5;
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
    #[serde(default)]
    pub no_proxy_hosts: Option<String>,
    /// Comma-separated host/IP patterns for which the MITM proxy skips upstream TLS
    /// verification. Unlike `no_proxy_hosts` (which bypasses the proxy entirely, so the
    /// request goes untraced), these hosts stay traced — only the proxy's own upstream
    /// certificate check is disabled. Same suffix-matching semantics as `no_proxy_hosts`.
    #[serde(default)]
    pub insecure_upstream_hosts: Option<String>,
    /// Extra CA certificates (PEM bundle) added to the MITM proxy's upstream trust store,
    /// on top of the system roots. Lets the proxy verify internal endpoints signed by a
    /// private CA without disabling verification.
    #[serde(default)]
    pub upstream_ca_certs: Option<String>,
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

    static ref SLEEP_QUEUE_BASE: u64 = std::env::var("SLEEP_QUEUE")
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

    /// Per-language override for the nsjail `rlimit_as` (virtual address space) cap.
    /// Value is in MiB, or `unlimited`/`none`/`inf`/`0` to uncap. Unset keeps the
    /// historical default baked into the proto. See `render_nsjail_rlimit_as`.
    pub static ref NSJAIL_PY_RLIMIT_AS_MB: Option<String> =
        std::env::var("NSJAIL_PY_RLIMIT_AS_MB").ok();
    pub static ref NSJAIL_ANSIBLE_RLIMIT_AS_MB: Option<String> =
        std::env::var("NSJAIL_ANSIBLE_RLIMIT_AS_MB").ok();
    pub static ref NSJAIL_DBT_RLIMIT_AS_MB: Option<String> =
        std::env::var("NSJAIL_DBT_RLIMIT_AS_MB").ok();

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

                tracing::error!(
                    "unshare test command failed (exit code: {}). stderr: '{}'. flags: '{}'. \
                    Unshare isolation will NOT be available. \
                    If job_isolation is set to 'unshare' in Instance Settings, jobs will run without isolation. \
                    Common causes: user namespaces disabled (sysctl kernel.unprivileged_userns_clone=0), \
                    max_user_namespaces=0, or missing privileges (--mount-proc requires privileged mode).",
                    output.status,
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
                    tracing::error!(
                        "unshare binary not found in PATH. Unshare isolation will NOT be available. \
                        Install the util-linux package to enable unshare isolation."
                    );
                } else {
                    tracing::error!(
                        "Failed to execute unshare test command: {}. Unshare isolation will NOT be available.",
                        e
                    );
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
                tracing::error!(
                    "nsjail test failed (exit code: {}). stderr: '{}'. path: '{}'. \
                    Nsjail sandboxing will NOT be available. \
                    nsjail should be included in all standard windmill images. \
                    If job_isolation is set to 'nsjail_sandboxing' in Instance Settings, jobs will fail.",
                    output.status,
                    stderr.trim(),
                    nsjail_path
                );
                None
            },
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    tracing::error!(
                        "nsjail not found at '{}'. Nsjail sandboxing will NOT be available. \
                        If using a custom image, ensure nsjail is installed.",
                        nsjail_path
                    );
                } else {
                    tracing::error!(
                        "Failed to execute nsjail test at '{}': {}. Nsjail sandboxing will NOT be available.",
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

    /// Static proxy environment variables from env vars (for languages not using dynamic OTEL tracing proxy config).
    /// The underlying `NO_PROXY` / `HTTP_PROXY` / `HTTPS_PROXY` snapshots live in `windmill_common`
    /// so other crates (e.g. native triggers) can reuse the same source of truth.
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
    pub static ref HOME_ENV: String = {
        #[cfg(not(windows))]
        { std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()) }
        #[cfg(windows)]
        {
            std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().to_string())
        }
    };
    pub static ref GIT_PATH: String = std::env::var("GIT_PATH").unwrap_or_else(|_| "/usr/bin/git".to_string());

    pub static ref NODE_PATH: Option<String> = std::env::var("NODE_PATH").ok();

    pub static ref TZ_ENV: String = std::env::var("TZ").unwrap_or_else(|_| String::new());
    pub static ref GOPRIVATE: Option<String> = std::env::var("GOPRIVATE").ok();
    pub static ref GOPROXY: Option<String> = std::env::var("GOPROXY").ok();
    pub static ref NETRC: Option<String> = std::env::var("NETRC").ok();


    pub static ref NPM_CONFIG_REGISTRY: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref BUNFIG_INSTALL_SCOPES: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref NPMRC: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
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

    pub static ref WORKSPACE_REGISTRIES: Arc<RwLock<Option<WorkspaceRegistryMap>>> = Arc::new(RwLock::new(None));

    pub static ref PIP_EXTRA_INDEX_URL: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref PIP_INDEX_URL: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref UV_INDEX_STRATEGY: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
    pub static ref UV_EXCLUDE_NEWER: Arc<RwLock<Option<i64>>> = Arc::new(RwLock::new(None));
    pub static ref BUN_INSTALL_MIN_RELEASE_AGE: Arc<RwLock<Option<i64>>> = Arc::new(RwLock::new(None));
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

lazy_static::lazy_static! {
    /// Registry TLS/timeout settings for uv. Env-only (they have no instance setting), and read
    /// both by the job path and by the DB-less `prepare-deps` CLI, which has no other source of
    /// registry configuration.
    pub static ref TRUSTED_HOST: Option<String> = non_empty_env("PY_TRUSTED_HOST").or_else(|| non_empty_env("PIP_TRUSTED_HOST"));
    pub static ref INDEX_CERT: Option<String> = non_empty_env("PY_INDEX_CERT").or_else(|| non_empty_env("PIP_INDEX_CERT"));
    pub static ref NATIVE_CERT: bool = non_empty_env("PY_NATIVE_CERT").or_else(|| non_empty_env("UV_NATIVE_TLS")).map(|flag| flag == "true").unwrap_or(false);
    /// uv's HTTP request timeout (seconds). The uv invocations use env_clear(), so a
    /// UV_HTTP_TIMEOUT set on the worker is dropped unless forwarded explicitly.
    /// Only forwarded when set; otherwise uv keeps its own default. Lets operators
    /// raise it for slow/contended private registries ("operation timed out").
    pub static ref UV_HTTP_TIMEOUT: Option<String> = non_empty_env("UV_HTTP_TIMEOUT");
}

/// A variable declared but left empty (a common shape in compose/k8s manifests) must not
/// shadow the fallback name it is checked against.
pub(crate) fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

lazy_static::lazy_static! {
    /// Optional override for the size of the `/tmp` tmpfs mount in nsjail sandboxes (in megabytes).
    /// When `None` (or non-positive), executors fall back to the unified
    /// `DEFAULT_NSJAIL_TMPFS_SIZE_BYTES` (800MB).
    pub static ref NSJAIL_TMPFS_SIZE_MB: Arc<RwLock<Option<i64>>> = Arc::new(RwLock::new(None));

    /// Selects how `/tmp` is backed inside nsjail sandboxes. `Some("disk")`
    /// switches to a bind mount on `{JOB_DIR}/jail_tmp` (disk-backed); any
    /// other value (including `None` or `Some("tmpfs")`) keeps the historical
    /// RAM-backed tmpfs sized by `nsjail_tmpfs_size_mb`.
    pub static ref NSJAIL_TMP_BACKING: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));

    /// Reject a `# sandbox <image>` whose compressed download size exceeds this many
    /// MB, before download. `None`/non-positive = no limit. (`sandbox_image_max_size_mb`.)
    pub static ref SANDBOX_IMAGE_MAX_SIZE_MB: Arc<RwLock<Option<i64>>> = Arc::new(RwLock::new(None));

    /// Best-effort cap (MB) on the worker's cached rootfs tars; oldest evicted after a
    /// run when exceeded. `None`/non-positive = unbounded. (`sandbox_image_cache_max_mb`.)
    pub static ref SANDBOX_IMAGE_CACHE_MAX_MB: Arc<RwLock<Option<i64>>> = Arc::new(RwLock::new(None));

    /// Sandbox image pull policy (`missing`/`newer`/`always`/`never`). `None`/unrecognized
    /// falls back to `newer`. (`sandbox_image_pull_policy`.)
    pub static ref SANDBOX_IMAGE_PULL_POLICY: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));

    /// If set, unqualified sandbox image refs (e.g. `alpine`) are pulled from this
    /// registry instead of docker.io. Fully-qualified refs are unaffected.
    /// (`sandbox_image_default_registry`.)
    pub static ref SANDBOX_IMAGE_DEFAULT_REGISTRY: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));

    /// Optional docker `auth.json` blob for private registries, written to a per-job
    /// `DOCKER_CONFIG` dir for crane. (`sandbox_registry_auth`.)
    pub static ref SANDBOX_REGISTRY_AUTH: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));

    /// Optional mirror URL for `uv python install`. Wires to the `UV_PYTHON_INSTALL_MIRROR`
    /// env var when forwarded to uv. Can be set via the `UV_PYTHON_INSTALL_MIRROR` env var
    /// or the `uv_python_install_mirror` instance setting.
    pub static ref UV_PYTHON_INSTALL_MIRROR: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
}

pub fn sleep_queue() -> u64 {
    if NATIVE_MODE_RESOLVED.load(std::sync::atomic::Ordering::Relaxed) {
        300
    } else {
        *SLEEP_QUEUE_BASE
    }
}

pub type WorkspaceRegistryMap =
    std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>;

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

/// Like `read_ee_registry`, but first checks for a workspace-specific override.
/// If the workspace has an override for `setting_key`, that value is used instead of `global_value`.
pub async fn read_ee_registry_with_workspace_override(
    global_value: Option<String>,
    setting_key: &str,
    display_name: &str,
    job_id: &uuid::Uuid,
    w_id: &str,
    conn: &Connection,
) -> Option<String> {
    let ws_value = {
        let registries = WORKSPACE_REGISTRIES.read().await;
        registries
            .as_ref()
            .and_then(|m| m.get(w_id))
            .and_then(|ws| ws.get(setting_key))
            .and_then(|v| match v {
                serde_json::Value::String(s) => Some(s.clone()),
                _ => None,
            })
    };
    // An empty/whitespace-only value (from either source) means "unset" — a
    // workspace override of `""` still takes precedence over the global
    // value, but neither triggers the spurious "requires Enterprise" warning
    // on CE jobs.
    let value = ws_value.or(global_value).filter(|s| !s.trim().is_empty());
    read_ee_registry(value, display_name, job_id, w_id, conn).await
}

/// Like `read_ee_registry_with_workspace_override`, but for `bool` values.
pub async fn read_ee_registry_bool_with_workspace_override(
    global_value: bool,
    setting_key: &str,
    display_name: &str,
    job_id: &uuid::Uuid,
    w_id: &str,
    conn: &Connection,
) -> bool {
    let ws_value = {
        let registries = WORKSPACE_REGISTRIES.read().await;
        registries
            .as_ref()
            .and_then(|m| m.get(w_id))
            .and_then(|ws| ws.get(setting_key))
            .and_then(|v| v.as_bool())
    };
    let value = ws_value.unwrap_or(global_value);
    if !cfg!(feature = "enterprise") && value {
        append_logs(
            job_id,
            w_id,
            format!("Private registry ({display_name}) configuration ignored: this feature requires Windmill Enterprise Edition\n"),
            conn,
        )
        .await;
        return false;
    }
    value
}

/// Like `read_ee_registry_with_workspace_override`, but for `Vec<url::Url>` values (e.g. ruby_repos).
pub async fn read_ee_registry_url_list_with_workspace_override(
    global_value: Option<Vec<url::Url>>,
    setting_key: &str,
    display_name: &str,
    job_id: &uuid::Uuid,
    w_id: &str,
    conn: &Connection,
) -> Option<Vec<url::Url>> {
    let ws_value = {
        let registries = WORKSPACE_REGISTRIES.read().await;
        registries
            .as_ref()
            .and_then(|m| m.get(w_id))
            .and_then(|ws| ws.get(setting_key))
            .and_then(|v| match v {
                serde_json::Value::String(s) => {
                    let urls: Vec<url::Url> = s
                        .split(|c: char| c == ',' || c.is_whitespace())
                        .filter(|s| !s.is_empty())
                        .filter_map(|s| url::Url::parse(s).ok())
                        .collect();
                    if urls.is_empty() {
                        None
                    } else {
                        Some(urls)
                    }
                }
                _ => None,
            })
    };
    let value = ws_value.or(global_value);
    read_ee_registry(value, display_name, job_id, w_id, conn).await
}

/// Returns a cache key suffix for workspace-specific registry overrides.
/// If the workspace has any registry overrides, returns `":ws:<w_id>"` to namespace
/// the resolution cache. Otherwise returns empty string.
///
/// Called on every job's cache lookup path. When no workspace overrides are
/// configured (the common case), this returns `""` with zero allocation —
/// the RwLock read is uncontended and costs only nanoseconds.
pub async fn workspace_registry_cache_suffix(w_id: &str) -> String {
    let registries = WORKSPACE_REGISTRIES.read().await;
    let has_overrides = registries
        .as_ref()
        .and_then(|m| m.get(w_id))
        .map_or(false, |ws| !ws.is_empty());
    if has_overrides {
        format!(":ws:{w_id}")
    } else {
        String::new()
    }
}

/// The name a build artifact is cached under, derived from `base` — the runnable's own
/// cache-key input — and its inline modules.
///
/// `write_module_files` puts module content in the job dir where the build inlines it into
/// the artifact, so a name without it serves one runnable's modules to another whose main
/// content and lockfile match — across workspaces, the cache being global.
///
/// Only the path and content may name the artifact, because they are all the build reads.
/// `ScriptModule::lock` especially must stay out: deploy regenerates it *after* the parent
/// has prebuilt, so naming it would strand every prebuilt artifact.
pub(crate) fn artifact_cache_name(
    base: String,
    modules: Option<&std::collections::HashMap<String, ScriptModule>>,
) -> String {
    let Some(modules) = modules.filter(|m| !m.is_empty()) else {
        // Byte-identical to the name a module-free runnable had before modules entered this,
        // so its cached artifacts stay reachable. A pre-fix multi-file runnable also stored
        // here, so one of those stays reachable too — accepted over invalidating every cache,
        // and once this ships nothing can be stored here with module content in it again.
        return windmill_common::utils::calculate_hash(&base);
    };
    let mut entries: Vec<(&String, &ScriptModule)> = modules.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    // `base` ends in caller-supplied bytes (a preview brings its own lockfile), so it is
    // sealed to a fixed width before the module block is appended — raw, a crafted lockfile
    // could spell out another runnable's block and reach its slot.
    let mut keyed = format!(
        "{}:modules:{}",
        windmill_common::utils::calculate_hash(&base),
        entries.len()
    );
    for (path, module) in entries {
        // Both length-prefixed, else `{"a": "bc"}` and `{"ab": "c"}` encode alike.
        keyed.push_str(&format!(
            ":{}:{path}:{}:{}",
            path.len(),
            module.content.len(),
            module.content,
        ));
    }
    // Its own namespace: `calculate_hash` emits hex, so however a module-free runnable
    // crafts its content and lockfile it can never land on a module-bearing name.
    format!("mod-{}", windmill_common::utils::calculate_hash(&keyed))
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

/// Strict check that a string is a well-formed W3C `traceparent`
/// (`version-traceid-spanid-flags`, lowercase hex, non-zero ids, version != ff).
/// Used before forwarding an inbound header value verbatim to a job subprocess,
/// so we don't hand downstream OTel parsers something they'll reject.
#[cfg(all(feature = "private", feature = "enterprise"))]
fn valid_w3c_traceparent(tp: &str) -> bool {
    let p: Vec<&str> = tp.split('-').collect();
    p.len() == 4
        && p[0].len() == 2
        && p[1].len() == 32
        && p[2].len() == 16
        && p[3].len() == 2
        // version "ff" is reserved/invalid per the W3C spec
        && p[0] != "ff"
        && p[1] != "00000000000000000000000000000000"
        && p[2] != "0000000000000000"
        // W3C mandates lowercase hex
        && p
            .iter()
            .all(|s| s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
}

/// Get OTEL trace context environment variables for a job (TRACEPARENT, OTEL_TRACE_ID, OTEL_SPAN_ID).
/// Returns an empty vec when OTEL tracing is not enabled or on non-enterprise builds.
///
/// When the request that enqueued the job carried a valid inbound `traceparent`
/// (propagated via the job's [`LogContext`](windmill_common::log_context::LogContext)),
/// it is forwarded verbatim so the script's spans join the originating
/// distributed trace. Otherwise the trace context is derived from the job UUID.
pub fn get_otel_context_envs(job_id: &uuid::Uuid) -> Vec<(&'static str, String)> {
    #[cfg(all(feature = "private", feature = "enterprise"))]
    if windmill_common::OTEL_TRACING_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        let inbound = windmill_common::log_context::current_log_context()
            .and_then(|c| c.inbound_traceparent.clone())
            .filter(|tp| valid_w3c_traceparent(tp));
        let (traceparent, trace_id, span_id) = if let Some(tp) = inbound {
            let trace_id = tp[3..35].to_string();
            let span_id = tp[36..52].to_string();
            (tp, trace_id, span_id)
        } else {
            let trace_id = format!("{:032x}", job_id.as_u128());
            let span_id = format!("{:016x}", job_id.as_u64_pair().1);
            (format!("00-{}-{}-01", trace_id, span_id), trace_id, span_id)
        };
        return vec![
            ("TRACEPARENT", traceparent),
            ("OTEL_TRACE_ID", trace_id),
            ("OTEL_SPAN_ID", span_id),
        ];
    }
    let _ = job_id;
    vec![]
}

/// Get proxy environment variables for job execution for a specific language.
/// When OTEL tracing proxy is enabled for this language, routes all traffic through the proxy.
/// Otherwise, uses the standard HTTP_PROXY/HTTPS_PROXY from environment.
///
/// Deployment callback jobs (git sync) always bypass the MITM tracing proxy and use the
/// stock corporate proxy. Routing git's HTTPS through the local MITM breaks TLS for
/// GitHub/GitLab in chained-upstream-proxy setups, and we don't need HTTP spans for the
/// system git sync script anyway.
pub async fn get_proxy_envs_for_lang(
    lang: &ScriptLang,
    job_kind: JobKind,
    job_id: &uuid::Uuid,
    w_id: &str,
    conn: &Connection,
) -> anyhow::Result<Vec<(&'static str, String)>> {
    #[allow(unused_mut)]
    let mut envs;
    #[cfg(all(feature = "private", feature = "enterprise"))]
    if !matches!(job_kind, JobKind::DeploymentCallback)
        && is_otel_tracing_proxy_enabled_for_lang(lang).await
    {
        envs = get_otel_tracing_proxy_envs(job_id, w_id, conn).await?;
    } else {
        envs = PROXY_ENVS.clone();
    }
    #[cfg(not(all(feature = "private", feature = "enterprise")))]
    {
        let _ = (lang, job_kind, w_id, conn);
        envs = PROXY_ENVS.clone();
    }
    envs.extend(get_otel_context_envs(job_id));
    Ok(envs)
}

#[cfg(all(feature = "private", feature = "enterprise"))]
async fn get_otel_tracing_proxy_envs(
    job_id: &uuid::Uuid,
    w_id: &str,
    conn: &Connection,
) -> anyhow::Result<Vec<(&'static str, String)>> {
    let port = match *crate::otel_tracing_proxy_ee::TRACING_PROXY_PORT
        .read()
        .await
    {
        Some(p) => p,
        None => {
            let reason = "OTEL tracing proxy is enabled but not available (not initialized yet, or NUM_WORKERS > 1). \
                This job's HTTP requests will not be traced.";
            tracing::warn!("{}", reason);
            append_logs(job_id, w_id, format!("\n[warning] {reason}\n"), conn).await;
            return Ok(PROXY_ENVS.clone());
        }
    };
    let proxy_url = format!("http://127.0.0.1:{}", port);
    let no_proxy = build_tracing_proxy_no_proxy().await;
    Ok(vec![
        ("HTTP_PROXY", proxy_url.clone()),
        ("HTTPS_PROXY", proxy_url.clone()),
        // Lowercase variants for Ruby and other runtimes that check lowercase first
        ("http_proxy", proxy_url.clone()),
        ("https_proxy", proxy_url),
        ("NO_PROXY", no_proxy.clone()),
        ("no_proxy", no_proxy),
        // CA cert for various runtimes to trust the tracing proxy
        ("SSL_CERT_FILE", TRACING_PROXY_CA_CERT_PATH.to_string()),
        ("REQUESTS_CA_BUNDLE", TRACING_PROXY_CA_CERT_PATH.to_string()),
        (
            "NODE_EXTRA_CA_CERTS",
            TRACING_PROXY_CA_CERT_PATH.to_string(),
        ),
        ("CURL_CA_BUNDLE", TRACING_PROXY_CA_CERT_PATH.to_string()),
        ("GIT_SSL_CAINFO", TRACING_PROXY_CA_CERT_PATH.to_string()),
        ("DENO_CERT", TRACING_PROXY_CA_CERT_PATH.to_string()),
    ])
}

/// NO_PROXY value injected into jobs so their HTTP clients bypass the local MITM proxy for
/// the configured hosts. This is distinct from the worker's own NO_PROXY env, which governs
/// what the MITM proxy bypasses when relaying upstream (e.g. through a corporate proxy) and
/// is honored automatically by the in-process MITM. The configured hosts are tunneled
/// through the proxy without TLS interception, so clients that pin their own CA (kubectl,
/// helm, terraform, etc.) keep working. Empty when unset, matching the prior behavior of
/// intercepting all destinations including loopback.
///
/// The worker's own `NO_PROXY` env is merged in so that enabling HTTP request tracing never
/// silently narrows exclusions an operator already configured at the container level: hosts
/// reachable directly before tracing was turned on stay reachable directly afterwards. The
/// upstream-relay side (`build_no_proxy_intercept`) already honors the container `NO_PROXY`;
/// this keeps the injected-into-jobs side symmetric. Note that job runtimes match `NO_PROXY`
/// by hostname/suffix, not by resolving against CIDR ranges, so CIDR-only entries (e.g.
/// `10.0.0.0/8`) carry over but won't match a hostname target — operators still need an
/// explicit host/suffix entry for those.
#[cfg(all(feature = "private", feature = "enterprise"))]
async fn build_tracing_proxy_no_proxy() -> String {
    let configured = OTEL_TRACING_PROXY_SETTINGS
        .read()
        .await
        .no_proxy_hosts
        .clone();
    normalize_no_proxy_hosts([configured.as_deref(), NO_PROXY.as_deref()])
}

/// Split comma-separated NO_PROXY sources, trim whitespace, drop empty entries, and
/// deduplicate while preserving first-occurrence order. Sources are concatenated in the
/// order given (earlier sources win on ordering). Returns an empty string when all sources
/// are `None`/empty.
#[cfg(all(feature = "private", feature = "enterprise"))]
fn normalize_no_proxy_hosts<'a>(sources: impl IntoIterator<Item = Option<&'a str>>) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<&str> = Vec::new();
    for source in sources.into_iter().flatten() {
        for entry in source.split(',') {
            let trimmed = entry.trim();
            if !trimmed.is_empty() && seen.insert(trimmed) {
                out.push(trimmed);
            }
        }
    }
    out.join(",")
}

#[cfg(all(test, feature = "private", feature = "enterprise"))]
mod no_proxy_tests {
    use super::normalize_no_proxy_hosts;

    #[test]
    fn unset_returns_empty() {
        assert_eq!(normalize_no_proxy_hosts([None]), "");
        assert_eq!(normalize_no_proxy_hosts([None, None]), "");
    }

    #[test]
    fn empty_and_whitespace_only_returns_empty() {
        assert_eq!(normalize_no_proxy_hosts([Some("")]), "");
        assert_eq!(normalize_no_proxy_hosts([Some("  ,  ,\t")]), "");
    }

    #[test]
    fn trims_and_skips_empties() {
        assert_eq!(
            normalize_no_proxy_hosts([Some("  *.eks.amazonaws.com  ,, *.internal ")]),
            "*.eks.amazonaws.com,*.internal"
        );
    }

    #[test]
    fn dedupes_preserving_first_occurrence_order() {
        assert_eq!(normalize_no_proxy_hosts([Some("a,b,a,c,b,d")]), "a,b,c,d");
    }

    #[test]
    fn merges_configured_with_container_no_proxy() {
        // Configured tracing hosts come first, then the container NO_PROXY is appended.
        assert_eq!(
            normalize_no_proxy_hosts([
                Some("gitlab.internal"),
                Some("localhost,127.0.0.1,10.0.0.0/8,.cluster.local")
            ]),
            "gitlab.internal,localhost,127.0.0.1,10.0.0.0/8,.cluster.local"
        );
    }

    #[test]
    fn merges_dedupe_across_sources() {
        // Entries present in both sources are not duplicated.
        assert_eq!(
            normalize_no_proxy_hosts([Some("localhost,gitlab.internal"), Some("localhost,.svc")]),
            "localhost,gitlab.internal,.svc"
        );
    }

    #[test]
    fn container_no_proxy_carries_over_when_unconfigured() {
        // Tracing setting unset but container NO_PROXY present: exclusions still carry over,
        // so enabling tracing does not silently drop them.
        assert_eq!(
            normalize_no_proxy_hosts([None, Some(".cluster.local,gitlab.internal")]),
            ".cluster.local,gitlab.internal"
        );
    }
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

// Share of the worker's memory budget one SQL result may occupy. Collecting rows
// costs several times the JSON they serialize to — a separately allocated value
// per row, then a contiguous buffer holding all of them — and the worker still
// needs the rest of its budget for what it already has resident.
const SQL_RESULT_SIZE_FRACTION: f64 = 0.15;
// Under this a result cannot threaten a worker of any size, so capping it would
// only reject work that would have succeeded.
const MIN_MAX_SQL_RESULT_SIZE: usize = 8 * 1024 * 1024;

// Share of the worker's memory budget a Go compilation may hold. Set high enough
// that an ordinary build never approaches it, so the only builds whose behavior
// changes are those that were about to take the worker down.
//
// What the rest covers is not the worker process, which is tens of MB and would
// argue for a constant: it is everything `GOMEMLIMIT` does not count and that grows
// with the build — the toolchain's mmapped inputs and outputs, its non-Go
// allocations, and the page cache its writes charge to the cgroup.
const GO_BUILD_MEMLIMIT_FRACTION: f64 = 0.75;
// Heap a Go compiler is comfortable in: cores are only put to work while the budget
// still affords each of them this much.
const GO_BUILD_TARGET_MEMLIMIT: usize = 384 * 1024 * 1024;
// Driver plus the compilers below which a build stops overlapping and starts
// waiting. Measured on a dependency-heavy build: at a budget too small to give this
// many the target share, splitting it further still compiles faster than handing
// fewer processes more — one compiler alone costs ~3x what five of them do on the
// same budget — so the process count holds and the share absorbs the difference.
const MIN_GO_BUILD_PROCESSES: usize = 6;
// Floor under the share, past which dividing the budget again buys nothing: the
// processes only trade compiling time for collecting time.
const MIN_GO_BUILD_MEMLIMIT: usize = 128 * 1024 * 1024;

/// `"512"`, `"512MB"`, `"2GiB"`, `"1.5GB"` -> bytes. Suffixes are case-insensitive
/// and binary, so `MB` and `MiB` both mean 1024².
///
/// Fractions have to be accepted even though `format_byte_size` never emits one:
/// the duckdb error is rendered by the FFI crate's own copy of that helper, which
/// rounds to a fraction above 1 GiB, and every limit an error quotes is meant to
/// be usable as a setting verbatim.
fn parse_byte_size(v: &str) -> Option<usize> {
    let upper = v.trim().to_ascii_uppercase();
    // Longest-first: `GB` would otherwise swallow `GIB`, and `B` every other suffix.
    let (digits, mult) = [
        ("GIB", 1u64 << 30),
        ("GB", 1u64 << 30),
        ("MIB", 1u64 << 20),
        ("MB", 1u64 << 20),
        ("KIB", 1u64 << 10),
        ("KB", 1u64 << 10),
        ("B", 1),
    ]
    .into_iter()
    .find_map(|(suffix, mult)| upper.strip_suffix(suffix).map(|d| (d, mult)))
    .unwrap_or((upper.as_str(), 1));
    let n = digits.trim().parse::<f64>().ok()?;
    if !n.is_finite() || n < 0.0 {
        return None;
    }
    let bytes = n * mult as f64;
    (bytes <= usize::MAX as f64).then(|| bytes as usize)
}

lazy_static::lazy_static! {
    /// Bytes one SQL job may collect before its executor gives up — the budget
    /// spans every query block in the job, since what the worker cannot survive is
    /// the total it ends up holding. Nothing else bounds it at collection time:
    /// every row is accumulated before anything can stream the result out, so an
    /// oversized one grows past the cgroup and the OOM killer takes the worker
    /// process down — every job colocated on it, not just the one that asked.
    /// (`MAX_RESULT_SIZE_MB` does bound the finished result, but only once the job
    /// completes, which is after the memory has already been spent.)
    ///
    /// This is a worker-survival limit, not a product one, so it applies the same
    /// on cloud as off it. `MAX_SQL_RESULT_SIZE` overrides it; `0` and a worker
    /// with no cgroup reading to scale from both mean no cap.
    ///
    /// It bounds what is *collected*, not the process: the collected rows are
    /// still live while a second whole copy is serialized out of them, so peak
    /// sits near twice the cap. The derived default leaves room for that — it is
    /// a fraction of the worker's budget, not the whole of it.
    pub(crate) static ref MAX_SQL_RESULT_SIZE: usize = {
        let explicit = std::env::var("MAX_SQL_RESULT_SIZE").ok().and_then(|v| {
            let parsed = parse_byte_size(&v);
            if parsed.is_none() {
                // Falling back silently would leave the operator believing a
                // limit is in force that never parsed.
                tracing::warn!(
                    "MAX_SQL_RESULT_SIZE={v:?} is not a byte size (e.g. 512MB, 2GiB); \
                     falling back to the memory-derived limit"
                );
            }
            parsed
        });
        match explicit {
            // `0` turns the cap off. Normalized here rather than forwarded,
            // so "no cap" is expressed as a limit nothing can exceed instead of
            // a sentinel every consumer has to know to special-case.
            Some(0) => usize::MAX,
            // The floor guards the derived value only. An explicit setting is
            // taken at face value, including one deliberately below it.
            Some(limit) => limit,
            None => windmill_common::worker::get_memory()
                .filter(|bytes| *bytes > 0)
                .map(|bytes| ((bytes as f64 * SQL_RESULT_SIZE_FRACTION) as usize)
                    .max(MIN_MAX_SQL_RESULT_SIZE))
                .unwrap_or(usize::MAX),
        }
    };

    /// What a Go compilation is allowed to cost, derived from the worker's memory
    /// budget. Nothing else bounds it: a compilation grows until the cgroup OOM
    /// killer takes the worker process down, every job colocated on it with it.
    /// `GOMEMLIMIT` is soft — the GC works harder as the heap nears it instead of
    /// failing the allocation — so a pathological build turns into a slow one.
    ///
    /// `GO_BUILD_MEMLIMIT` overrides the budget (`512MB`, `2GiB`, …), and `0`/`off`
    /// disables the whole thing, as does a worker with no cgroup memory reading to
    /// scale from.
    pub(crate) static ref GO_BUILD_LIMITS: Option<GoBuildLimits> = resolve_go_build_limits(
        std::env::var("GO_BUILD_MEMLIMIT").ok().as_deref(),
        windmill_common::worker::get_memory(),
        worker_vcpus(),
    );
}

/// How much memory the Go toolchain may hold while compiling a script, expressed
/// the only way the toolchain understands it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct GoBuildLimits {
    /// What the whole build may hold, and so what a step that is one process gets.
    pub budget: usize,
    /// `GOMEMLIMIT` for one process of a build that fans out.
    pub memlimit: usize,
    /// `GOMAXPROCS`, which is also `go build`'s default `-p`: how many compilers it
    /// runs at once.
    pub parallelism: usize,
}

fn worker_vcpus() -> usize {
    effective_vcpus(
        windmill_common::worker::get_vcpus(),
        windmill_common::worker::get_cpu_period(),
        windmill_common::worker::get_affinity_cpus()
            .or_else(|| std::thread::available_parallelism().ok().map(|n| n.get()))
            .unwrap_or(1),
    )
}

/// CPUs worth of work the worker can actually run at once.
///
/// The cgroup states its allowance as a quota over a period, and only their ratio
/// is a number of CPUs — `1500m` is `150000/100000`. A fraction still runs work, so
/// it rounds up the way the Go runtime's own container-aware `GOMAXPROCS` does:
/// flooring would call that worker single-core and serialize its builds.
///
/// `host_cpus` counts the CPUs the worker may run on, quota aside, and is both the
/// answer when there is no quota and the floor under one: Go's own container-aware
/// default never drops below two while the machine has two to give, since even a
/// fraction of a CPU compiles two packages faster than it compiles them in series.
fn effective_vcpus(quota_us: Option<i64>, period_us: Option<i64>, host_cpus: usize) -> usize {
    quota_us
        .zip(period_us)
        .filter(|(quota, period)| *quota > 0 && *period > 0)
        .map(|(quota, period)| ((quota + period - 1) / period) as usize)
        .unwrap_or(host_cpus)
        .max(host_cpus.min(2))
        .max(1)
}

/// Split a build budget into the per-process cap and the parallelism it assumes.
///
/// `GOMEMLIMIT` bounds one process, and a build is a driver plus up to `-p`
/// compilers that each inherit the same value, so the budget only holds if the
/// number of processes sharing it is pinned alongside it.
fn resolve_go_build_limits(
    env_override: Option<&str>,
    worker_memory: Option<i64>,
    vcpus: usize,
) -> Option<GoBuildLimits> {
    let derived = || {
        worker_memory
            .filter(|bytes| *bytes > 0)
            .map(|bytes| (bytes as f64 * GO_BUILD_MEMLIMIT_FRACTION) as usize)
    };

    let budget = match env_override.map(str::trim).filter(|v| !v.is_empty()) {
        None => derived()?,
        Some(v) if v.eq_ignore_ascii_case("off") => return None,
        Some(v) => match parse_byte_size(v) {
            // A zero budget would have the GC hold the heap at nothing, so it reads
            // as "no limit" instead.
            Some(0) => return None,
            Some(bytes) => bytes,
            None => {
                // Falling back silently would leave the operator believing the limit
                // they wrote is in force.
                tracing::warn!(
                    "Go build memory budget {v:?} is not a byte size (e.g. 512MB, \
                     2GiB); falling back to the worker's memory-derived budget"
                );
                derived()?
            }
        },
    };

    // One compiler per core while the budget affords each the target share, never
    // so few that the build stops overlapping, and never more than the cores can
    // run. The floor is the last word: on a worker too small to honor both, the
    // budget is the one that gives, since processes squeezed under it make no
    // progress to bound.
    let cap = vcpus.max(1) + 1;
    let processes = (budget / GO_BUILD_TARGET_MEMLIMIT).clamp(MIN_GO_BUILD_PROCESSES.min(cap), cap);
    Some(GoBuildLimits {
        // Floored like the share it is an alternative to: a step that holds the
        // whole budget must never end up with less than one of six compilers.
        budget: budget.max(MIN_GO_BUILD_MEMLIMIT),
        memlimit: (budget / processes).max(MIN_GO_BUILD_MEMLIMIT),
        parallelism: processes - 1,
    })
}

#[cfg(test)]
mod go_build_limits_tests {
    use super::{
        effective_vcpus, resolve_go_build_limits, GoBuildLimits, GO_BUILD_TARGET_MEMLIMIT,
        MIN_GO_BUILD_MEMLIMIT,
    };

    const GIB: i64 = 1024 * 1024 * 1024;

    fn limits(budget: usize, memlimit: usize, parallelism: usize) -> Option<GoBuildLimits> {
        Some(GoBuildLimits { budget, memlimit, parallelism })
    }

    #[test]
    fn reads_the_cgroup_allowance_as_cpus() {
        // 1500m: a fraction of a CPU still runs work, so it is two compilers' worth
        // of concurrency rather than one.
        assert_eq!(effective_vcpus(Some(150_000), Some(100_000), 24), 2);
        assert_eq!(effective_vcpus(Some(400_000), Some(100_000), 24), 4);
        // The period is configurable, so only the ratio means anything.
        assert_eq!(effective_vcpus(Some(400_000), Some(50_000), 24), 8);
        // The quota is the answer whenever there is one to read, since the count
        // it would otherwise be clamped to has already floored it.
        assert_eq!(effective_vcpus(Some(4_000_000), Some(100_000), 4), 40);
        assert_eq!(effective_vcpus(None, None, 8), 8);
        // Under a whole CPU the floor is two, as the Go runtime's own default is —
        // but only where there are two to give.
        assert_eq!(effective_vcpus(Some(50_000), Some(100_000), 24), 2);
        assert_eq!(effective_vcpus(Some(50_000), Some(100_000), 1), 1);
        // A one-CPU allowance stays one when that is all the worker may use, which
        // is how the Windows 1CU cap reports itself.
        assert_eq!(effective_vcpus(Some(100_000), Some(100_000), 1), 1);
    }

    #[test]
    fn splits_the_budget_across_the_build_tree() {
        // Fewer cores than the budget could feed: every core gets a compiler, and
        // they share 3GiB with the driver.
        assert_eq!(
            resolve_go_build_limits(None, Some(4 * GIB), 4),
            limits(3 * GIB as usize, 3 * GIB as usize / 5, 4)
        );
        // More cores than it can feed at the target share: the extra cores idle
        // rather than shrink every compiler.
        assert_eq!(
            resolve_go_build_limits(None, Some(4 * GIB), 64),
            limits(3 * GIB as usize, GO_BUILD_TARGET_MEMLIMIT, 7)
        );
        // Too small to give even the minimum process count the target share: the
        // build keeps overlapping and the share absorbs it.
        assert_eq!(
            resolve_go_build_limits(None, Some(GIB), 64),
            limits(3 * GIB as usize / 4, 3 * GIB as usize / 4 / 6, 5)
        );
        // Nothing to scale from leaves the toolchain unlimited, as it was before.
        assert_eq!(resolve_go_build_limits(None, None, 4), None);
        // Under the floor the budget gives instead of the share.
        assert_eq!(
            resolve_go_build_limits(None, Some(GIB / 8), 4),
            limits(MIN_GO_BUILD_MEMLIMIT, MIN_GO_BUILD_MEMLIMIT, 4)
        );
        assert_eq!(
            resolve_go_build_limits(Some("2GiB"), Some(4 * GIB), 4),
            limits(2 * GIB as usize, 2 * GIB as usize / 5, 4)
        );
        assert_eq!(resolve_go_build_limits(Some("off"), Some(4 * GIB), 4), None);
        assert_eq!(resolve_go_build_limits(Some("0MB"), Some(4 * GIB), 4), None);
        // An unparseable override falls back to the derived budget rather than
        // lifting the limit.
        assert_eq!(
            resolve_go_build_limits(Some("lots"), Some(4 * GIB), 4),
            limits(3 * GIB as usize, 3 * GIB as usize / 5, 4)
        );
    }
}

/// The limit postgres collection is bounded by: the cloud product cap where one
/// applies, and otherwise the worker-survival cap, which is the only thing worth
/// enforcing on a deployment that has no product limit to answer to.
///
/// Duckdb deliberately does not come through here — its cap is a survival limit
/// only, so it reads `MAX_SQL_RESULT_SIZE` directly and is never narrowed to the
/// cloud product limit.
pub(crate) fn max_sql_result_size() -> usize {
    if *CLOUD_HOSTED {
        MAX_RESULT_SIZE * 4
    } else {
        *MAX_SQL_RESULT_SIZE
    }
}

/// Renders a byte count so the figure in the error names the limit exactly and
/// can be set as `MAX_SQL_RESULT_SIZE` verbatim.
///
/// A unit is only used when it divides the count evenly. Rounding to the nearest
/// MB reads better but names a threshold nobody configured — a 1.5MB limit shown
/// as `1MB` both misreports it and lowers it if pasted back — and the
/// memory-derived default is rarely a whole number of MB, which is exactly the
/// case an operator is most likely to copy.
fn format_byte_size(bytes: usize) -> String {
    [("GB", 1usize << 30), ("MB", 1 << 20), ("KB", 1 << 10)]
        .into_iter()
        .find(|(_, unit)| bytes >= *unit && bytes % unit == 0)
        .map(|(suffix, unit)| format!("{}{suffix}", bytes / unit))
        .unwrap_or_else(|| format!("{bytes}B"))
}

/// Serializes `value` to JSON, refusing to allocate more than `budget` bytes.
///
/// A running total kept over values in memory does not see what serializing them
/// costs: JSON escaping expands text on the way out — one control character
/// becomes the six-byte escape `\u0001` — so a value that fit the budget unescaped
/// can still allocate several times it while being written, long past the point
/// where a check between rows could help. Bounding the writer is what keeps that
/// expansion inside the budget rather than inside the cgroup.
///
/// `None` means the output did not fit. Serializing a `serde_json::Value` cannot
/// fail for any other reason, which is what makes that reading unambiguous.
pub(crate) fn to_raw_value_within<T: serde::Serialize>(
    value: &T,
    budget: usize,
) -> Option<Box<serde_json::value::RawValue>> {
    struct Budgeted {
        buf: Vec<u8>,
        left: usize,
    }
    impl std::io::Write for Budgeted {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.write_all(bytes)?;
            Ok(bytes.len())
        }
        // `Vec<u8>` overrides this too: the default implementation loops over
        // `write`, and serde_json emits a great many small pieces per row.
        fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
            if bytes.len() > self.left {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "result over budget",
                ));
            }
            self.left -= bytes.len();
            self.buf.extend_from_slice(bytes);
            Ok(())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let mut writer = Budgeted { buf: Vec::new(), left: budget };
    serde_json::to_writer(&mut writer, value).ok()?;
    let json = String::from_utf8(writer.buf).ok()?;
    // SAFETY: `to_writer` returned `Ok`, so `json` holds one complete, well-formed
    // JSON value with no surrounding whitespace. Running out of budget is the only
    // way a partial write happens, and it takes the `?` above instead of reaching
    // here. The safe constructor re-parses every row to learn the same thing, which
    // measured ~1.8x the cost of serializing it in the first place; serde_json
    // itself builds a `RawValue` this way in `to_raw_value`, and `debug_assert!`s
    // the invariant by re-parsing in debug builds.
    Some(unsafe { serde_json::value::RawValue::from_string_unchecked(json) })
}

/// Wording shared by the SQL executors that collect rows in the worker process.
/// `MAX_SQL_RESULT_SIZE` is not settable on cloud, so only mention it off-cloud.
///
/// Only the limit is quoted: collection stops on the row that crosses it, so the
/// running total is the threshold plus one row, not the size of the result.
pub(crate) fn sql_result_too_large_error(limit: usize) -> Error {
    // Each branch is a whole sentence: splicing a prefix in leaves the cloud
    // message starting mid-sentence, since there is no prefix to splice there.
    let remedy = if *CLOUD_HOSTED {
        "Return fewer rows"
    } else {
        "Raise MAX_SQL_RESULT_SIZE, or return fewer rows"
    };
    Error::ExecutionErr(format!(
        "Query result too large: collecting it passed the {} limit. {remedy} — \
         aggregate, add a LIMIT, or write the rows out from the query instead of \
         returning them.",
        format_byte_size(limit),
    ))
}

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

    pub fn set_worker_killpill(&mut self, killpill_tx: KillpillSender) {
        if let Self::Sql(sql) = self {
            sql.worker_killpill_tx = Some(killpill_tx);
        }
    }

    pub fn send_worker_killpill(&self) {
        if let Self::Sql(SqlJobCompletedSender { worker_killpill_tx: Some(killpill_tx), .. }) = self
        {
            killpill_tx.send();
        }
    }
}

#[derive(Clone)]
pub struct SqlJobCompletedSender {
    sender: flume::Sender<SendResult>,
    unbounded_sender: flume::Sender<SendResult>,
    killpill_tx: broadcast::Sender<()>,
    worker_killpill_tx: Option<KillpillSender>,
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
            Self::Sql(SqlJobCompletedSender {
                sender,
                unbounded_sender,
                killpill_tx,
                worker_killpill_tx: None,
            }),
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
    // Aggregate onto the true top-level root (root_job → flow_innermost_root_job → parent_job).
    // `get_root_job_id` falls back to the job's own id when none are set; filter that out so
    // standalone scripts (no parent flow) skip the aggregate insertion.
    let root_job_id = Some(get_root_job_id(queued_job)).filter(|&id| id != job_id);
    let conn = conn.clone();

    if let Some(db) = conn.as_sql() {
        let db = db.clone();
        let span = tracing::Span::current();
        windmill_common::log_context::spawn_with_log_context(async move {
            async move {
                match insert_wait_time(job_id, root_job_id, &db, wait_time).await {
                    Ok(()) => tracing::warn!("job {job_id} waited for an executor for a significant amount of time. Recording value wait_time={}ms", wait_time),
                    Err(e) => tracing::error!("Failed to insert outstanding wait time: {}", e),
                }
            }
            .instrument(span)
            .await
        });
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
        job_kind = %arc_job.kind.as_str(),
        created_by = %arc_job.created_by,
        trigger_kind = field::Empty,
        trigger = field::Empty,
        script_hash = field::Empty,
        otel.name = field::Empty,
        otel.status_code = field::Empty,
        otel.status_message = field::Empty,
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
    if let Some(trigger_kind) = arc_job.trigger_kind.as_ref() {
        span.record("trigger_kind", trigger_kind.as_str());
    }
    if let Some(trigger) = arc_job.trigger.as_ref() {
        span.record("trigger", trigger.as_str());
    }
    if let Some(script_hash) = arc_job.runnable_id.as_ref() {
        span.record("script_hash", script_hash.to_string().as_str());
    }

    // Parent the job span on the inbound distributed trace when the request that
    // enqueued it (or its flow root) carried a W3C `traceparent`; otherwise on
    // the UUID-derived context. See `otel_ee::set_job_span_parent`.
    crate::otel_oss::set_job_span_parent(&span, arc_job, &rj);
    span
}

/// Max characters of an error message copied into the `otel.status_message`
/// span attribute. Prevents a single verbose failure from blowing up the span
/// payload on OTLP exporters.
const STATUS_DESCRIPTION_MAX_LEN: usize = 512;

/// Outcome of running a queued job, carrying enough information to set the
/// OTLP `Status` on the outer `"job"` span without conflating "another worker
/// raced us" with "the user's script raised an exception".
#[derive(Debug)]
pub enum JobOutcome {
    /// Job ran cleanly, was forwarded as a flow, was a no-op (test workspace),
    /// or was suspended waiting for child jobs (WAC v2).
    /// All of these leave the span `Status` `Unset`.
    Completed,
    /// A valid cached result was found for the job's args and path, so it was
    /// answered without running anything.
    CompletedFromCache,
    /// Job was attempted but its execution returned an error; the failure has
    /// been dispatched to the result processor. `description` holds the
    /// truncated error string for the outer span's `Status.message`.
    Failed { description: String },
    /// Another worker (or the same worker after a restart) already inserted a
    /// row in `v2_job_completed`; this worker has nothing to do.
    AlreadyCompleted,
}

impl JobOutcome {
    /// True when the job completed successfully on this worker. Used by
    /// callers that previously matched on `Ok(true)`.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Completed | Self::CompletedFromCache)
    }

    /// Whether anything ran here. Only a cached result is a true no-run:
    /// `AlreadyCompleted` is raised when the queue row disappears *while* the
    /// child process is running, so that job did execute, and was interrupted.
    fn ran_on_this_worker(&self) -> bool {
        !matches!(self, Self::CompletedFromCache)
    }
}

/// Record `otel.status_code` / `otel.status_message` on the current span
/// when a job fails. Called from inside the `.instrument(job_span)` future so
/// that `Span::current()` resolves to the `"job"` span created by
/// `create_span_with_name`.
///
/// `Completed` leaves the fields unset (`Status::Unset`, equivalent to OK
/// per OTel spec). The other variants set `Status.code = ERROR` with a
/// description that reflects the actual cause.
pub(crate) fn record_job_span_status(result: &windmill_common::error::Result<JobOutcome>) {
    let description = match result {
        Ok(JobOutcome::Completed) | Ok(JobOutcome::CompletedFromCache) => return,
        Ok(JobOutcome::Failed { description }) => description.clone(),
        Ok(JobOutcome::AlreadyCompleted) => "job already completed by another worker".to_string(),
        Err(err) => truncate_description(&err.to_string()),
    };
    let span = tracing::Span::current();
    span.record("otel.status_code", "ERROR");
    span.record("otel.status_message", description.as_str());
}

/// Cap an error description at `STATUS_DESCRIPTION_MAX_LEN` bytes, appending
/// an ellipsis when truncated. The cut is rounded down to the nearest UTF-8
/// codepoint boundary so the result is always valid UTF-8. Always returns an
/// owned `String` so callers don't have to juggle `Cow` lifetimes.
pub(crate) fn truncate_description(s: &str) -> String {
    if s.len() <= STATUS_DESCRIPTION_MAX_LEN {
        return s.to_string();
    }
    let mut end = STATUS_DESCRIPTION_MAX_LEN;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let mut truncated = String::with_capacity(end + 3);
    truncated.push_str(&s[..end]);
    truncated.push('…');
    truncated
}

/// Build the per-job `LogContext` that gets seeded at the top of job
/// execution. Mirrors the field set recorded on the `"job"` tracing span in
/// `create_span_with_name` so exported log records and traces carry the
/// same identifiers.
pub fn log_context_for_job(
    arc_job: &MiniPulledJob,
    worker_name: &str,
    hostname: Option<&str>,
) -> windmill_common::log_context::LogContext {
    let existing = windmill_common::log_context::current_log_context()
        .map(|arc| (*arc).clone())
        .unwrap_or_default();
    windmill_common::log_context::LogContext {
        job_id: Some(arc_job.id.to_string()),
        workspace_id: Some(arc_job.workspace_id.clone()),
        worker: Some(worker_name.to_string()),
        tag: Some(arc_job.tag.clone()),
        job_kind: Some(arc_job.kind.as_str().to_string()),
        created_by: Some(arc_job.created_by.clone()),
        script_path: arc_job.runnable_path.clone(),
        script_hash: arc_job.runnable_id.map(|h| h.to_string()),
        language: arc_job.script_lang.map(|l| l.as_str().to_string()),
        flow_step_id: arc_job.flow_step_id.clone(),
        parent_job: arc_job.parent_job.map(|id| id.to_string()),
        root_job: arc_job.flow_innermost_root_job.map(|id| id.to_string()),
        trigger_kind: arc_job
            .trigger_kind
            .as_ref()
            .map(|k| k.as_str().to_string()),
        trigger: arc_job.trigger.clone(),
        hostname: hostname.map(|h| h.to_string()),
        inbound_traceparent: job_inbound_traceparent(arc_job),
        ..existing
    }
}

/// Extract the inbound W3C `traceparent` captured at enqueue from a job's args
/// (reserved `_wm_traceparent` key). Present only on directly-triggered jobs
/// (and flow steps that inherited it).
pub(crate) fn job_inbound_traceparent(job: &MiniPulledJob) -> Option<String> {
    job.args
        .as_ref()
        .and_then(|a| a.get(windmill_common::jobs::WM_TRACEPARENT))
        .and_then(|raw| serde_json::from_str::<String>(raw.get()).ok())
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
                StepFailureKind::Normal,
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

/// How long the interactive shell loop waits before polling its tag again, when it found no
/// job. The sub-second cadence only pays off while somebody is typing into the shell, so it
/// is reserved for a session that has run a command: before the first one this process serves
/// there is nothing to keep responsive, only the next session to notice.
///
/// - a live session, last command under `TIMEOUT_TO_RESET_WORKER_SHELL_NAP_TIME_DURATION`
///   ago: `sleep_queue() * 10`
/// - no command yet this process: `WORKER_SHELL_INITIAL_NAP_TIME_DURATION`, which bounds how
///   long the first command of a session waits
/// - nothing for `TIMEOUT_TO_RESET_WORKER_SHELL_NAP_TIME_DURATION`, counted from the last
///   command or from process start: `WORKER_SHELL_NAP_TIME_DURATION`
///
/// A worker whose process is recycled after N jobs cannot count on living long enough to
/// reach that last state, and at N=1 never does, so it starts there instead. That holds for
/// any N, since N says nothing about how long a process lasts.
fn interactive_shell_nap(
    now: Instant,
    started_at: Instant,
    last_executed_job: Option<Instant>,
    recycles_after_n_jobs: bool,
) -> Duration {
    let quiet_since = last_executed_job.unwrap_or(started_at);
    if now.duration_since(quiet_since).as_secs() > TIMEOUT_TO_RESET_WORKER_SHELL_NAP_TIME_DURATION {
        return Duration::from_secs(WORKER_SHELL_NAP_TIME_DURATION);
    }
    match last_executed_job {
        Some(_) => Duration::from_millis(sleep_queue() * 10),
        None if recycles_after_n_jobs => Duration::from_secs(WORKER_SHELL_NAP_TIME_DURATION),
        None => Duration::from_secs(WORKER_SHELL_INITIAL_NAP_TIME_DURATION),
    }
}

#[cfg(test)]
mod interactive_shell_nap_tests {
    use super::*;

    const LONG: Duration = Duration::from_secs(WORKER_SHELL_NAP_TIME_DURATION);
    const INITIAL: Duration = Duration::from_secs(WORKER_SHELL_INITIAL_NAP_TIME_DURATION);

    #[test]
    fn only_a_live_shell_session_gets_the_sub_second_cadence() {
        let start = Instant::now();
        let quiet =
            start + Duration::from_secs(TIMEOUT_TO_RESET_WORKER_SHELL_NAP_TIME_DURATION + 1);
        let fast = Duration::from_millis(sleep_queue() * 10);
        // Nobody has opened a shell on this worker yet, so there is no session to keep
        // responsive: only the first command of the next one to notice.
        assert_eq!(interactive_shell_nap(start, start, None, false), INITIAL);
        assert_eq!(interactive_shell_nap(quiet, start, None, false), LONG);
        // A worker recycled after N jobs may never live to back off, so it starts backed off.
        assert_eq!(interactive_shell_nap(start, start, None, true), LONG);
        // Either way, a served shell job is a live session and gets the fast cadence.
        assert_eq!(interactive_shell_nap(quiet, start, Some(quiet), true), fast);
        assert_eq!(
            interactive_shell_nap(
                quiet + Duration::from_secs(TIMEOUT_TO_RESET_WORKER_SHELL_NAP_TIME_DURATION + 1),
                start,
                Some(quiet),
                true
            ),
            LONG
        );
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
        let started_at = Instant::now();
        let mut occupancy_metrics = OccupancyMetrics::new(started_at);

        // `None` means no shell job has been served yet, which the nap distinguishes from a
        // shell session that has gone quiet.
        let mut last_executed_job: Option<Instant> = None;

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
                    let nap_time = interactive_shell_nap(
                        Instant::now(),
                        started_at,
                        last_executed_job,
                        EXIT_AFTER_N_JOBS.is_some(),
                    );
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
                    tokio::time::sleep(Duration::from_millis(sleep_queue() * 20)).await;
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

/// Whether running this job may leave anything behind in the worker's environment, which is
/// what `EXIT_AFTER_N_JOBS` counts. Flow orchestration, noop/identity steps and the warmup
/// job run no user code at all, and the init/periodic scripts are the worker's own setup:
/// counting any of them would burn a restart cycle without a single user job having run.
///
/// The internal scripts are recognized by the path this very worker queues them under, and
/// not by their tag alone: a tag is only routing configuration, so a worker pulling
/// `init_script` could otherwise be fed user jobs that never age its environment.
fn dirties_worker_env(
    kind: JobKind,
    tag: &str,
    runnable_path: Option<&str>,
    job_id: Uuid,
    rejected_before_run: bool,
    worker_name: &str,
) -> bool {
    let own_script = |own_tag: &str, path_prefix: &str| {
        tag == own_tag
            && runnable_path.is_some_and(|p| p.starts_with(&format!("{path_prefix}{worker_name}")))
    };
    !rejected_before_run
        && !kind.is_flow()
        && !matches!(
            kind,
            JobKind::Noop
                | JobKind::Identity
                | JobKind::UnassignedScript
                | JobKind::UnassignedFlow
                | JobKind::UnassignedSinglestepFlow
        )
        && !own_script(INIT_SCRIPT_TAG, INIT_SCRIPT_PATH_PREFIX)
        && !own_script(PERIODIC_SCRIPT_TAG, PERIODIC_SCRIPT_PATH_PREFIX)
        && job_id != Uuid::nil()
}

#[cfg(test)]
mod exit_after_n_jobs_tests {
    use super::*;

    const WK: &str = "wk-default-host-a1b2c";

    /// A worker with `EXIT_AFTER_N_JOBS=1` that counted its own init script would shut down
    /// before ever running a user job, restart, and loop on that forever. Jobs rejected
    /// before the executor ran are the same waste of a restart.
    #[test]
    fn worker_own_jobs_do_not_count() {
        for (kind, tag, path) in [
            (
                JobKind::Script,
                INIT_SCRIPT_TAG,
                Some(format!("{INIT_SCRIPT_PATH_PREFIX}{WK}")),
            ),
            (
                JobKind::Script,
                PERIODIC_SCRIPT_TAG,
                Some(format!("{PERIODIC_SCRIPT_PATH_PREFIX}{WK}_1700000000")),
            ),
            (JobKind::Flow, "flow", Some("u/admin/f".to_string())),
            (JobKind::Noop, "other", None),
            (JobKind::UnassignedScript, "bash", Some("u/admin/s".into())),
        ] {
            assert!(
                !dirties_worker_env(kind, tag, path.as_deref(), Uuid::from_u128(1), false, WK),
                "{kind:?}/{tag} should not count"
            );
        }
        // The dedicated worker warmup job, which runs no user code.
        assert!(!dirties_worker_env(
            JobKind::Script,
            "bash",
            Some("u/admin/s"),
            Uuid::nil(),
            false,
            WK
        ));
        // Cancelled, or errored before the executor ran.
        assert!(!dirties_worker_env(
            JobKind::Script,
            "bash",
            Some("u/admin/s"),
            Uuid::from_u128(1),
            true,
            WK
        ));
    }

    /// A job whose queue row vanished mid-execution ran here all the same, and one that
    /// failed left behind whatever it had written before it did.
    #[test]
    fn only_a_cached_result_means_nothing_ran() {
        assert!(!JobOutcome::CompletedFromCache.ran_on_this_worker());
        assert!(JobOutcome::AlreadyCompleted.ran_on_this_worker());
        assert!(JobOutcome::Completed.ran_on_this_worker());
        assert!(JobOutcome::Failed { description: "boom".to_string() }.ran_on_this_worker());
    }

    /// The internal tags are ordinary routing tags: a worker can be configured to pull them,
    /// and the user jobs it then runs must still age its environment.
    #[test]
    fn user_jobs_count_whatever_tag_they_are_routed_with() {
        for (tag, path) in [
            ("bash", Some("u/admin/s")),
            (INIT_SCRIPT_TAG, Some("u/admin/s")),
            (PERIODIC_SCRIPT_TAG, Some("u/admin/s")),
            // Another worker's init script would not be this one's setup either.
            (INIT_SCRIPT_TAG, Some("init_script_wk-default-host-99999")),
            (PERIODIC_SCRIPT_TAG, None),
        ] {
            assert!(
                dirties_worker_env(JobKind::Script, tag, path, Uuid::from_u128(1), false, WK),
                "{tag}/{path:?} should count"
            );
        }
    }
}

pub async fn run_worker(
    conn: &Connection,
    hostname: &str,
    worker_name: String,
    i_worker: u64,
    num_workers: u32,
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

    // Force UNSHARE_PATH and NSJAIL_AVAILABLE initialization now for clear startup logging
    let _ = &*UNSHARE_PATH;
    let _ = &*NSJAIL_AVAILABLE;

    if (is_unshare_enabled() || *FAVOR_UNSHARE_PID) && UNSHARE_PATH.is_none() {
        tracing::error!(
            worker = %worker_name, hostname = %hostname,
            "Worker is configured to use unshare isolation (FAVOR_UNSHARE_PID={}, job_isolation={:?}) \
            but unshare is NOT available. Jobs will run without isolation. \
            See errors above for the specific reason unshare initialization failed.",
            *FAVOR_UNSHARE_PID,
            JobIsolationLevel::from_u8(JOB_ISOLATION.load(std::sync::atomic::Ordering::Relaxed))
        );
    }
    if is_sandboxing_enabled() && NSJAIL_AVAILABLE.is_none() {
        tracing::error!(
            worker = %worker_name, hostname = %hostname,
            "Worker is configured to use nsjail sandboxing but nsjail is NOT available. \
            Jobs requiring sandboxing will fail. \
            See errors above for the specific reason nsjail initialization failed."
        );
    }

    let start_time = Instant::now();

    let worker_dir = format!("{}/{worker_name}", *WINDMILL_DIR);
    tracing::debug!(worker = %worker_name, hostname = %hostname, worker_dir = %worker_dir, "Creating worker dir");

    #[cfg(feature = "python")]
    if !NATIVE_MODE_RESOLVED.load(std::sync::atomic::Ordering::Relaxed) {
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

    #[cfg(all(feature = "python", unix))]
    crate::ansible_executor::prepare_persistent_control_path_root().await;

    if is_sandboxing_enabled() {
        let _ = write_file(
            &worker_dir,
            "download_deps.py.sh",
            INCLUDE_DEPS_PY_SH_CONTENT,
        );
    }

    let mut last_ping = Instant::now() - Duration::from_secs(NUM_SECS_PING + 1);

    let mut reported_ip = cached_ip();
    let previous_jobs_executed = insert_ping(hostname, &worker_name, reported_ip, conn)
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
    // Seeded from the ping row so a worker that reclaimed its name keeps counting from where
    // the previous process left off instead of resetting the total shown for that worker.
    let mut jobs_executed = previous_jobs_executed;
    // Only jobs run by this process count towards EXIT_AFTER_N_JOBS: the point is the age of
    // the environment, not the lifetime total.
    let mut jobs_executed_in_env: u64 = 0;

    let is_dedicated_worker: bool = {
        let config = WORKER_CONFIG.load();
        config.dedicated_worker.is_some()
            || config
                .dedicated_workers
                .as_ref()
                .is_some_and(|dws| !dws.is_empty())
    };

    if let Some(max_jobs) = (*EXIT_AFTER_N_JOBS).filter(|_| i_worker == 1) {
        if num_workers > 1 {
            tracing::warn!(
                worker = %worker_name, hostname = %hostname,
                "EXIT_AFTER_N_JOBS is set but this process runs {num_workers} workers: they share \
                the environment it recycles, so the first one to reach the limit shuts the others \
                down as well, cancelling any job they still run in a container or a dedicated \
                worker. Run a single worker per process instead."
            );
        }
        if is_dedicated_worker {
            tracing::warn!(
                worker = %worker_name, hostname = %hostname,
                "EXIT_AFTER_N_JOBS does not apply to the jobs this worker hands to its dedicated \
                workers: those run outside its main loop and are never counted."
            );
        }
        let config = WORKER_CONFIG.load();
        if config.init_bash.is_some() {
            tracing::warn!(
                worker = %worker_name, hostname = %hostname,
                "EXIT_AFTER_N_JOBS is set and this worker group has an init script: the init \
                script prepares the environment the limit recycles, so it is not counted and runs \
                again on every restart. Every {max_jobs} job(s) therefore pushes and executes an \
                init job of its own first, and waits for it."
            );
        }
        // No interval check: loading a worker config whose periodic script has no interval, or
        // one below MIN_PERIODIC_SCRIPT_INTERVAL_SECONDS, fails and kills the worker, so a
        // script that reaches here is one the periodic task runs.
        if config.periodic_script_bash.is_some() {
            tracing::warn!(
                worker = %worker_name, hostname = %hostname,
                "EXIT_AFTER_N_JOBS is set and this worker group has a periodic script: it runs \
                once when the worker starts, so it runs every {max_jobs} job(s) whatever its \
                interval says."
            );
        }
    }

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

    otel_incr_worker_started();

    let (same_worker_tx, mut same_worker_rx) = mpsc::channel::<SameWorkerPayload>(5);

    let (mut job_completed_tx, job_completed_rx) = JobCompletedSender::new(&conn, 10);
    job_completed_tx.set_worker_killpill(killpill_tx.clone());

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
            false,
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
            WORKER_CONFIG.load()
        );
    }

    // Dedicated workers wait for the init script before installing their dependencies, so it has to
    // be queued before they are spawned.
    if i_worker == 1 {
        // Initialize runtime asset inserter for batched database inserts
        if let Connection::Sql(db) = conn {
            init_runtime_asset_loop(db.clone(), killpill_rx.resubscribe());
        }
        if let Err(e) = queue_init_bash_maybe(conn, same_worker_tx.clone(), &worker_name).await {
            resolve_init_script(InitScriptState::Aborted);
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

    // (dedi_path, dedicated_worker_tx, dedicated_worker_handle)
    // Option<Sender<Arc<QueuedJob>>>,
    // Option<JoinHandle<()>>,

    #[cfg(all(feature = "private", feature = "enterprise"))]
    let (dedicated_workers, dedicated_handles): (
        HashMap<String, Sender<DedicatedWorkerJob>>,
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
        Connection::Http(_) => (HashMap::new(), vec![]),
    };

    #[cfg(any(not(feature = "private"), not(feature = "enterprise")))]
    let (dedicated_workers, dedicated_handles): (
        HashMap<String, Sender<DedicatedWorkerJob>>,
        Vec<JoinHandle<()>>,
    ) = (HashMap::new(), vec![]);

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
            let valid_key = LICENSE_KEY_VALID.load(std::sync::atomic::Ordering::Relaxed);

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

        otel_set_worker_busy(&worker_name, 0);

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

        otel_set_worker_uptime(&worker_name, start_time.elapsed().as_secs_f64());

        // The external IP resolves in the background, after the initial ping. Pinging on the very
        // next iteration rather than the next periodic one is what gets it into the row of a worker
        // whose process is short-lived (EXIT_AFTER_N_JOBS).
        let ip = cached_ip();
        let ip_just_resolved = reported_ip.is_none() && ip.is_some();
        if ip_just_resolved || last_ping.elapsed().as_secs() > NUM_SECS_PING {
            // Servers older than the background lookup take an IP from the initial ping only, so an
            // agent has to register a second time to deliver whatever the lookup settled on, an
            // address or the unretrievable marker. Registering also clears the row's job columns,
            // which costs at most the last job's id here: no job of this worker is in flight at this
            // point in the loop, and the next one refills them.
            if ip_just_resolved && conn.as_sql().is_none() {
                if let Err(e) = insert_ping(hostname, &worker_name, ip, &conn).await {
                    tracing::warn!(
                        worker = %worker_name, hostname = %hostname,
                        "failed to re-register with the resolved external IP: {e}"
                    );
                }
            }

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
                ip,
            )
            .await;

            if read_cgroups {
                last_reading = Instant::now();
            }
            last_ping = Instant::now();
            reported_ip = ip;
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

        let mut was_suspended_job = false;
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
                            || last_suspend_first.elapsed().as_secs_f64() > 5.0
                            || crate::result_processor::WAC_SUSPEND_READY
                                .swap(false, Ordering::Relaxed);

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

                        was_suspended_job = job.as_ref().is_ok_and(|j| j.suspended);
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

                            otel_record_worker_pull_duration(
                                &worker_name,
                                j.job.is_some(),
                                duration_pull_s,
                            );
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

                otel_set_worker_busy(&worker_name, 1);

                occupancy_metrics.running_job_started_at = Some(Instant::now());

                last_executed_job = None;
                jobs_executed += 1;
                let mut dirties_env = dirties_worker_env(
                    job.kind,
                    &job.tag,
                    job.runnable_path.as_deref(),
                    job.id,
                    job.canceled_by.is_some() || job.pre_run_error.is_some(),
                    &worker_name,
                );

                tracing::debug!(target: VERBOSE_TARGET, worker = %worker_name, hostname = %hostname, "started handling of job {}", job.id);

                if matches!(
                    job.kind,
                    JobKind::Script | JobKind::Preview | JobKind::FlowScript
                ) {
                    // A job carrying a pre-run error never runs its code: it only has to be
                    // pulled so `handle_queued_job` can fail it. Both hand-off paths below
                    // dispatch by path and return before that check, so a job sent down them
                    // would run with whatever arguments survived the failure.
                    let fails_before_running = job.pre_run_error.is_some();

                    if !dedicated_workers.is_empty() && !fails_before_running {
                        let dedicated_worker_tx = job.runnable_path.as_ref().and_then(|path| {
                            // For flow steps inside branches/loops, runnable_path includes
                            // nesting segments (e.g. f/flow/branchone-0/a) but the dedicated
                            // worker map is keyed by flow_root/step_id (e.g. f/flow/a).
                            // When nesting segments are present, use flow_root + flow_step_id
                            // to construct the correct key.
                            let key =
                                if let Some(flow_root) = crate::common::extract_flow_root(path) {
                                    let step_id = job.flow_step_id.as_deref().unwrap_or("");
                                    format!("{}:{}/{}", job.workspace_id, flow_root, step_id)
                                } else {
                                    format!("{}:{}", job.workspace_id, path)
                                };
                            dedicated_workers.get(&key)
                        });
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

                    if let Some(flow_runners) = flow_runners.filter(|_| !fails_before_running) {
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
                    if !was_suspended_job {
                        add_outstanding_wait_time(&conn, &job, *OUTSTANDING_WAIT_TIME_THRESHOLD_MS);
                    }

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

                    otel_incr_worker_execution_count(&job.tag);

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

                    let otel_execution_start = Instant::now();

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

                    windmill_common::sensitive_log_masks::register_running_job(arc_job.id);

                    let span = create_span_with_name(&arc_job, &worker_name, Some(hostname), "job");
                    let log_ctx = log_context_for_job(&arc_job, &worker_name, Some(hostname));

                    let job_result = windmill_common::log_context::with_log_context(
                        log_ctx,
                        async {
                            let result = handle_queued_job(
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
                            .await;
                            record_job_span_status(&result);
                            result
                        }
                        .instrument(span),
                    )
                    .await;

                    // A result served from the cache went through the loop without running
                    // anything here.
                    dirties_env &= job_result
                        .as_ref()
                        .map_or(true, JobOutcome::ran_on_this_worker);

                    match job_result {
                        Ok(ref outcome) if !outcome.is_success() && is_init_script => {
                            tracing::error!("init script job failed, exiting");
                            update_worker_ping_for_failed_init_script(conn, &worker_name, job_id)
                                .await;
                            break;
                        }
                        Ok(ref outcome) if !outcome.is_success() && is_periodic_bash_script => {
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

                    windmill_common::sensitive_log_masks::unregister_running_job(job_id);

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

                    otel_record_worker_execution_duration(
                        &tag,
                        otel_execution_start.elapsed().as_secs_f64(),
                    );

                    if !KEEP_JOB_DIR.load(Ordering::Relaxed) && !(is_flow && same_worker) {
                        let _ = tokio::fs::remove_dir_all(job_dir).await;
                    }
                }

                if let Some(max_jobs) = *EXIT_AFTER_N_JOBS {
                    if dirties_env {
                        jobs_executed_in_env += 1;
                    }
                    if jobs_executed_in_env >= max_jobs {
                        // Killpill rather than `break`: the main loop still has to drain the
                        // same-worker jobs it owns (a same-worker flow runs to its end here, past
                        // the limit) and let the background processor persist the results of what
                        // it just ran, before the process goes away. `send` reports whether this is
                        // the shutdown that got scheduled.
                        if killpill_tx.send() {
                            tracing::info!(
                                worker = %worker_name, hostname = %hostname,
                                "executed {jobs_executed_in_env} job(s), EXIT_AFTER_N_JOBS={max_jobs} \
                                reached: shutting the worker process down so it restarts on a fresh environment"
                            );
                        }
                        // `jobs_executed` only reaches the ping row every NUM_SECS_PING, which this
                        // process is about to exit before: force one after every job from here on,
                        // drained ones included, so the row the restarted worker reclaims counts
                        // the jobs this one ran.
                        last_ping = Instant::now() - Duration::from_secs(NUM_SECS_PING + 1);
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

                tokio::time::sleep(Duration::from_millis(sleep_queue())).await;

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
                tokio::time::sleep(Duration::from_millis(sleep_queue() * 5)).await;
            }
        };
    }

    tracing::info!(worker = %worker_name, hostname = %hostname, "worker {} exiting", worker_name);

    // Only this worker runs the init job, so if its loop exited before doing so, nothing ever will:
    // release whoever waits on it, otherwise joining the dedicated worker handles below hangs.
    if i_worker == 1 {
        resolve_init_script(InitScriptState::Aborted);
    }

    #[cfg(feature = "enterprise")]
    {
        let valid_key = LICENSE_KEY_VALID.load(std::sync::atomic::Ordering::Relaxed);

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum InitScriptState {
    Pending,
    Completed,
    Aborted,
}

lazy_static::lazy_static! {
    /// State of the INIT_SCRIPT job, which is the documented hook to prepare the host (CA
    /// certificates, proxies, mounts), so anything reaching the network at startup waits on it. The
    /// init job is executed by the main loop, which only starts once dedicated workers have been
    /// spawned, hence a gate rather than plain ordering. Every path that gives up on running it
    /// MUST resolve the gate, or waiters park forever and worker teardown hangs joining them.
    static ref INIT_SCRIPT_STATE: tokio::sync::watch::Sender<InitScriptState> =
        tokio::sync::watch::channel(InitScriptState::Pending).0;
}

/// Called with the post-processing verdict, which is the only one that accounts for `wm_failure`.
pub(crate) fn init_script_finished(success: bool) {
    resolve_init_script(if success {
        InitScriptState::Completed
    } else {
        InitScriptState::Aborted
    });
}

fn resolve_init_script(state: InitScriptState) {
    INIT_SCRIPT_STATE.send_if_modified(|current| {
        if *current == InitScriptState::Pending {
            *current = state;
            true
        } else {
            false
        }
    });
}

/// Returns false when the init script will never succeed (it failed, or the worker is shutting
/// down), in which case the caller must give up instead of preparing anything.
// Only called from the dedicated worker paths, which are gated behind the `private` feature.
#[allow(dead_code)]
pub(crate) async fn wait_for_init_script_completed(
    killpill_rx: &mut tokio::sync::broadcast::Receiver<()>,
) -> bool {
    let mut rx = INIT_SCRIPT_STATE.subscribe();
    let state = *rx.borrow_and_update();
    if state != InitScriptState::Pending {
        return state == InitScriptState::Completed;
    }
    tracing::info!("waiting for init script to complete before installing dependencies");
    // recv() is cancel-safe, so losing the select does not consume the killpill the caller still
    // needs.
    tokio::select! {
        _ = rx.changed() => *rx.borrow_and_update() == InitScriptState::Completed,
        _ = killpill_rx.recv() => false,
    }
}

#[cfg(test)]
mod init_script_gate_tests {
    use super::*;

    // The gate is a process-global whose first resolution wins, so this must stay the only test
    // that resolves it.
    #[tokio::test]
    async fn aborting_the_init_script_releases_waiters_for_good() {
        let (_killpill_tx, mut killpill_rx) = tokio::sync::broadcast::channel(1);
        resolve_init_script(InitScriptState::Aborted);
        // Parking here instead would hang worker teardown on the dedicated worker handles.
        assert!(!wait_for_init_script_completed(&mut killpill_rx).await);
        resolve_init_script(InitScriptState::Completed);
        assert!(!wait_for_init_script_completed(&mut killpill_rx).await);
    }
}

async fn queue_init_bash_maybe<'c>(
    conn: &Connection,
    same_worker_tx: SameWorkerSender,
    worker_name: &str,
) -> anyhow::Result<bool> {
    let uuid_content = if let Some(content) = WORKER_CONFIG.load().init_bash.clone() {
        let uuid = match conn {
            Connection::Sql(db) => push_init_job(db, content.clone(), worker_name).await?,
            Connection::Http(client) => queue_init_job(client, &content).await?,
        };
        Some((uuid, content))
    } else {
        resolve_init_script(InitScriptState::Completed);
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
        let config = WORKER_CONFIG.load();

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
    pub step_failure: StepFailureKind,
}

/// Why the step a flow is being resumed from failed, which bounds what the engine may do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepFailureKind {
    /// The step failed by running, or did not fail at all.
    Normal,
    /// A suspend gate was disapproved or timed out. The worker that ran the approval step is
    /// still alive, but the failure is recorded against the step the gate was holding back,
    /// which never ran — so that step's `retry` and `continue_on_error` describe nothing that
    /// happened, and honouring them would re-open the gate or skip the step outright.
    /// `suspend.continue_on_disapprove_timeout` is how a flow opts into continuing past a gate.
    SuspendNotApproved,
    /// The step's worker died (OOM/zombie), or the flow status update itself errored. Neither
    /// leaves state worth pinning to: in the first case that worker is gone, in the second the
    /// flow's own bookkeeping is what just broke.
    Unrecoverable,
}

impl StepFailureKind {
    /// Whether the failed module's own `retry` / `continue_on_error` still describe the
    /// failure at hand. When they don't, the failure module is the only way forward.
    pub fn honors_step_error_policy(self) -> bool {
        matches!(self, Self::Normal)
    }

    /// Whether follow-up work may still be pinned to the worker that ran the previous step,
    /// via `same_worker` or dedicated flow-module runners.
    pub fn keeps_worker_pin(self) -> bool {
        !matches!(self, Self::Unrecoverable)
    }
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
) -> windmill_common::error::Result<JobOutcome> {
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
        // Block all dependency jobs: native scripts don't have dependency jobs, and bunnative
        // dep jobs are routed to bun workers (lang=bun, tag=bun) even when they have a custom tag .
        if matches!(
            job.kind,
            JobKind::FlowDependencies | JobKind::AppDependencies | JobKind::Dependencies
        ) || job.tag == "dependency"
        {
            return Err(Error::ExecutionErr(
                "Worker is in native mode and cannot execute dependency jobs".to_string(),
            ));
        }
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

                return Ok(JobOutcome::CompletedFromCache);
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
            Box::pin(handle_flow(
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
                // A freshly pulled flow job is being executed by a live worker; the prior
                // step (if any) completed normally.
                StepFailureKind::Normal,
            ))
            .warn_after_seconds(10)
            .await?;
            Ok(JobOutcome::Completed)
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

        // Skip verbose job header for WAC v2 replays (checkpoint has completed steps)
        let is_wac_replay = if let Connection::Sql(db) = conn {
            crate::wac_executor::load_checkpoint(db, &job.id)
                .await
                .map(|c| !c.completed_steps.is_empty())
                .unwrap_or(false)
        } else {
            false
        };

        if !is_wac_replay {
            logs.push_str(&format!(
                "job={} {}={} worker={} hostname={} isolation={}\n",
                &job.id, *LOG_TAG_NAME, &job.tag, &worker_name, &hostname, isolation_label
            ));
        }

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
            if (windmill_queue::jobs::has_active_concurrency_limit(job.concurrent_limit)
                || windmill_queue::jobs::has_active_concurrency_limit(
                    windmill_common::runnable_settings::prefetch_cached_from_handle(
                        job.runnable_settings_handle,
                        db,
                    )
                    .await?
                    .1
                    .concurrent_limit,
                ))
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
        // Set by the dependency handlers once they reach `handle_deployment_metadata`,
        // so the fallback tally below never counts the same deploy twice.
        let mut deployment_tallied = false;
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
                        &mut deployment_tallied,
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
                        &mut deployment_tallied,
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
                    &mut deployment_tallied,
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

        // A lock generation that failed or was cancelled still leaves the deployed
        // version live in the workspace, so its fork/parent change must be tallied.
        // `AlreadyCompleted` is not such a failure — another worker owns the job.
        if job.kind.is_dependency()
            && (result
                .as_ref()
                .is_err_and(|err| !matches!(err, &Error::AlreadyCompleted(_)))
                || canceled_by.is_some())
        {
            if let Connection::Sql(db) = conn {
                tally_unfinished_dependency_deploy(db, job.as_ref(), &mut deployment_tallied).await;
            }
        }

        let cjob = MiniCompletedJob::from(job.to_owned());
        drop(job);
        //it's a test job, no need to update the db
        if cjob.workspace_id == "" {
            return Ok(JobOutcome::Completed);
        }

        if result
            .as_ref()
            .is_err_and(|err| matches!(err, &Error::AlreadyCompleted(_)))
        {
            return Ok(JobOutcome::AlreadyCompleted);
        }
        if result
            .as_ref()
            .is_err_and(|err| matches!(err, &Error::WacSuspended(_)))
        {
            // WAC v2 job suspended while waiting for child jobs — don't complete it
            return Ok(JobOutcome::Completed);
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
    pub modules: Option<std::collections::HashMap<String, ScriptModule>>,
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
        modules: None,
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
        modules: data.modules.clone(),
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

/// Pipeline partition resolution at execution time. The script content is
/// already loaded for this job, so parsing the `// partitioned` annotation
/// here is free (no extra fetch / no DB column). The concrete partition
/// value is resolved exactly once — schedule fire-time for time kinds
/// (anchored on `scheduled_for`, NOT wall-clock, so a chain crossing
/// midnight stays coherent) or the triggering payload for `dynamic`. It is
/// then (a) injected into the in-memory args the body sees and (b)
/// persisted back to `v2_job.args` so the asset-dispatch cascade reads the
/// same value at completion and propagates it downstream (run identity is
/// immutable — never re-resolve once set).
///
/// `Ok(Some(job))` = a value was injected (caller must use the returned
/// clone). `Ok(None)` = nothing to do (no `// partitioned`, or the
/// partition is already set: explicit / backfill / cascade-propagated, or
/// before the `start` anchor). `Err` fails the job with a clear message
/// (partitioned but unresolvable — e.g. `dynamic` with no payload).
async fn resolve_partition_for_job(
    job: &MiniPulledJob,
    code: &str,
    conn: &Connection,
) -> error::Result<(Option<MiniPulledJob>, bool)> {
    use windmill_common::partition::{resolve_partition, PARTITION_ARG};
    use windmill_parser::asset_parser::PartitionKind;

    // Only deployed scripts participate in asset pipelines. Cheap substring
    // guard so the overwhelming majority of script jobs skip the annotation
    // scan; when one might be present we parse *once* here and reuse the result
    // for both `in_pipeline` (→ WM_PIPELINE env, read by the wmll.ducklake SDK to
    // record state) and `partition` resolution — no second parse downstream. The
    // bool is whether the script is a `// pipeline` member.
    if !matches!(job.kind, JobKind::Script)
        || !(code.contains("pipeline") || code.contains("partitioned"))
    {
        return Ok((None, false));
    }
    let ann = windmill_parser::asset_parser::parse_pipeline_annotations(code);
    let in_pipeline = ann.in_pipeline;
    let Some(spec) = ann.partition else {
        return Ok((None, in_pipeline));
    };

    // Already resolved upstream — explicit run arg, backfill, or
    // cascade-propagated (push_subscriber injects a top-level `partition`).
    // Run identity is immutable: use it as-is, do not re-resolve.
    let already_set = job.args.as_ref().is_some_and(|a| {
        a.0.get(PARTITION_ARG)
            .and_then(|v| serde_json::from_str::<String>(v.get()).ok())
            .is_some_and(|s| !s.is_empty())
    });
    if already_set {
        return Ok((None, in_pipeline));
    }

    // `dynamic` extracts from the triggering payload (the `trigger` object
    // for a cascade/event hop, else the run args themselves). Time kinds
    // ignore the payload.
    let payload: Option<serde_json::Value> = match &spec.kind {
        PartitionKind::Dynamic { .. } => job.args.as_ref().map(|a| {
            a.0.get("trigger")
                .and_then(|t| serde_json::from_str::<serde_json::Value>(t.get()).ok())
                .unwrap_or_else(|| {
                    serde_json::Value::Object(
                        a.0.iter()
                            .filter_map(|(k, v)| {
                                serde_json::from_str(v.get()).ok().map(|jv| (k.clone(), jv))
                            })
                            .collect(),
                    )
                })
        }),
        _ => None,
    };

    let resolved = resolve_partition(&spec, job.scheduled_for, payload.as_ref())
        .map_err(|e| Error::ExecutionErr(format!("partition resolution failed: {e:#}")))?;
    let Some(value) = resolved else {
        // Before the `start` anchor: this run has no partition to
        // materialize. v1 runs it without one (logged) rather than
        // introducing a skip-the-queued-job mechanism.
        tracing::warn!(
            job_id = %job.id,
            "partitioned script resolved to no partition (before start anchor); running without one"
        );
        return Ok((None, in_pipeline));
    };

    // Persist back so dispatch_asset_triggers (which reads the producer's
    // completed v2_job.args) propagates the same value down the cascade.
    if let Some(db) = conn.as_sql() {
        windmill_common::partition::set_resolved_partition(db, job.id, &value).await?;
    } else {
        tracing::warn!(
            job_id = %job.id,
            "agent worker: resolved partition not persisted; downstream cascade will not propagate it"
        );
    }

    // Inject into the in-memory args so the running body sees it.
    let mut updated = job.clone();
    let mut map = updated.args.take().map(|j| j.0).unwrap_or_default();
    map.insert(
        PARTITION_ARG.to_string(),
        windmill_common::worker::to_raw_value(&value),
    );
    updated.args = Some(Json(map));
    Ok((Some(updated), in_pipeline))
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
        ScriptData { code, lock, modules: modules_from_data },
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
            let ContentReqLangEnvs { content, lockfile, language, envs, codebase, schema, .. } =
                Box::pin(get_hub_script_content_and_requirements(
                    job.runnable_path.as_ref(),
                    conn.as_sql(),
                ))
                .await?;

            data = ScriptData { code: content, lock: lockfile, modules: None };
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
                    let ContentReqLangEnvs {
                        content,
                        lockfile,
                        language,
                        envs,
                        codebase,
                        schema,
                        ..
                    } = Box::pin(get_hub_script_content_and_requirements(
                        Some(script_path),
                        conn.as_sql(),
                    ))
                    .await?;
                    data = ScriptData { code: content, lock: lockfile, modules: None };
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

    // Pipeline partition resolution: the content is now loaded, so resolve
    // `// partitioned` (if any) and shadow `job` with a clone whose args
    // carry the resolved `partition` for the rest of execution.
    let _job_with_partition;
    let (resolved_job, in_pipeline) = resolve_partition_for_job(job, code, conn).await?;
    let job = match resolved_job {
        Some(j) => {
            _job_with_partition = j;
            &_job_with_partition
        }
        None => job,
    };

    // Any job kind, not just previews: whatever is here is what gets written to the job dir
    // and built in, so the agent-worker server precomputing a cache name has to resolve
    // modules the same way (`windmill-api-agent-workers`, `get_code_and_lock`).
    let modules = modules_from_data.clone().or_else(|| {
        job.args.as_ref().and_then(|args| {
            args.get("_MODULES").and_then(|raw| {
                serde_json::from_str::<std::collections::HashMap<String, ScriptModule>>(raw.get())
                    .ok()
            })
        })
    });

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
        &modules,
        false,
        in_pipeline,
    )
    .await
}

/// True when `path` contains only `Normal`/`CurDir` components, i.e. it cannot
/// escape the directory it is joined onto (no `..`, no absolute root, no Windows
/// drive prefix).
fn is_contained_relative_path(path: &str) -> bool {
    use std::path::Component;
    std::path::Path::new(path)
        .components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

pub async fn write_module_files(
    job_dir: &str,
    modules: &std::collections::HashMap<String, ScriptModule>,
    base_dir: Option<&str>,
) -> error::Result<()> {
    // base_dir is derived from the runnable path, which on a preview run can
    // carry `..` traversal (it is not the validated module-map key). Reject it
    // before it is used to build any write target, otherwise a module could
    // escape job_dir and write arbitrary files.
    if let Some(dir) = base_dir {
        if !is_contained_relative_path(dir) {
            return Err(error::Error::BadRequest(format!(
                "Invalid module base directory (path traversal): {dir}"
            )));
        }
    }
    for (relpath, module) in modules {
        // Reject path traversal attempts in module paths (the module-map key).
        if !is_contained_relative_path(relpath) {
            tracing::warn!("Skipping module with path traversal: {relpath}");
            continue;
        }
        let relpath_from_job_dir = match base_dir {
            Some(dir) => format!("{}/{}", dir, relpath),
            None => relpath.to_string(),
        };
        // Authoritative guard: resolve the path and assert it stays inside job_dir.
        let full_path = is_allowed_file_location(job_dir, &relpath_from_job_dir)?;
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        // For Python modules, create __init__.py in each intermediate directory
        // between base_dir and the module's parent so that relative imports work.
        if let Some(dir) = base_dir {
            let mut current = std::path::PathBuf::from(dir);
            for component in std::path::Path::new(relpath)
                .parent()
                .into_iter()
                .flat_map(|p| p.components())
            {
                current = current.join(component);
                let init_py = is_allowed_file_location(
                    job_dir,
                    &current.join("__init__.py").to_string_lossy(),
                )?;
                if !init_py.exists() {
                    tokio::fs::write(&init_py, "").await?;
                }
            }
        }
        tracing::debug!("Writing module file: {}", full_path.display());
        tokio::fs::write(&full_path, &module.content).await?;
    }
    Ok(())
}

#[cfg(test)]
mod byte_size_tests {
    use super::*;

    /// The suffix table is order-sensitive: `GB` matches the tail of `GIB` and
    /// `B` the tail of every other suffix, so a reordering silently truncates
    /// the multiplier instead of failing to parse.
    #[test]
    fn suffixes_do_not_shadow_each_other() {
        assert_eq!(parse_byte_size("512"), Some(512));
        assert_eq!(parse_byte_size("512B"), Some(512));
        assert_eq!(parse_byte_size("512KB"), Some(512 << 10));
        assert_eq!(parse_byte_size("300mb"), Some(300 << 20));
        assert_eq!(parse_byte_size("2GiB"), Some(2 << 30));
        assert_eq!(parse_byte_size("2 gb "), Some(2 << 30));
        assert_eq!(parse_byte_size("many"), None);
    }

    /// The duckdb error renders the limit so it can be pasted straight into
    /// `MAX_SQL_RESULT_SIZE`. That renderer lives in the FFI crate and emits a
    /// fraction above 1 GiB, so an integer-only parser here would silently reject
    /// the very value the error told the user to set.
    #[test]
    fn the_shapes_the_error_renders_all_parse() {
        for rendered in ["512B", "8KB", "307MB", "1.0GB", "2.4GB"] {
            assert!(
                parse_byte_size(rendered).is_some(),
                "{rendered} is rendered into errors but does not parse back"
            );
        }
    }

    /// Pins the property the budgeted writer exists for: a value is charged by
    /// its size in memory, and escaping makes the serialized form diverge from
    /// that by up to 6x.
    #[test]
    fn escaping_cannot_outgrow_the_budget() {
        // One control character in, six bytes of `\u0001` out.
        let value = serde_json::json!({ "a": "\u{1}".repeat(1000) });
        let serialized_len = serde_json::to_string(&value).unwrap().len();
        assert!(
            serialized_len > 6000,
            "expected escaping to expand: {serialized_len}"
        );

        // A budget that the unescaped bytes would clear, and the escaped ones cannot.
        assert!(to_raw_value_within(&value, 2000).is_none());

        // The `RawValue` is built without re-parsing, so nothing but this check
        // stands between a serializer change and a malformed value being handed
        // out as valid JSON. Escaped text is the case most likely to expose it.
        let fitted = to_raw_value_within(&value, serialized_len).expect("fits its own length");
        assert_eq!(fitted.get(), serde_json::to_string(&value).unwrap());
    }

    /// The error quotes the limit so it can be set verbatim, which only holds if
    /// the rendered figure means the same number of bytes. Rounding to a unit
    /// that does not divide it evenly parses fine and still names a different
    /// limit, so parseability alone is not the property worth pinning.
    #[test]
    fn every_rendered_limit_round_trips_exactly() {
        for bytes in [512, 8 << 20, 307 << 20, 2 << 30, 2_576_980_377, 16 << 30] {
            let rendered = format_byte_size(bytes);
            assert_eq!(
                parse_byte_size(&rendered),
                Some(bytes),
                "format_byte_size({bytes}) = {rendered:?}, which names a different limit"
            );
        }
    }
}

#[cfg(test)]
mod write_module_files_tests {
    use super::*;
    use std::collections::HashMap;
    use windmill_common::scripts::ScriptLang;
    use windmill_common::utils::calculate_hash;

    fn module(content: &str) -> ScriptModule {
        ScriptModule { content: content.to_string(), language: ScriptLang::Python3, lock: None }
    }

    /// Every language's artifact cache name funnels module content through this, so an
    /// ambiguous encoding puts two different runnables back on one name.
    #[test]
    fn artifact_name_cannot_be_re_cut_into_another_module_map() {
        fn name(entries: &[(&str, &str)]) -> String {
            let map: HashMap<String, ScriptModule> = entries
                .iter()
                .map(|(p, c)| (p.to_string(), module(c)))
                .collect();
            artifact_cache_name("base".to_string(), Some(&map))
        }

        // Naive `path + content` concatenation renders both of these as "abc".
        assert_ne!(name(&[("a", "bc")]), name(&[("ab", "c")]));
        // Splitting one module into two must not read back as the joined one.
        assert_ne!(name(&[("a", "b"), ("c", "d")]), name(&[("ac", "bd")]));
        // Iteration order of the map must not move the name.
        assert_eq!(
            name(&[("a", "1"), ("b", "2")]),
            name(&[("b", "2"), ("a", "1")])
        );
    }

    /// A module-free runnable must keep the exact name it had before modules entered the
    /// derivation, or upgrading strands every artifact already in the cache.
    #[test]
    fn artifact_name_is_unchanged_without_modules() {
        assert_eq!(
            artifact_cache_name("code+lock".to_string(), None),
            calculate_hash("code+lock")
        );
        assert_eq!(
            artifact_cache_name("code+lock".to_string(), Some(&HashMap::new())),
            calculate_hash("code+lock")
        );
    }

    /// A preview brings its own source and lockfile, so a module-free runnable picks its
    /// whole `base`. Module-bearing names live in their own namespace precisely so that no
    /// crafted `base` can be made to land on one.
    #[test]
    fn a_module_free_runnable_cannot_forge_a_module_bearing_name() {
        let modules = HashMap::from([("h.ts".to_string(), module("evil"))]);
        let victim = artifact_cache_name("code+lock".to_string(), Some(&modules));

        // `calculate_hash` emits hex, so the namespace is unreachable however `base` is
        // chosen — including by feeding it the victim's own name.
        assert!(victim.starts_with("mod-"));
        assert_ne!(artifact_cache_name(victim.clone(), None), victim);
        assert!(!artifact_cache_name("anything".to_string(), None).starts_with("mod-"));
    }

    /// The `mod-` namespace separates module-free from module-bearing, and nothing separates
    /// two module-bearing runnables — only the seal does. Unsealed, `base` is variable-width,
    /// so the split between it and the module block is ambiguous and a preview (which brings
    /// its own source *and* lockfile) can absorb part of another runnable's block.
    #[test]
    fn a_module_bearing_runnable_cannot_absorb_another_ones_block() {
        let victim = artifact_cache_name(
            "V".to_string(),
            Some(&HashMap::from([(
                "h.ts".to_string(),
                module(":modules:1:1:a:1:b"),
            )])),
        );
        // Byte-identical to the victim's without the seal: the forger's `base` spells out the
        // victim's leading block, leaving its own single module to supply the tail.
        let forged = artifact_cache_name(
            "V:modules:1:4:h.ts:18:".to_string(),
            Some(&HashMap::from([("a".to_string(), module("b"))])),
        );

        assert_ne!(victim, forged);
    }

    /// Deploy fills a module's lock in after the parent has prebuilt, so a name that moved
    /// with it would leave every prebuilt artifact unreachable by the runs it was built for.
    #[test]
    fn artifact_name_ignores_the_lock_deploy_fills_in_later() {
        let prebuild = artifact_cache_name(
            "base".to_string(),
            Some(&HashMap::from([("h.ts".to_string(), module("x"))])),
        );

        let mut locked = module("x");
        locked.lock = Some("{}\n//bun.lock\n<empty>".to_string());
        let after_deploy = artifact_cache_name(
            "base".to_string(),
            Some(&HashMap::from([("h.ts".to_string(), locked)])),
        );

        assert_eq!(prebuild, after_deploy);
    }

    #[test]
    fn contained_relative_path_rejects_traversal_and_absolute() {
        assert!(is_contained_relative_path("u/admin/pkg"));
        assert!(is_contained_relative_path("./pkg/sub"));
        // A `..` in a filename is a valid name, not a traversal.
        assert!(is_contained_relative_path("weird..name"));

        assert!(!is_contained_relative_path("u/x/../../../etc"));
        assert!(!is_contained_relative_path("../escape"));
        assert!(!is_contained_relative_path("/etc/cron.d/wm"));
    }

    #[tokio::test]
    async fn base_dir_traversal_is_rejected_and_writes_nothing() {
        let job = tempfile::tempdir().unwrap();
        let job_dir = job.path().to_str().unwrap();
        // Sentinel just outside job_dir that a successful traversal would create.
        let outside = job.path().parent().unwrap().join("wm_escaped_marker");

        let mut modules = HashMap::new();
        modules.insert(
            "wm_escaped_marker".to_string(),
            module("* * * * * root id\n"),
        );

        // base_dir derived from a preview path carrying `..` traversal.
        let res = write_module_files(job_dir, &modules, Some("u/x/../../../../../..")).await;
        assert!(res.is_err(), "traversal base_dir must be rejected");
        assert!(!outside.exists(), "no file may be written outside job_dir");
    }

    #[tokio::test]
    async fn relpath_traversal_is_skipped() {
        let job = tempfile::tempdir().unwrap();
        let job_dir = job.path().to_str().unwrap();
        let outside = job.path().parent().unwrap().join("wm_relpath_escape.py");

        let mut modules = HashMap::new();
        modules.insert("../wm_relpath_escape.py".to_string(), module("x = 1"));

        write_module_files(job_dir, &modules, None).await.unwrap();
        assert!(!outside.exists());
    }

    #[tokio::test]
    async fn legitimate_modules_are_written_with_init_py() {
        let job = tempfile::tempdir().unwrap();
        let job_dir = job.path().to_str().unwrap();

        let mut modules = HashMap::new();
        modules.insert("pkg/sub/mod.py".to_string(), module("VALUE = 42"));

        write_module_files(job_dir, &modules, Some("u/admin"))
            .await
            .unwrap();

        let base = job.path().join("u/admin");
        assert_eq!(
            std::fs::read_to_string(base.join("pkg/sub/mod.py")).unwrap(),
            "VALUE = 42"
        );
        assert!(base.join("pkg/__init__.py").exists());
        assert!(base.join("pkg/sub/__init__.py").exists());
    }
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
    modules: &Option<std::collections::HashMap<String, ScriptModule>>,
    run_inline: bool,
    // Whether the script is a `// pipeline` member (parsed once upstream) — sets
    // WM_PIPELINE so the wmll.ducklake SDK helpers record materialization state.
    in_pipeline: bool,
) -> error::Result<Box<RawValue>> {
    // Defense-in-depth (GHSA-wxjq-w5pj-jqhx): the entrypoint override is
    // interpolated verbatim into a code position of the generated language
    // wrappers below. It originates from the `_ENTRYPOINT_OVERRIDE` job arg,
    // which any caller with `jobs:run` can set on a deployed script, so reject
    // anything that is not a strict identifier before it reaches any wrapper.
    if let Some(entrypoint) = job.script_entrypoint_override.as_deref() {
        if !windmill_common::jobs::is_valid_entrypoint_name(entrypoint) {
            return Err(Error::BadRequest(format!(
                "Invalid entrypoint override {entrypoint:?}: must match \
                 ^[A-Za-z_][A-Za-z0-9_]*$ (letters, digits and underscores, \
                 not starting with a digit)"
            )));
        }
    }

    // Expand WM_INTERNAL_DB markers into real SQL before dispatching
    let expanded_code: String;
    let mut language = language;
    let code = if let Some(ref lang) = language {
        match windmill_common::query_builders::try_expand_internal_db_query(code, lang) {
            Some(Ok(expanded)) => {
                if let Some(lang_override) = expanded.language_override {
                    language = Some(lang_override);
                }
                expanded_code = expanded.code;
                &expanded_code
            }
            Some(Err(e)) => {
                return Err(Error::ExecutionErr(format!(
                    "Failed to expand WM_INTERNAL_DB marker: {}",
                    e
                )));
            }
            None => code, // Not a marker, use original code
        }
    } else {
        code
    };
    if let Some(modules) = modules {
        #[cfg(feature = "python")]
        let base_dir = if language == Some(ScriptLang::Python3) {
            let script_path = crate::common::use_flow_root_path(job.runnable_path());
            Some(crate::python_executor::compute_python_module_dir(
                &script_path,
            ))
        } else {
            None
        };
        #[cfg(not(feature = "python"))]
        let base_dir: Option<String> = None;
        write_module_files(job_dir, modules, base_dir.as_deref()).await?;
    }

    if language == Some(ScriptLang::Postgresql) {
        return Box::pin(do_postgresql(
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
        ))
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
            return Box::pin(do_mysql(
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
            ))
            .await;
        }
    } else if language == Some(ScriptLang::Bigquery) {
        #[cfg(not(feature = "bigquery"))]
        {
            return Err(Error::internal_err(
                "Bigquery requires the bigquery feature to be enabled".to_string(),
            ));
        }

        #[cfg(feature = "bigquery")]
        {
            if run_inline {
                return Err(Error::internal_err(
                    "Inline execution is not yet supported for this language".to_string(),
                ));
            }
            return Box::pin(do_bigquery(
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
            ))
            .await;
        }
    } else if language == Some(ScriptLang::Snowflake) {
        #[cfg(not(feature = "snowflake"))]
        {
            return Err(Error::internal_err(
                "Snowflake requires the snowflake feature to be enabled".to_string(),
            ));
        }

        #[cfg(feature = "snowflake")]
        {
            if run_inline {
                return Err(Error::internal_err(
                    "Inline execution is not yet supported for this language".to_string(),
                ));
            }
            return Box::pin(do_snowflake(
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
            ))
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
            return Box::pin(do_mssql(
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
            ))
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
            return Box::pin(do_oracledb(
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
            ))
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
            return Box::pin(do_duckdb(
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
                job_dir,
                run_inline,
            ))
            .await;
        }
    } else if language == Some(ScriptLang::Graphql) {
        if run_inline {
            return Err(Error::internal_err(
                "Inline execution is not yet supported for this language".to_string(),
            ));
        }
        return Box::pin(do_graphql(
            job,
            &client,
            &code,
            conn,
            mem_peak,
            canceled_by,
            worker_name,
            occupancy_metrics,
        ))
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
                .map(|(k, v)| {
                    let escaped = windmill_common::variables::escape_js_single_quoted(v);
                    let key_literal = windmill_common::variables::escape_js_single_quoted(k);
                    if windmill_common::variables::can_bind_as_prologue_const(k) {
                        format!("const {k} = '{escaped}';\nprocess.env['{key_literal}'] = '{escaped}';\n")
                    } else {
                        // Names that can't safely bind to `const {k}` (non-identifiers,
                        // reserved words, prologue-owned names) are exposed only through
                        // process.env with the key escaped — a `const` for them would be
                        // an injection or a SyntaxError.
                        format!("process.env['{key_literal}'] = '{escaped}';\n")
                    }
                })
                .collect::<Vec<String>>()
                .join("\n"));

        let result = Box::pin(do_nativets(
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
        ))
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

    #[allow(unused_mut)]
    let mut shared_mount = if job.same_worker && job.script_lang != Some(ScriptLang::Deno) {
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

    #[allow(unused_mut)]
    let mut envs = build_envs(envs.as_ref())?;
    // Signal pipeline context to the script so the wmll.ducklake SDK helpers
    // record materialization state (the grid/backfill) and skip it otherwise.
    if in_pipeline {
        envs.insert("WM_PIPELINE".to_string(), "true".to_string());
    }

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

    // Volume mount setup (requires workspace S3 storage; CE has file count/size limits)
    #[cfg(feature = "parquet")]
    let volume_mounts = {
        let comment_prefix = match language {
            ScriptLang::Python3
            | ScriptLang::Bash
            | ScriptLang::Powershell
            | ScriptLang::Ansible
            | ScriptLang::Ruby
            | ScriptLang::Rlang => "#",
            ScriptLang::Deno
            | ScriptLang::Bun
            | ScriptLang::Bunnative
            | ScriptLang::Nativets
            | ScriptLang::Go => "//",
            _ => "",
        };
        let raw_mounts = windmill_worker_volumes::parse_volume_annotations(&code, comment_prefix);
        let args_ref = job.args.as_ref().map(|a| &**a);
        let mut interpolated = Vec::new();
        for mut v in raw_mounts {
            v.name = windmill_worker_volumes::interpolate_volume_name(
                &v.name,
                args_ref,
                &job.workspace_id,
            );
            if let Err(e) = windmill_worker_volumes::validate_volume_name(&v.name) {
                return Err(Error::ExecutionErr(e));
            }
            if let Err(e) = windmill_worker_volumes::validate_volume_target(&v.target) {
                return Err(Error::ExecutionErr(e));
            }
            interpolated.push(v);
        }
        if let Err(e) = windmill_worker_volumes::validate_volume_mounts(&interpolated) {
            return Err(Error::ExecutionErr(e));
        }
        interpolated
    };

    #[cfg(feature = "parquet")]
    let mut volume_setup = crate::volume_oss::VolumeSetupResult {
        states: Vec::new(),
        writable: Vec::new(),
        client: None,
        lease_renewal: crate::volume_oss::LeaseRenewalGuard(None),
    };

    #[cfg(feature = "parquet")]
    if !volume_mounts.is_empty() {
        let vol_summary: Vec<String> = volume_mounts
            .iter()
            .map(|v| format!("'{}' -> {}", v.name, v.target))
            .collect();
        append_logs(
            &job.id,
            &job.workspace_id,
            format!(
                "\n--- VOLUME MOUNTS ---\nPulling {} volume(s): {}\n",
                volume_mounts.len(),
                vol_summary.join(", "),
            ),
            conn,
        )
        .await;

        if let Connection::Sql(db) = conn {
            volume_setup = crate::volume_oss::setup_volumes_sql_worker(
                &volume_mounts,
                db,
                &job.workspace_id,
                job.id,
                &job.permissioned_as,
                worker_name,
                job_dir,
                client,
                conn,
                language,
                &mut envs,
                &mut shared_mount,
            )
            .await?;
        } else if let Connection::Http(http) = conn {
            volume_setup = crate::volume_oss::setup_volumes_http_worker(
                &volume_mounts,
                http,
                &job.workspace_id,
                job.id,
                &job.permissioned_as,
                &job.canceled_by,
                worker_name,
                job_dir,
                conn,
                language,
                &mut envs,
                &mut shared_mount,
            )
            .await?;
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
                    modules,
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
                modules,
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
                modules.as_ref(),
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
            let maybe_lock = resolve_maybe_lock(
                &lock,
                &code,
                language,
                &job.workspace_id,
                job.runnable_path(),
                conn.clone(),
            )
            .await?;
            Box::pin(handle_powershell_job(
                maybe_lock,
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
                    modules.as_ref(),
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
                modules.as_ref(),
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
                    modules: modules.as_ref(),
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
        ScriptLang::Rlang => {
            #[cfg(not(feature = "rlang"))]
            return Err(
                anyhow::anyhow!("R is not available because the feature is not enabled").into(),
            );

            #[cfg(feature = "rlang")]
            {
                if run_inline {
                    return Err(Error::internal_err(
                        "Inline execution is not yet supported for this language".to_string(),
                    ));
                }
                Box::pin(handle_r_job(JobHandlerInputRlang {
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
        ScriptLang::Dbt => {
            if run_inline {
                return Err(Error::internal_err(
                    "Inline execution is not yet supported for this language".to_string(),
                ));
            }
            Box::pin(crate::dbt_executor::handle_dbt_job(
                lock.as_ref(),
                job_dir,
                worker_name,
                job,
                mem_peak,
                canceled_by,
                conn,
                client,
                &code,
                envs,
                occupancy_metrics,
                // The project's identity is derived from these, so the dbt
                // executor needs them even though they are already on disk.
                modules.as_ref(),
            ))
            .await
        }
        // for related places search: ADD_NEW_LANG
        _ => panic!("unreachable, language is not supported: {language:#?}"),
    };
    // Volume sync-back and lease release
    #[cfg(feature = "parquet")]
    if !volume_setup.states.is_empty() {
        // Stop lease renewal before sync-back
        volume_setup.lease_renewal.0.take().map(|h| h.abort());

        if let Some(ref vol_client) = volume_setup.client {
            if let Connection::Sql(db) = conn {
                crate::volume_oss::sync_volumes_sql_worker(
                    &volume_setup.states,
                    &volume_setup.writable,
                    vol_client,
                    db,
                    &job.workspace_id,
                    job.id,
                    worker_name,
                    conn,
                    result.is_ok(),
                )
                .await;
            }
        }

        if let Connection::Http(http) = conn {
            crate::volume_oss::sync_volumes_http_worker(
                &volume_setup.states,
                &volume_setup.writable,
                http,
                &job.workspace_id,
                job.id,
                worker_name,
                conn,
                result.is_ok(),
            )
            .await;
        }

        // Clean up absolute-path symlinks created by setup_volume_mount_paths
        if !is_sandboxing_enabled() {
            #[allow(unused_variables)] // state is only used on unix
            for state in &volume_setup.states {
                #[cfg(unix)]
                if state.mount.target.starts_with('/') {
                    let target_path = std::path::Path::new(&state.mount.target);
                    if target_path
                        .symlink_metadata()
                        .map(|m| m.file_type().is_symlink())
                        .unwrap_or(false)
                    {
                        std::fs::remove_file(target_path).ok();
                    }
                }
            }
        }
    }

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
            #[cfg(feature = "rlang")]
            ScriptLang::Rlang => Some(windmill_parser_r::parse_r_signature(code)?),
            #[cfg(not(feature = "rlang"))]
            ScriptLang::Rlang => None,
            ScriptLang::Dbt => Some(windmill_parser_yaml::parse_dbt_sig(code)?),
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
            let job = MiniPulledJob::new_inline(
                params.workspace_id,
                params.args,
                params.created_by,
                params.permissioned_as,
                params.permissioned_as_email,
                None,
                JobKind::Preview,
                None,
                "inline_preview".to_string(),
                Some(params.lang),
            );
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
                    &None,
                    true,
                    false,
                )
                .await
            })
        }),
        run_inline_script: Arc::new(|params: RunInlineScriptFnParams| {
            Box::pin(async move {
                let (script_hash, runnable_path) = match params.target {
                    InlineScriptTarget::Path(ref path) => {
                        let db = params
                            .conn
                            .as_sql()
                            .ok_or_else(|| {
                                error::Error::InternalErr(
                                    "run_inline_script by path requires a SQL connection"
                                        .to_string(),
                                )
                            })?
                            .clone();
                        let authed_ref = params.user_db.as_ref().map(|(_, a)| a.to_authed_ref());
                        let user_db_authed =
                            params.user_db.as_ref().zip(authed_ref.as_ref()).map(
                                |((udb, _), ar)| UserDbWithAuthed { db: udb.clone(), authed: ar },
                            );
                        let script_hash_info = get_latest_deployed_hash_for_path(
                            user_db_authed,
                            db,
                            &params.workspace_id,
                            path,
                        )
                        .await?;
                        (ScriptHash(script_hash_info.hash), Some(path.clone()))
                    }
                    InlineScriptTarget::Hash(hash) => (ScriptHash(hash), None),
                };
                let content_info =
                    get_script_content_by_hash(&script_hash, &params.workspace_id, &params.conn)
                        .await?;
                let job = MiniPulledJob::new_inline(
                    params.workspace_id,
                    params.args,
                    params.created_by,
                    params.permissioned_as,
                    params.permissioned_as_email,
                    runnable_path,
                    JobKind::Script,
                    Some(script_hash),
                    "inline_run".to_string(),
                    content_info.language,
                );
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
                    content_info.language,
                    &content_info.content,
                    &content_info.envs,
                    &content_info.codebase,
                    &content_info.lockfile,
                    &content_info.modules,
                    true,
                    false,
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
