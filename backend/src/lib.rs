/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use anyhow::Context;
use argon2::Argon2;
use axum::{handler::Handler, middleware::from_extractor, routing::get, Extension, Router};
use db::DB;
use futures::FutureExt;
use git_version::git_version;
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::trace::TraceLayer;

extern crate magic_crypt;

extern crate dotenv;

mod audit;
mod client;
mod db;
mod error;
mod external_ip;
mod flows;
mod granular_acls;
mod groups;
mod jobs;
mod js_eval;
mod more_serde;
mod oauth2;
mod parser;
mod parser_go;
mod parser_go_ast;
mod parser_go_scanner;
mod parser_go_token;
mod parser_py;
mod parser_ts;
mod resources;
mod schedule;
mod scripts;
mod static_assets;
mod tracing_init;
mod users;
mod utils;
mod variables;
mod worker;
mod worker_flow;
mod worker_ping;
mod workspaces;

use error::Error;

use crate::{
    db::UserDB,
    error::to_anyhow,
    oauth2::{build_oauth_clients, SlackVerifier},
    tracing_init::{MyMakeSpan, MyOnResponse},
    utils::rd_string,
};

pub use crate::tracing_init::initialize_tracing;
pub use crate::worker::WorkerConfig;

const GIT_VERSION: &str = git_version!(args = ["--tag", "--always"], fallback = "unknown-version");
pub const DEFAULT_NUM_WORKERS: usize = 3;
pub const DEFAULT_TIMEOUT: i32 = 300;
pub const DEFAULT_SLEEP_QUEUE: u64 = 50;
pub const DEFAULT_MAX_CONNECTIONS: u32 = 100;

pub async fn migrate_db(db: &DB) -> anyhow::Result<()> {
    db::migrate(db).await?;
    Ok(())
}

pub async fn connect_db() -> anyhow::Result<DB> {
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| Error::BadConfig("DATABASE_URL env var is missing".to_string()))?;

    let max_connections = match std::env::var("DATABASE_CONNECTIONS") {
        Ok(n) => n.parse::<u32>().context("invalid DATABASE_CONNECTIONS")?,
        Err(_) => DEFAULT_MAX_CONNECTIONS,
    };

    Ok(db::connect(&database_url, max_connections).await?)
}

struct BaseUrl(String);
struct IsSecure(bool);
struct CloudHosted(bool);

pub async fn run_server(
    db: DB,
    addr: SocketAddr,
    base_url: String,
    mut rx: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    let user_db = UserDB::new(db.clone());

    let auth_cache = Arc::new(users::AuthCache::new(db.clone()));
    let argon2 = Arc::new(Argon2::default());
    let basic_clients = Arc::new(build_oauth_clients(&base_url).await?);
    let slack_verifier = Arc::new(
        std::env::var("SLACK_SIGNING_SECRET")
            .ok()
            .map(|x| SlackVerifier::new(x).unwrap()),
    );
    let http_client = reqwest::ClientBuilder::new()
        .user_agent("windmill/beta")
        .build()
        .map_err(to_anyhow)?;
    let middleware_stack = ServiceBuilder::new()
        .layer(
            TraceLayer::new_for_http()
                .on_response(MyOnResponse {})
                .make_span_with(MyMakeSpan {})
                .on_request(()),
        )
        .layer(Extension(db.clone()))
        .layer(Extension(user_db))
        .layer(Extension(auth_cache.clone()))
        .layer(Extension(basic_clients))
        .layer(Extension(Arc::new(BaseUrl(base_url.to_string()))))
        .layer(Extension(Arc::new(CloudHosted(
            std::env::var("CLOUD_HOSTED").is_ok(),
        ))))
        .layer(Extension(Arc::new(IsSecure(
            base_url.starts_with("https://"),
        ))))
        .layer(Extension(http_client))
        .layer(CookieManagerLayer::new());
    // build our application with a route
    let app = Router::new()
        .nest(
            "/api",
            Router::new()
                .nest(
                    "/w/:workspace_id",
                    Router::new()
                        .nest("/scripts", scripts::workspaced_service())
                        .nest("/jobs", jobs::workspaced_service())
                        .nest(
                            "/users",
                            users::workspaced_service().layer(Extension(argon2.clone())),
                        )
                        .nest("/variables", variables::workspaced_service())
                        .nest("/oauth", oauth2::workspaced_service())
                        .nest("/resources", resources::workspaced_service())
                        .nest("/schedules", schedule::workspaced_service())
                        .nest("/groups", groups::workspaced_service())
                        .nest("/audit", audit::workspaced_service())
                        .nest("/acls", granular_acls::workspaced_service())
                        .nest("/workspaces", workspaces::workspaced_service())
                        .nest("/flows", flows::workspaced_service()),
                )
                .nest("/workspaces", workspaces::global_service())
                .nest(
                    "/users",
                    users::global_service().layer(Extension(argon2.clone())),
                )
                .nest("/workers", worker_ping::global_service())
                .nest("/scripts", scripts::global_service())
                .nest("/flows", flows::global_service())
                .nest("/schedules", schedule::global_service())
                .route_layer(from_extractor::<users::Authed>())
                .route_layer(from_extractor::<users::Tokened>())
                .nest("/w/:workspace_id/jobs", jobs::global_service())
                .nest(
                    "/auth",
                    users::make_unauthed_service().layer(Extension(argon2)),
                )
                .nest(
                    "/oauth",
                    oauth2::global_service().layer(Extension(slack_verifier)),
                )
                .route("/version", get(git_v))
                .route("/openapi.yaml", get(openapi)),
        )
        .fallback(static_assets::static_handler.into_service())
        .layer(middleware_stack);

    let instance_name = rd_string(5);

    tracing::info!(addr = %addr.to_string(), instance = %instance_name, "server started listening");
    let server = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(async {
            rx.recv().await.ok();
            println!("Graceful shutdown of server");
        });

    tokio::spawn(async move { auth_cache.monitor().await });

    server.await?;
    Ok(())
}

pub fn monitor_db(db: &DB, timeout: i32, rx: tokio::sync::broadcast::Receiver<()>) {
    let db1 = db.clone();
    let db2 = db.clone();

    let rx2 = rx.resubscribe();

    tokio::spawn(async move { worker::handle_zombie_jobs_periodically(&db1, timeout, rx).await });
    tokio::spawn(async move { users::delete_expired_items_perdiodically(&db2, rx2).await });
}

pub async fn run_workers(
    db: DB,
    addr: SocketAddr,
    timeout: i32,
    num_workers: i32,
    sleep_queue: u64,
    worker_config: WorkerConfig,
    rx: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    let instance_name = rd_string(5);
    let monitor = tokio_metrics::TaskMonitor::new();

    let ip = external_ip::get_ip().await.unwrap_or_else(|e| {
        tracing::warn!(error = e.to_string(), "failed to get external IP");
        "unretrievable IP".to_string()
    });

    let mut handles = Vec::with_capacity(num_workers as usize);

    for i in 1..(num_workers + 1) {
        let db1 = db.clone();
        let instance_name = instance_name.clone();
        let worker_name = format!("dt-worker-{}-{}", &instance_name, rd_string(5));
        let ip = ip.clone();
        let rx = rx.resubscribe();
        let worker_config = worker_config.clone();
        handles.push(tokio::spawn(monitor.instrument(async move {
            tracing::info!(addr = %addr.to_string(), worker = %worker_name, "starting worker");
            worker::run_worker(
                &db1,
                timeout,
                &instance_name,
                worker_name,
                i as u64,
                num_workers as u64,
                &ip,
                sleep_queue,
                worker_config,
                rx,
            )
            .await
        })));
    }

    futures::future::try_join_all(handles).await?;
    Ok(())
}

async fn git_v() -> &'static str {
    GIT_VERSION
}

async fn openapi() -> &'static str {
    include_str!("../openapi.yaml")
}

pub async fn shutdown_signal(tx: tokio::sync::broadcast::Sender<()>) -> anyhow::Result<()> {
    tokio::signal::ctrl_c().await.unwrap();
    println!("signal received, starting graceful shutdown");
    let _ = tx.send(());
    Ok(())
}

pub async fn serve_metrics(
    addr: SocketAddr,
    mut rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<(), hyper::Error> {
    axum::Server::bind(&addr)
        .serve(
            Router::new()
                .route("/metrics", get(metrics))
                .into_make_service(),
        )
        .with_graceful_shutdown(rx.recv().map(drop))
        .await
}

async fn metrics() -> Result<String, Error> {
    let metric_families = prometheus::gather();
    Ok(prometheus::TextEncoder::new()
        .encode_to_string(&metric_families)
        .map_err(anyhow::Error::from)?)
}
