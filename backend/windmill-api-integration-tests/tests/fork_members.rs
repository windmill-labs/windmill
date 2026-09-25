use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

/// With `add_admins_and_developers_to_forks` on, a fork starts with the parent's admins and
/// developers at their parent role, even when a developer forks it; operators are left out. The
/// copies are manual members: a parent membership that came from an instance group must not carry
/// that provenance into a fork that does not configure the group.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_fork_adds_parent_admins_and_developers(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base_url = format!(
        "http://localhost:{}/api/w/test-workspace/workspaces",
        server.addr.port()
    );
    let client = reqwest::Client::new();

    sqlx::query(
        "UPDATE usr SET operator = true WHERE workspace_id = 'test-workspace' AND username = 'test-user-3'",
    )
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO usr (workspace_id, email, username, is_admin, added_via)
         VALUES ('test-workspace', 'test4@windmill.dev', 'test-user-4', false,
                 '{\"source\": \"instance_group\", \"group\": \"devs\"}')",
    )
    .execute(&db)
    .await?;

    let resp = client
        .post(format!(
            "{base_url}/edit_add_admins_and_developers_to_forks"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({ "add_admins_and_developers_to_forks": true }))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "enabling the setting: {}",
        resp.text().await?
    );

    let resp = client
        .post(format!("{base_url}/create_fork"))
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .json(&json!({ "id": "wm-fork-team", "name": "Team fork" }))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "creating the fork: {}",
        resp.text().await?
    );

    let members: Vec<(String, bool, bool)> = sqlx::query_as(
        "SELECT username, is_admin, added_via IS NULL FROM usr
         WHERE workspace_id = 'wm-fork-team' ORDER BY username",
    )
    .fetch_all(&db)
    .await?;
    assert_eq!(
        members,
        vec![
            ("test-user".to_string(), true, true),
            ("test-user-2".to_string(), false, true),
            ("test-user-4".to_string(), false, true),
        ]
    );

    Ok(())
}
