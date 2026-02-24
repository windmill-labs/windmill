mod volume_ee;
pub use volume_ee::*;

pub use object_store::ObjectStore as DynObjectStore;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub struct VolumeMount {
    pub name: String,
    pub target: String,
}

pub struct VolumeState {
    pub mount: VolumeMount,
    pub local_dir: PathBuf,
    pub manifest: HashMap<String, FileManifestEntry>,
}

pub struct FileManifestEntry {
    pub size: u64,
    pub modified: SystemTime,
}

pub struct SyncStats {
    pub new_size_bytes: u64,
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
