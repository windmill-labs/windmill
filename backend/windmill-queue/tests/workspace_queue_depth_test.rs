//! Regression guard for `workspace_queue_depth`, which backs the cloud-only cap on how many
//! jobs a single workspace may have queued in total.
//!
//! Run with:
//!   cargo test -p windmill-queue --test workspace_queue_depth_test

use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_queue::jobs::workspace_queue_depth;

/// Queues `count` jobs in `workspace`, scheduled `offset_secs` from now, with `running` state.
async fn seed(db: &Pool<Postgres>, workspace: &str, count: usize, offset_secs: i64, running: bool) {
    for _ in 0..count {
        let id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO v2_job (id, workspace_id, tag) VALUES ($1, $2, 'other')",
            id,
            workspace,
        )
        .execute(db)
        .await
        .expect("seed v2_job");
        sqlx::query!(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag, running)
             VALUES ($1, $2, now() + ($3::bigint::text || ' s')::interval, 'other', $4)",
            id,
            workspace,
            offset_secs,
            running,
        )
        .execute(db)
        .await
        .expect("seed v2_job_queue");
    }
}

/// The count is per workspace and ignores running jobs: a runaway in one workspace must not
/// charge another, and jobs already executing are not backlog. Future-scheduled jobs count,
/// since a concurrency-parked backlog is almost entirely future-dated.
#[sqlx::test(migrations = "../migrations")]
async fn scoped_to_workspace_and_ignores_running(db: Pool<Postgres>) {
    seed(&db, "ws-a", 4, 3600, false).await; // waiting (parked in the future)
    seed(&db, "ws-a", 3, -10, false).await; // waiting (due now)
    seed(&db, "ws-a", 5, -10, true).await; // running — not backlog
    seed(&db, "ws-b", 9, 3600, false).await; // a different workspace

    let depth = workspace_queue_depth(&db, "ws-a", 1000)
        .await
        .expect("count depth");

    assert_eq!(
        depth, 7,
        "only ws-a's 7 waiting jobs count; running jobs and ws-b must not"
    );
}

/// The scan stops at `limit`, so a runaway backlog does not cost an unbounded count on every
/// push. The cap only needs to know the depth has reached the ceiling.
#[sqlx::test(migrations = "../migrations")]
async fn bounded_by_limit(db: Pool<Postgres>) {
    seed(&db, "ws-a", 50, -10, false).await;

    let depth = workspace_queue_depth(&db, "ws-a", 10)
        .await
        .expect("count depth");

    assert_eq!(
        depth, 10,
        "the count must stop at the limit, not scan all 50"
    );
}
