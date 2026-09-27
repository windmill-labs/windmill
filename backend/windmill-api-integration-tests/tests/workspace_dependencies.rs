use serde_json::json;
use sqlx::{Pool, Postgres};

#[allow(unused_imports)]
use windmill_test_utils::*;

/// An admin of one workspace must not be able to write dependency files into another
/// by naming it in the request body.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_create_rejects_body_workspace_other_than_path(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    // test-user-2 is a plain user of test-workspace and an admin of its own workspace.
    sqlx::query("INSERT INTO workspace (id, name, owner) VALUES ('own-workspace', 'own-workspace', 'test-user-2')")
        .execute(&db)
        .await?;
    sqlx::query("INSERT INTO usr (workspace_id, email, username, is_admin, role) VALUES ('own-workspace', 'test2@windmill.dev', 'test-user-2', true, 'Admin')")
        .execute(&db)
        .await?;

    let (_client, port, _server) = init_client(db.clone()).await;

    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/own-workspace/workspace_dependencies/create"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .json(&json!({
            "workspace_id": "test-workspace",
            "language": "python3",
            "content": "requests==2.28.0"
        }))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_eq!(
        status, 400,
        "expected the mismatched body to be rejected, got {body}"
    );
    assert!(
        body.contains("does not match"),
        "expected the workspace mismatch rejection, got {body}"
    );

    let written: i64 = sqlx::query_scalar("SELECT count(*) FROM workspace_dependencies")
        .fetch_one(&db)
        .await?;
    assert_eq!(
        written, 0,
        "no dependency file may be written in either workspace"
    );

    Ok(())
}
