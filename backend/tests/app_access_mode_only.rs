//! `EditApp.execution_mode`: changing who may open an app without sending its policy.
//!
//! The change is applied to the deployed policy under the row lock a deploy takes, so it keeps
//! every other field a concurrent deploy wrote. It is held to the rules a policy write is held
//! to, and it is refused to a caller who could not keep the app's run-as user, since a policy
//! write from them would rebind it.
//!
//! One test: the protection rules and the workspace roles are cached per workspace id across
//! the process, and every test here uses `test-workspace`.
//!
//! Users from the `base` fixture:
//!   test-user   (admin,     token SECRET_TOKEN)
//!   test-user-2 (non-admin, token SECRET_TOKEN_2)
//!   test-user-3 (made a compose-only operator below, token SECRET_TOKEN_3)

use reqwest::multipart;
use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_common::workspaces::invalidate_protection_rules_cache;
use windmill_test_utils::*;

const ADMIN: &str = "SECRET_TOKEN";
const USER: &str = "SECRET_TOKEN_2";
const BUILDER: &str = "SECRET_TOKEN_3";

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

async fn update(ws: &str, path: &str, token: &str, body: Value) -> anyhow::Result<(u16, String)> {
    let resp = client()
        .post(format!("{ws}/apps/update/{path}"))
        .bearer_auth(token)
        .json(&body)
        .send()
        .await?;
    Ok((resp.status().as_u16(), resp.text().await?))
}

async fn create(ws: &str, token: &str, path: &str, policy: Value) -> anyhow::Result<()> {
    let resp = client()
        .post(format!("{ws}/apps/create"))
        .bearer_auth(token)
        .json(&json!({ "path": path, "summary": "", "value": {}, "policy": policy }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "create {path}: {}", resp.text().await?);
    Ok(())
}

/// An anonymous, sandboxed raw app with no runnables, as a builder may deploy one.
async fn create_raw(ws: &str, token: &str, path: &str) -> anyhow::Result<()> {
    let app = json!({
        "path": path,
        "summary": "",
        "value": { "files": {}, "runnables": {} },
        "policy": { "execution_mode": "anonymous", "sandbox": true, "triggerables_v2": {} },
    });
    let form = multipart::Form::new()
        .part(
            "app",
            multipart::Part::text(app.to_string()).mime_str("application/json")?,
        )
        .part("js", multipart::Part::text("console.log('app')"));
    let resp = client()
        .post(format!("{ws}/apps/create_raw"))
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        201,
        "create_raw {path}: {}",
        resp.text().await?
    );
    Ok(())
}

async fn policy(db: &Pool<Postgres>, path: &str) -> anyhow::Result<Value> {
    Ok(sqlx::query_scalar(
        "SELECT policy FROM app WHERE path = $1 AND workspace_id = 'test-workspace'",
    )
    .bind(path)
    .fetch_one(db)
    .await?)
}

fn triggerable(key: &str) -> Value {
    json!({ key: { "static_inputs": {}, "one_of_inputs": {} } })
}

#[sqlx::test(fixtures("base"))]
async fn test_access_mode_only_update(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    invalidate_protection_rules_cache("test-workspace");
    let server = ApiServer::start(db.clone()).await?;
    let ws = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );

    // ========================================
    // 1. The mode alone changes; every other field of the deployed policy stays.
    // ========================================
    const SHARED: &str = "u/test-user-2/shared";
    create(
        &ws,
        ADMIN,
        SHARED,
        json!({
            "execution_mode": "publisher",
            "triggerables_v2": triggerable("c:script/u/test-user/noop"),
            "sandbox": true,
            "allowed_s3_keys": [{ "s3_path": "a/b.txt" }],
        }),
    )
    .await?;
    let before = policy(&db, SHARED).await?;
    assert_eq!(before["on_behalf_of"], "u/test-user");

    let (status, body) =
        update(&ws, SHARED, ADMIN, json!({ "execution_mode": "anonymous" })).await?;
    assert_eq!(status, 200, "{body}");
    let mut after = policy(&db, SHARED).await?;
    assert_eq!(after["execution_mode"], "anonymous");
    after["execution_mode"] = before["execution_mode"].clone();
    assert_eq!(
        after, before,
        "a mode-only change must keep every other policy field"
    );

    // Sending a policy too is ambiguous about which mode wins.
    let (status, _) = update(
        &ws,
        SHARED,
        ADMIN,
        json!({ "execution_mode": "publisher", "policy": { "execution_mode": "anonymous" } }),
    )
    .await?;
    assert_eq!(status, 400);

    // ========================================
    // 2. Someone who cannot keep the run-as user cannot change the mode of an app that runs
    //    as someone else, even with write access to it.
    // ========================================
    let (status, body) =
        update(&ws, SHARED, USER, json!({ "execution_mode": "publisher" })).await?;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("wm_deployers"), "{body}");
    assert_eq!(policy(&db, SHARED).await?["execution_mode"], "anonymous");

    // ========================================
    // 3. A policy without a mode keeps the deployed one.
    // ========================================
    const OWN: &str = "u/test-user-2/own";
    create(&ws, USER, OWN, json!({ "execution_mode": "publisher" })).await?;
    sqlx::query(
        "INSERT INTO draft (workspace_id, path, typ, value, email)
         VALUES ('test-workspace', $1, 'app', '{}', 'test2@windmill.dev')",
    )
    .bind(OWN)
    .execute(&db)
    .await?;
    let (status, body) = update(&ws, OWN, USER, json!({ "execution_mode": "anonymous" })).await?;
    assert_eq!(status, 200, "{body}");
    let drafts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM draft WHERE workspace_id = 'test-workspace' AND path = $1",
    )
    .bind(OWN)
    .fetch_one(&db)
    .await?;
    assert_eq!(drafts, 1, "a mode-only change must keep the caller's draft");
    let (status, body) = update(
        &ws,
        OWN,
        USER,
        json!({ "value": { "v": 2 }, "policy": { "triggerables_v2": triggerable("c:script/f/x/new") } }),
    )
    .await?;
    assert_eq!(status, 200, "{body}");
    let p = policy(&db, OWN).await?;
    assert_eq!(p["execution_mode"], "anonymous");
    assert!(
        p["triggerables_v2"].get("c:script/f/x/new").is_some(),
        "{p}"
    );

    // ========================================
    // 4. A compose-only operator: an omitted mode inherits the deployed one unless it is Viewer.
    // ========================================
    sqlx::query("UPDATE usr SET operator = true WHERE workspace_id = 'test-workspace' AND username = 'test-user-3'")
        .execute(&db)
        .await?;
    sqlx::query(
        r#"UPDATE workspace_settings SET role_overrides = '{"operator": {"capabilities": {"apps": "compose"}}}'
           WHERE workspace_id = 'test-workspace'"#,
    )
    .execute(&db)
    .await?;
    // In wm_deployers, so the refusal in the last step below holds for a builder who may keep
    // another user's identity on deploy.
    sqlx::query(
        "INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES ('test-workspace', 'wm_deployers', '', '{}')",
    )
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO usr_to_group (workspace_id, group_, usr) VALUES ('test-workspace', 'wm_deployers', 'test-user-3')",
    )
    .execute(&db)
    .await?;
    windmill_common::roles::invalidate_workspace_roles_cache("test-workspace");
    windmill_common::roles::invalidate_member_role_cache("test-workspace", "test-user-3");

    const BUILT: &str = "u/test-user-3/built";
    create_raw(&ws, BUILDER, BUILT).await?;

    let settings_only = json!({ "policy": { "sandbox": true, "triggerables_v2": {} } });
    let (status, body) = update(&ws, BUILT, BUILDER, settings_only.clone()).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        policy(&db, BUILT).await?["execution_mode"],
        "anonymous",
        "a builder's policy without a mode must keep a non-Viewer deployed mode"
    );

    // Viewer stays refused to a builder, alone as in a policy.
    let (status, body) = update(&ws, BUILT, BUILDER, json!({ "execution_mode": "viewer" })).await?;
    assert_eq!(status, 403, "{body}");

    // An admin may put the builder's app in Viewer; the builder's next policy without a mode
    // then cannot inherit it.
    let (status, body) = update(&ws, BUILT, ADMIN, json!({ "execution_mode": "viewer" })).await?;
    assert_eq!(status, 200, "{body}");
    let (status, body) = update(&ws, BUILT, BUILDER, settings_only).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(policy(&db, BUILT).await?["execution_mode"], "publisher");

    let (status, body) = update(
        &ws,
        BUILT,
        BUILDER,
        json!({ "execution_mode": "anonymous" }),
    )
    .await?;
    assert_eq!(
        status, 200,
        "a builder may change the mode of its own app: {body}"
    );
    assert_eq!(policy(&db, BUILT).await?["execution_mode"], "anonymous");

    // Changing the mode alone skips the check of the triggerables against the builder's RLS, so
    // it stays refused on an app that runs as someone else; deploying it is the way.
    const ADMIN_RUN: &str = "u/test-user-3/admin_run";
    create_raw(&ws, ADMIN, ADMIN_RUN).await?;
    let (status, body) = update(
        &ws,
        ADMIN_RUN,
        BUILDER,
        json!({ "execution_mode": "publisher" }),
    )
    .await?;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("by deploying it"), "{body}");

    // ========================================
    // 5. Widening is gated by the protection rule; narrowing is not.
    // ========================================
    let resp = client()
        .post(format!("{ws}/workspaces/protection_rules"))
        .bearer_auth(ADMIN)
        .json(&json!({
            "name": "no-public-apps",
            "rules": ["RestrictAnonymousAppDeployment"],
            "bypass_users": [],
            "bypass_groups": []
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    let (status, body) = update(&ws, OWN, USER, json!({ "execution_mode": "publisher" })).await?;
    assert_eq!(status, 200, "narrowing stays open: {body}");
    let (status, body) = update(&ws, OWN, USER, json!({ "execution_mode": "anonymous" })).await?;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("no-public-apps"), "{body}");
    assert_eq!(policy(&db, OWN).await?["execution_mode"], "publisher");

    Ok(())
}
