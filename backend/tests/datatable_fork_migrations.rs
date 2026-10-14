//! A fork's copy of a data table must report the migrations its schema already holds as applied.
//!
//! A schema-only copy restores `_wm_migrations` without its rows, and no copy carries the source's
//! `ALTER DATABASE ... SET search_path`, through which the bookkeeping table is found. Either one
//! alone makes the fork list every applied migration as pending, and running them fails on
//! objects that already exist.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

const ROLE: &str = "wm_dtfork_test_role";
const ROLE_PASSWORD: &str = "wm_dtfork_test_pwd";

fn authed(b: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    b.header("Authorization", "Bearer DTMIG_ADMIN_TOKEN")
}

#[sqlx::test(fixtures("datatable_migrations_grants"))]
async fn a_forked_copy_keeps_its_applied_migrations(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let opts = (*db.connect_options()).clone();
    // Databases are cluster-wide, so both are named for this run alone.
    let suffix = format!(
        "{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    let source = format!("wm_dtfork_src_{suffix}");
    let fork_id = format!("wm-fork-dtfork-{suffix}");
    let copy = format!("wm_fork_dtfork_{suffix}__main");

    sqlx::query(&format!(
        "DO $$ BEGIN \
           CREATE ROLE {ROLE} LOGIN CREATEDB PASSWORD '{ROLE_PASSWORD}'; \
         EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL; \
         END $$"
    ))
    .execute(&db)
    .await?;
    sqlx::query(&format!("CREATE DATABASE {source} OWNER {ROLE}"))
        .execute(&db)
        .await?;

    let result = async {
        let source_pool = sqlx::PgPool::connect_with(opts.clone().database(&source)).await?;
        sqlx::raw_sql(&format!(
            "SET ROLE {ROLE}; \
             CREATE SCHEMA app; \
             CREATE TABLE app.foo (id int); \
             CREATE TABLE app._wm_migrations (datatable TEXT NOT NULL, version BIGINT NOT NULL, \
                 installed_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY (datatable, version)); \
             INSERT INTO app._wm_migrations (datatable, version) VALUES ('main', 1); \
             ALTER DATABASE {source} SET search_path = app;"
        ))
        .execute(&source_pool)
        .await?;
        source_pool.close().await;

        sqlx::query(
            "INSERT INTO resource (workspace_id, path, value, resource_type, created_by) \
             VALUES ('dtmig-ws', 'u/dtmig-admin/pg', $1, 'postgresql', 'dtmig-admin')",
        )
        .bind(json!({
            "host": opts.get_host(),
            "port": opts.get_port(),
            "dbname": source,
            "user": ROLE,
            "password": ROLE_PASSWORD,
            "sslmode": "disable",
        }))
        .execute(&db)
        .await?;
        sqlx::query(
            "INSERT INTO datatable_migrations (workspace_id, datatable, timestamp, name, code_up) \
             VALUES ('dtmig-ws', 'main', 1, 'create_foo', 'CREATE TABLE foo (id int)')",
        )
        .execute(&db)
        .await?;

        let server = ApiServer::start(db.clone()).await?;
        let base = format!("http://localhost:{}/api", server.addr.port());
        let client = reqwest::Client::new();

        let resp = authed(client.post(format!("{base}/w/dtmig-ws/workspaces/create_fork")))
            .json(&json!({
                "id": fork_id,
                "name": "fork",
                "forked_datatables": [
                    {"name": "main", "new_dbname": copy, "fork_behavior": "schema_only"}
                ],
            }))
            .send()
            .await?;
        // `ensure!` rather than `assert!`: a panic would skip dropping the databases below.
        let fork_status = resp.status();
        anyhow::ensure!(fork_status == 200, "{}", resp.text().await?);

        let status: Value = authed(client.get(format!(
            "{base}/w/{fork_id}/workspaces/datatable_migrations_status/main"
        )))
        .send()
        .await?
        .json()
        .await?;
        anyhow::ensure!(status["migrations"][0]["status"] == "ran", "{status}");
        anyhow::Ok(())
    }
    .await;

    for name in [&copy, &source] {
        sqlx::query(&format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)"))
            .execute(&db)
            .await?;
    }
    result
}
