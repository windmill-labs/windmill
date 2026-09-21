//! An automatic pull runs as the admin stamped on the repository's settings, never as
//! someone picked from the workspace, and stops once that admin is revoked. A fork's
//! pull runs as the parent's pull identity, added to the fork first.
#![cfg(all(feature = "enterprise", feature = "private"))]

use sqlx::{Pool, Postgres};
use windmill_common::workspaces::GitRepositorySettings;
use windmill_git_sync::{reconcile_and_enqueue_pull, reconcile_fork_branch_pull};

const PARENT: &str = "ap-parent";
const FORK: &str = "wm-fork-feat";
const REPO: &str = "$res:u/alice/repo";

fn repo_enabled_by(email: &str) -> GitRepositorySettings {
    serde_json::from_value(serde_json::json!({
        "git_repo_resource_path": REPO,
        "use_individual_branch": false,
        "group_by_folder": false,
        "auto_pull": { "enabled": true, "enabled_by": email }
    }))
    .expect("repository settings")
}

/// `(created_by, permissioned_as, permissioned_as_email)` of every pull job in `w_id`.
async fn pull_identities(
    db: &Pool<Postgres>,
    w_id: &str,
) -> anyhow::Result<Vec<(String, String, Option<String>)>> {
    Ok(sqlx::query_as(
        "SELECT created_by, permissioned_as, permissioned_as_email FROM v2_job \
         WHERE workspace_id = $1 AND kind = 'deploymentcallback'",
    )
    .bind(w_id)
    .fetch_all(db)
    .await?)
}

fn identity(username: &str) -> (String, String, Option<String>) {
    (
        username.to_string(),
        format!("u/{username}"),
        Some(format!("{username}@windmill.dev")),
    )
}

async fn recorded_pull_error(db: &Pool<Postgres>, w_id: &str) -> anyhow::Result<String> {
    let git_sync: serde_json::Value =
        sqlx::query_scalar("SELECT git_sync FROM workspace_settings WHERE workspace_id = $1")
            .bind(w_id)
            .fetch_one(db)
            .await?;
    Ok(
        git_sync["repositories"][0]["auto_pull"]["last_pull_status"]["error"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
    )
}

#[sqlx::test(fixtures("git_sync_autopull_identity"))]
async fn pull_runs_as_the_admin_who_enabled_it(db: Pool<Postgres>) -> anyhow::Result<()> {
    let job = reconcile_and_enqueue_pull(
        &db,
        PARENT,
        &repo_enabled_by("alice@windmill.dev"),
        "main",
        "abc123",
        None,
    )
    .await?;

    assert!(job.is_some());
    assert_eq!(pull_identities(&db, PARENT).await?, vec![identity("alice")]);
    Ok(())
}

/// bob was demoted in the workspace; dora is still a workspace admin but deactivated on
/// the instance. Neither may run the pull, and the failure lands on the status rather
/// than as an error, which a webhook delivery would turn into a failed response.
#[sqlx::test(fixtures("git_sync_autopull_identity"))]
async fn pull_fails_on_the_status_once_the_enabling_admin_is_revoked(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    for email in ["bob@windmill.dev", "dora@windmill.dev"] {
        let job = reconcile_and_enqueue_pull(
            &db,
            PARENT,
            &repo_enabled_by(email),
            "main",
            "abc123",
            None,
        )
        .await?;
        assert!(job.is_none(), "{email} must not run the pull");
        let error = recorded_pull_error(&db, PARENT).await?;
        assert!(error.contains(email), "{error}");
    }
    assert!(pull_identities(&db, PARENT).await?.is_empty());
    Ok(())
}

#[sqlx::test(fixtures("git_sync_autopull_identity"))]
async fn fork_pull_runs_as_the_parent_admin_added_to_the_fork(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let job = reconcile_fork_branch_pull(&db, PARENT, REPO, "wm-fork/main/feat", "main", "abc123")
        .await?;
    assert!(
        job.is_some(),
        "the fork branch must route to the fork and enqueue"
    );

    let (is_admin, in_all): (bool, bool) = sqlx::query_as(
        "SELECT u.is_admin, EXISTS (SELECT 1 FROM usr_to_group g \
             WHERE g.workspace_id = u.workspace_id AND g.usr = u.username AND g.group_ = 'all') \
         FROM usr u WHERE u.workspace_id = $1 AND u.email = 'alice@windmill.dev'",
    )
    .bind(FORK)
    .fetch_one(&db)
    .await?;
    assert!(
        is_admin && in_all,
        "alice must be an admin member of the fork"
    );
    assert_eq!(pull_identities(&db, FORK).await?, vec![identity("alice")]);

    let grants: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_partitioned WHERE workspace_id = $1 \
         AND operation = 'users.git_sync_fork_add' AND resource = 'alice@windmill.dev'",
    )
    .bind(FORK)
    .fetch_one(&db)
    .await?;
    assert_eq!(grants, 1, "adding alice to the fork must be audited");
    Ok(())
}

/// A superadmin who is not a member runs the pull under their instance username, and
/// `u/<username>` resolves through the workspace's members first. sam's instance username
/// is carol's, so sam's stamp must not run the pull as carol; sue's is unclaimed.
#[sqlx::test(fixtures("git_sync_autopull_identity"))]
async fn a_non_member_superadmin_runs_the_pull_only_under_an_unclaimed_username(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let job = reconcile_and_enqueue_pull(
        &db,
        PARENT,
        &repo_enabled_by("sam@windmill.dev"),
        "main",
        "abc123",
        None,
    )
    .await?;
    assert!(job.is_none(), "sam's username belongs to carol");
    assert!(pull_identities(&db, PARENT).await?.is_empty());

    let job = reconcile_and_enqueue_pull(
        &db,
        PARENT,
        &repo_enabled_by("sue@windmill.dev"),
        "main",
        "abc123",
        None,
    )
    .await?;
    assert!(job.is_some());
    assert_eq!(pull_identities(&db, PARENT).await?, vec![identity("sue")]);
    Ok(())
}

/// With no stamp on the parent, the fork pull still runs as the parent's first active
/// admin: the fork holds only its non-admin creator, so no identity resolved in the fork
/// could run it.
#[sqlx::test(fixtures("git_sync_autopull_identity"))]
async fn unstamped_fork_pull_runs_as_the_parents_first_admin(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workspace_settings SET git_sync = git_sync #- '{repositories,0,auto_pull,enabled_by}' \
         WHERE workspace_id = $1",
    )
    .bind(PARENT)
    .execute(&db)
    .await?;

    let job = reconcile_fork_branch_pull(&db, PARENT, REPO, "wm-fork/main/feat", "main", "abc123")
        .await?;
    assert!(
        job.is_some(),
        "an unstamped parent must still sync its forks"
    );
    assert_eq!(pull_identities(&db, FORK).await?, vec![identity("aaron")]);
    Ok(())
}
