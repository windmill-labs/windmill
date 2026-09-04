//! A fork qualifies for the managed git features through its parent's credential.
//!
//! Fork creation copies the parent's git-sync repositories but drops the recorded
//! credential, which is server-owned per-workspace state. Without the parent
//! fallback a fresh fork stops qualifying, and every deploy until the next
//! credential pass pushes its branch and opens no PR — silently, because nothing
//! about a skipped PR surfaces anywhere.
#![cfg(all(feature = "enterprise", feature = "private"))]

use sqlx::{Pool, Postgres};
use windmill_common::git_sync_ee::{
    git_credential_for_url, repo_supports_managed_git_features, set_git_credential,
};

const REPO: &str = "$res:u/admin/repo";
const URL: &str = "https://gitlab.com/grp/proj.git";

#[sqlx::test(fixtures("git_sync_fork_credential"))]
async fn fork_qualifies_through_its_parents_credential(db: Pool<Postgres>) -> anyhow::Result<()> {
    assert!(
        repo_supports_managed_git_features(&db, "parent-ws", REPO).await,
        "the workspace holding the credential qualifies"
    );
    assert!(
        repo_supports_managed_git_features(&db, "fork-ws", REPO).await,
        "a fork with no credential of its own qualifies through its parent"
    );
    assert!(
        !repo_supports_managed_git_features(&db, "errored-fork-ws", REPO).await,
        "a fork whose own credential failed stays disqualified, rather than \
         borrowing the parent's healthy one"
    );
    assert!(
        !repo_supports_managed_git_features(&db, "orphan-ws", REPO).await,
        "a workspace with no credential and no parent does not qualify"
    );
    Ok(())
}

/// The stored credential is shared with forks and bound to one repository.
///
/// Both properties are the point of keeping it in `workspace_settings`: sharing
/// is what stops a rotation from stranding every fork on a revoked token, and
/// the binding is what stops a rewritten resource URL from carrying the token to
/// a host of the writer's choosing.
#[sqlx::test(fixtures("git_sync_fork_credential"))]
async fn a_fork_reads_its_parents_credential_for_the_bound_repository_only(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    set_git_credential(&db, "parent-ws", REPO, URL, "glpat-secret").await?;

    assert_eq!(
        git_credential_for_url(&db, "parent-ws", REPO, URL)
            .await?
            .as_deref(),
        Some("glpat-secret"),
        "the workspace that stored it reads it back"
    );
    assert_eq!(
        git_credential_for_url(&db, "fork-ws", REPO, URL)
            .await?
            .as_deref(),
        Some("glpat-secret"),
        "a fork stores none of its own and resolves the parent's"
    );
    assert_eq!(
        git_credential_for_url(&db, "fork-ws", REPO, "https://evil.example/grp/proj.git").await?,
        None,
        "a resource repointed at another repository resolves to no credential"
    );
    assert_eq!(
        git_credential_for_url(&db, "orphan-ws", REPO, URL).await?,
        None,
        "a workspace with no credential and no parent resolves nothing"
    );
    Ok(())
}
