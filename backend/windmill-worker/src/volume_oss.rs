#[cfg(feature = "private")]
pub(crate) use crate::volume_ee::*;

#[cfg(not(feature = "private"))]
#[cfg(feature = "parquet")]
pub(crate) struct LeaseRenewalGuard(pub Option<tokio::task::JoinHandle<()>>);

#[cfg(not(feature = "private"))]
#[cfg(feature = "parquet")]
impl Drop for LeaseRenewalGuard {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            handle.abort();
        }
    }
}

#[cfg(not(feature = "private"))]
#[cfg(feature = "parquet")]
pub(crate) struct VolumeSetupResult {
    pub states: Vec<windmill_worker_volumes::VolumeState>,
    pub writable: Vec<bool>,
    pub client: Option<std::sync::Arc<dyn windmill_worker_volumes::DynObjectStore>>,
    pub lease_renewal: LeaseRenewalGuard,
}

#[cfg(not(feature = "private"))]
#[cfg(feature = "parquet")]
#[allow(dead_code)]
pub(crate) fn setup_volume_mount_paths(
    _volume: &windmill_worker_volumes::VolumeMount,
    _state: &windmill_worker_volumes::VolumeState,
    _job_dir: &str,
    _language: windmill_common::scripts::ScriptLang,
    _envs: &mut std::collections::HashMap<String, String>,
    _shared_mount: &mut String,
) -> windmill_common::error::Result<()> {
    Err(windmill_common::error::Error::internal_err(
        "Volumes are not available in OSS".to_string(),
    ))
}

#[cfg(not(feature = "private"))]
#[cfg(feature = "parquet")]
pub(crate) async fn setup_volumes_sql_worker(
    _volume_mounts: &[windmill_worker_volumes::VolumeMount],
    _db: &windmill_common::DB,
    _workspace_id: &str,
    _job_id: uuid::Uuid,
    _permissioned_as: &str,
    _worker_name: &str,
    _job_dir: &str,
    _client: &windmill_common::client::AuthedClient,
    _conn: &windmill_common::worker::Connection,
    _language: windmill_common::scripts::ScriptLang,
    _envs: &mut std::collections::HashMap<String, String>,
    _shared_mount: &mut String,
) -> windmill_common::error::Result<VolumeSetupResult> {
    Err(windmill_common::error::Error::internal_err(
        "Volumes are not available in OSS".to_string(),
    ))
}

#[cfg(not(feature = "private"))]
#[cfg(feature = "parquet")]
pub(crate) async fn setup_volumes_http_worker(
    _volume_mounts: &[windmill_worker_volumes::VolumeMount],
    _http: &windmill_common::worker::HttpClient,
    _workspace_id: &str,
    _job_id: uuid::Uuid,
    _permissioned_as: &str,
    _canceled_by: &Option<String>,
    _worker_name: &str,
    _job_dir: &str,
    _conn: &windmill_common::worker::Connection,
    _language: windmill_common::scripts::ScriptLang,
    _envs: &mut std::collections::HashMap<String, String>,
    _shared_mount: &mut String,
) -> windmill_common::error::Result<VolumeSetupResult> {
    Err(windmill_common::error::Error::internal_err(
        "Volumes are not available in OSS".to_string(),
    ))
}

#[cfg(not(feature = "private"))]
#[cfg(feature = "parquet")]
pub(crate) async fn sync_volumes_sql_worker(
    _volume_states: &[windmill_worker_volumes::VolumeState],
    _volume_writable: &[bool],
    _vol_client: &std::sync::Arc<dyn windmill_worker_volumes::DynObjectStore>,
    _db: &windmill_common::DB,
    _workspace_id: &str,
    _job_id: uuid::Uuid,
    _worker_name: &str,
    _conn: &windmill_common::worker::Connection,
    _job_succeeded: bool,
) {
}

#[cfg(not(feature = "private"))]
#[cfg(feature = "parquet")]
pub(crate) async fn sync_volumes_http_worker(
    _volume_states: &[windmill_worker_volumes::VolumeState],
    _volume_writable: &[bool],
    _http: &windmill_common::worker::HttpClient,
    _workspace_id: &str,
    _job_id: uuid::Uuid,
    _worker_name: &str,
    _conn: &windmill_common::worker::Connection,
    _job_succeeded: bool,
) {
}
