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
async fn test_inputs_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/inputs");

    // GET /history with fake runnable → 200 empty array
    let resp = authed(client().get(format!(
        "{base}/history?runnable_id=u/test-user/test&runnable_type=ScriptPath"
    )))
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "GET /inputs/history");

    // GET /list with fake runnable → 200 empty array
    let resp = authed(client().get(format!(
        "{base}/list?runnable_id=u/test-user/test&runnable_type=ScriptPath"
    )))
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "GET /inputs/list");

    // POST /create → 200, returns UUID
    let resp = authed(client().post(format!(
        "{base}/create?runnable_id=u/test-user/test&runnable_type=ScriptPath"
    )))
    .json(&json!({"name": "test_input", "args": {}}))
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "POST /inputs/create");
    let input_id: String = serde_json::from_str(&body)?;

    // GET /{id}/args → 200
    let resp = authed(client().get(format!("{base}/{input_id}/args")))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "GET /inputs/{id}/args");

    // POST /delete/{id} → 200
    let resp = authed(client().post(format!("{base}/delete/{input_id}")))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "POST /inputs/delete/{id}");

    Ok(())
}
