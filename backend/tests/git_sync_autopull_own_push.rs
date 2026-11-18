//! A deploy push built on the synced head advances the synced sha, so auto-pull does
//! not pull Windmill's own commit back over newer workspace edits. A push built on
//! anything else, or one a pull may have overlapped, leaves it alone, so the next
//! poll still pulls.
#![cfg(all(feature = "enterprise", feature = "private"))]

use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_git_sync::advance_synced_sha_past_own_push;

async fn synced_main(db: &Pool<Postgres>) -> anyhow::Result<String> {
    Ok(sqlx::query_scalar(
        "SELECT git_sync->'repositories'->0->'auto_pull'->'last_synced_sha'->>'main'
         FROM workspace_settings WHERE workspace_id = 'ap-ws'",
    )
    .fetch_one(db)
    .await?)
}

/// A completed job of `path` that ran from `started` to `completed` seconds ago.
async fn completed_job(
    db: &Pool<Postgres>,
    path: &str,
    started: i32,
    completed: i32,
) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO v2_job (id, workspace_id, runnable_path) VALUES ($1, 'ap-ws', $2)")
        .bind(id)
        .bind(path)
        .execute(db)
        .await?;
    sqlx::query(
        "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status, started_at, completed_at)
         VALUES ($1, 'ap-ws', 0, 'success', now() - make_interval(secs => $2), now() - make_interval(secs => $3))",
    )
    .bind(id)
    .bind(started)
    .bind(completed)
    .execute(db)
    .await?;
    Ok(id)
}

#[sqlx::test(fixtures("git_sync_autopull_recovery"))]
async fn own_push_advances_only_from_the_synced_head(db: Pool<Postgres>) -> anyhow::Result<()> {
    let push = completed_job(&db, "hub/1/sync", 10, 0).await?;
    // Job args name the repository without the `$res:` prefix the settings store.
    advance_synced_sha_past_own_push(&db, "ap-ws", "u/admin/repo", "main", "aaa", "w1", push)
        .await?;
    assert_eq!(synced_main(&db).await?, "w1");

    // Built on a commit the workspace has not pulled: the pull must still happen.
    advance_synced_sha_past_own_push(&db, "ap-ws", "u/admin/repo", "main", "foreign", "w2", push)
        .await?;
    assert_eq!(synced_main(&db).await?, "w1");

    // A pull that finished while the push ran may have reverted what it pushed, whatever
    // hub version of the pull script it ran.
    let push = completed_job(&db, "hub/1/sync", 10, 0).await?;
    completed_job(&db, "hub/99999/git-sync-init-repository-windmill", 20, 5).await?;
    advance_synced_sha_past_own_push(&db, "ap-ws", "u/admin/repo", "main", "w1", "w3", push)
        .await?;
    assert_eq!(synced_main(&db).await?, "w1");
    Ok(())
}
