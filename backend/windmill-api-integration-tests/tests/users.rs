use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn user_url(port: u16, endpoint: &str, name: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/users/{endpoint}/{name}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_user_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/users");

    // ===== Global (non-workspace) endpoints =====
    let global_base = format!("http://localhost:{port}/api/users");

    // --- global whoami ---
    let resp = authed(client().get(format!("{global_base}/whoami")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["email"], "test@windmill.dev");

    // --- get_email ---
    let resp = authed(client().get(format!("{global_base}/email")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let email = resp.text().await?;
    assert_eq!(email, "test@windmill.dev");

    // --- exists_email ---
    let resp = authed(client().get(format!(
        "{global_base}/exists/test@windmill.dev"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed(client().get(format!(
        "{global_base}/exists/nonexistent@windmill.dev"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- list_as_super_admin ---
    let resp = authed(client().get(format!("{global_base}/list_as_super_admin")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!list.is_empty());

    // --- tokens/list ---
    let resp = authed(client().get(format!("{global_base}/tokens/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // --- tokens/create ---
    let resp = authed(client().post(format!("{global_base}/tokens/create")))
        .json(&json!({"label": "ephemeral-test-token"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let new_token = resp.text().await?;
    assert!(!new_token.is_empty());

    // --- tokens/delete ---
    let token_prefix = &new_token[..std::cmp::min(new_token.len(), 10)];
    let resp = authed(client().delete(format!(
        "{global_base}/tokens/delete/{token_prefix}"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "delete token: {}",
        resp.text().await?
    );

    // --- list_invites ---
    let resp = authed(client().get(format!("{global_base}/list_invites")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // --- username_info ---
    let resp = authed(client().get(format!(
        "{global_base}/username_info/test@windmill.dev"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["username"], "test-user");

    // --- global usage ---
    let resp = authed(client().get(format!("{global_base}/usage")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- tutorial_progress (get, then set, then get again) ---
    let resp = authed(client().get(format!("{global_base}/tutorial_progress")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<serde_json::Value>().await?;

    let resp = authed(client().post(format!("{global_base}/tutorial_progress")))
        .json(&json!({"progress": 42, "skipped_all": false}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "set tutorial_progress: {}",
        resp.text().await?
    );

    let resp = authed(client().get(format!("{global_base}/tutorial_progress")))
        .send()
        .await
        .unwrap();
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["progress"], 42);

    // --- global update user ---
    let resp = authed(client().post(format!(
        "{global_base}/update/test2@windmill.dev"
    )))
    .json(&json!({"name": "Updated Test User 2"}))
    .send()
    .await
    .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "global update user: {}",
        resp.text().await?
    );

    // --- setpassword (EE-gated in OSS) ---
    let resp = authed(client().post(format!("{global_base}/setpassword")))
        .json(&json!({"password": "new-test-password-123"}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "setpassword: unexpected status {}",
        resp.status()
    );

    // --- all_runnables ---
    let resp = authed(client().get(format!("{global_base}/all_runnables")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // --- onboarding (EE-gated in OSS) ---
    let resp = authed(client().post(format!("{global_base}/onboarding")))
        .json(&json!({"touch_point": "test", "use_case": "testing"}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "onboarding: unexpected status {}",
        resp.status()
    );

    // --- decline_invite (no pending invite, but endpoint should handle gracefully) ---
    let resp = authed(client().post(format!("{global_base}/decline_invite")))
        .json(&json!({"workspace_id": "nonexistent-ws"}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 404,
        "decline_invite: unexpected status {}",
        resp.status()
    );

    // --- auth: is_first_time_setup (unauthed) ---
    let resp = client()
        .get(format!("http://localhost:{port}/api/auth/is_first_time_setup"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let is_first = resp.json::<bool>().await?;
    assert_eq!(is_first, false);

    // --- auth: is_smtp_configured (unauthed) ---
    let resp = client()
        .get(format!("http://localhost:{port}/api/auth/is_smtp_configured"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<bool>().await?;

    // --- create user (global, EE-gated in OSS) ---
    let resp = authed(client().post(format!("{global_base}/create")))
        .json(&json!({
            "email": "newglobaluser@windmill.dev",
            "password": "test-password-123",
            "super_admin": false,
            "name": "New Global User"
        }))
        .send()
        .await
        .unwrap();
    let create_status = resp.status();
    assert!(
        create_status == 201 || create_status == 500,
        "create user: unexpected status {}",
        create_status
    );

    if create_status == 201 {
        // --- rename user (only if create succeeded / EE) ---
        let resp = authed(client().post(format!(
            "{global_base}/rename/newglobaluser@windmill.dev"
        )))
        .json(&json!({"new_username": "renamed_user"}))
        .send()
        .await
        .unwrap();
        assert_eq!(
            resp.status(),
            200,
            "rename user: {}",
            resp.text().await?
        );

        // --- global delete user ---
        let resp = authed(client().delete(format!(
            "{global_base}/delete/newglobaluser@windmill.dev"
        )))
        .send()
        .await
        .unwrap();
        assert_eq!(
            resp.status(),
            200,
            "global delete user: {}",
            resp.text().await?
        );
    }

    // ===== Auth (unauthed) endpoints =====
    let auth_base = format!("http://localhost:{port}/api/auth");

    // --- login (will fail: password hash in fixture is fake) ---
    let resp = client()
        .post(format!("{auth_base}/login"))
        .json(&json!({"email": "test@windmill.dev", "password": "wrong-password"}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 400 || resp.status() == 401 || resp.status() == 500,
        "login: unexpected status {}",
        resp.status()
    );

    // --- logout (POST, with auth token) ---
    let resp = authed(client().post(format!("{auth_base}/logout")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 303,
        "logout POST: unexpected status {}",
        resp.status()
    );

    // --- logout (GET, with auth token) ---
    let resp = authed(client().get(format!("{auth_base}/logout")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 303,
        "logout GET: unexpected status {}",
        resp.status()
    );

    // --- request_password_reset (returns 400 if SMTP not configured) ---
    let resp = client()
        .post(format!("{auth_base}/request_password_reset"))
        .json(&json!({"email": "test@windmill.dev"}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "request_password_reset: unexpected status {}",
        resp.status()
    );

    // --- reset_password (EE-gated, invalid token) ---
    let resp = client()
        .post(format!("{auth_base}/reset_password"))
        .json(&json!({"token": "invalid-token", "new_password": "new-pass"}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 400 || resp.status() == 500,
        "reset_password: unexpected status {}",
        resp.status()
    );

    // ===== Workspace-scoped endpoints =====

    // --- whoami ---
    let resp = authed(client().get(format!("{base}/whoami")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["username"], "test-user");
    assert_eq!(body["email"], "test@windmill.dev");
    assert_eq!(body["is_admin"], true);

    // --- list ---
    let resp = authed(client().get(format!("{base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!list.is_empty());
    assert!(list.iter().any(|u| u["username"] == "test-user"));

    // --- list_usernames ---
    let resp = authed(client().get(format!("{base}/list_usernames")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let usernames = resp.json::<Vec<String>>().await?;
    assert!(usernames.contains(&"test-user".to_string()));

    // --- get ---
    let resp = authed(client().get(user_url(port, "get", "test-user")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["username"], "test-user");

    // --- whois ---
    let resp = authed(client().get(user_url(port, "whois", "test-user")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["username"], "test-user");

    // --- username_to_email ---
    let resp = authed(client().get(user_url(port, "username_to_email", "test-user")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let email = resp.text().await?;
    assert_eq!(email, "test@windmill.dev");

    // --- exists ---
    let resp = authed(client().post(format!("{base}/exists")))
        .json(&json!({"username": "test-user"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed(client().post(format!("{base}/exists")))
        .json(&json!({"username": "nonexistent"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- list_usage ---
    let resp = authed(client().get(format!("{base}/list_usage")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // --- is_owner ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/users/is_owner/u/test-user/test"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    // --- update (make non-admin, then revert) ---
    let resp = authed(client().post(user_url(port, "update", "test-user")))
        .json(&json!({"is_admin": false}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "update user: {}", resp.text().await?);

    // revert back to admin
    let resp = authed(client().post(user_url(port, "update", "test-user")))
        .json(&json!({"is_admin": true}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- delete workspace user (admin deletes test-user-2) ---
    let resp = authed(client().delete(user_url(port, "delete", "test-user-2")))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "delete user: {}",
        resp.text().await?
    );

    // verify deleted
    let resp = authed(client().get(format!("{base}/list_usernames")))
        .send()
        .await
        .unwrap();
    let usernames = resp.json::<Vec<String>>().await?;
    assert!(!usernames.contains(&"test-user-2".to_string()));

    // --- leave workspace (test-user-3 leaves voluntarily) ---
    let resp = client()
        .post(format!("{base}/leave"))
        .header("Authorization", "Bearer SECRET_TOKEN_3")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "leave: {}", resp.text().await?);

    // verify left
    let resp = authed(client().get(format!("{base}/list_usernames")))
        .send()
        .await
        .unwrap();
    let usernames = resp.json::<Vec<String>>().await?;
    assert!(!usernames.contains(&"test-user-3".to_string()));

    Ok(())
}
