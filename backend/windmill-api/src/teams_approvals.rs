use crate::approvals::{
    get_approval_form, ApprovalFormDetails, QueryDefaultArgsJson, QueryDynamicEnumJson, QueryFlowStepId, QueryMessage, ResumeFormRow,
};
use crate::jobs::QueryApprover;
use crate::db::{ApiAuthed, DB};
use axum::{
    extract::{Path, Query},
    Extension,
};
use http::StatusCode;
use serde::Deserialize;
use uuid::Uuid;
use windmill_common::error::Error;
use windmill_common::teams_ee::get_global_teams_bot_token;

#[derive(Deserialize)]
pub struct RequestTeamsApprovalPayload {
    pub team_name: String,
    pub channel_name: String,
    pub message: String,
    pub approver: String,
    pub default_args_json: Option<serde_json::Value>,
    pub dynamic_enums_json: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct QueryTeamName {
    pub team_name: String,
}

#[derive(Deserialize)]
pub struct QueryChannelName {
    pub channel_name: String,
}

pub async fn request_teams_approval(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, job_id)): Path<(String, Uuid)>,
    Query(team_name): Query<QueryTeamName>,
    Query(channel_name): Query<QueryChannelName>,
    Query(approver): Query<QueryApprover>,
    Query(message): Query<QueryMessage>,
    Query(flow_step_id): Query<QueryFlowStepId>,
    Query(default_args_json): Query<QueryDefaultArgsJson>,
    Query(dynamic_enums_json): Query<QueryDynamicEnumJson>,
) -> Result<StatusCode, Error> {
    let teams_bot_token = get_global_teams_bot_token(&db).await?;

    let resume_id = rand::random::<u32>();

    let approval_details = get_approval_form(
        db,
        w_id.as_str(),
        job_id,
        Some(flow_step_id.flow_step_id.as_str()),
        resume_id,
        approver.approver.as_deref(),
        message.message.as_deref(),
    ).await?;

    let ApprovalFormDetails { mut message_str, urls, schema } = approval_details;

    let blocks = transform_schemas(
        message_str,
        schema,
    );

    Ok(StatusCode::OK)
}

fn transform_schemas(message_str: String, schema: Option<ResumeFormRow>) -> Vec<String> {
    vec![]
}
