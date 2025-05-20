#[cfg(feature = "private")]
use crate::teams_ee;

use http::status::StatusCode;
#[cfg(feature = "enterprise")]
use axum::Router;
use windmill_common::error::Error;

pub async fn edit_teams_command() -> Result<StatusCode, Error> {
    #[cfg(feature = "private")]
    {
        return teams_ee::edit_teams_command().await;
    }
    #[cfg(not(feature = "private"))]
    {
        return Err(Error::BadRequest(
            "Teams only available on enterprise".to_string(),
        ));
    }
}

pub async fn workspaces_list_available_teams_ids() -> Result<StatusCode, Error> {
    #[cfg(feature = "private")]
    {
        return teams_ee::workspaces_list_available_teams_ids().await;
    }
    #[cfg(not(feature = "private"))]
    {
        return Err(Error::BadRequest(
            "Teams only available on enterprise".to_string(),
        ));
    }
}

pub async fn connect_teams() -> Result<StatusCode, Error> {
    #[cfg(feature = "private")]
    {
        return teams_ee::connect_teams().await;
    }
    #[cfg(not(feature = "private"))]
    {
        return Err(Error::BadRequest(
            "Teams only available on enterprise".to_string(),
        ));
    }
}

pub async fn run_teams_message_test_job() -> Result<StatusCode, Error> {
    #[cfg(feature = "private")]
    {
        return teams_ee::run_teams_message_test_job().await;
    }
    #[cfg(not(feature = "private"))]
    {
        return Err(Error::BadRequest(
            "Teams only available on enterprise".to_string(),
        ));
    }
}

pub async fn workspaces_list_available_teams_channels() -> Result<StatusCode, Error> {
    #[cfg(feature = "private")]
    {
        return teams_ee::workspaces_list_available_teams_channels().await;
    }
    #[cfg(not(feature = "private"))]
    {
        return Err(Error::BadRequest(
            "Teams only available on enterprise".to_string(),
        ));
    }
}

#[cfg(feature = "enterprise")]
pub fn teams_service() -> Router {
    #[cfg(feature = "private")]
    {
        return teams_ee::teams_service();
    }
    #[cfg(not(feature = "private"))]
    {
        Router::new()
    }
}
