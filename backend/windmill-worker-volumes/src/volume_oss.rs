#[cfg(feature = "private")]
pub use crate::volume_ee::*;

#[cfg(not(feature = "private"))]
use crate::{DownloadStats, SyncStats, VolumeMount, VolumeState};
#[cfg(not(feature = "private"))]
use object_store::ObjectStore;
#[cfg(not(feature = "private"))]
use std::path::Path;
#[cfg(not(feature = "private"))]
use std::sync::Arc;
#[cfg(not(feature = "private"))]
use windmill_common::error;

#[cfg(not(feature = "private"))]
pub async fn download_volume(
    _client: Arc<dyn ObjectStore>,
    _volume: &VolumeMount,
    _job_dir: &str,
    _workspace_id: &str,
) -> error::Result<(VolumeState, DownloadStats)> {
    Err(error::Error::internal_err(
        "Volumes require Windmill Enterprise Edition".to_string(),
    ))
}

#[cfg(not(feature = "private"))]
pub fn volume_nsjail_mount(_local_dir: &Path, _target: &str) -> String {
    String::new()
}

#[cfg(not(feature = "private"))]
pub async fn sync_volume_back(
    _client: Arc<dyn ObjectStore>,
    _state: &VolumeState,
    _workspace_id: &str,
) -> error::Result<SyncStats> {
    Err(error::Error::internal_err(
        "Volumes require Windmill Enterprise Edition".to_string(),
    ))
}
