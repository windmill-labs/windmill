//! Its own test binary: the roles cache is process-wide and keyed by workspace id, so a sibling
//! test overriding roles in its own `test-workspace` could leave this one reading stale ones.
use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

/// An operator cannot fork until their role is granted `fork`, and the fork they then create makes
/// them a developer, not an operator.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_operator_fork_needs_capability_and_develops_in_fork(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
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

    let fork = || {
        client
            .post(format!("{base_url}/create_fork"))
            .header("Authorization", "Bearer SECRET_TOKEN_3")
            .json(&json!({ "id": "wm-fork-op", "name": "Operator fork" }))
            .send()
    };

    let resp = fork().await?;
    assert_eq!(resp.status(), 403, "{}", resp.text().await?);

    let resp = client
        .post(format!("{base_url}/role_overrides"))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({ "operator": { "capabilities": { "fork": true } } }))
        .send()
        .await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);

    let resp = fork().await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);

    let (operator, is_admin, role): (bool, bool, Option<String>) = sqlx::query_as(
        "SELECT operator, is_admin, workspace_role FROM usr WHERE workspace_id = 'wm-fork-op' AND username = 'test-user-3'",
    )
    .fetch_one(&db)
    .await?;
    assert!(!operator && !is_admin && role.is_none());

    Ok(())
}
