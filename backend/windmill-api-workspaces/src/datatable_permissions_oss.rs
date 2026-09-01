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
//! them — is the plan this returns, so an edition without the enterprise module
//! cannot make one.

#[cfg(feature = "private")]
#[allow(unused)]
pub(crate) use crate::datatable_permissions_ee::*;

#[cfg(not(feature = "private"))]
use {
    crate::datatable_permissions::{DefaultAclRule, RolePlan, SetDatatablePermissions},
    std::collections::HashSet,
    windmill_common::error::{Error, Result},
    windmill_common::workspaces::DataTablePermissions,
};

#[cfg(not(feature = "private"))]
pub(crate) fn plan_role_changes(
    _w_id: &str,
    _datatable: &str,
    _dbname: &str,
    _admin_pg_role: &str,
    _old: Option<&DataTablePermissions>,
    _req: &SetDatatablePermissions,
    _existing_pg_roles: &HashSet<String>,
    _public_schema_is_open: bool,
    _default_acl_rules: &[DefaultAclRule],
) -> Result<RolePlan> {
    Err(Error::BadRequest(
        "Data table permissions are a Windmill Enterprise Edition feature".to_string(),
    ))
}
