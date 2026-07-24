//! Regression test for WAC `waitForApproval` reading the wrong approval result.
//!
//! Sequential `waitForApproval()` calls share the parent job_id, so the resume
//! lookup must select the `resume_job` row whose `resume_id` matches the pending
//! approval step's key — not the oldest row for the job. Before the fix, a job's
//! second approval re-read the first approval's value (and cancels/timeouts
//! inherited the first approval's `approved=true`).

use serde_json::json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use windmill_common::wac::{save_checkpoint, WacCheckpoint, WacPendingSteps};
use windmill_worker::wac_executor::{approval_resume_id, prepare_checkpoint_for_resume};

async fn insert_resume(
    db: &Pool<Postgres>,
    job_id: Uuid,
    key: &str,
    value: serde_json::Value,
    approved: bool,
    age_seconds: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO resume_job (id, job, flow, created_at, value, approver, resume_id, approved)
         VALUES ($1, $2, $2, now() - make_interval(secs => $3), $4, 'tester', $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(job_id)
    .bind(age_seconds as f64)
    .bind(value)
    .bind(approval_resume_id(key) as i32)
    .bind(approved)
    .execute(db)
    .await?;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn resume_matches_pending_approval_step(db: Pool<Postgres>) -> anyhow::Result<()> {
    let job_id = Uuid::new_v4();

    // Minimal queue row so the resume_job.flow FK and the v2_job_status FK resolve.
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, suspend)
         VALUES ($1, 'test-workspace', now(), 1)",
    )
    .bind(job_id)
    .execute(&db)
    .await?;

    // The workflow is parked on its *second* approval.
    let pending_key = "approval_2";
    let checkpoint = WacCheckpoint {
        pending_steps: Some(WacPendingSteps {
            mode: "approval".to_string(),
            keys: vec![pending_key.to_string()],
            job_ids: Default::default(),
        }),
        ..Default::default()
    };
    save_checkpoint(&db, &job_id, &checkpoint).await?;

    // Two resume events for the same job: the first approval (older, approved) and
    // the pending second approval (newer, cancelled). The pre-fix query
    // `WHERE job = $1 ORDER BY created_at ASC LIMIT 1` would return the older row.
    insert_resume(
        &db,
        job_id,
        "approval",
        json!({ "which": "first" }),
        true,
        10,
    )
    .await?;
    insert_resume(
        &db,
        job_id,
        pending_key,
        json!({ "which": "second" }),
        false,
        0,
    )
    .await?;

    let resumed = prepare_checkpoint_for_resume(&db, &job_id, checkpoint).await?;

    let injected = resumed
        .completed_steps
        .get(pending_key)
        .expect("pending approval step should be resolved");
    assert_eq!(
        injected["value"],
        json!({ "which": "second" }),
        "must read its own resume event, not the workflow's first approval"
    );
    assert_eq!(
        injected["approved"],
        json!(false),
        "the cancelled second approval must keep approved=false"
    );

    Ok(())
}
