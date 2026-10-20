//! A memory id is held by one queued run at a time: a second run that claims it is refused, even
//! when both claim at once, and the id is free again once its run leaves the queue.
//!
//! Uses runtime `sqlx::query` (not the compile-time macros) so no offline query cache is
//! needed, matching flow_conversation_running_turn.rs.

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
    sqlx::query("INSERT INTO v2_job (id, workspace_id, kind) VALUES ($1, $2, 'flow')")
        .bind(job_id)
        .bind(WS)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for) VALUES ($1, $2, now())",
    )
    .bind(job_id)
    .bind(WS)
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
    sqlx::query("DELETE FROM v2_job_queue WHERE id = $1")
        .bind(first_job)
        .execute(&db)
        .await?;
    let mut tx = db.begin().await?;
    let (third_job, claimed) = push_with_memory(&mut tx).await?;
    tx.commit().await?;
    assert!(claimed);
    let holder: Uuid = sqlx::query_scalar(
        "SELECT job_id FROM ai_memory_turn WHERE workspace_id = $1 AND memory_id = $2",
    )
    .bind(WS)
    .bind(MEMORY)
    .fetch_one(&db)
    .await?;
    assert_eq!(holder, third_job);
    Ok(())
}
