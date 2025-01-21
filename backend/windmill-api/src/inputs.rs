/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::ApiAuthed;
use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::types::Uuid;
use std::fmt::{Display, Formatter};
use windmill_common::{
    db::UserDB,
    error::JsonResult,
    jobs::JobKind,
    scripts::to_i64,
    utils::{not_found_if_none, paginate, Pagination},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/history", get(get_input_history))
        .route("/list", get(list_saved_inputs))
        .route("/create", post(create_input))
        .route("/update", post(update_input))
        .route("/delete/:id", post(delete_input))
        .route(
            "/:job_or_input_id/args",
            get(get_args_from_history_or_saved_input),
        )
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Copy, Clone)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct RunnableParams {
    pub runnable_id: String,
    pub runnable_type: RunnableType,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Input {
    id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
    args: Value,
    created_by: String,
    is_public: bool,
    success: bool,
}

#[derive(Deserialize)]
struct GetInputHistory {
    include_preview: Option<bool>,
}

async fn get_input_history(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(r): Query<RunnableParams>,
    Query(g): Query<GetInputHistory>,
) -> JsonResult<Vec<Input>> {
    let (per_page, offset) = paginate(pagination);

    let mut tx = user_db.begin(&authed).await?;

    let job_kinds = match (r.runnable_type, g.include_preview.unwrap_or(false)) {
        (RunnableType::FlowPath, true) => [JobKind::Flow, JobKind::FlowPreview].as_slice(),
        (RunnableType::FlowPath, _) => [JobKind::Flow].as_slice(),
        (_, true) => [JobKind::Script, JobKind::Preview].as_slice(),
        _ => [JobKind::Script].as_slice(),
    };

    let (runnable_id, runnable_path) = match r.runnable_type {
        RunnableType::ScriptHash => (Some(to_i64(&r.runnable_id)?), None),
        _ => (None, Some(&r.runnable_id)),
    };

    let inputs = sqlx::query!(
        "SELECT c.id, created_at, created_by, status = 'success'::job_status AS \"success!\"
         FROM v2_job_completed c JOIN v2_job USING (id)
         WHERE (runnable_id = $1 OR runnable_path = $2) AND kind = ANY($3) AND c.workspace_id = $4
         ORDER BY created_at DESC LIMIT $5 OFFSET $6",
        runnable_id,
        runnable_path,
        job_kinds as &[JobKind],
        &w_id,
        per_page as i32,
        offset as i32
    )
    .map(|r| Input {
        id: r.id,
        name: format!("{} {}", r.created_at.format("%H:%M %-d/%-m"), r.created_by),
        created_at: r.created_at,
        args: Value::Null,
        created_by: r.created_by,
        is_public: true,
        success: r.success,
    })
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(inputs))
}

#[derive(Deserialize)]
struct GetArgs {
    input: Option<bool>,
    allow_large: Option<bool>,
}

async fn get_args_from_history_or_saved_input(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Query(g): Query<GetArgs>,
    Path((w_id, job_or_input_id)): Path<(String, Uuid)>,
) -> JsonResult<Option<Value>> {
    let mut tx = user_db.begin(&authed).await?;
    let result_o = if let Some(input) = g.input {
        if input {
            sqlx::query_scalar!(
                "SELECT CASE WHEN pg_column_size(args) < 40000 OR $3 THEN args ELSE '\"WINDMILL_TOO_BIG\"'::jsonb END as args FROM input WHERE id = $1 AND workspace_id = $2",
                job_or_input_id,
                w_id,
                g.allow_large.unwrap_or(true)
            )
            .fetch_optional(&mut *tx)
            .await?
        } else {
            sqlx::query_scalar!(
                "SELECT CASE WHEN pg_column_size(args) < 40000 OR $3 THEN args ELSE '\"WINDMILL_TOO_BIG\"'::jsonb END as args FROM v2_job WHERE id = $1 AND workspace_id = $2",
                job_or_input_id,
                w_id,
                g.allow_large.unwrap_or(true)
            )
            .fetch_optional(&mut *tx)
            .await?
        }
    } else {
        sqlx::query_scalar!(
            "SELECT CASE WHEN pg_column_size(args) < 40000 OR $3 THEN args ELSE '\"WINDMILL_TOO_BIG\"'::jsonb END as args FROM v2_job WHERE id = $1 AND workspace_id = $2
            UNION ALL
            SELECT CASE WHEN pg_column_size(args) < 40000 OR $3 THEN args ELSE '\"WINDMILL_TOO_BIG\"'::jsonb END as args FROM input WHERE id = $1 AND workspace_id = $2",
            job_or_input_id,
            w_id,
            g.allow_large.unwrap_or(true)
        )
        .fetch_optional(&mut *tx)
        .await?
    };

    tx.commit().await?;

    let result = not_found_if_none(result_o, "Input args", job_or_input_id.to_string())?;

    Ok(Json(result))
}

async fn list_saved_inputs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
    Query(r): Query<RunnableParams>,
) -> JsonResult<Vec<Input>> {
    let (per_page, offset) = paginate(pagination);

    let mut tx = user_db.begin(&authed).await?;

    let inputs = sqlx::query_as!(
        Input,
        "SELECT id, name, 'null'::JSONB AS args, created_by, created_at, is_public,
             TRUE AS \"success!\"
         FROM input
         WHERE runnable_id = $1 AND runnable_type = $2 AND workspace_id = $3
             AND (is_public IS TRUE OR created_by = $4)
         ORDER BY created_at DESC LIMIT $5 OFFSET $6",
        &r.runnable_id,
        &r.runnable_type as &RunnableType,
        &w_id,
        &authed.username,
        per_page as i32,
        offset as i32
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(inputs))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInput {
    name: String,
    args: Box<serde_json::value::RawValue>,
}

async fn create_input(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(r): Query<RunnableParams>,
    Json(input): Json<CreateInput>,
) -> JsonResult<String> {
    let mut tx = user_db.begin(&authed).await?;

    let id = Uuid::new_v4();

    sqlx::query!(
        "INSERT INTO input (id, workspace_id, runnable_id, runnable_type, name, args, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        &id,
        &w_id,
        &r.runnable_id,
        &r.runnable_type as &RunnableType,
        &input.name,
        sqlx::types::Json(&input.args) as sqlx::types::Json<&Box<serde_json::value::RawValue>>,
        &authed.username
    )
    .execute(&mut *tx)
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
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(input): Json<UpdateInput>,
) -> JsonResult<String> {
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "UPDATE input SET name = $1, is_public = $2 WHERE id = $3 and workspace_id = $4",
        &input.name,
        &input.is_public,
        &input.id,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(input.id.to_string()))
}

async fn delete_input(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, i_id)): Path<(String, Uuid)>,
) -> JsonResult<String> {
    let mut tx = user_db.begin(&authed).await?;

    sqlx::query!(
        "DELETE FROM input WHERE id = $1 and workspace_id = $2",
        &i_id,
        &w_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(i_id.to_string()))
}
