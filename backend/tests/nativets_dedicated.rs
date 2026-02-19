/*
 * Protocol-level tests for the V8 native dedicated worker (`run_dedicated_loop`).
 *
 * These mirror the bun/node dedicated worker protocol tests in `bun_jobs.rs`
 * but use in-memory `DuplexStream` pairs instead of subprocess stdio.
 *
 * Run with:
 *   cargo test -p windmill --features "deno_core" --test nativets_dedicated -- --nocapture
 */

#[cfg(feature = "deno_core")]
mod v8_dedicated_worker_protocol {
    use std::process::Command;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use windmill_test_utils::{parse_dedicated_worker_line, DedicatedWorkerResult};
    use windmill_worker::{build_loader, LoaderMode, BUN_PATH};

    /// Bundle a TypeScript script into JS suitable for `run_dedicated_loop`.
    ///
    /// Returns `(ts_source, js_bundle)`.
    async fn bundle_script(script: &str) -> (String, String) {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir = temp_dir.path();
        let dir_str = dir.to_str().unwrap();

        std::fs::write(dir.join("main.ts"), script).unwrap();

        build_loader(
            dir_str,
            "http://localhost:8000",
            "test_token",
            "test-workspace",
            "f/test/script",
            LoaderMode::BrowserBundle,
        )
        .await
        .expect("build_loader failed");

        let output = Command::new(BUN_PATH.as_str())
            .args(["run", dir.join("node_builder.ts").to_str().unwrap()])
            .current_dir(dir)
            .output()
            .expect("Failed to run bun build");

        if !output.status.success() {
            panic!(
                "Bun build failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let ts = std::fs::read_to_string(dir.join("main.ts")).unwrap();
        let js = std::fs::read_to_string(dir.join("main.js")).unwrap();
        (ts, js)
    }

    const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

    /// Run `run_dedicated_loop` with a bundled script and feed it `jobs`,
    /// collecting one result line per job. Times out after 30s to prevent
    /// hangs if V8 or the protocol deadlocks.
    async fn run_v8_dedicated_test(
        script: &str,
        jobs: Vec<serde_json::Value>,
    ) -> Vec<Result<serde_json::Value, String>> {
        tokio::time::timeout(TEST_TIMEOUT, run_v8_dedicated_test_inner(script, jobs))
            .await
            .expect("test timed out after 30s")
    }

    async fn run_v8_dedicated_test_inner(
        script: &str,
        jobs: Vec<serde_json::Value>,
    ) -> Vec<Result<serde_json::Value, String>> {
        windmill_runtime_nativets::setup_deno_runtime().expect("V8 init failed");

        let (ts, js) = bundle_script(script).await;

        let (stdin_w, stdin_r) = tokio::io::duplex(65536);
        let (stdout_w, stdout_r) = tokio::io::duplex(65536);

        let v8_handle = tokio::spawn(async move {
            windmill_runtime_nativets::run_dedicated_loop("".into(), ts, js, stdin_r, stdout_w)
                .await
        });

        let mut reader = BufReader::new(stdout_r).lines();
        let mut writer = stdin_w;

        // Wait for "start" signal
        let start_line = reader.next_line().await.unwrap().expect("no start line");
        assert_eq!(
            parse_dedicated_worker_line(start_line.trim()),
            DedicatedWorkerResult::Start,
            "Expected 'start', got: {}",
            start_line.trim()
        );

        let mut results = Vec::new();

        for job_args in &jobs {
            let line = format!("{}\n", job_args);
            writer.write_all(line.as_bytes()).await.unwrap();
            writer.flush().await.unwrap();

            // Read lines until we get a wm_res line (skip log lines)
            loop {
                let response = reader.next_line().await.unwrap().expect("no response line");
                match parse_dedicated_worker_line(response.trim()) {
                    DedicatedWorkerResult::Success(value) => {
                        results.push(Ok(value));
                        break;
                    }
                    DedicatedWorkerResult::Error(err) => {
                        let msg = err["message"]
                            .as_str()
                            .unwrap_or("Unknown error")
                            .to_string();
                        results.push(Err(msg));
                        break;
                    }
                    DedicatedWorkerResult::Other(_) => {
                        // Skip log lines, continue reading
                        continue;
                    }
                    DedicatedWorkerResult::Start => {
                        panic!("Unexpected second 'start' signal");
                    }
                }
            }
        }

        // Signal end
        writer.write_all(b"end\n").await.unwrap();
        writer.flush().await.unwrap();
        drop(writer);

        v8_handle
            .await
            .expect("V8 task panicked")
            .expect("V8 loop failed");

        results
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_v8_dedicated_simple() {
        let script = r#"
export function main(x: number, y: number): number {
    return x + y;
}
"#;
        let results =
            run_v8_dedicated_test(script, vec![serde_json::json!({"x": 2, "y": 3})]).await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], Ok(serde_json::json!(5)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_v8_dedicated_multiple_jobs() {
        let script = r#"
export function main(n: number): number {
    return n * 2;
}
"#;
        let jobs: Vec<serde_json::Value> = (1..=5).map(|i| serde_json::json!({"n": i})).collect();
        let results = run_v8_dedicated_test(script, jobs).await;

        assert_eq!(results.len(), 5);
        for (i, result) in results.iter().enumerate() {
            let expected = ((i + 1) * 2) as i64;
            assert_eq!(*result, Ok(serde_json::json!(expected)));
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_v8_dedicated_error() {
        let script = r#"
export function main(msg: string): never {
    throw new Error(msg);
}
"#;
        let results =
            run_v8_dedicated_test(script, vec![serde_json::json!({"msg": "test error"})]).await;

        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
        assert!(
            results[0].as_ref().unwrap_err().contains("test error"),
            "Error should contain 'test error', got: {}",
            results[0].as_ref().unwrap_err()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_v8_dedicated_async() {
        let script = r#"
export async function main(x: number): Promise<number> {
    const val = await Promise.resolve(x * 10);
    return val + 1;
}
"#;
        let results = run_v8_dedicated_test(script, vec![serde_json::json!({"x": 7})]).await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], Ok(serde_json::json!(71)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_v8_dedicated_complex_return() {
        let script = r#"
export function main(name: string, items: number[]): any {
    return {
        greeting: `hello ${name}`,
        sum: items.reduce((a, b) => a + b, 0),
        items: items.map(x => x * 2),
    };
}
"#;
        let results = run_v8_dedicated_test(
            script,
            vec![serde_json::json!({"name": "world", "items": [1, 2, 3]})],
        )
        .await;

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            Ok(serde_json::json!({
                "greeting": "hello world",
                "sum": 6,
                "items": [2, 4, 6],
            }))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_v8_dedicated_null_undefined() {
        let script = r#"
export function main(returnNull: boolean): any {
    if (returnNull) {
        return null;
    }
    return undefined;
}
"#;
        let results = run_v8_dedicated_test(
            script,
            vec![
                serde_json::json!({"returnNull": true}),
                serde_json::json!({"returnNull": false}),
            ],
        )
        .await;

        assert_eq!(results.len(), 2);
        // Both null and undefined should serialize to JSON null
        assert_eq!(results[0], Ok(serde_json::Value::Null));
        assert_eq!(results[1], Ok(serde_json::Value::Null));
    }
}
