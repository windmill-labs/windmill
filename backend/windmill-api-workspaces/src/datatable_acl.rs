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

use crate::datatable_permissions::{connect_as_admin, quote_ident, read_datatable};

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
    fn schema(&self) -> Option<&str> {
        match self {
            AclTarget::Database => None,
            AclTarget::Schema { schema } => Some(schema),
            AclTarget::Table { schema, .. } => Some(schema),
        }
    }

    /// How the target reads in the statements that name it, `dbname` being the
    /// database the connection is on — the target never carries it.
    fn object(&self, dbname: &str) -> String {
        match self {
            AclTarget::Database => format!("DATABASE {}", quote_ident(dbname)),
            AclTarget::Schema { schema } => format!("SCHEMA {}", quote_ident(schema)),
            AclTarget::Table { schema, table } => {
                format!("TABLE {}.{}", quote_ident(schema), quote_ident(table))
            }
        }
    }

    /// What it is called in a message.
    fn label(&self, dbname: &str) -> String {
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

impl GrantScope {
    /// The privileges Postgres accepts for what this scope names.
    fn allowed_privileges(&self, target: &AclTarget) -> Result<&'static [&'static str]> {
        Ok(match self {
            GrantScope::Target => match target {
                AclTarget::Database => DATABASE_PRIVILEGES,
                AclTarget::Schema { .. } => SCHEMA_PRIVILEGES,
                AclTarget::Table { .. } => TABLE_PRIVILEGES,
            },
            // Everything below reads `IN SCHEMA`, which a database target has
            // none of: schemas are granted on one at a time.
            _ if matches!(target, AclTarget::Database) => {
                return Err(Error::BadRequest(
                    "A database can only be granted on itself".to_string(),
                ))
            }
            GrantScope::AllTables | GrantScope::FutureTables => TABLE_PRIVILEGES,
            GrantScope::AllSequences | GrantScope::FutureSequences => SEQUENCE_PRIVILEGES,
            GrantScope::AllFunctions | GrantScope::FutureFunctions => FUNCTION_PRIVILEGES,
        })
    }

    fn is_future(&self) -> bool {
        matches!(
            self,
            GrantScope::FutureTables | GrantScope::FutureSequences | GrantScope::FutureFunctions
        )
    }

    /// The plural Postgres uses in `ON ALL <x> IN SCHEMA` and in
    /// `ALTER DEFAULT PRIVILEGES ... ON <x>`.
    fn object_plural(&self) -> Option<&'static str> {
        match self {
            GrantScope::Target => None,
            GrantScope::AllTables | GrantScope::FutureTables => Some("TABLES"),
            GrantScope::AllSequences | GrantScope::FutureSequences => Some("SEQUENCES"),
            GrantScope::AllFunctions | GrantScope::FutureFunctions => Some("FUNCTIONS"),
        }
    }
}

/// `CREATE` on a database is the privilege to create schemas in it.
const DATABASE_PRIVILEGES: &[&str] = &["CONNECT", "CREATE", "TEMPORARY"];
const SCHEMA_PRIVILEGES: &[&str] = &["USAGE", "CREATE"];
const TABLE_PRIVILEGES: &[&str] = &[
    "SELECT",
    "INSERT",
    "UPDATE",
    "DELETE",
    "TRUNCATE",
    "REFERENCES",
    "TRIGGER",
    // Postgres 17. Accepted whatever the server's version, so that a grant read
    // back from a 17 catalog can be revoked; an older server refuses it itself.
    "MAINTAIN",
];
const SEQUENCE_PRIVILEGES: &[&str] = &["USAGE", "SELECT", "UPDATE"];
const FUNCTION_PRIVILEGES: &[&str] = &["EXECUTE"];

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

fn validate_privileges(privileges: &[String], allowed: &[&str]) -> Result<Vec<String>> {
    if privileges.is_empty() {
        return Err(Error::BadRequest("No privilege selected".to_string()));
    }
    privileges
        .iter()
        .map(|p| {
            let upper = p.to_uppercase();
            allowed
                .iter()
                .find(|a| **a == upper)
                .map(|a| a.to_string())
                .ok_or_else(|| {
                    Error::BadRequest(format!(
                        "Privilege '{p}' does not apply here; expected one of {}",
                        allowed.join(", ")
                    ))
                })
        })
        .collect()
}

/// The statements one change plans out, against Postgres role names.
///
/// Pure so the preview the user confirms is the same string that runs.
fn plan_statements(
    target: &AclTarget,
    change: &AclChange,
    dbname: &str,
    pg_role: &str,
    other_pg_roles: &[String],
    existing_objects: &[OwnedObject],
) -> Result<AclPlan> {
    let role = quote_ident(pg_role);
    // Only the scopes that name a schema use this, and those are refused on a
    // database target.
    let schema = target.schema().map(quote_ident).unwrap_or_default();
    let mut statements = Vec::new();
    let mut warnings = Vec::new();

    match change {
        AclChange::SetOwner { .. } => {
            statements.push(format!("ALTER {} OWNER TO {}", target.object(dbname), role));
            for object in existing_objects {
                statements.push(format!(
                    "ALTER {} {} OWNER TO {}",
                    object.keyword,
                    object_ref(&schema, &object.name, object.args.as_deref()),
                    role
                ));
            }
            // Ownership cannot be set ahead of time: an object belongs to
            // whoever creates it. Default privileges are what keeps the owner
            // in reach of what the other roles create from here on — which only
            // means something for a schema, the thing objects are created in.
            for other in other_pg_roles
                .iter()
                .filter(|_| matches!(target, AclTarget::Schema { .. }))
            {
                for plural in ["TABLES", "SEQUENCES", "FUNCTIONS"] {
                    statements.push(format!(
                        "ALTER DEFAULT PRIVILEGES FOR ROLE {} IN SCHEMA {} GRANT ALL PRIVILEGES ON {} TO {}",
                        quote_ident(other),
                        schema,
                        plural,
                        role
                    ));
                }
            }
            if existing_objects.is_empty() && matches!(target, AclTarget::Schema { .. }) {
                warnings.push(format!(
                    "{} holds no objects yet; only the schema itself changes hands.",
                    target.label(dbname)
                ));
            }
        }
        AclChange::Grant { privileges, scope, .. }
        | AclChange::Revoke { privileges, scope, .. } => {
            let revoking = matches!(change, AclChange::Revoke { .. });
            let objects: &[AclObject] = match change {
                AclChange::Revoke { objects, .. } => objects,
                _ => &[],
            };
            // Every object of one revoke is the same kind of thing, so the first
            // decides which privileges are legal for all of them.
            let object = objects.first();
            // A named object decides which privileges are legal, not the scope:
            // `ON ALL TABLES` grants read back per object and revoke per object.
            let allowed = match object {
                Some(o) => match object_keyword(&o.kind)? {
                    "SEQUENCE" => SEQUENCE_PRIVILEGES,
                    "FUNCTION" => FUNCTION_PRIVILEGES,
                    _ => TABLE_PRIVILEGES,
                },
                None => scope.allowed_privileges(target)?,
            };
            let privileges = validate_privileges(privileges, allowed)?;
            let privileges = privileges.join(", ");
            let statement = match (scope.is_future(), scope.object_plural()) {
                (true, Some(plural)) => {
                    // Default privileges are recorded per creating role, so a
                    // rule has to be written for each of them.
                    let mut creators = other_pg_roles.to_vec();
                    creators.push(pg_role.to_string());
                    creators.sort();
                    creators.dedup();
                    for creator in creators {
                        statements.push(if revoking {
                            format!(
                                "ALTER DEFAULT PRIVILEGES FOR ROLE {} IN SCHEMA {} REVOKE {} ON {} FROM {}",
                                quote_ident(&creator), schema, privileges, plural, role
                            )
                        } else {
                            format!(
                                "ALTER DEFAULT PRIVILEGES FOR ROLE {} IN SCHEMA {} GRANT {} ON {} TO {}",
                                quote_ident(&creator), schema, privileges, plural, role
                            )
                        });
                    }
                    None
                }
                (false, Some(plural)) => Some(if revoking {
                    format!(
                        "REVOKE {} ON ALL {} IN SCHEMA {} FROM {}",
                        privileges, plural, schema, role
                    )
                } else {
                    format!(
                        "GRANT {} ON ALL {} IN SCHEMA {} TO {}",
                        privileges, plural, schema, role
                    )
                }),
                (_, None) if !objects.is_empty() => {
                    for object in objects {
                        statements.push(format!(
                            "REVOKE {} ON {} {} FROM {}",
                            privileges,
                            object_keyword(&object.kind)?,
                            object_ref(&schema, &object.name, object.args.as_deref()),
                            role
                        ));
                    }
                    None
                }
                (_, None) => Some(if revoking {
                    format!(
                        "REVOKE {} ON {} FROM {}",
                        privileges,
                        target.object(dbname),
                        role
                    )
                } else {
                    format!(
                        "GRANT {} ON {} TO {}",
                        privileges,
                        target.object(dbname),
                        role
                    )
                }),
            };
            if let Some(statement) = statement {
                statements.push(statement);
            }
            if !revoking && matches!(scope, GrantScope::AllTables | GrantScope::FutureTables) {
                warnings.push(
                    "Reaching a table also needs USAGE on the schema it lives in.".to_string(),
                );
            }
        }
    }

    Ok(AclPlan { statements, warnings })
}

/// The keyword a `REVOKE ... ON` takes for one object, checked rather than
/// interpolated: it lands in SQL unquoted.
fn object_keyword(kind: &str) -> Result<&'static str> {
    match kind.to_uppercase().as_str() {
        "TABLE" | "VIEW" | "MATERIALIZED VIEW" | "FOREIGN TABLE" => Ok("TABLE"),
        "SEQUENCE" => Ok("SEQUENCE"),
        "FUNCTION" => Ok("FUNCTION"),
        other => Err(Error::BadRequest(format!("Unknown object kind '{other}'"))),
    }
}

/// An object whose ownership follows the schema's.
#[derive(Debug, PartialEq)]
struct OwnedObject {
    name: String,
    /// The keyword `ALTER ... OWNER TO` takes for this kind of object.
    keyword: &'static str,
    /// Identity arguments of a routine, which is what tells two of the same
    /// name apart. `None` for a relation.
    args: Option<String>,
}

/// `"schema"."name"` — with `(args)` for a routine, which is not identified
/// without them. `quoted_schema` comes quoted already, being the same for a
/// whole plan.
fn object_ref(quoted_schema: &str, name: &str, args: Option<&str>) -> String {
    format!(
        "{}.{}{}",
        quoted_schema,
        quote_ident(name),
        args.map(|a| format!("({a})")).unwrap_or_default()
    )
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
    let authed_ref = authed.to_authed_ref();
    let datatable = read_datatable(&db, &w_id, &datatable_name).await?;
    let usable_roles: Vec<String> = match datatable.permissions.filter(|p| p.enabled) {
        Some(p) => p
            .roles
            .iter()
            .filter(|(_, role)| can_use_datatable_role(role, &authed_ref))
            .map(|(name, _)| name.clone())
            .collect(),
        None => vec![ADMIN_DATATABLE_ROLE.to_string()],
    };

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
    let (client, conn) =
        connect_as_caller(db, authed, w_id, datatable_name, req.role.as_deref()).await?;
    // What the caller's own role may change. Postgres cannot enforce the rule we
    // want on its own — handing an object to a role you are not a member of is
    // refused outright, and granting on one you own needs the grant option — so
    // this is the check, and the statements run as the data table's admin below.
    if !authed.is_admin && !can_manage_target(&client, &req.target).await? {
        return Err(Error::NotAuthorized(format!(
            "{} is owned by a role you are not a member of",
            req.target.label(&conn.dbname)
        )));
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
    let other_pg_roles: Vec<String> = roles
        .values()
        .filter(|pg| pg.as_str() != pg_role.as_str())
        .cloned()
        .collect();
    let existing_objects = match (&req.change, &req.target) {
        (AclChange::SetOwner { .. }, AclTarget::Schema { schema }) => {
            read_owned_objects(&client, schema).await?
        }
        _ => vec![],
    };
    let plan = plan_statements(
        &req.target,
        &req.change,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> AclTarget {
        AclTarget::Schema { schema: "analytics".to_string() }
    }

    #[test]
    fn set_owner_covers_the_schema_and_what_is_in_it() {
        let objects = vec![
            OwnedObject { name: "orders".to_string(), keyword: "TABLE", args: None },
            OwnedObject { name: "orders_id_seq".to_string(), keyword: "SEQUENCE", args: None },
            OwnedObject {
                name: "total".to_string(),
                keyword: "ROUTINE",
                args: Some("integer, text".to_string()),
            },
        ];
        let plan = plan_statements(
            &schema(),
            &AclChange::SetOwner { role: "analyst".to_string() },
            "dt_probe",
            "wm_analyst_1",
            &["wm_admin".to_string()],
            &objects,
        )
        .unwrap();
        assert_eq!(
            plan.statements[..4],
            [
                r#"ALTER SCHEMA "analytics" OWNER TO "wm_analyst_1""#.to_string(),
                r#"ALTER TABLE "analytics"."orders" OWNER TO "wm_analyst_1""#.to_string(),
                r#"ALTER SEQUENCE "analytics"."orders_id_seq" OWNER TO "wm_analyst_1""#.to_string(),
                // A routine is only named by its arguments.
                r#"ALTER ROUTINE "analytics"."total"(integer, text) OWNER TO "wm_analyst_1""#
                    .to_string(),
            ]
        );
        // What the other roles create later stays within the owner's reach.
        assert!(plan.statements.iter().any(|s| s
            == r#"ALTER DEFAULT PRIVILEGES FOR ROLE "wm_admin" IN SCHEMA "analytics" GRANT ALL PRIVILEGES ON TABLES TO "wm_analyst_1""#));
        assert!(plan.warnings.is_empty());
    }

    #[test]
    fn an_empty_schema_says_so() {
        let plan = plan_statements(
            &schema(),
            &AclChange::SetOwner { role: "analyst".to_string() },
            "dt_probe",
            "wm_analyst_1",
            &[],
            &[],
        )
        .unwrap();
        assert_eq!(plan.statements.len(), 1);
        assert_eq!(plan.warnings.len(), 1);
    }

    #[test]
    fn grants_render_the_scope_they_name() {
        let cases = [
            (
                GrantScope::Target,
                vec!["USAGE".to_string()],
                r#"GRANT USAGE ON SCHEMA "analytics" TO "wm_analyst_1""#,
            ),
            (
                GrantScope::AllTables,
                vec!["SELECT".to_string(), "INSERT".to_string()],
                r#"GRANT SELECT, INSERT ON ALL TABLES IN SCHEMA "analytics" TO "wm_analyst_1""#,
            ),
            (
                GrantScope::AllSequences,
                vec!["USAGE".to_string()],
                r#"GRANT USAGE ON ALL SEQUENCES IN SCHEMA "analytics" TO "wm_analyst_1""#,
            ),
        ];
        for (scope, privileges, expected) in cases {
            let plan = plan_statements(
                &schema(),
                &AclChange::Grant { role: "analyst".to_string(), privileges, scope },
                "dt_probe",
                "wm_analyst_1",
                &[],
                &[],
            )
            .unwrap();
            assert_eq!(plan.statements[0], expected);
        }
    }

    #[test]
    fn future_grants_are_written_for_every_creating_role() {
        let plan = plan_statements(
            &schema(),
            &AclChange::Grant {
                role: "analyst".to_string(),
                privileges: vec!["SELECT".to_string()],
                scope: GrantScope::FutureTables,
            },
            "dt_probe",
            "wm_analyst_1",
            &["wm_admin".to_string()],
            &[],
        )
        .unwrap();
        assert_eq!(
            plan.statements,
            [
                r#"ALTER DEFAULT PRIVILEGES FOR ROLE "wm_admin" IN SCHEMA "analytics" GRANT SELECT ON TABLES TO "wm_analyst_1""#,
                r#"ALTER DEFAULT PRIVILEGES FOR ROLE "wm_analyst_1" IN SCHEMA "analytics" GRANT SELECT ON TABLES TO "wm_analyst_1""#,
            ]
        );
    }

    #[test]
    fn revoke_mirrors_grant() {
        let plan = plan_statements(
            &schema(),
            &AclChange::Revoke {
                role: "analyst".to_string(),
                privileges: vec!["select".to_string()],
                scope: GrantScope::AllTables,
                objects: vec![],
            },
            "dt_probe",
            "wm_analyst_1",
            &[],
            &[],
        )
        .unwrap();
        assert_eq!(
            plan.statements,
            [r#"REVOKE SELECT ON ALL TABLES IN SCHEMA "analytics" FROM "wm_analyst_1""#]
        );
    }

    #[test]
    fn revoking_one_object_names_it() {
        let plan = plan_statements(
            &schema(),
            &AclChange::Revoke {
                role: "analyst".to_string(),
                privileges: vec!["SELECT".to_string()],
                scope: GrantScope::Target,
                objects: vec![AclObject {
                    name: "orders".to_string(),
                    kind: "TABLE".to_string(),
                    args: None,
                }],
            },
            "dt_probe",
            "wm_analyst_1",
            &[],
            &[],
        );
        // SELECT is no schema privilege, but the object named is a table: what
        // it is decides which privileges are legal.
        assert_eq!(
            plan.unwrap().statements,
            [r#"REVOKE SELECT ON TABLE "analytics"."orders" FROM "wm_analyst_1""#]
        );
    }

    #[test]
    fn a_database_grants_the_right_to_create_schemas() {
        let plan = plan_statements(
            &AclTarget::Database,
            &AclChange::Grant {
                role: "analyst".to_string(),
                privileges: vec!["CREATE".to_string()],
                scope: GrantScope::Target,
            },
            "dt_probe",
            "wm_analyst_1",
            &[],
            &[],
        )
        .unwrap();
        assert_eq!(
            plan.statements,
            [r#"GRANT CREATE ON DATABASE "dt_probe" TO "wm_analyst_1""#]
        );

        // A database has no schemas to scope onto, and none of its privileges
        // are a table's.
        for change in [
            AclChange::Grant {
                role: "analyst".to_string(),
                privileges: vec!["CREATE".to_string()],
                scope: GrantScope::AllTables,
            },
            AclChange::Grant {
                role: "analyst".to_string(),
                privileges: vec!["SELECT".to_string()],
                scope: GrantScope::Target,
            },
        ] {
            assert!(matches!(
                plan_statements(
                    &AclTarget::Database,
                    &change,
                    "dt_probe",
                    "wm_analyst_1",
                    &[],
                    &[]
                )
                .unwrap_err(),
                Error::BadRequest(_)
            ));
        }
    }

    #[test]
    fn a_tables_owner_change_is_only_that_table() {
        let plan = plan_statements(
            &AclTarget::Table { schema: "analytics".to_string(), table: "orders".to_string() },
            &AclChange::SetOwner { role: "analyst".to_string() },
            "dt_probe",
            "wm_analyst_1",
            &["wm_admin".to_string()],
            &[],
        )
        .unwrap();
        // Default privileges are about what gets created in a schema, which
        // changing one table's owner says nothing about.
        assert_eq!(
            plan.statements,
            [r#"ALTER TABLE "analytics"."orders" OWNER TO "wm_analyst_1""#]
        );
        assert!(plan.warnings.is_empty());
    }

    #[test]
    fn several_objects_are_revoked_together() {
        let plan = plan_statements(
            &schema(),
            &AclChange::Revoke {
                role: "analyst".to_string(),
                privileges: vec!["SELECT".to_string()],
                scope: GrantScope::Target,
                objects: vec![
                    AclObject { name: "a".to_string(), kind: "TABLE".to_string(), args: None },
                    AclObject { name: "b".to_string(), kind: "TABLE".to_string(), args: None },
                ],
            },
            "dt_probe",
            "wm_analyst_1",
            &[],
            &[],
        )
        .unwrap();
        assert_eq!(
            plan.statements,
            [
                r#"REVOKE SELECT ON TABLE "analytics"."a" FROM "wm_analyst_1""#,
                r#"REVOKE SELECT ON TABLE "analytics"."b" FROM "wm_analyst_1""#,
            ]
        );
    }

    #[test]
    fn a_privilege_the_object_does_not_have_is_refused() {
        for (scope, privilege) in [
            (GrantScope::Target, "SELECT"),
            (GrantScope::AllTables, "CREATE"),
            (GrantScope::AllFunctions, "SELECT"),
            (GrantScope::AllTables, "SELECT; DROP TABLE x"),
        ] {
            let err = plan_statements(
                &schema(),
                &AclChange::Grant {
                    role: "analyst".to_string(),
                    privileges: vec![privilege.to_string()],
                    scope,
                },
                "dt_probe",
                "wm_analyst_1",
                &[],
                &[],
            )
            .unwrap_err();
            assert!(
                matches!(err, Error::BadRequest(_)),
                "{privilege} on {scope:?}"
            );
        }
    }

    #[test]
    fn identifiers_are_quoted() {
        let plan = plan_statements(
            &AclTarget::Schema { schema: "we\"ird".to_string() },
            &AclChange::Grant {
                role: "analyst".to_string(),
                privileges: vec!["USAGE".to_string()],
                scope: GrantScope::Target,
            },
            "dt_probe",
            "ro\"le",
            &[],
            &[],
        )
        .unwrap();
        assert_eq!(
            plan.statements,
            [r#"GRANT USAGE ON SCHEMA "we""ird" TO "ro""le""#]
        );
    }

    #[test]
    fn a_table_target_names_the_table() {
        let plan = plan_statements(
            &AclTarget::Table { schema: "analytics".to_string(), table: "orders".to_string() },
            &AclChange::Grant {
                role: "analyst".to_string(),
                privileges: vec!["SELECT".to_string()],
                scope: GrantScope::Target,
            },
            "dt_probe",
            "wm_analyst_1",
            &[],
            &[],
        )
        .unwrap();
        assert_eq!(
            plan.statements,
            [r#"GRANT SELECT ON TABLE "analytics"."orders" TO "wm_analyst_1""#]
        );
    }
}
