/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::http::StatusCode;
use windmill_common::error::Error;

pub async fn request_teams_approval() -> Result<StatusCode, Error> {
    crate::teams_approvals_ee::request_teams_approval().await
}