/*
* Author & Copyright: Ruben Fiszel 2021
 * This file and its contents are licensed under the AGPL License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    body::{self, BoxBody},
    response::IntoResponse,
    Json,
};
use hyper::{Response, StatusCode};
use sqlx::migrate::MigrateError;
use thiserror::Error;
use tokio::io;

pub type Result<T> = std::result::Result<T, Error>;
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
    #[error("{0}")]
    ExecutionErr(String),
    #[error("IO error: {0}")]
    IoErr(#[from] io::Error),
    #[error("Sql error: {0}")]
    SqlErr(#[from] sqlx::Error),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal: {0}")]
    InternalErr(String),
    #[error("Hexadecimal decoding error: {0}")]
    HexErr(#[from] hex::FromHexError),
    #[error("Migrating database: {0}")]
    DatabaseMigration(#[from] MigrateError),
    #[error("{0}")]
    Anyhow(#[from] anyhow::Error),
}

pub fn to_anyhow<T: 'static + std::error::Error + Send + Sync>(e: T) -> anyhow::Error {
    From::from(e)
}

impl IntoResponse for Error {
    fn into_response(self) -> Response<BoxBody> {
        let e = &self;
        let body = body::boxed(body::Full::from(e.to_string()));
        let status = match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::NotAuthorized(_) => StatusCode::UNAUTHORIZED,
            Self::SqlErr(_) | Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Response::builder().status(status).body(body).unwrap()
    }
}
