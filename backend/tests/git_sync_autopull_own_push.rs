//! A deploy push built on the synced head advances the synced sha, so auto-pull does
//! not pull Windmill's own commit back over newer workspace edits. A push built on
//! anything else leaves it alone, so the commits it sits on still get pulled.
#![cfg(all(feature = "enterprise", feature = "private"))]

use sqlx::{Pool, Postgres};
use windmill_git_sync::advance_synced_sha_past_own_push;

async fn synced_main(db: &Pool<Postgres>) -> anyhow::Result<String> {
    Ok(sqlx::query_scalar(
        "SELECT git_sync->'repositories'->0->'auto_pull'->'last_synced_sha'->>'main'
         FROM workspace_settings WHERE workspace_id = 'ap-ws'",
    )
    .fetch_one(db)
    .await?)
}

#[sqlx::test(fixtures("git_sync_autopull_recovery"))]
async fn own_push_advances_only_from_the_synced_head(db: Pool<Postgres>) -> anyhow::Result<()> {
    // Job args name the repository without the `$res:` prefix the settings store.
    advance_synced_sha_past_own_push(&db, "ap-ws", "u/admin/repo", "main", "aaa", "w1").await?;
    assert_eq!(synced_main(&db).await?, "w1");

    // Built on a commit the workspace has not pulled: the pull must still happen.
    advance_synced_sha_past_own_push(&db, "ap-ws", "u/admin/repo", "main", "foreign", "w2").await?;
    assert_eq!(synced_main(&db).await?, "w1");
    Ok(())
}
