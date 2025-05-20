/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use sqlx::{Pool, Postgres};

use windmill_common::scripts::ScriptHash;

pub mod git_sync_ee;
pub mod git_sync_oss;

#[cfg(feature = "private")]
pub use git_sync_ee::handle_deployment_metadata;
#[cfg(not(feature = "private"))]
pub use git_sync_oss::handle_deployment_metadata;
pub type DB = Pool<Postgres>;

#[derive(Clone, Debug)]
pub enum DeployedObject {
    Script { hash: ScriptHash, path: String, parent_path: Option<String> },
    Flow { path: String, parent_path: Option<String>, version: i64 },
    App { path: String, version: i64, parent_path: Option<String> },
    Folder { path: String },
    Resource { path: String, parent_path: Option<String> },
    Variable { path: String, parent_path: Option<String> },
    Schedule { path: String },
    ResourceType { path: String },
    User { email: String },
    Group { name: String },
    HttpTrigger { path: String },
    WebsocketTrigger { path: String },
    KafkaTrigger { path: String },
    NatsTrigger { path: String },
    PostgresTrigger { path: String },
    MqttTrigger { path: String },
    SqsTrigger { path: String },
    GcpTrigger { path: String },
}

impl DeployedObject {
    pub fn get_path(&self) -> String {
        match self {
            DeployedObject::Script { path, .. } => path.to_owned(),
            DeployedObject::Flow { path, .. } => path.to_owned(),
            DeployedObject::App { path, .. } => path.to_owned(),
            DeployedObject::Folder { path, .. } => path.to_owned(),
            DeployedObject::Resource { path, .. } => path.to_owned(),
            DeployedObject::Variable { path, .. } => path.to_owned(),
            DeployedObject::Schedule { path, .. } => path.to_owned(),
            DeployedObject::ResourceType { path, .. } => path.to_owned(),
            DeployedObject::User { email } => format!("users/{email}"),
            DeployedObject::Group { name } => format!("groups/{name}"),
            DeployedObject::HttpTrigger { path } => path.to_owned(),
            DeployedObject::WebsocketTrigger { path } => path.to_owned(),
            DeployedObject::KafkaTrigger { path } => path.to_owned(),
            DeployedObject::NatsTrigger { path } => path.to_owned(),
            DeployedObject::PostgresTrigger { path } => path.to_owned(),
            DeployedObject::MqttTrigger { path } => path.to_owned(),
            DeployedObject::SqsTrigger { path } => path.to_owned(),
            DeployedObject::GcpTrigger { path } => path.to_owned(),
        }
    }

    pub fn get_ignore_regex_filter(&self) -> bool {
        match self {
            Self::User { .. } | Self::Group { .. } | Self::ResourceType { .. } => true,
            _ => false,
        }
    }

    pub fn get_parent_path(&self) -> Option<String> {
        match self {
            DeployedObject::Script { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::Flow { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::App { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::Folder { .. } => None,
            DeployedObject::Resource { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::Variable { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::Schedule { .. } => None,
            DeployedObject::ResourceType { .. } => None,
            DeployedObject::User { .. } => None,
            DeployedObject::Group { .. } => None,
            DeployedObject::HttpTrigger { .. } => None,
            DeployedObject::WebsocketTrigger { .. } => None,
            DeployedObject::KafkaTrigger { .. } => None,
            DeployedObject::NatsTrigger { .. } => None,
            DeployedObject::PostgresTrigger { .. } => None,
            DeployedObject::MqttTrigger { .. } => None,
            DeployedObject::SqsTrigger { .. } => None,
            DeployedObject::GcpTrigger { .. } => None,
        }
    }
}
