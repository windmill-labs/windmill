use serde::Deserialize;
use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_common::{
    jobs::{JobPayload, RawCode},
    scripts::ScriptLang,
};

use windmill_test_utils::*;

#[derive(Debug, Deserialize)]
struct ListJobsResponse {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    typ: String,
    id: String,
    #[serde(default)]
    args: Option<serde_json::Value>,
    #[serde(flatten)]
    _extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Response struct for queue/list endpoint (no type field)
#[derive(Debug, Deserialize)]
struct QueueJobResponse {
    id: String,
    #[serde(default)]
    args: Option<serde_json::Value>,
    #[serde(flatten)]
    _extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Response struct for completed/list endpoint
#[cfg(feature = "python")]
#[derive(Debug, Deserialize)]
struct CompletedJobResponse {
    id: String,
    #[serde(default)]
    args: Option<serde_json::Value>,
    #[serde(flatten)]
    _extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Test that list_jobs returns jobs without args by default
#[sqlx::test(fixtures("base"))]
async fn test_list_jobs_without_include_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    // Push a job to the queue with specific args
    let job_id = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "def main(x): return x * 2".to_string(),
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }))
    .arg("x", json!(42))
    .push(&db)
    .await;

    // Call list_jobs without include_args
    let response = client
        .client()
        .get(format!("{}/w/test-workspace/jobs/list", client.baseurl()))
        .send()
        .await?;

    assert!(response.status().is_success(), "list_jobs should succeed");

    let jobs: Vec<ListJobsResponse> = response.json().await?;

    // Find the job we created
    let job = jobs
        .iter()
        .find(|j| j.id == job_id.to_string())
        .expect("should find the created job");

    // Args should be None when include_args is not set
    assert!(
        job.args.is_none(),
        "args should not be included when include_args is not set"
    );

    Ok(())
}

/// Test that list_jobs returns jobs with args when include_args=true
#[sqlx::test(fixtures("base"))]
async fn test_list_jobs_with_include_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    // Push a job to the queue with specific args
    let job_id = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "def main(x): return x * 2".to_string(),
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }))
    .arg("x", json!(42))
    .push(&db)
    .await;

    // Call list_jobs with include_args=true
    let response = client
        .client()
        .get(format!(
            "{}/w/test-workspace/jobs/list?include_args=true",
            client.baseurl()
        ))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "list_jobs with include_args should succeed"
    );

    let jobs: Vec<ListJobsResponse> = response.json().await?;

    // Find the job we created
    let job = jobs
        .iter()
        .find(|j| j.id == job_id.to_string())
        .expect("should find the created job");

    // Args should be present when include_args=true
    assert!(
        job.args.is_some(),
        "args should be included when include_args=true"
    );

    let args = job.args.as_ref().unwrap();
    assert_eq!(
        args.get("x"),
        Some(&json!(42)),
        "args should contain the correct value"
    );

    Ok(())
}

/// Test that list_jobs returns completed jobs with args when include_args=true
#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_list_jobs_completed_with_include_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    // Run a job to completion
    let completed_job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "def main(x): return x * 2".to_string(),
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }))
    .arg("x", json!(42))
    .run_until_complete(&db, false, port)
    .await;

    let job_id = completed_job.id;

    // Call list_jobs with include_args=true
    let response = client
        .client()
        .get(format!(
            "{}/w/test-workspace/jobs/list?include_args=true",
            client.baseurl()
        ))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "list_jobs with include_args should succeed"
    );

    let jobs: Vec<ListJobsResponse> = response.json().await?;

    // Find the completed job
    let job = jobs
        .iter()
        .find(|j| j.id == job_id.to_string())
        .expect("should find the completed job");

    // Args should be present when include_args=true
    assert!(
        job.args.is_some(),
        "args should be included for completed jobs when include_args=true"
    );

    let args = job.args.as_ref().unwrap();
    assert_eq!(
        args.get("x"),
        Some(&json!(42)),
        "args should contain the correct value for completed jobs"
    );

    Ok(())
}

/// Test that list_jobs returns both queued and completed jobs with args
#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_list_jobs_mixed_queue_and_completed(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    // Run a job to completion first
    let completed_job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "def main(completed_arg): return completed_arg".to_string(),
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }))
    .arg("completed_arg", json!("completed_value"))
    .run_until_complete(&db, false, port)
    .await;

    let completed_job_id = completed_job.id;

    // Push another job to the queue (don't run it)
    let queued_job_id = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "def main(queued_arg): return queued_arg".to_string(),
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }))
    .arg("queued_arg", json!("queued_value"))
    .push(&db)
    .await;

    // Call list_jobs with include_args=true
    let response = client
        .client()
        .get(format!(
            "{}/w/test-workspace/jobs/list?include_args=true",
            client.baseurl()
        ))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "list_jobs with include_args should succeed"
    );

    let jobs: Vec<ListJobsResponse> = response.json().await?;

    // Find the completed job
    let completed = jobs
        .iter()
        .find(|j| j.id == completed_job_id.to_string())
        .expect("should find the completed job");

    assert!(
        completed.args.is_some(),
        "completed job should have args when include_args=true"
    );
    assert_eq!(
        completed.args.as_ref().unwrap().get("completed_arg"),
        Some(&json!("completed_value")),
        "completed job should have correct args"
    );

    // Find the queued job
    let queued = jobs
        .iter()
        .find(|j| j.id == queued_job_id.to_string())
        .expect("should find the queued job");

    assert!(
        queued.args.is_some(),
        "queued job should have args when include_args=true"
    );
    assert_eq!(
        queued.args.as_ref().unwrap().get("queued_arg"),
        Some(&json!("queued_value")),
        "queued job should have correct args"
    );

    Ok(())
}

/// Test list_jobs with multiple queued jobs and include_args
#[sqlx::test(fixtures("base"))]
async fn test_list_jobs_multiple_queued_with_include_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    // Push two jobs with different args
    let job1_id = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "def main(x): return x".to_string(),
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }))
    .arg("x", json!(1))
    .push(&db)
    .await;

    let job2_id = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "def main(y): return y".to_string(),
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }))
    .arg("y", json!(2))
    .push(&db)
    .await;

    // Test with include_args=true
    let response = client
        .client()
        .get(format!(
            "{}/w/test-workspace/jobs/list?include_args=true",
            client.baseurl()
        ))
        .send()
        .await?;

    assert!(response.status().is_success(), "list_jobs should succeed");

    let jobs: Vec<ListJobsResponse> = response.json().await?;

    // Find both jobs
    let job1 = jobs.iter().find(|j| j.id == job1_id.to_string())
        .expect("should find job1");
    let job2 = jobs.iter().find(|j| j.id == job2_id.to_string())
        .expect("should find job2");

    // Both should have args
    assert!(job1.args.is_some(), "job1 args should be included");
    assert!(job2.args.is_some(), "job2 args should be included");

    // Check the args are correct
    assert_eq!(
        job1.args.as_ref().unwrap().get("x"),
        Some(&json!(1)),
        "job1 should have correct args"
    );
    assert_eq!(
        job2.args.as_ref().unwrap().get("y"),
        Some(&json!(2)),
        "job2 should have correct args"
    );

    Ok(())
}

// ============================================================================
// Tests for /queue/list endpoint
// ============================================================================

/// Test that queue/list returns jobs without args by default
#[sqlx::test(fixtures("base"))]
async fn test_queue_list_without_include_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    // Push a job to the queue with specific args
    let job_id = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "def main(x): return x * 2".to_string(),
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }))
    .arg("x", json!(42))
    .push(&db)
    .await;

    // Call queue/list without include_args
    let response = client
        .client()
        .get(format!(
            "{}/w/test-workspace/jobs/queue/list",
            client.baseurl()
        ))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "queue/list should succeed"
    );

    let jobs: Vec<QueueJobResponse> = response.json().await?;

    // Find the job we created
    let job = jobs
        .iter()
        .find(|j| j.id == job_id.to_string())
        .expect("should find the created job");

    // Args should be None when include_args is not set
    assert!(
        job.args.is_none(),
        "args should not be included when include_args is not set"
    );

    Ok(())
}

/// Test that queue/list returns jobs with args when include_args=true
#[sqlx::test(fixtures("base"))]
async fn test_queue_list_with_include_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    // Push a job to the queue with specific args
    let job_id = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "def main(x): return x * 2".to_string(),
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }))
    .arg("x", json!(42))
    .push(&db)
    .await;

    // Call queue/list with include_args=true
    let response = client
        .client()
        .get(format!(
            "{}/w/test-workspace/jobs/queue/list?include_args=true",
            client.baseurl()
        ))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "queue/list with include_args should succeed"
    );

    let jobs: Vec<QueueJobResponse> = response.json().await?;

    // Find the job we created
    let job = jobs
        .iter()
        .find(|j| j.id == job_id.to_string())
        .expect("should find the created job");

    // Args should be present when include_args=true
    assert!(
        job.args.is_some(),
        "args should be included when include_args=true"
    );

    let args = job.args.as_ref().unwrap();
    assert_eq!(
        args.get("x"),
        Some(&json!(42)),
        "args should contain the correct value"
    );

    Ok(())
}

/// Test queue/list with multiple jobs and include_args=true
#[sqlx::test(fixtures("base"))]
async fn test_queue_list_multiple_jobs_with_include_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    // Push two jobs with different args
    let job1_id = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "def main(a): return a".to_string(),
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }))
    .arg("a", json!("value_a"))
    .push(&db)
    .await;

    let job2_id = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "def main(b): return b".to_string(),
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }))
    .arg("b", json!("value_b"))
    .push(&db)
    .await;

    // Call queue/list with include_args=true
    let response = client
        .client()
        .get(format!(
            "{}/w/test-workspace/jobs/queue/list?include_args=true",
            client.baseurl()
        ))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "queue/list should succeed"
    );

    let jobs: Vec<QueueJobResponse> = response.json().await?;

    // Find both jobs
    let job1 = jobs
        .iter()
        .find(|j| j.id == job1_id.to_string())
        .expect("should find job1");
    let job2 = jobs
        .iter()
        .find(|j| j.id == job2_id.to_string())
        .expect("should find job2");

    // Both should have args
    assert!(job1.args.is_some(), "job1 args should be included");
    assert!(job2.args.is_some(), "job2 args should be included");

    // Check the args are correct
    assert_eq!(
        job1.args.as_ref().unwrap().get("a"),
        Some(&json!("value_a")),
        "job1 should have correct args"
    );
    assert_eq!(
        job2.args.as_ref().unwrap().get("b"),
        Some(&json!("value_b")),
        "job2 should have correct args"
    );

    Ok(())
}

// ============================================================================
// Tests for /completed/list endpoint
// ============================================================================

/// Test that completed/list returns jobs without args by default
#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_completed_list_without_include_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    // Run a job to completion
    let completed_job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "def main(x): return x * 2".to_string(),
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }))
    .arg("x", json!(42))
    .run_until_complete(&db, false, port)
    .await;

    let job_id = completed_job.id;

    // Call completed/list without include_args
    let response = client
        .client()
        .get(format!(
            "{}/w/test-workspace/jobs/completed/list",
            client.baseurl()
        ))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "completed/list should succeed"
    );

    let jobs: Vec<CompletedJobResponse> = response.json().await?;

    // Find the completed job
    let job = jobs
        .iter()
        .find(|j| j.id == job_id.to_string())
        .expect("should find the completed job");

    // Args should be None when include_args is not set
    assert!(
        job.args.is_none(),
        "args should not be included when include_args is not set"
    );

    Ok(())
}

/// Test that completed/list returns jobs with args when include_args=true
#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_completed_list_with_include_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    // Run a job to completion
    let completed_job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "def main(x): return x * 2".to_string(),
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }))
    .arg("x", json!(42))
    .run_until_complete(&db, false, port)
    .await;

    let job_id = completed_job.id;

    // Call completed/list with include_args=true
    let response = client
        .client()
        .get(format!(
            "{}/w/test-workspace/jobs/completed/list?include_args=true",
            client.baseurl()
        ))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "completed/list with include_args should succeed"
    );

    let jobs: Vec<CompletedJobResponse> = response.json().await?;

    // Find the completed job
    let job = jobs
        .iter()
        .find(|j| j.id == job_id.to_string())
        .expect("should find the completed job");

    // Args should be present when include_args=true
    assert!(
        job.args.is_some(),
        "args should be included when include_args=true"
    );

    let args = job.args.as_ref().unwrap();
    assert_eq!(
        args.get("x"),
        Some(&json!(42)),
        "args should contain the correct value"
    );

    Ok(())
}

/// Test completed/list with multiple jobs and include_args=true
#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_completed_list_multiple_jobs_with_include_args(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    // Run first job to completion
    let completed_job1 = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "def main(a): return a".to_string(),
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }))
    .arg("a", json!("completed_a"))
    .run_until_complete(&db, false, port)
    .await;

    let job1_id = completed_job1.id;

    // Run second job to completion
    let completed_job2 = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content: "def main(b): return b".to_string(),
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings:
            windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }))
    .arg("b", json!("completed_b"))
    .run_until_complete(&db, false, port)
    .await;

    let job2_id = completed_job2.id;

    // Call completed/list with include_args=true
    let response = client
        .client()
        .get(format!(
            "{}/w/test-workspace/jobs/completed/list?include_args=true",
            client.baseurl()
        ))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "completed/list should succeed"
    );

    let jobs: Vec<CompletedJobResponse> = response.json().await?;

    // Find both completed jobs
    let job1 = jobs
        .iter()
        .find(|j| j.id == job1_id.to_string())
        .expect("should find job1");
    let job2 = jobs
        .iter()
        .find(|j| j.id == job2_id.to_string())
        .expect("should find job2");

    // Both should have args
    assert!(job1.args.is_some(), "job1 args should be included");
    assert!(job2.args.is_some(), "job2 args should be included");

    // Check the args are correct
    assert_eq!(
        job1.args.as_ref().unwrap().get("a"),
        Some(&json!("completed_a")),
        "job1 should have correct args"
    );
    assert_eq!(
        job2.args.as_ref().unwrap().get("b"),
        Some(&json!("completed_b")),
        "job2 should have correct args"
    );

    Ok(())
}
