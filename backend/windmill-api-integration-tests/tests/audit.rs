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
async fn test_audit_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/audit");

    // GET /list returns 200 (empty array)
    let resp = authed(client().get(format!("{base}/list"))).send().await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /audit/list",
    );

    Ok(())
}

/// A run fired by a labeled token must stay findable under that label, like every other
/// operation the token performs. `push` builds its own audit author from `(user,
/// permissioned_as)` rather than from the `ApiAuthed`, so the label only reaches the row
/// through the explicit end-user argument.
///
/// EE-only: the OSS `audit_log` writes nothing, and a lesser plan redacts the `parameters`
/// this matches on.
#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_job_run_is_searchable_by_token_label(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let resp = authed(client().post(format!("http://localhost:{port}/api/users/tokens/create")))
        .json(&json!({ "label": "audit-probe" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201);
    let token = resp.text().await?;

    let resp = client()
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/jobs/run/preview"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&json!({
            "content": "export function main() { return 1; }",
            "language": "deno",
            "args": {}
        }))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /jobs/run/preview",
    );

    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/audit/list?username=label-audit-probe"
    )))
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    let logs: Vec<serde_json::Value> = resp.json().await?;
    assert!(
        logs.iter()
            .any(|l| l["operation"] == "jobs.run.preview" && l["username"] == "test-user"),
        "the run must be searchable by token label and still credited to the token owner, got {logs:?}"
    );

    Ok(())
}
