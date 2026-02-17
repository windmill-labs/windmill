use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[cfg(feature = "parquet")]
use std::sync::Arc;

use windmill_common::error::{self, Error};

const OVERLAY_MARKER: &str = "# SANDBOX_OVERLAY_ACTIVE";

#[derive(Default)]
pub struct SandboxSetupState {
    pub overlay: Option<OverlayMount>,
    /// name → (local_dir, mount_path)
    pub volume_mounts: HashMap<String, (PathBuf, String)>,
}

pub struct OverlayMount {
    pub merged: PathBuf,
    pub upper: PathBuf,
    pub work: PathBuf,
}

/// Build additional nsjail mount blocks for sandbox config.
/// Returns a string to be appended to the `shared_mount` variable.
/// If an overlay is present, includes a marker comment that `finalize_nsjail_config`
/// uses to strip system directory mounts from the final config.
pub fn build_sandbox_mounts(setup: &SandboxSetupState) -> String {
    let mut mounts = String::new();

    if let Some(ref overlay) = setup.overlay {
        mounts.push_str(&format!(
            "\n{OVERLAY_MARKER}\nmount {{\n    src: \"{}\"\n    dst: \"/\"\n    is_bind: true\n    rw: true\n}}\n",
            overlay.merged.display()
        ));
    }

    for (_name, (local_dir, mount_path)) in &setup.volume_mounts {
        mounts.push_str(&format!(
            "\nmount {{\n    src: \"{}\"\n    dst: \"{}\"\n    is_bind: true\n    rw: true\n}}\n",
            local_dir.display(),
            mount_path
        ));
    }

    mounts
}

/// Post-process an nsjail config string. If the overlay marker is present,
/// remove bind mounts for system directories (/bin, /lib, /lib64, /usr, /etc)
/// since the overlay root mount provides them.
/// If no overlay marker is present, returns the config unchanged.
pub fn finalize_nsjail_config(config: &str) -> String {
    if !config.contains(OVERLAY_MARKER) {
        return config.to_string();
    }

    let system_dirs: &[&str] = &["/bin", "/lib", "/lib64", "/usr", "/etc"];
    let mut result_lines: Vec<&str> = Vec::new();
    let mut in_mount_block = false;
    let mut mount_block_start = 0;
    let mut should_remove_block = false;

    let lines: Vec<&str> = config.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();

        if trimmed == OVERLAY_MARKER {
            i += 1;
            continue;
        }

        if !in_mount_block && (trimmed.starts_with("mount {") || trimmed == "mount {") {
            in_mount_block = true;
            mount_block_start = result_lines.len();
            should_remove_block = false;
            result_lines.push(lines[i]);
            i += 1;
            continue;
        }

        if in_mount_block {
            if trimmed.starts_with("dst: \"") {
                if let Some(dst) = trimmed
                    .strip_prefix("dst: \"")
                    .and_then(|s| s.strip_suffix('"'))
                {
                    if system_dirs.contains(&dst) {
                        should_remove_block = true;
                    }
                }
            }

            if trimmed == "}" {
                if should_remove_block {
                    result_lines.truncate(mount_block_start);
                } else {
                    result_lines.push(lines[i]);
                }
                in_mount_block = false;
                i += 1;
                continue;
            }

            if !should_remove_block {
                result_lines.push(lines[i]);
            }
            i += 1;
            continue;
        }

        result_lines.push(lines[i]);
        i += 1;
    }

    result_lines.join("\n")
}

#[cfg(feature = "parquet")]
const SNAPSHOT_CACHE_DIR: &str = "/tmp/windmill/snapshots";

/// Unpack a tar.gz byte stream into `dest_path`.
#[cfg_attr(not(feature = "parquet"), allow(dead_code))]
pub(crate) fn untar_gz(bytes: &[u8], dest_path: &Path) -> error::Result<()> {
    use flate2::read::GzDecoder;
    use std::io::Cursor;
    use tar::Archive;

    let decoder = GzDecoder::new(Cursor::new(bytes));
    let mut archive = Archive::new(decoder);
    archive
        .unpack(dest_path)
        .map_err(|e| Error::ExecutionErr(format!("Failed to unpack tar.gz: {e}")))?;
    Ok(())
}

/// Tar+gzip a directory into bytes.
#[cfg_attr(not(feature = "parquet"), allow(dead_code))]
pub(crate) fn tar_gz(source_path: &Path) -> error::Result<Vec<u8>> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::Builder;

    let buf = Vec::new();
    let encoder = GzEncoder::new(buf, Compression::fast());
    let mut builder = Builder::new(encoder);
    builder
        .append_dir_all(".", source_path)
        .map_err(|e| Error::ExecutionErr(format!("Failed to tar directory: {e}")))?;
    let encoder = builder
        .into_inner()
        .map_err(|e| Error::ExecutionErr(format!("Failed to finish tar: {e}")))?;
    encoder
        .finish()
        .map_err(|e| Error::ExecutionErr(format!("Failed to finish gzip: {e}")))
}

/// Download bytes from object store. Returns `None` if the key doesn't exist.
#[cfg(feature = "parquet")]
pub(crate) async fn fetch_bytes_from_store(
    store: Arc<dyn object_store::ObjectStore>,
    s3_key: &str,
) -> error::Result<Option<bytes::Bytes>> {
    use object_store::path::Path as ObjPath;
    match store.get(&ObjPath::from(s3_key)).await {
        Ok(result) => {
            let bytes = result
                .bytes()
                .await
                .map_err(|e| Error::ExecutionErr(format!("Failed to read S3 bytes: {e}")))?;
            if bytes.is_empty() {
                Ok(None)
            } else {
                Ok(Some(bytes))
            }
        }
        Err(object_store::Error::NotFound { .. }) => Ok(None),
        Err(e) => Err(Error::ExecutionErr(format!("S3 fetch failed: {e}"))),
    }
}

/// Upload bytes to object store.
#[cfg(feature = "parquet")]
pub(crate) async fn put_bytes_to_store(
    store: Arc<dyn object_store::ObjectStore>,
    s3_key: &str,
    bytes: Vec<u8>,
) -> error::Result<()> {
    use object_store::path::Path as ObjPath;
    store
        .put(&ObjPath::from(s3_key), bytes.into())
        .await
        .map_err(|e| Error::ExecutionErr(format!("S3 upload failed: {e}")))?;
    Ok(())
}

/// Download snapshot from S3 if not cached locally. Returns path to unpacked rootfs.
#[cfg(feature = "parquet")]
pub async fn ensure_snapshot_cached(
    w_id: &str,
    name: &str,
    tag: &str,
    db: &windmill_common::DB,
) -> error::Result<PathBuf> {
    use windmill_common::s3_helpers::get_object_store;

    let row = sqlx::query!(
        "SELECT s3_key, content_hash, status FROM sandbox_snapshot \
         WHERE workspace_id = $1 AND name = $2 AND tag = $3",
        w_id,
        name,
        tag,
    )
    .fetch_optional(db)
    .await?;

    let row = row.ok_or_else(|| {
        Error::NotFound(format!(
            "sandbox snapshot {name}:{tag} not found in workspace {w_id}"
        ))
    })?;

    if row.status != "ready" {
        return Err(Error::ExecutionErr(format!(
            "sandbox snapshot {name}:{tag} is not ready (status: {})",
            row.status
        )));
    }

    let cache_key = if row.content_hash.is_empty() {
        format!("{w_id}_{name}_{tag}")
    } else {
        row.content_hash.clone()
    };
    let cache_path = PathBuf::from(SNAPSHOT_CACHE_DIR).join(&cache_key);

    if cache_path.exists() {
        tracing::info!("Snapshot {name}:{tag} found in cache at {}", cache_path.display());
        return Ok(cache_path);
    }

    let os = get_object_store()
        .await
        .ok_or_else(|| Error::ExecutionErr("S3 object store not configured".to_string()))?;

    tracing::info!(
        "Downloading snapshot {name}:{tag} from S3 key: {}",
        row.s3_key
    );

    let bytes = fetch_bytes_from_store(os, &row.s3_key)
        .await?
        .ok_or_else(|| {
            Error::ExecutionErr(format!("Snapshot S3 object not found: {}", row.s3_key))
        })?;

    tokio::fs::create_dir_all(&cache_path)
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to create snapshot cache dir: {e}")))?;

    let cache_path_clone = cache_path.clone();
    let bytes_vec = bytes.to_vec();
    tokio::task::spawn_blocking(move || untar_gz(&bytes_vec, &cache_path_clone))
        .await
        .map_err(|e| Error::ExecutionErr(format!("Spawn blocking failed: {e}")))??;

    tracing::info!(
        "Snapshot {name}:{tag} unpacked to {}",
        cache_path.display()
    );
    Ok(cache_path)
}

#[cfg(not(feature = "parquet"))]
pub async fn ensure_snapshot_cached(
    _w_id: &str,
    _name: &str,
    _tag: &str,
    _db: &windmill_common::DB,
) -> error::Result<PathBuf> {
    Err(Error::ExecutionErr(
        "Sandbox snapshots require the parquet feature (S3 object store)".to_string(),
    ))
}

/// Mount overlayfs: lower=snapshot (read-only), upper+work=per-job writable.
pub async fn mount_overlay(
    snapshot_path: &Path,
    job_dir: &str,
) -> error::Result<OverlayMount> {
    use tokio::process::Command;

    let upper = PathBuf::from(job_dir).join("overlay_upper");
    let work = PathBuf::from(job_dir).join("overlay_work");
    let merged = PathBuf::from(job_dir).join("overlay_merged");

    for dir in [&upper, &work, &merged] {
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|e| Error::ExecutionErr(format!("Failed to create overlay dir: {e}")))?;
    }

    let mount_opts = format!(
        "lowerdir={},upperdir={},workdir={}",
        snapshot_path.display(),
        upper.display(),
        work.display()
    );

    let output = Command::new("mount")
        .args([
            "-t",
            "overlay",
            "overlay",
            "-o",
            &mount_opts,
            &merged.to_string_lossy(),
        ])
        .output()
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to run mount: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::ExecutionErr(format!(
            "Failed to mount overlayfs: {stderr}"
        )));
    }

    tracing::info!("Overlayfs mounted at {}", merged.display());
    Ok(OverlayMount {
        merged,
        upper,
        work,
    })
}

/// Unmount overlayfs and clean up per-job dirs.
pub async fn unmount_overlay(overlay: &OverlayMount) -> error::Result<()> {
    use tokio::process::Command;

    let output = Command::new("umount")
        .arg(&overlay.merged)
        .output()
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to run umount: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("Failed to umount overlayfs at {}: {stderr}", overlay.merged.display());
    }

    for dir in [&overlay.merged, &overlay.upper, &overlay.work] {
        if let Err(e) = tokio::fs::remove_dir_all(dir).await {
            tracing::warn!("Failed to clean up overlay dir {}: {e}", dir.display());
        }
    }

    Ok(())
}

/// Download volume from S3 to local dir. If no S3 object exists, create empty dir (auto-create).
#[cfg(feature = "parquet")]
pub async fn download_volume(
    w_id: &str,
    name: &str,
    local_path: &Path,
    db: &windmill_common::DB,
) -> error::Result<()> {
    use windmill_common::s3_helpers::get_object_store;

    let row = sqlx::query!(
        "SELECT s3_key FROM sandbox_volume WHERE workspace_id = $1 AND name = $2",
        w_id,
        name,
    )
    .fetch_optional(db)
    .await?;

    let s3_key = match row {
        Some(r) => r.s3_key,
        None => {
            let s3_key = format!("sandbox/volumes/{w_id}/{name}.tar.gz");
            sqlx::query!(
                "INSERT INTO sandbox_volume (workspace_id, name, s3_key, created_by) \
                 VALUES ($1, $2, $3, 'system') ON CONFLICT DO NOTHING",
                w_id,
                name,
                &s3_key,
            )
            .execute(db)
            .await?;
            tracing::info!("Auto-created volume {name} in workspace {w_id}");
            return Ok(());
        }
    };

    let Some(os) = get_object_store().await else {
        tracing::info!("No object store configured, using empty volume for {name}");
        return Ok(());
    };

    download_volume_from_store(os, &s3_key, local_path).await
}

/// Core volume download logic, testable with any ObjectStore implementation.
#[cfg(feature = "parquet")]
pub(crate) async fn download_volume_from_store(
    store: Arc<dyn object_store::ObjectStore>,
    s3_key: &str,
    local_path: &Path,
) -> error::Result<()> {
    let bytes = match fetch_bytes_from_store(store, s3_key).await? {
        Some(b) => b,
        None => {
            tracing::info!("Volume has no S3 data yet, using empty dir");
            return Ok(());
        }
    };

    let local_path = local_path.to_path_buf();
    let bytes_vec = bytes.to_vec();
    tokio::task::spawn_blocking(move || untar_gz(&bytes_vec, &local_path))
        .await
        .map_err(|e| Error::ExecutionErr(format!("Spawn blocking failed: {e}")))??;

    Ok(())
}

#[cfg(not(feature = "parquet"))]
pub async fn download_volume(
    _w_id: &str,
    _name: &str,
    _local_path: &Path,
    _db: &windmill_common::DB,
) -> error::Result<()> {
    Err(Error::ExecutionErr(
        "Sandbox volumes require the parquet feature (S3 object store)".to_string(),
    ))
}

/// Re-tar volume dir, upload to S3. Update size_bytes/updated_at in DB.
#[cfg(feature = "parquet")]
pub async fn upload_volume(
    w_id: &str,
    name: &str,
    local_path: &Path,
    db: &windmill_common::DB,
) -> error::Result<()> {
    use windmill_common::s3_helpers::get_object_store;

    let Some(os) = get_object_store().await else {
        tracing::warn!("No object store configured, cannot upload volume {name}");
        return Ok(());
    };

    let s3_key = sqlx::query_scalar!(
        "SELECT s3_key FROM sandbox_volume WHERE workspace_id = $1 AND name = $2",
        w_id,
        name,
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("sandbox volume {name} not found")))?;

    let size = upload_volume_to_store(os, &s3_key, local_path).await?;

    sqlx::query!(
        "UPDATE sandbox_volume SET size_bytes = $3, updated_at = now() \
         WHERE workspace_id = $1 AND name = $2",
        w_id,
        name,
        size,
    )
    .execute(db)
    .await?;

    tracing::info!("Volume {name} uploaded to S3 ({size} bytes)");
    Ok(())
}

/// Core volume upload logic, testable with any ObjectStore implementation.
/// Returns the size in bytes of the uploaded tar.gz.
#[cfg(feature = "parquet")]
pub(crate) async fn upload_volume_to_store(
    store: Arc<dyn object_store::ObjectStore>,
    s3_key: &str,
    local_path: &Path,
) -> error::Result<i64> {
    let local_path = local_path.to_path_buf();
    let bytes = tokio::task::spawn_blocking(move || tar_gz(&local_path))
        .await
        .map_err(|e| Error::ExecutionErr(format!("Spawn blocking failed: {e}")))??;

    let size = bytes.len() as i64;
    put_bytes_to_store(store, s3_key, bytes).await?;
    Ok(size)
}

#[cfg(not(feature = "parquet"))]
pub async fn upload_volume(
    _w_id: &str,
    _name: &str,
    _local_path: &Path,
    _db: &windmill_common::DB,
) -> error::Result<()> {
    Err(Error::ExecutionErr(
        "Sandbox volumes require the parquet feature (S3 object store)".to_string(),
    ))
}

/// Pull Docker image + run optional setup script → tar.gz → upload to S3.
#[cfg(feature = "parquet")]
#[allow(dead_code)]
pub async fn build_snapshot(
    w_id: &str,
    name: &str,
    tag: &str,
    docker_image: &str,
    setup_script: Option<&str>,
    db: &windmill_common::DB,
) -> error::Result<()> {
    use tokio::process::Command;
    use windmill_common::s3_helpers::get_object_store;

    sqlx::query!(
        "UPDATE sandbox_snapshot SET status = 'building', updated_at = now() \
         WHERE workspace_id = $1 AND name = $2 AND tag = $3",
        w_id,
        name,
        tag,
    )
    .execute(db)
    .await?;

    let result = async {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::ExecutionErr(format!("Failed to create temp dir: {e}")))?;
        let rootfs_dir = temp_dir.path().join("rootfs");
        tokio::fs::create_dir_all(&rootfs_dir)
            .await
            .map_err(|e| Error::ExecutionErr(format!("Failed to create rootfs dir: {e}")))?;

        tracing::info!("Exporting docker image {docker_image} for snapshot {name}:{tag}");
        let crane_output = Command::new("crane")
            .args(["export", docker_image, "-"])
            .output()
            .await
            .map_err(|e| Error::ExecutionErr(format!("Failed to run crane: {e}")))?;

        if !crane_output.status.success() {
            let stderr = String::from_utf8_lossy(&crane_output.stderr);
            return Err(Error::ExecutionErr(format!("crane export failed: {stderr}")));
        }

        let crane_bytes = crane_output.stdout;
        let rootfs_dir_clone = rootfs_dir.clone();
        tokio::task::spawn_blocking(move || -> error::Result<()> {
            use std::io::Cursor;
            use tar::Archive;
            let mut archive = Archive::new(Cursor::new(crane_bytes));
            archive
                .unpack(&rootfs_dir_clone)
                .map_err(|e| Error::ExecutionErr(format!("Failed to unpack crane output: {e}")))?;
            Ok(())
        })
        .await
        .map_err(|e| Error::ExecutionErr(format!("Spawn blocking failed: {e}")))??;

        if let Some(script) = setup_script {
            if !script.trim().is_empty() {
                tracing::info!("Running setup script for snapshot {name}:{tag}");
                let output = Command::new("chroot")
                    .args([
                        &rootfs_dir.to_string_lossy().to_string(),
                        "/bin/sh",
                        "-c",
                        script,
                    ])
                    .output()
                    .await
                    .map_err(|e| Error::ExecutionErr(format!("Failed to run setup: {e}")))?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(Error::ExecutionErr(format!("Setup script failed: {stderr}")));
                }
            }
        }

        let rootfs_dir_clone = rootfs_dir.clone();
        let (bytes, content_hash) =
            tokio::task::spawn_blocking(move || -> error::Result<(Vec<u8>, String)> {
                use sha2::{Digest, Sha256};

                let bytes = tar_gz(&rootfs_dir_clone)?;
                let hash = format!("{:x}", Sha256::digest(&bytes));
                Ok((bytes, hash))
            })
            .await
            .map_err(|e| Error::ExecutionErr(format!("Spawn blocking failed: {e}")))??;

        let s3_key = format!("sandbox/snapshots/{w_id}/{name}/{content_hash}.tar.gz");
        let size = bytes.len() as i64;

        let os = get_object_store()
            .await
            .ok_or_else(|| Error::ExecutionErr("S3 object store not configured".to_string()))?;

        put_bytes_to_store(os, &s3_key, bytes).await?;

        sqlx::query!(
            "UPDATE sandbox_snapshot SET \
             s3_key = $4, content_hash = $5, size_bytes = $6, status = 'ready', \
             build_error = NULL, updated_at = now() \
             WHERE workspace_id = $1 AND name = $2 AND tag = $3",
            w_id,
            name,
            tag,
            &s3_key,
            &content_hash,
            size,
        )
        .execute(db)
        .await?;

        tracing::info!("Snapshot {name}:{tag} built successfully ({size} bytes)");
        Ok(())
    }
    .await;

    if let Err(ref e) = result {
        let err_str = e.to_string();
        sqlx::query!(
            "UPDATE sandbox_snapshot SET status = 'failed', build_error = $4, updated_at = now() \
             WHERE workspace_id = $1 AND name = $2 AND tag = $3",
            w_id,
            name,
            tag,
            &err_str,
        )
        .execute(db)
        .await
        .ok();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use windmill_common::sandbox::parse_sandbox_config;

    // =========================================================================
    // Unit tests: build_sandbox_mounts
    // =========================================================================

    #[test]
    fn test_build_sandbox_mounts_empty() {
        let setup = SandboxSetupState::default();
        let mounts = build_sandbox_mounts(&setup);
        assert!(mounts.is_empty());
    }

    #[test]
    fn test_build_sandbox_mounts_volumes_only() {
        let mut setup = SandboxSetupState::default();
        setup.volume_mounts.insert(
            "data".to_string(),
            (PathBuf::from("/job/volumes/data"), "/workspace/data".to_string()),
        );
        let mounts = build_sandbox_mounts(&setup);
        assert!(mounts.contains("dst: \"/workspace/data\""));
        assert!(mounts.contains("src: \"/job/volumes/data\""));
        assert!(mounts.contains("is_bind: true"));
        assert!(mounts.contains("rw: true"));
        assert!(!mounts.contains(OVERLAY_MARKER));
    }

    #[test]
    fn test_build_sandbox_mounts_multiple_volumes() {
        let mut setup = SandboxSetupState::default();
        setup.volume_mounts.insert(
            "input".to_string(),
            (PathBuf::from("/job/volumes/input"), "/mnt/input".to_string()),
        );
        setup.volume_mounts.insert(
            "output".to_string(),
            (PathBuf::from("/job/volumes/output"), "/mnt/output".to_string()),
        );
        let mounts = build_sandbox_mounts(&setup);
        assert!(mounts.contains("/mnt/input"));
        assert!(mounts.contains("/mnt/output"));
        assert_eq!(mounts.matches("mount {").count(), 2);
    }

    #[test]
    fn test_build_sandbox_mounts_overlay_only() {
        let setup = SandboxSetupState {
            overlay: Some(OverlayMount {
                merged: PathBuf::from("/job/overlay_merged"),
                upper: PathBuf::from("/job/overlay_upper"),
                work: PathBuf::from("/job/overlay_work"),
            }),
            volume_mounts: HashMap::new(),
        };
        let mounts = build_sandbox_mounts(&setup);
        assert!(mounts.contains(OVERLAY_MARKER));
        assert!(mounts.contains("src: \"/job/overlay_merged\""));
        assert!(mounts.contains("dst: \"/\""));
        assert!(mounts.contains("is_bind: true"));
        assert!(mounts.contains("rw: true"));
    }

    #[test]
    fn test_build_sandbox_mounts_overlay_and_volumes() {
        let mut setup = SandboxSetupState {
            overlay: Some(OverlayMount {
                merged: PathBuf::from("/job/overlay_merged"),
                upper: PathBuf::from("/job/overlay_upper"),
                work: PathBuf::from("/job/overlay_work"),
            }),
            volume_mounts: HashMap::new(),
        };
        setup.volume_mounts.insert(
            "data".to_string(),
            (PathBuf::from("/job/volumes/data"), "/workspace/data".to_string()),
        );
        let mounts = build_sandbox_mounts(&setup);
        assert!(mounts.contains(OVERLAY_MARKER));
        assert!(mounts.contains("dst: \"/\""));
        assert!(mounts.contains("dst: \"/workspace/data\""));
        assert_eq!(mounts.matches("mount {").count(), 2);
    }

    // =========================================================================
    // Unit tests: finalize_nsjail_config
    // =========================================================================

    #[test]
    fn test_finalize_no_overlay_passes_through() {
        let config = "name: \"test\"\n\
                       mount {\n    src: \"/bin\"\n    dst: \"/bin\"\n    is_bind: true\n}\n\
                       mount {\n    dst: \"/tmp\"\n    fstype: \"tmpfs\"\n    rw: true\n}\n";
        let result = finalize_nsjail_config(config);
        assert_eq!(result, config);
    }

    #[test]
    fn test_finalize_strips_all_system_dirs() {
        let config = "name: \"test\"\n\
                       mount {\n    src: \"/bin\"\n    dst: \"/bin\"\n    is_bind: true\n}\n\
                       mount {\n    src: \"/lib\"\n    dst: \"/lib\"\n    is_bind: true\n}\n\
                       mount {\n    src: \"/lib64\"\n    dst: \"/lib64\"\n    is_bind: true\n    mandatory: false\n}\n\
                       mount {\n    src: \"/usr\"\n    dst: \"/usr\"\n    is_bind: true\n}\n\
                       mount {\n    src: \"/etc\"\n    dst: \"/etc\"\n    is_bind: true\n}\n\
                       # SANDBOX_OVERLAY_ACTIVE\n\
                       mount {\n    src: \"/job/merged\"\n    dst: \"/\"\n    is_bind: true\n    rw: true\n}\n\
                       mount {\n    dst: \"/tmp\"\n    fstype: \"tmpfs\"\n    rw: true\n}\n";
        let result = finalize_nsjail_config(config);
        for dir in &["/bin", "/lib", "/lib64", "/usr", "/etc"] {
            assert!(
                !result.contains(&format!("dst: \"{dir}\"")),
                "System dir {dir} should be stripped"
            );
        }
        assert!(result.contains("dst: \"/\""), "Overlay root mount preserved");
        assert!(result.contains("dst: \"/tmp\""), "tmpfs mount preserved");
    }

    #[test]
    fn test_finalize_preserves_non_system_mounts() {
        let config = "name: \"test\"\n\
                       mount {\n    src: \"/bin\"\n    dst: \"/bin\"\n    is_bind: true\n}\n\
                       mount {\n    src: \"/dev/null\"\n    dst: \"/dev/null\"\n    is_bind: true\n    rw: true\n}\n\
                       mount {\n    src: \"/dev/random\"\n    dst: \"/dev/random\"\n    is_bind: true\n}\n\
                       mount {\n    src: \"/dev/urandom\"\n    dst: \"/dev/urandom\"\n    is_bind: true\n}\n\
                       mount {\n    src: \"/opt/microsoft\"\n    dst: \"/opt/microsoft\"\n    is_bind: true\n}\n\
                       mount {\n    src: \"/job/result.json\"\n    dst: \"/tmp/result.json\"\n    is_bind: true\n    rw: true\n}\n\
                       mount {\n    dst: \"/tmp\"\n    fstype: \"tmpfs\"\n    rw: true\n}\n\
                       mount {\n    dst: \"/dev/shm\"\n    fstype: \"tmpfs\"\n    rw: true\n}\n\
                       # SANDBOX_OVERLAY_ACTIVE\n\
                       mount {\n    src: \"/job/merged\"\n    dst: \"/\"\n    is_bind: true\n    rw: true\n}\n";
        let result = finalize_nsjail_config(config);
        assert!(!result.contains("dst: \"/bin\""), "/bin stripped");
        assert!(result.contains("dst: \"/dev/null\""), "/dev/null kept");
        assert!(result.contains("dst: \"/dev/random\""), "/dev/random kept");
        assert!(result.contains("dst: \"/dev/urandom\""), "/dev/urandom kept");
        assert!(result.contains("dst: \"/opt/microsoft\""), "/opt/microsoft kept");
        assert!(result.contains("dst: \"/tmp/result.json\""), "result.json kept");
        assert!(result.contains("dst: \"/tmp\""), "/tmp tmpfs kept");
        assert!(result.contains("dst: \"/dev/shm\""), "/dev/shm kept");
    }

    #[test]
    fn test_finalize_removes_overlay_marker_line() {
        let config = "name: \"test\"\n\
                       # SANDBOX_OVERLAY_ACTIVE\n\
                       mount {\n    src: \"/job/merged\"\n    dst: \"/\"\n    is_bind: true\n}\n";
        let result = finalize_nsjail_config(config);
        assert!(!result.contains(OVERLAY_MARKER));
        assert!(result.contains("dst: \"/\""));
    }

    // =========================================================================
    // Config pipeline tests: annotation → mounts → real nsjail proto template
    // =========================================================================

    #[test]
    fn test_config_pipeline_bash_snapshot_and_volumes() {
        let bash_script = "#!/bin/bash\n\
                           # sandbox: python-ml:gpu\n\
                           # volume: data:/workspace/data\n\
                           # volume: models:/workspace/models\n\
                           echo 'running ML pipeline'\n";

        let sandbox_config = parse_sandbox_config(bash_script);
        let snap = sandbox_config.snapshot.as_ref().unwrap();
        assert_eq!(snap.name, "python-ml");
        assert_eq!(snap.tag, "gpu");
        assert_eq!(sandbox_config.volumes.len(), 2);

        let setup = SandboxSetupState {
            overlay: Some(OverlayMount {
                merged: PathBuf::from("/tmp/job123/overlay_merged"),
                upper: PathBuf::from("/tmp/job123/overlay_upper"),
                work: PathBuf::from("/tmp/job123/overlay_work"),
            }),
            volume_mounts: HashMap::from([
                (
                    "data".to_string(),
                    (PathBuf::from("/tmp/job123/volumes/data"), "/workspace/data".to_string()),
                ),
                (
                    "models".to_string(),
                    (PathBuf::from("/tmp/job123/volumes/models"), "/workspace/models".to_string()),
                ),
            ]),
        };

        let sandbox_mounts = build_sandbox_mounts(&setup);
        let raw_config = include_str!("../nsjail/run.bash.config.proto");
        let config = raw_config
            .replace("{JOB_DIR}", "/tmp/job123")
            .replace("{CLONE_NEWUSER}", "true")
            .replace("{SHARED_MOUNT}", &sandbox_mounts)
            .replace("{TRACING_PROXY_CA_CERT_PATH}", "/dev/null");

        let final_config = finalize_nsjail_config(&config);

        for dir in &["/bin", "/lib", "/lib64", "/usr", "/etc"] {
            assert!(!final_config.contains(&format!("dst: \"{dir}\"")));
        }
        assert!(final_config.contains("dst: \"/\""));
        assert!(final_config.contains("src: \"/tmp/job123/overlay_merged\""));
        assert!(final_config.contains("dst: \"/workspace/data\""));
        assert!(final_config.contains("dst: \"/workspace/models\""));
        assert!(final_config.contains("dst: \"/tmp\""));
        assert!(final_config.contains("dst: \"/dev/null\""));
        assert!(final_config.contains("dst: \"/tmp/main.sh\""));
        assert!(final_config.contains("name: \"bash run script\""));
    }

    #[test]
    fn test_config_pipeline_python_snapshot_only() {
        let setup = SandboxSetupState {
            overlay: Some(OverlayMount {
                merged: PathBuf::from("/tmp/jobXYZ/overlay_merged"),
                upper: PathBuf::from("/tmp/jobXYZ/overlay_upper"),
                work: PathBuf::from("/tmp/jobXYZ/overlay_work"),
            }),
            volume_mounts: HashMap::new(),
        };

        let sandbox_mounts = build_sandbox_mounts(&setup);
        let raw_config = include_str!("../nsjail/run.python3.config.proto");
        let config = raw_config
            .replace("{JOB_DIR}", "/tmp/jobXYZ")
            .replace("{MAIN}", "main")
            .replace("{CLONE_NEWUSER}", "true")
            .replace("{SHARED_MOUNT}", &sandbox_mounts)
            .replace("{SHARED_DEPENDENCIES}", "")
            .replace("{PY_INSTALL_DIR}", "/usr/local")
            .replace("{GLOBAL_SITE_PACKAGES}", "/usr/lib/python3/dist-packages")
            .replace("{ADDITIONAL_PYTHON_PATHS}", "/tmp")
            .replace("{TRACING_PROXY_CA_CERT_PATH}", "/dev/null");

        let final_config = finalize_nsjail_config(&config);

        for dir in &["/bin", "/lib", "/lib64", "/usr", "/etc"] {
            assert!(!final_config.contains(&format!("dst: \"{dir}\"")));
        }
        assert!(final_config.contains("dst: \"/\""));
        assert!(final_config.contains("dst: \"/tmp/main.py\""));
        assert!(final_config.contains("dst: \"/tmp/wrapper.py\""));
        assert!(final_config.contains("dst: \"/dev/shm\""));
        assert!(final_config.contains("PYTHONPATH"));
    }

    #[test]
    fn test_config_pipeline_volumes_only_keeps_system_dirs() {
        let mut setup = SandboxSetupState::default();
        setup.volume_mounts.insert(
            "cache".to_string(),
            (PathBuf::from("/tmp/job456/volumes/cache"), "/tmp/pip-cache".to_string()),
        );
        let sandbox_mounts = build_sandbox_mounts(&setup);
        assert!(!sandbox_mounts.contains(OVERLAY_MARKER));

        let raw_config = include_str!("../nsjail/run.python3.config.proto");
        let config = raw_config
            .replace("{JOB_DIR}", "/tmp/job456")
            .replace("{MAIN}", "main")
            .replace("{CLONE_NEWUSER}", "true")
            .replace("{SHARED_MOUNT}", &sandbox_mounts)
            .replace("{SHARED_DEPENDENCIES}", "")
            .replace("{PY_INSTALL_DIR}", "/usr/local")
            .replace("{GLOBAL_SITE_PACKAGES}", "/usr/lib/python3/dist-packages")
            .replace("{ADDITIONAL_PYTHON_PATHS}", "/tmp")
            .replace("{TRACING_PROXY_CA_CERT_PATH}", "/dev/null");

        let final_config = finalize_nsjail_config(&config);

        assert!(final_config.contains("dst: \"/bin\""));
        assert!(final_config.contains("dst: \"/lib\""));
        assert!(final_config.contains("dst: \"/usr\""));
        assert!(final_config.contains("dst: \"/etc\""));
        assert!(final_config.contains("dst: \"/tmp/pip-cache\""));
    }

    #[test]
    fn test_config_pipeline_no_annotations_passthrough() {
        let raw_config = include_str!("../nsjail/run.bash.config.proto");
        let config = raw_config
            .replace("{JOB_DIR}", "/tmp/job789")
            .replace("{CLONE_NEWUSER}", "true")
            .replace("{SHARED_MOUNT}", "")
            .replace("{TRACING_PROXY_CA_CERT_PATH}", "/dev/null");

        let final_config = finalize_nsjail_config(&config);

        assert!(final_config.contains("dst: \"/bin\""));
        assert!(final_config.contains("dst: \"/lib\""));
        assert!(final_config.contains("dst: \"/usr\""));
        assert!(final_config.contains("dst: \"/etc\""));
        assert!(!final_config.contains(OVERLAY_MARKER));
    }

    #[test]
    fn test_mount_block_syntax() {
        let setup = SandboxSetupState {
            overlay: Some(OverlayMount {
                merged: PathBuf::from("/job/merged"),
                upper: PathBuf::from("/job/upper"),
                work: PathBuf::from("/job/work"),
            }),
            volume_mounts: HashMap::from([(
                "vol".to_string(),
                (PathBuf::from("/job/volumes/vol"), "/mnt/vol".to_string()),
            )]),
        };
        let mounts = build_sandbox_mounts(&setup);

        for block in mounts.split("mount {").skip(1) {
            assert!(block.contains('}'));
            assert!(block.contains("dst: \""));
            assert!(block.contains("is_bind: true"));
            assert!(block.contains("rw: true"));
        }
    }

    // =========================================================================
    // Unit tests: tar_gz / untar_gz round-trip (no S3, just filesystem)
    // =========================================================================

    #[test]
    fn test_tar_gz_roundtrip() {
        let source_dir = tempfile::tempdir().unwrap();
        std::fs::write(source_dir.path().join("hello.txt"), "world").unwrap();
        std::fs::create_dir(source_dir.path().join("subdir")).unwrap();
        std::fs::write(source_dir.path().join("subdir/nested.txt"), "nested content").unwrap();

        let bytes = tar_gz(source_dir.path()).unwrap();
        assert!(!bytes.is_empty());

        let dest_dir = tempfile::tempdir().unwrap();
        untar_gz(&bytes, dest_dir.path()).unwrap();

        assert_eq!(
            std::fs::read_to_string(dest_dir.path().join("hello.txt")).unwrap(),
            "world"
        );
        assert_eq!(
            std::fs::read_to_string(dest_dir.path().join("subdir/nested.txt")).unwrap(),
            "nested content"
        );
    }

    #[test]
    fn test_tar_gz_empty_dir() {
        let source_dir = tempfile::tempdir().unwrap();
        let bytes = tar_gz(source_dir.path()).unwrap();
        assert!(!bytes.is_empty());

        let dest_dir = tempfile::tempdir().unwrap();
        untar_gz(&bytes, dest_dir.path()).unwrap();
        // Should succeed without error even for empty dir
    }

    // =========================================================================
    // Integration tests: S3 round-trip using in-memory ObjectStore
    // =========================================================================

    #[cfg(feature = "parquet")]
    mod s3_integration {
        use super::*;
        use object_store::memory::InMemory;

        #[tokio::test]
        async fn test_volume_upload_download_roundtrip() {
            // 1. Create a temp dir with test files (simulating volume content)
            let volume_dir = tempfile::tempdir().unwrap();
            std::fs::write(volume_dir.path().join("data.csv"), "col1,col2\n1,2\n3,4").unwrap();
            std::fs::create_dir(volume_dir.path().join("models")).unwrap();
            std::fs::write(
                volume_dir.path().join("models/weights.bin"),
                vec![0xDE, 0xAD, 0xBE, 0xEF],
            )
            .unwrap();

            let store: Arc<dyn object_store::ObjectStore> = Arc::new(InMemory::new());
            let s3_key = "sandbox/volumes/test-ws/myvolume.tar.gz";

            // 2. Upload: tar+gz the volume dir → put to in-memory store
            let size = upload_volume_to_store(store.clone(), s3_key, volume_dir.path())
                .await
                .unwrap();
            assert!(size > 0, "Uploaded tar.gz should have non-zero size");

            // 3. Verify the object exists in the store
            let stored = fetch_bytes_from_store(store.clone(), s3_key)
                .await
                .unwrap();
            assert!(stored.is_some(), "Object should exist in store");
            assert_eq!(stored.unwrap().len() as i64, size);

            // 4. Download: fetch from store → untar to new dir
            let download_dir = tempfile::tempdir().unwrap();
            download_volume_from_store(store, s3_key, download_dir.path())
                .await
                .unwrap();

            // 5. Verify contents match
            assert_eq!(
                std::fs::read_to_string(download_dir.path().join("data.csv")).unwrap(),
                "col1,col2\n1,2\n3,4"
            );
            assert_eq!(
                std::fs::read(download_dir.path().join("models/weights.bin")).unwrap(),
                vec![0xDE, 0xAD, 0xBE, 0xEF]
            );
        }

        #[tokio::test]
        async fn test_download_missing_key_returns_empty() {
            let store: Arc<dyn object_store::ObjectStore> = Arc::new(InMemory::new());
            let download_dir = tempfile::tempdir().unwrap();

            // Download a key that doesn't exist → should succeed (empty volume)
            download_volume_from_store(
                store,
                "sandbox/volumes/nonexistent.tar.gz",
                download_dir.path(),
            )
            .await
            .unwrap();

            // Dir should still be empty (no files unpacked)
            let entries: Vec<_> = std::fs::read_dir(download_dir.path())
                .unwrap()
                .collect();
            assert!(entries.is_empty());
        }

        #[tokio::test]
        async fn test_fetch_bytes_nonexistent_key() {
            let store: Arc<dyn object_store::ObjectStore> = Arc::new(InMemory::new());
            let result = fetch_bytes_from_store(store, "does/not/exist").await.unwrap();
            assert!(result.is_none());
        }

        #[tokio::test]
        async fn test_put_and_fetch_bytes_roundtrip() {
            let store: Arc<dyn object_store::ObjectStore> = Arc::new(InMemory::new());
            let key = "test/data.bin";
            let data = vec![1, 2, 3, 4, 5];

            put_bytes_to_store(store.clone(), key, data.clone())
                .await
                .unwrap();

            let fetched = fetch_bytes_from_store(store, key).await.unwrap().unwrap();
            assert_eq!(fetched.to_vec(), data);
        }

        #[tokio::test]
        async fn test_volume_overwrite() {
            let store: Arc<dyn object_store::ObjectStore> = Arc::new(InMemory::new());
            let s3_key = "sandbox/volumes/ws/vol.tar.gz";

            // Upload v1
            let v1_dir = tempfile::tempdir().unwrap();
            std::fs::write(v1_dir.path().join("version.txt"), "v1").unwrap();
            upload_volume_to_store(store.clone(), s3_key, v1_dir.path())
                .await
                .unwrap();

            // Upload v2 (overwrites)
            let v2_dir = tempfile::tempdir().unwrap();
            std::fs::write(v2_dir.path().join("version.txt"), "v2").unwrap();
            std::fs::write(v2_dir.path().join("new_file.txt"), "new").unwrap();
            upload_volume_to_store(store.clone(), s3_key, v2_dir.path())
                .await
                .unwrap();

            // Download → should get v2
            let download_dir = tempfile::tempdir().unwrap();
            download_volume_from_store(store, s3_key, download_dir.path())
                .await
                .unwrap();

            assert_eq!(
                std::fs::read_to_string(download_dir.path().join("version.txt")).unwrap(),
                "v2"
            );
            assert_eq!(
                std::fs::read_to_string(download_dir.path().join("new_file.txt")).unwrap(),
                "new"
            );
        }

        #[tokio::test]
        async fn test_snapshot_tar_to_store_and_unpack() {
            // Simulates what build_snapshot does: tar a rootfs → upload → download → unpack
            let store: Arc<dyn object_store::ObjectStore> = Arc::new(InMemory::new());

            // Create a fake rootfs
            let rootfs = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(rootfs.path().join("usr/bin")).unwrap();
            std::fs::write(rootfs.path().join("usr/bin/python3"), "#!/fake").unwrap();
            std::fs::create_dir_all(rootfs.path().join("lib")).unwrap();
            std::fs::write(rootfs.path().join("lib/libpython.so"), "fake-lib").unwrap();

            // Tar + upload (like build_snapshot does)
            let bytes = tar_gz(rootfs.path()).unwrap();
            let s3_key = "sandbox/snapshots/ws/mysnap/abc123.tar.gz";
            put_bytes_to_store(store.clone(), s3_key, bytes.clone()).await.unwrap();

            // Fetch + unpack (like ensure_snapshot_cached does)
            let fetched = fetch_bytes_from_store(store, s3_key)
                .await
                .unwrap()
                .unwrap();
            let unpack_dir = tempfile::tempdir().unwrap();
            untar_gz(&fetched, unpack_dir.path()).unwrap();

            // Verify rootfs structure
            assert_eq!(
                std::fs::read_to_string(unpack_dir.path().join("usr/bin/python3")).unwrap(),
                "#!/fake"
            );
            assert_eq!(
                std::fs::read_to_string(unpack_dir.path().join("lib/libpython.so")).unwrap(),
                "fake-lib"
            );
        }
    }
}
