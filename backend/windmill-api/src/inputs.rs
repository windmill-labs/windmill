/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{db::UserDB, jobs::CompletedJob, users::Authed};
use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::types::Uuid;
use std::{
    fmt::{Display, Formatter},
    vec,
};
use windmill_common::{
    error::JsonResult,
    scripts::to_i64,
    utils::{paginate, Pagination},
};
use windmill_queue::JobKind;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/history", get(get_input_history))
        .route("/list", get(list_saved_inputs))
        .route("/create", post(create_input))
        .route("/update", post(update_input))
        .route("/delete/:id", post(delete_input))
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct InputRow {
    pub id: Uuid,
    pub workspace_id: String,
    pub runnable_id: String,
    pub runnable_type: RunnableType,
    pub name: String,
    pub args: Value,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub is_public: bool,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "runnable_type")]
pub enum RunnableType {
    ScriptHash,
    ScriptPath,
    FlowPath,
}

impl Display for RunnableType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RunnableType::ScriptHash => write!(f, "ScriptHash"),
            RunnableType::ScriptPath => write!(f, "ScriptPath"),
            RunnableType::FlowPath => write!(f, "FlowPath"),
        }
    }
}

impl RunnableType {
    fn job_kind(&self) -> JobKind {
        match self {
            RunnableType::ScriptHash => JobKind::Script,
            RunnableType::ScriptPath => JobKind::Script,
            RunnableType::FlowPath => JobKind::Flow,
        }
    }

    fn column_name(&self) -> &'static str {
        match self {
            RunnableType::ScriptHash => "script_hash",
            RunnableType::ScriptPath => "script_path",
            RunnableType::FlowPath => "script_path",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RunnableParams {
    pub runnable_id: String,
    pub runnable_type: RunnableType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
    id: Uuid,
    name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    args: serde_json::Value,
    created_by: String,
    is_public: bool,
}

async fn get_input_history(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(r): Query<RunnableParams>,
) -> JsonResult<Vec<Input>> {
    let (per_page, offset) = paginate(pagination);

    let mut tx = user_db.begin(&authed).await?;

    let sql = &format!(
        "select distinct on (args) * from completed_job \
        where {} = $1 and job_kind = $2 and workspace_id = $3 \
        order by args, started_at desc limit $4 offset $5",
        r.runnable_type.column_name()
    );

    let query = sqlx::query_as::<_, CompletedJob>(sql);

    let query = match r.runnable_type {
        RunnableType::ScriptHash => query.bind(to_i64(&r.runnable_id)?),
        _ => query.bind(&r.runnable_id),
    };

    let rows = query
        .bind(r.runnable_type.job_kind())
        .bind(&w_id)
        .bind(per_page as i32)
        .bind(offset as i32)
        .fetch_all(&mut tx)
        .await?;

    tx.commit().await?;

    let mut inputs = vec![];

    for row in rows {
        inputs.push(Input {
            id: row.id,
            name: format!(
                "{} {}",
                row.created_at.format("%H:%M %-d/%-m"),
                row.created_by
            ),
            created_at: row.created_at,
            args: row.args.unwrap_or(serde_json::json!({})),
            created_by: row.created_by,
            is_public: true,
        });
    }

    Ok(Json(inputs))
}

async fn list_saved_inputs(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(r): Query<RunnableParams>,
) -> JsonResult<Vec<Input>> {
    let (per_page, offset) = paginate(pagination);

    let mut tx = user_db.begin(&authed).await?;

    let rows = sqlx::query_as::<_, InputRow>(
        "select * from input \
         where runnable_id = $1 and runnable_type = $2 and workspace_id = $3 \
         and is_public IS true OR created_by = $4 \
         order by created_at desc limit $5 offset $6",
    )
    .bind(&r.runnable_id)
    .bind(&r.runnable_type)
    .bind(&w_id)
    .bind(&authed.username)
    .bind(per_page as i32)
    .bind(offset as i32)
    .fetch_all(&mut tx)
    .await?;

    tx.commit().await?;

    let mut inputs: Vec<Input> = Vec::new();

    for row in rows {
        inputs.push(Input {
            id: row.id,
            name: row.name,
            args: row.args,
            created_by: row.created_by,
            created_at: row.created_at,
            is_public: row.is_public,
        })
    }

    Ok(Json(inputs))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInput {
    name: String,
    args: serde_json::Value,
}

async fn create_input(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(r): Query<RunnableParams>,
    Json(input): Json<CreateInput>,
) -> JsonResult<String> {
    let mut tx = user_db.begin(&authed).await?;

    let id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO input (id, workspace_id, runnable_id, runnable_type, name, args, created_by) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&id)
    .bind(&w_id)
    .bind(&r.runnable_id)
    .bind(&r.runnable_type)
    .bind(&input.name)
    .bind(&input.args)
    .bind(&authed.username)
    .execute(&mut tx)
    .await?;

    tx.commit().await?;

    Ok(Json(id.to_string()))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateInput {
    id: Uuid,
    name: String,
    is_public: bool,
}

async fn update_input(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(input): Json<UpdateInput>,
) -> JsonResult<String> {
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query("UPDATE input SET name = $1, is_public = $2 WHERE id = $3 and workspace_id = $4")
        .bind(&input.name)
        .bind(&input.is_public)
        .bind(&input.id)
        .bind(&w_id)
        .execute(&mut tx)
        .await?;

    tx.commit().await?;

    Ok(Json(input.id.to_string()))
}

async fn delete_input(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, i_id)): Path<(String, Uuid)>,
) -> JsonResult<String> {
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query("DELETE FROM input WHERE id = $1 and workspace_id = $2")
        .bind(&i_id)
        .bind(&w_id)
        .execute(&mut tx)
        .await?;

    tx.commit().await?;

    Ok(Json(i_id.to_string()))
}
