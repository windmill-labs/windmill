/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(feature = "private")]
pub use crate::oauth_refresh_ee::_refresh_token;

#[cfg(feature = "private")]
pub use crate::oauth_refresh_ee::_refresh_workspace_integration_token;

#[cfg(not(feature = "private"))]
use sqlx::{Postgres, Transaction};
#[cfg(not(feature = "private"))]
use windmill_common::db::DB;
#[cfg(not(feature = "private"))]
use windmill_common::error;

#[cfg(not(feature = "private"))]
pub async fn _refresh_token<'c>(
    tx: Transaction<'c, Postgres>,
    path: &str,
    w_id: &str,
    id: i32,
    db: &DB,
) -> error::Result<String> {
    windmill_oauth::refresh_token(
        tx,
        path,
        w_id,
        id,
        db,
        &*windmill_oauth::OAUTH_CLIENTS.read().await,
        &windmill_oauth::OAUTH_HTTP_CLIENT,
        include_str!("../../oauth_connect.json"),
    )
    .await
}
