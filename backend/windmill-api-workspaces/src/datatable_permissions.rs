/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Role-based access control for data tables: read the config, preview the SQL a
//! change plans out, and apply it.
//!
//! A permissioned data table maps each Windmill role onto a real Postgres login
//! role, so a script that runs as `analyst` connects as `analyst` and the
//! database — not Windmill — enforces what it may touch. `admin` is the exception:
//! it is the connection the data table already resolved to before permissions
//! were turned on, so it owns every existing object and is never created,
//! renamed or dropped.

use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

use windmill_api_auth::ApiAuthed;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::ensure_instance_db_grant_options_unchecked;
use windmill_common::error::{pg_error_message, Error, JsonResult, Result};
use windmill_common::query_builders::{render_db_quoted_identifier, DbType};
use windmill_common::utils::require_admin;
use windmill_common::worker::SqlAnnotations;
use windmill_common::workspaces::{
    can_use_datatable_role_in_owner_workspace, database_permissions_by_key,
    delete_database_permissions, lock_database_permissions, lock_database_permissions_key,
    lock_datatable_permissions_unchecked, resolve_datatable_database_unchecked,
    upsert_database_permissions, DataTable, DataTableCatalogResourceType, DataTablePermissions,
    DatabasePermissions, DatatableAccess, ADMIN_DATATABLE_ROLE,
};
use windmill_common::{PgDatabase, DB};

pub(crate) fn routes() -> Router {
    Router::new()
        .route(
            "/datatable_permissions/{datatable_name}",
            get(get_datatable_permissions).post(set_datatable_permissions),
        )
        .route(
            "/datatable_permissions/{datatable_name}/preview",
            post(preview_datatable_permissions),
        )
        .route(
            "/datatable_usable_roles/{datatable_name}",
            get(list_usable_datatable_roles),
        )
        .route(
            "/datatable_permissions_import",
            post(import_datatable_permissions),
        )
}

/// A data table role as the UI sees it: the generated password never leaves the
/// server, since it grants direct database access to anyone who reads it.
#[derive(Serialize, Deserialize, Debug)]
pub struct DatatableRoleInfo {
    pub name: String,
    #[serde(default)]
    pub tenants: Vec<String>,
    /// The underlying Postgres role, so grants can be written by hand against it.
    /// Absent for `admin`, which reuses the data table's own connection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pg_rolename: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct DatatablePermissionsInfo {
    pub enabled: bool,
    pub roles: Vec<DatatableRoleInfo>,
    /// The role a script gets when it names none.
    pub default_role: String,
    /// The workspace whose admins manage these permissions and whose principals
    /// the tenants are: the one that turned them on. Absent while they are off.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_workspace_id: Option<String>,
    /// Whether the caller may change them from here: an admin of the owning
    /// workspace, or a superadmin.
    pub editable: bool,
}

/// A database's permissions as a workspace export carries them: named by a data
/// table of the workspace that reaches the database — the key is the server's
/// to derive, never the client's to choose — with the roles and their tenants.
/// Neither the login names nor the passwords are taken back: a login name is a
/// cluster-wide identifier the next save would rename or reset, so it is the
/// save's to generate.
#[derive(Deserialize, Debug)]
pub struct ImportedDatabasePermissions {
    pub datatable: String,
    pub permissions: DataTablePermissions,
}

#[derive(Deserialize, Debug)]
pub struct SetDatatablePermissions {
    pub enabled: bool,
    #[serde(default)]
    pub roles: Vec<DatatableRoleInfo>,
    /// The role a script gets when it names none. Absent means `admin`.
    #[serde(default)]
    pub default_role: Option<String>,
    /// Role renames (old -> new), tracked client-side by a stable id so a rename
    /// plans an `ALTER ROLE ... RENAME` instead of a drop plus a create — which
    /// would destroy the grants the role had accumulated.
    #[serde(default)]
    pub renames: Vec<DatatableRoleRename>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DatatableRoleRename {
    pub from: String,
    pub to: String,
}

/// The roles the caller may actually run as, for pickers. Unlike the admin-only
/// permissions view this exposes no tenant lists — only what the caller can use.
#[derive(Serialize, Debug)]
pub struct UsableDatatableRoles {
    pub enabled: bool,
    pub roles: Vec<String>,
    pub default_role: String,
}

#[derive(Serialize, Debug)]
pub struct DatatablePermissionsPreview {
    pub statements: Vec<String>,
    pub warnings: Vec<String>,
}

/// One planned statement. `display` is what the preview shows: identical to
/// `sql` except where a generated password would otherwise be printed.
#[derive(Debug)]
pub(crate) struct PlannedStatement {
    pub(crate) sql: String,
    pub(crate) display: String,
    /// The Postgres role this statement is part of destroying, if any.
    ///
    /// Creating a role before the config names it leaves at worst a role the
    /// next save adopts, so those run inside the request. Dropping one is not
    /// reversible — `DROP OWNED` discards every grant it accumulated — so these
    /// wait for the config that stops naming the role to commit, and are then
    /// checked against it one role at a time.
    pub(crate) drops_role: Option<String>,
}

impl PlannedStatement {
    /// Only the planner builds these, and that is the enterprise module.
    #[cfg_attr(
        not(all(feature = "private", feature = "enterprise")),
        allow(dead_code)
    )]
    pub(crate) fn plain(sql: String) -> Self {
        Self { display: sql.clone(), sql, drops_role: None }
    }
}

#[derive(Debug)]
pub(crate) struct RolePlan {
    pub(crate) statements: Vec<PlannedStatement>,
    /// The permissions block to persist once the statements have run.
    pub(crate) permissions: DataTablePermissions,
    pub(crate) warnings: Vec<String>,
}

/// Refuse to plan a change on an enterprise binary whose plan does not cover it.
/// A build that is not enterprise has no planner at all — see
/// [`crate::datatable_permissions_oss`] — so this only has the licensed
/// editions left to tell apart.
pub(crate) async fn require_datatable_permissions_license() -> Result<()> {
    #[cfg(feature = "enterprise")]
    if !matches!(
        windmill_common::ee_oss::get_license_plan().await,
        windmill_common::ee_oss::LicensePlan::Enterprise
    ) {
        return Err(Error::BadRequest(
            "Data table permissions require an Enterprise license".to_string(),
        ));
    }
    Ok(())
}

/// Only the planners quote identifiers, and those are the enterprise modules.
#[cfg_attr(
    not(all(feature = "private", feature = "enterprise")),
    allow(dead_code)
)]
pub(crate) fn quote_ident(ident: &str) -> String {
    render_db_quoted_identifier(ident, DbType::Postgresql)
}

/// Read a data table's config, whatever the caller is.
///
/// Authorization: performs none, for any workspace it is handed, so callers
/// MUST have authorized the read.
pub(crate) async fn read_datatable_unchecked(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
) -> Result<DataTable> {
    let value = sqlx::query_scalar!(
        "SELECT ws.datatable->'datatables'->$2 FROM workspace_settings ws WHERE ws.workspace_id = $1",
        w_id,
        datatable_name,
    )
    .fetch_one(db)
    .await?
    .filter(|v| !v.is_null())
    .ok_or_else(|| Error::NotFound(format!("Data table '{datatable_name}' not found")))?;
    serde_json::from_value(value)
        .map_err(|e| Error::internal_err(format!("Invalid data table config: {e}")))
}

/// Make sure `custom_instance_user` can pass its privileges on, for a data table
/// on an instance database.
///
/// Handing privileges to the roles this feature creates means granting them, and
/// a privilege held without `WITH GRANT OPTION` cannot be granted on — Postgres
/// answers such a statement with a warning and no effect. Databases provisioned
/// before those options were part of the grants still hold them plain, and only
/// the instance's own Postgres user, which owns them, can add the options.
///
/// Idempotent, and a no-op for a data table on a user-provided resource, where
/// Windmill does not own the Postgres user. Never fatal: the caller's own work
/// is what the admin asked for, and it may well not need any of this.
pub(crate) async fn ensure_instance_db_can_delegate(db: &DB, w_id: &str, datatable_name: &str) {
    let Ok(datatable) = read_datatable_unchecked(db, w_id, datatable_name).await else {
        return;
    };
    if datatable.database.resource_type != DataTableCatalogResourceType::Instance {
        return;
    }
    if let Err(e) =
        ensure_instance_db_grant_options_unchecked(db, &datatable.database.resource_path).await
    {
        tracing::warn!(
            "Could not refresh the grant options of instance database '{}': {}. Continuing.",
            datatable.database.resource_path,
            e
        );
    }
}

/// What the plan has to be built against, probed from the data table's own
/// database rather than assumed from its config.
pub(crate) struct AdminConnection {
    pub(crate) dbname: String,
    /// The key of the database this connection reaches, which is what its
    /// permissions are stored under.
    pub(crate) database_key: String,
    pub(crate) admin_pg_role: String,
    pub(crate) pg_roles: PgRoleInventory,
    /// Whether `PUBLIC` holds CREATE on schema `public`, i.e. every role in this
    /// database — including the ones created here — can make objects in it.
    pub(crate) public_schema_is_open: bool,
    /// The default-privilege rules already in force for this data table's roles.
    pub(crate) default_acl_rules: Vec<DefaultAclRule>,
}

/// The `wm_` logins the cluster holds, split by whether this data table may take
/// one over.
#[derive(Default)]
/// Only the planner reads these, and that is the enterprise module.
#[cfg_attr(
    not(all(feature = "private", feature = "enterprise")),
    allow(dead_code)
)]
pub(crate) struct PgRoleInventory {
    /// Every one of them. `pg_roles` is a cluster catalog, so this spans every
    /// database and every workspace on the instance — a name in here cannot be
    /// created again, wherever it came from.
    pub(crate) existing: HashSet<String>,
    /// Those the data table's own administrative login is a member of, which is
    /// what creating a role here does. A save may finish itself by adopting one
    /// of these — a role it created before dying — and nothing else: any other
    /// occupant of a generated name belongs to someone else, and resetting its
    /// password would hand this data table their login.
    pub(crate) adoptable: HashSet<String>,
}

/// One `ALTER DEFAULT PRIVILEGES` rule as the catalog has it.
///
/// Postgres records such a rule per creating role, so a role added later is not
/// covered by any of them: a grant on "future tables" would quietly stop
/// applying to whatever that new role creates. Replaying the existing rules for
/// each new role is what keeps the policy whole.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DefaultAclRule {
    /// `None` for a rule that is not scoped to a schema.
    pub(crate) schema: Option<String>,
    /// `TABLES`, `SEQUENCES`, `FUNCTIONS` or `TYPES`.
    pub(crate) objects: String,
    /// Postgres role the privileges go to, always one of this data table's own.
    pub(crate) grantee: String,
    pub(crate) privileges: Vec<String>,
}

/// The rules this data table's own roles wrote, which are the only ones a role
/// of this data table inherits.
///
/// Scoped to `own_pg_roles` on both sides. Two data tables can point at one
/// physical database — and share its administrative login — so a rule is ours
/// only when both the role that wrote it and the role it grants to are.
async fn read_default_acl_rules(
    client: &tokio_postgres::Client,
    own_pg_roles: &[String],
) -> Result<Vec<DefaultAclRule>> {
    let rows = client
        .query(
            "SELECT n.nspname,
                    CASE d.defaclobjtype
                        WHEN 'r' THEN 'TABLES' WHEN 'S' THEN 'SEQUENCES'
                        WHEN 'f' THEN 'FUNCTIONS' ELSE 'TYPES' END,
                    pg_get_userbyid(a.grantee),
                    a.privilege_type
             FROM pg_default_acl d
             LEFT JOIN pg_namespace n ON n.oid = d.defaclnamespace,
                  aclexplode(d.defaclacl) a
             WHERE pg_get_userbyid(d.defaclrole) = ANY($1)
               AND a.grantee <> 0
               AND pg_get_userbyid(a.grantee) = ANY($1)",
            &[&own_pg_roles],
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to read the default privileges: {}",
                pg_error_message(&e)
            ))
        })?;
    let mut folded: BTreeMap<(Option<String>, String, String), Vec<String>> = BTreeMap::new();
    for row in rows {
        folded
            .entry((row.get(0), row.get(1), row.get(2)))
            .or_default()
            .push(row.get(3));
    }
    Ok(folded
        .into_iter()
        .map(|((schema, objects, grantee), mut privileges)| {
            privileges.sort();
            privileges.dedup();
            DefaultAclRule { schema, objects, grantee, privileges }
        })
        .collect())
}

/// Connect to the data table's own database as `admin` and report the identity a
/// plan has to be built against: the database name, the role that owns the
/// existing objects, and the roles that actually exist in the cluster.
///
/// Authorization: performs none. This is the data table's own connection — it
/// owns every object in that database and can grant anything it holds — so
/// callers MUST have authorized the operation it is opened for, against the
/// identity making the request, and MUST NOT hand it to a request that has not
/// been.
pub(crate) async fn connect_as_admin_unchecked(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
) -> Result<(tokio_postgres::Client, AdminConnection)> {
    let (_, db_resource, database_key) =
        resolve_datatable_database_unchecked(db, w_id, datatable_name).await?;
    let pg_db: PgDatabase = serde_json::from_value(db_resource)
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {e}")))?;
    let dbname = pg_db.dbname.clone();
    let (client, connection) = pg_db.connect(Some(db)).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("Datatable permissions connection error: {}", e);
        }
    });

    let admin_pg_role: String = client
        .query_one("SELECT current_user", &[])
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to read the data table's connection identity: {}",
                pg_error_message(&e)
            ))
        })?
        .get(0);

    let existing = client
        .query(
            "SELECT rolname FROM pg_roles WHERE rolname LIKE 'wm\\_%'",
            &[],
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to list existing roles: {}",
                pg_error_message(&e)
            ))
        })?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect();

    // Membership is the mark a role of this data table carries: every one of
    // them is granted to this connection when it is created.
    let adoptable = client
        .query(
            "SELECT r.rolname
             FROM pg_auth_members m
             JOIN pg_roles r ON r.oid = m.roleid
             JOIN pg_roles a ON a.oid = m.member
             WHERE a.rolname = $1 AND r.rolname LIKE 'wm\\_%'",
            &[&admin_pg_role],
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to list the roles this data table owns: {}",
                pg_error_message(&e)
            ))
        })?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect();

    // grantee 0 is PUBLIC. A NULL acl means the server default, which granted
    // PUBLIC CREATE on `public` before Postgres 15.
    let public_schema_is_open: bool = client
        .query_one(
            "SELECT COALESCE(
                 (SELECT bool_or(a.privilege_type = 'CREATE' AND a.grantee = 0)
                  FROM pg_namespace n, aclexplode(n.nspacl) a
                  WHERE n.nspname = 'public'),
                 current_setting('server_version_num')::int < 150000
             )",
            &[],
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to inspect the public schema: {}",
                pg_error_message(&e)
            ))
        })?
        .get(0);

    // The roles a rule of this data table can be written by or granted to: its
    // own, plus the connection they were all created from.
    let mut own_pg_roles = vec![admin_pg_role.clone()];
    own_pg_roles.extend(
        database_permissions_by_key(db, &database_key)
            .await?
            .map(|r| r.permissions)
            .filter(|p| p.enabled)
            .into_iter()
            .flat_map(|p| p.roles.into_values())
            .filter_map(|role| role.pg_rolename),
    );
    let default_acl_rules = read_default_acl_rules(&client, &own_pg_roles).await?;

    Ok((
        client,
        AdminConnection {
            dbname,
            database_key,
            admin_pg_role,
            pg_roles: PgRoleInventory { existing, adoptable },
            public_schema_is_open,
            default_acl_rules,
        },
    ))
}

/// Plan `req` against the database the data table reaches, whose permissions
/// are `old` (read under the caller's lock, or absent when none are on yet) and
/// belong to `owner_w_id` — the workspace whose principals the tenants name,
/// which is the calling one unless a superadmin manages them from elsewhere.
async fn build_plan(
    db: &DB,
    w_id: &str,
    owner_w_id: &str,
    datatable_name: &str,
    old: Option<&DataTablePermissions>,
    req: &SetDatatablePermissions,
) -> Result<(tokio_postgres::Client, AdminConnection, RolePlan)> {
    require_datatable_permissions_license().await?;
    // Validated against the database the entry names before anything connects to
    // it: a stale tenant or a stranded migration is refused without a round trip,
    // and the connection is then checked to have reached that same database.
    let (_, _, key) = resolve_datatable_database_unchecked(db, w_id, datatable_name).await?;
    ensure_save_names_what_exists(db, owner_w_id, &key, old, req).await?;
    let (client, conn) = connect_as_admin_unchecked(db, w_id, datatable_name).await?;
    let mut plan = crate::datatable_permissions_oss::plan_role_changes(
        &conn.database_key,
        &conn.dbname,
        &conn.admin_pg_role,
        old,
        req,
        &conn.pg_roles,
        conn.public_schema_is_open,
        &conn.default_acl_rules,
    )?;
    if !req.enabled {
        // Opting out is never refused; what it strands is said out loud.
        let roles: HashSet<&str> = old
            .map(|p| p.roles.keys().map(String::as_str))
            .into_iter()
            .flatten()
            .filter(|name| *name != ADMIN_DATATABLE_ROLE)
            .collect();
        let stranded = migrations_naming(db, &conn.database_key, &roles).await?;
        if !stranded.is_empty() {
            plan.warnings.push(format!(
                "Migration(s) {} name a role in a `-- role` annotation; they will not run until \
                 the annotation is removed.",
                stranded.join(", ")
            ));
        }
    }
    Ok((client, conn, plan))
}

/// The connection and statements that drop every role of the database a data
/// table reaches, giving their objects back to admin first, with the key of that
/// database.
///
/// Resolved separately from being run: resolving needs the data table's config,
/// which a deletion may be about to remove, while running is irreversible and
/// must not happen until that deletion has committed.
pub(crate) type PlannedRoleDrop = (tokio_postgres::Client, RolePlan, String);

/// Plan the removal of every Postgres role of the database a data table reaches,
/// for a database that is going away with the data table — a workspace being
/// deleted, a fork's clone being dropped — when `w_id` owns its permissions. A
/// workspace that merely reaches the database, a fork holding a copy above all,
/// has no say over roles another workspace turned on.
///
/// A database that is already unreachable must not block the deletion, so a
/// failure here is logged and the deletion goes ahead without a plan — leaving
/// roles that only a `DROP ROLE` by hand will clear.
pub(crate) async fn plan_drop_of_datatable_roles(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
) -> Option<PlannedRoleDrop> {
    let req = SetDatatablePermissions {
        enabled: false,
        roles: vec![],
        default_role: None,
        renames: vec![],
    };
    let res = async {
        let (_, _, key) = resolve_datatable_database_unchecked(db, w_id, datatable_name).await?;
        let Some(record) = database_permissions_by_key(db, &key).await?.filter(|r| {
            r.owner_workspace_id.as_deref() == Some(w_id)
                && r.permissions.enabled
                && r.permissions.roles.len() > 1
        }) else {
            return Ok::<_, Error>(None);
        };
        let (client, _, plan) = build_plan(
            db,
            w_id,
            w_id,
            datatable_name,
            Some(&record.permissions),
            &req,
        )
        .await?;
        Ok(Some((client, plan, key)))
    }
    .await;
    match res {
        Ok(planned) => planned,
        Err(e) => {
            tracing::error!(
                "Could not plan dropping the Postgres roles behind data table {datatable_name} in {w_id}: {e:#}"
            );
            None
        }
    }
}

/// Destroy the roles a committed save stopped naming.
///
/// Asked under the database's permissions row, one role at a time: a role the
/// row names again — a save that put the role back — is left alone, and one it
/// does not name is dropped whatever else has changed in the meantime. Nobody
/// else can be planning against these between the question and the answer,
/// since that row is what every save takes first.
///
/// Best-effort: a role outliving its row is recoverable, dropping one a live
/// role depends on is not.
async fn drop_roles_the_record_no_longer_names(
    db: &DB,
    database_key: &str,
    client: &mut tokio_postgres::Client,
    statements: &[&PlannedStatement],
) -> Result<()> {
    let mut tx = db.begin().await?;
    let in_use: HashSet<String> = lock_database_permissions(&mut tx, database_key)
        .await?
        .map(|r| r.permissions)
        .filter(|p| p.enabled)
        .into_iter()
        .flat_map(|p| p.roles.into_values())
        .filter_map(|role| role.pg_rolename)
        .collect();
    let to_run: Vec<&PlannedStatement> = statements
        .iter()
        .filter(|s| {
            s.drops_role
                .as_ref()
                .is_none_or(|role| !in_use.contains(role))
        })
        .copied()
        .collect();
    if !to_run.is_empty() {
        let mut attempted: Vec<&str> = to_run
            .iter()
            .filter_map(|s| s.drops_role.as_deref())
            .collect();
        attempted.sort();
        attempted.dedup();
        let ran = async {
            // The row is held for as long as these run, and they run on a database
            // Windmill does not control — a lock held there, or a role with a great
            // deal to reassign, would otherwise stall every save behind it.
            client
                .batch_execute("SET statement_timeout = '60s'")
                .await
                .map_err(|e| {
                    Error::internal_err(format!(
                        "Failed to bound the role changes: {}",
                        pg_error_message(&e)
                    ))
                })?;
            run_statements(client, &to_run).await
        }
        .await;
        // Named here rather than by the caller, and on every way out: which of
        // them were skipped because the row names them again is only known under
        // the lock above, and whatever failed, the save that stopped naming these
        // has committed and nothing comes back for them.
        ran.map_err(|e| {
            Error::ExecutionErr(format!("{e}. Roles left behind: {}", attempted.join(", ")))
        })?;
    }
    tx.commit().await?;
    Ok(())
}

/// Drop the roles a plan names, leaving the database's permissions row in place:
/// with its logins gone every role is refused and `admin` stays the owning
/// workspace's alone, which is the safe state for a database that was meant to go
/// and did not — and for one whose owner is gone, which is what a workspace
/// deletion leaves behind. Returns whether the roles were dropped.
///
/// Run under the row's lock, so no save plans against these roles meanwhile.
/// The plan was made before the deletion committed; the row is read again here
/// and the plan runs only while the row is still `expected_owner`'s — none, after
/// a workspace deletion — so a superadmin who adopted the row from another
/// workspace in between keeps the roles their save now names.
pub(crate) async fn run_planned_drop_keeping_record(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
    expected_owner: Option<&str>,
    (mut client, plan, database_key): PlannedRoleDrop,
) -> bool {
    let statements: Vec<&PlannedStatement> = plan.statements.iter().collect();
    let ran = async {
        let mut tx = db.begin().await?;
        let record = lock_database_permissions(&mut tx, &database_key).await?;
        if record
            .as_ref()
            .is_some_and(|r| r.owner_workspace_id.as_deref() != expected_owner)
        {
            tracing::warn!(
                "The roles behind data table {datatable_name} in {w_id} were adopted by workspace {:?} before they could be dropped; left in place",
                record.and_then(|r| r.owner_workspace_id)
            );
            return Ok(false);
        }
        client
            .batch_execute("SET statement_timeout = '60s'")
            .await
            .map_err(|e| {
                Error::internal_err(format!(
                    "Failed to bound the role changes: {}",
                    pg_error_message(&e)
                ))
            })?;
        run_statements(&mut client, &statements).await?;
        tx.commit().await?;
        Ok::<bool, Error>(true)
    }
    .await;
    match ran {
        Ok(dropped) => dropped,
        Err(e) => {
            tracing::error!(
                "Could not drop the Postgres roles behind data table {datatable_name} in {w_id}: {e:#}"
            );
            false
        }
    }
}

/// Forget a database's permissions: for a database that is gone, roles and all.
///
/// Authorization: performs none. Callers MUST be acting on a database they have
/// just dropped, on behalf of a caller authorized to drop it.
pub(crate) async fn forget_database_permissions(db: &DB, database_key: &str) {
    let forgotten = async {
        let mut tx = db.begin().await?;
        lock_database_permissions(&mut tx, database_key).await?;
        delete_database_permissions(&mut tx, database_key).await?;
        tx.commit().await?;
        Ok::<(), Error>(())
    }
    .await;
    if let Err(e) = forgotten {
        tracing::error!("Could not forget the permissions of {database_key}: {e:#}");
    }
}

/// Run a plan's statements in a single transaction, so a failure part-way leaves
/// the database exactly as it was.
async fn run_statements(
    client: &mut tokio_postgres::Client,
    statements: &[&PlannedStatement],
) -> Result<()> {
    let pg_tx = client.transaction().await.map_err(|e| {
        Error::internal_err(format!(
            "Failed to open a transaction on the data table: {}",
            pg_error_message(&e)
        ))
    })?;
    for statement in statements.iter() {
        pg_tx.batch_execute(&statement.sql).await.map_err(|e| {
            Error::ExecutionErr(format!(
                "Failed to run `{}`: {}",
                statement.display,
                pg_error_message(&e)
            ))
        })?;
    }
    pg_tx.commit().await.map_err(|e| {
        Error::internal_err(format!(
            "Failed to commit the role changes: {}",
            pg_error_message(&e)
        ))
    })
}

/// The permissions the caller may manage from `w_id` for the database a data
/// table reaches: an admin of the owning workspace, or a superadmin. A record
/// that does not exist yet is created by the workspace that opts in — not from a
/// fork, whose data table is either a copy of a database another workspace owns
/// or a clone the fork can drop, roles and all.
async fn ensure_can_manage_permissions(
    db: &DB,
    authed: &ApiAuthed,
    w_id: &str,
    record: Option<&DatabasePermissions>,
    enabling: bool,
) -> Result<()> {
    require_admin(authed.is_admin, &authed.username)?;
    match record {
        Some(record) if record.owner_workspace_id.as_deref() != Some(w_id) => {
            if !windmill_common::auth::is_super_admin_email(db, &authed.email).await? {
                return Err(Error::NotAuthorized(match &record.owner_workspace_id {
                    Some(owner) => format!(
                        "The permissions of this database are managed from workspace '{owner}', \
                         which turned them on."
                    ),
                    None => "The workspace that turned this database's permissions on was \
                             deleted; only a superadmin can change them now."
                        .to_string(),
                }));
            }
        }
        Some(_) => {}
        None => {
            if enabling && crate::workspaces_extra::workspace_is_fork(db, w_id).await? {
                return Err(Error::BadRequest(
                    "Data table permissions cannot be enabled from a fork workspace: its data \
                     table points either at the database of the workspace it was forked from, \
                     which is where to set them, or at a copy the fork can drop."
                        .to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn permissions_info(
    record: Option<&DatabasePermissions>,
    editable: bool,
) -> DatatablePermissionsInfo {
    let permissions = record.map(|r| r.permissions.clone()).unwrap_or_default();
    DatatablePermissionsInfo {
        enabled: permissions.enabled,
        default_role: permissions.default_role().to_string(),
        roles: permissions
            .roles
            .into_iter()
            .map(|(name, role)| DatatableRoleInfo {
                name,
                tenants: role.tenants,
                pg_rolename: role.pg_rolename,
            })
            .collect(),
        owner_workspace_id: record.and_then(|r| r.owner_workspace_id.clone()),
        editable,
    }
}

async fn get_datatable_permissions(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
) -> JsonResult<DatatablePermissionsInfo> {
    require_admin(authed.is_admin, &authed.username)?;
    let (_, _, key) = resolve_datatable_database_unchecked(&db, &w_id, &datatable_name).await?;
    let record = database_permissions_by_key(&db, &key).await?;
    let editable = ensure_can_manage_permissions(&db, &authed, &w_id, record.as_ref(), false)
        .await
        .is_ok();
    Ok(Json(permissions_info(record.as_ref(), editable)))
}

/// Refuse a data table operation that would run as a role `authed` may not use.
///
/// The executor enforces this too, so this is not the security boundary — it is
/// what turns "the migration job failed" into an error naming the role and the
/// migration, before anything is pushed or recorded.
pub(crate) async fn ensure_can_use_datatable_role(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
    role: Option<&str>,
    authed: &ApiAuthed,
    context: &str,
) -> Result<()> {
    let (_, _, key) = resolve_datatable_database_unchecked(db, w_id, datatable_name).await?;
    let Some(record) = database_permissions_by_key(db, &key)
        .await?
        .filter(|r| r.permissions.enabled)
    else {
        // Unpermissioned: only the built-in role exists, and everyone reaches it.
        return match role {
            Some(role) if role != ADMIN_DATATABLE_ROLE => Err(Error::BadRequest(format!(
                "{context} names role '{role}', but permissions are not enabled on data table '{datatable_name}'"
            ))),
            _ => Ok(()),
        };
    };
    let role_name = role.unwrap_or_else(|| record.permissions.default_role());
    let entry = record.permissions.roles.get(role_name).ok_or_else(|| {
        Error::NotFound(format!(
            "{context} names role '{role_name}', which is not defined on data table '{datatable_name}'"
        ))
    })?;
    let allowed = can_use_datatable_role_in_owner_workspace(
        db,
        record.owner_workspace_id.as_deref(),
        w_id,
        entry,
        &DatatableAccess::Authed(authed.to_authed_ref()),
    )
    .await?;
    if !allowed {
        return Err(Error::NotAuthorized(format!(
            "{context} runs as role '{role_name}' of data table '{datatable_name}', which you are not allowed to use"
        )));
    }
    Ok(())
}

/// The roles of a database `access`, made from `w_id`, may run as.
pub(crate) async fn usable_roles(
    db: &DB,
    w_id: &str,
    record: &DatabasePermissions,
    access: &DatatableAccess<'_>,
) -> Result<Vec<String>> {
    let mut usable = Vec::new();
    for (name, role) in record.permissions.roles.iter() {
        if can_use_datatable_role_in_owner_workspace(
            db,
            record.owner_workspace_id.as_deref(),
            w_id,
            role,
            access,
        )
        .await?
        {
            usable.push(name.clone());
        }
    }
    Ok(usable)
}

/// Refuse a save that names something that no longer exists: a tenant whose
/// user, group or folder is gone, or a role that stored migrations still name
/// and the save no longer defines.
///
/// The drawer sends the whole role list it loaded, so a save can carry a tenant
/// that another admin's deletion took off the role in between (deleting the
/// principal removes its tenant, see `remove_datatable_tenant`): written back,
/// whoever is given that username next would inherit the role. Refusing is what
/// makes the stale save visible; the admin reloads and saves again. `w_id` is
/// the workspace whose principals the tenants are — the owning one.
///
/// A migration carries its role as a `-- role <name>` annotation in its own code,
/// which neither a rename nor a removal rewrites, so its next run — or the down
/// script of one already applied — would name a role the data table no longer
/// has.
///
/// Turning permissions off is exempt: it ignores the submitted roles and is the
/// escape hatch that drops them. What it leaves behind is a warning on the plan.
async fn ensure_save_names_what_exists(
    db: &DB,
    w_id: &str,
    database_key: &str,
    old: Option<&DataTablePermissions>,
    req: &SetDatatablePermissions,
) -> Result<()> {
    if !req.enabled {
        return Ok(());
    }
    let mut users = Vec::new();
    let mut groups = Vec::new();
    let mut folders = Vec::new();
    for tenant in req.roles.iter().flat_map(|r| r.tenants.iter()) {
        match tenant.split_once('/') {
            Some(("u", user)) => users.push(user.to_string()),
            Some(("g", group)) => groups.push(group.to_string()),
            Some(("f", folder)) => folders.push(folder.to_string()),
            _ => {}
        }
    }
    let mut missing = Vec::new();
    if !users.is_empty() {
        let found = sqlx::query_scalar!(
            r#"SELECT username AS "username!" FROM usr WHERE workspace_id = $1 AND username = ANY($2)"#,
            w_id,
            &users[..],
        )
        .fetch_all(db)
        .await?;
        missing.extend(
            users
                .iter()
                .filter(|u| !found.contains(u))
                .map(|u| format!("u/{u}")),
        );
    }
    if !groups.is_empty() {
        let found = sqlx::query_scalar!(
            r#"SELECT name AS "name!" FROM group_ WHERE workspace_id = $1 AND name = ANY($2)"#,
            w_id,
            &groups[..],
        )
        .fetch_all(db)
        .await?;
        missing.extend(
            groups
                .iter()
                .filter(|g| !found.contains(g))
                .map(|g| format!("g/{g}")),
        );
    }
    if !folders.is_empty() {
        let found = sqlx::query_scalar!(
            r#"SELECT name AS "name!" FROM folder WHERE workspace_id = $1 AND name = ANY($2)"#,
            w_id,
            &folders[..],
        )
        .fetch_all(db)
        .await?;
        missing.extend(
            folders
                .iter()
                .filter(|f| !found.contains(f))
                .map(|f| format!("f/{f}")),
        );
    }
    if !missing.is_empty() {
        missing.sort();
        missing.dedup();
        return Err(Error::BadRequest(format!(
            "Tenant(s) {} no longer exist in this workspace. Reload the roles and save again.",
            missing.join(", ")
        )));
    }

    // Renamed away or removed: every old name the save no longer defines.
    let kept: HashSet<&str> = req.roles.iter().map(|r| r.name.as_str()).collect();
    // `admin` resolves to the data table's own connection whether or not it is
    // named, so a migration naming it never strands.
    let gone: HashSet<&str> = old
        .map(|old| old.roles.keys().map(String::as_str))
        .into_iter()
        .flatten()
        .filter(|name| !kept.contains(name) && *name != ADMIN_DATATABLE_ROLE)
        .collect();
    let blocking = migrations_naming(db, database_key, &gone).await?;
    if !blocking.is_empty() {
        return Err(Error::BadRequest(format!(
            "Migration(s) {} name a role this save removes or renames, in a `-- role` \
             annotation the save does not rewrite. Update the migration(s) first, or keep \
             the role.",
            blocking.join(", ")
        )));
    }
    Ok(())
}

/// The stored migrations, of every data table entry on the instance that
/// reaches the database `database_key` names, whose `-- role` annotation names
/// one of `roles`, as `<workspace>/<data table>: '<migration>' (role '<role>')`.
///
/// Roles are the database's, so a migration of another entry reaching it — in
/// this workspace or a fork's copy — is stranded by a removal exactly as this
/// entry's own would be. Only migrations that could carry an annotation are
/// read — the filter is looser than the parser, never tighter, so a spelling the
/// executor honours is never missed — and only the entries those name are
/// resolved; an entry that does not resolve reaches nothing.
async fn migrations_naming(
    db: &DB,
    database_key: &str,
    roles: &HashSet<&str>,
) -> Result<Vec<String>> {
    if roles.is_empty() {
        return Ok(vec![]);
    }
    let migrations = sqlx::query!(
        r#"SELECT workspace_id AS "workspace_id!", datatable AS "datatable!", name AS "name!",
                  code_up AS "code_up!", code_down
           FROM datatable_migrations
           WHERE code_up LIKE '%--%role%' OR code_down LIKE '%--%role%'
           ORDER BY workspace_id, datatable, timestamp"#,
    )
    .fetch_all(db)
    .await?;
    let mut naming = Vec::new();
    let mut reaches: std::collections::HashMap<(String, String), bool> =
        std::collections::HashMap::new();
    for m in migrations {
        let names = [Some(m.code_up.as_str()), m.code_down.as_deref()]
            .into_iter()
            .flatten()
            .filter_map(SqlAnnotations::datatable_role)
            .filter(|role| roles.contains(role.as_str()))
            .collect::<HashSet<String>>();
        if names.is_empty() {
            continue;
        }
        let entry = (m.workspace_id.clone(), m.datatable.clone());
        let reached = match reaches.get(&entry) {
            Some(reached) => *reached,
            None => {
                let reached =
                    resolve_datatable_database_unchecked(db, &m.workspace_id, &m.datatable)
                        .await
                        .map(|(_, _, key)| key == database_key)
                        .unwrap_or(false);
                reaches.insert(entry, reached);
                reached
            }
        };
        if !reached {
            continue;
        }
        for role in names {
            naming.push(format!(
                "{}/{}: '{}' (role '{role}')",
                m.workspace_id, m.datatable, m.name
            ));
        }
    }
    Ok(naming)
}

/// List the roles `authed` may run this data table as. An unpermissioned data
/// table reports `enabled: false` and no roles, so a picker can hide itself.
async fn list_usable_datatable_roles(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
) -> JsonResult<UsableDatatableRoles> {
    let (_, _, key) = resolve_datatable_database_unchecked(&db, &w_id, &datatable_name).await?;
    let Some(record) = database_permissions_by_key(&db, &key)
        .await?
        .filter(|r| r.permissions.enabled)
    else {
        return Ok(Json(UsableDatatableRoles {
            enabled: false,
            roles: vec![],
            default_role: ADMIN_DATATABLE_ROLE.to_string(),
        }));
    };
    let roles = usable_roles(
        &db,
        &w_id,
        &record,
        &DatatableAccess::Authed(authed.to_authed_ref()),
    )
    .await?;
    Ok(Json(UsableDatatableRoles {
        enabled: true,
        default_role: record.permissions.default_role().to_string(),
        roles,
    }))
}

async fn preview_datatable_permissions(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Json(mut req): Json<SetDatatablePermissions>,
) -> JsonResult<DatatablePermissionsPreview> {
    forget_client_login_names(&mut req);
    let (_, _, key) = resolve_datatable_database_unchecked(&db, &w_id, &datatable_name).await?;
    let record = database_permissions_by_key(&db, &key).await?;
    // Refused here too: offering a plan that the save will not run is its own
    // kind of wrong.
    ensure_can_manage_permissions(&db, &authed, &w_id, record.as_ref(), req.enabled).await?;
    let owner_w_id = record
        .as_ref()
        .and_then(|r| r.owner_workspace_id.clone())
        .unwrap_or_else(|| w_id.clone());
    let (_client, _, plan) = build_plan(
        &db,
        &w_id,
        &owner_w_id,
        &datatable_name,
        record.as_ref().map(|r| &r.permissions),
        &req,
    )
    .await?;
    Ok(Json(DatatablePermissionsPreview {
        statements: plan.statements.into_iter().map(|s| s.display).collect(),
        warnings: plan.warnings,
    }))
}

async fn set_datatable_permissions(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Json(mut req): Json<SetDatatablePermissions>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    forget_client_login_names(&mut req);
    let (_, _, key) = resolve_datatable_database_unchecked(&db, &w_id, &datatable_name).await?;

    // Reading the permissions, planning against them, running the plan and
    // persisting them are one operation: interleaved with another save, or with
    // the removal of a principal some role names as a tenant, this would store
    // roles it computed before the other committed. The database's row is what
    // everything touching its permissions takes, so taking it here is what
    // serializes them — whether or not the row exists yet.
    let mut tx = db.begin().await?;
    lock_database_permissions_key(&mut tx, &key).await?;
    let record = database_permissions_by_key(&mut *tx, &key).await?;
    ensure_can_manage_permissions(&db, &authed, &w_id, record.as_ref(), req.enabled).await?;
    // A row without an owner — its workspace deleted — is adopted by the
    // workspace a superadmin saves it from.
    let owner_workspace_id = record
        .as_ref()
        .and_then(|r| r.owner_workspace_id.clone())
        .unwrap_or_else(|| w_id.clone());
    // The tenants are the owning workspace's principals: its lock is what a
    // principal's deletion takes before stripping them, first save or not.
    lock_datatable_permissions_unchecked(&mut tx, &owner_workspace_id).await?;

    // The roles about to be created are handed privileges by this connection,
    // which cannot pass on what it holds without the grant option.
    ensure_instance_db_can_delegate(&db, &w_id, &datatable_name).await;

    // The plan is rebuilt here rather than trusted from the preview: the client
    // never gets to choose what runs against the database.
    let (mut client, conn, plan) = build_plan(
        &db,
        &w_id,
        &owner_workspace_id,
        &datatable_name,
        record.as_ref().map(|r| &r.permissions),
        &req,
    )
    .await?;
    // The plan was built against the database the entry resolves to now; the
    // row it is about to be written under is the one locked above. An entry
    // pointed elsewhere in between would leave the database it left open and
    // the one it reached ungoverned.
    if conn.database_key != key {
        return Err(Error::BadRequest(format!(
            "Data table '{datatable_name}' was pointed at another database while its \
             permissions were being saved. Reload and save again."
        )));
    }

    // Creating and renaming roles is committed before the row: a Windmill-side
    // failure after this point leaves roles the row does not know about, which
    // the next plan adopts (it reads `pg_roles`), whereas the reverse order would
    // leave the row naming roles that were never created. Dropping one has no
    // such way back, so those wait below — except where this save gives the freed
    // name to another role, which only works in one order.
    let keeps_the_name = |statement: &PlannedStatement| {
        statement.drops_role.as_ref().is_some_and(|dropped| {
            plan.permissions
                .roles
                .values()
                .filter_map(|r| r.pg_rolename.as_ref())
                .any(|kept| kept == dropped)
        })
    };
    let (deferred, immediate): (Vec<&PlannedStatement>, Vec<&PlannedStatement>) = plan
        .statements
        .iter()
        .partition(|s| s.drops_role.is_some() && !keeps_the_name(s));
    run_statements(&mut client, &immediate).await?;

    if req.enabled {
        upsert_database_permissions(&mut tx, &key, &owner_workspace_id, &plan.permissions).await?;
    } else {
        delete_database_permissions(&mut tx, &key).await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        "workspaces.set_datatable_permissions",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some(
            [
                ("datatable", datatable_name.as_str()),
                ("database", key.as_str()),
                ("enabled", if req.enabled { "true" } else { "false" }),
            ]
            .into(),
        ),
    )
    .await?;

    tx.commit().await?;

    // Turned on just now: a trigger streaming this database was opened while
    // nothing governed it, and is made to ask again.
    if req.enabled && record.is_none() {
        if let Err(e) = restart_triggers_reaching(&db, &key).await {
            tracing::error!("Could not restart the triggers replicating {key}: {e:#}");
        }
    }

    // What the row no longer names, now that it says so. A failure here is the
    // end of the line for these logins: the save that stopped naming them has
    // committed, so no later plan diffs against them and nothing will try again.
    // Say which ones, since dropping them is now a database administrator's job.
    if !deferred.is_empty() {
        drop_roles_the_record_no_longer_names(&db, &key, &mut client, &deferred)
            .await
            .map_err(|e| {
                Error::ExecutionErr(format!(
                    "The permissions were saved, but some roles could not be dropped: {e}"
                ))
            })?;
    }

    Ok(format!(
        "Permissions of data table {datatable_name} updated"
    ))
}

/// Restore exported permissions on the databases this workspace's data tables
/// reach and nobody governs yet, owned by this workspace. Roles and tenants
/// only: every role is refused until an admin saves the drawer again, which
/// creates the logins under names the save generates; a database that is
/// already governed is left as it is and reported, by the data table that
/// reaches it.
async fn import_datatable_permissions(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(rows): Json<Vec<ImportedDatabasePermissions>>,
) -> JsonResult<Vec<String>> {
    require_admin(authed.is_admin, &authed.username)?;
    require_datatable_permissions_license().await?;
    if crate::workspaces_extra::workspace_is_fork(&db, &w_id).await? {
        return Err(Error::BadRequest(
            "Data table permissions cannot be imported into a fork workspace.".to_string(),
        ));
    }
    let mut skipped = Vec::new();
    for row in rows {
        let (_, _, database_key) =
            resolve_datatable_database_unchecked(&db, &w_id, &row.datatable).await?;
        let mut tx = db.begin().await?;
        lock_database_permissions_key(&mut tx, &database_key).await?;
        if database_permissions_by_key(&mut *tx, &database_key)
            .await?
            .is_some()
        {
            skipped.push(row.datatable);
            continue;
        }
        let mut permissions = row.permissions;
        validate_imported_permissions(&permissions)?;
        for role in permissions.roles.values_mut() {
            role.pg_rolename = None;
            role.pg_password = None;
        }
        if !permissions.roles.contains_key(ADMIN_DATATABLE_ROLE) {
            permissions
                .roles
                .insert(ADMIN_DATATABLE_ROLE.to_string(), Default::default());
        }
        lock_datatable_permissions_unchecked(&mut tx, &w_id).await?;
        upsert_database_permissions(&mut tx, &database_key, &w_id, &permissions).await?;
        audit_log(
            &mut *tx,
            &authed,
            "workspaces.import_datatable_permissions",
            ActionKind::Create,
            &w_id,
            Some(&authed.email),
            Some(
                [
                    ("datatable", row.datatable.as_str()),
                    ("database", database_key.as_str()),
                ]
                .into(),
            ),
        )
        .await?;
        tx.commit().await?;
    }
    Ok(Json(skipped))
}

/// A login name is a cluster-wide identifier the planner renames and drops; the
/// request shape carries the field because it is also the response shape, and
/// the planner reads a role's login from the stored row alone. Never from here.
fn forget_client_login_names(req: &mut SetDatatablePermissions) {
    for role in req.roles.iter_mut() {
        role.pg_rolename = None;
    }
}

/// The shape a save would have refused: role and tenant names as the planner
/// and the tenant matcher read them.
fn validate_imported_permissions(permissions: &DataTablePermissions) -> Result<()> {
    for (name, role) in permissions.roles.iter() {
        if name.is_empty()
            || name.len() > 63
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(Error::BadRequest(format!("Invalid role name '{name}'")));
        }
        for tenant in role.tenants.iter() {
            let valid = tenant == windmill_common::workspaces::DATATABLE_TENANT_WILDCARD
                || matches!(
                    tenant.split_once('/'),
                    Some(("u" | "g" | "f", rest)) if !rest.is_empty()
                );
            if !valid {
                return Err(Error::BadRequest(format!(
                    "Invalid tenant '{tenant}' on role '{name}'"
                )));
            }
        }
    }
    if let Some(default_role) = permissions.default_role.as_deref() {
        if !permissions.roles.contains_key(default_role) {
            return Err(Error::BadRequest(format!(
                "Default role '{default_role}' is not one of the roles"
            )));
        }
    }
    Ok(())
}

/// Refuse to clone a data table whose database has role permissions: the copy
/// lands in a new database, keyed on its own, where nothing governs it — and a
/// fork cannot turn permissions on — so every member of the fork would read, in
/// full, the data the roles existed to divide.
pub(crate) async fn refuse_clone_of_governed_datatable(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
) -> Result<()> {
    let (_, _, key) = resolve_datatable_database_unchecked(db, w_id, datatable_name).await?;
    if database_permissions_by_key(db, &key)
        .await?
        .is_some_and(|r| r.permissions.enabled)
    {
        return Err(Error::BadRequest(format!(
            "Data table '{datatable_name}' has role permissions enabled and cannot be cloned: \
             the copy would be a database of its own that nothing governs, readable in full by \
             every member of the fork. Keep the original, which shares the roles."
        )));
    }
    Ok(())
}

/// Make every Postgres trigger replicating a data table that reaches `database_key`
/// reconnect, so the admin check runs against the permissions just turned on: a
/// stream opened while the database was unpermissioned would otherwise keep
/// receiving every change. Clearing the listener's claim is what stops it; the
/// next claim resolves the resource again, and refuses where it must.
pub(crate) async fn restart_triggers_reaching(db: &DB, database_key: &str) -> Result<()> {
    let triggers = sqlx::query!(
        r#"SELECT workspace_id, path, postgres_resource_path
           FROM postgres_trigger WHERE postgres_resource_path LIKE 'datatable://%'"#
    )
    .fetch_all(db)
    .await?;
    let mut reaches: std::collections::HashMap<(String, String), bool> =
        std::collections::HashMap::new();
    for t in triggers {
        let Some(datatable) = t.postgres_resource_path.strip_prefix("datatable://") else {
            continue;
        };
        let (datatable, _) = windmill_common::workspaces::parse_datatable_ref(datatable);
        let entry = (t.workspace_id.clone(), datatable.to_string());
        let reached = match reaches.get(&entry) {
            Some(reached) => *reached,
            None => {
                let reached = resolve_datatable_database_unchecked(db, &t.workspace_id, datatable)
                    .await
                    .map(|(_, _, key)| key == database_key)
                    .unwrap_or(false);
                reaches.insert(entry, reached);
                reached
            }
        };
        if reached {
            sqlx::query!(
                "UPDATE postgres_trigger SET server_id = NULL, last_server_ping = NULL
                 WHERE workspace_id = $1 AND path = $2",
                t.workspace_id,
                t.path
            )
            .execute(db)
            .await?;
        }
    }
    Ok(())
}
