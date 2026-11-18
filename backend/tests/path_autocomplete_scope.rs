//! `GET /path_autocomplete/list_paths` returns only the paths the caller can read,
//! and its cache never serves one caller's list to another.

use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

async fn list_paths(port: u16, token: &str) -> anyhow::Result<Vec<String>> {
    let resp = reqwest::Client::new()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/path_autocomplete/list_paths"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "list_paths: {}", resp.text().await?);
    let body: serde_json::Value = resp.json().await?;
    Ok(serde_json::from_value(body["paths"].clone())?)
}

#[sqlx::test(fixtures("base"))]
async fn test_list_paths_is_scoped_to_caller(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    sqlx::query(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms)
         VALUES ('test-workspace', 'secret', 'secret', '{}', '{}')",
    )
    .execute(&db)
    .await?;
    for path in [
        "f/secret/hidden",
        "u/test-user/private",
        "u/test-user-2/mine",
    ] {
        sqlx::query(
            "INSERT INTO variable (workspace_id, path, value, is_secret, description)
             VALUES ('test-workspace', $1, 'x', false, '')",
        )
        .bind(path)
        .execute(&db)
        .await?;
    }

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // The admin's call fills the cache first, so a cache shared across callers
    // would hand the admin's list to the non-admin.
    assert_eq!(
        list_paths(port, "SECRET_TOKEN").await?,
        [
            "f/secret/hidden",
            "u/test-user-2/mine",
            "u/test-user/private"
        ]
    );
    assert_eq!(
        list_paths(port, "SECRET_TOKEN_2").await?,
        ["u/test-user-2/mine"]
    );
    Ok(())
}
