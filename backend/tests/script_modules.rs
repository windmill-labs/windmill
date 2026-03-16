use serde_json::json;
use sqlx::postgres::Postgres;
use sqlx::Pool;
use std::collections::HashMap;
use windmill_common::jobs::{JobPayload, RawCode};
use windmill_common::scripts::{ScriptLang, ScriptModule};
use windmill_test_utils::*;

// ============================================================================
// Python: script with inline module via relative import
// ============================================================================

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_script_with_module(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let main_content = r#"
from .helper import greet

def main(name: str):
    return greet(name)
"#
    .to_owned();

    let mut modules = HashMap::new();
    modules.insert(
        "helper.py".to_string(),
        ScriptModule {
            content: "def greet(name):\n    return f\"hello {name}\"\n".to_string(),
            language: ScriptLang::Python3,
            lock: None,
        },
    );

    let job = JobPayload::Code(RawCode {
        content: main_content,
        path: Some("f/test/my_script".to_string()),
        language: ScriptLang::Python3,
        lock: Some("".to_string()), // empty lock prevents pip from resolving module names as packages
        modules: Some(modules),
        ..RawCode::default()
    });

    let result = RunJob::from(job)
        .arg("name", json!("world"))
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, json!("hello world"));
    Ok(())
}

// ============================================================================
// Python: nested module path (subdirectory)
// ============================================================================

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_script_with_nested_module(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let main_content = r#"
from .utils.math import add

def main(a: int, b: int):
    return add(a, b)
"#
    .to_owned();

    let mut modules = HashMap::new();
    modules.insert(
        "utils/__init__.py".to_string(),
        ScriptModule { content: "".to_string(), language: ScriptLang::Python3, lock: None },
    );
    modules.insert(
        "utils/math.py".to_string(),
        ScriptModule {
            content: "def add(a, b):\n    return a + b\n".to_string(),
            language: ScriptLang::Python3,
            lock: None,
        },
    );

    let job = JobPayload::Code(RawCode {
        content: main_content,
        path: Some("f/test/my_script".to_string()),
        language: ScriptLang::Python3,
        lock: Some("".to_string()), // empty lock prevents pip from resolving module names as packages
        modules: Some(modules),
        ..RawCode::default()
    });

    let result = RunJob::from(job)
        .arg("a", json!(3))
        .arg("b", json!(4))
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, json!(7));
    Ok(())
}

// ============================================================================
// Bun: script with inline module via relative import
// ============================================================================

#[sqlx::test(fixtures("base"))]
#[ignore = "Bun module resolution in bundle mode needs loader.bun.js integration — tracked separately"]
async fn test_bun_script_with_module(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let main_content = r#"
import { greet } from "./helper.ts";

export function main(name: string) {
    return greet(name);
}
"#
    .to_owned();

    let mut modules = HashMap::new();
    modules.insert(
        "helper.ts".to_string(),
        ScriptModule {
            content:
                "export function greet(name: string): string {\n    return `hello ${name}`;\n}\n"
                    .to_string(),
            language: ScriptLang::Bun,
            lock: None,
        },
    );

    let job = JobPayload::Code(RawCode {
        content: main_content,
        path: Some("f/test/my_script".to_string()),
        language: ScriptLang::Bun,
        modules: Some(modules),
        ..RawCode::default()
    });

    let result = RunJob::from(job)
        .arg("name", json!("world"))
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, json!("hello world"));
    Ok(())
}
