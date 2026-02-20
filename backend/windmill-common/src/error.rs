/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::panic::Location;

use axum::body::Body;
use axum::response::Response;
use axum::{response::IntoResponse, response::Json};

use hyper::StatusCode;
use sqlx::migrate::MigrateError;
use thiserror::Error;
use tokio::io;

pub type Result<T> = std::result::Result<T, Error>;
pub type JsonResult<T> = std::result::Result<Json<T>, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Bad gateway: {0}")]
    BadGateway(String),
    #[error("Bad config: {0}")]
    BadConfig(String),
    #[error("Connecting to database: {0}")]
    ConnectingToDatabase(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Not authorized: {0}")]
    NotAuthorized(String),
    #[error("Metric not found: {0}")]
    MetricNotFound(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Require Admin privileges for {0}")]
    RequireAdmin(String),
    #[error("{0}")]
    ExecutionErr(String),
    #[error("{0}")]
    ResultTooLarge(String),
    #[error("IoErr: {error:#} @{location:#}")]
    IoErr { error: io::Error, location: String },
    #[error("Utf8Err: {error:#} @{location:#}")]
    Utf8Err { error: std::string::FromUtf8Error, location: String },
    #[error("UuidErr: {error:#} @{location:#}")]
    UuidErr { error: uuid::Error, location: String },
    #[error("SqlErr: {error:#} @{location:#}")]
    SqlErr { error: sqlx::Error, location: String },
    #[error("SerdeJson: {error:#} @{location:#}")]
    SerdeJson { error: serde_json::Error, location: String },
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),
    #[error("Internal: {0}")]
    InternalErr(String),
    #[error("Internal: {message} @{location}")]
    InternalErrLoc { message: String, location: String },
    #[error("Internal: {0}: {1}")]
    InternalErrAt(&'static Location<'static>, String),
    #[error("HexErr: {error:#} @{location:#}")]
    HexErr { error: hex::FromHexError, location: String },
    #[error("Migrating database: {0}")]
    DatabaseMigration(#[from] MigrateError),
    #[error("Non-zero exit status for {0}: {1}")]
    ExitStatus(String, i32),
    #[error("ExecutionRawError: {0}")]
    ExecutionRawError(Box<serde_json::value::RawValue>),
    #[error("Error: {error:#} @{location:#}")]
    Anyhow { error: anyhow::Error, location: String },
    #[error("Error: {0:#?}")]
    JsonErr(serde_json::Value),
    #[error("{0}")]
    AIError(String),
    #[error("{0}")]
    AlreadyCompleted(String),
    #[error("Find python error: {0}")]
    FindPythonError(String),
    #[error("Problem with arguments: {0}")]
    ArgumentErr(String),
    #[error("{1}")]
    Generic(StatusCode, String),
    #[error("{feature} is unavailable due to some workers being behind. Do not use the feature or make sure all workers run at least {min_version}")]
    WorkersAreBehind { feature: String, min_version: String },
    #[error(
        "Breaking change was introduced in v{version} ({feature}). Follow this migration guide: {guide_url}"
    )]
    MigrationNeeded { version: String, feature: String, guide_url: url::Url },
    #[error("{0} is unavailable. It is possible for this worker to be behind.")]
    FeatureUnavailable(String),
}

impl Error {
    pub fn name(&self) -> &str {
        match self {
            Self::ExecutionErr(_) => "ExecutionErr",
            Self::ResultTooLarge(_) => "ResultTooLarge",
            Self::BadRequest(_) => "BadRequest",
            Self::QuotaExceeded(_) => "QuotaExceeded",
            Self::InternalErr(_) => "InternalErr",
            Self::InternalErrLoc { .. } => "InternalErr",
            Self::InternalErrAt(_, _) => "InternalErr",
            Self::Anyhow { .. } => "Anyhow",
            Self::JsonErr(_) => "JsonErr",
            Self::AIError(_) => "AIError",
            Self::AlreadyCompleted(_) => "AlreadyCompleted",
            Self::FindPythonError(_) => "FindPythonError",
            Self::ArgumentErr(_) => "ArgumentErr",
            Self::Generic(_, _) => "Generic",
            Self::IoErr { .. } => "IoErr",
            Self::Utf8Err { .. } => "Utf8Err",
            Self::UuidErr { .. } => "UuidErr",
            Self::SqlErr { .. } => "SqlErr",
            Self::SerdeJson { .. } => "SerdeJson",
            Self::HexErr { .. } => "HexErr",
            Self::DatabaseMigration(_) => "DatabaseMigration",
            Self::ExitStatus(_, _) => "ExitStatus",
            Self::ExecutionRawError(_) => "ExecutionRawError",
            Self::BadGateway(_) => "BadGateway",
            Self::BadConfig(_) => "BadConfig",
            Self::ConnectingToDatabase(_) => "ConnectingToDatabase",
            Self::NotFound(_) => "NotFound",
            Self::NotAuthorized(_) => "NotAuthorized",
            Self::MetricNotFound(_) => "MetricNotFound",
            Self::PermissionDenied(_) => "PermissionDenied",
            _ => "InternalErr",
        }
    }
}

fn prettify_location(location: &'static Location<'static>) -> String {
    location
        .to_string()
        .split("/")
        .last()
        .unwrap_or("unknown")
        .to_string()
}
impl From<anyhow::Error> for Error {
    #[track_caller]
    fn from(e: anyhow::Error) -> Self {
        Self::Anyhow { error: e, location: prettify_location(std::panic::Location::caller()) }
    }
}

impl From<sqlx::Error> for Error {
    #[track_caller]
    fn from(e: sqlx::Error) -> Self {
        Self::SqlErr { error: e, location: prettify_location(std::panic::Location::caller()) }
    }
}

impl From<uuid::Error> for Error {
    #[track_caller]
    fn from(e: uuid::Error) -> Self {
        Self::UuidErr { error: e, location: prettify_location(std::panic::Location::caller()) }
    }
}

impl From<std::string::FromUtf8Error> for Error {
    #[track_caller]
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self::Utf8Err { error: e, location: prettify_location(std::panic::Location::caller()) }
    }
}

impl From<io::Error> for Error {
    #[track_caller]
    fn from(e: io::Error) -> Self {
        Self::IoErr { error: e, location: prettify_location(std::panic::Location::caller()) }
    }
}

impl From<hex::FromHexError> for Error {
    #[track_caller]
    fn from(e: hex::FromHexError) -> Self {
        Self::HexErr { error: e, location: prettify_location(std::panic::Location::caller()) }
    }
}

impl From<serde_json::Error> for Error {
    #[track_caller]
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeJson { error: e, location: prettify_location(std::panic::Location::caller()) }
    }
}

impl From<url::ParseError> for Error {
    #[track_caller]
    fn from(e: url::ParseError) -> Self {
        Self::ArgumentErr(format!("Cannot parse provided url. \ne: {e}"))
    }
}

impl From<tokio::time::error::Elapsed> for Error {
    fn from(value: tokio::time::error::Elapsed) -> Self {
        Self::InternalErr(value.to_string())
    }
}

impl From<semver::Error> for Error {
    fn from(e: semver::Error) -> Self {
        Self::ArgumentErr(format!("Cannot parse provided semver: {e}"))
    }
}

impl Error {
    /// https://docs.rs/anyhow/1/anyhow/struct.Error.html#display-representations
    pub fn alt(&self) -> String {
        format!("{:#}", self)
    }

    pub fn dbg(&self) -> String {
        format!("{:?}", self)
    }

    pub fn relocate_internal(self, loc: &'static Location<'static>) -> Self {
        match self {
            Self::InternalErrLoc { message, .. }
            | Self::InternalErrAt(_, message)
            | Self::InternalErr(message) => Self::InternalErrAt(loc, message),
            _ => self,
        }
    }

    #[track_caller]
    pub fn internal_err<T: AsRef<str>>(msg: T) -> Self {
        Self::InternalErrLoc {
            message: msg.as_ref().to_string(),
            location: prettify_location(std::panic::Location::caller()),
        }
    }
}

pub fn relocate_internal(loc: &'static Location<'static>) -> impl FnOnce(Error) -> Error {
    move |e| e.relocate_internal(loc)
}

pub fn to_anyhow<T: 'static + std::error::Error + Send + Sync>(e: T) -> anyhow::Error {
    From::from(e)
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            Self::NotFound(_) => axum::http::StatusCode::NOT_FOUND,
            Self::NotAuthorized(_) => axum::http::StatusCode::UNAUTHORIZED,
            Self::RequireAdmin(_) | Self::PermissionDenied(_) => axum::http::StatusCode::FORBIDDEN,
            Self::SqlErr { .. }
            | Self::BadRequest(_)
            | Self::AIError(_)
            | Self::QuotaExceeded(_) => axum::http::StatusCode::BAD_REQUEST,
            Self::BadGateway(_) => axum::http::StatusCode::BAD_GATEWAY,
            Self::Generic(status_code, _) => status_code,
            _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        };

        let e = &self;

        if matches!(status, axum::http::StatusCode::NOT_FOUND) {
            tracing::warn!(message = e.to_string());
        } else {
            tracing::error!(message = e.to_string(), error = ?e);
        };

        let body = Body::from(e.to_string());

        axum::response::Response::builder()
            .header("Content-Type", "text/plain")
            .status(status)
            .body(body)
            .unwrap()
    }
}

pub trait OrElseNotFound<T> {
    fn or_else_not_found(self, s: impl ToString) -> Result<T>;
}

impl<T> OrElseNotFound<T> for Option<T> {
    fn or_else_not_found(self, s: impl ToString) -> Result<T> {
        self.ok_or_else(|| Error::NotFound(s.to_string()))
    }
}

// Make our own error that wraps `anyhow::Error`.
pub struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Body::from(self.0.to_string());
        tracing::error!(error = self.0.to_string());
        axum::response::Response::builder()
            .header("Content-Type", "text/plain")
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(body)
            .unwrap()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
