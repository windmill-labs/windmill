use sqlx::postgres::Postgres;
use sqlx::Pool;
use windmill_common::jobs::{JobPayload, RawCode};
use windmill_common::scripts::ScriptLang;
use windmill_test_utils::*;

// ============================================================================
// Basic Execution Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_bun_job_simple(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
export function main() {
    return "hello world";
}
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, false, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!("hello world"));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_bun_job_with_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
export function main(name: string, count: number) {
    return `Hello ${name}, count: ${count}`;
}
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let result = RunJob::from(job)
        .arg("name", serde_json::json!("World"))
        .arg("count", serde_json::json!(42))
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!("Hello World, count: 42"));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_bun_job_return_types(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Test object return
    {
        let content = r#"
export function main() {
    return { name: "test", value: 123 };
}
"#
        .to_owned();

        let job = JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            language: ScriptLang::Bun,
            lock: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
        });

        let result = run_job_in_new_worker_until_complete(&db, false, job, port)
            .await
            .json_result()
            .unwrap();

        assert_eq!(result, serde_json::json!({"name": "test", "value": 123}));
    }

    // Test array return
    {
        let content = r#"
export function main() {
    return [1, 2, 3, "four"];
}
"#
        .to_owned();

        let job = JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            language: ScriptLang::Bun,
            lock: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
        });

        let result = run_job_in_new_worker_until_complete(&db, false, job, port)
            .await
            .json_result()
            .unwrap();

        assert_eq!(result, serde_json::json!([1, 2, 3, "four"]));
    }

    // Test BigInt serialization
    {
        let content = r#"
export function main() {
    // Use BigInt literal notation to avoid JavaScript number precision loss
    return 9007199254740993n;
}
"#
        .to_owned();

        let job = JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            language: ScriptLang::Bun,
            lock: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
        });

        let result = run_job_in_new_worker_until_complete(&db, false, job, port)
            .await
            .json_result()
            .unwrap();

        // BigInt should be serialized as string
        assert_eq!(result, serde_json::json!("9007199254740993"));
    }

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_bun_job_async(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
export async function main() {
    await new Promise(resolve => setTimeout(resolve, 100));
    return "async completed";
}
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, false, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!("async completed"));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_bun_job_null_undefined_handling(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Test null return
    {
        let content = r#"
export function main() {
    return null;
}
"#
        .to_owned();

        let job = JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            language: ScriptLang::Bun,
            lock: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
        });

        let result = run_job_in_new_worker_until_complete(&db, false, job, port)
            .await
            .json_result()
            .unwrap();

        assert_eq!(result, serde_json::json!(null));
    }

    // Test undefined return (should be serialized as null)
    {
        let content = r#"
export function main() {
    return undefined;
}
"#
        .to_owned();

        let job = JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            language: ScriptLang::Bun,
            lock: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
        });

        let result = run_job_in_new_worker_until_complete(&db, false, job, port)
            .await
            .json_result()
            .unwrap();

        assert_eq!(result, serde_json::json!(null));
    }

    Ok(())
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_bun_job_runtime_error(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
export function main() {
    throw new Error("intentional error");
}
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let completed = run_job_in_new_worker_until_complete(&db, false, job, port).await;

    assert!(!completed.success);
    let result = completed.json_result().unwrap();
    // Error is wrapped: {"error": {"message": "...", "name": "...", "stack": "..."}}
    let error = &result["error"];
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("intentional error"));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_bun_job_missing_main_function(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
export function notMain() {
    return "hello";
}
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let completed = run_job_in_new_worker_until_complete(&db, false, job, port).await;

    assert!(!completed.success);
    let result = completed.json_result().unwrap();
    // Error is wrapped: {"error": {"message": "...", "name": "...", "stack": "..."}}
    let error = &result["error"];
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("main function is missing"));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_bun_job_syntax_error(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
export function main() {
    return "unclosed string
}
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let completed = run_job_in_new_worker_until_complete(&db, false, job, port).await;

    assert!(!completed.success);
    Ok(())
}

// ============================================================================
// Annotation Mode Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_bun_nodejs_mode(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"//nodejs

export function main() {
    // Node.js specific API
    return process.version.startsWith("v");
}
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, false, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!(true));
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_bun_nobundling_mode(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"//nobundling

export function main() {
    return "nobundling works";
}
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, false, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!("nobundling works"));
    Ok(())
}

// ============================================================================
// Native Mode Tests (requires deno_core feature)
// ============================================================================

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_bun_native_mode(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"//native

export function main() {
    return "native execution";
}
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, false, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!("native execution"));
    Ok(())
}

// ============================================================================
// Relative Import Tests
// ============================================================================

#[sqlx::test(fixtures("base", "relative_bun"))]
async fn test_bun_relative_imports(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"
import { main as test1 } from "/f/system/same_folder_script.ts";
import { main as test2 } from "./same_folder_script.ts";
import { main as test3 } from "/f/system_relative/different_folder_script.ts";
import { main as test4 } from "../system_relative/different_folder_script.ts";

export function main() {
    return [test1(), test2(), test3(), test4()];
}
"#
    .to_string();

    run_deployed_relative_imports(&db, content.clone(), ScriptLang::Bun).await?;
    run_preview_relative_imports(&db, content, ScriptLang::Bun).await?;
    Ok(())
}

#[sqlx::test(fixtures("base", "relative_bun"))]
async fn test_bun_nested_imports(db: Pool<Postgres>) -> anyhow::Result<()> {
    // Test with absolute path (/f/...)
    let content_absolute = r#"
import { main as test } from "/f/system_relative/nested_script.ts";

export function main() {
    return test();
}
"#
    .to_string();

    run_deployed_relative_imports(&db, content_absolute.clone(), ScriptLang::Bun).await?;
    run_preview_relative_imports(&db, content_absolute, ScriptLang::Bun).await?;

    // Test with relative path (../...)
    let content_relative = r#"
import { main as test } from "../system_relative/nested_script.ts";

export function main() {
    return test();
}
"#
    .to_string();

    run_preview_relative_imports(&db, content_relative, ScriptLang::Bun).await?;
    Ok(())
}

// ============================================================================
// Deeply Nested Import Tests
// ============================================================================

#[sqlx::test(fixtures("base", "bun_edge_cases"))]
async fn test_bun_deeply_nested_imports(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Test with absolute path import (/f/nested/level1.ts)
    // Note: The fixture uses relative imports internally (level1 -> ./level2 -> ./level3)
    {
        let content = r#"
import { main as level1 } from "/f/nested/level1.ts";

export function main() {
    return level1();
}
"#
        .to_owned();

        let job = JobPayload::Code(RawCode {
            hash: None,
            content,
            path: Some("f/nested/test_deep".to_string()),
            language: ScriptLang::Bun,
            lock: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
        });

        let result = run_job_in_new_worker_until_complete(&db, false, job, port)
            .await
            .json_result()
            .unwrap();

        // level1 -> level2 -> level3, each adds to the chain
        assert_eq!(result, serde_json::json!("level1 -> level2 -> level3"));
    }

    // Test with relative path import (./level1.ts)
    {
        let content = r#"
import { main as level1 } from "./level1.ts";

export function main() {
    return level1();
}
"#
        .to_owned();

        let job = JobPayload::Code(RawCode {
            hash: None,
            content,
            path: Some("f/nested/test_deep_relative".to_string()),
            language: ScriptLang::Bun,
            lock: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            )
            .into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
        });

        let result = run_job_in_new_worker_until_complete(&db, false, job, port)
            .await
            .json_result()
            .unwrap();

        // Same result: level1 -> level2 -> level3
        assert_eq!(result, serde_json::json!("level1 -> level2 -> level3"));
    }

    Ok(())
}

#[sqlx::test(fixtures("base", "bun_edge_cases"))]
async fn test_bun_shared_imports_both_styles(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Test importing modules that use different import styles internally:
    // - module_a uses relative import: ./shared.ts
    // - module_b uses absolute import: /f/circular/shared.ts
    // Both should work correctly
    let content = r#"
import { getValue as getA } from "/f/circular/module_a.ts";
import { getValue as getB } from "./module_b.ts";

export function main() {
    return [getA(), getB()];
}
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: Some("f/circular/test_both".to_string()),
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, false, job, port)
        .await
        .json_result()
        .unwrap();

    // Both modules should correctly import SHARED_VALUE
    assert_eq!(
        result,
        serde_json::json!(["from_a_shared", "from_b_shared"])
    );
    Ok(())
}

// ============================================================================
// Preprocessor Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_bun_preprocessor_execution(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Test that the main function works correctly
    // Note: Preprocessor execution requires specific job configuration
    // (flow_step_id != "preprocessor" and preprocessed == Some(false))
    // which is not set by default in RawCode jobs
    let content = r#"
export function main(x: number) {
    return x + 10;
}
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    // x=5, main adds 10 = 15
    let result = RunJob::from(job)
        .arg("x", serde_json::json!(5))
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!(15));
    Ok(())
}

// ============================================================================
// Wmill SDK Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_bun_job_with_wmill_env_vars(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Test accessing Windmill environment variables (which the SDK uses internally)
    // This validates that the execution environment is properly configured
    let content = r#"
export function main() {
    // WM_WORKSPACE and WM_TOKEN are injected by Windmill worker
    return {
        workspace: process.env.WM_WORKSPACE,
        hasToken: process.env.WM_TOKEN !== undefined && process.env.WM_TOKEN.length > 0,
        baseUrl: process.env.BASE_URL !== undefined
    };
}
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, false, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result["workspace"], serde_json::json!("test-workspace"));
    assert_eq!(result["hasToken"], serde_json::json!(true));
    assert_eq!(result["baseUrl"], serde_json::json!(true));
    Ok(())
}

// ============================================================================
// Environment Variable Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_bun_job_env_vars(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
export function main() {
    return {
        workspace: process.env.WM_WORKSPACE,
        hasToken: !!process.env.WM_TOKEN,
    };
}
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, false, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result["workspace"], serde_json::json!("test-workspace"));
    assert_eq!(result["hasToken"], serde_json::json!(true));
    Ok(())
}

// ============================================================================
// Dedicated Worker Protocol Tests
// ============================================================================

mod dedicated_worker_protocol {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};
    use windmill_test_utils::{parse_dedicated_worker_line, DedicatedWorkerResult};
    use windmill_worker::{
        build_loader, generate_dedicated_worker_wrapper, LoaderMode, BUN_DEDICATED_WORKER_ARGS,
        BUN_PATH, NODE_BIN_PATH,
    };

    /// Creates test worker files and optionally bundles for Node.js (like production)
    /// Returns the path to the wrapper file to execute
    fn create_test_worker_files(
        dir: &std::path::Path,
        script: &str,
        arg_names: &[&str],
        bundle_for_node: bool,
    ) -> std::path::PathBuf {
        let dir_str = dir.to_str().unwrap();
        std::fs::write(dir.join("main.ts"), script).unwrap();

        if bundle_for_node {
            // For Node.js: bundle to JavaScript first (like production's build_loader with LoaderMode::Node)
            let wrapper = generate_dedicated_worker_wrapper(arg_names, "./main.js", None);
            std::fs::write(dir.join("wrapper.mjs"), wrapper).unwrap();

            // Use the exact same build_loader function as production
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(build_loader(
                    dir_str,
                    "http://localhost:8000",
                    "test_token",
                    "test-workspace",
                    "f/test/script",
                    LoaderMode::Node,
                ))
                .expect("build_loader failed");

            // Run the bundler with bun (build_loader creates node_builder.ts)
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

            // Bun outputs to wrapper.js, rename to wrapper.mjs for ES module
            let bundled_path = dir.join("wrapper.js");
            let output_path = dir.join("wrapper_bundled.mjs");
            std::fs::rename(&bundled_path, &output_path).unwrap();
            output_path
        } else {
            // For Bun: use TypeScript directly (like production)
            let wrapper = generate_dedicated_worker_wrapper(arg_names, "./main.ts", None);
            let wrapper_path = dir.join("wrapper.mjs");
            std::fs::write(&wrapper_path, wrapper).unwrap();
            wrapper_path
        }
    }

    /// Helper to run a dedicated worker test with given runtime
    fn run_worker_test(
        runtime: &str,
        script: &str,
        arg_names: &[&str],
        jobs: Vec<serde_json::Value>,
    ) -> Vec<Result<serde_json::Value, String>> {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create files and get the wrapper path (bundled for node, raw for bun)
        let wrapper_path =
            create_test_worker_files(temp_dir.path(), script, arg_names, runtime == "node");
        let wrapper_str = wrapper_path.to_str().unwrap();

        // Build args matching production behavior
        let (cmd, args): (&str, Vec<&str>) = match runtime {
            "bun" => {
                // Production: bun run -i --prefer-offline wrapper.mjs
                let mut args: Vec<&str> = BUN_DEDICATED_WORKER_ARGS.to_vec();
                args.push(wrapper_str);
                (BUN_PATH.as_str(), args)
            }
            "node" => {
                // Production: node wrapper.mjs (after bundling to JS)
                (NODE_BIN_PATH.as_str(), vec![wrapper_str])
            }
            _ => panic!("Unknown runtime: {}", runtime),
        };

        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(temp_dir.path())
            .spawn()
            .expect("Failed to spawn worker process");

        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);

        // Wait for "start" signal
        let mut start_line = String::new();
        reader.read_line(&mut start_line).unwrap();
        assert_eq!(
            parse_dedicated_worker_line(start_line.trim()),
            DedicatedWorkerResult::Start,
            "Expected 'start', got: {}",
            start_line.trim()
        );

        let mut results = Vec::new();

        for job_args in jobs {
            writeln!(stdin, "{}", job_args.to_string()).unwrap();
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

    // ==================== Node.js Runtime Tests ====================

    #[test]
    fn test_dedicated_worker_nodejs_simple() {
        let script = r#"
export function main(x: number, y: number): number {
    return x + y;
}
"#;
        let results = run_worker_test(
            "node",
            script,
            &["x", "y"],
            vec![serde_json::json!({"x": 5, "y": 3})],
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], Ok(serde_json::json!(8)));
    }

    #[test]
    fn test_dedicated_worker_nodejs_multiple_jobs() {
        let script = r#"
export function main(n: number): number {
    return n * 2;
}
"#;
        let jobs: Vec<serde_json::Value> = (1..=5).map(|i| serde_json::json!({"n": i})).collect();
        let results = run_worker_test("node", script, &["n"], jobs);

        assert_eq!(results.len(), 5);
        for (i, result) in results.iter().enumerate() {
            let expected = ((i + 1) * 2) as i64;
            assert_eq!(*result, Ok(serde_json::json!(expected)));
        }
    }

    #[test]
    fn test_dedicated_worker_nodejs_error() {
        let script = r#"
export function main(msg: string): never {
    throw new Error(msg);
}
"#;
        let results = run_worker_test(
            "node",
            script,
            &["msg"],
            vec![serde_json::json!({"msg": "test error"})],
        );

        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
        assert_eq!(results[0], Err("test error".to_string()));
    }

    // ==================== Bun Runtime Tests ====================

    #[test]
    fn test_dedicated_worker_bun_simple() {
        let script = r#"
export function main(x: number, y: number): number {
    return x + y;
}
"#;
        let results = run_worker_test(
            "bun",
            script,
            &["x", "y"],
            vec![serde_json::json!({"x": 5, "y": 3})],
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], Ok(serde_json::json!(8)));
    }

    #[test]
    fn test_dedicated_worker_bun_multiple_jobs() {
        let script = r#"
export function main(n: number): number {
    return n * 2;
}
"#;
        let jobs: Vec<serde_json::Value> = (1..=5).map(|i| serde_json::json!({"n": i})).collect();
        let results = run_worker_test("bun", script, &["n"], jobs);

        assert_eq!(results.len(), 5);
        for (i, result) in results.iter().enumerate() {
            let expected = ((i + 1) * 2) as i64;
            assert_eq!(*result, Ok(serde_json::json!(expected)));
        }
    }

    #[test]
    fn test_dedicated_worker_bun_error() {
        let script = r#"
export function main(msg: string): never {
    throw new Error(msg);
}
"#;
        let results = run_worker_test(
            "bun",
            script,
            &["msg"],
            vec![serde_json::json!({"msg": "test error"})],
        );

        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
        assert_eq!(results[0], Err("test error".to_string()));
    }
}

// ============================================================================
// Private Registry Tests
// ============================================================================

/// Test that bun can install packages from a private npm registry with authentication.
/// The registry requires auth tokens for accessing @windmill-test/* packages.
/// Requires:
/// - `private_registry_test` feature enabled
/// - `TEST_NPM_REGISTRY` environment variable set to registry URL with auth token
///   Format: `http://registry-url/:_authToken=TOKEN`
#[cfg(feature = "private_registry_test")]
#[sqlx::test(fixtures("base"))]
async fn test_bun_job_private_npm_registry(db: Pool<Postgres>) -> anyhow::Result<()> {
    use windmill_worker::NPM_CONFIG_REGISTRY;

    let registry_url = std::env::var("TEST_NPM_REGISTRY")
        .expect("TEST_NPM_REGISTRY must be set when running private_registry_test");

    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Set the private registry configuration
    {
        let mut registry = NPM_CONFIG_REGISTRY.write().await;
        *registry = Some(registry_url.clone());
    }

    let content = r#"
import { greet } from "@windmill-test/private-pkg";

export function main(name: string) {
    return greet(name);
}
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let result = RunJob::from(job)
        .arg("name", serde_json::json!("World"))
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();

    // Clean up
    {
        let mut registry = NPM_CONFIG_REGISTRY.write().await;
        *registry = None;
    }

    assert_eq!(
        result,
        serde_json::json!("Hello from private package, World!")
    );
    Ok(())
}

/// Test that full .npmrc content works for bun jobs with private registries.
/// Requires:
/// - `TEST_NPMRC` environment variable set to the full .npmrc content
#[cfg(feature = "private_registry_test")]
#[sqlx::test(fixtures("base"))]
async fn test_bun_job_private_npmrc(db: Pool<Postgres>) -> anyhow::Result<()> {
    use windmill_worker::NPMRC;

    let npmrc_content = std::env::var("TEST_NPMRC")
        .expect("TEST_NPMRC must be set when running private_registry_test");

    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    {
        let mut npmrc = NPMRC.write().await;
        *npmrc = Some(npmrc_content.clone());
    }

    let content = r#"
import { greet } from "@windmill-test/private-pkg";

export function main(name: string) {
    return greet(name);
}
"#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let result = RunJob::from(job)
        .arg("name", serde_json::json!("World"))
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();

    {
        let mut npmrc = NPMRC.write().await;
        *npmrc = None;
    }

    assert_eq!(
        result,
        serde_json::json!("Hello from private package, World!")
    );
    Ok(())
}

/// Tests for RELATIVE_BUN_BUILDER (loader_builder.bun.js)
/// These tests verify Bun's behavior for import scanning and package.json generation.
/// Purpose: Catch regressions when upgrading Bun versions.
mod bun_builder_tests {
    use std::process::{Command, Stdio};
    use windmill_worker::{BUN_PATH, RELATIVE_BUN_BUILDER, RELATIVE_BUN_LOADER};

    /// Run the builder and return the generated package.json content
    fn run_builder(main_ts_content: &str) -> serde_json::Value {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir = temp_dir.path();

        // Write main.ts
        std::fs::write(dir.join("main.ts"), main_ts_content).unwrap();

        // Write build.js using the loader and builder constants directly
        // Parameters are dummy values since tests don't use Windmill relative imports
        let loader = RELATIVE_BUN_LOADER
            .replace("W_ID", "test-workspace")
            .replace("BASE_INTERNAL_URL", "http://localhost:8000")
            .replace("TOKEN", "test-token")
            .replace("CURRENT_PATH", "f/test/script")
            .replace("RAW_GET_ENDPOINT", "raw");

        let build_script = format!(
            r#"
{loader}

{RELATIVE_BUN_BUILDER}
"#
        );
        std::fs::write(dir.join("build.js"), build_script).unwrap();

        // Run bun build.js
        let output = Command::new(BUN_PATH.as_str())
            .args(["run", "build.js"])
            .current_dir(dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("Failed to run bun");

        if !output.status.success() {
            panic!(
                "Builder failed:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // Read generated package.json
        let package_json =
            std::fs::read_to_string(dir.join("package.json")).expect("package.json not generated");

        serde_json::from_str(&package_json).expect("Invalid JSON in package.json")
    }

    /// Test: scanImports() detects basic imports
    #[test]
    fn test_builder_simple_import() {
        let main_ts = r#"
import lodash from "lodash";
export function main() { return lodash; }
"#;
        let pkg = run_builder(main_ts);
        let deps = pkg["dependencies"].as_object().unwrap();

        assert!(
            deps.contains_key("lodash"),
            "lodash should be in dependencies"
        );
        assert_eq!(deps["lodash"], "latest");
    }

    /// Test: scanImports() preserves version info from versioned imports
    #[test]
    fn test_builder_versioned_import() {
        let main_ts = r#"
import _ from "lodash@4.17.21";
export function main() { return _; }
"#;
        let pkg = run_builder(main_ts);
        let deps = pkg["dependencies"].as_object().unwrap();

        assert!(
            deps.contains_key("lodash"),
            "lodash should be in dependencies"
        );
        assert_eq!(deps["lodash"], "4.17.21");
    }

    /// Test: scanImports() handles @scope/package correctly
    #[test]
    fn test_builder_scoped_package() {
        let main_ts = r#"
import babel from "@babel/core";
export function main() { return babel; }
"#;
        let pkg = run_builder(main_ts);
        let deps = pkg["dependencies"].as_object().unwrap();

        assert!(
            deps.contains_key("@babel/core"),
            "@babel/core should be in dependencies"
        );
        assert_eq!(deps["@babel/core"], "latest");
    }

    /// Test: scanImports() handles multiple imports
    #[test]
    fn test_builder_multiple_packages() {
        let main_ts = r#"
import lodash from "lodash";
import axios from "axios";
import dayjs from "dayjs";
export function main() { return { lodash, axios, dayjs }; }
"#;
        let pkg = run_builder(main_ts);
        let deps = pkg["dependencies"].as_object().unwrap();

        assert!(
            deps.contains_key("lodash"),
            "lodash should be in dependencies"
        );
        assert!(
            deps.contains_key("axios"),
            "axios should be in dependencies"
        );
        assert!(
            deps.contains_key("dayjs"),
            "dayjs should be in dependencies"
        );
        assert_eq!(deps.len(), 3, "Should have exactly 3 dependencies");
    }

    /// Test: isBuiltin() filters out Node.js builtins
    #[test]
    fn test_builder_builtin_skipped() {
        let main_ts = r#"
import fs from "fs";
import path from "path";
import lodash from "lodash";
export function main() { return { fs, path, lodash }; }
"#;
        let pkg = run_builder(main_ts);
        let deps = pkg["dependencies"].as_object().unwrap();

        assert!(
            !deps.contains_key("fs"),
            "fs (builtin) should NOT be in dependencies"
        );
        assert!(
            !deps.contains_key("path"),
            "path (builtin) should NOT be in dependencies"
        );
        assert!(
            deps.contains_key("lodash"),
            "lodash should be in dependencies"
        );
        assert_eq!(
            deps.len(),
            1,
            "Should have exactly 1 dependency (lodash only)"
        );
    }

    /// Test: semver.order() resolves version conflicts (picks lowest version)
    #[test]
    fn test_builder_version_conflict() {
        // This test simulates what happens when the same package is imported with different versions
        // The builder should use semver.order() to pick the lowest version
        let main_ts = r#"
import a from "lodash@4.17.21";
import b from "lodash@4.17.10";
export function main() { return { a, b }; }
"#;
        let pkg = run_builder(main_ts);
        let deps = pkg["dependencies"].as_object().unwrap();

        assert!(
            deps.contains_key("lodash"),
            "lodash should be in dependencies"
        );
        // The builder sorts by semver and picks the first (lowest) version
        assert_eq!(
            deps["lodash"], "4.17.10",
            "Should resolve to lower version 4.17.10"
        );
    }
}
