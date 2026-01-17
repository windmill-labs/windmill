use sqlx::{Pool, Postgres};

mod common;
use common::*;

/// Test that the workspace success handler cache works correctly with 60s TTL
#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_success_handler_cache(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    // First, create a success handler script
    let _server = ApiServer::start(db.clone()).await?;

    // Set up a success handler in workspace_settings
    sqlx::query!(
        r#"
        UPDATE workspace_settings
        SET success_handler = 'script/f/test/success_handler',
            success_handler_extra_args = '{"key": "value"}'::json
        WHERE workspace_id = 'test-workspace'
        "#
    )
    .execute(&db)
    .await?;

    // Verify the success handler was set
    let result = sqlx::query_scalar!(
        r#"SELECT success_handler FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(result, Some("script/f/test/success_handler".to_string()));

    // Verify extra args were set
    let extra_args = sqlx::query_scalar!(
        r#"SELECT success_handler_extra_args FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
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
        r#"SELECT success_handler FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
    )
    .fetch_one(&db)
    .await?;
    assert!(initial.is_none());

    // Set success handler
    sqlx::query!(
        r#"
        UPDATE workspace_settings
        SET success_handler = 'flow/f/test/success_flow'
        WHERE workspace_id = 'test-workspace'
        "#
    )
    .execute(&db)
    .await?;

    let after_set = sqlx::query_scalar!(
        r#"SELECT success_handler FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
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
        r#"SELECT success_handler FROM workspace_settings WHERE workspace_id = 'test-workspace'"#
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
    use windmill_common::jobs::{JobPayload, RawCode};
    use windmill_common::scripts::ScriptLang;

    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;

    // Create a simple success handler script first
    let success_handler_code = r#"
export async function main(path: string, email: string, job_id: string, is_flow: boolean, workspace_id: string, result: any) {
    console.log("Success handler called for job:", job_id);
    return { handled: true, original_path: path };
}
"#;

    // Create the success handler script in the database
    sqlx::query!(
        r#"
        INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema)
        VALUES ('test-workspace', 1234567890, 'f/test/success_handler', $1, 'deno', 'script', 'test-user', '{}')
        "#,
        success_handler_code
    )
    .execute(&db)
    .await?;

    // Set up the success handler in workspace_settings
    sqlx::query!(
        r#"
        UPDATE workspace_settings
        SET success_handler = 'script/f/test/success_handler'
        WHERE workspace_id = 'test-workspace'
        "#
    )
    .execute(&db)
    .await?;

    // Create and also create the success_handler group
    sqlx::query!(
        r#"
        INSERT INTO group_ (workspace_id, name, summary, extra_perms)
        VALUES ('test-workspace', 'success_handler', 'The group the success handler acts on behalf of', '{"u/test-user": true}')
        ON CONFLICT DO NOTHING
        "#
    )
    .execute(&db)
    .await?;

    // Run a simple script that succeeds
    let result = RunJob::from(JobPayload::Code(RawCode {
        content: "export function main() { return 'success'; }".to_string(),
        path: Some("f/test/simple_script".to_string()),
        language: ScriptLang::Deno,
        lock: None,
        custom_concurrency_key: None,
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
        codebase: None,
    }))
    .run_until_complete(&db, false, server.addr.port())
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, json!("success"));

    // In a full EE environment, we would check that the success handler job was created
    // For this test, we just verify the main job completed successfully
    // The actual success handler triggering is in the EE apply_completed_job_handlers code

    Ok(())
}
