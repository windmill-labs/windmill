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

// A loop iteration's root_job is stored NULL, so this pins that the flow-level setting reaches
// jobs nested below the flow, and that the flow's own result stays cleared.
#[cfg(feature = "enterprise")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_level_delete_clears_every_job_of_the_run(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    use windmill_common::jobs::JobPayload;

    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let flow = serde_json::from_value(serde_json::json!({
        "delete_after_secs": 0,
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
    let flow_job =
        RunJob::from(JobPayload::RawFlow { value: flow, path: None, restarted_from: None })
            .arg("x", serde_json::json!("secret"))
            .run_until_complete(&db, false, server.addr.port())
            .await;

    let kept = sqlx::query_scalar::<_, String>(
        "WITH RECURSIVE run AS (
            SELECT id FROM v2_job WHERE id = $1
            UNION ALL
            SELECT j.id FROM v2_job j JOIN run r ON j.parent_job = r.id
        )
        SELECT COALESCE(j.flow_step_id, 'flow') || ' ' || j.args::text || ' ' || c.result::text
        FROM run JOIN v2_job j USING (id) JOIN v2_job_completed c USING (id)
        WHERE j.args <> '{}'::jsonb OR c.result <> '{}'::jsonb",
    )
    .bind(flow_job.id)
    .fetch_all(&db)
    .await?;
    let nb_jobs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM v2_job WHERE parent_job = $1")
        .bind(flow_job.id)
        .fetch_one(&db)
        .await?;

    assert!(nb_jobs > 0, "the flow should have run steps");
    assert!(kept.is_empty(), "jobs that kept args or result: {kept:?}");
    Ok(())
}
