use std::{
    fmt::{self, Display},
    hash::{Hash, Hasher},
    ops::Deref,
    str::FromStr,
};

use itertools::Itertools;
use serde::de::Error as _;
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize};

use crate::{
    assets::AssetWithAltAccessType,
    runnable_settings::{ConcurrencySettings, DebouncingSettings},
};

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
                Bun | Bunnative | Nativets => "package.json",
                Python3 => "requirements.in",
                // Go => "go.mod",
                Php => "composer.json",
                _ => return None,
            }
            .to_owned(),
        )
    }

    pub fn is_native(&self) -> bool {
        matches!(
            self,
            ScriptLang::Bunnative |
            ScriptLang::Nativets |
            ScriptLang::Postgresql |
            ScriptLang::Mysql |
            ScriptLang::Graphql |
            ScriptLang::Snowflake |
            ScriptLang::Mssql |
            ScriptLang::Bigquery |
            ScriptLang::OracleDB
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
}

impl FromStr for ScriptLang {
    type Err = anyhow::Error;
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
            language => return Err(anyhow::anyhow!("{} is currently not supported", language)),
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

#[derive(Serialize, sqlx::FromRow)]
pub struct ScriptWithStarred<SR> {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub script: Script<SR>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred: Option<bool>,
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
    pub dedicated_worker: Option<bool>,
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
        .map_err(|e: anyhow::Error| serde::de::Error::custom(e.to_string()))?;

    let languages = if languages.is_empty() {
        None
    } else {
        Some(languages)
    };

    Ok(languages)
}

pub fn to_i64(s: &str) -> anyhow::Result<i64> {
    let v = hex::decode(s)?;
    if v.len() < 8 {
        return Err(anyhow::anyhow!("hex string did not decode to an u64: {s}",));
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

#[derive(Deserialize, Serialize)]
pub struct HubScript {
    pub content: String,
    pub lockfile: Option<String>,
    pub language: ScriptLang,
    pub schema: Box<serde_json::value::RawValue>,
    pub summary: Option<String>,
}

pub fn hash_script(ns: impl std::hash::Hash) -> i64 {
    let mut dh = std::hash::DefaultHasher::new();
    ns.hash(&mut dh);
    dh.finish() as i64
}
