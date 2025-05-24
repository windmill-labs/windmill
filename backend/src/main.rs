/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
use anyhow::Context;
use monitor::{
    load_base_url, load_otel, reload_critical_alerts_on_db_oversize,
    reload_delete_logs_periodically_setting, reload_indexer_config,
    reload_instance_python_version_setting, reload_maven_repos_setting,
    reload_no_default_maven_setting, reload_nuget_config_setting,
    reload_timeout_wait_result_setting, send_current_log_file_to_object_store,
    send_logs_to_object_store, WORKERS_NAMES,
};
use rand::Rng;
use sqlx::postgres::PgListener;
use std::{
    collections::HashMap,
    fs::{create_dir_all, DirBuilder},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, Instant},
};
use strum::IntoEnumIterator;
use tokio::{fs::File, io::AsyncReadExt, task::JoinHandle};
use uuid::Uuid;
use windmill_api::HTTP_CLIENT;

#[cfg(feature = "enterprise")]
use windmill_common::ee::{maybe_renew_license_key_on_start, LICENSE_KEY_ID, LICENSE_KEY_VALID};

use windmill_common::{
    agent_workers::build_agent_http_client,
    get_database_url,
    global_settings::{
        BASE_URL_SETTING, BUNFIG_INSTALL_SCOPES_SETTING, CRITICAL_ALERTS_ON_DB_OVERSIZE_SETTING,
        CRITICAL_ALERT_MUTE_UI_SETTING, CRITICAL_ERROR_CHANNELS_SETTING, CUSTOM_TAGS_SETTING,
        DEFAULT_TAGS_PER_WORKSPACE_SETTING, DEFAULT_TAGS_WORKSPACES_SETTING, EMAIL_DOMAIN_SETTING,
        ENV_SETTINGS, EXPOSE_DEBUG_METRICS_SETTING, EXPOSE_METRICS_SETTING,
        EXTRA_PIP_INDEX_URL_SETTING, HUB_BASE_URL_SETTING, INDEXER_SETTING,
        INSTANCE_PYTHON_VERSION_SETTING, JOB_DEFAULT_TIMEOUT_SECS_SETTING, JWT_SECRET_SETTING,
        KEEP_JOB_DIR_SETTING, LICENSE_KEY_SETTING, MAVEN_REPOS_SETTING,
        MONITOR_LOGS_ON_OBJECT_STORE_SETTING, NO_DEFAULT_MAVEN_SETTING,
        NPM_CONFIG_REGISTRY_SETTING, NUGET_CONFIG_SETTING, OAUTH_SETTING, OTEL_SETTING,
        PIP_INDEX_URL_SETTING, REQUEST_SIZE_LIMIT_SETTING,
        REQUIRE_PREEXISTING_USER_FOR_OAUTH_SETTING, RETENTION_PERIOD_SECS_SETTING,
        SAML_METADATA_SETTING, SCIM_TOKEN_SETTING, SMTP_SETTING, TEAMS_SETTING,
        TIMEOUT_WAIT_RESULT_SETTING,
    },
    scripts::ScriptLang,
    stats_ee::schedule_stats,
    triggers::TriggerKind,
    utils::{hostname, rd_string, Mode, GIT_VERSION, MODE_AND_ADDONS},
    worker::{
        reload_custom_tags_setting, Connection, HUB_CACHE_DIR, TMP_DIR, TMP_LOGS_DIR, WORKER_GROUP,
    },
    KillpillSender, METRICS_ENABLED,
};

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
use monitor::monitor_mem;

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[cfg(feature = "parquet")]
use windmill_common::global_settings::OBJECT_STORE_CACHE_CONFIG_SETTING;

use windmill_worker::{
    get_hub_script_content_and_requirements, BUN_BUNDLE_CACHE_DIR, BUN_CACHE_DIR, CSHARP_CACHE_DIR,
    DENO_CACHE_DIR, DENO_CACHE_DIR_DEPS, DENO_CACHE_DIR_NPM, GO_BIN_CACHE_DIR, GO_CACHE_DIR,
    JAVA_CACHE_DIR, NU_CACHE_DIR, POWERSHELL_CACHE_DIR, PY310_CACHE_DIR, PY311_CACHE_DIR,
    PY312_CACHE_DIR, PY313_CACHE_DIR, RUST_CACHE_DIR, TAR_JAVA_CACHE_DIR, TAR_PY310_CACHE_DIR,
    TAR_PY311_CACHE_DIR, TAR_PY312_CACHE_DIR, TAR_PY313_CACHE_DIR, UV_CACHE_DIR,
};

use crate::monitor::{
    initial_load, load_keep_job_dir, load_metrics_debug_enabled, load_require_preexisting_user,
    load_tag_per_workspace_enabled, load_tag_per_workspace_workspaces, monitor_db,
    reload_base_url_setting, reload_bunfig_install_scopes_setting,
    reload_critical_alert_mute_ui_setting, reload_critical_error_channels_setting,
    reload_extra_pip_index_url_setting, reload_hub_base_url_setting,
    reload_job_default_timeout_setting, reload_jwt_secret_setting, reload_license_key,
    reload_npm_config_registry_setting, reload_pip_index_url_setting,
    reload_retention_period_setting, reload_scim_token_setting, reload_smtp_config,
    reload_worker_config,
};

#[cfg(feature = "parquet")]
use windmill_common::s3_helpers::reload_s3_cache_setting;

const DEFAULT_NUM_WORKERS: usize = 1;
const DEFAULT_PORT: u16 = 8000;
const DEFAULT_SERVER_BIND_ADDR: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

mod ee;
mod monitor;

pub fn setup_deno_runtime() -> anyhow::Result<()> {
    // https://github.com/denoland/deno/blob/main/cli/main.rs#L477
    #[cfg(feature = "deno_core")]
    let unrecognized_v8_flags = deno_core::v8_set_flags(vec![
        "--stack-size=1024".to_string(),
        // TODO(bartlomieju): I think this can be removed as it's handled by `deno_core`
        // and its settings.
        // deno_ast removes TypeScript `assert` keywords, so this flag only affects JavaScript
        // TODO(petamoriken): Need to check TypeScript `assert` keywords in deno_ast
        "--no-harmony-import-assertions".to_string(),
    ])
    .into_iter()
    .skip(1)
    .collect::<Vec<_>>();

    #[cfg(feature = "deno_core")]
    if !unrecognized_v8_flags.is_empty() {
        println!("Unrecognized V8 flags: {:?}", unrecognized_v8_flags);
    }

    #[cfg(feature = "deno_core")]
    deno_core::JsRuntime::init_platform(None, false);
    Ok(())
}

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

lazy_static::lazy_static! {
    static ref PG_LISTENER_REFRESH_PERIOD_SECS: u64 = std::env::var("PG_LISTENER_REFRESH_PERIOD_SECS")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or(3600 * 12);
}

pub fn main() -> anyhow::Result<()> {
    setup_deno_runtime()?;
    create_and_run_current_thread_inner(windmill_main())
}

async fn cache_hub_scripts(file_path: Option<String>) -> anyhow::Result<()> {
    let file_path = file_path.unwrap_or("./hubPaths.json".to_string());
    let mut file = File::open(&file_path)
        .await
        .with_context(|| format!("Could not open {}, make sure it exists", &file_path))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;
    let paths = serde_json::from_str::<HashMap<String, String>>(&contents).with_context(|| {
        format!(
            "Could not parse {}, make sure it is a valid JSON object with string keys and values",
            &file_path
        )
    })?;

    create_dir_all(HUB_CACHE_DIR)?;
    create_dir_all(BUN_BUNDLE_CACHE_DIR)?;

    for path in paths.values() {
        tracing::info!("Caching hub script at {path}");
        let res = get_hub_script_content_and_requirements(Some(path), None).await?;
        if res
            .language
            .as_ref()
            .is_some_and(|x| x == &ScriptLang::Deno)
        {
            let job_dir = format!("{}/cache_init/{}", TMP_DIR, Uuid::new_v4());
            create_dir_all(&job_dir)?;
            let _ = windmill_worker::generate_deno_lock(
                &Uuid::nil(),
                &res.content,
                &mut 0,
                &mut None,
                &job_dir,
                None,
                "global",
                "global",
                "",
                &mut None,
            )
            .await?;
            tokio::fs::remove_dir_all(job_dir).await?;
        } else if res.language.as_ref().is_some_and(|x| x == &ScriptLang::Bun) {
            let job_id = Uuid::new_v4();
            let job_dir = format!("{}/cache_init/{}", TMP_DIR, job_id);
            create_dir_all(&job_dir)?;
            if let Some(lockfile) = res.lockfile {
                let _ = windmill_worker::prepare_job_dir(&lockfile, &job_dir).await?;
                let envs = windmill_worker::get_common_bun_proc_envs(None).await;
                let _ = windmill_worker::install_bun_lockfile(
                    &mut 0,
                    &mut None,
                    &job_id,
                    "admins",
                    None,
                    &job_dir,
                    "cache_init",
                    envs.clone(),
                    false,
                    &mut None,
                )
                .await?;

                let _ = windmill_common::worker::write_file(&job_dir, "main.js", &res.content)?;

                if let Err(e) = windmill_worker::prebundle_bun_script(
                    &res.content,
                    Some(&lockfile),
                    &path,
                    &job_id,
                    "admins",
                    None,
                    &job_dir,
                    "",
                    "cache_init",
                    "",
                    &mut None,
                )
                .await
                {
                    panic!("Error prebundling bun script: {e:#}");
                }
            } else {
                tracing::warn!("No lockfile found for bun script {path}, skipping...");
            }
            tokio::fs::remove_dir_all(job_dir).await?;
        }
    }
    Ok(())
}

async fn windmill_main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }

    if let Err(_e) = rustls::crypto::ring::default_provider().install_default() {
        tracing::error!("Failed to install rustls crypto provider");
    }

    let hostname = hostname();

    let mode_and_addons = MODE_AND_ADDONS.clone();
    let mode = mode_and_addons.mode;

    if mode == Mode::Standalone {
        println!("Running in standalone mode");
    } else if mode == Mode::MCP {
        println!("Running in MCP mode");
    }

    #[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
    println!("jemalloc enabled");

    let cli_arg = std::env::args().nth(1).unwrap_or_default();

    match cli_arg.as_str() {
        "cache" => {
            #[cfg(feature = "embedding")]
            {
                println!("Caching embedding model...");
                windmill_api::embeddings::ModelInstance::load_model_files().await?;
                println!("Cached embedding model");
            }
            #[cfg(not(feature = "embedding"))]
            {
                println!("Embeddings are not enabled, ignoring...");
            }

            cache_hub_scripts(std::env::args().nth(2)).await?;

            return Ok(());
        }
        "-v" | "--version" | "version" => {
            println!("Windmill {}", GIT_VERSION);
            return Ok(());
        }
        _ => {}
    }

    #[allow(unused_mut)]
    let mut num_workers = if mode == Mode::Server || mode == Mode::Indexer || mode == Mode::MCP {
        0
    } else {
        std::env::var("NUM_WORKERS")
            .ok()
            .and_then(|x| x.parse::<i32>().ok())
            .unwrap_or(DEFAULT_NUM_WORKERS as i32)
    };

    if num_workers > 1 && !std::env::var("WORKER_GROUP").is_ok_and(|x| x == "native") {
        println!(
            "We STRONGLY recommend using at most 1 worker per container, use at your own risks"
        );
    }

    let server_mode = !std::env::var("DISABLE_SERVER")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false)
        && (mode == Mode::Server || mode == Mode::Standalone);

    let indexer_mode = mode == Mode::Indexer;
    let mcp_mode = mode == Mode::MCP;

    let server_bind_address: IpAddr = if server_mode || indexer_mode || mcp_mode {
        std::env::var("SERVER_BIND_ADDR")
            .ok()
            .and_then(|x| x.parse().ok())
            .unwrap_or(IpAddr::from(DEFAULT_SERVER_BIND_ADDR))
    } else {
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
    };

    let (conn, first_suffix) = if mode == Mode::Agent {
        tracing::info!(
            "Creating http client for cluster using base internal url {}",
            std::env::var("BASE_INTERNAL_URL").unwrap_or_default()
        );
        let suffix = windmill_common::utils::worker_suffix(&hostname, &rd_string(5));
        (
            Connection::Http(build_agent_http_client(&suffix)),
            Some(suffix),
        )
    } else {
        println!("Connecting to database...");

        let db = windmill_common::initial_connection().await?;

        let num_version = sqlx::query_scalar!("SELECT version()").fetch_one(&db).await;

        tracing::info!(
            "PostgreSQL version: {} (windmill require PG >= 14)",
            num_version
                .ok()
                .flatten()
                .unwrap_or_else(|| "UNKNOWN".to_string())
        );
        load_otel(&db).await;

        tracing::info!("Database connected");
        (Connection::Sql(db), None)
    };

    let environment = load_base_url(&conn)
        .await
        .unwrap_or_else(|_| "local".to_string())
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split(".")
        .next()
        .unwrap_or_else(|| "local")
        .to_string();

    let _guard = windmill_common::tracing_init::initialize_tracing(&hostname, &mode, &environment);

    let is_agent = mode == Mode::Agent;

    let mut migration_handle: Option<JoinHandle<()>> = None;
    #[cfg(feature = "parquet")]
    let disable_s3_store = std::env::var("DISABLE_S3_STORE")
        .ok()
        .is_some_and(|x| x == "1" || x == "true");

    if let Some(db) = conn.as_sql() {
        if !is_agent && !indexer_mode && !mcp_mode {
            let skip_migration = std::env::var("SKIP_MIGRATION")
                .map(|val| val == "true")
                .unwrap_or(false);

            if !skip_migration {
                // migration code to avoid break
                migration_handle = windmill_api::migrate_db(&db).await?;
            } else {
                tracing::info!("SKIP_MIGRATION set, skipping db migration...")
            }
        }
    }

    let worker_mode = num_workers > 0;

    let conn = if mode == Mode::Agent {
        conn
    } else {
        // This time we use a pool of connections
        let db = windmill_common::connect_db(server_mode, indexer_mode, worker_mode).await?;
        Connection::Sql(db)
    };

    let (killpill_tx, mut killpill_rx) = KillpillSender::new(2);
    let mut monitor_killpill_rx = killpill_tx.subscribe();
    let (killpill_phase2_tx, _killpill_phase2_rx) = tokio::sync::broadcast::channel::<()>(2);
    let server_killpill_rx = killpill_phase2_tx.subscribe();

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

    #[cfg(feature = "enterprise")]
    {
        // load the license key and check if it's valid
        // if not valid and not server mode just quit
        // if not expired and server mode then force renewal
        // if key still invalid and num_workers > 0, set to 0
        if let Err(err) = reload_license_key(&conn).await {
            tracing::error!("Failed to reload license key: {err:#}");
        }
        let valid_key = *LICENSE_KEY_VALID.read().await;
        if !valid_key && !server_mode {
            tracing::error!("Invalid license key, workers require a valid license key");
        }
        if server_mode || mcp_mode {
            if let Some(db) = conn.as_sql() {
                // only force renewal if invalid but not empty (= expired)
                let renewed_now = maybe_renew_license_key_on_start(
                    &HTTP_CLIENT,
                    &db,
                    !valid_key && !LICENSE_KEY_ID.read().await.is_empty(),
                )
                .await;
                if renewed_now {
                    if let Err(err) = reload_license_key(&conn).await {
                        tracing::error!("Failed to reload license key: {err:#}");
                    }
                }
            } else {
                panic!("Server mode requires a database connection");
            }
        }
    }

    if server_mode || worker_mode || indexer_mode || mcp_mode {
        let port_var = std::env::var("PORT").ok().and_then(|x| x.parse().ok());

        let port = if server_mode || indexer_mode || mcp_mode {
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

        initial_load(
            &conn,
            killpill_tx.clone(),
            worker_mode,
            server_mode,
            #[cfg(feature = "parquet")]
            disable_s3_store,
        )
        .await;

        monitor_db(
            &conn,
            &base_internal_url,
            server_mode,
            worker_mode,
            true,
            killpill_tx.clone(),
        )
        .await;

        #[cfg(feature = "prometheus")]
        if let Some(db) = conn.as_sql() {
            crate::monitor::monitor_pool(&db).await;
        }

        send_logs_to_object_store(&conn, &hostname, &mode);

        #[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
        if !worker_mode {
            monitor_mem().await;
        }

        let addr = SocketAddr::from((server_bind_address, port));

        let (base_internal_tx, base_internal_rx) = tokio::sync::oneshot::channel::<String>();

        DirBuilder::new()
            .recursive(true)
            .create("/tmp/windmill")
            .expect("could not create initial server dir");

        #[cfg(feature = "tantivy")]
        let should_index_jobs = mode == Mode::Indexer || mode_and_addons.indexer;

        #[cfg(feature = "tantivy")]
        if should_index_jobs {
            if let Some(db) = conn.as_sql() {
                reload_indexer_config(&db).await;
            }
        }

        #[cfg(feature = "tantivy")]
        let (index_reader, index_writer) = if should_index_jobs {
            if let Some(db) = conn.as_sql() {
                let mut indexer_rx = killpill_rx.resubscribe();

                let (mut reader, mut writer) = (None, None);
                tokio::select! {
                _ = indexer_rx.recv() => {
                    tracing::info!("Received killpill, aborting index initialization");
                },
                res = windmill_indexer::completed_runs_ee::init_index(&db) => {
                        let res = res?;
                        reader = Some(res.0);
                        writer = Some(res.1);
                }

                }
                (reader, writer)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        #[cfg(feature = "tantivy")]
        let indexer_f = {
            let indexer_rx = killpill_rx.resubscribe();
            let index_writer2 = index_writer.clone();
            async {
                if let Some(db) = conn.as_sql() {
                    if let Some(index_writer) = index_writer2 {
                        windmill_indexer::completed_runs_ee::run_indexer(
                            db.clone(),
                            index_writer,
                            indexer_rx,
                        )
                        .await?;
                    }
                }
                Ok(())
            }
        };

        #[cfg(all(feature = "tantivy", feature = "parquet"))]
        let (log_index_reader, log_index_writer) = if should_index_jobs {
            if let Some(db) = conn.as_sql() {
                let mut indexer_rx = killpill_rx.resubscribe();

                let (mut reader, mut writer) = (None, None);
                tokio::select! {
                    _ = indexer_rx.recv() => {
                        tracing::info!("Received killpill, aborting index initialization");
                    },
                    res = windmill_indexer::service_logs_ee::init_index(&db, killpill_tx.clone()) => {
                            let res = res?;
                            reader = Some(res.0);
                            writer = Some(res.1);
                    }

                }
                (reader, writer)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        #[cfg(all(feature = "tantivy", feature = "parquet"))]
        let log_indexer_f = {
            let log_indexer_rx = killpill_rx.resubscribe();
            let log_index_writer2 = log_index_writer.clone();
            async {
                if let Some(db) = conn.as_sql() {
                    if let Some(log_index_writer) = log_index_writer2 {
                        windmill_indexer::service_logs_ee::run_indexer(
                            db.clone(),
                            log_index_writer,
                            log_indexer_rx,
                        )
                        .await?;
                    }
                }
                Ok(())
            }
        };

        #[cfg(not(feature = "tantivy"))]
        let index_reader = None;

        #[cfg(not(feature = "tantivy"))]
        let indexer_f = async { Ok(()) as anyhow::Result<()> };

        #[cfg(not(all(feature = "tantivy", feature = "parquet")))]
        let log_index_reader = None;

        #[cfg(not(all(feature = "tantivy", feature = "parquet")))]
        let log_indexer_f = async { Ok(()) as anyhow::Result<()> };

        let server_f = async {
            if !is_agent {
                if let Some(db) = conn.as_sql() {
                    windmill_api::run_server(
                        db.clone(),
                        index_reader,
                        log_index_reader,
                        addr,
                        server_killpill_rx,
                        base_internal_tx,
                        server_mode,
                        mcp_mode,
                        base_internal_url.clone(),
                    )
                    .await?;
                }
            } else {
                base_internal_tx
                    .send(base_internal_url.clone())
                    .map_err(|e| {
                        anyhow::anyhow!("Could not send base_internal_url to agent: {e:#}")
                    })?;
            }
            Ok(()) as anyhow::Result<()>
        };

        let workers_f = async {
            let mut rx = killpill_rx.resubscribe();

            if !killpill_rx.try_recv().is_ok() {
                let base_internal_url = base_internal_rx.await?;
                if worker_mode {
                    let mut workers = vec![];
                    for i in 0..num_workers {
                        let suffix: String = if i == 0 && first_suffix.as_ref().is_some() {
                            first_suffix.as_ref().unwrap().clone()
                        } else {
                            windmill_common::utils::worker_suffix(&hostname, &rd_string(5))
                        };
                        let worker_conn = WorkerConn {
                            conn: if i == 0 || mode != Mode::Agent {
                                conn.clone()
                            } else {
                                Connection::Http(build_agent_http_client(&suffix))
                            },
                            worker_name: windmill_common::utils::worker_name_with_suffix(
                                mode == Mode::Agent,
                                WORKER_GROUP.as_str(),
                                &suffix,
                            ),
                        };
                        workers.push(worker_conn);
                    }

                    run_workers(
                        rx,
                        killpill_tx.clone(),
                        base_internal_url.clone(),
                        hostname.clone(),
                        &workers,
                    )
                    .await?;
                    tracing::info!("All workers exited.");
                    killpill_tx.send();
                } else {
                    rx.recv().await?;
                }
            }
            if killpill_phase2_tx.receiver_count() > 0 {
                if worker_mode {
                    tracing::info!("Starting phase 2 of shutdown");
                }
                killpill_phase2_tx.send(())?;
                if worker_mode {
                    tracing::info!("Phase 2 of shutdown completed");
                }
            }
            Ok(())
        };

        let monitor_f = async {
            let tx = killpill_tx.clone();
            let conn = conn.clone();
            match conn {
                Connection::Sql(ref db) => {
                    let base_internal_url = base_internal_url.to_string();
                    let db_url: String = get_database_url().await?;
                    let db = db.clone();
                    let h = tokio::spawn(async move {
                        let mut listener = retry_listen_pg(&db_url).await;
                        let mut last_listener_refresh = Instant::now();
                        loop {
                            let db = db.clone();
                            tokio::select! {
                                biased;
                                Some(_) = async { if let Some(jh) = migration_handle.take() {
                                    tracing::info!("migration job finished");
                                    Some(jh.await)
                                } else {
                                    None
                                }} => {
                                   continue;
                                },
                                _ = monitor_killpill_rx.recv() => {
                                    tracing::info!("received killpill for monitor job");
                                    break;
                                },
                                notification = listener.try_recv() => {
                                    match notification {
                                        Ok(n) => {
                                            if n.is_none() {
                                                tracing::error!("Could not receive notification, attempting to reconnect to pg listener");
                                                continue;
                                            }
                                            let n = n.unwrap();
                                            tracing::info!("Received new pg notification: {n:?}");
                                            match n.channel() {
                                                "notify_config_change" => {
                                                    match n.payload() {
                                                        "server" if server_mode => {
                                                            tracing::error!("Server config change detected but server config is obsolete: {}", n.payload());
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
                                                "notify_webhook_change" => {
                                                    let workspace_id = n.payload();
                                                    tracing::info!("Webhook change detected, invalidating webhook cache: {}", workspace_id);
                                                    windmill_api::webhook_util::WEBHOOK_CACHE.remove(workspace_id);
                                                },
                                                "notify_workspace_envs_change" => {
                                                    let workspace_id = n.payload();
                                                    tracing::info!("Workspace envs change detected, invalidating workspace envs cache: {}", workspace_id);
                                                    windmill_common::variables::CUSTOM_ENVS_CACHE.remove(workspace_id);
                                                },
                                                "notify_workspace_premium_change" => {
                                                    let workspace_id = n.payload();
                                                    tracing::info!("Workspace premium change detected, invalidating workspace premium cache: {}", workspace_id);
                                                    windmill_common::workspaces::IS_PREMIUM_CACHE.remove(workspace_id);
                                                },
                                                "notify_runnable_version_change" => {
                                                    let payload = n.payload();
                                                    tracing::info!("Runnable version change detected: {}", payload);
                                                    match payload.split(':').collect::<Vec<&str>>().as_slice() {
                                                        [workspace_id, source_type, path, kind] => {
                                                            let key = (workspace_id.to_string(), path.to_string());
                                                            match source_type {
                                                                &"script" => {
                                                                    windmill_common::DEPLOYED_SCRIPT_HASH_CACHE.remove(&key);
                                                                    match kind {
                                                                        &"preprocessor" => {
                                                                            match sqlx::query_scalar!(
                                                                                "SELECT fv.id
                                                                                FROM flow f
                                                                                INNER JOIN flow_version fv ON fv.id = f.versions[array_upper(f.versions, 1)]
                                                                                WHERE fv.value->'preprocessor_module'->'value'->>'path' = $1 AND f.workspace_id = $2",
                                                                                path,
                                                                                workspace_id
                                                                            ).fetch_all(&db).await {
                                                                                Ok(flow_versions) => {
                                                                                    tracing::debug!("Workspace preprocessor {} changed, removing runnable format version cache for flow versions {:?}", path, flow_versions);
                                                                                    for version in flow_versions {
                                                                                        for trigger_kind in TriggerKind::iter() {
                                                                                            let key = (windmill_common::triggers::HubOrWorkspaceId::WorkspaceId(workspace_id.to_string()), version, trigger_kind);
                                                                                            windmill_common::triggers::RUNNABLE_FORMAT_VERSION_CACHE.remove(&key);
                                                                                        }
                                                                                    }
                                                                                }
                                                                                Err(e) => {
                                                                                    tracing::error!("Error fetching flow paths: {e:#}");
                                                                                }
                                                                            }
                                                                        },
                                                                        _ => {}
                                                                    }
                                                                }
                                                                &"flow" => {
                                                                    windmill_common::FLOW_VERSION_CACHE.remove(&key);
                                                                },
                                                                _ => {
                                                                    tracing::warn!("Unknown runnable version change payload: {}", payload);
                                                                }
                                                            }
                                                        },
                                                        _ => {
                                                            tracing::warn!("Unknown runnable version change payload: {}", payload);
                                                        }
                                                    }
                                                },
                                                #[cfg(feature = "http_trigger")]
                                                "notify_http_trigger_change" => {
                                                    tracing::info!("HTTP trigger change detected: {}", n.payload());
                                                    match windmill_api::http_triggers::refresh_routers(&db).await {
                                                        Ok((true, _)) => {
                                                            tracing::info!("Refreshed HTTP routers (trigger change)");
                                                        },
                                                        Ok((false, _)) => {
                                                            tracing::warn!("Should have refreshed HTTP routers (trigger change) but did not");
                                                        },
                                                        Err(err) => {
                                                            tracing::error!("Error refreshing HTTP routers (trigger change): {err:#}");
                                                        }
                                                    };
                                                },
                                                "notify_global_setting_change" => {
                                                    tracing::info!("Global setting change detected: {}", n.payload());
                                                    match n.payload() {
                                                        BASE_URL_SETTING => {
                                                            if let Err(e) = reload_base_url_setting(&conn).await {
                                                                tracing::error!(error = %e, "Could not reload base url setting");
                                                            }
                                                        },
                                                        OAUTH_SETTING => {
                                                            if let Err(e) = reload_base_url_setting(&conn).await {
                                                                tracing::error!(error = %e, "Could not reload oauth setting");
                                                            }
                                                        },
                                                        CUSTOM_TAGS_SETTING => {
                                                            if let Err(e) = reload_custom_tags_setting(&db).await {
                                                                tracing::error!(error = %e, "Could not reload custom tags setting");
                                                            }
                                                        },
                                                        LICENSE_KEY_SETTING => {
                                                            if let Err(e) = reload_license_key(&db.into()).await {
                                                                tracing::error!("Failed to reload license key: {e:#}");
                                                            }
                                                        },
                                                        DEFAULT_TAGS_PER_WORKSPACE_SETTING => {
                                                            if let Err(e) = load_tag_per_workspace_enabled(&db).await {
                                                                tracing::error!("Error loading default tag per workspace: {e:#}");
                                                            }
                                                        },
                                                        DEFAULT_TAGS_WORKSPACES_SETTING => {
                                                            if let Err(e) = load_tag_per_workspace_workspaces(&db).await {
                                                                tracing::error!("Error loading default tag per workspace workspaces: {e:#}");
                                                            }
                                                        }
                                                        SMTP_SETTING => {
                                                            reload_smtp_config(&db).await;
                                                        },
                                                        TEAMS_SETTING => {
                                                            tracing::info!("Teams setting changed.");
                                                        },
                                                        INDEXER_SETTING => {
                                                            reload_indexer_config(&db).await;
                                                        },
                                                        TIMEOUT_WAIT_RESULT_SETTING => {
                                                            reload_timeout_wait_result_setting(&conn).await
                                                        },
                                                        RETENTION_PERIOD_SECS_SETTING => {
                                                            reload_retention_period_setting(&conn).await
                                                        },
                                                        MONITOR_LOGS_ON_OBJECT_STORE_SETTING => {
                                                            reload_delete_logs_periodically_setting(&conn).await
                                                        },
                                                        JOB_DEFAULT_TIMEOUT_SECS_SETTING => {
                                                            reload_job_default_timeout_setting(&conn).await
                                                        },
                                                        #[cfg(feature = "parquet")]
                                                        OBJECT_STORE_CACHE_CONFIG_SETTING => {
                                                            if !disable_s3_store {
                                                                reload_s3_cache_setting(&db).await
                                                            }
                                                        },
                                                        SCIM_TOKEN_SETTING => {
                                                            reload_scim_token_setting(&conn).await
                                                        },
                                                        EXTRA_PIP_INDEX_URL_SETTING => {
                                                            reload_extra_pip_index_url_setting(&conn).await
                                                        },
                                                        PIP_INDEX_URL_SETTING => {
                                                            reload_pip_index_url_setting(&conn).await
                                                        },
                                                        INSTANCE_PYTHON_VERSION_SETTING => {
                                                            reload_instance_python_version_setting(&conn).await
                                                        },
                                                        NPM_CONFIG_REGISTRY_SETTING => {
                                                            reload_npm_config_registry_setting(&conn).await
                                                        },
                                                        BUNFIG_INSTALL_SCOPES_SETTING => {
                                                            reload_bunfig_install_scopes_setting(&conn).await
                                                        },
                                                        NUGET_CONFIG_SETTING => {
                                                            reload_nuget_config_setting(&conn).await
                                                        },
                                                        MAVEN_REPOS_SETTING => {
                                                            reload_maven_repos_setting(&conn).await
                                                        },
                                                        NO_DEFAULT_MAVEN_SETTING => {
                                                            reload_no_default_maven_setting(&conn).await
                                                        },
                                                        KEEP_JOB_DIR_SETTING => {
                                                            load_keep_job_dir(&conn).await;
                                                        },
                                                        REQUIRE_PREEXISTING_USER_FOR_OAUTH_SETTING => {
                                                            load_require_preexisting_user(&db).await;
                                                        },
                                                        EXPOSE_METRICS_SETTING  => {
                                                            tracing::info!("Metrics setting changed, restarting");
                                                            send_delayed_killpill(&tx, 40, "metrics setting change").await;
                                                        },
                                                        EMAIL_DOMAIN_SETTING => {
                                                            tracing::info!("Email domain setting changed");
                                                            if server_mode {
                                                                send_delayed_killpill(&tx, 4, "email domain setting change").await;
                                                            }
                                                        },
                                                        EXPOSE_DEBUG_METRICS_SETTING => {
                                                            if let Err(e) = load_metrics_debug_enabled(&conn).await {
                                                                tracing::error!(error = %e, "Could not reload debug metrics setting");
                                                            }
                                                        },
                                                        OTEL_SETTING => {
                                                            tracing::info!("OTEL setting changed, restarting");
                                                            send_delayed_killpill(&tx, 4, "OTEL setting change").await;
                                                        },
                                                        REQUEST_SIZE_LIMIT_SETTING => {
                                                            if server_mode {
                                                                tracing::info!("Request limit size change detected, killing server expecting to be restarted");
                                                                send_delayed_killpill(&tx, 4, "request size limit change").await;
                                                            }
                                                        },
                                                        SAML_METADATA_SETTING => {
                                                            tracing::info!("SAML metadata change detected, killing server expecting to be restarted");
                                                            send_delayed_killpill(&tx, 0, "SAML metadata change").await;
                                                        },
                                                        HUB_BASE_URL_SETTING => {
                                                            if let Err(e) = reload_hub_base_url_setting(&conn, server_mode).await {
                                                                tracing::error!(error = %e, "Could not reload hub base url setting");
                                                            }
                                                        },
                                                        CRITICAL_ERROR_CHANNELS_SETTING => {
                                                            if let Err(e) = reload_critical_error_channels_setting(&db).await {
                                                                tracing::error!(error = %e, "Could not reload critical error emails setting");
                                                            }
                                                        },
                                                        CRITICAL_ALERTS_ON_DB_OVERSIZE_SETTING => {
                                                            if let Err(e) = reload_critical_alerts_on_db_oversize(&db).await {
                                                                tracing::error!(error = %e, "Could not reload critical alerts on db oversize setting");
                                                            }

                                                        },
                                                        JWT_SECRET_SETTING => {
                                                            if let Err(e) = reload_jwt_secret_setting(&db).await {
                                                                tracing::error!(error = %e, "Could not reload jwt secret setting");
                                                            }
                                                        },
                                                        CRITICAL_ALERT_MUTE_UI_SETTING => {
                                                            tracing::info!("Critical alert UI setting changed");
                                                            if let Err(e) = reload_critical_alert_mute_ui_setting(&conn).await {
                                                                tracing::error!(error = %e, "Could not reload critical alert UI setting");
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
                                            tokio::select! {
                                                biased;
                                                _ = monitor_killpill_rx.recv() => {
                                                    tracing::info!("received killpill for monitor job");
                                                    break;
                                                },
                                                new_listener = retry_listen_pg(&db_url) => {
                                                    listener = new_listener;
                                                    continue;
                                                }
                                            }
                                        }
                                    };
                                },
                                _ = tokio::time::sleep(Duration::from_secs(30))    => {
                                    if last_listener_refresh.elapsed() > Duration::from_secs(*PG_LISTENER_REFRESH_PERIOD_SECS) {
                                        tracing::info!("Refreshing pg listeners, settings and license key after {}s", Duration::from_secs(*PG_LISTENER_REFRESH_PERIOD_SECS).as_secs());
                                        if let Err(e) = listener.unlisten_all().await {
                                            tracing::error!(error = %e, "Could not unlisten to database");
                                        }
                                        listener = retry_listen_pg(&db_url).await;
                                        initial_load(
                                            &conn,
                                            tx.clone(),
                                            worker_mode,
                                            server_mode,
                                            #[cfg(feature = "parquet")]
                                            disable_s3_store,
                                        )
                                        .await;
                                        #[cfg(feature = "enterprise")]
                                        if let Err(err) = reload_license_key(&conn).await {
                                            tracing::error!("Failed to reload license key: {err:#}");
                                        }
                                        last_listener_refresh = Instant::now();
                                    }

                                    if server_mode {
                                        tracing::info!("monitor task started");
                                    }
                                    monitor_db(
                                        &conn,
                                        &base_internal_url,
                                        server_mode,
                                        worker_mode,
                                        false,
                                        tx.clone(),
                                    )
                                    .await;
                                    if server_mode {
                                        tracing::info!("monitor task finished");
                                    }
                                },
                            }
                        }
                    });

                    if let Err(e) = h.await {
                        tracing::error!("Error waiting for monitor handle: {e:#}")
                    }
                }
                Connection::Http(_) => loop {
                    tokio::select! {
                        _ = monitor_killpill_rx.recv() => {
                            tracing::info!("Received killpill, exiting");
                            break;
                        },
                        _ = tokio::time::sleep(Duration::from_secs(12 * 60 * 60)) => {
                            tracing::info!("Reloading config after 12 hours");
                            initial_load(&conn, tx.clone(), worker_mode, server_mode, #[cfg(feature = "parquet")] disable_s3_store).await;
                            #[cfg(feature = "enterprise")]
                            ee::verify_license_key().await;
                        }
                    }
                },
            };

            tracing::info!("Monitor exited");
            killpill_tx.send();
            Ok(()) as anyhow::Result<()>
        };

        let metrics_f = async {
            let enabled = METRICS_ENABLED.load(std::sync::atomic::Ordering::Relaxed);

            #[cfg(not(all(feature = "enterprise", feature = "prometheus")))]
            if enabled {
                tracing::error!("Metrics are only available in the EE, ignoring...");
            }

            #[cfg(all(feature = "enterprise", feature = "prometheus"))]
            if let Err(e) = windmill_common::serve_metrics(
                *windmill_common::METRICS_ADDR,
                _killpill_phase2_rx,
                num_workers > 0,
                enabled,
            )
            .await
            {
                tracing::error!("Error serving metrics: {e:#}");
            }

            Ok(()) as anyhow::Result<()>
        };

        if server_mode {
            if let Some(db) = conn.as_sql() {
                schedule_stats(&db, &HTTP_CLIENT).await;
            }
        }

        if mcp_mode {
            futures::try_join!(shutdown_signal, workers_f, server_f)?;
        } else {
            futures::try_join!(
                shutdown_signal,
                workers_f,
                monitor_f,
                server_f,
                metrics_f,
                indexer_f,
                log_indexer_f
            )?;
        }
    } else {
        tracing::info!("Nothing to do, exiting.");
    }
    send_current_log_file_to_object_store(&conn, &hostname, &mode).await;

    if let Some(db) = conn.as_sql() {
        tracing::info!("Exiting connection pool");
        tokio::select! {
            _ = db.close() => {
                tracing::info!("Database connection pool closed");
            },
            _ = tokio::time::sleep(Duration::from_secs(15)) => {
                tracing::warn!("Could not close database connection pool in time (15s). Exiting anyway.");
            }
        }
    }
    Ok(())
}

async fn listen_pg(url: &str) -> Option<PgListener> {
    let mut listener = match PgListener::connect(url).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(error = %e, "Could not connect to database");
            return None;
        }
    };

    #[allow(unused_mut)]
    let mut channels = vec![
        "notify_config_change",
        "notify_global_setting_change",
        "notify_webhook_change",
        "notify_workspace_envs_change",
        "notify_runnable_version_change",
    ];

    #[cfg(feature = "http_trigger")]
    channels.push("notify_http_trigger_change");

    #[cfg(feature = "cloud")]
    channels.push("notify_workspace_premium_change");

    if let Err(e) = listener.listen_all(channels).await {
        tracing::error!(error = %e, "Could not listen to database");
        return None;
    }

    return Some(listener);
}

async fn retry_listen_pg(url: &str) -> PgListener {
    let mut listener = listen_pg(url).await;
    loop {
        if listener.is_none() {
            tracing::info!("Retrying listening to pg listen in 5 seconds");
            tokio::time::sleep(Duration::from_secs(5)).await;
            listener = listen_pg(url).await;
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

pub struct WorkerConn {
    conn: Connection,
    worker_name: String,
}

pub async fn run_workers(
    mut rx: tokio::sync::broadcast::Receiver<()>,
    tx: KillpillSender,
    base_internal_url: String,
    hostname: String,
    workers: &[WorkerConn],
) -> anyhow::Result<()> {
    let mut killpill_rxs = vec![];
    let num_workers = workers.len();
    for _ in 0..num_workers {
        killpill_rxs.push(rx.resubscribe());
    }

    if rx.try_recv().is_ok() {
        tracing::info!("Received killpill, exiting");
        return Ok(());
    }

    // #[cfg(tokio_unstable)]
    // let monitor = tokio_metrics::TaskMonitor::new();

    let ip = windmill_common::external_ip::get_ip()
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = e.to_string(), "failed to get external IP");
            "unretrievable IP".to_string()
        });

    let mut handles = Vec::with_capacity(num_workers as usize);

    for x in [
        TMP_LOGS_DIR,
        UV_CACHE_DIR,
        DENO_CACHE_DIR,
        DENO_CACHE_DIR_DEPS,
        DENO_CACHE_DIR_NPM,
        BUN_CACHE_DIR,
        PY310_CACHE_DIR,
        PY311_CACHE_DIR,
        PY312_CACHE_DIR,
        PY313_CACHE_DIR,
        TAR_PY310_CACHE_DIR,
        TAR_PY311_CACHE_DIR,
        TAR_PY312_CACHE_DIR,
        TAR_PY313_CACHE_DIR,
        BUN_BUNDLE_CACHE_DIR,
        GO_CACHE_DIR,
        GO_BIN_CACHE_DIR,
        RUST_CACHE_DIR,
        CSHARP_CACHE_DIR,
        NU_CACHE_DIR,
        HUB_CACHE_DIR,
        POWERSHELL_CACHE_DIR,
        JAVA_CACHE_DIR,
        TAR_JAVA_CACHE_DIR, // for related places search: ADD_NEW_LANG
    ] {
        DirBuilder::new()
            .recursive(true)
            .create(x)
            .expect("could not create initial worker dir");
    }

    tracing::info!(
        "Starting {num_workers} workers and SLEEP_QUEUE={}ms",
        *windmill_worker::SLEEP_QUEUE
    );

    for i in 1..(num_workers + 1) {
        let wk_conf = &workers[i as usize - 1];
        let conn1 = wk_conf.conn.clone();
        let worker_name = wk_conf.worker_name.clone();
        WORKERS_NAMES.write().await.push(worker_name.clone());
        let ip = ip.clone();
        let rx = killpill_rxs.pop().unwrap();
        let tx = tx.clone();
        let base_internal_url = base_internal_url.clone();
        let hostname = hostname.clone();

        handles.push(tokio::spawn(async move {
            if num_workers > 1 {
                tracing::info!(worker = %worker_name, "starting worker {i}");
            }

            let f = windmill_worker::run_worker(
                &conn1,
                &hostname,
                worker_name,
                i as u64,
                num_workers as u32,
                &ip,
                rx,
                tx,
                &base_internal_url,
            );

            // #[cfg(tokio_unstable)]
            // {
            //     monitor.monitor(f, "worker").await
            // }

            // #[cfg(not(tokio_unstable))]
            // {
            f.await
            // }
        }));
    }

    futures::future::try_join_all(handles).await?;
    Ok(())
}

async fn send_delayed_killpill(tx: &KillpillSender, mut max_delay_secs: u64, context: &str) {
    if max_delay_secs == 0 {
        max_delay_secs = 1;
    }
    // Random delay to avoid all servers/workers shutting down simultaneously
    let rd_delay = rand::rng().random_range(0..max_delay_secs);
    tracing::info!("Scheduling {context} shutdown in {rd_delay}s");
    tokio::time::sleep(Duration::from_secs(rd_delay)).await;

    tx.send();
}
