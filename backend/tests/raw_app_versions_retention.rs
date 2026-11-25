//! What `raw_app_versions_to_keep` may delete: never the deployed version, whatever its
//! timestamp, a version just superseded, a draft's base or a low-code app's version.

use sqlx::{Pool, Postgres};

#[sqlx::test(fixtures("base"))]
async fn prune_keeps_head_recent_draft_base_and_low_code(db: Pool<Postgres>) -> anyhow::Result<()> {
    // 106 is deployed yet has the oldest timestamp, so 104 and the 105 it replaced were both
    // superseded minutes ago at the earliest. A draft is based on 102.
    sqlx::raw_sql(
        "INSERT INTO app (id, workspace_id, path, policy, versions) VALUES
            (100, 'test-workspace', 'u/test-user/raw', '{}', '{101,102,103,105,104,106}'),
            (200, 'test-workspace', 'u/test-user/low', '{}', '{201,202}');
         INSERT INTO app_version (id, app_id, value, created_by, created_at, raw_app) VALUES
            (101, 100, '{}', 'test-user', now() - interval '6 days', true),
            (102, 100, '{}', 'test-user', now() - interval '5 days', true),
            (103, 100, '{}', 'test-user', now() - interval '4 days', true),
            (105, 100, '{}', 'test-user', now() - interval '3 days', true),
            (104, 100, '{}', 'test-user', now() - interval '5 minutes', true),
            (106, 100, '{}', 'test-user', now() - interval '7 days', true),
            (201, 200, '{}', 'test-user', now() - interval '6 days', false),
            (202, 200, '{}', 'test-user', now() - interval '5 days', false);
         INSERT INTO app_bundles (app_version_id, w_id, file_type, data) VALUES
            (101, 'test-workspace', 'js', 'old'), (106, 'test-workspace', 'js', 'deployed');
         INSERT INTO draft (workspace_id, path, typ, value, base)
            VALUES ('test-workspace', 'u/test-user/raw', 'raw_app', '{}', '102');
         INSERT INTO global_settings (name, value) VALUES ('raw_app_versions_to_keep', '1');",
    )
    .execute(&db)
    .await?;

    assert_eq!(windmill_api::prune_raw_app_versions(&db).await?, 2);

    let versions = |app: i64| {
        sqlx::query_scalar::<_, Vec<i64>>("SELECT versions FROM app WHERE id = $1")
            .bind(app)
            .fetch_one(&db)
    };
    assert_eq!(versions(100).await?, [102, 105, 104, 106]);
    assert_eq!(versions(200).await?, [201, 202]);
    let bundles: Vec<i64> = sqlx::query_scalar("SELECT app_version_id FROM app_bundles")
        .fetch_all(&db)
        .await?;
    assert_eq!(bundles, [106]);
    Ok(())
}
