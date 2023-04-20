/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{atomic::Ordering, Arc},
};

use git_version::git_version;
use monitor::handle_zombie_jobs_periodically;
use sqlx::{Pool, Postgres};
use tokio::{fs::DirBuilder, sync::RwLock};
use windmill_common::{utils::rd_string, IS_READY, METRICS_ADDR};
use windmill_worker::{DENO_CACHE_DIR, GO_CACHE_DIR, PIP_CACHE_DIR, S3_CACHE_BUCKET};

const GIT_VERSION: &str = git_version!(args = ["--tag", "--always"], fallback = "unknown-version");
const DEFAULT_NUM_WORKERS: usize = 3;
const DEFAULT_PORT: u16 = 8000;
const DEFAULT_SERVER_BIND_ADDR: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

mod ee;
mod monitor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    windmill_common::tracing_init::initialize_tracing();

    let num_workers = std::env::var("NUM_WORKERS")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(DEFAULT_NUM_WORKERS as i32);

    let metrics_addr: Option<SocketAddr> = *METRICS_ADDR;

    let server_bind_address: IpAddr = std::env::var("SERVER_BIND_ADDR")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(IpAddr::from(DEFAULT_SERVER_BIND_ADDR));

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

    let rsmq_config = std::env::var("REDIS_URL").ok().map(|x| {
        let url = x.parse::<url::Url>().unwrap();
        let mut config = rsmq_async::RsmqOptions { ..Default::default() };

        config.host = url.host_str().expect("redis host required").to_owned();
        config.password = url.password().map(|s| s.to_owned());
        config.db = url
            .path_segments()
            .and_then(|mut segments| segments.next())
            .and_then(|segment| segment.parse().ok())
            .unwrap_or(0);
        config.ns = url
            .query_pairs()
            .find(|s| s.0 == "rsmq_namespace")
            .map(|s| s.1)
            .unwrap_or(std::borrow::Cow::Borrowed("rsmq"))
            .into_owned();
        config.port = url.port().unwrap_or(6379).to_string();

        config
    });

    let db = windmill_common::connect_db(server_mode).await?;

    let rsmq = if let Some(config) = rsmq_config {
        let mut rsmq = rsmq_async::MultiplexedRsmq::new(config).await.unwrap();

        let _ = rsmq_async::RsmqConnection::create_queue(&mut rsmq, "main_queue", None, None, None)
            .await;
        Some(rsmq)
    } else {
        None
    };

    if server_mode {
        windmill_api::migrate_db(&db).await?;
    }

    let (tx, rx) = tokio::sync::broadcast::channel::<()>(3);
    let shutdown_signal = windmill_common::shutdown_signal(tx);

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
        "ZOMBIE_JOB_TIMEOUT",
        "RESTART_ZOMBIE_JOBS",
        "SLEEP_QUEUE",
        "MAX_LOG_SIZE",
        "SERVER_BIND_ADDR",
        "PORT",
        "KEEP_JOB_DIR",
        "S3_CACHE_BUCKET",
        "TAR_CACHE_RATE",
        "COOKIE_DOMAIN",
        "PYTHON_PATH",
        "DENO_PATH",
        "GO_PATH",
        "GOPRIVATE",
        "NETRC",
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
        "NPM_CONFIG_REGISTRY",
        "PIP_LOCAL_DEPENDENCIES",
        "ADDITIONAL_PYTHON_PATHS",
        "INCLUDE_HEADERS",
        "WHITELIST_WORKSPACES",
        "BLACKLIST_WORKSPACES",
        "INSTANCE_EVENTS_WEBHOOK",
        "CLOUD_HOSTED",
        "GLOBAL_CACHE_INTERVAL",
    ]);

    if server_mode || num_workers > 0 {
        let addr = SocketAddr::from((server_bind_address, port));

        let rsmq2 = rsmq.clone();
        let server_f = async {
            if server_mode {
                windmill_api::run_server(db.clone(), rsmq2, addr, rx.resubscribe()).await?;
            }
            Ok(()) as anyhow::Result<()>
        };

        let workers_f = async {
            if num_workers > 0 {
                run_workers(
                    db.clone(),
                    rx.resubscribe(),
                    num_workers,
                    base_internal_url.clone(),
                    rsmq.clone(),
                )
                .await?;
            }
            Ok(()) as anyhow::Result<()>
        };

        let rsmq2 = rsmq.clone();
        let monitor_f = async {
            if server_mode {
                monitor_db(&db, rx.resubscribe(), &base_internal_url, rsmq2);
            }
            Ok(()) as anyhow::Result<()>
        };

        let metrics_f = async {
            match metrics_addr {
                Some(addr) => {
                    windmill_common::serve_metrics(addr, rx.resubscribe(), num_workers > 0)
                        .await
                        .map_err(anyhow::Error::from)
                }
                None => Ok(()),
            }
        };

        futures::try_join!(shutdown_signal, server_f, metrics_f, workers_f, monitor_f)?;
    } else {
        tracing::info!("Nothing to do, exiting.");
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

pub fn monitor_db<R: rsmq_async::RsmqConnection + Send + Sync + Clone + 'static>(
    db: &Pool<Postgres>,
    rx: tokio::sync::broadcast::Receiver<()>,
    base_internal_url: &str,
    rsmq: Option<R>,
) {
    let db1 = db.clone();
    let db2 = db.clone();

    let rx2 = rx.resubscribe();
    let base_internal_url = base_internal_url.to_string();
    tokio::spawn(async move {
        handle_zombie_jobs_periodically(&db1, rx, &base_internal_url, rsmq).await
    });
    tokio::spawn(async move { windmill_api::delete_expired_items_perdiodically(&db2, rx2).await });
}

pub async fn run_workers<R: rsmq_async::RsmqConnection + Send + Sync + Clone + 'static>(
    db: Pool<Postgres>,
    rx: tokio::sync::broadcast::Receiver<()>,
    num_workers: i32,
    base_internal_url: String,
    rsmq: Option<R>,
) -> anyhow::Result<()> {
    let license_key = std::env::var("LICENSE_KEY").ok();
    #[cfg(feature = "enterprise")]
    ee::verify_license_key(license_key)?;

    #[cfg(not(feature = "enterprise"))]
    if license_key.is_some() {
        panic!("License key is required ONLY for the enterprise edition");
    }

    let deno_runner_pool = std::sync::Arc::new(windmill_deno::runner::DenoRunnerPool::new()); // TODO: This arc is sort of unecessary, but I can't be bothered explaining that to rust

    let instance_name = rd_string(5);
    let monitor = tokio_metrics::TaskMonitor::new();

    let ip = windmill_common::external_ip::get_ip()
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = e.to_string(), "failed to get external IP");
            "unretrievable IP".to_string()
        });

    let mut handles = Vec::with_capacity(num_workers as usize);

    for x in [PIP_CACHE_DIR, DENO_CACHE_DIR, GO_CACHE_DIR] {
        DirBuilder::new()
            .recursive(true)
            .create(x)
            .await
            .expect("could not create initial worker dir");
    }

    #[cfg(feature = "enterprise")]
    if let Some(ref s) = S3_CACHE_BUCKET.clone() {
        // We donwload the entire cache as tar
        windmill_worker::copy_cache_from_bucket_as_tar(&s).await;
    }

    IS_READY.store(true, Ordering::Relaxed);
    let sync_barrier = Arc::new(RwLock::new(None));
    for i in 1..(num_workers + 1) {
        let db1 = db.clone();
        let instance_name = instance_name.clone();
        let worker_name = format!("dt-worker-{}-{}", &instance_name, rd_string(5));
        let ip = ip.clone();
        let rx = rx.resubscribe();
        let base_internal_url = base_internal_url.clone();
        let rsmq2 = rsmq.clone();
        let deno_runner_pool = deno_runner_pool.clone();
        let sync_barrier = sync_barrier.clone();
        handles.push(tokio::spawn(monitor.instrument(async move {
            tracing::info!(worker = %worker_name, "starting worker");
            windmill_worker::run_worker(
                &db1,
                &instance_name,
                worker_name,
                i as u64,
                num_workers as u32,
                &ip,
                rx,
                &base_internal_url,
                rsmq2,
                deno_runner_pool.clone(),
                sync_barrier,
            )
            .await
        })));
    }

    futures::future::try_join_all(handles).await?;
    Ok(())
}
