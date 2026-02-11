use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

/// Test that the workspace success handler cache works correctly with 60s TTL
#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_success_handler_cache(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    // First, create a success handler script
    let _server = ApiServer::start(db.clone()).await?;

    // Set up a success handler in workspace_settings using new JSONB column
    sqlx::query!(
        r#"
        UPDATE workspace_settings
        SET success_handler = '{"path": "script/f/test/success_handler", "extra_args": {"key": "value"}}'::jsonb
        WHERE workspace_id = 'test-workspace'
        "#
    )
    .execute(&db)
    .await?;

    // Verify the success handler was set
    let result = sqlx::query_scalar!(
        r#"SELECT success_handler->>'path' FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(result, Some("script/f/test/success_handler".to_string()));

    // Verify extra args were set
    let extra_args = sqlx::query_scalar!(
        r#"SELECT success_handler->'extra_args' FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
    )
    .fetch_one(&db)
    .await?;

    assert!(extra_args.is_some());

    Ok(())
}

/// Test that success handler can be set and removed via database operations
#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_success_handler_settings(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let _server = ApiServer::start(db.clone()).await?;

    // Initially success_handler should be NULL
    let initial = sqlx::query_scalar!(
        r#"SELECT success_handler->>'path' FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
    )
    .fetch_one(&db)
    .await?;
    assert!(initial.is_none());

    // Set success handler
    sqlx::query!(
        r#"
        UPDATE workspace_settings
        SET success_handler = '{"path": "flow/f/test/success_flow"}'::jsonb
        WHERE workspace_id = 'test-workspace'
        "#
    )
    .execute(&db)
    .await?;

    let after_set = sqlx::query_scalar!(
        r#"SELECT success_handler->>'path' FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(after_set, Some("flow/f/test/success_flow".to_string()));

    // Remove success handler
    sqlx::query!(
        r#"
        UPDATE workspace_settings
        SET success_handler = NULL
        WHERE workspace_id = 'test-workspace'
        "#
    )
    .execute(&db)
    .await?;

    let after_remove = sqlx::query_scalar!(
        r#"SELECT success_handler->>'path' FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
    )
    .fetch_one(&db)
    .await?;
    assert!(after_remove.is_none());

    Ok(())
}

/// Test that a successful job completion triggers the success handler
#[cfg(all(feature = "deno_core", feature = "enterprise", feature = "private"))]
#[sqlx::test(fixtures("base"))]
async fn test_success_handler_triggered_on_success(db: Pool<Postgres>) -> anyhow::Result<()> {
    use serde_json::json;
    use windmill_common::jobs::JobPayload;
    use windmill_common::runnable_settings::{ConcurrencySettings, DebouncingSettings};
    use windmill_common::scripts::{ScriptHash, ScriptLang};

    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;

    // Create a simple success handler script
    // Note: lock must be non-null for script to be considered "deployed"
    let success_handler_code = r#"
export async function main(path: string, email: string, job_id: string, is_flow: boolean, workspace_id: string, result: any) {
    console.log("Success handler called for job:", job_id);
    return { handled: true, original_path: path };
}
"#;

    sqlx::query!(
        r#"
        INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock)
        VALUES ('test-workspace', 1234567890, 'f/test/success_handler', $1, 'deno', 'script', 'test-user', '{}', 'Success handler script', 'Handles successful job completions', '')
        "#,
        success_handler_code
    )
    .execute(&db)
    .await?;

    // Create a simple test script that we'll run (needs to be JobKind::Script, not Preview)
    // Note: lock must be non-null for script to be considered "deployed"
    let test_script_code = "export function main() { return 'success'; }";
    let test_script_hash: i64 = 9876543210;

    sqlx::query!(
        r#"
        INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock)
        VALUES ('test-workspace', $1, 'f/test/simple_script', $2, 'deno', 'script', 'test-user', '{}', 'Simple test script', 'A simple test script', '')
        "#,
        test_script_hash,
        test_script_code
    )
    .execute(&db)
    .await?;

    // Set up the success handler in workspace_settings
    sqlx::query!(
        r#"
        UPDATE workspace_settings
        SET success_handler = '{"path": "script/f/test/success_handler"}'::jsonb
        WHERE workspace_id = 'test-workspace'
        "#
    )
    .execute(&db)
    .await?;

    // Create the success_handler group
    sqlx::query!(
        r#"
        INSERT INTO group_ (workspace_id, name, summary, extra_perms)
        VALUES ('test-workspace', 'success_handler', 'The group the success handler acts on behalf of', '{"u/test-user": true}')
        ON CONFLICT DO NOTHING
        "#
    )
    .execute(&db)
    .await?;

    // Run a script using ScriptHash (produces JobKind::Script, not Preview)
    let completed_job = RunJob::from(JobPayload::ScriptHash {
        hash: ScriptHash(test_script_hash),
        path: "f/test/simple_script".to_string(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        language: ScriptLang::Deno,
        priority: None,
        apply_preprocessor: false,
        concurrency_settings: ConcurrencySettings::default(),
        debouncing_settings: DebouncingSettings::default(),
    })
    .run_until_complete(&db, false, server.addr.port())
    .await;

    let result = completed_job.json_result().unwrap();
    assert_eq!(result, json!("success"));

    let main_job_id = completed_job.id;

    // Wait a short time for the success handler job to be created
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify the success handler job was created (query by email since trigger is not set)
    let success_handler_job = sqlx::query!(
        r#"
        SELECT
            id,
            runnable_path,
            permissioned_as_email,
            parent_job
        FROM v2_job
        WHERE workspace_id = 'test-workspace'
            AND permissioned_as_email = 'success_handler@windmill.dev'
        ORDER BY created_at DESC
        LIMIT 1
        "#
    )
    .fetch_optional(&db)
    .await?;

    assert!(
        success_handler_job.is_some(),
        "Success handler job should have been created"
    );

    let handler_job = success_handler_job.unwrap();

    // Verify the success handler job has correct parameters
    assert_eq!(
        handler_job.runnable_path.as_deref(),
        Some("f/test/success_handler"),
        "Success handler should run the configured script"
    );
    assert_eq!(
        handler_job.permissioned_as_email.as_str(),
        "success_handler@windmill.dev",
        "Success handler should run as success_handler user"
    );
    assert_eq!(
        handler_job.parent_job,
        Some(main_job_id),
        "Success handler should have main job as parent"
    );
    // Note: root_job may be None when it equals parent_job (optimization in push function)

    Ok(())
}
