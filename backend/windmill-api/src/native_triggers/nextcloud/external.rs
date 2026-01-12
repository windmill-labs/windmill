use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::value::{to_raw_value, RawValue};
use sqlx::PgConnection;
use std::collections::HashMap;
use windmill_common::{
    error::{Error, Result},
    triggers::TriggerKind,
    BASE_URL, DB,
};

use crate::{
    native_triggers::{
        generate_webhook_service_url,
        nextcloud::{
            routes, NextCloud, NextCloudOAuthData, NextCloudPayload, NextCloudTriggerData,
            OcsResponse,
        },
        EventType, External, NativeTriggerData, ServiceName,
    },
    triggers::trigger_helpers::TriggerJobArgs,
};

lazy_static::lazy_static! {
    pub static ref TOKEN_NEEDED: Box<serde_json::value::RawValue> = to_raw_value(&serde_json::json!({
        "user_roles": ["owner", "trigger"]
    })).unwrap();
}

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
    pub token_needed: Box<serde_json::value::RawValue>,
    #[serde(flatten)]
    payload: NextCloudPayload,
}

impl FullNextCloudPayload {
    async fn new(
        w_id: &str,
        external_id: Option<&str>,
        data: &NativeTriggerData<NextCloudPayload>,
    ) -> FullNextCloudPayload {
        let EventType::Webhook(webhook_config) = &data.event_type;
        let base_url = &*BASE_URL.read().await;
        let uri = generate_webhook_service_url(
            base_url,
            w_id,
            &data.script_path,
            data.is_flow,
            external_id,
            ServiceName::Nextcloud,
            webhook_config,
        );

        FullNextCloudPayload {
            http_method: http::Method::POST.to_string().to_uppercase(),
            auth_method: AuthMethod::None,
            uri,
            token_needed: TOKEN_NEEDED.clone(),
            payload: data.payload.clone(),
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
    type Payload = Box<RawValue>;
    fn v1_payload_fn(payload: &Self::Payload) -> HashMap<String, Box<RawValue>> {
        HashMap::from([("payload".to_owned(), payload.to_owned())])
    }
}

#[async_trait::async_trait]
impl External for NextCloud {
    type Payload = NextCloudPayload;
    type TriggerData = NextCloudTriggerData;
    type OAuthData = NextCloudOAuthData;
    type CreateResponse = RegisterWebhookResponse;
    const SERVICE_NAME: ServiceName = ServiceName::Nextcloud;
    const DISPLAY_NAME: &'static str = "NextCloud";
    const SUPPORT_WEBHOOK: bool = true;
    const TOKEN_ENDPOINT: &'static str = "/apps/oauth2/api/v1/token";
    const REFRESH_ENDPOINT: &'static str = "/apps/oauth2/api/v1/token";

    async fn create(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        data: &NativeTriggerData<Self::Payload>,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<Self::CreateResponse> {
        // During create, we don't have external_id yet (it comes from NextCloud's response)
        let full_nextcloud_payload = FullNextCloudPayload::new(w_id, None, data).await;

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
                tx,
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
        data: &NativeTriggerData<Self::Payload>,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<()> {
        // During update, we have the external_id so include it in the webhook URL
        let full_nextcloud_payload = FullNextCloudPayload::new(w_id, Some(external_id), data).await;

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
                tx,
                db,
                Some(headers),
                Some(&full_nextcloud_payload),
            )
            .await?;

        Ok(())
    }

    async fn validate_data_config(&self, data: &NativeTriggerData<Self::Payload>) -> Result<()> {
        let event_type = &data.event_type;
        if !matches!(event_type, &EventType::Webhook(_)) {
            return Err(Error::BadRequest(
                "Nextcloud native trigger only support webhook event".to_string(),
            ));
        }

        return Ok(());
    }

    async fn get(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<Self::TriggerData> {
        let url = format!(
            "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks/{}",
            oauth_data.base_url, external_id
        );

        let mut headers = HashMap::new();
        headers.insert("OCS-APIRequest".to_string(), "true".to_string());

        let ocs_response: OcsResponse<NextCloudTriggerData> = self
            .http_client_request::<_, ()>(&url, Method::GET, w_id, tx, db, Some(headers), None)
            .await?;

        Ok(ocs_response.ocs.data)
    }

    async fn delete(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<()> {
        let url = format!(
            "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks/{}",
            oauth_data.base_url, external_id
        );

        let mut headers = HashMap::new();
        headers.insert("OCS-APIRequest".to_string(), "true".to_string());

        let _: serde_json::Value = self
            .http_client_request::<_, ()>(&url, Method::DELETE, w_id, tx, db, Some(headers), None)
            .await
            .or_else(|e| match &e {
                Error::InternalErr(msg) if msg.contains("404") => Ok(serde_json::Value::Null),
                _ => Err(e),
            })?;

        Ok(())
    }

    async fn exists(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<bool> {
        let url = format!(
            "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks/{}",
            oauth_data.base_url, external_id
        );

        let mut headers = HashMap::new();
        headers.insert("OCS-APIRequest".to_string(), "true".to_string());

        let _ = self
            .http_client_request::<serde_json::Value, ()>(
                &url,
                Method::GET,
                w_id,
                tx,
                db,
                Some(headers),
                None,
            )
            .await?;

        Ok(true)
    }

    async fn list_all(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<Vec<Self::TriggerData>> {
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
                tx,
                db,
                Some(headers),
                None,
            )
            .await?;

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

    fn extract_service_config_from_payload(&self, payload: &Self::Payload) -> Box<RawValue> {
        to_raw_value(&serde_json::json!({
            "event": payload.event,
            "event_filter": payload.event_filter.as_ref().map(|v| v.get()),
            "user_id_filter": payload.user_id_filter,
        }))
        .unwrap()
    }

    fn extract_service_config_from_trigger_data(&self, data: &Self::TriggerData) -> Box<RawValue> {
        to_raw_value(&serde_json::json!({
            "event": data.event,
            "event_filter": data.event_filter.as_ref().map(|v| v.get()),
            "user_id_filter": data.user_id_filter,
        }))
        .unwrap()
    }

    fn additional_routes(&self) -> axum::Router {
        routes::nextcloud_routes(self.clone())
    }
}
