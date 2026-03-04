#[cfg(feature = "private")]
mod volume_ee;
mod volume_oss;
pub use volume_oss::*;

pub use object_store::ObjectStore as DynObjectStore;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;

pub const MAX_VOLUMES_PER_JOB: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub md5: Option<String>,
}

pub fn compute_md5_hex(data: &[u8]) -> String {
    use md5::{Digest, Md5};
    let result = Md5::digest(data);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut hex = String::with_capacity(32);
    for &b in result.iter() {
        hex.push(HEX[(b >> 4) as usize] as char);
        hex.push(HEX[(b & 0x0f) as usize] as char);
    }
    hex
}

/// Extract an MD5 hash from an S3 ETag, if it's a simple (non-multipart) ETag.
pub fn etag_to_md5(e_tag: Option<&str>) -> Option<String> {
    let tag = e_tag?.trim_matches('"');
    // Multipart ETags contain a '-' (e.g. "abc123-5"), skip those
    if tag.contains('-') || tag.is_empty() {
        return None;
    }
    Some(tag.to_string())
}

lazy_static::lazy_static! {
    static ref ARGS_INTERPOLATION_RE: regex::Regex =
        regex::Regex::new(r#"\$args\[((?:\w+\.)*\w+)\]"#).unwrap();
    static ref VALID_VOLUME_NAME_RE: regex::Regex =
        regex::Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9._-]{0,253}[a-zA-Z0-9]$").unwrap();
}

#[derive(Debug, Clone, PartialEq)]
pub struct VolumeMount {
    pub name: String,
    pub target: String,
}

pub struct VolumeState {
    pub mount: VolumeMount,
    pub local_dir: PathBuf,
    pub manifest: HashMap<String, FileEntry>,
    pub symlinks: HashMap<String, String>,
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

pub fn validate_volume_name(name: &str) -> Result<(), String> {
    if name.contains("..") {
        return Err(format!(
            "Volume name '{}' contains '..' which is not allowed",
            name
        ));
    }
    if !VALID_VOLUME_NAME_RE.is_match(name) {
        return Err(format!(
            "Volume name '{}' is invalid. Names must be 2-255 characters, \
             start and end with alphanumeric, and contain only alphanumeric, '.', '_', or '-'",
            name
        ));
    }
    Ok(())
}

const ALLOWED_ABSOLUTE_PREFIXES: &[&str] = &["/tmp/", "/mnt/", "/opt/", "/home/", "/data/"];

pub fn validate_volume_target(target: &str) -> Result<(), String> {
    if target.split('/').any(|seg| seg == "..") {
        return Err(format!(
            "Volume target '{target}' contains '..' segments which is not allowed"
        ));
    }
    if target.starts_with('/')
        && !ALLOWED_ABSOLUTE_PREFIXES
            .iter()
            .any(|p| target.starts_with(p))
    {
        return Err(format!(
            "Volume target '{target}' must be a relative path or start with one of: {}",
            ALLOWED_ABSOLUTE_PREFIXES.join(", ")
        ));
    }
    Ok(())
}

pub fn validate_volume_mounts(mounts: &[VolumeMount]) -> Result<(), String> {
    if mounts.len() > MAX_VOLUMES_PER_JOB {
        return Err(format!(
            "Too many volume mounts ({}, max {})",
            mounts.len(),
            MAX_VOLUMES_PER_JOB
        ));
    }
    let mut seen_names = HashSet::new();
    let mut seen_targets = HashSet::new();
    for v in mounts {
        if !seen_names.insert(&v.name) {
            return Err(format!("Duplicate volume name: '{}'", v.name));
        }
        if !seen_targets.insert(&v.target) {
            return Err(format!("Duplicate volume target: '{}'", v.target));
        }
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
    let mut result = name.clone();
    for cap in ARGS_INTERPOLATION_RE.captures_iter(&name) {
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

    #[test]
    fn validate_valid_names() {
        assert!(validate_volume_name("mydata").is_ok());
        assert!(validate_volume_name("my-data_v2").is_ok());
        assert!(validate_volume_name("acme-staging-cache").is_ok());
        assert!(validate_volume_name("a1").is_ok());
        assert!(validate_volume_name("data.v2").is_ok());
        assert!(validate_volume_name("A0").is_ok());
    }

    #[test]
    fn validate_rejects_path_traversal() {
        assert!(validate_volume_name("../other-workspace").is_err());
        assert!(validate_volume_name("data/../secrets").is_err());
        assert!(validate_volume_name("a..b").is_err());
    }

    #[test]
    fn validate_rejects_special_start_end() {
        assert!(validate_volume_name("-data").is_err());
        assert!(validate_volume_name("data-").is_err());
        assert!(validate_volume_name(".data").is_err());
        assert!(validate_volume_name("data.").is_err());
        assert!(validate_volume_name("_data").is_err());
    }

    #[test]
    fn validate_rejects_path_separators() {
        assert!(validate_volume_name("data/secrets").is_err());
        assert!(validate_volume_name("data\\secrets").is_err());
    }

    #[test]
    fn validate_rejects_too_short() {
        assert!(validate_volume_name("").is_err());
        assert!(validate_volume_name("a").is_err());
    }

    #[test]
    fn validate_rejects_too_long() {
        let long_name = format!("a{}a", "b".repeat(254));
        assert!(validate_volume_name(&long_name).is_err());
    }

    #[test]
    fn validate_rejects_spaces_and_special() {
        assert!(validate_volume_name("my data").is_err());
        assert!(validate_volume_name("my@data").is_err());
        assert!(validate_volume_name("my$data").is_err());
    }

    #[test]
    fn validate_target_allows_relative() {
        assert!(validate_volume_target("data").is_ok());
        assert!(validate_volume_target("data/models").is_ok());
        assert!(validate_volume_target(".claude").is_ok());
    }

    #[test]
    fn validate_target_allows_safe_absolute() {
        assert!(validate_volume_target("/tmp/data").is_ok());
        assert!(validate_volume_target("/mnt/data").is_ok());
        assert!(validate_volume_target("/opt/models").is_ok());
        assert!(validate_volume_target("/home/user/data").is_ok());
        assert!(validate_volume_target("/data/cache").is_ok());
    }

    #[test]
    fn validate_target_rejects_dangerous_absolute() {
        assert!(validate_volume_target("/etc/passwd").is_err());
        assert!(validate_volume_target("/proc/self").is_err());
        assert!(validate_volume_target("/sys/fs").is_err());
        assert!(validate_volume_target("/dev/null").is_err());
        assert!(validate_volume_target("/usr/bin").is_err());
        assert!(validate_volume_target("/var/log").is_err());
    }

    #[test]
    fn validate_target_rejects_traversal() {
        assert!(validate_volume_target("../../etc").is_err());
        assert!(validate_volume_target("data/../../../etc").is_err());
        assert!(validate_volume_target("/tmp/../etc/passwd").is_err());
    }

    #[test]
    fn validate_mounts_rejects_too_many() {
        let mounts: Vec<VolumeMount> = (0..11)
            .map(|i| VolumeMount { name: format!("v{:02}", i), target: format!("t{}", i) })
            .collect();
        assert!(validate_volume_mounts(&mounts).is_err());
    }

    #[test]
    fn validate_mounts_rejects_duplicate_name() {
        let mounts = vec![
            VolumeMount { name: "data".to_string(), target: "/tmp/a".to_string() },
            VolumeMount { name: "data".to_string(), target: "/tmp/b".to_string() },
        ];
        assert!(validate_volume_mounts(&mounts).is_err());
    }

    #[test]
    fn validate_mounts_rejects_duplicate_target() {
        let mounts = vec![
            VolumeMount { name: "v1".to_string(), target: "/tmp/data".to_string() },
            VolumeMount { name: "v2".to_string(), target: "/tmp/data".to_string() },
        ];
        assert!(validate_volume_mounts(&mounts).is_err());
    }

    #[test]
    fn validate_mounts_ok() {
        let mounts = vec![
            VolumeMount { name: "v1".to_string(), target: "/tmp/a".to_string() },
            VolumeMount { name: "v2".to_string(), target: "/tmp/b".to_string() },
        ];
        assert!(validate_volume_mounts(&mounts).is_ok());
    }
}
