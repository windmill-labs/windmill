use crate::approvals::{
    get_approval_form, ApprovalFormDetails, QueryDefaultArgsJson, QueryDynamicEnumJson,
    QueryFlowStepId, QueryMessage, ResumeFormRow,
};
use crate::db::{ApiAuthed, DB};
use crate::jobs::{QueryApprover, ResumeUrls};
use axum::{
    extract::{Path, Query},
    Extension,
};
use http::StatusCode;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;
use windmill_common::ee::send_notification;
use windmill_common::error::Error;
use windmill_common::teams_ee::{get_global_teams_bot_token, retrieve_channels};

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

#[derive(Debug, Deserialize)]
struct ResumeSchema {
    schema: Schema,
}

#[derive(Debug, Deserialize)]
struct Schema {
    order: Vec<String>,
    required: Vec<String>,
    properties: HashMap<String, ResumeFormField>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum FieldType {
    Boolean,
    String,
    Number,
    Integer,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
struct ResumeFormField {
    r#type: FieldType,
    format: Option<String>,
    default: Option<Value>,
    description: Option<String>,
    title: Option<String>,
    r#enum: Option<Vec<String>>,
    #[serde(rename = "enumLabels")]
    enum_labels: Option<HashMap<String, String>>,
    nullable: Option<bool>,
    placeholder: Option<String>,
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
        db.clone(),
        w_id.as_str(),
        job_id,
        Some(flow_step_id.flow_step_id.as_str()),
        resume_id,
        approver.approver.as_deref(),
        message.message.as_deref(),
    )
    .await?;

    let ApprovalFormDetails { message_str, urls, schema } = approval_details;

    // Get the card content
    let card_content = transform_schemas(
        message_str,
        schema,
        default_args_json.default_args_json.as_ref(),
        dynamic_enums_json.dynamic_enums_json.as_ref(),
        &urls,
    )?;

    // Get the conversation ID for the specified channel
    let workspace_team_internal_id = sqlx::query_scalar!(
        "SELECT teams_team_id FROM workspace_settings WHERE workspace_id = $1",
        &w_id
    )
    .fetch_one(&db)
    .await?;

    let teams_info = retrieve_channels(&db).await?;
    let workspace_team = teams_info
        .iter()
        .find(|t| t.team_internal_id == workspace_team_internal_id.as_deref().unwrap_or_default())
        .ok_or_else(|| Error::BadRequest("Team not found".to_string()))?;

    let channel = workspace_team
        .channels
        .iter()
        .find(|c| c.channel_name == channel_name.channel_name)
        .ok_or_else(|| Error::BadRequest("Channel not found".to_string()))?;

    // Format the card for Teams
    let teams_card = serde_json::json!({
        "type": "message",
        "attachments": [
            {
                "contentType": "application/vnd.microsoft.card.adaptive",
                "content": {
                    "type": "AdaptiveCard",
                    "$schema": "https://adaptivecards.io/schemas/adaptive-card.json",
                    "version": "1.6",
                    "body": card_content["body"],
                    "actions": card_content["actions"]
                }
            }
        ],
        "conversation": {
            "id": channel.channel_id
        }
    });

    // Send the card to Teams
    let url = format!(
        "https://smba.trafficmanager.net/amer/v3/conversations/{}/activities",
        channel.channel_id
    );

    send_notification(&url, &teams_card, Some(&teams_bot_token))
        .await
        .map_err(|e| {
            tracing::error!("Failed to send Teams card: {}", e);
            Error::InternalErr("Failed to send Teams card".to_string())
        })?;

    Ok(StatusCode::OK)
}

fn transform_schemas(
    message_str: String,
    schema: Option<ResumeFormRow>,
    default_args_json: Option<&serde_json::Value>,
    dynamic_enums_json: Option<&serde_json::Value>,
    urls: &ResumeUrls,
) -> Result<Value, Error> {
    let mut card = serde_json::json!({
        "type": "AdaptiveCard",
        "version": "1.0",
        "body": [
            {
                "type": "TextBlock",
                "text": "Workflow Suspended",
                "size": "Large",
                "weight": "Bolder",
                "wrap": true
            },
            {
                "type": "TextBlock",
                "text": message_str,
                "wrap": true,
                "spacing": "Medium"
            },
            {
                "type": "TextBlock",
                "text": "Input Requested",
                "size": "Medium",
                "weight": "Bolder",
                "spacing": "Large",
                "wrap": true
            },
            {
                "type": "TextBlock",
                "text": format!("[Flow suspension details]({})", urls.approvalPage),
                "spacing": "Medium",
                "wrap": true
            }
        ],
        "actions": []
    });

    if let Some(resume_schema) = schema {
        if let Some(schema_obj) = resume_schema.resume_form {
            let inner_schema: ResumeSchema =
                serde_json::from_value(schema_obj.clone()).map_err(|e| {
                    tracing::error!("Failed to deserialize form schema: {:?}", e);
                    Error::BadRequest(
                        "Failed to deserialize resume form schema! Unsupported form field used."
                            .to_string(),
                    )
                })?;

            // Add form fields based on order
            for key in inner_schema.schema.order {
                if let Some(field) = inner_schema.schema.properties.get(&key) {
                    let is_required = inner_schema.schema.required.contains(&key);
                    let title = field.title.as_deref().unwrap_or(&key);
                    let description = field.description.as_deref().unwrap_or("");
                    let placeholder = field.placeholder.as_deref().unwrap_or("");

                    // Get default value from default_args_json if provided, otherwise use schema default
                    tracing::info!("default_args_json: {:?}", default_args_json);
                    let default_value = default_args_json
                        .and_then(|json_str| {
                            serde_json::from_str::<Value>(json_str.as_str().unwrap_or_default())
                                .ok()
                                .and_then(|json| json.get(&key).cloned())
                        })
                        .or_else(|| field.default.clone());
                    tracing::info!("default_value: {:?}", default_value);

                    // Get enum values from dynamic_enums_json if provided, otherwise use schema enum
                    tracing::info!("dynamic_enums_json: {:?}", dynamic_enums_json);
                    let enum_values = dynamic_enums_json
                        .and_then(|json_str| {
                            serde_json::from_str::<Value>(json_str.as_str().unwrap_or_default())
                                .ok()
                                .and_then(|json| json.get(&key).and_then(|v| v.as_array().cloned()))
                        })
                        .or_else(|| {
                            field
                                .r#enum
                                .as_ref()
                                .map(|e| e.iter().map(|s| serde_json::json!(s)).collect())
                        });
                    tracing::info!("enum_values: {:?}", enum_values);
                    // Add a separator before each field
                    card["body"]
                        .as_array_mut()
                        .unwrap()
                        .push(serde_json::json!({
                            "type": "TextBlock",
                            "text": "",
                            "spacing": "Small"
                        }));

                    let label = if !description.is_empty() {
                        format!("{}: {}", title, description)
                    } else {
                        title.to_string()
                    };

                    match field.r#type {
                        FieldType::Boolean => {
                            card["body"]
                                .as_array_mut()
                                .unwrap()
                                .push(serde_json::json!({
                                    "type": "Input.Toggle",
                                    "id": key,
                                    "label": label,
                                    "value": default_value.unwrap_or_else(|| Value::Bool(false)),
                                    "isRequired": is_required
                                }));
                        }
                        FieldType::String => {
                            if let Some(enums) = enum_values {
                                // Create a choice set for enum values
                                let choices = enums.iter().map(|value| {
                                    let value_str = value.as_str().unwrap_or_default();
                                    serde_json::json!({
                                        "title": field.enum_labels.as_ref()
                                            .and_then(|labels| labels.get(&value_str.to_string()))
                                            .map_or(value_str.to_string(), |v| v.clone()),
                                        "value": value
                                    })
                                }).collect::<Vec<_>>();

                                card["body"].as_array_mut().unwrap().push(serde_json::json!({
                                    "type": "Input.ChoiceSet",
                                    "id": key,
                                    "label": label,
                                    "placeholder": if !placeholder.is_empty() { placeholder } else { description },
                                    "choices": choices,
                                    "isRequired": is_required,
                                    "value": default_value.unwrap_or_else(|| Value::String("".to_string()))
                                }));
                            } else {
                                card["body"].as_array_mut().unwrap().push(serde_json::json!({
                                    "type": "Input.Text",
                                    "id": key,
                                    "label": label,
                                    "placeholder": if !placeholder.is_empty() { placeholder } else { description },
                                    "isRequired": is_required,
                                    "value": default_value.unwrap_or_else(|| Value::String("".to_string()))
                                }));
                            }
                        }
                        FieldType::Number | FieldType::Integer => {
                            card["body"].as_array_mut().unwrap().push(serde_json::json!({
                                "type": "Input.Number",
                                "id": key,
                                "label": label,
                                "placeholder": if !placeholder.is_empty() { placeholder } else { description },
                                "isRequired": is_required,
                                "value": default_value.unwrap_or_else(|| Value::Number(0.into()))
                            }));
                        }
                        _ => {
                            tracing::warn!("Unsupported field type: {:?}", field.r#type);
                        }
                    }
                }
            }

            // Add submit and cancel actions
            card["actions"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({
                    "type": "Action.Submit",
                    "title": "Resume Workflow",
                    "data": {
                        "action": "resume",
                        "private_metadata": urls.resume,
                        "msteams": {
                            "type": "messageBack",
                            "displayText": "Resuming workflow...",
                            "text": "Resuming workflow",
                            "value": {
                                "action": "resume"
                            }
                        }
                    }
                }));

            if !resume_schema.hide_cancel.unwrap_or(false) {
                card["actions"]
                    .as_array_mut()
                    .unwrap()
                    .push(serde_json::json!({
                        "type": "Action.Submit",
                        "title": "Cancel Workflow",
                        "data": {
                            "action": "cancel",
                            "private_metadata": urls.cancel,
                            "msteams": {
                                "type": "messageBack",
                                "displayText": "Cancelling workflow...",
                                "text": "Cancelling workflow",
                                "value": {
                                    "action": "cancel"
                                }
                            }
                        }
                    }));
            }
        }
    }

    tracing::debug!("Generated Teams card: {:#?}", card);
    tracing::debug!(
        "Generated Teams card JSON:\n{}",
        serde_json::to_string_pretty(&card).unwrap()
    );
    Ok(card)
}
