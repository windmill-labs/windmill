#![allow(non_snake_case)]

use axum::headers::HeaderValue;
use axum::http::{header, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use bytes::{BufMut, BytesMut};
use mime_guess::mime;
use serde::{Deserialize, Serialize};
use serde_json::json;

lazy_static::lazy_static! {
    static ref SCIM_TOKEN: Option<String> = std::env::var("SCIM_TOKEN")
        .ok();
}

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonScim<T>(pub T);

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

pub fn resource_response<S>(schema: &str, resources: Vec<S>) -> JsonScim<serde_json::Value>
where
    S: Serialize,
{
    return JsonScim(json!({
        "schemas": [schema],
        "totalResults": resources.len(),
        "Resources": resources,
        "startIndex": 1,
        "itemsPerPage": 100,
    }));
}

#[derive(Serialize, Debug)]
pub struct ScimUser {
    pub id: String,
    pub userName: String,
    pub active: bool,
}

#[derive(Deserialize)]
pub struct ScimQuery {
    pub startIndex: Option<u32>,
    pub count: Option<u32>,
    pub filter: Option<String>,
}

pub async fn has_scim_token<B>(request: Request<B>, next: Next<B>) -> Response {
    let header = request.headers().get("Authorization");
    if let Some(header) = header {
        if let Ok(header) = header.to_str() {
            if header.starts_with("Bearer ") {
                let token = header.trim_start_matches("Bearer ");
                if let Some(scim_token) = crate::scim_helpers::SCIM_TOKEN.as_ref() {
                    if token == scim_token {
                        return next.run(request).await;
                    }
                }
            }
        }
    }
    return (
        StatusCode::UNAUTHORIZED,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
        )],
        "Unauthorized",
    )
        .into_response();
}
