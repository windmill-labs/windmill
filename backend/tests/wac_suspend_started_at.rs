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
