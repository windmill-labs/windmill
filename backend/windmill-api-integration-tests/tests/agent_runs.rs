use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

const AGENT: &str = "u/test-user-3/agent";

async fn run(base_url: &str, token: &str) -> anyhow::Result<(u16, String)> {
    let resp = reqwest::Client::new()
        .post(format!(
            "{base_url}/w/test-workspace/jobs/run/agent/{AGENT}"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "user_message": "hi" }))
        .send()
        .await?;
    Ok((resp.status().as_u16(), resp.text().await?))
}

/// Reading an agent is what allows running it, and the run is the caller's own: `test-user-2`, a
/// plain member, runs `test-user-3`'s agent once it may read it, lending nobody's permissions.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_agent_run_needs_read_and_runs_as_caller(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base_url = format!("http://localhost:{}/api", server.addr.port());

    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type, extra_perms, created_by)
         VALUES ('test-workspace', $1, $2, 'ai_agent', '{}', 'test-user-3')",
    )
    .bind(AGENT)
    .bind(json!({ "system_prompt": "hi" }))
    .execute(&db)
    .await?;

    let (status, body) = run(&base_url, "SECRET_TOKEN_2").await?;
    assert_eq!(status, 404, "an unreadable agent: {body}");

    sqlx::query(
        "UPDATE resource SET extra_perms = '{\"u/test-user-2\": false}'
         WHERE workspace_id = 'test-workspace' AND path = $1",
    )
    .bind(AGENT)
    .execute(&db)
    .await?;
    let (status, body) = run(&base_url, "SECRET_TOKEN_2").await?;
    assert_eq!(status, 201, "a readable agent: {body}");

    let (permissioned_as, created_by) = sqlx::query_as::<_, (String, String)>(
        "SELECT permissioned_as, created_by FROM v2_job WHERE id = $1::uuid",
    )
    .bind(&body)
    .fetch_one(&db)
    .await?;
    assert_eq!(permissioned_as, "u/test-user-2");
    assert_eq!(created_by, "test-user-2");

    Ok(())
}
