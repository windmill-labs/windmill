use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {token}"))
}

/// Who the data table's one role currently lets run as it.
async fn tenants(db: &Pool<Postgres>) -> Vec<String> {
    let value: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT datatable->'datatables'->'main'->'permissions'->'roles'->'analyst'->'tenants'
         FROM workspace_settings WHERE workspace_id = 'test-workspace'",
    )
    .fetch_one(db)
    .await
    .unwrap();
    serde_json::from_value(value.unwrap()).unwrap()
}

/// A tenant is a name, and a name outlives the principal that held it: whoever
/// takes it next would run as the role it still names. Every route that frees
/// one has to take it off the role, which is easy to miss from any single one of
/// them — so they are pinned together.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn freeing_a_principal_takes_its_datatable_tenant(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    sqlx::query(
        r#"UPDATE workspace_settings SET datatable = $1 WHERE workspace_id = 'test-workspace'"#,
    )
    .bind(json!({
        "datatables": {
            "main": {
                "database": { "resource_type": "instance", "resource_path": "dt_main" },
                "permissions": { "enabled": true, "roles": {
                    "admin": { "tenants": [] },
                    "analyst": { "tenants": [
                        "*", "u/test-user-2", "u/test-user-3", "g/leaving_group", "f/leaving_folder"
                    ]}
                }}
            }
        }
    }))
    .execute(&db)
    .await?;

    for (endpoint, body) in [
        ("groups/create", json!({ "name": "leaving_group" })),
        ("folders/create", json!({ "name": "leaving_folder" })),
    ] {
        let resp = authed(client().post(format!("{ws}/{endpoint}")), "SECRET_TOKEN")
            .json(&body)
            .send()
            .await?;
        assert_eq!(resp.status(), 200, "{endpoint}: {}", resp.text().await?);
    }

    let resp = authed(
        client().delete(format!("{ws}/groups/delete/leaving_group")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "delete group: {}", resp.text().await?);
    assert!(!tenants(&db).await.contains(&"g/leaving_group".to_string()));

    let resp = authed(
        client().delete(format!("{ws}/folders/delete/leaving_folder")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "delete folder: {}", resp.text().await?);
    assert!(!tenants(&db).await.contains(&"f/leaving_folder".to_string()));

    // Leaving frees the username as surely as an admin removing the member does.
    let resp = authed(client().post(format!("{ws}/users/leave")), "SECRET_TOKEN_2")
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "leave: {}", resp.text().await?);
    assert!(!tenants(&db).await.contains(&"u/test-user-2".to_string()));

    let resp = authed(
        client().delete(format!(
            "http://localhost:{port}/api/users/delete/test3@windmill.dev"
        )),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "global delete: {}", resp.text().await?);
    assert!(!tenants(&db).await.contains(&"u/test-user-3".to_string()));

    // What no deletion named is left alone — the wildcard above all, which is
    // not a principal and cannot be freed.
    assert_eq!(tenants(&db).await, vec!["*".to_string()]);

    Ok(())
}

/// A fork made while the data table was unpermissioned carries a copy of it that
/// points at the same database; opting in would leave every member of the fork
/// reaching that database through the copy's own connection. Pinned on the save
/// and on the preview, since a plan the save refuses to run must not be offered.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn enabling_permissions_is_refused_while_a_fork_exists(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    sqlx::query(
        r#"UPDATE workspace_settings SET datatable = $1 WHERE workspace_id = 'test-workspace'"#,
    )
    .bind(json!({
        "datatables": {
            "main": { "database": { "resource_type": "instance", "resource_path": "dt_main" } },
            "byo": { "database": { "resource_type": "postgresql", "resource_path": "u/test-user/pg" } }
        }
    }))
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO workspace (id, name, owner, parent_workspace_id)
         VALUES ('wm-fork-t', 'wm-fork-t', 'test-user', 'test-workspace')",
    )
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO workspace_settings (workspace_id, datatable) VALUES ('wm-fork-t', $1)",
    )
    .bind(json!({
        "datatables": {
            "main": { "database": { "resource_type": "instance", "resource_path": "dt_main" } },
            "byo": { "database": { "resource_type": "postgresql", "resource_path": "u/test-user/pg" } }
        }
    }))
    .execute(&db)
    .await?;

    let body = json!({ "enabled": true, "roles": [] });
    for endpoint in [
        "workspaces/datatable_permissions/main/preview",
        "workspaces/datatable_permissions/main",
    ] {
        let resp = authed(client().post(format!("{ws}/{endpoint}")), "SECRET_TOKEN")
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        assert_eq!(status, 400, "{endpoint}: {text}");
        assert!(
            text.contains("wm-fork-t (data table 'main')"),
            "{endpoint}: {text}"
        );
    }

    // Only the opt-in is gated: once permissions are on, forks made afterwards
    // never receive the data table, and editing the roles — revoking a tenant
    // above all — has to keep working while they exist. The preview then gets as
    // far as the database, which this test does not have.
    sqlx::query(
        r#"UPDATE workspace_settings
           SET datatable = jsonb_set(datatable, '{datatables,main,permissions}',
               '{"enabled": true, "roles": {"admin": {"tenants": []}}}')
           WHERE workspace_id = 'test-workspace'"#,
    )
    .execute(&db)
    .await?;
    let edit = json!({ "enabled": true, "roles": [
        { "name": "admin", "tenants": [] }, { "name": "analyst", "tenants": ["u/test-user"] }
    ]});
    let resp = authed(
        client().post(format!(
            "{ws}/workspaces/datatable_permissions/main/preview"
        )),
        "SECRET_TOKEN",
    )
    .json(&edit)
    .send()
    .await?;
    let text = resp.text().await?;
    assert!(!text.contains("cannot be enabled"), "{text}");
    sqlx::query(
        r#"UPDATE workspace_settings
           SET datatable = datatable #- '{datatables,main,permissions}'
           WHERE workspace_id = 'test-workspace'"#,
    )
    .execute(&db)
    .await?;

    // A resource-backed copy keeps the parent's pointer when cloned — the cloned
    // resource is what changes — so `forked_from` is what tells the two apart.
    let byo = format!("{ws}/workspaces/datatable_permissions/byo/preview");
    let resp = authed(client().post(&byo), "SECRET_TOKEN")
        .json(&body)
        .send()
        .await?;
    assert_eq!(resp.status(), 400);
    let text = resp.text().await?;
    assert!(text.contains("wm-fork-t (data table 'byo')"), "{text}");
    sqlx::query(
        r#"UPDATE workspace_settings
           SET datatable = jsonb_set(datatable, '{datatables,byo,forked_from}', '{"schema": {}}')
           WHERE workspace_id = 'wm-fork-t'"#,
    )
    .execute(&db)
    .await?;
    // Past the refusal the preview fails on the resource, which this test does
    // not have; the refusal is what is pinned.
    let resp = authed(client().post(&byo), "SECRET_TOKEN")
        .json(&body)
        .send()
        .await?;
    let text = resp.text().await?;
    assert!(!text.contains("cannot be enabled"), "{text}");

    // Archiving keeps the fork's members and its copy, so it still counts; a fork
    // whose copy is a clone of its own does not.
    sqlx::query("UPDATE workspace SET deleted = true WHERE id = 'wm-fork-t'")
        .execute(&db)
        .await?;
    let resp = authed(
        client().post(format!(
            "{ws}/workspaces/datatable_permissions/main/preview"
        )),
        "SECRET_TOKEN",
    )
    .json(&body)
    .send()
    .await?;
    assert_eq!(resp.status(), 400);
    let text = resp.text().await?;
    assert!(
        text.contains("wm-fork-t, archived (data table 'main')"),
        "{text}"
    );
    sqlx::query(
        r#"UPDATE workspace_settings
           SET datatable = jsonb_set(
               jsonb_set(datatable, '{datatables,main,forked_from}', '{"schema": {}}'),
               '{datatables,main,database,resource_path}', '"wm_fork_dt_main"')
           WHERE workspace_id = 'wm-fork-t'"#,
    )
    .execute(&db)
    .await?;
    let resp = authed(
        client().post(format!(
            "{ws}/workspaces/datatable_permissions/main/preview"
        )),
        "SECRET_TOKEN",
    )
    .json(&body)
    .send()
    .await?;
    let text = resp.text().await?;
    assert!(!text.contains("cannot be enabled"), "{text}");

    // A workspace that is no longer a fork — a detached dev workspace — keeps its
    // copy of the data table, pointing at the same instance database.
    sqlx::query("DELETE FROM workspace_settings WHERE workspace_id = 'wm-fork-t'")
        .execute(&db)
        .await?;
    sqlx::query("DELETE FROM workspace WHERE id = 'wm-fork-t'")
        .execute(&db)
        .await?;
    sqlx::query(
        "INSERT INTO workspace (id, name, owner) VALUES ('detached', 'detached', 'test-user')",
    )
    .execute(&db)
    .await?;
    sqlx::query("INSERT INTO workspace_settings (workspace_id, datatable) VALUES ('detached', $1)")
        .bind(json!({
            "datatables": {
                "copy": { "database": { "resource_type": "instance", "resource_path": "dt_main" } }
            }
        }))
        .execute(&db)
        .await?;
    let resp = authed(
        client().post(format!(
            "{ws}/workspaces/datatable_permissions/main/preview"
        )),
        "SECRET_TOKEN",
    )
    .json(&body)
    .send()
    .await?;
    assert_eq!(resp.status(), 400);
    let text = resp.text().await?;
    assert!(text.contains("detached (data table 'copy')"), "{text}");

    // Archived, it still counts; with the copy gone the refusal lifts, and the
    // preview then gets as far as the database, which this test does not have.
    sqlx::query("UPDATE workspace SET deleted = true WHERE id = 'detached'")
        .execute(&db)
        .await?;
    let resp = authed(
        client().post(format!(
            "{ws}/workspaces/datatable_permissions/main/preview"
        )),
        "SECRET_TOKEN",
    )
    .json(&body)
    .send()
    .await?;
    let text = resp.text().await?;
    assert!(
        text.contains("detached, archived (data table 'copy')"),
        "{text}"
    );
    sqlx::query("DELETE FROM workspace_settings WHERE workspace_id = 'detached'")
        .execute(&db)
        .await?;
    let resp = authed(
        client().post(format!(
            "{ws}/workspaces/datatable_permissions/main/preview"
        )),
        "SECRET_TOKEN",
    )
    .json(&body)
    .send()
    .await?;
    let text = resp.text().await?;
    assert!(!text.contains("cannot be enabled"), "{text}");

    Ok(())
}

/// The rename keeps a copy of the settings under the archived id, and commits it
/// before the old id is archived. No data table may be in that copy: a
/// permissioned one without its `permissions` block would resolve, for anyone
/// still using the old id, to the owner connection, and any one naming the same
/// instance database would keep the renamed workspace from opting in.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_rename_leaves_no_datatable_under_the_old_id(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    sqlx::query(
        r#"UPDATE workspace_settings SET datatable = $1 WHERE workspace_id = 'test-workspace'"#,
    )
    .bind(json!({
        "datatables": {
            "open": { "database": { "resource_type": "instance", "resource_path": "dt_open" } },
            "main": {
                "database": { "resource_type": "instance", "resource_path": "dt_main" },
                "permissions": { "enabled": true, "roles": {
                    "admin": { "tenants": [] },
                    "analyst": { "tenants": ["*"], "pg_rolename": "wm_x", "pg_password": "s3cret" }
                }}
            }
        }
    }))
    .execute(&db)
    .await?;

    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/test-workspace/workspaces/change_workspace_id"
        )),
        "SECRET_TOKEN",
    )
    .json(&json!({ "new_id": "renamed-ws", "new_name": "Renamed" }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    let names = |w: &'static str| {
        let db = db.clone();
        async move {
            let value: serde_json::Value = sqlx::query_scalar(
                "SELECT datatable->'datatables' FROM workspace_settings WHERE workspace_id = $1",
            )
            .bind(w)
            .fetch_one(&db)
            .await
            .unwrap();
            let mut keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();
            keys.sort();
            (keys, value)
        }
    };
    let (old_names, _) = names("test-workspace").await;
    assert!(old_names.is_empty(), "{old_names:?}");
    let (new_names, new_value) = names("renamed-ws").await;
    assert_eq!(new_names, vec!["main".to_string(), "open".to_string()]);
    assert_eq!(new_value["main"]["permissions"]["enabled"], json!(true));
    assert_eq!(
        new_value["main"]["permissions"]["roles"]["analyst"]["pg_password"],
        json!("s3cret")
    );

    Ok(())
}

/// A fork that keeps the original copies the parent's entry verbatim, `forked_from`
/// included when the parent's own entry is a clone (a detached dev workspace keeps
/// its clones). The stamp means "cloned into this workspace's own database", and
/// the opt-in trusts it, so a copy must not carry one.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_kept_original_does_not_inherit_the_clone_stamp(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    sqlx::query(
        r#"UPDATE workspace_settings SET datatable = $1 WHERE workspace_id = 'test-workspace'"#,
    )
    .bind(json!({
        "datatables": {
            "byo": {
                "database": { "resource_type": "postgresql", "resource_path": "u/test-user/pg" },
                "forked_from": { "schema": {} }
            }
        }
    }))
    .execute(&db)
    .await?;

    let resp = authed(
        client().post(format!("{ws}/workspaces/create_fork")),
        "SECRET_TOKEN",
    )
    .json(&json!({ "id": "wm-fork-kept", "name": "kept" }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    let copy: serde_json::Value = sqlx::query_scalar(
        "SELECT datatable->'datatables'->'byo' FROM workspace_settings WHERE workspace_id = 'wm-fork-kept'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(copy["database"]["resource_path"], json!("u/test-user/pg"));
    assert!(copy.get("forked_from").is_none(), "{copy}");

    let resp = authed(
        client().post(format!("{ws}/workspaces/datatable_permissions/byo/preview")),
        "SECRET_TOKEN",
    )
    .json(&json!({ "enabled": true, "roles": [] }))
    .send()
    .await?;
    assert_eq!(resp.status(), 400);
    let text = resp.text().await?;
    assert!(text.contains("wm-fork-kept (data table 'byo')"), "{text}");

    // The form cannot stamp the copy either.
    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/wm-fork-kept/workspaces/edit_datatable_config"
        )),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "settings": { "datatables": {
            "byo": {
                "database": { "resource_type": "postgresql", "resource_path": "u/test-user/pg" },
                "forked_from": { "schema": {} }
            }
        }}
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let copy: serde_json::Value = sqlx::query_scalar(
        "SELECT datatable->'datatables'->'byo' FROM workspace_settings WHERE workspace_id = 'wm-fork-kept'",
    )
    .fetch_one(&db)
    .await?;
    assert!(copy.get("forked_from").is_none(), "{copy}");

    Ok(())
}
