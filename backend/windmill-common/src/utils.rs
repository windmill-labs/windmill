/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::ee::LICENSE_KEY_ID;
#[cfg(feature = "enterprise")]
use crate::ee::{send_critical_alert, CriticalAlertKind};
use crate::error::{to_anyhow, Error, Result};
use crate::global_settings::UNIQUE_ID_SETTING;
use crate::server::Smtp;
use crate::DB;
use anyhow::Context;
use gethostname::gethostname;
use git_version::git_version;
use mail_send::mail_builder::MessageBuilder;
use mail_send::SmtpClientBuilder;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Postgres};

pub const MAX_PER_PAGE: usize = 10000;
pub const DEFAULT_PER_PAGE: usize = 1000;

pub const GIT_VERSION: &str =
    git_version!(args = ["--tag", "--always"], fallback = "unknown-version");

lazy_static::lazy_static! {
    pub static ref HTTP_CLIENT: Client = reqwest::ClientBuilder::new()
        .user_agent("windmill/beta")
        .timeout(std::time::Duration::from_secs(20))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build().unwrap();
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

pub fn hostname() -> String {
    gethostname()
        .to_str()
        .map(|x| x.to_string())
        .unwrap_or_else(|| rd_string(5))
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
    thread_rng()
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

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Worker,
    Agent,
    Server,
    Standalone,
    Indexer,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Worker => write!(f, "worker"),
            Mode::Agent => write!(f, "agent"),
            Mode::Server => write!(f, "server"),
            Mode::Standalone => write!(f, "standalone"),
            Mode::Indexer => write!(f, "indexer"),
        }
    }
}

// inspired from rails: https://github.com/rails/rails/blob/6e49cc77ab3d16c06e12f93158eaf3e507d4120e/activerecord/lib/active_record/migration.rb#L1308
pub fn generate_lock_id(database_name: &str) -> i64 {
    const CRC_IEEE: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
    // 0x3d32ad9e chosen by fair dice roll
    0x3d32ad9e * (CRC_IEEE.checksum(database_name.as_bytes()) as i64)
}

pub async fn send_email(
    subject: &str,
    content: &str,
    to: Vec<String>,
    smtp: Smtp,
    client_timeout: Option<tokio::time::Duration>,
) -> Result<()> {
    let mut client = SmtpClientBuilder::new(smtp.host, smtp.port)
        .implicit_tls(smtp.tls_implicit.unwrap_or(false));
    if std::env::var("ACCEPT_INVALID_CERTS").is_ok() {
        client = client.allow_invalid_certs();
    }
    let client = if let (Some(username), Some(password)) = (smtp.username, smtp.password) {
        if !username.is_empty() {
            client.credentials((username, password))
        } else {
            client
        }
    } else {
        client
    };
    let message = MessageBuilder::new()
        .from(("Windmill", smtp.from.as_str()))
        .to(to.clone())
        .subject(subject)
        .text_body(content);

    match client_timeout {
        Some(timeout) => {
            tokio::time::timeout(timeout, client.connect())
                .await
                .map_err(to_anyhow)?
                .map_err(to_anyhow)?
                .send(message)
                .await
                .map_err(to_anyhow)?;
        }
        None => {
            client
                .connect()
                .await
                .map_err(to_anyhow)?
                .send(message)
                .await
                .map_err(to_anyhow)?;
        }
    }
    tracing::info!("Sent email to {:#?}: {subject}", to);

    return Ok(());
}

pub async fn report_critical_error(error_message: String, _db: DB) -> () {
    tracing::error!("CRITICAL ERROR: {error_message}");

    if let Err(err) = sqlx::query!(
        "INSERT INTO alerts (alert_type, message) VALUES ('critical_error', $1)",
        error_message
    )
    .execute(&_db)
    .await
    {
        tracing::error!("Failed to save critical error to database: {}", err);
    }

    #[cfg(feature = "enterprise")]
    send_critical_alert(error_message, &_db, CriticalAlertKind::CriticalError, None).await;
}

pub async fn report_recovered_critical_error(message: String, _db: DB) -> () {
    tracing::info!("RECOVERED CRITICAL ERROR: {message}");

    if let Err(err) = sqlx::query!(
        "INSERT INTO alerts (alert_type, message) VALUES ('recovered_critical_error', $1)",
        message
    )
    .execute(&_db)
    .await
    {
        tracing::error!("Failed to save critical error to database: {}", err);
    }
    #[cfg(feature = "enterprise")]
    send_critical_alert(
        message,
        &_db,
        CriticalAlertKind::RecoveredCriticalError,
        None,
    )
    .await;
}
