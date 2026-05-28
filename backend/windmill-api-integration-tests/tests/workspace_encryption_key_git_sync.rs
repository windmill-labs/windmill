/*!
 * Integration test for workspace encryption key rotation triggering git sync.
 *
 * Regression test for windmill-labs/windmill#9344 — re-encrypting all secret
 * variables on workspace key change must dispatch a git-sync job that carries
 * every re-encrypted variable plus the encryption_key entry, so repos with
 * Secrets sync enabled receive the new ciphertexts in one commit.
 *
 * Run with enterprise features:
 * ```bash
 * cargo test --test workspace_encryption_key_git_sync --features enterprise,private
 * ```
 */

use serde_json::json;
use sqlx::{Pool, Postgres};
use std::time::Duration;

#[allow(unused_imports)]
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

#[allow(dead_code)]
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

#[allow(dead_code)]
async fn setup_git_sync_config(db: &Pool<Postgres>, sync_script_path: &str) -> anyhow::Result<()> {
    // Include Variable + Secret + Key so the encryption rotation has a reason
    // to push every re-encrypted variable. Anchor include_path to root so all
    // u/... and f/... paths pass the regex filter.
    let git_sync_config = json!({
        "include_type": ["variable", "secret", "key"],
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

/// Insert N secret variables, encrypting their values with the workspace's
/// current key so the re-encryption path can decrypt them.
#[allow(dead_code)]
async fn insert_secret_variables(db: &Pool<Postgres>, paths: &[&str]) -> anyhow::Result<()> {
    use windmill_common::variables::{build_crypt, encrypt};
    let mc = build_crypt(db, "test-workspace").await?;
    for path in paths {
        let plaintext = format!("secret-value-for-{path}");
        let encrypted = encrypt(&mc, &plaintext);
        sqlx::query!(
            r#"
            INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms, account)
            VALUES ($1, $2, $3, true, '', '{}'::jsonb, NULL)
            ON CONFLICT (workspace_id, path) DO UPDATE SET value = EXCLUDED.value
            "#,
            "test-workspace",
            path,
            encrypted,
        )
        .execute(db)
        .await?;
    }
    Ok(())
}

#[derive(Debug)]
#[allow(dead_code)]
struct DeploymentCallbackJob {
    id: uuid::Uuid,
    args: Option<serde_json::Value>,
}

/// Poll until at least `min_count` deployment-callback jobs exist for the
/// script path, or the timeout elapses. Returns whatever was found.
#[allow(dead_code)]
async fn wait_for_deployment_callbacks(
    db: &Pool<Postgres>,
    script_path: &str,
    min_count: usize,
    timeout: Duration,
) -> anyhow::Result<Vec<DeploymentCallbackJob>> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let rows = sqlx::query_as!(
            DeploymentCallbackJob,
            r#"
            SELECT j.id, j.args
            FROM v2_job j
            JOIN v2_job_queue q ON j.id = q.id
            WHERE j.runnable_path = $1
              AND j.kind = 'deploymentcallback'
              AND j.workspace_id = 'test-workspace'
            ORDER BY j.created_at DESC
            "#,
            script_path,
        )
        .fetch_all(db)
        .await?;
        if rows.len() >= min_count || tokio::time::Instant::now() >= deadline {
            return Ok(rows);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_encryption_key_rotation_dispatches_batched_git_sync(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    // Setup git sync repo + sync script (folder/path encodes the hub min-version)
    create_folder(&db, "28103").await?;
    create_git_repo_resource(&db).await?;
    let sync_script_path = "f/28103/test_sync_script_encryption";
    create_sync_script(&db, sync_script_path).await?;
    setup_git_sync_config(&db, sync_script_path).await?;

    let secret_paths = [
        "u/test-user/secret_a",
        "u/test-user/secret_b",
        "u/test-user/secret_c",
    ];
    insert_secret_variables(&db, &secret_paths).await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

    // 64-char alphanumeric per the route's WORKSPACE_KEY_REGEXP
    let new_key = "a".repeat(64);
    let resp = authed(client().post(format!("{base}/encryption_key")))
        .json(&json!({"new_key": new_key, "skip_reencrypt": false}))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        200,
        "set_encryption_key failed: {}",
        resp.text().await?
    );

    // The git-sync dispatch runs in a tokio::spawn'd task. Poll up to a few
    // seconds for the deployment callback to land in the queue.
    let jobs =
        wait_for_deployment_callbacks(&db, sync_script_path, 1, Duration::from_secs(5)).await?;
    assert_eq!(
        jobs.len(),
        1,
        "expected exactly one batched deployment callback job, got {}",
        jobs.len()
    );
    let job = &jobs[0];

    let args = job.args.as_ref().expect("job should have args");
    let items = args
        .get("items")
        .and_then(|v| v.as_array())
        .expect("args.items should be a JSON array");

    // Expect exactly one job carrying the Key entry + every re-encrypted variable
    assert_eq!(
        items.len(),
        secret_paths.len() + 1,
        "expected {} items (key + {} variables) in a single sync job, got {} — items: {:#?}",
        secret_paths.len() + 1,
        secret_paths.len(),
        items.len(),
        items
    );

    let mut variable_paths: Vec<String> = Vec::new();
    let mut saw_key = false;
    for item in items {
        let path_type = item.get("path_type").and_then(|v| v.as_str()).unwrap_or("");
        let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("");
        match path_type {
            "variable" => variable_paths.push(path.to_string()),
            "key" => saw_key = true,
            other => panic!("unexpected path_type in batch: {other}"),
        }
    }
    assert!(
        saw_key,
        "expected a path_type=key entry in items: {:#?}",
        items
    );
    variable_paths.sort();
    let mut expected: Vec<String> = secret_paths.iter().map(|s| s.to_string()).collect();
    expected.sort();
    assert_eq!(
        variable_paths, expected,
        "items array should contain every re-encrypted secret variable"
    );

    // Secrets sync is enabled (ObjectType::Secret in include_type), so the sync
    // script should be invoked with skip_secret=false.
    let skip_secret = args
        .get("skip_secret")
        .and_then(|v| v.as_bool())
        .expect("args.skip_secret should be set when batch carries variables");
    assert!(
        !skip_secret,
        "skip_secret should be false when Secret is included in the repo's types"
    );

    Ok(())
}

/// Regression test for the non-debouncing fallback: a workspace whose sync
/// script predates hub version 28103 must still receive git-sync jobs for the
/// encryption_key entry and every re-encrypted secret. Before the fallback was
/// added, the batch path `continue`d past such repos and queued nothing,
/// silently leaving the repo stale after a key rotation.
#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_encryption_key_rotation_falls_back_without_debouncing(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    // Folder/path encodes a hub version BELOW 28103, so
    // is_script_meets_min_version(28103) is false → debouncing unsupported.
    create_folder(&db, "28000").await?;
    create_git_repo_resource(&db).await?;
    let sync_script_path = "f/28000/test_sync_script_legacy";
    create_sync_script(&db, sync_script_path).await?;
    setup_git_sync_config(&db, sync_script_path).await?;

    let secret_paths = ["u/test-user/secret_a", "u/test-user/secret_b"];
    insert_secret_variables(&db, &secret_paths).await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

    let new_key = "b".repeat(64);
    let resp = authed(client().post(format!("{base}/encryption_key")))
        .json(&json!({"new_key": new_key, "skip_reencrypt": false}))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        200,
        "set_encryption_key failed: {}",
        resp.text().await?
    );

    // Legacy fallback pushes one job per item (flat args, no `items` array):
    // the Key entry + one per re-encrypted variable.
    let expected = secret_paths.len() + 1;
    let jobs =
        wait_for_deployment_callbacks(&db, sync_script_path, expected, Duration::from_secs(5))
            .await?;
    assert_eq!(
        jobs.len(),
        expected,
        "expected {expected} legacy deployment-callback jobs (key + {} variables), got {} — a repo on an old sync script must not be silently skipped",
        secret_paths.len(),
        jobs.len()
    );

    let mut variable_paths: Vec<String> = Vec::new();
    let mut saw_key = false;
    for job in &jobs {
        let args = job.args.as_ref().expect("job should have args");
        // Legacy format: flat fields, never an `items` array.
        assert!(
            args.get("items").is_none(),
            "fallback jobs must use the flat legacy format, not an items array: {args:#?}"
        );
        let path_type = args.get("path_type").and_then(|v| v.as_str()).unwrap_or("");
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
        match path_type {
            "variable" => variable_paths.push(path.to_string()),
            "key" => saw_key = true,
            other => panic!("unexpected path_type in fallback job: {other}"),
        }
    }
    assert!(saw_key, "expected a path_type=key fallback job");
    variable_paths.sort();
    let mut expected_paths: Vec<String> = secret_paths.iter().map(|s| s.to_string()).collect();
    expected_paths.sort();
    assert_eq!(
        variable_paths, expected_paths,
        "fallback must queue a job for every re-encrypted secret variable"
    );

    Ok(())
}
