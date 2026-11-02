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

    let resp = client
        .get(&url)
        .query(&[("principal", "u/test-user")])
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .send()
        .await?;
    assert_eq!(resp.status(), 403, "{}", resp.text().await?);

    let resp = client
        .get(&url)
        .query(&[("principal", "u/test-user-2")])
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    Ok(())
}

/// The navigator writes grants with the kind it is given, and `acls/add` sends a full-code app's
/// change to git sync only under `raw_app`. `blocked` must refuse what `acls/add` would refuse.
#[sqlx::test(fixtures("base"))]
async fn test_browse_items_kind_and_blocked(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    sqlx::raw_sql(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
            VALUES ('test-workspace', 'fin', 'fin', '{u/test-user}', '{\"u/test-user-2\": false}', 'test-user');
         INSERT INTO app (id, workspace_id, path, summary, policy, versions, extra_perms)
            VALUES (1, 'test-workspace', 'f/fin/dash', '', '{}', '{1}', '{}');
         INSERT INTO app_version (id, app_id, value, created_by, raw_app)
            VALUES (1, 1, '{}', 'test-user', true);",
    )
    .execute(&db)
    .await?;
    let server = ApiServer::start(db.clone()).await?;
    let url = format!(
        "http://localhost:{}/api/w/test-workspace/access/browse_items",
        server.addr.port()
    );
    let browse = |token: &str| {
        reqwest::Client::new()
            .get(&url)
            .query(&[("principal", "u/test-user-2"), ("prefix", "f/fin/")])
            .header("Authorization", format!("Bearer {token}"))
            .send()
    };

    let member: serde_json::Value = browse("SECRET_TOKEN_2")
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        member["items"],
        serde_json::json!([{
            "kind": "raw_app",
            "path": "f/fin/dash",
            "grant": null,
            "blocked": "Only workspace admins and owners of f/fin can share it"
        }])
    );

    let admin: serde_json::Value = browse("SECRET_TOKEN")
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(admin["items"][0]["blocked"], serde_json::Value::Null);

    Ok(())
}
