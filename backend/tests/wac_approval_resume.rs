//! WIN-2241: each sequential `wait_for_approval()` in a WAC workflow must
//! resolve to its own approval, not the workflow's first. Guards approved,
//! cancelled and timed-out steps sharing one parent job.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::wac::{WacCheckpoint, WacPendingSteps};
use windmill_worker::wac_executor::prepare_checkpoint_for_resume;

async fn insert_resume_row(
    db: &Pool<Postgres>,
    job_id: Uuid,
    resume_id: i32,
    approver: &str,
    approved: bool,
    value: Value,
) -> anyhow::Result<()> {
    // resume_id is deliberately arbitrary (a stand-in for the random ids the
    // interactive approval channels store) — the fix must not depend on it.
    sqlx::query(
        "INSERT INTO resume_job (id, job, flow, value, approver, resume_id, approved) \
         VALUES ($1, $2, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(job_id)
    .bind(value)
    .bind(approver)
    .bind(resume_id)
    .bind(approved)
    .execute(db)
    .await?;
    Ok(())
}

fn pending_approval(mut prior: WacCheckpoint, key: &str) -> WacCheckpoint {
    prior.pending_steps = Some(WacPendingSteps {
        mode: "approval".to_string(),
        keys: vec![key.to_string()],
        job_ids: Default::default(),
    });
    prior
}

#[sqlx::test]
async fn wac_sequential_approvals_read_own_row(db: Pool<Postgres>) -> anyhow::Result<()> {
    let job_id = Uuid::new_v4();
    // FK target for resume_job.flow and v2_job_status.id (written by save_checkpoint).
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for) VALUES ($1, 'test-workspace', now())",
    )
    .bind(job_id)
    .execute(&db)
    .await?;

    // Step A: approved.
    insert_resume_row(&db, job_id, 1, "alice", true, json!({"n": 1})).await?;
    let ckpt = pending_approval(WacCheckpoint::default(), "apprA");
    let ckpt = prepare_checkpoint_for_resume(&db, &job_id, ckpt).await?;
    assert_eq!(ckpt.completed_steps["apprA"]["approved"], json!(true));
    assert_eq!(ckpt.completed_steps["apprA"]["approver"], json!("alice"));
    assert_eq!(ckpt.completed_steps["apprA"]["value"], json!({"n": 1}));

    // Step B: cancelled. Carrying `ckpt` forward preserves consumed_resume_row_ids,
    // so B must resolve to its own row, not A's stale approved=true.
    insert_resume_row(&db, job_id, 2, "bob", false, json!({"n": 2})).await?;
    let ckpt = pending_approval(ckpt, "apprB");
    let ckpt = prepare_checkpoint_for_resume(&db, &job_id, ckpt).await?;
    assert_eq!(
        ckpt.completed_steps["apprB"]["approved"],
        json!(false),
        "2nd approval must read its own (cancelled) row, not the 1st's"
    );
    assert_eq!(ckpt.completed_steps["apprB"]["approver"], json!("bob"));

    // Step C: timed out — no resume_job row. Falls to the approved=false default.
    let ckpt = pending_approval(ckpt, "apprC");
    let ckpt = prepare_checkpoint_for_resume(&db, &job_id, ckpt).await?;
    assert_eq!(ckpt.completed_steps["apprC"]["approved"], json!(false));
    assert_eq!(ckpt.completed_steps["apprC"]["approver"], json!(null));

    Ok(())
}

/// A row carrying another step's bound resume_id belongs to that step. The API
/// refuses such resumes but cannot do so atomically with the insert, so
/// consumption must skip them however they got in — while leaving the random ids
/// the interactive channels sign fully eligible.
#[sqlx::test]
async fn wac_approval_skips_another_steps_bound_row(db: Pool<Postgres>) -> anyhow::Result<()> {
    let job_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for) VALUES ($1, 'test-workspace', now())",
    )
    .bind(job_id)
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_status (id, workflow_as_code_status) VALUES ($1, $2)",
    )
    .bind(job_id)
    .bind(sqlx::types::Json(json!({
        "_minted_approval_keys": { "legal": true, "finance": true }
    })))
    .execute(&db)
    .await?;

    // Oldest row is bound to `finance`; the later one is a plain Slack-style resume.
    insert_resume_row(
        &db,
        job_id,
        windmill_common::wac::approval_resume_id("finance") as i32,
        "bob",
        true,
        json!({"n": "finance"}),
    )
    .await?;
    insert_resume_row(&db, job_id, 4242, "alice", true, json!({"n": "slack"})).await?;

    let ckpt = pending_approval(WacCheckpoint::default(), "legal");
    let ckpt = prepare_checkpoint_for_resume(&db, &job_id, ckpt).await?;
    assert_eq!(
        ckpt.completed_steps["legal"]["approver"],
        json!("alice"),
        "`legal` must skip finance's bound row and take the unbound one"
    );

    Ok(())
}
