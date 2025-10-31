use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{types::Json as SqlxJson, FromRow};
use std::{collections::HashMap, fmt::Debug};
use windmill_common::{
    error::Result,
    mailbox::{Mailbox, MailboxType},
    DB,
};
use windmill_queue::PushArgsOwned;

#[cfg(all(feature = "smtp", feature = "enterprise", feature = "private"))]
pub mod email;
#[cfg(all(feature = "gcp_trigger", feature = "enterprise", feature = "private"))]
pub mod gcp;
#[cfg(feature = "http_trigger")]
pub mod http;
#[cfg(all(feature = "kafka", feature = "enterprise", feature = "private"))]
pub mod kafka;
#[cfg(feature = "mqtt_trigger")]
pub mod mqtt;
#[cfg(all(feature = "nats", feature = "enterprise", feature = "private"))]
pub mod nats;
#[cfg(feature = "postgres_trigger")]
pub mod postgres;
#[cfg(all(feature = "sqs_trigger", feature = "enterprise", feature = "private"))]
pub mod sqs;
#[cfg(feature = "websocket")]
pub mod websocket;

mod handler;
mod listener;
pub mod trigger_helpers;

pub use handler::TriggerCrud;
pub use handler::{generate_trigger_routers, get_triggers_count_internal, TriggersCount};
pub use listener::start_all_listeners;
#[allow(unused)]
pub use listener::Listener;

use crate::{
    db::ApiAuthed,
    triggers::trigger_helpers::{push_runnable_into_queue, ActionToTake},
};

pub const COMMON_TRIGGER_FIELDS: [&'static str; 9] = [
    "workspace_id",
    "path",
    "script_path",
    "action_to_take",
    "is_flow",
    "edited_by",
    "email",
    "edited_at",
    "extra_perms",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardTriggerQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub path: Option<String>,
    pub is_flow: Option<bool>,
    pub path_start: Option<String>,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct BaseTrigger {
    pub workspace_id: String,
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    pub edited_by: String,
    pub email: String,
    pub action_to_take: ActionToTake,
    pub edited_at: DateTime<Utc>,
    pub extra_perms: Option<serde_json::Value>,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct ServerState {
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_server_ping: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct TriggerErrorHandling {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handler_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handler_args: Option<SqlxJson<HashMap<String, serde_json::Value>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<sqlx::types::Json<windmill_common::flows::Retry>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Trigger<T>
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow>,
{
    #[serde(flatten)]
    pub base: BaseTrigger,

    #[serde(flatten)]
    pub config: T,

    #[serde(flatten)]
    pub server_state: Option<ServerState>,

    #[serde(flatten)]
    pub error_handling: TriggerErrorHandling,
}

impl<T> FromRow<'_, sqlx::postgres::PgRow> for Trigger<T>
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow>,
{
    fn from_row(row: &sqlx::postgres::PgRow) -> std::result::Result<Self, sqlx::Error> {
        Ok(Trigger {
            base: BaseTrigger::from_row(row)?,
            config: T::from_row(row)?,
            server_state: ServerState::from_row(row).ok(),
            error_handling: TriggerErrorHandling::from_row(row)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseTriggerData {
    pub path: String,
    pub script_path: String,
    pub action_to_take: ActionToTake,
    pub is_flow: bool,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerData<T: Debug> {
    #[serde(flatten)]
    pub base: BaseTriggerData,

    #[serde(flatten)]
    pub config: T,

    #[serde(flatten)]
    pub error_handling: TriggerErrorHandling,
}

impl StandardTriggerQuery {
    pub fn offset(&self) -> i64 {
        let page = self.page.unwrap_or(0);
        let per_page = self.per_page.unwrap_or(100);
        (page * per_page) as i64
    }

    pub fn limit(&self) -> i64 {
        self.per_page.unwrap_or(100) as i64
    }
}

impl Default for StandardTriggerQuery {
    fn default() -> Self {
        Self { page: Some(0), per_page: Some(100), path: None, path_start: None, is_flow: None }
    }
}

async fn process_mailbox_for_trigger(
    db: DB,
    workspace_id: String,
    authed: ApiAuthed,
    mailbox_id: String,
    trigger_path: String,
    script_path: String,
    is_flow: bool,
    error_handling: Option<TriggerErrorHandling>,
) -> Result<()> {
    #[derive(sqlx::FromRow)]
    struct MailboxPayload {
        message_id: i64,
        payload: serde_json::Value,
    }
    let payloads = sqlx::query_as!(
        MailboxPayload,
        r#"
            SELECT 
                message_id,
                payload
            FROM 
                mailbox 
            WHERE 
                mailbox_id = $1 AND 
                workspace_id = $2
        "#,
        &mailbox_id,
        &workspace_id
    )
    .fetch_all(&db)
    .await?;

    if payloads.is_empty() {
        return Ok(());
    }

    tracing::info!(
        "Processing {} mailbox messages for trigger {} in workspace {}",
        payloads.len(),
        trigger_path,
        workspace_id
    );

    let (retry, error_handler_path, error_handler_args) = match error_handling.as_ref() {
        Some(error_handling) => (
            error_handling.retry.as_ref(),
            error_handling.error_handler_path.as_deref(),
            error_handling.error_handler_args.as_ref(),
        ),
        None => (None, None, None),
    };

    let mailbox = Mailbox::open(Some(&mailbox_id), MailboxType::Trigger, &workspace_id);

    for MailboxPayload { message_id, payload } in payloads {
        let args = serde_json::from_value::<PushArgsOwned>(payload);

        if let Ok(args) = args {
            let result = push_runnable_into_queue(
                &db,
                None,
                authed.clone(),
                &workspace_id,
                &script_path,
                is_flow,
                args,
                retry,
                error_handler_path,
                error_handler_args,
                trigger_path.clone(),
                None,
            )
            .await;

            if let Ok((_, mut tx, ..)) = result {
                let delete_result = mailbox.delete(message_id, &mut *tx).await;
                if let Ok(_) = delete_result {
                    let transaction_result = tx.commit().await;
                    if let Ok(_) = transaction_result {
                        tracing::info!("");
                    }
                }
            }
        }
    }

    Ok(())
}
