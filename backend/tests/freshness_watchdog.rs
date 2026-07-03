//! End-to-end tests for the pipeline freshness watchdog (Enterprise).
//!
//! `windmill_queue::freshness_watchdog::tick` is called directly against
//! seeded `script` / `v2_job(_completed)` rows — no worker or API server is
//! needed, since the watchdog's job ends at the push (the pushed job sitting
//! in `v2_job_queue` is itself part of the assertions). Covers: staleness on
//! never-ran and aged-out members, the fresh short-circuit + state reset,
//! the in-flight suppression, the backoff claim, and the skip rules
//! (partitioned, malformed window, non-pipeline scripts).

#![cfg(feature = "private")]

use sqlx::{Pool, Postgres};
use windmill_queue::freshness_watchdog::tick;
use windmill_test_utils::initialize_tracing;

const WS: &str = "test-workspace";
const PATH: &str = "u/test-user/freshness_producer";

/// Seed a deployed pipeline-member script. Mirrors the deploy path's output:
/// `auto_kind = 'pipeline'`, empty (non-NULL) lock so run-by-path resolution
/// treats it as deployed, hash derived from path+content for uniqueness.
async fn seed_pipeline_script(
    db: &Pool<Postgres>,
    path: &str,
    content: &str,
) -> anyhow::Result<()> {
    let mut h = 0i64;
    for b in path.bytes().chain(content.bytes()) {
        h = h.wrapping_mul(31).wrapping_add(b as i64);
    }
    sqlx::query(
        r#"INSERT INTO script (workspace_id, hash, path, summary, description, content,
                                created_by, language, tag, lock, auto_kind)
           VALUES ($1, $2, $3, '', '', $4, 'test-user', 'bash'::script_lang, 'bash', '', 'pipeline')
           ON CONFLICT DO NOTHING"#,
    )
    .bind(WS)
    .bind(h)
    .bind(path)
    .bind(content)
    .execute(db)
    .await?;
    // Process-global deployed-script caches are keyed by (workspace, path) /
    // (workspace, hash) and would leak between #[sqlx::test] isolated DBs
    // that reuse both — resolve everything from this test's own DB.
    windmill_common::DEPLOYED_SCRIPT_CACHE_DISABLED
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

/// Seed a completed root run of `path` that finished `age_s` seconds ago.
async fn seed_completed_run(
    db: &Pool<Postgres>,
    path: &str,
    age_s: i64,
    success: bool,
) -> anyhow::Result<()> {
    let id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO v2_job (id, workspace_id, runnable_path, kind, created_at,
                                created_by, permissioned_as, permissioned_as_email, tag)
           VALUES ($1, $2, $3, 'script'::job_kind,
                   now() - ($4::bigint::text || ' seconds')::interval,
                   'test-user', 'u/test-user', 'test@windmill.dev', 'bash')"#,
    )
    .bind(id)
    .bind(WS)
    .bind(path)
    .bind(age_s)
    .execute(db)
    .await?;
    sqlx::query(
        r#"INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status, started_at, completed_at)
           VALUES ($1, $2, 0, CASE WHEN $3 THEN 'success'::job_status ELSE 'failure'::job_status END,
                   now() - ($4::bigint::text || ' seconds')::interval,
                   now() - ($4::bigint::text || ' seconds')::interval)"#,
    )
    .bind(id)
    .bind(WS)
    .bind(success)
    .bind(age_s)
    .execute(db)
    .await?;
    Ok(())
}

/// Jobs the watchdog pushed: (path, created_by, args) rows attributed to
/// `trigger_kind = 'freshness'`.
async fn fetch_pushed(
    db: &Pool<Postgres>,
) -> anyhow::Result<Vec<(String, String, Option<serde_json::Value>)>> {
    let rows = sqlx::query!(
        r#"SELECT runnable_path AS "runnable_path!", created_by AS "created_by!",
                  args AS "args: sqlx::types::Json<serde_json::Value>"
             FROM v2_job
             WHERE workspace_id = $1 AND trigger_kind = 'freshness'
             ORDER BY created_at"#,
        WS,
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| (r.runnable_path, r.created_by, r.args.map(|a| a.0)))
        .collect())
}

async fn state_row(db: &Pool<Postgres>, path: &str) -> anyhow::Result<Option<i32>> {
    Ok(sqlx::query_scalar!(
        "SELECT attempts FROM pipeline_freshness_state WHERE workspace_id = $1 AND script_path = $2",
        WS,
        path,
    )
    .fetch_optional(db)
    .await?)
}

#[sqlx::test(fixtures("base"))]
async fn never_ran_member_is_pushed_once(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed_pipeline_script(&db, PATH, "# pipeline\n# freshness 30s\necho hi\n").await?;

    tick(&db).await;

    let pushed = fetch_pushed(&db).await?;
    assert_eq!(pushed.len(), 1, "one watchdog push expected");
    let (path, created_by, args) = &pushed[0];
    assert_eq!(path, PATH);
    assert_eq!(created_by, &format!("freshness-{PATH}"));
    let args = args.as_ref().expect("args recorded");
    assert_eq!(
        args.get("_wmill_skip_asset_dispatch"),
        Some(&serde_json::json!(true)),
        "watchdog runs must not re-fire the cascade"
    );
    assert_eq!(
        args.pointer("/trigger/kind"),
        Some(&serde_json::json!("freshness"))
    );
    assert_eq!(state_row(&db, PATH).await?, Some(1), "claim row recorded");

    // Second tick: the pushed job is queued-and-due, so the in-flight guard
    // suppresses a duplicate regardless of backoff.
    tick(&db).await;
    assert_eq!(
        fetch_pushed(&db).await?.len(),
        1,
        "no duplicate while queued"
    );

    // Simulate the queued job vanishing without a completion: the backoff
    // claim (next_attempt_at in the future) now carries the suppression.
    sqlx::query!("DELETE FROM v2_job_queue WHERE workspace_id = $1", WS)
        .execute(&db)
        .await?;
    tick(&db).await;
    assert_eq!(fetch_pushed(&db).await?.len(), 1, "backoff holds the retry");

    // Force the backoff window open: the watchdog retries and escalates.
    sqlx::query!(
        "UPDATE pipeline_freshness_state SET next_attempt_at = now() - interval '1 second'
         WHERE workspace_id = $1 AND script_path = $2",
        WS,
        PATH,
    )
    .execute(&db)
    .await?;
    tick(&db).await;
    assert_eq!(fetch_pushed(&db).await?.len(), 2, "due retry pushed");
    assert_eq!(state_row(&db, PATH).await?, Some(2), "attempts escalated");
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn fresh_member_is_skipped_and_state_reset(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed_pipeline_script(&db, PATH, "# pipeline\n# freshness 1h\necho hi\n").await?;
    seed_completed_run(&db, PATH, 10, true).await?;
    // Leftover backoff row from an earlier staleness episode.
    sqlx::query!(
        "INSERT INTO pipeline_freshness_state (workspace_id, script_path, attempts) VALUES ($1, $2, 3)",
        WS,
        PATH,
    )
    .execute(&db)
    .await?;

    tick(&db).await;

    assert!(
        fetch_pushed(&db).await?.is_empty(),
        "fresh member not pushed"
    );
    assert_eq!(state_row(&db, PATH).await?, None, "backoff reset on fresh");
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn aged_out_member_is_pushed(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    seed_pipeline_script(&db, PATH, "# pipeline\n# freshness 1h\necho hi\n").await?;
    // Old success outside the window + a recent failure: still stale.
    seed_completed_run(&db, PATH, 7200, true).await?;
    seed_completed_run(&db, PATH, 60, false).await?;

    tick(&db).await;

    assert_eq!(fetch_pushed(&db).await?.len(), 1, "aged-out member pushed");
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn partitioned_malformed_and_plain_members_are_skipped(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    // Partitioned: freshness means partition-gap detection, out of scope.
    seed_pipeline_script(
        &db,
        "u/test-user/partitioned",
        "# pipeline\n# partitioned daily\n# freshness 1h\necho hi\n",
    )
    .await?;
    // Malformed window: fails safe to unwatched.
    seed_pipeline_script(
        &db,
        "u/test-user/malformed",
        "# pipeline\n# freshness soonish\necho hi\n",
    )
    .await?;
    // Freshness only in prose (parser must reject; ILIKE prefilter passes).
    seed_pipeline_script(
        &db,
        "u/test-user/prose",
        "# pipeline\n# ensure freshness of data below\necho hi\n",
    )
    .await?;

    tick(&db).await;

    assert!(fetch_pushed(&db).await?.is_empty(), "no member is watched");
    let rows = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM pipeline_freshness_state WHERE workspace_id = $1"#,
        WS,
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(rows, 0, "no state rows for unwatched members");
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn state_of_unwatched_member_is_cleaned_up(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    // A stale-bookkeeping row whose script no longer declares freshness
    // (e.g. annotation removed and redeployed) must not survive the sweep.
    sqlx::query!(
        "INSERT INTO pipeline_freshness_state (workspace_id, script_path) VALUES ($1, $2)",
        WS,
        "u/test-user/gone",
    )
    .execute(&db)
    .await?;

    tick(&db).await;

    assert_eq!(state_row(&db, "u/test-user/gone").await?, None);
    Ok(())
}
