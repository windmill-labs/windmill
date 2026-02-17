use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

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

/// Download snapshot from S3 if not cached locally. Returns path to unpacked rootfs.
#[cfg(feature = "parquet")]
pub async fn ensure_snapshot_cached(
    w_id: &str,
    name: &str,
    tag: &str,
    db: &windmill_common::DB,
) -> error::Result<PathBuf> {
    use windmill_common::s3_helpers::{attempt_fetch_bytes, get_object_store};

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
    let bytes = attempt_fetch_bytes(os, &row.s3_key).await?;

    tokio::fs::create_dir_all(&cache_path)
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to create snapshot cache dir: {e}")))?;

    let cache_path_clone = cache_path.clone();
    let bytes_clone = bytes.to_vec();
    tokio::task::spawn_blocking(move || -> error::Result<()> {
        use flate2::read::GzDecoder;
        use std::io::Cursor;
        use tar::Archive;

        let decoder = GzDecoder::new(Cursor::new(bytes_clone));
        let mut archive = Archive::new(decoder);
        archive
            .unpack(&cache_path_clone)
            .map_err(|e| Error::ExecutionErr(format!("Failed to unpack snapshot tar.gz: {e}")))?;
        Ok(())
    })
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
    use windmill_common::s3_helpers::{attempt_fetch_bytes, get_object_store};

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

    let bytes = match attempt_fetch_bytes(os, &s3_key).await {
        Ok(b) => b,
        Err(_) => {
            tracing::info!("Volume {name} has no S3 data yet, using empty dir");
            return Ok(());
        }
    };

    let local_path = local_path.to_path_buf();
    let bytes_vec = bytes.to_vec();
    tokio::task::spawn_blocking(move || -> error::Result<()> {
        use flate2::read::GzDecoder;
        use std::io::Cursor;
        use tar::Archive;

        let decoder = GzDecoder::new(Cursor::new(bytes_vec));
        let mut archive = Archive::new(decoder);
        archive
            .unpack(&local_path)
            .map_err(|e| Error::ExecutionErr(format!("Failed to unpack volume tar.gz: {e}")))?;
        Ok(())
    })
    .await
    .map_err(|e| Error::ExecutionErr(format!("Spawn blocking failed: {e}")))??;

    tracing::info!("Volume {name} downloaded and unpacked");
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

    let local_path = local_path.to_path_buf();
    let bytes = tokio::task::spawn_blocking(move || -> error::Result<Vec<u8>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use tar::Builder;

        let buf = Vec::new();
        let encoder = GzEncoder::new(buf, Compression::fast());
        let mut builder = Builder::new(encoder);
        builder
            .append_dir_all(".", &local_path)
            .map_err(|e| Error::ExecutionErr(format!("Failed to tar volume: {e}")))?;
        let encoder = builder
            .into_inner()
            .map_err(|e| Error::ExecutionErr(format!("Failed to finish tar: {e}")))?;
        let bytes = encoder
            .finish()
            .map_err(|e| Error::ExecutionErr(format!("Failed to finish gzip: {e}")))?;
        Ok(bytes)
    })
    .await
    .map_err(|e| Error::ExecutionErr(format!("Spawn blocking failed: {e}")))??;

    let size = bytes.len() as i64;

    os.put(
        &object_store::path::Path::from(s3_key.as_str()),
        bytes.into(),
    )
    .await
    .map_err(|e| Error::ExecutionErr(format!("Failed to upload volume to S3: {e}")))?;

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
                use flate2::write::GzEncoder;
                use flate2::Compression;
                use sha2::{Digest, Sha256};
                use tar::Builder;

                let buf = Vec::new();
                let encoder = GzEncoder::new(buf, Compression::default());
                let mut builder = Builder::new(encoder);
                builder
                    .append_dir_all(".", &rootfs_dir_clone)
                    .map_err(|e| Error::ExecutionErr(format!("Failed to tar snapshot: {e}")))?;
                let encoder = builder
                    .into_inner()
                    .map_err(|e| Error::ExecutionErr(format!("Failed to finish tar: {e}")))?;
                let bytes = encoder
                    .finish()
                    .map_err(|e| Error::ExecutionErr(format!("Failed to finish gzip: {e}")))?;
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

        os.put(
            &object_store::path::Path::from(s3_key.as_str()),
            bytes.into(),
        )
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to upload snapshot to S3: {e}")))?;

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

    // --- Unit tests for build_sandbox_mounts ---

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

    // --- Unit tests for finalize_nsjail_config ---

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

    // --- End-to-end: real bash proto template with sandbox ---

    #[test]
    fn test_e2e_bash_with_snapshot_and_volume() {
        // Simulate what happens in bash_executor.rs:
        // 1. Parse sandbox annotations from script code
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

        // 2. Build sandbox mounts (simulate worker.rs setup)
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

        // 3. Simulate executor: template replacement + append sandbox mounts
        let raw_config = include_str!("../nsjail/run.bash.config.proto");
        let config = raw_config
            .replace("{JOB_DIR}", "/tmp/job123")
            .replace("{CLONE_NEWUSER}", "true")
            .replace("{SHARED_MOUNT}", &sandbox_mounts)
            .replace("{TRACING_PROXY_CA_CERT_PATH}", "/dev/null");

        // 4. Finalize: strip system dir mounts since overlay provides rootfs
        let final_config = finalize_nsjail_config(&config);

        // Verify system dirs removed
        for dir in &["/bin", "/lib", "/lib64", "/usr", "/etc"] {
            assert!(
                !final_config.contains(&format!("dst: \"{dir}\"")),
                "System dir {dir} should be removed when overlay is active"
            );
        }

        // Verify overlay root mount present
        assert!(final_config.contains("dst: \"/\""));
        assert!(final_config.contains("src: \"/tmp/job123/overlay_merged\""));

        // Verify volume mounts present
        assert!(final_config.contains("dst: \"/workspace/data\""));
        assert!(final_config.contains("src: \"/tmp/job123/volumes/data\""));
        assert!(final_config.contains("dst: \"/workspace/models\""));
        assert!(final_config.contains("src: \"/tmp/job123/volumes/models\""));

        // Verify essential non-system mounts preserved
        assert!(final_config.contains("dst: \"/tmp\""));
        assert!(final_config.contains("dst: \"/dev/null\""));
        assert!(final_config.contains("dst: \"/dev/random\""));
        assert!(final_config.contains("dst: \"/dev/urandom\""));
        assert!(final_config.contains("dst: \"/tmp/main.sh\""));
        assert!(final_config.contains("dst: \"/tmp/wrapper.sh\""));
        assert!(final_config.contains("dst: \"/tmp/result.json\""));

        // Verify nsjail metadata preserved
        assert!(final_config.contains("name: \"bash run script\""));
        assert!(final_config.contains("mode: ONCE"));
        assert!(final_config.contains("envar: \"HOME=/tmp\""));
    }

    // --- End-to-end: real python proto template with sandbox ---

    #[test]
    fn test_e2e_python_with_snapshot_only() {
        let python_script = "# sandbox: python312:latest\n\
                             import pandas as pd\n\
                             def main():\n    return pd.DataFrame()\n";

        let sandbox_config = parse_sandbox_config(python_script);
        assert!(sandbox_config.snapshot.is_some());
        assert!(sandbox_config.volumes.is_empty());

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

        // System dirs removed
        for dir in &["/bin", "/lib", "/lib64", "/usr", "/etc"] {
            assert!(
                !final_config.contains(&format!("dst: \"{dir}\"")),
                "{dir} should be stripped with overlay"
            );
        }

        // Overlay root
        assert!(final_config.contains("dst: \"/\""));
        assert!(final_config.contains("src: \"/tmp/jobXYZ/overlay_merged\""));

        // Python-specific mounts preserved
        assert!(final_config.contains("dst: \"/tmp/main.py\""));
        assert!(final_config.contains("dst: \"/tmp/wrapper.py\""));
        assert!(final_config.contains("dst: \"/tmp/args.json\""));
        assert!(final_config.contains("dst: \"/tmp/result.json\""));
        assert!(final_config.contains("dst: \"/dev/shm\""));
        assert!(final_config.contains("dst: \"/tmp\""));

        // Python metadata
        assert!(final_config.contains("name: \"python run script\""));
        assert!(final_config.contains("PYTHONPATH"));
    }

    // --- End-to-end: volumes without snapshot (no overlay) ---

    #[test]
    fn test_e2e_volumes_only_no_overlay() {
        let script = "# volume: cache:/tmp/pip-cache\ndef main(): pass\n";
        let sandbox_config = parse_sandbox_config(script);
        assert!(sandbox_config.snapshot.is_none());
        assert_eq!(sandbox_config.volumes.len(), 1);

        let mut setup = SandboxSetupState::default();
        setup.volume_mounts.insert(
            "cache".to_string(),
            (PathBuf::from("/tmp/job456/volumes/cache"), "/tmp/pip-cache".to_string()),
        );

        let sandbox_mounts = build_sandbox_mounts(&setup);

        // No overlay marker → system dirs should NOT be stripped
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

        // System dirs preserved (no overlay)
        assert!(final_config.contains("dst: \"/bin\""), "/bin should be kept");
        assert!(final_config.contains("dst: \"/lib\""), "/lib should be kept");
        assert!(final_config.contains("dst: \"/usr\""), "/usr should be kept");
        assert!(final_config.contains("dst: \"/etc\""), "/etc should be kept");

        // Volume mount present
        assert!(final_config.contains("dst: \"/tmp/pip-cache\""));
        assert!(final_config.contains("src: \"/tmp/job456/volumes/cache\""));
    }

    // --- End-to-end: no sandbox annotations at all (normal job) ---

    #[test]
    fn test_e2e_no_annotations_passthrough() {
        let script = "def main():\n    return 42\n";
        let sandbox_config = parse_sandbox_config(script);
        assert!(sandbox_config.snapshot.is_none());
        assert!(sandbox_config.volumes.is_empty());

        // No sandbox setup → shared_mount is empty → config unchanged
        let raw_config = include_str!("../nsjail/run.bash.config.proto");
        let config = raw_config
            .replace("{JOB_DIR}", "/tmp/job789")
            .replace("{CLONE_NEWUSER}", "true")
            .replace("{SHARED_MOUNT}", "")
            .replace("{TRACING_PROXY_CA_CERT_PATH}", "/dev/null");

        let final_config = finalize_nsjail_config(&config);

        // Everything preserved as-is
        assert!(final_config.contains("dst: \"/bin\""));
        assert!(final_config.contains("dst: \"/lib\""));
        assert!(final_config.contains("dst: \"/usr\""));
        assert!(final_config.contains("dst: \"/etc\""));
        assert!(final_config.contains("dst: \"/tmp\""));
        assert!(!final_config.contains(OVERLAY_MARKER));
    }

    // --- Verify mount block syntax is valid nsjail proto format ---

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

        // Each mount block should have proper nsjail proto syntax
        for block in mounts.split("mount {").skip(1) {
            assert!(block.contains('}'), "Every mount block must close");
            assert!(block.contains("dst: \""), "Every mount needs a dst");
            assert!(block.contains("is_bind: true"), "Sandbox mounts are bind mounts");
            assert!(block.contains("rw: true"), "Sandbox mounts are read-write");
        }
    }
}
