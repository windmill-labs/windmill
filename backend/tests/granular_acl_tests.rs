use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

/// Insert a script row with a given hash and extra_perms. The `script` PK is
/// (workspace_id, hash), so multiple versions of the same path coexist as
/// distinct rows.
async fn insert_script_version(
    db: &Pool<Postgres>,
    path: &str,
    hash: i64,
    extra_perms: serde_json::Value,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, extra_perms)
         VALUES ('test-workspace', $1, $2, '', '', '', 'test-user', $3)",
    )
    .bind(hash)
    .bind(path)
    .bind(extra_perms)
    .execute(db)
    .await?;
    Ok(())
}

/// Regression test for the "more than one row returned by a subquery used as an
/// expression" error in remove_granular_acl.
///
/// The `script` table is keyed on (workspace_id, hash), so a path with multiple
/// versions yields several rows sharing the same (workspace_id, path). The CTE
/// in remove_granular_acl selects one `old_write` row per matching version, and
/// the scalar subquery `(SELECT old_write FROM old)` in the RETURNING clause
/// errors when `old` has more than one row. The fix adds `LIMIT 1`.
#[sqlx::test(fixtures("base"))]
async fn test_remove_granular_acl_multiple_script_versions(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let path = "u/test-user/multi_version_script";
    let perms = json!({ "g/wm_deployers": true });

    // Two versions of the same script, both carrying the ACL key.
    insert_script_version(&db, path, 1, perms.clone()).await?;
    insert_script_version(&db, path, 2, perms.clone()).await?;

    // This is the exact query shape used by remove_granular_acl for the `script`
    // kind. Before the fix it raised "more than one row returned by a subquery
    // used as an expression".
    let old_write = sqlx::query_scalar::<_, bool>(
        "WITH old AS (
            SELECT extra_perms->$1 as old_write FROM script
            WHERE path = $2 AND workspace_id = $3 AND extra_perms ? $1
        )
        UPDATE script SET extra_perms = extra_perms - $1
        WHERE path = $2 AND workspace_id = $3 AND extra_perms ? $1
        RETURNING (SELECT old_write FROM old LIMIT 1)::bool",
    )
    .bind("g/wm_deployers")
    .bind(path)
    .bind("test-workspace")
    .fetch_optional(&db)
    .await?;

    // The previous write value is returned (true), confirming the query ran.
    assert_eq!(old_write, Some(true));

    // Both versions had the ACL key stripped.
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM script WHERE path = $1 AND workspace_id = $2 AND extra_perms ? $3",
    )
    .bind(path)
    .bind("test-workspace")
    .bind("g/wm_deployers")
    .fetch_one(&db)
    .await?;
    assert_eq!(remaining, 0);

    Ok(())
}
