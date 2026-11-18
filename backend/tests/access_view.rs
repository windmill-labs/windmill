//! Another member's access lists paths the caller may not be able to read, so only a workspace
//! admin may ask for it.

use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

#[sqlx::test(fixtures("base"))]
async fn test_member_cannot_read_another_members_access(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let url = format!(
        "http://localhost:{}/api/w/test-workspace/access/get",
        server.addr.port()
    );
    let client = reqwest::Client::new();

    sqlx::raw_sql(
        "INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES ('test-workspace', 'ops', '', '{}');
         INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ('test-workspace', 'test-user-2', 'ops');
         INSERT INTO instance_group (name) VALUES ('sre');
         INSERT INTO email_to_igroup (email, igroup) VALUES ('test2@windmill.dev', 'sre')",
    )
    .execute(&db)
    .await?;

    // A member may read their own access and their groups', instance groups included, nobody
    // else's.
    for (principal, status) in [
        ("u/test-user", 403),
        ("g/all", 403),
        ("u/test-user-2", 200),
        ("g/ops", 200),
        ("g/sre", 200),
    ] {
        let resp = client
            .get(&url)
            .query(&[("principal", principal)])
            .header("Authorization", "Bearer SECRET_TOKEN_2")
            .send()
            .await?;
        assert_eq!(resp.status(), status, "{principal}: {}", resp.text().await?);
    }

    // A scoped token reaches the route only if `/access` maps to a scope domain.
    let scoped = client
        .post(format!(
            "http://localhost:{}/api/users/tokens/create",
            server.addr.port()
        ))
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .json(&serde_json::json!({ "label": "scoped", "scopes": ["acls:read"], "workspace_id": "test-workspace" }))
        .send()
        .await?
        .text()
        .await?;
    let resp = client
        .get(&url)
        .query(&[("principal", "u/test-user-2")])
        .header("Authorization", format!("Bearer {scoped}"))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    Ok(())
}
