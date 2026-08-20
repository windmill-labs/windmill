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

#[allow(unused_imports)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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

/// Create a git repository resource at an arbitrary path.
#[allow(dead_code)]
async fn create_git_repo_resource_at(db: &Pool<Postgres>, path: &str) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO resource (workspace_id, path, value, resource_type, extra_perms, created_by)
        VALUES ('test-workspace', $2, $1::jsonb, 'git_repository', '{}'::jsonb, 'test-user')
        ON CONFLICT (workspace_id, path) DO NOTHING
        "#,
    )
    .bind(json!({
        "url": format!("https://github.com/test/{}.git", path.rsplit('/').next().unwrap_or("test")),
        "branch": "main",
        "token": "test-token"
    }))
    .bind(path)
    .execute(db)
    .await?;
    Ok(())
}

/// Create a git repository resource for testing
#[allow(dead_code)]
async fn create_git_repo_resource(db: &Pool<Postgres>) -> anyhow::Result<()> {
    create_git_repo_resource_at(db, "u/test-user/test_git_repo").await
}

/// Create a dummy sync script for testing (with version >= 28103 for debouncing support)
#[allow(dead_code)]
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
#[allow(dead_code)]
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

        // Path should be "dependencies/test-deps.requirements.in" or similar
        let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            path.contains("dependencies") || path.contains("requirements"),
            "path should contain dependencies or requirements: got {}",
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

    // Verify at least one deployment callback job exists
    // (create test already validates create triggers git sync; this test validates archive does too)
    let jobs = get_deployment_callback_jobs(&db, sync_script_path, Duration::from_secs(5)).await?;

    assert!(
        !jobs.is_empty(),
        "Expected at least one deployment callback job after archive"
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

// ============================================================================
// Promotion-mode debounce-key tests
// ============================================================================

/// Configure git sync in promotion mode (one branch per object), with explicit
/// include_type and include_path lists so script deploys fire the callback.
#[allow(dead_code)]
async fn setup_promotion_git_sync_config(
    db: &Pool<Postgres>,
    sync_script_path: &str,
    group_by_folder: bool,
) -> anyhow::Result<()> {
    let git_sync_config = json!({
        "include_type": ["script"],
        "include_path": ["**"],
        "repositories": [{
            "script_path": sync_script_path,
            "git_repo_resource_path": "$res:u/test-user/test_git_repo",
            "use_individual_branch": true,
            "group_by_folder": group_by_folder
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

/// Create a script via the API (triggering handle_deployment_metadata).
#[allow(dead_code)]
async fn create_test_script(
    client: &windmill_api_client::Client,
    path: &str,
) -> anyhow::Result<()> {
    let resp = client
        .client()
        .post(format!(
            "{}/w/test-workspace/scripts/create",
            client.baseurl()
        ))
        .json(&json!({
            "path": path,
            "summary": "",
            "description": "",
            // bash has no lock step, so handle_deployment_metadata runs
            "content": "echo hi",
            "language": "bash"
        }))
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    anyhow::ensure!(
        status.is_success(),
        "failed to create script {}: {} {}",
        path,
        status,
        body
    );
    Ok(())
}

/// Poll the `debounce_key` table until `expected` appears, or timeout.
#[allow(dead_code)]
async fn wait_for_debounce_key(
    db: &Pool<Postgres>,
    expected: &str,
    timeout: Duration,
) -> anyhow::Result<Vec<String>> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let keys: Vec<String> =
            sqlx::query_scalar!("SELECT key FROM debounce_key WHERE key LIKE 'git_sync:%'")
                .fetch_all(db)
                .await?;
        if keys.iter().any(|k| k == expected) {
            return Ok(keys);
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok(keys);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Promotion mode with one branch per object: the debounce key must scope to
/// (path_type, path) so rapid re-edits of the same script collapse while
/// different scripts run in parallel.
#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_promotion_individual_branch_debounces_per_path(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    create_folder(&db, "28103").await?;
    create_folder(&db, "target").await?;
    create_git_repo_resource(&db).await?;
    let sync_script_path = "f/28103/test_sync_individual_branch";
    create_sync_script(&db, sync_script_path).await?;
    setup_promotion_git_sync_config(&db, sync_script_path, false).await?;

    let (client, _port, _server) = init_client(db.clone()).await;

    create_test_script(&client, "f/target/alpha").await?;
    create_test_script(&client, "f/target/beta").await?;

    // Wait for both deployment callbacks to resolve a debounce key. The
    // alpha key is polled first as a warm-up, then we also wait for beta
    // so the assertions don't race the second spawned callback.
    // Keys are namespaced by the repo's resource path so multiple promotion
    // repos don't collide on the same key.
    let expected_alpha = "git_sync:$res:u/test-user/test_git_repo:script:f/target/alpha";
    let expected_beta = "git_sync:$res:u/test-user/test_git_repo:script:f/target/beta";
    let _ = wait_for_debounce_key(&db, expected_alpha, Duration::from_secs(5)).await?;
    let keys = wait_for_debounce_key(&db, expected_beta, Duration::from_secs(5)).await?;
    assert!(
        keys.iter().any(|k| k == expected_alpha),
        "expected per-path debounce key {expected_alpha} in {keys:?}"
    );
    assert!(
        keys.iter().any(|k| k == expected_beta),
        "expected per-path debounce key {expected_beta} in {keys:?}"
    );

    Ok(())
}

/// Poll until `expected` callbacks for `script_path` are queued, or time out.
#[allow(dead_code)]
async fn wait_for_callback_jobs(
    db: &Pool<Postgres>,
    script_path: &str,
    expected: usize,
    timeout: Duration,
) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let jobs =
            get_deployment_callback_jobs(db, script_path, Duration::from_millis(200)).await?;
        if jobs.len() >= expected {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!(
                "timed out waiting for {expected} deployment callbacks on {script_path}, got {}",
                jobs.len()
            );
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// The concurrency key each queued deployment callback was pushed with, read
/// back through the runnable-settings handle the push stored it under.
#[allow(dead_code)]
async fn get_concurrency_keys(
    db: &Pool<Postgres>,
    script_path: &str,
) -> anyhow::Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT COALESCE(cs.concurrency_key, '<no concurrency settings row>')
        FROM v2_job j
        JOIN v2_job_queue q ON q.id = j.id
        LEFT JOIN runnable_settings rs ON rs.hash = q.runnable_settings_handle
        LEFT JOIN concurrency_settings cs ON cs.hash = rs.concurrency_settings
        WHERE j.runnable_path = $1 AND j.kind = 'deploymentcallback'
        "#,
    )
    .bind(script_path)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|(k,)| k).collect())
}

/// Create a second git repository resource for multi-repo tests.
#[allow(dead_code)]
async fn create_second_git_repo_resource(db: &Pool<Postgres>) -> anyhow::Result<()> {
    create_git_repo_resource_at(db, "u/test-user/test_git_repo_2").await
}

/// Configure git sync with TWO promotion-mode repositories pointing at distinct
/// git repo resources. Both repos use the same sync script and the same item
/// filters — they only differ in the repo they target.
#[allow(dead_code)]
async fn setup_two_promotion_repos_config(
    db: &Pool<Postgres>,
    sync_script_path: &str,
    group_by_folder: bool,
) -> anyhow::Result<()> {
    let git_sync_config = json!({
        "include_type": ["script"],
        "include_path": ["**"],
        "repositories": [
            {
                "script_path": sync_script_path,
                "git_repo_resource_path": "$res:u/test-user/test_git_repo",
                "use_individual_branch": true,
                "group_by_folder": group_by_folder
            },
            {
                "script_path": sync_script_path,
                "git_repo_resource_path": "$res:u/test-user/test_git_repo_2",
                "use_individual_branch": true,
                "group_by_folder": group_by_folder
            }
        ]
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

/// Regression test for: when two promotion-mode repos are configured with the
/// same `use_individual_branch=true` settings (i.e. a primary and a secondary
/// promotion repo), deploying a single script must enqueue ONE deployment
/// callback per repo. Both callbacks must remain in the queue — neither may
/// be debounced into oblivion by the other.
///
/// The bug this guards against: the debounce key for promotion mode was
/// derived only from (path_type, path) and omitted any per-repo identifier,
/// so the second repo's push hit ON CONFLICT in `upsert_debounce_key` and
/// `complete_debounced_job` flagged the first repo's job as `status='skipped'`
/// — silently dropping one of the two pushes.
#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_two_promotion_repos_both_enqueue_callback(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    create_folder(&db, "28103").await?;
    create_folder(&db, "target").await?;
    create_git_repo_resource(&db).await?;
    create_second_git_repo_resource(&db).await?;
    let sync_script_path = "f/28103/test_sync_two_promotion_repos";
    create_sync_script(&db, sync_script_path).await?;
    setup_two_promotion_repos_config(&db, sync_script_path, false).await?;

    let (client, _port, _server) = init_client(db.clone()).await;

    // Deploy a single script — handle_deployment_metadata should iterate
    // both repos and create one callback job per repo.
    create_test_script(&client, "f/target/alpha").await?;

    // Both callbacks should reach the queue. With the bug, only one survives
    // (the other is moved to v2_job_completed with status='skipped').
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut last_jobs: Vec<DeploymentCallbackJob> = vec![];
    loop {
        last_jobs =
            get_deployment_callback_jobs(&db, sync_script_path, Duration::from_millis(200)).await?;
        if last_jobs.len() >= 2 {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Inspect what landed in v2_job_completed so failure messages explain why.
    let skipped: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        r#"
        SELECT c.id, c.status::text
        FROM v2_job_completed c
        JOIN v2_job j ON j.id = c.id
        WHERE j.runnable_path = $1 AND j.kind = 'deploymentcallback'
        "#,
    )
    .bind(sync_script_path)
    .fetch_all(&db)
    .await?;

    assert_eq!(
        last_jobs.len(),
        2,
        "expected 2 deployment callback jobs in v2_job_queue (one per promotion repo), got {} queued + {:?} completed",
        last_jobs.len(),
        skipped,
    );

    // Per-repo args sanity check: the two jobs must target different repos.
    let mut repo_paths: Vec<String> = last_jobs
        .iter()
        .filter_map(|j| {
            j.args
                .as_ref()
                .and_then(|a| a.get("repo_url_resource_path"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    repo_paths.sort();
    repo_paths.dedup();
    assert_eq!(
        repo_paths.len(),
        2,
        "expected callbacks to target two distinct repos, got: {:?}",
        last_jobs.iter().map(|j| &j.args).collect::<Vec<_>>()
    );

    // No callback should have been silently skipped via debouncing collision.
    assert!(
        skipped.iter().all(|(_, s)| s != "skipped"),
        "no deployment callback should be marked skipped, got: {:?}",
        skipped,
    );

    Ok(())
}

/// Two repositories, first in workspace-wide mode and then in promotion mode:
/// each repo's sync must get its own concurrency lane in both. Sharing
/// `{workspace}:git_sync` serialises unrelated remotes, and a concurrency-limited
/// job is re-queued to an estimate derived from the key's average duration with no
/// wake-up when the slot frees, so one slow repo delays every other repo by
/// multiples of its own runtime.
#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_two_repos_get_distinct_concurrency_lanes(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    create_folder(&db, "28103").await?;
    create_folder(&db, "target").await?;
    // Concurrency settings are written through a filesystem-backed cache keyed by
    // their own hash: a hash the cache has seen is assumed to be in the database
    // already, so a key any earlier run inserted never lands in this test's fresh
    // database and `get_concurrency_keys` cannot read it back. Repo paths unique to
    // the run keep every key new.
    let run_id: u32 = rand::random();
    let repo_a = format!("u/test-user/lane_repo_a_{run_id}");
    let repo_b = format!("u/test-user/lane_repo_b_{run_id}");
    create_git_repo_resource_at(&db, &repo_a).await?;
    create_git_repo_resource_at(&db, &repo_b).await?;
    let sync_script_path = "f/28103/test_sync_two_workspace_wide_repos";
    create_sync_script(&db, sync_script_path).await?;

    let git_sync_config = json!({
        "include_type": ["script"],
        "include_path": ["**"],
        "repositories": [
            {
                "script_path": sync_script_path,
                "git_repo_resource_path": format!("$res:{repo_a}"),
                "use_individual_branch": false,
                "group_by_folder": false
            },
            {
                "script_path": sync_script_path,
                "git_repo_resource_path": format!("$res:{repo_b}"),
                "use_individual_branch": false,
                "group_by_folder": false
            }
        ]
    });
    sqlx::query!(
        "UPDATE workspace_settings SET git_sync = $1 WHERE workspace_id = $2",
        git_sync_config,
        "test-workspace"
    )
    .execute(&db)
    .await?;

    let (client, _port, _server) = init_client(db.clone()).await;

    create_test_script(&client, "f/target/alpha").await?;
    wait_for_callback_jobs(&db, sync_script_path, 2, Duration::from_secs(5)).await?;

    let mut conc_keys = get_concurrency_keys(&db, sync_script_path).await?;
    conc_keys.sort();
    assert_eq!(
        conc_keys,
        vec![
            format!("test-workspace:git_sync:{repo_a}"),
            format!("test-workspace:git_sync:{repo_b}"),
        ],
        "each repo must push on its own concurrency lane"
    );

    // Promotion mode keys per branch, but two repos deploying the same object name
    // the same branch, so the repo has to be in the key there too.
    let promo_script_path = "f/28103/test_sync_two_promotion_lanes";
    create_sync_script(&db, promo_script_path).await?;
    let promo_config = json!({
        "include_type": ["script"],
        "include_path": ["**"],
        "repositories": [
            {
                "script_path": promo_script_path,
                "git_repo_resource_path": format!("$res:{repo_a}"),
                "use_individual_branch": true,
                "group_by_folder": false
            },
            {
                "script_path": promo_script_path,
                "git_repo_resource_path": format!("$res:{repo_b}"),
                "use_individual_branch": true,
                "group_by_folder": false
            }
        ]
    });
    sqlx::query!(
        "UPDATE workspace_settings SET git_sync = $1 WHERE workspace_id = $2",
        promo_config,
        "test-workspace"
    )
    .execute(&db)
    .await?;

    create_test_script(&client, "f/target/beta").await?;
    wait_for_callback_jobs(&db, promo_script_path, 2, Duration::from_secs(5)).await?;

    let mut promo_keys = get_concurrency_keys(&db, promo_script_path).await?;
    promo_keys.sort();
    assert_eq!(
        promo_keys,
        vec![
            format!("test-workspace:git_sync:{repo_a}:script:f/target/beta"),
            format!("test-workspace:git_sync:{repo_b}:script:f/target/beta"),
        ],
        "each repo must push its branch on its own concurrency lane"
    );

    Ok(())
}

/// Pulls must NOT follow pushes into a per-repo lane. A pull writes workspace
/// objects, so the workspace-wide lane is what stops two repos applying creates,
/// updates and deletes to the same scripts and flows at once — the safety argument
/// for per-repo push lanes rests on this staying put.
#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_pull_stays_on_the_workspace_lane(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    create_git_repo_resource_at(&db, "u/test-user/pull_repo").await?;

    let repo: windmill_common::workspaces::GitRepositorySettings = serde_json::from_value(json!({
        "git_repo_resource_path": "$res:u/test-user/pull_repo",
        "use_individual_branch": false,
        "group_by_folder": false
    }))?;

    windmill_git_sync::enqueue_git_pull_job(
        &db,
        "test-workspace",
        &repo,
        None,
        false,
        None,
        None,
        None,
    )
    .await?;

    let keys: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT ck.key
        FROM v2_job j
        JOIN concurrency_key ck ON ck.job_id = j.id
        WHERE j.workspace_id = 'test-workspace' AND j.kind = 'deploymentcallback'
        "#,
    )
    .fetch_all(&db)
    .await?;
    assert_eq!(
        keys.iter().map(|(k,)| k.as_str()).collect::<Vec<_>>(),
        vec!["test-workspace:git_sync"],
        "a pull must share the workspace-wide lane with every other repo's pull"
    );

    Ok(())
}

/// Putting the repo in the lane made the key long enough to matter:
/// `concurrency_key.key` is VARCHAR(255) and its INSERT runs inside `push`, so an
/// overflowing key fails the push and the sync job is never created at all. A repo
/// path that does not fit must be hashed down instead.
///
/// A cloud build narrows the budget further — `resolve_concurrency_key` prepends
/// `{workspace_id}/` there — but `cloud` is a separate cargo feature this test
/// binary does not enable, so this covers the un-prefixed budget only.
#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_long_repo_path_still_enqueues_callback(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    create_folder(&db, "28103").await?;
    create_folder(&db, "target").await?;
    // Long enough that `{workspace}:git_sync:{repo}` alone exceeds 255.
    let long_repo = format!("u/test-user/{}", "x".repeat(228));
    create_git_repo_resource_at(&db, &long_repo).await?;
    let sync_script_path = "f/28103/test_sync_long_repo_path";
    create_sync_script(&db, sync_script_path).await?;

    let git_sync_config = json!({
        "include_type": ["script"],
        "include_path": ["**"],
        "repositories": [{
            "script_path": sync_script_path,
            "git_repo_resource_path": format!("$res:{long_repo}"),
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

    create_test_script(&client, "f/target/alpha").await?;
    wait_for_callback_jobs(&db, sync_script_path, 1, Duration::from_secs(5)).await?;

    // The row exists only if the INSERT inside `push` accepted the key.
    let keys: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT ck.key
        FROM v2_job j
        JOIN concurrency_key ck ON ck.job_id = j.id
        WHERE j.runnable_path = $1 AND j.kind = 'deploymentcallback'
        "#,
    )
    .bind(sync_script_path)
    .fetch_all(&db)
    .await?;
    assert_eq!(
        keys.len(),
        1,
        "expected a stored concurrency key, got {keys:?}"
    );
    assert!(
        keys[0].0.len() <= 255,
        "concurrency key must fit the column: {} chars",
        keys[0].0.len()
    );

    Ok(())
}

/// Promotion mode with group_by_folder: items destined for the same per-folder
/// branch must share one debounce key so they accumulate into a single sync
/// job; scripts in different folders must get distinct keys.
#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_promotion_group_by_folder_debounces_per_folder(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    create_folder(&db, "28103").await?;
    create_folder(&db, "grouped").await?;
    create_folder(&db, "other").await?;
    create_git_repo_resource(&db).await?;
    let sync_script_path = "f/28103/test_sync_group_by_folder";
    create_sync_script(&db, sync_script_path).await?;
    setup_promotion_git_sync_config(&db, sync_script_path, true).await?;

    let (client, _port, _server) = init_client(db.clone()).await;

    // Two scripts in the same folder — should share one debounce key.
    create_test_script(&client, "f/grouped/alpha").await?;
    create_test_script(&client, "f/grouped/beta").await?;
    // One in a different folder — should get its own key.
    create_test_script(&client, "f/other/gamma").await?;

    // Keys are namespaced by the repo's resource path.
    let expected_grouped = "git_sync:$res:u/test-user/test_git_repo:folder:f/grouped";
    let expected_other = "git_sync:$res:u/test-user/test_git_repo:folder:f/other";
    // Wait for BOTH folder keys to appear, not just the first one.
    let keys = wait_for_debounce_key(&db, expected_other, Duration::from_secs(5)).await?;
    assert!(
        keys.iter().any(|k| k == expected_grouped),
        "expected per-folder debounce key {expected_grouped} in {keys:?}"
    );
    assert!(
        keys.iter().any(|k| k == expected_other),
        "expected per-folder debounce key {expected_other} in {keys:?}"
    );
    // Paths within the same folder must NOT leak as their own keys.
    assert!(
        !keys
            .iter()
            .any(|k| k.contains(":script:f/grouped/") || k.contains(":script:f/other/")),
        "group_by_folder mode should not emit per-path keys, got: {keys:?}"
    );

    Ok(())
}
