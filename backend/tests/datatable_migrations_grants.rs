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

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

const ROLE: &str = "wm_dtmig_test_role";
const ROLE_PASSWORD: &str = "wm_dtmig_test_pwd";

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
        // Roles are cluster objects, not per-test-database ones, so this both
        // survives re-runs and tolerates a concurrent test creating it first.
        "DO $$ BEGIN \
           CREATE ROLE {ROLE} LOGIN PASSWORD '{ROLE_PASSWORD}'; \
         EXCEPTION WHEN duplicate_object THEN NULL; \
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
