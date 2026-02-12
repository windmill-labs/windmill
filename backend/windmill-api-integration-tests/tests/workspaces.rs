use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_workspace_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");
    let global_base = format!("http://localhost:{port}/api/workspaces");

    // ===== Global endpoints =====

    // --- list ---
    let resp = authed(client().get(format!("{global_base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(list.iter().any(|w| w["id"] == "test-workspace"));

    // --- list_as_superadmin ---
    let resp = authed(client().get(format!("{global_base}/list_as_superadmin")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(list.iter().any(|w| w["id"] == "test-workspace"));

    // --- users (user's workspaces) ---
    let resp = authed(client().get(format!("{global_base}/users")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    let workspaces = body["workspaces"].as_array().unwrap();
    assert!(workspaces.iter().any(|w| w["id"] == "test-workspace"));

    // --- exists ---
    let resp = authed(client().post(format!("{global_base}/exists")))
        .json(&json!({"id": "test-workspace"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed(client().post(format!("{global_base}/exists")))
        .json(&json!({"id": "nonexistent-workspace"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- exists_username (validates username is available) ---
    let resp = authed(client().post(format!("{global_base}/exists_username")))
        .json(&json!({"id": "test-workspace", "username": "test-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    let resp = authed(client().post(format!("{global_base}/exists_username")))
        .json(&json!({"id": "test-workspace", "username": "available-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- allowed_domain_auto_invite ---
    let resp = authed(client().get(format!(
        "{global_base}/allowed_domain_auto_invite"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<bool>().await?;

    // --- create workspace ---
    let resp = authed(client().post(format!("{global_base}/create")))
        .json(&json!({
            "id": "new-test-ws",
            "name": "New Test Workspace"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "create: {}", resp.text().await?);

    // verify it exists
    let resp = authed(client().post(format!("{global_base}/exists")))
        .json(&json!({"id": "new-test-ws"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.json::<bool>().await?, true);

    // ===== Workspace-scoped endpoints (read) =====

    // --- get_settings ---
    let resp = authed(client().get(format!("{base}/get_settings")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let settings = resp.json::<serde_json::Value>().await?;
    assert!(settings.is_object());

    // --- get_deploy_to ---
    let resp = authed(client().get(format!("{base}/get_deploy_to")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- is_premium ---
    let resp = authed(client().get(format!("{base}/is_premium")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- default_app ---
    let resp = authed(client().get(format!("{base}/default_app")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- default_scripts ---
    let resp = authed(client().get(format!("{base}/default_scripts")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- list_pending_invites ---
    let resp = authed(client().get(format!("{base}/list_pending_invites")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // --- encryption_key ---
    let resp = authed(client().get(format!("{base}/encryption_key")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- get_dependency_map ---
    let resp = authed(client().get(format!("{base}/get_dependency_map")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- get_as_superadmin ---
    let resp = authed(client().get(format!("{base}/get_as_superadmin")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["id"], "test-workspace");

    // --- get_workspace_name ---
    let resp = authed(client().get(format!("{base}/get_workspace_name")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let name = resp.text().await?;
    assert_eq!(name, "test-workspace");

    // --- get_usage ---
    let resp = authed(client().get(format!("{base}/usage")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- get_used_triggers ---
    let resp = authed(client().get(format!("{base}/used_triggers")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<serde_json::Value>().await?;

    // --- get_secondary_storage_names ---
    let resp = authed(client().get(format!("{base}/get_secondary_storage_names")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<String>>().await?;

    // --- get_dependents (empty, no dependencies exist) ---
    let resp = authed(client().get(format!(
        "{base}/get_dependents/u/test-user/nonexistent"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    let dependents = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(dependents.is_empty());

    // --- get_dependents_amounts ---
    let resp = authed(client().post(format!("{base}/get_dependents_amounts")))
        .json(&json!(["u/test-user/some_script"]))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // --- list_ducklakes ---
    let resp = authed(client().get(format!("{base}/list_ducklakes")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<String>>().await?;

    // --- list_datatables ---
    let resp = authed(client().get(format!("{base}/list_datatables")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<String>>().await?;

    // --- list_datatable_schemas ---
    let resp = authed(client().get(format!("{base}/list_datatable_schemas")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // ===== Workspace-scoped endpoints (mutations) =====

    // --- update (edit_workspace) ---
    let resp = authed(client().post(format!("{base}/update")))
        .json(&json!({"name": "renamed-workspace", "owner": "test-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "update: {}", resp.text().await?);

    // --- change_workspace_name ---
    let resp = authed(client().post(format!("{base}/change_workspace_name")))
        .json(&json!({"new_name": "Test Workspace Renamed"}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "change_workspace_name: {}",
        resp.text().await?
    );

    // verify name changed
    let resp = authed(client().get(format!("{base}/get_workspace_name")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.text().await?, "Test Workspace Renamed");

    // --- change_workspace_color ---
    let resp = authed(client().post(format!("{base}/change_workspace_color")))
        .json(&json!({"color": "#FF5733"}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "change_workspace_color: {}",
        resp.text().await?
    );

    // --- edit_webhook ---
    let resp = authed(client().post(format!("{base}/edit_webhook")))
        .json(&json!({"webhook": "https://example.com/hook"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "edit_webhook: {}", resp.text().await?);

    // verify in settings
    let resp = authed(client().get(format!("{base}/get_settings")))
        .send()
        .await
        .unwrap();
    let settings = resp.json::<serde_json::Value>().await?;
    assert_eq!(settings["webhook"], "https://example.com/hook");

    // clear webhook
    let resp = authed(client().post(format!("{base}/edit_webhook")))
        .json(&json!({"webhook": null}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- edit_auto_invite (EE-gated) ---
    let resp = authed(client().post(format!("{base}/edit_auto_invite")))
        .json(&json!({"operator": false, "invite_all": false, "auto_add": false}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "edit_auto_invite: unexpected status {}",
        resp.status()
    );

    // --- edit_slack_command ---
    let resp = authed(client().post(format!("{base}/edit_slack_command")))
        .json(&json!({"slack_command_script": null}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "edit_slack_command: {}",
        resp.text().await?
    );

    // --- edit_error_handler (new format) ---
    let resp = authed(client().post(format!("{base}/edit_error_handler")))
        .json(&json!({"path": null, "extra_args": null}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "edit_error_handler: {}",
        resp.text().await?
    );

    // --- edit_success_handler (new format) ---
    let resp = authed(client().post(format!("{base}/edit_success_handler")))
        .json(&json!({"path": null, "extra_args": null}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "edit_success_handler: {}",
        resp.text().await?
    );

    // --- edit_default_scripts ---
    let resp = authed(client().post(format!("{base}/default_scripts")))
        .json(&json!(null))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "edit_default_scripts: {}",
        resp.text().await?
    );

    // --- edit_default_app (EE-gated, may return 200 or error) ---
    let resp = authed(client().post(format!("{base}/edit_default_app")))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "edit_default_app: unexpected status {}",
        resp.status()
    );

    // --- set_environment_variable ---
    let resp = authed(client().post(format!("{base}/set_environment_variable")))
        .json(&json!({"name": "TEST_ENV_VAR", "value": "test_value"}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "set_environment_variable: {}",
        resp.text().await?
    );

    // --- edit_deploy_to (EE-gated) ---
    let resp = authed(client().post(format!("{base}/edit_deploy_to")))
        .json(&json!({"deploy_to": null}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "edit_deploy_to: unexpected status {}",
        resp.status()
    );

    // --- edit_large_file_storage_config ---
    let resp = authed(client().post(format!(
        "{base}/edit_large_file_storage_config"
    )))
    .json(&json!({"large_file_storage": null}))
    .send()
    .await
    .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "edit_large_file_storage_config: {}",
        resp.text().await?
    );

    // --- edit_deploy_ui_config (EE-gated) ---
    let resp = authed(client().post(format!("{base}/edit_deploy_ui_config")))
        .json(&json!({"deploy_ui": null}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "edit_deploy_ui_config: unexpected status {}",
        resp.status()
    );

    // --- edit_git_sync_config (EE-gated) ---
    let resp = authed(client().post(format!("{base}/edit_git_sync_config")))
        .json(&json!({"git_sync_settings": null}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "edit_git_sync_config: unexpected status {}",
        resp.status()
    );

    // --- update_operator_settings ---
    let resp = authed(client().post(format!("{base}/operator_settings")))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "update_operator_settings: {}",
        resp.text().await?
    );

    // --- edit_public_app_rate_limit ---
    let resp = authed(client().post(format!("{base}/public_app_rate_limit")))
        .json(&json!({"public_app_execution_limit_per_minute": null}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "edit_public_app_rate_limit: {}",
        resp.text().await?
    );

    // --- rebuild_dependency_map ---
    let resp = authed(client().post(format!("{base}/rebuild_dependency_map")))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "rebuild_dependency_map: {}",
        resp.text().await?
    );

    // --- add_user ---
    let resp = authed(client().post(format!("{base}/add_user")))
        .json(&json!({
            "email": "newuser@windmill.dev",
            "is_admin": false,
            "operator": false
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "add_user: {}", resp.text().await?);

    // --- invite_user + list_pending_invites + delete_invite ---
    let resp = authed(client().post(format!("{base}/invite_user")))
        .json(&json!({
            "email": "invited@example.com",
            "is_admin": false,
            "operator": false
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "invite_user: {}", resp.text().await?);

    // verify invite shows in pending
    let resp = authed(client().get(format!("{base}/list_pending_invites")))
        .send()
        .await
        .unwrap();
    let invites = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(
        invites
            .iter()
            .any(|i| i["email"] == "invited@example.com"),
        "invite not found: {:?}",
        invites
    );

    // delete invite
    let resp = authed(client().post(format!("{base}/delete_invite")))
        .json(&json!({
            "email": "invited@example.com",
            "is_admin": false,
            "operator": false
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        201,
        "delete_invite: {}",
        resp.text().await?
    );

    // ===== Critical alerts (EE-gated, returns 404 in OSS) =====

    // --- get critical_alerts ---
    let resp = authed(client().get(format!("{base}/critical_alerts")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 404,
        "critical_alerts: unexpected status {}",
        resp.status()
    );

    // --- acknowledge critical alert (nonexistent id) ---
    let resp = authed(client().post(format!("{base}/critical_alerts/1/acknowledge")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 404,
        "acknowledge_critical_alert: unexpected status {}",
        resp.status()
    );

    // --- acknowledge_all critical alerts ---
    let resp = authed(client().post(format!("{base}/critical_alerts/acknowledge_all")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 404,
        "acknowledge_all_critical_alerts: unexpected status {}",
        resp.status()
    );

    // --- mute critical alerts ---
    let resp = authed(client().post(format!("{base}/critical_alerts/mute")))
        .json(&json!({"mute_critical_alerts": false}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 404,
        "mute_critical_alerts: unexpected status {}",
        resp.status()
    );

    // ===== Tarball export =====

    // --- tarball (download workspace as tar archive) ---
    let resp = authed(client().get(format!("{base}/tarball")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "tarball: {}", resp.status());

    // ===== Fork operations (on the newly created workspace) =====

    // --- create_fork (workspace-scoped, from new-test-ws) ---
    let new_ws_base = format!("http://localhost:{port}/api/w/new-test-ws/workspaces");
    let resp = authed(client().post(format!("{new_ws_base}/create_fork")))
        .json(&json!({
            "id": "wm-fork-test-ws",
            "name": "Forked Test Workspace"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "create_fork: {}",
        resp.text().await?
    );

    // verify fork exists
    let resp = authed(client().post(format!("{global_base}/exists")))
        .json(&json!({"id": "wm-fork-test-ws"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.json::<bool>().await?, true);

    // --- change_workspace_id ---
    let fork_ws_base = format!("http://localhost:{port}/api/w/wm-fork-test-ws/workspaces");
    let resp = authed(client().post(format!("{fork_ws_base}/change_workspace_id")))
        .json(&json!({
            "new_id": "wm-fork-renamed",
            "new_name": "Renamed Fork"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "change_workspace_id: {}",
        resp.text().await?
    );

    // verify renamed workspace exists
    let resp = authed(client().post(format!("{global_base}/exists")))
        .json(&json!({"id": "wm-fork-renamed"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.json::<bool>().await?, true);

    // clean up renamed fork
    let resp = authed(client().delete(format!("{global_base}/delete/wm-fork-renamed")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- archive workspace (on the newly created one, not our main test workspace) ---
    let new_ws_base = format!("http://localhost:{port}/api/w/new-test-ws/workspaces");
    let resp = authed(client().post(format!("{new_ws_base}/archive")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "archive: {}", resp.text().await?);

    // --- unarchive workspace (global) ---
    let resp = authed(client().post(format!("{global_base}/unarchive/new-test-ws")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "unarchive: {}", resp.text().await?);

    // --- delete workspace (global) ---
    let resp = authed(client().delete(format!("{global_base}/delete/new-test-ws")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "delete: {}", resp.text().await?);

    // verify deleted
    let resp = authed(client().post(format!("{global_base}/exists")))
        .json(&json!({"id": "new-test-ws"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.json::<bool>().await?, false);

    // --- create_workspace_require_superadmin ---
    let resp = authed(client().get(format!(
        "{global_base}/create_workspace_require_superadmin"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    Ok(())
}
