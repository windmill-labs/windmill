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
use std::sync::Arc;
use std::{fs::DirBuilder as SyncDirBuilder, str::FromStr};
use tokio::fs::DirBuilder as AsyncDirBuilder;
use tokio::sync::RwLock;
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
use std::sync::atomic::Ordering;

use crate::worker::CLOUD_HOSTED;

lazy_static::lazy_static! {
    pub static ref COOKIE_DOMAIN: Option<String> = std::env::var("COOKIE_DOMAIN").ok();
    pub static ref IS_SECURE: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));

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

    pub static ref HUB_API_SECRET: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));
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
    db: &DB,
) -> Result<()> {
    if !is_admin {
        if !is_devops_email(db, email).await? {
            return Err(Error::RequireAdmin(username.to_string()));
        }
    }
    Ok(())
}

fn instance_name(hostname: &str) -> String {
    hostname
        .replace(" ", "")
        .split("-")
        .last()
        .unwrap()
        .to_ascii_lowercase()
        .to_string()
}

const DEFAULT_WORKER_SUFFIX_LEN: usize = 5;
pub const SSH_AGENT_WORKER_SUFFIX: &'static str = "/ssh";

pub fn create_worker_suffix(hostname: &str, rd_string_len: usize) -> String {
    let wk_suffix = format!("{}-{}", instance_name(hostname), rd_string(rd_string_len));
    wk_suffix
}

pub fn create_default_worker_suffix(hostname: &str) -> String {
    create_worker_suffix(hostname, DEFAULT_WORKER_SUFFIX_LEN)
}

pub fn worker_name_with_suffix(is_agent: bool, worker_group: &str, suffix: &str) -> String {
    if is_agent {
        format!("{}-{}-{}", AGENT_WORKER_NAME_PREFIX, worker_group, suffix)
    } else {
        format!("{}-{}-{}", WORKER_NAME_PREFIX, worker_group, suffix)
    }
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
        .expect("could not create dir");
}

pub fn create_directory_sync(directory_path: &str) {
    SyncDirBuilder::new()
        .recursive(true)
        .create(directory_path)
        .expect("could not create dir");
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

    if let Some(hub_api_secret) = HUB_API_SECRET.read().await.clone() {
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
    let license_id = LICENSE_KEY_ID.read().await.clone();

    if license_id.is_empty() {
        get_instance_uid(db).await
    } else {
        Ok(license_id)
    }
}

async fn get_instance_uid<'c, E: sqlx::Executor<'c, Database = Postgres>>(db: E) -> Result<String> {
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
    let license_id = LICENSE_KEY_ID.read().await.clone();
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
                        Error::BadRequest(format!("cron: {}", e))
                    })
            }
            Some("v2") | Some(_) => {
                // Use Croner for v2
                let schedule_type_result = panic::catch_unwind(AssertUnwindSafe(|| {
                    let mut croner = Cron::new(schedule_str);
                    if seconds_required {
                        croner.with_seconds_required();
                    } else {
                        croner.with_seconds_optional();
                    };
                    croner.parse()
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
                        Error::BadRequest(format!("cron: {}", e))
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

#[cfg(test)]
mod tests {
    use super::*;
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
