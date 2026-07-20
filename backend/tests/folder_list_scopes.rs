//! Token-scope filtering on the folder list endpoints.
//!
//! `folders/list` and `folders/listnames` must narrow their results to the
//! paths a scoped token may read, and must do so *before* pagination — an
//! out-of-scope folder sorting ahead of an in-scope one must not consume the
//! caller's page. Unrestricted tokens take the SQL-paginated path and must keep
//! their existing page boundaries.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

async fn create_folder(port: u16, name: &str) -> anyhow::Result<()> {
    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/test-workspace/folders/create"
        )),
        "SECRET_TOKEN",
    )
    .json(&json!({ "name": name }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "create folder: {}", resp.text().await?);
    Ok(())
}

async fn mint_scoped_token(port: u16, scopes: Vec<&str>) -> anyhow::Result<String> {
    let resp = authed(
        client().post(format!("http://localhost:{port}/api/users/tokens/create")),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "label": "scoped",
        "scopes": scopes,
        "workspace_id": "test-workspace",
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "mint token");
    Ok(resp.text().await?)
}

async fn list_names(port: u16, token: &str, query: &str) -> anyhow::Result<Vec<String>> {
    let resp = authed(
        client().get(format!(
            "http://localhost:{port}/api/w/test-workspace/folders/listnames?{query}"
        )),
        token,
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "listnames: {}", resp.text().await?);
    Ok(resp.json().await?)
}

async fn list_folder_names(port: u16, token: &str, query: &str) -> anyhow::Result<Vec<String>> {
    let resp = authed(
        client().get(format!(
            "http://localhost:{port}/api/w/test-workspace/folders/list?{query}"
        )),
        token,
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "list: {}", resp.text().await?);
    let folders: Vec<serde_json::Value> = resp.json().await?;
    Ok(folders
        .into_iter()
        .map(|f| f["name"].as_str().unwrap().to_string())
        .collect())
}

#[sqlx::test(fixtures("base"))]
async fn test_folder_list_scope_filtering(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // `alpha` sorts first, so it would occupy page 1 if pagination ran before
    // the scope filter.
    create_folder(port, "alpha").await?;
    create_folder(port, "zeta").await?;

    let scoped = mint_scoped_token(port, vec!["folders:read:f/zeta"]).await?;

    assert_eq!(list_names(port, &scoped, "").await?, vec!["zeta"]);
    assert_eq!(list_folder_names(port, &scoped, "").await?, vec!["zeta"]);
    assert_eq!(
        list_names(port, &scoped, "per_page=1&page=1").await?,
        vec!["zeta"]
    );
    assert_eq!(
        list_folder_names(port, &scoped, "per_page=1&page=1").await?,
        vec!["zeta"]
    );

    // A token whose scopes cover every folder is still scope-restricted, so it
    // takes the filtered path — it must see the same listing as an unscoped one.
    let broad = mint_scoped_token(port, vec!["folders:read:*"]).await?;
    assert_eq!(list_names(port, &broad, "").await?, vec!["alpha", "zeta"]);

    // An unscoped token keeps the full listing and the usual page boundaries.
    assert_eq!(
        list_names(port, "SECRET_TOKEN", "").await?,
        vec!["alpha", "zeta"]
    );
    assert_eq!(
        list_names(port, "SECRET_TOKEN", "per_page=1&page=1").await?,
        vec!["alpha"]
    );
    assert_eq!(
        list_names(port, "SECRET_TOKEN", "per_page=1&page=2").await?,
        vec!["zeta"]
    );
    assert_eq!(
        list_folder_names(port, "SECRET_TOKEN", "per_page=1&page=2").await?,
        vec!["zeta"]
    );

    Ok(())
}

/// The scoped path scans in `SCOPE_SCAN_CHUNK`-sized batches; a workspace larger
/// than one chunk must still page correctly across the boundary.
#[sqlx::test(fixtures("base"))]
async fn test_folder_list_scope_filtering_across_chunks(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    // Zero-padded so lexicographic order matches numeric order.
    sqlx::query(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms)
         SELECT 'test-workspace', 'f' || to_char(i, 'FM0000'), 'f' || to_char(i, 'FM0000'),
                ARRAY[]::text[], '{}'::jsonb
           FROM generate_series(0, 799) AS i",
    )
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // One match in the first chunk, one past the 500-row boundary.
    let scoped = mint_scoped_token(port, vec!["folders:read:f/f0100,f/f0600"]).await?;

    assert_eq!(
        list_names(port, &scoped, "").await?,
        vec!["f0100", "f0600"],
        "both matches must survive the chunked scan"
    );
    assert_eq!(
        list_names(port, &scoped, "per_page=1&page=1").await?,
        vec!["f0100"]
    );
    assert_eq!(
        list_names(port, &scoped, "per_page=1&page=2").await?,
        vec!["f0600"],
        "offset must be honored across the chunk boundary"
    );
    assert_eq!(
        list_folder_names(port, &scoped, "per_page=1&page=2").await?,
        vec!["f0600"]
    );
    assert!(list_names(port, &scoped, "per_page=1&page=3")
        .await?
        .is_empty());

    Ok(())
}
