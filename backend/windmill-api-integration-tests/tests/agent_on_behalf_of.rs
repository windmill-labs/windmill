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

async fn job_identity(db: &Pool<Postgres>, id: &str) -> anyhow::Result<(String, String)> {
    Ok(
        sqlx::query_as("SELECT permissioned_as, created_by FROM v2_job WHERE id = $1::uuid")
            .bind(id)
            .fetch_one(db)
            .await?,
    )
}

/// The identity an agent runs as is in its value, which anyone who can write the agent writes: it
/// is kept only for an admin asking to keep it, and every other write makes it the writer's own,
/// so a writer cannot change what runs while it keeps running as someone else. It exists only
/// while the agent opts into running as it.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_agent_write_resets_identity(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base_url = format!("http://localhost:{}/api", server.addr.port());
    let value = json!({ "system_prompt": "hi", "on_behalf_of": "u/test-user-2",
                        "run_on_behalf_of": true });

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

    let mut off = value.clone();
    off["run_on_behalf_of"] = json!(false);
    let (status, body) = post(
        &base_url,
        "SECRET_TOKEN",
        &format!("resources/update/{AGENT}"),
        json!({ "value": off, "preserve_on_behalf_of": true }),
    )
    .await?;
    assert!((200..300).contains(&status), "toggle off: {status} {body}");
    assert_eq!(agent_identity(&db).await?, None);

    Ok(())
}

/// Reading an agent is what allows running it. The run is the caller's unless the agent opts into
/// running as its identity: then `test-user-2`, a plain member, runs as `test-user-3`.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_agent_run_identity_and_needs_read(db: Pool<Postgres>) -> anyhow::Result<()> {
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
    assert_eq!(
        job_identity(&db, &body).await?,
        ("u/test-user-2".into(), "test-user-2".into())
    );

    sqlx::query(
        "UPDATE resource SET value = value || '{\"run_on_behalf_of\": true}'
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
    assert_eq!(status, 201, "an agent run on behalf of: {body}");
    assert_eq!(
        job_identity(&db, &body).await?,
        ("u/test-user-3".into(), "test-user-2".into())
    );

    // The worker would resolve it with the identity's token, however the JSON spells the `$`.
    for raw in [
        r#"{"user_message":"$var:u/test-user-3/secret"}"#,
        r#"{"user_message":"\u0024var:u/test-user-3/secret"}"#,
        // Resolved once a step combines it with an argument holding a reference.
        r#"{"user_message":"$WM_TOKEN"}"#,
        // jsonb drops the undecodable duplicate and keeps the reference.
        r#"{"user_message":{"n":1e400,"n":0,"r":"\u0024var:u/test-user-3/secret"}}"#,
    ] {
        let resp = reqwest::Client::new()
            .post(format!("{base_url}/w/test-workspace/{run}"))
            .header("Authorization", "Bearer SECRET_TOKEN_2")
            .header("Content-Type", "application/json")
            .body(raw)
            .send()
            .await?;
        assert_eq!(resp.status().as_u16(), 400, "a reference in {raw}");
    }
    let (status, body) = post(
        &base_url,
        "SECRET_TOKEN_2",
        &run,
        json!({ "user_message": {"$match": {"currency": "$USD", "_id": "$category"}} }),
    )
    .await?;
    assert_eq!(status, 201, "nothing the worker resolves: {body}");

    // The run takes its conversation outside the caller's row policy, so a reader must not
    // continue another user's conversation; an admin still may, as the policy allows.
    let chat = format!("{run}?memory_id=7d3f6a52-1c4e-4b8a-9f2d-0e5c6b7a8d91");
    let msg = json!({ "user_message": "hi" });
    let (status, body) = post(&base_url, "SECRET_TOKEN_3", &chat, msg.clone()).await?;
    assert_eq!(status, 201, "the owner starts it: {body}");
    let (status, body) = post(&base_url, "SECRET_TOKEN_2", &chat, msg.clone()).await?;
    assert_eq!(status, 400, "another reader: {body}");
    assert!(body.contains("another user"), "{body}");
    let (status, body) = post(&base_url, "SECRET_TOKEN", &chat, msg).await?;
    assert_eq!(
        status, 409,
        "an admin passes ownership, held only by the running turn: {body}"
    );

    Ok(())
}

/// The identity is a username inside the value, so renaming the user has to move it there too.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_agent_identity_follows_user_rename(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base_url = format!("http://localhost:{}/api", server.addr.port());

    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type, created_by)
         VALUES ('test-workspace', $1, $2, 'ai_agent', 'test-user-3')",
    )
    .bind(AGENT)
    .bind(json!({ "run_on_behalf_of": true, "on_behalf_of": "u/test-user-2" }))
    .execute(&db)
    .await?;
    let email: String =
        sqlx::query_scalar("SELECT email FROM usr WHERE username = 'test-user-2' LIMIT 1")
            .fetch_one(&db)
            .await?;

    let resp = reqwest::Client::new()
        .post(format!("{base_url}/users/rename/{email}"))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({ "new_username": "renamed" }))
        .send()
        .await?;
    let status = resp.status().as_u16();
    assert_eq!(status, 200, "rename: {}", resp.text().await?);
    assert_eq!(agent_identity(&db).await?.as_deref(), Some("u/renamed"));

    Ok(())
}
