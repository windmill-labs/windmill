use serde::{Deserialize, Serialize};

pub mod external;
mod routes;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NextCloudOAuthData {
    pub base_url: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OcsResponse<T = String> {
    pub ocs: OcsData<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Meta {
    status: String,
    #[serde(rename = "statuscode")]
    status_code: u16,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OcsData<T> {
    pub meta: Meta,
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextCloudEventType {
    pub name: String,
    pub description: Option<String>,
    pub path: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Copy, Clone)]
pub struct NextCloud;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextcloudServiceConfig {
    pub event: String,
    pub event_filter: Option<Box<serde_json::value::RawValue>>,
    pub user_id_filter: Option<String>,
    pub headers: Option<Box<serde_json::value::RawValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextCloudTriggerData {
    #[serde(skip_serializing)]
    pub id: i64,
    #[serde(skip_serializing)]
    pub uri: String,
    pub event: String,
    pub event_filter: Option<Box<serde_json::value::RawValue>>,
    pub user_id_filter: Option<String>,
    pub headers: Option<Box<serde_json::value::RawValue>>,
}
