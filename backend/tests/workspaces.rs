use serde_json::json;
use sqlx::{Pool, Postgres};

mod common;
use common::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

#[sqlx::test(fixtures("base"))]
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
    // existing username -> 400
    let resp = authed(client().post(format!("{global_base}/exists_username")))
        .json(&json!({"id": "test-workspace", "username": "test-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // available username -> 200
    let resp = authed(client().post(format!("{global_base}/exists_username")))
        .json(&json!({"id": "test-workspace", "username": "available-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // ===== Workspace-scoped endpoints =====

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

    Ok(())
}
