use std::path::{Path, PathBuf};
use windmill_common::error::Error;

use crate::OverlayMount;

/// Search for fuse-overlayfs binary in PATH and common locations.
fn find_fuse_overlayfs() -> Option<String> {
    if let Ok(output) = std::process::Command::new("which")
        .arg("fuse-overlayfs")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }
    let candidates = [
        "/usr/local/bin/fuse-overlayfs",
        "/usr/bin/fuse-overlayfs",
    ];
    let home_bin = std::env::var("HOME")
        .ok()
        .map(|h| format!("{h}/bin/fuse-overlayfs"));

    for path in candidates
        .iter()
        .map(|s| s.to_string())
        .chain(home_bin.into_iter())
    {
        if Path::new(&path).exists() {
            return Some(path);
        }
    }
    None
}

/// Mount overlayfs: lower=snapshot (read-only), upper+work=per-job writable.
/// Tries kernel overlay first (needs root/CAP_SYS_ADMIN), falls back to
/// fuse-overlayfs if available (works without root).
pub async fn mount_overlay(
    snapshot_path: &Path,
    job_dir: &str,
) -> windmill_common::error::Result<OverlayMount> {
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

    // Try kernel overlay mount first (requires root or CAP_SYS_ADMIN)
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

    if output.status.success() {
        tracing::info!("Overlayfs (kernel) mounted at {}", merged.display());
        return Ok(OverlayMount {
            merged,
            upper,
            work,
            is_fuse: false,
        });
    }

    // Fallback: try fuse-overlayfs (works without root)
    let fuse_bin = find_fuse_overlayfs().unwrap_or("fuse-overlayfs".to_string());
    let fuse_output = Command::new(&fuse_bin)
        .args(["-o", &mount_opts, &merged.to_string_lossy()])
        .output()
        .await;

    match fuse_output {
        Ok(out) if out.status.success() => {
            tracing::info!("Overlayfs (fuse) mounted at {}", merged.display());
            Ok(OverlayMount {
                merged,
                upper,
                work,
                is_fuse: true,
            })
        }
        Ok(out) => {
            let kernel_err = String::from_utf8_lossy(&output.stderr);
            let fuse_err = String::from_utf8_lossy(&out.stderr);
            Err(Error::ExecutionErr(format!(
                "Failed to mount overlayfs.\n  \
                 kernel mount: {kernel_err}\n  \
                 fuse-overlayfs: {fuse_err}"
            )))
        }
        Err(_) => {
            let kernel_err = String::from_utf8_lossy(&output.stderr);
            Err(Error::ExecutionErr(format!(
                "Failed to mount overlayfs (kernel: {kernel_err}; \
                 fuse-overlayfs not found)"
            )))
        }
    }
}

/// Unmount overlayfs and clean up per-job dirs.
pub async fn unmount_overlay(overlay: &OverlayMount) -> windmill_common::error::Result<()> {
    use tokio::process::Command;

    let merged_str = overlay.merged.to_string_lossy().to_string();
    let (cmd, args): (&str, Vec<&str>) = if overlay.is_fuse {
        ("fusermount3", vec!["-u", &merged_str])
    } else {
        ("umount", vec![&merged_str])
    };

    let output = Command::new(cmd)
        .args(&args)
        .output()
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to run {cmd}: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(
            "Failed to unmount overlayfs at {}: {stderr}",
            overlay.merged.display()
        );
    }

    for dir in [&overlay.merged, &overlay.upper, &overlay.work] {
        if let Err(e) = tokio::fs::remove_dir_all(dir).await {
            tracing::warn!("Failed to clean up overlay dir {}: {e}", dir.display());
        }
    }

    Ok(())
}
