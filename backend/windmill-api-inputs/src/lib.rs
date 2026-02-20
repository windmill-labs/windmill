/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use windmill_api_auth::ApiAuthed;
use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{types::Uuid, FromRow};
use std::{
    fmt::{Display, Formatter},
    vec,
};
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

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct InputRow {
    pub id: Uuid,
    pub workspace_id: String,
    pub runnable_id: String,
    pub runnable_type: RunnableType,
    pub name: String,
    pub args: sqlx::types::Json<Box<serde_json::value::RawValue>>,
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
            RunnableType::ScriptHash => "runnable_id",
            RunnableType::ScriptPath => "runnable_path",
            RunnableType::FlowPath => "runnable_path",
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
    created_at: chrono::DateTime<chrono::Utc>,
    args: sqlx::types::Json<Box<serde_json::value::RawValue>>,
    created_by: String,
    is_public: bool,
    success: bool,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CompletedJobMini {
    id: Uuid,
    completed_at: chrono::DateTime<chrono::Utc>,
    args: Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
    created_by: String,
    success: bool,
}

#[derive(Deserialize)]
struct GetInputHistory {
    include_preview: Option<bool>,
    args: Option<String>,
    include_non_root: Option<bool>,
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

    let args_query = if let Some(args) = &g.args {
        sql_builder::bind::Bind::bind(&"and v2_job.args @> ?", &args.replace("'", "''"))
    } else {
        "".to_string()
    };

    let include_non_root = if g.include_non_root.unwrap_or(false) {
        ""
    } else {
        "AND parent_job IS NULL"
    };

    let sql = &format!(
        "select id, v2_job_completed.completed_at, created_by, 'null'::jsonb as args, status = 'success' as success from v2_job JOIN v2_job_completed USING (id) \
        where v2_job.workspace_id = $3 and {} = $1 and kind = any($2) {args_query} AND v2_job_completed.status != 'skipped' {include_non_root} \
        order by v2_job_completed.completed_at desc limit $4 offset $5",
        r.runnable_type.column_name(),

    );

    // tracing::info!("sql: {}", sql);
    let query = sqlx::query_as::<_, CompletedJobMini>(sql);

    let query = match r.runnable_type {
        RunnableType::ScriptHash => query.bind(to_i64(&r.runnable_id)?),
        _ => query.bind(&r.runnable_id),
    };

    let job_kinds = match r.runnable_type.job_kind() {
        kind @ JobKind::Script if g.include_preview.unwrap_or(false) => {
            vec![kind, JobKind::Preview]
        }
        kind @ JobKind::Flow if g.include_preview.unwrap_or(false) => {
            vec![kind, JobKind::FlowPreview]
        }
        kind => vec![kind],
    };

    let rows = query
        .bind(job_kinds)
        .bind(&w_id)
        .bind(per_page as i32)
        .bind(offset as i32)
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;

    let mut inputs = vec![];

    for row in rows {
        inputs.push(Input {
            id: row.id,
            name: format!(
                "{} {}",
                row.completed_at.format("%H:%M %-d/%-m"),
                row.created_by
            ),
            created_at: row.completed_at,
            args: sqlx::types::Json(
                serde_json::value::RawValue::from_string("null".to_string()).unwrap(),
            ),
            created_by: row.created_by,
            is_public: true,
            success: row.success,
        });
    }

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

    let rows = sqlx::query_as::<_, InputRow>(
        "select id, workspace_id, runnable_id, runnable_type, name, 'null'::jsonb as args, created_at, created_by, is_public from input \
         where runnable_id = $1 and runnable_type = $2 and workspace_id = $3 \
         and (is_public IS true OR created_by = $4) \
         order by created_at desc limit $5 offset $6",
    )
    .bind(&r.runnable_id)
    .bind(&r.runnable_type)
    .bind(&w_id)
    .bind(&authed.username)
    .bind(per_page as i32)
    .bind(offset as i32)
    .fetch_all(&mut *tx)
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
            success: true,
        })
    }

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

    sqlx::query(
        "INSERT INTO input (id, workspace_id, runnable_id, runnable_type, name, args, created_by) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&id)
    .bind(&w_id)
    .bind(&r.runnable_id)
    .bind(&r.runnable_type)
    .bind(&input.name)
    .bind(sqlx::types::Json(&input.args))
    .bind(&authed.username)
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

    sqlx::query("UPDATE input SET name = $1, is_public = $2 WHERE id = $3 and workspace_id = $4")
        .bind(&input.name)
        .bind(&input.is_public)
        .bind(&input.id)
        .bind(&w_id)
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

    sqlx::query("DELETE FROM input WHERE id = $1 and workspace_id = $2")
        .bind(&i_id)
        .bind(&w_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(i_id.to_string()))
}
