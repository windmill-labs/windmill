use sqlx::{Pool, Postgres};
use uuid::Uuid;
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
async fn test_concurrency_groups_2xx(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/concurrency_groups/list"
    )))
    .send()
    .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /api/concurrency_groups/list",
    );

    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/concurrency_groups/list_jobs"
    )))
    .send()
    .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /api/w/test-workspace/concurrency_groups/list_jobs",
    );

    Ok(())
}

/// A concurrency key names the workspace, the runnable path and any `$args`-templated
/// argument values, so it must not be readable by a member who cannot read the run it
/// belongs to — the route is global, so nothing in the path scopes it to a workspace.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_concurrency_key_requires_job_read_access(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let insert_job = |id: Uuid, w_id: &'static str, path: &'static str| {
        let db = db.clone();
        async move {
            sqlx::query(
                "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, runnable_path, kind, tag, args)
                 VALUES ($1, $2, 'test-user', 'u/test-user', $3, 'script', 'deno', '{}'::jsonb)",
            )
            .bind(id)
            .bind(w_id)
            .bind(path)
            .execute(&db)
            .await?;
            sqlx::query("INSERT INTO concurrency_key (key, job_id) VALUES ($1, $2)")
                .bind(format!("{w_id}/script/{path}"))
                .bind(id)
                .execute(&db)
                .await?;
            Ok::<_, sqlx::Error>(())
        }
    };
    let url = |id: Uuid| format!("http://localhost:{port}/api/concurrency_groups/{id}/key");
    let get = |id: Uuid, token: &'static str| {
        client()
            .get(url(id))
            .header("Authorization", format!("Bearer {token}"))
            .send()
    };

    let job_id = Uuid::new_v4();
    insert_job(job_id, "test-workspace", "u/test-user/secret_script").await?;

    // test-user-2 is a member of the workspace but the run is neither theirs nor
    // visible to them.
    let resp = get(job_id, "SECRET_TOKEN_2").await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_eq!(status, 403, "expected 403 for a non-viewer, got {body}");
    assert!(
        !body.contains("secret_script"),
        "denied response leaked the key: {body}"
    );

    // A job in a workspace the caller is not a member of must be indistinguishable
    // from a job that does not exist.
    sqlx::query("INSERT INTO workspace (id, name, owner) VALUES ($1, $1, 'test-user')")
        .bind("other-workspace")
        .execute(&db)
        .await?;
    let other_job_id = Uuid::new_v4();
    insert_job(other_job_id, "other-workspace", "u/test-user/other_script").await?;

    let resp = get(other_job_id, "SECRET_TOKEN_2").await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_eq!(status, 404, "expected 404 for a non-member, got {body}");
    assert!(
        !body.contains("other_script"),
        "denied response leaked the key: {body}"
    );

    let resp = get(Uuid::new_v4(), "SECRET_TOKEN_2").await?;
    assert_eq!(
        resp.status().as_u16(),
        404,
        "an unknown job must answer like an inaccessible one"
    );

    let resp = authed(client().get(url(job_id))).send().await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(status, &body, "GET /api/concurrency_groups/{id}/key");
    assert_eq!(body, "\"test-workspace/script/u/test-user/secret_script\"");

    Ok(())
}
