use sqlx::{Pool, Postgres};
use windmill_common::trashbin::delete_expired_trash;

/// A deleted flow's versions must survive until no queued run uses them, and only them:
/// a run of a flow recreated at the same path must not keep the old ones alive.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn expired_trash_purges_versions_no_queued_run_uses(db: Pool<Postgres>) {
    sqlx::raw_sql(
        "INSERT INTO flow_version (id, workspace_id, path, value, created_by) VALUES
            (100, 'test-workspace', 'f/t/running', '{}', 'u'),
            (200, 'test-workspace', 'f/t/idle', '{}', 'u'),
            (300, 'test-workspace', 'f/t/recreated', '{}', 'u'),
            (301, 'test-workspace', 'f/t/recreated', '{}', 'u');
         INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, versions)
            VALUES ('test-workspace', 'f/t/recreated', '', '', '{}', 'u', ARRAY[301::bigint]);
         INSERT INTO trashbin (workspace_id, item_kind, item_path, item_data, deleted_by, expires_at)
            SELECT 'test-workspace', 'flow', p, '{}', 'u', now() - interval '1 second'
            FROM unnest(ARRAY['f/t/running', 'f/t/idle', 'f/t/recreated']) p;
         WITH job AS (
            INSERT INTO v2_job (id, workspace_id, kind, runnable_path, runnable_id)
            VALUES (gen_random_uuid(), 'test-workspace', 'flow', 'f/t/running', 100),
                   (gen_random_uuid(), 'test-workspace', 'flow', 'f/t/recreated', 301)
            RETURNING id
         )
         INSERT INTO v2_job_queue (id, workspace_id, scheduled_for) SELECT id, 'test-workspace', now() FROM job",
    )
    .execute(&db)
    .await
    .unwrap();

    assert_eq!(delete_expired_trash(&db).await.unwrap(), 3);

    let left: Vec<i64> = sqlx::query_scalar("SELECT id FROM flow_version ORDER BY id")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(left, vec![100, 301]);
}
