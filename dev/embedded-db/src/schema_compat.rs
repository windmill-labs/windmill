//! Schema-compatibility probe (WIN-2130).
//!
//! Boots embedded Postgres (pglite-oxide) and applies Windmill's full migration
//! set on a single connection, then samples core tables. Proves the schema is
//! Postgres-17 compatible independent of the runtime connection-concurrency
//! blocker documented in docs/experiments/oliphaunt-lightweight-dev-mode.md.
//!
//! Uses sqlx's runtime `Migrator::new(path)` (not the compile-time macro) so it
//! can read a patched copy of the migrations with the single `uuid-ossp`
//! CREATE EXTENSION line neutralized — the same thing Windmill's own migrator
//! does via OVERRIDDEN_MIGRATIONS (windmill-api/src/db.rs).

use pglite_oxide::PgliteServer;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use std::path::PathBuf;
use std::time::Duration;

const UUID_OSSP_LINE: &str =
    "create extension if not exists \"uuid-ossp\"      with schema extensions;";

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("../../backend/migrations").canonicalize()?;
    eprintln!("[schema-compat] source migrations: {}", src.display());

    // Copy the (flat) migrations dir to a temp location and neutralize the one
    // uuid-ossp line pglite can't satisfy.
    let tmp = std::env::temp_dir().join("wm-embedded-db-migrations");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp)?;
    let mut patched = 0usize;
    for entry in std::fs::read_dir(&src)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let mut body = std::fs::read_to_string(entry.path())?;
        if body.contains(UUID_OSSP_LINE) {
            body = body.replace(
                UUID_OSSP_LINE,
                "-- uuid-ossp unavailable in pglite; neutralized (see OVERRIDDEN_MIGRATIONS)",
            );
            patched += 1;
        }
        std::fs::write(tmp.join(name), body)?;
    }
    eprintln!("[schema-compat] copied migrations to temp, patched {patched} file(s)");

    let server = PgliteServer::builder()
        .path(
            manifest
                .join(".schema-compat-pgdata")
                .to_string_lossy()
                .to_string(),
        )
        .start()?;
    let url = server.database_url();
    eprintln!("[schema-compat] embedded Postgres up: {url}");

    let pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(120))
        .connect(&url)
        .await?;

    let migrator = Migrator::new(tmp.as_path()).await?;
    eprintln!(
        "[schema-compat] applying {} migrations (single connection)...",
        migrator.migrations.len()
    );
    let t0 = std::time::Instant::now();
    match migrator.run(&pool).await {
        Ok(()) => eprintln!(
            "[schema-compat] ALL {} migrations applied OK in {:?}",
            migrator.migrations.len(),
            t0.elapsed()
        ),
        Err(e) => {
            eprintln!("[schema-compat] FAILED after {:?}: {e}", t0.elapsed());
            pool.close().await;
            server.shutdown().ok();
            std::process::exit(1);
        }
    }

    for t in ["workspace", "script", "flow", "v2_job", "usr"] {
        let q = format!("select count(*) from {t}");
        match sqlx::query_scalar::<_, i64>(&q).fetch_one(&pool).await {
            Ok(c) => eprintln!("[schema-compat]   table {t}: OK ({c} rows)"),
            Err(e) => eprintln!("[schema-compat]   table {t}: MISSING/ERR: {e}"),
        }
    }

    pool.close().await;
    server.shutdown().ok();
    Ok(())
}
