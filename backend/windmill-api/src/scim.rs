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
    Extension, Json, Router,
};
use bytes::{BufMut, BytesMut};
use hyper::{header, http::HeaderValue, Request, StatusCode};
use mime_guess::mime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sql_builder::SqlBuilder;
use windmill_common::{
    error::{Error, Result},
    utils::not_found_if_none,
};

use crate::db::DB;

lazy_static::lazy_static! {
    static ref SCIM_TOKEN: Option<String> = std::env::var("SCIM_TOKEN")
        .ok();
}

#[cfg(feature = "enterprise")]
pub fn global_service() -> Router {
    Router::new()
        .route("/Users", get(get_users).post(create_user))
        .route("/Groups", get(get_groups).post(create_group))
        .route(
            "/Groups/:id",
            get(get_group)
                .put(update_group)
                .patch(update_group)
                .delete(delete_group),
        )
}

#[cfg(not(feature = "enterprise"))]
pub fn global_service() -> Router {
    Router::new().route("/Users", get(get_users))
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

#[derive(Serialize, Debug)]
struct User {
    id: String,
    userName: String,
    active: bool,
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
    filter: Option<String>,
}

pub async fn get_users(
    Extension(db): Extension<DB>,
    Query(query): Query<ScimQuery>,
) -> Result<JsonScim<serde_json::Value>> {
    let mut sqlb = SqlBuilder::select_from("usr")
        .fields(&["email"])
        .limit(query.count.unwrap_or(100000))
        .offset(query.startIndex.map(|x| x - 1).unwrap_or(0))
        .clone();

    tracing::info!("SCIM filter: {:?}", query.filter);

    if let Some(filter) = query.filter {
        let filter = filter
            .replace("userName", "email")
            .replace("eq", "=")
            .replace("\"", "'");
        sqlb.and_where(&filter);
    }

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let users = sqlx::query_scalar(&sql)
        .fetch_all(&db)
        .await?
        .into_iter()
        .map(|x: String| User { id: x.clone(), userName: x, active: true })
        .collect();
    tracing::info!("SCIM users: {:?}", users);
    Ok(resource_response(
        "urn:ietf:params:scim:api:messages:2.0:ListResponse",
        users,
    ))
}

#[derive(Deserialize, Debug)]
pub struct CreateUser {
    userName: String,
}
// #[cfg(feature = "enterprise")]
pub async fn create_user(
    Extension(db): Extension<DB>,
    Json(body): Json<CreateUser>,
) -> Result<JsonScim<serde_json::Value>> {
    tracing::info!("SCIM creating user: {:?}", body);
    sqlx::query!(
        "INSERT INTO password (email, login_type, verified) VALUES ($1, 'saml', true) ON CONFLICT DO NOTHING",
        body.userName,
    ).execute(&db).await?;
    Ok(JsonScim(json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": body.userName,
        "userName": body.userName,
        "active": true
    })))
}

#[cfg(feature = "enterprise")]
pub async fn get_groups(
    Extension(db): Extension<DB>,
    Query(query): Query<ScimQuery>,
) -> Result<JsonScim<serde_json::Value>> {
    let sqlb = SqlBuilder::select_from("instance_group")
        .fields(&["name"])
        .limit(query.count.unwrap_or(100000))
        .offset(query.startIndex.map(|x| x - 1).unwrap_or(0))
        .clone();

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let users = sqlx::query_scalar(&sql)
        .fetch_all(&db)
        .await?
        .into_iter()
        .map(|x: String| User { id: x.clone(), userName: x, active: true })
        .collect();
    Ok(resource_response(
        "urn:ietf:params:scim:api:messages:2.0:ListResponse",
        users,
    ))
}

// {
//     "schemas": [],
//     "id": "abf4dd94-a4c0-4f67-89c9-76b03340cb9b",
//     "displayName": "Test SCIMv2",
//     "members": [],
//     "meta": {
//         "resourceType": "Group"
//     }
// }

#[cfg(feature = "enterprise")]
#[derive(Serialize)]
struct Group {
    name: String,
    emails: Option<Vec<String>>,
}
#[cfg(feature = "enterprise")]
fn group_response(group: Group) -> JsonScim<serde_json::Value> {
    return JsonScim(json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": group.name,
        "id": convert_name(&group.name),
        "members": group.emails,
        "meta": {
            "resourceType": "Group"
        }
    }));
}

#[cfg(feature = "enterprise")]
pub async fn get_group(
    Extension(db): Extension<DB>,
    Path(id): Path<String>,
) -> Result<JsonScim<serde_json::Value>> {
    let group= sqlx::query_as!(
        Group,
        "SELECT name, array_remove(array_agg(email_to_igroup.email), null) as emails FROM email_to_igroup RIGHT JOIN instance_group ON instance_group.name = email_to_igroup.igroup WHERE name = $1 GROUP BY name",
        id
    )
    .fetch_optional(&db)
    .await?;
    let group = not_found_if_none(group, "Group", id)?;
    Ok(group_response(group))
}

// {
//     "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
//     "displayName": "Test SCIMv2",
//     "members": []
// }

#[cfg(feature = "enterprise")]
#[derive(Deserialize, Debug)]
pub struct CreateGroup {
    pub displayName: String,
    pub members: Vec<Member>,
}

#[cfg(feature = "enterprise")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Member {
    pub value: String,
    pub display: String,
}

#[cfg(feature = "enterprise")]
pub async fn create_group(
    Extension(db): Extension<DB>,
    Json(body): Json<CreateGroup>,
) -> Result<JsonScim<serde_json::Value>> {
    tracing::info!("SCIM creating group: {:?}", body);
    let mut tx: sqlx::Transaction<'_, sqlx::Postgres> = db.begin().await?;
    sqlx::query!(
        "INSERT INTO instance_group (name) VALUES ($1) ON CONFLICT DO NOTHING",
        convert_name(&body.displayName)
    )
    .execute(&mut *tx)
    .await?;
    for member in &body.members {
        sqlx::query!(
            "INSERT INTO email_to_igroup (email, igroup) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            convert_name(&member.display),
            body.displayName,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(group_response(Group {
        name: body.displayName.clone(),
        emails: Some(
            body.members
                .clone()
                .into_iter()
                .map(|x| x.display.clone())
                .collect(),
        ),
    }))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Operation {
    pub op: String,
    pub path: String,
    pub value: Option<serde_json::Value>,
}

#[cfg(feature = "enterprise")]
#[derive(Deserialize, Debug)]
pub struct UpdateGroup {
    pub schemas: Vec<String>,
    pub displayName: Option<String>,
    pub members: Option<Vec<Member>>,
    pub Operations: Option<Vec<Operation>>,
}

#[cfg(feature = "enterprise")]
pub async fn update_group(
    Extension(db): Extension<DB>,
    Path(id): Path<String>,
    Json(body): Json<UpdateGroup>,
) -> Result<JsonScim<serde_json::Value>> {
    tracing::info!("SCIM updating group: {:?}", body);
    let mut tx: sqlx::Transaction<'_, sqlx::Postgres> = db.begin().await?;
    if body.schemas.len() == 1 {
        let schema = body.schemas.get(0).unwrap();
        if schema == "urn:ietf:params:scim:schemas:core:2.0:Group" {
            if let Some(members) = body.members.clone() {
                for m in members {
                    sqlx::query!(
                        "INSERT INTO email_to_igroup (email, igroup) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                        m.display,
                        id
                    )
                    .execute(&mut *tx)
                    .await?;
                }
                tx.commit().await?;
                Ok(group_response(Group {
                    name: body.displayName.unwrap_or_default(),
                    emails: Some(
                        body.members
                            .unwrap_or_default()
                            .clone()
                            .into_iter()
                            .map(|x| x.display.clone())
                            .collect(),
                    ),
                }))
            } else {
                Err(Error::BadRequest("expected members".to_string()))
            }
        } else {
            Err(Error::BadRequest("Invalid schemas".to_string()))
        }
    } else {
        Err(Error::BadRequest("Invalid schemas".to_string()))
    }
}

pub async fn delete_group(Extension(db): Extension<DB>, Path(id): Path<String>) -> Result<()> {
    tracing::info!("SCIM delete group: {:?}", id);
    sqlx::query!("DELETE FROM email_to_igroup WHERE igroup = $1", id)
        .execute(&db)
        .await?;
    sqlx::query!("DELETE FROM instance_group WHERE name = $1", id)
        .execute(&db)
        .await?;
    Ok(())
}

fn convert_name(name: &str) -> String {
    name.replace(" ", "_").to_lowercase()
}
