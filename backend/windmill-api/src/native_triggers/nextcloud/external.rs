use http::StatusCode;
use reqwest::Client;
use serde::Deserialize;
use windmill_common::error::{to_anyhow, Error, Result};

use crate::native_triggers::{
    nextcloud::{
        routes, NextCloud, NextCloudEventType, NextCloudPayload, NextCloudResource,
        NextCloudTriggerData,
    },
    External, ServiceName, TriggerMetadata,
};

async fn create_trigger(
    auth: &NextCloudResource,
    config: Option<&serde_json::Value>,
) -> Result<NextCloudTriggerData> {
    let client = Client::new();
    let response = client
        .post(format!(
            "{}/ocs/v2.php/apps/webhook/api/v1/webhooks",
            auth.base_url
        ))
        .basic_auth(&auth.username, Some(&auth.password))
        .header("OCS-APIRequest", "true")
        .header("Content-Type", "application/json")
        .json(&config)
        .send()
        .await
        .map_err(to_anyhow)?
        .error_for_status()
        .map_err(to_anyhow)?;

    let trigger = response
        .json::<NextCloudTriggerData>()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to parse NextCloud response: {}", e)))?;

    Ok(trigger)
}

async fn update_trigger(
    auth: &NextCloudResource,
    config: Option<&serde_json::Value>,
    external_id: &str,
) -> Result<NextCloudTriggerData> {
    let client = Client::new();

    let response = client
        .put(format!(
            "{}/ocs/v2.php/apps/webhook/api/v1/webhooks/{}",
            auth.base_url, external_id
        ))
        .basic_auth(&auth.username, Some(&auth.password))
        .header("OCS-APIRequest", "true")
        .header("Content-Type", "application/json")
        .json(&config)
        .send()
        .await
        .map_err(to_anyhow)?
        .error_for_status()
        .map_err(to_anyhow)?;

    let trigger: NextCloudTriggerData = response
        .json()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to parse NextCloud response: {}", e)))?;

    Ok(trigger)
}

async fn get_trigger(auth: &NextCloudResource, external_id: &str) -> Result<NextCloudTriggerData> {
    let client = Client::new();
    let response = client
        .get(format!(
            "{}/ocs/v2.php/apps/webhook/api/v1/webhooks/{}",
            auth.base_url, external_id
        ))
        .basic_auth(&auth.username, Some(&auth.password))
        .header("OCS-APIRequest", "true")
        .send()
        .await
        .map_err(to_anyhow)?;

    if response.status() == StatusCode::NOT_FOUND {
        return Err(Error::NotFound(format!(
            "Trigger {} not found on NextCloud",
            external_id
        )));
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::InternalErr(format!(
            "NextCloud API error ({}): {}",
            status, body
        )));
    }

    let trigger: NextCloudTriggerData = response
        .json()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to parse NextCloud response: {}", e)))?;

    Ok(trigger)
}

async fn delete_trigger(auth: &NextCloudResource, external_id: &str) -> Result<()> {
    let client = Client::new();
    let response = client
        .delete(format!(
            "{}/ocs/v2.php/apps/webhook/api/v1/webhooks/{}",
            auth.base_url, external_id
        ))
        .basic_auth(&auth.username, Some(&auth.password))
        .header("OCS-APIRequest", "true")
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to delete NextCloud trigger: {}", e)))?;

    if response.status() == StatusCode::NOT_FOUND {
        // Already deleted, consider it success
        return Ok(());
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::InternalErr(format!(
            "NextCloud API error ({}): {}",
            status, body
        )));
    }

    Ok(())
}

async fn trigger_exists(auth: &NextCloudResource, external_id: &str) -> Result<bool> {
    let client = Client::new();
    let response = client
        .get(format!(
            "{}/ocs/v2.php/apps/webhook/api/v1/webhooks/{}",
            auth.base_url, external_id
        ))
        .basic_auth(&auth.username, Some(&auth.password))
        .header("OCS-APIRequest", "true")
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to check NextCloud trigger: {}", e)))?;

    Ok(response.status() == StatusCode::OK)
}

async fn list_all_triggers(auth: &NextCloudResource) -> Result<Vec<NextCloudTriggerData>> {
    let client = Client::new();
    let response = client
        .get(format!(
            "{}/ocs/v2.php/apps/webhook/api/v1/webhooks",
            auth.base_url
        ))
        .basic_auth(&auth.username, Some(&auth.password))
        .header("OCS-APIRequest", "true")
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to list NextCloud triggers: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::InternalErr(format!(
            "NextCloud API error ({}): {}",
            status, body
        )));
    }

    #[derive(Deserialize)]
    struct OcsResponse {
        ocs: OcsData,
    }

    #[derive(Deserialize)]
    struct OcsData {
        data: Vec<NextCloudTriggerData>,
    }

    let ocs_response: OcsResponse = response
        .json()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to parse NextCloud response: {}", e)))?;

    Ok(ocs_response.ocs.data)
}

async fn get_available_events(auth: &NextCloudResource) -> Result<Vec<NextCloudEventType>> {
    let client = Client::new();
    let response = client
        .get(format!(
            "{}/ocs/v2.php/apps/webhook/api/v1/events",
            auth.base_url
        ))
        .basic_auth(&auth.username, Some(&auth.password))
        .header("OCS-APIRequest", "true")
        .send()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to get NextCloud events: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::InternalErr(format!(
            "NextCloud API error ({}): {}",
            status, body
        )));
    }

    #[derive(Deserialize)]
    struct OcsResponse {
        ocs: OcsData,
    }

    #[derive(Deserialize)]
    struct OcsData {
        data: Vec<NextCloudEventType>,
    }

    let ocs_response: OcsResponse = response
        .json()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to parse NextCloud response: {}", e)))?;

    Ok(ocs_response.ocs.data)
}

#[async_trait::async_trait]
impl External for NextCloud {
    type Payload = NextCloudPayload;
    type TriggerData = NextCloudTriggerData;
    type Resource = NextCloudResource;
    const SERVICE_NAME: ServiceName = ServiceName::Nextcloud;
    const DISPLAY_NAME: &'static str = "NextCloud";
    const RESOURCE_TYPE: &'static str = "nextcloud";

    async fn create(&self, resource: &Self::Resource, payload: &Self::Payload) -> Result<()> {
        let _ = Client::new()
            .post(format!(
                "{}/ocs/v2.php/apps/webhook/api/v1/webhooks",
                resource.base_url
            ))
            .basic_auth(&resource.username, Some(&resource.password))
            .header("OCS-APIRequest", "true")
            .header("Content-Type", "application/json")
            .json(&payload.config)
            .send()
            .await
            .map_err(to_anyhow)?
            .error_for_status()
            .map_err(to_anyhow)?;

        Ok(())
    }

    async fn update(
        &self,
        resource: &Self::Resource,
        external_id: &str,
        payload: &Self::Payload,
    ) -> Result<()> {
        let _ = Client::new()
            .put(format!(
                "{}/ocs/v2.php/apps/webhook/api/v1/webhooks/{}",
                resource.base_url, external_id
            ))
            .basic_auth(&resource.username, Some(&resource.password))
            .header("OCS-APIRequest", "true")
            .header("Content-Type", "application/json")
            .json(&payload.config)
            .send()
            .await
            .map_err(to_anyhow)?
            .error_for_status()
            .map_err(to_anyhow)?;

        Ok(())
    }

    async fn get(&self, resource: &Self::Resource, external_id: &str) -> Result<Self::TriggerData> {
        let response = Client::new()
            .get(format!(
                "{}/ocs/v2.php/apps/webhook/api/v1/webhooks/{}",
                resource.base_url, external_id
            ))
            .basic_auth(&resource.username, Some(&resource.password))
            .header("OCS-APIRequest", "true")
            .send()
            .await
            .map_err(|e| Error::InternalErr(format!("Failed to get NextCloud trigger: {}", e)))?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(Error::NotFound(format!(
                "Trigger {} not found on NextCloud",
                external_id
            )));
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::InternalErr(format!(
                "NextCloud API error ({}): {}",
                status, body
            )));
        }

        let trigger: NextCloudTriggerData = response.json().await.map_err(|e| {
            Error::InternalErr(format!("Failed to parse NextCloud response: {}", e))
        })?;

        Ok(trigger)
    }

    async fn delete(&self, resource: &Self::Resource, external_id: &str) -> Result<()> {
        let response = Client::new()
            .delete(format!(
                "{}/ocs/v2.php/apps/webhook/api/v1/webhooks/{}",
                resource.base_url, external_id
            ))
            .basic_auth(&resource.username, Some(&resource.password))
            .header("OCS-APIRequest", "true")
            .send()
            .await
            .map_err(|e| {
                Error::InternalErr(format!("Failed to delete NextCloud trigger: {}", e))
            })?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::InternalErr(format!(
                "NextCloud API error ({}): {}",
                status, body
            )));
        }

        Ok(())
    }

    async fn exists(&self, resource: &Self::Resource, external_id: &str) -> Result<bool> {
        let response = Client::new()
            .get(format!(
                "{}/ocs/v2.php/apps/webhook/api/v1/webhooks/{}",
                resource.base_url, external_id
            ))
            .basic_auth(&resource.username, Some(&resource.password))
            .header("OCS-APIRequest", "true")
            .send()
            .await
            .map_err(|e| Error::InternalErr(format!("Failed to check NextCloud trigger: {}", e)))?;

        Ok(response.status() == StatusCode::OK)
    }

    fn extract_metadata_from_payload(
        &self,
        payload: &Self::Payload,
        external_id: Option<&str>,
    ) -> Result<TriggerMetadata> {
        todo!()
    }

    async fn list_all(&self, resource: &Self::Resource) -> Result<Vec<Self::TriggerData>> {
        let response = Client::new()
            .get(format!(
                "{}/ocs/v2.php/apps/webhook/api/v1/webhooks",
                resource.base_url
            ))
            .basic_auth(&resource.username, Some(&resource.password))
            .header("OCS-APIRequest", "true")
            .send()
            .await
            .map_err(|e| Error::InternalErr(format!("Failed to list NextCloud triggers: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::InternalErr(format!(
                "NextCloud API error ({}): {}",
                status, body
            )));
        }

        #[derive(Deserialize)]
        struct OcsResponse {
            ocs: OcsData,
        }

        #[derive(Deserialize)]
        struct OcsData {
            data: Vec<NextCloudTriggerData>,
        }

        let ocs_response: OcsResponse = response.json().await.map_err(|e| {
            Error::InternalErr(format!("Failed to parse NextCloud response: {}", e))
        })?;

        Ok(ocs_response.ocs.data)
    }

    fn additional_routes(&self) -> axum::Router {
        routes::nextcloud_routes()
    }
}
