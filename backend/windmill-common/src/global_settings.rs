// Adding a global setting? Decide whether agent workers may read it. Agent
// workers (remote workers connected over HTTP) fetch settings through an
// endpoint that is deny-by-exception: every key is served except those in
// AGENT_WORKER_BLOCKED_SETTINGS (defined below). If a new setting holds an
// instance secret the server should keep to itself, add its key there.
pub const CUSTOM_TAGS_SETTING: &str = "custom_tags";
pub const DEFAULT_TAGS_PER_WORKSPACE_SETTING: &str = "default_tags_per_workspace";
pub const DEFAULT_TAGS_WORKSPACES_SETTING: &str = "default_tags_workspaces";
pub const FORK_WORKSPACE_TAG_APPEND_FORK_SUFFIX_SETTING: &str =
    "fork_workspace_tag_append_fork_suffix";
pub const PREVIEW_TAGS_OVERRIDE_SETTING: &str = "preview_tags_override";
pub const BASE_URL_SETTING: &str = "base_url";
pub const WS_BASE_URL_SETTING: &str = "ws_base_url";
pub const OAUTH_SETTING: &str = "oauths";
pub const AI_CONFIG_SETTING: &str = "ai_config";
pub const RETENTION_PERIOD_SECS_SETTING: &str = "retention_period_secs";
pub const RETENTION_PERIOD_SECS_OVERRIDES_SETTING: &str = "retention_period_secs_overrides";
/// Upper bound on how many per-workspace retention overrides may be configured. The periodic monitor
/// sweeps each override workspace in its own transaction every pass, so this keeps a pass bounded
/// (and the feature is a targeted escape hatch for a handful of special workspaces, not a bulk knob).
/// Enforced at write time and defensively on load.
pub const MAX_RETENTION_OVERRIDE_WORKSPACES: usize = 10;
pub const AUDIT_LOG_RETENTION_DAYS_SETTING: &str = "audit_log_retention_days";
pub const STORE_AUDIT_LOGS_S3_SETTING: &str = "store_audit_logs_s3";
/// `background_task_state.name` for the audit-log → object-store export cursor.
/// NOT a global setting — runtime task state lives in `background_task_state`
/// (see the migration `audit_logs_s3_anchor_on_enable`), so it is never part
/// of instance config / config sync. Keep in sync with the trigger SQL literal.
pub const AUDIT_LOGS_S3_EXPORT_TASK: &str = "audit_logs_s3_export";
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
pub const UV_EXCLUDE_NEWER_SETTING: &str = "uv_exclude_newer";
pub const UV_PYTHON_INSTALL_MIRROR_SETTING: &str = "uv_python_install_mirror";
pub const BUN_INSTALL_MIN_RELEASE_AGE_SETTING: &str = "bun_install_min_release_age";
pub const INSTANCE_PYTHON_VERSION_SETTING: &str = "instance_python_version";
pub const RUFF_CONFIG_SETTING: &str = "ruff_config";
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
pub const NSJAIL_TMPFS_SIZE_MB_SETTING: &str = "nsjail_tmpfs_size_mb";
pub const NSJAIL_TMP_BACKING_SETTING: &str = "nsjail_tmp_backing";
pub const NSJAIL_TMP_BACKING_DISK: &str = "disk";
pub const NSJAIL_TMP_BACKING_TMPFS: &str = "tmpfs";
pub const SANDBOX_IMAGE_MAX_SIZE_MB_SETTING: &str = "sandbox_image_max_size_mb";
pub const SANDBOX_IMAGE_CACHE_MAX_MB_SETTING: &str = "sandbox_image_cache_max_mb";
pub const SANDBOX_IMAGE_PULL_POLICY_SETTING: &str = "sandbox_image_pull_policy";
pub const SANDBOX_IMAGE_DEFAULT_REGISTRY_SETTING: &str = "sandbox_image_default_registry";
pub const SANDBOX_REGISTRY_AUTH_SETTING: &str = "sandbox_registry_auth";
// Enables the `#ssh <resource>` directive that reroutes bash execution to a
// remote host over SSH (enterprise feature). Off by default. See
// windmill-worker/src/ssh_executor_ee.rs.
pub const SSH_EXECUTION_SETTING: &str = "ssh_execution_enabled";
pub const OBJECT_STORE_CONFIG_SETTING: &str = "object_store_cache_config";
pub const HUB_API_SECRET_SETTING: &str = "hub_api_secret";

pub const AUTOMATE_USERNAME_CREATION_SETTING: &str = "automate_username_creation";
pub const DISABLE_WORKSPACE_INVITE_EMAILS_SETTING: &str = "disable_workspace_invite_emails";
pub const DISABLE_PASSWORD_LOGIN_SETTING: &str = "disable_password_login";
pub const AUTO_LOGIN_PROVIDER_SETTING: &str = "auto_login_provider";
pub const HUB_BASE_URL_SETTING: &str = "hub_base_url";
pub const HUB_ACCESSIBLE_URL_SETTING: &str = "hub_accessible_url";
pub const DISABLE_HUB_SETTING: &str = "disable_hub";
pub const CRITICAL_ERROR_CHANNELS_SETTING: &str = "critical_error_channels";
pub const CRITICAL_ALERT_MUTE_UI_SETTING: &str = "critical_alert_mute_ui";
pub const CRITICAL_ALERTS_ON_DB_OVERSIZE_SETTING: &str = "critical_alerts_on_db_oversize";
pub const CRITICAL_ALERTS_ON_TOKEN_EXPIRY_SETTING: &str = "critical_alerts_on_token_expiry";
pub const DEV_INSTANCE_SETTING: &str = "dev_instance";
pub const JWT_SECRET_SETTING: &str = "jwt_secret";
pub const EMAIL_DOMAIN_SETTING: &str = "email_domain";
pub const OTEL_SETTING: &str = "otel";
pub const OTEL_TRACING_PROXY_SETTING: &str = "otel_tracing_proxy";
pub const APP_WORKSPACED_ROUTE_SETTING: &str = "app_workspaced_route";
pub const HTTP_ROUTE_WORKSPACED_ROUTE_SETTING: &str = "http_route_workspaced_route";
pub const SECRET_BACKEND_SETTING: &str = "secret_backend";
pub const MIN_KEEP_ALIVE_VERSION_SETTING: &str = "min_keep_alive_version";
pub const GITHUB_ENTERPRISE_APP_SETTING: &str = "github_enterprise_app";
pub const INSTANCE_EVENTS_WEBHOOK_SETTING: &str = "instance_events_webhook";
pub const WORKSPACE_REGISTRIES_SETTING: &str = "workspace_registries";
pub const RESTART_COORDINATION_SETTING: &str = "_restart_coordination";
pub const ALERT_CONFIG_SETTING: &str = "alert_job_queue_waiting";

// Workspace fairness: cloud-only mechanism that caps any single workspace at
// `workspace_fairness_max_percent`% of the shared worker pool once it has been
// occupying it for more than `workspace_fairness_duration_secs` seconds. See
// `windmill-queue/src/workspace_fairness.rs`.
pub const WORKSPACE_FAIRNESS_ENABLED_SETTING: &str = "workspace_fairness_enabled";
pub const WORKSPACE_FAIRNESS_MAX_PERCENT_SETTING: &str = "workspace_fairness_max_percent";
pub const WORKSPACE_FAIRNESS_DURATION_SECS_SETTING: &str = "workspace_fairness_duration_secs";
pub const WORKSPACE_FAIRNESS_MIN_TOTAL_SETTING: &str = "workspace_fairness_min_total_jobs";

// Cloud-only ceiling on how many jobs may sit in the queue behind a single
// concurrency key. `0` disables the cap. See `windmill-queue/src/jobs.rs`,
// `check_concurrency_key_queue_cap`.
pub const CONCURRENCY_KEY_MAX_QUEUED_SETTING: &str = "concurrency_key_max_queued_jobs";

// Cloud-only ceiling on how many jobs a single workspace may have queued in
// total, across every key and script. Applies even to premium workspaces. `0`
// disables the cap. See `windmill-queue/src/jobs.rs`, `check_workspace_queue_cap`.
pub const WORKSPACE_MAX_QUEUED_JOBS_SETTING: &str = "workspace_max_queued_jobs";

/// Global settings an agent worker (a remote worker connected over HTTP instead
/// of to the database) must NEVER read through
/// `GET /api/agent_workers/get_global_setting/{key}`. Every other key is served.
///
/// SECURITY: that endpoint is authenticated only by an agent-worker JWT and
/// returns the raw `global_settings` value for the requested key. Because the
/// policy is deny-by-exception (anything not listed here is readable), every
/// setting that holds an instance secret or credential an agent worker does not
/// need MUST be listed below. Missing one discloses it to every agent worker —
/// `jwt_secret` is the worst case (a token holder could forge a superadmin JWT),
/// but `oauths`, `smtp_settings`, `secret_backend`, object-store credentials,
/// etc. are instance-wide secrets too.
///
/// NOT blocked, on purpose: the operational credentials an agent worker loads to
/// run jobs (`license_key`, `hub_api_secret`, `sandbox_registry_auth`,
/// `powershell_repo_pat`, `npmrc`, ...). Those are already within an agent
/// worker's trust boundary, and blocking them breaks worker startup or
/// dependency installation. When adding a new setting that stores a secret the
/// server keeps to itself, add it here.
pub const AGENT_WORKER_BLOCKED_SETTINGS: &[&str] = &[
    // Instance identity / auth secrets — disclosure enables privilege escalation
    // or impersonation.
    JWT_SECRET_SETTING,
    OAUTH_SETTING,
    SMTP_SETTING,
    SCIM_TOKEN_SETTING,
    SAML_METADATA_SETTING,
    SECRET_BACKEND_SETTING,
    GITHUB_ENTERPRISE_APP_SETTING,
    OBJECT_STORE_CONFIG_SETTING,
    AI_CONFIG_SETTING,
    TEAMS_SETTING,
    INDEXER_SETTING,
    // Server-only configs that may embed credentials, webhook URLs or tokens and
    // are never loaded by an agent worker.
    CRITICAL_ERROR_CHANNELS_SETTING,
    INSTANCE_EVENTS_WEBHOOK_SETTING,
    OTEL_SETTING,
    OTEL_TRACING_PROXY_SETTING,
    // Custom-instance DB credentials: `custom_instance_pg_databases` holds `user_pwd`,
    // `custom_instance_replication_pwd` holds the REPLICATION-role password. Agent workers
    // resolve datatable connections through the dedicated datatable endpoints, never these.
    "custom_instance_pg_databases",
    "custom_instance_replication_pwd",
];

/// Whether an agent worker may read the given global setting over HTTP.
/// Deny-by-exception: everything is readable except [`AGENT_WORKER_BLOCKED_SETTINGS`].
pub fn is_setting_readable_by_agent_worker(name: &str) -> bool {
    !AGENT_WORKER_BLOCKED_SETTINGS.contains(&name)
}

use std::sync::atomic::AtomicBool;

lazy_static::lazy_static! {
    pub static ref HTTP_ROUTE_WORKSPACED_ROUTE: AtomicBool = AtomicBool::new(false);
    pub static ref DISABLE_PASSWORD_LOGIN: AtomicBool = AtomicBool::new(false);
}

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
    "RSCRIPT_PATH",
    // for related places search: ADD_NEW_LANG
    "GOPRIVATE",
    "GOPROXY",
    "NETRC",
    "CARGO_REGISTRIES",
    "INSTANCE_PYTHON_VERSION",
    "PIP_INDEX_URL",
    "PIP_EXTRA_INDEX_URL",
    "PIP_TRUSTED_HOST",
    "UV_PYTHON_INSTALL_MIRROR",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_workers_can_read_operational_settings() {
        // Operational knobs and the credentials a worker needs to run jobs are
        // intentionally NOT blocked. Deny-by-exception also means an arbitrary
        // unlisted key is readable.
        for key in [
            NPMRC_SETTING,
            PIP_INDEX_URL_SETTING,
            JOB_ISOLATION_SETTING,
            LICENSE_KEY_SETTING,
            HUB_API_SECRET_SETTING,
            SANDBOX_REGISTRY_AUTH_SETTING,
            POWERSHELL_REPO_PAT_SETTING,
            "some_future_operational_setting",
        ] {
            assert!(
                is_setting_readable_by_agent_worker(key),
                "'{key}' must remain readable by agent workers"
            );
        }
    }

    #[test]
    fn agent_workers_cannot_read_instance_secrets() {
        // Disclosing any of these to a remote worker enables privilege
        // escalation (jwt_secret -> forged superadmin JWT) or leaks instance
        // secrets. They must never be served by the agent-worker endpoint.
        for key in [
            JWT_SECRET_SETTING,
            OAUTH_SETTING,
            SMTP_SETTING,
            SCIM_TOKEN_SETTING,
            SAML_METADATA_SETTING,
            SECRET_BACKEND_SETTING,
            GITHUB_ENTERPRISE_APP_SETTING,
            OBJECT_STORE_CONFIG_SETTING,
            AI_CONFIG_SETTING,
            TEAMS_SETTING,
            INDEXER_SETTING,
            CRITICAL_ERROR_CHANNELS_SETTING,
            INSTANCE_EVENTS_WEBHOOK_SETTING,
            OTEL_SETTING,
            OTEL_TRACING_PROXY_SETTING,
            "custom_instance_pg_databases",
            "custom_instance_replication_pwd",
        ] {
            assert!(
                !is_setting_readable_by_agent_worker(key),
                "'{key}' is an instance secret and must not be readable by agent workers"
            );
        }
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
