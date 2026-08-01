use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

/// A principal only means something in the workspace whose `usr`/`group_` rows define it, and a
/// fork copies the creator and the groups but not the rest of the membership. Carrying one over
/// blindly would leave a runnable naming somebody who cannot authenticate there; dropping them
/// all would silently hand every configured runnable in the fork to whoever runs it.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_fork_keeps_only_resolvable_on_behalf_of(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base_url = format!("http://localhost:{port}/api");

    // A superadmin with no `usr` row anywhere: they authenticate from `password` alone, so
    // their principal resolves in the fork as much as it did in the parent.
    sqlx::query!(
        "INSERT INTO password(email, password_hash, login_type, super_admin, verified, name, username)
         VALUES ('sa@windmill.dev', 'not-a-real-hash', 'password', true, true, 'Ext', 'ext-sa')"
    )
    .execute(&db)
    .await?;

    sqlx::query!(
        "INSERT INTO script (workspace_id, path, hash, content, summary, description, language, created_by, created_at, on_behalf_of_permissioned_as)
         VALUES
         ('test-workspace', 'u/test-user/obo_member', 91001, 'def main(): pass', '', '', 'python3', 'test-user', NOW(), 'u/test-user'),
         ('test-workspace', 'u/test-user/obo_stranger', 91002, 'def main(): pass', '', '', 'python3', 'test-user', NOW(), 'u/test-user-2'),
         ('test-workspace', 'u/test-user/obo_group', 91003, 'def main(): pass', '', '', 'python3', 'test-user', NOW(), 'g/all'),
         ('test-workspace', 'u/test-user/obo_superadmin', 91004, 'def main(): pass', '', '', 'python3', 'test-user', NOW(), 'u/ext-sa')"
    )
    .execute(&db)
    .await?;

    sqlx::query!(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at, on_behalf_of_permissioned_as)
         VALUES ('test-workspace', 'u/test-user/obo_flow', '', '', $1, 'test-user', NOW(), 'u/test-user-2')",
        json!({"modules": []})
    )
    .execute(&db)
    .await?;

    let resp = reqwest::Client::new()
        .post(format!(
            "{base_url}/w/test-workspace/workspaces/create_fork"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({ "id": "wm-fork-obo", "name": "Fork", "color": "#0000ff" }))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "creating the fork: {}",
        resp.text().await?
    );

    let cloned = sqlx::query!(
        "SELECT path, on_behalf_of_permissioned_as FROM script WHERE workspace_id = 'wm-fork-obo' ORDER BY path"
    )
    .fetch_all(&db)
    .await?;
    let identity = |path: &str| {
        cloned
            .iter()
            .find(|r| r.path == path)
            .unwrap_or_else(|| panic!("{path} was cloned"))
            .on_behalf_of_permissioned_as
            .clone()
    };

    // The creator is the one member the fork always gets, so their principal still resolves.
    assert_eq!(
        identity("u/test-user/obo_member").as_deref(),
        Some("u/test-user")
    );
    // Groups are cloned wholesale, so a group principal resolves too.
    assert_eq!(identity("u/test-user/obo_group").as_deref(), Some("g/all"));
    assert_eq!(
        identity("u/test-user/obo_superadmin").as_deref(),
        Some("u/ext-sa")
    );
    // `test-user-2` is not carried into the fork, so nothing there can run as them.
    assert_eq!(identity("u/test-user/obo_stranger"), None);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT on_behalf_of_permissioned_as FROM flow WHERE workspace_id = 'wm-fork-obo'"
        )
        .fetch_one(&db)
        .await?,
        None
    );

    Ok(())
}
