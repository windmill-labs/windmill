//! Minimal Windmill API client for tests
//!
//! This is a handwritten minimal client that provides just enough functionality
//! for the integration tests. It replaces the auto-generated progenitor client.

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Client for Windmill API
pub struct Client {
    pub baseurl: String,
    pub client: reqwest::Client,
}

impl Client {
    /// Create a new client with an existing reqwest::Client
    pub fn new_with_client(baseurl: &str, client: reqwest::Client) -> Self {
        Self {
            baseurl: baseurl.to_string(),
            client,
        }
    }

    /// Get the base URL
    pub fn baseurl(&self) -> &String {
        &self.baseurl
    }

    /// Get the internal reqwest::Client
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Create a script
    pub async fn create_script(
        &self,
        workspace: &str,
        body: &types::NewScript,
    ) -> Result<String, Error> {
        let url = format!(
            "{}/w/{}/scripts/create",
            self.baseurl,
            urlencoding::encode(workspace)
        );
        let response = self.client.post(&url).json(body).send().await?;

        if response.status().is_success() {
            Ok(response.text().await?)
        } else {
            Err(Error::UnexpectedResponse(response.status().as_u16(), response.text().await.unwrap_or_default()))
        }
    }

    /// Create a flow
    pub async fn create_flow(
        &self,
        workspace: &str,
        body: &types::CreateFlowBody,
    ) -> Result<String, Error> {
        let url = format!(
            "{}/w/{}/flows/create",
            self.baseurl,
            urlencoding::encode(workspace)
        );
        let response = self.client.post(&url).json(body).send().await?;

        if response.status().is_success() {
            Ok(response.text().await?)
        } else {
            Err(Error::UnexpectedResponse(response.status().as_u16(), response.text().await.unwrap_or_default()))
        }
    }

    /// Get flow by path
    pub async fn get_flow_by_path(
        &self,
        workspace: &str,
        path: &str,
        with_starred_info: Option<bool>,
    ) -> Result<types::Flow, Error> {
        let url = format!(
            "{}/w/{}/flows/get/{}",
            self.baseurl,
            urlencoding::encode(workspace),
            urlencoding::encode(path)
        );

        let mut request = self.client.get(&url);
        if let Some(starred) = with_starred_info {
            request = request.query(&[("with_starred_info", starred.to_string())]);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(Error::UnexpectedResponse(response.status().as_u16(), response.text().await.unwrap_or_default()))
        }
    }

    /// Create a schedule
    pub async fn create_schedule(
        &self,
        workspace: &str,
        body: &types::NewSchedule,
    ) -> Result<String, Error> {
        let url = format!(
            "{}/w/{}/schedules/create",
            self.baseurl,
            urlencoding::encode(workspace)
        );
        let response = self.client.post(&url).json(body).send().await?;

        if response.status().is_success() {
            Ok(response.text().await?)
        } else {
            Err(Error::UnexpectedResponse(response.status().as_u16(), response.text().await.unwrap_or_default()))
        }
    }

    /// Update a schedule
    pub async fn update_schedule(
        &self,
        workspace: &str,
        path: &str,
        body: &types::EditSchedule,
    ) -> Result<String, Error> {
        let url = format!(
            "{}/w/{}/schedules/update/{}",
            self.baseurl,
            urlencoding::encode(workspace),
            urlencoding::encode(path)
        );
        let response = self.client.post(&url).json(body).send().await?;

        if response.status().is_success() {
            Ok(response.text().await?)
        } else {
            Err(Error::UnexpectedResponse(response.status().as_u16(), response.text().await.unwrap_or_default()))
        }
    }

    /// List workspaces
    pub async fn list_workspaces(&self) -> Result<Vec<types::Workspace>, Error> {
        let url = format!("{}/workspaces/list", self.baseurl);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(Error::UnexpectedResponse(response.status().as_u16(), response.text().await.unwrap_or_default()))
        }
    }
}

/// Create a client with bearer token authentication
pub fn create_client(base_url: &str, token: String) -> Client {
    let mut val = HeaderValue::from_str(&format!("Bearer {token}")).expect("header creation");
    val.set_sensitive(true);
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, val);
    let client = reqwest::ClientBuilder::new()
        .default_headers(headers)
        .build()
        .expect("client build");
    Client::new_with_client(&format!("{}/api", base_url.trim_end_matches('/')), client)
}

/// Error type for API client
#[derive(Debug)]
pub enum Error {
    /// Request error
    Request(reqwest::Error),
    /// Unexpected response status
    UnexpectedResponse(u16, String),
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Request(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Request(e) => write!(f, "Request error: {}", e),
            Error::UnexpectedResponse(status, body) => {
                write!(f, "Unexpected response ({}): {}", status, body)
            }
        }
    }
}

impl std::error::Error for Error {}

/// API types
pub mod types {
    use super::*;

    /// Script language
    #[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
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
        #[serde(rename = "java")]
        Java,
        #[serde(rename = "ruby")]
        Ruby,
        #[serde(rename = "duckdb")]
        Duckdb,
    }

    impl std::str::FromStr for ScriptLang {
        type Err = &'static str;
        fn from_str(value: &str) -> Result<Self, Self::Err> {
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
                "java" => Ok(Self::Java),
                "ruby" => Ok(Self::Ruby),
                "duckdb" => Ok(Self::Duckdb),
                _ => Err("invalid script language"),
            }
        }
    }

    /// Raw script language (for flow modules)
    #[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
    pub enum RawScriptLanguage {
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
        #[serde(rename = "java")]
        Java,
        #[serde(rename = "ruby")]
        Ruby,
        #[serde(rename = "duckdb")]
        Duckdb,
    }

    /// New script request body
    #[derive(Clone, Debug, Serialize)]
    pub struct NewScript {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub assets: Vec<serde_json::Value>,
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
        pub kind: Option<String>,
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
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        pub schema: HashMap<String, serde_json::Value>,
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

    /// Script arguments (used in schedules)
    pub type ScriptArgs = HashMap<String, serde_json::Value>;

    /// New schedule request body
    #[derive(Clone, Debug, Serialize)]
    pub struct NewSchedule {
        pub args: ScriptArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cron_version: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
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
        pub retry: Option<serde_json::Value>,
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

    /// Edit schedule request body
    #[derive(Clone, Debug, Serialize)]
    pub struct EditSchedule {
        pub args: ScriptArgs,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cron_version: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
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
        pub retry: Option<serde_json::Value>,
        pub schedule: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tag: Option<String>,
        pub timezone: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ws_error_handler_muted: Option<bool>,
    }

    /// Open flow definition
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct OpenFlow {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        pub schema: HashMap<String, serde_json::Value>,
        pub summary: String,
        pub value: FlowValue,
    }

    /// Flow value containing modules
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

    /// Flow module
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
        pub mock: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub priority: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub retry: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub skip_if: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub sleep: Option<InputTransform>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub stop_after_all_iters_if: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub stop_after_if: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub suspend: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub timeout: Option<InputTransform>,
        pub value: FlowModuleValue,
    }

    /// Input transform
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum InputTransform {
        Static {
            #[serde(rename = "type")]
            type_: String,
            value: serde_json::Value
        },
        Javascript {
            #[serde(rename = "type")]
            type_: String,
            expr: String
        },
    }

    /// Flow module value (the actual module content)
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum FlowModuleValue {
        RawScript(RawScript),
        Script(ScriptModule),
        Flow(FlowModule2),
        ForLoop(ForLoopModule),
        WhileLoop(WhileLoopModule),
        BranchOne(BranchOneModule),
        BranchAll(BranchAllModule),
        Identity(IdentityModule),
        Other(serde_json::Value),
    }

    /// Raw script module
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RawScript {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub assets: Vec<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub concurrency_time_window_s: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub concurrent_limit: Option<f64>,
        pub content: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub custom_concurrency_key: Option<String>,
        pub input_transforms: HashMap<String, InputTransform>,
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
        pub type_: String,
    }

    impl RawScript {
        pub fn new(content: String, language: RawScriptLanguage) -> Self {
            Self {
                assets: vec![],
                concurrency_time_window_s: None,
                concurrent_limit: None,
                content,
                custom_concurrency_key: None,
                input_transforms: HashMap::new(),
                is_trigger: None,
                language,
                lock: None,
                path: None,
                tag: None,
                type_: "rawscript".to_string(),
            }
        }
    }

    /// Script module reference
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ScriptModule {
        #[serde(rename = "type")]
        pub type_: String,
        pub path: String,
        pub input_transforms: HashMap<String, InputTransform>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub hash: Option<String>,
    }

    /// Flow module reference
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct FlowModule2 {
        #[serde(rename = "type")]
        pub type_: String,
        pub path: String,
        pub input_transforms: HashMap<String, InputTransform>,
    }

    /// For loop module
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct ForLoopModule {
        #[serde(rename = "type")]
        pub type_: String,
        pub iterator: InputTransform,
        pub modules: Vec<FlowModule>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub parallel: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub parallelism: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub skip_failures: Option<bool>,
    }

    /// While loop module
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct WhileLoopModule {
        #[serde(rename = "type")]
        pub type_: String,
        pub modules: Vec<FlowModule>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub skip_failures: Option<bool>,
    }

    /// Branch one module
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct BranchOneModule {
        #[serde(rename = "type")]
        pub type_: String,
        pub branches: Vec<serde_json::Value>,
        pub default: Vec<FlowModule>,
    }

    /// Branch all module
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct BranchAllModule {
        #[serde(rename = "type")]
        pub type_: String,
        pub branches: Vec<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub parallel: Option<bool>,
    }

    /// Identity module
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct IdentityModule {
        #[serde(rename = "type")]
        pub type_: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub flow: Option<bool>,
    }

    /// Open flow with path
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

    /// Create flow request body
    #[derive(Clone, Debug, Serialize)]
    pub struct CreateFlowBody {
        #[serde(flatten)]
        pub open_flow_w_path: OpenFlowWPath,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub deployment_message: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub draft_only: Option<bool>,
    }

    /// Flow response type
    #[derive(Clone, Debug, Deserialize)]
    pub struct Flow {
        pub path: String,
        pub summary: String,
        #[serde(default)]
        pub description: Option<String>,
        pub value: serde_json::Value,
        #[serde(default)]
        pub schema: Option<serde_json::Value>,
        #[serde(flatten)]
        pub extra: HashMap<String, serde_json::Value>,
    }

    /// Workspace
    #[derive(Clone, Debug, Deserialize)]
    pub struct Workspace {
        pub id: String,
        pub name: String,
        #[serde(default)]
        pub owner: Option<String>,
        #[serde(flatten)]
        pub extra: HashMap<String, serde_json::Value>,
    }
}
