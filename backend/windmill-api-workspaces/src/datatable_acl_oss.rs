/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Where the ACL planner comes from: the enterprise one, or a refusal.
//!
//! Reading who owns what stays open; every owner change and every grant is the
//! plan this returns, so an edition without the enterprise module cannot make
//! one.

#[cfg(feature = "private")]
#[allow(unused)]
pub(crate) use crate::datatable_acl_ee::*;

#[cfg(not(feature = "private"))]
use {
    crate::datatable_acl::{AclChange, AclPlan, AclTarget, OwnedObject},
    windmill_common::error::{Error, Result},
};

#[cfg(not(feature = "private"))]
pub(crate) fn plan_statements(
    _target: &AclTarget,
    _change: &AclChange,
    _dbname: &str,
    _pg_role: &str,
    _other_pg_roles: &[String],
    _existing_objects: &[OwnedObject],
) -> Result<AclPlan> {
    Err(Error::BadRequest(
        "Data table permissions are a Windmill Enterprise Edition feature".to_string(),
    ))
}
