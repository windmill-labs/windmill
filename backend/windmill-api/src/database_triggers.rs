use axum::{
    extract::{Path, Query},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use windmill_common::{db::UserDB, error, utils::StripPath};

use crate::db::ApiAuthed;

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
struct TableTrack {
    table_name: String,
    columns_name: Vec<String>,
}

#[derive(Deserialize)]
struct EditDatabaseTrigger {}

#[derive(Deserialize, Serialize)]
struct CreateDatabaseTrigger {
    database: Database,
    table_to_track: Option<Vec<TableTrack>>,
}

#[derive(Deserialize, Serialize)]
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
    Json(CreateDatabaseTrigger { database, table_to_track }): Json<CreateDatabaseTrigger>,
) -> error::Result<(StatusCode, String)> {
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
    todo!()
}

async fn update_database_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(ct): Json<EditDatabaseTrigger>,
) -> error::Result<String> {
    Ok(String::new())
}

async fn delete_database_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> error::Result<String> {
    Ok(format!("Databse trigger deleted"))
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create", post(create_database_trigger))
        .route("/list", get(list_database_triggers))
        .route("/get/*path", get(get_database_trigger))
        .route("/update/*path", post(update_database_trigger))
        .route("/delete/*path", delete(delete_database_trigger))
}
