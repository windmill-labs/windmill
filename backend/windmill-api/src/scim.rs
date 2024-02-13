#![allow(non_snake_case)]

/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::Query,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Router,
};
use bytes::{BufMut, BytesMut};
use hyper::{header, http::HeaderValue, Request, StatusCode};
use mime_guess::mime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sql_builder::SqlBuilder;
use windmill_common::error::{Error, Result};

#[cfg(feature = "enterprise")]
use axum::{extract::Path, Json};

#[cfg(feature = "enterprise")]
use windmill_common::utils::not_found_if_none;

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
        .route(
            "/Users/:username",
            get(get_user)
                .put(update_user)
                .patch(update_user)
                .delete(delete_user),
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
    if let Some(header) = header {
        if let Ok(header) = header.to_str() {
            if header.starts_with("Bearer ") {
                let token = header.trim_start_matches("Bearer ");
                if let Some(scim_token) = SCIM_TOKEN.as_ref() {
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
    let mut sqlb = SqlBuilder::select_from("password")
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

#[cfg(feature = "enterprise")]
#[derive(Deserialize, Debug)]
pub struct CreateUser {
    userName: String,
}
#[cfg(feature = "enterprise")]
pub async fn create_user(
    Extension(db): Extension<DB>,
    Json(body): Json<CreateUser>,
) -> Result<JsonScim<serde_json::Value>> {
    use crate::workspaces::invite_user_to_all_auto_invite_worspaces;

    tracing::info!("SCIM creating user: {:?}", body);
    let email = body.userName.clone();
    sqlx::query!(
        "INSERT INTO password (email, login_type, verified) VALUES ($1, 'saml', true) ON CONFLICT DO NOTHING",
        &email,
    ).execute(&db).await?;
    invite_user_to_all_auto_invite_worspaces(&db, &email).await?;
    Ok(JsonScim(json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": body.userName,
        "userName": body.userName,
        "active": true
    })))
}

#[cfg(feature = "enterprise")]
pub async fn get_user(
    Extension(db): Extension<DB>,
    Path(username): Path<String>,
) -> Result<JsonScim<serde_json::Value>> {
    // This query looks dumb but it's a way to validate the user exists and it sets the stage if we want to support more fields in the future
    let username_from_db =
        sqlx::query_scalar!("SELECT email FROM password WHERE email = $1", username)
            .fetch_optional(&db)
            .await?;
    let username_from_db = not_found_if_none(username_from_db, "User", username)?;
    Ok(JsonScim(json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
        "id": username_from_db,
        "userName": username_from_db,
        "active": true
    })))
}

#[cfg(feature = "enterprise")]
pub async fn delete_user(Extension(db): Extension<DB>, Path(username): Path<String>) -> Result<()> {
    sqlx::query!("DELETE FROM email_to_igroup WHERE email = $1", username)
        .execute(&db)
        .await?;
    sqlx::query!("DELETE FROM password WHERE email = $1", username)
        .execute(&db)
        .await?;
    Ok(())
}

#[cfg(feature = "enterprise")]
#[derive(Deserialize, Debug)]
pub struct UpdateUser {
    pub schemas: Vec<String>,
    pub Operations: Option<Vec<Operation>>,
}

#[cfg(feature = "enterprise")]
pub async fn update_user(
    Extension(db): Extension<DB>,
    Path(username): Path<String>,
    Json(body): Json<UpdateUser>,
) -> Result<JsonScim<serde_json::Value>> {
    tracing::info!("SCIM updating user: {:?}", body);
    let mut tx: sqlx::Transaction<'_, sqlx::Postgres> = db.begin().await?;
    if body.schemas.len() == 1 {
        let schema = body.schemas.get(0).unwrap();
        let mut new_username = username.clone();
        if schema == "urn:ietf:params:scim:schemas:core:2.0:User" {
            // Since we only care about the username at this point, all we can do here is check that the user currently exists
            let username_from_db =
                sqlx::query_scalar!("SELECT email FROM password WHERE email = $1", username)
                    .fetch_optional(&mut *tx)
                    .await?;
            tx.commit().await?;
            let username_from_db = not_found_if_none(username_from_db, "User", username)?;
            Ok(JsonScim(json!({
                "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                "id": username_from_db,
                "userName": username_from_db,
                "active": true
            })))
        } else if schema == "urn:ietf:params:scim:api:messages:2.0:PatchOp" {
            for operation in body.Operations.unwrap_or_default() {
                match operation.op.as_str() {
                    "Replace" => match operation.path.as_str() {
                        "userName" => {
                            if let Some(username_raw) = operation.value {
                                let new_username_tmp = serde_json::from_value::<String>(
                                    username_raw,
                                )
                                .map_err(|err| {
                                    Error::InternalErr(format!("Error parsing user name: {}", err))
                                })?;
                                // TODO: that's fishy, maybe we shouldn't allow updating the email
                                // I think it makes sense in a world where you rely on external IDs, but since it's not
                                // our case, it makes things a bit borderline
                                sqlx::query!(
                                    "UPDATE password SET email = $1 WHERE email = $2",
                                    new_username_tmp,
                                    username
                                )
                                .execute(&mut *tx)
                                .await?;
                                sqlx::query!(
                                    "UPDATE email_to_igroup SET email = $1 WHERE email = $2",
                                    new_username_tmp,
                                    username
                                )
                                .execute(&mut *tx)
                                .await?;
                                new_username = new_username_tmp
                            }
                        }
                        unknown => {
                            tracing::info!("Unsupported patch operation on user: {} with path {}. It will be ignored", new_username, unknown)
                        }
                    },
                    "Add" => match operation.path.as_str() {
                        unknown => {
                            tracing::info!("Unsupported patch-add operation on user: {} with path {}. It will be ignored", new_username, unknown)
                        }
                    },
                    "Remove" => match operation.path.as_str() {
                        unknown => {
                            tracing::info!("Unsupported patch-add operation on user: {} with path {}. It will be ignored", new_username, unknown)
                        }
                    },
                    unknown => {
                        return Err(Error::BadRequest(
                            format!("Invalid operation: {}", unknown).to_string(),
                        ))
                    }
                }
            }
            tx.commit().await?;
            Ok(JsonScim(json!({
                "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
                "id": new_username,
                "userName": new_username,
                "active": true
            })))
        } else {
            Err(Error::BadRequest("Invalid schemas".to_string()))
        }
    } else {
        Err(Error::BadRequest("Invalid schemas".to_string()))
    }
}

#[cfg(feature = "enterprise")]
pub async fn get_groups(
    Extension(db): Extension<DB>,
    Query(query): Query<ScimQuery>,
) -> Result<JsonScim<serde_json::Value>> {
    let sqlb = SqlBuilder::select_from("instance_group")
        .fields(&[
            "name",
            "external_id",
            "scim_display_name",
            "array_remove(array_agg(email_to_igroup.email), null) as emails",
        ])
        .left()
        .join("email_to_igroup")
        .on("instance_group.name = email_to_igroup.igroup")
        .group_by("name, external_id")
        .limit(query.count.unwrap_or(100000))
        .offset(query.startIndex.map(|x| x - 1).unwrap_or(0))
        .clone();

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let groups = sqlx::query_as::<_, Group>(&sql)
        .fetch_all(&db)
        .await?
        .into_iter()
        .map(|x| group_response(x).0)
        .collect();
    Ok(resource_response(
        "urn:ietf:params:scim:api:messages:2.0:ListResponse",
        groups,
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
#[derive(Serialize, sqlx::FromRow)]
pub struct Group {
    name: String,
    emails: Option<Vec<String>>,
    external_id: Option<String>,
    scim_display_name: Option<String>,
}
#[cfg(feature = "enterprise")]
fn group_response(group: Group) -> JsonScim<serde_json::Value> {
    let external_id = group
        .external_id
        .unwrap_or_else(|| convert_name(&group.name));
    let json = json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Group"],
        "displayName": group.scim_display_name.unwrap_or_default(),
        "id": external_id,
        "objectId": external_id,
        "members": group.emails.unwrap_or_default().into_iter().map(|x| json!({"value": x, "display": x})).collect::<Vec<serde_json::Value>>(),
        "meta": {
            "resourceType": "Group"
        }
    });
    tracing::info!("SCIM group: {:?}", json);
    return JsonScim(json);
}

#[cfg(feature = "enterprise")]
pub async fn get_group(
    Extension(db): Extension<DB>,
    Path(id): Path<String>,
) -> Result<JsonScim<serde_json::Value>> {
    let group= sqlx::query_as!(
        Group,
        "SELECT name, external_id, scim_display_name, array_remove(array_agg(email_to_igroup.email), null) as emails FROM email_to_igroup RIGHT JOIN instance_group ON instance_group.name = email_to_igroup.igroup WHERE external_id = $1 group by name, external_id",
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
    use uuid::Uuid;

    tracing::info!("SCIM creating group: {:?}", body);
    let mut tx: sqlx::Transaction<'_, sqlx::Postgres> = db.begin().await?;
    let id = Uuid::new_v4().to_string();
    let scim_display_name = Some(body.displayName.clone());
    let name = convert_name(&body.displayName.clone());
    sqlx::query!(
        "INSERT INTO instance_group (name, scim_display_name, external_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        name,
        body.displayName,
        id,
    )
    .execute(&mut *tx)
    .await?;
    tracing::info!(
        "SCIM created group: {} with external_id: {} (display name: {})",
        name,
        id,
        body.displayName
    );
    for member in &body.members {
        sqlx::query!(
            "INSERT INTO email_to_igroup (email, igroup) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            convert_name(&member.display),
            name,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(group_response(Group {
        external_id: Some(id),
        name,
        scim_display_name,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateMembersOperationValue {
    pub value: String,
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
            let group= sqlx::query_as!(
                Group,
                "SELECT name, external_id, scim_display_name, array_remove(array_agg(email_to_igroup.email), null) as emails FROM email_to_igroup RIGHT JOIN instance_group ON instance_group.name = email_to_igroup.igroup WHERE external_id = $1 GROUP BY name",
                id
            )
            .fetch_optional(&db)
            .await?;
            let mut group = not_found_if_none(group, "Group", id.clone())?;

            sqlx::query!("DELETE FROM email_to_igroup WHERE igroup = $1", group.name)
                .execute(&mut *tx)
                .await?;

            let new_name = if let Some(name) = body.displayName.clone() {
                let new_name = convert_name(name.as_str());
                sqlx::query!(
                    "UPDATE instance_group SET scim_display_name = $1,  name = $2 where external_id = $3",
                    name,
                    new_name,
                    id
                )
                .execute(&mut *tx)
                .await?;
                group.scim_display_name = Some(name);
                new_name
            } else {
                group.name.clone()
            };

            if let Some(members) = body.members.clone() {
                let mut emails = vec![];
                for m in members {
                    emails.push(m.display.clone());
                    sqlx::query!(
                        "INSERT INTO email_to_igroup (email, igroup) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                        m.display,
                        new_name,
                    )
                    .execute(&mut *tx)
                    .await?;
                }
                tx.commit().await?;
                group.emails = Some(emails);
                Ok(group_response(group))
            } else {
                Err(Error::BadRequest("expected members".to_string()))
            }
        } else if schema == "urn:ietf:params:scim:api:messages:2.0:PatchOp" {
            let group_name_opt =
                sqlx::query_scalar!("SELECT name FROM instance_group WHERE external_id = $1", id)
                    .fetch_optional(&db)
                    .await?;
            let group_name = not_found_if_none(group_name_opt, "Group", id.clone())?;
            let mut new_external_id = id;
            for operation in body.Operations.unwrap_or_default() {
                match operation.op.as_str() {
                    "Replace" => match operation.path.as_str() {
                        "displayName" => {
                            if let Some(name_raw) = operation.value {
                                let name =
                                    serde_json::from_value::<String>(name_raw).map_err(|err| {
                                        Error::InternalErr(format!(
                                            "Error parsing group name: {}",
                                            err
                                        ))
                                    })?;
                                let new_name = convert_name(name.as_str());
                                sqlx::query!(
                                    "UPDATE instance_group SET scim_display_name = $1, name = $2 where external_id = $3",
                                    name,
                                    new_name,
                                    new_external_id
                                )
                                .execute(&mut *tx)
                                .await?;
                            }
                        }
                        unknown => {
                            tracing::info!("Unsupported patch operation on group: {} with path {}. It will be ignored", new_external_id, unknown)
                        }
                    },
                    "Add" => match operation.path.as_str() {
                        "members" => {
                            if let Some(value) = operation.value {
                                let parsed_ops = serde_json::from_value::<
                                    Vec<UpdateMembersOperationValue>,
                                >(value)
                                .map_err(|err| {
                                    Error::InternalErr(format!("Error parsing operation: {}", err))
                                })?;
                                for op in parsed_ops {
                                    sqlx::query!(
                                        "INSERT INTO email_to_igroup (email, igroup) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                                        op.value,
                                        group_name,
                                    )
                                    .execute(&mut *tx)
                                    .await?;
                                }
                            }
                        }
                        "externalId" => {
                            if let Some(external_id_raw) = operation.value {
                                let external_id = serde_json::from_value::<String>(external_id_raw)
                                    .map_err(|err| {
                                        Error::InternalErr(format!(
                                            "Error parsing group external ID: {}",
                                            err
                                        ))
                                    })?;
                                sqlx::query!(
                                    "UPDATE instance_group SET external_id = $1 WHERE name = $2",
                                    external_id,
                                    group_name
                                )
                                .execute(&mut *tx)
                                .await?;
                                new_external_id = external_id
                            }
                        }
                        unknown => {
                            tracing::info!("Unsupported patch-add operation on group: {} with path {}. It will be ignored", new_external_id, unknown)
                        }
                    },
                    "Remove" => match operation.path.as_str() {
                        "members" => {
                            if let Some(value) = operation.value {
                                let parsed_ops = serde_json::from_value::<
                                    Vec<UpdateMembersOperationValue>,
                                >(value)
                                .map_err(|err| {
                                    Error::InternalErr(format!("Error parsing operation: {}", err))
                                })?;
                                for op in parsed_ops {
                                    sqlx::query!(
                                        "DELETE FROM email_to_igroup WHERE email = $1 AND igroup = $2",
                                        op.value,
                                        group_name
                                    )
                                    .execute(&mut *tx)
                                    .await?;
                                }
                            }
                        }
                        unknown => {
                            tracing::info!("Unsupported patch-delete operation on group: {} with path {}. It will be ignored", new_external_id, unknown)
                        }
                    },
                    unknown => {
                        return Err(Error::BadRequest(
                            format!("Invalid operation: {}", unknown).to_string(),
                        ))
                    }
                }
            }
            tx.commit().await?;
            let group= sqlx::query_as!(
                Group,
                "SELECT name, external_id, scim_display_name, array_remove(array_agg(email_to_igroup.email), null) as emails FROM email_to_igroup RIGHT JOIN instance_group ON instance_group.name = email_to_igroup.igroup WHERE external_id = $1 GROUP BY name",
                new_external_id
            )
                .fetch_optional(&db)
                .await?;
            let group = not_found_if_none(group, "Group", new_external_id.clone())?;
            Ok(group_response(group))
        } else {
            Err(Error::BadRequest("Invalid schemas".to_string()))
        }
    } else {
        Err(Error::BadRequest("Invalid schemas".to_string()))
    }
}

#[cfg(feature = "enterprise")]
pub async fn delete_group(Extension(db): Extension<DB>, Path(id): Path<String>) -> Result<()> {
    tracing::info!("SCIM delete group: {:?}", id);
    let group = sqlx::query_scalar!("SELECT name FROM instance_group WHERE external_id = $1", id)
        .fetch_optional(&db)
        .await?;
    let group = not_found_if_none(group, "Group", id.clone())?;

    sqlx::query!("DELETE FROM email_to_igroup WHERE igroup = $1", group)
        .execute(&db)
        .await?;
    sqlx::query!("DELETE FROM instance_group WHERE name = $1", group)
        .execute(&db)
        .await?;
    Ok(())
}

#[cfg(feature = "enterprise")]
fn convert_name(name: &str) -> String {
    name.replace(" ", "_").to_lowercase()
}
