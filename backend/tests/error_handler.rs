use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

/// Test that workspace error handler can be set and removed via database operations
#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_error_handler_settings(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let _server = ApiServer::start(db.clone()).await?;

    // Initially error_handler should be NULL
    let initial = sqlx::query_scalar!(
        r#"SELECT error_handler->>'path' FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
    )
    .fetch_one(&db)
    .await?;
    assert!(initial.is_none());

    // Set error handler with all options
    sqlx::query!(
        r#"
        UPDATE workspace_settings
        SET error_handler = '{"path": "script/f/test/error_handler", "extra_args": {"notify": true}, "muted_on_cancel": true, "muted_on_user_path": false}'::jsonb
        WHERE workspace_id = 'test-workspace'
        "#
    )
    .execute(&db)
    .await?;

    let after_set = sqlx::query_scalar!(
        r#"SELECT error_handler->>'path' FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(
        after_set,
        Some("script/f/test/error_handler".to_string())
    );

    // Verify extra_args
    let extra_args = sqlx::query_scalar!(
        r#"SELECT error_handler->'extra_args' FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
    )
    .fetch_one(&db)
    .await?;
    assert!(extra_args.is_some());

    // Verify muted_on_cancel
    let muted_on_cancel = sqlx::query_scalar!(
        r#"SELECT (error_handler->>'muted_on_cancel')::boolean FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(muted_on_cancel, Some(true));

    // Verify muted_on_user_path
    let muted_on_user_path = sqlx::query_scalar!(
        r#"SELECT (error_handler->>'muted_on_user_path')::boolean FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(muted_on_user_path, Some(false));

    // Remove error handler
    sqlx::query!(
        r#"
        UPDATE workspace_settings
        SET error_handler = NULL
        WHERE workspace_id = 'test-workspace'
        "#
    )
    .execute(&db)
    .await?;

    let after_remove = sqlx::query_scalar!(
        r#"SELECT error_handler->>'path' FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
    )
    .fetch_one(&db)
    .await?;
    assert!(after_remove.is_none());

    Ok(())
}

/// Test that a failed job triggers the workspace error handler
#[cfg(all(feature = "deno_core", feature = "enterprise", feature = "private"))]
#[sqlx::test(fixtures("base"))]
async fn test_error_handler_triggered_on_failure(db: Pool<Postgres>) -> anyhow::Result<()> {
    use windmill_common::jobs::JobPayload;
    use windmill_common::runnable_settings::{ConcurrencySettings, DebouncingSettings};
    use windmill_common::scripts::{ScriptHash, ScriptLang};

    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;

    // Create the error handler script
    let error_handler_code = r#"
export async function main(path: string, email: string, job_id: string, is_flow: boolean, workspace_id: string, error: any) {
    console.log("Error handler called for job:", job_id);
    return { handled: true, original_path: path };
}
"#;

    sqlx::query!(
        r#"
        INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock)
        VALUES ('test-workspace', 1111111111, 'f/test/error_handler', $1, 'deno', 'script', 'test-user', '{}', 'Error handler script', 'Handles failed job completions', '')
        "#,
        error_handler_code
    )
    .execute(&db)
    .await?;

    // Create a script that will fail
    let failing_script_code = "export function main() { throw new Error('intentional failure'); }";
    let failing_script_hash: i64 = 2222222222;

    sqlx::query!(
        r#"
        INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock)
        VALUES ('test-workspace', $1, 'f/test/failing_script', $2, 'deno', 'script', 'test-user', '{}', 'Failing test script', 'A script that always fails', '')
        "#,
        failing_script_hash,
        failing_script_code
    )
    .execute(&db)
    .await?;

    // Set up the error handler in workspace_settings
    sqlx::query!(
        r#"
        UPDATE workspace_settings
        SET error_handler = '{"path": "script/f/test/error_handler"}'::jsonb
        WHERE workspace_id = 'test-workspace'
        "#
    )
    .execute(&db)
    .await?;

    // Create the error_handler group
    sqlx::query!(
        r#"
        INSERT INTO group_ (workspace_id, name, summary, extra_perms)
        VALUES ('test-workspace', 'error_handler', 'The group the error handler acts on behalf of', '{"u/test-user": true}')
        ON CONFLICT DO NOTHING
        "#
    )
    .execute(&db)
    .await?;

    // Run the failing script
    let completed_job = RunJob::from(JobPayload::ScriptHash {
        hash: ScriptHash(failing_script_hash),
        path: "f/test/failing_script".to_string(),
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

    // Verify the job actually failed
    assert!(!completed_job.success, "Job should have failed");

    let main_job_id = completed_job.id;

    // Wait for the error handler job to be created
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify the error handler job was created
    let error_handler_job = sqlx::query!(
        r#"
        SELECT
            id,
            runnable_path,
            permissioned_as_email,
            parent_job
        FROM v2_job
        WHERE workspace_id = 'test-workspace'
            AND permissioned_as_email = 'error_handler@windmill.dev'
        ORDER BY created_at DESC
        LIMIT 1
        "#
    )
    .fetch_optional(&db)
    .await?;

    assert!(
        error_handler_job.is_some(),
        "Error handler job should have been created"
    );

    let handler_job = error_handler_job.unwrap();

    assert_eq!(
        handler_job.runnable_path.as_deref(),
        Some("f/test/error_handler"),
        "Error handler should run the configured script"
    );
    assert_eq!(
        handler_job.permissioned_as_email.as_str(),
        "error_handler@windmill.dev",
        "Error handler should run as error_handler user"
    );
    assert_eq!(
        handler_job.parent_job,
        Some(main_job_id),
        "Error handler should have the failed job as parent"
    );

    Ok(())
}

/// Test that error handler is NOT triggered when ws_error_handler_muted is set on the script
#[cfg(all(feature = "deno_core", feature = "enterprise", feature = "private"))]
#[sqlx::test(fixtures("base"))]
async fn test_error_handler_muted_on_script(db: Pool<Postgres>) -> anyhow::Result<()> {
    use windmill_common::jobs::JobPayload;
    use windmill_common::runnable_settings::{ConcurrencySettings, DebouncingSettings};
    use windmill_common::scripts::{ScriptHash, ScriptLang};

    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;

    // Create the error handler script
    sqlx::query!(
        r#"
        INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock)
        VALUES ('test-workspace', 3333333333, 'f/test/error_handler', 'export function main() { return "handled"; }', 'deno', 'script', 'test-user', '{}', '', '', '')
        "#,
    )
    .execute(&db)
    .await?;

    // Create a failing script with ws_error_handler_muted = true
    let failing_script_hash: i64 = 4444444444;
    sqlx::query!(
        r#"
        INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock, ws_error_handler_muted)
        VALUES ('test-workspace', $1, 'f/test/muted_failing_script', 'export function main() { throw new Error("fail"); }', 'deno', 'script', 'test-user', '{}', '', '', '', true)
        "#,
        failing_script_hash,
    )
    .execute(&db)
    .await?;

    // Set up the error handler
    sqlx::query!(
        r#"
        UPDATE workspace_settings
        SET error_handler = '{"path": "script/f/test/error_handler"}'::jsonb
        WHERE workspace_id = 'test-workspace'
        "#
    )
    .execute(&db)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO group_ (workspace_id, name, summary, extra_perms)
        VALUES ('test-workspace', 'error_handler', 'Error handler group', '{"u/test-user": true}')
        ON CONFLICT DO NOTHING
        "#
    )
    .execute(&db)
    .await?;

    // Run the muted failing script
    let completed_job = RunJob::from(JobPayload::ScriptHash {
        hash: ScriptHash(failing_script_hash),
        path: "f/test/muted_failing_script".to_string(),
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

    assert!(!completed_job.success, "Job should have failed");

    // Wait and check that NO error handler job was created
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let error_handler_job = sqlx::query_scalar!(
        r#"
        SELECT id
        FROM v2_job
        WHERE workspace_id = 'test-workspace'
            AND permissioned_as_email = 'error_handler@windmill.dev'
        "#
    )
    .fetch_optional(&db)
    .await?;

    assert!(
        error_handler_job.is_none(),
        "Error handler should NOT have been triggered for a muted script"
    );

    Ok(())
}

/// Test that error handler is NOT triggered on successful job completion
#[cfg(all(feature = "deno_core", feature = "enterprise", feature = "private"))]
#[sqlx::test(fixtures("base"))]
async fn test_error_handler_not_triggered_on_success(db: Pool<Postgres>) -> anyhow::Result<()> {
    use windmill_common::jobs::JobPayload;
    use windmill_common::runnable_settings::{ConcurrencySettings, DebouncingSettings};
    use windmill_common::scripts::{ScriptHash, ScriptLang};

    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;

    // Create the error handler script
    sqlx::query!(
        r#"
        INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock)
        VALUES ('test-workspace', 5555555555, 'f/test/error_handler', 'export function main() { return "handled"; }', 'deno', 'script', 'test-user', '{}', '', '', '')
        "#,
    )
    .execute(&db)
    .await?;

    // Create a successful script
    let success_script_hash: i64 = 6666666666;
    sqlx::query!(
        r#"
        INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock)
        VALUES ('test-workspace', $1, 'f/test/success_script', 'export function main() { return "ok"; }', 'deno', 'script', 'test-user', '{}', '', '', '')
        "#,
        success_script_hash,
    )
    .execute(&db)
    .await?;

    // Set up the error handler
    sqlx::query!(
        r#"
        UPDATE workspace_settings
        SET error_handler = '{"path": "script/f/test/error_handler"}'::jsonb
        WHERE workspace_id = 'test-workspace'
        "#
    )
    .execute(&db)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO group_ (workspace_id, name, summary, extra_perms)
        VALUES ('test-workspace', 'error_handler', 'Error handler group', '{"u/test-user": true}')
        ON CONFLICT DO NOTHING
        "#
    )
    .execute(&db)
    .await?;

    // Run the successful script
    let completed_job = RunJob::from(JobPayload::ScriptHash {
        hash: ScriptHash(success_script_hash),
        path: "f/test/success_script".to_string(),
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

    assert!(completed_job.success, "Job should have succeeded");

    // Wait and check that NO error handler job was created
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let error_handler_job = sqlx::query_scalar!(
        r#"
        SELECT id
        FROM v2_job
        WHERE workspace_id = 'test-workspace'
            AND permissioned_as_email = 'error_handler@windmill.dev'
        "#
    )
    .fetch_optional(&db)
    .await?;

    assert!(
        error_handler_job.is_none(),
        "Error handler should NOT have been triggered for a successful job"
    );

    Ok(())
}
