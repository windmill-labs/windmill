/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Where the data table permissions endpoints and their gates come from: the enterprise
//! implementation, or a refusal. Roles are an Enterprise Edition feature; see
//! `windmill_common::datatable_roles_oss`.

#[cfg(all(feature = "private", feature = "enterprise"))]
pub(crate) use crate::datatable_permissions_ee::{
    ensure_governs_datatable, ensure_reaches_datatable, get_datatable_permissions,
    list_usable_datatable_roles, set_datatable_permissions,
};

#[cfg(not(all(feature = "private", feature = "enterprise")))]
pub(crate) use ce::*;

#[cfg(not(all(feature = "private", feature = "enterprise")))]
mod ce {
    use windmill_api_auth::ApiAuthed;
    use windmill_common::{
        datatable_roles_oss::datatable_roles_unavailable as unavailable,
        error::Result,
        workspaces::{resolve_governing_datatable, GoverningDatatable},
        DB,
    };

    /// Nobody administers a data table's roles without them.
    #[allow(dead_code)]
    pub(crate) async fn ensure_governs_datatable(
        _db: &DB,
        _authed: &ApiAuthed,
        _w_id: &str,
        _governing: &GoverningDatatable,
    ) -> Result<()> {
        Err(unavailable())
    }

    /// A data table not under roles is reached as it was before roles existed. One under roles is
    /// refused: no role of it can be connected as.
    pub(crate) async fn ensure_reaches_datatable(
        db: &DB,
        w_id: &str,
        datatable_name: &str,
        _authed: &ApiAuthed,
    ) -> Result<()> {
        let governing = resolve_governing_datatable(db, w_id, datatable_name).await?;
        if governing.datatable.permissions.is_none() {
            Ok(())
        } else {
            Err(unavailable())
        }
    }

    // The routes stay registered so the API has one shape; each answers after authentication,
    // before anything is read.

    pub(crate) async fn get_datatable_permissions(_authed: ApiAuthed) -> Result<String> {
        Err(unavailable())
    }

    pub(crate) async fn set_datatable_permissions(_authed: ApiAuthed) -> Result<String> {
        Err(unavailable())
    }

    pub(crate) async fn list_usable_datatable_roles(_authed: ApiAuthed) -> Result<String> {
        Err(unavailable())
    }
}
