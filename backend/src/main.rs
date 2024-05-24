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
    time::Duration,
};
use tokio::fs::DirBuilder;
use windmill_api::HTTP_CLIENT;

#[cfg(feature = "enterprise")]
use windmill_common::ee::schedule_key_renewal;

use windmill_common::{
    global_settings::{
        BASE_URL_SETTING, BUNFIG_INSTALL_SCOPES_SETTING, CRITICAL_ERROR_CHANNELS_SETTING,
        CUSTOM_TAGS_SETTING, DEFAULT_TAGS_PER_WORKSPACE_SETTING, ENV_SETTINGS,
        EXPOSE_DEBUG_METRICS_SETTING, EXPOSE_METRICS_SETTING, EXTRA_PIP_INDEX_URL_SETTING,
        HUB_BASE_URL_SETTING, JOB_DEFAULT_TIMEOUT_SECS_SETTING, KEEP_JOB_DIR_SETTING,
        LICENSE_KEY_SETTING, NPM_CONFIG_REGISTRY_SETTING, OAUTH_SETTING, PIP_INDEX_URL_SETTING,
        REQUEST_SIZE_LIMIT_SETTING, REQUIRE_PREEXISTING_USER_FOR_OAUTH_SETTING,
        RETENTION_PERIOD_SECS_SETTING, SAML_METADATA_SETTING, SCIM_TOKEN_SETTING,
    },
    stats_ee::schedule_stats,
    utils::{rd_string, Mode},
    worker::{reload_custom_tags_setting, WORKER_GROUP},
    DB, METRICS_ENABLED,
};

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
use monitor::monitor_mem;

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[cfg(feature = "enterprise")]
use windmill_common::METRICS_ADDR;

#[cfg(feature = "parquet")]
use windmill_common::global_settings::OBJECT_STORE_CACHE_CONFIG_SETTING;

use windmill_worker::{
    BUN_CACHE_DIR, DENO_CACHE_DIR, DENO_CACHE_DIR_DEPS, DENO_CACHE_DIR_NPM, GO_BIN_CACHE_DIR,
    GO_CACHE_DIR, HUB_CACHE_DIR, LOCK_CACHE_DIR, PIP_CACHE_DIR, POWERSHELL_CACHE_DIR,
    TAR_PIP_CACHE_DIR, TMP_LOGS_DIR,
};

use crate::monitor::{
    initial_load, load_keep_job_dir, load_metrics_debug_enabled, load_require_preexisting_user,
    load_tag_per_workspace_enabled, monitor_db, monitor_pool, reload_base_url_setting,
    reload_bunfig_install_scopes_setting, reload_critical_error_channels_setting,
    reload_extra_pip_index_url_setting, reload_hub_base_url_setting,
    reload_job_default_timeout_setting, reload_license_key, reload_npm_config_registry_setting,
    reload_pip_index_url_setting, reload_retention_period_setting, reload_scim_token_setting,
    reload_server_config, reload_worker_config,
};

#[cfg(feature = "parquet")]
use crate::monitor::reload_s3_cache_setting;

const GIT_VERSION: &str = git_version!(args = ["--tag", "--always"], fallback = "unknown-version");
const DEFAULT_NUM_WORKERS: usize = 1;
const DEFAULT_PORT: u16 = 8000;
const DEFAULT_SERVER_BIND_ADDR: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

mod ee;
mod monitor;

#[cfg(feature = "pg_embed")]
mod pg_embed;

#[inline(always)]
fn create_and_run_current_thread_inner<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R> + 'static,
    R: Send + 'static,
{
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(32)
        .build()
        .unwrap();

    // Since this is the main future, we want to box it in debug mode because it tends to be fairly
    // large and the compiler won't optimize repeated copies. We also make this runtime factory
    // function #[inline(always)] to avoid holding the unboxed, unused future on the stack.
    #[cfg(debug_assertions)]
    // SAFETY: this this is guaranteed to be running on a current-thread executor
    let future = Box::pin(future);

    rt.block_on(future)
}

pub fn main() -> anyhow::Result<()> {
    deno_core::JsRuntime::init_platform(None);
    create_and_run_current_thread_inner(windmill_main())
}

async fn windmill_main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }

    #[cfg(not(feature = "flamegraph"))]
    windmill_common::tracing_init::initialize_tracing();

    #[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
    tracing::info!("jemalloc enabled");

    #[cfg(feature = "flamegraph")]
    let _guard = windmill_common::tracing_init::setup_flamegraph();

    let cli_arg = std::env::args().nth(1).unwrap_or_default();

    match cli_arg.as_str() {
        "cache" => {
            #[cfg(feature = "embedding")]
            {
                tracing::info!("Caching embedding model...");
                windmill_api::embeddings::ModelInstance::load_model_files().await?;
                tracing::info!("Cached embedding model");
            }
            #[cfg(not(feature = "embedding"))]
            {
                tracing::warn!("Embeddings are not enabled, ignoring...");
            }
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
                #[cfg(feature = "enterprise")]
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
        // migration code to avoid break
        windmill_api::migrate_db(&db).await?;
    }
    let (killpill_tx, mut killpill_rx) = tokio::sync::broadcast::channel::<()>(2);
    let mut monitor_killpill_rx = killpill_tx.subscribe();
    let server_killpill_rx = killpill_tx.subscribe();
    let (killpill_phase2_tx, _killpill_phase2_rx) = tokio::sync::broadcast::channel::<()>(2);

    let shutdown_signal =
        windmill_common::shutdown_signal(killpill_tx.clone(), killpill_tx.subscribe());

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

        initial_load(&db, killpill_tx.clone(), worker_mode, server_mode, is_agent).await;

        monitor_db(&db, &base_internal_url, rsmq.clone(), server_mode, true).await;

        monitor_pool(&db).await;

        #[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
        monitor_mem().await;

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
                    server_killpill_rx,
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
            let mut rx = killpill_rx.resubscribe();

            if !killpill_rx.try_recv().is_ok() {
                let base_internal_url = base_internal_rx.await?;
                if worker_mode {
                    run_workers(
                        db.clone(),
                        rx,
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
                    rx.recv().await?;
                }
            }
            tracing::info!("Starting phase 2 of shutdown");
            killpill_phase2_tx.send(())?;
            Ok(()) as anyhow::Result<()>
        };

        let monitor_f = async {
            let db = db.clone();
            let tx = killpill_tx.clone();
            let rsmq = rsmq.clone();

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
                                false
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
                                                DEFAULT_TAGS_PER_WORKSPACE_SETTING => {
                                                    if let Err(e) = load_tag_per_workspace_enabled(&db).await {
                                                        tracing::error!("Error loading default tag per workpsace: {e}");
                                                    }
                                                },
                                                RETENTION_PERIOD_SECS_SETTING => {
                                                    reload_retention_period_setting(&db).await
                                                },
                                                JOB_DEFAULT_TIMEOUT_SECS_SETTING => {
                                                    reload_job_default_timeout_setting(&db).await
                                                },
                                                #[cfg(feature = "parquet")]
                                                OBJECT_STORE_CACHE_CONFIG_SETTING if !is_agent => {
                                                    reload_s3_cache_setting(&db).await
                                                },
                                                SCIM_TOKEN_SETTING => {
                                                    reload_scim_token_setting(&db).await
                                                },
                                                EXTRA_PIP_INDEX_URL_SETTING => {
                                                    reload_extra_pip_index_url_setting(&db).await
                                                },
                                                PIP_INDEX_URL_SETTING => {
                                                    reload_pip_index_url_setting(&db).await
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
                                                EXPOSE_METRICS_SETTING  => {
                                                    if worker_mode {
                                                        tracing::info!("Metrics setting changed, restarting");
                                                        // we wait a bit randomly to avoid having all serverss and workers shutdown at same time
                                                        let rd_delay = rand::thread_rng().gen_range(0..4);
                                                        tokio::time::sleep(Duration::from_secs(rd_delay)).await;
                                                        if let Err(e) = tx.send(()) {
                                                            tracing::error!(error = %e, "Could not send killpill to server");
                                                        }
                                                    }
                                                },
                                                EXPOSE_DEBUG_METRICS_SETTING => {
                                                    if let Err(e) = load_metrics_debug_enabled(&db).await {
                                                        tracing::error!(error = %e, "Could not reload debug metrics setting");
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
                                                SAML_METADATA_SETTING => {
                                                    tracing::info!("SAML metadata change detected, killing server expecting to be restarted");
                                                    if let Err(e) = tx.send(()) {
                                                        tracing::error!(error = %e, "Could not send killpill to server");
                                                    }
                                                },
                                                HUB_BASE_URL_SETTING => {
                                                    if let Err(e) = reload_hub_base_url_setting(&db, server_mode).await {
                                                        tracing::error!(error = %e, "Could not reload hub base url setting");
                                                    }
                                                },
                                                CRITICAL_ERROR_CHANNELS_SETTING => {
                                                    if let Err(e) = reload_critical_error_channels_setting(&db).await {
                                                        tracing::error!(error = %e, "Could not reload critical error emails setting");
                                                    }
                                                },
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
                        _ = monitor_killpill_rx.recv() => {
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
                windmill_common::serve_metrics(*METRICS_ADDR, _killpill_phase2_rx, num_workers > 0)
                    .await;
            }
            Ok(()) as anyhow::Result<()>
        };

        let instance_name = rd_string(8);
        if mode == Mode::Server || mode == Mode::Standalone {
            schedule_stats(instance_name, &db, &HTTP_CLIENT).await;
        }

        #[cfg(feature = "enterprise")]
        if mode == Mode::Server || mode == Mode::Standalone {
            schedule_key_renewal(&HTTP_CLIENT, &db).await;
        }

        futures::try_join!(shutdown_signal, workers_f, monitor_f, server_f, metrics_f)?;
    } else {
        tracing::info!("Nothing to do, exiting.");
    }
    tracing::info!("Exiting connection pool");
    tokio::select! {
        _ = db.close() => {
            tracing::info!("Database connection pool closed");
        },
        _ = tokio::time::sleep(Duration::from_secs(15)) => {
            tracing::warn!("Could not close database connection pool in time (15s). Exiting anyway.");
        }
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
    mut rx: tokio::sync::broadcast::Receiver<()>,
    tx: tokio::sync::broadcast::Sender<()>,
    num_workers: i32,
    base_internal_url: String,
    rsmq: Option<R>,
    agent_mode: bool,
) -> anyhow::Result<()> {
    let mut killpill_rxs = vec![];
    for _ in 0..num_workers {
        killpill_rxs.push(rx.resubscribe());
    }

    if rx.try_recv().is_ok() {
        tracing::info!("Received killpill, exiting");
        return Ok(());
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

    #[cfg(tokio_unstable)]
    let monitor = tokio_metrics::TaskMonitor::new();

    let ip = windmill_common::external_ip::get_ip()
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = e.to_string(), "failed to get external IP");
            "unretrievable IP".to_string()
        });

    let mut handles = Vec::with_capacity(num_workers as usize);

    for x in [
        LOCK_CACHE_DIR,
        TMP_LOGS_DIR,
        PIP_CACHE_DIR,
        TAR_PIP_CACHE_DIR,
        DENO_CACHE_DIR,
        DENO_CACHE_DIR_DEPS,
        DENO_CACHE_DIR_NPM,
        BUN_CACHE_DIR,
        GO_CACHE_DIR,
        GO_BIN_CACHE_DIR,
        HUB_CACHE_DIR,
        POWERSHELL_CACHE_DIR,
    ] {
        DirBuilder::new()
            .recursive(true)
            .create(x)
            .await
            .expect("could not create initial worker dir");
    }

    for i in 1..(num_workers + 1) {
        let db1 = db.clone();
        let instance_name = instance_name.clone();
        let worker_name = format!("wk-{}-{}-{}", *WORKER_GROUP, &instance_name, rd_string(5));
        let ip = ip.clone();
        let rx = killpill_rxs.pop().unwrap();
        let tx = tx.clone();
        let base_internal_url = base_internal_url.clone();
        let rsmq2 = rsmq.clone();

        handles.push(tokio::spawn(async move {
            tracing::info!(worker = %worker_name, "starting worker");

            let f = windmill_worker::run_worker(
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
                agent_mode,
            );

            #[cfg(tokio_unstable)]
            {
                monitor.monitor(f, "worker").await
            }

            #[cfg(not(tokio_unstable))]
            {
                f.await
            }
        }));
    }

    futures::future::try_join_all(handles).await?;
    Ok(())
}
