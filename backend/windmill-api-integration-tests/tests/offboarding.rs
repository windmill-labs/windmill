use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn ws_url(port: u16, endpoint: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/users/{endpoint}")
}

fn global_url(port: u16, endpoint: &str) -> String {
    format!("http://localhost:{port}/api/users/{endpoint}")
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "offboarding_test"))]
async fn test_offboard_preview(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let resp = authed(client().get(ws_url(port, "offboard_preview/test-user-2")))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["scripts"], 3); // script_a, script_b, conflict_script
    assert_eq!(body["flows"], 1); // flow_a
    assert_eq!(body["resources"], 1); // res_a
    assert_eq!(body["variables"], 1); // var_a
    assert_eq!(body["schedules"], 1); // sched_a
    assert_eq!(body["tokens"], 1); // OFFBOARD_TOKEN_1

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "offboarding_test"))]
async fn test_offboard_to_user(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Remove conflict scripts so offboard can proceed
    sqlx::query!("DELETE FROM script WHERE hash = 1003 AND workspace_id = 'test-workspace'")
        .execute(&db)
        .await?;
    sqlx::query!("DELETE FROM script WHERE hash = 1004 AND workspace_id = 'test-workspace'")
        .execute(&db)
        .await?;

    let resp = authed(client().post(ws_url(port, "offboard/test-user-2")))
        .json(&json!({
            "reassign_to": "u/test-user",
            "delete_user": true
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert!(body["conflicts"].as_array().map_or(true, |a| a.is_empty()));
    assert!(body["summary"].is_object());

    let summary = &body["summary"];
    assert!(summary["scripts_reassigned"].as_i64().unwrap() > 0);
    assert!(summary["flows_reassigned"].as_i64().unwrap() > 0);
    assert!(summary["resources_reassigned"].as_i64().unwrap() > 0);
    assert!(summary["variables_reassigned"].as_i64().unwrap() > 0);
    assert!(summary["schedules_reassigned"].as_i64().unwrap() > 0);

    // Verify scripts moved
    let moved = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM script WHERE path LIKE 'u/test-user/%' AND workspace_id = 'test-workspace' AND NOT archived AND NOT deleted"
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);
    assert!(moved > 0, "scripts should be under u/test-user now");

    let old = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM script WHERE path LIKE 'u/test-user-2/%' AND workspace_id = 'test-workspace' AND NOT archived AND NOT deleted"
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);
    assert_eq!(old, 0, "no scripts should remain under u/test-user-2");

    // Verify schedule permissioned_as updated
    let perm = sqlx::query_scalar!(
        "SELECT permissioned_as FROM schedule WHERE path = 'u/test-user/sched_a' AND workspace_id = 'test-workspace'"
    )
    .fetch_optional(&db)
    .await?;
    assert_eq!(perm.as_deref(), Some("u/test-user"));

    // Verify user deleted from workspace
    let user_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE username = 'test-user-2' AND workspace_id = 'test-workspace')"
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(true);
    assert!(!user_exists, "user should be removed from workspace");

    // Verify tokens revoked (default behavior when no reassign_tokens_to)
    let token_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM token WHERE owner = 'u/test-user-2' AND workspace_id = 'test-workspace'"
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(1);
    assert_eq!(token_count, 0, "tokens should be revoked");

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "offboarding_test"))]
async fn test_offboard_to_folder(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let resp = authed(client().post(ws_url(port, "offboard/test-user-2")))
        .json(&json!({
            "reassign_to": "f/test-folder",
            "new_operator": "test-user",
            "delete_user": true
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await?;
    assert!(body["conflicts"].as_array().map_or(true, |a| a.is_empty()));

    // Verify scripts moved to folder
    let moved = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM script WHERE path LIKE 'f/test-folder/%' AND workspace_id = 'test-workspace'"
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(0);
    assert!(moved > 0, "scripts should be under f/test-folder now");

    // Verify schedule permissioned_as is u/test-user (operator), not g/f/test-folder
    let perm = sqlx::query_scalar!(
        "SELECT permissioned_as FROM schedule WHERE path = 'f/test-folder/sched_a' AND workspace_id = 'test-workspace'"
    )
    .fetch_optional(&db)
    .await?;
    assert_eq!(perm.as_deref(), Some("u/test-user"));

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "offboarding_test"))]
async fn test_offboard_reassign_only(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let resp = authed(client().post(ws_url(port, "offboard/test-user-2")))
        .json(&json!({
            "reassign_to": "u/test-user",
            "delete_user": false
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);

    // Verify user still exists in workspace
    let user_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE username = 'test-user-2' AND workspace_id = 'test-workspace')"
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);
    assert!(
        user_exists,
        "user should still exist when delete_user=false"
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "offboarding_test"))]
async fn test_offboard_conflicts(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // conflict_script exists under both u/test-user-2/ and u/test-user/
    let resp = authed(client().post(ws_url(port, "offboard/test-user-2")))
        .json(&json!({
            "reassign_to": "u/test-user",
            "delete_user": true
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await?;
    let conflicts = body["conflicts"]
        .as_array()
        .expect("conflicts should be array");
    assert!(!conflicts.is_empty(), "should have path conflicts");
    assert!(
        conflicts
            .iter()
            .any(|c| c.as_str().unwrap().contains("conflict_script")),
        "conflict should mention conflict_script"
    );
    // No summary when conflicts exist
    assert!(body["summary"].is_null());

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "offboarding_test"))]
async fn test_offboard_tokens_deleted(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Remove conflict_script so offboard can proceed
    sqlx::query!("DELETE FROM script WHERE hash = 1004 AND workspace_id = 'test-workspace'")
        .execute(&db)
        .await?;

    let resp = authed(client().post(ws_url(port, "offboard/test-user-2")))
        .json(&json!({
            "reassign_to": "u/test-user",
            "delete_user": true
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);

    // Verify tokens are deleted
    let token_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM token WHERE owner = 'u/test-user-2' AND workspace_id = 'test-workspace'"
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(1);
    assert_eq!(token_count, 0, "tokens should be deleted after offboarding");

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "offboarding_test"))]
async fn test_offboard_folder_requires_operator(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Missing new_operator when reassigning to folder should fail
    let resp = authed(client().post(ws_url(port, "offboard/test-user-2")))
        .json(&json!({
            "reassign_to": "f/test-folder",
            "delete_user": true
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 400);

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "offboarding_test"))]
async fn test_global_offboard_preview(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let resp = authed(client().get(global_url(port, "offboard_preview/test2@windmill.dev")))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await?;
    let workspaces = body["workspaces"]
        .as_array()
        .expect("should have workspaces");
    assert_eq!(workspaces.len(), 1);
    assert_eq!(workspaces[0]["workspace_id"], "test-workspace");
    assert_eq!(workspaces[0]["username"], "test-user-2");
    assert!(workspaces[0]["preview"]["scripts"].as_i64().unwrap() > 0);

    Ok(())
}
