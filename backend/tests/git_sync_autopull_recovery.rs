//! A recorded auto-pull failure is cleared once the tracked head is observed again
//! at the already-synced sha, and only then: the decision is taken on a snapshot,
//! so the write must re-check the stored row rather than overwrite it.
#![cfg(all(feature = "enterprise", feature = "private"))]

use sqlx::{Pool, Postgres};
use std::collections::HashMap;
use uuid::Uuid;
use windmill_common::workspaces::AutoPullStatus;
use windmill_git_sync::{clear_auto_pull_failure, persist_auto_pull_state};

const WS: &str = "ap-ws";
const REPO: &str = "$res:u/admin/repo";
/// `at` of the failure the fixture records.
const FIXTURE_FAILED_AT: i64 = 1;

fn recovered(head: &str) -> AutoPullStatus {
    AutoPullStatus {
        synced_sha: Some(head.to_string()),
        at: 2,
        job_id: None,
        success: true,
        error: None,
    }
}

async fn stored_auto_pull(db: &Pool<Postgres>) -> anyhow::Result<serde_json::Value> {
    let git_sync: serde_json::Value =
        sqlx::query_scalar("SELECT git_sync FROM workspace_settings WHERE workspace_id = $1")
            .bind(WS)
            .fetch_one(db)
            .await?;
    Ok(git_sync["repositories"][0]["auto_pull"].clone())
}

#[sqlx::test(fixtures("git_sync_autopull_recovery"))]
async fn recovery_clears_the_failure_at_the_synced_head(db: Pool<Postgres>) -> anyhow::Result<()> {
    clear_auto_pull_failure(
        &db,
        WS,
        REPO,
        "main",
        "aaa",
        FIXTURE_FAILED_AT,
        &recovered("aaa"),
    )
    .await?;

    let auto_pull = stored_auto_pull(&db).await?;
    assert_eq!(auto_pull["last_pull_status"]["success"], true);
    assert!(auto_pull["last_pull_status"].get("error").is_none());
    assert_eq!(auto_pull["last_pull_status"]["synced_sha"], "aaa");
    assert_eq!(
        auto_pull["last_synced_sha"]["main"], "aaa",
        "the sha map is not part of a recovery write"
    );
    Ok(())
}

/// Between the poller observing head "aaa" unchanged and its recovery write, a
/// webhook may have enqueued newer head "bbb". The stale recovery must leave that
/// optimistic state (sha, success, job id) in place; the job's completion hook
/// relies on it, and rolling the sha back would re-enqueue "bbb" on the next tick.
#[sqlx::test(fixtures("git_sync_autopull_recovery"))]
async fn stale_recovery_leaves_a_newer_state_alone(db: Pool<Postgres>) -> anyhow::Result<()> {
    let job_id = Uuid::new_v4();
    let advanced = AutoPullStatus {
        synced_sha: Some("bbb".to_string()),
        at: 3,
        job_id: Some(job_id),
        success: true,
        error: None,
    };
    persist_auto_pull_state(
        &db,
        WS,
        REPO,
        &HashMap::from([("main".to_string(), "bbb".to_string())]),
        &advanced,
    )
    .await?;

    clear_auto_pull_failure(
        &db,
        WS,
        REPO,
        "main",
        "aaa",
        FIXTURE_FAILED_AT,
        &recovered("aaa"),
    )
    .await?;

    let auto_pull = stored_auto_pull(&db).await?;
    assert_eq!(auto_pull["last_synced_sha"]["main"], "bbb");
    assert_eq!(auto_pull["last_pull_status"]["synced_sha"], "bbb");
    assert_eq!(auto_pull["last_pull_status"]["job_id"], job_id.to_string());
    assert_eq!(auto_pull["last_pull_status"]["at"], 3);
    Ok(())
}

/// The head can stay at "aaa" while a newer failure is recorded (a later head
/// check, a pull job that failed). A recovery decided on the older failure must
/// not paper over the newer one.
#[sqlx::test(fixtures("git_sync_autopull_recovery"))]
async fn stale_recovery_keeps_a_newer_failure_at_the_same_head(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let newer_failure = AutoPullStatus {
        synced_sha: None,
        at: 5,
        job_id: None,
        success: false,
        error: Some("head check failed: newer".to_string()),
    };
    persist_auto_pull_state(
        &db,
        WS,
        REPO,
        &HashMap::from([("main".to_string(), "aaa".to_string())]),
        &newer_failure,
    )
    .await?;

    clear_auto_pull_failure(
        &db,
        WS,
        REPO,
        "main",
        "aaa",
        FIXTURE_FAILED_AT,
        &recovered("aaa"),
    )
    .await?;

    let auto_pull = stored_auto_pull(&db).await?;
    assert_eq!(auto_pull["last_pull_status"]["success"], false);
    assert_eq!(auto_pull["last_pull_status"]["at"], 5);
    assert_eq!(
        auto_pull["last_pull_status"]["error"],
        "head check failed: newer"
    );
    Ok(())
}
