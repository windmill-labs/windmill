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
