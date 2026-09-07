use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {token}"))
}

const MAIN_KEY: &str = "instance:dt_main";

/// A data table `main` on the instance database `dt_main`, in `w`.
async fn plant_main(db: &Pool<Postgres>, w: &str) {
    sqlx::query(
        "INSERT INTO workspace_settings (workspace_id, datatable) VALUES ($1, $2)
         ON CONFLICT (workspace_id) DO UPDATE SET datatable = EXCLUDED.datatable",
    )
    .bind(w)
    .bind(json!({
        "datatables": {
            "main": { "database": { "resource_type": "instance", "resource_path": "dt_main" } }
        }
    }))
    .execute(db)
    .await
    .unwrap();
}

/// Permissions on a database, owned by `owner`, with one `analyst` role.
async fn plant_permissions(db: &Pool<Postgres>, key: &str, owner: &str, analyst_tenants: &[&str]) {
    sqlx::query(
        "INSERT INTO datatable_database_permissions (database_key, owner_workspace_id, permissions)
         VALUES ($1, $2, $3)",
    )
    .bind(key)
    .bind(owner)
    .bind(json!({ "enabled": true, "roles": {
        "admin": { "tenants": [] },
        "analyst": { "tenants": analyst_tenants, "pg_rolename": "wm_analyst_x", "pg_password": "pw" }
    }}))
    .execute(db)
    .await
    .unwrap();
}

/// Who the `analyst` role of `key` currently lets run as it.
async fn tenants(db: &Pool<Postgres>, key: &str) -> Vec<String> {
    let value: serde_json::Value = sqlx::query_scalar(
        "SELECT permissions->'roles'->'analyst'->'tenants'
         FROM datatable_database_permissions WHERE database_key = $1",
    )
    .bind(key)
    .fetch_one(db)
    .await
    .unwrap();
    serde_json::from_value(value).unwrap()
}

async fn usable_roles(port: u16, w: &str, datatable: &str, token: &str) -> serde_json::Value {
    let resp = authed(
        client().get(format!(
            "http://localhost:{port}/api/w/{w}/workspaces/datatable_usable_roles/{datatable}"
        )),
        token,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200, "{}", resp.text().await.unwrap());
    resp.json().await.unwrap()
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

    plant_main(&db, "test-workspace").await;
    plant_permissions(
        &db,
        MAIN_KEY,
        "test-workspace",
        &[
            "*",
            "u/test-user-2",
            "u/test-user-3",
            "g/leaving_group",
            "f/leaving_folder",
        ],
    )
    .await;

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
    assert!(!tenants(&db, MAIN_KEY)
        .await
        .contains(&"g/leaving_group".to_string()));

    let resp = authed(
        client().delete(format!("{ws}/folders/delete/leaving_folder")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "delete folder: {}", resp.text().await?);
    assert!(!tenants(&db, MAIN_KEY)
        .await
        .contains(&"f/leaving_folder".to_string()));

    // Leaving frees the username as surely as an admin removing the member does.
    let resp = authed(client().post(format!("{ws}/users/leave")), "SECRET_TOKEN_2")
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "leave: {}", resp.text().await?);
    assert!(!tenants(&db, MAIN_KEY)
        .await
        .contains(&"u/test-user-2".to_string()));

    let resp = authed(
        client().delete(format!(
            "http://localhost:{port}/api/users/delete/test3@windmill.dev"
        )),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "global delete: {}", resp.text().await?);
    assert!(!tenants(&db, MAIN_KEY)
        .await
        .contains(&"u/test-user-3".to_string()));

    // What no deletion named is left alone — the wildcard above all, which is
    // not a principal and cannot be freed.
    assert_eq!(tenants(&db, MAIN_KEY).await, vec!["*".to_string()]);

    Ok(())
}

/// The permissions are the database's, so a fork's copy of the data table reaches
/// the same roles — evaluated as a member of the workspace that owns them. Being
/// admin of the fork, which any member is of a fork they made, counts for
/// nothing; a superadmin reaches every role from anywhere; and the roles are
/// managed from the owning workspace alone.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_fork_copy_is_evaluated_as_a_member_of_the_owning_workspace(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    plant_main(&db, "test-workspace").await;
    plant_permissions(&db, MAIN_KEY, "test-workspace", &["u/test-user-3"]).await;
    sqlx::query(
        "INSERT INTO workspace (id, name, owner, parent_workspace_id)
         VALUES ('wm-fork-t', 'wm-fork-t', 'test2@windmill.dev', 'test-workspace')",
    )
    .execute(&db)
    .await?;
    plant_main(&db, "wm-fork-t").await;
    // test-user-2 made the fork and is admin of it, and is no member of the parent.
    sqlx::query(
        "INSERT INTO usr (workspace_id, email, username, is_admin, role) VALUES
           ('wm-fork-t', 'test2@windmill.dev', 'test-user-2', true, 'Admin'),
           ('wm-fork-t', 'test3@windmill.dev', 'test-user-3', false, 'User')",
    )
    .execute(&db)
    .await?;
    sqlx::query(
        "DELETE FROM usr WHERE workspace_id = 'test-workspace' AND email = 'test2@windmill.dev'",
    )
    .execute(&db)
    .await?;

    let fork_admin = usable_roles(port, "wm-fork-t", "main", "SECRET_TOKEN_2").await;
    assert_eq!(fork_admin["enabled"], json!(true));
    assert_eq!(fork_admin["roles"], json!([]), "{fork_admin}");

    let tenant = usable_roles(port, "wm-fork-t", "main", "SECRET_TOKEN_3").await;
    assert_eq!(tenant["roles"], json!(["analyst"]), "{tenant}");

    let superadmin = usable_roles(port, "wm-fork-t", "main", "SECRET_TOKEN").await;
    assert_eq!(
        superadmin["roles"],
        json!(["admin", "analyst"]),
        "{superadmin}"
    );

    // Managed from the owning workspace: the fork's admin reads them, changes nothing.
    let resp = authed(
        client().get(format!(
            "http://localhost:{port}/api/w/wm-fork-t/workspaces/datatable_permissions/main"
        )),
        "SECRET_TOKEN_2",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let info: serde_json::Value = resp.json().await?;
    assert_eq!(info["owner_workspace_id"], json!("test-workspace"));
    assert_eq!(info["editable"], json!(false));
    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/wm-fork-t/workspaces/datatable_permissions/main"
        )),
        "SECRET_TOKEN_2",
    )
    .json(&json!({ "enabled": true, "roles": [
        { "name": "admin", "tenants": [] }, { "name": "analyst", "tenants": ["u/test-user-2"] }
    ]}))
    .send()
    .await?;
    let status = resp.status().as_u16();
    let text = resp.text().await?;
    assert_eq!(status, 401, "{text}");
    assert!(
        text.contains("managed from workspace 'test-workspace'"),
        "{text}"
    );

    Ok(())
}

/// A resource-backed database is the host, port and database its resource
/// resolves to: entries naming it under other paths and other logins, in other
/// workspaces, reach the same permissions; a resource pointed at another database
/// reaches none.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn entries_reaching_one_database_share_its_permissions(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    sqlx::query("INSERT INTO workspace (id, name, owner) VALUES ('other', 'other', 'test-user')")
        .execute(&db)
        .await?;
    sqlx::query(
        "INSERT INTO usr (workspace_id, email, username, is_admin, role)
         VALUES ('other', 'test3@windmill.dev', 'user-three', false, 'User')",
    )
    .execute(&db)
    .await?;
    let plant = |w: &'static str, path: &'static str, user: &'static str, dbname: &'static str| {
        let db = db.clone();
        async move {
            sqlx::query(
                "INSERT INTO resource (workspace_id, path, value, resource_type, created_by, edited_at)
                 VALUES ($1, $2, $3, 'postgresql', 'test-user', now())",
            )
            .bind(w)
            .bind(path)
            .bind(json!({ "host": "db.example", "port": 5432, "dbname": dbname, "user": user, "password": "pw" }))
            .execute(&db)
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO workspace_settings (workspace_id, datatable) VALUES ($1, $2)
                 ON CONFLICT (workspace_id) DO UPDATE SET datatable = EXCLUDED.datatable",
            )
            .bind(w)
            .bind(json!({ "datatables": {
                "byo": { "database": { "resource_type": "postgresql", "resource_path": path } }
            }}))
            .execute(&db)
            .await
            .unwrap();
        }
    };
    plant("test-workspace", "u/test-user/pg", "app", "prod").await;
    plant("other", "f/moved/pg", "postgres", "prod").await;

    let key = windmill_common::workspaces::datatable_database_key(
        &windmill_common::workspaces::DataTableDatabase {
            resource_type: windmill_common::workspaces::DataTableCatalogResourceType::Postgresql,
            resource_path: "u/test-user/pg".to_string(),
        },
        &json!({ "host": "db.example", "port": 5432, "dbname": "prod" }),
    );
    plant_permissions(&db, &key, "test-workspace", &["u/test-user-3"]).await;

    // test-user-3 is `test-user-3` in the owning workspace and `user-three` in
    // `other`: the tenant is matched where it was written.
    let from_other = usable_roles(port, "other", "byo", "SECRET_TOKEN_3").await;
    assert_eq!(from_other["enabled"], json!(true));
    assert_eq!(from_other["roles"], json!(["analyst"]), "{from_other}");

    sqlx::query(
        "UPDATE resource SET value = jsonb_set(value, '{dbname}', '\"staging\"')
         WHERE workspace_id = 'other' AND path = 'f/moved/pg'",
    )
    .execute(&db)
    .await?;
    let elsewhere = usable_roles(port, "other", "byo", "SECRET_TOKEN_3").await;
    assert_eq!(elsewhere["enabled"], json!(false), "{elsewhere}");

    Ok(())
}

/// The permissions a workspace owns follow it through a change of its id.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_rename_moves_the_permissions_it_owns(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    plant_main(&db, "test-workspace").await;
    plant_permissions(&db, MAIN_KEY, "test-workspace", &["u/test-user-3"]).await;

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

    let owner: String = sqlx::query_scalar(
        "SELECT owner_workspace_id FROM datatable_database_permissions WHERE database_key = $1",
    )
    .bind(MAIN_KEY)
    .fetch_one(&db)
    .await?;
    assert_eq!(owner, "renamed-ws");
    let roles = usable_roles(port, "renamed-ws", "main", "SECRET_TOKEN_3").await;
    assert_eq!(roles["roles"], json!(["analyst"]), "{roles}");

    Ok(())
}

/// A fork that keeps the original copies the parent's entry verbatim, `forked_from`
/// included when the parent's own entry is a clone. The stamp means "cloned into
/// this workspace's own database" — it is what lets the fork's deletion drop that
/// database — so a copy must not carry one, and the config form may update the
/// schema snapshot inside an existing stamp but never add or remove one.
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

    let save = |w: &'static str, datatables: serde_json::Value| async move {
        let resp = authed(
            client().post(format!(
                "http://localhost:{port}/api/w/{w}/workspaces/edit_datatable_config"
            )),
            "SECRET_TOKEN",
        )
        .json(&json!({ "settings": { "datatables": datatables } }))
        .send()
        .await
        .unwrap();
        assert_eq!(resp.status(), 200, "{}", resp.text().await.unwrap());
    };
    let stamp = |w: &'static str| {
        let db = db.clone();
        async move {
            let entry: serde_json::Value = sqlx::query_scalar(
                "SELECT datatable->'datatables'->'byo' FROM workspace_settings WHERE workspace_id = $1",
            )
            .bind(w)
            .fetch_one(&db)
            .await
            .unwrap();
            entry["forked_from"].clone()
        }
    };
    let byo =
        json!({ "database": { "resource_type": "postgresql", "resource_path": "u/test-user/pg" } });
    // The form cannot stamp the copy...
    save(
        "wm-fork-kept",
        json!({ "byo": {
            "database": byo["database"], "forked_from": { "schema": {} }
        }}),
    )
    .await;
    assert_eq!(stamp("wm-fork-kept").await, serde_json::Value::Null);
    // ...nor take the parent's stamp away, and it may update the snapshot inside it.
    save("test-workspace", json!({ "byo": byo })).await;
    assert_eq!(stamp("test-workspace").await, json!({ "schema": {} }));
    save(
        "test-workspace",
        json!({ "byo": {
            "database": byo["database"], "forked_from": { "schema": { "t": ["id"] } }
        }}),
    )
    .await;
    assert_eq!(
        stamp("test-workspace").await,
        json!({ "schema": { "t": ["id"] } })
    );

    Ok(())
}

/// A save carries the whole role list the drawer loaded, so it can name a tenant
/// another admin's deletion took off the role in between, and it can leave a
/// role stored migrations still carry in their `-- role` annotation undefined.
/// Both are refused rather than written; turning permissions off is not.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_save_names_only_what_exists(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    plant_main(&db, "test-workspace").await;
    plant_permissions(&db, MAIN_KEY, "test-workspace", &["u/test-user"]).await;
    // Spelled the way the parser accepts and a `-- role ` search would miss.
    sqlx::query(
        "INSERT INTO datatable_migrations (workspace_id, datatable, timestamp, name, code_up, code_down)
         VALUES ('test-workspace', 'main', 1, 'add_orders', '--role analyst\nCREATE TABLE orders ()', NULL)",
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

    let (status, text) = preview(json!({ "enabled": true,
        "roles": [{ "name": "admin", "tenants": [] }]
    }))
    .await;
    assert_eq!(status, 400, "{text}");
    assert!(text.contains("'add_orders' (role 'analyst')"), "{text}");

    // The roles are the database's: a migration of another entry reaching it,
    // here an alias in the same workspace, is stranded all the same.
    sqlx::query(
        r#"UPDATE workspace_settings
           SET datatable = jsonb_set(datatable, '{datatables,alias}',
               '{"database": {"resource_type": "instance", "resource_path": "dt_main"}}')
           WHERE workspace_id = 'test-workspace'"#,
    )
    .execute(&db)
    .await?;
    sqlx::query(
        "UPDATE datatable_migrations SET datatable = 'alias' WHERE workspace_id = 'test-workspace'",
    )
    .execute(&db)
    .await?;
    let (status, text) = preview(json!({ "enabled": true,
        "roles": [{ "name": "admin", "tenants": [] }]
    }))
    .await;
    assert_eq!(status, 400, "{text}");
    assert!(
        text.contains("test-workspace/alias: 'add_orders' (role 'analyst')"),
        "{text}"
    );
    sqlx::query("DELETE FROM datatable_migrations WHERE workspace_id = 'test-workspace'")
        .execute(&db)
        .await?;

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

/// Deleting a workspace takes only the roles it owns with it. A fork that held a
/// copy of the parent's data table owns nothing there, so its deletion leaves the
/// parent's row alone; the owner's own deletion drops its logins but leaves the
/// row, ownerless and closed: every role refused, only a superadmin through.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_deletion_takes_only_the_permissions_it_owns(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    plant_main(&db, "test-workspace").await;
    plant_permissions(&db, MAIN_KEY, "test-workspace", &["u/test-user-3"]).await;
    sqlx::query(
        "INSERT INTO workspace (id, name, owner, parent_workspace_id)
         VALUES ('wm-fork-t', 'wm-fork-t', 'test2@windmill.dev', 'test-workspace')",
    )
    .execute(&db)
    .await?;
    plant_main(&db, "wm-fork-t").await;
    sqlx::query(
        "INSERT INTO usr (workspace_id, email, username, is_admin, role) VALUES
           ('wm-fork-t', 'test2@windmill.dev', 'test-user-2', true, 'Admin'),
           ('wm-fork-t', 'test3@windmill.dev', 'test-user-3', false, 'User')",
    )
    .execute(&db)
    .await?;

    // The fork's owner deletes it: the parent's row is untouched.
    let resp = authed(
        client().delete(format!(
            "http://localhost:{port}/api/workspaces/delete/wm-fork-t"
        )),
        "SECRET_TOKEN_2",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let owner: Option<String> = sqlx::query_scalar(
        "SELECT owner_workspace_id FROM datatable_database_permissions WHERE database_key = $1",
    )
    .bind(MAIN_KEY)
    .fetch_one(&db)
    .await?;
    assert_eq!(owner.as_deref(), Some("test-workspace"));
    let roles = usable_roles(port, "test-workspace", "main", "SECRET_TOKEN_3").await;
    assert_eq!(roles["roles"], json!(["analyst"]), "{roles}");

    // Another workspace reaching the same database, then the owner is deleted.
    sqlx::query(
        "INSERT INTO workspace (id, name, owner) VALUES ('elsewhere', 'elsewhere', 'test-user')",
    )
    .execute(&db)
    .await?;
    plant_main(&db, "elsewhere").await;
    sqlx::query(
        "INSERT INTO usr (workspace_id, email, username, is_admin, role)
         VALUES ('elsewhere', 'test3@windmill.dev', 'test-user-3', true, 'Admin')",
    )
    .execute(&db)
    .await?;
    let resp = authed(
        client().delete(format!(
            "http://localhost:{port}/api/workspaces/delete/test-workspace"
        )),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let owner: Option<String> = sqlx::query_scalar(
        "SELECT owner_workspace_id FROM datatable_database_permissions WHERE database_key = $1",
    )
    .bind(MAIN_KEY)
    .fetch_one(&db)
    .await?;
    assert_eq!(owner, None);
    // Admin of `elsewhere`, and the tenant the row names: neither counts without an owner.
    let closed = usable_roles(port, "elsewhere", "main", "SECRET_TOKEN_3").await;
    assert_eq!(closed["enabled"], json!(true));
    assert_eq!(closed["roles"], json!([]), "{closed}");
    let superadmin = usable_roles(port, "elsewhere", "main", "SECRET_TOKEN").await;
    assert_eq!(
        superadmin["roles"],
        json!(["admin", "analyst"]),
        "{superadmin}"
    );
    // Nobody but a superadmin manages it now: an admin of `elsewhere` reads it, changes nothing.
    let resp = authed(
        client().get(format!(
            "http://localhost:{port}/api/w/elsewhere/workspaces/datatable_permissions/main"
        )),
        "SECRET_TOKEN_3",
    )
    .send()
    .await?;
    let status = resp.status().as_u16();
    let text = resp.text().await?;
    assert_eq!(status, 200, "{text}");
    let info: serde_json::Value = serde_json::from_str(&text)?;
    assert_eq!(info["editable"], json!(false), "{info}");

    Ok(())
}

/// An export carries a database's roles and tenants, never its login names or
/// passwords; importing them governs a database nobody governs yet, owned by
/// the importing workspace, with every role refused until a save creates the
/// logins. A database already governed is left alone and reported.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn imported_permissions_govern_without_logins(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    sqlx::query(
        "INSERT INTO workspace_settings (workspace_id, datatable) VALUES ('test-workspace', $1)
         ON CONFLICT (workspace_id) DO UPDATE SET datatable = EXCLUDED.datatable",
    )
    .bind(json!({ "datatables": {
        "main": { "database": { "resource_type": "instance", "resource_path": "dt_main" } },
        "other": { "database": { "resource_type": "instance", "resource_path": "dt_other" } }
    }}))
    .execute(&db)
    .await?;
    plant_permissions(&db, "instance:dt_other", "test-workspace", &["*"]).await;
    let import = |rows: serde_json::Value| {
        let ws = ws.clone();
        async move {
            let resp = authed(
                client().post(format!("{ws}/workspaces/datatable_permissions_import")),
                "SECRET_TOKEN",
            )
            .json(&rows)
            .send()
            .await
            .unwrap();
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap();
            (status, text)
        }
    };
    let exported = json!([
        { "datatable": "main", "permissions": { "enabled": true, "roles": {
            "admin": { "tenants": [] },
            "analyst": { "tenants": ["u/test-user-3"], "pg_rolename": "wm_analyst_x", "pg_password": "leaked?" }
        }}},
        { "datatable": "other", "permissions": { "enabled": true, "roles": { "admin": { "tenants": [] } } } }
    ]);
    let (status, text) = import(exported).await;
    assert_eq!(status, 200, "{text}");
    assert_eq!(text, "[\"other\"]");
    // A database is named through a data table of this workspace, never by key.
    let (status, text) = import(json!([
        { "datatable": "nope", "permissions": { "enabled": true, "roles": { "admin": { "tenants": [] } } } }
    ]))
    .await;
    assert_eq!(status, 404, "{text}");

    let row: (Option<String>, serde_json::Value) = sqlx::query_as(
        "SELECT owner_workspace_id, permissions FROM datatable_database_permissions WHERE database_key = $1",
    )
    .bind(MAIN_KEY)
    .fetch_one(&db)
    .await?;
    assert_eq!(row.0.as_deref(), Some("test-workspace"));
    // A login name is a cluster-wide identifier the next save would rename or
    // reset, so it is not taken from an import either.
    assert!(
        row.1["roles"]["analyst"].get("pg_rolename").is_none(),
        "{}",
        row.1
    );
    assert!(
        row.1["roles"]["analyst"].get("pg_password").is_none(),
        "{}",
        row.1
    );
    // The tenant is listed as usable; resolving the role, which has no login
    // yet, gives the caller nothing rather than the owning connection.
    let roles = usable_roles(port, "test-workspace", "main", "SECRET_TOKEN_3").await;
    assert_eq!(roles["roles"], json!(["analyst"]), "{roles}");
    let resp = authed(
        client().get(format!(
            "{ws}/workspaces/get_datatable_table_schema?datatable_name=main&schema_name=public&table_name=t&role=analyst"
        )),
        "SECRET_TOKEN_3",
    )
    .send()
    .await?;
    let status = resp.status().as_u16();
    let text = resp.text().await?;
    assert_eq!(status, 401, "{text}");
    assert!(text.contains("has no login yet"), "{text}");

    Ok(())
}

/// A clone of a permissioned data table would be a database of its own that
/// nothing governs, readable in full by every member of the fork: refused at the
/// one place a caller cannot go around, the fork creation that wires it in.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_governed_datatable_cannot_be_cloned_into_a_fork(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    plant_main(&db, "test-workspace").await;
    plant_permissions(&db, MAIN_KEY, "test-workspace", &["*"]).await;
    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/test-workspace/workspaces/create_fork"
        )),
        "SECRET_TOKEN",
    )
    .json(&json!({
        "id": "wm-fork-clone", "name": "clone",
        "forked_datatables": [{ "name": "main", "new_dbname": "wm_fork_clone_main" }]
    }))
    .send()
    .await?;
    let status = resp.status().as_u16();
    let text = resp.text().await?;
    assert_eq!(status, 400, "{text}");
    assert!(text.contains("cannot be cloned"), "{text}");
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM workspace WHERE id = 'wm-fork-clone')")
            .fetch_one(&db)
            .await?;
    assert!(!exists);

    Ok(())
}
