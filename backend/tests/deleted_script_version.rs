//! Script data is cached by hash with no expiry, so a deleted version must be evicted, and
//! refused once evicted, or a worker that ran it before keeps running its code.

use sqlx::{Pool, Postgres};
use windmill_common::{cache, scripts::ScriptHash, worker::Connection};

const HASH: i64 = 0x5de1_e7ed_0001;

#[sqlx::test(fixtures("base"))]
async fn deleted_version_stops_running_from_a_warm_cache(db: Pool<Postgres>) {
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content,
                             created_by, language, lock)
         VALUES ('test-workspace', $1, 'f/infra/tool', '', '', 'echo old', 'test-user',
                 'bash', '')",
    )
    .bind(HASH)
    .execute(&db)
    .await
    .unwrap();

    let conn = Connection::Sql(db.clone());
    let (data, _) = cache::script::fetch(&conn, ScriptHash(HASH)).await.unwrap();
    assert_eq!(data.code, "echo old");

    // What `delete_script_by_hash` writes.
    sqlx::query(
        "UPDATE script SET content = '', archived = true, deleted = true, lock = ''
         WHERE hash = $1",
    )
    .bind(HASH)
    .execute(&db)
    .await
    .unwrap();
    windmill_common::notify_script_versions_deleted(&db, "test-workspace", &[HASH])
        .await
        .unwrap();

    // What every process's notify poller does with the event.
    let payload: String = sqlx::query_scalar(
        "SELECT payload FROM notify_event WHERE channel = $1 ORDER BY id DESC LIMIT 1",
    )
    .bind(windmill_common::SCRIPT_VERSION_DELETED_CHANNEL)
    .fetch_one(&db)
    .await
    .unwrap();
    let (w_id, hash) = windmill_common::parse_script_version_deleted(&payload).unwrap();
    assert_eq!((w_id, hash), ("test-workspace", HASH));
    windmill_common::evict_deleted_script_version(w_id, hash);

    let err = cache::script::fetch(&conn, ScriptHash(HASH))
        .await
        .expect_err("a deleted version must not be served");
    assert!(err.to_string().contains("was deleted"), "{err}");
}

/// The hash leaves out the workspace, so a fork shares it: deleting the fork's copy must
/// not stop the parent's.
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
    let (data, _) = cache::script::fetch(&conn, ScriptHash(SHARED)).await.unwrap();
    assert_eq!(data.code, "echo shared");
}
