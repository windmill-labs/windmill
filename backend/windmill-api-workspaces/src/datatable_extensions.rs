/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Copying a database that uses extensions.
//!
//! A copy is replayed as the target's own connection role, which can neither create an extension
//! only a superuser may create nor comment on one it does not own, and `pg_dump` writes both for
//! every extension. So the dump leaves extensions out, and the copy is given them before the
//! replay by whoever may create them on its server ([`ExtensionPlan::prepare_target`]).
//!
//! TimescaleDB keeps a hypertable's rows in chunk tables under its own schemas and bookkeeping,
//! which a replay can neither write nor reproduce. The chunks are left out of the dump, and each
//! hypertable is made one again in the copy, its rows streamed from the source
//! ([`ExtensionPlan::finish_target`]).

use std::collections::BTreeSet;

use futures::{SinkExt, StreamExt};
use tokio_postgres::types::ToSql;
use windmill_common::error::{pg_error_message, Error, Result};
use windmill_common::workspaces::DataTableCatalogResourceType;
use windmill_common::{PgDatabase, DB};

use crate::workspaces::DumpFile;

const TIMESCALEDB: &str = "timescaledb";

/// What a copy of one database needs done about the extensions it uses.
#[derive(Default)]
pub(crate) struct ExtensionPlan {
    /// In creation order, so an extension comes after those it requires.
    extensions: Vec<SourceExtension>,
    /// Left out of the dump: every extension above.
    pub(crate) excluded_extensions: Vec<String>,
    /// Left out of the dump with everything in them: the schemas hypertable chunks live in.
    pub(crate) excluded_schemas: Vec<String>,
    /// Schemas holding an extension that the dump creates too. The copy needs them before its
    /// extensions, so they are made first and the dump's `CREATE SCHEMA` is commented out.
    extension_schemas: Vec<String>,
    hypertables: Vec<Hypertable>,
}

struct SourceExtension {
    name: String,
    schema: String,
}

struct Hypertable {
    schema: String,
    name: String,
    /// In dimension order: the first is the one `create_hypertable` takes.
    dimensions: Vec<Dimension>,
    /// The columns a `COPY` carries: every one but the generated, which the copy computes itself.
    columns: Vec<String>,
}

struct Dimension {
    column: String,
    time_interval: Option<String>,
    integer_interval: Option<i64>,
    num_partitions: Option<i16>,
}

impl Hypertable {
    fn relation(&self) -> String {
        format!("{}.{}", quote_ident(&self.schema), quote_ident(&self.name))
    }
}

fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn source_err(dbname: &str) -> impl FnOnce(tokio_postgres::Error) -> Error + '_ {
    move |e| {
        Error::internal_err(format!(
            "Failed to read the extensions of database '{dbname}': {}",
            pg_error_message(&e)
        ))
    }
}

/// Read what copying `source` takes: its extensions and, with TimescaleDB, its hypertables.
/// Refuses, before anything is created, what a copy cannot carry.
pub(crate) async fn read_extension_plan(
    db: &DB,
    source: &PgDatabase,
    schema_only: bool,
) -> Result<ExtensionPlan> {
    let (client, connection) = source.connect(Some(db)).await?;
    let join_handle = tokio::spawn(async move { connection.await });
    let plan = read_plan(&client, &source.dbname, schema_only).await;
    drop(client);
    windmill_common::shutdown_pg_connection(join_handle).await?;
    plan
}

async fn read_plan(
    client: &tokio_postgres::Client,
    dbname: &str,
    schema_only: bool,
) -> Result<ExtensionPlan> {
    // An extension's schema is dumped unless it is Postgres's own, `public`, which every database
    // already has, or one an extension created as part of itself.
    let rows = client
        .query(
            "SELECT e.extname::text, n.nspname::text,
                    n.nspname NOT IN ('public', 'pg_catalog') AND NOT EXISTS (
                        SELECT 1 FROM pg_depend d
                        WHERE d.classid = 'pg_namespace'::regclass AND d.objid = n.oid
                          AND d.deptype = 'e')
             FROM pg_extension e JOIN pg_namespace n ON n.oid = e.extnamespace
             WHERE e.extname <> 'plpgsql'
             ORDER BY e.oid",
            &[],
        )
        .await
        .map_err(source_err(dbname))?;
    let mut plan = ExtensionPlan::default();
    let mut extension_schemas = BTreeSet::new();
    for row in rows {
        let extension = SourceExtension { name: row.get(0), schema: row.get(1) };
        if row.get::<_, bool>(2) {
            extension_schemas.insert(extension.schema.clone());
        }
        plan.excluded_extensions.push(extension.name.clone());
        plan.extensions.push(extension);
    }
    plan.extension_schemas = extension_schemas.into_iter().collect();

    if !schema_only {
        refuse_extension_rows(client, dbname).await?;
    }
    if plan.extensions.iter().any(|e| e.name == TIMESCALEDB) {
        read_hypertables(client, dbname, schema_only, &mut plan).await?;
    }
    Ok(plan)
}

/// Refuse a copy with data of a database whose extensions' configuration tables hold rows of its
/// users, such as PostGIS's custom spatial references: leaving the extensions out of the dump
/// leaves those rows out too, and the copy's role could not write them anyway. TimescaleDB's are
/// its catalog, which the copy rebuilds.
async fn refuse_extension_rows(client: &tokio_postgres::Client, dbname: &str) -> Result<()> {
    // `extcondition` is the filter pg_dump applies to pick the rows that are the user's.
    let tables = client
        .query(
            "SELECT e.extname::text, c.oid::regclass::text, COALESCE(e.extcondition[i], '')
             FROM pg_extension e
             CROSS JOIN LATERAL generate_subscripts(e.extconfig, 1) AS i
             JOIN pg_class c ON c.oid = e.extconfig[i]
             WHERE c.relkind = 'r' AND e.extname <> $1
             ORDER BY 1, 2",
            &[&TIMESCALEDB],
        )
        .await
        .map_err(source_err(dbname))?;
    let mut holding = vec![];
    for table in tables {
        let (extension, relation, condition): (String, String, String) =
            (table.get(0), table.get(1), table.get(2));
        let has_rows: bool = client
            .query_one(
                &format!("SELECT EXISTS (SELECT 1 FROM {relation} {condition})"),
                &[],
            )
            .await
            .map_err(source_err(dbname))?
            .get(0);
        if has_rows {
            holding.push(format!("{relation} of {extension}"));
        }
    }
    if !holding.is_empty() {
        return Err(Error::BadRequest(format!(
            "Database '{dbname}' has rows of its own in tables of its extensions ({}), which a \
             copy with data cannot carry: copy its schema only",
            holding.join(", ")
        )));
    }
    Ok(())
}

async fn read_hypertables(
    client: &tokio_postgres::Client,
    dbname: &str,
    schema_only: bool,
    plan: &mut ExtensionPlan,
) -> Result<()> {
    // A continuous aggregate's view reads a materialization hypertable among the chunks, which the
    // dump leaves out, so its definition would fail to replay.
    let aggregates: Vec<String> = client
        .query(
            "SELECT format('%I.%I', view_schema, view_name)
             FROM timescaledb_information.continuous_aggregates ORDER BY 1",
            &[],
        )
        .await
        .map_err(source_err(dbname))?
        .iter()
        .map(|row| row.get(0))
        .collect();
    if !aggregates.is_empty() {
        return Err(Error::BadRequest(format!(
            "Database '{dbname}' has TimescaleDB continuous aggregates ({}), which a copy cannot \
             carry yet",
            aggregates.join(", ")
        )));
    }

    plan.excluded_schemas = client
        .query(
            "SELECT DISTINCT associated_schema_name::text FROM _timescaledb_catalog.hypertable
             UNION SELECT '_timescaledb_internal' ORDER BY 1",
            &[],
        )
        .await
        .map_err(source_err(dbname))?
        .iter()
        .map(|row| row.get(0))
        .collect();

    let rows = client
        .query(
            "SELECT d.hypertable_schema::text, d.hypertable_name::text, d.column_name::text,
                    d.time_interval::text, d.integer_interval::bigint, d.num_partitions::smallint
             FROM timescaledb_information.dimensions d
             ORDER BY d.hypertable_schema, d.hypertable_name, d.dimension_number",
            &[],
        )
        .await
        .map_err(source_err(dbname))?;
    for row in rows {
        let (schema, name): (String, String) = (row.get(0), row.get(1));
        let dimension = Dimension {
            column: row.get(2),
            time_interval: row.get(3),
            integer_interval: row.get(4),
            num_partitions: row.get(5),
        };
        match plan.hypertables.last_mut() {
            Some(last) if last.schema == schema && last.name == name => {
                last.dimensions.push(dimension)
            }
            _ => plan.hypertables.push(Hypertable {
                schema,
                name,
                dimensions: vec![dimension],
                columns: vec![],
            }),
        }
    }

    for hypertable in &mut plan.hypertables {
        hypertable.columns = client
            .query(
                "SELECT attname::text FROM pg_attribute
                 WHERE attrelid = $1::text::regclass AND attnum > 0 AND NOT attisdropped
                   AND attgenerated = ''
                 ORDER BY attnum",
                &[&hypertable.relation()],
            )
            .await
            .map_err(source_err(dbname))?
            .iter()
            .map(|row| row.get(0))
            .collect();
    }

    // The rows of a hypertable reach the copy after the replay, which checks a foreign key when it
    // adds it: one referencing a hypertable would find it empty.
    if !schema_only && !plan.hypertables.is_empty() {
        let relations: Vec<String> = plan.hypertables.iter().map(Hypertable::relation).collect();
        let referencing: Vec<String> = client
            .query(
                "SELECT format('%s references %s', conrelid::regclass, confrelid::regclass)
                 FROM pg_constraint
                 WHERE contype = 'f'
                   AND confrelid = ANY(SELECT r::regclass FROM unnest($1::text[]) r)
                 ORDER BY 1",
                &[&relations],
            )
            .await
            .map_err(source_err(dbname))?
            .iter()
            .map(|row| row.get(0))
            .collect();
        if !referencing.is_empty() {
            return Err(Error::BadRequest(format!(
                "Database '{dbname}' has foreign keys to TimescaleDB hypertables ({}), which a \
                 copy with data cannot check: copy its schema only",
                referencing.join(", ")
            )));
        }
    }
    Ok(())
}

impl ExtensionPlan {
    /// Give `target` the source's extensions before `dump` is replayed into it, `creator`
    /// creating each one it lacks.
    ///
    /// Authorization: none. Callers MUST have authorized writing to `target`, and pick `creator`
    /// with [`ExtensionCreator::for_copy`].
    pub(crate) async fn prepare_target(
        &self,
        db: &DB,
        target: &PgDatabase,
        creator: ExtensionCreator,
        dump: &DumpFile,
    ) -> Result<()> {
        if self.extensions.is_empty() {
            return Ok(());
        }
        let (client, connection) = target.connect(Some(db)).await?;
        let join_handle = tokio::spawn(async move { connection.await });
        let missing = self.create_schemas_and_list_missing(&client, target).await;
        drop(client);
        windmill_common::shutdown_pg_connection(join_handle).await?;
        for extension in missing? {
            create_extension(db, target, creator, extension).await?;
        }
        comment_out_schema_creations(dump, &self.extension_schemas).await
    }

    /// Made by the target's own role, so what the replay puts in them is its own.
    async fn create_schemas_and_list_missing(
        &self,
        client: &tokio_postgres::Client,
        target: &PgDatabase,
    ) -> Result<Vec<&SourceExtension>> {
        let target_err = |e: tokio_postgres::Error| {
            Error::internal_err(format!(
                "Failed to prepare database '{}' for a copy: {}",
                target.dbname,
                pg_error_message(&e)
            ))
        };
        for schema in &self.extension_schemas {
            client
                .batch_execute(&format!(
                    "CREATE SCHEMA IF NOT EXISTS {}",
                    quote_ident(schema)
                ))
                .await
                .map_err(target_err)?;
        }
        // A database made from a template carrying extensions already has them, and creating one
        // it has would still need the rights to.
        let present: BTreeSet<String> = client
            .query("SELECT extname::text FROM pg_extension", &[])
            .await
            .map_err(target_err)?
            .iter()
            .map(|row| row.get(0))
            .collect();
        Ok(self
            .extensions
            .iter()
            .filter(|e| !present.contains(&e.name))
            .collect())
    }

    /// Make each hypertable of the source one again in `target`, after the replay, and unless
    /// `schema_only` stream its rows over. A hypertable `target` already has is left as it is.
    ///
    /// Authorization: none. Callers MUST have authorized reading `source` and writing to `target`.
    pub(crate) async fn finish_target(
        &self,
        db: &DB,
        source: &PgDatabase,
        target: &PgDatabase,
        schema_only: bool,
    ) -> Result<()> {
        if self.hypertables.is_empty() {
            return Ok(());
        }
        let (source_client, source_connection) = source.connect(Some(db)).await?;
        let source_join = tokio::spawn(async move { source_connection.await });
        let (mut target_client, target_connection) = target.connect(Some(db)).await?;
        let target_join = tokio::spawn(async move { target_connection.await });
        let result = self
            .restore_hypertables(&source_client, &mut target_client, schema_only)
            .await;
        drop(source_client);
        drop(target_client);
        windmill_common::shutdown_pg_connection(source_join).await?;
        windmill_common::shutdown_pg_connection(target_join).await?;
        result.map_err(|e| {
            Error::internal_err(format!(
                "Failed to restore the hypertables of '{}' into '{}': {e}",
                source.dbname, target.dbname
            ))
        })
    }

    async fn restore_hypertables(
        &self,
        source: &tokio_postgres::Client,
        target: &mut tokio_postgres::Client,
        schema_only: bool,
    ) -> std::result::Result<(), String> {
        let pg = |e: tokio_postgres::Error| pg_error_message(&e);
        let tx = target.transaction().await.map_err(pg)?;
        for hypertable in &self.hypertables {
            let relation = hypertable.relation();
            let exists: bool = tx
                .query_one(
                    "SELECT EXISTS (SELECT 1 FROM timescaledb_information.hypertables
                     WHERE hypertable_schema = $1 AND hypertable_name = $2)",
                    &[&hypertable.schema, &hypertable.name],
                )
                .await
                .map_err(pg)?
                .get(0);
            if exists {
                continue;
            }
            for (i, dimension) in hypertable.dimensions.iter().enumerate() {
                // The replay already made the source's indexes, the default ones among them.
                let (function, extra) = if i == 0 {
                    ("create_hypertable", ", create_default_indexes => false")
                } else {
                    ("add_dimension", "")
                };
                let partitions = dimension.num_partitions.map(i32::from);
                let (argument, value): (&str, Option<&(dyn ToSql + Sync)>) = match (
                    &dimension.time_interval,
                    &dimension.integer_interval,
                    &partitions,
                ) {
                    (Some(interval), _, _) => (
                        ", chunk_time_interval => $3::text::interval",
                        Some(interval),
                    ),
                    (None, Some(interval), _) => {
                        (", chunk_time_interval => $3::bigint", Some(interval))
                    }
                    (None, None, Some(partitions)) => {
                        (", number_partitions => $3::int", Some(partitions))
                    }
                    (None, None, None) => ("", None),
                };
                let mut params: Vec<&(dyn ToSql + Sync)> = vec![&relation, &dimension.column];
                params.extend(value);
                tx.execute(
                    &format!(
                        "SELECT {function}($1::text::regclass, $2::text::name{argument}{extra})"
                    ),
                    &params,
                )
                .await
                .map_err(|e| format!("{relation}: {}", pg_error_message(&e)))?;
            }
            if !schema_only {
                copy_rows(source, &tx, hypertable)
                    .await
                    .map_err(|e| format!("copying the rows of {relation}: {e}"))?;
            }
        }
        tx.commit().await.map_err(pg)
    }
}

/// Stream the rows of `hypertable` from `source` into `target`. A `COPY` of a hypertable itself
/// reads none of its rows, which live in its chunks: the source side copies a query instead.
async fn copy_rows(
    source: &tokio_postgres::Client,
    target: &tokio_postgres::Transaction<'_>,
    hypertable: &Hypertable,
) -> std::result::Result<(), String> {
    let pg = |e: tokio_postgres::Error| pg_error_message(&e);
    let relation = hypertable.relation();
    let columns = hypertable
        .columns
        .iter()
        .map(|c| quote_ident(c))
        .collect::<Vec<_>>()
        .join(", ");
    let rows = source
        .copy_out(&format!(
            "COPY (SELECT {columns} FROM {relation}) TO STDOUT"
        ))
        .await
        .map_err(pg)?;
    let sink = target
        .copy_in(&format!("COPY {relation} ({columns}) FROM STDIN"))
        .await
        .map_err(pg)?;
    futures::pin_mut!(rows, sink);
    while let Some(chunk) = rows.next().await {
        sink.send(chunk.map_err(pg)?).await.map_err(pg)?;
    }
    sink.finish().await.map_err(pg)?;
    Ok(())
}

/// Who creates the extensions a copy lacks.
#[derive(Clone, Copy)]
pub(crate) enum ExtensionCreator {
    /// The target's own connection role, which may create only trusted extensions.
    TargetRole,
    /// The administrator of the cluster Windmill manages the target on.
    ClusterAdmin(DataTableCatalogResourceType),
}

impl ExtensionCreator {
    /// The source names the extensions created, so the administrator creates them only for a
    /// fresh copy of a database on its own cluster, whose extensions an administrator there
    /// installed: any other source could name an untrusted extension, and a live target's role
    /// may have planted objects in the schemas an extension script runs against.
    pub(crate) fn for_copy(
        source: Option<DataTableCatalogResourceType>,
        target: Option<DataTableCatalogResourceType>,
        fresh_copy: bool,
    ) -> Self {
        match (source, target) {
            (Some(source), Some(target))
                if source == target && fresh_copy && target.is_windmill_managed() =>
            {
                Self::ClusterAdmin(target)
            }
            _ => Self::TargetRole,
        }
    }
}

async fn create_extension(
    db: &DB,
    target: &PgDatabase,
    creator: ExtensionCreator,
    extension: &SourceExtension,
) -> Result<()> {
    let sql = format!(
        "CREATE EXTENSION IF NOT EXISTS {} WITH SCHEMA {} CASCADE",
        quote_ident(&extension.name),
        quote_ident(&extension.schema)
    );
    let name = &extension.name;
    let kind = match creator {
        ExtensionCreator::ClusterAdmin(kind) => kind,
        ExtensionCreator::TargetRole => DataTableCatalogResourceType::Postgresql,
    };
    match kind {
        DataTableCatalogResourceType::ExternalInstance => {
            windmill_common::external_instance_pg::execute_as_external_instance_admin_unchecked(
                db,
                &target.dbname,
                &sql,
            )
            .await
            .map_err(|e| {
                Error::BadRequest(format!(
                    "Could not create extension '{name}' in the copy as the external instance \
                     cluster's admin login: {e}"
                ))
            })
        }
        DataTableCatalogResourceType::Instance => {
            let windmill =
                PgDatabase::parse_uri(&windmill_common::get_database_url().await?.as_str().await)?;
            let admin = PgDatabase { dbname: target.dbname.clone(), ..windmill };
            execute(db, &admin, &sql).await.map_err(|e| {
                Error::BadRequest(format!(
                    "Could not create extension '{name}' in the copy as Windmill's database user: \
                     {e}"
                ))
            })
        }
        DataTableCatalogResourceType::Postgresql => execute(db, target, &sql).await.map_err(|e| {
            Error::BadRequest(format!(
                "The copy needs extension '{name}', which its new database does not have and \
                     role '{}' may not create: {}. Install the extension in the server's \
                     template1 database, which new databases are made from, or let the role \
                     create it.",
                target.login_name(),
                e.trim_end_matches('.')
            ))
        }),
    }
}

async fn execute(db: &DB, pg: &PgDatabase, sql: &str) -> std::result::Result<(), String> {
    let (client, connection) = pg.connect(Some(db)).await.map_err(|e| e.to_string())?;
    let join_handle = tokio::spawn(async move { connection.await });
    let result = client
        .batch_execute(sql)
        .await
        .map_err(|e| pg_error_message(&e));
    drop(client);
    if let Err(e) = windmill_common::shutdown_pg_connection(join_handle).await {
        tracing::warn!("Closing the connection to '{}': {e}", pg.dbname);
    }
    result
}

/// The schema a `CREATE SCHEMA <name>;` line of a dump creates, unquoted.
fn created_schema(line: &str) -> Option<String> {
    let name = line
        .strip_prefix("CREATE SCHEMA ")?
        .trim_end()
        .strip_suffix(';')?;
    match name.strip_prefix('"').and_then(|n| n.strip_suffix('"')) {
        Some(quoted) => Some(quoted.replace("\"\"", "\"")),
        None => Some(name.to_string()),
    }
}

/// Comment out the dump's `CREATE SCHEMA` of each of `schemas`, which the copy already has, in
/// place and three bytes each so the rest of the dump stays where it is. pg_dump writes every
/// schema before any other object, so the scan stops at the first other `CREATE` or `COPY`.
async fn comment_out_schema_creations(dump: &DumpFile, schemas: &[String]) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncSeekExt, AsyncWriteExt};

    if schemas.is_empty() {
        return Ok(());
    }
    let io_err = |e: std::io::Error| Error::internal_err(format!("Failed to patch dump file: {e}"));
    let file = tokio::fs::File::open(&dump.path).await.map_err(io_err)?;
    let mut reader = tokio::io::BufReader::new(file);
    let mut offsets = vec![];
    let mut offset = 0u64;
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line).await.map_err(io_err)?;
        if read == 0 {
            break;
        }
        if let Some(schema) = created_schema(&line) {
            if schemas.contains(&schema) {
                offsets.push(offset);
            }
        } else if line.starts_with("CREATE ") || line.starts_with("COPY ") {
            break;
        }
        offset += read as u64;
    }
    if offsets.is_empty() {
        return Ok(());
    }
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .open(&dump.path)
        .await
        .map_err(io_err)?;
    for offset in offsets {
        file.seek(std::io::SeekFrom::Start(offset))
            .await
            .map_err(io_err)?;
        file.write_all(b"-- ").await.map_err(io_err)?;
    }
    file.flush().await.map_err(io_err)
}

#[cfg(test)]
mod tests {
    use super::created_schema;

    #[test]
    fn created_schema_unquotes_what_pg_dump_quotes() {
        assert_eq!(
            created_schema("CREATE SCHEMA extensions;\n").as_deref(),
            Some("extensions")
        );
        assert_eq!(
            created_schema("CREATE SCHEMA \"My \"\"Ext\"\"\";").as_deref(),
            Some("My \"Ext\"")
        );
        assert_eq!(created_schema("CREATE TABLE public.t ("), None);
    }
}
