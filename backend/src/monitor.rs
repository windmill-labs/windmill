use std::{
    collections::HashMap,
    fmt::Display,
    ops::Mul,
    str::FromStr,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use rsmq_async::MultiplexedRsmq;
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{Pool, Postgres};
use tokio::{
    join,
    sync::{mpsc, RwLock},
};

#[cfg(feature = "embedding")]
use windmill_api::embeddings::update_embeddings_db;
use windmill_api::{
    oauth2_ee::{build_oauth_clients, OAuthClient},
    DEFAULT_BODY_LIMIT, IS_SECURE, OAUTH_CLIENTS, REQUEST_SIZE_LIMIT, SAML_METADATA, SCIM_TOKEN,
};
use windmill_common::{
    ee::CriticalErrorChannel, error, flow_status::FlowStatusModule, global_settings::{
        BASE_URL_SETTING, BUNFIG_INSTALL_SCOPES_SETTING, CRITICAL_ERROR_CHANNELS_SETTING,
        DEFAULT_TAGS_PER_WORKSPACE_SETTING, EXPOSE_DEBUG_METRICS_SETTING, EXPOSE_METRICS_SETTING,
        EXTRA_PIP_INDEX_URL_SETTING, HUB_BASE_URL_SETTING, JOB_DEFAULT_TIMEOUT_SECS_SETTING,
        KEEP_JOB_DIR_SETTING, LICENSE_KEY_SETTING, NPM_CONFIG_REGISTRY_SETTING, OAUTH_SETTING,
        PIP_INDEX_URL_SETTING, REQUEST_SIZE_LIMIT_SETTING,
        REQUIRE_PREEXISTING_USER_FOR_OAUTH_SETTING, RETENTION_PERIOD_SECS_SETTING,
        SAML_METADATA_SETTING, SCIM_TOKEN_SETTING,
    }, jobs::QueuedJob, oauth2::REQUIRE_PREEXISTING_USER_FOR_OAUTH, server::load_server_config, stats_ee::get_user_usage, users::truncate_token, worker::{
        load_worker_config, reload_custom_tags_setting, DEFAULT_TAGS_PER_WORKSPACE, SERVER_CONFIG,
        WORKER_CONFIG,
    }, BASE_URL, CRITICAL_ERROR_CHANNELS, DB, DEFAULT_HUB_BASE_URL, HUB_BASE_URL, METRICS_DEBUG_ENABLED, METRICS_ENABLED
};
use windmill_queue::cancel_job;
use windmill_worker::{
    create_token_for_owner, handle_job_error, AuthedClient, SameWorkerPayload, SendResult,
    BUNFIG_INSTALL_SCOPES, JOB_DEFAULT_TIMEOUT, KEEP_JOB_DIR, NPM_CONFIG_REGISTRY,
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

    static ref JOB_RETENTION_SECS: Arc<RwLock<i64>> = Arc::new(RwLock::new(0));
}

pub async fn initial_load(
    db: &Pool<Postgres>,
    tx: tokio::sync::broadcast::Sender<()>,
    worker_mode: bool,
    server_mode: bool,
    _is_agent: bool,
) {
    if let Err(e) = load_metrics_enabled(db).await {
        tracing::error!("Error loading expose metrics: {e}");
    }

    if let Err(e) = load_metrics_debug_enabled(db).await {
        tracing::error!("Error loading expose debug metrics: {e}");
    }

    if let Err(e) = load_tag_per_workspace_enabled(db).await {
        tracing::error!("Error loading default tag per workpsace: {e}");
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

    #[cfg(feature = "parquet")]
    if !_is_agent {
        reload_s3_cache_setting(&db).await;
    }

    if server_mode {
        reload_server_config(&db).await;
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
        },
        _ => (),
    };
    Ok(())
}

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
#[derive(Debug, Clone)]
pub struct MallctlError { pub code: i32 }

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
fn set_prof_active(new_value: bool) -> Result<(), MallctlError> {
    let option_name = std::ffi::CString::new("prof.active").unwrap();

    tracing::info!("Setting jemalloc prof_active to {}", new_value);
    let result = unsafe {

        tikv_jemalloc_sys::mallctl(
            option_name.as_ptr(), // const char *name
            std::ptr::null_mut(), // void *oldp
            std::ptr::null_mut(), // size_t *oldlenp
            &new_value as *const _ as *mut _, // void *newp
            std::mem::size_of_val(&new_value) // size_t newlen
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
    use tikv_jemalloc_ctl::{stats, epoch};

    tokio::spawn(async move {

        // Obtain a MIB for the `epoch`, `stats.allocated`, and
        // `atats.resident` keys:
        let e = epoch::mib().unwrap();
        let allocated = stats::allocated::mib().unwrap();
        let resident = stats::resident::mib().unwrap();
        
        loop {
            // Many statistics are cached and only updated 
            // when the epoch is advanced:
            e.advance().unwrap();
            
            // Read statistics using MIB key:
            let allocated = allocated.read().unwrap();
            let resident = resident.read().unwrap();
            tracing::info!("{} mb allocated/{} mb resident", bytes_to_mb(allocated as u64), bytes_to_mb(resident as u64));
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });
}

pub async fn load_keep_job_dir(db: &DB) {
    let value = load_value_from_global_settings(db, KEEP_JOB_DIR_SETTING).await;
    match value {
        Ok(Some(serde_json::Value::Bool(t))) => KEEP_JOB_DIR.store(t, Ordering::Relaxed),
        Err(e) => {
            tracing::error!("Error loading keep job dir metrics: {e}");
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
            tracing::error!("Error loading keep job dir metrics: {e}");
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
                    allow_http: None,
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

    let save_usage_metrics_f = async {
        if !initial_load && server_mode {
            save_usage_metrics(&db).await;
        }
    };

    join!(
        expired_items_f,
        zombie_jobs_f,
        expose_queue_metrics_f,
        save_usage_metrics_f,
        verify_license_key_f
    );
}

pub async fn expose_queue_metrics(db: &Pool<Postgres>) {
    let tx = db.begin().await;
    if let Ok(mut tx) = tx {
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
            .fetch_all(&mut *tx)
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
                    .execute(&mut *tx)
                    .await
                    .ok();
                    if count > 0 {
                        sqlx::query!(
                            "INSERT INTO metrics (id, value)
                            VALUES ($1, to_jsonb((SELECT EXTRACT(EPOCH FROM now() - scheduled_for)
                            FROM queue WHERE tag = $2 AND running = false AND scheduled_for <= now() - ('3 seconds')::interval
                            ORDER BY priority DESC NULLS LAST, scheduled_for, created_at LIMIT 1)))",
                            format!("queue_delay_{}", tag),
                            tag
                        ).execute(&mut *tx).await.ok();
                    }
                }
            }
        }

        // clean queue metrics older than 14 days
        sqlx::query!(
            "DELETE FROM metrics WHERE id LIKE 'queue_%' AND created_at < NOW() - INTERVAL '14 day'"
        ).execute(&mut *tx).await.ok();

        tx.commit().await.ok();
    }
}


#[derive(Serialize)]
struct WorkerUsage {
    worker: String,
    worker_instance: String,
    vcpus: Option<i64>,
    memory: Option<i64>,

}

pub async fn save_usage_metrics(db: &Pool<Postgres>) {
    let tx = db.begin().await;

    if let Ok(mut tx) = tx {
        let last_check = sqlx::query_scalar!(
            "SELECT created_at FROM metrics WHERE id = 'author_count' ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(db)
        .await
        .unwrap_or(Some(chrono::Utc::now()));

        let random_nb = rand::random::<i64>();

        // save author and operator count every ~24 hours
        if last_check
            .map(|last_check| chrono::Utc::now() - last_check > chrono::Duration::hours(24) - chrono::Duration::minutes(random_nb % 60))
            .unwrap_or(true)
        {
            let user_usage = get_user_usage(&mut *tx).await.ok();

            if let Some(user_usage) = user_usage {
                sqlx::query!(
                    "INSERT INTO metrics (id, value) VALUES ('author_count', $1), ('operator_count', $2)",
                    serde_json::json!(user_usage.author_count.unwrap_or(0)), 
                    serde_json::json!(user_usage.operator_count.unwrap_or(0))
                )
                .execute(&mut *tx)
                .await
                .ok();
            }


            // clean metrics older than 6 months (including worker usage)
            sqlx::query!(
                "DELETE FROM metrics 
                WHERE (id = 'author_count' OR id = 'operator_count' OR id = 'worker_usage') AND created_at < NOW() - INTERVAL '6 month'"
            )
            .execute(&mut *tx)
            .await
            .ok();
        }

        // save worker usage every ~60 minutes
        if last_check
            .map(|last_check| chrono::Utc::now() - last_check > chrono::Duration::minutes(60) - chrono::Duration::seconds(random_nb % 300))
            .unwrap_or(true)
        {
            let worker_usage = sqlx::query_as!(
                WorkerUsage,
                "SELECT worker, worker_instance, vcpus, memory FROM worker_ping WHERE ping_at > NOW() - INTERVAL '2 minutes'"
            )
            .fetch_all(&mut *tx)
            .await
            .ok();

            if let Some(worker_usage) = worker_usage {
                sqlx::query!(
                    "INSERT INTO metrics (id, value) VALUES ('worker_usage', $1)",
                    serde_json::json!(worker_usage)
                )
                .execute(&mut *tx)
                .await
                .ok();
            }
        }

        tx.commit().await.ok();
    }

}

pub async fn reload_server_config(db: &Pool<Postgres>) {
    let config = load_server_config(&db).await;
    if let Err(e) = config {
        tracing::error!("Error reloading server config: {:?}", e)
    } else {
        let mut wc = SERVER_CONFIG.write().await;
        tracing::info!("Reloading server config...");
        *wc = config.unwrap()
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
                        tracing::error!("Error cleaning the cache: {e}");
                    }
                }
            }
            drop(wc);

            let mut wc = WORKER_CONFIG.write().await;
            tracing::info!("Reloading worker config...");
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
                "UPDATE queue SET running = false, started_at = null, logs = logs || '\nRestarted job after not receiving job''s ping for too long the ' || now() || '\n\n' 
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
        for r in restarted {
            tracing::error!(
                "Zombie job detected, restarting it: {} {} {:?}",
                r.id,
                r.workspace_id,
                r.last_ping
            );
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
        let (send_result_never_used, _send_result_rx_never_used) = mpsc::channel::<SendResult>(1);

        let token = create_token_for_owner(
            &db,
            &job.workspace_id,
            &job.permissioned_as,
            "ephemeral-script",
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
            AND last_ping IS NOT NULL AND last_ping < NOW() - ($1 || ' seconds')::interval 
        "#,
    ).bind(FLOW_ZOMBIE_TRANSITION_TIMEOUT.as_str())
    .fetch_all(db)
    .await?;

    for flow in flows {
        let status = flow.parse_flow_status();
        if status.is_some_and(|s| {
            s.modules
                .get(0)
                .is_some_and(|x| matches!(x, FlowStatusModule::WaitingForPriorSteps { .. }))
        }) {
            tracing::error!(
                "Zombie flow detected: {} in workspace {}. It hasn't started yet, restarting it.",
                flow.id,
                flow.workspace_id
            );
            // if the flow hasn't started and is a zombie, we can simply restart it
            sqlx::query!(
                "UPDATE queue SET running = false, started_at = null WHERE id = $1",
                flow.id
            )
            .execute(db)
            .await?;
        } else {
            let id = flow.id.clone();
            cancel_zombie_flow_job(
                db,
                flow,
                &rsmq,
                format!("Flow {} cancelled as it was hanging in between 2 steps", id),
            )
            .await?;
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
        let in_queue =
            sqlx::query_as::<_, QueuedJob>("SELECT * FROM queue WHERE id = $1 AND running = true")
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
    let (mut ntx, _) = cancel_job(
        "monitor",
        Some(message),
        flow.id,
        flow.workspace_id.as_str(),
        tx,
        db,
        rsmq.clone(),
        false,
    )
    .await?;
    sqlx::query!(
        "UPDATE queue SET running = false, started_at = null WHERE id = $1",
        flow.id
    )
    .execute(&mut *ntx)
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
