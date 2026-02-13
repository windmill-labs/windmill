use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::value::to_raw_value;
use sqlx::PgConnection;
use std::collections::HashMap;
use windmill_common::{
    error::{Error, Result},
    BASE_URL, DB,
};

use crate::{
    generate_webhook_service_url,
    nextcloud::{
        routes, NextCloud, NextCloudOAuthData, NextCloudTriggerData, NextcloudServiceConfig,
        OcsResponse,
    },
    External, NativeTriggerData, ServiceName,
};

lazy_static::lazy_static! {
    pub static ref TOKEN_NEEDED: Box<serde_json::value::RawValue> = to_raw_value(&serde_json::json!({
        "user_roles": ["owner", "trigger"]
    })).unwrap();
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FullNextcloudPayload {
    pub http_method: String,
    pub uri: String,
    pub token_needed: Box<serde_json::value::RawValue>,
    #[serde(flatten)]
    service_config: NextcloudServiceConfig,
}

impl FullNextcloudPayload {
    async fn new(
        w_id: &str,
        external_id: Option<&str>,
        webhook_token: &str,
        data: &NativeTriggerData<NextcloudServiceConfig>,
    ) -> FullNextcloudPayload {
        let base_url = &*BASE_URL.read().await;
        let uri = generate_webhook_service_url(
            base_url,
            w_id,
            &data.script_path,
            data.is_flow,
            external_id,
            ServiceName::Nextcloud,
            webhook_token,
        );

        FullNextcloudPayload {
            http_method: http::Method::POST.to_string().to_uppercase(),
            uri,
            token_needed: TOKEN_NEEDED.clone(),
            service_config: data.service_config.clone(),
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

#[async_trait::async_trait]
impl External for NextCloud {
    type ServiceConfig = NextcloudServiceConfig;
    type TriggerData = NextCloudTriggerData;
    type OAuthData = NextCloudOAuthData;
    type CreateResponse = RegisterWebhookResponse;
    const SERVICE_NAME: ServiceName = ServiceName::Nextcloud;
    const DISPLAY_NAME: &'static str = "Nextcloud";
    const SUPPORT_WEBHOOK: bool = true;
    const TOKEN_ENDPOINT: &'static str = "/apps/oauth2/api/v1/token";
    const REFRESH_ENDPOINT: &'static str = "/apps/oauth2/api/v1/token";
    const AUTH_ENDPOINT: &'static str = "/oauth/authorize";

    async fn create(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        webhook_token: &str,
        data: &NativeTriggerData<Self::ServiceConfig>,
        db: &DB,
        _tx: &mut PgConnection,
    ) -> Result<Self::CreateResponse> {
        // During create, we don't have external_id yet (it comes from NextCloud's response)
        let full_nextcloud_payload =
            FullNextcloudPayload::new(w_id, None, webhook_token, data).await;

        let url = format!(
            "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks",
            oauth_data.base_url
        );

        let mut headers = HashMap::new();
        headers.insert("OCS-APIRequest".to_string(), "true".to_string());

        let ocs_response = self
            .http_client_request::<OcsResponse<RegisterWebhookResponse>, _>(
                &url,
                Method::POST,
                w_id,
                db,
                Some(headers),
                Some(&full_nextcloud_payload),
            )
            .await?;

        Ok(ocs_response.ocs.data)
    }

    async fn update(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        webhook_token: &str,
        data: &NativeTriggerData<Self::ServiceConfig>,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<serde_json::Value> {
        // During update, we have the external_id so include it in the webhook URL
        let full_nextcloud_payload =
            FullNextcloudPayload::new(w_id, Some(external_id), webhook_token, data).await;

        let url = format!(
            "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks/{}",
            oauth_data.base_url, external_id
        );

        let mut headers = HashMap::new();
        headers.insert("OCS-APIRequest".to_string(), "true".to_string());

        let _ = self
            .http_client_request::<serde_json::Value, _>(
                &url,
                Method::POST,
                w_id,
                db,
                Some(headers),
                Some(&full_nextcloud_payload),
            )
            .await?;

        // Fetch back the updated state and convert to JSON config
        let trigger_data = self
            .get(w_id, oauth_data, external_id, db, tx)
            .await?
            .ok_or_else(|| {
                Error::InternalErr(format!(
                    "Failed to fetch back trigger {} after update",
                    external_id
                ))
            })?;
        serde_json::to_value(&trigger_data).map_err(|e| {
            Error::internal_err(format!("Failed to convert trigger data to JSON: {}", e))
        })
    }

    async fn get(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        _tx: &mut PgConnection,
    ) -> Result<Option<Self::TriggerData>> {
        let url = format!(
            "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks/{}",
            oauth_data.base_url, external_id
        );

        let mut headers = HashMap::new();
        headers.insert("OCS-APIRequest".to_string(), "true".to_string());

        let ocs_response: OcsResponse<NextCloudTriggerData> = self
            .http_client_request::<_, ()>(&url, Method::GET, w_id, db, Some(headers), None)
            .await?;

        Ok(Some(ocs_response.ocs.data))
    }

    async fn delete(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        _tx: &mut PgConnection,
    ) -> Result<()> {
        let url = format!(
            "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks/{}",
            oauth_data.base_url, external_id
        );

        let mut headers = HashMap::new();
        headers.insert("OCS-APIRequest".to_string(), "true".to_string());

        let _: serde_json::Value = self
            .http_client_request::<_, ()>(&url, Method::DELETE, w_id, db, Some(headers), None)
            .await
            .or_else(|e| match &e {
                Error::InternalErr(msg) if msg.contains("404") => Ok(serde_json::Value::Null),
                _ => Err(e),
            })?;

        Ok(())
    }

    async fn maintain_triggers(
        &self,
        db: &DB,
        workspace_id: &str,
        triggers: &[crate::NativeTrigger],
        oauth_data: &Self::OAuthData,
        synced: &mut Vec<crate::sync::TriggerSyncInfo>,
        errors: &mut Vec<crate::sync::SyncError>,
    ) {
        let external_triggers = match self.list_all(workspace_id, oauth_data, db).await {
            Ok(triggers) => triggers,
            Err(e) => {
                tracing::error!(
                    "Failed to fetch external triggers for {}: {}",
                    workspace_id,
                    e
                );
                errors.push(crate::sync::SyncError {
                    resource_path: format!("workspace:{}", workspace_id),
                    error_message: format!("Failed to fetch external triggers: {}", e),
                    error_type: "external_service_error".to_string(),
                });
                return;
            }
        };

        // Convert to (external_id, config_json) pairs for reconciliation
        let external_pairs: Vec<(String, serde_json::Value)> = external_triggers
            .iter()
            .filter_map(|data| {
                let config = serde_json::to_value(data).ok()?;
                Some((data.id.to_string(), config))
            })
            .collect();

        crate::sync::reconcile_with_external_state(
            db,
            workspace_id,
            ServiceName::Nextcloud,
            triggers,
            &external_pairs,
            synced,
            errors,
        )
        .await;
    }

    fn external_id_and_metadata_from_response(
        &self,
        resp: &Self::CreateResponse,
    ) -> (String, Option<serde_json::Value>) {
        (resp.id.to_string(), None)
    }

    fn additional_routes(&self) -> axum::Router {
        routes::nextcloud_routes(self.clone())
    }
}

impl NextCloud {
    async fn list_all(
        &self,
        w_id: &str,
        oauth_data: &NextCloudOAuthData,
        db: &DB,
    ) -> Result<Vec<NextCloudTriggerData>> {
        let url = format!(
            "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks",
            oauth_data.base_url
        );

        let mut headers = HashMap::new();
        headers.insert("OCS-APIRequest".to_string(), "true".to_string());

        let ocs_response = self
            .http_client_request::<OcsResponse<Vec<NextCloudTriggerData>>, ()>(
                &url,
                Method::GET,
                w_id,
                db,
                Some(headers),
                None,
            )
            .await?;

        Ok(ocs_response.ocs.data)
    }
}
