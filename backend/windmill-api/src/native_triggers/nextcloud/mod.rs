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
pub struct NextCloudPayload {
    pub event: String,
    #[serde(
        rename(serialize = "eventFilter"),
        skip_serializing_if = "Option::is_none"
    )]
    pub event_filter: Option<Box<serde_json::value::RawValue>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename(serialize = "userIdFilter")
    )]
    pub user_id_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Box<serde_json::value::RawValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextCloudTriggerData {
    pub id: i64,
    pub uri: String,
    pub event: String,
    #[serde(rename(deserialize = "eventFilter"))]
    pub event_filter: Option<Box<serde_json::value::RawValue>>,
    #[serde(rename(deserialize = "userIdFilter"))]
    pub user_id_filter: Option<String>,
    pub headers: Option<Box<serde_json::value::RawValue>>,
    #[serde(rename(deserialize = "authMethod"))]
    pub auth_method: String,
    #[serde(rename(deserialize = "authData"))]
    pub auth_data: Option<Box<serde_json::value::RawValue>>,
}
