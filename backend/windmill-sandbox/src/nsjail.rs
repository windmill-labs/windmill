use crate::SandboxSetupState;

const OVERLAY_MARKER: &str = "# SANDBOX_OVERLAY_ACTIVE";

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

/// Post-process an nsjail config string. If the overlay marker is present:
/// 1. Remove bind mounts for system directories (/bin, /lib, /lib64, /usr, /etc)
///    since the overlay root mount provides them.
/// 2. Move the overlay root mount (`dst: "/"`) to be the FIRST mount block.
///    nsjail applies mounts in order, so the root bind must come before
///    tmpfs/bind mounts that go on top of it.
/// If no overlay marker is present, returns the config unchanged.
pub fn finalize_nsjail_config(config: &str) -> String {
    if !config.contains(OVERLAY_MARKER) {
        return config.to_string();
    }

    let system_dirs: &[&str] = &["/bin", "/lib", "/lib64", "/usr", "/etc"];
    let mut result_lines: Vec<&str> = Vec::new();
    let mut overlay_root_lines: Vec<&str> = Vec::new();
    let mut in_mount_block = false;
    let mut mount_block_start = 0;
    let mut should_remove_block = false;
    let mut is_overlay_root = false;

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
            is_overlay_root = false;
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
                    if dst == "/" {
                        is_overlay_root = true;
                    }
                }
            }

            if trimmed == "}" {
                if should_remove_block {
                    result_lines.truncate(mount_block_start);
                } else if is_overlay_root {
                    overlay_root_lines
                        .extend_from_slice(&result_lines[mount_block_start..]);
                    overlay_root_lines.push(lines[i]);
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

    // Insert overlay root mount as the FIRST mount block
    if !overlay_root_lines.is_empty() {
        let first_mount_pos = result_lines.iter().position(|line| {
            let t = line.trim();
            t.starts_with("mount {") || t == "mount {"
        });

        match first_mount_pos {
            Some(pos) => {
                for (j, line) in overlay_root_lines.iter().enumerate() {
                    result_lines.insert(pos + j, line);
                }
            }
            None => {
                result_lines.extend(overlay_root_lines.iter());
            }
        }
    }

    result_lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OverlayMount;
    use std::collections::HashMap;
    use std::path::PathBuf;

    // =========================================================================
    // Unit tests: build_sandbox_mounts
    // =========================================================================

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
                is_fuse: false,
            }),
            volume_mounts: HashMap::new(),
        };
        let mounts = build_sandbox_mounts(&setup);
        assert!(mounts.contains(OVERLAY_MARKER));
        assert!(mounts.contains("src: \"/job/overlay_merged\""));
        assert!(mounts.contains("dst: \"/\""));
    }

    #[test]
    fn test_build_sandbox_mounts_overlay_and_volumes() {
        let mut setup = SandboxSetupState {
            overlay: Some(OverlayMount {
                merged: PathBuf::from("/job/overlay_merged"),
                upper: PathBuf::from("/job/overlay_upper"),
                work: PathBuf::from("/job/overlay_work"),
                is_fuse: false,
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

    // =========================================================================
    // Unit tests: finalize_nsjail_config
    // =========================================================================

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
                       mount {\n    src: \"/opt/microsoft\"\n    dst: \"/opt/microsoft\"\n    is_bind: true\n}\n\
                       mount {\n    dst: \"/tmp\"\n    fstype: \"tmpfs\"\n    rw: true\n}\n\
                       # SANDBOX_OVERLAY_ACTIVE\n\
                       mount {\n    src: \"/job/merged\"\n    dst: \"/\"\n    is_bind: true\n    rw: true\n}\n";
        let result = finalize_nsjail_config(config);
        assert!(!result.contains("dst: \"/bin\""), "/bin stripped");
        assert!(result.contains("dst: \"/dev/null\""), "/dev/null kept");
        assert!(result.contains("dst: \"/opt/microsoft\""), "/opt/microsoft kept");
        assert!(result.contains("dst: \"/tmp\""), "/tmp tmpfs kept");
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

    #[test]
    fn test_mount_block_syntax() {
        let setup = SandboxSetupState {
            overlay: Some(OverlayMount {
                merged: PathBuf::from("/job/merged"),
                upper: PathBuf::from("/job/upper"),
                work: PathBuf::from("/job/work"),
                is_fuse: false,
            }),
            volume_mounts: HashMap::from([(
                "vol".to_string(),
                (PathBuf::from("/job/volumes/vol"), "/mnt/vol".to_string()),
            )]),
        };
        let mounts = build_sandbox_mounts(&setup);

        for block in mounts.split("mount {").skip(1) {
            assert!(block.contains('}'));
            assert!(block.contains("dst: \""));
            assert!(block.contains("is_bind: true"));
            assert!(block.contains("rw: true"));
        }
    }
}
