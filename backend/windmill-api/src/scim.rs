/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::{Path, Query},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Router,
};
use bytes::{BufMut, BytesMut};
use hyper::{header, http::HeaderValue, Request, StatusCode};
use mime_guess::mime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sql_builder::SqlBuilder;
use windmill_common::error::{Error, Result};

use crate::db::DB;

lazy_static::lazy_static! {
    static ref SCIM_TOKEN: Option<String> = std::env::var("SCIM_TOKEN")
        .ok();
}

pub fn global_service() -> Router {
    Router::new().route("/Users", get(get_users))
    // .route("/Groups", get(get_groups).post(create_group))
    // .route("/Groups/:id", get(get_group))
}

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonScim<T>(pub T);

pub async fn has_scim_token<B>(request: Request<B>, next: Next<B>) -> Response {
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

#[derive(Serialize)]
struct User {
    id: String,
    userName: String,
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

#[derive(Deserialize)]
pub struct ScimQuery {
    startIndex: Option<u32>,
    count: Option<u32>,
}

pub async fn get_users(
    Extension(db): Extension<DB>,
    Query(query): Query<ScimQuery>,
) -> Result<JsonScim<serde_json::Value>> {
    let sqlb = SqlBuilder::select_from("usr")
        .fields(&["email"])
        .limit(query.count.unwrap_or(100000))
        .offset(query.startIndex.map(|x| x - 1).unwrap_or(0))
        .clone();

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let users = sqlx::query_scalar(&sql)
        .fetch_all(&db)
        .await?
        .into_iter()
        .map(|x: String| User { id: x.clone(), userName: x })
        .collect();
    Ok(resource_response(
        "urn:ietf:params:scim:api:messages:2.0:ListResponse",
        users,
    ))
}

pub async fn get_groups(
    Extension(db): Extension<DB>,
    Query(query): Query<ScimQuery>,
) -> Result<JsonScim<serde_json::Value>> {
    let sqlb = SqlBuilder::select_from("instance_group")
        .fields(&["email"])
        .limit(query.count.unwrap_or(100000))
        .offset(query.startIndex.map(|x| x - 1).unwrap_or(0))
        .clone();

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let users = sqlx::query_scalar(&sql)
        .fetch_all(&db)
        .await?
        .into_iter()
        .map(|x: String| User { id: x.clone(), userName: x })
        .collect();
    Ok(resource_response(
        "urn:ietf:params:scim:api:messages:2.0:ListResponse",
        users,
    ))
}

// pub async fn get_group(
//     Extension(db): Extension<DB>,
//     Query(query): Query<ScimQuery>,
//     Path(id): Path<String>,
// ) -> Result<JsonScim<serde_json::Value>> {
//     let groups = sqlx::query_as!(
//         IGroup,
//         "SELECT igroup as name, array_agg(email_to_igroup.email) as emails FROM email_to_igroup GROUP BY igroup"
//     )
//     .fetch_all(&mut *tx)
//     .await?;
//     Ok(resource_response(
//         "urn:ietf:params:scim:api:messages:2.0:ListResponse",
//         groups,
//     ))
// }
// pub async fn create_group(
//     Extension(db): Extension<DB>,
//     Query(query): Query<ScimQuery>,
// ) -> Result<JsonScim<serde_json::Value>> {
// }
