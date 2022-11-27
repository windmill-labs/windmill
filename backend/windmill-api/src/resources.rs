/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{
    db::{UserDB, DB},
    users::Authed,
};
use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::FromRow;
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    error::{Error, JsonResult, Result},
    utils::{not_found_if_none, paginate, require_admin, Pagination, StripPath},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_resources))
        .route("/get/*path", get(get_resource))
        .route("/exists/*path", get(exists_resource))
        .route("/get_value/*path", get(get_resource_value))
        .route("/update/*path", post(update_resource))
        .route("/delete/*path", delete(delete_resource))
        .route("/create", post(create_resource))
        .route("/type/list", get(list_resource_types))
        .route("/type/listnames", get(list_resource_types_names))
        .route("/type/get/:name", get(get_resource_type))
        .route("/type/exists/:name", get(exists_resource_type))
        .route("/type/update/:name", post(update_resource_type))
        .route("/type/delete/:name", delete(delete_resource_type))
        .route("/type/create", post(create_resource_type))
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct ResourceType {
    pub workspace_id: String,
    pub name: String,
    pub schema: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateResourceType {
    pub name: String,
    pub schema: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct EditResourceType {
    pub schema: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct Resource {
    pub workspace_id: String,
    pub path: String,
    pub value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub resource_type: String,
    pub extra_perms: serde_json::Value,
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct ListableResource {
    pub workspace_id: String,
    pub path: String,
    pub value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub resource_type: String,
    pub extra_perms: serde_json::Value,
    pub is_linked: Option<bool>,
    pub is_oauth: Option<bool>,
    pub is_expired: Option<bool>,
    pub refresh_error: Option<String>,
    pub account: Option<i32>,
}

#[derive(Deserialize)]
pub struct CreateResource {
    pub path: String,
    pub value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub resource_type: String,
}
#[derive(Deserialize)]
struct EditResource {
    path: Option<String>,
    description: Option<String>,
    value: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct ListResourceQuery {
    resource_type: Option<String>,
}
async fn list_resources(
    authed: Authed,
    Query(lq): Query<ListResourceQuery>,
    Query(pagination): Query<Pagination>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<ListableResource>> {
    let (per_page, offset) = paginate(pagination);

    let mut sqlb = SqlBuilder::select_from("resource")
        .fields(&[
            "resource.workspace_id",
            "resource.path",
            "null::JSONB as value",
            "resource.description",
            "resource_type",
            "resource.extra_perms",
            "(now() > account.expires_at) as is_expired",
            "variable.path IS NOT NULL as is_linked",
            "variable.is_oauth",
            "variable.account",
            "account.refresh_error",
        ])
        .left()
        .join("variable")
        .on("variable.path = resource.path AND variable.workspace_id = resource.workspace_id")
        .left()
        .join("account")
        .on("variable.account = account.id AND account.workspace_id = variable.workspace_id")
        .order_by("path", true)
        .and_where("resource.workspace_id = ? OR resource.workspace_id = 'starter'".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();

    if let Some(rt) = &lq.resource_type {
        sqlb.and_where_eq("resource_type", "?".bind(rt));
    }

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as::<_, ListableResource>(&sql)
        .fetch_all(&mut tx)
        .await?;

    tx.commit().await?;

    Ok(Json(rows))
}

async fn get_resource(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<ListableResource> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let resource_o = sqlx::query_as!(
        ListableResource,
        "SELECT resource.*, (now() > account.expires_at) as is_expired, account.refresh_error,
        variable.path IS NOT NULL as is_linked,
        variable.is_oauth as \"is_oauth?\",
        variable.account
        FROM resource
        LEFT JOIN variable ON variable.path = resource.path AND variable.workspace_id = resource.workspace_id
        LEFT JOIN account ON variable.account = account.id AND account.workspace_id = resource.workspace_id
        WHERE resource.path = $1 AND (resource.workspace_id = $2 OR resource.workspace_id = 'starter')",
        path.to_owned(),
        &w_id
    )
    .fetch_optional(&mut tx)
    .await?;
    tx.commit().await?;

    let resource = not_found_if_none(resource_o, "Resource", path)?;
    Ok(Json(resource))
}

async fn exists_resource(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM resource WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

async fn get_resource_value(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Option<serde_json::Value>> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let value_o = sqlx::query_scalar!(
        "SELECT value from resource WHERE path = $1 AND (workspace_id = $2 OR workspace_id = \
         'starter')",
        path.to_owned(),
        &w_id
    )
    .fetch_optional(&mut tx)
    .await?;
    tx.commit().await?;

    let value = not_found_if_none(value_o, "Resource", path)?;
    Ok(Json(value))
}

async fn create_resource(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(resource): Json<CreateResource>,
) -> Result<(StatusCode, String)> {
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "INSERT INTO resource
            (workspace_id, path, value, description, resource_type)
            VALUES ($1, $2, $3, $4, $5)",
        w_id,
        resource.path,
        resource.value,
        resource.description,
        resource.resource_type,
    )
    .execute(&mut tx)
    .await?;
    audit_log(
        &mut tx,
        &authed.username,
        "resources.create",
        ActionKind::Create,
        &w_id,
        Some(&resource.path),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        format!("resource {} created", resource.path),
    ))
}

async fn delete_resource(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "DELETE FROM resource WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .execute(&mut tx)
    .await?;
    sqlx::query!(
        "DELETE FROM variable WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .execute(&mut tx)
    .await?;
    audit_log(
        &mut tx,
        &authed.username,
        "resources.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("resource {} deleted", path))
}

async fn update_resource(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(ns): Json<EditResource>,
) -> Result<String> {
    use sql_builder::prelude::*;

    let path = path.to_path();

    let mut sqlb = SqlBuilder::update_table("resource");
    sqlb.and_where_eq("path", "?".bind(&path));
    sqlb.and_where_eq("workspace_id", "?".bind(&w_id));

    if let Some(npath) = &ns.path {
        sqlb.set_str("path", npath);
    }
    if let Some(nvalue) = ns.value {
        sqlb.set_str("value", nvalue.to_string());
    }
    if let Some(ndesc) = ns.description {
        sqlb.set_str("description", ndesc);
    }

    sqlb.returning("path");

    let mut tx = user_db.begin(&authed).await?;

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let npath_o: Option<String> = sqlx::query_scalar(&sql).fetch_optional(&mut tx).await?;

    let npath = not_found_if_none(npath_o, "Resource", path)?;

    if let Some(npath) = ns.path {
        sqlx::query!(
            "UPDATE variable SET path = $1 WHERE path = $2 AND workspace_id = $3",
            npath,
            path,
            w_id
        )
        .execute(&mut tx)
        .await?;
    }

    audit_log(
        &mut tx,
        &authed.username,
        "resources.update",
        ActionKind::Update,
        &w_id,
        Some(path),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("resource {} updated (npath: {:?})", path, npath))
}

async fn list_resource_types(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<ResourceType>> {
    let rows = sqlx::query_as!(
        ResourceType,
        "SELECT * from resource_type WHERE (workspace_id = $1 OR workspace_id = 'starter') ORDER \
         BY name",
        &w_id
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(rows))
}

async fn list_resource_types_names(
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<String>> {
    let rows = sqlx::query_scalar!(
        "SELECT name from resource_type WHERE (workspace_id = $1 OR workspace_id = 'starter') \
         ORDER BY name",
        &w_id
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(rows))
}

async fn get_resource_type(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
) -> JsonResult<ResourceType> {
    let mut tx = user_db.begin(&authed).await?;

    let resource_type_o = sqlx::query_as!(
        ResourceType,
        "SELECT * from resource_type WHERE name = $1 AND (workspace_id = $2 OR workspace_id = \
         'starter')",
        &name,
        &w_id
    )
    .fetch_optional(&mut tx)
    .await?;
    tx.commit().await?;

    let resource_type = not_found_if_none(resource_type_o, "ResourceType", name)?;
    Ok(Json(resource_type))
}

async fn exists_resource_type(
    Extension(db): Extension<DB>,
    Path((w_id, name)): Path<(String, String)>,
) -> JsonResult<bool> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM resource_type WHERE name = $1 AND workspace_id = $2)",
        name,
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

async fn create_resource_type(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(resource_type): Json<CreateResourceType>,
) -> Result<(StatusCode, String)> {
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "INSERT INTO resource_type
            (workspace_id, name, schema, description)
            VALUES ($1, $2, $3, $4)",
        w_id,
        resource_type.name,
        resource_type.schema,
        resource_type.description,
    )
    .execute(&mut tx)
    .await?;
    audit_log(
        &mut tx,
        &authed.username,
        "resource_types.create",
        ActionKind::Create,
        &w_id,
        Some(&resource_type.name),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        format!("resource_type {} created", resource_type.name),
    ))
}

async fn delete_resource_type(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "DELETE FROM resource_type WHERE name = $1 AND workspace_id = $2",
        name,
        w_id
    )
    .execute(&mut tx)
    .await?;
    audit_log(
        &mut tx,
        &authed.username,
        "resource_types.delete",
        ActionKind::Delete,
        &w_id,
        Some(&name),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("resource_type {} deleted", name))
}

async fn update_resource_type(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
    Json(ns): Json<EditResourceType>,
) -> Result<String> {
    use sql_builder::prelude::*;

    let mut sqlb = SqlBuilder::update_table("resource_type");
    sqlb.and_where_eq("name", "?".bind(&name));
    sqlb.and_where_eq("workspace_id", "?".bind(&w_id));
    if let Some(nschema) = ns.schema {
        sqlb.set_str("schema", nschema);
    }
    if let Some(ndesc) = ns.description {
        sqlb.set_str("description", ndesc);
    }
    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query(&sql).execute(&mut tx).await?;
    audit_log(
        &mut tx,
        &authed.username,
        "resource_types.update",
        ActionKind::Update,
        &w_id,
        Some(&name),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("resource_type {} updated", name))
}
