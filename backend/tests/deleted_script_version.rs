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
