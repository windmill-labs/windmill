pub const CUSTOM_TAGS_SETTING: &str = "custom_tags";
pub const DEFAULT_TAGS_PER_WORKSPACE_SETTING: &str = "default_tags_per_workspace";
pub const DEFAULT_TAGS_WORKSPACES_SETTING: &str = "default_tags_workspaces";
pub const BASE_URL_SETTING: &str = "base_url";
pub const OAUTH_SETTING: &str = "oauths";
pub const RETENTION_PERIOD_SECS_SETTING: &str = "retention_period_secs";
pub const MONITOR_LOGS_ON_OBJECT_STORE_SETTING: &str = "monitor_logs_on_s3";
pub const JOB_DEFAULT_TIMEOUT_SECS_SETTING: &str = "job_default_timeout";
pub const REQUEST_SIZE_LIMIT_SETTING: &str = "request_size_limit_mb";
pub const LICENSE_KEY_SETTING: &str = "license_key";
pub const NPM_CONFIG_REGISTRY_SETTING: &str = "npm_config_registry";
pub const BUNFIG_INSTALL_SCOPES_SETTING: &str = "bunfig_install_scopes";
pub const NPMRC_SETTING: &str = "npmrc";
pub const NUGET_CONFIG_SETTING: &str = "nuget_config";
pub const POWERSHELL_REPO_URL_SETTING: &str = "powershell_repo_url";
pub const POWERSHELL_REPO_PAT_SETTING: &str = "powershell_repo_pat";
pub const MAVEN_REPOS_SETTING: &str = "maven_repos";
pub const MAVEN_SETTINGS_XML_SETTING: &str = "maven_settings_xml";
pub const NO_DEFAULT_MAVEN_SETTING: &str = "no_default_maven";
pub const RUBY_REPOS_SETTING: &str = "ruby_repos";
pub const CARGO_REGISTRIES_SETTING: &str = "cargo_registries";

pub const EXTRA_PIP_INDEX_URL_SETTING: &str = "pip_extra_index_url";
pub const PIP_INDEX_URL_SETTING: &str = "pip_index_url";
pub const UV_INDEX_STRATEGY_SETTING: &str = "uv_index_strategy";
pub const INSTANCE_PYTHON_VERSION_SETTING: &str = "instance_python_version";
pub const SCIM_TOKEN_SETTING: &str = "scim_token";
pub const SAML_METADATA_SETTING: &str = "saml_metadata";
pub const SMTP_SETTING: &str = "smtp_settings";
pub const TEAMS_SETTING: &str = "teams";
pub const INDEXER_SETTING: &str = "indexer_settings";
pub const TIMEOUT_WAIT_RESULT_SETTING: &str = "timeout_wait_result";

pub const UNIQUE_ID_SETTING: &str = "uid";
pub const DISABLE_STATS_SETTING: &str = "disable_stats";
pub const EXPOSE_METRICS_SETTING: &str = "expose_metrics";
pub const EXPOSE_DEBUG_METRICS_SETTING: &str = "expose_debug_metrics";
pub const KEEP_JOB_DIR_SETTING: &str = "keep_job_dir";
pub const REQUIRE_PREEXISTING_USER_FOR_OAUTH_SETTING: &str = "require_preexisting_user_for_oauth";
pub const JOB_ISOLATION_SETTING: &str = "job_isolation";
pub const OBJECT_STORE_CONFIG_SETTING: &str = "object_store_cache_config";
pub const HUB_API_SECRET_SETTING: &str = "hub_api_secret";

pub const AUTOMATE_USERNAME_CREATION_SETTING: &str = "automate_username_creation";
pub const HUB_BASE_URL_SETTING: &str = "hub_base_url";
pub const HUB_ACCESSIBLE_URL_SETTING: &str = "hub_accessible_url";
pub const CRITICAL_ERROR_CHANNELS_SETTING: &str = "critical_error_channels";
pub const CRITICAL_ALERT_MUTE_UI_SETTING: &str = "critical_alert_mute_ui";
pub const CRITICAL_ALERTS_ON_DB_OVERSIZE_SETTING: &str = "critical_alerts_on_db_oversize";
pub const DEV_INSTANCE_SETTING: &str = "dev_instance";
pub const JWT_SECRET_SETTING: &str = "jwt_secret";
pub const EMAIL_DOMAIN_SETTING: &str = "email_domain";
pub const OTEL_SETTING: &str = "otel";
pub const OTEL_TRACING_PROXY_SETTING: &str = "otel_tracing_proxy";
pub const APP_WORKSPACED_ROUTE_SETTING: &str = "app_workspaced_route";
pub const SECRET_BACKEND_SETTING: &str = "secret_backend";
pub const MIN_KEEP_ALIVE_VERSION_SETTING: &str = "min_keep_alive_version";

pub const ENV_SETTINGS: &[&str] = &[
    "DISABLE_NSJAIL",
    "MODE",
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
    "COOKIE_DOMAIN",
    "PYTHON_PATH",
    "NU_PATH",
    "DENO_PATH",
    "GO_PATH",
    "JAVA_PATH",
    "RUBY_PATH",
    "BUNDLE_PATH",
    "GEM_PATH",
    "RUBY_CONCURRENT_DOWNLOADS",
    // for related places search: ADD_NEW_LANG
    "GOPRIVATE",
    "GOPROXY",
    "NETRC",
    "CARGO_REGISTRIES",
    "INSTANCE_PYTHON_VERSION",
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
    "INSTANCE_EVENTS_WEBHOOK",
    "CLOUD_HOSTED",
    "GLOBAL_CACHE_INTERVAL",
    "WAIT_RESULT_FAST_POLL_DURATION_SECS",
    "WAIT_RESULT_SLOW_POLL_INTERVAL_MS",
    "WAIT_RESULT_FAST_POLL_INTERVAL_MS",
    "EXIT_AFTER_NO_JOB_FOR_SECS",
    "REQUEST_SIZE_LIMIT",
    "CREATE_WORKSPACE_REQUIRE_SUPERADMIN",
    "GLOBAL_ERROR_HANDLER_PATH_IN_ADMINS_WORKSPACE",
    "MAX_WAIT_FOR_SIGINT",
    "MAX_WAIT_FOR_SIGTERM",
    "WORKER_GROUP",
    "SAML_METADATA",
    "INSTANCE_IS_DEV",
    "OTEL_METRICS",
    "OTEL_TRACING",
    "OTEL_LOGS",
    "DISABLE_S3_STORE",
    "PG_SCHEMA",
    "PG_LISTENER_REFRESH_PERIOD_SECS",
    "AI_REQUEST_TIMEOUT_SECONDS",
    "JOB_CLEANUP_BATCH_SIZE",
    "JOB_CLEANUP_MAX_BATCHES",
];

use crate::error;
use sqlx::postgres::Postgres;
use sqlx::Pool;

pub async fn load_value_from_global_settings(
    db: &Pool<Postgres>,
    setting_name: &str,
) -> error::Result<Option<serde_json::Value>> {
    let r = sqlx::query!(
        "SELECT value FROM global_settings WHERE name = $1",
        setting_name
    )
    .fetch_optional(db)
    .await?
    .map(|x| x.value);
    Ok(r)
}

/// Read OAuth client_id and client_secret from instance-level global settings.
/// `oauth_key` is the key under `oauths` (e.g., "gworkspace", "nextcloud").
pub async fn get_instance_oauth_credentials(
    db: &Pool<Postgres>,
    oauth_key: &str,
) -> error::Result<(String, String)> {
    let oauths_value = load_value_from_global_settings(db, OAUTH_SETTING)
        .await?
        .ok_or_else(|| {
            error::Error::InternalErr("Instance OAuth settings not found".to_string())
        })?;

    let entry = oauths_value.get(oauth_key).ok_or_else(|| {
        error::Error::InternalErr(format!("No {} entry in instance OAuth settings", oauth_key))
    })?;

    let id = entry
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let secret = entry
        .get("secret")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if id.is_empty() || secret.is_empty() {
        return Err(error::Error::InternalErr(format!(
            "Instance OAuth credentials for {} are incomplete",
            oauth_key
        )));
    }

    Ok((id, secret))
}

/// Map service client name to the OAuth settings key in global_settings.
/// e.g. "google" -> "gworkspace", "nextcloud" -> "nextcloud"
pub fn workspace_integration_oauth_key(client_name: &str) -> &str {
    match client_name {
        "google" => "gworkspace",
        other => other,
    }
}

/// Resolve the token endpoint URL for a workspace integration service.
pub fn workspace_integration_token_endpoint(client_name: &str, base_url: &str) -> String {
    match client_name {
        "google" => "https://oauth2.googleapis.com/token".to_string(),
        _ => format!("{}/apps/oauth2/api/v1/token", base_url),
    }
}

/// Resolve the auth endpoint URL for a workspace integration service.
pub fn workspace_integration_auth_endpoint(client_name: &str, base_url: &str) -> String {
    match client_name {
        "google" => "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
        _ => format!("{}/apps/oauth2/authorize", base_url),
    }
}

pub async fn set_value_in_global_settings(
    db: &Pool<Postgres>,
    setting_name: &str,
    value: serde_json::Value,
) -> error::Result<()> {
    sqlx::query!(
        "INSERT INTO global_settings (name, value) VALUES ($1, $2) ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
        setting_name,
        value
    )
    .execute(db)
    .await?;
    Ok(())
}
