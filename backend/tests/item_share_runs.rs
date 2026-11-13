//! A script or flow shared item by item shows its runs to whoever it is shared with, as a
//! folder grant does, and to no one else.

use sqlx::{Pool, Postgres};

#[sqlx::test(fixtures("base"))]
async fn item_shares_show_runs(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::raw_sql(
        r#"
        INSERT INTO workspace (id, name, owner) VALUES ('other', 'other', 'test-user');
        INSERT INTO password (email, login_type, verified) VALUES ('bob@windmill.dev', 'password', true);
        INSERT INTO usr (workspace_id, email, username) VALUES
            ('test-workspace', 'bob@windmill.dev', 'bob'), ('other', 'bob@windmill.dev', 'bob');
        INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, language, extra_perms) VALUES
            ('test-workspace', 1, 'f/ops/to_bob', '', '', '', 'test-user', 'deno', '{"u/bob": false}'),
            ('test-workspace', 2, 'f/ops/to_team', '', '', '', 'test-user', 'deno', '{"g/team": false}'),
            ('test-workspace', 3, 'f/ops/private', '', '', '', 'test-user', 'deno', '{}'),
            ('test-workspace', 4, 'f/ops/hidden_runs', '', '', '', 'test-user', 'deno', '{"u/bob": false}'),
            ('other', 5, 'f/ops/private', '', '', '', 'test-user', 'deno', '{"u/bob": false}');
        INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, extra_perms) VALUES
            ('test-workspace', 'f/ops/flow_to_bob', '', '', '{"modules": []}', 'test-user', '{"u/bob": true}');
        INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, kind, runnable_path, visible_to_owner) VALUES
            (gen_random_uuid(), 'test-workspace', 'carol', 'u/carol', 'script', 'f/ops/to_bob', true),
            (gen_random_uuid(), 'test-workspace', 'carol', 'u/carol', 'script', 'f/ops/to_team', true),
            (gen_random_uuid(), 'test-workspace', 'carol', 'u/carol', 'script', 'f/ops/private', true),
            (gen_random_uuid(), 'test-workspace', 'carol', 'u/carol', 'script', 'f/ops/hidden_runs', false),
            (gen_random_uuid(), 'test-workspace', 'carol', 'u/carol', 'flow', 'f/ops/flow_to_bob', true);
        "#,
    )
    .execute(&db)
    .await?;

    let visible = |groups: &'static str| {
        let db = db.clone();
        async move {
            let mut tx = db.begin().await?;
            sqlx::query("SELECT set_session_context(false, 'bob', $1, $2, '', '')")
                .bind(groups)
                .bind(
                    groups
                        .split(',')
                        .map(|g| format!("g/{g}"))
                        .collect::<Vec<_>>()
                        .join(","),
                )
                .execute(&mut *tx)
                .await?;
            let paths: Vec<String> = sqlx::query_scalar(
                "SELECT runnable_path FROM v2_job WHERE workspace_id = 'test-workspace' ORDER BY runnable_path",
            )
            .fetch_all(&mut *tx)
            .await?;
            anyhow::Ok(paths)
        }
    };

    // `f/ops/private` is shared with bob only in another workspace, where `bob` is someone else.
    assert_eq!(visible("all").await?, ["f/ops/flow_to_bob", "f/ops/to_bob"]);
    assert_eq!(
        visible("all,team").await?,
        ["f/ops/flow_to_bob", "f/ops/to_bob", "f/ops/to_team"]
    );
    Ok(())
}
