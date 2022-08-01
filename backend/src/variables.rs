/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::sync::Arc;

use crate::{
    audit::{audit_log, ActionKind},
    db::{UserDB, DB},
    error::{Error, JsonResult, Result},
    oauth2::{AllClients, _refresh_token},
    users::Authed,
    utils::StripPath,
};
use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use hyper::StatusCode;

use magic_crypt::{MagicCrypt256, MagicCryptTrait};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Postgres, Transaction};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_variables))
        .route("/list_contextual", get(list_contextual_variables))
        .route("/get/*path", get(get_variable))
        .route("/exists/*path", get(exists_variable))
        .route("/update/*path", post(update_variable))
        .route("/delete/*path", delete(delete_variable))
        .route("/create", post(create_variable))
}

#[derive(Serialize, Clone)]

pub struct ContextualVariable {
    pub name: String,
    pub value: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, FromRow)]

pub struct ListableVariable {
    pub workspace_id: String,
    pub path: String,
    pub value: Option<String>,
    pub is_secret: bool,
    pub description: String,
    pub extra_perms: serde_json::Value,
    pub account: Option<i32>,
    pub is_oauth: bool,
    pub is_expired: Option<bool>,
}

#[derive(Deserialize)]
pub struct CreateVariable {
    pub path: String,
    pub value: String,
    pub is_secret: bool,
    pub description: String,
    pub account: Option<i32>,
    pub is_oauth: Option<bool>,
}

#[derive(Deserialize)]
struct EditVariable {
    path: Option<String>,
    value: Option<String>,
    is_secret: Option<bool>,
    description: Option<String>,
}

pub fn get_reserved_variables(
    w_id: &str,
    token: &str,
    email: &str,
    username: &str,
    job_id: &str,
    permissioned_as: &str,
    path: Option<String>,
    flow_path: Option<String>,
    schedule_path: Option<String>,
) -> [ContextualVariable; 9] {
    [
        ContextualVariable {
            name: "WM_WORKSPACE".to_string(),
            value: w_id.to_string(),
            description: "Workspace id of the current script".to_string(),
        },
        ContextualVariable {
            name: "WM_TOKEN".to_string(),
            value: token.to_string(),
            description: "Token ephemeral to the current script with equal permission to the \
                          permission of the run (Usable as a bearer token)"
                .to_string(),
        },
        ContextualVariable {
            name: "WM_EMAIL".to_string(),
            value: email.to_string(),
            description: "Email of the user that executed the current script".to_string(),
        },
        ContextualVariable {
            name: "WM_USERNAME".to_string(),
            value: username.to_string(),
            description: "Username of the user that executed the current script".to_string(),
        },
        ContextualVariable {
            name: "WM_JOB_ID".to_string(),
            value: job_id.to_string(),
            description: "Job id of the current script".to_string(),
        },
        ContextualVariable {
            name: "WM_JOB_PATH".to_string(),
            value: path.unwrap_or_else(|| "".to_string()),
            description: "Path of the script or flow being run if any".to_string(),
        },
        ContextualVariable {
            name: "WM_FLOW_PATH".to_string(),
            value: flow_path.unwrap_or_else(|| "".to_string()),
            description: "Path of the encapsulating flow if the job is a flow step".to_string(),
        },
        ContextualVariable {
            name: "WM_SCHEDULE_PATH".to_string(),
            value: schedule_path.unwrap_or_else(|| "".to_string()),
            description: "Path of the schedule if the job of the step or encapsulating step has \
                          been triggered by a schedule"
                .to_string(),
        },
        ContextualVariable {
            name: "WM_PERMISSIONED_AS".to_string(),
            value: permissioned_as.to_string(),
            description: "Fully Qualified (u/g) owner name of executor of the job".to_string(),
        },
    ]
}

async fn list_contextual_variables(
    Path(w_id): Path<String>,
    Authed { username, email, .. }: Authed,
) -> JsonResult<Vec<ContextualVariable>> {
    Ok(Json(
        get_reserved_variables(
            &w_id,
            "q1A0qcPuO00yxioll7iph76N9CJDqn",
            &email.unwrap_or_else(|| "no email".to_string()),
            &username,
            "017e0ad5-f499-73b6-5488-92a61c5196dd",
            format!("u/{username}").as_str(),
            Some("u/user/script_path".to_string()),
            Some("u/user/encapsulating_flow_path".to_string()),
            Some("u/user/triggering_flow_path".to_string()),
        )
        .to_vec(),
    ))
}

async fn list_variables(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<ListableVariable>> {
    let mut tx = user_db.begin(&authed).await?;

    let rows = sqlx::query_as::<_, ListableVariable>(
        "SELECT workspace_id, path, CASE WHEN is_secret IS TRUE THEN null ELSE value::text END as \
         value, is_secret, description, extra_perms, account, is_oauth, false as is_expired from \
         variable
         WHERE (workspace_id = $1 OR (is_secret IS NOT TRUE AND workspace_id = 'starter')) ORDER \
         BY path",
    )
    .bind(&w_id)
    .fetch_all(&mut tx)
    .await?;

    tx.commit().await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
struct GetVariableQuery {
    decrypt_secret: Option<bool>,
}

async fn get_variable(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Query(q): Query<GetVariableQuery>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Extension(clients): Extension<Arc<AllClients>>,
    Extension(http_client): Extension<Client>,
) -> JsonResult<ListableVariable> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let variable_o = sqlx::query_as::<_, ListableVariable>(
        "SELECT variable.*, (now() > account.expires_at) as is_expired from variable
        LEFT JOIN account ON variable.account = account.id
        WHERE variable.path = $1 AND (variable.workspace_id = $2 OR (is_secret IS NOT TRUE AND \
         variable.workspace_id = 'starter')) 
        LIMIT 1",
    )
    .bind(&path)
    .bind(&w_id)
    .fetch_optional(&mut tx)
    .await?;

    let variable = crate::utils::not_found_if_none(variable_o, "Variable", &path)?;

    let decrypt_secret = q.decrypt_secret.unwrap_or(true);

    let r = if variable.is_secret {
        audit_log(
            &mut tx,
            &authed.username,
            "variables.decrypt_secret",
            ActionKind::Execute,
            &w_id,
            Some(&variable.path),
            None,
        )
        .await?;
        let value = variable.value.unwrap_or_else(|| "".to_string());
        ListableVariable {
            value: if variable.is_expired.unwrap_or(false) && variable.account.is_some() {
                Some(
                    _refresh_token(
                        tx,
                        &variable.path,
                        w_id,
                        variable.account.unwrap(),
                        clients,
                        http_client,
                    )
                    .await?,
                )
            } else if !value.is_empty() && decrypt_secret {
                let mc = build_crypt(&mut tx, &w_id).await?;
                tx.commit().await?;

                Some(
                    mc.decrypt_base64_to_string(value)
                        .map_err(|e| Error::InternalErr(e.to_string()))?,
                )
            } else {
                None
            },
            ..variable
        }
    } else {
        variable
    };

    Ok(Json(r))
}

async fn exists_variable(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM variable WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

async fn create_variable(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(variable): Json<CreateVariable>,
) -> Result<(StatusCode, String)> {
    let mut tx = user_db.begin(&authed).await?;

    let value = if variable.is_secret {
        let mc = build_crypt(&mut tx, &w_id).await?;
        encrypt(&mc, &variable.value)
    } else {
        variable.value
    };

    sqlx::query!(
        "INSERT INTO variable
            (workspace_id, path, value, is_secret, description, account, is_oauth)
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
        &w_id,
        variable.path,
        value,
        variable.is_secret,
        variable.description,
        variable.account,
        variable.is_oauth.unwrap_or(false),
    )
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
        &authed.username,
        "variables.create",
        ActionKind::Create,
        &w_id,
        Some(&variable.path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        format!("variable {} created", variable.path),
    ))
}

async fn delete_variable(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

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
        "variables.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("variable {} deleted", path))
}

async fn update_variable(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(ns): Json<EditVariable>,
) -> Result<String> {
    use sql_builder::prelude::*;

    let path = path.to_path();

    let mut tx = user_db.begin(&authed).await?;

    let mut sqlb = SqlBuilder::update_table("variable");
    sqlb.and_where_eq("path", "?".bind(&path));
    sqlb.and_where_eq("workspace_id", "?".bind(&w_id));

    if let Some(npath) = &ns.path {
        sqlb.set_str("path", npath);
    }
    if let Some(nvalue) = ns.value {
        let is_secret = sqlx::query_scalar!(
            "SELECT is_secret from variable WHERE path = $1 AND workspace_id = $2",
            &path,
            &w_id
        )
        .fetch_optional(&mut tx)
        .await?
        .unwrap_or(false);

        let value = if is_secret {
            let mc = build_crypt(&mut tx, &w_id).await?;
            encrypt(&mc, &nvalue)
        } else {
            nvalue
        };
        sqlb.set_str("value", &value);
    }

    if let Some(desc) = ns.description {
        sqlb.set_str("description", &desc);
    }

    if let Some(nbool) = ns.is_secret {
        if !nbool {
            return Err(Error::BadRequest(
                "A variable can not be updated to be non secret".to_owned(),
            ));
        }
        sqlb.set_str("is_secret", nbool);
    }
    sqlb.returning("path");

    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;

    let npath_o: Option<String> = sqlx::query_scalar(&sql).fetch_optional(&mut tx).await?;

    let npath = crate::utils::not_found_if_none(npath_o, "Variable", path)?;

    audit_log(
        &mut tx,
        &authed.username,
        "variables.update",
        ActionKind::Update,
        &w_id,
        Some(path),
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(format!("variable {} updated (npath: {:?})", path, npath))
}

pub async fn build_crypt<'c>(
    db: &mut Transaction<'c, Postgres>,
    w_id: &str,
) -> Result<MagicCrypt256> {
    let key = sqlx::query_scalar!(
        "SELECT key FROM workspace_key WHERE workspace_id = $1 AND kind = 'cloud'",
        w_id
    )
    .fetch_one(db)
    .await
    .map_err(|e| Error::InternalErr(format!("fetching crypt key: {e}")))?;

    Ok(magic_crypt::new_magic_crypt!(key, 256))
}

pub fn encrypt(mc: &MagicCrypt256, value: &str) -> String {
    mc.encrypt_str_to_base64(value)
}
