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
pub use git_sync_ee::{
    enqueue_git_pull_dry_run, enqueue_git_pull_job, handle_deployment_metadata,
    handle_deployment_metadata_batch, handle_fork_branch_creation, persist_auto_pull_state,
    reconcile_and_enqueue_pull, reconcile_fork_branch_pull, record_auto_pull_failure,
    tally_deployed_object_changes,
};

#[cfg(not(feature = "private"))]
pub use git_sync_oss::{
    handle_deployment_metadata, handle_deployment_metadata_batch, handle_fork_branch_creation,
    tally_deployed_object_changes,
};

#[derive(Clone, Debug)]
pub enum DeployedObject {
    Script {
        hash: ScriptHash,
        path: String,
        parent_path: Option<String>,
    },
    Flow {
        path: String,
        parent_path: Option<String>,
        version: i64,
    },
    App {
        path: String,
        version: i64,
        parent_path: Option<String>,
    },
    RawApp {
        path: String,
        version: i64,
        parent_path: Option<String>,
    },
    Folder {
        path: String,
    },
    Resource {
        path: String,
        parent_path: Option<String>,
    },
    Variable {
        path: String,
        parent_path: Option<String>,
    },
    Schedule {
        path: String,
    },
    ResourceType {
        path: String,
    },
    User {
        email: String,
    },
    Group {
        name: String,
    },
    HttpTrigger {
        path: String,
        parent_path: Option<String>,
    },
    WebsocketTrigger {
        path: String,
        parent_path: Option<String>,
    },
    KafkaTrigger {
        path: String,
        parent_path: Option<String>,
    },
    NatsTrigger {
        path: String,
        parent_path: Option<String>,
    },
    PostgresTrigger {
        path: String,
        parent_path: Option<String>,
    },
    MqttTrigger {
        path: String,
        parent_path: Option<String>,
    },
    AmqpTrigger {
        path: String,
        parent_path: Option<String>,
    },
    SqsTrigger {
        path: String,
        parent_path: Option<String>,
    },
    GcpTrigger {
        path: String,
        parent_path: Option<String>,
    },
    AzureTrigger {
        path: String,
        parent_path: Option<String>,
    },
    EmailTrigger {
        path: String,
        parent_path: Option<String>,
    },
    Settings {
        setting_type: String,
    },
    Key {
        key_type: String,
    },
    WorkspaceDependencies {
        path: String,
    },
    /// A single data table migration, identified by `<datatable>/<timestamp>_<name>`.
    DatatableMigration {
        path: String,
    },
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
            DeployedObject::AmqpTrigger { path, .. } => path.to_owned(),
            DeployedObject::SqsTrigger { path, .. } => path.to_owned(),
            DeployedObject::GcpTrigger { path, .. } => path.to_owned(),
            DeployedObject::AzureTrigger { path, .. } => path.to_owned(),
            DeployedObject::EmailTrigger { path, .. } => path.to_owned(),
            DeployedObject::Settings { .. } => "settings.yaml".to_string(),
            DeployedObject::Key { .. } => "encryption_key.yaml".to_string(),
            DeployedObject::WorkspaceDependencies { path, .. } => path.to_owned(),
            DeployedObject::DatatableMigration { path } => path.to_owned(),
        }
    }

    /// The repo-relative path git sync syncs on: what the sync item carries, and
    /// therefore what the CLI turns into `--extra-includes` globs and `git add`
    /// pathspecs.
    ///
    /// It only differs from [`Self::get_path`] for data table migrations, whose
    /// object path (`<datatable>/<version>_<name>`, the `workspace_diff` key) is
    /// relative to the `migrations/datatable/` prefix the workspace export writes
    /// them under. Without the prefix the CLI derives globs that match no file,
    /// so nothing gets staged and the push silently commits nothing.
    pub fn get_git_sync_path(&self) -> String {
        match self {
            DeployedObject::DatatableMigration { path } => {
                format!("migrations/datatable/{path}")
            }
            _ => self.get_path(),
        }
    }

    /// Whether the object skips the repo's include/exclude path filters. True for
    /// every kind that lives outside the `f/`/`u/` path namespaces those filters
    /// are written against — the object-type filter is what governs them instead.
    pub fn get_ignore_regex_filter(&self) -> bool {
        match self {
            Self::User { .. }
            | Self::Group { .. }
            | Self::ResourceType { .. }
            | Self::Settings { .. }
            | Self::Key { .. }
            | Self::WorkspaceDependencies { .. }
            | Self::DatatableMigration { .. } => true,
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
            DeployedObject::AmqpTrigger { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::SqsTrigger { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::GcpTrigger { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::AzureTrigger { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::EmailTrigger { parent_path, .. } => parent_path.to_owned(),
            DeployedObject::Settings { .. } => None,
            DeployedObject::Key { .. } => None,
            DeployedObject::WorkspaceDependencies { .. } => None,
            DeployedObject::DatatableMigration { .. } => None,
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
            DeployedObject::AmqpTrigger { .. } => "amqp_trigger",
            DeployedObject::SqsTrigger { .. } => "sqs_trigger",
            DeployedObject::GcpTrigger { .. } => "gcp_trigger",
            DeployedObject::AzureTrigger { .. } => "azure_trigger",
            DeployedObject::EmailTrigger { .. } => "email_trigger",
            DeployedObject::Settings { .. } => "settings",
            DeployedObject::Key { .. } => "key",
            DeployedObject::WorkspaceDependencies { .. } => "workspace_dependencies",
            DeployedObject::DatatableMigration { .. } => "datatable_migration",
        }
        .to_string()
    }
}

/// Record, from the request that made it, that a rename left `vacated` empty.
/// Only its path and kind are read, so build it naming the path the rename left.
///
/// A deploy whose metadata is handled by its dependency job needs this: that job
/// runs at an unknown remove from the write, and the row it would write is
/// deleted as soon as the two workspaces agree on the path — so a claim it made
/// could reappear later against an item the parent has since recreated. Call
/// once the rename has committed; the tally reads what the path holds now.
pub async fn tally_rename_vacated_path(
    db: &DB,
    w_id: &str,
    vacated: DeployedObject,
) -> windmill_common::error::Result<()> {
    tally_deployed_object_changes(
        w_id,
        &vacated,
        db,
        None,
        windmill_common::deploy_origin::current(),
    )
    .await
}

/// Item kinds whose `workspace_diff` path is the `path` column of a table named
/// after the kind. Interpolated into SQL, so it must stay a hardcoded allowlist —
/// and, unlike the `sqlx::query!` arms below, a wrong name here is not a compile
/// error, so `workspace_comparison.rs` sweeps this list against a live database.
pub const PATH_KEYED_TABLES: &[&str] = &[
    "resource",
    "variable",
    "schedule",
    "http_trigger",
    "websocket_trigger",
    "kafka_trigger",
    "nats_trigger",
    "postgres_trigger",
    "mqtt_trigger",
    "amqp_trigger",
    "sqs_trigger",
    "gcp_trigger",
    "azure_trigger",
    "email_trigger",
];

/// What the deploy event that just committed did to `path`: [`DeployEventKind::Write`]
/// if the workspace still holds an item there, [`DeployEventKind::Delete`] if it does
/// not. `None` for a kind this does not know how to probe, so an unmapped kind is
/// recorded as no evidence rather than as a deletion.
///
/// Existence mirrors the comparison's (`compare_two_*`): an archived script or flow
/// counts as absent. Runs on `&db` (no RLS) — the caller is a post-commit tally, not
/// a user-facing read.
pub async fn probe_deploy_event_kind(
    db: &DB,
    w_id: &str,
    kind: &str,
    path: &str,
) -> windmill_common::error::Result<Option<windmill_common::deploy_origin::DeployEventKind>> {
    use windmill_common::deploy_origin::DeployEventKind;

    let exists: Option<bool> = match kind {
        "script" => Some(
            sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM script WHERE workspace_id = $1 AND path = $2 AND archived = false)",
                w_id,
                path
            )
            .fetch_one(db)
            .await?
            .unwrap_or(false),
        ),
        "flow" => Some(
            sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM flow WHERE workspace_id = $1 AND path = $2 AND archived = false)",
                w_id,
                path
            )
            .fetch_one(db)
            .await?
            .unwrap_or(false),
        ),
        // Raw apps live in the `app` table too, keyed by path like a regular app.
        "app" | "raw_app" => Some(
            sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM app WHERE workspace_id = $1 AND path = $2)",
                w_id,
                path
            )
            .fetch_one(db)
            .await?
            .unwrap_or(false),
        ),
        "folder" => {
            let name = path.strip_prefix("f/").unwrap_or(path);
            Some(
                sqlx::query_scalar!(
                    "SELECT EXISTS(SELECT 1 FROM folder WHERE workspace_id = $1 AND name = $2)",
                    w_id,
                    name
                )
                .fetch_one(db)
                .await?
                .unwrap_or(false),
            )
        }
        "resource_type" => Some(
            sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM resource_type WHERE workspace_id = $1 AND name = $2)",
                w_id,
                path
            )
            .fetch_one(db)
            .await?
            .unwrap_or(false),
        ),
        // Identified by (datatable, timestamp), which survives a rename of the
        // migration's `name` segment; the full path does not.
        "datatable_migration" => match path
            .split_once('/')
            .and_then(|(dt, rest)| Some((dt, rest.split_once('_')?.0.parse::<i64>().ok()?)))
        {
            Some((datatable, timestamp)) => Some(
                sqlx::query_scalar!(
                    "SELECT EXISTS(SELECT 1 FROM datatable_migrations \
                     WHERE workspace_id = $1 AND datatable = $2 AND timestamp = $3)",
                    w_id,
                    datatable,
                    timestamp
                )
                .fetch_one(db)
                .await?
                .unwrap_or(false),
            ),
            None => None,
        },
        k if PATH_KEYED_TABLES.contains(&k) => {
            // SAFETY: `k` comes from the hardcoded PATH_KEYED_TABLES allowlist.
            let sql =
                format!("SELECT EXISTS(SELECT 1 FROM {k} WHERE workspace_id = $1 AND path = $2)");
            Some(
                sqlx::query_scalar(&sql)
                    .bind(w_id)
                    .bind(path)
                    .fetch_one(db)
                    .await?,
            )
        }
        _ => None,
    };

    Ok(exists.map(|exists| {
        if exists {
            DeployEventKind::Write
        } else {
            DeployEventKind::Delete
        }
    }))
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

    // --- DeployedObject::get_git_sync_path tests ---

    #[test]
    fn test_get_git_sync_path_datatable_migration_is_repo_relative() {
        let obj = DeployedObject::DatatableMigration {
            path: "mydb/20260101000000_add_users".to_string(),
        };
        // The object path keys `workspace_diff`; the git-sync path must carry the
        // export's prefix or the CLI stages nothing.
        assert_eq!(obj.get_path(), "mydb/20260101000000_add_users");
        assert_eq!(
            obj.get_git_sync_path(),
            "migrations/datatable/mydb/20260101000000_add_users"
        );
    }

    #[test]
    fn test_get_git_sync_path_defaults_to_object_path() {
        let obj = DeployedObject::Script {
            hash: ScriptHash(123),
            path: "f/folder/script".to_string(),
            parent_path: None,
        };
        assert_eq!(obj.get_git_sync_path(), obj.get_path());
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
    fn test_ignore_regex_filter_datatable_migration() {
        // `migrations/datatable/**` is outside the `f/`/`u/` namespaces the path
        // filters are written against; the `datatablemigration` include type is
        // what governs these.
        let obj = DeployedObject::DatatableMigration {
            path: "mydb/20260101000000_add_users".to_string(),
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
        assert_eq!(
            obj.get_parent_path(),
            Some("f/folder/old_script".to_string())
        );
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
        let obj = DeployedObject::Flow { path: "test".to_string(), parent_path: None, version: 1 };
        assert_eq!(obj.get_kind(), "flow");
    }

    #[test]
    fn test_get_kind_app() {
        let obj = DeployedObject::App { path: "test".to_string(), version: 1, parent_path: None };
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
            DeployedObject::WebsocketTrigger { path: "t".to_string(), parent_path: None }
                .get_kind(),
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
            DeployedObject::AmqpTrigger { path: "t".to_string(), parent_path: None }.get_kind(),
            "amqp_trigger"
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
            DeployedObject::AzureTrigger { path: "t".to_string(), parent_path: None }.get_kind(),
            "azure_trigger"
        );
        assert_eq!(
            DeployedObject::EmailTrigger { path: "t".to_string(), parent_path: None }.get_kind(),
            "email_trigger"
        );
    }
}
