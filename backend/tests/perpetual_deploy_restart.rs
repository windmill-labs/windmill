//! A deploy stops the perpetual runs at its path and starts them again on the version it made
//! runnable, unless that version no longer loops.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_queue::restart_perpetual_runs_on_new_version;

const W_ID: &str = "test-workspace";

// Paths are distinct across tests: a path resolves to its deployed version through a
// process-wide cache, while each test runs against its own database.
async fn insert_version(
    db: &Pool<Postgres>,
    path: &str,
    hash: i64,
    age_s: f64,
    perpetual: bool,
) -> anyhow::Result<()> {
    insert_tagged_version(db, path, hash, age_s, perpetual, None).await
}

async fn insert_tagged_version(
    db: &Pool<Postgres>,
    path: &str,
    hash: i64,
    age_s: f64,
    perpetual: bool,
    tag: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, \
            language, lock, restart_unless_cancelled, tag, created_at) \
         VALUES ($1, $2, $3, '', '', 'echo', 'test-user', 'bash', '', $4, $5, \
            now() - make_interval(secs => $6))",
    )
    .bind(W_ID)
    .bind(hash)
    .bind(path)
    .bind(perpetual)
    .bind(tag)
    .bind(age_s)
    .execute(db)
    .await?;
    Ok(())
}

/// A run of `hash` a worker has started.
async fn start_run(db: &Pool<Postgres>, path: &str, hash: i64) -> anyhow::Result<Uuid> {
    start_run_as(db, path, hash, "test@windmill.dev").await
}

async fn start_run_as(
    db: &Pool<Postgres>,
    path: &str,
    hash: i64,
    email: &str,
) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, created_at, permissioned_as, \
            permissioned_as_email, kind, runnable_id, runnable_path, script_lang, tag, args, \
            visible_to_owner) \
         VALUES ($1, $2, 'test-user', now(), 'u/test-user', $5, 'script', $3, \
            $4, 'bash', 'bash', '{\"n\": 1}', true)",
    )
    .bind(id)
    .bind(W_ID)
    .bind(hash)
    .bind(path)
    .bind(email)
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, started_at, tag) \
         VALUES ($1, $2, now(), true, now(), 'bash')",
    )
    .bind(id)
    .bind(W_ID)
    .execute(db)
    .await?;
    Ok(id)
}

#[sqlx::test(fixtures("base"))]
async fn a_deploy_restarts_the_runs_of_earlier_versions(db: Pool<Postgres>) -> anyhow::Result<()> {
    let path = "u/test-user/restarted";
    insert_version(&db, path, 101, 60.0, true).await?;
    let running = start_run(&db, path, 101).await?;
    insert_version(&db, path, 102, 0.0, true).await?;

    restart_perpetual_runs_on_new_version(&db, W_ID, path, "test-user").await;

    let canceled: Option<String> =
        sqlx::query_scalar("SELECT canceled_reason FROM v2_job_queue WHERE id = $1")
            .bind(running)
            .fetch_one(&db)
            .await?;
    assert!(
        canceled.is_some_and(|reason| reason.contains(path)),
        "the run of the earlier version is canceled"
    );

    let (hash, args): (i64, Value) = sqlx::query_as(
        "SELECT runnable_id, args FROM v2_job \
         WHERE workspace_id = $1 AND runnable_path = $2 AND id <> $3",
    )
    .bind(W_ID)
    .bind(path)
    .bind(running)
    .fetch_one(&db)
    .await?;
    assert_eq!(hash, 102, "the next run is on the deployed version");
    assert_eq!(
        args,
        json!({ "n": 1 }),
        "with the arguments of the run it replaces"
    );
    Ok(())
}

/// The arguments a run carries went through a preprocessor only if its own version has one, so a
/// version that adds one has to run it over what the replaced run was started with.
#[sqlx::test(fixtures("base"))]
async fn a_deployed_preprocessor_runs_over_arguments_that_never_saw_one(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let path = "u/test-user/preprocessed";
    insert_version(&db, path, 501, 60.0, true).await?;
    let running = start_run(&db, path, 501).await?;
    insert_version(&db, path, 502, 0.0, true).await?;
    sqlx::query("UPDATE script SET has_preprocessor = true WHERE hash = 502 AND workspace_id = $1")
        .bind(W_ID)
        .execute(&db)
        .await?;

    restart_perpetual_runs_on_new_version(&db, W_ID, path, "test-user").await;

    // `false` until a completion swaps in what the preprocessor returned, so the run is queued to
    // go through it.
    let preprocessed: Option<bool> = sqlx::query_scalar(
        "SELECT preprocessed FROM v2_job \
         WHERE workspace_id = $1 AND runnable_path = $2 AND id <> $3",
    )
    .bind(W_ID)
    .bind(path)
    .bind(running)
    .fetch_one(&db)
    .await?;
    assert_eq!(
        preprocessed,
        Some(false),
        "the deployed version's preprocessor runs for the replacement"
    );
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn a_run_stays_on_its_version_when_it_may_not_use_the_deployed_tag(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let path = "u/test-user/tagged";
    insert_version(&db, path, 301, 60.0, true).await?;
    // Runs as the fixture's user who is no superadmin, and no workspace tag allows this one.
    let running = start_run_as(&db, path, 301, "test2@windmill.dev").await?;
    insert_tagged_version(&db, path, 302, 0.0, true, Some("restricted")).await?;

    restart_perpetual_runs_on_new_version(&db, W_ID, path, "test-user").await;

    let queued: Vec<(Uuid, Option<String>)> = sqlx::query_as(
        "SELECT q.id, q.canceled_reason FROM v2_job_queue q JOIN v2_job j USING (id) \
         WHERE j.workspace_id = $1 AND j.runnable_path = $2",
    )
    .bind(W_ID)
    .bind(path)
    .fetch_all(&db)
    .await?;
    assert_eq!(
        queued,
        vec![(running, None)],
        "a tag the run's identity may not use leaves it as it is"
    );
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn a_deploy_that_stops_looping_leaves_the_runs_alone(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let path = "u/test-user/untouched";
    insert_version(&db, path, 201, 60.0, true).await?;
    let running = start_run(&db, path, 201).await?;
    insert_version(&db, path, 202, 0.0, false).await?;

    restart_perpetual_runs_on_new_version(&db, W_ID, path, "test-user").await;

    let queued: Vec<(Uuid, Option<String>)> = sqlx::query_as(
        "SELECT q.id, q.canceled_reason FROM v2_job_queue q JOIN v2_job j USING (id) \
         WHERE j.workspace_id = $1 AND j.runnable_path = $2",
    )
    .bind(W_ID)
    .bind(path)
    .fetch_all(&db)
    .await?;
    assert_eq!(
        queued,
        vec![(running, None)],
        "turning perpetual off leaves the run as it is"
    );
    Ok(())
}
