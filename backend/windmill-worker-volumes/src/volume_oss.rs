#[cfg(feature = "private")]
pub use crate::volume_ee::*;

#[cfg(not(feature = "private"))]
use std::collections::{HashMap, HashSet};
#[cfg(not(feature = "private"))]
use std::path::{Path, PathBuf};
#[cfg(not(feature = "private"))]
use std::sync::Arc;

#[cfg(not(feature = "private"))]
use bytes::Bytes;
#[cfg(not(feature = "private"))]
use futures::TryStreamExt;
#[cfg(not(feature = "private"))]
use object_store::ObjectStore;

#[cfg(not(feature = "private"))]
use crate::{
    collect_symlinks, copy_dir_recursive, restore_symlinks, walk_dir, DownloadStats, SyncStats,
    VolumeMount, VolumeState, VOLUME_CACHE,
};
#[cfg(not(feature = "private"))]
use windmill_common::error;

#[cfg(not(feature = "private"))]
fn volume_prefix(name: &str) -> object_store::path::Path {
    object_store::path::Path::from(format!("volumes/{name}"))
}

#[cfg(not(feature = "private"))]
const SYMLINKS_OBJECT_KEY: &str = "_symlinks.json";

#[cfg(not(feature = "private"))]
pub async fn download_volume(
    client: Arc<dyn ObjectStore>,
    volume: &VolumeMount,
    job_dir: &str,
    workspace_id: &str,
) -> error::Result<(VolumeState, DownloadStats)> {
    let job_volume_dir = PathBuf::from(job_dir).join("_volumes").join(&volume.name);
    std::fs::create_dir_all(&job_volume_dir).map_err(|e| {
        error::Error::internal_err(format!(
            "Failed to create volume directory {}: {e}",
            job_volume_dir.display(),
        ))
    })?;

    let prefix = volume_prefix(&volume.name);

    // 1. Load cached manifest and symlinks if they exist
    let (cached_manifest, cached_symlinks) = {
        let cache = VOLUME_CACHE.lock().unwrap();
        (
            cache.load_cached_manifest(workspace_id, &volume.name),
            cache.load_cached_symlinks(workspace_id, &volume.name),
        )
    };

    // 2. List remote objects → build remote manifest
    let list_result = client.list(Some(&prefix)).try_collect::<Vec<_>>().await;
    let objects = match list_result {
        Ok(objects) => objects,
        Err(e) => {
            tracing::warn!("Failed to list volume '{}': {e}", volume.name);
            Vec::new()
        }
    };

    let mut remote_manifest: HashMap<String, u64> = HashMap::new();
    let mut remote_symlinks_meta: Option<object_store::ObjectMeta> = None;

    for meta in &objects {
        let relative = meta
            .location
            .as_ref()
            .strip_prefix(prefix.as_ref())
            .unwrap_or(meta.location.as_ref())
            .trim_start_matches('/');
        if relative.is_empty() {
            continue;
        }
        if relative == SYMLINKS_OBJECT_KEY {
            remote_symlinks_meta = Some(meta.clone());
            continue;
        }
        remote_manifest.insert(relative.to_string(), meta.size as u64);
    }

    // 3. Download _symlinks.json from remote if it exists
    let remote_symlinks: HashMap<String, String> = if let Some(meta) = &remote_symlinks_meta {
        match client.get(&meta.location).await {
            Ok(data) => match data.bytes().await {
                Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or(cached_symlinks),
                Err(_) => cached_symlinks,
            },
            Err(_) => cached_symlinks,
        }
    } else {
        cached_symlinks
    };

    // 4. Diff: download new/changed into cache dir, delete removed from cache
    let cache_dir = {
        let cache = VOLUME_CACHE.lock().unwrap();
        let dir = cache.volume_cache_dir(workspace_id, &volume.name);
        std::fs::create_dir_all(&dir).ok();
        dir
    };

    let cached = cached_manifest.unwrap_or_default();

    let mut downloaded_count: usize = 0;
    let mut from_cache_count: usize = 0;

    for meta in &objects {
        let relative = meta
            .location
            .as_ref()
            .strip_prefix(prefix.as_ref())
            .unwrap_or(meta.location.as_ref())
            .trim_start_matches('/');
        if relative.is_empty() || relative == SYMLINKS_OBJECT_KEY {
            continue;
        }

        let needs_download = match cached.get(relative) {
            Some(&cached_size) => cached_size != meta.size as u64,
            None => true,
        };

        if needs_download {
            let local_path = cache_dir.join(relative);
            if let Some(parent) = local_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let data = client.get(&meta.location).await.map_err(|e| {
                error::Error::internal_err(format!(
                    "Failed to download volume file '{}': {e}",
                    meta.location,
                ))
            })?;
            let bytes = data.bytes().await.map_err(|e| {
                error::Error::internal_err(format!(
                    "Failed to read volume file '{}': {e}",
                    meta.location,
                ))
            })?;
            std::fs::write(&local_path, &bytes).map_err(|e| {
                error::Error::internal_err(format!(
                    "Failed to write volume file {}: {e}",
                    local_path.display(),
                ))
            })?;
            downloaded_count += 1;
        } else {
            from_cache_count += 1;
        }
    }

    // Delete cached files not in remote
    for key in cached.keys() {
        if !remote_manifest.contains_key(key) {
            let local_path = cache_dir.join(key);
            std::fs::remove_file(&local_path).ok();
        }
    }

    // 5. Save updated manifest and symlinks in cache
    {
        let cache = VOLUME_CACHE.lock().unwrap();
        cache.save_manifest(workspace_id, &volume.name, &remote_manifest);
        cache.save_symlinks(workspace_id, &volume.name, &remote_symlinks);
    }

    // 6. Copy cache dir → job volume dir
    copy_dir_recursive(&cache_dir, &job_volume_dir).map_err(|e| {
        error::Error::internal_err(format!(
            "Failed to copy cache to job dir {}: {e}",
            job_volume_dir.display(),
        ))
    })?;

    // 7. Restore symlinks in job dir
    restore_symlinks(&job_volume_dir, &remote_symlinks);

    // 8. Update cache index, run LRU eviction
    let total_size: u64 = remote_manifest.values().sum();
    {
        let mut cache = VOLUME_CACHE.lock().unwrap();
        cache.touch(workspace_id, &volume.name, total_size);
        cache.evict_if_needed();
    }

    let stats = DownloadStats {
        total_files: downloaded_count + from_cache_count,
        from_cache: from_cache_count,
        downloaded: downloaded_count,
    };

    Ok((
        VolumeState {
            mount: volume.clone(),
            local_dir: job_volume_dir,
            manifest: remote_manifest,
            symlinks: remote_symlinks,
        },
        stats,
    ))
}

#[cfg(not(feature = "private"))]
pub fn volume_nsjail_mount(local_dir: &Path, target: &str) -> String {
    format!(
        r#"
mount {{
    src: "{}"
    dst: "{}"
    is_bind: true
    rw: true
}}
"#,
        local_dir.display(),
        target,
    )
}

#[cfg(not(feature = "private"))]
pub async fn sync_volume_back(
    client: Arc<dyn ObjectStore>,
    state: &VolumeState,
    workspace_id: &str,
) -> error::Result<SyncStats> {
    let prefix = volume_prefix(&state.mount.name);
    let mut total_bytes: u64 = 0;

    // 1. Walk regular files in job dir, diff against download manifest
    let entries = walk_dir(&state.local_dir).map_err(|e| {
        error::Error::internal_err(format!(
            "Failed to walk volume directory {}: {e}",
            state.local_dir.display(),
        ))
    })?;

    #[cfg(not(feature = "enterprise"))]
    if entries.len() > 50 {
        return Err(error::Error::internal_err(format!(
            "Volume '{}' contains {} files which exceeds the 50-file limit. \
             Upgrade to Windmill Enterprise Edition to use larger volumes.",
            state.mount.name,
            entries.len(),
        )));
    }

    let mut seen_keys = HashSet::new();
    let mut changed_files: Vec<(String, PathBuf)> = Vec::new();
    let mut skipped_count: usize = 0;

    for entry in &entries {
        let relative = entry
            .strip_prefix(&state.local_dir)
            .unwrap_or(entry)
            .to_string_lossy()
            .replace('\\', "/");

        seen_keys.insert(relative.clone());

        let local_size = std::fs::metadata(entry).map(|m| m.len()).unwrap_or(0);
        total_bytes += local_size;

        #[cfg(not(feature = "enterprise"))]
        if local_size > 50 * 1024 * 1024 {
            return Err(error::Error::internal_err(format!(
                "Volume file '{}' is {} bytes which exceeds the 50 MB limit. \
                 Upgrade to Windmill Enterprise Edition to upload larger files.",
                relative, local_size,
            )));
        }

        let needs_upload = match state.manifest.get(&relative) {
            Some(&manifest_size) => manifest_size != local_size,
            None => true,
        };

        if needs_upload {
            let data = match std::fs::read(entry) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("Skipping unreadable volume file {}: {e}", entry.display());
                    continue;
                }
            };

            let object_path =
                object_store::path::Path::from(format!("{}/{relative}", prefix.as_ref()));

            client
                .put(&object_path, Bytes::from(data).into())
                .await
                .map_err(|e| {
                    error::Error::internal_err(format!(
                        "Failed to upload volume file '{}': {e}",
                        object_path,
                    ))
                })?;

            changed_files.push((relative, entry.clone()));
        } else {
            skipped_count += 1;
        }
    }

    // 2. Collect symlinks and upload _symlinks.json
    let new_symlinks = collect_symlinks(&state.local_dir);
    if new_symlinks != state.symlinks || !new_symlinks.is_empty() {
        let symlinks_path =
            object_store::path::Path::from(format!("{}/{SYMLINKS_OBJECT_KEY}", prefix.as_ref()));
        if new_symlinks.is_empty() {
            client.delete(&symlinks_path).await.ok();
        } else {
            let data = serde_json::to_vec_pretty(&new_symlinks).unwrap_or_default();
            client
                .put(&symlinks_path, Bytes::from(data).into())
                .await
                .ok();
        }
    }

    // 3. Delete removed files from remote
    let mut removed_keys = Vec::new();
    for key in state.manifest.keys() {
        if !seen_keys.contains(key) {
            let object_path = object_store::path::Path::from(format!("{}/{key}", prefix.as_ref()));
            client.delete(&object_path).await.ok();
            removed_keys.push(key.clone());
        }
    }

    // 4. Sync changes back into cache
    let cache_dir = {
        let cache = VOLUME_CACHE.lock().unwrap();
        cache.volume_cache_dir(workspace_id, &state.mount.name)
    };

    for (relative, src_path) in &changed_files {
        let dst_path = cache_dir.join(relative);
        if let Some(parent) = dst_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::copy(src_path, &dst_path).ok();
    }

    for key in &removed_keys {
        let cache_path = cache_dir.join(key);
        std::fs::remove_file(&cache_path).ok();
    }

    // Build new manifest from current job dir state
    let mut new_manifest: HashMap<String, u64> = HashMap::new();
    for key in &seen_keys {
        let entry_path = state.local_dir.join(key);
        let size = std::fs::metadata(&entry_path).map(|m| m.len()).unwrap_or(0);
        new_manifest.insert(key.clone(), size);
    }

    // Update cache manifest, symlinks, and index
    {
        let mut cache = VOLUME_CACHE.lock().unwrap();
        cache.save_manifest(workspace_id, &state.mount.name, &new_manifest);
        cache.save_symlinks(workspace_id, &state.mount.name, &new_symlinks);
        cache.touch(workspace_id, &state.mount.name, total_bytes);
    }

    Ok(SyncStats {
        new_size_bytes: total_bytes,
        file_count: changed_files.len() + skipped_count,
        uploaded: changed_files.len(),
        skipped: skipped_count,
    })
}
