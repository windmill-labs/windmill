//! Integration tests for sensitive log masking.
//!
//! A single comprehensive test that runs real bun scripts through real workers,
//! covering all masking scenarios: secret variables, non-secret variables,
//! multiple secrets, mid-string secrets, `$encrypted:` args, resources
//! referencing secret variables, and cross-job isolation.
//!
//! Run with:
//!   cargo test -p windmill-api-integration-tests --test sensitive_log_masking -- --nocapture
//!
//! Requires: bun runtime, live database (migrations applied by sqlx::test).

use futures::StreamExt;
use serde_json::json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::jobs::{JobPayload, RawCode};
use windmill_common::scripts::ScriptLang;
use windmill_common::worker::to_raw_value;
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

/// Helper: create a variable via the API.
async fn create_variable(port: u16, path: &str, value: &str, is_secret: bool) {
    let base = format!("http://localhost:{port}/api/w/test-workspace/variables");
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": path,
            "value": value,
            "is_secret": is_secret,
            "description": "test variable for log masking"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        201,
        "failed to create variable {path}: {}",
        resp.text().await.unwrap_or_default()
    );
}

/// Helper: create a resource via the API.
async fn create_resource(port: u16, path: &str, value: serde_json::Value) {
    let base = format!("http://localhost:{port}/api/w/test-workspace/resources");
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": path,
            "value": value,
            "resource_type": "object",
            "description": "test resource for log masking"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        201,
        "failed to create resource {path}: {}",
        resp.text().await.unwrap_or_default()
    );
}

/// Helper: fetch job logs from the job_logs table.
async fn get_job_logs(db: &Pool<Postgres>, job_id: Uuid) -> Option<String> {
    sqlx::query_scalar!(
        r#"SELECT logs as "logs!" FROM job_logs WHERE job_id = $1"#,
        job_id,
    )
    .fetch_optional(db)
    .await
    .unwrap()
}

/// Helper: push a bun preview job and return its UUID.
async fn push_bun_job(db: &Pool<Postgres>, code: String) -> Uuid {
    RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: code,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .push(db)
    .await
}

/// Helper: push a bun preview job with encrypted args.
async fn push_bun_job_with_encrypted_arg(
    db: &Pool<Postgres>,
    code: String,
    arg_name: &str,
    plaintext_value: &str,
) -> Uuid {
    // We need to know the job_id in advance to encrypt with the right key suffix.
    let job_id = Uuid::new_v4();

    // Encrypt the value the same way the frontend does:
    // build_crypt_with_key_suffix(db, workspace, root_job_id)
    let mc = windmill_common::variables::build_crypt_with_key_suffix(
        db,
        "test-workspace",
        &job_id.to_string(),
    )
    .await
    .expect("build_crypt_with_key_suffix");

    // Encrypt the JSON-serialized string value
    let json_str = serde_json::to_string(plaintext_value).unwrap();
    let encrypted = windmill_common::variables::encrypt(&mc, &json_str);
    let arg_value = format!("$encrypted:{encrypted}");

    let mut args = std::collections::HashMap::new();
    args.insert(arg_name.to_string(), to_raw_value(&json!(arg_value)));

    RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: code,
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        modules: None,
    }))
    .job_id(job_id)
    .arg(arg_name, json!(arg_value))
    .push(db)
    .await
}

/// Comprehensive test covering all sensitive log masking scenarios in a single
/// test function to amortize server/worker startup cost.
///
/// Scenarios covered (each as a separate job inside the same worker):
///   1. Secret variable fetched and logged → masked
///   2. Non-secret variable fetched and logged → NOT masked (no false positives)
///   3. Two different secrets fetched and logged in the same job → both masked
///   4. Secret embedded mid-string (e.g. "token=SECRET&user=bob") → masked
///   5. Same secret logged 3 times → all occurrences masked
///   6. `$encrypted:` password arg logged → masked
///   7. Resource referencing a secret variable via `$var:` → secret masked when logged
///   8. Cross-job isolation: job A's secret does NOT leak into job B's logs
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_sensitive_log_masking(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // === Setup: create variables and resources ===
    let secret1 = "alpha_secret_value_9x7k2m";
    let secret2 = "beta_secret_token_4j8n3p";
    let plain_val = "plain_visible_value_12345";
    let encrypted_password = "encrypted_pass_w0rd_zq5r";
    let resource_secret = "resource_db_password_h7t2";

    create_variable(port, "u/test-user/secret_alpha", secret1, true).await;
    create_variable(port, "u/test-user/secret_beta", secret2, true).await;
    create_variable(port, "u/test-user/plain_var", plain_val, false).await;
    // Secret variable that will be referenced by a resource via $var:
    create_variable(port, "u/test-user/res_secret_var", resource_secret, true).await;
    // Resource whose "password" field references the secret variable
    create_resource(
        port,
        "u/test-user/db_with_secret",
        json!({"host": "db.example.com", "password": "$var:u/test-user/res_secret_var"}),
    )
    .await;

    let mut completed = listen_for_completed_jobs(&db).await;
    let db2 = db.clone();
    in_test_worker(
        db.clone(),
        async move {
            // ================================================================
            // Scenario 1: Secret variable fetched and console.logged → masked
            // ================================================================
            let job1 = push_bun_job(
                &db2,
                r#"import * as wmill from "windmill-client";
export async function main() {
  const secret = await wmill.getVariable("u/test-user/secret_alpha");
  console.log("The secret value is: " + secret);
  return "ok";
}"#
                .into(),
            )
            .await;
            completed.next().await;
            let cjob1 = completed_job(job1, &db2).await;
            assert!(cjob1.success, "scenario 1 job failed");
            let logs1 = get_job_logs(&db2, job1).await.expect("scenario 1: no logs");

            assert!(
                !logs1.contains(secret1),
                "scenario 1: secret value leaked in logs\nLogs:\n{logs1}"
            );
            assert!(
                logs1.contains("The secret value is: alp*****"),
                "scenario 1: expected masked output with first 3 chars\nLogs:\n{logs1}"
            );
            assert!(
                logs1.contains("[windmill] secret value was masked for security reasons, use string transformations to display full value"),
                "scenario 1: expected security notice\nLogs:\n{logs1}"
            );

            // ================================================================
            // Scenario 2: Non-secret variable → NOT masked (no false positives)
            // ================================================================
            let job2 = push_bun_job(
                &db2,
                r#"import * as wmill from "windmill-client";
export async function main() {
  const val = await wmill.getVariable("u/test-user/plain_var");
  console.log("The plain value is: " + val);
  return "ok";
}"#
                .into(),
            )
            .await;
            completed.next().await;
            let cjob2 = completed_job(job2, &db2).await;
            assert!(cjob2.success, "scenario 2 job failed");
            let logs2 = get_job_logs(&db2, job2).await.expect("scenario 2: no logs");

            assert!(
                logs2.contains(plain_val),
                "scenario 2: plain value should appear unmasked\nLogs:\n{logs2}"
            );

            // ================================================================
            // Scenario 3: Two different secrets fetched in the same job → both masked
            // ================================================================
            let job3 = push_bun_job(
                &db2,
                r#"import * as wmill from "windmill-client";
export async function main() {
  const s1 = await wmill.getVariable("u/test-user/secret_alpha");
  const s2 = await wmill.getVariable("u/test-user/secret_beta");
  console.log("secret1=" + s1);
  console.log("secret2=" + s2);
  return "ok";
}"#
                .into(),
            )
            .await;
            completed.next().await;
            let cjob3 = completed_job(job3, &db2).await;
            assert!(cjob3.success, "scenario 3 job failed");
            let logs3 = get_job_logs(&db2, job3).await.expect("scenario 3: no logs");

            assert!(
                !logs3.contains(secret1),
                "scenario 3: secret1 leaked\nLogs:\n{logs3}"
            );
            assert!(
                !logs3.contains(secret2),
                "scenario 3: secret2 leaked\nLogs:\n{logs3}"
            );
            assert!(
                logs3.contains("secret1=alp*****"),
                "scenario 3: secret1 not masked\nLogs:\n{logs3}"
            );
            assert!(
                logs3.contains("secret2=bet*****"),
                "scenario 3: secret2 not masked\nLogs:\n{logs3}"
            );

            // ================================================================
            // Scenario 4: Secret embedded mid-string → masked in place
            // ================================================================
            let job4 = push_bun_job(
                &db2,
                r#"import * as wmill from "windmill-client";
export async function main() {
  const secret = await wmill.getVariable("u/test-user/secret_alpha");
  console.log("token=" + secret + "&user=bob&format=json");
  return "ok";
}"#
                .into(),
            )
            .await;
            completed.next().await;
            let cjob4 = completed_job(job4, &db2).await;
            assert!(cjob4.success, "scenario 4 job failed");
            let logs4 = get_job_logs(&db2, job4).await.expect("scenario 4: no logs");

            assert!(
                !logs4.contains(secret1),
                "scenario 4: secret leaked mid-string\nLogs:\n{logs4}"
            );
            assert!(
                logs4.contains("token=alp*****&user=bob&format=json"),
                "scenario 4: mid-string masking failed\nLogs:\n{logs4}"
            );

            // ================================================================
            // Scenario 5: Same secret logged 3 times → all occurrences masked
            // ================================================================
            let job5 = push_bun_job(
                &db2,
                r#"import * as wmill from "windmill-client";
export async function main() {
  const secret = await wmill.getVariable("u/test-user/secret_beta");
  console.log("First: " + secret);
  console.log("Second: " + secret);
  console.log("Third: " + secret);
  return "ok";
}"#
                .into(),
            )
            .await;
            completed.next().await;
            let cjob5 = completed_job(job5, &db2).await;
            assert!(cjob5.success, "scenario 5 job failed");
            let logs5 = get_job_logs(&db2, job5).await.expect("scenario 5: no logs");

            assert!(
                !logs5.contains(secret2),
                "scenario 5: secret leaked\nLogs:\n{logs5}"
            );
            let mask_count = logs5.matches("bet*****").count();
            assert!(
                mask_count >= 3,
                "scenario 5: expected >= 3 masked occurrences, found {mask_count}\nLogs:\n{logs5}"
            );

            // ================================================================
            // Scenario 6: $encrypted: password arg → masked when logged
            // ================================================================
            let job6 = push_bun_job_with_encrypted_arg(
                &db2,
                r#"export async function main(password: string) {
  console.log("password is: " + password);
  return "ok";
}"#
                .into(),
                "password",
                encrypted_password,
            )
            .await;
            completed.next().await;
            let cjob6 = completed_job(job6, &db2).await;
            assert!(cjob6.success, "scenario 6 job failed");
            let logs6 = get_job_logs(&db2, job6).await.expect("scenario 6: no logs");

            assert!(
                !logs6.contains(encrypted_password),
                "scenario 6: encrypted password leaked\nLogs:\n{logs6}"
            );
            assert!(
                logs6.contains("password is: enc*****"),
                "scenario 6: encrypted password not masked\nLogs:\n{logs6}"
            );

            // ================================================================
            // Scenario 7: Resource with $var: referencing a secret → masked
            // ================================================================
            let job7 = push_bun_job(
                &db2,
                r#"import * as wmill from "windmill-client";
export async function main() {
  const res = await wmill.getResource("u/test-user/db_with_secret");
  console.log("db password: " + res.password);
  console.log("db host: " + res.host);
  return "ok";
}"#
                .into(),
            )
            .await;
            completed.next().await;
            let cjob7 = completed_job(job7, &db2).await;
            assert!(cjob7.success, "scenario 7 job failed");
            let logs7 = get_job_logs(&db2, job7).await.expect("scenario 7: no logs");

            assert!(
                !logs7.contains(resource_secret),
                "scenario 7: resource secret leaked\nLogs:\n{logs7}"
            );
            assert!(
                logs7.contains("db password: res*****"),
                "scenario 7: resource secret not masked\nLogs:\n{logs7}"
            );
            // Non-secret field should remain visible
            assert!(
                logs7.contains("db host: db.example.com"),
                "scenario 7: non-secret resource field should be visible\nLogs:\n{logs7}"
            );

            // ================================================================
            // Scenario 8: Cross-job isolation — job A fetches secret_alpha,
            //              then job B logs "alpha_secret_value_9x7k2m" as a
            //              literal string (not fetched as a secret).
            //              Job B should NOT mask it because the secret belongs
            //              to job A which already completed.
            // ================================================================
            // Job A: fetch the secret (registers it) then completes
            let job_a = push_bun_job(
                &db2,
                r#"import * as wmill from "windmill-client";
export async function main() {
  const s = await wmill.getVariable("u/test-user/secret_alpha");
  console.log("fetched secret");
  return "ok";
}"#
                .into(),
            )
            .await;
            completed.next().await;
            let cjob_a = completed_job(job_a, &db2).await;
            assert!(cjob_a.success, "scenario 8 job A failed");

            // Job B: logs the same string as a hardcoded literal (NOT fetched as secret)
            // Since job A already completed and unregistered, and job B never
            // fetched the secret, it should NOT be masked.
            let job_b_code = format!(
                r#"export async function main() {{
  console.log("literal value: {secret1}");
  return "ok";
}}"#
            );
            let job_b = push_bun_job(&db2, job_b_code).await;
            completed.next().await;
            let cjob_b = completed_job(job_b, &db2).await;
            assert!(cjob_b.success, "scenario 8 job B failed");
            let logs_b = get_job_logs(&db2, job_b)
                .await
                .expect("scenario 8 job B: no logs");

            assert!(
                logs_b.contains(secret1),
                "scenario 8: job B should show the literal string unmasked (it never fetched a secret)\nLogs:\n{logs_b}"
            );
        },
        port,
    )
    .await;

    Ok(())
}
