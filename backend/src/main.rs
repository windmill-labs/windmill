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
use windmill_api::HTTP_CLIENT;
use windmill_common::{
    global_settings::{
        BASE_URL_SETTING, BUNFIG_INSTALL_SCOPES_SETTING, CUSTOM_TAGS_SETTING,
        DISABLE_STATS_SETTING, ENV_SETTINGS, EXPOSE_DEBUG_METRICS_SETTING, EXPOSE_METRICS_SETTING,
        EXTRA_PIP_INDEX_URL_SETTING, JOB_DEFAULT_TIMEOUT_SECS_SETTING, KEEP_JOB_DIR_SETTING,
        LICENSE_KEY_SETTING, NPM_CONFIG_REGISTRY_SETTING, OAUTH_SETTING,
        REQUEST_SIZE_LIMIT_SETTING, REQUIRE_PREEXISTING_USER_FOR_OAUTH_SETTING,
        RETENTION_PERIOD_SECS_SETTING,
    },
    stats::schedule_stats,
    utils::{rd_string, Mode},
    worker::{reload_custom_tags_setting, WORKER_GROUP},
    DB, METRICS_ADDR, METRICS_ENABLED,
};
use windmill_worker::{
    BUN_CACHE_DIR, BUN_TMP_CACHE_DIR, DENO_CACHE_DIR, DENO_CACHE_DIR_DEPS, DENO_CACHE_DIR_NPM,
    DENO_TMP_CACHE_DIR, DENO_TMP_CACHE_DIR_DEPS, DENO_TMP_CACHE_DIR_NPM, GO_BIN_CACHE_DIR,
    GO_CACHE_DIR, GO_TMP_CACHE_DIR, HUB_CACHE_DIR, HUB_TMP_CACHE_DIR, LOCK_CACHE_DIR,
    PIP_CACHE_DIR, POWERSHELL_CACHE_DIR, ROOT_TMP_CACHE_DIR, TAR_PIP_TMP_CACHE_DIR,
};

use crate::monitor::{
    initial_load, load_keep_job_dir, load_require_preexisting_user, monitor_db, monitor_pool,
    reload_base_url_setting, reload_bunfig_install_scopes_setting,
    reload_extra_pip_index_url_setting, reload_job_default_timeout_setting, reload_license_key,
    reload_npm_config_registry_setting, reload_retention_period_setting, reload_server_config,
    reload_worker_config,
};

const GIT_VERSION: &str = git_version!(args = ["--tag", "--always"], fallback = "unknown-version");
const DEFAULT_NUM_WORKERS: usize = 1;
const DEFAULT_PORT: u16 = 8000;
const DEFAULT_SERVER_BIND_ADDR: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

mod ee;
mod monitor;

#[cfg(feature = "pg_embed")]
mod pg_embed;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }

    #[cfg(not(feature = "flamegraph"))]
    windmill_common::tracing_init::initialize_tracing();

    #[cfg(feature = "flamegraph")]
    let _guard = windmill_common::tracing_init::setup_flamegraph();

    let cli_arg = std::env::args().nth(1).unwrap_or_default();

    match cli_arg.as_str() {
        "cache" => {
            tracing::info!("Caching embedding model...");
            windmill_api::embeddings::ModelInstance::load_model_files().await?;
            tracing::info!("Cached embedding model");
            return Ok(());
        }
        "-v" | "--version" | "version" => {
            println!("Windmill {}", GIT_VERSION);
            return Ok(());
        }
        _ => {}
    }

    let mode = std::env::var("MODE")
        .map(|x| x.to_lowercase())
        .map(|x| {
            if &x == "server" {
                tracing::info!("Binary is in 'server' mode");
                Mode::Server
            } else if &x == "worker" {
                tracing::info!("Binary is in 'worker' mode");
                Mode::Worker
            } else if &x == "agent" {
                tracing::info!("Binary is in 'agent' mode");
                if std::env::var("BASE_INTERNAL_URL").is_err() {
                    panic!("BASE_INTERNAL_URL is required in agent mode")
                }
                if std::env::var("JOB_TOKEN").is_err() {
                    tracing::warn!("JOB_TOKEN is not passed, hence workers will still create one ephemeral token per job and the DATABASE_URL need to be of a role that can INSERT into the token table")
                }

                #[cfg(not(feature = "enterprise"))]
                {
                    panic!("Agent mode is only available in the EE, ignoring...");
                }

                Mode::Agent
            } else {
                if &x != "standalone" {
                    tracing::error!("mode not recognized, defaulting to standalone: {x}");
                } else {
                    tracing::info!("Binary is in 'standalone' mode");
                }
                Mode::Standalone
            }
        })
        .unwrap_or_else(|_| {
            tracing::info!("Mode not specified, defaulting to standalone");
            Mode::Standalone
        });

    let num_workers = if mode == Mode::Server {
        0
    } else {
        std::env::var("NUM_WORKERS")
            .ok()
            .and_then(|x| x.parse::<i32>().ok())
            .unwrap_or(DEFAULT_NUM_WORKERS as i32)
    };

    if num_workers > 1 {
        tracing::warn!(
            "We STRONGLY recommend using at most 1 worker per container, use at your own risks"
        );
    }

    let server_mode = !std::env::var("DISABLE_SERVER")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false)
        && (mode == Mode::Server || mode == Mode::Standalone);

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

    #[cfg(feature = "pg_embed")]
    let _pg = {
        let (db_url, pg) = pg_embed::start().await.expect("pg embed");
        tracing::info!("Use embedded pg: {db_url}");
        std::env::set_var("DATABASE_URL", db_url);
        pg
    };

    tracing::info!("Connecting to database...");
    let db = windmill_common::connect_db(server_mode).await?;
    tracing::info!("Database connected");

    let num_version = sqlx::query_scalar!("SELECT version()").fetch_one(&db).await;

    tracing::info!(
        "PostgreSQL version: {} (windmill require PG >= 14)",
        num_version
            .ok()
            .flatten()
            .unwrap_or_else(|| "UNKNOWN".to_string())
    );

    let rsmq = if let Some(config) = rsmq_config {
        tracing::info!("Redis config set: {:?}", config);
        Some(rsmq_async::MultiplexedRsmq::new(config).await.unwrap())
    } else {
        None
    };

    let is_agent = mode == Mode::Agent;

    if !is_agent {
        let last_mig_version = sqlx::query_scalar!(
            "select version from _sqlx_migrations order by version desc limit 1;"
        )
        .fetch_optional(&db)
        .await
        .ok()
        .flatten();

        tracing::info!(
        "Last migration version: {last_mig_version:?}. Starting potential migration of the db if first connection on a new windmill version (can take a while depending on the migration) ...",
    );

        // migration code to avoid break
        windmill_api::migrate_db(&db).await?;

        let new_last_mig_version = sqlx::query_scalar!(
            "select version from _sqlx_migrations order by version desc limit 1;"
        )
        .fetch_optional(&db)
        .await
        .ok()
        .flatten();

        if last_mig_version != new_last_mig_version {
            tracing::info!(
                "Completed migration of the db. New  migration version: {}",
                new_last_mig_version.unwrap_or(-1)
            );
        } else {
            tracing::info!("No migration, db was up-to-date");
        }
    }
    let (killpill_tx, killpill_rx) = tokio::sync::broadcast::channel::<()>(2);
    let (killpill_phase2_tx, killpill_phase2_rx) = tokio::sync::broadcast::channel::<()>(2);
    let shutdown_signal =
        windmill_common::shutdown_signal(killpill_tx.clone(), killpill_rx.resubscribe());

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

        let default_base_internal_url = format!("http://localhost:{}", port.to_string());
        // since it's only on server mode, the port is statically defined
        let base_internal_url: String = if let Ok(base_url) = std::env::var("BASE_INTERNAL_URL") {
            if !is_agent {
                tracing::warn!("BASE_INTERNAL_URL is now unecessary and ignored unless the mode is 'agent', you can remove it.");
                default_base_internal_url.clone()
            } else {
                base_url
            }
        } else {
            default_base_internal_url.clone()
        };

        initial_load(&db, killpill_tx.clone(), worker_mode, server_mode).await;

        monitor_db(&db, &base_internal_url, rsmq.clone(), server_mode).await;

        monitor_pool(&db).await;

        let addr = SocketAddr::from((server_bind_address, port));

        let rsmq2 = rsmq.clone();
        let (base_internal_tx, base_internal_rx) = tokio::sync::oneshot::channel::<String>();

        DirBuilder::new()
            .recursive(true)
            .create("/tmp/windmill")
            .await
            .expect("could not create initial server dir");

        let server_f = async {
            if !is_agent {
                windmill_api::run_server(
                    db.clone(),
                    rsmq2,
                    addr,
                    killpill_phase2_rx.resubscribe(),
                    base_internal_tx,
                    server_mode,
                )
                .await?;
            } else {
                base_internal_tx
                    .send(base_internal_url.clone())
                    .map_err(|e| {
                        anyhow::anyhow!("Could not send base_internal_url to agent: {e}")
                    })?;
            }
            Ok(()) as anyhow::Result<()>
        };

        let workers_f = async {
            let base_internal_url = base_internal_rx.await?;
            if worker_mode {
                run_workers(
                    db.clone(),
                    killpill_rx.resubscribe(),
                    killpill_tx.clone(),
                    num_workers,
                    base_internal_url.clone(),
                    rsmq.clone(),
                    mode.clone() == Mode::Agent,
                )
                .await?;
                tracing::info!("All workers exited.");
                killpill_tx.send(())?;
            } else {
                killpill_rx.resubscribe().recv().await?;
            }
            tracing::info!("Starting phase 2 of shutdown");
            killpill_phase2_tx.send(())?;
            Ok(()) as anyhow::Result<()>
        };

        let monitor_f = async {
            let db = db.clone();
            let tx = killpill_tx.clone();
            let rsmq = rsmq.clone();

            let mut rx = killpill_rx.resubscribe();
            let base_internal_url = base_internal_url.to_string();
            let h = tokio::spawn(async move {
                let mut listener = retry_listen_pg(&db).await;

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
                                            match n.payload() {
                                                "server" if server_mode => {
                                                    tracing::info!("Server config change detected: {}", n.payload());

                                                    reload_server_config(&db).await;
                                                },
                                                a@ _ if worker_mode && a == format!("worker__{}", *WORKER_GROUP) => {
                                                    tracing::info!("Worker config change detected: {}", n.payload());
                                                    reload_worker_config(&db, tx.clone(), true).await;
                                                },
                                                _ => {
                                                    tracing::debug!("config changed but did not target this server/worker");
                                                }
                                            }
                                        },
                                        "notify_global_setting_change" => {
                                            tracing::info!("Global setting change detected: {}", n.payload());
                                            match n.payload() {
                                                BASE_URL_SETTING => {
                                                    if let Err(e) = reload_base_url_setting(&db).await {
                                                        tracing::error!(error = %e, "Could not reload base url setting");
                                                    }
                                                },
                                                OAUTH_SETTING => {
                                                    if let Err(e) = reload_base_url_setting(&db).await {
                                                        tracing::error!(error = %e, "Could not reload oauth setting");
                                                    }
                                                },
                                                CUSTOM_TAGS_SETTING => {
                                                    if let Err(e) = reload_custom_tags_setting(&db).await {
                                                        tracing::error!(error = %e, "Could not reload custom tags setting");
                                                    }
                                                },
                                                LICENSE_KEY_SETTING => {
                                                    if let Err(e) = reload_license_key(&db).await {
                                                        tracing::error!(error = %e, "Could not reload license key setting");
                                                    }
                                                },
                                                RETENTION_PERIOD_SECS_SETTING => {
                                                    reload_retention_period_setting(&db).await
                                                },
                                                JOB_DEFAULT_TIMEOUT_SECS_SETTING => {
                                                    reload_job_default_timeout_setting(&db).await
                                                },
                                                EXTRA_PIP_INDEX_URL_SETTING => {
                                                    reload_extra_pip_index_url_setting(&db).await
                                                },
                                                NPM_CONFIG_REGISTRY_SETTING => {
                                                    reload_npm_config_registry_setting(&db).await
                                                },
                                                BUNFIG_INSTALL_SCOPES_SETTING => {
                                                    reload_bunfig_install_scopes_setting(&db).await
                                                },
                                                KEEP_JOB_DIR_SETTING => {
                                                    load_keep_job_dir(&db).await;
                                                },
                                                REQUIRE_PREEXISTING_USER_FOR_OAUTH_SETTING => {
                                                    load_require_preexisting_user(&db).await;
                                                },
                                                EXPOSE_METRICS_SETTING | EXPOSE_DEBUG_METRICS_SETTING => {
                                                    if n.payload() != EXPOSE_DEBUG_METRICS_SETTING || worker_mode {
                                                        tracing::info!("Metrics setting changed, restarting");
                                                        // we wait a bit randomly to avoid having all serverss and workers shutdown at same time
                                                        let rd_delay = rand::thread_rng().gen_range(0..4);
                                                        tokio::time::sleep(Duration::from_secs(rd_delay)).await;
                                                        if let Err(e) = tx.send(()) {
                                                            tracing::error!(error = %e, "Could not send killpill to server");
                                                        }
                                                    }
                                                },
                                                REQUEST_SIZE_LIMIT_SETTING => {
                                                    if server_mode {
                                                        tracing::info!("Request limit size change detected, killing server expecting to be restarted");
                                                        // we wait a bit randomly to avoid having all servers shutdown at same time
                                                        let rd_delay = rand::thread_rng().gen_range(0..4);
                                                        tokio::time::sleep(Duration::from_secs(rd_delay)).await;
                                                        if let Err(e) = tx.send(()) {
                                                            tracing::error!(error = %e, "Could not send killpill to server");
                                                        }
                                                    }
                                                },
                                                DISABLE_STATS_SETTING => {},
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
                                    tracing::error!(error = %e, "Could not receive notification, attempting to reconnect listener");
                                    listener = retry_listen_pg(&db).await;
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

            if let Err(e) = h.await {
                tracing::error!("Error waiting for monitor handle:{e}")
            }
            Ok(()) as anyhow::Result<()>
        };

        let metrics_f = async {
            if METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
                #[cfg(not(feature = "enterprise"))]
                tracing::error!("Metrics are only available in the EE, ignoring...");

                #[cfg(feature = "enterprise")]
                windmill_common::serve_metrics(
                    *METRICS_ADDR,
                    killpill_phase2_rx.resubscribe(),
                    num_workers > 0,
                )
                .await;
            }
            Ok(()) as anyhow::Result<()>
        };

        let instance_name = rd_string(8);
        schedule_stats(
            instance_name,
            mode.clone(),
            &db,
            &HTTP_CLIENT,
            cfg!(feature = "enterprise"),
        )
        .await;

        futures::try_join!(shutdown_signal, workers_f, monitor_f, server_f, metrics_f)?;
    } else {
        tracing::info!("Nothing to do, exiting.");
    }
    Ok(())
}

async fn listen_pg(db: &DB) -> Option<PgListener> {
    let mut listener = match PgListener::connect_with(&db).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(error = %e, "Could not connect to database");
            return None;
        }
    };

    if let Err(e) = listener
        .listen_all(vec!["notify_config_change", "notify_global_setting_change"])
        .await
    {
        tracing::error!(error = %e, "Could not listen to database");
        return None;
    }

    return Some(listener);
}

async fn retry_listen_pg(db: &DB) -> PgListener {
    let mut listener = listen_pg(db).await;
    loop {
        if listener.is_none() {
            tracing::info!("Retrying listening to pg listen in 5 seconds");
            tokio::time::sleep(Duration::from_secs(5)).await;
            listener = listen_pg(db).await;
        } else {
            tracing::info!("Successfully connected to pg listen");
            return listener.unwrap();
        }
    }
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
    agent_mode: bool,
) -> anyhow::Result<()> {
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
        POWERSHELL_CACHE_DIR,
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
        let worker_name = format!("wk-{}-{}-{}", *WORKER_GROUP, &instance_name, rd_string(5));
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
                agent_mode,
            )
            .await
        })));
    }

    futures::future::try_join_all(handles).await?;
    Ok(())
}
