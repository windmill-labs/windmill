//! Another member's access lists paths the caller may not be able to read, so only a workspace
//! admin may ask for it.
//!
//! The auth cache is process-wide and keyed by workspace and token, so a test that gives
//! test-user-2 other groups or folders than another test does signs in with a token of its own.

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

#[sqlx::test(fixtures("base"))]
async fn test_member_cannot_read_another_members_access(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let url = format!(
        "http://localhost:{}/api/w/test-workspace/access/get",
        server.addr.port()
    );
    let client = reqwest::Client::new();

    sqlx::raw_sql(
        "INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES ('test-workspace', 'ops', '', '{}');
         INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES ('test-workspace', 'test-user-2', 'ops');
         INSERT INTO instance_group (name) VALUES ('sre');
         INSERT INTO email_to_igroup (email, igroup) VALUES ('test2@windmill.dev', 'sre')",
    )
    .execute(&db)
    .await?;

    // A member may read their own access and their groups', instance groups included, nobody
    // else's.
    for (principal, status) in [
        ("u/test-user", 403),
        ("g/all", 403),
        ("u/test-user-2", 200),
        ("g/ops", 200),
        ("g/sre", 200),
    ] {
        let resp = client
            .get(&url)
            .query(&[("principal", principal)])
            .header("Authorization", "Bearer SECRET_TOKEN_2")
            .send()
            .await?;
        assert_eq!(resp.status(), status, "{principal}: {}", resp.text().await?);
    }

    // A scoped token reaches the route only if `/access` maps to a scope domain.
    let scoped = client
        .post(format!(
            "http://localhost:{}/api/users/tokens/create",
            server.addr.port()
        ))
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .json(&serde_json::json!({ "label": "scoped", "scopes": ["acls:read"], "workspace_id": "test-workspace" }))
        .send()
        .await?
        .text()
        .await?;
    let resp = client
        .get(&url)
        .query(&[("principal", "u/test-user-2")])
        .header("Authorization", format!("Bearer {scoped}"))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    Ok(())
}

/// The navigator writes grants with the kind it is given, and `acls/add` sends a full-code app's
/// change to git sync only under `raw_app`. `blocked` must refuse what `acls/add` would refuse.
#[sqlx::test(fixtures("base"))]
async fn test_browse_items_kind_and_blocked(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    sqlx::raw_sql(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
            VALUES ('test-workspace', 'fin', 'fin', '{u/test-user}', '{\"u/test-user-2\": false}', 'test-user');
         INSERT INTO app (id, workspace_id, path, summary, policy, versions, extra_perms)
            VALUES (1, 'test-workspace', 'f/fin/dash', '', '{}', '{1}', '{}');
         INSERT INTO app_version (id, app_id, value, created_by, raw_app)
            VALUES (1, 1, '{}', 'test-user', true);
         INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin)
            VALUES (encode(sha256('BROWSE_TOKEN'::bytea), 'hex'), 'BROWSE_TOK', 'BROWSE_TOKEN',
                    'test2@windmill.dev', 'browse', false);",
    )
    .execute(&db)
    .await?;
    let server = ApiServer::start(db.clone()).await?;
    let url = format!(
        "http://localhost:{}/api/w/test-workspace/access/browse_items",
        server.addr.port()
    );
    let browse = |token: &str| {
        reqwest::Client::new()
            .get(&url)
            .query(&[("principal", "u/test-user-2"), ("prefix", "f/fin/")])
            .header("Authorization", format!("Bearer {token}"))
            .send()
    };

    let member: serde_json::Value = browse("BROWSE_TOKEN")
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        member["items"],
        serde_json::json!([{
            "kind": "raw_app",
            "path": "f/fin/dash",
            "grant": null,
            "blocked": "Only admins of the workspace or of f/fin can share it"
        }])
    );

    let admin: serde_json::Value = browse("SECRET_TOKEN")
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(admin["items"][0]["blocked"], serde_json::Value::Null);

    Ok(())
}

async fn reads(port: u16, body: serde_json::Value) -> anyhow::Result<(u16, serde_json::Value)> {
    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/test-workspace/access/reads"
        ))
        .header("Authorization", "Bearer READS_TOKEN")
        .json(&body)
        .send()
        .await?;
    let status = resp.status().as_u16();
    let text = resp.text().await?;
    Ok((
        status,
        serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text)),
    ))
}

/// A step runs as its target's own account when it has one, and only what the caller can read
/// is followed.
#[sqlx::test(fixtures("base", "access_reads"))]
async fn test_access_reads_flow(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let root = json!({"type": "root"});
    let bot = json!({"type": "account", "principal": "u/test-user"});

    let (status, flow) = reads(port, json!({"kind": "flow", "path": "u/test-user-2/flow"})).await?;
    assert_eq!(status, 200, "{flow}");
    assert_eq!(
        flow,
        json!({
            "runs_as": {"type": "caller"},
            "references": [
                // A path step's static input is resolved by the step's own job.
                {"kind": "resource", "path": "u/test-user-2/bot_input", "access": "read",
                 "reader": bot, "sources": ["u/test-user-2/flow › Bot"]},
                {"kind": "variable", "path": "u/test-user-2/bot_token", "access": "read",
                 "reader": bot, "sources": ["u/test-user-2/bot"]},
                {"kind": "variable", "path": "u/test-user-2/expr_var", "access": "read",
                 "reader": root, "sources": ["u/test-user-2/flow › Inline"]},
                {"kind": "variable", "path": "u/test-user-2/linked", "access": "read",
                 "reader": bot, "sources": ["u/test-user-2/bot_input"]},
                // The pinned version's read, not the latest version's.
                {"kind": "variable", "path": "u/test-user-2/old_var", "access": "read",
                 "reader": root, "sources": ["u/test-user-2/versioned"]},
                {"kind": "variable", "path": "u/test-user-2/state", "access": "write",
                 "reader": root, "sources": ["u/test-user-2/flow › Inline"]},
                {"kind": "script", "path": "u/test-user-2/bot", "access": "run",
                 "reader": root, "sources": ["u/test-user-2/flow › Inline"]},
            ],
            "others": [],
            "errors": ["u/test-user/private: not found or not readable"],
        })
    );

    // Unsaved content stands in for the saved item, including where it names its own path.
    let (status, draft) = reads(
        port,
        json!({
            "kind": "script",
            "path": "u/test-user-2/versioned",
            "source": {"language": "bun", "content": "import * as wmill from 'windmill-client'
await wmill.getVariable('u/test-user-2/draft_var')
await wmill.runScriptByPath('u/test-user-2/versioned')"},
        }),
    )
    .await?;
    assert_eq!(status, 200, "{draft}");
    assert_eq!(
        draft["references"],
        json!([
            {"kind": "variable", "path": "u/test-user-2/draft_var", "access": "read",
             "reader": root, "sources": ["u/test-user-2/versioned"]},
            {"kind": "script", "path": "u/test-user-2/versioned", "access": "run",
             "reader": root, "sources": ["u/test-user-2/versioned"]},
        ])
    );

    let (status, private) = reads(
        port,
        json!({"kind": "script", "path": "u/test-user/private"}),
    )
    .await?;
    assert_eq!(status, 404, "{private}");
    Ok(())
}

/// A full-code app's frontend reads as whoever opens it, and its runnables read as the app
/// whatever account the script they call runs as.
#[sqlx::test(fixtures("base", "access_reads"))]
async fn test_access_reads_app(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let root = json!({"type": "root"});
    let viewer = json!({"type": "viewer"});

    let (status, app) = reads(
        server.addr.port(),
        json!({"kind": "app", "path": "u/test-user-2/app"}),
    )
    .await?;
    assert_eq!(status, 200, "{app}");
    assert_eq!(
        app,
        json!({
            "runs_as": {"type": "account", "principal": "u/test-user"},
            "references": [
                {"kind": "variable", "path": "u/test-user-2/bot_token", "access": "read",
                 "reader": root, "sources": ["u/test-user-2/bot"]},
                {"kind": "variable", "path": "u/test-user-2/runnable_var", "access": "read",
                 "reader": root, "sources": ["u/test-user-2/app › a"]},
                {"kind": "variable", "path": "u/test-user-2/svelte_var", "access": "read",
                 "reader": viewer, "sources": ["u/test-user-2/app › /Widget.svelte"]},
                {"kind": "variable", "path": "u/test-user-2/tsx_var", "access": "read",
                 "reader": viewer, "sources": ["u/test-user-2/app › /index.tsx"]},
            ],
            "others": [],
            "errors": [],
        })
    );
    Ok(())
}
