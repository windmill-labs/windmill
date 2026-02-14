use std::collections::BTreeMap;

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Re-export all config types from windmill-common for downstream consumers.
pub use windmill_common::instance_config::{
    AutoscalingConfig, AutoscalingIntegration, CriticalErrorChannel, CustomInstanceDb,
    CustomInstanceDbLogs, CustomInstancePgDatabases, DbOversizeAlert, Ducklake, DucklakeCatalog,
    DucklakeCatalogResourceType, DucklakeSettings, DucklakeStorage, GlobalSettings,
    IndexerSettings, OAuthClient, OAuthConfig, OtelSettings, OtelTracingProxySettings, ScriptLang,
    SecretKeyRef, SecretKeyRefWrapper, SmtpSettings, StringOrSecretRef, TeamsChannel,
    WorkerGroupConfig,
};

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

    #[test]
    fn crd_schema_supports_secret_refs() {
        let crd = WindmillInstance::crd();
        let yaml = serde_yml::to_string(&crd).expect("CRD should serialize to YAML");
        assert!(
            yaml.contains("secretKeyRef"),
            "CRD schema should contain secretKeyRef for secret reference support"
        );
    }

    #[test]
    fn spec_deserializes_secret_ref_fields() {
        let json = r#"{
            "global_settings": {
                "license_key": {"secretKeyRef": {"name": "wm-secrets", "key": "license"}},
                "base_url": "https://example.com"
            }
        }"#;
        let spec: WindmillInstanceSpec = serde_json::from_str(json).unwrap();
        assert!(spec
            .global_settings
            .license_key
            .as_ref()
            .unwrap()
            .is_secret_ref());
        assert_eq!(
            spec.global_settings.base_url.as_deref(),
            Some("https://example.com")
        );
    }
}
