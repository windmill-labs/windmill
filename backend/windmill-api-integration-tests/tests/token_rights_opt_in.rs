//! A token from `tokens/create` carries its owner's workspace admin and superadmin rights only
//! when asked, the superadmin gates honour it although they key off the account's email, and a
//! token withholding them can neither mint one back nor trade itself for a session.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::{auth::create_jwt_token_withholding, db::Authed};
use windmill_test_utils::*;

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_token_rights_are_opt_in(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api", server.addr.port());
    let http = reqwest::Client::new();

    let mint = |bearer: &str, body: serde_json::Value| {
        http.post(format!("{base}/users/tokens/create"))
            .bearer_auth(bearer.to_string())
            .json(&body)
            .send()
    };
    // SECRET_TOKEN's owner is a superadmin and an admin of test-workspace.
    let status = |path: &str, token: &str| {
        http.get(format!("{base}{path}"))
            .bearer_auth(token.to_string())
            .send()
    };
    let admin_route = "/w/test-workspace/workspaces/list_pending_invites";
    let superadmin_route = "/users/list_as_super_admin";

    let plain = mint("SECRET_TOKEN", json!({})).await?.text().await?;
    assert_eq!(status(admin_route, &plain).await?.status(), 403);
    assert_eq!(status(superadmin_route, &plain).await?.status(), 401);

    let admin = mint("SECRET_TOKEN", json!({ "admin": true }))
        .await?
        .text()
        .await?;
    assert_eq!(status(admin_route, &admin).await?.status(), 200);
    assert_eq!(status(superadmin_route, &admin).await?.status(), 401);

    let superadmin = mint("SECRET_TOKEN", json!({ "super_admin": true }))
        .await?
        .text()
        .await?;
    assert_eq!(status(superadmin_route, &superadmin).await?.status(), 200);

    let regained = mint(&plain, json!({ "admin": true, "super_admin": true }))
        .await?
        .text()
        .await?;
    assert_eq!(status(admin_route, &regained).await?.status(), 403);
    assert_eq!(status(superadmin_route, &regained).await?.status(), 401);

    // A JWT minted from such a token (the AI session tools mint one per call) keeps the caps.
    let caller_jwt = create_jwt_token_withholding(
        Authed {
            email: "test@windmill.dev".to_string(),
            username: "test-user".to_string(),
            is_admin: false,
            is_operator: false,
            groups: vec![],
            folders: vec![],
            scopes: None,
            token_prefix: None,
            mcp_client: None,
            capabilities: None,
        },
        "test-workspace",
        3600,
        None,
        None,
        None,
        None,
        (true, true),
    )
    .await?;
    assert_eq!(status(superadmin_route, &caller_jwt).await?.status(), 401);
    let from_jwt = mint(&caller_jwt, json!({ "admin": true, "super_admin": true }))
        .await?
        .text()
        .await?;
    assert_eq!(status(admin_route, &from_jwt).await?.status(), 403);
    assert_eq!(status(superadmin_route, &from_jwt).await?.status(), 401);

    assert_eq!(
        status("/users/refresh_token", &plain).await?.status(),
        401,
        "a session would carry the rights the token withholds"
    );

    Ok(())
}
