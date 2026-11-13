//! Who checks a pull request head: the webhook receiver and the auto-pull poller
//! both claim it, and each head is checked once whichever sees it first.
#![cfg(all(feature = "enterprise", feature = "private"))]

use sqlx::{Pool, Postgres};
use windmill_api::git_sync_ee::{claim_pr_check_head, mark_pr_check_started};

const REPO: &str = "$res:u/admin/repo";

#[sqlx::test]
async fn a_head_is_claimed_once(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO workspace (id, name, owner) VALUES ('ws', 'ws', 'test-user')")
        .execute(&db)
        .await?;
    let claim = |sha: &'static str, listed_at| {
        let db = db.clone();
        async move { claim_pr_check_head(&db, "ws", REPO, 7, sha, listed_at, false).await }
    };

    let listed_at = Some(chrono::Utc::now() - chrono::Duration::minutes(1));
    assert!(claim("a", None).await?, "a new head is claimed");
    mark_pr_check_started(&db, "ws", REPO, 7, "a").await?;
    assert!(
        !claim("a", None).await?,
        "the same head is not checked twice"
    );

    assert!(
        !claim("b", listed_at).await?,
        "a listing older than the claim cannot replace it with the head it saw"
    );
    assert!(claim("b", None).await?, "a new head is claimed");

    // Its check never started: a crash between claim and start, or an error.
    assert!(!claim("b", None).await?, "not retried right away");
    sqlx::query("UPDATE git_sync_pr_check_head SET checked_at = now() - interval '11 minutes'")
        .execute(&db)
        .await?;
    assert!(claim("b", None).await?, "retried once the delay passed");
    mark_pr_check_started(&db, "ws", REPO, 7, "b").await?;
    sqlx::query("UPDATE git_sync_pr_check_head SET checked_at = now() - interval '11 minutes'")
        .execute(&db)
        .await?;
    assert!(!claim("b", None).await?, "a started check is never retried");
    assert!(
        claim_pr_check_head(&db, "ws", REPO, 7, "b", None, true).await?,
        "a reopen or retarget re-runs it"
    );
    Ok(())
}
