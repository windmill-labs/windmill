/*
 * NativeTS (deno_core) Integration Tests
 *
 * These tests verify the NativeTS runtime capabilities including:
 * - Fetch operations against local mock server
 * - Concurrent execution stress testing
 * - JavaScript runtime features (async/await, Promises, timers, etc.)
 * - Error handling and edge cases
 */

#![cfg(feature = "deno_core")]

mod common;

use common::{
    completed_job, in_test_worker, initialize_tracing, listen_for_completed_jobs, ApiServer,
    CompletedJob, RunJob,
};
use futures::StreamExt;
use serde_json::json;
use sqlx::{Pool, Postgres};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use windmill_common::jobs::{JobPayload, RawCode};
use windmill_common::scripts::ScriptLang;

// ============================================================================
// Mock HTTP Server for Local Fetch Testing
// ============================================================================

/// A simple mock HTTP server for testing fetch operations without network I/O latency
struct MockServer {
    addr: SocketAddr,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
    handle: tokio::task::JoinHandle<()>,
}

impl MockServer {
    async fn start() -> Self {
        use axum::{
            extract::{Path, Query},
            http::StatusCode,
            routing::{get, post},
            Json, Router,
        };
        use std::collections::HashMap;
        use tokio::net::TcpListener;

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        // Echo endpoint - returns the query params and path
        async fn echo_get(
            Path(path): Path<String>,
            Query(params): Query<HashMap<String, String>>,
        ) -> Json<serde_json::Value> {
            Json(json!({
                "method": "GET",
                "path": path,
                "params": params
            }))
        }

        // Echo POST body
        async fn echo_post(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
            Json(json!({
                "method": "POST",
                "body": body
            }))
        }

        // Delayed response for testing async/timeout
        async fn delayed(Path(ms): Path<u64>) -> Json<serde_json::Value> {
            tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
            Json(json!({ "delayed_ms": ms }))
        }

        // Return specific status code
        async fn status(Path(code): Path<u16>) -> StatusCode {
            StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
        }

        // Large response for testing data handling
        async fn large_response(Path(size): Path<usize>) -> String {
            "x".repeat(size.min(10_000_000))
        }

        // JSON array response
        async fn array(Path(count): Path<usize>) -> Json<Vec<serde_json::Value>> {
            Json((0..count.min(10000)).map(|i| json!({ "index": i })).collect())
        }

        // Headers echo
        async fn headers(
            headers: axum::http::HeaderMap,
        ) -> Json<HashMap<String, String>> {
            let mut map = HashMap::new();
            for (key, value) in headers.iter() {
                if let Ok(v) = value.to_str() {
                    map.insert(key.to_string(), v.to_string());
                }
            }
            Json(map)
        }

        // Counter endpoint for concurrent access testing
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        async fn increment() -> Json<serde_json::Value> {
            let val = COUNTER.fetch_add(1, Ordering::SeqCst);
            Json(json!({ "count": val + 1 }))
        }

        async fn reset_counter() -> Json<serde_json::Value> {
            COUNTER.store(0, Ordering::SeqCst);
            Json(json!({ "reset": true }))
        }

        async fn get_counter() -> Json<serde_json::Value> {
            Json(json!({ "count": COUNTER.load(Ordering::SeqCst) }))
        }

        let app = Router::new()
            .route("/echo/*path", get(echo_get))
            .route("/echo", post(echo_post))
            .route("/delay/:ms", get(delayed))
            .route("/status/:code", get(status))
            .route("/large/:size", get(large_response))
            .route("/array/:count", get(array))
            .route("/headers", get(headers))
            .route("/counter/increment", post(increment))
            .route("/counter/reset", post(reset_counter))
            .route("/counter", get(get_counter));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });

        // Give the server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        MockServer { addr, shutdown_tx, handle }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }

    async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        let _ = self.handle.await;
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn nativets_code(code: &str) -> JobPayload {
    JobPayload::Code(RawCode {
        hash: None,
        content: code.to_string(),
        path: None,
        language: ScriptLang::Nativets,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: Default::default(),
        debouncing_settings: Default::default(),
    })
}

async fn run_nativets(
    db: &Pool<Postgres>,
    code: &str,
    port: u16,
) -> CompletedJob {
    RunJob::from(nativets_code(code))
        .run_until_complete(db, false, port)
        .await
}

async fn run_nativets_with_args(
    db: &Pool<Postgres>,
    code: &str,
    args: serde_json::Map<String, serde_json::Value>,
    port: u16,
) -> CompletedJob {
    let mut job = RunJob::from(nativets_code(code));
    job.args = args;
    job.run_until_complete(db, false, port).await
}

// ============================================================================
// Basic Fetch Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_fetch_get_basic(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let mock = MockServer::start().await;

    let code = format!(r#"
export async function main() {{
    const response = await fetch("{}");
    return await response.json();
}}
"#, mock.url("/echo/test"));

    let result = run_nativets(&db, &code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["method"], "GET");
    assert_eq!(json["path"], "test");

    mock.shutdown().await;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_fetch_post_json(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let mock = MockServer::start().await;

    let code = format!(r#"
export async function main() {{
    const response = await fetch("{}", {{
        method: "POST",
        headers: {{ "Content-Type": "application/json" }},
        body: JSON.stringify({{ name: "test", value: 42 }})
    }});
    return await response.json();
}}
"#, mock.url("/echo"));

    let result = run_nativets(&db, &code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["method"], "POST");
    assert_eq!(json["body"]["name"], "test");
    assert_eq!(json["body"]["value"], 42);

    mock.shutdown().await;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_fetch_with_query_params(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let mock = MockServer::start().await;

    let code = format!(r#"
export async function main() {{
    const url = new URL("{}");
    url.searchParams.set("foo", "bar");
    url.searchParams.set("num", "123");
    const response = await fetch(url);
    return await response.json();
}}
"#, mock.url("/echo/params"));

    let result = run_nativets(&db, &code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["params"]["foo"], "bar");
    assert_eq!(json["params"]["num"], "123");

    mock.shutdown().await;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_fetch_custom_headers(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let mock = MockServer::start().await;

    let code = format!(r#"
export async function main() {{
    const response = await fetch("{}", {{
        headers: {{
            "X-Custom-Header": "custom-value",
            "Authorization": "Bearer test-token"
        }}
    }});
    return await response.json();
}}
"#, mock.url("/headers"));

    let result = run_nativets(&db, &code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["x-custom-header"], "custom-value");
    assert_eq!(json["authorization"], "Bearer test-token");

    mock.shutdown().await;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_fetch_error_status(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let mock = MockServer::start().await;

    let code = format!(r#"
export async function main() {{
    const response = await fetch("{}");
    return {{
        ok: response.ok,
        status: response.status
    }};
}}
"#, mock.url("/status/404"));

    let result = run_nativets(&db, &code, server.addr.port()).await;
    assert!(result.success, "Job should succeed (fetch doesn't throw on 404)");

    let json = result.json_result().unwrap();
    assert_eq!(json["ok"], false);
    assert_eq!(json["status"], 404);

    mock.shutdown().await;
    Ok(())
}

// ============================================================================
// Async/Await and Promise Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_async_await_basic(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let code = r#"
export async function main() {
    const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
    await delay(10);
    return "completed";
}
"#;

    let result = run_nativets(&db, code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");
    assert_eq!(result.json_result().unwrap(), "completed");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_promise_all(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let mock = MockServer::start().await;

    let code = format!(r#"
export async function main() {{
    const urls = [
        "{}/echo/a",
        "{}/echo/b",
        "{}/echo/c"
    ];

    const results = await Promise.all(
        urls.map(url => fetch(url).then(r => r.json()))
    );

    return results.map(r => r.path);
}}
"#, mock.url(""), mock.url(""), mock.url(""));

    let result = run_nativets(&db, &code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json, json!(["a", "b", "c"]));

    mock.shutdown().await;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_promise_race(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let mock = MockServer::start().await;

    let code = format!(r#"
export async function main() {{
    const fast = fetch("{}").then(r => r.json());
    const slow = fetch("{}").then(r => r.json());

    const result = await Promise.race([fast, slow]);
    return result;
}}
"#, mock.url("/delay/10"), mock.url("/delay/100"));

    let result = run_nativets(&db, &code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["delayed_ms"], 10);

    mock.shutdown().await;
    Ok(())
}

// ============================================================================
// Data Processing Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_json_processing(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let mock = MockServer::start().await;

    let code = format!(r#"
export async function main() {{
    const response = await fetch("{}");
    const data = await response.json();

    // Process the array
    const processed = data
        .filter((item: any) => item.index % 2 === 0)
        .map((item: any) => item.index * 2)
        .slice(0, 5);

    return processed;
}}
"#, mock.url("/array/20"));

    let result = run_nativets(&db, &code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json, json!([0, 4, 8, 12, 16]));

    mock.shutdown().await;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_text_response(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let mock = MockServer::start().await;

    let code = format!(r#"
export async function main() {{
    const response = await fetch("{}");
    const text = await response.text();
    return {{ length: text.length, sample: text.substring(0, 10) }};
}}
"#, mock.url("/large/100"));

    let result = run_nativets(&db, &code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["length"], 100);
    assert_eq!(json["sample"], "xxxxxxxxxx");

    mock.shutdown().await;
    Ok(())
}

// ============================================================================
// JavaScript Feature Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_spread_destructuring(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let code = r#"
export async function main() {
    const arr1 = [1, 2, 3];
    const arr2 = [4, 5, 6];
    const combined = [...arr1, ...arr2];

    const obj1 = { a: 1, b: 2 };
    const obj2 = { c: 3, ...obj1 };

    const [first, ...rest] = combined;
    const { a, ...remaining } = obj2;

    return { combined, obj2, first, rest, a, remaining };
}
"#;

    let result = run_nativets(&db, code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["combined"], json!([1, 2, 3, 4, 5, 6]));
    assert_eq!(json["obj2"], json!({"a": 1, "b": 2, "c": 3}));
    assert_eq!(json["first"], 1);
    assert_eq!(json["rest"], json!([2, 3, 4, 5, 6]));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_class_and_inheritance(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let code = r#"
class Animal {
    name: string;
    constructor(name: string) {
        this.name = name;
    }
    speak(): string {
        return `${this.name} makes a sound`;
    }
}

class Dog extends Animal {
    breed: string;
    constructor(name: string, breed: string) {
        super(name);
        this.breed = breed;
    }
    speak(): string {
        return `${this.name} barks`;
    }
}

export async function main() {
    const dog = new Dog("Rex", "German Shepherd");
    return {
        name: dog.name,
        breed: dog.breed,
        speak: dog.speak()
    };
}
"#;

    let result = run_nativets(&db, code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["name"], "Rex");
    assert_eq!(json["breed"], "German Shepherd");
    assert_eq!(json["speak"], "Rex barks");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_generators_and_iterators(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let code = r#"
function* fibonacci(n: number) {
    let [a, b] = [0, 1];
    for (let i = 0; i < n; i++) {
        yield a;
        [a, b] = [b, a + b];
    }
}

export async function main() {
    const fibs = [...fibonacci(10)];
    return fibs;
}
"#;

    let result = run_nativets(&db, code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json, json!([0, 1, 1, 2, 3, 5, 8, 13, 21, 34]));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_map_set_operations(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let code = r#"
export async function main() {
    // Map operations
    const map = new Map<string, number>();
    map.set("a", 1);
    map.set("b", 2);
    map.set("c", 3);

    const mapEntries = [...map.entries()];
    const mapKeys = [...map.keys()];
    const mapValues = [...map.values()];

    // Set operations
    const set = new Set([1, 2, 2, 3, 3, 3]);
    const setValues = [...set];

    return {
        mapEntries,
        mapKeys,
        mapValues,
        setValues,
        mapSize: map.size,
        setSize: set.size
    };
}
"#;

    let result = run_nativets(&db, code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["mapKeys"], json!(["a", "b", "c"]));
    assert_eq!(json["mapValues"], json!([1, 2, 3]));
    assert_eq!(json["setValues"], json!([1, 2, 3]));
    assert_eq!(json["mapSize"], 3);
    assert_eq!(json["setSize"], 3);

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_regex_operations(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let code = r#"
export async function main() {
    const text = "The quick brown fox jumps over 3 lazy dogs in 2023";

    // Match all words
    const words = text.match(/\b[a-z]+\b/gi) || [];

    // Match all numbers
    const numbers = text.match(/\d+/g)?.map(Number) || [];

    // Replace
    const replaced = text.replace(/fox|dogs/gi, "***");

    // Test
    const hasNumbers = /\d+/.test(text);

    return {
        wordCount: words.length,
        numbers,
        replaced,
        hasNumbers
    };
}
"#;

    let result = run_nativets(&db, code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["numbers"], json!([3, 2023]));
    assert_eq!(json["hasNumbers"], true);
    assert!(json["replaced"].as_str().unwrap().contains("***"));

    Ok(())
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_try_catch(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let code = r#"
export async function main() {
    let caught = null;
    try {
        throw new Error("Test error");
    } catch (e) {
        caught = (e as Error).message;
    }
    return { caught };
}
"#;

    let result = run_nativets(&db, code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["caught"], "Test error");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_fetch_network_error(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    // Try to fetch from a non-existent server
    let code = r#"
export async function main() {
    try {
        await fetch("http://127.0.0.1:59999/nonexistent");
        return { error: null };
    } catch (e) {
        return { error: "fetch_failed" };
    }
}
"#;

    let result = run_nativets(&db, code, server.addr.port()).await;
    assert!(result.success, "Job should succeed (error was caught)");

    let json = result.json_result().unwrap();
    assert_eq!(json["error"], "fetch_failed");

    Ok(())
}

// ============================================================================
// Stress Tests - Concurrent Execution
// ============================================================================

/// Push multiple jobs and process them with a single worker.
/// This tests concurrent V8 isolate execution more realistically.
async fn push_jobs_and_wait_for_completion(
    db: &Pool<Postgres>,
    payloads: Vec<JobPayload>,
    port: u16,
) -> Vec<CompletedJob> {
    use windmill_common::worker::Connection;

    // Push all jobs first
    let mut job_ids = Vec::with_capacity(payloads.len());
    for payload in payloads {
        let uuid = RunJob::from(payload).push(db).await;
        job_ids.push(uuid);
    }

    // Listen for completed jobs
    let mut listener = listen_for_completed_jobs(db).await;

    // Start the worker and wait for all jobs to complete
    let job_count = job_ids.len();
    let job_ids_clone = job_ids.clone();

    in_test_worker(
        Connection::Sql(db.clone()),
        async move {
            let mut completed_count = 0;
            while completed_count < job_count {
                if let Some(uuid) = listener.next().await {
                    if job_ids_clone.contains(&uuid) {
                        completed_count += 1;
                    }
                }
            }
        },
        port,
    )
    .await;

    // Fetch all completed jobs
    let mut results = Vec::with_capacity(job_ids.len());
    for uuid in job_ids {
        results.push(completed_job(uuid, db).await);
    }
    results
}

#[sqlx::test(fixtures("base"))]
async fn test_stress_concurrent_fetch(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let mock = MockServer::start().await;
    let port = server.addr.port();

    // Reset counter
    reqwest::Client::new()
        .post(mock.url("/counter/reset"))
        .send()
        .await?;

    const NUM_CONCURRENT: usize = 20;

    // Create all job payloads
    let mut payloads = Vec::with_capacity(NUM_CONCURRENT);
    for i in 0..NUM_CONCURRENT {
        let code = format!(r#"
export async function main() {{
    const response = await fetch("{}", {{ method: "POST" }});
    const data = await response.json();
    return {{ ...data, jobIndex: {} }};
}}
"#, mock.url("/counter/increment"), i);
        payloads.push(nativets_code(&code));
    }

    // Push all jobs and wait for completion
    let results = push_jobs_and_wait_for_completion(&db, payloads, port).await;

    // Verify all succeeded
    for (i, result) in results.iter().enumerate() {
        assert!(result.success, "Job {} should succeed", i);
    }

    // Verify counter was incremented NUM_CONCURRENT times
    let counter: serde_json::Value = reqwest::Client::new()
        .get(mock.url("/counter"))
        .send()
        .await?
        .json()
        .await?;

    assert_eq!(counter["count"], NUM_CONCURRENT);

    mock.shutdown().await;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_stress_cpu_bound(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    const NUM_CONCURRENT: usize = 10;

    // Create all job payloads with CPU-bound computation
    let mut payloads = Vec::with_capacity(NUM_CONCURRENT);
    for i in 0..NUM_CONCURRENT {
        let code = format!(r#"
export async function main() {{
    // Compute fibonacci iteratively
    function fib(n: number): number {{
        let [a, b] = [0, 1];
        for (let i = 0; i < n; i++) {{
            [a, b] = [b, a + b];
        }}
        return a;
    }}

    // Do some work
    const results = [];
    for (let i = 0; i < 100; i++) {{
        results.push(fib(20));
    }}

    return {{ computed: results.length, sample: results[0], jobIndex: {} }};
}}
"#, i);
        payloads.push(nativets_code(&code));
    }

    // Push all jobs and wait for completion
    let results = push_jobs_and_wait_for_completion(&db, payloads, port).await;

    for (i, result) in results.iter().enumerate() {
        assert!(result.success, "Job {} should succeed", i);
        let json = result.json_result().unwrap();
        assert_eq!(json["computed"], 100);
        assert_eq!(json["sample"], 6765); // fib(20)
    }

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_stress_mixed_workload(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let mock = MockServer::start().await;
    let port = server.addr.port();

    const NUM_CONCURRENT: usize = 15;

    // Create all job payloads with mixed workload
    let mut payloads = Vec::with_capacity(NUM_CONCURRENT);
    for i in 0..NUM_CONCURRENT {
        let code = format!(r#"
export async function main() {{
    // Fetch data
    const response = await fetch("{}/array/50");
    const data = await response.json();

    // Process data
    const processed = data
        .map((item: any) => item.index * 2)
        .filter((n: number) => n % 3 === 0)
        .reduce((a: number, b: number) => a + b, 0);

    // More computation
    const primes = [];
    for (let n = 2; n < 100; n++) {{
        let isPrime = true;
        for (let d = 2; d * d <= n; d++) {{
            if (n % d === 0) {{ isPrime = false; break; }}
        }}
        if (isPrime) primes.push(n);
    }}

    return {{
        processedSum: processed,
        primeCount: primes.length,
        jobIndex: {}
    }};
}}
"#, mock.url(""), i);
        payloads.push(nativets_code(&code));
    }

    // Push all jobs and wait for completion
    let results = push_jobs_and_wait_for_completion(&db, payloads, port).await;

    for result in &results {
        assert!(result.success, "Job should succeed");
        let json = result.json_result().unwrap();
        assert_eq!(json["primeCount"], 25); // Primes under 100
    }

    mock.shutdown().await;
    Ok(())
}

// ============================================================================
// Timer and Async Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_settimeout(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let code = r#"
export async function main() {
    const start = Date.now();
    await new Promise(resolve => setTimeout(resolve, 50));
    const elapsed = Date.now() - start;
    return { elapsed_gte_50: elapsed >= 50 };
}
"#;

    let result = run_nativets(&db, code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["elapsed_gte_50"], true);

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_setinterval_with_clear(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let code = r#"
export async function main() {
    let count = 0;

    await new Promise<void>((resolve) => {
        const interval = setInterval(() => {
            count++;
            if (count >= 3) {
                clearInterval(interval);
                resolve();
            }
        }, 10);
    });

    return { count };
}
"#;

    let result = run_nativets(&db, code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["count"], 3);

    Ok(())
}

// ============================================================================
// Base64 and Encoding Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_base64_encoding(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let code = r#"
export async function main() {
    const original = "Hello, World!";
    const encoded = btoa(original);
    const decoded = atob(encoded);

    return {
        original,
        encoded,
        decoded,
        roundtrip: original === decoded
    };
}
"#;

    let result = run_nativets(&db, code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["original"], "Hello, World!");
    assert_eq!(json["encoded"], "SGVsbG8sIFdvcmxkIQ==");
    assert_eq!(json["decoded"], "Hello, World!");
    assert_eq!(json["roundtrip"], true);

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_text_encoder_decoder(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let code = r#"
export async function main() {
    const encoder = new TextEncoder();
    const decoder = new TextDecoder();

    const original = "Hello, 世界!";
    const encoded = encoder.encode(original);
    const decoded = decoder.decode(encoded);

    return {
        original,
        encodedLength: encoded.length,
        decoded,
        roundtrip: original === decoded
    };
}
"#;

    let result = run_nativets(&db, code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["original"], "Hello, 世界!");
    assert_eq!(json["decoded"], "Hello, 世界!");
    assert_eq!(json["roundtrip"], true);

    Ok(())
}

// ============================================================================
// URL API Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_url_api(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let code = r#"
export async function main() {
    const url = new URL("https://example.com:8080/path/to/resource?foo=bar&baz=qux#section");

    return {
        href: url.href,
        protocol: url.protocol,
        host: url.host,
        hostname: url.hostname,
        port: url.port,
        pathname: url.pathname,
        search: url.search,
        hash: url.hash,
        searchParams: Object.fromEntries(url.searchParams)
    };
}
"#;

    let result = run_nativets(&db, code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["protocol"], "https:");
    assert_eq!(json["hostname"], "example.com");
    assert_eq!(json["port"], "8080");
    assert_eq!(json["pathname"], "/path/to/resource");
    assert_eq!(json["searchParams"]["foo"], "bar");
    assert_eq!(json["searchParams"]["baz"], "qux");

    Ok(())
}

// ============================================================================
// AbortController Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_abort_controller(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let mock = MockServer::start().await;

    let code = format!(r#"
export async function main() {{
    const controller = new AbortController();
    const signal = controller.signal;

    // Abort after 10ms
    setTimeout(() => controller.abort(), 10);

    try {{
        // This should take 1000ms but will be aborted
        await fetch("{}", {{ signal }});
        return {{ aborted: false }};
    }} catch (e) {{
        return {{ aborted: true, reason: (e as Error).name }};
    }}
}}
"#, mock.url("/delay/1000"));

    let result = run_nativets(&db, &code, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["aborted"], true);

    mock.shutdown().await;
    Ok(())
}

// ============================================================================
// Input Arguments Tests
// ============================================================================

#[sqlx::test(fixtures("base"))]
async fn test_function_arguments(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;

    let code = r#"
export async function main(name: string, count: number, items: string[]) {
    return {
        greeting: `Hello, ${name}!`,
        doubled: count * 2,
        itemCount: items.length,
        firstItem: items[0]
    };
}
"#;

    let mut args = serde_json::Map::new();
    args.insert("name".to_string(), json!("World"));
    args.insert("count".to_string(), json!(21));
    args.insert("items".to_string(), json!(["apple", "banana", "cherry"]));

    let result = run_nativets_with_args(&db, code, args, server.addr.port()).await;
    assert!(result.success, "Job should succeed");

    let json = result.json_result().unwrap();
    assert_eq!(json["greeting"], "Hello, World!");
    assert_eq!(json["doubled"], 42);
    assert_eq!(json["itemCount"], 3);
    assert_eq!(json["firstItem"], "apple");

    Ok(())
}

// ============================================================================
// Pool Status and Benchmark Tests
// ============================================================================

/// Test that reports pool status - useful for verifying pool is enabled/disabled
#[sqlx::test(fixtures("base"))]
async fn test_pool_status(_db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let (pool_active, jobs_processed) = windmill_nativets::get_pool_stats();
    let pool_enabled = windmill_nativets::is_isolate_pool_enabled();

    tracing::info!(
        "NativeTS Isolate Pool - enabled: {}, active: {}, jobs_processed: {}",
        pool_enabled,
        pool_active,
        jobs_processed
    );

    // If pool is enabled in env, it should be active after init
    if pool_enabled {
        // Initialize the pool (idempotent)
        windmill_nativets::init_isolate_pool();
        let (pool_active, _) = windmill_nativets::get_pool_stats();
        assert!(pool_active, "Pool should be active when enabled");
    }

    Ok(())
}

/// Benchmark test - run multiple sequential jobs to measure cold start overhead
/// Run with NATIVETS_ISOLATE_POOL=true and without to compare
#[sqlx::test(fixtures("base"))]
async fn test_benchmark_sequential_jobs(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Initialize pool if enabled
    windmill_nativets::init_isolate_pool();

    let (pool_active, _) = windmill_nativets::get_pool_stats();
    tracing::info!("Running benchmark with pool_active: {}", pool_active);

    const NUM_JOBS: usize = 5;
    let mut durations = Vec::with_capacity(NUM_JOBS);

    for i in 0..NUM_JOBS {
        let code = format!(r#"
export async function main() {{
    // Simple computation
    let sum = 0;
    for (let j = 0; j < 1000; j++) {{
        sum += j;
    }}
    return {{ iteration: {}, sum }};
}}
"#, i);

        let start = std::time::Instant::now();
        let result = run_nativets(&db, &code, port).await;
        let elapsed = start.elapsed();

        assert!(result.success, "Job {} should succeed", i);
        durations.push(elapsed);
    }

    // Log timing information
    let avg = durations.iter().map(|d| d.as_millis()).sum::<u128>() / NUM_JOBS as u128;
    let first = durations[0].as_millis();
    let rest_avg = durations[1..].iter().map(|d| d.as_millis()).sum::<u128>() / (NUM_JOBS - 1) as u128;

    tracing::info!(
        "Benchmark results (pool_active={}): first={}ms, rest_avg={}ms, overall_avg={}ms",
        pool_active, first, rest_avg, avg
    );

    Ok(())
}

/// Stress test with high concurrency - 20 parallel jobs
/// This tests the pool under load vs spawn_blocking
#[sqlx::test(fixtures("base"))]
async fn test_stress_high_concurrency(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let mock = MockServer::start().await;
    let port = server.addr.port();

    // Initialize pool if enabled
    windmill_nativets::init_isolate_pool();

    let (pool_active, initial_jobs) = windmill_nativets::get_pool_stats();
    tracing::info!(
        "Starting high concurrency stress test with pool_active: {}, initial_jobs: {}",
        pool_active, initial_jobs
    );

    const NUM_CONCURRENT: usize = 20;

    let start = std::time::Instant::now();

    // Create all job payloads
    let mut payloads = Vec::with_capacity(NUM_CONCURRENT);
    for i in 0..NUM_CONCURRENT {
        let code = format!(r#"
export async function main() {{
    // Mixed workload: fetch + computation
    const response = await fetch("{}/array/10");
    const data = await response.json();

    // CPU work
    let sum = 0;
    for (let j = 0; j < 10000; j++) {{
        sum += j;
    }}

    return {{
        fetchedCount: data.length,
        computedSum: sum,
        jobIndex: {}
    }};
}}
"#, mock.url(""), i);
        payloads.push(nativets_code(&code));
    }

    // Push all jobs and wait for completion
    let results = push_jobs_and_wait_for_completion(&db, payloads, port).await;
    let total_elapsed = start.elapsed();

    // Verify all succeeded
    let success_count = results.iter().filter(|r| r.success).count();
    assert_eq!(success_count, NUM_CONCURRENT, "All jobs should succeed");

    let (_, final_jobs) = windmill_nativets::get_pool_stats();

    tracing::info!(
        "High concurrency test complete: {} jobs in {:?} (pool_active={}, pool_jobs={})",
        NUM_CONCURRENT,
        total_elapsed,
        pool_active,
        final_jobs - initial_jobs
    );

    mock.shutdown().await;
    Ok(())
}
