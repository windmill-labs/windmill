use serde::{Deserialize, Serialize};

mod external;
mod routes;

#[derive(Debug, Clone, Deserialize)]
pub struct NextCloudResource {
    pub base_url: String,
    pub username: String,
    pub password: String,
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
    #[serde(default)]
    #[serde(rename(serialize = "eventFilter"))]
    pub event_filter: Option<Box<serde_json::value::RawValue>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename(serialize = "userIdFilter")
    )]
    pub user_id_filter: Option<String>,
    #[serde(default)]
    pub headers: Option<Box<serde_json::value::RawValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextCloudTriggerData {
    pub id: String,
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
