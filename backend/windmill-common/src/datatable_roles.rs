/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! The instance's data table role catalogs.
//!
//! A data table role is a real Postgres login role on one cluster — Windmill's own, or the external
//! instance cluster — named exactly as the user named it, shared by every database Windmill manages
//! on that cluster. Each cluster has its own catalog: a role exists where it was created and nowhere
//! else. Windmill decides who may ask for a role (the per-data-table tenant lists in
//! [`crate::workspaces`]); Postgres decides what the role may then touch. The catalog here is only
//! the first half's vocabulary plus the cluster provisioning.
//!
//! Entries are keyed by a generated id so a rename moves nothing else: tenants name the id.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    workspaces::DataTableCatalogResourceType,
    DB,
};

/// The cluster a role catalog belongs to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatatableRoleCluster {
    /// Windmill's own Postgres, behind `instance` data tables.
    #[default]
    Instance,
    /// The external instance cluster, behind `external_instance` data tables.
    ExternalInstance,
}

impl DatatableRoleCluster {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Instance => "instance",
            Self::ExternalInstance => "external_instance",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "instance" => Ok(Self::Instance),
            "external_instance" => Ok(Self::ExternalInstance),
            other => Err(Error::BadRequest(format!(
                "Unknown data table role cluster '{other}': expected instance or external_instance"
            ))),
        }
    }

    /// The cluster whose roles a data table on `kind` can use. `None` for a resource-backed one,
    /// which is never under roles.
    pub fn of(kind: DataTableCatalogResourceType) -> Option<Self> {
        match kind {
            DataTableCatalogResourceType::Instance => Some(Self::Instance),
            DataTableCatalogResourceType::ExternalInstance => Some(Self::ExternalInstance),
            DataTableCatalogResourceType::Postgresql => None,
        }
    }
}

/// The connection every data table resolved to before roles existed (`custom_instance_user`). It
/// owns every pre-existing object, so it is a reserved name rather than a catalog entry: never
/// created, renamed or dropped.
pub const ADMIN_DATATABLE_ROLE: &str = "admin";

/// The login the admin connection uses, and the role every created role is granted to — that
/// membership is what later lets it `ALTER ... OWNER TO` a role and drop it.
pub const CUSTOM_INSTANCE_USER: &str = "custom_instance_user";

/// One catalog entry, as stored in `datatable_role`. The password is per role and instance-wide;
/// it belongs to the instance, not to any workspace's settings.
/// No `Serialize`/`Deserialize`: the catalog is rows now, and a derived `Serialize` would emit
/// `pwd` — the same way out for a credential that the hand-written `Debug` below closes on the log
/// side.
#[derive(Clone)]
pub struct InstanceDatatableRole {
    /// The Postgres role name, verbatim.
    pub name: String,
    pub enabled: bool,
    /// Absent only for a role whose provisioning did not finish; resolving as it then errors
    /// rather than falling back to admin.
    ///
    /// A plain string rather than a `StringOrSecretRef` like the instance user's password: that
    /// one is a secret ref because an operator supplies it and may want it to come from their own
    /// backend, while this one is minted here and never entered by anyone, so there is nothing for
    /// a ref to point at. Encrypting generated secrets at rest is a separate change that would
    /// take the replication password with it.
    pub pwd: Option<String>,
}

/// Hand-written so `{:?}` on a catalog cannot put a live Postgres password in a log line or an
/// audit record. Everything else about the entry is safe to print.
impl std::fmt::Debug for InstanceDatatableRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanceDatatableRole")
            .field("name", &self.name)
            .field("enabled", &self.enabled)
            .field("pwd", &self.pwd.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

pub type DatatableRoleCatalog = BTreeMap<String, InstanceDatatableRole>;

/// Names Postgres or Windmill already owns. `admin` is excluded because it never reaches the
/// cluster as a role name at all — it resolves to `custom_instance_user`.
fn is_reserved_role_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == ADMIN_DATATABLE_ROLE
        || lower == "postgres"
        || lower == "public"
        || lower.starts_with("pg_")
        || lower.starts_with("windmill_")
        || lower.starts_with("custom_instance_")
}

/// The charset is what makes every downstream interpolation safe: the name reaches Postgres as a
/// quoted identifier, a `-- role <name>` annotation, and a `?role=` query parameter.
pub fn validate_role_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 63 {
        return Err(Error::BadRequest(format!(
            "Invalid data table role name '{name}': it must be between 1 and 63 characters"
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(Error::BadRequest(format!(
            "Invalid data table role name '{name}': only letters, digits, '_' and '-' are allowed"
        )));
    }
    if is_reserved_role_name(name) {
        return Err(Error::BadRequest(format!(
            "'{name}' is reserved and cannot be used as a data table role name"
        )));
    }
    Ok(())
}

/// A double-quoted Postgres identifier. Doubling `"` is Postgres's own escaping inside one, so this
/// quotes any name — schema, table or role. Role names are validated as well
/// ([`validate_role_name`]) because they also travel unquoted, in `-- role <name>` and `?role=`.
pub fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Serialize the mutations that are not already serialized by the row itself.
///
/// A create is an insert and a delete is a delete, which Postgres orders for us — the unique index
/// on `name` is what makes two concurrent creates of the same name one winner and one error. What
/// still needs it is the window between the cluster DDL and the row: `CREATE ROLE` is not visible
/// to another transaction's `pg_roles` check until commit, so without this two creates of the same
/// name both pass their existence check and one fails on the index having already made the login.
/// Held for the transaction, so the DDL has to run on that same transaction to be covered.
pub async fn lock_role_catalog(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<()> {
    sqlx::query!("SELECT pg_advisory_xact_lock(hashtext('datatable_role_catalog'))")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// A replication stream reads every row whatever a data table's roles grant. Turning roles on looks
/// for streams holding this exclusive; whatever can start a Postgres trigger or capture streaming
/// holds it shared on the transaction that commits it. So either the look sees the stream, or the
/// stream's listener connects after roles are committed and refuses. Held for the transaction.
pub async fn lock_datatable_streams(conn: &mut sqlx::PgConnection, exclusive: bool) -> Result<()> {
    let lock = if exclusive {
        "pg_advisory_xact_lock"
    } else {
        "pg_advisory_xact_lock_shared"
    };
    sqlx::query(&format!("SELECT {lock}(hashtext('datatable_streams'))"))
        .execute(conn)
        .await?;
    Ok(())
}

/// Whether an instance database is reached only through entries under roles is decided by two
/// writes that lock different workspaces' settings rows: turning roles on for one entry, and a
/// settings save pointing an entry without roles at the database. Each holds this for every
/// database it decides on, so neither reads past the other's uncommitted write. Held for the
/// transaction; the names are locked in sorted order so two holders cannot deadlock.
pub async fn lock_instance_databases_governance<'a>(
    conn: &mut sqlx::PgConnection,
    dbnames: impl IntoIterator<Item = &'a str>,
) -> Result<()> {
    let dbnames: std::collections::BTreeSet<&str> = dbnames.into_iter().collect();
    for dbname in dbnames {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext('datatable_instance_database:' || $1))")
            .bind(dbname)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

/// Disclosure: returns every role's stored Postgres password in plaintext. Any server path that
/// has to resolve or name a role may call it — including handlers open to a workspace member, who
/// need the names — but callers MUST NOT let `pwd` reach a response, a log line, an audit record
/// or an export. Nothing about who may call it: the credential is the whole risk, and `Debug` is
/// hand-written to redact it for the same reason.
pub async fn read_role_catalog(
    db: &DB,
    cluster: DatatableRoleCluster,
) -> Result<DatatableRoleCatalog> {
    crate::datatable_roles_oss::read_role_catalog(db, cluster).await
}

/// As [`read_role_catalog`], reading inside the caller's transaction so the value is the one
/// [`lock_role_catalog`] is protecting. Same disclosure contract.
pub async fn read_role_catalog_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cluster: DatatableRoleCluster,
) -> Result<DatatableRoleCatalog> {
    crate::datatable_roles_oss::read_role_catalog_tx(tx, cluster).await
}

/// The cluster a role belongs to, or `None` if no role has this id.
pub async fn role_cluster(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: &str,
) -> Result<Option<DatatableRoleCluster>> {
    crate::datatable_roles_oss::role_cluster(tx, id).await
}

/// Record a role, in the caller's transaction. On Windmill's own cluster that commits it with the
/// `CREATE ROLE` it describes; on the external cluster the role already exists by then.
///
/// Authorization: writes a generated Postgres credential. Callers MUST restrict this to superadmin
/// paths and MUST hold [`lock_role_catalog`] on `tx`.
pub async fn insert_role_catalog_entry(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: &str,
    cluster: DatatableRoleCluster,
    role: &InstanceDatatableRole,
) -> Result<()> {
    crate::datatable_roles_oss::insert_role_catalog_entry(tx, id, cluster, role).await
}

/// Update a role's recorded name, login flag and password. Same contract as
/// [`insert_role_catalog_entry`].
pub async fn update_role_catalog_entry(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: &str,
    role: &InstanceDatatableRole,
) -> Result<()> {
    crate::datatable_roles_oss::update_role_catalog_entry(tx, id, role).await
}

/// Forget a role. Same contract as [`insert_role_catalog_entry`]; run it in the transaction that
/// drops the cluster login, so the two cannot disagree.
pub async fn delete_role_catalog_entry(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: &str,
) -> Result<()> {
    crate::datatable_roles_oss::delete_role_catalog_entry(tx, id).await
}

/// Resolve the role a caller named to its catalog id. A disabled role is an error rather than a
/// silent fallback: the caller asked for something the instance deliberately turned off.
pub fn role_id_by_name<'a>(catalog: &'a DatatableRoleCatalog, name: &str) -> Result<&'a str> {
    let entry = catalog
        .iter()
        .find(|(_, role)| role.name == name)
        .ok_or_else(|| {
            Error::NotFound(format!(
                "'{name}' is not a data table role of this database's cluster. Defined roles: {}.",
                catalog
                    .values()
                    .map(|r| r.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;
    if !entry.1.enabled {
        return Err(Error::BadRequest(format!(
            "Data table role '{name}' is disabled on this instance"
        )));
    }
    Ok(entry.0.as_str())
}

/// Every database Windmill manages on `cluster`. Role provisioning has to reach all of them: a role
/// that cannot `CONNECT` to a database is refused by Postgres before any grant matters.
pub async fn registered_instance_databases(
    db: &DB,
    cluster: DatatableRoleCluster,
) -> Result<Vec<String>> {
    crate::datatable_roles_oss::registered_instance_databases(db, cluster).await
}

/// `CONNECT` on `dbname` for every enabled role of `cluster`, and none for `PUBLIC`. Run at role
/// creation, at database creation, and lazily whenever a managed data table is administered, so a
/// database provisioned before a role existed is repaired rather than left silently unreachable.
///
/// Authorization: rewrites a database's ACL with the server's own credentials and checks nothing.
/// Callers MUST have authorized administration of `dbname` — superadmin, or an admin of the
/// workspace governing a data table on it.
pub async fn converge_connect_grants(
    db: &DB,
    cluster: DatatableRoleCluster,
    dbname: &str,
) -> Result<()> {
    crate::datatable_roles_oss::converge_connect_grants(db, cluster, dbname).await
}

/// As [`converge_connect_grants`], with the catalog of `cluster` the caller already read. Same
/// contract.
pub async fn converge_connect_grants_with(
    db: &DB,
    cluster: DatatableRoleCluster,
    dbname: &str,
    catalog: &DatatableRoleCatalog,
) -> Result<()> {
    crate::datatable_roles_oss::converge_connect_grants_with(db, cluster, dbname, catalog).await
}

/// `CREATE ROLE <name> LOGIN PASSWORD ...; GRANT <name> TO custom_instance_user` on `cluster`. No
/// privileges beyond that — an admin grants them through SQL or the ACL editor.
///
/// On Windmill's own cluster the DDL runs on `tx`, so it commits with the catalog row. The external
/// cluster is another server: the role is created there before `tx` commits, and callers MUST drop
/// it again ([`drop_datatable_role`]) if `tx` then fails to commit.
///
/// Authorization: creates a cluster-wide Postgres login. Callers MUST restrict this to superadmin
/// paths, and MUST hold [`lock_role_catalog`] on `tx`.
pub async fn create_datatable_role(
    db: &DB,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cluster: DatatableRoleCluster,
    name: &str,
    password: &str,
) -> Result<()> {
    crate::datatable_roles_oss::create_datatable_role(db, tx, cluster, name, password).await
}

/// Authorization: alters a cluster-wide Postgres login. Callers MUST restrict this to superadmin
/// paths, and MUST hold [`lock_role_catalog`] on `tx`.
pub async fn set_datatable_role_login(
    db: &DB,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cluster: DatatableRoleCluster,
    name: &str,
    enabled: bool,
) -> Result<()> {
    crate::datatable_roles_oss::set_datatable_role_login(db, tx, cluster, name, enabled).await
}

/// A rename discards an md5-hashed password, so the caller has to hand over a fresh one. On the
/// external cluster the rename lands before `tx` commits, and callers MUST rename it back if `tx`
/// then fails to commit.
///
/// Authorization: renames a cluster-wide Postgres login. Callers MUST restrict this to superadmin
/// paths, and MUST hold [`lock_role_catalog`] on `tx`.
pub async fn rename_datatable_role(
    db: &DB,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cluster: DatatableRoleCluster,
    from: &str,
    to: &str,
    password: &str,
) -> Result<()> {
    crate::datatable_roles_oss::rename_datatable_role(db, tx, cluster, from, to, password).await
}

/// A role owning anything in any database blocks its own `DROP ROLE`, and both its objects and the
/// privileges granted to it are only visible from inside each database — hence the pass over the
/// registry. An unreachable database aborts the whole delete: dropping the role while one database
/// still holds objects owned by it leaves those objects owned by a numeric OID nobody can name.
///
/// Each pass runs as the cluster's administrator rather than `custom_instance_user`: on Windmill's
/// own cluster the instance's Postgres user, on the external one its configured admin login. Both
/// own the databases and can therefore revoke a grant whoever made it. `custom_instance_user`
/// could only undo what it granted itself, so a privilege planted by an operator in psql — the
/// ordinary way privileges reach a role — would survive and block the drop.
///
/// Authorization: drops a cluster-wide Postgres login and reassigns everything it owns. Callers
/// MUST restrict this to superadmin paths, and MUST hold [`lock_role_catalog`] on `tx`.
///
/// The per-database passes open their own connections and cannot join `tx`; the lock is what keeps
/// a concurrent mutation out while they run. On Windmill's own cluster only the final `DROP ROLE`
/// is on `tx`, so it commits or rolls back with the catalog write that forgets the role; on the
/// external cluster it runs there, and tolerates a role already gone so a retry after a failed
/// commit can finish. The passes commit as they go, so callers MUST have disabled the role in an
/// earlier committed transaction: a failure part-way then leaves a disabled role to retry, not an
/// enabled one already stripped in some databases.
pub async fn drop_datatable_role(
    db: &DB,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cluster: DatatableRoleCluster,
    name: &str,
) -> Result<()> {
    crate::datatable_roles_oss::drop_datatable_role(db, tx, cluster, name).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_names_are_validated() {
        assert!(validate_role_name("analytics").is_ok());
        assert!(validate_role_name("read-only_2").is_ok());
        assert!(validate_role_name("").is_err());
        assert!(validate_role_name(&"a".repeat(64)).is_err());
        assert!(validate_role_name("has space").is_err());
        assert!(validate_role_name("quote\"injection").is_err());
        // Reserved, case-insensitively.
        assert!(validate_role_name("admin").is_err());
        assert!(validate_role_name("Postgres").is_err());
        assert!(validate_role_name("pg_read_all_data").is_err());
        assert!(validate_role_name("windmill_user").is_err());
        assert!(validate_role_name("custom_instance_user").is_err());
    }

    #[test]
    fn a_disabled_role_is_an_error_not_a_fallback() {
        let mut catalog = DatatableRoleCatalog::new();
        catalog.insert(
            "id1".to_string(),
            InstanceDatatableRole {
                name: "analytics".to_string(),
                enabled: false,
                pwd: Some("x".to_string()),
            },
        );
        assert!(role_id_by_name(&catalog, "analytics").is_err());
        assert!(role_id_by_name(&catalog, "nope").is_err());
        catalog.get_mut("id1").unwrap().enabled = true;
        assert_eq!(role_id_by_name(&catalog, "analytics").unwrap(), "id1");
    }
}
