pub use progenitor_client::{ByteStream, Error, ResponseValue};
#[allow(unused_imports)]
use progenitor_client::{encode_path, RequestBuilderExt};
#[allow(unused_imports)]
use reqwest::header::{HeaderMap, HeaderValue};
pub mod types {
    use serde::{Deserialize, Serialize};
    #[allow(unused_imports)]
    use std::convert::TryFrom;
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct AcceptInviteBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub username: Option<String>,
        pub workspace_id: String,
    }
    impl From<&AcceptInviteBody> for AcceptInviteBody {
        fn from(value: &AcceptInviteBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct AddGranularAclsBody {
        pub owner: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub write: Option<bool>,
    }
    impl From<&AddGranularAclsBody> for AddGranularAclsBody {
        fn from(value: &AddGranularAclsBody) -> Self {
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
    pub enum AddGranularAclsKind {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "group_")]
        Group,
        #[serde(rename = "resource")]
        Resource,
        #[serde(rename = "schedule")]
        Schedule,
        #[serde(rename = "variable")]
        Variable,
        #[serde(rename = "flow")]
        Flow,
        #[serde(rename = "folder")]
        Folder,
        #[serde(rename = "app")]
        App,
        #[serde(rename = "raw_app")]
        RawApp,
        #[serde(rename = "http_trigger")]
        HttpTrigger,
        #[serde(rename = "websocket_trigger")]
        WebsocketTrigger,
        #[serde(rename = "kafka_trigger")]
        KafkaTrigger,
        #[serde(rename = "nats_trigger")]
        NatsTrigger,
        #[serde(rename = "postgres_trigger")]
        PostgresTrigger,
        #[serde(rename = "mqtt_trigger")]
        MqttTrigger,
        #[serde(rename = "sqs_trigger")]
        SqsTrigger,
    }
    impl From<&AddGranularAclsKind> for AddGranularAclsKind {
        fn from(value: &AddGranularAclsKind) -> Self {
            value.clone()
        }
    }
    impl ToString for AddGranularAclsKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Group => "group_".to_string(),
                Self::Resource => "resource".to_string(),
                Self::Schedule => "schedule".to_string(),
                Self::Variable => "variable".to_string(),
                Self::Flow => "flow".to_string(),
                Self::Folder => "folder".to_string(),
                Self::App => "app".to_string(),
                Self::RawApp => "raw_app".to_string(),
                Self::HttpTrigger => "http_trigger".to_string(),
                Self::WebsocketTrigger => "websocket_trigger".to_string(),
                Self::KafkaTrigger => "kafka_trigger".to_string(),
                Self::NatsTrigger => "nats_trigger".to_string(),
                Self::PostgresTrigger => "postgres_trigger".to_string(),
                Self::MqttTrigger => "mqtt_trigger".to_string(),
                Self::SqsTrigger => "sqs_trigger".to_string(),
            }
        }
    }
    impl std::str::FromStr for AddGranularAclsKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "group_" => Ok(Self::Group),
                "resource" => Ok(Self::Resource),
                "schedule" => Ok(Self::Schedule),
                "variable" => Ok(Self::Variable),
                "flow" => Ok(Self::Flow),
                "folder" => Ok(Self::Folder),
                "app" => Ok(Self::App),
                "raw_app" => Ok(Self::RawApp),
                "http_trigger" => Ok(Self::HttpTrigger),
                "websocket_trigger" => Ok(Self::WebsocketTrigger),
                "kafka_trigger" => Ok(Self::KafkaTrigger),
                "nats_trigger" => Ok(Self::NatsTrigger),
                "postgres_trigger" => Ok(Self::PostgresTrigger),
                "mqtt_trigger" => Ok(Self::MqttTrigger),
                "sqs_trigger" => Ok(Self::SqsTrigger),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for AddGranularAclsKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for AddGranularAclsKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for AddGranularAclsKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct AddOwnerToFolderBody {
        pub owner: String,
    }
    impl From<&AddOwnerToFolderBody> for AddOwnerToFolderBody {
        fn from(value: &AddOwnerToFolderBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct AddUserBody {
        pub email: String,
        pub is_admin: bool,
        pub operator: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub username: Option<String>,
    }
    impl From<&AddUserBody> for AddUserBody {
        fn from(value: &AddUserBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct AddUserToGroupBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub username: Option<String>,
    }
    impl From<&AddUserToGroupBody> for AddUserToGroupBody {
        fn from(value: &AddUserToGroupBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct AddUserToInstanceGroupBody {
        pub email: String,
    }
    impl From<&AddUserToInstanceGroupBody> for AddUserToInstanceGroupBody {
        fn from(value: &AddUserToInstanceGroupBody) -> Self {
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
    pub struct ArchiveFlowByPathBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub archived: Option<bool>,
    }
    impl From<&ArchiveFlowByPathBody> for ArchiveFlowByPathBody {
        fn from(value: &ArchiveFlowByPathBody) -> Self {
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
    pub struct CancelPersistentQueuedJobsBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub reason: Option<String>,
    }
    impl From<&CancelPersistentQueuedJobsBody> for CancelPersistentQueuedJobsBody {
        fn from(value: &CancelPersistentQueuedJobsBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CancelQueuedJobBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub reason: Option<String>,
    }
    impl From<&CancelQueuedJobBody> for CancelQueuedJobBody {
        fn from(value: &CancelQueuedJobBody) -> Self {
            value.clone()
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
    pub struct ChangeWorkspaceColorBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub color: Option<String>,
    }
    impl From<&ChangeWorkspaceColorBody> for ChangeWorkspaceColorBody {
        fn from(value: &ChangeWorkspaceColorBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ChangeWorkspaceIdBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub new_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub new_name: Option<String>,
    }
    impl From<&ChangeWorkspaceIdBody> for ChangeWorkspaceIdBody {
        fn from(value: &ChangeWorkspaceIdBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ChangeWorkspaceNameBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub new_name: Option<String>,
    }
    impl From<&ChangeWorkspaceNameBody> for ChangeWorkspaceNameBody {
        fn from(value: &ChangeWorkspaceNameBody) -> Self {
            value.clone()
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
    pub enum ClearIndexIdxName {
        JobIndex,
        ServiceLogIndex,
    }
    impl From<&ClearIndexIdxName> for ClearIndexIdxName {
        fn from(value: &ClearIndexIdxName) -> Self {
            value.clone()
        }
    }
    impl ToString for ClearIndexIdxName {
        fn to_string(&self) -> String {
            match *self {
                Self::JobIndex => "JobIndex".to_string(),
                Self::ServiceLogIndex => "ServiceLogIndex".to_string(),
            }
        }
    }
    impl std::str::FromStr for ClearIndexIdxName {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "JobIndex" => Ok(Self::JobIndex),
                "ServiceLogIndex" => Ok(Self::ServiceLogIndex),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for ClearIndexIdxName {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for ClearIndexIdxName {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for ClearIndexIdxName {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
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
    pub struct ConnectCallbackBody {
        pub code: String,
        pub state: String,
    }
    impl From<&ConnectCallbackBody> for ConnectCallbackBody {
        fn from(value: &ConnectCallbackBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ConnectSlackCallbackBody {
        pub code: String,
        pub state: String,
    }
    impl From<&ConnectSlackCallbackBody> for ConnectSlackCallbackBody {
        fn from(value: &ConnectSlackCallbackBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ConnectSlackCallbackInstanceBody {
        pub code: String,
        pub state: String,
    }
    impl From<&ConnectSlackCallbackInstanceBody> for ConnectSlackCallbackInstanceBody {
        fn from(value: &ConnectSlackCallbackInstanceBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ConnectTeamsBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub team_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub team_name: Option<String>,
    }
    impl From<&ConnectTeamsBody> for ConnectTeamsBody {
        fn from(value: &ConnectTeamsBody) -> Self {
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
    pub struct CountJobsByTagResponseItem {
        pub count: i64,
        pub tag: String,
    }
    impl From<&CountJobsByTagResponseItem> for CountJobsByTagResponseItem {
        fn from(value: &CountJobsByTagResponseItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CountSearchLogsIndexResponse {
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub count_per_host: std::collections::HashMap<String, serde_json::Value>,
        ///a list of the terms that couldn't be parsed (and thus ignored)
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub query_parse_errors: Vec<String>,
    }
    impl From<&CountSearchLogsIndexResponse> for CountSearchLogsIndexResponse {
        fn from(value: &CountSearchLogsIndexResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CreateAccountBody {
        pub client: String,
        pub expires_in: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub refresh_token: Option<String>,
    }
    impl From<&CreateAccountBody> for CreateAccountBody {
        fn from(value: &CreateAccountBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CreateAppBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub custom_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deployment_message: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub draft_only: Option<bool>,
        pub path: String,
        pub policy: Policy,
        pub summary: String,
        pub value: serde_json::Value,
    }
    impl From<&CreateAppBody> for CreateAppBody {
        fn from(value: &CreateAppBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CreateDraftBody {
        pub path: String,
        pub typ: CreateDraftBodyTyp,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<serde_json::Value>,
    }
    impl From<&CreateDraftBody> for CreateDraftBody {
        fn from(value: &CreateDraftBody) -> Self {
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
    pub enum CreateDraftBodyTyp {
        #[serde(rename = "flow")]
        Flow,
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "app")]
        App,
    }
    impl From<&CreateDraftBodyTyp> for CreateDraftBodyTyp {
        fn from(value: &CreateDraftBodyTyp) -> Self {
            value.clone()
        }
    }
    impl ToString for CreateDraftBodyTyp {
        fn to_string(&self) -> String {
            match *self {
                Self::Flow => "flow".to_string(),
                Self::Script => "script".to_string(),
                Self::App => "app".to_string(),
            }
        }
    }
    impl std::str::FromStr for CreateDraftBodyTyp {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "flow" => Ok(Self::Flow),
                "script" => Ok(Self::Script),
                "app" => Ok(Self::App),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for CreateDraftBodyTyp {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for CreateDraftBodyTyp {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for CreateDraftBodyTyp {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
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
    pub struct CreateFolderBody {
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub extra_perms: std::collections::HashMap<String, bool>,
        pub name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub owners: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
    }
    impl From<&CreateFolderBody> for CreateFolderBody {
        fn from(value: &CreateFolderBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CreateGroupBody {
        pub name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
    }
    impl From<&CreateGroupBody> for CreateGroupBody {
        fn from(value: &CreateGroupBody) -> Self {
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
    pub struct CreateInstanceGroupBody {
        pub name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
    }
    impl From<&CreateInstanceGroupBody> for CreateInstanceGroupBody {
        fn from(value: &CreateInstanceGroupBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CreateRawAppBody {
        pub path: String,
        pub summary: String,
        pub value: String,
    }
    impl From<&CreateRawAppBody> for CreateRawAppBody {
        fn from(value: &CreateRawAppBody) -> Self {
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
    pub struct CreateUserGloballyBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub company: Option<String>,
        pub email: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        pub password: String,
        pub super_admin: bool,
    }
    impl From<&CreateUserGloballyBody> for CreateUserGloballyBody {
        fn from(value: &CreateUserGloballyBody) -> Self {
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
    pub struct DeclineInviteBody {
        pub workspace_id: String,
    }
    impl From<&DeclineInviteBody> for DeclineInviteBody {
        fn from(value: &DeclineInviteBody) -> Self {
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
    pub enum DeleteDraftKind {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "flow")]
        Flow,
        #[serde(rename = "app")]
        App,
    }
    impl From<&DeleteDraftKind> for DeleteDraftKind {
        fn from(value: &DeleteDraftKind) -> Self {
            value.clone()
        }
    }
    impl ToString for DeleteDraftKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Flow => "flow".to_string(),
                Self::App => "app".to_string(),
            }
        }
    }
    impl std::str::FromStr for DeleteDraftKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "flow" => Ok(Self::Flow),
                "app" => Ok(Self::App),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for DeleteDraftKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for DeleteDraftKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for DeleteDraftKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct DeleteInviteBody {
        pub email: String,
        pub is_admin: bool,
        pub operator: bool,
    }
    impl From<&DeleteInviteBody> for DeleteInviteBody {
        fn from(value: &DeleteInviteBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct DuckdbConnectionSettingsBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub s3_resource: Option<S3Resource>,
    }
    impl From<&DuckdbConnectionSettingsBody> for DuckdbConnectionSettingsBody {
        fn from(value: &DuckdbConnectionSettingsBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct DuckdbConnectionSettingsResponse {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub connection_settings_str: Option<String>,
    }
    impl From<&DuckdbConnectionSettingsResponse> for DuckdbConnectionSettingsResponse {
        fn from(value: &DuckdbConnectionSettingsResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct DuckdbConnectionSettingsV2Body {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub s3_resource_path: Option<String>,
    }
    impl From<&DuckdbConnectionSettingsV2Body> for DuckdbConnectionSettingsV2Body {
        fn from(value: &DuckdbConnectionSettingsV2Body) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct DuckdbConnectionSettingsV2Response {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub azure_container_path: Option<String>,
        pub connection_settings_str: String,
    }
    impl From<&DuckdbConnectionSettingsV2Response>
    for DuckdbConnectionSettingsV2Response {
        fn from(value: &DuckdbConnectionSettingsV2Response) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditAutoInviteBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub auto_add: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub invite_all: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub operator: Option<bool>,
    }
    impl From<&EditAutoInviteBody> for EditAutoInviteBody {
        fn from(value: &EditAutoInviteBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditCopilotConfigBody {
        pub ai_models: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ai_resource: Option<AiResource>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub code_completion_model: Option<String>,
    }
    impl From<&EditCopilotConfigBody> for EditCopilotConfigBody {
        fn from(value: &EditCopilotConfigBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditDeployToBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deploy_to: Option<String>,
    }
    impl From<&EditDeployToBody> for EditDeployToBody {
        fn from(value: &EditDeployToBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditErrorHandlerBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error_handler: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error_handler_extra_args: Option<ScriptArgs>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error_handler_muted_on_cancel: Option<bool>,
    }
    impl From<&EditErrorHandlerBody> for EditErrorHandlerBody {
        fn from(value: &EditErrorHandlerBody) -> Self {
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
    pub struct EditLargeFileStorageConfigBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub large_file_storage: Option<LargeFileStorage>,
    }
    impl From<&EditLargeFileStorageConfigBody> for EditLargeFileStorageConfigBody {
        fn from(value: &EditLargeFileStorageConfigBody) -> Self {
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
        pub description: Option<String>,
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
    pub struct EditSlackCommandBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub slack_command_script: Option<String>,
    }
    impl From<&EditSlackCommandBody> for EditSlackCommandBody {
        fn from(value: &EditSlackCommandBody) -> Self {
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
    pub struct EditTeamsCommandBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub slack_command_script: Option<String>,
    }
    impl From<&EditTeamsCommandBody> for EditTeamsCommandBody {
        fn from(value: &EditTeamsCommandBody) -> Self {
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
    pub struct EditWebhookBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub webhook: Option<String>,
    }
    impl From<&EditWebhookBody> for EditWebhookBody {
        fn from(value: &EditWebhookBody) -> Self {
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
    pub struct EditWorkspaceDefaultAppBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub default_app_path: Option<String>,
    }
    impl From<&EditWorkspaceDefaultAppBody> for EditWorkspaceDefaultAppBody {
        fn from(value: &EditWorkspaceDefaultAppBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditWorkspaceDeployUiSettingsBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deploy_ui_settings: Option<WorkspaceDeployUiSettings>,
    }
    impl From<&EditWorkspaceDeployUiSettingsBody> for EditWorkspaceDeployUiSettingsBody {
        fn from(value: &EditWorkspaceDeployUiSettingsBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct EditWorkspaceGitSyncConfigBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub git_sync_settings: Option<WorkspaceGitSyncSettings>,
    }
    impl From<&EditWorkspaceGitSyncConfigBody> for EditWorkspaceGitSyncConfigBody {
        fn from(value: &EditWorkspaceGitSyncConfigBody) -> Self {
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
    pub struct ExecuteComponentBody {
        pub args: serde_json::Value,
        pub component: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub force_viewer_allow_user_resources: Vec<String>,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub force_viewer_one_of_fields: std::collections::HashMap<
            String,
            serde_json::Value,
        >,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub force_viewer_static_fields: std::collections::HashMap<
            String,
            serde_json::Value,
        >,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub id: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub raw_code: Option<ExecuteComponentBodyRawCode>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub version: Option<i64>,
    }
    impl From<&ExecuteComponentBody> for ExecuteComponentBody {
        fn from(value: &ExecuteComponentBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ExecuteComponentBodyRawCode {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cache_ttl: Option<i64>,
        pub content: String,
        pub language: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub lock: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub path: Option<String>,
    }
    impl From<&ExecuteComponentBodyRawCode> for ExecuteComponentBodyRawCode {
        fn from(value: &ExecuteComponentBodyRawCode) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ExistsRouteBody {
        pub http_method: ExistsRouteBodyHttpMethod,
        pub route_path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub trigger_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspaced_route: Option<bool>,
    }
    impl From<&ExistsRouteBody> for ExistsRouteBody {
        fn from(value: &ExistsRouteBody) -> Self {
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
    pub enum ExistsRouteBodyHttpMethod {
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
    impl From<&ExistsRouteBodyHttpMethod> for ExistsRouteBodyHttpMethod {
        fn from(value: &ExistsRouteBodyHttpMethod) -> Self {
            value.clone()
        }
    }
    impl ToString for ExistsRouteBodyHttpMethod {
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
    impl std::str::FromStr for ExistsRouteBodyHttpMethod {
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
    impl std::convert::TryFrom<&str> for ExistsRouteBodyHttpMethod {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for ExistsRouteBodyHttpMethod {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for ExistsRouteBodyHttpMethod {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ExistsUsernameBody {
        pub id: String,
        pub username: String,
    }
    impl From<&ExistsUsernameBody> for ExistsUsernameBody {
        fn from(value: &ExistsUsernameBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ExistsWorkspaceBody {
        pub id: String,
    }
    impl From<&ExistsWorkspaceBody> for ExistsWorkspaceBody {
        fn from(value: &ExistsWorkspaceBody) -> Self {
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
    pub struct FileUploadResponse {
        pub file_key: String,
    }
    impl From<&FileUploadResponse> for FileUploadResponse {
        fn from(value: &FileUploadResponse) -> Self {
            value.clone()
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error_message: Option<String>
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
    pub struct ForceCancelQueuedJobBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub reason: Option<String>,
    }
    impl From<&ForceCancelQueuedJobBody> for ForceCancelQueuedJobBody {
        fn from(value: &ForceCancelQueuedJobBody) -> Self {
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
    pub enum GetCaptureConfigsRunnableKind {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "flow")]
        Flow,
    }
    impl From<&GetCaptureConfigsRunnableKind> for GetCaptureConfigsRunnableKind {
        fn from(value: &GetCaptureConfigsRunnableKind) -> Self {
            value.clone()
        }
    }
    impl ToString for GetCaptureConfigsRunnableKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Flow => "flow".to_string(),
            }
        }
    }
    impl std::str::FromStr for GetCaptureConfigsRunnableKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "flow" => Ok(Self::Flow),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for GetCaptureConfigsRunnableKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for GetCaptureConfigsRunnableKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for GetCaptureConfigsRunnableKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetCompletedCountResponse {
        pub database_length: i64,
    }
    impl From<&GetCompletedCountResponse> for GetCompletedCountResponse {
        fn from(value: &GetCompletedCountResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetCompletedJobResultMaybeResponse {
        pub completed: bool,
        pub result: serde_json::Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub started: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub success: Option<bool>,
    }
    impl From<&GetCompletedJobResultMaybeResponse>
    for GetCompletedJobResultMaybeResponse {
        fn from(value: &GetCompletedJobResultMaybeResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetCriticalAlertsResponse {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub alerts: Vec<CriticalAlert>,
        ///Total number of pages based on the page size.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub total_pages: Option<i64>,
        ///Total number of rows matching the query.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub total_rows: Option<i64>,
    }
    impl From<&GetCriticalAlertsResponse> for GetCriticalAlertsResponse {
        fn from(value: &GetCriticalAlertsResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetDeployToResponse {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deploy_to: Option<String>,
    }
    impl From<&GetDeployToResponse> for GetDeployToResponse {
        fn from(value: &GetDeployToResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetFlowByPathWithDraftResponse {
        #[serde(flatten)]
        pub flow: Flow,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub draft: Option<Flow>,
    }
    impl From<&GetFlowByPathWithDraftResponse> for GetFlowByPathWithDraftResponse {
        fn from(value: &GetFlowByPathWithDraftResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetFlowDeploymentStatusResponse {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub lock_error_logs: Option<String>,
    }
    impl From<&GetFlowDeploymentStatusResponse> for GetFlowDeploymentStatusResponse {
        fn from(value: &GetFlowDeploymentStatusResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetFolderUsageResponse {
        pub apps: f64,
        pub flows: f64,
        pub resources: f64,
        pub schedules: f64,
        pub scripts: f64,
        pub variables: f64,
    }
    impl From<&GetFolderUsageResponse> for GetFolderUsageResponse {
        fn from(value: &GetFolderUsageResponse) -> Self {
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
    pub enum GetGranularAclsKind {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "group_")]
        Group,
        #[serde(rename = "resource")]
        Resource,
        #[serde(rename = "schedule")]
        Schedule,
        #[serde(rename = "variable")]
        Variable,
        #[serde(rename = "flow")]
        Flow,
        #[serde(rename = "folder")]
        Folder,
        #[serde(rename = "app")]
        App,
        #[serde(rename = "raw_app")]
        RawApp,
        #[serde(rename = "http_trigger")]
        HttpTrigger,
        #[serde(rename = "websocket_trigger")]
        WebsocketTrigger,
        #[serde(rename = "kafka_trigger")]
        KafkaTrigger,
        #[serde(rename = "nats_trigger")]
        NatsTrigger,
        #[serde(rename = "postgres_trigger")]
        PostgresTrigger,
        #[serde(rename = "mqtt_trigger")]
        MqttTrigger,
        #[serde(rename = "sqs_trigger")]
        SqsTrigger,
    }
    impl From<&GetGranularAclsKind> for GetGranularAclsKind {
        fn from(value: &GetGranularAclsKind) -> Self {
            value.clone()
        }
    }
    impl ToString for GetGranularAclsKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Group => "group_".to_string(),
                Self::Resource => "resource".to_string(),
                Self::Schedule => "schedule".to_string(),
                Self::Variable => "variable".to_string(),
                Self::Flow => "flow".to_string(),
                Self::Folder => "folder".to_string(),
                Self::App => "app".to_string(),
                Self::RawApp => "raw_app".to_string(),
                Self::HttpTrigger => "http_trigger".to_string(),
                Self::WebsocketTrigger => "websocket_trigger".to_string(),
                Self::KafkaTrigger => "kafka_trigger".to_string(),
                Self::NatsTrigger => "nats_trigger".to_string(),
                Self::PostgresTrigger => "postgres_trigger".to_string(),
                Self::MqttTrigger => "mqtt_trigger".to_string(),
                Self::SqsTrigger => "sqs_trigger".to_string(),
            }
        }
    }
    impl std::str::FromStr for GetGranularAclsKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "group_" => Ok(Self::Group),
                "resource" => Ok(Self::Resource),
                "schedule" => Ok(Self::Schedule),
                "variable" => Ok(Self::Variable),
                "flow" => Ok(Self::Flow),
                "folder" => Ok(Self::Folder),
                "app" => Ok(Self::App),
                "raw_app" => Ok(Self::RawApp),
                "http_trigger" => Ok(Self::HttpTrigger),
                "websocket_trigger" => Ok(Self::WebsocketTrigger),
                "kafka_trigger" => Ok(Self::KafkaTrigger),
                "nats_trigger" => Ok(Self::NatsTrigger),
                "postgres_trigger" => Ok(Self::PostgresTrigger),
                "mqtt_trigger" => Ok(Self::MqttTrigger),
                "sqs_trigger" => Ok(Self::SqsTrigger),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for GetGranularAclsKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for GetGranularAclsKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for GetGranularAclsKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetHubAppByIdResponse {
        pub app: GetHubAppByIdResponseApp,
    }
    impl From<&GetHubAppByIdResponse> for GetHubAppByIdResponse {
        fn from(value: &GetHubAppByIdResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetHubAppByIdResponseApp {
        pub summary: String,
        pub value: serde_json::Value,
    }
    impl From<&GetHubAppByIdResponseApp> for GetHubAppByIdResponseApp {
        fn from(value: &GetHubAppByIdResponseApp) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetHubFlowByIdResponse {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub flow: Option<OpenFlow>,
    }
    impl From<&GetHubFlowByIdResponse> for GetHubFlowByIdResponse {
        fn from(value: &GetHubFlowByIdResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetHubScriptByPathResponse {
        pub content: String,
        pub language: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub lockfile: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub schema: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
    }
    impl From<&GetHubScriptByPathResponse> for GetHubScriptByPathResponse {
        fn from(value: &GetHubScriptByPathResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetJobMetricsBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub from_timestamp: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub timeseries_max_datapoints: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub to_timestamp: Option<chrono::DateTime<chrono::offset::Utc>>,
    }
    impl From<&GetJobMetricsBody> for GetJobMetricsBody {
        fn from(value: &GetJobMetricsBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetJobMetricsResponse {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub metrics_metadata: Vec<MetricMetadata>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub scalar_metrics: Vec<ScalarMetric>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub timeseries_metrics: Vec<TimeseriesMetric>,
    }
    impl From<&GetJobMetricsResponse> for GetJobMetricsResponse {
        fn from(value: &GetJobMetricsResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetJobUpdatesResponse {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub completed: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub flow_status: Option<WorkflowStatusRecord>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub log_offset: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub mem_peak: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub new_logs: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub progress: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub running: Option<bool>,
    }
    impl From<&GetJobUpdatesResponse> for GetJobUpdatesResponse {
        fn from(value: &GetJobUpdatesResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetLatestKeyRenewalAttemptResponse {
        pub attempted_at: chrono::DateTime<chrono::offset::Utc>,
        pub result: String,
    }
    impl From<&GetLatestKeyRenewalAttemptResponse>
    for GetLatestKeyRenewalAttemptResponse {
        fn from(value: &GetLatestKeyRenewalAttemptResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetOAuthConnectResponse {
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub extra_params: std::collections::HashMap<String, serde_json::Value>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub scopes: Vec<String>,
    }
    impl From<&GetOAuthConnectResponse> for GetOAuthConnectResponse {
        fn from(value: &GetOAuthConnectResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetPremiumInfoResponse {
        pub automatic_billing: bool,
        pub owner: String,
        pub premium: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub seats: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub usage: Option<f64>,
    }
    impl From<&GetPremiumInfoResponse> for GetPremiumInfoResponse {
        fn from(value: &GetPremiumInfoResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetQueueCountResponse {
        pub database_length: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub suspended: Option<i64>,
    }
    impl From<&GetQueueCountResponse> for GetQueueCountResponse {
        fn from(value: &GetQueueCountResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetQueueMetricsResponseItem {
        pub id: String,
        pub values: Vec<GetQueueMetricsResponseItemValuesItem>,
    }
    impl From<&GetQueueMetricsResponseItem> for GetQueueMetricsResponseItem {
        fn from(value: &GetQueueMetricsResponseItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetQueueMetricsResponseItemValuesItem {
        pub created_at: String,
        pub value: f64,
    }
    impl From<&GetQueueMetricsResponseItemValuesItem>
    for GetQueueMetricsResponseItemValuesItem {
        fn from(value: &GetQueueMetricsResponseItemValuesItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetResumeUrlsResponse {
        #[serde(rename = "approvalPage")]
        pub approval_page: String,
        pub cancel: String,
        pub resume: String,
    }
    impl From<&GetResumeUrlsResponse> for GetResumeUrlsResponse {
        fn from(value: &GetResumeUrlsResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetRunnableResponse {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        pub endpoint_async: String,
        pub endpoint_openai_sync: String,
        pub endpoint_sync: String,
        pub kind: String,
        pub summary: String,
        pub workspace: String,
    }
    impl From<&GetRunnableResponse> for GetRunnableResponse {
        fn from(value: &GetRunnableResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetScriptDeploymentStatusResponse {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub lock: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub lock_error_logs: Option<String>,
    }
    impl From<&GetScriptDeploymentStatusResponse> for GetScriptDeploymentStatusResponse {
        fn from(value: &GetScriptDeploymentStatusResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetSettingsResponse {
        pub ai_models: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ai_resource: Option<AiResource>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub auto_add: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub auto_invite_domain: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub auto_invite_operator: Option<bool>,
        pub automatic_billing: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub code_completion_model: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub color: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub customer_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub default_app: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub default_scripts: Option<WorkspaceDefaultScripts>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deploy_to: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deploy_ui: Option<WorkspaceDeployUiSettings>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error_handler: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error_handler_extra_args: Option<ScriptArgs>,
        pub error_handler_muted_on_cancel: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub git_sync: Option<WorkspaceGitSyncSettings>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub large_file_storage: Option<LargeFileStorage>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub mute_critical_alerts: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub operator_settings: Option<OperatorSettings>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub plan: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub slack_command_script: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub slack_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub slack_team_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub teams_command_script: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub teams_team_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub teams_team_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub webhook: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspace_id: Option<String>,
    }
    impl From<&GetSettingsResponse> for GetSettingsResponse {
        fn from(value: &GetSettingsResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetSuspendedJobFlowResponse {
        pub approvers: Vec<GetSuspendedJobFlowResponseApproversItem>,
        pub job: Job,
    }
    impl From<&GetSuspendedJobFlowResponse> for GetSuspendedJobFlowResponse {
        fn from(value: &GetSuspendedJobFlowResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetSuspendedJobFlowResponseApproversItem {
        pub approver: String,
        pub resume_id: i64,
    }
    impl From<&GetSuspendedJobFlowResponseApproversItem>
    for GetSuspendedJobFlowResponseApproversItem {
        fn from(value: &GetSuspendedJobFlowResponseApproversItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetThresholdAlertResponse {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub last_alert_sent: Option<chrono::DateTime<chrono::offset::Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub threshold_alert_amount: Option<f64>,
    }
    impl From<&GetThresholdAlertResponse> for GetThresholdAlertResponse {
        fn from(value: &GetThresholdAlertResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetTopHubScriptsResponse {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub asks: Vec<GetTopHubScriptsResponseAsksItem>,
    }
    impl From<&GetTopHubScriptsResponse> for GetTopHubScriptsResponse {
        fn from(value: &GetTopHubScriptsResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetTopHubScriptsResponseAsksItem {
        pub app: String,
        pub ask_id: f64,
        pub id: f64,
        pub kind: HubScriptKind,
        pub summary: String,
        pub version_id: f64,
        pub views: f64,
        pub votes: f64,
    }
    impl From<&GetTopHubScriptsResponseAsksItem> for GetTopHubScriptsResponseAsksItem {
        fn from(value: &GetTopHubScriptsResponseAsksItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetTutorialProgressResponse {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub progress: Option<i64>,
    }
    impl From<&GetTutorialProgressResponse> for GetTutorialProgressResponse {
        fn from(value: &GetTutorialProgressResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetUsedTriggersResponse {
        pub http_routes_used: bool,
        pub kafka_used: bool,
        pub mqtt_used: bool,
        pub nats_used: bool,
        pub postgres_used: bool,
        pub sqs_used: bool,
        pub websocket_used: bool,
    }
    impl From<&GetUsedTriggersResponse> for GetUsedTriggersResponse {
        fn from(value: &GetUsedTriggersResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetWorkspaceDefaultAppResponse {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub default_app_path: Option<String>,
    }
    impl From<&GetWorkspaceDefaultAppResponse> for GetWorkspaceDefaultAppResponse {
        fn from(value: &GetWorkspaceDefaultAppResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GetWorkspaceEncryptionKeyResponse {
        pub key: String,
    }
    impl From<&GetWorkspaceEncryptionKeyResponse> for GetWorkspaceEncryptionKeyResponse {
        fn from(value: &GetWorkspaceEncryptionKeyResponse) -> Self {
            value.clone()
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
    pub struct GlobalUserRenameBody {
        pub new_username: String,
    }
    impl From<&GlobalUserRenameBody> for GlobalUserRenameBody {
        fn from(value: &GlobalUserRenameBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GlobalUserUpdateBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub is_devops: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub is_super_admin: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
    }
    impl From<&GlobalUserUpdateBody> for GlobalUserUpdateBody {
        fn from(value: &GlobalUserUpdateBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GlobalUsernameInfoResponse {
        pub username: String,
        pub workspace_usernames: Vec<GlobalUsernameInfoResponseWorkspaceUsernamesItem>,
    }
    impl From<&GlobalUsernameInfoResponse> for GlobalUsernameInfoResponse {
        fn from(value: &GlobalUsernameInfoResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct GlobalUsernameInfoResponseWorkspaceUsernamesItem {
        pub username: String,
        pub workspace_id: String,
    }
    impl From<&GlobalUsernameInfoResponseWorkspaceUsernamesItem>
    for GlobalUsernameInfoResponseWorkspaceUsernamesItem {
        fn from(value: &GlobalUsernameInfoResponseWorkspaceUsernamesItem) -> Self {
            value.clone()
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
    pub struct InviteUserBody {
        pub email: String,
        pub is_admin: bool,
        pub operator: bool,
    }
    impl From<&InviteUserBody> for InviteUserBody {
        fn from(value: &InviteUserBody) -> Self {
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
    pub enum ListAuditLogsActionKind {
        Create,
        Update,
        Delete,
        Execute,
    }
    impl From<&ListAuditLogsActionKind> for ListAuditLogsActionKind {
        fn from(value: &ListAuditLogsActionKind) -> Self {
            value.clone()
        }
    }
    impl ToString for ListAuditLogsActionKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Create => "Create".to_string(),
                Self::Update => "Update".to_string(),
                Self::Delete => "Delete".to_string(),
                Self::Execute => "Execute".to_string(),
            }
        }
    }
    impl std::str::FromStr for ListAuditLogsActionKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "Create" => Ok(Self::Create),
                "Update" => Ok(Self::Update),
                "Delete" => Ok(Self::Delete),
                "Execute" => Ok(Self::Execute),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for ListAuditLogsActionKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for ListAuditLogsActionKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for ListAuditLogsActionKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListAvailableTeamsChannelsResponseItem {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub channel_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub channel_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub service_url: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tenant_id: Option<String>,
    }
    impl From<&ListAvailableTeamsChannelsResponseItem>
    for ListAvailableTeamsChannelsResponseItem {
        fn from(value: &ListAvailableTeamsChannelsResponseItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListAvailableTeamsIdsResponseItem {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub team_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub team_name: Option<String>,
    }
    impl From<&ListAvailableTeamsIdsResponseItem> for ListAvailableTeamsIdsResponseItem {
        fn from(value: &ListAvailableTeamsIdsResponseItem) -> Self {
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
    pub enum ListCapturesRunnableKind {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "flow")]
        Flow,
    }
    impl From<&ListCapturesRunnableKind> for ListCapturesRunnableKind {
        fn from(value: &ListCapturesRunnableKind) -> Self {
            value.clone()
        }
    }
    impl ToString for ListCapturesRunnableKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Flow => "flow".to_string(),
            }
        }
    }
    impl std::str::FromStr for ListCapturesRunnableKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "flow" => Ok(Self::Flow),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for ListCapturesRunnableKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for ListCapturesRunnableKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for ListCapturesRunnableKind {
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
    pub enum ListFlowPathsFromWorkspaceRunnableRunnableKind {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "flow")]
        Flow,
    }
    impl From<&ListFlowPathsFromWorkspaceRunnableRunnableKind>
    for ListFlowPathsFromWorkspaceRunnableRunnableKind {
        fn from(value: &ListFlowPathsFromWorkspaceRunnableRunnableKind) -> Self {
            value.clone()
        }
    }
    impl ToString for ListFlowPathsFromWorkspaceRunnableRunnableKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Flow => "flow".to_string(),
            }
        }
    }
    impl std::str::FromStr for ListFlowPathsFromWorkspaceRunnableRunnableKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "flow" => Ok(Self::Flow),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for ListFlowPathsFromWorkspaceRunnableRunnableKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String>
    for ListFlowPathsFromWorkspaceRunnableRunnableKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String>
    for ListFlowPathsFromWorkspaceRunnableRunnableKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListFlowsResponseItem {
        #[serde(flatten)]
        pub flow: Flow,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub draft_only: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub has_draft: Option<bool>,
    }
    impl From<&ListFlowsResponseItem> for ListFlowsResponseItem {
        fn from(value: &ListFlowsResponseItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListHubAppsResponse {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub apps: Vec<ListHubAppsResponseAppsItem>,
    }
    impl From<&ListHubAppsResponse> for ListHubAppsResponse {
        fn from(value: &ListHubAppsResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListHubAppsResponseAppsItem {
        pub app_id: f64,
        pub approved: bool,
        pub apps: Vec<String>,
        pub id: f64,
        pub summary: String,
        pub votes: f64,
    }
    impl From<&ListHubAppsResponseAppsItem> for ListHubAppsResponseAppsItem {
        fn from(value: &ListHubAppsResponseAppsItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListHubFlowsResponse {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub flows: Vec<ListHubFlowsResponseFlowsItem>,
    }
    impl From<&ListHubFlowsResponse> for ListHubFlowsResponse {
        fn from(value: &ListHubFlowsResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListHubFlowsResponseFlowsItem {
        pub approved: bool,
        pub apps: Vec<String>,
        pub flow_id: f64,
        pub id: f64,
        pub summary: String,
        pub votes: f64,
    }
    impl From<&ListHubFlowsResponseFlowsItem> for ListHubFlowsResponseFlowsItem {
        fn from(value: &ListHubFlowsResponseFlowsItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListHubIntegrationsResponseItem {
        pub name: String,
    }
    impl From<&ListHubIntegrationsResponseItem> for ListHubIntegrationsResponseItem {
        fn from(value: &ListHubIntegrationsResponseItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListLogFilesResponseItem {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub err_lines: Option<i64>,
        pub file_path: String,
        pub hostname: String,
        pub json_fmt: bool,
        pub log_ts: chrono::DateTime<chrono::offset::Utc>,
        pub mode: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ok_lines: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub worker_group: Option<String>,
    }
    impl From<&ListLogFilesResponseItem> for ListLogFilesResponseItem {
        fn from(value: &ListLogFilesResponseItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListOAuthLoginsResponse {
        pub oauth: Vec<ListOAuthLoginsResponseOauthItem>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub saml: Option<String>,
    }
    impl From<&ListOAuthLoginsResponse> for ListOAuthLoginsResponse {
        fn from(value: &ListOAuthLoginsResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListOAuthLoginsResponseOauthItem {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub display_name: Option<String>,
        #[serde(rename = "type")]
        pub type_: String,
    }
    impl From<&ListOAuthLoginsResponseOauthItem> for ListOAuthLoginsResponseOauthItem {
        fn from(value: &ListOAuthLoginsResponseOauthItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListResourceNamesResponseItem {
        pub name: String,
        pub path: String,
    }
    impl From<&ListResourceNamesResponseItem> for ListResourceNamesResponseItem {
        fn from(value: &ListResourceNamesResponseItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListSearchAppResponseItem {
        pub path: String,
        pub value: serde_json::Value,
    }
    impl From<&ListSearchAppResponseItem> for ListSearchAppResponseItem {
        fn from(value: &ListSearchAppResponseItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListSearchFlowResponseItem {
        pub path: String,
        pub value: serde_json::Value,
    }
    impl From<&ListSearchFlowResponseItem> for ListSearchFlowResponseItem {
        fn from(value: &ListSearchFlowResponseItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListSearchResourceResponseItem {
        pub path: String,
        pub value: serde_json::Value,
    }
    impl From<&ListSearchResourceResponseItem> for ListSearchResourceResponseItem {
        fn from(value: &ListSearchResourceResponseItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListSearchScriptResponseItem {
        pub content: String,
        pub path: String,
    }
    impl From<&ListSearchScriptResponseItem> for ListSearchScriptResponseItem {
        fn from(value: &ListSearchScriptResponseItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListStoredFilesResponse {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub next_marker: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub restricted_access: Option<bool>,
        pub windmill_large_files: Vec<WindmillLargeFile>,
    }
    impl From<&ListStoredFilesResponse> for ListStoredFilesResponse {
        fn from(value: &ListStoredFilesResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ListWorkerGroupsResponseItem {
        pub config: serde_json::Value,
        pub name: String,
    }
    impl From<&ListWorkerGroupsResponseItem> for ListWorkerGroupsResponseItem {
        fn from(value: &ListWorkerGroupsResponseItem) -> Self {
            value.clone()
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
    pub struct LoadTableRowCountResponse {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub count: Option<f64>,
    }
    impl From<&LoadTableRowCountResponse> for LoadTableRowCountResponse {
        fn from(value: &LoadTableRowCountResponse) -> Self {
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
    pub struct LoginWithOauthBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub code: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub state: Option<String>,
    }
    impl From<&LoginWithOauthBody> for LoginWithOauthBody {
        fn from(value: &LoginWithOauthBody) -> Self {
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
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct MoveCapturesAndConfigsBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub new_path: Option<String>,
    }
    impl From<&MoveCapturesAndConfigsBody> for MoveCapturesAndConfigsBody {
        fn from(value: &MoveCapturesAndConfigsBody) -> Self {
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
    pub enum MoveCapturesAndConfigsRunnableKind {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "flow")]
        Flow,
    }
    impl From<&MoveCapturesAndConfigsRunnableKind>
    for MoveCapturesAndConfigsRunnableKind {
        fn from(value: &MoveCapturesAndConfigsRunnableKind) -> Self {
            value.clone()
        }
    }
    impl ToString for MoveCapturesAndConfigsRunnableKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Flow => "flow".to_string(),
            }
        }
    }
    impl std::str::FromStr for MoveCapturesAndConfigsRunnableKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "flow" => Ok(Self::Flow),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for MoveCapturesAndConfigsRunnableKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for MoveCapturesAndConfigsRunnableKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for MoveCapturesAndConfigsRunnableKind {
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
        pub description: Option<String>,
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
    pub enum PingCaptureConfigRunnableKind {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "flow")]
        Flow,
    }
    impl From<&PingCaptureConfigRunnableKind> for PingCaptureConfigRunnableKind {
        fn from(value: &PingCaptureConfigRunnableKind) -> Self {
            value.clone()
        }
    }
    impl ToString for PingCaptureConfigRunnableKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Flow => "flow".to_string(),
            }
        }
    }
    impl std::str::FromStr for PingCaptureConfigRunnableKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "flow" => Ok(Self::Flow),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for PingCaptureConfigRunnableKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for PingCaptureConfigRunnableKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for PingCaptureConfigRunnableKind {
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
    pub struct PolarsConnectionSettingsBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub s3_resource: Option<S3Resource>,
    }
    impl From<&PolarsConnectionSettingsBody> for PolarsConnectionSettingsBody {
        fn from(value: &PolarsConnectionSettingsBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct PolarsConnectionSettingsResponse {
        pub cache_regions: bool,
        pub client_kwargs: PolarsClientKwargs,
        pub endpoint_url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub key: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub secret: Option<String>,
        pub use_ssl: bool,
    }
    impl From<&PolarsConnectionSettingsResponse> for PolarsConnectionSettingsResponse {
        fn from(value: &PolarsConnectionSettingsResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct PolarsConnectionSettingsV2Body {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub s3_resource_path: Option<String>,
    }
    impl From<&PolarsConnectionSettingsV2Body> for PolarsConnectionSettingsV2Body {
        fn from(value: &PolarsConnectionSettingsV2Body) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct PolarsConnectionSettingsV2Response {
        pub s3fs_args: PolarsConnectionSettingsV2ResponseS3fsArgs,
        pub storage_options: PolarsConnectionSettingsV2ResponseStorageOptions,
    }
    impl From<&PolarsConnectionSettingsV2Response>
    for PolarsConnectionSettingsV2Response {
        fn from(value: &PolarsConnectionSettingsV2Response) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct PolarsConnectionSettingsV2ResponseS3fsArgs {
        pub cache_regions: bool,
        pub client_kwargs: PolarsClientKwargs,
        pub endpoint_url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub key: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub secret: Option<String>,
        pub use_ssl: bool,
    }
    impl From<&PolarsConnectionSettingsV2ResponseS3fsArgs>
    for PolarsConnectionSettingsV2ResponseS3fsArgs {
        fn from(value: &PolarsConnectionSettingsV2ResponseS3fsArgs) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct PolarsConnectionSettingsV2ResponseStorageOptions {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub aws_access_key_id: Option<String>,
        pub aws_allow_http: String,
        pub aws_endpoint_url: String,
        pub aws_region: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub aws_secret_access_key: Option<String>,
    }
    impl From<&PolarsConnectionSettingsV2ResponseStorageOptions>
    for PolarsConnectionSettingsV2ResponseStorageOptions {
        fn from(value: &PolarsConnectionSettingsV2ResponseStorageOptions) -> Self {
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
    pub struct PreviewScheduleBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cron_version: Option<String>,
        pub schedule: String,
        pub timezone: String,
    }
    impl From<&PreviewScheduleBody> for PreviewScheduleBody {
        fn from(value: &PreviewScheduleBody) -> Self {
            value.clone()
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
    pub struct QueryHubScriptsResponseItem {
        pub app: String,
        pub ask_id: f64,
        pub id: f64,
        pub kind: HubScriptKind,
        pub score: f64,
        pub summary: String,
        pub version_id: f64,
    }
    impl From<&QueryHubScriptsResponseItem> for QueryHubScriptsResponseItem {
        fn from(value: &QueryHubScriptsResponseItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct QueryResourceTypesResponseItem {
        pub name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub schema: Option<serde_json::Value>,
        pub score: f64,
    }
    impl From<&QueryResourceTypesResponseItem> for QueryResourceTypesResponseItem {
        fn from(value: &QueryResourceTypesResponseItem) -> Self {
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
    pub struct RefreshTokenBody {
        pub path: String,
    }
    impl From<&RefreshTokenBody> for RefreshTokenBody {
        fn from(value: &RefreshTokenBody) -> Self {
            value.clone()
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
    pub struct RemoveGranularAclsBody {
        pub owner: String,
    }
    impl From<&RemoveGranularAclsBody> for RemoveGranularAclsBody {
        fn from(value: &RemoveGranularAclsBody) -> Self {
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
    pub enum RemoveGranularAclsKind {
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "group_")]
        Group,
        #[serde(rename = "resource")]
        Resource,
        #[serde(rename = "schedule")]
        Schedule,
        #[serde(rename = "variable")]
        Variable,
        #[serde(rename = "flow")]
        Flow,
        #[serde(rename = "folder")]
        Folder,
        #[serde(rename = "app")]
        App,
        #[serde(rename = "raw_app")]
        RawApp,
        #[serde(rename = "http_trigger")]
        HttpTrigger,
        #[serde(rename = "websocket_trigger")]
        WebsocketTrigger,
        #[serde(rename = "kafka_trigger")]
        KafkaTrigger,
        #[serde(rename = "nats_trigger")]
        NatsTrigger,
        #[serde(rename = "postgres_trigger")]
        PostgresTrigger,
        #[serde(rename = "mqtt_trigger")]
        MqttTrigger,
        #[serde(rename = "sqs_trigger")]
        SqsTrigger,
    }
    impl From<&RemoveGranularAclsKind> for RemoveGranularAclsKind {
        fn from(value: &RemoveGranularAclsKind) -> Self {
            value.clone()
        }
    }
    impl ToString for RemoveGranularAclsKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Script => "script".to_string(),
                Self::Group => "group_".to_string(),
                Self::Resource => "resource".to_string(),
                Self::Schedule => "schedule".to_string(),
                Self::Variable => "variable".to_string(),
                Self::Flow => "flow".to_string(),
                Self::Folder => "folder".to_string(),
                Self::App => "app".to_string(),
                Self::RawApp => "raw_app".to_string(),
                Self::HttpTrigger => "http_trigger".to_string(),
                Self::WebsocketTrigger => "websocket_trigger".to_string(),
                Self::KafkaTrigger => "kafka_trigger".to_string(),
                Self::NatsTrigger => "nats_trigger".to_string(),
                Self::PostgresTrigger => "postgres_trigger".to_string(),
                Self::MqttTrigger => "mqtt_trigger".to_string(),
                Self::SqsTrigger => "sqs_trigger".to_string(),
            }
        }
    }
    impl std::str::FromStr for RemoveGranularAclsKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "script" => Ok(Self::Script),
                "group_" => Ok(Self::Group),
                "resource" => Ok(Self::Resource),
                "schedule" => Ok(Self::Schedule),
                "variable" => Ok(Self::Variable),
                "flow" => Ok(Self::Flow),
                "folder" => Ok(Self::Folder),
                "app" => Ok(Self::App),
                "raw_app" => Ok(Self::RawApp),
                "http_trigger" => Ok(Self::HttpTrigger),
                "websocket_trigger" => Ok(Self::WebsocketTrigger),
                "kafka_trigger" => Ok(Self::KafkaTrigger),
                "nats_trigger" => Ok(Self::NatsTrigger),
                "postgres_trigger" => Ok(Self::PostgresTrigger),
                "mqtt_trigger" => Ok(Self::MqttTrigger),
                "sqs_trigger" => Ok(Self::SqsTrigger),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for RemoveGranularAclsKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for RemoveGranularAclsKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for RemoveGranularAclsKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RemoveOwnerToFolderBody {
        pub owner: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub write: Option<bool>,
    }
    impl From<&RemoveOwnerToFolderBody> for RemoveOwnerToFolderBody {
        fn from(value: &RemoveOwnerToFolderBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RemoveUserFromInstanceGroupBody {
        pub email: String,
    }
    impl From<&RemoveUserFromInstanceGroupBody> for RemoveUserFromInstanceGroupBody {
        fn from(value: &RemoveUserFromInstanceGroupBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RemoveUserToGroupBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub username: Option<String>,
    }
    impl From<&RemoveUserToGroupBody> for RemoveUserToGroupBody {
        fn from(value: &RemoveUserToGroupBody) -> Self {
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
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RunRawScriptDependenciesBody {
        pub entrypoint: String,
        pub raw_scripts: Vec<RawScriptForDependencies>,
    }
    impl From<&RunRawScriptDependenciesBody> for RunRawScriptDependenciesBody {
        fn from(value: &RunRawScriptDependenciesBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RunRawScriptDependenciesResponse {
        pub lock: String,
    }
    impl From<&RunRawScriptDependenciesResponse> for RunRawScriptDependenciesResponse {
        fn from(value: &RunRawScriptDependenciesResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RunSlackMessageTestJobBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub channel: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub hub_script_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub test_msg: Option<String>,
    }
    impl From<&RunSlackMessageTestJobBody> for RunSlackMessageTestJobBody {
        fn from(value: &RunSlackMessageTestJobBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RunTeamsMessageTestJobBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub channel: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub hub_script_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub test_msg: Option<String>,
    }
    impl From<&RunTeamsMessageTestJobBody> for RunTeamsMessageTestJobBody {
        fn from(value: &RunTeamsMessageTestJobBody) -> Self {
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
    pub struct S3ResourceInfoBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub s3_resource_path: Option<String>,
    }
    impl From<&S3ResourceInfoBody> for S3ResourceInfoBody {
        fn from(value: &S3ResourceInfoBody) -> Self {
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
    pub struct SearchJobsIndexResponse {
        ///the jobs that matched the query
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub hits: Vec<JobSearchHit>,
        ///a list of the terms that couldn't be parsed (and thus ignored)
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub query_parse_errors: Vec<SearchJobsIndexResponseQueryParseErrorsItem>,
    }
    impl From<&SearchJobsIndexResponse> for SearchJobsIndexResponse {
        fn from(value: &SearchJobsIndexResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SearchJobsIndexResponseQueryParseErrorsItem {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub dancer: Option<String>,
    }
    impl From<&SearchJobsIndexResponseQueryParseErrorsItem>
    for SearchJobsIndexResponseQueryParseErrorsItem {
        fn from(value: &SearchJobsIndexResponseQueryParseErrorsItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SearchLogsIndexResponse {
        ///log files that matched the query
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub hits: Vec<LogSearchHit>,
        ///a list of the terms that couldn't be parsed (and thus ignored)
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub query_parse_errors: Vec<String>,
    }
    impl From<&SearchLogsIndexResponse> for SearchLogsIndexResponse {
        fn from(value: &SearchLogsIndexResponse) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SendMessageToConversationBody {
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub card_block: std::collections::HashMap<String, serde_json::Value>,
        ///The ID of the Teams conversation/activity
        pub conversation_id: String,
        ///Used for styling the card conditionally
        #[serde(default = "defaults::default_bool::<true>")]
        pub success: bool,
        ///The message text to be sent in the Teams card
        pub text: String,
    }
    impl From<&SendMessageToConversationBody> for SendMessageToConversationBody {
        fn from(value: &SendMessageToConversationBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetAutomaticBillingBody {
        pub automatic_billing: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub seats: Option<f64>,
    }
    impl From<&SetAutomaticBillingBody> for SetAutomaticBillingBody {
        fn from(value: &SetAutomaticBillingBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetCaptureConfigBody {
        pub is_flow: bool,
        pub path: String,
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub trigger_config: std::collections::HashMap<String, serde_json::Value>,
        pub trigger_kind: CaptureTriggerKind,
    }
    impl From<&SetCaptureConfigBody> for SetCaptureConfigBody {
        fn from(value: &SetCaptureConfigBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetDefaultErrorOrRecoveryHandlerBody {
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub extra_args: std::collections::HashMap<String, serde_json::Value>,
        pub handler_type: SetDefaultErrorOrRecoveryHandlerBodyHandlerType,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub number_of_occurence: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub number_of_occurence_exact: Option<bool>,
        pub override_existing: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub workspace_handler_muted: Option<bool>,
    }
    impl From<&SetDefaultErrorOrRecoveryHandlerBody>
    for SetDefaultErrorOrRecoveryHandlerBody {
        fn from(value: &SetDefaultErrorOrRecoveryHandlerBody) -> Self {
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
    pub enum SetDefaultErrorOrRecoveryHandlerBodyHandlerType {
        #[serde(rename = "error")]
        Error,
        #[serde(rename = "recovery")]
        Recovery,
        #[serde(rename = "success")]
        Success,
    }
    impl From<&SetDefaultErrorOrRecoveryHandlerBodyHandlerType>
    for SetDefaultErrorOrRecoveryHandlerBodyHandlerType {
        fn from(value: &SetDefaultErrorOrRecoveryHandlerBodyHandlerType) -> Self {
            value.clone()
        }
    }
    impl ToString for SetDefaultErrorOrRecoveryHandlerBodyHandlerType {
        fn to_string(&self) -> String {
            match *self {
                Self::Error => "error".to_string(),
                Self::Recovery => "recovery".to_string(),
                Self::Success => "success".to_string(),
            }
        }
    }
    impl std::str::FromStr for SetDefaultErrorOrRecoveryHandlerBodyHandlerType {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "error" => Ok(Self::Error),
                "recovery" => Ok(Self::Recovery),
                "success" => Ok(Self::Success),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str>
    for SetDefaultErrorOrRecoveryHandlerBodyHandlerType {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String>
    for SetDefaultErrorOrRecoveryHandlerBodyHandlerType {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String>
    for SetDefaultErrorOrRecoveryHandlerBodyHandlerType {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetEnvironmentVariableBody {
        pub name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<String>,
    }
    impl From<&SetEnvironmentVariableBody> for SetEnvironmentVariableBody {
        fn from(value: &SetEnvironmentVariableBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetGlobalBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<serde_json::Value>,
    }
    impl From<&SetGlobalBody> for SetGlobalBody {
        fn from(value: &SetGlobalBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetJobProgressBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub flow_job_id: Option<uuid::Uuid>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub percent: Option<i64>,
    }
    impl From<&SetJobProgressBody> for SetJobProgressBody {
        fn from(value: &SetJobProgressBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetKafkaTriggerEnabledBody {
        pub enabled: bool,
    }
    impl From<&SetKafkaTriggerEnabledBody> for SetKafkaTriggerEnabledBody {
        fn from(value: &SetKafkaTriggerEnabledBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetLoginTypeForUserBody {
        pub login_type: String,
    }
    impl From<&SetLoginTypeForUserBody> for SetLoginTypeForUserBody {
        fn from(value: &SetLoginTypeForUserBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetMqttTriggerEnabledBody {
        pub enabled: bool,
    }
    impl From<&SetMqttTriggerEnabledBody> for SetMqttTriggerEnabledBody {
        fn from(value: &SetMqttTriggerEnabledBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetNatsTriggerEnabledBody {
        pub enabled: bool,
    }
    impl From<&SetNatsTriggerEnabledBody> for SetNatsTriggerEnabledBody {
        fn from(value: &SetNatsTriggerEnabledBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetPasswordBody {
        pub password: String,
    }
    impl From<&SetPasswordBody> for SetPasswordBody {
        fn from(value: &SetPasswordBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetPasswordForUserBody {
        pub password: String,
    }
    impl From<&SetPasswordForUserBody> for SetPasswordForUserBody {
        fn from(value: &SetPasswordForUserBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetPostgresTriggerEnabledBody {
        pub enabled: bool,
    }
    impl From<&SetPostgresTriggerEnabledBody> for SetPostgresTriggerEnabledBody {
        fn from(value: &SetPostgresTriggerEnabledBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetScheduleEnabledBody {
        pub enabled: bool,
    }
    impl From<&SetScheduleEnabledBody> for SetScheduleEnabledBody {
        fn from(value: &SetScheduleEnabledBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetSqsTriggerEnabledBody {
        pub enabled: bool,
    }
    impl From<&SetSqsTriggerEnabledBody> for SetSqsTriggerEnabledBody {
        fn from(value: &SetSqsTriggerEnabledBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetThresholdAlertBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub threshold_alert_amount: Option<f64>,
    }
    impl From<&SetThresholdAlertBody> for SetThresholdAlertBody {
        fn from(value: &SetThresholdAlertBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetWebsocketTriggerEnabledBody {
        pub enabled: bool,
    }
    impl From<&SetWebsocketTriggerEnabledBody> for SetWebsocketTriggerEnabledBody {
        fn from(value: &SetWebsocketTriggerEnabledBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SetWorkspaceEncryptionKeyBody {
        pub new_key: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub skip_reencrypt: Option<bool>,
    }
    impl From<&SetWorkspaceEncryptionKeyBody> for SetWorkspaceEncryptionKeyBody {
        fn from(value: &SetWorkspaceEncryptionKeyBody) -> Self {
            value.clone()
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
    pub struct StarBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub favorite_kind: Option<StarBodyFavoriteKind>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub path: Option<String>,
    }
    impl From<&StarBody> for StarBody {
        fn from(value: &StarBody) -> Self {
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
    pub enum StarBodyFavoriteKind {
        #[serde(rename = "flow")]
        Flow,
        #[serde(rename = "app")]
        App,
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "raw_app")]
        RawApp,
    }
    impl From<&StarBodyFavoriteKind> for StarBodyFavoriteKind {
        fn from(value: &StarBodyFavoriteKind) -> Self {
            value.clone()
        }
    }
    impl ToString for StarBodyFavoriteKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Flow => "flow".to_string(),
                Self::App => "app".to_string(),
                Self::Script => "script".to_string(),
                Self::RawApp => "raw_app".to_string(),
            }
        }
    }
    impl std::str::FromStr for StarBodyFavoriteKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "flow" => Ok(Self::Flow),
                "app" => Ok(Self::App),
                "script" => Ok(Self::Script),
                "raw_app" => Ok(Self::RawApp),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for StarBodyFavoriteKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for StarBodyFavoriteKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for StarBodyFavoriteKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
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
    pub struct TestCriticalChannelsBodyItem {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub email: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub slack_channel: Option<String>,
    }
    impl From<&TestCriticalChannelsBodyItem> for TestCriticalChannelsBodyItem {
        fn from(value: &TestCriticalChannelsBodyItem) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TestKafkaConnectionBody {
        pub connection: std::collections::HashMap<String, serde_json::Value>,
    }
    impl From<&TestKafkaConnectionBody> for TestKafkaConnectionBody {
        fn from(value: &TestKafkaConnectionBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TestLicenseKeyBody {
        pub license_key: String,
    }
    impl From<&TestLicenseKeyBody> for TestLicenseKeyBody {
        fn from(value: &TestLicenseKeyBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TestMqttConnectionBody {
        pub connection: std::collections::HashMap<String, serde_json::Value>,
    }
    impl From<&TestMqttConnectionBody> for TestMqttConnectionBody {
        fn from(value: &TestMqttConnectionBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TestNatsConnectionBody {
        pub connection: std::collections::HashMap<String, serde_json::Value>,
    }
    impl From<&TestNatsConnectionBody> for TestNatsConnectionBody {
        fn from(value: &TestNatsConnectionBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TestPostgresConnectionBody {
        pub database: String,
    }
    impl From<&TestPostgresConnectionBody> for TestPostgresConnectionBody {
        fn from(value: &TestPostgresConnectionBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TestSmtpBody {
        pub smtp: TestSmtpBodySmtp,
        pub to: String,
    }
    impl From<&TestSmtpBody> for TestSmtpBody {
        fn from(value: &TestSmtpBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TestSmtpBodySmtp {
        pub disable_tls: bool,
        pub from: String,
        pub host: String,
        pub password: String,
        pub port: i64,
        pub tls_implicit: bool,
        pub username: String,
    }
    impl From<&TestSmtpBodySmtp> for TestSmtpBodySmtp {
        fn from(value: &TestSmtpBodySmtp) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TestSqsConnectionBody {
        pub connection: std::collections::HashMap<String, serde_json::Value>,
    }
    impl From<&TestSqsConnectionBody> for TestSqsConnectionBody {
        fn from(value: &TestSqsConnectionBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct TestWebsocketConnectionBody {
        pub can_return_message: bool,
        pub url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub url_runnable_args: Option<ScriptArgs>,
    }
    impl From<&TestWebsocketConnectionBody> for TestWebsocketConnectionBody {
        fn from(value: &TestWebsocketConnectionBody) -> Self {
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
    pub struct ToggleWorkspaceErrorHandlerForFlowBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub muted: Option<bool>,
    }
    impl From<&ToggleWorkspaceErrorHandlerForFlowBody>
    for ToggleWorkspaceErrorHandlerForFlowBody {
        fn from(value: &ToggleWorkspaceErrorHandlerForFlowBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ToggleWorkspaceErrorHandlerForScriptBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub muted: Option<bool>,
    }
    impl From<&ToggleWorkspaceErrorHandlerForScriptBody>
    for ToggleWorkspaceErrorHandlerForScriptBody {
        fn from(value: &ToggleWorkspaceErrorHandlerForScriptBody) -> Self {
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
    pub struct UnstarBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub favorite_kind: Option<UnstarBodyFavoriteKind>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub path: Option<String>,
    }
    impl From<&UnstarBody> for UnstarBody {
        fn from(value: &UnstarBody) -> Self {
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
    pub enum UnstarBodyFavoriteKind {
        #[serde(rename = "flow")]
        Flow,
        #[serde(rename = "app")]
        App,
        #[serde(rename = "script")]
        Script,
        #[serde(rename = "raw_app")]
        RawApp,
    }
    impl From<&UnstarBodyFavoriteKind> for UnstarBodyFavoriteKind {
        fn from(value: &UnstarBodyFavoriteKind) -> Self {
            value.clone()
        }
    }
    impl ToString for UnstarBodyFavoriteKind {
        fn to_string(&self) -> String {
            match *self {
                Self::Flow => "flow".to_string(),
                Self::App => "app".to_string(),
                Self::Script => "script".to_string(),
                Self::RawApp => "raw_app".to_string(),
            }
        }
    }
    impl std::str::FromStr for UnstarBodyFavoriteKind {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, &'static str> {
            match value {
                "flow" => Ok(Self::Flow),
                "app" => Ok(Self::App),
                "script" => Ok(Self::Script),
                "raw_app" => Ok(Self::RawApp),
                _ => Err("invalid value"),
            }
        }
    }
    impl std::convert::TryFrom<&str> for UnstarBodyFavoriteKind {
        type Error = &'static str;
        fn try_from(value: &str) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<&String> for UnstarBodyFavoriteKind {
        type Error = &'static str;
        fn try_from(value: &String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    impl std::convert::TryFrom<String> for UnstarBodyFavoriteKind {
        type Error = &'static str;
        fn try_from(value: String) -> Result<Self, &'static str> {
            value.parse()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct UpdateAppBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub custom_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deployment_message: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub policy: Option<Policy>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<serde_json::Value>,
    }
    impl From<&UpdateAppBody> for UpdateAppBody {
        fn from(value: &UpdateAppBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct UpdateAppHistoryBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deployment_msg: Option<String>,
    }
    impl From<&UpdateAppHistoryBody> for UpdateAppHistoryBody {
        fn from(value: &UpdateAppHistoryBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct UpdateFlowBody {
        #[serde(flatten)]
        pub open_flow_w_path: OpenFlowWPath,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deployment_message: Option<String>,
    }
    impl From<&UpdateFlowBody> for UpdateFlowBody {
        fn from(value: &UpdateFlowBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct UpdateFlowHistoryBody {
        pub deployment_msg: String,
    }
    impl From<&UpdateFlowHistoryBody> for UpdateFlowHistoryBody {
        fn from(value: &UpdateFlowHistoryBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct UpdateFolderBody {
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        pub extra_perms: std::collections::HashMap<String, bool>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub owners: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
    }
    impl From<&UpdateFolderBody> for UpdateFolderBody {
        fn from(value: &UpdateFolderBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct UpdateGroupBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
    }
    impl From<&UpdateGroupBody> for UpdateGroupBody {
        fn from(value: &UpdateGroupBody) -> Self {
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
    pub struct UpdateInstanceGroupBody {
        pub new_summary: String,
    }
    impl From<&UpdateInstanceGroupBody> for UpdateInstanceGroupBody {
        fn from(value: &UpdateInstanceGroupBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct UpdateRawAppBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<String>,
    }
    impl From<&UpdateRawAppBody> for UpdateRawAppBody {
        fn from(value: &UpdateRawAppBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct UpdateResourceValueBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<serde_json::Value>,
    }
    impl From<&UpdateResourceValueBody> for UpdateResourceValueBody {
        fn from(value: &UpdateResourceValueBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct UpdateScriptHistoryBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deployment_msg: Option<String>,
    }
    impl From<&UpdateScriptHistoryBody> for UpdateScriptHistoryBody {
        fn from(value: &UpdateScriptHistoryBody) -> Self {
            value.clone()
        }
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct UpdateTutorialProgressBody {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub progress: Option<i64>,
    }
    impl From<&UpdateTutorialProgressBody> for UpdateTutorialProgressBody {
        fn from(value: &UpdateTutorialProgressBody) -> Self {
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
    pub struct UploadS3FileFromAppResponse {
        pub delete_token: String,
        pub file_key: String,
    }
    impl From<&UploadS3FileFromAppResponse> for UploadS3FileFromAppResponse {
        fn from(value: &UploadS3FileFromAppResponse) -> Self {
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
    pub struct WorkspaceGetCriticalAlertsResponse {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub alerts: Vec<CriticalAlert>,
        ///Total number of pages based on the page size.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub total_pages: Option<i64>,
        ///Total number of rows matching the query.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub total_rows: Option<i64>,
    }
    impl From<&WorkspaceGetCriticalAlertsResponse>
    for WorkspaceGetCriticalAlertsResponse {
        fn from(value: &WorkspaceGetCriticalAlertsResponse) -> Self {
            value.clone()
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
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WorkspaceMuteCriticalAlertsUiBody {
        ///Whether critical alerts should be muted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub mute_critical_alerts: Option<bool>,
    }
    impl From<&WorkspaceMuteCriticalAlertsUiBody> for WorkspaceMuteCriticalAlertsUiBody {
        fn from(value: &WorkspaceMuteCriticalAlertsUiBody) -> Self {
            value.clone()
        }
    }
    pub mod defaults {
        pub(super) fn default_bool<const V: bool>() -> bool {
            V
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
    /**get backend version

Sends a `GET` request to `/version`

*/
    pub async fn backend_version<'a>(
        &'a self,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/version", self.baseurl,);
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**is backend up to date

Sends a `GET` request to `/uptodate`

*/
    pub async fn backend_uptodate<'a>(
        &'a self,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/uptodate", self.baseurl,);
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get license id

Sends a `GET` request to `/ee_license`

*/
    pub async fn get_license_id<'a>(
        &'a self,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/ee_license", self.baseurl,);
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get openapi yaml spec

Sends a `GET` request to `/openapi.yaml`

*/
    pub async fn get_open_api_yaml<'a>(
        &'a self,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/openapi.yaml", self.baseurl,);
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get audit log (requires admin privilege)

Sends a `GET` request to `/w/{workspace}/audit/get/{id}`

*/
    pub async fn get_audit_log<'a>(
        &'a self,
        workspace: &'a str,
        id: i64,
    ) -> Result<ResponseValue<types::AuditLog>, Error<()>> {
        let url = format!(
            "{}/w/{}/audit/get/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& id.to_string()),
        );
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
    /**list audit logs (requires admin privilege)

Sends a `GET` request to `/w/{workspace}/audit/list`

Arguments:
- `workspace`
- `action_kind`: filter on type of operation
- `after`: filter on created after (exclusive) timestamp
- `all_workspaces`: get audit logs for all workspaces
- `before`: filter on started before (inclusive) timestamp
- `exclude_operations`: comma separated list of operations to exclude
- `operation`: filter on exact or prefix name of operation
- `operations`: comma separated list of exact operations to include
- `page`: which page to return (start at 1, default 1)
- `per_page`: number of items to return for a given page (default 30, max 100)
- `resource`: filter on exact or prefix name of resource
- `username`: filter on exact username of user
*/
    pub async fn list_audit_logs<'a>(
        &'a self,
        workspace: &'a str,
        action_kind: Option<types::ListAuditLogsActionKind>,
        after: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        all_workspaces: Option<bool>,
        before: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        exclude_operations: Option<&'a str>,
        operation: Option<&'a str>,
        operations: Option<&'a str>,
        page: Option<i64>,
        per_page: Option<i64>,
        resource: Option<&'a str>,
        username: Option<&'a str>,
    ) -> Result<ResponseValue<Vec<types::AuditLog>>, Error<()>> {
        let url = format!(
            "{}/w/{}/audit/list", self.baseurl, encode_path(& workspace.to_string()),
        );
        let mut query = Vec::with_capacity(11usize);
        if let Some(v) = &action_kind {
            query.push(("action_kind", v.to_string()));
        }
        if let Some(v) = &after {
            query.push(("after", v.to_string()));
        }
        if let Some(v) = &all_workspaces {
            query.push(("all_workspaces", v.to_string()));
        }
        if let Some(v) = &before {
            query.push(("before", v.to_string()));
        }
        if let Some(v) = &exclude_operations {
            query.push(("exclude_operations", v.to_string()));
        }
        if let Some(v) = &operation {
            query.push(("operation", v.to_string()));
        }
        if let Some(v) = &operations {
            query.push(("operations", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
        }
        if let Some(v) = &resource {
            query.push(("resource", v.to_string()));
        }
        if let Some(v) = &username {
            query.push(("username", v.to_string()));
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
    /**login with password

Sends a `POST` request to `/auth/login`

Arguments:
- `body`: credentials
*/
    pub async fn login<'a>(
        &'a self,
        body: &'a types::Login,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/auth/login", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**logout

Sends a `POST` request to `/auth/logout`

*/
    pub async fn logout<'a>(&'a self) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/auth/logout", self.baseurl,);
        let request = self.client.post(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get user (require admin privilege)

Sends a `GET` request to `/w/{workspace}/users/get/{username}`

*/
    pub async fn get_user<'a>(
        &'a self,
        workspace: &'a str,
        username: &'a str,
    ) -> Result<ResponseValue<types::User>, Error<()>> {
        let url = format!(
            "{}/w/{}/users/get/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& username.to_string()),
        );
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
    /**update user (require admin privilege)

Sends a `POST` request to `/w/{workspace}/users/update/{username}`

Arguments:
- `workspace`
- `username`
- `body`: new user
*/
    pub async fn update_user<'a>(
        &'a self,
        workspace: &'a str,
        username: &'a str,
        body: &'a types::EditWorkspaceUser,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/users/update/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& username.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**is owner of path

Sends a `GET` request to `/w/{workspace}/users/is_owner/{path}`

*/
    pub async fn is_owner_of_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/users/is_owner/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**set password

Sends a `POST` request to `/users/setpassword`

Arguments:
- `body`: set password
*/
    pub async fn set_password<'a>(
        &'a self,
        body: &'a types::SetPasswordBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/users/setpassword", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**set password for a specific user (require super admin)

Sends a `POST` request to `/users/set_password_of/{user}`

Arguments:
- `user`
- `body`: set password
*/
    pub async fn set_password_for_user<'a>(
        &'a self,
        user: &'a str,
        body: &'a types::SetPasswordForUserBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/users/set_password_of/{}", self.baseurl, encode_path(& user.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**set login type for a specific user (require super admin)

Sends a `POST` request to `/users/set_login_type/{user}`

Arguments:
- `user`
- `body`: set login type
*/
    pub async fn set_login_type_for_user<'a>(
        &'a self,
        user: &'a str,
        body: &'a types::SetLoginTypeForUserBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/users/set_login_type/{}", self.baseurl, encode_path(& user.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create user

Sends a `POST` request to `/users/create`

Arguments:
- `body`: user info
*/
    pub async fn create_user_globally<'a>(
        &'a self,
        body: &'a types::CreateUserGloballyBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/users/create", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**global update user (require super admin)

Sends a `POST` request to `/users/update/{email}`

Arguments:
- `email`
- `body`: new user info
*/
    pub async fn global_user_update<'a>(
        &'a self,
        email: &'a str,
        body: &'a types::GlobalUserUpdateBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/users/update/{}", self.baseurl, encode_path(& email.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**global username info (require super admin)

Sends a `GET` request to `/users/username_info/{email}`

*/
    pub async fn global_username_info<'a>(
        &'a self,
        email: &'a str,
    ) -> Result<ResponseValue<types::GlobalUsernameInfoResponse>, Error<()>> {
        let url = format!(
            "{}/users/username_info/{}", self.baseurl, encode_path(& email.to_string()),
        );
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
    /**global rename user (require super admin)

Sends a `POST` request to `/users/rename/{email}`

Arguments:
- `email`
- `body`: new username
*/
    pub async fn global_user_rename<'a>(
        &'a self,
        email: &'a str,
        body: &'a types::GlobalUserRenameBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/users/rename/{}", self.baseurl, encode_path(& email.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**global delete user (require super admin)

Sends a `DELETE` request to `/users/delete/{email}`

*/
    pub async fn global_user_delete<'a>(
        &'a self,
        email: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/users/delete/{}", self.baseurl, encode_path(& email.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**global overwrite users (require super admin and EE)

Sends a `POST` request to `/users/overwrite`

Arguments:
- `body`: List of users
*/
    pub async fn global_users_overwrite<'a>(
        &'a self,
        body: &'a Vec<types::ExportedUser>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/users/overwrite", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**global export users (require super admin and EE)

Sends a `GET` request to `/users/export`

*/
    pub async fn global_users_export<'a>(
        &'a self,
    ) -> Result<ResponseValue<Vec<types::ExportedUser>>, Error<()>> {
        let url = format!("{}/users/export", self.baseurl,);
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
    /**delete user (require admin privilege)

Sends a `DELETE` request to `/w/{workspace}/users/delete/{username}`

*/
    pub async fn delete_user<'a>(
        &'a self,
        workspace: &'a str,
        username: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/users/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& username.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
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
    /**is domain allowed for auto invi

Sends a `GET` request to `/workspaces/allowed_domain_auto_invite`

*/
    pub async fn is_domain_allowed<'a>(
        &'a self,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!("{}/workspaces/allowed_domain_auto_invite", self.baseurl,);
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
    /**list all workspaces visible to me with user info

Sends a `GET` request to `/workspaces/users`

*/
    pub async fn list_user_workspaces<'a>(
        &'a self,
    ) -> Result<ResponseValue<types::UserWorkspaceList>, Error<()>> {
        let url = format!("{}/workspaces/users", self.baseurl,);
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
    /**list all workspaces as super admin (require to be super admin)

Sends a `GET` request to `/workspaces/list_as_superadmin`

Arguments:
- `page`: which page to return (start at 1, default 1)
- `per_page`: number of items to return for a given page (default 30, max 100)
*/
    pub async fn list_workspaces_as_super_admin<'a>(
        &'a self,
        page: Option<i64>,
        per_page: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::Workspace>>, Error<()>> {
        let url = format!("{}/workspaces/list_as_superadmin", self.baseurl,);
        let mut query = Vec::with_capacity(2usize);
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
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
    /**create workspace

Sends a `POST` request to `/workspaces/create`

Arguments:
- `body`: new token
*/
    pub async fn create_workspace<'a>(
        &'a self,
        body: &'a types::CreateWorkspace,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/workspaces/create", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**exists workspace

Sends a `POST` request to `/workspaces/exists`

Arguments:
- `body`: id of workspace
*/
    pub async fn exists_workspace<'a>(
        &'a self,
        body: &'a types::ExistsWorkspaceBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/workspaces/exists", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**exists username

Sends a `POST` request to `/workspaces/exists_username`

*/
    pub async fn exists_username<'a>(
        &'a self,
        body: &'a types::ExistsUsernameBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/workspaces/exists_username", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get global settings

Sends a `GET` request to `/settings/global/{key}`

*/
    pub async fn get_global<'a>(
        &'a self,
        key: &'a str,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/settings/global/{}", self.baseurl, encode_path(& key.to_string()),
        );
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
    /**post global settings

Sends a `POST` request to `/settings/global/{key}`

Arguments:
- `key`
- `body`: value set
*/
    pub async fn set_global<'a>(
        &'a self,
        key: &'a str,
        body: &'a types::SetGlobalBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/settings/global/{}", self.baseurl, encode_path(& key.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get local settings

Sends a `GET` request to `/settings/local`

*/
    pub async fn get_local<'a>(
        &'a self,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!("{}/settings/local", self.baseurl,);
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
    /**test smtp

Sends a `POST` request to `/settings/test_smtp`

Arguments:
- `body`: test smtp payload
*/
    pub async fn test_smtp<'a>(
        &'a self,
        body: &'a types::TestSmtpBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/settings/test_smtp", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**test critical channels

Sends a `POST` request to `/settings/test_critical_channels`

Arguments:
- `body`: test critical channel payload
*/
    pub async fn test_critical_channels<'a>(
        &'a self,
        body: &'a Vec<types::TestCriticalChannelsBodyItem>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/settings/test_critical_channels", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**Get all critical alerts

Sends a `GET` request to `/settings/critical_alerts`

*/
    pub async fn get_critical_alerts<'a>(
        &'a self,
        acknowledged: Option<bool>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<ResponseValue<types::GetCriticalAlertsResponse>, Error<()>> {
        let url = format!("{}/settings/critical_alerts", self.baseurl,);
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &acknowledged {
            query.push(("acknowledged", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &page_size {
            query.push(("page_size", v.to_string()));
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
    /**Acknowledge a critical alert

Sends a `POST` request to `/settings/critical_alerts/{id}/acknowledge`

Arguments:
- `id`: The ID of the critical alert to acknowledge
*/
    pub async fn acknowledge_critical_alert<'a>(
        &'a self,
        id: i64,
    ) -> Result<ResponseValue<String>, Error<()>> {
        let url = format!(
            "{}/settings/critical_alerts/{}/acknowledge", self.baseurl, encode_path(& id
            .to_string()),
        );
        let request = self
            .client
            .post(url)
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
    /**Acknowledge all unacknowledged critical alerts

Sends a `POST` request to `/settings/critical_alerts/acknowledge_all`

*/
    pub async fn acknowledge_all_critical_alerts<'a>(
        &'a self,
    ) -> Result<ResponseValue<String>, Error<()>> {
        let url = format!("{}/settings/critical_alerts/acknowledge_all", self.baseurl,);
        let request = self
            .client
            .post(url)
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
    /**test license key

Sends a `POST` request to `/settings/test_license_key`

Arguments:
- `body`: test license key
*/
    pub async fn test_license_key<'a>(
        &'a self,
        body: &'a types::TestLicenseKeyBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/settings/test_license_key", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**test object storage config

Sends a `POST` request to `/settings/test_object_storage_config`

Arguments:
- `body`: test object storage config
*/
    pub async fn test_object_storage_config<'a>(
        &'a self,
        body: &'a std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/settings/test_object_storage_config", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**send stats

Sends a `POST` request to `/settings/send_stats`

*/
    pub async fn send_stats<'a>(
        &'a self,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/settings/send_stats", self.baseurl,);
        let request = self.client.post(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get latest key renewal attempt

Sends a `GET` request to `/settings/latest_key_renewal_attempt`

*/
    pub async fn get_latest_key_renewal_attempt<'a>(
        &'a self,
    ) -> Result<
        ResponseValue<Option<types::GetLatestKeyRenewalAttemptResponse>>,
        Error<()>,
    > {
        let url = format!("{}/settings/latest_key_renewal_attempt", self.baseurl,);
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
    /**renew license key

Sends a `POST` request to `/settings/renew_license_key`

*/
    pub async fn renew_license_key<'a>(
        &'a self,
        license_key: Option<&'a str>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/settings/renew_license_key", self.baseurl,);
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &license_key {
            query.push(("license_key", v.to_string()));
        }
        let request = self.client.post(url).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create customer portal session

Sends a `POST` request to `/settings/customer_portal`

*/
    pub async fn create_customer_portal_session<'a>(
        &'a self,
        license_key: Option<&'a str>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/settings/customer_portal", self.baseurl,);
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &license_key {
            query.push(("license_key", v.to_string()));
        }
        let request = self.client.post(url).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**test metadata

Sends a `POST` request to `/saml/test_metadata`

Arguments:
- `body`: test metadata
*/
    pub async fn test_metadata<'a>(
        &'a self,
        body: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/saml/test_metadata", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list global settings

Sends a `GET` request to `/settings/list_global`

*/
    pub async fn list_global_settings<'a>(
        &'a self,
    ) -> Result<ResponseValue<Vec<types::GlobalSetting>>, Error<()>> {
        let url = format!("{}/settings/list_global", self.baseurl,);
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
    /**get current user email (if logged in)

Sends a `GET` request to `/users/email`

*/
    pub async fn get_current_email<'a>(
        &'a self,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/users/email", self.baseurl,);
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**refresh the current token

Sends a `GET` request to `/users/refresh_token`

*/
    pub async fn refresh_user_token<'a>(
        &'a self,
        if_expiring_in_less_than_s: Option<i64>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/users/refresh_token", self.baseurl,);
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &if_expiring_in_less_than_s {
            query.push(("if_expiring_in_less_than_s", v.to_string()));
        }
        let request = self.client.get(url).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get tutorial progress

Sends a `GET` request to `/users/tutorial_progress`

*/
    pub async fn get_tutorial_progress<'a>(
        &'a self,
    ) -> Result<ResponseValue<types::GetTutorialProgressResponse>, Error<()>> {
        let url = format!("{}/users/tutorial_progress", self.baseurl,);
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
    /**update tutorial progress

Sends a `POST` request to `/users/tutorial_progress`

Arguments:
- `body`: progress update
*/
    pub async fn update_tutorial_progress<'a>(
        &'a self,
        body: &'a types::UpdateTutorialProgressBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/users/tutorial_progress", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**leave instance

Sends a `POST` request to `/users/leave_instance`

*/
    pub async fn leave_instance<'a>(
        &'a self,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/users/leave_instance", self.baseurl,);
        let request = self.client.post(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get current usage outside of premium workspaces

Sends a `GET` request to `/users/usage`

*/
    pub async fn get_usage<'a>(
        &'a self,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/users/usage", self.baseurl,);
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get all runnables in every workspace

Sends a `GET` request to `/users/all_runnables`

*/
    pub async fn get_runnable<'a>(
        &'a self,
    ) -> Result<ResponseValue<types::GetRunnableResponse>, Error<()>> {
        let url = format!("{}/users/all_runnables", self.baseurl,);
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
    /**get current global whoami (if logged in)

Sends a `GET` request to `/users/whoami`

*/
    pub async fn global_whoami<'a>(
        &'a self,
    ) -> Result<ResponseValue<types::GlobalUserInfo>, Error<()>> {
        let url = format!("{}/users/whoami", self.baseurl,);
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
    /**list all workspace invites

Sends a `GET` request to `/users/list_invites`

*/
    pub async fn list_workspace_invites<'a>(
        &'a self,
    ) -> Result<ResponseValue<Vec<types::WorkspaceInvite>>, Error<()>> {
        let url = format!("{}/users/list_invites", self.baseurl,);
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
    /**whoami

Sends a `GET` request to `/w/{workspace}/users/whoami`

*/
    pub async fn whoami<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<types::User>, Error<()>> {
        let url = format!(
            "{}/w/{}/users/whoami", self.baseurl, encode_path(& workspace.to_string()),
        );
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
    /**accept invite to workspace

Sends a `POST` request to `/users/accept_invite`

Arguments:
- `body`: accept invite
*/
    pub async fn accept_invite<'a>(
        &'a self,
        body: &'a types::AcceptInviteBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/users/accept_invite", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**decline invite to workspace

Sends a `POST` request to `/users/decline_invite`

Arguments:
- `body`: decline invite
*/
    pub async fn decline_invite<'a>(
        &'a self,
        body: &'a types::DeclineInviteBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/users/decline_invite", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**invite user to workspace

Sends a `POST` request to `/w/{workspace}/workspaces/invite_user`

Arguments:
- `workspace`
- `body`: WorkspaceInvite
*/
    pub async fn invite_user<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::InviteUserBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/invite_user", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**add user to workspace

Sends a `POST` request to `/w/{workspace}/workspaces/add_user`

Arguments:
- `workspace`
- `body`: WorkspaceInvite
*/
    pub async fn add_user<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::AddUserBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/add_user", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**delete user invite

Sends a `POST` request to `/w/{workspace}/workspaces/delete_invite`

Arguments:
- `workspace`
- `body`: WorkspaceInvite
*/
    pub async fn delete_invite<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::DeleteInviteBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/delete_invite", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**archive workspace

Sends a `POST` request to `/w/{workspace}/workspaces/archive`

*/
    pub async fn archive_workspace<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/archive", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**unarchive workspace

Sends a `POST` request to `/workspaces/unarchive/{workspace}`

*/
    pub async fn unarchive_workspace<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/workspaces/unarchive/{}", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**delete workspace (require super admin)

Sends a `DELETE` request to `/workspaces/delete/{workspace}`

*/
    pub async fn delete_workspace<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/workspaces/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**leave workspace

Sends a `POST` request to `/w/{workspace}/workspaces/leave`

*/
    pub async fn leave_workspace<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/leave", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get workspace name

Sends a `GET` request to `/w/{workspace}/workspaces/get_workspace_name`

*/
    pub async fn get_workspace_name<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/get_workspace_name", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**change workspace name

Sends a `POST` request to `/w/{workspace}/workspaces/change_workspace_name`

*/
    pub async fn change_workspace_name<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::ChangeWorkspaceNameBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/change_workspace_name", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**change workspace id

Sends a `POST` request to `/w/{workspace}/workspaces/change_workspace_id`

*/
    pub async fn change_workspace_id<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::ChangeWorkspaceIdBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/change_workspace_id", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**change workspace id

Sends a `POST` request to `/w/{workspace}/workspaces/change_workspace_color`

*/
    pub async fn change_workspace_color<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::ChangeWorkspaceColorBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/change_workspace_color", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**whois

Sends a `GET` request to `/w/{workspace}/users/whois/{username}`

*/
    pub async fn whois<'a>(
        &'a self,
        workspace: &'a str,
        username: &'a str,
    ) -> Result<ResponseValue<types::User>, Error<()>> {
        let url = format!(
            "{}/w/{}/users/whois/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& username.to_string()),
        );
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
    /**Update operator settings for a workspace

Updates the operator settings for a specific workspace. Requires workspace admin privileges.

Sends a `POST` request to `/w/{workspace}/workspaces/operator_settings`

*/
    pub async fn update_operator_settings<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::OperatorSettings,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/operator_settings", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**exists email

Sends a `GET` request to `/users/exists/{email}`

*/
    pub async fn exists_email<'a>(
        &'a self,
        email: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/users/exists/{}", self.baseurl, encode_path(& email.to_string()),
        );
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
    /**list all users as super admin (require to be super amdin)

Sends a `GET` request to `/users/list_as_super_admin`

Arguments:
- `active_only`: filter only active users
- `page`: which page to return (start at 1, default 1)
- `per_page`: number of items to return for a given page (default 30, max 100)
*/
    pub async fn list_users_as_super_admin<'a>(
        &'a self,
        active_only: Option<bool>,
        page: Option<i64>,
        per_page: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::GlobalUserInfo>>, Error<()>> {
        let url = format!("{}/users/list_as_super_admin", self.baseurl,);
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &active_only {
            query.push(("active_only", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
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
    /**list pending invites for a workspace

Sends a `GET` request to `/w/{workspace}/workspaces/list_pending_invites`

*/
    pub async fn list_pending_invites<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<Vec<types::WorkspaceInvite>>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/list_pending_invites", self.baseurl, encode_path(&
            workspace.to_string()),
        );
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
    /**get settings

Sends a `GET` request to `/w/{workspace}/workspaces/get_settings`

*/
    pub async fn get_settings<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<types::GetSettingsResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/get_settings", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**get deploy to

Sends a `GET` request to `/w/{workspace}/workspaces/get_deploy_to`

*/
    pub async fn get_deploy_to<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<types::GetDeployToResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/get_deploy_to", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**get if workspace is premium

Sends a `GET` request to `/w/{workspace}/workspaces/is_premium`

*/
    pub async fn get_is_premium<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/is_premium", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**get premium info

Sends a `GET` request to `/w/{workspace}/workspaces/premium_info`

*/
    pub async fn get_premium_info<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<types::GetPremiumInfoResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/premium_info", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**set automatic billing

Sends a `POST` request to `/w/{workspace}/workspaces/set_automatic_billing`

Arguments:
- `workspace`
- `body`: automatic billing
*/
    pub async fn set_automatic_billing<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::SetAutomaticBillingBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/set_automatic_billing", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get threshold alert info

Sends a `GET` request to `/w/{workspace}/workspaces/threshold_alert`

*/
    pub async fn get_threshold_alert<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<types::GetThresholdAlertResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/threshold_alert", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**set threshold alert info

Sends a `POST` request to `/w/{workspace}/workspaces/threshold_alert`

Arguments:
- `workspace`
- `body`: threshold alert info
*/
    pub async fn set_threshold_alert<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::SetThresholdAlertBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/threshold_alert", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**edit slack command

Sends a `POST` request to `/w/{workspace}/workspaces/edit_slack_command`

Arguments:
- `workspace`
- `body`: WorkspaceInvite
*/
    pub async fn edit_slack_command<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::EditSlackCommandBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/edit_slack_command", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**edit teams command

Sends a `POST` request to `/w/{workspace}/workspaces/edit_teams_command`

Arguments:
- `workspace`
- `body`: WorkspaceInvite
*/
    pub async fn edit_teams_command<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::EditTeamsCommandBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/edit_teams_command", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list available teams ids

Sends a `GET` request to `/w/{workspace}/workspaces/available_teams_ids`

*/
    pub async fn list_available_teams_ids<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<
        ResponseValue<Vec<types::ListAvailableTeamsIdsResponseItem>>,
        Error<()>,
    > {
        let url = format!(
            "{}/w/{}/workspaces/available_teams_ids", self.baseurl, encode_path(&
            workspace.to_string()),
        );
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
    /**list available teams channels

Sends a `GET` request to `/w/{workspace}/workspaces/available_teams_channels`

*/
    pub async fn list_available_teams_channels<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<
        ResponseValue<Vec<types::ListAvailableTeamsChannelsResponseItem>>,
        Error<()>,
    > {
        let url = format!(
            "{}/w/{}/workspaces/available_teams_channels", self.baseurl, encode_path(&
            workspace.to_string()),
        );
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
    /**connect teams

Sends a `POST` request to `/w/{workspace}/workspaces/connect_teams`

Arguments:
- `workspace`
- `body`: connect teams
*/
    pub async fn connect_teams<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::ConnectTeamsBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/connect_teams", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**run a job that sends a message to Slack

Sends a `POST` request to `/w/{workspace}/workspaces/run_slack_message_test_job`

Arguments:
- `workspace`
- `body`: path to hub script to run and its corresponding args
*/
    pub async fn run_slack_message_test_job<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::RunSlackMessageTestJobBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/run_slack_message_test_job", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**run a job that sends a message to Teams

Sends a `POST` request to `/w/{workspace}/workspaces/run_teams_message_test_job`

Arguments:
- `workspace`
- `body`: path to hub script to run and its corresponding args
*/
    pub async fn run_teams_message_test_job<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::RunTeamsMessageTestJobBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/run_teams_message_test_job", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**edit deploy to

Sends a `POST` request to `/w/{workspace}/workspaces/edit_deploy_to`

*/
    pub async fn edit_deploy_to<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::EditDeployToBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/edit_deploy_to", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**edit auto invite

Sends a `POST` request to `/w/{workspace}/workspaces/edit_auto_invite`

Arguments:
- `workspace`
- `body`: WorkspaceInvite
*/
    pub async fn edit_auto_invite<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::EditAutoInviteBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/edit_auto_invite", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**edit webhook

Sends a `POST` request to `/w/{workspace}/workspaces/edit_webhook`

Arguments:
- `workspace`
- `body`: WorkspaceWebhook
*/
    pub async fn edit_webhook<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::EditWebhookBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/edit_webhook", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**edit copilot config

Sends a `POST` request to `/w/{workspace}/workspaces/edit_copilot_config`

Arguments:
- `workspace`
- `body`: WorkspaceCopilotConfig
*/
    pub async fn edit_copilot_config<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::EditCopilotConfigBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/edit_copilot_config", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get copilot info

Sends a `GET` request to `/w/{workspace}/workspaces/get_copilot_info`

*/
    pub async fn get_copilot_info<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/get_copilot_info", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**edit error handler

Sends a `POST` request to `/w/{workspace}/workspaces/edit_error_handler`

Arguments:
- `workspace`
- `body`: WorkspaceErrorHandler
*/
    pub async fn edit_error_handler<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::EditErrorHandlerBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/edit_error_handler", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**edit large file storage settings

Sends a `POST` request to `/w/{workspace}/workspaces/edit_large_file_storage_config`

Arguments:
- `workspace`
- `body`: LargeFileStorage info
*/
    pub async fn edit_large_file_storage_config<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::EditLargeFileStorageConfigBody,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/edit_large_file_storage_config", self.baseurl,
            encode_path(& workspace.to_string()),
        );
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**edit workspace git sync settings

Sends a `POST` request to `/w/{workspace}/workspaces/edit_git_sync_config`

Arguments:
- `workspace`
- `body`: Workspace Git sync settings
*/
    pub async fn edit_workspace_git_sync_config<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::EditWorkspaceGitSyncConfigBody,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/edit_git_sync_config", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**edit workspace deploy ui settings

Sends a `POST` request to `/w/{workspace}/workspaces/edit_deploy_ui_config`

Arguments:
- `workspace`
- `body`: Workspace deploy UI settings
*/
    pub async fn edit_workspace_deploy_ui_settings<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::EditWorkspaceDeployUiSettingsBody,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/edit_deploy_ui_config", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**edit default app for workspace

Sends a `POST` request to `/w/{workspace}/workspaces/edit_default_app`

Arguments:
- `workspace`
- `body`: Workspace default app
*/
    pub async fn edit_workspace_default_app<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::EditWorkspaceDefaultAppBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/edit_default_app", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get default scripts for workspace

Sends a `GET` request to `/w/{workspace}/workspaces/default_scripts`

*/
    pub async fn get_default_scripts<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<types::WorkspaceDefaultScripts>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/default_scripts", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**edit default scripts for workspace

Sends a `POST` request to `/w/{workspace}/workspaces/default_scripts`

Arguments:
- `workspace`
- `body`: Workspace default app
*/
    pub async fn edit_default_scripts<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::WorkspaceDefaultScripts,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/default_scripts", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**set environment variable

Sends a `POST` request to `/w/{workspace}/workspaces/set_environment_variable`

Arguments:
- `workspace`
- `body`: Workspace default app
*/
    pub async fn set_environment_variable<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::SetEnvironmentVariableBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/set_environment_variable", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**retrieves the encryption key for this workspace

Sends a `GET` request to `/w/{workspace}/workspaces/encryption_key`

*/
    pub async fn get_workspace_encryption_key<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<types::GetWorkspaceEncryptionKeyResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/encryption_key", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**update the encryption key for this workspace

Sends a `POST` request to `/w/{workspace}/workspaces/encryption_key`

Arguments:
- `workspace`
- `body`: New encryption key
*/
    pub async fn set_workspace_encryption_key<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::SetWorkspaceEncryptionKeyBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/encryption_key", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get default app for workspace

Sends a `GET` request to `/w/{workspace}/workspaces/default_app`

*/
    pub async fn get_workspace_default_app<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<types::GetWorkspaceDefaultAppResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/default_app", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**get large file storage config

Sends a `GET` request to `/w/{workspace}/workspaces/get_large_file_storage_config`

*/
    pub async fn get_large_file_storage_config<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<types::LargeFileStorage>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/get_large_file_storage_config", self.baseurl,
            encode_path(& workspace.to_string()),
        );
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
    /**get usage

Sends a `GET` request to `/w/{workspace}/workspaces/usage`

*/
    pub async fn get_workspace_usage<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/usage", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get used triggers

Sends a `GET` request to `/w/{workspace}/workspaces/used_triggers`

*/
    pub async fn get_used_triggers<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<types::GetUsedTriggersResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/used_triggers", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**list users

Sends a `GET` request to `/w/{workspace}/users/list`

*/
    pub async fn list_users<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<Vec<types::User>>, Error<()>> {
        let url = format!(
            "{}/w/{}/users/list", self.baseurl, encode_path(& workspace.to_string()),
        );
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
    /**list users usage

Sends a `GET` request to `/w/{workspace}/users/list_usage`

*/
    pub async fn list_users_usage<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<Vec<types::UserUsage>>, Error<()>> {
        let url = format!(
            "{}/w/{}/users/list_usage", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**list usernames

Sends a `GET` request to `/w/{workspace}/users/list_usernames`

*/
    pub async fn list_usernames<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<Vec<String>>, Error<()>> {
        let url = format!(
            "{}/w/{}/users/list_usernames", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**get email from username

Sends a `GET` request to `/w/{workspace}/users/username_to_email/{username}`

*/
    pub async fn username_to_email<'a>(
        &'a self,
        workspace: &'a str,
        username: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/users/username_to_email/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& username.to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create token

Sends a `POST` request to `/users/tokens/create`

Arguments:
- `body`: new token
*/
    pub async fn create_token<'a>(
        &'a self,
        body: &'a types::NewToken,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/users/tokens/create", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create token to impersonate a user (require superadmin)

Sends a `POST` request to `/users/tokens/impersonate`

Arguments:
- `body`: new token
*/
    pub async fn create_token_impersonate<'a>(
        &'a self,
        body: &'a types::NewTokenImpersonate,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/users/tokens/impersonate", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**delete token

Sends a `DELETE` request to `/users/tokens/delete/{token_prefix}`

*/
    pub async fn delete_token<'a>(
        &'a self,
        token_prefix: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/users/tokens/delete/{}", self.baseurl, encode_path(& token_prefix
            .to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list token

Sends a `GET` request to `/users/tokens/list`

Arguments:
- `exclude_ephemeral`
- `page`: which page to return (start at 1, default 1)
- `per_page`: number of items to return for a given page (default 30, max 100)
*/
    pub async fn list_tokens<'a>(
        &'a self,
        exclude_ephemeral: Option<bool>,
        page: Option<i64>,
        per_page: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::TruncatedToken>>, Error<()>> {
        let url = format!("{}/users/tokens/list", self.baseurl,);
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &exclude_ephemeral {
            query.push(("exclude_ephemeral", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
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
    /**get OIDC token (ee only)

Sends a `POST` request to `/w/{workspace}/oidc/token/{audience}`

*/
    pub async fn get_oidc_token<'a>(
        &'a self,
        workspace: &'a str,
        audience: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/oidc/token/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& audience.to_string()),
        );
        let request = self.client.post(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create variable

Sends a `POST` request to `/w/{workspace}/variables/create`

Arguments:
- `workspace`
- `already_encrypted`
- `body`: new variable
*/
    pub async fn create_variable<'a>(
        &'a self,
        workspace: &'a str,
        already_encrypted: Option<bool>,
        body: &'a types::CreateVariable,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/variables/create", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &already_encrypted {
            query.push(("already_encrypted", v.to_string()));
        }
        let request = self.client.post(url).json(&body).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**encrypt value

Sends a `POST` request to `/w/{workspace}/variables/encrypt`

Arguments:
- `workspace`
- `body`: new variable
*/
    pub async fn encrypt_value<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/variables/encrypt", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**delete variable

Sends a `DELETE` request to `/w/{workspace}/variables/delete/{path}`

*/
    pub async fn delete_variable<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/variables/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**update variable

Sends a `POST` request to `/w/{workspace}/variables/update/{path}`

Arguments:
- `workspace`
- `path`
- `already_encrypted`
- `body`: updated variable
*/
    pub async fn update_variable<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        already_encrypted: Option<bool>,
        body: &'a types::EditVariable,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/variables/update/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &already_encrypted {
            query.push(("already_encrypted", v.to_string()));
        }
        let request = self.client.post(url).json(&body).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get variable

Sends a `GET` request to `/w/{workspace}/variables/get/{path}`

Arguments:
- `workspace`
- `path`
- `decrypt_secret`: ask to decrypt secret if this variable is secret
(if not secret no effect, default: true)

- `include_encrypted`: ask to include the encrypted value if secret and decrypt secret is not true (default: false)

*/
    pub async fn get_variable<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        decrypt_secret: Option<bool>,
        include_encrypted: Option<bool>,
    ) -> Result<ResponseValue<types::ListableVariable>, Error<()>> {
        let url = format!(
            "{}/w/{}/variables/get/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(2usize);
        if let Some(v) = &decrypt_secret {
            query.push(("decrypt_secret", v.to_string()));
        }
        if let Some(v) = &include_encrypted {
            query.push(("include_encrypted", v.to_string()));
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
    /**get variable value

Sends a `GET` request to `/w/{workspace}/variables/get_value/{path}`

*/
    pub async fn get_variable_value<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<String>, Error<()>> {
        let url = format!(
            "{}/w/{}/variables/get_value/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**does variable exists at path

Sends a `GET` request to `/w/{workspace}/variables/exists/{path}`

*/
    pub async fn exists_variable<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/variables/exists/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**list variables

Sends a `GET` request to `/w/{workspace}/variables/list`

Arguments:
- `workspace`
- `page`: which page to return (start at 1, default 1)
- `path_start`
- `per_page`: number of items to return for a given page (default 30, max 100)
*/
    pub async fn list_variable<'a>(
        &'a self,
        workspace: &'a str,
        page: Option<i64>,
        path_start: Option<&'a str>,
        per_page: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::ListableVariable>>, Error<()>> {
        let url = format!(
            "{}/w/{}/variables/list", self.baseurl, encode_path(& workspace.to_string()),
        );
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &path_start {
            query.push(("path_start", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
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
    /**list contextual variables

Sends a `GET` request to `/w/{workspace}/variables/list_contextual`

*/
    pub async fn list_contextual_variables<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<Vec<types::ContextualVariable>>, Error<()>> {
        let url = format!(
            "{}/w/{}/variables/list_contextual", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**Get all critical alerts for this workspace

Sends a `GET` request to `/w/{workspace}/workspaces/critical_alerts`

*/
    pub async fn workspace_get_critical_alerts<'a>(
        &'a self,
        workspace: &'a str,
        acknowledged: Option<bool>,
        page: Option<i64>,
        page_size: Option<i64>,
    ) -> Result<ResponseValue<types::WorkspaceGetCriticalAlertsResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/critical_alerts", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &acknowledged {
            query.push(("acknowledged", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &page_size {
            query.push(("page_size", v.to_string()));
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
    /**Acknowledge a critical alert for this workspace

Sends a `POST` request to `/w/{workspace}/workspaces/critical_alerts/{id}/acknowledge`

Arguments:
- `workspace`
- `id`: The ID of the critical alert to acknowledge
*/
    pub async fn workspace_acknowledge_critical_alert<'a>(
        &'a self,
        workspace: &'a str,
        id: i64,
    ) -> Result<ResponseValue<String>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/critical_alerts/{}/acknowledge", self.baseurl,
            encode_path(& workspace.to_string()), encode_path(& id.to_string()),
        );
        let request = self
            .client
            .post(url)
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
    /**Acknowledge all unacknowledged critical alerts for this workspace

Sends a `POST` request to `/w/{workspace}/workspaces/critical_alerts/acknowledge_all`

*/
    pub async fn workspace_acknowledge_all_critical_alerts<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<String>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/critical_alerts/acknowledge_all", self.baseurl,
            encode_path(& workspace.to_string()),
        );
        let request = self
            .client
            .post(url)
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
    /**Mute critical alert UI for this workspace

Sends a `POST` request to `/w/{workspace}/workspaces/critical_alerts/mute`

Arguments:
- `workspace`
- `body`: Boolean flag to mute critical alerts.
*/
    pub async fn workspace_mute_critical_alerts_ui<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::WorkspaceMuteCriticalAlertsUiBody,
    ) -> Result<ResponseValue<String>, Error<()>> {
        let url = format!(
            "{}/w/{}/workspaces/critical_alerts/mute", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**login with oauth authorization flow

Sends a `POST` request to `/oauth/login_callback/{client_name}`

Arguments:
- `client_name`
- `body`: Partially filled script
*/
    pub async fn login_with_oauth<'a>(
        &'a self,
        client_name: &'a str,
        body: &'a types::LoginWithOauthBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/oauth/login_callback/{}", self.baseurl, encode_path(& client_name
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**connect slack callback

Sends a `POST` request to `/w/{workspace}/oauth/connect_slack_callback`

Arguments:
- `workspace`
- `body`: code endpoint
*/
    pub async fn connect_slack_callback<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::ConnectSlackCallbackBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/oauth/connect_slack_callback", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**connect slack callback instance

Sends a `POST` request to `/oauth/connect_slack_callback`

Arguments:
- `body`: code endpoint
*/
    pub async fn connect_slack_callback_instance<'a>(
        &'a self,
        body: &'a types::ConnectSlackCallbackInstanceBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/oauth/connect_slack_callback", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**connect callback

Sends a `POST` request to `/oauth/connect_callback/{client_name}`

Arguments:
- `client_name`
- `body`: code endpoint
*/
    pub async fn connect_callback<'a>(
        &'a self,
        client_name: &'a str,
        body: &'a types::ConnectCallbackBody,
    ) -> Result<ResponseValue<types::TokenResponse>, Error<()>> {
        let url = format!(
            "{}/oauth/connect_callback/{}", self.baseurl, encode_path(& client_name
            .to_string()),
        );
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create OAuth account

Sends a `POST` request to `/w/{workspace}/oauth/create_account`

Arguments:
- `workspace`
- `body`: code endpoint
*/
    pub async fn create_account<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::CreateAccountBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/oauth/create_account", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**refresh token

Sends a `POST` request to `/w/{workspace}/oauth/refresh_token/{id}`

Arguments:
- `workspace`
- `id`
- `body`: variable path
*/
    pub async fn refresh_token<'a>(
        &'a self,
        workspace: &'a str,
        id: i64,
        body: &'a types::RefreshTokenBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/oauth/refresh_token/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**disconnect account

Sends a `POST` request to `/w/{workspace}/oauth/disconnect/{id}`

*/
    pub async fn disconnect_account<'a>(
        &'a self,
        workspace: &'a str,
        id: i64,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/oauth/disconnect/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()),
        );
        let request = self.client.post(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**disconnect slack

Sends a `POST` request to `/w/{workspace}/oauth/disconnect_slack`

*/
    pub async fn disconnect_slack<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/oauth/disconnect_slack", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**disconnect teams

Sends a `POST` request to `/w/{workspace}/oauth/disconnect_teams`

*/
    pub async fn disconnect_teams<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/oauth/disconnect_teams", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list oauth logins

Sends a `GET` request to `/oauth/list_logins`

*/
    pub async fn list_o_auth_logins<'a>(
        &'a self,
    ) -> Result<ResponseValue<types::ListOAuthLoginsResponse>, Error<()>> {
        let url = format!("{}/oauth/list_logins", self.baseurl,);
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
    /**list oauth connects

Sends a `GET` request to `/oauth/list_connects`

*/
    pub async fn list_o_auth_connects<'a>(
        &'a self,
    ) -> Result<ResponseValue<Vec<String>>, Error<()>> {
        let url = format!("{}/oauth/list_connects", self.baseurl,);
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
    /**get oauth connect

Sends a `GET` request to `/oauth/get_connect/{client}`

Arguments:
- `client`: client name
*/
    pub async fn get_o_auth_connect<'a>(
        &'a self,
        client: &'a str,
    ) -> Result<ResponseValue<types::GetOAuthConnectResponse>, Error<()>> {
        let url = format!(
            "{}/oauth/get_connect/{}", self.baseurl, encode_path(& client.to_string()),
        );
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
    /**synchronize Microsoft Teams information (teams/channels)

Sends a `POST` request to `/teams/sync`

*/
    pub async fn sync_teams<'a>(
        &'a self,
    ) -> Result<ResponseValue<Vec<types::TeamInfo>>, Error<()>> {
        let url = format!("{}/teams/sync", self.baseurl,);
        let request = self
            .client
            .post(url)
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
    /**send update to Microsoft Teams activity

Respond to a Microsoft Teams activity after a workspace command is run

Sends a `POST` request to `/teams/activities`

*/
    pub async fn send_message_to_conversation<'a>(
        &'a self,
        body: &'a types::SendMessageToConversationBody,
    ) -> Result<ResponseValue<()>, Error<()>> {
        let url = format!("{}/teams/activities", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::empty(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create resource

Sends a `POST` request to `/w/{workspace}/resources/create`

Arguments:
- `workspace`
- `update_if_exists`
- `body`: new resource
*/
    pub async fn create_resource<'a>(
        &'a self,
        workspace: &'a str,
        update_if_exists: Option<bool>,
        body: &'a types::CreateResource,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/create", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &update_if_exists {
            query.push(("update_if_exists", v.to_string()));
        }
        let request = self.client.post(url).json(&body).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**delete resource

Sends a `DELETE` request to `/w/{workspace}/resources/delete/{path}`

*/
    pub async fn delete_resource<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**update resource

Sends a `POST` request to `/w/{workspace}/resources/update/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated resource
*/
    pub async fn update_resource<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::EditResource,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/update/{}", self.baseurl, encode_path(& workspace
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
    /**update resource value

Sends a `POST` request to `/w/{workspace}/resources/update_value/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated resource
*/
    pub async fn update_resource_value<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::UpdateResourceValueBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/update_value/{}", self.baseurl, encode_path(& workspace
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
    /**get resource

Sends a `GET` request to `/w/{workspace}/resources/get/{path}`

*/
    pub async fn get_resource<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::Resource>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/get/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**get resource interpolated (variables and resources are fully unrolled)

Sends a `GET` request to `/w/{workspace}/resources/get_value_interpolated/{path}`

Arguments:
- `workspace`
- `path`
- `job_id`: job id
*/
    pub async fn get_resource_value_interpolated<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        job_id: Option<&'a uuid::Uuid>,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/get_value_interpolated/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &job_id {
            query.push(("job_id", v.to_string()));
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
    /**get resource value

Sends a `GET` request to `/w/{workspace}/resources/get_value/{path}`

*/
    pub async fn get_resource_value<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/get_value/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**does resource exists

Sends a `GET` request to `/w/{workspace}/resources/exists/{path}`

*/
    pub async fn exists_resource<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/exists/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**list resources

Sends a `GET` request to `/w/{workspace}/resources/list`

Arguments:
- `workspace`
- `page`: which page to return (start at 1, default 1)
- `path_start`
- `per_page`: number of items to return for a given page (default 30, max 100)
- `resource_type`: resource_types to list from, separated by ',',
- `resource_type_exclude`: resource_types to not list from, separated by ',',
*/
    pub async fn list_resource<'a>(
        &'a self,
        workspace: &'a str,
        page: Option<i64>,
        path_start: Option<&'a str>,
        per_page: Option<i64>,
        resource_type: Option<&'a str>,
        resource_type_exclude: Option<&'a str>,
    ) -> Result<ResponseValue<Vec<types::ListableResource>>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/list", self.baseurl, encode_path(& workspace.to_string()),
        );
        let mut query = Vec::with_capacity(5usize);
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &path_start {
            query.push(("path_start", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
        }
        if let Some(v) = &resource_type {
            query.push(("resource_type", v.to_string()));
        }
        if let Some(v) = &resource_type_exclude {
            query.push(("resource_type_exclude", v.to_string()));
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
    /**list resources for search

Sends a `GET` request to `/w/{workspace}/resources/list_search`

*/
    pub async fn list_search_resource<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<Vec<types::ListSearchResourceResponseItem>>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/list_search", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**list resource names

Sends a `GET` request to `/w/{workspace}/resources/list_names/{name}`

*/
    pub async fn list_resource_names<'a>(
        &'a self,
        workspace: &'a str,
        name: &'a str,
    ) -> Result<ResponseValue<Vec<types::ListResourceNamesResponseItem>>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/list_names/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& name.to_string()),
        );
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
    /**create resource_type

Sends a `POST` request to `/w/{workspace}/resources/type/create`

Arguments:
- `workspace`
- `body`: new resource_type
*/
    pub async fn create_resource_type<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::ResourceType,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/type/create", self.baseurl, encode_path(& workspace
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
    /**get map from resource type to format extension

Sends a `GET` request to `/w/{workspace}/resources/file_resource_type_to_file_ext_map`

*/
    pub async fn file_resource_type_to_file_ext_map<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/file_resource_type_to_file_ext_map", self.baseurl,
            encode_path(& workspace.to_string()),
        );
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
    /**delete resource_type

Sends a `DELETE` request to `/w/{workspace}/resources/type/delete/{path}`

*/
    pub async fn delete_resource_type<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/type/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**update resource_type

Sends a `POST` request to `/w/{workspace}/resources/type/update/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated resource_type
*/
    pub async fn update_resource_type<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::EditResourceType,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/type/update/{}", self.baseurl, encode_path(& workspace
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
    /**get resource_type

Sends a `GET` request to `/w/{workspace}/resources/type/get/{path}`

*/
    pub async fn get_resource_type<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::ResourceType>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/type/get/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**does resource_type exists

Sends a `GET` request to `/w/{workspace}/resources/type/exists/{path}`

*/
    pub async fn exists_resource_type<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/type/exists/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**list resource_types

Sends a `GET` request to `/w/{workspace}/resources/type/list`

*/
    pub async fn list_resource_type<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<Vec<types::ResourceType>>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/type/list", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**list resource_types names

Sends a `GET` request to `/w/{workspace}/resources/type/listnames`

*/
    pub async fn list_resource_type_names<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<Vec<String>>, Error<()>> {
        let url = format!(
            "{}/w/{}/resources/type/listnames", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**query resource types by similarity

Sends a `GET` request to `/w/{workspace}/embeddings/query_resource_types`

Arguments:
- `workspace`
- `limit`: query limit
- `text`: query text
*/
    pub async fn query_resource_types<'a>(
        &'a self,
        workspace: &'a str,
        limit: Option<f64>,
        text: &'a str,
    ) -> Result<ResponseValue<Vec<types::QueryResourceTypesResponseItem>>, Error<()>> {
        let url = format!(
            "{}/w/{}/embeddings/query_resource_types", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let mut query = Vec::with_capacity(2usize);
        if let Some(v) = &limit {
            query.push(("limit", v.to_string()));
        }
        query.push(("text", text.to_string()));
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
    /**list hub integrations

Sends a `GET` request to `/integrations/hub/list`

Arguments:
- `kind`: query integrations kind
*/
    pub async fn list_hub_integrations<'a>(
        &'a self,
        kind: Option<&'a str>,
    ) -> Result<ResponseValue<Vec<types::ListHubIntegrationsResponseItem>>, Error<()>> {
        let url = format!("{}/integrations/hub/list", self.baseurl,);
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &kind {
            query.push(("kind", v.to_string()));
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
    /**list all hub flows

Sends a `GET` request to `/flows/hub/list`

*/
    pub async fn list_hub_flows<'a>(
        &'a self,
    ) -> Result<ResponseValue<types::ListHubFlowsResponse>, Error<()>> {
        let url = format!("{}/flows/hub/list", self.baseurl,);
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
    /**get hub flow by id

Sends a `GET` request to `/flows/hub/get/{id}`

*/
    pub async fn get_hub_flow_by_id<'a>(
        &'a self,
        id: i64,
    ) -> Result<ResponseValue<types::GetHubFlowByIdResponse>, Error<()>> {
        let url = format!(
            "{}/flows/hub/get/{}", self.baseurl, encode_path(& id.to_string()),
        );
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
    /**list all hub apps

Sends a `GET` request to `/apps/hub/list`

*/
    pub async fn list_hub_apps<'a>(
        &'a self,
    ) -> Result<ResponseValue<types::ListHubAppsResponse>, Error<()>> {
        let url = format!("{}/apps/hub/list", self.baseurl,);
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
    /**get hub app by id

Sends a `GET` request to `/apps/hub/get/{id}`

*/
    pub async fn get_hub_app_by_id<'a>(
        &'a self,
        id: i64,
    ) -> Result<ResponseValue<types::GetHubAppByIdResponse>, Error<()>> {
        let url = format!(
            "{}/apps/hub/get/{}", self.baseurl, encode_path(& id.to_string()),
        );
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
    /**get public app by custom path

Sends a `GET` request to `/apps_u/public_app_by_custom_path/{custom_path}`

*/
    pub async fn get_public_app_by_custom_path<'a>(
        &'a self,
        custom_path: &'a str,
    ) -> Result<ResponseValue<types::AppWithLastVersion>, Error<()>> {
        let url = format!(
            "{}/apps_u/public_app_by_custom_path/{}", self.baseurl, encode_path(&
            custom_path.to_string()),
        );
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
    /**get hub script content by path

Sends a `GET` request to `/scripts/hub/get/{path}`

*/
    pub async fn get_hub_script_content_by_path<'a>(
        &'a self,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/scripts/hub/get/{}", self.baseurl, encode_path(& path.to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get full hub script by path

Sends a `GET` request to `/scripts/hub/get_full/{path}`

*/
    pub async fn get_hub_script_by_path<'a>(
        &'a self,
        path: &'a str,
    ) -> Result<ResponseValue<types::GetHubScriptByPathResponse>, Error<()>> {
        let url = format!(
            "{}/scripts/hub/get_full/{}", self.baseurl, encode_path(& path.to_string()),
        );
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
    /**get top hub scripts

Sends a `GET` request to `/scripts/hub/top`

Arguments:
- `app`: query scripts app
- `kind`: query scripts kind
- `limit`: query limit
*/
    pub async fn get_top_hub_scripts<'a>(
        &'a self,
        app: Option<&'a str>,
        kind: Option<&'a str>,
        limit: Option<f64>,
    ) -> Result<ResponseValue<types::GetTopHubScriptsResponse>, Error<()>> {
        let url = format!("{}/scripts/hub/top", self.baseurl,);
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &app {
            query.push(("app", v.to_string()));
        }
        if let Some(v) = &kind {
            query.push(("kind", v.to_string()));
        }
        if let Some(v) = &limit {
            query.push(("limit", v.to_string()));
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
    /**query hub scripts by similarity

Sends a `GET` request to `/embeddings/query_hub_scripts`

Arguments:
- `app`: query scripts app
- `kind`: query scripts kind
- `limit`: query limit
- `text`: query text
*/
    pub async fn query_hub_scripts<'a>(
        &'a self,
        app: Option<&'a str>,
        kind: Option<&'a str>,
        limit: Option<f64>,
        text: &'a str,
    ) -> Result<ResponseValue<Vec<types::QueryHubScriptsResponseItem>>, Error<()>> {
        let url = format!("{}/embeddings/query_hub_scripts", self.baseurl,);
        let mut query = Vec::with_capacity(4usize);
        if let Some(v) = &app {
            query.push(("app", v.to_string()));
        }
        if let Some(v) = &kind {
            query.push(("kind", v.to_string()));
        }
        if let Some(v) = &limit {
            query.push(("limit", v.to_string()));
        }
        query.push(("text", text.to_string()));
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
    /**list scripts for search

Sends a `GET` request to `/w/{workspace}/scripts/list_search`

*/
    pub async fn list_search_script<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<Vec<types::ListSearchScriptResponseItem>>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/list_search", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**list all scripts

Sends a `GET` request to `/w/{workspace}/scripts/list`

Arguments:
- `workspace`
- `created_by`: mask to filter exact matching user creator
- `first_parent_hash`: mask to filter scripts whom first direct parent has exact hash
- `include_draft_only`: (default false)
include scripts that have no deployed version

- `include_without_main`: (default false)
include scripts without an exported main function

- `is_template`: (default regardless)
if true show only the templates
if false show only the non templates
if not defined, show all regardless of if the script is a template

- `kinds`: (default regardless)
script kinds to filter, split by comma

- `last_parent_hash`: mask to filter scripts whom last parent in the chain has exact hash.
Beware that each script stores only a limited number of parents. Hence
the last parent hash for a script is not necessarily its top-most parent.
To find the top-most parent you will have to jump from last to last hash
 until finding the parent

- `order_desc`: order by desc order (default true)
- `page`: which page to return (start at 1, default 1)
- `parent_hash`: is the hash present in the array of stored parent hashes for this script.
The same warning applies than for last_parent_hash. A script only store a
limited number of direct parent

- `path_exact`: mask to filter exact matching path
- `path_start`: mask to filter matching starting path
- `per_page`: number of items to return for a given page (default 30, max 100)
- `show_archived`: (default false)
show only the archived files.
when multiple archived hash share the same path, only the ones with the latest create_at
are
ed.

- `starred_only`: (default false)
show only the starred items

- `with_deployment_msg`: (default false)
include deployment message

*/
    pub async fn list_scripts<'a>(
        &'a self,
        workspace: &'a str,
        created_by: Option<&'a str>,
        first_parent_hash: Option<&'a str>,
        include_draft_only: Option<bool>,
        include_without_main: Option<bool>,
        is_template: Option<bool>,
        kinds: Option<&'a str>,
        last_parent_hash: Option<&'a str>,
        order_desc: Option<bool>,
        page: Option<i64>,
        parent_hash: Option<&'a str>,
        path_exact: Option<&'a str>,
        path_start: Option<&'a str>,
        per_page: Option<i64>,
        show_archived: Option<bool>,
        starred_only: Option<bool>,
        with_deployment_msg: Option<bool>,
    ) -> Result<ResponseValue<Vec<types::Script>>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/list", self.baseurl, encode_path(& workspace.to_string()),
        );
        let mut query = Vec::with_capacity(16usize);
        if let Some(v) = &created_by {
            query.push(("created_by", v.to_string()));
        }
        if let Some(v) = &first_parent_hash {
            query.push(("first_parent_hash", v.to_string()));
        }
        if let Some(v) = &include_draft_only {
            query.push(("include_draft_only", v.to_string()));
        }
        if let Some(v) = &include_without_main {
            query.push(("include_without_main", v.to_string()));
        }
        if let Some(v) = &is_template {
            query.push(("is_template", v.to_string()));
        }
        if let Some(v) = &kinds {
            query.push(("kinds", v.to_string()));
        }
        if let Some(v) = &last_parent_hash {
            query.push(("last_parent_hash", v.to_string()));
        }
        if let Some(v) = &order_desc {
            query.push(("order_desc", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &parent_hash {
            query.push(("parent_hash", v.to_string()));
        }
        if let Some(v) = &path_exact {
            query.push(("path_exact", v.to_string()));
        }
        if let Some(v) = &path_start {
            query.push(("path_start", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
        }
        if let Some(v) = &show_archived {
            query.push(("show_archived", v.to_string()));
        }
        if let Some(v) = &starred_only {
            query.push(("starred_only", v.to_string()));
        }
        if let Some(v) = &with_deployment_msg {
            query.push(("with_deployment_msg", v.to_string()));
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
    /**list all scripts paths

Sends a `GET` request to `/w/{workspace}/scripts/list_paths`

*/
    pub async fn list_script_paths<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/list_paths", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create draft

Sends a `POST` request to `/w/{workspace}/drafts/create`

*/
    pub async fn create_draft<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::CreateDraftBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/drafts/create", self.baseurl, encode_path(& workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**delete draft

Sends a `DELETE` request to `/w/{workspace}/drafts/delete/{kind}/{path}`

*/
    pub async fn delete_draft<'a>(
        &'a self,
        workspace: &'a str,
        kind: types::DeleteDraftKind,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/drafts/delete/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& kind.to_string()), encode_path(& path
            .to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
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
    /**Toggle ON and OFF the workspace error handler for a given script

Sends a `POST` request to `/w/{workspace}/scripts/toggle_workspace_error_handler/p/{path}`

Arguments:
- `workspace`
- `path`
- `body`: Workspace error handler enabled
*/
    pub async fn toggle_workspace_error_handler_for_script<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::ToggleWorkspaceErrorHandlerForScriptBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/toggle_workspace_error_handler/p/{}", self.baseurl,
            encode_path(& workspace.to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get all instance custom tags (tags are used to dispatch jobs to different worker groups)

Sends a `GET` request to `/workers/custom_tags`

*/
    pub async fn get_custom_tags<'a>(
        &'a self,
        show_workspace_restriction: Option<bool>,
        workspace: Option<&'a str>,
    ) -> Result<ResponseValue<Vec<String>>, Error<()>> {
        let url = format!("{}/workers/custom_tags", self.baseurl,);
        let mut query = Vec::with_capacity(2usize);
        if let Some(v) = &show_workspace_restriction {
            query.push(("show_workspace_restriction", v.to_string()));
        }
        if let Some(v) = &workspace {
            query.push(("workspace", v.to_string()));
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
    /**get all instance default tags

Sends a `GET` request to `/workers/get_default_tags`

*/
    pub async fn ge_default_tags<'a>(
        &'a self,
    ) -> Result<ResponseValue<Vec<String>>, Error<()>> {
        let url = format!("{}/workers/get_default_tags", self.baseurl,);
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
    /**is default tags per workspace

Sends a `GET` request to `/workers/is_default_tags_per_workspace`

*/
    pub async fn is_default_tags_per_workspace<'a>(
        &'a self,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!("{}/workers/is_default_tags_per_workspace", self.baseurl,);
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
    /**archive script by path

Sends a `POST` request to `/w/{workspace}/scripts/archive/p/{path}`

*/
    pub async fn archive_script_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/archive/p/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.post(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**archive script by hash

Sends a `POST` request to `/w/{workspace}/scripts/archive/h/{hash}`

*/
    pub async fn archive_script_by_hash<'a>(
        &'a self,
        workspace: &'a str,
        hash: &'a str,
    ) -> Result<ResponseValue<types::Script>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/archive/h/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& hash.to_string()),
        );
        let request = self
            .client
            .post(url)
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
    /**delete script by hash (erase content but keep hash, require admin)

Sends a `POST` request to `/w/{workspace}/scripts/delete/h/{hash}`

*/
    pub async fn delete_script_by_hash<'a>(
        &'a self,
        workspace: &'a str,
        hash: &'a str,
    ) -> Result<ResponseValue<types::Script>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/delete/h/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& hash.to_string()),
        );
        let request = self
            .client
            .post(url)
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
    /**delete script at a given path (require admin)

Sends a `POST` request to `/w/{workspace}/scripts/delete/p/{path}`

Arguments:
- `workspace`
- `path`
- `keep_captures`: keep captures
*/
    pub async fn delete_script_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        keep_captures: Option<bool>,
    ) -> Result<ResponseValue<String>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/delete/p/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &keep_captures {
            query.push(("keep_captures", v.to_string()));
        }
        let request = self
            .client
            .post(url)
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
    /**get script by path

Sends a `GET` request to `/w/{workspace}/scripts/get/p/{path}`

*/
    pub async fn get_script_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        with_starred_info: Option<bool>,
    ) -> Result<ResponseValue<types::Script>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/get/p/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
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
    /**get triggers count of script

Sends a `GET` request to `/w/{workspace}/scripts/get_triggers_count/{path}`

*/
    pub async fn get_triggers_count_of_script<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::TriggersCount>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/get_triggers_count/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& path.to_string()),
        );
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
    /**get tokens with script scope

Sends a `GET` request to `/w/{workspace}/scripts/list_tokens/{path}`

*/
    pub async fn list_tokens_of_script<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<Vec<types::TruncatedToken>>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/list_tokens/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**get script by path with draft

Sends a `GET` request to `/w/{workspace}/scripts/get/draft/{path}`

*/
    pub async fn get_script_by_path_with_draft<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::NewScriptWithDraft>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/get/draft/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**get history of a script by path

Sends a `GET` request to `/w/{workspace}/scripts/history/p/{path}`

*/
    pub async fn get_script_history_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<Vec<types::ScriptHistory>>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/history/p/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**get scripts's latest version (hash)

Sends a `GET` request to `/w/{workspace}/scripts/get_latest_version/{path}`

*/
    pub async fn get_script_latest_version<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::ScriptHistory>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/get_latest_version/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& path.to_string()),
        );
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
    /**update history of a script

Sends a `POST` request to `/w/{workspace}/scripts/history_update/h/{hash}/p/{path}`

Arguments:
- `workspace`
- `hash`
- `path`
- `body`: Script deployment message
*/
    pub async fn update_script_history<'a>(
        &'a self,
        workspace: &'a str,
        hash: &'a str,
        path: &'a str,
        body: &'a types::UpdateScriptHistoryBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/history_update/h/{}/p/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& hash.to_string()), encode_path(& path
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**raw script by path

Sends a `GET` request to `/w/{workspace}/scripts/raw/p/{path}`

*/
    pub async fn raw_script_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/raw/p/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**raw script by path with a token (mostly used by lsp to be used with import maps to resolve scripts)

Sends a `GET` request to `/scripts_u/tokened_raw/{workspace}/{token}/{path}`

*/
    pub async fn raw_script_by_path_tokened<'a>(
        &'a self,
        workspace: &'a str,
        token: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/scripts_u/tokened_raw/{}/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& token.to_string()), encode_path(& path
            .to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**exists script by path

Sends a `GET` request to `/w/{workspace}/scripts/exists/p/{path}`

*/
    pub async fn exists_script_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/exists/p/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**get script by hash

Sends a `GET` request to `/w/{workspace}/scripts/get/h/{hash}`

*/
    pub async fn get_script_by_hash<'a>(
        &'a self,
        workspace: &'a str,
        hash: &'a str,
        with_starred_info: Option<bool>,
    ) -> Result<ResponseValue<types::Script>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/get/h/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& hash.to_string()),
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
    /**raw script by hash

Sends a `GET` request to `/w/{workspace}/scripts/raw/h/{path}`

*/
    pub async fn raw_script_by_hash<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/raw/h/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get script deployment status

Sends a `GET` request to `/w/{workspace}/scripts/deployment_status/h/{hash}`

*/
    pub async fn get_script_deployment_status<'a>(
        &'a self,
        workspace: &'a str,
        hash: &'a str,
    ) -> Result<ResponseValue<types::GetScriptDeploymentStatusResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/scripts/deployment_status/h/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& hash.to_string()),
        );
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
    /**run script by path

Sends a `POST` request to `/w/{workspace}/jobs/run/p/{path}`

Arguments:
- `workspace`
- `path`
- `cache_ttl`: Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
- `invisible_to_owner`: make the run invisible to the the script owner (default false)
- `job_id`: The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
- `parent_job`: The parent job that is at the origin and responsible for the execution of this script if any
- `scheduled_for`: when to schedule this job (leave empty for immediate run)
- `scheduled_in_secs`: schedule the script to execute in the number of seconds starting now
- `skip_preprocessor`: skip the preprocessor
- `tag`: Override the tag to use
- `body`: script args
*/
    pub async fn run_script_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        cache_ttl: Option<&'a str>,
        invisible_to_owner: Option<bool>,
        job_id: Option<&'a uuid::Uuid>,
        parent_job: Option<&'a uuid::Uuid>,
        scheduled_for: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        scheduled_in_secs: Option<i64>,
        skip_preprocessor: Option<bool>,
        tag: Option<&'a str>,
        body: &'a types::ScriptArgs,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/run/p/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(8usize);
        if let Some(v) = &cache_ttl {
            query.push(("cache_ttl", v.to_string()));
        }
        if let Some(v) = &invisible_to_owner {
            query.push(("invisible_to_owner", v.to_string()));
        }
        if let Some(v) = &job_id {
            query.push(("job_id", v.to_string()));
        }
        if let Some(v) = &parent_job {
            query.push(("parent_job", v.to_string()));
        }
        if let Some(v) = &scheduled_for {
            query.push(("scheduled_for", v.to_string()));
        }
        if let Some(v) = &scheduled_in_secs {
            query.push(("scheduled_in_secs", v.to_string()));
        }
        if let Some(v) = &skip_preprocessor {
            query.push(("skip_preprocessor", v.to_string()));
        }
        if let Some(v) = &tag {
            query.push(("tag", v.to_string()));
        }
        let request = self.client.post(url).json(&body).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**run script by path in openai format

Sends a `POST` request to `/w/{workspace}/jobs/openai_sync/p/{path}`

Arguments:
- `workspace`
- `path`
- `include_header`: List of headers's keys (separated with ',') whove value are added to the args
Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key

- `job_id`: The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
- `parent_job`: The parent job that is at the origin and responsible for the execution of this script if any
- `queue_limit`: The maximum size of the queue for which the request would get rejected if that job would push it above that limit

- `body`: script args
*/
    pub async fn openai_sync_script_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        include_header: Option<&'a str>,
        job_id: Option<&'a uuid::Uuid>,
        parent_job: Option<&'a uuid::Uuid>,
        queue_limit: Option<&'a str>,
        body: &'a types::ScriptArgs,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/openai_sync/p/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(4usize);
        if let Some(v) = &include_header {
            query.push(("include_header", v.to_string()));
        }
        if let Some(v) = &job_id {
            query.push(("job_id", v.to_string()));
        }
        if let Some(v) = &parent_job {
            query.push(("parent_job", v.to_string()));
        }
        if let Some(v) = &queue_limit {
            query.push(("queue_limit", v.to_string()));
        }
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .query(&query)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**run script by path with get

Sends a `GET` request to `/w/{workspace}/jobs/run_wait_result/p/{path}`

Arguments:
- `workspace`
- `path`
- `cache_ttl`: Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
- `include_header`: List of headers's keys (separated with ',') whove value are added to the args
Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key

- `job_id`: The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
- `parent_job`: The parent job that is at the origin and responsible for the execution of this script if any
- `payload`: The base64 encoded payload that has been encoded as a JSON. e.g how to encode such payload encodeURIComponent
`encodeURIComponent(btoa(JSON.stringify({a: 2})))`

- `queue_limit`: The maximum size of the queue for which the request would get rejected if that job would push it above that limit

- `tag`: Override the tag to use
*/
    pub async fn run_wait_result_script_by_path_get<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        cache_ttl: Option<&'a str>,
        include_header: Option<&'a str>,
        job_id: Option<&'a uuid::Uuid>,
        parent_job: Option<&'a uuid::Uuid>,
        payload: Option<&'a str>,
        queue_limit: Option<&'a str>,
        tag: Option<&'a str>,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/run_wait_result/p/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(7usize);
        if let Some(v) = &cache_ttl {
            query.push(("cache_ttl", v.to_string()));
        }
        if let Some(v) = &include_header {
            query.push(("include_header", v.to_string()));
        }
        if let Some(v) = &job_id {
            query.push(("job_id", v.to_string()));
        }
        if let Some(v) = &parent_job {
            query.push(("parent_job", v.to_string()));
        }
        if let Some(v) = &payload {
            query.push(("payload", v.to_string()));
        }
        if let Some(v) = &queue_limit {
            query.push(("queue_limit", v.to_string()));
        }
        if let Some(v) = &tag {
            query.push(("tag", v.to_string()));
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
    /**run script by path

Sends a `POST` request to `/w/{workspace}/jobs/run_wait_result/p/{path}`

Arguments:
- `workspace`
- `path`
- `cache_ttl`: Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
- `include_header`: List of headers's keys (separated with ',') whove value are added to the args
Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key

- `job_id`: The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
- `parent_job`: The parent job that is at the origin and responsible for the execution of this script if any
- `queue_limit`: The maximum size of the queue for which the request would get rejected if that job would push it above that limit

- `tag`: Override the tag to use
- `body`: script args
*/
    pub async fn run_wait_result_script_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        cache_ttl: Option<&'a str>,
        include_header: Option<&'a str>,
        job_id: Option<&'a uuid::Uuid>,
        parent_job: Option<&'a uuid::Uuid>,
        queue_limit: Option<&'a str>,
        tag: Option<&'a str>,
        body: &'a types::ScriptArgs,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/run_wait_result/p/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(6usize);
        if let Some(v) = &cache_ttl {
            query.push(("cache_ttl", v.to_string()));
        }
        if let Some(v) = &include_header {
            query.push(("include_header", v.to_string()));
        }
        if let Some(v) = &job_id {
            query.push(("job_id", v.to_string()));
        }
        if let Some(v) = &parent_job {
            query.push(("parent_job", v.to_string()));
        }
        if let Some(v) = &queue_limit {
            query.push(("queue_limit", v.to_string()));
        }
        if let Some(v) = &tag {
            query.push(("tag", v.to_string()));
        }
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .query(&query)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**run flow by path and wait until completion in openai format

Sends a `POST` request to `/w/{workspace}/jobs/openai_sync/f/{path}`

Arguments:
- `workspace`
- `path`
- `include_header`: List of headers's keys (separated with ',') whove value are added to the args
Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key

- `job_id`: The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
- `queue_limit`: The maximum size of the queue for which the request would get rejected if that job would push it above that limit

- `body`: script args
*/
    pub async fn openai_sync_flow_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        include_header: Option<&'a str>,
        job_id: Option<&'a uuid::Uuid>,
        queue_limit: Option<&'a str>,
        body: &'a types::ScriptArgs,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/openai_sync/f/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &include_header {
            query.push(("include_header", v.to_string()));
        }
        if let Some(v) = &job_id {
            query.push(("job_id", v.to_string()));
        }
        if let Some(v) = &queue_limit {
            query.push(("queue_limit", v.to_string()));
        }
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .query(&query)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**run flow by path and wait until completion

Sends a `POST` request to `/w/{workspace}/jobs/run_wait_result/f/{path}`

Arguments:
- `workspace`
- `path`
- `include_header`: List of headers's keys (separated with ',') whove value are added to the args
Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key

- `job_id`: The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
- `queue_limit`: The maximum size of the queue for which the request would get rejected if that job would push it above that limit

- `body`: script args
*/
    pub async fn run_wait_result_flow_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        include_header: Option<&'a str>,
        job_id: Option<&'a uuid::Uuid>,
        queue_limit: Option<&'a str>,
        body: &'a types::ScriptArgs,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/run_wait_result/f/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &include_header {
            query.push(("include_header", v.to_string()));
        }
        if let Some(v) = &job_id {
            query.push(("job_id", v.to_string()));
        }
        if let Some(v) = &queue_limit {
            query.push(("queue_limit", v.to_string()));
        }
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .query(&query)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get job result by id

Sends a `GET` request to `/w/{workspace}/jobs/result_by_id/{flow_job_id}/{node_id}`

*/
    pub async fn result_by_id<'a>(
        &'a self,
        workspace: &'a str,
        flow_job_id: &'a str,
        node_id: &'a str,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/result_by_id/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& flow_job_id.to_string()), encode_path(& node_id
            .to_string()),
        );
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
    /**list all flow paths

Sends a `GET` request to `/w/{workspace}/flows/list_paths`

*/
    pub async fn list_flow_paths<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/list_paths", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list flows for search

Sends a `GET` request to `/w/{workspace}/flows/list_search`

*/
    pub async fn list_search_flow<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<Vec<types::ListSearchFlowResponseItem>>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/list_search", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**list all flows

Sends a `GET` request to `/w/{workspace}/flows/list`

Arguments:
- `workspace`
- `created_by`: mask to filter exact matching user creator
- `include_draft_only`: (default false)
include items that have no deployed version

- `order_desc`: order by desc order (default true)
- `page`: which page to return (start at 1, default 1)
- `path_exact`: mask to filter exact matching path
- `path_start`: mask to filter matching starting path
- `per_page`: number of items to return for a given page (default 30, max 100)
- `show_archived`: (default false)
show only the archived files.
when multiple archived hash share the same path, only the ones with the latest create_at
are displayed.

- `starred_only`: (default false)
show only the starred items

- `with_deployment_msg`: (default false)
include deployment message

*/
    pub async fn list_flows<'a>(
        &'a self,
        workspace: &'a str,
        created_by: Option<&'a str>,
        include_draft_only: Option<bool>,
        order_desc: Option<bool>,
        page: Option<i64>,
        path_exact: Option<&'a str>,
        path_start: Option<&'a str>,
        per_page: Option<i64>,
        show_archived: Option<bool>,
        starred_only: Option<bool>,
        with_deployment_msg: Option<bool>,
    ) -> Result<ResponseValue<Vec<types::ListFlowsResponseItem>>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/list", self.baseurl, encode_path(& workspace.to_string()),
        );
        let mut query = Vec::with_capacity(10usize);
        if let Some(v) = &created_by {
            query.push(("created_by", v.to_string()));
        }
        if let Some(v) = &include_draft_only {
            query.push(("include_draft_only", v.to_string()));
        }
        if let Some(v) = &order_desc {
            query.push(("order_desc", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &path_exact {
            query.push(("path_exact", v.to_string()));
        }
        if let Some(v) = &path_start {
            query.push(("path_start", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
        }
        if let Some(v) = &show_archived {
            query.push(("show_archived", v.to_string()));
        }
        if let Some(v) = &starred_only {
            query.push(("starred_only", v.to_string()));
        }
        if let Some(v) = &with_deployment_msg {
            query.push(("with_deployment_msg", v.to_string()));
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
    /**get flow history by path

Sends a `GET` request to `/w/{workspace}/flows/history/p/{path}`

*/
    pub async fn get_flow_history<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<Vec<types::FlowVersion>>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/history/p/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**get flow's latest version

Sends a `GET` request to `/w/{workspace}/flows/get_latest_version/{path}`

*/
    pub async fn get_flow_latest_version<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::FlowVersion>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/get_latest_version/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**list flow paths from workspace runnable

Sends a `GET` request to `/w/{workspace}/flows/list_paths_from_workspace_runnable/{runnable_kind}/{path}`

*/
    pub async fn list_flow_paths_from_workspace_runnable<'a>(
        &'a self,
        workspace: &'a str,
        runnable_kind: types::ListFlowPathsFromWorkspaceRunnableRunnableKind,
        path: &'a str,
    ) -> Result<ResponseValue<Vec<String>>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/list_paths_from_workspace_runnable/{}/{}", self.baseurl,
            encode_path(& workspace.to_string()), encode_path(& runnable_kind
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**get flow version

Sends a `GET` request to `/w/{workspace}/flows/get/v/{version}/p/{path}`

*/
    pub async fn get_flow_version<'a>(
        &'a self,
        workspace: &'a str,
        version: f64,
        path: &'a str,
    ) -> Result<ResponseValue<types::Flow>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/get/v/{}/p/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& version.to_string()), encode_path(& path
            .to_string()),
        );
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
    /**update flow history

Sends a `POST` request to `/w/{workspace}/flows/history_update/v/{version}/p/{path}`

Arguments:
- `workspace`
- `version`
- `path`
- `body`: Flow deployment message
*/
    pub async fn update_flow_history<'a>(
        &'a self,
        workspace: &'a str,
        version: f64,
        path: &'a str,
        body: &'a types::UpdateFlowHistoryBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/history_update/v/{}/p/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& version.to_string()), encode_path(&
            path.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
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
    /**get flow deployment status

Sends a `GET` request to `/w/{workspace}/flows/deployment_status/p/{path}`

*/
    pub async fn get_flow_deployment_status<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::GetFlowDeploymentStatusResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/deployment_status/p/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**get triggers count of flow

Sends a `GET` request to `/w/{workspace}/flows/get_triggers_count/{path}`

*/
    pub async fn get_triggers_count_of_flow<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::TriggersCount>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/get_triggers_count/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**get tokens with flow scope

Sends a `GET` request to `/w/{workspace}/flows/list_tokens/{path}`

*/
    pub async fn list_tokens_of_flow<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<Vec<types::TruncatedToken>>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/list_tokens/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**Toggle ON and OFF the workspace error handler for a given flow

Sends a `POST` request to `/w/{workspace}/flows/toggle_workspace_error_handler/{path}`

Arguments:
- `workspace`
- `path`
- `body`: Workspace error handler enabled
*/
    pub async fn toggle_workspace_error_handler_for_flow<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::ToggleWorkspaceErrorHandlerForFlowBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/toggle_workspace_error_handler/{}", self.baseurl,
            encode_path(& workspace.to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get flow by path with draft

Sends a `GET` request to `/w/{workspace}/flows/get/draft/{path}`

*/
    pub async fn get_flow_by_path_with_draft<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::GetFlowByPathWithDraftResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/get/draft/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**exists flow by path

Sends a `GET` request to `/w/{workspace}/flows/exists/{path}`

*/
    pub async fn exists_flow_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/exists/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**update flow

Sends a `POST` request to `/w/{workspace}/flows/update/{path}`

Arguments:
- `workspace`
- `path`
- `body`: Partially filled flow
*/
    pub async fn update_flow<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::UpdateFlowBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/update/{}", self.baseurl, encode_path(& workspace
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
    /**archive flow by path

Sends a `POST` request to `/w/{workspace}/flows/archive/{path}`

Arguments:
- `workspace`
- `path`
- `body`: archiveFlow
*/
    pub async fn archive_flow_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::ArchiveFlowByPathBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/archive/{}", self.baseurl, encode_path(& workspace
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
    /**delete flow by path

Sends a `DELETE` request to `/w/{workspace}/flows/delete/{path}`

Arguments:
- `workspace`
- `path`
- `keep_captures`: keep captures
*/
    pub async fn delete_flow_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        keep_captures: Option<bool>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/flows/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &keep_captures {
            query.push(("keep_captures", v.to_string()));
        }
        let request = self.client.delete(url).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list all raw apps

Sends a `GET` request to `/w/{workspace}/raw_apps/list`

Arguments:
- `workspace`
- `created_by`: mask to filter exact matching user creator
- `order_desc`: order by desc order (default true)
- `page`: which page to return (start at 1, default 1)
- `path_exact`: mask to filter exact matching path
- `path_start`: mask to filter matching starting path
- `per_page`: number of items to return for a given page (default 30, max 100)
- `starred_only`: (default false)
show only the starred items

*/
    pub async fn list_raw_apps<'a>(
        &'a self,
        workspace: &'a str,
        created_by: Option<&'a str>,
        order_desc: Option<bool>,
        page: Option<i64>,
        path_exact: Option<&'a str>,
        path_start: Option<&'a str>,
        per_page: Option<i64>,
        starred_only: Option<bool>,
    ) -> Result<ResponseValue<Vec<types::ListableRawApp>>, Error<()>> {
        let url = format!(
            "{}/w/{}/raw_apps/list", self.baseurl, encode_path(& workspace.to_string()),
        );
        let mut query = Vec::with_capacity(7usize);
        if let Some(v) = &created_by {
            query.push(("created_by", v.to_string()));
        }
        if let Some(v) = &order_desc {
            query.push(("order_desc", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &path_exact {
            query.push(("path_exact", v.to_string()));
        }
        if let Some(v) = &path_start {
            query.push(("path_start", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
        }
        if let Some(v) = &starred_only {
            query.push(("starred_only", v.to_string()));
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
    /**does an app exisst at path

Sends a `GET` request to `/w/{workspace}/raw_apps/exists/{path}`

*/
    pub async fn exists_raw_app<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/raw_apps/exists/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**get app by path

Sends a `GET` request to `/w/{workspace}/apps/get_data/{version}/{path}`

*/
    pub async fn get_raw_app_data<'a>(
        &'a self,
        workspace: &'a str,
        version: f64,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/get_data/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& version.to_string()), encode_path(& path
            .to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list apps for search

Sends a `GET` request to `/w/{workspace}/apps/list_search`

*/
    pub async fn list_search_app<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<Vec<types::ListSearchAppResponseItem>>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/list_search", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**list all apps

Sends a `GET` request to `/w/{workspace}/apps/list`

Arguments:
- `workspace`
- `created_by`: mask to filter exact matching user creator
- `include_draft_only`: (default false)
include items that have no deployed version

- `order_desc`: order by desc order (default true)
- `page`: which page to return (start at 1, default 1)
- `path_exact`: mask to filter exact matching path
- `path_start`: mask to filter matching starting path
- `per_page`: number of items to return for a given page (default 30, max 100)
- `starred_only`: (default false)
show only the starred items

- `with_deployment_msg`: (default false)
include deployment message

*/
    pub async fn list_apps<'a>(
        &'a self,
        workspace: &'a str,
        created_by: Option<&'a str>,
        include_draft_only: Option<bool>,
        order_desc: Option<bool>,
        page: Option<i64>,
        path_exact: Option<&'a str>,
        path_start: Option<&'a str>,
        per_page: Option<i64>,
        starred_only: Option<bool>,
        with_deployment_msg: Option<bool>,
    ) -> Result<ResponseValue<Vec<types::ListableApp>>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/list", self.baseurl, encode_path(& workspace.to_string()),
        );
        let mut query = Vec::with_capacity(9usize);
        if let Some(v) = &created_by {
            query.push(("created_by", v.to_string()));
        }
        if let Some(v) = &include_draft_only {
            query.push(("include_draft_only", v.to_string()));
        }
        if let Some(v) = &order_desc {
            query.push(("order_desc", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &path_exact {
            query.push(("path_exact", v.to_string()));
        }
        if let Some(v) = &path_start {
            query.push(("path_start", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
        }
        if let Some(v) = &starred_only {
            query.push(("starred_only", v.to_string()));
        }
        if let Some(v) = &with_deployment_msg {
            query.push(("with_deployment_msg", v.to_string()));
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
    /**create app

Sends a `POST` request to `/w/{workspace}/apps/create`

Arguments:
- `workspace`
- `body`: new app
*/
    pub async fn create_app<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::CreateAppBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/create", self.baseurl, encode_path(& workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**does an app exisst at path

Sends a `GET` request to `/w/{workspace}/apps/exists/{path}`

*/
    pub async fn exists_app<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/exists/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& path.to_string()),
        );
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
    /**get app by path

Sends a `GET` request to `/w/{workspace}/apps/get/p/{path}`

*/
    pub async fn get_app_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        with_starred_info: Option<bool>,
    ) -> Result<ResponseValue<types::AppWithLastVersion>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/get/p/{}", self.baseurl, encode_path(& workspace.to_string()),
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
    /**get app lite by path

Sends a `GET` request to `/w/{workspace}/apps/get/lite/{path}`

*/
    pub async fn get_app_lite_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::AppWithLastVersion>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/get/lite/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**get app by path with draft

Sends a `GET` request to `/w/{workspace}/apps/get/draft/{path}`

*/
    pub async fn get_app_by_path_with_draft<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::AppWithLastVersionWDraft>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/get/draft/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**get app history by path

Sends a `GET` request to `/w/{workspace}/apps/history/p/{path}`

*/
    pub async fn get_app_history_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<Vec<types::AppHistory>>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/history/p/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**get apps's latest version

Sends a `GET` request to `/w/{workspace}/apps/get_latest_version/{path}`

*/
    pub async fn get_app_latest_version<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::AppHistory>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/get_latest_version/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**update app history

Sends a `POST` request to `/w/{workspace}/apps/history_update/a/{id}/v/{version}`

Arguments:
- `workspace`
- `id`
- `version`
- `body`: App deployment message
*/
    pub async fn update_app_history<'a>(
        &'a self,
        workspace: &'a str,
        id: i64,
        version: i64,
        body: &'a types::UpdateAppHistoryBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/history_update/a/{}/v/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& id.to_string()), encode_path(& version
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get public app by secret

Sends a `GET` request to `/w/{workspace}/apps_u/public_app/{path}`

*/
    pub async fn get_public_app_by_secret<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::AppWithLastVersion>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps_u/public_app/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**get public resource

Sends a `GET` request to `/w/{workspace}/apps_u/public_resource/{path}`

*/
    pub async fn get_public_resource<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps_u/public_resource/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**get public secret of app

Sends a `GET` request to `/w/{workspace}/apps/secret_of/{path}`

*/
    pub async fn get_public_secret_of_app<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/secret_of/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get app by version

Sends a `GET` request to `/w/{workspace}/apps/get/v/{id}`

*/
    pub async fn get_app_by_version<'a>(
        &'a self,
        workspace: &'a str,
        id: i64,
    ) -> Result<ResponseValue<types::AppWithLastVersion>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/get/v/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& id.to_string()),
        );
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
    /**create raw app

Sends a `POST` request to `/w/{workspace}/raw_apps/create`

Arguments:
- `workspace`
- `body`: new raw app
*/
    pub async fn create_raw_app<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::CreateRawAppBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/raw_apps/create", self.baseurl, encode_path(& workspace
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
    /**update app

Sends a `POST` request to `/w/{workspace}/raw_apps/update/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updateraw  app
*/
    pub async fn update_raw_app<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::UpdateRawAppBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/raw_apps/update/{}", self.baseurl, encode_path(& workspace
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
    /**delete raw app

Sends a `DELETE` request to `/w/{workspace}/raw_apps/delete/{path}`

*/
    pub async fn delete_raw_app<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/raw_apps/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**delete app

Sends a `DELETE` request to `/w/{workspace}/apps/delete/{path}`

*/
    pub async fn delete_app<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/delete/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& path.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**update app

Sends a `POST` request to `/w/{workspace}/apps/update/{path}`

Arguments:
- `workspace`
- `path`
- `body`: update app
*/
    pub async fn update_app<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::UpdateAppBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/update/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& path.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**check if custom path exists

Sends a `GET` request to `/w/{workspace}/apps/custom_path_exists/{custom_path}`

*/
    pub async fn custom_path_exists<'a>(
        &'a self,
        workspace: &'a str,
        custom_path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps/custom_path_exists/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& custom_path.to_string()),
        );
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
    /**executeComponent

Sends a `POST` request to `/w/{workspace}/apps_u/execute_component/{path}`

Arguments:
- `workspace`
- `path`
- `body`: update app
*/
    pub async fn execute_component<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::ExecuteComponentBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps_u/execute_component/{}", self.baseurl, encode_path(& workspace
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
    /**upload s3 file from app

Sends a `POST` request to `/w/{workspace}/apps_u/upload_s3_file/{path}`

Arguments:
- `workspace`
- `path`
- `content_disposition`
- `content_type`
- `file_extension`
- `file_key`
- `resource_type`
- `s3_resource_path`
- `storage`
- `body`: File content
*/
    pub async fn upload_s3_file_from_app<'a, B: Into<reqwest::Body>>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        content_disposition: Option<&'a str>,
        content_type: Option<&'a str>,
        file_extension: Option<&'a str>,
        file_key: Option<&'a str>,
        resource_type: Option<&'a str>,
        s3_resource_path: Option<&'a str>,
        storage: Option<&'a str>,
        body: B,
    ) -> Result<ResponseValue<types::UploadS3FileFromAppResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps_u/upload_s3_file/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(7usize);
        if let Some(v) = &content_disposition {
            query.push(("content_disposition", v.to_string()));
        }
        if let Some(v) = &content_type {
            query.push(("content_type", v.to_string()));
        }
        if let Some(v) = &file_extension {
            query.push(("file_extension", v.to_string()));
        }
        if let Some(v) = &file_key {
            query.push(("file_key", v.to_string()));
        }
        if let Some(v) = &resource_type {
            query.push(("resource_type", v.to_string()));
        }
        if let Some(v) = &s3_resource_path {
            query.push(("s3_resource_path", v.to_string()));
        }
        if let Some(v) = &storage {
            query.push(("storage", v.to_string()));
        }
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .header(
                reqwest::header::CONTENT_TYPE,
                reqwest::header::HeaderValue::from_static("application/octet-stream"),
            )
            .body(body)
            .query(&query)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**delete s3 file from app

Sends a `DELETE` request to `/w/{workspace}/apps_u/delete_s3_file`

*/
    pub async fn delete_s3_file_from_app<'a>(
        &'a self,
        workspace: &'a str,
        delete_token: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/apps_u/delete_s3_file", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        query.push(("delete_token", delete_token.to_string()));
        let request = self.client.delete(url).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**run flow by path

Sends a `POST` request to `/w/{workspace}/jobs/run/f/{path}`

Arguments:
- `workspace`
- `path`
- `include_header`: List of headers's keys (separated with ',') whove value are added to the args
Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key

- `invisible_to_owner`: make the run invisible to the the flow owner (default false)
- `job_id`: The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
- `parent_job`: The parent job that is at the origin and responsible for the execution of this script if any
- `scheduled_for`: when to schedule this job (leave empty for immediate run)
- `scheduled_in_secs`: schedule the script to execute in the number of seconds starting now
- `skip_preprocessor`: skip the preprocessor
- `tag`: Override the tag to use
- `body`: flow args
*/
    pub async fn run_flow_by_path<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        include_header: Option<&'a str>,
        invisible_to_owner: Option<bool>,
        job_id: Option<&'a uuid::Uuid>,
        parent_job: Option<&'a uuid::Uuid>,
        scheduled_for: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        scheduled_in_secs: Option<i64>,
        skip_preprocessor: Option<bool>,
        tag: Option<&'a str>,
        body: &'a types::ScriptArgs,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/run/f/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(8usize);
        if let Some(v) = &include_header {
            query.push(("include_header", v.to_string()));
        }
        if let Some(v) = &invisible_to_owner {
            query.push(("invisible_to_owner", v.to_string()));
        }
        if let Some(v) = &job_id {
            query.push(("job_id", v.to_string()));
        }
        if let Some(v) = &parent_job {
            query.push(("parent_job", v.to_string()));
        }
        if let Some(v) = &scheduled_for {
            query.push(("scheduled_for", v.to_string()));
        }
        if let Some(v) = &scheduled_in_secs {
            query.push(("scheduled_in_secs", v.to_string()));
        }
        if let Some(v) = &skip_preprocessor {
            query.push(("skip_preprocessor", v.to_string()));
        }
        if let Some(v) = &tag {
            query.push(("tag", v.to_string()));
        }
        let request = self.client.post(url).json(&body).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**restart a completed flow at a given step

Sends a `POST` request to `/w/{workspace}/jobs/restart/f/{id}/from/{step_id}/{branch_or_iteration_n}`

Arguments:
- `workspace`
- `id`
- `step_id`: step id to restart the flow from
- `branch_or_iteration_n`: for branchall or loop, the iteration at which the flow should restart
- `include_header`: List of headers's keys (separated with ',') whove value are added to the args
Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key

- `invisible_to_owner`: make the run invisible to the the flow owner (default false)
- `job_id`: The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
- `parent_job`: The parent job that is at the origin and responsible for the execution of this script if any
- `scheduled_for`: when to schedule this job (leave empty for immediate run)
- `scheduled_in_secs`: schedule the script to execute in the number of seconds starting now
- `tag`: Override the tag to use
- `body`: flow args
*/
    pub async fn restart_flow_at_step<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        step_id: &'a str,
        branch_or_iteration_n: i64,
        include_header: Option<&'a str>,
        invisible_to_owner: Option<bool>,
        job_id: Option<&'a uuid::Uuid>,
        parent_job: Option<&'a uuid::Uuid>,
        scheduled_for: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        scheduled_in_secs: Option<i64>,
        tag: Option<&'a str>,
        body: &'a types::ScriptArgs,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/restart/f/{}/from/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()), encode_path(& step_id
            .to_string()), encode_path(& branch_or_iteration_n.to_string()),
        );
        let mut query = Vec::with_capacity(7usize);
        if let Some(v) = &include_header {
            query.push(("include_header", v.to_string()));
        }
        if let Some(v) = &invisible_to_owner {
            query.push(("invisible_to_owner", v.to_string()));
        }
        if let Some(v) = &job_id {
            query.push(("job_id", v.to_string()));
        }
        if let Some(v) = &parent_job {
            query.push(("parent_job", v.to_string()));
        }
        if let Some(v) = &scheduled_for {
            query.push(("scheduled_for", v.to_string()));
        }
        if let Some(v) = &scheduled_in_secs {
            query.push(("scheduled_in_secs", v.to_string()));
        }
        if let Some(v) = &tag {
            query.push(("tag", v.to_string()));
        }
        let request = self.client.post(url).json(&body).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**run script by hash

Sends a `POST` request to `/w/{workspace}/jobs/run/h/{hash}`

Arguments:
- `workspace`
- `hash`
- `cache_ttl`: Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
- `include_header`: List of headers's keys (separated with ',') whove value are added to the args
Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key

- `invisible_to_owner`: make the run invisible to the the script owner (default false)
- `job_id`: The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
- `parent_job`: The parent job that is at the origin and responsible for the execution of this script if any
- `scheduled_for`: when to schedule this job (leave empty for immediate run)
- `scheduled_in_secs`: schedule the script to execute in the number of seconds starting now
- `skip_preprocessor`: skip the preprocessor
- `tag`: Override the tag to use
- `body`: Partially filled args
*/
    pub async fn run_script_by_hash<'a>(
        &'a self,
        workspace: &'a str,
        hash: &'a str,
        cache_ttl: Option<&'a str>,
        include_header: Option<&'a str>,
        invisible_to_owner: Option<bool>,
        job_id: Option<&'a uuid::Uuid>,
        parent_job: Option<&'a uuid::Uuid>,
        scheduled_for: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        scheduled_in_secs: Option<i64>,
        skip_preprocessor: Option<bool>,
        tag: Option<&'a str>,
        body: &'a std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/run/h/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& hash.to_string()),
        );
        let mut query = Vec::with_capacity(9usize);
        if let Some(v) = &cache_ttl {
            query.push(("cache_ttl", v.to_string()));
        }
        if let Some(v) = &include_header {
            query.push(("include_header", v.to_string()));
        }
        if let Some(v) = &invisible_to_owner {
            query.push(("invisible_to_owner", v.to_string()));
        }
        if let Some(v) = &job_id {
            query.push(("job_id", v.to_string()));
        }
        if let Some(v) = &parent_job {
            query.push(("parent_job", v.to_string()));
        }
        if let Some(v) = &scheduled_for {
            query.push(("scheduled_for", v.to_string()));
        }
        if let Some(v) = &scheduled_in_secs {
            query.push(("scheduled_in_secs", v.to_string()));
        }
        if let Some(v) = &skip_preprocessor {
            query.push(("skip_preprocessor", v.to_string()));
        }
        if let Some(v) = &tag {
            query.push(("tag", v.to_string()));
        }
        let request = self.client.post(url).json(&body).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**run script preview

Sends a `POST` request to `/w/{workspace}/jobs/run/preview`

Arguments:
- `workspace`
- `include_header`: List of headers's keys (separated with ',') whove value are added to the args
Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key

- `invisible_to_owner`: make the run invisible to the the script owner (default false)
- `job_id`: The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
- `body`: preview
*/
    pub async fn run_script_preview<'a>(
        &'a self,
        workspace: &'a str,
        include_header: Option<&'a str>,
        invisible_to_owner: Option<bool>,
        job_id: Option<&'a uuid::Uuid>,
        body: &'a types::Preview,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/run/preview", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &include_header {
            query.push(("include_header", v.to_string()));
        }
        if let Some(v) = &invisible_to_owner {
            query.push(("invisible_to_owner", v.to_string()));
        }
        if let Some(v) = &job_id {
            query.push(("job_id", v.to_string()));
        }
        let request = self.client.post(url).json(&body).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**run code-workflow task

Sends a `POST` request to `/w/{workspace}/jobs/workflow_as_code/{job_id}/{entrypoint}`

Arguments:
- `workspace`
- `job_id`
- `entrypoint`
- `body`: preview
*/
    pub async fn run_code_workflow_task<'a>(
        &'a self,
        workspace: &'a str,
        job_id: &'a str,
        entrypoint: &'a str,
        body: &'a types::WorkflowTask,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/workflow_as_code/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& job_id.to_string()), encode_path(& entrypoint
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
    /**run a one-off dependencies job

Sends a `POST` request to `/w/{workspace}/jobs/run/dependencies`

Arguments:
- `workspace`
- `body`: raw script content
*/
    pub async fn run_raw_script_dependencies<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::RunRawScriptDependenciesBody,
    ) -> Result<ResponseValue<types::RunRawScriptDependenciesResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/run/dependencies", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**run flow preview

Sends a `POST` request to `/w/{workspace}/jobs/run/preview_flow`

Arguments:
- `workspace`
- `include_header`: List of headers's keys (separated with ',') whove value are added to the args
Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key

- `invisible_to_owner`: make the run invisible to the the script owner (default false)
- `job_id`: The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
- `body`: preview
*/
    pub async fn run_flow_preview<'a>(
        &'a self,
        workspace: &'a str,
        include_header: Option<&'a str>,
        invisible_to_owner: Option<bool>,
        job_id: Option<&'a uuid::Uuid>,
        body: &'a types::FlowPreview,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/run/preview_flow", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &include_header {
            query.push(("include_header", v.to_string()));
        }
        if let Some(v) = &invisible_to_owner {
            query.push(("invisible_to_owner", v.to_string()));
        }
        if let Some(v) = &job_id {
            query.push(("job_id", v.to_string()));
        }
        let request = self.client.post(url).json(&body).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list all queued jobs

Sends a `GET` request to `/w/{workspace}/jobs/queue/list`

Arguments:
- `workspace`
- `all_workspaces`: get jobs from all workspaces (only valid if request come from the `admins` workspace)
- `args`: filter on jobs containing those args as a json subset (@> in postgres)
- `created_by`: mask to filter exact matching user creator
- `is_not_schedule`: is not a scheduled job
- `job_kinds`: filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
- `order_desc`: order by desc order (default true)
- `page`: which page to return (start at 1, default 1)
- `parent_job`: The parent job that is at the origin and responsible for the execution of this script if any
- `per_page`: number of items to return for a given page (default 30, max 100)
- `result`: filter on jobs containing those result as a json subset (@> in postgres)
- `running`: filter on running jobs
- `schedule_path`: mask to filter by schedule path
- `scheduled_for_before_now`: filter on jobs scheduled_for before now (hence waitinf for a worker)
- `script_hash`: mask to filter exact matching path
- `script_path_exact`: mask to filter exact matching path
- `script_path_start`: mask to filter matching starting path
- `started_after`: filter on started after (exclusive) timestamp
- `started_before`: filter on started before (inclusive) timestamp
- `success`: filter on successful jobs
- `suspended`: filter on suspended jobs
- `tag`: filter on jobs with a given tag/worker group
- `worker`: worker this job was ran on
*/
    pub async fn list_queue<'a>(
        &'a self,
        workspace: &'a str,
        all_workspaces: Option<bool>,
        args: Option<&'a str>,
        created_by: Option<&'a str>,
        is_not_schedule: Option<bool>,
        job_kinds: Option<&'a str>,
        order_desc: Option<bool>,
        page: Option<i64>,
        parent_job: Option<&'a uuid::Uuid>,
        per_page: Option<i64>,
        result: Option<&'a str>,
        running: Option<bool>,
        schedule_path: Option<&'a str>,
        scheduled_for_before_now: Option<bool>,
        script_hash: Option<&'a str>,
        script_path_exact: Option<&'a str>,
        script_path_start: Option<&'a str>,
        started_after: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        started_before: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        success: Option<bool>,
        suspended: Option<bool>,
        tag: Option<&'a str>,
        worker: Option<&'a str>,
    ) -> Result<ResponseValue<Vec<types::QueuedJob>>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/queue/list", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(22usize);
        if let Some(v) = &all_workspaces {
            query.push(("all_workspaces", v.to_string()));
        }
        if let Some(v) = &args {
            query.push(("args", v.to_string()));
        }
        if let Some(v) = &created_by {
            query.push(("created_by", v.to_string()));
        }
        if let Some(v) = &is_not_schedule {
            query.push(("is_not_schedule", v.to_string()));
        }
        if let Some(v) = &job_kinds {
            query.push(("job_kinds", v.to_string()));
        }
        if let Some(v) = &order_desc {
            query.push(("order_desc", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &parent_job {
            query.push(("parent_job", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
        }
        if let Some(v) = &result {
            query.push(("result", v.to_string()));
        }
        if let Some(v) = &running {
            query.push(("running", v.to_string()));
        }
        if let Some(v) = &schedule_path {
            query.push(("schedule_path", v.to_string()));
        }
        if let Some(v) = &scheduled_for_before_now {
            query.push(("scheduled_for_before_now", v.to_string()));
        }
        if let Some(v) = &script_hash {
            query.push(("script_hash", v.to_string()));
        }
        if let Some(v) = &script_path_exact {
            query.push(("script_path_exact", v.to_string()));
        }
        if let Some(v) = &script_path_start {
            query.push(("script_path_start", v.to_string()));
        }
        if let Some(v) = &started_after {
            query.push(("started_after", v.to_string()));
        }
        if let Some(v) = &started_before {
            query.push(("started_before", v.to_string()));
        }
        if let Some(v) = &success {
            query.push(("success", v.to_string()));
        }
        if let Some(v) = &suspended {
            query.push(("suspended", v.to_string()));
        }
        if let Some(v) = &tag {
            query.push(("tag", v.to_string()));
        }
        if let Some(v) = &worker {
            query.push(("worker", v.to_string()));
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
    /**get queue count

Sends a `GET` request to `/w/{workspace}/jobs/queue/count`

Arguments:
- `workspace`
- `all_workspaces`: get jobs from all workspaces (only valid if request come from the `admins` workspace)
*/
    pub async fn get_queue_count<'a>(
        &'a self,
        workspace: &'a str,
        all_workspaces: Option<bool>,
    ) -> Result<ResponseValue<types::GetQueueCountResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/queue/count", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &all_workspaces {
            query.push(("all_workspaces", v.to_string()));
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
    /**get completed count

Sends a `GET` request to `/w/{workspace}/jobs/completed/count`

*/
    pub async fn get_completed_count<'a>(
        &'a self,
        workspace: &'a str,
    ) -> Result<ResponseValue<types::GetCompletedCountResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/completed/count", self.baseurl, encode_path(& workspace
            .to_string()),
        );
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
    /**count number of completed jobs with filter

Sends a `GET` request to `/w/{workspace}/jobs/completed/count_jobs`

*/
    pub async fn count_completed_jobs<'a>(
        &'a self,
        workspace: &'a str,
        all_workspaces: Option<bool>,
        completed_after_s_ago: Option<i64>,
        success: Option<bool>,
        tags: Option<&'a str>,
    ) -> Result<ResponseValue<i64>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/completed/count_jobs", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(4usize);
        if let Some(v) = &all_workspaces {
            query.push(("all_workspaces", v.to_string()));
        }
        if let Some(v) = &completed_after_s_ago {
            query.push(("completed_after_s_ago", v.to_string()));
        }
        if let Some(v) = &success {
            query.push(("success", v.to_string()));
        }
        if let Some(v) = &tags {
            query.push(("tags", v.to_string()));
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
    /**get the ids of all jobs matching the given filters

Sends a `GET` request to `/w/{workspace}/jobs/queue/list_filtered_uuids`

Arguments:
- `workspace`
- `all_workspaces`: get jobs from all workspaces (only valid if request come from the `admins` workspace)
- `args`: filter on jobs containing those args as a json subset (@> in postgres)
- `concurrency_key`
- `created_by`: mask to filter exact matching user creator
- `is_not_schedule`: is not a scheduled job
- `job_kinds`: filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
- `order_desc`: order by desc order (default true)
- `page`: which page to return (start at 1, default 1)
- `parent_job`: The parent job that is at the origin and responsible for the execution of this script if any
- `per_page`: number of items to return for a given page (default 30, max 100)
- `result`: filter on jobs containing those result as a json subset (@> in postgres)
- `running`: filter on running jobs
- `schedule_path`: mask to filter by schedule path
- `scheduled_for_before_now`: filter on jobs scheduled_for before now (hence waitinf for a worker)
- `script_hash`: mask to filter exact matching path
- `script_path_exact`: mask to filter exact matching path
- `script_path_start`: mask to filter matching starting path
- `started_after`: filter on started after (exclusive) timestamp
- `started_before`: filter on started before (inclusive) timestamp
- `success`: filter on successful jobs
- `suspended`: filter on suspended jobs
- `tag`: filter on jobs with a given tag/worker group
*/
    pub async fn list_filtered_uuids<'a>(
        &'a self,
        workspace: &'a str,
        all_workspaces: Option<bool>,
        args: Option<&'a str>,
        concurrency_key: Option<&'a str>,
        created_by: Option<&'a str>,
        is_not_schedule: Option<bool>,
        job_kinds: Option<&'a str>,
        order_desc: Option<bool>,
        page: Option<i64>,
        parent_job: Option<&'a uuid::Uuid>,
        per_page: Option<i64>,
        result: Option<&'a str>,
        running: Option<bool>,
        schedule_path: Option<&'a str>,
        scheduled_for_before_now: Option<bool>,
        script_hash: Option<&'a str>,
        script_path_exact: Option<&'a str>,
        script_path_start: Option<&'a str>,
        started_after: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        started_before: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        success: Option<bool>,
        suspended: Option<bool>,
        tag: Option<&'a str>,
    ) -> Result<ResponseValue<Vec<String>>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/queue/list_filtered_uuids", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let mut query = Vec::with_capacity(22usize);
        if let Some(v) = &all_workspaces {
            query.push(("all_workspaces", v.to_string()));
        }
        if let Some(v) = &args {
            query.push(("args", v.to_string()));
        }
        if let Some(v) = &concurrency_key {
            query.push(("concurrency_key", v.to_string()));
        }
        if let Some(v) = &created_by {
            query.push(("created_by", v.to_string()));
        }
        if let Some(v) = &is_not_schedule {
            query.push(("is_not_schedule", v.to_string()));
        }
        if let Some(v) = &job_kinds {
            query.push(("job_kinds", v.to_string()));
        }
        if let Some(v) = &order_desc {
            query.push(("order_desc", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &parent_job {
            query.push(("parent_job", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
        }
        if let Some(v) = &result {
            query.push(("result", v.to_string()));
        }
        if let Some(v) = &running {
            query.push(("running", v.to_string()));
        }
        if let Some(v) = &schedule_path {
            query.push(("schedule_path", v.to_string()));
        }
        if let Some(v) = &scheduled_for_before_now {
            query.push(("scheduled_for_before_now", v.to_string()));
        }
        if let Some(v) = &script_hash {
            query.push(("script_hash", v.to_string()));
        }
        if let Some(v) = &script_path_exact {
            query.push(("script_path_exact", v.to_string()));
        }
        if let Some(v) = &script_path_start {
            query.push(("script_path_start", v.to_string()));
        }
        if let Some(v) = &started_after {
            query.push(("started_after", v.to_string()));
        }
        if let Some(v) = &started_before {
            query.push(("started_before", v.to_string()));
        }
        if let Some(v) = &success {
            query.push(("success", v.to_string()));
        }
        if let Some(v) = &suspended {
            query.push(("suspended", v.to_string()));
        }
        if let Some(v) = &tag {
            query.push(("tag", v.to_string()));
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
    /**cancel jobs based on the given uuids

Sends a `POST` request to `/w/{workspace}/jobs/queue/cancel_selection`

Arguments:
- `workspace`
- `body`: uuids of the jobs to cancel
*/
    pub async fn cancel_selection<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a Vec<String>,
    ) -> Result<ResponseValue<Vec<String>>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/queue/cancel_selection", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list all completed jobs

Sends a `GET` request to `/w/{workspace}/jobs/completed/list`

Arguments:
- `workspace`
- `args`: filter on jobs containing those args as a json subset (@> in postgres)
- `created_by`: mask to filter exact matching user creator
- `has_null_parent`: has null parent
- `is_flow_step`: is the job a flow step
- `is_not_schedule`: is not a scheduled job
- `is_skipped`: is the job skipped
- `job_kinds`: filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
- `label`: mask to filter exact matching job's label (job labels are completed jobs with as a result an object containing a string in the array at key 'wm_labels')
- `order_desc`: order by desc order (default true)
- `page`: which page to return (start at 1, default 1)
- `parent_job`: The parent job that is at the origin and responsible for the execution of this script if any
- `per_page`: number of items to return for a given page (default 30, max 100)
- `result`: filter on jobs containing those result as a json subset (@> in postgres)
- `schedule_path`: mask to filter by schedule path
- `script_hash`: mask to filter exact matching path
- `script_path_exact`: mask to filter exact matching path
- `script_path_start`: mask to filter matching starting path
- `started_after`: filter on started after (exclusive) timestamp
- `started_before`: filter on started before (inclusive) timestamp
- `success`: filter on successful jobs
- `tag`: filter on jobs with a given tag/worker group
- `worker`: worker this job was ran on
*/
    pub async fn list_completed_jobs<'a>(
        &'a self,
        workspace: &'a str,
        args: Option<&'a str>,
        created_by: Option<&'a str>,
        has_null_parent: Option<bool>,
        is_flow_step: Option<bool>,
        is_not_schedule: Option<bool>,
        is_skipped: Option<bool>,
        job_kinds: Option<&'a str>,
        label: Option<&'a str>,
        order_desc: Option<bool>,
        page: Option<i64>,
        parent_job: Option<&'a uuid::Uuid>,
        per_page: Option<i64>,
        result: Option<&'a str>,
        schedule_path: Option<&'a str>,
        script_hash: Option<&'a str>,
        script_path_exact: Option<&'a str>,
        script_path_start: Option<&'a str>,
        started_after: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        started_before: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        success: Option<bool>,
        tag: Option<&'a str>,
        worker: Option<&'a str>,
    ) -> Result<ResponseValue<Vec<types::CompletedJob>>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/completed/list", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(22usize);
        if let Some(v) = &args {
            query.push(("args", v.to_string()));
        }
        if let Some(v) = &created_by {
            query.push(("created_by", v.to_string()));
        }
        if let Some(v) = &has_null_parent {
            query.push(("has_null_parent", v.to_string()));
        }
        if let Some(v) = &is_flow_step {
            query.push(("is_flow_step", v.to_string()));
        }
        if let Some(v) = &is_not_schedule {
            query.push(("is_not_schedule", v.to_string()));
        }
        if let Some(v) = &is_skipped {
            query.push(("is_skipped", v.to_string()));
        }
        if let Some(v) = &job_kinds {
            query.push(("job_kinds", v.to_string()));
        }
        if let Some(v) = &label {
            query.push(("label", v.to_string()));
        }
        if let Some(v) = &order_desc {
            query.push(("order_desc", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &parent_job {
            query.push(("parent_job", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
        }
        if let Some(v) = &result {
            query.push(("result", v.to_string()));
        }
        if let Some(v) = &schedule_path {
            query.push(("schedule_path", v.to_string()));
        }
        if let Some(v) = &script_hash {
            query.push(("script_hash", v.to_string()));
        }
        if let Some(v) = &script_path_exact {
            query.push(("script_path_exact", v.to_string()));
        }
        if let Some(v) = &script_path_start {
            query.push(("script_path_start", v.to_string()));
        }
        if let Some(v) = &started_after {
            query.push(("started_after", v.to_string()));
        }
        if let Some(v) = &started_before {
            query.push(("started_before", v.to_string()));
        }
        if let Some(v) = &success {
            query.push(("success", v.to_string()));
        }
        if let Some(v) = &tag {
            query.push(("tag", v.to_string()));
        }
        if let Some(v) = &worker {
            query.push(("worker", v.to_string()));
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
    /**list all jobs

Sends a `GET` request to `/w/{workspace}/jobs/list`

Arguments:
- `workspace`
- `all_workspaces`: get jobs from all workspaces (only valid if request come from the `admins` workspace)
- `args`: filter on jobs containing those args as a json subset (@> in postgres)
- `created_after`: filter on created after (exclusive) timestamp
- `created_before`: filter on created before (inclusive) timestamp
- `created_by`: mask to filter exact matching user creator
- `created_or_started_after`: filter on created_at for non non started job and started_at otherwise after (exclusive) timestamp
- `created_or_started_after_completed_jobs`: filter on created_at for non non started job and started_at otherwise after (exclusive) timestamp but only for the completed jobs
- `created_or_started_before`: filter on created_at for non non started job and started_at otherwise before (inclusive) timestamp
- `has_null_parent`: has null parent
- `is_flow_step`: is the job a flow step
- `is_not_schedule`: is not a scheduled job
- `is_skipped`: is the job skipped
- `job_kinds`: filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
- `label`: mask to filter exact matching job's label (job labels are completed jobs with as a result an object containing a string in the array at key 'wm_labels')
- `page`: which page to return (start at 1, default 1)
- `parent_job`: The parent job that is at the origin and responsible for the execution of this script if any
- `per_page`: number of items to return for a given page (default 30, max 100)
- `result`: filter on jobs containing those result as a json subset (@> in postgres)
- `running`: filter on running jobs
- `schedule_path`: mask to filter by schedule path
- `scheduled_for_before_now`: filter on jobs scheduled_for before now (hence waitinf for a worker)
- `script_hash`: mask to filter exact matching path
- `script_path_exact`: mask to filter exact matching path
- `script_path_start`: mask to filter matching starting path
- `started_after`: filter on started after (exclusive) timestamp
- `started_before`: filter on started before (inclusive) timestamp
- `success`: filter on successful jobs
- `suspended`: filter on suspended jobs
- `tag`: filter on jobs with a given tag/worker group
- `worker`: worker this job was ran on
*/
    pub async fn list_jobs<'a>(
        &'a self,
        workspace: &'a str,
        all_workspaces: Option<bool>,
        args: Option<&'a str>,
        created_after: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        created_before: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        created_by: Option<&'a str>,
        created_or_started_after: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        created_or_started_after_completed_jobs: Option<
            &'a chrono::DateTime<chrono::offset::Utc>,
        >,
        created_or_started_before: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        has_null_parent: Option<bool>,
        is_flow_step: Option<bool>,
        is_not_schedule: Option<bool>,
        is_skipped: Option<bool>,
        job_kinds: Option<&'a str>,
        label: Option<&'a str>,
        page: Option<i64>,
        parent_job: Option<&'a uuid::Uuid>,
        per_page: Option<i64>,
        result: Option<&'a str>,
        running: Option<bool>,
        schedule_path: Option<&'a str>,
        scheduled_for_before_now: Option<bool>,
        script_hash: Option<&'a str>,
        script_path_exact: Option<&'a str>,
        script_path_start: Option<&'a str>,
        started_after: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        started_before: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        success: Option<bool>,
        suspended: Option<bool>,
        tag: Option<&'a str>,
        worker: Option<&'a str>,
    ) -> Result<ResponseValue<Vec<types::Job>>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/list", self.baseurl, encode_path(& workspace.to_string()),
        );
        let mut query = Vec::with_capacity(30usize);
        if let Some(v) = &all_workspaces {
            query.push(("all_workspaces", v.to_string()));
        }
        if let Some(v) = &args {
            query.push(("args", v.to_string()));
        }
        if let Some(v) = &created_after {
            query.push(("created_after", v.to_string()));
        }
        if let Some(v) = &created_before {
            query.push(("created_before", v.to_string()));
        }
        if let Some(v) = &created_by {
            query.push(("created_by", v.to_string()));
        }
        if let Some(v) = &created_or_started_after {
            query.push(("created_or_started_after", v.to_string()));
        }
        if let Some(v) = &created_or_started_after_completed_jobs {
            query.push(("created_or_started_after_completed_jobs", v.to_string()));
        }
        if let Some(v) = &created_or_started_before {
            query.push(("created_or_started_before", v.to_string()));
        }
        if let Some(v) = &has_null_parent {
            query.push(("has_null_parent", v.to_string()));
        }
        if let Some(v) = &is_flow_step {
            query.push(("is_flow_step", v.to_string()));
        }
        if let Some(v) = &is_not_schedule {
            query.push(("is_not_schedule", v.to_string()));
        }
        if let Some(v) = &is_skipped {
            query.push(("is_skipped", v.to_string()));
        }
        if let Some(v) = &job_kinds {
            query.push(("job_kinds", v.to_string()));
        }
        if let Some(v) = &label {
            query.push(("label", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &parent_job {
            query.push(("parent_job", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
        }
        if let Some(v) = &result {
            query.push(("result", v.to_string()));
        }
        if let Some(v) = &running {
            query.push(("running", v.to_string()));
        }
        if let Some(v) = &schedule_path {
            query.push(("schedule_path", v.to_string()));
        }
        if let Some(v) = &scheduled_for_before_now {
            query.push(("scheduled_for_before_now", v.to_string()));
        }
        if let Some(v) = &script_hash {
            query.push(("script_hash", v.to_string()));
        }
        if let Some(v) = &script_path_exact {
            query.push(("script_path_exact", v.to_string()));
        }
        if let Some(v) = &script_path_start {
            query.push(("script_path_start", v.to_string()));
        }
        if let Some(v) = &started_after {
            query.push(("started_after", v.to_string()));
        }
        if let Some(v) = &started_before {
            query.push(("started_before", v.to_string()));
        }
        if let Some(v) = &success {
            query.push(("success", v.to_string()));
        }
        if let Some(v) = &suspended {
            query.push(("suspended", v.to_string()));
        }
        if let Some(v) = &tag {
            query.push(("tag", v.to_string()));
        }
        if let Some(v) = &worker {
            query.push(("worker", v.to_string()));
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
    /**get db clock

Sends a `GET` request to `/jobs/db_clock`

*/
    pub async fn get_db_clock<'a>(&'a self) -> Result<ResponseValue<i64>, Error<()>> {
        let url = format!("{}/jobs/db_clock", self.baseurl,);
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
    /**Count jobs by tag

Sends a `GET` request to `/jobs/completed/count_by_tag`

Arguments:
- `horizon_secs`: Past Time horizon in seconds (when to start the count = now - horizon) (default is 3600)
- `workspace_id`: Specific workspace ID to filter results (optional)
*/
    pub async fn count_jobs_by_tag<'a>(
        &'a self,
        horizon_secs: Option<i64>,
        workspace_id: Option<&'a str>,
    ) -> Result<ResponseValue<Vec<types::CountJobsByTagResponseItem>>, Error<()>> {
        let url = format!("{}/jobs/completed/count_by_tag", self.baseurl,);
        let mut query = Vec::with_capacity(2usize);
        if let Some(v) = &horizon_secs {
            query.push(("horizon_secs", v.to_string()));
        }
        if let Some(v) = &workspace_id {
            query.push(("workspace_id", v.to_string()));
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
    /**get job

Sends a `GET` request to `/w/{workspace}/jobs_u/get/{id}`

*/
    pub async fn get_job<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        no_logs: Option<bool>,
    ) -> Result<ResponseValue<types::Job>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/get/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& id.to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &no_logs {
            query.push(("no_logs", v.to_string()));
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
    /**get root job id

Sends a `GET` request to `/w/{workspace}/jobs_u/get_root_job_id/{id}`

*/
    pub async fn get_root_job_id<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
    ) -> Result<ResponseValue<String>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/get_root_job_id/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()),
        );
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
    /**get job logs

Sends a `GET` request to `/w/{workspace}/jobs_u/get_logs/{id}`

*/
    pub async fn get_job_logs<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/get_logs/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get job args

Sends a `GET` request to `/w/{workspace}/jobs_u/get_args/{id}`

*/
    pub async fn get_job_args<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/get_args/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()),
        );
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
    /**get job updates

Sends a `GET` request to `/w/{workspace}/jobs_u/getupdate/{id}`

*/
    pub async fn get_job_updates<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        get_progress: Option<bool>,
        log_offset: Option<i64>,
        running: Option<bool>,
    ) -> Result<ResponseValue<types::GetJobUpdatesResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/getupdate/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()),
        );
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &get_progress {
            query.push(("get_progress", v.to_string()));
        }
        if let Some(v) = &log_offset {
            query.push(("log_offset", v.to_string()));
        }
        if let Some(v) = &running {
            query.push(("running", v.to_string()));
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
    /**get log file from object store

Sends a `GET` request to `/w/{workspace}/jobs_u/get_log_file/{path}`

*/
    pub async fn get_log_file_from_store<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/get_log_file/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get flow debug info

Sends a `GET` request to `/w/{workspace}/jobs_u/get_flow_debug_info/{id}`

*/
    pub async fn get_flow_debug_info<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/get_flow_debug_info/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& id.to_string()),
        );
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
    /**get completed job

Sends a `GET` request to `/w/{workspace}/jobs_u/completed/get/{id}`

*/
    pub async fn get_completed_job<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
    ) -> Result<ResponseValue<types::CompletedJob>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/completed/get/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()),
        );
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
    /**get completed job result

Sends a `GET` request to `/w/{workspace}/jobs_u/completed/get_result/{id}`

*/
    pub async fn get_completed_job_result<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        approver: Option<&'a str>,
        resume_id: Option<i64>,
        secret: Option<&'a str>,
        suspended_job: Option<&'a str>,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/completed/get_result/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& id.to_string()),
        );
        let mut query = Vec::with_capacity(4usize);
        if let Some(v) = &approver {
            query.push(("approver", v.to_string()));
        }
        if let Some(v) = &resume_id {
            query.push(("resume_id", v.to_string()));
        }
        if let Some(v) = &secret {
            query.push(("secret", v.to_string()));
        }
        if let Some(v) = &suspended_job {
            query.push(("suspended_job", v.to_string()));
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
    /**get completed job result if job is completed

Sends a `GET` request to `/w/{workspace}/jobs_u/completed/get_result_maybe/{id}`

*/
    pub async fn get_completed_job_result_maybe<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        get_started: Option<bool>,
    ) -> Result<ResponseValue<types::GetCompletedJobResultMaybeResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/completed/get_result_maybe/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& id.to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &get_started {
            query.push(("get_started", v.to_string()));
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
    /**delete completed job (erase content but keep run id)

Sends a `POST` request to `/w/{workspace}/jobs/completed/delete/{id}`

*/
    pub async fn delete_completed_job<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
    ) -> Result<ResponseValue<types::CompletedJob>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/completed/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()),
        );
        let request = self
            .client
            .post(url)
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
    /**cancel queued or running job

Sends a `POST` request to `/w/{workspace}/jobs_u/queue/cancel/{id}`

Arguments:
- `workspace`
- `id`
- `body`: reason
*/
    pub async fn cancel_queued_job<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        body: &'a types::CancelQueuedJobBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/queue/cancel/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**cancel all queued jobs for persistent script

Sends a `POST` request to `/w/{workspace}/jobs_u/queue/cancel_persistent/{path}`

Arguments:
- `workspace`
- `path`
- `body`: reason
*/
    pub async fn cancel_persistent_queued_jobs<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::CancelPersistentQueuedJobsBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/queue/cancel_persistent/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**force cancel queued job

Sends a `POST` request to `/w/{workspace}/jobs_u/queue/force_cancel/{id}`

Arguments:
- `workspace`
- `id`
- `body`: reason
*/
    pub async fn force_cancel_queued_job<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        body: &'a types::ForceCancelQueuedJobBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/queue/force_cancel/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create an HMac signature given a job id and a resume id

Sends a `GET` request to `/w/{workspace}/jobs/job_signature/{id}/{resume_id}`

*/
    pub async fn create_job_signature<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        resume_id: i64,
        approver: Option<&'a str>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/job_signature/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()), encode_path(& resume_id
            .to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &approver {
            query.push(("approver", v.to_string()));
        }
        let request = self.client.get(url).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get resume urls given a job_id, resume_id and a nonce to resume a flow

Sends a `GET` request to `/w/{workspace}/jobs/resume_urls/{id}/{resume_id}`

*/
    pub async fn get_resume_urls<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        resume_id: i64,
        approver: Option<&'a str>,
    ) -> Result<ResponseValue<types::GetResumeUrlsResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/resume_urls/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()), encode_path(& resume_id
            .to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &approver {
            query.push(("approver", v.to_string()));
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
    /**generate interactive slack approval for suspended job

Sends a `GET` request to `/w/{workspace}/jobs/slack_approval/{id}`

*/
    pub async fn get_slack_approval_payload<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        approver: Option<&'a str>,
        channel_id: &'a str,
        default_args_json: Option<&'a str>,
        dynamic_enums_json: Option<&'a str>,
        flow_step_id: &'a str,
        message: Option<&'a str>,
        slack_resource_path: &'a str,
    ) -> Result<ResponseValue<()>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/slack_approval/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()),
        );
        let mut query = Vec::with_capacity(7usize);
        if let Some(v) = &approver {
            query.push(("approver", v.to_string()));
        }
        query.push(("channel_id", channel_id.to_string()));
        if let Some(v) = &default_args_json {
            query.push(("default_args_json", v.to_string()));
        }
        if let Some(v) = &dynamic_enums_json {
            query.push(("dynamic_enums_json", v.to_string()));
        }
        query.push(("flow_step_id", flow_step_id.to_string()));
        if let Some(v) = &message {
            query.push(("message", v.to_string()));
        }
        query.push(("slack_resource_path", slack_resource_path.to_string()));
        let request = self.client.get(url).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::empty(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**resume a job for a suspended flow

Sends a `GET` request to `/w/{workspace}/jobs_u/resume/{id}/{resume_id}/{signature}`

Arguments:
- `workspace`
- `id`
- `resume_id`
- `signature`
- `approver`
- `payload`: The base64 encoded payload that has been encoded as a JSON. e.g how to encode such payload encodeURIComponent
`encodeURIComponent(btoa(JSON.stringify({a: 2})))`

*/
    pub async fn resume_suspended_job_get<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        resume_id: i64,
        signature: &'a str,
        approver: Option<&'a str>,
        payload: Option<&'a str>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/resume/{}/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()), encode_path(& resume_id
            .to_string()), encode_path(& signature.to_string()),
        );
        let mut query = Vec::with_capacity(2usize);
        if let Some(v) = &approver {
            query.push(("approver", v.to_string()));
        }
        if let Some(v) = &payload {
            query.push(("payload", v.to_string()));
        }
        let request = self.client.get(url).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**resume a job for a suspended flow

Sends a `POST` request to `/w/{workspace}/jobs_u/resume/{id}/{resume_id}/{signature}`

*/
    pub async fn resume_suspended_job_post<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        resume_id: i64,
        signature: &'a str,
        approver: Option<&'a str>,
        body: &'a std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/resume/{}/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()), encode_path(& resume_id
            .to_string()), encode_path(& signature.to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &approver {
            query.push(("approver", v.to_string()));
        }
        let request = self.client.post(url).json(&body).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get flow user state at a given key

Sends a `GET` request to `/w/{workspace}/jobs/flow/user_states/{id}/{key}`

*/
    pub async fn get_flow_user_state<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        key: &'a str,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/flow/user_states/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()), encode_path(& key.to_string()),
        );
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
    /**set flow user state at a given key

Sends a `POST` request to `/w/{workspace}/jobs/flow/user_states/{id}/{key}`

Arguments:
- `workspace`
- `id`
- `key`
- `body`: new value
*/
    pub async fn set_flow_user_state<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        key: &'a str,
        body: &'a serde_json::Value,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/flow/user_states/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()), encode_path(& key.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**resume a job for a suspended flow as an owner

Sends a `POST` request to `/w/{workspace}/jobs/flow/resume/{id}`

*/
    pub async fn resume_suspended_flow_as_owner<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        body: &'a std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs/flow/resume/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**cancel a job for a suspended flow

Sends a `GET` request to `/w/{workspace}/jobs_u/cancel/{id}/{resume_id}/{signature}`

*/
    pub async fn cancel_suspended_job_get<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        resume_id: i64,
        signature: &'a str,
        approver: Option<&'a str>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/cancel/{}/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()), encode_path(& resume_id
            .to_string()), encode_path(& signature.to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &approver {
            query.push(("approver", v.to_string()));
        }
        let request = self.client.get(url).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**cancel a job for a suspended flow

Sends a `POST` request to `/w/{workspace}/jobs_u/cancel/{id}/{resume_id}/{signature}`

*/
    pub async fn cancel_suspended_job_post<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        resume_id: i64,
        signature: &'a str,
        approver: Option<&'a str>,
        body: &'a std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/cancel/{}/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()), encode_path(& resume_id
            .to_string()), encode_path(& signature.to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &approver {
            query.push(("approver", v.to_string()));
        }
        let request = self.client.post(url).json(&body).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get parent flow job of suspended job

Sends a `GET` request to `/w/{workspace}/jobs_u/get_flow/{id}/{resume_id}/{signature}`

*/
    pub async fn get_suspended_job_flow<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        resume_id: i64,
        signature: &'a str,
        approver: Option<&'a str>,
    ) -> Result<ResponseValue<types::GetSuspendedJobFlowResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/jobs_u/get_flow/{}/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()), encode_path(& resume_id
            .to_string()), encode_path(& signature.to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &approver {
            query.push(("approver", v.to_string()));
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
    /**preview schedule

Sends a `POST` request to `/schedules/preview`

Arguments:
- `body`: schedule
*/
    pub async fn preview_schedule<'a>(
        &'a self,
        body: &'a types::PreviewScheduleBody,
    ) -> Result<ResponseValue<Vec<chrono::DateTime<chrono::offset::Utc>>>, Error<()>> {
        let url = format!("{}/schedules/preview", self.baseurl,);
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
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
    /**set enabled schedule

Sends a `POST` request to `/w/{workspace}/schedules/setenabled/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated schedule enable
*/
    pub async fn set_schedule_enabled<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::SetScheduleEnabledBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/schedules/setenabled/{}", self.baseurl, encode_path(& workspace
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
    /**delete schedule

Sends a `DELETE` request to `/w/{workspace}/schedules/delete/{path}`

*/
    pub async fn delete_schedule<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/schedules/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get schedule

Sends a `GET` request to `/w/{workspace}/schedules/get/{path}`

*/
    pub async fn get_schedule<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::Schedule>, Error<()>> {
        let url = format!(
            "{}/w/{}/schedules/get/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**does schedule exists

Sends a `GET` request to `/w/{workspace}/schedules/exists/{path}`

*/
    pub async fn exists_schedule<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/schedules/exists/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**list schedules

Sends a `GET` request to `/w/{workspace}/schedules/list`

Arguments:
- `workspace`
- `args`: filter on jobs containing those args as a json subset (@> in postgres)
- `is_flow`
- `page`: which page to return (start at 1, default 1)
- `path`: filter by path
- `path_start`
- `per_page`: number of items to return for a given page (default 30, max 100)
*/
    pub async fn list_schedules<'a>(
        &'a self,
        workspace: &'a str,
        args: Option<&'a str>,
        is_flow: Option<bool>,
        page: Option<i64>,
        path: Option<&'a str>,
        path_start: Option<&'a str>,
        per_page: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::Schedule>>, Error<()>> {
        let url = format!(
            "{}/w/{}/schedules/list", self.baseurl, encode_path(& workspace.to_string()),
        );
        let mut query = Vec::with_capacity(6usize);
        if let Some(v) = &args {
            query.push(("args", v.to_string()));
        }
        if let Some(v) = &is_flow {
            query.push(("is_flow", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &path {
            query.push(("path", v.to_string()));
        }
        if let Some(v) = &path_start {
            query.push(("path_start", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
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
    /**list schedules with last 20 jobs

Sends a `GET` request to `/w/{workspace}/schedules/list_with_jobs`

Arguments:
- `workspace`
- `page`: which page to return (start at 1, default 1)
- `per_page`: number of items to return for a given page (default 30, max 100)
*/
    pub async fn list_schedules_with_jobs<'a>(
        &'a self,
        workspace: &'a str,
        page: Option<i64>,
        per_page: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::ScheduleWJobs>>, Error<()>> {
        let url = format!(
            "{}/w/{}/schedules/list_with_jobs", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(2usize);
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
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
    /**Set default error or recoevery handler

Sends a `POST` request to `/w/{workspace}/schedules/setdefaulthandler`

Arguments:
- `workspace`
- `body`: Handler description
*/
    pub async fn set_default_error_or_recovery_handler<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::SetDefaultErrorOrRecoveryHandlerBody,
    ) -> Result<ResponseValue<()>, Error<()>> {
        let url = format!(
            "{}/w/{}/schedules/setdefaulthandler", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::empty(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create http trigger

Sends a `POST` request to `/w/{workspace}/http_triggers/create`

Arguments:
- `workspace`
- `body`: new http trigger
*/
    pub async fn create_http_trigger<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::NewHttpTrigger,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/http_triggers/create", self.baseurl, encode_path(& workspace
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
    /**update http trigger

Sends a `POST` request to `/w/{workspace}/http_triggers/update/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated trigger
*/
    pub async fn update_http_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::EditHttpTrigger,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/http_triggers/update/{}", self.baseurl, encode_path(& workspace
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
    /**delete http trigger

Sends a `DELETE` request to `/w/{workspace}/http_triggers/delete/{path}`

*/
    pub async fn delete_http_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/http_triggers/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get http trigger

Sends a `GET` request to `/w/{workspace}/http_triggers/get/{path}`

*/
    pub async fn get_http_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::HttpTrigger>, Error<()>> {
        let url = format!(
            "{}/w/{}/http_triggers/get/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**list http triggers

Sends a `GET` request to `/w/{workspace}/http_triggers/list`

Arguments:
- `workspace`
- `is_flow`
- `page`: which page to return (start at 1, default 1)
- `path`: filter by path
- `path_start`
- `per_page`: number of items to return for a given page (default 30, max 100)
*/
    pub async fn list_http_triggers<'a>(
        &'a self,
        workspace: &'a str,
        is_flow: Option<bool>,
        page: Option<i64>,
        path: Option<&'a str>,
        path_start: Option<&'a str>,
        per_page: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::HttpTrigger>>, Error<()>> {
        let url = format!(
            "{}/w/{}/http_triggers/list", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(5usize);
        if let Some(v) = &is_flow {
            query.push(("is_flow", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &path {
            query.push(("path", v.to_string()));
        }
        if let Some(v) = &path_start {
            query.push(("path_start", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
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
    /**does http trigger exists

Sends a `GET` request to `/w/{workspace}/http_triggers/exists/{path}`

*/
    pub async fn exists_http_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/http_triggers/exists/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**does route exists

Sends a `POST` request to `/w/{workspace}/http_triggers/route_exists`

Arguments:
- `workspace`
- `body`: route exists request
*/
    pub async fn exists_route<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::ExistsRouteBody,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/http_triggers/route_exists", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create websocket trigger

Sends a `POST` request to `/w/{workspace}/websocket_triggers/create`

Arguments:
- `workspace`
- `body`: new websocket trigger
*/
    pub async fn create_websocket_trigger<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::NewWebsocketTrigger,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/websocket_triggers/create", self.baseurl, encode_path(& workspace
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
    /**update websocket trigger

Sends a `POST` request to `/w/{workspace}/websocket_triggers/update/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated trigger
*/
    pub async fn update_websocket_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::EditWebsocketTrigger,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/websocket_triggers/update/{}", self.baseurl, encode_path(& workspace
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
    /**delete websocket trigger

Sends a `DELETE` request to `/w/{workspace}/websocket_triggers/delete/{path}`

*/
    pub async fn delete_websocket_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/websocket_triggers/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get websocket trigger

Sends a `GET` request to `/w/{workspace}/websocket_triggers/get/{path}`

*/
    pub async fn get_websocket_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::WebsocketTrigger>, Error<()>> {
        let url = format!(
            "{}/w/{}/websocket_triggers/get/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**list websocket triggers

Sends a `GET` request to `/w/{workspace}/websocket_triggers/list`

Arguments:
- `workspace`
- `is_flow`
- `page`: which page to return (start at 1, default 1)
- `path`: filter by path
- `path_start`
- `per_page`: number of items to return for a given page (default 30, max 100)
*/
    pub async fn list_websocket_triggers<'a>(
        &'a self,
        workspace: &'a str,
        is_flow: Option<bool>,
        page: Option<i64>,
        path: Option<&'a str>,
        path_start: Option<&'a str>,
        per_page: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::WebsocketTrigger>>, Error<()>> {
        let url = format!(
            "{}/w/{}/websocket_triggers/list", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(5usize);
        if let Some(v) = &is_flow {
            query.push(("is_flow", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &path {
            query.push(("path", v.to_string()));
        }
        if let Some(v) = &path_start {
            query.push(("path_start", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
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
    /**does websocket trigger exists

Sends a `GET` request to `/w/{workspace}/websocket_triggers/exists/{path}`

*/
    pub async fn exists_websocket_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/websocket_triggers/exists/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**set enabled websocket trigger

Sends a `POST` request to `/w/{workspace}/websocket_triggers/setenabled/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated websocket trigger enable
*/
    pub async fn set_websocket_trigger_enabled<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::SetWebsocketTriggerEnabledBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/websocket_triggers/setenabled/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**test websocket connection

Sends a `POST` request to `/w/{workspace}/websocket_triggers/test`

Arguments:
- `workspace`
- `body`: test websocket connection
*/
    pub async fn test_websocket_connection<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::TestWebsocketConnectionBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/websocket_triggers/test", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create kafka trigger

Sends a `POST` request to `/w/{workspace}/kafka_triggers/create`

Arguments:
- `workspace`
- `body`: new kafka trigger
*/
    pub async fn create_kafka_trigger<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::NewKafkaTrigger,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/kafka_triggers/create", self.baseurl, encode_path(& workspace
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
    /**update kafka trigger

Sends a `POST` request to `/w/{workspace}/kafka_triggers/update/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated trigger
*/
    pub async fn update_kafka_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::EditKafkaTrigger,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/kafka_triggers/update/{}", self.baseurl, encode_path(& workspace
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
    /**delete kafka trigger

Sends a `DELETE` request to `/w/{workspace}/kafka_triggers/delete/{path}`

*/
    pub async fn delete_kafka_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/kafka_triggers/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get kafka trigger

Sends a `GET` request to `/w/{workspace}/kafka_triggers/get/{path}`

*/
    pub async fn get_kafka_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::KafkaTrigger>, Error<()>> {
        let url = format!(
            "{}/w/{}/kafka_triggers/get/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**list kafka triggers

Sends a `GET` request to `/w/{workspace}/kafka_triggers/list`

Arguments:
- `workspace`
- `is_flow`
- `page`: which page to return (start at 1, default 1)
- `path`: filter by path
- `path_start`
- `per_page`: number of items to return for a given page (default 30, max 100)
*/
    pub async fn list_kafka_triggers<'a>(
        &'a self,
        workspace: &'a str,
        is_flow: Option<bool>,
        page: Option<i64>,
        path: Option<&'a str>,
        path_start: Option<&'a str>,
        per_page: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::KafkaTrigger>>, Error<()>> {
        let url = format!(
            "{}/w/{}/kafka_triggers/list", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(5usize);
        if let Some(v) = &is_flow {
            query.push(("is_flow", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &path {
            query.push(("path", v.to_string()));
        }
        if let Some(v) = &path_start {
            query.push(("path_start", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
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
    /**does kafka trigger exists

Sends a `GET` request to `/w/{workspace}/kafka_triggers/exists/{path}`

*/
    pub async fn exists_kafka_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/kafka_triggers/exists/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**set enabled kafka trigger

Sends a `POST` request to `/w/{workspace}/kafka_triggers/setenabled/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated kafka trigger enable
*/
    pub async fn set_kafka_trigger_enabled<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::SetKafkaTriggerEnabledBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/kafka_triggers/setenabled/{}", self.baseurl, encode_path(& workspace
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
    /**test kafka connection

Sends a `POST` request to `/w/{workspace}/kafka_triggers/test`

Arguments:
- `workspace`
- `body`: test kafka connection
*/
    pub async fn test_kafka_connection<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::TestKafkaConnectionBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/kafka_triggers/test", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create nats trigger

Sends a `POST` request to `/w/{workspace}/nats_triggers/create`

Arguments:
- `workspace`
- `body`: new nats trigger
*/
    pub async fn create_nats_trigger<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::NewNatsTrigger,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/nats_triggers/create", self.baseurl, encode_path(& workspace
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
    /**update nats trigger

Sends a `POST` request to `/w/{workspace}/nats_triggers/update/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated trigger
*/
    pub async fn update_nats_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::EditNatsTrigger,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/nats_triggers/update/{}", self.baseurl, encode_path(& workspace
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
    /**delete nats trigger

Sends a `DELETE` request to `/w/{workspace}/nats_triggers/delete/{path}`

*/
    pub async fn delete_nats_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/nats_triggers/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get nats trigger

Sends a `GET` request to `/w/{workspace}/nats_triggers/get/{path}`

*/
    pub async fn get_nats_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::NatsTrigger>, Error<()>> {
        let url = format!(
            "{}/w/{}/nats_triggers/get/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**list nats triggers

Sends a `GET` request to `/w/{workspace}/nats_triggers/list`

Arguments:
- `workspace`
- `is_flow`
- `page`: which page to return (start at 1, default 1)
- `path`: filter by path
- `path_start`
- `per_page`: number of items to return for a given page (default 30, max 100)
*/
    pub async fn list_nats_triggers<'a>(
        &'a self,
        workspace: &'a str,
        is_flow: Option<bool>,
        page: Option<i64>,
        path: Option<&'a str>,
        path_start: Option<&'a str>,
        per_page: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::NatsTrigger>>, Error<()>> {
        let url = format!(
            "{}/w/{}/nats_triggers/list", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(5usize);
        if let Some(v) = &is_flow {
            query.push(("is_flow", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &path {
            query.push(("path", v.to_string()));
        }
        if let Some(v) = &path_start {
            query.push(("path_start", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
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
    /**does nats trigger exists

Sends a `GET` request to `/w/{workspace}/nats_triggers/exists/{path}`

*/
    pub async fn exists_nats_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/nats_triggers/exists/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**set enabled nats trigger

Sends a `POST` request to `/w/{workspace}/nats_triggers/setenabled/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated nats trigger enable
*/
    pub async fn set_nats_trigger_enabled<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::SetNatsTriggerEnabledBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/nats_triggers/setenabled/{}", self.baseurl, encode_path(& workspace
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
    /**test NATS connection

Sends a `POST` request to `/w/{workspace}/nats_triggers/test`

Arguments:
- `workspace`
- `body`: test nats connection
*/
    pub async fn test_nats_connection<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::TestNatsConnectionBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/nats_triggers/test", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create sqs trigger

Sends a `POST` request to `/w/{workspace}/sqs_triggers/create`

Arguments:
- `workspace`
- `body`: new sqs trigger
*/
    pub async fn create_sqs_trigger<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::NewSqsTrigger,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/sqs_triggers/create", self.baseurl, encode_path(& workspace
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
    /**update sqs trigger

Sends a `POST` request to `/w/{workspace}/sqs_triggers/update/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated trigger
*/
    pub async fn update_sqs_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::EditSqsTrigger,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/sqs_triggers/update/{}", self.baseurl, encode_path(& workspace
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
    /**delete sqs trigger

Sends a `DELETE` request to `/w/{workspace}/sqs_triggers/delete/{path}`

*/
    pub async fn delete_sqs_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/sqs_triggers/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get sqs trigger

Sends a `GET` request to `/w/{workspace}/sqs_triggers/get/{path}`

*/
    pub async fn get_sqs_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::SqsTrigger>, Error<()>> {
        let url = format!(
            "{}/w/{}/sqs_triggers/get/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**list sqs triggers

Sends a `GET` request to `/w/{workspace}/sqs_triggers/list`

Arguments:
- `workspace`
- `is_flow`
- `page`: which page to return (start at 1, default 1)
- `path`: filter by path
- `path_start`
- `per_page`: number of items to return for a given page (default 30, max 100)
*/
    pub async fn list_sqs_triggers<'a>(
        &'a self,
        workspace: &'a str,
        is_flow: Option<bool>,
        page: Option<i64>,
        path: Option<&'a str>,
        path_start: Option<&'a str>,
        per_page: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::SqsTrigger>>, Error<()>> {
        let url = format!(
            "{}/w/{}/sqs_triggers/list", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(5usize);
        if let Some(v) = &is_flow {
            query.push(("is_flow", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &path {
            query.push(("path", v.to_string()));
        }
        if let Some(v) = &path_start {
            query.push(("path_start", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
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
    /**does sqs trigger exists

Sends a `GET` request to `/w/{workspace}/sqs_triggers/exists/{path}`

*/
    pub async fn exists_sqs_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/sqs_triggers/exists/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**set enabled sqs trigger

Sends a `POST` request to `/w/{workspace}/sqs_triggers/setenabled/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated sqs trigger enable
*/
    pub async fn set_sqs_trigger_enabled<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::SetSqsTriggerEnabledBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/sqs_triggers/setenabled/{}", self.baseurl, encode_path(& workspace
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
    /**test sqs connection

Sends a `POST` request to `/w/{workspace}/sqs_triggers/test`

Arguments:
- `workspace`
- `body`: test sqs connection
*/
    pub async fn test_sqs_connection<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::TestSqsConnectionBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/sqs_triggers/test", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create mqtt trigger

Sends a `POST` request to `/w/{workspace}/mqtt_triggers/create`

Arguments:
- `workspace`
- `body`: new mqtt trigger
*/
    pub async fn create_mqtt_trigger<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::NewMqttTrigger,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/mqtt_triggers/create", self.baseurl, encode_path(& workspace
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
    /**update mqtt trigger

Sends a `POST` request to `/w/{workspace}/mqtt_triggers/update/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated trigger
*/
    pub async fn update_mqtt_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::EditMqttTrigger,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/mqtt_triggers/update/{}", self.baseurl, encode_path(& workspace
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
    /**delete mqtt trigger

Sends a `DELETE` request to `/w/{workspace}/mqtt_triggers/delete/{path}`

*/
    pub async fn delete_mqtt_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/mqtt_triggers/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get mqtt trigger

Sends a `GET` request to `/w/{workspace}/mqtt_triggers/get/{path}`

*/
    pub async fn get_mqtt_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::MqttTrigger>, Error<()>> {
        let url = format!(
            "{}/w/{}/mqtt_triggers/get/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**list mqtt triggers

Sends a `GET` request to `/w/{workspace}/mqtt_triggers/list`

Arguments:
- `workspace`
- `is_flow`
- `page`: which page to return (start at 1, default 1)
- `path`: filter by path
- `path_start`
- `per_page`: number of items to return for a given page (default 30, max 100)
*/
    pub async fn list_mqtt_triggers<'a>(
        &'a self,
        workspace: &'a str,
        is_flow: Option<bool>,
        page: Option<i64>,
        path: Option<&'a str>,
        path_start: Option<&'a str>,
        per_page: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::MqttTrigger>>, Error<()>> {
        let url = format!(
            "{}/w/{}/mqtt_triggers/list", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(5usize);
        if let Some(v) = &is_flow {
            query.push(("is_flow", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &path {
            query.push(("path", v.to_string()));
        }
        if let Some(v) = &path_start {
            query.push(("path_start", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
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
    /**does mqtt trigger exists

Sends a `GET` request to `/w/{workspace}/mqtt_triggers/exists/{path}`

*/
    pub async fn exists_mqtt_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/mqtt_triggers/exists/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**set enabled mqtt trigger

Sends a `POST` request to `/w/{workspace}/mqtt_triggers/setenabled/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated mqtt trigger enable
*/
    pub async fn set_mqtt_trigger_enabled<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::SetMqttTriggerEnabledBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/mqtt_triggers/setenabled/{}", self.baseurl, encode_path(& workspace
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
    /**test mqtt connection

Sends a `POST` request to `/w/{workspace}/mqtt_triggers/test`

Arguments:
- `workspace`
- `body`: test mqtt connection
*/
    pub async fn test_mqtt_connection<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::TestMqttConnectionBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/mqtt_triggers/test", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**check if postgres configuration is set to logical

Sends a `GET` request to `/w/{workspace}/postgres_triggers/is_valid_postgres_configuration/{path}`

*/
    pub async fn is_valid_postgres_configuration<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/is_valid_postgres_configuration/{}", self.baseurl,
            encode_path(& workspace.to_string()), encode_path(& path.to_string()),
        );
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
    /**create template script

Sends a `POST` request to `/w/{workspace}/postgres_triggers/create_template_script`

Arguments:
- `workspace`
- `body`: template script
*/
    pub async fn create_template_script<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::TemplateScript,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/create_template_script", self.baseurl,
            encode_path(& workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get template script

Sends a `GET` request to `/w/{workspace}/postgres_triggers/get_template_script/{id}`

*/
    pub async fn get_template_script<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/get_template_script/{}", self.baseurl,
            encode_path(& workspace.to_string()), encode_path(& id.to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list postgres replication slot

Sends a `GET` request to `/w/{workspace}/postgres_triggers/slot/list/{path}`

*/
    pub async fn list_postgres_replication_slot<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<Vec<types::SlotList>>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/slot/list/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& path.to_string()),
        );
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
    /**create replication slot for postgres

Sends a `POST` request to `/w/{workspace}/postgres_triggers/slot/create/{path}`

Arguments:
- `workspace`
- `path`
- `body`: new slot for postgres
*/
    pub async fn create_postgres_replication_slot<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::Slot,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/slot/create/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**delete postgres replication slot

Sends a `DELETE` request to `/w/{workspace}/postgres_triggers/slot/delete/{path}`

Arguments:
- `workspace`
- `path`
- `body`: replication slot of postgres
*/
    pub async fn delete_postgres_replication_slot<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::Slot,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/slot/delete/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.delete(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list postgres publication

Sends a `GET` request to `/w/{workspace}/postgres_triggers/publication/list/{path}`

*/
    pub async fn list_postgres_publication<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<Vec<String>>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/publication/list/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& path.to_string()),
        );
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
    /**get postgres publication

Sends a `GET` request to `/w/{workspace}/postgres_triggers/publication/get/{publication}/{path}`

*/
    pub async fn get_postgres_publication<'a>(
        &'a self,
        workspace: &'a str,
        publication: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::PublicationData>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/publication/get/{}/{}", self.baseurl,
            encode_path(& workspace.to_string()), encode_path(& publication.to_string()),
            encode_path(& path.to_string()),
        );
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
    /**create publication for postgres

Sends a `POST` request to `/w/{workspace}/postgres_triggers/publication/create/{publication}/{path}`

Arguments:
- `workspace`
- `publication`
- `path`
- `body`: new publication for postgres
*/
    pub async fn create_postgres_publication<'a>(
        &'a self,
        workspace: &'a str,
        publication: &'a str,
        path: &'a str,
        body: &'a types::PublicationData,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/publication/create/{}/{}", self.baseurl,
            encode_path(& workspace.to_string()), encode_path(& publication.to_string()),
            encode_path(& path.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**update publication for postgres

Sends a `POST` request to `/w/{workspace}/postgres_triggers/publication/update/{publication}/{path}`

Arguments:
- `workspace`
- `publication`
- `path`
- `body`: update publication for postgres
*/
    pub async fn update_postgres_publication<'a>(
        &'a self,
        workspace: &'a str,
        publication: &'a str,
        path: &'a str,
        body: &'a types::PublicationData,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/publication/update/{}/{}", self.baseurl,
            encode_path(& workspace.to_string()), encode_path(& publication.to_string()),
            encode_path(& path.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**delete postgres publication

Sends a `DELETE` request to `/w/{workspace}/postgres_triggers/publication/delete/{publication}/{path}`

*/
    pub async fn delete_postgres_publication<'a>(
        &'a self,
        workspace: &'a str,
        publication: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/publication/delete/{}/{}", self.baseurl,
            encode_path(& workspace.to_string()), encode_path(& publication.to_string()),
            encode_path(& path.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**create postgres trigger

Sends a `POST` request to `/w/{workspace}/postgres_triggers/create`

Arguments:
- `workspace`
- `body`: new postgres trigger
*/
    pub async fn create_postgres_trigger<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::NewPostgresTrigger,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/create", self.baseurl, encode_path(& workspace
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
    /**update postgres trigger

Sends a `POST` request to `/w/{workspace}/postgres_triggers/update/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated trigger
*/
    pub async fn update_postgres_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::EditPostgresTrigger,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/update/{}", self.baseurl, encode_path(& workspace
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
    /**delete postgres trigger

Sends a `DELETE` request to `/w/{workspace}/postgres_triggers/delete/{path}`

*/
    pub async fn delete_postgres_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get postgres trigger

Sends a `GET` request to `/w/{workspace}/postgres_triggers/get/{path}`

*/
    pub async fn get_postgres_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<types::PostgresTrigger>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/get/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**list postgres triggers

Sends a `GET` request to `/w/{workspace}/postgres_triggers/list`

Arguments:
- `workspace`
- `is_flow`
- `page`: which page to return (start at 1, default 1)
- `path`: filter by path
- `path_start`
- `per_page`: number of items to return for a given page (default 30, max 100)
*/
    pub async fn list_postgres_triggers<'a>(
        &'a self,
        workspace: &'a str,
        is_flow: Option<bool>,
        page: Option<i64>,
        path: Option<&'a str>,
        path_start: Option<&'a str>,
        per_page: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::PostgresTrigger>>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/list", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(5usize);
        if let Some(v) = &is_flow {
            query.push(("is_flow", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &path {
            query.push(("path", v.to_string()));
        }
        if let Some(v) = &path_start {
            query.push(("path_start", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
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
    /**does postgres trigger exists

Sends a `GET` request to `/w/{workspace}/postgres_triggers/exists/{path}`

*/
    pub async fn exists_postgres_trigger<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/exists/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& path.to_string()),
        );
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
    /**set enabled postgres trigger

Sends a `POST` request to `/w/{workspace}/postgres_triggers/setenabled/{path}`

Arguments:
- `workspace`
- `path`
- `body`: updated postgres trigger enable
*/
    pub async fn set_postgres_trigger_enabled<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        body: &'a types::SetPostgresTriggerEnabledBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/setenabled/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**test postgres connection

Sends a `POST` request to `/w/{workspace}/postgres_triggers/test`

Arguments:
- `workspace`
- `body`: test postgres connection
*/
    pub async fn test_postgres_connection<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::TestPostgresConnectionBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/postgres_triggers/test", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list instance groups

Sends a `GET` request to `/groups/list`

*/
    pub async fn list_instance_groups<'a>(
        &'a self,
    ) -> Result<ResponseValue<Vec<types::InstanceGroup>>, Error<()>> {
        let url = format!("{}/groups/list", self.baseurl,);
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
    /**get instance group

Sends a `GET` request to `/groups/get/{name}`

*/
    pub async fn get_instance_group<'a>(
        &'a self,
        name: &'a str,
    ) -> Result<ResponseValue<types::InstanceGroup>, Error<()>> {
        let url = format!(
            "{}/groups/get/{}", self.baseurl, encode_path(& name.to_string()),
        );
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
    /**create instance group

Sends a `POST` request to `/groups/create`

Arguments:
- `body`: create instance group
*/
    pub async fn create_instance_group<'a>(
        &'a self,
        body: &'a types::CreateInstanceGroupBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/groups/create", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**update instance group

Sends a `POST` request to `/groups/update/{name}`

Arguments:
- `name`
- `body`: update instance group
*/
    pub async fn update_instance_group<'a>(
        &'a self,
        name: &'a str,
        body: &'a types::UpdateInstanceGroupBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/groups/update/{}", self.baseurl, encode_path(& name.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**delete instance group

Sends a `DELETE` request to `/groups/delete/{name}`

*/
    pub async fn delete_instance_group<'a>(
        &'a self,
        name: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/groups/delete/{}", self.baseurl, encode_path(& name.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**add user to instance group

Sends a `POST` request to `/groups/adduser/{name}`

Arguments:
- `name`
- `body`: user to add to instance group
*/
    pub async fn add_user_to_instance_group<'a>(
        &'a self,
        name: &'a str,
        body: &'a types::AddUserToInstanceGroupBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/groups/adduser/{}", self.baseurl, encode_path(& name.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**remove user from instance group

Sends a `POST` request to `/groups/removeuser/{name}`

Arguments:
- `name`
- `body`: user to remove from instance group
*/
    pub async fn remove_user_from_instance_group<'a>(
        &'a self,
        name: &'a str,
        body: &'a types::RemoveUserFromInstanceGroupBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/groups/removeuser/{}", self.baseurl, encode_path(& name.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**export instance groups

Sends a `GET` request to `/groups/export`

*/
    pub async fn export_instance_groups<'a>(
        &'a self,
    ) -> Result<ResponseValue<Vec<types::ExportedInstanceGroup>>, Error<()>> {
        let url = format!("{}/groups/export", self.baseurl,);
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
    /**overwrite instance groups

Sends a `POST` request to `/groups/overwrite`

Arguments:
- `body`: overwrite instance groups
*/
    pub async fn overwrite_instance_groups<'a>(
        &'a self,
        body: &'a Vec<types::ExportedInstanceGroup>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!("{}/groups/overwrite", self.baseurl,);
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list groups

Sends a `GET` request to `/w/{workspace}/groups/list`

Arguments:
- `workspace`
- `page`: which page to return (start at 1, default 1)
- `per_page`: number of items to return for a given page (default 30, max 100)
*/
    pub async fn list_groups<'a>(
        &'a self,
        workspace: &'a str,
        page: Option<i64>,
        per_page: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::Group>>, Error<()>> {
        let url = format!(
            "{}/w/{}/groups/list", self.baseurl, encode_path(& workspace.to_string()),
        );
        let mut query = Vec::with_capacity(2usize);
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
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
    /**list group names

Sends a `GET` request to `/w/{workspace}/groups/listnames`

Arguments:
- `workspace`
- `only_member_of`: only list the groups the user is member of (default false)
*/
    pub async fn list_group_names<'a>(
        &'a self,
        workspace: &'a str,
        only_member_of: Option<bool>,
    ) -> Result<ResponseValue<Vec<String>>, Error<()>> {
        let url = format!(
            "{}/w/{}/groups/listnames", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &only_member_of {
            query.push(("only_member_of", v.to_string()));
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
    /**create group

Sends a `POST` request to `/w/{workspace}/groups/create`

Arguments:
- `workspace`
- `body`: create group
*/
    pub async fn create_group<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::CreateGroupBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/groups/create", self.baseurl, encode_path(& workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**update group

Sends a `POST` request to `/w/{workspace}/groups/update/{name}`

Arguments:
- `workspace`
- `name`
- `body`: updated group
*/
    pub async fn update_group<'a>(
        &'a self,
        workspace: &'a str,
        name: &'a str,
        body: &'a types::UpdateGroupBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/groups/update/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& name.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**delete group

Sends a `DELETE` request to `/w/{workspace}/groups/delete/{name}`

*/
    pub async fn delete_group<'a>(
        &'a self,
        workspace: &'a str,
        name: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/groups/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& name.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get group

Sends a `GET` request to `/w/{workspace}/groups/get/{name}`

*/
    pub async fn get_group<'a>(
        &'a self,
        workspace: &'a str,
        name: &'a str,
    ) -> Result<ResponseValue<types::Group>, Error<()>> {
        let url = format!(
            "{}/w/{}/groups/get/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& name.to_string()),
        );
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
    /**add user to group

Sends a `POST` request to `/w/{workspace}/groups/adduser/{name}`

Arguments:
- `workspace`
- `name`
- `body`: added user to group
*/
    pub async fn add_user_to_group<'a>(
        &'a self,
        workspace: &'a str,
        name: &'a str,
        body: &'a types::AddUserToGroupBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/groups/adduser/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& name.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**remove user to group

Sends a `POST` request to `/w/{workspace}/groups/removeuser/{name}`

Arguments:
- `workspace`
- `name`
- `body`: added user to group
*/
    pub async fn remove_user_to_group<'a>(
        &'a self,
        workspace: &'a str,
        name: &'a str,
        body: &'a types::RemoveUserToGroupBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/groups/removeuser/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& name.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list folders

Sends a `GET` request to `/w/{workspace}/folders/list`

Arguments:
- `workspace`
- `page`: which page to return (start at 1, default 1)
- `per_page`: number of items to return for a given page (default 30, max 100)
*/
    pub async fn list_folders<'a>(
        &'a self,
        workspace: &'a str,
        page: Option<i64>,
        per_page: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::Folder>>, Error<()>> {
        let url = format!(
            "{}/w/{}/folders/list", self.baseurl, encode_path(& workspace.to_string()),
        );
        let mut query = Vec::with_capacity(2usize);
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
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
    /**list folder names

Sends a `GET` request to `/w/{workspace}/folders/listnames`

Arguments:
- `workspace`
- `only_member_of`: only list the folders the user is member of (default false)
*/
    pub async fn list_folder_names<'a>(
        &'a self,
        workspace: &'a str,
        only_member_of: Option<bool>,
    ) -> Result<ResponseValue<Vec<String>>, Error<()>> {
        let url = format!(
            "{}/w/{}/folders/listnames", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &only_member_of {
            query.push(("only_member_of", v.to_string()));
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
    /**create folder

Sends a `POST` request to `/w/{workspace}/folders/create`

Arguments:
- `workspace`
- `body`: create folder
*/
    pub async fn create_folder<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::CreateFolderBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/folders/create", self.baseurl, encode_path(& workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**update folder

Sends a `POST` request to `/w/{workspace}/folders/update/{name}`

Arguments:
- `workspace`
- `name`
- `body`: update folder
*/
    pub async fn update_folder<'a>(
        &'a self,
        workspace: &'a str,
        name: &'a str,
        body: &'a types::UpdateFolderBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/folders/update/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& name.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**delete folder

Sends a `DELETE` request to `/w/{workspace}/folders/delete/{name}`

*/
    pub async fn delete_folder<'a>(
        &'a self,
        workspace: &'a str,
        name: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/folders/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& name.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get folder

Sends a `GET` request to `/w/{workspace}/folders/get/{name}`

*/
    pub async fn get_folder<'a>(
        &'a self,
        workspace: &'a str,
        name: &'a str,
    ) -> Result<ResponseValue<types::Folder>, Error<()>> {
        let url = format!(
            "{}/w/{}/folders/get/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& name.to_string()),
        );
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
    /**exists folder

Sends a `GET` request to `/w/{workspace}/folders/exists/{name}`

*/
    pub async fn exists_folder<'a>(
        &'a self,
        workspace: &'a str,
        name: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!(
            "{}/w/{}/folders/exists/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& name.to_string()),
        );
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
    /**get folder usage

Sends a `GET` request to `/w/{workspace}/folders/getusage/{name}`

*/
    pub async fn get_folder_usage<'a>(
        &'a self,
        workspace: &'a str,
        name: &'a str,
    ) -> Result<ResponseValue<types::GetFolderUsageResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/folders/getusage/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& name.to_string()),
        );
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
    /**add owner to folder

Sends a `POST` request to `/w/{workspace}/folders/addowner/{name}`

Arguments:
- `workspace`
- `name`
- `body`: owner user to folder
*/
    pub async fn add_owner_to_folder<'a>(
        &'a self,
        workspace: &'a str,
        name: &'a str,
        body: &'a types::AddOwnerToFolderBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/folders/addowner/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& name.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**remove owner to folder

Sends a `POST` request to `/w/{workspace}/folders/removeowner/{name}`

Arguments:
- `workspace`
- `name`
- `body`: added owner to folder
*/
    pub async fn remove_owner_to_folder<'a>(
        &'a self,
        workspace: &'a str,
        name: &'a str,
        body: &'a types::RemoveOwnerToFolderBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/folders/removeowner/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& name.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list workers

Sends a `GET` request to `/workers/list`

Arguments:
- `page`: which page to return (start at 1, default 1)
- `per_page`: number of items to return for a given page (default 30, max 100)
- `ping_since`: number of seconds the worker must have had a last ping more recent of (default to 300)
*/
    pub async fn list_workers<'a>(
        &'a self,
        page: Option<i64>,
        per_page: Option<i64>,
        ping_since: Option<i64>,
    ) -> Result<ResponseValue<Vec<types::WorkerPing>>, Error<()>> {
        let url = format!("{}/workers/list", self.baseurl,);
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
        }
        if let Some(v) = &ping_since {
            query.push(("ping_since", v.to_string()));
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
    /**exists worker with tag

Sends a `GET` request to `/workers/exists_worker_with_tag`

*/
    pub async fn exists_worker_with_tag<'a>(
        &'a self,
        tag: &'a str,
    ) -> Result<ResponseValue<bool>, Error<()>> {
        let url = format!("{}/workers/exists_worker_with_tag", self.baseurl,);
        let mut query = Vec::with_capacity(1usize);
        query.push(("tag", tag.to_string()));
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
    /**get queue metrics

Sends a `GET` request to `/workers/queue_metrics`

*/
    pub async fn get_queue_metrics<'a>(
        &'a self,
    ) -> Result<ResponseValue<Vec<types::GetQueueMetricsResponseItem>>, Error<()>> {
        let url = format!("{}/workers/queue_metrics", self.baseurl,);
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
    /**get counts of jobs waiting for an executor per tag

Sends a `GET` request to `/workers/queue_counts`

*/
    pub async fn get_counts_of_jobs_waiting_per_tag<'a>(
        &'a self,
    ) -> Result<ResponseValue<std::collections::HashMap<String, i64>>, Error<()>> {
        let url = format!("{}/workers/queue_counts", self.baseurl,);
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
    /**list worker groups

Sends a `GET` request to `/configs/list_worker_groups`

*/
    pub async fn list_worker_groups<'a>(
        &'a self,
    ) -> Result<ResponseValue<Vec<types::ListWorkerGroupsResponseItem>>, Error<()>> {
        let url = format!("{}/configs/list_worker_groups", self.baseurl,);
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
    /**get config

Sends a `GET` request to `/configs/get/{name}`

*/
    pub async fn get_config<'a>(
        &'a self,
        name: &'a str,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/configs/get/{}", self.baseurl, encode_path(& name.to_string()),
        );
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
    /**Update config

Sends a `POST` request to `/configs/update/{name}`

Arguments:
- `name`
- `body`: worker group
*/
    pub async fn update_config<'a>(
        &'a self,
        name: &'a str,
        body: &'a serde_json::Value,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/configs/update/{}", self.baseurl, encode_path(& name.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**Delete Config

Sends a `DELETE` request to `/configs/update/{name}`

*/
    pub async fn delete_config<'a>(
        &'a self,
        name: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/configs/update/{}", self.baseurl, encode_path(& name.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**list configs

Sends a `GET` request to `/configs/list`

*/
    pub async fn list_configs<'a>(
        &'a self,
    ) -> Result<ResponseValue<Vec<types::Config>>, Error<()>> {
        let url = format!("{}/configs/list", self.baseurl,);
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
    /**List autoscaling events

Sends a `GET` request to `/configs/list_autoscaling_events/{worker_group}`

*/
    pub async fn list_autoscaling_events<'a>(
        &'a self,
        worker_group: &'a str,
    ) -> Result<ResponseValue<Vec<types::AutoscalingEvent>>, Error<()>> {
        let url = format!(
            "{}/configs/list_autoscaling_events/{}", self.baseurl, encode_path(&
            worker_group.to_string()),
        );
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
    /**get granular acls

Sends a `GET` request to `/w/{workspace}/acls/get/{kind}/{path}`

*/
    pub async fn get_granular_acls<'a>(
        &'a self,
        workspace: &'a str,
        kind: types::GetGranularAclsKind,
        path: &'a str,
    ) -> Result<ResponseValue<std::collections::HashMap<String, bool>>, Error<()>> {
        let url = format!(
            "{}/w/{}/acls/get/{}/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& kind.to_string()), encode_path(& path.to_string()),
        );
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
    /**add granular acls

Sends a `POST` request to `/w/{workspace}/acls/add/{kind}/{path}`

Arguments:
- `workspace`
- `kind`
- `path`
- `body`: acl to add
*/
    pub async fn add_granular_acls<'a>(
        &'a self,
        workspace: &'a str,
        kind: types::AddGranularAclsKind,
        path: &'a str,
        body: &'a types::AddGranularAclsBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/acls/add/{}/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& kind.to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**remove granular acls

Sends a `POST` request to `/w/{workspace}/acls/remove/{kind}/{path}`

Arguments:
- `workspace`
- `kind`
- `path`
- `body`: acl to add
*/
    pub async fn remove_granular_acls<'a>(
        &'a self,
        workspace: &'a str,
        kind: types::RemoveGranularAclsKind,
        path: &'a str,
        body: &'a types::RemoveGranularAclsBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/acls/remove/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& kind.to_string()), encode_path(& path
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**set capture config

Sends a `POST` request to `/w/{workspace}/capture/set_config`

Arguments:
- `workspace`
- `body`: capture config
*/
    pub async fn set_capture_config<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::SetCaptureConfigBody,
    ) -> Result<ResponseValue<()>, Error<()>> {
        let url = format!(
            "{}/w/{}/capture/set_config", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::empty(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**ping capture config

Sends a `POST` request to `/w/{workspace}/capture/ping_config/{trigger_kind}/{runnable_kind}/{path}`

*/
    pub async fn ping_capture_config<'a>(
        &'a self,
        workspace: &'a str,
        trigger_kind: types::CaptureTriggerKind,
        runnable_kind: types::PingCaptureConfigRunnableKind,
        path: &'a str,
    ) -> Result<ResponseValue<()>, Error<()>> {
        let url = format!(
            "{}/w/{}/capture/ping_config/{}/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& trigger_kind.to_string()), encode_path(&
            runnable_kind.to_string()), encode_path(& path.to_string()),
        );
        let request = self.client.post(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::empty(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get capture configs for a script or flow

Sends a `GET` request to `/w/{workspace}/capture/get_configs/{runnable_kind}/{path}`

*/
    pub async fn get_capture_configs<'a>(
        &'a self,
        workspace: &'a str,
        runnable_kind: types::GetCaptureConfigsRunnableKind,
        path: &'a str,
    ) -> Result<ResponseValue<Vec<types::CaptureConfig>>, Error<()>> {
        let url = format!(
            "{}/w/{}/capture/get_configs/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& runnable_kind.to_string()), encode_path(& path
            .to_string()),
        );
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
    /**list captures for a script or flow

Sends a `GET` request to `/w/{workspace}/capture/list/{runnable_kind}/{path}`

Arguments:
- `workspace`
- `runnable_kind`
- `path`
- `page`: which page to return (start at 1, default 1)
- `per_page`: number of items to return for a given page (default 30, max 100)
- `trigger_kind`
*/
    pub async fn list_captures<'a>(
        &'a self,
        workspace: &'a str,
        runnable_kind: types::ListCapturesRunnableKind,
        path: &'a str,
        page: Option<i64>,
        per_page: Option<i64>,
        trigger_kind: Option<types::CaptureTriggerKind>,
    ) -> Result<ResponseValue<Vec<types::Capture>>, Error<()>> {
        let url = format!(
            "{}/w/{}/capture/list/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& runnable_kind.to_string()), encode_path(& path
            .to_string()),
        );
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
        }
        if let Some(v) = &trigger_kind {
            query.push(("trigger_kind", v.to_string()));
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
    /**move captures and configs for a script or flow

Sends a `POST` request to `/w/{workspace}/capture/move/{runnable_kind}/{path}`

Arguments:
- `workspace`
- `runnable_kind`
- `path`
- `body`: move captures and configs to a new path
*/
    pub async fn move_captures_and_configs<'a>(
        &'a self,
        workspace: &'a str,
        runnable_kind: types::MoveCapturesAndConfigsRunnableKind,
        path: &'a str,
        body: &'a types::MoveCapturesAndConfigsBody,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/capture/move/{}/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& runnable_kind.to_string()), encode_path(& path
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get a capture

Sends a `GET` request to `/w/{workspace}/capture/{id}`

*/
    pub async fn get_capture<'a>(
        &'a self,
        workspace: &'a str,
        id: i64,
    ) -> Result<ResponseValue<types::Capture>, Error<()>> {
        let url = format!(
            "{}/w/{}/capture/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& id.to_string()),
        );
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
    /**delete a capture

Sends a `DELETE` request to `/w/{workspace}/capture/{id}`

*/
    pub async fn delete_capture<'a>(
        &'a self,
        workspace: &'a str,
        id: i64,
    ) -> Result<ResponseValue<()>, Error<()>> {
        let url = format!(
            "{}/w/{}/capture/{}", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& id.to_string()),
        );
        let request = self.client.delete(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::empty(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**star item

Sends a `POST` request to `/w/{workspace}/favorites/star`

*/
    pub async fn star<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::StarBody,
    ) -> Result<ResponseValue<()>, Error<()>> {
        let url = format!(
            "{}/w/{}/favorites/star", self.baseurl, encode_path(& workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::empty(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**unstar item

Sends a `POST` request to `/w/{workspace}/favorites/unstar`

*/
    pub async fn unstar<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::UnstarBody,
    ) -> Result<ResponseValue<()>, Error<()>> {
        let url = format!(
            "{}/w/{}/favorites/unstar", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::empty(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**List Inputs used in previously completed jobs

Sends a `GET` request to `/w/{workspace}/inputs/history`

Arguments:
- `workspace`
- `args`: filter on jobs containing those args as a json subset (@> in postgres)
- `include_preview`
- `page`: which page to return (start at 1, default 1)
- `per_page`: number of items to return for a given page (default 30, max 100)
- `runnable_id`
- `runnable_type`
*/
    pub async fn get_input_history<'a>(
        &'a self,
        workspace: &'a str,
        args: Option<&'a str>,
        include_preview: Option<bool>,
        page: Option<i64>,
        per_page: Option<i64>,
        runnable_id: Option<&'a str>,
        runnable_type: Option<types::RunnableType>,
    ) -> Result<ResponseValue<Vec<types::Input>>, Error<()>> {
        let url = format!(
            "{}/w/{}/inputs/history", self.baseurl, encode_path(& workspace.to_string()),
        );
        let mut query = Vec::with_capacity(6usize);
        if let Some(v) = &args {
            query.push(("args", v.to_string()));
        }
        if let Some(v) = &include_preview {
            query.push(("include_preview", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
        }
        if let Some(v) = &runnable_id {
            query.push(("runnable_id", v.to_string()));
        }
        if let Some(v) = &runnable_type {
            query.push(("runnable_type", v.to_string()));
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
    /**Get args from history or saved input

Sends a `GET` request to `/w/{workspace}/inputs/{jobOrInputId}/args`

*/
    pub async fn get_args_from_history_or_saved_input<'a>(
        &'a self,
        workspace: &'a str,
        job_or_input_id: &'a str,
        allow_large: Option<bool>,
        input: Option<bool>,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/inputs/{}/args", self.baseurl, encode_path(& workspace.to_string()),
            encode_path(& job_or_input_id.to_string()),
        );
        let mut query = Vec::with_capacity(2usize);
        if let Some(v) = &allow_large {
            query.push(("allow_large", v.to_string()));
        }
        if let Some(v) = &input {
            query.push(("input", v.to_string()));
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
    /**List saved Inputs for a Runnable

Sends a `GET` request to `/w/{workspace}/inputs/list`

Arguments:
- `workspace`
- `page`: which page to return (start at 1, default 1)
- `per_page`: number of items to return for a given page (default 30, max 100)
- `runnable_id`
- `runnable_type`
*/
    pub async fn list_inputs<'a>(
        &'a self,
        workspace: &'a str,
        page: Option<i64>,
        per_page: Option<i64>,
        runnable_id: Option<&'a str>,
        runnable_type: Option<types::RunnableType>,
    ) -> Result<ResponseValue<Vec<types::Input>>, Error<()>> {
        let url = format!(
            "{}/w/{}/inputs/list", self.baseurl, encode_path(& workspace.to_string()),
        );
        let mut query = Vec::with_capacity(4usize);
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
        }
        if let Some(v) = &runnable_id {
            query.push(("runnable_id", v.to_string()));
        }
        if let Some(v) = &runnable_type {
            query.push(("runnable_type", v.to_string()));
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
    /**Create an Input for future use in a script or flow

Sends a `POST` request to `/w/{workspace}/inputs/create`

Arguments:
- `workspace`
- `runnable_id`
- `runnable_type`
- `body`: Input
*/
    pub async fn create_input<'a>(
        &'a self,
        workspace: &'a str,
        runnable_id: Option<&'a str>,
        runnable_type: Option<types::RunnableType>,
        body: &'a types::CreateInput,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/inputs/create", self.baseurl, encode_path(& workspace.to_string()),
        );
        let mut query = Vec::with_capacity(2usize);
        if let Some(v) = &runnable_id {
            query.push(("runnable_id", v.to_string()));
        }
        if let Some(v) = &runnable_type {
            query.push(("runnable_type", v.to_string()));
        }
        let request = self.client.post(url).json(&body).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**Update an Input

Sends a `POST` request to `/w/{workspace}/inputs/update`

Arguments:
- `workspace`
- `body`: UpdateInput
*/
    pub async fn update_input<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::UpdateInput,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/inputs/update", self.baseurl, encode_path(& workspace.to_string()),
        );
        let request = self.client.post(url).json(&body).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            201u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**Delete a Saved Input

Sends a `POST` request to `/w/{workspace}/inputs/delete/{input}`

*/
    pub async fn delete_input<'a>(
        &'a self,
        workspace: &'a str,
        input: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/inputs/delete/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& input.to_string()),
        );
        let request = self.client.post(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**Converts an S3 resource to the set of instructions necessary to connect DuckDB to an S3 bucket

Sends a `POST` request to `/w/{workspace}/job_helpers/duckdb_connection_settings`

Arguments:
- `workspace`
- `body`: S3 resource to connect to
*/
    pub async fn duckdb_connection_settings<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::DuckdbConnectionSettingsBody,
    ) -> Result<ResponseValue<types::DuckdbConnectionSettingsResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/duckdb_connection_settings", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**Converts an S3 resource to the set of instructions necessary to connect DuckDB to an S3 bucket

Sends a `POST` request to `/w/{workspace}/job_helpers/v2/duckdb_connection_settings`

Arguments:
- `workspace`
- `body`: S3 resource path to use to generate the connection settings. If empty, the S3 resource defined in the workspace settings will be used
*/
    pub async fn duckdb_connection_settings_v2<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::DuckdbConnectionSettingsV2Body,
    ) -> Result<ResponseValue<types::DuckdbConnectionSettingsV2Response>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/v2/duckdb_connection_settings", self.baseurl,
            encode_path(& workspace.to_string()),
        );
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**Converts an S3 resource to the set of arguments necessary to connect Polars to an S3 bucket

Sends a `POST` request to `/w/{workspace}/job_helpers/polars_connection_settings`

Arguments:
- `workspace`
- `body`: S3 resource to connect to
*/
    pub async fn polars_connection_settings<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::PolarsConnectionSettingsBody,
    ) -> Result<ResponseValue<types::PolarsConnectionSettingsResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/polars_connection_settings", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**Converts an S3 resource to the set of arguments necessary to connect Polars to an S3 bucket

Sends a `POST` request to `/w/{workspace}/job_helpers/v2/polars_connection_settings`

Arguments:
- `workspace`
- `body`: S3 resource path to use to generate the connection settings. If empty, the S3 resource defined in the workspace settings will be used
*/
    pub async fn polars_connection_settings_v2<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::PolarsConnectionSettingsV2Body,
    ) -> Result<ResponseValue<types::PolarsConnectionSettingsV2Response>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/v2/polars_connection_settings", self.baseurl,
            encode_path(& workspace.to_string()),
        );
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**Returns the s3 resource associated to the provided path, or the workspace default S3 resource

Sends a `POST` request to `/w/{workspace}/job_helpers/v2/s3_resource_info`

Arguments:
- `workspace`
- `body`: S3 resource path to use. If empty, the S3 resource defined in the workspace settings will be used
*/
    pub async fn s3_resource_info<'a>(
        &'a self,
        workspace: &'a str,
        body: &'a types::S3ResourceInfoBody,
    ) -> Result<ResponseValue<types::S3Resource>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/v2/s3_resource_info", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**Test connection to the workspace object storage

Sends a `GET` request to `/w/{workspace}/job_helpers/test_connection`

*/
    pub async fn dataset_storage_test_connection<'a>(
        &'a self,
        workspace: &'a str,
        storage: Option<&'a str>,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/test_connection", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        if let Some(v) = &storage {
            query.push(("storage", v.to_string()));
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
    /**List the file keys available in a workspace object storage

Sends a `GET` request to `/w/{workspace}/job_helpers/list_stored_files`

*/
    pub async fn list_stored_files<'a>(
        &'a self,
        workspace: &'a str,
        marker: Option<&'a str>,
        max_keys: i64,
        prefix: Option<&'a str>,
        storage: Option<&'a str>,
    ) -> Result<ResponseValue<types::ListStoredFilesResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/list_stored_files", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let mut query = Vec::with_capacity(4usize);
        if let Some(v) = &marker {
            query.push(("marker", v.to_string()));
        }
        query.push(("max_keys", max_keys.to_string()));
        if let Some(v) = &prefix {
            query.push(("prefix", v.to_string()));
        }
        if let Some(v) = &storage {
            query.push(("storage", v.to_string()));
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
    /**Load metadata of the file

Sends a `GET` request to `/w/{workspace}/job_helpers/load_file_metadata`

*/
    pub async fn load_file_metadata<'a>(
        &'a self,
        workspace: &'a str,
        file_key: &'a str,
        storage: Option<&'a str>,
    ) -> Result<ResponseValue<types::WindmillFileMetadata>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/load_file_metadata", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let mut query = Vec::with_capacity(2usize);
        query.push(("file_key", file_key.to_string()));
        if let Some(v) = &storage {
            query.push(("storage", v.to_string()));
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
    /**Load a preview of the file

Sends a `GET` request to `/w/{workspace}/job_helpers/load_file_preview`

*/
    pub async fn load_file_preview<'a>(
        &'a self,
        workspace: &'a str,
        csv_has_header: Option<bool>,
        csv_separator: Option<&'a str>,
        file_key: &'a str,
        file_mime_type: Option<&'a str>,
        file_size_in_bytes: Option<i64>,
        read_bytes_from: Option<i64>,
        read_bytes_length: Option<i64>,
        storage: Option<&'a str>,
    ) -> Result<ResponseValue<types::WindmillFilePreview>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/load_file_preview", self.baseurl, encode_path(&
            workspace.to_string()),
        );
        let mut query = Vec::with_capacity(8usize);
        if let Some(v) = &csv_has_header {
            query.push(("csv_has_header", v.to_string()));
        }
        if let Some(v) = &csv_separator {
            query.push(("csv_separator", v.to_string()));
        }
        query.push(("file_key", file_key.to_string()));
        if let Some(v) = &file_mime_type {
            query.push(("file_mime_type", v.to_string()));
        }
        if let Some(v) = &file_size_in_bytes {
            query.push(("file_size_in_bytes", v.to_string()));
        }
        if let Some(v) = &read_bytes_from {
            query.push(("read_bytes_from", v.to_string()));
        }
        if let Some(v) = &read_bytes_length {
            query.push(("read_bytes_length", v.to_string()));
        }
        if let Some(v) = &storage {
            query.push(("storage", v.to_string()));
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
    /**Load a preview of a parquet file

Sends a `GET` request to `/w/{workspace}/job_helpers/load_parquet_preview/{path}`

*/
    pub async fn load_parquet_preview<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        limit: Option<f64>,
        offset: Option<f64>,
        search_col: Option<&'a str>,
        search_term: Option<&'a str>,
        sort_col: Option<&'a str>,
        sort_desc: Option<bool>,
        storage: Option<&'a str>,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/load_parquet_preview/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(7usize);
        if let Some(v) = &limit {
            query.push(("limit", v.to_string()));
        }
        if let Some(v) = &offset {
            query.push(("offset", v.to_string()));
        }
        if let Some(v) = &search_col {
            query.push(("search_col", v.to_string()));
        }
        if let Some(v) = &search_term {
            query.push(("search_term", v.to_string()));
        }
        if let Some(v) = &sort_col {
            query.push(("sort_col", v.to_string()));
        }
        if let Some(v) = &sort_desc {
            query.push(("sort_desc", v.to_string()));
        }
        if let Some(v) = &storage {
            query.push(("storage", v.to_string()));
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
    /**Load the table row count

Sends a `GET` request to `/w/{workspace}/job_helpers/load_table_count/{path}`

*/
    pub async fn load_table_row_count<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        search_col: Option<&'a str>,
        search_term: Option<&'a str>,
        storage: Option<&'a str>,
    ) -> Result<ResponseValue<types::LoadTableRowCountResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/load_table_count/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &search_col {
            query.push(("search_col", v.to_string()));
        }
        if let Some(v) = &search_term {
            query.push(("search_term", v.to_string()));
        }
        if let Some(v) = &storage {
            query.push(("storage", v.to_string()));
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
    /**Load a preview of a csv file

Sends a `GET` request to `/w/{workspace}/job_helpers/load_csv_preview/{path}`

*/
    pub async fn load_csv_preview<'a>(
        &'a self,
        workspace: &'a str,
        path: &'a str,
        csv_separator: Option<&'a str>,
        limit: Option<f64>,
        offset: Option<f64>,
        search_col: Option<&'a str>,
        search_term: Option<&'a str>,
        sort_col: Option<&'a str>,
        sort_desc: Option<bool>,
        storage: Option<&'a str>,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/load_csv_preview/{}", self.baseurl, encode_path(&
            workspace.to_string()), encode_path(& path.to_string()),
        );
        let mut query = Vec::with_capacity(8usize);
        if let Some(v) = &csv_separator {
            query.push(("csv_separator", v.to_string()));
        }
        if let Some(v) = &limit {
            query.push(("limit", v.to_string()));
        }
        if let Some(v) = &offset {
            query.push(("offset", v.to_string()));
        }
        if let Some(v) = &search_col {
            query.push(("search_col", v.to_string()));
        }
        if let Some(v) = &search_term {
            query.push(("search_term", v.to_string()));
        }
        if let Some(v) = &sort_col {
            query.push(("sort_col", v.to_string()));
        }
        if let Some(v) = &sort_desc {
            query.push(("sort_desc", v.to_string()));
        }
        if let Some(v) = &storage {
            query.push(("storage", v.to_string()));
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
    /**Permanently delete file from S3

Sends a `DELETE` request to `/w/{workspace}/job_helpers/delete_s3_file`

*/
    pub async fn delete_s3_file<'a>(
        &'a self,
        workspace: &'a str,
        file_key: &'a str,
        storage: Option<&'a str>,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/delete_s3_file", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(2usize);
        query.push(("file_key", file_key.to_string()));
        if let Some(v) = &storage {
            query.push(("storage", v.to_string()));
        }
        let request = self
            .client
            .delete(url)
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
    /**Move a S3 file from one path to the other within the same bucket

Sends a `GET` request to `/w/{workspace}/job_helpers/move_s3_file`

*/
    pub async fn move_s3_file<'a>(
        &'a self,
        workspace: &'a str,
        dest_file_key: &'a str,
        src_file_key: &'a str,
        storage: Option<&'a str>,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/move_s3_file", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(3usize);
        query.push(("dest_file_key", dest_file_key.to_string()));
        query.push(("src_file_key", src_file_key.to_string()));
        if let Some(v) = &storage {
            query.push(("storage", v.to_string()));
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
    /**Upload file to S3 bucket

Sends a `POST` request to `/w/{workspace}/job_helpers/upload_s3_file`

Arguments:
- `workspace`
- `content_disposition`
- `content_type`
- `file_extension`
- `file_key`
- `resource_type`
- `s3_resource_path`
- `storage`
- `body`: File content
*/
    pub async fn file_upload<'a, B: Into<reqwest::Body>>(
        &'a self,
        workspace: &'a str,
        content_disposition: Option<&'a str>,
        content_type: Option<&'a str>,
        file_extension: Option<&'a str>,
        file_key: Option<&'a str>,
        resource_type: Option<&'a str>,
        s3_resource_path: Option<&'a str>,
        storage: Option<&'a str>,
        body: B,
    ) -> Result<ResponseValue<types::FileUploadResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/upload_s3_file", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(7usize);
        if let Some(v) = &content_disposition {
            query.push(("content_disposition", v.to_string()));
        }
        if let Some(v) = &content_type {
            query.push(("content_type", v.to_string()));
        }
        if let Some(v) = &file_extension {
            query.push(("file_extension", v.to_string()));
        }
        if let Some(v) = &file_key {
            query.push(("file_key", v.to_string()));
        }
        if let Some(v) = &resource_type {
            query.push(("resource_type", v.to_string()));
        }
        if let Some(v) = &s3_resource_path {
            query.push(("s3_resource_path", v.to_string()));
        }
        if let Some(v) = &storage {
            query.push(("storage", v.to_string()));
        }
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .header(
                reqwest::header::CONTENT_TYPE,
                reqwest::header::HeaderValue::from_static("application/octet-stream"),
            )
            .body(body)
            .query(&query)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**Download file from S3 bucket

Sends a `GET` request to `/w/{workspace}/job_helpers/download_s3_file`

*/
    pub async fn file_download<'a>(
        &'a self,
        workspace: &'a str,
        file_key: &'a str,
        resource_type: Option<&'a str>,
        s3_resource_path: Option<&'a str>,
        storage: Option<&'a str>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/download_s3_file", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(4usize);
        query.push(("file_key", file_key.to_string()));
        if let Some(v) = &resource_type {
            query.push(("resource_type", v.to_string()));
        }
        if let Some(v) = &s3_resource_path {
            query.push(("s3_resource_path", v.to_string()));
        }
        if let Some(v) = &storage {
            query.push(("storage", v.to_string()));
        }
        let request = self.client.get(url).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**Download file to S3 bucket

Sends a `GET` request to `/w/{workspace}/job_helpers/download_s3_parquet_file_as_csv`

*/
    pub async fn file_download_parquet_as_csv<'a>(
        &'a self,
        workspace: &'a str,
        file_key: &'a str,
        resource_type: Option<&'a str>,
        s3_resource_path: Option<&'a str>,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_helpers/download_s3_parquet_file_as_csv", self.baseurl,
            encode_path(& workspace.to_string()),
        );
        let mut query = Vec::with_capacity(3usize);
        query.push(("file_key", file_key.to_string()));
        if let Some(v) = &resource_type {
            query.push(("resource_type", v.to_string()));
        }
        if let Some(v) = &s3_resource_path {
            query.push(("s3_resource_path", v.to_string()));
        }
        let request = self.client.get(url).query(&query).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get job metrics

Sends a `POST` request to `/w/{workspace}/job_metrics/get/{id}`

Arguments:
- `workspace`
- `id`
- `body`: parameters for statistics retrieval
*/
    pub async fn get_job_metrics<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        body: &'a types::GetJobMetricsBody,
    ) -> Result<ResponseValue<types::GetJobMetricsResponse>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_metrics/get/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()),
        );
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**set job metrics

Sends a `POST` request to `/w/{workspace}/job_metrics/set_progress/{id}`

Arguments:
- `workspace`
- `id`
- `body`: parameters for statistics retrieval
*/
    pub async fn set_job_progress<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
        body: &'a types::SetJobProgressBody,
    ) -> Result<ResponseValue<serde_json::Value>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_metrics/set_progress/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()),
        );
        let request = self
            .client
            .post(url)
            .header(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .json(&body)
            .build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => ResponseValue::from_response(response).await,
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**get job progress

Sends a `GET` request to `/w/{workspace}/job_metrics/get_progress/{id}`

*/
    pub async fn get_job_progress<'a>(
        &'a self,
        workspace: &'a str,
        id: &'a uuid::Uuid,
    ) -> Result<ResponseValue<i64>, Error<()>> {
        let url = format!(
            "{}/w/{}/job_metrics/get_progress/{}", self.baseurl, encode_path(& workspace
            .to_string()), encode_path(& id.to_string()),
        );
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
    /**list log files ordered by timestamp

Sends a `GET` request to `/service_logs/list_files`

Arguments:
- `after`: filter on created after (exclusive) timestamp
- `before`: filter on started before (inclusive) timestamp
- `with_error`
*/
    pub async fn list_log_files<'a>(
        &'a self,
        after: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        before: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        with_error: Option<bool>,
    ) -> Result<ResponseValue<Vec<types::ListLogFilesResponseItem>>, Error<()>> {
        let url = format!("{}/service_logs/list_files", self.baseurl,);
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &after {
            query.push(("after", v.to_string()));
        }
        if let Some(v) = &before {
            query.push(("before", v.to_string()));
        }
        if let Some(v) = &with_error {
            query.push(("with_error", v.to_string()));
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
    /**get log file by path

Sends a `GET` request to `/service_logs/get_log_file/{path}`

*/
    pub async fn get_log_file<'a>(
        &'a self,
        path: &'a str,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/service_logs/get_log_file/{}", self.baseurl, encode_path(& path
            .to_string()),
        );
        let request = self.client.get(url).build()?;
        let result = self.client.execute(request).await;
        let response = result?;
        match response.status().as_u16() {
            200u16 => Ok(ResponseValue::stream(response)),
            _ => Err(Error::UnexpectedResponse(response)),
        }
    }
    /**List all concurrency groups

Sends a `GET` request to `/concurrency_groups/list`

*/
    pub async fn list_concurrency_groups<'a>(
        &'a self,
    ) -> Result<ResponseValue<Vec<types::ConcurrencyGroup>>, Error<()>> {
        let url = format!("{}/concurrency_groups/list", self.baseurl,);
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
    /**Delete concurrency group

Sends a `DELETE` request to `/concurrency_groups/prune/{concurrency_id}`

*/
    pub async fn delete_concurrency_group<'a>(
        &'a self,
        concurrency_id: &'a str,
    ) -> Result<
        ResponseValue<std::collections::HashMap<String, serde_json::Value>>,
        Error<()>,
    > {
        let url = format!(
            "{}/concurrency_groups/prune/{}", self.baseurl, encode_path(& concurrency_id
            .to_string()),
        );
        let request = self
            .client
            .delete(url)
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
    /**Get the concurrency key for a job that has concurrency limits enabled

Sends a `GET` request to `/concurrency_groups/{id}/key`

*/
    pub async fn get_concurrency_key<'a>(
        &'a self,
        id: &'a uuid::Uuid,
    ) -> Result<ResponseValue<String>, Error<()>> {
        let url = format!(
            "{}/concurrency_groups/{}/key", self.baseurl, encode_path(& id.to_string()),
        );
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
    /**Get intervals of job runtime concurrency

Sends a `GET` request to `/w/{workspace}/concurrency_groups/list_jobs`

Arguments:
- `workspace`
- `all_workspaces`: get jobs from all workspaces (only valid if request come from the `admins` workspace)
- `args`: filter on jobs containing those args as a json subset (@> in postgres)
- `concurrency_key`
- `created_by`: mask to filter exact matching user creator
- `created_or_started_after`: filter on created_at for non non started job and started_at otherwise after (exclusive) timestamp
- `created_or_started_after_completed_jobs`: filter on created_at for non non started job and started_at otherwise after (exclusive) timestamp but only for the completed jobs
- `created_or_started_before`: filter on created_at for non non started job and started_at otherwise before (inclusive) timestamp
- `has_null_parent`: has null parent
- `is_flow_step`: is the job a flow step
- `is_not_schedule`: is not a scheduled job
- `is_skipped`: is the job skipped
- `job_kinds`: filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
- `label`: mask to filter exact matching job's label (job labels are completed jobs with as a result an object containing a string in the array at key 'wm_labels')
- `page`: which page to return (start at 1, default 1)
- `parent_job`: The parent job that is at the origin and responsible for the execution of this script if any
- `per_page`: number of items to return for a given page (default 30, max 100)
- `result`: filter on jobs containing those result as a json subset (@> in postgres)
- `row_limit`
- `running`: filter on running jobs
- `schedule_path`: mask to filter by schedule path
- `scheduled_for_before_now`: filter on jobs scheduled_for before now (hence waitinf for a worker)
- `script_hash`: mask to filter exact matching path
- `script_path_exact`: mask to filter exact matching path
- `script_path_start`: mask to filter matching starting path
- `started_after`: filter on started after (exclusive) timestamp
- `started_before`: filter on started before (inclusive) timestamp
- `success`: filter on successful jobs
- `tag`: filter on jobs with a given tag/worker group
*/
    pub async fn list_extended_jobs<'a>(
        &'a self,
        workspace: &'a str,
        all_workspaces: Option<bool>,
        args: Option<&'a str>,
        concurrency_key: Option<&'a str>,
        created_by: Option<&'a str>,
        created_or_started_after: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        created_or_started_after_completed_jobs: Option<
            &'a chrono::DateTime<chrono::offset::Utc>,
        >,
        created_or_started_before: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        has_null_parent: Option<bool>,
        is_flow_step: Option<bool>,
        is_not_schedule: Option<bool>,
        is_skipped: Option<bool>,
        job_kinds: Option<&'a str>,
        label: Option<&'a str>,
        page: Option<i64>,
        parent_job: Option<&'a uuid::Uuid>,
        per_page: Option<i64>,
        result: Option<&'a str>,
        row_limit: Option<f64>,
        running: Option<bool>,
        schedule_path: Option<&'a str>,
        scheduled_for_before_now: Option<bool>,
        script_hash: Option<&'a str>,
        script_path_exact: Option<&'a str>,
        script_path_start: Option<&'a str>,
        started_after: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        started_before: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        success: Option<bool>,
        tag: Option<&'a str>,
    ) -> Result<ResponseValue<types::ExtendedJobs>, Error<()>> {
        let url = format!(
            "{}/w/{}/concurrency_groups/list_jobs", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(28usize);
        if let Some(v) = &all_workspaces {
            query.push(("all_workspaces", v.to_string()));
        }
        if let Some(v) = &args {
            query.push(("args", v.to_string()));
        }
        if let Some(v) = &concurrency_key {
            query.push(("concurrency_key", v.to_string()));
        }
        if let Some(v) = &created_by {
            query.push(("created_by", v.to_string()));
        }
        if let Some(v) = &created_or_started_after {
            query.push(("created_or_started_after", v.to_string()));
        }
        if let Some(v) = &created_or_started_after_completed_jobs {
            query.push(("created_or_started_after_completed_jobs", v.to_string()));
        }
        if let Some(v) = &created_or_started_before {
            query.push(("created_or_started_before", v.to_string()));
        }
        if let Some(v) = &has_null_parent {
            query.push(("has_null_parent", v.to_string()));
        }
        if let Some(v) = &is_flow_step {
            query.push(("is_flow_step", v.to_string()));
        }
        if let Some(v) = &is_not_schedule {
            query.push(("is_not_schedule", v.to_string()));
        }
        if let Some(v) = &is_skipped {
            query.push(("is_skipped", v.to_string()));
        }
        if let Some(v) = &job_kinds {
            query.push(("job_kinds", v.to_string()));
        }
        if let Some(v) = &label {
            query.push(("label", v.to_string()));
        }
        if let Some(v) = &page {
            query.push(("page", v.to_string()));
        }
        if let Some(v) = &parent_job {
            query.push(("parent_job", v.to_string()));
        }
        if let Some(v) = &per_page {
            query.push(("per_page", v.to_string()));
        }
        if let Some(v) = &result {
            query.push(("result", v.to_string()));
        }
        if let Some(v) = &row_limit {
            query.push(("row_limit", v.to_string()));
        }
        if let Some(v) = &running {
            query.push(("running", v.to_string()));
        }
        if let Some(v) = &schedule_path {
            query.push(("schedule_path", v.to_string()));
        }
        if let Some(v) = &scheduled_for_before_now {
            query.push(("scheduled_for_before_now", v.to_string()));
        }
        if let Some(v) = &script_hash {
            query.push(("script_hash", v.to_string()));
        }
        if let Some(v) = &script_path_exact {
            query.push(("script_path_exact", v.to_string()));
        }
        if let Some(v) = &script_path_start {
            query.push(("script_path_start", v.to_string()));
        }
        if let Some(v) = &started_after {
            query.push(("started_after", v.to_string()));
        }
        if let Some(v) = &started_before {
            query.push(("started_before", v.to_string()));
        }
        if let Some(v) = &success {
            query.push(("success", v.to_string()));
        }
        if let Some(v) = &tag {
            query.push(("tag", v.to_string()));
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
    /**Search through jobs with a string query

Sends a `GET` request to `/srch/w/{workspace}/index/search/job`

*/
    pub async fn search_jobs_index<'a>(
        &'a self,
        workspace: &'a str,
        search_query: &'a str,
    ) -> Result<ResponseValue<types::SearchJobsIndexResponse>, Error<()>> {
        let url = format!(
            "{}/srch/w/{}/index/search/job", self.baseurl, encode_path(& workspace
            .to_string()),
        );
        let mut query = Vec::with_capacity(1usize);
        query.push(("search_query", search_query.to_string()));
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
    /**Search through service logs with a string query

Sends a `GET` request to `/srch/index/search/service_logs`

*/
    pub async fn search_logs_index<'a>(
        &'a self,
        hostname: &'a str,
        max_ts: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        min_ts: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        mode: &'a str,
        search_query: &'a str,
        worker_group: Option<&'a str>,
    ) -> Result<ResponseValue<types::SearchLogsIndexResponse>, Error<()>> {
        let url = format!("{}/srch/index/search/service_logs", self.baseurl,);
        let mut query = Vec::with_capacity(6usize);
        query.push(("hostname", hostname.to_string()));
        if let Some(v) = &max_ts {
            query.push(("max_ts", v.to_string()));
        }
        if let Some(v) = &min_ts {
            query.push(("min_ts", v.to_string()));
        }
        query.push(("mode", mode.to_string()));
        query.push(("search_query", search_query.to_string()));
        if let Some(v) = &worker_group {
            query.push(("worker_group", v.to_string()));
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
    /**Search and count the log line hits on every provided host

Sends a `GET` request to `/srch/index/search/count_service_logs`

*/
    pub async fn count_search_logs_index<'a>(
        &'a self,
        max_ts: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        min_ts: Option<&'a chrono::DateTime<chrono::offset::Utc>>,
        search_query: &'a str,
    ) -> Result<ResponseValue<types::CountSearchLogsIndexResponse>, Error<()>> {
        let url = format!("{}/srch/index/search/count_service_logs", self.baseurl,);
        let mut query = Vec::with_capacity(3usize);
        if let Some(v) = &max_ts {
            query.push(("max_ts", v.to_string()));
        }
        if let Some(v) = &min_ts {
            query.push(("min_ts", v.to_string()));
        }
        query.push(("search_query", search_query.to_string()));
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
    /**Restart container and delete the index to recreate it

Sends a `DELETE` request to `/srch/index/delete/{idx_name}`

*/
    pub async fn clear_index<'a>(
        &'a self,
        idx_name: types::ClearIndexIdxName,
    ) -> Result<ResponseValue<ByteStream>, Error<()>> {
        let url = format!(
            "{}/srch/index/delete/{}", self.baseurl, encode_path(& idx_name.to_string()),
        );
        let request = self.client.delete(url).build()?;
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
