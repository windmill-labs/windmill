use axum::{extract::{Path, Query}, Extension, Router};
use http::StatusCode;
use serde::Deserialize;
use windmill_common::{db::UserDB, error};

use crate::db::ApiAuthed;

#[derive(Deserialize)]
struct DatabaseTrigger {}

#[derive(Deserialize)]
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
) -> error::Result<(StatusCode, String)> {
    todo!()
}

async fn list_database_triggers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(lst): Query<ListDatabaseTriggerQuery>,
) -> error::JsonResult<Vec<DatabaseTrigger>> {
    todo!()
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create", create_database_trigger)
        .nest("/list", list_database_triggers)
}
