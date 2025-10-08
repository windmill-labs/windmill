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
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
}

#[derive(Copy, Clone)]
pub struct NextCloud;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextCloudPayload {
    pub event_type: String,
    pub nextcloud_resource_path: String,
    pub file_path: Option<String>,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextCloudTriggerData {
    pub id: String,
    pub event_type: String,
    pub resource_path: Option<String>,
    pub config: Option<serde_json::Value>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}
