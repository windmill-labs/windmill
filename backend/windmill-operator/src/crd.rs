use std::collections::BTreeMap;

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// WindmillInstance CRD spec.
///
/// Declares the desired state for instance-level configuration:
/// - `global_settings` maps directly to the `global_settings` table
/// - `worker_configs` maps to the `config` table with a `worker__` prefix
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "windmill.dev",
    version = "v1alpha1",
    kind = "WindmillInstance",
    namespaced,
    shortname = "wmi",
    status = "WindmillInstanceStatus",
    printcolumn = r#"{"name":"Synced","type":"string","jsonPath":".status.synced"}"#,
    printcolumn = r#"{"name":"Last Synced","type":"date","jsonPath":".status.lastSyncedAt"}"#,
    printcolumn = r#"{"name":"Age","type":"date","jsonPath":".metadata.creationTimestamp"}"#
)]
pub struct WindmillInstanceSpec {
    /// Global settings to sync to the `global_settings` table.
    #[serde(default)]
    pub global_settings: GlobalSettings,

    /// Worker group configs to sync to the `config` table.
    /// Keys are worker group names (e.g. "default", "gpu").
    /// Each key is stored in the DB as `worker__<key>`.
    #[serde(default)]
    pub worker_configs: BTreeMap<String, WorkerGroupConfig>,
}

// ---------------------------------------------------------------------------
// Global settings
// ---------------------------------------------------------------------------

/// Typed global settings with schema validation.
/// Known settings have explicit fields; unknown settings pass through via `extra`.
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct GlobalSettings {
    // Numeric settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_key: Option<String>,
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
    pub hub_api_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scim_token: Option<String>,
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
    pub secret_backend: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slack: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teams: Option<serde_json::Value>,

    /// Catch-all for settings not yet covered by typed fields.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl GlobalSettings {
    /// Convert to a flat `BTreeMap` suitable for `db_sync::sync_global_settings`.
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
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct SmtpSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_password: Option<String>,
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
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
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
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct OAuthClient {
    pub id: String,
    pub secret: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_config: Option<OAuthConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_config: Option<OAuthConfig>,
}

/// OAuth provider endpoint configuration.
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
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
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
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
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct OtelTracingProxySettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled_languages: Vec<ScriptLang>,
}

/// Script language identifier.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash, JsonSchema)]
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
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(untagged)]
pub enum CriticalErrorChannel {
    Email { email: String },
    Slack { slack_channel: String },
    Teams { teams_channel: TeamsChannel },
}

/// Microsoft Teams channel reference.
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
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
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
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
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct DucklakeSettings {
    pub ducklakes: BTreeMap<String, Ducklake>,
}

/// A single DuckLake instance configuration.
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct Ducklake {
    pub catalog: DucklakeCatalog,
    pub storage: DucklakeStorage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<String>,
}

/// DuckLake catalog backend reference.
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct DucklakeCatalog {
    pub resource_type: DucklakeCatalogResourceType,
    pub resource_path: String,
}

/// DuckLake storage location.
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct DucklakeStorage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,
    pub path: String,
}

/// The type of database backing a DuckLake catalog.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
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
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct CustomInstancePgDatabases {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_pwd: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub databases: BTreeMap<String, CustomInstanceDb>,
}

/// Status of a single custom instance database.
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
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
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
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
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
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
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
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
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
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
    pub cache_clear: Option<u32>,
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

    /// Catch-all for fields not yet covered by typed fields.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Status subresource
// ---------------------------------------------------------------------------

/// Status subresource for WindmillInstance.
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WindmillInstanceStatus {
    /// Whether the last reconciliation was successful.
    pub synced: bool,
    /// Human-readable status message.
    #[serde(default)]
    pub message: String,
    /// The `.metadata.generation` that was last observed.
    #[serde(default)]
    pub observed_generation: i64,
    /// Timestamp of the last successful sync.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use kube::CustomResourceExt;

    #[test]
    fn crd_generation_produces_valid_yaml() {
        let crd = WindmillInstance::crd();
        let yaml = serde_yml::to_string(&crd).expect("CRD should serialize to YAML");
        assert!(
            yaml.contains("windmill.dev"),
            "CRD should have group windmill.dev"
        );
        assert!(
            yaml.contains("v1alpha1"),
            "CRD should have version v1alpha1"
        );
        assert!(
            yaml.contains("WindmillInstance"),
            "CRD should have kind WindmillInstance"
        );
        assert!(yaml.contains("wmi"), "CRD should have shortname wmi");
    }

    #[test]
    fn crd_metadata() {
        let crd = WindmillInstance::crd();
        assert_eq!(
            crd.metadata.name.as_deref(),
            Some("windmillinstances.windmill.dev")
        );
    }

    #[test]
    fn spec_deserializes_with_defaults() {
        let json = r#"{"global_settings": {}, "worker_configs": {}}"#;
        let spec: WindmillInstanceSpec =
            serde_json::from_str(json).expect("Should deserialize empty spec");
        assert!(spec.global_settings.to_settings_map().is_empty());
        assert!(spec.worker_configs.is_empty());
    }

    #[test]
    fn spec_deserializes_omitted_fields() {
        let json = r#"{}"#;
        let spec: WindmillInstanceSpec =
            serde_json::from_str(json).expect("Should deserialize spec with missing fields");
        assert!(spec.global_settings.to_settings_map().is_empty());
        assert!(spec.worker_configs.is_empty());
    }

    #[test]
    fn spec_deserializes_full_example() {
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
        let spec: WindmillInstanceSpec =
            serde_json::from_str(json).expect("Should deserialize full spec");
        assert_eq!(
            spec.global_settings.base_url.as_deref(),
            Some("https://windmill.example.com")
        );
        assert_eq!(spec.global_settings.retention_period_secs, Some(2592000));
        assert!(spec.global_settings.smtp_settings.is_some());
        let smtp = spec.global_settings.smtp_settings.as_ref().unwrap();
        assert_eq!(smtp.smtp_host.as_deref(), Some("smtp.example.com"));
        assert_eq!(smtp.smtp_port, Some(587));

        let otel = spec.global_settings.otel.as_ref().unwrap();
        assert_eq!(otel.metrics_enabled, Some(true));
        assert_eq!(
            otel.otel_exporter_otlp_endpoint.as_deref(),
            Some("http://otel:4317")
        );

        let channels = spec
            .global_settings
            .critical_error_channels
            .as_ref()
            .unwrap();
        assert_eq!(channels.len(), 2);

        let tags = spec.global_settings.custom_tags.as_ref().unwrap();
        assert_eq!(tags, &["gpu", "high-mem"]);

        assert_eq!(spec.worker_configs.len(), 2);
        assert_eq!(
            spec.worker_configs["default"].init_bash.as_deref(),
            Some("echo hello")
        );
        assert_eq!(
            spec.worker_configs["gpu"].dedicated_worker.as_deref(),
            Some("ws:f/gpu")
        );
        let autoscaling = spec.worker_configs["gpu"].autoscaling.as_ref().unwrap();
        assert!(autoscaling.enabled);
        assert_eq!(autoscaling.min_workers, Some(1));
        assert_eq!(autoscaling.max_workers, Some(10));
        let integration = autoscaling.integration.as_ref().unwrap();
        assert_eq!(integration.integration_type, "kubernetes");
    }

    #[test]
    fn spec_roundtrips_through_json() {
        let spec = WindmillInstanceSpec {
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

        let json = serde_json::to_string(&spec).expect("Should serialize");
        let deserialized: WindmillInstanceSpec =
            serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(
            spec.global_settings.base_url,
            deserialized.global_settings.base_url
        );
        assert_eq!(
            spec.global_settings.expose_metrics,
            deserialized.global_settings.expose_metrics
        );
        assert_eq!(
            spec.worker_configs["default"].init_bash,
            deserialized.worker_configs["default"].init_bash
        );
    }

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
    fn crd_schema_has_typed_properties() {
        let crd = WindmillInstance::crd();
        let yaml = serde_yml::to_string(&crd).expect("CRD should serialize to YAML");
        assert!(
            yaml.contains("base_url"),
            "Schema should contain base_url property"
        );
        assert!(
            yaml.contains("smtp_settings"),
            "Schema should contain smtp_settings property"
        );
        assert!(
            yaml.contains("worker_tags"),
            "Schema should contain worker_tags property"
        );
        assert!(
            yaml.contains("retention_period_secs"),
            "Schema should contain retention_period_secs property"
        );
        assert!(
            yaml.contains("otel_exporter_otlp_endpoint"),
            "Schema should contain OTel endpoint property"
        );
        assert!(
            yaml.contains("min_workers"),
            "Schema should contain autoscaling min_workers field"
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

    #[test]
    fn status_serializes_with_camel_case() {
        let status = WindmillInstanceStatus {
            synced: true,
            message: "OK".to_string(),
            observed_generation: 3,
            last_synced_at: Some("2025-01-01T00:00:00Z".to_string()),
        };
        let json = serde_json::to_value(&status).expect("Should serialize status");
        assert!(json.get("lastSyncedAt").is_some(), "Should use camelCase");
        assert!(
            json.get("observedGeneration").is_some(),
            "Should use camelCase"
        );
    }

    #[test]
    fn status_omits_null_last_synced_at() {
        let status = WindmillInstanceStatus {
            synced: false,
            message: "Error".to_string(),
            observed_generation: 1,
            last_synced_at: None,
        };
        let json = serde_json::to_value(&status).expect("Should serialize status");
        assert!(
            json.get("lastSyncedAt").is_none(),
            "Should omit null lastSyncedAt"
        );
    }

    #[test]
    fn status_default() {
        let status = WindmillInstanceStatus::default();
        assert!(!status.synced);
        assert!(status.message.is_empty());
        assert_eq!(status.observed_generation, 0);
        assert!(status.last_synced_at.is_none());
    }
}
