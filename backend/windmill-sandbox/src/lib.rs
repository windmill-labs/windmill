mod types;
pub use types::*;

mod nsjail;
pub use nsjail::*;

mod overlay;
pub use overlay::*;

#[cfg(feature = "private")]
mod s3_ee;
mod s3_oss;
pub use s3_oss::*;

use std::path::Path;
use windmill_common::error::{self, Error};

/// Directory where Docker config.json is written for crane registry auth.
pub const DOCKER_CONFIG_DIR: &str = "/tmp/windmill/docker";

/// Write CLAUDE.md and AGENTS.md into `job_dir` and return nsjail mount
/// entries that bind-mount them into `/tmp/` (the sandbox cwd) so AI coding
/// agents discover them automatically.
pub fn write_agent_files(job_dir: &str) -> String {
    let dir = Path::new(job_dir);
    let mut mounts = String::new();

    for filename in &["CLAUDE.md", "AGENTS.md"] {
        let host_path = dir.join(filename);
        let content = if *filename == "CLAUDE.md" {
            "# Windmill Sandbox\n\
             \n\
             You are running inside a Windmill sandbox environment.\n\
             \n\
             If the `wmill` CLI is available, run `wmill init` to bootstrap \
             project configuration and AI agent guidance files (AGENTS.md, skills, etc.).\n\
             \n\
             Check with: `which wmill && wmill init`\n"
        } else {
            "# Windmill AI Agent Instructions\n\
             \n\
             You are running inside a Windmill sandbox environment.\n\
             \n\
             If the `wmill` CLI binary is present, run `wmill init` to generate \
             full agent guidance files with skill definitions for writing scripts, \
             flows, apps, triggers, schedules, and resources.\n\
             \n\
             Check with: `which wmill && wmill init`\n"
        };
        if std::fs::write(&host_path, content).is_ok() {
            mounts.push_str(&format!(
                "\nmount {{\n    src: \"{}\"\n    dst: \"/tmp/{}\"\n    is_bind: true\n}}\n",
                host_path.display(),
                filename
            ));
        }
    }

    mounts
}

/// Install snapshot tools (wmill CLI and/or AI agent binary) into the rootfs.
///
/// - npm packages (wmill, claude-code, codex) are installed via `bun install -g`
///   inside an nsjail chroot with the host bun binary bind-mounted.
/// - opencode is a standalone Go binary downloaded from GitHub releases.
pub async fn install_snapshot_tools(
    rootfs: &std::path::Path,
    include_wmill: bool,
    agent_binary: Option<&str>,
) -> windmill_common::error::Result<()> {
    let mut npm_packages = Vec::new();

    if include_wmill {
        npm_packages.push("windmill-cli");
    }

    match agent_binary {
        Some("claude") => npm_packages.push("@anthropic-ai/claude-code"),
        Some("codex") => npm_packages.push("@openai/codex"),
        Some("opencode") => {
            install_opencode(rootfs).await?;
        }
        _ => {}
    }

    if !npm_packages.is_empty() {
        install_npm_packages(rootfs, &npm_packages).await?;
    }

    // Create a `node` symlink pointing to the host bun binary path.
    // At runtime, the host bun is bind-mounted into the sandbox (e.g. at
    // /usr/bin/bun). npm-installed CLIs have `#!/usr/bin/env node` shebangs,
    // so we need `node` in PATH to resolve to bun (which is node-compatible).
    if include_wmill || matches!(agent_binary, Some("claude") | Some("codex")) {
        create_node_to_bun_symlink(rootfs).await?;
    }

    Ok(())
}

/// Install npm packages globally into the rootfs using the host `bun install -g`.
///
/// Sets `BUN_INSTALL=<rootfs>/usr/local` so that:
///   - binaries land in `<rootfs>/usr/local/bin/` (as relative symlinks)
///   - node_modules land in `<rootfs>/usr/local/install/global/node_modules/`
///
/// At runtime the host bun binary is bind-mounted into the sandbox, and a
/// `node -> bun` symlink in the rootfs ensures `#!/usr/bin/env node` shebangs
/// resolve to bun (which is fully node-compatible).
async fn install_npm_packages(
    rootfs: &std::path::Path,
    packages: &[&str],
) -> windmill_common::error::Result<()> {
    use tokio::process::Command;
    use windmill_common::error::Error;

    let bun_path = std::env::var("BUN_PATH").unwrap_or_else(|_| "/usr/bin/bun".to_string());
    if !Path::new(&bun_path).exists() {
        return Err(Error::ExecutionErr(format!(
            "Bun binary not found at {bun_path}. Cannot install npm packages into snapshot."
        )));
    }

    // BUN_INSTALL controls where `bun install -g` puts binaries and node_modules.
    let bun_install_dir = rootfs.join("usr/local");
    tokio::fs::create_dir_all(bun_install_dir.join("bin"))
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to create bin dir in rootfs: {e}")))?;

    tracing::info!(
        "Installing npm packages in snapshot: {}",
        packages.join(", ")
    );

    let output = Command::new(&bun_path)
        .args(["install", "-g"])
        .args(packages)
        .env("BUN_INSTALL", &bun_install_dir)
        .output()
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to run bun install: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(Error::ExecutionErr(format!(
            "npm package installation failed: {stderr}\n{stdout}"
        )));
    }

    tracing::info!(
        "npm packages installed successfully: {}",
        packages.join(", ")
    );
    Ok(())
}

/// Create a `node` symlink in the rootfs that points to the bun runtime path.
///
/// The host bun binary is bind-mounted into the sandbox at its host path
/// (e.g. `/usr/bin/bun`). npm-installed CLIs use `#!/usr/bin/env node` shebangs,
/// so this symlink makes `node` resolve to bun (which is fully node-compatible
/// and only needs glibc, unlike the real node binary which needs libuv, libssl, etc.).
async fn create_node_to_bun_symlink(
    rootfs: &std::path::Path,
) -> windmill_common::error::Result<()> {
    let bun_runtime_path = std::env::var("BUN_PATH").unwrap_or_else(|_| "/usr/bin/bun".to_string());

    // Place the symlink in /usr/bin/ which is always in PATH, rather than
    // /usr/local/bin/ which may not be in the worker's PATH_ENV.
    let node_symlink = rootfs.join("usr/bin/node");
    tokio::fs::create_dir_all(node_symlink.parent().unwrap())
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to create bin dir: {e}")))?;

    // Remove any existing node binary/symlink
    let _ = tokio::fs::remove_file(&node_symlink).await;

    tokio::fs::symlink(&bun_runtime_path, &node_symlink)
        .await
        .map_err(|e| {
            Error::ExecutionErr(format!(
                "Failed to create node -> bun symlink at {}: {e}",
                node_symlink.display()
            ))
        })?;

    tracing::info!(
        "Created node symlink: {} -> {}",
        node_symlink.display(),
        bun_runtime_path
    );
    Ok(())
}

/// Download the opencode Go binary from GitHub releases into the rootfs.
async fn install_opencode(rootfs: &std::path::Path) -> windmill_common::error::Result<()> {
    use tokio::process::Command;
    use windmill_common::error::Error;

    let bin_dir = rootfs.join("usr/local/bin");
    tokio::fs::create_dir_all(&bin_dir)
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to create bin dir: {e}")))?;

    let dest = bin_dir.join("opencode");

    tracing::info!("Downloading opencode binary into snapshot");

    // Download latest opencode release for linux amd64
    let output = Command::new("sh")
        .args([
            "-c",
            &format!(
                "curl -fsSL https://github.com/opencode-ai/opencode/releases/latest/download/opencode_linux_amd64 -o {}",
                dest.display()
            ),
        ])
        .output()
        .await
        .map_err(|e| Error::ExecutionErr(format!("Failed to download opencode: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::ExecutionErr(format!(
            "Failed to download opencode binary: {stderr}"
        )));
    }

    // Make executable
    Command::new("chmod")
        .args(["+x", &dest.to_string_lossy()])
        .output()
        .await
        .ok();

    tracing::info!("opencode binary installed at {}", dest.display());
    Ok(())
}

/// Unpack a tar.gz byte stream into `dest_path`.
pub fn untar_gz(bytes: &[u8], dest_path: &Path) -> error::Result<()> {
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

/// Tar+gzip a directory into bytes. Symlinks are preserved as-is (not followed).
pub fn tar_gz(source_path: &Path) -> error::Result<Vec<u8>> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::Builder;

    let buf = Vec::new();
    let encoder = GzEncoder::new(buf, Compression::fast());
    let mut builder = Builder::new(encoder);
    builder.follow_symlinks(false);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tar_gz_roundtrip() {
        let source_dir = tempfile::tempdir().unwrap();
        std::fs::write(source_dir.path().join("hello.txt"), "world").unwrap();
        std::fs::create_dir(source_dir.path().join("subdir")).unwrap();
        std::fs::write(
            source_dir.path().join("subdir/nested.txt"),
            "nested content",
        )
        .unwrap();

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
    }

    #[test]
    fn test_write_agent_files_creates_both_and_returns_mounts() {
        let dir = tempfile::tempdir().unwrap();
        let mounts = write_agent_files(&dir.path().to_string_lossy());

        let claude = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(claude.contains("wmill init"));
        assert!(claude.contains("Windmill"));

        let agents = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
        assert!(agents.contains("wmill init"));
        assert!(agents.contains("Windmill"));

        assert!(mounts.contains("dst: \"/tmp/CLAUDE.md\""));
        assert!(mounts.contains("dst: \"/tmp/AGENTS.md\""));
        assert!(mounts.contains("is_bind: true"));
    }
}
