//! A perpetual run moves to a newer version at its path, once its current run finishes, only when
//! that version was deployed with `apply_to_perpetual_runs`.

use serde_json::{json, Value};
use sqlx::{types::Json, Pool, Postgres};
use uuid::Uuid;
use windmill_queue::{add_completed_job, get_mini_completed_job};

const W_ID: &str = "test-workspace";
const PATH: &str = "u/test-user/loop";

// Hashes are distinct across tests: the per-version restart settings are cached process-wide,
// while each test runs against its own database.
async fn insert_version(
    db: &Pool<Postgres>,
    hash: i64,
    age_s: f64,
    timeout: Option<i32>,
    apply_to_perpetual_runs: Option<bool>,
    on_behalf_of: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by, \
            language, lock, restart_unless_cancelled, timeout, apply_to_perpetual_runs, \
            on_behalf_of, created_at) \
         VALUES ($1, $2, $3, '', '', 'echo', 'test-user', 'bash', '', true, $4, $5, $6, \
            now() - make_interval(secs => $7))",
    )
    .bind(W_ID)
    .bind(hash)
    .bind(PATH)
    .bind(timeout)
    .bind(apply_to_perpetual_runs)
    .bind(on_behalf_of)
    .bind(age_s)
    .execute(db)
    .await?;
    Ok(())
}

/// Finishes a run of `hash` and returns the version, timeout and arguments of the run it queued.
async fn next_run_after_run_of(
    db: &Pool<Postgres>,
    hash: i64,
) -> anyhow::Result<(i64, Option<i32>, Value)> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, created_at, permissioned_as, \
            permissioned_as_email, kind, runnable_id, runnable_path, script_lang, tag, args, \
            visible_to_owner) \
         VALUES ($1, $2, 'test-user', now(), 'u/test-user', 'test@windmill.dev', 'script', $3, \
            $4, 'bash', 'bash', '{\"n\": 1}', true)",
    )
    .bind(id)
    .bind(W_ID)
    .bind(hash)
    .bind(PATH)
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

    let job = get_mini_completed_job(&id, W_ID, db).await?.unwrap();
    add_completed_job(
        db,
        &job,
        true,
        false,
        Json(&json!("done")),
        None,
        0,
        None,
        false,
        None,
        false,
    )
    .await?;

    Ok(sqlx::query_as(
        "SELECT runnable_id, timeout, args FROM v2_job \
         WHERE workspace_id = $1 AND runnable_path = $2 AND id <> $3",
    )
    .bind(W_ID)
    .bind(PATH)
    .bind(id)
    .fetch_one(db)
    .await?)
}

#[sqlx::test(fixtures("base"))]
async fn a_flagged_newer_version_takes_over_the_next_run(db: Pool<Postgres>) -> anyhow::Result<()> {
    insert_version(&db, 1001, 60.0, None, None, None).await?;
    insert_version(&db, 1002, 0.0, Some(120), Some(true), None).await?;

    let (hash, timeout, args) = next_run_after_run_of(&db, 1001).await?;
    assert_eq!(hash, 1002);
    assert_eq!(
        timeout,
        Some(120),
        "the next run takes the new version's settings"
    );
    assert_eq!(
        args,
        json!({"n": 1}),
        "and the arguments of the run that finished"
    );
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn a_newer_version_without_the_flag_leaves_the_run_on_its_version(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    insert_version(&db, 2001, 60.0, None, None, None).await?;
    insert_version(&db, 2002, 0.0, None, None, None).await?;

    let (hash, _, _) = next_run_after_run_of(&db, 2001).await?;
    assert_eq!(hash, 2001);
    Ok(())
}

/// A run of a version with an on-behalf-of identity is permissioned as that identity, which a new
/// version naming no one must not inherit.
#[sqlx::test(fixtures("base"))]
async fn a_run_on_behalf_of_an_identity_stays_when_the_new_version_names_none(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    insert_version(&db, 3001, 60.0, None, None, Some("u/test-user")).await?;
    insert_version(&db, 3002, 0.0, None, Some(true), None).await?;

    let (hash, _, _) = next_run_after_run_of(&db, 3001).await?;
    assert_eq!(hash, 3001);
    Ok(())
}
