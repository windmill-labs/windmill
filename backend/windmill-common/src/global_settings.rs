pub const CUSTOM_TAGS_SETTING: &str = "custom_tags";
pub const DEFAULT_TAGS_PER_WORKSPACE_SETTING: &str = "default_tags_per_workspace";
pub const DEFAULT_TAGS_WORKSPACES_SETTING: &str = "default_tags_workspaces";
pub const BASE_URL_SETTING: &str = "base_url";
pub const OAUTH_SETTING: &str = "oauths";
pub const RETENTION_PERIOD_SECS_SETTING: &str = "retention_period_secs";
pub const JOB_DEFAULT_TIMEOUT_SECS_SETTING: &str = "job_default_timeout";
pub const REQUEST_SIZE_LIMIT_SETTING: &str = "request_size_limit_mb";
pub const LICENSE_KEY_SETTING: &str = "license_key";
pub const NPM_CONFIG_REGISTRY_SETTING: &str = "npm_config_registry";
pub const BUNFIG_INSTALL_SCOPES_SETTING: &str = "bunfig_install_scopes";

pub const EXTRA_PIP_INDEX_URL_SETTING: &str = "pip_extra_index_url";
pub const PIP_INDEX_URL_SETTING: &str = "pip_index_url";
pub const SCIM_TOKEN_SETTING: &str = "scim_token";
pub const SAML_METADATA_SETTING: &str = "saml_metadata";

pub const UNIQUE_ID_SETTING: &str = "uid";
pub const DISABLE_STATS_SETTING: &str = "disable_stats";
pub const EXPOSE_METRICS_SETTING: &str = "expose_metrics";
pub const EXPOSE_DEBUG_METRICS_SETTING: &str = "expose_debug_metrics";
pub const KEEP_JOB_DIR_SETTING: &str = "keep_job_dir";
pub const REQUIRE_PREEXISTING_USER_FOR_OAUTH_SETTING: &str = "require_preexisting_user_for_oauth";
pub const OBJECT_STORE_CACHE_CONFIG_SETTING: &str = "object_store_cache_config";

pub const AUTOMATE_USERNAME_CREATION_SETTING: &str = "automate_username_creation";
pub const HUB_BASE_URL_SETTING: &str = "hub_base_url";
pub const CRITICAL_ERROR_CHANNELS_SETTING: &str = "critical_error_channels";
pub const DEV_INSTANCE_SETTING: &str = "dev_instance";
pub const JWT_SECRET_SETTING: &str = "jwt_secret";
pub const EMAIL_DOMAIN_SETTING: &str = "email_domain";

pub const ENV_SETTINGS: [&str; 50] = [
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
];
