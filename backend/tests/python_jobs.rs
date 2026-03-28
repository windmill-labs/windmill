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
            };
            writeln!(stdin, "{}", line).unwrap();
            stdin.flush().unwrap();

            let expected_lines = match cmd {
                ProtocolCmd::ExecPreprocess { .. } => 2,
                ProtocolCmd::Exec { .. } => 1,
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

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "typechecked_python"))]
async fn test_typechecked_decorator_python(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
from f.system.typechecked_helper import greet

def main():
    return greet("World")
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: Some("f/system/test_typechecked".to_string()),
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

    let result_str = result.as_str().unwrap();
    assert!(result_str.starts_with("Hello, World! from "), "unexpected result: {result_str}");
    Ok(())
}
