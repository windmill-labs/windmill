//! Guards what `suspend_wac_parent` promises: the `started_at` invariant documented on
//! it, the segment length it hands back for metering, and that it stands down for a
//! cancel already on the row.

use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_worker::wac_executor::{suspend_wac_parent, WacPark};

#[sqlx::test]
async fn wac_suspend_clears_started_at(db: Pool<Postgres>) -> anyhow::Result<()> {
    let job_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, started_at) \
         VALUES ($1, 'test-workspace', now(), true, now() - interval '4 days')",
    )
    .bind(job_id)
    .execute(&db)
    .await?;

    let mut tx = db.begin().await?;
    let WacPark::Parked(segment_ms) =
        suspend_wac_parent(&mut tx, &job_id, "test-workspace", 1, 3600.0).await?
    else {
        panic!("an uncancelled parent must park");
    };
    tx.commit().await?;

    // The segment is what gets billed, so it must be the run that just ended, measured
    // from the pull — not the park ahead of it, and not zero.
    let four_days_ms = 4 * 24 * 3600 * 1000;
    assert!(
        segment_ms.is_some_and(|ms| (ms - four_days_ms).abs() < 60_000),
        "expected the ended segment (~{four_days_ms}ms), got {segment_ms:?}"
    );

    let (started_at, running, suspend, suspend_until): (
        Option<chrono::DateTime<chrono::Utc>>,
        bool,
        i32,
        Option<chrono::DateTime<chrono::Utc>>,
    ) = sqlx::query_as(
        "SELECT started_at, running, suspend, suspend_until FROM v2_job_queue WHERE id = $1",
    )
    .bind(job_id)
    .fetch_one(&db)
    .await?;

    assert_eq!(
        started_at, None,
        "a parked parent must not carry the previous segment's started_at"
    );
    assert_eq!(suspend, 1);
    assert!(suspend_until.is_some());
    assert!(
        running,
        "running stays true so the normal pull query skips the parked row"
    );

    Ok(())
}

/// A soft cancel sets `canceled_by` and `suspend = 0` and leaves acting on it to the next
/// pull. Parking over that holds the row until `suspend_until` — a whole day on a
/// `sleep(86400)` — so the park has to stand down and let the job complete instead.
#[sqlx::test]
async fn wac_suspend_stands_down_for_a_cancel(db: Pool<Postgres>) -> anyhow::Result<()> {
    let job_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job_queue \
           (id, workspace_id, scheduled_for, running, started_at, suspend, canceled_by, canceled_reason) \
         VALUES ($1, 'test-workspace', now(), true, now() - interval '30 seconds', 0, 'alice', 'no longer needed')",
    )
    .bind(job_id)
    .execute(&db)
    .await?;

    let mut tx = db.begin().await?;
    let parked = suspend_wac_parent(&mut tx, &job_id, "test-workspace", 1, 86400.0).await?;
    tx.commit().await?;

    match &parked {
        WacPark::Cancelled(cancel) => {
            assert_eq!(cancel.username.as_deref(), Some("alice"));
            assert_eq!(cancel.reason.as_deref(), Some("no longer needed"));
        }
        other => panic!("a cancelled parent must not park, got {other:?}"),
    }

    let (suspend, suspend_until, started_at): (
        i32,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
    ) = sqlx::query_as(
        "SELECT suspend, suspend_until, started_at FROM v2_job_queue WHERE id = $1",
    )
    .bind(job_id)
    .fetch_one(&db)
    .await?;

    assert_eq!(suspend, 0, "the cancel's suspend = 0 must survive");
    assert_eq!(
        suspend_until, None,
        "a suspend_until would hold the row back for the whole park window"
    );
    assert!(
        started_at.is_some(),
        "the segment ran, so its start must stay for the completion's duration"
    );

    Ok(())
}

/// Each park clears `started_at`, so the completion would otherwise store only the last
/// segment: a workflow that waited an hour on its tasks would read as a sub-second job that
/// started an hour late. The completed row spans the workflow from its first start, and
/// keeps the last segment aside: that is what is metered and what worker-time readers sum.
#[sqlx::test]
async fn wac_completion_spans_the_workflow_and_keeps_the_last_segment(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let job_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, created_at, permissioned_as, \
            permissioned_as_email, kind, script_lang, runnable_path, tag, visible_to_owner) \
         VALUES ($1, 'test-workspace', 'test-user', now() - interval '2 hours', 'u/test-user', \
            'test@windmill.dev', 'script', 'bun', 'u/test-user/wac', 'bun', true)",
    )
    .bind(job_id)
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, tag, started_at) \
         VALUES ($1, 'test-workspace', now(), true, 'bun', now() - interval '1 hour')",
    )
    .bind(job_id)
    .execute(&db)
    .await?;

    // Two parks, each followed by the pull re-stamping `started_at`: the second park must
    // not replace the first start with its own segment's.
    for resumed_ago in ["10 minutes", "3 seconds"] {
        let mut tx = db.begin().await?;
        suspend_wac_parent(&mut tx, &job_id, "test-workspace", 1, 3600.0).await?;
        tx.commit().await?;
        sqlx::query(
            "UPDATE v2_job_queue SET suspend = 0, started_at = now() - $2::text::interval \
             WHERE id = $1",
        )
        .bind(job_id)
        .bind(resumed_ago)
        .execute(&db)
        .await?;
    }

    let job = windmill_queue::get_mini_completed_job(&job_id, "test-workspace", &db)
        .await?
        .unwrap();
    let result = serde_json::value::to_raw_value(&serde_json::json!("done"))?;
    let (_, span) = windmill_queue::add_completed_job(
        &db,
        &job,
        true,
        false,
        sqlx::types::Json(&result),
        None,
        0,
        None,
        false,
        Some(3000),
        false,
    )
    .await?;

    let (started_ago_s, duration_ms, last_segment_ms): (f64, i64, Option<i64>) = sqlx::query_as(
        "SELECT extract(epoch FROM now() - started_at)::float8, duration_ms, \
                (extras->>'wac_last_segment_ms')::bigint \
         FROM v2_job_completed WHERE id = $1",
    )
    .bind(job_id)
    .fetch_one(&db)
    .await?;
    assert!(
        (started_ago_s - 3600.0).abs() < 60.0,
        "started_at must be the first start, got {started_ago_s}s ago"
    );
    assert!(
        (duration_ms - 3_600_000).abs() < 60_000,
        "duration_ms must be the wall time since the first start, got {duration_ms}"
    );
    assert_eq!(last_segment_ms, Some(3000));
    assert_eq!(
        span.duration_ms, duration_ms,
        "a flow records the step with the span the row holds"
    );
    assert!(span.outlasts_run(job.started_at));
    assert_eq!(span.last_run_ms, 3000, "only the last segment is left to meter");

    Ok(())
}
