//! Token-scope filtering on the folder list endpoints.
//!
//! `folders/list` and `folders/listnames` must narrow their results to the
//! paths a scoped token may read, and must do so *before* pagination — an
//! out-of-scope folder sorting ahead of an in-scope one must not consume the
//! caller's page.

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

    // An unscoped token keeps the full listing and the usual page boundaries.
    assert_eq!(
        list_names(port, "SECRET_TOKEN", "").await?,
        vec!["alpha", "zeta"]
    );
    assert_eq!(
        list_names(port, "SECRET_TOKEN", "per_page=1&page=2").await?,
        vec!["zeta"]
    );

    Ok(())
}
