//! A migration that has already run may still gain the down it was missing.
//!
//! Rewriting an applied migration is refused because its `_wm_migrations` record
//! would no longer match its SQL. Filling in an absent `code_down` is the one
//! edit that keeps that record true — and the only way to make an already-run
//! migration revertable — so it must stay allowed while every other edit stays
//! refused.

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

const ROLE: &str = "wm_dtmig_down_role";
const ROLE_PASSWORD: &str = "wm_dtmig_down_pwd";
const VERSION: i64 = 20260101000000;
const CODE_UP: &str = "CREATE TABLE widgets (id int);";
const CODE_DOWN: &str = "DROP TABLE widgets;";

fn authed(b: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    b.header("Authorization", "Bearer DTMIG_ADMIN_TOKEN")
}

/// Point the fixture's data table at this test's own database and put it in the
/// state that matters: one migration defined without a down, recorded as applied.
async fn setup_applied_migration_without_down(db: &Pool<Postgres>) -> anyhow::Result<()> {
    let opts = (*db.connect_options()).clone();
    let dbname = opts.get_database().expect("test database name").to_string();

    sqlx::query(&format!(
        // Roles are cluster objects, so a leftover role or a parallel test
        // session reaching here at the same time must not fail the setup.
        "DO $$ BEGIN \
           CREATE ROLE {ROLE} LOGIN PASSWORD '{ROLE_PASSWORD}'; \
         EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL; \
         END $$"
    ))
    .execute(db)
    .await?;
    sqlx::raw_sql(&format!(
        "GRANT CONNECT ON DATABASE \"{dbname}\" TO {ROLE}; \
         GRANT USAGE ON SCHEMA public TO {ROLE}; \
         CREATE TABLE _wm_migrations ( \
            datatable TEXT NOT NULL, \
            version BIGINT NOT NULL, \
            installed_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
            PRIMARY KEY (datatable, version)); \
         GRANT SELECT ON _wm_migrations TO {ROLE}; \
         INSERT INTO _wm_migrations (datatable, version) VALUES ('main', {VERSION});"
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

    sqlx::query(
        "INSERT INTO datatable_migrations (workspace_id, datatable, timestamp, name, code_up) \
         VALUES ('dtmig-ws', 'main', $1, 'create_widgets', $2)",
    )
    .bind(VERSION)
    .bind(CODE_UP)
    .execute(db)
    .await?;

    Ok(())
}

#[sqlx::test(fixtures("datatable_migrations_grants"))]
async fn test_add_down_to_an_applied_migration(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    setup_applied_migration_without_down(&db).await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let url = format!(
        "http://localhost:{port}/api/w/dtmig-ws/workspaces/upsert_datatable_migration/main"
    );
    let upsert = |code_up: &str, code_down: &str| {
        authed(reqwest::Client::new().post(&url)).json(&json!({
            "timestamp": VERSION,
            "name": "create_widgets",
            "code_up": code_up,
            "code_down": code_down,
        }))
    };

    let resp = upsert(CODE_UP, CODE_DOWN).send().await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 200,
        "adding a missing down to an applied migration should be allowed: {body}"
    );
    let stored = sqlx::query_scalar::<_, Option<String>>(
        "SELECT code_down FROM datatable_migrations \
         WHERE workspace_id = 'dtmig-ws' AND datatable = 'main' AND timestamp = $1",
    )
    .bind(VERSION)
    .fetch_one(&db)
    .await?;
    assert_eq!(stored.as_deref(), Some(CODE_DOWN));

    // The up it ran is what the `_wm_migrations` record stands for: still frozen.
    let resp = upsert("CREATE TABLE gadgets (id int);", CODE_DOWN)
        .send()
        .await?;
    assert_eq!(resp.status(), 400);
    assert!(
        resp.text().await?.contains("has already been applied"),
        "rewriting the up of an applied migration must stay refused"
    );

    // And so is a down that has already been recorded — only the absent-to-present
    // step is exempt, in that direction alone.
    let resp = upsert(CODE_UP, "DROP TABLE widgets CASCADE;")
        .send()
        .await?;
    assert_eq!(resp.status(), 400);

    let resp = authed(reqwest::Client::new().post(&url))
        .json(&json!({ "timestamp": VERSION, "name": "create_widgets", "code_up": CODE_UP }))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        400,
        "dropping the down of an applied migration must stay refused"
    );

    Ok(())
}
