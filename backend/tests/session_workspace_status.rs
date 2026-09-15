//! `POST /workspaces/session_workspace_status` is what the client uses to decide whether to
//! keep or destroy an AI session, so its notion of "reachable" must match what the authed
//! extractor actually grants. Membership is not the only path: a superadmin is authed into
//! any existing workspace without a `usr` row, and `admins` has no `usr` rows at all, so
//! answering from `usr` alone reports live workspaces as unresolvable and the client deletes
//! sessions that still work.

use serde_json::json;
use sqlx::{Pool, Postgres};
use std::collections::HashMap;
use windmill_test_utils::*;

async fn status(port: u16, token: &str, ids: &[&str]) -> anyhow::Result<HashMap<String, String>> {
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
