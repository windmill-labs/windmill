use axum::Router;
use windmill_common::error::{Error, Result};

pub fn teams_service() -> Router {
    Router::new()
}

pub fn edit_teams_command() -> Result<String> {
    return Err(Error::BadRequest(
        "Deploy to is only available on enterprise".to_string(),
    ));
}

pub fn workspaces_list_available_teams_ids() -> Result<String> {
    return Err(Error::BadRequest(
        "Deploy to is only available on enterprise".to_string(),
    ));
}

pub fn connect_teams() -> Result<String> {
    return Err(Error::BadRequest(
        "Deploy to is only available on enterprise".to_string(),
    ));
}