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
    let resp = authed(client().get(format!("{global_base}/exists/test@windmill.dev")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed(client().get(format!("{global_base}/exists/nonexistent@windmill.dev")))
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

    let token_prefix = &new_token[..std::cmp::min(new_token.len(), 10)];

    // --- tokens/update_scopes (set explicit scopes) ---
    let resp = authed(client().post(format!("{global_base}/tokens/update_scopes/{token_prefix}")))
        .json(&json!({"scopes": ["jobs:run:scripts"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "update_scopes: {}", resp.text().await?);

    // Verify via tokens/list that scopes were applied.
    let resp = authed(client().get(format!("{global_base}/tokens/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let tokens = resp.json::<Vec<serde_json::Value>>().await?;
    let updated = tokens
        .iter()
        .find(|t| t["token_prefix"] == *token_prefix)
        .expect("token in list");
    assert_eq!(updated["scopes"], json!(["jobs:run:scripts"]));

    // --- tokens/update_scopes (clear scopes via null = full access) ---
    let resp = authed(client().post(format!("{global_base}/tokens/update_scopes/{token_prefix}")))
        .json(&json!({"scopes": null}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- tokens/update_scopes on nonexistent prefix returns 404 ---
    let resp = authed(client().post(format!("{global_base}/tokens/update_scopes/zzznotreal")))
        .json(&json!({"scopes": ["jobs:run:scripts"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    // --- tokens/delete ---
    let resp = authed(client().delete(format!("{global_base}/tokens/delete/{token_prefix}")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "delete token: {}", resp.text().await?);

    // --- list_invites ---
    let resp = authed(client().get(format!("{global_base}/list_invites")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // --- username_info ---
    let resp = authed(client().get(format!("{global_base}/username_info/test@windmill.dev")))
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
    let resp = authed(client().post(format!("{global_base}/update/test2@windmill.dev")))
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
        .get(format!(
            "http://localhost:{port}/api/auth/is_first_time_setup"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let is_first = resp.json::<bool>().await?;
    assert_eq!(is_first, false);

    // --- auth: is_smtp_configured (unauthed) ---
    let resp = client()
        .get(format!(
            "http://localhost:{port}/api/auth/is_smtp_configured"
        ))
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
        let resp =
            authed(client().post(format!("{global_base}/rename/newglobaluser@windmill.dev")))
                .json(&json!({"new_username": "renamed_user"}))
                .send()
                .await
                .unwrap();
        assert_eq!(resp.status(), 200, "rename user: {}", resp.text().await?);

        // --- global delete user ---
        let resp =
            authed(client().delete(format!("{global_base}/delete/newglobaluser@windmill.dev")))
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
    // An unparseable stored hash must read as a failed login, not as a server error
    // relaying the hash parser's message to an unauthenticated caller.
    let resp = client()
        .post(format!("{auth_base}/login"))
        .json(&json!({"email": "test@windmill.dev", "password": "wrong-password"}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        400,
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
    assert_eq!(resp.status(), 200, "delete user: {}", resp.text().await?);

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

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_list_addable_instance_users(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/users/list_addable");

    // The three fixture accounts are all members of test-workspace already.
    sqlx::query(
        "INSERT INTO password(email, password_hash, login_type, verified, username, disabled) VALUES
         ('addable@windmill.dev', 'x', 'password', true, 'addable-user', false),
         ('with_underscore@windmill.dev', 'x', 'password', true, 'underscored', false),
         ('gone@windmill.dev', 'x', 'password', true, 'gone-user', true)"
    )
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO usr(workspace_id, email, username, is_admin, role, is_service_account)
         VALUES ('test-workspace', 'sa@creator.test-workspace.sa.wm.dev', 'sa', false, 'User', true)"
    )
    .execute(&db)
    .await?;

    let emails = |query: &str| {
        let url = format!("{base}?{query}");
        async move {
            let resp = authed(client().get(url)).send().await.unwrap();
            assert_eq!(resp.status(), 200);
            resp.json::<Vec<serde_json::Value>>()
                .await
                .unwrap()
                .into_iter()
                .map(|u| u["email"].as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        }
    };

    // Members, disabled accounts and service accounts are all out; a service account has no
    // `password` row at all, so it can never reach the picker.
    assert_eq!(
        emails("").await,
        vec![
            "addable@windmill.dev",
            // seeded by the migrations, not a member of test-workspace
            "admin@windmill.dev",
            "with_underscore@windmill.dev"
        ]
    );

    // The exclusions are part of the query, so the limit counts addable accounts only — a
    // workspace whose members sort first must not swallow the whole page.
    assert_eq!(emails("per_page=1").await, vec!["addable@windmill.dev"]);

    // Search matches the instance username as well as the email.
    assert_eq!(
        emails("search=addable-user").await,
        vec!["addable@windmill.dev"]
    );

    // Wildcards in the search are matched literally rather than expanded.
    assert_eq!(
        emails("search=_").await,
        vec!["with_underscore@windmill.dev"]
    );
    assert!(emails("search=%25").await.is_empty());

    // Listing instance-wide accounts is superadmin-only, workspace admin is not enough.
    let resp = client()
        .get(&base)
        .header("Authorization", "Bearer SECRET_TOKEN_3")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_change_user_email(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let global_base = format!("http://localhost:{port}/api/users");

    let change_email = |email: &str, new_email: &str| {
        authed(client().post(format!("{global_base}/change_email/{email}")))
            .json(&json!({ "new_email": new_email }))
            .send()
    };

    sqlx::query!("UPDATE password SET username = 'test-user-2' WHERE email = 'test2@windmill.dev'")
        .execute(&db)
        .await?;
    sqlx::query!("UPDATE workspace SET owner = 'test2@windmill.dev' WHERE id = 'test-workspace'")
        .execute(&db)
        .await?;
    // A user whose username is their email is stored as the bare address in `permissioned_as` and
    // in an app's policy, rather than as `u/{username}`.
    sqlx::query!(
        "INSERT INTO schedule(workspace_id, path, edited_by, schedule, timezone, enabled, script_path, is_flow, args, email, permissioned_as)
         VALUES ('test-workspace', 'u/test-user-2/sched', 'test-user-2', '0 0 1 1 *', 'UTC', false, 'u/test-user-2/s', false, '{}'::json, 'test2@windmill.dev', 'test2@windmill.dev')"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "INSERT INTO app(workspace_id, path, summary, policy, versions)
         VALUES ('test-workspace', 'u/test-user-2/app', '', '{\"on_behalf_of\": \"test2@windmill.dev\", \"on_behalf_of_email\": \"test2@windmill.dev\"}'::jsonb, '{}')"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "INSERT INTO folder(workspace_id, name, display_name, owners, extra_perms, default_permissioned_as)
         VALUES ('test-workspace', 'fold', 'fold', '{}', '{}'::jsonb,
                 '[{\"path_glob\": \"a/**\", \"permissioned_as\": \"u/other\"}, {\"path_glob\": \"**\", \"permissioned_as\": \"test2@windmill.dev\"}]'::jsonb)"
    )
    .execute(&db)
    .await?;

    let resp = change_email("test2@windmill.dev", "renamed@windmill.dev")
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "change_email: {}", resp.text().await?);

    // The account row is moved, not recreated, so the instance-wide username and the workspace
    // membership follow the new address.
    let username =
        sqlx::query_scalar!("SELECT username FROM password WHERE email = 'renamed@windmill.dev'")
            .fetch_one(&db)
            .await?;
    assert_eq!(username.as_deref(), Some("test-user-2"));

    let workspaces =
        sqlx::query_scalar!("SELECT workspace_id FROM usr WHERE email = 'renamed@windmill.dev'")
            .fetch_all(&db)
            .await?;
    assert_eq!(workspaces, vec!["test-workspace".to_string()]);

    let owner = sqlx::query_scalar!("SELECT owner FROM workspace WHERE id = 'test-workspace'")
        .fetch_one(&db)
        .await?;
    assert_eq!(owner, "renamed@windmill.dev");

    // A `permissioned_as` (or app policy) holding the bare address is the sole identity reference
    // those rows have: left stale, the schedule tick and the deployed app run as an account that no
    // longer exists.
    let permissioned_as = sqlx::query_scalar!(
        "SELECT permissioned_as FROM schedule WHERE path = 'u/test-user-2/sched'"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(permissioned_as, "renamed@windmill.dev");

    let policy = sqlx::query_scalar!(
        "SELECT policy::text FROM app WHERE path = 'u/test-user-2/app' AND workspace_id = 'test-workspace'"
    )
    .fetch_one(&db)
    .await?
    .unwrap_or_default();
    assert!(
        !policy.contains("test2@windmill.dev") && policy.contains("renamed@windmill.dev"),
        "app policy should carry only the new address: {policy}"
    );

    // The rules keep their order, since the folder resolver takes the first glob that matches.
    let rules = sqlx::query_scalar!(
        "SELECT default_permissioned_as::text FROM folder WHERE workspace_id = 'test-workspace' AND name = 'fold'"
    )
    .fetch_one(&db)
    .await?
    .unwrap_or_default();
    let rules: serde_json::Value = serde_json::from_str(&rules)?;
    assert_eq!(rules[0]["permissioned_as"], "u/other");
    assert_eq!(rules[1]["permissioned_as"], "renamed@windmill.dev");

    let old_rows =
        sqlx::query_scalar!("SELECT COUNT(*) FROM password WHERE email = 'test2@windmill.dev'")
            .fetch_one(&db)
            .await?;
    assert_eq!(old_rows, Some(0));

    // Moving onto an address that already has an account would merge two identities. Login
    // lowercases what it is given, so a case-only difference collides just the same.
    let resp = change_email("test3@windmill.dev", "renamed@windmill.dev")
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    sqlx::query!(
        "UPDATE password SET email = 'Legacy@windmill.dev' WHERE email = 'renamed@windmill.dev'"
    )
    .execute(&db)
    .await?;
    let resp = change_email("test3@windmill.dev", "legacy@windmill.dev")
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    let resp = change_email("test3@windmill.dev", "not-an-email")
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    let resp = change_email("nobody@windmill.dev", "somebody@windmill.dev")
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    // Moving your own account would leave your cached identity pointing at a deleted address.
    let resp = change_email("test@windmill.dev", "self@windmill.dev")
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // Only super admins may move an account.
    let resp = client()
        .post(format!("{global_base}/change_email/test3@windmill.dev"))
        .header("Authorization", "Bearer SECRET_TOKEN_3")
        .json(&json!({ "new_email": "hijacked@windmill.dev" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    Ok(())
}

/// A superadmin acting outside every workspace has no username of their own, so their runnables
/// name them by their address — and an address may contain a `/`, which every reader of a
/// principal splits on. Moving such an account has to leave behind the form that decodes back to
/// them rather than one that reads as a group.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_change_user_email_to_slash_address(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let global_base = format!("http://localhost:{}/api/users", server.addr.port());

    sqlx::query!(
        "INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
         VALUES ('ext@windmill.dev', 'not-a-real-hash', 'password', true, true, 'Ext')"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "INSERT INTO script (workspace_id, path, hash, content, summary, description, language, created_by, created_at, on_behalf_of)
         VALUES ('test-workspace', 'u/test-user/s', 93001, 'def main(): pass', '', '', 'python3', 'test-user', NOW(), 'ext@windmill.dev')"
    )
    .execute(&db)
    .await?;

    // The principal follows the address, and a job row carries it in a narrower column than the
    // runnable does, so a move that would make it unenqueueable is refused rather than silently
    // leaving runnables that look configured and cannot start.
    let resp = authed(client().post(format!("{global_base}/change_email/ext@windmill.dev")))
        .json(&json!({ "new_email": "a-very-long-superadmin-address-for-this-test@windmill.dev" }))
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("characters a job can carry"), "{body}");

    let resp = authed(client().post(format!("{global_base}/change_email/ext@windmill.dev")))
        .json(&json!({ "new_email": "ops/alice@windmill.dev" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "change_email: {}", resp.text().await?);

    assert_eq!(
        sqlx::query_scalar!(
            "SELECT on_behalf_of FROM script WHERE path = 'u/test-user/s' AND workspace_id = 'test-workspace'"
        )
        .fetch_one(&db)
        .await?
        .as_deref(),
        Some("u/ops/alice@windmill.dev"),
        "left bare, the new address would come back as group 'alice@windmill.dev'"
    );

    Ok(())
}

/// A group's synthetic address (`group-{name}@windmill.dev`) can also be a real user's, and a
/// runnable configured for the *group* carries that address next to `g/{name}`. Moving the
/// colliding user's account must leave it alone: rewriting one half of the pair would leave it
/// naming two different people, which the deploy path rejects.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_change_user_email_leaves_group_identities(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let global_base = format!("http://localhost:{}/api/users", server.addr.port());

    sqlx::query!(
        "UPDATE password SET email = 'group-ops@windmill.dev' WHERE email = 'test2@windmill.dev'"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "UPDATE usr SET email = 'group-ops@windmill.dev' WHERE email = 'test2@windmill.dev'"
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "INSERT INTO group_(workspace_id, name, summary, extra_perms) VALUES ('test-workspace', 'ops', '', '{}')"
    )
    .execute(&db)
    .await?;

    // An email change never moves a username, so the principal these rows hold stays put. What
    // moves is the address beside it — kept for the workers that still read it — and in every
    // pair a group-owned identity keeps the group's synthetic address even though a real account
    // now holds it: rewriting one half leaves the pair naming two accounts.
    sqlx::query!(
        "INSERT INTO app(workspace_id, path, summary, policy, versions)
         VALUES ('test-workspace', 'u/test-user/g', '', '{\"on_behalf_of\": \"g/ops\", \"on_behalf_of_email\": \"group-ops@windmill.dev\"}'::jsonb, '{}'),
                ('test-workspace', 'u/test-user/u', '', '{\"on_behalf_of\": \"u/test-user-2\", \"on_behalf_of_email\": \"group-ops@windmill.dev\"}'::jsonb, '{}')"
    )
    .execute(&db)
    .await?;

    sqlx::query!(
        "INSERT INTO script (workspace_id, path, hash, content, summary, description, language, created_by, created_at, on_behalf_of, on_behalf_of_email)
         VALUES ('test-workspace', 'u/test-user/sg', 95001, 'def main(): pass', '', '', 'python3', 'test-user', NOW(), 'g/ops', 'group-ops@windmill.dev'),
                ('test-workspace', 'u/test-user/su', 95002, 'def main(): pass', '', '', 'python3', 'test-user', NOW(), 'u/test-user-2', 'group-ops@windmill.dev')"
    )
    .execute(&db)
    .await?;

    sqlx::query!(
        "INSERT INTO draft(workspace_id, path, typ, value, email)
         VALUES ('test-workspace', 'u/test-user/dg', 'script', '{\"on_behalf_of\": \"g/ops\", \"on_behalf_of_email\": \"group-ops@windmill.dev\"}'::json, 'test@windmill.dev'),
                ('test-workspace', 'u/test-user/du', 'script', '{\"on_behalf_of\": \"u/test-user-2\", \"on_behalf_of_email\": \"group-ops@windmill.dev\"}'::json, 'test@windmill.dev')"
    )
    .execute(&db)
    .await?;

    let resp = authed(client().post(format!("{global_base}/change_email/group-ops@windmill.dev")))
        .json(&json!({ "new_email": "renamed@windmill.dev" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "change_email: {}", resp.text().await?);

    let apps = sqlx::query!(
        "SELECT path, policy->>'on_behalf_of_email' AS email FROM app WHERE workspace_id = 'test-workspace' ORDER BY path"
    )
    .fetch_all(&db)
    .await?;
    assert_eq!(
        apps.iter()
            .map(|r| (r.path.as_str(), r.email.as_deref()))
            .collect::<Vec<_>>(),
        vec![
            ("u/test-user/g", Some("group-ops@windmill.dev")),
            ("u/test-user/u", Some("renamed@windmill.dev")),
        ],
        "the group-owned app keeps the group's address; the user-owned one moves"
    );

    let scripts = sqlx::query!(
        "SELECT path, on_behalf_of_email AS email FROM script WHERE workspace_id = 'test-workspace' AND path LIKE 'u/test-user/s%' ORDER BY path"
    )
    .fetch_all(&db)
    .await?;
    assert_eq!(
        scripts
            .iter()
            .map(|r| (r.path.as_str(), r.email.as_deref()))
            .collect::<Vec<_>>(),
        vec![
            ("u/test-user/sg", Some("group-ops@windmill.dev")),
            ("u/test-user/su", Some("renamed@windmill.dev")),
        ],
        "the group-owned script keeps the group's address; the user-owned one moves"
    );

    let drafts = sqlx::query!(
        "SELECT path, value->>'on_behalf_of_email' AS email, value->>'on_behalf_of' AS principal FROM draft WHERE workspace_id = 'test-workspace' ORDER BY path"
    )
    .fetch_all(&db)
    .await?;
    assert_eq!(
        drafts
            .iter()
            .map(|r| (r.path.as_str(), r.principal.as_deref(), r.email.as_deref()))
            .collect::<Vec<_>>(),
        vec![
            (
                "u/test-user/dg",
                Some("g/ops"),
                Some("group-ops@windmill.dev")
            ),
            (
                "u/test-user/du",
                Some("u/test-user-2"),
                Some("renamed@windmill.dev")
            ),
        ],
        "a draft's pair moves as a whole or not at all — either half left behind is a 400 on deploy"
    );

    Ok(())
}

/// An address with no `password` row can own a draft, and the account paths carry the delete and
/// rename that no foreign key does any more.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_drafts_follow_their_owner_without_a_fkey(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let global_base = format!("http://localhost:{port}/api/users");

    // The destination of the rename below already holds a draft of the same item — it belongs to
    // an accountless principal, so `change_email`'s "address is free" check does not see it.
    sqlx::query!(
        "INSERT INTO draft(workspace_id, path, typ, value, email) VALUES
            ('test-workspace', 'u/ext/s', 'script', '{}'::json, 'ext-jwt@windmill.dev'),
            ('test-workspace', 'u/two/s', 'script', '{\"summary\": \"moving\"}'::json, 'test2@windmill.dev'),
            ('test-workspace', 'u/two/s', 'script', '{\"summary\": \"displaced\"}'::json, 'renamed@windmill.dev'),
            ('test-workspace', 'u/three/s', 'script', '{}'::json, 'test3@windmill.dev')"
    )
    .execute(&db)
    .await?;

    // A null username is how the legacy workspace-level row is encoded, so an owner nobody can
    // name must be absent from the owner circles rather than pose as one.
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/drafts/list?all_users=true"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    let listed = resp.json::<serde_json::Value>().await?;
    let ext = listed
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["path"] == "u/ext/s")
        .expect("the accountless owner's draft is listed");
    assert_eq!(ext.get("draft_users"), None);

    let resp = authed(client().post(format!("{global_base}/change_email/test2@windmill.dev")))
        .json(&json!({ "new_email": "renamed@windmill.dev" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "change_email: {}", resp.text().await?);
    let moved = sqlx::query!(
        "SELECT email, value->>'summary' AS summary FROM draft WHERE path = 'u/two/s'"
    )
    .fetch_all(&db)
    .await?;
    assert_eq!(
        moved
            .iter()
            .map(|r| (r.email.as_deref(), r.summary.as_deref()))
            .collect::<Vec<_>>(),
        vec![(Some("renamed@windmill.dev"), Some("moving"))],
        "the moving account's draft wins the unique index it now collides on"
    );

    let resp = authed(client().delete(format!("{global_base}/delete/test3@windmill.dev")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "delete_user: {}", resp.text().await?);
    let remaining = sqlx::query_scalar!("SELECT path FROM draft ORDER BY path")
        .fetch_all(&db)
        .await?;
    assert_eq!(
        remaining,
        vec!["u/ext/s".to_string(), "u/two/s".to_string()],
        "the deleted account's draft goes, the accountless owner's stays"
    );

    Ok(())
}
