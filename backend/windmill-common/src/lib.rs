/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use quick_cache::sync::Cache;
use std::{
    future::Future,
    net::SocketAddr,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use tokio::sync::broadcast;

use ee_oss::CriticalErrorChannel;
use error::Error;
use scripts::ScriptLang;
use sqlx::{Pool, Postgres};

pub mod agent_workers;
pub mod apps;
pub mod auth;
#[cfg(feature = "benchmark")]
pub mod bench;
pub mod cache;
pub mod client;
pub mod db;
mod ee;
pub mod ee_oss;
mod email_ee;
pub mod email_oss;
pub mod error;
pub mod external_ip;
pub mod flow_status;
pub mod flows;
pub mod global_settings;
pub mod indexer;
pub mod job_metrics;
#[cfg(feature = "parquet")]
mod job_s3_helpers_ee;
#[cfg(feature = "parquet")]
pub mod job_s3_helpers_oss;

#[cfg(all(feature = "enterprise", feature = "openidconnect"))]
mod oidc_ee;
#[cfg(all(feature = "enterprise", feature = "openidconnect"))]
pub mod oidc_oss;

pub mod jobs;
pub mod jwt;
pub mod more_serde;
pub mod oauth2;
mod otel_ee;
pub mod otel_oss;
pub mod queue;
pub mod s3_helpers;
pub mod schedule;
pub mod schema;
pub mod scripts;
pub mod server;
mod stats_ee;
pub mod stats_oss;
mod teams_ee;
pub mod teams_oss;
pub mod tracing_init;
pub mod users;
pub mod utils;
pub mod variables;
pub mod worker;
pub mod workspaces;
pub mod triggers;

pub const DEFAULT_MAX_CONNECTIONS_SERVER: u32 = 50;
pub const DEFAULT_MAX_CONNECTIONS_WORKER: u32 = 5;
pub const DEFAULT_MAX_CONNECTIONS_INDEXER: u32 = 5;

pub const DEFAULT_HUB_BASE_URL: &str = "https://hub.windmill.dev";
pub const SERVICE_LOG_RETENTION_SECS: i64 = 60 * 60 * 24 * 14; // 2 weeks retention period for logs

#[macro_export]
macro_rules! add_time {
    ($bench:expr, $name:expr) => {
        #[cfg(feature = "benchmark")]
        {
            $bench.add_timing($name);
            // println!("{}: {:?}", $z, $y.elapsed());
        }
    };
}

lazy_static::lazy_static! {
    pub static ref METRICS_PORT: u16 = std::env::var("METRICS_PORT")
    .ok()
    .and_then(|s| s.parse::<u16>().ok())
    .unwrap_or(8001);

    pub static ref METRICS_ADDR: SocketAddr = std::env::var("METRICS_ADDR")
    .ok()
    .map(|s| {
        s.parse::<bool>()
            .map(|b| b.then(|| SocketAddr::from(([0, 0, 0, 0], *METRICS_PORT))))
            .or_else(|_| s.parse::<SocketAddr>().map(Some))
    })
    .transpose().ok()
    .flatten()
    .flatten()
    .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], *METRICS_PORT)));

    pub static ref METRICS_ENABLED: AtomicBool = AtomicBool::new(std::env::var("METRICS_PORT").is_ok() || std::env::var("METRICS_ADDR").is_ok());

    pub static ref OTEL_METRICS_ENABLED: AtomicBool = AtomicBool::new(std::env::var("OTEL_METRICS").is_ok());
    pub static ref OTEL_TRACING_ENABLED: AtomicBool = AtomicBool::new(std::env::var("OTEL_TRACING").is_ok());
    pub static ref OTEL_LOGS_ENABLED: AtomicBool = AtomicBool::new(std::env::var("OTEL_LOGS").is_ok());


    pub static ref METRICS_DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

    pub static ref CRITICAL_ALERT_MUTE_UI_ENABLED: AtomicBool = AtomicBool::new(false);

    pub static ref BASE_URL: Arc<RwLock<String>> = Arc::new(RwLock::new("".to_string()));
    pub static ref IS_READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    pub static ref HUB_BASE_URL: Arc<RwLock<String>> = Arc::new(RwLock::new(DEFAULT_HUB_BASE_URL.to_string()));


    pub static ref CRITICAL_ERROR_CHANNELS: Arc<RwLock<Vec<CriticalErrorChannel>>> = Arc::new(RwLock::new(vec![]));
    pub static ref CRITICAL_ALERTS_ON_DB_OVERSIZE: Arc<RwLock<Option<f32>>> = Arc::new(RwLock::new(None));

    pub static ref JOB_RETENTION_SECS: Arc<RwLock<i64>> = Arc::new(RwLock::new(0));

    pub static ref MONITOR_LOGS_ON_OBJECT_STORE: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));

    pub static ref INSTANCE_NAME: String = rd_string(5);

    pub static ref DEPLOYED_SCRIPT_HASH_CACHE: Cache<(String, String), ExpiringLatestVersionId> = Cache::new(1000);
    pub static ref FLOW_VERSION_CACHE: Cache<(String, String), ExpiringLatestVersionId> = Cache::new(1000);
    pub static ref DEPLOYED_SCRIPT_INFO_CACHE: Cache<(String, i64), ScriptHashInfo> = Cache::new(1000);
    pub static ref FLOW_INFO_CACHE: Cache<(String, i64), FlowVersionInfo> = Cache::new(1000);

}

const LATEST_VERSION_ID_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(60);

pub async fn shutdown_signal(
    tx: KillpillSender,
    mut rx: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    use std::io;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    async fn terminate() -> io::Result<()> {
        use tokio::signal::unix::SignalKind;
        tokio::signal::unix::signal(SignalKind::terminate())?
            .recv()
            .await;
        Ok(())
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    tokio::select! {
        _ = terminate() => {
            tracing::info!("shutdown monitor received terminate");
        },
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("shutdown monitor received ctrl-c");
        },
        _ = rx.recv() => {
            tracing::info!("shutdown monitor received killpill");
        },
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = rx.recv() => {
            tracing::info!("shutdown monitor received killpill");
        },
    }

    tracing::info!("signal received, starting graceful shutdown");
    let _ = tx.send();
    Ok(())
}

use tokio::sync::RwLock;
use utils::rd_string;

#[cfg(feature = "prometheus")]
pub async fn serve_metrics(
    addr: SocketAddr,
    mut rx: tokio::sync::broadcast::Receiver<()>,
    ready_worker_endpoint: bool,
    metrics_endpoint: bool,
) -> anyhow::Result<()> {
    if !metrics_endpoint && !ready_worker_endpoint {
        return Ok(());
    }
    use axum::{
        routing::{get, post},
        Router,
    };
    use hyper::StatusCode;
    let router = Router::new();

    let router = if metrics_endpoint {
        router
            .route("/metrics", get(metrics))
            .route("/reset", post(reset))
    } else {
        router
    };

    let router = if ready_worker_endpoint {
        router.route(
            "/ready",
            get(|| async {
                if IS_READY.load(std::sync::atomic::Ordering::Relaxed) {
                    (StatusCode::OK, "ready")
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, "not ready")
                }
            }),
        )
    } else {
        router
    };

    tokio::spawn(async move {
        tracing::info!("Serving metrics at: {addr}");
        let listener = tokio::net::TcpListener::bind(addr).await;
        if let Err(e) = listener {
            tracing::error!("Error binding to metrics address: {}", e);
            return;
        }
        if let Err(e) = axum::serve(listener.unwrap(), router.into_make_service())
            .with_graceful_shutdown(async move {
                rx.recv().await.ok();
                tracing::info!("Graceful shutdown of metrics");
            })
            .await
        {
            tracing::error!("Error serving metrics: {}", e);
        }
    })
    .await?;
    Ok(())
}

#[cfg(feature = "prometheus")]
async fn metrics() -> Result<String, Error> {
    let metric_families = prometheus::gather();
    Ok(prometheus::TextEncoder::new()
        .encode_to_string(&metric_families)
        .map_err(anyhow::Error::from)?)
}

#[cfg(feature = "prometheus")]
async fn reset() -> () {
    todo!()
}

pub async fn get_database_url() -> Result<String, Error> {
    use std::env::var;
    use tokio::fs::File;
    use tokio::io::AsyncReadExt;
    match var("DATABASE_URL_FILE") {
        Ok(file_path) => {
            let mut file = File::open(file_path).await?;
            let mut contents = String::new();
            file.read_to_string(&mut contents).await?;
            Ok(contents.trim().to_string())
        }
        Err(_) => var("DATABASE_URL").map_err(|_| {
            Error::BadConfig(
                "Either DATABASE_URL_FILE or DATABASE_URL env var is missing".to_string(),
            )
        }),
    }
}

pub async fn initial_connection() -> Result<sqlx::Pool<sqlx::Postgres>, error::Error> {
    let database_url = get_database_url().await?;
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect_with(sqlx::postgres::PgConnectOptions::from_str(&database_url)?)
        .await
        .map_err(|err| Error::ConnectingToDatabase(err.to_string()))
}

pub async fn connect_db(
    server_mode: bool,
    indexer_mode: bool,
    worker_mode: bool,
) -> anyhow::Result<sqlx::Pool<sqlx::Postgres>> {
    use anyhow::Context;

    let database_url = get_database_url().await?;

    let max_connections = match std::env::var("DATABASE_CONNECTIONS") {
        Ok(n) => n.parse::<u32>().context("invalid DATABASE_CONNECTIONS")?,
        Err(_) => {
            if server_mode {
                DEFAULT_MAX_CONNECTIONS_SERVER
            } else if indexer_mode {
                DEFAULT_MAX_CONNECTIONS_INDEXER
            } else {
                DEFAULT_MAX_CONNECTIONS_WORKER
                    + std::env::var("NUM_WORKERS")
                        .ok()
                        .map(|x| x.parse().ok())
                        .flatten()
                        .unwrap_or(1)
                    - 1
            }
        }
    };

    Ok(connect(&database_url, max_connections, worker_mode).await?)
}

pub async fn connect(
    database_url: &str,
    max_connections: u32,
    worker_mode: bool,
) -> Result<sqlx::Pool<sqlx::Postgres>, error::Error> {
    use std::time::Duration;

    sqlx::postgres::PgPoolOptions::new()
        .min_connections((max_connections / 5).clamp(3, max_connections))
        .max_connections(max_connections)
        .max_lifetime(Duration::from_secs(30 * 60)) // 30 mins
        .after_connect(move |conn, _| {
            if worker_mode {
                Box::pin(async move {
                    sqlx::query("SET enable_seqscan = OFF;")
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            } else {
                Box::pin(async move { Ok(()) })
            }
        })
        .connect_with(
            sqlx::postgres::PgConnectOptions::from_str(database_url)?.statement_cache_capacity(400),
        )
        .await
        .map_err(|err| Error::ConnectingToDatabase(err.to_string()))
}

type Tag = String;

pub type DB = Pool<Postgres>;

#[derive(Clone)]
pub struct ExpiringLatestVersionId {
    id: i64,
    expires_at: std::time::Instant,
}

#[derive(Clone)]
pub struct ScriptHashInfo {
    pub path: String,
    pub hash: i64,
    pub tag: Option<String>,
    pub concurrency_key: Option<String>,
    pub concurrent_limit: Option<i32>,
    pub concurrency_time_window_s: Option<i32>,
    pub cache_ttl: Option<i32>,
    pub language: ScriptLang,
    pub dedicated_worker: Option<bool>,
    pub priority: Option<i16>,
    pub delete_after_use: Option<bool>,
    pub timeout: Option<i32>,
    pub has_preprocessor: Option<bool>,
    pub on_behalf_of_email: Option<String>,
    pub created_by: String,
}

pub fn get_latest_deployed_hash_for_path<
    'a,
    'e,
    E: sqlx::Acquire<'e, Database = Postgres> + Send + 'a,
>(
    db: E,
    w_id: &'a str,
    script_path: &'a str,
) -> impl Future<Output = error::Result<ScriptHashInfo>> + Send + 'a {
    async move {
        let mut conn = db.acquire().await?;
        let cache_key = (w_id.to_string(), script_path.to_string());

        let hash = match DEPLOYED_SCRIPT_HASH_CACHE.get(&cache_key) {
            Some(cached_hash) if cached_hash.expires_at > std::time::Instant::now() => {
                tracing::debug!(
                    "Using cached script hash {} for {script_path}",
                    cached_hash.id
                );
                cached_hash.id
            }
            _ => {
                tracing::debug!("Fetching script hash for {script_path}");
                let hash = sqlx::query_scalar!( 
                    "select hash from script where path = $1 AND workspace_id = $2 AND deleted = false AND lock IS not NULL AND lock_error_logs IS NULL ORDER BY created_at DESC LIMIT 1",
                    script_path,
                    w_id
                )
                .fetch_optional(&mut *conn)
                .await?;

                let hash = utils::not_found_if_none(hash, "script", script_path)?;

                DEPLOYED_SCRIPT_HASH_CACHE.insert(
                    cache_key,
                    ExpiringLatestVersionId {
                        id: hash,
                        expires_at: std::time::Instant::now() + LATEST_VERSION_ID_CACHE_TTL,
                    },
                );

                hash
            }
        };

        get_script_info_for_hash(&mut *conn, w_id, hash).await
    }
}

pub async fn get_script_info_for_hash<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    w_id: &str,
    hash: i64,
) -> error::Result<ScriptHashInfo> {
    let key = (w_id.to_string(), hash);

    match DEPLOYED_SCRIPT_INFO_CACHE.get(&key) {
        Some(info) => {
            tracing::debug!("Using cached deployed script info for {hash}");
            Ok(info)
        }
        _ => {
            tracing::debug!("Fetching deployed script info for {hash}");
            let info = sqlx::query_as!(
                    ScriptHashInfo,
                    "select hash, tag, concurrency_key, concurrent_limit, concurrency_time_window_s, cache_ttl, language as \"language: ScriptLang\", dedicated_worker, priority, delete_after_use, timeout, has_preprocessor, on_behalf_of_email, created_by, path from script where hash = $1 AND workspace_id = $2",
                    hash,
                    w_id
                )
                .fetch_optional(db)
                .await?;

            let info = utils::not_found_if_none(info, "script", &hash.to_string())?;

            DEPLOYED_SCRIPT_INFO_CACHE.insert(key, info.clone());

            Ok(info)
        }
    }
}

#[derive(Clone)]
pub struct FlowVersionInfo {
    pub version: i64,
    pub tag: Option<String>,
    pub early_return: Option<String>,
    pub has_preprocessor: Option<bool>,
    pub on_behalf_of_email: Option<String>,
    pub edited_by: String,
    pub dedicated_worker: Option<bool>,
}

pub fn get_latest_flow_version_info_for_path<
    'a,
    'e,
    A: sqlx::Acquire<'e, Database = Postgres> + Send + 'a,
>(
    db: A,
    w_id: &'a str,
    path: &'a str,
    use_cache: bool,
) -> impl Future<Output = error::Result<FlowVersionInfo>> + Send + 'a {
    // as instructed in the docstring of sqlx::Acquire
    async move {
        let mut conn = db.acquire().await?;

        let cache_key = (w_id.to_string(), path.to_string());
        let cached_version = if use_cache {
            FLOW_VERSION_CACHE.get(&cache_key)
        } else {
            None
        };

        let version = match cached_version {
            Some(cached_version) if cached_version.expires_at > std::time::Instant::now() => {
                tracing::debug!("Using cached flow version {} for {path}", cached_version.id);
                cached_version.id
            }
            _ => {
                tracing::debug!("Fetching flow version for {path}");
                let version = sqlx::query_scalar!(
                    "SELECT flow_version.id from flow
                    INNER JOIN flow_version
                    ON flow_version.id = flow.versions[array_upper(flow.versions, 1)]
                    WHERE flow.path = $1 and flow.workspace_id = $2",
                    path,
                    w_id
                )
                .fetch_optional(&mut *conn)
                .await?;

                let version = utils::not_found_if_none(version, "flow", path)?;

                FLOW_VERSION_CACHE.insert(
                    cache_key,
                    ExpiringLatestVersionId {
                        id: version,
                        expires_at: std::time::Instant::now() + LATEST_VERSION_ID_CACHE_TTL,
                    },
                );

                version
            }
        };

        let key = (w_id.to_string(), version);

        match FLOW_INFO_CACHE.get(&key) {
            Some(info) => {
                tracing::debug!("Using cached flow version info for {version} ({path})");
                Ok(info)
            }
            _ => {
                tracing::debug!("Fetching flow version info for {version} ({path})");
                let info = sqlx::query_as!(
                    FlowVersionInfo,
                    "SELECT tag, dedicated_worker, flow_version.value->>'early_return' as early_return, flow_version.value->>'preprocessor_module' IS NOT NULL as has_preprocessor, on_behalf_of_email, edited_by, flow_version.id AS version
                    FROM flow
                    INNER JOIN flow_version
                        ON flow_version.id = $3
                    WHERE flow.path = $1 and flow.workspace_id = $2",
                path,
                w_id,
                version
            )
                .fetch_optional(&mut *conn)
                .await?;

                let info = utils::not_found_if_none(info, "flow", path)?;

                FLOW_INFO_CACHE.insert(key, info.clone());

                Ok(info)
            }
        }
    }
}

pub async fn get_latest_hash_for_path<'c>(
    db: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    w_id: &str,
    script_path: &str,
) -> error::Result<(
    scripts::ScriptHash,
    Option<Tag>,
    Option<String>,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    ScriptLang,
    Option<bool>,
    Option<i16>,
    Option<i32>,
    Option<String>,
    String,
)> {
    let r_o = sqlx::query!(
        "select hash, tag, concurrency_key, concurrent_limit, concurrency_time_window_s, cache_ttl, language as \"language: ScriptLang\", dedicated_worker, priority, timeout, on_behalf_of_email, created_by FROM script where path = $1 AND workspace_id = $2 AND
    created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND workspace_id = $2 AND
    deleted = false AND archived = false)",
        script_path,
        w_id
    )
    .fetch_optional(&mut **db)
    .await?;

    let script = utils::not_found_if_none(r_o, "script", script_path)?;

    Ok((
        scripts::ScriptHash(script.hash),
        script.tag,
        script.concurrency_key,
        script.concurrent_limit,
        script.concurrency_time_window_s,
        script.cache_ttl,
        script.language,
        script.dedicated_worker,
        script.priority,
        script.timeout,
        script.on_behalf_of_email,
        script.created_by,
    ))
}

pub struct KillpillSender {
    tx: broadcast::Sender<()>,
    already_sent: Arc<AtomicBool>,
}

impl Clone for KillpillSender {
    fn clone(&self) -> Self {
        KillpillSender { tx: self.tx.clone(), already_sent: self.already_sent.clone() }
    }
}

impl KillpillSender {
    pub fn new(capacity: usize) -> (Self, broadcast::Receiver<()>) {
        let (tx, rx) = broadcast::channel(capacity);
        let sender = KillpillSender { tx, already_sent: Arc::new(AtomicBool::new(false)) };
        (sender, rx)
    }

    pub fn clone(&self) -> Self {
        KillpillSender { tx: self.tx.clone(), already_sent: self.already_sent.clone() }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }

    // Try to send the killpill if it hasn't been sent already
    pub fn send(&self) -> bool {
        // Check if it's already been sent, and if not, set the flag to true
        if !self.already_sent.swap(true, Ordering::SeqCst) {
            // We're the first to set it to true, so send the signal
            if let Err(e) = self.tx.send(()) {
                tracing::error!("failed to send killpill: {:?}", e);
            }
            true
        } else {
            // Signal was already sent
            false
        }
    }

    // // Force send a signal regardless of previous sends
    // fn force_send(&self) -> Result<usize, broadcast::error::SendError<()>> {
    //     self.already_sent.store(true, Ordering::SeqCst);
    //     self.tx.send(())
    // }

    // // Check if the killpill has been sent
    // fn is_sent(&self) -> bool {
    //     self.already_sent.load(Ordering::SeqCst)
    // }
}
