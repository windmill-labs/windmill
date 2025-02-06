use axum::{
    extract::{Form, Path, Query},
    Extension,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::value::{RawValue, Value};

use sqlx::types::Uuid;
use std::{collections::HashMap, str::FromStr};

use regex::Regex;
use reqwest::Client;

use crate::db::{ApiAuthed, DB};
use crate::jobs::{
    cancel_suspended_job, get_resume_urls_internal, resume_suspended_job, QueryApprover,
    QueryOrBody, ResumeUrls,
};

use windmill_common::{
    cache,
    error::{self, Error},
    jobs::JobKind,
    scripts::ScriptHash,
    variables::{build_crypt, decrypt},
};

#[derive(Deserialize, Debug)]
pub struct SlackFormData {
    payload: String,
}

#[derive(Deserialize, Debug)]
struct Payload {
    actions: Option<Vec<Action>>,
    view: Option<View>,
    trigger_id: Option<String>,
    r#type: PayloadType,
    container: Option<Container>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum PayloadType {
    ViewSubmission,
    ViewClosed,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize, Debug, Serialize)]
struct Container {
    message_ts: String,
    channel_id: String,
}

#[derive(Deserialize, Debug)]
struct View {
    state: Option<State>,
    private_metadata: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "action_id")]
enum Action {
    #[serde(rename = "open_modal")]
    OpenModal { value: String },
    #[serde(other)]
    Unknown,
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

#[derive(Debug, Deserialize, Serialize)]
struct ResumeSchema {
    schema: Schema,
}

#[derive(Debug, Deserialize)]
struct ResumeFormRow {
    resume_form: Option<serde_json::Value>,
    hide_cancel: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Schema {
    order: Vec<String>,
    required: Vec<String>,
    properties: HashMap<String, ResumeFormField>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum FieldType {
    Boolean,
    String,
    Number,
    Integer,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize, Serialize)]
struct ResumeFormField {
    r#type: FieldType,
    format: Option<String>,
    default: Option<serde_json::Value>,
    description: Option<String>,
    title: Option<String>,
    r#enum: Option<Vec<String>>,
    #[serde(rename = "enumLabels")]
    enum_labels: Option<HashMap<String, String>>,
    nullable: Option<bool>,
}

#[derive(Deserialize)]
pub struct QueryMessage {
    message: Option<String>,
}

#[derive(Deserialize)]
pub struct QueryResourcePath {
    slack_resource_path: String,
}

#[derive(Deserialize)]
pub struct QueryChannelId {
    channel_id: String,
}

#[derive(Deserialize)]
pub struct QueryFlowStepId {
    flow_step_id: String,
}

#[derive(Deserialize, Debug)]
pub struct QueryDefaultArgsJson {
    default_args_json: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct QueryDynamicEnumJson {
    dynamic_enums_json: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
struct ModalActionValue {
    w_id: String,
    job_id: String,
    path: String,
    approver: Option<String>,
    message: Option<String>,
    flow_step_id: Option<String>,
    default_args_json: Option<String>,
    dynamic_enums_json: Option<String>,
}

#[derive(Deserialize, Debug)]
struct PrivateMetadata {
    resume_url: String,
    resource_path: String,
    container: Container,
    hide_cancel: Option<bool>,
}

pub async fn slack_app_callback_handler(
    authed: Option<ApiAuthed>,
    Extension(db): Extension<DB>,
    Form(form_data): Form<SlackFormData>,
) -> Result<StatusCode, Error> {
    tracing::debug!("Form data: {:#?}", form_data);
    let payload: Payload = serde_json::from_str(&form_data.payload)?;
    tracing::debug!("Payload: {:#?}", payload);

    match payload.r#type {
        PayloadType::ViewSubmission => handle_submission(authed, db, &payload, "resume").await?,
        PayloadType::ViewClosed => handle_submission(authed, db, &payload, "cancel").await?,
        _ => {
            if let Some(actions) = &payload.actions {
                if let Some(action) = actions.first() {
                    match action {
                        Action::OpenModal { value } => {
                            let trigger_id = payload.trigger_id.as_deref().ok_or_else(|| {
                                Error::BadRequest("Missing trigger_id".to_string())
                            })?;

                            let parsed_value: ModalActionValue =
                                serde_json::from_str(value.as_str()).map_err(|_| {
                                    Error::BadRequest("Invalid JSON in action value".to_string())
                                })?;

                            let w_id = &parsed_value.w_id;
                            let path = &parsed_value.path;
                            let approver = parsed_value.approver.as_deref();
                            let message = parsed_value.message.as_deref();
                            let job_id = Uuid::parse_str(&parsed_value.job_id)?;
                            let flow_step_id = parsed_value.flow_step_id.as_deref();

                            let slack_token = get_slack_token(&db, path, w_id).await?;
                            let client = Client::new();
                            let container = payload.container.ok_or_else(|| {
                                Error::BadRequest("No container found.".to_string())
                            })?;

                            let default_args_json: Option<serde_json::Value> = parsed_value
                                .default_args_json
                                .as_deref()
                                .map(|s| serde_json::from_str(s))
                                .transpose()
                                .map_err(|_| {
                                    Error::BadRequest(
                                        "Invalid JSON in default_args_json".to_string(),
                                    )
                                })?;

                            let dynamic_enums_json: Option<serde_json::Value> = parsed_value
                                .dynamic_enums_json
                                .as_deref()
                                .map(|s| serde_json::from_str(s))
                                .transpose()
                                .map_err(|_| {
                                    Error::BadRequest(
                                        "Invalid JSON in dynamic_enums_json".to_string(),
                                    )
                                })?;

                            tracing::debug!("Default args json: {:#?}", default_args_json);
                            tracing::debug!("Dynamic enum json: {:#?}", dynamic_enums_json);

                            open_modal_with_blocks(
                                &client,
                                slack_token.as_str(),
                                trigger_id,
                                db,
                                w_id,
                                path,
                                job_id,
                                approver,
                                message,
                                flow_step_id,
                                container,
                                default_args_json.as_ref(),
                                dynamic_enums_json.as_ref(),
                            )
                            .await
                            .map_err(|e| Error::BadRequest(e.to_string()))?;
                        }
                        Action::Unknown => println!("Unknown action_id"),
                    }
                } else {
                    tracing::debug!("Unknown Slack Action!");
                }
            } else {
                tracing::debug!("Unknown Slack Action!");
            }
        }
    }

    Ok(StatusCode::OK)
}

pub async fn request_slack_approval(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, job_id)): Path<(String, Uuid)>,
    Query(approver): Query<QueryApprover>,
    Query(message): Query<QueryMessage>,
    Query(slack_resource_path): Query<QueryResourcePath>,
    Query(channel_id): Query<QueryChannelId>,
    Query(flow_step_id): Query<QueryFlowStepId>,
    Query(default_args_json): Query<QueryDefaultArgsJson>,
    Query(dynamic_enums_json): Query<QueryDynamicEnumJson>,
) -> Result<StatusCode, Error> {
    let slack_resource_path = slack_resource_path.slack_resource_path;
    let channel_id = channel_id.channel_id;
    let flow_step_id = flow_step_id.flow_step_id;

    let slack_token = get_slack_token(&db, slack_resource_path.as_str(), &w_id).await?;
    let client = Client::new();

    tracing::debug!("Approver: {:?}", approver.approver);
    tracing::debug!("Message: {:?}", message.message);
    tracing::debug!("W ID: {:?}", w_id);
    tracing::debug!("Slack Resource Path: {:?}", slack_resource_path);
    tracing::debug!("Channel ID: {:?}", channel_id);

    send_slack_message(
        &client,
        slack_token.as_str(),
        channel_id.as_str(),
        &w_id,
        job_id,
        &slack_resource_path,
        approver.approver.as_deref(),
        message.message.as_deref(),
        flow_step_id.as_str(),
        default_args_json.default_args_json.as_ref(),
        dynamic_enums_json.dynamic_enums_json.as_ref(),
    )
    .await
    .map_err(|e| Error::BadRequest(e.to_string()))?;

    Ok(StatusCode::OK)
}

async fn handle_submission(
    authed: Option<ApiAuthed>,
    db: DB,
    payload: &Payload,
    action: &str,
) -> Result<(), Error> {
    let view = payload
        .view
        .as_ref()
        .ok_or_else(|| Error::BadRequest("No view found in payload.".to_string()))?;

    let state = view
        .state
        .as_ref()
        .ok_or_else(|| Error::BadRequest("No state found in view.".to_string()))?;

    let values = &state.values;
    tracing::debug!("Values: {:#?}", values);

    let state_values = parse_state_values(state);
    let state_json = serde_json::to_value(state_values).unwrap_or_else(|_| serde_json::json!({}));

    let private_metadata = view
        .private_metadata
        .as_ref()
        .ok_or_else(|| Error::BadRequest("No private metadata found.".to_string()))?;

    tracing::debug!("Private Metadata: {}", private_metadata);
    let private_metadata: PrivateMetadata = serde_json::from_str(private_metadata)?;
    let resume_url = private_metadata.resume_url;
    let resource_path = private_metadata.resource_path;
    let container: Container = private_metadata.container;
    let hide_cancel = private_metadata.hide_cancel;

    // If hide_cancel is true, we don't need to extract information from the private_metadata
    if hide_cancel.unwrap_or(false) && action == "cancel" {
        return Ok(());
    }

    // Use regex to extract information from private_metadata
    let re = Regex::new(r"/api/w/(?P<w_id>[^/]+)/jobs_u/(?P<action>resume|cancel)/(?P<job_id>[^/]+)/(?P<resume_id>[^/]+)/(?P<secret>[a-fA-F0-9]+)(?:\?approver=(?P<approver>[^&]+))?").unwrap();
    let captures = re.captures(resume_url.as_str()).ok_or_else(|| {
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

    let approver = QueryApprover { approver: approver };

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
            QueryOrBody(Some(state_json)),
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
            QueryOrBody(Some(state_json)),
        )
        .await
    };
    tracing::debug!("Resume job action result: {:#?}", res);
    let slack_token = get_slack_token(&db, &resource_path, &w_id).await?;
    update_original_slack_message(action, slack_token, container).await?;
    Ok(())
}

async fn transform_schemas(
    text: &str,
    properties: Option<&HashMap<String, ResumeFormField>>,
    urls: &ResumeUrls,
    order: Option<&Vec<String>>,
    required: Option<&Vec<String>>,
    default_args_json: Option<&serde_json::Value>,
    dynamic_enums_json: Option<&serde_json::Value>,
) -> Result<serde_json::Value, Error> {
    tracing::debug!("Resume urls: {:#?}", urls);

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

                let default_value = default_args_json.and_then(|json| json.get(key).cloned());
                let dynamic_enums_value =
                    dynamic_enums_json.and_then(|json| json.get(key).cloned());

                let input_block = create_input_block(
                    key,
                    schema,
                    is_required,
                    default_value,
                    dynamic_enums_value,
                );
                match input_block {
                    serde_json::Value::Array(arr) => blocks.extend(arr),
                    _ => blocks.push(input_block),
                }
            }
        }
    }

    Ok(serde_json::Value::Array(blocks))
}

fn create_input_block(
    key: &str,
    schema: &ResumeFormField,
    required: bool,
    default_value: Option<serde_json::Value>,
    dynamic_enums_value: Option<serde_json::Value>,
) -> serde_json::Value {
    let placeholder = schema
        .description
        .as_deref()
        .filter(|desc| !desc.is_empty())
        .unwrap_or("Select an option");

    let title = schema.title.as_deref().unwrap_or(key);
    let title_with_required = if required {
        format!("{}*", title)
    } else {
        title.to_string()
    };

    // Handle boolean type
    if let FieldType::Boolean = schema.r#type {
        let initial_value = default_value
            .as_ref()
            .and_then(|v| v.as_bool())
            .or_else(|| {
                schema
                    .default
                    .as_ref()
                    .and_then(|default| default.as_bool())
            })
            .unwrap_or(false);

        let mut element = serde_json::json!({
            "type": "checkboxes",
            "options": [{
                "text": {
                    "type": "plain_text",
                    "text": title_with_required,
                    "emoji": true
                },
                "value": "true"
            }],
            "action_id": key
        });

        if initial_value {
            element["initial_options"] = serde_json::json!([
                {
                    "text": {
                        "type": "plain_text",
                        "text": title_with_required,
                        "emoji": true
                    },
                    "value": "true"
                }
            ]);
        }

        return serde_json::json!({
            "type": "input",
            "optional": !required,
            "element": element,
            "label": {
                "type": "plain_text",
                "text": title_with_required,
                "emoji": true
            }
        });
    }

    // Handle date-time format
    if let FieldType::String = schema.r#type {
        if schema.format.as_deref() == Some("date-time") {
            tracing::debug!("Date-time type");
            let now = chrono::Local::now();
            let current_date = now.format("%Y-%m-%d").to_string();
            let current_time = now.format("%H:%M").to_string();

            let (default_date, default_time) = default_value
                .as_ref()
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|parsed_date| {
                    (
                        parsed_date.format("%Y-%m-%d").to_string(),
                        parsed_date.format("%H:%M").to_string(),
                    )
                })
                .or_else(|| {
                    schema
                        .default
                        .as_ref()
                        .and_then(|default| {
                            chrono::DateTime::parse_from_rfc3339(default.as_str().unwrap()).ok()
                        })
                        .map(|parsed_date| {
                            (
                                parsed_date.format("%Y-%m-%d").to_string(),
                                parsed_date.format("%H:%M").to_string(),
                            )
                        })
                })
                .unwrap_or((current_date.clone(), current_time.clone()));

            return serde_json::json!([
                {
                    "type": "input",
                    "optional": !required,
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
                    "optional": !required,
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
    }

    // Handle enum type
    if let Some(enums) = &schema.r#enum {
        tracing::debug!("Enum type");
        let enums = dynamic_enums_value
            .as_ref()
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_else(|| enums.iter().map(|s| serde_json::json!(s)).collect());

        let initial_option = schema.default.as_ref().and_then(|default_value| {
            enums
                .iter()
                .find(|enum_value| enum_value == &&serde_json::json!(default_value))
                .map(|enum_value| {
                    serde_json::json!({
                        "text": {
                            "type": "plain_text",
                            "text": schema.enum_labels.as_ref()
                                .and_then(|labels| labels.get(enum_value.as_str().unwrap()))
                                .unwrap_or(&enum_value.as_str().unwrap().to_string()),
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
                            .and_then(|labels| labels.get(enum_value.as_str().unwrap()))
                            .unwrap_or(&enum_value.as_str().unwrap().to_string()),
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
            "optional": !required,
            "element": element,
            "label": {
                "type": "plain_text",
                "text": title_with_required,
                "emoji": true
            }
        })
    } else if let FieldType::Number | FieldType::Integer = schema.r#type {
        tracing::debug!("Number or integer type");
        // Handle number and integer types
        let initial_value = default_value
            .as_ref()
            .and_then(|v| v.as_f64())
            .or_else(|| schema.default.as_ref().and_then(|default| default.as_f64()))
            .unwrap_or(0.0);

        let action_id_suffix = if let FieldType::Number = schema.r#type {
            "_type_number"
        } else {
            "_type_integer"
        };

        serde_json::json!({
            "type": "input",
            "optional": !required,
            "element": {
                "type": "plain_text_input",
                "action_id": format!("{}{}", key, action_id_suffix),
                "initial_value": initial_value.to_string()
            },
            "label": {
                "type": "plain_text",
                "text": title_with_required,
                "emoji": true
            }
        })
    } else {
        tracing::debug!("Other type");
        // Handle other types as string
        let initial_value = default_value
            .as_ref()
            .and_then(|v| v.as_str())
            .or_else(|| schema.default.as_ref().and_then(|default| default.as_str()))
            .unwrap_or("");

        serde_json::json!({
            "type": "input",
            "optional": !required,
            "element": {
                "type": "plain_text_input",
                "action_id": key,
                "initial_value": initial_value.to_string()
            },
            "label": {
                "type": "plain_text",
                "text": title_with_required,
                "emoji": true
            }
        })
    }
}

fn parse_state_values(state: &State) -> HashMap<String, serde_json::Value> {
    state
        .values
        .iter()
        .flat_map(|(_, inputs)| {
            inputs.iter().filter_map(|(action_id, input)| {
                if action_id.ends_with("_date") {
                    process_datetime_inputs(action_id, input, &state)
                } else {
                    process_non_datetime_inputs(action_id, input)
                }
            })
        })
        .collect()
}

fn process_datetime_inputs(
    action_id: &str,
    input: &ValueInput,
    state: &State,
) -> Option<(String, serde_json::Value)> {
    let base_key = action_id.strip_suffix("_date").unwrap();
    let time_key = format!("{}_time", base_key);

    if let ValueInput::Datepicker { selected_date: Some(date) } = input {
        let matching_time = state
            .values
            .values()
            .flat_map(|inputs| inputs.get(&time_key))
            .find_map(|time_input| match time_input {
                ValueInput::Timepicker { selected_time: Some(time) } => Some(time),
                _ => None,
            });

        return matching_time.map(|time| {
            (
                base_key.to_string(),
                serde_json::json!(format!("{}T{}:00.000Z", date, time)),
            )
        });
    }
    None
}

fn process_non_datetime_inputs(
    action_id: &str,
    input: &ValueInput,
) -> Option<(String, serde_json::Value)> {
    match input {
        // Handle number and integer types first
        ValueInput::PlainTextInput { value }
            if action_id.ends_with("_type_number") || action_id.ends_with("_type_integer") =>
        {
            let base_action_id = action_id
                .trim_end_matches("_type_number")
                .trim_end_matches("_type_integer");
            value
                .as_ref()
                .and_then(|v| {
                    v.as_str().and_then(|s| s.parse::<f64>().ok()).map(|num| {
                        if action_id.ends_with("_type_integer") {
                            (
                                base_action_id.to_string(),
                                serde_json::json!(num.floor() as i64),
                            )
                        } else {
                            (base_action_id.to_string(), serde_json::json!(num))
                        }
                    })
                })
                .or_else(|| {
                    tracing::error!("Failed to parse value to number: {:?}", value);
                    None
                })
        }

        // Plain text input: Extracts the text value
        ValueInput::PlainTextInput { value } => value
            .as_ref()
            .map(|v| (action_id.to_string(), v.clone().into())),

        // Static select: Extracts the selected option's value
        ValueInput::StaticSelect { selected_option } => selected_option
            .as_ref()
            .map(|so| (action_id.to_string(), serde_json::json!(so.value))),

        // Radio buttons: Extracts the selected option's value
        ValueInput::RadioButtons { selected_option } => selected_option
            .as_ref()
            .map(|so| (action_id.to_string(), serde_json::json!(so.value))),

        // Checkboxes: Convert single "true/false" string to boolean true/false
        ValueInput::Checkboxes { selected_options } => selected_options.as_ref().map(|options| {
            let is_true = options.iter().any(|opt| opt.value == "true");
            (action_id.to_string(), serde_json::json!(is_true))
        }),

        // Default case: Unsupported types return `None`
        _ => None,
    }
}

async fn get_slack_token(db: &DB, slack_resource_path: &str, w_id: &str) -> anyhow::Result<String> {
    let slack_token = match sqlx::query!(
        "SELECT value, is_secret FROM variable WHERE path = $1",
        slack_resource_path
    )
    .fetch_optional(db)
    .await?
    {
        Some(row) => row,
        None => {
            return Err(anyhow::anyhow!("No slack token found"));
        }
    };

    if slack_token.is_secret {
        let mc = build_crypt(&db, w_id).await?;
        let bot_token = decrypt(&mc, slack_token.value)?;
        Ok(bot_token)
    } else {
        Ok(slack_token.value)
    }
}

// Sends a Slack message with a button that opens a modal
async fn send_slack_message(
    client: &Client,
    bot_token: &str,
    channel_id: &str,
    w_id: &str,
    job_id: Uuid,
    resource_path: &str,
    approver: Option<&str>,
    message: Option<&str>,
    flow_step_id: &str,
    default_args_json: Option<&serde_json::Value>,
    dynamic_enums_json: Option<&serde_json::Value>,
) -> Result<StatusCode, Box<dyn std::error::Error>> {
    let url = "https://slack.com/api/chat.postMessage";

    let mut value = serde_json::json!({
        "w_id": w_id,
        "job_id": job_id,
        "path": resource_path,
        "channel": channel_id,
        "flow_step_id": flow_step_id,
    });

    if let Some(approver) = approver {
        value["approver"] = serde_json::json!(approver);
    }

    if let Some(message) = message {
        value["message"] = serde_json::json!(message);
    }

    if let Some(default_args_json) = default_args_json {
        value["default_args_json"] = default_args_json.clone();
    }

    if let Some(dynamic_enums_json) = dynamic_enums_json {
        value["dynamic_enums_json"] = dynamic_enums_json.clone();
    }

    let payload = serde_json::json!({
        "channel": channel_id,
        "text": "A flow has been suspended. Please approve or reject the flow.",
        "blocks": [
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": "A flow has been suspended. Please approve or reject the flow."
                }
            },
            {
                "type": "actions",
                "elements": [
                    {
                        "type": "button",
                        "text": {
                            "type": "plain_text",
                            "text": "View"
                        },
                        "action_id": "open_modal",
                        "value": value.to_string()
                    }
                ]
            }
        ]
    });

    tracing::debug!("Payload: {:?}", payload);

    let response = client
        .post(url)
        .bearer_auth(bot_token)
        .json(&payload)
        .send()
        .await?;

    if response.status().is_success() {
        tracing::info!("Interactive Slack approval message sent successfully!");
        tracing::debug!("Response: {:#?}", response.text().await?);
    } else {
        tracing::error!(
            "Failed to send interactive Slack approval message: {}",
            response.text().await?
        );
    }

    Ok(StatusCode::OK)
}

async fn get_modal_blocks(
    db: DB,
    w_id: &str,
    job_id: Uuid,
    resume_id: u32,
    approver: Option<&str>,
    message: Option<&str>,
    trigger_id: &str,
    flow_step_id: Option<&str>,
    resource_path: &str,
    container: Container,
    default_args_json: Option<&serde_json::Value>,
    dynamic_enums_json: Option<&serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, Error> {
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

    if let Some(resume_schema) = schema {
        let hide_cancel = resume_schema.hide_cancel.unwrap_or(false);

        // if hide cancel is false add note to message
        if !hide_cancel {
            message_str.push_str("\n\n*NOTE*: closing this modal will cancel the workflow.\n\n");
        }

        // Convert message_str back to &str when needed
        let message_str_ref: &str = &message_str;

        if let Some(schema_obj) = resume_schema.resume_form {
            let inner_schema: ResumeSchema =
                serde_json::from_value(schema_obj.clone()).map_err(|e| {
                    tracing::error!("Failed to deserialize form schema: {:?}", e);
                    Error::BadRequest(
                        "Failed to deserialize resume form schema! Unsupported form field used."
                            .to_string(),
                    )
                })?;

            let blocks = transform_schemas(
                message_str_ref,
                Some(&inner_schema.schema.properties),
                &urls,
                Some(&inner_schema.schema.order),
                Some(&inner_schema.schema.required),
                default_args_json,
                dynamic_enums_json,
            )
            .await?;

            tracing::debug!("Slack Blocks: {:#?}", blocks);
            return Ok(axum::Json(construct_payload(
                blocks,
                hide_cancel,
                trigger_id,
                &urls.resume,
                resource_path,
                container,
            )));
        } else {
            tracing::debug!("No suspend form found!");
            let blocks = transform_schemas(
                message_str_ref,
                None,
                &urls,
                None,
                None,
                default_args_json,
                dynamic_enums_json,
            )
            .await?;
            return Ok(axum::Json(construct_payload(
                blocks,
                hide_cancel,
                trigger_id,
                &urls.resume,
                resource_path,
                container,
            )));
        }
    } else {
        Err(Error::BadRequest(
            "No approval form schema found.".to_string(),
        ))
    }
}

fn construct_payload(
    blocks: serde_json::Value,
    hide_cancel: bool,
    trigger_id: &str,
    resume_url: &str,
    resource_path: &str,
    container: Container,
) -> serde_json::Value {
    let mut view = serde_json::json!({
        "type": "modal",
        "callback_id": "submit_form",
        "notify_on_close": true,
        "title": {
            "type": "plain_text",
            "text": "Worfklow Suspended"
        },
        "blocks": blocks,
        "submit": {
            "type": "plain_text",
            "text": "Resume Workflow"
        },
        "private_metadata": serde_json::json!({ "resume_url": resume_url, "resource_path": resource_path, "container": container, "hide_cancel": hide_cancel }).to_string(),
    });

    if !hide_cancel {
        view["close"] = serde_json::json!({
            "type": "plain_text",
            "text": "Cancel Workflow"
        });
    }

    serde_json::json!({
        "trigger_id": trigger_id,
        "view": view
    })
}

async fn open_modal_with_blocks(
    client: &Client,
    bot_token: &str,
    trigger_id: &str,
    db: DB,
    w_id: &str,
    resource_path: &str,
    job_id: Uuid,
    approver: Option<&str>,
    message: Option<&str>,
    flow_step_id: Option<&str>,
    container: Container,
    default_args_json: Option<&serde_json::Value>,
    dynamic_enums_json: Option<&serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    let resume_id = rand::random::<u32>();
    let blocks_json = match get_modal_blocks(
        db,
        w_id,
        job_id,
        resume_id,
        approver,
        message,
        trigger_id,
        flow_step_id,
        resource_path,
        container,
        default_args_json,
        dynamic_enums_json,
    )
    .await
    {
        Ok(blocks) => blocks,
        Err(e) => axum::Json(serde_json::json!({
            "trigger_id": trigger_id,
            "view": {
                "type": "modal",
                "title": { "type": "plain_text", "text": "Error" },
                "blocks": [ { "type": "section", "text": { "type": "mrkdwn", "text": e.to_string() } } ]
            }
        })),
    };

    let blocks = &*blocks_json;

    tracing::debug!("Blocks: {:#?}", blocks);

    let url = "https://slack.com/api/views.open";

    let response = client
        .post(url)
        .bearer_auth(bot_token)
        .json(&blocks)
        .send()
        .await?;

    if response.status().is_success() {
        tracing::info!("Slack modal opened successfully!");
    } else {
        tracing::error!("Failed to open Slack modal: {}", response.text().await?);
    }

    Ok(())
}

async fn update_original_slack_message(
    action: &str,
    token: String,
    container: Container,
) -> Result<(), Error> {
    let message = if action == "resume" {
        "\n\n*Workflow has been resumed!* :white_check_mark:"
    } else {
        "\n\n*Workflow has been canceled!* :x:"
    };

    let final_blocks = vec![serde_json::json!({
        "type": "section",
        "text": {
            "type": "mrkdwn",
            "text": message
        }
    })];

    let payload = serde_json::json!({
        "channel": container.channel_id,
        "ts": container.message_ts,
        "text": message,
        "blocks": final_blocks,
        "mrkdwn": true // Enable markdown to support emojis
    });

    let client = Client::new();

    let response = client
        .post("https://slack.com/api/chat.update")
        .bearer_auth(token) // Use the token for authentication
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| Error::from(anyhow::Error::new(e)))?;

    if response.status().is_success() {
        tracing::debug!("Slack message updated successfully!");
    } else {
        tracing::error!(
            "Failed to update Slack message. Status: {}, Response: {:?}",
            response.status(),
            response
                .text()
                .await
                .map_err(|e| Error::from(anyhow::Error::new(e)))?
        );
    }

    Ok(())
}
