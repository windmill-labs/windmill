use sqlx::{Pool, Postgres};
use windmill_common::trashbin::{delete_expired_trash, set_aside_flow_versions};

/// A deleted flow's versions must survive its trash entry while a queued job references
/// them, whatever path that job was queued under, and nothing else at the path is touched.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn expired_trash_purges_set_aside_versions_no_queued_job_uses(db: Pool<Postgres>) {
    sqlx::raw_sql(
        "INSERT INTO flow_version (id, workspace_id, path, value, created_by) VALUES
            (100, 'test-workspace', 'f/t/a', '{}', 'u'),
            (101, 'test-workspace', 'f/t/a', '{}', 'u'),
            (200, 'test-workspace', 'f/t/a', '{}', 'u');
         WITH job AS (
            INSERT INTO v2_job (id, workspace_id, kind, runnable_path, runnable_id)
            VALUES (gen_random_uuid(), 'test-workspace', 'unassigned_flow', 'f/t/renamed', 100)
            RETURNING id
         )
         INSERT INTO v2_job_queue (id, workspace_id, scheduled_for) SELECT id, 'test-workspace', now() FROM job",
    )
    .execute(&db)
    .await
    .unwrap();

    let mut tx = db.begin().await.unwrap();
    let trash_id: i64 = sqlx::query_scalar(
        "INSERT INTO trashbin (workspace_id, item_kind, item_path, item_data, deleted_by, expires_at)
         VALUES ('test-workspace', 'flow', 'f/t/a', '{}', 'u', now() - interval '1 second') RETURNING id",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    set_aside_flow_versions(&mut tx, "test-workspace", "f/t/a", trash_id)
        .await
        .unwrap();
    // A flow recreated at the path after the deletion.
    sqlx::query(
        "INSERT INTO flow_version (id, workspace_id, path, value, created_by)
         VALUES (300, 'test-workspace', 'f/t/a', '{}', 'u')",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(delete_expired_trash(&db).await.unwrap(), 1);

    let left: Vec<(i64, String)> = sqlx::query_as("SELECT id, path FROM flow_version ORDER BY id")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(
        left,
        vec![
            (100, format!("__deleted/{trash_id}")),
            (300, "f/t/a".to_string())
        ]
    );
}
