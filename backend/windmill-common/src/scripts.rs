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
    ops::Deref,
    str::FromStr,
};

use crate::{
    assets::AssetWithAltAccessType,
    error::{to_anyhow, Error},
    runnable_settings::{ConcurrencySettings, DebouncingSettings, RunnableSettings},
    utils::http_get_from_hub,
    workspace_dependencies::WorkspaceDependenciesAnnotatedRefs,
    DB, DEFAULT_HUB_BASE_URL, HUB_BASE_URL, PRIVATE_HUB_MIN_VERSION,
};

use crate::worker::HUB_CACHE_DIR;
use anyhow::Context;
use backon::ConstantBuilder;
use backon::{BackoffBuilder, Retryable};
use itertools::Itertools;
use regex::Regex;
use serde::de::Error as _;
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize};

use crate::utils::StripPath;

#[derive(
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Copy,
    Clone,
    Hash,
    Eq,
    sqlx::Type,
    Default,
    Ord,
    PartialOrd,
)]
#[sqlx(type_name = "SCRIPT_LANG", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ScriptLang {
    Nativets,
    #[default]
    Deno,
    Python3,
    Go,
    Bash,
    Powershell,
    Postgresql,
    Bun,
    Bunnative,
    Mysql,
    Bigquery,
    Snowflake,
    Graphql,
    Mssql,
    OracleDB,
    DuckDb,
    Php,
    Rust,
    Ansible,
    CSharp,
    Nu,
    Java,
    Ruby,
    // for related places search: ADD_NEW_LANG
}

impl ScriptLang {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScriptLang::Bun => "bun",
            ScriptLang::Bunnative => "bunnative",
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
            ScriptLang::OracleDB => "oracledb",
            ScriptLang::DuckDb => "duckdb",
            ScriptLang::Php => "php",
            ScriptLang::Rust => "rust",
            ScriptLang::Ansible => "ansible",
            ScriptLang::CSharp => "csharp",
            ScriptLang::Nu => "nu",
            ScriptLang::Java => "java",
            ScriptLang::Ruby => "ruby",
            // for related places search: ADD_NEW_LANG
        }
    }

    pub fn as_dependencies_filename(&self) -> Option<String> {
        use ScriptLang::*;
        Some(
            match self {
                Bun | Bunnative => "package.json",
                Python3 => "requirements.in",
                // Go => "go.mod",
                Php => "composer.json",
                _ => return None,
            }
            .to_owned(),
        )
    }

    pub fn as_comment_lit(&self) -> String {
        use ScriptLang::*;
        match self {
            Nativets | Bun | Bunnative | Deno | Go | Php | CSharp | Java => "//",
            Python3 | Bash | Powershell | Graphql | Ansible | Nu | Ruby => "#",
            Postgresql | Mysql | Bigquery | Snowflake | Mssql | OracleDB | DuckDb => "--",
            Rust => "//!",
            // for related places search: ADD_NEW_LANG
        }
        .to_owned()
    }

    pub fn extract_workspace_dependencies_annotated_refs(
        &self,
        code: &str,
        runnable_path: &str,
    ) -> Option<WorkspaceDependenciesAnnotatedRefs<String>> {
        use ScriptLang::*;
        lazy_static::lazy_static! {
            static ref RE_PYTHON: Regex = Regex::new(r"^\#\s?(\S+)\s*$").unwrap();
        }
        match self {
            // TODO: Maybe use regex
            Bun | Bunnative => WorkspaceDependenciesAnnotatedRefs::parse(
                "//",
                "package_json",
                code,
                None,
                runnable_path,
            ),
            Python3 => WorkspaceDependenciesAnnotatedRefs::parse(
                "#",
                "requirements",
                code,
                Some(&RE_PYTHON),
                runnable_path,
            ),
            Go => {
                WorkspaceDependenciesAnnotatedRefs::parse("//", "go_mod", code, None, runnable_path)
            }
            Php => WorkspaceDependenciesAnnotatedRefs::parse(
                "//",
                "composer_json",
                code,
                None,
                runnable_path,
            ),
            _ => return None,
        }
    }
}

impl FromStr for ScriptLang {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let language = match s.to_lowercase().as_str() {
            "bun" => ScriptLang::Bun,
            "bunnative" => ScriptLang::Bunnative,
            "nativets" => ScriptLang::Nativets,
            "deno" => ScriptLang::Deno,
            "python3" => ScriptLang::Python3,
            "go" => ScriptLang::Go,
            "bash" => ScriptLang::Bash,
            "powershell" => ScriptLang::Powershell,
            "postgresql" => ScriptLang::Postgresql,
            "mysql" => ScriptLang::Mysql,
            "bigquery" => ScriptLang::Bigquery,
            "snowflake" => ScriptLang::Snowflake,
            "mssql" => ScriptLang::Mssql,
            "graphql" => ScriptLang::Graphql,
            "oracledb" => ScriptLang::OracleDB,
            "php" => ScriptLang::Php,
            "rust" => ScriptLang::Rust,
            "ansible" => ScriptLang::Ansible,
            "csharp" => ScriptLang::CSharp,
            "nu" => ScriptLang::Nu,
            "java" => ScriptLang::Java,
            "ruby" => ScriptLang::Ruby,
            // for related places search: ADD_NEW_LANG
            language => {
                return Err(anyhow::anyhow!("{} is currently not supported", language).into())
            }
        };

        Ok(language)
    }
}

#[derive(Eq, PartialEq, Debug, Hash, Clone, Copy, sqlx::Type)]
#[sqlx(transparent)]
pub struct ScriptHash(pub i64);

impl Deref for ScriptHash {
    type Target = i64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Into<u64> for ScriptHash {
    fn into(self) -> u64 {
        self.0 as u64
    }
}

impl From<i64> for ScriptHash {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

#[derive(PartialEq, sqlx::Type, Debug)]
#[sqlx(transparent, no_pg_array)]
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
        let i = to_i64(&s).map_err(|e| {
            tracing::error!("Could not deserialize ScriptHash. Note, input should be in Hex and digit amount should be divisible by 16 (can be padded). err: {}", &e);
            D::Error::custom(format!("{}", e))
        })?;
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

#[derive(Serialize, Deserialize, Debug, Hash, sqlx::Type)]
#[sqlx(type_name = "SCRIPT_KIND", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ScriptKind {
    Trigger,
    Failure,
    Script,
    Approval,
    Preprocessor,
}

impl Display for ScriptKind {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(match self {
            ScriptKind::Trigger => "trigger",
            ScriptKind::Failure => "failure",
            ScriptKind::Script => "script",
            ScriptKind::Approval => "approval",
            ScriptKind::Preprocessor => "preprocessor",
        })?;
        Ok(())
    }
}

const PREVIEW_IS_CODEBASE_HASH: i64 = -42;
const PREVIEW_IS_TAR_CODEBASE_HASH: i64 = -43;
const PREVIEW_IS_ESM_CODEBASE_HASH: i64 = -44;
const PREVIEW_IS_TAR_ESM_CODEBASE_HASH: i64 = -45;

pub fn is_special_codebase_hash(hash: i64) -> bool {
    hash == PREVIEW_IS_CODEBASE_HASH
        || hash == PREVIEW_IS_TAR_CODEBASE_HASH
        || hash == PREVIEW_IS_ESM_CODEBASE_HASH
        || hash == PREVIEW_IS_TAR_ESM_CODEBASE_HASH
}

pub fn codebase_to_hash(is_tar: bool, is_esm: bool) -> i64 {
    if is_tar {
        if is_esm {
            PREVIEW_IS_TAR_ESM_CODEBASE_HASH
        } else {
            PREVIEW_IS_TAR_CODEBASE_HASH
        }
    } else {
        if is_esm {
            PREVIEW_IS_ESM_CODEBASE_HASH
        } else {
            PREVIEW_IS_CODEBASE_HASH
        }
    }
}

pub fn hash_to_codebase_id(job_id: &str, hash: i64) -> Option<String> {
    match hash {
        PREVIEW_IS_CODEBASE_HASH => Some(job_id.to_string()),
        PREVIEW_IS_TAR_CODEBASE_HASH => Some(format!("{}.tar", job_id)),
        PREVIEW_IS_ESM_CODEBASE_HASH => Some(format!("{}.esm", job_id)),
        PREVIEW_IS_TAR_ESM_CODEBASE_HASH => Some(format!("{}.esm.tar", job_id)),
        _ => None,
    }
}

pub struct CodebaseInfo {
    pub is_tar: bool,
    pub is_esm: bool,
}

pub fn id_to_codebase_info(id: &str) -> CodebaseInfo {
    let is_tar = id.ends_with(".tar");
    let is_esm = id.contains(".esm");
    CodebaseInfo { is_tar, is_esm }
}
#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct Script<SR> {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_template: Option<bool>,
    pub extra_perms: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    pub dedicated_worker: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_error_handler_muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_ttl: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_ignore_s3_path: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_after_use: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_unless_cancelled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_to_runner_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_main_func: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_preprocessor: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_behalf_of_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[sqlx(json(nullable))]
    pub assets: Option<Vec<AssetWithAltAccessType>>,
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub runnable_settings: SR,
}

// Not serializable
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct ScriptRunnableSettingsHandle {
    // legacy - for backwards compatibility
    // don't add new values.
    pub concurrency_key: Option<String>,
    pub concurrent_limit: Option<i32>,
    pub concurrency_time_window_s: Option<i32>,
    pub debounce_key: Option<String>,
    pub debounce_delay_s: Option<i32>,

    // add here as well.
    pub runnable_settings_handle: Option<i64>,
}

// Not sqlx queriable
#[derive(Serialize, Debug, Clone, Default)]
pub struct ScriptRunnableSettingsInline {
    #[serde(flatten)]
    pub concurrency_settings: ConcurrencySettings,
    #[serde(flatten)]
    pub debouncing_settings: DebouncingSettings,
}

impl Script<ScriptRunnableSettingsHandle> {
    pub async fn prefetch_cached<'a>(
        self,
        db: &DB,
    ) -> crate::error::Result<Script<ScriptRunnableSettingsInline>> {
        let (debouncing_settings, concurrency_settings) =
            RunnableSettings::from_runnable_settings_handle(
                self.runnable_settings.runnable_settings_handle,
                db,
            )
            .await?
            .prefetch_cached(db)
            .await?;

        Ok(Script {
            workspace_id: self.workspace_id,
            hash: self.hash,
            path: self.path,
            parent_hashes: self.parent_hashes,
            summary: self.summary,
            description: self.description,
            content: self.content,
            created_by: self.created_by,
            created_at: self.created_at,
            archived: self.archived,
            schema: self.schema,
            deleted: self.deleted,
            is_template: self.is_template,
            extra_perms: self.extra_perms,
            lock: self.lock,
            lock_error_logs: self.lock_error_logs,
            language: self.language,
            kind: self.kind,
            tag: self.tag,
            draft_only: self.draft_only,
            envs: self.envs,
            dedicated_worker: self.dedicated_worker,
            ws_error_handler_muted: self.ws_error_handler_muted,
            priority: self.priority,
            cache_ttl: self.cache_ttl,
            cache_ignore_s3_path: self.cache_ignore_s3_path,
            timeout: self.timeout,
            delete_after_use: self.delete_after_use,
            restart_unless_cancelled: self.restart_unless_cancelled,
            visible_to_runner_only: self.visible_to_runner_only,
            no_main_func: self.no_main_func,
            codebase: self.codebase,
            has_preprocessor: self.has_preprocessor,
            on_behalf_of_email: self.on_behalf_of_email,
            assets: self.assets,
            runnable_settings: ScriptRunnableSettingsInline {
                concurrency_settings: concurrency_settings.maybe_fallback(
                    self.runnable_settings.concurrency_key,
                    self.runnable_settings.concurrent_limit,
                    self.runnable_settings.concurrency_time_window_s,
                ),
                debouncing_settings: debouncing_settings.maybe_fallback(
                    self.runnable_settings.debounce_key,
                    self.runnable_settings.debounce_delay_s,
                ),
            },
        })
    }
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ScriptWithStarred<SR> {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub script: Script<SR>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred: Option<bool>,
}
impl ScriptWithStarred<ScriptRunnableSettingsHandle> {
    pub async fn prefetch_cached<'a>(
        self,
        db: &DB,
    ) -> crate::error::Result<ScriptWithStarred<ScriptRunnableSettingsInline>> {
        Ok(ScriptWithStarred {
            script: self.script.prefetch_cached(db).await?,
            starred: self.starred,
        })
    }
}

#[derive(Serialize, sqlx::FromRow)]
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
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_draft: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_only: Option<bool>,
    pub has_deploy_errors: bool,
    pub ws_error_handler_muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_main_func: Option<bool>,
    #[serde(skip_serializing_if = "is_false")]
    pub use_codebase: bool,
    #[sqlx(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_msg: Option<String>,
    pub kind: ScriptKind,
}

fn is_false(x: &bool) -> bool {
    return !x;
}

#[derive(Serialize)]
pub struct ScriptHistory {
    pub script_hash: ScriptHash,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_msg: Option<String>,
}

#[derive(Deserialize)]
pub struct ScriptHistoryUpdate {
    pub deployment_msg: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, sqlx::Type, Clone)]
#[sqlx(transparent)]
#[serde(transparent)]
pub struct Schema(pub sqlx::types::Json<Box<serde_json::value::RawValue>>);

impl Hash for Schema {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.get().hash(state);
    }
}

#[derive(Serialize, Deserialize, Hash, Debug)]
pub struct NewScript {
    pub path: String,
    pub parent_hash: Option<ScriptHash>,
    pub summary: String,
    pub description: String,
    pub content: String,
    pub schema: Option<Schema>,
    pub is_template: Option<bool>,
    #[serde(default = "Option::default")]
    #[serde(deserialize_with = "lock_deserialize")]
    pub lock: Option<String>,
    pub language: ScriptLang,
    pub kind: Option<ScriptKind>,
    pub tag: Option<String>,
    pub draft_only: Option<bool>,
    pub envs: Option<Vec<String>>,
    #[serde(flatten)]
    pub concurrency_settings: ConcurrencySettings,
    #[serde(flatten)]
    pub debouncing_settings: DebouncingSettings,
    pub cache_ttl: Option<i32>,
    pub cache_ignore_s3_path: Option<bool>,
    pub dedicated_worker: Option<bool>,
    pub ws_error_handler_muted: Option<bool>,
    pub priority: Option<i16>,
    pub timeout: Option<i32>,
    pub delete_after_use: Option<bool>,
    pub restart_unless_cancelled: Option<bool>,
    pub deployment_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_to_runner_only: Option<bool>,
    pub no_main_func: Option<bool>,
    pub codebase: Option<String>,
    pub has_preprocessor: Option<bool>,
    pub on_behalf_of_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<Vec<AssetWithAltAccessType>>,
}

fn lock_deserialize<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    struct StringOrArrayVisitor;

    impl<'de> serde::de::Visitor<'de> for StringOrArrayVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("either a string or an array of strings")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(v.to_string()))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut split_lock: Vec<String> = vec![];
            loop {
                if let Ok(Some(elem)) = seq.next_element::<String>() {
                    split_lock.push(elem);
                } else {
                    break;
                }
            }
            let lock = split_lock.join("\n");
            return Ok(Some(lock));
        }
    }
    deserializer.deserialize_any(StringOrArrayVisitor)
}

#[derive(Debug, Deserialize)]
pub struct ListScriptQuery {
    pub without_description: Option<bool>,
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
    pub include_without_main: Option<bool>,
    pub include_draft_only: Option<bool>,
    pub with_deployment_msg: Option<bool>,
    #[serde(default, deserialize_with = "from_seq")]
    pub languages: Option<Vec<ScriptLang>>,
}

fn from_seq<'de, D>(deserializer: D) -> Result<Option<Vec<ScriptLang>>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = <String>::deserialize(deserializer)?;

    let languages: Vec<ScriptLang> = s
        .split(",")
        .map(ScriptLang::from_str)
        .try_collect()
        .map_err(|e| serde::de::Error::custom(e.to_string()))?;

    let languages = if languages.is_empty() {
        None
    } else {
        Some(languages)
    };

    Ok(languages)
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

pub async fn get_hub_script_by_path(
    path: StripPath,
    http_client: &reqwest::Client,
    db: &DB,
) -> crate::error::Result<String> {
    let path = path
        .to_path()
        .strip_prefix("hub/")
        .ok_or_else(|| Error::BadRequest("Impossible to remove prefix hex".to_string()))?;

    let hub_base_url = HUB_BASE_URL.read().await.clone();

    //
    let result = http_get_from_hub(
        http_client,
        &format!("{}/raw/{}.ts", hub_base_url, path),
        true,
        None,
        Some(db),
    )
    .await?
    .error_for_status()
    .map_err(to_anyhow)?
    .text()
    .await
    .map_err(to_anyhow);

    match result {
        Ok(result) => Ok(result),
        Err(e) => {
            if hub_base_url != DEFAULT_HUB_BASE_URL
                && path
                    .split("/")
                    .next()
                    .is_some_and(|x| x.parse::<i32>().is_ok_and(|x| x < PRIVATE_HUB_MIN_VERSION))
            {
                tracing::info!(
                    "Not found on private hub, fallback to default hub for {}",
                    path
                );
                let content = http_get_from_hub(
                    http_client,
                    &format!("{}/raw/{}.ts", DEFAULT_HUB_BASE_URL, path),
                    true,
                    None,
                    Some(db),
                )
                .await?
                .error_for_status()
                .map_err(to_anyhow)?
                .text()
                .await
                .map_err(to_anyhow)?;

                Ok(content)
            } else {
                Err(e)?
            }
        }
    }
}

pub async fn get_full_hub_script_by_path(
    path: StripPath,
    http_client: &reqwest::Client,
    db: Option<&DB>,
) -> crate::error::Result<HubScript> {
    let path = path
        .to_path()
        .strip_prefix("hub/")
        .ok_or_else(|| Error::BadRequest("Impossible to remove prefix hex".to_string()))?;

    let mut path_iterator = path.split("/");
    let version = path_iterator
        .next()
        .ok_or_else(|| Error::internal_err(format!("expected hub path to have version number")))?;
    let cache_path = format!("{HUB_CACHE_DIR}/{version}");
    let script;
    if tokio::fs::metadata(&cache_path).await.is_err() {
        script = get_full_hub_script_by_path_inner(path, http_client, db).await?;
        if let Err(e) = crate::worker::write_file(
            HUB_CACHE_DIR,
            &version,
            &serde_json::to_string(&script).map_err(to_anyhow)?,
        ) {
            tracing::error!("failed to write hub script {path} to cache: {e}");
        } else {
            tracing::info!("wrote hub script {path} to cache");
        }
    } else {
        let cache_content = tokio::fs::read_to_string(cache_path).await?;
        script = serde_json::from_str(&cache_content).unwrap();
        tracing::info!("read hub script {path} from cache");
    }
    Ok(script)
}

async fn get_full_hub_script_by_path_inner(
    path: &str,
    http_client: &reqwest::Client,
    db: Option<&DB>,
) -> crate::error::Result<HubScript> {
    let hub_base_url = HUB_BASE_URL.read().await.clone();

    let response = (|| async {
        let response = http_get_from_hub(
            http_client,
            &format!("{}/raw2/{}", hub_base_url, path),
            true,
            None,
            db,
        )
        .await
        .and_then(|r| r.error_for_status().map_err(|e| to_anyhow(e).into()));

        match response {
            Ok(response) => Ok(response),
            Err(e) => {
                if hub_base_url != DEFAULT_HUB_BASE_URL
                    && path.split("/").next().is_some_and(|x| {
                        x.parse::<i32>().is_ok_and(|x| x < PRIVATE_HUB_MIN_VERSION)
                    })
                {
                    // TODO: should only fallback to default hub if status is 404 (hub returns 500 currently)
                    tracing::info!(
                        "Not found on private hub, fallback to default hub for {}",
                        path
                    );
                    http_get_from_hub(
                        http_client,
                        &format!("{}/raw2/{}", DEFAULT_HUB_BASE_URL, path),
                        true,
                        None,
                        db,
                    )
                    .await?
                    .error_for_status()
                    .map_err(|e| to_anyhow(e).into())
                } else {
                    Err(e)
                }
            }
        }
    })
    .retry(
        ConstantBuilder::default()
            .with_delay(std::time::Duration::from_secs(5))
            .with_max_times(2)
            .build(),
    )
    .notify(|err, dur| {
        tracing::warn!(
            "Could not get hub script at path {path}, retrying in {dur:#?}, err: {err:#?}"
        );
    })
    .sleep(tokio::time::sleep)
    .await?;

    let script = response
        .json::<HubScript>()
        .await
        .context(format!("Decoding hub response for script at path {path}"))?;

    Ok(script)
}

#[derive(Deserialize, Serialize)]
pub struct HubScript {
    pub content: String,
    pub lockfile: Option<String>,
    pub language: ScriptLang,
    pub schema: Box<serde_json::value::RawValue>,
    pub summary: Option<String>,
}

pub fn hash_script(ns: &NewScript) -> i64 {
    let mut dh = std::hash::DefaultHasher::new();
    ns.hash(&mut dh);
    dh.finish() as i64
}

pub async fn fetch_script_for_update<'a>(
    path: &str,
    w_id: &str,
    e: impl sqlx::Executor<'a, Database = sqlx::Postgres>,
) -> crate::error::Result<Option<Script<ScriptRunnableSettingsHandle>>> {
    sqlx::query_as::<_, Script<ScriptRunnableSettingsHandle>>(
        "SELECT
            workspace_id,
            hash,
            path,
            parent_hashes,
            summary,
            description,
            content,
            created_by,
            created_at,
            archived,
            schema,
            deleted,
            is_template,
            extra_perms,
            lock,
            lock_error_logs,
            language,
            kind,
            tag,
            draft_only,
            envs,
            concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            debounce_key,
            debounce_delay_s,
            dedicated_worker,
            runnable_settings_handle,
            ws_error_handler_muted,
            priority,
            cache_ttl,
            cache_ignore_s3_path,
            timeout,
            delete_after_use,
            restart_unless_cancelled,
            visible_to_runner_only,
            no_main_func,
            codebase,
            has_preprocessor,
            on_behalf_of_email,
            assets
         FROM script WHERE path = $1 AND workspace_id = $2 AND archived = false ORDER BY created_at DESC LIMIT 1 FOR UPDATE",
    )
    .bind(path)
    .bind(w_id)
    .fetch_optional(e)
    .await
    .map_err(crate::error::Error::from)
}

pub struct ClonedScript {
    pub old_script: NewScript,
    pub new_hash: i64,
}
// TODO: What if dependency job fails, there is script with NULL in the lock
pub async fn clone_script<'c>(
    path: &str,
    w_id: &str,
    deployment_message: Option<String>,
    db: &DB,
) -> crate::error::Result<ClonedScript> {
    let mut tx = db.begin().await?;
    let s = if let Some(s) = fetch_script_for_update(path, w_id, &mut *tx).await? {
        s
    } else {
        return Err(crate::error::Error::NotFound(format!(
            "Non-archived script with path '{}' not found", path
        )));
    };

    let (debouncing_settings, concurrency_settings) =
        RunnableSettings::from_runnable_settings_handle(
            s.runnable_settings.runnable_settings_handle,
            db,
        )
        .await?
        .prefetch_cached(db)
        .await?;

    let ns = NewScript {
        path: s.path.clone(),
        parent_hash: Some(s.hash),
        summary: s.summary,
        description: s.description,
        content: s.content,
        schema: s.schema,
        is_template: s.is_template,
        // TODO: Make it either None everywhere (particularly when raw reqs are calculated)
        // Or handle this case and conditionally make Some (only with raw reqs)
        lock: None,
        language: s.language,
        kind: Some(s.kind),
        tag: s.tag,
        draft_only: s.draft_only,
        envs: s.envs,
        concurrency_settings: concurrency_settings.maybe_fallback(
            s.runnable_settings.concurrency_key,
            s.runnable_settings.concurrent_limit,
            s.runnable_settings.concurrency_time_window_s,
        ),
        debouncing_settings: debouncing_settings.maybe_fallback(
            s.runnable_settings.debounce_key,
            s.runnable_settings.debounce_delay_s,
        ),
        cache_ttl: s.cache_ttl,
        cache_ignore_s3_path: s.cache_ignore_s3_path,
        dedicated_worker: s.dedicated_worker,
        ws_error_handler_muted: s.ws_error_handler_muted,
        priority: s.priority,
        timeout: s.timeout,
        delete_after_use: s.delete_after_use,
        restart_unless_cancelled: s.restart_unless_cancelled,
        deployment_message,
        visible_to_runner_only: s.visible_to_runner_only,
        no_main_func: s.no_main_func,
        codebase: s.codebase,
        has_preprocessor: s.has_preprocessor,
        on_behalf_of_email: s.on_behalf_of_email,
        assets: s.assets,
    };

    let new_hash = hash_script(&ns);

    tracing::debug!(
        "cloning script at path {} from '{}' to '{}'",
        s.path,
        *s.hash,
        new_hash
    );

    sqlx::query!("
    INSERT INTO script
    (workspace_id, hash, path, parent_hashes, summary, description, content, \
    created_by, schema, is_template, extra_perms, lock, language, kind, tag, \
    draft_only, envs, concurrent_limit, concurrency_time_window_s, cache_ttl, cache_ignore_s3_path, \
    dedicated_worker, ws_error_handler_muted, priority, restart_unless_cancelled, \
    delete_after_use, timeout, concurrency_key, visible_to_runner_only, no_main_func, \
    codebase, has_preprocessor, on_behalf_of_email, schema_validation, assets, debounce_key, debounce_delay_s, runnable_settings_handle)

    SELECT  workspace_id, $1, path, array_prepend($2::bigint, COALESCE(parent_hashes, '{}'::bigint[])), summary, description, \
            content, created_by, schema, is_template, extra_perms, NULL, language, kind, tag, \
            draft_only, envs, concurrent_limit, concurrency_time_window_s, cache_ttl, cache_ignore_s3_path, \
            dedicated_worker, ws_error_handler_muted, priority, restart_unless_cancelled, \
            delete_after_use, timeout, concurrency_key, visible_to_runner_only, no_main_func, \
            codebase, has_preprocessor, on_behalf_of_email, schema_validation, assets, debounce_key, debounce_delay_s, runnable_settings_handle

    FROM script WHERE hash = $2 AND workspace_id = $3;
            ", new_hash, s.hash.0, w_id).execute(&mut *tx).await?;

    // Archive base.
    sqlx::query!(
        "UPDATE script SET archived = true WHERE hash = $1 AND workspace_id = $2",
        *s.hash,
        w_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(ClonedScript { old_script: ns, new_hash })
}
