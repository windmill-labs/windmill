//! `GET /raw_apps/list` must honor a path-scoped token. The route layer only checks
//! the scope domain and RLS knows nothing of token scopes, so the per-path filter
//! lives in the handler.

use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

async fn list_paths(port: u16, token: &str) -> anyhow::Result<Vec<String>> {
    let resp = reqwest::Client::new()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/raw_apps/list"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "list: {}", resp.text().await?);
    let body: Vec<serde_json::Value> = resp.json().await?;
    let mut paths: Vec<String> = body
        .iter()
        .map(|a| a["path"].as_str().unwrap().to_string())
        .collect();
    paths.sort();
    Ok(paths)
}

#[sqlx::test(fixtures("base"))]
async fn test_raw_apps_list_filters_by_token_scope(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    for path in ["u/test-user/a", "u/test-user/sub/b", "f/other/c"] {
        sqlx::query(
            "INSERT INTO raw_app (path, workspace_id, data) VALUES ($1, 'test-workspace', '')",
        )
        .bind(path)
        .execute(&db)
        .await?;
    }
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, scopes)
         VALUES (encode(sha256('SCOPED_TOKEN'::bytea), 'hex'), 'SCOPED_TOK', 'SCOPED_TOKEN',
                 'test@windmill.dev', 'scoped', ARRAY['raw_apps:read:u/test-user/*'])",
    )
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // The same user through an unscoped token sees every row.
    assert_eq!(
        list_paths(port, "SECRET_TOKEN").await?,
        ["f/other/c", "u/test-user/a", "u/test-user/sub/b"]
    );
    assert_eq!(
        list_paths(port, "SCOPED_TOKEN").await?,
        ["u/test-user/a", "u/test-user/sub/b"]
    );
    Ok(())
}
