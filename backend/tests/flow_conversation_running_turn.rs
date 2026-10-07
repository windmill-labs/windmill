//! A conversation answers one message at a time: while the run its newest user message started
//! is queued or running, the list reports that turn and a run into the conversation is refused.
//!
//! Uses runtime `sqlx::query` (not the compile-time macros) so no offline query cache is
//! needed, matching v2_job_delete_orphans.rs.

use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::error::Error;
use windmill_common::flow_conversations::{get_or_create_conversation_with_id, running_turns};
use windmill_test_utils::*;

const WS: &str = "test-workspace";
const CONV: Uuid = Uuid::from_u128(0x5eed);

/// Takes the conversation for a turn, as a run does, without keeping what it writes.
async fn take_conversation(db: &Pool<Postgres>) -> windmill_common::error::Result<Uuid> {
    let mut tx = db.begin().await?;
    let result =
        get_or_create_conversation_with_id(&mut tx, WS, "f/flow", "test-user", "t", CONV, false)
            .await;
    tx.rollback().await?;
    result.map(|c| c.id)
}

#[sqlx::test(fixtures("base"))]
async fn test_a_conversation_refuses_a_turn_while_one_runs(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let job_id = Uuid::new_v4();
    sqlx::query("INSERT INTO v2_job (id, workspace_id, kind) VALUES ($1, $2, 'flow')")
        .bind(job_id)
        .bind(WS)
        .execute(&db)
        .await?;
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for) VALUES ($1, $2, now())",
    )
    .bind(job_id)
    .bind(WS)
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO flow_conversation (id, workspace_id, flow_path, created_by)
         VALUES ($1, $2, 'f/flow', 'test-user')",
    )
    .bind(CONV)
    .bind(WS)
    .execute(&db)
    .await?;
    let user_seq: i64 = sqlx::query_scalar(
        "INSERT INTO flow_conversation_message (conversation_id, message_type, content, job_id)
         VALUES ($1, 'user', 'hi', $2) RETURNING created_seq",
    )
    .bind(CONV)
    .bind(job_id)
    .fetch_one(&db)
    .await?;

    let running = running_turns(&db, &[CONV]).await?;
    let turn = running
        .get(&CONV)
        .expect("the queued run is the conversation's running turn");
    assert_eq!((turn.job_id, turn.user_seq), (job_id, user_seq));

    match take_conversation(&db).await {
        Err(Error::Generic(status, body)) => {
            assert_eq!(status.as_u16(), 409);
            assert!(
                body.contains(&job_id.to_string()),
                "the refusal names the running job: {body}"
            );
        }
        other => panic!("expected a 409 while the first turn runs, got {other:?}"),
    }

    // The run ends: it leaves the queue, and the conversation takes the next message.
    sqlx::query("DELETE FROM v2_job_queue WHERE id = $1")
        .bind(job_id)
        .execute(&db)
        .await?;
    assert!(running_turns(&db, &[CONV]).await?.is_empty());
    assert_eq!(take_conversation(&db).await?, CONV);
    Ok(())
}
