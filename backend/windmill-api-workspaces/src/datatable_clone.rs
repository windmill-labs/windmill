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
//! is then, and a fork that fails drops every copy it made ([`drop_copies_after`]). Nothing about a
//! copy outlives the request that made it.

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
use crate::datatable_permissions::ensure_reaches_datatable;
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
    /// The connection the copy was made through, which is also what drops it.
    server: PgDatabase,
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
        ensure_reaches_datatable(db, parent_w_id, request.name, authed).await?;
        let governing = ensure_datatable_is_clonable(db, parent_w_id, request.name).await?;
        if governing.datatable.permissions.is_some() {
            crate::datatable_replay_oss::ensure_replay()?;
        }
    }
    Ok(())
}

/// Make every copy in `requests`, in order. When one fails, the copies already made are dropped
/// before the error is returned.
pub(crate) async fn make_copies(
    db: &DB,
    authed: &ApiAuthed,
    parent_w_id: &str,
    requests: &[CopyRequest<'_>],
) -> Result<Vec<MadeCopy>> {
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
        if let Err(e) = drop_copy(db, &copy.server, &copy.source_database, &copy.dbname).await {
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

async fn drop_copy(
    db: &DB,
    server: &PgDatabase,
    source_database: &DataTableDatabase,
    dbname: &str,
) -> Result<()> {
    if source_database.resource_type
        == windmill_common::workspaces::DataTableCatalogResourceType::Instance
    {
        windmill_common::drop_custom_instance_database(db, dbname).await
    } else {
        drop_database_on_server(db, server, dbname).await
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
    let server: PgDatabase = serde_json::from_value(
        get_datatable_resource_from_db_unchecked(db, parent_w_id, request.name).await?,
    )
    .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {e}")))?;
    let is_instance = governing.is_instance();

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
            server,
        }),
        Err(e) => {
            if let Err(drop_err) = drop_copy(db, &server, &source_database, request.dbname).await {
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
    let mut tx = db.begin().await?;
    lock_role_catalog(&mut tx).await?;
    lock_settings_rows(&mut tx, governing).await?;

    // Everything so far was decided before the locks.
    ensure_reaches_datatable(db, parent_w_id, name, authed).await?;
    let now = ensure_datatable_is_clonable(db, parent_w_id, name).await?;
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

/// Lock the settings rows `governing` was resolved from: where its entry lives, and where its roles
/// are decided.
pub(crate) async fn lock_settings_rows(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    governing: &GoverningDatatable,
) -> Result<Vec<String>> {
    let mut workspace_ids = vec![
        governing.workspace_id.clone(),
        governing.governing_workspace_id().to_string(),
    ];
    workspace_ids.sort();
    workspace_ids.dedup();
    sqlx::query(
        "SELECT 1 FROM workspace_settings WHERE workspace_id = ANY($1)
         ORDER BY workspace_id FOR UPDATE",
    )
    .bind(&workspace_ids)
    .fetch_all(&mut **tx)
    .await?;
    Ok(workspace_ids)
}

pub(crate) fn same_database(a: &Option<DataTableDatabase>, b: &Option<DataTableDatabase>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            a.resource_type == b.resource_type && a.resource_path == b.resource_path
        }
        _ => false,
    }
}

/// `DROP DATABASE` on the server `server` connects to, disconnecting whoever is still on it.
async fn drop_database_on_server(db: &DB, server: &PgDatabase, dbname: &str) -> Result<()> {
    windmill_common::validate_dbname(dbname)?;
    let (client, connection) = server.connect(Some(db)).await?;
    let join_handle = tokio::spawn(async move { connection.await });
    let result = client
        .execute(
            &format!("DROP DATABASE IF EXISTS \"{dbname}\" WITH (FORCE)"),
            &[],
        )
        .await;
    drop(client);
    let _ = windmill_common::shutdown_pg_connection(join_handle).await;
    result.map(|_| ()).map_err(|e| {
        Error::internal_err(format!(
            "Failed to drop database '{dbname}': {}",
            pg_error_message(&e)
        ))
    })
}
