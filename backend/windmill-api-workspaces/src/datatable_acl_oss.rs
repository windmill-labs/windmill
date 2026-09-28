/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Where the ACL planner comes from: the enterprise one, or a refusal.
//!
//! Data table roles are an Enterprise Edition feature, and so is everything here — reading who
//! owns what included. `private` alone is not that edition — community builds carry it — so the
//! planner is behind `enterprise` as well.

#[cfg(all(feature = "private", feature = "enterprise"))]
pub(crate) use crate::datatable_acl_ee::plan_statements;

#[cfg(all(feature = "private", feature = "enterprise"))]
pub(crate) fn ensure_datatable_acl_available() -> windmill_common::error::Result<()> {
    Ok(())
}

#[cfg(not(all(feature = "private", feature = "enterprise")))]
use {
    crate::datatable_acl::{AclChange, AclPlan, AclTarget, CatalogFacts},
    windmill_common::{datatable_roles_oss::datatable_roles_unavailable, error::Result},
};

/// Checked first by every ACL route, before anything is read or connected to.
#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub(crate) fn ensure_datatable_acl_available() -> Result<()> {
    Err(datatable_roles_unavailable())
}

#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub(crate) fn plan_statements(
    _target: &AclTarget,
    _change: &AclChange,
    _dbname: &str,
    _pg_role: &str,
    _facts: &CatalogFacts,
) -> Result<AclPlan> {
    Err(datatable_roles_unavailable())
}
