use serde::Deserialize;
use serde_json::value::RawValue;
use sqlx::types::Uuid;
use windmill_common::cache;
use windmill_common::db::DB;
use windmill_common::error::Error;

use axum::extract::{Path, Query};

use crate::jobs::{get_resume_urls_internal, ResumeUrls, QueryApprover};
use windmill_common::{
    error::{self},
    jobs::JobKind,
    scripts::ScriptHash,
};

#[derive(Debug, Deserialize)]
pub struct ResumeFormRow {
    pub resume_form: Option<serde_json::Value>,
    pub hide_cancel: Option<bool>,
}

#[derive(Deserialize)]
pub struct QueryMessage {
    pub message: Option<String>,
}

#[derive(Deserialize)]
pub struct QueryResourcePath {
    pub slack_resource_path: String,
}

#[derive(Deserialize)]
pub struct QueryChannelId {
    pub channel_id: String,
}

#[derive(Deserialize)]
pub struct QueryFlowStepId {
    pub flow_step_id: String,
}

#[derive(Deserialize, Debug)]
pub struct QueryDefaultArgsJson {
    pub default_args_json: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct QueryDynamicEnumJson {
    pub dynamic_enums_json: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct ApprovalFormDetails {
    pub message_str: String,
    pub urls: ResumeUrls,
    pub schema: Option<ResumeFormRow>,
}

pub async fn get_approval_form(
    db: DB,
    w_id: &str,
    job_id: Uuid,
    flow_step_id: Option<&str>,
    resume_id: u32,
    approver: Option<&str>,
    message: Option<&str>,
) -> Result<ApprovalFormDetails, Error> {
    let res = get_resume_urls_internal(
        axum::Extension(db.clone()),
        Path((w_id.to_string(), job_id, resume_id)),
        Query(QueryApprover { approver: approver.map(|a| a.to_string()) }),
    )
    .await?;

    let urls = res.0;

    tracing::debug!("Job ID: {:?}", job_id);

    let (job_kind, script_hash, raw_flow, parent_job_id, created_at, created_by, script_path, args) = sqlx::query!(
        "SELECT
            v2_as_queue.job_kind AS \"job_kind!: JobKind\",
            v2_as_queue.script_hash AS \"script_hash: ScriptHash\",
            v2_as_queue.raw_flow AS \"raw_flow: sqlx::types::Json<Box<RawValue>>\",
            v2_as_completed_job.parent_job AS \"parent_job: Uuid\",
            v2_as_completed_job.created_at AS \"created_at!: chrono::NaiveDateTime\",
            v2_as_completed_job.created_by AS \"created_by!\",
            v2_as_queue.script_path,
            v2_as_queue.args AS \"args: sqlx::types::Json<Box<RawValue>>\"
        FROM v2_as_queue
        JOIN v2_as_completed_job ON v2_as_completed_job.parent_job = v2_as_queue.id
        WHERE v2_as_completed_job.id = $1 AND v2_as_completed_job.workspace_id = $2
        LIMIT 1",
        job_id,
        &w_id
    )
    .fetch_optional(&db)
    .await
    .map_err(|e| error::Error::BadRequest(e.to_string()))?
    .ok_or_else(|| error::Error::BadRequest("This workflow is no longer running and has either already timed out or been cancelled or completed.".to_string()))
    .map(|r| (r.job_kind, r.script_hash, r.raw_flow, r.parent_job, r.created_at, r.created_by, r.script_path, r.args))?;

    let flow_data = match cache::job::fetch_flow(&db, job_kind, script_hash).await {
        Ok(data) => data,
        Err(_) => {
            if let Some(parent_job_id) = parent_job_id.as_ref() {
                cache::job::fetch_preview_flow(&db, parent_job_id, raw_flow).await?
            } else {
                return Err(error::Error::BadRequest(
                    "This workflow is no longer running and has either already timed out or been cancelled or completed.".to_string(),
                ));
            }
        }
    };

    let flow_value = &flow_data.flow;
    let flow_step_id = flow_step_id.unwrap_or("");
    let module = flow_value.modules.iter().find(|m| m.id == flow_step_id);

    tracing::debug!("Module: {:#?}", module);

    let schema = module.and_then(|module| {
        module.suspend.as_ref().map(|suspend| ResumeFormRow {
            resume_form: suspend.resume_form.clone(),
            hide_cancel: suspend.hide_cancel,
        })
    });

    let args_str = args.map_or("None".to_string(), |a| a.get().to_string());
    let parent_job_id_str = parent_job_id.map_or("None".to_string(), |id| id.to_string());
    let script_path_str = script_path.as_deref().unwrap_or("None");

    let created_at_formatted = created_at.format("%Y-%m-%d %H:%M:%S").to_string();

    let mut message_str = format!(
        "A workflow has been suspended and is waiting for approval:\n\n\
        *Created by*: {created_by}\n\
        *Created at*: {created_at_formatted}\n\
        *Script path*: {script_path_str}\n\
        *Args*: {args_str}\n\
        *Flow ID*: {parent_job_id_str}\n\n"
    );

    // Append custom message if provided
    if let Some(msg) = message {
        message_str.push_str(msg);
    }

    tracing::debug!("Schema: {:#?}", schema);

    Ok(ApprovalFormDetails { message_str, urls, schema })
}
