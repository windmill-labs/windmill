//! Regression test for running data table migrations against a database whose
//! role only holds DML grants.
//!
//! Two failure modes are pinned here:
//!  - the Postgres message must reach the caller. `tokio_postgres::Error`'s
//!    `Display` renders only the error kind, so interpolating it with `{}`
//!    produced a bare `Failed to ensure _wm_migrations table: db error`.
//!  - `CREATE TABLE IF NOT EXISTS` checks CREATE on the schema *before* it
//!    checks existence, so the run must probe for `_wm_migrations` first or an
//!    unprivileged role can never migrate, even against a pre-created table.
//!
//! Plus the privilege report that surfaces the same state from workspace
//! settings before anyone reaches a migration.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

const ROLE: &str = "wm_dtmig_test_role";
const ROLE_PASSWORD: &str = "wm_dtmig_test_pwd";
/// Deliberately hyphenated: it only parses inside double quotes, so it pins that
/// the emitted recovery statement quotes the role rather than interpolating it.
const NOSCHEMA_ROLE: &str = "wm-dtmig-noschema";

fn authed(b: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    b.header("Authorization", "Bearer DTMIG_ADMIN_TOKEN")
}

/// Point the fixture's data table at this test's own database, connecting as a
/// role that may read and write but not create: `GRANT USAGE` without `CREATE`,
/// and the schema's own CREATE revoked from PUBLIC so the outcome does not
/// depend on the server's default `public` grants (relaxed before Postgres 15).
async fn setup_unprivileged_datatable_role(db: &Pool<Postgres>) -> anyhow::Result<()> {
    let opts = (*db.connect_options()).clone();
    let dbname = opts.get_database().expect("test database name").to_string();

    sqlx::query(&format!(
        // Roles are cluster objects, not per-test-database ones. A previous run
        // leaving the role behind raises duplicate_object; the tests in this
        // binary run in parallel, so two sessions can also clear that check
        // together and collide on pg_authid's unique index instead.
        "DO $$ BEGIN \
           CREATE ROLE {ROLE} LOGIN PASSWORD '{ROLE_PASSWORD}'; \
         EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL; \
         END $$"
    ))
    .execute(db)
    .await?;
    sqlx::raw_sql(&format!(
        "REVOKE CREATE ON SCHEMA public FROM PUBLIC; \
         GRANT CONNECT ON DATABASE \"{dbname}\" TO {ROLE}; \
         GRANT USAGE ON SCHEMA public TO {ROLE};"
    ))
    .execute(db)
    .await?;

    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type, created_by) \
         VALUES ('dtmig-ws', 'u/dtmig-admin/pg', $1, 'postgresql', 'dtmig-admin')",
    )
    .bind(json!({
        "host": opts.get_host(),
        "port": opts.get_port(),
        "dbname": dbname,
        "user": ROLE,
        "password": ROLE_PASSWORD,
        "sslmode": "disable",
    }))
    .execute(db)
    .await?;

    Ok(())
}

#[sqlx::test(fixtures("datatable_migrations_grants"))]
async fn test_run_migrations_without_create_privilege(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    setup_unprivileged_datatable_role(&db).await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let url =
        format!("http://localhost:{port}/api/w/dtmig-ws/workspaces/run_datatable_migrations/main");

    // No `_wm_migrations` yet and no way to create one: the caller must be told
    // what Postgres actually refused, not "db error".
    let resp = authed(reqwest::Client::new().post(&url)).send().await?;
    assert_eq!(resp.status(), 500);
    let body = resp.text().await?;
    assert!(
        body.contains("permission denied for schema"),
        "the Postgres message should reach the caller, got: {body}"
    );
    // The suggested statement must be complete and quoted, not a placeholder.
    assert!(
        body.contains(&format!("GRANT CREATE ON SCHEMA \"public\" TO \"{ROLE}\"")),
        "the hint should name the actual role and schema, got: {body}"
    );

    // Once an operator has created the bookkeeping table and granted DML on it,
    // migrations run even though the role still cannot create tables.
    sqlx::raw_sql(&format!(
        "CREATE TABLE _wm_migrations ( \
            datatable TEXT NOT NULL, \
            version BIGINT NOT NULL, \
            installed_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
            PRIMARY KEY (datatable, version)); \
         GRANT SELECT, INSERT, UPDATE, DELETE ON _wm_migrations TO {ROLE};"
    ))
    .execute(&db)
    .await?;

    let resp = authed(reqwest::Client::new().post(&url)).send().await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 200,
        "run should succeed on a pre-created table: {body}"
    );

    Ok(())
}

#[sqlx::test(fixtures("datatable_migrations_grants"))]
async fn test_datatable_connection_report(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    setup_unprivileged_datatable_role(&db).await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let url =
        format!("http://localhost:{port}/api/w/dtmig-ws/workspaces/test_datatable_connection/main");

    // The report is a privilege disclosure about the data table's database, so
    // it stays behind the same bar as editing the data table config.
    let resp = reqwest::Client::new()
        .get(&url)
        .header("Authorization", "Bearer DTMIG_USER_TOKEN")
        .send()
        .await?;
    assert_eq!(resp.status(), 403, "non-admins must not get the report");

    let report: Value = authed(reqwest::Client::new().get(&url))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(report["user"], ROLE);
    assert_eq!(report["schema"], "public");
    assert_eq!(report["can_create_table"], false);
    assert_eq!(report["can_create_schema"], false);
    let grants = report["suggested_grants"].as_array().unwrap();
    assert!(
        grants
            .iter()
            .any(|g| g.as_str().unwrap()
                == format!("GRANT CREATE ON SCHEMA \"public\" TO \"{ROLE}\"")),
        "missing schema grant: {report}"
    );
    // Pin the name, not just the shape: the endpoint reads it from
    // `current_database()` rather than the resource, and a prefix assertion
    // would pass either way.
    let dbname = (*db.connect_options())
        .clone()
        .get_database()
        .expect("test database name")
        .to_string();
    assert!(
        grants.iter().any(|g| g.as_str().unwrap()
            == format!("GRANT CREATE ON DATABASE \"{dbname}\" TO \"{ROLE}\"")),
        "missing database grant for {dbname}: {report}"
    );

    // A pre-created bookkeeping table lets migration *tracking* work, but the
    // role still cannot create anything: the report must keep saying so rather
    // than falling silent because nothing needs creating right now.
    sqlx::raw_sql(&format!(
        "CREATE TABLE _wm_migrations ( \
            datatable TEXT NOT NULL, \
            version BIGINT NOT NULL, \
            installed_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
            PRIMARY KEY (datatable, version)); \
         GRANT SELECT, INSERT, UPDATE, DELETE ON _wm_migrations TO {ROLE};"
    ))
    .execute(&db)
    .await?;

    let report: Value = authed(reqwest::Client::new().get(&url))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(report["migrations_table_exists"], true);
    assert_eq!(report["can_create_table"], false);
    assert!(
        report["suggested_grants"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g.as_str().unwrap().contains("ON SCHEMA")),
        "an existing bookkeeping table must not suppress the schema grant: {report}"
    );

    // Granting the privileges clears the suggestions.
    sqlx::raw_sql(&format!(
        "GRANT CREATE ON SCHEMA public TO {ROLE}; \
         GRANT CREATE ON DATABASE \"{dbname}\" TO {ROLE};"
    ))
    .execute(&db)
    .await?;

    let report: Value = authed(reqwest::Client::new().get(&url))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(report["can_create_table"], true);
    assert_eq!(report["can_create_schema"], true);
    assert_eq!(report["suggested_grants"].as_array().unwrap().len(), 0);

    Ok(())
}

/// Point the fixture's second data table at a role whose `search_path` resolves
/// to nothing, the one state where no grant helps.
async fn setup_schemaless_datatable_role(db: &Pool<Postgres>) -> anyhow::Result<()> {
    let opts = (*db.connect_options()).clone();
    let dbname = opts.get_database().expect("test database name").to_string();

    sqlx::query(&format!(
        "DO $$ BEGIN \
           CREATE ROLE \"{NOSCHEMA_ROLE}\" LOGIN PASSWORD '{ROLE_PASSWORD}'; \
         EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL; \
         END $$"
    ))
    .execute(db)
    .await?;
    // Cluster-wide for this role, which is why it gets one of its own rather
    // than sharing the role the other assertions connect with.
    sqlx::raw_sql(&format!(
        "ALTER ROLE \"{NOSCHEMA_ROLE}\" SET search_path = wm_dtmig_absent_schema; \
         GRANT CONNECT ON DATABASE \"{dbname}\" TO \"{NOSCHEMA_ROLE}\";"
    ))
    .execute(db)
    .await?;

    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type, created_by) \
         VALUES ('dtmig-ws', 'u/dtmig-admin/pg_noschema', $1, 'postgresql', 'dtmig-admin')",
    )
    .bind(json!({
        "host": opts.get_host(),
        "port": opts.get_port(),
        "dbname": dbname,
        "user": NOSCHEMA_ROLE,
        "password": ROLE_PASSWORD,
        "sslmode": "disable",
    }))
    .execute(db)
    .await?;

    Ok(())
}

#[sqlx::test(fixtures("datatable_migrations_grants"))]
async fn test_datatable_connection_without_a_resolvable_schema(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    setup_schemaless_datatable_role(&db).await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let report: Value = authed(reqwest::Client::new().get(format!(
        "http://localhost:{port}/api/w/dtmig-ws/workspaces/test_datatable_connection/noschema"
    )))
    .send()
    .await?
    .json()
    .await?;

    assert!(report["schema"].is_null(), "expected no schema: {report}");
    // No grant fixes an empty search_path, so suggesting one would send the
    // reader after a statement that changes nothing.
    assert!(
        !report["suggested_grants"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g.as_str().unwrap().contains("ON SCHEMA")),
        "an empty search_path must not yield a schema grant: {report}"
    );
    assert_eq!(
        report["suggested_search_path"],
        format!("ALTER ROLE \"{NOSCHEMA_ROLE}\" SET search_path = public")
    );

    Ok(())
}
