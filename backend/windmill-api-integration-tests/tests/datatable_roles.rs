//! Who may connect to a data table as which role, across the two shapes an entry can take: one
//! that owns its database, and a fork's pointer at it.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {token}"))
}

/// The `analytics` role's tenant list as stored, so a cascade can be observed directly.
async fn tenants(db: &Pool<Postgres>, w_id: &str) -> Vec<String> {
    let value: Option<Value> = sqlx::query_scalar(
        "SELECT datatable->'datatables'->'main'->'permissions'->'roles'->'role1'->'tenants'
         FROM workspace_settings WHERE workspace_id = $1",
    )
    .bind(w_id)
    .fetch_one(db)
    .await
    .unwrap();
    serde_json::from_value(value.unwrap_or(json!([]))).unwrap()
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "datatable_roles"))]
async fn freeing_a_principal_takes_its_datatable_tenant(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    assert_eq!(
        tenants(&db, "test-workspace").await,
        vec!["u/test-user-2", "g/analysts", "f/finance"]
    );

    let resp = authed(
        client().delete(format!("{base}/groups/delete/analysts")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "delete group: {}", resp.text().await?);

    let resp = authed(
        client().delete(format!("{base}/folders/delete/finance")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "delete folder: {}", resp.text().await?);

    let resp = authed(
        client().delete(format!("{base}/users/delete/test-user-2")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "delete user: {}", resp.text().await?);

    // Nothing left naming a principal that no longer exists: a later group or account reusing one
    // of those names must not inherit the access this one had.
    assert!(tenants(&db, "test-workspace").await.is_empty());
    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "datatable_roles"))]
async fn a_fork_uses_the_data_table_it_points_at_but_never_administers_it(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let fork = format!("http://localhost:{port}/api/w/wm-fork-dt/workspaces");

    // `test-user-2` is an admin of the fork and a plain member of the parent. The roles they can
    // use are the ones the parent's tenants give them there, not what their fork admin bit says.
    let resp = authed(
        client().get(format!("{fork}/datatable_usable_roles/main")),
        "SECRET_TOKEN_2",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await?;
    assert_eq!(body["roles"], json!(["analytics"]), "{body}");
    assert_eq!(body["default_role"], "analytics");

    // The drawer names the workspace that decides, and refuses to let the fork edit it.
    let resp = authed(
        client().get(format!("{fork}/datatable_permissions/main")),
        "SECRET_TOKEN_2",
    )
    .send()
    .await?;
    let body: Value = resp.json().await?;
    assert_eq!(body["governing_workspace_id"], "test-workspace");
    assert_eq!(body["editable"], false, "{body}");

    let resp = authed(
        client().post(format!("{fork}/datatable_permissions/main")),
        "SECRET_TOKEN_2",
    )
    .json(&json!({"permissioned": true, "default_role": "admin",
                  "roles": [{"id": "admin", "tenants": ["*"]}]}))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "a fork admin widened the parent's access"
    );

    // Nor by saving the settings form: the pointer is server-owned, so a payload naming the
    // parent's database leaves the entry exactly as it was.
    let resp = authed(
        client().post(format!("{fork}/edit_datatable_config")),
        "SECRET_TOKEN_2",
    )
    .json(&json!({
        "settings": {"datatables": {"main": {
            "database": {"resource_type": "instance", "resource_path": "dt_main"}
        }}},
        "renames": [], "deleted_datatables": []
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    let entry: Option<Value> = sqlx::query_scalar(
        "SELECT datatable->'datatables'->'main' FROM workspace_settings WHERE workspace_id = $1",
    )
    .bind("wm-fork-dt")
    .fetch_one(&db)
    .await?;
    let entry = entry.unwrap();
    assert_eq!(
        entry["reference"]["workspace_id"], "test-workspace",
        "{entry}"
    );
    assert!(entry["database"].is_null(), "{entry}");
    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "datatable_roles"))]
async fn a_second_entry_on_the_same_database_is_reported_rather_than_governed(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    // A copy of the parent's entry, as a fork created before data table roles would hold. It keeps
    // its own access, so the owner is told about it instead of being told it is covered.
    sqlx::query(
        r#"UPDATE workspace_settings SET datatable = '{"datatables": {"copy": {
            "database": {"resource_type": "instance", "resource_path": "dt_main"}}}}'::jsonb
           WHERE workspace_id = 'wm-fork-dt'"#,
    )
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let resp = authed(
        client().get(format!(
            "http://localhost:{port}/api/w/test-workspace/workspaces/datatable_permissions/main"
        )),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    let body: Value = resp.json().await?;
    assert_eq!(
        body["ungoverned_reachers"],
        json!([{"workspace_id": "wm-fork-dt", "datatable": "copy"}]),
        "{body}"
    );
    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "datatable_roles"))]
async fn a_resource_backed_data_table_cannot_be_put_under_roles(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    // A role is a login on Windmill's own cluster. A resource-backed data table dials a host the
    // workspace admin chose, so accepting one here would hand that host a real cluster credential.
    sqlx::query(
        r#"UPDATE workspace_settings SET datatable = '{"datatables": {"byo": {
            "database": {"resource_type": "postgresql", "resource_path": "u/test-user/pg"}}}}'::jsonb
           WHERE workspace_id = 'test-workspace'"#,
    )
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

    let resp = authed(
        client().get(format!("{base}/datatable_permissions/byo")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    let body: Value = resp.json().await?;
    assert_eq!(body["supported"], false, "{body}");

    let resp = authed(
        client().post(format!("{base}/datatable_permissions/byo")),
        "SECRET_TOKEN",
    )
    .json(&json!({"permissioned": true, "default_role": "role1",
                  "roles": [{"id": "role1", "tenants": ["*"]}]}))
    .send()
    .await?;
    assert_eq!(resp.status(), 400, "{}", resp.text().await?);
    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "datatable_roles"))]
async fn a_fork_renaming_its_own_entry_leaves_the_governing_bookkeeping_alone(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    // The parent's migration definitions. A rename or delete through the fork's settings form
    // resolves through the pointer, so without a guard it would relabel or wipe these.
    sqlx::query(
        "INSERT INTO datatable_migrations (workspace_id, datatable, timestamp, name, code_up)
         VALUES ('test-workspace', 'main', 1, 'init', 'SELECT 1')",
    )
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let resp = authed(
        client().post(format!(
            "http://localhost:{port}/api/w/wm-fork-dt/workspaces/edit_datatable_config"
        )),
        "SECRET_TOKEN_2",
    )
    .json(&json!({
        "settings": {"datatables": {}},
        "renames": [],
        "deleted_datatables": ["main"]
    }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    let left: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM datatable_migrations WHERE workspace_id = 'test-workspace'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(left, 1, "the fork's delete reached the parent's migrations");
    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "datatable_roles"))]
async fn a_caller_who_is_not_a_member_of_the_governing_workspace_reaches_nothing(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    // A fork member who was never added to the parent. Their fork membership says nothing there,
    // and the email lookup that would evaluate them as a member of it finds no row.
    sqlx::query(
        "INSERT INTO usr (workspace_id, email, username, is_admin, role)
         VALUES ('wm-fork-dt', 'test3@windmill.dev', 'test-user-3', false, 'User')",
    )
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let resp = authed(
        client().get(format!(
            "http://localhost:{port}/api/w/wm-fork-dt/workspaces/datatable_usable_roles/main"
        )),
        "SECRET_TOKEN_3",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await?;
    assert_eq!(body["roles"], json!([]), "{body}");
    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "datatable_roles"))]
async fn a_caller_with_no_identity_reaches_a_permissioned_data_table_not_at_all(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    use windmill_common::workspaces::{get_datatable_resource_from_db, DatatableAccess};

    initialize_tracing().await;
    // The compatibility story for an agent worker that predates data table roles and sends no job
    // id: it keeps resolving an unpermissioned data table, and is refused on a permissioned one
    // rather than handed an unattributed admin connection.
    let refused = get_datatable_resource_from_db(
        &db,
        "test-workspace",
        "main",
        None,
        DatatableAccess::NoIdentity,
    )
    .await;
    assert!(refused.is_err(), "an unidentified caller was let in");

    sqlx::query(
        "UPDATE workspace_settings
         SET datatable = datatable #- '{datatables,main,permissions}'
         WHERE workspace_id = 'test-workspace'",
    )
    .execute(&db)
    .await?;
    let resolved = get_datatable_resource_from_db(
        &db,
        "test-workspace",
        "main",
        None,
        DatatableAccess::NoIdentity,
    )
    .await?;
    assert_eq!(resolved["dbname"], "dt_main", "{resolved}");
    Ok(())
}
