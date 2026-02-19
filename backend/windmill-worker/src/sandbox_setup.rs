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
                    (
                        PathBuf::from("/tmp/job123/volumes/data"),
                        "/workspace/data".to_string(),
                    ),
                ),
                (
                    "models".to_string(),
                    (
                        PathBuf::from("/tmp/job123/volumes/models"),
                        "/workspace/models".to_string(),
                    ),
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

        let final_config = finalize_nsjail_config(&config, &[]);

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

        let final_config = finalize_nsjail_config(&config, &[]);

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
            (
                PathBuf::from("/tmp/job456/volumes/cache"),
                "/tmp/pip-cache".to_string(),
            ),
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

        let final_config = finalize_nsjail_config(&config, &[]);

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

        let final_config = finalize_nsjail_config(&config, &[]);

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

            let final_config = finalize_nsjail_config(&config, &[]);
            std::fs::write(job_dir.join("run.config.proto"), &final_config).unwrap();

            let output = Command::new("nsjail")
                .args([
                    "--config",
                    "run.config.proto",
                    "--",
                    "/bin/bash",
                    "wrapper.sh",
                ])
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
            setup
                .volume_mounts
                .insert("data".to_string(), (vol_dir, "/workspace/data".to_string()));
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

        // =====================================================================
        // Python nsjail integration tests
        // =====================================================================

        fn python3_path() -> Option<String> {
            Command::new("which")
                .arg("python3")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        }

        fn run_nsjail_python(
            job_dir: &std::path::Path,
            wrapper_script: &str,
            extra_shared_mount: &str,
            runtime_bins: &[&str],
        ) -> (String, String, i32) {
            let python3 = python3_path().expect("python3 must be available");
            let py_prefix = String::from_utf8(
                Command::new(&python3)
                    .args(["-c", "import sys; print(sys.prefix)"])
                    .output()
                    .unwrap()
                    .stdout,
            )
            .unwrap()
            .trim()
            .to_string();

            std::fs::write(job_dir.join("wrapper.py"), wrapper_script).unwrap();
            std::fs::write(job_dir.join("main.py"), "# placeholder").unwrap();
            std::fs::write(job_dir.join("args.json"), "{}").unwrap();
            std::fs::write(job_dir.join("result.json"), "").unwrap();

            let raw_config = include_str!("../nsjail/run.python3.config.proto");
            let config = raw_config
                .replace("{JOB_DIR}", &job_dir.to_string_lossy())
                .replace("{MAIN}", "main")
                .replace("{CLONE_NEWUSER}", "true")
                .replace("{SHARED_MOUNT}", extra_shared_mount)
                .replace("{SHARED_DEPENDENCIES}", "")
                .replace("{PY_INSTALL_DIR}", &py_prefix)
                .replace("{GLOBAL_SITE_PACKAGES}", "/nonexistent")
                .replace("{ADDITIONAL_PYTHON_PATHS}", "/tmp")
                .replace("{TRACING_PROXY_CA_CERT_PATH}", "/dev/null")
                .replace("#{DEV}", "");

            let final_config = finalize_nsjail_config(&config, runtime_bins);
            std::fs::write(job_dir.join("run.config.proto"), &final_config).unwrap();

            let output = Command::new("nsjail")
                .args([
                    "--config",
                    "run.config.proto",
                    "--",
                    &python3,
                    "-u",
                    "/tmp/wrapper.py",
                ])
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
        fn test_nsjail_basic_python_execution() {
            if !nsjail_available() || python3_path().is_none() {
                eprintln!("Skipping: nsjail or python3 not available");
                return;
            }

            let job_dir = tempfile::tempdir().unwrap();
            let (stdout, stderr, exit_code) = run_nsjail_python(
                job_dir.path(),
                "import json, sys\njson.dump({'lang': 'python', 'v': sys.version_info[0]}, open('result.json', 'w'))\n",
                "",
                &[],
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0, "python nsjail should exit successfully");

            let result_json = std::fs::read_to_string(job_dir.path().join("result.json")).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(result_json.trim()).unwrap();
            assert_eq!(parsed["lang"], "python");
            assert_eq!(parsed["v"], 3);
        }

        #[test]
        fn test_nsjail_python_with_overlay_root() {
            if !nsjail_available() || python3_path().is_none() {
                eprintln!("Skipping: nsjail or python3 not available");
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

            let (stdout, stderr, exit_code) = run_nsjail_python(
                job_dir.path(),
                "import json, platform\njson.dump({'os': platform.system()}, open('result.json', 'w'))\n",
                &sandbox_mounts,
                &[],
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0);

            let result_json = std::fs::read_to_string(job_dir.path().join("result.json")).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(result_json.trim()).unwrap();
            assert_eq!(parsed["os"], "Linux");
        }

        #[test]
        fn test_nsjail_python_volume_readwrite() {
            if !nsjail_available() || python3_path().is_none() {
                eprintln!("Skipping: nsjail or python3 not available");
                return;
            }

            let job_dir = tempfile::tempdir().unwrap();
            let vol_dir = job_dir.path().join("volumes/data");
            std::fs::create_dir_all(&vol_dir).unwrap();
            std::fs::write(vol_dir.join("input.txt"), "python volume data").unwrap();

            let mut setup = SandboxSetupState::default();
            setup.volume_mounts.insert(
                "data".to_string(),
                (vol_dir.clone(), "/workspace/data".to_string()),
            );
            let sandbox_mounts = build_sandbox_mounts(&setup);

            let (stdout, stderr, exit_code) = run_nsjail_python(
                job_dir.path(),
                "import json\n\
                 data = open('/workspace/data/input.txt').read().strip()\n\
                 open('/workspace/data/output.txt', 'w').write('written by python')\n\
                 json.dump({'read': data}, open('result.json', 'w'))\n",
                &sandbox_mounts,
                &[],
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0);

            let result_json = std::fs::read_to_string(job_dir.path().join("result.json")).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(result_json.trim()).unwrap();
            assert_eq!(parsed["read"], "python volume data");

            let written = std::fs::read_to_string(vol_dir.join("output.txt")).unwrap();
            assert_eq!(written, "written by python");
        }

        #[test]
        fn test_nsjail_python_overlay_with_volume() {
            if !nsjail_available() || python3_path().is_none() {
                eprintln!("Skipping: nsjail or python3 not available");
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

            let (stdout, stderr, exit_code) = run_nsjail_python(
                job_dir.path(),
                "import json\n\
                 data = open('/tmp/volumes/shared/config.yaml').read().strip()\n\
                 open('/tmp/volumes/shared/output.txt', 'w').write('py output')\n\
                 json.dump({'read': data}, open('result.json', 'w'))\n",
                &sandbox_mounts,
                &[],
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0);

            let result_json = std::fs::read_to_string(job_dir.path().join("result.json")).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(result_json.trim()).unwrap();
            assert_eq!(parsed["read"], "key: value");

            let output = std::fs::read_to_string(vol_dir.join("output.txt")).unwrap();
            assert_eq!(output, "py output");
        }

        // =====================================================================
        // Bun nsjail integration tests
        // =====================================================================

        fn bun_available() -> bool {
            Command::new("bun")
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }

        fn bun_binary_path() -> String {
            String::from_utf8(Command::new("which").arg("bun").output().unwrap().stdout)
                .unwrap()
                .trim()
                .to_string()
        }

        fn run_nsjail_bun(
            job_dir: &std::path::Path,
            wrapper_script: &str,
            extra_shared_mount: &str,
            runtime_bins: &[&str],
        ) -> (String, String, i32) {
            let bun = bun_binary_path();

            std::fs::write(job_dir.join("wrapper.mjs"), wrapper_script).unwrap();
            std::fs::write(job_dir.join("args.json"), "{}").unwrap();
            std::fs::write(job_dir.join("result.json"), "").unwrap();

            let raw_config = include_str!("../nsjail/run.bun.config.proto");
            let config = raw_config
                .replace("{JOB_DIR}", &job_dir.to_string_lossy())
                .replace("{LANG}", "bun")
                .replace("{CLONE_NEWUSER}", "true")
                .replace("{SHARED_MOUNT}", extra_shared_mount)
                .replace("{TRACING_PROXY_CA_CERT_PATH}", "/dev/null")
                .replace("#{DEV}", "");

            let final_config = finalize_nsjail_config(&config, runtime_bins);
            std::fs::write(job_dir.join("run.config.proto"), &final_config).unwrap();

            let output = Command::new("nsjail")
                .args([
                    "--config",
                    "run.config.proto",
                    "--",
                    &bun,
                    "run",
                    "wrapper.mjs",
                ])
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
        fn test_nsjail_basic_bun_execution() {
            if !nsjail_available() || !bun_available() {
                eprintln!("Skipping: nsjail or bun not available");
                return;
            }

            let job_dir = tempfile::tempdir().unwrap();
            let (stdout, stderr, exit_code) = run_nsjail_bun(
                job_dir.path(),
                "import { writeFileSync } from 'fs';\n\
                 writeFileSync('result.json', JSON.stringify({ lang: 'bun' }));\n",
                "",
                &[],
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0, "bun nsjail should exit successfully");

            let result_json = std::fs::read_to_string(job_dir.path().join("result.json")).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(result_json.trim()).unwrap();
            assert_eq!(parsed["lang"], "bun");
        }

        #[test]
        fn test_nsjail_bun_with_overlay_root() {
            if !nsjail_available() || !bun_available() {
                eprintln!("Skipping: nsjail or bun not available");
                return;
            }

            let bun = bun_binary_path();
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

            // With overlay, /usr is stripped so bun binary needs runtime_bins
            let (stdout, stderr, exit_code) = run_nsjail_bun(
                job_dir.path(),
                "import { writeFileSync } from 'fs';\n\
                 writeFileSync('result.json', JSON.stringify({ lang: 'bun', overlay: true }));\n",
                &sandbox_mounts,
                &[&bun],
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0);

            let result_json = std::fs::read_to_string(job_dir.path().join("result.json")).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(result_json.trim()).unwrap();
            assert_eq!(parsed["lang"], "bun");
            assert_eq!(parsed["overlay"], true);
        }

        #[test]
        fn test_nsjail_bun_volume_readwrite() {
            if !nsjail_available() || !bun_available() {
                eprintln!("Skipping: nsjail or bun not available");
                return;
            }

            let job_dir = tempfile::tempdir().unwrap();
            let vol_dir = job_dir.path().join("volumes/data");
            std::fs::create_dir_all(&vol_dir).unwrap();
            std::fs::write(vol_dir.join("input.txt"), "bun volume data").unwrap();

            let mut setup = SandboxSetupState::default();
            setup.volume_mounts.insert(
                "data".to_string(),
                (vol_dir.clone(), "/workspace/data".to_string()),
            );
            let sandbox_mounts = build_sandbox_mounts(&setup);

            let (stdout, stderr, exit_code) = run_nsjail_bun(
                job_dir.path(),
                "import { readFileSync, writeFileSync } from 'fs';\n\
                 const data = readFileSync('/workspace/data/input.txt', 'utf8').trim();\n\
                 writeFileSync('/workspace/data/output.txt', 'written by bun');\n\
                 writeFileSync('result.json', JSON.stringify({ read: data }));\n",
                &sandbox_mounts,
                &[],
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0);

            let result_json = std::fs::read_to_string(job_dir.path().join("result.json")).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(result_json.trim()).unwrap();
            assert_eq!(parsed["read"], "bun volume data");

            let written = std::fs::read_to_string(vol_dir.join("output.txt")).unwrap();
            assert_eq!(written, "written by bun");
        }

        #[test]
        fn test_nsjail_bun_overlay_with_volume() {
            if !nsjail_available() || !bun_available() {
                eprintln!("Skipping: nsjail or bun not available");
                return;
            }

            let bun = bun_binary_path();
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

            let (stdout, stderr, exit_code) = run_nsjail_bun(
                job_dir.path(),
                "import { readFileSync, writeFileSync } from 'fs';\n\
                 const data = readFileSync('/tmp/volumes/shared/config.yaml', 'utf8').trim();\n\
                 writeFileSync('/tmp/volumes/shared/output.txt', 'bun output');\n\
                 writeFileSync('result.json', JSON.stringify({ read: data }));\n",
                &sandbox_mounts,
                &[&bun],
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0);

            let result_json = std::fs::read_to_string(job_dir.path().join("result.json")).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(result_json.trim()).unwrap();
            assert_eq!(parsed["read"], "key: value");

            let output = std::fs::read_to_string(vol_dir.join("output.txt")).unwrap();
            assert_eq!(output, "bun output");
        }

        // =====================================================================
        // Go nsjail integration tests
        // =====================================================================

        fn go_available() -> bool {
            Command::new("go")
                .arg("version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }

        fn compile_go_binary(job_dir: &std::path::Path, go_source: &str) -> bool {
            let src_dir = tempfile::tempdir().unwrap();
            std::fs::write(src_dir.path().join("main.go"), go_source).unwrap();
            Command::new("go")
                .args([
                    "build",
                    "-o",
                    &job_dir.join("main").to_string_lossy(),
                    "main.go",
                ])
                .current_dir(src_dir.path())
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }

        fn run_nsjail_go(
            job_dir: &std::path::Path,
            go_source: &str,
            extra_shared_mount: &str,
        ) -> (String, String, i32) {
            if !compile_go_binary(job_dir, go_source) {
                return (String::new(), "Go compilation failed".to_string(), -1);
            }

            std::fs::write(job_dir.join("args.json"), "{}").unwrap();
            std::fs::write(job_dir.join("result.json"), "").unwrap();

            let raw_config = include_str!("../nsjail/run.go.config.proto");
            let config = raw_config
                .replace("{JOB_DIR}", &job_dir.to_string_lossy())
                .replace("{CLONE_NEWUSER}", "true")
                .replace("{SHARED_MOUNT}", extra_shared_mount)
                .replace("{TRACING_PROXY_CA_CERT_PATH}", "/dev/null")
                .replace("#{DEV}", "");

            let final_config = finalize_nsjail_config(&config, &[]);
            std::fs::write(job_dir.join("run.config.proto"), &final_config).unwrap();

            let output = Command::new("nsjail")
                .args(["--config", "run.config.proto", "--", "/tmp/go/main"])
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
        fn test_nsjail_basic_go_execution() {
            if !nsjail_available() || !go_available() {
                eprintln!("Skipping: nsjail or go not available");
                return;
            }

            let job_dir = tempfile::tempdir().unwrap();
            let (stdout, stderr, exit_code) = run_nsjail_go(
                job_dir.path(),
                r#"package main
import "os"
func main() {
    os.WriteFile("result.json", []byte(`{"lang":"go"}`), 0644)
}"#,
                "",
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0, "go nsjail should exit successfully");

            let result_json = std::fs::read_to_string(job_dir.path().join("result.json")).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(result_json.trim()).unwrap();
            assert_eq!(parsed["lang"], "go");
        }

        #[test]
        fn test_nsjail_go_with_overlay_root() {
            if !nsjail_available() || !go_available() {
                eprintln!("Skipping: nsjail or go not available");
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

            let (stdout, stderr, exit_code) = run_nsjail_go(
                job_dir.path(),
                r#"package main
import (
    "os"
    "runtime"
)
func main() {
    os.WriteFile("result.json", []byte(`{"os":"`+runtime.GOOS+`"}`), 0644)
}"#,
                &sandbox_mounts,
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0);

            let result_json = std::fs::read_to_string(job_dir.path().join("result.json")).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(result_json.trim()).unwrap();
            assert_eq!(parsed["os"], "linux");
        }

        #[test]
        fn test_nsjail_go_volume_readwrite() {
            if !nsjail_available() || !go_available() {
                eprintln!("Skipping: nsjail or go not available");
                return;
            }

            let job_dir = tempfile::tempdir().unwrap();
            let vol_dir = job_dir.path().join("volumes/data");
            std::fs::create_dir_all(&vol_dir).unwrap();
            std::fs::write(vol_dir.join("input.txt"), "go volume data").unwrap();

            let mut setup = SandboxSetupState::default();
            setup.volume_mounts.insert(
                "data".to_string(),
                (vol_dir.clone(), "/workspace/data".to_string()),
            );
            let sandbox_mounts = build_sandbox_mounts(&setup);

            let (stdout, stderr, exit_code) = run_nsjail_go(
                job_dir.path(),
                r#"package main
import (
    "encoding/json"
    "os"
    "strings"
)
func main() {
    data, _ := os.ReadFile("/workspace/data/input.txt")
    os.WriteFile("/workspace/data/output.txt", []byte("written by go"), 0644)
    result, _ := json.Marshal(map[string]string{"read": strings.TrimSpace(string(data))})
    os.WriteFile("result.json", result, 0644)
}"#,
                &sandbox_mounts,
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0);

            let result_json = std::fs::read_to_string(job_dir.path().join("result.json")).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(result_json.trim()).unwrap();
            assert_eq!(parsed["read"], "go volume data");

            let written = std::fs::read_to_string(vol_dir.join("output.txt")).unwrap();
            assert_eq!(written, "written by go");
        }

        #[test]
        fn test_nsjail_go_overlay_with_volume() {
            if !nsjail_available() || !go_available() {
                eprintln!("Skipping: nsjail or go not available");
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

            let (stdout, stderr, exit_code) = run_nsjail_go(
                job_dir.path(),
                r#"package main
import (
    "encoding/json"
    "os"
    "strings"
)
func main() {
    data, _ := os.ReadFile("/tmp/volumes/shared/config.yaml")
    os.WriteFile("/tmp/volumes/shared/output.txt", []byte("go output"), 0644)
    result, _ := json.Marshal(map[string]string{"read": strings.TrimSpace(string(data))})
    os.WriteFile("result.json", result, 0644)
}"#,
                &sandbox_mounts,
            );

            if exit_code != 0 {
                eprintln!("nsjail stdout: {stdout}");
                eprintln!("nsjail stderr: {stderr}");
            }
            assert_eq!(exit_code, 0);

            let result_json = std::fs::read_to_string(job_dir.path().join("result.json")).unwrap();
            let parsed: serde_json::Value = serde_json::from_str(result_json.trim()).unwrap();
            assert_eq!(parsed["read"], "key: value");

            let output = std::fs::read_to_string(vol_dir.join("output.txt")).unwrap();
            assert_eq!(output, "go output");
        }
    }

    // =========================================================================
    // Integration tests: mount_overlay / unmount_overlay
    // =========================================================================

    mod overlay_integration {
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

            windmill_sandbox::unmount_overlay(&overlay)
                .await
                .expect("unmount should succeed");
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
            for path in &[
                "crane",
                &format!("{}/go/bin/crane", std::env::var("HOME").unwrap_or_default()),
            ] {
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
                archive
                    .unpack(rootfs_dir.path())
                    .expect("Failed to unpack crane output");
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
            let final_config = finalize_nsjail_config(&config, &[]);
            std::fs::write(job_dir.path().join("run.config.proto"), &final_config).unwrap();

            let output = Command::new("nsjail")
                .args([
                    "--config",
                    "run.config.proto",
                    "--",
                    "/bin/sh",
                    "wrapper.sh",
                ])
                .current_dir(job_dir.path())
                .output()
                .expect("Failed to run nsjail");

            if !output.status.success() {
                eprintln!("nsjail stderr: {}", String::from_utf8_lossy(&output.stderr));
            }
            assert_eq!(output.status.code().unwrap_or(-1), 0);

            let result = std::fs::read_to_string(job_dir.path().join("result.out")).unwrap();
            assert!(!result.trim().is_empty());
            assert!(result.trim().starts_with("3."));
        }
    }

    // =========================================================================
    // Integration tests: volume round-trip via filesystem object store
    // =========================================================================

    #[cfg(feature = "parquet")]
    mod volume_store_integration {
        use super::*;
        use std::process::Command;

        fn nsjail_available() -> bool {
            Command::new("nsjail")
                .arg("--help")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }

        fn run_nsjail_bash_simple(
            job_dir: &std::path::Path,
            main_script: &str,
            extra_shared_mount: &str,
        ) -> i32 {
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

            let final_config = finalize_nsjail_config(&config, &[]);
            std::fs::write(job_dir.join("run.config.proto"), &final_config).unwrap();

            let output = Command::new("nsjail")
                .args([
                    "--config",
                    "run.config.proto",
                    "--",
                    "/bin/bash",
                    "wrapper.sh",
                ])
                .current_dir(job_dir)
                .output()
                .expect("Failed to execute nsjail");

            if !output.status.success() {
                eprintln!("nsjail stderr: {}", String::from_utf8_lossy(&output.stderr));
            }
            output.status.code().unwrap_or(-1)
        }

        /// Test: store volume data in filesystem object store → retrieve → mount in nsjail → read
        #[tokio::test]
        async fn test_volume_read_via_filesystem_store() {
            if !nsjail_available() {
                eprintln!("Skipping: nsjail not available");
                return;
            }

            // 1. Create filesystem-backed object store
            let store_root = tempfile::tempdir().unwrap();
            let store =
                windmill_object_store::build_filesystem_client(store_root.path().to_str().unwrap())
                    .unwrap();

            // 2. Create volume content and upload to store
            let source_dir = tempfile::tempdir().unwrap();
            std::fs::write(source_dir.path().join("data.txt"), "hello from store\n").unwrap();
            std::fs::create_dir(source_dir.path().join("subdir")).unwrap();
            std::fs::write(
                source_dir.path().join("subdir/nested.txt"),
                "nested content\n",
            )
            .unwrap();

            let tar_bytes = windmill_sandbox::tar_gz(source_dir.path()).unwrap();
            windmill_object_store::put_bytes_to_store(
                store.clone(),
                "sandbox/volumes/test-ws/mydata.tar.gz",
                tar_bytes.into(),
            )
            .await
            .unwrap();

            // 3. Download from store and unpack to volume dir
            let job_dir = tempfile::tempdir().unwrap();
            let vol_dir = job_dir.path().join("volumes/mydata");
            std::fs::create_dir_all(&vol_dir).unwrap();

            let downloaded = windmill_object_store::fetch_bytes_from_store(
                store.clone(),
                "sandbox/volumes/test-ws/mydata.tar.gz",
            )
            .await
            .unwrap()
            .expect("should find stored volume data");
            windmill_sandbox::untar_gz(&downloaded, &vol_dir).unwrap();

            // 4. Mount in nsjail and verify from inside sandbox
            let mut setup = SandboxSetupState::default();
            setup.volume_mounts.insert(
                "mydata".to_string(),
                (vol_dir, "/workspace/data".to_string()),
            );
            let sandbox_mounts = build_sandbox_mounts(&setup);

            let exit_code = run_nsjail_bash_simple(
                job_dir.path(),
                "#!/bin/bash\n\
                 cat /workspace/data/data.txt > /tmp/result.out\n\
                 cat /workspace/data/subdir/nested.txt >> /tmp/result.out\n",
                &sandbox_mounts,
            );
            assert_eq!(exit_code, 0);

            let result = std::fs::read_to_string(job_dir.path().join("result.out")).unwrap();
            let lines: Vec<&str> = result.trim().lines().collect();
            assert_eq!(lines[0], "hello from store");
            assert_eq!(lines[1], "nested content");
        }

        /// Test: nsjail writes to volume → tar → store → retrieve → verify round-trip
        #[tokio::test]
        async fn test_volume_write_roundtrip_via_filesystem_store() {
            if !nsjail_available() {
                eprintln!("Skipping: nsjail not available");
                return;
            }

            let store_root = tempfile::tempdir().unwrap();
            let store =
                windmill_object_store::build_filesystem_client(store_root.path().to_str().unwrap())
                    .unwrap();

            // 1. Run nsjail with an empty volume mount, write files inside
            let job_dir = tempfile::tempdir().unwrap();
            let vol_dir = job_dir.path().join("volumes/output");
            std::fs::create_dir_all(&vol_dir).unwrap();

            let mut setup = SandboxSetupState::default();
            setup.volume_mounts.insert(
                "output".to_string(),
                (vol_dir.clone(), "/workspace/output".to_string()),
            );
            let sandbox_mounts = build_sandbox_mounts(&setup);

            let exit_code = run_nsjail_bash_simple(
                job_dir.path(),
                "#!/bin/bash\n\
                 echo 'written in sandbox' > /workspace/output/result.txt\n\
                 mkdir -p /workspace/output/subdir\n\
                 echo 'nested write' > /workspace/output/subdir/nested.txt\n\
                 echo 'done' > /tmp/result.out\n",
                &sandbox_mounts,
            );
            assert_eq!(exit_code, 0);

            // 2. Upload the written volume to the filesystem store
            let tar_bytes = windmill_sandbox::tar_gz(&vol_dir).unwrap();
            windmill_object_store::put_bytes_to_store(
                store.clone(),
                "sandbox/volumes/test-ws/output.tar.gz",
                tar_bytes.into(),
            )
            .await
            .unwrap();

            // 3. Download to a fresh directory and verify content survived the round-trip
            let verify_dir = tempfile::tempdir().unwrap();
            let downloaded = windmill_object_store::fetch_bytes_from_store(
                store.clone(),
                "sandbox/volumes/test-ws/output.tar.gz",
            )
            .await
            .unwrap()
            .expect("should find stored volume data");
            windmill_sandbox::untar_gz(&downloaded, verify_dir.path()).unwrap();

            let content = std::fs::read_to_string(verify_dir.path().join("result.txt")).unwrap();
            assert_eq!(content.trim(), "written in sandbox");

            let nested =
                std::fs::read_to_string(verify_dir.path().join("subdir/nested.txt")).unwrap();
            assert_eq!(nested.trim(), "nested write");
        }

        /// Test: simulate two successive job runs sharing a volume via the object store.
        /// Job 1 writes to the volume, volume gets persisted to the store.
        /// Job 2 reads the persisted volume and verifies the data.
        #[tokio::test]
        async fn test_volume_persistence_across_jobs_via_filesystem_store() {
            if !nsjail_available() {
                eprintln!("Skipping: nsjail not available");
                return;
            }

            let store_root = tempfile::tempdir().unwrap();
            let store =
                windmill_object_store::build_filesystem_client(store_root.path().to_str().unwrap())
                    .unwrap();
            let s3_key = "sandbox/volumes/test-ws/shared.tar.gz";

            // --- Job 1: write to volume ---
            let job1_dir = tempfile::tempdir().unwrap();
            let vol1_dir = job1_dir.path().join("volumes/shared");
            std::fs::create_dir_all(&vol1_dir).unwrap();

            let mut setup1 = SandboxSetupState::default();
            setup1.volume_mounts.insert(
                "shared".to_string(),
                (vol1_dir.clone(), "/workspace/shared".to_string()),
            );
            let mounts1 = build_sandbox_mounts(&setup1);

            let exit1 = run_nsjail_bash_simple(
                job1_dir.path(),
                "#!/bin/bash\n\
                 echo 'counter=1' > /workspace/shared/state.txt\n\
                 echo 'done' > /tmp/result.out\n",
                &mounts1,
            );
            assert_eq!(exit1, 0);

            // Persist volume to store
            let tar_bytes = windmill_sandbox::tar_gz(&vol1_dir).unwrap();
            windmill_object_store::put_bytes_to_store(store.clone(), s3_key, tar_bytes.into())
                .await
                .unwrap();

            // --- Job 2: read volume from store ---
            let job2_dir = tempfile::tempdir().unwrap();
            let vol2_dir = job2_dir.path().join("volumes/shared");
            std::fs::create_dir_all(&vol2_dir).unwrap();

            let downloaded = windmill_object_store::fetch_bytes_from_store(store.clone(), s3_key)
                .await
                .unwrap()
                .expect("volume should exist in store");
            windmill_sandbox::untar_gz(&downloaded, &vol2_dir).unwrap();

            let mut setup2 = SandboxSetupState::default();
            setup2.volume_mounts.insert(
                "shared".to_string(),
                (vol2_dir.clone(), "/workspace/shared".to_string()),
            );
            let mounts2 = build_sandbox_mounts(&setup2);

            let exit2 = run_nsjail_bash_simple(
                job2_dir.path(),
                "#!/bin/bash\n\
                 cat /workspace/shared/state.txt > /tmp/result.out\n",
                &mounts2,
            );
            assert_eq!(exit2, 0);

            let result = std::fs::read_to_string(job2_dir.path().join("result.out")).unwrap();
            assert_eq!(result.trim(), "counter=1");
        }
    }
}
