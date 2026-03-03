#[cfg(feature = "private")]
mod volume_ee;
mod volume_oss;
pub use volume_oss::*;

pub use object_store::ObjectStore as DynObjectStore;

use std::collections::HashMap;
use std::path::PathBuf;

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
