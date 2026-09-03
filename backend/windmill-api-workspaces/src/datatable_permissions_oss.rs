/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Where the role planner comes from: the enterprise one, or a refusal.
//!
//! Every change to a data table's Postgres roles — creating, renaming, dropping
//! them — is the plan this returns, so an edition that is not enterprise cannot
//! make one. `private` alone is not that edition: community builds carry it, so
//! the planner is behind `enterprise` as well.

#[cfg(all(feature = "private", feature = "enterprise"))]
#[allow(unused)]
pub(crate) use crate::datatable_permissions_ee::*;

#[cfg(not(all(feature = "private", feature = "enterprise")))]
use {
    crate::datatable_permissions::{
        DefaultAclRule, PgRoleInventory, RolePlan, SetDatatablePermissions,
    },
    windmill_common::error::{Error, Result},
    windmill_common::workspaces::DataTablePermissions,
};

#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub(crate) fn plan_role_changes(
    _w_id: &str,
    _datatable: &str,
    _dbname: &str,
    _admin_pg_role: &str,
    _old: Option<&DataTablePermissions>,
    _req: &SetDatatablePermissions,
    _pg_roles: &PgRoleInventory,
    _public_schema_is_open: bool,
    _default_acl_rules: &[DefaultAclRule],
) -> Result<RolePlan> {
    Err(Error::BadRequest(
        "Data table permissions are a Windmill Enterprise Edition feature".to_string(),
    ))
}

#[cfg(all(test, not(all(feature = "private", feature = "enterprise"))))]
mod tests {
    /// Compiled in every edition that is not enterprise — community builds
    /// included, which carry the enterprise sources but must not plan with them.
    #[test]
    fn a_non_enterprise_build_plans_nothing() {
        let plan = super::plan_role_changes(
            "acme",
            "main",
            "db",
            "admin",
            None,
            &serde_json::from_value(serde_json::json!({ "enabled": true })).unwrap(),
            &Default::default(),
            false,
            &[],
        );
        assert!(plan
            .unwrap_err()
            .to_string()
            .contains("Windmill Enterprise Edition"));
    }
}
