/*
 * Tests for the PrewarmedIsolate used by nativets dedicated workers.
 *
 * Run with:
 *   cargo test -p windmill --features "deno_core" --test nativets_dedicated -- --nocapture
 */

#[cfg(feature = "deno_core")]
mod prewarmed_isolate_tests {
    use std::process::Command;
    use windmill_runtime_nativets::{NativeAnnotation, PrewarmedIsolate};
    use windmill_worker::{build_loader, LoaderMode, BUN_PATH};

    fn default_annotation() -> NativeAnnotation {
        NativeAnnotation { useragent: None, proxy: None }
    }

    /// Bundle a TypeScript script into JS suitable for `PrewarmedIsolate`.
    ///
    /// Returns `(ts_source, js_bundle, arg_names)`.
    async fn bundle_script(script: &str) -> (String, String, Vec<String>) {
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
        let parsed = windmill_parser_ts::parse_deno_signature(&ts, true, false, None)
            .expect("failed to parse signature");
        let arg_names: Vec<String> = parsed.args.into_iter().map(|a| a.name).collect();
        (ts, js, arg_names)
    }

    const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

    async fn run_prewarmed_test(
        script: &str,
        jobs: Vec<serde_json::Value>,
    ) -> Vec<Result<serde_json::Value, String>> {
        tokio::time::timeout(TEST_TIMEOUT, run_prewarmed_test_inner(script, jobs))
            .await
            .expect("test timed out after 30s")
    }

    async fn run_prewarmed_test_inner(
        script: &str,
        jobs: Vec<serde_json::Value>,
    ) -> Vec<Result<serde_json::Value, String>> {
        windmill_runtime_nativets::setup_deno_runtime().expect("V8 init failed");

        let (_ts, js, arg_names) = bundle_script(script).await;
        let ann = default_annotation();

        let mut results = Vec::new();

        for job_args in &jobs {
            let mut isolate =
                PrewarmedIsolate::spawn("".to_string(), js.clone(), ann.clone(), arg_names.clone());
            isolate.wait_ready().await.expect("isolate failed to warm");

            let args = serde_json::to_string(job_args).unwrap();
            let executing = isolate.start_execution(args);
            let prewarmed_result = executing.wait().await.expect("isolate execution failed");

            match prewarmed_result.result {
                Ok(raw) => {
                    let value: serde_json::Value =
                        serde_json::from_str(raw.get()).unwrap_or(serde_json::Value::Null);
                    results.push(Ok(value));
                }
                Err(e) => {
                    results.push(Err(e));
                }
            }
        }

        results
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_prewarmed_simple() {
        let script = r#"
export function main(x: number, y: number): number {
    return x + y;
}
"#;
        let results = run_prewarmed_test(script, vec![serde_json::json!({"x": 2, "y": 3})]).await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], Ok(serde_json::json!(5)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_prewarmed_isolation() {
        let script = r#"
let counter = 0;
export function main(): number {
    counter++;
    return counter;
}
"#;
        let results =
            run_prewarmed_test(script, vec![serde_json::json!({}), serde_json::json!({})]).await;

        assert_eq!(results.len(), 2);
        // Each job gets a fresh isolate, so counter should be 1 both times
        assert_eq!(results[0], Ok(serde_json::json!(1)));
        assert_eq!(results[1], Ok(serde_json::json!(1)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_prewarmed_error() {
        let script = r#"
export function main(msg: string): never {
    throw new Error(msg);
}
"#;
        let results =
            run_prewarmed_test(script, vec![serde_json::json!({"msg": "test error"})]).await;

        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
        assert!(
            results[0].as_ref().unwrap_err().contains("test error"),
            "Error should contain 'test error', got: {}",
            results[0].as_ref().unwrap_err()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_prewarmed_async() {
        let script = r#"
export async function main(x: number): Promise<number> {
    const val = await Promise.resolve(x * 10);
    return val + 1;
}
"#;
        let results = run_prewarmed_test(script, vec![serde_json::json!({"x": 7})]).await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], Ok(serde_json::json!(71)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_prewarmed_pipeline() {
        windmill_runtime_nativets::setup_deno_runtime().expect("V8 init failed");

        let script = r#"
export function main(n: number): number {
    return n * 2;
}
"#;
        let (_ts, js, arg_names) = bundle_script(script).await;
        let ann = default_annotation();

        // Pre-warm first isolate
        let mut warm =
            PrewarmedIsolate::spawn("".to_string(), js.clone(), ann.clone(), arg_names.clone());
        warm.wait_ready()
            .await
            .expect("first isolate failed to warm");

        let mut results = Vec::new();

        for i in 1..=3 {
            let args = serde_json::to_string(&serde_json::json!({"n": i})).unwrap();
            let executing = warm.start_execution(args);

            // Pipeline: start pre-warming next isolate while current one runs
            warm =
                PrewarmedIsolate::spawn("".to_string(), js.clone(), ann.clone(), arg_names.clone());

            let prewarmed_result = executing.wait().await.expect("isolate execution failed");
            match prewarmed_result.result {
                Ok(raw) => {
                    let value: serde_json::Value =
                        serde_json::from_str(raw.get()).unwrap_or(serde_json::Value::Null);
                    results.push(value);
                }
                Err(e) => panic!("unexpected error: {e}"),
            }

            warm.wait_ready()
                .await
                .expect("next isolate failed to warm");
        }

        assert_eq!(
            results,
            vec![
                serde_json::json!(2),
                serde_json::json!(4),
                serde_json::json!(6),
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_prewarmed_complex_return() {
        let script = r#"
export function main(name: string, items: number[]): any {
    return {
        greeting: `hello ${name}`,
        sum: items.reduce((a, b) => a + b, 0),
        items: items.map(x => x * 2),
    };
}
"#;
        let results = run_prewarmed_test(
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
    async fn test_prewarmed_null_undefined() {
        let script = r#"
export function main(returnNull: boolean): any {
    if (returnNull) {
        return null;
    }
    return undefined;
}
"#;
        let results = run_prewarmed_test(
            script,
            vec![
                serde_json::json!({"returnNull": true}),
                serde_json::json!({"returnNull": false}),
            ],
        )
        .await;

        assert_eq!(results.len(), 2);
        assert_eq!(results[0], Ok(serde_json::Value::Null));
        assert_eq!(results[1], Ok(serde_json::Value::Null));
    }
}
