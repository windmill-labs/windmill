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

const GIT_VERSION: &str = git_version!(args = ["--tag", "--always"], fallback = "unknown-version");
const DEFAULT_NUM_WORKERS: usize = 3;
const DEFAULT_PORT: u16 = 8000;

mod ee;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    windmill_common::tracing_init::initialize_tracing();

    let num_workers = std::env::var("NUM_WORKERS")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(DEFAULT_NUM_WORKERS as i32);

    let metrics_addr: Option<SocketAddr> = std::env::var("METRICS_ADDR")
        .ok()
        .map(|s| {
            s.parse::<bool>()
                .map(|b| b.then(|| SocketAddr::from(([0, 0, 0, 0], 8001))))
                .or_else(|_| s.parse::<SocketAddr>().map(Some))
        })
        .transpose()?
        .flatten();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|x| x.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT as u16);
    let base_internal_url: String = std::env::var("BASE_INTERNAL_URL")
        .unwrap_or_else(|_| format!("http://localhost:{}", port.to_string()));

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

    if server_mode || num_workers > 0 {
        let addr = SocketAddr::from(([0, 0, 0, 0], port));

        let server_f = async {
            if server_mode {
                windmill_api::run_server(db.clone(), addr, rx.resubscribe()).await?;
            }
            Ok(()) as anyhow::Result<()>
        };

        let workers_f = async {
            if num_workers > 0 {
                #[cfg(feature = "enterprise")]
                tracing::info!(
                    "
##############################
Windmill Enterprise Edition {GIT_VERSION}
##############################"
                );

                #[cfg(not(feature = "enterprise"))]
                tracing::info!(
                    "
##############################
Windmill Community Edition {GIT_VERSION}
##############################"
                );

                display_config(vec![
                    "DISABLE_NSJAIL",
                    "DISABLE_SERVER",
                    "NUM_WORKERS",
                    "METRICS_ADDR",
                    "JSON_FMT",
                    "BASE_URL",
                    "BASE_INTERNAL_URL",
                    "TIMEOUT",
                    "SLEEP_QUEUE",
                    "MAX_LOG_SIZE",
                    "PORT",
                    "KEEP_JOB_DIR",
                    "S3_CACHE_BUCKET",
                    "TAR_CACHE_RATE",
                    "COOKIE_DOMAIN",
                    "PYTHON_PATH",
                    "DENO_PATH",
                    "GO_PATH",
                    "PIP_INDEX_URL",
                    "PIP_EXTRA_INDEX_URL",
                    "PIP_TRUSTED_HOST",
                    "PATH",
                    "HOME",
                    "DATABASE_CONNECTIONS",
                    "TIMEOUT_WAIT_RESULT",
                    "QUEUE_LIMIT_WAIT_RESULT",
                    "DENO_AUTH_TOKENS",
                    "DENO_FLAGS",
                    "PIP_LOCAL_DEPENDENCIES",
                    "ADDITIONAL_PYTHON_PATHS",
                    "INCLUDE_HEADERS",
                    "WHITELIST_WORKSPACES",
                    "BLACKLIST_WORKSPACES",
                    "NEW_USER_WEBHOOK",
                    "CLOUD_HOSTED",
                ]);

                run_workers(
                    db.clone(),
                    rx.resubscribe(),
                    num_workers,
                    base_internal_url.clone(),
                )
                .await?;
            }
            Ok(()) as anyhow::Result<()>
        };

        let monitor_f = async {
            if server_mode {
                monitor_db(&db, rx.resubscribe(), &base_internal_url);
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

fn display_config(envs: Vec<&str>) {
    tracing::info!(
        "config: {}",
        envs.iter()
            .filter(|env| std::env::var(env).is_ok())
            .map(|env| {
                format!(
                    "{}: {}",
                    env,
                    std::env::var(env).unwrap_or_else(|_| "not set".to_string())
                )
            })
            .collect::<Vec<String>>()
            .join(", ")
    )
}

pub fn monitor_db(
    db: &Pool<Postgres>,
    rx: tokio::sync::broadcast::Receiver<()>,
    base_internal_url: &str,
) {
    let db1 = db.clone();
    let db2 = db.clone();

    let rx2 = rx.resubscribe();
    let base_internal_url = base_internal_url.to_string();
    tokio::spawn(async move {
        windmill_worker::handle_zombie_jobs_periodically(&db1, rx, &base_internal_url).await
    });
    tokio::spawn(async move { windmill_api::delete_expired_items_perdiodically(&db2, rx2).await });
}

pub async fn run_workers(
    db: Pool<Postgres>,
    rx: tokio::sync::broadcast::Receiver<()>,
    num_workers: i32,
    base_internal_url: String,
) -> anyhow::Result<()> {
    let license_key = std::env::var("LICENSE_KEY").ok();
    #[cfg(feature = "enterprise")]
    ee::verify_license_key(license_key)?;

    #[cfg(not(feature = "enterprise"))]
    if license_key.is_some() {
        panic!("License key is required ONLY for the enterprise edition");
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
        let base_internal_url = base_internal_url.clone();
        handles.push(tokio::spawn(monitor.instrument(async move {
            tracing::info!(worker = %worker_name, "starting worker");
            windmill_worker::run_worker(
                &db1,
                &instance_name,
                worker_name,
                i as u64,
                &ip,
                rx,
                &base_internal_url,
            )
            .await
        })));
    }

    futures::future::try_join_all(handles).await?;
    Ok(())
}
