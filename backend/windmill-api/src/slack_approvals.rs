use axum::{
    extract::{Form, Path, Query},
    Extension,
};
use hyper::StatusCode;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::types::Uuid;
use std::collections::HashMap;
use windmill_common::error::Error;
use windmill_common::variables::get_secret_value_as_admin;

use crate::db::{ApiAuthed, DB};
use crate::jobs::{QueryApprover, ResumeUrls};
use crate::{
    approvals::{
        extract_w_id_from_resume_url, handle_resume_action, ApprovalFormDetails, FieldType,
        MessageFormat, QueryButtonText, QueryDefaultArgsJson, QueryDynamicEnumJson,
        QueryFlowStepId, QueryMessage, ResumeFormField, ResumeSchema,
    },
    auth::OptTokened,
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

#[derive(Deserialize)]
pub struct QueryResourcePath {
    slack_resource_path: String,
}

#[derive(Deserialize)]
pub struct QueryChannelId {
    channel_id: String,
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
    resume_button_text: Option<String>,
    cancel_button_text: Option<String>,
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
    opt_tokened: OptTokened,
    Extension(db): Extension<DB>,
    Form(form_data): Form<SlackFormData>,
) -> Result<StatusCode, Error> {
    tracing::debug!("Form data: {:#?}", form_data);
    let payload: Payload = serde_json::from_str(&form_data.payload)?;
    tracing::debug!("Payload: {:#?}", payload);

    match payload.r#type {
        PayloadType::ViewSubmission => {
            handle_submission(authed, opt_tokened, db, &payload, "resume").await?
        }
        PayloadType::ViewClosed => {
            handle_submission(authed, opt_tokened, db, &payload, "cancel").await?
        }
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
                                parsed_value.resume_button_text.as_deref(),
                                parsed_value.cancel_button_text.as_deref(),
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
    Query(button_text): Query<QueryButtonText>,
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
        button_text.resume_button_text.as_deref(),
        button_text.cancel_button_text.as_deref(),
    )
    .await
    .map_err(|e| Error::BadRequest(e.to_string()))?;

    Ok(StatusCode::OK)
}

async fn handle_submission(
    authed: Option<ApiAuthed>,
    opt_tokened: OptTokened,
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

    // Use the common handler to process the resume/cancel action
    handle_resume_action(
        authed,
        opt_tokened,
        db.clone(),
        &resume_url,
        state_json,
        action,
    )
    .await?;

    let w_id = extract_w_id_from_resume_url(&resume_url)?;
    let slack_token = get_slack_token(&db, &resource_path, w_id).await?;
    update_original_slack_message(action, slack_token, container).await?;
    Ok(())
}

async fn transform_schemas(
    text: &str,
    properties: Option<HashMap<String, ResumeFormField>>,
    urls: &ResumeUrls,
    order: Option<Vec<String>>,
    required: Option<Vec<String>>,
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
        for key in order.unwrap_or_default() {
            if let Some(schema) = properties.get(&key) {
                let is_required = required.as_ref().map_or(false, |r| r.contains(&key));

                let default_value = default_args_json.and_then(|json| json.get(&key).cloned());
                let dynamic_enums_value =
                    dynamic_enums_json.and_then(|json| json.get(&key).cloned());

                let input_block = create_input_block(
                    &key,
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
    get_secret_value_as_admin(db, w_id, slack_resource_path)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))
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
    resume_button_text: Option<&str>,
    cancel_button_text: Option<&str>,
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

    if let Some(resume_button_text) = resume_button_text {
        value["resume_button_text"] = serde_json::json!(resume_button_text);
    }

    if let Some(cancel_button_text) = cancel_button_text {
        value["cancel_button_text"] = serde_json::json!(cancel_button_text);
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
    resume_button_text: Option<&str>,
    cancel_button_text: Option<&str>,
) -> Result<axum::Json<serde_json::Value>, Error> {
    let approval_details = crate::approvals::get_approval_form_details(
        db,
        w_id,
        job_id,
        flow_step_id,
        resume_id,
        approver,
        message,
        MessageFormat::Slack,
    )
    .await?;

    let ApprovalFormDetails { message_str, urls, schema } = approval_details;

    // Get the card content
    let card_content = transform_schemas(
        &message_str,
        schema
            .as_ref()
            .and_then(|s| s.resume_form.as_ref())
            .map(|f| {
                let inner_schema: ResumeSchema = serde_json::from_value(f.clone()).unwrap();
                inner_schema.schema.properties
            }),
        &urls,
        schema
            .as_ref()
            .and_then(|s| s.resume_form.as_ref())
            .map(|f| {
                let inner_schema: ResumeSchema = serde_json::from_value(f.clone()).unwrap();
                inner_schema.schema.order
            }),
        schema
            .as_ref()
            .and_then(|s| s.resume_form.as_ref())
            .map(|f| {
                let inner_schema: ResumeSchema = serde_json::from_value(f.clone()).unwrap();
                inner_schema.schema.required
            }),
        default_args_json,
        dynamic_enums_json,
    )
    .await?;

    tracing::debug!("Slack Blocks: {:#?}", card_content);
    Ok(axum::Json(construct_payload(
        card_content,
        schema.as_ref().and_then(|s| s.hide_cancel).unwrap_or(false),
        trigger_id,
        &urls.resume,
        resource_path,
        container,
        resume_button_text,
        cancel_button_text,
    )))
}

fn construct_payload(
    blocks: serde_json::Value,
    hide_cancel: bool,
    trigger_id: &str,
    resume_url: &str,
    resource_path: &str,
    container: Container,
    resume_button_text: Option<&str>,
    cancel_button_text: Option<&str>,
) -> serde_json::Value {
    let mut view = serde_json::json!({
        "type": "modal",
        "callback_id": "submit_form",
        "notify_on_close": true,
        "title": {
            "type": "plain_text",
            "text": "Workflow Suspended"
        },
        "blocks": blocks,
        "submit": {
            "type": "plain_text",
            "text": resume_button_text.unwrap_or("Resume Workflow")
        },
        "private_metadata": serde_json::json!({ "resume_url": resume_url, "resource_path": resource_path, "container": container, "hide_cancel": hide_cancel }).to_string(),
    });

    if !hide_cancel {
        view["close"] = serde_json::json!({
            "type": "plain_text",
            "text": cancel_button_text.unwrap_or("Cancel Workflow")
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
    resume_button_text: Option<&str>,
    cancel_button_text: Option<&str>,
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
        resume_button_text,
        cancel_button_text,
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
