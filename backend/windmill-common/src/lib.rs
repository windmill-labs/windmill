/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::net::SocketAddr;

use error::Error;

pub mod apps;
pub mod error;
pub mod external_ip;
pub mod flow_status;
pub mod flows;
pub mod more_serde;
pub mod oauth2;
pub mod scripts;
pub mod users;
pub mod utils;
pub mod variables;

#[cfg(feature = "tracing_init")]
pub mod tracing_init;

pub const DEFAULT_NUM_WORKERS: usize = 3;
pub const DEFAULT_TIMEOUT: i32 = 300;
pub const DEFAULT_SLEEP_QUEUE: u64 = 50;
pub const DEFAULT_MAX_CONNECTIONS: u32 = 100;

#[cfg(feature = "tokio")]
pub async fn shutdown_signal(tx: tokio::sync::broadcast::Sender<()>) -> anyhow::Result<()> {
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
    }
    println!("signal received, starting graceful shutdown");
    let _ = tx.send(());
    Ok(())
}

#[cfg(feature = "prometheus")]
pub async fn serve_metrics(
    addr: SocketAddr,
    mut rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), hyper::Error> {
    use axum::{routing::get, Router};
    axum::Server::bind(&addr)
        .serve(
            Router::new()
                .route("/metrics", get(metrics))
                .into_make_service(),
        )
        .with_graceful_shutdown(async {
            rx.recv().await.ok();
            println!("Graceful shutdown of metrics");
        })
        .await
}

async fn metrics() -> Result<String, Error> {
    let metric_families = prometheus::gather();
    Ok(prometheus::TextEncoder::new()
        .encode_to_string(&metric_families)
        .map_err(anyhow::Error::from)?)
}

#[cfg(feature = "sqlx")]
pub async fn connect_db() -> anyhow::Result<sqlx::Pool<sqlx::Postgres>> {
    use anyhow::Context;

    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| Error::BadConfig("DATABASE_URL env var is missing".to_string()))?;

    let max_connections = match std::env::var("DATABASE_CONNECTIONS") {
        Ok(n) => n.parse::<u32>().context("invalid DATABASE_CONNECTIONS")?,
        Err(_) => DEFAULT_MAX_CONNECTIONS,
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
        .max_connections(max_connections)
        .max_lifetime(Duration::from_secs(30 * 60)) // 30 mins
        .connect(database_url)
        .await
        .map_err(|err| Error::ConnectingToDatabase(err.to_string()))
}

// TODO: Move this elsewhere
pub async fn get_latest_hash_for_path<'c>(
    db: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    w_id: &str,
    script_path: &str,
) -> error::Result<scripts::ScriptHash> {
    let script_hash_o = sqlx::query_scalar!(
        "select hash from script where path = $1 AND (workspace_id = $2 OR workspace_id = \
         'starter') AND
    created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND (workspace_id = $2 OR \
         workspace_id = 'starter')) AND
    deleted = false",
        script_path,
        w_id
    )
    .fetch_optional(db)
    .await?;

    let script_hash = utils::not_found_if_none(script_hash_o, "ScriptHash", script_path)?;

    Ok(scripts::ScriptHash(script_hash))
}
