//! Pins the `parent_hash` filter of `GET /w/{workspace}/scripts/list`: the
//! request must succeed and return exactly the scripts whose `parent_hashes`
//! array contains the given hash. The filter is assembled into a raw SQL string,
//! so a malformed predicate is only caught by executing the query.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

fn new_script(path: &str, content: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": "",
        "description": "",
        "content": content,
        "language": "deno",
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        }
    })
}

async fn create(port: u16, script: serde_json::Value) -> anyhow::Result<String> {
    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/test-workspace/scripts/create"
        )),
        "SECRET_TOKEN",
    )
    .json(&script)
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(status, 201, "script create should succeed: {body}");
    Ok(body)
}

/// Paths returned by `scripts/list` for the given query string.
async fn list_paths(port: u16, query: &str) -> anyhow::Result<Vec<String>> {
    let resp = authed(
        client().get(format!(
            "http://localhost:{port}/api/w/test-workspace/scripts/list?{query}"
        )),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(status, 200, "scripts/list?{query} should succeed: {body}");
    let items: Vec<serde_json::Value> = serde_json::from_str(&body)?;
    Ok(items
        .iter()
        .map(|it| it["path"].as_str().unwrap().to_string())
        .collect())
}

#[sqlx::test(fixtures("base"))]
async fn test_list_scripts_parent_hash_filter(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let lineage_path = "u/test-user/parent_hash_lineage";
    let unrelated_path = "u/test-user/parent_hash_unrelated";

    // A two-version lineage: v2's `parent_hashes` contains v1's hash.
    let v1 = create(
        port,
        new_script(lineage_path, "export async function main() { return 1; }"),
    )
    .await?;
    let mut v2_body = new_script(lineage_path, "export async function main() { return 2; }");
    v2_body["parent_hash"] = json!(v1);
    let v2 = create(port, v2_body).await?;

    // ...plus a script with no lineage at all, which must never match.
    create(
        port,
        new_script(unrelated_path, "export async function main() { return 3; }"),
    )
    .await?;

    let all = list_paths(port, "").await?;
    assert!(
        all.contains(&lineage_path.to_string()) && all.contains(&unrelated_path.to_string()),
        "unfiltered listing should return both scripts, got {all:?}"
    );

    let descendants = list_paths(port, &format!("parent_hash={v1}")).await?;
    assert_eq!(
        descendants,
        vec![lineage_path.to_string()],
        "parent_hash should return only the scripts descending from that hash"
    );

    // v2 is the head, so nothing descends from it.
    let none = list_paths(port, &format!("parent_hash={v2}")).await?;
    assert!(
        none.is_empty(),
        "no script descends from the head version, got {none:?}"
    );

    Ok(())
}
