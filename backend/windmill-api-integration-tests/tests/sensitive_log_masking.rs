//! Integration tests for sensitive log masking.
//!
//! These tests run real script previews (bun) that fetch secret variables
//! and console.log their values, then verify the job logs contain masked output.
//!
//! Run with:
//!   cargo test -p windmill-api-integration-tests --test sensitive_log_masking -- --nocapture
//!
//! Requires: bun runtime, live database (migrations applied by sqlx::test).

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

/// Helper: create a secret variable via the API (ensures proper encryption).
async fn create_secret_variable(port: u16, path: &str, value: &str) {
    let base = format!("http://localhost:{port}/api/w/test-workspace/variables");
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": path,
            "value": value,
            "is_secret": true,
            "description": "test secret for log masking"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        201,
        "failed to create secret variable: {}",
        resp.text().await.unwrap_or_default()
    );
}

/// Helper: fetch job logs from the job_logs table.
async fn get_job_logs(db: &Pool<Postgres>, job_id: uuid::Uuid) -> Option<String> {
    sqlx::query_scalar!(
        r#"SELECT logs as "logs!" FROM job_logs WHERE job_id = $1"#,
        job_id,
    )
    .fetch_optional(db)
    .await
    .unwrap()
}

/// Run a bun preview script that fetches a secret variable and console.logs it.
/// Assert that the secret value is masked in the job logs.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_secret_variable_masked_in_logs(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let secret_value = "super_secret_password_12345";
    let var_path = "u/test-user/mask_test_secret";

    // 1. Create a secret variable via API (properly encrypted)
    create_secret_variable(port, var_path, secret_value).await;

    // 2. Run a bun preview script that fetches the secret and console.logs it
    use futures::StreamExt;
    use windmill_common::jobs::RawCode;
    use windmill_common::scripts::ScriptLang;

    let mut completed = listen_for_completed_jobs(&db).await;
    let db2 = db.clone();
    in_test_worker(
        db.clone(),
        async move {
            let bun_code = format!(
                r#"import * as wmill from "windmill-client";
export async function main() {{
  const secret = await wmill.getVariable("{var_path}");
  console.log("The secret value is: " + secret);
  console.log("done logging");
  return "ok";
}}"#
            );

            let job_id = RunJob::from(windmill_common::jobs::JobPayload::Code(RawCode {
                hash: None,
                content: bun_code,
                path: None,
                language: ScriptLang::Bun,
                lock: None,
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: None,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default().into(),
                debouncing_settings:
                    windmill_common::runnable_settings::DebouncingSettings::default(),
                modules: None,
            }))
            .push(&db2)
            .await;

            completed.next().await;
            let cjob = completed_job(job_id, &db2).await;

            // 3. Verify the job succeeded
            assert!(cjob.success, "job should have succeeded");

            // 4. Check the logs: secret value must NOT appear, masked placeholder MUST appear
            let logs = get_job_logs(&db2, job_id)
                .await
                .expect("job_logs entry should exist");

            assert!(
                !logs.contains(secret_value),
                "FAIL: secret value '{}' was found in job logs!\nLogs:\n{}",
                secret_value,
                logs
            );
            assert!(
                logs.contains("*******"),
                "FAIL: masked placeholder '******* ' not found in logs\nLogs:\n{}",
                logs
            );
            assert!(
                logs.contains("The secret value is: *******"),
                "FAIL: expected 'The secret value is: *******' in logs\nLogs:\n{}",
                logs
            );
        },
        port,
    )
    .await;

    Ok(())
}

/// Run a bun preview with a non-secret variable and verify it is NOT masked.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_non_secret_variable_not_masked(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let plain_value = "plain_visible_value_12345";
    let var_path = "u/test-user/mask_test_plain";

    // Create a NON-secret variable
    let base = format!("http://localhost:{port}/api/w/test-workspace/variables");
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": var_path,
            "value": plain_value,
            "is_secret": false,
            "description": "test plain variable"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    use futures::StreamExt;
    use windmill_common::jobs::RawCode;
    use windmill_common::scripts::ScriptLang;

    let mut completed = listen_for_completed_jobs(&db).await;
    let db2 = db.clone();
    in_test_worker(
        db.clone(),
        async move {
            let bun_code = format!(
                r#"import * as wmill from "windmill-client";
export async function main() {{
  const val = await wmill.getVariable("{var_path}");
  console.log("The plain value is: " + val);
  return "ok";
}}"#
            );

            let job_id = RunJob::from(windmill_common::jobs::JobPayload::Code(RawCode {
                hash: None,
                content: bun_code,
                path: None,
                language: ScriptLang::Bun,
                lock: None,
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: None,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default().into(),
                debouncing_settings:
                    windmill_common::runnable_settings::DebouncingSettings::default(),
                modules: None,
            }))
            .push(&db2)
            .await;

            completed.next().await;
            let cjob = completed_job(job_id, &db2).await;
            assert!(cjob.success, "job should have succeeded");

            let logs = get_job_logs(&db2, job_id)
                .await
                .expect("job_logs entry should exist");

            // Plain variable value SHOULD appear in logs (not masked)
            assert!(
                logs.contains(plain_value),
                "plain value '{}' should appear in logs unmasked\nLogs:\n{}",
                plain_value,
                logs
            );
        },
        port,
    )
    .await;

    Ok(())
}

/// Run a bun preview that logs a secret multiple times — all occurrences must be masked.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_secret_masked_multiple_occurrences(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let secret_value = "repeated_secret_token_xyz";
    let var_path = "u/test-user/mask_test_repeated";
    create_secret_variable(port, var_path, secret_value).await;

    use futures::StreamExt;
    use windmill_common::jobs::RawCode;
    use windmill_common::scripts::ScriptLang;

    let mut completed = listen_for_completed_jobs(&db).await;
    let db2 = db.clone();
    in_test_worker(
        db.clone(),
        async move {
            let bun_code = format!(
                r#"import * as wmill from "windmill-client";
export async function main() {{
  const secret = await wmill.getVariable("{var_path}");
  console.log("First: " + secret);
  console.log("Second: " + secret);
  console.log("Third: " + secret);
  return "ok";
}}"#
            );

            let job_id = RunJob::from(windmill_common::jobs::JobPayload::Code(RawCode {
                hash: None,
                content: bun_code,
                path: None,
                language: ScriptLang::Bun,
                lock: None,
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: None,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default().into(),
                debouncing_settings:
                    windmill_common::runnable_settings::DebouncingSettings::default(),
                modules: None,
            }))
            .push(&db2)
            .await;

            completed.next().await;
            let cjob = completed_job(job_id, &db2).await;
            assert!(cjob.success, "job should have succeeded");

            let logs = get_job_logs(&db2, job_id)
                .await
                .expect("job_logs entry should exist");

            assert!(
                !logs.contains(secret_value),
                "secret value must not appear anywhere in logs\nLogs:\n{}",
                logs
            );

            // Count occurrences of the mask
            let mask_count = logs.matches("*******").count();
            assert!(
                mask_count >= 3,
                "expected at least 3 masked occurrences, found {}\nLogs:\n{}",
                mask_count,
                logs
            );
        },
        port,
    )
    .await;

    Ok(())
}
