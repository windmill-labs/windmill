//! `POST /workspaces/session_workspace_status` is what the client uses to decide whether to
//! keep or destroy an AI session, so its notion of "reachable" must match what the authed
//! extractor actually grants. Membership is not the only path: a superadmin is authed into
//! any existing workspace without a `usr` row, and `admins` has no `usr` rows at all, so
//! answering from `usr` alone reports live workspaces as unresolvable and the client deletes
//! sessions that still work.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use std::collections::HashMap;
use windmill_test_utils::*;

async fn status(port: u16, token: &str, ids: &[&str]) -> anyhow::Result<HashMap<String, Value>> {
    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/workspaces/session_workspace_status"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "workspace_ids": ids }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    Ok(resp.json().await?)
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

    // The browser sweeps its sessions by the retention the answer carries, which a caller
    // who cannot reach the workspace is not told.
    sqlx::query(
        "UPDATE workspace_settings SET ai_config = '{\"sessions_retention_days\": 7}' \
         WHERE workspace_id IN ('test-workspace', 'foreign-workspace')",
    )
    .execute(&db)
    .await?;

    // test@windmill.dev is an instance superadmin, and a member of test-workspace only.
    let sa = status(port, "SECRET_TOKEN", &ids).await?;
    assert_eq!(sa["admins"]["status"], "active");
    assert_eq!(sa["foreign-workspace"]["status"], "active");
    assert_eq!(sa["foreign-workspace"]["sessions_retention_days"], 7);
    // Reachable, but soft-deleted: superadmins must not bypass the archived state.
    assert_eq!(sa["archived-workspace"]["status"], "archived");
    assert_eq!(sa["test-workspace"]["status"], "active");
    // A workspace that never existed stays unresolvable — the superadmin arm must not
    // swallow the hard-deleted case, or those sessions would linger forever.
    assert_eq!(sa["no-such-workspace"]["status"], "deleted");

    // test2@windmill.dev is not a superadmin, and a member of test-workspace only.
    let usr = status(port, "SECRET_TOKEN_2", &ids).await?;
    assert_eq!(usr["admins"]["status"], "deleted");
    assert_eq!(usr["foreign-workspace"]["status"], "deleted");
    assert!(usr["foreign-workspace"]
        .get("sessions_retention_days")
        .is_none());
    assert_eq!(usr["archived-workspace"]["status"], "deleted");
    assert_eq!(usr["test-workspace"]["status"], "active");
    assert_eq!(usr["test-workspace"]["sessions_retention_days"], 7);
    assert_eq!(usr["no-such-workspace"]["status"], "deleted");

    // A disabled membership still reconciles its sessions, which is why the status stays
    // `active`, but it cannot authenticate into the workspace, so it is told no setting.
    sqlx::query("UPDATE usr SET disabled = true WHERE workspace_id = 'test-workspace'")
        .execute(&db)
        .await?;
    let off = status(port, "SECRET_TOKEN_2", &ids).await?;
    assert_eq!(off["test-workspace"]["status"], "active");
    assert!(off["test-workspace"]
        .get("sessions_retention_days")
        .is_none());

    Ok(())
}
