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

/// A run fired by a labeled token must be findable both by the token that fired it and by the
/// caller who fired it. `push` builds its own audit author from `(user, permissioned_as)` rather
/// than from the `ApiAuthed`, so the label only reaches the row through the explicit end-user
/// argument; and when the runnable declares `on_behalf_of`, the run-as identity takes `username`,
/// so the caller only stays searchable through the `created_by` parameter.
///
/// EE-only: the OSS `audit_log` writes nothing, and a lesser plan redacts the `parameters` this
/// matches on.
#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_job_run_is_searchable_by_token_and_by_caller(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    use serde_json::json;

    initialize_tracing().await;

    // The recorded identity makes a run against this take `u/test-user-2` as its
    // permissioned_as, so the audit `username` slot goes to it rather than to the caller.
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by,
             on_behalf_of, schema, summary, description, lock, extra_perms)
         VALUES ('test-workspace', 900101, 'u/test-user-2/onbehalf', 'export function main() {}',
             'deno', 'script', 'test-user-2', 'u/test-user-2', '{}', '', '', '', '{\"g/all\": true}')",
    )
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let resp = authed(client().post(format!("http://localhost:{port}/api/users/tokens/create")))
        .json(&json!({ "label": "audit-probe" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201);
    let token = resp.text().await?;
    let bearer = |b: reqwest::RequestBuilder| b.header("Authorization", format!("Bearer {token}"));

    let resp = bearer(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/jobs/run/preview"
    )))
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

    let resp = bearer(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/jobs/run/p/u/test-user-2/onbehalf"
    )))
    .json(&json!({}))
    .send()
    .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /jobs/run/p/u/test-user-2/onbehalf",
    );

    let search = |q: &str| {
        let url = format!("http://localhost:{port}/api/w/test-workspace/audit/list?username={q}");
        async move {
            let resp = authed(client().get(url)).send().await.unwrap();
            assert_eq!(resp.status(), 200);
            resp.json::<Vec<serde_json::Value>>().await.unwrap()
        }
    };

    let by_token = search("label-audit-probe").await;
    assert!(
        by_token
            .iter()
            .any(|l| l["operation"] == "jobs.run.preview"),
        "the run must be searchable by token label, got {by_token:?}"
    );
    assert!(
        by_token
            .iter()
            .any(|l| l["operation"] == "jobs.run.script" && l["username"] == "test-user-2"),
        "the on-behalf run must be searchable by token label, got {by_token:?}"
    );

    // `username` is `test-user-2` on that row, so this can only match through `created_by`.
    let by_caller = search("test-user").await;
    assert!(
        by_caller
            .iter()
            .any(|l| l["operation"] == "jobs.run.script" && l["username"] == "test-user-2"),
        "the on-behalf run must stay searchable by the caller who fired it, got {by_caller:?}"
    );

    Ok(())
}
