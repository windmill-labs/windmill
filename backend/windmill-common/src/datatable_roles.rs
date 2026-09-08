/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! The instance's data table role catalog.
//!
//! A data table role is a real Postgres login role on the Windmill cluster, named exactly as the
//! user named it, shared by every instance database. Windmill decides who may ask for a role (the
//! per-data-table tenant lists in [`crate::workspaces`]); Postgres decides what the role may then
//! touch. The catalog here is only the first half's vocabulary plus the cluster provisioning.
//!
//! Entries are keyed by a generated id so a rename moves nothing else: tenants name the id.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    DB,
};

/// The connection every data table resolved to before roles existed (`custom_instance_user`). It
/// owns every pre-existing object, so it is a reserved name rather than a catalog entry: never
/// created, renamed or dropped.
pub const ADMIN_DATATABLE_ROLE: &str = "admin";

/// The login the admin connection uses, and the role every created role is granted to — that
/// membership is what later lets it `ALTER ... OWNER TO` a role and drop it.
pub const CUSTOM_INSTANCE_USER: &str = "custom_instance_user";

/// One catalog entry. The password is per role and instance-wide; it lives here rather than in any
/// workspace's settings, next to the `custom_instance_user` password in the same
/// `custom_instance_pg_databases` row.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[cfg_attr(feature = "instance_config_schema", derive(schemars::JsonSchema))]
pub struct InstanceDatatableRole {
    /// The Postgres role name, verbatim.
    pub name: String,
    #[serde(default = "crate::more_serde::default_true")]
    pub enabled: bool,
    /// Absent only for a role whose provisioning did not finish; resolving as it then errors
    /// rather than falling back to admin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pwd: Option<String>,
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

/// SAFETY: every caller must have run [`validate_role_name`] first — the charset it enforces is
/// what makes this quoting sufficient.
fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn quote_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub async fn read_role_catalog(db: &DB) -> Result<DatatableRoleCatalog> {
    let value = sqlx::query_scalar!(
        "SELECT value->'roles' FROM global_settings WHERE name = 'custom_instance_pg_databases'"
    )
    .fetch_optional(db)
    .await?
    .flatten();
    match value {
        Some(v) => Ok(serde_json::from_value(v).unwrap_or_default()),
        None => Ok(Default::default()),
    }
}

/// Resolve the role a caller named to its catalog id. A disabled role is an error rather than a
/// silent fallback: the caller asked for something the instance deliberately turned off.
pub fn role_id_by_name<'a>(catalog: &'a DatatableRoleCatalog, name: &str) -> Result<&'a str> {
    let entry = catalog
        .iter()
        .find(|(_, role)| role.name == name)
        .ok_or_else(|| {
            Error::NotFound(format!(
                "'{name}' is not a data table role of this instance. Defined roles: {}.",
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

/// Every instance database the registry knows about. Role provisioning has to reach all of them:
/// a role that cannot `CONNECT` to a database is refused by Postgres before any grant matters.
pub async fn registered_instance_databases(db: &DB) -> Result<Vec<String>> {
    let names = sqlx::query_scalar!(
        "SELECT jsonb_object_keys(value->'databases') FROM global_settings
         WHERE name = 'custom_instance_pg_databases'"
    )
    .fetch_all(db)
    .await?;
    Ok(names.into_iter().flatten().collect())
}

/// `CONNECT` on `dbname` for every enabled role, and none for `PUBLIC`. Run at role creation, at
/// database creation, and lazily whenever an instance data table is administered, so a database
/// provisioned before a role existed is repaired rather than left silently unreachable.
pub async fn converge_connect_grants(db: &DB, dbname: &str) -> Result<()> {
    let catalog = read_role_catalog(db).await?;
    converge_connect_grants_with(db, dbname, &catalog).await
}

pub async fn converge_connect_grants_with(
    db: &DB,
    dbname: &str,
    catalog: &DatatableRoleCatalog,
) -> Result<()> {
    crate::validate_dbname(dbname)?;
    let quoted_db = quote_ident(dbname);
    let mut sql = format!("REVOKE CONNECT ON DATABASE {quoted_db} FROM PUBLIC;\n");
    for role in catalog.values().filter(|r| r.enabled) {
        validate_role_name(&role.name)?;
        sql.push_str(&format!(
            "GRANT CONNECT ON DATABASE {quoted_db} TO {};\n",
            quote_ident(&role.name)
        ));
    }
    sqlx::raw_sql(&sql).execute(db).await?;
    Ok(())
}

/// `CREATE ROLE <name> LOGIN PASSWORD ...; GRANT <name> TO custom_instance_user`, and `CONNECT` on
/// every registered database. No privileges beyond that — an admin grants them through SQL or the
/// ACL editor.
pub async fn create_instance_role(db: &DB, name: &str, password: &str) -> Result<()> {
    validate_role_name(name)?;
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = $1)",
        name
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);
    if exists {
        return Err(Error::BadRequest(format!(
            "A Postgres role named '{name}' already exists on this cluster"
        )));
    }
    let quoted = quote_ident(name);
    sqlx::raw_sql(&format!(
        "CREATE ROLE {quoted} LOGIN PASSWORD {};\nGRANT {quoted} TO {};",
        quote_literal(password),
        quote_ident(CUSTOM_INSTANCE_USER),
    ))
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_instance_role_login(db: &DB, name: &str, enabled: bool) -> Result<()> {
    validate_role_name(name)?;
    sqlx::query(&format!(
        "ALTER ROLE {} {}",
        quote_ident(name),
        if enabled { "LOGIN" } else { "NOLOGIN" }
    ))
    .execute(db)
    .await?;
    Ok(())
}

/// A rename discards an md5-hashed password, so the caller has to hand over a fresh one.
pub async fn rename_instance_role(db: &DB, from: &str, to: &str, password: &str) -> Result<()> {
    validate_role_name(from)?;
    validate_role_name(to)?;
    let taken = sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = $1)",
        to
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);
    if taken {
        return Err(Error::BadRequest(format!(
            "A Postgres role named '{to}' already exists on this cluster"
        )));
    }
    sqlx::raw_sql(&format!(
        "ALTER ROLE {} RENAME TO {};\nALTER ROLE {} PASSWORD {};",
        quote_ident(from),
        quote_ident(to),
        quote_ident(to),
        quote_literal(password),
    ))
    .execute(db)
    .await?;
    Ok(())
}

/// A role owning anything in any database blocks its own `DROP ROLE`, and both its objects and the
/// privileges granted to it are only visible from inside each database — hence the pass over the
/// registry. An unreachable database aborts the whole delete: dropping the role while one database
/// still holds objects owned by it leaves those objects owned by a numeric OID nobody can name.
///
/// Each pass runs as the instance's own Postgres user rather than `custom_instance_user`, which
/// owns the databases and can therefore revoke a grant whoever made it. `custom_instance_user`
/// could only undo what it granted itself, so a privilege planted by an operator in psql — the
/// ordinary way privileges reach a role — would survive and block the drop.
pub async fn drop_instance_role(db: &DB, name: &str) -> Result<()> {
    validate_role_name(name)?;
    let quoted = quote_ident(name);
    let reassign = format!(
        "REASSIGN OWNED BY {quoted} TO {};\nDROP OWNED BY {quoted};",
        quote_ident(CUSTOM_INSTANCE_USER)
    );

    let base = crate::PgDatabase::parse_uri(&crate::get_database_url().await?.as_str().await)?;
    for dbname in registered_instance_databases(db).await? {
        let creds = crate::PgDatabase { dbname: dbname.clone(), ..base.clone() };
        let (client, connection) = creds.connect(Some(db)).await.map_err(|e| {
            Error::BadRequest(format!(
                "Cannot delete role '{name}': instance database '{dbname}' is unreachable ({e}). \
                 Objects it owns there would be orphaned."
            ))
        })?;
        let join_handle = tokio::spawn(async move { connection.await });
        let result = client.batch_execute(&reassign).await;
        drop(client);
        crate::shutdown_pg_connection(join_handle).await?;
        result.map_err(|e| {
            Error::internal_err(format!(
                "Reassigning what role '{name}' owns in '{dbname}': {}",
                crate::error::pg_error_message(&e)
            ))
        })?;
    }

    sqlx::raw_sql(&format!("{reassign}\nDROP ROLE {quoted};"))
        .execute(db)
        .await?;
    Ok(())
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
