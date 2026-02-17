#[cfg(unix)]
use anyhow::anyhow;
use axum::http::HeaderMap;
use bytes::Bytes;
use const_format::concatcp;
use itertools::Itertools;
use regex::Regex;
use reqwest_middleware::ClientWithMiddleware;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, value::RawValue};
use sqlx::{types::Json, Pool, Postgres};
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::Write,
    ops::Deref,
    panic::Location,
    path::{Component, Path, PathBuf},
    str::FromStr,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
#[cfg(windows)]
use sysinfo::System;
use tokio::{sync::RwLock, time::timeout};
use uuid::Uuid;
use windmill_macros::annotations;

use crate::{
    agent_workers::PingJobStatusResponse,
    cache::{unwrap_or_error, RawNode, RawScript},
    error::{self, to_anyhow},
    global_settings::CUSTOM_TAGS_SETTING,
    indexer::TantivyIndexerSettings,
    server::Smtp,
    utils::{merge_nested_raw_values_to_array, merge_raw_values_to_array},
    KillpillSender, DB,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CustomTags {
    pub global: Vec<String>,
    pub specific: HashMap<String, SpecificTagData>,
}

impl CustomTags {
    pub fn from(tags: Vec<String>) -> Self {
        let mut global = vec![];
        let mut specific: HashMap<String, SpecificTagData> = HashMap::new();
        for e in tags {
            if let Some(cap) = CUSTOM_TAG_REGEX.captures(&e) {
                let tag_name = cap.get(1).unwrap().as_str().to_string();
                let workspace_str = cap.get(2).unwrap().as_str();
                let tag_type = SpecificTagType::from_regex_string(workspace_str);
                let workspaces: Vec<String> = workspace_str
                    .split(tag_type.corresponding_separator())
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
                if workspaces.is_empty() {
                    tracing::warn!("Ignoring tag `{}` with empty exclusion/inclusion list", e);
                    global.push(e);
                    continue;
                }
                specific.insert(tag_name, SpecificTagData { tag_type, workspaces });
            } else {
                global.push(e.to_string());
            }
        }
        Self { global, specific }
    }

    pub fn to_string_vec(&self, filter_with_workspace: Option<String>) -> Vec<String> {
        let specific = if let Some(workspace) = filter_with_workspace {
            self.specific
                .iter()
                .filter(|(_, tag_data)| tag_data.applies_to_workspace(&workspace))
                .map(|(tag, _)| tag.clone())
                .collect::<Vec<String>>()
        } else {
            self.specific
                .iter()
                .map(|(tag, tag_data)| {
                    let separator = tag_data.tag_type.corresponding_separator();
                    let mut workspaces = tag_data.workspaces.join(&*separator.to_string());
                    if tag_data.tag_type == SpecificTagType::AllExcluding {
                        // the AllExcluding tag syntax has a leading separator
                        workspaces.insert(0, separator);
                    }
                    format!("{}({})", tag, workspaces)
                })
                .collect::<Vec<String>>()
        };
        let all_tags = self.global.clone();
        all_tags.into_iter().chain(specific.into_iter()).collect()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpecificTagData {
    pub tag_type: SpecificTagType,
    pub workspaces: Vec<String>,
}

impl SpecificTagData {
    pub fn applies_to_workspace(&self, workspace_id: &str) -> bool {
        match self.tag_type {
            SpecificTagType::AllExcluding => !self.workspaces.contains(&workspace_id.to_string()),
            SpecificTagType::NoneExcept => self.workspaces.contains(&workspace_id.to_string()),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpecificTagType {
    AllExcluding,
    NoneExcept,
}

impl SpecificTagType {
    pub fn corresponding_separator(&self) -> char {
        match self {
            SpecificTagType::AllExcluding => '^',
            SpecificTagType::NoneExcept => '+',
        }
    }

    pub fn from_regex_string(workspaces_str: &str) -> Self {
        if workspaces_str.contains(SpecificTagType::AllExcluding.corresponding_separator()) {
            SpecificTagType::AllExcluding
        } else {
            // Regex match ensures the second branch is correct
            SpecificTagType::NoneExcept
        }
    }
}

pub const DEFAULT_CLOUD_TIMEOUT: u64 = 900;
pub const DEFAULT_SELFHOSTED_TIMEOUT: u64 = 604800; // 7 days
pub const MIN_PERIODIC_SCRIPT_INTERVAL_SECONDS: u64 = 60;
lazy_static::lazy_static! {
    pub static ref WORKER_GROUP: String = std::env::var("WORKER_GROUP").unwrap_or_else(|_| {
        #[cfg(not(feature = "enterprise"))]
        {
            "default".to_string()
        }

        #[cfg(feature = "enterprise")]
        {
            if let Some(token) = crate::agent_workers::DECODED_AGENT_TOKEN.as_ref() {
                token.worker_group.clone()
            } else {
                "default".to_string()
            }
        }
    });

    pub static ref NO_LOGS: bool = std::env::var("NO_LOGS").ok().is_some_and(|x| x == "1" || x == "true");

    pub static ref NATIVE_MODE: bool = std::env::var("NATIVE_MODE").ok().is_some_and(|x| x == "1" || x == "true");

    pub static ref CGROUP_V2_PATH_RE: Regex = Regex::new(r#"(?m)^0::(/.*)$"#).unwrap();
    pub static ref CGROUP_V2_CPU_RE: Regex = Regex::new(r#"(?m)^(\d+) \S+$"#).unwrap();
    pub static ref CGROUP_V1_INACTIVE_FILE_RE: Regex = Regex::new(r#"(?m)^total_inactive_file (\d+)$"#).unwrap();
    pub static ref CGROUP_V2_INACTIVE_FILE_RE: Regex = Regex::new(r#"(?m)^inactive_file (\d+)$"#).unwrap();

    pub static ref DEFAULT_TAGS: Vec<String> = vec![
        "deno".to_string(),
        "python3".to_string(),
        "go".to_string(),
        "bash".to_string(),
        "powershell".to_string(),
        "nativets".to_string(),
        "mysql".to_string(),
        "oracledb".to_string(),
        "bun".to_string(),
        "postgresql".to_string(),
        "bigquery".to_string(),
        "snowflake".to_string(),
        "mssql".to_string(),
        "graphql".to_string(),
        "php".to_string(),
        "rust".to_string(),
        "ansible".to_string(),
        "csharp".to_string(),
        "nu".to_string(),
        "java".to_string(),
        "ruby".to_string(),
        "duckdb".to_string(),
        // for related places search: ADD_NEW_LANG
        "dependency".to_string(),
        "flow".to_string(),
        "other".to_string()
    ];

    pub static ref NATIVE_TAGS: Vec<String> = vec![
        "nativets".to_string(),
        "postgresql".to_string(),
        "mysql".to_string(),
        "graphql".to_string(),
        "snowflake".to_string(),
        "mssql".to_string(),
        "bigquery".to_string(),
        "oracledb".to_string()
        // for related places search: ADD_NEW_LANG
    ];

    pub static ref DEFAULT_TAGS_PER_WORKSPACE: AtomicBool = AtomicBool::new(false);
    pub static ref DEFAULT_TAGS_WORKSPACES: Arc<RwLock<Option<Vec<String>>>> = Arc::new(RwLock::new(None));

    pub static ref MAX_TIMEOUT: u64 = std::env::var("TIMEOUT")
    .ok()
    .and_then(|x| x.parse::<u64>().ok())
    .unwrap_or_else(|| if *CLOUD_HOSTED { DEFAULT_CLOUD_TIMEOUT } else { DEFAULT_SELFHOSTED_TIMEOUT });

    pub static ref SCRIPT_TOKEN_EXPIRY: u64 = std::env::var("SCRIPT_TOKEN_EXPIRY")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
        .unwrap_or(*MAX_TIMEOUT);

    pub static ref WORKER_CONFIG: Arc<RwLock<WorkerConfig>> = Arc::new(RwLock::new(WorkerConfig {
        worker_tags: Default::default(),
        priority_tags_sorted: Default::default(),
        dedicated_worker: Default::default(),
        dedicated_workers: Default::default(),
        cache_clear: Default::default(),
        init_bash: Default::default(),
        periodic_script_bash: Default::default(),
        periodic_script_interval_seconds: Default::default(),
        additional_python_paths: Default::default(),
        pip_local_dependencies: Default::default(),
        env_vars: Default::default(),
        native_mode: false,
    }));

    pub static ref WORKER_PULL_QUERIES: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(vec![]));
    pub static ref WORKER_SUSPENDED_PULL_QUERY: Arc<RwLock<String>> = Arc::new(RwLock::new("".to_string()));


    pub static ref SMTP_CONFIG: Arc<RwLock<Option<Smtp>>> = Arc::new(RwLock::new(None));
    pub static ref INDEXER_CONFIG: Arc<RwLock<TantivyIndexerSettings>> = Arc::new(RwLock::new(TantivyIndexerSettings::default()));


    pub static ref CLOUD_HOSTED: bool = std::env::var("CLOUD_HOSTED").is_ok();

    pub static ref CUSTOM_TAGS: Vec<String> = std::env::var("CUSTOM_TAGS")
        .ok()
        .map(|x| x.split(',').map(|x| x.to_string()).collect::<Vec<_>>()).unwrap_or_default();


    pub static ref CUSTOM_TAGS_PER_WORKSPACE: Arc<RwLock<CustomTags>> = Arc::new(RwLock::new(CustomTags::default()));

    pub static ref ALL_TAGS: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(vec![]));



    //    ^([\w-]+)         # Group 1: tag name
    //    \(                # Literal '('
    //    (                 # Group 2: the full workspace list
    //      (?:[\w-]+\+)*[\w-]+     # NoneExcept pattern: ws1+ws2
    //      |                      # OR
    //      (?:\^[\w-]+)+          # AllExcluding pattern: ^ws1^ws2
    //    )
    //    \)$               # Closing ')'
    static ref CUSTOM_TAG_REGEX: Regex = Regex::new(r"^([\w-]+)\(((?:[\w-]+\+)*[\w-]+|(?:\^[\w-]+)+)\)$").unwrap();

    pub static ref DISABLE_BUNDLING: bool = std::env::var("DISABLE_BUNDLING")
    .ok()
    .and_then(|x| x.parse::<bool>().ok())
    .unwrap_or(false);

    // Features flags:
    pub static ref DISABLE_FLOW_SCRIPT: bool = std::env::var("DISABLE_FLOW_SCRIPT").ok().is_some_and(|x| x == "1" || x == "true");

    pub static ref ROOT_STANDALONE_BUNDLE_DIR: String = format!("{}/.windmill/standalone_bundle", std::env::var("HOME").unwrap_or_else(|_| "/root".to_string()));
}

pub const ROOT_CACHE_NOMOUNT_DIR: &str = concatcp!(TMP_DIR, "/cache_nomount/");

/// Whether native mode is forced by the environment (NATIVE_MODE=true env var or WORKER_GROUP=native).
/// This does NOT account for native_mode set in the DB worker group config — for that, read
/// `WORKER_CONFIG.native_mode` which combines all sources.
pub fn is_native_mode_from_env() -> bool {
    *NATIVE_MODE || *WORKER_GROUP == "native"
}

/// Cached resolved native mode flag, updated when worker config is reloaded.
/// Use this for hot-path checks (e.g. per-job dispatch) to avoid read-locking WORKER_CONFIG.
pub static NATIVE_MODE_RESOLVED: AtomicBool = AtomicBool::new(false);

pub static MIN_VERSION_IS_LATEST: AtomicBool = AtomicBool::new(false);
#[derive(Clone)]
pub struct HttpClient {
    pub client: ClientWithMiddleware,
    pub base_internal_url: String,
}

impl Deref for HttpClient {
    type Target = ClientWithMiddleware;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl HttpClient {
    pub async fn post<T: Serialize, R: DeserializeOwned>(
        &self,
        url: &str,
        headers: Option<HeaderMap>,
        body: &T,
    ) -> anyhow::Result<R> {
        let base_url = self.base_internal_url.clone();

        let response_builder = self.client.post(format!("{}{}", base_url, url)).json(body);

        let response_builder = match headers {
            Some(headers) => response_builder.headers(headers),
            None => response_builder,
        };

        let response = response_builder
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        let status = response.status();
        if status.is_success() {
            Ok(response.json().await?)
        } else {
            Err(anyhow::anyhow!(format!(
                "HTTP agent request POST {} failed {}",
                url,
                response.status()
            )))
        }
    }

    pub async fn get<R: DeserializeOwned>(&self, url: &str) -> anyhow::Result<R> {
        let base_url = self.base_internal_url.clone();
        let response = self
            .client
            .get(format!("{}{}", base_url, url))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        let status = response.status();
        if status.is_success() {
            Ok(response.json().await?)
        } else {
            Err(anyhow::anyhow!(format!(
                "HTTP agent request GET {} failed {}",
                url,
                response.status()
            )))
        }
    }
}

#[derive(Clone)]
pub enum Connection {
    Sql(Pool<Postgres>),
    Http(HttpClient),
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Connection::Sql(_) => write!(f, "Sql"),
            Connection::Http(_) => write!(f, "Http"),
        }
    }
}

impl Connection {
    pub fn as_sql(&self) -> Option<&Pool<Postgres>> {
        match self {
            Connection::Sql(db) => Some(db),
            Connection::Http(_) => None,
        }
    }
}

impl From<Pool<Postgres>> for Connection {
    fn from(value: Pool<Postgres>) -> Self {
        Connection::Sql(value)
    }
}

impl From<&Pool<Postgres>> for Connection {
    fn from(value: &Pool<Postgres>) -> Self {
        Connection::Sql(value.clone())
    }
}

fn format_pull_query(peek: String) -> String {
    let r = format!(
        "WITH peek AS (
            {}
        ), q AS NOT MATERIALIZED (
            UPDATE v2_job_queue SET
                running = true,
                started_at = coalesce(started_at, now()),
                suspend_until = null,
                worker = $1
            WHERE id = (SELECT id FROM peek)
            RETURNING
                started_at, scheduled_for,
                canceled_by, canceled_reason, worker, cache_ignore_s3_path, runnable_settings_handle
        ), r AS NOT MATERIALIZED (
            UPDATE v2_job_runtime SET
                ping = now()
            WHERE id = (SELECT id FROM peek)
        ), j AS NOT MATERIALIZED (
            SELECT
                id, workspace_id, parent_job, created_by, created_at, runnable_id,
                runnable_path, args, kind, trigger, trigger_kind,
                permissioned_as, permissioned_as_email, script_lang,
                flow_innermost_root_job, root_job, flow_step_id,
                same_worker, pre_run_error, visible_to_owner, tag, concurrent_limit,
                concurrency_time_window_s, timeout, cache_ttl, priority, raw_code, raw_lock,
                raw_flow, script_entrypoint_override, preprocessed
            FROM v2_job
            WHERE id = (SELECT id FROM peek)
        ) SELECT j.id, j.workspace_id, j.parent_job, j.created_by, started_at, scheduled_for,
            j.runnable_id, j.runnable_path, j.args, canceled_by,
            canceled_reason, j.kind, j.trigger, j.trigger_kind, j.permissioned_as,
            flow_status, j.script_lang,
            j.same_worker, j.pre_run_error, j.visible_to_owner,
            j.tag, j.concurrent_limit, j.concurrency_time_window_s, j.flow_innermost_root_job, j.root_job,
            j.timeout, j.flow_step_id, j.cache_ttl, q.cache_ignore_s3_path, q.runnable_settings_handle, j.priority, j.raw_code, j.raw_lock, j.raw_flow,
            j.script_entrypoint_override, j.preprocessed, pj.runnable_path as parent_runnable_path,
            COALESCE(p.email, j.permissioned_as_email) as permissioned_as_email, p.username as permissioned_as_username, p.is_admin as permissioned_as_is_admin,
            p.is_operator as permissioned_as_is_operator, p.groups as permissioned_as_groups, p.folders as permissioned_as_folders, p.end_user_email as permissioned_as_end_user_email
        FROM q, j
            LEFT JOIN v2_job_status f USING (id)
            LEFT JOIN job_perms p ON p.job_id = j.id
            LEFT JOIN v2_job pj ON j.parent_job = pj.id
            ",
        peek
    );
    // tracing::debug!("pull query: {}", r);
    r
}

pub fn make_suspended_pull_query(tags: &[String]) -> String {
    format_pull_query(format!(
        "SELECT id
        FROM v2_job_queue
        WHERE suspend_until IS NOT NULL AND (suspend <= 0 OR suspend_until <= now()) AND tag IN ({})
        ORDER BY priority DESC NULLS LAST, created_at
        FOR UPDATE SKIP LOCKED
        LIMIT 1",
        tags.iter().map(|x| format!("'{x}'")).join(", ")
    ))
}
// pub async fn make_suspended
pub async fn store_suspended_pull_query(wc: &WorkerConfig) {
    if wc.worker_tags.len() == 0 {
        tracing::error!("Empty tags in worker tags, skipping");
        return;
    }
    let query = make_suspended_pull_query(&wc.worker_tags);
    let mut l = WORKER_SUSPENDED_PULL_QUERY.write().await;
    *l = query;
}

pub fn make_pull_query(tags: &[String]) -> String {
    let query = format_pull_query(format!(
        "SELECT id
        FROM v2_job_queue
        WHERE running = false AND tag IN ({}) AND scheduled_for <= now()
        ORDER BY priority DESC NULLS LAST, scheduled_for
        FOR UPDATE SKIP LOCKED
        LIMIT 1",
        tags.iter().map(|x| format!("'{x}'")).join(", ")
    ));
    query
}

pub async fn store_pull_query(wc: &WorkerConfig) {
    let mut queries = vec![];
    for tags in wc.priority_tags_sorted.iter() {
        if tags.tags.len() == 0 {
            tracing::error!("Empty tags in priority tags, skipping");
            continue;
        }
        let query = make_pull_query(&tags.tags);
        queries.push(query);
    }
    let mut l = WORKER_PULL_QUERIES.write().await;
    *l = queries;
}

pub const TMP_DIR: &str = "/tmp/windmill";
pub const TMP_LOGS_DIR: &str = concatcp!(TMP_DIR, "/logs");

pub const HUB_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "hub");
pub const HUB_RT_CACHE_DIR: &str = concatcp!(ROOT_CACHE_DIR, "hub_rt");

pub const ROOT_CACHE_DIR: &str = concatcp!(TMP_DIR, "/cache/");

pub fn write_file(dir: &str, path: &str, content: &str) -> error::Result<File> {
    let path = format!("{}/{}", dir, path);
    let mut file = File::create(&path).map_err(|e| {
        tracing::error!("Failed to create file at {path}: {:?}", &e);
        e
    })?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    Ok(file)
}

pub fn write_file_bytes(path: &str, content: &Bytes) -> error::Result<File> {
    let mut file = File::create(path)?;
    file.write_all(content)?;
    file.flush()?;
    Ok(file)
}
/// from : https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

pub fn is_allowed_file_location(job_dir: &str, user_defined_path: &str) -> error::Result<PathBuf> {
    let job_dir = Path::new(job_dir);
    let user_path = PathBuf::from(user_defined_path);

    let full_path = job_dir.join(&user_path);

    // let normalized_job_dir = std::fs::canonicalize(job_dir)?;
    // let normalized_full_path = std::fs::canonicalize(&full_path)?;
    let normalized_job_dir = normalize_path(job_dir);
    let normalized_full_path = normalize_path(&full_path);

    if !normalized_full_path.starts_with(&normalized_job_dir) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Path is outside the allowed job directory.",
        )
        .into());
    }

    Ok(normalized_full_path)
}

pub fn write_file_at_user_defined_location(
    job_dir: &str,
    user_defined_path: &str,
    content: &str,
    mode: Option<u32>,
) -> error::Result<PathBuf> {
    let normalized_full_path = is_allowed_file_location(job_dir, user_defined_path)?;

    let full_path = normalized_full_path.as_path();
    if let Some(parent_dir) = full_path.parent() {
        std::fs::create_dir_all(parent_dir)?;
    }

    let mut file = File::create(full_path)?;

    #[cfg(unix)]
    if let Some(mode) = mode {
        let perm = std::os::unix::fs::PermissionsExt::from_mode(mode);
        file.set_permissions(perm)
            .map_err(|e| anyhow!("Failed to set permissions to {}: {e}", user_defined_path))?;
    }

    #[cfg(windows)]
    if mode.is_some() {
        tracing::error!("Cannot use `mode` to set file permissions on windows workers");
    }

    file.write_all(content.as_bytes())?;
    file.flush()?;
    Ok(normalized_full_path)
}

pub async fn reload_custom_tags_setting(db: &DB) -> error::Result<()> {
    let q = sqlx::query!(
        "SELECT value FROM global_settings WHERE name = $1",
        CUSTOM_TAGS_SETTING
    )
    .fetch_optional(db)
    .await?;

    let tags = if let Some(q) = q {
        if let Ok(v) = serde_json::from_value::<Vec<String>>(q.value.clone()) {
            v
        } else {
            tracing::error!(
                "Could not parse custom tags setting as vec of strings, found: {:#?}",
                &q.value
            );
            vec![]
        }
    } else {
        CUSTOM_TAGS.clone()
    };

    let custom_tags = CustomTags::from(tags);

    tracing::info!(
        "Loaded setting custom_tags, common: {:?}, per-workspace: {:?}",
        custom_tags.global,
        custom_tags.specific,
    );

    {
        let mut l = CUSTOM_TAGS_PER_WORKSPACE.write().await;
        *l = custom_tags.clone()
    }
    {
        let mut l = ALL_TAGS.write().await;
        *l = [
            custom_tags.global.clone(),
            custom_tags
                .specific
                .keys()
                .map(|x| x.to_string())
                .collect_vec(),
        ]
        .concat();
    }
    Ok(())
}

#[cfg(not(windows))]
fn parse_file<T: FromStr>(path: &str) -> Option<T> {
    std::process::Command::new("cat")
        .args([path])
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .to_string()
                .trim()
                .parse::<T>()
                .ok()
        })
        .flatten()
}

#[annotations("#")]
pub struct RubyAnnotations {
    pub verbose: bool,
}

#[annotations("#")]
pub struct PythonAnnotations {
    pub no_cache: bool,
    pub no_postinstall: bool,
    pub py_select_latest: bool,
    pub skip_result_postprocessing: bool,
    pub py310: bool,
    pub py311: bool,
    pub py312: bool,
    pub py313: bool,
}

#[annotations("//")]
pub struct GoAnnotations {
    pub go1_22_compat: bool,
}

#[annotations("//")]
pub struct TypeScriptAnnotations {
    pub npm: bool,
    pub nodejs: bool,
    pub native: bool,
    pub nobundling: bool,
}

#[annotations("--")]
pub struct SqlAnnotations {
    pub return_last_result: bool, // deprecated, use result_collection instead
    pub result_collection: SqlResultCollectionStrategy,
    pub prepare: bool, // Used to prepare datatable queries without executing
}

#[annotations("#")]
pub struct BashAnnotations {
    pub docker: bool,
    pub sandbox: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SqlResultCollectionStrategy {
    LastStatementAllRows,
    LastStatementFirstRow,
    LastStatementAllRowsScalar,
    LastStatementFirstRowScalar,
    AllStatementsAllRows,
    AllStatementsFirstRow,
    AllStatementsAllRowsScalar,
    AllStatementsFirstRowScalar,
    Legacy,
}

impl SqlResultCollectionStrategy {
    pub fn parse(s: &str) -> Self {
        use SqlResultCollectionStrategy::*;
        match s {
            "last_statement_all_rows" => LastStatementAllRows,
            "last_statement_first_row" => LastStatementFirstRow,
            "last_statement_all_rows_scalar" => LastStatementAllRowsScalar,
            "last_statement_first_row_scalar" => LastStatementFirstRowScalar,
            "all_statements_all_rows" => AllStatementsAllRows,
            "all_statements_first_row" => AllStatementsFirstRow,
            "all_statements_all_rows_scalar" => AllStatementsAllRowsScalar,
            "all_statements_first_row_scalar" => AllStatementsFirstRowScalar,
            "legacy" => Legacy,
            _ => SqlResultCollectionStrategy::default(),
        }
    }

    pub fn collect_last_statement_only(&self, query_count: usize) -> bool {
        use SqlResultCollectionStrategy::*;
        match self {
            LastStatementAllRows
            | LastStatementFirstRow
            | LastStatementFirstRowScalar
            | LastStatementAllRowsScalar => true,
            Legacy => query_count == 1,
            _ => false,
        }
    }
    pub fn collect_first_row_only(&self) -> bool {
        use SqlResultCollectionStrategy::*;
        match self {
            LastStatementFirstRow
            | LastStatementFirstRowScalar
            | AllStatementsFirstRow
            | AllStatementsFirstRowScalar => true,
            _ => false,
        }
    }
    pub fn collect_scalar(&self) -> bool {
        use SqlResultCollectionStrategy::*;
        match self {
            LastStatementFirstRowScalar
            | AllStatementsFirstRowScalar
            | LastStatementAllRowsScalar
            | AllStatementsAllRowsScalar => true,
            _ => false,
        }
    }

    // This function transforms the shape (e.g Row[][] -> Row)
    // It is the responsibility of the executor to avoid fetching unnecessary statements/rows
    pub fn collect(
        &self,
        values: Vec<Vec<Box<serde_json::value::RawValue>>>,
    ) -> error::Result<Box<serde_json::value::RawValue>> {
        let null = || serde_json::value::RawValue::from_string("null".to_string()).unwrap();

        let values = if self.collect_last_statement_only(values.len()) {
            values.into_iter().rev().take(1).collect()
        } else {
            values
        };

        let values = if self.collect_first_row_only() {
            values
                .into_iter()
                .map(|rows| rows.into_iter().take(1).collect())
                .collect()
        } else {
            values
        };

        let values = if self.collect_scalar() {
            values
                .into_iter()
                .map(|rows| {
                    rows.into_iter()
                        .map(|row| {
                            // Take the first value in the object
                            let record =
                                match serde_json::from_str(row.get()) {
                                    Ok(serde_json::Value::Object(record)) => record,
                                    Ok(_) => return Err(error::Error::ExecutionErr(
                                        "Could not collect sql scalar value from non-object row"
                                            .to_string(),
                                    )),
                                    Err(e) => {
                                        return Err(error::Error::ExecutionErr(format!(
                                    "Could not collect sql scalar value (failed to parse row): {}",
                                    e
                                )))
                                    }
                                };
                            let Some((_, value)) = record.iter().next() else {
                                return Err(error::Error::ExecutionErr(
                                    "Could not collect sql scalar value from empty row".to_string(),
                                ));
                            };
                            Ok(serde_json::value::RawValue::from_string(
                                serde_json::to_string(value).map_err(to_anyhow)?,
                            )
                            .map_err(to_anyhow)?)
                        })
                        .collect::<error::Result<Vec<_>>>()
                })
                .collect::<error::Result<Vec<_>>>()?
        } else {
            values
        };

        match (
            self.collect_last_statement_only(values.len()),
            self.collect_first_row_only(),
        ) {
            (true, true) => {
                match values
                    .into_iter()
                    .last()
                    .map(|rows| rows.into_iter().next())
                {
                    Some(Some(row)) => Ok(row.clone()),
                    _ => Ok(null()),
                }
            }
            (true, false) => match values.into_iter().last() {
                Some(rows) => Ok(merge_raw_values_to_array(rows.as_slice())),
                None => Ok(null()),
            },
            (false, true) => {
                let values = values
                    .into_iter()
                    .map(|rows| rows.into_iter().next().unwrap_or_else(null))
                    .collect::<Vec<_>>();
                Ok(merge_raw_values_to_array(values.as_slice()))
            }
            (false, false) => Ok(merge_nested_raw_values_to_array(
                values.iter().map(|x| x.iter()),
            )),
        }
    }
}

impl Default for SqlResultCollectionStrategy {
    fn default() -> Self {
        SqlResultCollectionStrategy::LastStatementAllRows
    }
}

/// length = 5
/// value  = "foo"
/// output = "foo  "
///           12345
pub fn pad_string(value: &str, total_length: usize) -> String {
    if value.len() >= total_length {
        value.to_string() // Return the original string if it's already long enough
    } else {
        let padding_needed = total_length - value.len();
        format!("{value}{}", " ".repeat(padding_needed)) // Pad with spaces
    }
}
pub fn copy_dir_recursively(src: &Path, dst: &Path) -> error::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    tracing::debug!("Copying recursively from {:?} to {:?}", src, dst);

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() && !src_path.is_symlink() {
            copy_dir_recursively(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    tracing::debug!("Finished copying recursively from {:?} to {:?}", src, dst);

    Ok(())
}

pub async fn load_cache(bin_path: &str, _remote_path: &str, is_dir: bool) -> (bool, String) {
    if tokio::fs::metadata(&bin_path).await.is_ok() {
        (true, format!("loaded from local cache: {}\n", bin_path))
    } else {
        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if let Some(os) = crate::s3_helpers::get_object_store().await {
            let started = std::time::Instant::now();
            use crate::s3_helpers::attempt_fetch_bytes;

            if let Ok(mut x) = attempt_fetch_bytes(os, _remote_path).await {
                if is_dir {
                    if let Err(e) = extract_tar(x, bin_path).await {
                        tracing::error!("could not write tar archive locally: {e:?}");
                        return (
                            false,
                            "error writing tar archive from object store".to_string(),
                        );
                    }
                } else {
                    if let Err(e) = write_binary_file(bin_path, &mut x) {
                        tracing::error!("could not write bundle/bin file locally: {e:?}");
                        return (
                            false,
                            "error writing bundle/bin file from object store".to_string(),
                        );
                    }
                }
                tracing::info!("loaded from object store {}", bin_path);
                return (
                    true,
                    format!(
                        "loaded bin/bundle from object store {} in {}ms",
                        bin_path,
                        started.elapsed().as_millis()
                    ),
                );
            }
        }
        let _ = is_dir;
        (false, "".to_string())
    }
}

pub async fn exists_in_cache(bin_path: &str, _remote_path: &str) -> bool {
    if tokio::fs::metadata(&bin_path).await.is_ok() {
        return true;
    } else {
        #[cfg(all(feature = "enterprise", feature = "parquet"))]
        if let Some(os) = crate::s3_helpers::get_object_store().await {
            return os
                .get(&object_store::path::Path::from(_remote_path))
                .await
                .is_ok();
        }
        return false;
    }
}

pub async fn save_cache(
    local_cache_path: &str,
    _remote_cache_path: &str,
    origin: &str,
    is_dir: bool,
) -> crate::error::Result<String> {
    let mut _cached_to_s3 = false;
    #[cfg(all(feature = "enterprise", feature = "parquet"))]
    if let Some(os) = crate::s3_helpers::get_object_store().await {
        use object_store::path::Path;
        let file_to_cache = if is_dir {
            let tar_path = format!(
                "{ROOT_CACHE_DIR}/tar/{}_tar.tar",
                local_cache_path
                    .split("/")
                    .last()
                    .unwrap_or(&uuid::Uuid::new_v4().to_string())
            );
            let tar_file = std::fs::File::create(&tar_path)?;
            let mut tar = tar::Builder::new(tar_file);
            tar.append_dir_all(".", &origin)?;
            let tar_metadata = tokio::fs::metadata(&tar_path).await;
            if tar_metadata.is_err() || tar_metadata.as_ref().unwrap().len() == 0 {
                tracing::info!("Failed to tar cache: {origin}");
                return Err(error::Error::ExecutionErr(format!(
                    "Failed to tar cache: {origin}"
                )));
            }
            tar_path
        } else {
            origin.to_owned()
        };

        if let Err(e) = os
            .put(
                &Path::from(_remote_cache_path),
                std::fs::read(&file_to_cache)?.into(),
            )
            .await
        {
            tracing::error!(
                "Failed to put go bin to object store: {_remote_cache_path}. Error: {:?}",
                e
            );
        } else {
            _cached_to_s3 = true;
            if is_dir {
                tokio::fs::remove_dir_all(&file_to_cache).await?;
            }
        }
    }

    // if !*CLOUD_HOSTED {
    if true {
        if is_dir {
            copy_dir_recursively(&PathBuf::from(origin), &PathBuf::from(local_cache_path))?;
        } else {
            std::fs::copy(origin, local_cache_path)?;
        }
        Ok(format!(
            "\nwrote cached binary: {} (backed by EE distributed object store: {_cached_to_s3})\n",
            local_cache_path
        ))
    } else if _cached_to_s3 {
        Ok(format!(
            "wrote cached binary to object store {}\n",
            local_cache_path
        ))
    } else {
        Ok("".to_string())
    }
}

#[cfg(all(feature = "enterprise", feature = "parquet"))]
pub async fn extract_tar(tar: bytes::Bytes, folder: &str) -> error::Result<()> {
    use std::time::Instant;

    use bytes::Buf;
    use tokio::fs::{self};

    let start: Instant = Instant::now();
    fs::create_dir_all(&folder).await?;

    let mut ar = tar::Archive::new(tar.reader());

    if let Err(e) = ar.unpack(folder) {
        tracing::info!("Failed to untar to {folder}. Error: {:?}", e);
        fs::remove_dir_all(&folder).await?;
        return Err(error::Error::ExecutionErr(format!(
            "Failed to untar tar {folder}"
        )));
    }
    tracing::info!(
        "Finished extracting tar to {folder}. Took {}ms",
        start.elapsed().as_millis(),
    );
    Ok(())
}
#[cfg(all(feature = "enterprise", feature = "parquet"))]
fn write_binary_file(main_path: &str, byts: &mut bytes::Bytes) -> error::Result<()> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(main_path)?;
    file.write_all(byts)?;
    #[cfg(unix)]
    {
        use std::fs::Permissions;
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(Permissions::from_mode(0o755))?;
    }
    file.flush()?;
    Ok(())
}

#[cfg(not(windows))]
fn get_cgroupv2_path() -> Option<String> {
    let cgroup_path: String = parse_file("/proc/self/cgroup")?;

    CGROUP_V2_PATH_RE
        .captures(&cgroup_path)
        .map(|x| format!("/sys/fs/cgroup{}", x.get(1).unwrap().as_str()))
}

#[cfg(not(windows))]
pub fn get_vcpus() -> Option<i64> {
    if Path::new("/sys/fs/cgroup/cpu/cpu.cfs_quota_us").exists() {
        // cgroup v1
        parse_file("/sys/fs/cgroup/cpu/cpu.cfs_quota_us")
            .map(|x: i64| if x > 0 { Some(x) } else { None })
            .flatten()
    } else {
        // cgroup v2
        let cgroup_path = get_cgroupv2_path()?;

        let cpu_max_path = format!("{cgroup_path}/cpu.max");

        parse_file(&cpu_max_path)
            .map(|x: String| {
                CGROUP_V2_CPU_RE
                    .captures(&x)
                    .map(|x| x.get(1).unwrap().as_str().parse::<i64>().ok())
                    .flatten()
            })
            .flatten()
            .map(|x| if x > 0 { Some(x) } else { None })
            .flatten()
    }
}

#[cfg(windows)]
pub fn get_vcpus() -> Option<i64> {
    let mut sys = System::new();
    sys.refresh_cpu_all();
    (sys.cpus().len() * 100000).try_into().ok()
}

#[cfg(not(windows))]
fn get_memory_from_meminfo() -> Option<i64> {
    let memory_info = parse_file::<String>("/proc/meminfo")?;
    if memory_info.contains("MemTotal") {
        let memory_total = memory_info
            .split("MemTotal:")
            .nth(1)?
            .split("kB")
            .next()?
            .trim()
            .parse::<i64>()
            .ok()?;
        return Some(memory_total * 1024);
    }
    None
}

#[cfg(not(windows))]
pub fn get_memory() -> Option<i64> {
    let memory_limit: Option<i64> =
        if Path::new("/sys/fs/cgroup/memory/memory.limit_in_bytes").exists() {
            // cgroup v1
            parse_file("/sys/fs/cgroup/memory/memory.limit_in_bytes")
        } else {
            // cgroup v2
            let cgroup_path = get_cgroupv2_path()?;
            let memory_max_path = format!("{cgroup_path}/memory.max");

            parse_file(&memory_max_path)
        };

    // if memory_max is super high, read machine total memory (meminfo)
    if memory_limit
        .as_ref()
        .is_some_and(|x| x > &(1024 * 1024 * 1024 * 1024 * 1024))
    {
        get_memory_from_meminfo()
    } else {
        memory_limit
    }
}

#[cfg(windows)]
pub fn get_memory() -> Option<i64> {
    let mut sys = System::new();
    sys.refresh_memory();
    Some(sys.total_memory() as i64)
}

#[cfg(not(windows))]
pub fn get_worker_memory_usage() -> Option<i64> {
    if Path::new("/sys/fs/cgroup/memory/memory.usage_in_bytes").exists() {
        // cgroup v1
        let total_memory_usage: i64 = parse_file("/sys/fs/cgroup/memory/memory.usage_in_bytes")?;

        let inactive_file = parse_file("/sys/fs/cgroup/memory/memory.stat")
            .map(|x: String| {
                CGROUP_V1_INACTIVE_FILE_RE
                    .captures(&x)
                    .map(|x| x.get(1).unwrap().as_str().parse::<i64>().ok())
                    .flatten()
            })
            .flatten()?;

        Some(total_memory_usage - inactive_file)
    } else {
        // cgroup v2
        let cgroup_path = get_cgroupv2_path()?;
        let memory_current_path = format!("{cgroup_path}/memory.current");

        let total_memory_usage: i64 = parse_file(&memory_current_path)?;

        let memory_stat_path = format!("{cgroup_path}/memory.stat");

        let inactive_file = parse_file(&memory_stat_path)
            .map(|x: String| {
                CGROUP_V2_INACTIVE_FILE_RE
                    .captures(&x)
                    .map(|x| x.get(1).unwrap().as_str().parse::<i64>().ok())
                    .flatten()
            })
            .flatten()?;

        Some(total_memory_usage - inactive_file)
    }
}

#[cfg(windows)]
pub fn get_worker_memory_usage() -> Option<i64> {
    let mut sys = System::new();
    sys.refresh_memory();
    Some(sys.used_memory() as i64)
}

pub fn get_windmill_memory_usage() -> Option<i64> {
    #[cfg(all(not(target_env = "msvc"), feature = "jemalloc"))]
    {
        match tikv_jemalloc_ctl::epoch::advance() {
            Ok(_) => match tikv_jemalloc_ctl::stats::resident::read() {
                Ok(resident) => i64::try_from(resident).ok(),
                Err(e) => {
                    tracing::error!("jemalloc resident memory read failed: {:?}", e);
                    None
                }
            },
            Err(e) => {
                tracing::error!("jemalloc epoch advance failed: {:?}", e);
                None
            }
        }
    }

    #[cfg(any(target_env = "msvc", not(feature = "jemalloc")))]
    {
        None
    }
}

#[derive(Serialize, Deserialize)]
pub enum PingType {
    Initial,
    MainLoop,
    Job,
    InitScript,
}
#[derive(Serialize, Deserialize)]
pub struct Ping {
    pub last_job_executed: Option<Uuid>,
    pub last_job_workspace_id: Option<String>,
    pub worker_instance: Option<String>,
    pub ip: Option<String>,
    pub tags: Option<Vec<String>>,
    pub dw: Option<String>,
    pub dws: Option<Vec<String>>,
    pub version: Option<String>,
    pub vcpus: Option<i64>,
    pub memory: Option<i64>,
    pub memory_usage: Option<i64>,
    pub wm_memory_usage: Option<i64>,
    pub jobs_executed: Option<i32>,
    pub occupancy_rate: Option<f32>,
    pub occupancy_rate_15s: Option<f32>,
    pub occupancy_rate_5m: Option<f32>,
    pub occupancy_rate_30m: Option<f32>,
    pub job_isolation: Option<String>,
    pub native_mode: Option<bool>,
    pub ping_type: PingType,
}
pub async fn update_ping_http(
    insert_ping: Ping,
    worker_name: &str,
    worker_group: &str,
    db: &DB,
) -> anyhow::Result<()> {
    // tracing::info!("update ping: {}", insert_ping.tags.join(","));
    match insert_ping.ping_type {
        PingType::MainLoop => {
            update_worker_ping_main_loop_query(
                worker_name,
                insert_ping.tags.unwrap_or_default().as_slice(),
                insert_ping.vcpus,
                insert_ping.memory,
                insert_ping.jobs_executed,
                insert_ping.occupancy_rate,
                insert_ping.memory_usage,
                insert_ping.wm_memory_usage,
                insert_ping.occupancy_rate_15s,
                insert_ping.occupancy_rate_5m,
                insert_ping.occupancy_rate_30m,
                insert_ping.native_mode.unwrap_or(false),
                db,
            )
            .await?
        }
        PingType::Initial => {
            if insert_ping.worker_instance.is_none()
                || insert_ping.version.is_none()
                || insert_ping.ip.is_none()
            {
                return Err(anyhow::anyhow!(
                    "Worker instance, version and ip are required"
                ));
            }

            insert_ping_query(
                &insert_ping.worker_instance.unwrap(),
                &worker_name,
                worker_group,
                &insert_ping.ip.unwrap(),
                insert_ping.tags.unwrap_or_default().as_slice(),
                insert_ping.dw,
                insert_ping.dws.as_deref(),
                &insert_ping.version.unwrap(),
                insert_ping.vcpus,
                insert_ping.memory,
                insert_ping.job_isolation,
                insert_ping.native_mode.unwrap_or(false),
                db,
            )
            .await?;
        }
        PingType::Job => {
            update_worker_ping_from_job_query(
                &insert_ping.last_job_executed.unwrap_or_default(),
                &insert_ping.last_job_workspace_id.unwrap_or_default(),
                worker_name,
                insert_ping.memory_usage,
                insert_ping.wm_memory_usage,
                insert_ping.occupancy_rate,
                insert_ping.occupancy_rate_15s,
                insert_ping.occupancy_rate_5m,
                insert_ping.occupancy_rate_30m,
                insert_ping.job_isolation,
                db,
            )
            .await?;
        }
        PingType::InitScript => {
            update_ping_for_failed_init_script_query(
                worker_name,
                insert_ping.last_job_executed.unwrap_or_default(),
                db,
            )
            .await?
        }
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobCancelled {
    pub canceled_by: String,
    pub reason: String,
}

pub async fn set_job_cancelled_query(
    job_id: Uuid,
    db: &DB,
    canceled_by: &str,
    reason: &str,
) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE v2_job_queue
    SET canceled_by = $1
      , canceled_reason = $2
WHERE id = $3",
        canceled_by,
        reason,
        job_id
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn update_ping_for_failed_init_script_query(
    worker_name: &str,
    last_job_id: Uuid,
    db: &DB,
) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE worker_ping SET
ping_at = now(),
jobs_executed = 1,
current_job_id = $1,
current_job_workspace_id = 'admins'
WHERE worker = $2",
        last_job_id,
        worker_name
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn fetch_flow_node_query(
    db: &DB,
    id: i64,
    loc: &'static Location<'_>,
) -> error::Result<RawNode> {
    let r = sqlx::query!(
        "SELECT \
            code AS \"raw_code: String\", \
            lock AS \"raw_lock: String\", \
            flow AS \"raw_flow: Json<Box<RawValue>>\" \
        FROM flow_node WHERE id = $1 LIMIT 1",
        id,
    )
    .fetch_optional(db)
    .await
    .map_err(Into::into)
    .and_then(unwrap_or_error(loc, "Flow node", id))
    .map(|r| RawNode {
        raw_code: r.raw_code,
        raw_lock: r.raw_lock,
        raw_flow: r.raw_flow.map(|Json(raw_flow)| raw_flow),
    })?;
    Ok(r)
}

pub async fn fetch_raw_script_from_app_query(
    db: &DB,
    id: i64,
    loc: &'static Location<'_>,
) -> error::Result<RawScript> {
    sqlx::query!(
        "SELECT lock, code FROM app_script WHERE id = $1 LIMIT 1",
        id,
    )
    .fetch_optional(db)
    .await
    .map_err(Into::into)
    .and_then(unwrap_or_error(&loc, "Application script", id))
    .map(|r| RawScript { content: r.code, lock: r.lock, meta: None })
}

pub async fn insert_ping_query(
    worker_instance: &str,
    worker_name: &str,
    worker_group: &str,
    ip: &str,
    tags: &[String],
    dw: Option<String>,
    dws: Option<&[String]>,
    version: &str,
    vcpus: Option<i64>,
    memory: Option<i64>,
    job_isolation: Option<String>,
    native_mode: bool,
    db: &DB,
) -> anyhow::Result<()> {
    sqlx::query!(
        "INSERT INTO worker_ping (worker_instance, worker, ip, custom_tags, worker_group, dedicated_worker, dedicated_workers, wm_version, vcpus, memory, job_isolation, native_mode) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) ON CONFLICT (worker)
        DO UPDATE set ip = EXCLUDED.ip, custom_tags = EXCLUDED.custom_tags, worker_group = EXCLUDED.worker_group, dedicated_workers = EXCLUDED.dedicated_workers, native_mode = EXCLUDED.native_mode",
        worker_instance,
        worker_name,
        ip,
        tags,
        worker_group,
        dw,
        dws,
        version,
        vcpus,
        memory,
        job_isolation.as_deref(),
        native_mode,
        )
        .execute(db)
        .await?;
    Ok(())
}

pub async fn update_worker_ping_from_job_query(
    job_id: &Uuid,
    w_id: &str,
    worker_name: &str,
    memory_usage: Option<i64>,
    wm_memory_usage: Option<i64>,
    occupancy_rate: Option<f32>,
    occupancy_rate_15s: Option<f32>,
    occupancy_rate_5m: Option<f32>,
    occupancy_rate_30m: Option<f32>,
    job_isolation: Option<String>,
    db: &DB,
) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE worker_ping SET ping_at = now(), current_job_id = $1, current_job_workspace_id = $2, memory_usage = $3, wm_memory_usage = $4,
        occupancy_rate = $6, occupancy_rate_15s = $7, occupancy_rate_5m = $8, occupancy_rate_30m = $9, job_isolation = $10 WHERE worker = $5",
            job_id,
        w_id,
        memory_usage,
        wm_memory_usage,
        worker_name,
        occupancy_rate,
        occupancy_rate_15s,
        occupancy_rate_5m,
        occupancy_rate_30m,
        job_isolation,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn update_job_ping_query(
    job_id: &Uuid,
    db: &DB,
    mem_peak: Option<i32>,
) -> anyhow::Result<PingJobStatusResponse> {
    let ro = sqlx::query!(
        "UPDATE v2_job_runtime r SET
        memory_peak = $1,
        ping = now()
    FROM v2_job_queue q
    WHERE r.id = $2 AND q.id = r.id
    RETURNING canceled_by, canceled_reason",
        mem_peak,
        job_id
    )
    .map(|x| PingJobStatusResponse {
        canceled_by: x.canceled_by,
        canceled_reason: x.canceled_reason,
        already_completed: false,
    })
    .fetch_optional(db)
    .await;

    // TODO: add memory metrics to memory time series

    if let Ok(r) = ro {
        if let Some(i) = r {
            Ok(i)
        } else {
            Ok(PingJobStatusResponse {
                canceled_by: None,
                canceled_reason: None,
                already_completed: true,
            })
        }
    } else {
        Err(to_anyhow(ro.unwrap_err()))
    }
}

pub async fn update_worker_ping_main_loop_query(
    worker_name: &str,
    tags: &[String],
    vcpus: Option<i64>,
    memory: Option<i64>,
    jobs_executed: Option<i32>,
    occupancy_rate: Option<f32>,
    memory_usage: Option<i64>,
    wm_memory_usage: Option<i64>,
    occupancy_rate_15s: Option<f32>,
    occupancy_rate_5m: Option<f32>,
    occupancy_rate_30m: Option<f32>,
    native_mode: bool,
    db: &DB,
) -> anyhow::Result<()> {
    timeout(Duration::from_secs(10), sqlx::query!(
        "UPDATE worker_ping SET ping_at = now(), jobs_executed = $1, custom_tags = $2,
         occupancy_rate = $3, memory_usage = $4, wm_memory_usage = $5, vcpus = COALESCE($7, vcpus),
         memory = COALESCE($8, memory), occupancy_rate_15s = $9, occupancy_rate_5m = $10, occupancy_rate_30m = $11, native_mode = $12 WHERE worker = $6",
        jobs_executed,
        tags,
        occupancy_rate,
        memory_usage,
        wm_memory_usage,
        worker_name,
        vcpus,
        memory,
        occupancy_rate_15s,
        occupancy_rate_5m,
        occupancy_rate_30m,
        native_mode,
    )
        .execute(db))
    .await??;
    Ok(())
}

// "UPDATE worker_ping SET ping_at = now(), jobs_executed = $1, custom_tags = $2,
// occupancy_rate = $3, memory_usage = $4, wm_memory_usage = $5, vcpus = COALESCE($7, vcpus),
// memory = COALESCE($8, memory), occupancy_rate_15s = $9, occupancy_rate_5m = $10, occupancy_rate_30m = $11 WHERE worker = $6",

const MAX_TAG_LEN: usize = 50;
const HASH_SUFFIX_LEN: usize = 16;

pub fn dedicated_worker_tag(workspace_id: &str, path: &str) -> String {
    let full_tag = format!("{}:{}", workspace_id, path);
    if full_tag.len() <= MAX_TAG_LEN {
        return full_tag;
    }
    let hash = <sha2::Sha256 as sha2::Digest>::digest(full_tag.as_bytes());
    let hex_hash = hex::encode(hash);
    let prefix_len = MAX_TAG_LEN - 1 - HASH_SUFFIX_LEN;
    format!(
        "{}#{}",
        &full_tag[..prefix_len],
        &hex_hash[..HASH_SUFFIX_LEN]
    )
}

pub async fn load_worker_config(
    db: &DB,
    killpill_tx: KillpillSender,
) -> error::Result<WorkerConfig> {
    tracing::info!("Loading config from WORKER_GROUP: {}", *WORKER_GROUP);
    let mut config: WorkerConfigOpt = sqlx::query_scalar!(
        "SELECT config FROM config WHERE name = $1",
        format!("worker__{}", *WORKER_GROUP)
    )
    .fetch_optional(db)
    .await?
    .flatten()
    .map(|x| serde_json::from_value(x).ok())
    .flatten()
    .unwrap_or_default();

    if config.dedicated_worker.is_none() {
        let dw = std::env::var("DEDICATED_WORKER").ok();
        if dw.is_some() {
            tracing::info!(
                "DEDICATED_WORKER set from env variable: {}",
                dw.as_ref().unwrap()
            );
            config.dedicated_worker = dw;
        }
    } else {
        tracing::info!(
            "DEDICATED_WORKER set from config: {}",
            config.dedicated_worker.as_ref().unwrap()
        );
    }
    let dedicated_worker = config
        .dedicated_worker
        .map(|x| {
            let splitted = x.split(':').to_owned().collect_vec();
            if splitted.len() != 2 {
                killpill_tx.send();
                return Err(anyhow::anyhow!(
                    "Invalid dedicated_worker format. Got {x}, expects <workspace_id>:<path>"
                ));
            } else {
                let workspace = splitted[0];
                let script_path = splitted[1];
                Ok(WorkspacedPath {
                    workspace_id: workspace.to_string(),
                    path: script_path.to_string(),
                })
            }
        })
        .transpose()?;

    // Parse dedicated_workers (multiple dedicated workers)
    let dedicated_workers = config
        .dedicated_workers
        .map(|workers| {
            workers
                .into_iter()
                .map(|x| {
                    let splitted = x.split(':').to_owned().collect_vec();
                    if splitted.len() != 2 {
                        return Err(anyhow::anyhow!(
                            "Invalid dedicated_workers format. Got {x}, expects <workspace_id>:<path>"
                        ));
                    }
                    let workspace = splitted[0];
                    let script_path = splitted[1];
                    Ok(WorkspacedPath {
                        workspace_id: workspace.to_string(),
                        path: script_path.to_string(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    if *WORKER_GROUP == "default" && dedicated_worker.is_none() {
        let mut all_tags = config
            .worker_tags
            .unwrap_or_default()
            .into_iter()
            .chain(
                std::env::var("WORKER_TAGS")
                    .ok()
                    .map(|x| x.split(',').map(|x| x.to_string()).collect_vec())
                    .unwrap_or_default(),
            )
            .sorted()
            .collect_vec();
        all_tags.dedup();
        config.worker_tags = Some(all_tags);
    }

    if let Some(force_worker_tags) = std::env::var("FORCE_WORKER_TAGS")
        .ok()
        .filter(|x| !x.is_empty())
        .map(|x| x.split(',').map(|x| x.to_string()).collect::<Vec<String>>())
    {
        tracing::info!(
            "Detected FORCE_WORKER_TAGS, forcing worker tags to: {:#?}",
            force_worker_tags
        );
        config.worker_tags = Some(force_worker_tags);
    }

    // set worker_tags using default if none. If priority tags is set, compute the sorted priority tags as well
    let worker_tags = config
        .worker_tags
        .or_else(|| {
            // Check for multiple dedicated workers first
            if let Some(ref dws) = dedicated_workers.as_ref() {
                let mut dedi_tags: Vec<String> = dws
                    .iter()
                    .map(|dw| dedicated_worker_tag(&dw.workspace_id, &dw.path))
                    .collect();
                if std::env::var("ADD_FLOW_TAG").is_ok() {
                    dedi_tags.push("flow".to_string());
                }
                Some(dedi_tags)
            } else if let Some(ref dedicated_worker) = dedicated_worker.as_ref() {
                // Fallback to single dedicated worker for backward compatibility
                let mut dedi_tags = vec![dedicated_worker_tag(
                    &dedicated_worker.workspace_id,
                    &dedicated_worker.path,
                )];
                if std::env::var("ADD_FLOW_TAG").is_ok() {
                    dedi_tags.push("flow".to_string());
                }
                Some(dedi_tags)
            } else {
                std::env::var("WORKER_TAGS")
                    .ok()
                    .map(|x| x.split(',').map(|x| x.to_string()).collect())
            }
        })
        .unwrap_or_else(|| DEFAULT_TAGS.clone());

    let mut priority_tags_sorted: Vec<PriorityTags> = Vec::new();
    let priority_tags_map = config.priority_tags.unwrap_or_else(HashMap::new);
    if priority_tags_map.len() > 0 {
        let mut all_tags_set: HashSet<String> = HashSet::from_iter(worker_tags.clone());

        let mut tags_by_priority: HashMap<u8, Vec<String>> = HashMap::new();

        for (tag, priority) in priority_tags_map.iter() {
            if *priority == 0 {
                // ignore tags with no priority as they will be added at the end from the `all_tags` set
                continue;
            }
            match tags_by_priority.get_mut(priority) {
                Some(tags) => {
                    tags.push(tag.clone());
                }
                None => {
                    let mut t: Vec<String> = Vec::new();
                    t.push(tag.clone());
                    tags_by_priority.insert(*priority, t);
                }
            };
            all_tags_set.remove(tag);
        }
        priority_tags_sorted = tags_by_priority
            .iter()
            .map(|(priority, tags)| PriorityTags { priority: priority.clone(), tags: tags.clone() })
            .collect();
        priority_tags_sorted.push(PriorityTags { priority: 0, tags: Vec::from_iter(all_tags_set) }); // push the tags that were not listed as high priority with a priority = 0
        priority_tags_sorted.sort_by_key(|elt| Reverse(elt.priority)); // sort by priority DESC
    } else {
        // if no priority is used, push all tags with a priority to 0
        priority_tags_sorted.push(PriorityTags { priority: 0, tags: worker_tags.clone() });
    }
    tracing::debug!("Custom tags priority set: {:?}", priority_tags_sorted);

    let env_vars_static = config.env_vars_static.unwrap_or_default().clone();
    let resolved_env_vars: HashMap<String, String> = load_env_vars(
        config
            .env_vars_allowlist
            .unwrap_or_default()
            .into_iter()
            .chain(load_whitelist_env_vars_from_env())
            .chain(env_vars_static.keys().map(|x| x.to_string())),
        &env_vars_static,
    );

    let periodic_script_bash = config
        .periodic_script_bash
        .or_else(|| load_periodic_bash_script_from_env())
        .and_then(|x| if x.is_empty() { None } else { Some(x) });
    let periodic_script_interval_seconds = config
        .periodic_script_interval_seconds
        .or_else(|| load_periodic_bash_script_interval_from_env());

    if periodic_script_bash.is_some() {
        if let Some(interval) = periodic_script_interval_seconds {
            if interval < MIN_PERIODIC_SCRIPT_INTERVAL_SECONDS {
                killpill_tx.send();
                return Err(anyhow::anyhow!(
                    "Periodic script interval must be at least {} seconds, got {} seconds",
                    MIN_PERIODIC_SCRIPT_INTERVAL_SECONDS,
                    interval
                )
                .into());
            }
        } else {
            killpill_tx.send();
            return Err(anyhow::anyhow!(
                "Periodic script interval must be specified when periodic script is configured"
            )
            .into());
        }
    }

    let native_mode = is_native_mode_from_env() || config.native_mode.unwrap_or(false);
    NATIVE_MODE_RESOLVED.store(native_mode, std::sync::atomic::Ordering::Relaxed);

    Ok(WorkerConfig {
        worker_tags,
        priority_tags_sorted,
        dedicated_worker,
        dedicated_workers,
        init_bash: config
            .init_bash
            .or_else(|| load_init_bash_from_env())
            .and_then(|x| if x.is_empty() { None } else { Some(x) }),
        periodic_script_bash,
        periodic_script_interval_seconds,
        cache_clear: config.cache_clear,
        pip_local_dependencies: config
            .pip_local_dependencies
            .or_else(|| load_pip_local_dependencies_from_env()),
        additional_python_paths: config
            .additional_python_paths
            .or_else(|| load_additional_python_paths_from_env()),
        env_vars: resolved_env_vars,
        native_mode,
    })
}

pub fn load_init_bash_from_env() -> Option<String> {
    std::env::var("INIT_SCRIPT")
        .ok()
        .and_then(|x| if x.is_empty() { None } else { Some(x) })
}

pub fn load_periodic_bash_script_from_env() -> Option<String> {
    std::env::var("PERIODIC_BASH_SCRIPT")
        .ok()
        .and_then(|x| if x.is_empty() { None } else { Some(x) })
}

pub fn load_periodic_bash_script_interval_from_env() -> Option<u64> {
    std::env::var("PERIODIC_BASH_SCRIPT_INTERVAL_SECONDS")
        .ok()
        .and_then(|x| x.parse::<u64>().ok())
}

pub fn load_pip_local_dependencies_from_env() -> Option<Vec<String>> {
    std::env::var("PIP_LOCAL_DEPENDENCIES")
        .ok()
        .map(|x| x.split(',').map(|x| x.to_string()).collect_vec())
}

pub fn load_additional_python_paths_from_env() -> Option<Vec<String>> {
    std::env::var("ADDITIONAL_PYTHON_PATHS")
        .ok()
        .map(|x| x.split(':').map(|x| x.to_string()).collect_vec())
}

pub fn load_whitelist_env_vars_from_env() -> std::vec::IntoIter<String> {
    std::env::var("WHITELIST_ENVS")
        .ok()
        .map(|x| x.split(',').map(|x| x.to_string()).collect_vec())
        .unwrap_or_default()
        .into_iter()
}

pub fn load_env_vars(
    iter: impl Iterator<Item = String>,
    env_vars_static: &HashMap<String, String>,
) -> HashMap<String, String> {
    iter.sorted()
        .unique()
        .map(|envvar_name| {
            (
                envvar_name.clone(),
                env_vars_static
                    .get::<String>(&envvar_name)
                    .map(|v| v.to_owned())
                    .unwrap_or_else(|| {
                        std::env::var(envvar_name.clone()).unwrap_or("".to_string())
                    }),
            )
        })
        .collect()
}

pub fn error_to_value(err: &error::Error) -> serde_json::Value {
    match err {
        error::Error::JsonErr(err) => err.clone(),
        _ => json!({"message": err.to_string(), "name": err.name()}),
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct WorkspacedPath {
    pub workspace_id: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkerConfigOpt {
    pub worker_tags: Option<Vec<String>>,
    pub priority_tags: Option<HashMap<String, u8>>,
    pub dedicated_worker: Option<String>,
    pub dedicated_workers: Option<Vec<String>>,
    pub init_bash: Option<String>,
    pub periodic_script_bash: Option<String>,
    pub periodic_script_interval_seconds: Option<u64>,
    pub cache_clear: Option<u32>,
    pub additional_python_paths: Option<Vec<String>>,
    pub pip_local_dependencies: Option<Vec<String>>,
    pub env_vars_static: Option<HashMap<String, String>>,
    pub env_vars_allowlist: Option<Vec<String>>,
    pub native_mode: Option<bool>,
}

impl Default for WorkerConfigOpt {
    fn default() -> Self {
        Self {
            worker_tags: Default::default(),
            priority_tags: Default::default(),
            dedicated_worker: Default::default(),
            dedicated_workers: Default::default(),
            init_bash: Default::default(),
            periodic_script_bash: Default::default(),
            periodic_script_interval_seconds: Default::default(),
            cache_clear: Default::default(),
            additional_python_paths: Default::default(),
            pip_local_dependencies: Default::default(),
            env_vars_static: Default::default(),
            env_vars_allowlist: Default::default(),
            native_mode: Default::default(),
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct WorkerConfig {
    pub worker_tags: Vec<String>,
    pub priority_tags_sorted: Vec<PriorityTags>,
    pub dedicated_worker: Option<WorkspacedPath>,
    pub dedicated_workers: Option<Vec<WorkspacedPath>>,
    pub init_bash: Option<String>,
    pub periodic_script_bash: Option<String>,
    pub periodic_script_interval_seconds: Option<u64>,
    pub cache_clear: Option<u32>,
    pub additional_python_paths: Option<Vec<String>>,
    pub pip_local_dependencies: Option<Vec<String>>,
    pub env_vars: HashMap<String, String>,
    pub native_mode: bool,
}

impl std::fmt::Debug for WorkerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WorkerConfig {{ worker_tags: {:?}, priority_tags_sorted: {:?}, dedicated_worker: {:?}, dedicated_workers: {:?}, init_bash: {:?}, periodic_script_bash: {:?}, periodic_script_interval_seconds: {:?}, cache_clear: {:?}, additional_python_paths: {:?}, pip_local_dependencies: {:?}, env_vars: {:?}, native_mode: {:?} }}",
        self.worker_tags, self.priority_tags_sorted, self.dedicated_worker, self.dedicated_workers, self.init_bash, self.periodic_script_bash, self.periodic_script_interval_seconds, self.cache_clear, self.additional_python_paths, self.pip_local_dependencies, self.env_vars.iter().map(|(k, v)| format!("{}: {}{} ({} chars)", k, &v[..3.min(v.len())], "***", v.len())).collect::<Vec<String>>().join(", "), self.native_mode)
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct PriorityTags {
    pub priority: u8,
    pub tags: Vec<String>,
}

pub fn to_raw_value<T: Serialize>(result: &T) -> Box<RawValue> {
    serde_json::value::to_raw_value(result)
        .unwrap_or_else(|_| RawValue::from_string("{}".to_string()).unwrap())
}

pub fn to_raw_value_owned(result: serde_json::Value) -> Box<RawValue> {
    serde_json::value::to_raw_value(&result)
        .unwrap_or_else(|_| RawValue::from_string("{}".to_string()).unwrap())
}

pub fn split_python_requirements<T: AsRef<str>>(requirements: T) -> Vec<String> {
    requirements
        .as_ref()
        .lines()
        .filter(|x| !x.trim_start().starts_with("--") && !x.trim().is_empty())
        .map(String::from)
        .collect()
}

#[derive(Eq, PartialEq, Clone, Copy, Default, Debug)]
#[repr(u32)]
pub enum PyVAlias {
    Py310 = 10,
    Py311,
    #[default]
    Py312,
    Py313,
}

impl Into<pep440_rs::Version> for PyVAlias {
    fn into(self) -> pep440_rs::Version {
        pep440_rs::Version::new([self.major() as u64, self as u64])
    }
}

impl Into<u32> for PyVAlias {
    fn into(self) -> u32 {
        self.major() * 100 + self as u32
    }
}

impl PyVAlias {
    pub fn all<T: From<PyVAlias>>() -> Vec<T> {
        use PyVAlias::*;
        vec![Py310.into(), Py311.into(), Py312.into(), Py313.into()]
    }
    // Get MAJOR part of alias. (semver: MAJOR.MINOR.PATCH)
    fn major(&self) -> u32 {
        use PyVAlias::*;
        match self {
            Py310 | Py311 | Py312 | Py313 => 3,
            // Py400 | Py401 => 4
        }
    }

    /// Converts numeric format to alias
    /// Example:
    /// 310u32 (in) -> PyVAlias::Py310 (out)
    pub fn try_from_v1<T: ToString>(numeric: T) -> Option<Self> {
        use PyVAlias::*;
        match numeric.to_string().as_str() {
            "310" => Some(Py310),
            "311" => Some(Py311),
            "312" => Some(Py312),
            "313" => Some(Py313),
            _ => None,
        }
    }
}

/// Parse lockfile for assigned python version.
/// If not found returns None
pub fn try_parse_locked_python_version_from_requirements<S: AsRef<str>>(
    requirements_lines: &[S],
) -> Option<pep440_rs::Version> {
    let parse_version = |s: &str| -> Option<pep440_rs::Version> {
        // Possible inputs:
        // V2:
        // # py: 3.11.0 or #py:3.11.0 or #py: 3.11.0
        //
        // V1:
        // # py311 or #py311
        let version_unparsed = s
            .to_owned()
            // Remove whitespaces. That leaves us with:
            // V2: #py:3.11.0
            // V1: #py311
            //
            // Remove #
            // V2: py:3.11.0
            // V1: py311
            //
            // Remove :
            // V2: py3.11.0
            // V1: py311
            .replace([' ', '#', ':'], "")
            // Remove "py"
            // V2: 3.11.0
            // V1: 311
            .replace("py", "");

        // We will support reading V1 syntax, but it will be overwritten next deploy
        PyVAlias::try_from_v1(&version_unparsed)
            .map(PyVAlias::into)
            .or(pep440_rs::Version::from_str(&version_unparsed)
                .ok()
                .map(pep440_rs::Version::into))
    };

    requirements_lines
        .iter()
        .find(|s| {
            let s = s.as_ref();
            s.starts_with("#py") || s.starts_with("# py")
        })
        .map(S::as_ref)
        .and_then(parse_version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_mixed_tags() {
        let input = vec![
            "global".to_string(),
            "feat(ws1+ws2)".to_string(),
            "hotfix(^ws3^ws4)".to_string(),
        ];
        let result = CustomTags::from(input);

        assert_eq!(result.global, vec!["global"]);

        let mut expected = HashMap::new();
        expected.insert(
            "feat".to_string(),
            SpecificTagData {
                tag_type: SpecificTagType::NoneExcept,
                workspaces: vec!["ws1".to_string(), "ws2".to_string()],
            },
        );
        expected.insert(
            "hotfix".to_string(),
            SpecificTagData {
                tag_type: SpecificTagType::AllExcluding,
                workspaces: vec!["ws3".to_string(), "ws4".to_string()],
            },
        );

        assert_eq!(result.specific, expected);
    }

    #[test]
    fn test_invalid_specific_tag_format() {
        let input = vec!["invalid(custom+format".to_string()];
        let result = CustomTags::from(input);

        // Regex does not match, so it's treated as a global tag.
        assert_eq!(result.global, vec!["invalid(custom+format"]);
        assert!(result.specific.is_empty());
    }

    #[test]
    fn test_empty_input() {
        let input: Vec<String> = vec![];
        let result = CustomTags::from(input);
        assert!(result.global.is_empty());
        assert!(result.specific.is_empty());
    }

    #[test]
    fn test_custom_tags_from_parses_global_tags_correctly() {
        let input = vec!["frontend".to_string(), "urgent".to_string()];
        let tags = CustomTags::from(input.clone());
        assert_eq!(tags.global, input);
        assert!(tags.specific.is_empty());
    }

    #[test]
    fn test_custom_tags_from_parses_specific_tags_none_except() {
        let input = vec!["urgent(ws1+ws2)".to_string()];
        let tags = CustomTags::from(input.clone());

        assert!(tags.global.is_empty());
        assert_eq!(tags.specific.len(), 1);

        let data = tags.specific.get("urgent").unwrap();
        assert_eq!(data.tag_type, SpecificTagType::NoneExcept);
        assert_eq!(data.workspaces, vec!["ws1", "ws2"]);
    }

    #[test]
    fn test_custom_tags_from_parses_specific_tags_all_excluding() {
        let input = vec!["legacy(^ws1^ws2)".to_string()];
        let tags = CustomTags::from(input.clone());

        assert!(tags.global.is_empty());
        assert_eq!(tags.specific.len(), 1);

        let data = tags.specific.get("legacy").unwrap();
        assert_eq!(data.tag_type, SpecificTagType::AllExcluding);
        assert_eq!(data.workspaces, vec!["ws1", "ws2"]);
    }

    #[test]
    fn test_custom_tags_from_ignores_empty_workspace_list_as_global() {
        let input = vec!["foo()".to_string(), "bar(^)".to_string()];
        let tags = CustomTags::from(input.clone());

        assert_eq!(tags.global, input);
        assert!(tags.specific.is_empty());
    }

    #[test]
    fn test_custom_tags_from_filters_specific_tags_by_workspace_none_except() {
        let input = vec!["urgent(ws1+ws2)".to_string()];
        let tags = CustomTags::from(input);

        let output = tags.to_string_vec(Some("ws1".to_string()));
        assert_eq!(output, vec!["urgent"]);

        let output_none = tags.to_string_vec(Some("ws3".to_string()));
        assert!(output_none.is_empty());
    }

    #[test]
    fn test_custom_tags_from_filters_specific_tags_by_workspace_all_excluding() {
        let input = vec!["legacy(^ws1^ws2)".to_string()];
        let tags = CustomTags::from(input);

        let output = tags.to_string_vec(Some("ws3".to_string()));
        assert_eq!(output, vec!["legacy"]);

        let output_excluded = tags.to_string_vec(Some("ws1".to_string()));
        assert!(output_excluded.is_empty());
    }

    #[test]
    fn test_custom_tags_from_reconstructs_all_tags_when_no_filter() {
        let input = vec![
            "foo".to_string(),
            "urgent(ws1+ws2)".to_string(),
            "legacy(^ws1^ws2)".to_string(),
        ];
        let tags = CustomTags::from(input);

        let mut result = tags.to_string_vec(None);
        result.sort();
        assert_eq!(result, vec!["foo", "legacy(^ws1^ws2)", "urgent(ws1+ws2)"]);
    }

    #[test]
    fn test_dedicated_worker_tag_short() {
        let tag = dedicated_worker_tag("demo", "u/alice/script");
        assert_eq!(tag, "demo:u/alice/script");
        assert!(tag.len() <= MAX_TAG_LEN);
    }

    #[test]
    fn test_dedicated_worker_tag_exactly_50() {
        // 50 chars exactly should not be hashed
        let workspace = "ws";
        let path = "a".repeat(50 - workspace.len() - 1); // -1 for ':'
        let tag = dedicated_worker_tag(workspace, &path);
        assert_eq!(tag.len(), 50);
        assert!(!tag.contains('#'));
    }

    #[test]
    fn test_dedicated_worker_tag_long_is_hashed() {
        let tag = dedicated_worker_tag(
            "my_workspace",
            "u/engineering/team/automation/critical_workflow_script_v2",
        );
        assert_eq!(tag.len(), MAX_TAG_LEN);
        assert_eq!(tag, "my_workspace:u/engineering/team/a#5bc26db79926d4f0");
    }

    #[test]
    fn test_dedicated_worker_tag_deterministic() {
        let a = dedicated_worker_tag(
            "ws",
            "some/very/long/path/that/exceeds/the/fifty/char/limit/easily",
        );
        assert_eq!(a, "ws:some/very/long/path/that/excee#bbb038d4268a0b41");
        let b = dedicated_worker_tag(
            "ws",
            "some/very/long/path/that/exceeds/the/fifty/char/limit/easily",
        );
        assert_eq!(a, b);
    }

    #[test]
    fn test_dedicated_worker_tag_different_paths_differ() {
        let a = dedicated_worker_tag(
            "ws",
            "some/very/long/path/that/exceeds/the/fifty/char/limit/easily_a",
        );
        let b = dedicated_worker_tag(
            "ws",
            "some/very/long/path/that/exceeds/the/fifty/char/limit/easily_b",
        );
        assert_ne!(a, b);
    }
}
