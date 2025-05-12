use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use std::str::FromStr;
use regex::Regex;
use serde_json::Value;
use crate::db::{ApiAuthed, DB};
use crate::jobs::{cancel_suspended_job, resume_suspended_job, QueryApprover, QueryOrBody};
use axum::{extract::{Path, Query}, Extension};
use windmill_common::error::Error;

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

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Boolean,
    String,
    Number,
    Integer,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize)]
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
