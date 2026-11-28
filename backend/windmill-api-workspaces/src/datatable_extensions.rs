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
//! replay by an [`ExtensionCreator`].
//!
//! TimescaleDB keeps a hypertable's rows in chunk tables and bookkeeping of its own, which a replay
//! can neither write nor reproduce. The chunks are left out of the dump and each hypertable is made
//! one again in the copy. Its rows are streamed from the source between the dump's data and its
//! post-data section, where pg_dump would load them: before the triggers that would fire on them
//! and the materialized views refreshed from them, and in the dump's own snapshot.

use std::collections::BTreeSet;
use std::io::Write;

use futures::{SinkExt, StreamExt};
use tokio_postgres::types::ToSql;
use windmill_common::error::{pg_error_message, Error, Result};
use windmill_common::workspaces::DataTableCatalogResourceType;
use windmill_common::{PgDatabase, DB};

use crate::workspaces::{
    pg_dump_database, pg_import_dump, pg_restore_section, DumpFile, PgDumpOptions,
};

const TIMESCALEDB: &str = "timescaledb";
/// Where TimescaleDB keeps the relations it makes for itself, compressed chunks among them.
const TIMESCALEDB_INTERNAL: &str = "_timescaledb_internal";

/// What a copy of one database needs done about the extensions it uses.
#[derive(Default)]
pub(crate) struct ExtensionPlan {
    /// In creation order, so an extension comes after those it requires.
    extensions: Vec<SourceExtension>,
    /// Schemas holding an extension that the dump creates too. The copy needs them before its
    /// extensions, so they are made first and the dump's `CREATE SCHEMA` is commented out.
    extension_schemas: Vec<String>,
    hypertables: Vec<Hypertable>,
    /// Every chunk of every hypertable, `"schema"."table"` as pg_dump patterns read them.
    chunks: Vec<String>,
    /// With TimescaleDB, the session the hypertables and chunks were read in, and the snapshot it
    /// exported for the dump.
    snapshot: Option<(SnapshotSession, String)>,
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
    let read = async {
        let plan = read_plan(&client, &source.dbname, schema_only).await?;
        let hypertables: Vec<String> = if plan.extensions.iter().any(|e| e.name == TIMESCALEDB) {
            client
                .query(
                    "SELECT format('%I.%I', hypertable_schema, hypertable_name)
                     FROM timescaledb_information.hypertables ORDER BY 1",
                    &[],
                )
                .await
                .map_err(source_err(&source.dbname))?
                .iter()
                .map(|row| row.get(0))
                .collect()
        } else {
            vec![]
        };
        Ok::<_, Error>((plan, hypertables))
    }
    .await;
    drop(client);
    windmill_common::shutdown_pg_connection(join_handle).await?;
    let (mut plan, hypertables) = read?;
    // The chunks to leave out are read in the snapshot the dump is then taken in: a chunk made in
    // between would otherwise be dumped as a table of its own, inheriting from its hypertable.
    if plan.extensions.iter().any(|e| e.name == TIMESCALEDB) {
        let (snapshot, snapshot_id) = SnapshotSession::open(db, source, &hypertables).await?;
        if let Err(e) = read_hypertables(&snapshot.client, &source.dbname, &mut plan).await {
            snapshot.close().await;
            return Err(e);
        }
        plan.snapshot = Some((snapshot, snapshot_id));
    }
    Ok(plan)
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
        if row.get::<_, bool>(2) {
            extension_schemas.insert(row.get::<_, String>(1));
        }
        plan.extensions
            .push(SourceExtension { name: row.get(0), schema: row.get(1) });
    }
    plan.extension_schemas = extension_schemas.into_iter().collect();

    if !schema_only {
        refuse_extension_rows(client, dbname).await?;
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
    plan: &mut ExtensionPlan,
) -> Result<()> {
    // A continuous aggregate's view reads a materialization hypertable among the relations the
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

    // A chunk inherits from its hypertable, internal compressed ones included, in every
    // TimescaleDB version, while its catalog's columns differ between them.
    plan.chunks = client
        .query(
            "SELECT n.nspname::text, c.relname::text
             FROM pg_inherits i
             JOIN pg_class c ON c.oid = i.inhrelid
             JOIN pg_namespace n ON n.oid = c.relnamespace
             WHERE i.inhparent IN (
                 SELECT format('%I.%I', schema_name, table_name)::regclass
                 FROM _timescaledb_catalog.hypertable)
             ORDER BY 1, 2",
            &[],
        )
        .await
        .map_err(source_err(dbname))?
        .iter()
        .map(|row| {
            format!(
                "{}.{}",
                quote_ident(row.get::<_, &str>(0)),
                quote_ident(row.get::<_, &str>(1))
            )
        })
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
    /// For a copy of a database of `kind` into a database the caller has just created on the same
    /// server, before anyone could have written to it. The source names the extensions created,
    /// so the administrator creates them only there: a source elsewhere could name an untrusted
    /// extension, and a database its role could already write to may hold objects that an
    /// extension script running as the administrator would pick up.
    pub(crate) fn for_new_copy_of(kind: DataTableCatalogResourceType) -> Self {
        if kind.is_windmill_managed() {
            Self::ClusterAdmin(kind)
        } else {
            Self::TargetRole
        }
    }
}

/// A dump of a source, made by [`ExtensionPlan::dump`] and replayed by
/// [`ExtensionPlan::restore`].
pub(crate) enum PlannedDump {
    Script(DumpFile),
    /// A copy with data of hypertables, whose rows load between `data` and `post_data` through
    /// `snapshot`, the session whose snapshot the dump was taken in.
    Sections {
        pre_data: DumpFile,
        data: DumpFile,
        post_data: DumpFile,
        snapshot: SnapshotSession,
    },
}

/// A session holding open the transaction whose snapshot a dump was taken in.
pub(crate) struct SnapshotSession {
    client: tokio_postgres::Client,
    join_handle: tokio::task::JoinHandle<std::result::Result<(), tokio_postgres::Error>>,
}

impl SnapshotSession {
    /// `hypertables` are quoted qualified names, locked with their chunks before the snapshot is
    /// taken and until the session closes: a chunk dropped meanwhile, by a retention policy say,
    /// takes its rows with it even from a snapshot that still sees them.
    async fn open(db: &DB, source: &PgDatabase, hypertables: &[String]) -> Result<(Self, String)> {
        let (client, connection) = source.connect(Some(db)).await?;
        let join_handle = tokio::spawn(async move { connection.await });
        let session = Self { client, join_handle };
        let snapshot = async {
            // The plan's intervals are read as text and handed back to TimescaleDB on the copy.
            session
                .client
                .batch_execute(
                    "BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY;
                     SET LOCAL IntervalStyle = 'postgres'",
                )
                .await?;
            if !hypertables.is_empty() {
                // Before any query, which would take the snapshot.
                session
                    .client
                    .batch_execute(&format!(
                        "LOCK TABLE {} IN ACCESS SHARE MODE",
                        hypertables.join(", ")
                    ))
                    .await?;
            }
            Ok::<String, tokio_postgres::Error>(
                session
                    .client
                    .query_one("SELECT pg_export_snapshot()", &[])
                    .await?
                    .get(0),
            )
        }
        .await;
        match snapshot {
            Ok(snapshot) => Ok((session, snapshot)),
            Err(e) => {
                session.close().await;
                Err(Error::internal_err(format!(
                    "Failed to take a snapshot of database '{}': {}",
                    source.dbname,
                    pg_error_message(&e)
                )))
            }
        }
    }

    async fn close(self) {
        drop(self.client);
        if let Err(e) = windmill_common::shutdown_pg_connection(self.join_handle).await {
            tracing::warn!("Closing a snapshot session: {e}");
        }
    }
}

impl ExtensionPlan {
    /// The `--filter` lines leaving out what the copy gets otherwise: every extension, and every
    /// relation TimescaleDB makes for itself.
    fn filter_lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .extensions
            .iter()
            .map(|e| format!("exclude extension {}", quote_ident(&e.name)))
            .collect();
        if self.extensions.iter().any(|e| e.name == TIMESCALEDB) {
            lines.push(format!(
                "exclude schema {}",
                quote_ident(TIMESCALEDB_INTERNAL)
            ));
        }
        lines.extend(
            self.chunks
                .iter()
                .map(|chunk| format!("exclude table {chunk}")),
        );
        lines
    }

    /// Dump `source`, the database this plan was read from, in the snapshot it was read in when it
    /// has one. `no_acl` as in [`PgDumpOptions`]; ownership never carries over, as the restore runs
    /// as the target's connection user.
    pub(crate) async fn dump(
        &mut self,
        source: &PgDatabase,
        schema_only: bool,
        no_acl: bool,
    ) -> Result<PlannedDump> {
        let lines = self.filter_lines();
        let filter = if lines.is_empty() {
            None
        } else {
            let file = DumpFile::new()?;
            std::fs::File::create(&file.path)
                .and_then(|mut f| f.write_all(lines.join("\n").as_bytes()))
                .map_err(|e| Error::internal_err(format!("Failed to write dump filter: {e}")))?;
            Some(file)
        };
        let filter_path = filter.as_ref().map(|f| f.path.as_path());

        let (snapshot, snapshot_id) = match self.snapshot.take() {
            Some((session, id)) => (Some(session), Some(id)),
            None => (None, None),
        };
        if schema_only || self.hypertables.is_empty() {
            let dump = pg_dump_database(
                source,
                PgDumpOptions {
                    schema_only,
                    no_owner: true,
                    no_acl,
                    filter: filter_path,
                    snapshot: snapshot_id.as_deref(),
                    ..Default::default()
                },
            )
            .await;
            if let Some(snapshot) = snapshot {
                snapshot.close().await;
            }
            return Ok(PlannedDump::Script(dump?));
        }

        let (Some(snapshot), Some(snapshot_id)) = (snapshot, snapshot_id) else {
            return Err(Error::internal_err(
                "the hypertables of a copy were read outside a snapshot",
            ));
        };
        let sections = async {
            let archive = pg_dump_database(
                source,
                PgDumpOptions {
                    archive: true,
                    filter: filter_path,
                    snapshot: Some(&snapshot_id),
                    ..Default::default()
                },
            )
            .await?;
            Ok::<_, Error>((
                pg_restore_section(&archive, "pre-data", true, no_acl).await?,
                pg_restore_section(&archive, "data", true, no_acl).await?,
                pg_restore_section(&archive, "post-data", true, no_acl).await?,
            ))
        }
        .await;
        match sections {
            Ok((pre_data, data, post_data)) => {
                Ok(PlannedDump::Sections { pre_data, data, post_data, snapshot })
            }
            Err(e) => {
                snapshot.close().await;
                Err(e)
            }
        }
    }

    /// Replay `dump` into `target`: its extensions first, created by `creator`, then the dump,
    /// with each hypertable made one again and, in a copy with data, its rows loaded where pg_dump
    /// would load them. A copy with data of hypertables replays in several transactions, so one
    /// that fails part way leaves `target` partly filled.
    ///
    /// Authorization: none. Callers MUST have authorized writing to `target`, and pick `creator`
    /// as [`ExtensionCreator::for_new_copy_of`] says.
    pub(crate) async fn restore(
        &self,
        db: &DB,
        dump: PlannedDump,
        target: &PgDatabase,
        creator: ExtensionCreator,
    ) -> Result<()> {
        match dump {
            PlannedDump::Script(script) => {
                self.prepare_target(db, target, creator, &script).await?;
                // Replayed before the hypertables are made again, so `ONLY` still applies.
                self.patch_for_hypertables(&script, false).await?;
                pg_import_dump(target, &script).await?;
                self.make_hypertables(db, target, None).await
            }
            PlannedDump::Sections { pre_data, data, post_data, snapshot } => {
                let restored = async {
                    self.prepare_target(db, target, creator, &pre_data).await?;
                    pg_import_dump(target, &pre_data).await?;
                    self.make_hypertables(db, target, Some(&snapshot.client))
                        .await?;
                    pg_import_dump(target, &data).await?;
                    self.patch_for_hypertables(&post_data, true).await?;
                    pg_import_dump(target, &post_data).await
                }
                .await;
                snapshot.close().await;
                restored
            }
        }
    }

    /// Make `script`'s statements on the source's hypertables replayable on the copy's:
    /// - `ts_insert_blocker`, the trigger older TimescaleDB versions put on a hypertable, is made
    ///   by `create_hypertable` too, so the script's would already exist;
    /// - with `drop_only`, for a script replayed once they are hypertables: TimescaleDB refuses
    ///   `ALTER TABLE ONLY` on one, which pg_dump adds constraints with. Without `ONLY` the
    ///   constraint goes on the hypertable, and TimescaleDB gives each chunk its own.
    async fn patch_for_hypertables(&self, script: &DumpFile, drop_only: bool) -> Result<()> {
        if self.hypertables.is_empty() {
            return Ok(());
        }
        let io_err =
            |e: std::io::Error| Error::internal_err(format!("Failed to patch dump file: {e}"));
        let text = tokio::fs::read_to_string(&script.path)
            .await
            .map_err(io_err)?;
        let hypertables: BTreeSet<(&str, &str)> = self
            .hypertables
            .iter()
            .map(|h| (h.schema.as_str(), h.name.as_str()))
            .collect();
        let mut patched = String::with_capacity(text.len());
        for line in text.split_inclusive('\n') {
            if line.starts_with("CREATE TRIGGER ts_insert_blocker ") {
                patched.push_str("-- ");
                patched.push_str(line);
                continue;
            }
            let only_on_hypertable = drop_only
                && line
                    .trim_end()
                    .strip_prefix("ALTER TABLE ONLY ")
                    .and_then(parse_qualified)
                    .is_some_and(|(schema, name)| hypertables.contains(&(&schema, &name)));
            if only_on_hypertable {
                patched.push_str(&line.replacen("ALTER TABLE ONLY ", "ALTER TABLE ", 1));
            } else {
                patched.push_str(line);
            }
        }
        tokio::fs::write(&script.path, patched)
            .await
            .map_err(io_err)
    }

    /// Give `target` the source's extensions before `script` is replayed into it.
    async fn prepare_target(
        &self,
        db: &DB,
        target: &PgDatabase,
        creator: ExtensionCreator,
        script: &DumpFile,
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
        comment_out_schema_creations(script, &self.extension_schemas).await
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

    /// Make each hypertable of the source one again in `target`, streaming its rows from `rows`
    /// when given. A hypertable `target` already has is left as it is.
    async fn make_hypertables(
        &self,
        db: &DB,
        target: &PgDatabase,
        rows: Option<&tokio_postgres::Client>,
    ) -> Result<()> {
        if self.hypertables.is_empty() {
            return Ok(());
        }
        let (mut client, connection) = target.connect(Some(db)).await?;
        let join_handle = tokio::spawn(async move { connection.await });
        let result = self.make_hypertables_on(&mut client, rows).await;
        drop(client);
        windmill_common::shutdown_pg_connection(join_handle).await?;
        result.map_err(|e| {
            Error::internal_err(format!(
                "Failed to restore the hypertables of the copy '{}': {e}",
                target.dbname
            ))
        })
    }

    async fn make_hypertables_on(
        &self,
        target: &mut tokio_postgres::Client,
        rows: Option<&tokio_postgres::Client>,
    ) -> std::result::Result<(), String> {
        let pg = |e: tokio_postgres::Error| pg_error_message(&e);
        // Qualified: copying rows narrows the search path for the rest of the transaction.
        let timescaledb = self
            .extensions
            .iter()
            .find(|e| e.name == TIMESCALEDB)
            .map(|e| quote_ident(&e.schema))
            .ok_or("hypertables without TimescaleDB")?;
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
                // The replay makes the source's indexes, the default ones among them.
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
                        "SELECT {timescaledb}.{function}($1::text::regclass, \
                         $2::text::name{argument}{extra})"
                    ),
                    &params,
                )
                .await
                .map_err(|e| format!("{relation}: {}", pg_error_message(&e)))?;
            }
            if let Some(source) = rows {
                copy_rows(source, &tx, hypertable)
                    .await
                    .map_err(|e| format!("copying the rows of {relation}: {e}"))?;
            }
        }
        tx.commit().await.map_err(pg)
    }
}

/// What both sides of [`copy_rows`] set, so each value's text reads the same on the target as it
/// was written on the source. Not binary: that carries OIDs, which differ between databases, for
/// `regclass` values and the element types of arrays and composites of user-defined types. With
/// only `pg_catalog` on the search path, `regclass` and its kin are written schema-qualified.
const PORTABLE_TEXT_SETTINGS: &str = "SET LOCAL DateStyle = 'ISO, YMD';
     SET LOCAL IntervalStyle = 'postgres';
     SET LOCAL TimeZone = 'UTC';
     SET LOCAL extra_float_digits = 3;
     SET LOCAL bytea_output = 'hex';
     SET LOCAL lc_monetary = 'C';
     SET LOCAL search_path = pg_catalog";

/// Stream the rows of `hypertable` from `source`, a session inside the dump's snapshot, into
/// `target`. A `COPY` of a hypertable itself reads none of its rows, which live in its chunks: the
/// source side copies a query instead.
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
    source
        .batch_execute(PORTABLE_TEXT_SETTINGS)
        .await
        .map_err(pg)?;
    target
        .batch_execute(PORTABLE_TEXT_SETTINGS)
        .await
        .map_err(pg)?;
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
    match creator {
        ExtensionCreator::ClusterAdmin(DataTableCatalogResourceType::ExternalInstance) => {
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
        ExtensionCreator::ClusterAdmin(DataTableCatalogResourceType::Instance) => {
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
        ExtensionCreator::ClusterAdmin(DataTableCatalogResourceType::Postgresql)
        | ExtensionCreator::TargetRole => execute(db, target, &sql).await.map_err(|e| {
            Error::BadRequest(format!(
                "The copy needs extension '{name}', which its new database does not have and \
                 role '{}' may not create: {}. Install the extension in the server's template1 \
                 database, which new databases are made from, or let the role create it.",
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

/// A `schema.name` as pg_dump writes it, each part double-quoted only when it has to be, unquoted.
fn parse_qualified(qualified: &str) -> Option<(String, String)> {
    let (schema, rest) = parse_ident(qualified)?;
    let (name, rest) = parse_ident(rest.strip_prefix('.')?)?;
    rest.is_empty().then_some((schema, name))
}

fn parse_ident(s: &str) -> Option<(String, &str)> {
    let Some(quoted) = s.strip_prefix('"') else {
        let end = s
            .find(|c: char| c == '.' || c.is_whitespace())
            .unwrap_or(s.len());
        return (end > 0).then(|| (s[..end].to_string(), &s[end..]));
    };
    let mut ident = String::new();
    let mut rest = quoted;
    loop {
        let close = rest.find('"')?;
        ident.push_str(&rest[..close]);
        rest = &rest[close + 1..];
        match rest.strip_prefix('"') {
            Some(after) => {
                ident.push('"');
                rest = after;
            }
            None => return Some((ident, rest)),
        }
    }
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

/// Comment out the script's `CREATE SCHEMA` of each of `schemas`, which the copy already has, in
/// place and three bytes each so the rest of the script stays where it is. pg_dump writes every
/// schema before any other object, so the scan stops at the first other `CREATE` or `COPY`.
async fn comment_out_schema_creations(script: &DumpFile, schemas: &[String]) -> Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncSeekExt, AsyncWriteExt};

    if schemas.is_empty() {
        return Ok(());
    }
    let io_err = |e: std::io::Error| Error::internal_err(format!("Failed to patch dump file: {e}"));
    let file = tokio::fs::File::open(&script.path).await.map_err(io_err)?;
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
        .open(&script.path)
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
    use super::{created_schema, parse_qualified};

    #[test]
    fn parse_qualified_unquotes_what_pg_dump_quotes() {
        let parsed = |s: &str| parse_qualified(s).map(|(a, b)| format!("{a}|{b}"));
        assert_eq!(parsed("public.metrics").as_deref(), Some("public|metrics"));
        assert_eq!(
            parsed("\"My.Schema\".\"a\"\"b\"").as_deref(),
            Some("My.Schema|a\"b")
        );
        assert_eq!(parsed("public.metrics ADD"), None);
        assert_eq!(parsed("metrics"), None);
    }

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
