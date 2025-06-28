pub use progenitor_client::{ByteStream, Error, ResponseValue};
#[allow(unused_imports)]
use progenitor_client::{encode_path, RequestBuilderExt};
#[allow(unused_imports)]
use reqwest::header::{HeaderMap, HeaderValue};
pub mod types {
    use serde::{Deserialize, Serialize};
    #[allow(unused_imports)]
    use std::convert::TryFrom;
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum AiProvider {
        #[serde(rename = "openai")]
        Openai,
        #[serde(rename = "anthropic")]
        Anthropic,
        #[serde(rename = "mistral")]
        Mistral,
        #[serde(rename = "deepseek")]
        Deepseek,
        #[serde(rename = "googleai")]
        Googleai,
        #[serde(rename = "groq")]
        Groq,
        #[serde(rename = "openrouter")]
        Openrouter,
        #[serde(rename = "customai")]
        Customai,
    }
    impl From<&AiProvider> for AiProvider {
        fn from(value: &AiProvider) -> Self {
            value.clone()
        }
    }
    impl ToString for AiProvider {
        fn to_string(&self) -> String {
            match *self {
                Self::Openai => "openai".to_string(),
                Self::Anthropic => "anthropic".to_string(),
                Self::Mistral => "mistral".to_string(),
                Self::Deepseek => "deepseek".to_string(),
                Self::Googleai => "googleai".to_string(),
                Self::Groq => "groq".to_string(),
                Self::Openrouter => "openrouter".to_string(),
                Self::Customai => "customai".to_string(),
            }
        }
    }
    impl std::str::FromStr for AiProvider {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "openai" => Ok(Self::Openai),
                "anthropic" => Ok(Self::Anthropic),
                "mistral" => Ok(Self::Mistral),
                "deepseek" => Ok(Self::Deepseek),
                "googleai" => Ok(Self::Googleai),
                "groq" => Ok(Self::Groq),
                "openrouter" => Ok(Self::Openrouter),
                "customai" => Ok(Self::Customai),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for AiProvider {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for AiProvider {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for AiProvider {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct AiResource {
        pub path: String,
        pub provider: AiProvider,
    }
    impl From<&AiResource> for AiResource {
        fn from(value: &AiResource) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct AppHistory {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deployment_msg: Option<String>,
        pub version: i64,
    }
    impl From<&AppHistory> for AppHistory {
        fn from(value: &AppHistory) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct AppWithLastVersion {
        pub created_at: chrono::DateTime<chrono::offset::Utc>,
        pub created_by: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub custom_path: Option<String>,
        pub execution_mode: AppWithLastVersionExecutionMode,
        pub extra_perms: std::collections::HashMap<String, bool>,
        pub id: i64,
        pub path: String,
        pub policy: Policy,
        pub summary: String,
        pub value: std::collections::HashMap<String, serde_json::Value>,
        pub versions: Vec<i64>,
        pub workspace_id: String,
    }
    impl From<&AppWithLastVersion> for AppWithLastVersion {
        fn from(value: &AppWithLastVersion) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum AppWithLastVersionExecutionMode {
        #[serde(rename = "viewer")]
        Viewer,
        #[serde(rename = "publisher")]
        Publisher,
        #[serde(rename = "anonymous")]
        Anonymous,
    }
    impl From<&AppWithLastVersionExecutionMode> for AppWithLastVersionExecutionMode {
        fn from(value: &AppWithLastVersionExecutionMode) -> Self {
            value.clone()
        }
    }
    impl ToString for AppWithLastVersionExecutionMode {
        fn to_string(&self) -> String {
            match *self {
                Self::Viewer => "viewer".to_string(),
                Self::Publisher => "publisher".to_string(),
                Self::Anonymous => "anonymous".to_string(),
            }
        }
    }
    impl std::str::FromStr for AppWithLastVersionExecutionMode {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "viewer" => Ok(Self::Viewer),
                "publisher" => Ok(Self::Publisher),
                "anonymous" => Ok(Self::Anonymous),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for AppWithLastVersionExecutionMode {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for AppWithLastVersionExecutionMode {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for AppWithLastVersionExecutionMode {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct AppWithLastVersionWDraft {
        #[serde(flatten)]
        pub app_with_last_version: AppWithLastVersion,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub draft: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub draft_only: Option<bool>,
    }
    impl From<&AppWithLastVersionWDraft> for AppWithLastVersionWDraft {
        fn from(value: &AppWithLastVersionWDraft) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct AuditLog {
        pub action_kind: AuditLogActionKind,
        pub id: i64,
        pub operation: AuditLogOperation,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub parameters: std::collections::HashMap<String, serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub resource: Option<String>,
        pub timestamp: chrono::DateTime<chrono::offset::Utc>,
        pub username: String,
    }
    impl From<&AuditLog> for AuditLog {
        fn from(value: &AuditLog) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum AuditLogActionKind {
        Created,
        Updated,
        Delete,
        Execute,
    }
    impl From<&AuditLogActionKind> for AuditLogActionKind {
        fn from(value: &AuditLogActionKind) -> Self {
            value.clone()
        }
    }
    impl ToString for AuditLogActionKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Created => "Created".to_string(),
                Self::Updated => "Updated".to_string(),
                Self::Delete => "Delete".to_string(),
                Self::Execute => "Execute".to_string(),
            }
        }
    }
    impl std::str::FromStr for AuditLogActionKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "Created" => Ok(Self::Created),
                "Updated" => Ok(Self::Updated),
                "Delete" => Ok(Self::Delete),
                "Execute" => Ok(Self::Execute),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for AuditLogActionKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for AuditLogActionKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for AuditLogActionKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum AuditLogOperation {
        #[serde(rename = "jobs.run")]
        JobsRun,
        #[serde(rename = "jobs.run.script")]
        JobsRunScript,
        #[serde(rename = "jobs.run.preview")]
        JobsRunPreview,
        #[serde(rename = "jobs.run.flow")]
        JobsRunFlow,
        #[serde(rename = "jobs.run.flow_preview")]
        JobsRunFlowPreview,
        #[serde(rename = "jobs.run.script_hub")]
        JobsRunScriptHub,
        #[serde(rename = "jobs.run.dependencies")]
        JobsRunDependencies,
        #[serde(rename = "jobs.run.identity")]
        JobsRunIdentity,
        #[serde(rename = "jobs.run.noop")]
        JobsRunNoop,
        #[serde(rename = "jobs.flow_dependencies")]
        JobsFlowDependencies,
        #[serde(rename = "jobs")]
        Jobs,
        #[serde(rename = "jobs.cancel")]
        JobsCancel,
        #[serde(rename = "jobs.force_cancel")]
        JobsForceCancel,
        #[serde(rename = "jobs.disapproval")]
        JobsDisapproval,
        #[serde(rename = "jobs.delete")]
        JobsDelete,
        #[serde(rename = "account.delete")]
        AccountDelete,
        #[serde(rename = "ai.request")]
        AiRequest,
        #[serde(rename = "resources.create")]
        ResourcesCreate,
        #[serde(rename = "resources.update")]
        ResourcesUpdate,
        #[serde(rename = "resources.delete")]
        ResourcesDelete,
        #[serde(rename = "resource_types.create")]
        ResourceTypesCreate,
        #[serde(rename = "resource_types.update")]
        ResourceTypesUpdate,
        #[serde(rename = "resource_types.delete")]
        ResourceTypesDelete,
        #[serde(rename = "schedule.create")]
        ScheduleCreate,
        #[serde(rename = "schedule.setenabled")]
        ScheduleSetenabled,
        #[serde(rename = "schedule.edit")]
        ScheduleEdit,
        #[serde(rename = "schedule.delete")]
        ScheduleDelete,
        #[serde(rename = "scripts.create")]
        ScriptsCreate,
        #[serde(rename = "scripts.update")]
        ScriptsUpdate,
        #[serde(rename = "scripts.archive")]
        ScriptsArchive,
        #[serde(rename = "scripts.delete")]
        ScriptsDelete,
        #[serde(rename = "users.create")]
        UsersCreate,
        #[serde(rename = "users.delete")]
        UsersDelete,
        #[serde(rename = "users.update")]
        UsersUpdate,
        #[serde(rename = "users.login")]
        UsersLogin,
        #[serde(rename = "users.login_failure")]
        UsersLoginFailure,
        #[serde(rename = "users.logout")]
        UsersLogout,
        #[serde(rename = "users.accept_invite")]
        UsersAcceptInvite,
        #[serde(rename = "users.decline_invite")]
        UsersDeclineInvite,
        #[serde(rename = "users.token.create")]
        UsersTokenCreate,
        #[serde(rename = "users.token.delete")]
        UsersTokenDelete,
        #[serde(rename = "users.add_to_workspace")]
        UsersAddToWorkspace,
        #[serde(rename = "users.add_global")]
        UsersAddGlobal,
        #[serde(rename = "users.setpassword")]
        UsersSetpassword,
        #[serde(rename = "users.impersonate")]
        UsersImpersonate,
        #[serde(rename = "users.leave_workspace")]
        UsersLeaveWorkspace,
        #[serde(rename = "oauth.login")]
        OauthLogin,
        #[serde(rename = "oauth.login_failure")]
        OauthLoginFailure,
        #[serde(rename = "oauth.signup")]
        OauthSignup,
        #[serde(rename = "variables.create")]
        VariablesCreate,
        #[serde(rename = "variables.delete")]
        VariablesDelete,
        #[serde(rename = "variables.update")]
        VariablesUpdate,
        #[serde(rename = "flows.create")]
        FlowsCreate,
        #[serde(rename = "flows.update")]
        FlowsUpdate,
        #[serde(rename = "flows.delete")]
        FlowsDelete,
        #[serde(rename = "flows.archive")]
        FlowsArchive,
        #[serde(rename = "apps.create")]
        AppsCreate,
        #[serde(rename = "apps.update")]
        AppsUpdate,
        #[serde(rename = "apps.delete")]
        AppsDelete,
        #[serde(rename = "folder.create")]
        FolderCreate,
        #[serde(rename = "folder.update")]
        FolderUpdate,
        #[serde(rename = "folder.delete")]
        FolderDelete,
        #[serde(rename = "folder.add_owner")]
        FolderAddOwner,
        #[serde(rename = "folder.remove_owner")]
        FolderRemoveOwner,
        #[serde(rename = "group.create")]
        GroupCreate,
        #[serde(rename = "group.delete")]
        GroupDelete,
        #[serde(rename = "group.edit")]
        GroupEdit,
        #[serde(rename = "group.adduser")]
        GroupAdduser,
        #[serde(rename = "group.removeuser")]
        GroupRemoveuser,
        #[serde(rename = "igroup.create")]
        IgroupCreate,
        #[serde(rename = "igroup.delete")]
        IgroupDelete,
        #[serde(rename = "igroup.adduser")]
        IgroupAdduser,
        #[serde(rename = "igroup.removeuser")]
        IgroupRemoveuser,
        #[serde(rename = "variables.decrypt_secret")]
        VariablesDecryptSecret,
        #[serde(rename = "workspaces.edit_command_script")]
        WorkspacesEditCommandScript,
        #[serde(rename = "workspaces.edit_deploy_to")]
        WorkspacesEditDeployTo,
        #[serde(rename = "workspaces.edit_auto_invite_domain")]
        WorkspacesEditAutoInviteDomain,
        #[serde(rename = "workspaces.edit_webhook")]
        WorkspacesEditWebhook,
        #[serde(rename = "workspaces.edit_copilot_config")]
        WorkspacesEditCopilotConfig,
        #[serde(rename = "workspaces.edit_error_handler")]
        WorkspacesEditErrorHandler,
        #[serde(rename = "workspaces.create")]
        WorkspacesCreate,
        #[serde(rename = "workspaces.update")]
        WorkspacesUpdate,
        #[serde(rename = "workspaces.archive")]
        WorkspacesArchive,
        #[serde(rename = "workspaces.unarchive")]
        WorkspacesUnarchive,
        #[serde(rename = "workspaces.delete")]
        WorkspacesDelete,
    }
    impl From<&AuditLogOperation> for AuditLogOperation {
        fn from(value: &AuditLogOperation) -> Self {
            value.clone()
        }
    }
    impl ToString for AuditLogOperation {
        fn to_string(&self) -> String {
            match *self {
                Self::JobsRun => "jobs.run".to_string(),
                Self::JobsRunScript => "jobs.run.script".to_string(),
                Self::JobsRunPreview => "jobs.run.preview".to_string(),
                Self::JobsRunFlow => "jobs.run.flow".to_string(),
                Self::JobsRunFlowPreview => "jobs.run.flow_preview".to_string(),
                Self::JobsRunScriptHub => "jobs.run.script_hub".to_string(),
                Self::JobsRunDependencies => "jobs.run.dependencies".to_string(),
                Self::JobsRunIdentity => "jobs.run.identity".to_string(),
                Self::JobsRunNoop => "jobs.run.noop".to_string(),
                Self::JobsFlowDependencies => "jobs.flow_dependencies".to_string(),
                Self::Jobs => "jobs".to_string(),
                Self::JobsCancel => "jobs.cancel".to_string(),
                Self::JobsForceCancel => "jobs.force_cancel".to_string(),
                Self::JobsDisapproval => "jobs.disapproval".to_string(),
                Self::JobsDelete => "jobs.delete".to_string(),
                Self::AccountDelete => "account.delete".to_string(),
                Self::AiRequest => "ai.request".to_string(),
                Self::ResourcesCreate => "resources.create".to_string(),
                Self::ResourcesUpdate => "resources.update".to_string(),
                Self::ResourcesDelete => "resources.delete".to_string(),
                Self::ResourceTypesCreate => "resource_types.create".to_string(),
                Self::ResourceTypesUpdate => "resource_types.update".to_string(),
                Self::ResourceTypesDelete => "resource_types.delete".to_string(),
                Self::ScheduleCreate => "schedule.create".to_string(),
                Self::ScheduleSetenabled => "schedule.setenabled".to_string(),
                Self::ScheduleEdit => "schedule.edit".to_string(),
                Self::ScheduleDelete => "schedule.delete".to_string(),
                Self::ScriptsCreate => "scripts.create".to_string(),
                Self::ScriptsUpdate => "scripts.update".to_string(),
                Self::ScriptsArchive => "scripts.archive".to_string(),
                Self::ScriptsDelete => "scripts.delete".to_string(),
                Self::UsersCreate => "users.create".to_string(),
                Self::UsersDelete => "users.delete".to_string(),
                Self::UsersUpdate => "users.update".to_string(),
                Self::UsersLogin => "users.login".to_string(),
                Self::UsersLoginFailure => "users.login_failure".to_string(),
                Self::UsersLogout => "users.logout".to_string(),
                Self::UsersAcceptInvite => "users.accept_invite".to_string(),
                Self::UsersDeclineInvite => "users.decline_invite".to_string(),
                Self::UsersTokenCreate => "users.token.create".to_string(),
                Self::UsersTokenDelete => "users.token.delete".to_string(),
                Self::UsersAddToWorkspace => "users.add_to_workspace".to_string(),
                Self::UsersAddGlobal => "users.add_global".to_string(),
                Self::UsersSetpassword => "users.setpassword".to_string(),
                Self::UsersImpersonate => "users.impersonate".to_string(),
                Self::UsersLeaveWorkspace => "users.leave_workspace".to_string(),
                Self::OauthLogin => "oauth.login".to_string(),
                Self::OauthLoginFailure => "oauth.login_failure".to_string(),
                Self::OauthSignup => "oauth.signup".to_string(),
                Self::VariablesCreate => "variables.create".to_string(),
                Self::VariablesDelete => "variables.delete".to_string(),
                Self::VariablesUpdate => "variables.update".to_string(),
                Self::FlowsCreate => "flows.create".to_string(),
                Self::FlowsUpdate => "flows.update".to_string(),
                Self::FlowsDelete => "flows.delete".to_string(),
                Self::FlowsArchive => "flows.archive".to_string(),
                Self::AppsCreate => "apps.create".to_string(),
                Self::AppsUpdate => "apps.update".to_string(),
                Self::AppsDelete => "apps.delete".to_string(),
                Self::FolderCreate => "folder.create".to_string(),
                Self::FolderUpdate => "folder.update".to_string(),
                Self::FolderDelete => "folder.delete".to_string(),
                Self::FolderAddOwner => "folder.add_owner".to_string(),
                Self::FolderRemoveOwner => "folder.remove_owner".to_string(),
                Self::GroupCreate => "group.create".to_string(),
                Self::GroupDelete => "group.delete".to_string(),
                Self::GroupEdit => "group.edit".to_string(),
                Self::GroupAdduser => "group.adduser".to_string(),
                Self::GroupRemoveuser => "group.removeuser".to_string(),
                Self::IgroupCreate => "igroup.create".to_string(),
                Self::IgroupDelete => "igroup.delete".to_string(),
                Self::IgroupAdduser => "igroup.adduser".to_string(),
                Self::IgroupRemoveuser => "igroup.removeuser".to_string(),
                Self::VariablesDecryptSecret => "variables.decrypt_secret".to_string(),
                Self::WorkspacesEditCommandScript => {
                    "workspaces.edit_command_script".to_string()
                }
                Self::WorkspacesEditDeployTo => "workspaces.edit_deploy_to".to_string(),
                Self::WorkspacesEditAutoInviteDomain => {
                    "workspaces.edit_auto_invite_domain".to_string()
                }
                Self::WorkspacesEditWebhook => "workspaces.edit_webhook".to_string(),
                Self::WorkspacesEditCopilotConfig => {
                    "workspaces.edit_copilot_config".to_string()
                }
                Self::WorkspacesEditErrorHandler => {
                    "workspaces.edit_error_handler".to_string()
                }
                Self::WorkspacesCreate => "workspaces.create".to_string(),
                Self::WorkspacesUpdate => "workspaces.update".to_string(),
                Self::WorkspacesArchive => "workspaces.archive".to_string(),
                Self::WorkspacesUnarchive => "workspaces.unarchive".to_string(),
                Self::WorkspacesDelete => "workspaces.delete".to_string(),
            }
        }
    }
    impl std::str::FromStr for AuditLogOperation {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "jobs.run" => Ok(Self::JobsRun),
                "jobs.run.script" => Ok(Self::JobsRunScript),
                "jobs.run.preview" => Ok(Self::JobsRunPreview),
                "jobs.run.flow" => Ok(Self::JobsRunFlow),
                "jobs.run.flow_preview" => Ok(Self::JobsRunFlowPreview),
                "jobs.run.script_hub" => Ok(Self::JobsRunScriptHub),
                "jobs.run.dependencies" => Ok(Self::JobsRunDependencies),
                "jobs.run.identity" => Ok(Self::JobsRunIdentity),
                "jobs.run.noop" => Ok(Self::JobsRunNoop),
                "jobs.flow_dependencies" => Ok(Self::JobsFlowDependencies),
                "jobs" => Ok(Self::Jobs),
                "jobs.cancel" => Ok(Self::JobsCancel),
                "jobs.force_cancel" => Ok(Self::JobsForceCancel),
                "jobs.disapproval" => Ok(Self::JobsDisapproval),
                "jobs.delete" => Ok(Self::JobsDelete),
                "account.delete" => Ok(Self::AccountDelete),
                "ai.request" => Ok(Self::AiRequest),
                "resources.create" => Ok(Self::ResourcesCreate),
                "resources.update" => Ok(Self::ResourcesUpdate),
                "resources.delete" => Ok(Self::ResourcesDelete),
                "resource_types.create" => Ok(Self::ResourceTypesCreate),
                "resource_types.update" => Ok(Self::ResourceTypesUpdate),
                "resource_types.delete" => Ok(Self::ResourceTypesDelete),
                "schedule.create" => Ok(Self::ScheduleCreate),
                "schedule.setenabled" => Ok(Self::ScheduleSetenabled),
                "schedule.edit" => Ok(Self::ScheduleEdit),
                "schedule.delete" => Ok(Self::ScheduleDelete),
                "scripts.create" => Ok(Self::ScriptsCreate),
                "scripts.update" => Ok(Self::ScriptsUpdate),
                "scripts.archive" => Ok(Self::ScriptsArchive),
                "scripts.delete" => Ok(Self::ScriptsDelete),
                "users.create" => Ok(Self::UsersCreate),
                "users.delete" => Ok(Self::UsersDelete),
                "users.update" => Ok(Self::UsersUpdate),
                "users.login" => Ok(Self::UsersLogin),
                "users.login_failure" => Ok(Self::UsersLoginFailure),
                "users.logout" => Ok(Self::UsersLogout),
                "users.accept_invite" => Ok(Self::UsersAcceptInvite),
                "users.decline_invite" => Ok(Self::UsersDeclineInvite),
                "users.token.create" => Ok(Self::UsersTokenCreate),
                "users.token.delete" => Ok(Self::UsersTokenDelete),
                "users.add_to_workspace" => Ok(Self::UsersAddToWorkspace),
                "users.add_global" => Ok(Self::UsersAddGlobal),
                "users.setpassword" => Ok(Self::UsersSetpassword),
                "users.impersonate" => Ok(Self::UsersImpersonate),
                "users.leave_workspace" => Ok(Self::UsersLeaveWorkspace),
                "oauth.login" => Ok(Self::OauthLogin),
                "oauth.login_failure" => Ok(Self::OauthLoginFailure),
                "oauth.signup" => Ok(Self::OauthSignup),
                "variables.create" => Ok(Self::VariablesCreate),
                "variables.delete" => Ok(Self::VariablesDelete),
                "variables.update" => Ok(Self::VariablesUpdate),
                "flows.create" => Ok(Self::FlowsCreate),
                "flows.update" => Ok(Self::FlowsUpdate),
                "flows.delete" => Ok(Self::FlowsDelete),
                "flows.archive" => Ok(Self::FlowsArchive),
                "apps.create" => Ok(Self::AppsCreate),
                "apps.update" => Ok(Self::AppsUpdate),
                "apps.delete" => Ok(Self::AppsDelete),
                "folder.create" => Ok(Self::FolderCreate),
                "folder.update" => Ok(Self::FolderUpdate),
                "folder.delete" => Ok(Self::FolderDelete),
                "folder.add_owner" => Ok(Self::FolderAddOwner),
                "folder.remove_owner" => Ok(Self::FolderRemoveOwner),
                "group.create" => Ok(Self::GroupCreate),
                "group.delete" => Ok(Self::GroupDelete),
                "group.edit" => Ok(Self::GroupEdit),
                "group.adduser" => Ok(Self::GroupAdduser),
                "group.removeuser" => Ok(Self::GroupRemoveuser),
                "igroup.create" => Ok(Self::IgroupCreate),
                "igroup.delete" => Ok(Self::IgroupDelete),
                "igroup.adduser" => Ok(Self::IgroupAdduser),
                "igroup.removeuser" => Ok(Self::IgroupRemoveuser),
                "variables.decrypt_secret" => Ok(Self::VariablesDecryptSecret),
                "workspaces.edit_command_script" => Ok(Self::WorkspacesEditCommandScript),
                "workspaces.edit_deploy_to" => Ok(Self::WorkspacesEditDeployTo),
                "workspaces.edit_auto_invite_domain" => {
                    Ok(Self::WorkspacesEditAutoInviteDomain)
                }
                "workspaces.edit_webhook" => Ok(Self::WorkspacesEditWebhook),
                "workspaces.edit_copilot_config" => Ok(Self::WorkspacesEditCopilotConfig),
                "workspaces.edit_error_handler" => Ok(Self::WorkspacesEditErrorHandler),
                "workspaces.create" => Ok(Self::WorkspacesCreate),
                "workspaces.update" => Ok(Self::WorkspacesUpdate),
                "workspaces.archive" => Ok(Self::WorkspacesArchive),
                "workspaces.unarchive" => Ok(Self::WorkspacesUnarchive),
                "workspaces.delete" => Ok(Self::WorkspacesDelete),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for AuditLogOperation {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for AuditLogOperation {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for AuditLogOperation {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct AutoscalingEvent {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub applied_at: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub desired_workers: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub event_type: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub id: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub reason: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub worker_group: Option<String>,
    }
    impl From<&AutoscalingEvent> for AutoscalingEvent {
        fn from(value: &AutoscalingEvent) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct BranchAll {
        pub branches: Vec<BranchAllBranchesItem>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub parallel: Option<bool>,
        #[serde(rename = "type")]
        pub type_: BranchAllType,
    }
    impl From<&BranchAll> for BranchAll {
        fn from(value: &BranchAll) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct BranchAllBranchesItem {
        pub modules: Vec<FlowModule>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub skip_failure: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
    }
    impl From<&BranchAllBranchesItem> for BranchAllBranchesItem {
        fn from(value: &BranchAllBranchesItem) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum BranchAllType {
        #[serde(rename = "branchall")]
        Branchall,
    }
    impl From<&BranchAllType> for BranchAllType {
        fn from(value: &BranchAllType) -> Self {
            value.clone()
        }
    }
    impl ToString for BranchAllType {
        fn to_string(&self) -> String {
            match *self {
                Self::Branchall => "branchall".to_string(),
            }
        }
    }
    impl std::str::FromStr for BranchAllType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "branchall" => Ok(Self::Branchall),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for BranchAllType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for BranchAllType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for BranchAllType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct BranchOne {
        pub branches: Vec<BranchOneBranchesItem>,
        pub default: Vec<FlowModule>,
        #[serde(rename = "type")]
        pub type_: BranchOneType,
    }
    impl From<&BranchOne> for BranchOne {
        fn from(value: &BranchOne) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct BranchOneBranchesItem {
        pub expr: String,
        pub modules: Vec<FlowModule>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
    }
    impl From<&BranchOneBranchesItem> for BranchOneBranchesItem {
        fn from(value: &BranchOneBranchesItem) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum BranchOneType {
        #[serde(rename = "branchone")]
        Branchone,
    }
    impl From<&BranchOneType> for BranchOneType {
        fn from(value: &BranchOneType) -> Self {
            value.clone()
        }
    }
    impl ToString for BranchOneType {
        fn to_string(&self) -> String {
            match *self {
                Self::Branchone => "branchone".to_string(),
            }
        }
    }
    impl std::str::FromStr for BranchOneType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "branchone" => Ok(Self::Branchone),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for BranchOneType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for BranchOneType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for BranchOneType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Capture {
        pub created_at: chrono::DateTime<chrono::offset::Utc>,
        pub id: i64,
        pub payload: serde_json::Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub trigger_extra: Option<serde_json::Value>,
        pub trigger_kind: CaptureTriggerKind,
    }
    impl From<&Capture> for Capture {
        fn from(value: &Capture) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CaptureConfig {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub last_server_ping: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub trigger_config: Option<serde_json::Value>,
        pub trigger_kind: CaptureTriggerKind,
    }
    impl From<&CaptureConfig> for CaptureConfig {
        fn from(value: &CaptureConfig) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum CaptureTriggerKind {
        #[serde(rename = "webhook")]
        Webhook,
        #[serde(rename = "http")]
        Http,
        #[serde(rename = "websocket")]
        Websocket,
        #[serde(rename = "kafka")]
        Kafka,
        #[serde(rename = "email")]
        Email,
        #[serde(rename = "nats")]
        Nats,
        #[serde(rename = "postgres")]
        Postgres,
        #[serde(rename = "sqs")]
        Sqs,
        #[serde(rename = "mqtt")]
        Mqtt,
    }
    impl From<&CaptureTriggerKind> for CaptureTriggerKind {
        fn from(value: &CaptureTriggerKind) -> Self {
            value.clone()
        }
    }
    impl ToString for CaptureTriggerKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Webhook => "webhook".to_string(),
                Self::Http => "http".to_string(),
                Self::Websocket => "websocket".to_string(),
                Self::Kafka => "kafka".to_string(),
                Self::Email => "email".to_string(),
                Self::Nats => "nats".to_string(),
                Self::Postgres => "postgres".to_string(),
                Self::Sqs => "sqs".to_string(),
                Self::Mqtt => "mqtt".to_string(),
            }
        }
    }
    impl std::str::FromStr for CaptureTriggerKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "webhook" => Ok(Self::Webhook),
                "http" => Ok(Self::Http),
                "websocket" => Ok(Self::Websocket),
                "kafka" => Ok(Self::Kafka),
                "email" => Ok(Self::Email),
                "nats" => Ok(Self::Nats),
                "postgres" => Ok(Self::Postgres),
                "sqs" => Ok(Self::Sqs),
                "mqtt" => Ok(Self::Mqtt),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for CaptureTriggerKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for CaptureTriggerKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for CaptureTriggerKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ChannelInfo {
        ///The unique identifier of the channel
        pub channel_id: String,
        ///The display name of the channel
        pub channel_name: String,
        ///The service URL for the channel
        pub service_url: String,
        ///The Microsoft Teams tenant identifier
        pub tenant_id: String,
    }
    impl From<&ChannelInfo> for ChannelInfo {
        fn from(value: &ChannelInfo) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CompletedJob {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub aggregate_wait_time_ms: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub args: Option<ScriptArgs>,
        pub canceled: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub canceled_by: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub canceled_reason: Option<String>,
        pub created_at: chrono::DateTime<chrono::offset::Utc>,
        pub created_by: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deleted: Option<bool>,
        pub duration_ms: i64,
        pub email: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub flow_status: Option<FlowStatus>,
        pub id: uuid::Uuid,
        pub is_flow_step: bool,
        pub is_skipped: bool,
        pub job_kind: CompletedJobJobKind,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub labels: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub language: Option<ScriptLang>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub logs: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub mem_peak: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub parent_job: Option<uuid::Uuid>,
        /**The user (u/userfoo) or group (g/groupfoo) whom
the execution of this script will be permissioned_as and by extension its DT_TOKEN.
*/
        pub permissioned_as: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub preprocessed: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub priority: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub raw_code: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub raw_flow: Option<FlowValue>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub result: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub schedule_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub script_hash: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub script_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub self_wait_time_ms: Option<f64>,
        pub started_at: chrono::DateTime<chrono::offset::Utc>,
        pub success: bool,
        pub tag: String,
        pub visible_to_owner: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub worker: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspace_id: Option<String>,
    }
    impl From<&CompletedJob> for CompletedJob {
        fn from(value: &CompletedJob) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum CompletedJobJobKind {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "preview")]
        Preview,
        #[serde(rename = "dependencies")]
        Dependencies,
        #[serde(rename = "flow")]
        Flow,
        #[serde(rename = "flowdependencies")]
        Flowdependencies,
        #[serde(rename = "appdependencies")]
        Appdependencies,
        #[serde(rename = "flowpreview")]
        Flowpreview,
        #[serde(rename = "script_hub")]
        ScriptHub,
        #[serde(rename = "identity")]
        Identity,
        #[serde(rename = "deploymentcallback")]
        Deploymentcallback,
        #[serde(rename = "singlescriptflow")]
        Singlescriptflow,
        #[serde(rename = "flowscript")]
        Flowscript,
        #[serde(rename = "flownode")]
        Flownode,
        #[serde(rename = "appscript")]
        Appscript,
    }
    impl From<&CompletedJobJobKind> for CompletedJobJobKind {
        fn from(value: &CompletedJobJobKind) -> Self {
            value.clone()
        }
    }
    impl ToString for CompletedJobJobKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Preview => "preview".to_string(),
                Self::Dependencies => "dependencies".to_string(),
                Self::Flow => "flow".to_string(),
                Self::Flowdependencies => "flowdependencies".to_string(),
                Self::Appdependencies => "appdependencies".to_string(),
                Self::Flowpreview => "flowpreview".to_string(),
                Self::ScriptHub => "script_hub".to_string(),
                Self::Identity => "identity".to_string(),
                Self::Deploymentcallback => "deploymentcallback".to_string(),
                Self::Singlescriptflow => "singlescriptflow".to_string(),
                Self::Flowscript => "flowscript".to_string(),
                Self::Flownode => "flownode".to_string(),
                Self::Appscript => "appscript".to_string(),
            }
        }
    }
    impl std::str::FromStr for CompletedJobJobKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "preview" => Ok(Self::Preview),
                "dependencies" => Ok(Self::Dependencies),
                "flow" => Ok(Self::Flow),
                "flowdependencies" => Ok(Self::Flowdependencies),
                "appdependencies" => Ok(Self::Appdependencies),
                "flowpreview" => Ok(Self::Flowpreview),
                "script_hub" => Ok(Self::ScriptHub),
                "identity" => Ok(Self::Identity),
                "deploymentcallback" => Ok(Self::Deploymentcallback),
                "singlescriptflow" => Ok(Self::Singlescriptflow),
                "flowscript" => Ok(Self::Flowscript),
                "flownode" => Ok(Self::Flownode),
                "appscript" => Ok(Self::Appscript),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for CompletedJobJobKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for CompletedJobJobKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for CompletedJobJobKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ConcurrencyGroup {
        pub concurrency_key: String,
        pub total_running: f64,
    }
    impl From<&ConcurrencyGroup> for ConcurrencyGroup {
        fn from(value: &ConcurrencyGroup) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Config {
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub config: std::collections::HashMap<String, serde_json::Value>,
        pub name: String,
    }
    impl From<&Config> for Config {
        fn from(value: &Config) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ContextualVariable {
        pub description: String,
        pub is_custom: bool,
        pub name: String,
        pub value: String,
    }
    impl From<&ContextualVariable> for ContextualVariable {
        fn from(value: &ContextualVariable) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CreateFlowBody {
        #[serde(flatten)]
        pub open_flow_w_path: OpenFlowWPath,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deployment_message: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub draft_only: Option<bool>,
    }
    impl From<&CreateFlowBody> for CreateFlowBody {
        fn from(value: &CreateFlowBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CreateInput {
        pub args: std::collections::HashMap<String, serde_json::Value>,
        pub name: String,
    }
    impl From<&CreateInput> for CreateInput {
        fn from(value: &CreateInput) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CreateResource {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        pub path: String,
        pub resource_type: String,
        pub value: serde_json::Value,
    }
    impl From<&CreateResource> for CreateResource {
        fn from(value: &CreateResource) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CreateVariable {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub account: Option<i64>,
        pub description: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub expires_at: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub is_oauth: Option<bool>,
        pub is_secret: bool,
        pub path: String,
        pub value: String,
    }
    impl From<&CreateVariable> for CreateVariable {
        fn from(value: &CreateVariable) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CreateWorkspace {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub color: Option<String>,
        pub id: String,
        pub name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub username: Option<String>,
    }
    impl From<&CreateWorkspace> for CreateWorkspace {
        fn from(value: &CreateWorkspace) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CriticalAlert {
        ///Acknowledgment status of the alert, can be true, false, or null if not set
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub acknowledged: Option<bool>,
        ///Type of alert (e.g., critical_error)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub alert_type: Option<String>,
        ///Time when the alert was created
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub created_at: Option<chrono::DateTime<chrono::offset::Utc>>,
        ///Unique identifier for the alert
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub id: Option<i64>,
        ///The message content of the alert
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub message: Option<String>,
        ///Workspace id if the alert is in the scope of a workspace
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspace_id: Option<String>,
    }
    impl From<&CriticalAlert> for CriticalAlert {
        fn from(value: &CriticalAlert) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditHttpTrigger {
        pub http_method: EditHttpTriggerHttpMethod,
        pub is_async: bool,
        pub is_flow: bool,
        pub is_static_website: bool,
        pub path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub raw_string: Option<bool>,
        pub requires_auth: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub route_path: Option<String>,
        pub script_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub static_asset_config: Option<EditHttpTriggerStaticAssetConfig>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspaced_route: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub wrap_body: Option<bool>,
    }
    impl From<&EditHttpTrigger> for EditHttpTrigger {
        fn from(value: &EditHttpTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum EditHttpTriggerHttpMethod {
        #[serde(rename = "get")]
        Get,
        #[serde(rename = "post")]
        Post,
        #[serde(rename = "put")]
        Put,
        #[serde(rename = "delete")]
        Delete,
        #[serde(rename = "patch")]
        Patch,
    }
    impl From<&EditHttpTriggerHttpMethod> for EditHttpTriggerHttpMethod {
        fn from(value: &EditHttpTriggerHttpMethod) -> Self {
            value.clone()
        }
    }
    impl ToString for EditHttpTriggerHttpMethod {
        fn to_string(&self) -> String {
            match *self {
                Self::Get => "get".to_string(),
                Self::Post => "post".to_string(),
                Self::Put => "put".to_string(),
                Self::Delete => "delete".to_string(),
                Self::Patch => "patch".to_string(),
            }
        }
    }
    impl std::str::FromStr for EditHttpTriggerHttpMethod {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "get" => Ok(Self::Get),
                "post" => Ok(Self::Post),
                "put" => Ok(Self::Put),
                "delete" => Ok(Self::Delete),
                "patch" => Ok(Self::Patch),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for EditHttpTriggerHttpMethod {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for EditHttpTriggerHttpMethod {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for EditHttpTriggerHttpMethod {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditHttpTriggerStaticAssetConfig {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub filename: Option<String>,
        pub s3: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub storage: Option<String>,
    }
    impl From<&EditHttpTriggerStaticAssetConfig> for EditHttpTriggerStaticAssetConfig {
        fn from(value: &EditHttpTriggerStaticAssetConfig) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditKafkaTrigger {
        pub group_id: String,
        pub is_flow: bool,
        pub kafka_resource_path: String,
        pub path: String,
        pub script_path: String,
        pub topics: Vec<String>,
    }
    impl From<&EditKafkaTrigger> for EditKafkaTrigger {
        fn from(value: &EditKafkaTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditMqttTrigger {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub client_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub client_version: Option<MqttClientVersion>,
        pub enabled: bool,
        pub is_flow: bool,
        pub mqtt_resource_path: String,
        pub path: String,
        pub script_path: String,
        pub subscribe_topics: Vec<MqttSubscribeTopic>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub v3_config: Option<MqttV3Config>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub v5_config: Option<MqttV5Config>,
    }
    impl From<&EditMqttTrigger> for EditMqttTrigger {
        fn from(value: &EditMqttTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditNatsTrigger {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub consumer_name: Option<String>,
        pub is_flow: bool,
        pub nats_resource_path: String,
        pub path: String,
        pub script_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub stream_name: Option<String>,
        pub subjects: Vec<String>,
        pub use_jetstream: bool,
    }
    impl From<&EditNatsTrigger> for EditNatsTrigger {
        fn from(value: &EditNatsTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditPostgresTrigger {
        pub enabled: bool,
        pub is_flow: bool,
        pub path: String,
        pub postgres_resource_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub publication: Option<PublicationData>,
        pub publication_name: String,
        pub replication_slot_name: String,
        pub script_path: String,
    }
    impl From<&EditPostgresTrigger> for EditPostgresTrigger {
        fn from(value: &EditPostgresTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditResource {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<serde_json::Value>,
    }
    impl From<&EditResource> for EditResource {
        fn from(value: &EditResource) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditResourceType {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub schema: Option<serde_json::Value>,
    }
    impl From<&EditResourceType> for EditResourceType {
        fn from(value: &EditResourceType) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditSchedule {
        pub args: ScriptArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cron_version: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub no_flow_overlap: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_failure: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_failure_exact: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_failure_extra_args: Option<ScriptArgs>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_failure_times: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_recovery: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_recovery_extra_args: Option<ScriptArgs>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_recovery_times: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_success: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_success_extra_args: Option<ScriptArgs>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub paused_until: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub retry: Option<Retry>,
        pub schedule: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tag: Option<String>,
        pub timezone: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ws_error_handler_muted: Option<bool>,
    }
    impl From<&EditSchedule> for EditSchedule {
        fn from(value: &EditSchedule) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditSqsTrigger {
        pub aws_resource_path: String,
        pub enabled: bool,
        pub is_flow: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub message_attributes: Vec<String>,
        pub path: String,
        pub queue_url: String,
        pub script_path: String,
    }
    impl From<&EditSqsTrigger> for EditSqsTrigger {
        fn from(value: &EditSqsTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditVariable {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub is_secret: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<String>,
    }
    impl From<&EditVariable> for EditVariable {
        fn from(value: &EditVariable) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditWebsocketTrigger {
        pub can_return_message: bool,
        pub filters: Vec<EditWebsocketTriggerFiltersItem>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub initial_messages: Vec<WebsocketTriggerInitialMessage>,
        pub is_flow: bool,
        pub path: String,
        pub script_path: String,
        pub url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub url_runnable_args: Option<ScriptArgs>,
    }
    impl From<&EditWebsocketTrigger> for EditWebsocketTrigger {
        fn from(value: &EditWebsocketTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditWebsocketTriggerFiltersItem {
        pub key: String,
        pub value: serde_json::Value,
    }
    impl From<&EditWebsocketTriggerFiltersItem> for EditWebsocketTriggerFiltersItem {
        fn from(value: &EditWebsocketTriggerFiltersItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditWorkspaceUser {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub disabled: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub is_admin: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub operator: Option<bool>,
    }
    impl From<&EditWorkspaceUser> for EditWorkspaceUser {
        fn from(value: &EditWorkspaceUser) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ExportedInstanceGroup {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub emails: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub external_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub id: Option<String>,
        pub name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub scim_display_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
    }
    impl From<&ExportedInstanceGroup> for ExportedInstanceGroup {
        fn from(value: &ExportedInstanceGroup) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ExportedUser {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub company: Option<String>,
        pub email: String,
        pub first_time_user: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub password_hash: Option<String>,
        pub super_admin: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub username: Option<String>,
        pub verified: bool,
    }
    impl From<&ExportedUser> for ExportedUser {
        fn from(value: &ExportedUser) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ExtendedJobs {
        pub jobs: Vec<Job>,
        pub obscured_jobs: Vec<ObscuredJob>,
        ///Obscured jobs omitted for security because of too specific filtering
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub omitted_obscured_jobs: Option<bool>,
    }
    impl From<&ExtendedJobs> for ExtendedJobs {
        fn from(value: &ExtendedJobs) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ExtraPerms(pub std::collections::HashMap<String, bool>);
    impl std::ops::Deref for ExtraPerms {
        type Target = std::collections::HashMap<String, bool>;
        fn deref(&self) -> &std::collections::HashMap<String, bool> {
            &self.0
        }
    }
    impl From<ExtraPerms> for std::collections::HashMap<String, bool> {
        fn from(value: ExtraPerms) -> Self {
            value.0
        }
    }
    impl From<&ExtraPerms> for ExtraPerms {
        fn from(value: &ExtraPerms) -> Self {
            value.clone()
        }
    }
    impl From<std::collections::HashMap<String, bool>> for ExtraPerms {
        fn from(value: std::collections::HashMap<String, bool>) -> Self {
            Self(value)
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Flow {
        #[serde(flatten)]
        pub open_flow: OpenFlow,
        #[serde(flatten)]
        pub flow_metadata: FlowMetadata,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub lock_error_logs: Option<String>,
    }
    impl From<&Flow> for Flow {
        fn from(value: &Flow) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowMetadata {
        pub archived: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub dedicated_worker: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub draft_only: Option<bool>,
        pub edited_at: chrono::DateTime<chrono::offset::Utc>,
        pub edited_by: String,
        pub extra_perms: ExtraPerms,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_behalf_of_email: Option<String>,
        pub path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub priority: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub starred: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tag: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub timeout: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub visible_to_runner_only: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspace_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ws_error_handler_muted: Option<bool>,
    }
    impl From<&FlowMetadata> for FlowMetadata {
        fn from(value: &FlowMetadata) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowModule {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cache_ttl: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub continue_on_error: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub delete_after_use: Option<bool>,
        pub id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub mock: Option<FlowModuleMock>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub priority: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub retry: Option<Retry>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub skip_if: Option<FlowModuleSkipIf>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub sleep: Option<InputTransform>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub stop_after_all_iters_if: Option<FlowModuleStopAfterAllItersIf>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub stop_after_if: Option<FlowModuleStopAfterIf>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub suspend: Option<FlowModuleSuspend>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub timeout: Option<f64>,
        pub value: FlowModuleValue,
    }
    impl From<&FlowModule> for FlowModule {
        fn from(value: &FlowModule) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowModuleMock {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub enabled: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub return_value: Option<serde_json::Value>,
    }
    impl From<&FlowModuleMock> for FlowModuleMock {
        fn from(value: &FlowModuleMock) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowModuleSkipIf {
        pub expr: String,
    }
    impl From<&FlowModuleSkipIf> for FlowModuleSkipIf {
        fn from(value: &FlowModuleSkipIf) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowModuleStopAfterAllItersIf {
        pub expr: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub skip_if_stopped: Option<bool>,
    }
    impl From<&FlowModuleStopAfterAllItersIf> for FlowModuleStopAfterAllItersIf {
        fn from(value: &FlowModuleStopAfterAllItersIf) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowModuleStopAfterIf {
        pub expr: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub skip_if_stopped: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error_message: Option<String>
    }
    impl From<&FlowModuleStopAfterIf> for FlowModuleStopAfterIf {
        fn from(value: &FlowModuleStopAfterIf) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowModuleSuspend {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub continue_on_disapprove_timeout: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub hide_cancel: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub required_events: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub resume_form: Option<FlowModuleSuspendResumeForm>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub self_approval_disabled: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub timeout: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub user_auth_required: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub user_groups_required: Option<InputTransform>,
    }
    impl From<&FlowModuleSuspend> for FlowModuleSuspend {
        fn from(value: &FlowModuleSuspend) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowModuleSuspendResumeForm {
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub schema: std::collections::HashMap<String, serde_json::Value>,
    }
    impl From<&FlowModuleSuspendResumeForm> for FlowModuleSuspendResumeForm {
        fn from(value: &FlowModuleSuspendResumeForm) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum FlowModuleValue {
        RawScript(RawScript),
        PathScript(PathScript),
        PathFlow(PathFlow),
        ForloopFlow(ForloopFlow),
        WhileloopFlow(WhileloopFlow),
        BranchOne(BranchOne),
        BranchAll(BranchAll),
        Identity(Identity),
    }
    impl From<&FlowModuleValue> for FlowModuleValue {
        fn from(value: &FlowModuleValue) -> Self {
            value.clone()
        }
    }
    impl From<RawScript> for FlowModuleValue {
        fn from(value: RawScript) -> Self {
            Self::RawScript(value)
        }
    }
    impl From<PathScript> for FlowModuleValue {
        fn from(value: PathScript) -> Self {
            Self::PathScript(value)
        }
    }
    impl From<PathFlow> for FlowModuleValue {
        fn from(value: PathFlow) -> Self {
            Self::PathFlow(value)
        }
    }
    impl From<ForloopFlow> for FlowModuleValue {
        fn from(value: ForloopFlow) -> Self {
            Self::ForloopFlow(value)
        }
    }
    impl From<WhileloopFlow> for FlowModuleValue {
        fn from(value: WhileloopFlow) -> Self {
            Self::WhileloopFlow(value)
        }
    }
    impl From<BranchOne> for FlowModuleValue {
        fn from(value: BranchOne) -> Self {
            Self::BranchOne(value)
        }
    }
    impl From<BranchAll> for FlowModuleValue {
        fn from(value: BranchAll) -> Self {
            Self::BranchAll(value)
        }
    }
    impl From<Identity> for FlowModuleValue {
        fn from(value: Identity) -> Self {
            Self::Identity(value)
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowPreview {
        pub args: ScriptArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub restarted_from: Option<RestartedFrom>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tag: Option<String>,
        pub value: FlowValue,
    }
    impl From<&FlowPreview> for FlowPreview {
        fn from(value: &FlowPreview) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowStatus {
        pub failure_module: FlowStatusFailureModule,
        pub modules: Vec<FlowStatusModule>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub preprocessor_module: Option<FlowStatusModule>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub retry: Option<FlowStatusRetry>,
        pub step: i64,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub user_states: std::collections::HashMap<String, serde_json::Value>,
    }
    impl From<&FlowStatus> for FlowStatus {
        fn from(value: &FlowStatus) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowStatusFailureModule {
        #[serde(flatten)]
        pub flow_status_module: FlowStatusModule,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub parent_module: Option<String>,
    }
    impl From<&FlowStatusFailureModule> for FlowStatusFailureModule {
        fn from(value: &FlowStatusFailureModule) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowStatusModule {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub approvers: Vec<FlowStatusModuleApproversItem>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub branch_chosen: Option<FlowStatusModuleBranchChosen>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub branchall: Option<FlowStatusModuleBranchall>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub count: Option<i64>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub failed_retries: Vec<uuid::Uuid>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub flow_jobs: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub flow_jobs_success: Vec<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub iterator: Option<FlowStatusModuleIterator>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub job: Option<uuid::Uuid>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub progress: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub skipped: Option<bool>,
        #[serde(rename = "type")]
        pub type_: FlowStatusModuleType,
    }
    impl From<&FlowStatusModule> for FlowStatusModule {
        fn from(value: &FlowStatusModule) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowStatusModuleApproversItem {
        pub approver: String,
        pub resume_id: i64,
    }
    impl From<&FlowStatusModuleApproversItem> for FlowStatusModuleApproversItem {
        fn from(value: &FlowStatusModuleApproversItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowStatusModuleBranchChosen {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub branch: Option<i64>,
        #[serde(rename = "type")]
        pub type_: FlowStatusModuleBranchChosenType,
    }
    impl From<&FlowStatusModuleBranchChosen> for FlowStatusModuleBranchChosen {
        fn from(value: &FlowStatusModuleBranchChosen) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum FlowStatusModuleBranchChosenType {
        #[serde(rename = "branch")]
        Branch,
        #[serde(rename = "default")]
        Default,
    }
    impl From<&FlowStatusModuleBranchChosenType> for FlowStatusModuleBranchChosenType {
        fn from(value: &FlowStatusModuleBranchChosenType) -> Self {
            value.clone()
        }
    }
    impl ToString for FlowStatusModuleBranchChosenType {
        fn to_string(&self) -> String {
            match *self {
                Self::Branch => "branch".to_string(),
                Self::Default => "default".to_string(),
            }
        }
    }
    impl std::str::FromStr for FlowStatusModuleBranchChosenType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "branch" => Ok(Self::Branch),
                "default" => Ok(Self::Default),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for FlowStatusModuleBranchChosenType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for FlowStatusModuleBranchChosenType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for FlowStatusModuleBranchChosenType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowStatusModuleBranchall {
        pub branch: i64,
        pub len: i64,
    }
    impl From<&FlowStatusModuleBranchall> for FlowStatusModuleBranchall {
        fn from(value: &FlowStatusModuleBranchall) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowStatusModuleIterator {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub args: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub index: Option<i64>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub itered: Vec<serde_json::Value>,
    }
    impl From<&FlowStatusModuleIterator> for FlowStatusModuleIterator {
        fn from(value: &FlowStatusModuleIterator) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum FlowStatusModuleType {
        WaitingForPriorSteps,
        WaitingForEvents,
        WaitingForExecutor,
        InProgress,
        Success,
        Failure,
    }
    impl From<&FlowStatusModuleType> for FlowStatusModuleType {
        fn from(value: &FlowStatusModuleType) -> Self {
            value.clone()
        }
    }
    impl ToString for FlowStatusModuleType {
        fn to_string(&self) -> String {
            match *self {
                Self::WaitingForPriorSteps => "WaitingForPriorSteps".to_string(),
                Self::WaitingForEvents => "WaitingForEvents".to_string(),
                Self::WaitingForExecutor => "WaitingForExecutor".to_string(),
                Self::InProgress => "InProgress".to_string(),
                Self::Success => "Success".to_string(),
                Self::Failure => "Failure".to_string(),
            }
        }
    }
    impl std::str::FromStr for FlowStatusModuleType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "WaitingForPriorSteps" => Ok(Self::WaitingForPriorSteps),
                "WaitingForEvents" => Ok(Self::WaitingForEvents),
                "WaitingForExecutor" => Ok(Self::WaitingForExecutor),
                "InProgress" => Ok(Self::InProgress),
                "Success" => Ok(Self::Success),
                "Failure" => Ok(Self::Failure),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for FlowStatusModuleType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for FlowStatusModuleType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for FlowStatusModuleType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowStatusRetry {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub fail_count: Option<i64>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub failed_jobs: Vec<uuid::Uuid>,
    }
    impl From<&FlowStatusRetry> for FlowStatusRetry {
        fn from(value: &FlowStatusRetry) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowValue {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cache_ttl: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub concurrency_key: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub concurrency_time_window_s: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub concurrent_limit: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub early_return: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub failure_module: Option<FlowModule>,
        pub modules: Vec<FlowModule>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub preprocessor_module: Option<FlowModule>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub priority: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub same_worker: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub skip_expr: Option<String>,
    }
    impl From<&FlowValue> for FlowValue {
        fn from(value: &FlowValue) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowVersion {
        pub created_at: chrono::DateTime<chrono::offset::Utc>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deployment_msg: Option<String>,
        pub id: i64,
    }
    impl From<&FlowVersion> for FlowVersion {
        fn from(value: &FlowVersion) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Folder {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub created_by: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub edited_at: Option<chrono::DateTime<chrono::offset::Utc>>,
        pub extra_perms: std::collections::HashMap<String, bool>,
        pub name: String,
        pub owners: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
    }
    impl From<&Folder> for Folder {
        fn from(value: &Folder) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ForloopFlow {
        pub iterator: InputTransform,
        pub modules: Vec<FlowModule>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub parallel: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub parallelism: Option<i64>,
        pub skip_failures: bool,
        #[serde(rename = "type")]
        pub type_: ForloopFlowType,
    }
    impl From<&ForloopFlow> for ForloopFlow {
        fn from(value: &ForloopFlow) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum ForloopFlowType {
        #[serde(rename = "forloopflow")]
        Forloopflow,
    }
    impl From<&ForloopFlowType> for ForloopFlowType {
        fn from(value: &ForloopFlowType) -> Self {
            value.clone()
        }
    }
    impl ToString for ForloopFlowType {
        fn to_string(&self) -> String {
            match *self {
                Self::Forloopflow => "forloopflow".to_string(),
            }
        }
    }
    impl std::str::FromStr for ForloopFlowType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "forloopflow" => Ok(Self::Forloopflow),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for ForloopFlowType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for ForloopFlowType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for ForloopFlowType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GitRepositorySettings {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub exclude_types_override: Vec<GitRepositorySettingsExcludeTypesOverrideItem>,
        pub git_repo_resource_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub group_by_folder: Option<bool>,
        pub script_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub use_individual_branch: Option<bool>,
    }
    impl From<&GitRepositorySettings> for GitRepositorySettings {
        fn from(value: &GitRepositorySettings) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum GitRepositorySettingsExcludeTypesOverrideItem {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "flow")]
        Flow,
        #[serde(rename = "app")]
        App,
        #[serde(rename = "folder")]
        Folder,
        #[serde(rename = "resource")]
        Resource,
        #[serde(rename = "variable")]
        Variable,
        #[serde(rename = "secret")]
        Secret,
        #[serde(rename = "resourcetype")]
        Resourcetype,
        #[serde(rename = "schedule")]
        Schedule,
        #[serde(rename = "user")]
        User,
        #[serde(rename = "group")]
        Group,
    }
    impl From<&GitRepositorySettingsExcludeTypesOverrideItem>
    for GitRepositorySettingsExcludeTypesOverrideItem {
        fn from(value: &GitRepositorySettingsExcludeTypesOverrideItem) -> Self {
            value.clone()
        }
    }
    impl ToString for GitRepositorySettingsExcludeTypesOverrideItem {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Flow => "flow".to_string(),
                Self::App => "app".to_string(),
                Self::Folder => "folder".to_string(),
                Self::Resource => "resource".to_string(),
                Self::Variable => "variable".to_string(),
                Self::Secret => "secret".to_string(),
                Self::Resourcetype => "resourcetype".to_string(),
                Self::Schedule => "schedule".to_string(),
                Self::User => "user".to_string(),
                Self::Group => "group".to_string(),
            }
        }
    }
    impl std::str::FromStr for GitRepositorySettingsExcludeTypesOverrideItem {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "flow" => Ok(Self::Flow),
                "app" => Ok(Self::App),
                "folder" => Ok(Self::Folder),
                "resource" => Ok(Self::Resource),
                "variable" => Ok(Self::Variable),
                "secret" => Ok(Self::Secret),
                "resourcetype" => Ok(Self::Resourcetype),
                "schedule" => Ok(Self::Schedule),
                "user" => Ok(Self::User),
                "group" => Ok(Self::Group),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for GitRepositorySettingsExcludeTypesOverrideItem {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String>
    for GitRepositorySettingsExcludeTypesOverrideItem {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String>
    for GitRepositorySettingsExcludeTypesOverrideItem {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GlobalSetting {
        pub name: String,
        pub value: std::collections::HashMap<String, serde_json::Value>,
    }
    impl From<&GlobalSetting> for GlobalSetting {
        fn from(value: &GlobalSetting) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GlobalUserInfo {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub company: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub devops: Option<bool>,
        pub email: String,
        pub login_type: GlobalUserInfoLoginType,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub operator_only: Option<bool>,
        pub super_admin: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub username: Option<String>,
        pub verified: bool,
    }
    impl From<&GlobalUserInfo> for GlobalUserInfo {
        fn from(value: &GlobalUserInfo) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum GlobalUserInfoLoginType {
        #[serde(rename = "password")]
        Password,
        #[serde(rename = "github")]
        Github,
    }
    impl From<&GlobalUserInfoLoginType> for GlobalUserInfoLoginType {
        fn from(value: &GlobalUserInfoLoginType) -> Self {
            value.clone()
        }
    }
    impl ToString for GlobalUserInfoLoginType {
        fn to_string(&self) -> String {
            match *self {
                Self::Password => "password".to_string(),
                Self::Github => "github".to_string(),
            }
        }
    }
    impl std::str::FromStr for GlobalUserInfoLoginType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "password" => Ok(Self::Password),
                "github" => Ok(Self::Github),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for GlobalUserInfoLoginType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for GlobalUserInfoLoginType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for GlobalUserInfoLoginType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Group {
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub extra_perms: std::collections::HashMap<String, bool>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub members: Vec<String>,
        pub name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
    }
    impl From<&Group> for Group {
        fn from(value: &Group) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct HttpTrigger {
        pub http_method: HttpTriggerHttpMethod,
        pub is_async: bool,
        pub is_static_website: bool,
        pub raw_string: bool,
        pub requires_auth: bool,
        pub route_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub static_asset_config: Option<HttpTriggerStaticAssetConfig>,
        pub workspaced_route: bool,
        pub wrap_body: bool,
    }
    impl From<&HttpTrigger> for HttpTrigger {
        fn from(value: &HttpTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum HttpTriggerHttpMethod {
        #[serde(rename = "get")]
        Get,
        #[serde(rename = "post")]
        Post,
        #[serde(rename = "put")]
        Put,
        #[serde(rename = "delete")]
        Delete,
        #[serde(rename = "patch")]
        Patch,
    }
    impl From<&HttpTriggerHttpMethod> for HttpTriggerHttpMethod {
        fn from(value: &HttpTriggerHttpMethod) -> Self {
            value.clone()
        }
    }
    impl ToString for HttpTriggerHttpMethod {
        fn to_string(&self) -> String {
            match *self {
                Self::Get => "get".to_string(),
                Self::Post => "post".to_string(),
                Self::Put => "put".to_string(),
                Self::Delete => "delete".to_string(),
                Self::Patch => "patch".to_string(),
            }
        }
    }
    impl std::str::FromStr for HttpTriggerHttpMethod {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "get" => Ok(Self::Get),
                "post" => Ok(Self::Post),
                "put" => Ok(Self::Put),
                "delete" => Ok(Self::Delete),
                "patch" => Ok(Self::Patch),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for HttpTriggerHttpMethod {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for HttpTriggerHttpMethod {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for HttpTriggerHttpMethod {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct HttpTriggerStaticAssetConfig {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub filename: Option<String>,
        pub s3: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub storage: Option<String>,
    }
    impl From<&HttpTriggerStaticAssetConfig> for HttpTriggerStaticAssetConfig {
        fn from(value: &HttpTriggerStaticAssetConfig) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct HubScriptKind(pub serde_json::Value);
    impl std::ops::Deref for HubScriptKind {
        type Target = serde_json::Value;
        fn deref(&self) -> &serde_json::Value {
            &self.0
        }
    }
    impl From<HubScriptKind> for serde_json::Value {
        fn from(value: HubScriptKind) -> Self {
            value.0
        }
    }
    impl From<&HubScriptKind> for HubScriptKind {
        fn from(value: &HubScriptKind) -> Self {
            value.clone()
        }
    }
    impl From<serde_json::Value> for HubScriptKind {
        fn from(value: serde_json::Value) -> Self {
            Self(value)
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Identity {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub flow: Option<bool>,
        #[serde(rename = "type")]
        pub type_: IdentityType,
    }
    impl From<&Identity> for Identity {
        fn from(value: &Identity) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum IdentityType {
        #[serde(rename = "identity")]
        Identity,
    }
    impl From<&IdentityType> for IdentityType {
        fn from(value: &IdentityType) -> Self {
            value.clone()
        }
    }
    impl ToString for IdentityType {
        fn to_string(&self) -> String {
            match *self {
                Self::Identity => "identity".to_string(),
            }
        }
    }
    impl std::str::FromStr for IdentityType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "identity" => Ok(Self::Identity),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for IdentityType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for IdentityType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for IdentityType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Input {
        pub created_at: chrono::DateTime<chrono::offset::Utc>,
        pub created_by: String,
        pub id: String,
        pub is_public: bool,
        pub name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub success: Option<bool>,
    }
    impl From<&Input> for Input {
        fn from(value: &Input) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum InputTransform {
        StaticTransform(StaticTransform),
        JavascriptTransform(JavascriptTransform),
    }
    impl From<&InputTransform> for InputTransform {
        fn from(value: &InputTransform) -> Self {
            value.clone()
        }
    }
    impl From<StaticTransform> for InputTransform {
        fn from(value: StaticTransform) -> Self {
            Self::StaticTransform(value)
        }
    }
    impl From<JavascriptTransform> for InputTransform {
        fn from(value: JavascriptTransform) -> Self {
            Self::JavascriptTransform(value)
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct InstanceGroup {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub emails: Vec<String>,
        pub name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
    }
    impl From<&InstanceGroup> for InstanceGroup {
        fn from(value: &InstanceGroup) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct JavascriptTransform {
        pub expr: String,
        #[serde(rename = "type")]
        pub type_: JavascriptTransformType,
    }
    impl From<&JavascriptTransform> for JavascriptTransform {
        fn from(value: &JavascriptTransform) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum JavascriptTransformType {
        #[serde(rename = "javascript")]
        Javascript,
    }
    impl From<&JavascriptTransformType> for JavascriptTransformType {
        fn from(value: &JavascriptTransformType) -> Self {
            value.clone()
        }
    }
    impl ToString for JavascriptTransformType {
        fn to_string(&self) -> String {
            match *self {
                Self::Javascript => "javascript".to_string(),
            }
        }
    }
    impl std::str::FromStr for JavascriptTransformType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "javascript" => Ok(Self::Javascript),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for JavascriptTransformType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for JavascriptTransformType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for JavascriptTransformType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum Job {
        Variant0(JobVariant0),
        Variant1(JobVariant1),
    }
    impl From<&Job> for Job {
        fn from(value: &Job) -> Self {
            value.clone()
        }
    }
    impl From<JobVariant0> for Job {
        fn from(value: JobVariant0) -> Self {
            Self::Variant0(value)
        }
    }
    impl From<JobVariant1> for Job {
        fn from(value: JobVariant1) -> Self {
            Self::Variant1(value)
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct JobSearchHit {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub dancer: Option<String>,
    }
    impl From<&JobSearchHit> for JobSearchHit {
        fn from(value: &JobSearchHit) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct JobVariant0 {
        #[serde(flatten)]
        pub completed_job: CompletedJob,
        #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
        pub type_: Option<JobVariant0Type>,
    }
    impl From<&JobVariant0> for JobVariant0 {
        fn from(value: &JobVariant0) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum JobVariant0Type {
        CompletedJob,
    }
    impl From<&JobVariant0Type> for JobVariant0Type {
        fn from(value: &JobVariant0Type) -> Self {
            value.clone()
        }
    }
    impl ToString for JobVariant0Type {
        fn to_string(&self) -> String {
            match *self {
                Self::CompletedJob => "CompletedJob".to_string(),
            }
        }
    }
    impl std::str::FromStr for JobVariant0Type {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "CompletedJob" => Ok(Self::CompletedJob),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for JobVariant0Type {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for JobVariant0Type {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for JobVariant0Type {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct JobVariant1 {
        #[serde(flatten)]
        pub queued_job: QueuedJob,
        #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
        pub type_: Option<JobVariant1Type>,
    }
    impl From<&JobVariant1> for JobVariant1 {
        fn from(value: &JobVariant1) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum JobVariant1Type {
        QueuedJob,
    }
    impl From<&JobVariant1Type> for JobVariant1Type {
        fn from(value: &JobVariant1Type) -> Self {
            value.clone()
        }
    }
    impl ToString for JobVariant1Type {
        fn to_string(&self) -> String {
            match *self {
                Self::QueuedJob => "QueuedJob".to_string(),
            }
        }
    }
    impl std::str::FromStr for JobVariant1Type {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "QueuedJob" => Ok(Self::QueuedJob),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for JobVariant1Type {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for JobVariant1Type {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for JobVariant1Type {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct KafkaTrigger {
        pub enabled: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,
        pub group_id: String,
        pub kafka_resource_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub last_server_ping: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub server_id: Option<String>,
        pub topics: Vec<String>,
    }
    impl From<&KafkaTrigger> for KafkaTrigger {
        fn from(value: &KafkaTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum Language {
        Typescript,
    }
    impl From<&Language> for Language {
        fn from(value: &Language) -> Self {
            value.clone()
        }
    }
    impl ToString for Language {
        fn to_string(&self) -> String {
            match *self {
                Self::Typescript => "Typescript".to_string(),
            }
        }
    }
    impl std::str::FromStr for Language {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "Typescript" => Ok(Self::Typescript),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for Language {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for Language {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for Language {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LargeFileStorage {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub azure_blob_resource_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub public_resource: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub s3_resource_path: Option<String>,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub secondary_storage: std::collections::HashMap<
            String,
            LargeFileStorageSecondaryStorageValue,
        >,
        #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
        pub type_: Option<LargeFileStorageType>,
    }
    impl From<&LargeFileStorage> for LargeFileStorage {
        fn from(value: &LargeFileStorage) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LargeFileStorageSecondaryStorageValue {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub azure_blob_resource_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub public_resource: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub s3_resource_path: Option<String>,
        #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
        pub type_: Option<LargeFileStorageSecondaryStorageValueType>,
    }
    impl From<&LargeFileStorageSecondaryStorageValue>
    for LargeFileStorageSecondaryStorageValue {
        fn from(value: &LargeFileStorageSecondaryStorageValue) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum LargeFileStorageSecondaryStorageValueType {
        S3Storage,
        AzureBlobStorage,
        AzureWorkloadIdentity,
        S3AwsOidc,
    }
    impl From<&LargeFileStorageSecondaryStorageValueType>
    for LargeFileStorageSecondaryStorageValueType {
        fn from(value: &LargeFileStorageSecondaryStorageValueType) -> Self {
            value.clone()
        }
    }
    impl ToString for LargeFileStorageSecondaryStorageValueType {
        fn to_string(&self) -> String {
            match *self {
                Self::S3Storage => "S3Storage".to_string(),
                Self::AzureBlobStorage => "AzureBlobStorage".to_string(),
                Self::AzureWorkloadIdentity => "AzureWorkloadIdentity".to_string(),
                Self::S3AwsOidc => "S3AwsOidc".to_string(),
            }
        }
    }
    impl std::str::FromStr for LargeFileStorageSecondaryStorageValueType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "S3Storage" => Ok(Self::S3Storage),
                "AzureBlobStorage" => Ok(Self::AzureBlobStorage),
                "AzureWorkloadIdentity" => Ok(Self::AzureWorkloadIdentity),
                "S3AwsOidc" => Ok(Self::S3AwsOidc),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for LargeFileStorageSecondaryStorageValueType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for LargeFileStorageSecondaryStorageValueType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for LargeFileStorageSecondaryStorageValueType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum LargeFileStorageType {
        S3Storage,
        AzureBlobStorage,
        AzureWorkloadIdentity,
        S3AwsOidc,
    }
    impl From<&LargeFileStorageType> for LargeFileStorageType {
        fn from(value: &LargeFileStorageType) -> Self {
            value.clone()
        }
    }
    impl ToString for LargeFileStorageType {
        fn to_string(&self) -> String {
            match *self {
                Self::S3Storage => "S3Storage".to_string(),
                Self::AzureBlobStorage => "AzureBlobStorage".to_string(),
                Self::AzureWorkloadIdentity => "AzureWorkloadIdentity".to_string(),
                Self::S3AwsOidc => "S3AwsOidc".to_string(),
            }
        }
    }
    impl std::str::FromStr for LargeFileStorageType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "S3Storage" => Ok(Self::S3Storage),
                "AzureBlobStorage" => Ok(Self::AzureBlobStorage),
                "AzureWorkloadIdentity" => Ok(Self::AzureWorkloadIdentity),
                "S3AwsOidc" => Ok(Self::S3AwsOidc),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for LargeFileStorageType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for LargeFileStorageType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for LargeFileStorageType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListableApp {
        pub edited_at: chrono::DateTime<chrono::offset::Utc>,
        pub execution_mode: ListableAppExecutionMode,
        pub extra_perms: std::collections::HashMap<String, bool>,
        pub id: i64,
        pub path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub starred: Option<bool>,
        pub summary: String,
        pub version: i64,
        pub workspace_id: String,
    }
    impl From<&ListableApp> for ListableApp {
        fn from(value: &ListableApp) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum ListableAppExecutionMode {
        #[serde(rename = "viewer")]
        Viewer,
        #[serde(rename = "publisher")]
        Publisher,
        #[serde(rename = "anonymous")]
        Anonymous,
    }
    impl From<&ListableAppExecutionMode> for ListableAppExecutionMode {
        fn from(value: &ListableAppExecutionMode) -> Self {
            value.clone()
        }
    }
    impl ToString for ListableAppExecutionMode {
        fn to_string(&self) -> String {
            match *self {
                Self::Viewer => "viewer".to_string(),
                Self::Publisher => "publisher".to_string(),
                Self::Anonymous => "anonymous".to_string(),
            }
        }
    }
    impl std::str::FromStr for ListableAppExecutionMode {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "viewer" => Ok(Self::Viewer),
                "publisher" => Ok(Self::Publisher),
                "anonymous" => Ok(Self::Anonymous),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for ListableAppExecutionMode {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for ListableAppExecutionMode {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for ListableAppExecutionMode {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListableRawApp {
        pub edited_at: chrono::DateTime<chrono::offset::Utc>,
        pub extra_perms: std::collections::HashMap<String, bool>,
        pub path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub starred: Option<bool>,
        pub summary: String,
        pub version: f64,
        pub workspace_id: String,
    }
    impl From<&ListableRawApp> for ListableRawApp {
        fn from(value: &ListableRawApp) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListableResource {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub account: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub created_by: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub edited_at: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub extra_perms: std::collections::HashMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub is_expired: Option<bool>,
        pub is_linked: bool,
        pub is_oauth: bool,
        pub is_refreshed: bool,
        pub path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub refresh_error: Option<String>,
        pub resource_type: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspace_id: Option<String>,
    }
    impl From<&ListableResource> for ListableResource {
        fn from(value: &ListableResource) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListableVariable {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub account: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub expires_at: Option<chrono::DateTime<chrono::offset::Utc>>,
        pub extra_perms: std::collections::HashMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub is_expired: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub is_linked: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub is_oauth: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub is_refreshed: Option<bool>,
        pub is_secret: bool,
        pub path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub refresh_error: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<String>,
        pub workspace_id: String,
    }
    impl From<&ListableVariable> for ListableVariable {
        fn from(value: &ListableVariable) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct LogSearchHit {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub dancer: Option<String>,
    }
    impl From<&LogSearchHit> for LogSearchHit {
        fn from(value: &LogSearchHit) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Login {
        pub email: String,
        pub password: String,
    }
    impl From<&Login> for Login {
        fn from(value: &Login) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct MainArgSignature {
        pub args: Vec<MainArgSignatureArgsItem>,
        pub error: String,
        pub has_preprocessor: Option<bool>,
        pub no_main_func: Option<bool>,
        pub star_args: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub star_kwargs: Option<bool>,
        #[serde(rename = "type")]
        pub type_: MainArgSignatureType,
    }
    impl From<&MainArgSignature> for MainArgSignature {
        fn from(value: &MainArgSignature) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct MainArgSignatureArgsItem {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub default: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub has_default: Option<bool>,
        pub name: String,
        pub typ: MainArgSignatureArgsItemTyp,
    }
    impl From<&MainArgSignatureArgsItem> for MainArgSignatureArgsItem {
        fn from(value: &MainArgSignatureArgsItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub enum MainArgSignatureArgsItemTyp {
        #[serde(rename = "float")]
        Float,
        #[serde(rename = "int")]
        Int,
        #[serde(rename = "bool")]
        Bool,
        #[serde(rename = "email")]
        Email,
        #[serde(rename = "unknown")]
        Unknown,
        #[serde(rename = "bytes")]
        Bytes,
        #[serde(rename = "dict")]
        Dict,
        #[serde(rename = "datetime")]
        Datetime,
        #[serde(rename = "sql")]
        Sql,
        #[serde(rename = "resource")]
        Resource(Option<String>),
        #[serde(rename = "str")]
        Str(Option<Vec<String>>),
        #[serde(rename = "object")]
        Object(Vec<MainArgSignatureArgsItemTypObjectItem>),
        #[serde(rename = "list")]
        List(MainArgSignatureArgsItemTypList),
    }
    impl From<&MainArgSignatureArgsItemTyp> for MainArgSignatureArgsItemTyp {
        fn from(value: &MainArgSignatureArgsItemTyp) -> Self {
            value.clone()
        }
    }
    impl From<Option<String>> for MainArgSignatureArgsItemTyp {
        fn from(value: Option<String>) -> Self {
            Self::Resource(value)
        }
    }
    impl From<Option<Vec<String>>> for MainArgSignatureArgsItemTyp {
        fn from(value: Option<Vec<String>>) -> Self {
            Self::Str(value)
        }
    }
    impl From<Vec<MainArgSignatureArgsItemTypObjectItem>>
    for MainArgSignatureArgsItemTyp {
        fn from(value: Vec<MainArgSignatureArgsItemTypObjectItem>) -> Self {
            Self::Object(value)
        }
    }
    impl From<MainArgSignatureArgsItemTypList> for MainArgSignatureArgsItemTyp {
        fn from(value: MainArgSignatureArgsItemTypList) -> Self {
            Self::List(value)
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub enum MainArgSignatureArgsItemTypList {
        #[serde(rename = "float")]
        Float,
        #[serde(rename = "int")]
        Int,
        #[serde(rename = "bool")]
        Bool,
        #[serde(rename = "email")]
        Email,
        #[serde(rename = "unknown")]
        Unknown,
        #[serde(rename = "bytes")]
        Bytes,
        #[serde(rename = "dict")]
        Dict,
        #[serde(rename = "datetime")]
        Datetime,
        #[serde(rename = "sql")]
        Sql,
        #[serde(rename = "str")]
        Str(serde_json::Value),
    }
    impl From<&MainArgSignatureArgsItemTypList> for MainArgSignatureArgsItemTypList {
        fn from(value: &MainArgSignatureArgsItemTypList) -> Self {
            value.clone()
        }
    }
    impl From<serde_json::Value> for MainArgSignatureArgsItemTypList {
        fn from(value: serde_json::Value) -> Self {
            Self::Str(value)
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct MainArgSignatureArgsItemTypObjectItem {
        pub key: String,
        pub typ: MainArgSignatureArgsItemTypObjectItemTyp,
    }
    impl From<&MainArgSignatureArgsItemTypObjectItem>
    for MainArgSignatureArgsItemTypObjectItem {
        fn from(value: &MainArgSignatureArgsItemTypObjectItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub enum MainArgSignatureArgsItemTypObjectItemTyp {
        #[serde(rename = "float")]
        Float,
        #[serde(rename = "int")]
        Int,
        #[serde(rename = "bool")]
        Bool,
        #[serde(rename = "email")]
        Email,
        #[serde(rename = "unknown")]
        Unknown,
        #[serde(rename = "bytes")]
        Bytes,
        #[serde(rename = "dict")]
        Dict,
        #[serde(rename = "datetime")]
        Datetime,
        #[serde(rename = "sql")]
        Sql,
        #[serde(rename = "str")]
        Str(serde_json::Value),
    }
    impl From<&MainArgSignatureArgsItemTypObjectItemTyp>
    for MainArgSignatureArgsItemTypObjectItemTyp {
        fn from(value: &MainArgSignatureArgsItemTypObjectItemTyp) -> Self {
            value.clone()
        }
    }
    impl From<serde_json::Value> for MainArgSignatureArgsItemTypObjectItemTyp {
        fn from(value: serde_json::Value) -> Self {
            Self::Str(value)
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum MainArgSignatureType {
        Valid,
        Invalid,
    }
    impl From<&MainArgSignatureType> for MainArgSignatureType {
        fn from(value: &MainArgSignatureType) -> Self {
            value.clone()
        }
    }
    impl ToString for MainArgSignatureType {
        fn to_string(&self) -> String {
            match *self {
                Self::Valid => "Valid".to_string(),
                Self::Invalid => "Invalid".to_string(),
            }
        }
    }
    impl std::str::FromStr for MainArgSignatureType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "Valid" => Ok(Self::Valid),
                "Invalid" => Ok(Self::Invalid),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for MainArgSignatureType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for MainArgSignatureType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for MainArgSignatureType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct MetricDataPoint {
        pub timestamp: chrono::DateTime<chrono::offset::Utc>,
        pub value: f64,
    }
    impl From<&MetricDataPoint> for MetricDataPoint {
        fn from(value: &MetricDataPoint) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct MetricMetadata {
        pub id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
    }
    impl From<&MetricMetadata> for MetricMetadata {
        fn from(value: &MetricMetadata) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum MqttClientVersion {
        #[serde(rename = "v3")]
        V3,
        #[serde(rename = "v5")]
        V5,
    }
    impl From<&MqttClientVersion> for MqttClientVersion {
        fn from(value: &MqttClientVersion) -> Self {
            value.clone()
        }
    }
    impl ToString for MqttClientVersion {
        fn to_string(&self) -> String {
            match *self {
                Self::V3 => "v3".to_string(),
                Self::V5 => "v5".to_string(),
            }
        }
    }
    impl std::str::FromStr for MqttClientVersion {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "v3" => Ok(Self::V3),
                "v5" => Ok(Self::V5),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for MqttClientVersion {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for MqttClientVersion {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for MqttClientVersion {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum MqttQoS {
        #[serde(rename = "qos0")]
        Qos0,
        #[serde(rename = "qos1")]
        Qos1,
        #[serde(rename = "qos2")]
        Qos2,
    }
    impl From<&MqttQoS> for MqttQoS {
        fn from(value: &MqttQoS) -> Self {
            value.clone()
        }
    }
    impl ToString for MqttQoS {
        fn to_string(&self) -> String {
            match *self {
                Self::Qos0 => "qos0".to_string(),
                Self::Qos1 => "qos1".to_string(),
                Self::Qos2 => "qos2".to_string(),
            }
        }
    }
    impl std::str::FromStr for MqttQoS {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "qos0" => Ok(Self::Qos0),
                "qos1" => Ok(Self::Qos1),
                "qos2" => Ok(Self::Qos2),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for MqttQoS {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for MqttQoS {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for MqttQoS {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct MqttSubscribeTopic {
        pub qos: MqttQoS,
        pub topic: String,
    }
    impl From<&MqttSubscribeTopic> for MqttSubscribeTopic {
        fn from(value: &MqttSubscribeTopic) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct MqttTrigger {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub client_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub client_version: Option<MqttClientVersion>,
        pub enabled: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub last_server_ping: Option<chrono::DateTime<chrono::offset::Utc>>,
        pub mqtt_resource_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub server_id: Option<String>,
        pub subscribe_topics: Vec<MqttSubscribeTopic>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub v3_config: Option<MqttV3Config>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub v5_config: Option<MqttV5Config>,
    }
    impl From<&MqttTrigger> for MqttTrigger {
        fn from(value: &MqttTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct MqttV3Config {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub clean_session: Option<bool>,
    }
    impl From<&MqttV3Config> for MqttV3Config {
        fn from(value: &MqttV3Config) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct MqttV5Config {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub clean_start: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub session_expiry_interval: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub topic_alias: Option<f64>,
    }
    impl From<&MqttV5Config> for MqttV5Config {
        fn from(value: &MqttV5Config) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct NatsTrigger {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub consumer_name: Option<String>,
        pub enabled: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub last_server_ping: Option<chrono::DateTime<chrono::offset::Utc>>,
        pub nats_resource_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub server_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub stream_name: Option<String>,
        pub subjects: Vec<String>,
        pub use_jetstream: bool,
    }
    impl From<&NatsTrigger> for NatsTrigger {
        fn from(value: &NatsTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct NewHttpTrigger {
        pub http_method: NewHttpTriggerHttpMethod,
        pub is_async: bool,
        pub is_flow: bool,
        pub is_static_website: bool,
        pub path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub raw_string: Option<bool>,
        pub requires_auth: bool,
        pub route_path: String,
        pub script_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub static_asset_config: Option<NewHttpTriggerStaticAssetConfig>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspaced_route: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub wrap_body: Option<bool>,
    }
    impl From<&NewHttpTrigger> for NewHttpTrigger {
        fn from(value: &NewHttpTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum NewHttpTriggerHttpMethod {
        #[serde(rename = "get")]
        Get,
        #[serde(rename = "post")]
        Post,
        #[serde(rename = "put")]
        Put,
        #[serde(rename = "delete")]
        Delete,
        #[serde(rename = "patch")]
        Patch,
    }
    impl From<&NewHttpTriggerHttpMethod> for NewHttpTriggerHttpMethod {
        fn from(value: &NewHttpTriggerHttpMethod) -> Self {
            value.clone()
        }
    }
    impl ToString for NewHttpTriggerHttpMethod {
        fn to_string(&self) -> String {
            match *self {
                Self::Get => "get".to_string(),
                Self::Post => "post".to_string(),
                Self::Put => "put".to_string(),
                Self::Delete => "delete".to_string(),
                Self::Patch => "patch".to_string(),
            }
        }
    }
    impl std::str::FromStr for NewHttpTriggerHttpMethod {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "get" => Ok(Self::Get),
                "post" => Ok(Self::Post),
                "put" => Ok(Self::Put),
                "delete" => Ok(Self::Delete),
                "patch" => Ok(Self::Patch),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for NewHttpTriggerHttpMethod {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for NewHttpTriggerHttpMethod {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for NewHttpTriggerHttpMethod {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct NewHttpTriggerStaticAssetConfig {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub filename: Option<String>,
        pub s3: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub storage: Option<String>,
    }
    impl From<&NewHttpTriggerStaticAssetConfig> for NewHttpTriggerStaticAssetConfig {
        fn from(value: &NewHttpTriggerStaticAssetConfig) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct NewKafkaTrigger {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub enabled: Option<bool>,
        pub group_id: String,
        pub is_flow: bool,
        pub kafka_resource_path: String,
        pub path: String,
        pub script_path: String,
        pub topics: Vec<String>,
    }
    impl From<&NewKafkaTrigger> for NewKafkaTrigger {
        fn from(value: &NewKafkaTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct NewMqttTrigger {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub client_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub client_version: Option<MqttClientVersion>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub enabled: Option<bool>,
        pub is_flow: bool,
        pub mqtt_resource_path: String,
        pub path: String,
        pub script_path: String,
        pub subscribe_topics: Vec<MqttSubscribeTopic>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub v3_config: Option<MqttV3Config>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub v5_config: Option<MqttV5Config>,
    }
    impl From<&NewMqttTrigger> for NewMqttTrigger {
        fn from(value: &NewMqttTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct NewNatsTrigger {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub consumer_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub enabled: Option<bool>,
        pub is_flow: bool,
        pub nats_resource_path: String,
        pub path: String,
        pub script_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub stream_name: Option<String>,
        pub subjects: Vec<String>,
        pub use_jetstream: bool,
    }
    impl From<&NewNatsTrigger> for NewNatsTrigger {
        fn from(value: &NewNatsTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct NewPostgresTrigger {
        pub enabled: bool,
        pub is_flow: bool,
        pub path: String,
        pub postgres_resource_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub publication: Option<PublicationData>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub publication_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub replication_slot_name: Option<String>,
        pub script_path: String,
    }
    impl From<&NewPostgresTrigger> for NewPostgresTrigger {
        fn from(value: &NewPostgresTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct NewSchedule {
        pub args: ScriptArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cron_version: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub enabled: Option<bool>,
        pub is_flow: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub no_flow_overlap: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_failure: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_failure_exact: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_failure_extra_args: Option<ScriptArgs>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_failure_times: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_recovery: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_recovery_extra_args: Option<ScriptArgs>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_recovery_times: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_success: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_success_extra_args: Option<ScriptArgs>,
        pub path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub paused_until: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub retry: Option<Retry>,
        pub schedule: String,
        pub script_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tag: Option<String>,
        pub timezone: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ws_error_handler_muted: Option<bool>,
    }
    impl From<&NewSchedule> for NewSchedule {
        fn from(value: &NewSchedule) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct NewScript {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cache_ttl: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub codebase: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub concurrency_key: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub concurrency_time_window_s: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub concurrent_limit: Option<i64>,
        pub content: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub dedicated_worker: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub delete_after_use: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deployment_message: Option<String>,
        pub description: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub draft_only: Option<bool>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub envs: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub has_preprocessor: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub is_template: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub kind: Option<NewScriptKind>,
        pub language: ScriptLang,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub lock: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub no_main_func: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_behalf_of_email: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub parent_hash: Option<String>,
        pub path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub priority: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub restart_unless_cancelled: Option<bool>,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub schema: std::collections::HashMap<String, serde_json::Value>,
        pub summary: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tag: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub timeout: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub visible_to_runner_only: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ws_error_handler_muted: Option<bool>,
    }
    impl From<&NewScript> for NewScript {
        fn from(value: &NewScript) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum NewScriptKind {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "failure")]
        Failure,
        #[serde(rename = "trigger")]
        Trigger,
        #[serde(rename = "command")]
        Command,
        #[serde(rename = "approval")]
        Approval,
        #[serde(rename = "preprocessor")]
        Preprocessor,
    }
    impl From<&NewScriptKind> for NewScriptKind {
        fn from(value: &NewScriptKind) -> Self {
            value.clone()
        }
    }
    impl ToString for NewScriptKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Failure => "failure".to_string(),
                Self::Trigger => "trigger".to_string(),
                Self::Command => "command".to_string(),
                Self::Approval => "approval".to_string(),
                Self::Preprocessor => "preprocessor".to_string(),
            }
        }
    }
    impl std::str::FromStr for NewScriptKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "failure" => Ok(Self::Failure),
                "trigger" => Ok(Self::Trigger),
                "command" => Ok(Self::Command),
                "approval" => Ok(Self::Approval),
                "preprocessor" => Ok(Self::Preprocessor),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for NewScriptKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for NewScriptKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for NewScriptKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct NewScriptWithDraft {
        #[serde(flatten)]
        pub new_script: NewScript,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub draft: Option<NewScript>,
        pub hash: String,
    }
    impl From<&NewScriptWithDraft> for NewScriptWithDraft {
        fn from(value: &NewScriptWithDraft) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct NewSqsTrigger {
        pub aws_resource_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub enabled: Option<bool>,
        pub is_flow: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub message_attributes: Vec<String>,
        pub path: String,
        pub queue_url: String,
        pub script_path: String,
    }
    impl From<&NewSqsTrigger> for NewSqsTrigger {
        fn from(value: &NewSqsTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct NewToken {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub expiration: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub label: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub scopes: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspace_id: Option<String>,
    }
    impl From<&NewToken> for NewToken {
        fn from(value: &NewToken) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct NewTokenImpersonate {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub expiration: Option<chrono::DateTime<chrono::offset::Utc>>,
        pub impersonate_email: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspace_id: Option<String>,
    }
    impl From<&NewTokenImpersonate> for NewTokenImpersonate {
        fn from(value: &NewTokenImpersonate) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct NewWebsocketTrigger {
        pub can_return_message: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub enabled: Option<bool>,
        pub filters: Vec<NewWebsocketTriggerFiltersItem>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub initial_messages: Vec<WebsocketTriggerInitialMessage>,
        pub is_flow: bool,
        pub path: String,
        pub script_path: String,
        pub url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub url_runnable_args: Option<ScriptArgs>,
    }
    impl From<&NewWebsocketTrigger> for NewWebsocketTrigger {
        fn from(value: &NewWebsocketTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct NewWebsocketTriggerFiltersItem {
        pub key: String,
        pub value: serde_json::Value,
    }
    impl From<&NewWebsocketTriggerFiltersItem> for NewWebsocketTriggerFiltersItem {
        fn from(value: &NewWebsocketTriggerFiltersItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ObscuredJob {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub duration_ms: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub started_at: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub typ: Option<String>,
    }
    impl From<&ObscuredJob> for ObscuredJob {
        fn from(value: &ObscuredJob) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct OpenFlow {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub schema: std::collections::HashMap<String, serde_json::Value>,
        pub summary: String,
        pub value: FlowValue,
    }
    impl From<&OpenFlow> for OpenFlow {
        fn from(value: &OpenFlow) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct OpenFlowWPath {
        #[serde(flatten)]
        pub open_flow: OpenFlow,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub dedicated_worker: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_behalf_of_email: Option<String>,
        pub path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub priority: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tag: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub timeout: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub visible_to_runner_only: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ws_error_handler_muted: Option<bool>,
    }
    impl From<&OpenFlowWPath> for OpenFlowWPath {
        fn from(value: &OpenFlowWPath) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct OperatorSettings(pub Option<OperatorSettingsInner>);
    impl std::ops::Deref for OperatorSettings {
        type Target = Option<OperatorSettingsInner>;
        fn deref(&self) -> &Option<OperatorSettingsInner> {
            &self.0
        }
    }
    impl From<OperatorSettings> for Option<OperatorSettingsInner> {
        fn from(value: OperatorSettings) -> Self {
            value.0
        }
    }
    impl From<&OperatorSettings> for OperatorSettings {
        fn from(value: &OperatorSettings) -> Self {
            value.clone()
        }
    }
    impl From<Option<OperatorSettingsInner>> for OperatorSettings {
        fn from(value: Option<OperatorSettingsInner>) -> Self {
            Self(value)
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct OperatorSettingsInner {
        ///Whether operators can view audit logs
        pub audit_logs: bool,
        ///Whether operators can view folders page
        pub folders: bool,
        ///Whether operators can view groups page
        pub groups: bool,
        ///Whether operators can view resources
        pub resources: bool,
        ///Whether operators can view runs
        pub runs: bool,
        ///Whether operators can view schedules
        pub schedules: bool,
        ///Whether operators can view triggers
        pub triggers: bool,
        ///Whether operators can view variables
        pub variables: bool,
        ///Whether operators can view workers page
        pub workers: bool,
    }
    impl From<&OperatorSettingsInner> for OperatorSettingsInner {
        fn from(value: &OperatorSettingsInner) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct PathFlow {
        pub input_transforms: std::collections::HashMap<String, InputTransform>,
        pub path: String,
        #[serde(rename = "type")]
        pub type_: PathFlowType,
    }
    impl From<&PathFlow> for PathFlow {
        fn from(value: &PathFlow) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum PathFlowType {
        #[serde(rename = "flow")]
        Flow,
    }
    impl From<&PathFlowType> for PathFlowType {
        fn from(value: &PathFlowType) -> Self {
            value.clone()
        }
    }
    impl ToString for PathFlowType {
        fn to_string(&self) -> String {
            match *self {
                Self::Flow => "flow".to_string(),
            }
        }
    }
    impl std::str::FromStr for PathFlowType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "flow" => Ok(Self::Flow),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for PathFlowType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for PathFlowType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for PathFlowType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct PathScript {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub hash: Option<String>,
        pub input_transforms: std::collections::HashMap<String, InputTransform>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub is_trigger: Option<bool>,
        pub path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tag_override: Option<String>,
        #[serde(rename = "type")]
        pub type_: PathScriptType,
    }
    impl From<&PathScript> for PathScript {
        fn from(value: &PathScript) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum PathScriptType {
        #[serde(rename = "script")]
        Script,
    }
    impl From<&PathScriptType> for PathScriptType {
        fn from(value: &PathScriptType) -> Self {
            value.clone()
        }
    }
    impl ToString for PathScriptType {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
            }
        }
    }
    impl std::str::FromStr for PathScriptType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for PathScriptType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for PathScriptType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for PathScriptType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct PolarsClientKwargs {
        pub region_name: String,
    }
    impl From<&PolarsClientKwargs> for PolarsClientKwargs {
        fn from(value: &PolarsClientKwargs) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Policy {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub allowed_s3_keys: Vec<PolicyAllowedS3KeysItem>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub execution_mode: Option<PolicyExecutionMode>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_behalf_of: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_behalf_of_email: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub s3_inputs: Vec<std::collections::HashMap<String, serde_json::Value>>,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub triggerables: std::collections::HashMap<
            String,
            std::collections::HashMap<String, serde_json::Value>,
        >,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub triggerables_v2: std::collections::HashMap<
            String,
            std::collections::HashMap<String, serde_json::Value>,
        >,
    }
    impl From<&Policy> for Policy {
        fn from(value: &Policy) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct PolicyAllowedS3KeysItem {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub resource: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub s3_path: Option<String>,
    }
    impl From<&PolicyAllowedS3KeysItem> for PolicyAllowedS3KeysItem {
        fn from(value: &PolicyAllowedS3KeysItem) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum PolicyExecutionMode {
        #[serde(rename = "viewer")]
        Viewer,
        #[serde(rename = "publisher")]
        Publisher,
        #[serde(rename = "anonymous")]
        Anonymous,
    }
    impl From<&PolicyExecutionMode> for PolicyExecutionMode {
        fn from(value: &PolicyExecutionMode) -> Self {
            value.clone()
        }
    }
    impl ToString for PolicyExecutionMode {
        fn to_string(&self) -> String {
            match *self {
                Self::Viewer => "viewer".to_string(),
                Self::Publisher => "publisher".to_string(),
                Self::Anonymous => "anonymous".to_string(),
            }
        }
    }
    impl std::str::FromStr for PolicyExecutionMode {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "viewer" => Ok(Self::Viewer),
                "publisher" => Ok(Self::Publisher),
                "anonymous" => Ok(Self::Anonymous),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for PolicyExecutionMode {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for PolicyExecutionMode {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for PolicyExecutionMode {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct PostgresTrigger {
        pub enabled: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub last_server_ping: Option<chrono::DateTime<chrono::offset::Utc>>,
        pub postgres_resource_path: String,
        pub publication_name: String,
        pub replication_slot_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub server_id: Option<String>,
    }
    impl From<&PostgresTrigger> for PostgresTrigger {
        fn from(value: &PostgresTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Preview {
        pub args: ScriptArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub content: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub dedicated_worker: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub kind: Option<PreviewKind>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub language: Option<ScriptLang>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub lock: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tag: Option<String>,
    }
    impl From<&Preview> for Preview {
        fn from(value: &Preview) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum PreviewKind {
        #[serde(rename = "code")]
        Code,
        #[serde(rename = "identity")]
        Identity,
        #[serde(rename = "http")]
        Http,
    }
    impl From<&PreviewKind> for PreviewKind {
        fn from(value: &PreviewKind) -> Self {
            value.clone()
        }
    }
    impl ToString for PreviewKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Code => "code".to_string(),
                Self::Identity => "identity".to_string(),
                Self::Http => "http".to_string(),
            }
        }
    }
    impl std::str::FromStr for PreviewKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "code" => Ok(Self::Code),
                "identity" => Ok(Self::Identity),
                "http" => Ok(Self::Http),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for PreviewKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for PreviewKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for PreviewKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct PublicationData {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub table_to_track: Vec<Relations>,
        pub transaction_to_track: Vec<String>,
    }
    impl From<&PublicationData> for PublicationData {
        fn from(value: &PublicationData) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct QueuedJob {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub aggregate_wait_time_ms: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub args: Option<ScriptArgs>,
        pub canceled: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub canceled_by: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub canceled_reason: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub created_at: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub created_by: Option<String>,
        pub email: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub flow_status: Option<FlowStatus>,
        pub id: uuid::Uuid,
        pub is_flow_step: bool,
        pub job_kind: QueuedJobJobKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub language: Option<ScriptLang>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub last_ping: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub logs: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub mem_peak: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub parent_job: Option<uuid::Uuid>,
        /**The user (u/userfoo) or group (g/groupfoo) whom
the execution of this script will be permissioned_as and by extension its DT_TOKEN.
*/
        pub permissioned_as: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub preprocessed: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub priority: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub raw_code: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub raw_flow: Option<FlowValue>,
        pub running: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub schedule_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub scheduled_for: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub script_hash: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub script_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub self_wait_time_ms: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub started_at: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub suspend: Option<f64>,
        pub tag: String,
        pub visible_to_owner: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub worker: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspace_id: Option<String>,
    }
    impl From<&QueuedJob> for QueuedJob {
        fn from(value: &QueuedJob) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum QueuedJobJobKind {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "preview")]
        Preview,
        #[serde(rename = "dependencies")]
        Dependencies,
        #[serde(rename = "flowdependencies")]
        Flowdependencies,
        #[serde(rename = "appdependencies")]
        Appdependencies,
        #[serde(rename = "flow")]
        Flow,
        #[serde(rename = "flowpreview")]
        Flowpreview,
        #[serde(rename = "script_hub")]
        ScriptHub,
        #[serde(rename = "identity")]
        Identity,
        #[serde(rename = "deploymentcallback")]
        Deploymentcallback,
        #[serde(rename = "singlescriptflow")]
        Singlescriptflow,
        #[serde(rename = "flowscript")]
        Flowscript,
        #[serde(rename = "flownode")]
        Flownode,
        #[serde(rename = "appscript")]
        Appscript,
    }
    impl From<&QueuedJobJobKind> for QueuedJobJobKind {
        fn from(value: &QueuedJobJobKind) -> Self {
            value.clone()
        }
    }
    impl ToString for QueuedJobJobKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Preview => "preview".to_string(),
                Self::Dependencies => "dependencies".to_string(),
                Self::Flowdependencies => "flowdependencies".to_string(),
                Self::Appdependencies => "appdependencies".to_string(),
                Self::Flow => "flow".to_string(),
                Self::Flowpreview => "flowpreview".to_string(),
                Self::ScriptHub => "script_hub".to_string(),
                Self::Identity => "identity".to_string(),
                Self::Deploymentcallback => "deploymentcallback".to_string(),
                Self::Singlescriptflow => "singlescriptflow".to_string(),
                Self::Flowscript => "flowscript".to_string(),
                Self::Flownode => "flownode".to_string(),
                Self::Appscript => "appscript".to_string(),
            }
        }
    }
    impl std::str::FromStr for QueuedJobJobKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "preview" => Ok(Self::Preview),
                "dependencies" => Ok(Self::Dependencies),
                "flowdependencies" => Ok(Self::Flowdependencies),
                "appdependencies" => Ok(Self::Appdependencies),
                "flow" => Ok(Self::Flow),
                "flowpreview" => Ok(Self::Flowpreview),
                "script_hub" => Ok(Self::ScriptHub),
                "identity" => Ok(Self::Identity),
                "deploymentcallback" => Ok(Self::Deploymentcallback),
                "singlescriptflow" => Ok(Self::Singlescriptflow),
                "flowscript" => Ok(Self::Flowscript),
                "flownode" => Ok(Self::Flownode),
                "appscript" => Ok(Self::Appscript),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for QueuedJobJobKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for QueuedJobJobKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for QueuedJobJobKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RawScript {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub concurrency_time_window_s: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub concurrent_limit: Option<f64>,
        pub content: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub custom_concurrency_key: Option<String>,
        pub input_transforms: std::collections::HashMap<String, InputTransform>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub is_trigger: Option<bool>,
        pub language: RawScriptLanguage,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub lock: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tag: Option<String>,
        #[serde(rename = "type")]
        pub type_: RawScriptType,
    }
    impl From<&RawScript> for RawScript {
        fn from(value: &RawScript) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RawScriptForDependencies {
        pub language: ScriptLang,
        pub path: String,
        pub raw_code: String,
    }
    impl From<&RawScriptForDependencies> for RawScriptForDependencies {
        fn from(value: &RawScriptForDependencies) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum RawScriptLanguage {
        #[serde(rename = "deno")]
        Deno,
        #[serde(rename = "bun")]
        Bun,
        #[serde(rename = "python3")]
        Python3,
        #[serde(rename = "go")]
        Go,
        #[serde(rename = "bash")]
        Bash,
        #[serde(rename = "powershell")]
        Powershell,
        #[serde(rename = "postgresql")]
        Postgresql,
        #[serde(rename = "mysql")]
        Mysql,
        #[serde(rename = "bigquery")]
        Bigquery,
        #[serde(rename = "snowflake")]
        Snowflake,
        #[serde(rename = "mssql")]
        Mssql,
        #[serde(rename = "oracledb")]
        Oracledb,
        #[serde(rename = "graphql")]
        Graphql,
        #[serde(rename = "nativets")]
        Nativets,
        #[serde(rename = "php")]
        Php,
    }
    impl From<&RawScriptLanguage> for RawScriptLanguage {
        fn from(value: &RawScriptLanguage) -> Self {
            value.clone()
        }
    }
    impl ToString for RawScriptLanguage {
        fn to_string(&self) -> String {
            match *self {
                Self::Deno => "deno".to_string(),
                Self::Bun => "bun".to_string(),
                Self::Python3 => "python3".to_string(),
                Self::Go => "go".to_string(),
                Self::Bash => "bash".to_string(),
                Self::Powershell => "powershell".to_string(),
                Self::Postgresql => "postgresql".to_string(),
                Self::Mysql => "mysql".to_string(),
                Self::Bigquery => "bigquery".to_string(),
                Self::Snowflake => "snowflake".to_string(),
                Self::Mssql => "mssql".to_string(),
                Self::Oracledb => "oracledb".to_string(),
                Self::Graphql => "graphql".to_string(),
                Self::Nativets => "nativets".to_string(),
                Self::Php => "php".to_string(),
            }
        }
    }
    impl std::str::FromStr for RawScriptLanguage {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "deno" => Ok(Self::Deno),
                "bun" => Ok(Self::Bun),
                "python3" => Ok(Self::Python3),
                "go" => Ok(Self::Go),
                "bash" => Ok(Self::Bash),
                "powershell" => Ok(Self::Powershell),
                "postgresql" => Ok(Self::Postgresql),
                "mysql" => Ok(Self::Mysql),
                "bigquery" => Ok(Self::Bigquery),
                "snowflake" => Ok(Self::Snowflake),
                "mssql" => Ok(Self::Mssql),
                "oracledb" => Ok(Self::Oracledb),
                "graphql" => Ok(Self::Graphql),
                "nativets" => Ok(Self::Nativets),
                "php" => Ok(Self::Php),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for RawScriptLanguage {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for RawScriptLanguage {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for RawScriptLanguage {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum RawScriptType {
        #[serde(rename = "rawscript")]
        Rawscript,
    }
    impl From<&RawScriptType> for RawScriptType {
        fn from(value: &RawScriptType) -> Self {
            value.clone()
        }
    }
    impl ToString for RawScriptType {
        fn to_string(&self) -> String {
            match *self {
                Self::Rawscript => "rawscript".to_string(),
            }
        }
    }
    impl std::str::FromStr for RawScriptType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "rawscript" => Ok(Self::Rawscript),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for RawScriptType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for RawScriptType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for RawScriptType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Relations {
        pub schema_name: String,
        pub table_to_track: TableToTrack,
    }
    impl From<&Relations> for Relations {
        fn from(value: &Relations) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Resource {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub created_by: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub edited_at: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub extra_perms: std::collections::HashMap<String, bool>,
        pub is_oauth: bool,
        pub path: String,
        pub resource_type: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspace_id: Option<String>,
    }
    impl From<&Resource> for Resource {
        fn from(value: &Resource) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ResourceType {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub created_by: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub edited_at: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub format_extension: Option<String>,
        pub name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub schema: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspace_id: Option<String>,
    }
    impl From<&ResourceType> for ResourceType {
        fn from(value: &ResourceType) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RestartedFrom {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub branch_or_iteration_n: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub flow_job_id: Option<uuid::Uuid>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub step_id: Option<String>,
    }
    impl From<&RestartedFrom> for RestartedFrom {
        fn from(value: &RestartedFrom) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Retry {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub constant: Option<RetryConstant>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub exponential: Option<RetryExponential>,
    }
    impl From<&Retry> for Retry {
        fn from(value: &Retry) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RetryConstant {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub attempts: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub seconds: Option<i64>,
    }
    impl From<&RetryConstant> for RetryConstant {
        fn from(value: &RetryConstant) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RetryExponential {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub attempts: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub multiplier: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub random_factor: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub seconds: Option<i64>,
    }
    impl From<&RetryExponential> for RetryExponential {
        fn from(value: &RetryExponential) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum RunnableType {
        ScriptHash,
        ScriptPath,
        FlowPath,
    }
    impl From<&RunnableType> for RunnableType {
        fn from(value: &RunnableType) -> Self {
            value.clone()
        }
    }
    impl ToString for RunnableType {
        fn to_string(&self) -> String {
            match *self {
                Self::ScriptHash => "ScriptHash".to_string(),
                Self::ScriptPath => "ScriptPath".to_string(),
                Self::FlowPath => "FlowPath".to_string(),
            }
        }
    }
    impl std::str::FromStr for RunnableType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "ScriptHash" => Ok(Self::ScriptHash),
                "ScriptPath" => Ok(Self::ScriptPath),
                "FlowPath" => Ok(Self::FlowPath),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for RunnableType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for RunnableType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for RunnableType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct S3Resource {
        #[serde(rename = "accessKey", default, skip_serializing_if = "Option::is_none")]
        pub access_key: Option<String>,
        pub bucket: String,
        #[serde(rename = "endPoint")]
        pub end_point: String,
        #[serde(rename = "pathStyle")]
        pub path_style: bool,
        pub region: String,
        #[serde(rename = "secretKey", default, skip_serializing_if = "Option::is_none")]
        pub secret_key: Option<String>,
        #[serde(rename = "useSSL")]
        pub use_ssl: bool,
    }
    impl From<&S3Resource> for S3Resource {
        fn from(value: &S3Resource) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ScalarMetric {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub metric_id: Option<String>,
        pub value: f64,
    }
    impl From<&ScalarMetric> for ScalarMetric {
        fn from(value: &ScalarMetric) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Schedule {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub args: Option<ScriptArgs>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cron_version: Option<String>,
        pub edited_at: chrono::DateTime<chrono::offset::Utc>,
        pub edited_by: String,
        pub email: String,
        pub enabled: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,
        pub extra_perms: std::collections::HashMap<String, bool>,
        pub is_flow: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub no_flow_overlap: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_failure: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_failure_exact: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_failure_extra_args: Option<ScriptArgs>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_failure_times: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_recovery: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_recovery_extra_args: Option<ScriptArgs>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_recovery_times: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_success: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_success_extra_args: Option<ScriptArgs>,
        pub path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub paused_until: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub retry: Option<Retry>,
        pub schedule: String,
        pub script_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tag: Option<String>,
        pub timezone: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ws_error_handler_muted: Option<bool>,
    }
    impl From<&Schedule> for Schedule {
        fn from(value: &Schedule) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ScheduleWJobs {
        #[serde(flatten)]
        pub schedule: Schedule,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub jobs: Vec<ScheduleWJobsJobsItem>,
    }
    impl From<&ScheduleWJobs> for ScheduleWJobs {
        fn from(value: &ScheduleWJobs) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ScheduleWJobsJobsItem {
        pub duration_ms: f64,
        pub id: String,
        pub success: bool,
    }
    impl From<&ScheduleWJobsJobsItem> for ScheduleWJobsJobsItem {
        fn from(value: &ScheduleWJobsJobsItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Script {
        pub archived: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cache_ttl: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub codebase: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub concurrency_key: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub concurrency_time_window_s: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub concurrent_limit: Option<i64>,
        pub content: String,
        pub created_at: chrono::DateTime<chrono::offset::Utc>,
        pub created_by: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub dedicated_worker: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub delete_after_use: Option<bool>,
        pub deleted: bool,
        pub description: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub draft_only: Option<bool>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub envs: Vec<String>,
        pub extra_perms: std::collections::HashMap<String, bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub has_draft: Option<bool>,
        pub has_preprocessor: bool,
        pub hash: String,
        pub is_template: bool,
        pub kind: ScriptKind,
        pub language: ScriptLang,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub lock: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub lock_error_logs: Option<String>,
        pub no_main_func: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub on_behalf_of_email: Option<String>,
        /**The first element is the direct parent of the script, the second is the parent of the first, etc
*/
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub parent_hashes: Vec<String>,
        pub path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub priority: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub restart_unless_cancelled: Option<bool>,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub schema: std::collections::HashMap<String, serde_json::Value>,
        pub starred: bool,
        pub summary: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tag: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub timeout: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub visible_to_runner_only: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspace_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ws_error_handler_muted: Option<bool>,
    }
    impl From<&Script> for Script {
        fn from(value: &Script) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ScriptArgs(pub std::collections::HashMap<String, serde_json::Value>);
    impl std::ops::Deref for ScriptArgs {
        type Target = std::collections::HashMap<String, serde_json::Value>;
        fn deref(&self) -> &std::collections::HashMap<String, serde_json::Value> {
            &self.0
        }
    }
    impl From<ScriptArgs> for std::collections::HashMap<String, serde_json::Value> {
        fn from(value: ScriptArgs) -> Self {
            value.0
        }
    }
    impl From<&ScriptArgs> for ScriptArgs {
        fn from(value: &ScriptArgs) -> Self {
            value.clone()
        }
    }
    impl From<std::collections::HashMap<String, serde_json::Value>> for ScriptArgs {
        fn from(value: std::collections::HashMap<String, serde_json::Value>) -> Self {
            Self(value)
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ScriptHistory {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deployment_msg: Option<String>,
        pub script_hash: String,
    }
    impl From<&ScriptHistory> for ScriptHistory {
        fn from(value: &ScriptHistory) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum ScriptKind {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "failure")]
        Failure,
        #[serde(rename = "trigger")]
        Trigger,
        #[serde(rename = "command")]
        Command,
        #[serde(rename = "approval")]
        Approval,
        #[serde(rename = "preprocessor")]
        Preprocessor,
    }
    impl From<&ScriptKind> for ScriptKind {
        fn from(value: &ScriptKind) -> Self {
            value.clone()
        }
    }
    impl ToString for ScriptKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Failure => "failure".to_string(),
                Self::Trigger => "trigger".to_string(),
                Self::Command => "command".to_string(),
                Self::Approval => "approval".to_string(),
                Self::Preprocessor => "preprocessor".to_string(),
            }
        }
    }
    impl std::str::FromStr for ScriptKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "failure" => Ok(Self::Failure),
                "trigger" => Ok(Self::Trigger),
                "command" => Ok(Self::Command),
                "approval" => Ok(Self::Approval),
                "preprocessor" => Ok(Self::Preprocessor),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for ScriptKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for ScriptKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for ScriptKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum ScriptLang {
        #[serde(rename = "python3")]
        Python3,
        #[serde(rename = "deno")]
        Deno,
        #[serde(rename = "go")]
        Go,
        #[serde(rename = "bash")]
        Bash,
        #[serde(rename = "powershell")]
        Powershell,
        #[serde(rename = "postgresql")]
        Postgresql,
        #[serde(rename = "mysql")]
        Mysql,
        #[serde(rename = "bigquery")]
        Bigquery,
        #[serde(rename = "snowflake")]
        Snowflake,
        #[serde(rename = "mssql")]
        Mssql,
        #[serde(rename = "oracledb")]
        Oracledb,
        #[serde(rename = "graphql")]
        Graphql,
        #[serde(rename = "nativets")]
        Nativets,
        #[serde(rename = "bun")]
        Bun,
        #[serde(rename = "php")]
        Php,
        #[serde(rename = "rust")]
        Rust,
        #[serde(rename = "ansible")]
        Ansible,
        #[serde(rename = "csharp")]
        Csharp,
        #[serde(rename = "nu")]
        Nu,
    }
    impl From<&ScriptLang> for ScriptLang {
        fn from(value: &ScriptLang) -> Self {
            value.clone()
        }
    }
    impl ToString for ScriptLang {
        fn to_string(&self) -> String {
            match *self {
                Self::Python3 => "python3".to_string(),
                Self::Deno => "deno".to_string(),
                Self::Go => "go".to_string(),
                Self::Bash => "bash".to_string(),
                Self::Powershell => "powershell".to_string(),
                Self::Postgresql => "postgresql".to_string(),
                Self::Mysql => "mysql".to_string(),
                Self::Bigquery => "bigquery".to_string(),
                Self::Snowflake => "snowflake".to_string(),
                Self::Mssql => "mssql".to_string(),
                Self::Oracledb => "oracledb".to_string(),
                Self::Graphql => "graphql".to_string(),
                Self::Nativets => "nativets".to_string(),
                Self::Bun => "bun".to_string(),
                Self::Php => "php".to_string(),
                Self::Rust => "rust".to_string(),
                Self::Ansible => "ansible".to_string(),
                Self::Csharp => "csharp".to_string(),
                Self::Nu => "nu".to_string(),
            }
        }
    }
    impl std::str::FromStr for ScriptLang {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "python3" => Ok(Self::Python3),
                "deno" => Ok(Self::Deno),
                "go" => Ok(Self::Go),
                "bash" => Ok(Self::Bash),
                "powershell" => Ok(Self::Powershell),
                "postgresql" => Ok(Self::Postgresql),
                "mysql" => Ok(Self::Mysql),
                "bigquery" => Ok(Self::Bigquery),
                "snowflake" => Ok(Self::Snowflake),
                "mssql" => Ok(Self::Mssql),
                "oracledb" => Ok(Self::Oracledb),
                "graphql" => Ok(Self::Graphql),
                "nativets" => Ok(Self::Nativets),
                "bun" => Ok(Self::Bun),
                "php" => Ok(Self::Php),
                "rust" => Ok(Self::Rust),
                "ansible" => Ok(Self::Ansible),
                "csharp" => Ok(Self::Csharp),
                "nu" => Ok(Self::Nu),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for ScriptLang {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for ScriptLang {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for ScriptLang {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SlackToken {
        pub access_token: String,
        pub bot: SlackTokenBot,
        pub team_id: String,
        pub team_name: String,
    }
    impl From<&SlackToken> for SlackToken {
        fn from(value: &SlackToken) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SlackTokenBot {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub bot_access_token: Option<String>,
    }
    impl From<&SlackTokenBot> for SlackTokenBot {
        fn from(value: &SlackTokenBot) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Slot {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
    }
    impl From<&Slot> for Slot {
        fn from(value: &Slot) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SlotList {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub active: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub slot_name: Option<String>,
    }
    impl From<&SlotList> for SlotList {
        fn from(value: &SlotList) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SqsTrigger {
        pub aws_resource_path: String,
        pub enabled: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub last_server_ping: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub message_attributes: Vec<String>,
        pub queue_url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub server_id: Option<String>,
    }
    impl From<&SqsTrigger> for SqsTrigger {
        fn from(value: &SqsTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct StaticTransform {
        #[serde(rename = "type")]
        pub type_: StaticTransformType,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<serde_json::Value>,
    }
    impl From<&StaticTransform> for StaticTransform {
        fn from(value: &StaticTransform) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum StaticTransformType {
        #[serde(rename = "javascript")]
        Javascript,
    }
    impl From<&StaticTransformType> for StaticTransformType {
        fn from(value: &StaticTransformType) -> Self {
            value.clone()
        }
    }
    impl ToString for StaticTransformType {
        fn to_string(&self) -> String {
            match *self {
                Self::Javascript => "javascript".to_string(),
            }
        }
    }
    impl std::str::FromStr for StaticTransformType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "javascript" => Ok(Self::Javascript),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for StaticTransformType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for StaticTransformType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for StaticTransformType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TableToTrack(pub Vec<TableToTrackItem>);
    impl std::ops::Deref for TableToTrack {
        type Target = Vec<TableToTrackItem>;
        fn deref(&self) -> &Vec<TableToTrackItem> {
            &self.0
        }
    }
    impl From<TableToTrack> for Vec<TableToTrackItem> {
        fn from(value: TableToTrack) -> Self {
            value.0
        }
    }
    impl From<&TableToTrack> for TableToTrack {
        fn from(value: &TableToTrack) -> Self {
            value.clone()
        }
    }
    impl From<Vec<TableToTrackItem>> for TableToTrack {
        fn from(value: Vec<TableToTrackItem>) -> Self {
            Self(value)
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TableToTrackItem {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub columns_name: Vec<String>,
        pub table_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub where_clause: Option<String>,
    }
    impl From<&TableToTrackItem> for TableToTrackItem {
        fn from(value: &TableToTrackItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TeamInfo {
        ///List of channels within the team
        pub channels: Vec<ChannelInfo>,
        ///The unique identifier of the Microsoft Teams team
        pub team_id: String,
        ///The display name of the Microsoft Teams team
        pub team_name: String,
    }
    impl From<&TeamInfo> for TeamInfo {
        fn from(value: &TeamInfo) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TemplateScript {
        pub language: Language,
        pub postgres_resource_path: String,
        pub relations: Vec<Relations>,
    }
    impl From<&TemplateScript> for TemplateScript {
        fn from(value: &TemplateScript) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TimeseriesMetric {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub metric_id: Option<String>,
        pub values: Vec<MetricDataPoint>,
    }
    impl From<&TimeseriesMetric> for TimeseriesMetric {
        fn from(value: &TimeseriesMetric) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TokenResponse {
        pub access_token: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub expires_in: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub refresh_token: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub scope: Vec<String>,
    }
    impl From<&TokenResponse> for TokenResponse {
        fn from(value: &TokenResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TriggerExtraProperty {
        pub edited_at: chrono::DateTime<chrono::offset::Utc>,
        pub edited_by: String,
        pub email: String,
        pub extra_perms: std::collections::HashMap<String, bool>,
        pub is_flow: bool,
        pub path: String,
        pub script_path: String,
        pub workspace_id: String,
    }
    impl From<&TriggerExtraProperty> for TriggerExtraProperty {
        fn from(value: &TriggerExtraProperty) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TriggersCount {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub email_count: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub http_routes_count: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub kafka_count: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub mqtt_count: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub nats_count: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub postgres_count: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub primary_schedule: Option<TriggersCountPrimarySchedule>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub schedule_count: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub sqs_count: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub webhook_count: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub websocket_count: Option<f64>,
    }
    impl From<&TriggersCount> for TriggersCount {
        fn from(value: &TriggersCount) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TriggersCountPrimarySchedule {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub schedule: Option<String>,
    }
    impl From<&TriggersCountPrimarySchedule> for TriggersCountPrimarySchedule {
        fn from(value: &TriggersCountPrimarySchedule) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TruncatedToken {
        pub created_at: chrono::DateTime<chrono::offset::Utc>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub email: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub expiration: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub label: Option<String>,
        pub last_used_at: chrono::DateTime<chrono::offset::Utc>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub scopes: Vec<String>,
        pub token_prefix: String,
    }
    impl From<&TruncatedToken> for TruncatedToken {
        fn from(value: &TruncatedToken) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct UpdateInput {
        pub id: String,
        pub is_public: bool,
        pub name: String,
    }
    impl From<&UpdateInput> for UpdateInput {
        fn from(value: &UpdateInput) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct UploadFilePart {
        pub part_number: i64,
        pub tag: String,
    }
    impl From<&UploadFilePart> for UploadFilePart {
        fn from(value: &UploadFilePart) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct User {
        pub created_at: chrono::DateTime<chrono::offset::Utc>,
        pub disabled: bool,
        pub email: String,
        pub folders: Vec<String>,
        pub folders_owners: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub groups: Vec<String>,
        pub is_admin: bool,
        pub is_super_admin: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        pub operator: bool,
        pub username: String,
    }
    impl From<&User> for User {
        fn from(value: &User) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct UserUsage {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub email: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub executions: Option<f64>,
    }
    impl From<&UserUsage> for UserUsage {
        fn from(value: &UserUsage) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct UserWorkspaceList {
        pub email: String,
        pub workspaces: Vec<UserWorkspaceListWorkspacesItem>,
    }
    impl From<&UserWorkspaceList> for UserWorkspaceList {
        fn from(value: &UserWorkspaceList) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct UserWorkspaceListWorkspacesItem {
        pub color: String,
        pub id: String,
        pub name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub operator_settings: Option<OperatorSettings>,
        pub username: String,
    }
    impl From<&UserWorkspaceListWorkspacesItem> for UserWorkspaceListWorkspacesItem {
        fn from(value: &UserWorkspaceListWorkspacesItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WebsocketTrigger {
        pub can_return_message: bool,
        pub enabled: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,
        pub filters: Vec<WebsocketTriggerFiltersItem>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub initial_messages: Vec<WebsocketTriggerInitialMessage>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub last_server_ping: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub server_id: Option<String>,
        pub url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub url_runnable_args: Option<ScriptArgs>,
    }
    impl From<&WebsocketTrigger> for WebsocketTrigger {
        fn from(value: &WebsocketTrigger) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WebsocketTriggerFiltersItem {
        pub key: String,
        pub value: serde_json::Value,
    }
    impl From<&WebsocketTriggerFiltersItem> for WebsocketTriggerFiltersItem {
        fn from(value: &WebsocketTriggerFiltersItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub enum WebsocketTriggerInitialMessage {
        #[serde(rename = "raw_message")]
        RawMessage(String),
        #[serde(rename = "runnable_result")]
        RunnableResult { args: ScriptArgs, is_flow: bool, path: String },
    }
    impl From<&WebsocketTriggerInitialMessage> for WebsocketTriggerInitialMessage {
        fn from(value: &WebsocketTriggerInitialMessage) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WhileloopFlow {
        pub modules: Vec<FlowModule>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub parallel: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub parallelism: Option<i64>,
        pub skip_failures: bool,
        #[serde(rename = "type")]
        pub type_: WhileloopFlowType,
    }
    impl From<&WhileloopFlow> for WhileloopFlow {
        fn from(value: &WhileloopFlow) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum WhileloopFlowType {
        #[serde(rename = "forloopflow")]
        Forloopflow,
    }
    impl From<&WhileloopFlowType> for WhileloopFlowType {
        fn from(value: &WhileloopFlowType) -> Self {
            value.clone()
        }
    }
    impl ToString for WhileloopFlowType {
        fn to_string(&self) -> String {
            match *self {
                Self::Forloopflow => "forloopflow".to_string(),
            }
        }
    }
    impl std::str::FromStr for WhileloopFlowType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "forloopflow" => Ok(Self::Forloopflow),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for WhileloopFlowType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for WhileloopFlowType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for WhileloopFlowType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WindmillFileMetadata {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub expires: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub last_modified: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub mime_type: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub size_in_bytes: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub version_id: Option<String>,
    }
    impl From<&WindmillFileMetadata> for WindmillFileMetadata {
        fn from(value: &WindmillFileMetadata) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WindmillFilePreview {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub content: Option<String>,
        pub content_type: WindmillFilePreviewContentType,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub msg: Option<String>,
    }
    impl From<&WindmillFilePreview> for WindmillFilePreview {
        fn from(value: &WindmillFilePreview) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum WindmillFilePreviewContentType {
        RawText,
        Csv,
        Parquet,
        Unknown,
    }
    impl From<&WindmillFilePreviewContentType> for WindmillFilePreviewContentType {
        fn from(value: &WindmillFilePreviewContentType) -> Self {
            value.clone()
        }
    }
    impl ToString for WindmillFilePreviewContentType {
        fn to_string(&self) -> String {
            match *self {
                Self::RawText => "RawText".to_string(),
                Self::Csv => "Csv".to_string(),
                Self::Parquet => "Parquet".to_string(),
                Self::Unknown => "Unknown".to_string(),
            }
        }
    }
    impl std::str::FromStr for WindmillFilePreviewContentType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "RawText" => Ok(Self::RawText),
                "Csv" => Ok(Self::Csv),
                "Parquet" => Ok(Self::Parquet),
                "Unknown" => Ok(Self::Unknown),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for WindmillFilePreviewContentType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for WindmillFilePreviewContentType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for WindmillFilePreviewContentType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WindmillLargeFile {
        pub s3: String,
    }
    impl From<&WindmillLargeFile> for WindmillLargeFile {
        fn from(value: &WindmillLargeFile) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WorkerPing {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub custom_tags: Vec<String>,
        pub ip: String,
        pub jobs_executed: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub last_job_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub last_job_workspace_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub last_ping: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub memory: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub memory_usage: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub occupancy_rate: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub occupancy_rate_15s: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub occupancy_rate_30m: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub occupancy_rate_5m: Option<f64>,
        pub started_at: chrono::DateTime<chrono::offset::Utc>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub vcpus: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub wm_memory_usage: Option<f64>,
        pub wm_version: String,
        pub worker: String,
        pub worker_group: String,
        pub worker_instance: String,
    }
    impl From<&WorkerPing> for WorkerPing {
        fn from(value: &WorkerPing) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WorkflowStatus {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub duration_ms: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub scheduled_for: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub started_at: Option<chrono::DateTime<chrono::offset::Utc>>,
    }
    impl From<&WorkflowStatus> for WorkflowStatus {
        fn from(value: &WorkflowStatus) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WorkflowStatusRecord(
        pub std::collections::HashMap<String, WorkflowStatus>,
    );
    impl std::ops::Deref for WorkflowStatusRecord {
        type Target = std::collections::HashMap<String, WorkflowStatus>;
        fn deref(&self) -> &std::collections::HashMap<String, WorkflowStatus> {
            &self.0
        }
    }
    impl From<WorkflowStatusRecord>
    for std::collections::HashMap<String, WorkflowStatus> {
        fn from(value: WorkflowStatusRecord) -> Self {
            value.0
        }
    }
    impl From<&WorkflowStatusRecord> for WorkflowStatusRecord {
        fn from(value: &WorkflowStatusRecord) -> Self {
            value.clone()
        }
    }
    impl From<std::collections::HashMap<String, WorkflowStatus>>
    for WorkflowStatusRecord {
        fn from(value: std::collections::HashMap<String, WorkflowStatus>) -> Self {
            Self(value)
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WorkflowTask {
        pub args: ScriptArgs,
    }
    impl From<&WorkflowTask> for WorkflowTask {
        fn from(value: &WorkflowTask) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Workspace {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub color: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub domain: Option<String>,
        pub id: String,
        pub name: String,
        pub owner: String,
    }
    impl From<&Workspace> for Workspace {
        fn from(value: &Workspace) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WorkspaceDefaultScripts {
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub default_script_content: std::collections::HashMap<String, String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub hidden: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub order: Vec<String>,
    }
    impl From<&WorkspaceDefaultScripts> for WorkspaceDefaultScripts {
        fn from(value: &WorkspaceDefaultScripts) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WorkspaceDeployUiSettings {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub include_path: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub include_type: Vec<WorkspaceDeployUiSettingsIncludeTypeItem>,
    }
    impl From<&WorkspaceDeployUiSettings> for WorkspaceDeployUiSettings {
        fn from(value: &WorkspaceDeployUiSettings) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum WorkspaceDeployUiSettingsIncludeTypeItem {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "flow")]
        Flow,
        #[serde(rename = "app")]
        App,
        #[serde(rename = "resource")]
        Resource,
        #[serde(rename = "variable")]
        Variable,
        #[serde(rename = "secret")]
        Secret,
        #[serde(rename = "trigger")]
        Trigger,
    }
    impl From<&WorkspaceDeployUiSettingsIncludeTypeItem>
    for WorkspaceDeployUiSettingsIncludeTypeItem {
        fn from(value: &WorkspaceDeployUiSettingsIncludeTypeItem) -> Self {
            value.clone()
        }
    }
    impl ToString for WorkspaceDeployUiSettingsIncludeTypeItem {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Flow => "flow".to_string(),
                Self::App => "app".to_string(),
                Self::Resource => "resource".to_string(),
                Self::Variable => "variable".to_string(),
                Self::Secret => "secret".to_string(),
                Self::Trigger => "trigger".to_string(),
            }
        }
    }
    impl std::str::FromStr for WorkspaceDeployUiSettingsIncludeTypeItem {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "flow" => Ok(Self::Flow),
                "app" => Ok(Self::App),
                "resource" => Ok(Self::Resource),
                "variable" => Ok(Self::Variable),
                "secret" => Ok(Self::Secret),
                "trigger" => Ok(Self::Trigger),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for WorkspaceDeployUiSettingsIncludeTypeItem {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for WorkspaceDeployUiSettingsIncludeTypeItem {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for WorkspaceDeployUiSettingsIncludeTypeItem {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WorkspaceGitSyncSettings {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub include_path: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub include_type: Vec<WorkspaceGitSyncSettingsIncludeTypeItem>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub repositories: Vec<GitRepositorySettings>,
    }
    impl From<&WorkspaceGitSyncSettings> for WorkspaceGitSyncSettings {
        fn from(value: &WorkspaceGitSyncSettings) -> Self {
            value.clone()
        }
    }
    #[derive(
        Clone,
        Copy,
        Debug,
        Deserialize,
        Eq,
        Hash,
        Ord,
        PartialEq,
        PartialOrd,
        Serialize
    )]
    pub enum WorkspaceGitSyncSettingsIncludeTypeItem {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "flow")]
        Flow,
        #[serde(rename = "app")]
        App,
        #[serde(rename = "folder")]
        Folder,
        #[serde(rename = "resource")]
        Resource,
        #[serde(rename = "variable")]
        Variable,
        #[serde(rename = "secret")]
        Secret,
        #[serde(rename = "resourcetype")]
        Resourcetype,
        #[serde(rename = "schedule")]
        Schedule,
        #[serde(rename = "user")]
        User,
        #[serde(rename = "group")]
        Group,
    }
    impl From<&WorkspaceGitSyncSettingsIncludeTypeItem>
    for WorkspaceGitSyncSettingsIncludeTypeItem {
        fn from(value: &WorkspaceGitSyncSettingsIncludeTypeItem) -> Self {
            value.clone()
        }
    }
    impl ToString for WorkspaceGitSyncSettingsIncludeTypeItem {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Flow => "flow".to_string(),
                Self::App => "app".to_string(),
                Self::Folder => "folder".to_string(),
                Self::Resource => "resource".to_string(),
                Self::Variable => "variable".to_string(),
                Self::Secret => "secret".to_string(),
                Self::Resourcetype => "resourcetype".to_string(),
                Self::Schedule => "schedule".to_string(),
                Self::User => "user".to_string(),
                Self::Group => "group".to_string(),
            }
        }
    }
    impl std::str::FromStr for WorkspaceGitSyncSettingsIncludeTypeItem {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "flow" => Ok(Self::Flow),
                "app" => Ok(Self::App),
                "folder" => Ok(Self::Folder),
                "resource" => Ok(Self::Resource),
                "variable" => Ok(Self::Variable),
                "secret" => Ok(Self::Secret),
                "resourcetype" => Ok(Self::Resourcetype),
                "schedule" => Ok(Self::Schedule),
                "user" => Ok(Self::User),
                "group" => Ok(Self::Group),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for WorkspaceGitSyncSettingsIncludeTypeItem {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for WorkspaceGitSyncSettingsIncludeTypeItem {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for WorkspaceGitSyncSettingsIncludeTypeItem {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WorkspaceInvite {
        pub email: String,
        pub is_admin: bool,
        pub operator: bool,
        pub workspace_id: String,
    }
    impl From<&WorkspaceInvite> for WorkspaceInvite {
        fn from(value: &WorkspaceInvite) -> Self {
            value.clone()
        }
    }
}
#[derive(Clone, Debug)]
/**Client for Windmill API

Version: 1.478.1*/
pub struct Client {
    pub(crate) baseurl: String,
    pub(crate) client: reqwest::Client,
}
impl Client {
    /// Create a new client.
    ///
    /// `baseurl` is the base URL provided to the internal
    /// `reqwest::Client`, and should include a scheme and hostname,
    /// as well as port and a path stem if applicable.
    pub fn new(baseurl: &str) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let client = {
            let dur = std::time::Duration::from_secs(15);
            reqwest::ClientBuilder::new().connect_timeout(dur).timeout(dur)
        };
        #[cfg(target_arch = "wasm32")]
        let client = reqwest::ClientBuilder::new();
        Self::new_with_client(baseurl, client.build().unwrap())
    }
    /// Construct a new client with an existing `reqwest::Client`,
    /// allowing more control over its configuration.
    ///
    /// `baseurl` is the base URL provided to the internal
    /// `reqwest::Client`, and should include a scheme and hostname,
    /// as well as port and a path stem if applicable.
    pub fn new_with_client(baseurl: &str, client: reqwest::Client) -> Self {
        Self {
            baseurl: baseurl.to_string(),
            client,
        }
    }
    /// Get the base URL to which requests are made.
    pub fn baseurl(&self) -> &String {
        &self.baseurl
    }
    /// Get the internal `reqwest::Client` used to make requests.
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }
    /// Get the version of this API.
    ///
    /// This string is pulled directly from the source OpenAPI
    /// document and may be in any format the API selects.
    pub fn api_version(&self) -> &'static str {
        "1.478.1"
    }
}
impl Client {
    /**list all workspaces visible to me

Sends a `GET` request to `/workspaces/list`

*/
    pub async fn list_workspaces<'a>(
        &'a self,
    ) -> Result<ResponseValue<Vec<types::Workspace>>, Error<()>> {
        let url = format!("{}/workspaces/list", self.baseurl,);
        let request = self
            .client
            .get(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create script

Sends a `POST` request to `/w/{workspace}/scripts/create`

Arguments:
- `workspace`
- `body`: Partially filled script
*/
    pub async fn create_script<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::NewScript,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/create", self.baseurl, encode_path(& workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get flow by path

Sends a `GET` request to `/w/{workspace}/flows/get/{path}`

*/
    pub async fn get_flow_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        with_starred_info: Option<bool>,
    ) -> Result<ResponseValue<types::Flow>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/get/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &with_starred_info {
            query.push(("with_starred_info", v.to_string()));
        }
        let request = self
            .client
            .get(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .query(&query)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create flow

Sends a `POST` request to `/w/{workspace}/flows/create`

Arguments:
- `workspace`
- `body`: Partially filled flow
*/
    pub async fn create_flow<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::CreateFlowBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/create", self.baseurl, encode_path(& workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create schedule

Sends a `POST` request to `/w/{workspace}/schedules/create`

Arguments:
- `workspace`
- `body`: new schedule
*/
    pub async fn create_schedule<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::NewSchedule,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/schedules/create", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**update schedule

Sends a `POST` request to `/w/{workspace}/schedules/update/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated schedule
*/
    pub async fn update_schedule<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::EditSchedule,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/schedules/update/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
}
pub mod prelude {
    pub use super::Client;
}
