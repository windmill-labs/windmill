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
    /// Keys are setting names (e.g. "base_url", "license_key"),
    /// values are arbitrary JSON.
    #[serde(default)]
    pub global_settings: BTreeMap<String, serde_json::Value>,

    /// Worker group configs to sync to the `config` table.
    /// Keys are worker group names (e.g. "default", "gpu").
    /// Each key is stored in the DB as `worker__<key>`.
    #[serde(default)]
    pub worker_configs: BTreeMap<String, serde_json::Value>,
}

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
        assert!(spec.global_settings.is_empty());
        assert!(spec.worker_configs.is_empty());
    }

    #[test]
    fn spec_deserializes_omitted_fields() {
        let json = r#"{}"#;
        let spec: WindmillInstanceSpec =
            serde_json::from_str(json).expect("Should deserialize spec with missing fields");
        assert!(spec.global_settings.is_empty());
        assert!(spec.worker_configs.is_empty());
    }

    #[test]
    fn spec_deserializes_full_example() {
        let json = r#"{
            "global_settings": {
                "base_url": "https://windmill.example.com",
                "license_key": "my-key",
                "retention_period_secs": 2592000,
                "smtp_settings": {"host": "smtp.example.com", "port": 587}
            },
            "worker_configs": {
                "default": {"dedicated_worker": false},
                "gpu": {"dedicated_worker": true}
            }
        }"#;
        let spec: WindmillInstanceSpec =
            serde_json::from_str(json).expect("Should deserialize full spec");
        assert_eq!(spec.global_settings.len(), 4);
        assert_eq!(spec.worker_configs.len(), 2);
        assert_eq!(
            spec.global_settings["base_url"],
            serde_json::json!("https://windmill.example.com")
        );
        assert_eq!(
            spec.worker_configs["gpu"],
            serde_json::json!({"dedicated_worker": true})
        );
    }

    #[test]
    fn spec_roundtrips_through_json() {
        let mut global_settings = BTreeMap::new();
        global_settings.insert(
            "base_url".to_string(),
            serde_json::json!("https://example.com"),
        );
        global_settings.insert("expose_metrics".to_string(), serde_json::json!(true));

        let mut worker_configs = BTreeMap::new();
        worker_configs.insert(
            "default".to_string(),
            serde_json::json!({"init_bash": "echo hello"}),
        );

        let spec = WindmillInstanceSpec { global_settings, worker_configs };

        let json = serde_json::to_string(&spec).expect("Should serialize");
        let deserialized: WindmillInstanceSpec =
            serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(spec.global_settings, deserialized.global_settings);
        assert_eq!(spec.worker_configs, deserialized.worker_configs);
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
