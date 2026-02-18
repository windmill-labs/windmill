/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    hash::{Hash, Hasher},
    net::SocketAddr,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use tokio::{spawn, sync::broadcast};

use ee_oss::CriticalErrorChannel;
use error::Error;
use scripts::ScriptLang;
use sqlx::{Acquire, Postgres};

pub mod agent_workers;
#[cfg(feature = "bedrock")]
pub mod ai_bedrock;
pub mod ai_providers;
pub mod ai_types;
pub mod apps;
pub mod assets;
pub mod audit;
pub mod auth;
#[cfg(feature = "benchmark")]
pub mod bench;
pub mod cache;
pub mod client;
pub mod db;
#[cfg(all(feature = "enterprise", feature = "private"))]
mod db_iam_ee;
#[cfg(feature = "private")]
pub mod ee;
pub mod ee_oss;
#[cfg(feature = "private")]
pub mod email_ee;
pub mod email_oss;
pub mod error;
pub mod external_ip;
pub mod flow_conversations;
pub mod flow_status;
pub mod flows;
pub mod global_settings;
pub mod indexer;
pub mod instance_config;
pub mod job_metrics;
pub mod min_version;
pub mod notify_events;
pub mod runtime_assets;
pub mod workspace_dependencies;

#[cfg(feature = "private")]
pub mod git_sync_ee;
pub mod git_sync_oss;
pub mod jobs;
pub mod jwt;
pub mod more_serde;
pub mod oauth2;
#[cfg(all(feature = "enterprise", feature = "openidconnect", feature = "private"))]
pub mod oidc_ee;
#[cfg(all(feature = "enterprise", feature = "openidconnect"))]
pub mod oidc_oss;
#[cfg(feature = "private")]
pub mod otel_ee;
pub mod otel_oss;
pub mod queue;
pub mod result_stream;
pub mod runnable_settings;
pub mod schedule;
pub mod schema;
pub mod scripts;
pub mod secret_backend;
pub mod server;
#[cfg(feature = "private")]
pub mod stats_ee;
pub mod stats_oss;
pub mod stream;
#[cfg(feature = "private")]
pub mod teams_ee;
pub mod teams_oss;
pub mod tracing_init;
pub mod triggers;
pub mod usernames;
pub mod users;
pub mod utils;
pub mod variables;
pub mod webhook;
pub mod worker;
pub mod worker_group_job_stats;
pub mod workspaces;

pub const DEFAULT_MAX_CONNECTIONS_SERVER: u32 = 50;
pub const DEFAULT_MAX_CONNECTIONS_WORKER: u32 = 5;
pub const DEFAULT_MAX_CONNECTIONS_INDEXER: u32 = 5;

pub const DEFAULT_HUB_BASE_URL: &str = "https://hub.windmill.dev";
pub const PRIVATE_HUB_MIN_VERSION: i32 = 10_000_000;
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
    pub static ref DYNAMIC_INPUT_CACHE: Cache<String, Arc<jobs::DynamicInput>> = Cache::new(1000);
    pub static ref DEPLOYED_SCRIPT_INFO_CACHE: Cache<(String, i64), ScriptHashInfo<ScriptRunnableSettingsHandle>> = Cache::new(1000);
    pub static ref FLOW_INFO_CACHE: Cache<(String, i64), FlowVersionInfo> = Cache::new(1000);

    pub static ref QUIET_LOGS: bool = std::env::var("QUIET_LOGS").map(|s| s.parse::<bool>().unwrap_or(false)).unwrap_or(false);

}

const LATEST_VERSION_ID_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(60);

pub async fn shutdown_signal(
    tx: KillpillSender,
    mut rx: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    async fn terminate() -> std::io::Result<()> {
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

    spawn(async move {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        tokio::select! {
            _ = terminate() => {
                tracing::error!("2nd shutdown monitor received terminate");
            },
            _ = tokio::signal::ctrl_c() => {
                tracing::error!("2nd shutdown monitor received ctrl-c");
            },
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::error!("2nd shutdown monitor received ctrl-c")
            },
        }

        tracing::info!("Second terminate signal received, forcefully exiting");

        let handle = tokio::runtime::Handle::current();
        let metrics = handle.metrics();
        tracing::info!(
            "Alive tasks: {}, global queue depth: {}",
            metrics.num_alive_tasks(),
            metrics.global_queue_depth()
        );

        std::process::exit(1);
    });

    tracing::info!("signal received, starting graceful shutdown");
    let _ = tx.send();

    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(24 * 7 * 60 * 60)).await;
        tracing::info!("Forcefully exiting after 7 days");
        std::process::exit(1);
    });

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

#[derive(Serialize, Debug)]
pub struct PrepareQueryColumnInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
}

#[derive(Serialize, Debug)]
pub struct PrepareQueryResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<PrepareQueryColumnInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct PgDatabase {
    pub host: String,
    pub user: Option<String>,
    pub password: Option<String>,
    pub port: Option<u16>,
    pub sslmode: Option<String>,
    pub dbname: String,
    pub root_certificate_pem: Option<String>,
}

// Wrapper enum to hold either Tls or NoTls connection
pub enum TokioPgConnection {
    Tls(
        tokio_postgres::Connection<
            tokio_postgres::Socket,
            postgres_native_tls::TlsStream<tokio_postgres::Socket>,
        >,
    ),
    NoTls(tokio_postgres::Connection<tokio_postgres::Socket, tokio_postgres::tls::NoTlsStream>),
}

impl Future for TokioPgConnection {
    type Output = Result<(), tokio_postgres::Error>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        // SAFETY: We're simply projecting the Pin from the outer enum to the inner connection field.
        // The inner connection is never moved out, so this is safe.
        unsafe {
            match self.get_unchecked_mut() {
                TokioPgConnection::Tls(conn) => std::pin::Pin::new_unchecked(conn).poll(cx),
                TokioPgConnection::NoTls(conn) => std::pin::Pin::new_unchecked(conn).poll(cx),
            }
        }
    }
}

impl PgDatabase {
    pub fn to_uri(&self) -> String {
        let sslmode = match self.sslmode.as_deref() {
            Some("allow") => "prefer".to_string(),
            Some("require") | Some("verify-ca") | Some("verify-full") => "require".to_string(),
            Some(s) => s.to_string(),
            None => "prefer".to_string(),
        };
        format!(
            "postgres://{user}:{password}@{host}:{port}/{dbname}?sslmode={sslmode}",
            user = urlencoding::encode(&self.user.as_deref().unwrap_or("postgres")),
            password = urlencoding::encode(&self.password.as_deref().unwrap_or("")),
            host = &self.host,
            port = self.port.unwrap_or(5432),
            dbname = self.dbname,
            sslmode = sslmode
        )
    }

    pub fn to_conn_str(&self) -> String {
        format!(
            "dbname={dbname} {user} host={host} {password} {port} {sslmode}",
            dbname = self.dbname,
            user = self
                .user
                .as_ref()
                .map(|u| format!("user={}", urlencoding::encode(u)))
                .unwrap_or_default(),
            host = self.host,
            password = self
                .password
                .as_ref()
                .map(|p| format!("password={}", urlencoding::encode(p)))
                .unwrap_or_default(),
            port = self.port.map(|p| format!("port={}", p)).unwrap_or_default(),
            sslmode = self
                .sslmode
                .as_ref()
                .map(|s| format!("sslmode={}", s.clone()))
                .unwrap_or_default(),
        )
    }

    pub async fn connect(
        &self,
    ) -> Result<(tokio_postgres::Client, TokioPgConnection), error::Error> {
        use native_tls::{Certificate, TlsConnector};
        use postgres_native_tls::MakeTlsConnector;
        use tokio_postgres::tls::NoTls;
        let ssl_mode_is_require = matches!(
            self.sslmode.as_deref(),
            Some("require") | Some("verify-ca") | Some("verify-full")
        );

        if ssl_mode_is_require {
            tracing::info!("Creating new connection");
            let mut connector = TlsConnector::builder();
            if let Some(root_certificate_pem) = &self.root_certificate_pem {
                if !root_certificate_pem.is_empty() {
                    connector.add_root_certificate(
                        Certificate::from_pem(root_certificate_pem.as_bytes()).map_err(|e| {
                            error::Error::BadConfig(format!("Invalid Certs: {e:#}"))
                        })?,
                    );
                } else {
                    connector.danger_accept_invalid_certs(true);
                    connector.danger_accept_invalid_hostnames(true);
                }
            } else {
                connector
                    .danger_accept_invalid_certs(true)
                    .danger_accept_invalid_hostnames(true);
            }

            let (client, connection) = tokio::time::timeout(
                std::time::Duration::from_secs(20),
                tokio_postgres::connect(
                    &self.to_uri(),
                    MakeTlsConnector::new(connector.build().map_err(to_anyhow)?),
                ),
            )
            .await
            .map_err(to_anyhow)?
            .map_err(to_anyhow)?;

            Ok((client, TokioPgConnection::Tls(connection)))
        } else {
            tracing::info!("Creating new connection");
            let (client, connection) = tokio::time::timeout(
                std::time::Duration::from_secs(20),
                tokio_postgres::connect(&self.to_uri(), NoTls),
            )
            .await
            .map_err(to_anyhow)?
            .map_err(to_anyhow)?;

            Ok((client, TokioPgConnection::NoTls(connection)))
        }
    }

    pub fn parse_uri(url: &str) -> Result<Self, Error> {
        let parsed_url = url::Url::parse(url)
            .map_err(|_| Error::BadConfig("Invalid PostgreSQL URL".to_string()))?;

        let username = parsed_url.username().to_string();
        let username = urlencoding::decode(&username)
            .map_err(to_anyhow)?
            .to_string();
        let password = parsed_url.password().map(|p| p.to_string());
        let password = match password {
            Some(p) => Some(urlencoding::decode(&p).map_err(to_anyhow)?.to_string()),
            None => None,
        };
        let host = parsed_url
            .host_str()
            .ok_or_else(|| Error::BadConfig("Missing host in PostgreSQL URL".to_string()))?
            .to_string();
        let port = parsed_url.port();
        let dbname = parsed_url.path().trim_start_matches('/').to_string();
        let mut sslmode = None;
        for query in parsed_url.query_pairs() {
            if query.0 == "sslmode" {
                sslmode = Some(query.1.to_string());
            }
        }

        Ok(PgDatabase {
            user: if username.is_empty() {
                None
            } else {
                Some(username)
            },
            password,
            host,
            port,
            dbname,
            sslmode,
            root_certificate_pem: None,
        })
    }
}

#[derive(Clone)]
pub enum DatabaseUrl {
    #[cfg(all(feature = "enterprise", feature = "private"))]
    IamRds(std::sync::Arc<tokio::sync::RwLock<db_iam_ee::IamRdsUrl>>),
    Static(String),
}

impl DatabaseUrl {
    /// Get the database URL as a string.
    /// Note: For IAM RDS, this returns the original URL (for metadata extraction).
    /// For actual database connections, use connect_options() instead.
    pub async fn as_str(&self) -> String {
        match self {
            #[cfg(all(feature = "enterprise", feature = "private"))]
            DatabaseUrl::IamRds(rds_url) => {
                let guard = rds_url.read().await;
                guard.as_str().to_string()
            }
            DatabaseUrl::Static(url) => url.clone(),
        }
    }

    /// Get PgConnectOptions for this database URL.
    /// For IAM RDS, this returns options built directly from the token to avoid double-encoding
    /// issues with temporary credentials (IRSA/Pod Identity).
    /// For static URLs, this parses the URL string.
    pub async fn connect_options(&self) -> Result<sqlx::postgres::PgConnectOptions, Error> {
        match self {
            #[cfg(all(feature = "enterprise", feature = "private"))]
            DatabaseUrl::IamRds(rds_url) => {
                let guard = rds_url.read().await;
                Ok(guard.connect_options())
            }
            DatabaseUrl::Static(url) => sqlx::postgres::PgConnectOptions::from_str(url)
                .map_err(|e| Error::InternalErr(format!("Failed to parse database URL: {}", e))),
        }
    }

    pub async fn refresh(&self) -> anyhow::Result<()> {
        match self {
            #[cfg(all(feature = "enterprise", feature = "private"))]
            DatabaseUrl::IamRds(rds_url) => rds_url.write().await.refresh().await,
            DatabaseUrl::Static(_) => Ok(()),
        }
    }
}

static DATABASE_URL_CACHE: tokio::sync::OnceCell<DatabaseUrl> = tokio::sync::OnceCell::const_new();

pub async fn get_database_url() -> Result<DatabaseUrl, Error> {
    let database_url = DATABASE_URL_CACHE
        .get_or_try_init(|| async {
            use std::env::var;
            use tokio::fs::File;
            use tokio::io::AsyncReadExt;

            let url = match var("DATABASE_URL_FILE") {
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
            }?;

            let parsed_url = url::Url::parse(&url)?;

            if parsed_url.password().is_some_and(|x| x == "iamrds") {
                let region = var("AWS_REGION").map_err(|_| {
                    Error::BadConfig(
                        "AWS_REGION env var is required for IAM RDS authentication".to_string(),
                    )
                })?;

                tracing::info!("iamrds mode detected, generating IAM RDS URL for region: {region}");
                #[cfg(all(feature = "enterprise", feature = "private"))]
                {
                    let rds_url = db_iam_ee::generate_database_url(&url, &region)
                        .await
                        .map_err(|e| {
                            Error::InternalErr(format!(
                                "Failed to generate IAM database URL: {}",
                                e
                            ))
                        })?;
                    tracing::info!("IAM RDS URL generated successfully");
                    Ok::<DatabaseUrl, Error>(DatabaseUrl::IamRds(std::sync::Arc::new(
                        tokio::sync::RwLock::new(rds_url),
                    )))
                }

                #[cfg(not(all(feature = "enterprise", feature = "private")))]
                {
                    return Err(Error::BadConfig(
                        "IAM RDS authentication is not enabled in OSS mode".to_string(),
                    ));
                }
            } else {
                Ok::<DatabaseUrl, Error>(DatabaseUrl::Static(url.to_string()))
            }
        })
        .await?;

    // Check if we need to refresh and do so if necessary
    #[cfg(all(feature = "enterprise", feature = "private"))]
    if let DatabaseUrl::IamRds(ref rds_url_lock) = database_url {
        // Check if refresh is needed
        let needs_refresh = {
            let read_guard = rds_url_lock.read().await;
            read_guard.needs_refresh()
        };

        // If refresh is needed, acquire write lock and refresh
        if needs_refresh {
            let mut write_guard = rds_url_lock.write().await;
            // Double-check after acquiring write lock (another task might have refreshed)
            if write_guard.needs_refresh() {
                write_guard.refresh().await.map_err(|e| {
                    Error::InternalErr(format!("Failed to refresh IAM token: {}", e))
                })?;
            }
        }
    }

    // Return the URL string
    Ok(database_url.clone())
}

type Tag = String;

pub use db::DB;

use crate::{
    auth::{PermsCache, FLOW_PERMS_CACHE, HASH_PERMS_CACHE},
    db::{AuthedRef, UserDbWithAuthed},
    error::to_anyhow,
    scripts::{ScriptHash, ScriptRunnableSettingsHandle, ScriptRunnableSettingsInline},
};

#[derive(Clone)]
pub struct ExpiringLatestVersionId {
    id: i64,
    expires_at: std::time::Instant,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ScriptHashInfo<SR> {
    pub path: String,
    pub hash: i64,
    pub tag: Option<String>,
    pub cache_ttl: Option<i32>,
    pub cache_ignore_s3_path: Option<bool>,
    pub language: ScriptLang,
    pub dedicated_worker: Option<bool>,
    pub priority: Option<i16>,
    pub delete_after_use: Option<bool>,
    pub timeout: Option<i32>,
    pub has_preprocessor: Option<bool>,
    pub on_behalf_of_email: Option<String>,
    pub created_by: String,
    #[sqlx(flatten)]
    pub runnable_settings: SR,
}

impl ScriptHashInfo<ScriptRunnableSettingsHandle> {
    pub async fn prefetch_cached<'a>(
        self,
        db: &DB,
    ) -> error::Result<ScriptHashInfo<ScriptRunnableSettingsInline>> {
        let rs =
            runnable_settings::from_handle(self.runnable_settings.runnable_settings_handle, db)
                .await?;
        let (debouncing_settings, concurrency_settings) =
            runnable_settings::prefetch_cached(&rs, db).await?;

        Ok(ScriptHashInfo {
            path: self.path,
            hash: self.hash,
            tag: self.tag,
            cache_ttl: self.cache_ttl,
            cache_ignore_s3_path: self.cache_ignore_s3_path,
            language: self.language,
            dedicated_worker: self.dedicated_worker,
            priority: self.priority,
            delete_after_use: self.delete_after_use,
            timeout: self.timeout,
            has_preprocessor: self.has_preprocessor,
            on_behalf_of_email: self.on_behalf_of_email,
            created_by: self.created_by,
            runnable_settings: ScriptRunnableSettingsInline {
                concurrency_settings: concurrency_settings.maybe_fallback(
                    self.runnable_settings.concurrency_key,
                    self.runnable_settings.concurrent_limit,
                    self.runnable_settings.concurrency_time_window_s,
                ),
                debouncing_settings: debouncing_settings.maybe_fallback(
                    self.runnable_settings.debounce_key,
                    self.runnable_settings.debounce_delay_s,
                ),
            },
        })
    }
}

pub fn get_latest_deployed_hash_for_path<'e>(
    db: Option<UserDbWithAuthed<'e, AuthedRef<'e>>>,
    db2: DB,
    w_id: &'e str,
    script_path: &'e str,
) -> impl Future<Output = error::Result<ScriptHashInfo<ScriptRunnableSettingsHandle>>> + Send + 'e {
    async move {
        let cache_key = (w_id.to_string(), script_path.to_string());
        let mut computed_hash = None;
        let hash = match DEPLOYED_SCRIPT_HASH_CACHE.get(&cache_key) {
            Some(cached_hash)
                if cached_hash.expires_at > std::time::Instant::now()
                    && db.as_ref().is_none_or(|x| {
                        let r = HASH_PERMS_CACHE
                            .check_perms_in_cache(x.authed, ScriptHash(cached_hash.id));
                        computed_hash = Some(r.1);
                        return r.0;
                    }) =>
            {
                tracing::debug!(
                    "Using cached script hash {} for {script_path}",
                    cached_hash.id
                );
                cached_hash.id
            }
            _ => {
                tracing::debug!("Fetching script hash for {script_path}");
                let hash = if let Some(db) = db {
                    let authed = db.authed;
                    let mut conn = db.acquire().await?;
                    let hash = get_latest_script_hash(&mut *conn, script_path, w_id).await?;
                    if let Some(hash) = hash {
                        HASH_PERMS_CACHE.insert(
                            computed_hash.unwrap_or_else(|| PermsCache::compute_hash(authed)),
                            ScriptHash(hash),
                        );
                    } else {
                        let mut conn = db2.acquire().await?;
                        let exists = get_latest_script_hash(&mut *conn, script_path, w_id)
                            .await?
                            .is_some();
                        if exists {
                            return Err(Error::NotAuthorized(format!("You are not authorized to access this script: {script_path} (but it exists). Your permissions are: {:?}", authed)));
                        }
                    }
                    hash
                } else {
                    let mut conn = db2.acquire().await?;
                    get_latest_script_hash(&mut *conn, script_path, w_id).await?
                };

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

        get_script_info_for_hash(None, &db2, w_id, hash).await
    }
}

pub async fn get_latest_script_hash<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    script_path: &'e str,
    w_id: &'e str,
) -> error::Result<Option<i64>> {
    let hash = sqlx::query_scalar!(
        "select hash from script where path = $1 AND workspace_id = $2 AND deleted = false AND lock IS not NULL AND lock_error_logs IS NULL ORDER BY created_at DESC LIMIT 1",
        script_path,
        w_id
    )
    .fetch_optional(db)
    .await?;
    return Ok(hash);
}

pub async fn get_script_info_for_hash<'e, E: sqlx::PgExecutor<'e>>(
    db_authed: Option<UserDbWithAuthed<'e, AuthedRef<'e>>>,
    db: E,
    w_id: &str,
    hash: i64,
) -> error::Result<ScriptHashInfo<ScriptRunnableSettingsHandle>> {
    let key = (w_id.to_string(), hash);

    let mut computed_hash = None;
    match DEPLOYED_SCRIPT_INFO_CACHE.get(&key) {
        Some(info)
            if db_authed.as_ref().is_none_or(|x| {
                let r = HASH_PERMS_CACHE.check_perms_in_cache(x.authed, scripts::ScriptHash(hash));
                computed_hash = Some(r.1);
                return r.0;
            }) =>
        {
            tracing::debug!("Using cached deployed script info for {hash}");
            Ok(info)
        }
        _ => {
            tracing::debug!("Fetching deployed script info for {hash}");
            let info = if let Some(db_authed) = db_authed {
                let mut conn = db_authed.acquire().await?;
                let hash_info = get_script_info_for_hash_inner(&mut *conn, w_id, hash).await?;
                if hash_info.is_some() {
                    HASH_PERMS_CACHE.insert(
                        computed_hash.unwrap_or_else(|| PermsCache::compute_hash(db_authed.authed)),
                        ScriptHash(hash),
                    );
                }
                hash_info
            } else {
                get_script_info_for_hash_inner(db, w_id, hash).await?
            };

            let info = utils::not_found_if_none(info, "script", &hash.to_string())?;

            DEPLOYED_SCRIPT_INFO_CACHE.insert(key, info.clone());

            Ok(info)
        }
    }
}

async fn get_script_info_for_hash_inner<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    w_id: &str,
    hash: i64,
) -> error::Result<Option<ScriptHashInfo<ScriptRunnableSettingsHandle>>> {
    let r = sqlx::query_as::<_, ScriptHashInfo<ScriptRunnableSettingsHandle>>(
        "SELECT
                hash,
                tag,
                concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                debounce_key,
                debounce_delay_s,
                runnable_settings_handle,
                cache_ttl,
                cache_ignore_s3_path,
                language,
                dedicated_worker,
                priority,
                delete_after_use,
                timeout,
                has_preprocessor,
                on_behalf_of_email,
                created_by,
                path
            FROM script WHERE hash = $1 AND workspace_id = $2",
    )
    .bind(hash)
    .bind(w_id)
    .fetch_optional(db)
    .await?;
    Ok(r)
}
#[derive(Clone)]
pub struct FlowVersionInfo {
    pub version: i64,
    pub tag: Option<String>,
    pub early_return: Option<String>,
    pub has_preprocessor: Option<bool>,
    pub chat_input_enabled: Option<bool>,
    pub on_behalf_of_email: Option<String>,
    pub edited_by: String,
    pub dedicated_worker: Option<bool>,
}

struct CachedFlowPath(String);

impl Into<u64> for CachedFlowPath {
    fn into(self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.0.hash(&mut hasher);
        hasher.finish()
    }
}
pub fn get_latest_flow_version_id_for_path<
    'a,
    'e,
    A: sqlx::Acquire<'e, Database = Postgres> + Send + 'a,
>(
    db_authed: Option<UserDbWithAuthed<'e, AuthedRef<'e>>>,
    db: A,
    w_id: &'a str,
    path: &'a str,
    use_cache: bool,
) -> impl Future<Output = error::Result<i64>> + Send + 'a
where
    'e: 'a,
{
    // as instructed in the docstring of sqlx::Acquire
    async move {
        let cache_key = (w_id.to_string(), path.to_string());
        let cached_version = if use_cache {
            FLOW_VERSION_CACHE.get(&cache_key)
        } else {
            None
        };
        let mut computed_hash: Option<_> = None;

        let version = match cached_version {
            Some(cached_version)
                if cached_version.expires_at > std::time::Instant::now()
                    && db_authed.as_ref().is_none_or(|x| {
                        let r = FLOW_PERMS_CACHE
                            .check_perms_in_cache(x.authed, CachedFlowPath(path.to_string()));
                        computed_hash = Some(r.1);
                        return r.0;
                    }) =>
            {
                tracing::debug!("Using cached flow version {} for {path}", cached_version.id);
                cached_version.id
            }
            _ => {
                tracing::debug!("Fetching flow version for {path}");
                let version = if let Some(db_authed) = db_authed {
                    let mut conn = db_authed.acquire().await?;
                    let r = get_latest_flow_version_for_path(&mut *conn, w_id, path).await?;
                    if r.is_some() {
                        FLOW_PERMS_CACHE.insert(
                            computed_hash
                                .unwrap_or_else(|| PermsCache::compute_hash(db_authed.authed)),
                            CachedFlowPath(path.to_string()),
                        );
                    } else {
                        let mut conn = db.acquire().await?;
                        let exists = get_latest_flow_version_for_path(&mut *conn, w_id, path)
                            .await?
                            .is_some();
                        if exists {
                            return Err(Error::NotAuthorized(format!(
                                "You are not authorized to access this flow: {path} (but it exists). Your permissions are: {:?}",
                                db_authed.authed
                            )));
                        }
                    }
                    r
                } else {
                    let mut conn = db.acquire().await?;
                    get_latest_flow_version_for_path(&mut *conn, w_id, path).await?
                };

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
        Ok(version)
    }
}

pub fn get_flow_version_info_from_version<
    'a,
    'e,
    A: sqlx::Acquire<'e, Database = Postgres> + Send + 'a,
>(
    db: A,
    version: i64,
    w_id: &'a str,
    path: &'a str,
) -> impl Future<Output = error::Result<FlowVersionInfo>> + Send + 'a {
    async move {
        // as instructed in the docstring of sqlx::Acquire
        let key = (w_id.to_string(), version);
        match FLOW_INFO_CACHE.get(&key) {
            Some(info) => {
                tracing::debug!("Using cached flow version info for {version} ({path})");
                Ok(info)
            }
            _ => {
                tracing::debug!("Fetching flow version info for {version} ({path})");
                let mut conn = db.acquire().await?;
                let flow_info =
                        sqlx::query_as!(
                            FlowVersionInfo,
                            r#"
                                SELECT
                                    flow_version.id AS version,
                                    flow_version.value->>'early_return' as early_return,
                                    flow_version.value->>'preprocessor_module' IS NOT NULL as has_preprocessor,
                                    (flow_version.value->>'chat_input_enabled')::boolean as chat_input_enabled,
                                    flow.tag,
                                    flow.dedicated_worker,
                                    flow.on_behalf_of_email,
                                    flow.edited_by
                                FROM
                                    flow_version
                                INNER JOIN flow
                                    ON flow.path = flow_version.path AND
                                       flow.workspace_id = flow_version.workspace_id
                                WHERE
                                    flow_version.workspace_id = $1 AND
                                    flow_version.path = $2 AND
                                    flow_version.id = $3
                            "#,
                            w_id,
                            path,
                            version,
                        )
                        .fetch_optional(&mut *conn)
                        .await?;

                let info = utils::not_found_if_none(flow_info, "flow", path)?;

                FLOW_INFO_CACHE.insert(key, info.clone());

                Ok(info)
            }
        }
    }
}

pub async fn get_latest_flow_version_info_for_path<'e>(
    db_authed: Option<UserDbWithAuthed<'e, AuthedRef<'e>>>,
    db: &DB,
    w_id: &'e str,
    path: &'e str,
    use_cache: bool,
) -> error::Result<FlowVersionInfo> {
    // as instructed in the docstring of sqlx::Acquire
    let version =
        get_latest_flow_version_id_for_path(db_authed, &db.clone(), w_id, path, use_cache).await?;
    get_flow_version_info_from_version(db, version, w_id, path).await
}

async fn get_latest_flow_version_for_path<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    w_id: &str,
    path: &str,
) -> error::Result<Option<i64>> {
    let version = sqlx::query_scalar!(
        "SELECT flow_version.id from flow
        INNER JOIN flow_version
        ON flow_version.id = flow.versions[array_upper(flow.versions, 1)]
        WHERE flow.path = $1 and flow.workspace_id = $2",
        path,
        w_id
    )
    .fetch_optional(db)
    .await?;
    Ok(version)
}

pub async fn get_latest_hash_for_path<'c, E: sqlx::PgExecutor<'c>>(
    db: E,
    w_id: &str,
    script_path: &str,
    require_locked: bool,
) -> error::Result<(
    scripts::ScriptHash,
    Option<Tag>,
    Option<String>,
    Option<i32>,
    Option<i32>,
    Option<String>,
    Option<i32>,
    Option<i32>,
    Option<bool>,
    ScriptLang,
    Option<bool>,
    Option<i16>,
    Option<i32>,
    Option<String>,
    String,
    Option<i64>,
)> {
    let r_o = sqlx::query!(
            "select hash, tag, concurrency_key, concurrent_limit, concurrency_time_window_s, debounce_key, debounce_delay_s, cache_ttl, cache_ignore_s3_path, runnable_settings_handle, language as \"language: ScriptLang\", dedicated_worker, priority, timeout, on_behalf_of_email, created_by FROM script
             WHERE path = $1 AND workspace_id = $2 AND archived = false AND (lock IS NOT NULL OR $3 = false)
             ORDER BY created_at DESC LIMIT 1",
            script_path,
            w_id,
            require_locked
        )
        .fetch_optional(db)
        .await?;

    let script = utils::not_found_if_none(r_o, "script", script_path)?;

    Ok((
        scripts::ScriptHash(script.hash),
        script.tag,
        script.concurrency_key,
        script.concurrent_limit,
        script.concurrency_time_window_s,
        script.debounce_key,
        script.debounce_delay_s,
        script.cache_ttl,
        script.cache_ignore_s3_path,
        script.language,
        script.dedicated_worker,
        script.priority,
        script.timeout,
        script.on_behalf_of_email,
        script.created_by,
        script.runnable_settings_handle,
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
