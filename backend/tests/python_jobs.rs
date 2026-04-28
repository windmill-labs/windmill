use serde_json::json;
#[cfg(feature = "python")]
use sqlx::postgres::Postgres;
#[cfg(feature = "python")]
use sqlx::Pool;
#[cfg(feature = "python")]
use windmill_common::scripts::ScriptLang;
use windmill_test_utils::*;

// ============================================================================
// Dedicated Worker Protocol Tests (Python)
// ============================================================================

#[cfg(feature = "python")]
mod dedicated_worker_protocol_python {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};
    use windmill_test_utils::{parse_dedicated_worker_line, DedicatedWorkerResult};
    use windmill_worker::{compute_py_codegen, generate_py_multi_script_wrapper, PyScriptEntry};

    struct MultiScriptJob {
        script_path: String,
        args: serde_json::Value,
    }

    /// Creates a multi-script Python wrapper, writes scripts to proper module paths
    fn create_py_worker_files(
        dir: &std::path::Path,
        scripts: &[(&str, &str)], // (original_path, content)
    ) -> std::path::PathBuf {
        let mut codegens = Vec::new();
        for (path, content) in scripts {
            let cg = compute_py_codegen(content, path);
            let module_dir = dir.join(&cg.dirs);
            std::fs::create_dir_all(&module_dir).unwrap();
            std::fs::write(module_dir.join(format!("{}.py", cg.module_name)), content).unwrap();
            codegens.push((path.to_string(), cg));
        }

        let entries: Vec<PyScriptEntry<'_>> = codegens
            .iter()
            .map(|(path, cg)| PyScriptEntry { original_path: path.as_str(), codegen: cg })
            .collect();

        let wrapper = generate_py_multi_script_wrapper(&entries, false, false);
        let wrapper_path = dir.join("wrapper.py");
        std::fs::write(&wrapper_path, &wrapper).unwrap();
        wrapper_path
    }

    fn run_py_multi_script_test(
        scripts: &[(&str, &str)],
        jobs: Vec<MultiScriptJob>,
    ) -> Vec<Result<serde_json::Value, String>> {
        let temp_dir = tempfile::tempdir().unwrap();
        create_py_worker_files(temp_dir.path(), scripts);

        let mut child = Command::new("python3")
            .args(["-u", "-m", "wrapper"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(temp_dir.path())
            .spawn()
            .expect("Failed to spawn python3 process");

        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);

        let mut start_line = String::new();
        reader.read_line(&mut start_line).unwrap();
        assert_eq!(
            parse_dedicated_worker_line(start_line.trim()),
            DedicatedWorkerResult::Start,
            "Expected 'start', got: {}",
            start_line.trim()
        );

        let mut results = Vec::new();
        for job in &jobs {
            writeln!(stdin, "exec:{}:{}", job.script_path, job.args.to_string()).unwrap();
            stdin.flush().unwrap();

            let mut response = String::new();
            reader.read_line(&mut response).unwrap();

            match parse_dedicated_worker_line(response.trim()) {
                DedicatedWorkerResult::Success(value) => results.push(Ok(value)),
                DedicatedWorkerResult::Error(err) => {
                    let msg = err["message"]
                        .as_str()
                        .unwrap_or("Unknown error")
                        .to_string();
                    results.push(Err(msg));
                }
                other => panic!("Unexpected response: {:?}", other),
            }
        }

        writeln!(stdin, "end").unwrap();
        stdin.flush().unwrap();
        let _ = child.wait().expect("Worker process failed to exit");
        results
    }

    fn run_py_single_script_test(
        script_path: &str,
        content: &str,
        jobs: Vec<serde_json::Value>,
    ) -> Vec<Result<serde_json::Value, String>> {
        run_py_multi_script_test(
            &[(script_path, content)],
            jobs.into_iter()
                .map(|args| MultiScriptJob { script_path: script_path.to_string(), args })
                .collect(),
        )
    }

    #[test]
    fn test_python_dedicated_worker_simple() {
        let results = run_py_single_script_test(
            "f/test/add",
            "def main(a: int, b: int):\n    return a + b\n",
            vec![serde_json::json!({"a": 3, "b": 4})],
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], Ok(serde_json::json!(7)));
    }

    #[test]
    fn test_python_dedicated_worker_multiple_jobs() {
        let results = run_py_single_script_test(
            "f/test/double",
            "def main(n: int):\n    return n * 2\n",
            (1..=5).map(|i| serde_json::json!({"n": i})).collect(),
        );
        assert_eq!(results.len(), 5);
        for (i, result) in results.iter().enumerate() {
            assert_eq!(*result, Ok(serde_json::json!(((i + 1) * 2) as i64)));
        }
    }

    #[test]
    fn test_python_multi_script_routing() {
        let results = run_py_multi_script_test(
            &[
                (
                    "f/math/add",
                    "def main(a: int, b: int):\n    return a + b\n",
                ),
                (
                    "f/math/mul",
                    "def main(x: int, y: int):\n    return x * y\n",
                ),
            ],
            vec![
                MultiScriptJob {
                    script_path: "f/math/add".to_string(),
                    args: serde_json::json!({"a": 3, "b": 4}),
                },
                MultiScriptJob {
                    script_path: "f/math/mul".to_string(),
                    args: serde_json::json!({"x": 5, "y": 6}),
                },
                MultiScriptJob {
                    script_path: "f/math/add".to_string(),
                    args: serde_json::json!({"a": 10, "b": 20}),
                },
            ],
        );
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], Ok(serde_json::json!(7)));
        assert_eq!(results[1], Ok(serde_json::json!(30)));
        assert_eq!(results[2], Ok(serde_json::json!(30)));
    }

    #[test]
    fn test_python_multi_script_error_isolation() {
        let results = run_py_multi_script_test(
            &[
                ("f/ok", "def main(x: int):\n    return x * 2\n"),
                ("f/err", "def main(msg: str):\n    raise Exception(msg)\n"),
            ],
            vec![
                MultiScriptJob {
                    script_path: "f/ok".to_string(),
                    args: serde_json::json!({"x": 5}),
                },
                MultiScriptJob {
                    script_path: "f/err".to_string(),
                    args: serde_json::json!({"msg": "boom"}),
                },
                MultiScriptJob {
                    script_path: "f/ok".to_string(),
                    args: serde_json::json!({"x": 10}),
                },
            ],
        );
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], Ok(serde_json::json!(10)));
        assert!(results[1].is_err());
        assert_eq!(results[1], Err("boom".to_string()));
        assert_eq!(results[2], Ok(serde_json::json!(20)));
    }

    #[test]
    fn test_python_multi_script_unknown_path() {
        let results = run_py_multi_script_test(
            &[("f/known", "def main(x: int):\n    return x\n")],
            vec![MultiScriptJob {
                script_path: "f/unknown".to_string(),
                args: serde_json::json!({"x": 1}),
            }],
        );
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
        assert!(results[0]
            .as_ref()
            .unwrap_err()
            .contains("Script not found"));
    }

    // ==================== exec_preprocess Tests ====================

    /// Raw protocol command for Python
    enum ProtocolCmd {
        Exec { path: String, args: serde_json::Value },
        ExecPreprocess { path: String, args: serde_json::Value },
        Execd { args: serde_json::Value },
        ExecdPreprocess { args: serde_json::Value },
    }

    /// Run a Python worker test with raw protocol commands
    fn run_py_raw_protocol_test(
        scripts: &[(&str, &str)],
        commands: Vec<ProtocolCmd>,
    ) -> Vec<DedicatedWorkerResult> {
        let temp_dir = tempfile::tempdir().unwrap();
        create_py_worker_files(temp_dir.path(), scripts);

        let mut child = Command::new("python3")
            .args(["-u", "-m", "wrapper"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(temp_dir.path())
            .spawn()
            .expect("Failed to spawn python3 process");

        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);

        let mut start_line = String::new();
        reader.read_line(&mut start_line).unwrap();
        assert_eq!(
            parse_dedicated_worker_line(start_line.trim()),
            DedicatedWorkerResult::Start,
        );

        let mut results = Vec::new();

        for cmd in &commands {
            let line = match cmd {
                ProtocolCmd::Exec { path, args } => format!("exec:{}:{}", path, args),
                ProtocolCmd::ExecPreprocess { path, args } => {
                    format!("exec_preprocess:{}:{}", path, args)
                }
                ProtocolCmd::Execd { args } => format!("execd:{}", args),
                ProtocolCmd::ExecdPreprocess { args } => format!("execd_preprocess:{}", args),
            };
            writeln!(stdin, "{}", line).unwrap();
            stdin.flush().unwrap();

            let expected_lines = match cmd {
                ProtocolCmd::ExecPreprocess { .. } | ProtocolCmd::ExecdPreprocess { .. } => 2,
                ProtocolCmd::Exec { .. } | ProtocolCmd::Execd { .. } => 1,
            };

            for _ in 0..expected_lines {
                let mut response = String::new();
                reader.read_line(&mut response).unwrap();
                let parsed = parse_dedicated_worker_line(response.trim());
                if matches!(parsed, DedicatedWorkerResult::Error(_)) {
                    results.push(parsed);
                    break;
                }
                results.push(parsed);
            }
        }

        writeln!(stdin, "end").unwrap();
        stdin.flush().unwrap();
        let _ = child.wait().expect("Worker process failed to exit");

        results
    }

    #[test]
    fn test_python_exec_preprocess() {
        let script = r#"
def preprocessor(x: int):
    return {"x": x * 10}

def main(x: int):
    return x + 1
"#;
        let results = run_py_raw_protocol_test(
            &[("f/test/pre", script)],
            vec![ProtocolCmd::ExecPreprocess {
                path: "f/test/pre".to_string(),
                args: serde_json::json!({"x": 5}),
            }],
        );
        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0],
            DedicatedWorkerResult::PreprocessedArgs(serde_json::json!({"x": 50}))
        );
        // main(50) => 51
        assert_eq!(
            results[1],
            DedicatedWorkerResult::Success(serde_json::json!(51))
        );
    }

    #[test]
    fn test_python_exec_preprocess_missing_preprocessor() {
        let script = "def main(x: int):\n    return x\n";
        let results = run_py_raw_protocol_test(
            &[("f/test/nopre", script)],
            vec![ProtocolCmd::ExecPreprocess {
                path: "f/test/nopre".to_string(),
                args: serde_json::json!({"x": 5}),
            }],
        );
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], DedicatedWorkerResult::Error(_)));
    }

    #[test]
    fn test_python_exec_preprocess_then_exec() {
        let script = r#"
def preprocessor(x: int):
    return {"x": x * 2}

def main(x: int):
    return x + 100
"#;
        let results = run_py_raw_protocol_test(
            &[("f/test/mixed", script)],
            vec![
                ProtocolCmd::ExecPreprocess {
                    path: "f/test/mixed".to_string(),
                    args: serde_json::json!({"x": 5}),
                },
                ProtocolCmd::Exec {
                    path: "f/test/mixed".to_string(),
                    args: serde_json::json!({"x": 7}),
                },
            ],
        );
        // preprocess: preprocessor(5) => {"x":10}, main(10) => 110
        // exec: main(7) => 107
        assert_eq!(results.len(), 3);
        assert_eq!(
            results[0],
            DedicatedWorkerResult::PreprocessedArgs(serde_json::json!({"x": 10}))
        );
        assert_eq!(
            results[1],
            DedicatedWorkerResult::Success(serde_json::json!(110))
        );
        assert_eq!(
            results[2],
            DedicatedWorkerResult::Success(serde_json::json!(107))
        );
    }

    // ==================== execd / execd_preprocess Tests ====================

    #[test]
    fn test_python_execd_single_script() {
        let results = run_py_raw_protocol_test(
            &[(
                "f/test/add",
                "def main(a: int, b: int):\n    return a + b\n",
            )],
            vec![ProtocolCmd::Execd { args: serde_json::json!({"a": 3, "b": 4}) }],
        );
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            DedicatedWorkerResult::Success(serde_json::json!(7))
        );
    }

    #[test]
    fn test_python_execd_preprocess() {
        let script = r#"
def preprocessor(x: int):
    return {"x": x * 10}

def main(x: int):
    return x + 1
"#;
        let results = run_py_raw_protocol_test(
            &[("f/test/pre", script)],
            vec![ProtocolCmd::ExecdPreprocess { args: serde_json::json!({"x": 5}) }],
        );
        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0],
            DedicatedWorkerResult::PreprocessedArgs(serde_json::json!({"x": 50}))
        );
        // main(50) => 51
        assert_eq!(
            results[1],
            DedicatedWorkerResult::Success(serde_json::json!(51))
        );
    }

    #[test]
    fn test_python_execd_preprocess_missing_preprocessor() {
        let script = "def main(x: int):\n    return x\n";
        let results = run_py_raw_protocol_test(
            &[("f/test/nopre", script)],
            vec![ProtocolCmd::ExecdPreprocess { args: serde_json::json!({"x": 5}) }],
        );
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], DedicatedWorkerResult::Error(_)));
    }

    #[test]
    fn test_python_execd_preprocess_then_execd() {
        let script = r#"
def preprocessor(x: int):
    return {"x": x * 2}

def main(x: int):
    return x + 100
"#;
        let results = run_py_raw_protocol_test(
            &[("f/test/mixed", script)],
            vec![
                ProtocolCmd::ExecdPreprocess { args: serde_json::json!({"x": 5}) },
                ProtocolCmd::Execd { args: serde_json::json!({"x": 7}) },
            ],
        );
        // preprocess: preprocessor(5) => {"x":10}, main(10) => 110
        // execd: main(7) => 107
        assert_eq!(results.len(), 3);
        assert_eq!(
            results[0],
            DedicatedWorkerResult::PreprocessedArgs(serde_json::json!({"x": 10}))
        );
        assert_eq!(
            results[1],
            DedicatedWorkerResult::Success(serde_json::json!(110))
        );
        assert_eq!(
            results[2],
            DedicatedWorkerResult::Success(serde_json::json!(107))
        );
    }

    // ==================== Argument Transformation Tests ====================

    #[test]
    fn test_python_datetime_arg_transformation() {
        let script = r#"
from datetime import datetime

def main(d: datetime):
    return d.isoformat()
"#;
        let results = run_py_single_script_test(
            "f/test/dt",
            script,
            vec![serde_json::json!({"d": "2024-01-15T10:30:00+00:00"})],
        );
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            Ok(serde_json::json!("2024-01-15T10:30:00+00:00"))
        );
    }

    #[test]
    fn test_python_bytes_arg_transformation() {
        let script = r#"
def main(data: bytes):
    return len(data)
"#;
        // base64 of "hello" is "aGVsbG8="
        let results = run_py_single_script_test(
            "f/test/bytes",
            script,
            vec![serde_json::json!({"data": "aGVsbG8="})],
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], Ok(serde_json::json!(5)));
    }

    #[test]
    fn test_python_kwargs_filtering() {
        // Test that extra kwargs are filtered out and only declared args are passed
        let script = "def main(a: int, b: int):\n    return a + b\n";
        let results = run_py_single_script_test(
            "f/test/kwargs",
            script,
            vec![serde_json::json!({"a": 1, "b": 2, "extra": 99})],
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], Ok(serde_json::json!(3)));
    }

    #[test]
    fn test_python_function_call_sentinel_removal() {
        // Test that '<function call>' sentinel values are removed from args
        let script = "def main(a: int, b: int = 10):\n    return a + b\n";
        let results = run_py_single_script_test(
            "f/test/sentinel",
            script,
            vec![serde_json::json!({"a": 5, "b": "<function call>"})],
        );
        assert_eq!(results.len(), 1);
        // b should be removed (sentinel), default 10 used
        assert_eq!(results[0], Ok(serde_json::json!(15)));
    }

    // ==================== Relative Import Tests ====================

    #[test]
    fn test_python_dedicated_worker_with_relative_import_detection() {
        // Test that the wrapper includes 'import loader' when scripts have relative imports
        let script_with_relative = "from f.helper import util\ndef main(x: int):\n    return x\n";
        let cg = compute_py_codegen(script_with_relative, "f/test/rel");
        let entries = [PyScriptEntry { original_path: "f/test/rel", codegen: &cg }];
        let wrapper = generate_py_multi_script_wrapper(&entries, false, true);
        assert!(
            wrapper.contains("import loader"),
            "wrapper should contain 'import loader' when any_relative_imports=true"
        );

        // Without relative imports
        let script_no_relative = "def main(x: int):\n    return x\n";
        let cg2 = compute_py_codegen(script_no_relative, "f/test/norel");
        let entries2 = [PyScriptEntry { original_path: "f/test/norel", codegen: &cg2 }];
        let wrapper2 = generate_py_multi_script_wrapper(&entries2, false, false);
        assert!(
            !wrapper2.contains("import loader"),
            "wrapper should NOT contain 'import loader' when any_relative_imports=false"
        );
    }
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "lockfile_python"))]
async fn test_requirements_python(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"# py: ==3.11.11
# requirements:
# tiny==0.1.3

import bar
import baz # pin: foo
import baz # repin: fee
import bug # repin: free
    
def main():
    pass
"#
    .to_string();

    assert_lockfile(
        &db,
        content,
        ScriptLang::Python3,
        vec![
            "# workspace-dependencies-mode: manual\n# py: 3.11.11",
            "tiny==0.1.3",
        ],
    )
    .await?;
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "lockfile_python"))]
async fn test_extra_requirements_python(db: Pool<Postgres>) -> anyhow::Result<()> {
    {
        use windmill_common::scripts::ScriptLang;

        let content = r#"# py: ==3.11.11
# extra_requirements:
# tiny

import f.system.extra_requirements
import tiny # pin: tiny==0.1.0
import tiny # pin: tiny==0.1.1
import tiny # repin: tiny==0.1.2

def main():
    pass
    "#
        .to_string();

        assert_lockfile(
            &db,
            content,
            ScriptLang::Python3,
            vec![
                "# workspace-dependencies-mode: extra\n# py: 3.11.11",
                "bottle==0.13.2",
                "tiny==0.1.2",
            ],
        )
        .await?;
    }
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "lockfile_python"))]
async fn test_extra_requirements_python2(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"# py: ==3.11.11
# extra_requirements:
# tiny==0.1.3

import simplejson # pin: simplejson==3.20.1
def main():
    pass
"#
    .to_string();

    assert_lockfile(
        &db,
        content,
        ScriptLang::Python3,
        vec![
            "# workspace-dependencies-mode: extra\n# py: 3.11.11",
            "simplejson==3.20.1",
            "tiny==0.1.3",
        ],
    )
    .await?;
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "lockfile_python"))]
async fn test_pins_python(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"# py: ==3.11.11
# extra_requirements:
# tiny==0.1.3
# bottle==0.13.2

import f.system.requirements
import f.system.pins
import tiny # repin: tiny==0.1.3
import simplejson

def main():
    pass
"#
    .to_string();

    assert_lockfile(
        &db,
        content,
        ScriptLang::Python3,
        vec![
            "# workspace-dependencies-mode: extra\n# py: 3.11.11",
            "bottle==0.13.2",
            "microdot==2.2.0",
            "simplejson==3.19.3",
            "tiny==0.1.3",
        ],
    )
    .await?;
    Ok(())
}
#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "multipython"))]
async fn test_multipython_python(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"# py: <=3.12.2, >=3.12.0
import f.multipython.script1
import f.multipython.aliases
"#
    .to_string();

    assert_lockfile(&db, content, ScriptLang::Python3, vec!["# py: 3.12.1\n"]).await?;
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "multipython"))]
async fn test_inline_script_metadata_python(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"# py_select_latest
# /// script
# requires-python = ">3.11,<3.12.3,!=3.12.2"
# dependencies = [
#   "tiny==0.1.3",
# ]
# ///
"#
    .to_string();

    assert_lockfile(
        &db,
        content,
        ScriptLang::Python3,
        vec!["# py: 3.12.1", "tiny==0.1.3"],
    )
    .await?;
    Ok(())
}

#[cfg(feature = "python")]
use windmill_common::jobs::JobPayload;
#[cfg(feature = "python")]
use windmill_common::jobs::RawCode;

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
def main():
    return "hello world"
        "#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        modules: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, false, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!("hello world"));
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_global_site_packages(db: Pool<Postgres>) -> anyhow::Result<()> {
    use windmill_common::worker::ROOT_CACHE_DIR;

    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Shared for all 3.12.*
    let path = format!("{}python_3_12/global-site-packages", *ROOT_CACHE_DIR);
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path + "/my_global_site_package_3_12_any.py", "").unwrap();

    // 3.12
    {
        let content = r#"# py: ==3.12
#requirements:
#

import my_global_site_package_3_12_any

def main():
    return "hello world"
                "#
        .to_owned();

        let job = JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            language: ScriptLang::Python3,
            lock: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            modules: None,
        });

        let result = run_job_in_new_worker_until_complete(&db, false, job, port)
            .await
            .json_result()
            .unwrap();

        assert_eq!(result, serde_json::json!("hello world"));
    }

    // 3.12.1
    {
        let content = r#"# py: ==3.12.1
#requirements:
#

import my_global_site_package_3_12_any

def main():
    return "hello world"
                "#
        .to_owned();

        let job = JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            language: ScriptLang::Python3,
            lock: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            modules: None,
        });

        let result = run_job_in_new_worker_until_complete(&db, false, job, port)
            .await
            .json_result()
            .unwrap();

        assert_eq!(result, serde_json::json!("hello world"));
    }
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_job_heavy_dep(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
import numpy as np

def main():
    a = np.arange(15).reshape(3, 5)
    return len(a)
        "#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        modules: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, false, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!(3));
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_job_with_imports(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
import wmill

def main():
    return wmill.get_workspace()
        "#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        modules: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, false, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!("test-workspace"));
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "relative_python"))]
async fn test_relative_imports_python(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"
from f.system.same_folder_script import main as test1
from .same_folder_script import main as test2
from f.system_relative.different_folder_script import main as test3
from ..system_relative.different_folder_script import main as test4
    
def main():
    return [test1(), test2(), test3(), test4()]
"#
    .to_string();

    run_deployed_relative_imports(&db, content.clone(), ScriptLang::Python3).await?;
    run_preview_relative_imports(&db, content, ScriptLang::Python3).await?;
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "relative_python"))]
async fn test_nested_imports_python(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"

from f.system_relative.nested_script import main as test

def main():
    return test()
"#
    .to_string();

    run_deployed_relative_imports(&db, content.clone(), ScriptLang::Python3).await?;
    run_preview_relative_imports(&db, content, ScriptLang::Python3).await?;
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_wac_v2_with_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
from wmill import task, workflow

@task()
def greet(label: str, count: int) -> str:
    return f"hello {label} x{count}"

@workflow
async def main(item: str, qty: int, email: str):
    greeting = await greet(item, qty)
    return {"item": item, "qty": qty, "email": email, "greeting": greeting}
"#
    .to_string();

    // WAC requires at least 2 workers (parent + task sub-jobs)
    let db = &db;
    in_test_worker(
        db,
        async move {
            let job = Box::pin(
                RunJob::from(JobPayload::Code(RawCode {
                    language: ScriptLang::Python3,
                    content,
                    ..RawCode::default()
                }))
                .arg("item", json!("widget"))
                .arg("qty", json!(5))
                .arg("email", json!("test@example.com"))
                .run_until_complete(db, false, port),
            )
            .await;

            let result = job.json_result().unwrap();
            assert_eq!(result["item"], json!("widget"));
            assert_eq!(result["qty"], json!(5));
            assert_eq!(result["email"], json!("test@example.com"));
            assert_eq!(result["greeting"], json!("hello widget x5"));
        },
        port,
    )
    .await;
    Ok(())
}

/// Reproducer for issue #8946: WAC scripts (`auto_kind = 'wac'`) must show up
/// in `GET /api/w/{workspace}/scripts/list?kinds=script`. The previous filter
/// hid them by requiring `auto_kind IS NULL`, breaking trigger configuration
/// dropdowns.
#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "wac_preprocessor"))]
async fn test_scripts_list_includes_wac(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let url = format!(
        "http://localhost:{port}/api/w/test-workspace/scripts/list?kinds=script&without_description=true"
    );
    let resp = reqwest::Client::new()
        .get(&url)
        .header("Authorization", "Bearer SECRET_TOKEN")
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "scripts/list failed: {}",
        resp.status()
    );
    let scripts: Vec<serde_json::Value> = resp.json().await?;
    let paths: Vec<&str> = scripts.iter().filter_map(|s| s["path"].as_str()).collect();
    assert!(
        paths.contains(&"f/system/wac_with_preprocessor"),
        "WAC script missing from scripts/list response. Got: {:?}",
        paths
    );
    Ok(())
}

/// Reproducer for issue #8947: a Python WAC script that defines a
/// `preprocessor` function alongside the `@workflow` entrypoint must run the
/// preprocessor on the trigger payload before invoking the workflow, persist
/// the preprocessed args to `v2_job.args`, and dispatch inline child re-runs
/// using the post-preprocessor args (so children of the WAC parent see the
/// transformed shape, not the raw event).
#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "wac_preprocessor"))]
async fn test_python_wac_v2_with_preprocessor(db: Pool<Postgres>) -> anyhow::Result<()> {
    use windmill_common::scripts::ScriptHash;
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let db = &db;
    in_test_worker(
        db,
        async move {
            let job = Box::pin(
                RunJob::from(JobPayload::ScriptHash {
                    hash: ScriptHash(123419),
                    path: "f/system/wac_with_preprocessor".to_string(),
                    cache_ttl: None,
                    cache_ignore_s3_path: None,
                    dedicated_worker: None,
                    language: ScriptLang::Python3,
                    priority: None,
                    apply_preprocessor: true,
                    concurrency_settings:
                        windmill_common::runnable_settings::ConcurrencySettings::default(),
                    debouncing_settings:
                        windmill_common::runnable_settings::DebouncingSettings::default(),
                    labels: None,
                })
                .arg("who", json!("alice"))
                .arg("count", json!(7))
                .run_until_complete(db, false, port),
            )
            .await;

            let result = job.json_result().unwrap();
            // Preprocessor maps {who, count} -> {name, qty}. The @task
            // `shout` upper-cases the name; the workflow returns greeting + qty.
            assert_eq!(result["greeting"], json!("ALICE"));
            assert_eq!(result["qty"], json!(7));

            // Args column should now hold the preprocessed shape.
            let args: serde_json::Value =
                sqlx::query_scalar("SELECT args FROM v2_job WHERE id = $1")
                    .bind(job.id)
                    .fetch_one(db)
                    .await
                    .unwrap();
            assert_eq!(args["name"], json!("alice"));
            assert_eq!(args["qty"], json!(7));

            let preprocessed: Option<bool> =
                sqlx::query_scalar("SELECT preprocessed FROM v2_job WHERE id = $1")
                    .bind(job.id)
                    .fetch_one(db)
                    .await
                    .unwrap();
            assert_eq!(preprocessed, Some(true));
        },
        port,
    )
    .await;
    Ok(())
}

/// End-to-end comparison between the legacy `step()` suspend-and-replay path
/// and the new SDK inline-persist fast path, toggled per-job via the
/// `WM_WAC_INLINE_FAST_PATH` env var which the Python script sets on its own
/// `os.environ` at the top so parallel tests can't race on a global env var.
///
/// Runs the same 5-step WAC v2 Python workflow twice, asserts both modes
/// produce the same final result and the same `completed_steps` map, and
/// prints a wall-clock benchmark line so CI and manual runs can track the
/// speedup. The fast path is expected to be faster because it avoids N-1
/// subprocess spawns and N-1 queue round-trips for a workflow with N
/// `step()` calls, but the exact ratio depends on the CI runner so we don't
/// assert a hard threshold — behavioral equivalence is the important check.
#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_wac_v2_step_inline_fast_path(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    fn workflow_content(fast_path_enabled: bool) -> String {
        let flag = if fast_path_enabled { "1" } else { "0" };
        // NB: the `os.environ[...] = ...` line must execute BEFORE the first
        // `step()` call inside the workflow subprocess — setting it at module
        // scope (before `from wmill import ...` finishes importing is still
        // fine because the SDK reads it lazily at call time, not at import).
        format!(
            r#"import os
os.environ["WM_WAC_INLINE_FAST_PATH"] = "{flag}"
from wmill import workflow, step

@workflow
async def main(n: int):
    a = await step("a", lambda: n)
    b = await step("b", lambda: a + 1)
    c = await step("c", lambda: b + 1)
    d = await step("d", lambda: c + 1)
    e = await step("e", lambda: d + 1)
    return {{"a": a, "b": b, "c": c, "d": d, "e": e}}
"#
        )
    }

    let db_ref = &db;

    async fn run_once(
        db: &Pool<Postgres>,
        port: u16,
        content: String,
    ) -> (serde_json::Value, serde_json::Value, std::time::Duration) {
        let mut job_id_out: Option<sqlx::types::Uuid> = None;
        let mut result_out: Option<serde_json::Value> = None;
        let t0 = std::time::Instant::now();
        in_test_worker(
            db,
            async {
                let job = Box::pin(
                    RunJob::from(JobPayload::Code(RawCode {
                        language: ScriptLang::Python3,
                        content,
                        ..RawCode::default()
                    }))
                    .arg("n", json!(1))
                    .run_until_complete(db, false, port),
                )
                .await;
                result_out = Some(job.json_result().unwrap_or_else(|| {
                    panic!("job {} returned no result — raw job = {:?}", job.id, job)
                }));
                job_id_out = Some(job.id);
            },
            port,
        )
        .await;
        let elapsed = t0.elapsed();

        let job_id = job_id_out.expect("job id");
        // Fetch the full workflow_as_code_status for diagnostics, then extract
        // _checkpoint.completed_steps. A None here means the step() path never
        // wrote the checkpoint, which is exactly the signal we want to surface
        // clearly (instead of panicking with an opaque Option::unwrap() error).
        let full_status: Option<serde_json::Value> = sqlx::query_scalar!(
            r#"SELECT workflow_as_code_status as "v: serde_json::Value"
               FROM v2_job_completed WHERE id = $1"#,
            job_id
        )
        .fetch_one(db)
        .await
        .expect("v2_job_completed row fetch");

        let ckpt = full_status
            .as_ref()
            .and_then(|s| s.get("_checkpoint"))
            .and_then(|c| c.get("completed_steps"))
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "job {job_id} completed_steps missing — full workflow_as_code_status = {:?}, job_result = {:?}",
                    full_status, result_out
                )
            });

        (result_out.unwrap(), ckpt, elapsed)
    }

    // --- Legacy path: worker-side suspend & replay ---
    let (legacy_result, legacy_ckpt, legacy_elapsed) =
        run_once(db_ref, port, workflow_content(false)).await;

    // --- Fast path: SDK persists the delta via the new API endpoint ---
    let (fast_result, fast_ckpt, fast_elapsed) =
        run_once(db_ref, port, workflow_content(true)).await;

    // Behavioral equivalence: same final result and same completed_steps.
    assert_eq!(
        legacy_result, fast_result,
        "legacy and fast path produced different workflow results"
    );
    assert_eq!(
        legacy_result,
        json!({"a": 1, "b": 2, "c": 3, "d": 4, "e": 5}),
        "unexpected workflow result"
    );
    assert_eq!(
        legacy_ckpt, fast_ckpt,
        "legacy and fast path stored different completed_steps in the checkpoint"
    );

    // Benchmark output. We log and print but do NOT assert a hard threshold:
    // CI runners have variable noise floors and a "fast < legacy" check would
    // flake. Behavioral equivalence above is the important invariant.
    let legacy_ms = legacy_elapsed.as_millis();
    let fast_ms = fast_elapsed.as_millis();
    let speedup = if fast_ms > 0 {
        legacy_ms as f64 / fast_ms as f64
    } else {
        f64::INFINITY
    };
    tracing::info!(
        "WAC v2 step() benchmark: legacy={}ms fast={}ms speedup={:.2}x",
        legacy_ms,
        fast_ms,
        speedup
    );
    println!(
        "WAC v2 step() benchmark: legacy={}ms fast={}ms speedup={:.2}x",
        legacy_ms, fast_ms, speedup
    );
    Ok(())
}
