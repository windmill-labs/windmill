/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Where the ACL planner comes from: the enterprise one, or a refusal.
//!
//! Reading who owns what stays open in every edition; every change is a plan, so an edition
//! without the planner cannot make one. `private` alone is not that edition — community builds
//! carry it — so the planner is behind `enterprise` as well.

#[cfg(all(feature = "private", feature = "enterprise"))]
pub(crate) use crate::datatable_acl_ee::plan_statements;

#[cfg(all(feature = "private", feature = "enterprise"))]
pub(crate) fn ensure_acl_planner() -> windmill_common::error::Result<()> {
    Ok(())
}

#[cfg(not(all(feature = "private", feature = "enterprise")))]
use {
    crate::datatable_acl::{AclChange, AclPlan, AclTarget, OwnedObject},
    windmill_common::error::{Error, Result},
};

/// Checked right after authorization, before anything connects with the instance's credentials.
#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub(crate) fn ensure_acl_planner() -> Result<()> {
    Err(Error::BadRequest(
        "Data table permissions are a Windmill Enterprise Edition feature".to_string(),
    ))
}

#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub(crate) fn plan_statements(
    _target: &AclTarget,
    _change: &AclChange,
    _dbname: &str,
    _pg_role: &str,
    _other_pg_roles: &[String],
    _existing_objects: &[OwnedObject],
) -> Result<AclPlan> {
    ensure_acl_planner()?;
    Err(Error::internal_err(
        "No ACL planner in this edition".to_string(),
    ))
}
