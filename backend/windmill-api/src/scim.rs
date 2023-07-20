/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use bytes::{BufMut, BytesMut};
use hyper::{header, http::HeaderValue, Request, StatusCode};
use mime_guess::mime;
use serde::Serialize;
use windmill_common::error::{Error, Result};

lazy_static::lazy_static! {
    static ref SCIM_PASSWORD: Option<String> = std::env::var("SCIM_PASSWORD")
        .ok();
}

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonScim<T>(pub T);

pub async fn has_basic_auth<B>(request: Request<B>, next: Next<B>) -> Response {
    let header = request.headers().get("Authorization");
    next.run(request).await
}

pub type JsonScimResult<T> = std::result::Result<JsonScim<T>, Error>;

impl<T> IntoResponse for JsonScim<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        // Use a small initial capacity of 128 bytes like serde_json::to_vec
        // https://docs.rs/serde_json/1.0.82/src/serde_json/ser.rs.html#2189
        let mut buf = BytesMut::with_capacity(128).writer();
        match serde_json::to_writer(&mut buf, &self.0) {
            Ok(()) => (
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/scim+json"),
                )],
                buf.into_inner().freeze(),
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
                )],
                err.to_string(),
            )
                .into_response(),
        }
    }
}

pub fn global_service() -> Router {
    Router::new()
        .route("/authorize", post(authorize))
        .route("/Users", get(get_users))
}

pub async fn authorize() -> Result<String> {
    Ok("Hello, World!".to_string())
}

pub async fn get_users() -> JsonScimResult<String> {
    Ok(JsonScim("Hello, World!".to_string()))
}
