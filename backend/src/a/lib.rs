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

use crate::{
    db::UserDB,
    error::to_anyhow,
    oauth2::{build_oauth_clients, SlackVerifier},
    tracing_init::{MyMakeSpan, MyOnResponse},
    utils::rd_string,
};

extern crate magic_crypt;

extern crate dotenv;

mod client;
mod error;
mod external_ip;
mod js_eval;
mod more_serde;
mod parser;
mod parser_go;
mod parser_go_ast;
mod parser_go_scanner;
mod parser_go_token;
mod parser_py;
mod parser_ts;
mod tracing_init;
mod utils;

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
