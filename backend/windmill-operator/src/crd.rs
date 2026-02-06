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
