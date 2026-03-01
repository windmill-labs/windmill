#[cfg(feature = "private")]
mod volume_ee;
mod volume_oss;
pub use volume_oss::*;

pub use object_store::ObjectStore as DynObjectStore;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub struct VolumeMount {
    pub name: String,
    pub target: String,
}

pub struct VolumeState {
    pub mount: VolumeMount,
    pub local_dir: PathBuf,
    pub manifest: HashMap<String, u64>,
    pub symlinks: HashMap<String, String>,
}

pub struct FileManifestEntry {
    pub size: u64,
    pub modified: SystemTime,
}

pub struct DownloadStats {
    pub total_files: usize,
    pub from_cache: usize,
    pub downloaded: usize,
}

pub struct SyncStats {
    pub new_size_bytes: u64,
    pub file_count: usize,
    pub uploaded: usize,
    pub skipped: usize,
}

pub static VOLUME_CACHE: Mutex<VolumeCache> = Mutex::new(VolumeCache::new());

const MAX_CACHE_BYTES: u64 = 10 * 1024 * 1024 * 1024; // 10 GB

pub struct VolumeCache {
    /// (workspace_id, volume_name) -> (last_access_time, size_bytes)
    index: Vec<VolumeCacheEntry>,
    base_dir: Option<PathBuf>,
}

struct VolumeCacheEntry {
    workspace_id: String,
    volume_name: String,
    last_access: SystemTime,
    size_bytes: u64,
}

impl VolumeCache {
    pub const fn new() -> Self {
        VolumeCache { index: Vec::new(), base_dir: None }
    }

    fn base_dir(&self) -> PathBuf {
        self.base_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("/tmp/windmill/volume_cache"))
    }

    pub fn volume_cache_dir(&self, workspace_id: &str, volume_name: &str) -> PathBuf {
        self.base_dir().join(workspace_id).join(volume_name)
    }

    fn manifest_path(&self, workspace_id: &str, volume_name: &str) -> PathBuf {
        self.base_dir()
            .join(workspace_id)
            .join(format!("{volume_name}.manifest.json"))
    }

    fn symlinks_path(&self, workspace_id: &str, volume_name: &str) -> PathBuf {
        self.base_dir()
            .join(workspace_id)
            .join(format!("{volume_name}.symlinks.json"))
    }

    pub fn load_cached_manifest(
        &self,
        workspace_id: &str,
        volume_name: &str,
    ) -> Option<HashMap<String, u64>> {
        let path = self.manifest_path(workspace_id, volume_name);
        let data = std::fs::read(&path).ok()?;
        serde_json::from_slice(&data).ok()
    }

    pub fn load_cached_symlinks(
        &self,
        workspace_id: &str,
        volume_name: &str,
    ) -> HashMap<String, String> {
        let path = self.symlinks_path(workspace_id, volume_name);
        std::fs::read(&path)
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default()
    }

    pub fn save_manifest(
        &self,
        workspace_id: &str,
        volume_name: &str,
        manifest: &HashMap<String, u64>,
    ) {
        let path = self.manifest_path(workspace_id, volume_name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Ok(data) = serde_json::to_vec(manifest) {
            std::fs::write(&path, data).ok();
        }
    }

    pub fn save_symlinks(
        &self,
        workspace_id: &str,
        volume_name: &str,
        symlinks: &HashMap<String, String>,
    ) {
        let path = self.symlinks_path(workspace_id, volume_name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Ok(data) = serde_json::to_vec(symlinks) {
            std::fs::write(&path, data).ok();
        }
    }

    pub fn touch(&mut self, workspace_id: &str, volume_name: &str, size_bytes: u64) {
        let now = SystemTime::now();
        if let Some(entry) = self
            .index
            .iter_mut()
            .find(|e| e.workspace_id == workspace_id && e.volume_name == volume_name)
        {
            entry.last_access = now;
            entry.size_bytes = size_bytes;
        } else {
            self.index.push(VolumeCacheEntry {
                workspace_id: workspace_id.to_string(),
                volume_name: volume_name.to_string(),
                last_access: now,
                size_bytes,
            });
        }
    }

    pub fn evict_if_needed(&mut self) {
        let total: u64 = self.index.iter().map(|e| e.size_bytes).sum();
        if total <= MAX_CACHE_BYTES {
            return;
        }

        self.index.sort_by_key(|e| e.last_access);

        let mut current = total;
        while current > MAX_CACHE_BYTES {
            if let Some(entry) = self.index.first() {
                let dir = self.volume_cache_dir(&entry.workspace_id, &entry.volume_name);
                std::fs::remove_dir_all(&dir).ok();
                // Also remove manifest/symlinks files
                let manifest = self.manifest_path(&entry.workspace_id, &entry.volume_name);
                std::fs::remove_file(&manifest).ok();
                let symlinks = self.symlinks_path(&entry.workspace_id, &entry.volume_name);
                std::fs::remove_file(&symlinks).ok();

                current = current.saturating_sub(entry.size_bytes);
                self.index.remove(0);
            } else {
                break;
            }
        }
    }
}

/// Walk a directory recursively, returning paths to regular files only (not symlinks or dirs).
pub fn walk_dir(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    walk_dir_inner(dir, &mut result)?;
    Ok(result)
}

fn walk_dir_inner(dir: &Path, result: &mut Vec<PathBuf>) -> std::io::Result<()> {
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
        // Skip symlinks — they are handled separately
    }
    Ok(())
}

/// Collect all symlinks in a directory tree, returning a map of relative path → target.
pub fn collect_symlinks(dir: &Path) -> HashMap<String, String> {
    let mut symlinks = HashMap::new();
    collect_symlinks_inner(dir, dir, &mut symlinks);
    symlinks
}

fn collect_symlinks_inner(base: &Path, dir: &Path, symlinks: &mut HashMap<String, String>) {
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

/// Restore symlinks in a directory from a symlink map.
pub fn restore_symlinks(dir: &Path, symlinks: &HashMap<String, String>) {
    for (relative, target) in symlinks {
        let link_path = dir.join(relative);
        if let Some(parent) = link_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        // Remove any existing file/dir at the link path
        if link_path.exists() || link_path.symlink_metadata().is_ok() {
            std::fs::remove_file(&link_path).ok();
            std::fs::remove_dir_all(&link_path).ok();
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, &link_path).ok();
        }
    }
}

/// Recursively copy a directory tree.
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !src.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let meta = std::fs::symlink_metadata(&src_path)?;
        if meta.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if meta.is_file() {
            std::fs::copy(&src_path, &dst_path)?;
        }
        // Skip symlinks during copy — they are restored separately
    }
    Ok(())
}

pub fn interpolate_volume_name(
    name: &str,
    args: Option<&HashMap<String, Box<serde_json::value::RawValue>>>,
    workspace_id: &str,
) -> String {
    let name = name.replace("$workspace", workspace_id);
    if !name.contains("$args[") {
        return name;
    }
    let Some(args) = args else {
        return name;
    };
    let re = regex::Regex::new(r#"\$args\[((?:\w+\.)*\w+)\]"#).unwrap();
    let mut result = name.clone();
    for cap in re.captures_iter(&name) {
        let full_match = cap.get(0).unwrap().as_str();
        let arg_name = cap.get(1).unwrap().as_str();
        let arg_value = if arg_name.contains('.') {
            let parts: Vec<&str> = arg_name.split('.').collect();
            let root = parts[0];
            let mut value = args
                .get(root)
                .map(|x| x.get().to_string())
                .unwrap_or_default();
            for part in parts.iter().skip(1) {
                if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&value) {
                    value = obj
                        .get(part)
                        .map(|v| v.to_string())
                        .unwrap_or_default()
                        .to_string();
                } else {
                    value = String::new();
                    break;
                }
            }
            value.trim_matches('"').to_string()
        } else {
            args.get(arg_name)
                .map(|x| x.get().trim_matches('"').to_string())
                .unwrap_or_default()
        };
        result = result.replace(full_match, &arg_value);
    }
    result
}

pub fn parse_volume_annotations(content: &str, comment_prefix: &str) -> Vec<VolumeMount> {
    let mut volumes = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !trimmed.starts_with(comment_prefix) {
            break;
        }
        let after_prefix = trimmed[comment_prefix.len()..].trim();
        if let Some(rest) = after_prefix.strip_prefix("volume:") {
            let rest = rest.trim();
            let mut parts = rest.splitn(2, char::is_whitespace);
            if let (Some(name), Some(target)) = (parts.next(), parts.next()) {
                let name = name.trim();
                let target = target.trim();
                if !name.is_empty() && !target.is_empty() {
                    volumes
                        .push(VolumeMount { name: name.to_string(), target: target.to_string() });
                }
            }
        }
    }
    volumes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_python_single_volume() {
        let content = "# volume: mydata /tmp/data\ndef main():\n    pass";
        let result = parse_volume_annotations(content, "#");
        assert_eq!(
            result,
            vec![VolumeMount { name: "mydata".to_string(), target: "/tmp/data".to_string() }]
        );
    }

    #[test]
    fn parse_typescript_single_volume() {
        let content = "// volume: mydata /tmp/data\nexport function main() {}";
        let result = parse_volume_annotations(content, "//");
        assert_eq!(
            result,
            vec![VolumeMount { name: "mydata".to_string(), target: "/tmp/data".to_string() }]
        );
    }

    #[test]
    fn parse_multiple_volumes() {
        let content = "# volume: data1 /tmp/data1\n# volume: data2 /tmp/data2\n# volume: models /opt/models\ndef main():\n    pass";
        let result = parse_volume_annotations(content, "#");
        assert_eq!(
            result,
            vec![
                VolumeMount { name: "data1".to_string(), target: "/tmp/data1".to_string() },
                VolumeMount { name: "data2".to_string(), target: "/tmp/data2".to_string() },
                VolumeMount { name: "models".to_string(), target: "/opt/models".to_string() },
            ]
        );
    }

    #[test]
    fn parse_mixed_annotations_and_volumes() {
        let content = "# sandbox\n# volume: mydata /tmp/data\ndef main():\n    pass";
        let result = parse_volume_annotations(content, "#");
        assert_eq!(
            result,
            vec![VolumeMount { name: "mydata".to_string(), target: "/tmp/data".to_string() }]
        );
    }

    #[test]
    fn parse_no_volumes() {
        let content = "# sandbox\ndef main():\n    pass";
        let result = parse_volume_annotations(content, "#");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_empty_content() {
        let result = parse_volume_annotations("", "#");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_stops_at_non_comment_line() {
        let content =
            "# volume: data1 /tmp/data1\ndef main():\n    # volume: data2 /tmp/data2\n    pass";
        let result = parse_volume_annotations(content, "#");
        assert_eq!(
            result,
            vec![VolumeMount { name: "data1".to_string(), target: "/tmp/data1".to_string() }]
        );
    }

    #[test]
    fn parse_skips_blank_lines_in_header() {
        let content =
            "# volume: data1 /tmp/data1\n\n# volume: data2 /tmp/data2\ndef main():\n    pass";
        let result = parse_volume_annotations(content, "#");
        assert_eq!(
            result,
            vec![
                VolumeMount { name: "data1".to_string(), target: "/tmp/data1".to_string() },
                VolumeMount { name: "data2".to_string(), target: "/tmp/data2".to_string() },
            ]
        );
    }

    #[test]
    fn parse_ignores_malformed_volume_lines() {
        let content =
            "# volume:\n# volume: onlyname\n# volume: good /tmp/good\ndef main():\n    pass";
        let result = parse_volume_annotations(content, "#");
        assert_eq!(
            result,
            vec![VolumeMount { name: "good".to_string(), target: "/tmp/good".to_string() }]
        );
    }

    #[test]
    fn parse_extra_whitespace() {
        let content = "#   volume:   mydata   /tmp/data  \ndef main():\n    pass";
        let result = parse_volume_annotations(content, "#");
        assert_eq!(
            result,
            vec![VolumeMount { name: "mydata".to_string(), target: "/tmp/data".to_string() }]
        );
    }

    #[test]
    fn parse_target_with_spaces_in_path() {
        let content = "// volume: mydata /tmp/my data dir\nexport function main() {}";
        let result = parse_volume_annotations(content, "//");
        assert_eq!(
            result,
            vec![VolumeMount {
                name: "mydata".to_string(),
                target: "/tmp/my data dir".to_string(),
            }]
        );
    }

    #[test]
    fn parse_volume_with_dashes_and_underscores() {
        let content = "# volume: my-data_v2 /tmp/data\ndef main():\n    pass";
        let result = parse_volume_annotations(content, "#");
        assert_eq!(
            result,
            vec![VolumeMount { name: "my-data_v2".to_string(), target: "/tmp/data".to_string() }]
        );
    }

    #[test]
    fn interpolate_workspace() {
        let name = "$workspace-data";
        let result = interpolate_volume_name(name, None, "my_ws");
        assert_eq!(result, "my_ws-data");
    }

    #[test]
    fn interpolate_args_simple() {
        let mut args = HashMap::new();
        args.insert(
            "env".to_string(),
            serde_json::value::RawValue::from_string("\"prod\"".to_string()).unwrap(),
        );
        let result = interpolate_volume_name("data-$args[env]", Some(&args), "ws");
        assert_eq!(result, "data-prod");
    }

    #[test]
    fn interpolate_args_and_workspace() {
        let mut args = HashMap::new();
        args.insert(
            "env".to_string(),
            serde_json::value::RawValue::from_string("\"staging\"".to_string()).unwrap(),
        );
        let result = interpolate_volume_name("$workspace-$args[env]-cache", Some(&args), "acme");
        assert_eq!(result, "acme-staging-cache");
    }

    #[test]
    fn interpolate_no_placeholders() {
        let result = interpolate_volume_name("plain-name", None, "ws");
        assert_eq!(result, "plain-name");
    }

    #[test]
    fn interpolate_missing_arg() {
        let args = HashMap::new();
        let result = interpolate_volume_name("data-$args[missing]", Some(&args), "ws");
        assert_eq!(result, "data-");
    }

    #[test]
    fn interpolate_nested_arg() {
        let mut args = HashMap::new();
        args.insert(
            "config".to_string(),
            serde_json::value::RawValue::from_string(
                r#"{"env": "prod", "region": "us-east"}"#.to_string(),
            )
            .unwrap(),
        );
        let result = interpolate_volume_name(
            "data-$args[config.env]-$args[config.region]",
            Some(&args),
            "ws",
        );
        assert_eq!(result, "data-prod-us-east");
    }

    #[test]
    fn parse_wrong_prefix_returns_empty() {
        let content = "# volume: mydata /tmp/data\ndef main():\n    pass";
        let result = parse_volume_annotations(content, "//");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_relative_path() {
        let content = "// volume: agent-memory .claude\nexport function main() {}";
        let result = parse_volume_annotations(content, "//");
        assert_eq!(
            result,
            vec![VolumeMount { name: "agent-memory".to_string(), target: ".claude".to_string() }]
        );
    }

    #[test]
    fn parse_relative_nested_path() {
        let content = "# volume: data data/models\ndef main():\n    pass";
        let result = parse_volume_annotations(content, "#");
        assert_eq!(
            result,
            vec![VolumeMount { name: "data".to_string(), target: "data/models".to_string() }]
        );
    }
}
