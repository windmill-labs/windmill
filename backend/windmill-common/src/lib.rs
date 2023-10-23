/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{net::SocketAddr, sync::Arc};

use error::Error;
use scripts::ScriptLang;
use sqlx::{Pool, Postgres};

pub mod apps;
pub mod db;
pub mod error;
pub mod external_ip;
pub mod flow_status;
pub mod flows;
pub mod global_settings;
pub mod jobs;
pub mod more_serde;
pub mod oauth2;
pub mod schedule;
pub mod scripts;
pub mod server;
pub mod users;
pub mod utils;
pub mod variables;
pub mod worker;

#[cfg(feature = "tracing_init")]
pub mod tracing_init;

pub const DEFAULT_MAX_CONNECTIONS_SERVER: u32 = 50;
pub const DEFAULT_MAX_CONNECTIONS_WORKER: u32 = 5;

lazy_static::lazy_static! {
    pub static ref METRICS_ADDR: Option<SocketAddr> = std::env::var("METRICS_ADDR")
    .ok()
    .map(|s| {
        s.parse::<bool>()
            .map(|b| b.then(|| SocketAddr::from(([0, 0, 0, 0], 8001))))
            .or_else(|_| s.parse::<SocketAddr>().map(Some))
    })
    .transpose().ok()
    .flatten()
    .flatten();
    pub static ref METRICS_ENABLED: bool = METRICS_ADDR.is_some();
    pub static ref BASE_URL: Arc<RwLock<String>> = Arc::new(RwLock::new("".to_string()));
    pub static ref IS_READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
}

#[cfg(feature = "tokio")]
pub async fn shutdown_signal(
    tx: tokio::sync::broadcast::Sender<()>,
    mut rx: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    use std::io;
    use tokio::signal::unix::SignalKind;

    async fn terminate() -> io::Result<()> {
        tokio::signal::unix::signal(SignalKind::terminate())?
            .recv()
            .await;
        Ok(())
    }

    tokio::select! {
        _ = terminate() => {},
        _ = tokio::signal::ctrl_c() => {},
        _ = rx.recv() => {
            tracing::info!("shutdown monitor received killpill");
        },
    }
    println!("signal received, starting graceful shutdown");
    let _ = tx.send(());
    Ok(())
}

use tokio::sync::RwLock;
#[cfg(feature = "prometheus")]
use tokio::task::JoinHandle;

#[cfg(feature = "prometheus")]
pub async fn serve_metrics(
    addr: SocketAddr,
    mut rx: tokio::sync::broadcast::Receiver<()>,
    ready_worker_endpoint: bool,
) -> JoinHandle<()> {
    use std::sync::atomic::Ordering;

    use axum::{routing::get, Router};
    use hyper::StatusCode;
    let router = Router::new().route("/metrics", get(metrics));

    let router = if ready_worker_endpoint {
        router.route(
            "/ready",
            get(|| async {
                if IS_READY.load(Ordering::Relaxed) {
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
        if let Err(e) = axum::Server::bind(&addr)
            .serve(router.into_make_service())
            .with_graceful_shutdown(async {
                rx.recv().await.ok();
                println!("Graceful shutdown of metrics");
            })
            .await
        {
            tracing::error!("Error serving metrics: {}", e);
        }
    })
}

async fn metrics() -> Result<String, Error> {
    let metric_families = prometheus::gather();
    Ok(prometheus::TextEncoder::new()
        .encode_to_string(&metric_families)
        .map_err(anyhow::Error::from)?)
}

#[cfg(feature = "sqlx")]
pub async fn connect_db(server_mode: bool) -> anyhow::Result<sqlx::Pool<sqlx::Postgres>> {
    use anyhow::Context;
    use std::env::var;
    use tokio::fs::File;
    use tokio::io::AsyncReadExt;

    let database_url = match var("DATABASE_URL_FILE") {
        Ok(file_path) => {
            let mut file = File::open(file_path).await?;
            let mut contents = String::new();
            file.read_to_string(&mut contents).await?;
            contents.trim().to_string()
        }
        Err(_) => var("DATABASE_URL").map_err(|_| {
            Error::BadConfig(
                "Either DATABASE_URL_FILE or DATABASE_URL env var is missing".to_string(),
            )
        })?,
    };

    let max_connections = match std::env::var("DATABASE_CONNECTIONS") {
        Ok(n) => n.parse::<u32>().context("invalid DATABASE_CONNECTIONS")?,
        Err(_) => {
            if server_mode {
                DEFAULT_MAX_CONNECTIONS_SERVER
            } else {
                DEFAULT_MAX_CONNECTIONS_WORKER
            }
        }
    };

    Ok(connect(&database_url, max_connections).await?)
}

#[cfg(feature = "sqlx")]
pub async fn connect(
    database_url: &str,
    max_connections: u32,
) -> Result<sqlx::Pool<sqlx::Postgres>, error::Error> {
    use std::time::Duration;

    sqlx::postgres::PgPoolOptions::new()
        .min_connections(3)
        .max_connections(max_connections)
        .max_lifetime(Duration::from_secs(30 * 60)) // 30 mins
        .connect(database_url)
        .await
        .map_err(|err| Error::ConnectingToDatabase(err.to_string()))
}

type Tag = String;

pub type DB = Pool<Postgres>;

pub async fn get_latest_deployed_hash_for_path(
    db: &DB,
    w_id: &str,
    script_path: &str,
) -> error::Result<(
    scripts::ScriptHash,
    Option<Tag>,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    ScriptLang,
    Option<bool>,
)> {
    let r_o = sqlx::query!(
        "select hash, tag, concurrent_limit, concurrency_time_window_s, cache_ttl, language as \"language: ScriptLang\", dedicated_worker from script where path = $1 AND workspace_id = $2 AND
    created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND workspace_id = $2 AND
    deleted = false AND lock IS not NULL AND lock_error_logs IS NULL)",
        script_path,
        w_id
    )
    .fetch_optional(db)
    .await?;

    let script = utils::not_found_if_none(r_o, "deployed script", script_path)?;

    Ok((
        scripts::ScriptHash(script.hash),
        script.tag,
        script.concurrent_limit,
        script.concurrency_time_window_s,
        script.cache_ttl,
        script.language,
        script.dedicated_worker,
    ))
}

pub async fn get_latest_hash_for_path<'c>(
    db: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    w_id: &str,
    script_path: &str,
) -> error::Result<(
    scripts::ScriptHash,
    Option<Tag>,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    ScriptLang,
    Option<bool>,
)> {
    let r_o = sqlx::query!(
        "select hash, tag, concurrent_limit, concurrency_time_window_s, cache_ttl, language as \"language: ScriptLang\", dedicated_worker from script where path = $1 AND workspace_id = $2 AND
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
        script.concurrent_limit,
        script.concurrency_time_window_s,
        script.cache_ttl,
        script.language,
        script.dedicated_worker,
    ))
}
