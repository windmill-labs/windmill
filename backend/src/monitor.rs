#[cfg(feature = "oauth2")]
use std::collections::HashMap;
use std::{
    fmt::Display,
    ops::Mul,
    str::FromStr,
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use chrono::{DateTime, NaiveDateTime, Utc};
use futures::{stream::FuturesUnordered, StreamExt};
use serde::{de::DeserializeOwned, Deserialize};
use sqlx::{Pool, Postgres};
use tokio::{
    join,
    sync::{mpsc, RwLock},
    time::timeout,
};
use uuid::Uuid;

#[cfg(feature = "embedding")]
use windmill_api::embeddings::update_embeddings_db;
use windmill_api::{
    jobs::TIMEOUT_WAIT_RESULT, DEFAULT_BODY_LIMIT, IS_SECURE, REQUEST_SIZE_LIMIT, SAML_METADATA,
    SCIM_TOKEN,
};

#[cfg(feature = "native_trigger")]
use windmill_api::native_triggers::sync::sync_all_triggers;

#[cfg(feature = "enterprise")]
use windmill_common::ee_oss::low_disk_alerts;
#[cfg(feature = "enterprise")]
use windmill_common::ee_oss::{jobs_waiting_alerts, worker_groups_alerts};

#[cfg(feature = "oauth2")]
use windmill_common::global_settings::OAUTH_SETTING;
#[cfg(feature = "parquet")]
use windmill_object_store::reload_object_store_setting;
use windmill_common::{
    agent_workers::DECODED_AGENT_TOKEN,
    apps::APP_WORKSPACED_ROUTE,
    auth::create_token_for_owner,
    ee_oss::CriticalErrorChannel,
    error,
    flow_status::{FlowStatus, FlowStatusModule},
    global_settings::{
        BASE_URL_SETTING, BUNFIG_INSTALL_SCOPES_SETTING, CRITICAL_ALERTS_ON_DB_OVERSIZE_SETTING,
        CRITICAL_ALERT_MUTE_UI_SETTING, CRITICAL_ERROR_CHANNELS_SETTING,
        DEFAULT_TAGS_PER_WORKSPACE_SETTING, DEFAULT_TAGS_WORKSPACES_SETTING,
        EXPOSE_DEBUG_METRICS_SETTING, EXPOSE_METRICS_SETTING, EXTRA_PIP_INDEX_URL_SETTING,
        HUB_API_SECRET_SETTING, HUB_BASE_URL_SETTING, INSTANCE_PYTHON_VERSION_SETTING,
        JOB_DEFAULT_TIMEOUT_SECS_SETTING, JOB_ISOLATION_SETTING, JWT_SECRET_SETTING,
        KEEP_JOB_DIR_SETTING, LICENSE_KEY_SETTING, MONITOR_LOGS_ON_OBJECT_STORE_SETTING,
        NPM_CONFIG_REGISTRY_SETTING, NUGET_CONFIG_SETTING, OTEL_SETTING,
        OTEL_TRACING_PROXY_SETTING, PIP_INDEX_URL_SETTING, POWERSHELL_REPO_PAT_SETTING,
        POWERSHELL_REPO_URL_SETTING, REQUEST_SIZE_LIMIT_SETTING,
        REQUIRE_PREEXISTING_USER_FOR_OAUTH_SETTING, RETENTION_PERIOD_SECS_SETTING,
        SAML_METADATA_SETTING, SCIM_TOKEN_SETTING, TIMEOUT_WAIT_RESULT_SETTING,
        UV_INDEX_STRATEGY_SETTING,
    },
    indexer::load_indexer_config,
    jwt::JWT_SECRET,
    oauth2::REQUIRE_PREEXISTING_USER_FOR_OAUTH,
    server::load_smtp_config,
    tracing_init::JSON_FMT,
    users::truncate_token,
    utils::{empty_as_none, now_from_db, rd_string, report_critical_error, Mode, HUB_API_SECRET},
    worker::{
        load_env_vars, load_init_bash_from_env, load_periodic_bash_script_from_env,
        load_periodic_bash_script_interval_from_env, load_whitelist_env_vars_from_env,
        load_worker_config, reload_custom_tags_setting, store_pull_query,
        store_suspended_pull_query, Connection, WorkerConfig, DEFAULT_TAGS_PER_WORKSPACE,
        DEFAULT_TAGS_WORKSPACES, INDEXER_CONFIG, SCRIPT_TOKEN_EXPIRY, SMTP_CONFIG, TMP_DIR,
        WORKER_CONFIG, WORKER_GROUP,
    },
    KillpillSender, BASE_URL, CRITICAL_ALERTS_ON_DB_OVERSIZE, CRITICAL_ALERT_MUTE_UI_ENABLED,
    CRITICAL_ERROR_CHANNELS, DB, DEFAULT_HUB_BASE_URL, HUB_BASE_URL, JOB_RETENTION_SECS,
    METRICS_DEBUG_ENABLED, METRICS_ENABLED, MONITOR_LOGS_ON_OBJECT_STORE, OTEL_LOGS_ENABLED,
    OTEL_METRICS_ENABLED, OTEL_TRACING_ENABLED, SERVICE_LOG_RETENTION_SECS,
};
use windmill_common::{client::AuthedClient, global_settings::APP_WORKSPACED_ROUTE_SETTING};
use windmill_queue::{cancel_job, get_queued_job_v2, SameWorkerPayload};
use windmill_worker::{
    result_processor::handle_job_error, JobCompletedSender, JobIsolationLevel,
    OtelTracingProxySettings, SameWorkerSender, BUNFIG_INSTALL_SCOPES, CARGO_REGISTRIES,
    INSTANCE_PYTHON_VERSION, JAVA_HOME_DIR, JOB_DEFAULT_TIMEOUT, JOB_ISOLATION, KEEP_JOB_DIR,
    MAVEN_REPOS, MAVEN_SETTINGS_XML, NO_DEFAULT_MAVEN, NPM_CONFIG_REGISTRY, NSJAIL_AVAILABLE,
    NUGET_CONFIG, OTEL_TRACING_PROXY_SETTINGS, PIP_EXTRA_INDEX_URL, PIP_INDEX_URL,
    POWERSHELL_REPO_PAT, POWERSHELL_REPO_URL, UV_INDEX_STRATEGY,
};

#[cfg(feature = "parquet")]
use windmill_object_store::ObjectStoreReload;

#[cfg(feature = "enterprise")]
use crate::ee_oss::verify_license_key;

use crate::ee_oss::set_license_key;

#[cfg(feature = "prometheus")]
lazy_static::lazy_static! {

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

    static ref QUEUE_COUNT: prometheus::IntGaugeVec = prometheus::register_int_gauge_vec!(
        "queue_count",
        "Number of jobs in the queue",
        &["tag"]
    ).unwrap();

    static ref QUEUE_RUNNING_COUNT: prometheus::IntGaugeVec = prometheus::register_int_gauge_vec!(
        "queue_running_count",
        "Number of running jobs in the queue",
        &["tag"]
    ).unwrap();

}
lazy_static::lazy_static! {
    static ref ZOMBIE_JOB_TIMEOUT: String = std::env::var("ZOMBIE_JOB_TIMEOUT")
    .ok()
    .and_then(|x| x.parse::<String>().ok())
    .unwrap_or_else(|| "60".to_string());

    static ref FLOW_ZOMBIE_TRANSITION_TIMEOUT: String = std::env::var("FLOW_ZOMBIE_TRANSITION_TIMEOUT")
    .ok()
    .and_then(|x| x.parse::<String>().ok())
    .unwrap_or_else(|| "60".to_string());


    pub static ref RESTART_ZOMBIE_JOBS: bool = std::env::var("RESTART_ZOMBIE_JOBS")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(true);

    pub static ref DISABLE_ZOMBIE_JOBS_MONITORING: bool = std::env::var("DISABLE_ZOMBIE_JOBS_MONITORING")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(false);

    pub static ref WORKERS_NAMES: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));

    static ref QUEUE_COUNT_TAGS: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    static ref QUEUE_RUNNING_COUNT_TAGS: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    static ref DISABLE_CONCURRENCY_LIMIT: bool = std::env::var("DISABLE_CONCURRENCY_LIMIT").is_ok_and(|s| s == "true");

    //legacy typo
    static ref STALE_JOB_THRESHOLD_MINUTES: Option<u64> = std::env::var("STALE_JOB_THRESHOLD_MINUTES").or_else(|_| std::env::var("STALE_JOB_THRESHOLD_MINUTES"))

    .ok()
    .and_then(|x| x.parse::<u64>().ok());

    /// Batch size for job cleanup deletion queries. Default: 10000.
    /// Larger values delete more jobs per batch but hold locks longer.
    static ref JOB_CLEANUP_BATCH_SIZE: i64 = std::env::var("JOB_CLEANUP_BATCH_SIZE")
        .ok()
        .and_then(|x| x.parse::<i64>().ok())
        .unwrap_or(20000);

    /// Maximum number of batches to process per cleanup iteration. Default: 10.
    /// Set to 0 for unlimited (process until no expired jobs remain).
    static ref JOB_CLEANUP_MAX_BATCHES: i32 = std::env::var("JOB_CLEANUP_MAX_BATCHES")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(20);
}

pub async fn initial_load(
    conn: &Connection,
    tx: KillpillSender,
    worker_mode: bool,
    server_mode: bool,
    #[cfg(feature = "parquet")] disable_s3_store: bool,
) {
    if let Err(e) = reload_base_url_setting(&conn).await {
        tracing::error!("Error loading base url: {:?}", e)
    }

    if let Some(db) = conn.as_sql() {
        if let Err(e) = reload_critical_error_channels_setting(&db).await {
            tracing::error!("Could loading critical error emails setting: {:?}", e);
        }
    }

    if let Err(e) = load_metrics_enabled(conn).await {
        tracing::error!("Error loading expose metrics: {e:#}");
    }

    if let Err(e) = load_metrics_debug_enabled(conn).await {
        tracing::error!("Error loading expose debug metrics: {e:#}");
    }

    if let Err(e) = reload_critical_alert_mute_ui_setting(conn).await {
        tracing::error!("Error loading critical alert mute ui setting: {e:#}");
    }

    if let Some(db) = conn.as_sql() {
        if let Err(e) = load_tag_per_workspace_enabled(db).await {
            tracing::error!("Error loading default tag per workpsace: {e:#}");
        }

        if let Err(e) = load_tag_per_workspace_workspaces(db).await {
            tracing::error!("Error loading default tag per workpsace workspaces: {e:#}");
        }
    }

    if server_mode {
        if let Some(db) = conn.as_sql() {
            load_require_preexisting_user(db).await;
            if let Err(e) = reload_critical_alerts_on_db_oversize(db).await {
                tracing::error!(
                    "Error reloading critical alerts on db oversize setting: {:?}",
                    e
                )
            }
            windmill_common::min_version::store_min_keep_alive_version(db).await;
        }
    }

    if worker_mode {
        load_keep_job_dir(conn).await;
        match conn {
            Connection::Sql(db) => {
                reload_worker_config(&db, tx, false).await;
            }
            Connection::Http(_) => {
                // TODO: reload worker config from http
                let mut config = WORKER_CONFIG.write().await;
                let worker_tags = DECODED_AGENT_TOKEN
                    .as_ref()
                    .map(|x| x.tags.clone())
                    .unwrap_or_default();
                // we only check from env as native_mode is not stored in the token
                let native_mode = windmill_common::worker::is_native_mode_from_env();
                windmill_common::worker::NATIVE_MODE_RESOLVED
                    .store(native_mode, std::sync::atomic::Ordering::Relaxed);
                *config = WorkerConfig {
                    worker_tags,
                    env_vars: load_env_vars(
                        load_whitelist_env_vars_from_env(),
                        &std::collections::HashMap::new(),
                    ),
                    priority_tags_sorted: vec![],
                    dedicated_worker: None,
                    dedicated_workers: None,
                    init_bash: load_init_bash_from_env(),
                    periodic_script_bash: load_periodic_bash_script_from_env(),
                    periodic_script_interval_seconds: load_periodic_bash_script_interval_from_env(),
                    cache_clear: None,
                    additional_python_paths: None,
                    pip_local_dependencies: None,
                    native_mode,
                };
            }
        }
    }

    if let Err(e) = reload_hub_base_url_setting(conn, server_mode).await {
        tracing::error!("Error reloading hub base url: {:?}", e)
    }

    if let Some(db) = conn.as_sql() {
        if let Err(e) = reload_jwt_secret_setting(db).await {
            tracing::error!("Could not reload jwt secret setting: {:?}", e);
        }

        if let Err(e) = reload_custom_tags_setting(db).await {
            tracing::error!("Error reloading custom tags: {:?}", e)
        }

        if let Err(e) = reload_app_workspaced_route_setting(db).await {
            tracing::error!("Error reloading app workspaced route: {:?}", e)
        }
    }

    #[cfg(feature = "parquet")]
    if !disable_s3_store {
        if let Some(db) = conn.as_sql() {
            let db2 = db.clone();
            match reload_object_store_setting(db).await {
                ObjectStoreReload::Later => {
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(10)).await;
                        match reload_object_store_setting(&db2).await {
                            ObjectStoreReload::Later => {
                                tracing::error!("Giving up on loading object store setting");
                            }
                            ObjectStoreReload::Never => {
                                tracing::info!("Object store setting successfully loaded");
                            }
                        }
                    });
                }
                ObjectStoreReload::Never => (),
            }
        }
    }

    if let Some(db) = conn.as_sql() {
        reload_smtp_config(db).await;
    }

    reload_hub_api_secret_setting(&conn).await;

    if server_mode {
        reload_retention_period_setting(&conn).await;
        reload_request_size(&conn).await;
        reload_saml_metadata_setting(&conn).await;
        reload_scim_token_setting(&conn).await;
    }

    if worker_mode {
        reload_job_default_timeout_setting(&conn).await;
        reload_job_isolation_setting(&conn).await;
        reload_extra_pip_index_url_setting(&conn).await;
        reload_pip_index_url_setting(&conn).await;
        reload_uv_index_strategy_setting(&conn).await;
        reload_npm_config_registry_setting(&conn).await;
        reload_bunfig_install_scopes_setting(&conn).await;
        reload_instance_python_version_setting(&conn).await;
        reload_nuget_config_setting(&conn).await;
        reload_powershell_repo_url_setting(&conn).await;
        reload_powershell_repo_pat_setting(&conn).await;
        reload_maven_repos_setting(&conn).await;
        reload_maven_settings_xml_setting(&conn).await;
        reload_no_default_maven_setting(&conn).await;
        reload_ruby_repos_setting(&conn).await;
        reload_cargo_registries_setting(&conn).await;
    }
}

pub async fn load_metrics_enabled(conn: &Connection) -> error::Result<()> {
    let metrics_enabled =
        load_value_from_global_settings_with_conn(conn, EXPOSE_METRICS_SETTING, true).await;
    match metrics_enabled {
        Ok(Some(serde_json::Value::Bool(t))) => METRICS_ENABLED.store(t, Ordering::Relaxed),
        _ => (),
    };
    Ok(())
}

#[derive(serde::Deserialize)]
struct OtelSetting {
    metrics_enabled: Option<bool>,
    logs_enabled: Option<bool>,
    tracing_enabled: Option<bool>,
    #[serde(default, deserialize_with = "empty_as_none")]
    otel_exporter_otlp_endpoint: Option<String>,
    #[serde(default, deserialize_with = "empty_as_none")]
    otel_exporter_otlp_headers: Option<String>,
    #[serde(default, deserialize_with = "empty_as_none")]
    otel_exporter_otlp_protocol: Option<String>,
    #[serde(default, deserialize_with = "empty_as_none")]
    otel_exporter_otlp_compression: Option<String>,
}

pub async fn load_otel(db: &DB) {
    let otel = load_value_from_global_settings(db, OTEL_SETTING).await;
    if let Ok(v) = otel {
        if let Some(v) = v {
            let deser = serde_json::from_value::<OtelSetting>(v);
            if let Ok(o) = deser {
                let metrics_enabled = o.metrics_enabled.unwrap_or_else(|| {
                    std::env::var("OTEL_METRICS_ENABLED")
                        .map(|x| x.parse::<bool>().unwrap_or(false))
                        .unwrap_or(false)
                });
                let logs_enabled = o.logs_enabled.unwrap_or_else(|| {
                    std::env::var("OTEL_LOGS_ENABLED")
                        .map(|x| x.parse::<bool>().unwrap_or(false))
                        .unwrap_or(false)
                });
                let tracing_enabled = o.tracing_enabled.unwrap_or_else(|| {
                    std::env::var("OTEL_TRACING_ENABLED")
                        .map(|x| x.parse::<bool>().unwrap_or(false))
                        .unwrap_or(false)
                });

                OTEL_METRICS_ENABLED.store(metrics_enabled, Ordering::Relaxed);
                OTEL_LOGS_ENABLED.store(logs_enabled, Ordering::Relaxed);
                OTEL_TRACING_ENABLED.store(tracing_enabled, Ordering::Relaxed);

                let endpoint = if let Some(endpoint) = o.otel_exporter_otlp_endpoint {
                    unsafe {
                        std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", endpoint.clone());
                    }
                    Some(endpoint.clone())
                } else {
                    std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()
                };

                let headers = if let Some(headers) = o.otel_exporter_otlp_headers {
                    unsafe {
                        std::env::set_var("OTEL_EXPORTER_OTLP_HEADERS", headers.clone());
                    }
                    Some(headers.clone())
                } else {
                    std::env::var("OTEL_EXPORTER_OTLP_HEADERS").ok()
                };

                if let Some(protocol) = o.otel_exporter_otlp_protocol {
                    unsafe {
                        std::env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", protocol);
                    }
                }
                if let Some(compression) = o.otel_exporter_otlp_compression {
                    unsafe {
                        std::env::set_var("OTEL_EXPORTER_OTLP_COMPRESSION", compression);
                    }
                }
                println!("OTEL settings loaded: tracing ({tracing_enabled}), logs ({logs_enabled}), metrics ({metrics_enabled}), endpoint ({:?}), headers defined: ({})",
                endpoint, headers.is_some());
            } else {
                tracing::error!("Error deserializing otel settings");
            }
        }
    } else {
        tracing::error!("Error loading otel settings: {}", otel.unwrap_err());
    }
}

pub async fn load_tag_per_workspace_enabled(db: &DB) -> error::Result<()> {
    let metrics_enabled =
        load_value_from_global_settings(db, DEFAULT_TAGS_PER_WORKSPACE_SETTING).await;

    match metrics_enabled {
        Ok(Some(serde_json::Value::Bool(t))) => {
            DEFAULT_TAGS_PER_WORKSPACE.store(t, Ordering::Relaxed)
        }
        _ => (),
    };
    Ok(())
}

pub async fn load_tag_per_workspace_workspaces(db: &DB) -> error::Result<()> {
    let workspaces = load_value_from_global_settings(db, DEFAULT_TAGS_WORKSPACES_SETTING).await;

    match workspaces {
        Ok(Some(serde_json::Value::Array(t))) => {
            let workspaces = t
                .iter()
                .filter_map(|x| x.as_str())
                .map(|x| x.to_string())
                .collect::<Vec<String>>();
            let mut w = DEFAULT_TAGS_WORKSPACES.write().await;
            *w = Some(workspaces);
        }
        Ok(None) => {
            let mut w = DEFAULT_TAGS_WORKSPACES.write().await;
            *w = None;
        }
        _ => (),
    };
    Ok(())
}

pub async fn reload_critical_alert_mute_ui_setting(conn: &Connection) -> error::Result<()> {
    if let Ok(Some(serde_json::Value::Bool(t))) =
        load_value_from_global_settings_with_conn(conn, CRITICAL_ALERT_MUTE_UI_SETTING, true).await
    {
        CRITICAL_ALERT_MUTE_UI_ENABLED.store(t, Ordering::Relaxed);
    }
    Ok(())
}

pub async fn load_metrics_debug_enabled(conn: &Connection) -> error::Result<()> {
    let metrics_enabled =
        load_value_from_global_settings_with_conn(conn, EXPOSE_DEBUG_METRICS_SETTING, true).await;
    match metrics_enabled {
        Ok(Some(serde_json::Value::Bool(t))) => {
            METRICS_DEBUG_ENABLED.store(t, Ordering::Relaxed);
            //_RJEM_MALLOC_CONF=prof:true,prof_active:false,lg_prof_interval:30,lg_prof_sample:21,prof_prefix:/tmp/jeprof
            #[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
            if std::env::var("_RJEM_MALLOC_CONF").is_ok() {
                if let Err(e) = set_prof_active(t) {
                    tracing::error!("Error setting jemalloc prof_active: {e:?}");
                }
            }
        }
        _ => (),
    };
    Ok(())
}

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
#[derive(Debug, Clone)]
pub struct MallctlError {
    #[allow(unused)]
    pub code: i32,
}

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
fn set_prof_active(new_value: bool) -> Result<(), MallctlError> {
    let option_name = std::ffi::CString::new("prof.active").unwrap();

    tracing::info!("Setting jemalloc prof_active to {}", new_value);
    let result = unsafe {
        tikv_jemalloc_sys::mallctl(
            option_name.as_ptr(),              // const char *name
            std::ptr::null_mut(),              // void *oldp
            std::ptr::null_mut(),              // size_t *oldlenp
            &new_value as *const _ as *mut _,  // void *newp
            std::mem::size_of_val(&new_value), // size_t newlen
        )
    };

    if result != 0 {
        return Err(MallctlError { code: result });
    }

    Ok(())
}

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
pub fn bytes_to_mb(bytes: u64) -> f64 {
    const BYTES_PER_MB: f64 = 1_048_576.0;
    bytes as f64 / BYTES_PER_MB
}

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
pub async fn monitor_mem() {
    use std::time::Duration;
    use tikv_jemalloc_ctl::{epoch, stats};

    tokio::spawn(async move {
        // Obtain a MIB for the `epoch`, `stats.allocated`, and
        // `atats.resident` keys:
        let e = match epoch::mib() {
            Ok(mib) => mib,
            Err(e) => {
                tracing::error!("Error getting jemalloc epoch mib: {:?}", e);
                return;
            }
        };
        let allocated = match stats::allocated::mib() {
            Ok(mib) => mib,
            Err(e) => {
                tracing::error!("Error getting jemalloc allocated mib: {:?}", e);
                return;
            }
        };
        let resident = match stats::resident::mib() {
            Ok(mib) => mib,
            Err(e) => {
                tracing::error!("Error getting jemalloc resident mib: {:?}", e);
                return;
            }
        };

        loop {
            // Many statistics are cached and only updated
            // when the epoch is advanced:
            match e.advance() {
                Ok(_) => {
                    // Read statistics using MIB key:
                    let allocated = allocated.read().unwrap_or_default();
                    let resident = resident.read().unwrap_or_default();
                    tracing::info!(
                        "{} mb allocated/{} mb resident",
                        bytes_to_mb(allocated as u64),
                        bytes_to_mb(resident as u64)
                    );
                }
                Err(e) => {
                    tracing::error!("Error advancing jemalloc epoch: {:?}", e);
                }
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
}

async fn sleep_until_next_minute_start_plus_one_s() {
    let now = Utc::now();
    let next_minute = now + Duration::from_secs(60 - now.timestamp() as u64 % 60 + 1);
    tokio::time::sleep(tokio::time::Duration::from_secs(
        next_minute.timestamp() as u64 - now.timestamp() as u64,
    ))
    .await;
}

use windmill_common::tracing_init::TMP_WINDMILL_LOGS_SERVICE;
async fn find_two_highest_files(hostname: &str) -> (Option<String>, Option<String>) {
    let log_dir = format!("{}/{}/", TMP_WINDMILL_LOGS_SERVICE, hostname);
    let rd_dir = tokio::fs::read_dir(log_dir).await;
    if let Ok(mut log_files) = rd_dir {
        let mut highest_file: Option<String> = None;
        let mut second_highest_file: Option<String> = None;
        while let Ok(Some(file)) = log_files.next_entry().await {
            let file_name = file
                .file_name()
                .to_str()
                .map(|x| x.to_string())
                .unwrap_or_default();
            if file_name > highest_file.clone().unwrap_or_default() {
                second_highest_file = highest_file;
                highest_file = Some(file_name);
            }
        }
        (highest_file, second_highest_file)
    } else {
        tracing::error!(
            "Error reading log files: {TMP_WINDMILL_LOGS_SERVICE}, {:#?}",
            rd_dir.unwrap_err()
        );
        (None, None)
    }
}

fn get_worker_group(mode: &Mode) -> Option<String> {
    let worker_group = WORKER_GROUP.clone();
    if worker_group.is_empty() || mode == &Mode::Server || mode == &Mode::Indexer {
        None
    } else {
        Some(worker_group)
    }
}

pub fn send_logs_to_object_store(conn: &Connection, hostname: &str, mode: &Mode) {
    let conn = conn.clone();
    let hostname = hostname.to_string();
    let mode = mode.clone();
    let worker_group = get_worker_group(&mode);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        sleep_until_next_minute_start_plus_one_s().await;
        loop {
            interval.tick().await;
            let (_, snd_highest_file) = find_two_highest_files(&hostname).await;
            send_log_file_to_object_store(
                &hostname,
                &mode,
                &worker_group,
                &conn,
                snd_highest_file,
                false,
            )
            .await;
        }
    });
}

pub async fn send_current_log_file_to_object_store(conn: &Connection, hostname: &str, mode: &Mode) {
    tracing::info!("Sending current log file to object store");
    let (highest_file, _) = find_two_highest_files(hostname).await;
    let worker_group = get_worker_group(&mode);
    send_log_file_to_object_store(hostname, mode, &worker_group, conn, highest_file, true).await;
}

fn get_now_and_str() -> (NaiveDateTime, String) {
    let ts = Utc::now().naive_utc();
    (
        ts,
        ts.format(windmill_common::tracing_init::LOG_TIMESTAMP_FMT)
            .to_string(),
    )
}

lazy_static::lazy_static! {
    static ref LAST_LOG_FILE_SENT: Arc<Mutex<Option<NaiveDateTime>>> = Arc::new(Mutex::new(None));
}

async fn send_log_file_to_object_store(
    hostname: &str,
    mode: &Mode,
    worker_group: &Option<String>,
    conn: &Connection,
    snd_highest_file: Option<String>,
    use_now: bool,
) {
    if let Some(highest_file) = snd_highest_file {
        //parse datetime frome file xxxx.yyyy-MM-dd-HH-mm
        let (ts, ts_str) = if use_now {
            get_now_and_str()
        } else {
            highest_file
                .split(".")
                .last()
                .and_then(|x| {
                    NaiveDateTime::parse_from_str(
                        x,
                        windmill_common::tracing_init::LOG_TIMESTAMP_FMT,
                    )
                    .ok()
                    .map(|y| (y, x.to_string()))
                })
                .unwrap_or_else(get_now_and_str)
        };

        let exists = LAST_LOG_FILE_SENT.lock().map(|last_log_file_sent| {
            last_log_file_sent
                .map(|last_log_file_sent| last_log_file_sent >= ts)
                .unwrap_or(false)
        });

        if exists.unwrap_or(false) {
            return;
        }

        #[cfg(feature = "parquet")]
        let s3_client = windmill_object_store::get_object_store().await;
        #[cfg(feature = "parquet")]
        if let Some(s3_client) = s3_client {
            let path = std::path::Path::new(TMP_WINDMILL_LOGS_SERVICE)
                .join(hostname)
                .join(&highest_file);

            //read file as byte stream
            let bytes = tokio::fs::read(&path).await;
            if let Err(e) = bytes {
                tracing::error!("Error reading log file: {:?}", e);
                return;
            }
            let path = windmill_object_store::object_store_reexports::Path::from_url_path(format!(
                "{}{hostname}/{highest_file}",
                windmill_common::tracing_init::LOGS_SERVICE
            ));
            if let Err(e) = path {
                tracing::error!("Error creating log file path: {:?}", e);
                return;
            }
            if let Err(e) = s3_client.put(&path.unwrap(), bytes.unwrap().into()).await {
                tracing::error!("Error sending logs to object store: {:?}", e);
            }
        }

        let (ok_lines, err_lines) = read_log_counters(ts_str);

        if let Some(db) = conn.as_sql() {
            match timeout(Duration::from_secs(10), sqlx::query!("INSERT INTO log_file (hostname, mode, worker_group, log_ts, file_path, ok_lines, err_lines, json_fmt)
             VALUES ($1, $2::text::LOG_MODE, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (hostname, log_ts) DO UPDATE SET ok_lines = log_file.ok_lines + $6, err_lines = log_file.err_lines + $7",
                hostname, mode.to_string(), worker_group.clone(), ts, highest_file, ok_lines as i64, err_lines as i64, *JSON_FMT)
                .execute(db)).await {
                Ok(Ok(_)) => {
                    if let Err(e) = LAST_LOG_FILE_SENT.lock().map(|mut last_log_file_sent| {
                        last_log_file_sent.replace(ts);
                    }) {
                        tracing::error!("Error updating last log file sent: {:?}", e);
                    }
                    tracing::info!("Log file sent: {}", highest_file);
                }
                Ok(Err(e)) => {
                    tracing::error!("Error inserting log file: {:?}", e);
                }
                Err(e) => {
                    tracing::error!("Error inserting log file, timeout elapsed: {:?}", e);
                }
            }
        } else {
            // tracing::warn!("Not sending log file to object store in agent mode");
            ()
        }
    }
}

fn read_log_counters(ts_str: String) -> (usize, usize) {
    let counters = windmill_common::tracing_init::LOG_COUNTING_BY_MIN.read();
    let mut ok_lines = 0;
    let mut err_lines = 0;
    if let Ok(ref c) = counters {
        let counter = c.get(&ts_str);
        if let Some(counter) = counter {
            ok_lines = counter.non_error_count;
            err_lines = counter.error_count;
        } else {
            // println!("no counter found for {ts_str}");
        }
    } else {
        println!("Error reading log counters 2");
    }
    (ok_lines, err_lines)
}

pub async fn load_keep_job_dir(conn: &Connection) {
    let value = load_value_from_global_settings_with_conn(conn, KEEP_JOB_DIR_SETTING, true).await;
    match value {
        Ok(Some(serde_json::Value::Bool(t))) => KEEP_JOB_DIR.store(t, Ordering::Relaxed),
        Err(e) => {
            tracing::error!("Error loading keep job dir metrics: {e:#}");
        }
        _ => (),
    };
}

pub async fn reload_otel_tracing_proxy_setting(conn: &Connection) {
    match load_value_from_global_settings_with_conn(conn, OTEL_TRACING_PROXY_SETTING, true).await {
        Ok(Some(settings)) => match serde_json::from_value::<OtelTracingProxySettings>(settings) {
            Ok(new_settings) => {
                let mut current = OTEL_TRACING_PROXY_SETTINGS.write().await;
                if current.enabled != new_settings.enabled
                    || current.enabled_languages != new_settings.enabled_languages
                {
                    tracing::info!(
                        "OTEL tracing proxy settings changed: enabled={}, languages={:?}",
                        new_settings.enabled,
                        new_settings.enabled_languages
                    );
                    *current = new_settings;
                }
            }
            Err(e) => {
                tracing::error!("Error parsing OTEL tracing proxy settings: {e:#}");
            }
        },
        Err(e) => {
            tracing::error!("Error loading OTEL tracing proxy setting: {e:#}");
        }
        _ => (),
    };
}

pub async fn load_require_preexisting_user(db: &DB) {
    let value =
        load_value_from_global_settings(db, REQUIRE_PREEXISTING_USER_FOR_OAUTH_SETTING).await;
    match value {
        Ok(Some(serde_json::Value::Bool(t))) => {
            REQUIRE_PREEXISTING_USER_FOR_OAUTH.store(t, Ordering::Relaxed)
        }
        Err(e) => {
            tracing::error!("Error loading keep job dir metrics: {e:#}");
        }
        _ => (),
    };
}

struct LogFile {
    file_path: String,
    hostname: String,
}

pub async fn delete_expired_items(db: &DB) -> () {
    let tokens_deleted_r: std::result::Result<Vec<String>, _> = sqlx::query_scalar(
        "DELETE FROM token WHERE expiration <= now()
        RETURNING concat(substring(token for 10), '*****')",
    )
    .fetch_all(db)
    .await;

    match tokens_deleted_r {
        Ok(tokens) => {
            if tokens.len() > 0 {
                tracing::info!("deleted {} tokens: {:?}", tokens.len(), tokens)
            }
        }
        Err(e) => tracing::error!("Error deleting token: {}", e.to_string()),
    }

    let pip_resolution_r = sqlx::query_scalar!(
        "DELETE FROM pip_resolution_cache WHERE expiration <= now() RETURNING hash",
    )
    .fetch_all(db)
    .await;

    match pip_resolution_r {
        Ok(res) => {
            if res.len() > 0 {
                tracing::info!("deleted {} pip_resolution: {:?}", res.len(), res)
            }
        }
        Err(e) => tracing::error!("Error deleting pip_resolution: {}", e.to_string()),
    }

    // Clean up expired MCP OAuth refresh tokens
    let mcp_refresh_tokens_r: std::result::Result<Vec<i64>, _> = sqlx::query_scalar(
        "DELETE FROM mcp_oauth_refresh_token WHERE expires_at <= now() RETURNING id",
    )
    .fetch_all(db)
    .await;

    match mcp_refresh_tokens_r {
        Ok(ids) => {
            if ids.len() > 0 {
                tracing::info!("deleted {} expired MCP OAuth refresh tokens", ids.len())
            }
        }
        Err(e) => tracing::error!("Error deleting MCP OAuth refresh tokens: {}", e.to_string()),
    }

    let deleted_cache = sqlx::query_scalar!(
            "DELETE FROM resource WHERE resource_type = 'cache' AND to_timestamp((value->>'expire')::int) < now() RETURNING path",
        )
        .fetch_all(db)
        .await;

    match deleted_cache {
        Ok(res) => {
            if res.len() > 0 {
                tracing::info!("deleted {} cache resource: {:?}", res.len(), res)
            }
        }
        Err(e) => tracing::error!("Error deleting cache resource {}", e.to_string()),
    }

    let deleted_expired_variables = sqlx::query_scalar!(
        "DELETE FROM variable WHERE expires_at IS NOT NULL AND expires_at < now() RETURNING path",
    )
    .fetch_all(db)
    .await;

    match deleted_expired_variables {
        Ok(res) => {
            if res.len() > 0 {
                tracing::info!("deleted {} expired variables {:?}", res.len(), res)
            }
        }
        Err(e) => tracing::error!("Error deleting cache resource {}", e.to_string()),
    }

    match sqlx::query_as!(
        LogFile,
        "DELETE FROM log_file WHERE log_ts <= now() - ($1::bigint::text || ' s')::interval RETURNING file_path, hostname",
        SERVICE_LOG_RETENTION_SECS,
    )
    .fetch_all(db)
    .await
    {
        Ok(log_files_to_delete) => {
                let paths = log_files_to_delete
                    .iter()
                    .map(|f| format!("{}/{}", f.hostname, f.file_path))
                    .collect();
                delete_log_files_from_disk_and_store(paths, TMP_WINDMILL_LOGS_SERVICE, windmill_common::tracing_init::LOGS_SERVICE).await;

        }
        Err(e) => tracing::error!("Error deleting log file: {:?}", e),
    }

    #[cfg(not(feature = "enterprise"))]
    let audit_retention_secs = 1 * 60 * 60 * 24 * 14;

    #[cfg(feature = "enterprise")]
    let audit_retention_secs = 1 * 60 * 60 * 24 * 365;

    if let Err(e) = sqlx::query_scalar!(
        "DELETE FROM audit WHERE timestamp <= now() - ($1::bigint::text || ' s')::interval",
        audit_retention_secs,
    )
    .fetch_all(db)
    .await
    {
        tracing::error!("Error deleting audit log on CE: {:?}", e);
    }

    if let Err(e) = sqlx::query_scalar!(
        "DELETE FROM autoscaling_event WHERE applied_at <= now() - ($1::bigint::text || ' s')::interval",
        30 * 24 * 60 * 60, // 30 days
    )
    .fetch_all(db)
    .await
    {
        tracing::error!("Error deleting autoscaling event on CE: {:?}", e);
    }

    match sqlx::query_scalar!(
        "DELETE FROM agent_token_blacklist WHERE expires_at <= now() RETURNING token",
    )
    .fetch_all(db)
    .await
    {
        Ok(deleted_tokens) => {
            if deleted_tokens.len() > 0 {
                tracing::info!(
                    "deleted {} expired blacklisted agent tokens: {:?}",
                    deleted_tokens.len(),
                    deleted_tokens
                );
            }
        }
        Err(e) => tracing::error!("Error deleting expired blacklisted agent tokens: {:?}", e),
    }

    match sqlx::query_scalar!(
        "DELETE FROM mcp_oauth_server_code WHERE expires_at <= now() RETURNING code",
    )
    .fetch_all(db)
    .await
    {
        Ok(deleted_codes) => {
            if deleted_codes.len() > 0 {
                tracing::info!(
                    "deleted {} expired MCP OAuth authorization codes",
                    deleted_codes.len()
                );
            }
        }
        Err(e) => tracing::error!(
            "Error deleting expired MCP OAuth authorization codes: {:?}",
            e
        ),
    }

    let job_retention_secs = *JOB_RETENTION_SECS.read().await;
    if job_retention_secs > 0 {
        let batch_size = *JOB_CLEANUP_BATCH_SIZE;
        let max_batches = *JOB_CLEANUP_MAX_BATCHES;
        let cleanup_start = Instant::now();
        let mut total_deleted = 0u64;
        let mut batch_num = 0i32;

        // Process batches until no more expired jobs or max batches reached
        loop {
            if max_batches > 0 && batch_num >= max_batches {
                tracing::debug!(
                    "Job cleanup: reached max batches limit ({}), will continue next iteration",
                    max_batches
                );
                break;
            }

            // Each batch runs in its own transaction to avoid long-running locks
            let batch_result = delete_expired_jobs_batch(db, job_retention_secs, batch_size).await;

            match batch_result {
                Ok(deleted_count) => {
                    if deleted_count == 0 {
                        // No more expired jobs to delete
                        break;
                    }
                    total_deleted += deleted_count as u64;
                    batch_num += 1;
                }
                Err(e) => {
                    tracing::error!("Error in job cleanup batch {}: {:?}", batch_num, e);
                    break;
                }
            }
        }

        if total_deleted > 0 {
            tracing::info!(
                "Job cleanup completed: deleted {} jobs in {} batches, took {:?}",
                total_deleted,
                batch_num,
                cleanup_start.elapsed()
            );
        }

        // Clean up concurrency keys separately (not tied to specific job IDs)
        if let Err(e) = sqlx::query!(
            "DELETE FROM concurrency_key WHERE ended_at <= now() - ($1::bigint::text || ' s')::interval",
            job_retention_secs
        )
        .execute(db)
        .await
        {
            tracing::error!("Error deleting custom concurrency key: {:?}", e);
        }
    }
}

/// Delete a batch of expired jobs with LIMIT and SKIP LOCKED for high-scale environments.
/// Uses a single transaction per batch to minimize lock duration.
/// Returns the number of jobs deleted in this batch.
async fn delete_expired_jobs_batch(
    db: &DB,
    job_retention_secs: i64,
    batch_size: i64,
) -> error::Result<usize> {
    let mut tx = db.begin().await?;

    // Fetch active ROOT job IDs that started before the retention period. We only care about
    // these because their child jobs could be old enough to be deletion candidates.
    // Jobs started after the retention period can't have children old enough to delete.
    let active_root_job_ids: Vec<Uuid> = sqlx::query_scalar!(
        "SELECT q.id FROM v2_job_queue q
         JOIN v2_job j ON j.id = q.id
         WHERE j.parent_job IS NULL
           AND j.created_at <= now() - ($1::bigint::text || ' s')::interval",
        job_retention_secs
    )
    .fetch_all(&mut *tx)
    .await?;

    // Use FOR UPDATE SKIP LOCKED to avoid contention between replicas
    // ORDER BY completed_at ensures we delete oldest jobs first
    let deleted_jobs: Vec<Uuid> = sqlx::query_scalar!(
        "DELETE FROM v2_job_completed
         WHERE id IN (
             SELECT jc.id FROM v2_job_completed jc
             LEFT JOIN v2_job j ON j.id = jc.id
             WHERE jc.completed_at <= now() - ($1::bigint::text || ' s')::interval
               AND COALESCE(j.root_job, j.flow_innermost_root_job, jc.id) != ALL($3)
             ORDER BY jc.completed_at ASC
             LIMIT $2
             FOR UPDATE OF jc SKIP LOCKED
         )
         RETURNING id",
        job_retention_secs,
        batch_size,
        &active_root_job_ids
    )
    .fetch_all(&mut *tx)
    .await?;

    let deleted_count = deleted_jobs.len();

    if deleted_count > 0 {
        tracing::debug!(
            "Deleting batch of {} expired jobs (retention: {}s)",
            deleted_count,
            job_retention_secs
        );

        if let Err(e) = sqlx::query!(
            "DELETE FROM job_stats WHERE job_id = ANY($1)",
            &deleted_jobs
        )
        .execute(&mut *tx)
        .await
        {
            tracing::error!("Error deleting job stats: {:?}", e);
        }

        match sqlx::query_scalar!(
            "DELETE FROM job_logs WHERE job_id = ANY($1) RETURNING log_file_index",
            &deleted_jobs
        )
        .fetch_all(&mut *tx)
        .await
        {
            Ok(log_file_index) => {
                let paths = log_file_index
                    .into_iter()
                    .filter_map(|opt| opt)
                    .flat_map(|inner_vec| inner_vec.into_iter())
                    .collect();
                delete_log_files_from_disk_and_store(paths, TMP_DIR, "").await;
            }
            Err(e) => tracing::error!("Error deleting job logs: {:?}", e),
        }

        if let Err(e) = sqlx::query!("DELETE FROM v2_job WHERE id = ANY($1)", &deleted_jobs)
            .execute(&mut *tx)
            .await
        {
            tracing::error!("Error deleting job: {:?}", e);
        }

        // Should already be deleted but just in case
        if let Err(e) = sqlx::query!(
            "DELETE FROM job_result_stream_v2 WHERE job_id = ANY($1)",
            &deleted_jobs
        )
        .execute(&mut *tx)
        .await
        {
            tracing::error!("Error deleting job result stream: {:?}", e);
        }
    }

    tx.commit().await?;

    Ok(deleted_count)
}

async fn delete_log_files_from_disk_and_store(
    paths_to_delete: Vec<String>,
    tmp_dir: &str,
    _s3_prefix: &str,
) {
    #[cfg(feature = "parquet")]
    let os = windmill_object_store::get_object_store().await;
    #[cfg(not(feature = "parquet"))]
    let os: Option<()> = None;

    let _should_del_from_store = MONITOR_LOGS_ON_OBJECT_STORE.read().await.clone();

    let delete_futures = FuturesUnordered::new();

    for path in paths_to_delete {
        let _os2 = &os;

        delete_futures.push(async move {
            let disk_path = std::path::Path::new(tmp_dir).join(&path);
            if tokio::fs::metadata(&disk_path).await.is_ok() {
                if let Err(e) = tokio::fs::remove_file(&disk_path).await {
                    tracing::error!(
                        "Failed to delete from disk {}: {e}",
                        disk_path.to_string_lossy()
                    );
                } else {
                    tracing::debug!(
                        "Succesfully deleted {} from disk",
                        disk_path.to_string_lossy()
                    );
                }
            }

            #[cfg(feature = "parquet")]
            if _should_del_from_store {
                if let Some(os) = _os2 {
                    let p = windmill_object_store::object_store_reexports::Path::from(format!("{}{}", _s3_prefix, path));
                    if let Err(e) = os.delete(&p).await {
                        tracing::error!("Failed to delete from object store {}: {e}", p.to_string())
                    } else {
                        tracing::debug!("Succesfully deleted {} from object store", p.to_string());
                    }
                }
            }
        });
    }

    let _: Vec<_> = delete_futures.collect().await;
}

pub async fn reload_scim_token_setting(conn: &Connection) {
    reload_option_setting_with_tracing(conn, SCIM_TOKEN_SETTING, "SCIM_TOKEN", SCIM_TOKEN.clone())
        .await;
}

pub async fn reload_timeout_wait_result_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        TIMEOUT_WAIT_RESULT_SETTING,
        "TIMEOUT_WAIT_RESULT",
        TIMEOUT_WAIT_RESULT.clone(),
    )
    .await;
}

pub async fn reload_saml_metadata_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        SAML_METADATA_SETTING,
        "SAML_METADATA",
        SAML_METADATA.clone(),
    )
    .await;
}

pub async fn reload_extra_pip_index_url_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        EXTRA_PIP_INDEX_URL_SETTING,
        "PIP_EXTRA_INDEX_URL",
        PIP_EXTRA_INDEX_URL.clone(),
    )
    .await;
}

pub async fn reload_pip_index_url_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        PIP_INDEX_URL_SETTING,
        "PIP_INDEX_URL",
        PIP_INDEX_URL.clone(),
    )
    .await;
}

pub async fn reload_uv_index_strategy_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        UV_INDEX_STRATEGY_SETTING,
        "UV_INDEX_STRATEGY",
        UV_INDEX_STRATEGY.clone(),
    )
    .await;
}

pub async fn reload_instance_python_version_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        INSTANCE_PYTHON_VERSION_SETTING,
        "INSTANCE_PYTHON_VERSION",
        INSTANCE_PYTHON_VERSION.clone(),
    )
    .await;
}

pub async fn reload_npm_config_registry_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        NPM_CONFIG_REGISTRY_SETTING,
        "NPM_CONFIG_REGISTRY",
        NPM_CONFIG_REGISTRY.clone(),
    )
    .await;
}

pub async fn reload_bunfig_install_scopes_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        BUNFIG_INSTALL_SCOPES_SETTING,
        "BUNFIG_INSTALL_SCOPES",
        BUNFIG_INSTALL_SCOPES.clone(),
    )
    .await;
}

pub async fn reload_nuget_config_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        NUGET_CONFIG_SETTING,
        "NUGET_CONFIG",
        NUGET_CONFIG.clone(),
    )
    .await;
}

pub async fn reload_powershell_repo_url_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        POWERSHELL_REPO_URL_SETTING,
        "POWERSHELL_REPO_URL",
        POWERSHELL_REPO_URL.clone(),
    )
    .await;
}

pub async fn reload_powershell_repo_pat_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        POWERSHELL_REPO_PAT_SETTING,
        "POWERSHELL_REPO_PAT",
        POWERSHELL_REPO_PAT.clone(),
    )
    .await;
}

pub async fn reload_maven_repos_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        windmill_common::global_settings::MAVEN_REPOS_SETTING,
        "MAVEN_REPOS",
        MAVEN_REPOS.clone(),
    )
    .await;
}

pub async fn reload_maven_settings_xml_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        windmill_common::global_settings::MAVEN_SETTINGS_XML_SETTING,
        "MAVEN_SETTINGS_XML",
        MAVEN_SETTINGS_XML.clone(),
    )
    .await;

    if !cfg!(feature = "enterprise") {
        return;
    }

    let settings_xml = MAVEN_SETTINGS_XML.read().await.clone();
    match settings_xml {
        Some(ref content) if !content.trim().is_empty() => {
            let m2_dir = format!("{JAVA_HOME_DIR}/.m2");
            if let Err(e) = tokio::fs::create_dir_all(&m2_dir).await {
                tracing::error!("Failed to create .m2 directory: {e:#}");
                return;
            }
            let settings_path = format!("{m2_dir}/settings.xml");
            if let Err(e) = tokio::fs::write(&settings_path, content).await {
                tracing::error!("Failed to write Maven settings.xml: {e:#}");
            }
        }
        _ => {
            let settings_path = format!("{JAVA_HOME_DIR}/.m2/settings.xml");
            let _ = tokio::fs::remove_file(&settings_path).await;
        }
    }
}

pub async fn reload_no_default_maven_setting(conn: &Connection) {
    let value = load_value_from_global_settings_with_conn(
        conn,
        windmill_common::global_settings::NO_DEFAULT_MAVEN_SETTING,
        true,
    )
    .await;
    match value {
        Ok(Some(serde_json::Value::Bool(t))) => NO_DEFAULT_MAVEN.store(t, Ordering::Relaxed),
        Err(e) => {
            tracing::error!("Error loading no default maven repository: {e:#}");
        }
        _ => (),
    };
}

pub async fn reload_ruby_repos_setting(conn: &Connection) {
    reload_url_list_setting_with_tracing(
        conn,
        windmill_common::global_settings::RUBY_REPOS_SETTING,
        "RUBY_REPOS",
        windmill_worker::RUBY_REPOS.clone(),
    )
    .await;
}

pub async fn reload_cargo_registries_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        windmill_common::global_settings::CARGO_REGISTRIES_SETTING,
        "CARGO_REGISTRIES",
        CARGO_REGISTRIES.clone(),
    )
    .await;
}

pub async fn reload_hub_api_secret_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        HUB_API_SECRET_SETTING,
        "HUB_API_SECRET",
        HUB_API_SECRET.clone(),
    )
    .await;
}

pub async fn reload_retention_period_setting(conn: &Connection) {
    if let Err(e) = reload_setting(
        conn,
        RETENTION_PERIOD_SECS_SETTING,
        "JOB_RETENTION_SECS",
        60 * 60 * 24 * 30,
        JOB_RETENTION_SECS.clone(),
        |x| x,
    )
    .await
    {
        tracing::error!("Error reloading retention period: {:?}", e)
    }
}
pub async fn reload_delete_logs_periodically_setting(conn: &Connection) {
    if let Err(e) = reload_setting(
        conn,
        MONITOR_LOGS_ON_OBJECT_STORE_SETTING,
        "MONITOR_LOGS_ON_OBJECT_STORE",
        false,
        MONITOR_LOGS_ON_OBJECT_STORE.clone(),
        |x| x,
    )
    .await
    {
        tracing::error!("Error reloading retention period: {:?}", e)
    }
}

pub async fn reload_job_default_timeout_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        JOB_DEFAULT_TIMEOUT_SECS_SETTING,
        "JOB_DEFAULT_TIMEOUT_SECS",
        JOB_DEFAULT_TIMEOUT.clone(),
    )
    .await;
}

pub async fn reload_job_isolation_setting(conn: &Connection) {
    let value =
        match load_value_from_global_settings_with_conn(conn, JOB_ISOLATION_SETTING, true).await {
            Ok(Some(v)) => JobIsolationLevel::from_str(v.as_str().unwrap_or("")),
            Ok(None) => JobIsolationLevel::Undefined,
            Err(e) => {
                tracing::error!("Error reloading job_isolation setting: {:?}", e);
                return;
            }
        };
    let old_value = JobIsolationLevel::from_u8(JOB_ISOLATION.swap(value as u8, Ordering::Relaxed));
    if old_value != value {
        tracing::info!(
            "job_isolation setting changed from {:?} to {:?}",
            old_value,
            value
        );
    }
    if value == JobIsolationLevel::NsjailSandboxing && NSJAIL_AVAILABLE.is_none() {
        tracing::error!(
            "job_isolation is set to nsjail_sandboxing but nsjail is not available on this worker. \
            All jobs will fail until nsjail is installed or the setting is changed."
        );
    }
}

pub async fn reload_request_size(conn: &Connection) {
    if let Err(e) = reload_setting(
        conn,
        REQUEST_SIZE_LIMIT_SETTING,
        "REQUEST_SIZE_LIMIT",
        DEFAULT_BODY_LIMIT,
        REQUEST_SIZE_LIMIT.clone(),
        |x| x.mul(1024 * 1024),
    )
    .await
    {
        tracing::error!("Error reloading retention period: {:?}", e)
    }
}

pub async fn reload_license_key(conn: &Connection) -> anyhow::Result<()> {
    let q = load_value_from_global_settings_with_conn(conn, LICENSE_KEY_SETTING, true)
        .await
        .map_err(|err| anyhow::anyhow!("Error reloading license key: {}", err.to_string()))?;

    let mut value = std::env::var("LICENSE_KEY")
        .ok()
        .and_then(|x| x.parse::<String>().ok())
        .unwrap_or(String::new());

    if let Some(q) = q {
        if let Ok(v) = serde_json::from_value::<String>(q.clone()) {
            tracing::info!(
                "Loaded setting LICENSE_KEY from db config: {}",
                truncate_token(&v)
            );
            value = v;
        } else {
            tracing::error!("Could not parse LICENSE_KEY found: {:#?}", &q);
        }
    };
    set_license_key(value, conn.as_sql()).await;
    Ok(())
}

pub async fn reload_option_setting_with_tracing<T: FromStr + DeserializeOwned>(
    conn: &Connection,
    setting_name: &str,
    std_env_var: &str,
    lock: Arc<RwLock<Option<T>>>,
) {
    if let Err(e) = reload_option_setting(conn, setting_name, std_env_var, lock.clone()).await {
        tracing::error!("Error reloading setting {}: {:?}", setting_name, e)
    }
}

pub async fn load_value_from_global_settings(
    db: &DB,
    setting_name: &str,
) -> error::Result<Option<serde_json::Value>> {
    let r = sqlx::query!(
        "SELECT value FROM global_settings WHERE name = $1",
        setting_name
    )
    .fetch_optional(db)
    .await?
    .map(|x| x.value);
    Ok(r)
}

pub async fn load_value_from_global_settings_with_conn(
    conn: &Connection,
    setting_name: &str,
    load_from_http: bool,
) -> anyhow::Result<Option<serde_json::Value>> {
    match conn {
        Connection::Sql(db) => Ok(load_value_from_global_settings(db, setting_name).await?),
        Connection::Http(client) => {
            if load_from_http {
                client
                    .get::<Option<serde_json::Value>>(&format!(
                        "/api/agent_workers/get_global_setting/{}",
                        setting_name
                    ))
                    .await
                    .map_err(|e| anyhow::anyhow!("Error loading setting {}: {}", setting_name, e))
            } else {
                Ok(None)
            }
        }
    }
}

pub async fn reload_option_setting<T: FromStr + DeserializeOwned>(
    conn: &Connection,
    setting_name: &str,
    std_env_var: &str,
    lock: Arc<RwLock<Option<T>>>,
) -> error::Result<()> {
    let force_value = std::env::var(format!("FORCE_{}", std_env_var))
        .ok()
        .and_then(|x| x.parse::<T>().ok());

    if let Some(force_value) = force_value {
        let mut l = lock.write().await;
        *l = Some(force_value);
        return Ok(());
    }

    let q = load_value_from_global_settings_with_conn(conn, setting_name, true).await?;

    let mut value = std::env::var(std_env_var)
        .ok()
        .and_then(|x| x.parse::<T>().ok());

    if let Some(q) = q {
        if let Ok(v) = serde_json::from_value::<T>(q.clone()) {
            tracing::info!("Loaded setting {setting_name} from db config: {:#?}", &q);
            value = Some(v)
        } else {
            tracing::error!("Could not parse {setting_name} found: {:#?}", &q);
        }
    };

    {
        if value.is_none() {
            tracing::info!("Loaded {setting_name} setting to None");
        }
        let mut l = lock.write().await;
        *l = value;
    }

    Ok(())
}

pub async fn reload_url_list_setting_with_tracing(
    conn: &Connection,
    setting_name: &str,
    std_env_var: &str,
    lock: Arc<RwLock<Option<Vec<url::Url>>>>,
) {
    if let Err(e) = reload_url_list_setting(conn, setting_name, std_env_var, lock.clone()).await {
        tracing::error!("Error reloading setting {}: {:?}", setting_name, e)
    }
}

pub async fn reload_url_list_setting(
    conn: &Connection,
    setting_name: &str,
    std_env_var: &str,
    lock: Arc<RwLock<Option<Vec<url::Url>>>>,
) -> error::Result<()> {
    // Check for force environment variable
    if let Ok(force_value) = std::env::var(format!("FORCE_{}", std_env_var)) {
        let mut urls = Vec::new();
        for url_str in force_value.trim().split_whitespace() {
            match url::Url::parse(url_str) {
                Ok(url) => urls.push(url),
                Err(e) => {
                    return Err(error::Error::BadRequest(format!(
                        "Invalid URL in FORCE_{}: '{}': {}",
                        std_env_var, url_str, e
                    )));
                }
            }
        }
        let mut l = lock.write().await;
        *l = if urls.is_empty() { None } else { Some(urls) };
        return Ok(());
    }

    let q = load_value_from_global_settings_with_conn(conn, setting_name, true).await?;

    // Check regular environment variable
    let mut value = if let Ok(env_value) = std::env::var(std_env_var) {
        let mut urls = Vec::new();
        for url_str in env_value.trim().split_whitespace() {
            match url::Url::parse(url_str) {
                Ok(url) => urls.push(url),
                Err(_) => {
                    // Log error but continue, similar to force variable handling
                    tracing::error!("Invalid URL in {}: '{}'", std_env_var, url_str);
                }
            }
        }
        if urls.is_empty() {
            None
        } else {
            Some(urls)
        }
    } else {
        None
    };

    // Check database setting
    if let Some(q) = q {
        if let Ok(repos_str) = serde_json::from_value::<String>(q.clone()) {
            let mut urls = Vec::new();
            for url_str in repos_str.trim().split_whitespace() {
                match url::Url::parse(url_str) {
                    Ok(url) => urls.push(url),
                    Err(e) => {
                        tracing::error!("Invalid URL in {}: '{}': {}", setting_name, url_str, e);
                        // Continue with other URLs, just skip invalid ones
                    }
                }
            }
            tracing::info!(
                "Loaded setting {} from db config: {} URLs",
                setting_name,
                urls.len()
            );
            value = if urls.is_empty() { None } else { Some(urls) };
        } else {
            tracing::error!("Could not parse {} found: {:#?}", setting_name, &q);
        }
    }

    {
        if value.is_none() {
            tracing::info!("Loaded {} setting to None", setting_name);
        }
        let mut l = lock.write().await;
        *l = value;
    }

    Ok(())
}

pub async fn reload_setting<T: FromStr + DeserializeOwned + Display>(
    conn: &Connection,
    setting_name: &str,
    std_env_var: &str,
    default: T,
    lock: Arc<RwLock<T>>,
    transformer: fn(T) -> T,
) -> error::Result<()> {
    let q = load_value_from_global_settings_with_conn(conn, setting_name, true).await?;

    let mut value = std::env::var(std_env_var)
        .ok()
        .and_then(|x| x.parse::<T>().ok())
        .unwrap_or(default);

    if let Some(q) = q {
        if let Ok(v) = serde_json::from_value::<T>(q.clone()) {
            tracing::info!("Loaded setting {setting_name} from db config: {:#?}", &q);
            value = transformer(v);
        } else {
            tracing::error!("Could not parse {setting_name} found: {:#?}", &q);
        }
    };

    {
        let mut l = lock.write().await;
        *l = value;
    }

    Ok(())
}

#[cfg(feature = "prometheus")]
pub async fn monitor_pool(db: &DB) {
    if METRICS_ENABLED.load(Ordering::Relaxed) {
        let db = db.clone();
        tokio::spawn(async move {
            let active_pool_connections: prometheus::IntGauge = prometheus::register_int_gauge!(
                "pool_connections_active",
                "Number of active postgresql connections in the pool"
            )
            .unwrap();

            let idle_pool_connections: prometheus::IntGauge = prometheus::register_int_gauge!(
                "pool_connections_idle",
                "Number of idle postgresql connections in the pool"
            )
            .unwrap();

            let max_pool_connections: prometheus::IntGauge = prometheus::register_int_gauge!(
                "pool_connections_max",
                "Number of max postgresql connections in the pool"
            )
            .unwrap();

            max_pool_connections.set(db.options().get_max_connections() as i64);
            loop {
                active_pool_connections.set(db.size() as i64);
                idle_pool_connections.set(db.num_idle() as i64);
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });
    }
}

pub struct MonitorIteration {
    pub rd_shift: u8,
    pub iter: u64,
}

impl MonitorIteration {
    pub fn should_run(&self, period: u8) -> bool {
        (self.iter + self.rd_shift as u64) % (period as u64) == 0
    }
}

pub async fn monitor_db(
    conn: &Connection,
    base_internal_url: &str,
    server_mode: bool,
    _worker_mode: bool,
    initial_load: bool,
    _killpill_tx: KillpillSender,
    iteration: Option<MonitorIteration>,
) {
    let zombie_jobs_f = async {
        if server_mode && !initial_load && !*DISABLE_ZOMBIE_JOBS_MONITORING {
            if let Some(db) = conn.as_sql() {
                handle_zombie_jobs(db, base_internal_url, "server").await;
                match handle_zombie_flows(db).await {
                    Err(err) => {
                        tracing::error!("Error handling zombie flows: {:?}", err);
                    }
                    _ => {}
                }
            }
        }
    };

    let stale_jobs_f = async {
        if server_mode && !initial_load {
            if let Some(db) = conn.as_sql() {
                stale_job_cancellation(&db).await;
            }
        }
    };

    // run every 5 minutes
    let cleanup_concurrency_counters_f = async {
        if server_mode && iteration.is_some() && iteration.as_ref().unwrap().should_run(10) {
            if let Some(db) = conn.as_sql() {
                if let Err(e) = cleanup_concurrency_counters_orphaned_keys(&db).await {
                    tracing::error!("Error cleaning up concurrency counters: {:?}", e);
                }
            }
        }
    };

    // run every 10 minutes
    let cleanup_concurrency_counters_empty_keys_f = async {
        if server_mode && iteration.is_some() && iteration.as_ref().unwrap().should_run(20) {
            if let Some(db) = conn.as_sql() {
                if let Err(e) = cleanup_concurrency_counters_empty_keys(&db).await {
                    tracing::error!("Error cleaning up concurrency counters: {:?}", e);
                }
            }
        }
    };

    let cleanup_debounce_keys_f = async {
        if server_mode && iteration.is_some() && iteration.as_ref().unwrap().should_run(10) {
            if let Some(db) = conn.as_sql() {
                if let Err(e) = cleanup_debounce_orphaned_keys(&db).await {
                    tracing::error!("Error cleaning up debounce keys: {:?}", e);
                }
            }
        }
    };

    // run every 30s (every iteration)
    let cleanup_debounce_keys_completed_f = async {
        if server_mode && !initial_load {
            if let Some(db) = conn.as_sql() {
                if let Err(e) = cleanup_debounce_keys_for_completed_jobs(&db).await {
                    tracing::error!(
                        "Error cleaning up debounce keys for completed jobs: {:?}",
                        e
                    );
                }
            }
        }
    };

    let cleanup_flow_iterator_data_f = async {
        if server_mode && iteration.is_some() && iteration.as_ref().unwrap().should_run(10) {
            if let Some(db) = conn.as_sql() {
                if let Err(e) = cleanup_flow_iterator_data_orphaned_jobs(&db).await {
                    tracing::error!("Error cleaning up flow_iterator_data: {:?}", e);
                }
            }
        }
    };

    let cleanup_job_live_rows_f = async {
        if server_mode && !initial_load {
            if let Some(db) = conn.as_sql() {
                if let Err(e) = cleanup_job_perms_orphaned(&db).await {
                    tracing::error!("Error cleaning up orphaned job_perms: {:?}", e);
                }
                if let Err(e) = cleanup_job_result_stream_orphaned_jobs(&db).await {
                    tracing::error!("Error cleaning up orphaned job_result_stream_v2: {:?}", e);
                }
            }
        }
    };

    // run every hour (60 minutes / 30 seconds = 120)
    let cleanup_worker_group_stats_f = async {
        if server_mode && iteration.is_some() && iteration.as_ref().unwrap().should_run(120) {
            if let Some(db) = conn.as_sql() {
                match windmill_common::worker_group_job_stats::cleanup_old_stats(db, 60).await {
                    Ok(count) if count > 0 => {
                        tracing::info!("Deleted {} old worker group job stats rows", count);
                    }
                    Err(e) => {
                        tracing::error!("Error cleaning up worker group job stats: {:?}", e);
                    }
                    _ => {}
                }
            }
        }
    };

    // run every hour
    let vacuum_queue_f = async {
        if server_mode && iteration.is_some() && iteration.as_ref().unwrap().should_run(60) {
            if let Some(db) = conn.as_sql() {
                let instant = Instant::now();
                tracing::info!("vacuuming tables");
                if let Err(e) = vacuuming_tables(&db).await {
                    tracing::error!("Error vacuuming v2_job: {:?}", e);
                }
                tracing::info!("vacuum tables done in {}s", instant.elapsed().as_secs());
            }
        }
    };

    let expired_items_f = async {
        if server_mode && !initial_load {
            if let Some(db) = conn.as_sql() {
                delete_expired_items(&db).await;
            }
        }
    };

    let verify_license_key_f = async {
        #[cfg(feature = "enterprise")]
        if !initial_load {
            verify_license_key().await;
        }
    };

    let expose_queue_metrics_f = async {
        if !initial_load && server_mode {
            if let Some(db) = conn.as_sql() {
                expose_queue_metrics(&db).await;
            }
        }
    };

    let worker_groups_alerts_f = async {
        #[cfg(feature = "enterprise")]
        if server_mode && !initial_load {
            if let Some(db) = conn.as_sql() {
                worker_groups_alerts(&db).await;
            }
        }
    };

    let jobs_waiting_alerts_f = async {
        #[cfg(feature = "enterprise")]
        if server_mode {
            if let Some(db) = conn.as_sql() {
                jobs_waiting_alerts(&db).await;
            }
        }
    };

    let low_disk_alerts_f = async {
        #[cfg(feature = "enterprise")]
        if let Some(db) = conn.as_sql() {
            low_disk_alerts(
                &db,
                server_mode,
                _worker_mode,
                WORKERS_NAMES.read().await.clone(),
            )
            .await;
        }
        #[cfg(not(feature = "enterprise"))]
        {
            ()
        }
    };

    let apply_autoscaling_f = async {
        #[cfg(feature = "enterprise")]
        if server_mode && !initial_load {
            if let Some(db) = conn.as_sql() {
                if let Err(e) = windmill_autoscaling::apply_all_autoscaling(db).await {
                    tracing::error!("Error applying autoscaling: {:?}", e);
                }
            }
        }
    };

    let update_min_worker_version_f = async {
        windmill_common::min_version::update_min_version(
            conn,
            _worker_mode,
            WORKERS_NAMES.read().await.clone(),
            initial_load,
        )
        .await;
    };

    // Run every 5 minutes (10 iterations * 30s = 5 minutes)
    let native_triggers_sync_f = async {
        #[cfg(feature = "native_trigger")]
        if server_mode && iteration.is_some() && iteration.as_ref().unwrap().should_run(10) {
            if let Some(db) = conn.as_sql() {
                match sync_all_triggers(db).await {
                    Ok(result) => {
                        tracing::debug!(
                            "Native triggers sync completed: {} workspaces, {} synced, {} errors",
                            result.workspaces_processed,
                            result.total_synced,
                            result.total_errors
                        );
                        if result.total_errors > 0 {
                            tracing::warn!(
                                "Native triggers sync encountered {} errors",
                                result.total_errors
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error during native triggers sync: {:#}", e);
                    }
                }
            }
        }
    };

    // Run every 5 minutes (10 iterations * 30s = 5 minutes)
    // Cleanup old notify events (older than 10 minutes)
    let cleanup_notify_events_f = async {
        if server_mode && iteration.is_some() && iteration.as_ref().unwrap().should_run(10) {
            if let Some(db) = conn.as_sql() {
                match windmill_common::notify_events::cleanup_old_events(db, 10).await {
                    Ok(count) if count > 0 => {
                        tracing::debug!("Cleaned up {} old notify events", count);
                    }
                    Err(e) => {
                        tracing::error!("Error cleaning up notify events: {:?}", e);
                    }
                    _ => {}
                }
            }
        }
    };

    join!(
        expired_items_f,
        zombie_jobs_f,
        stale_jobs_f,
        vacuum_queue_f,
        expose_queue_metrics_f,
        verify_license_key_f,
        worker_groups_alerts_f,
        jobs_waiting_alerts_f,
        low_disk_alerts_f,
        apply_autoscaling_f,
        update_min_worker_version_f,
        cleanup_concurrency_counters_f,
        cleanup_concurrency_counters_empty_keys_f,
        cleanup_debounce_keys_f,
        cleanup_debounce_keys_completed_f,
        cleanup_flow_iterator_data_f,
        cleanup_job_live_rows_f,
        cleanup_worker_group_stats_f,
        native_triggers_sync_f,
        cleanup_notify_events_f,
    );
}

async fn vacuuming_tables(db: &Pool<Postgres>) -> error::Result<()> {
    sqlx::query!("VACUUM v2_job, v2_job_completed, job_result_stream_v2, job_stats, job_logs, job_perms, concurrency_key, log_file, metrics")
        .execute(db)
        .await?;
    Ok(())
}

pub async fn expose_queue_metrics(db: &Pool<Postgres>) {
    let last_check = sqlx::query_scalar!(
            "SELECT created_at FROM metrics WHERE id LIKE 'queue_count_%' ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(db)
        .await
        .unwrap_or(Some(chrono::Utc::now()));

    let metrics_enabled = METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed);
    let save_metrics = last_check
        .map(|last_check| chrono::Utc::now() - last_check > chrono::Duration::seconds(25))
        .unwrap_or(true);

    if metrics_enabled || save_metrics {
        let queue_counts = windmill_common::queue::get_queue_counts(db).await;

        #[cfg(feature = "prometheus")]
        if metrics_enabled {
            for q in QUEUE_COUNT_TAGS.read().await.iter() {
                if queue_counts.get(q).is_none() {
                    (*QUEUE_COUNT).with_label_values(&[q]).set(0);
                }
            }
        }

        #[allow(unused_mut)]
        let mut tags_to_watch = vec![];
        for q in queue_counts {
            let count = q.1;
            let tag = q.0;

            #[cfg(feature = "prometheus")]
            if metrics_enabled {
                let metric = (*QUEUE_COUNT).with_label_values(&[&tag]);
                metric.set(count as i64);
                tags_to_watch.push(tag.to_string());
            }

            // save queue_count and delay metrics per tag
            if save_metrics {
                sqlx::query!(
                    "INSERT INTO metrics (id, value) VALUES ($1, $2)",
                    format!("queue_count_{}", tag),
                    serde_json::json!(count)
                )
                .execute(db)
                .await
                .ok();
                if count > 0 {
                    sqlx::query!(
                        "INSERT INTO metrics (id, value)
                        VALUES ($1, to_jsonb((
                            SELECT EXTRACT(EPOCH FROM now() - scheduled_for)
                            FROM v2_job_queue
                            WHERE tag = $2 AND running = false AND scheduled_for <= now() - ('3 seconds')::interval
                            ORDER BY priority DESC NULLS LAST, scheduled_for LIMIT 1
                        )))",
                        format!("queue_delay_{}", tag),
                        tag
                    )
                    .execute(db)
                    .await
                    .ok();
                }
            }
        }
        if metrics_enabled {
            let mut w = QUEUE_COUNT_TAGS.write().await;
            *w = tags_to_watch;
        }

        #[cfg(feature = "prometheus")]
        if metrics_enabled {
            // Handle queue running count metrics
            let queue_running_counts = windmill_common::queue::get_queue_running_counts(db).await;

            for q in QUEUE_RUNNING_COUNT_TAGS.read().await.iter() {
                if queue_running_counts.get(q).is_none() {
                    (*QUEUE_RUNNING_COUNT).with_label_values(&[q]).set(0);
                }
            }

            let mut running_tags_to_watch = vec![];
            for q in queue_running_counts {
                let count = q.1;
                let tag = q.0;

                let metric = (*QUEUE_RUNNING_COUNT).with_label_values(&[&tag]);
                metric.set(count as i64);
                running_tags_to_watch.push(tag.to_string());
            }
            let mut w = QUEUE_RUNNING_COUNT_TAGS.write().await;
            *w = running_tags_to_watch;
        }
    }

    // clean queue metrics older than 14 days
    sqlx::query!(
        "DELETE FROM metrics WHERE id LIKE 'queue_%' AND created_at < NOW() - INTERVAL '14 day'"
    )
    .execute(db)
    .await
    .ok();
}

pub async fn reload_smtp_config(db: &Pool<Postgres>) {
    let smtp_config = load_smtp_config(&db).await;
    if let Err(e) = smtp_config {
        tracing::error!("Error reloading smtp config: {:?}", e)
    } else {
        let mut wc = SMTP_CONFIG.write().await;
        tracing::info!("Reloading smtp config...");
        *wc = smtp_config.unwrap()
    }
}

pub async fn reload_indexer_config(db: &Pool<Postgres>) {
    let indexer_config = load_indexer_config(&db).await;
    if let Err(e) = indexer_config {
        tracing::error!("Error reloading indexer config: {:?}", e)
    } else {
        let mut wc = INDEXER_CONFIG.write().await;
        tracing::info!("Reloading smtp config...");
        *wc = indexer_config.unwrap()
    }
}

pub async fn reload_worker_config(db: &DB, tx: KillpillSender, kill_if_change: bool) {
    let config = load_worker_config(db, tx.clone()).await;
    if let Err(e) = config {
        tracing::error!("Error reloading worker config: {:?}", e)
    } else {
        let wc = WORKER_CONFIG.read().await;
        let config = config.unwrap();
        let has_dedicated = config.dedicated_worker.is_some()
            || config
                .dedicated_workers
                .as_ref()
                .is_some_and(|dws| !dws.is_empty());
        if *wc != config || has_dedicated {
            if kill_if_change {
                if has_dedicated
                    || (*wc).dedicated_worker != config.dedicated_worker
                    || (*wc).dedicated_workers != config.dedicated_workers
                {
                    tracing::info!("Dedicated worker config changed, sending killpill. Expecting to be restarted by supervisor.");
                    let _ = tx.send();
                }

                if (*wc).init_bash != config.init_bash {
                    tracing::info!("Init bash config changed, sending killpill. Expecting to be restarted by supervisor.");
                    let _ = tx.send();
                }

                if (*wc).cache_clear != config.cache_clear {
                    tracing::info!("Cache clear changed, sending killpill. Expecting to be restarted by supervisor.");
                    let _ = tx.send();
                    tracing::info!("Waiting 5 seconds to allow others workers to start potential jobs that depend on a potential shared cache volume");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    if let Err(e) = windmill_worker::common::clean_cache().await {
                        tracing::error!("Error cleaning the cache: {e:#}");
                    }
                }

                if (*wc).periodic_script_bash != config.periodic_script_bash {
                    tracing::info!("Periodic script bash config changed, sending killpill. Expecting to be restarted by supervisor.");
                    let _ = tx.send();
                }

                if (*wc).periodic_script_interval_seconds != config.periodic_script_interval_seconds
                {
                    tracing::info!("Periodic script interval config changed, sending killpill. Expecting to be restarted by supervisor.");
                    let _ = tx.send();
                }

                if (*wc).native_mode != config.native_mode {
                    tracing::info!("Native mode config changed, sending killpill. Expecting to be restarted by supervisor.");
                    let _ = tx.send();
                }
            }
            drop(wc);

            let mut wc = WORKER_CONFIG.write().await;
            tracing::info!("Reloading worker config...");
            store_suspended_pull_query(&config).await;
            store_pull_query(&config).await;
            *wc = config
        }
    }
}

pub async fn load_base_url(conn: &Connection) -> error::Result<String> {
    let q_base_url =
        load_value_from_global_settings_with_conn(conn, BASE_URL_SETTING, false).await?;

    let std_base_url = std::env::var("BASE_URL")
        .ok()
        .unwrap_or_else(|| "http://localhost".to_string());
    let base_url = if let Some(q) = q_base_url {
        if let Ok(v) = serde_json::from_value::<String>(q.clone()) {
            if v != "" {
                v
            } else {
                std_base_url
            }
        } else {
            tracing::error!(
                "Could not parse base_url setting as a string, found: {:#?}",
                &q
            );
            std_base_url
        }
    } else {
        std_base_url
    };
    {
        let mut l = BASE_URL.write().await;
        *l = base_url.clone();
    }
    Ok(base_url)
}

pub async fn reload_base_url_setting(conn: &Connection) -> error::Result<()> {
    #[cfg(feature = "oauth2")]
    let oauths = if let Some(db) = conn.as_sql() {
        let q_oauth = load_value_from_global_settings(db, OAUTH_SETTING).await?;

        if let Some(q) = q_oauth {
            if let Ok(v) = serde_json::from_value::<
                Option<HashMap<String, windmill_api::oauth2_oss::OAuthClient>>,
            >(q.clone())
            {
                v
            } else {
                tracing::error!("Could not parse oauth setting as a json, found: {:#?}", &q);
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    let base_url = load_base_url(conn).await?;
    let is_secure = base_url.starts_with("https://");

    #[cfg(feature = "oauth2")]
    {
        if let Some(db) = conn.as_sql() {
            let mut l = windmill_api::OAUTH_CLIENTS.write().await;
            *l = windmill_api::oauth2_oss::build_oauth_clients(&base_url, oauths, db).await
            .map_err(|e| tracing::error!("Error building oauth clients (is the oauth.json mounted and in correct format? Use '{}' as minimal oauth.json): {}", "{}", e))
            .unwrap();
        }
    }

    {
        let mut l = IS_SECURE.write().await;
        *l = is_secure;
    }

    Ok(())
}

async fn stale_job_cancellation(db: &Pool<Postgres>) {
    if let Some(threshold) = *STALE_JOB_THRESHOLD_MINUTES {
        let stale_jobs = sqlx::query!(
            "SELECT v2_job_queue.id, v2_job.tag, v2_job_queue.scheduled_for, v2_job_queue.workspace_id FROM v2_job_queue LEFT JOIN v2_job ON v2_job_queue.id = v2_job.id WHERE running = false AND scheduled_for < now() - ($1 || ' minutes')::interval",
            threshold.to_string()
        )
        .fetch_all(db)
            .await
            .ok()
            .unwrap_or_else(|| vec![]);

        if !stale_jobs.is_empty() {
            tracing::info!(
                "Cancelling {} stale jobs (> {} minutes old)",
                stale_jobs.len(),
                threshold
            );
        }
        for job in stale_jobs {
            if let Err(e) =
                cancel_stale_job(db, job.id, job.tag, job.workspace_id, job.scheduled_for).await
            {
                tracing::error!("Error cancelling stale job {}: {}", job.id, e);
            }
        }
    }
}

async fn cancel_stale_job(
    db: &Pool<Postgres>,
    id: Uuid,
    tag: String,
    workspace_id: String,
    scheduled_for: DateTime<Utc>,
) -> error::Result<()> {
    let mut tx = db.begin().await?;
    tracing::error!(
        "Stale job detected: {} in workspace {} with tag {} (scheduled for: {}) . Cancelling it.",
        id,
        workspace_id,
        tag,
        scheduled_for
    );
    (tx, _) = cancel_job(
        "monitor",
        Some(format!(
            "Stale job cancellation (scheduled for: {})",
            scheduled_for
        )),
        id,
        &workspace_id,
        tx,
        db,
        true,
        false,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

const RESTART_LIMIT: i32 = 3;

async fn handle_zombie_jobs(db: &Pool<Postgres>, base_internal_url: &str, node_name: &str) {
    let mut zombie_jobs_uuid_restart_limit_reached = vec![];

    if *RESTART_ZOMBIE_JOBS {
        let restarted = sqlx::query!(
            "WITH to_update AS (
                SELECT q.id, q.workspace_id, r.ping, COALESCE(zjc.counter, 0) as counter
                FROM v2_job_queue q
                JOIN v2_job j ON j.id = q.id
                JOIN v2_job_runtime r ON r.id = j.id
                LEFT JOIN zombie_job_counter zjc ON zjc.job_id = q.id
                WHERE ping < now() - ($1 || ' seconds')::interval
                    AND running = true
                    AND kind NOT IN ('flow', 'flowpreview', 'flownode', 'singlestepflow')
                    AND same_worker = false
                    AND (zjc.counter IS NULL OR zjc.counter <= $2)
                FOR UPDATE of q SKIP LOCKED
            ),
            zombie_jobs AS (
                UPDATE v2_job_queue q
                SET running = false, started_at = null
                FROM to_update tu
                WHERE q.id = tu.id AND (tu.counter IS NULL OR tu.counter < $2)
                RETURNING q.id, q.workspace_id, ping, tu.counter
            ),
            update_ping AS (
                UPDATE v2_job_runtime r
                SET ping = null
                FROM zombie_jobs zj
                WHERE r.id = zj.id
            ),
            increment_counter AS (
                INSERT INTO zombie_job_counter (job_id, counter)
                SELECT id, 1 FROM to_update WHERE counter < $2
                ON CONFLICT (job_id) DO UPDATE
                SET counter = zombie_job_counter.counter + 1
            ),
            update_concurrency AS (
                UPDATE concurrency_counter cc
                SET job_uuids = job_uuids - zj.id::text
                FROM zombie_jobs zj
                INNER JOIN concurrency_key ck ON ck.job_id = zj.id
                WHERE cc.concurrency_id = ck.key
            )
            SELECT id AS \"id!\", workspace_id AS \"workspace_id!\", ping, counter + 1 AS counter FROM to_update",
            *ZOMBIE_JOB_TIMEOUT,
            RESTART_LIMIT
        )
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![]);

        #[cfg(feature = "prometheus")]
        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            QUEUE_ZOMBIE_RESTART_COUNT.inc_by(restarted.len() as _);
        }

        let base_url = BASE_URL.read().await.clone();
        for r in restarted {
            let last_ping = if let Some(x) = r.ping {
                format!("last ping at {x}")
            } else {
                "no last ping".to_string()
            };
            let url = format!("{}/run/{}?workspace={}", base_url, r.id, r.workspace_id,);
            let restart = r.counter.is_none_or(|x| x < RESTART_LIMIT);
            let (critical_error_message, restart_message) = if restart {
                (
                        format!(
                        "Zombie job {} on {} ({}) detected, restarting it ({}/{} attempts), last ping: {}",
                        r.id,
                        r.workspace_id,
                        url,
                        r.counter.unwrap_or(0) + 1,
                        RESTART_LIMIT,
                        last_ping
                    ),
                    format!(
                        "Restarted job after not receiving job's ping for too long the {} ({}/{} attempts)\n\n",
                        last_ping,
                        r.counter.unwrap_or(0) + 1,
                        RESTART_LIMIT
                    )
                )
            } else {
                (
                        format!(
                        "Zombie job {} on {} ({}) detected, but restart limit ({}) reached, job will be processed as an error, last ping: {}",
                        r.id, r.workspace_id, url, RESTART_LIMIT, last_ping
                    ),
                    format!(
                        "job's ping was received last at {}, job will be processed as an error since all {} restart attempts failed",
                        last_ping, RESTART_LIMIT
                    )
                )
            };

            let _ = sqlx::query!(
                "
                INSERT INTO job_logs (job_id, logs)
                VALUES ($1, $2)
                ON CONFLICT (job_id) DO UPDATE SET logs = job_logs.logs || '\n' || EXCLUDED.logs
                WHERE job_logs.job_id = $1",
                r.id,
                restart_message
            )
            .execute(db)
            .await;
            tracing::error!(critical_error_message);
            report_critical_error(
                critical_error_message,
                db.clone(),
                Some(&r.workspace_id),
                None,
            )
            .await;

            if !restart {
                zombie_jobs_uuid_restart_limit_reached.push(r.id);
            }
        }
    }

    let same_worker_timeout_jobs = {
        let long_same_worker_jobs = sqlx::query!(
            "SELECT worker, array_agg(v2_job_queue.id) as ids FROM v2_job_queue LEFT JOIN v2_job ON v2_job_queue.id = v2_job.id LEFT JOIN v2_job_runtime ON v2_job_queue.id = v2_job_runtime.id WHERE v2_job_queue.created_at < now() - ('60 seconds')::interval
    AND running = true AND (ping IS NULL OR ping < now() - ('60 seconds')::interval) AND same_worker = true AND worker IS NOT NULL GROUP BY worker",
        )
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![]);

        let worker_ids = long_same_worker_jobs
            .iter()
            .map(|x| x.worker.clone().unwrap_or_default())
            .collect::<Vec<_>>();

        let long_dead_workers: std::collections::HashSet<String> = sqlx::query_scalar!(
            "WITH worker_ids AS (SELECT unnest($1::text[]) as worker)
            SELECT worker_ids.worker FROM worker_ids
            LEFT JOIN worker_ping ON worker_ids.worker = worker_ping.worker
                WHERE worker_ping.worker IS NULL OR worker_ping.ping_at < now() - ('60 seconds')::interval",
            &worker_ids[..]
        )
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![])
        .into_iter()
        .filter_map(|x| x)
        .collect();

        let mut timeouts: Vec<Uuid> = vec![];
        for worker in long_same_worker_jobs {
            if worker.worker.is_some() && long_dead_workers.contains(&worker.worker.unwrap()) {
                if let Some(ids) = worker.ids {
                    timeouts.extend(ids);
                }
            }
        }
        if !timeouts.is_empty() {
            tracing::error!(
                "Failing same worker zombie jobs: {:?}",
                timeouts
                    .iter()
                    .map(|x| x.hyphenated().to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }

        timeouts
    };

    let non_restartable_jobs = if *RESTART_ZOMBIE_JOBS {
        vec![]
    } else {
        sqlx::query_scalar!("SELECT j.id
             FROM v2_job_queue q JOIN v2_job j USING (id) LEFT JOIN v2_job_runtime r USING (id) LEFT JOIN v2_job_status s USING (id)
             WHERE r.ping < now() - ($1 || ' seconds')::interval
             AND q.running = true AND j.kind NOT IN ('flow', 'flowpreview', 'flownode', 'singlestepflow') AND j.same_worker = false",
             ZOMBIE_JOB_TIMEOUT.as_str())
        .fetch_all(db)
        .await
        .ok()
            .unwrap_or_else(|| vec![])
    };

    enum ErrorMessage {
        RestartLimit,
        SameWorker,
        RestartDisabled,
    }

    impl ErrorMessage {
        fn to_string(&self) -> String {
            match self {
                ErrorMessage::RestartLimit => format!("RestartLimit ({})", RESTART_LIMIT),
                ErrorMessage::SameWorker => "SameWorker".to_string(),
                ErrorMessage::RestartDisabled => "RestartDisabled".to_string(),
            }
        }
    }

    let timeouts = non_restartable_jobs
        .into_iter()
        .map(|x| (x, ErrorMessage::RestartDisabled))
        .chain(
            same_worker_timeout_jobs
                .into_iter()
                .map(|x| (x, ErrorMessage::SameWorker)),
        )
        .chain(
            zombie_jobs_uuid_restart_limit_reached
                .into_iter()
                .map(|x| (x, ErrorMessage::RestartLimit)),
        )
        .collect::<Vec<_>>();

    #[cfg(feature = "prometheus")]
    if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        QUEUE_ZOMBIE_DELETE_COUNT.inc_by(timeouts.len() as _);
    }

    for (job_id, error_kind) in timeouts {
        // since the job is unrecoverable, the same worker queue should never be sent anything
        let (same_worker_tx_never_used, _same_worker_rx_never_used) =
            mpsc::channel::<SameWorkerPayload>(1);
        let same_worker_tx_never_used =
            SameWorkerSender(same_worker_tx_never_used, Arc::new(AtomicU16::new(0)));
        let (send_result_never_used, _send_result_rx_never_used) =
            JobCompletedSender::new_never_used();

        let job = get_queued_job_v2(db, &job_id).await;
        if let Err(e) = job {
            tracing::error!("Error getting queued job: {:?}", e);
            continue;
        }
        if let Some(job) = job.unwrap() {
            let label = if job.permissioned_as != format!("u/{}", job.created_by)
                && job.permissioned_as != job.created_by
            {
                format!("ephemeral-script-end-user-{}", job.created_by)
            } else {
                "ephemeral-script".to_string()
            };
            let token = create_token_for_owner(
                &db,
                &job.workspace_id,
                &job.permissioned_as,
                &label,
                *SCRIPT_TOKEN_EXPIRY,
                &job.permissioned_as_email,
                &job.id,
                None,
                Some(format!("handle_zombie_jobs")),
            )
            .await
            .expect("could not create job token");

            let client = AuthedClient::new(
                base_internal_url.to_string(),
                job.workspace_id.to_string(),
                token,
                None,
            );

            let error_message = format!(
                "Job timed out after no ping from job since {} (ZOMBIE_JOB_TIMEOUT: {}, reason: {:?}).\nThis likely means that the job died on worker {}, OOM are a common reason for worker crashes.\nCheck the workers around the time of the last ping and the exit code if any.",
                job.last_ping.unwrap_or_default(),
                *ZOMBIE_JOB_TIMEOUT,
                error_kind.to_string(),
                job.worker.clone().unwrap_or_default(),
            );
            let memory_peak = job.memory_peak.unwrap_or(0);
            let (_, killpill_rx_never_used) = KillpillSender::new(1);
            let _ = handle_job_error(
                db,
                &client,
                &windmill_queue::MiniCompletedJob::from(job),
                memory_peak,
                None,
                error::Error::ExecutionErr(error_message),
                matches!(error_kind, ErrorMessage::SameWorker), // unrecoverable if the job is a same worker zombie
                Some(&same_worker_tx_never_used),
                "",
                node_name,
                send_result_never_used,
                &killpill_rx_never_used,
                #[cfg(feature = "benchmark")]
                &mut windmill_common::bench::BenchmarkIter::new(),
            )
            .await;
        }
    }
}

async fn cleanup_concurrency_counters_orphaned_keys(db: &DB) -> error::Result<()> {
    let result = sqlx::query!(
        "
WITH lockable_counters AS (
    SELECT concurrency_id, job_uuids
    FROM concurrency_counter
    WHERE job_uuids != '{}'::jsonb
    FOR UPDATE SKIP LOCKED
),
all_job_uuids AS (
    SELECT DISTINCT jsonb_object_keys(job_uuids) AS job_uuid
    FROM lockable_counters
),
orphaned_job_uuids AS (
    SELECT job_uuid
    FROM all_job_uuids
    WHERE job_uuid NOT IN (
        SELECT id::text
        FROM v2_job_queue
        FOR SHARE SKIP LOCKED
    )
),
orphaned_array AS (
    SELECT ARRAY(SELECT job_uuid FROM orphaned_job_uuids) AS orphaned_keys
),
before_update AS (
    SELECT lc.concurrency_id, lc.job_uuids, oa.orphaned_keys
    FROM lockable_counters lc, orphaned_array oa
    WHERE lc.job_uuids ?| oa.orphaned_keys
),
affected_rows AS (
    UPDATE concurrency_counter
    SET job_uuids = job_uuids - orphaned_array.orphaned_keys
    FROM orphaned_array
    WHERE concurrency_counter.concurrency_id IN (
        SELECT concurrency_id FROM before_update
    )
    RETURNING concurrency_id, job_uuids AS updated_job_uuids
),
expanded_orphaned AS (
    SELECT bu.concurrency_id,
           bu.job_uuids AS original_job_uuids,
           unnest(bu.orphaned_keys) AS orphaned_key
    FROM before_update bu
)
SELECT
    eo.concurrency_id,
    eo.orphaned_key,
    eo.original_job_uuids,
    ar.updated_job_uuids
FROM expanded_orphaned eo
JOIN affected_rows ar ON eo.concurrency_id = ar.concurrency_id
WHERE eo.original_job_uuids ? eo.orphaned_key
ORDER BY eo.concurrency_id, eo.orphaned_key
",
    )
    .fetch_all(db)
    .await?;

    if result.len() > 0 {
        tracing::info!("Cleaned up {} concurrency counters", result.len());
        for row in result {
            tracing::info!("Concurrency counter cleaned up: concurrency_id: {}, orphaned_key: {:?}, original_job_uuids: {:?}, updated_job_uuids: {:?}", row.concurrency_id, row.orphaned_key, row.original_job_uuids, row.updated_job_uuids);
        }
    }
    Ok(())
}

async fn cleanup_concurrency_counters_empty_keys(db: &DB) -> error::Result<()> {
    let result = sqlx::query!(
        "
WITH rows_to_delete AS (
    SELECT concurrency_id
    FROM concurrency_counter

    WHERE job_uuids = '{}'::jsonb
    FOR UPDATE SKIP LOCKED
)
DELETE FROM concurrency_counter
WHERE concurrency_id IN (SELECT concurrency_id FROM rows_to_delete)  RETURNING concurrency_id",
    )
    .fetch_all(db)
    .await?;

    if result.len() > 0 {
        tracing::info!(
            "Cleaned up {} empty concurrency counters: {:?}",
            result.len(),
            result
                .iter()
                .map(|x| x.concurrency_id.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
    }

    Ok(())
}

async fn handle_zombie_flows(db: &DB) -> error::Result<()> {
    let flows = sqlx::query!(
        r#"
        SELECT
            j.id AS "id!", j.workspace_id AS "workspace_id!", j.parent_job, j.flow_step_id IS NOT NULL AS "is_flow_step?",
            COALESCE(s.flow_status, s.workflow_as_code_status) AS "flow_status: Box<str>", r.ping AS last_ping, j.same_worker AS "same_worker?"
        FROM v2_job_queue q JOIN v2_job j USING (id) LEFT JOIN v2_job_runtime r USING (id) LEFT JOIN v2_job_status s USING (id)
        WHERE q.running = true AND q.suspend = 0 AND q.suspend_until IS null AND q.scheduled_for <= now()
            AND (j.kind = 'flow' OR j.kind = 'flowpreview' OR j.kind = 'flownode')
            AND r.ping IS NOT NULL AND r.ping < NOW() - ($1 || ' seconds')::interval
            AND q.canceled_by IS NULL

        "#,
        FLOW_ZOMBIE_TRANSITION_TIMEOUT.as_str()
    )
    .fetch_all(db)
    .await?;

    for flow in flows {
        let status = flow
            .flow_status
            .as_deref()
            .and_then(|x| serde_json::from_str::<FlowStatus>(x).ok());
        if !flow.same_worker.unwrap_or(false)
            && status.is_some_and(|s| {
                s.modules
                    .get(0)
                    .is_some_and(|x| matches!(x, FlowStatusModule::WaitingForPriorSteps { .. }))
            })
        {
            let error_message = format!(
                "Zombie flow detected: {} in workspace {}. It hasn't started yet, restarting it.",
                flow.id, flow.workspace_id
            );
            tracing::error!(error_message);
            report_critical_error(error_message, db.clone(), Some(&flow.workspace_id), None).await;
            // if the flow hasn't started and is a zombie, we can simply restart it
            let mut tx = db.begin().await?;

            let concurrency_key =
                sqlx::query_scalar!("SELECT key FROM concurrency_key WHERE job_id = $1", flow.id)
                    .fetch_optional(&mut *tx)
                    .await?;

            if let Some(key) = concurrency_key {
                if *DISABLE_CONCURRENCY_LIMIT {
                    tracing::warn!("Concurrency limit is disabled, skipping");
                } else {
                    sqlx::query!(
                        "UPDATE concurrency_counter SET job_uuids = job_uuids - $2 WHERE concurrency_id = $1",
                        key,
                        flow.id.hyphenated().to_string()
                    )
                    .execute(&mut *tx)
                    .await?;
                }
            }

            sqlx::query!(
                "UPDATE v2_job_queue SET running = false, started_at = null
                WHERE id = $1 AND canceled_by IS NULL",
                flow.id
            )
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;
        } else {
            let id = flow.id.clone();
            let last_ping = flow.last_ping.clone();
            let now = now_from_db(db).await?;
            let base_url = BASE_URL.read().await;
            let workspace_id = flow.workspace_id.clone();
            let reason = format!(
                "{} was hanging in between 2 steps. Last ping: {last_ping:?} (now: {now})",
                if flow.is_flow_step.unwrap_or(false) && flow.parent_job.is_some() {
                    format!("Flow was cancelled because subflow {id} ({base_url}/run/{id}?workspace={workspace_id})")
                } else {
                    format!("Flow {id} ({base_url}/run/{id}?workspace={workspace_id}) was cancelled because it")
                }
            );
            report_critical_error(reason.clone(), db.clone(), Some(&flow.workspace_id), None).await;
            cancel_zombie_flow_job(db, flow.id, &flow.workspace_id,
                format!(r#"{reason}
This would happen if a worker was interrupted, killed or crashed while doing a state transition at the end of a job which is always an unexpected behavior that should never happen.
Please check your worker logs for more details and feel free to report it to the Windmill team on our Discord or support@windmill.dev (response for non EE customers will be best effort) with as much context as possible, ideally:
- Windmill version
- Worker logs right after the job referenced has finished running
- Is the error consistent when running the same flow
- A minimal flow and its flow.yaml that reproduces the error and that is importable in a fresh workspace
- Your infra setup (helm, docker-compose, configuration of the workers and their number, memory of the database, etc.)
"#)).await?;
        }
    }

    let flows2 = sqlx::query!(
        r#"
        DELETE
        FROM parallel_monitor_lock
        WHERE last_ping IS NOT NULL AND last_ping < NOW() - ($1 || ' seconds')::interval
        RETURNING parent_flow_id, job_id, last_ping, (SELECT workspace_id FROM v2_job_queue q
            WHERE q.id = parent_flow_id AND q.running = true AND q.canceled_by IS NULL
        ) AS workspace_id
        "#,
        FLOW_ZOMBIE_TRANSITION_TIMEOUT.as_str()
    )
    .fetch_all(db)
    .await?;

    for flow in flows2 {
        if let Some(parent_flow_workspace_id) = flow.workspace_id {
            tracing::error!(
                "parallel Zombie flow detected: {} in workspace {}. Last ping was: {:?}.",
                flow.parent_flow_id,
                parent_flow_workspace_id,
                flow.last_ping
            );
            cancel_zombie_flow_job(db, flow.parent_flow_id, &parent_flow_workspace_id,
                format!("Flow {} cancelled as one of the parallel branch {} was unable to make the last transition ", flow.parent_flow_id, flow.job_id))
                .await?;
        } else {
            tracing::info!("releasing lock for parallel flow: {}", flow.parent_flow_id);
        }
    }
    Ok(())
}

async fn cancel_zombie_flow_job(
    db: &Pool<Postgres>,
    id: Uuid,
    workspace_id: &str,
    message: String,
) -> Result<(), error::Error> {
    let mut tx = db.begin().await?;
    tracing::error!(
        "zombie flow detected: {} in workspace {}. Cancelling it.",
        id,
        workspace_id
    );
    (tx, _) = cancel_job(
        "monitor",
        Some(message),
        id,
        workspace_id,
        tx,
        db,
        true,
        false,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn reload_hub_base_url_setting(
    conn: &Connection,
    server_mode: bool,
) -> error::Result<()> {
    let hub_base_url =
        load_value_from_global_settings_with_conn(conn, HUB_BASE_URL_SETTING, true).await?;

    let base_url = if let Some(q) = hub_base_url {
        if let Ok(v) = serde_json::from_value::<String>(q.clone()) {
            if v != "" {
                v
            } else {
                DEFAULT_HUB_BASE_URL.to_string()
            }
        } else {
            tracing::error!(
                "Could not parse hub_base_url setting as a string, found: {:#?}",
                &q
            );
            DEFAULT_HUB_BASE_URL.to_string()
        }
    } else {
        DEFAULT_HUB_BASE_URL.to_string()
    };

    let mut l = HUB_BASE_URL.write().await;
    if server_mode {
        #[cfg(feature = "embedding")]
        if let Some(db) = conn.as_sql() {
            if *l != base_url {
                let disable_embedding = std::env::var("DISABLE_EMBEDDING")
                    .ok()
                    .map(|x| x.parse::<bool>().unwrap_or(false))
                    .unwrap_or(false);
                if !disable_embedding {
                    let db_clone = db.clone();
                    tokio::spawn(async move {
                        update_embeddings_db(&db_clone).await;
                    });
                }
            }
        }
    }
    *l = base_url;

    Ok(())
}

pub async fn reload_critical_error_channels_setting(conn: &DB) -> error::Result<()> {
    let critical_error_channels =
        load_value_from_global_settings(conn, CRITICAL_ERROR_CHANNELS_SETTING).await?;

    let critical_error_channels = if let Some(q) = critical_error_channels {
        if let Ok(v) = serde_json::from_value::<Vec<CriticalErrorChannel>>(q.clone()) {
            v
        } else {
            tracing::error!(
                "Could not parse critical_error_channels setting as an array of channels, found: {:#?}",
                &q
            );
            vec![]
        }
    } else {
        vec![]
    };

    let mut l = CRITICAL_ERROR_CHANNELS.write().await;
    *l = critical_error_channels;

    Ok(())
}

pub async fn reload_app_workspaced_route_setting(conn: &DB) -> error::Result<()> {
    let app_workspaced_route =
        load_value_from_global_settings(conn, APP_WORKSPACED_ROUTE_SETTING).await?;

    let ws_route = match app_workspaced_route {
        Some(serde_json::Value::Bool(ws_route)) => ws_route,
        None => false,
        _ => {
            tracing::error!(
                "Expected {} to be a boolean got: {:?}. Defaulting to false",
                APP_WORKSPACED_ROUTE_SETTING,
                app_workspaced_route
            );
            false
        }
    };

    let mut l = APP_WORKSPACED_ROUTE.write().await;

    *l = ws_route;
    Ok(())
}

pub async fn reload_critical_alerts_on_db_oversize(conn: &DB) -> error::Result<()> {
    #[derive(Deserialize)]
    struct DBOversize {
        #[serde(default)]
        enabled: bool,
        #[serde(default)]
        value: f32,
    }
    let db_oversize_value =
        load_value_from_global_settings(conn, CRITICAL_ALERTS_ON_DB_OVERSIZE_SETTING).await?;

    let db_oversize = if let Some(q) = db_oversize_value {
        match serde_json::from_value::<DBOversize>(q.clone()) {
            Ok(DBOversize { enabled: true, value }) => Some(value),
            Ok(_) => None,
            Err(q) => {
                tracing::error!(
                    "Could not parse critical_alerts_on_db_oversize setting, found: {:#?}",
                    &q
                );
                None
            }
        }
    } else {
        None
    };

    let mut l = CRITICAL_ALERTS_ON_DB_OVERSIZE.write().await;
    *l = db_oversize;

    Ok(())
}

async fn generate_and_save_jwt_secret(db: &DB) -> error::Result<String> {
    let secret = rd_string(32);
    sqlx::query!(
        "INSERT INTO global_settings (name, value) VALUES ($1, $2) ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        JWT_SECRET_SETTING,
        serde_json::to_value(&secret).unwrap()
    ).execute(db).await?;

    Ok(secret)
}

pub async fn reload_jwt_secret_setting(db: &DB) -> error::Result<()> {
    let jwt_secret = load_value_from_global_settings(db, JWT_SECRET_SETTING).await?;

    let jwt_secret = if let Some(q) = jwt_secret {
        if let Ok(v) = serde_json::from_value::<String>(q.clone()) {
            v
        } else {
            tracing::error!("Could not parse jwt_secret setting, generating new one");
            generate_and_save_jwt_secret(db).await?
        }
    } else {
        tracing::info!("Not jwt secret found, generating one");
        generate_and_save_jwt_secret(db).await?
    };

    let mut l = JWT_SECRET.write().await;
    *l = jwt_secret;

    Ok(())
}

async fn cleanup_debounce_orphaned_keys(db: &DB) -> error::Result<()> {
    let result = sqlx::query!(
        "
DELETE FROM debounce_key
WHERE job_id NOT IN (SELECT id FROM v2_job_queue)
RETURNING key,job_id
        ",
    )
    .fetch_all(db)
    .await?;

    tracing::debug!("Cleaning up debounce keys");

    if result.len() > 0 {
        tracing::info!("Cleaned up {} debounce keys", result.len());
        for row in result {
            tracing::info!(
                "Debounce key cleaned up: key: {}, job_id: {:?}",
                row.key,
                row.job_id
            );
        }
    }
    Ok(())
}

async fn cleanup_debounce_keys_for_completed_jobs(db: &DB) -> error::Result<()> {
    // If min version doesn't support runnable settings, clean up debounce keys for completed jobs
    if !windmill_common::min_version::MIN_VERSION_SUPPORTS_RUNNABLE_SETTINGS_V0
        .met()
        .await
    {
        let result = sqlx::query!(
            "
DELETE FROM debounce_key
WHERE job_id IN (SELECT id FROM v2_job_completed)
RETURNING key,job_id
            ",
        )
        .fetch_all(db)
        .await?;

        if result.len() > 0 {
            tracing::warn!(
                "Cleaned up {} debounce keys for completed jobs (runnable settings v0 not supported by all workers)",
                result.len()
            );
            for row in result {
                tracing::debug!(
                    "Debounce key for completed job cleaned up: key: {}, job_id: {:?}",
                    row.key,
                    row.job_id
                );
            }
        }
    }
    Ok(())
}

async fn cleanup_job_perms_orphaned(db: &DB) -> error::Result<()> {
    let result = sqlx::query_scalar!(
        "DELETE FROM job_perms
WHERE job_id NOT IN (SELECT id FROM v2_job_queue)
RETURNING job_id"
    )
    .fetch_all(db)
    .await?;

    if !result.is_empty() {
        tracing::info!("Cleaned up {} orphaned job_perms rows", result.len());
    }
    Ok(())
}

async fn cleanup_job_result_stream_orphaned_jobs(db: &DB) -> error::Result<()> {
    let result = sqlx::query!(
        "DELETE FROM job_result_stream_v2
         WHERE job_id NOT IN (SELECT id FROM v2_job_queue)
           AND job_id NOT IN (
               SELECT id FROM v2_job_completed
               WHERE completed_at > NOW() - INTERVAL '60 seconds'
           )
         RETURNING job_id",
    )
    .fetch_all(db)
    .await?;

    if result.len() > 0 {
        tracing::info!(
            "Cleaned up {} orphaned job_result_stream_v2 rows",
            result.len()
        );
    }
    Ok(())
}

async fn cleanup_flow_iterator_data_orphaned_jobs(db: &DB) -> error::Result<()> {
    let result = sqlx::query!(
        "
DELETE FROM flow_iterator_data
WHERE job_id NOT IN (SELECT id FROM v2_job_queue)
RETURNING job_id
        ",
    )
    .fetch_all(db)
    .await?;

    if result.len() > 0 {
        tracing::info!(
            "Cleaned up {} orphaned flow_iterator_data rows",
            result.len()
        );
    }
    Ok(())
}
