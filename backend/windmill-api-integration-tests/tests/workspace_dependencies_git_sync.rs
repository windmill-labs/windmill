/*!
 * Integration tests for workspace dependencies git sync.
 *
 * These tests verify that creating, archiving, and deleting workspace dependencies
 * triggers deployment callback jobs with the correct arguments for git sync.
 *
 * Run with enterprise features:
 * ```bash
 * cargo test --test workspace_dependencies_git_sync --features enterprise,private
 * ```
 */

use serde_json::json;
use sqlx::{Pool, Postgres};
use std::time::Duration;

use windmill_test_utils::*;

/// Row shape for querying deployment callback jobs from v2_job_queue
#[derive(Debug)]
#[allow(dead_code)]
struct DeploymentCallbackJob {
    id: uuid::Uuid,
    runnable_path: Option<String>,
    args: Option<serde_json::Value>,
    kind: String,
}

/// Poll for deployment callback jobs in the queue for a given script path
async fn get_deployment_callback_jobs(
    db: &Pool<Postgres>,
    script_path: &str,
    timeout: Duration,
) -> anyhow::Result<Vec<DeploymentCallbackJob>> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let rows = sqlx::query_as!(
            DeploymentCallbackJob,
            r#"
            SELECT j.id, j.runnable_path, j.args, j.kind::text AS "kind!"
            FROM v2_job j
            JOIN v2_job_queue q ON j.id = q.id
            WHERE j.runnable_path = $1
              AND j.kind = 'deploymentcallback'
            ORDER BY j.created_at DESC
            "#,
            script_path,
        )
        .fetch_all(db)
        .await?;

        if !rows.is_empty() {
            return Ok(rows);
        }

        if tokio::time::Instant::now() >= deadline {
            // Return empty if timeout - caller will handle assertion
            return Ok(vec![]);
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Configure git sync for the test workspace with workspace dependencies enabled
async fn setup_git_sync_config(db: &Pool<Postgres>, sync_script_path: &str) -> anyhow::Result<()> {
    let git_sync_config = json!({
        "include_type": ["workspacedependencies"],
        "include_path": ["**"],
        "repositories": [{
            "script_path": sync_script_path,
            "git_repo_resource_path": "$res:u/test-user/test_git_repo",
            "use_individual_branch": false,
            "group_by_folder": false
        }]
    });

    sqlx::query!(
        "UPDATE workspace_settings SET git_sync = $1 WHERE workspace_id = $2",
        git_sync_config,
        "test-workspace"
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Create a git repository resource for testing
async fn create_git_repo_resource(db: &Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO resource (workspace_id, path, value, resource_type, extra_perms, created_by)
        VALUES ('test-workspace', 'u/test-user/test_git_repo', $1::jsonb, 'git_repository', '{}'::jsonb, 'test-user')
        ON CONFLICT (workspace_id, path) DO NOTHING
        "#,
    )
    .bind(json!({
        "url": "https://github.com/test/test.git",
        "branch": "main",
        "token": "test-token"
    }))
    .execute(db)
    .await?;

    Ok(())
}

/// Create a dummy sync script for testing (with version >= 28103 for debouncing support)
async fn create_sync_script(db: &Pool<Postgres>, path: &str) -> anyhow::Result<i64> {
    let hash: i64 = rand::random::<i64>().unsigned_abs() as i64;
    sqlx::query(
        r#"
        INSERT INTO script (workspace_id, hash, path, summary, description, content,
                  created_by, language, kind, lock)
        VALUES ('test-workspace', $1, $2, 'sync script', '',
                'export function main(items: any[]) { return { synced: items.length }; }',
                'test-user', 'bun', 'script', '')
        "#,
    )
    .bind(hash)
    .bind(path)
    .execute(db)
    .await?;
    Ok(hash)
}

/// Create a folder for the versioned script path
async fn create_folder(db: &Pool<Postgres>, name: &str) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
        VALUES ('test-workspace', $1, $1, ARRAY['u/test-user'], '{}'::jsonb, 'test-user')
        ON CONFLICT (workspace_id, name) DO NOTHING
        "#,
    )
    .bind(name)
    .execute(db)
    .await?;
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

/// Test that creating a workspace dependency triggers a git sync deployment callback
/// with the correct path_type and path arguments.
#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_create_workspace_dependencies_triggers_git_sync(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    // Setup: Create folder, git repo resource, sync script, and configure git sync
    create_folder(&db, "28103").await?;
    create_git_repo_resource(&db).await?;
    let sync_script_path = "f/28103/test_sync_script";
    create_sync_script(&db, sync_script_path).await?;
    setup_git_sync_config(&db, sync_script_path).await?;

    // Start API server
    let (client, _port, _server) = init_client(db.clone()).await;

    // Create workspace dependency via API
    let response = client
        .client()
        .post(format!(
            "{}/w/test-workspace/workspace_dependencies/create",
            client.baseurl()
        ))
        .json(&json!({
            "workspace_id": "test-workspace",
            "language": "python3",
            "name": "test-deps",
            "content": "requests==2.28.0\nnumpy==1.24.0"
        }))
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "Failed to create workspace dependency: {:?}",
        response.text().await
    );

    // Wait for deployment callback job to be created
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Query for deployment callback jobs
    let jobs = get_deployment_callback_jobs(&db, sync_script_path, Duration::from_secs(5)).await?;

    assert!(
        !jobs.is_empty(),
        "Expected at least one deployment callback job to be created"
    );

    // Verify the job arguments
    let job = &jobs[0];
    let args = job.args.as_ref().expect("Job should have args");

    // Check that path_type is "workspace_dependencies" (or check items array)
    // The exact structure depends on whether debouncing is enabled
    if let Some(items) = args.get("items") {
        // Debounced format: items is an array
        let items_arr = items.as_array().expect("items should be an array");
        assert!(!items_arr.is_empty(), "items array should not be empty");

        let item = &items_arr[0];
        assert_eq!(
            item.get("path_type").and_then(|v| v.as_str()),
            Some("workspace_dependencies"),
            "path_type should be 'workspace_dependencies'"
        );

        // Path should be "workspace-dependencies/python3/test-deps" or similar
        let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            path.contains("workspace-dependencies") || path.contains("python3"),
            "path should contain workspace-dependencies or language: got {}",
            path
        );
    } else if let Some(path_type) = args.get("path_type") {
        // Non-debounced format: path_type is a direct field
        assert_eq!(
            path_type.as_str(),
            Some("workspace_dependencies"),
            "path_type should be 'workspace_dependencies'"
        );
    } else {
        panic!(
            "Job args should contain either 'items' array or 'path_type' field: {:?}",
            args
        );
    }

    Ok(())
}

/// Test that archiving a workspace dependency triggers a git sync deployment callback
#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_archive_workspace_dependencies_triggers_git_sync(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    // Setup
    create_folder(&db, "28103").await?;
    create_git_repo_resource(&db).await?;
    let sync_script_path = "f/28103/test_sync_script_archive";
    create_sync_script(&db, sync_script_path).await?;
    setup_git_sync_config(&db, sync_script_path).await?;

    let (client, _port, _server) = init_client(db.clone()).await;

    // First create a workspace dependency
    let create_response = client
        .client()
        .post(format!(
            "{}/w/test-workspace/workspace_dependencies/create",
            client.baseurl()
        ))
        .json(&json!({
            "workspace_id": "test-workspace",
            "language": "python3",
            "name": "archive-test-deps",
            "content": "flask==2.0.0"
        }))
        .send()
        .await?;

    assert!(create_response.status().is_success());

    // Wait a bit for the create job to be processed
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Now archive it
    let archive_response = client
        .client()
        .post(format!(
            "{}/w/test-workspace/workspace_dependencies/archive/python3?name=archive-test-deps",
            client.baseurl()
        ))
        .send()
        .await?;

    assert!(
        archive_response.status().is_success(),
        "Failed to archive workspace dependency: {:?}",
        archive_response.text().await
    );

    // Wait for deployment callback job
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Should have at least 2 jobs (one for create, one for archive)
    let jobs = get_deployment_callback_jobs(&db, sync_script_path, Duration::from_secs(5)).await?;

    assert!(
        jobs.len() >= 2,
        "Expected at least 2 deployment callback jobs (create + archive), got {}",
        jobs.len()
    );

    Ok(())
}

/// Test that workspace dependencies are NOT synced when workspacedependencies type is excluded
#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_workspace_dependencies_respects_include_type_filter(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    // Setup with git sync that EXCLUDES workspacedependencies
    create_folder(&db, "28103").await?;
    create_git_repo_resource(&db).await?;
    let sync_script_path = "f/28103/test_sync_script_filter";
    create_sync_script(&db, sync_script_path).await?;

    // Configure git sync to only include scripts (not workspace dependencies)
    let git_sync_config = json!({
        "include_type": ["script"],  // Note: workspacedependencies is NOT included
        "include_path": ["**"],
        "repositories": [{
            "script_path": sync_script_path,
            "git_repo_resource_path": "$res:u/test-user/test_git_repo",
            "use_individual_branch": false,
            "group_by_folder": false
        }]
    });

    sqlx::query!(
        "UPDATE workspace_settings SET git_sync = $1 WHERE workspace_id = $2",
        git_sync_config,
        "test-workspace"
    )
    .execute(&db)
    .await?;

    let (client, _port, _server) = init_client(db.clone()).await;

    // Create workspace dependency
    let response = client
        .client()
        .post(format!(
            "{}/w/test-workspace/workspace_dependencies/create",
            client.baseurl()
        ))
        .json(&json!({
            "workspace_id": "test-workspace",
            "language": "python3",
            "name": "filtered-deps",
            "content": "django==4.0.0"
        }))
        .send()
        .await?;

    assert!(response.status().is_success());

    // Wait a bit
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Should NOT have any deployment callback jobs because workspacedependencies is filtered out
    let jobs =
        get_deployment_callback_jobs(&db, sync_script_path, Duration::from_millis(500)).await?;

    assert!(
        jobs.is_empty(),
        "Expected NO deployment callback jobs when workspacedependencies is not in include_type, got {}",
        jobs.len()
    );

    Ok(())
}

/// Test that the commit message is correctly generated for workspace dependencies
#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_workspace_dependencies_commit_message(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    // Setup
    create_folder(&db, "28103").await?;
    create_git_repo_resource(&db).await?;
    let sync_script_path = "f/28103/test_sync_script_msg";
    create_sync_script(&db, sync_script_path).await?;
    setup_git_sync_config(&db, sync_script_path).await?;

    let (client, _port, _server) = init_client(db.clone()).await;

    // Create workspace dependency
    let response = client
        .client()
        .post(format!(
            "{}/w/test-workspace/workspace_dependencies/create",
            client.baseurl()
        ))
        .json(&json!({
            "workspace_id": "test-workspace",
            "language": "bun",
            "name": null,  // unnamed/default dependency
            "content": "lodash: ^4.17.21"
        }))
        .send()
        .await?;

    assert!(response.status().is_success());

    tokio::time::sleep(Duration::from_millis(500)).await;

    let jobs = get_deployment_callback_jobs(&db, sync_script_path, Duration::from_secs(5)).await?;
    assert!(!jobs.is_empty());

    let job = &jobs[0];
    let args = job.args.as_ref().expect("Job should have args");

    // Check commit message format
    if let Some(items) = args.get("items") {
        let items_arr = items.as_array().expect("items should be an array");
        if !items_arr.is_empty() {
            let commit_msg = items_arr[0]
                .get("commit_msg")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            assert!(
                commit_msg.contains("[WM]"),
                "Commit message should contain '[WM]' prefix: {}",
                commit_msg
            );
            assert!(
                commit_msg.to_lowercase().contains("workspace")
                    || commit_msg.to_lowercase().contains("dependency")
                    || commit_msg.to_lowercase().contains("deployed"),
                "Commit message should mention workspace dependency or deployed: {}",
                commit_msg
            );
        }
    } else if let Some(commit_msg) = args.get("commit_msg").and_then(|v| v.as_str()) {
        assert!(
            commit_msg.contains("[WM]"),
            "Commit message should contain '[WM]' prefix: {}",
            commit_msg
        );
    }

    Ok(())
}
