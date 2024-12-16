use std::str::FromStr;
use axum::{
    extract::{Form, Json, Path, Query},
    Extension,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::types::Uuid;
use std::collections::HashMap;
use windmill_common::error::{self, Error};

use regex::Regex;
use reqwest::Client;

use crate::db::{ApiAuthed, DB};
use crate::jobs::{get_resume_urls, resume_suspended_job, QueryApprover, QueryOrBody, ResumeUrls};

#[derive(Deserialize, Debug)]
pub struct SlackFormData {
    payload: String,
}

#[derive(Deserialize, Debug)]
struct Payload {
    actions: Vec<Action>,
    state: State,
    response_url: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Action {
    value: String,
}

#[derive(Deserialize, Debug)]
struct State {
    values: HashMap<String, HashMap<String, ValueInput>>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ValueInput {
    PlainTextInput { value: Option<Value> },
    Datepicker { selected_date: Option<String> },
    Timepicker { selected_time: Option<String> },
    StaticSelect { selected_option: Option<SelectedOption> },
    RadioButtons { selected_option: Option<SelectedOption> },
    Checkboxes { selected_options: Option<Vec<SelectedOption>> },
}

#[derive(Deserialize, Debug)]
struct SelectedOption {
    value: String,
}

pub async fn slack_app_callback_handler(
    authed: Option<ApiAuthed>,
    Extension(db): Extension<DB>,
    Form(form_data): Form<SlackFormData>,
) -> error::Result<StatusCode> {
    tracing::debug!("Form data: {:?}", form_data);
    let payload: Payload = serde_json::from_str(&form_data.payload)?;

    let action_value = payload.actions[0].value.clone();
    let response_url = payload.response_url;

    let re = Regex::new(r"/api/w/(?P<w_id>[^/]+)/jobs_u/(?P<action>resume|cancel)/(?P<job_id>[^/]+)/(?P<resume_id>[^/]+)/(?P<secret>[a-fA-F0-9]+)(?:\?approver=(?P<approver>[^&]+))?").unwrap();

    if let Some(captures) = re.captures(&action_value) {
        let w_id = captures.name("w_id").map_or("", |m| m.as_str());
        let action = captures.name("action").map_or("", |m| m.as_str());
        let job_id = captures.name("job_id").map_or("", |m| m.as_str());
        let resume_id = captures.name("resume_id").map_or("", |m| m.as_str());
        let secret = captures.name("secret").map_or("", |m| m.as_str());
        let approver =
            QueryApprover { approver: captures.name("approver").map(|m| m.as_str().to_string()) };

        tracing::debug!("Request: {:?}", &form_data.payload.clone());

        let state_values: HashMap<String, serde_json::Value> = payload
            .state
            .values
            .iter()
            .flat_map(|(_, inputs)| {
                inputs.iter().filter_map(|(action_id, input)| {
                    if action_id.ends_with("_date") {
                        let base_key = action_id.strip_suffix("_date").unwrap();
                        let time_key = format!("{}_time", base_key);

                        // Check for Datepicker and Timepicker inputs specifically
                        if let ValueInput::Datepicker { selected_date: Some(date) } = input {
                            let matching_time = payload
                                .state
                                .values
                                .values()
                                .flat_map(|inputs| {
                                    inputs.get(&time_key).and_then(|time_input| {
                                        if let ValueInput::Timepicker {
                                            selected_time: Some(time),
                                        } = time_input
                                        {
                                            Some(time)
                                        } else {
                                            None
                                        }
                                    })
                                })
                                .next();

                            if let Some(time) = matching_time {
                                return Some((
                                    base_key.to_string(),
                                    serde_json::json!(format!("{}T{}:00.000Z", date, time)),
                                ));
                            }
                        }
                    }

                    // Process non-datetime inputs, including plain text or other types with `_date`
                    match input {
                        ValueInput::PlainTextInput { value } => value
                            .as_ref()
                            .map(|v| (action_id.clone(), v.clone().into())),
                        ValueInput::StaticSelect { selected_option } => selected_option
                            .as_ref()
                            .map(|so| (action_id.clone(), serde_json::json!(so.value))),
                        ValueInput::RadioButtons { selected_option } => selected_option
                            .as_ref()
                            .map(|so| (action_id.clone(), serde_json::json!(so.value))),
                        ValueInput::Checkboxes { selected_options } => {
                            selected_options.as_ref().map(|so| {
                                (
                                    action_id.clone(),
                                    serde_json::json!(so
                                        .iter()
                                        .map(|option| option.value.clone())
                                        .collect::<Vec<_>>()),
                                )
                            })
                        }
                        _ => None,
                    }
                })
            })
            .collect();

        let state_json =
            serde_json::to_value(state_values).unwrap_or_else(|_| serde_json::json!({}));

        tracing::debug!("W ID: {}", w_id);
        tracing::debug!("Action: {}", action);
        tracing::debug!("Job ID: {}", job_id);
        tracing::debug!("Resume ID: {}", resume_id);
        tracing::debug!("Secret: {}", secret);
        tracing::debug!("Approver: {:?}", approver.approver);
        tracing::debug!("State JSON: {:?}", state_json);

        let res = resume_suspended_job(
            authed,
            Extension(db),
            Path((
                w_id.to_string(),
                Uuid::from_str(job_id).unwrap_or_default(),
                resume_id.parse::<u32>().unwrap_or_default(),
                secret.to_string(),
            )),
            Query(approver),
            QueryOrBody(Some(state_json)),
        )
        .await;

        tracing::debug!("Res: {:?}", res);

        if let Some(url) = response_url {
            let message = if action == "resume" {
                "\n\n*Workflow has been resumed!*"
            } else {
                "\n\n*Workflow has been canceled!*"
            };
            let _ = post_slack_response(&url, message).await;
        }
    } else {
        tracing::error!("Resume URL does not match the pattern.");
    }

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct QueryMessage {
    pub message: Option<String>,
}

pub async fn request_slack_approval(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, job_id, resume_id)): Path<(String, Uuid, u32)>,
    Query(approver): Query<QueryApprover>,
    Query(message): Query<QueryMessage>,
) -> windmill_common::error::JsonResult<serde_json::Value> {
    let res = get_resume_urls(
        authed,
        axum::Extension(db.clone()),
        Path((w_id, job_id, resume_id)),
        axum::extract::Query(approver),
    )
    .await;

    let schema: Option<ResumeFormRow> = sqlx::query_as!(
        ResumeFormRow,
        "SELECT
            module.value->'suspend'->'resume_form' AS resume_form,
            (module.value->'suspend'->>'hide_cancel')::boolean AS hide_cancel
        FROM
            job
        LEFT JOIN
            queue ON job.id = queue.parent_job
        LEFT JOIN
            jsonb_array_elements(job.raw_flow->'modules') AS module
                ON module->>'id' = queue.flow_step_id
        WHERE
            queue.id = $1",
        job_id
    )
    .fetch_optional(&db)
    .await?;

    tracing::debug!("schema: {:?}", schema);
    tracing::debug!("job_id: {:?}", job_id);

    let message_str = message
        .message
        .as_deref()
        .unwrap_or("*A workflow has been suspended and is waiting for approval:*\n");

    if let Some(resume_schema) = schema {
        let hide_cancel = resume_schema.hide_cancel.unwrap_or(false);

        let schema_obj = match resume_schema.resume_form {
            Some(schema) => schema,
            None => {
                tracing::debug!("No suspend form found!");
                return transform_schemas(
                    message_str,
                    None,
                    &res.unwrap().0,
                    None,
                    None,
                    hide_cancel,
                )
                .await
                .map(Json);
            }
        };

        let inner_schema = schema_obj.get("schema").ok_or_else(|| {
            Error::BadRequest("Schema object is missing the 'schema' field!".to_string())
        })?;

        let order_value = inner_schema
            .get("order")
            .ok_or_else(|| Error::BadRequest("Schema does not contain order field!".to_string()))?;

        let order: Vec<String> = serde_json::from_value(order_value.clone()).map_err(|e| {
            tracing::error!("Failed to deserialize order: {:?}", e);
            Error::BadRequest("Failed to deserialize order!".to_string())
        })?;

        let required_value = inner_schema.get("required").ok_or_else(|| {
            Error::BadRequest("Schema does not contain required field!".to_string())
        })?;

        let required: Vec<String> =
            serde_json::from_value(required_value.clone()).map_err(|e| {
                tracing::error!("Failed to deserialize required: {:?}", e);
                Error::BadRequest("Failed to deserialize required!".to_string())
            })?;

        let properties_value = inner_schema.get("properties").ok_or_else(|| {
            Error::BadRequest("Schema does not contain properties field!".to_string())
        })?;
        let properties: HashMap<String, ResumeFormField> =
            serde_json::from_value(properties_value.clone()).map_err(|e| {
                tracing::error!("Deserialization failed: {:?}", e);
                Error::BadRequest("Failed to deserialize properties!".to_string())
            })?;

        let blocks = transform_schemas(
            message_str,
            Some(&properties),
            &res.unwrap().0,
            Some(&order),
            Some(&required),
            hide_cancel,
        )
        .await?;
        Ok(Json(blocks))
    } else {
        Err(Error::BadRequest(
            "Could not generate interactive Slack message!".to_string(),
        ))
    }
}

async fn post_slack_response(
    response_url: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let final_blocks = vec![serde_json::json!({
        "type": "section",
        "text": {
            "type": "mrkdwn",
            "text": message
        }
    })];

    let payload = serde_json::json!({
        "replace_original": "true",
        "text": message,
        "blocks": final_blocks
    });

    let client = Client::new();

    let response = client
        .post(response_url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        tracing::debug!("Slack response to approval sent successfully!");
    } else {
        tracing::error!(
            "Slack response to approval failed. Status: {}",
            response.status()
        );
    }

    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResumeSchema {
    pub schema: Schema,
}

#[derive(Debug, Deserialize)]
pub struct ResumeFormRow {
    pub resume_form: Option<serde_json::Value>,
    pub hide_cancel: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Schema {
    pub order: Vec<String>,
    pub required: Vec<String>,
    pub properties: HashMap<String, ResumeFormField>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResumeFormField {
    #[serde(rename = "type")]
    pub r#type: String,
    pub format: Option<String>,
    pub default: Option<String>,
    pub description: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "enum")]
    pub r#enum: Option<Vec<String>>,
    #[serde(rename = "enumLabels")]
    pub enum_labels: Option<HashMap<String, String>>,
}

async fn transform_schemas(
    text: &str,
    properties: Option<&HashMap<String, ResumeFormField>>,
    urls: &ResumeUrls,
    order: Option<&Vec<String>>,
    required: Option<&Vec<String>>,
    hide_cancel: bool,
) -> Result<serde_json::Value, Error> {
    tracing::debug!("{:?}", urls);

    let mut blocks = vec![serde_json::json!({
        "type": "section",
        "text": {
            "type": "mrkdwn",
            "text": format!("{}\n<{}|Flow suspension details>", text, urls.approvalPage),
        }
    })];

    if let Some(properties) = properties {
        for key in order.unwrap() {
            if let Some(schema) = properties.get(key) {
                let is_required = required.unwrap().contains(key);
                let input_block = create_input_block(key, schema, is_required);
                tracing::debug!("Key: {:?}", key);
                tracing::debug!("Required: {:?}", required);
                match input_block {
                    serde_json::Value::Array(arr) => blocks.extend(arr),
                    _ => blocks.push(input_block),
                }
            }
        }
    }

    blocks.push(create_action_buttons(urls, hide_cancel));

    Ok(serde_json::Value::Array(blocks))
}

fn create_input_block(key: &str, schema: &ResumeFormField, required: bool) -> serde_json::Value {
    let placeholder = schema
        .description
        .as_deref()
        .filter(|desc| !desc.is_empty())
        .unwrap_or("Select an option");

    // Handle date-time format
    if schema.r#type == "string" && schema.format.as_deref() == Some("date-time") {
        let now = chrono::Local::now();
        let current_date = now.format("%Y-%m-%d").to_string();
        let current_time = now.format("%H:%M").to_string();

        let (default_date, default_time) = if let Some(default) = &schema.default {
            if let Ok(parsed_date) = chrono::DateTime::parse_from_rfc3339(default) {
                (
                    parsed_date.format("%Y-%m-%d").to_string(),
                    parsed_date.format("%H:%M").to_string(),
                )
            } else {
                (current_date.clone(), current_time.clone())
            }
        } else {
            (current_date.clone(), current_time.clone())
        };

        let title = schema.title.as_deref().unwrap_or(key);
        let title_with_required = if required {
            format!("{}*", title)
        } else {
            title.to_string()
        };

        return serde_json::json!([
            {
                "type": "input",
                "element": {
                    "type": "datepicker",
                    "initial_date": &default_date,
                    "placeholder": {
                        "type": "plain_text",
                        "text": "Select a date",
                        "emoji": true
                    },
                    "action_id": format!("{}_date", key)
                },
                "label": {
                    "type": "plain_text",
                    "text": title_with_required,
                    "emoji": true
                }
            },
            {
                "type": "input",
                "element": {
                    "type": "timepicker",
                    "initial_time": &default_time,
                    "placeholder": {
                        "type": "plain_text",
                        "text": "Select time",
                        "emoji": true
                    },
                    "action_id": format!("{}_time", key)
                },
                "label": {
                    "type": "plain_text",
                    "text": " ",
                    "emoji": true
                }
            }
        ]);
    }

    // Handle enum type
    if let Some(enums) = &schema.r#enum {
        let initial_option = schema.default.as_ref().and_then(|default_value| {
            enums
                .iter()
                .find(|enum_value| enum_value == &default_value)
                .map(|enum_value| {
                    serde_json::json!({
                        "text": {
                            "type": "plain_text",
                            "text": schema.enum_labels.as_ref()
                                .and_then(|labels| labels.get(enum_value))
                                .unwrap_or(enum_value),
                            "emoji": true
                        },
                        "value": enum_value
                    })
                })
        });

        let mut element = serde_json::json!({
            "type": "static_select",
            "placeholder": {
                "type": "plain_text",
                "text": placeholder,
                "emoji": true,
            },
            "options": enums.iter().map(|enum_value| {
                serde_json::json!({
                    "text": {
                        "type": "plain_text",
                        "text": schema.enum_labels.as_ref()
                            .and_then(|labels| labels.get(enum_value))
                            .unwrap_or(enum_value),
                        "emoji": true
                    },
                    "value": enum_value
                })
            }).collect::<Vec<_>>(),
            "action_id": key
        });

        if let Some(option) = initial_option {
            element["initial_option"] = option;
        }

        serde_json::json!({
            "type": "input",
            "element": element,
            "label": {
                "type": "plain_text",
                "text": schema.title.as_deref().unwrap_or(key),
                "emoji": true
            }
        })
    } else {
        // Handle other types
        serde_json::json!({
            "type": "input",
            "element": {
                "type": "plain_text_input",
                "action_id": key,
                "initial_value": schema.default.as_deref().unwrap_or("")
            },
            "label": {
                "type": "plain_text",
                "text": schema.title.as_deref().unwrap_or(key),
                "emoji": true
            }
        })
    }
}

fn create_action_buttons(urls: &ResumeUrls, hide_cancel: bool) -> serde_json::Value {
    let mut elements = vec![serde_json::json!({
        "type": "button",
        "text": {
            "type": "plain_text",
            "text": "Continue"
        },
        "style": "primary",
        "action_id": "resume_action",
        "value": urls.resume
    })];

    if !hide_cancel {
        elements.push(serde_json::json!({
            "type": "button",
            "text": {
                "type": "plain_text",
                "text": "Abort"
            },
            "style": "danger",
            "action_id": "cancel_action",
            "value": urls.cancel
        }));
    }

    serde_json::json!({
        "type": "actions",
        "elements": elements
    })
}
