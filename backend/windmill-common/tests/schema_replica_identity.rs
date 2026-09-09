//! Every table must be replicable.
//!
//! PostgreSQL refuses UPDATE and DELETE on a table that has neither a PRIMARY KEY
//! nor an explicit REPLICA IDENTITY once the database is published to a logical
//! replication slot. That is what a low-downtime major-version upgrade runs on
//! (RDS and Aurora Blue/Green, pglogical) and what every CDC pipeline reads, so a
//! single keyless table blocks the upgrade outright. This runs against a freshly
//! migrated database and fails on the migration that introduces one.

use sqlx::{Pool, Postgres};

/// Partitioned parents are checked alongside ordinary tables: a parent without a
/// key hands the same defect to every partition created under it later.
#[sqlx::test(migrations = "../migrations")]
async fn every_table_is_replicable(db: Pool<Postgres>) -> anyhow::Result<()> {
    let offenders: Vec<String> = sqlx::query_scalar(
        "SELECT n.nspname || '.' || c.relname
         FROM pg_class c
         JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE c.relkind IN ('r', 'p')
           AND n.nspname NOT IN ('pg_catalog', 'information_schema')
           AND NOT EXISTS (
               SELECT 1 FROM pg_index i WHERE i.indrelid = c.oid AND i.indisprimary
           )
           AND c.relreplident = 'd'
         ORDER BY 1",
    )
    .fetch_all(&db)
    .await?;

    assert!(
        offenders.is_empty(),
        "these tables have neither a PRIMARY KEY nor an explicit REPLICA IDENTITY, \
         so logical replication will reject UPDATE and DELETE on them: {}. \
         Give each one a primary key -- a natural composite key where every column \
         is NOT NULL, otherwise a surrogate `BIGINT GENERATED ALWAYS AS IDENTITY`.",
        offenders.join(", ")
    );

    Ok(())
}
