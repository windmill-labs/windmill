/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::net::SocketAddr;

use git_version::git_version;
use sqlx::{Pool, Postgres};
use windmill_common::utils::rd_string;
use windmill_worker::WorkerConfig;

const GIT_VERSION: &str = git_version!(args = ["--tag", "--always"], fallback = "unknown-version");

mod ee;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    windmill_common::tracing_init::initialize_tracing();

    let num_workers = std::env::var("NUM_WORKERS")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(windmill_common::DEFAULT_NUM_WORKERS as i32);

    let metrics_addr: Option<SocketAddr> = std::env::var("METRICS_ADDR")
        .ok()
        .map(|s| {
            s.parse::<bool>()
                .map(|b| b.then(|| SocketAddr::from(([0, 0, 0, 0], 8001))))
                .or_else(|_| s.parse::<SocketAddr>().map(Some))
        })
        .transpose()?
        .flatten();

    let server_mode = !std::env::var("DISABLE_SERVER")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false);

    let db = windmill_common::connect_db(server_mode).await?;

    if server_mode {
        windmill_api::migrate_db(&db).await?;
    }

    let (tx, rx) = tokio::sync::broadcast::channel::<()>(3);
    let shutdown_signal = windmill_common::shutdown_signal(tx);

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost".to_string());

    let base_internal_url =
        std::env::var("BASE_INTERNAL_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    let timeout = std::env::var("TIMEOUT")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(windmill_common::DEFAULT_TIMEOUT);

    if server_mode || num_workers > 0 {
        let addr = SocketAddr::from(([0, 0, 0, 0], 8000));

        let base_url2 = base_url.clone();
        let server_f = async {
            if server_mode {
                windmill_api::run_server(db.clone(), addr, base_url, rx.resubscribe()).await?;
            }
            Ok(()) as anyhow::Result<()>
        };

        let base_url = base_url2.clone();
        let workers_f = async {
            if num_workers > 0 {
                let sleep_queue = std::env::var("SLEEP_QUEUE")
                    .ok()
                    .and_then(|x| x.parse::<u64>().ok())
                    .unwrap_or(windmill_common::DEFAULT_SLEEP_QUEUE);
                let disable_nuser = std::env::var("DISABLE_NUSER")
                    .ok()
                    .and_then(|x| x.parse::<bool>().ok())
                    .unwrap_or(false);
                let disable_nsjail = std::env::var("DISABLE_NSJAIL")
                    .ok()
                    .and_then(|x| x.parse::<bool>().ok())
                    .unwrap_or(true);
                let keep_job_dir = std::env::var("KEEP_JOB_DIR")
                    .ok()
                    .and_then(|x| x.parse::<bool>().ok())
                    .unwrap_or(false);
                let license_key = std::env::var("LICENSE_KEY").ok();
                let sync_bucket = std::env::var("S3_CACHE_BUCKET")
                    .ok()
                    .map(|e| Some(e))
                    .unwrap_or(None);

                #[cfg(feature = "enterprise")]
                tracing::info!(
                        "
##############################
Windmill Enterprise Edition {GIT_VERSION} LICENSE_KEY: {license_key:?}, S3_CACHE_BUCKET: {sync_bucket:?}
##############################"
                    );

                #[cfg(not(feature = "enterprise"))]
                tracing::info!(
                    "
##############################
Windmill Community Edition {GIT_VERSION}
##############################"
                );

                tracing::info!(
                    "DISABLE_NSJAIL: {disable_nsjail}, DISABLE_NUSER: {disable_nuser}, BASE_URL: \
                     {base_url}, SLEEP_QUEUE: {sleep_queue}, NUM_WORKERS: {num_workers}, TIMEOUT: \
                     {timeout}, KEEP_JOB_DIR: {keep_job_dir}"
                );

                run_workers(
                    db.clone(),
                    addr,
                    timeout,
                    num_workers,
                    sleep_queue,
                    WorkerConfig {
                        disable_nsjail,
                        disable_nuser,
                        base_internal_url,
                        base_url,
                        keep_job_dir,
                    },
                    rx.resubscribe(),
                    sync_bucket,
                    license_key,
                )
                .await?;
            }
            Ok(()) as anyhow::Result<()>
        };

        let base_url = base_url2;
        let monitor_f = async {
            if server_mode {
                monitor_db(&db, timeout, base_url, rx.resubscribe());
            }
            Ok(()) as anyhow::Result<()>
        };

        let metrics_f = async {
            match metrics_addr {
                Some(addr) => windmill_common::serve_metrics(addr, rx.resubscribe())
                    .await
                    .map_err(anyhow::Error::from),
                None => Ok(()),
            }
        };

        futures::try_join!(shutdown_signal, server_f, metrics_f, workers_f, monitor_f)?;
    }
    Ok(())
}

pub fn monitor_db(
    db: &Pool<Postgres>,
    timeout: i32,
    base_url: String,
    rx: tokio::sync::broadcast::Receiver<()>,
) {
    let db1 = db.clone();
    let db2 = db.clone();

    let rx2 = rx.resubscribe();

    tokio::spawn(async move {
        windmill_worker::handle_zombie_jobs_periodically(&db1, timeout, &base_url, rx).await
    });
    tokio::spawn(async move { windmill_api::delete_expired_items_perdiodically(&db2, rx2).await });
}

pub async fn run_workers(
    db: Pool<Postgres>,
    addr: SocketAddr,
    timeout: i32,
    num_workers: i32,
    sleep_queue: u64,
    worker_config: WorkerConfig,
    rx: tokio::sync::broadcast::Receiver<()>,
    mut periodic_script: Option<String>,
    license_key: Option<String>,
) -> anyhow::Result<()> {
    #[cfg(feature = "enterprise")]
    ee::verify_license_key(license_key)?;

    #[cfg(not(feature = "enterprise"))]
    if license_key.is_some() {
        panic!("License key is required ONLY for the enterprise edition");
    }
    #[cfg(not(feature = "enterprise"))]
    if !worker_config.disable_nsjail {
        tracing::warn!(
            "NSJAIL to sandbox process in untrusted environments is an enterprise feature but allowed to be used for testing purposes"
        );
    }

    let instance_name = rd_string(5);
    let monitor = tokio_metrics::TaskMonitor::new();

    let ip = windmill_common::external_ip::get_ip()
        .await
        .unwrap_or_else(|e| {
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
        let wp = periodic_script.take();
        handles.push(tokio::spawn(monitor.instrument(async move {
            tracing::info!(addr = %addr.to_string(), worker = %worker_name, "starting worker");
            windmill_worker::run_worker(
                &db1,
                timeout,
                &instance_name,
                worker_name,
                i as u64,
                num_workers as u64,
                &ip,
                sleep_queue,
                worker_config,
                wp,
                rx,
            )
            .await
        })));
    }

    futures::future::try_join_all(handles).await?;
    Ok(())
}
