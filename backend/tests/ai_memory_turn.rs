//! A memory id is held by one queued run at a time: a second run that claims it is refused, even
//! when both claim at once, and the id is free again once its run leaves the queue. A run whose
//! AI agent is parked at its last step hands the id over instead.

use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;
use windmill_api::jobs::set_flow_memory_id;
use windmill_common::error::Error;
use windmill_test_utils::*;

const WS: &str = "test-workspace";
const MEMORY: Uuid = Uuid::from_u128(0x3e3);

/// Queue a run in `tx` and claim the memory id for it, as a push does.
async fn push_with_memory(tx: &mut Transaction<'_, Postgres>) -> anyhow::Result<(Uuid, bool)> {
    let job_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO v2_job (id, workspace_id, kind) VALUES ($1, $2, 'flow')",
        job_id,
        WS
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for) VALUES ($1, $2, now())",
        job_id,
        WS
    )
    .execute(&mut **tx)
    .await?;
    match set_flow_memory_id(tx, WS, job_id, MEMORY).await {
        Ok(()) => Ok((job_id, true)),
        Err(Error::Generic(status, _)) if status.as_u16() == 409 => Ok((job_id, false)),
        Err(e) => Err(e.into()),
    }
}

#[sqlx::test(fixtures("base"))]
async fn test_one_queued_run_holds_a_memory_id(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    // Two claims at once: the second waits on the first's row, then must see its run queued.
    let mut first = db.begin().await?;
    let (first_job, claimed) = push_with_memory(&mut first).await?;
    assert!(claimed);
    let second = tokio::spawn({
        let db = db.clone();
        async move {
            let mut tx = db.begin().await?;
            let outcome = push_with_memory(&mut tx).await?;
            tx.commit().await?;
            anyhow::Ok(outcome)
        }
    });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    first.commit().await?;
    let (_, claimed) = second.await??;
    assert!(
        !claimed,
        "a run claimed the memory id another queued run holds"
    );

    // Once the holder leaves the queue, the next run takes the id over.
    sqlx::query!("DELETE FROM v2_job_queue WHERE id = $1", first_job)
        .execute(&db)
        .await?;
    let mut tx = db.begin().await?;
    let (third_job, claimed) = push_with_memory(&mut tx).await?;
    tx.commit().await?;
    assert!(claimed);
    let holder = sqlx::query_scalar!(
        "SELECT job_id FROM ai_memory_turn WHERE workspace_id = $1 AND memory_id = $2",
        WS,
        MEMORY
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(holder, third_job);
    Ok(())
}

/// A queued run holding the memory id, at step `at` of `steps` top-level steps, where an AI agent
/// step is parked on one background job. Returns the agent's job.
async fn hold_with_parked_agent(
    db: &Pool<Postgres>,
    steps: usize,
    at: usize,
) -> anyhow::Result<Uuid> {
    let mut tx = db.begin().await?;
    let (holder, claimed) = push_with_memory(&mut tx).await?;
    assert!(claimed);
    let agent = Uuid::new_v4();
    let modules: Vec<serde_json::Value> = (0..steps)
        .map(|i| match i == at {
            true => serde_json::json!({ "type": "InProgress", "id": "a", "job": agent }),
            false => serde_json::json!({ "type": "WaitingForPriorSteps", "id": format!("s{i}") }),
        })
        .collect();
    let flow_status = serde_json::json!({ "step": at, "modules": modules });
    sqlx::query!(
        "INSERT INTO v2_job_status (id, flow_status) VALUES ($1, $2)",
        holder,
        flow_status
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO v2_job (id, workspace_id, kind, parent_job, flow_step_id)
         VALUES ($1, $2, 'aiagent', $3, 'a')",
        agent,
        WS,
        holder
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, suspend, suspend_until)
         VALUES ($1, $2, now(), 1, now() + interval '1 hour')",
        agent,
        WS
    )
    .execute(&mut *tx)
    .await?;
    let checkpoint = serde_json::json!({ "background": [{ "job_id": BACKGROUND_JOB }] });
    sqlx::query!(
        "INSERT INTO ai_agent_checkpoint (job_id, workspace_id, checkpoint, waiting_on)
         VALUES ($1, $2, $3, $4)",
        agent,
        WS,
        checkpoint,
        &[BACKGROUND_JOB]
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO v2_job (id, workspace_id, kind, parent_job, root_job)
         VALUES ($1, $2, 'script', $3, $4)",
        BACKGROUND_JOB,
        WS,
        agent,
        holder
    )
    .execute(&mut *tx)
    .await?;
    // A nested agent's own background job, under the handed one.
    sqlx::query!(
        "INSERT INTO v2_job (id, workspace_id, kind, parent_job, root_job)
         VALUES ($1, $2, 'script', $3, $4)",
        NESTED_JOB,
        WS,
        BACKGROUND_JOB,
        holder
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for)
         VALUES ($1, $2, now()), ($3, $2, now())",
        BACKGROUND_JOB,
        WS,
        NESTED_JOB
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(agent)
}

const BACKGROUND_JOB: Uuid = Uuid::from_u128(0xb6);
const NESTED_JOB: Uuid = Uuid::from_u128(0xb7);

#[sqlx::test(fixtures("base"))]
async fn test_a_parked_last_step_hands_its_memory_id_over(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let agent = hold_with_parked_agent(&db, 2, 1).await?;

    let mut tx = db.begin().await?;
    let (new_run, claimed) = push_with_memory(&mut tx).await?;
    tx.commit().await?;
    assert!(claimed, "a run parked at its last step refused a new one");

    let turn = sqlx::query!(
        "SELECT job_id, handed_jobs FROM ai_memory_turn WHERE workspace_id = $1 AND memory_id = $2",
        WS,
        MEMORY
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(turn.job_id, new_run);
    assert_eq!(
        turn.handed_jobs,
        Some(serde_json::json!({ "a": [{ "job_id": BACKGROUND_JOB }] }))
    );
    let woken = sqlx::query!(
        r#"SELECT q.suspend, c.checkpoint->'handed_over' AS "handed_over", c.waiting_on
           FROM v2_job_queue q JOIN ai_agent_checkpoint c ON c.job_id = q.id WHERE q.id = $1"#,
        agent
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(woken.suspend, 0);
    assert_eq!(woken.handed_over, Some(serde_json::json!(true)));
    assert!(woken.waiting_on.is_empty());
    // The new run ends before its agent adopts the job: the next claim cancels it.
    sqlx::query!("DELETE FROM v2_job_queue WHERE id = $1", new_run)
        .execute(&db)
        .await?;
    let mut tx = db.begin().await?;
    let (_, claimed) = push_with_memory(&mut tx).await?;
    tx.commit().await?;
    assert!(claimed);
    let cancelled = sqlx::query_scalar!(
        "SELECT canceled_by FROM v2_job_queue WHERE id = ANY($1)",
        &[BACKGROUND_JOB, NESTED_JOB]
    )
    .fetch_all(&db)
    .await?;
    assert_eq!(cancelled, vec![Some("monitor".to_string()); 2]);
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_a_parked_step_with_steps_after_it_keeps_its_memory_id(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let agent = hold_with_parked_agent(&db, 2, 0).await?;

    let mut tx = db.begin().await?;
    let (_, claimed) = push_with_memory(&mut tx).await?;
    tx.rollback().await?;
    assert!(
        !claimed,
        "a run with a step after its parked agent handed its memory id over"
    );

    let suspend = sqlx::query_scalar!("SELECT suspend FROM v2_job_queue WHERE id = $1", agent)
        .fetch_one(&db)
        .await?;
    assert_eq!(suspend, 1, "the refused hand-over woke the parked agent");
    Ok(())
}
