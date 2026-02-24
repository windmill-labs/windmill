use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct SandboxConfig {
    pub snapshot: Option<SnapshotRef>,
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
}

#[derive(Default)]
pub struct SandboxSetupState {
    pub overlay: Option<OverlayMount>,
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

/// Returns `true` when the snapshot needs host binaries bind-mounted at runtime.
/// This mounts the host bun binary and the wmill CLI (with its node_modules)
/// into the sandbox so they're available regardless of the base Docker image.
pub fn snapshot_needs_host_bun() -> bool {
    true
}

/// Parse sandbox annotations from script content.
///
/// Supports comment styles: `#` (Python/bash) and `//` (TS/Go/Rust).
///
/// ```text
/// # sandbox: python-env:latest
///
/// // sandbox: node-env:v2
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
                     def main():\n    pass\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "python-env");
        assert_eq!(snap.tag, "latest");
    }

    #[test]
    fn test_parse_ts_annotations() {
        let code = "// sandbox: node-env:v2\n\
                     export async function main() {}\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "node-env");
        assert_eq!(snap.tag, "v2");
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
    }

    #[test]
    fn test_parse_empty_code() {
        let config = parse_sandbox_config("");
        assert!(config.snapshot.is_none());
    }

    #[test]
    fn test_parse_leading_whitespace() {
        let code = "  # sandbox: indented-env:v1\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "indented-env");
        assert_eq!(snap.tag, "v1");
    }

    #[test]
    fn test_parse_mixed_comment_styles() {
        let code = "# sandbox: py-env:v3\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "py-env");
        assert_eq!(snap.tag, "v3");
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
    fn test_parse_annotations_among_code() {
        let code = "import os\n\
                     # sandbox: ml-env:gpu\n\
                     \n\
                     def main():\n\
                         # This is a regular comment, not an annotation\n\
                         return os.getenv('PATH')\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "ml-env");
        assert_eq!(snap.tag, "gpu");
    }

    #[test]
    fn test_parse_go_style() {
        let code = "// sandbox: go-env:1.22\npackage main\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "go-env");
        assert_eq!(snap.tag, "1.22");
    }

    #[test]
    fn test_parse_bash_style_snapshot() {
        let code = "#!/bin/bash\n\
                     # sandbox: custom-runtime\n\
                     echo 'hello'\n";
        let config = parse_sandbox_config(code);
        let snap = config.snapshot.unwrap();
        assert_eq!(snap.name, "custom-runtime");
        assert_eq!(snap.tag, "latest");
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
    fn test_parse_allowed_domains_with_sandbox() {
        let code = "# sandbox: py-env:v1\n\
                     # allowed_domains: api.example.com, cdn.example.com\n\
                     def main(): pass\n";
        let config = parse_sandbox_config(code);
        assert!(config.snapshot.is_some());
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
        assert_eq!(config.allowed_domains, vec!["api.openai.com"]);
    }

    #[test]
    fn test_parse_ignores_unrelated_comments() {
        let code = "# This is a normal comment\n\
                     # Another comment\n\
                     // Yet another\n\
                     # sandboxing is cool (not a directive)\n";
        let config = parse_sandbox_config(code);
        assert!(config.snapshot.is_none());
    }

    // =========================================================================
    // Unit tests: snapshot_needs_host_bun
    // =========================================================================

    #[test]
    fn test_needs_host_bun_always_true() {
        // wmill CLI is always in the image and requires bun
        assert!(snapshot_needs_host_bun());
    }
}
