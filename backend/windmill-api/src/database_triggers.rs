use axum::{
    extract::{Path, Query},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use windmill_audit::{audit_ee::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{self, JsonResult},
    utils::{not_found_if_none, StripPath},
    worker::CLOUD_HOSTED,
};

use crate::db::{ApiAuthed, DB};

#[derive(FromRow, Serialize, Deserialize)]
struct Database {
    username: String,
    password: Option<String>,
    host: String,
    port: u16,
}

impl Database {
    pub fn new(username: String, password: Option<String>, host: String, port: u16) -> Self {
        Self { username, password, host, port }
    }
}

#[derive(FromRow, Serialize, Deserialize)]
struct TableTrack {
    table_name: String,
    columns_name: Option<Vec<String>>,
    columns_name: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct EditDatabaseTrigger {
    path: Option<String>,
    script_path: Option<String>,
    is_flow: Option<String>,
    enabled: Option<bool>,
    database: Option<Database>,
    table_to_track: Option<Vec<TableTrack>>,
}

#[derive(Deserialize, Serialize)]

struct NewDatabaseTrigger {
    path: String,
    script_path: String,
    is_flow: bool,
    enabled: bool,
    database: Database,
    table_to_track: Option<Vec<TableTrack>>,
}

#[derive(FromRow, Deserialize, Serialize)]
struct DatabaseTrigger {
    database: Database,
    table_to_track: Option<Vec<TableTrack>>,
}

#[derive(Deserialize, Serialize)]
pub struct ListDatabaseTriggerQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub path: Option<String>,
    pub is_flow: Option<bool>,
    pub path_start: Option<String>,
}

async fn create_database_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(new_database_trigger): Json<NewDatabaseTrigger>,
    Json(new_database_trigger): Json<NewDatabaseTrigger>,
) -> error::Result<(StatusCode, String)> {
    let NewDatabaseTrigger { database, table_to_track, path, script_path, enabled, is_flow } =
        new_database_trigger;
    if *CLOUD_HOSTED {
        return Err(error::Error::BadRequest(
            "Database triggers are not supported on multi-tenant cloud, use dedicated cloud or self-host".to_string(),
        ));
    }
    let mut tx = user_db.begin(&authed).await?;

    let NewDatabaseTrigger { database, table_to_track, path, script_path, enabled, is_flow } =
        new_database_trigger;
    if *CLOUD_HOSTED {
        return Err(error::Error::BadRequest(
            "Database triggers are not supported on multi-tenant cloud, use dedicated cloud or self-host".to_string(),
        ));
    }
    let mut tx = user_db.begin(&authed).await?;

    Ok((StatusCode::OK, "OK".to_string()))
}

async fn list_database_triggers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(lst): Query<ListDatabaseTriggerQuery>,
) -> error::JsonResult<Vec<DatabaseTrigger>> {
    Ok(Json(Vec::new()))
}

async fn get_database_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::JsonResult<DatabaseTrigger> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();
    let trigger = sqlx::query_as::<_, DatabaseTrigger>(
        r#"SELECT *
          FROM database_trigger
          WHERE workspace_id = $1 AND path = $2"#,
    )
    .bind(w_id)
    .bind(path)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let trigger = not_found_if_none(trigger, "Trigger", path)?;

    Ok(Json(trigger))
}

async fn update_database_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(database_trigger): Json<EditDatabaseTrigger>,
) -> error::Result<String> {
    let workspace_path = path.to_path();
    let EditDatabaseTrigger { script_path, path, is_flow, enabled, database, table_to_track } =
        database_trigger;
    let mut tx = user_db.begin(&authed).await?;

    /*sqlx::query!(
        "UPDATE database_trigger SET script_path = $1, path = $2, is_flow = $3, edited_by = $4, email = $5, edited_at = now(), error = NULL
            WHERE workspace_id = $6 AND path = $7",
        script_path,
        path,
        is_flow,
        &authed.username,
        &authed.email,
        w_id,
        workspace_path,
    )
    .execute(&mut *tx).await?;*/

    audit_log(
        &mut *tx,
        &authed,
        "database_triggers.update",
        ActionKind::Create,
        &w_id,
        Some(workspace_path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(workspace_path.to_string())
}

async fn delete_database_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::Result<String> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    sqlx::query!(
        "DELETE FROM database_trigger WHERE workspace_id = $1 AND path = $2",
        w_id,
        path,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "database_triggers.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("Database trigger {path} deleted"))
}

async fn exists_database_trigger(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let path = path.to_path();
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM database_trigger WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);
    Ok(Json(exists))
}

async fn set_enabled() -> error::Result<String> {
    Ok(format!("Ok"))
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create_database_trigger))
        .route("/list", get(list_database_triggers))
        .route("/get/*path", get(get_database_trigger))
        .route("/update/*path", post(update_database_trigger))
        .route("/delete/*path", delete(delete_database_trigger))
        .route("/exists/*path", get(exists_database_trigger))
        .route("/setenabled/*path", post(set_enabled))
}
