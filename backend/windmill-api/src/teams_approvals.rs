use crate::db::DB;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Extension,
};
use windmill_common::error::Error;

pub async fn request_teams_approval(
    Extension(_db): Extension<DB>,
    Path((_w_id, _job_id)): Path<(String, uuid::Uuid)>,
    Query(_approver): Query<crate::jobs::QueryApprover>,
    Query(_message): Query<crate::approvals::QueryMessage>,
) -> Result<StatusCode, Error> {
    Err(Error::InternalErr("enterprise feature only".to_string()))
}
