use http::status::StatusCode;
#[cfg(feature = "enterprise")]
use axum::Router;
use windmill_common::error::Error;

pub async fn edit_teams_command() -> Result<StatusCode, Error> {
    return Err(Error::BadRequest(
        "Teams only available on enterprise".to_string(),
    ));
}

pub async fn workspaces_list_available_teams_ids() -> Result<StatusCode, Error> {
    return Err(Error::BadRequest(
        "Teams only available on enterprise".to_string(),
    ));
}

pub async fn connect_teams() -> Result<StatusCode, Error> {
    return Err(Error::BadRequest(
        "Teams only available on enterprise".to_string(),
    ));
}

pub async fn run_teams_message_test_job() -> Result<StatusCode, Error> {
    return Err(Error::BadRequest(
        "Teams only available on enterprise".to_string(),
    ));
}

pub async fn workspaces_list_available_teams_channels() -> Result<StatusCode, Error> {
    return Err(Error::BadRequest(
        "Teams only available on enterprise".to_string(),
    ));
}

#[cfg(feature = "enterprise")]
pub fn teams_service() -> Router {
    Router::new()
}