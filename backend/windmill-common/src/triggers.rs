use lazy_static::lazy_static;
use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use std::fmt;
use strum_macros::EnumIter;

use crate::jobs::JobTriggerKind;

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash, EnumIter)]
#[sqlx(type_name = "TRIGGER_KIND", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TriggerKind {
    Webhook,
    Http,
    Websocket,
    Kafka,
    DefaultEmail,
    Email,
    Nats,
    Mqtt,
    Sqs,
    Postgres,
    Gcp,
}

impl TriggerKind {
    pub fn to_key(&self) -> String {
        match self {
            TriggerKind::Webhook => "webhook".to_string(),
            TriggerKind::Http => "http".to_string(),
            TriggerKind::Websocket => "websocket".to_string(),
            TriggerKind::Kafka => "kafka".to_string(),
            TriggerKind::Email => "email".to_string(),
            TriggerKind::DefaultEmail => "email".to_string(), // to the user we also show kind email for default email
            TriggerKind::Nats => "nats".to_string(),
            TriggerKind::Mqtt => "mqtt".to_string(),
            TriggerKind::Sqs => "sqs".to_string(),
            TriggerKind::Postgres => "postgres".to_string(),
            TriggerKind::Gcp => "gcp".to_string(),
        }
    }
}

impl fmt::Display for TriggerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TriggerKind::Webhook => "webhook",
            TriggerKind::Http => "http",
            TriggerKind::Websocket => "websocket",
            TriggerKind::Kafka => "kafka",
            TriggerKind::Email => "email",
            TriggerKind::DefaultEmail => "default_email",
            TriggerKind::Nats => "nats",
            TriggerKind::Mqtt => "mqtt",
            TriggerKind::Sqs => "sqs",
            TriggerKind::Postgres => "postgres",
            TriggerKind::Gcp => "gcp",
        };
        write!(f, "{}", s)
    }
}

#[derive(Eq, PartialEq, Hash)]
pub enum HubOrWorkspaceId {
    Hub,
    WorkspaceId(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy)]
pub struct RunnableFormat {
    pub version: RunnableFormatVersion,
    pub has_preprocessor: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy)]
pub enum RunnableFormatVersion {
    V1,
    V2,
}

pub type RunnableFormatCacheKey = (HubOrWorkspaceId, i64, TriggerKind);

lazy_static! {
    pub static ref RUNNABLE_FORMAT_VERSION_CACHE: Cache<RunnableFormatCacheKey, RunnableFormat> =
        Cache::new(1000);
}

#[derive(Debug, Clone)]
pub struct TriggerMetadata {
    pub trigger_path: Option<String>,
    pub trigger_kind: JobTriggerKind,
}

impl TriggerMetadata {
    pub fn new(trigger_path: Option<String>, trigger_kind: JobTriggerKind) -> TriggerMetadata {
        TriggerMetadata { trigger_path, trigger_kind }
    }
}
