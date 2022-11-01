/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::{Path, Query},
    routing::get,
    Extension, Json, Router,
};
use windmill_audit::{AuditLog, ListAuditLogQuery};
use windmill_common::{error::JsonResult, utils::Pagination};

use crate::{db::UserDB, users::Authed};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_audit))
        .route("/get/:id", get(get_audit))
}

async fn get_audit(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(id): Path<i32>,
) -> JsonResult<AuditLog> {
    let tx = user_db.begin(&authed).await?;
    let audit = windmill_audit::get_audit(tx, id).await?;
    Ok(Json(audit))
}
async fn list_audit(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(lq): Query<ListAuditLogQuery>,
) -> JsonResult<Vec<AuditLog>> {
    let tx = user_db.begin(&authed).await?;
    let rows = windmill_audit::list_audit(tx, w_id, pagination, lq).await?;
    Ok(Json(rows))
}
