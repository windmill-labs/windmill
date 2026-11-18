//! `push` must not need a second pool connection while it holds its transaction: a burst of
//! pushes the size of the pool would each hold one and wait for another until the acquire
//! timeout. On a one-connection pool any such second checkout hangs.

use std::time::Duration;

use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use windmill_common::jobs::JobPayload;
use windmill_test_utils::RunJob;

#[sqlx::test(fixtures("base"))]
async fn concurrency_limited_flow_pushes_on_one_connection(db: Pool<Postgres>) {
    // The flow and settings caches outlive the test database (they are on disk), so only a
    // version and a limit no run has used yet make `push` read the flow and write its settings.
    let limit = (uuid::Uuid::new_v4().as_u128() % 1_000_000_000) as i32 + 1;
    let version = limit as i64;
    let value = format!(r#"{{"modules":[],"concurrent_limit":{limit}}}"#);
    sqlx::query(
        "WITH f AS (
            INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, schema, extra_perms, versions)
            VALUES ('test-workspace', 'f/t/limited', '', '', $1::jsonb, 'test-user', '{}', '{}', ARRAY[$2])
         )
         INSERT INTO flow_version (id, workspace_id, path, value, schema, created_by)
         VALUES ($2, 'test-workspace', 'f/t/limited', $1::jsonb, '{}', 'test-user')",
    )
    .bind(&value)
    .bind(version)
    .execute(&db)
    .await
    .unwrap();

    let one = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with((*db.connect_options()).clone())
        .await
        .unwrap();

    let push = RunJob::from(JobPayload::Flow {
        path: "f/t/limited".to_string(),
        dedicated_worker: None,
        apply_preprocessor: false,
        version,
        labels: None,
        job_token_scopes: None,
    })
    .push(&one);
    let job = tokio::time::timeout(Duration::from_secs(15), push)
        .await
        .unwrap();

    let stored: Option<i32> = sqlx::query_scalar(
        "SELECT cs.concurrent_limit FROM v2_job_queue q
         JOIN runnable_settings rs ON rs.hash = q.runnable_settings_handle
         JOIN concurrency_settings cs ON cs.hash = rs.concurrency_settings
         WHERE q.id = $1",
    )
    .bind(job)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(stored, Some(limit));
}
