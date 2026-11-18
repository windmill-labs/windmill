//! Script data is cached by hash, and the hash leaves out the workspace, so forks and clones
//! share it: a deleted copy in one workspace must not stop the live copy in another.

use sqlx::{Pool, Postgres};
use windmill_common::{cache, scripts::ScriptHash, worker::Connection};

#[sqlx::test(fixtures("base"))]
async fn a_deleted_copy_does_not_shadow_a_live_one(db: Pool<Postgres>) {
    const SHARED: i64 = 0x5de1_e7ed_0002;
    sqlx::query("INSERT INTO workspace (id, name, owner) VALUES ('fork', 'fork', 'test-user')")
        .execute(&db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content,
                             created_by, language, lock, archived, deleted)
         VALUES ('fork', $1, 'f/infra/tool', '', '', '', 'test-user', 'bash', '', true, true),
                ('test-workspace', $1, 'f/infra/tool', '', '', 'echo shared', 'test-user',
                 'bash', '', false, false)",
    )
    .bind(SHARED)
    .execute(&db)
    .await
    .unwrap();

    let conn = Connection::Sql(db.clone());
    let (data, _) = cache::script::fetch(&conn, ScriptHash(SHARED))
        .await
        .unwrap();
    assert_eq!(data.code, "echo shared");
}

#[sqlx::test(fixtures("base"))]
async fn a_large_deletion_is_split_across_events(db: Pool<Postgres>) {
    let deleted = windmill_common::DeletedScriptVersions::new(
        "test-workspace",
        (0..501).map(|h| (format!("f/infra/s{}", h % 2), h)),
    );
    let mut conn = db.acquire().await.unwrap();
    deleted.notify(&mut conn).await.unwrap();

    let events: Vec<String> = sqlx::query_scalar(
        "SELECT payload FROM notify_event WHERE channel = $1 ORDER BY id",
    )
    .bind(windmill_common::SCRIPT_VERSION_DELETED_CHANNEL)
    .fetch_all(&db)
    .await
    .unwrap();
    let events: Vec<windmill_common::DeletedScriptVersions> =
        events.iter().map(|e| serde_json::from_str(e).unwrap()).collect();
    assert_eq!(events.len(), 2);
    let hashes: Vec<i64> = events.iter().flat_map(|e| e.hashes.clone()).collect();
    let paths: Vec<String> = events.iter().flat_map(|e| e.paths.clone()).collect();
    assert_eq!(hashes, deleted.hashes);
    assert_eq!(paths, deleted.paths);
}
