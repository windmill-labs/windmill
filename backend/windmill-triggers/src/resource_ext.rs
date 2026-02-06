/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

// This module provides try_get_resource_from_db_as via the JobOps trait.
// The actual implementation lives in windmill-api and is injected at runtime.

use windmill_api_auth::ApiAuthed;
use windmill_common::{db::UserDB, error::Result, DB};

use crate::jobs_ext::get_ops;

pub async fn try_get_resource_from_db_as<T>(
    authed: &ApiAuthed,
    user_db: Option<UserDB>,
    db: &DB,
    resource_path: &str,
    w_id: &str,
) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let ops = get_ops();
    let value = ops
        .try_get_resource_from_db(authed, user_db, db, resource_path, w_id)
        .await?;
    serde_json::from_value(value).map_err(|e| {
        windmill_common::error::Error::internal_err(format!(
            "Error deserializing resource {resource_path}: {e}"
        ))
    })
}
