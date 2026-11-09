//! `GET /w/{workspace}/runnables/existing`: per-kind answers at a shared path, archived
//! and non-agent rows, RLS, a library script for an operator, and the path cap.

use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

async fn existing(port: u16, token: &str, query: &str) -> (u16, serde_json::Value) {
    let resp = reqwest::Client::new()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/runnables/existing?{query}"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    let status = resp.status().as_u16();
    let body = resp.json().await.unwrap_or(serde_json::Value::Null);
    (status, body)
}

async fn insert_script(
    db: &Pool<Postgres>,
    hash: i64,
    path: &str,
    archived: bool,
    auto_kind: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, language, archived, auto_kind)
         VALUES ('test-workspace', $1, $2, '', '', 'x', 'test-user', 'deno', $3, $4)",
    )
    .bind(hash)
    .bind(path)
    .bind(archived)
    .bind(auto_kind)
    .execute(db)
    .await?;
    Ok(())
}

async fn insert_flow(db: &Pool<Postgres>, path: &str, archived: bool) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, schema, archived)
         VALUES ('test-workspace', $1, '', '', '{}'::jsonb, 'test-user', '{}'::json, $2)",
    )
    .bind(path)
    .bind(archived)
    .execute(db)
    .await?;
    Ok(())
}

async fn insert_resource(
    db: &Pool<Postgres>,
    path: &str,
    resource_type: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO resource_type (workspace_id, name, schema) VALUES ('test-workspace', $1, '{}'::jsonb)
         ON CONFLICT DO NOTHING",
    )
    .bind(resource_type)
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type, created_by)
         VALUES ('test-workspace', $1, '{}'::jsonb, $2, 'test-user')",
    )
    .bind(path)
    .bind(resource_type)
    .execute(db)
    .await?;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_existing_runnables(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    // A renamed script leaves its archived versions behind; a flow can then take the path.
    insert_script(&db, 9100001, "u/test-user/moved", true, None).await?;
    insert_flow(&db, "u/test-user/moved", false).await?;
    insert_script(&db, 9100002, "u/test-user/live", false, None).await?;
    insert_flow(&db, "u/test-user/archived_flow", true).await?;
    insert_resource(&db, "u/test-user/agent", "ai_agent").await?;
    insert_resource(&db, "u/test-user/not_agent", "postgresql").await?;
    sqlx::query(
        "INSERT INTO app (workspace_id, path, summary, policy, versions, extra_perms)
         VALUES ('test-workspace', 'u/test-user/app', '', '{}'::jsonb, ARRAY[]::bigint[], '{}'::jsonb)",
    )
    .execute(&db)
    .await?;
    // A library script in an operator's own space: hidden from them by the listing, still
    // theirs to open, so still in their Recent list.
    sqlx::query("UPDATE usr SET operator = true WHERE workspace_id = 'test-workspace' AND username = 'test-user-3'")
        .execute(&db)
        .await?;
    insert_script(&db, 9100003, "u/test-user-3/lib", false, Some("lib")).await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let (status, body) = existing(
        port,
        "SECRET_TOKEN",
        "script=u/test-user/moved&flow=u/test-user/moved&script=u/test-user/live\
         &flow=u/test-user/archived_flow&agent=u/test-user/agent&agent=u/test-user/not_agent\
         &app=u/test-user/app&app=u/test-user/gone",
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(
        body,
        serde_json::json!({
            "script": ["u/test-user/live"],
            "flow": ["u/test-user/moved"],
            "app": ["u/test-user/app"],
            "agent": ["u/test-user/agent"],
        }),
        "each kind answered on its own; archived and non-agent rows are not live"
    );

    // RLS: another user's space is not readable, so it is reported as not there.
    let (_, body) = existing(port, "SECRET_TOKEN_2", "script=u/test-user/live").await;
    assert_eq!(body["script"], serde_json::json!([]));

    let (_, body) = existing(port, "SECRET_TOKEN_3", "script=u/test-user-3/lib").await;
    assert_eq!(
        body["script"],
        serde_json::json!(["u/test-user-3/lib"]),
        "a library script stays live for an operator who can open it"
    );

    let too_many = (0..101)
        .map(|i| format!("script=u/x/p{i}"))
        .collect::<Vec<_>>()
        .join("&");
    let (status, _) = existing(port, "SECRET_TOKEN", &too_many).await;
    assert_eq!(
        status, 400,
        "over the cap is refused, not answered partially"
    );

    Ok(())
}
