use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use windmill_common::error::{Error, Result};

use crate::native_triggers::{External, ServiceName, TriggerMetadata};

mod client;
mod routes;

pub use client::{NextCloudClient, NextCloudEventType};

#[derive(Copy, Clone)]
pub struct NextCloudHandler;

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

#[async_trait]
impl External for NextCloudHandler {
    type Payload = NextCloudPayload;
    type TriggerData = NextCloudTriggerData;
    type Resource = client::NextCloudResource;

    const SERVICE_NAME: ServiceName = ServiceName::Nextcloud;
    const DISPLAY_NAME: &'static str = "NextCloud";
    const RESOURCE_TYPE: &'static str = "nextcloud";

    async fn create(
        &self,
        resource: &Self::Resource,
        path: &str,
        payload: &Self::Payload,
    ) -> Result<TriggerMetadata> {
        let trigger = NextCloudClient::create_trigger(
            resource,
            &payload.event_type,
            path,
            payload.file_path.as_deref(),
            payload.config.as_ref(),
        )
        .await?;

        Ok(TriggerMetadata {
            external_id: trigger.id,
            resource_path: payload.nextcloud_resource_path.clone(),
            summary: format!(
                "NextCloud {} trigger{}",
                payload.event_type,
                payload
                    .file_path
                    .as_ref()
                    .map(|p| format!(" for {}", p))
                    .unwrap_or_default()
            ),
            metadata: Some(serde_json::json!({
                "event_type": payload.event_type,
                "file_path": payload.file_path,
                "status": trigger.status,
            })),
        })
    }

    async fn update(
        &self,
        resource: &Self::Resource,
        external_id: &str,
        path: &str,
        payload: &Self::Payload,
    ) -> Result<TriggerMetadata> {
        let trigger = NextCloudClient::update_trigger(
            resource,
            external_id,
            &payload.event_type,
            path,
            payload.file_path.as_deref(),
            payload.config.as_ref(),
        )
        .await?;

        Ok(TriggerMetadata {
            external_id: trigger.id,
            resource_path: payload.nextcloud_resource_path.clone(),
            summary: format!(
                "NextCloud {} trigger{}",
                payload.event_type,
                payload
                    .file_path
                    .as_ref()
                    .map(|p| format!(" for {}", p))
                    .unwrap_or_default()
            ),
            metadata: Some(serde_json::json!({
                "event_type": payload.event_type,
                "file_path": payload.file_path,
                "status": trigger.status,
            })),
        })
    }

    async fn get(
        &self,
        resource: &Self::Resource,
        external_id: &str,
    ) -> Result<Self::TriggerData> {
        NextCloudClient::get_trigger(resource, external_id).await
    }

    async fn delete(
        &self,
        resource: &Self::Resource,
        external_id: &str,
    ) -> Result<()> {
        NextCloudClient::delete_trigger(resource, external_id).await
    }

    async fn exists(
        &self,
        resource: &Self::Resource,
        external_id: &str,
    ) -> Result<bool> {
        NextCloudClient::trigger_exists(resource, external_id).await
    }

    fn extract_metadata_from_payload(
        &self,
        payload: &Self::Payload,
        external_id: Option<&str>,
    ) -> Result<TriggerMetadata> {
        Ok(TriggerMetadata {
            external_id: external_id
                .ok_or_else(|| Error::BadRequest("External ID is required".to_string()))?
                .to_string(),
            resource_path: payload.nextcloud_resource_path.clone(),
            summary: format!(
                "NextCloud {} trigger{}",
                payload.event_type,
                payload
                    .file_path
                    .as_ref()
                    .map(|p| format!(" for {}", p))
                    .unwrap_or_default()
            ),
            metadata: Some(serde_json::json!({
                "event_type": payload.event_type,
                "file_path": payload.file_path,
            })),
        })
    }

    async fn list_all(
        &self,
        resource: &Self::Resource,
    ) -> Result<Vec<Self::TriggerData>> {
        NextCloudClient::list_all_triggers(resource).await
    }

    fn additional_routes(&self) -> axum::Router {
        routes::nextcloud_routes()
    }
}
