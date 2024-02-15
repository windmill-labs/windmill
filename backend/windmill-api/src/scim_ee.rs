#![allow(non_snake_case)]

/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{extract::Query, routing::get, Extension, Router};
use sql_builder::SqlBuilder;
use windmill_common::error::{Error, Result};

use crate::db::DB;
use crate::scim_helpers::{resource_response, JsonScim, ScimQuery, ScimUser};

pub fn global_service() -> Router {
    Router::new().route("/Users", get(get_users))
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
        .map(|x: String| ScimUser { id: x.clone(), userName: x, active: true })
        .collect();
    tracing::info!("SCIM users: {:?}", users);
    Ok(resource_response(
        "urn:ietf:params:scim:api:messages:2.0:ListResponse",
        users,
    ))
}
