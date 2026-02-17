use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::global_settings::LICENSE_KEY_SETTING;

// ---------------------------------------------------------------------------
// Kubernetes Secret reference support
// ---------------------------------------------------------------------------

/// Reference to a key inside a Kubernetes Secret.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct SecretKeyRef {
    pub name: String,
    pub key: String,
}

/// Wrapper that produces `{ "secretKeyRef": { "name": "…", "key": "…" } }` in YAML/JSON.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct SecretKeyRefWrapper {
    #[serde(rename = "secretKeyRef")]
    pub secret_key_ref: SecretKeyRef,
}

/// Reference to an environment variable.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct EnvRefWrapper {
    #[serde(rename = "envRef")]
    pub env_ref: String,
}

/// A string field that can be a literal value, a reference to a Kubernetes
/// Secret key, or a reference to an environment variable.
///
/// Uses `#[serde(untagged)]` so that in YAML/JSON:
///
/// - A plain string deserializes as `Literal("…")`
/// - An object `{ secretKeyRef: { name, key } }` deserializes as `SecretRef(…)`
/// - An object `{ envRef: "VAR_NAME" }` deserializes as `EnvRef(…)`
///
/// `Literal` serializes back to a plain JSON string, preserving backwards
/// compatibility with existing consumers.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum StringOrSecretRef {
    Literal(String),
    SecretRef(SecretKeyRefWrapper),
    EnvRef(EnvRefWrapper),
}

impl StringOrSecretRef {
    /// Returns the literal string value, or `None` if this is an unresolved ref.
    pub fn as_literal(&self) -> Option<&str> {
        match self {
            Self::Literal(s) => Some(s),
            Self::SecretRef(_) | Self::EnvRef(_) => None,
        }
    }

    /// Returns the literal value, panicking if this is an unresolved ref.
    /// Use only after resolution.
    pub fn literal_value(&self) -> &str {
        match self {
            Self::Literal(s) => s,
            Self::SecretRef(_) => panic!("literal_value() called on unresolved secret ref"),
            Self::EnvRef(_) => panic!("literal_value() called on unresolved env ref"),
        }
    }

    /// Returns `true` if this is a secret reference (not yet resolved).
    pub fn is_secret_ref(&self) -> bool {
        matches!(self, Self::SecretRef(_))
    }

    /// Returns the inner `SecretKeyRef` if this is a secret reference.
    pub fn as_secret_ref(&self) -> Option<&SecretKeyRef> {
        match self {
            Self::SecretRef(w) => Some(&w.secret_key_ref),
            _ => None,
        }
    }

    /// Returns `true` if this is an environment variable reference.
    pub fn is_env_ref(&self) -> bool {
        matches!(self, Self::EnvRef(_))
    }

    /// Returns the env var name if this is an env ref.
    pub fn as_env_ref(&self) -> Option<&str> {
        match self {
            Self::EnvRef(w) => Some(&w.env_ref),
            _ => None,
        }
    }
}

impl Default for StringOrSecretRef {
    fn default() -> Self {
        Self::Literal(String::new())
    }
}

impl From<String> for StringOrSecretRef {
    fn from(s: String) -> Self {
        Self::Literal(s)
    }
}

impl From<&str> for StringOrSecretRef {
    fn from(s: &str) -> Self {
        Self::Literal(s.to_string())
    }
}

impl PartialEq<str> for StringOrSecretRef {
    fn eq(&self, other: &str) -> bool {
        self.as_literal() == Some(other)
    }
}

impl PartialEq<StringOrSecretRef> for StringOrSecretRef {
    fn eq(&self, other: &StringOrSecretRef) -> bool {
        match (self, other) {
            (Self::Literal(a), Self::Literal(b)) => a == b,
            (Self::SecretRef(a), Self::SecretRef(b)) => a == b,
            (Self::EnvRef(a), Self::EnvRef(b)) => a == b,
            _ => false,
        }
    }
}

impl fmt::Display for StringOrSecretRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Literal(s) => f.write_str(s),
            Self::SecretRef(w) => write!(
                f,
                "secretKeyRef({}/{})",
                w.secret_key_ref.name, w.secret_key_ref.key
            ),
            Self::EnvRef(w) => write!(f, "envRef({})", w.env_ref),
        }
    }
}

// ---------------------------------------------------------------------------
// Top-level wrapper
// ---------------------------------------------------------------------------

/// Unified instance configuration combining global settings and worker configs.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct InstanceConfig {
    #[serde(default)]
    pub global_settings: GlobalSettings,
    #[serde(default)]
    pub worker_configs: BTreeMap<String, WorkerGroupConfig>,
}

// ---------------------------------------------------------------------------
// Global settings
// ---------------------------------------------------------------------------

/// Generate a schema for opaque JSON objects (used for EE-private settings).
/// Produces `{"type": "object", "nullable": true}` so the CRD passes K8s
/// structural schema validation while still accepting any JSON object.
#[cfg(feature = "instance_config_schema")]
fn opaque_json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
    schemars::schema::SchemaObject {
        instance_type: Some(schemars::schema::InstanceType::Object.into()),
        metadata: Some(Box::default()),
        extensions: {
            let mut m = schemars::Map::new();
            m.insert("nullable".to_string(), serde_json::Value::Bool(true));
            m.insert(
                "x-kubernetes-preserve-unknown-fields".to_string(),
                serde_json::Value::Bool(true),
            );
            m
        },
        ..Default::default()
    }
    .into()
}

/// Typed global settings with schema validation.
/// Known settings have explicit fields; unknown settings pass through via `extra`.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct GlobalSettings {
    // Numeric settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_key: Option<StringOrSecretRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_period_secs: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_default_timeout: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_size_limit_mb: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_wait_result: Option<i64>,

    // Boolean settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expose_metrics: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expose_debug_metrics: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_job_dir: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_stats: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_preexisting_user_for_oauth: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_instance: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critical_alert_mute_ui: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor_logs_on_s3: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_workspaced_route: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_default_maven: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_tags_per_workspace: Option<bool>,

    // String settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hub_base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hub_accessible_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hub_api_secret: Option<StringOrSecretRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scim_token: Option<StringOrSecretRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saml_metadata: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai_azure_base_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_keep_alive_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_python_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pip_index_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pip_extra_index_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npm_config_registry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bunfig_install_scopes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nuget_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maven_repos: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ruby_repos: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub powershell_repo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub powershell_repo_pat: Option<String>,

    // Array settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_tags_workspaces: Option<Vec<String>>,

    // Structured settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_settings: Option<SmtpSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexer_settings: Option<IndexerSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauths: Option<BTreeMap<String, OAuthClient>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub otel: Option<OtelSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub otel_tracing_proxy: Option<OtelTracingProxySettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "instance_config_schema",
        schemars(schema_with = "opaque_json_schema")
    )]
    pub object_store_cache_config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critical_error_channels: Option<Vec<CriticalErrorChannel>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critical_alerts_on_db_oversize: Option<DbOversizeAlert>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ducklake_settings: Option<DucklakeSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_instance_pg_databases: Option<CustomInstancePgDatabases>,

    // Opaque settings (EE-private structs or no clear schema)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "instance_config_schema",
        schemars(schema_with = "opaque_json_schema")
    )]
    pub secret_backend: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "instance_config_schema",
        schemars(schema_with = "opaque_json_schema")
    )]
    pub slack: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "instance_config_schema",
        schemars(schema_with = "opaque_json_schema")
    )]
    pub teams: Option<serde_json::Value>,

    /// Catch-all for settings not yet covered by typed fields.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl GlobalSettings {
    /// Convert to a flat `BTreeMap` suitable for DB sync.
    pub fn to_settings_map(&self) -> BTreeMap<String, serde_json::Value> {
        // Serialize the whole struct to a JSON object, which flattens all fields
        // including `extra` into a single map. skip_serializing_if ensures
        // None fields are omitted.
        let value = serde_json::to_value(self).expect("GlobalSettings serialization cannot fail");
        match value {
            serde_json::Value::Object(map) => map.into_iter().collect(),
            _ => unreachable!(),
        }
    }
}

// ---------------------------------------------------------------------------
// SMTP
// ---------------------------------------------------------------------------

/// SMTP server configuration.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct SmtpSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_password: Option<StringOrSecretRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_tls_implicit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_disable_tls: Option<bool>,
}

// ---------------------------------------------------------------------------
// Indexer
// ---------------------------------------------------------------------------

/// Full-text search indexer configuration.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct IndexerSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writer_memory_budget: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_job_max_batch_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_log_max_batch_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_index_period: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_log_index_period: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_indexed_job_log_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub should_clear_job_index: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub should_clear_log_index: Option<bool>,
}

// ---------------------------------------------------------------------------
// OAuth
// ---------------------------------------------------------------------------

/// OAuth client configuration for a single provider.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct OAuthClient {
    pub id: String,
    pub secret: StringOrSecretRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_config: Option<OAuthConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_config: Option<OAuthConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share_with_workspaces: Option<bool>,
}

/// OAuth provider endpoint configuration.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct OAuthConfig {
    pub auth_url: String,
    pub token_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub userinfo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_params: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_params_callback: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_body_auth: Option<bool>,
}

// ---------------------------------------------------------------------------
// OpenTelemetry
// ---------------------------------------------------------------------------

/// OpenTelemetry exporter configuration.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct OtelSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracing_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub otel_exporter_otlp_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub otel_exporter_otlp_headers: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub otel_exporter_otlp_protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub otel_exporter_otlp_compression: Option<String>,
}

/// Per-language HTTP request tracing proxy configuration.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct OtelTracingProxySettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled_languages: Vec<ScriptLang>,
}

/// Script language identifier (for instance config use).
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ScriptLang {
    Python3,
    Deno,
    Go,
    Bash,
    Powershell,
    Postgresql,
    Bun,
    Bunnative,
    Mysql,
    Bigquery,
    Snowflake,
    Graphql,
    Nativets,
    Mssql,
    #[serde(rename = "oracledb")]
    OracleDB,
    #[serde(rename = "duckdb")]
    DuckDb,
    Php,
    Rust,
    Ansible,
    #[serde(rename = "csharp")]
    CSharp,
    Nu,
    Java,
    Ruby,
}

// ---------------------------------------------------------------------------
// Critical error channels
// ---------------------------------------------------------------------------

/// A channel for delivering critical error alerts.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum CriticalErrorChannel {
    Email { email: String },
    Slack { slack_channel: String },
    Teams { teams_channel: TeamsChannel },
}

/// Microsoft Teams channel reference.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct TeamsChannel {
    pub team_id: String,
    pub team_name: String,
    pub channel_id: String,
    pub channel_name: String,
}

// ---------------------------------------------------------------------------
// DB oversize alert
// ---------------------------------------------------------------------------

/// Configuration for critical alerts when the database exceeds a size threshold.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct DbOversizeAlert {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub value: f32,
}

// ---------------------------------------------------------------------------
// DuckLake
// ---------------------------------------------------------------------------

/// DuckLake catalog database settings.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct DucklakeSettings {
    pub ducklakes: BTreeMap<String, Ducklake>,
}

/// A single DuckLake instance configuration.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct Ducklake {
    pub catalog: DucklakeCatalog,
    pub storage: DucklakeStorage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<String>,
}

/// DuckLake catalog backend reference.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct DucklakeCatalog {
    pub resource_type: DucklakeCatalogResourceType,
    pub resource_path: String,
}

/// DuckLake storage location.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct DucklakeStorage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,
    pub path: String,
}

/// The type of database backing a DuckLake catalog.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DucklakeCatalogResourceType {
    Postgresql,
    Mysql,
    Instance,
}

// ---------------------------------------------------------------------------
// Custom instance PG databases
// ---------------------------------------------------------------------------

/// Custom PostgreSQL databases managed by the instance.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct CustomInstancePgDatabases {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_pwd: Option<StringOrSecretRef>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub databases: BTreeMap<String, CustomInstanceDb>,
}

/// Status of a single custom instance database.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct CustomInstanceDb {
    #[serde(default)]
    pub logs: CustomInstanceDbLogs,
    #[serde(default)]
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

/// Setup log entries for a custom instance database.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct CustomInstanceDbLogs {
    pub super_admin: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub database_credentials: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub valid_dbname: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub created_database: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub db_connect: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub grant_permissions: String,
}

// ---------------------------------------------------------------------------
// Autoscaling (worker config)
// ---------------------------------------------------------------------------

/// Worker group autoscaling configuration.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct AutoscalingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_workers: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_workers: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inc_scale_num_jobs_waiting: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_scale_cooldown_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_scale_jobs_waiting: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dec_scale_occupancy_rate: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inc_scale_occupancy_rate: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inc_num_workers: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration: Option<AutoscalingIntegration>,
}

/// Autoscaling integration backend.
///
/// The `type` field selects the backend: `"script"`, `"dryrun"`, or `"kubernetes"`.
/// For `"script"`, `path` is required and `tag` is optional.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct AutoscalingIntegration {
    #[serde(rename = "type")]
    pub integration_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

// ---------------------------------------------------------------------------
// Worker group config
// ---------------------------------------------------------------------------

/// Worker group configuration.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct WorkerGroupConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_tags: Option<BTreeMap<String, u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedicated_worker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedicated_workers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init_bash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub periodic_script_bash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub periodic_script_interval_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_python_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pip_local_dependencies: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_vars_static: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_vars_allowlist: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_alive_workers_alert_threshold: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autoscaling: Option<AutoscalingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_mode: Option<bool>,

    /// Catch-all for fields not yet covered by typed fields.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Diff + apply logic
// ---------------------------------------------------------------------------

/// Controls whether absent keys are deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyMode {
    /// UI: only upsert changed keys, never delete absent ones.
    Merge,
    /// Operator: upsert changed + delete absent (except protected).
    Replace,
}

/// The diff result for global settings.
#[derive(Debug, Default)]
pub struct SettingsDiff {
    pub upserts: BTreeMap<String, serde_json::Value>,
    pub deletes: Vec<String>,
    /// Previous values for keys being updated or deleted (for logging).
    pub previous_values: BTreeMap<String, serde_json::Value>,
    /// Count of desired keys that matched the current value exactly.
    pub unchanged_count: usize,
}

/// The diff result for worker configs.
#[derive(Debug, Default)]
pub struct ConfigsDiff {
    pub upserts: BTreeMap<String, serde_json::Value>,
    pub deletes: Vec<String>,
}

/// Settings that must never be deleted by the operator or bulk API.
pub const PROTECTED_SETTINGS: &[&str] = &[
    "ducklake_user_pg_pwd",
    "ducklake_settings",
    "custom_instance_pg_databases",
    "uid",
    "rsa_keys",
    "jwt_secret",
    "min_keep_alive_version",
];

/// Internal settings that are never exposed via the API or included in config exports.
/// Note: jwt_secret is intentionally NOT hidden — it is included in YAML exports so that
/// operators can set it via ConfigMap. It is protected from deletion (PROTECTED_SETTINGS)
/// and from being set to empty/null, and its value is partially redacted in log output.
pub const HIDDEN_SETTINGS: &[&str] = &["uid", "rsa_keys", "min_keep_alive_version"];

/// Top-level settings whose entire value is sensitive and must be fully redacted in logs.
const SENSITIVE_SETTINGS: &[&str] = &[
    "jwt_secret",
    "scim_token",
    "hub_api_secret",
    "license_key",
    "pip_index_url",
    "pip_extra_index_url",
    "npm_config_registry",
    "bunfig_install_scopes",
    "maven_repos",
    "ruby_repos",
    "powershell_repo_pat",
];

/// Object-valued settings that contain sensitive sub-fields.
/// Maps a top-level key to the sub-field names that must be redacted.
const NESTED_SENSITIVE_FIELDS: &[(&str, &[&str])] = &[
    ("smtp_settings", &["smtp_password"]),
    ("secret_backend", &["token"]),
    ("object_store_cache_config", &["secret_key", "serviceAccountKey"]),
];

fn redact_json_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => serde_json::json!(redact_string(s)),
        _ => serde_json::json!("[redacted]"),
    }
}

fn mask_nested_sensitive(key: &str, value: &serde_json::Value) -> serde_json::Value {
    for &(parent_key, sub_fields) in NESTED_SENSITIVE_FIELDS {
        if key == parent_key {
            if let serde_json::Value::Object(map) = value {
                let mut masked = map.clone();
                for &field in sub_fields {
                    if let Some(v) = masked.get(field) {
                        masked.insert(field.to_string(), redact_json_value(v));
                    }
                }
                return serde_json::Value::Object(masked);
            }
        }
    }
    // Settings that are maps-of-objects where each child has a sensitive sub-field.
    const NESTED_MAP_SENSITIVE: &[(&str, &str)] = &[
        ("oauths", "secret"),
        ("custom_instance_pg_databases", "user_pwd"),
    ];
    for &(parent_key, child_field) in NESTED_MAP_SENSITIVE {
        if key == parent_key {
            if let serde_json::Value::Object(entries) = value {
                let mut masked = entries.clone();
                for (_entry_key, entry_val) in masked.iter_mut() {
                    if let serde_json::Value::Object(ref mut obj) = entry_val {
                        if let Some(v) = obj.get(child_field) {
                            obj.insert(child_field.to_string(), redact_json_value(v));
                        }
                    }
                }
                return serde_json::Value::Object(masked);
            }
        }
    }
    value.clone()
}

fn redact_string(s: &str) -> String {
    let char_count = s.chars().count();
    if char_count <= 6 {
        "****".to_string()
    } else {
        let show = (char_count / 4).min(4);
        let prefix: String = s.chars().take(show).collect();
        let suffix: String = s.chars().skip(char_count - show).collect();
        format!("{prefix}****{suffix}")
    }
}

fn format_setting_value(key: &str, value: &serde_json::Value) -> String {
    if SENSITIVE_SETTINGS.contains(&key) {
        return match value {
            serde_json::Value::String(s) => format!("\"{}\"", redact_string(s)),
            _ => "[redacted]".to_string(),
        };
    }
    let value = mask_nested_sensitive(key, value);
    let s = value.to_string();
    if s.len() > 200 {
        format!("{}...", &s[..197])
    } else {
        s
    }
}

/// Extract the expiry timestamp from a license key JSON value.
///
/// License keys have the format `<client_id>.<expiry>.<signature>`.
/// Returns `None` if the value is not a string or doesn't match the format.
fn license_key_expiry(value: &serde_json::Value) -> Option<u64> {
    let s = value.as_str()?;
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    parts[1].parse::<u64>().ok()
}

/// Returns true if two license key values share the same client ID and signature
/// (i.e. they differ only in the expiry field).
fn license_keys_same_except_expiry(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    let (Some(a_str), Some(b_str)) = (a.as_str(), b.as_str()) else {
        return false;
    };
    let a_parts: Vec<&str> = a_str.split('.').collect();
    let b_parts: Vec<&str> = b_str.split('.').collect();
    if a_parts.len() != 3 || b_parts.len() != 3 {
        return false;
    }
    a_parts[0] == b_parts[0] && a_parts[2] == b_parts[2]
}

fn is_empty_or_null(value: &serde_json::Value) -> bool {
    value.is_null() || value.as_str().map_or(false, |s| s.is_empty())
}

/// Maximum retention period in seconds for CE builds (30 days).
pub const CE_MAX_RETENTION_PERIOD_SECS: i64 = 30 * 24 * 3600;

/// Clamp `retention_period_secs` to `CE_MAX_RETENTION_PERIOD_SECS` on CE builds.
/// Returns the (possibly clamped) value. On EE builds this is a no-op.
pub fn clamp_retention_period(value: serde_json::Value) -> serde_json::Value {
    #[cfg(feature = "enterprise")]
    {
        value
    }
    #[cfg(not(feature = "enterprise"))]
    {
        if let Some(secs) = value.as_i64() {
            if secs > CE_MAX_RETENTION_PERIOD_SECS {
                tracing::warn!(
                    "Clamping retention_period_secs from {} to {} (CE max: 30 days)",
                    secs,
                    CE_MAX_RETENTION_PERIOD_SECS
                );
                return serde_json::json!(CE_MAX_RETENTION_PERIOD_SECS);
            }
        }
        value
    }
}

/// Compute the diff between current and desired global settings.
pub fn diff_global_settings(
    current: &BTreeMap<String, serde_json::Value>,
    desired: &BTreeMap<String, serde_json::Value>,
    mode: ApplyMode,
) -> SettingsDiff {
    let mut upserts = BTreeMap::new();
    let mut previous_values = BTreeMap::new();
    let mut unchanged_count: usize = 0;
    for (key, desired_value) in desired {
        if key == "jwt_secret" && is_empty_or_null(desired_value) {
            tracing::warn!(
                "Skipping jwt_secret update: value must not be empty or null"
            );
            continue;
        }
        let value = if key == "retention_period_secs" {
            clamp_retention_period(desired_value.clone())
        } else {
            desired_value.clone()
        };
        match current.get(key) {
            Some(existing) if *existing == value => {
                unchanged_count += 1;
            }
            Some(existing) if key == LICENSE_KEY_SETTING => {
                if license_keys_same_except_expiry(existing, &value) {
                    let current_expiry = license_key_expiry(existing).unwrap_or(0);
                    let desired_expiry = license_key_expiry(&value).unwrap_or(0);
                    if desired_expiry > current_expiry {
                        previous_values.insert(key.clone(), existing.clone());
                        upserts.insert(key.clone(), value);
                    } else {
                        tracing::info!(
                            "Skipping license_key update: desired expiry ({}) is not posterior to current expiry ({})",
                            desired_expiry,
                            current_expiry
                        );
                        unchanged_count += 1;
                    }
                } else {
                    previous_values.insert(key.clone(), existing.clone());
                    upserts.insert(key.clone(), value);
                }
            }
            Some(existing) => {
                previous_values.insert(key.clone(), existing.clone());
                upserts.insert(key.clone(), value);
            }
            None => {
                upserts.insert(key.clone(), value);
            }
        }
    }
    let mut deletes = Vec::new();
    if matches!(mode, ApplyMode::Replace) {
        for key in current.keys() {
            if !desired.contains_key(key) && !PROTECTED_SETTINGS.contains(&key.as_str()) {
                previous_values.insert(key.clone(), current[key].clone());
                deletes.push(key.clone());
            }
        }
    }
    SettingsDiff { upserts, deletes, previous_values, unchanged_count }
}

/// Compute the diff between current and desired worker configs.
pub fn diff_worker_configs(
    current: &BTreeMap<String, serde_json::Value>,
    desired: &BTreeMap<String, serde_json::Value>,
    mode: ApplyMode,
) -> ConfigsDiff {
    let mut upserts = BTreeMap::new();
    for (key, value) in desired {
        match current.get(key) {
            Some(existing) if existing == value => {} // no change
            _ => {
                upserts.insert(key.clone(), value.clone());
            }
        }
    }
    let mut deletes = Vec::new();
    if matches!(mode, ApplyMode::Replace) {
        for key in current.keys() {
            if !desired.contains_key(key) {
                deletes.push(key.clone());
            }
        }
    }
    ConfigsDiff { upserts, deletes }
}

/// Apply a settings diff to the global_settings table.
pub async fn apply_settings_diff(
    db: &sqlx::Pool<sqlx::Postgres>,
    diff: &SettingsDiff,
) -> anyhow::Result<()> {
    for (key, value) in &diff.upserts {
        sqlx::query(
            "INSERT INTO global_settings (name, value) VALUES ($1, $2) \
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
        )
        .bind(key)
        .bind(value)
        .execute(db)
        .await?;
        if let Some(old_value) = diff.previous_values.get(key) {
            tracing::info!(
                "Updated global setting: {key} ({} -> {})",
                format_setting_value(key, old_value),
                format_setting_value(key, value)
            );
        } else {
            tracing::info!(
                "Created global setting: {key} (value: {})",
                format_setting_value(key, value)
            );
        }
    }
    for key in &diff.deletes {
        sqlx::query("DELETE FROM global_settings WHERE name = $1")
            .bind(key)
            .execute(db)
            .await?;
        if let Some(old_value) = diff.previous_values.get(key) {
            tracing::info!(
                "Deleted global setting: {key} (was: {})",
                format_setting_value(key, old_value)
            );
        } else {
            tracing::info!("Deleted global setting: {key}");
        }
    }
    if diff.unchanged_count > 0 {
        tracing::info!("{} global setting(s) unchanged", diff.unchanged_count);
    }
    Ok(())
}

/// Apply a configs diff to the config table (worker__ prefix).
pub async fn apply_configs_diff(
    db: &sqlx::Pool<sqlx::Postgres>,
    diff: &ConfigsDiff,
) -> anyhow::Result<()> {
    for (group_name, config_value) in &diff.upserts {
        let db_key = format!("worker__{group_name}");
        sqlx::query(
            "INSERT INTO config (name, config) VALUES ($1, $2) \
             ON CONFLICT (name) DO UPDATE SET config = EXCLUDED.config",
        )
        .bind(&db_key)
        .bind(config_value)
        .execute(db)
        .await?;
        tracing::info!("Synced worker config: {db_key}");
    }
    for group_name in &diff.deletes {
        let db_key = format!("worker__{group_name}");
        sqlx::query("DELETE FROM config WHERE name = $1")
            .bind(&db_key)
            .execute(db)
            .await?;
        tracing::info!("Deleted worker config: {db_key}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Env ref resolution
// ---------------------------------------------------------------------------

fn resolve_env_field(field: &mut StringOrSecretRef) -> Result<(), String> {
    if let Some(var_name) = field.as_env_ref() {
        let var_name = var_name.to_string();
        let value = std::env::var(&var_name).map_err(|_| var_name)?;
        *field = StringOrSecretRef::Literal(value);
    }
    Ok(())
}

fn resolve_env_option(field: &mut Option<StringOrSecretRef>) -> Result<(), String> {
    if let Some(val) = field {
        resolve_env_field(val)?;
    }
    Ok(())
}

/// Resolve all `EnvRef` fields in `GlobalSettings` by reading the
/// corresponding environment variables. Each resolved `EnvRef` is replaced
/// with a `Literal`. Returns `Err(var_name)` if a referenced env var is not set.
pub fn resolve_env_refs(settings: &mut GlobalSettings) -> Result<(), String> {
    resolve_env_option(&mut settings.license_key)?;
    resolve_env_option(&mut settings.hub_api_secret)?;
    resolve_env_option(&mut settings.scim_token)?;

    if let Some(smtp) = &mut settings.smtp_settings {
        resolve_env_option(&mut smtp.smtp_password)?;
    }

    if let Some(oauths) = &mut settings.oauths {
        for oauth in oauths.values_mut() {
            resolve_env_field(&mut oauth.secret)?;
        }
    }

    if let Some(pg) = &mut settings.custom_instance_pg_databases {
        resolve_env_option(&mut pg.user_pwd)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Read from DB
// ---------------------------------------------------------------------------

impl InstanceConfig {
    /// Read the full instance configuration from the database.
    pub async fn from_db(db: &sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<Self> {
        // Read global_settings table → flat map → deserialize into GlobalSettings
        let rows: Vec<(String, serde_json::Value)> =
            sqlx::query_as("SELECT name, value FROM global_settings")
                .fetch_all(db)
                .await?;
        let map: serde_json::Map<String, serde_json::Value> = rows
            .into_iter()
            .filter(|(name, _)| !HIDDEN_SETTINGS.contains(&name.as_str()))
            .collect();
        let global_settings: GlobalSettings =
            serde_json::from_value(serde_json::Value::Object(map))?;

        // Read config table (worker__ prefix) → BTreeMap<String, WorkerGroupConfig>
        let config_rows: Vec<(String, serde_json::Value)> =
            sqlx::query_as("SELECT name, config FROM config WHERE name LIKE 'worker__%'")
                .fetch_all(db)
                .await?;
        let worker_configs = config_rows
            .into_iter()
            .map(|(name, config)| {
                let group = name.strip_prefix("worker__").unwrap_or(&name).to_string();
                let wc: WorkerGroupConfig = serde_json::from_value(config).unwrap_or_default();
                (group, wc)
            })
            .collect();

        Ok(Self { global_settings, worker_configs })
    }

    /// Sync this config to the database using `ApplyMode::Replace`.
    ///
    /// Reads the current state, computes a diff, then applies upserts and
    /// deletes for both global settings and worker configs.
    pub async fn sync_to_db(&self, db: &sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
        let desired_settings = self.global_settings.to_settings_map();

        // Current global settings
        let rows: Vec<(String, serde_json::Value)> =
            sqlx::query_as("SELECT name, value FROM global_settings")
                .fetch_all(db)
                .await?;
        let current_settings: BTreeMap<String, serde_json::Value> = rows.into_iter().collect();

        let settings_diff =
            diff_global_settings(&current_settings, &desired_settings, ApplyMode::Replace);
        apply_settings_diff(db, &settings_diff).await?;

        // Worker configs
        let desired_configs: BTreeMap<String, serde_json::Value> = self
            .worker_configs
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    serde_json::to_value(v).expect("WorkerGroupConfig serialization cannot fail"),
                )
            })
            .collect();

        let config_rows: Vec<(String, serde_json::Value)> =
            sqlx::query_as("SELECT name, config FROM config WHERE name LIKE 'worker__%'")
                .fetch_all(db)
                .await?;
        let current_configs: BTreeMap<String, serde_json::Value> = config_rows
            .into_iter()
            .map(|(name, config)| {
                let group = name.strip_prefix("worker__").unwrap_or(&name).to_string();
                (group, config)
            })
            .collect();

        let configs_diff =
            diff_worker_configs(&current_configs, &desired_configs, ApplyMode::Replace);
        apply_configs_diff(db, &configs_diff).await?;

        Ok(())
    }

    /// Serialize to YAML with sorted global_settings keys and worker_configs
    /// ordered with "default" and "native" first.
    pub fn to_sorted_yaml(&self) -> Result<String, String> {
        let settings_map = self.global_settings.to_settings_map();

        let mut yaml = String::from("global_settings:\n");
        for (key, value) in &settings_map {
            let value_yaml = serde_yml::to_string(value)
                .map_err(|e| format!("YAML serialization failed for {key}: {e}"))?;
            write_yaml_field(&mut yaml, key, value, &value_yaml, 1);
        }

        if !self.worker_configs.is_empty() {
            yaml.push_str("worker_configs:\n");
            let priority_keys = ["default", "native"];
            for &pk in &priority_keys {
                if let Some(wc) = self.worker_configs.get(pk) {
                    let wc_value = serde_json::to_value(wc)
                        .map_err(|e| format!("JSON serialization failed for {pk}: {e}"))?;
                    let wc_yaml = serde_yml::to_string(&wc_value)
                        .map_err(|e| format!("YAML serialization failed for {pk}: {e}"))?;
                    write_yaml_field(&mut yaml, pk, &wc_value, &wc_yaml, 1);
                }
            }
            for (key, wc) in &self.worker_configs {
                if priority_keys.contains(&key.as_str()) {
                    continue;
                }
                let wc_value = serde_json::to_value(wc)
                    .map_err(|e| format!("JSON serialization failed for {key}: {e}"))?;
                let wc_yaml = serde_yml::to_string(&wc_value)
                    .map_err(|e| format!("YAML serialization failed for {key}: {e}"))?;
                write_yaml_field(&mut yaml, key, &wc_value, &wc_yaml, 1);
            }
        }

        Ok(yaml)
    }
}

fn write_yaml_field(
    yaml: &mut String,
    key: &str,
    value: &serde_json::Value,
    value_yaml: &str,
    indent: usize,
) {
    use std::fmt::Write;

    let prefix = "  ".repeat(indent);
    let trimmed = value_yaml.trim();
    let is_nested = matches!(value, serde_json::Value::Object(_) | serde_json::Value::Array(_));
    if is_nested {
        let _ = writeln!(yaml, "{prefix}{key}:");
        let inner_prefix = "  ".repeat(indent + 1);
        for line in trimmed.lines() {
            let _ = writeln!(yaml, "{inner_prefix}{line}");
        }
    } else {
        let _ = writeln!(yaml, "{prefix}{key}: {trimmed}");
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_settings_to_map() {
        let settings = GlobalSettings {
            base_url: Some("https://example.com".to_string()),
            retention_period_secs: Some(86400),
            expose_metrics: Some(true),
            smtp_settings: Some(SmtpSettings {
                smtp_host: Some("mail.example.com".to_string()),
                smtp_port: Some(587),
                ..Default::default()
            }),
            ..Default::default()
        };
        let map = settings.to_settings_map();
        assert_eq!(map["base_url"], serde_json::json!("https://example.com"));
        assert_eq!(map["retention_period_secs"], serde_json::json!(86400));
        assert_eq!(map["expose_metrics"], serde_json::json!(true));
        assert_eq!(
            map["smtp_settings"],
            serde_json::json!({"smtp_host": "mail.example.com", "smtp_port": 587})
        );
        assert!(
            !map.contains_key("license_key"),
            "None fields should be omitted"
        );
    }

    #[test]
    fn global_settings_extra_fields_passthrough() {
        let json = r#"{
            "base_url": "https://example.com",
            "some_future_setting": {"nested": true}
        }"#;
        let settings: GlobalSettings =
            serde_json::from_str(json).expect("Should deserialize with unknown fields");
        assert_eq!(settings.base_url.as_deref(), Some("https://example.com"));
        assert_eq!(
            settings.extra["some_future_setting"],
            serde_json::json!({"nested": true})
        );

        let map = settings.to_settings_map();
        assert!(map.contains_key("some_future_setting"));
    }

    #[test]
    fn worker_config_extra_fields_passthrough() {
        let json = r#"{
            "init_bash": "echo hello",
            "some_future_field": 42
        }"#;
        let config: WorkerGroupConfig =
            serde_json::from_str(json).expect("Should deserialize with unknown fields");
        assert_eq!(config.init_bash.as_deref(), Some("echo hello"));
        assert_eq!(config.extra["some_future_field"], serde_json::json!(42));
    }

    #[test]
    fn instance_config_deserializes_full_example() {
        let json = r##"{
            "global_settings": {
                "base_url": "https://windmill.example.com",
                "license_key": "my-key",
                "retention_period_secs": 2592000,
                "smtp_settings": {"smtp_host": "smtp.example.com", "smtp_port": 587},
                "otel": {
                    "metrics_enabled": true,
                    "otel_exporter_otlp_endpoint": "http://otel:4317"
                },
                "critical_error_channels": [
                    {"email": "admin@example.com"},
                    {"slack_channel": "#alerts"}
                ],
                "custom_tags": ["gpu", "high-mem"]
            },
            "worker_configs": {
                "default": {"init_bash": "echo hello"},
                "gpu": {
                    "dedicated_worker": "ws:f/gpu",
                    "autoscaling": {
                        "enabled": true,
                        "min_workers": 1,
                        "max_workers": 10,
                        "integration": {"type": "kubernetes"}
                    }
                }
            }
        }"##;
        let config: InstanceConfig =
            serde_json::from_str(json).expect("Should deserialize full config");
        assert_eq!(
            config.global_settings.base_url.as_deref(),
            Some("https://windmill.example.com")
        );
        assert_eq!(config.global_settings.retention_period_secs, Some(2592000));
        assert!(config.global_settings.smtp_settings.is_some());
        let smtp = config.global_settings.smtp_settings.as_ref().unwrap();
        assert_eq!(smtp.smtp_host.as_deref(), Some("smtp.example.com"));
        assert_eq!(smtp.smtp_port, Some(587));

        let otel = config.global_settings.otel.as_ref().unwrap();
        assert_eq!(otel.metrics_enabled, Some(true));
        assert_eq!(
            otel.otel_exporter_otlp_endpoint.as_deref(),
            Some("http://otel:4317")
        );

        let channels = config
            .global_settings
            .critical_error_channels
            .as_ref()
            .unwrap();
        assert_eq!(channels.len(), 2);

        let tags = config.global_settings.custom_tags.as_ref().unwrap();
        assert_eq!(tags, &["gpu", "high-mem"]);

        assert_eq!(config.worker_configs.len(), 2);
        assert_eq!(
            config.worker_configs["default"].init_bash.as_deref(),
            Some("echo hello")
        );
        assert_eq!(
            config.worker_configs["gpu"].dedicated_worker.as_deref(),
            Some("ws:f/gpu")
        );
        let autoscaling = config.worker_configs["gpu"].autoscaling.as_ref().unwrap();
        assert!(autoscaling.enabled);
        assert_eq!(autoscaling.min_workers, Some(1));
        assert_eq!(autoscaling.max_workers, Some(10));
        let integration = autoscaling.integration.as_ref().unwrap();
        assert_eq!(integration.integration_type, "kubernetes");
    }

    #[test]
    fn instance_config_roundtrips() {
        let config = InstanceConfig {
            global_settings: GlobalSettings {
                base_url: Some("https://example.com".to_string()),
                expose_metrics: Some(true),
                ..Default::default()
            },
            worker_configs: {
                let mut m = BTreeMap::new();
                m.insert(
                    "default".to_string(),
                    WorkerGroupConfig {
                        init_bash: Some("echo hello".to_string()),
                        ..Default::default()
                    },
                );
                m
            },
        };

        let json = serde_json::to_string(&config).expect("Should serialize");
        let deserialized: InstanceConfig = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(
            config.global_settings.base_url,
            deserialized.global_settings.base_url
        );
        assert_eq!(
            config.global_settings.expose_metrics,
            deserialized.global_settings.expose_metrics
        );
        assert_eq!(
            config.worker_configs["default"].init_bash,
            deserialized.worker_configs["default"].init_bash
        );
    }

    #[test]
    fn autoscaling_script_integration_roundtrips() {
        let json = r#"{
            "enabled": true,
            "min_workers": 2,
            "max_workers": 8,
            "integration": {"type": "script", "path": "f/scale", "tag": "admin"}
        }"#;
        let config: AutoscalingConfig =
            serde_json::from_str(json).expect("Should deserialize autoscaling config");
        assert!(config.enabled);
        let integration = config.integration.as_ref().unwrap();
        assert_eq!(integration.integration_type, "script");
        assert_eq!(integration.path.as_deref(), Some("f/scale"));
        assert_eq!(integration.tag.as_deref(), Some("admin"));
    }

    #[test]
    fn ducklake_settings_roundtrips() {
        let json = r#"{
            "ducklakes": {
                "main": {
                    "catalog": {"resource_type": "postgresql", "resource_path": "u/admin/pg"},
                    "storage": {"path": "/data/ducklake"}
                }
            }
        }"#;
        let settings: DucklakeSettings =
            serde_json::from_str(json).expect("Should deserialize ducklake settings");
        assert!(settings.ducklakes.contains_key("main"));
        assert_eq!(
            settings.ducklakes["main"].catalog.resource_type,
            DucklakeCatalogResourceType::Postgresql
        );
    }

    #[test]
    fn critical_error_channels_roundtrips() {
        let json = r##"[
            {"email": "admin@example.com"},
            {"slack_channel": "#alerts"},
            {"teams_channel": {"team_id": "t1", "team_name": "T1", "channel_id": "c1", "channel_name": "C1"}}
        ]"##;
        let channels: Vec<CriticalErrorChannel> =
            serde_json::from_str(json).expect("Should deserialize channels");
        assert_eq!(channels.len(), 3);
        assert!(matches!(channels[0], CriticalErrorChannel::Email { .. }));
        assert!(matches!(channels[1], CriticalErrorChannel::Slack { .. }));
        assert!(matches!(channels[2], CriticalErrorChannel::Teams { .. }));
    }

    // -----------------------------------------------------------------------
    // Diff tests
    // -----------------------------------------------------------------------

    #[test]
    fn diff_global_settings_detects_changes() {
        let mut current = BTreeMap::new();
        current.insert("a".to_string(), serde_json::json!("old"));
        current.insert("b".to_string(), serde_json::json!(1));

        let mut desired = BTreeMap::new();
        desired.insert("a".to_string(), serde_json::json!("new"));
        desired.insert("b".to_string(), serde_json::json!(1)); // unchanged
        desired.insert("c".to_string(), serde_json::json!(true)); // new

        let diff = diff_global_settings(&current, &desired, ApplyMode::Merge);
        assert_eq!(diff.upserts.len(), 2); // a (changed) + c (new)
        assert!(diff.upserts.contains_key("a"));
        assert!(diff.upserts.contains_key("c"));
        assert!(!diff.upserts.contains_key("b"));
        assert!(diff.deletes.is_empty(), "Merge mode should not delete");
    }

    #[test]
    fn diff_global_settings_replace_mode_deletes() {
        let mut current = BTreeMap::new();
        current.insert("keep".to_string(), serde_json::json!("v"));
        current.insert("remove".to_string(), serde_json::json!("v"));
        current.insert(
            "ducklake_settings".to_string(),
            serde_json::json!("protected"),
        );

        let mut desired = BTreeMap::new();
        desired.insert("keep".to_string(), serde_json::json!("v"));

        let diff = diff_global_settings(&current, &desired, ApplyMode::Replace);
        assert!(diff.upserts.is_empty(), "No changes to upsert");
        assert_eq!(diff.deletes, vec!["remove".to_string()]);
        assert!(
            !diff.deletes.contains(&"ducklake_settings".to_string()),
            "Protected settings should not be deleted"
        );
    }

    #[test]
    fn diff_worker_configs_merge() {
        let mut current = BTreeMap::new();
        current.insert("default".to_string(), serde_json::json!({"a": 1}));
        current.insert("old_group".to_string(), serde_json::json!({"b": 2}));

        let mut desired = BTreeMap::new();
        desired.insert("default".to_string(), serde_json::json!({"a": 1})); // unchanged
        desired.insert("new_group".to_string(), serde_json::json!({"c": 3}));

        let diff = diff_worker_configs(&current, &desired, ApplyMode::Merge);
        assert_eq!(diff.upserts.len(), 1);
        assert!(diff.upserts.contains_key("new_group"));
        assert!(diff.deletes.is_empty());
    }

    #[test]
    fn diff_worker_configs_replace() {
        let mut current = BTreeMap::new();
        current.insert("default".to_string(), serde_json::json!({"a": 1}));
        current.insert("old_group".to_string(), serde_json::json!({"b": 2}));

        let mut desired = BTreeMap::new();
        desired.insert("default".to_string(), serde_json::json!({"a": 1}));

        let diff = diff_worker_configs(&current, &desired, ApplyMode::Replace);
        assert!(diff.upserts.is_empty());
        assert_eq!(diff.deletes, vec!["old_group".to_string()]);
    }

    // -----------------------------------------------------------------------
    // Diff edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn diff_global_settings_both_empty() {
        let current = BTreeMap::new();
        let desired = BTreeMap::new();

        let diff_merge = diff_global_settings(&current, &desired, ApplyMode::Merge);
        assert!(diff_merge.upserts.is_empty());
        assert!(diff_merge.deletes.is_empty());

        let diff_replace = diff_global_settings(&current, &desired, ApplyMode::Replace);
        assert!(diff_replace.upserts.is_empty());
        assert!(diff_replace.deletes.is_empty());
    }

    #[test]
    fn diff_global_settings_empty_current() {
        let current = BTreeMap::new();
        let mut desired = BTreeMap::new();
        desired.insert("a".to_string(), serde_json::json!(1));
        desired.insert("b".to_string(), serde_json::json!("two"));

        let diff = diff_global_settings(&current, &desired, ApplyMode::Merge);
        assert_eq!(diff.upserts.len(), 2);
        assert!(diff.deletes.is_empty());
    }

    #[test]
    fn diff_global_settings_identical() {
        let mut current = BTreeMap::new();
        current.insert("a".to_string(), serde_json::json!(1));
        current.insert("b".to_string(), serde_json::json!({"nested": true}));

        let desired = current.clone();

        let diff_merge = diff_global_settings(&current, &desired, ApplyMode::Merge);
        assert!(diff_merge.upserts.is_empty());
        assert!(diff_merge.deletes.is_empty());

        let diff_replace = diff_global_settings(&current, &desired, ApplyMode::Replace);
        assert!(diff_replace.upserts.is_empty());
        assert!(diff_replace.deletes.is_empty());
    }

    #[test]
    fn diff_global_settings_replace_protects_all() {
        let mut current = BTreeMap::new();
        for key in PROTECTED_SETTINGS {
            current.insert(key.to_string(), serde_json::json!("val"));
        }
        current.insert("unprotected".to_string(), serde_json::json!("val"));

        let desired = BTreeMap::new();
        let diff = diff_global_settings(&current, &desired, ApplyMode::Replace);
        assert_eq!(diff.deletes, vec!["unprotected".to_string()]);
        for key in PROTECTED_SETTINGS {
            assert!(
                !diff.deletes.contains(&key.to_string()),
                "Protected key {key} should not be in deletes"
            );
        }
    }

    #[test]
    fn diff_global_settings_merge_never_deletes() {
        let mut current = BTreeMap::new();
        current.insert("a".to_string(), serde_json::json!(1));
        current.insert("b".to_string(), serde_json::json!(2));
        current.insert("c".to_string(), serde_json::json!(3));

        let mut desired = BTreeMap::new();
        desired.insert("a".to_string(), serde_json::json!(1));
        // b and c are absent from desired

        let diff = diff_global_settings(&current, &desired, ApplyMode::Merge);
        assert!(
            diff.deletes.is_empty(),
            "Merge mode should never produce deletes regardless of absent keys"
        );
    }

    #[test]
    fn diff_worker_configs_both_empty() {
        let current = BTreeMap::new();
        let desired = BTreeMap::new();

        let diff = diff_worker_configs(&current, &desired, ApplyMode::Replace);
        assert!(diff.upserts.is_empty());
        assert!(diff.deletes.is_empty());
    }

    #[test]
    fn diff_worker_configs_empty_current() {
        let current = BTreeMap::new();
        let mut desired = BTreeMap::new();
        desired.insert("default".to_string(), serde_json::json!({"init": "bash"}));

        let diff = diff_worker_configs(&current, &desired, ApplyMode::Replace);
        assert_eq!(diff.upserts.len(), 1);
        assert!(diff.deletes.is_empty());
    }

    #[test]
    fn diff_worker_configs_identical() {
        let mut data = BTreeMap::new();
        data.insert("g1".to_string(), serde_json::json!({"a": 1}));

        let diff = diff_worker_configs(&data, &data.clone(), ApplyMode::Replace);
        assert!(diff.upserts.is_empty());
        assert!(diff.deletes.is_empty());
    }

    // -----------------------------------------------------------------------
    // to_settings_map edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn to_settings_map_empty_defaults() {
        let settings = GlobalSettings::default();
        let map = settings.to_settings_map();
        assert!(
            map.is_empty(),
            "Default GlobalSettings should produce an empty map"
        );
    }

    #[test]
    fn to_settings_map_preserves_extra_and_typed_fields() {
        let mut settings = GlobalSettings {
            base_url: Some("https://example.com".to_string()),
            expose_metrics: Some(true),
            ..Default::default()
        };
        settings
            .extra
            .insert("custom_key".to_string(), serde_json::json!("custom_val"));

        let map = settings.to_settings_map();
        assert_eq!(map["base_url"], serde_json::json!("https://example.com"));
        assert_eq!(map["expose_metrics"], serde_json::json!(true));
        assert_eq!(map["custom_key"], serde_json::json!("custom_val"));
    }

    #[test]
    fn to_settings_map_roundtrip_through_deserialization() {
        let original = GlobalSettings {
            base_url: Some("https://windmill.dev".to_string()),
            retention_period_secs: Some(86400),
            expose_metrics: Some(false),
            smtp_settings: Some(SmtpSettings {
                smtp_host: Some("mail.example.com".to_string()),
                smtp_port: Some(465),
                smtp_tls_implicit: Some(true),
                ..Default::default()
            }),
            custom_tags: Some(vec!["gpu".to_string(), "mem".to_string()]),
            ..Default::default()
        };

        let map = original.to_settings_map();
        let json_map: serde_json::Map<String, serde_json::Value> = map.into_iter().collect();
        let reconstructed: GlobalSettings =
            serde_json::from_value(serde_json::Value::Object(json_map))
                .expect("Should deserialize from settings map");

        assert_eq!(original.base_url, reconstructed.base_url);
        assert_eq!(
            original.retention_period_secs,
            reconstructed.retention_period_secs
        );
        assert_eq!(original.expose_metrics, reconstructed.expose_metrics);
        assert_eq!(
            original.smtp_settings.as_ref().unwrap().smtp_host,
            reconstructed.smtp_settings.as_ref().unwrap().smtp_host
        );
        assert_eq!(
            original.smtp_settings.as_ref().unwrap().smtp_port,
            reconstructed.smtp_settings.as_ref().unwrap().smtp_port
        );
        assert_eq!(original.custom_tags, reconstructed.custom_tags);
    }

    // -----------------------------------------------------------------------
    // Serialization edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn global_settings_null_json_value_in_extra() {
        let json = r#"{"unknown_null_key": null, "base_url": "https://x.com"}"#;
        let settings: GlobalSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.base_url.as_deref(), Some("https://x.com"));
        assert_eq!(settings.extra["unknown_null_key"], serde_json::Value::Null);

        let map = settings.to_settings_map();
        assert_eq!(map["unknown_null_key"], serde_json::Value::Null);
    }

    #[test]
    fn worker_config_all_typed_fields() {
        let json = r#"{
            "worker_tags": ["tag1", "tag2"],
            "priority_tags": {"tag1": 5},
            "dedicated_worker": "ws:f/my_script",
            "dedicated_workers": ["ws:f/a", "ws:f/b"],
            "init_bash": "apt-get install -y curl",
            "periodic_script_bash": "echo ping",
            "periodic_script_interval_seconds": 300,
            "cache_clear": 7,
            "additional_python_paths": ["/opt/py"],
            "pip_local_dependencies": ["requests"],
            "env_vars_static": {"FOO": "bar"},
            "env_vars_allowlist": ["PATH"],
            "min_alive_workers_alert_threshold": 2,
            "autoscaling": {
                "enabled": true,
                "min_workers": 1,
                "max_workers": 5,
                "integration": {"type": "dryrun"}
            }
        }"#;
        let config: WorkerGroupConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.worker_tags.as_ref().unwrap().len(), 2);
        assert_eq!(config.priority_tags.as_ref().unwrap()["tag1"], 5);
        assert_eq!(config.dedicated_worker.as_deref(), Some("ws:f/my_script"));
        assert_eq!(config.dedicated_workers.as_ref().unwrap().len(), 2);
        assert_eq!(config.init_bash.as_deref(), Some("apt-get install -y curl"));
        assert_eq!(config.periodic_script_bash.as_deref(), Some("echo ping"));
        assert_eq!(config.periodic_script_interval_seconds, Some(300));
        assert_eq!(config.extra["cache_clear"], serde_json::json!(7));
        assert_eq!(config.additional_python_paths.as_ref().unwrap().len(), 1);
        assert_eq!(config.pip_local_dependencies.as_ref().unwrap().len(), 1);
        assert_eq!(config.env_vars_static.as_ref().unwrap()["FOO"], "bar");
        assert_eq!(config.env_vars_allowlist.as_ref().unwrap().len(), 1);
        assert_eq!(config.min_alive_workers_alert_threshold, Some(2));
        let auto = config.autoscaling.as_ref().unwrap();
        assert!(auto.enabled);
        assert_eq!(auto.min_workers, Some(1));
        assert_eq!(auto.max_workers, Some(5));
        assert_eq!(
            auto.integration.as_ref().unwrap().integration_type,
            "dryrun"
        );
    }

    #[test]
    fn otel_tracing_proxy_languages_roundtrip() {
        let json = r#"{"enabled": true, "enabled_languages": ["python3", "bun", "deno"]}"#;
        let settings: OtelTracingProxySettings = serde_json::from_str(json).unwrap();
        assert!(settings.enabled);
        assert_eq!(settings.enabled_languages.len(), 3);
        assert_eq!(settings.enabled_languages[0], ScriptLang::Python3);
        assert_eq!(settings.enabled_languages[1], ScriptLang::Bun);
        assert_eq!(settings.enabled_languages[2], ScriptLang::Deno);

        let serialized = serde_json::to_string(&settings).unwrap();
        let deserialized: OtelTracingProxySettings = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.enabled_languages.len(), 3);
    }

    #[test]
    fn oauth_full_config_roundtrips() {
        let json = r#"{
            "id": "client_id_123",
            "secret": "secret_456",
            "allowed_domains": ["example.com", "test.com"],
            "connect_config": {
                "auth_url": "https://auth.example.com/authorize",
                "token_url": "https://auth.example.com/token",
                "userinfo_url": "https://auth.example.com/userinfo",
                "scopes": ["openid", "profile"],
                "extra_params": {"prompt": "consent"},
                "req_body_auth": true
            },
            "login_config": {
                "auth_url": "https://login.example.com/authorize",
                "token_url": "https://login.example.com/token"
            }
        }"#;
        let client: OAuthClient = serde_json::from_str(json).unwrap();
        assert_eq!(client.id, "client_id_123");
        assert_eq!(client.secret, *"secret_456");
        assert_eq!(client.allowed_domains.as_ref().unwrap().len(), 2);
        let cc = client.connect_config.as_ref().unwrap();
        assert_eq!(cc.auth_url, "https://auth.example.com/authorize");
        assert_eq!(cc.scopes.as_ref().unwrap().len(), 2);
        assert_eq!(cc.extra_params.as_ref().unwrap()["prompt"], "consent");
        assert_eq!(cc.req_body_auth, Some(true));
        let lc = client.login_config.as_ref().unwrap();
        assert_eq!(lc.token_url, "https://login.example.com/token");
        assert!(lc.userinfo_url.is_none());

        let roundtripped = serde_json::to_string(&client).unwrap();
        let parsed: OAuthClient = serde_json::from_str(&roundtripped).unwrap();
        assert_eq!(parsed.id, client.id);
    }

    #[test]
    fn custom_instance_pg_databases_roundtrips() {
        let json = r#"{
            "user_pwd": "secret123",
            "databases": {
                "mydb": {
                    "logs": {
                        "super_admin": "OK",
                        "database_credentials": "OK",
                        "valid_dbname": "OK",
                        "created_database": "OK",
                        "db_connect": "OK",
                        "grant_permissions": "OK"
                    },
                    "success": true,
                    "tag": "production"
                }
            }
        }"#;
        let pg: CustomInstancePgDatabases = serde_json::from_str(json).unwrap();
        assert_eq!(
            pg.user_pwd.as_ref().and_then(|v| v.as_literal()),
            Some("secret123")
        );
        let db = &pg.databases["mydb"];
        assert!(db.success);
        assert_eq!(db.tag.as_deref(), Some("production"));
        assert_eq!(db.logs.super_admin, "OK");
        assert_eq!(db.logs.grant_permissions, "OK");
    }

    #[test]
    fn db_oversize_alert_defaults() {
        let alert: DbOversizeAlert = serde_json::from_str("{}").unwrap();
        assert!(!alert.enabled);
        assert_eq!(alert.value, 0.0);

        let alert_set: DbOversizeAlert =
            serde_json::from_str(r#"{"enabled": true, "value": 50.5}"#).unwrap();
        assert!(alert_set.enabled);
        assert!((alert_set.value - 50.5).abs() < f32::EPSILON);
    }

    #[test]
    fn instance_config_empty_roundtrips() {
        let config = InstanceConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: InstanceConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.global_settings.base_url.is_none());
        assert!(parsed.worker_configs.is_empty());
    }

    #[test]
    fn diff_value_type_change_detected() {
        let mut current = BTreeMap::new();
        current.insert("key".to_string(), serde_json::json!("string_value"));

        let mut desired = BTreeMap::new();
        desired.insert("key".to_string(), serde_json::json!(42));

        let diff = diff_global_settings(&current, &desired, ApplyMode::Merge);
        assert_eq!(diff.upserts.len(), 1);
        assert_eq!(diff.upserts["key"], serde_json::json!(42));
    }

    #[test]
    fn diff_nested_json_change_detected() {
        let mut current = BTreeMap::new();
        current.insert(
            "smtp".to_string(),
            serde_json::json!({"host": "a.com", "port": 25}),
        );

        let mut desired = BTreeMap::new();
        desired.insert(
            "smtp".to_string(),
            serde_json::json!({"host": "a.com", "port": 587}),
        );

        let diff = diff_global_settings(&current, &desired, ApplyMode::Merge);
        assert_eq!(diff.upserts.len(), 1);
        assert_eq!(diff.upserts["smtp"]["port"], 587);
    }

    // -----------------------------------------------------------------------
    // StringOrSecretRef tests
    // -----------------------------------------------------------------------

    #[test]
    fn string_or_secret_ref_deserialize_literal() {
        let v: StringOrSecretRef = serde_json::from_str(r#""hello""#).unwrap();
        assert_eq!(v.as_literal(), Some("hello"));
        assert!(!v.is_secret_ref());
    }

    #[test]
    fn string_or_secret_ref_deserialize_secret_ref() {
        let json = r#"{"secretKeyRef": {"name": "my-secret", "key": "password"}}"#;
        let v: StringOrSecretRef = serde_json::from_str(json).unwrap();
        assert!(v.is_secret_ref());
        assert!(v.as_literal().is_none());
        let r = v.as_secret_ref().unwrap();
        assert_eq!(r.name, "my-secret");
        assert_eq!(r.key, "password");
    }

    #[test]
    fn string_or_secret_ref_serialize_literal() {
        let v = StringOrSecretRef::Literal("plain".to_string());
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json, serde_json::json!("plain"));
    }

    #[test]
    fn string_or_secret_ref_serialize_secret_ref() {
        let v = StringOrSecretRef::SecretRef(SecretKeyRefWrapper {
            secret_key_ref: SecretKeyRef { name: "s".to_string(), key: "k".to_string() },
        });
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"secretKeyRef": {"name": "s", "key": "k"}})
        );
    }

    #[test]
    fn string_or_secret_ref_from_impls() {
        let v: StringOrSecretRef = "hello".into();
        assert_eq!(v, *"hello");

        let v: StringOrSecretRef = String::from("world").into();
        assert_eq!(v, *"world");
    }

    #[test]
    #[should_panic(expected = "literal_value() called on unresolved secret ref")]
    fn string_or_secret_ref_literal_value_panics_on_ref() {
        let v = StringOrSecretRef::SecretRef(SecretKeyRefWrapper {
            secret_key_ref: SecretKeyRef { name: "s".to_string(), key: "k".to_string() },
        });
        let _ = v.literal_value();
    }

    #[test]
    fn global_settings_with_mixed_literal_and_ref() {
        let json = r#"{
            "license_key": {"secretKeyRef": {"name": "wm-secrets", "key": "license"}},
            "hub_api_secret": "plain-secret",
            "base_url": "https://example.com"
        }"#;
        let gs: GlobalSettings = serde_json::from_str(json).unwrap();
        assert!(gs.license_key.as_ref().unwrap().is_secret_ref());
        assert_eq!(
            gs.hub_api_secret.as_ref().and_then(|v| v.as_literal()),
            Some("plain-secret")
        );
        assert_eq!(gs.base_url.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn string_or_secret_ref_backward_compat_plain_string() {
        let json = r#"{"license_key": "my-key", "scim_token": "tok"}"#;
        let gs: GlobalSettings = serde_json::from_str(json).unwrap();
        assert_eq!(
            gs.license_key.as_ref().and_then(|v| v.as_literal()),
            Some("my-key")
        );
        assert_eq!(
            gs.scim_token.as_ref().and_then(|v| v.as_literal()),
            Some("tok")
        );

        let roundtripped = serde_json::to_string(&gs).unwrap();
        let gs2: GlobalSettings = serde_json::from_str(&roundtripped).unwrap();
        assert_eq!(
            gs2.license_key.as_ref().and_then(|v| v.as_literal()),
            Some("my-key")
        );
    }

    // -----------------------------------------------------------------------
    // EnvRef tests
    // -----------------------------------------------------------------------

    #[test]
    fn env_ref_deserialize() {
        let json = r#"{"envRef": "MY_LICENSE_KEY"}"#;
        let v: StringOrSecretRef = serde_json::from_str(json).unwrap();
        assert!(v.is_env_ref());
        assert!(!v.is_secret_ref());
        assert!(v.as_literal().is_none());
        assert_eq!(v.as_env_ref(), Some("MY_LICENSE_KEY"));
    }

    #[test]
    fn env_ref_serialize() {
        let v = StringOrSecretRef::EnvRef(EnvRefWrapper { env_ref: "MY_VAR".to_string() });
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json, serde_json::json!({"envRef": "MY_VAR"}));
    }

    #[test]
    fn env_ref_display() {
        let v = StringOrSecretRef::EnvRef(EnvRefWrapper { env_ref: "FOO".to_string() });
        assert_eq!(format!("{v}"), "envRef(FOO)");
    }

    #[test]
    #[should_panic(expected = "literal_value() called on unresolved env ref")]
    fn env_ref_literal_value_panics() {
        let v = StringOrSecretRef::EnvRef(EnvRefWrapper { env_ref: "X".to_string() });
        let _ = v.literal_value();
    }

    #[test]
    fn env_ref_equality() {
        let a = StringOrSecretRef::EnvRef(EnvRefWrapper { env_ref: "X".to_string() });
        let b = StringOrSecretRef::EnvRef(EnvRefWrapper { env_ref: "X".to_string() });
        let c = StringOrSecretRef::Literal("X".to_string());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn global_settings_with_env_ref() {
        let json = r#"{
            "license_key": {"envRef": "WM_LICENSE_KEY"},
            "hub_api_secret": "plain",
            "base_url": "https://example.com"
        }"#;
        let gs: GlobalSettings = serde_json::from_str(json).unwrap();
        assert!(gs.license_key.as_ref().unwrap().is_env_ref());
        assert_eq!(
            gs.license_key.as_ref().unwrap().as_env_ref(),
            Some("WM_LICENSE_KEY")
        );
        assert_eq!(
            gs.hub_api_secret.as_ref().and_then(|v| v.as_literal()),
            Some("plain")
        );
    }

    #[test]
    fn resolve_env_refs_replaces_env_ref() {
        // Set a test env var
        unsafe { std::env::set_var("__WM_TEST_LICENSE", "resolved-value") };

        let mut gs = GlobalSettings {
            license_key: Some(StringOrSecretRef::EnvRef(EnvRefWrapper {
                env_ref: "__WM_TEST_LICENSE".to_string(),
            })),
            hub_api_secret: Some(StringOrSecretRef::Literal("plain".to_string())),
            ..Default::default()
        };

        resolve_env_refs(&mut gs).unwrap();
        assert_eq!(
            gs.license_key.as_ref().and_then(|v| v.as_literal()),
            Some("resolved-value")
        );
        // Literal fields are unchanged
        assert_eq!(
            gs.hub_api_secret.as_ref().and_then(|v| v.as_literal()),
            Some("plain")
        );

        unsafe { std::env::remove_var("__WM_TEST_LICENSE") };
    }

    #[test]
    fn resolve_env_refs_errors_on_missing_var() {
        let mut gs = GlobalSettings {
            license_key: Some(StringOrSecretRef::EnvRef(EnvRefWrapper {
                env_ref: "__WM_TEST_NONEXISTENT_12345".to_string(),
            })),
            ..Default::default()
        };

        let err = resolve_env_refs(&mut gs).unwrap_err();
        assert_eq!(err, "__WM_TEST_NONEXISTENT_12345");
    }

    #[test]
    fn resolve_env_refs_handles_smtp_and_oauth() {
        unsafe {
            std::env::set_var("__WM_TEST_SMTP_PWD", "smtp-secret");
            std::env::set_var("__WM_TEST_OAUTH_SECRET", "oauth-secret");
        }

        let mut gs = GlobalSettings {
            smtp_settings: Some(SmtpSettings {
                smtp_password: Some(StringOrSecretRef::EnvRef(EnvRefWrapper {
                    env_ref: "__WM_TEST_SMTP_PWD".to_string(),
                })),
                ..Default::default()
            }),
            oauths: Some({
                let mut m = BTreeMap::new();
                m.insert(
                    "google".to_string(),
                    OAuthClient {
                        id: "id".to_string(),
                        secret: StringOrSecretRef::EnvRef(EnvRefWrapper {
                            env_ref: "__WM_TEST_OAUTH_SECRET".to_string(),
                        }),
                        allowed_domains: None,
                        connect_config: None,
                        login_config: None,
                        share_with_workspaces: None,
                    },
                );
                m
            }),
            ..Default::default()
        };

        resolve_env_refs(&mut gs).unwrap();

        assert_eq!(
            gs.smtp_settings
                .as_ref()
                .unwrap()
                .smtp_password
                .as_ref()
                .and_then(|v| v.as_literal()),
            Some("smtp-secret")
        );
        assert_eq!(
            gs.oauths.as_ref().unwrap()["google"].secret.as_literal(),
            Some("oauth-secret")
        );

        unsafe {
            std::env::remove_var("__WM_TEST_SMTP_PWD");
            std::env::remove_var("__WM_TEST_OAUTH_SECRET");
        }
    }

    #[test]
    fn mixed_literal_secret_ref_env_ref() {
        let json = r#"{
            "license_key": {"envRef": "MY_LIC"},
            "hub_api_secret": {"secretKeyRef": {"name": "s", "key": "k"}},
            "scim_token": "plain-token"
        }"#;
        let gs: GlobalSettings = serde_json::from_str(json).unwrap();
        assert!(gs.license_key.as_ref().unwrap().is_env_ref());
        assert!(gs.hub_api_secret.as_ref().unwrap().is_secret_ref());
        assert_eq!(
            gs.scim_token.as_ref().and_then(|v| v.as_literal()),
            Some("plain-token")
        );
    }

    // -----------------------------------------------------------------------
    // License key expiry diff tests
    // -----------------------------------------------------------------------

    #[test]
    fn diff_license_key_skips_older_expiry() {
        let mut current = BTreeMap::new();
        current.insert(
            "license_key".to_string(),
            serde_json::json!("client1.2000000000.sig123"),
        );

        let mut desired = BTreeMap::new();
        desired.insert(
            "license_key".to_string(),
            serde_json::json!("client1.1000000000.sig123"),
        );

        let diff = diff_global_settings(&current, &desired, ApplyMode::Merge);
        assert!(
            diff.upserts.is_empty(),
            "Should not update license_key when desired expiry is older"
        );
    }

    #[test]
    fn diff_license_key_skips_equal_expiry() {
        let mut current = BTreeMap::new();
        current.insert(
            "license_key".to_string(),
            serde_json::json!("client1.2000000000.sig_a"),
        );

        let mut desired = BTreeMap::new();
        desired.insert(
            "license_key".to_string(),
            serde_json::json!("client1.2000000000.sig_b"),
        );

        let diff = diff_global_settings(&current, &desired, ApplyMode::Merge);
        assert_eq!(
            diff.upserts.len(),
            1,
            "Different signature means different key, should update"
        );
    }

    #[test]
    fn diff_license_key_updates_newer_expiry() {
        let mut current = BTreeMap::new();
        current.insert(
            "license_key".to_string(),
            serde_json::json!("client1.1000000000.sig123"),
        );

        let mut desired = BTreeMap::new();
        desired.insert(
            "license_key".to_string(),
            serde_json::json!("client1.2000000000.sig123"),
        );

        let diff = diff_global_settings(&current, &desired, ApplyMode::Merge);
        assert_eq!(diff.upserts.len(), 1);
        assert_eq!(
            diff.upserts["license_key"],
            serde_json::json!("client1.2000000000.sig123")
        );
    }

    #[test]
    fn diff_license_key_updates_different_client() {
        let mut current = BTreeMap::new();
        current.insert(
            "license_key".to_string(),
            serde_json::json!("client1.2000000000.sig123"),
        );

        let mut desired = BTreeMap::new();
        desired.insert(
            "license_key".to_string(),
            serde_json::json!("client2.1000000000.sig456"),
        );

        let diff = diff_global_settings(&current, &desired, ApplyMode::Merge);
        assert_eq!(
            diff.upserts.len(),
            1,
            "Different client ID means different key, should always update"
        );
    }

    #[test]
    fn diff_license_key_non_string_always_updates() {
        let mut current = BTreeMap::new();
        current.insert(
            "license_key".to_string(),
            serde_json::json!({"envRef": "LIC_KEY"}),
        );

        let mut desired = BTreeMap::new();
        desired.insert(
            "license_key".to_string(),
            serde_json::json!({"envRef": "NEW_LIC_KEY"}),
        );

        let diff = diff_global_settings(&current, &desired, ApplyMode::Merge);
        assert_eq!(
            diff.upserts.len(),
            1,
            "Non-string license keys should always be updated when different"
        );
    }
}
