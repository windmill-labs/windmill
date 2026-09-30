use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

const AGENT: &str = "u/test-user-3/agent";

async fn post(
    base_url: &str,
    token: &str,
    route: &str,
    body: serde_json::Value,
) -> anyhow::Result<(u16, String)> {
    let resp = reqwest::Client::new()
        .post(format!("{base_url}/w/test-workspace/{route}"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await?;
    Ok((resp.status().as_u16(), resp.text().await?))
}

async fn agent_identity(db: &Pool<Postgres>) -> anyhow::Result<Option<String>> {
    Ok(sqlx::query_scalar::<_, Option<String>>(
        "SELECT value->>'on_behalf_of' FROM resource
         WHERE workspace_id = 'test-workspace' AND path = $1",
    )
    .bind(AGENT)
    .fetch_one(db)
    .await?)
}

/// The identity an agent runs as is in its value, which anyone who can write the agent writes: it
/// is kept only for an admin asking to keep it, and every other write makes it the writer's own,
/// so a writer cannot change what runs while it keeps running as someone else.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_agent_write_resets_identity(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base_url = format!("http://localhost:{}/api", server.addr.port());
    let value = json!({ "system_prompt": "hi", "on_behalf_of": "u/test-user-2" });

    let (status, body) = post(
        &base_url,
        "SECRET_TOKEN",
        "resources/create",
        json!({ "path": AGENT, "resource_type": "ai_agent", "value": value,
                "preserve_on_behalf_of": true }),
    )
    .await?;
    assert!((200..300).contains(&status), "create: {status} {body}");
    assert_eq!(agent_identity(&db).await?.as_deref(), Some("u/test-user-2"));

    // The agent's owner, not an admin, asking to keep it.
    let (status, body) = post(
        &base_url,
        "SECRET_TOKEN_3",
        &format!("resources/update/{AGENT}"),
        json!({ "value": value, "preserve_on_behalf_of": true }),
    )
    .await?;
    assert!((200..300).contains(&status), "update: {status} {body}");
    assert_eq!(agent_identity(&db).await?.as_deref(), Some("u/test-user-3"));

    Ok(())
}

/// Reading an agent is what allows running it, and the run is the agent's identity, not the
/// caller's: `test-user-2`, a plain member, runs an agent that runs as `test-user-3`.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_agent_run_is_its_identity_and_needs_read(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base_url = format!("http://localhost:{}/api", server.addr.port());

    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type, extra_perms, created_by)
         VALUES ('test-workspace', $1, $2, 'ai_agent', '{}', 'test-user-3')",
    )
    .bind(AGENT)
    .bind(json!({ "system_prompt": "hi", "on_behalf_of": "u/test-user-3" }))
    .execute(&db)
    .await?;

    let run = format!("jobs/run/agent/{AGENT}");
    let (status, body) = post(&base_url, "SECRET_TOKEN_2", &run, json!({})).await?;
    assert_eq!(status, 404, "an unreadable agent: {body}");

    sqlx::query(
        "UPDATE resource SET extra_perms = '{\"u/test-user-2\": false}'
         WHERE workspace_id = 'test-workspace' AND path = $1",
    )
    .bind(AGENT)
    .execute(&db)
    .await?;
    let (status, body) = post(
        &base_url,
        "SECRET_TOKEN_2",
        &run,
        json!({ "user_message": "hi" }),
    )
    .await?;
    assert_eq!(status, 201, "a readable agent: {body}");

    let (permissioned_as, created_by) = sqlx::query_as::<_, (String, String)>(
        "SELECT permissioned_as, created_by FROM v2_job WHERE id = $1::uuid",
    )
    .bind(&body)
    .fetch_one(&db)
    .await?;
    assert_eq!(permissioned_as, "u/test-user-3");
    assert_eq!(created_by, "test-user-2");

    Ok(())
}
