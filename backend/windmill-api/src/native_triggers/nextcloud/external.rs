use http::StatusCode;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use windmill_common::{
    error::{to_anyhow, Error, Result},
    BASE_URL,
};

use crate::native_triggers::{
    nextcloud::{routes, NextCloud, NextCloudPayload, NextCloudResource, NextCloudTriggerData},
    External, ServiceName,
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

#[derive(Debug, Serialize)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
enum AuthMethod {
    None,
    Bearer,
}

#[derive(Debug, Serialize)]
struct FullNextCloudPayload {
    pub http_method: String,
    pub uri: String,
    pub auth_method: AuthMethod,
    #[serde(flatten)]
    payload: NextCloudPayload,
}

impl FullNextCloudPayload {
    async fn new(
        w_id: &str,
        runnable_path: &str,
        payload: NextCloudPayload,
    ) -> FullNextCloudPayload {
        let base_url = &*BASE_URL.read().await;
        let uri = format!(
            "{}/api/w/{}/native_triggers/nextcloud/webhook/{}",
            base_url, w_id, runnable_path
        );
        FullNextCloudPayload {
            http_method: http::Method::POST.to_string().to_uppercase(),
            auth_method: AuthMethod::Bearer,
            uri,
            payload,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateWebhookResponse {
    pub id: String,
}

#[async_trait::async_trait]
impl External for NextCloud {
    type Payload = NextCloudPayload;
    type TriggerData = NextCloudTriggerData;
    type Resource = NextCloudResource;
    type CreateResponse = CreateWebhookResponse;
    const SERVICE_NAME: ServiceName = ServiceName::Nextcloud;
    const DISPLAY_NAME: &'static str = "NextCloud";
    const RESOURCE_TYPE: &'static str = "nextcloud";
    const SUPPORT_WEBHOOK: bool = true;

    async fn create(
        &self,
        w_id: &str,
        runnable_path: &str,
        resource: &Self::Resource,
        payload: &Self::Payload,
    ) -> Result<Self::CreateResponse> {
        let full_nextcloud_payload =
            FullNextCloudPayload::new(w_id, runnable_path, payload.to_owned()).await;
        let response = Client::new()
            .post(format!(
                "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks",
                resource.base_url
            ))
            .basic_auth(&resource.username, Some(&resource.password))
            .header("OCS-APIRequest", "true")
            .header("Content-Type", "application/json")
            .json(&full_nextcloud_payload)
            .send()
            .await
            .map_err(to_anyhow)?
            .error_for_status()
            .map_err(to_anyhow)?;

        let webhook_response = response.json::<Self::CreateResponse>().await.map_err(|e| {
            Error::InternalErr(format!("Failed to parse NextCloud response: {}", e))
        })?;

        Ok(webhook_response)
    }

    async fn update(
        &self,
        w_id: &str,
        runnable_path: &str,
        resource: &Self::Resource,
        external_id: &str,
        payload: &Self::Payload,
    ) -> Result<()> {
        let full_nextcloud_payload =
            FullNextCloudPayload::new(w_id, runnable_path, payload.to_owned()).await;
        let _ = Client::new()
            .post(format!(
                "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks/{}",
                resource.base_url, external_id
            ))
            .basic_auth(&resource.username, Some(&resource.password))
            .header("OCS-APIRequest", "true")
            .header("Content-Type", "application/json")
            .json(&full_nextcloud_payload)
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
                "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks/{}",
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
                "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks/{}",
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
                "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks/{}",
                resource.base_url, external_id
            ))
            .basic_auth(&resource.username, Some(&resource.password))
            .header("OCS-APIRequest", "true")
            .send()
            .await
            .map_err(|e| Error::InternalErr(format!("Failed to check NextCloud trigger: {}", e)))?;

        Ok(response.status() == StatusCode::OK)
    }

    async fn list_all(&self, resource: &Self::Resource) -> Result<Vec<Self::TriggerData>> {
        let response = Client::new()
            .get(format!(
                "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks",
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

    fn external_id_and_metadata_from_response(
        &self,
        resp: &Self::CreateResponse,
    ) -> (String, Option<serde_json::Value>) {
        (resp.id.clone(), None)
    }

    fn additional_routes(&self) -> axum::Router {
        routes::nextcloud_routes()
    }
}
