/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Copying a data table's database for the fork being created.
//!
//! The fork request makes its copies before it writes the fork: each is created, filled and — for a
//! data table under roles — given the source's owners and grants. What each copy was made from
//! stays in the request ([`MadeCopy`]), so the fork's transaction checks it against the source as it
//! is then, and a fork that fails drops the instance copies it made ([`drop_copies_after`]) and names
//! the others, which live on servers of the workspace's own.

use std::collections::BTreeSet;

use windmill_api_auth::ApiAuthed;
use windmill_common::datatable_roles::{lock_role_catalog, read_role_catalog_tx};
use windmill_common::error::{pg_error_message, Error, Result};
use windmill_common::utils::require_admin;
use windmill_common::worker::CLOUD_HOSTED;
use windmill_common::workspaces::{
    get_datatable_resource_from_db_unchecked, DataTableDatabase, DataTableForkBehavior,
    GoverningDatatable,
};
use windmill_common::{PgDatabase, DB};

use crate::datatable_acl::connect_with_notices;
use crate::datatable_permissions::ensure_reaches_governing_datatable;
use crate::workspaces::{
    create_database_on_server, ensure_datatable_is_clonable, pg_dump_database, pg_import_dump,
    DumpFile, PgDumpOptions,
};

/// A database this request created and filled for one data table of the fork.
pub(crate) struct MadeCopy {
    /// The data table's name, in the parent and in the fork.
    pub(crate) name: String,
    pub(crate) dbname: String,
    pub(crate) behavior: DataTableForkBehavior,
    /// The database the source resolved to when it was copied.
    pub(crate) source_database: DataTableDatabase,
    /// Whether the source's owners and grants were replayed into the copy: it was under roles.
    pub(crate) replayed: bool,
    /// For a resource-backed copy, the resource and variables its connection was resolved from.
    pub(crate) connection: Option<ConnectionSnapshot>,
}

/// What a resource-backed data table's connection is resolved from: its resource, and every
/// resource and variable that one references, as stored — and, for a secret kept in an external
/// backend, the value that backend holds. The connection is resolved from the snapshot itself
/// ([`ConnectionSnapshot::resolve`]), so it is exactly the one these rows describe.
#[derive(PartialEq)]
pub(crate) struct ConnectionSnapshot {
    root: String,
    resources: std::collections::BTreeMap<String, Option<serde_json::Value>>,
    /// Stored value and whether it is secret.
    variables: std::collections::BTreeMap<String, Option<(String, bool)>>,
    external_secrets: std::collections::BTreeMap<String, String>,
}

/// Read the [`ConnectionSnapshot`] of resource `resource_path` in workspace `w_id`.
///
/// Authorization: none, and it holds secret values. Callers MUST have authorized using that
/// resource, and never disclose the snapshot or what it resolves to.
pub(crate) async fn connection_snapshot(
    db: &DB,
    conn: &mut sqlx::PgConnection,
    w_id: &str,
    resource_path: &str,
) -> Result<ConnectionSnapshot> {
    let root = resource_path.trim_start_matches("$res:").to_string();
    let mut snapshot = ConnectionSnapshot {
        root: root.clone(),
        resources: Default::default(),
        variables: Default::default(),
        external_secrets: Default::default(),
    };
    let mut pending = vec![root];
    while let Some(path) = pending.pop() {
        if snapshot.resources.contains_key(&path) {
            continue;
        }
        let value: Option<serde_json::Value> =
            sqlx::query_scalar("SELECT value FROM resource WHERE workspace_id = $1 AND path = $2")
                .bind(w_id)
                .bind(&path)
                .fetch_optional(&mut *conn)
                .await?
                .flatten();
        let mut strings = vec![];
        collect_strings(value.as_ref(), &mut strings);
        for reference in strings {
            if let Some(var) = reference.strip_prefix("$var:") {
                if snapshot.variables.contains_key(var) {
                    continue;
                }
                let row: Option<(String, bool)> = sqlx::query_as(
                    "SELECT value, is_secret FROM variable WHERE workspace_id = $1 AND path = $2",
                )
                .bind(w_id)
                .bind(var)
                .fetch_optional(&mut *conn)
                .await?;
                if let Some((stored, true)) = &row {
                    if windmill_common::secret_backend::is_external_stored_value(stored) {
                        let secret = windmill_common::secret_backend::get_secret_value(
                            db, w_id, var, stored,
                        )
                        .await?;
                        snapshot.external_secrets.insert(var.to_string(), secret);
                    }
                }
                snapshot.variables.insert(var.to_string(), row);
            } else if let Some(res) = reference.strip_prefix("$res:") {
                pending.push(res.to_string());
            }
        }
        snapshot.resources.insert(path, value);
    }
    Ok(snapshot)
}

impl ConnectionSnapshot {
    /// The connection these rows resolve to, as the data table's own resolution would: references
    /// substituted, secrets decrypted.
    pub(crate) async fn resolve(&self, db: &DB, w_id: &str) -> Result<serde_json::Value> {
        let root = self.resource(&self.root)?;
        self.substitute(db, w_id, root, 0).await
    }

    /// Whether the resource itself holds the connection's fields, which the fork repoints by setting
    /// its `dbname`, rather than being a reference to another resource.
    pub(crate) fn root_holds_connection(&self) -> bool {
        self.resource(&self.root).is_ok_and(|v| v.is_object())
    }

    fn resource(&self, path: &str) -> Result<&serde_json::Value> {
        self.resources
            .get(path)
            .and_then(|v| v.as_ref())
            .ok_or_else(|| Error::NotFound(format!("resource {path} does not exist")))
    }

    fn substitute<'a>(
        &'a self,
        db: &'a DB,
        w_id: &'a str,
        value: &'a serde_json::Value,
        depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send + 'a>>
    {
        Box::pin(async move {
            if depth > 32 {
                return Err(Error::BadRequest(
                    "resource references nest too deeply".to_string(),
                ));
            }
            Ok(match value {
                serde_json::Value::Object(map) => {
                    let mut out = serde_json::Map::new();
                    for (key, val) in map {
                        out.insert(key.clone(), self.substitute(db, w_id, val, depth).await?);
                    }
                    serde_json::Value::Object(out)
                }
                serde_json::Value::Array(items) => {
                    let mut out = vec![];
                    for val in items {
                        out.push(self.substitute(db, w_id, val, depth).await?);
                    }
                    serde_json::Value::Array(out)
                }
                serde_json::Value::String(s) if s.starts_with("$res:") => {
                    let path = &s[5..];
                    self.substitute(db, w_id, self.resource(path)?, depth + 1)
                        .await?
                }
                serde_json::Value::String(s) if s.starts_with("$var:") => {
                    let path = &s[5..];
                    let (stored, secret) = self
                        .variables
                        .get(path)
                        .and_then(|v| v.as_ref())
                        .ok_or_else(|| {
                            Error::NotFound(format!("variable {path} does not exist"))
                        })?;
                    serde_json::Value::String(match (secret, self.external_secrets.get(path)) {
                        (false, _) => stored.clone(),
                        (true, Some(external)) => external.clone(),
                        (true, None) => windmill_common::variables::decrypt(
                            &windmill_common::variables::build_crypt(db, w_id).await?,
                            stored.clone(),
                        )
                        .map_err(|e| {
                            Error::internal_err(format!("Error decrypting variable {s}: {e}"))
                        })?,
                    })
                }
                other => other.clone(),
            })
        })
    }
}

fn collect_strings<'a>(value: Option<&'a serde_json::Value>, out: &mut Vec<&'a str>) {
    match value {
        Some(serde_json::Value::String(s)) => out.push(s),
        Some(serde_json::Value::Array(items)) => {
            items.iter().for_each(|v| collect_strings(Some(v), out))
        }
        Some(serde_json::Value::Object(map)) => {
            map.values().for_each(|v| collect_strings(Some(v), out))
        }
        _ => {}
    }
}

/// What a data table of the fork should be copied as.
pub(crate) struct CopyRequest<'a> {
    pub(crate) name: &'a str,
    pub(crate) dbname: &'a str,
    pub(crate) behavior: DataTableForkBehavior,
}

/// Check that `authed` may copy each data table of `parent_w_id` as asked, before anything is
/// created.
///
/// Who may copy is what it was before data table roles — anyone for the schema, an admin of the
/// workspace for the rows — narrowed under roles to whoever may connect as one of them: even the
/// schema alone lists every table, which under roles only role holders can read in the parent.
pub(crate) async fn authorize_copies(
    db: &DB,
    authed: &ApiAuthed,
    parent_w_id: &str,
    requests: &[CopyRequest<'_>],
) -> Result<()> {
    for request in requests {
        match request.behavior {
            DataTableForkBehavior::KeepOriginal => {
                return Err(Error::BadRequest(format!(
                    "Data table '{}' is kept, which copies nothing",
                    request.name
                )))
            }
            DataTableForkBehavior::SchemaOnly => {}
            DataTableForkBehavior::SchemaAndData => {
                require_admin(authed.is_admin, &authed.username)?;
                if *CLOUD_HOSTED {
                    return Err(Error::BadRequest(
                        "Cloning schema and data is not available on cloud".to_string(),
                    ));
                }
            }
        }
        windmill_common::validate_dbname(request.dbname)?;
        if !request.dbname.starts_with("wm_fork_") {
            return Err(Error::BadRequest(format!(
                "Forked datatable database name '{}' must start with 'wm_fork_'",
                request.dbname
            )));
        }
        let governing = ensure_datatable_is_clonable(db, parent_w_id, request.name).await?;
        ensure_reaches_governing_datatable(db, parent_w_id, request.name, &governing, authed)
            .await?;
        if governing.datatable.permissions.is_some() {
            crate::datatable_replay_oss::ensure_replay()?;
        }
    }
    Ok(())
}

/// Make every copy in `requests`, in order, once [`authorize_copies`] allows them all. When one
/// fails, the copies already made are dropped before the error is returned.
pub(crate) async fn make_copies(
    db: &DB,
    authed: &ApiAuthed,
    parent_w_id: &str,
    requests: &[CopyRequest<'_>],
) -> Result<Vec<MadeCopy>> {
    authorize_copies(db, authed, parent_w_id, requests).await?;
    let mut copies = Vec::with_capacity(requests.len());
    for request in requests {
        match make_copy(db, authed, parent_w_id, request).await {
            Ok(copy) => copies.push(copy),
            Err(e) => return Err(drop_copies_after(db, copies, e).await),
        }
    }
    Ok(copies)
}

/// Drop every copy, returning `error` — with what could not be dropped appended, since nothing
/// else will name those databases again.
pub(crate) async fn drop_copies_after(db: &DB, copies: Vec<MadeCopy>, error: Error) -> Error {
    let mut stranded = Vec::new();
    for copy in copies {
        if let Err(e) = drop_copy(db, &copy.source_database, &copy.dbname).await {
            tracing::error!("Could not drop '{}' after a failed fork: {e}", copy.dbname);
            stranded.push(format!("'{}' ({e})", copy.dbname));
        }
    }
    if stranded.is_empty() {
        error
    } else {
        Error::ExecutionErr(format!(
            "{error}. These databases created for the fork could not be dropped: {}",
            stranded.join(", ")
        ))
    }
}

async fn drop_copy(db: &DB, source_database: &DataTableDatabase, dbname: &str) -> Result<()> {
    if source_database.resource_type
        == windmill_common::workspaces::DataTableCatalogResourceType::Instance
    {
        match windmill_common::drop_unused_instance_database(db, dbname).await? {
            windmill_common::Cleanup::Dropped => Ok(()),
            windmill_common::Cleanup::InUse(users) => Err(Error::BadRequest(format!(
                "kept, since workspaces {} now use it",
                users.join(", ")
            ))),
            windmill_common::Cleanup::Waiter(pid) => Err(Error::BadRequest(format!(
                "kept, since a request (pid {pid}) is still waiting to name it"
            ))),
        }
    } else {
        // On a server of the workspace's own, where a resource edited meanwhile can already name it
        // and nothing locks such an edit: dropping it could take someone's data.
        Err(Error::BadRequest(
            "kept on its PostgreSQL server, where it may already be in use; drop it there once it \
             is not, before forking the same data table under this id again"
                .to_string(),
        ))
    }
}

async fn make_copy(
    db: &DB,
    authed: &ApiAuthed,
    parent_w_id: &str,
    request: &CopyRequest<'_>,
) -> Result<MadeCopy> {
    let governing = ensure_datatable_is_clonable(db, parent_w_id, request.name).await?;
    let source_database = governing.datatable.database.clone().ok_or_else(|| {
        Error::internal_err(format!(
            "Data table '{}' resolves to an entry that owns no database",
            request.name
        ))
    })?;
    let is_instance = governing.is_instance();
    // Resolved from the snapshot, not read again: the connection the copy is made on is then
    // exactly what these rows describe, which the fork's own clone of them is checked against.
    let (server, connection): (PgDatabase, Option<ConnectionSnapshot>) = if is_instance {
        let server = serde_json::from_value(
            get_datatable_resource_from_db_unchecked(db, parent_w_id, request.name).await?,
        )
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {e}")))?;
        (server, None)
    } else {
        let snapshot = connection_snapshot(
            db,
            &mut *db.acquire().await?,
            &governing.workspace_id,
            &source_database.resource_path,
        )
        .await?;
        if !snapshot.root_holds_connection() {
            return Err(Error::BadRequest(format!(
                "Data table '{}' uses resource '{}', which only refers to another resource: the \
                 fork's copy of it could not be pointed at the copied database. Point the data \
                 table at the resource holding the connection, then fork again.",
                request.name, source_database.resource_path
            )));
        }
        let server = serde_json::from_value(snapshot.resolve(db, &governing.workspace_id).await?)
            .map_err(|e| {
            Error::internal_err(format!("Failed to parse database credentials: {e}"))
        })?;
        (server, Some(snapshot))
    };

    // Dumped before the database exists, so a source that cannot be read leaves nothing behind.
    // Ownership never carries over: the restore runs as the target's connection user. Grants do,
    // except on the instance, where the replay below is what reproduces them.
    let dump = pg_dump_database(
        &server,
        PgDumpOptions {
            schema_only: request.behavior == DataTableForkBehavior::SchemaOnly,
            no_owner: true,
            no_acl: is_instance,
            ..Default::default()
        },
    )
    .await?;

    if is_instance {
        windmill_common::create_custom_instance_database(db, request.dbname, "datatable").await?;
    } else {
        create_database_on_server(db, &server, request.dbname).await?;
    }

    let target = PgDatabase { dbname: request.dbname.to_string(), ..server.clone() };
    let filled = fill(
        db,
        authed,
        parent_w_id,
        request.name,
        &governing,
        &server,
        &target,
        &dump,
    )
    .await;
    match filled {
        Ok(replayed) => Ok(MadeCopy {
            name: request.name.to_string(),
            dbname: request.dbname.to_string(),
            behavior: request.behavior,
            source_database,
            replayed,
            connection,
        }),
        Err(e) => {
            if let Err(drop_err) = drop_copy(db, &source_database, request.dbname).await {
                tracing::error!(
                    "Could not drop '{}' after a failed copy of data table '{}': {drop_err}",
                    request.dbname,
                    request.name
                );
                return Err(Error::ExecutionErr(format!(
                    "{e}. The database '{}' created for the copy could not be dropped: {drop_err}",
                    request.dbname
                )));
            }
            Err(e)
        }
    }
}

/// Restore the dump into the new database and, for a data table under roles, replay the source's
/// owners and grants into it. Returns whether it replayed.
#[allow(clippy::too_many_arguments)]
async fn fill(
    db: &DB,
    authed: &ApiAuthed,
    parent_w_id: &str,
    name: &str,
    governing: &GoverningDatatable,
    source: &PgDatabase,
    target: &PgDatabase,
    dump: &DumpFile,
) -> Result<bool> {
    pg_import_dump(target, dump).await?;

    // Held until the replay commits: a role renamed or dropped meanwhile would change what the
    // replay names, and a settings save could move the source onto another database or put it under
    // roles. Taken in the same order as the permissions save and the ACL apply.
    // On a connection of its own: the checks below take theirs from the pool, and a lock holder
    // drawn from the pool too could leave a small one with nothing to give them.
    let database_url = windmill_common::get_database_url().await?;
    let mut lock_holder = <sqlx::PgConnection as sqlx::Connection>::connect_with(
        &database_url.connect_options().await?,
    )
    .await
    .map_err(|e| Error::internal_err(format!("Failed to connect to the database: {e}")))?;
    let mut tx = sqlx::Connection::begin(&mut lock_holder).await?;
    lock_role_catalog(&mut tx).await?;
    lock_settings_rows(&mut tx, parent_w_id, name).await?;

    // Everything so far was decided before the locks.
    let now = ensure_datatable_is_clonable(db, parent_w_id, name).await?;
    ensure_reaches_governing_datatable(db, parent_w_id, name, &now, authed).await?;
    if !same_database(&now.datatable.database, &governing.datatable.database) {
        return Err(Error::BadRequest(format!(
            "Data table '{name}' moved to another database while it was being copied; fork again"
        )));
    }

    let replayed = now.datatable.permissions.is_some();
    if replayed {
        crate::datatable_replay_oss::ensure_replay()?;
        let catalog = read_role_catalog_tx(&mut tx).await?;
        // The replay leaves `CONNECT` to the catalog, and creating the database only tried to set
        // it: a copy `PUBLIC` could still connect to would admit logins the source turns away.
        windmill_common::datatable_roles::converge_connect_grants_with(
            db,
            &target.dbname,
            &catalog,
        )
        .await?;
        let catalog_roles: BTreeSet<String> = catalog.values().map(|r| r.name.clone()).collect();
        let (source, _source_notices) = connect_with_notices(db, source).await?;
        let (mut target, mut notices) = connect_with_notices(db, target).await?;
        // One transaction on the copy: a replay that stops halfway leaves objects owned by one role
        // and granted as another.
        let pg_tx = target.transaction().await.map_err(|e| {
            Error::internal_err(format!(
                "Failed to open a transaction on the copy: {}",
                pg_error_message(&e)
            ))
        })?;
        crate::datatable_replay_oss::replay_owners_and_grants(
            &source,
            &pg_tx,
            &mut notices,
            &catalog_roles,
        )
        .await?;
        pg_tx.commit().await.map_err(|e| {
            Error::internal_err(format!(
                "Failed to commit the replayed grants: {}",
                pg_error_message(&e)
            ))
        })?;
    }
    tx.commit().await?;
    Ok(replayed)
}

/// Lock every settings row the resolution of data table `name` of `start_w_id` passes through, in
/// the order it passes them — each pointer, the entry that owns the database, and each workspace its
/// roles come from — and return them. Deleting or repointing any of them would leave a copy made
/// for the resolution answering for another one.
pub(crate) async fn lock_settings_rows(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    start_w_id: &str,
    name: &str,
) -> Result<Vec<String>> {
    let mut path: Vec<(String, String)> = vec![];
    let (mut w_id, mut datatable) = (start_w_id.to_string(), name.to_string());
    loop {
        if path.contains(&(w_id.clone(), datatable.clone())) || path.len() > 32 {
            return Err(Error::BadRequest(format!(
                "Data table '{name}' of workspace '{start_w_id}' resolves through a cycle"
            )));
        }
        let entry: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT datatable->'datatables'->$2 FROM workspace_settings
             WHERE workspace_id = $1 FOR UPDATE",
        )
        .bind(&w_id)
        .bind(&datatable)
        .fetch_optional(&mut **tx)
        .await?
        .flatten();
        path.push((w_id.clone(), datatable.clone()));
        let entry: Option<windmill_common::workspaces::DataTable> =
            entry.and_then(|e| serde_json::from_value(e).ok());
        match entry.and_then(|e| e.reference.or(e.governed_by)) {
            Some(next) => (w_id, datatable) = (next.workspace_id, next.datatable),
            None => break,
        }
    }
    Ok(path.into_iter().map(|(w_id, _)| w_id).collect())
}

pub(crate) fn same_database(a: &Option<DataTableDatabase>, b: &Option<DataTableDatabase>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            a.resource_type == b.resource_type && a.resource_path == b.resource_path
        }
        _ => false,
    }
}
