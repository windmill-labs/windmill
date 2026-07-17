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
use windmill_common::otel_oss::{
    otel_incr_zombie_delete_count, otel_incr_zombie_restart_count, otel_set_db_pool,
    otel_set_queue_count, otel_set_queue_running_count,
};
use windmill_common::{
    agent_workers::DECODED_AGENT_TOKEN,
    apps::APP_WORKSPACED_ROUTE,
    auth::create_token_for_owner,
    ee_oss::CriticalErrorChannel,
    email_oss::send_email_if_possible,
    error,
    flow_status::{FlowStatus, FlowStatusModule},
    global_settings::{
        AUDIT_LOG_RETENTION_DAYS_SETTING, BASE_URL_SETTING, BUNFIG_INSTALL_SCOPES_SETTING,
        BUN_INSTALL_MIN_RELEASE_AGE_SETTING, CRITICAL_ALERTS_ON_DB_OVERSIZE_SETTING,
        CRITICAL_ALERTS_ON_TOKEN_EXPIRY_SETTING, CRITICAL_ALERT_MUTE_UI_SETTING,
        CRITICAL_ERROR_CHANNELS_SETTING, DEFAULT_TAGS_PER_WORKSPACE_SETTING,
        DEFAULT_TAGS_WORKSPACES_SETTING, DISABLE_PASSWORD_LOGIN, DISABLE_PASSWORD_LOGIN_SETTING,
        EXPOSE_DEBUG_METRICS_SETTING, EXPOSE_METRICS_SETTING, EXTRA_PIP_INDEX_URL_SETTING,
        FORK_WORKSPACE_TAG_APPEND_FORK_SUFFIX_SETTING, HUB_API_SECRET_SETTING,
        HUB_BASE_URL_SETTING, INSTANCE_PYTHON_VERSION_SETTING, JOB_DEFAULT_TIMEOUT_SECS_SETTING,
        JOB_ISOLATION_SETTING, JWT_SECRET_SETTING, KEEP_JOB_DIR_SETTING, LICENSE_KEY_SETTING,
        MONITOR_LOGS_ON_OBJECT_STORE_SETTING, NPMRC_SETTING, NPM_CONFIG_REGISTRY_SETTING,
        NSJAIL_TMPFS_SIZE_MB_SETTING, NSJAIL_TMP_BACKING_SETTING, NUGET_CONFIG_SETTING,
        OTEL_SETTING, OTEL_TRACING_PROXY_SETTING, PIP_INDEX_URL_SETTING,
        POWERSHELL_REPO_PAT_SETTING, POWERSHELL_REPO_URL_SETTING, PREVIEW_TAGS_OVERRIDE_SETTING,
        REQUEST_SIZE_LIMIT_SETTING, REQUIRE_PREEXISTING_USER_FOR_OAUTH_SETTING,
        RETENTION_PERIOD_SECS_SETTING, SAML_METADATA_SETTING, SANDBOX_IMAGE_CACHE_MAX_MB_SETTING,
        SANDBOX_IMAGE_DEFAULT_REGISTRY_SETTING, SANDBOX_IMAGE_MAX_SIZE_MB_SETTING,
        SANDBOX_IMAGE_PULL_POLICY_SETTING, SANDBOX_REGISTRY_AUTH_SETTING, SCIM_TOKEN_SETTING,
        STORE_AUDIT_LOGS_S3_SETTING, TIMEOUT_WAIT_RESULT_SETTING, UV_EXCLUDE_NEWER_SETTING,
        UV_INDEX_STRATEGY_SETTING, UV_PYTHON_INSTALL_MIRROR_SETTING,
        WORKSPACE_FAIRNESS_DURATION_SECS_SETTING, WORKSPACE_FAIRNESS_ENABLED_SETTING,
        WORKSPACE_FAIRNESS_MAX_PERCENT_SETTING, WORKSPACE_FAIRNESS_MIN_TOTAL_SETTING,
    },
    indexer::load_indexer_config,
    jobs::delete_jobs,
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
        DEFAULT_TAGS_WORKSPACES, FORK_WORKSPACE_TAG_APPEND_FORK_SUFFIX, INDEXER_CONFIG,
        PREVIEW_TAGS_OVERRIDE, SCRIPT_TOKEN_EXPIRY, SMTP_CONFIG, WINDMILL_DIR, WORKER_CONFIG,
        WORKER_GROUP, WORKSPACE_FAIRNESS_DURATION_SECS, WORKSPACE_FAIRNESS_ENABLED,
        WORKSPACE_FAIRNESS_MAX_PERCENT, WORKSPACE_FAIRNESS_MIN_TOTAL,
    },
    KillpillSender, AUDIT_LOG_RETENTION_DAYS, BASE_URL, CRITICAL_ALERTS_ON_DB_OVERSIZE,
    CRITICAL_ALERTS_ON_TOKEN_EXPIRY, CRITICAL_ALERT_MUTE_UI_ENABLED, CRITICAL_ERROR_CHANNELS, DB,
    DEFAULT_HUB_BASE_URL, HUB_BASE_URL, JOB_RETENTION_SECS, JOB_RETENTION_SECS_OVERRIDES,
    JOB_RETENTION_SECS_OVERRIDES_LOADED, METRICS_DEBUG_ENABLED, METRICS_ENABLED,
    MONITOR_LOGS_ON_OBJECT_STORE, OTEL_LOGS_ENABLED, OTEL_METRICS_ENABLED, OTEL_TRACING_ENABLED,
    SERVICE_LOG_RETENTION_SECS, STORE_AUDIT_LOGS_S3,
};
use windmill_common::{
    client::AuthedClient,
    global_settings::{
        APP_WORKSPACED_ROUTE_SETTING, HTTP_ROUTE_WORKSPACED_ROUTE,
        HTTP_ROUTE_WORKSPACED_ROUTE_SETTING,
    },
};
#[cfg(feature = "parquet")]
use windmill_object_store::reload_object_store_setting;
use windmill_queue::{cancel_job, get_queued_job_v2, SameWorkerPayload};
use windmill_worker::{
    result_processor::handle_job_error, JobCompletedSender, JobIsolationLevel,
    OtelTracingProxySettings, SameWorkerSender, WorkspaceRegistryMap, BUNFIG_INSTALL_SCOPES,
    BUN_INSTALL_MIN_RELEASE_AGE, CARGO_REGISTRIES, INSTANCE_PYTHON_VERSION, JAVA_HOME_DIR,
    JOB_DEFAULT_TIMEOUT, JOB_ISOLATION, KEEP_JOB_DIR, MAVEN_REPOS, MAVEN_SETTINGS_XML,
    NO_DEFAULT_MAVEN, NPMRC, NPM_CONFIG_REGISTRY, NSJAIL_AVAILABLE, NSJAIL_TMPFS_SIZE_MB,
    NSJAIL_TMP_BACKING, NUGET_CONFIG, OTEL_TRACING_PROXY_SETTINGS, PIP_EXTRA_INDEX_URL,
    PIP_INDEX_URL, POWERSHELL_REPO_PAT, POWERSHELL_REPO_URL, SANDBOX_IMAGE_CACHE_MAX_MB,
    SANDBOX_IMAGE_DEFAULT_REGISTRY, SANDBOX_IMAGE_MAX_SIZE_MB, SANDBOX_IMAGE_PULL_POLICY,
    SANDBOX_REGISTRY_AUTH, UNSHARE_PATH, UV_EXCLUDE_NEWER, UV_INDEX_STRATEGY,
    UV_PYTHON_INSTALL_MIRROR, WORKSPACE_REGISTRIES,
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

    // Ops kill switch for the pipeline freshness watchdog (a background
    // pusher — being able to stop it without a redeploy matters more than
    // for read-only monitors).
    pub static ref DISABLE_FRESHNESS_WATCHDOG: bool = std::env::var("DISABLE_FRESHNESS_WATCHDOG")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(false);

    pub static ref WORKERS_NAMES: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));

    static ref QUEUE_COUNT_TAGS: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    static ref QUEUE_RUNNING_COUNT_TAGS: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    static ref OTEL_QUEUE_COUNT_TAGS: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    static ref OTEL_QUEUE_RUNNING_COUNT_TAGS: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
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

    if let Err(e) = reload_critical_alerts_on_token_expiry_setting(conn).await {
        tracing::error!("Error loading critical alerts on token expiry setting: {e:#}");
    }

    if let Some(db) = conn.as_sql() {
        if let Err(e) = load_tag_per_workspace_enabled(db).await {
            tracing::error!("Error loading default tag per workpsace: {e:#}");
        }

        if let Err(e) = load_tag_per_workspace_workspaces(db).await {
            tracing::error!("Error loading default tag per workpsace workspaces: {e:#}");
        }

        if let Err(e) = load_fork_workspace_tag_append_fork_suffix(db).await {
            tracing::error!("Error loading fork workspace tag append fork suffix: {e:#}");
        }

        if let Err(e) = load_preview_tags_override(db).await {
            tracing::error!("Error loading preview tags override: {e:#}");
        }

        // Load per-workspace retention overrides before the first cleanup tick so a fresh server
        // never sweeps globally without honoring configured longer-retention workspaces.
        if let Err(e) = load_retention_period_overrides(db).await {
            tracing::error!("Error loading per-workspace retention overrides: {e:#}");
        }

        // Workspace fairness (cloud-only). Load the percentage/duration/min knobs
        // *before* the enabled flag so that `load_workspace_fairness_enabled` reads
        // current values when re-storing the pull queries.
        if let Err(e) = load_workspace_fairness_max_percent(db).await {
            tracing::error!("Error loading workspace fairness max percent: {e:#}");
        }
        if let Err(e) = load_workspace_fairness_duration_secs(db).await {
            tracing::error!("Error loading workspace fairness duration secs: {e:#}");
        }
        if let Err(e) = load_workspace_fairness_min_total(db).await {
            tracing::error!("Error loading workspace fairness min total: {e:#}");
        }
        if let Err(e) = load_workspace_fairness_enabled(db).await {
            tracing::error!("Error loading workspace fairness enabled: {e:#}");
        }
    }

    if server_mode {
        if let Some(db) = conn.as_sql() {
            load_require_preexisting_user(db).await;
            load_disable_password_login(db).await;
            if let Err(e) = reload_critical_alerts_on_db_oversize(db).await {
                tracing::error!(
                    "Error reloading critical alerts on db oversize setting: {:?}",
                    e
                )
            }
            windmill_common::min_version::store_min_keep_alive_version(db).await;
            reload_instance_events_webhook_setting(db).await;
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
                let worker_tags = DECODED_AGENT_TOKEN
                    .as_ref()
                    .map(|x| x.tags.clone())
                    .unwrap_or_default();
                // we only check from env as native_mode is not stored in the token
                // NATIVE_MODE_RESOLVED is already set in main.rs during startup
                let native_mode = windmill_common::worker::is_native_mode_from_env();
                WORKER_CONFIG.store(std::sync::Arc::new(WorkerConfig {
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
                }));
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

        if let Err(e) = reload_http_route_workspaced_route_setting(db).await {
            tracing::error!("Error reloading http route workspaced route: {:?}", e)
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
        reload_audit_log_retention_days_setting(&conn).await;
        reload_store_audit_logs_s3_setting(&conn).await;
        // Env-var enable has no settings-row xmin and no runtime enable event;
        // anchor the export cursor at startup so rows committed before the
        // first export tick are not skipped (no-op when a settings row exists
        // or a checkpoint is already present). Audit-log S3 export is an
        // Enterprise feature; the core logic lives in `crate::ee` (OSS gets a
        // no-op), gated here on a valid Enterprise license.
        #[cfg(feature = "parquet")]
        if STORE_AUDIT_LOGS_S3.load(std::sync::atomic::Ordering::Relaxed)
            && matches!(
                windmill_common::ee_oss::get_license_plan().await,
                windmill_common::ee_oss::LicensePlan::Enterprise
            )
        {
            if let Some(db) = conn.as_sql() {
                crate::ee_oss::anchor_audit_logs_s3_checkpoint_env_var(&db).await;
            }
        }
        reload_request_size(&conn).await;
        reload_saml_metadata_setting(&conn).await;
        reload_scim_token_setting(&conn).await;

        // Ensure audit partitions exist before any requests arrive
        if let Some(db) = conn.as_sql() {
            manage_audit_partitions(&db, audit_log_retention_days().await).await;
        }
    }

    if worker_mode {
        reload_job_default_timeout_setting(&conn).await;
        reload_job_isolation_setting(&conn).await;
        reload_nsjail_tmpfs_size_setting(&conn).await;
        reload_nsjail_tmp_backing_setting(&conn).await;
        reload_sandbox_image_max_size_setting(&conn).await;
        reload_sandbox_image_cache_max_setting(&conn).await;
        reload_sandbox_image_pull_policy_setting(&conn).await;
        reload_sandbox_image_default_registry_setting(&conn).await;
        reload_sandbox_registry_auth_setting(&conn).await;
        reload_extra_pip_index_url_setting(&conn).await;
        reload_pip_index_url_setting(&conn).await;
        reload_uv_index_strategy_setting(&conn).await;
        reload_uv_exclude_newer_setting(&conn).await;
        reload_uv_python_install_mirror_setting(&conn).await;
        reload_bun_install_min_release_age_setting(&conn).await;
        reload_npm_config_registry_setting(&conn).await;
        reload_bunfig_install_scopes_setting(&conn).await;
        reload_npmrc_setting(&conn).await;
        reload_instance_python_version_setting(&conn).await;
        reload_nuget_config_setting(&conn).await;
        reload_powershell_repo_url_setting(&conn).await;
        reload_powershell_repo_pat_setting(&conn).await;
        reload_maven_repos_setting(&conn).await;
        reload_maven_settings_xml_setting(&conn).await;
        reload_no_default_maven_setting(&conn).await;
        reload_ruby_repos_setting(&conn).await;
        reload_cargo_registries_setting(&conn).await;
        reload_workspace_registries_setting(&conn).await;
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
            DEFAULT_TAGS_WORKSPACES.store(std::sync::Arc::new(Some(workspaces)));
        }
        Ok(None) => {
            DEFAULT_TAGS_WORKSPACES.store(std::sync::Arc::new(None));
        }
        _ => (),
    };
    Ok(())
}

pub async fn load_preview_tags_override(db: &DB) -> error::Result<()> {
    let value = load_value_from_global_settings(db, PREVIEW_TAGS_OVERRIDE_SETTING).await;

    match value {
        Ok(Some(serde_json::Value::Bool(t))) => PREVIEW_TAGS_OVERRIDE.store(t, Ordering::Relaxed),
        _ => (),
    };
    Ok(())
}

// Upper bound on the duration window. Postgres `make_interval(secs => $1::int4)` is the consumer
// downstream, so this stays comfortably below `i32::MAX` and the subsequent `u32 -> i32` cast in
// `workspace_fairness::refresh_overloaded` cannot wrap into a negative interval (which would
// silently turn `now() - interval` into a future timestamp and disable the completed-jobs half
// of the activity signal). A day is the practical ceiling for a "rolling window" knob.
const WORKSPACE_FAIRNESS_DURATION_SECS_MAX: u64 = 86_400;

/// Min-total floor is a counting threshold; cap at `u32::MAX` to make wraparound impossible
/// while still leaving more headroom than any realistic cluster will need.
const WORKSPACE_FAIRNESS_MIN_TOTAL_MAX: u64 = u32::MAX as u64;

// Defaults used when a fairness knob is unset (row missing or row deleted via NULL/empty value).
// Must stay in sync with the `AtomicU32::new(...)` initialisers in `windmill-common/src/worker.rs`
// so a process that has never seen the setting reads the same value as one that just saw it
// cleared.
const WORKSPACE_FAIRNESS_MAX_PERCENT_DEFAULT: u32 = 50;
const WORKSPACE_FAIRNESS_DURATION_SECS_DEFAULT: u32 = 10;
const WORKSPACE_FAIRNESS_MIN_TOTAL_DEFAULT: u32 = 4;

pub async fn load_workspace_fairness_enabled(db: &DB) -> error::Result<()> {
    // Match the convention used by `load_preview_tags_override` /
    // `load_fork_workspace_tag_append_fork_suffix`: on transient DB errors, leave the in-memory
    // atomic untouched rather than silently toggling the feature off across the whole cluster
    // (which would also trigger an unnecessary `store_pull_query` rebuild — exactly when DB load
    // is probably highest).
    let new_enabled =
        match load_value_from_global_settings(db, WORKSPACE_FAIRNESS_ENABLED_SETTING).await? {
            Some(serde_json::Value::Bool(t)) => t,
            // Setting unset / non-bool → explicit off.
            _ => false,
        };
    let prev = WORKSPACE_FAIRNESS_ENABLED.swap(new_enabled, Ordering::Relaxed);
    // Re-store the pull queries so the fairness variants appear/disappear in
    // lockstep with the toggle.
    if prev != new_enabled {
        let wc = windmill_common::worker::WORKER_CONFIG.load_full();
        store_pull_query(&wc).await;
    }
    Ok(())
}

pub async fn load_workspace_fairness_max_percent(db: &DB) -> error::Result<()> {
    // Distinguish three outcomes:
    //   - `Err(_)`: transient DB issue. Leave the atomic alone (don't clobber a known-good value
    //     because of a network blip during a notify-event propagation).
    //   - `Ok(None)` or `Ok(Some(invalid))`: setting is unset / explicitly cleared / corrupt.
    //     Restore the default so a deletion via the admin UI actually takes effect at runtime
    //     instead of leaving the stale in-memory value pinned until restart.
    //   - `Ok(Some(valid))`: clamp and store.
    match load_value_from_global_settings(db, WORKSPACE_FAIRNESS_MAX_PERCENT_SETTING).await? {
        Some(serde_json::Value::Number(n)) => {
            let v = n
                .as_u64()
                .map(|u| u.clamp(1, 100) as u32)
                .unwrap_or(WORKSPACE_FAIRNESS_MAX_PERCENT_DEFAULT);
            WORKSPACE_FAIRNESS_MAX_PERCENT.store(v, Ordering::Relaxed);
        }
        _ => {
            WORKSPACE_FAIRNESS_MAX_PERCENT
                .store(WORKSPACE_FAIRNESS_MAX_PERCENT_DEFAULT, Ordering::Relaxed);
        }
    }
    Ok(())
}

pub async fn load_workspace_fairness_duration_secs(db: &DB) -> error::Result<()> {
    // See `load_workspace_fairness_max_percent` for the Err / None / invalid policy.
    match load_value_from_global_settings(db, WORKSPACE_FAIRNESS_DURATION_SECS_SETTING).await? {
        Some(serde_json::Value::Number(n)) => {
            // Clamp to the safe range before narrowing. The downstream `u32 -> i32` cast in
            // `workspace_fairness::refresh_overloaded` makes any value above `i32::MAX` toxic
            // (sign flip → negative interval → silent disable of the completed-jobs scan).
            let v = n
                .as_u64()
                .map(|u| u.clamp(1, WORKSPACE_FAIRNESS_DURATION_SECS_MAX) as u32)
                .unwrap_or(WORKSPACE_FAIRNESS_DURATION_SECS_DEFAULT);
            WORKSPACE_FAIRNESS_DURATION_SECS.store(v, Ordering::Relaxed);
        }
        _ => {
            WORKSPACE_FAIRNESS_DURATION_SECS
                .store(WORKSPACE_FAIRNESS_DURATION_SECS_DEFAULT, Ordering::Relaxed);
        }
    }
    Ok(())
}

pub async fn load_workspace_fairness_min_total(db: &DB) -> error::Result<()> {
    // See `load_workspace_fairness_max_percent` for the Err / None / invalid policy.
    match load_value_from_global_settings(db, WORKSPACE_FAIRNESS_MIN_TOTAL_SETTING).await? {
        Some(serde_json::Value::Number(n)) => {
            // Clamp before narrowing — same reasoning as `_duration_secs`, just for the
            // counting threshold rather than the interval.
            let v = n
                .as_u64()
                .map(|u| u.min(WORKSPACE_FAIRNESS_MIN_TOTAL_MAX) as u32)
                .unwrap_or(WORKSPACE_FAIRNESS_MIN_TOTAL_DEFAULT);
            WORKSPACE_FAIRNESS_MIN_TOTAL.store(v, Ordering::Relaxed);
        }
        _ => {
            WORKSPACE_FAIRNESS_MIN_TOTAL
                .store(WORKSPACE_FAIRNESS_MIN_TOTAL_DEFAULT, Ordering::Relaxed);
        }
    }
    Ok(())
}

pub async fn load_fork_workspace_tag_append_fork_suffix(db: &DB) -> error::Result<()> {
    let value =
        load_value_from_global_settings(db, FORK_WORKSPACE_TAG_APPEND_FORK_SUFFIX_SETTING).await;

    match value {
        Ok(Some(serde_json::Value::Bool(t))) => {
            FORK_WORKSPACE_TAG_APPEND_FORK_SUFFIX.store(t, Ordering::Relaxed)
        }
        Ok(None) => FORK_WORKSPACE_TAG_APPEND_FORK_SUFFIX.store(false, Ordering::Relaxed),
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

pub async fn reload_critical_alerts_on_token_expiry_setting(
    conn: &Connection,
) -> error::Result<()> {
    if let Ok(Some(serde_json::Value::Bool(t))) = load_value_from_global_settings_with_conn(
        conn,
        CRITICAL_ALERTS_ON_TOKEN_EXPIRY_SETTING,
        true,
    )
    .await
    {
        CRITICAL_ALERTS_ON_TOKEN_EXPIRY.store(t, Ordering::Relaxed);
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
    let log_dir = format!("{}/{}/", *TMP_WINDMILL_LOGS_SERVICE, hostname);
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
            "Error reading log files: {}, {:#?}",
            *TMP_WINDMILL_LOGS_SERVICE,
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
            let path = std::path::Path::new(&*TMP_WINDMILL_LOGS_SERVICE)
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
                    || current.no_proxy_hosts != new_settings.no_proxy_hosts
                    || current.insecure_upstream_hosts != new_settings.insecure_upstream_hosts
                    || current.upstream_ca_certs != new_settings.upstream_ca_certs
                {
                    tracing::info!(
                        "OTEL tracing proxy settings changed: enabled={}, languages={:?}, no_proxy_hosts={:?}, insecure_upstream_hosts={:?}, upstream_ca_certs={}",
                        new_settings.enabled,
                        new_settings.enabled_languages,
                        new_settings.no_proxy_hosts,
                        new_settings.insecure_upstream_hosts,
                        if new_settings.upstream_ca_certs.as_deref().unwrap_or("").trim().is_empty() { "unset" } else { "set" },
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

pub async fn load_disable_password_login(db: &DB) {
    let value = load_value_from_global_settings(db, DISABLE_PASSWORD_LOGIN_SETTING).await;
    match value {
        Ok(Some(serde_json::Value::Bool(t))) => DISABLE_PASSWORD_LOGIN.store(t, Ordering::Relaxed),
        Ok(None) => DISABLE_PASSWORD_LOGIN.store(false, Ordering::Relaxed),
        Err(e) => {
            tracing::error!("Error loading disable_password_login setting: {e:#}");
        }
        _ => (),
    };
}

struct LogFile {
    file_path: String,
    hostname: String,
}

struct TokenRow {
    token_prefix: Option<String>,
    label: Option<String>,
    email: Option<String>,
    workspace_id: Option<String>,
}

async fn report_token_expiration(db: &DB, token: &TokenRow, expired: bool) {
    if !windmill_common::auth::is_user_token(token.label.as_deref()) {
        return;
    }
    let prefix = token.token_prefix.as_deref().unwrap_or("??????????");
    let email_addr = token.email.as_deref().unwrap_or("unknown");
    let token_desc = match token.label.as_deref() {
        Some(l) if !l.is_empty() => format!("'{l}' ({prefix}****)"),
        _ => format!("{prefix}****"),
    };

    let (alert_message, email_subject, email_body) = if expired {
        (
            format!(
                "API token {token_desc} of '{email_addr}' has expired and been deleted"
            ),
            "Windmill: Your API token has expired",
            format!(
                "Your API token {token_desc} has expired and been deleted.\n\nPlease create a new token if you still need API access."
            ),
        )
    } else {
        (
            format!("API token {token_desc} of '{email_addr}' is expiring soon"),
            "Windmill: Your API token is expiring soon",
            format!(
                "Your API token {token_desc} is expiring soon.\n\nPlease rotate or renew your token to avoid service disruption."
            ),
        )
    };

    tracing::info!("{}", alert_message);
    if CRITICAL_ALERTS_ON_TOKEN_EXPIRY.load(Ordering::Relaxed) {
        report_critical_error(
            alert_message,
            db.clone(),
            token.workspace_id.as_deref(),
            None,
        )
        .await;
    }
    if let Some(email) = &token.email {
        send_email_if_possible(email_subject, &email_body, email);
    }
}

pub async fn delete_expired_items(db: &DB) -> () {
    let expired_tokens_r = sqlx::query_as!(
        TokenRow,
        "DELETE FROM token WHERE expiration <= now()
        RETURNING token_prefix, label, email, workspace_id",
    )
    .fetch_all(db)
    .await;

    match expired_tokens_r {
        Ok(tokens) => {
            if !tokens.is_empty() {
                tracing::info!("deleted {} expired tokens", tokens.len());
                for t in &tokens {
                    report_token_expiration(db, t, true).await;
                }
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
                delete_log_files_from_disk_and_store(paths, &*TMP_WINDMILL_LOGS_SERVICE, windmill_common::tracing_init::LOGS_SERVICE).await;

        }
        Err(e) => tracing::error!("Error deleting log file: {:?}", e),
    }

    let audit_retention_days = audit_log_retention_days().await;
    let audit_retention_secs: i64 = audit_retention_days * 60 * 60 * 24;

    // Clean up old (non-partitioned) audit table — will eventually be empty and dropped
    if let Err(e) = sqlx::query_scalar!(
        "DELETE FROM audit WHERE timestamp <= now() - ($1::bigint::text || ' s')::interval",
        audit_retention_secs,
    )
    .fetch_all(db)
    .await
    {
        tracing::error!("Error deleting audit log: {:?}", e);
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

    // native_retry_attempt has no FK to v2_job (kept off the hot bulk delete).
    // Retention sweeps markers alongside the jobs it deletes, but direct job
    // deletions (workspace/job/schedule clearing) leave markers orphaned — reap
    // any whose job is gone. The table is sparse, so this anti-join is cheap.
    if let Err(e) = sqlx::query!(
        "DELETE FROM native_retry_attempt nra WHERE NOT EXISTS (SELECT 1 FROM v2_job WHERE id = nra.job_id)"
    )
    .execute(db)
    .await
    {
        tracing::error!("Error reaping orphaned native retry markers: {:?}", e);
    }

    if let Err(e) = windmill_queue::cascade::reap_stale_join_slots(db).await {
        tracing::error!("Error reaping stale join_pending_inputs slots: {:?}", e);
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

    // Per-workspace retention overrides (EE-only; the cache is always empty on CE). A workspace may
    // keep jobs LONGER or SHORTER than the instance-wide window. Phase 1 sweeps globally on the
    // instance window but excludes override workspaces; Phase 2 sweeps each override workspace on its
    // own window (a sargable `workspace_id = $w` scan). The override count is capped small
    // (`MAX_RETENTION_OVERRIDE_WORKSPACES`), so Phase 2's per-workspace fan-out stays bounded.
    //
    // Deliberate simplicity/scale trade-off: a LONGER or keep-forever override lets that workspace's
    // old rows accumulate at the front of the completed_at index, and Phase 1's first batch each tick
    // scans past that retained prefix (an index scan, thanks to the sargable floor — not a Seq Scan)
    // before reaching a deletable row. This is only material at extreme scale (millions of retained
    // rows on one busy keep-forever workspace); we accept it rather than carrying a cross-tick
    // watermark, given overrides are a capped, targeted escape hatch.
    //
    // Gate the whole sweep on a confirmed-known override set: if the load never succeeded (e.g. a
    // startup DB hiccup, or malformed data), the empty cache is "unknown", not "no overrides", and
    // sweeping globally would delete jobs a longer-retention workspace configured. Retry the load
    // once here (on CE the flag is already set at startup, so this is a no-op), and skip the whole
    // job-cleanup phase this tick if still unknown — it runs again shortly.
    if !JOB_RETENTION_SECS_OVERRIDES_LOADED.load(std::sync::atomic::Ordering::Relaxed) {
        if let Err(e) = load_retention_period_overrides(db).await {
            tracing::error!("Error (re)loading per-workspace retention overrides: {e:#}");
        }
    }
    if !JOB_RETENTION_SECS_OVERRIDES_LOADED.load(std::sync::atomic::Ordering::Relaxed) {
        tracing::error!(
            "Skipping job retention cleanup this cycle: per-workspace overrides not yet loaded"
        );
    } else {
        let job_retention_secs = JOB_RETENTION_SECS.load(std::sync::atomic::Ordering::Relaxed);
        // `load_full` (owned Arc) rather than `load` (Guard): the sweep below holds this across many
        // `.await`s, and an arc_swap Guard is not meant to be held for long.
        let retention_overrides = JOB_RETENTION_SECS_OVERRIDES.load_full();
        let override_workspace_ids: Vec<String> = retention_overrides.keys().cloned().collect();

        // Phase 1: global sweep with the instance window, skipping override workspaces.
        if job_retention_secs > 0 {
            run_retention_cleanup(
                db,
                job_retention_secs,
                RetentionScope::GlobalExcluding(&override_workspace_ids),
            )
            .await;

            // Clean up concurrency keys separately (not tied to specific job IDs). Kept global on
            // the instance window — concurrency keys are short-lived and not worth per-workspace
            // scoping.
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

        // Phase 2: each override workspace swept on its own window. A window of 0 means "keep
        // forever" for that workspace, so it is excluded from Phase 1 above and skipped here. The
        // override count is capped at MAX_RETENTION_OVERRIDE_WORKSPACES (enforced at write time), so
        // this loop runs a bounded number of scoped sweeps per pass.
        for (w_id, retention_secs) in retention_overrides.iter() {
            if *retention_secs > 0 {
                run_retention_cleanup(db, *retention_secs, RetentionScope::OnlyWorkspace(w_id))
                    .await;
            }
        }
    }

    match windmill_common::trashbin::delete_expired_trash(db).await {
        Ok(count) => {
            if count > 0 {
                tracing::info!("deleted {} expired trash items", count);
            }
        }
        Err(e) => tracing::error!("Error deleting expired trash items: {}", e.to_string()),
    }
}

#[cfg(feature = "enterprise")]
async fn cleanup_scheduled_job_deletions(db: &Pool<Postgres>) {
    const BATCH_SIZE: i64 = 1000;
    const MAX_BATCHES: i32 = 10;

    let mut total_deleted = 0u64;
    for batch_num in 0..MAX_BATCHES {
        let mut tx = match db.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!("Error starting transaction for scheduled job deletion: {e:?}");
                break;
            }
        };

        let rows = match sqlx::query_scalar!(
            "DELETE FROM job_delete_schedule
             WHERE job_id IN (
                 SELECT job_id FROM job_delete_schedule
                 WHERE delete_at <= now()
                 ORDER BY delete_at
                 LIMIT $1
                 FOR UPDATE SKIP LOCKED
             )
             RETURNING job_id",
            BATCH_SIZE,
        )
        .fetch_all(&mut *tx)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!("Error in scheduled job deletion batch {batch_num}: {e:?}");
                break;
            }
        };

        if rows.is_empty() {
            break;
        }

        let job_ids = rows;
        let count = job_ids.len() as u64;

        let cleanup_result: Result<(), sqlx::Error> = async {
            sqlx::query!(
                "UPDATE v2_job SET args = '{}'::jsonb WHERE id = ANY($1)",
                &job_ids,
            )
            .execute(&mut *tx)
            .await?;
            sqlx::query!(
                "UPDATE v2_job_completed SET result = '{}'::jsonb WHERE id = ANY($1)",
                &job_ids,
            )
            .execute(&mut *tx)
            .await?;
            sqlx::query!(
                "UPDATE job_logs SET logs = '##DELETED##' WHERE job_id = ANY($1)",
                &job_ids,
            )
            .execute(&mut *tx)
            .await?;
            Ok(())
        }
        .await;

        if let Err(e) = cleanup_result {
            // Roll back so schedule rows survive for retry on next cycle
            tracing::error!("Error cleaning job data in batch {batch_num}, rolling back: {e:?}");
            break;
        }

        if let Err(e) = tx.commit().await {
            tracing::error!("Error committing scheduled job deletion batch {batch_num}: {e:?}");
            break;
        }

        total_deleted += count;
    }

    if total_deleted > 0 {
        tracing::info!("Scheduled job deletion: cleaned {total_deleted} jobs");
    }
}

pub async fn check_expiring_tokens(db: &DB) {
    // Find tokens expiring within 7 days that still have a pending notification row.
    // The notification table stores token_hash (not plaintext) so the join works
    // even after the hash migration makes token.token nullable.
    let expiring_tokens_r = sqlx::query_as!(
        TokenRow,
        "DELETE FROM token_expiry_notification n
         USING token t
         WHERE n.token_hash = t.token_hash
           AND n.expiration > now()
           AND n.expiration <= now() + interval '7 days'
         RETURNING t.token_prefix, t.label, t.email, t.workspace_id",
    )
    .fetch_all(db)
    .await;

    match expiring_tokens_r {
        Ok(tokens) => {
            for t in &tokens {
                report_token_expiration(db, t, false).await;
            }
            if !tokens.is_empty() {
                tracing::info!("Sent expiration warnings for {} token(s)", tokens.len());
            }
        }
        Err(e) => tracing::error!("Error checking expiring tokens: {}", e),
    }

    // Clean up notification rows whose expiration has passed
    if let Err(e) = sqlx::query!("DELETE FROM token_expiry_notification WHERE expiration <= now()")
        .execute(db)
        .await
    {
        tracing::error!("Error cleaning up expired token notifications: {}", e);
    }
}

/// Delete a batch of expired jobs with LIMIT and SKIP LOCKED for high-scale environments.
/// Uses a single transaction per batch to minimize lock duration.
///
/// `completed_at_floor` is the watermark from the previous batch in the same cleanup run (the
/// max `completed_at` it deleted); pass `None` for the first batch. It is re-applied as
/// `completed_at >= floor` so the scan resumes past the rows already processed instead of
/// re-walking them (see the inline comment on the DELETE for why this matters).
///
/// Returns `(jobs deleted in this batch, max completed_at deleted)`. The caller feeds the
/// returned watermark back in as `completed_at_floor` for the next batch.
///
/// `only_workspace` and `exclude_workspaces` implement the per-workspace retention override and are
/// mutually exclusive: Phase 1 passes `exclude_workspaces` (skip override workspaces, sweep the
/// rest), Phase 2 passes `only_workspace` (sweep just that workspace on its own window). Both `None`
/// reproduces the plain global sweep exactly. See `run_retention_cleanup` / `delete_expired_items`.
async fn delete_expired_jobs_batch(
    db: &DB,
    job_retention_secs: i64,
    batch_size: i64,
    completed_at_floor: Option<DateTime<Utc>>,
    only_workspace: Option<&str>,
    exclude_workspaces: Option<&[String]>,
) -> error::Result<(usize, Option<DateTime<Utc>>)> {
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

    // `completed_at_floor` is a watermark carried across batches within a cleanup run: it is the
    // max(completed_at) deleted by the previous batch. Re-applying it as `completed_at >= floor`
    // lets each batch resume after the rows the previous batch already processed instead of
    // re-scanning them. This matters when the oldest rows are undeletable (children of a
    // still-active root flow, or override workspaces excluded from the global sweep): without the
    // floor the `ORDER BY completed_at ASC` scan walks that same protected/retained prefix on every
    // batch, turning a cleanup run quadratic in prefix size.
    // Floor only ever skips rows the current run already deleted, was protecting, or skip-locked —
    // all correctly deferred to the next run, identical to the unbounded scan's semantics.
    //
    // It is applied as `completed_at >= COALESCE($floor, '-infinity')`, NOT `$floor IS NULL OR
    // completed_at >= $floor`: the `OR ... IS NULL` disjunction is non-sargable, so the planner
    // cannot use the floor as an index lower bound and falls back to a Seq Scan of the whole table —
    // walking the entire prefix regardless of the floor. The COALESCE sentinel keeps a single cached
    // query while making the bound a plain range predicate the completed_at / composite index drives.
    //
    // Use FOR UPDATE SKIP LOCKED to avoid contention between replicas; ORDER BY completed_at
    // deletes oldest jobs first.
    // Two orthogonal choices drive which DELETE we run:
    //  - `only_workspace`: Some => a single-workspace (Phase 2) sweep. We bind `workspace_id = $n`
    //    directly (no `OR $n IS NULL` guard) so the composite `(workspace_id, completed_at)` index
    //    can drive the ordered scan — a sargable equality the OR-form would defeat. `None` => a
    //    global (Phase 1) sweep that instead excludes override workspaces via a hashed `NOT IN
    //    (SELECT ... unnest($exclude))` SubPlan (same one-time-hash trick as the active-root
    //    exclusion below): O(1) membership per candidate, vs `<> ALL($exclude)`'s per-row linear
    //    array scan which degrades sharply once many workspaces have overrides.
    //  - `active_root_job_ids.is_empty()`: skip the `v2_job` join entirely when nothing is
    //    protected (a PK lookup per candidate is pure overhead in the common case).
    let (deleted_jobs, max_completed_at) = match only_workspace {
        Some(w_id) if active_root_job_ids.is_empty() => {
            let rows = sqlx::query!(
                "DELETE FROM v2_job_completed
                 WHERE id IN (
                     SELECT id FROM v2_job_completed
                     WHERE workspace_id = $4
                       AND completed_at <= now() - ($1::bigint::text || ' s')::interval
                       AND completed_at >= COALESCE($3::timestamptz, '-infinity'::timestamptz)
                     ORDER BY completed_at ASC
                     LIMIT $2
                     FOR UPDATE SKIP LOCKED
                 )
                 RETURNING id, completed_at",
                job_retention_secs,
                batch_size,
                completed_at_floor,
                w_id,
            )
            .fetch_all(&mut *tx)
            .await?;
            let max = rows.iter().map(|r| r.completed_at).max();
            (rows.into_iter().map(|r| r.id).collect::<Vec<Uuid>>(), max)
        }
        Some(w_id) => {
            let rows = sqlx::query!(
                "DELETE FROM v2_job_completed
                 WHERE id IN (
                     SELECT jc.id FROM v2_job_completed jc
                     LEFT JOIN v2_job j ON j.id = jc.id
                     WHERE jc.workspace_id = $5
                       AND jc.completed_at <= now() - ($1::bigint::text || ' s')::interval
                       AND jc.completed_at >= COALESCE($4::timestamptz, '-infinity'::timestamptz)
                       AND COALESCE(j.root_job, j.flow_innermost_root_job, jc.id) NOT IN (
                           SELECT u FROM unnest($3::uuid[]) AS u WHERE u IS NOT NULL
                       )
                     ORDER BY jc.completed_at ASC
                     LIMIT $2
                     FOR UPDATE OF jc SKIP LOCKED
                 )
                 RETURNING id, completed_at",
                job_retention_secs,
                batch_size,
                &active_root_job_ids,
                completed_at_floor,
                w_id,
            )
            .fetch_all(&mut *tx)
            .await?;
            let max = rows.iter().map(|r| r.completed_at).max();
            (rows.into_iter().map(|r| r.id).collect::<Vec<Uuid>>(), max)
        }
        None if active_root_job_ids.is_empty() => {
            let rows = sqlx::query!(
                "DELETE FROM v2_job_completed
                 WHERE id IN (
                     SELECT id FROM v2_job_completed
                     WHERE completed_at <= now() - ($1::bigint::text || ' s')::interval
                       AND completed_at >= COALESCE($3::timestamptz, '-infinity'::timestamptz)
                       AND ($4::text[] IS NULL OR workspace_id NOT IN (
                           SELECT u FROM unnest($4::text[]) AS u WHERE u IS NOT NULL
                       ))
                     ORDER BY completed_at ASC
                     LIMIT $2
                     FOR UPDATE SKIP LOCKED
                 )
                 RETURNING id, completed_at",
                job_retention_secs,
                batch_size,
                completed_at_floor,
                exclude_workspaces,
            )
            .fetch_all(&mut *tx)
            .await?;
            let max = rows.iter().map(|r| r.completed_at).max();
            (rows.into_iter().map(|r| r.id).collect::<Vec<Uuid>>(), max)
        }
        None => {
            // Active-root exclusion uses `NOT IN (SELECT ... unnest($3))` rather than `!= ALL($3)`:
            // the subquery form lets the planner build a one-time hashed SubPlan and apply it as a
            // filter on the ordered index scan, giving O(1) membership per candidate instead of a
            // per-row linear array scan (which degrades sharply when many root jobs are active). The
            // `u IS NOT NULL` guard sidesteps NOT IN's null-trap semantics ($3 holds non-null PK ids).
            let rows = sqlx::query!(
                "DELETE FROM v2_job_completed
                 WHERE id IN (
                     SELECT jc.id FROM v2_job_completed jc
                     LEFT JOIN v2_job j ON j.id = jc.id
                     WHERE jc.completed_at <= now() - ($1::bigint::text || ' s')::interval
                       AND jc.completed_at >= COALESCE($4::timestamptz, '-infinity'::timestamptz)
                       AND ($5::text[] IS NULL OR jc.workspace_id NOT IN (
                           SELECT u FROM unnest($5::text[]) AS u WHERE u IS NOT NULL
                       ))
                       AND COALESCE(j.root_job, j.flow_innermost_root_job, jc.id) NOT IN (
                           SELECT u FROM unnest($3::uuid[]) AS u WHERE u IS NOT NULL
                       )
                     ORDER BY jc.completed_at ASC
                     LIMIT $2
                     FOR UPDATE OF jc SKIP LOCKED
                 )
                 RETURNING id, completed_at",
                job_retention_secs,
                batch_size,
                &active_root_job_ids,
                completed_at_floor,
                exclude_workspaces,
            )
            .fetch_all(&mut *tx)
            .await?;
            let max = rows.iter().map(|r| r.completed_at).max();
            (rows.into_iter().map(|r| r.id).collect::<Vec<Uuid>>(), max)
        }
    };

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
                delete_log_files_from_disk_and_store(paths, &*WINDMILL_DIR, "").await;
            }
            Err(e) => tracing::error!("Error deleting job logs: {:?}", e),
        }

        // Native retry markers have no FK (to keep this bulk delete cheap) — sweep
        // them with their jobs here too (the periodic retention path), same as the
        // other side tables. The table is created by a startup migration, so it
        // always exists by the time cleanup runs.
        if let Err(e) = sqlx::query!(
            "DELETE FROM native_retry_attempt WHERE job_id = ANY($1)",
            &deleted_jobs
        )
        .execute(&mut *tx)
        .await
        {
            tracing::error!("Error deleting native retry markers: {:?}", e);
        }

        if let Err(e) = delete_jobs(&mut *tx, &deleted_jobs).await {
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

    Ok((deleted_count, max_completed_at))
}

/// Which workspaces a retention cleanup run targets.
#[derive(Debug)]
enum RetentionScope<'a> {
    /// Sweep every workspace except the listed ones (they run in their own Phase-2 pass).
    GlobalExcluding(&'a [String]),
    /// Sweep only this single workspace, on its own retention window.
    OnlyWorkspace(&'a str),
}

/// Drives the batched job-retention delete for a given `retention_secs` window and `scope`.
/// Preserves the per-run `completed_at_floor` watermark across batches (see
/// `delete_expired_jobs_batch`). Returns the number of jobs deleted.
///
/// `JOB_CLEANUP_MAX_BATCHES` bounds the batches per call, i.e. per scope. A full cleanup cycle can
/// therefore run up to `(1 + n_override_workspaces) * max_batches` batches; the override count is
/// capped at `MAX_RETENTION_OVERRIDE_WORKSPACES`, and any residue is picked up on the next tick.
async fn run_retention_cleanup(db: &DB, retention_secs: i64, scope: RetentionScope<'_>) -> u64 {
    let (only_workspace, exclude_workspaces): (Option<&str>, Option<&[String]>) = match &scope {
        // An empty exclusion list binds as NULL so the guard short-circuits to the plain sweep.
        RetentionScope::GlobalExcluding(ids) => {
            (None, if ids.is_empty() { None } else { Some(*ids) })
        }
        RetentionScope::OnlyWorkspace(w_id) => (Some(*w_id), None),
    };

    let batch_size = *JOB_CLEANUP_BATCH_SIZE;
    let max_batches = *JOB_CLEANUP_MAX_BATCHES;
    let cleanup_start = Instant::now();
    let mut total_deleted = 0u64;
    let mut batch_num = 0i32;
    // Watermark carried across batches so each one resumes after the rows the previous batch
    // already processed instead of re-scanning the (potentially undeletable) oldest prefix.
    let mut completed_at_floor: Option<DateTime<Utc>> = None;

    // Process batches until no more expired jobs or max batches reached
    loop {
        if max_batches > 0 && batch_num >= max_batches {
            tracing::debug!(
                "Job cleanup ({scope:?}): reached max batches limit ({max_batches}), will continue next iteration"
            );
            break;
        }

        // Each batch runs in its own transaction to avoid long-running locks
        let batch_result = delete_expired_jobs_batch(
            db,
            retention_secs,
            batch_size,
            completed_at_floor,
            only_workspace,
            exclude_workspaces,
        )
        .await;

        match batch_result {
            Ok((deleted_count, max_completed_at)) => {
                if deleted_count == 0 {
                    // No more expired jobs to delete
                    break;
                }
                completed_at_floor = max_completed_at.or(completed_at_floor);
                total_deleted += deleted_count as u64;
                batch_num += 1;
            }
            Err(e) => {
                tracing::error!("Error in job cleanup batch {batch_num} ({scope:?}): {e:?}");
                break;
            }
        }
    }

    if total_deleted > 0 {
        tracing::info!(
            "Job cleanup completed ({scope:?}): deleted {total_deleted} jobs in {batch_num} batches, took {:?}",
            cleanup_start.elapsed()
        );
    }

    total_deleted
}

/// Parses the raw `{workspace_id: seconds}` global-setting object into an override map. Returns
/// `Err` (with the offending workspace) if ANY value is not a non-negative integer, so the caller
/// can keep the last-good map instead of dropping just that entry — dropping a longer-retention
/// entry would let the Phase-1 global window delete its jobs, and a negative value would silently
/// become keep-forever (Phase 2 only sweeps `> 0`).
#[cfg(feature = "enterprise")]
fn parse_retention_overrides(
    map: serde_json::Map<String, serde_json::Value>,
) -> std::result::Result<std::collections::HashMap<String, i64>, String> {
    use windmill_common::global_settings::MAX_RETENTION_OVERRIDE_WORKSPACES;
    if map.len() > MAX_RETENTION_OVERRIDE_WORKSPACES {
        return Err(format!(
            "at most {MAX_RETENTION_OVERRIDE_WORKSPACES} per-workspace retention overrides are allowed, got {}",
            map.len()
        ));
    }
    let mut overrides = std::collections::HashMap::with_capacity(map.len());
    for (w_id, v) in map {
        match v.as_i64() {
            Some(secs) if secs >= 0 => {
                overrides.insert(w_id, secs);
            }
            _ => {
                return Err(format!(
                    "override for '{w_id}' must be a non-negative integer number of seconds, got {v}"
                ));
            }
        }
    }
    Ok(overrides)
}

/// Loads the per-workspace retention overrides from the `retention_period_secs_overrides` global
/// setting (a JSON `{workspace_id: secs}` object) into the in-memory `JOB_RETENTION_SECS_OVERRIDES`
/// cache, so the cleanup sweep reads them without a per-tick DB query. Enterprise-only — CE leaves
/// the cache empty so the sweep behaves exactly as before.
///
/// On a load error, unexpected value shape, or malformed data the previous map is kept but
/// `JOB_RETENTION_SECS_OVERRIDES_LOADED` is set to FALSE, marking the cache unknown. Clobbering the
/// map to empty would let the global sweep delete jobs a workspace asked to keep longer; leaving the
/// flag TRUE would keep the stale (possibly shorter) policy in force after a lengthened/added
/// override fails to refresh, deleting those jobs prematurely. Marking it unknown makes the sweep
/// fail closed — it skips and the monitor retries the load next tick until a confirmed-current state
/// loads. `LOADED` is set true only on a valid map, explicit unset (`Ok(None)`), or CE's no-op.
pub async fn load_retention_period_overrides(db: &DB) -> error::Result<()> {
    #[cfg(not(feature = "enterprise"))]
    {
        let _ = db;
        // Overrides are EE-only; empty is the correct, fully-known state on CE.
        JOB_RETENTION_SECS_OVERRIDES_LOADED.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    #[cfg(feature = "enterprise")]
    {
        use windmill_common::global_settings::RETENTION_PERIOD_SECS_OVERRIDES_SETTING;
        let value =
            load_value_from_global_settings(db, RETENTION_PERIOD_SECS_OVERRIDES_SETTING).await;
        match value {
            Ok(Some(serde_json::Value::Object(map))) => match parse_retention_overrides(map) {
                Ok(overrides) => {
                    JOB_RETENTION_SECS_OVERRIDES.store(std::sync::Arc::new(overrides));
                    JOB_RETENTION_SECS_OVERRIDES_LOADED
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                // Malformed persisted value: we can't confirm the current override set. Keep the
                // last-good map but mark the cache unknown so the sweep fails closed (skips) and
                // retries, rather than deleting with a stale — possibly shorter — policy.
                Err(reason) => {
                    JOB_RETENTION_SECS_OVERRIDES_LOADED
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                    tracing::error!(
                        "Malformed per-workspace retention overrides, gating cleanup until it loads: {reason}"
                    );
                }
            },
            Ok(None) => {
                // Explicit unset is a known state: no overrides.
                JOB_RETENTION_SECS_OVERRIDES
                    .store(std::sync::Arc::new(std::collections::HashMap::new()));
                JOB_RETENTION_SECS_OVERRIDES_LOADED
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            // Unexpected shape / read failure: mark unknown so a lengthened or added override that
            // failed to refresh can't be missed by a sweep still running the previous policy.
            Ok(Some(other)) => {
                JOB_RETENTION_SECS_OVERRIDES_LOADED
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                tracing::error!(
                    "Per-workspace retention overrides setting is not a JSON object (got {other}); gating cleanup until it loads"
                );
            }
            Err(e) => {
                JOB_RETENTION_SECS_OVERRIDES_LOADED
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                tracing::error!(
                    "Error loading per-workspace retention overrides, gating cleanup until it loads: {e:#}"
                );
            }
        }
    }
    Ok(())
}

async fn delete_log_files_from_disk_and_store(
    paths_to_delete: Vec<String>,
    tmp_dir: &str,
    _s3_prefix: &str,
) {
    // S3 bulk delete (batched via delete_stream — on S3 this uses the DeleteObjects
    // API, up to 1000 objects per request).
    #[cfg(feature = "parquet")]
    {
        let should_del_from_store =
            MONITOR_LOGS_ON_OBJECT_STORE.load(std::sync::atomic::Ordering::Relaxed);
        if should_del_from_store {
            if let Some(os) = windmill_object_store::get_object_store().await {
                let s3_paths: Vec<_> = paths_to_delete
                    .iter()
                    .map(|p| {
                        windmill_object_store::object_store_reexports::Path::from(format!(
                            "{}{}",
                            _s3_prefix, p
                        ))
                    })
                    .map(Ok)
                    .collect();
                let stream = futures::stream::iter(s3_paths).boxed();
                let mut result = os.delete_stream(stream);
                let mut deleted = 0u64;
                let mut not_found = 0u64;
                let mut failed = 0u64;
                while let Some(r) = result.next().await {
                    match r {
                        Ok(_) => deleted += 1,
                        // Deleting a non-existent object is a successful no-op. S3's
                        // DeleteObjects ignores missing keys, but GCS returns 404 per
                        // delete, surfacing as NotFound — count it separately rather
                        // than logging it as an error.
                        Err(windmill_object_store::object_store_reexports::ObjectStoreError::NotFound { .. }) => {
                            not_found += 1;
                        }
                        Err(e) => {
                            failed += 1;
                            tracing::error!("Failed to delete from object store: {e}");
                        }
                    }
                }
                if deleted + not_found + failed > 0 {
                    tracing::info!(
                        "object store log cleanup: {deleted} deleted, {not_found} already absent (404), {failed} failed"
                    );
                }
            }
        }
    }

    // Disk delete in parallel.
    let delete_futures = FuturesUnordered::new();
    for path in paths_to_delete {
        delete_futures.push(async move {
            let disk_path = std::path::Path::new(tmp_dir).join(&path);
            if tokio::fs::metadata(&disk_path).await.is_ok() {
                if let Err(e) = tokio::fs::remove_file(&disk_path).await {
                    tracing::error!(
                        "Failed to delete from disk {}: {e}",
                        disk_path.to_string_lossy()
                    );
                }
            }
        });
    }
    let _: Vec<_> = delete_futures.collect().await;
}

pub async fn reload_instance_events_webhook_setting(db: &DB) {
    use windmill_common::global_settings::INSTANCE_EVENTS_WEBHOOK_SETTING;
    use windmill_common::webhook::INSTANCE_EVENTS_WEBHOOK;

    let value = load_value_from_global_settings(db, INSTANCE_EVENTS_WEBHOOK_SETTING).await;
    match value {
        Ok(Some(serde_json::Value::String(s))) if !s.is_empty() => {
            INSTANCE_EVENTS_WEBHOOK.store(std::sync::Arc::new(Some(s)));
        }
        Ok(None) | Ok(Some(serde_json::Value::Null)) | Ok(Some(serde_json::Value::String(_))) => {
            // Fall back to env var if DB has no value
            INSTANCE_EVENTS_WEBHOOK.store(std::sync::Arc::new(
                std::env::var("INSTANCE_EVENTS_WEBHOOK").ok(),
            ));
        }
        Err(e) => {
            tracing::error!("Error loading instance_events_webhook setting: {e:#}");
        }
        _ => (),
    };
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

pub async fn reload_uv_exclude_newer_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        UV_EXCLUDE_NEWER_SETTING,
        "UV_EXCLUDE_NEWER",
        UV_EXCLUDE_NEWER.clone(),
    )
    .await;
}

pub async fn reload_uv_python_install_mirror_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        UV_PYTHON_INSTALL_MIRROR_SETTING,
        "UV_PYTHON_INSTALL_MIRROR",
        UV_PYTHON_INSTALL_MIRROR.clone(),
    )
    .await;
}

pub async fn reload_bun_install_min_release_age_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        BUN_INSTALL_MIN_RELEASE_AGE_SETTING,
        "BUN_INSTALL_MIN_RELEASE_AGE",
        BUN_INSTALL_MIN_RELEASE_AGE.clone(),
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

pub async fn reload_npmrc_setting(conn: &Connection) {
    reload_option_setting_with_tracing(conn, NPMRC_SETTING, "NPMRC", NPMRC.clone()).await;
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
            let m2_dir = format!("{}/.m2", *JAVA_HOME_DIR);
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
            let settings_path = format!("{}/.m2/settings.xml", *JAVA_HOME_DIR);
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

pub async fn reload_workspace_registries_setting(conn: &Connection) {
    let value = load_value_from_global_settings_with_conn(
        conn,
        windmill_common::global_settings::WORKSPACE_REGISTRIES_SETTING,
        true,
    )
    .await;
    match value {
        Ok(Some(v)) => match serde_json::from_value::<WorkspaceRegistryMap>(v) {
            Ok(parsed) => {
                tracing::info!(
                    "Loaded workspace registries for {} workspaces",
                    parsed.len()
                );
                *WORKSPACE_REGISTRIES.write().await = Some(parsed);
            }
            Err(e) => {
                tracing::error!("Error parsing workspace_registries setting: {e:#}");
            }
        },
        Ok(None) => {
            *WORKSPACE_REGISTRIES.write().await = None;
        }
        Err(e) => {
            tracing::error!("Error loading workspace_registries setting: {e:#}");
        }
    }
}

pub async fn reload_hub_api_secret_setting(conn: &Connection) {
    match load_option_setting_value::<String>(conn, HUB_API_SECRET_SETTING, "HUB_API_SECRET").await
    {
        Ok(v) => HUB_API_SECRET.store(std::sync::Arc::new(v)),
        Err(e) => tracing::error!("Error reloading setting HUB_API_SECRET: {:?}", e),
    }
}

pub async fn reload_retention_period_setting(conn: &Connection) {
    match load_setting_value::<i64>(
        conn,
        RETENTION_PERIOD_SECS_SETTING,
        "JOB_RETENTION_SECS",
        60 * 60 * 24 * 30,
        |x| x,
    )
    .await
    {
        Ok(v) => JOB_RETENTION_SECS.store(v, Ordering::Relaxed),
        Err(e) => tracing::error!("Error reloading retention period: {:?}", e),
    }
}

pub async fn reload_audit_log_retention_days_setting(conn: &Connection) {
    match load_setting_value::<i64>(
        conn,
        AUDIT_LOG_RETENTION_DAYS_SETTING,
        "AUDIT_LOG_RETENTION_DAYS",
        0, // 0 means use default: 365 for EE, 14 for CE
        |x| x,
    )
    .await
    {
        Ok(v) => AUDIT_LOG_RETENTION_DAYS.store(v, Ordering::Relaxed),
        Err(e) => tracing::error!("Error reloading audit log retention days: {:?}", e),
    }
}

pub async fn reload_delete_logs_periodically_setting(conn: &Connection) {
    match load_setting_value::<bool>(
        conn,
        MONITOR_LOGS_ON_OBJECT_STORE_SETTING,
        "MONITOR_LOGS_ON_OBJECT_STORE",
        false,
        |x| x,
    )
    .await
    {
        Ok(v) => MONITOR_LOGS_ON_OBJECT_STORE.store(v, Ordering::Relaxed),
        Err(e) => tracing::error!("Error reloading retention period: {:?}", e),
    }
}

pub async fn reload_store_audit_logs_s3_setting(conn: &Connection) {
    match load_setting_value::<bool>(
        conn,
        STORE_AUDIT_LOGS_S3_SETTING,
        "STORE_AUDIT_LOGS_S3",
        false,
        |x| x,
    )
    .await
    {
        Ok(v) => STORE_AUDIT_LOGS_S3.store(v, Ordering::Relaxed),
        Err(e) => tracing::error!("Error reloading store_audit_logs_s3 setting: {:?}", e),
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

pub async fn reload_nsjail_tmpfs_size_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        NSJAIL_TMPFS_SIZE_MB_SETTING,
        "NSJAIL_TMPFS_SIZE_MB",
        NSJAIL_TMPFS_SIZE_MB.clone(),
    )
    .await;
}

pub async fn reload_nsjail_tmp_backing_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        NSJAIL_TMP_BACKING_SETTING,
        "NSJAIL_TMP_BACKING",
        NSJAIL_TMP_BACKING.clone(),
    )
    .await;
}

pub async fn reload_sandbox_image_max_size_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        SANDBOX_IMAGE_MAX_SIZE_MB_SETTING,
        "SANDBOX_IMAGE_MAX_SIZE_MB",
        SANDBOX_IMAGE_MAX_SIZE_MB.clone(),
    )
    .await;
}

pub async fn reload_sandbox_image_cache_max_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        SANDBOX_IMAGE_CACHE_MAX_MB_SETTING,
        "SANDBOX_IMAGE_CACHE_MAX_MB",
        SANDBOX_IMAGE_CACHE_MAX_MB.clone(),
    )
    .await;
}

pub async fn reload_sandbox_image_pull_policy_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        SANDBOX_IMAGE_PULL_POLICY_SETTING,
        "SANDBOX_IMAGE_PULL_POLICY",
        SANDBOX_IMAGE_PULL_POLICY.clone(),
    )
    .await;
}

pub async fn reload_sandbox_image_default_registry_setting(conn: &Connection) {
    reload_option_setting_with_tracing(
        conn,
        SANDBOX_IMAGE_DEFAULT_REGISTRY_SETTING,
        "SANDBOX_IMAGE_DEFAULT_REGISTRY",
        SANDBOX_IMAGE_DEFAULT_REGISTRY.clone(),
    )
    .await;
}

pub async fn reload_sandbox_registry_auth_setting(conn: &Connection) {
    // Secret-aware: the value is a raw docker/podman auth.json with credentials, so
    // it must never be logged. Load directly (the generic reload_option_setting path
    // logs the value via load_option_setting_value) and only log a redacted message.
    let q =
        match load_value_from_global_settings_with_conn(conn, SANDBOX_REGISTRY_AUTH_SETTING, true)
            .await
        {
            Ok(q) => q,
            Err(e) => {
                tracing::error!("Error reloading setting SANDBOX_REGISTRY_AUTH: {e:?}");
                return;
            }
        };
    let value = q.and_then(|q| serde_json::from_value::<String>(q).ok());
    let configured = value.as_ref().is_some_and(|v| !v.trim().is_empty());
    *SANDBOX_REGISTRY_AUTH.write().await = value;
    tracing::info!("Loaded setting SANDBOX_REGISTRY_AUTH (redacted), configured={configured}");
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
    if value == JobIsolationLevel::Unshare && UNSHARE_PATH.is_none() {
        tracing::error!(
            "job_isolation is set to unshare but the unshare binary is not available on this worker. \
            Jobs will run without isolation until unshare is installed or the setting is changed."
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

async fn resolve_license_key_value(conn: &Connection, quiet: bool) -> anyhow::Result<String> {
    let q = load_value_from_global_settings_with_conn(conn, LICENSE_KEY_SETTING, true)
        .await
        .map_err(|err| anyhow::anyhow!("Error reloading license key: {}", err.to_string()))?;

    let mut value = std::env::var("LICENSE_KEY")
        .ok()
        .and_then(|x| x.parse::<String>().ok())
        .unwrap_or(String::new());

    if let Some(q) = q {
        if let Ok(v) = serde_json::from_value::<String>(q.clone()) {
            if !quiet {
                tracing::info!(
                    "Loaded setting LICENSE_KEY from db config: {}",
                    truncate_token(&v)
                );
            }
            value = v;
        } else {
            tracing::error!("Could not parse LICENSE_KEY found: {:#?}", &q);
        }
    };
    Ok(value)
}

pub async fn reload_license_key(conn: &Connection) -> anyhow::Result<()> {
    let value = resolve_license_key_value(conn, false).await?;
    apply_license_key(value, conn).await;
    Ok(())
}

/// Applies the key and records it as the last accepted value only when
/// `set_license_key` actually stored it (validation passed, even if expired).
/// A rejected key is deliberately not recorded: validation can fail transiently
/// (DB error during the instance-hash check, offline key validated before
/// base_url is configured), and recording it would stop
/// `refetch_license_key_if_invalid` from ever retrying an unchanged key.
async fn apply_license_key(value: String, conn: &Connection) {
    set_license_key(value.clone(), conn.as_sql()).await;
    #[cfg(feature = "enterprise")]
    if (**windmill_common::ee_oss::LICENSE_KEY.load()).as_str() == value {
        *LAST_ACCEPTED_LICENSE_KEY.lock().unwrap() = Some(value);
    }
}

#[cfg(feature = "enterprise")]
lazy_static::lazy_static! {
    static ref LAST_INVALID_KEY_REFETCH_AT: std::sync::Mutex<Option<Instant>> =
        std::sync::Mutex::new(None);
    static ref LAST_ACCEPTED_LICENSE_KEY: std::sync::Mutex<Option<String>> =
        std::sync::Mutex::new(None);
}

#[cfg(feature = "enterprise")]
const INVALID_LICENSE_KEY_REFETCH_INTERVAL: Duration = Duration::from_secs(60);

/// When the in-memory license key is invalid, the key in settings may have moved on
/// (failed initial load, missed `license_key` notification, or renewed/fixed by
/// another instance) — without this, the stale in-memory key would keep being
/// flagged invalid until the next full settings reload (12h by default). Refetch
/// the latest key at most once per `INVALID_LICENSE_KEY_REFETCH_INTERVAL` and
/// re-apply it unless it matches the last accepted value (an unchanged key that
/// validated fine and is invalid for another reason — e.g. expired — is not
/// re-validated in a loop).
#[cfg(feature = "enterprise")]
pub async fn refetch_license_key_if_invalid(conn: &Connection) {
    use windmill_common::ee_oss::LICENSE_KEY_VALID;

    if LICENSE_KEY_VALID.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    {
        let mut last = LAST_INVALID_KEY_REFETCH_AT.lock().unwrap();
        if last.is_some_and(|t| t.elapsed() < INVALID_LICENSE_KEY_REFETCH_INTERVAL) {
            return;
        }
        *last = Some(Instant::now());
    }
    let value = match resolve_license_key_value(conn, true).await {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to refetch license key while invalid: {err:#}");
            return;
        }
    };
    let changed = LAST_ACCEPTED_LICENSE_KEY.lock().unwrap().as_deref() != Some(value.as_str());
    if changed {
        tracing::info!(
            "In-memory license key is invalid, applying license key from settings: {}",
            truncate_token(&value)
        );
        apply_license_key(value, conn).await;
    }
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

/// Load an optional setting value without writing it anywhere.
///
/// Extracted from [`reload_option_setting`] so callers that store the value
/// in something other than `Arc<RwLock<Option<T>>>` (e.g. `ArcSwap<Option<T>>`,
/// an `AtomicBool`, etc.) can reuse the load pipeline.
pub async fn load_option_setting_value<T: FromStr + DeserializeOwned>(
    conn: &Connection,
    setting_name: &str,
    std_env_var: &str,
) -> error::Result<Option<T>> {
    let force_value = std::env::var(format!("FORCE_{}", std_env_var))
        .ok()
        .and_then(|x| x.parse::<T>().ok());

    if let Some(force_value) = force_value {
        return Ok(Some(force_value));
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

    if value.is_none() {
        tracing::info!("Loaded {setting_name} setting to None");
    }

    Ok(value)
}

pub async fn reload_option_setting<T: FromStr + DeserializeOwned>(
    conn: &Connection,
    setting_name: &str,
    std_env_var: &str,
    lock: Arc<RwLock<Option<T>>>,
) -> error::Result<()> {
    let value = load_option_setting_value::<T>(conn, setting_name, std_env_var).await?;
    {
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

/// Load an optional URL list setting without writing it anywhere.
///
/// Extracted from [`reload_url_list_setting`] so callers that store the
/// value in something other than `Arc<RwLock<Option<Vec<Url>>>>` (e.g.
/// `ArcSwap<Option<Vec<Url>>>`) can reuse the parsing pipeline.
pub async fn load_url_list_setting_value(
    conn: &Connection,
    setting_name: &str,
    std_env_var: &str,
) -> error::Result<Option<Vec<url::Url>>> {
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
        return Ok(if urls.is_empty() { None } else { Some(urls) });
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

    if value.is_none() {
        tracing::info!("Loaded {} setting to None", setting_name);
    }

    Ok(value)
}

pub async fn reload_url_list_setting(
    conn: &Connection,
    setting_name: &str,
    std_env_var: &str,
    lock: Arc<RwLock<Option<Vec<url::Url>>>>,
) -> error::Result<()> {
    let value = load_url_list_setting_value(conn, setting_name, std_env_var).await?;
    {
        let mut l = lock.write().await;
        *l = value;
    }
    Ok(())
}

/// Load a required setting value without writing it anywhere.
///
/// Extracted from [`reload_setting`] so callers that store the value in
/// something other than `Arc<RwLock<T>>` (e.g. `AtomicI64`, `AtomicBool`,
/// `ArcSwap<T>`) can reuse the load pipeline.
pub async fn load_setting_value<T: FromStr + DeserializeOwned + Display>(
    conn: &Connection,
    setting_name: &str,
    std_env_var: &str,
    default: T,
    transformer: fn(T) -> T,
) -> error::Result<T> {
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

    Ok(value)
}

pub async fn reload_setting<T: FromStr + DeserializeOwned + Display>(
    conn: &Connection,
    setting_name: &str,
    std_env_var: &str,
    default: T,
    lock: Arc<RwLock<T>>,
    transformer: fn(T) -> T,
) -> error::Result<()> {
    let value = load_setting_value(conn, setting_name, std_env_var, default, transformer).await?;
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
            let active_gauge = prometheus::register_int_gauge!(
                "pool_connections_active",
                "Number of active postgresql connections in the pool"
            )
            .unwrap();
            let idle_gauge = prometheus::register_int_gauge!(
                "pool_connections_idle",
                "Number of idle postgresql connections in the pool"
            )
            .unwrap();
            let max_gauge = prometheus::register_int_gauge!(
                "pool_connections_max",
                "Number of max postgresql connections in the pool"
            )
            .unwrap();

            max_gauge.set(db.options().get_max_connections() as i64);
            loop {
                active_gauge.set(db.size() as i64);
                idle_gauge.set(db.num_idle() as i64);
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });
    }
}

pub async fn monitor_pool_otel(db: &DB) {
    if OTEL_METRICS_ENABLED.load(Ordering::Relaxed) {
        let db = db.clone();
        tokio::spawn(async move {
            let max = db.options().get_max_connections() as i64;
            loop {
                otel_set_db_pool(db.size() as i64, db.num_idle() as i64, max);
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
                if let Err(e) = cleanup_consumed_debounce_batches(&db).await {
                    tracing::error!("Error cleaning up consumed debounce batches: {:?}", e);
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
            verify_license_key(conn.as_sql()).await;
            refetch_license_key_if_invalid(conn).await;
        }
    };

    let enforce_offline_caps_f = async {
        #[cfg(feature = "enterprise")]
        if server_mode && !initial_load {
            if let Some(db) = conn.as_sql() {
                // Cheap: one query for workers active in the last 2 minutes.
                if let Err(e) = windmill_common::ee_oss::enforce_offline_caps(db).await {
                    tracing::error!("Failed to enforce offline license caps: {e:#}");
                }
            }
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

    // Run every hour (10 iterations * 30s = 5 minutes)
    // Check for tokens expiring within 7 days and send alerts
    let check_expiring_tokens_f = async {
        if server_mode && iteration.is_some() && iteration.as_ref().unwrap().should_run(10) {
            if let Some(db) = conn.as_sql() {
                check_expiring_tokens(&db).await;
            }
        }
    };

    // run every hour (120 iterations * 30s = 3600s)
    let cleanup_stale_server_heartbeats_f = async {
        if server_mode && iteration.is_some() && iteration.as_ref().unwrap().should_run(120) {
            if let Some(db) = conn.as_sql() {
                match windmill_api::cleanup_stale_server_heartbeats(db).await {
                    Ok(count) if count > 0 => {
                        tracing::info!(
                            "Deleted {} stale server_heartbeat background_task_state rows",
                            count
                        );
                    }
                    Err(e) => {
                        tracing::error!("Error cleaning up stale server_heartbeat rows: {:?}", e);
                    }
                    _ => {}
                }
            }
        }
    };

    // run every hour (120 iterations * 30s = 3600s)
    let manage_audit_partitions_f = async {
        if server_mode && iteration.is_some() && iteration.as_ref().unwrap().should_run(120) {
            if let Some(db) = conn.as_sql() {
                manage_audit_partitions(&db, audit_log_retention_days().await).await;
            }
        }
    };

    // run every 2 iterations (~20s at the default LISTEN_NEW_EVENTS_INTERVAL_SEC).
    // Enterprise feature: core logic is in `crate::ee` (OSS gets a no-op stub);
    // gated on a valid Enterprise license, mirroring how `audit_log()` itself
    // is license-aware.
    let export_audit_logs_to_object_store_f = async {
        #[cfg(feature = "parquet")]
        if server_mode && iteration.is_some() && iteration.as_ref().unwrap().should_run(2) {
            if let Some(db) = conn.as_sql() {
                if matches!(
                    windmill_common::ee_oss::get_license_plan().await,
                    windmill_common::ee_oss::LicensePlan::Enterprise
                ) {
                    crate::ee_oss::export_audit_logs_to_object_store(&db).await;
                }
            }
        }
    };

    let cleanup_scheduled_job_deletions_f = async {
        #[cfg(feature = "enterprise")]
        if server_mode && !initial_load {
            if let Some(db) = conn.as_sql() {
                cleanup_scheduled_job_deletions(&db).await;
            }
        }
    };

    // Poll git-sync repositories for new commits and pull them into the
    // workspace (repo → Windmill auto-pull). Runs every 2 iterations.
    let git_auto_pull_f = async {
        #[cfg(feature = "private")]
        if server_mode && iteration.is_some() && iteration.as_ref().unwrap().should_run(2) {
            if let Some(db) = conn.as_sql() {
                poll_git_auto_pull(db).await;
            }
        }
    };

    // run every 2 iterations (~20s at the default LISTEN_NEW_EVENTS_INTERVAL_SEC).
    // Enterprise feature: the active `// freshness` backstop lives in
    // windmill-queue's `freshness_watchdog` (`private`); OSS gets a no-op stub.
    // Runtime-gated on an Enterprise license like the audit export above. Safe
    // on concurrent servers — the watchdog claims per-script state rows
    // atomically before pushing.
    let pipeline_freshness_watchdog_f = async {
        if server_mode
            && !*DISABLE_FRESHNESS_WATCHDOG
            && iteration.is_some()
            && iteration.as_ref().unwrap().should_run(2)
        {
            if let Some(db) = conn.as_sql() {
                if matches!(
                    windmill_common::ee_oss::get_license_plan().await,
                    windmill_common::ee_oss::LicensePlan::Enterprise
                ) {
                    windmill_queue::freshness_watchdog::tick(db).await;
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
        enforce_offline_caps_f,
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
        check_expiring_tokens_f,
        cleanup_stale_server_heartbeats_f,
        manage_audit_partitions_f,
        export_audit_logs_to_object_store_f,
        cleanup_scheduled_job_deletions_f,
        git_auto_pull_f,
        pipeline_freshness_watchdog_f,
    );
}

/// Advisory lock id ensuring only one server replica runs the git auto-pull
/// poll at a time (adjacent to RESTART_LOCK_ID used for restart coordination).
#[cfg(feature = "private")]
const GIT_AUTO_PULL_LOCK_ID: i64 = 737_483_921;

/// Poll every git-sync repository with auto-pull enabled and enqueue a pull when
/// the tracked branch has new commits (repo → Windmill direction).
///
/// Runs on a single replica at a time (advisory lock) and only on
/// Enterprise-licensed instances. Detection is `git ls-remote`; GitHub-App
/// repositories are skipped here and sync via webhooks instead (phase 2).
#[cfg(feature = "private")]
pub async fn poll_git_auto_pull(db: &Pool<Postgres>) {
    use windmill_common::ee_oss::{get_license_plan, LicensePlan};

    if !matches!(get_license_plan().await, LicensePlan::Enterprise) {
        return;
    }

    let mut lock_conn = match db.acquire().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("git auto-pull: failed to acquire connection: {e:#}");
            return;
        }
    };
    let locked: bool = match sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(GIT_AUTO_PULL_LOCK_ID)
        .fetch_one(&mut *lock_conn)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("git auto-pull: advisory lock failed: {e:#}");
            return;
        }
    };
    if !locked {
        // Another replica is already polling this tick.
        return;
    }

    if let Err(e) = poll_git_auto_pull_inner(db).await {
        tracing::error!("git auto-pull: poll error: {e:#}");
    }

    if let Err(e) = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(GIT_AUTO_PULL_LOCK_ID)
        .execute(&mut *lock_conn)
        .await
    {
        tracing::error!("git auto-pull: advisory unlock failed: {e:#}");
    }
}

#[cfg(feature = "private")]
lazy_static::lazy_static! {
    /// Last auto-pull poll time (unix secs) per `workspace|repo_path`, so each repo
    /// is only probed once per its effective interval instead of every ~60s tick.
    /// Bounded by the number of auto-pull repos; stale entries for removed repos are
    /// harmless.
    static ref AUTO_PULL_LAST_POLL: std::sync::Mutex<std::collections::HashMap<String, i64>> =
        std::sync::Mutex::new(std::collections::HashMap::new());
}

/// Slack (seconds) subtracted from the effective interval so a repo whose interval
/// equals the ~60s tick isn't skipped by tick jitter.
#[cfg(feature = "private")]
const AUTO_PULL_POLL_SLACK_S: i64 = 30;

#[cfg(feature = "private")]
async fn poll_git_auto_pull_inner(db: &Pool<Postgres>) -> error::Result<()> {
    use windmill_common::workspaces::{AutoPullMode, WorkspaceGitSyncSettings};

    // Join `workspace` and skip deleted/archived ones: their `workspace_settings`
    // rows persist (archive is a soft delete, and change_workspace_id leaves the old
    // id as an archived shell), so an auto-pull repo would otherwise keep polling and
    // deploying into a dead workspace.
    let rows = sqlx::query!(
        r#"SELECT ws.workspace_id, ws.git_sync
           FROM workspace_settings ws
           JOIN workspace w ON w.id = ws.workspace_id
           WHERE NOT w.deleted
             AND ws.git_sync IS NOT NULL
             AND ws.git_sync->'repositories' @> '[{"auto_pull": {"enabled": true}}]'::jsonb"#
    )
    .fetch_all(db)
    .await?;

    for row in rows {
        let Some(git_sync) = row.git_sync else {
            continue;
        };
        let settings: WorkspaceGitSyncSettings = match serde_json::from_value(git_sync) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    "git auto-pull: invalid git_sync settings for workspace {}: {e}",
                    row.workspace_id
                );
                continue;
            }
        };

        for repo in &settings.repositories {
            let Some(auto_pull) = &repo.auto_pull else {
                continue;
            };
            if !auto_pull.enabled || auto_pull.mode == AutoPullMode::Webhook {
                continue;
            }

            // Honor the repo's effective poll interval (relaxed to ~10 min when a
            // webhook is live) instead of probing every ~60s tick.
            let interval_s = auto_pull.effective_poll_interval_s() as i64;
            let poll_key = format!("{}|{}", row.workspace_id, repo.git_repo_resource_path);
            let now = chrono::Utc::now().timestamp();
            {
                let mut last = AUTO_PULL_LAST_POLL.lock().unwrap();
                if let Some(&t) = last.get(&poll_key) {
                    if now - t < interval_s - AUTO_PULL_POLL_SLACK_S {
                        continue;
                    }
                }
                last.insert(poll_key, now);
            }

            let head = match windmill_store::resources::get_git_repo_head_for_autopull(
                db,
                &row.workspace_id,
                &repo.git_repo_resource_path,
            )
            .await
            {
                Ok(Some(h)) => Ok(Some(h)),
                // App-backed repos store a tokenless URL, so the ls-remote head
                // check can't authenticate and returns None. Poll the head over
                // the GitHub API with a minted installation token instead. This
                // is the polling fallback/safety-net for auto- and polling-mode
                // app repos whose webhook isn't live (unreachable instance,
                // missing permission, or a dropped delivery). `webhook`-mode
                // repos are skipped above and stay webhook-only.
                Ok(None) => {
                    #[cfg(feature = "enterprise")]
                    {
                        windmill_common::git_sync_ee::get_app_repo_head_for_autopull(
                            db,
                            &row.workspace_id,
                            &repo.git_repo_resource_path,
                        )
                        .await
                    }
                    #[cfg(not(feature = "enterprise"))]
                    {
                        Ok(None)
                    }
                }
                Err(e) => Err(e),
            };

            match head {
                Ok(Some((git_ref, sha))) => {
                    // Shared reconcile (also used by the webhook receiver):
                    // checks should_pull, enqueues, and records status/failure.
                    if let Err(e) = windmill_git_sync::reconcile_and_enqueue_pull(
                        db,
                        &row.workspace_id,
                        repo,
                        &git_ref,
                        &sha,
                        None,
                    )
                    .await
                    {
                        tracing::warn!(
                            "git auto-pull: reconcile failed for {}/{}: {e:#}",
                            row.workspace_id,
                            repo.git_repo_resource_path
                        );
                    }

                    // Parent-managed fork sync: list the fork branches' heads in
                    // the same poll tick (one extra ls-remote / API call) and
                    // route each into its fork workspace. Needs the concrete
                    // tracked branch name to scope `wm-fork/<branch>/*`; a
                    // branch-less resource resolves its default branch via
                    // `ls-remote --symref`, so "HEAD" only remains when that
                    // resolution failed.
                    if auto_pull.sync_forks && git_ref != "HEAD" {
                        poll_git_fork_branches(
                            db,
                            &row.workspace_id,
                            &repo.git_repo_resource_path,
                            &git_ref,
                        )
                        .await;
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    windmill_git_sync::record_auto_pull_failure(
                        db,
                        &row.workspace_id,
                        &repo.git_repo_resource_path,
                        &auto_pull.last_synced_sha,
                        format!("head check failed: {e}"),
                    )
                    .await;
                }
            }
        }
    }

    Ok(())
}

/// Poll-side half of parent-managed fork sync (`sync_forks`): list every
/// `wm-fork/<base_branch>/*` head of the parent's repo and reconcile each into
/// its fork workspace. Failures are logged, not recorded in the parent's pull
/// status — the parent's own sync state is unaffected by a fork's.
#[cfg(feature = "private")]
async fn poll_git_fork_branches(
    db: &Pool<Postgres>,
    parent_w_id: &str,
    repo_path: &str,
    base_branch: &str,
) {
    // Dev-workspace children sync with their environment-label branch (`dev`/
    // `staging`) rather than the `wm-fork/**` pattern, so their branches must be
    // listed explicitly. A label equal to the tracked branch is excluded — the
    // parent's own head check covers it.
    let label_refs: Vec<String> = match sqlx::query_scalar!(
        r#"SELECT DISTINCT COALESCE(dev_workspace_label, 'dev') as "label!"
           FROM workspace
           WHERE parent_workspace_id = $1 AND is_dev_workspace AND NOT deleted"#,
        parent_w_id
    )
    .fetch_all(db)
    .await
    {
        Ok(labels) => labels.into_iter().filter(|l| l != base_branch).collect(),
        Err(e) => {
            tracing::warn!(
                "git fork sync: failed to list dev-workspace labels for {parent_w_id}: {e:#}"
            );
            Vec::new()
        }
    };

    let fork_heads = match windmill_store::resources::get_git_repo_fork_heads_for_autopull(
        db,
        parent_w_id,
        repo_path,
        base_branch,
        &label_refs,
    )
    .await
    {
        Ok(Some(heads)) => Ok(heads),
        // App-backed repos list fork refs over the GitHub API, mirroring the
        // parent head check's fallback.
        Ok(None) => {
            #[cfg(feature = "enterprise")]
            {
                windmill_common::git_sync_ee::get_app_repo_fork_heads_for_autopull(
                    db,
                    parent_w_id,
                    repo_path,
                    base_branch,
                    &label_refs,
                )
                .await
                .map(|heads| heads.unwrap_or_default())
            }
            #[cfg(not(feature = "enterprise"))]
            {
                Ok(Vec::new())
            }
        }
        Err(e) => Err(e),
    };
    match fork_heads {
        Ok(heads) => {
            for (branch, sha) in heads {
                if let Err(e) = windmill_git_sync::reconcile_fork_branch_pull(
                    db,
                    parent_w_id,
                    repo_path,
                    &branch,
                    base_branch,
                    &sha,
                )
                .await
                {
                    tracing::warn!(
                        "git fork sync: reconcile failed for {parent_w_id}/{repo_path} branch {branch}: {e:#}"
                    );
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                "git fork sync: fork branch listing failed for {parent_w_id}/{repo_path}: {e:#}"
            );
        }
    }
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

    if metrics_enabled || save_metrics || OTEL_METRICS_ENABLED.load(Ordering::Relaxed) {
        let queue_counts = windmill_common::queue::get_queue_counts(db).await;

        #[cfg(feature = "prometheus")]
        if metrics_enabled {
            for q in QUEUE_COUNT_TAGS.read().await.iter() {
                if queue_counts.get(q).is_none() {
                    (*QUEUE_COUNT).with_label_values(&[q]).set(0);
                }
            }
        }

        let otel_enabled = OTEL_METRICS_ENABLED.load(Ordering::Relaxed);

        if otel_enabled {
            for q in OTEL_QUEUE_COUNT_TAGS.read().await.iter() {
                if queue_counts.get(q).is_none() {
                    otel_set_queue_count(q, 0);
                }
            }
        }

        #[allow(unused_mut)]
        let mut tags_to_watch = vec![];
        #[allow(unused_mut)]
        let mut otel_tags_to_watch = vec![];
        for q in queue_counts {
            let count = q.1;
            let tag = q.0;

            #[cfg(feature = "prometheus")]
            if metrics_enabled {
                let metric = (*QUEUE_COUNT).with_label_values(&[&tag]);
                metric.set(count as i64);
                tags_to_watch.push(tag.to_string());
            }

            if otel_enabled {
                otel_tags_to_watch.push(tag.to_string());
            }
            otel_set_queue_count(&tag, count as i64);

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
        if otel_enabled {
            let mut w = OTEL_QUEUE_COUNT_TAGS.write().await;
            *w = otel_tags_to_watch;
        }

        // Single DB query for running counts, shared by Prometheus and OTel
        let otel_running = otel_enabled;
        #[cfg(feature = "prometheus")]
        let need_running_counts = metrics_enabled || otel_running;
        #[cfg(not(feature = "prometheus"))]
        let need_running_counts = otel_running;

        if need_running_counts {
            let queue_running_counts = windmill_common::queue::get_queue_running_counts(db).await;

            #[cfg(feature = "prometheus")]
            if metrics_enabled {
                for q in QUEUE_RUNNING_COUNT_TAGS.read().await.iter() {
                    if queue_running_counts.get(q).is_none() {
                        (*QUEUE_RUNNING_COUNT).with_label_values(&[q]).set(0);
                    }
                }
            }

            if otel_running {
                for q in OTEL_QUEUE_RUNNING_COUNT_TAGS.read().await.iter() {
                    if queue_running_counts.get(q).is_none() {
                        otel_set_queue_running_count(q, 0);
                    }
                }
            }

            #[allow(unused_mut, unused_variables)]
            let mut running_tags_to_watch: Vec<String> = vec![];
            #[allow(unused_mut, unused_variables)]
            let mut otel_running_tags_to_watch: Vec<String> = vec![];
            for (tag, count) in &queue_running_counts {
                #[cfg(feature = "prometheus")]
                if metrics_enabled {
                    let metric = (*QUEUE_RUNNING_COUNT).with_label_values(&[tag]);
                    metric.set(*count as i64);
                    running_tags_to_watch.push(tag.to_string());
                }

                if otel_running {
                    otel_set_queue_running_count(tag, *count as i64);
                    otel_running_tags_to_watch.push(tag.to_string());
                }
            }

            #[cfg(feature = "prometheus")]
            if metrics_enabled {
                let mut w = QUEUE_RUNNING_COUNT_TAGS.write().await;
                *w = running_tags_to_watch;
            }
            if otel_running {
                let mut w = OTEL_QUEUE_RUNNING_COUNT_TAGS.write().await;
                *w = otel_running_tags_to_watch;
            }
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
        tracing::info!("Reloading smtp config...");
        SMTP_CONFIG.store(std::sync::Arc::new(smtp_config.unwrap()));
    }
}

pub async fn reload_indexer_config(db: &Pool<Postgres>) {
    let indexer_config = load_indexer_config(&db).await;
    if let Err(e) = indexer_config {
        tracing::error!("Error reloading indexer config: {:?}", e)
    } else {
        tracing::info!("Reloading indexer config...");
        INDEXER_CONFIG.store(std::sync::Arc::new(indexer_config.unwrap()));
    }
}

pub async fn reload_worker_config(db: &DB, tx: KillpillSender, kill_if_change: bool) {
    let config = load_worker_config(db, tx.clone()).await;
    if let Err(e) = config {
        tracing::error!("Error reloading worker config: {:?}", e)
    } else {
        let wc = WORKER_CONFIG.load();
        let config = config.unwrap();
        let has_dedicated = config.dedicated_worker.is_some()
            || config
                .dedicated_workers
                .as_ref()
                .is_some_and(|dws| !dws.is_empty());
        if **wc != config || has_dedicated {
            if kill_if_change {
                if has_dedicated
                    || wc.dedicated_worker != config.dedicated_worker
                    || wc.dedicated_workers != config.dedicated_workers
                {
                    tracing::info!("Dedicated worker config changed, sending killpill. Expecting to be restarted by supervisor.");
                    let _ = tx.send();
                }

                if wc.init_bash != config.init_bash {
                    tracing::info!("Init bash config changed, sending killpill. Expecting to be restarted by supervisor.");
                    let _ = tx.send();
                }

                if wc.cache_clear != config.cache_clear {
                    tracing::info!("Cache clear changed, sending killpill. Expecting to be restarted by supervisor.");
                    let _ = tx.send();
                    tracing::info!("Waiting 5 seconds to allow others workers to start potential jobs that depend on a potential shared cache volume");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    if let Err(e) = windmill_worker::common::clean_cache().await {
                        tracing::error!("Error cleaning the cache: {e:#}");
                    }
                }

                if wc.periodic_script_bash != config.periodic_script_bash {
                    tracing::info!("Periodic script bash config changed, sending killpill. Expecting to be restarted by supervisor.");
                    let _ = tx.send();
                }

                if wc.periodic_script_interval_seconds != config.periodic_script_interval_seconds {
                    tracing::info!("Periodic script interval config changed, sending killpill. Expecting to be restarted by supervisor.");
                    let _ = tx.send();
                }

                if wc.native_mode != config.native_mode {
                    tracing::info!("Native mode config changed, sending killpill. Expecting to be restarted by supervisor.");
                    let _ = tx.send();
                }
            }
            drop(wc);

            tracing::info!("Reloading worker config...");
            store_suspended_pull_query(&config).await;
            store_pull_query(&config).await;
            WORKER_CONFIG.store(std::sync::Arc::new(config));
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
    BASE_URL.store(std::sync::Arc::new(base_url.clone()));
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
            let clients = windmill_api::oauth2_oss::build_oauth_clients(&base_url, oauths, db).await
                .map_err(|e| tracing::error!("Error building oauth clients (is the oauth.json mounted and in correct format? Use '{}' as minimal oauth.json): {}", "{}", e))
                .unwrap();
            windmill_api::OAUTH_CLIENTS.store(std::sync::Arc::new(clients));
        }
    }

    IS_SECURE.store(is_secure, Ordering::Relaxed);

    #[cfg(feature = "enterprise")]
    {
        crate::ee_oss::verify_license_key(conn.as_sql()).await;
    }

    Ok(())
}

async fn stale_job_cancellation(db: &Pool<Postgres>) {
    if let Some(threshold) = *STALE_JOB_THRESHOLD_MINUTES {
        let stale_jobs = sqlx::query!(
            "SELECT v2_job_queue.id, v2_job.tag, v2_job_queue.scheduled_for, v2_job_queue.workspace_id FROM v2_job_queue LEFT JOIN v2_job ON v2_job_queue.id = v2_job.id WHERE running = false AND scheduled_for < now() - ($1 || ' minutes')::interval AND v2_job.trigger_kind IS DISTINCT FROM 'schedule'::job_trigger_kind",
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
                    AND q.suspend_until IS NULL
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

        otel_incr_zombie_restart_count(restarted.len() as u64);

        let base_url = (**BASE_URL.load()).clone();
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
    AND running = true AND (ping IS NULL OR ping < now() - ('60 seconds')::interval) AND same_worker = true AND worker IS NOT NULL AND v2_job_queue.suspend_until IS NULL GROUP BY worker",
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
             AND q.running = true AND j.kind NOT IN ('flow', 'flowpreview', 'flownode', 'singlestepflow') AND j.same_worker = false AND q.suspend_until IS NULL",
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

    otel_incr_zombie_delete_count(timeouts.len() as u64);

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
                error::Error::ExecutionErr(error_message.clone()),
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

            // If handle_job_error failed (e.g. schedule push failure rolled back the tx),
            // the job is still in the queue. Force-complete it to prevent infinite zombie loops.
            if let Err(e) = force_complete_zombie_job(db, &job_id, &error_message).await {
                tracing::error!("Failed to force-complete zombie job {}: {e:#}", job_id);
            }
        }
    }
}

/// Force-complete a zombie job that handle_job_error failed to complete.
/// This is a minimal fallback: it inserts a failed completed job and deletes
/// from the queue in a single transaction, without schedule pushing or
/// error handler logic that could cause the completion to fail.
async fn force_complete_zombie_job(
    db: &Pool<Postgres>,
    job_id: &Uuid,
    error_message: &str,
) -> error::Result<()> {
    let still_queued = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM v2_job_queue WHERE id = $1)",
        job_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);

    if !still_queued {
        return Ok(());
    }

    tracing::error!(
        "Zombie job {job_id} was not completed by handle_job_error, force-completing it"
    );

    let error_value = serde_json::json!({
        "message": error_message,
        "name": "ExecutionErr",
    });

    let mut tx = db.begin().await?;

    sqlx::query!(
        "INSERT INTO v2_job_completed
            (workspace_id, id, started_at, duration_ms, result, memory_peak, status, worker)
        SELECT q.workspace_id, q.id, q.started_at,
            COALESCE((EXTRACT('epoch' FROM now()) - EXTRACT('epoch' FROM COALESCE(q.started_at, now()))) * 1000, 0)::bigint,
            $2::jsonb, r.memory_peak, 'failure'::job_status, q.worker
        FROM v2_job_queue q
        LEFT JOIN v2_job_runtime r ON r.id = q.id
        WHERE q.id = $1
        ON CONFLICT (id) DO UPDATE SET status = 'failure', result = $2::jsonb",
        job_id,
        error_value,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!("DELETE FROM v2_job_queue WHERE id = $1", job_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    tracing::info!("Force-completed zombie job {job_id}");
    Ok(())
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
            COALESCE(s.flow_status, s.workflow_as_code_status) AS "flow_status: Box<str>", r.ping AS last_ping, j.same_worker AS "same_worker?",
            q.worker AS "worker?",
            wp.ping_at AS "worker_last_ping?",
            wp.memory_usage AS "worker_memory_usage?",
            wp.wm_memory_usage AS "worker_wm_memory_usage?",
            wp.memory AS "worker_memory_total?",
            wp.worker_group AS "worker_group?",
            wp.wm_version AS "worker_version?",
            wp.current_job_id AS "worker_current_job_id?",
            wp.worker_instance AS "worker_instance?"
        FROM v2_job_queue q JOIN v2_job j USING (id) LEFT JOIN v2_job_runtime r USING (id) LEFT JOIN v2_job_status s USING (id)
            LEFT JOIN worker_ping wp ON wp.worker = q.worker
        WHERE q.running = true AND q.suspend = 0 AND q.suspend_until IS null AND q.scheduled_for <= now()
            AND (j.kind = 'flow' OR j.kind = 'flowpreview' OR j.kind = 'flownode' OR j.kind = 'singlestepflow')
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
            let base_url = BASE_URL.load();
            let workspace_id = flow.workspace_id.clone();

            let fmt_mb = |b: i64| format!("{:.1} MB", b as f64 / 1024.0 / 1024.0);
            let worker_ping_stale = flow
                .worker_last_ping
                .map(|wp| (now - wp).num_seconds() > 60);
            // Zombie flows between steps are, in practice, almost always the worker
            // getting OOM-killed mid state-transition. Score the memory signal at
            // the worker's last ping as a fraction of the cgroup limit, taking the
            // larger of the cgroup-wide reading (`worker_memory_usage`) and the
            // windmill process's jemalloc resident (`worker_wm_memory_usage`) — if
            // only one is present, that value wins; if both are present, the larger
            // is the more conservative (higher-signal) choice. If the pod restarted
            // in-place, a replacement worker process comes up under a new windmill
            // worker name; this flow's recorded worker name still points at the
            // dead process whose last ping can be under 60s old, and the memory
            // signal is what lets us catch that window.
            let memory_pct: Option<f64> = flow.worker_memory_total.and_then(|total| {
                if total <= 0 {
                    return None;
                }
                let used = flow.worker_memory_usage.max(flow.worker_wm_memory_usage)?;
                Some(used as f64 / total as f64)
            });
            let oom_strong = memory_pct.is_some_and(|p| p >= 0.85);
            let oom_moderate = memory_pct.is_some_and(|p| p >= 0.60);
            let mem_pct_str = memory_pct
                .map(|p| format!("{:.1}% of container limit", (p * 100.0).min(100.0)))
                .unwrap_or_else(|| "memory unknown at last ping".to_string());
            let worker_info = if let Some(worker_name) = flow.worker.as_deref() {
                let mut s = format!("\nWorker handling the flow: {worker_name}");
                match (flow.worker_group.as_deref(), flow.worker_version.as_deref()) {
                    (Some(g), Some(v)) => s.push_str(&format!(" (group: {g}, version: {v})")),
                    (Some(g), None) => s.push_str(&format!(" (group: {g})")),
                    (None, Some(v)) => s.push_str(&format!(" (version: {v})")),
                    (None, None) => {}
                }
                if let Some(wp) = flow.worker_last_ping {
                    let age = (now - wp).num_seconds();
                    let status: String = match (
                        worker_ping_stale.unwrap_or(false),
                        oom_strong,
                        oom_moderate,
                    ) {
                        (true, true, _) => format!(
                            "OOM-KILLED — {mem_pct_str} at last ping, worker then stopped pinging"
                        ),
                        (true, false, true) => format!(
                            "LIKELY OOM-KILLED — {mem_pct_str} at last ping, worker then stopped pinging"
                        ),
                        (true, false, false) => {
                            "WORKER DIED — most likely OOM-killed (no strong memory evidence captured at last ping); less likely: crash, host failure, or network partition".to_string()
                        }
                        (false, true, _) => format!(
                            "OOM-KILLED — {mem_pct_str} at last ping (a replacement worker process may have started in the same pod under a new windmill worker name; this alert references the dead process's record, which pinged just before being killed)"
                        ),
                        (false, false, true) => format!(
                            "LIKELY OOM-KILLED — {mem_pct_str} at last ping (a replacement worker process may have started in the same pod under a new windmill worker name)"
                        ),
                        (false, false, false) => {
                            "worker still pinging with healthy memory — likely deadlocked or blocking on the state transition".to_string()
                        }
                    };
                    s.push_str(&format!("\nWorker last ping: {wp} ({age}s ago) — {status}"));
                    if let Some(cjid) = flow.worker_current_job_id {
                        if cjid != flow.id {
                            s.push_str(&format!("\nWorker has since moved on to job {cjid}"));
                        }
                    }
                } else {
                    s.push_str("\nWorker last ping: unknown (no worker_ping record)");
                }
                let total_suffix = flow
                    .worker_memory_total
                    .map(|t| format!(" (total available: {})", fmt_mb(t)))
                    .unwrap_or_default();
                match (flow.worker_memory_usage, flow.worker_wm_memory_usage) {
                    (Some(host), Some(wm)) => s.push_str(&format!(
                        "\nWorker memory at last ping: host={}, wm process={}{}",
                        fmt_mb(host),
                        fmt_mb(wm),
                        total_suffix
                    )),
                    (Some(host), None) => s.push_str(&format!(
                        "\nWorker memory at last ping: host={}{}",
                        fmt_mb(host),
                        total_suffix
                    )),
                    (None, Some(wm)) => s.push_str(&format!(
                        "\nWorker memory at last ping: wm process={}{}",
                        fmt_mb(wm),
                        total_suffix
                    )),
                    (None, None) => {
                        if let Some(total) = flow.worker_memory_total {
                            s.push_str(&format!("\nWorker total memory: {}", fmt_mb(total)));
                        }
                    }
                }
                s
            } else {
                "\nWorker handling the flow: unknown (no worker recorded on v2_job_queue)"
                    .to_string()
            };

            let hint: String = match (worker_ping_stale, oom_moderate) {
                (Some(_), true) => format!(
                    "\nThis is almost certainly an OOM-kill: container memory at the worker's last ping was at {mem_pct_str}. Raise the worker memory limit (e.g. k8s `resources.limits.memory`) or reduce per-flow memory usage. Confirm via pod restart count (`kubectl describe pod` / `kube_pod_container_status_last_terminated_reason`)."
                ),
                (Some(true), false) => {
                    "\nWorker stopped pinging and its last memory snapshot did not look high — in practice the overwhelmingly common cause here is still OOM-kill (memory may have spiked between the last ping and the kill, or never been reported). First check pod restart count (`kubectl describe pod` / `kube_pod_container_status_last_terminated_reason`). Less likely: host failure, network partition, or a panic — check worker logs / k8s events around the last ping time.".to_string()
                }
                (Some(false), false) => {
                    "\nWorker is still pinging and memory looked healthy at its last ping — most likely a deadlock or blocking call during the state transition. Capture a stack trace (e.g. via SIGQUIT) from the worker process. As a sanity check, also verify pod restart count in case a replacement worker process in the same pod has silently taken over.".to_string()
                }
                (None, _) => String::new(),
            };

            let service_logs_info = match (flow.worker_instance.as_deref(), flow.worker_last_ping) {
                (Some(host), Some(wlp)) => {
                    let after = (wlp - chrono::Duration::seconds(90))
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                    let before = (wlp + chrono::Duration::seconds(30))
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                    let log_files_result = sqlx::query!(
                        "SELECT file_path, log_ts, ok_lines, err_lines
                         FROM log_file
                         WHERE hostname = $1
                           AND log_ts BETWEEN $2::timestamp - interval '90 seconds' AND $2::timestamp + interval '30 seconds'
                         ORDER BY log_ts DESC
                         LIMIT 10",
                        host,
                        wlp.naive_utc(),
                    )
                    .fetch_all(db)
                    .await;
                    let log_files = match log_files_result {
                        Ok(rows) => rows,
                        Err(e) => {
                            tracing::warn!(
                                "failed to query log_file for hanging-flow diagnostics (host={host}): {e:#}"
                            );
                            Vec::new()
                        }
                    };
                    if log_files.is_empty() {
                        format!(
                            "\nService logs: no log_file rows found for hostname '{host}' between {after} and {before}. If service log collection is enabled (requires S3/parquet), try /api/service_logs/list_files?after={after}&before={before}."
                        )
                    } else {
                        let listed = log_files
                            .iter()
                            .map(|f| {
                                format!(
                                    "  - {} (log_ts: {}, ok_lines: {}, err_lines: {}) — GET /api/service_logs/get_log_file/{}/{}",
                                    f.file_path,
                                    f.log_ts,
                                    f.ok_lines.unwrap_or(0),
                                    f.err_lines.unwrap_or(0),
                                    host,
                                    f.file_path,
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        format!("\nService logs for worker instance '{host}' around last ping ({after} to {before}) (download URLs require S3/parquet service log collection):\n{listed}")
                    }
                }
                _ => String::new(),
            };

            let reason = format!(
                "{} was hanging in between 2 steps. Last ping: {last_ping:?} (now: {now}){worker_info}{hint}{service_logs_info}",
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

    let previous = HUB_BASE_URL.load();
    if server_mode {
        #[cfg(feature = "embedding")]
        if let Some(db) = conn.as_sql() {
            if **previous != base_url {
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
    drop(previous);
    HUB_BASE_URL.store(std::sync::Arc::new(base_url));

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

    CRITICAL_ERROR_CHANNELS.store(std::sync::Arc::new(critical_error_channels));

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

    APP_WORKSPACED_ROUTE.store(ws_route, Ordering::Relaxed);
    Ok(())
}

pub async fn reload_http_route_workspaced_route_setting(conn: &DB) -> error::Result<()> {
    let http_route_workspaced_route =
        load_value_from_global_settings(conn, HTTP_ROUTE_WORKSPACED_ROUTE_SETTING).await?;

    let ws_route = match http_route_workspaced_route {
        Some(serde_json::Value::Bool(ws_route)) => ws_route,
        None => false,
        _ => {
            tracing::error!(
                "Expected {} to be a boolean got: {:?}. Defaulting to false",
                HTTP_ROUTE_WORKSPACED_ROUTE_SETTING,
                http_route_workspaced_route
            );
            false
        }
    };

    let previous = HTTP_ROUTE_WORKSPACED_ROUTE.swap(ws_route, Ordering::Relaxed);
    if previous != ws_route {
        // Bump the HTTP trigger version so the route cache is rebuilt with
        // the updated workspaced_route behavior on the next request.
        sqlx::query!("SELECT nextval('http_trigger_version_seq')")
            .fetch_one(conn)
            .await?;
    }
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

    CRITICAL_ALERTS_ON_DB_OVERSIZE.store(std::sync::Arc::new(db_oversize));

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

    JWT_SECRET.store(std::sync::Arc::new(jwt_secret));

    // The debug signing key is derived from JWT_SECRET, so re-derive it here so
    // rotation propagates to /api/debug/* signing without requiring a restart.
    windmill_api::reload_debug_signing_key().await;

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

/// GC for claim-based debounce batches: once a batch row has been consumed (its
/// args accumulated into some survivor's run), it only lingers to let a later-pulled
/// survivor of the same batch tell "already consumed" from "never batched". A generous
/// grace period (>> any debounce window) makes that decision safe; after it, the rows
/// are dead weight. A re-pulled survivor whose row was GC'd correctly falls back to its
/// own (already-accumulated, persisted) args, so the grace period is not correctness-
/// critical.
async fn cleanup_consumed_debounce_batches(db: &DB) -> error::Result<()> {
    // Only reclaim a consumed row once its job has LEFT the queue. A consumed sibling
    // (its contribution already accumulated by another survivor) can sit queued well
    // past any time-based grace under a concurrency limit / worker backlog; removing its
    // row while still queued would make its eventual pull treat it as never-batched and
    // re-run its item (a duplicate). Keeping the row until the job is no longer queued
    // guarantees that pull still sees "already consumed" and runs empty. The age floor
    // is just a safety margin on top.
    let deleted = sqlx::query_scalar!(
        "WITH del AS (
            DELETE FROM v2_job_debounce_batch
            WHERE consumed_at IS NOT NULL
              AND consumed_at < now() - interval '10 minutes'
              AND id NOT IN (SELECT id FROM v2_job_queue)
            RETURNING 1
        ) SELECT count(*) FROM del"
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    if deleted > 0 {
        tracing::info!("Cleaned up {deleted} consumed debounce batch rows");
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

// Per-statement cap keeps each delete short and lock-light; the per-cycle batch
// cap bounds total work per monitor iteration so monitor_db stays responsive.
// A large backlog drains across several iterations rather than one long delete.
//
// These sweeps anti-join the whole table to find orphans, so their cost tracks the heap's
// physical size. job_perms / job_result_stream_v2 are high-churn (one row per job, deleted
// here), so their bloat — not the query shape — is what makes the sweep slow. These sweeps run
// every monitor cycle, but the bulk vacuuming_tables() runs only ~hourly, so dead tuples pile
// up between bulk vacuums; each sweep VACUUMs its own table right after deleting (see below) to
// keep the heap near the live working set. The outer `ctid IN (SELECT ... LIMIT)` is
// deliberate: a `job_id IN (...)` rewrite adds a second scan/probe for the delete and
// benchmarks slower, so don't "simplify" it.
const ORPHAN_CLEANUP_BATCH_SIZE: u64 = 100_000;
const ORPHAN_CLEANUP_MAX_BATCHES: usize = 10;

// Reclaim the dead tuples a sweep just created so the next sweep's anti-join scans a lean heap
// instead of a bloated one. Plain VACUUM (not FULL) only takes SHARE UPDATE EXCLUSIVE, so
// concurrent reads/writes (every job create touches job_perms) keep running, and the visibility
// map lets it skip unchanged pages so repeated runs are cheap. SKIP_LOCKED means HA replicas
// don't pile up: one vacuums, the rest skip rather than queue behind it.
async fn vacuum_after_sweep(db: &DB, table: &str) {
    if let Err(e) = sqlx::query(&format!("VACUUM (SKIP_LOCKED) {table}"))
        .execute(db)
        .await
    {
        tracing::warn!("Error vacuuming {table} after orphan cleanup: {e:?}");
    }
}

async fn cleanup_job_perms_orphaned(db: &DB) -> error::Result<()> {
    let mut total: u64 = 0;
    for _ in 0..ORPHAN_CLEANUP_MAX_BATCHES {
        let count = sqlx::query!(
            "DELETE FROM job_perms
             WHERE ctid IN (
                 SELECT jp.ctid FROM job_perms jp
                 WHERE NOT EXISTS (SELECT 1 FROM v2_job_queue q WHERE q.id = jp.job_id)
                 LIMIT 100000
             )"
        )
        .execute(db)
        .await?
        .rows_affected();
        total += count;
        if count < ORPHAN_CLEANUP_BATCH_SIZE {
            break;
        }
    }

    if total > 0 {
        tracing::info!("Cleaned up {total} orphaned job_perms rows");
        vacuum_after_sweep(db, "job_perms").await;
    }
    Ok(())
}

async fn cleanup_job_result_stream_orphaned_jobs(db: &DB) -> error::Result<()> {
    let mut total: u64 = 0;
    for _ in 0..ORPHAN_CLEANUP_MAX_BATCHES {
        let count = sqlx::query!(
            "DELETE FROM job_result_stream_v2
             WHERE ctid IN (
                 SELECT jrs.ctid FROM job_result_stream_v2 jrs
                 WHERE NOT EXISTS (SELECT 1 FROM v2_job_queue q WHERE q.id = jrs.job_id)
                   AND NOT EXISTS (
                       SELECT 1 FROM v2_job_completed c
                       WHERE c.id = jrs.job_id
                         AND c.completed_at > NOW() - INTERVAL '60 seconds'
                   )
                 LIMIT 100000
             )",
        )
        .execute(db)
        .await?
        .rows_affected();
        total += count;
        if count < ORPHAN_CLEANUP_BATCH_SIZE {
            break;
        }
    }

    if total > 0 {
        tracing::info!("Cleaned up {total} orphaned job_result_stream_v2 rows");
        vacuum_after_sweep(db, "job_result_stream_v2").await;
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

async fn audit_log_retention_days() -> i64 {
    let v = AUDIT_LOG_RETENTION_DAYS.load(std::sync::atomic::Ordering::Relaxed);
    if v > 0 {
        v
    } else if cfg!(feature = "enterprise") {
        365
    } else {
        14
    }
}

/// Number of days ahead (including today) for which an audit partition must
/// always exist. A missing partition in this window means audit inserts fail
/// once that date is reached — and because some callers (notably login) write
/// the audit row in the same transaction as their own work, that failure
/// poisons the whole transaction, so a missing partition is a hard outage, not
/// just a dropped audit row.
const AUDIT_PARTITION_LOOKAHEAD_DAYS: i64 = 3;

async fn manage_audit_partitions(db: &DB, retention_days: i64) {
    let today = chrono::Utc::now().date_naive();

    // Create partitions for today and the next few days
    for days_ahead in 0..=AUDIT_PARTITION_LOOKAHEAD_DAYS {
        let date = today + chrono::Duration::days(days_ahead);
        let next_date = date + chrono::Duration::days(1);
        let partition_name = format!("audit_{}", date.format("%Y%m%d"));
        let quoted_name = format!("\"{}\"", partition_name.replace('"', "\"\""));
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {quoted_name} PARTITION OF audit_partitioned \
             FOR VALUES FROM ('{date}') TO ('{next_date}')"
        );
        if let Err(e) = sqlx::query(&sql).execute(db).await {
            if !e.to_string().contains("already exists") {
                tracing::error!("Error creating audit partition {partition_name}: {e:?}");
            }
        }
    }

    let partitions = sqlx::query_scalar::<_, String>(
        "SELECT c.relname::text \
         FROM pg_inherits i \
         JOIN pg_class c ON c.oid = i.inhrelid \
         WHERE i.inhparent = 'audit_partitioned'::regclass",
    )
    .fetch_all(db)
    .await;

    let partitions = match partitions {
        Ok(partitions) => partitions,
        Err(e) => {
            tracing::error!("Error listing audit partitions: {e:?}");
            return;
        }
    };

    // Verify the lookahead window is actually covered. If a create above failed
    // (or this loop has not run for several days), alert loudly instead of
    // letting it surface days later as failed audit inserts and broken logins.
    let existing: std::collections::HashSet<&str> = partitions.iter().map(|s| s.as_str()).collect();
    let missing: Vec<String> = (0..=AUDIT_PARTITION_LOOKAHEAD_DAYS)
        .map(|days_ahead| {
            format!(
                "audit_{}",
                (today + chrono::Duration::days(days_ahead)).format("%Y%m%d")
            )
        })
        .filter(|name| !existing.contains(name.as_str()))
        .collect();
    if !missing.is_empty() {
        report_critical_error(
            format!(
                "Audit log partitions missing after maintenance run: {}. \
                 Audit inserts will fail once these dates are reached, which also \
                 breaks logins (the login audit row shares the login transaction). \
                 Check for earlier 'Error creating audit partition' logs and verify \
                 the audit-partition maintenance loop is still running.",
                missing.join(", ")
            ),
            db.clone(),
            None,
            None,
        )
        .await;
    }

    // Drop expired partitions
    let cutoff_date = today - chrono::Duration::days(retention_days);
    for partition_name in &partitions {
        if let Some(date_str) = partition_name.strip_prefix("audit_") {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y%m%d") {
                if date < cutoff_date {
                    let quoted_name = format!("\"{}\"", partition_name.replace('"', "\"\""));
                    let sql = format!("DROP TABLE IF EXISTS {quoted_name}");
                    match sqlx::query(&sql).execute(db).await {
                        Ok(_) => {
                            tracing::info!("Dropped expired audit partition {partition_name}")
                        }
                        Err(e) => tracing::error!(
                            "Error dropping audit partition {partition_name}: {e:?}"
                        ),
                    }
                }
            }
        }
    }
}

#[cfg(all(test, feature = "enterprise"))]
mod retention_overrides_tests {
    use super::parse_retention_overrides;
    use serde_json::json;

    fn obj(v: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn parses_valid_map() {
        let m = parse_retention_overrides(obj(json!({"a": 3600, "b": 0}))).unwrap();
        assert_eq!(m.get("a"), Some(&3600));
        assert_eq!(m.get("b"), Some(&0)); // 0 = keep forever, allowed
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn empty_map_is_ok() {
        assert!(parse_retention_overrides(obj(json!({})))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn rejects_negative() {
        // A negative value must not silently become keep-forever; the whole map is rejected.
        assert!(parse_retention_overrides(obj(json!({"a": 3600, "b": -1}))).is_err());
    }

    #[test]
    fn rejects_non_integer() {
        assert!(parse_retention_overrides(obj(json!({"a": "3600"}))).is_err());
        assert!(parse_retention_overrides(obj(json!({"a": 3600.5}))).is_err());
        assert!(parse_retention_overrides(obj(json!({"a": null}))).is_err());
    }

    #[test]
    fn rejects_too_many_overrides() {
        use windmill_common::global_settings::MAX_RETENTION_OVERRIDE_WORKSPACES;
        let at_cap: serde_json::Map<_, _> = (0..MAX_RETENTION_OVERRIDE_WORKSPACES)
            .map(|i| (format!("ws_{i}"), json!(3600)))
            .collect();
        assert!(parse_retention_overrides(at_cap.clone()).is_ok());
        let over_cap: serde_json::Map<_, _> = (0..MAX_RETENTION_OVERRIDE_WORKSPACES + 1)
            .map(|i| (format!("ws_{i}"), json!(3600)))
            .collect();
        assert!(parse_retention_overrides(over_cap).is_err());
    }
}
