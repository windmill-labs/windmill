//! A fork reaches the git credential and status held above it in its fork chain.
//!
//! Fork creation copies the parent's git-sync repositories but neither the
//! recorded credential status nor the credential itself, both of which are
//! server-owned per-workspace state. Without the fallback a fresh fork stops
//! qualifying, and every deploy until the next credential pass pushes its branch
//! and opens no PR — silently, because nothing about a skipped PR surfaces
//! anywhere. Chains nest (a fork of a dev workspace, a fork of that), so the
//! depth-2 cases here are what keep the lookup from regressing to the parent.
#![cfg(all(feature = "enterprise", feature = "private"))]

use sqlx::{Pool, Postgres};
use windmill_common::git_sync_ee::{
    git_credential_for_url, repo_supports_managed_git_features, set_git_credential,
};

const REPO: &str = "$res:u/admin/repo";
const URL: &str = "https://gitlab.com/grp/proj.git";

#[sqlx::test(fixtures("git_sync_fork_credential"))]
async fn a_fork_qualifies_through_the_nearest_ancestors_credential(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    assert!(
        repo_supports_managed_git_features(&db, "parent-ws", REPO).await,
        "the workspace holding the credential qualifies"
    );
    assert!(
        repo_supports_managed_git_features(&db, "fork-ws", REPO).await,
        "a fork with no credential of its own qualifies through its parent"
    );
    assert!(
        repo_supports_managed_git_features(&db, "deep-fork-ws", REPO).await,
        "a fork of a fork qualifies through the root, two levels up"
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

/// The stored credential is shared with forks and keyed by one repository.
///
/// Both properties are the point of keeping it in `workspace_settings` under the
/// repository's identity: sharing is what stops a rotation from stranding every
/// fork on a revoked token, and the key is what stops a rewritten resource URL
/// from carrying the token to a host of the writer's choosing.
#[sqlx::test(fixtures("git_sync_fork_credential"))]
async fn a_fork_reads_an_ancestors_credential_for_the_bound_repository_only(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    set_git_credential(&db, "parent-ws", URL, "glpat-secret").await?;

    assert_eq!(
        git_credential_for_url(&db, "parent-ws", URL)
            .await?
            .as_deref(),
        Some("glpat-secret"),
        "the workspace that stored it reads it back"
    );
    assert_eq!(
        git_credential_for_url(&db, "fork-ws", URL)
            .await?
            .as_deref(),
        Some("glpat-secret"),
        "a fork stores none of its own and resolves the parent's"
    );
    assert_eq!(
        git_credential_for_url(&db, "deep-fork-ws", URL)
            .await?
            .as_deref(),
        Some("glpat-secret"),
        "a fork of a fork resolves the root's, two levels up"
    );
    assert_eq!(
        git_credential_for_url(&db, "fork-ws", "https://evil.example/grp/proj.git").await?,
        None,
        "a resource repointed at another repository asks for that one's \
         credential and finds none"
    );
    assert_eq!(
        git_credential_for_url(&db, "orphan-ws", URL).await?,
        None,
        "a workspace with no credential and no parent resolves nothing"
    );
    Ok(())
}

/// One repository's credential is untouched by another's.
///
/// The key is the repository, so picking a second repository stores beside the
/// first rather than over it. Keyed by the resource instead, a workspace editing
/// one repository's resource to point somewhere else would replace the token the
/// original repository was still syncing with.
#[sqlx::test(fixtures("git_sync_fork_credential"))]
async fn each_repository_keeps_its_own_credential(db: Pool<Postgres>) -> anyhow::Result<()> {
    const OTHER_URL: &str = "https://gitlab.com/grp/other.git";

    set_git_credential(&db, "parent-ws", URL, "glpat-first").await?;
    set_git_credential(&db, "parent-ws", OTHER_URL, "glpat-second").await?;

    assert_eq!(
        git_credential_for_url(&db, "parent-ws", URL)
            .await?
            .as_deref(),
        Some("glpat-first"),
        "storing a second repository's token leaves the first's in place"
    );
    assert_eq!(
        git_credential_for_url(&db, "parent-ws", OTHER_URL)
            .await?
            .as_deref(),
        Some("glpat-second")
    );

    set_git_credential(&db, "parent-ws", URL, "glpat-replacement").await?;
    assert_eq!(
        git_credential_for_url(&db, "parent-ws", URL)
            .await?
            .as_deref(),
        Some("glpat-replacement"),
        "storing the same repository again replaces rather than duplicates"
    );
    assert_eq!(
        git_credential_for_url(&db, "parent-ws", OTHER_URL)
            .await?
            .as_deref(),
        Some("glpat-second"),
        "and still leaves the other repository alone"
    );
    Ok(())
}

/// A credential issued for `https` is not served for the `http` spelling.
///
/// The resource holding the URL is writable by anyone with write on its path, so
/// without the scheme in the key that edit would send the token over cleartext.
#[sqlx::test(fixtures("git_sync_fork_credential"))]
async fn a_credential_is_not_served_over_a_downgraded_transport(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    set_git_credential(&db, "parent-ws", URL, "glpat-secret").await?;
    assert_eq!(
        git_credential_for_url(&db, "parent-ws", "http://gitlab.com/grp/proj.git").await?,
        None
    );
    Ok(())
}
