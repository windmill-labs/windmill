//! `GET /w/{workspace}/runnables/list?paths=`: only rows at the given paths, each kind on its
//! own at a shared path, archived rows left out.

use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

async fn insert_script(
    db: &Pool<Postgres>,
    hash: i64,
    path: &str,
    archived: bool,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, language, archived)
         VALUES ('test-workspace', $1, $2, '', '', 'x', 'test-user', 'deno', $3)",
    )
    .bind(hash)
    .bind(path)
    .bind(archived)
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

#[sqlx::test(fixtures("base"))]
async fn test_list_runnables_by_paths(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    // A renamed script leaves its archived versions behind; a flow can then take the path.
    insert_script(&db, 9100001, "u/test-user/moved", true).await?;
    insert_flow(&db, "u/test-user/moved", false).await?;
    insert_script(&db, 9100002, "u/test-user/live", false).await?;
    insert_flow(&db, "u/test-user/archived_flow", true).await?;
    insert_script(&db, 9100003, "u/test-user/not_asked", false).await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let body: serde_json::Value = reqwest::Client::new()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/runnables/list\
             ?paths=u/test-user/moved,u/test-user/live,u/test-user/archived_flow,u/test-user/gone"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut found: Vec<String> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            format!(
                "{}:{}",
                x["type"].as_str().unwrap(),
                x["path"].as_str().unwrap()
            )
        })
        .collect();
    found.sort();
    assert_eq!(found, ["flow:u/test-user/moved", "script:u/test-user/live"]);

    Ok(())
}
