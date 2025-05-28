use axum::http::StatusCode;
use windmill_common::error::Error;

pub async fn request_teams_approval() -> Result<StatusCode, Error> {
    crate::teams_approvals_ee::request_teams_approval().await
}