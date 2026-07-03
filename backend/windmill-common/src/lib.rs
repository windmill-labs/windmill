/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    hash::{Hash, Hasher},
    net::SocketAddr,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc,
    },
};

use tokio::{spawn, sync::broadcast};

use ee_oss::CriticalErrorChannel;
use error::Error;
use scripts::ScriptLang;
use sqlx::{Acquire, Postgres};

pub mod agent_workers;
pub mod apps;
pub mod assets;
pub mod audit;
pub mod auth;
#[cfg(feature = "benchmark")]
pub mod bench;
pub mod cache;
pub mod client;
pub mod db;
#[cfg(all(feature = "enterprise", feature = "private"))]
mod db_entra_ee;
#[cfg(all(feature = "enterprise", feature = "private"))]
mod db_iam_ee;
pub mod db_params;
#[cfg(feature = "private")]
pub mod deployment_requests_ee;
pub mod deployment_requests_oss;
#[cfg(feature = "private")]
pub mod ee;
pub mod ee_oss;
#[cfg(feature = "private")]
pub mod email_ee;
pub mod email_oss;
pub mod error;
pub mod external_ip;
pub mod flow_conversations;
pub mod flow_status;
pub mod flows;
pub mod folders;
pub mod global_settings;
pub mod indexer;
pub mod instance_config;
pub mod job_metrics;
pub mod log_context;
pub mod materialization;
pub mod min_version;
pub mod notify_events;
pub mod runtime_assets;
pub mod workspace_dependencies;

#[cfg(feature = "private")]
pub mod git_sync_ee;
pub mod git_sync_oss;
pub mod jobs;
pub mod jwt;
pub mod login_rate_limit;
pub mod more_serde;
pub mod oauth2;
#[cfg(all(feature = "enterprise", feature = "openidconnect", feature = "private"))]
pub mod oidc_ee;
#[cfg(all(feature = "enterprise", feature = "openidconnect"))]
pub mod oidc_oss;
#[cfg(feature = "private")]
pub mod otel_ee;
pub mod otel_oss;
#[cfg(feature = "private")]
pub mod partition_ee;
pub mod partition_oss;
#[cfg(feature = "private")]
pub use partition_ee as partition;
#[cfg(not(feature = "private"))]
pub use partition_oss as partition;
#[cfg(feature = "private")]
pub mod pipeline_advanced_ee;
pub mod pipeline_advanced_oss;
#[cfg(feature = "private")]
pub use pipeline_advanced_ee as pipeline_advanced;
#[cfg(not(feature = "private"))]
pub use pipeline_advanced_oss as pipeline_advanced;
pub mod query_builders;
pub mod queue;
pub mod result_stream;
pub mod runnable_settings;
pub mod schedule;
pub mod schema;
pub mod scripts;
pub mod secret_backend;
pub mod sensitive_log_masks;
pub mod server;
pub mod ssrf;
#[cfg(feature = "private")]
pub mod stats_ee;
pub mod stats_oss;
pub mod stream;
#[cfg(feature = "private")]
pub mod teams_ee;
pub mod teams_oss;
pub mod tracing_init;
pub mod trashbin;
pub mod triggers;
pub mod user_drafts;
pub mod usernames;
pub mod users;
pub mod utils;
pub mod variables;
pub mod wac;
pub mod webhook;
pub mod worker;
pub mod worker_group_job_stats;
pub mod workspaces;

pub const DEFAULT_MAX_CONNECTIONS_SERVER: u32 = 50;
pub const DEFAULT_MAX_CONNECTIONS_WORKER: u32 = 5;
pub const DEFAULT_MAX_CONNECTIONS_INDEXER: u32 = 5;

pub const DEFAULT_HUB_BASE_URL: &str = "https://hub.windmill.dev";
pub const PRIVATE_HUB_MIN_VERSION: i32 = 10_000_000;
pub const SERVICE_LOG_RETENTION_SECS: i64 = 60 * 60 * 24 * 14; // 2 weeks retention period for logs
pub const WM_DEPLOYERS_GROUP: &str = "wm_deployers";

/// Canonical form of a base URL, used as one of the inputs to the offline-license
/// instance hash (`compute_instance_hash`).
///
/// Rules: lowercase scheme and host, drop default ports (80/443), strip path/query/fragment,
/// strip trailing slash. If URL parsing fails, falls back to a best-effort lowercase +
/// trailing-slash strip so two semantically-equivalent inputs still produce the same
/// canonical form.
pub fn canonical_base_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    match url::Url::parse(trimmed) {
        Ok(u) => {
            let scheme = u.scheme().to_ascii_lowercase();
            let host = u
                .host_str()
                .map(|h| h.to_ascii_lowercase())
                .unwrap_or_default();
            let port = match (u.port(), scheme.as_str()) {
                (Some(80), "http") | (Some(443), "https") => String::new(),
                (Some(p), _) => format!(":{p}"),
                (None, _) => String::new(),
            };
            format!("{scheme}://{host}{port}")
        }
        Err(_) => trimmed.trim_end_matches('/').to_ascii_lowercase(),
    }
}

/// Checks if the user is allowed to preserve on_behalf_of values (admin or deployer).
pub fn can_preserve_on_behalf_of(authed: &impl db::Authable) -> bool {
    authed.is_admin() || authed.groups().iter().any(|g| g == &WM_DEPLOYERS_GROUP)
}

/// Checks if on-behalf-of preservation actually happened (the target user differs from the acting user).
/// Returns Some(target_identifier) if preservation occurred, None otherwise.
pub fn check_on_behalf_of_preservation(
    on_behalf_of_identifier: Option<&str>,
    preserve: bool,
    authed: &impl db::Authable,
    authed_identifier: &str,
) -> Option<String> {
    if preserve && can_preserve_on_behalf_of(authed) {
        if let Some(id) = on_behalf_of_identifier {
            if id != authed_identifier {
                return Some(id.to_string());
            }
        }
    }
    None
}

/// Determines the on_behalf_of_email value to use when creating/updating a flow or script.
/// - If `on_behalf_of_email` is None, returns None
/// - If `preserve` is true and the user is admin or in the deployers group, returns the original value
/// - Otherwise, returns the authenticated user's email
pub fn resolve_on_behalf_of_email<'a>(
    on_behalf_of_email: Option<&'a str>,
    preserve: bool,
    authed: &'a impl db::Authable,
) -> Option<&'a str> {
    if on_behalf_of_email.is_some() {
        if preserve && can_preserve_on_behalf_of(authed) {
            on_behalf_of_email
        } else {
            Some(authed.email())
        }
    } else {
        None
    }
}

#[macro_export]
macro_rules! add_time {
    ($bench:expr, $name:expr) => {
        #[cfg(feature = "benchmark")]
        {
            $bench.add_timing($name);
            // println!("{}: {:?}", $z, $y.elapsed());
        }
    };
}

lazy_static::lazy_static! {
    pub static ref METRICS_PORT: u16 = std::env::var("METRICS_PORT")
    .ok()
    .and_then(|s| s.parse::<u16>().ok())
    .unwrap_or(8001);

    pub static ref METRICS_ADDR: SocketAddr = std::env::var("METRICS_ADDR")
    .ok()
    .map(|s| {
        s.parse::<bool>()
            .map(|b| b.then(|| SocketAddr::from(([0, 0, 0, 0], *METRICS_PORT))))
            .or_else(|_| s.parse::<SocketAddr>().map(Some))
    })
    .transpose().ok()
    .flatten()
    .flatten()
    .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], *METRICS_PORT)));

    pub static ref METRICS_ENABLED: AtomicBool = AtomicBool::new(std::env::var("METRICS_PORT").is_ok() || std::env::var("METRICS_ADDR").is_ok());

    pub static ref OTEL_METRICS_ENABLED: AtomicBool = AtomicBool::new(std::env::var("OTEL_METRICS").is_ok());
    pub static ref OTEL_TRACING_ENABLED: AtomicBool = AtomicBool::new(std::env::var("OTEL_TRACING").is_ok());
    pub static ref OTEL_LOGS_ENABLED: AtomicBool = AtomicBool::new(std::env::var("OTEL_LOGS").is_ok());

    pub static ref METRICS_DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

    pub static ref CRITICAL_ALERT_MUTE_UI_ENABLED: AtomicBool = AtomicBool::new(false);
    pub static ref CRITICAL_ALERTS_ON_TOKEN_EXPIRY: AtomicBool = AtomicBool::new(false);

    pub static ref BASE_URL: arc_swap::ArcSwap<String> = arc_swap::ArcSwap::from_pointee("".to_string());
    pub static ref IS_READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    pub static ref HUB_BASE_URL: arc_swap::ArcSwap<String> = arc_swap::ArcSwap::from_pointee(DEFAULT_HUB_BASE_URL.to_string());


    pub static ref CRITICAL_ERROR_CHANNELS: arc_swap::ArcSwap<Vec<CriticalErrorChannel>> = arc_swap::ArcSwap::from_pointee(vec![]);
    pub static ref CRITICAL_ALERTS_ON_DB_OVERSIZE: arc_swap::ArcSwap<Option<f32>> = arc_swap::ArcSwap::from_pointee(None);

    pub static ref JOB_RETENTION_SECS: AtomicI64 = AtomicI64::new(0);
    pub static ref AUDIT_LOG_RETENTION_DAYS: AtomicI64 = AtomicI64::new(0);

    pub static ref MONITOR_LOGS_ON_OBJECT_STORE: AtomicBool = AtomicBool::new(false);

    pub static ref STORE_AUDIT_LOGS_S3: AtomicBool = AtomicBool::new(false);

    pub static ref INSTANCE_NAME: String = rd_string(5);

    pub static ref DEPLOYED_SCRIPT_HASH_CACHE: Cache<(String, String), ExpiringLatestVersionId> = Cache::new(1000);
    // Latest non-archived version per (workspace, path) for bundle cache keying —
    // looser predicate than DEPLOYED_SCRIPT_HASH_CACHE (no lock requirement), so
    // the two must not share entries. See get_latest_script_hash_for_import_cached.
    pub static ref IMPORTED_SCRIPT_HASH_CACHE: Cache<(String, String), ExpiringLatestVersionId> = Cache::new(1000);
    pub static ref FLOW_VERSION_CACHE: Cache<(String, String), ExpiringLatestVersionId> = Cache::new(1000);
    pub static ref DYNAMIC_INPUT_CACHE: Cache<String, Arc<jobs::DynamicInput>> = Cache::new(1000);
    pub static ref DEPLOYED_SCRIPT_INFO_CACHE: Cache<(String, i64), ScriptHashInfo<ScriptRunnableSettingsHandle>> = Cache::new(1000);
    pub static ref FLOW_INFO_CACHE: Cache<(String, i64), FlowVersionInfo> = Cache::new(1000);

    pub static ref QUIET_LOGS: bool = std::env::var("QUIET_LOGS").map(|s| s.parse::<bool>().unwrap_or(false)).unwrap_or(false);

    /// Snapshot of the standard outbound-proxy env vars, read once at startup.
    /// Lowercase (`no_proxy`, `http_proxy`, `https_proxy`) is preferred to match
    /// the convention used by libcurl / reqwest; uppercase is the fallback.
    pub static ref NO_PROXY: Option<String> = std::env::var("no_proxy").ok().or_else(|| std::env::var("NO_PROXY").ok());
    pub static ref HTTP_PROXY: Option<String> = std::env::var("http_proxy").ok().or_else(|| std::env::var("HTTP_PROXY").ok());
    pub static ref HTTPS_PROXY: Option<String> = std::env::var("https_proxy").ok().or_else(|| std::env::var("HTTPS_PROXY").ok());
}

const LATEST_VERSION_ID_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(60);

/// Test hook: disables the process-global deployed-script hash/info caches so
/// every resolution reads the current DB. Integration tests use `#[sqlx::test]`
/// isolated DBs that share one workspace id and reuse script paths, so a cache
/// keyed by `(workspace, path)`/`(workspace, hash)` resolves a path to a hash
/// that lives in a *different* test's DB — and when the info cache misses for
/// that foreign hash the lookup 404s in the wrong DB. Always `false` in
/// production (the caches are TTL/LRU-bounded against real deploys).
pub static DEPLOYED_SCRIPT_CACHE_DISABLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

pub async fn shutdown_signal(
    tx: KillpillSender,
    mut rx: tokio::sync::broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    async fn terminate() -> std::io::Result<()> {
        use tokio::signal::unix::SignalKind;
        tokio::signal::unix::signal(SignalKind::terminate())?
            .recv()
            .await;
        Ok(())
    }

    // Defined for the whole non-unix scope (not just windows) so it can be a
    // plain `tokio::select!` branch: that macro does not accept `#[cfg(...)]`
    // attributes on individual branches. On non-windows non-unix targets the
    // future never resolves, so the branch is effectively inert there.
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    async fn ctrl_break() -> std::io::Result<()> {
        #[cfg(windows)]
        {
            tokio::signal::windows::ctrl_break()?.recv().await;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            std::future::pending::<()>().await;
            Ok(())
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    tokio::select! {
        _ = terminate() => {
            tracing::info!("shutdown monitor received terminate");
        },
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("shutdown monitor received ctrl-c");
        },
        _ = rx.recv() => {
            tracing::info!("shutdown monitor received killpill");
        },
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("shutdown monitor received ctrl-c");
        },
        _ = ctrl_break() => {
            tracing::info!("shutdown monitor received ctrl-break");
        },
        _ = rx.recv() => {
            tracing::info!("shutdown monitor received killpill");
        },
    }

    spawn(async move {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        tokio::select! {
            _ = terminate() => {
                tracing::error!("2nd shutdown monitor received terminate");
            },
            _ = tokio::signal::ctrl_c() => {
                tracing::error!("2nd shutdown monitor received ctrl-c");
            },
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::error!("2nd shutdown monitor received ctrl-c")
            },
            _ = ctrl_break() => {
                tracing::error!("2nd shutdown monitor received ctrl-break")
            },
        }

        tracing::info!("Second terminate signal received, forcefully exiting");

        let handle = tokio::runtime::Handle::current();
        let metrics = handle.metrics();
        tracing::info!(
            "Alive tasks: {}, global queue depth: {}",
            metrics.num_alive_tasks(),
            metrics.global_queue_depth()
        );

        std::process::exit(1);
    });

    tracing::info!("signal received, starting graceful shutdown");
    let _ = tx.send();

    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(24 * 7 * 60 * 60)).await;
        tracing::info!("Forcefully exiting after 7 days");
        std::process::exit(1);
    });

    Ok(())
}

use utils::rd_string;

#[cfg(feature = "prometheus")]
pub async fn serve_metrics(
    addr: SocketAddr,
    mut rx: tokio::sync::broadcast::Receiver<()>,
    ready_worker_endpoint: bool,
    metrics_endpoint: bool,
) -> anyhow::Result<()> {
    if !metrics_endpoint && !ready_worker_endpoint {
        return Ok(());
    }
    use axum::{
        routing::{get, post},
        Router,
    };
    use hyper::StatusCode;
    let router = Router::new();

    let router = if metrics_endpoint {
        router
            .route("/metrics", get(metrics))
            .route("/reset", post(reset))
    } else {
        router
    };

    let router = if ready_worker_endpoint {
        router.route(
            "/ready",
            get(|| async {
                if IS_READY.load(std::sync::atomic::Ordering::Relaxed) {
                    (StatusCode::OK, "ready")
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, "not ready")
                }
            }),
        )
    } else {
        router
    };

    tokio::spawn(async move {
        tracing::info!("Serving metrics at: {addr}");
        let listener = tokio::net::TcpListener::bind(addr).await;
        if let Err(e) = listener {
            tracing::error!("Error binding to metrics address: {}", e);
            return;
        }
        if let Err(e) = axum::serve(listener.unwrap(), router.into_make_service())
            .with_graceful_shutdown(async move {
                rx.recv().await.ok();
                tracing::info!("Graceful shutdown of metrics");
            })
            .await
        {
            tracing::error!("Error serving metrics: {}", e);
        }
    })
    .await?;
    Ok(())
}

#[cfg(feature = "prometheus")]
async fn metrics() -> Result<String, Error> {
    let metric_families = prometheus::gather();
    Ok(prometheus::TextEncoder::new()
        .encode_to_string(&metric_families)
        .map_err(anyhow::Error::from)?)
}

#[cfg(feature = "prometheus")]
async fn reset() -> () {
    todo!()
}

/// Parse the canonical Python `logging.basicConfig()` line format
/// `LEVELNAME:logger.name:message` and return the corresponding tracing level.
///
/// Returns `None` for lines that don't match — tracebacks, raw `print` to
/// stderr, third-party tools with custom formats — leaving those to the caller's
/// default (typically `tracing::error!`).
pub fn classify_python_logging_line(line: &str) -> Option<tracing::Level> {
    let (level, rest) = line.split_once(':')?;
    if !rest.contains(':') {
        return None;
    }
    match level {
        "CRITICAL" | "ERROR" => Some(tracing::Level::ERROR),
        "WARNING" => Some(tracing::Level::WARN),
        "INFO" => Some(tracing::Level::INFO),
        "DEBUG" => Some(tracing::Level::DEBUG),
        _ => None,
    }
}

#[cfg(test)]
mod classify_python_logging_line_tests {
    use super::classify_python_logging_line;
    use tracing::Level;

    #[test]
    fn matches_python_levels() {
        assert_eq!(
            classify_python_logging_line("WARNING:dlt.normalize:msg"),
            Some(Level::WARN)
        );
        assert_eq!(
            classify_python_logging_line("INFO:app:hello"),
            Some(Level::INFO)
        );
        assert_eq!(
            classify_python_logging_line("ERROR:a:b"),
            Some(Level::ERROR)
        );
        assert_eq!(
            classify_python_logging_line("CRITICAL:a:b"),
            Some(Level::ERROR)
        );
        assert_eq!(
            classify_python_logging_line("DEBUG:a:b"),
            Some(Level::DEBUG)
        );
    }

    #[test]
    fn rejects_non_python_format() {
        assert_eq!(
            classify_python_logging_line("Traceback (most recent call last):"),
            None
        );
        assert_eq!(
            classify_python_logging_line("WARNING:no-second-colon"),
            None
        );
        assert_eq!(classify_python_logging_line("warning:lowercase:msg"), None);
        assert_eq!(classify_python_logging_line("plain stderr text"), None);
        assert_eq!(classify_python_logging_line(""), None);
    }
}

#[cfg(test)]
mod validate_dbname_tests {
    use super::validate_dbname;

    #[test]
    fn accepts_letters_digits_underscores_and_hyphens() {
        assert!(validate_dbname("mydb").is_ok());
        assert!(validate_dbname("my_db").is_ok());
        assert!(validate_dbname("my-database").is_ok());
        assert!(validate_dbname("My-Db_1").is_ok());
    }

    #[test]
    fn rejects_invalid_names() {
        // Must start with a letter (hyphen/digit/underscore leads are rejected).
        assert!(validate_dbname("-db").is_err());
        assert!(validate_dbname("1db").is_err());
        assert!(validate_dbname("_db").is_err());
        // No other special characters or whitespace.
        assert!(validate_dbname("my db").is_err());
        assert!(validate_dbname("my;db").is_err());
        assert!(validate_dbname("").is_err());
    }
}

#[cfg(test)]
mod pg_tls_tests {
    use super::PgDatabase;

    // A syntactically valid (self-signed) certificate, used only to exercise the
    // "root certificate supplied" branch — its contents are never validated here.
    const VALID_PEM: &str = "-----BEGIN CERTIFICATE-----\n\
MIIDETCCAfmgAwIBAgIUX/yHsMoWBljFzJr5Xh7V2I6ykMEwDQYJKoZIhvcNAQEL\n\
BQAwGDEWMBQGA1UEAwwNd2luZG1pbGwtdGVzdDAeFw0yNjA2MjkwOTUwNTlaFw0z\n\
NjA2MjYwOTUwNTlaMBgxFjAUBgNVBAMMDXdpbmRtaWxsLXRlc3QwggEiMA0GCSqG\n\
SIb3DQEBAQUAA4IBDwAwggEKAoIBAQCvF2hMw8adQGG6EnDk8GsOIoHT+kLN1W0F\n\
yYFwH1wGVmzVP1YNfUts8aQfMtl/ZjW7SQlvKeK+18id4fVNYvZpbFhj66IsKMOU\n\
MnJHcC6X/IAdhANyhM1fcrS6YupanAKOhLPk4HYRD5tGI4Y1vzTnQKGffIZ0bof7\n\
3GtCiJLv8wrJKszeoKPtdFazdW+CYePbFq3Owc7HMo8CwA7A5TsgcowELhCfYwZv\n\
Pn/9v+NDHQO0jJclH7qK221RkbqZGD+nPJ4rUm7oRi0vfApBQZ0FFJZjiki/Kg2+\n\
RACb6Ud/LOeRBerKQHbN8KeYnGafCaIC4s/XytVwxAz+kgK1qyl7AgMBAAGjUzBR\n\
MB0GA1UdDgQWBBRo2Jby4SZlrwMNbhA4bswZcBNRyjAfBgNVHSMEGDAWgBRo2Jby\n\
4SZlrwMNbhA4bswZcBNRyjAPBgNVHRMBAf8EBTADAQH/MA0GCSqGSIb3DQEBCwUA\n\
A4IBAQBlED+FQW3GB3Wa1NdVN252vihuFNnbq81yvhf4T7dfAxwkxI9jiM+ZWCw2\n\
g59FbLupj8Rwun5gE2H/9M8ZunISdlwaMH5nyDJlbRjttPfY1cEoyGEY+UXIslfg\n\
BoiI5rOtz9R2qurxEic1VtEVfXhEuWwCG86vCBDdHrL/qqqUJEx/P8qyC7uVc8XC\n\
uclnJVL7x1ax0jTmEPur9K+DQn2ws01mzpq2QwSunibpDL5D5xM1oYekv0tQFEkT\n\
ta9ELulniZau8zUAtwqwecxodzl+KO8NYj0a9PGgAM64dMqkRtRA8P4UP350Nag3\n\
+hOq1qpWD7yPVyycx/KCilICOKVf\n\
-----END CERTIFICATE-----\n";

    fn pg(sslmode: Option<&str>, root_cert: Option<&str>) -> PgDatabase {
        PgDatabase {
            host: "db.example.com".to_string(),
            user: Some("u".to_string()),
            password: Some("p".to_string()),
            port: Some(5432),
            sslmode: sslmode.map(|s| s.to_string()),
            dbname: "mydb".to_string(),
            root_certificate_pem: root_cert.map(|s| s.to_string()),
            accept_invalid_certs: None,
            use_iam_auth: None,
            region: None,
        }
    }

    /// Whether the connector enforces certificate verification for the given config.
    fn verifies(
        sslmode: Option<&str>,
        root_cert: Option<&str>,
        accept_invalid_certs: Option<bool>,
    ) -> bool {
        let mut builder = native_tls::TlsConnector::builder();
        PgDatabase::configure_pg_tls_verification(
            &mut builder,
            sslmode,
            root_cert,
            accept_invalid_certs,
        )
        .unwrap()
    }

    #[test]
    fn verify_modes_enforce_verification_when_explicitly_requested() {
        // accept_invalid_certs=Some(false) is what newly created resources carry: it
        // verifies even with no custom cert (against the OS trust store).
        assert!(verifies(Some("verify-full"), None, Some(false)));
        assert!(verifies(Some("verify-ca"), None, Some(false)));
        assert!(verifies(Some("verify-full"), Some(""), Some(false)));
        assert!(verifies(Some("verify-full"), Some(VALID_PEM), Some(false)));
        assert!(verifies(Some("verify-ca"), Some(VALID_PEM), Some(false)));
    }

    #[test]
    fn verify_modes_unset_fall_back_to_legacy_behavior() {
        // Unset (None): verify iff a root cert is present — preserves the behavior of
        // resources that predate the flag (incl. git-synced), so upgrades don't break.
        assert!(!verifies(Some("verify-full"), None, None));
        assert!(!verifies(Some("verify-ca"), None, None));
        assert!(!verifies(Some("verify-full"), Some(""), None));
        assert!(verifies(Some("verify-full"), Some(VALID_PEM), None));
        assert!(verifies(Some("verify-ca"), Some(VALID_PEM), None));
    }

    #[test]
    fn accept_invalid_certs_true_disables_verification_for_verify_modes() {
        assert!(!verifies(Some("verify-full"), Some(VALID_PEM), Some(true)));
        assert!(!verifies(Some("verify-ca"), None, Some(true)));
    }

    #[test]
    fn accept_invalid_certs_is_ignored_outside_verify_modes() {
        // require never consults the flag: it verifies iff a cert is present, and
        // encrypts-without-verifying otherwise, regardless of accept_invalid_certs.
        assert!(!verifies(Some("require"), None, Some(false)));
        assert!(!verifies(Some("require"), None, Some(true)));
        assert!(!verifies(None, None, Some(true)));
        assert!(verifies(Some("require"), Some(VALID_PEM), Some(true)));
        assert!(verifies(Some("require"), Some(VALID_PEM), None));
    }

    #[test]
    fn invalid_pem_is_rejected() {
        let mut builder = native_tls::TlsConnector::builder();
        let err = PgDatabase::configure_pg_tls_verification(
            &mut builder,
            Some("verify-full"),
            Some("not a certificate"),
            Some(false),
        );
        assert!(err.is_err());
    }

    #[test]
    fn to_uri_collapses_verify_modes_for_tokio_postgres() {
        // to_uri() feeds tokio-postgres, which only parses disable/prefer/require;
        // verify-* therefore map to require there (verification is connector-driven).
        for mode in ["require", "verify-ca", "verify-full"] {
            assert!(
                pg(Some(mode), None).to_uri().contains("sslmode=require"),
                "{mode} should map to sslmode=require in to_uri()"
            );
        }
        assert!(pg(Some("disable"), None)
            .to_uri()
            .contains("sslmode=disable"));
        assert!(pg(Some("allow"), None).to_uri().contains("sslmode=prefer"));
        assert!(pg(None, None).to_uri().contains("sslmode=prefer"));
    }
}

#[derive(Serialize, Debug)]
pub struct PrepareQueryColumnInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
}

#[derive(Serialize, Debug)]
pub struct PrepareQueryResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<PrepareQueryColumnInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct PgDatabase {
    pub host: String,
    pub user: Option<String>,
    pub password: Option<String>,
    pub port: Option<u16>,
    pub sslmode: Option<String>,
    pub dbname: String,
    pub root_certificate_pem: Option<String>,
    /// Only meaningful for sslmode verify-ca/verify-full. `Some(true)` accepts any
    /// server certificate (no chain or hostname check); `Some(false)` enforces
    /// verification. `None` falls back to legacy behavior — verify only when a root
    /// certificate is present — so resources that predate this flag (including
    /// git-synced ones, whose source never sets it) keep working unchanged.
    pub accept_invalid_certs: Option<bool>,
    pub use_iam_auth: Option<bool>,
    pub region: Option<String>,
}

// Wrapper enum to hold either Tls or NoTls connection
pub enum TokioPgConnection {
    Tls(
        tokio_postgres::Connection<
            tokio_postgres::Socket,
            postgres_native_tls::TlsStream<tokio_postgres::Socket>,
        >,
    ),
    NoTls(tokio_postgres::Connection<tokio_postgres::Socket, tokio_postgres::tls::NoTlsStream>),
}

impl Future for TokioPgConnection {
    type Output = Result<(), tokio_postgres::Error>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        // SAFETY: We're simply projecting the Pin from the outer enum to the inner connection field.
        // The inner connection is never moved out, so this is safe.
        unsafe {
            match self.get_unchecked_mut() {
                TokioPgConnection::Tls(conn) => std::pin::Pin::new_unchecked(conn).poll(cx),
                TokioPgConnection::NoTls(conn) => std::pin::Pin::new_unchecked(conn).poll(cx),
            }
        }
    }
}

impl PgDatabase {
    pub fn to_uri(&self) -> String {
        let sslmode = match self.sslmode.as_deref() {
            Some("allow") => "prefer".to_string(),
            Some("require") | Some("verify-ca") | Some("verify-full") => "require".to_string(),
            Some(s) => s.to_string(),
            None => "prefer".to_string(),
        };
        // Encode host/dbname too: an unencoded '@', '/', '?' or '&' would otherwise
        // reshape the parsed URI (inject libpq params / alter host). Bracketed IPv6
        // literals ([::1]) are passed through unencoded — percent-encoding their
        // '['/']'/':' would stop them parsing as a host.
        let host = if self.host.starts_with('[') && self.host.ends_with(']') {
            self.host.clone()
        } else {
            urlencoding::encode(&self.host).into_owned()
        };
        format!(
            "postgres://{user}:{password}@{host}:{port}/{dbname}?sslmode={sslmode}",
            user = urlencoding::encode(&self.user.as_deref().unwrap_or("postgres")),
            password = urlencoding::encode(&self.password.as_deref().unwrap_or("")),
            host = host,
            port = self.port.unwrap_or(5432),
            dbname = urlencoding::encode(&self.dbname),
            sslmode = sslmode
        )
    }

    pub async fn connect(
        &self,
        main_db: Option<&DB>,
    ) -> Result<(tokio_postgres::Client, TokioPgConnection), error::Error> {
        match self.connect_inner().await {
            Ok(result) => Ok(result),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("password authentication failed for user")
                    && err_str.contains("custom_instance_user")
                {
                    if let Some(db) = main_db {
                        tracing::warn!(
                            "custom_instance_user password auth failed, refreshing and retrying..."
                        );
                        crate::utils::refresh_custom_instance_user_pwd(db).await?;
                        let new_pwd = crate::utils::get_custom_pg_instance_password(db).await?;
                        let mut retried = self.clone();
                        retried.password = Some(new_pwd);
                        return retried.connect_inner().await;
                    }
                }
                Err(e)
            }
        }
    }

    /// True when sslmode requests verification (verify-ca/verify-full) but the
    /// effective configuration disables it, so the server's identity is not
    /// checked. Mirrors the verify-* decision in `configure_pg_tls_verification`.
    pub fn verify_mode_skips_verification(&self) -> bool {
        matches!(
            self.sslmode.as_deref(),
            Some("verify-ca") | Some("verify-full")
        ) && self.accept_invalid_certs.unwrap_or(
            self.root_certificate_pem
                .as_deref()
                .unwrap_or("")
                .is_empty(),
        )
    }

    /// Configure certificate and hostname verification on a native-tls connector
    /// according to the requested Postgres `sslmode`. The crates.io tokio-postgres
    /// build only parses disable/prefer/require, so verify-ca and verify-full are
    /// enforced here, on the connector, rather than through the connection URI.
    ///
    ///   verify-full — verify the certificate chain AND that it matches the host.
    ///   verify-ca   — verify the chain only; libpq does not check the hostname.
    ///   require / other — encrypt without verifying identity, unless a root
    ///                     certificate is supplied (then verify the chain).
    ///
    /// `accept_invalid_certs` only applies to verify-ca/verify-full: `Some(true)`
    /// accepts any certificate, `Some(false)` enforces verification, and `None`
    /// falls back to the legacy behavior — verify only when a root certificate is
    /// present — so resources predating the flag (including git-synced ones, whose
    /// source never sets it) keep working unchanged. Verification uses the OS trust
    /// store plus any supplied root certificate. Returns false when the connector
    /// was set to accept any certificate, so callers can surface that an unverified
    /// connection is being made.
    fn configure_pg_tls_verification(
        builder: &mut native_tls::TlsConnectorBuilder,
        sslmode: Option<&str>,
        root_certificate_pem: Option<&str>,
        accept_invalid_certs: Option<bool>,
    ) -> Result<bool, error::Error> {
        use native_tls::Certificate;

        let custom_root = match root_certificate_pem {
            Some(pem) if !pem.is_empty() => Some(
                Certificate::from_pem(pem.as_bytes())
                    .map_err(|e| error::Error::BadConfig(format!("Invalid Certs: {e:#}")))?,
            ),
            _ => None,
        };

        match sslmode {
            Some("verify-full") | Some("verify-ca") => {
                // Unset falls back to the legacy behavior: verify iff a cert is present.
                if accept_invalid_certs.unwrap_or(custom_root.is_none()) {
                    builder
                        .danger_accept_invalid_certs(true)
                        .danger_accept_invalid_hostnames(true);
                    return Ok(false);
                }
                if let Some(cert) = custom_root {
                    builder.add_root_certificate(cert);
                }
                if sslmode == Some("verify-ca") {
                    // verify-ca verifies the chain but, per libpq, not the hostname.
                    builder.danger_accept_invalid_hostnames(true);
                }
                Ok(true)
            }
            _ => {
                // "require": accept_invalid_certs does not apply. Encrypt but do not
                // verify identity, unless an explicit root certificate was supplied
                // (then verify the chain).
                if let Some(cert) = custom_root {
                    builder.add_root_certificate(cert);
                    Ok(true)
                } else {
                    builder
                        .danger_accept_invalid_certs(true)
                        .danger_accept_invalid_hostnames(true);
                    Ok(false)
                }
            }
        }
    }

    async fn connect_inner(
        &self,
    ) -> Result<(tokio_postgres::Client, TokioPgConnection), error::Error> {
        use native_tls::TlsConnector;
        use postgres_native_tls::MakeTlsConnector;
        use tokio_postgres::tls::NoTls;
        let ssl_mode_is_require = matches!(
            self.sslmode.as_deref(),
            Some("require") | Some("verify-ca") | Some("verify-full")
        );

        if ssl_mode_is_require {
            tracing::info!("Creating new connection");
            let mut connector = TlsConnector::builder();
            Self::configure_pg_tls_verification(
                &mut connector,
                self.sslmode.as_deref(),
                self.root_certificate_pem.as_deref(),
                self.accept_invalid_certs,
            )?;
            if self.verify_mode_skips_verification() {
                tracing::warn!(
                    "Postgres connection with sslmode={} is not verifying the server certificate (accept_invalid_certs is set, or no root certificate is configured and the resource predates that flag). Set accept_invalid_certs=false or provide root_certificate_pem to enforce verification.",
                    self.sslmode.as_deref().unwrap_or("")
                );
            }

            let (client, connection) = tokio::time::timeout(
                std::time::Duration::from_secs(20),
                tokio_postgres::connect(
                    &self.to_uri(),
                    MakeTlsConnector::new(connector.build().map_err(to_anyhow)?),
                ),
            )
            .await
            .map_err(to_anyhow)?
            .map_err(to_anyhow)?;

            Ok((client, TokioPgConnection::Tls(connection)))
        } else {
            tracing::info!("Creating new connection");
            let (client, connection) = tokio::time::timeout(
                std::time::Duration::from_secs(20),
                tokio_postgres::connect(&self.to_uri(), NoTls),
            )
            .await
            .map_err(to_anyhow)?
            .map_err(to_anyhow)?;

            Ok((client, TokioPgConnection::NoTls(connection)))
        }
    }

    #[cfg(all(feature = "enterprise", feature = "private"))]
    pub async fn connect_with_iam(
        &self,
    ) -> Result<(tokio_postgres::Client, TokioPgConnection), error::Error> {
        use native_tls::TlsConnector;
        use postgres_native_tls::MakeTlsConnector;

        // Resolve region: resource field takes priority, then env var
        let region = match self.region.as_deref() {
            Some(r) => r.to_string(),
            None => std::env::var("AWS_REGION").map_err(|_| {
                error::Error::BadConfig(
                    "Region is required for IAM RDS auth. Set 'region' on the resource or AWS_REGION env var".to_string(),
                )
            })?,
        };

        let port = self.port.unwrap_or(5432);
        let user = self.user.as_deref().unwrap_or("postgres");

        let token = db_iam_ee::generate_auth_token(&region, &self.host, port as u64, user)
            .await
            .map_err(|e| {
                error::Error::InternalErr(format!("IAM token generation failed: {e:#}"))
            })?;

        // RDS IAM auth requires SSL.
        let mut connector = TlsConnector::builder();
        let verified = Self::configure_pg_tls_verification(
            &mut connector,
            self.sslmode.as_deref(),
            self.root_certificate_pem.as_deref(),
            self.accept_invalid_certs,
        )?;
        if !verified {
            tracing::warn!("IAM RDS auth without certificate verification: TLS certificate verification is disabled. Provide root_certificate_pem (and set sslmode=verify-full) to enforce verification.");
        }

        tracing::info!("Creating new IAM RDS connection to {}", &self.host);

        // Use Config builder directly to pass the IAM token as the password.
        // This avoids needing to URL-encode the token into a connection string.
        let mut config = tokio_postgres::Config::new();
        config
            .host(&self.host)
            .port(port as u16)
            .user(user)
            .password(&token)
            .dbname(&self.dbname)
            .ssl_mode(tokio_postgres::config::SslMode::Require);

        let (client, connection) = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            config.connect(MakeTlsConnector::new(connector.build().map_err(to_anyhow)?)),
        )
        .await
        .map_err(to_anyhow)?
        .map_err(to_anyhow)?;

        Ok((client, TokioPgConnection::Tls(connection)))
    }

    pub fn parse_uri(url: &str) -> Result<Self, Error> {
        let parsed_url = url::Url::parse(url)
            .map_err(|_| Error::BadConfig("Invalid PostgreSQL URL".to_string()))?;

        let username = parsed_url.username().to_string();
        let username = urlencoding::decode(&username)
            .map_err(to_anyhow)?
            .to_string();
        let password = parsed_url.password().map(|p| p.to_string());
        let password = match password {
            Some(p) => Some(urlencoding::decode(&p).map_err(to_anyhow)?.to_string()),
            None => None,
        };
        let host = parsed_url
            .host_str()
            .ok_or_else(|| Error::BadConfig("Missing host in PostgreSQL URL".to_string()))?
            .to_string();
        let port = parsed_url.port();
        let dbname = parsed_url.path().trim_start_matches('/').to_string();
        let mut sslmode = None;
        for query in parsed_url.query_pairs() {
            if query.0 == "sslmode" {
                sslmode = Some(query.1.to_string());
            }
        }

        Ok(PgDatabase {
            user: if username.is_empty() {
                None
            } else {
                Some(username)
            },
            password,
            host,
            port,
            dbname,
            sslmode,
            root_certificate_pem: None,
            accept_invalid_certs: None,
            use_iam_auth: None,
            region: None,
        })
    }
}

/// Validate a database name to prevent SQL injection.
/// Must start with a letter, contain only alphanumeric characters, underscores, or hyphens, and be <= 63 chars.
pub fn validate_dbname(dbname: &str) -> error::Result<()> {
    let dbname = dbname.trim();
    if dbname.is_empty() {
        return Err(error::Error::BadRequest(
            "Database name cannot be empty".to_string(),
        ));
    }
    if dbname.len() > 63 {
        return Err(error::Error::BadRequest(
            "Database name cannot exceed 63 characters".to_string(),
        ));
    }
    if !dbname
        .chars()
        .next()
        .map_or(false, |c| c.is_ascii_alphabetic())
    {
        return Err(error::Error::BadRequest(
            "Database name must start with a letter".to_string(),
        ));
    }
    if !dbname
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(error::Error::BadRequest(
            "Database name must contain only alphanumeric characters, underscores, or hyphens"
                .to_string(),
        ));
    }
    Ok(())
}

/// Drop a custom instance database: validate, terminate connections, DROP DATABASE, remove from global_settings.
pub async fn drop_custom_instance_database(db: &DB, dbname: &str) -> error::Result<()> {
    let dbname = dbname.trim();
    validate_dbname(dbname)?;

    let wmill_pg_creds = PgDatabase::parse_uri(&get_database_url().await?.as_str().await)?;
    if wmill_pg_creds.dbname.trim().eq_ignore_ascii_case(dbname) {
        return Err(error::Error::BadRequest(
            "Cannot drop the main Windmill database".to_string(),
        ));
    }

    let db_exists = sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM pg_catalog.pg_database WHERE datname = $1)",
        dbname
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);

    if db_exists {
        // Terminate active connections
        // SAFETY: `dbname` has been validated via validate_dbname() before reaching this point.
        if let Err(e) = sqlx::query(&format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}' AND pid <> pg_backend_pid()",
            dbname.replace('\'', "''")
        ))
        .execute(db)
        .await
        {
            tracing::warn!("Failed to terminate connections to '{}': {}", dbname, e);
        }

        // Drop the database
        // SAFETY: `dbname` has been validated via validate_dbname() before reaching this point.
        sqlx::query(&format!("DROP DATABASE IF EXISTS \"{}\"", dbname))
            .execute(db)
            .await
            .map_err(|e| {
                error::Error::internal_err(format!("Failed to drop database '{}': {}", dbname, e))
            })?;

        tracing::info!("Dropped instance database '{}'", dbname);
    } else {
        tracing::info!("Database '{}' does not exist, skipping drop", dbname);
    }

    // Always remove from global_settings
    sqlx::query!(
        r#"UPDATE global_settings SET value = value #- ARRAY['databases', $1] WHERE name = 'custom_instance_pg_databases'"#,
        dbname
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Create a custom instance database: CREATE DATABASE, grant permissions, register in global_settings.
/// The `tag` is stored in global_settings metadata (e.g. "datatable" or "ducklake").
pub async fn create_custom_instance_database(
    db: &DB,
    dbname: &str,
    tag: &str,
) -> error::Result<()> {
    let dbname = dbname.trim();
    validate_dbname(dbname)?;

    let db_exists = sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM pg_catalog.pg_database WHERE datname = $1)",
        dbname
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);

    if db_exists {
        return Err(error::Error::BadRequest(format!(
            "Database '{}' already exists",
            dbname
        )));
    }

    // SAFETY: `dbname` has been validated via validate_dbname() before reaching this point.
    sqlx::query(&format!("CREATE DATABASE \"{}\"", dbname))
        .execute(db)
        .await
        .map_err(|e| {
            error::Error::internal_err(format!("Failed to create database '{}': {}", dbname, e))
        })?;

    // Grant permissions to custom_instance_user
    let wmill_pg_creds = PgDatabase::parse_uri(&get_database_url().await?.as_str().await)?;
    let new_pg_creds = PgDatabase { dbname: dbname.to_string(), ..wmill_pg_creds };
    let (client, connection) = new_pg_creds.connect(Some(db)).await?;
    let join_handle = tokio::spawn(async move { connection.await });

    if let Err(e) = client
        .batch_execute(&format!(
            "GRANT CONNECT ON DATABASE \"{dbname}\" TO custom_instance_user;
             GRANT USAGE ON SCHEMA public TO custom_instance_user;
             GRANT CREATE ON SCHEMA public TO custom_instance_user;
             GRANT CREATE ON DATABASE \"{dbname}\" TO custom_instance_user;
             ALTER DEFAULT PRIVILEGES IN SCHEMA public
                 GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO custom_instance_user;"
        ))
        .await
    {
        tracing::warn!(
            "Failed to grant permissions on '{}': {}. Continuing.",
            dbname,
            e
        );
    }

    drop(client);
    join_handle
        .await
        .map_err(|e| error::Error::internal_err(format!("join error: {}", e)))?
        .map_err(|e| error::Error::internal_err(format!("tokio_postgres error: {}", e)))?;

    // Register in global_settings
    let status_json = serde_json::json!({
        "logs": {
            "created_database": "OK",
            "db_connect": "OK",
            "grant_permissions": "OK"
        },
        "success": true,
        "error": null,
        "tag": tag
    });
    sqlx::query!(
        r#"UPDATE global_settings SET value = jsonb_set(value, '{databases}', (COALESCE(value->'databases', '{}'::jsonb) || to_jsonb($1::json))) WHERE name = 'custom_instance_pg_databases'"#,
        serde_json::json!({ (dbname): status_json })
    )
    .execute(db)
    .await?;

    tracing::info!("Created custom instance database '{}'", dbname);
    Ok(())
}

#[derive(Clone)]
pub enum DatabaseUrl {
    #[cfg(all(feature = "enterprise", feature = "private"))]
    IamRds(std::sync::Arc<tokio::sync::RwLock<db_iam_ee::IamRdsUrl>>),
    #[cfg(all(feature = "enterprise", feature = "private"))]
    EntraId(std::sync::Arc<tokio::sync::RwLock<db_entra_ee::EntraIdUrl>>),
    Static(String),
}

impl DatabaseUrl {
    /// Get the database URL as a string.
    /// For token-based auth, this returns the original URL (for metadata extraction).
    /// For actual database connections, use connect_options() instead.
    pub async fn as_str(&self) -> String {
        match self {
            #[cfg(all(feature = "enterprise", feature = "private"))]
            DatabaseUrl::IamRds(rds_url) => {
                let guard = rds_url.read().await;
                guard.as_str().to_string()
            }
            #[cfg(all(feature = "enterprise", feature = "private"))]
            DatabaseUrl::EntraId(entra_url) => {
                let guard = entra_url.read().await;
                guard.as_str().to_string()
            }
            DatabaseUrl::Static(url) => url.clone(),
        }
    }

    /// Get PgConnectOptions for this database URL.
    /// For token-based auth (IAM RDS, Entra ID), this returns options built directly from the
    /// token to avoid double-encoding issues with temporary credentials.
    /// For static URLs, this parses the URL string.
    pub async fn connect_options(&self) -> Result<sqlx::postgres::PgConnectOptions, Error> {
        match self {
            #[cfg(all(feature = "enterprise", feature = "private"))]
            DatabaseUrl::IamRds(rds_url) => {
                let guard = rds_url.read().await;
                Ok(guard.connect_options())
            }
            #[cfg(all(feature = "enterprise", feature = "private"))]
            DatabaseUrl::EntraId(entra_url) => {
                let guard = entra_url.read().await;
                Ok(guard.connect_options())
            }
            DatabaseUrl::Static(url) => sqlx::postgres::PgConnectOptions::from_str(url)
                .map_err(|e| Error::InternalErr(format!("Failed to parse database URL: {}", e))),
        }
    }

    pub async fn refresh(&self) -> anyhow::Result<()> {
        match self {
            #[cfg(all(feature = "enterprise", feature = "private"))]
            DatabaseUrl::IamRds(rds_url) => rds_url.write().await.refresh().await,
            #[cfg(all(feature = "enterprise", feature = "private"))]
            DatabaseUrl::EntraId(entra_url) => entra_url.write().await.refresh().await,
            DatabaseUrl::Static(_) => Ok(()),
        }
    }

    pub async fn needs_refresh(&self) -> bool {
        match self {
            #[cfg(all(feature = "enterprise", feature = "private"))]
            DatabaseUrl::IamRds(rds_url) => rds_url.read().await.needs_refresh(),
            #[cfg(all(feature = "enterprise", feature = "private"))]
            DatabaseUrl::EntraId(entra_url) => entra_url.read().await.needs_refresh(),
            DatabaseUrl::Static(_) => false,
        }
    }

    /// Double-checked refresh: read-lock to check, then write-lock to refresh if still needed.
    pub async fn refresh_if_needed(&self) -> Result<(), Error> {
        if self.needs_refresh().await {
            self.refresh().await.map_err(|e| {
                Error::InternalErr(format!("Failed to refresh database token: {}", e))
            })?;
        }
        Ok(())
    }
}

static DATABASE_URL_CACHE: tokio::sync::OnceCell<DatabaseUrl> = tokio::sync::OnceCell::const_new();

pub async fn get_database_url() -> Result<DatabaseUrl, Error> {
    let database_url = DATABASE_URL_CACHE
        .get_or_try_init(|| async {
            use std::env::var;
            use tokio::fs::File;
            use tokio::io::AsyncReadExt;

            let url = match var("DATABASE_URL_FILE") {
                Ok(file_path) => {
                    let mut file = File::open(file_path).await?;
                    let mut contents = String::new();
                    file.read_to_string(&mut contents).await?;
                    Ok(contents.trim().to_string())
                }
                Err(_) => var("DATABASE_URL").map_err(|_| {
                    Error::BadConfig(
                        "Either DATABASE_URL_FILE or DATABASE_URL env var is missing".to_string(),
                    )
                }),
            }?;

            let parsed_url = url::Url::parse(&url)?;

            let password = parsed_url.password().unwrap_or_default();

            if password == "iamrds" {
                let region = var("AWS_REGION").map_err(|_| {
                    Error::BadConfig(
                        "AWS_REGION env var is required for IAM RDS authentication".to_string(),
                    )
                })?;

                tracing::info!("iamrds mode detected, generating IAM RDS URL for region: {region}");
                #[cfg(all(feature = "enterprise", feature = "private"))]
                {
                    let rds_url = db_iam_ee::generate_database_url(&url, &region)
                        .await
                        .map_err(|e| {
                            Error::InternalErr(format!(
                                "Failed to generate IAM database URL: {}",
                                e
                            ))
                        })?;
                    tracing::info!("IAM RDS URL generated successfully");
                    Ok::<DatabaseUrl, Error>(DatabaseUrl::IamRds(std::sync::Arc::new(
                        tokio::sync::RwLock::new(rds_url),
                    )))
                }

                #[cfg(not(all(feature = "enterprise", feature = "private")))]
                {
                    return Err(Error::BadConfig(
                        "IAM RDS authentication is not enabled in OSS mode".to_string(),
                    ));
                }
            } else if password == "entraid" {
                let tenant_id = var("AZURE_TENANT_ID").map_err(|_| {
                    Error::BadConfig(
                        "AZURE_TENANT_ID env var is required for Entra ID authentication"
                            .to_string(),
                    )
                })?;

                tracing::info!(
                    "entraid mode detected, generating Entra ID URL for tenant: {tenant_id}"
                );
                #[cfg(all(feature = "enterprise", feature = "private"))]
                {
                    let client_id = var("AZURE_CLIENT_ID").map_err(|_| {
                        Error::BadConfig(
                            "AZURE_CLIENT_ID env var is required for Entra ID authentication"
                                .to_string(),
                        )
                    })?;
                    let federated_token_file =
                        var("AZURE_FEDERATED_TOKEN_FILE").map_err(|_| {
                            Error::BadConfig(
                                "AZURE_FEDERATED_TOKEN_FILE env var is required for Entra ID authentication".to_string(),
                            )
                        })?;
                    let authority_host = var("AZURE_AUTHORITY_HOST")
                        .unwrap_or_else(|_| "login.microsoftonline.com".to_string());

                    let entra_url = db_entra_ee::generate_database_url(
                        &url,
                        &tenant_id,
                        &client_id,
                        &federated_token_file,
                        &authority_host,
                    )
                    .await
                    .map_err(|e| {
                        Error::InternalErr(format!(
                            "Failed to generate Entra ID database URL: {}",
                            e
                        ))
                    })?;
                    tracing::info!("Entra ID URL generated successfully");
                    Ok::<DatabaseUrl, Error>(DatabaseUrl::EntraId(std::sync::Arc::new(
                        tokio::sync::RwLock::new(entra_url),
                    )))
                }

                #[cfg(not(all(feature = "enterprise", feature = "private")))]
                {
                    return Err(Error::BadConfig(
                        "Entra ID authentication is not enabled in OSS mode".to_string(),
                    ));
                }
            } else {
                Ok::<DatabaseUrl, Error>(DatabaseUrl::Static(url.to_string()))
            }
        })
        .await?;

    database_url.refresh_if_needed().await?;

    Ok(database_url.clone())
}

type Tag = String;

pub use db::DB;

use crate::{
    auth::{PermsCache, FLOW_PERMS_CACHE, HASH_PERMS_CACHE},
    db::{AuthedRef, UserDbWithAuthed},
    error::to_anyhow,
    scripts::{ScriptHash, ScriptRunnableSettingsHandle, ScriptRunnableSettingsInline},
};

#[derive(Clone)]
pub struct ExpiringLatestVersionId {
    id: i64,
    expires_at: std::time::Instant,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ScriptHashInfo<SR> {
    pub path: String,
    pub hash: i64,
    pub tag: Option<String>,
    pub cache_ttl: Option<i32>,
    pub cache_ignore_s3_path: Option<bool>,
    pub language: ScriptLang,
    pub dedicated_worker: Option<bool>,
    pub priority: Option<i16>,
    pub delete_after_use: Option<bool>,
    pub delete_after_secs: Option<i32>,
    pub timeout: Option<i32>,
    pub has_preprocessor: Option<bool>,
    pub on_behalf_of_email: Option<String>,
    pub created_by: String,
    pub labels: Option<Vec<String>>,
    #[sqlx(flatten)]
    pub runnable_settings: SR,
}

impl ScriptHashInfo<ScriptRunnableSettingsHandle> {
    pub async fn prefetch_cached<'a>(
        self,
        db: &DB,
    ) -> error::Result<ScriptHashInfo<ScriptRunnableSettingsInline>> {
        let rs =
            runnable_settings::from_handle(self.runnable_settings.runnable_settings_handle, db)
                .await?;
        let (debouncing_settings, concurrency_settings) =
            runnable_settings::prefetch_cached(&rs, db).await?;

        Ok(ScriptHashInfo {
            path: self.path,
            hash: self.hash,
            tag: self.tag,
            cache_ttl: self.cache_ttl,
            cache_ignore_s3_path: self.cache_ignore_s3_path,
            language: self.language,
            dedicated_worker: self.dedicated_worker,
            priority: self.priority,
            delete_after_use: self.delete_after_use,
            delete_after_secs: self.delete_after_secs,
            timeout: self.timeout,
            has_preprocessor: self.has_preprocessor,
            on_behalf_of_email: self.on_behalf_of_email,
            created_by: self.created_by,
            labels: self.labels,
            runnable_settings: ScriptRunnableSettingsInline {
                concurrency_settings: concurrency_settings.maybe_fallback(
                    self.runnable_settings.concurrency_key,
                    self.runnable_settings.concurrent_limit,
                    self.runnable_settings.concurrency_time_window_s,
                ),
                debouncing_settings: debouncing_settings.maybe_fallback(
                    self.runnable_settings.debounce_key,
                    self.runnable_settings.debounce_delay_s,
                ),
            },
        })
    }
}

pub fn get_latest_deployed_hash_for_path<'e>(
    db: Option<UserDbWithAuthed<'e, AuthedRef<'e>>>,
    db2: DB,
    w_id: &'e str,
    script_path: &'e str,
) -> impl Future<Output = error::Result<ScriptHashInfo<ScriptRunnableSettingsHandle>>> + Send + 'e {
    async move {
        let cache_key = (w_id.to_string(), script_path.to_string());
        let use_cache = !DEPLOYED_SCRIPT_CACHE_DISABLED.load(std::sync::atomic::Ordering::Relaxed);
        let mut computed_hash = None;
        let hash = match DEPLOYED_SCRIPT_HASH_CACHE
            .get(&cache_key)
            .filter(|_| use_cache)
        {
            Some(cached_hash)
                if cached_hash.expires_at > std::time::Instant::now()
                    && db.as_ref().is_none_or(|x| {
                        let r = HASH_PERMS_CACHE
                            .check_perms_in_cache(x.authed, ScriptHash(cached_hash.id));
                        computed_hash = Some(r.1);
                        return r.0;
                    }) =>
            {
                tracing::debug!(
                    "Using cached script hash {} for {script_path}",
                    cached_hash.id
                );
                cached_hash.id
            }
            _ => {
                tracing::debug!("Fetching script hash for {script_path}");
                let hash = if let Some(db) = db {
                    let authed = db.authed;
                    let mut conn = db.acquire().await?;
                    let hash = get_latest_script_hash(&mut *conn, script_path, w_id).await?;
                    if let Some(hash) = hash {
                        HASH_PERMS_CACHE.insert(
                            computed_hash.unwrap_or_else(|| PermsCache::compute_hash(authed)),
                            ScriptHash(hash),
                        );
                    } else {
                        let mut conn = db2.acquire().await?;
                        let exists = get_latest_script_hash(&mut *conn, script_path, w_id)
                            .await?
                            .is_some();
                        if exists {
                            return Err(Error::NotAuthorized(format!("You are not authorized to access this script: {script_path} (but it exists). Your permissions are: {:?}", authed)));
                        }
                    }
                    hash
                } else {
                    let mut conn = db2.acquire().await?;
                    get_latest_script_hash(&mut *conn, script_path, w_id).await?
                };

                let hash = utils::not_found_if_none(hash, "script", script_path)?;
                if use_cache {
                    DEPLOYED_SCRIPT_HASH_CACHE.insert(
                        cache_key,
                        ExpiringLatestVersionId {
                            id: hash,
                            expires_at: std::time::Instant::now() + LATEST_VERSION_ID_CACHE_TTL,
                        },
                    );
                }

                hash
            }
        };

        get_script_info_for_hash(None, &db2, w_id, hash).await
    }
}

pub async fn get_latest_script_hash<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    script_path: &'e str,
    w_id: &'e str,
) -> error::Result<Option<i64>> {
    let hash = sqlx::query_scalar!(
        "select hash from script where path = $1 AND workspace_id = $2 AND deleted = false AND lock IS not NULL AND lock_error_logs IS NULL ORDER BY created_at DESC LIMIT 1",
        script_path,
        w_id
    )
    .fetch_optional(db)
    .await?;
    return Ok(hash);
}

/// Latest non-archived hash for an imported `path`, for bundle cache keying.
/// MUST select the same row as the bundler's content endpoint
/// (`raw_script_by_path_internal`: `archived = false ORDER BY created_at DESC`,
/// no lock predicate) — a stricter filter here would let the key point at an
/// older version than the content that gets inlined. Cached with the same
/// freshness contract as that endpoint's `RAW_SCRIPT_LATEST_HASH_CACHE`:
/// evicted by `notify_runnable_version_change` events, 60s TTL fallback.
pub async fn get_latest_script_hash_for_import_cached(
    db: &DB,
    w_id: &str,
    script_path: &str,
) -> error::Result<Option<i64>> {
    let use_cache = !DEPLOYED_SCRIPT_CACHE_DISABLED.load(std::sync::atomic::Ordering::Relaxed);
    let cache_key = (w_id.to_string(), script_path.to_string());
    if use_cache {
        if let Some(cached) = IMPORTED_SCRIPT_HASH_CACHE.get(&cache_key) {
            if cached.expires_at > std::time::Instant::now() {
                return Ok(Some(cached.id));
            }
        }
    }
    let hash = sqlx::query_scalar!(
        "SELECT hash FROM script WHERE path = $1 AND workspace_id = $2 AND archived = false ORDER BY created_at DESC LIMIT 1",
        script_path,
        w_id
    )
    .fetch_optional(db)
    .await?;
    if let (true, Some(hash)) = (use_cache, hash) {
        IMPORTED_SCRIPT_HASH_CACHE.insert(
            cache_key,
            ExpiringLatestVersionId {
                id: hash,
                expires_at: std::time::Instant::now() + LATEST_VERSION_ID_CACHE_TTL,
            },
        );
    }
    Ok(hash)
}

pub async fn get_script_info_for_hash<'e, E: sqlx::PgExecutor<'e>>(
    db_authed: Option<UserDbWithAuthed<'e, AuthedRef<'e>>>,
    db: E,
    w_id: &str,
    hash: i64,
) -> error::Result<ScriptHashInfo<ScriptRunnableSettingsHandle>> {
    let key = (w_id.to_string(), hash);
    let use_cache = !DEPLOYED_SCRIPT_CACHE_DISABLED.load(std::sync::atomic::Ordering::Relaxed);

    let mut computed_hash = None;
    match DEPLOYED_SCRIPT_INFO_CACHE.get(&key).filter(|_| use_cache) {
        Some(info)
            if db_authed.as_ref().is_none_or(|x| {
                let r = HASH_PERMS_CACHE.check_perms_in_cache(x.authed, scripts::ScriptHash(hash));
                computed_hash = Some(r.1);
                return r.0;
            }) =>
        {
            tracing::debug!("Using cached deployed script info for {hash}");
            Ok(info)
        }
        _ => {
            tracing::debug!("Fetching deployed script info for {hash}");
            let info = if let Some(db_authed) = db_authed {
                let mut conn = db_authed.acquire().await?;
                let hash_info = get_script_info_for_hash_inner(&mut *conn, w_id, hash).await?;
                if hash_info.is_some() {
                    HASH_PERMS_CACHE.insert(
                        computed_hash.unwrap_or_else(|| PermsCache::compute_hash(db_authed.authed)),
                        ScriptHash(hash),
                    );
                }
                hash_info
            } else {
                get_script_info_for_hash_inner(db, w_id, hash).await?
            };

            let info = utils::not_found_if_none(info, "script", &hash.to_string())?;

            if use_cache {
                DEPLOYED_SCRIPT_INFO_CACHE.insert(key, info.clone());
            }

            Ok(info)
        }
    }
}

async fn get_script_info_for_hash_inner<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    w_id: &str,
    hash: i64,
) -> error::Result<Option<ScriptHashInfo<ScriptRunnableSettingsHandle>>> {
    let r = sqlx::query_as::<_, ScriptHashInfo<ScriptRunnableSettingsHandle>>(
        "SELECT
                hash,
                tag,
                concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                debounce_key,
                debounce_delay_s,
                runnable_settings_handle,
                cache_ttl,
                cache_ignore_s3_path,
                language,
                dedicated_worker,
                priority,
                delete_after_use,
                delete_after_secs,
                timeout,
                has_preprocessor,
                on_behalf_of_email,
                created_by,
                labels,
                path
            FROM script WHERE hash = $1 AND workspace_id = $2",
    )
    .bind(hash)
    .bind(w_id)
    .fetch_optional(db)
    .await?;
    Ok(r)
}
#[derive(Clone)]
pub struct FlowVersionInfo {
    pub version: i64,
    pub tag: Option<String>,
    pub early_return: Option<String>,
    pub has_preprocessor: Option<bool>,
    pub has_failure_module: Option<bool>,
    pub chat_input_enabled: Option<bool>,
    pub on_behalf_of_email: Option<String>,
    pub edited_by: String,
    pub dedicated_worker: Option<bool>,
    pub labels: Option<Vec<String>>,
}

struct CachedFlowPath(String);

impl Into<u64> for CachedFlowPath {
    fn into(self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.0.hash(&mut hasher);
        hasher.finish()
    }
}
pub fn get_latest_flow_version_id_for_path<
    'a,
    'e,
    A: sqlx::Acquire<'e, Database = Postgres> + Send + 'a,
>(
    db_authed: Option<UserDbWithAuthed<'e, AuthedRef<'e>>>,
    db: A,
    w_id: &'a str,
    path: &'a str,
    use_cache: bool,
) -> impl Future<Output = error::Result<i64>> + Send + 'a
where
    'e: 'a,
{
    // as instructed in the docstring of sqlx::Acquire
    async move {
        let cache_key = (w_id.to_string(), path.to_string());
        let cached_version = if use_cache {
            FLOW_VERSION_CACHE.get(&cache_key)
        } else {
            None
        };
        let mut computed_hash: Option<_> = None;

        let version = match cached_version {
            Some(cached_version)
                if cached_version.expires_at > std::time::Instant::now()
                    && db_authed.as_ref().is_none_or(|x| {
                        let r = FLOW_PERMS_CACHE
                            .check_perms_in_cache(x.authed, CachedFlowPath(path.to_string()));
                        computed_hash = Some(r.1);
                        return r.0;
                    }) =>
            {
                tracing::debug!("Using cached flow version {} for {path}", cached_version.id);
                cached_version.id
            }
            _ => {
                tracing::debug!("Fetching flow version for {path}");
                let version = if let Some(db_authed) = db_authed {
                    let mut conn = db_authed.acquire().await?;
                    let r = get_latest_flow_version_for_path(&mut *conn, w_id, path).await?;
                    if r.is_some() {
                        FLOW_PERMS_CACHE.insert(
                            computed_hash
                                .unwrap_or_else(|| PermsCache::compute_hash(db_authed.authed)),
                            CachedFlowPath(path.to_string()),
                        );
                    } else {
                        let mut conn = db.acquire().await?;
                        let exists = get_latest_flow_version_for_path(&mut *conn, w_id, path)
                            .await?
                            .is_some();
                        if exists {
                            return Err(Error::NotAuthorized(format!(
                                "You are not authorized to access this flow: {path} (but it exists). Your permissions are: {:?}",
                                db_authed.authed
                            )));
                        }
                    }
                    r
                } else {
                    let mut conn = db.acquire().await?;
                    get_latest_flow_version_for_path(&mut *conn, w_id, path).await?
                };

                let version = utils::not_found_if_none(version, "flow", path)?;

                FLOW_VERSION_CACHE.insert(
                    cache_key,
                    ExpiringLatestVersionId {
                        id: version,
                        expires_at: std::time::Instant::now() + LATEST_VERSION_ID_CACHE_TTL,
                    },
                );

                version
            }
        };
        Ok(version)
    }
}

pub fn get_flow_version_info_from_version<
    'a,
    'e,
    A: sqlx::Acquire<'e, Database = Postgres> + Send + 'a,
>(
    db: A,
    version: i64,
    w_id: &'a str,
    path: &'a str,
) -> impl Future<Output = error::Result<FlowVersionInfo>> + Send + 'a {
    async move {
        // as instructed in the docstring of sqlx::Acquire
        let key = (w_id.to_string(), version);
        match FLOW_INFO_CACHE.get(&key) {
            Some(info) => {
                tracing::debug!("Using cached flow version info for {version} ({path})");
                Ok(info)
            }
            _ => {
                tracing::debug!("Fetching flow version info for {version} ({path})");
                let mut conn = db.acquire().await?;
                let flow_info =
                        sqlx::query_as!(
                            FlowVersionInfo,
                            r#"
                                SELECT
                                    flow_version.id AS version,
                                    flow_version.value->>'early_return' as early_return,
                                    flow_version.value->>'preprocessor_module' IS NOT NULL as has_preprocessor,
                                    flow_version.value->>'failure_module' IS NOT NULL as has_failure_module,
                                    (flow_version.value->>'chat_input_enabled')::boolean as chat_input_enabled,
                                    flow.tag,
                                    flow.dedicated_worker,
                                    flow.on_behalf_of_email,
                                    flow.edited_by,
                                    flow.labels
                                FROM
                                    flow_version
                                INNER JOIN flow
                                    ON flow.path = flow_version.path AND
                                       flow.workspace_id = flow_version.workspace_id
                                WHERE
                                    flow_version.workspace_id = $1 AND
                                    flow_version.path = $2 AND
                                    flow_version.id = $3
                            "#,
                            w_id,
                            path,
                            version,
                        )
                        .fetch_optional(&mut *conn)
                        .await?;

                let info = utils::not_found_if_none(flow_info, "flow", path)?;

                FLOW_INFO_CACHE.insert(key, info.clone());

                Ok(info)
            }
        }
    }
}

/// Resolve a `flow_version.id` to its flow path while enforcing the caller's
/// folder-level ACL. The `flow_version` table has no row-level security, so the
/// authorization gate is an RLS-filtered lookup against the `flow` table through
/// `user_db`. Mirrors the "exists but not authorized -> NotAuthorized" semantics
/// of [`get_latest_flow_version_id_for_path`] so version-keyed run routes are
/// gated identically to their path-keyed siblings.
pub async fn get_flow_path_for_version_authed(
    db_authed: &UserDbWithAuthed<'_, AuthedRef<'_>>,
    db: &DB,
    version: i64,
    w_id: &str,
) -> error::Result<String> {
    let mut conn = db_authed.acquire().await?;
    let authed_path = sqlx::query_scalar!(
        "SELECT flow_version.path FROM flow_version
         INNER JOIN flow
            ON flow.path = flow_version.path AND
               flow.workspace_id = flow_version.workspace_id
         WHERE flow_version.id = $1 AND flow_version.workspace_id = $2",
        version,
        w_id,
    )
    .fetch_optional(&mut *conn)
    .await?;

    if let Some(path) = authed_path {
        return Ok(path);
    }

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM flow_version WHERE id = $1 AND workspace_id = $2)",
        version,
        w_id,
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);

    if exists {
        // Unlike the path-keyed sibling (where the caller already supplied the
        // path), here the caller only supplied an opaque version id. Echoing
        // back the resolved path would disclose an id->path mapping for a flow
        // they cannot access, so the message is intentionally generic.
        return Err(Error::NotAuthorized(
            "You are not authorized to run this flow version".to_string(),
        ));
    }

    Err(Error::NotFound(format!(
        "flow_version not found at id {version}"
    )))
}

pub async fn get_latest_flow_version_info_for_path<'e>(
    db_authed: Option<UserDbWithAuthed<'e, AuthedRef<'e>>>,
    db: &DB,
    w_id: &'e str,
    path: &'e str,
    use_cache: bool,
) -> error::Result<FlowVersionInfo> {
    // as instructed in the docstring of sqlx::Acquire
    let version =
        get_latest_flow_version_id_for_path(db_authed, &db.clone(), w_id, path, use_cache).await?;
    get_flow_version_info_from_version(db, version, w_id, path).await
}

async fn get_latest_flow_version_for_path<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    w_id: &str,
    path: &str,
) -> error::Result<Option<i64>> {
    let version = sqlx::query_scalar!(
        "SELECT flow_version.id from flow
        INNER JOIN flow_version
        ON flow_version.id = flow.versions[array_upper(flow.versions, 1)]
        WHERE flow.path = $1 and flow.workspace_id = $2",
        path,
        w_id
    )
    .fetch_optional(db)
    .await?;
    Ok(version)
}

pub async fn get_latest_hash_for_path<'c, E: sqlx::PgExecutor<'c>>(
    db: E,
    w_id: &str,
    script_path: &str,
    require_locked: bool,
) -> error::Result<(
    scripts::ScriptHash,
    Option<Tag>,
    Option<String>,
    Option<i32>,
    Option<i32>,
    Option<String>,
    Option<i32>,
    Option<i32>,
    Option<bool>,
    ScriptLang,
    Option<bool>,
    Option<i16>,
    Option<i32>,
    Option<String>,
    String,
    Option<i64>,
    Option<Vec<String>>,
)> {
    let r_o = sqlx::query!(
            "select hash, tag, concurrency_key, concurrent_limit, concurrency_time_window_s, debounce_key, debounce_delay_s, cache_ttl, cache_ignore_s3_path, runnable_settings_handle, language as \"language: ScriptLang\", dedicated_worker, priority, timeout, on_behalf_of_email, created_by, labels FROM script
             WHERE path = $1 AND workspace_id = $2 AND archived = false AND (lock IS NOT NULL OR $3 = false)
             ORDER BY created_at DESC LIMIT 1",
            script_path,
            w_id,
            require_locked
        )
        .fetch_optional(db)
        .await?;

    let script = utils::not_found_if_none(r_o, "script", script_path)?;

    Ok((
        scripts::ScriptHash(script.hash),
        script.tag,
        script.concurrency_key,
        script.concurrent_limit,
        script.concurrency_time_window_s,
        script.debounce_key,
        script.debounce_delay_s,
        script.cache_ttl,
        script.cache_ignore_s3_path,
        script.language,
        script.dedicated_worker,
        script.priority,
        script.timeout,
        script.on_behalf_of_email,
        script.created_by,
        script.runnable_settings_handle,
        script.labels,
    ))
}

pub struct KillpillSender {
    tx: broadcast::Sender<()>,
    already_sent: Arc<AtomicBool>,
}

impl Clone for KillpillSender {
    fn clone(&self) -> Self {
        KillpillSender { tx: self.tx.clone(), already_sent: self.already_sent.clone() }
    }
}

impl KillpillSender {
    pub fn new(capacity: usize) -> (Self, broadcast::Receiver<()>) {
        let (tx, rx) = broadcast::channel(capacity);
        let sender = KillpillSender { tx, already_sent: Arc::new(AtomicBool::new(false)) };
        (sender, rx)
    }

    pub fn clone(&self) -> Self {
        KillpillSender { tx: self.tx.clone(), already_sent: self.already_sent.clone() }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }

    // Try to send the killpill if it hasn't been sent already
    pub fn send(&self) -> bool {
        // Check if it's already been sent, and if not, set the flag to true
        if !self.already_sent.swap(true, Ordering::SeqCst) {
            // We're the first to set it to true, so send the signal
            if let Err(e) = self.tx.send(()) {
                tracing::error!("failed to send killpill: {:?}", e);
            }
            true
        } else {
            // Signal was already sent
            false
        }
    }

    // // Force send a signal regardless of previous sends
    // fn force_send(&self) -> Result<usize, broadcast::error::SendError<()>> {
    //     self.already_sent.store(true, Ordering::SeqCst);
    //     self.tx.send(())
    // }

    // // Check if the killpill has been sent
    // fn is_sent(&self) -> bool {
    //     self.already_sent.load(Ordering::SeqCst)
    // }
}
