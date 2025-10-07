use once_cell::sync::Lazy;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use windmill_common::{
    error::{Error, Result},
    DB,
};

use super::NextCloudTriggerData;

/// Shared HTTP client for all NextCloud requests (created once, reused forever)
static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| Client::new());

/// NextCloud resource schema
#[derive(Debug, Clone, Deserialize)]
pub struct NextCloudResource {
    pub base_url: String,
    pub username: String,
    pub password: String,
}

/// NextCloud API client - stateless methods that operate on shared HTTP client
#[derive(Copy, Clone)]
pub struct NextCloudClient;

#[derive(Debug, Serialize)]
struct CreateTriggerRequest {
    event_type: String,
    callback_url: String,
    resource_path: Option<String>,
    config: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct UpdateTriggerRequest {
    event_type: String,
    callback_url: String,
    resource_path: Option<String>,
    config: Option<serde_json::Value>,
}

impl NextCloudClient {
    fn build_callback_url(path: &str) -> String {
        let windmill_base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
        format!("{}/api/webhooks/nextcloud/{}", windmill_base_url, path)
    }

    pub async fn create_trigger(
        auth: &NextCloudResource,
        event_type: &str,
        path: &str,
        resource_path: Option<&str>,
        config: Option<&serde_json::Value>,
    ) -> Result<NextCloudTriggerData> {
        let callback_url = Self::build_callback_url(path);

        let request = CreateTriggerRequest {
            event_type: event_type.to_string(),
            callback_url,
            resource_path: resource_path.map(String::from),
            config: config.cloned(),
        };

        let response = HTTP_CLIENT
            .post(format!(
                "{}/ocs/v2.php/apps/webhook/api/v1/webhooks",
                auth.base_url
            ))
            .basic_auth(&auth.username, Some(&auth.password))
            .header("OCS-APIRequest", "true")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                Error::InternalErr(format!("Failed to create NextCloud trigger: {}", e))
            })?;

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

    pub async fn update_trigger(
        auth: &NextCloudResource,
        external_id: &str,
        event_type: &str,
        path: &str,
        resource_path: Option<&str>,
        config: Option<&serde_json::Value>,
    ) -> Result<NextCloudTriggerData> {
        let callback_url = Self::build_callback_url(path);

        let request = UpdateTriggerRequest {
            event_type: event_type.to_string(),
            callback_url,
            resource_path: resource_path.map(String::from),
            config: config.cloned(),
        };

        let response = HTTP_CLIENT
            .put(format!(
                "{}/ocs/v2.php/apps/webhook/api/v1/webhooks/{}",
                auth.base_url, external_id
            ))
            .basic_auth(&auth.username, Some(&auth.password))
            .header("OCS-APIRequest", "true")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                Error::InternalErr(format!("Failed to update NextCloud trigger: {}", e))
            })?;

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

    pub async fn get_trigger(auth: &NextCloudResource, external_id: &str) -> Result<NextCloudTriggerData> {
        let response = HTTP_CLIENT
            .get(format!(
                "{}/ocs/v2.php/apps/webhook/api/v1/webhooks/{}",
                auth.base_url, external_id
            ))
            .basic_auth(&auth.username, Some(&auth.password))
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

    pub async fn delete_trigger(auth: &NextCloudResource, external_id: &str) -> Result<()> {
        let response = HTTP_CLIENT
            .delete(format!(
                "{}/ocs/v2.php/apps/webhook/api/v1/webhooks/{}",
                auth.base_url, external_id
            ))
            .basic_auth(&auth.username, Some(&auth.password))
            .header("OCS-APIRequest", "true")
            .send()
            .await
            .map_err(|e| {
                Error::InternalErr(format!("Failed to delete NextCloud trigger: {}", e))
            })?;

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

    pub async fn trigger_exists(auth: &NextCloudResource, external_id: &str) -> Result<bool> {
        let response = HTTP_CLIENT
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

    pub async fn list_all_triggers(auth: &NextCloudResource) -> Result<Vec<NextCloudTriggerData>> {
        let response = HTTP_CLIENT
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

        let ocs_response: OcsResponse = response.json().await.map_err(|e| {
            Error::InternalErr(format!("Failed to parse NextCloud response: {}", e))
        })?;

        Ok(ocs_response.ocs.data)
    }

    pub async fn get_available_events(auth: &NextCloudResource) -> Result<Vec<NextCloudEventType>> {
        let response = HTTP_CLIENT
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

        let ocs_response: OcsResponse = response.json().await.map_err(|e| {
            Error::InternalErr(format!("Failed to parse NextCloud response: {}", e))
        })?;

        Ok(ocs_response.ocs.data)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextCloudEventType {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
}
