/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use gethostname::gethostname;
use git_version::git_version;
use monitor::handle_zombie_jobs_periodically;
use sqlx::{Pool, Postgres};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc},
};
use tokio::{
    fs::{metadata, DirBuilder},
    join,
    sync::RwLock,
};
use windmill_api::{LICENSE_KEY, OAUTH_CLIENTS, SMTP_CLIENT};
use windmill_common::{utils::rd_string, METRICS_ADDR};
use windmill_worker::{
    BUN_CACHE_DIR, BUN_TMP_CACHE_DIR, DENO_CACHE_DIR, DENO_TMP_CACHE_DIR, GO_CACHE_DIR,
    GO_TMP_CACHE_DIR, HUB_CACHE_DIR, HUB_TMP_CACHE_DIR, PIP_CACHE_DIR, ROOT_TMP_CACHE_DIR,
    TAR_PIP_TMP_CACHE_DIR,
};

const GIT_VERSION: &str = git_version!(args = ["--tag", "--always"], fallback = "unknown-version");
const DEFAULT_NUM_WORKERS: usize = 1;
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

    if num_workers > 1 {
        tracing::warn!("We recommend using at most 1 worker per container, use more only if you know what you are doing.");
    }
    let metrics_addr: Option<SocketAddr> = *METRICS_ADDR;

    let server_mode = !std::env::var("DISABLE_SERVER")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false);

    let server_bind_address: IpAddr = if server_mode {
        std::env::var("SERVER_BIND_ADDR")
            .ok()
            .and_then(|x| x.parse().ok())
            .unwrap_or(IpAddr::from(DEFAULT_SERVER_BIND_ADDR))
    } else {
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
    };

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|x| x.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT as u16);

    if std::env::var("BASE_INTERNAL_URL").is_ok() {
        tracing::warn!("BASE_INTERNAL_URL is now unecessary and ignored, you can remove it.");
    }

    let base_internal_url: String = format!("http://localhost:{}", port.to_string());

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

    tracing::info!("Connecting to database...");
    let db = windmill_common::connect_db(server_mode).await?;
    tracing::info!("Database connected");

    let rsmq = if let Some(config) = rsmq_config {
        let mut rsmq = rsmq_async::MultiplexedRsmq::new(config).await.unwrap();

        let _ = rsmq_async::RsmqConnection::create_queue(&mut rsmq, "main_queue", None, None, None)
            .await;
        Some(rsmq)
    } else {
        None
    };

    // migration code to avoid break
    windmill_api::migrate_db(&db).await?;

    let (tx, rx) = tokio::sync::broadcast::channel::<()>(3);
    let shutdown_signal = windmill_common::shutdown_signal(tx.clone(), rx.resubscribe());

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
        "GOPROXY",
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
        "WORKER_TAGS",
        "CUSTOM_TAGS",
        "JOB_RETENTION_SECS",
        "WAIT_RESULT_FAST_POLL_DURATION_SECS",
        "WAIT_RESULT_SLOW_POLL_INTERVAL_MS",
        "WAIT_RESULT_FAST_POLL_INTERVAL_MS",
        "EXIT_AFTER_NO_JOB_FOR_SECS",
        "REQUEST_SIZE_LIMIT",
        "SMTP_HOST",
        "SMTP_USERNAME",
        "SMTP_PORT",
        "SMTP_TLS_IMPLICIT",
        "CREATE_WORKSPACE_REQUIRE_SUPERADMIN",
        "GLOBAL_ERROR_HANDLER_PATH_IN_ADMINS_WORKSPACE",
    ]);

    tracing::info!("Loading OAuth providers...: {:#?}", *OAUTH_CLIENTS);
    if let Some(ref smtp) = *SMTP_CLIENT {
        tracing::info!("Smtp client defined. Testing connection...");
        if let Err(e) = smtp.connect().await {
            tracing::error!("Failed to connect to smtp server: {}", e);
        } else {
            tracing::info!("Smtp client connected.");
        }
    }
    if server_mode || num_workers > 0 {
        let addr = SocketAddr::from((server_bind_address, port));

        let rsmq2 = rsmq.clone();
        let server_f = async {
            windmill_api::run_server(db.clone(), rsmq2, addr, rx.resubscribe()).await?;
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
                tracing::info!("All workers exited.");
                tx.send(())?; // signal server to shutdown
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
        join!(
            handle_zombie_jobs_periodically(&db1, rx, &base_internal_url, rsmq),
            windmill_api::delete_expired_items_perdiodically(&db2, rx2)
        );
    });
}

pub async fn run_workers<R: rsmq_async::RsmqConnection + Send + Sync + Clone + 'static>(
    db: Pool<Postgres>,
    rx: tokio::sync::broadcast::Receiver<()>,
    num_workers: i32,
    base_internal_url: String,
    rsmq: Option<R>,
) -> anyhow::Result<()> {
    #[cfg(feature = "enterprise")]
    ee::verify_license_key(LICENSE_KEY.clone())?;

    #[cfg(not(feature = "enterprise"))]
    if LICENSE_KEY.as_ref().is_some_and(|x| !x.is_empty()) {
        panic!("License key is required ONLY for the enterprise edition");
    }

    let instance_name = gethostname()
        .to_str()
        .map(|x| {
            x.replace(" ", "")
                .split("-")
                .last()
                .unwrap()
                .to_ascii_lowercase()
                .to_string()
        })
        .unwrap_or_else(|| rd_string(5));

    let monitor = tokio_metrics::TaskMonitor::new();

    let ip = windmill_common::external_ip::get_ip()
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = e.to_string(), "failed to get external IP");
            "unretrievable IP".to_string()
        });

    let mut handles = Vec::with_capacity(num_workers as usize);

    if metadata(&ROOT_TMP_CACHE_DIR).await.is_ok() {
        if let Err(e) = tokio::fs::remove_dir_all(&ROOT_TMP_CACHE_DIR).await {
            tracing::info!(error = %e, "Could not remove root tmp cache dir");
        }
    }

    for x in [
        PIP_CACHE_DIR,
        DENO_CACHE_DIR,
        BUN_CACHE_DIR,
        GO_CACHE_DIR,
        HUB_CACHE_DIR,
        TAR_PIP_TMP_CACHE_DIR,
        DENO_TMP_CACHE_DIR,
        BUN_TMP_CACHE_DIR,
        GO_TMP_CACHE_DIR,
        HUB_TMP_CACHE_DIR,
    ] {
        DirBuilder::new()
            .recursive(true)
            .create(x)
            .await
            .expect("could not create initial worker dir");
    }

    let sync_barrier = Arc::new(RwLock::new(None));
    for i in 1..(num_workers + 1) {
        let db1 = db.clone();
        let instance_name = instance_name.clone();
        let worker_name = format!("wk-{}-{}", &instance_name, rd_string(5));
        let ip = ip.clone();
        let rx = rx.resubscribe();
        let base_internal_url = base_internal_url.clone();
        let rsmq2 = rsmq.clone();
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
                sync_barrier,
            )
            .await
        })));
    }

    futures::future::try_join_all(handles).await?;
    Ok(())
}
