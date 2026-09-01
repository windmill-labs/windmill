/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Ownership and grants on the objects of a permissioned data table.
//!
//! [`datatable_permissions`](crate::datatable_permissions) decides which
//! Postgres roles exist; this decides what they may touch. Both speak in
//! Windmill role names — `admin`, `analyst` — and translate to the generated
//! Postgres roles here, so a caller never has to know one.
//!
//! Everything is expressed against an [`AclTarget`]. Only schemas are reachable
//! from the UI today; tables carry the same shape so the same plan/apply path
//! serves them.

use axum::{
    extract::{Extension, Path, Query},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use windmill_api_auth::ApiAuthed;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::error::{pg_error_message, Error, JsonResult, Result};
use windmill_common::workspaces::{
    can_use_datatable_role, get_datatable_resource_from_db,
    get_datatable_resource_from_db_unchecked, DatatableAccess, ADMIN_DATATABLE_ROLE,
};
use windmill_common::{PgDatabase, DB};

use crate::datatable_permissions::{connect_as_admin, read_datatable};

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

/// What a read or a change is about. `Table` is unused by the UI so far and is
/// here because the SQL only differs in the object it names.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AclTarget {
    /// The data table's own database — where the privilege to create schemas
    /// lives.
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
    /// The role to read as, when it is not the caller's default one.
    role: Option<String>,
}

impl TryFrom<AclTargetQuery> for AclTarget {
    type Error = Error;
    fn try_from(q: AclTargetQuery) -> Result<Self> {
        match (q.kind.as_str(), q.schema.clone(), q.table.clone()) {
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
/// `Future` covers what does not exist yet: those become `ALTER DEFAULT
/// PRIVILEGES`, which only binds objects created by the roles it names.
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GrantScope {
    /// The target itself — the schema, or the table.
    Target,
    AllTables,
    AllSequences,
    AllFunctions,
    FutureTables,
    FutureSequences,
    FutureFunctions,
}

/// A change to plan. One at a time: each is confirmed against its own SQL.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AclChange {
    /// Hand the target — and everything already in it — to another role.
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
        /// Objects inside the target, empty for the target itself. `ON ALL
        /// TABLES` grants read back per object, so they are revoked per object —
        /// and the same privileges on several of them are revoked together.
        #[serde(default)]
        objects: Vec<AclObject>,
    },
}

#[derive(Deserialize, Debug)]
pub struct AclChangeRequest {
    pub target: AclTarget,
    pub change: AclChange,
    /// The role to act as, when it is not the caller's default one.
    #[serde(default)]
    pub role: Option<String>,
}

/// An object inside a schema, named the way `REVOKE ... ON <keyword>` needs it.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct AclObject {
    pub name: String,
    /// `TABLE`, `SEQUENCE`, ... — what the object is, since the keyword differs.
    pub kind: String,
    /// A routine is identified by its argument types, not by its name: two
    /// `f` in one schema are two objects. Absent for everything else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
}

/// A grant as the database has it, in Windmill's vocabulary where it can be.
#[derive(Serialize, Debug, PartialEq)]
pub struct AclGrant {
    /// Windmill role name when the grantee is one of the data table's roles,
    /// else the raw Postgres role (`PUBLIC` included).
    pub grantee: String,
    pub privileges: Vec<String>,
    /// `None` for the target itself, else the object inside it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<AclObject>,
    /// `TABLES` / `SEQUENCES` / `FUNCTIONS` when this is a default privilege,
    /// which applies to objects that do not exist yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub future: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct DatatableAclInfo {
    /// Windmill role name when the owner is one of the data table's roles, else
    /// the raw Postgres role.
    pub owner: String,
    /// The data table's roles, in the order the config has them. All of them:
    /// the list is not private, only what each may reach.
    pub roles: Vec<String>,
    /// The subset the caller may themselves run as. Handing an object to a role
    /// outside this list gives it away.
    pub usable_roles: Vec<String>,
    /// Whether the role this connection is on may change the target at all —
    /// Postgres asks for membership of the owning role.
    pub can_manage: bool,
    /// The Windmill role this connection is on.
    pub current_role: String,
    /// Whether the server is Postgres 17 or later, which added the `MAINTAIN`
    /// table privilege.
    pub supports_maintain: bool,
    /// The database the target lives in, which no target carries itself.
    pub dbname: String,
    pub grants: Vec<AclGrant>,
}

#[derive(Serialize, Debug)]
pub struct AclPlan {
    pub statements: Vec<String>,
    pub warnings: Vec<String>,
}

/// The data table's own connection, which is what `admin` resolves to. Read from
/// the resource rather than from a connection: the caller may well not be able to
/// open one as that role.
async fn admin_pg_role(db: &DB, w_id: &str, datatable_name: &str) -> Result<String> {
    let resource = get_datatable_resource_from_db_unchecked(db, w_id, datatable_name).await?;
    let pg: PgDatabase = serde_json::from_value(resource)
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {e}")))?;
    pg.user
        .ok_or_else(|| Error::internal_err("The data table's connection names no user".to_string()))
}

/// What the caller reaches the data table as. Postgres is what decides whether
/// that role may change an owner or hand out a privilege, so the connection is
/// theirs — not the data table's admin one.
struct CallerConnection {
    dbname: String,
    /// The Postgres role this connection authenticated as.
    current_user: String,
}

async fn connect_as_caller(
    db: &DB,
    authed: &ApiAuthed,
    w_id: &str,
    datatable_name: &str,
    role: Option<&str>,
) -> Result<(tokio_postgres::Client, CallerConnection)> {
    // A permissioned data table hands out a connection per role, and the tenant
    // lists are what say who reaches which. Without permissions every member
    // resolves to the data table's own connection, which owns everything — so
    // there it is the workspace admins' to change, as the roles themselves are.
    if !authed.is_admin
        && !read_datatable(db, w_id, datatable_name)
            .await?
            .permissions
            .is_some_and(|p| p.enabled)
    {
        return Err(Error::NotAuthorized(format!(
            "Only an admin can manage access on data table '{datatable_name}', which has no roles"
        )));
    }
    let resource = get_datatable_resource_from_db(
        db,
        w_id,
        datatable_name,
        role,
        DatatableAccess::Authed(authed.to_authed_ref()),
    )
    .await?;
    let pg_db: PgDatabase = serde_json::from_value(resource)
        .map_err(|e| Error::internal_err(format!("Failed to parse database credentials: {e}")))?;
    let dbname = pg_db.dbname.clone();
    let (client, connection) = pg_db.connect(Some(db)).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("Datatable ACL connection error: {}", e);
        }
    });
    let current_user: String = client
        .query_one("SELECT current_user", &[])
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to read the connection identity: {}",
                pg_error_message(&e)
            ))
        })?
        .get(0);
    Ok((client, CallerConnection { dbname, current_user }))
}

/// The first object a change reaches that the connection's role does not own, if
/// any — the reason a caller may be refused even on a target they do own.
///
/// The scopes that read `IN SCHEMA` cover the whole schema whoever owns what is
/// in it, and `SET OWNER` on a schema moves every object in it; a revoke may
/// also name objects one at a time. `Target` scope on a table or a schema is the
/// target itself, which [`can_manage_target`] has already answered.
async fn first_unmanageable_object(
    client: &tokio_postgres::Client,
    target: &AclTarget,
    change: &AclChange,
) -> Result<Option<String>> {
    let Some(schema) = target.schema() else {
        return Ok(None);
    };

    // A revoke that names its objects reaches exactly those.
    if let AclChange::Revoke { objects, .. } = change {
        if !objects.is_empty() {
            let named: Vec<String> = objects
                .iter()
                .map(|o| match &o.args {
                    Some(args) => format!("{}({args})", o.name),
                    None => o.name.clone(),
                })
                .collect();
            let row = client
                .query_opt(
                    "SELECT name FROM (
                         SELECT c.relname AS name, pg_has_role(c.relowner, 'USAGE') AS mine
                         FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                         WHERE n.nspname = $1 AND c.relname = ANY($2)
                         UNION ALL
                         SELECT p.proname || '(' || pg_get_function_identity_arguments(p.oid) || ')',
                                pg_has_role(p.proowner, 'USAGE')
                         FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
                         WHERE n.nspname = $1
                           AND p.proname || '(' || pg_get_function_identity_arguments(p.oid) || ')'
                               = ANY($2)
                     ) o WHERE NOT o.mine ORDER BY name LIMIT 1",
                    &[&schema, &named],
                )
                .await
                .map_err(|e| {
                    Error::internal_err(format!(
                        "Failed to read the owners of schema '{schema}': {}",
                        pg_error_message(&e)
                    ))
                })?;
            return Ok(row.map(|row| format!("{schema}.{}", row.get::<_, String>(0))));
        }
    }

    // The object classes the change touches, as `relkind`s and whether routines
    // are included — Postgres groups views and foreign tables under TABLES.
    let (relkinds, routines): (&[&str], bool) = match change {
        AclChange::SetOwner { .. } if matches!(target, AclTarget::Schema { .. }) => {
            (&["r", "p", "v", "m", "S", "f"], true)
        }
        AclChange::SetOwner { .. } => return Ok(None),
        AclChange::Grant { scope, .. } | AclChange::Revoke { scope, .. } => match scope {
            GrantScope::AllTables => (&["r", "p", "v", "f"], false),
            GrantScope::AllSequences => (&["S"], false),
            GrantScope::AllFunctions => (&[], true),
            // A future grant creates no statement about an existing object.
            GrantScope::FutureTables
            | GrantScope::FutureSequences
            | GrantScope::FutureFunctions
            | GrantScope::Target => return Ok(None),
        },
    };

    let relkinds: Vec<String> = relkinds.iter().map(|k| k.to_string()).collect();
    let row = client
        .query_opt(
            "SELECT name FROM (
                 SELECT c.relname AS name, pg_has_role(c.relowner, 'USAGE') AS mine
                 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE n.nspname = $1 AND c.relkind::text = ANY($2)
                 UNION ALL
                 SELECT p.proname || '(' || pg_get_function_identity_arguments(p.oid) || ')',
                        pg_has_role(p.proowner, 'USAGE')
                 FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
                 WHERE $3 AND n.nspname = $1
             ) o WHERE NOT o.mine ORDER BY name LIMIT 1",
            &[&schema, &relkinds, &routines],
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to read the owners of schema '{schema}': {}",
                pg_error_message(&e)
            ))
        })?;
    Ok(row.map(|row| format!("{schema}.{}", row.get::<_, String>(0))))
}

/// The data table roles `authed` may themselves run as.
async fn usable_role_names(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
    authed: &ApiAuthed,
) -> Result<Vec<String>> {
    let authed_ref = authed.to_authed_ref();
    let datatable = read_datatable(db, w_id, datatable_name).await?;
    Ok(match datatable.permissions.filter(|p| p.enabled) {
        Some(p) => p
            .roles
            .iter()
            .filter(|(_, role)| can_use_datatable_role(role, &authed_ref))
            .map(|(name, _)| name.clone())
            .collect(),
        None => vec![ADMIN_DATATABLE_ROLE.to_string()],
    })
}

/// Whether the connection's role is a member of the target's owner, which is
/// what "may change its access" means here.
async fn can_manage_target(client: &tokio_postgres::Client, target: &AclTarget) -> Result<bool> {
    let row = match target {
        AclTarget::Database => client
            .query_opt(
                "SELECT pg_has_role(datdba, 'USAGE') FROM pg_database WHERE datname = current_database()",
                &[],
            )
            .await,
        AclTarget::Schema { schema } => client
            .query_opt(
                "SELECT pg_has_role(nspowner, 'USAGE') FROM pg_namespace WHERE nspname = $1",
                &[schema],
            )
            .await,
        AclTarget::Table { schema, table } => client
            .query_opt(
                "SELECT pg_has_role(c.relowner, 'USAGE')
                 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE n.nspname = $1 AND c.relname = $2",
                &[schema, table],
            )
            .await,
    }
    .map_err(|e| {
        Error::internal_err(format!(
            "Failed to read ownership: {}",
            pg_error_message(&e)
        ))
    })?;
    Ok(row.map(|r| r.get::<_, bool>(0)).unwrap_or(false))
}

/// Windmill role name -> Postgres role name, for the roles of one data table.
///
/// `admin` maps to whatever the data table's own connection is, which is not
/// stored in the config: it is read off the connection.
async fn role_map(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
    admin_pg_role: &str,
) -> Result<BTreeMap<String, String>> {
    let datatable = read_datatable(db, w_id, datatable_name).await?;
    let mut map = BTreeMap::new();
    map.insert(ADMIN_DATATABLE_ROLE.to_string(), admin_pg_role.to_string());
    if let Some(permissions) = datatable.permissions.filter(|p| p.enabled) {
        for (name, role) in permissions.roles {
            if let Some(pg_rolename) = role.pg_rolename {
                map.insert(name, pg_rolename);
            }
        }
    }
    Ok(map)
}

fn pg_role_of(roles: &BTreeMap<String, String>, role: &str) -> Result<String> {
    roles
        .get(role)
        .cloned()
        .ok_or_else(|| Error::BadRequest(format!("Unknown role '{role}'")))
}

/// Read back a Postgres role as the Windmill role it belongs to, so the UI never
/// has to show a generated name.
fn windmill_role_of(roles: &BTreeMap<String, String>, pg_role: &str) -> String {
    roles
        .iter()
        .find(|(_, pg)| pg.as_str() == pg_role)
        .map(|(name, _)| name.clone())
        .unwrap_or_else(|| pg_role.to_string())
}

/// An object whose ownership follows the schema's.
#[derive(Debug, PartialEq)]
pub(crate) struct OwnedObject {
    pub(crate) name: String,
    /// The keyword `ALTER ... OWNER TO` takes for this kind of object.
    pub(crate) keyword: &'static str,
    /// Identity arguments of a routine, which is what tells two of the same
    /// name apart. `None` for a relation.
    pub(crate) args: Option<String>,
}

fn keyword_of_relkind(relkind: i8) -> Option<&'static str> {
    match relkind as u8 as char {
        'r' | 'p' => Some("TABLE"),
        'v' => Some("VIEW"),
        'm' => Some("MATERIALIZED VIEW"),
        'S' => Some("SEQUENCE"),
        'f' => Some("FOREIGN TABLE"),
        // Indexes and TOAST tables follow their table; composite types are not
        // reachable through ALTER TABLE ... OWNER TO.
        _ => None,
    }
}

async fn read_owned_objects(
    client: &tokio_postgres::Client,
    schema: &str,
) -> Result<Vec<OwnedObject>> {
    let rows = client
        .query(
            "SELECT c.relname, c.relkind
             FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
             WHERE n.nspname = $1 AND c.relkind = ANY(ARRAY['r','p','v','m','S','f']::\"char\"[])
             ORDER BY c.relname",
            &[&schema],
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to list the objects of schema '{schema}': {}",
                pg_error_message(&e)
            ))
        })?;
    let mut objects: Vec<OwnedObject> = rows
        .into_iter()
        .filter_map(|row| {
            keyword_of_relkind(row.get::<_, i8>(1)).map(|keyword| OwnedObject {
                name: row.get(0),
                keyword,
                args: None,
            })
        })
        .collect();
    // Routines live in `pg_proc`, not `pg_class`, and would keep the previous
    // owner while the schema they are in changes hands. `ALTER ROUTINE` covers
    // functions, procedures and aggregates alike.
    let routines = client
        .query(
            "SELECT p.proname, pg_get_function_identity_arguments(p.oid)
             FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
             WHERE n.nspname = $1
             ORDER BY p.proname",
            &[&schema],
        )
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "Failed to list the routines of schema '{schema}': {}",
                pg_error_message(&e)
            ))
        })?;
    objects.extend(routines.into_iter().map(|row| OwnedObject {
        name: row.get(0),
        keyword: "ROUTINE",
        args: Some(row.get(1)),
    }));
    Ok(objects)
}

/// The keyword a `REVOKE ... ON` takes for one object, checked rather than
/// interpolated: it lands in SQL unquoted.
pub(crate) fn object_keyword(kind: &str) -> Result<&'static str> {
    match kind.to_uppercase().as_str() {
        "TABLE" | "VIEW" | "MATERIALIZED VIEW" | "FOREIGN TABLE" => Ok("TABLE"),
        "SEQUENCE" => Ok("SEQUENCE"),
        "FUNCTION" => Ok("FUNCTION"),
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
             SELECT 'FUNCTION', p.proname, pg_get_function_identity_arguments(p.oid)
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
/// A routine is identified by its argument types, and those go into the
/// statement as written — there is no quoting for them — so the request may name
/// an object but never spell one: what reaches the SQL is read back from
/// Postgres. An object that resolves to nothing is refused rather than dropped,
/// since a revoke that silently covers less than it says is worse than an error.
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

async fn get_datatable_acl(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Query(query): Query<AclTargetQuery>,
) -> JsonResult<DatatableAclInfo> {
    let role = query.role.clone();
    let target: AclTarget = query.try_into()?;
    let (client, conn) =
        connect_as_caller(&db, &authed, &w_id, &datatable_name, role.as_deref()).await?;
    let roles = role_map(
        &db,
        &w_id,
        &datatable_name,
        &admin_pg_role(&db, &w_id, &datatable_name).await?,
    )
    .await?;

    let owner_row = match &target {
        AclTarget::Database => client
            .query_opt(
                "SELECT pg_get_userbyid(datdba), pg_has_role(datdba, 'USAGE')
                 FROM pg_database WHERE datname = current_database()",
                &[],
            )
            .await
            .map_err(|e| {
                Error::internal_err(format!("Failed to read the owner: {}", pg_error_message(&e)))
            })?,
        AclTarget::Schema { schema } => client
            .query_opt(
                // `public` is owned by `pg_database_owner`, a placeholder role
                // whose membership is whoever owns the database — naming it back
                // would say nothing, so resolve it to that owner.
                "SELECT pg_get_userbyid(owner), pg_has_role(owner, 'USAGE') FROM (
                     SELECT CASE WHEN n.nspowner = (SELECT oid FROM pg_roles WHERE rolname = 'pg_database_owner')
                                 THEN (SELECT d.datdba FROM pg_database d WHERE d.datname = current_database())
                                 ELSE n.nspowner END AS owner
                     FROM pg_namespace n WHERE n.nspname = $1
                 ) o",
                &[schema],
            )
            .await
            .map_err(|e| {
                Error::internal_err(format!(
                    "Failed to read the owner: {}",
                    pg_error_message(&e)
                ))
            })?,
        AclTarget::Table { schema, table } => client
            .query_opt(
                "SELECT pg_get_userbyid(c.relowner), pg_has_role(c.relowner, 'USAGE')
                 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE n.nspname = $1 AND c.relname = $2",
                &[schema, table],
            )
            .await
            .map_err(|e| {
                Error::internal_err(format!(
                    "Failed to read the owner: {}",
                    pg_error_message(&e)
                ))
            })?,
    };
    let owner_row = owner_row
        .ok_or_else(|| Error::NotFound(format!("{} not found", target.label(&conn.dbname))))?;
    let owner: String = owner_row.get(0);
    // Membership in the owning role is what Postgres asks for before an ALTER
    // ... OWNER or a GRANT on something you do not own; `admin` holds every role
    // this feature creates, so it passes everywhere. A workspace admin manages
    // the data table itself and is never shut out of it — a schema Windmill did
    // not create, `public` above all, is owned by neither.
    let can_manage: bool = authed.is_admin || owner_row.get::<_, bool>(1);

    let mut grants = read_grants(&client, &target, &roles).await?;
    grants.sort_by(|a, b| {
        let key = |g: &AclGrant| {
            (
                g.grantee.clone(),
                g.object.as_ref().map(|o| o.name.clone()),
                g.future.clone(),
            )
        };
        key(a).cmp(&key(b))
    });

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

    // Every role is named, since the list is not what is private — what each of
    // them may reach is.
    let usable_roles = usable_role_names(&db, &w_id, &datatable_name, &authed).await?;

    Ok(Json(DatatableAclInfo {
        owner: windmill_role_of(&roles, &owner),
        roles: roles.keys().cloned().collect(),
        usable_roles,
        can_manage,
        current_role: windmill_role_of(&roles, &conn.current_user),
        supports_maintain,
        dbname: conn.dbname,
        grants,
    }))
}

async fn read_grants(
    client: &tokio_postgres::Client,
    target: &AclTarget,
    roles: &BTreeMap<String, String>,
) -> Result<Vec<AclGrant>> {
    // `aclexplode` turns an acl array into one row per (grantee, privilege);
    // grantee 0 is PUBLIC, which has no name to resolve.
    let mut rows = match target {
        AclTarget::Database => client
            .query(
                "SELECT CASE WHEN a.grantee = 0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,
                        a.privilege_type, NULL::text, NULL::text, NULL::text, NULL::text
                 FROM pg_database d, aclexplode(d.datacl) a
                 WHERE d.datname = current_database()",
                &[],
            )
            .await
            .map_err(grant_read_error)?,
        AclTarget::Schema { schema } => {
            let mut out = client
                .query(
                    "SELECT CASE WHEN a.grantee = 0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,
                            a.privilege_type, NULL::text, NULL::text, NULL::text, NULL::text
                     FROM pg_namespace n, aclexplode(n.nspacl) a
                     WHERE n.nspname = $1",
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
                              aclexplode(c.relacl) a
                         WHERE n.nspname = $1",
                        &[schema],
                    )
                    .await
                    .map_err(grant_read_error)?,
            );
            out.extend(
                client
                    .query(
                        // Routines carry their own acl in `pg_proc`; without this
                        // a grant made here would vanish on the next read and
                        // could never be revoked back.
                        "SELECT CASE WHEN a.grantee = 0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,
                                a.privilege_type, p.proname, NULL::text, 'FUNCTION',
                                pg_get_function_identity_arguments(p.oid)
                         FROM pg_proc p
                         JOIN pg_namespace n ON n.oid = p.pronamespace,
                              aclexplode(p.proacl) a
                         WHERE n.nspname = $1",
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
                      aclexplode(c.relacl) a
                 WHERE n.nspname = $1 AND c.relname = $2",
                &[schema, table],
            )
            .await
            .map_err(grant_read_error)?,
    };

    // One row per privilege — and, for default privileges, one per creating
    // role. Fold them back into one entry per grantee and object.
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
                windmill_role_of(roles, &grantee),
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

async fn build_acl_plan(
    db: &DB,
    authed: &ApiAuthed,
    w_id: &str,
    datatable_name: &str,
    req: &AclChangeRequest,
) -> Result<(tokio_postgres::Client, AclPlan, String)> {
    crate::datatable_permissions::require_datatable_permissions_license().await?;
    let (client, conn) =
        connect_as_caller(db, authed, w_id, datatable_name, req.role.as_deref()).await?;
    // The objects a revoke names come from the request; the ones it is checked
    // and planned against come from the catalog.
    let change = match &req.change {
        AclChange::Revoke { role, privileges, scope, objects } => AclChange::Revoke {
            role: role.clone(),
            privileges: privileges.clone(),
            scope: *scope,
            objects: resolve_acl_objects(&client, &req.target, objects).await?,
        },
        change => change.clone(),
    };
    // What the caller's own role may change. Postgres cannot enforce the rule we
    // want on its own — handing an object to a role you are not a member of is
    // refused outright, and granting on one you own needs the grant option — so
    // this is the check, and the statements run as the data table's admin below.
    if !authed.is_admin {
        if !can_manage_target(&client, &req.target).await? {
            return Err(Error::NotAuthorized(format!(
                "{} is owned by a role you are not a member of",
                req.target.label(&conn.dbname)
            )));
        }
        // Owning the target is not owning what a change through it reaches: a
        // schema-wide scope names every object in the schema, and handing a
        // schema over takes them all with it. Postgres would have skipped the
        // ones the caller does not own; these statements run as admin, so it
        // will not.
        if let Some(unreachable) = first_unmanageable_object(&client, &req.target, &change).await? {
            return Err(Error::NotAuthorized(format!(
                "This change also covers {unreachable}, which is owned by a role you are not a member of"
            )));
        }
    }
    let roles = role_map(
        db,
        w_id,
        datatable_name,
        &admin_pg_role(db, w_id, datatable_name).await?,
    )
    .await?;
    let role_name = match &req.change {
        AclChange::SetOwner { role } => role,
        AclChange::Grant { role, .. } | AclChange::Revoke { role, .. } => role,
    };
    let pg_role = pg_role_of(&roles, role_name)?;
    // The roles a plan may also write about: what a schema's new owner is kept
    // in reach of, and whose future objects a `created later` grant covers. Both
    // are `ALTER DEFAULT PRIVILEGES FOR ROLE <other>`, which speaks for that role
    // — so a non-admin only gets the ones they may run as.
    let usable = usable_role_names(db, w_id, datatable_name, authed).await?;
    let other_pg_roles: Vec<String> = roles
        .iter()
        .filter(|(name, pg)| {
            pg.as_str() != pg_role.as_str()
                && (authed.is_admin || usable.iter().any(|u| u == name.as_str()))
        })
        .map(|(_, pg)| pg.clone())
        .collect();
    let existing_objects = match (&req.change, &req.target) {
        (AclChange::SetOwner { .. }, AclTarget::Schema { schema }) => {
            read_owned_objects(&client, schema).await?
        }
        _ => vec![],
    };
    let plan = crate::datatable_acl_oss::plan_statements(
        &req.target,
        &change,
        &conn.dbname,
        &pg_role,
        &other_pg_roles,
        &existing_objects,
    )?;
    drop(client);
    let (admin_client, _) = connect_as_admin(db, w_id, datatable_name).await?;
    Ok((admin_client, plan, conn.dbname))
}

async fn plan_datatable_acl(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Json(req): Json<AclChangeRequest>,
) -> JsonResult<AclPlan> {
    let (_client, plan, _dbname) =
        build_acl_plan(&db, &authed, &w_id, &datatable_name, &req).await?;
    Ok(Json(plan))
}

async fn apply_datatable_acl(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, datatable_name)): Path<(String, String)>,
    Json(req): Json<AclChangeRequest>,
) -> Result<String> {
    // Granting is passing a privilege on, which this connection cannot do for a
    // privilege it holds without the grant option.
    crate::datatable_permissions::ensure_instance_db_can_delegate(&db, &w_id, &datatable_name)
        .await;

    let (mut client, plan, dbname) =
        build_acl_plan(&db, &authed, &w_id, &datatable_name, &req).await?;

    // One transaction: a half-applied ownership transfer leaves objects of one
    // schema owned by two different roles.
    let pg_tx = client.transaction().await.map_err(|e| {
        Error::internal_err(format!(
            "Failed to open a transaction on the data table: {}",
            pg_error_message(&e)
        ))
    })?;
    for statement in plan.statements.iter() {
        pg_tx.batch_execute(statement).await.map_err(|e| {
            Error::ExecutionErr(format!(
                "Failed to run `{statement}`: {}",
                pg_error_message(&e)
            ))
        })?;
    }
    pg_tx.commit().await.map_err(|e| {
        Error::internal_err(format!(
            "Failed to commit the changes: {}",
            pg_error_message(&e)
        ))
    })?;

    audit_log(
        &db,
        &authed,
        "datatables.acl",
        ActionKind::Update,
        &w_id,
        Some(&datatable_name),
        Some([("target", format!("{:?}", req.target).as_str())].into()),
    )
    .await?;

    Ok(format!("Updated access on {}", req.target.label(&dbname)))
}
