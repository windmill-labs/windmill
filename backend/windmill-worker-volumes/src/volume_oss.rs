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
        "Volumes are not available in this build".to_string(),
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
        "Volumes are not available in this build".to_string(),
    ))
}

#[cfg(not(feature = "private"))]
pub fn walk_dir(dir: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut result = Vec::new();
    walk_dir_inner(dir, &mut result)?;
    Ok(result)
}

#[cfg(not(feature = "private"))]
fn walk_dir_inner(dir: &Path, result: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_dir() {
            walk_dir_inner(&path, result)?;
        } else if meta.is_file() {
            result.push(path);
        }
    }
    Ok(())
}

#[cfg(not(feature = "private"))]
pub fn collect_symlinks(dir: &Path) -> std::collections::HashMap<String, String> {
    let mut symlinks = std::collections::HashMap::new();
    collect_symlinks_inner(dir, dir, &mut symlinks);
    symlinks
}

#[cfg(not(feature = "private"))]
fn collect_symlinks_inner(
    base: &Path,
    dir: &Path,
    symlinks: &mut std::collections::HashMap<String, String>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() {
            if let Ok(target) = std::fs::read_link(&path) {
                let relative = path
                    .strip_prefix(base)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                symlinks.insert(relative, target.to_string_lossy().to_string());
            }
        } else if meta.is_dir() {
            collect_symlinks_inner(base, &path, symlinks);
        }
    }
}

#[cfg(not(feature = "private"))]
pub fn restore_symlinks(_dir: &Path, _symlinks: &std::collections::HashMap<String, String>) {
    // No-op in OSS build
}
