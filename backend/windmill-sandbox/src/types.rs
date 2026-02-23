use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct SandboxConfig {
    pub snapshot: Option<SnapshotRef>,
    pub volumes: HashMap<String, String>,
    pub allowed_domains: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SnapshotRef {
    pub name: String,
    pub tag: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct SandboxSnapshot {
    pub workspace_id: String,
    pub name: String,
    pub tag: String,
    pub s3_key: String,
    pub content_hash: String,
    pub docker_image: String,
    pub setup_script: Option<String>,
    pub size_bytes: Option<i64>,
    pub status: String,
    pub build_error: Option<String>,
    pub build_job_id: Option<Uuid>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub extra_perms: serde_json::Value,
    pub include_wmill: bool,
    pub agent_binary: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::FromRow)]
pub struct SandboxVolume {
    pub workspace_id: String,
    pub name: String,
    pub s3_key: String,
    pub size_bytes: Option<i64>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub extra_perms: serde_json::Value,
}

#[derive(Default)]
pub struct SandboxSetupState {
    pub overlay: Option<OverlayMount>,
    /// name → (local_dir, mount_path)
    pub volume_mounts: HashMap<String, (PathBuf, String)>,
    /// Whether the snapshot needs the host bun/node binaries bind-mounted at runtime.
    pub needs_host_bun: bool,
}

pub struct OverlayMount {
    pub merged: PathBuf,
    pub upper: PathBuf,
    pub work: PathBuf,
    /// Whether fuse-overlayfs was used (affects unmount command).
    pub is_fuse: bool,
}

pub const VALID_AGENT_BINARIES: &[&str] = &["claude", "codex", "opencode"];

/// Returns `true` when the snapshot needs the host bun/node binaries at runtime.
/// `wmill` CLI and Claude Code / Codex are npm packages requiring a JS runtime.
/// OpenCode is a standalone Go binary — no runtime needed.
pub fn snapshot_needs_host_bun(include_wmill: bool, agent_binary: Option<&str>) -> bool {
    include_wmill || matches!(agent_binary, Some("claude") | Some("codex"))
}

/// Parse sandbox annotations from script content.
///
/// Supports comment styles: `#` (Python/bash) and `//` (TS/Go/Rust).
///
/// ```text
/// # sandbox: python-env:latest
/// # volume: data:/workspace/data
/// # volume: models:/workspace/models
///
/// // sandbox: node-env:v2
/// // volume: cache:/tmp/cache
/// ```
///
/// Tag defaults to `"latest"` if omitted: `# sandbox: python-env`
pub fn parse_sandbox_config(code: &str) -> SandboxConfig {
    let mut config = SandboxConfig::default();
    for line in code.lines() {
        let trimmed = line.trim();
        let content = trimmed
            .strip_prefix('#')
            .or_else(|| trimmed.strip_prefix("//"))
            .map(|s| s.trim());
        let Some(content) = content else {
            continue;
        };
        if let Some(spec) = content.strip_prefix("sandbox:").map(|s| s.trim()) {
            let (name, tag) = spec
                .split_once(':')
                .map(|(n, t)| (n.to_string(), t.to_string()))
                .unwrap_or_else(|| (spec.to_string(), "latest".to_string()));
            config.snapshot = Some(SnapshotRef { name, tag });
        } else if let Some(spec) = content.strip_prefix("volume:").map(|s| s.trim()) {
            if let Some((name, path)) = spec.split_once(':') {
                config.volumes.insert(name.to_string(), path.to_string());
            }
        } else if let Some(spec) = content.strip_prefix("allowed_domains:").map(|s| s.trim()) {
            config.allowed_domains = spec
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_python_annotations() {
        let code = "# sandbox: python-env:latest\n\
                     # volume: data:/workspace/data\n\
                     # volume: models:/workspace/models\n\
                     def main():\n    pass\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "python-env");
        assert_eq!(snap.tag, "latest");
        assert_eq!(config.volumes.len(), 2);
        assert_eq!(config.volumes["data"], "/workspace/data");
        assert_eq!(config.volumes["models"], "/workspace/models");
    }

    #[test]
    fn test_parse_ts_annotations() {
        let code = "// sandbox: node-env:v2\n\
                     // volume: cache:/tmp/cache\n\
                     export async function main() {}\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "node-env");
        assert_eq!(snap.tag, "v2");
        assert_eq!(config.volumes["cache"], "/tmp/cache");
    }

    #[test]
    fn test_parse_default_tag() {
        let code = "# sandbox: myenv\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "myenv");
        assert_eq!(snap.tag, "latest");
    }

    #[test]
    fn test_parse_no_annotations() {
        let code = "def main():\n    pass\n";
        let config = parse_sandbox_config(code);
        assert!(config.snapshot.is_none());
        assert!(config.volumes.is_empty());
    }

    #[test]
    fn test_parse_volumes_only() {
        let code = "# volume: data:/mnt/data\ndef main(): pass\n";
        let config = parse_sandbox_config(code);
        assert!(config.snapshot.is_none());
        assert_eq!(config.volumes.len(), 1);
        assert_eq!(config.volumes["data"], "/mnt/data");
    }

    #[test]
    fn test_parse_empty_code() {
        let config = parse_sandbox_config("");
        assert!(config.snapshot.is_none());
        assert!(config.volumes.is_empty());
    }

    #[test]
    fn test_parse_leading_whitespace() {
        let code = "  # sandbox: indented-env:v1\n  # volume: cache:/mnt/cache\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "indented-env");
        assert_eq!(snap.tag, "v1");
        assert_eq!(config.volumes["cache"], "/mnt/cache");
    }

    #[test]
    fn test_parse_mixed_comment_styles() {
        let code = "# sandbox: py-env:v3\n// volume: data:/mnt/data\n# volume: logs:/var/log\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "py-env");
        assert_eq!(snap.tag, "v3");
        assert_eq!(config.volumes.len(), 2);
        assert_eq!(config.volumes["data"], "/mnt/data");
        assert_eq!(config.volumes["logs"], "/var/log");
    }

    #[test]
    fn test_parse_last_sandbox_wins() {
        let code = "# sandbox: first:v1\n# sandbox: second:v2\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "second");
        assert_eq!(snap.tag, "v2");
    }

    #[test]
    fn test_parse_volume_path_with_nested_colons() {
        let code = "# volume: db:/opt/db:data\n";
        let config = parse_sandbox_config(code);
        assert_eq!(config.volumes["db"], "/opt/db:data");
    }

    #[test]
    fn test_parse_annotations_among_code() {
        let code = "import os\n\
                     # sandbox: ml-env:gpu\n\
                     # volume: models:/workspace/models\n\
                     \n\
                     def main():\n\
                         # This is a regular comment, not an annotation\n\
                         return os.getenv('PATH')\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "ml-env");
        assert_eq!(snap.tag, "gpu");
        assert_eq!(config.volumes.len(), 1);
        assert_eq!(config.volumes["models"], "/workspace/models");
    }

    #[test]
    fn test_parse_go_style() {
        let code = "// sandbox: go-env:1.22\n// volume: cache:/tmp/go-cache\npackage main\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "go-env");
        assert_eq!(snap.tag, "1.22");
        assert_eq!(config.volumes["cache"], "/tmp/go-cache");
    }

    #[test]
    fn test_parse_bash_style_snapshot_with_multiple_volumes() {
        let code = "#!/bin/bash\n\
                     # sandbox: custom-runtime\n\
                     # volume: input:/mnt/input\n\
                     # volume: output:/mnt/output\n\
                     # volume: scratch:/tmp/scratch\n\
                     echo 'hello'\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "custom-runtime");
        assert_eq!(snap.tag, "latest");
        assert_eq!(config.volumes.len(), 3);
    }

    #[test]
    fn test_parse_allowed_domains() {
        let code = "# sandbox: py-env:v1\n# allowed_domains: api.example.com, cdn.example.com\n";
        let config = parse_sandbox_config(code);
        assert_eq!(
            config.allowed_domains,
            vec!["api.example.com", "cdn.example.com"]
        );
    }

    #[test]
    fn test_parse_allowed_domains_ts_style() {
        let code = "// allowed_domains: ghcr.io, registry.npm.org\n// sandbox: node:v2\n";
        let config = parse_sandbox_config(code);
        assert_eq!(config.allowed_domains, vec!["ghcr.io", "registry.npm.org"]);
    }

    #[test]
    fn test_parse_allowed_domains_empty() {
        let code = "# allowed_domains:   \ndef main(): pass\n";
        let config = parse_sandbox_config(code);
        assert!(config.allowed_domains.is_empty());
    }

    #[test]
    fn test_parse_allowed_domains_case_insensitive() {
        let code = "# allowed_domains: API.Example.COM\n";
        let config = parse_sandbox_config(code);
        assert_eq!(config.allowed_domains, vec!["api.example.com"]);
    }

    #[test]
    fn test_parse_allowed_domains_with_sandbox_and_volume() {
        let code = "# sandbox: py-env:v1\n\
                     # volume: data:/mnt/data\n\
                     # allowed_domains: api.example.com, cdn.example.com\n\
                     def main(): pass\n";
        let config = parse_sandbox_config(code);
        assert!(config.snapshot.is_some());
        assert_eq!(config.volumes.len(), 1);
        assert_eq!(
            config.allowed_domains,
            vec!["api.example.com", "cdn.example.com"]
        );
    }

    #[test]
    fn test_parse_allowed_domains_last_wins() {
        let code = "# allowed_domains: first.com\n# allowed_domains: second.com, third.com\n";
        let config = parse_sandbox_config(code);
        assert_eq!(config.allowed_domains, vec!["second.com", "third.com"]);
    }

    #[test]
    fn test_parse_allowed_domains_standalone() {
        let code = "// allowed_domains: api.openai.com\nasync function main() { }";
        let config = parse_sandbox_config(code);
        assert!(config.snapshot.is_none());
        assert!(config.volumes.is_empty());
        assert_eq!(config.allowed_domains, vec!["api.openai.com"]);
    }

    #[test]
    fn test_parse_ignores_unrelated_comments() {
        let code = "# This is a normal comment\n\
                     # Another comment\n\
                     // Yet another\n\
                     # sandboxing is cool (not a directive)\n\
                     # volume: data:/mnt/data\n";
        let config = parse_sandbox_config(code);
        assert!(config.snapshot.is_none());
        assert_eq!(config.volumes.len(), 1);
    }

    // =========================================================================
    // Unit tests: snapshot_needs_host_bun
    // =========================================================================

    #[test]
    fn test_needs_host_bun_wmill_only() {
        assert!(snapshot_needs_host_bun(true, None));
    }

    #[test]
    fn test_needs_host_bun_claude() {
        assert!(snapshot_needs_host_bun(false, Some("claude")));
    }

    #[test]
    fn test_needs_host_bun_codex() {
        assert!(snapshot_needs_host_bun(false, Some("codex")));
    }

    #[test]
    fn test_needs_host_bun_opencode_does_not() {
        assert!(!snapshot_needs_host_bun(false, Some("opencode")));
    }

    #[test]
    fn test_needs_host_bun_nothing() {
        assert!(!snapshot_needs_host_bun(false, None));
    }

    #[test]
    fn test_needs_host_bun_wmill_and_claude() {
        assert!(snapshot_needs_host_bun(true, Some("claude")));
    }

    #[test]
    fn test_needs_host_bun_wmill_and_opencode() {
        // wmill alone requires bun, even though opencode doesn't
        assert!(snapshot_needs_host_bun(true, Some("opencode")));
    }

    #[test]
    fn test_valid_agent_binaries() {
        assert!(VALID_AGENT_BINARIES.contains(&"claude"));
        assert!(VALID_AGENT_BINARIES.contains(&"codex"));
        assert!(VALID_AGENT_BINARIES.contains(&"opencode"));
        assert!(!VALID_AGENT_BINARIES.contains(&"gemini"));
    }
}
