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
    create_repo_webhook, git_app_installations_for, git_credential_for_url, managed_pr_base_branch,
    repo_provider, repo_supports_managed_git_features, set_git_credential, GitProvider,
};
use windmill_common::workspaces::GitCredentialProvider;

const REPO: &str = "$res:u/admin/repo";
const URL: &str = "https://gitlab.com/grp/proj.git";

/// A repository is managed when a credential is held for the repository its
/// URL names now and the last check found it healthy. The recorded status is
/// keyed by resource path, so alone it would outlive a repoint; the held
/// credential alone says nothing about whether the host still accepts it.
#[sqlx::test(fixtures("git_sync_fork_credential"))]
async fn credential_status_is_a_workspaces_own(db: Pool<Postgres>) -> anyhow::Result<()> {
    assert!(
        !repo_supports_managed_git_features(&db, "parent-ws", REPO).await,
        "a healthy status with nothing held behind it does not qualify"
    );
    set_git_credential(
        &db,
        "parent-ws",
        URL,
        "glpat-secret",
        GitCredentialProvider::Gitlab,
    )
    .await?;
    assert!(
        repo_supports_managed_git_features(&db, "parent-ws", REPO).await,
        "the workspace holding both the credential and the recorded status qualifies"
    );
    assert!(
        !repo_supports_managed_git_features(&db, "fork-ws", REPO).await,
        "a fork borrowing the credential with no record of its own does not: the \
         status describes one repository, and this fork's resource could name another"
    );
    assert!(
        !repo_supports_managed_git_features(&db, "errored-fork-ws", REPO).await,
        "a workspace whose own credential failed stays disqualified"
    );
    Ok(())
}

/// The host a repository talks to is declared when its credential is stored, and
/// travels with the credential down the fork chain.
///
/// Read from the recorded status instead, a fork answered with the default
/// provider until its own check ran, which is long enough to register a webhook
/// against the wrong receiver.
#[sqlx::test(fixtures("git_sync_fork_credential"))]
async fn the_provider_comes_from_the_credential_and_reaches_forks(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    assert_eq!(
        repo_provider(&db, "parent-ws", REPO).await,
        GitProvider::GitHub,
        "with nothing stored there is no declaration to read, so the default stands"
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
        GitProvider::GitHub,
        "a token written into the URL makes the repository a plain remote: the \
         parent's credential is not consulted and no host is declared"
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

/// A GitLab the server cannot reach is the error reported, not the GitHub App
/// lookup that runs after it: for a self-managed GitLab behind a firewall or an
/// untrusted certificate, "no GitHub App installation" names neither the host
/// nor the cause.
#[sqlx::test(fixtures("git_sync_fork_credential"))]
async fn an_unreachable_gitlab_host_is_the_reported_error(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let err = create_repo_webhook(
        &db,
        "parent-ws",
        "http://glpat-secret@127.0.0.1:1/grp/proj.git",
        "https://windmill.example/api/w/parent-ws/git_sync/webhook/gitlab",
        "hook-secret",
    )
    .await
    .expect_err("nothing listens on port 1");
    assert!(
        err.to_string().contains("Could not reach the git host"),
        "unexpected error: {err}"
    );
    assert!(
        !err.to_string().contains("glpat-secret"),
        "the URL credential leaked into the error: {err}"
    );
    Ok(())
}

/// GitHub App installations are normally copied into a fork, but a workspace
/// attached as a dev workspace, or forked before its parent connected the App,
/// holds none, and neither does anything forked from it. The lookup reaches the
/// nearest workspace up the chain that holds some, and the background App path
/// (PR base resolution here) authenticates with that installation's token.
#[sqlx::test(fixtures("git_sync_fork_credential"))]
async fn app_installations_come_from_the_nearest_ancestor_holding_some(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    use axum::{routing::get, Router};
    use std::sync::{Arc, Mutex};

    // A stand-in GitHub API: one repository, and a record of who asked for it.
    let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let app = Router::new().route(
        "/api/v3/repos/acme/repo",
        get({
            let seen = seen.clone();
            move |headers: axum::http::HeaderMap| {
                let seen = seen.clone();
                async move {
                    let auth = headers
                        .get("authorization")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .to_string();
                    seen.lock().unwrap().push(auth);
                    axum::Json(serde_json::json!({ "default_branch": "trunk" }))
                }
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let stub = format!("http://127.0.0.1:{port}");

    // The root holds the installation, with a cached token so nothing is minted.
    sqlx::query(
        "UPDATE workspace_settings SET git_app_installations = $1::jsonb WHERE workspace_id = 'parent-ws'",
    )
    .bind(serde_json::json!([{
        "installation_id": 42, "account_id": "acme", "jwt_token": "x",
        "github_base_url": stub,
        "installation_token": "root-token", "installation_token_expiration": 4102444800i64
    }]))
    .execute(&db)
    .await?;
    // The fork's copy of the resource names the App-backed repository.
    sqlx::query("UPDATE resource SET value = $1::jsonb WHERE workspace_id = 'deep-fork-ws' AND path = 'u/admin/repo'")
        .bind(serde_json::json!({ "url": format!("{stub}/acme/repo.git"), "is_github_app": true }))
        .execute(&db)
        .await?;

    assert_eq!(
        git_app_installations_for(&db, "deep-fork-ws").await?,
        ("parent-ws".to_string(), vec![(42, Some(stub.clone()))]),
        "two levels down, the root's installations are the ones to use"
    );
    assert_eq!(
        git_app_installations_for(&db, "orphan-ws").await?,
        ("orphan-ws".to_string(), vec![]),
        "a workspace with nothing above it resolves nothing"
    );
    assert_eq!(
        managed_pr_base_branch(&db, "deep-fork-ws", REPO)
            .await?
            .as_deref(),
        Some("trunk"),
        "the background App path reaches the repository through the root's installation"
    );
    let seen = seen.lock().unwrap().clone();
    assert!(
        !seen.is_empty() && seen.iter().all(|auth| auth == "Bearer root-token"),
        "every call authenticated with the root's cached token: {seen:?}"
    );

    // A closer holder takes precedence over the root.
    sqlx::query(
        "UPDATE workspace_settings SET git_app_installations = $1::jsonb WHERE workspace_id = 'fork-ws'",
    )
    .bind(serde_json::json!([{
        "installation_id": 7, "account_id": "acme", "jwt_token": "x",
        "github_base_url": stub,
        "installation_token": "mid-token", "installation_token_expiration": 4102444800i64
    }]))
    .execute(&db)
    .await?;
    assert_eq!(
        git_app_installations_for(&db, "deep-fork-ws").await?.0,
        "fork-ws",
        "the nearest holder wins over the root"
    );
    Ok(())
}
