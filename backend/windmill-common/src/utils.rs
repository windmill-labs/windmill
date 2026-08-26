/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::auth::is_devops_email;
use crate::ee_oss::LICENSE_KEY_ID;
#[cfg(feature = "enterprise")]
use crate::ee_oss::{send_critical_alert, CriticalAlertKind};
use crate::error::{to_anyhow, Error, Result};
use crate::global_settings::UNIQUE_ID_SETTING;
use crate::worker::{EXIT_AFTER_N_JOBS, WORKER_SUFFIX};
use crate::DB;
use anyhow::Context;
use gethostname::gethostname;
use git_version::git_version;

use chrono::Utc;
use croner::Cron;
use itertools::Itertools;
use rand::{distr::Alphanumeric, rng, Rng};
use reqwest::Client;
use semver::Version;
use serde::{de::Error as SerdeDeserializerError, Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Postgres};
use std::borrow::Cow;
use std::fmt::Display;
use std::{fs::DirBuilder as SyncDirBuilder, str::FromStr};
use tokio::fs::DirBuilder as AsyncDirBuilder;
use url::Url;

pub const MAX_PER_PAGE: usize = 10000;
pub const DEFAULT_PER_PAGE: usize = 1000;

pub const GIT_VERSION: &str =
    git_version!(args = ["--tag", "--always"], fallback = "unknown-version");

pub const AGENT_JWT_PREFIX: &str = "jwt_agent_";
pub const WORKER_NAME_PREFIX: &str = "wk";
pub const AGENT_WORKER_NAME_PREFIX: &str = "ag";

use crate::CRITICAL_ALERT_MUTE_UI_ENABLED;
use std::panic::{self, AssertUnwindSafe, Location};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::worker::CLOUD_HOSTED;

lazy_static::lazy_static! {
    pub static ref COOKIE_DOMAIN: Option<String> = std::env::var("COOKIE_DOMAIN").ok();
    pub static ref IS_SECURE: AtomicBool = AtomicBool::new(false);

    pub static ref FORCE_IPV4: bool = std::env::var("FORCE_IPV4")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);

    pub static ref HTTP_CLIENT: Client = {
        let mut builder = reqwest::ClientBuilder::new()
            .user_agent("windmill/beta")
            .timeout(std::time::Duration::from_secs(20))
            .connect_timeout(std::time::Duration::from_secs(10));

        if *FORCE_IPV4 {
            tracing::info!("FORCE_IPV4 is enabled - HTTP client will only use IPv4");
            builder = builder.local_address(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)));
        }

        builder.build().unwrap()
    };
    /// HTTP client for streaming uploads (no total request timeout, only connect timeout).
    /// Used for S3 file uploads where the body is streamed and total time depends on data size.
    pub static ref HTTP_CLIENT_STREAMING: Client = {
        let mut builder = reqwest::ClientBuilder::new()
            .user_agent("windmill/beta")
            .connect_timeout(std::time::Duration::from_secs(10));

        if *FORCE_IPV4 {
            builder = builder.local_address(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)));
        }

        builder.build().unwrap()
    };
    pub static ref HTTP_CLIENT_PERMISSIVE: Client = configure_client(reqwest::ClientBuilder::new()
        .user_agent("windmill/beta")
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .danger_accept_invalid_certs(std::env::var("ACCEPT_INVALID_CERTS").is_ok()))
        .build().unwrap();
    pub static ref GIT_SEM_VERSION: Version = Version::parse(
        if GIT_VERSION.starts_with('v') {
            &GIT_VERSION[1..]
        } else {
            GIT_VERSION
        }
    ).unwrap_or(Version::new(0, 1, 0));

    pub static ref HOSTNAME :String = std::env::var("FORCE_HOSTNAME").unwrap_or_else(|_| {
        gethostname()
            .to_str()
            .map(|x| x.to_string())
            .unwrap_or_else(|| rd_string(5))
    });

    pub static ref MODE_AND_ADDONS: ModeAndAddons = {
        let mut search_addon = false;
        let mode = std::env::var("MODE")
        .map(|x| x.to_lowercase())
        .map(|x| {
            if &x == "server" {
                println!("Binary is in 'server' mode");
                Mode::Server
            } else if &x == "worker" {
                tracing::info!("Binary is in 'worker' mode");
                #[cfg(windows)]
                {
                    println!("It is highly recommended to use the agent mode instead on windows (MODE=agent) and to pass a BASE_INTERNAL_URL");
                }
                Mode::Worker
            } else if &x == "agent" {
                println!("Binary is in 'agent' mode with BASE_INTERNAL_URL={}", std::env::var("BASE_INTERNAL_URL").unwrap_or_default());
                if std::env::var("BASE_INTERNAL_URL").is_err() {
                    panic!("BASE_INTERNAL_URL is required in agent mode")
                }
                if std::env::var("AGENT_TOKEN").is_err() {
                    println!("AGENT_TOKEN is not passed. This is required for the agent to work and contains the JWT to authenticate with the server.")
                }

                #[cfg(not(feature = "enterprise"))]
                {
                    panic!("Agent mode is only available in the EE, ignoring...");
                }
                #[cfg(feature = "enterprise")]
                Mode::Agent
            } else if &x == "indexer" {
                tracing::info!("Binary is in 'indexer' mode");
                #[cfg(not(feature = "tantivy"))]
                {
                    eprintln!("Cannot start the indexer because tantivy is not included in this binary/image. Make sure you are using the EE image if you want to access the full text search features.");
                    panic!("Indexer mode requires compiling with the tantivy feature flag.");
                }
                #[cfg(feature = "tantivy")]
                Mode::Indexer
            } else if &x == "standalone+search"{
                search_addon = true;
                    println!("Binary is in 'standalone' mode with search enabled");
                    Mode::Standalone
            } else if &x == "mcp" {
                println!("Binary is in 'mcp' mode");
                Mode::MCP
            } else {
                if &x != "standalone" {
                    eprintln!("mode not recognized, defaulting to standalone: {x}");
                } else {
                    println!("Binary is in 'standalone' mode");
                }
                Mode::Standalone
            }
        })
        .unwrap_or_else(|_| {
            tracing::info!("Mode not specified, defaulting to standalone");
            Mode::Standalone
        });
        #[cfg(feature = "benchmark")]
        let mode = {
            if mode != Mode::Worker {
                println!("Benchmark mode: forcing MODE=worker");
            }
            Mode::Worker
        };
        ModeAndAddons {
            indexer: search_addon,
            mode,
        }
    };

    pub static ref HUB_API_SECRET: arc_swap::ArcSwap<Option<String>> = arc_swap::ArcSwap::from_pointee(None);
}

#[derive(Clone)]
pub struct ModeAndAddons {
    pub indexer: bool,
    pub mode: Mode,
}

#[derive(Deserialize, Clone)]
pub struct Pagination {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

#[derive(Deserialize)]
pub struct WithStarredInfoQuery {
    pub with_starred_info: Option<bool>,
}

#[derive(Deserialize)]
pub struct BulkDeleteRequest {
    pub paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StripPath(pub String);

impl StripPath {
    pub fn to_path(&self) -> &str {
        if self.0.starts_with('/') {
            self.0.strip_prefix('/').unwrap()
        } else {
            &self.0
        }
    }
}

/// Escape ILIKE special characters (`%`, `_`, `\`) so user input is matched
/// literally. Use this when building `ILIKE '%…%'` patterns from user-supplied
/// strings to prevent wildcard injection.
pub fn escape_ilike_pattern(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

lazy_static::lazy_static! {
    /// `[\p{Alphabetic}\p{Nd}_-]`, not `[\w-]`: Postgres' `\w` is `alnum` plus
    /// underscore, while Rust's also covers combining marks, connector
    /// punctuation and join controls. Spelling it out keeps these in step with
    /// the CHECK constraints below, which are the real authority — a character
    /// accepted here and rejected there puts the raw constraint violation back
    /// on the wire, which is what this guard exists to prevent.
    static ref PROPER_CHAR: &'static str = r"\p{Alphabetic}\p{Nd}_-";

    /// Mirrors the `proper_id` CHECK shared by `script`, `flow`, `variable`,
    /// `resource` and `schedule`.
    static ref PROPER_PATH_RE: regex::Regex =
        regex::Regex::new(&format!(r"^[ufg](/[{}]+){{2,}}$", *PROPER_CHAR)).unwrap();
    /// Mirrors the `proper_name` CHECK on `resource_type.name`, which
    /// `resource.resource_type` references without a foreign key of its own.
    static ref PROPER_TYPE_NAME_RE: regex::Regex =
        regex::Regex::new(&format!(r"^[{}]{{1,50}}$", *PROPER_CHAR)).unwrap();
}

/// Reject a path the `proper_id` constraint would reject anyway, so the caller
/// gets a plain 400 instead of the raw Postgres constraint-violation string,
/// which names the table and constraint and echoes the input back.
pub fn check_proper_path(path: &str) -> Result<()> {
    // The column is varchar(255); without this an over-long but well-formed path
    // still reaches Postgres and leaks the same kind of message back.
    if path.chars().count() > 255 {
        return Err(Error::BadRequest(
            "Invalid path: it must be at most 255 characters".to_string(),
        ));
    }
    if !PROPER_PATH_RE.is_match(path) {
        return Err(Error::BadRequest(
            "Invalid path: it must be of the form u/<user>/<name>, f/<folder>/<name> or \
             g/<group>/<name>, where every segment contains only alphanumeric characters, \
             '_' or '-'"
                .to_string(),
        ));
    }
    Ok(())
}

/// Replace a Postgres rejection with a message that says what the caller did
/// wrong and nothing about the schema. The raw error names the table and the
/// constraint and echoes the input back.
///
/// This, not `check_proper_path`, is what makes the leak unreachable: Postgres
/// classifies `\w` by the database's `LC_CTYPE`, so no fixed Rust charset can
/// mirror `proper_id` across deployments (`u/usér/nom` is valid under a UTF-8
/// locale and rejected under `C`). The pre-checks exist to give the common cases
/// a precise message; this catches whatever they let through.
pub fn sanitize_db_error(e: sqlx::Error) -> Error {
    let Some(db_err) = e.as_database_error() else {
        return Error::from(e);
    };
    match db_err.code().as_deref() {
        // check_violation — `proper_id` / `proper_name` and friends
        Some("23514") => Error::BadRequest(
            "Invalid path or name: it does not match the required format".to_string(),
        ),
        // string_data_right_truncation
        Some("22001") => Error::BadRequest("A field exceeds its maximum length".to_string()),
        // insufficient_privilege — row-level security rejected the row
        Some("42501") => {
            Error::NotAuthorized("You don't have write permission at this path".to_string())
        }
        _ => Error::from(e),
    }
}

/// Confine a resource type name to the charset `resource_type.name` already
/// enforces. `resource.resource_type` has no such constraint of its own, so
/// without this any string up to 50 chars can be stored and later rendered as
/// the type of a resource everyone in the workspace sees.
pub fn check_proper_type_name(name: &str) -> Result<()> {
    if !PROPER_TYPE_NAME_RE.is_match(name) {
        return Err(Error::BadRequest(
            "Invalid resource type: it must be 1 to 50 alphanumeric characters, '_' or '-'"
                .to_string(),
        ));
    }
    Ok(())
}

pub fn require_admin(is_admin: bool, username: &str) -> Result<()> {
    if !is_admin {
        Err(Error::RequireAdmin(username.to_string()))
    } else {
        Ok(())
    }
}

/// Configure reqwest::ClientBuilder with environment-based settings
/// When FORCE_IPV4=true environment variable is set, this configures the client
/// to only use IPv4 addresses by binding to 0.0.0.0
pub fn configure_client(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    if *FORCE_IPV4 {
        tracing::info!("FORCE_IPV4 is enabled - HTTP client will only use IPv4");
        builder.local_address(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)))
    } else {
        builder
    }
}

pub async fn require_admin_or_devops(
    is_admin: bool,
    username: &str,
    email: &str,
    // True when the caller is a job token (`$WM_TOKEN`). `devops` is instance-level
    // and `is_devops_email` is true for superadmins, so a job token whose on_behalf_of
    // a `wm_deployers` member pointed at a superadmin would otherwise clear the devops
    // branch on a workspace it isn't admin of (GHSA-hfh4-cx4h-3fcr). Workspace admin
    // (`is_admin`) stays allowed — that is the cap ceiling.
    is_job_token: bool,
    db: &DB,
) -> Result<()> {
    if !is_admin {
        if is_job_token || !is_devops_email(db, email).await? {
            return Err(Error::RequireAdmin(username.to_string()));
        }
    }
    Ok(())
}

fn instance_name(hostname: &str) -> String {
    hostname
        .replace(" ", "")
        .replace('_', "")
        .split("-")
        .last()
        .unwrap()
        .to_ascii_lowercase()
}

const DEFAULT_WORKER_SUFFIX_LEN: usize = 5;
const MAX_WORKER_SUFFIX_LABEL_LEN: usize = 64;
/// `worker_ping.worker` is a `VARCHAR(255)`.
const MAX_WORKER_NAME_LEN: usize = 255;
pub const SSH_AGENT_WORKER_SUFFIX: &'static str = "/ssh";

pub fn create_worker_suffix(hostname: &str, rd_string_len: usize) -> String {
    let wk_suffix = format!("{}-{}", instance_name(hostname), rd_string(rd_string_len));
    wk_suffix
}

pub fn create_default_worker_suffix(hostname: &str) -> String {
    create_worker_suffix(hostname, DEFAULT_WORKER_SUFFIX_LEN)
}

/// Same shape as [`create_default_worker_suffix`] but derived from the hostname and the
/// worker index instead of randomness, so the process gets the same worker name every time
/// it starts on that host. The index is folded into the digest rather than appended so that
/// the name still has exactly one suffix segment, which is what
/// [`retrieve_common_worker_prefix`] (the interactive shell tag) strips off.
fn create_stable_worker_suffix(hostname: &str, index: usize) -> String {
    let digest = calculate_hash(&format!("{hostname}#{index}"));
    format!(
        "{}-{}",
        instance_name(hostname),
        &digest[..DEFAULT_WORKER_SUFFIX_LEN]
    )
}

/// Suffix of the name of the `index`-th (1-based) worker of this process.
///
/// A worker name is the primary key of its `worker_ping` row, so it decides whether a
/// restarted process appears as a new worker or resumes the previous one. Random by default
/// (two processes must never share a row); deterministic when the worker is expected to
/// restart in place, which is the case for `EXIT_AFTER_N_JOBS`. `WORKER_SUFFIX` overrides
/// both, for hosts running several worker processes of the same worker group: the hostname
/// alone cannot tell those apart.
pub fn resolve_worker_suffix(hostname: &str, index: usize) -> anyhow::Result<String> {
    Ok(match &*WORKER_SUFFIX {
        Some(label) => create_labelled_worker_suffix(hostname, label, index)?,
        None if EXIT_AFTER_N_JOBS.is_some() => create_stable_worker_suffix(hostname, index),
        None => create_default_worker_suffix(hostname),
    })
}

/// The operator's label only has to tell the worker processes of one host apart, so it is
/// appended to the stable suffix rather than replacing it: the digest is what keeps two hosts
/// whose names end on the same segment (`worker-east-1`, `worker-west-1`) from sharing an
/// identity, and it already folds in the worker index. A `-` in the label would add a segment
/// to the worker name, which [`retrieve_common_worker_prefix`] reads as the part to strip;
/// rejected rather than rewritten, since the point of the label is that two different ones
/// give two different names.
fn create_labelled_worker_suffix(
    hostname: &str,
    label: &str,
    index: usize,
) -> anyhow::Result<String> {
    if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(anyhow::anyhow!(
            "WORKER_SUFFIX must only contain ASCII letters, digits and underscores, got '{label}'"
        ));
    }
    // The worker name is a `VARCHAR(255)` primary key and a component of the worker
    // directory's path: a label long enough to blow either only surfaces at the initial ping,
    // which the worker `expect`s.
    if label.len() > MAX_WORKER_SUFFIX_LABEL_LEN {
        return Err(anyhow::anyhow!(
            "WORKER_SUFFIX must be at most {MAX_WORKER_SUFFIX_LABEL_LEN} characters, got {}",
            label.len()
        ));
    }
    Ok(format!(
        "{}_{label}",
        create_stable_worker_suffix(hostname, index)
    ))
}

pub fn worker_name_with_suffix(is_agent: bool, worker_group: &str, suffix: &str) -> String {
    if is_agent {
        format!("{}-{}-{}", AGENT_WORKER_NAME_PREFIX, worker_group, suffix)
    } else {
        format!("{}-{}-{}", WORKER_NAME_PREFIX, worker_group, suffix)
    }
}

/// The name is the `VARCHAR(255)` primary key of `worker_ping` and a component of the worker
/// directory's path, and every part of it comes from the environment (`WORKER_GROUP`,
/// hostname, `WORKER_SUFFIX`). A name that does not fit has to stop the process here rather
/// than at the directory it creates or the initial ping it `expect`s.
pub fn checked_worker_name(
    is_agent: bool,
    worker_group: &str,
    suffix: &str,
) -> anyhow::Result<String> {
    let name = worker_name_with_suffix(is_agent, worker_group, suffix);
    if name.len() > MAX_WORKER_NAME_LEN {
        return Err(anyhow::anyhow!(
            "worker name '{name}' is {} characters, more than the {MAX_WORKER_NAME_LEN} a worker \
            name may have: shorten WORKER_GROUP or WORKER_SUFFIX",
            name.len()
        ));
    }
    Ok(name)
}

pub fn retrieve_common_worker_prefix(worker_name: &str) -> String {
    let (prefix, _) = worker_name.rsplit_once('-').unzip();

    prefix
        .expect("Invalid worker_name: expected at least one '-' in the name")
        .to_owned()
}

pub fn paginate(pagination: Pagination) -> (usize, usize) {
    let per_page = pagination
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .max(1)
        .min(MAX_PER_PAGE);
    let offset = (pagination.page.unwrap_or(1).max(1) - 1) * per_page;
    (per_page, offset)
}

pub fn paginate_without_limits(pagination: Pagination) -> (usize, usize) {
    let per_page = pagination.per_page.unwrap_or(MAX_PER_PAGE);
    let offset = (pagination.page.unwrap_or(1).max(1) - 1) * per_page;
    (per_page, offset)
}

pub async fn now_from_db<'c, E: sqlx::PgExecutor<'c>>(
    db: E,
) -> Result<chrono::DateTime<chrono::Utc>> {
    Ok(sqlx::query_scalar!("SELECT now()")
        .fetch_one(db)
        .warn_after_seconds_with_sql(1, "now_from_db".to_string())
        .await?
        .unwrap())
}

pub async fn create_directory_async(directory_path: &str) {
    AsyncDirBuilder::new()
        .recursive(true)
        .create(directory_path)
        .await
        .unwrap_or_else(|e| panic!("could not create dir '{}': {}", directory_path, e));
}

pub fn create_directory_sync(directory_path: &str) {
    SyncDirBuilder::new()
        .recursive(true)
        .create(directory_path)
        .unwrap_or_else(|e| panic!("could not create dir '{}': {}", directory_path, e));
}

#[track_caller]
pub fn not_found_if_none<T, U: AsRef<str>>(opt: Option<T>, kind: &str, name: U) -> Result<T> {
    if let Some(o) = opt {
        Ok(o)
    } else {
        let loc = Location::caller();
        Err(Error::NotFound(format!(
            "{} not found at name {} ({}:{})",
            kind,
            name.as_ref(),
            loc.file().split("/").last().unwrap_or_default(),
            loc.line()
        )))
    }
}

pub async fn query_elems_from_hub(
    http_client: &reqwest::Client,
    url: &str,
    query_params: Option<Vec<(&str, String)>>,
    db: &DB,
) -> Result<(
    reqwest::StatusCode,
    reqwest::header::HeaderMap,
    axum::body::Body,
)> {
    let response = http_get_from_hub(http_client, url, false, query_params, Some(db)).await?;

    let status = response.status();

    Ok((
        status,
        response.headers().clone(),
        axum::body::Body::from_stream(response.bytes_stream()),
    ))
}

pub async fn http_get_from_hub(
    http_client: &reqwest::Client,
    url: &str,
    plain: bool,
    query_params: Option<Vec<(&str, String)>>,
    db: Option<&Pool<Postgres>>,
) -> Result<reqwest::Response> {
    let uid = match db {
        Some(db) => match get_license_id_or_uid(db).await {
            Ok(uid) => Some(uid),
            Err(err) => {
                tracing::info!("No valid uid found: {}", err);
                None
            }
        },
        None => None,
    };

    let mut request = http_client.get(url).header(
        "Accept",
        if plain {
            "text/plain"
        } else {
            "application/json"
        },
    );

    if let Some(uid) = uid {
        request = request.header("X-uid", uid);
    }

    if let Some(hub_api_secret) = (**HUB_API_SECRET.load()).clone() {
        request = request.header("X-api-secret", hub_api_secret);
    }

    if let Some(query_params) = query_params {
        for (key, value) in query_params {
            request = request.query(&[(key, value)]);
        }
    }

    let response = request
        .send()
        .await
        .context(format!("error fetching script at {url} from hub"))?;

    Ok(response)
}

pub fn rd_string(len: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

pub fn calculate_hash(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s);
    format!("{:x}", hasher.finalize())
}

pub async fn get_license_id_or_uid<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
) -> Result<String> {
    let license_id = (**LICENSE_KEY_ID.load()).clone();

    if license_id.is_empty() {
        get_instance_uid(db).await
    } else {
        Ok(license_id)
    }
}

pub async fn get_instance_uid<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
) -> Result<String> {
    let uid_value = sqlx::query_scalar!(
        "SELECT value FROM global_settings WHERE name = $1",
        UNIQUE_ID_SETTING
    )
    .fetch_one(db)
    .await?;

    let uid = serde_json::from_value::<String>(uid_value).map_err(to_anyhow)?;

    Ok(uid)
}

pub async fn get_telemetry_ids<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
) -> Result<(String, String)> {
    let license_id = (**LICENSE_KEY_ID.load()).clone();
    let instance_uid = get_instance_uid(db).await?;
    if license_id.is_empty() {
        Ok((instance_uid.clone(), instance_uid))
    } else {
        Ok((license_id, instance_uid))
    }
}

pub fn map_string_to_number(s: &str, max_number: u64) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish() % (max_number + 1)
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Worker,
    Agent,
    Server,
    Standalone,
    Indexer,
    MCP,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Worker => write!(f, "worker"),
            Mode::Agent => write!(f, "agent"),
            Mode::Server => write!(f, "server"),
            Mode::Standalone => write!(f, "standalone"),
            Mode::Indexer => write!(f, "indexer"),
            Mode::MCP => write!(f, "mcp"),
        }
    }
}

// inspired from rails: https://github.com/rails/rails/blob/6e49cc77ab3d16c06e12f93158eaf3e507d4120e/activerecord/lib/active_record/migration.rb#L1308
pub fn generate_lock_id(database_name: &str) -> i64 {
    const CRC_IEEE: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
    // 0x3d32ad9e chosen by fair dice roll
    0x3d32ad9e * (CRC_IEEE.checksum(database_name.as_bytes()) as i64)
}

pub async fn report_critical_error(
    error_message: String,
    _db: DB,
    workspace_id: Option<&str>,
    resource: Option<&str>,
) -> () {
    tracing::error!("CRITICAL ERROR: {error_message}");

    let mute_global = CRITICAL_ALERT_MUTE_UI_ENABLED.load(Ordering::Relaxed);
    let mute_workspace = if let Some(workspace_id) = workspace_id {
        match fetch_mute_workspace(&_db, workspace_id).await {
            Ok(flag) => flag,
            Err(err) => {
                tracing::error!("Error fetching mute_workspace: {}", err);
                false
            }
        }
    } else {
        false
    };

    // we ack_global if mute_global is true, or if mute_workspace is true
    // but we ignore global mute setting for ack_workspace
    let acknowledge_workspace = mute_workspace;
    let acknowledge_global =
        mute_global || mute_workspace || (workspace_id.is_some() && *CLOUD_HOSTED);

    if let Err(err) = sqlx::query!(
        "INSERT INTO alerts (alert_type, message, acknowledged, acknowledged_workspace, workspace_id, resource)
        VALUES ('critical_error', $1, $2, $3, $4, $5)",
        error_message,
        acknowledge_global,
        acknowledge_workspace,
        workspace_id,
        resource,
    )
    .execute(&_db)
    .await
    {
        tracing::error!("Failed to save critical error to database: {}", err);
    }

    #[cfg(feature = "enterprise")]
    if *CLOUD_HOSTED && workspace_id.is_some() {
        tracing::error!(error_message)
    } else {
        send_critical_alert(error_message, &_db, CriticalAlertKind::CriticalError, None).await;
    }
}

/// Route a workspace-level failure to the instance critical alert channels without
/// recording an `alerts` row: job failures are workspace noise and would otherwise flood
/// the instance-wide feed superadmins triage. The channels belong to the instance operator,
/// who on cloud is not the workspace owner, hence the hard stop there. Callers own the
/// per-workspace opt-in.
pub async fn send_workspace_error_to_instance_channels(_error_message: String, _db: &DB) -> () {
    if *CLOUD_HOSTED {
        return;
    }

    #[cfg(feature = "enterprise")]
    send_critical_alert(_error_message, _db, CriticalAlertKind::CriticalError, None).await;
}

pub async fn report_recovered_critical_error(
    message: String,
    _db: DB,
    workspace_id: Option<&str>,
    resource: Option<&str>,
) -> () {
    tracing::info!("RECOVERED CRITICAL ERROR: {message}");

    if let Err(err) = sqlx::query!(
        "INSERT INTO alerts (alert_type, message, acknowledged, acknowledged_workspace, workspace_id, resource)
        VALUES ('recovered_critical_error', $1, $2, $3, $4, $5)",
        message,
        true,
        true,
        workspace_id,
        resource,
    )
    .execute(&_db)
    .await
    {
        tracing::error!("Failed to save recovered critical error to database: {}", err);
    }

    // acknowledge all alerts with the same resource
    if let Some(resource) = resource {
        if let Err(err) = sqlx::query!(
            "UPDATE alerts SET acknowledged = true, acknowledged_workspace = true WHERE resource = $1 AND alert_type = 'critical_error'",
            resource,
        )
        .execute(&_db)
        .await
        {
            tracing::error!("Failed to acknowledge critical error alerts for resource {}: {}", resource, err);
        }
    }

    #[cfg(feature = "enterprise")]
    if *CLOUD_HOSTED && workspace_id.is_some() {
        tracing::error!(message);
    } else {
        send_critical_alert(
            message,
            &_db,
            CriticalAlertKind::RecoveredCriticalError,
            None,
        )
        .await;
    }
}

pub trait IsEmpty {
    fn is_empty(&self) -> bool;
}

impl IsEmpty for String {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<T> IsEmpty for Vec<T> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<T> IsEmpty for Option<T>
where
    T: IsEmpty,
{
    fn is_empty(&self) -> bool {
        match self {
            Some(v) => v.is_empty(),
            None => true,
        }
    }
}

pub fn empty_as_none<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + IsEmpty,
{
    let option = <Option<T> as serde::Deserialize>::deserialize(deserializer)?;
    Ok(option.filter(|s| !s.is_empty()))
}

pub fn is_empty<T>(value: &T) -> bool
where
    T: IsEmpty,
{
    value.is_empty()
}

pub fn deserialize_url<'de, D: Deserializer<'de>>(
    de: D,
) -> std::result::Result<Option<Url>, D::Error> {
    let intermediate = <Option<Cow<'de, str>>>::deserialize(de)?;

    match intermediate.as_deref() {
        None | Some("") => Ok(None),
        Some(non_empty_string) => Url::parse(non_empty_string)
            .map(Some)
            .map_err(D::Error::custom),
    }
}

pub async fn fetch_mute_workspace(_db: &DB, workspace_id: &str) -> Result<bool> {
    match sqlx::query!(
        "SELECT mute_critical_alerts FROM workspace_settings WHERE workspace_id = $1",
        workspace_id
    )
    .fetch_optional(_db)
    .await
    {
        Ok(Some(record)) => Ok(record.mute_critical_alerts.unwrap_or(false)),
        Ok(None) => {
            tracing::warn!(
                "Workspace ID {} not found in workspace_settings table",
                workspace_id
            );
            Ok(false)
        }
        Err(err) => {
            tracing::error!(
                "Error querying workspace_settings for workspace_id {}: {}",
                workspace_id,
                err
            );
            return Err(err.into());
        }
    }
}

// build_arg_str(&[("name", Some("value")), ("name2", None)], " ", "=")
pub fn build_arg_str(args: &[(&str, Option<&str>)], sep: &str, eq: &str) -> String {
    args.iter()
        .filter_map(|(k, v)| {
            if let Some(value) = v {
                Some(format!("{}{}{}", k, eq, value))
            } else {
                None
            }
        })
        .join(sep)
}

// Some errors (duckdb) leak the password in the error message
pub fn sanitize_string_from_password(s: &str, passwd: &str) -> Option<String> {
    if s.contains(passwd) {
        return Some(s.replace(passwd, "******"));
    }
    // Do NOT check substrings
    // In the case the user finds a string and notices that it gets substituted,
    // He can very easily find the next character in O(1) and thus the entire password
    None
}

pub enum ScheduleType {
    Croner(Cron),
    Cron(cron::Schedule),
}

/// croner reads the leading seconds field as optional or required depending on these flags,
/// so anything asking whether an expression parses has to ask it the way the caller did.
fn croner_parser(schedule_str: &str, seconds_required: bool) -> Cron {
    let mut croner = Cron::new(schedule_str);
    if seconds_required {
        croner.with_seconds_required();
    } else {
        croner.with_seconds_optional();
    }
    croner
}

/// Probes an expression this module synthesized rather than one that was submitted, so it
/// goes around `from_str`, whose failure path logs at ERROR level.
fn parses_as_cron(schedule_str: &str, version: Option<&str>, seconds_required: bool) -> bool {
    match version {
        Some("v1") | None => cron::Schedule::from_str(schedule_str).is_ok(),
        Some(_) => panic::catch_unwind(AssertUnwindSafe(|| {
            croner_parser(schedule_str, seconds_required)
                .parse()
                .is_ok()
        }))
        .unwrap_or(false),
    }
}

/// Both cron parsers reject the standard 5-field crontab syntax without naming the missing
/// leading seconds field, and croner even advertises five fields as valid while we parse
/// with seconds required. The hint belongs to that seconds-required parse alone: croner does
/// accept five fields once seconds are optional, which is how the worker re-reads a schedule.
fn six_fields_hint(schedule_str: &str, version: Option<&str>, seconds_required: bool) -> String {
    let fields = schedule_str.split_whitespace().collect::<Vec<_>>();
    if !seconds_required || fields.len() >= 6 {
        return String::new();
    }
    // A restricted weekday is where v1 parts ways with crontab: it numbers weekdays from
    // Sunday=1, and it intersects day-of-month with day-of-week where crontab unions them.
    // Once the weekday is unrestricted the remaining fields carry their crontab meaning, so
    // that is the only case on v1 where a concrete expression can be handed back.
    let v1_weekday_restricted = matches!(version, Some("v1") | None)
        && fields.get(4).is_some_and(|dow| *dow != "*" && *dow != "?");
    let with_seconds = format!("0 {}", fields.join(" "));
    let example = if fields.len() == 5
        && !v1_weekday_restricted
        && parses_as_cron(&with_seconds, version, seconds_required)
    {
        format!(
            " The 5-field crontab syntax is not accepted; prepend a seconds field, e.g. '{}'.",
            with_seconds
        )
    } else {
        String::new()
    };
    format!(
        "\nWindmill cron expressions have 6 fields and start with seconds: \
         'sec min hour day-of-month month day-of-week'.{}",
        example
    )
}

impl ScheduleType {
    pub fn find_next(
        &self,
        starting_from: &chrono::DateTime<chrono_tz::Tz>,
    ) -> chrono::DateTime<chrono_tz::Tz> {
        match self {
            ScheduleType::Croner(croner_schedule) => croner_schedule
                .find_next_occurrence(starting_from, false)
                .expect("cron: a schedule should have a next event"),
            ScheduleType::Cron(schedule) => schedule
                .after(starting_from)
                .next()
                .expect("cron: a schedule should have a next event"),
        }
    }

    pub fn from_str(
        schedule_str: &str,
        version: Option<&str>,
        seconds_required: bool,
    ) -> Result<ScheduleType> {
        tracing::debug!(
            "Attempting to parse schedule string: {}, with version: {:?}",
            schedule_str,
            version
        );

        match version {
            Some("v1") | None => {
                // Use Cron for v1 or if not provided
                cron::Schedule::from_str(schedule_str)
                    .map(ScheduleType::Cron)
                    .map_err(|e| {
                        tracing::error!(
                            "Failed to parse schedule string '{}' using Cron: {}",
                            schedule_str,
                            e
                        );
                        Error::BadRequest(format!(
                            "cron: {}{}",
                            e,
                            six_fields_hint(schedule_str, version, seconds_required)
                        ))
                    })
            }
            Some("v2") | Some(_) => {
                // Use Croner for v2
                let schedule_type_result = panic::catch_unwind(AssertUnwindSafe(|| {
                    croner_parser(schedule_str, seconds_required).parse()
                }))
                .map_err(|_| {
                    tracing::error!(
                        "A panic occurred while parsing schedule string '{}' using Croner",
                        schedule_str,
                    );
                    Error::BadRequest(format!("cron: a panic occurred during schedule parsing"))
                })
                .and_then(|parse_result| {
                    parse_result.map(ScheduleType::Croner).map_err(|e| {
                        tracing::error!(
                            "Failed to parse schedule string '{}' using Croner: {}",
                            schedule_str,
                            e
                        );
                        Error::BadRequest(format!(
                            "cron: {}{}",
                            e,
                            six_fields_hint(schedule_str, version, seconds_required)
                        ))
                    })
                });

                // Additional check to make sure the provided schedule can generate a next event
                if let Ok(ScheduleType::Croner(croner_schedule)) = &schedule_type_result {
                    let test_time = chrono::Utc::now().with_timezone(&chrono_tz::UTC);
                    let result = panic::catch_unwind(AssertUnwindSafe(|| {
                        croner_schedule
                            .find_next_occurrence(&test_time, false)
                            .expect("cron: a schedule should have a next event");
                    }));
                    if let Err(_) = result {
                        tracing::error!("A panic occurred while finding the next occurrence");
                        return Err(Error::BadRequest(format!(
                            "cron: a panic occurred during find_next_occurrence"
                        )));
                    }

                    if let Err(e) = result {
                        tracing::error!(
                            "An error occurred while finding the next occurrence: {:?}",
                            e
                        );
                        return Err(Error::BadRequest(format!(
                            "cron: error during find_next_occurrence: {:?}",
                            e
                        )));
                    }
                }

                schedule_type_result
            }
        }
    }

    pub fn upcoming(
        &self,
        tz: chrono_tz::Tz,
        count: usize, // Number of upcoming events to take
    ) -> Result<Vec<chrono::DateTime<Utc>>> {
        let start_time = Utc::now().with_timezone(&tz);

        let mut events: Vec<chrono::DateTime<Utc>> = Vec::with_capacity(count);

        match self {
            ScheduleType::Croner(croner_schedule) => {
                croner_schedule
                    .iter_from(start_time)
                    .map(|dt| dt.with_timezone(&Utc))
                    .take(count)
                    .for_each(|event| events.push(event));
            }
            ScheduleType::Cron(schedule) => {
                schedule
                    .upcoming(tz)
                    .map(|x| x.with_timezone(&Utc))
                    .take(count)
                    .for_each(|event| events.push(event));
            }
        };

        // Make sure the schedule is valid and can actually generate "count" events
        if events.len() != count {
            return Err(Error::BadRequest(format!(
                "cron: failed to generate the requested number of events. Expected {}, got {}",
                count,
                events.len()
            )));
        }

        Ok(events)
    }
}

use std::future::Future;
use std::pin::Pin;
use std::task::{Context as TContext, Poll};
use tokio::time::{self, Duration, Sleep};

use pin_project_lite::pin_project;

pub trait WarnAfterExt: Future + Sized {
    /// Warns if the future takes longer than the specified number of seconds to complete.
    #[track_caller]
    fn warn_after_seconds(self, seconds: u8) -> WarnAfterFuture<Self> {
        let caller = Location::caller();
        self.build_from_caller(seconds, caller, None)
    }

    fn build_from_caller(
        self,
        seconds: u8,
        caller: &Location,
        sql: Option<String>,
    ) -> WarnAfterFuture<Self> {
        let location = format!("{}:{}", caller.file(), caller.line());
        WarnAfterFuture {
            future: self,
            timeout: time::sleep(Duration::from_secs(seconds as u64)),
            warned: false,
            start_time: std::time::Instant::now(),
            location,
            seconds,
            sql,
        }
    }
    #[track_caller]
    fn warn_after_seconds_with_sql(self, seconds: u8, sql: String) -> WarnAfterFuture<Self> {
        let caller = Location::caller();
        self.build_from_caller(seconds, caller, Some(sql))
    }
}

// Blanket implementation for all futures.
impl<F: Future> WarnAfterExt for F {}

pin_project! {
    /// A future that wraps another future and prints a warning if it takes too long.
    pub struct WarnAfterFuture<F> {
        #[pin]
        future: F,
        #[pin]
        timeout: Sleep,
        warned: bool,
        location: String,
        start_time: std::time::Instant,
        seconds: u8,
        sql: Option<String>,
    }
}

impl<F: Future> Future for WarnAfterFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut TContext<'_>) -> Poll<Self::Output> {
        let this = self.project();

        fn build_query_string(location: &str, sql: Option<&str>) -> String {
            match sql {
                Some(sql) => format!("{}: {}", location, sql),
                None => location.to_string(),
            }
        }

        // Poll the timeout future to check if it has elapsed.
        if !*this.warned {
            if this.timeout.poll(cx).is_ready() {
                tracing::warn!(
                    location = this.location,
                    "SLOW_QUERY: query {} to db taking longer than expected (> {} seconds)",
                    build_query_string(&this.location, this.sql.as_deref()),
                    this.seconds,
                );
                *this.warned = true;
            }
        }

        // Poll the wrapped future.
        match this.future.poll(cx) {
            Poll::Ready(output) => {
                if *this.warned {
                    let elapsed = this.start_time.elapsed();
                    tracing::warn!(
                        location = this.location,
                        "SLOW_QUERY: completed query {} with total duration: {:.2?}",
                        build_query_string(&this.location, this.sql.as_deref()),
                        elapsed
                    );
                }
                Poll::Ready(output)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(sqlx::Type, Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[sqlx(type_name = "runnable_kind", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum RunnableKind {
    Script,
    Flow,
}

impl Display for RunnableKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let runnable_kind = match self {
            RunnableKind::Script => "script",
            RunnableKind::Flow => "flow",
        };
        write!(f, "{}", runnable_kind)
    }
}

#[derive(Clone)]
pub struct ExpiringCacheEntry<T> {
    pub value: T,
    pub expiry: std::time::Instant,
}

impl<T> ExpiringCacheEntry<T> {
    pub fn is_expired(&self) -> bool {
        self.expiry < std::time::Instant::now()
    }
}

pub async fn refresh_custom_instance_user_pwd(db: &DB) -> Result<()> {
    let query = r#"
    DO $$
        DECLARE
            pwd text;
        BEGIN
            SELECT gen_random_uuid()::text INTO pwd;

            IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'custom_instance_user') THEN
                EXECUTE format('ALTER USER custom_instance_user WITH PASSWORD %L', pwd);
            ELSE
                EXECUTE format('CREATE USER custom_instance_user WITH PASSWORD %L', pwd);
            END IF;

            IF NOT EXISTS (SELECT 1 FROM global_settings WHERE name = 'custom_instance_pg_databases') THEN
                INSERT INTO global_settings (name, value)
                VALUES ('custom_instance_pg_databases', jsonb_build_object(
                'user_pwd', pwd::text,
                'databases', jsonb_build_object()
                ));
            ELSE
                UPDATE global_settings
                SET value = jsonb_set(COALESCE(value, '{}'::jsonb), '{user_pwd}', to_jsonb(pwd::text)::jsonb)
                WHERE name = 'custom_instance_pg_databases';
            END IF;
        END
        $$;
    "#;
    sqlx::query(query).execute(db).await?;
    Ok(())
}

pub async fn get_custom_pg_instance_password(db: &DB) -> Result<String> {
    sqlx::query_scalar!(
        "SELECT value->>'user_pwd' FROM global_settings WHERE name = 'custom_instance_pg_databases';"
    )
    .fetch_optional(db)
    .await?
    .flatten().ok_or_else(||
        Error::BadRequest(format!(
            "Custom instance db password not found, did you run migrations ?"
        ))
    )
}

/// PL/pgSQL granting `custom_instance_replication_user` the ability to open replication
/// connections, inlined into the `DO` blocks below.
///
/// The REPLICATION attribute requires a real superuser on PG <= 15 (PG 16+ accepts
/// CREATEROLE + REPLICATION), and managed postgres never hands one out: on RDS the
/// capability is carried by the `rds_replication` role instead. Both privilege failures
/// raise `insufficient_privilege`, so the attribute is attempted first and the provider
/// role is the fallback.
const GRANT_REPLICATION_CAPABILITY_PLPGSQL: &str = r#"
            BEGIN
                EXECUTE 'ALTER ROLE custom_instance_replication_user REPLICATION';
            EXCEPTION WHEN insufficient_privilege THEN
                IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'rds_replication') THEN
                    EXECUTE 'GRANT rds_replication TO custom_instance_replication_user';
                ELSE
                    RAISE;
                END IF;
            END;

            IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'custom_instance_user') THEN
                EXECUTE 'GRANT custom_instance_user TO custom_instance_replication_user';
                -- Stripping REPLICATION off custom_instance_user is a cleanup, so it is both
                -- guarded and best-effort: the clause is superuser-only on PG <= 15 even when the
                -- attribute is already unset, and failing it must not roll back the role above.
                IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'custom_instance_user' AND rolreplication) THEN
                    BEGIN
                        EXECUTE 'ALTER ROLE custom_instance_user NOREPLICATION';
                    EXCEPTION WHEN insufficient_privilege THEN
                        NULL;
                    END;
                END IF;
            END IF;
"#;

/// `rotate_condition` decides when a new password is generated: always for the refresh
/// endpoint, only when the role or its stored password is missing for the boot converge.
/// Both variants are a single statement so they can run on any executor, and both are
/// atomic: a role that cannot be granted replication is rolled back rather than left
/// half-provisioned.
fn provision_replication_user_sql(rotate_condition: &str) -> String {
    format!(
        r#"
    DO $$
        DECLARE
            pwd text;
        BEGIN
            -- Same lock as get_custom_pg_instance_replication_password: without it, every API
            -- replica booting onto a version that provisions the role races into CREATE USER,
            -- and the losers abort on duplicate_object.
            PERFORM pg_advisory_xact_lock(hashtext('custom_instance_replication_pwd'));

            IF {rotate_condition} THEN
                SELECT gen_random_uuid()::text INTO pwd;

                IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'custom_instance_replication_user') THEN
                    EXECUTE format('ALTER USER custom_instance_replication_user WITH PASSWORD %L', pwd);
                ELSE
                    EXECUTE format('CREATE USER custom_instance_replication_user WITH PASSWORD %L', pwd);
                END IF;

                INSERT INTO global_settings (name, value)
                VALUES ('custom_instance_replication_pwd', to_jsonb(pwd::text))
                ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value;
            END IF;
{GRANT_REPLICATION_CAPABILITY_PLPGSQL}
        END
        $$;
"#
    )
}

lazy_static::lazy_static! {
    static ref REFRESH_CUSTOM_INSTANCE_REPLICATION_USER_SQL: String =
        provision_replication_user_sql("TRUE");

    // The password test mirrors REPLICATION_PWD_READ_SQL, whose readers flatten a NULL away: a row
    // holding a JSON null reads back as no password, so it must rotate rather than count as
    // provisioned and strand the getter.
    static ref ENSURE_CUSTOM_INSTANCE_REPLICATION_USER_SQL: String = provision_replication_user_sql(
        "NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'custom_instance_replication_user')
                OR NOT EXISTS (SELECT 1 FROM global_settings
                    WHERE name = 'custom_instance_replication_pwd' AND value #>> '{}' IS NOT NULL)"
    );
}

const REPLICATION_PWD_READ_SQL: &str =
    "SELECT value #>> '{}' FROM global_settings WHERE name = 'custom_instance_replication_pwd'";

/// (Re)create `custom_instance_replication_user` with a fresh password. This role is
/// used by postgres trigger connections on custom-instance datatables; membership in
/// `custom_instance_user` lets it manage publications on the datatable tables.
///
/// Authorization: rotates a stored database credential and performs no authorization
/// itself — callers MUST restrict this to superadmin or internal server paths.
pub async fn refresh_custom_instance_replication_user_pwd(db: &DB) -> Result<()> {
    sqlx::query(REFRESH_CUSTOM_INSTANCE_REPLICATION_USER_SQL.as_str())
        .execute(db)
        .await?;
    Ok(())
}

/// Create `custom_instance_replication_user` if it is missing and (re)grant it replication,
/// leaving an existing password in place. Idempotent, so it can be converged on every boot.
///
/// Authorization: same contract as [`refresh_custom_instance_replication_user_pwd`] —
/// callers MUST restrict this to superadmin or internal server paths.
pub async fn ensure_custom_instance_replication_user<'c>(
    executor: impl sqlx::PgExecutor<'c>,
) -> Result<()> {
    sqlx::query(ENSURE_CUSTOM_INSTANCE_REPLICATION_USER_SQL.as_str())
        .execute(executor)
        .await?;
    Ok(())
}

/// Authorization: returns a stored database credential and performs no authorization
/// itself — callers MUST restrict this to superadmin or internal server paths (mirrors
/// [`get_custom_pg_instance_password`]).
pub async fn get_custom_pg_instance_replication_password(db: &DB) -> Result<String> {
    // Fast path: already provisioned by the migration.
    if let Some(pwd) = sqlx::query_scalar::<_, Option<String>>(REPLICATION_PWD_READ_SQL)
        .fetch_optional(db)
        .await?
        .flatten()
    {
        return Ok(pwd);
    }
    // Self-heal when the role-creating migration was swallowed. The advisory lock + re-check
    // serialize concurrent workers: otherwise two callers both rotate, and the second
    // rotation invalidates the password the first already returned. Rotating and reading in
    // one locked transaction keeps the decision atomic.
    let mut tx = db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('custom_instance_replication_pwd'))")
        .execute(&mut *tx)
        .await?;
    if let Some(pwd) = sqlx::query_scalar::<_, Option<String>>(REPLICATION_PWD_READ_SQL)
        .fetch_optional(&mut *tx)
        .await?
        .flatten()
    {
        tx.commit().await?;
        return Ok(pwd);
    }
    sqlx::query(ENSURE_CUSTOM_INSTANCE_REPLICATION_USER_SQL.as_str())
        .execute(&mut *tx)
        .await?;
    let pwd = sqlx::query_scalar::<_, Option<String>>(REPLICATION_PWD_READ_SQL)
        .fetch_optional(&mut *tx)
        .await?
        .flatten()
        .ok_or_else(|| {
            Error::BadRequest(
                "Custom instance replication user password not found, did you run migrations ?"
                    .to_string(),
            )
        })?;
    tx.commit().await?;
    Ok(pwd)
}

/// Convert a JSON string to a `Box<RawValue>` without validation.
///
/// # Safety
/// The caller must ensure the string is valid JSON.
pub fn unsafe_raw(json: String) -> Box<serde_json::value::RawValue> {
    unsafe { std::mem::transmute::<Box<str>, Box<serde_json::value::RawValue>>(json.into()) }
}

// Avoid JSON parsing for merging raw JSON values into an object
pub fn merge_raw_values_to_object(
    pairs: &[(String, Box<serde_json::value::RawValue>)],
) -> Box<serde_json::value::RawValue> {
    let mut result = String::from("{");

    for (i, (key, value)) in pairs.iter().enumerate() {
        if i > 0 {
            result.push(',');
        }
        // Serialize the key (handles escaping)
        result.push_str(&serde_json::to_string(&key).unwrap());
        result.push(':');
        result.push_str(value.get());
    }

    result.push('}');

    serde_json::value::RawValue::from_string(result).unwrap()
}

// Avoid JSON parsing for merging raw JSON values into an array
pub fn merge_raw_values_to_array(
    values: &[Box<serde_json::value::RawValue>],
) -> Box<serde_json::value::RawValue> {
    let mut result = String::from("[");

    for (i, value) in values.iter().enumerate() {
        if i > 0 {
            result.push(',');
        }
        result.push_str(value.get());
    }

    result.push(']');

    serde_json::value::RawValue::from_string(result).unwrap()
}

// Optimisation to avoid allocating intermediate strings when merging nested raw JSON values into an array
pub fn merge_nested_raw_values_to_array<
    'a,
    It1: Iterator<Item = It2>,
    It2: Iterator<Item = &'a Box<serde_json::value::RawValue>>,
>(
    nested_values: It1,
) -> Box<serde_json::value::RawValue> {
    let mut result = String::from("[");
    let mut outer_first = true;

    for inner_iter in nested_values {
        if !outer_first {
            result.push(',');
        } else {
            outer_first = false;
        }

        result.push('[');
        let mut inner_first = true;

        for value in inner_iter {
            if !inner_first {
                result.push(',');
            } else {
                inner_first = false;
            }
            result.push_str(value.get());
        }

        result.push(']');
    }

    result.push(']');

    serde_json::value::RawValue::from_string(result).unwrap()
}

/// Remove every U+0000 (NUL) from a serialized JSON document so it is safe to
/// store in a `jsonb` column, which rejects the `\u0000` escape with 22P05
/// ("unsupported Unicode escape sequence"). A `json`-typed column accepts the
/// escape but propagates the same failure to any later `->>`/`to_jsonb`/`json`→
/// `jsonb` conversion.
///
/// A NUL can only appear in JSON text as a backslash-u0000 escape, and a
/// backslash only ever occurs inside a string, so one backslash-parity-aware
/// pass removes every real NUL escape — covering values and keys alike — while
/// leaving a legitimate `\\u0000` (an escaped backslash followed by the literal
/// text `u0000`, common in minified JS regexes) intact. O(n) over the bytes with
/// no `serde_json::Value` tree to allocate, and the fast path (no such substring
/// at all) returns the input borrowed and untouched. The slow path is reached
/// not only by genuinely poisoned values but by any value that legitimately
/// contains `u0000` after a backslash (e.g. script source), so it must stay
/// allocation-light for potentially large documents.
pub fn strip_json_nul(serialized: &str) -> Cow<'_, str> {
    // SIMD substring scan (several times faster than `str::contains`'s Two-Way)
    // for the guard, since this runs on every completed job's serialized result.
    if memchr::memmem::find(serialized.as_bytes(), b"\\u0000").is_none() {
        return Cow::Borrowed(serialized);
    }
    let bytes = serialized.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    // The substring guard above is satisfied by legitimate `\\u0000` too, so only
    // an odd-parity NUL escape actually drops bytes. Borrow back out when nothing
    // was stripped, so `Cow::Owned` reliably means "a NUL was removed" — callers
    // (e.g. apps.rs) key a warning on that.
    let mut stripped = false;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        // Consume the whole run of backslashes. An even run is N/2 escaped
        // backslashes and leaves the next char unescaped; an odd run ends in an
        // escaping backslash, so a following `u0000` is a real NUL escape.
        let run_start = i;
        while i < bytes.len() && bytes[i] == b'\\' {
            i += 1;
        }
        let run = i - run_start;
        if run % 2 == 1 && bytes[i..].starts_with(b"u0000") {
            // Drop the escaping backslash + `u0000`; keep the leading literal pairs.
            out.extend(std::iter::repeat(b'\\').take(run - 1));
            i += 5;
            stripped = true;
        } else {
            out.extend(std::iter::repeat(b'\\').take(run));
        }
    }
    if !stripped {
        return Cow::Borrowed(serialized);
    }
    // Only whole ASCII backslash-u0000 escapes were removed, so the bytes remain
    // valid UTF-8 (and valid JSON).
    Cow::Owned(String::from_utf8(out).expect("removing a NUL escape preserves valid UTF-8"))
}

/// Prefix of `s` holding at most `max_chars` characters.
/// Slicing by byte index (`&s[..n]`) panics when `n` lands inside a multibyte character.
pub fn truncate_chars(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((byte_idx, _)) => &s[..byte_idx],
        None => s,
    }
}

/// Keep at most `max_chars` characters of `s`, appending `...` when anything was dropped —
/// so a truncated result is `max_chars + 3` characters long, not `max_chars`.
pub fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    let truncated = truncate_chars(s, max_chars);
    if truncated.len() < s.len() {
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 5-field crontab line is the most common way to get a schedule rejected, and both
    /// parsers report it in terms a crontab user cannot act on, so the seconds field and the
    /// equivalent expression must reach the caller for v1 and v2 alike.
    #[test]
    fn five_field_cron_error_names_the_seconds_field() {
        for version in [None, Some("v1"), Some("v2")] {
            let err = ScheduleType::from_str("0 2 * * *", version, true)
                .err()
                .expect("5-field cron must be rejected")
                .to_string();
            assert!(err.contains("6 fields"), "{version:?}: {err}");
            assert!(
                err.contains("prepend a seconds field, e.g. '0 0 2 * * *'."),
                "{version:?}: {err}"
            );
        }
    }

    /// On v1 a restricted weekday means something else than it does in the crontab line being
    /// rewritten: `1` is Sunday there, and a weekday alongside a day-of-month intersects
    /// instead of unions. Neither can be handed back as an expression to use; croner reads
    /// both the crontab way, so the same inputs keep their example.
    #[test]
    fn restricted_weekday_example_is_withheld_on_v1_only() {
        for schedule in ["0 2 * * 1", "0 2 1 * MON"] {
            let v1 = ScheduleType::from_str(schedule, None, true)
                .err()
                .expect("5-field cron must be rejected")
                .to_string();
            assert!(v1.contains("6 fields"), "{schedule}: {v1}");
            assert!(!v1.contains("e.g."), "{schedule}: {v1}");

            let v2 = ScheduleType::from_str(schedule, Some("v2"), true)
                .err()
                .expect("5-field cron must be rejected")
                .to_string();
            assert!(
                v2.contains(&format!("prepend a seconds field, e.g. '0 {schedule}'.")),
                "{schedule}: {v2}"
            );
        }
    }

    #[test]
    fn cron_error_on_other_arities_is_left_alone() {
        let err = ScheduleType::from_str("0 0 2 * * bogus", Some("v2"), true)
            .err()
            .expect("invalid cron must be rejected")
            .to_string();
        assert!(!err.contains("6 fields"), "{err}");
    }

    /// A worker that restarts must land on the exact same name to reclaim its `worker_ping`
    /// row, while still never colliding with the other workers of its own process. The
    /// suffix must also stay a single `-` segment, which is what the interactive shell tag
    /// strips off.
    #[test]
    fn stable_worker_suffix_is_per_host_and_per_index() {
        let first = create_stable_worker_suffix("wm-worker-7d8f9c-abcde", 1);
        assert_eq!(
            first,
            create_stable_worker_suffix("wm-worker-7d8f9c-abcde", 1)
        );
        assert_ne!(
            first,
            create_stable_worker_suffix("wm-worker-7d8f9c-abcde", 2)
        );
        assert_ne!(
            first,
            create_stable_worker_suffix("wm-worker-7d8f9c-fghij", 1)
        );
        assert_eq!(
            retrieve_common_worker_prefix(&worker_name_with_suffix(false, "default", &first)),
            "wk-default-abcde"
        );
    }

    /// The operator's label has to survive into the name for them to recognize the worker,
    /// while staying one segment so it does not shift the interactive shell tag, and it must
    /// still leave the workers of one process with distinct names. Two processes given
    /// different labels must never end up with the same one, which is why a label that does
    /// not fit a single segment is rejected instead of being rewritten into one that does.
    #[test]
    fn labelled_worker_suffix_stays_one_segment() {
        let label =
            |hostname, label, index| create_labelled_worker_suffix(hostname, label, index).unwrap();
        let first = label("wm-worker-abcde", "slot_a", 1);
        assert!(
            first.starts_with("abcde-") && first.ends_with("_slot_a"),
            "{first}"
        );
        assert_eq!(first, label("wm-worker-abcde", "slot_a", 1));
        // Neither another worker of this process, nor another label, nor a host whose name
        // happens to end on the same segment, may land on this identity.
        for other in [
            label("wm-worker-abcde", "slot_a", 2),
            label("wm-worker-abcde", "slot_b", 1),
            label("wm-worker-east-abcde", "slot_a", 1),
        ] {
            assert_ne!(first, other);
            assert_eq!(
                retrieve_common_worker_prefix(&worker_name_with_suffix(false, "default", &other)),
                "wk-default-abcde"
            );
        }
        for rejected in [
            "slot-a",
            "slot a",
            "slot.a",
            "slot/a",
            &"s".repeat(MAX_WORKER_SUFFIX_LABEL_LEN + 1),
        ] {
            assert!(
                create_labelled_worker_suffix("wm-worker-abcde", rejected, 1).is_err(),
                "{rejected} should be rejected"
            );
        }
    }

    /// Every part of a worker name comes from the environment, so the name can only be kept
    /// within what `worker_ping.worker` holds by checking the assembled thing.
    #[test]
    fn worker_name_longer_than_the_ping_key_is_refused() {
        let suffix = "abcde-a1b2c_slot";
        let room = MAX_WORKER_NAME_LEN - worker_name_with_suffix(false, "", suffix).len();
        let fits = checked_worker_name(false, &"g".repeat(room), suffix).unwrap();
        assert_eq!(fits.len(), MAX_WORKER_NAME_LEN);
        assert!(checked_worker_name(false, &"g".repeat(room + 1), suffix).is_err());
    }

    /// The guards are only safe because they are never stricter than the DB
    /// constraints they front. Narrowing `\w` to ASCII reads equivalent and
    /// compiles, but would start rejecting paths that already deploy today.
    #[test]
    fn proper_path_matches_the_db_constraint() {
        for ok in [
            "u/admin/foo",
            "f/some-folder/bar/baz",
            "g/all/x",
            "u/usér/nom",
        ] {
            assert!(check_proper_path(ok).is_ok(), "{ok} should be accepted");
        }
        for bad in [
            "a/admin/foo",
            "u/admin",
            "u/admin/foo/",
            "u/admin/lawful_variable/x<script>alert(1)</script>",
            "<script>alert(1)</script>",
            // Postgres' `\w` covers neither join controls nor combining marks;
            // Rust's `\w` covers both, so `[\w-]` here would pass these through
            // to the constraint and leak its message back to the caller.
            "u/admin/a\u{200C}b",
            "u/admin/a\u{0301}b",
            "u/admin/a\u{203F}b",
        ] {
            assert!(check_proper_path(bad).is_err(), "{bad} should be rejected");
        }
        assert!(check_proper_path(&format!("u/admin/{}", "a".repeat(300))).is_err());
    }

    #[test]
    fn proper_type_name_matches_the_db_constraint() {
        for ok in ["postgresql", "c_aws_account", "my-type", &"a".repeat(50)] {
            assert!(
                check_proper_type_name(ok).is_ok(),
                "{ok} should be accepted"
            );
        }
        for bad in [
            "",
            &"a".repeat(51),
            "<img src=x onerror=prompt('hacked')>",
            "a\u{200C}b",
        ] {
            assert!(
                check_proper_type_name(bad).is_err(),
                "{bad} should be rejected"
            );
        }
    }

    #[test]
    fn truncate_handles_multibyte_at_boundary() {
        // Byte 25 of this string falls inside a 2-byte 'а'; naive `&s[..25]` would panic.
        let cyrillic = "а".repeat(30);
        assert_eq!(truncate_chars(&cyrillic, 25), "а".repeat(25));
        assert_eq!(
            truncate_with_ellipsis(&cyrillic, 25),
            format!("{}...", "а".repeat(25))
        );
        assert_eq!(truncate_with_ellipsis("ааааааааааааа", 25), "ааааааааааааа");
        assert_eq!(truncate_chars("abcd", 3), "abc");
        assert_eq!(truncate_with_ellipsis("abc", 3), "abc");
        assert_eq!(truncate_with_ellipsis("abcd", 3), "abc...");
    }

    // The 6-char JSON escape for U+0000: backslash + "u0000". Written via an
    // escaped backslash so no literal NUL byte ever appears in this source.
    const NUL_ESC: &str = "\\u0000";

    // Parse the (NUL-free) result so assertions read clearly.
    fn parsed(s: &str) -> serde_json::Value {
        serde_json::from_str(s).expect("strip_json_nul must return valid JSON")
    }

    #[test]
    fn strip_json_nul_clean_value_is_borrowed_byte_for_byte() {
        let s = r#"{"summary":"all good","n":1}"#;
        let out = strip_json_nul(s);
        assert!(matches!(out, Cow::Borrowed(_)));
        assert_eq!(out, s);
    }

    #[test]
    fn strip_json_nul_real_nul_in_value_is_stripped() {
        let input = format!(r#"{{"summary":"hi{NUL_ESC}there"}}"#);
        let out = strip_json_nul(&input);
        assert!(!out.contains(NUL_ESC));
        assert_eq!(parsed(&out)["summary"], "hithere");
    }

    #[test]
    fn strip_json_nul_legit_escaped_backslash_is_a_noop() {
        // JSON "a\\u0000b" decodes to a,backslash,u,0,0,0,0,b - not a NUL - so
        // the value is already clean and round-trips byte-for-byte. It hits the
        // slow path (the substring is present) but strips nothing, so it must
        // still return Cow::Borrowed - callers key a "stripped NUL" warning on
        // the Owned variant.
        let s = r#"{"summary":"a\\u0000b"}"#;
        let out = strip_json_nul(s);
        assert!(matches!(out, Cow::Borrowed(_)));
        assert_eq!(out, s);
    }

    #[test]
    fn strip_json_nul_collision_real_and_literal_both_handled() {
        // "a" carries a real NUL escape; "b" carries the literal text backslash-u0000.
        let v = parsed(&strip_json_nul(&format!(
            r#"{{"a":"x{NUL_ESC}y","b":"p\\u0000q"}}"#
        )));
        assert_eq!(v["a"], "xy");
        assert_eq!(v["b"], "p\\u0000q");
    }

    #[test]
    fn strip_json_nul_nested_values_and_keys_are_cleaned() {
        let input =
            format!(r#"{{"o":{{"k{NUL_ESC}":["a{NUL_ESC}b",{{"deep{NUL_ESC}":"v{NUL_ESC}"}}]}}}}"#);
        let out = strip_json_nul(&input);
        assert!(!out.contains(NUL_ESC));
        let v = parsed(&out);
        assert_eq!(v["o"]["k"][0], "ab");
        assert_eq!(v["o"]["k"][1]["deep"], "v");
    }

    #[test]
    fn strip_json_nul_odd_backslash_run_keeps_literal_drops_nul() {
        // JSON "a\\ b" is an escaped backslash (kept) immediately followed
        // by a real NUL escape (dropped) -> decodes to a,backslash,b.
        let v = parsed(&strip_json_nul(&format!(r#"{{"x":"a\\{NUL_ESC}b"}}"#)));
        assert_eq!(v["x"], "a\\b");
    }

    #[test]
    fn test_build_arg_str() {
        let r = build_arg_str(
            &[
                ("host", Some("localhost")),
                ("port", Some("5432")),
                ("password", None),
                ("user", Some("postgres")),
                ("dbname", Some("test_db")),
            ],
            " ",
            "=",
        );
        assert_eq!(r, "host=localhost port=5432 user=postgres dbname=test_db");
    }

    #[test]
    fn test_merge_raw_values_to_object() {
        let key1 = "name".to_string();
        let val1 = serde_json::value::RawValue::from_string("\"John\"".to_string()).unwrap();
        let key2 = "age".to_string();
        let val2 = serde_json::value::RawValue::from_string("30".to_string()).unwrap();

        let pairs = vec![(key1, val1), (key2, val2)];
        let result = merge_raw_values_to_object(&pairs);

        let parsed: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(parsed["name"], "John");
        assert_eq!(parsed["age"], 30);
    }

    #[test]
    fn test_merge_raw_values_to_object_empty() {
        let pairs: Vec<(String, Box<serde_json::value::RawValue>)> = vec![];
        let result = merge_raw_values_to_object(&pairs);

        let parsed: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(parsed, serde_json::json!({}));
    }

    #[test]
    fn test_merge_raw_values_to_object_special_chars() {
        let key1 = "key with spaces".to_string();
        let val1 = serde_json::value::RawValue::from_string("\"value\"".to_string()).unwrap();
        let key2 = "key\"with\"quotes".to_string();
        let val2 = serde_json::value::RawValue::from_string("42".to_string()).unwrap();

        let pairs = vec![(key1, val1), (key2, val2)];
        let result = merge_raw_values_to_object(&pairs);

        let parsed: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(parsed["key with spaces"], "value");
        assert_eq!(parsed["key\"with\"quotes"], 42);
    }

    #[test]
    fn test_merge_raw_values_to_array() {
        let val1 = serde_json::value::RawValue::from_string("1".to_string()).unwrap();
        let val2 = serde_json::value::RawValue::from_string("\"text\"".to_string()).unwrap();
        let val3 = serde_json::value::RawValue::from_string("true".to_string()).unwrap();

        let values = vec![val1, val2, val3];
        let result = merge_raw_values_to_array(&values);

        let parsed: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(parsed, serde_json::json!([1, "text", true]));
    }

    #[test]
    fn test_merge_raw_values_to_array_empty() {
        let values: Vec<Box<serde_json::value::RawValue>> = vec![];
        let result = merge_raw_values_to_array(&values);

        let parsed: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(parsed, serde_json::json!([]));
    }

    #[test]
    fn test_merge_raw_values_to_array_nested_objects() {
        let val1 = serde_json::value::RawValue::from_string("{\"a\":1}".to_string()).unwrap();
        let val2 = serde_json::value::RawValue::from_string("{\"b\":2}".to_string()).unwrap();

        let values = vec![val1, val2];
        let result = merge_raw_values_to_array(&values);

        let parsed: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(parsed, serde_json::json!([{"a": 1}, {"b": 2}]));
    }

    #[test]
    fn test_merge_nested_raw_values_to_array() {
        let val1 = serde_json::value::RawValue::from_string("1".to_string()).unwrap();
        let val2 = serde_json::value::RawValue::from_string("2".to_string()).unwrap();
        let val3 = serde_json::value::RawValue::from_string("3".to_string()).unwrap();
        let val4 = serde_json::value::RawValue::from_string("4".to_string()).unwrap();

        let inner1 = vec![val1, val2];
        let inner2 = vec![val3, val4];
        let nested = vec![inner1.iter(), inner2.iter()];

        let result = merge_nested_raw_values_to_array(nested.into_iter());

        let parsed: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(parsed, serde_json::json!([[1, 2], [3, 4]]));
    }

    #[test]
    fn test_merge_nested_raw_values_to_array_empty_outer() {
        let nested: Vec<std::slice::Iter<Box<serde_json::value::RawValue>>> = vec![];
        let result = merge_nested_raw_values_to_array(nested.into_iter());

        let parsed: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(parsed, serde_json::json!([]));
    }

    #[test]
    fn test_merge_nested_raw_values_to_array_empty_inner() {
        let inner1: Vec<Box<serde_json::value::RawValue>> = vec![];
        let val1 = serde_json::value::RawValue::from_string("1".to_string()).unwrap();
        let inner2 = vec![val1];
        let nested = vec![inner1.iter(), inner2.iter()];

        let result = merge_nested_raw_values_to_array(nested.into_iter());

        let parsed: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(parsed, serde_json::json!([[], [1]]));
    }

    #[test]
    fn test_merge_nested_raw_values_to_array_all_empty_inner() {
        let inner1: Vec<Box<serde_json::value::RawValue>> = vec![];
        let inner2: Vec<Box<serde_json::value::RawValue>> = vec![];
        let nested = vec![inner1.iter(), inner2.iter()];

        let result = merge_nested_raw_values_to_array(nested.into_iter());

        let parsed: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(parsed, serde_json::json!([[], []]));
    }

    #[test]
    fn test_merge_nested_raw_values_to_array_complex_types() {
        let val1 =
            serde_json::value::RawValue::from_string("{\"name\":\"Alice\"}".to_string()).unwrap();
        let val2 = serde_json::value::RawValue::from_string("[1,2,3]".to_string()).unwrap();
        let val3 = serde_json::value::RawValue::from_string("\"text\"".to_string()).unwrap();
        let val4 = serde_json::value::RawValue::from_string("null".to_string()).unwrap();

        let inner1 = vec![val1, val2];
        let inner2 = vec![val3, val4];
        let nested = vec![inner1.iter(), inner2.iter()];

        let result = merge_nested_raw_values_to_array(nested.into_iter());

        let parsed: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(
            parsed,
            serde_json::json!([[{"name": "Alice"}, [1, 2, 3]], ["text", null]])
        );
    }

    #[test]
    fn test_merge_nested_raw_values_to_array_single_inner() {
        let val1 = serde_json::value::RawValue::from_string("1".to_string()).unwrap();
        let val2 = serde_json::value::RawValue::from_string("2".to_string()).unwrap();
        let val3 = serde_json::value::RawValue::from_string("3".to_string()).unwrap();

        let inner1 = vec![val1, val2, val3];
        let nested = vec![inner1.iter()];

        let result = merge_nested_raw_values_to_array(nested.into_iter());

        let parsed: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(parsed, serde_json::json!([[1, 2, 3]]));
    }

    #[test]
    fn test_merge_nested_raw_values_to_array_many_inner() {
        let val1 = serde_json::value::RawValue::from_string("1".to_string()).unwrap();
        let val2 = serde_json::value::RawValue::from_string("2".to_string()).unwrap();
        let val3 = serde_json::value::RawValue::from_string("3".to_string()).unwrap();
        let val4 = serde_json::value::RawValue::from_string("4".to_string()).unwrap();
        let val5 = serde_json::value::RawValue::from_string("5".to_string()).unwrap();

        let inner1 = vec![val1];
        let inner2 = vec![val2];
        let inner3 = vec![val3];
        let inner4 = vec![val4];
        let inner5 = vec![val5];
        let nested = vec![
            inner1.iter(),
            inner2.iter(),
            inner3.iter(),
            inner4.iter(),
            inner5.iter(),
        ];

        let result = merge_nested_raw_values_to_array(nested.into_iter());

        let parsed: serde_json::Value = serde_json::from_str(result.get()).unwrap();
        assert_eq!(parsed, serde_json::json!([[1], [2], [3], [4], [5]]));
    }
}

/// Parse .npmrc content to extract the default registry URL and its auth token.
/// Returns `Some((registry_url, Option<auth_token>))` if a default registry is found.
pub fn parse_npmrc_registry(npmrc_content: &str) -> Option<(String, Option<String>)> {
    let mut registry_url: Option<String> = None;
    let mut auth_tokens: Vec<(String, String)> = Vec::new();

    for line in npmrc_content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if let Some(url) = line.strip_prefix("registry=") {
            registry_url = Some(url.trim().to_string());
        }

        if line.starts_with("//") {
            if let Some((prefix, token)) = line.split_once(":_authToken=") {
                auth_tokens.push((prefix.to_string(), token.to_string()));
            }
        }
    }

    let url = registry_url?;
    let url_without_protocol = url.trim_start_matches("https:").trim_start_matches("http:");
    let url_prefix = url_without_protocol.trim_end_matches('/');

    let token = auth_tokens
        .iter()
        .find(|(prefix, _)| {
            let p = prefix.trim_end_matches('/');
            p == url_prefix
        })
        .map(|(_, token)| token.clone());

    Some((url, token))
}

#[cfg(test)]
mod npmrc_tests {
    use super::parse_npmrc_registry;

    #[test]
    fn test_parse_simple_registry() {
        let npmrc = "registry=https://registry.mycompany.com/\n//registry.mycompany.com/:_authToken=secret123\n";
        let result = parse_npmrc_registry(npmrc);
        assert_eq!(
            result,
            Some((
                "https://registry.mycompany.com/".to_string(),
                Some("secret123".to_string())
            ))
        );
    }

    #[test]
    fn test_parse_registry_without_auth() {
        let npmrc = "registry=https://registry.npmjs.org/\n";
        let result = parse_npmrc_registry(npmrc);
        assert_eq!(
            result,
            Some(("https://registry.npmjs.org/".to_string(), None))
        );
    }

    #[test]
    fn test_parse_scoped_only_no_default() {
        let npmrc =
            "@myorg:registry=https://registry.myorg.com/\n//registry.myorg.com/:_authToken=tok\n";
        let result = parse_npmrc_registry(npmrc);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_with_comments() {
        let npmrc = "# My registry\nregistry=https://r.example.com/\n; auth\n//r.example.com/:_authToken=tok\n";
        let result = parse_npmrc_registry(npmrc);
        assert_eq!(
            result,
            Some((
                "https://r.example.com/".to_string(),
                Some("tok".to_string())
            ))
        );
    }

    #[test]
    fn test_parse_empty_npmrc() {
        assert_eq!(parse_npmrc_registry(""), None);
        assert_eq!(parse_npmrc_registry("# just a comment"), None);
    }
}
