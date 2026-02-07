#[cfg(feature = "private")]
#[allow(unused)]
pub use crate::teams_ee::*;

#[cfg(all(feature = "enterprise", not(feature = "private")))]
use axum::Router;
#[cfg(not(feature = "private"))]
use http::status::StatusCode;
#[cfg(not(feature = "private"))]
use windmill_common::error::Error;

#[cfg(not(feature = "private"))]
pub async fn edit_teams_command() -> Result<StatusCode, Error> {
    return Err(Error::BadRequest(
        "Teams only available on enterprise".to_string(),
    ));
}

#[cfg(not(feature = "private"))]
pub async fn workspaces_list_available_teams_ids() -> Result<StatusCode, Error> {
    return Err(Error::BadRequest(
        "Teams only available on enterprise".to_string(),
    ));
}

#[cfg(not(feature = "private"))]
pub async fn workspaces_list_available_teams_channels() -> Result<StatusCode, Error> {
    return Err(Error::BadRequest(
        "Teams only available on enterprise".to_string(),
    ));
}

#[cfg(not(feature = "private"))]
pub async fn connect_teams() -> Result<StatusCode, Error> {
    return Err(Error::BadRequest(
        "Teams only available on enterprise".to_string(),
    ));
}

#[cfg(not(feature = "private"))]
pub async fn run_teams_message_test_job() -> Result<StatusCode, Error> {
    return Err(Error::BadRequest(
        "Teams only available on enterprise".to_string(),
    ));
}

#[cfg(all(feature = "enterprise", not(feature = "private")))]
pub fn teams_service() -> Router {
    Router::new()
}
