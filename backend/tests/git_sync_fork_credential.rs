//! A fork reaches the git credential held above it in its fork chain.
//!
//! Fork creation copies the parent's git-sync repositories but not the credential,
//! which is stored per workspace so that rotation has one owner. Chains nest (a
//! fork of a dev workspace, a fork of that), so the depth-2 cases here are what
//! keep the lookup from regressing to the parent.
//!
//! The recorded *status* is not shared the same way: it describes one repository,
//! and a fork can repoint its copy of the resource, so each workspace answers from
//! its own record and gets one by fork creation copying it down.
#![cfg(all(feature = "enterprise", feature = "private"))]

use sqlx::{Pool, Postgres};
use windmill_common::git_sync_ee::{
    git_credential_for_url, repo_provider, repo_supports_managed_git_features, set_git_credential,
    GitProvider,
};
use windmill_common::workspaces::GitCredentialProvider;

const REPO: &str = "$res:u/admin/repo";
const URL: &str = "https://gitlab.com/grp/proj.git";

#[sqlx::test(fixtures("git_sync_fork_credential"))]
async fn credential_status_is_a_workspaces_own(db: Pool<Postgres>) -> anyhow::Result<()> {
    assert!(
        repo_supports_managed_git_features(&db, "parent-ws", REPO).await,
        "the workspace holding the recorded status qualifies"
    );
    assert!(
        !repo_supports_managed_git_features(&db, "fork-ws", REPO).await,
        "a fork with no record of its own does not borrow the parent's: the status \
         describes one repository, and this fork's resource could name another"
    );
    assert!(
        !repo_supports_managed_git_features(&db, "errored-fork-ws", REPO).await,
        "a workspace whose own credential failed stays disqualified"
    );
    Ok(())
}

/// The host a repository talks to is declared when its credential is stored, and
/// travels with the credential down the fork chain. A repository whose token
/// rides in its URL has no stored credential, so its host is known only from the
/// check that introspected the token.
///
/// Read from the recorded status alone, a fork answered with the default
/// provider until its own check ran, which is long enough to register a webhook
/// against the wrong receiver. Read from the credential alone, a URL-token
/// repository answered with the default forever.
#[sqlx::test(fixtures("git_sync_fork_credential"))]
async fn the_provider_comes_from_the_credential_and_reaches_forks(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    assert_eq!(
        repo_provider(&db, "parent-ws", REPO).await,
        GitProvider::GitLab,
        "with nothing stored, the host the check recorded is the answer"
    );
    assert_eq!(
        repo_provider(&db, "fork-ws", REPO).await,
        GitProvider::GitHub,
        "a fork with neither a credential to resolve nor a check of its own \
         answers the default"
    );

    set_git_credential(
        &db,
        "parent-ws",
        URL,
        "glpat-secret",
        GitCredentialProvider::Gitlab,
    )
    .await?;

    assert_eq!(
        repo_provider(&db, "parent-ws", REPO).await,
        GitProvider::GitLab,
        "the workspace that stored it reads its own declaration"
    );
    assert_eq!(
        repo_provider(&db, "fork-ws", REPO).await,
        GitProvider::GitLab,
        "and a fork resolving that credential reads it too, without a check of its own"
    );
    assert_eq!(
        repo_provider(&db, "deep-fork-ws", REPO).await,
        GitProvider::GitLab,
        "two levels down as well"
    );
    assert_eq!(
        repo_provider(&db, "orphan-ws", REPO).await,
        GitProvider::GitHub,
        "a workspace outside the chain resolves no credential and no declaration"
    );
    assert_eq!(
        repo_provider(&db, "errored-fork-ws", REPO).await,
        GitProvider::GitLab,
        "a token carried in the URL is held by nobody, so the parent's credential \
         is not consulted and the recorded check alone names the host"
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
    set_git_credential(
        &db,
        "parent-ws",
        URL,
        "glpat-secret",
        GitCredentialProvider::Gitlab,
    )
    .await?;

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

    set_git_credential(
        &db,
        "parent-ws",
        URL,
        "glpat-first",
        GitCredentialProvider::Gitlab,
    )
    .await?;
    set_git_credential(
        &db,
        "parent-ws",
        OTHER_URL,
        "glpat-second",
        GitCredentialProvider::Gitlab,
    )
    .await?;

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

    set_git_credential(
        &db,
        "parent-ws",
        URL,
        "glpat-replacement",
        GitCredentialProvider::Gitlab,
    )
    .await?;
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
    set_git_credential(
        &db,
        "parent-ws",
        URL,
        "glpat-secret",
        GitCredentialProvider::Gitlab,
    )
    .await?;
    assert_eq!(
        git_credential_for_url(&db, "parent-ws", "http://gitlab.com/grp/proj.git").await?,
        None
    );
    Ok(())
}
