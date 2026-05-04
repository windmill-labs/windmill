use sqlx::postgres::Postgres;
use sqlx::Pool;
use windmill_test_utils::{initialize_tracing, ApiServer};

/// Integration test: exercises every explicit-column query in `tarball_workspace`.
///
/// Creates one entity of each type (folder, script, resource, resource_type,
/// variable, schedule, group) in the test workspace, then calls the tarball
/// export endpoint with all include_* flags enabled.  Success means every
/// `SELECT col1, col2, ...` list matches the database schema.
///
/// Tables exercised (one explicit-column query each):
///   folder, script, resource, resource_type, variable, schedule, usr, group_
#[sqlx::test(fixtures("base"))]
async fn test_tarball_export_all_tables(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base_url = format!("http://localhost:{port}");

    let client = windmill_api_client::create_client(&base_url, "SECRET_TOKEN".to_string());
    let http = client.client();

    // ---- folder ----
    sqlx::query(
        r#"INSERT INTO folder
               (workspace_id, name, display_name, owners, extra_perms, summary)
           VALUES ($1, $2, $3, $4, '{}'::jsonb, $5)"#,
    )
    .bind("test-workspace")
    .bind("test_folder")
    .bind("Test Folder")
    .bind(vec!["u/test-user"])
    .bind("a test folder")
    .execute(&db)
    .await?;

    // ---- script (exercises the 30-column Script<ScriptRunnableSettingsHandle> query) ----
    client
        .create_script(
            "test-workspace",
            &windmill_api_client::types::NewScript {
                content: "export function main() { return 42; }".to_string(),
                language: windmill_api_client::types::ScriptLang::Bun,
                path: "f/test_folder/test_script".to_string(),
                summary: "test script".to_string(),
                description: "script for export test".to_string(),
                kind: Some("script".to_string()),
                tag: Some("test".to_string()),
                lock: None,
                parent_hash: None,
                schema: Default::default(),
                is_template: None,
                draft_only: None,
                dedicated_worker: None,
                ws_error_handler_muted: None,
                priority: None,
                cache_ttl: None,
                concurrent_limit: None,
                concurrency_time_window_s: None,
                timeout: None,
                delete_after_secs: None,
                restart_unless_cancelled: None,
                visible_to_runner_only: None,
                auto_kind: None,
                on_behalf_of_email: None,
                has_preprocessor: None,
                codebase: None,
                envs: vec![],
                deployment_message: None,
                assets: vec![],
                modules: None,
                concurrency_key: None,
            },
        )
        .await?;

    // ---- resource ----
    sqlx::query(
        r#"INSERT INTO resource
               (workspace_id, path, value, description, resource_type, created_by)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind("test-workspace")
    .bind("f/test_folder/test_res")
    .bind(serde_json::json!({"url": "http://example.com"}))
    .bind("test resource")
    .bind("http")
    .bind("test-user")
    .execute(&db)
    .await?;

    // ---- resource_type ----
    sqlx::query(
        r#"INSERT INTO resource_type
               (workspace_id, name, schema, description, created_by)
           VALUES ($1, $2, $3, $4, $5)"#,
    )
    .bind("test-workspace")
    .bind("http")
    .bind(serde_json::json!({"type": "object"}))
    .bind("HTTP resource type")
    .bind("system")
    .execute(&db)
    .await?;

    // ---- variable ----
    sqlx::query(
        r#"INSERT INTO variable
               (workspace_id, path, value, is_secret, description, account)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind("test-workspace")
    .bind("f/test_folder/test_var")
    .bind("test_value")
    .bind(false)
    .bind("test variable")
    .bind(None::<i32>)
    .execute(&db)
    .await?;

    // ---- schedule ----
    client
        .create_schedule(
            "test-workspace",
            &windmill_api_client::types::NewSchedule {
                schedule: "0 0 0 * * *".to_string(),
                script_path: "f/test_folder/test_script".to_string(),
                path: "f/test_folder/test_schedule".to_string(),
                is_flow: false,
                timezone: "UTC".to_string(),
                args: Default::default(),
                enabled: Some(false),
                description: Some("test schedule".to_string()),
                summary: Some("test schedule".to_string()),
                tag: None,
                cron_version: Some("v2".to_string()),
                on_failure: None,
                on_failure_times: None,
                on_failure_exact: None,
                on_failure_extra_args: None,
                on_recovery: None,
                on_recovery_times: None,
                on_recovery_extra_args: None,
                on_success: None,
                on_success_extra_args: None,
                ws_error_handler_muted: None,
                retry: None,
                no_flow_overlap: None,
            },
        )
        .await?;

    // ---- group_ (base fixture already has "all", create one more for include_groups) ----
    sqlx::query(
        "INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES ($1, $2, $3, '{}'::jsonb)",
    )
    .bind("test-workspace")
    .bind("testgroup")
    .bind("test group")
    .execute(&db)
    .await?;

    // ---- tarball export: hits ALL explicit-column queries at once ----
    let params = [
        "archive_type=tar",
        "include_schedules=true",
        "include_users=true",
        "include_groups=true",
        "include_settings=true",
        "include_workspace_dependencies=true",
        "settings_version=v1",
    ];

    let resp = http
        .get(format!(
            "{}/api/w/test-workspace/workspaces/tarball?{}",
            base_url,
            params.join("&")
        ))
        .bearer_auth("SECRET_TOKEN")
        .send()
        .await?;

    assert_eq!(
        resp.status(),
        200,
        "tarball export failed: {}",
        resp.text().await.unwrap_or_default()
    );

    // Verify we got actual bytes back
    let body = resp.bytes().await?;
    assert!(!body.is_empty(), "tarball export returned empty body");

    Ok(())
}

