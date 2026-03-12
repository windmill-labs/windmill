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

#[cfg(feature = "private")]
pub use git_sync_ee::{handle_deployment_metadata, handle_fork_branch_creation};

#[cfg(not(feature = "private"))]
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
    WorkspaceDependencies { path: String },
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
            DeployedObject::WorkspaceDependencies { path, .. } => path.to_owned(),
        }
    }

    pub fn get_ignore_regex_filter(&self) -> bool {
        match self {
            Self::User { .. }
            | Self::Group { .. }
            | Self::ResourceType { .. }
            | Self::Settings { .. }
            | Self::Key { .. }
            | Self::WorkspaceDependencies { .. } => true,
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
            DeployedObject::WorkspaceDependencies { .. } => None,
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
            DeployedObject::WorkspaceDependencies { .. } => "workspace_dependencies",
        }
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windmill_common::scripts::ScriptHash;

    // --- DeployedObject::get_path tests ---

    #[test]
    fn test_get_path_script() {
        let obj = DeployedObject::Script {
            hash: ScriptHash(123),
            path: "f/folder/script".to_string(),
            parent_path: None,
        };
        assert_eq!(obj.get_path(), "f/folder/script");
    }

    #[test]
    fn test_get_path_flow() {
        let obj = DeployedObject::Flow {
            path: "f/folder/flow".to_string(),
            parent_path: Some("f/folder/old_flow".to_string()),
            version: 1,
        };
        assert_eq!(obj.get_path(), "f/folder/flow");
    }

    #[test]
    fn test_get_path_user() {
        let obj = DeployedObject::User { email: "user@example.com".to_string() };
        assert_eq!(obj.get_path(), "users/user@example.com");
    }

    #[test]
    fn test_get_path_group() {
        let obj = DeployedObject::Group { name: "admins".to_string() };
        assert_eq!(obj.get_path(), "groups/admins");
    }

    #[test]
    fn test_get_path_settings() {
        let obj = DeployedObject::Settings { setting_type: "error_handler".to_string() };
        assert_eq!(obj.get_path(), "settings.yaml");
    }

    #[test]
    fn test_get_path_key() {
        let obj = DeployedObject::Key { key_type: "encryption".to_string() };
        assert_eq!(obj.get_path(), "encryption_key.yaml");
    }

    #[test]
    fn test_get_path_workspace_dependencies() {
        let obj = DeployedObject::WorkspaceDependencies {
            path: "workspace-dependencies/python".to_string(),
        };
        assert_eq!(obj.get_path(), "workspace-dependencies/python");
    }

    // --- DeployedObject::get_ignore_regex_filter tests ---

    #[test]
    fn test_ignore_regex_filter_user() {
        let obj = DeployedObject::User { email: "user@example.com".to_string() };
        assert!(obj.get_ignore_regex_filter());
    }

    #[test]
    fn test_ignore_regex_filter_group() {
        let obj = DeployedObject::Group { name: "admins".to_string() };
        assert!(obj.get_ignore_regex_filter());
    }

    #[test]
    fn test_ignore_regex_filter_resource_type() {
        let obj = DeployedObject::ResourceType { path: "postgresql".to_string() };
        assert!(obj.get_ignore_regex_filter());
    }

    #[test]
    fn test_ignore_regex_filter_settings() {
        let obj = DeployedObject::Settings { setting_type: "error_handler".to_string() };
        assert!(obj.get_ignore_regex_filter());
    }

    #[test]
    fn test_ignore_regex_filter_key() {
        let obj = DeployedObject::Key { key_type: "encryption".to_string() };
        assert!(obj.get_ignore_regex_filter());
    }

    #[test]
    fn test_ignore_regex_filter_workspace_dependencies() {
        let obj = DeployedObject::WorkspaceDependencies {
            path: "workspace-dependencies/python".to_string(),
        };
        assert!(obj.get_ignore_regex_filter());
    }

    #[test]
    fn test_ignore_regex_filter_script() {
        let obj = DeployedObject::Script {
            hash: ScriptHash(123),
            path: "f/folder/script".to_string(),
            parent_path: None,
        };
        assert!(!obj.get_ignore_regex_filter());
    }

    #[test]
    fn test_ignore_regex_filter_flow() {
        let obj = DeployedObject::Flow {
            path: "f/folder/flow".to_string(),
            parent_path: None,
            version: 1,
        };
        assert!(!obj.get_ignore_regex_filter());
    }

    // --- DeployedObject::get_parent_path tests ---

    #[test]
    fn test_get_parent_path_script_with_parent() {
        let obj = DeployedObject::Script {
            hash: ScriptHash(123),
            path: "f/folder/script".to_string(),
            parent_path: Some("f/folder/old_script".to_string()),
        };
        assert_eq!(obj.get_parent_path(), Some("f/folder/old_script".to_string()));
    }

    #[test]
    fn test_get_parent_path_script_without_parent() {
        let obj = DeployedObject::Script {
            hash: ScriptHash(123),
            path: "f/folder/script".to_string(),
            parent_path: None,
        };
        assert_eq!(obj.get_parent_path(), None);
    }

    #[test]
    fn test_get_parent_path_folder() {
        let obj = DeployedObject::Folder { path: "f/folder".to_string() };
        assert_eq!(obj.get_parent_path(), None);
    }

    #[test]
    fn test_get_parent_path_workspace_dependencies() {
        let obj = DeployedObject::WorkspaceDependencies {
            path: "workspace-dependencies/python".to_string(),
        };
        assert_eq!(obj.get_parent_path(), None);
    }

    // --- DeployedObject::get_kind tests ---

    #[test]
    fn test_get_kind_script() {
        let obj = DeployedObject::Script {
            hash: ScriptHash(123),
            path: "test".to_string(),
            parent_path: None,
        };
        assert_eq!(obj.get_kind(), "script");
    }

    #[test]
    fn test_get_kind_flow() {
        let obj = DeployedObject::Flow {
            path: "test".to_string(),
            parent_path: None,
            version: 1,
        };
        assert_eq!(obj.get_kind(), "flow");
    }

    #[test]
    fn test_get_kind_app() {
        let obj = DeployedObject::App {
            path: "test".to_string(),
            version: 1,
            parent_path: None,
        };
        assert_eq!(obj.get_kind(), "app");
    }

    #[test]
    fn test_get_kind_workspace_dependencies() {
        let obj = DeployedObject::WorkspaceDependencies {
            path: "workspace-dependencies/python".to_string(),
        };
        assert_eq!(obj.get_kind(), "workspace_dependencies");
    }

    #[test]
    fn test_get_kind_all_triggers() {
        assert_eq!(
            DeployedObject::HttpTrigger { path: "t".to_string(), parent_path: None }.get_kind(),
            "http_trigger"
        );
        assert_eq!(
            DeployedObject::WebsocketTrigger { path: "t".to_string(), parent_path: None }.get_kind(),
            "websocket_trigger"
        );
        assert_eq!(
            DeployedObject::KafkaTrigger { path: "t".to_string(), parent_path: None }.get_kind(),
            "kafka_trigger"
        );
        assert_eq!(
            DeployedObject::NatsTrigger { path: "t".to_string(), parent_path: None }.get_kind(),
            "nats_trigger"
        );
        assert_eq!(
            DeployedObject::PostgresTrigger { path: "t".to_string(), parent_path: None }.get_kind(),
            "postgres_trigger"
        );
        assert_eq!(
            DeployedObject::MqttTrigger { path: "t".to_string(), parent_path: None }.get_kind(),
            "mqtt_trigger"
        );
        assert_eq!(
            DeployedObject::SqsTrigger { path: "t".to_string(), parent_path: None }.get_kind(),
            "sqs_trigger"
        );
        assert_eq!(
            DeployedObject::GcpTrigger { path: "t".to_string(), parent_path: None }.get_kind(),
            "gcp_trigger"
        );
        assert_eq!(
            DeployedObject::EmailTrigger { path: "t".to_string(), parent_path: None }.get_kind(),
            "email_trigger"
        );
    }
}
