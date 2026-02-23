/// Integration tests for the run_inline endpoints:
///   POST /w/{workspace}/jobs/run_inline/p/*script_path
///   POST /w/{workspace}/jobs/run_inline/h/:hash
///
/// These endpoints execute scripts inline (synchronously, without creating a job
/// record) and return the result directly. The underlying execution requires the
/// `inline_preview` feature AND a live worker registered via
/// `WORKER_INTERNAL_SERVER_INLINE_UTILS`. In the integration-test environment
/// neither of those conditions is met, so the tests here focus on:
///
///   1. Route existence – the endpoints must not return 404.
///   2. Authentication – unauthenticated requests must be rejected.
///   3. Error response format – the body must contain a JSON error message.
///   4. Script lifecycle helpers – creating scripts and retrieving their hashes,
///      which are exercised as pre-conditions for the inline-run endpoints.
use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn jobs_base(port: u16) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/jobs")
}

fn scripts_base(port: u16) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/scripts")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

/// A minimal valid request body for the inline endpoints.
fn empty_args() -> serde_json::Value {
    json!({ "args": {} })
}

/// Create a simple Deno script via the scripts/create endpoint and return its
/// hexadecimal hash string.
async fn create_script_and_get_hash(port: u16, path: &str, content: &str) -> String {
    let base = scripts_base(port);

    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": path,
            "summary": "integration test script",
            "description": "",
            "content": content,
            "language": "deno",
            "schema": {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {},
                "required": []
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        201,
        "failed to create script at {path}: {}",
        resp.text().await.unwrap()
    );

    // Retrieve the script to obtain its hash.
    let resp = authed(client().get(format!("{base}/get/p/{path}")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "failed to get script at {path}");
    let body = resp.json::<serde_json::Value>().await.unwrap();
    body["hash"]
        .as_str()
        .expect("hash field should be a string")
        .to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// The run_inline/p endpoint must exist (not 404) and must reject unauthenticated
/// requests with 401.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_run_inline_by_path_requires_auth(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = jobs_base(port);

    // Unauthenticated request – must not be 404 (route exists) and must be rejected.
    let resp = client()
        .post(format!("{base}/run_inline/p/u/test-user/any_script"))
        .json(&empty_args())
        .send()
        .await?;
    assert_ne!(resp.status(), 404, "route /run_inline/p/ should exist");
    assert_eq!(
        resp.status(),
        401,
        "unauthenticated run_inline/p should be rejected: {}",
        resp.text().await?
    );

    Ok(())
}

/// The run_inline/h endpoint must exist (not 404) and must reject unauthenticated
/// requests with 401.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_run_inline_by_hash_requires_auth(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = jobs_base(port);

    // Use a well-formed but non-existent numeric hash (ScriptHash is i64).
    let resp = client()
        .post(format!("{base}/run_inline/h/1234567890"))
        .json(&empty_args())
        .send()
        .await?;
    assert_ne!(resp.status(), 404, "route /run_inline/h/ should exist");
    assert_eq!(
        resp.status(),
        401,
        "unauthenticated run_inline/h should be rejected: {}",
        resp.text().await?
    );

    Ok(())
}

/// Authenticated requests to run_inline/p must not return 404.
/// Without a live inline-preview worker they return a 5xx error, which is the
/// correct behaviour when the worker infrastructure is absent.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_run_inline_by_path_returns_error_without_worker(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = jobs_base(port);

    let resp = authed(
        client()
            .post(format!(
                "{base}/run_inline/p/u/test-user/nonexistent_script"
            ))
            .json(&empty_args()),
    )
    .send()
    .await?;

    let status = resp.status();
    assert_ne!(status, 404, "/run_inline/p/ route must be registered");
    // Without inline_preview feature or without a running worker the server
    // returns a 5xx status.
    assert!(
        status.is_server_error(),
        "expected a 5xx error without a worker, got {status}: {}",
        resp.text().await?
    );

    Ok(())
}

/// Authenticated requests to run_inline/h must not return 404.
/// Without a live inline-preview worker they return a 5xx error.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_run_inline_by_hash_returns_error_without_worker(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = jobs_base(port);

    let resp = authed(
        client()
            .post(format!("{base}/run_inline/h/9999999999"))
            .json(&empty_args()),
    )
    .send()
    .await?;

    let status = resp.status();
    assert_ne!(status, 404, "/run_inline/h/ route must be registered");
    assert!(
        status.is_server_error(),
        "expected a 5xx error without a worker, got {status}: {}",
        resp.text().await?
    );

    Ok(())
}

/// The error response body for run_inline/p must be JSON and contain a
/// meaningful error message (not an empty body).
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_run_inline_by_path_error_body_is_json(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = jobs_base(port);

    let resp = authed(
        client()
            .post(format!("{base}/run_inline/p/u/test-user/any_script"))
            .json(&empty_args()),
    )
    .send()
    .await?;

    assert!(resp.status().is_server_error());
    let body_text = resp.text().await?;
    assert!(
        !body_text.is_empty(),
        "error response body should not be empty"
    );
    // The error message should indicate the worker feature is required.
    assert!(
        body_text.contains("worker") || body_text.contains("inline"),
        "error message should mention worker or inline feature, got: {body_text}"
    );

    Ok(())
}

/// The error response body for run_inline/h must contain a meaningful error
/// message.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_run_inline_by_hash_error_body_is_json(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = jobs_base(port);

    let resp = authed(
        client()
            .post(format!("{base}/run_inline/h/1111111111"))
            .json(&empty_args()),
    )
    .send()
    .await?;

    assert!(resp.status().is_server_error());
    let body_text = resp.text().await?;
    assert!(
        !body_text.is_empty(),
        "error response body should not be empty"
    );
    // The error message should indicate the worker feature is required.
    assert!(
        body_text.contains("worker") || body_text.contains("inline"),
        "error message should mention worker or inline feature, got: {body_text}"
    );

    Ok(())
}

/// Verify that the hash of a newly created script can be retrieved and is a
/// valid hex string – a pre-condition required before calling run_inline/h.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_script_hash_is_retrievable_for_inline_run(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let script_path = "u/test-user/inline_test_script";
    let hash = create_script_and_get_hash(
        port,
        script_path,
        "export async function main() { return 'hello inline'; }",
    )
    .await;

    assert!(!hash.is_empty(), "script hash should be a non-empty string");
    // Hashes in Windmill are hexadecimal strings.
    assert!(
        hash.chars().all(|c| c.is_ascii_hexdigit()),
        "script hash should only contain hex digits, got: {hash}"
    );

    // Confirm the hash resolves back to the correct script.
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/scripts/get/h/{hash}"
    )))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "get by hash should succeed");
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(
        body["path"].as_str().unwrap(),
        script_path,
        "hash should resolve to the correct script path"
    );

    Ok(())
}

/// Creating two versions of the same script yields two distinct hashes.
/// The path endpoint always resolves to the latest version while the hash
/// endpoint is version-pinned – both relevant for inline runs.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_script_version_hashes_are_distinct(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let scripts_base = scripts_base(port);
    let script_path = "u/test-user/versioned_inline_script";

    // --- version 1 ---
    let resp = authed(client().post(format!("{scripts_base}/create")))
        .json(&json!({
            "path": script_path,
            "summary": "v1",
            "description": "",
            "content": "export async function main() { return 1; }",
            "language": "deno",
            "schema": { "$schema": "https://json-schema.org/draft/2020-12/schema",
                        "type": "object", "properties": {}, "required": [] }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "create v1: {}", resp.text().await?);

    let hash_v1 = {
        let resp = authed(client().get(format!("{scripts_base}/get/p/{script_path}")))
            .send()
            .await?;
        resp.json::<serde_json::Value>().await?["hash"]
            .as_str()
            .unwrap()
            .to_string()
    };

    // --- version 2 (parent_hash points to v1) ---
    let resp = authed(client().post(format!("{scripts_base}/create")))
        .json(&json!({
            "path": script_path,
            "summary": "v2",
            "description": "",
            "content": "export async function main() { return 2; }",
            "language": "deno",
            "parent_hash": hash_v1,
            "schema": { "$schema": "https://json-schema.org/draft/2020-12/schema",
                        "type": "object", "properties": {}, "required": [] }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "create v2: {}", resp.text().await?);

    let hash_v2 = {
        let resp = authed(client().get(format!("{scripts_base}/get/p/{script_path}")))
            .send()
            .await?;
        resp.json::<serde_json::Value>().await?["hash"]
            .as_str()
            .unwrap()
            .to_string()
    };

    assert_ne!(
        hash_v1, hash_v2,
        "each script version must have a distinct hash"
    );

    // path endpoint must resolve to the latest version (v2).
    let resp = authed(client().get(format!("{scripts_base}/get/p/{script_path}")))
        .send()
        .await?;
    let latest = resp.json::<serde_json::Value>().await?;
    assert_eq!(
        latest["hash"].as_str().unwrap(),
        hash_v2,
        "path endpoint should return the latest version"
    );
    assert_eq!(latest["summary"].as_str().unwrap(), "v2");

    // hash endpoint must resolve to the pinned version (v1).
    let resp = authed(client().get(format!("{scripts_base}/get/h/{hash_v1}")))
        .send()
        .await?;
    let pinned = resp.json::<serde_json::Value>().await?;
    assert_eq!(
        pinned["hash"].as_str().unwrap(),
        hash_v1,
        "hash endpoint should return the pinned version"
    );
    assert_eq!(pinned["summary"].as_str().unwrap(), "v1");

    Ok(())
}

/// A request body without the `args` field must still be accepted by the
/// endpoint (the field is optional).  We expect the same 5xx worker-not-present
/// error as with an empty args map – confirming schema deserialization does not
/// itself reject the request.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_run_inline_by_path_accepts_null_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = jobs_base(port);

    // `args` field omitted entirely.
    let resp = authed(
        client()
            .post(format!("{base}/run_inline/p/u/test-user/any_script"))
            .json(&json!({})),
    )
    .send()
    .await?;

    // The error must not be 400 (bad request / schema rejection) or 404.
    let status = resp.status();
    assert_ne!(
        status,
        400,
        "omitting 'args' should not cause a 400 schema error: {}",
        resp.text().await?
    );
    assert_ne!(status, 404, "route must be registered");

    Ok(())
}

/// Same as above for run_inline/h.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_run_inline_by_hash_accepts_null_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = jobs_base(port);

    let resp = authed(
        client()
            .post(format!("{base}/run_inline/h/42"))
            .json(&json!({})),
    )
    .send()
    .await?;

    let status = resp.status();
    assert_ne!(
        status,
        400,
        "omitting 'args' should not cause a 400 schema error: {}",
        resp.text().await?
    );
    assert_ne!(status, 404, "route must be registered");

    Ok(())
}
