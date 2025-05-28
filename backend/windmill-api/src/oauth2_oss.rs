/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;
use axum::Router;
use sqlx::{Postgres, Transaction};
use crate::db::DB;
use windmill_common::error;

pub fn global_service() -> Router {
    crate::oauth2_ee::global_service()
}

pub fn workspaced_service() -> Router {
    crate::oauth2_ee::workspaced_service()
}

pub async fn build_oauth_clients(
    base_url: &str,
    oauths_from_config: Option<HashMap<String, OAuthClient>>,
    db: &DB,
) -> anyhow::Result<AllClients> {
    crate::oauth2_ee::build_oauth_clients(base_url, oauths_from_config, db).await
}

pub async fn _refresh_token<'c>(
    tx: Transaction<'c, Postgres>,
    path: &str,
    w_id: &str,
    id: i32,
    db: &DB,
) -> error::Result<String> {
    crate::oauth2_ee::_refresh_token(tx, path, w_id, id, db).await
}

pub async fn check_nb_of_user(db: &DB) -> error::Result<()> {
    crate::oauth2_ee::check_nb_of_user(db).await
}

// Re-export all public types
pub use crate::oauth2_ee::AllClients;
pub use crate::oauth2_ee::BasicClientsMap;
pub use crate::oauth2_ee::ClientWithScopes;
pub use crate::oauth2_ee::OAuthClient;
pub use crate::oauth2_ee::OAuthConfig;
pub use crate::oauth2_ee::SlackVerifier;
pub use crate::oauth2_ee::TokenResponse;