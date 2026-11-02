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

/// A forker joins the fork with the role they hold in the parent, unless a ruleset of the parent
/// maps it; a role without `fork` cannot fork at all, and the fork's owner cannot seat a member
/// as a developer the mapping makes something else. EE: CE caps the workspace count below the
/// forks this creates.
#[cfg(feature = "enterprise")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_fork_carries_role_and_mapping(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    // Not `test-workspace`: the role and ruleset caches are process-wide and keyed by workspace
    // id, so the other test here, running concurrently on its own database, would fill them.
    sqlx::raw_sql(
        "INSERT INTO workspace (id, name, owner) VALUES ('rf-ws', 'rf-ws', 'test-user');
         INSERT INTO workspace_settings (workspace_id) VALUES ('rf-ws');
         INSERT INTO workspace_key (workspace_id, kind, key) VALUES ('rf-ws', 'cloud', 'rf-key');
         INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
             ('rf-ws', 'all', 'All users', '{}');
         INSERT INTO usr (workspace_id, email, username, is_admin, operator, role) VALUES
             ('rf-ws', 'test@windmill.dev', 'test-user', true, false, 'Admin'),
             ('rf-ws', 'test2@windmill.dev', 'test-user-2', false, true, 'User'),
             ('rf-ws', 'test3@windmill.dev', 'test-user-3', false, false, 'User')",
    )
    .execute(&db)
    .await?;
    windmill_common::roles::invalidate_all_roles_cache();
    windmill_common::workspaces::invalidate_protection_rules_cache("rf-ws");

    let server = ApiServer::start(db.clone()).await?;
    let base_url = format!(
        "http://localhost:{}/api/w/rf-ws/workspaces",
        server.addr.port()
    );
    let client = reqwest::Client::new();
    let fork = |id: &str| {
        client
            .post(format!("{base_url}/create_fork"))
            .header("Authorization", "Bearer SECRET_TOKEN_2")
            .json(&json!({ "id": id, "name": id }))
            .send()
    };
    let fork_role = |id: &'static str| {
        sqlx::query_as::<_, (Option<String>, bool)>(
            "SELECT workspace_role, operator FROM usr WHERE workspace_id = $1 AND username = 'test-user-2'",
        )
        .bind(id)
        .fetch_one(&db)
    };

    let resp = fork("wm-fork-refused").await?;
    assert_eq!(resp.status(), 403, "operator lacks fork: {}", resp.text().await?);

    // `code` makes the copy write `operator = false`: the legacy row's NULL role must not then
    // read as developer.
    sqlx::query(
        r#"UPDATE workspace_settings SET role_overrides = '{"operator": {"capabilities": {"fork": true, "code": true}}}'
           WHERE workspace_id = 'rf-ws'"#,
    )
    .execute(&db)
    .await?;
    windmill_common::roles::invalidate_all_roles_cache();

    let resp = fork("wm-fork-kept").await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);
    assert_eq!(
        fork_role("wm-fork-kept").await?,
        (Some("operator".to_string()), false),
        "stays an operator"
    );

    let resp = client
        .post(format!("{base_url}/protection_rules"))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({
            "name": "forks", "rules": [], "bypass_groups": [], "bypass_users": [],
            "fork_role_mapping": { "operator": "developer", "developer": "operator" }
        }))
        .send()
        .await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);

    let resp = fork("wm-fork-mapped").await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);
    assert_eq!(
        fork_role("wm-fork-mapped").await?,
        (Some("developer".to_string()), false),
        "mapped to developer"
    );

    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{}/api/w/wm-fork-mapped/workspaces/add_user",
            server.addr.port()
        ))
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .json(&json!({ "email": "test3@windmill.dev", "is_admin": false, "operator": false }))
        .send()
        .await?;
    let (status, text) = (resp.status(), resp.text().await?);
    assert!(
        status == 403 && text.contains("role in its forks"),
        "a developer mapped to operator: {status} {text}"
    );

    Ok(())
}
