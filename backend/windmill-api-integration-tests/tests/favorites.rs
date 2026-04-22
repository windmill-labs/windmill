use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn assert_2xx(status: u16, body: &str, endpoint: &str) {
    assert!(
        (200..300).contains(&status),
        "{endpoint} returned {status}: {body}",
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_favorites_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    // Setup: create a script to favorite
    let resp = authed(client().post(format!("{ws}/scripts/create")))
        .json(&json!({
            "path": "u/test-user/test_fav_script",
            "summary": "test",
            "description": "",
            "content": "export function main() { return 1; }",
            "language": "deno",
            "schema": {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {},
                "required": []
            }
        }))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "POST /scripts/create (setup)");

    let fav_body = json!({
        "favorite_kind": "script",
        "path": "u/test-user/test_fav_script"
    });

    // POST /favorites/star → 200
    let resp = authed(client().post(format!("{ws}/favorites/star")))
        .json(&fav_body)
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "POST /favorites/star");

    // POST /favorites/unstar → 200
    let resp = authed(client().post(format!("{ws}/favorites/unstar")))
        .json(&fav_body)
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "POST /favorites/unstar");

    Ok(())
}
