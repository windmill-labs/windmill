/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{
    fmt::{self, Display},
    hash::{Hash, Hasher},
};

use crate::{
    error::{to_anyhow, Error},
    utils::http_get_from_hub,
    DB,
};
use serde::de::Error as _;
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize};
use serde_json::to_string_pretty;

use crate::utils::StripPath;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Hash, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "SCRIPT_LANG", rename_all = "lowercase")
)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ScriptLang {
    Nativets,
    Deno,
    Python3,
    Go,
    Bash,
    Powershell,
    Postgresql,
    Bun,
    Mysql,
    Bigquery,
    Snowflake,
    Graphql,
    Mssql,
}

impl ScriptLang {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScriptLang::Bun => "bun",
            ScriptLang::Nativets => "nativets",
            ScriptLang::Deno => "deno",
            ScriptLang::Python3 => "python3",
            ScriptLang::Go => "go",
            ScriptLang::Bash => "bash",
            ScriptLang::Powershell => "powershell",
            ScriptLang::Postgresql => "postgresql",
            ScriptLang::Mysql => "mysql",
            ScriptLang::Bigquery => "bigquery",
            ScriptLang::Snowflake => "snowflake",
            ScriptLang::Mssql => "mssql",
            ScriptLang::Graphql => "graphql",
        }
    }
}

#[derive(PartialEq, Debug, Hash, Clone, Copy)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(transparent))]
pub struct ScriptHash(pub i64);

#[derive(PartialEq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(transparent, no_pg_array))]
pub struct ScriptHashes(pub Vec<i64>);

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

#[derive(Serialize, Deserialize, Debug, Hash)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "SCRIPT_KIND", rename_all = "lowercase")
)]
#[serde(rename_all = "lowercase")]
pub enum ScriptKind {
    Trigger,
    Failure,
    Script,
    Approval,
}

impl Display for ScriptKind {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(match self {
            ScriptKind::Trigger => "trigger",
            ScriptKind::Failure => "failure",
            ScriptKind::Script => "script",
            ScriptKind::Approval => "approval",
        })?;
        Ok(())
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
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
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrent_limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_time_window_s: Option<i32>,
    pub dedicated_worker: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_error_handler_muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_ttl: Option<i32>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct ListableScript {
    pub hash: ScriptHash,
    pub path: String,
    pub summary: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub archived: bool,
    pub extra_perms: serde_json::Value,
    pub language: ScriptLang,
    pub starred: bool,
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_draft: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_only: Option<bool>,
    pub has_deploy_errors: bool,
    pub ws_error_handler_muted: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx)]
#[cfg_attr(feature = "sqlx", sqlx(transparent))]
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
    pub tag: Option<String>,
    pub draft_only: Option<bool>,
    pub envs: Option<Vec<String>>,
    pub concurrent_limit: Option<i32>,
    pub concurrency_time_window_s: Option<i32>,
    pub cache_ttl: Option<i32>,
    pub dedicated_worker: Option<bool>,
    pub ws_error_handler_muted: Option<bool>,
    pub priority: Option<i16>,
}

#[derive(Deserialize)]
pub struct ListScriptQuery {
    pub path_start: Option<String>,
    pub path_exact: Option<String>,
    pub created_by: Option<String>,
    pub first_parent_hash: Option<ScriptHash>,
    pub last_parent_hash: Option<ScriptHash>,
    pub parent_hash: Option<ScriptHash>,
    pub show_archived: Option<bool>,
    pub order_by: Option<String>,
    pub order_desc: Option<bool>,
    pub is_template: Option<bool>,
    pub kinds: Option<String>,
    pub starred_only: Option<bool>,
}

pub fn to_i64(s: &str) -> crate::error::Result<i64> {
    let v = hex::decode(s)?;
    if v.len() < 8 {
        return Err(crate::error::Error::BadRequest(format!(
            "hex string did not decode to an u64: {s}",
        )));
    }
    let nb: u64 = u64::from_be_bytes(
        v[0..8]
            .try_into()
            .map_err(|_| hex::FromHexError::InvalidStringLength)?,
    );
    Ok(nb as i64)
}

pub fn to_hex_string(i: &i64) -> String {
    hex::encode(i.to_be_bytes())
}

#[cfg(feature = "reqwest")]
pub async fn get_hub_script_by_path(
    path: StripPath,
    http_client: &reqwest::Client,
    db: &DB,
) -> crate::error::Result<String> {
    let path = path
        .to_path()
        .strip_prefix("hub/")
        .ok_or_else(|| Error::BadRequest("Impossible to remove prefix hex".to_string()))?;

    let content = http_get_from_hub(
        http_client,
        &format!("https://hub.windmill.dev/raw/{path}.ts"),
        true,
        None,
        db,
    )
    .await?
    .text()
    .await
    .map_err(to_anyhow)?;
    Ok(content)
}

#[cfg(feature = "reqwest")]
pub async fn get_full_hub_script_by_path(
    path: StripPath,
    http_client: &reqwest::Client,
    db: &DB,
) -> crate::error::Result<HubScript> {
    let path = path
        .to_path()
        .strip_prefix("hub/")
        .ok_or_else(|| Error::BadRequest("Impossible to remove prefix hex".to_string()))?;

    let value = http_get_from_hub(
        http_client,
        &format!("https://hub.windmill.dev/raw2/{path}"),
        true,
        None,
        db,
    )
    .await?
    .json::<HubScript>()
    .await
    .map_err(to_anyhow)?;
    Ok(value)
}

#[derive(Deserialize, Serialize)]
pub struct HubScript {
    pub content: String,
    pub lockfile: Option<String>,
    pub language: ScriptLang,
    pub schema: serde_json::Value,
    pub summary: Option<String>,
}
