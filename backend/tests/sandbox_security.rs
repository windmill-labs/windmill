use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::jobs::{JobPayload, RawCode};
use windmill_common::scripts::ScriptLang;
use windmill_test_utils::*;

// ============================================================================
// wmill_scopes: Scoped token integration tests
// ============================================================================

/// Test that a Bun job with `// wmill_scopes: jobs:read` can read the API
/// (GET /api/version is always allowed) but cannot create a resource
/// (POST /api/w/.../resources requires resources:write scope).
#[sqlx::test(fixtures("base"))]
async fn test_scoped_token_blocks_unauthorized_write(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = format!(
        r#"
// wmill_scopes: jobs:read
export async function main() {{
    const token = process.env.WM_TOKEN;
    const baseUrl = process.env.BASE_URL;

    // GET /api/version should succeed (not scope-gated)
    const versionRes = await fetch(`${{baseUrl}}/api/version`);
    const versionOk = versionRes.ok;

    // POST to create a variable requires variables:write scope, which we don't have
    const writeRes = await fetch(`${{baseUrl}}/api/w/test-workspace/variables/create`, {{
        method: "POST",
        headers: {{
            "Authorization": `Bearer ${{token}}`,
            "Content-Type": "application/json",
        }},
        body: JSON.stringify({{
            path: "u/test-user/test_var",
            value: "test",
            is_secret: false,
            description: "test",
        }}),
    }});

    return {{
        version_ok: versionOk,
        write_status: writeRes.status,
        write_blocked: writeRes.status === 401,
    }};
}}
"#
    );

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
    assert!(
        completed.success,
        "Job should succeed (it catches the error)"
    );

    let result = completed.json_result().unwrap();
    assert_eq!(
        result["version_ok"],
        json!(true),
        "GET /api/version should be allowed"
    );
    assert_eq!(
        result["write_blocked"],
        json!(true),
        "POST to create variable should be blocked by scoped token (got status {})",
        result["write_status"]
    );
    Ok(())
}

/// Test that a Bun job WITHOUT wmill_scopes can create a variable (full permissions).
#[sqlx::test(fixtures("base"))]
async fn test_unscoped_token_allows_write(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
export async function main() {
    const token = process.env.WM_TOKEN;
    const baseUrl = process.env.BASE_URL;

    // POST to create a variable should succeed with full-access token
    const writeRes = await fetch(`${baseUrl}/api/w/test-workspace/variables/create`, {
        method: "POST",
        headers: {
            "Authorization": `Bearer ${token}`,
            "Content-Type": "application/json",
        },
        body: JSON.stringify({
            path: "u/test-user/unscoped_var",
            value: "test_value",
            is_secret: false,
            description: "test variable",
        }),
    });

    return {
        write_status: writeRes.status,
        write_ok: writeRes.ok,
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

    let completed = run_job_in_new_worker_until_complete(&db, false, job, port).await;
    assert!(completed.success, "Job should succeed");

    let result = completed.json_result().unwrap();
    assert!(
        result["write_ok"].as_bool().unwrap_or(false),
        "Unscoped token should allow variable creation (got status {})",
        result["write_status"]
    );
    Ok(())
}

/// Test that a scoped token with the correct write scope CAN perform the write operation.
#[sqlx::test(fixtures("base"))]
async fn test_scoped_token_allows_matching_scope(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
// wmill_scopes: variables:write,variables:read
export async function main() {
    const token = process.env.WM_TOKEN;
    const baseUrl = process.env.BASE_URL;

    // POST to create a variable should succeed because we have variables:write scope
    const writeRes = await fetch(`${baseUrl}/api/w/test-workspace/variables/create`, {
        method: "POST",
        headers: {
            "Authorization": `Bearer ${token}`,
            "Content-Type": "application/json",
        },
        body: JSON.stringify({
            path: "u/test-user/scoped_allowed_var",
            value: "scoped_value",
            is_secret: false,
            description: "created with scoped token",
        }),
    });

    return {
        write_status: writeRes.status,
        write_ok: writeRes.ok,
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

    let completed = run_job_in_new_worker_until_complete(&db, false, job, port).await;
    assert!(completed.success, "Job should succeed");

    let result = completed.json_result().unwrap();
    assert!(
        result["write_ok"].as_bool().unwrap_or(false),
        "Token with variables:write scope should allow variable creation (got status {})",
        result["write_status"]
    );
    Ok(())
}

// ============================================================================
// allowed_domains: Domain filtering tests
// ============================================================================

/// Test that `// allowed_domains:` without a `// sandbox:` annotation fails the job.
#[sqlx::test(fixtures("base"))]
async fn test_allowed_domains_requires_sandbox(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
// allowed_domains: api.example.com
export function main() {
    return "should not reach here";
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
    assert!(
        !completed.success,
        "Job should fail because allowed_domains requires a sandbox annotation"
    );
    let result = completed.json_result().unwrap();
    let error_msg = result["error"]["message"].as_str().unwrap_or("");
    assert!(
        error_msg.contains("allowed_domains") && error_msg.contains("sandbox"),
        "Error should mention that allowed_domains requires sandbox annotation, got: {}",
        error_msg
    );
    Ok(())
}
