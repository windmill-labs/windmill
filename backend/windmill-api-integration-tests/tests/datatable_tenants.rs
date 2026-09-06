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

    // Where the stamp exists it survives a save that omits it — the form
    // round-trips configs the client trimmed — and its schema snapshot follows a
    // save that carries one, which is how the schema diff records its baseline.
    let save = |datatables: serde_json::Value| {
        let ws = ws.clone();
        async move {
            let resp = authed(
                client().post(format!("{ws}/workspaces/edit_datatable_config")),
                "SECRET_TOKEN",
            )
            .json(&json!({ "settings": { "datatables": datatables } }))
            .send()
            .await
            .unwrap();
            assert_eq!(resp.status(), 200, "{}", resp.text().await.unwrap());
        }
    };
    let stamp = |db: Pool<Postgres>| async move {
        let entry: serde_json::Value = sqlx::query_scalar(
            "SELECT datatable->'datatables'->'byo' FROM workspace_settings WHERE workspace_id = 'test-workspace'",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        entry["forked_from"].clone()
    };
    save(json!({ "byo": {
        "database": { "resource_type": "postgresql", "resource_path": "u/test-user/pg" }
    }}))
    .await;
    assert_eq!(stamp(db.clone()).await, json!({ "schema": {} }));
    save(json!({ "byo": {
        "database": { "resource_type": "postgresql", "resource_path": "u/test-user/pg" },
        "forked_from": { "schema": { "t": ["id"] } }
    }}))
    .await;
    assert_eq!(
        stamp(db.clone()).await,
        json!({ "schema": { "t": ["id"] } })
    );

    Ok(())
}

/// A save carries the whole role list the drawer loaded, so it can name a tenant
/// another admin's deletion took off the role in between, and a rename can name
/// a role that stored migrations still carry in their `-- role` annotation. Both
/// are refused rather than written back.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_save_names_only_what_exists(db: Pool<Postgres>) -> anyhow::Result<()> {
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
                    "analyst": { "tenants": ["u/test-user"] }
                }}
            }
        }
    }))
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO datatable_migrations (workspace_id, datatable, timestamp, name, code_up, code_down)
         VALUES ('test-workspace', 'main', 1, 'add_orders', '-- role analyst\nCREATE TABLE orders ()', NULL)",
    )
    .execute(&db)
    .await?;

    let preview = |body: serde_json::Value| {
        let ws = ws.clone();
        async move {
            let resp = authed(
                client().post(format!(
                    "{ws}/workspaces/datatable_permissions/main/preview"
                )),
                "SECRET_TOKEN",
            )
            .json(&body)
            .send()
            .await
            .unwrap();
            (resp.status().as_u16(), resp.text().await.unwrap())
        }
    };

    let (status, text) = preview(json!({ "enabled": true, "roles": [
        { "name": "admin", "tenants": [] },
        { "name": "analyst", "tenants": ["u/test-user", "u/ghost", "g/nobody", "f/nowhere", "*"] }
    ]}))
    .await;
    assert_eq!(status, 400, "{text}");
    assert!(text.contains("f/nowhere, g/nobody, u/ghost"), "{text}");

    let (status, text) = preview(json!({ "enabled": true,
        "roles": [{ "name": "admin", "tenants": [] }, { "name": "reader", "tenants": ["u/test-user"] }],
        "renames": [{ "from": "analyst", "to": "reader" }]
    }))
    .await;
    assert_eq!(status, 400, "{text}");
    assert!(text.contains("'add_orders' (role 'analyst')"), "{text}");

    // Removing the role strands the migration the same way.
    let (status, text) = preview(json!({ "enabled": true,
        "roles": [{ "name": "admin", "tenants": [] }]
    }))
    .await;
    assert_eq!(status, 400, "{text}");
    assert!(text.contains("'add_orders' (role 'analyst')"), "{text}");

    // Turning permissions off ignores the submitted roles and is never refused,
    // stale tenant or not: it gets as far as the database this test lacks.
    let (_, text) = preview(json!({ "enabled": false, "roles": [
        { "name": "admin", "tenants": [] },
        { "name": "analyst", "tenants": ["u/ghost"] }
    ]}))
    .await;
    assert!(
        !text.contains("no longer exist") && !text.contains("Migration(s)"),
        "{text}"
    );

    // The same save with what exists gets past both checks, to the database this
    // test does not have.
    let (_, text) = preview(json!({ "enabled": true, "roles": [
        { "name": "admin", "tenants": [] },
        { "name": "analyst", "tenants": ["u/test-user", "*"] }
    ]}))
    .await;
    assert!(
        !text.contains("no longer exist") && !text.contains("Migration(s)"),
        "{text}"
    );

    Ok(())
}

/// Another workspace naming the same resource path holds a resource of its own,
/// which counts only when it resolves to the same database — a detached dev
/// workspace's cloned resource does, an unrelated workspace's same-named one
/// does not.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_same_named_resource_counts_only_when_it_reaches_the_same_database(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    sqlx::query(
        "INSERT INTO workspace (id, name, owner) VALUES ('detached', 'detached', 'test-user')",
    )
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO usr (workspace_id, email, username, is_admin, role)
         VALUES ('detached', 'test@windmill.dev', 'test-user', true, 'Admin')",
    )
    .execute(&db)
    .await?;
    let byo =
        json!({ "database": { "resource_type": "postgresql", "resource_path": "u/test-user/pg" } });
    for w in ["test-workspace", "detached"] {
        sqlx::query(
            "INSERT INTO workspace_settings (workspace_id, datatable) VALUES ($1, $2)
             ON CONFLICT (workspace_id) DO UPDATE SET datatable = EXCLUDED.datatable",
        )
        .bind(w)
        .bind(json!({ "datatables": { "byo": byo } }))
        .execute(&db)
        .await?;
    }
    let create_pg = |w: &'static str, dbname: &'static str| async move {
        let resp = authed(
            client().post(format!("http://localhost:{port}/api/w/{w}/resources/create")),
            "SECRET_TOKEN",
        )
        .json(&json!({
            "path": "u/test-user/pg",
            "resource_type": "postgresql",
            "value": { "host": "db.example", "port": 5432, "dbname": dbname, "user": "app", "password": "pw", "sslmode": "disable" }
        }))
        .send()
        .await
        .unwrap();
        assert_eq!(resp.status(), 201, "{w}: {}", resp.text().await.unwrap());
    };
    create_pg("test-workspace", "prod").await;
    create_pg("detached", "prod").await;

    let preview = || async {
        let resp = authed(
            client().post(format!(
                "http://localhost:{port}/api/w/test-workspace/workspaces/datatable_permissions/byo/preview"
            )),
            "SECRET_TOKEN",
        )
        .json(&json!({ "enabled": true, "roles": [] }))
        .send()
        .await
        .unwrap();
        resp.text().await.unwrap()
    };
    let text = preview().await;
    assert!(
        text.contains("same database: detached (data table 'byo')"),
        "{text}"
    );

    // Same path, another database: not a copy.
    sqlx::query(
        "UPDATE resource SET value = jsonb_set(value, '{dbname}', '\"other\"')
         WHERE workspace_id = 'detached' AND path = 'u/test-user/pg'",
    )
    .execute(&db)
    .await?;
    let text = preview().await;
    assert!(!text.contains("cannot be enabled"), "{text}");

    // Another path, the same database: a copy, wherever the resource was moved.
    sqlx::query(
        "UPDATE resource SET path = 'f/moved/pg', value = jsonb_set(value, '{dbname}', '\"prod\"')
         WHERE workspace_id = 'detached' AND path = 'u/test-user/pg'",
    )
    .execute(&db)
    .await?;
    sqlx::query(
        r#"UPDATE workspace_settings
           SET datatable = jsonb_set(datatable, '{datatables,byo,database,resource_path}', '"f/moved/pg"')
           WHERE workspace_id = 'detached'"#,
    )
    .execute(&db)
    .await?;
    let text = preview().await;
    assert!(
        text.contains("same database: detached (data table 'byo')"),
        "{text}"
    );

    // A second entry of this workspace on the same resource is a second door.
    sqlx::query("DELETE FROM workspace_settings WHERE workspace_id = 'detached'")
        .execute(&db)
        .await?;
    sqlx::query(
        r#"UPDATE workspace_settings
           SET datatable = jsonb_set(datatable, '{datatables,byo2}', $1)
           WHERE workspace_id = 'test-workspace'"#,
    )
    .bind(&byo)
    .execute(&db)
    .await?;
    let text = preview().await;
    assert!(
        text.contains("same database: test-workspace (data table 'byo2')"),
        "{text}"
    );

    Ok(())
}
