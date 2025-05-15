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
    time::Duration,
};

use chrono::{NaiveDateTime, Utc};
use futures::{stream::FuturesUnordered, StreamExt};
use serde::{de::DeserializeOwned, Deserialize};
use sqlx::{Pool, Postgres};
use tokio::{
    join,
    sync::{mpsc, RwLock},
};
use uuid::Uuid;

#[cfg(feature = "embedding")]
use windmill_api::embeddings::update_embeddings_db;
use windmill_api::{
    jobs::TIMEOUT_WAIT_RESULT, DEFAULT_BODY_LIMIT, IS_SECURE, REQUEST_SIZE_LIMIT, SAML_METADATA,
    SCIM_TOKEN,
};

#[cfg(feature = "enterprise")]
use windmill_common::ee::low_disk_alerts;
#[cfg(feature = "enterprise")]
use windmill_common::ee::{jobs_waiting_alerts, worker_groups_alerts};

#[cfg(feature = "oauth2")]
use windmill_common::global_settings::OAUTH_SETTING;
use windmill_common::{
    agent_workers::DECODED_AGENT_TOKEN,
    auth::create_token_for_owner,
    ee::CriticalErrorChannel,
    error,
    flow_status::{FlowStatus, FlowStatusModule},
    global_settings::{
        BASE_URL_SETTING, BUNFIG_INSTALL_SCOPES_SETTING, CRITICAL_ALERTS_ON_DB_OVERSIZE_SETTING,
        CRITICAL_ALERT_MUTE_UI_SETTING, CRITICAL_ERROR_CHANNELS_SETTING,
        DEFAULT_TAGS_PER_WORKSPACE_SETTING, DEFAULT_TAGS_WORKSPACES_SETTING,
        EXPOSE_DEBUG_METRICS_SETTING, EXPOSE_METRICS_SETTING, EXTRA_PIP_INDEX_URL_SETTING,
        HUB_BASE_URL_SETTING, INSTANCE_PYTHON_VERSION_SETTING, JOB_DEFAULT_TIMEOUT_SECS_SETTING,
        JWT_SECRET_SETTING, KEEP_JOB_DIR_SETTING, LICENSE_KEY_SETTING,
        MONITOR_LOGS_ON_OBJECT_STORE_SETTING, NPM_CONFIG_REGISTRY_SETTING, NUGET_CONFIG_SETTING,
        OTEL_SETTING, PIP_INDEX_URL_SETTING, REQUEST_SIZE_LIMIT_SETTING,
        REQUIRE_PREEXISTING_USER_FOR_OAUTH_SETTING, RETENTION_PERIOD_SECS_SETTING,
        SAML_METADATA_SETTING, SCIM_TOKEN_SETTING, TIMEOUT_WAIT_RESULT_SETTING,
    },
    indexer::load_indexer_config,
    jobs::QueuedJob,
    jwt::JWT_SECRET,
    oauth2::REQUIRE_PREEXISTING_USER_FOR_OAUTH,
    server::load_smtp_config,
    tracing_init::JSON_FMT,
    users::truncate_token,
    utils::{empty_as_none, now_from_db, rd_string, report_critical_error, Mode},
    worker::{
        load_env_vars, load_init_bash_from_env, load_whitelist_env_vars_from_env,
        load_worker_config, reload_custom_tags_setting, store_pull_query,
        store_suspended_pull_query, update_min_version, Connection, WorkerConfig,
        DEFAULT_TAGS_PER_WORKSPACE, DEFAULT_TAGS_WORKSPACES, INDEXER_CONFIG, SCRIPT_TOKEN_EXPIRY,
        SMTP_CONFIG, TMP_DIR, WORKER_CONFIG, WORKER_GROUP,
    },
    KillpillSender, BASE_URL, CRITICAL_ALERTS_ON_DB_OVERSIZE, CRITICAL_ALERT_MUTE_UI_ENABLED,
    CRITICAL_ERROR_CHANNELS, DB, DEFAULT_HUB_BASE_URL, HUB_BASE_URL, JOB_RETENTION_SECS,
    METRICS_DEBUG_ENABLED, METRICS_ENABLED, MONITOR_LOGS_ON_OBJECT_STORE, OTEL_LOGS_ENABLED,
    OTEL_METRICS_ENABLED, OTEL_TRACING_ENABLED, SERVICE_LOG_RETENTION_SECS,
};
use windmill_queue::{cancel_job, MiniPulledJob, SameWorkerPayload};
use windmill_worker::{
    handle_job_error, AuthedClient, JobCompletedSender, SameWorkerSender, BUNFIG_INSTALL_SCOPES,
    INSTANCE_PYTHON_VERSION, JOB_DEFAULT_TIMEOUT, KEEP_JOB_DIR, MAVEN_REPOS, NO_DEFAULT_MAVEN,
    NPM_CONFIG_REGISTRY, NUGET_CONFIG, PIP_EXTRA_INDEX_URL, PIP_INDEX_URL,
};

#[cfg(feature = "parquet")]
use windmill_common::s3_helpers::{
    build_object_store_from_settings, build_s3_client_from_settings, S3Settings,
    OBJECT_STORE_CACHE_SETTINGS,
};

#[cfg(feature = "parquet")]
use windmill_common::global_settings::OBJECT_STORE_CACHE_CONFIG_SETTING;

#[cfg(feature = "enterprise")]
use crate::ee::verify_license_key;

use crate::ee::set_license_key;

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
    static ref DISABLE_CONCURRENCY_LIMIT: bool = std::env::var("DISABLE_CONCURRENCY_LIMIT").is_ok_and(|s| s == "true");

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
                *config = WorkerConfig {
                    worker_tags: DECODED_AGENT_TOKEN
                        .as_ref()
                        .map(|x| x.tags.clone())
                        .unwrap_or_default(),
                    env_vars: load_env_vars(
                        load_whitelist_env_vars_from_env(),
                        &std::collections::HashMap::new(),
                    ),
                    priority_tags_sorted: vec![],
                    dedicated_worker: None,
                    init_bash: load_init_bash_from_env(),
                    cache_clear: None,
                    additional_python_paths: None,
                    pip_local_dependencies: None,
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
    }

    #[cfg(feature = "parquet")]
    if !disable_s3_store {
        if let Some(db) = conn.as_sql() {
            reload_s3_cache_setting(db).await;
        }
    }

    if let Some(db) = conn.as_sql() {
        reload_smtp_config(db).await;
    }

    if server_mode {
        reload_retention_period_setting(&conn).await;
        reload_request_size(&conn).await;
        reload_saml_metadata_setting(&conn).await;
        reload_scim_token_setting(&conn).await;
    }

    if worker_mode {
        reload_job_default_timeout_setting(&conn).await;
        reload_extra_pip_index_url_setting(&conn).await;
        reload_pip_index_url_setting(&conn).await;
        reload_npm_config_registry_setting(&conn).await;
        reload_bunfig_install_scopes_setting(&conn).await;
        reload_instance_python_version_setting(&conn).await;
        reload_nuget_config_setting(&conn).await;
        reload_maven_repos_setting(&conn).await;
        reload_no_default_maven_setting(&conn).await;
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
                    std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", endpoint.clone());
                    Some(endpoint.clone())
                } else {
                    std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()
                };

                let headers = if let Some(headers) = o.otel_exporter_otlp_headers {
                    std::env::set_var("OTEL_EXPORTER_OTLP_HEADERS", headers.clone());
                    Some(headers.clone())
                } else {
                    std::env::var("OTEL_EXPORTER_OTLP_HEADERS").ok()
                };

                if let Some(protocol) = o.otel_exporter_otlp_protocol {
                    std::env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", protocol);
                }
                if let Some(compression) = o.otel_exporter_otlp_compression {
                    std::env::set_var("OTEL_EXPORTER_OTLP_COMPRESSION", compression);
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
        let s3_client = OBJECT_STORE_CACHE_SETTINGS.read().await.clone();
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
            let path = object_store::path::Path::from_url_path(format!(
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
            if let Err(e) = sqlx::query!("INSERT INTO log_file (hostname, mode, worker_group, log_ts, file_path, ok_lines, err_lines, json_fmt)
             VALUES ($1, $2::text::LOG_MODE, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (hostname, log_ts) DO UPDATE SET ok_lines = log_file.ok_lines + $6, err_lines = log_file.err_lines + $7",
                hostname, mode.to_string(), worker_group.clone(), ts, highest_file, ok_lines as i64, err_lines as i64, *JSON_FMT)
                .execute(db)
                .await {
                tracing::error!("Error inserting log file: {:?}", e);
            } else {
                if let Err(e) = LAST_LOG_FILE_SENT.lock().map(|mut last_log_file_sent| {
                    last_log_file_sent.replace(ts);
                }) {
                    tracing::error!("Error updating last log file sent: {:?}", e);
                }
                tracing::info!("Log file sent: {}", highest_file);
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

    let job_retention_secs = *JOB_RETENTION_SECS.read().await;
    if job_retention_secs > 0 {
        match db.begin().await {
            Ok(mut tx) => {
                let deleted_jobs = sqlx::query_scalar!(
                    "DELETE FROM v2_job_completed c
                    WHERE completed_at <= now() - ($1::bigint::text || ' s')::interval 
                    RETURNING c.id",
                    job_retention_secs
                )
                .fetch_all(&mut *tx)
                .await;

                match deleted_jobs {
                    Ok(deleted_jobs) => {
                        if deleted_jobs.len() > 0 {
                            tracing::info!(
                                "deleted {} jobs completed JOB_RETENTION_SECS {} ago: {:?}",
                                deleted_jobs.len(),
                                job_retention_secs,
                                deleted_jobs,
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
                                Err(e) => tracing::error!("Error deleting job stats: {:?}", e),
                            }
                            if let Err(e) = sqlx::query!(
                                "DELETE FROM concurrency_key WHERE  ended_at <= now() - ($1::bigint::text || ' s')::interval ",
                                job_retention_secs
                            )
                            .execute(&mut *tx)
                            .await
                            {
                                tracing::error!("Error deleting  custom concurrency key: {:?}", e);
                            }

                            if let Err(e) =
                                sqlx::query!("DELETE FROM v2_job WHERE id = ANY($1)", &deleted_jobs)
                                    .execute(&mut *tx)
                                    .await
                            {
                                tracing::error!("Error deleting job: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error deleting expired jobs: {:?}", e)
                    }
                }

                match tx.commit().await {
                    Ok(_) => (),
                    Err(err) => tracing::error!("Error deleting expired jobs: {:?}", err),
                }
            }
            Err(err) => {
                tracing::error!("Error deleting expired jobs: {:?}", err)
            }
        }
    }
}

async fn delete_log_files_from_disk_and_store(
    paths_to_delete: Vec<String>,
    tmp_dir: &str,
    _s3_prefix: &str,
) {
    #[cfg(feature = "parquet")]
    let os = windmill_common::s3_helpers::OBJECT_STORE_CACHE_SETTINGS
        .read()
        .await
        .clone();
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
                    let p = object_store::path::Path::from(format!("{}{}", _s3_prefix, path));
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
pub async fn reload_maven_repos_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        windmill_common::global_settings::MAVEN_REPOS_SETTING,
        "MAVEN_REPOS",
        MAVEN_REPOS.clone(),
    )
    .await;
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

#[cfg(feature = "parquet")]
pub async fn reload_s3_cache_setting(db: &DB) {
    use windmill_common::{
        ee::{get_license_plan, LicensePlan},
        s3_helpers::ObjectSettings,
    };

    let s3_config = load_value_from_global_settings(db, OBJECT_STORE_CACHE_CONFIG_SETTING).await;
    if let Err(e) = s3_config {
        tracing::error!("Error reloading s3 cache config: {:?}", e)
    } else {
        if let Some(v) = s3_config.unwrap() {
            if matches!(get_license_plan().await, LicensePlan::Pro) {
                tracing::error!("S3 cache is not available for pro plan");
                return;
            }
            let mut s3_cache_settings = OBJECT_STORE_CACHE_SETTINGS.write().await;
            let setting = serde_json::from_value::<ObjectSettings>(v);
            if let Err(e) = setting {
                tracing::error!("Error parsing s3 cache config: {:?}", e)
            } else {
                let s3_client = build_object_store_from_settings(setting.unwrap()).await;
                if let Err(e) = s3_client {
                    tracing::error!("Error building s3 client from settings: {:?}", e)
                } else {
                    *s3_cache_settings = Some(s3_client.unwrap());
                }
            }
        } else {
            let mut s3_cache_settings = OBJECT_STORE_CACHE_SETTINGS.write().await;
            if std::env::var("S3_CACHE_BUCKET").is_ok() {
                if matches!(get_license_plan().await, LicensePlan::Pro) {
                    tracing::error!("S3 cache is not available for pro plan");
                    return;
                }
                *s3_cache_settings = build_s3_client_from_settings(S3Settings {
                    bucket: None,
                    region: None,
                    access_key: None,
                    secret_key: None,
                    endpoint: None,
                    store_logs: None,
                    path_style: None,
                    allow_http: None,
                    port: None,
                })
                .await
                .ok();
            } else {
                *s3_cache_settings = None;
            }
        }
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
    set_license_key(value).await;
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

pub async fn monitor_db(
    conn: &Connection,
    base_internal_url: &str,
    server_mode: bool,
    _worker_mode: bool,
    initial_load: bool,
    _killpill_tx: KillpillSender,
) {
    tracing::info!("Starting periodic monitor task");
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
        update_min_version(conn).await;
    };

    join!(
        expired_items_f,
        zombie_jobs_f,
        expose_queue_metrics_f,
        verify_license_key_f,
        worker_groups_alerts_f,
        jobs_waiting_alerts_f,
        low_disk_alerts_f,
        apply_autoscaling_f,
        update_min_worker_version_f,
    );
    tracing::info!("Periodic monitor task completed");
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
        if *wc != config || config.dedicated_worker.is_some() {
            if kill_if_change {
                if config.dedicated_worker.is_some()
                    || (*wc).dedicated_worker != config.dedicated_worker
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
                Option<HashMap<String, windmill_api::oauth2_ee::OAuthClient>>,
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
            *l = windmill_api::oauth2_ee::build_oauth_clients(&base_url, oauths, db).await
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

const RESTART_LIMIT: i32 = 3;

async fn handle_zombie_jobs(db: &Pool<Postgres>, base_internal_url: &str, worker_name: &str) {
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
                    AND kind NOT IN ('flow', 'flowpreview', 'flownode', 'singlescriptflow')
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
    AND running = true AND ping IS NULL AND same_worker = true AND worker IS NOT NULL GROUP BY worker",
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

        let jobs = sqlx::query_as::<_, QueuedJob>("SELECT * FROM v2_as_queue WHERE id = ANY($1)")
            .bind(&timeouts[..])
            .fetch_all(db)
            .await
            .map_err(|e| tracing::error!("Error fetching same worker jobs: {:?}", e))
            .unwrap_or_default();

        jobs
    };

    let non_restartable_jobs = if *RESTART_ZOMBIE_JOBS {
        vec![]
    } else {
        sqlx::query_as::<_, QueuedJob>("SELECT * FROM v2_as_queue WHERE last_ping < now() - ($1 || ' seconds')::interval
    AND running = true  AND job_kind NOT IN ('flow', 'flowpreview', 'flownode', 'singlescriptflow') AND same_worker = false")
        .bind(ZOMBIE_JOB_TIMEOUT.as_str())
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

    let zombie_jobs_restart_limit_reached =
        sqlx::query_as::<_, QueuedJob>("SELECT * FROM v2_as_queue WHERE id = ANY($1)")
            .bind(&zombie_jobs_uuid_restart_limit_reached[..])
            .fetch_all(db)
            .await
            .ok()
            .unwrap_or_else(|| vec![]);

    let timeouts = non_restartable_jobs
        .into_iter()
        .map(|x| (x, ErrorMessage::RestartDisabled))
        .chain(
            same_worker_timeout_jobs
                .into_iter()
                .map(|x| (x, ErrorMessage::SameWorker)),
        )
        .chain(
            zombie_jobs_restart_limit_reached
                .into_iter()
                .map(|x| (x, ErrorMessage::RestartLimit)),
        )
        .collect::<Vec<_>>();

    #[cfg(feature = "prometheus")]
    if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        QUEUE_ZOMBIE_DELETE_COUNT.inc_by(timeouts.len() as _);
    }

    for (job, error_kind) in timeouts {
        // since the job is unrecoverable, the same worker queue should never be sent anything
        let (same_worker_tx_never_used, _same_worker_rx_never_used) =
            mpsc::channel::<SameWorkerPayload>(1);
        let same_worker_tx_never_used =
            SameWorkerSender(same_worker_tx_never_used, Arc::new(AtomicU16::new(0)));
        let (send_result_never_used, _send_result_rx_never_used) =
            JobCompletedSender::new_never_used();

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
            &job.email,
            &job.id,
            None,
        )
        .await
        .expect("could not create job token");

        let client = AuthedClient {
            base_internal_url: base_internal_url.to_string(),
            token,
            workspace: job.workspace_id.to_string(),
            force_client: None,
        };

        let last_ping = job.last_ping.clone();
        let error_message = format!(
            "Job timed out after no ping from job since {} (ZOMBIE_JOB_TIMEOUT: {}, reason: {:?})",
            last_ping
                .map(|x| x.to_string())
                .unwrap_or_else(|| "no ping".to_string()),
            *ZOMBIE_JOB_TIMEOUT,
            error_kind.to_string()
        );
        let _ = handle_job_error(
            db,
            &client,
            &MiniPulledJob::from(&job),
            0,
            None,
            error::Error::ExecutionErr(error_message),
            true,
            same_worker_tx_never_used,
            "",
            worker_name,
            send_result_never_used,
            #[cfg(feature = "benchmark")]
            &mut windmill_common::bench::BenchmarkIter::new(),
        )
        .await;
    }
}

async fn handle_zombie_flows(db: &DB) -> error::Result<()> {
    let flows = sqlx::query!(
        r#"
        SELECT
            id AS "id!", workspace_id AS "workspace_id!", parent_job, is_flow_step,
            flow_status AS "flow_status: Box<str>", last_ping, same_worker
        FROM v2_as_queue
        WHERE running = true AND suspend = 0 AND suspend_until IS null AND scheduled_for <= now()
            AND (job_kind = 'flow' OR job_kind = 'flowpreview' OR job_kind = 'flownode')
            AND last_ping IS NOT NULL AND last_ping < NOW() - ($1 || ' seconds')::interval
            AND canceled = false
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
        "INSERT INTO global_settings (name, value) VALUES ($1, $2) ON CONFLICT (name) DO UPDATE SET value = $2",
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
