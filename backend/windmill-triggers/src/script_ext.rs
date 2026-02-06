/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use windmill_api_auth::ApiAuthed;
use windmill_common::{error::Result, DB};

use crate::require_is_writer_internal;

pub async fn require_is_writer(authed: &ApiAuthed, path: &str, w_id: &str, db: DB) -> Result<()> {
    require_is_writer_internal(
        authed,
        path,
        w_id,
        db,
        "SELECT extra_perms FROM script WHERE path = $1 AND workspace_id = $2 ORDER BY created_at DESC LIMIT 1",
        "script",
    )
    .await
}
