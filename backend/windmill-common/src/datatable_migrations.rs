use crate::error::{Error, Result};
use crate::{PgDatabase, DB};
use serde::{Deserialize, Serialize};

/// Name of the table created inside a datatable's own database to track which
/// migrations have been applied. This mirrors how tools like sqlx/flyway track
/// migration state inside the target database, which makes the applied state
/// survive forks (the table is copied along with the rest of the database).
pub const TRACKING_TABLE: &str = "_windmill_datatable_migrations";

/// A migration to potentially apply against a datatable database.
#[derive(Debug, Clone)]
pub struct MigrationToApply {
    pub version: String,
    pub name: String,
    pub content: String,
    pub checksum: String,
}

/// A migration recorded as applied inside the datatable database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedMigration {
    pub version: String,
    pub name: String,
    pub checksum: String,
}

const CREATE_TRACKING_TABLE: &str = "CREATE TABLE IF NOT EXISTS _windmill_datatable_migrations (\
    version text PRIMARY KEY, \
    name text NOT NULL DEFAULT '', \
    checksum text NOT NULL DEFAULT '', \
    applied_at timestamptz NOT NULL DEFAULT now())";

/// Connect to the datatable database and ensure the tracking table exists, then
/// run `f` with the connected client. Cleans up the connection task afterwards.
async fn with_client<F, Fut, T>(pg: &PgDatabase, main_db: Option<&DB>, f: F) -> Result<T>
where
    F: FnOnce(tokio_postgres::Client) -> Fut,
    Fut: std::future::Future<Output = Result<(tokio_postgres::Client, T)>>,
{
    let (client, connection) = pg.connect(main_db).await?;
    let join_handle = tokio::spawn(async move { connection.await });

    client
        .batch_execute(CREATE_TRACKING_TABLE)
        .await
        .map_err(|e| {
            Error::internal_err(format!("Failed to ensure migration tracking table: {e}"))
        })?;

    let (client, result) = f(client).await?;

    drop(client);
    join_handle
        .await
        .map_err(|e| Error::internal_err(format!("join error: {e}")))?
        .map_err(|e| Error::internal_err(format!("tokio_postgres error: {e}")))?;

    Ok(result)
}

/// List the migrations recorded as applied inside the datatable database.
pub async fn list_applied_migrations(
    pg: &PgDatabase,
    main_db: Option<&DB>,
) -> Result<Vec<AppliedMigration>> {
    with_client(pg, main_db, |client| async move {
        let rows = client
            .query(
                "SELECT version, name, checksum FROM _windmill_datatable_migrations ORDER BY version",
                &[],
            )
            .await
            .map_err(|e| Error::internal_err(format!("Failed to list applied migrations: {e}")))?;
        let applied = rows
            .into_iter()
            .map(|r| AppliedMigration {
                version: r.get::<_, String>("version"),
                name: r.get::<_, Option<String>>("name").unwrap_or_default(),
                checksum: r.get::<_, Option<String>>("checksum").unwrap_or_default(),
            })
            .collect();
        Ok((client, applied))
    })
    .await
}

/// Apply all migrations in `migrations` (assumed already sorted by version) that
/// have not yet been applied to the datatable database, in order. Each migration
/// runs in its own transaction together with its tracking-table insert, so a
/// migration is either fully applied and recorded or not at all.
///
/// Migration SQL must not contain its own explicit transaction control
/// (BEGIN/COMMIT), as each migration is already wrapped in a transaction.
///
/// Returns the versions that were newly applied by this call.
pub async fn apply_pending_migrations(
    pg: &PgDatabase,
    migrations: &[MigrationToApply],
    main_db: Option<&DB>,
) -> Result<Vec<String>> {
    let migrations = migrations.to_vec();
    with_client(pg, main_db, |mut client| async move {
        let applied_rows = client
            .query("SELECT version FROM _windmill_datatable_migrations", &[])
            .await
            .map_err(|e| Error::internal_err(format!("Failed to read applied migrations: {e}")))?;
        let applied: std::collections::HashSet<String> = applied_rows
            .into_iter()
            .map(|r| r.get::<_, String>("version"))
            .collect();

        let mut newly_applied = Vec::new();
        for m in migrations.iter() {
            if applied.contains(&m.version) {
                continue;
            }
            let tx = client
                .transaction()
                .await
                .map_err(|e| Error::internal_err(format!("Failed to start transaction: {e}")))?;
            tx.batch_execute(&m.content).await.map_err(|e| {
                Error::internal_err(format!("Migration {} ({}) failed: {e}", m.version, m.name))
            })?;
            tx.execute(
                "INSERT INTO _windmill_datatable_migrations (version, name, checksum) VALUES ($1, $2, $3)",
                &[&m.version, &m.name, &m.checksum],
            )
            .await
            .map_err(|e| {
                Error::internal_err(format!("Failed to record migration {}: {e}", m.version))
            })?;
            tx.commit().await.map_err(|e| {
                Error::internal_err(format!("Failed to commit migration {}: {e}", m.version))
            })?;
            newly_applied.push(m.version.clone());
        }
        Ok((client, newly_applied))
    })
    .await
}
