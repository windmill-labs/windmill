use sqlx::postgres::Postgres;
use sqlx::Pool;
use windmill_test_utils::{initialize_tracing, set_jwt_secret, ApiServer};

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
                draft_only: None,
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

/// The archive carries every resource's and variable's `value`, which the per-item
/// routes gate on `resources:read:<path>` / `variables:read:<path>`. A token holding
/// only `workspaces:read` (what the route itself needs) must not collect them, and a
/// path-scoped token cannot stand in for the whole workspace either.
#[sqlx::test(fixtures("base"))]
async fn test_tarball_export_gates_values_on_item_scopes(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base_url = format!("http://localhost:{}", server.addr.port());

    sqlx::query(
        r#"INSERT INTO resource (workspace_id, path, value, resource_type, created_by)
           VALUES ('test-workspace', 'u/test-user/creds',
                   '{"password": "RESOURCE_VALUE"}'::jsonb, 'postgresql', 'test-user')"#,
    )
    .execute(&db)
    .await?;

    sqlx::query(
        r#"INSERT INTO variable (workspace_id, path, value, is_secret, description)
           VALUES ('test-workspace', 'u/test-user/plain', 'VARIABLE_VALUE', false, '')"#,
    )
    .execute(&db)
    .await?;

    sqlx::query(
        r#"INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin, scopes) VALUES
             (encode(sha256('WS_READ_TOKEN'::bytea), 'hex'), 'WS_READ_TO', 'WS_READ_TOKEN',
              'test@windmill.dev', 'workspaces:read only', false, '{workspaces:read}'),
             (encode(sha256('PATH_SCOPED_TOKEN'::bytea), 'hex'), 'PATH_SCOPE', 'PATH_SCOPED_TOKEN',
              'test@windmill.dev', 'path-scoped item read', false,
              '{workspaces:read,resources:read:u/test-user/creds,variables:read:u/test-user/plain}'),
             (encode(sha256('ITEM_READ_TOKEN'::bytea), 'hex'), 'ITEM_READ_', 'ITEM_READ_TOKEN',
              'test@windmill.dev', 'item read', false,
              '{workspaces:read,resources:read,variables:read}'),
             (encode(sha256('WILDCARD_TOKEN'::bytea), 'hex'), 'WILDCARD_T', 'WILDCARD_TOKEN',
              'test@windmill.dev', 'wildcard item read', false,
              '{workspaces:read,resources:read:*,variables:read:*}')"#,
    )
    .execute(&db)
    .await?;

    let export = async |token: &str, query: &str| -> anyhow::Result<(u16, String)> {
        let resp = reqwest::Client::new()
            .get(format!(
                "{base_url}/api/w/test-workspace/workspaces/tarball?{query}"
            ))
            .bearer_auth(token)
            .send()
            .await?;
        let status = resp.status().as_u16();
        // Lossy: a successful export is a tar, not UTF-8. Only the values matter here.
        Ok((
            status,
            String::from_utf8_lossy(&resp.bytes().await?).into_owned(),
        ))
    };

    for token in ["WS_READ_TOKEN", "PATH_SCOPED_TOKEN"] {
        let (status, body) = export(token, "").await?;
        assert_eq!(status, 403, "{token} exported values: {body}");
        assert!(
            !body.contains("RESOURCE_VALUE"),
            "{token} leaked a resource"
        );

        let (status, body) = export(token, "skip_resources=true").await?;
        assert_eq!(status, 403, "{token} exported variables: {body}");
        assert!(
            !body.contains("VARIABLE_VALUE"),
            "{token} leaked a variable"
        );

        // Skipping both kinds leaves an export the route's own scope covers.
        let (status, body) = export(token, "skip_resources=true&skip_variables=true").await?;
        assert_eq!(status, 200, "{token} denied a value-free export: {body}");
    }

    // `*` is a resource path the scope picker mints, and it spans the whole domain,
    // so it must export exactly as the unqualified grant does.
    for token in ["ITEM_READ_TOKEN", "WILDCARD_TOKEN"] {
        let (status, body) = export(token, "").await?;
        assert_eq!(status, 200, "{token} denied: {body}");
        assert!(
            body.contains("RESOURCE_VALUE") && body.contains("VARIABLE_VALUE"),
            "{token} exported no values"
        );
    }

    Ok(())
}

/// `settings.json` carries the admin-managed integration config that `get_settings`
/// is admin-only for (the webhook URL, ai_config, git_sync, handler extra_args), so
/// `include_settings` takes the same admin check as `get_settings` rather than
/// riding on the route's `workspaces:read`. Git sync exports settings through the
/// same route, so the gate must still admit its system identity.
#[sqlx::test(fixtures("base"))]
async fn test_tarball_export_settings_are_admin_only(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    let server = ApiServer::start(db.clone()).await?;
    let base_url = format!("http://localhost:{}", server.addr.port());

    sqlx::query(
        r#"UPDATE workspace_settings
              SET webhook = 'https://hook.example/?token=WEBHOOK_SECRET',
                  ai_config = '{"providers":{"openai":{"api_key":"AI_CONFIG_SECRET"}}}'::jsonb
            WHERE workspace_id = 'test-workspace'"#,
    )
    .execute(&db)
    .await?;

    let export = async |token: &str| -> anyhow::Result<(u16, String)> {
        let resp = reqwest::Client::new()
            .get(format!(
                "{base_url}/api/w/test-workspace/workspaces/tarball?include_settings=true&settings_version=v2"
            ))
            .bearer_auth(token)
            .send()
            .await?;
        let status = resp.status().as_u16();
        // Lossy: a successful export is a tar, not UTF-8. Only the values matter here.
        Ok((
            status,
            String::from_utf8_lossy(&resp.bytes().await?).into_owned(),
        ))
    };

    // SECRET_TOKEN_2 belongs to test-user-2, a non-admin member of test-workspace.
    let (status, body) = export("SECRET_TOKEN_2").await?;
    assert_eq!(status, 403, "non-admin exported settings: {body}");

    let (status, body) = export("SECRET_TOKEN").await?;
    assert_eq!(status, 200, "admin denied settings: {body}");
    assert!(
        body.contains("WEBHOOK_SECRET") && body.contains("AI_CONFIG_SECRET"),
        "admin got no settings"
    );

    // Git sync pushes the workspace to the repo by exporting it under
    // `superadmin_sync@windmill.dev`, which belongs to no workspace: the job token
    // it runs with is the export's only admin claim.
    let sync_email = windmill_common::users::SUPERADMIN_SYNC_EMAIL;
    let sync_token = windmill_common::auth::create_token_for_owner(
        &db,
        "test-workspace",
        sync_email,
        "git-sync",
        300,
        sync_email,
        &uuid::Uuid::new_v4(),
        None,
        None,
    )
    .await?;
    let (status, body) = export(&sync_token).await?;
    assert_eq!(status, 200, "git-sync identity denied settings: {body}");
    assert!(
        body.contains("WEBHOOK_SECRET"),
        "git-sync identity got no settings"
    );

    Ok(())
}
