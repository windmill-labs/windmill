/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use reqwest::Client;
use serde::Deserializer;
use sql_builder::prelude::*;

use crate::{
    audit::{audit_log, ActionKind},
    db::{UserDB, DB},
    error::{to_anyhow, Error, JsonResult, Result},
    jobs, parser, parser_go, parser_py, parser_ts,
    users::{owner_to_token_owner, truncate_token, Authed, Tokened},
    utils::{http_get_from_hub, list_elems_from_hub, require_admin, Pagination, StripPath},
};
use axum::{
    extract::{Extension, Host, Path, Query},
    routing::{get, post},
    Json, Router,
};
use hyper::StatusCode;
use serde::{de::Error as _, ser::SerializeSeq, Deserialize, Serialize};
use serde_json::{json, to_string_pretty};
use sql_builder::SqlBuilder;
use sqlx::{FromRow, Postgres, Transaction};
use std::{
    collections::hash_map::DefaultHasher,
    fmt::Display,
    hash::{Hash, Hasher},
};

const MAX_HASH_HISTORY_LENGTH_STORED: usize = 20;

pub fn global_service() -> Router {
    Router::new()
        .route(
            "/python/tojsonschema",
            post(parse_python_code_to_jsonschema),
        )
        .route("/deno/tojsonschema", post(parse_deno_code_to_jsonschema))
        .route("/go/tojsonschema", post(parse_go_code_to_jsonschema))
        .route("/hub/list", get(list_hub_scripts))
        .route("/hub/get/*path", get(get_hub_script_by_path))
        .route("/hub/get_full/*path", get(get_full_hub_script_by_path))
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_scripts))
        .route("/create", post(create_script))
        .route("/archive/p/*path", post(archive_script_by_path))
        .route("/get/p/*path", get(get_script_by_path))
        .route("/raw/p/*path", get(raw_script_by_path))
        .route("/exists/p/*path", get(exists_script_by_path))
        .route("/archive/h/:hash", post(archive_script_by_hash))
        .route("/delete/h/:hash", post(delete_script_by_hash))
        .route("/get/h/:hash", get(get_script_by_hash))
        .route("/raw/h/:hash", get(raw_script_by_hash))
        .route("/deployment_status/h/:hash", get(get_deployment_status))
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, PartialEq, Clone, Hash)]
#[sqlx(type_name = "SCRIPT_LANG", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ScriptLang {
    Deno,
    Python3,
    Go,
}

impl ScriptLang {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScriptLang::Deno => "deno",
            ScriptLang::Python3 => "python3",
            ScriptLang::Go => "go",
        }
    }
}

#[derive(sqlx::Type, PartialEq, Debug, Hash, Clone, Copy)]
#[sqlx(transparent)]
pub struct ScriptHash(pub i64);

#[derive(sqlx::Type, PartialEq)]
#[sqlx(transparent)]
pub struct ScriptHashes(Vec<i64>);

impl Display for ScriptHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", to_hex_string(&self.0))
    }
}
impl Serialize for ScriptHash {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(to_hex_string(&self.0).as_str())
    }
}
impl<'de> Deserialize<'de> for ScriptHash {
    fn deserialize<D>(deserializer: D) -> std::result::Result<ScriptHash, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let i = to_i64(&s).map_err(|e| D::Error::custom(format!("{}", e)))?;
        Ok(ScriptHash(i))
    }
}

impl Serialize for ScriptHashes {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for element in &self.0 {
            seq.serialize_element(&ScriptHash(*element))?;
        }
        seq.end()
    }
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Hash)]
#[sqlx(type_name = "SCRIPT_KIND", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ScriptKind {
    Trigger,
    Failure,
    Script,
    Approval,
}

#[derive(FromRow, Serialize)]
pub struct Script {
    pub workspace_id: String,
    pub hash: ScriptHash,
    pub path: String,
    pub parent_hashes: Option<ScriptHashes>,
    pub summary: String,
    pub description: String,
    pub content: String,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub archived: bool,
    pub schema: Option<Schema>,
    pub deleted: bool,
    pub is_template: bool,
    pub extra_perms: serde_json::Value,
    pub lock: Option<String>,
    pub lock_error_logs: Option<String>,
    pub language: ScriptLang,
    pub kind: ScriptKind,
}

#[derive(Serialize, Deserialize, sqlx::Type, Debug)]
#[sqlx(transparent)]
#[serde(transparent)]
pub struct Schema(pub serde_json::Value);

impl Hash for Schema {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Ok(s) = to_string_pretty(&self.0) {
            s.hash(state);
        }
    }
}

#[derive(Serialize, Deserialize, Hash)]
pub struct NewScript {
    pub path: String,
    pub parent_hash: Option<ScriptHash>,
    pub summary: String,
    pub description: String,
    pub content: String,
    pub schema: Option<Schema>,
    pub is_template: Option<bool>,
    pub lock: Option<Vec<String>>,
    pub language: ScriptLang,
    pub kind: Option<ScriptKind>,
}
