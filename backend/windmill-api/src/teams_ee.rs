use http::status::StatusCode;
use windmill_common::error::Error;

pub async fn edit_teams_command() -> Result<StatusCode, Error> {
    return Err(Error::BadRequest(
        "Deploy to is only available on enterprise".to_string(),
    ));
}

pub async fn workspaces_list_available_teams_ids() -> Result<StatusCode, Error> {
    return Err(Error::BadRequest(
        "Deploy to is only available on enterprise".to_string(),
    ));
}

pub async fn connect_teams() -> Result<StatusCode, Error> {
    return Err(Error::BadRequest(
        "Deploy to is only available on enterprise".to_string(),
    ));
}