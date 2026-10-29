//! Workspace role overrides travel in the settings document that workspace export, `wmill sync`
//! and git sync carry, and come back through its apply.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

/// The contents of `name` in an uncompressed tar archive.
fn tar_entry(archive: &[u8], name: &str) -> Option<String> {
    let mut at = 0;
    while at + 512 <= archive.len() {
        let header = &archive[at..at + 512];
        if header.iter().all(|b| *b == 0) {
            return None;
        }
        let entry_name = String::from_utf8_lossy(&header[..100])
            .trim_end_matches('\0')
            .to_string();
        let size = usize::from_str_radix(
            String::from_utf8_lossy(&header[124..136])
                .trim_matches(|c: char| c == '\0' || c == ' '),
            8,
        )
        .unwrap();
        let body = at + 512;
        if entry_name == name {
            return Some(String::from_utf8_lossy(&archive[body..body + size]).to_string());
        }
        at = body + size.div_ceil(512) * 512;
    }
    None
}

async fn create_workspace(db: &Pool<Postgres>, id: &str) {
    for q in [
        "INSERT INTO workspace (id, name, owner) VALUES ($1, $1, 'test-user')",
        "INSERT INTO workspace_settings (workspace_id) VALUES ($1)",
        "INSERT INTO workspace_key (workspace_id, kind, key) VALUES ($1, 'cloud', 'target-key')",
        "INSERT INTO usr (workspace_id, email, username, is_admin, role)
         VALUES ($1, 'test@windmill.dev', 'test-user', true, 'Admin')",
        "INSERT INTO group_ (workspace_id, name, summary, extra_perms)
         VALUES ($1, 'all', 'All users', '{}')",
    ] {
        sqlx::query(q).bind(id).execute(db).await.unwrap();
    }
}

async fn stored_overrides(db: &Pool<Postgres>, w_id: &str) -> Option<Value> {
    sqlx::query_scalar("SELECT role_overrides FROM workspace_settings WHERE workspace_id = $1")
        .bind(w_id)
        .fetch_one(db)
        .await
        .unwrap()
}

async fn apply(port: u16, w_id: &str, document: Value, dry_run: bool) -> reqwest::Response {
    authed(client().post(format!(
        "http://localhost:{port}/api/w/{w_id}/workspaces/settings_document"
    )))
    .json(&json!({ "document": document, "replace": true, "dry_run": dry_run }))
    .send()
    .await
    .unwrap()
}

fn overrides_of_default_roles() -> Value {
    json!({
        "developer": { "capabilities": { "fork": false } },
        "operator": { "label": "Runner", "capabilities": { "flows": "compose" } },
    })
}

/// A workspace-only role is EE: a CE workspace has only overrides of the default roles.
fn workspace_only_roles() -> Value {
    if cfg!(feature = "enterprise") {
        json!({ "auditor": { "label": "Auditor", "capabilities": { "code": false } } })
    } else {
        json!({})
    }
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn role_overrides_round_trip_through_export_and_import(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let mut all = overrides_of_default_roles();
    all.as_object_mut()
        .unwrap()
        .extend(workspace_only_roles().as_object().unwrap().clone());
    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/workspaces/role_overrides"
    )))
    .json(&all)
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/workspaces/tarball\
         ?include_settings=true&settings_version=v2&archive_type=tar"
    )))
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    let archive = resp.bytes().await?;
    let settings: Value = serde_json::from_str(
        &tar_entry(&archive, "settings.json").expect("the export carries settings.json"),
    )?;
    assert_eq!(settings["role_overrides"], overrides_of_default_roles());
    assert_eq!(settings["workspace_only_roles"], workspace_only_roles());

    // The settings document `wmill sync pull` writes states the same.
    let pulled: Value = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/workspaces/settings_document?secrets=encrypted"
    )))
    .send()
    .await?
    .json()
    .await?;
    assert_eq!(pulled["role_overrides"], settings["role_overrides"]);
    assert_eq!(
        pulled["workspace_only_roles"],
        settings["workspace_only_roles"]
    );

    // Importing the export into another workspace reproduces the roles.
    create_workspace(&db, "target-ws").await;
    let resp = apply(port, "target-ws", settings.clone(), false).await;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let result: Value = resp.json().await?;
    assert_eq!(result["applied"], true);
    assert_eq!(
        stored_overrides(&db, "target-ws").await,
        stored_overrides(&db, "test-workspace").await
    );
    let roles: Value = authed(client().get(format!(
        "http://localhost:{port}/api/w/target-ws/workspaces/roles"
    )))
    .send()
    .await?
    .json()
    .await?;
    let role = |name: &str| {
        roles["roles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == name)
            .cloned()
    };
    assert_eq!(role("operator").unwrap()["label"], "Runner");
    assert_eq!(
        role("operator").unwrap()["capabilities"]["flows"],
        "compose"
    );
    assert_eq!(role("developer").unwrap()["capabilities"]["fork"], false);
    if cfg!(feature = "enterprise") {
        let auditor = role("auditor").expect("the workspace-only role is imported");
        assert_eq!(auditor["source"], "workspace");
        assert_eq!(auditor["label"], "Auditor");
    }

    // A second import changes nothing, so a repeated push does not rewrite the roles.
    let result: Value = apply(port, "target-ws", settings, false)
        .await
        .json()
        .await?;
    assert_eq!(result["changes"], json!([]), "{result}");
    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn an_override_of_a_role_the_instance_lacks_is_refused(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let before = stored_overrides(&db, "test-workspace").await;

    let document = json!({
        "name": "test-workspace",
        "webhook": "https://example.com/hook",
        "role_overrides": {
            "operator": { "capabilities": { "manage_schedules": true } },
            "reviewer": { "capabilities": { "code": false } },
        },
    });
    for dry_run in [true, false] {
        let resp = apply(port, "test-workspace", document.clone(), dry_run).await;
        assert_eq!(resp.status(), 400, "dry_run: {dry_run}");
        let message = resp.text().await?;
        assert!(
            message.contains("`reviewer`") && message.contains("does not define"),
            "{message}"
        );
    }
    assert_eq!(stored_overrides(&db, "test-workspace").await, before);
    let webhook: Option<String> =
        sqlx::query_scalar("SELECT webhook FROM workspace_settings WHERE workspace_id = $1")
            .bind("test-workspace")
            .fetch_one(&db)
            .await?;
    assert_eq!(
        webhook, None,
        "the refusal comes before any setting is written"
    );

    // A role the instance defines cannot be added as a workspace-only one either.
    let resp = apply(
        port,
        "test-workspace",
        json!({ "workspace_only_roles": { "operator": { "label": "Mine" } } }),
        true,
    )
    .await;
    assert_eq!(resp.status(), 400);
    assert!(resp.text().await?.contains("already defines"));
    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_document_without_the_roles_keys_leaves_them_alone(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    sqlx::query("UPDATE workspace_settings SET role_overrides = $1 WHERE workspace_id = $2")
        .bind(json!({ "operator": { "capabilities": { "flows": "compose" } } }))
        .bind("test-workspace")
        .execute(&db)
        .await?;

    // A settings.yaml pulled before the keys existed.
    let resp = apply(
        port,
        "test-workspace",
        json!({ "name": "test-workspace", "webhook": "https://example.com/hook" }),
        false,
    )
    .await;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    assert_eq!(
        stored_overrides(&db, "test-workspace").await,
        Some(json!({ "operator": { "capabilities": { "flows": "compose" } } }))
    );
    Ok(())
}

/// Stating only `role_overrides` keeps the workspace-only roles, and stating it as empty clears
/// the overrides.
#[cfg(feature = "enterprise")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn each_roles_key_is_managed_on_its_own(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    sqlx::query("UPDATE workspace_settings SET role_overrides = $1 WHERE workspace_id = $2")
        .bind(json!({
            "operator": { "capabilities": { "flows": "compose" } },
            "auditor": { "label": "Auditor", "capabilities": {} },
        }))
        .bind("test-workspace")
        .execute(&db)
        .await?;

    let resp = apply(
        port,
        "test-workspace",
        json!({ "role_overrides": {} }),
        false,
    )
    .await;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    assert_eq!(
        stored_overrides(&db, "test-workspace").await,
        Some(json!({ "auditor": { "label": "Auditor", "capabilities": {} } }))
    );
    Ok(())
}
