use axum::{http::StatusCode, Router};
use windmill_common::error::Error;

pub async fn edit_teams_command() -> Result<StatusCode, Error> {
    crate::teams_ee::edit_teams_command().await
}

pub async fn workspaces_list_available_teams_ids() -> Result<StatusCode, Error> {
    crate::teams_ee::workspaces_list_available_teams_ids().await
}

pub async fn connect_teams() -> Result<StatusCode, Error> {
    crate::teams_ee::connect_teams().await
}

pub async fn run_teams_message_test_job() -> Result<StatusCode, Error> {
    crate::teams_ee::run_teams_message_test_job().await
}

pub async fn workspaces_list_available_teams_channels() -> Result<StatusCode, Error> {
    crate::teams_ee::workspaces_list_available_teams_channels().await
}

pub fn teams_service() -> Router {
    crate::teams_ee::teams_service()
}