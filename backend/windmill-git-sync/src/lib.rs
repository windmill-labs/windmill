/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use windmill_common::{scripts::ScriptHash, DB};

#[cfg(feature = "private")]
pub mod git_sync_ee;
pub mod git_sync_oss;

pub use git_sync_oss::{handle_deployment_metadata, handle_fork_branch_creation};

#[derive(Clone, Debug)]
pub enum DeployedObject {
    Script { hash: ScriptHash, path: String, parent_path: Option<String> },
    Flow { path: String, parent_path: Option<String>, version: i64 },
    App { path: String, version: i64, parent_path: Option<String> },
    RawApp { path: String, version: i64, parent_path: Option<String> },
    Folder { path: String },
    Resource { path: String, parent_path: Option<String> },
    Variable { path: String, parent_path: Option<String> },
    Schedule { path: String },
    ResourceType { path: String },
    User { email: String },
    Group { name: String },
    HttpTrigger { path: String, parent_path: Option<String> },
    WebsocketTrigger { path: String, parent_path: Option<String> },
    KafkaTrigger { path: String, parent_path: Option<String> },
    NatsTrigger { path: String, parent_path: Option<String> },
    PostgresTrigger { path: String, parent_path: Option<String> },
    MqttTrigger { path: String, parent_path: Option<String> },
    SqsTrigger { path: String, parent_path: Option<String> },
    GcpTrigger { path: String, parent_path: Option<String> },
    EmailTrigger { path: String, parent_path: Option<String> },
    Settings { setting_type: String },
    Key { key_type: String },
}

impl DeployedObject {
    pub fn get_path(&self) -> String {
        match self {
            DeployedObject::Script { path, .. } => path.to_owned(),
            DeployedObject::Flow { path, .. } => path.to_owned(),
            DeployedObject::App { path, .. } => path.to_owned(),
            DeployedObject::RawApp { path, .. } => path.to_owned(),
            DeployedObject::Folder { path, .. } => path.to_owned(),
            DeployedObject::Resource { path, .. } => path.to_owned(),
            DeployedObject::Variable { path, .. } => path.to_owned(),
            DeployedObject::Schedule { path, .. } => path.to_owned(),
            DeployedObject::ResourceType { path, .. } => path.to_owned(),
            DeployedObject::User { email } => format!("users/{email}"),
            DeployedObject::Group { name } => format!("groups/{name}"),
            DeployedObject::HttpTrigger { path, .. } => path.to_owned(),
            DeployedObject::WebsocketTrigger { path, .. } => path.to_owned(),
            DeployedObject::KafkaTrigger { path, .. } => path.to_owned(),
            DeployedObject::NatsTrigger { path, .. } => path.to_owned(),
            DeployedObject::PostgresTrigger { path, .. } => path.to_owned(),
            DeployedObject::MqttTrigger { path, .. } => path.to_owned(),
            DeployedObject::SqsTrigger { path, .. } => path.to_owned(),
            DeployedObject::GcpTrigger { path, .. } => path.to_owned(),
            DeployedObject::EmailTrigger { path, .. } => path.to_owned(),
            DeployedObject::Settings { .. } => "settings.yaml".to_string(),
            DeployedObject::Key { .. } => "encryption_key.yaml".to_string(),
        }
    }

    pub fn get_ignore_regex_filter(&self) -> bool {
        match self {
            Self::User { .. }
            | Self::Group { .. }
            | Self::ResourceType { .. }
            | Self::Settings { .. }
            | Self::Key { .. } => true,
            _ => false,
        }
    }

    pub fn get_parent_path(&self) -> Option<String> {
        match self {
            DeployedObject::Script { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::Flow { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::App { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::RawApp { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::Folder { .. } => None,
            DeployedObject::Resource { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::Variable { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::Schedule { .. } => None,
            DeployedObject::ResourceType { .. } => None,
            DeployedObject::User { .. } => None,
            DeployedObject::Group { .. } => None,
            DeployedObject::HttpTrigger { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::WebsocketTrigger { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::KafkaTrigger { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::NatsTrigger { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::PostgresTrigger { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::MqttTrigger { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::SqsTrigger { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::GcpTrigger { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::EmailTrigger { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::Settings { .. } => None,
            DeployedObject::Key { .. } => None,
        }
    }

    pub fn get_kind(&self) -> String {
        match self {
            DeployedObject::Script { .. } => "script",
            DeployedObject::Flow { .. } => "flow",
            DeployedObject::App { .. } => "app",
            DeployedObject::RawApp { .. } => "raw_app",
            DeployedObject::Folder { .. } => "folder",
            DeployedObject::Resource { .. } => "resource",
            DeployedObject::Variable { .. } => "variable",
            DeployedObject::Schedule { .. } => "schedule",
            DeployedObject::ResourceType { .. } => "resource_type",
            DeployedObject::User { .. } => "user",
            DeployedObject::Group { .. } => "group",
            DeployedObject::HttpTrigger { .. } => "http_trigger",
            DeployedObject::WebsocketTrigger { .. } => "websocket_trigger",
            DeployedObject::KafkaTrigger { .. } => "kafka_trigger",
            DeployedObject::NatsTrigger { .. } => "nats_trigger",
            DeployedObject::PostgresTrigger { .. } => "postgres_trigger",
            DeployedObject::MqttTrigger { .. } => "mqtt_trigger",
            DeployedObject::SqsTrigger { .. } => "sqs_trigger",
            DeployedObject::GcpTrigger { .. } => "gcp_trigger",
            DeployedObject::EmailTrigger { .. } => "email_trigger",
            DeployedObject::Settings { .. } => "settings",
            DeployedObject::Key { .. } => "key",
        }.to_string()
    }
}
