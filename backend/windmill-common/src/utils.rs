/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::auth::is_devops_email;
use crate::ee::LICENSE_KEY_ID;
#[cfg(feature = "enterprise")]
use crate::ee::{send_critical_alert, CriticalAlertKind};
use crate::error::{to_anyhow, Error, Result};
use crate::global_settings::UNIQUE_ID_SETTING;
use crate::DB;
use anyhow::Context;
use gethostname::gethostname;
use git_version::git_version;

use chrono::Utc;
use croner::Cron;
use rand::{distr::Alphanumeric, rng, Rng};
use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Postgres};
use std::{fs::DirBuilder as SyncDirBuilder, str::FromStr};
use tokio::fs::DirBuilder as AsyncDirBuilder;

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
    pub static ref HTTP_CLIENT: Client = reqwest::ClientBuilder::new()
        .user_agent("windmill/beta")
        .timeout(std::time::Duration::from_secs(20))
        .connect_timeout(std::time::Duration::from_secs(10))
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
                println!("Binary is in 'agent' mode");
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
        ModeAndAddons {
            indexer: search_addon,
            mode,
        }
    };
}

lazy_static::lazy_static! {
    pub static ref AGENT_TOKEN: String = std::env::var("AGENT_TOKEN").unwrap_or_default();
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

pub fn require_admin(is_admin: bool, username: &str) -> Result<()> {
    if !is_admin {
        Err(Error::RequireAdmin(username.to_string()))
    } else {
        Ok(())
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

pub fn worker_suffix(hostname: &str, rd_string: &str) -> String {
    format!("{}-{}", instance_name(hostname), rd_string)
}

pub fn worker_name_with_suffix(is_agent: bool, worker_group: &str, suffix: &str) -> String {
    if is_agent {
        format!("{}-{}-{}", AGENT_WORKER_NAME_PREFIX, worker_group, suffix)
    } else {
        format!("{}-{}-{}", WORKER_NAME_PREFIX, worker_group, suffix)
    }
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

pub fn not_found_if_none<T, U: AsRef<str>>(opt: Option<T>, kind: &str, name: U) -> Result<T> {
    if let Some(o) = opt {
        Ok(o)
    } else {
        Err(Error::NotFound(format!(
            "{} not found at name {}",
            kind,
            name.as_ref()
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
        Some(db) => match get_uid(db).await {
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

pub async fn get_uid<'c, E: sqlx::Executor<'c, Database = Postgres>>(db: E) -> Result<String> {
    let mut uid = LICENSE_KEY_ID.read().await.clone();

    if uid == "" {
        let uid_value = sqlx::query_scalar!(
            "SELECT value FROM global_settings WHERE name = $1",
            UNIQUE_ID_SETTING
        )
        .fetch_one(db)
        .await?;

        uid = serde_json::from_value::<String>(uid_value).map_err(to_anyhow)?;
    }

    Ok(uid)
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

pub fn empty_as_none<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + IsEmpty,
{
    let option = <Option<T> as serde::Deserialize>::deserialize(deserializer)?;
    Ok(option.filter(|s| !s.is_empty()))
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
        let location = format!("{}:{}", caller.file(), caller.line());
        WarnAfterFuture {
            future: self,
            timeout: time::sleep(Duration::from_secs(seconds as u64)),
            warned: false,
            start_time: std::time::Instant::now(),
            location: location,
            seconds,
        }
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
    }
}

impl<F: Future> Future for WarnAfterFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut TContext<'_>) -> Poll<Self::Output> {
        let this = self.project();

        // Poll the timeout future to check if it has elapsed.
        if !*this.warned {
            if this.timeout.poll(cx).is_ready() {
                tracing::warn!(
                    location = this.location,
                    "SLOW_QUERY: query {} to db taking longer than expected (> {} seconds)",
                    this.location,
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
                        this.location,
                        elapsed
                    );
                }
                Poll::Ready(output)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
