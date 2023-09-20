use std::{collections::HashMap, fmt::Display, ops::Mul, str::FromStr, sync::Arc};

use once_cell::sync::OnceCell;
use serde::de::DeserializeOwned;
use sqlx::{Pool, Postgres};
use tokio::{
    join,
    sync::{mpsc, RwLock},
};
use uuid::Uuid;
use windmill_api::{
    oauth2::{build_oauth_clients, OAuthClient},
    DEFAULT_BODY_LIMIT, IS_SECURE, OAUTH_CLIENTS, REQUEST_SIZE_LIMIT,
};
use windmill_common::{
    error,
    global_settings::{
        BASE_URL_SETTING, OAUTH_SETTING, REQUEST_SIZE_LIMIT_SETTING, RETENTION_PERIOD_SECS_SETTING,
    },
    jobs::{JobKind, QueuedJob},
    server::load_server_config,
    worker::{load_worker_config, reload_custom_tags_setting, SERVER_CONFIG, WORKER_CONFIG},
    BASE_URL, DB, METRICS_ENABLED,
};
use windmill_worker::{
    create_token_for_owner, handle_job_error, AuthedClient, SCRIPT_TOKEN_EXPIRY,
};

lazy_static::lazy_static! {
    static ref ZOMBIE_JOB_TIMEOUT: String = std::env::var("ZOMBIE_JOB_TIMEOUT")
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
) {
    let reload_worker_config_f = async {
        if worker_mode {
            reload_worker_config(&db, tx, false).await;
        }
    };
    let reload_custom_tags_f = async {
        if server_mode {
            if let Err(e) = reload_custom_tags_setting(db).await {
                tracing::error!("Error reloading custom tags: {:?}", e)
            }
        }
    };

    let reload_base_url_f = async {
        if server_mode {
            if let Err(e) = reload_base_url_setting(db).await {
                tracing::error!("Error reloading custom tags: {:?}", e)
            }
        }
    };

    let reload_server_config_f = async {
        if server_mode {
            reload_server_config(&db).await;
        }
    };
    let reload_retention_period_f = async {
        if server_mode {
            reload_retention_period_setting(&db).await;
        }
    };

    let reload_request_size_f = async {
        if server_mode {
            reload_request_size(&db).await;
        }
    };
    join!(
        reload_worker_config_f,
        reload_server_config_f,
        reload_custom_tags_f,
        reload_request_size_f,
        reload_base_url_f,
        reload_retention_period_f
    );
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
        let deleted_jobs = sqlx::query_scalar!(
                "DELETE FROM completed_job WHERE started_at + ((duration_ms/1000 + $1) || ' s')::interval <= now() RETURNING id",
                job_retention_secs
            )
            .fetch_all(db)
            .await;

        match deleted_jobs {
            Ok(deleted_jobs) => {
                if deleted_jobs.len() > 0 {
                    tracing::info!(
                        "deleted {} jobs completed JOB_RETENTION_SECS {} ago: {:?}",
                        deleted_jobs.len(),
                        job_retention_secs,
                        deleted_jobs,
                    )
                }
            }
            Err(e) => tracing::error!("Error deleting jobs: {}", e.to_string()),
        }
    }
}

pub async fn reload_retention_period_setting(db: &DB) {
    if let Err(e) = reload_setting(
        db,
        RETENTION_PERIOD_SECS_SETTING,
        "JOB_RETENTION_SECS",
        60 * 60 * 24 * 60,
        JOB_RETENTION_SECS.clone(),
        |x| x,
    )
    .await
    {
        tracing::error!("Error reloading retention period: {:?}", e)
    }
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

pub async fn reload_setting<T: FromStr + DeserializeOwned + Display>(
    db: &DB,
    setting_name: &str,
    std_env_var: &str,
    default: T,
    lock: Arc<RwLock<T>>,
    transformer: fn(T) -> T,
) -> error::Result<()> {
    let q = sqlx::query!(
        "SELECT value FROM global_settings WHERE name = $1",
        setting_name
    )
    .fetch_optional(db)
    .await?;

    let mut value = std::env::var(std_env_var)
        .ok()
        .and_then(|x| x.parse::<T>().ok())
        .unwrap_or(default);

    if let Some(q) = q {
        if let Ok(v) = serde_json::from_value::<T>(q.value.clone()) {
            tracing::info!(
                "Loaded setting {setting_name} from db config: {:#?}",
                &q.value
            );
            value = transformer(v);
        } else {
            tracing::error!("Could not parse {setting_name} found: {:#?}", &q.value);
        }
    };

    {
        let mut l = lock.write().await;
        *l = value;
    }

    Ok(())
}

pub async fn monitor_db<R: rsmq_async::RsmqConnection + Send + Sync + Clone + 'static>(
    db: &Pool<Postgres>,
    base_internal_url: &str,
    rsmq: Option<R>,
    server_mode: bool,
) {
    let zombie_jobs_f = async {
        if server_mode {
            handle_zombie_jobs(db, base_internal_url, rsmq.clone()).await;
        }
    };
    let expired_items_f = async {
        if server_mode {
            delete_expired_items(&db).await;
        }
    };

    let expose_queue_metrics_f = async {
        if *METRICS_ENABLED && server_mode {
            expose_queue_metrics(&db).await;
        }
    };
    join!(expired_items_f, zombie_jobs_f, expose_queue_metrics_f);
}

pub async fn expose_queue_metrics(db: &Pool<Postgres>) {
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
        let metric = (*QUEUE_COUNT).with_label_values(&[&tag]);
        metric.set(count as i64);
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
    let config = load_worker_config(&db).await;
    if let Err(e) = config {
        tracing::error!("Error reloading worker config: {:?}", e)
    } else {
        let wc = WORKER_CONFIG.read().await;
        let config = config.unwrap();
        if *wc != config {
            if kill_if_change && (*wc).dedicated_worker != config.dedicated_worker {
                tracing::info!("Dedicated worker config changed, sending killpill. Expecting to be restarted by supervisor.");
                let _ = tx.send(());
            }
            drop(wc);

            let mut wc = WORKER_CONFIG.write().await;
            tracing::info!("Reloading worker config...");
            *wc = config
        }
    }
}

pub async fn reload_base_url_setting(db: &DB) -> error::Result<()> {
    let q_base_url = sqlx::query!(
        "SELECT value FROM global_settings WHERE name = $1",
        BASE_URL_SETTING
    )
    .fetch_optional(db)
    .await?;

    let base_url = if let Some(q) = q_base_url {
        if let Ok(v) = serde_json::from_value::<String>(q.value.clone()) {
            v
        } else {
            tracing::error!(
                "Could not parse base_url setting as a string, found: {:#?}",
                &q.value
            );
            std::env::var("BASE_URL")
                .ok()
                .unwrap_or_else(|| "http://localhost".to_string())
        }
    } else {
        std::env::var("BASE_URL")
            .ok()
            .unwrap_or_else(|| "http://localhost".to_string())
    };

    let q_oauth = sqlx::query!(
        "SELECT value FROM global_settings WHERE name = $1",
        OAUTH_SETTING
    )
    .fetch_optional(db)
    .await?;

    let oauths = if let Some(q) = q_oauth {
        if let Ok(v) =
            serde_json::from_value::<Option<HashMap<String, OAuthClient>>>(q.value.clone())
        {
            v
        } else {
            tracing::error!(
                "Could not parse oauth setting as a json, found: {:#?}",
                &q.value
            );
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
) {
    if *RESTART_ZOMBIE_JOBS {
        let restarted = sqlx::query!(
                "UPDATE queue SET running = false, started_at = null, logs = logs || '\nRestarted job after not receiving job''s ping for too long the ' || now() || '\n\n' WHERE last_ping < now() - ($1 || ' seconds')::interval AND running = true AND job_kind != $2 AND job_kind != $3 AND same_worker = false RETURNING id, workspace_id, last_ping",
                *ZOMBIE_JOB_TIMEOUT,
                JobKind::Flow as JobKind,
                JobKind::FlowPreview as JobKind,
            )
            .fetch_all(db)
            .await
            .ok()
            .unwrap_or_else(|| vec![]);

        if *METRICS_ENABLED {
            QUEUE_ZOMBIE_RESTART_COUNT.inc_by(restarted.len() as _);
        }
        for r in restarted {
            tracing::info!(
                "restarted zombie job {} {} {}",
                r.id,
                r.workspace_id,
                r.last_ping
            );
        }
    }

    let mut timeout_query = "SELECT * FROM queue WHERE last_ping < now() - ($1 || ' seconds')::interval AND running = true AND job_kind != $2 AND job_kind != $3".to_string();
    if *RESTART_ZOMBIE_JOBS {
        timeout_query.push_str(" AND same_worker = true");
    };
    let timeouts = sqlx::query_as::<_, QueuedJob>(&timeout_query)
        .bind(ZOMBIE_JOB_TIMEOUT.as_str())
        .bind(JobKind::Flow)
        .bind(JobKind::FlowPreview)
        .fetch_all(db)
        .await
        .ok()
        .unwrap_or_else(|| vec![]);

    if *METRICS_ENABLED {
        QUEUE_ZOMBIE_DELETE_COUNT.inc_by(timeouts.len() as _);
    }

    for job in timeouts {
        tracing::info!("timedout zombie job {} {}", job.id, job.workspace_id,);

        // since the job is unrecoverable, the same worker queue should never be sent anything
        let (same_worker_tx_never_used, _same_worker_rx_never_used) = mpsc::channel::<Uuid>(1);

        let token = create_token_for_owner(
            &db,
            &job.workspace_id,
            &job.permissioned_as,
            "ephemeral-zombie-jobs",
            *SCRIPT_TOKEN_EXPIRY,
            &job.email,
        )
        .await
        .expect("could not create job token");

        let client = AuthedClient {
            base_internal_url: base_internal_url.to_string(),
            token,
            workspace: job.workspace_id.to_string(),
            client: OnceCell::new(),
        };

        let last_ping = job.last_ping.clone();
        let _ = handle_job_error(
            db,
            &client,
            &job,
            error::Error::ExecutionErr(format!(
                "Job timed out after no ping from job since {} (ZOMBIE_JOB_TIMEOUT: {})",
                last_ping
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| "no ping".to_string()),
                *ZOMBIE_JOB_TIMEOUT
            )),
            None,
            true,
            same_worker_tx_never_used,
            "",
            rsmq.clone(),
        )
        .await;
    }
}
