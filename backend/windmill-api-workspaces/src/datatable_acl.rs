/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Ownership and grants on the objects of an instance data table.
//!
//! [`datatable_permissions`](crate::datatable_permissions) decides who may connect as which role;
//! this decides what each role may then touch. Every change is a real `GRANT`, `REVOKE`,
//! `ALTER ... OWNER TO` or `ALTER DEFAULT PRIVILEGES`, so Postgres is what enforces it.
//!
//! Reading is open to anyone who reaches the data table. Planning and applying are for those who
//! administer it — admins of the workspace that governs it, and superadmins — and the planner
//! itself is Enterprise Edition ([`crate::datatable_acl_oss`]).

use std::collections::BTreeMap;

use axum::{
    extract::{Extension, Path, Query},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_postgres::error::{DbError, SqlState};
use tokio_postgres::AsyncMessage;

use windmill_api_auth::ApiAuthed;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::datatable_roles::{
    lock_role_catalog, read_role_catalog, read_role_catalog_tx, DatatableRoleCatalog,
    ADMIN_DATATABLE_ROLE, CUSTOM_INSTANCE_USER,
};
use windmill_common::error::{pg_error_message, Error, JsonResult, Result};
use windmill_common::workspaces::{
    get_datatable_resource_from_db_unchecked, resolve_governing_datatable, GoverningDatatable,
};
use windmill_common::{PgDatabase, DB};

use crate::datatable_permissions::{ensure_governs_datatable, ensure_reaches_datatable};

pub(crate) fn routes() -> Router {
    Router::new()
        .route("/datatable_acl/{datatable_name}", get(get_datatable_acl))
        .route(
            "/datatable_acl/{datatable_name}/plan",
            post(plan_datatable_acl),
        )
        .route(
            "/datatable_acl/{datatable_name}/apply",
            post(apply_datatable_acl),
        )
}

/// What a read or a change is about.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AclTarget {
    /// The data table's own database — where the privilege to create schemas lives.
    Database,
    Schema {
        schema: String,
    },
    Table {
        schema: String,
        table: String,
    },
}

impl AclTarget {
    /// The schema the target is in, absent for the database itself.
    pub(crate) fn schema(&self) -> Option<&str> {
        match self {
            AclTarget::Database => None,
            AclTarget::Schema { schema } => Some(schema),
            AclTarget::Table { schema, .. } => Some(schema),
        }
    }

    /// What it is called in a message.
    pub(crate) fn label(&self, dbname: &str) -> String {
        match self {
            AclTarget::Database => dbname.to_string(),
            AclTarget::Schema { schema } => schema.clone(),
            AclTarget::Table { schema, table } => format!("{schema}.{table}"),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct AclTargetQuery {
    kind: String,
    schema: Option<String>,
    table: Option<String>,
}

impl TryFrom<AclTargetQuery> for AclTarget {
    type Error = Error;
    fn try_from(q: AclTargetQuery) -> Result<Self> {
        match (q.kind.as_str(), q.schema, q.table) {
            ("database", _, _) => Ok(AclTarget::Database),
            ("schema", Some(schema), _) => Ok(AclTarget::Schema { schema }),
            ("table", Some(schema), Some(table)) => Ok(AclTarget::Table { schema, table }),
            ("schema" | "table", None, _) => {
                Err(Error::BadRequest("This target needs a schema".to_string()))
            }
            ("table", _, None) => Err(Error::BadRequest(
                "A table target needs a table".to_string(),
            )),
            (kind, _, _) => Err(Error::BadRequest(format!("Unknown ACL target '{kind}'"))),
        }
    }
}

/// Where a set of privileges applies, relative to the target.
///
/// `Future*` covers what does not exist yet: those become `ALTER DEFAULT PRIVILEGES`, which only
/// binds objects created by the roles it names.
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GrantScope {
    /// The target itself — the database, the schema, or the table.
    Target,
    AllTables,
    AllSequences,
    AllFunctions,
    FutureTables,
    FutureSequences,
    FutureFunctions,
}

impl GrantScope {
    pub(crate) fn is_future(&self) -> bool {
        matches!(
            self,
            GrantScope::FutureTables | GrantScope::FutureSequences | GrantScope::FutureFunctions
        )
    }
}

/// A change to plan. One at a time: each is confirmed against its own SQL.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AclChange {
    /// Hand the target — and, for a schema, everything already in it — to another role.
    SetOwner {
        role: String,
    },
    Grant {
        role: String,
        privileges: Vec<String>,
        scope: GrantScope,
    },
    Revoke {
        role: String,
        privileges: Vec<String>,
        scope: GrantScope,
        /// Objects inside the target, empty for the target itself. `ON ALL TABLES` grants read
        /// back per object, so they are revoked per object — and the same privileges on several
        /// of them are revoked together.
        #[serde(default)]
        objects: Vec<AclObject>,
    },
}

impl AclChange {
    /// The role the change is about, as the editor names it.
    fn role(&self) -> &str {
        match self {
            AclChange::SetOwner { role }
            | AclChange::Grant { role, .. }
            | AclChange::Revoke { role, .. } => role,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct AclChangeRequest {
    pub target: AclTarget,
    pub change: AclChange,
    /// The statements the plan showed. An apply runs only those: it plans again and refuses if the
    /// result differs.
    #[serde(default)]
    pub statements: Option<Vec<String>>,
}

/// An object inside a schema, named the way `REVOKE ... ON <keyword>` needs it.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct AclObject {
    pub name: String,
    /// `TABLE`, `SEQUENCE`, `FUNCTION` or `PROCEDURE`: what the object is. [`object_keyword`]
    /// turns it into the keyword a revoke takes.
    pub kind: String,
    /// A routine is identified by its argument types, not by its name: two `f` in one schema are
    /// two objects. Absent for everything else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
}

/// A grant as the database has it, under the role names the editor uses.
#[derive(Serialize, Debug, PartialEq)]
pub struct AclGrant {
    /// A data table role's name, `admin` for `custom_instance_user`, else the raw Postgres role
    /// (`PUBLIC` included).
    pub grantee: String,
    pub privileges: Vec<String>,
    /// `None` for the target itself, else the object inside it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<AclObject>,
    /// `TABLES` / `SEQUENCES` / `FUNCTIONS` when this is a default privilege, which applies to
    /// objects that do not exist yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub future: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct DatatableAclInfo {
    /// Under the same names as [`AclGrant::grantee`].
    pub owner: String,
    /// The roles a change may name: `admin`, then every role of the instance catalog. Only for a
    /// caller who may change anything, as the catalog is in the permissions drawer.
    pub roles: Vec<String>,
    /// Whether this caller may plan and apply changes: they administer the data table, on an
    /// edition that has the planner.
    pub editable: bool,
    /// Whether the server is Postgres 17 or later, which added the `MAINTAIN` table privilege.
    pub supports_maintain: bool,
    /// The database the target lives in, which no target carries itself.
    pub dbname: String,
    pub grants: Vec<AclGrant>,
    /// What the target holds that is a target of its own: a database's schemas, a schema's tables.
    pub children: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct AclPlan {
    pub statements: Vec<String>,
    pub warnings: Vec<String>,
}

/// The Postgres role a role name stands for. A data table role is a login named exactly like the
/// role, so this is the identity — except `admin`, which is `custom_instance_user`.
///
/// Anything else is refused, never resolved to some default: every statement a plan writes names
/// the role it is about.
pub(crate) fn pg_role_of(name: &str, catalog: &DatatableRoleCatalog) -> Result<String> {
    if name == ADMIN_DATATABLE_ROLE {
        return Ok(CUSTOM_INSTANCE_USER.to_string());
    }
    if catalog.values().any(|r| r.name == name) {
        return Ok(name.to_string());
    }
    Err(Error::BadRequest(format!(
        "'{name}' is not a data table role of this instance"
    )))
}

/// The reverse of [`pg_role_of`], for display. A role that is not a data table role reads back as
/// itself.
pub(crate) fn role_name_of(pg_role: &str) -> String {
    if pg_role == CUSTOM_INSTANCE_USER {
        ADMIN_DATATABLE_ROLE.to_string()
    } else {
        pg_role.to_string()
    }
}

/// Every role a change may name: `admin` first, then the catalog.
fn role_names(catalog: &DatatableRoleCatalog) -> Vec<String> {
    let mut names: Vec<String> = catalog.values().map(|r| r.name.clone()).collect();
    names.sort();
    names.insert(0, ADMIN_DATATABLE_ROLE.to_string());
    names
}

fn ensure_instance(governing: &GoverningDatatable) -> Result<()> {
    if governing.is_instance() {
        return Ok(());
    }
    Err(Error::BadRequest(format!(
        "Data table '{}' is backed by a Postgres resource, so its access is managed on that \
         server directly. Only a data table on the Windmill instance's own database has data \
         table roles to grant to.",
        governing.name
    )))
}

/// The data table's `admin` connection, and the notices Postgres sends on it.
///
/// Authorization: connects as `custom_instance_user` with the instance's own credentials and checks
/// nothing. Callers MUST have authorized the request first — a request about to be refused must
/// not get as far as this connection.
async fn connect_as_admin_unchecked(
    db: &DB,
    governing: &GoverningDatatable,
) -> Result<(
    tokio_postgres::Client,
    mpsc::UnboundedReceiver<DbError>,
    String,
)> {
    let resource =
        get_datatable_resource_from_db_unchecked(db, &governing.workspace_id, &governing.name)
            .await?;
    let pg: PgDatabase = serde_json::from_value(resource)
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {e}")))?;
    let dbname = pg.dbname.clone();
    let (client, mut connection) = pg.connect(Some(db)).await?;
    // Unbounded: the driver must never wait on the receiver, which only drains once the statement
    // the driver is carrying has completed.
    let (notices_tx, notices) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        loop {
            match std::future::poll_fn(|cx| connection.poll_message(cx)).await {
                Some(Ok(AsyncMessage::Notice(notice))) => {
                    let _ = notices_tx.send(notice);
                }
                Some(Ok(_)) => {}
                Some(Err(e)) => {
                    tracing::error!("Datatable ACL connection error: {e}");
                    break;
                }
                None => break,
            }
        }
    });
    Ok((client, notices, dbname))
}

/// An object whose ownership follows the schema's.
#[derive(Debug, PartialEq)]
pub(crate) struct OwnedObject {
    /// The keyword `ALTER ... OWNER TO` takes for this kind of object.
    pub(crate) keyword: &'static str,
    /// How Postgres names the object (`pg_identify_object`): schema-qualified, quoted where
    /// needed, with a routine's arguments or an operator class's access method. It goes into the
    /// statement as it is.
    pub(crate) identity: String,
}

/// Everything a schema's change of owner takes along, in schema `$1`, as (kind, identity, owner
/// oid), the first two as `pg_identify_object` gives them. The plan, the check of what this
/// connection may move and the check after the move all read this one list, so what is moved and
/// what is checked cannot differ.
///
/// Read from what depends on the schema rather than catalog by catalog, so no kind of object is
/// left out by omission; array types, row types and indexes depend on another object instead.
/// Left out: an object's internal parts (a range type's constructors and multirange, an identity
/// column's sequence), a sequence tied to a column (it follows its table, and refuses an owner of
/// its own), an extension's members, and what has no `ALTER ... OWNER` at all (an extension, a text
/// search parser or template). Postgres records no owner for the bootstrap superuser, oid 10.
macro_rules! schema_owned_objects {
    () => {
        "SELECT o.type AS kind, o.identity, COALESCE(s.refobjid, 10::oid) AS owner
         FROM pg_depend d
         CROSS JOIN LATERAL pg_identify_object(d.classid, d.objid, 0) o
         LEFT JOIN pg_shdepend s
             ON s.dbid = (SELECT oid FROM pg_database WHERE datname = current_database())
            AND s.classid = d.classid AND s.objid = d.objid AND s.deptype = 'o'
         WHERE d.refclassid = 'pg_namespace'::regclass AND d.deptype = 'n'
           AND d.refobjid = (SELECT oid FROM pg_namespace WHERE nspname = $1)
           AND d.classid <> ALL(ARRAY['pg_extension'::regclass, 'pg_ts_parser'::regclass,
                                      'pg_ts_template'::regclass]::oid[])
           AND NOT EXISTS (
               SELECT 1 FROM pg_depend x
               WHERE x.classid = d.classid AND x.objid = d.objid AND x.objsubid = 0
                 AND (x.deptype IN ('i', 'e')
                      OR (x.deptype = 'a' AND d.classid = 'pg_class'::regclass
                          AND x.refobjsubid <> 0)))"
    };
}

/// The keyword `ALTER ... OWNER TO` takes for a kind of object, as `pg_identify_object` names the
/// kind. A kind missing here is refused rather than skipped, which would leave it behind.
fn owned_keyword(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "table" => "TABLE",
        "view" => "VIEW",
        "materialized view" => "MATERIALIZED VIEW",
        "sequence" => "SEQUENCE",
        "foreign table" => "FOREIGN TABLE",
        "type" => "TYPE",
        "function" | "procedure" | "aggregate" => "ROUTINE",
        "collation" => "COLLATION",
        "conversion" => "CONVERSION",
        "operator" => "OPERATOR",
        "operator class" => "OPERATOR CLASS",
        "operator family" => "OPERATOR FAMILY",
        "statistics object" => "STATISTICS",
        "text search dictionary" => "TEXT SEARCH DICTIONARY",
        "text search configuration" => "TEXT SEARCH CONFIGURATION",
        _ => return None,
    })
}

async fn read_owned_objects(
    client: &tokio_postgres::Client,
    schema: &str,
) -> Result<Vec<OwnedObject>> {
    let rows = client
        .query(
            concat!(
                "SELECT kind, identity FROM (",
                schema_owned_objects!(),
                ") o ORDER BY kind, identity"
            ),
            &[&schema],
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to list the objects of schema '{schema}': {}",
                pg_error_message(&e)
            ))
        })?;
    rows.into_iter()
        .map(|row| {
            let kind: &str = row.get(0);
            let identity: String = row.get(1);
            match owned_keyword(kind) {
                Some(keyword) => Ok(OwnedObject { keyword, identity }),
                None => Err(Error::BadRequest(format!(
                    "{identity} is a {kind}, whose owner cannot be changed from here, and schema \
                     {schema} would change hands without it. Move it to another schema first."
                ))),
            }
        })
        .collect()
}

/// The keyword a `REVOKE ... ON` takes for one object, checked rather than interpolated: it lands
/// in SQL unquoted.
pub(crate) fn object_keyword(kind: &str) -> Result<&'static str> {
    match kind.to_uppercase().as_str() {
        "TABLE" | "VIEW" | "MATERIALIZED VIEW" | "FOREIGN TABLE" => Ok("TABLE"),
        "SEQUENCE" => Ok("SEQUENCE"),
        // `FUNCTION` names no procedure; `ROUTINE` names either.
        "FUNCTION" | "PROCEDURE" | "ROUTINE" => Ok("ROUTINE"),
        other => Err(Error::BadRequest(format!("Unknown object kind '{other}'"))),
    }
}

/// Every object of a schema, named the way the catalog names it.
async fn read_schema_objects(
    client: &tokio_postgres::Client,
    schema: &str,
) -> Result<Vec<AclObject>> {
    let rows = client
        .query(
            "SELECT CASE c.relkind WHEN 'S' THEN 'SEQUENCE' ELSE 'TABLE' END, c.relname, NULL::text
             FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
             WHERE n.nspname = $1 AND c.relkind = ANY(ARRAY['r','p','v','m','S','f']::\"char\"[])
             UNION ALL
             SELECT CASE p.prokind WHEN 'p' THEN 'PROCEDURE' ELSE 'FUNCTION' END, p.proname,
                    pg_get_function_identity_arguments(p.oid)
             FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
             WHERE n.nspname = $1",
            &[&schema],
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to list the objects of schema '{schema}': {}",
                pg_error_message(&e)
            ))
        })?;
    Ok(rows
        .into_iter()
        .map(|row| AclObject { kind: row.get(0), name: row.get(1), args: row.get(2) })
        .collect())
}

/// Replace the objects a revoke names with the catalog's own entry for each.
///
/// A routine is identified by its argument types, and those go into the statement as written —
/// there is no quoting for them — so the request may name an object but never spell one: what
/// reaches the SQL is read back from Postgres. An object that resolves to nothing is refused rather
/// than dropped, since a revoke that silently covers less than it says is worse than an error.
async fn resolve_acl_objects(
    client: &tokio_postgres::Client,
    target: &AclTarget,
    objects: &[AclObject],
) -> Result<Vec<AclObject>> {
    if objects.is_empty() {
        return Ok(vec![]);
    }
    let Some(schema) = target.schema() else {
        return Err(Error::BadRequest(
            "A database has no objects of its own to revoke on".to_string(),
        ));
    };
    let known = read_schema_objects(client, schema).await?;
    objects
        .iter()
        .map(|requested| {
            let keyword = object_keyword(&requested.kind)?;
            known
                .iter()
                .find(|k| {
                    k.name == requested.name
                        && k.args == requested.args
                        && object_keyword(&k.kind).is_ok_and(|k| k == keyword)
                })
                .cloned()
                .ok_or_else(|| {
                    Error::NotFound(format!(
                        "'{}' is not an object of schema '{schema}'",
                        requested.name
                    ))
                })
        })
        .collect()
}

async fn read_owner(client: &tokio_postgres::Client, target: &AclTarget) -> Result<Option<String>> {
    let row = match target {
        AclTarget::Database => {
            client
                .query_opt(
                    "SELECT pg_get_userbyid(datdba) FROM pg_database WHERE datname = current_database()",
                    &[],
                )
                .await
        }
        AclTarget::Schema { schema } => {
            client
                .query_opt(
                    // `public` is owned by `pg_database_owner`, a placeholder role whose membership
                    // is whoever owns the database — naming it back would say nothing, so resolve
                    // it to that owner.
                    "SELECT pg_get_userbyid(owner) FROM (
                         SELECT CASE WHEN n.nspowner = (SELECT oid FROM pg_roles WHERE rolname = 'pg_database_owner')
                                     THEN (SELECT d.datdba FROM pg_database d WHERE d.datname = current_database())
                                     ELSE n.nspowner END AS owner
                         FROM pg_namespace n WHERE n.nspname = $1
                     ) o",
                    &[schema],
                )
                .await
        }
        AclTarget::Table { schema, table } => {
            client
                .query_opt(
                    "SELECT pg_get_userbyid(c.relowner)
                     FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = $1 AND c.relname = $2",
                    &[schema, table],
                )
                .await
        }
    }
    .map_err(|e| Error::internal_err(format!("Failed to read the owner: {}", pg_error_message(&e))))?;
    Ok(row.map(|row| row.get(0)))
}

async fn read_children(client: &tokio_postgres::Client, target: &AclTarget) -> Result<Vec<String>> {
    let rows = match target {
        AclTarget::Database => {
            client
                .query(
                    "SELECT nspname::text FROM pg_namespace
                     WHERE nspname <> 'information_schema' AND nspname NOT LIKE 'pg\\_%'
                     ORDER BY nspname",
                    &[],
                )
                .await
        }
        AclTarget::Schema { schema } => {
            client
                .query(
                    "SELECT c.relname::text
                     FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = $1 AND c.relkind = ANY(ARRAY['r','p']::\"char\"[])
                     ORDER BY c.relname",
                    &[schema],
                )
                .await
        }
        AclTarget::Table { .. } => return Ok(vec![]),
    }
    .map_err(|e| {
        Error::internal_err(format!(
            "Failed to list what the target holds: {}",
            pg_error_message(&e)
        ))
    })?;
    Ok(rows.into_iter().map(|row| row.get(0)).collect())
}

async fn get_datatable_acl(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Query(query): Query<AclTargetQuery>,
) -> JsonResult<DatatableAclInfo> {
    let target: AclTarget = query.try_into()?;
    ensure_reaches_datatable(&db, &w_id, &datatable_name, &authed).await?;
    let governing = resolve_governing_datatable(&db, &w_id, &datatable_name).await?;
    ensure_instance(&governing)?;
    let editable = crate::datatable_acl_oss::ensure_acl_planner().is_ok()
        && ensure_governs_datatable(&db, &authed, &w_id, &governing)
            .await
            .is_ok();
    let roles = if editable {
        role_names(&read_role_catalog(&db).await?)
    } else {
        vec![]
    };

    let (client, _notices, dbname) = connect_as_admin_unchecked(&db, &governing).await?;
    let owner = read_owner(&client, &target)
        .await?
        .ok_or_else(|| Error::NotFound(format!("{} not found", target.label(&dbname))))?;
    let grants = read_grants(&client, &target).await?;
    let supports_maintain: bool = client
        .query_one(
            "SELECT current_setting('server_version_num')::int >= 170000",
            &[],
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to read the server version: {}",
                pg_error_message(&e)
            ))
        })?
        .get(0);
    let children = read_children(&client, &target).await?;

    Ok(Json(DatatableAclInfo {
        owner: role_name_of(&owner),
        roles,
        editable,
        supports_maintain,
        dbname,
        grants,
        children,
    }))
}

async fn read_grants(client: &tokio_postgres::Client, target: &AclTarget) -> Result<Vec<AclGrant>> {
    // `aclexplode` turns an acl array into one row per (grantee, privilege); grantee 0 is PUBLIC,
    // which has no name to resolve. A NULL acl is not "no access" but Postgres's built-in default —
    // the owner holds everything and, on a routine, PUBLIC may EXECUTE — hence `acldefault`. The
    // owner's own entries are left out: what it holds comes with ownership, which the owner shows,
    // not with a grant a revoke here could take back.
    let mut rows = match target {
        AclTarget::Database => client
            .query(
                "SELECT CASE WHEN a.grantee = 0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,
                        a.privilege_type, NULL::text, NULL::text, NULL::text, NULL::text
                 FROM pg_database d, aclexplode(COALESCE(d.datacl, acldefault('d', d.datdba))) a
                 WHERE d.datname = current_database() AND a.grantee <> d.datdba",
                &[],
            )
            .await
            .map_err(grant_read_error)?,
        AclTarget::Schema { schema } => {
            let mut out = client
                .query(
                    "SELECT CASE WHEN a.grantee = 0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,
                            a.privilege_type, NULL::text, NULL::text, NULL::text, NULL::text
                     FROM pg_namespace n, aclexplode(COALESCE(n.nspacl, acldefault('n', n.nspowner))) a
                     WHERE n.nspname = $1 AND a.grantee <> n.nspowner",
                    &[schema],
                )
                .await
                .map_err(grant_read_error)?;
            out.extend(
                client
                    .query(
                        "SELECT CASE WHEN a.grantee = 0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,
                                a.privilege_type, c.relname, NULL::text,
                                CASE c.relkind WHEN 'S' THEN 'SEQUENCE' ELSE 'TABLE' END,
                                NULL::text
                         FROM pg_class c
                         JOIN pg_namespace n ON n.oid = c.relnamespace,
                              aclexplode(COALESCE(c.relacl, acldefault(
                                  CASE c.relkind WHEN 'S' THEN 's' ELSE 'r' END::\"char\", c.relowner))) a
                         WHERE n.nspname = $1
                           AND c.relkind = ANY(ARRAY['r','p','v','m','S','f']::\"char\"[])
                           AND a.grantee <> c.relowner",
                        &[schema],
                    )
                    .await
                    .map_err(grant_read_error)?,
            );
            out.extend(
                client
                    .query(
                        // Routines carry their own acl in `pg_proc`; without this a grant made here
                        // would vanish on the next read and could never be revoked back.
                        "SELECT CASE WHEN a.grantee = 0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,
                                a.privilege_type, p.proname, NULL::text,
                                CASE p.prokind WHEN 'p' THEN 'PROCEDURE' ELSE 'FUNCTION' END,
                                pg_get_function_identity_arguments(p.oid)
                         FROM pg_proc p
                         JOIN pg_namespace n ON n.oid = p.pronamespace,
                              aclexplode(COALESCE(p.proacl, acldefault('f', p.proowner))) a
                         WHERE n.nspname = $1 AND a.grantee <> p.proowner",
                        &[schema],
                    )
                    .await
                    .map_err(grant_read_error)?,
            );
            out.extend(
                client
                    .query(
                        "SELECT CASE WHEN a.grantee = 0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,
                                a.privilege_type, NULL::text,
                                CASE d.defaclobjtype
                                    WHEN 'r' THEN 'TABLES' WHEN 'S' THEN 'SEQUENCES'
                                    WHEN 'f' THEN 'FUNCTIONS' ELSE 'TYPES' END, NULL::text, NULL::text
                         FROM pg_default_acl d
                         JOIN pg_namespace n ON n.oid = d.defaclnamespace,
                              aclexplode(d.defaclacl) a
                         WHERE n.nspname = $1",
                        &[schema],
                    )
                    .await
                    .map_err(grant_read_error)?,
            );
            out
        }
        AclTarget::Table { schema, table } => client
            .query(
                "SELECT CASE WHEN a.grantee = 0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,
                        a.privilege_type, NULL::text, NULL::text, NULL::text, NULL::text
                 FROM pg_class c
                 JOIN pg_namespace n ON n.oid = c.relnamespace,
                      aclexplode(COALESCE(c.relacl, acldefault(
                          CASE c.relkind WHEN 'S' THEN 's' ELSE 'r' END::\"char\", c.relowner))) a
                 WHERE n.nspname = $1 AND c.relname = $2 AND a.grantee <> c.relowner",
                &[schema, table],
            )
            .await
            .map_err(grant_read_error)?,
    };

    // One row per privilege — and, for default privileges, one per creating role. Fold them back
    // into one entry per grantee and object.
    let mut folded: BTreeMap<
        (
            String,
            Option<(String, String, Option<String>)>,
            Option<String>,
        ),
        Vec<String>,
    > = BTreeMap::new();
    for row in rows.drain(..) {
        let grantee: String = row.get(0);
        let privilege: String = row.get(1);
        let object: Option<String> = row.get(2);
        let future: Option<String> = row.get(3);
        let object_kind: Option<String> = row.get(4);
        let object_args: Option<String> = row.get(5);
        folded
            .entry((
                role_name_of(&grantee),
                object.map(|name| {
                    (
                        name,
                        object_kind.unwrap_or_else(|| "TABLE".to_string()),
                        object_args,
                    )
                }),
                future,
            ))
            .or_default()
            .push(privilege);
    }
    Ok(folded
        .into_iter()
        .map(|((grantee, object, future), mut privileges)| {
            privileges.sort();
            privileges.dedup();
            AclGrant {
                grantee,
                privileges,
                object: object.map(|(name, kind, args)| AclObject { name, kind, args }),
                future,
            }
        })
        .collect())
}

fn grant_read_error(e: tokio_postgres::Error) -> Error {
    Error::internal_err(format!("Failed to read grants: {}", pg_error_message(&e)))
}

/// Changing a data table's access is administering it. Checked in full before anything connects
/// with the instance's credentials — the edition included, since without the planner every
/// request ends in the same refusal.
async fn authorize_acl_change(
    db: &DB,
    authed: &ApiAuthed,
    w_id: &str,
    datatable_name: &str,
) -> Result<GoverningDatatable> {
    let governing = resolve_governing_datatable(db, w_id, datatable_name).await?;
    ensure_governs_datatable(db, authed, w_id, &governing).await?;
    ensure_instance(&governing)?;
    crate::datatable_acl_oss::ensure_acl_planner()?;
    Ok(governing)
}

/// Plan one change against the catalog and the database as they are now.
async fn build_plan(
    client: &tokio_postgres::Client,
    dbname: &str,
    catalog: &DatatableRoleCatalog,
    target: &AclTarget,
    change: &AclChange,
) -> Result<AclPlan> {
    let role = change.role();
    // `admin` is the login the data table itself reaches Postgres through, and the one every
    // change here runs as: a revoke that lands leaves nothing able to grant it back.
    if matches!(change, AclChange::Revoke { .. }) && role == ADMIN_DATATABLE_ROLE {
        return Err(Error::BadRequest(format!(
            "'{ADMIN_DATATABLE_ROLE}' is how this data table reaches its database; \
             its own access is not revocable from here"
        )));
    }
    let pg_role = pg_role_of(role, catalog)?;
    let change = match change {
        AclChange::Revoke { role, privileges, scope, objects } => AclChange::Revoke {
            role: role.clone(),
            privileges: privileges.clone(),
            scope: *scope,
            objects: resolve_acl_objects(client, target, objects).await?,
        },
        change => change.clone(),
    };
    // Default privileges are recorded per creating role, and a schema's new owner is kept in reach
    // of what the others create there, so both are written for every role there is.
    let other_pg_roles = role_names(catalog)
        .iter()
        .map(|name| pg_role_of(name, catalog))
        .filter(|r| r.as_ref().map_or(true, |r| *r != pg_role))
        .collect::<Result<Vec<_>>>()?;
    let existing_objects = match (&change, target) {
        (AclChange::SetOwner { .. }, AclTarget::Schema { schema }) => {
            read_owned_objects(client, schema).await?
        }
        _ => vec![],
    };
    if matches!(change, AclChange::SetOwner { .. }) {
        if let Some((object, owner)) = unmanaged_owner(client, target).await? {
            return Err(Error::BadRequest(format!(
                "{object} is owned by {owner}, which this data table's connection cannot act \
                 for, so its owner cannot be changed from here"
            )));
        }
    }
    let mut plan = crate::datatable_acl_oss::plan_statements(
        target,
        &change,
        dbname,
        &pg_role,
        &other_pg_roles,
        &existing_objects,
    )?;
    if matches!(change, AclChange::SetOwner { .. }) {
        if let Some(missing) = missing_owner_privilege(client, target, &pg_role).await? {
            plan.warnings.push(format!(
                "{role} does not have {missing}, which Postgres requires of a new owner, so this \
                 will be refused. Grant it first."
            ));
        }
    }
    Ok(plan)
}

/// Postgres only hands an object to a role that could have created it: a table to one with
/// `CREATE` on its schema, a schema to one with `CREATE` on the database.
async fn missing_owner_privilege(
    client: &tokio_postgres::Client,
    target: &AclTarget,
    pg_role: &str,
) -> Result<Option<String>> {
    let (row, missing) = match target {
        AclTarget::Table { schema, .. } => (
            client
                .query_one(
                    "SELECT has_schema_privilege($1::name, $2::text, 'CREATE')",
                    &[&pg_role, schema],
                )
                .await,
            format!("CREATE on schema {schema}"),
        ),
        AclTarget::Schema { .. } => (
            client
                .query_one(
                    "SELECT has_database_privilege($1::name, current_database(), 'CREATE')",
                    &[&pg_role],
                )
                .await,
            "CREATE on the database".to_string(),
        ),
        AclTarget::Database => return Ok(None),
    };
    let has: bool = row
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to read the new owner's privileges: {}",
                pg_error_message(&e)
            ))
        })?
        .get(0);
    Ok((!has).then_some(missing))
}

/// The first thing a change of owner would move that this connection cannot act for, with its
/// owner. Postgres lets only a member of the current owner move an object, and every change runs as
/// `custom_instance_user`, so an object it does not hold the owner of — `public`, owned by the
/// database's owner, above all — is refused here rather than at apply. Running as the instance's
/// own user instead would reach objects Windmill never created.
async fn unmanaged_owner(
    client: &tokio_postgres::Client,
    target: &AclTarget,
) -> Result<Option<(String, String)>> {
    let read_error = |e: tokio_postgres::Error| {
        Error::internal_err(format!(
            "Failed to read who owns what the change moves: {}",
            pg_error_message(&e)
        ))
    };
    match target {
        AclTarget::Database => Ok(None),
        AclTarget::Table { schema, table } => {
            let row = client
                .query_opt(
                    "SELECT pg_get_userbyid(c.relowner)
                     FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = $1 AND c.relname = $2 AND NOT pg_has_role(c.relowner, 'USAGE')",
                    &[schema, table],
                )
                .await
                .map_err(read_error)?;
            Ok(row.map(|row| (format!("{schema}.{table}"), row.get(0))))
        }
        AclTarget::Schema { schema } => {
            let row = client
                .query_opt(
                    concat!(
                        "SELECT ord, identity, pg_get_userbyid(owner) FROM (
                             SELECT 0 AS ord, NULL::text AS identity, n.nspowner AS owner
                             FROM pg_namespace n WHERE n.nspname = $1
                             UNION ALL
                             SELECT 1, identity, owner FROM (",
                        schema_owned_objects!(),
                        ") m
                         ) o WHERE NOT pg_has_role(owner, 'USAGE') ORDER BY ord, identity LIMIT 1"
                    ),
                    &[schema],
                )
                .await
                .map_err(read_error)?;
            Ok(row.map(|row| {
                let label = row
                    .get::<_, Option<String>>(1)
                    .unwrap_or_else(|| format!("schema {schema}"));
                (label, row.get(2))
            }))
        }
    }
}

async fn plan_datatable_acl(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Json(req): Json<AclChangeRequest>,
) -> JsonResult<AclPlan> {
    let governing = authorize_acl_change(&db, &authed, &w_id, &datatable_name).await?;
    let catalog = read_role_catalog(&db).await?;
    let (client, _notices, dbname) = connect_as_admin_unchecked(&db, &governing).await?;
    Ok(Json(
        build_plan(&client, &dbname, &catalog, &req.target, &req.change).await?,
    ))
}

async fn apply_datatable_acl(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Json(req): Json<AclChangeRequest>,
) -> Result<String> {
    let confirmed = req.statements.as_ref().ok_or_else(|| {
        Error::BadRequest(
            "An apply runs exactly the statements its plan showed; plan the change first"
                .to_string(),
        )
    })?;
    // Refuses without taking a lock; everything is checked again once they are held.
    let governing = authorize_acl_change(&db, &authed, &w_id, &datatable_name).await?;

    // Held until the change is committed: a role renamed or dropped meanwhile would change what
    // the plan names, and a settings save could move the entry onto another database. Taken in the
    // same order as the permissions save, so the two cannot deadlock.
    let mut tx = db.begin().await?;
    lock_role_catalog(&mut tx).await?;
    sqlx::query!(
        "SELECT 1 AS one FROM workspace_settings WHERE workspace_id = $1 FOR UPDATE",
        &governing.workspace_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let governing = authorize_acl_change(&db, &authed, &w_id, &datatable_name).await?;
    let catalog = read_role_catalog_tx(&mut tx).await?;

    let (mut client, mut notices, dbname) = connect_as_admin_unchecked(&db, &governing).await?;
    let plan = build_plan(&client, &dbname, &catalog, &req.target, &req.change).await?;
    if &plan.statements != confirmed {
        return Err(Error::BadRequest(
            "The data table or its roles changed since this was planned, so it would no longer \
             run what was confirmed. Plan it again."
                .to_string(),
        ));
    }

    // Postgres only lets a role pass on a privilege it holds with grant option, and an instance
    // database provisioned before data table roles holds none. Best-effort: a grant this fails to
    // enable is refused below rather than skipped.
    if let Err(e) = windmill_common::ensure_instance_db_grant_options_unchecked(&db, &dbname).await
    {
        tracing::warn!("Could not refresh grant options on '{dbname}': {e}");
    }

    // One transaction: a half-applied ownership transfer leaves one schema's objects owned by two
    // different roles.
    let pg_tx = client.transaction().await.map_err(|e| {
        Error::internal_err(format!(
            "Failed to open a transaction on the data table: {}",
            pg_error_message(&e)
        ))
    })?;
    for statement in &plan.statements {
        pg_tx.batch_execute(statement).await.map_err(|e| {
            Error::ExecutionErr(format!(
                "Failed to run `{statement}`: {}",
                pg_error_message(&e)
            ))
        })?;
        // A privilege the connection cannot pass on is only a warning to Postgres, which then
        // carries on having changed nothing. Returning drops the transaction, rolling back
        // everything before it.
        while let Ok(notice) = notices.try_recv() {
            if *notice.code() == SqlState::WARNING_PRIVILEGE_NOT_GRANTED
                || *notice.code() == SqlState::WARNING_PRIVILEGE_NOT_REVOKED
            {
                return Err(Error::ExecutionErr(format!(
                    "`{statement}` did not take effect ({}), so nothing was applied",
                    notice.message()
                )));
            }
        }
    }

    // A schema's objects were listed before the transaction opened; one created since would stay
    // with its old owner while the schema changes hands.
    if let (AclChange::SetOwner { role }, AclTarget::Schema { schema }) = (&req.change, &req.target)
    {
        let new_owner = pg_role_of(role, &catalog)?;
        let straggler = pg_tx
            .query_opt(
                concat!(
                    "SELECT identity FROM (",
                    schema_owned_objects!(),
                    ") m WHERE owner <> (SELECT oid FROM pg_roles WHERE rolname = $2)
                     ORDER BY 1 LIMIT 1"
                ),
                &[schema, &new_owner],
            )
            .await
            .map_err(|e| {
                Error::internal_err(format!(
                    "Failed to check what the schema holds: {}",
                    pg_error_message(&e)
                ))
            })?;
        if let Some(row) = straggler {
            return Err(Error::BadRequest(format!(
                "{} appeared while this ran and would keep its old owner, so nothing was \
                 applied. Plan it again.",
                row.get::<_, String>(0)
            )));
        }
    }

    let target_label = req.target.label(&dbname);
    audit_log(
        &mut *tx,
        &authed,
        "workspaces.datatable_acl",
        ActionKind::Update,
        &governing.workspace_id,
        Some(&governing.name),
        Some(
            [
                ("target", target_label.as_str()),
                ("change", change_kind(&req.change)),
                ("role", req.change.role()),
            ]
            .into(),
        ),
    )
    .await?;
    pg_tx.commit().await.map_err(|e| {
        Error::internal_err(format!(
            "Failed to commit the changes: {}",
            pg_error_message(&e)
        ))
    })?;
    tx.commit().await?;

    windmill_common::feature_usage::log_feature_usage(
        "datatable",
        "acl_applied",
        change_kind(&req.change),
    );

    Ok(format!("Updated access on {target_label}"))
}

/// What kind of change, never what it named: the telemetry key and the audit's summary.
fn change_kind(change: &AclChange) -> &'static str {
    match change {
        AclChange::SetOwner { .. } => "owner",
        AclChange::Grant { scope, .. } | AclChange::Revoke { scope, .. } if scope.is_future() => {
            "default_privileges"
        }
        AclChange::Grant { .. } => "grant",
        AclChange::Revoke { .. } => "revoke",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windmill_common::datatable_roles::InstanceDatatableRole;

    #[test]
    fn a_role_is_its_own_postgres_role_except_admin() {
        let catalog: DatatableRoleCatalog = BTreeMap::from([(
            "role1".to_string(),
            InstanceDatatableRole { name: "analytics".to_string(), enabled: true, pwd: None },
        )]);
        assert_eq!(
            pg_role_of("admin", &catalog).unwrap(),
            "custom_instance_user"
        );
        assert_eq!(pg_role_of("analytics", &catalog).unwrap(), "analytics");
        assert_eq!(role_name_of("custom_instance_user"), "admin");
        assert_eq!(role_name_of("analytics"), "analytics");
        // A catalog id, a name the catalog lacks, or the admin login spelled out never stands for
        // some other role.
        for unknown in ["role1", "operator", "custom_instance_user", "PUBLIC", ""] {
            assert!(
                matches!(pg_role_of(unknown, &catalog), Err(Error::BadRequest(_))),
                "{unknown}"
            );
        }
    }

    /// A kind of object the list misses stays with its old owner while the schema changes hands,
    /// which only a real catalog shows.
    #[sqlx::test(migrations = false)]
    async fn a_schemas_owner_change_takes_every_object_in_it(pool: sqlx::PgPool) {
        let mut config: tokio_postgres::Config =
            std::env::var("DATABASE_URL").unwrap().parse().unwrap();
        config.dbname(pool.connect_options().get_database().unwrap());
        let (client, connection) = config.connect(tokio_postgres::NoTls).await.unwrap();
        tokio::spawn(connection);
        client
            .batch_execute(
                "CREATE SCHEMA moved;
                 CREATE TABLE moved.t (id serial PRIMARY KEY, a int, b int);
                 CREATE STATISTICS moved.st ON a, b FROM moved.t;
                 CREATE TABLE moved.pt (id int) PARTITION BY RANGE (id);
                 CREATE TABLE moved.pt1 PARTITION OF moved.pt FOR VALUES FROM (0) TO (10);
                 CREATE TYPE moved.r AS RANGE (subtype = float8);
                 CREATE DOMAIN moved.tags AS text[];
                 CREATE COLLATION moved.coll (provider = libc, locale = 'C');",
            )
            .await
            .unwrap();
        let owned = read_owned_objects(&client, "moved").await.unwrap();
        // Not the serial's sequence, the range's constructors and multirange, or any array or row
        // type: each follows the object it belongs to.
        assert_eq!(
            owned
                .iter()
                .map(|o| (o.keyword, o.identity.as_str()))
                .collect::<Vec<_>>(),
            [
                ("COLLATION", "moved.coll"),
                ("STATISTICS", "moved.st"),
                ("TABLE", "moved.pt"),
                ("TABLE", "moved.pt1"),
                ("TABLE", "moved.t"),
                ("TYPE", "moved.r"),
                ("TYPE", "moved.tags"),
            ]
        );
    }
}
