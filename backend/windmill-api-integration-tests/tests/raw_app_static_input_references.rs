use serde_json::json;
use sqlx::{Pool, Postgres};

/// The migration has already run against the empty test database, so the test plants the
/// earlier shape and re-executes it, twice, to also pin that a second run changes nothing.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn raw_app_policy_static_inputs_are_references(db: Pool<Postgres>) -> anyhow::Result<()> {
    // App 910's value holds NUL escapes, which jsonb refuses, and a literal `\\u0000`.
    sqlx::raw_sql(
        r#"
        INSERT INTO app (id, workspace_id, path, policy, versions) VALUES (900, 'test-workspace', 'u/test-user/raw', '{
            "execution_mode": "anonymous",
            "triggerables_v2": {
                "orders:rawscript/abc": {"static_inputs": {"pw": "plain", "db": {"host": "h"}, "note": "edited", "same": "$var:u/test-user/same"}},
                "lookup:script/u/test-user/s": {"static_inputs": {"pw": "plain", "key": "plain"}},
                "orders:preview:rawscript/def": {"static_inputs": {"pw": "public"}},
                "orders:script/preview:rawscript/fed": {"static_inputs": {"pw": "public"}}
            }}', '{901}');
        INSERT INTO app_version (id, app_id, value, created_by, raw_app) VALUES (901, 900, '{"runnables": {
            "orders": {"type": "inline", "fields": {
                "pw": {"type": "static", "value": "$var:u/test-user/pw"},
                "db": {"type": "static", "value": "$res:u/test-user/db"},
                "note": {"type": "static", "value": "hello"},
                "same": {"type": "static", "value": "$var:u/test-user/same"},
                "asked": {"type": "user", "value": "$var:u/test-user/pw"}}},
            "lookup": {"type": "path", "fields": {"pw": {"type": "user", "value": "$var:u/test-user/pw"},
                "key": {"type": "static", "value": "$jsonvar:u/test-user/key"}}},
            "orders:preview": {"type": "inline", "fields": {"pw": {"type": "static", "value": "public"}}},
            "orders:script/preview": {"type": "inline", "fields": {"pw": {"type": "static", "value": "public"}}}}}', 'test-user', true);

        INSERT INTO app (id, workspace_id, path, policy, versions) VALUES (910, 'test-workspace', 'u/test-user/nul',
            '{"triggerables_v2": {"orders:rawscript/abc": {"static_inputs": {"pw": "plain"}}}}', '{911}');
        INSERT INTO app_version (id, app_id, value, created_by, raw_app) VALUES (911, 910, '{
            "files": {"/.DS_Store": "\u0000\u0000\u0001Bud1\\u0000"},
            "runnables": {"orders": {"fields": {"pw": {"type": "static", "value": "$var:u/test-user/pw"}}}}}',
            'test-user', true);

        INSERT INTO app (id, workspace_id, path, policy, versions) VALUES (920, 'test-workspace', 'u/test-user/control',
            '{"triggerables_v2": {"orders:rawscript/abc": {"static_inputs": {"note": "edited"}}}}', '{921}');
        INSERT INTO app_version (id, app_id, value, created_by, raw_app) VALUES (921, 920,
            '{"runnables": {"orders": {"fields": {"note": {"type": "static", "value": "hello"}}}}}', 'test-user', true);
        "#,
    )
    .execute(&db)
    .await?;

    for _ in 0..2 {
        sqlx::raw_sql(include_str!(
            "../../migrations/20261113164906_raw_app_policy_static_input_references.up.sql"
        ))
        .execute(&db)
        .await?;
    }

    let policies: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT policy->'triggerables_v2' FROM app WHERE id IN (900, 910, 920) ORDER BY id",
    )
    .fetch_all(&db)
    .await?;
    assert_eq!(
        policies,
        [
            json!({
                "orders:rawscript/abc": {"static_inputs": {
                    "pw": "$var:u/test-user/pw", "db": "$res:u/test-user/db",
                    "note": "edited", "same": "$var:u/test-user/same"}},
                "lookup:script/u/test-user/s": {"static_inputs": {"pw": "plain", "key": "$jsonvar:u/test-user/key"}},
                "orders:preview:rawscript/def": {"static_inputs": {"pw": "public"}},
                "orders:script/preview:rawscript/fed": {"static_inputs": {"pw": "public"}}
            }),
            json!({"orders:rawscript/abc": {"static_inputs": {"pw": "$var:u/test-user/pw"}}}),
            json!({"orders:rawscript/abc": {"static_inputs": {"note": "edited"}}}),
        ]
    );

    Ok(())
}
