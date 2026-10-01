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
    windmill_common::evict_deleted_script_version("test-workspace", HASH);

    let err = cache::script::fetch(&conn, ScriptHash(HASH))
        .await
        .expect_err("a deleted version must not be served");
    assert!(err.to_string().contains("was deleted"), "{err}");
}
