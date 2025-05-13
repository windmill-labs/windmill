use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use std::str::FromStr;
use regex::Regex;
use serde_json::Value;
use crate::db::{ApiAuthed, DB};
use crate::jobs::{cancel_suspended_job, resume_suspended_job, QueryApprover, QueryOrBody, ResumeUrls, get_resume_urls_internal};
use axum::{extract::{Path, Query}, Extension};
use windmill_common::error::Error;
use windmill_common::cache;
use windmill_common::jobs::JobKind;
use windmill_common::scripts::ScriptHash;
use serde_json::value::RawValue;

#[derive(Debug, Deserialize, Serialize)]
pub struct ResumeSchema {
    pub schema: Schema,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Schema {
    pub order: Vec<String>,
    pub required: Vec<String>,
    pub properties: HashMap<String, ResumeFormField>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Boolean,
    String,
    Number,
    Integer,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResumeFormField {
    pub r#type: FieldType,
    pub format: Option<String>,
    pub default: Option<serde_json::Value>,
    pub description: Option<String>,
    pub title: Option<String>,
    pub r#enum: Option<Vec<String>>,
    #[serde(rename = "enumLabels")]
    pub enum_labels: Option<HashMap<String, String>>,
    pub nullable: Option<bool>,
    pub placeholder: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResumeFormRow {
    pub resume_form: Option<serde_json::Value>,
    pub hide_cancel: Option<bool>,
}

#[derive(Deserialize)]
pub struct QueryMessage {
    pub message: Option<String>,
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

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum MessageFormat {
    Slack,
    Teams,
}

pub fn extract_w_id_from_resume_url(resume_url: &str) -> Result<&str, Error> {
    let re = Regex::new(r"/api/w/(?P<w_id>[^/]+)/jobs_u/(?P<action>resume|cancel)/(?P<job_id>[^/]+)/(?P<resume_id>[^/]+)/(?P<secret>[a-fA-F0-9]+)(?:\?approver=(?P<approver>[^&]+))?").unwrap();
    let captures = re.captures(resume_url).ok_or_else(|| {
        tracing::error!("Resume URL does not match the pattern.");
        Error::BadRequest("Invalid URL format.".to_string())
    })?;
    Ok(captures.name("w_id").map_or("", |m| m.as_str()))
}

pub async fn handle_resume_action(
    authed: Option<ApiAuthed>,
    db: DB,
    resume_url: &str,
    form_data: Value,
    action: &str,
) -> Result<(), Error> {
    // Extract information from resume_url using regex
    let re = Regex::new(r"/api/w/(?P<w_id>[^/]+)/jobs_u/(?P<action>resume|cancel)/(?P<job_id>[^/]+)/(?P<resume_id>[^/]+)/(?P<secret>[a-fA-F0-9]+)(?:\?approver=(?P<approver>[^&]+))?").unwrap();
    let captures = re.captures(resume_url).ok_or_else(|| {
        tracing::error!("Resume URL does not match the pattern.");
        Error::BadRequest("Invalid URL format.".to_string())
    })?;

    let (w_id, job_id, resume_id, secret, approver) = (
        captures.name("w_id").map_or("", |m| m.as_str()),
        captures.name("job_id").map_or("", |m| m.as_str()),
        captures.name("resume_id").map_or("", |m| m.as_str()),
        captures.name("secret").map_or("", |m| m.as_str()),
        captures.name("approver").map(|m| m.as_str().to_string()),
    );

    let approver = QueryApprover { approver };

    // Convert job_id and resume_id to appropriate types
    let job_uuid = Uuid::from_str(job_id)
        .map_err(|_| Error::BadRequest("Invalid job ID format.".to_string()))?;

    let resume_id_parsed = resume_id
        .parse::<u32>()
        .map_err(|_| Error::BadRequest("Invalid resume ID format.".to_string()))?;

    // Call the appropriate function based on the action
    let res = if action == "resume" {
        resume_suspended_job(
            authed,
            Extension(db.clone()),
            Path((
                w_id.to_string(),
                job_uuid,
                resume_id_parsed,
                secret.to_string(),
            )),
            Query(approver),
            QueryOrBody(Some(form_data)),
        )
        .await
    } else {
        cancel_suspended_job(
            authed,
            Extension(db.clone()),
            Path((
                w_id.to_string(),
                job_uuid,
                resume_id_parsed,
                secret.to_string(),
            )),
            Query(approver),
            QueryOrBody(Some(form_data)),
        )
        .await
    };

    tracing::debug!("Job action result: {:#?}", res);
    res?;

    Ok(())
}

pub async fn get_approval_form_details(
    db: DB,
    w_id: &str,
    job_id: Uuid,
    flow_step_id: Option<&str>,
    resume_id: u32,
    approver: Option<&str>,
    message: Option<&str>,
    format: MessageFormat,
) -> Result<ApprovalFormDetails, Error> {
    let res = get_resume_urls_internal(
        axum::Extension(db.clone()),
        Path((w_id.to_string(), job_id, resume_id)),
        Query(QueryApprover { approver: approver.map(|a| a.to_string()) }),
    )
    .await?;

    let urls = res.0;

    tracing::debug!("Job ID: {:?}", job_id);

    // TODO: do we have a helper function for this?
    let (job_kind, script_hash, raw_flow, parent_job_id, created_at, created_by, script_path, args) = sqlx::query!(
        "WITH job_info AS (
            -- Query for Teams (running jobs)
            SELECT
                parent.job_kind AS \"job_kind!: JobKind\",
                parent.script_hash AS \"script_hash: ScriptHash\",
                parent.raw_flow AS \"raw_flow: sqlx::types::Json<Box<RawValue>>\",
                child.parent_job AS \"parent_job: Uuid\",
                parent.created_at AS \"created_at!: chrono::NaiveDateTime\",
                parent.created_by AS \"created_by!\",
                parent.script_path,
                parent.args AS \"args: sqlx::types::Json<Box<RawValue>>\"
            FROM v2_as_queue child
            JOIN v2_as_queue parent ON parent.id = child.parent_job
            WHERE child.id = $1 AND child.workspace_id = $2
            UNION ALL
            -- Query for Slack (completed jobs)
            SELECT
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
        )
        SELECT * FROM job_info LIMIT 1",
        job_id,
        &w_id
    )
    .fetch_optional(&db)
    .await
    .map_err(|e| Error::BadRequest(e.to_string()))?
    .ok_or_else(|| Error::BadRequest("This workflow is no longer running and has either already timed out or been cancelled or completed.".to_string()))
    .map(|r| (r.job_kind, r.script_hash, r.raw_flow, r.parent_job, r.created_at, r.created_by, r.script_path, r.args))?;

    let flow_data = match cache::job::fetch_flow(&db, job_kind, script_hash).await {
        Ok(data) => data,
        Err(_) => {
            if let Some(parent_job_id) = parent_job_id.as_ref() {
                cache::job::fetch_preview_flow(&db, parent_job_id, raw_flow).await?
            } else {
                return Err(Error::BadRequest(
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

    let bold_format = match format {
        MessageFormat::Slack => "*{}*",
        MessageFormat::Teams => "**{}**",
    };

    let mut message_str = format!(
        "A workflow has been suspended and is waiting for approval:\n\n\
        {}: {created_by}\n\n\
        {}: {created_at_formatted}\n\n\
        {}: {script_path_str}\n\n\
        {}: {args_str}\n\n\
        {}: {parent_job_id_str}\n\n",
        bold_format.replace("{}", "Created by"),
        bold_format.replace("{}", "Created at"),
        bold_format.replace("{}", "Script path"),
        bold_format.replace("{}", "Args"),
        bold_format.replace("{}", "Flow ID")
    );

    // Append custom message if provided
    if let Some(msg) = message {
        message_str.push_str(msg);
    }

    tracing::debug!("Schema: {:#?}", schema);

    Ok(ApprovalFormDetails { message_str, urls, schema })
}
