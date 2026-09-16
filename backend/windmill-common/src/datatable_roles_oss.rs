/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Where data table roles come from: the enterprise implementation, or a refusal.
//!
//! Roles are an Enterprise Edition feature. An edition without them creates, grants and connects
//! as none, and a data table saved under roles — by an enterprise build, before a downgrade — is
//! refused rather than resolved as `admin`. A data table not under roles, asked for no role,
//! resolves as it always has. `private` alone is not that edition: community builds carry it.

use crate::error::Error;

/// What every roles path answers without the Enterprise Edition.
pub fn datatable_roles_unavailable() -> Error {
    Error::BadRequest("Data table roles are a Windmill Enterprise Edition feature".to_string())
}

#[cfg(all(feature = "private", feature = "enterprise"))]
pub(crate) use crate::datatable_roles_ee::{
    can_use_datatable_role, can_use_datatable_role_in_governing_workspace, converge_connect_grants,
    converge_connect_grants_with, create_instance_role, delete_role_catalog_entry,
    drop_instance_role, ensure_can_use_datatable_role, ensure_datatable_admin_access,
    ensure_instance_db_grant_options_unchecked, forget_datatable_role_everywhere,
    insert_role_catalog_entry, read_role_catalog, read_role_catalog_tx,
    registered_instance_databases, rename_instance_role, resolve_datatable_role_connection,
    set_instance_role_login, update_role_catalog_entry,
};

#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub(crate) use ce::*;

#[cfg(not(all(feature = "private", feature = "enterprise")))]
mod ce {
    use super::datatable_roles_unavailable as unavailable;
    use crate::{
        datatable_roles::{DatatableRoleCatalog, InstanceDatatableRole},
        db::AuthedRef,
        error::Result,
        workspaces::{
            resolve_governing_datatable, DataTableRoleTenants, DatatableAccess, GoverningDatatable,
        },
        DB,
    };

    type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

    pub(crate) async fn read_role_catalog(_db: &DB) -> Result<DatatableRoleCatalog> {
        Err(unavailable())
    }

    pub(crate) async fn read_role_catalog_tx(_tx: &mut Tx<'_>) -> Result<DatatableRoleCatalog> {
        Err(unavailable())
    }

    pub(crate) async fn insert_role_catalog_entry(
        _tx: &mut Tx<'_>,
        _id: &str,
        _role: &InstanceDatatableRole,
    ) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn update_role_catalog_entry(
        _tx: &mut Tx<'_>,
        _id: &str,
        _role: &InstanceDatatableRole,
    ) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn delete_role_catalog_entry(_tx: &mut Tx<'_>, _id: &str) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn registered_instance_databases(_db: &DB) -> Result<Vec<String>> {
        Err(unavailable())
    }

    /// Nothing to converge: with no roles to admit, an instance database keeps the `CONNECT`
    /// grants it was created with, `PUBLIC`'s included, as it did before roles existed.
    pub(crate) async fn converge_connect_grants(_db: &DB, _dbname: &str) -> Result<()> {
        Ok(())
    }

    /// As [`converge_connect_grants`].
    pub(crate) async fn converge_connect_grants_with(
        _db: &DB,
        _dbname: &str,
        _catalog: &DatatableRoleCatalog,
    ) -> Result<()> {
        Ok(())
    }

    pub(crate) async fn create_instance_role(
        _tx: &mut Tx<'_>,
        _name: &str,
        _password: &str,
    ) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn set_instance_role_login(
        _tx: &mut Tx<'_>,
        _name: &str,
        _enabled: bool,
    ) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn rename_instance_role(
        _tx: &mut Tx<'_>,
        _from: &str,
        _to: &str,
        _password: &str,
    ) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn drop_instance_role(_db: &DB, _tx: &mut Tx<'_>, _name: &str) -> Result<()> {
        Err(unavailable())
    }

    pub(crate) async fn ensure_instance_db_grant_options_unchecked(
        _db: &DB,
        _dbname: &str,
    ) -> Result<()> {
        Err(unavailable())
    }

    /// No tenant list covers anyone: there is no role to connect as.
    pub(crate) fn can_use_datatable_role(
        _tenants: &DataTableRoleTenants,
        _authed: &AuthedRef<'_>,
    ) -> bool {
        false
    }

    pub(crate) async fn can_use_datatable_role_in_governing_workspace(
        _db: &DB,
        _governing_w_id: &str,
        _w_id: &str,
        _tenants: &DataTableRoleTenants,
        _access: &DatatableAccess<'_>,
    ) -> Result<bool> {
        Err(unavailable())
    }

    /// Reached only for a data table under roles or a caller naming a role: both are refused.
    pub(crate) async fn resolve_datatable_role_connection(
        _db: &DB,
        _w_id: &str,
        _name: &str,
        _governing: &GoverningDatatable,
        _db_resource: serde_json::Value,
        _role: Option<&str>,
        _access: DatatableAccess<'_>,
    ) -> Result<serde_json::Value> {
        Err(unavailable())
    }

    /// A data table not under roles, asked for no role or for `admin`, is not a role decision and
    /// passes, as it did before roles existed. Anything else is refused.
    pub(crate) async fn ensure_can_use_datatable_role(
        db: &DB,
        w_id: &str,
        name: &str,
        role: Option<&str>,
        _access: &DatatableAccess<'_>,
        _context: &str,
    ) -> Result<()> {
        let governing = resolve_governing_datatable(db, w_id, name).await?;
        if governing.datatable.permissions.is_none()
            && role.is_none_or(|r| r == crate::datatable_roles::ADMIN_DATATABLE_ROLE)
        {
            Ok(())
        } else {
            Err(unavailable())
        }
    }

    /// A data table not under roles is the `admin` connection for anyone who reaches it, as before
    /// roles existed. One under roles is refused.
    pub(crate) async fn ensure_datatable_admin_access(
        db: &DB,
        w_id: &str,
        name: &str,
        _access: &DatatableAccess<'_>,
    ) -> Result<()> {
        let governing = resolve_governing_datatable(db, w_id, name).await?;
        if governing.datatable.permissions.is_none() {
            Ok(())
        } else {
            Err(unavailable())
        }
    }

    pub(crate) async fn forget_datatable_role_everywhere(
        _tx: &mut Tx<'_>,
        _role_id: &str,
    ) -> Result<()> {
        Err(unavailable())
    }
}
