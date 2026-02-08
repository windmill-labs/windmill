/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

// Re-export everything from windmill-api-scripts
pub use windmill_api_scripts::scripts::*;

use crate::triggers::{get_triggers_count_internal, TriggersCount};
use axum::{
    extract::{Extension, Path},
    routing::get,
    Router,
};
use windmill_common::{error::JsonResult, utils::StripPath, DB};

/// Wraps the subcrate's workspaced_service with the trigger count route
/// that depends on windmill-api internals.
pub fn workspaced_service() -> Router {
    windmill_api_scripts::scripts::workspaced_service()
        .route("/get_triggers_count/*path", get(get_triggers_count))
}

async fn get_triggers_count(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<TriggersCount> {
    let path = path.to_path();
    get_triggers_count_internal(&db, &w_id, &path, false).await
}
