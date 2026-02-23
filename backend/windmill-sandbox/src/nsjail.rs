use crate::SandboxSetupState;

const OVERLAY_MARKER: &str = "# SANDBOX_OVERLAY_ACTIVE";
pub const NEEDS_HOST_BUN_MARKER: &str = "# SNAPSHOT_NEEDS_HOST_BUN";

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

    if setup.needs_host_bun {
        mounts.push_str(&format!("\n{NEEDS_HOST_BUN_MARKER}\n"));
    }

    mounts
}

/// Post-process an nsjail config string. If the overlay marker is present:
/// 1. Remove bind mounts for system directories (/bin, /lib, /lib64, /usr, /etc)
///    since the overlay root mount provides them.
/// 2. Move the overlay root mount (`dst: "/"`) to be the FIRST mount block.
///    nsjail applies mounts in order, so the root bind must come before
///    tmpfs/bind mounts that go on top of it.
/// 3. Re-add read-only bind mounts for `runtime_bins` paths that fall under
///    stripped system dirs (e.g. `/usr/bin/bun`). These are layered on top of
///    the overlay so the host runtime binary is available inside the sandbox.
/// If no overlay marker is present, returns the config unchanged.
pub fn finalize_nsjail_config(config: &str, runtime_bins: &[&str]) -> String {
    if !config.contains(OVERLAY_MARKER) {
        return config.to_string();
    }

    // If the host-bun marker is present, auto-add bun to runtime_bins.
    // We do NOT mount the host node binary — it has too many shared-lib
    // dependencies (libuv, libssl, libicu…) that won't exist in minimal
    // container images. Instead, a `node -> bun` symlink is baked into
    // the snapshot rootfs during build so `#!/usr/bin/env node` shebangs
    // resolve to bun (which is fully node-compatible and only needs glibc).
    let needs_host_bun = config.contains(NEEDS_HOST_BUN_MARKER);
    let bun_path = std::env::var("BUN_PATH").unwrap_or_else(|_| "/usr/bin/bun".to_string());
    let mut effective_bins: Vec<&str> = runtime_bins.to_vec();
    let bun_path_ref: &str;
    if needs_host_bun {
        bun_path_ref = &bun_path;
        if !effective_bins.contains(&bun_path_ref) {
            effective_bins.push(bun_path_ref);
        }
    }
    let runtime_bins = &effective_bins;

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

        if trimmed == OVERLAY_MARKER || trimmed == NEEDS_HOST_BUN_MARKER {
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
                    overlay_root_lines.extend_from_slice(&result_lines[mount_block_start..]);
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

    // Re-add bind mounts for runtime binaries that fall under stripped system dirs.
    // These are appended at the end so they layer on top of the overlay root.
    let runtime_mounts: String = runtime_bins
        .iter()
        .filter(|bin| system_dirs.iter().any(|d| bin.starts_with(d)))
        .map(|bin| {
            format!("\nmount {{\n    src: \"{bin}\"\n    dst: \"{bin}\"\n    is_bind: true\n}}")
        })
        .collect();

    let mut result = result_lines.join("\n");
    if !runtime_mounts.is_empty() {
        result.push_str(&runtime_mounts);
        result.push('\n');
    }
    result
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
            (
                PathBuf::from("/job/volumes/data"),
                "/workspace/data".to_string(),
            ),
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
            (
                PathBuf::from("/job/volumes/input"),
                "/mnt/input".to_string(),
            ),
        );
        setup.volume_mounts.insert(
            "output".to_string(),
            (
                PathBuf::from("/job/volumes/output"),
                "/mnt/output".to_string(),
            ),
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
            ..Default::default()
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
            ..Default::default()
        };
        setup.volume_mounts.insert(
            "data".to_string(),
            (
                PathBuf::from("/job/volumes/data"),
                "/workspace/data".to_string(),
            ),
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
        let result = finalize_nsjail_config(config, &[]);
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
        let result = finalize_nsjail_config(config, &[]);
        for dir in &["/bin", "/lib", "/lib64", "/usr", "/etc"] {
            assert!(
                !result.contains(&format!("dst: \"{dir}\"")),
                "System dir {dir} should be stripped"
            );
        }
        assert!(
            result.contains("dst: \"/\""),
            "Overlay root mount preserved"
        );
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
        let result = finalize_nsjail_config(config, &[]);
        assert!(!result.contains("dst: \"/bin\""), "/bin stripped");
        assert!(result.contains("dst: \"/dev/null\""), "/dev/null kept");
        assert!(
            result.contains("dst: \"/opt/microsoft\""),
            "/opt/microsoft kept"
        );
        assert!(result.contains("dst: \"/tmp\""), "/tmp tmpfs kept");
    }

    #[test]
    fn test_finalize_removes_overlay_marker_line() {
        let config = "name: \"test\"\n\
                       # SANDBOX_OVERLAY_ACTIVE\n\
                       mount {\n    src: \"/job/merged\"\n    dst: \"/\"\n    is_bind: true\n}\n";
        let result = finalize_nsjail_config(config, &[]);
        assert!(!result.contains(OVERLAY_MARKER));
        assert!(result.contains("dst: \"/\""));
    }

    #[test]
    fn test_finalize_readds_runtime_bins_under_stripped_dirs() {
        let config = "name: \"test\"\n\
                       mount {\n    src: \"/usr\"\n    dst: \"/usr\"\n    is_bind: true\n}\n\
                       # SANDBOX_OVERLAY_ACTIVE\n\
                       mount {\n    src: \"/job/merged\"\n    dst: \"/\"\n    is_bind: true\n    rw: true\n}\n";
        let result = finalize_nsjail_config(config, &["/usr/bin/bun"]);
        assert!(!result.contains("dst: \"/usr\""), "/usr stripped");
        assert!(result.contains("dst: \"/usr/bin/bun\""), "bun re-added");
        assert!(result.contains("src: \"/usr/bin/bun\""), "bun src set");
    }

    #[test]
    fn test_finalize_skips_runtime_bins_not_under_system_dirs() {
        let config = "name: \"test\"\n\
                       # SANDBOX_OVERLAY_ACTIVE\n\
                       mount {\n    src: \"/job/merged\"\n    dst: \"/\"\n    is_bind: true\n}\n";
        let result = finalize_nsjail_config(config, &["/tmp/windmill/cache/py_runtime"]);
        // Should NOT add an extra mount since /tmp is not a stripped system dir
        assert_eq!(result.matches("mount {").count(), 1);
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
            ..Default::default()
        };
        let mounts = build_sandbox_mounts(&setup);

        for block in mounts.split("mount {").skip(1) {
            assert!(block.contains('}'));
            assert!(block.contains("dst: \""));
            assert!(block.contains("is_bind: true"));
            assert!(block.contains("rw: true"));
        }
    }

    // =========================================================================
    // Unit tests: NEEDS_HOST_BUN_MARKER
    // =========================================================================

    #[test]
    fn test_build_sandbox_mounts_needs_host_bun_marker() {
        let setup = SandboxSetupState {
            overlay: Some(OverlayMount {
                merged: PathBuf::from("/job/merged"),
                upper: PathBuf::from("/job/upper"),
                work: PathBuf::from("/job/work"),
                is_fuse: false,
            }),
            volume_mounts: HashMap::new(),
            needs_host_bun: true,
        };
        let mounts = build_sandbox_mounts(&setup);
        assert!(mounts.contains(NEEDS_HOST_BUN_MARKER));
        assert!(mounts.contains(OVERLAY_MARKER));
    }

    #[test]
    fn test_build_sandbox_mounts_no_host_bun_marker_when_false() {
        let setup = SandboxSetupState {
            overlay: Some(OverlayMount {
                merged: PathBuf::from("/job/merged"),
                upper: PathBuf::from("/job/upper"),
                work: PathBuf::from("/job/work"),
                is_fuse: false,
            }),
            volume_mounts: HashMap::new(),
            needs_host_bun: false,
        };
        let mounts = build_sandbox_mounts(&setup);
        assert!(!mounts.contains(NEEDS_HOST_BUN_MARKER));
        assert!(mounts.contains(OVERLAY_MARKER));
    }

    #[test]
    fn test_finalize_strips_host_bun_marker_and_adds_bun() {
        let config = "name: \"test\"\n\
                       mount {\n    src: \"/usr\"\n    dst: \"/usr\"\n    is_bind: true\n}\n\
                       # SANDBOX_OVERLAY_ACTIVE\n\
                       # SNAPSHOT_NEEDS_HOST_BUN\n\
                       mount {\n    src: \"/job/merged\"\n    dst: \"/\"\n    is_bind: true\n    rw: true\n}\n";

        std::env::set_var("BUN_PATH", "/usr/bin/bun");

        let result = finalize_nsjail_config(config, &[]);

        assert!(!result.contains(NEEDS_HOST_BUN_MARKER));
        assert!(!result.contains(OVERLAY_MARKER));
        assert!(result.contains("dst: \"/\""), "overlay root preserved");
        assert!(!result.contains("dst: \"/usr\""), "/usr stripped");
        // Only bun should be auto-added (node is a symlink in the rootfs)
        assert!(
            result.contains("dst: \"/usr/bin/bun\""),
            "bun should be auto-added"
        );
        assert!(
            !result.contains("dst: \"/usr/bin/node\""),
            "node should NOT be mounted (symlink in rootfs instead)"
        );
    }

    #[test]
    fn test_finalize_host_bun_marker_does_not_duplicate_existing_bins() {
        let config = "name: \"test\"\n\
                       mount {\n    src: \"/usr\"\n    dst: \"/usr\"\n    is_bind: true\n}\n\
                       # SANDBOX_OVERLAY_ACTIVE\n\
                       # SNAPSHOT_NEEDS_HOST_BUN\n\
                       mount {\n    src: \"/job/merged\"\n    dst: \"/\"\n    is_bind: true\n    rw: true\n}\n";

        std::env::set_var("BUN_PATH", "/usr/bin/bun");

        // Pass bun as an explicit runtime bin too
        let result = finalize_nsjail_config(config, &["/usr/bin/bun"]);

        // bun should appear exactly once as a mount destination
        let bun_count = result.matches("dst: \"/usr/bin/bun\"").count();
        assert_eq!(bun_count, 1, "bun mount should not be duplicated");
    }

    #[test]
    fn test_finalize_without_host_bun_marker_no_auto_bins() {
        let config = "name: \"test\"\n\
                       mount {\n    src: \"/usr\"\n    dst: \"/usr\"\n    is_bind: true\n}\n\
                       # SANDBOX_OVERLAY_ACTIVE\n\
                       mount {\n    src: \"/job/merged\"\n    dst: \"/\"\n    is_bind: true\n    rw: true\n}\n";

        std::env::set_var("BUN_PATH", "/usr/bin/bun");

        let result = finalize_nsjail_config(config, &[]);

        // Without the host bun marker, bun should NOT be auto-added
        assert!(!result.contains("dst: \"/usr/bin/bun\""));
    }
}
