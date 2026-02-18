// Sandbox setup logic has moved to the `windmill-sandbox` crate.
// This module only contains integration tests that need access to
// the nsjail proto templates in ../nsjail/.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use windmill_sandbox::{
        build_sandbox_mounts, finalize_nsjail_config, parse_sandbox_config, OverlayMount,
        SandboxSetupState,
    };

    // =========================================================================
    // Config pipeline tests: annotation → mounts → real nsjail proto template
    // =========================================================================

    #[test]
    fn test_config_pipeline_bash_snapshot_and_volumes() {
        let bash_script = "#!/bin/bash\n\
                           # sandbox: python-ml:gpu\n\
                           # volume: data:/workspace/data\n\
                           # volume: models:/workspace/models\n\
                           echo 'running ML pipeline'\n";

        let sandbox_config = parse_sandbox_config(bash_script);
        let snap = sandbox_config.snapshot.as_ref().unwrap();
        assert_eq!(snap.name, "python-ml");
        assert_eq!(snap.tag, "gpu");
        assert_eq!(sandbox_config.volumes.len(), 2);

        let setup = SandboxSetupState {
            overlay: Some(OverlayMount {
                merged: PathBuf::from("/tmp/job123/overlay_merged"),
                upper: PathBuf::from("/tmp/job123/overlay_upper"),
                work: PathBuf::from("/tmp/job123/overlay_work"),
                is_fuse: false,
            }),
            volume_mounts: HashMap::from([
                (
                    "data".to_string(),
                    (PathBuf::from("/tmp/job123/volumes/data"), "/workspace/data".to_string()),
                ),
                (
                    "models".to_string(),
                    (PathBuf::from("/tmp/job123/volumes/models"), "/workspace/models".to_string()),
                ),
            ]),
        };

        let sandbox_mounts = build_sandbox_mounts(&setup);
        let raw_config = include_str!("../nsjail/run.bash.config.proto");
        let config = raw_config
            .replace("{JOB_DIR}", "/tmp/job123")
            .replace("{CLONE_NEWUSER}", "true")
            .replace("{SHARED_MOUNT}", &sandbox_mounts)
            .replace("{TRACING_PROXY_CA_CERT_PATH}", "/dev/null");

        let final_config = finalize_nsjail_config(&config);

        for dir in &["/bin", "/lib", "/lib64", "/usr", "/etc"] {
            assert!(!final_config.contains(&format!("dst: \"{dir}\"")));
        }
        assert!(final_config.contains("dst: \"/\""));
        assert!(final_config.contains("src: \"/tmp/job123/overlay_merged\""));
        assert!(final_config.contains("dst: \"/workspace/data\""));
        assert!(final_config.contains("dst: \"/workspace/models\""));
        assert!(final_config.contains("dst: \"/tmp\""));
        assert!(final_config.contains("dst: \"/dev/null\""));
        assert!(final_config.contains("dst: \"/tmp/main.sh\""));
        assert!(final_config.contains("name: \"bash run script\""));
    }

    #[test]
    fn test_config_pipeline_python_snapshot_only() {
        let setup = SandboxSetupState {
            overlay: Some(OverlayMount {
                merged: PathBuf::from("/tmp/jobXYZ/overlay_merged"),
                upper: PathBuf::from("/tmp/jobXYZ/overlay_upper"),
                work: PathBuf::from("/tmp/jobXYZ/overlay_work"),
                is_fuse: false,
            }),
            volume_mounts: HashMap::new(),
        };

        let sandbox_mounts = build_sandbox_mounts(&setup);
        let raw_config = include_str!("../nsjail/run.python3.config.proto");
        let config = raw_config
            .replace("{JOB_DIR}", "/tmp/jobXYZ")
            .replace("{MAIN}", "main")
            .replace("{CLONE_NEWUSER}", "true")
            .replace("{SHARED_MOUNT}", &sandbox_mounts)
            .replace("{SHARED_DEPENDENCIES}", "")
            .replace("{PY_INSTALL_DIR}", "/usr/local")
            .replace("{GLOBAL_SITE_PACKAGES}", "/usr/lib/python3/dist-packages")
            .replace("{ADDITIONAL_PYTHON_PATHS}", "/tmp")
            .replace("{TRACING_PROXY_CA_CERT_PATH}", "/dev/null");

        let final_config = finalize_nsjail_config(&config);

        for dir in &["/bin", "/lib", "/lib64", "/usr", "/etc"] {
            assert!(!final_config.contains(&format!("dst: \"{dir}\"")));
        }
        assert!(final_config.contains("dst: \"/\""));
        assert!(final_config.contains("dst: \"/tmp/main.py\""));
        assert!(final_config.contains("dst: \"/tmp/wrapper.py\""));
        assert!(final_config.contains("dst: \"/dev/shm\""));
        assert!(final_config.contains("PYTHONPATH"));
    }

    #[test]
    fn test_config_pipeline_volumes_only_keeps_system_dirs() {
        let mut setup = SandboxSetupState::default();
        setup.volume_mounts.insert(
            "cache".to_string(),
            (PathBuf::from("/tmp/job456/volumes/cache"), "/tmp/pip-cache".to_string()),
        );
        let sandbox_mounts = build_sandbox_mounts(&setup);

        let raw_config = include_str!("../nsjail/run.python3.config.proto");
        let config = raw_config
            .replace("{JOB_DIR}", "/tmp/job456")
            .replace("{MAIN}", "main")
            .replace("{CLONE_NEWUSER}", "true")
            .replace("{SHARED_MOUNT}", &sandbox_mounts)
            .replace("{SHARED_DEPENDENCIES}", "")
            .replace("{PY_INSTALL_DIR}", "/usr/local")
            .replace("{GLOBAL_SITE_PACKAGES}", "/usr/lib/python3/dist-packages")
            .replace("{ADDITIONAL_PYTHON_PATHS}", "/tmp")
            .replace("{TRACING_PROXY_CA_CERT_PATH}", "/dev/null");

        let final_config = finalize_nsjail_config(&config);

        assert!(final_config.contains("dst: \"/bin\""));
        assert!(final_config.contains("dst: \"/lib\""));
        assert!(final_config.contains("dst: \"/usr\""));
        assert!(final_config.contains("dst: \"/etc\""));
        assert!(final_config.contains("dst: \"/tmp/pip-cache\""));
    }

    #[test]
    fn test_config_pipeline_no_annotations_passthrough() {
        let raw_config = include_str!("../nsjail/run.bash.config.proto");
        let config = raw_config
            .replace("{JOB_DIR}", "/tmp/job789")
            .replace("{CLONE_NEWUSER}", "true")
            .replace("{SHARED_MOUNT}", "")
            .replace("{TRACING_PROXY_CA_CERT_PATH}", "/dev/null");

        let final_config = finalize_nsjail_config(&config);

        assert!(final_config.contains("dst: \"/bin\""));
        assert!(final_config.contains("dst: \"/lib\""));
        assert!(final_config.contains("dst: \"/usr\""));
        assert!(final_config.contains("dst: \"/etc\""));
    }

    // =========================================================================
    // Integration tests: nsjail execution (requires nsjail binary)
    // =========================================================================

    mod nsjail_integration {
        use super::*;
        use std::process::Command;

        fn nsjail_available() -> bool {
            Command::new("nsjail")
                .arg("--help")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }

        fn run_nsjail_bash(
            job_dir: &std::path::Path,
            main_script: &str,
            extra_shared_mount: &str,
        ) -> (String, String, i32) {
            std::fs::write(job_dir.join("main.sh"), main_script).unwrap();
            std::fs::write(
                job_dir.join("wrapper.sh"),
                "#!/bin/bash\n/bin/bash /tmp/main.sh\n",
            )
            .unwrap();
            std::fs::write(job_dir.join("result.json"), "").unwrap();
            std::fs::write(job_dir.join("result.out"), "").unwrap();
            std::fs::write(job_dir.join("result2.out"), "").unwrap();

            let raw_config = include_str!("../nsjail/run.bash.config.proto");
            let config = raw_config
                .replace("{JOB_DIR}", &job_dir.to_string_lossy())
                .replace("{CLONE_NEWUSER}", "true")
                .replace("{SHARED_MOUNT}", extra_shared_mount)
                .replace("{TRACING_PROXY_CA_CERT_PATH}", "/dev/null")
                .replace("#{DEV}", "");

            let final_config = finalize_nsjail_config(&config);
            std::fs::write(job_dir.join("run.config.proto"), &final_config).unwrap();

            let output = Command::new("nsjail")
                .args(["--config", "run.config.proto", "--", "/bin/bash", "wrapper.sh"])
                .current_dir(job_dir)
                .output()
                .expect("Failed to execute nsjail");

            (
                String::from_utf8_lossy(&output.stdout).to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
                output.status.code().unwrap_or(-1),
            )
        }

        #[test]
        fn test_nsjail_basic_bash_execution() {
            if !nsjail_available() {
                eprintln!("Skipping: nsjail not available");
                return;
            }

            let job_dir = tempfile::tempdir().unwrap();
            let (stdout, stderr, exit_code) = run_nsjail_bash(
                job_dir.path(),
                "#!/bin/bash\necho 'hello from nsjail' > /tmp/result.out\n",
                "",
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0, "nsjail should exit successfully");

            let result = std::fs::read_to_string(job_dir.path().join("result.out")).unwrap();
            assert_eq!(result.trim(), "hello from nsjail");
        }

        #[test]
        fn test_nsjail_bash_with_volume_read() {
            if !nsjail_available() {
                eprintln!("Skipping: nsjail not available");
                return;
            }

            let job_dir = tempfile::tempdir().unwrap();
            let vol_dir = job_dir.path().join("volumes/data");
            std::fs::create_dir_all(&vol_dir).unwrap();
            std::fs::write(vol_dir.join("input.txt"), "volume data here").unwrap();

            let mut setup = SandboxSetupState::default();
            setup.volume_mounts.insert(
                "data".to_string(),
                (vol_dir, "/workspace/data".to_string()),
            );
            let sandbox_mounts = build_sandbox_mounts(&setup);

            let (stdout, stderr, exit_code) = run_nsjail_bash(
                job_dir.path(),
                "#!/bin/bash\ncat /workspace/data/input.txt > /tmp/result.out\n",
                &sandbox_mounts,
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0);

            let result = std::fs::read_to_string(job_dir.path().join("result.out")).unwrap();
            assert_eq!(result.trim(), "volume data here");
        }

        #[test]
        fn test_nsjail_bash_volume_write_back() {
            if !nsjail_available() {
                eprintln!("Skipping: nsjail not available");
                return;
            }

            let job_dir = tempfile::tempdir().unwrap();
            let vol_dir = job_dir.path().join("volumes/output");
            std::fs::create_dir_all(&vol_dir).unwrap();

            let mut setup = SandboxSetupState::default();
            setup.volume_mounts.insert(
                "output".to_string(),
                (vol_dir.clone(), "/workspace/output".to_string()),
            );
            let sandbox_mounts = build_sandbox_mounts(&setup);

            let (stdout, stderr, exit_code) = run_nsjail_bash(
                job_dir.path(),
                "#!/bin/bash\necho 'written by nsjail' > /workspace/output/result.txt\necho 'done' > /tmp/result.out\n",
                &sandbox_mounts,
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0);

            let written = std::fs::read_to_string(vol_dir.join("result.txt")).unwrap();
            assert_eq!(written.trim(), "written by nsjail");
        }

        #[test]
        fn test_nsjail_bash_with_overlay_root() {
            if !nsjail_available() {
                eprintln!("Skipping: nsjail not available");
                return;
            }

            let job_dir = tempfile::tempdir().unwrap();
            let setup = SandboxSetupState {
                overlay: Some(OverlayMount {
                    merged: PathBuf::from("/"),
                    upper: PathBuf::from("/unused"),
                    work: PathBuf::from("/unused"),
                    is_fuse: false,
                }),
                volume_mounts: HashMap::new(),
            };
            let sandbox_mounts = build_sandbox_mounts(&setup);

            let (stdout, stderr, exit_code) = run_nsjail_bash(
                job_dir.path(),
                "#!/bin/bash\nuname -s > /tmp/result.out\n",
                &sandbox_mounts,
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0);

            let result = std::fs::read_to_string(job_dir.path().join("result.out")).unwrap();
            assert_eq!(result.trim(), "Linux");
        }

        #[test]
        fn test_nsjail_bash_overlay_with_volume() {
            if !nsjail_available() {
                eprintln!("Skipping: nsjail not available");
                return;
            }

            let job_dir = tempfile::tempdir().unwrap();
            let vol_dir = job_dir.path().join("volumes/shared");
            std::fs::create_dir_all(&vol_dir).unwrap();
            std::fs::write(vol_dir.join("config.yaml"), "key: value").unwrap();

            let mut setup = SandboxSetupState {
                overlay: Some(OverlayMount {
                    merged: PathBuf::from("/"),
                    upper: PathBuf::from("/unused"),
                    work: PathBuf::from("/unused"),
                    is_fuse: false,
                }),
                volume_mounts: HashMap::new(),
            };
            setup.volume_mounts.insert(
                "shared".to_string(),
                (vol_dir.clone(), "/tmp/volumes/shared".to_string()),
            );
            let sandbox_mounts = build_sandbox_mounts(&setup);

            let (stdout, stderr, exit_code) = run_nsjail_bash(
                job_dir.path(),
                "#!/bin/bash\ncat /tmp/volumes/shared/config.yaml > /tmp/result.out\necho 'output' > /tmp/volumes/shared/output.txt\n",
                &sandbox_mounts,
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0);

            let result = std::fs::read_to_string(job_dir.path().join("result.out")).unwrap();
            assert_eq!(result.trim(), "key: value");

            let output = std::fs::read_to_string(vol_dir.join("output.txt")).unwrap();
            assert_eq!(output.trim(), "output");
        }

        #[test]
        fn test_nsjail_bash_result_json() {
            if !nsjail_available() {
                eprintln!("Skipping: nsjail not available");
                return;
            }

            let job_dir = tempfile::tempdir().unwrap();
            let (stdout, stderr, exit_code) = run_nsjail_bash(
                job_dir.path(),
                "#!/bin/bash\necho '{\"status\": \"ok\", \"count\": 42}' > /tmp/result.json\necho 'done' > /tmp/result.out\n",
                "",
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0);

            let result_json = std::fs::read_to_string(job_dir.path().join("result.json")).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(result_json.trim()).unwrap();
            assert_eq!(parsed["status"], "ok");
            assert_eq!(parsed["count"], 42);
        }
    }

    // =========================================================================
    // Integration tests: mount_overlay / unmount_overlay
    // =========================================================================

    mod overlay_integration {
        use super::*;

        #[tokio::test]
        async fn test_mount_overlay_read_write_semantics() {
            let snapshot_dir = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(snapshot_dir.path().join("usr/bin")).unwrap();
            std::fs::write(
                snapshot_dir.path().join("usr/bin/hello"),
                "#!/bin/sh\necho hi",
            )
            .unwrap();
            std::fs::write(snapshot_dir.path().join("base.txt"), "from snapshot").unwrap();

            let job_dir = tempfile::tempdir().unwrap();

            let overlay = match windmill_sandbox::mount_overlay(
                snapshot_dir.path(),
                &job_dir.path().to_string_lossy(),
            )
            .await
            {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("Skipping: mount_overlay failed (need root or fuse-overlayfs): {e}");
                    return;
                }
            };

            assert_eq!(
                std::fs::read_to_string(overlay.merged.join("base.txt")).unwrap(),
                "from snapshot"
            );
            assert!(overlay.merged.join("usr/bin/hello").exists());

            std::fs::write(overlay.merged.join("new_file.txt"), "written").unwrap();
            assert!(overlay.upper.join("new_file.txt").exists());
            assert!(!snapshot_dir.path().join("new_file.txt").exists());

            std::fs::write(overlay.merged.join("base.txt"), "modified").unwrap();
            assert_eq!(
                std::fs::read_to_string(overlay.merged.join("base.txt")).unwrap(),
                "modified"
            );
            assert_eq!(
                std::fs::read_to_string(snapshot_dir.path().join("base.txt")).unwrap(),
                "from snapshot"
            );

            windmill_sandbox::unmount_overlay(&overlay).await.expect("unmount should succeed");
            assert!(!overlay.merged.exists());
        }
    }

    // =========================================================================
    // Integration tests: crane export (snapshot build pipeline)
    // =========================================================================

    mod crane_integration {
        use super::*;
        use std::process::Command;

        fn crane_path() -> Option<String> {
            for path in &["crane", &format!("{}/go/bin/crane", std::env::var("HOME").unwrap_or_default())] {
                if Command::new(path)
                    .arg("version")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
                {
                    return Some(path.to_string());
                }
            }
            None
        }

        #[test]
        fn test_crane_export_and_tar_roundtrip() {
            let Some(crane) = crane_path() else {
                eprintln!("Skipping: crane not available");
                return;
            };

            let rootfs_dir = tempfile::tempdir().unwrap();

            let crane_output = Command::new(&crane)
                .args(["export", "alpine:latest", "-"])
                .output()
                .expect("Failed to run crane");

            if !crane_output.status.success() {
                eprintln!(
                    "crane export failed (network?): {}",
                    String::from_utf8_lossy(&crane_output.stderr)
                );
                return;
            }

            {
                use std::io::Cursor;
                use tar::Archive;
                let mut archive = Archive::new(Cursor::new(&crane_output.stdout));
                archive.unpack(rootfs_dir.path()).expect("Failed to unpack crane output");
            }

            assert!(
                rootfs_dir.path().join("bin/sh").exists()
                    || rootfs_dir.path().join("bin/busybox").exists()
            );

            let tar_bytes = windmill_sandbox::tar_gz(rootfs_dir.path()).unwrap();
            assert!(tar_bytes.len() > 1_000_000);

            let unpack_dir = tempfile::tempdir().unwrap();
            windmill_sandbox::untar_gz(&tar_bytes, unpack_dir.path()).unwrap();

            assert!(
                unpack_dir.path().join("bin/sh").exists()
                    || unpack_dir.path().join("bin/busybox").exists()
            );
        }

        #[test]
        fn test_crane_rootfs_in_nsjail() {
            let Some(crane) = crane_path() else {
                eprintln!("Skipping: crane not available");
                return;
            };
            if !Command::new("nsjail")
                .arg("--help")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                eprintln!("Skipping: nsjail not available");
                return;
            }

            let rootfs_dir = tempfile::tempdir().unwrap();

            let crane_output = Command::new(&crane)
                .args(["export", "alpine:latest", "-"])
                .output()
                .expect("Failed to run crane");

            if !crane_output.status.success() {
                eprintln!(
                    "crane export failed (network?): {}",
                    String::from_utf8_lossy(&crane_output.stderr)
                );
                return;
            }

            {
                use std::io::Cursor;
                use tar::Archive;
                let mut archive = Archive::new(Cursor::new(&crane_output.stdout));
                archive.unpack(rootfs_dir.path()).unwrap();
            }

            let job_dir = tempfile::tempdir().unwrap();
            std::fs::write(
                job_dir.path().join("main.sh"),
                "#!/bin/sh\n/bin/cat /etc/alpine-release > /tmp/result.out\n",
            )
            .unwrap();
            std::fs::write(
                job_dir.path().join("wrapper.sh"),
                "#!/bin/sh\n/bin/sh /tmp/main.sh\n",
            )
            .unwrap();
            std::fs::write(job_dir.path().join("result.json"), "").unwrap();
            std::fs::write(job_dir.path().join("result.out"), "").unwrap();
            std::fs::write(job_dir.path().join("result2.out"), "").unwrap();

            let setup = SandboxSetupState {
                overlay: Some(OverlayMount {
                    merged: rootfs_dir.path().to_path_buf(),
                    upper: PathBuf::from("/unused"),
                    work: PathBuf::from("/unused"),
                    is_fuse: false,
                }),
                volume_mounts: HashMap::new(),
            };
            let sandbox_mounts = build_sandbox_mounts(&setup);

            let raw_config = include_str!("../nsjail/run.bash.config.proto");
            let config = raw_config
                .replace("{JOB_DIR}", &job_dir.path().to_string_lossy())
                .replace("{CLONE_NEWUSER}", "true")
                .replace("{SHARED_MOUNT}", &sandbox_mounts)
                .replace("{TRACING_PROXY_CA_CERT_PATH}", "/dev/null")
                .replace("#{DEV}", "");
            let final_config = finalize_nsjail_config(&config);
            std::fs::write(job_dir.path().join("run.config.proto"), &final_config).unwrap();

            let output = Command::new("nsjail")
                .args(["--config", "run.config.proto", "--", "/bin/sh", "wrapper.sh"])
                .current_dir(job_dir.path())
                .output()
                .expect("Failed to run nsjail");

            if !output.status.success() {
                eprintln!(
                    "nsjail stderr: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            assert_eq!(output.status.code().unwrap_or(-1), 0);

            let result = std::fs::read_to_string(job_dir.path().join("result.out")).unwrap();
            assert!(!result.trim().is_empty());
            assert!(result.trim().starts_with("3."));
        }
    }
}
