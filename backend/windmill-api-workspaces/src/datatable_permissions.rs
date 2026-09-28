/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Who may connect to a data table as which role.
//!
//! The decision lives on the data table entry of the workspace that governs it, which is not
//! necessarily the workspace asking: a fork's entry points at its parent's, and everything here
//! resolves through that pointer first. Nothing in this module runs SQL against the data table —
//! a save is tenant lists and a default, and the Postgres roles themselves are the instance
//! catalog's business.

use axum::{routing::get, Router};

use windmill_api_auth::ApiAuthed;
use windmill_common::error::Result;
use windmill_common::workspaces::GoverningDatatable;
use windmill_common::DB;

use crate::datatable_permissions_oss as roles;

pub(crate) fn routes() -> Router {
    Router::new()
        .route(
            "/datatable_permissions/{datatable_name}",
            get(roles::get_datatable_permissions).post(roles::set_datatable_permissions),
        )
        .route(
            "/datatable_usable_roles/{datatable_name}",
            get(roles::list_usable_datatable_roles),
        )
}

/// Administering a data table — its permissions, its migrations that declare no role, its exports
/// — is for the admins of the workspace that governs it. A fork can use the data table; it never
/// administers it.
// The gate for whatever administers a data table under roles, which the routes of this module alone
// do not always reach.
#[allow(dead_code)]
pub(crate) async fn ensure_governs_datatable(
    db: &DB,
    authed: &ApiAuthed,
    w_id: &str,
    governing: &GoverningDatatable,
) -> Result<()> {
    roles::ensure_governs_datatable(db, authed, w_id, governing).await
}

/// Refuse a caller that no tenant of this data table covers.
///
/// The bookkeeping endpoints below open the data table's `admin` connection to read or create
/// `_wm_migrations` before they know which migration will run — so without this, someone covered
/// by no role at all can still force admin-backed reads and writes on a database they may not
/// touch. It asks only "may you reach this data table as anything"; which role a given migration
/// runs as is still decided per migration, and by the executor after that.
pub(crate) async fn ensure_reaches_datatable(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
    authed: &ApiAuthed,
) -> Result<()> {
    roles::ensure_reaches_datatable(db, w_id, datatable_name, authed).await
}

/// [`ensure_reaches_datatable`] against an entry already resolved, for a caller that goes on to
/// connect from that same entry.
pub(crate) async fn ensure_reaches_governing_datatable(
    db: &DB,
    w_id: &str,
    datatable_name: &str,
    governing: &GoverningDatatable,
    authed: &ApiAuthed,
) -> Result<()> {
    roles::ensure_reaches_governing_datatable(db, w_id, datatable_name, governing, authed).await
}
