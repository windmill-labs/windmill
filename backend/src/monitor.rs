use std::{
    collections::HashMap,
    fmt::Display,
    ops::Mul,
    str::FromStr,
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc,
    },
    time::Duration,
};

use chrono::{NaiveDateTime, Utc};
use rsmq_async::MultiplexedRsmq;
use serde::de::DeserializeOwned;
use sqlx::{Pool, Postgres};
use tokio::{
    join,
    sync::{mpsc, RwLock},
};

#[cfg(feature = "embedding")]
use windmill_api::embeddings::update_embeddings_db;
use windmill_api::{
    jobs::TIMEOUT_WAIT_RESULT,
    oauth2_ee::{build_oauth_clients, OAuthClient},
    DEFAULT_BODY_LIMIT, IS_SECURE, OAUTH_CLIENTS, REQUEST_SIZE_LIMIT, SAML_METADATA, SCIM_TOKEN,
};
#[cfg(feature = "enterprise")]
use windmill_common::ee::{worker_groups_alerts, jobs_waiting_alerts};
use windmill_common::{
    auth::JWT_SECRET,
    ee::CriticalErrorChannel,
    error,
    flow_status::FlowStatusModule,
    global_settings::{
        BASE_URL_SETTING, BUNFIG_INSTALL_SCOPES_SETTING, CRITICAL_ERROR_CHANNELS_SETTING,
        DEFAULT_TAGS_PER_WORKSPACE_SETTING, DEFAULT_TAGS_WORKSPACES_SETTING,
        EXPOSE_DEBUG_METRICS_SETTING, EXPOSE_METRICS_SETTING, EXTRA_PIP_INDEX_URL_SETTING,
        HUB_BASE_URL_SETTING, JOB_DEFAULT_TIMEOUT_SECS_SETTING, JWT_SECRET_SETTING,
        KEEP_JOB_DIR_SETTING, LICENSE_KEY_SETTING, NPM_CONFIG_REGISTRY_SETTING, OAUTH_SETTING,
        PIP_INDEX_URL_SETTING, REQUEST_SIZE_LIMIT_SETTING,
        REQUIRE_PREEXISTING_USER_FOR_OAUTH_SETTING, RETENTION_PERIOD_SECS_SETTING,
        SAML_METADATA_SETTING, SCIM_TOKEN_SETTING, TIMEOUT_WAIT_RESULT_SETTING,
    },
    jobs::QueuedJob,
    oauth2::REQUIRE_PREEXISTING_USER_FOR_OAUTH,
    server::load_smtp_config,
    tracing_init::JSON_FMT,
    users::truncate_token,
    utils::{now_from_db, rd_string, report_critical_error, Mode},
    worker::{
        load_worker_config, make_pull_query, make_suspended_pull_query, reload_custom_tags_setting,
        DEFAULT_TAGS_PER_WORKSPACE, DEFAULT_TAGS_WORKSPACES, SMTP_CONFIG, WORKER_CONFIG,
        WORKER_GROUP,
    },
    BASE_URL, CRITICAL_ERROR_CHANNELS, DB, DEFAULT_HUB_BASE_URL, HUB_BASE_URL, JOB_RETENTION_SECS,
    METRICS_DEBUG_ENABLED, METRICS_ENABLED,
};
use windmill_queue::cancel_job;
use windmill_worker::{
    create_token_for_owner, handle_job_error, AuthedClient, SameWorkerPayload, SameWorkerSender,
    SendResult, BUNFIG_INSTALL_SCOPES, JOB_DEFAULT_TIMEOUT, KEEP_JOB_DIR, NPM_CONFIG_REGISTRY,
    PIP_EXTRA_INDEX_URL, PIP_INDEX_URL, SCRIPT_TOKEN_EXPIRY,
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

#[cfg(feature = "enterprise")]
use windmill_common::ee::LICENSE_KEY_VALID;

use crate::ee::set_license_key;

lazy_static::lazy_static! {
    static ref ZOMBIE_JOB_TIMEOUT: String = std::env::var("ZOMBIE_JOB_TIMEOUT")
    .ok()
    .and_then(|x| x.parse::<String>().ok())
    .unwrap_or_else(|| "30".to_string());

    static ref FLOW_ZOMBIE_TRANSITION_TIMEOUT: String = std::env::var("FLOW_ZOMBIE_TRANSITION_TIMEOUT")
    .ok()
    .and_then(|x| x.parse::<String>().ok())
    .unwrap_or_else(|| "30".to_string());


    pub static ref RESTART_ZOMBIE_JOBS: bool = std::env::var("RESTART_ZOMBIE_JOBS")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(true);

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

pub async fn initial_load(
    db: &Pool<Postgres>,
    tx: tokio::sync::broadcast::Sender<()>,
    worker_mode: bool,
    server_mode: bool,
    _is_agent: bool,
) {
    if let Err(e) = load_metrics_enabled(db).await {
        tracing::error!("Error loading expose metrics: {e:#}");
    }

    if let Err(e) = load_metrics_debug_enabled(db).await {
        tracing::error!("Error loading expose debug metrics: {e:#}");
    }

    if let Err(e) = load_tag_per_workspace_enabled(db).await {
        tracing::error!("Error loading default tag per workpsace: {e:#}");
    }

    if let Err(e) = load_tag_per_workspace_workspaces(db).await {
        tracing::error!("Error loading default tag per workpsace workspaces: {e:#}");
    }

    if server_mode {
        load_require_preexisting_user(db).await;
    }

    if worker_mode {
        load_keep_job_dir(db).await;
        reload_worker_config(&db, tx, false).await;
    }

    if let Err(e) = reload_custom_tags_setting(db).await {
        tracing::error!("Error reloading custom tags: {:?}", e)
    }

    if let Err(e) = reload_base_url_setting(db).await {
        tracing::error!("Error reloading base url: {:?}", e)
    }

    if let Err(e) = reload_hub_base_url_setting(db, server_mode).await {
        tracing::error!("Error reloading hub base url: {:?}", e)
    }

    if let Err(e) = reload_critical_error_channels_setting(&db).await {
        tracing::error!("Could not reload critical error emails setting: {:?}", e);
    }

    if let Err(e) = reload_jwt_secret_setting(&db).await {
        tracing::error!("Could not reload jwt secret setting: {:?}", e);
    }

    #[cfg(feature = "parquet")]
    if !_is_agent {
        reload_s3_cache_setting(&db).await;
    }

    reload_smtp_config(&db).await;

    if server_mode {
        reload_retention_period_setting(&db).await;
        reload_request_size(&db).await;
        reload_saml_metadata_setting(&db).await;
        reload_scim_token_setting(&db).await;
    }

    #[cfg(feature = "enterprise")]
    if let Err(e) = reload_license_key(&db).await {
        tracing::error!("Error reloading license key: {:?}", e)
    }

    if worker_mode {
        reload_extra_pip_index_url_setting(&db).await;
        reload_pip_index_url_setting(&db).await;
        reload_npm_config_registry_setting(&db).await;
        reload_bunfig_install_scopes_setting(&db).await;
    }
}

pub async fn load_metrics_enabled(db: &DB) -> error::Result<()> {
    let metrics_enabled = load_value_from_global_settings(db, EXPOSE_METRICS_SETTING).await;
    match metrics_enabled {
        Ok(Some(serde_json::Value::Bool(t))) => METRICS_ENABLED.store(t, Ordering::Relaxed),
        _ => (),
    };
    Ok(())
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

pub async fn load_metrics_debug_enabled(db: &DB) -> error::Result<()> {
    let metrics_enabled = load_value_from_global_settings(db, EXPOSE_DEBUG_METRICS_SETTING).await;
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

pub fn send_logs_to_object_store(db: &DB, hostname: &str, mode: &Mode) {
    let db = db.clone();
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
                &db,
                snd_highest_file,
                false,
            )
            .await;
        }
    });
}

pub async fn send_current_log_file_to_object_store(db: &DB, hostname: &str, mode: &Mode) {
    tracing::info!("Sending current log file to object store");
    let (highest_file, _) = find_two_highest_files(hostname).await;
    let worker_group = get_worker_group(&mode);
    send_log_file_to_object_store(hostname, mode, &worker_group, db, highest_file, true).await;
}

fn get_now_and_str() -> (NaiveDateTime, String) {
    let ts = Utc::now().naive_utc();
    (
        ts,
        ts.format(windmill_common::tracing_init::LOG_TIMESTAMP_FMT)
            .to_string(),
    )
}

async fn send_log_file_to_object_store(
    hostname: &str,
    mode: &Mode,
    worker_group: &Option<String>,
    db: &Pool<Postgres>,
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

        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM log_file WHERE hostname = $1 AND log_ts = $2)",
            hostname,
            ts
        )
        .fetch_one(db)
        .await;

        match exists {
            Ok(Some(true)) => {
                return;
            }
            Err(e) => {
                tracing::error!("Error checking if log file exists: {:?}", e);
                return;
            }
            _ => (),
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

        if let Err(e) = sqlx::query!("INSERT INTO log_file (hostname, mode, worker_group, log_ts, file_path, ok_lines, err_lines, json_fmt) VALUES ($1, $2::text::LOG_MODE, $3, $4, $5, $6, $7, $8)", 
            hostname, mode.to_string(), worker_group.clone(), ts, highest_file, ok_lines as i64, err_lines as i64, *JSON_FMT)
            .execute(db)
            .await {
            tracing::error!("Error inserting log file: {:?}", e);
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

pub async fn load_keep_job_dir(db: &DB) {
    let value = load_value_from_global_settings(db, KEEP_JOB_DIR_SETTING).await;
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

    let job_retention_secs = *JOB_RETENTION_SECS.read().await;
    if job_retention_secs > 0 {
        match db.begin().await {
            Ok(mut tx) => {
                let deleted_jobs = sqlx::query_scalar!(
                            "DELETE FROM completed_job WHERE created_at <= now() - ($1::bigint::text || ' s')::interval  AND started_at + ((duration_ms/1000 + $1::bigint) || ' s')::interval <= now() RETURNING id",
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
                            if let Err(e) = sqlx::query!(
                                "DELETE FROM job_logs WHERE job_id = ANY($1)",
                                &deleted_jobs
                            )
                            .execute(&mut *tx)
                            .await
                            {
                                tracing::error!("Error deleting job stats: {:?}", e);
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
                            if let Err(e) = sqlx::query!(
                                "DELETE FROM log_file WHERE log_ts <= now() - ($1::bigint::text || ' s')::interval ",
                                job_retention_secs
                            )
                            .execute(&mut *tx)
                            .await
                            {
                                tracing::error!("Error deleting log file: {:?}", e);
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

pub async fn reload_scim_token_setting(db: &DB) {
    reload_option_setting_with_tracing(db, SCIM_TOKEN_SETTING, "SCIM_TOKEN", SCIM_TOKEN.clone())
        .await;
}

pub async fn reload_timeout_wait_result_setting(db: &DB) {
    reload_option_setting_with_tracing(
        db,
        TIMEOUT_WAIT_RESULT_SETTING,
        "TIMEOUT_WAIT_RESULT",
        TIMEOUT_WAIT_RESULT.clone(),
    )
    .await;
}
pub async fn reload_saml_metadata_setting(db: &DB) {
    reload_option_setting_with_tracing(
        db,
        SAML_METADATA_SETTING,
        "SAML_METADATA",
        SAML_METADATA.clone(),
    )
    .await;
}

pub async fn reload_extra_pip_index_url_setting(db: &DB) {
    reload_option_setting_with_tracing(
        db,
        EXTRA_PIP_INDEX_URL_SETTING,
        "PIP_EXTRA_INDEX_URL",
        PIP_EXTRA_INDEX_URL.clone(),
    )
    .await;
}

pub async fn reload_pip_index_url_setting(db: &DB) {
    reload_option_setting_with_tracing(
        db,
        PIP_INDEX_URL_SETTING,
        "PIP_INDEX_URL",
        PIP_INDEX_URL.clone(),
    )
    .await;
}

pub async fn reload_npm_config_registry_setting(db: &DB) {
    reload_option_setting_with_tracing(
        db,
        NPM_CONFIG_REGISTRY_SETTING,
        "NPM_CONFIG_REGISTRY",
        NPM_CONFIG_REGISTRY.clone(),
    )
    .await;
}

pub async fn reload_bunfig_install_scopes_setting(db: &DB) {
    reload_option_setting_with_tracing(
        db,
        BUNFIG_INSTALL_SCOPES_SETTING,
        "BUNFIG_INSTALL_SCOPES",
        BUNFIG_INSTALL_SCOPES.clone(),
    )
    .await;
}

pub async fn reload_retention_period_setting(db: &DB) {
    if let Err(e) = reload_setting(
        db,
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

pub async fn reload_job_default_timeout_setting(db: &DB) {
    reload_option_setting_with_tracing(
        db,
        JOB_DEFAULT_TIMEOUT_SECS_SETTING,
        "JOB_DEFAULT_TIMEOUT_SECS",
        JOB_DEFAULT_TIMEOUT.clone(),
    )
    .await;
}

pub async fn reload_request_size(db: &DB) {
    if let Err(e) = reload_setting(
        db,
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

pub async fn reload_license_key(db: &DB) -> error::Result<()> {
    let q = load_value_from_global_settings(db, LICENSE_KEY_SETTING).await?;

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

    set_license_key(value).await?;

    Ok(())
}

pub async fn reload_option_setting_with_tracing<T: FromStr + DeserializeOwned>(
    db: &DB,
    setting_name: &str,
    std_env_var: &str,
    lock: Arc<RwLock<Option<T>>>,
) {
    if let Err(e) = reload_option_setting(db, setting_name, std_env_var, lock.clone()).await {
        tracing::error!("Error reloading setting {}: {:?}", setting_name, e)
    }
}

async fn load_value_from_global_settings(
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
pub async fn reload_option_setting<T: FromStr + DeserializeOwned>(
    db: &DB,
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

    let q = load_value_from_global_settings(db, setting_name).await?;

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
    db: &DB,
    setting_name: &str,
    std_env_var: &str,
    default: T,
    lock: Arc<RwLock<T>>,
    transformer: fn(T) -> T,
) -> error::Result<()> {
    let q = load_value_from_global_settings(db, setting_name).await?;

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
    db: &Pool<Postgres>,
    base_internal_url: &str,
    rsmq: Option<MultiplexedRsmq>,
    server_mode: bool,
    initial_load: bool,
) {
    let zombie_jobs_f = async {
        if server_mode && !initial_load {
            handle_zombie_jobs(db, base_internal_url, rsmq.clone(), "server").await;
            match handle_zombie_flows(db, rsmq.clone()).await {
                Err(err) => {
                    tracing::error!("Error handling zombie flows: {:?}", err);
                }
                _ => {}
            }
        }
    };
    let expired_items_f = async {
        if server_mode && !initial_load {
            delete_expired_items(&db).await;
        }
    };

    let verify_license_key_f = async {
        #[cfg(feature = "enterprise")]
        if let Err(e) = verify_license_key().await {
            tracing::error!("Error verifying license key: {:?}", e);
            let mut l = LICENSE_KEY_VALID.write().await;
            *l = false;
        } else {
            let is_valid = LICENSE_KEY_VALID.read().await.clone();
            if !is_valid {
                let mut l = LICENSE_KEY_VALID.write().await;
                *l = true;
            }
        }
    };

    let expose_queue_metrics_f = async {
        if !initial_load && server_mode {
            expose_queue_metrics(&db).await;
        }
    };

    let worker_groups_alerts_f = async {
        #[cfg(feature = "enterprise")]
        if server_mode && !initial_load {
            worker_groups_alerts(&db).await;
        }
    };

    let jobs_waiting_alerts_f = async {
        #[cfg(feature = "enterprise")]
        if server_mode {
            jobs_waiting_alerts(&db).await;
        }
    };

    join!(
        expired_items_f,
        zombie_jobs_f,
        expose_queue_metrics_f,
        verify_license_key_f,
        worker_groups_alerts_f,
        jobs_waiting_alerts_f,
    );
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
        let queue_counts = sqlx::query!(
            "SELECT tag, count(*) as count FROM queue WHERE
                scheduled_for <= now() - ('3 seconds')::interval AND running = false
                GROUP BY tag"
        )
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![]);

        for q in queue_counts {
            let count = q.count.unwrap_or(0);
            let tag = q.tag;
            if metrics_enabled {
                let metric = (*QUEUE_COUNT).with_label_values(&[&tag]);
                metric.set(count as i64);
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
                            VALUES ($1, to_jsonb((SELECT EXTRACT(EPOCH FROM now() - scheduled_for)
                            FROM queue WHERE tag = $2 AND running = false AND scheduled_for <= now() - ('3 seconds')::interval
                            ORDER BY priority DESC NULLS LAST, scheduled_for LIMIT 1)))",
                            format!("queue_delay_{}", tag),
                            tag
                        ).execute(db).await.ok();
                }
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
        let mut wc = SMTP_CONFIG.write().await;
        tracing::info!("Reloading smtp config...");
        *wc = smtp_config.unwrap()
    }
}

pub async fn reload_worker_config(
    db: &DB,
    tx: tokio::sync::broadcast::Sender<()>,
    kill_if_change: bool,
) {
    let config = load_worker_config(&db, tx.clone()).await;
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
                    let _ = tx.send(());
                }

                if (*wc).init_bash != config.init_bash {
                    tracing::info!("Init bash config changed, sending killpill. Expecting to be restarted by supervisor.");
                    let _ = tx.send(());
                }

                if (*wc).cache_clear != config.cache_clear {
                    tracing::info!("Cache clear changed, sending killpill. Expecting to be restarted by supervisor.");
                    let _ = tx.send(());
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
            make_suspended_pull_query(&config).await;
            make_pull_query(&config).await;
            *wc = config
        }
    }
}

pub async fn reload_base_url_setting(db: &DB) -> error::Result<()> {
    let q_base_url = load_value_from_global_settings(db, BASE_URL_SETTING).await?;

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

    let q_oauth = load_value_from_global_settings(db, OAUTH_SETTING).await?;

    let oauths = if let Some(q) = q_oauth {
        if let Ok(v) = serde_json::from_value::<Option<HashMap<String, OAuthClient>>>(q.clone()) {
            v
        } else {
            tracing::error!("Could not parse oauth setting as a json, found: {:#?}", &q);
            None
        }
    } else {
        None
    };

    let is_secure = base_url.starts_with("https://");

    {
        let mut l = OAUTH_CLIENTS.write().await;
        *l = build_oauth_clients(&base_url, oauths)
        .map_err(|e| tracing::error!("Error building oauth clients (is the oauth.json mounted and in correct format? Use '{}' as minimal oauth.json): {}", "{}", e))
        .unwrap();
    }

    {
        let mut l = BASE_URL.write().await;
        *l = base_url
    }

    {
        let mut l = IS_SECURE.write().await;
        *l = is_secure;
    }

    Ok(())
}

async fn handle_zombie_jobs<R: rsmq_async::RsmqConnection + Send + Sync + Clone>(
    db: &Pool<Postgres>,
    base_internal_url: &str,
    rsmq: Option<R>,
    worker_name: &str,
) {
    if *RESTART_ZOMBIE_JOBS {
        let restarted = sqlx::query!(
                "UPDATE queue SET running = false, started_at = null
                WHERE last_ping < now() - ($1 || ' seconds')::interval
                 AND running = true AND job_kind NOT IN ('flow', 'flowpreview', 'singlescriptflow') AND same_worker = false RETURNING id, workspace_id, last_ping",
                *ZOMBIE_JOB_TIMEOUT,
            )
            .fetch_all(db)
            .await
            .ok()
            .unwrap_or_else(|| vec![]);

        if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            QUEUE_ZOMBIE_RESTART_COUNT.inc_by(restarted.len() as _);
        }
        let base_url = BASE_URL.read().await.clone();
        for r in restarted {
            let last_ping = if let Some(x) = r.last_ping {
                format!("last ping at {x}")
            } else {
                "no last ping".to_string()
            };
            let url = format!("{}/run/{}?workspace={}", base_url, r.id, r.workspace_id,);
            let error_message = format!(
                "Zombie job {} on {} ({}) detected, restarting it, {}",
                r.id, r.workspace_id, url, last_ping
            );

            let _ = sqlx::query!("
                INSERT INTO job_logs (job_id, logs) VALUES ($1,'Restarted job after not receiving job''s ping for too long the ' || now() || '\n\n') 
                ON CONFLICT (job_id) DO UPDATE SET logs = job_logs.logs || '\nRestarted job after not receiving job''s ping for too long the ' || now() || '\n\n' WHERE job_logs.job_id = $1", r.id)
                .execute(db).await;
            tracing::error!(error_message);
            report_critical_error(error_message, db.clone()).await;
        }
    }

    let mut timeout_query =
        "SELECT * FROM queue WHERE last_ping < now() - ($1 || ' seconds')::interval 
    AND running = true  AND job_kind NOT IN ('flow', 'flowpreview', 'singlescriptflow')"
            .to_string();
    if *RESTART_ZOMBIE_JOBS {
        timeout_query.push_str(" AND same_worker = true");
    };
    let timeouts = sqlx::query_as::<_, QueuedJob>(&timeout_query)
        .bind(ZOMBIE_JOB_TIMEOUT.as_str())
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![]);

    if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        QUEUE_ZOMBIE_DELETE_COUNT.inc_by(timeouts.len() as _);
    }

    for job in timeouts {
        tracing::info!("timedout zombie job {} {}", job.id, job.workspace_id,);

        // since the job is unrecoverable, the same worker queue should never be sent anything
        let (same_worker_tx_never_used, _same_worker_rx_never_used) =
            mpsc::channel::<SameWorkerPayload>(1);
        let same_worker_tx_never_used =
            SameWorkerSender(same_worker_tx_never_used, Arc::new(AtomicU16::new(0)));
        let (send_result_never_used, _send_result_rx_never_used) = mpsc::channel::<SendResult>(1);

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
        let _ = handle_job_error(
            db,
            &client,
            &job,
            0,
            None,
            error::Error::ExecutionErr(format!(
                "Job timed out after no ping from job since {} (ZOMBIE_JOB_TIMEOUT: {})",
                last_ping
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| "no ping".to_string()),
                *ZOMBIE_JOB_TIMEOUT
            )),
            true,
            same_worker_tx_never_used,
            "",
            rsmq.clone(),
            worker_name,
            send_result_never_used,
            #[cfg(feature = "benchmark")]
            &mut windmill_common::bench::BenchmarkIter::new(),
        )
        .await;
    }
}

async fn handle_zombie_flows(
    db: &DB,
    rsmq: Option<rsmq_async::MultiplexedRsmq>,
) -> error::Result<()> {
    let flows = sqlx::query_as::<_, QueuedJob>(
        r#"
        SELECT *
        FROM queue
        WHERE running = true AND suspend = 0 AND suspend_until IS null AND scheduled_for <= now() AND (job_kind = 'flow' OR job_kind = 'flowpreview')
            AND last_ping IS NOT NULL AND last_ping < NOW() - ($1 || ' seconds')::interval AND canceled = false
        "#,
    ).bind(FLOW_ZOMBIE_TRANSITION_TIMEOUT.as_str())
    .fetch_all(db)
    .await?;

    for flow in flows {
        let status = flow.parse_flow_status();
        if !flow.same_worker
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
            report_critical_error(error_message, db.clone()).await;
            // if the flow hasn't started and is a zombie, we can simply restart it
            sqlx::query!(
                "UPDATE queue SET running = false, started_at = null WHERE id = $1 AND canceled = false",
                flow.id
            )
            .execute(db)
            .await?;
        } else {
            let id = flow.id.clone();
            let last_ping = flow.last_ping.clone();
            let now = now_from_db(db).await?;
            let reason = format!(
                "{} was hanging in between 2 steps. Last ping: {last_ping:?} (now: {now})",
                if flow.is_flow_step && flow.parent_job.is_some() {
                    format!("Flow was cancelled because subflow {id}")
                } else {
                    format!("Flow {id} was cancelled because it")
                }
            );
            report_critical_error(reason.clone(), db.clone()).await;
            cancel_zombie_flow_job(db, flow, &rsmq, reason).await?;
        }
    }

    let flows2 = sqlx::query!(
        "
    DELETE
    FROM parallel_monitor_lock
    WHERE last_ping IS NOT NULL AND last_ping < NOW() - ($1 || ' seconds')::interval 
    RETURNING parent_flow_id, job_id, last_ping
        ",
        FLOW_ZOMBIE_TRANSITION_TIMEOUT.as_str()
    )
    .fetch_all(db)
    .await?;

    for flow in flows2 {
        let in_queue = sqlx::query_as::<_, QueuedJob>(
            "SELECT * FROM queue WHERE id = $1 AND running = true AND canceled = false",
        )
        .bind(flow.parent_flow_id)
        .fetch_optional(db)
        .await?;
        if let Some(job) = in_queue {
            tracing::error!(
                "parallel Zombie flow detected: {} in workspace {}. Last ping was: {:?}.",
                job.id,
                job.workspace_id,
                flow.last_ping
            );
            cancel_zombie_flow_job(db, job, &rsmq,
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
    flow: QueuedJob,
    rsmq: &Option<MultiplexedRsmq>,
    message: String,
) -> Result<(), error::Error> {
    let tx = db.begin().await.unwrap();
    tracing::error!(
        "zombie flow detected: {} in workspace {}. Cancelling it.",
        flow.id,
        flow.workspace_id
    );
    let (ntx, _) = cancel_job(
        "monitor",
        Some(message),
        flow.id,
        flow.workspace_id.as_str(),
        tx,
        db,
        rsmq.clone(),
        true,
        false,
    )
    .await?;
    ntx.commit().await?;
    Ok(())
}

pub async fn reload_hub_base_url_setting(db: &DB, server_mode: bool) -> error::Result<()> {
    let hub_base_url = load_value_from_global_settings(db, HUB_BASE_URL_SETTING).await?;

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
    *l = base_url;

    Ok(())
}

pub async fn reload_critical_error_channels_setting(db: &DB) -> error::Result<()> {
    let critical_error_channels =
        load_value_from_global_settings(db, CRITICAL_ERROR_CHANNELS_SETTING).await?;

    let critical_error_channels = if let Some(q) = critical_error_channels {
        if let Ok(v) = serde_json::from_value::<Vec<CriticalErrorChannel>>(q.clone()) {
            v
        } else {
            tracing::error!(
                "Could not parse critical_error_emails setting as an array of channels, found: {:#?}",
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
