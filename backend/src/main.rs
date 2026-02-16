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
    reload_maven_settings_xml_setting, reload_no_default_maven_setting,
    reload_nuget_config_setting, reload_powershell_repo_pat_setting,
    reload_powershell_repo_url_setting, reload_ruby_repos_setting,
    reload_timeout_wait_result_setting, send_current_log_file_to_object_store,
    send_logs_to_object_store, WORKERS_NAMES,
};
use rand::Rng;
use sqlx::{Pool, Postgres};
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
use windmill_common::ee_oss::{
    maybe_renew_license_key_on_start, LICENSE_KEY_ID, LICENSE_KEY_VALID,
};

use windmill_common::{
    agent_workers::AgentConfig,
    global_settings::{
        APP_WORKSPACED_ROUTE_SETTING, BASE_URL_SETTING, BUNFIG_INSTALL_SCOPES_SETTING,
        CRITICAL_ALERTS_ON_DB_OVERSIZE_SETTING, CRITICAL_ALERT_MUTE_UI_SETTING,
        CRITICAL_ERROR_CHANNELS_SETTING, CUSTOM_TAGS_SETTING, DEFAULT_TAGS_PER_WORKSPACE_SETTING,
        DEFAULT_TAGS_WORKSPACES_SETTING, EMAIL_DOMAIN_SETTING, ENV_SETTINGS,
        EXPOSE_DEBUG_METRICS_SETTING, EXPOSE_METRICS_SETTING, EXTRA_PIP_INDEX_URL_SETTING,
        HUB_API_SECRET_SETTING, HUB_BASE_URL_SETTING, INDEXER_SETTING,
        INSTANCE_PYTHON_VERSION_SETTING, JOB_DEFAULT_TIMEOUT_SECS_SETTING, JOB_ISOLATION_SETTING,
        JWT_SECRET_SETTING, KEEP_JOB_DIR_SETTING, LICENSE_KEY_SETTING, MAVEN_REPOS_SETTING,
        MAVEN_SETTINGS_XML_SETTING, MONITOR_LOGS_ON_OBJECT_STORE_SETTING, NO_DEFAULT_MAVEN_SETTING,
        NPM_CONFIG_REGISTRY_SETTING, NUGET_CONFIG_SETTING, OAUTH_SETTING, OTEL_SETTING,
        OTEL_TRACING_PROXY_SETTING, PIP_INDEX_URL_SETTING, POWERSHELL_REPO_PAT_SETTING,
        POWERSHELL_REPO_URL_SETTING, REQUEST_SIZE_LIMIT_SETTING,
        REQUIRE_PREEXISTING_USER_FOR_OAUTH_SETTING, RETENTION_PERIOD_SECS_SETTING,
        RUBY_REPOS_SETTING, SAML_METADATA_SETTING, SCIM_TOKEN_SETTING, SMTP_SETTING, TEAMS_SETTING,
        TIMEOUT_WAIT_RESULT_SETTING, UV_INDEX_STRATEGY_SETTING,
    },
    scripts::ScriptLang,
    stats_oss::schedule_stats,
    triggers::TriggerKind,
    utils::{
        create_default_worker_suffix, worker_name_with_suffix, Mode, GIT_VERSION, HOSTNAME,
        MODE_AND_ADDONS,
    },
    worker::{
        is_native_mode_from_env, reload_custom_tags_setting, Connection, HUB_CACHE_DIR,
        HUB_RT_CACHE_DIR, NATIVE_MODE_RESOLVED, TMP_DIR, TMP_LOGS_DIR, WORKER_GROUP,
    },
    KillpillSender, DEFAULT_HUB_BASE_URL, METRICS_ENABLED,
};

#[cfg(feature = "enterprise")]
use windmill_common::worker::CLOUD_HOSTED;

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
use monitor::monitor_mem;

#[cfg(any(target_os = "linux"))]
use crate::cgroups::disable_oom_group;

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[cfg(feature = "parquet")]
use windmill_common::global_settings::OBJECT_STORE_CONFIG_SETTING;

use windmill_worker::{
    get_hub_script_content_and_requirements, init_worker_internal_server_inline_utils,
    BUN_BUNDLE_CACHE_DIR, BUN_CACHE_DIR, CSHARP_CACHE_DIR, DENO_CACHE_DIR, DENO_CACHE_DIR_DEPS,
    DENO_CACHE_DIR_NPM, GO_BIN_CACHE_DIR, GO_CACHE_DIR, JAVA_CACHE_DIR, NU_CACHE_DIR,
    POWERSHELL_CACHE_DIR, PY310_CACHE_DIR, PY311_CACHE_DIR, PY312_CACHE_DIR, PY313_CACHE_DIR,
    RUBY_CACHE_DIR, RUST_CACHE_DIR, TAR_JAVA_CACHE_DIR, UV_CACHE_DIR,
};

use crate::monitor::{
    initial_load, load_keep_job_dir, load_metrics_debug_enabled, load_require_preexisting_user,
    load_tag_per_workspace_enabled, load_tag_per_workspace_workspaces, monitor_db,
    reload_app_workspaced_route_setting, reload_base_url_setting,
    reload_bunfig_install_scopes_setting, reload_critical_alert_mute_ui_setting,
    reload_critical_error_channels_setting, reload_extra_pip_index_url_setting,
    reload_hub_api_secret_setting, reload_hub_base_url_setting, reload_job_default_timeout_setting,
    reload_job_isolation_setting, reload_jwt_secret_setting, reload_license_key,
    reload_npm_config_registry_setting, reload_otel_tracing_proxy_setting,
    reload_pip_index_url_setting, reload_retention_period_setting, reload_scim_token_setting,
    reload_smtp_config, reload_uv_index_strategy_setting, reload_worker_config, MonitorIteration,
};

#[cfg(feature = "parquet")]
use windmill_common::s3_helpers::reload_object_store_setting;

const DEFAULT_NUM_WORKERS: usize = 1;
const DEFAULT_PORT: u16 = 8000;
const DEFAULT_SERVER_BIND_ADDR: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
const DEFAULT_WORKER_BIND_ADDR: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const BIND_ADDR_ENV: &str = "SERVER_BIND_ADDR";

#[cfg(target_os = "linux")]
mod cgroups;
mod db_connect;
#[cfg(feature = "private")]
pub mod ee;
mod ee_oss;
mod monitor;

// Windows service support - EE feature
#[cfg(all(windows, feature = "enterprise", feature = "private"))]
mod windows_service_ee;

pub fn setup_deno_runtime() -> anyhow::Result<()> {
    #[cfg(feature = "deno_core")]
    windmill_runtime_nativets::setup_deno_runtime()?;
    Ok(())
}

fn update_ca_certificates_if_requested() {
    if std::env::var("RUN_UPDATE_CA_CERTIFICATE_AT_START")
        .ok()
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false)
    {
        let ca_cert_path = std::env::var("RUN_UPDATE_CA_CERTIFICATE_PATH")
            .unwrap_or_else(|_| "/usr/sbin/update-ca-certificates".to_string());

        println!(
            "RUN_UPDATE_CA_CERTIFICATE_AT_START=true, running: {}",
            ca_cert_path
        );

        let output = std::process::Command::new(&ca_cert_path).output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    println!("Successfully updated CA certificates");
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    println!(
                        "Failed to update CA certificates, but continuing startup: {}",
                        stderr.trim()
                    );
                }
            }
            Err(e) => {
                println!(
                    "Could not run update-ca-certificates command, but continuing startup: {}",
                    e
                );
            }
        }
    }
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
    // Period in seconds between full settings reload (12 hours by default)
    static ref SETTINGS_RELOAD_PERIOD_SECS: u64 = std::env::var("SETTINGS_RELOAD_PERIOD_SECS")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or(3600 * 12);

    // Period in seconds between polling for notify events (10s by default)
    static ref LISTEN_NEW_EVENTS_INTERVAL_SEC: u64 = std::env::var("LISTEN_NEW_EVENTS_INTERVAL_SEC")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or(10);
}

pub fn main() -> anyhow::Result<()> {
    // On Windows with enterprise feature, check if running as a service
    #[cfg(all(windows, feature = "enterprise", feature = "private"))]
    {
        if windows_service_ee::is_running_as_service() {
            // Run as Windows service with SCM handlers
            return windows_service_ee::run_as_windows_service()
                .map_err(|e| anyhow::anyhow!("Failed to run as Windows service: {}", e));
        }
    }

    // Normal execution (console/foreground mode)
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
            if let Some(lock) = res.lockfile {
                let _ = windmill_worker::prepare_job_dir(&lock, &job_dir).await?;
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
                    &lock,
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

/// Raw resource type from hub API (schema is a JSON string)
#[derive(serde::Deserialize)]
struct HubResourceTypeRaw {
    pub id: i64,
    pub name: String,
    pub schema: Option<String>,
    pub app: String,
    pub description: Option<String>,
}

/// Processed resource type with parsed schema
#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct HubResourceType {
    pub id: i64,
    pub name: String,
    pub schema: Option<serde_json::Value>,
    pub app: String,
    pub description: Option<String>,
}

const HUB_RT_CACHE_FILE: &str = "resource_types.json";

async fn cache_hub_resource_types() -> anyhow::Result<()> {
    println!("Caching resource types from hub...");

    let response = HTTP_CLIENT
        .get(format!("{}/resource_types/list", DEFAULT_HUB_BASE_URL))
        .header("Accept", "application/json")
        .send()
        .await
        .with_context(|| "Failed to fetch resource types from hub")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch resource types from hub: {}",
            response.status()
        );
    }

    let raw_types: Vec<HubResourceTypeRaw> = response
        .json::<Vec<HubResourceTypeRaw>>()
        .await
        .with_context(|| "Failed to parse resource types from hub")?;

    // Parse schema strings into JSON values
    let resource_types: Vec<HubResourceType> = raw_types
        .into_iter()
        .filter_map(|rt| {
            let schema = match rt.schema {
                Some(s) => match serde_json::from_str(&s) {
                    Ok(v) => Some(v),
                    Err(e) => {
                        println!("Warning: failed to parse schema for {}: {}", rt.name, e);
                        return None;
                    }
                },
                None => None,
            };
            Some(HubResourceType {
                id: rt.id,
                name: rt.name,
                schema,
                app: rt.app,
                description: rt.description,
            })
        })
        .collect();

    println!("Fetched {} resource types from hub", resource_types.len());

    create_dir_all(HUB_RT_CACHE_DIR)?;

    let cache_path = format!("{}/{}", HUB_RT_CACHE_DIR, HUB_RT_CACHE_FILE);
    let content = serde_json::to_string_pretty(&resource_types)
        .with_context(|| "Failed to serialize resource types")?;

    std::fs::write(&cache_path, content)
        .with_context(|| format!("Failed to write cache file to {}", cache_path))?;

    println!("Cached resource types to {}", cache_path);
    Ok(())
}

pub async fn sync_cached_resource_types(db: &sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
    let cache_path = format!("{}/{}", HUB_RT_CACHE_DIR, HUB_RT_CACHE_FILE);

    if tokio::fs::metadata(&cache_path).await.is_err() {
        tracing::info!(
            "No cached resource types found at {}, skipping sync",
            cache_path
        );
        return Ok(());
    }

    tracing::info!("Syncing cached resource types to admins workspace...");

    let content = tokio::fs::read_to_string(&cache_path)
        .await
        .with_context(|| format!("Failed to read cache file from {}", cache_path))?;

    let cached_types: Vec<HubResourceType> =
        serde_json::from_str(&content).with_context(|| "Failed to parse cached resource types")?;

    tracing::info!("Found {} cached resource types", cached_types.len());

    // Get existing resource types in admins workspace
    let existing_types: Vec<(String, Option<serde_json::Value>, Option<String>)> = sqlx::query_as(
        "SELECT name, schema, description FROM resource_type WHERE workspace_id = 'admins'",
    )
    .fetch_all(db)
    .await
    .with_context(|| "Failed to fetch existing resource types")?;

    let existing_map: std::collections::HashMap<
        String,
        (Option<serde_json::Value>, Option<String>),
    > = existing_types
        .into_iter()
        .map(|(name, schema, desc)| (name, (schema, desc)))
        .collect();

    let mut synced_count = 0;
    let mut skipped_count = 0;

    for rt in cached_types {
        // Check if resource type already exists with same schema and description
        if let Some((existing_schema, existing_desc)) = existing_map.get(&rt.name) {
            if existing_schema == &rt.schema && existing_desc == &rt.description {
                skipped_count += 1;
                continue;
            }
        }

        // Insert or update resource type
        sqlx::query(
            "INSERT INTO resource_type (workspace_id, name, schema, description, edited_at)
             VALUES ('admins', $1, $2, $3, now())
             ON CONFLICT (workspace_id, name) DO UPDATE
             SET schema = EXCLUDED.schema, description = EXCLUDED.description, edited_at = now()",
        )
        .bind(&rt.name)
        .bind(&rt.schema)
        .bind(&rt.description)
        .execute(db)
        .await
        .with_context(|| format!("Failed to upsert resource type {}", rt.name))?;

        synced_count += 1;
    }

    tracing::info!(
        "Synced {} resource types to admins workspace ({} skipped as unchanged)",
        synced_count,
        skipped_count
    );

    Ok(())
}

fn print_help() {
    println!("Windmill - a fast, open-source workflow engine and job runner.");
    println!();
    println!("Usage:");
    println!("  windmill [SUBCOMMAND]");
    println!();
    println!("Subcommands:");
    println!("  help | -h | --help   Show this help information and exit");
    println!("  version              Show Windmill version and exit");
    println!("  cache [hubPaths.json]  Pre-cache hub scripts (default: ./hubPaths.json)");
    println!("  cache-rt             Pre-cache hub resource types");
    println!("  sync-config <file>   Sync instance config from a YAML file to the database");
    println!("  operator             Run the Kubernetes operator (watches WindmillInstance CRDs)");
    println!("  operator crd         Print the WindmillInstance CRD YAML to stdout");
    println!();
    println!("Environment variables (name = default):");
    println!("  DATABASE_URL = <required>              The Postgres database url.");
    println!("  MODE = standalone                      Mode: standalone | worker | server | agent");
    println!("  BASE_URL = http://localhost:8000       Public base URL of your instance (overridden by instance settings)");
    println!(
        "  PORT = {}                              HTTP port (server/indexer/MCP modes)",
        DEFAULT_PORT
    );
    println!(
        "  SERVER_BIND_ADDR = <mode dependent>    IP to bind to (server: {}, worker: {})",
        DEFAULT_SERVER_BIND_ADDR, DEFAULT_WORKER_BIND_ADDR
    );
    println!(
        "  NUM_WORKERS = {}                       Number of workers (standalone/worker modes)",
        DEFAULT_NUM_WORKERS
    );
    println!("  WORKER_GROUP = default                 Worker group this worker belongs to",);
    println!("  JSON_FMT = false                       Output logs in JSON instead of logfmt");
    println!("  METRICS_ADDR = None                    (EE only) Prometheus metrics addr at /metrics; set \"true\" to use :8001");
    println!("  SUPERADMIN_SECRET = None               Virtual superadmin token (server)");
    println!("  LICENSE_KEY = None                     (EE only) Enterprise license key (workers require valid key)");
    println!("  RUN_UPDATE_CA_CERTIFICATE_AT_START = false  Run system CA update at startup");
    println!("  RUN_UPDATE_CA_CERTIFICATE_PATH = /usr/sbin/update-ca-certificates  Path to CA update tool");
    println!("  SYNC_CACHED_RT = false                 Sync cached resource types to admins workspace on server start");
    println!();
    println!("Notes:");
    println!("- Advanced and less commonly used settings are managed via the database and are omitted here.");
    println!("- At startup, Windmill logs currently set configuration keys for visibility.");
}

async fn windmill_main() -> anyhow::Result<()> {
    let (killpill_tx, mut killpill_rx) = KillpillSender::new(2);
    let mut monitor_killpill_rx = killpill_tx.subscribe();
    let (killpill_phase2_tx, _killpill_phase2_rx) = tokio::sync::broadcast::channel::<()>(2);
    let server_killpill_rx = killpill_phase2_tx.subscribe();

    let shutdown_tx = killpill_tx.clone();
    let shutdown_rx = killpill_tx.subscribe();
    tokio::spawn(async move {
        if let Err(e) = windmill_common::shutdown_signal(shutdown_tx, shutdown_rx).await {
            tracing::error!("Error in shutdown signal: {e:#}");
        }
    });

    dotenv::dotenv().ok();

    update_ca_certificates_if_requested();

    if std::env::var("RUST_LOG").is_err() {
        unsafe { std::env::set_var("RUST_LOG", "info") }
    }

    if let Err(_e) = rustls::crypto::ring::default_provider().install_default() {
        println!("Failed to install rustls crypto provider");
    }

    #[cfg(feature = "enterprise")]
    if *CLOUD_HOSTED {
        // Block access to AWS/GCP metadata endpoints for security in cloud-hosted mode.
        // This is a best-effort attempt; if it fails, just warn and continue.
        if let Err(e) = std::process::Command::new("sh")
            .arg("-c")
            .arg("iptables -A OUTPUT -d 169.254.169.254 -j DROP && iptables -A FORWARD -d 169.254.169.254 -j DROP")
            .status()
        {
            println!("Failed to run iptables to block metadata endpoint: {e}");
        } else {
            println!("Successfully blocked metadata endpoint using iptables");
        }
    }

    let hostname = HOSTNAME.to_owned();

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
        "-h" | "--help" | "help" => {
            print_help();
            return Ok(());
        }
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
        "prepare-deps" => {
            // CLI command for preparing dependencies without database access
            // Used by the debugger to install dependencies for scripts
            windmill_worker::run_prepare_deps_cli().await?;
            return Ok(());
        }
        "cache-rt" => {
            cache_hub_resource_types().await?;
            return Ok(());
        }
        "sync-config" => {
            tracing_subscriber::fmt::init();
            let path = std::env::args().nth(2).unwrap_or_else(|| {
                eprintln!("Usage: windmill sync-config <file>");
                std::process::exit(1);
            });
            let contents = tokio::fs::read_to_string(&path)
                .await
                .with_context(|| format!("Could not read config file: {path}"))?;
            let mut config: windmill_common::instance_config::InstanceConfig =
                serde_yml::from_str(&contents)
                    .with_context(|| format!("Could not parse YAML from: {path}"))?;
            windmill_common::instance_config::resolve_env_refs(&mut config.global_settings)
                .map_err(|var| anyhow::anyhow!("environment variable '{var}' not found"))?;

            tracing::info!("Connecting to database...");
            let db = crate::db_connect::initial_connection().await?;
            config.sync_to_db(&db).await?;
            tracing::info!("Synced instance config from {path}");
            return Ok(());
        }
        #[cfg(feature = "operator")]
        "operator" => {
            let sub_arg = std::env::args().nth(2).unwrap_or_default();
            if sub_arg == "crd" {
                windmill_operator::print_crd_yaml();
                return Ok(());
            }

            tracing_subscriber::fmt::init();
            tracing::info!("Starting Windmill Kubernetes operator...");
            tracing::info!("Connecting to database...");
            let db = crate::db_connect::initial_connection().await?;
            tracing::info!("Database connected. Starting controller...");
            windmill_operator::run(db).await?;
            return Ok(());
        }
        _ => {}
    }

    #[allow(unused_mut)]
    let mut num_workers = if mode == Mode::Server || mode == Mode::Indexer || mode == Mode::MCP {
        0
    } else if is_native_mode_from_env() {
        println!("Native mode enabled: forcing NUM_WORKERS=8");
        8
    } else {
        std::env::var("NUM_WORKERS")
            .ok()
            .and_then(|x| x.parse::<i32>().ok())
            .unwrap_or(DEFAULT_NUM_WORKERS as i32)
    };

    if num_workers > 1 && !is_native_mode_from_env() {
        if std::env::var("I_ACK_NUM_WORKERS_IS_UNSAFE").is_ok_and(|x| x == "1" || x == "true") {
            println!(
                "WARNING: Running with NUM_WORKERS={} without native mode. \
                 This is not recommended. Use at your own risk.",
                num_workers
            );
        } else {
            eprintln!(
                "WARNING: NUM_WORKERS={} > 1 is only safe for native workers. \
                 Falling back to NUM_WORKERS=1. Set NATIVE_MODE=true for native-only workers.",
                num_workers
            );
            num_workers = 1;
        }
    }

    let server_mode = !std::env::var("DISABLE_SERVER")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false)
        && (mode == Mode::Server || mode == Mode::Standalone);

    let indexer_mode = mode == Mode::Indexer;
    let mcp_mode = mode == Mode::MCP;

    let default_bind_addr = if server_mode || indexer_mode || mcp_mode {
        DEFAULT_SERVER_BIND_ADDR
    } else {
        DEFAULT_WORKER_BIND_ADDR
    };
    let server_bind_address: IpAddr = std::env::var(BIND_ADDR_ENV)
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(IpAddr::from(default_bind_addr));

    let (conn, first_suffix, agent_config) = if mode == Mode::Agent {
        let agent_config = match AgentConfig::from_env() {
            Ok(config) => config,
            Err(e) => {
                tracing::error!("{e}");
                std::process::exit(1);
            }
        };
        tracing::info!(
            "Creating http client for cluster using base internal url {}",
            agent_config.base_internal_url
        );
        let suffix = create_default_worker_suffix(&hostname);
        (
            Connection::Http(agent_config.build_http_client(&suffix)),
            Some(suffix),
            Some(agent_config),
        )
    } else {
        println!("Connecting to database...");

        let db = crate::db_connect::initial_connection().await?;

        let num_version = sqlx::query_scalar!("SELECT version()").fetch_one(&db).await;

        println!(
            "PostgreSQL version: {} (windmill require PG >= 14)",
            num_version
                .ok()
                .flatten()
                .unwrap_or_else(|| "UNKNOWN".to_string())
        );

        // Load OTEL tracing proxy settings and initialize deno_telemetry if nativets tracing is enabled
        // This must happen before any Deno runtime is created
        #[cfg(all(feature = "private", feature = "enterprise"))]
        {
            reload_otel_tracing_proxy_setting(&Connection::Sql(db.clone())).await;

            #[cfg(feature = "deno_core")]
            if windmill_worker::is_otel_tracing_proxy_enabled_for_lang(&ScriptLang::Nativets).await
            {
                match windmill_worker::load_internal_otel_exporter().await {
                    Ok(()) => {
                        tracing::info!("Internal OTEL exporter initialized for nativets tracing");
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize internal OTEL exporter: {}", e);
                    }
                }
            }
        }

        load_otel(&db).await;

        println!("Database connected");
        (Connection::Sql(db), None, None)
    };

    let environment = if let Ok(environment) = std::env::var("OTEL_ENVIRONMENT") {
        environment
    } else {
        load_base_url(&conn)
            .await
            .unwrap_or_else(|_| "local".to_string())
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split(".")
            .next()
            .unwrap_or_else(|| "local")
            .to_string()
    };

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
                if mode == Mode::Worker {
                    windmill_api::wait_for_db_migrations(&db, killpill_rx.resubscribe()).await?;
                } else {
                    migration_handle =
                        windmill_api::migrate_db(&db, killpill_rx.resubscribe()).await?;
                }
            } else {
                tracing::info!("SKIP_MIGRATION set, skipping db migration...")
            }

            // Sync cached resource types to admins workspace if SYNC_CACHED_RT is set
            if std::env::var("SYNC_CACHED_RT")
                .ok()
                .map(|v| v.to_lowercase() == "true" || v == "1")
                .unwrap_or(false)
            {
                if let Err(e) = sync_cached_resource_types(db).await {
                    tracing::warn!("Failed to sync cached resource types: {:#}", e);
                }
            }
        }
    }

    if killpill_rx.try_recv().is_ok() {
        tracing::info!("Received early killpill, aborting startup");
        return Ok(());
    }

    let worker_mode = num_workers > 0;

    if worker_mode {
        #[cfg(any(target_os = "linux"))]
        if let Err(e) = disable_oom_group() {
            tracing::warn!("failed to disable oom group: {:?}", e);
        }
    }

    let conn = if mode == Mode::Agent {
        conn
    } else {
        // Drop the initial connection pool before creating the main one.
        // With low PostgreSQL max_connections, both pools existing simultaneously
        // can exhaust all available connection slots, causing connect_db to hang.
        drop(conn);

        let db = crate::db_connect::connect_db(
            server_mode,
            indexer_mode,
            worker_mode,
            #[cfg(feature = "private")]
            killpill_rx.resubscribe(),
        )
        .await?;

        // NOTE: Variable/resource cache initialization moved to API server in windmill-api

        Connection::Sql(db)
    };

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
            if is_agent {
                tracing::error!(
                    "Agent worker cannot connect to server. Please check AGENT_TOKEN and BASE_INTERNAL_URL"
                );
                std::process::exit(1);
            }
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

        // native_mode may also be set via DB worker group config (not just env).
        // NATIVE_MODE_RESOLVED is updated by load_worker_config during initial_load.
        if worker_mode
            && !is_native_mode_from_env()
            && NATIVE_MODE_RESOLVED.load(std::sync::atomic::Ordering::Relaxed)
        {
            num_workers = 8;
            tracing::info!("Native mode detected from worker config: forcing NUM_WORKERS=8");
        }

        monitor_db(
            &conn,
            &base_internal_url,
            server_mode,
            worker_mode,
            true,
            killpill_tx.clone(),
            None,
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
                res = windmill_indexer::completed_runs_oss::init_index(&db) => {
                    let res = res?;
                    if let Some(r) = res {
                        reader = Some(r.0);
                        writer = Some(r.1);
                    }
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
                        windmill_indexer::completed_runs_oss::run_indexer(
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
                    res = windmill_indexer::service_logs_oss::init_index(&db, killpill_tx.clone()) => {
                        let res = res?;
                        if let Some(r) = res {
                            reader = Some(r.0);
                            writer = Some(r.1);
                        }
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
                        windmill_indexer::service_logs_oss::run_indexer(
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

        // Resubscribe for OTEL tracing proxy before workers_f captures killpill_rx
        #[cfg(all(feature = "private", feature = "enterprise"))]
        let otel_killpill_rx = killpill_rx.resubscribe();

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
                        None,
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
                    let worker_internal_server_killpill_rx = killpill_rx.resubscribe();
                    init_worker_internal_server_inline_utils(
                        worker_internal_server_killpill_rx,
                        base_internal_url.clone(),
                    )?;
                    let mut workers = vec![];

                    for i in 0..num_workers {
                        let suffix = if i == 0 && first_suffix.is_some() {
                            first_suffix.as_ref().unwrap().clone()
                        } else {
                            create_default_worker_suffix(&hostname)
                        };

                        let worker_conn = WorkerConn {
                            conn: if i == 0 || mode != Mode::Agent {
                                conn.clone()
                            } else {
                                Connection::Http(
                                    agent_config
                                        .as_ref()
                                        .expect("agent_config must be set in agent mode")
                                        .build_http_client(&suffix),
                                )
                            },
                            worker_name: worker_name_with_suffix(
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
                    let db = db.clone();
                    let h = tokio::spawn(async move {
                        // Initialize last_event_id to current max to avoid processing old events on startup
                        let mut last_event_id: i64 =
                            match windmill_common::notify_events::get_latest_event_id(&db).await {
                                Ok(id) => {
                                    tracing::info!(
                                        "Initialized notify event polling with last_event_id: {}",
                                        id
                                    );
                                    id
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Could not get latest event id, starting from 0: {e:#}"
                                    );
                                    0
                                }
                            };
                        let mut last_settings_reload = Instant::now();
                        let mut monitor_iteration: u64 = 0;
                        let rd_shift: u8 = rand::rng().random_range(0..200);
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
                                _ = tokio::time::sleep(Duration::from_secs(*LISTEN_NEW_EVENTS_INTERVAL_SEC)) => {
                                    // Poll for new events from notify_event table
                                    match windmill_common::notify_events::poll_notify_events(&db, last_event_id).await {
                                        Ok(events) => {
                                            for event in events {
                                                if !*windmill_common::QUIET_LOGS {
                                                    tracing::info!("Processing notify event: channel={}, payload={}", event.channel, event.payload);
                                                }
                                                process_notify_event(
                                                    &event.channel,
                                                    &event.payload,
                                                    &db,
                                                    &conn,
                                                    &tx,
                                                    server_mode,
                                                    worker_mode,
                                                    #[cfg(feature = "parquet")]
                                                    disable_s3_store,
                                                ).await;
                                                last_event_id = last_event_id.max(event.id);
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Error polling notify events: {e:#}");
                                        }
                                    }

                                    // Periodic full settings reload
                                    if last_settings_reload.elapsed() > Duration::from_secs(*SETTINGS_RELOAD_PERIOD_SECS) {
                                        tracing::info!("Reloading settings and license key after {}s", Duration::from_secs(*SETTINGS_RELOAD_PERIOD_SECS).as_secs());
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
                                        last_settings_reload = Instant::now();
                                    }

                                    let monitor_start = Instant::now();
                                    let warn_handle = if server_mode {
                                        Some(tokio::spawn(async move {
                                            tokio::time::sleep(Duration::from_secs(5)).await;
                                            tracing::warn!("monitor task has been running for more than 5s");
                                        }))
                                    } else {
                                        None
                                    };
                                    monitor_db(
                                        &conn,
                                        &base_internal_url,
                                        server_mode,
                                        worker_mode,
                                        false,
                                        tx.clone(),
                                        Some(MonitorIteration {
                                            rd_shift,
                                            iter: monitor_iteration,
                                        }),
                                    )
                                    .await;
                                    monitor_iteration += 1;
                                    if let Some(handle) = warn_handle {
                                        handle.abort();
                                    }
                                    let elapsed = monitor_start.elapsed();
                                    if server_mode && elapsed >= Duration::from_secs(5) {
                                        tracing::info!("monitor task finished in {elapsed:.1?}");
                                    }
                                },
                            }
                        }
                    });

                    if let Err(e) = h.await {
                        tracing::error!("Error waiting for monitor handle: {e:#}")
                    }
                }
                ref conn @ Connection::Http(_) => {
                    pub const RELOAD_FREQUENCY: Duration = Duration::from_secs(12 * 60 * 60);
                    let mut last_time_config_reload: Instant = Instant::now();

                    loop {
                        tokio::select! {
                            _ = monitor_killpill_rx.recv() => {
                                tracing::info!("Received killpill, exiting");
                                break;
                            },
                            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                                // Reload config every 12h
                                if last_time_config_reload.elapsed() > RELOAD_FREQUENCY {
                                    last_time_config_reload = Instant::now();
                                    tracing::info!("Reloading config after 12 hours");
                                    initial_load(&conn, tx.clone(), worker_mode, server_mode, #[cfg(feature = "parquet")] disable_s3_store).await;
                                    if let Err(e) = reload_license_key(&conn).await {
                                        tracing::error!("Failed to reload license key on agent: {e:#}");
                                    }
                                    #[cfg(feature = "enterprise")]
                                    ee_oss::verify_license_key().await;
                                }

                                // update min version explicitly.
                                // for sql connection it is the part of monitor_db.
                                // TODO: pass worker names for min keep-alive alerts (for HTTP connection)
                                windmill_common::min_version::update_min_version(conn, true, vec![], false).await;
                            }
                        };
                    }
                }
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

        let otel_tracing_proxy_f = async {
            #[cfg(all(feature = "private", feature = "enterprise"))]
            if worker_mode {
                if let Some(db) = conn.as_sql() {
                    if let Err(e) = windmill_worker::start_jobs_otel_tracing(
                        db.clone(),
                        otel_killpill_rx,
                        num_workers,
                    )
                    .await
                    {
                        tracing::error!("Jobs OTEL tracing error: {}", e);
                    }
                }
            }

            Ok(()) as anyhow::Result<()>
        };

        if server_mode {
            if let Some(db) = conn.as_sql() {
                schedule_stats(&db, &HTTP_CLIENT).await;
            }
        }

        if mcp_mode {
            futures::try_join!(workers_f, server_f)?;
        } else {
            futures::try_join!(
                workers_f,
                monitor_f,
                server_f,
                metrics_f,
                otel_tracing_proxy_f,
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
    std::process::exit(0);
}

/// Process a single notify event from the polling-based event system.
/// This replaces the old PgListener notification handling.
#[allow(unused_variables)]
async fn process_notify_event(
    channel: &str,
    payload: &str,
    db: &Pool<Postgres>,
    conn: &Connection,
    tx: &KillpillSender,
    server_mode: bool,
    worker_mode: bool,
    #[cfg(feature = "parquet")] disable_s3_store: bool,
) {
    match channel {
        "notify_config_change" => {
            if payload == "server" && server_mode {
                tracing::error!(
                    "Server config change detected but server config is obsolete: {}",
                    payload
                );
            } else if worker_mode && payload == format!("worker__{}", *WORKER_GROUP) {
                tracing::info!("Worker config change detected: {}", payload);
                reload_worker_config(db, tx.clone(), true).await;
            } else {
                tracing::debug!("config changed but did not target this server/worker");
            }
        }
        "notify_webhook_change" => {
            tracing::info!(
                "Webhook change detected, invalidating webhook cache: {}",
                payload
            );
            windmill_api::webhook_util::WEBHOOK_CACHE.remove(payload);
        }
        "notify_workspace_envs_change" => {
            tracing::info!(
                "Workspace envs change detected, invalidating workspace envs cache: {}",
                payload
            );
            windmill_common::variables::CUSTOM_ENVS_CACHE.remove(payload);
        }
        "notify_workspace_key_change" => {
            tracing::info!(
                "Workspace key change detected, invalidating workspace key cache: {}",
                payload
            );
            windmill_common::variables::WORKSPACE_CRYPT_CACHE.remove(payload);
        }
        "notify_workspace_premium_change" => {
            tracing::info!(
                "Workspace premium change detected, invalidating workspace premium cache: {}",
                payload
            );
            windmill_common::workspaces::TEAM_PLAN_CACHE.remove(payload);
        }
        "notify_workspace_rate_limit_change" => {
            tracing::info!(
                "Workspace rate limit change detected, invalidating rate limit cache: {}",
                payload
            );
            windmill_common::workspaces::PUBLIC_APP_RATE_LIMIT_CACHE.remove(payload);
        }
        "notify_runnable_version_change" => {
            tracing::info!("Runnable version change detected: {}", payload);
            match payload.split(':').collect::<Vec<&str>>().as_slice() {
                [workspace_id, source_type, path, kind] => {
                    let key = (workspace_id.to_string(), path.to_string());
                    match *source_type {
                        "script" => {
                            windmill_common::DEPLOYED_SCRIPT_HASH_CACHE.remove(&key);
                            if *kind == "preprocessor" {
                                match sqlx::query_scalar::<_, i64>(
                                    "SELECT fv.id
                                    FROM flow f
                                    INNER JOIN flow_version fv ON fv.id = f.versions[array_upper(f.versions, 1)]
                                    WHERE fv.value->'preprocessor_module'->'value'->>'path' = $1 AND f.workspace_id = $2",
                                )
                                .bind(*path)
                                .bind(*workspace_id)
                                .fetch_all(db).await {
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
                            }
                        }
                        "flow" => {
                            let dynamic_input_key =
                                windmill_common::jobs::generate_dynamic_input_key(
                                    workspace_id,
                                    path,
                                );
                            windmill_common::DYNAMIC_INPUT_CACHE.remove(&dynamic_input_key);
                            windmill_common::FLOW_VERSION_CACHE.remove(&key);
                        }
                        _ => {
                            tracing::warn!("Unknown runnable version change payload: {}", payload);
                        }
                    }
                }
                _ => {
                    tracing::warn!("Unknown runnable version change payload: {}", payload);
                }
            }
        }
        #[cfg(feature = "http_trigger")]
        "notify_http_trigger_change" => {
            tracing::info!("HTTP trigger change detected: {}", payload);
            match windmill_api::triggers::http::refresh_routers(db).await {
                Ok((true, _)) => {
                    tracing::info!("Refreshed HTTP routers (trigger change)");
                }
                Ok((false, _)) => {
                    tracing::warn!(
                        "Should have refreshed HTTP routers (trigger change) but did not"
                    );
                }
                Err(err) => {
                    tracing::error!("Error refreshing HTTP routers (trigger change): {err:#}");
                }
            };
        }
        "notify_token_invalidation" => {
            tracing::info!(
                "Token invalidation detected for token: {}...",
                payload.get(..8).unwrap_or(payload)
            );
            windmill_api::auth::invalidate_token_from_cache(payload);
        }
        "notify_global_setting_change" => {
            tracing::info!("Global setting change detected: {}", payload);
            match payload {
                BASE_URL_SETTING => {
                    if let Err(e) = reload_base_url_setting(conn).await {
                        tracing::error!(error = %e, "Could not reload base url setting");
                    }
                }
                OAUTH_SETTING => {
                    if let Err(e) = reload_base_url_setting(conn).await {
                        tracing::error!(error = %e, "Could not reload oauth setting");
                    }
                }
                CUSTOM_TAGS_SETTING => {
                    if let Err(e) = reload_custom_tags_setting(db).await {
                        tracing::error!(error = %e, "Could not reload custom tags setting");
                    }
                }
                LICENSE_KEY_SETTING => {
                    if let Err(e) = reload_license_key(&db.into()).await {
                        tracing::error!("Failed to reload license key: {e:#}");
                    }
                }
                DEFAULT_TAGS_PER_WORKSPACE_SETTING => {
                    if let Err(e) = load_tag_per_workspace_enabled(db).await {
                        tracing::error!("Error loading default tag per workspace: {e:#}");
                    }
                }
                DEFAULT_TAGS_WORKSPACES_SETTING => {
                    if let Err(e) = load_tag_per_workspace_workspaces(db).await {
                        tracing::error!(
                            "Error loading default tag per workspace workspaces: {e:#}"
                        );
                    }
                }
                SMTP_SETTING => {
                    reload_smtp_config(db).await;
                }
                TEAMS_SETTING => {
                    tracing::info!("Teams setting changed.");
                }
                INDEXER_SETTING => {
                    reload_indexer_config(db).await;
                }
                TIMEOUT_WAIT_RESULT_SETTING => reload_timeout_wait_result_setting(conn).await,
                RETENTION_PERIOD_SECS_SETTING => reload_retention_period_setting(conn).await,
                MONITOR_LOGS_ON_OBJECT_STORE_SETTING => {
                    reload_delete_logs_periodically_setting(conn).await
                }
                JOB_DEFAULT_TIMEOUT_SECS_SETTING => reload_job_default_timeout_setting(conn).await,
                JOB_ISOLATION_SETTING => reload_job_isolation_setting(conn).await,
                #[cfg(feature = "parquet")]
                OBJECT_STORE_CONFIG_SETTING => {
                    if !disable_s3_store {
                        reload_object_store_setting(db).await;
                    }
                }
                SCIM_TOKEN_SETTING => reload_scim_token_setting(conn).await,
                EXTRA_PIP_INDEX_URL_SETTING => reload_extra_pip_index_url_setting(conn).await,
                PIP_INDEX_URL_SETTING => reload_pip_index_url_setting(conn).await,
                UV_INDEX_STRATEGY_SETTING => reload_uv_index_strategy_setting(conn).await,
                INSTANCE_PYTHON_VERSION_SETTING => {
                    reload_instance_python_version_setting(conn).await
                }
                NPM_CONFIG_REGISTRY_SETTING => reload_npm_config_registry_setting(conn).await,
                BUNFIG_INSTALL_SCOPES_SETTING => reload_bunfig_install_scopes_setting(conn).await,
                NUGET_CONFIG_SETTING => reload_nuget_config_setting(conn).await,
                POWERSHELL_REPO_URL_SETTING => reload_powershell_repo_url_setting(conn).await,
                POWERSHELL_REPO_PAT_SETTING => reload_powershell_repo_pat_setting(conn).await,
                MAVEN_REPOS_SETTING => reload_maven_repos_setting(conn).await,
                MAVEN_SETTINGS_XML_SETTING => reload_maven_settings_xml_setting(conn).await,
                NO_DEFAULT_MAVEN_SETTING => reload_no_default_maven_setting(conn).await,
                RUBY_REPOS_SETTING => reload_ruby_repos_setting(conn).await,
                HUB_API_SECRET_SETTING => reload_hub_api_secret_setting(conn).await,
                KEEP_JOB_DIR_SETTING => {
                    load_keep_job_dir(conn).await;
                }
                OTEL_TRACING_PROXY_SETTING => {
                    reload_otel_tracing_proxy_setting(conn).await;
                    if worker_mode {
                        tracing::info!("OTEL tracing proxy setting changed, restarting worker");
                        send_delayed_killpill(tx, 4, "OTEL tracing proxy setting change").await;
                    }
                }
                REQUIRE_PREEXISTING_USER_FOR_OAUTH_SETTING => {
                    load_require_preexisting_user(db).await;
                }
                EXPOSE_METRICS_SETTING => {
                    tracing::info!("Metrics setting changed, restarting");
                    send_delayed_killpill(tx, 40, "metrics setting change").await;
                }
                EMAIL_DOMAIN_SETTING => {
                    tracing::info!("Email domain setting changed");
                    if server_mode {
                        send_delayed_killpill(tx, 4, "email domain setting change").await;
                    }
                }
                EXPOSE_DEBUG_METRICS_SETTING => {
                    if let Err(e) = load_metrics_debug_enabled(conn).await {
                        tracing::error!(error = %e, "Could not reload debug metrics setting");
                    }
                }
                APP_WORKSPACED_ROUTE_SETTING => {
                    if let Err(e) = reload_app_workspaced_route_setting(db).await {
                        tracing::error!(error = %e, "Could not reload app workspaced route setting");
                    }
                }
                OTEL_SETTING => {
                    tracing::info!("OTEL setting changed, restarting");
                    send_delayed_killpill(tx, 4, "OTEL setting change").await;
                }
                REQUEST_SIZE_LIMIT_SETTING => {
                    if server_mode {
                        tracing::info!("Request limit size change detected, killing server expecting to be restarted");
                        send_delayed_killpill(tx, 4, "request size limit change").await;
                    }
                }
                SAML_METADATA_SETTING => {
                    tracing::info!(
                        "SAML metadata change detected, killing server expecting to be restarted"
                    );
                    send_delayed_killpill(tx, 0, "SAML metadata change").await;
                }
                HUB_BASE_URL_SETTING => {
                    if let Err(e) = reload_hub_base_url_setting(conn, server_mode).await {
                        tracing::error!(error = %e, "Could not reload hub base url setting");
                    }
                }
                CRITICAL_ERROR_CHANNELS_SETTING => {
                    if let Err(e) = reload_critical_error_channels_setting(db).await {
                        tracing::error!(error = %e, "Could not reload critical error emails setting");
                    }
                }
                CRITICAL_ALERTS_ON_DB_OVERSIZE_SETTING => {
                    if let Err(e) = reload_critical_alerts_on_db_oversize(db).await {
                        tracing::error!(error = %e, "Could not reload critical alerts on db oversize setting");
                    }
                }
                JWT_SECRET_SETTING => {
                    if let Err(e) = reload_jwt_secret_setting(db).await {
                        tracing::error!(error = %e, "Could not reload jwt secret setting");
                    }
                }
                CRITICAL_ALERT_MUTE_UI_SETTING => {
                    tracing::info!("Critical alert UI setting changed");
                    if let Err(e) = reload_critical_alert_mute_ui_setting(conn).await {
                        tracing::error!(error = %e, "Could not reload critical alert UI setting");
                    }
                }
                "workspace_telemetry_enabled" => {
                    // Read the new value from the database and log it
                    let enabled = sqlx::query_scalar!(
                        "SELECT value FROM global_settings WHERE name = 'workspace_telemetry_enabled'"
                    )
                    .fetch_optional(db)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                    tracing::info!("Workspace telemetry setting changed: enabled={}", enabled);
                }
                _ => {
                    tracing::info!("Unrecognized Global Setting Change Payload: {:?}", payload);
                }
            }
        }
        _ => {
            tracing::warn!("Unknown notification channel: {}", channel);
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
        BUN_BUNDLE_CACHE_DIR,
        GO_CACHE_DIR,
        GO_BIN_CACHE_DIR,
        RUST_CACHE_DIR,
        CSHARP_CACHE_DIR,
        NU_CACHE_DIR,
        HUB_CACHE_DIR,
        POWERSHELL_CACHE_DIR,
        JAVA_CACHE_DIR,
        RUBY_CACHE_DIR,
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
