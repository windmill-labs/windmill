#![cfg(all(feature = "private", feature = "agent_worker_server"))]

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::{
    jobs::{JobPayload, RawCode},
    scripts::ScriptLang,
};
use windmill_test_utils::*;

fn bun_code(code: &str) -> RawCode {
    RawCode {
        hash: None,
        content: code.to_string(),
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    }
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_simple_script(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (_client, port, _server) = init_client_agent_mode(db.clone()).await;

    let result = RunJob::from(JobPayload::Code(bun_code(
        "export function main() { return 42; }",
    )))
    .run_until_complete(&db, false, port)
    .await;

    assert!(result.success, "job should succeed");
    assert_eq!(result.json_result(), Some(json!(42)));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_script_with_args(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (_client, port, _server) = init_client_agent_mode(db.clone()).await;

    let result = RunJob::from(JobPayload::Code(bun_code(
        "export function main(x: number, y: number) { return x + y; }",
    )))
    .arg("x", json!(10))
    .arg("y", json!(32))
    .run_until_complete(&db, false, port)
    .await;

    assert!(result.success, "job should succeed");
    assert_eq!(result.json_result(), Some(json!(42)));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_script_with_logs(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (_client, port, _server) = init_client_agent_mode(db.clone()).await;

    let result = RunJob::from(JobPayload::Code(bun_code(
        r#"export function main() {
    console.log("hello from agent worker");
    console.log("processing step 1");
    console.log("processing step 2");
    return "done";
}"#,
    )))
    .run_until_complete(&db, false, port)
    .await;

    assert!(result.success, "job should succeed");
    assert_eq!(result.json_result(), Some(json!("done")));

    let logs = sqlx::query_scalar::<_, String>(
        "SELECT logs FROM job_logs WHERE job_id = $1 AND workspace_id = 'test-workspace'",
    )
    .bind(result.id)
    .fetch_optional(&db)
    .await?;

    let logs = logs.expect("logs should exist");
    assert!(
        logs.contains("hello from agent worker"),
        "logs should contain the printed output, got: {logs}"
    );

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_script_failure(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (_client, port, _server) = init_client_agent_mode(db.clone()).await;

    let result = RunJob::from(JobPayload::Code(bun_code(
        "export function main() { throw new Error('test error'); }",
    )))
    .run_until_complete(&db, false, port)
    .await;

    assert!(!result.success, "job should fail");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_complex_result(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (_client, port, _server) = init_client_agent_mode(db.clone()).await;

    let result = RunJob::from(JobPayload::Code(bun_code(
        r#"export function main() {
    return {
        items: [1, 2, 3],
        metadata: { key: "value" },
        count: 3,
    };
}"#,
    )))
    .run_until_complete(&db, false, port)
    .await;

    assert!(result.success, "job should succeed");
    let json = result.json_result().unwrap();
    assert_eq!(json["items"], json!([1, 2, 3]));
    assert_eq!(json["metadata"]["key"], json!("value"));
    assert_eq!(json["count"], json!(3));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_token_creation(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (client, _port, _server) = init_client_agent_mode(db.clone()).await;

    // client.baseurl() already includes /api
    let resp = client
        .client()
        .post(format!(
            "{}/agent_workers/create_agent_token",
            client.baseurl()
        ))
        .json(&json!({
            "worker_group": "lifecycle-test",
            "tags": ["bun", "flow", "dependency"],
            "exp": usize::MAX
        }))
        .send()
        .await?;

    assert!(
        resp.status().is_success(),
        "create_agent_token should succeed, got: {}",
        resp.status()
    );

    let token = resp.text().await?;
    let token = token.trim_matches('"');
    assert!(
        token.starts_with("jwt_agent_"),
        "token should start with jwt_agent_ prefix, got: {token}"
    );

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_token_and_ping(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (client, port, _server) = init_client_agent_mode(db.clone()).await;

    let resp = client
        .client()
        .post(format!(
            "{}/agent_workers/create_agent_token",
            client.baseurl()
        ))
        .json(&json!({
            "worker_group": "lifecycle-test",
            "tags": ["bun", "flow", "dependency"],
            "exp": usize::MAX
        }))
        .send()
        .await?;

    assert!(resp.status().is_success());
    let token = resp.text().await?;
    let token = token.trim_matches('"');

    let suffix = windmill_common::utils::create_default_worker_suffix("lifecycle-test");
    let base_url = format!("http://localhost:{port}");
    let http_client =
        windmill_common::agent_workers::build_agent_http_client(&suffix, &token, &base_url);

    // Initial ping inserts the worker record into the database
    let resp = http_client
        .client
        .post(format!("{}/api/agent_workers/update_ping", base_url))
        .json(&json!({
            "worker_instance": "test-instance",
            "ip": "127.0.0.1",
            "tags": ["bun"],
            "version": "test",
            "vcpus": 4,
            "memory": 8192,
            "ping_type": "Initial"
        }))
        .send()
        .await?;

    assert!(
        resp.status().is_success(),
        "initial ping should succeed, got: {}",
        resp.status()
    );

    // Verify the ping was recorded in the database
    let worker_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM worker_ping WHERE worker_instance = 'test-instance'",
    )
    .fetch_one(&db)
    .await?;

    assert!(
        worker_count > 0,
        "worker ping should be recorded in database"
    );

    // MainLoop ping updates the existing record
    let resp = http_client
        .client
        .post(format!("{}/api/agent_workers/update_ping", base_url))
        .json(&json!({
            "tags": ["bun"],
            "vcpus": 4,
            "memory": 8192,
            "jobs_executed": 0,
            "ping_type": "MainLoop"
        }))
        .send()
        .await?;

    assert!(
        resp.status().is_success(),
        "main loop ping should succeed, got: {}",
        resp.status()
    );

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_multiple_jobs_sequential(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (_client, port, _server) = init_client_agent_mode(db.clone()).await;

    for i in 0..3 {
        let result = RunJob::from(JobPayload::Code(bun_code(&format!(
            "export function main() {{ return {i}; }}"
        ))))
        .run_until_complete(&db, false, port)
        .await;

        assert!(result.success, "job {i} should succeed");
        assert_eq!(result.json_result(), Some(json!(i)));
    }

    Ok(())
}

/// Test the volume HTTP proxy endpoints that agent workers use.
///
/// Exercises the full volume lifecycle via HTTP:
/// 1. Configure workspace S3 storage (FilesystemStorage)
/// 2. Pre-populate a volume with a file
/// 3. POST /begin — acquire lease, get manifest
/// 4. GET /file/* — download existing file
/// 5. PUT /file/* — upload a new file
/// 6. POST /commit — finalize with stats, release lease
/// 7. Verify DB state and storage
#[cfg(feature = "parquet")]
#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_volume_e2e(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (client, _port, _server) = init_client_agent_mode(db.clone()).await;

    // 1. Set up filesystem-based object storage in a temp dir
    let storage_dir = tempfile::tempdir()?;
    let storage_root = storage_dir.path().to_string_lossy().to_string();

    let lfs_config = json!({
        "type": "FilesystemStorage",
        "root_path": storage_root,
        "public_resource": null,
        "advanced_permissions": null
    });

    sqlx::query!(
        "UPDATE workspace_settings SET large_file_storage = $1 WHERE workspace_id = $2",
        lfs_config,
        "test-workspace"
    )
    .execute(&db)
    .await?;

    // 2. Pre-populate the volume with a file
    let vol_dir = storage_dir.path().join("volumes").join("test-vol");
    std::fs::create_dir_all(&vol_dir)?;
    std::fs::write(vol_dir.join("hello.txt"), b"hello from volume")?;

    let base = client.baseurl();
    let http = client.client();
    let vol_base = format!("{base}/w/test-workspace/volumes/test-vol");

    // 3. POST /begin — acquire lease, get manifest + permissions
    let resp = http
        .post(format!("{vol_base}/begin"))
        .json(&json!({
            "worker_name": "test-worker-1",
            "permissioned_as": "u/test-user"
        }))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "begin should succeed, got: {}",
        resp.status()
    );

    let begin_body: serde_json::Value = resp.json().await?;
    assert!(
        begin_body["writable"].as_bool().unwrap(),
        "should be writable"
    );
    let manifest = begin_body["manifest"].as_object().unwrap();
    assert!(
        manifest.contains_key("hello.txt"),
        "manifest should contain hello.txt, got: {manifest:?}"
    );

    // 4. GET /file/* — download the existing file
    let resp = http
        .get(format!("{vol_base}/file/hello.txt"))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "file download should succeed, got: {}",
        resp.status()
    );
    let file_bytes = resp.bytes().await?;
    assert_eq!(
        file_bytes.as_ref(),
        b"hello from volume",
        "downloaded file content should match"
    );

    // 5. PUT /file/* — upload a new file
    let resp = http
        .put(format!("{vol_base}/file/output.txt"))
        .body(b"written by agent worker".to_vec())
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "file upload should succeed, got: {}",
        resp.status()
    );

    // 6. POST /commit — finalize: report stats, release lease
    let resp = http
        .post(format!("{vol_base}/commit"))
        .json(&json!({
            "worker_name": "test-worker-1",
            "deleted_keys": [],
            "symlinks": {},
            "file_count": 2,
            "size_bytes": 39
        }))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "commit should succeed, got: {}",
        resp.status()
    );

    // 7. Verify volume DB row was updated
    let vol_row = sqlx::query!(
        "SELECT size_bytes, file_count, leased_by, lease_until
         FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "test-vol"
    )
    .fetch_optional(&db)
    .await?;

    let vol_row = vol_row.expect("volume row should exist");
    assert_eq!(vol_row.file_count, 2, "file_count should be 2");
    assert_eq!(vol_row.size_bytes, 39, "size_bytes should match");
    assert!(vol_row.leased_by.is_none(), "lease should be released");
    assert!(
        vol_row.lease_until.is_none() || vol_row.lease_until.unwrap() < chrono::Utc::now(),
        "lease_until should be cleared or in the past"
    );

    // 8. Verify the uploaded file was persisted in storage
    let output_path = vol_dir.join("output.txt");
    assert!(output_path.exists(), "output.txt should be in storage");
    let output_content = std::fs::read_to_string(&output_path)?;
    assert_eq!(output_content, "written by agent worker");

    Ok(())
}

/// Full E2E test: agent worker in HTTP mode runs a Bun script with a volume mount.
///
/// The worker pulls the job via HTTP, downloads volume files via the server-side
/// volume proxy endpoints, executes the script, and syncs changes back.
#[cfg(all(feature = "parquet", feature = "enterprise"))]
#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_volume_http_worker_e2e(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (_client, port, _server) = init_client_agent_mode(db.clone()).await;

    // 1. Set up filesystem-based object storage in a temp dir
    let storage_dir = tempfile::tempdir()?;
    let storage_root = storage_dir.path().to_string_lossy().to_string();

    let lfs_config = json!({
        "type": "FilesystemStorage",
        "root_path": storage_root,
        "public_resource": null,
        "advanced_permissions": null
    });

    sqlx::query!(
        "UPDATE workspace_settings SET large_file_storage = $1 WHERE workspace_id = $2",
        lfs_config,
        "test-workspace"
    )
    .execute(&db)
    .await?;

    // 2. Pre-populate the volume with a file
    let vol_dir = storage_dir.path().join("volumes").join("test-vol");
    std::fs::create_dir_all(&vol_dir)?;
    std::fs::write(vol_dir.join("hello.txt"), b"hello from volume")?;

    // 3. Push the job, then run worker with HTTP connection (bun tag)
    let code = r#"// volume: test-vol /tmp/data
import { readFileSync, writeFileSync, existsSync } from "fs";

export function main() {
    const content = readFileSync("/tmp/data/hello.txt", "utf-8");
    writeFileSync("/tmp/data/output.txt", "written by agent worker");
    return {
        read_content: content,
        output_exists: existsSync("/tmp/data/output.txt"),
    };
}"#;

    let uuid = RunJob::from(JobPayload::Code(bun_code(code)))
        .push(&db)
        .await;
    let listener = listen_for_completed_jobs(&db).await;

    let conn = testing_http_connection_with_tags(
        port,
        vec!["bun".into(), "flow".into(), "dependency".into()],
    )
    .await;

    in_test_worker(conn, listener.find(&uuid), port).await;

    let result = completed_job(uuid, &db).await;

    assert!(result.success, "job should succeed: {:?}", result.result);
    let json = result.json_result().expect("should have JSON result");
    assert_eq!(json["read_content"], json!("hello from volume"));
    assert_eq!(json["output_exists"], json!(true));

    // 4. Verify volume DB row was updated
    let vol_row = sqlx::query!(
        "SELECT size_bytes, file_count, leased_by, lease_until
         FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "test-vol"
    )
    .fetch_optional(&db)
    .await?;

    let vol_row = vol_row.expect("volume row should exist");
    assert!(
        vol_row.file_count >= 2,
        "should have at least 2 files (hello.txt + output.txt), got: {}",
        vol_row.file_count
    );
    assert!(vol_row.size_bytes > 0, "size_bytes should be > 0");
    assert!(vol_row.leased_by.is_none(), "lease should be released");
    assert!(
        vol_row.lease_until.is_none() || vol_row.lease_until.unwrap() < chrono::Utc::now(),
        "lease_until should be cleared or in the past"
    );

    // 5. Verify the new file was written back to the storage
    let output_path = vol_dir.join("output.txt");
    assert!(
        output_path.exists(),
        "output.txt should be synced back to storage"
    );
    let output_content = std::fs::read_to_string(&output_path)?;
    assert_eq!(output_content, "written by agent worker");

    Ok(())
}

/// Test the volume release endpoint (error/cancel path).
#[cfg(feature = "parquet")]
#[sqlx::test(fixtures("base"))]
async fn test_agent_worker_volume_release(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (client, _port, _server) = init_client_agent_mode(db.clone()).await;

    // Set up filesystem storage
    let storage_dir = tempfile::tempdir()?;
    let storage_root = storage_dir.path().to_string_lossy().to_string();
    let lfs_config = json!({
        "type": "FilesystemStorage",
        "root_path": storage_root,
        "public_resource": null,
        "advanced_permissions": null
    });
    sqlx::query!(
        "UPDATE workspace_settings SET large_file_storage = $1 WHERE workspace_id = $2",
        lfs_config,
        "test-workspace"
    )
    .execute(&db)
    .await?;

    let base = client.baseurl();
    let http = client.client();
    let vol_base = format!("{base}/w/test-workspace/volumes/test-vol");

    // Begin (acquire lease)
    let resp = http
        .post(format!("{vol_base}/begin"))
        .json(&json!({
            "worker_name": "test-worker-2",
            "permissioned_as": "u/test-user"
        }))
        .send()
        .await?;
    assert!(resp.status().is_success(), "begin should succeed");

    // Verify lease is held
    let leased = sqlx::query_scalar!(
        "SELECT leased_by FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "test-vol"
    )
    .fetch_optional(&db)
    .await?
    .flatten();
    assert_eq!(leased.as_deref(), Some("test-worker-2"));

    // Release without commit (simulating error path)
    let resp = http
        .post(format!("{vol_base}/release"))
        .json(&json!({ "worker_name": "test-worker-2" }))
        .send()
        .await?;
    assert!(resp.status().is_success(), "release should succeed");

    // Verify lease is cleared
    let leased = sqlx::query_scalar!(
        "SELECT leased_by FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "test-vol"
    )
    .fetch_optional(&db)
    .await?
    .flatten();
    assert!(leased.is_none(), "lease should be released");

    Ok(())
}
