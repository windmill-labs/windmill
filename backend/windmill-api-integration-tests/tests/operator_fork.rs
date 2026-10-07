//! Its own test binary: the protection-rules cache is process-wide and keyed by workspace id, so a
//! sibling test forking its own `test-workspace` can cache an empty ruleset list over this one's.
use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

/// Operators cannot fork until a ruleset carries `AllowOperatorForking`, and the fork keeps them an
/// operator rather than handing them the developer role the bare creator row would default to.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_operator_fork_needs_ruleset_and_stays_operator(
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
        .post(format!("{base_url}/protection_rules"))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({
            "name": "ops-can-fork",
            "rules": ["AllowOperatorForking"],
            "bypass_groups": [],
            "bypass_users": []
        }))
        .send()
        .await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);

    let resp = fork().await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);

    let (operator, is_admin): (bool, bool) = sqlx::query_as(
        "SELECT operator, is_admin FROM usr WHERE workspace_id = 'wm-fork-op' AND username = 'test-user-3'",
    )
    .fetch_one(&db)
    .await?;
    assert!(operator && !is_admin);

    Ok(())
}
