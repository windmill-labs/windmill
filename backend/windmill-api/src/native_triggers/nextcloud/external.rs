use http::StatusCode;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::HashMap;
use windmill_common::{
    error::{to_anyhow, Error, Result},
    triggers::TriggerKind,
    worker::to_raw_value,
    BASE_URL, DB,
};
use windmill_queue::PushArgsOwned;

use crate::{
    native_triggers::{
        generate_webhook_service_url,
        nextcloud::{
            routes, NextCloud, NextCloudPayload, NextCloudResource, NextCloudTriggerData,
            OcsResponse,
        },
        External, ServiceName,
    },
    triggers::trigger_helpers::TriggerJobArgs,
};

#[allow(unused)]
#[derive(Debug, Serialize)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
enum AuthMethod {
    Null,
    None,
    Header,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FullNextCloudPayload {
    pub http_method: String,
    pub uri: String,
    pub auth_method: AuthMethod,
    #[serde(flatten)]
    payload: NextCloudPayload,
}

impl FullNextCloudPayload {
    async fn new(w_id: &str, internal_id: &str, payload: NextCloudPayload) -> FullNextCloudPayload {
        let base_url = &*BASE_URL.read().await;
        let uri = generate_webhook_service_url(base_url, w_id, internal_id);

        FullNextCloudPayload {
            http_method: http::Method::POST.to_string().to_uppercase(),
            auth_method: AuthMethod::None,
            uri,
            payload,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterWebhookResponse {
    pub id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub uid: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event: EventPayload,
    pub user: User,
    pub time: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventPayload {
    pub node: Node,
    #[serde(rename = "class")]
    pub class_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: i64,
    pub path: String,
}

impl TriggerJobArgs for NextCloud {
    const TRIGGER_KIND: TriggerKind = TriggerKind::Nextcloud;
    type Payload = WebhookPayload;
    fn v1_payload_fn(payload: &Self::Payload) -> HashMap<String, Box<RawValue>> {
        HashMap::from([("payload".to_string(), to_raw_value(&payload))])
    }
}

#[async_trait::async_trait]
impl External for NextCloud {
    type Payload = NextCloudPayload;
    type TriggerData = NextCloudTriggerData;
    type Resource = NextCloudResource;
    type CreateResponse = RegisterWebhookResponse;
    const SERVICE_NAME: ServiceName = ServiceName::Nextcloud;
    const DISPLAY_NAME: &'static str = "NextCloud";
    const RESOURCE_TYPE: &'static str = "nextcloud";
    const SUPPORT_WEBHOOK: bool = true;

    async fn create(
        &self,
        w_id: &str,
        internal_id: i64,
        resource: &Self::Resource,
        payload: &Self::Payload,
    ) -> Result<Self::CreateResponse> {
        let full_nextcloud_payload =
            FullNextCloudPayload::new(w_id, &internal_id.to_string(), payload.to_owned()).await;

        let response = Client::new()
            .post(format!(
                "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks",
                resource.base_url
            ))
            .basic_auth(&resource.username, Some(&resource.password))
            .header("OCS-APIRequest", "true")
            .header("Content-Type", "application/json")
            .header("accept", "application/json")
            .json(&full_nextcloud_payload)
            .send()
            .await
            .map_err(to_anyhow)?;

        if response.status() != StatusCode::OK {
            let error_response = response
                .json::<OcsResponse<Box<serde_json::value::RawValue>>>()
                .await
                .map_err(|e| {
                    Error::InternalErr(format!("Failed to parse NextCloud response: {}", e))
                })?;

            return Err(Error::BadRequest(error_response.ocs.meta.message));
        }

        let webhook_response = response
            .json::<OcsResponse<RegisterWebhookResponse>>()
            .await
            .map_err(|e| {
                Error::InternalErr(format!("Failed to parse NextCloud response: {}", e))
            })?;

        Ok(webhook_response.ocs.data)
    }

    async fn update(
        &self,
        w_id: &str,
        internal_id: i64,
        resource: &Self::Resource,
        external_id: &str,
        payload: &Self::Payload,
    ) -> Result<()> {
        let full_nextcloud_payload =
            FullNextCloudPayload::new(w_id, &internal_id.to_string(), payload.to_owned()).await;
        let _ = Client::new()
            .post(format!(
                "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks/{}",
                resource.base_url, external_id
            ))
            .basic_auth(&resource.username, Some(&resource.password))
            .header("OCS-APIRequest", "true")
            .header("Content-Type", "application/json")
            .header("accept", "application/json")
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
            .header("accept", "application/json")
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

        let trigger = response
            .json::<Box<serde_json::value::RawValue>>()
            .await
            .map_err(|e| {
                Error::InternalErr(format!("Failed to parse NextCloud response: {}", e))
            })?;

        let trigger = serde_json::from_str::<OcsResponse<NextCloudTriggerData>>(trigger.get())?;
        Ok(trigger.ocs.data)
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

    async fn prepare_webhook(
        &self,
        db: &DB,
        w_id: &str,
        header: HashMap<String, String>,
        body: String,
        runnable_path: &str,
        is_flow: bool,
    ) -> Result<PushArgsOwned> {
        let payload = serde_json::from_str::<WebhookPayload>(&body)?;
        let job_args = Self::build_job_args(
            runnable_path,
            is_flow,
            w_id,
            &db,
            payload,
            HashMap::from([("headers".to_string(), to_raw_value(&header))]),
        )
        .await?;

        Ok(job_args)
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
            .header("accept", "application/json")
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

        let ocs_response = response
            .json::<OcsResponse<Vec<NextCloudTriggerData>>>()
            .await
            .map_err(|e| {
                Error::InternalErr(format!("Failed to parse NextCloud response: {}", e))
            })?;

        Ok(ocs_response.ocs.data)
    }

    fn external_id_and_metadata_from_response(
        &self,
        resp: &Self::CreateResponse,
    ) -> (String, Option<serde_json::Value>) {
        (resp.id.to_string(), None)
    }

    fn get_external_id_from_trigger_data(&self, data: &Self::TriggerData) -> String {
        data.id.to_string()
    }

    fn additional_routes(&self) -> axum::Router {
        routes::nextcloud_routes()
    }
}
