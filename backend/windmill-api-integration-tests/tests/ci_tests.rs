use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn script_url(port: u16, endpoint: &str, path: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/scripts/{endpoint}/{path}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn new_script(path: &str, content: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": "",
        "description": "",
        "content": content,
        "language": "deno",
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        }
    })
}

/// Test 1: Creating a script with a CI test annotation inserts rows into ci_test_reference,
/// and deploying a new version without the annotation removes them.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_ci_test_annotation_creates_and_removes_references(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/scripts");

    // Create target script (no annotation)
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            "u/test-user/target_script",
            "export async function main() { return 42; }",
        ))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "create target: {}", resp.text().await?);

    // Create test script with CI annotation
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            "u/test-user/ci_test_script",
            "// test: script/u/test-user/target_script\nexport async function main() { return 'test passed'; }",
        ))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "create test: {}", resp.text().await?);

    // Verify ci_test_reference row exists
    let refs = sqlx::query!(
        "SELECT test_script_path, tested_item_path, tested_item_kind \
         FROM ci_test_reference WHERE workspace_id = 'test-workspace'"
    )
    .fetch_all(&db)
    .await?;

    assert_eq!(refs.len(), 1, "expected 1 ci_test_reference row");
    assert_eq!(refs[0].test_script_path, "u/test-user/ci_test_script");
    assert_eq!(refs[0].tested_item_path, "u/test-user/target_script");
    assert_eq!(refs[0].tested_item_kind, "script");

    // Get the hash of the test script (needed as parent_hash for the update)
    let resp = authed(client().get(script_url(port, "get/p", "u/test-user/ci_test_script")))
        .send()
        .await?;
    let body = resp.json::<serde_json::Value>().await?;
    let hash = body["hash"].as_str().unwrap().to_string();

    // Create new version of test script WITHOUT annotation
    let mut updated = new_script(
        "u/test-user/ci_test_script",
        "export async function main() { return 'no longer a test'; }",
    );
    updated["parent_hash"] = json!(hash);
    let resp = authed(client().post(format!("{base}/create")))
        .json(&updated)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        201,
        "remove annotation: {}",
        resp.text().await?
    );

    // Verify ci_test_reference row was deleted
    let refs = sqlx::query!(
        "SELECT test_script_path FROM ci_test_reference \
         WHERE workspace_id = 'test-workspace'"
    )
    .fetch_all(&db)
    .await?;
    assert_eq!(
        refs.len(),
        0,
        "ci_test_reference should be empty after removing annotation"
    );

    Ok(())
}

/// Test 2: The CI test results API returns test references (with null job info when
/// no CI test job has run yet), and the batch endpoint aggregates correctly.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_ci_test_results_api(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/scripts");

    // Create target script
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            "u/test-user/target_for_results",
            "export async function main() { return 1; }",
        ))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "create target: {}", resp.text().await?);

    // Create test script with annotation
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            "u/test-user/test_for_results",
            "// test: script/u/test-user/target_for_results\nexport async function main() { return true; }",
        ))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "create test: {}", resp.text().await?);

    // --- Single item endpoint ---
    let resp = authed(client().get(script_url(
        port,
        "ci_test_results/script",
        "u/test-user/target_for_results",
    )))
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    let results = resp.json::<Vec<serde_json::Value>>().await?;

    assert_eq!(results.len(), 1, "expected 1 CI test result");
    assert_eq!(
        results[0]["test_script_path"],
        "u/test-user/test_for_results"
    );
    // No CI test job has been triggered yet
    assert!(results[0]["job_id"].is_null());
    assert!(results[0]["status"].is_null());

    // --- Batch endpoint ---
    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/scripts/ci_test_results_batch"
    )))
    .json(&json!({
        "items": [
            {"path": "u/test-user/target_for_results", "kind": "script"},
            {"path": "u/test-user/nonexistent", "kind": "script"}
        ]
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    let batch = resp
        .json::<serde_json::Map<String, serde_json::Value>>()
        .await?;

    // Target should have one test
    let key = "script:u/test-user/target_for_results";
    assert!(batch.contains_key(key), "missing key {key}");
    let target_results = batch[key].as_array().unwrap();
    assert_eq!(target_results.len(), 1);
    assert_eq!(
        target_results[0]["test_script_path"],
        "u/test-user/test_for_results"
    );

    // Nonexistent should have empty array
    let key = "script:u/test-user/nonexistent";
    assert!(batch.contains_key(key), "missing key {key}");
    assert_eq!(batch[key].as_array().unwrap().len(), 0);

    Ok(())
}

/// Test 3: A wildcard annotation (`u/test-user/*`) matches multiple deployed targets
/// for both the stored reference and the ci_test_results endpoint, and respects glob
/// semantics (`*` stays within a single segment).
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_ci_test_wildcard_annotation(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/scripts");

    // Create test script with wildcard annotation.
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            "u/test-user/wildcard_test",
            "// test: script/u/test-user/*\nexport async function main() { return true; }",
        ))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        201,
        "create wildcard test: {}",
        resp.text().await?
    );

    // Verify the row was stored with has_wildcard = true.
    let stored = sqlx::query!(
        "SELECT tested_item_path, has_wildcard as \"has_wildcard!\" \
         FROM ci_test_reference WHERE workspace_id = 'test-workspace'"
    )
    .fetch_all(&db)
    .await?;
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].tested_item_path, "u/test-user/*");
    assert!(stored[0].has_wildcard, "has_wildcard flag should be set");

    // Create two sibling scripts that the pattern should match.
    for name in ["alpha", "beta"] {
        let resp = authed(client().post(format!("{base}/create")))
            .json(&new_script(
                &format!("u/test-user/{name}"),
                "export async function main() { return 1; }",
            ))
            .send()
            .await?;
        assert_eq!(resp.status(), 201, "create {name}: {}", resp.text().await?);
    }

    // Query the results endpoint for each — both should resolve to the same test.
    for name in ["alpha", "beta"] {
        let resp = authed(client().get(script_url(
            port,
            "ci_test_results/script",
            &format!("u/test-user/{name}"),
        )))
        .send()
        .await?;
        assert_eq!(resp.status(), 200);
        let results = resp.json::<Vec<serde_json::Value>>().await?;
        assert_eq!(
            results.len(),
            1,
            "expected 1 CI test result for {name} via wildcard"
        );
        assert_eq!(results[0]["test_script_path"], "u/test-user/wildcard_test");
    }

    // A path with an extra `/` segment must NOT match `u/test-user/*` (single segment).
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            "u/test-user/sub/deep",
            "export async function main() { return 2; }",
        ))
        .send()
        .await?;
    assert_eq!(resp.status(), 201);

    let resp = authed(client().get(script_url(
        port,
        "ci_test_results/script",
        "u/test-user/sub/deep",
    )))
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    let results = resp.json::<Vec<serde_json::Value>>().await?;
    assert_eq!(
        results.len(),
        0,
        "single-star should not cross path segments"
    );

    Ok(())
}

/// Test 4: `**` wildcard crosses path segments end-to-end.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_ci_test_double_star_wildcard(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/scripts");

    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            "u/test-user/deep_test",
            "// test: script/u/test-user/**\nexport async function main() { return true; }",
        ))
        .send()
        .await?;
    assert_eq!(resp.status(), 201);

    for path in ["u/test-user/top", "u/test-user/nested/deep/leaf"] {
        let resp = authed(client().post(format!("{base}/create")))
            .json(&new_script(
                path,
                "export async function main() { return 1; }",
            ))
            .send()
            .await?;
        assert_eq!(resp.status(), 201, "create {path}: {}", resp.text().await?);

        let resp = authed(client().get(script_url(port, "ci_test_results/script", path)))
            .send()
            .await?;
        let results = resp.json::<Vec<serde_json::Value>>().await?;
        assert_eq!(results.len(), 1, "** should match {path} across any depth");
        assert_eq!(results[0]["test_script_path"], "u/test-user/deep_test");
    }

    Ok(())
}
