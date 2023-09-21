/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use gethostname::gethostname;
use git_version::git_version;
use rand::Rng;
use sqlx::{postgres::PgListener, Pool, Postgres};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::{
    fs::{metadata, DirBuilder},
    sync::RwLock,
};
use windmill_api::LICENSE_KEY;
use windmill_common::{
    global_settings::{
        BASE_URL_SETTING, CUSTOM_TAGS_SETTING, ENV_SETTINGS, OAUTH_SETTING,
        REQUEST_SIZE_LIMIT_SETTING, RETENTION_PERIOD_SECS_SETTING,
    },
    utils::rd_string,
    worker::{reload_custom_tags_setting, WORKER_GROUP},
    METRICS_ADDR,
};
use windmill_worker::{
    BUN_CACHE_DIR, BUN_TMP_CACHE_DIR, DENO_CACHE_DIR, DENO_CACHE_DIR_DEPS, DENO_CACHE_DIR_NPM,
    DENO_TMP_CACHE_DIR, DENO_TMP_CACHE_DIR_DEPS, DENO_TMP_CACHE_DIR_NPM, GO_BIN_CACHE_DIR,
    GO_CACHE_DIR, GO_TMP_CACHE_DIR, HUB_CACHE_DIR, HUB_TMP_CACHE_DIR, LOCK_CACHE_DIR,
    PIP_CACHE_DIR, ROOT_TMP_CACHE_DIR, TAR_PIP_TMP_CACHE_DIR,
};

use crate::monitor::{
    initial_load, monitor_db, reload_base_url_setting, reload_retention_period_setting,
    reload_server_config, reload_worker_config,
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

    #[cfg(not(feature = "flamegraph"))]
    windmill_common::tracing_init::initialize_tracing();

    #[cfg(feature = "flamegraph")]
    let _guard = windmill_common::tracing_init::setup_flamegraph();

    let num_workers = std::env::var("NUM_WORKERS")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(DEFAULT_NUM_WORKERS as i32);

    if num_workers > 1 {
        tracing::warn!("We STRONGLY recommend using at most 1 worker per container, unless this worker is dedicated to native jobs only. ");
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
        tracing::info!("Redis config set: {:?}", config);
        Some(rsmq_async::MultiplexedRsmq::new(config).await.unwrap())
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

    display_config(&ENV_SETTINGS);

    let worker_mode = num_workers > 0;

    if server_mode || worker_mode {
        let port_var = std::env::var("PORT").ok().and_then(|x| x.parse().ok());

        let port = if server_mode {
            port_var.unwrap_or(DEFAULT_PORT as u16)
        } else {
            port_var.unwrap_or(0)
        };

        // since it's only on server mode, the port is statically defined
        let base_internal_url: String = format!("http://localhost:{}", port.to_string());

        monitor_db(&db, &base_internal_url, rsmq.clone(), server_mode).await;

        initial_load(&db, tx.clone(), worker_mode, server_mode).await;

        if std::env::var("BASE_INTERNAL_URL").is_ok() {
            tracing::warn!("BASE_INTERNAL_URL is now unecessary and ignored, you can remove it.");
        }

        let addr = SocketAddr::from((server_bind_address, port));

        let rsmq2 = rsmq.clone();
        let (port_tx, port_rx) = tokio::sync::oneshot::channel::<u16>();

        let server_f = async {
            windmill_api::run_server(db.clone(), rsmq2, addr, rx.resubscribe(), port_tx).await?;
            Ok(()) as anyhow::Result<()>
        };

        let workers_f = async {
            let port = port_rx.await?;
            let base_internal_url: String = format!("http://localhost:{}", port.to_string());
            if worker_mode {
                run_workers(
                    db.clone(),
                    rx.resubscribe(),
                    tx.clone(),
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

        let monitor_f = async {
            let db = db.clone();
            let tx = tx.clone();
            let rsmq = rsmq.clone();

            let mut rx = rx.resubscribe();
            let base_internal_url = base_internal_url.to_string();
            let rd_delay = rand::thread_rng().gen_range(0..30);
            tokio::spawn(async move {
                //monitor_db is applied at start, no need to apply it twice
                tokio::time::sleep(Duration::from_secs(rd_delay)).await;

                let mut listener = match PgListener::connect_with(&db).await {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::error!(error = %e, "Could not connect to database");
                        return;
                    }
                };

                if let Err(e) = listener
                    .listen_all(vec!["notify_config_change", "notify_global_setting_change"])
                    .await
                {
                    tracing::error!(error = %e, "Could not listen to database");
                    return;
                }

                loop {
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(30))    => {
                            monitor_db(
                                &db,
                                &base_internal_url,
                                rsmq.clone(),
                                server_mode,
                            )
                            .await;
                        },
                        notification = listener.recv() => {
                            match notification {
                                Ok(n) => {
                                    tracing::info!("Received new pg notification: {n:?}");
                                    match n.channel() {
                                        "notify_config_change" => {
                                            tracing::info!("Config change detected");
                                            match n.payload() {
                                                "server" if server_mode => {
                                                    tracing::info!("Server config change detected");
                                                    reload_server_config(&db).await;
                                                },
                                                a@ _ if worker_mode && a == format!("worker__{}", *WORKER_GROUP) => {
                                                    tracing::info!("Worker config change detected");
                                                    reload_worker_config(&db, tx.clone(), true).await;
                                                },
                                                _ => {
                                                    ()
                                                }
                                            }
                                        },
                                        "notify_global_setting_change" => {
                                            tracing::info!("Global setting change detected");
                                            match n.payload() {
                                                BASE_URL_SETTING => {
                                                    tracing::info!("Base URL setting change detected");
                                                    if let Err(e) = reload_base_url_setting(&db).await {
                                                        tracing::error!(error = %e, "Could not reload base url setting");
                                                    }
                                                },
                                                OAUTH_SETTING => {
                                                    tracing::info!("OAuth setting change detected");
                                                    if let Err(e) = reload_base_url_setting(&db).await {
                                                        tracing::error!(error = %e, "Could not reload oauth setting");
                                                    }
                                                },
                                                CUSTOM_TAGS_SETTING => {
                                                    tracing::info!("Custom tags setting change detected");
                                                    if let Err(e) = reload_custom_tags_setting(&db).await {
                                                        tracing::error!(error = %e, "Could not reload custom tags setting");
                                                    }
                                                },
                                                RETENTION_PERIOD_SECS_SETTING => {
                                                    tracing::info!("Retention period setting change detected");
                                                    reload_retention_period_setting(&db).await
                                                },
                                                REQUEST_SIZE_LIMIT_SETTING => {
                                                    tracing::info!("Request limit size change detected, killing server expecting to be restarted");
                                                    // we wait a bit randomly to avoid having all servers shutdown at same time
                                                    let rd_delay = rand::thread_rng().gen_range(0..4);
                                                    tokio::time::sleep(Duration::from_secs(rd_delay)).await;
                                                    if let Err(e) = tx.send(()) {
                                                        tracing::error!(error = %e, "Could not send killpill to server");
                                                    }
                                                }
                                                a @_ => {
                                                    tracing::info!("Unrecognized Global Setting Change Payload: {:?}", a);
                                                }
                                            }
                                        },
                                        _ => {
                                            tracing::warn!("Unknown notification received");
                                            continue;
                                        }
                                    }
                                },
                                Err(e) => {
                                    tracing::error!(error = %e, "Could not receive notification");
                                    continue;
                                }
                            };
                        },
                        _ = rx.recv() => {
                                println!("received killpill for monitor job");
                                break;
                        }
                    }
                }
            });

            Ok(()) as anyhow::Result<()>
        };

        let metrics_f = async {
            if let Some(_addr) = metrics_addr {
                #[cfg(not(feature = "enterprise"))]
                panic!("Metrics are only available in the Enterprise Edition");

                #[cfg(feature = "enterprise")]
                windmill_common::serve_metrics(_addr, rx.resubscribe(), num_workers > 0).await;
            }
            Ok(()) as anyhow::Result<()>
        };

        futures::try_join!(shutdown_signal, server_f, metrics_f, workers_f, monitor_f)?;
    } else {
        tracing::info!("Nothing to do, exiting.");
    }
    Ok(())
}

fn display_config(envs: &[&str]) {
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

pub async fn run_workers<R: rsmq_async::RsmqConnection + Send + Sync + Clone + 'static>(
    db: Pool<Postgres>,
    rx: tokio::sync::broadcast::Receiver<()>,
    tx: tokio::sync::broadcast::Sender<()>,
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
        LOCK_CACHE_DIR,
        PIP_CACHE_DIR,
        DENO_CACHE_DIR,
        DENO_CACHE_DIR_DEPS,
        DENO_CACHE_DIR_NPM,
        BUN_CACHE_DIR,
        GO_CACHE_DIR,
        GO_BIN_CACHE_DIR,
        HUB_CACHE_DIR,
        TAR_PIP_TMP_CACHE_DIR,
        DENO_TMP_CACHE_DIR,
        DENO_TMP_CACHE_DIR_DEPS,
        DENO_TMP_CACHE_DIR_NPM,
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
        let tx = tx.clone();
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
                tx,
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
