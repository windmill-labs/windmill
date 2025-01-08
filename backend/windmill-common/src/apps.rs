/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Id in the `app_script` table.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, Hash, Eq, PartialEq)]
#[serde(transparent)]
pub struct AppScriptId(pub i64);

#[derive(Deserialize)]
pub struct ListAppQuery {
    pub starred_only: Option<bool>,
    pub path_exact: Option<String>,
    pub path_start: Option<String>,
    pub include_draft_only: Option<bool>,
    pub with_deployment_msg: Option<bool>,
}

#[derive(Deserialize)]
pub struct RawAppValue {
    pub files: HashMap<String, String>,
}
