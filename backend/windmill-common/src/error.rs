/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::response::Response;
#[cfg(feature = "axum")]
use axum::{
    body::{self, BoxBody},
    response::IntoResponse,
    response::Json,
};

use hyper::StatusCode;
#[cfg(feature = "sqlx")]
use sqlx::migrate::MigrateError;
use thiserror::Error;
#[cfg(feature = "tokio")]
use tokio::io;

pub type Result<T> = std::result::Result<T, Error>;
#[cfg(feature = "axum")]
pub type JsonResult<T> = std::result::Result<Json<T>, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Uuid Error {0}")]
    UuidErr(#[from] uuid::Error),
    #[error("Bad config: {0}")]
    BadConfig(String),
    #[error("Connecting to database: {0}")]
    ConnectingToDatabase(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Not authorized: {0}")]
    NotAuthorized(String),
    #[error("Require Admin privileges for {0}")]
    RequireAdmin(String),
    #[error("{0}")]
    ExecutionErr(String),
    #[error("IO error: {0}")]
    #[cfg(feature = "tokio")]
    IoErr(#[from] io::Error),
    #[error("Sql error: {0}")]
    #[cfg(feature = "sqlx")]
    SqlErr(#[from] sqlx::Error),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal: {0}")]
    InternalErr(String),
    #[error("Hexadecimal decoding error: {0}")]
    HexErr(#[from] hex::FromHexError),
    #[error("Migrating database: {0}")]
    #[cfg(feature = "sqlx")]
    DatabaseMigration(#[from] MigrateError),
    #[error("Non-zero exit status: {0}")]
    ExitStatus(i32),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error("Error: {0:#?}")]
    JsonErr(serde_json::Value),
    #[error("Custom Status Code: {0:#?}")]
    CustomStatusCode(StatusCode, serde_json::Value),
    #[error("{0}")]
    OpenAIError(String),
}

impl Error {
    /// https://docs.rs/anyhow/1/anyhow/struct.Error.html#display-representations
    pub fn alt(&self) -> String {
        format!("{:#}", self)
    }
}

pub fn to_anyhow<T: 'static + std::error::Error + Send + Sync>(e: T) -> anyhow::Error {
    From::from(e)
}

#[cfg(feature = "axum")]
impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response<BoxBody> {
        let e = &self;
        let body = body::boxed(body::Full::from(e.to_string()));
        let status = match self {
            Self::NotFound(_) => axum::http::StatusCode::NOT_FOUND,
            Self::NotAuthorized(_) => axum::http::StatusCode::UNAUTHORIZED,
            Self::RequireAdmin(_) => axum::http::StatusCode::FORBIDDEN,
            Self::CustomStatusCode(code, _) => code,
            Self::SqlErr(_) | Self::BadRequest(_) | Self::OpenAIError(_) => {
                axum::http::StatusCode::BAD_REQUEST
            }
            _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        };
        tracing::error!(error = e.to_string());
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
        let body = body::boxed(body::Full::from(self.0.to_string()));
        tracing::error!(error = self.0.to_string());
        axum::response::Response::builder()
            .header("Content-Type", "text/plain")
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(body)
            .unwrap()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
