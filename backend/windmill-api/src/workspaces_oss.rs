/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::{ApiAuthed, DB};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct EditAutoInvite {
    pub auto_invite_domain: Option<String>,
    pub auto_invite_operator: Option<bool>,
}

pub async fn edit_auto_invite(
    authed: ApiAuthed,
    db: DB,
    w_id: String,
    ea: EditAutoInvite,
) -> windmill_common::error::Result<String> {
    crate::workspaces_ee::edit_auto_invite(authed, db, w_id, ea).await
}