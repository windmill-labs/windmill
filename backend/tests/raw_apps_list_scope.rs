//! `GET /raw_apps/list` must honor a path-scoped token. The route layer only checks
//! the scope domain and RLS knows nothing of token scopes, so the per-path filter
//! lives in the handler, ahead of the page's LIMIT.

use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

async fn list_paths(port: u16, token: &str, query: &str) -> anyhow::Result<Vec<String>> {
    let resp = reqwest::Client::new()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/raw_apps/list?{query}"
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

    // The list sorts by edited_at desc, so the out-of-scope apps fill the first page
    // unless the scope is applied before the LIMIT.
    for (path, age_minutes) in [
        ("f/other/c", 0),
        ("u/test-userx/d", 1),
        ("u/test-user/a", 2),
        ("u/test-user/sub/b", 3),
    ] {
        sqlx::query(
            "INSERT INTO raw_app (path, workspace_id, data, edited_at)
             VALUES ($1, 'test-workspace', '', now() - make_interval(mins => $2))",
        )
        .bind(path)
        .bind(age_minutes)
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
        list_paths(port, "SECRET_TOKEN", "").await?,
        [
            "f/other/c",
            "u/test-user/a",
            "u/test-user/sub/b",
            "u/test-userx/d"
        ]
    );
    assert_eq!(
        list_paths(port, "SCOPED_TOKEN", "per_page=2").await?,
        ["u/test-user/a", "u/test-user/sub/b"]
    );
    Ok(())
}
