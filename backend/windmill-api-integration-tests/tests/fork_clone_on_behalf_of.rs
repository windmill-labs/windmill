use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

/// Seed an anonymous public app owned by the parent's admin, then fork as `token`. Returns the
/// cloned app's policy and custom path.
async fn fork_with_public_app(
    db: &Pool<Postgres>,
    token: &str,
) -> anyhow::Result<(serde_json::Value, Option<String>)> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let base_url = format!("http://localhost:{}/api", server.addr.port());

    let app_id = sqlx::query_scalar!(
        "INSERT INTO app (workspace_id, path, summary, policy, versions, custom_path)
         VALUES ('test-workspace', 'u/test-user/pub', '', $1, '{}', 'pub-path')
         RETURNING id",
        json!({
            "on_behalf_of": "u/test-user",
            "on_behalf_of_email": "test@windmill.dev",
            "execution_mode": "anonymous",
        })
    )
    .fetch_one(db)
    .await?;
    // The clone re-aggregates `versions` from `app_version`, so an app without one lands in the
    // fork with a NULL array.
    sqlx::query!(
        "WITH v AS (
            INSERT INTO app_version (app_id, value, created_by)
            VALUES ($1, '{}'::json, 'test-user') RETURNING id
         )
         UPDATE app SET versions = ARRAY[v.id] FROM v WHERE app.id = $1",
        app_id
    )
    .execute(db)
    .await?;

    let resp = reqwest::Client::new()
        .post(format!(
            "{base_url}/w/test-workspace/workspaces/create_fork"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&json!({ "id": "wm-fork-app", "name": "Fork", "color": "#0000ff" }))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "creating the fork: {}",
        resp.text().await?
    );

    let cloned =
        sqlx::query!("SELECT policy, custom_path FROM app WHERE workspace_id = 'wm-fork-app'")
            .fetch_one(db)
            .await?;
    Ok((cloned.policy, cloned.custom_path))
}

/// An app policy's `on_behalf_of` is the identity anonymous and publisher executions queue jobs
/// under, and the fork's endpoint outlives any revocation in the parent — so a creator who may
/// not preserve someone else's identity must not receive one by forking. `test-user-2` is a
/// plain member of the parent.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_fork_repoints_app_identity_for_unprivileged_creator(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let (policy, custom_path) = fork_with_public_app(&db, "SECRET_TOKEN_2").await?;

    assert_eq!(policy["on_behalf_of"], json!("u/test-user-2"));
    assert_eq!(policy["on_behalf_of_email"], json!("test2@windmill.dev"));
    // `execution_mode` rides along untouched — see `clone_apps`.
    assert_eq!(policy["execution_mode"], json!("anonymous"));
    assert_eq!(custom_path, None);

    Ok(())
}

/// An admin could have set any of this through the app API, so their fork keeps the policy — which
/// is also what keeps dev workspaces, always admin-created, behaving like their parent. The custom
/// path still goes: it is the instance-wide address of the parent's live public app, and two rows
/// claiming it make it resolve to either one.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_fork_keeps_app_policy_for_admin_creator(db: Pool<Postgres>) -> anyhow::Result<()> {
    let (policy, custom_path) = fork_with_public_app(&db, "SECRET_TOKEN").await?;

    assert_eq!(policy["on_behalf_of"], json!("u/test-user"));
    assert_eq!(policy["on_behalf_of_email"], json!("test@windmill.dev"));
    assert_eq!(policy["execution_mode"], json!("anonymous"));
    assert_eq!(custom_path, None);

    Ok(())
}

/// A cloned app's identity is re-pointed at an unprivileged fork creator, and no deploy can
/// converge that back — the deployer picks the target's own value, their own, or a typed-in one,
/// never the source's. Reporting it would leave an entry the merge UI can never clear, on every
/// app in such a fork. A summary change alongside it keeps this honest.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_compare_ignores_app_identity(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let base_url = format!("http://localhost:{}/api", server.addr.port());
    let client = reqwest::Client::new();

    for path in ["u/test-user/identity_only", "u/test-user/summary_too"] {
        let app_id = sqlx::query_scalar!(
            "INSERT INTO app (workspace_id, path, summary, policy, versions)
             VALUES ('test-workspace', $1, 'original', $2, '{}')
             RETURNING id",
            path,
            json!({ "on_behalf_of": "u/test-user", "on_behalf_of_email": "test@windmill.dev" })
        )
        .fetch_one(&db)
        .await?;
        sqlx::query!(
            "WITH v AS (
                INSERT INTO app_version (app_id, value, created_by)
                VALUES ($1, '{}'::json, 'test-user') RETURNING id
             )
             UPDATE app SET versions = ARRAY[v.id] FROM v WHERE app.id = $1",
            app_id
        )
        .execute(&db)
        .await?;
    }

    let resp = client
        .post(format!(
            "{base_url}/w/test-workspace/workspaces/create_fork"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({ "id": "wm-fork-cmp", "name": "Fork", "color": "#0000ff" }))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "creating the fork: {}",
        resp.text().await?
    );

    // Forked as an admin, so both apps arrive identical. Diverge each on one axis.
    sqlx::query!(
        "UPDATE app SET policy = policy || $1::jsonb
         WHERE workspace_id = 'wm-fork-cmp' AND path = 'u/test-user/identity_only'",
        json!({ "on_behalf_of": "u/someone-else", "on_behalf_of_email": "else@windmill.dev" })
    )
    .execute(&db)
    .await?;
    sqlx::query!(
        "UPDATE app SET summary = 'edited'
         WHERE workspace_id = 'wm-fork-cmp' AND path = 'u/test-user/summary_too'"
    )
    .execute(&db)
    .await?;

    sqlx::query!(
        "INSERT INTO workspace_diff
         (source_workspace_id, fork_workspace_id, path, kind, ahead, behind, has_changes)
         VALUES ('test-workspace', 'wm-fork-cmp', 'u/test-user/identity_only', 'app', 0, 1, NULL),
                ('test-workspace', 'wm-fork-cmp', 'u/test-user/summary_too', 'app', 0, 1, NULL)"
    )
    .execute(&db)
    .await?;
    // The bootstrap migration flags pre-existing workspaces as untallied, which short-circuits
    // the comparison.
    sqlx::query!("DELETE FROM skip_workspace_diff_tally")
        .execute(&db)
        .await?;

    let comparison: serde_json::Value = client
        .get(format!(
            "{base_url}/w/test-workspace/workspaces/compare/wm-fork-cmp"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .send()
        .await?
        .json()
        .await?;
    let listed: Vec<&str> = comparison["diffs"]
        .as_array()
        .expect("diffs array")
        .iter()
        .filter_map(|d| d["path"].as_str())
        .collect();

    assert!(
        !listed.contains(&"u/test-user/identity_only"),
        "an identity-only difference must not be reported: {listed:?}"
    );
    assert!(
        listed.contains(&"u/test-user/summary_too"),
        "a real change must still be reported: {listed:?}"
    );

    Ok(())
}

/// A principal only means something in the workspace whose `usr`/`group_` rows define it, and a
/// fork copies the creator and the groups but not the rest of the membership. Carrying one over
/// blindly would leave a runnable naming somebody who cannot authenticate there; dropping them
/// all would silently hand every configured runnable in the fork to whoever runs it.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_fork_keeps_only_resolvable_on_behalf_of(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base_url = format!("http://localhost:{port}/api");

    // A superadmin with no `usr` row anywhere: they authenticate from `password` alone, so
    // their principal resolves in the fork as much as it did in the parent.
    sqlx::query!(
        "INSERT INTO password(email, password_hash, login_type, super_admin, verified, name, username)
         VALUES ('sa@windmill.dev', 'not-a-real-hash', 'password', true, true, 'Ext', 'ext-sa')"
    )
    .execute(&db)
    .await?;

    sqlx::query!(
        "INSERT INTO script (workspace_id, path, hash, content, summary, description, language, created_by, created_at, on_behalf_of, on_behalf_of_email)
         VALUES
         ('test-workspace', 'u/test-user/obo_member', 91001, 'def main(): pass', '', '', 'python3', 'test-user', NOW(), 'u/test-user', 'test@windmill.dev'),
         ('test-workspace', 'u/test-user/obo_stranger', 91002, 'def main(): pass', '', '', 'python3', 'test-user', NOW(), 'u/test-user-2', 'test2@windmill.dev'),
         ('test-workspace', 'u/test-user/obo_group', 91003, 'def main(): pass', '', '', 'python3', 'test-user', NOW(), 'g/all', 'group-all@windmill.dev'),
         ('test-workspace', 'u/test-user/obo_superadmin', 91004, 'def main(): pass', '', '', 'python3', 'test-user', NOW(), 'u/ext-sa', 'sa@windmill.dev'),
         ('test-workspace', 'u/test-user/obo_address_only', 91005, 'def main(): pass', '', '', 'python3', 'test-user', NOW(), NULL, 'test2@windmill.dev')"
    )
    .execute(&db)
    .await?;

    sqlx::query!(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at, on_behalf_of, on_behalf_of_email)
         VALUES ('test-workspace', 'u/test-user/obo_flow', '', '', $1, 'test-user', NOW(), 'u/test-user-2', 'test2@windmill.dev')",
        json!({"modules": []})
    )
    .execute(&db)
    .await?;

    let resp = reqwest::Client::new()
        .post(format!(
            "{base_url}/w/test-workspace/workspaces/create_fork"
        ))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({ "id": "wm-fork-obo", "name": "Fork", "color": "#0000ff" }))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "creating the fork: {}",
        resp.text().await?
    );

    let cloned = sqlx::query!(
        "SELECT path, on_behalf_of, on_behalf_of_email FROM script WHERE workspace_id = 'wm-fork-obo' ORDER BY path"
    )
    .fetch_all(&db)
    .await?;
    let identity = |path: &str| {
        cloned
            .iter()
            .find(|r| r.path == path)
            .unwrap_or_else(|| panic!("{path} was cloned"))
            .on_behalf_of
            .clone()
    };

    // The creator is the one member the fork always gets, so their principal still resolves.
    assert_eq!(
        identity("u/test-user/obo_member").as_deref(),
        Some("u/test-user")
    );
    // Groups are cloned wholesale, so a group principal resolves too.
    assert_eq!(identity("u/test-user/obo_group").as_deref(), Some("g/all"));
    assert_eq!(
        identity("u/test-user/obo_superadmin").as_deref(),
        Some("u/ext-sa")
    );
    // `test-user-2` is not carried into the fork, so nothing there can run as them — and the
    // address has to go with the principal, or a worker that reads only the address still would.
    assert_eq!(identity("u/test-user/obo_stranger"), None);
    // A row a server predating this release wrote carries the address alone; the clone must not
    // mistake it for one it orphaned, because that address is all a later re-derivation has.
    assert_eq!(
        cloned
            .iter()
            .find(|r| r.path == "u/test-user/obo_address_only")
            .and_then(|r| r.on_behalf_of_email.as_deref()),
        Some("test2@windmill.dev"),
    );

    let orphaned = cloned
        .iter()
        .filter(|r| {
            r.on_behalf_of.is_none()
                && r.on_behalf_of_email.is_some()
                && r.path != "u/test-user/obo_address_only"
        })
        .count();
    assert_eq!(orphaned, 0, "a dropped principal leaves no address behind");
    assert_eq!(
        sqlx::query_scalar!("SELECT on_behalf_of FROM flow WHERE workspace_id = 'wm-fork-obo'")
            .fetch_one(&db)
            .await?,
        None
    );

    Ok(())
}
