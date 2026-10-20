//! Paths that hold a pool connection (an open transaction or an acquired connection) must not
//! wait for a second one from the same pool: a burst the size of the pool would have every
//! holder wait for another until the acquire timeout. On a one-connection pool any such second
//! checkout hangs.

use std::time::Duration;

use serde_json::json;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use windmill_common::{
    db::{AuthedRef, UserDB, UserDbWithAuthed},
    error::Error,
    jobs::JobPayload,
    worker::Connection,
};
use windmill_test_utils::*;

async fn one_connection_pool(db: &Pool<Postgres>) -> Pool<Postgres> {
    PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with((*db.connect_options()).clone())
        .await
        .unwrap()
}

#[sqlx::test(fixtures("base"))]
async fn run_by_path_of_a_missing_script_uses_one_connection(db: Pool<Postgres>) {
    let one = one_connection_pool(&db).await;
    let groups = vec![];
    let folders = vec![];
    let authed = AuthedRef {
        email: "test2@windmill.dev",
        username: "test-user-2",
        is_admin: &false,
        is_operator: &false,
        groups: &groups,
        folders: &folders,
        scopes: &None,
        token_prefix: &None,
    };
    let lookup = windmill_common::get_latest_deployed_hash_for_path(
        Some(UserDbWithAuthed { authed: &authed, db: UserDB::new(one.clone()) }),
        one.clone(),
        "test-workspace",
        "f/missing/script",
    );
    let res = tokio::time::timeout(Duration::from_secs(15), lookup)
        .await
        .unwrap();
    assert!(matches!(res, Err(Error::NotFound(_))), "{:?}", res.err());
}

#[sqlx::test(fixtures("base"))]
async fn for_loop_flow_runs_on_one_connection(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let flow = serde_json::from_value(json!({
        "modules": [{
            "id": "loop",
            "value": {
                "type": "forloopflow",
                "iterator": { "type": "static", "value": [1, 2] },
                "skip_failures": false,
                "parallel": false,
                "modules": [{ "id": "inner", "value": { "type": "identity" } }]
            }
        }]
    }))?;
    let job = RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
        .push(&db)
        .await;

    let one = one_connection_pool(&db).await;
    let db2 = db.clone();
    in_test_worker(
        Connection::Sql(one),
        async move {
            while sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (SELECT 1 FROM v2_job_completed WHERE id = $1)",
            )
            .bind(job)
            .fetch_one(&db2)
            .await
            .unwrap()
                == false
            {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        },
        server.addr.port(),
    )
    .await;

    let status: String =
        sqlx::query_scalar("SELECT status::text FROM v2_job_completed WHERE id = $1")
            .bind(job)
            .fetch_one(&db)
            .await?;
    assert_eq!(status, "success");
    Ok(())
}

#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(fixtures("base", "schedule"))]
async fn schedule_handlers_run_on_one_connection(db: Pool<Postgres>) -> anyhow::Result<()> {
    use windmill_queue::{add_completed_job, get_mini_completed_job};

    const SCHEDULE: &str = "f/system/handled_schedule";
    const HANDLER: &str = "script/f/system/schedule_recovery_handler";
    sqlx::query(
        "INSERT INTO schedule (workspace_id, path, edited_by, schedule, enabled, script_path, args,
            is_flow, email, timezone, permissioned_as, on_success, on_recovery)
         VALUES ('test-workspace', $1, 'test-user', '0 0 0 1 1 *', true, 'f/system/failing_script',
            '{}', false, 'test@windmill.dev', 'UTC', 'u/test-user', $2, $2)",
    )
    .bind(SCHEDULE)
    .bind(HANDLER)
    .execute(&db)
    .await?;

    // The previous occurrence failed, so this successful one is a recovery.
    let insert_job =
        "INSERT INTO v2_job (id, workspace_id, created_by, created_at, permissioned_as,
            permissioned_as_email, kind, runnable_id, runnable_path, script_lang, tag,
            visible_to_owner, trigger, trigger_kind)
         VALUES ($1, 'test-workspace', 'test-user', now() - $2 * interval '1 minute', 'u/test-user',
            'test@windmill.dev', 'script', 12349, 'f/system/failing_script', 'bun', 'bun', true,
            $3, 'schedule')";
    let failed = uuid::Uuid::new_v4();
    sqlx::query(insert_job)
        .bind(failed)
        .bind(1.0)
        .bind(SCHEDULE)
        .execute(&db)
        .await?;
    sqlx::query(
        "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, result, started_at,
            completed_at, status)
         VALUES ($1, 'test-workspace', 1, '{\"error\": \"Failed\"}', now(), now(), 'failure')",
    )
    .bind(failed)
    .execute(&db)
    .await?;

    let running = uuid::Uuid::new_v4();
    sqlx::query(insert_job)
        .bind(running)
        .bind(0.0)
        .bind(SCHEDULE)
        .execute(&db)
        .await?;
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, started_at, tag)
         VALUES ($1, 'test-workspace', now(), true, now(), 'bun')",
    )
    .bind(running)
    .execute(&db)
    .await?;
    sqlx::query("INSERT INTO v2_job_runtime (id, ping) VALUES ($1, now())")
        .bind(running)
        .execute(&db)
        .await?;

    let one = one_connection_pool(&db).await;
    let job = get_mini_completed_job(&running, "test-workspace", &db)
        .await?
        .unwrap();
    let result = json!("OK");
    let complete = add_completed_job(
        &one,
        &job,
        true,
        false,
        sqlx::types::Json(&result),
        None,
        0,
        None,
        false,
        None,
        false,
    );
    tokio::time::timeout(Duration::from_secs(30), complete).await??;

    // One on_success job and one on_recovery job.
    let handlers: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM v2_job WHERE runnable_path = 'f/system/schedule_recovery_handler'
            AND parent_job = $1",
    )
    .bind(running)
    .fetch_one(&db)
    .await?;
    assert_eq!(handlers, 2);
    Ok(())
}
