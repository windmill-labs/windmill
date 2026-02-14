use std::collections::HashMap;

use windmill_api_auth::ApiAuthed;
use windmill_trigger::trigger_helpers::{
    trigger_runnable_and_wait_for_raw_result_with_error_ctx, TriggerJobArgs,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::{types::Json as SqlxJson, FromRow};
use windmill_common::{
    error::{Error, Result},
    jobs::JobTriggerKind,
    triggers::{TriggerKind, TriggerMetadata},
    worker::to_raw_value,
    DB,
};
use windmill_queue::PushArgsOwned;

pub mod handler;
pub mod listener;

#[derive(Copy, Clone)]
pub struct WebsocketTrigger;

impl TriggerJobArgs for WebsocketTrigger {
    type Payload = String;
    const TRIGGER_KIND: TriggerKind = TriggerKind::Websocket;
    fn v1_payload_fn(payload: &Self::Payload) -> HashMap<String, Box<RawValue>> {
        HashMap::from([("msg".to_string(), to_raw_value(&payload))])
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WebsocketConfig {
    pub url: String,
    #[serde(default)]
    pub filters: Vec<SqlxJson<Box<RawValue>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_messages: Option<Vec<SqlxJson<Box<RawValue>>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_runnable_args: Option<SqlxJson<Box<RawValue>>>,
    #[serde(default)]
    pub can_return_message: bool,
    #[serde(default)]
    pub can_return_error_result: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsocketConfigRequest {
    url: String,
    filters: Vec<serde_json::Value>,
    initial_messages: Option<Vec<serde_json::Value>>,
    url_runnable_args: Option<serde_json::Value>,
    can_return_message: bool,
    can_return_error_result: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestWebsocketConfig {
    url: String,
    url_runnable_args: Option<serde_json::Value>,
}

pub fn value_to_args_hashmap(
    args: Option<&Box<RawValue>>,
) -> Result<HashMap<String, Box<RawValue>>> {
    let args = if let Some(args) = args {
        let args_map: Option<HashMap<String, serde_json::Value>> = serde_json::from_str(args.get())
            .map_err(|e| Error::BadRequest(format!("invalid json: {}", e)))?;

        args_map
            .unwrap_or_else(HashMap::new)
            .into_iter()
            .map(|(k, v)| {
                let raw_value = serde_json::value::to_raw_value(&v).map_err(|e| {
                    Error::BadRequest(format!("failed to convert to raw value: {}", e))
                })?;
                Ok((k, raw_value))
            })
            .collect::<Result<HashMap<String, Box<RawValue>>>>()
    } else {
        Ok(HashMap::new())
    }?;
    Ok(args)
}

pub async fn get_url_from_runnable_value(
    path: &str,
    is_flow: bool,
    db: &DB,
    authed: ApiAuthed,
    args: Option<&Box<RawValue>>,
    workspace_id: &str,
) -> Result<String> {
    tracing::info!(
        "Running {} {} to get WebSocket URL",
        if is_flow { "flow" } else { "script" },
        path
    );

    let args = value_to_args_hashmap(args)?;

    let result = trigger_runnable_and_wait_for_raw_result_with_error_ctx(
        db,
        None,
        authed,
        workspace_id,
        path,
        is_flow,
        PushArgsOwned { args, extra: None },
        None,
        None,
        None,
        "".to_string(), // doesn't matter as no retry/error handler
        TriggerMetadata::new(Some(path.to_owned()), JobTriggerKind::Websocket),
    )
    .await?;

    serde_json::from_str::<String>(result.get()).map_err(|_| {
        Error::BadConfig(format!(
            "{} {} did not return a string",
            if is_flow { "Flow" } else { "Script" },
            path,
        ))
    })
}
