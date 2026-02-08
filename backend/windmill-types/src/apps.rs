use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::scripts::ScriptLang;

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

pub struct AppInlineScript {
    pub language: Option<ScriptLang>,
    pub content: String,
    pub lock: Option<String>,
}
