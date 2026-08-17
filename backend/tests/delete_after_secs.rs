use sqlx::{Pool, Postgres};
use windmill_common::jobs::{resolve_delete_after_secs, schedule_job_deletion};
use windmill_test_utils::*;

// ---------------------------------------------------------------------------
// Unit tests for resolve_delete_after_secs
// ---------------------------------------------------------------------------

#[test]
fn test_resolve_no_deletion() {
    assert_eq!(resolve_delete_after_secs(None, None), None);
    assert_eq!(resolve_delete_after_secs(Some(false), None), None);
}

#[test]
fn test_resolve_immediate_backward_compat() {
    // delete_after_use=true with no secs → immediate (0)
    assert_eq!(resolve_delete_after_secs(Some(true), None), Some(0));
}

#[test]
fn test_resolve_explicit_secs() {
    assert_eq!(resolve_delete_after_secs(None, Some(0)), Some(0));
    assert_eq!(resolve_delete_after_secs(None, Some(3600)), Some(3600));
    assert_eq!(resolve_delete_after_secs(Some(true), Some(60)), Some(60));
    assert_eq!(resolve_delete_after_secs(Some(false), Some(120)), Some(120));
}

#[test]
fn test_resolve_rejects_negative() {
    assert_eq!(resolve_delete_after_secs(None, Some(-1)), None);
    assert_eq!(resolve_delete_after_secs(Some(true), Some(-100)), None);
}

// ---------------------------------------------------------------------------
// Integration: schedule_job_deletion inserts into job_delete_schedule
// ---------------------------------------------------------------------------

#[sqlx::test(fixtures("base"))]
async fn test_schedule_job_deletion_inserts_row(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let job_id = uuid::Uuid::new_v4();

    schedule_job_deletion(&db, job_id, "test-workspace", 3600).await?;

    let row = sqlx::query_as::<_, (uuid::Uuid, String)>(
        "SELECT job_id, workspace_id FROM job_delete_schedule WHERE job_id = $1",
    )
    .bind(job_id)
    .fetch_one(&db)
    .await?;

    assert_eq!(row.0, job_id);
    assert_eq!(row.1, "test-workspace");

    // Verify delete_at is approximately now + 3600s
    let delete_at: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT delete_at FROM job_delete_schedule WHERE job_id = $1")
            .bind(job_id)
            .fetch_one(&db)
            .await?;

    let expected_min = chrono::Utc::now() + chrono::Duration::seconds(3500);
    let expected_max = chrono::Utc::now() + chrono::Duration::seconds(3700);
    assert!(
        delete_at > expected_min && delete_at < expected_max,
        "delete_at should be ~1 hour from now, got {delete_at}"
    );

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_schedule_job_deletion_is_idempotent(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let job_id = uuid::Uuid::new_v4();

    schedule_job_deletion(&db, job_id, "test-workspace", 60).await?;
    // Second call should not error (ON CONFLICT DO NOTHING)
    schedule_job_deletion(&db, job_id, "test-workspace", 120).await?;

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM job_delete_schedule WHERE job_id = $1")
            .bind(job_id)
            .fetch_one(&db)
            .await?;
    assert_eq!(count, 1, "should have exactly one row (idempotent)");

    Ok(())
}
