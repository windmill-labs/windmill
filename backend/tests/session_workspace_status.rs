//! `POST /workspaces/session_workspace_status` is what the client uses to decide whether to
//! keep or destroy an AI session, so its notion of "reachable" must match what the authed
//! extractor actually grants. Membership is not the only path: a superadmin is authed into
//! any existing workspace without a `usr` row, and `admins` has no `usr` rows at all, so
//! answering from `usr` alone reports live workspaces as unresolvable and the client deletes
//! sessions that still work. `POST /workspaces/session_workspace_retention`, the AI session
//! retention the same client deletes its own copies by, is a workspace setting and answers to
//! the stricter bar, which is why the two are separate routes and tested together.

use serde_json::json;
use sqlx::{Pool, Postgres};
use std::collections::HashMap;
use windmill_test_utils::*;

async fn post<T: serde::de::DeserializeOwned>(
    port: u16,
    route: &str,
    token: &str,
    ids: &[&str],
) -> anyhow::Result<T> {
    let resp = reqwest::Client::new()
        .post(format!("http://localhost:{port}/api/workspaces/{route}"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "workspace_ids": ids }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    Ok(resp.json().await?)
}

async fn status(port: u16, token: &str, ids: &[&str]) -> anyhow::Result<HashMap<String, String>> {
    post(port, "session_workspace_status", token, ids).await
}

async fn retention(port: u16, token: &str, ids: &[&str]) -> anyhow::Result<HashMap<String, u32>> {
    post(port, "session_workspace_retention", token, ids).await
}

#[sqlx::test(fixtures("base", "session_workspace_status"))]
async fn test_superadmin_reaches_workspaces_without_a_usr_row(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let ids = [
        "admins",
        "foreign-workspace",
        "archived-workspace",
        "test-workspace",
        "no-such-workspace",
    ];

    // test@windmill.dev is an instance superadmin, and a member of test-workspace only.
    let sa = status(port, "SECRET_TOKEN", &ids).await?;
    assert_eq!(sa["admins"], "active");
    assert_eq!(sa["foreign-workspace"], "active");
    // Reachable, but soft-deleted: superadmins must not bypass the archived state.
    assert_eq!(sa["archived-workspace"], "archived");
    assert_eq!(sa["test-workspace"], "active");
    // A workspace that never existed stays unresolvable — the superadmin arm must not
    // swallow the hard-deleted case, or those sessions would linger forever.
    assert_eq!(sa["no-such-workspace"], "deleted");

    // test2@windmill.dev is not a superadmin, and a member of test-workspace only.
    let usr = status(port, "SECRET_TOKEN_2", &ids).await?;
    assert_eq!(usr["admins"], "deleted");
    assert_eq!(usr["foreign-workspace"], "deleted");
    assert_eq!(usr["archived-workspace"], "deleted");
    assert_eq!(usr["test-workspace"], "active");
    assert_eq!(usr["no-such-workspace"], "deleted");

    Ok(())
}

/// The retention a browser deletes its own copies by is a workspace setting, so unlike the
/// status it is told only to a caller the authed extractor would let in.
#[sqlx::test(fixtures("base", "session_workspace_status"))]
async fn test_session_retention_is_told_only_to_members_who_can_be_authed(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let ids = ["foreign-workspace", "test-workspace", "no-such-workspace"];
    sqlx::query(
        "UPDATE workspace_settings SET ai_config = '{\"sessions_retention_days\": 7}' \
         WHERE workspace_id IN ('test-workspace', 'foreign-workspace')",
    )
    .execute(&db)
    .await?;

    // test@windmill.dev is a superadmin: authed into every workspace that exists.
    let sa = retention(port, "SECRET_TOKEN", &ids).await?;
    assert_eq!(sa["test-workspace"], 7);
    assert_eq!(sa["foreign-workspace"], 7);
    assert!(!sa.contains_key("no-such-workspace"));

    // test2@windmill.dev is a member of test-workspace only.
    let usr = retention(port, "SECRET_TOKEN_2", &ids).await?;
    assert_eq!(usr["test-workspace"], 7);
    assert!(!usr.contains_key("foreign-workspace"));

    // A disabled membership still reconciles its sessions — the status stays `active` — but
    // cannot be authed into the workspace, so it is told no setting.
    sqlx::query("UPDATE usr SET disabled = true WHERE workspace_id = 'test-workspace'")
        .execute(&db)
        .await?;
    assert_eq!(
        status(port, "SECRET_TOKEN_2", &ids).await?["test-workspace"],
        "active"
    );
    assert!(!retention(port, "SECRET_TOKEN_2", &ids)
        .await?
        .contains_key("test-workspace"));

    Ok(())
}
