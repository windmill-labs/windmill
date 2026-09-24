use sqlx::{Pool, Postgres};
use windmill_common::scripts::runnable_list_fingerprint;

/// MCP clients refresh their tool list when this fingerprint moves, so it must move on every
/// change to what the listing shows and stay put on writes it does not show.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn fingerprint_tracks_the_live_runnable_list(db: Pool<Postgres>) {
    let fp = || async {
        runnable_list_fingerprint(&db, "test-workspace")
            .await
            .unwrap()
    };
    let exec = |sql: &'static str| {
        let db = db.clone();
        async move { sqlx::query(sql).execute(&db).await.unwrap() }
    };

    let mut last = fp().await;
    let mut assert_moved = async |what: &str, moved: bool| {
        let now = fp().await;
        assert_eq!(now != last, moved, "{what}");
        last = now;
    };

    exec(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, language, kind)
         VALUES ('test-workspace', 1, 'f/test/a', '', '', 'def main(): pass', 'test-user', 'python3', 'script')",
    )
    .await;
    assert_moved("deploy", true).await;

    exec("UPDATE script SET summary = 'x', lock = '' WHERE hash = 1").await;
    assert_moved("unrelated update", false).await;

    exec("UPDATE script SET path = 'f/test/b' WHERE hash = 1").await;
    assert_moved("path move", true).await;

    exec("UPDATE script SET archived = true WHERE hash = 1").await;
    assert_moved("archive", true).await;

    exec(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, schema, versions)
         VALUES ('test-workspace', 'f/test/flow', '', '', '{}'::jsonb, 'test-user', '{}'::json, ARRAY[1]::bigint[])",
    )
    .await;
    assert_moved("flow create", true).await;

    exec("UPDATE flow SET versions = array_append(versions, 2::bigint) WHERE path = 'f/test/flow'")
        .await;
    assert_moved("flow redeploy", true).await;

    exec("DELETE FROM flow WHERE path = 'f/test/flow'").await;
    assert_moved("flow delete", true).await;
}
