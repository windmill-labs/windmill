use std::collections::HashMap;

use axum::extract::{FromRequest, Request};
use base64::{prelude::BASE64_STANDARD, Engine};
use bytes::Bytes;
use google_cloud_googleapis::pubsub::v1::{
    push_config::{AuthenticationMethod, OidcToken, PubsubWrapper, Wrapper},
    PushConfig as GooglePubSubPushConfig,
};
use google_cloud_pubsub::{
    client::{google_cloud_auth::credentials::CredentialsFile, Client, ClientConfig},
    subscription::{SubscriptionConfig, SubscriptionConfigToUpdate},
};
use http::HeaderMap;
use jsonwebtoken::{DecodingKey, Validation};
use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use serde_json::{value::RawValue, Value};
use sqlx::{types::Json as SqlxJson, FromRow};
use windmill_common::{
    db::UserDB,
    error::{to_anyhow, Error, Result},
    utils::empty_as_none,
    worker::to_raw_value,
    DB,
};

lazy_static::lazy_static! {
    pub static ref GOOGLE_PUBLIC_KEY_CACHE: Cache<String, String> = Cache::new(500);
}

use crate::{db::ApiAuthed, resources::try_get_resource_from_db_as};

mod handler_ee;
pub mod handler_oss;

#[derive(Debug, Deserialize, Serialize, Clone, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "delivery_mode", rename_all = "lowercase")]
pub enum DeliveryMode {
    Push,
    Pull,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct GcpConfig {
    pub gcp_resource_path: String,
    pub topic_id: String,
    pub subscription_id: String,
    pub delivery_type: DeliveryMode,
    pub delivery_config: Option<SqlxJson<PushConfig>>,
    pub subscription_mode: GcpSubscriptionMode,
}

#[derive(Default, Clone, FromRow, Deserialize, Serialize, Debug)]
pub struct PushConfig {
    #[serde(default, deserialize_with = "empty_as_none")]
    audience: Option<String>,
    authenticate: bool,
}

#[derive(sqlx::Type, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
#[sqlx(type_name = "DELIVERY_MODE", rename_all = "lowercase")]
pub enum DeliveryType {
    Pull,
    Push,
}

impl Default for DeliveryType {
    fn default() -> Self {
        Self::Pull
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct CreateUpdateConfig {
    pub delivery_type: DeliveryType,
    pub delivery_config: Option<SqlxJson<PushConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewGcpConfig {
    pub gcp_resource_path: String,
    pub topic_id: String,
    #[serde(default, deserialize_with = "empty_as_none")]
    subscription_id: Option<String>,
    #[serde(default, deserialize_with = "empty_as_none")]
    pub base_endpoint: Option<String>,
    pub subscription_mode: GcpSubscriptionMode,
    #[serde(flatten)]
    pub create_update_config: Option<CreateUpdateConfig>,
    pub auto_acknowledge_msg: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditGcpConfig {
    pub gcp_resource_path: String,
    pub topic_id: String,
    #[serde(default, deserialize_with = "empty_as_none")]
    subscription_id: Option<String>,
    #[serde(default, deserialize_with = "empty_as_none")]
    pub base_endpoint: Option<String>,
    pub subscription_mode: GcpSubscriptionMode,
    #[serde(flatten)]
    pub create_update_config: Option<CreateUpdateConfig>,
    auto_acknowledge_msg: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGcpConfig {
    pub gcp_resource_path: String,
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
struct TriggerPushData {
    gcp_resource_path: String,
    path: String,
    script_path: String,
    is_flow: bool,
    workspace_id: String,
    edited_by: String,
    email: String,
    delivery_config: Option<SqlxJson<PushConfig>>,
    retry: Option<SqlxJson<windmill_common::flows::Retry>>,
    error_handler_path: Option<String>,
    error_handler_args: Option<SqlxJson<HashMap<String, Box<RawValue>>>>,
}

#[derive(Debug, Deserialize)]
struct ServiceAccount {
    pub project_id: String,
    pub private_key: String,
    pub client_email: String,
    pub private_key_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
}

impl From<ServiceAccount> for CredentialsFile {
    fn from(value: ServiceAccount) -> Self {
        CredentialsFile {
            tp: "service_account".to_string(),
            client_email: Some(value.client_email),
            private_key_id: Some(value.private_key_id),
            private_key: Some(value.private_key),
            auth_uri: Some(value.auth_uri),
            token_uri: Some(value.token_uri),
            project_id: Some(value.project_id),
            client_secret: None,
            client_id: None,
            refresh_token: None,
            audience: None,
            subject_token_type: None,
            token_url_external: None,
            token_info_url: None,
            service_account_impersonation_url: None,
            service_account_impersonation: None,
            delegates: None,
            credential_source: None,
            quota_project_id: None,
            workforce_pool_user_project: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "GCP_SUBSCRIPTION_MODE", rename_all = "snake_case")]
pub enum GcpSubscriptionMode {
    Existing,
    CreateUpdate,
}

pub fn validate_topic_id(topic_id: &str) -> windmill_common::error::Result<()> {
    if topic_id.trim().is_empty() {
        return Err(windmill_common::error::Error::BadRequest(
            "Topic ID cannot be empty".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_gcp_resource_path(path: &str) -> windmill_common::error::Result<()> {
    if path.trim().is_empty() {
        return Err(windmill_common::error::Error::BadRequest(
            "GCP resource path cannot be empty".to_string(),
        ));
    }
    Ok(())
}

async fn get_gcp_client(
    authed: ApiAuthed,
    db: &DB,
    gcp_resource_path: &str,
    w_id: &str,
) -> Result<(Client, String)> {
    let mut service_account = try_get_resource_from_db_as::<ServiceAccount>(
        &authed,
        Some(UserDB::new(db.clone())),
        &db,
        gcp_resource_path,
        w_id,
    )
    .await?;
    let email = service_account.client_email.clone();
    service_account.private_key =
        serde_json::from_str(&format!("\"{}\"", service_account.private_key)).map_err(to_anyhow)?;
    let credentials = service_account.into();
    let config = ClientConfig::default()
        .with_credentials(credentials)
        .await
        .map_err(|e| Error::BadGateway(e.to_string()))?;

    let client = Client::new(config)
        .await
        .map_err(|e| Error::BadGateway(e.to_string()))?;

    Ok((client, email))
}

#[inline]
fn generate_endpoint(
    base_endpoint: &str,
    workspace_id: &str,
    trigger_mode: bool,
    is_flow: bool,
) -> String {
    if trigger_mode {
        return format!("{}/api/gcp/w/{}", base_endpoint, workspace_id);
    }
    let runnable_kind = if is_flow { "flow" } else { "script" };
    return format!(
        "{}/api/w/{}/capture_u/gcp/{}",
        base_endpoint, workspace_id, runnable_kind
    );
}

fn prepare_push_config(
    path: &str,
    base_endpoint: String,
    push_config: &mut SqlxJson<PushConfig>,
    workspace_id: &str,
    service_account_email: &str,
    trigger_mode: bool,
    is_flow: bool,
) -> Result<GooglePubSubPushConfig> {
    let base_endpoint = base_endpoint.trim_end_matches('/').to_string();

    let route_path = path.replace('.', "/");

    let push_endpoint = format!(
        "{}/{}",
        generate_endpoint(&base_endpoint, workspace_id, trigger_mode, is_flow),
        route_path
    );

    tracing::debug!("Push endpoint: {}", &push_endpoint);

    let authentication_method = push_config.authenticate.then(|| {
        let audience = push_config.audience.get_or_insert(push_endpoint.clone());
        AuthenticationMethod::OidcToken(OidcToken {
            service_account_email: service_account_email.to_owned(),
            audience: audience.to_owned(),
        })
    });

    if authentication_method.is_none() && push_config.audience.is_some() {
        push_config.audience = None;
    }

    Ok(GooglePubSubPushConfig {
        push_endpoint,
        authentication_method,
        attributes: HashMap::new(),
        wrapper: Some(Wrapper::PubsubWrapper(PubsubWrapper {})),
    })
}

pub async fn manage_google_subscription(
    authed: ApiAuthed,
    db: &DB,
    workspace_id: &str,
    gcp_resource_path: &str,
    path: &str,
    topic_id: &str,
    subscription_id: &mut Option<String>,
    base_endpoint: &mut Option<String>,
    subscription_mode: GcpSubscriptionMode,
    create_update_config: Option<CreateUpdateConfig>,
    trigger_mode: bool,
    is_flow: bool,
) -> Result<CreateUpdateConfig> {
    let (client, gcloud_service_account_email) =
        get_gcp_client(authed, db, gcp_resource_path, &workspace_id).await?;

    let topic = client.topic(topic_id);

    let _ = topic
        .exists(None)
        .await
        .map_err(|e| Error::BadGateway(e.to_string()))?
        .then_some(Ok(()))
        .unwrap_or_else(|| {
            Err(Error::BadRequest(format!(
                "Topic {} does not exist",
                topic_id
            )))
        })?;

    let google_pubsub_config = match subscription_mode {
        GcpSubscriptionMode::Existing => {
            let subscription_id = subscription_id
                .as_ref()
                .ok_or(Error::BadRequest("Missing subscription id".to_string()))?;
            let subscription = client.subscription(subscription_id);

            let (topic, subscription_config) = subscription
                .config(None)
                .await
                .map_err(|e| Error::BadGateway(e.message().to_owned()))?;

            if !topic.eq(topic_id) {
                return Err(Error::BadRequest(format!(
                    "Expected topic: {} got: {}",
                    &topic, topic_id
                )));
            }

            let mut google_pubsub_config = CreateUpdateConfig::default();

            let google_pubsub_config = match subscription_config.push_config {
                Some(gcloud_push_config) if !gcloud_push_config.push_endpoint.is_empty() => {
                    let base_endpoint = base_endpoint
                        .as_deref()
                        .ok_or(Error::BadRequest("Missing base endpoint".to_string()))?;
                    let url_start = format!(
                        "{}/",
                        generate_endpoint(base_endpoint, workspace_id, trigger_mode, is_flow)
                    );
                    if !gcloud_push_config.push_endpoint.starts_with(&url_start) {
                        return Err(Error::BadRequest(format!(
                            "Subscription url is {}, expected yours to starts with: {}",
                            &gcloud_push_config.push_endpoint, &url_start
                        )));
                    }

                    let route_path = &gcloud_push_config.push_endpoint[url_start.len()..];

                    if !route_path.eq(path) {
                        return Err(Error::BadRequest(format!(
                            "Expected route path to be same as trigger path: {}, got: {}",
                            path, route_path
                        )));
                    }

                    let mut push_config = PushConfig::default();

                    match gcloud_push_config.authentication_method {
                        Some(authentication_method) => match authentication_method {
                            AuthenticationMethod::OidcToken(oidc_token) => {
                                if !oidc_token
                                    .service_account_email
                                    .eq(&gcloud_service_account_email)
                                {
                                    return Err(Error::NotAuthorized(format!("Service account email of subscription does not match. got: {}, expected: {}", &oidc_token.service_account_email, &gcloud_service_account_email)));
                                }

                                let audience = oidc_token
                                    .audience
                                    .is_empty()
                                    .then_some(gcloud_push_config.push_endpoint)
                                    .or(Some(oidc_token.audience));

                                push_config.audience = audience;
                                push_config.authenticate = true;
                            }
                        },
                        None => {}
                    }
                    google_pubsub_config.delivery_type = DeliveryType::Push;
                    google_pubsub_config.delivery_config = Some(SqlxJson(push_config));

                    google_pubsub_config
                }
                _ => google_pubsub_config,
            };

            google_pubsub_config
        }
        GcpSubscriptionMode::CreateUpdate => {
            let generate_random_subscription_id = || {
                format!(
                    "windmill_{}_{}",
                    workspace_id,
                    path.replace(&['.', '/'], "_")
                )
            };

            let subscription_id = subscription_id.get_or_insert(generate_random_subscription_id());
            let subscription = client.subscription(&subscription_id);
            let mut create_update_config = create_update_config.ok_or(Error::BadRequest(
                "Missing create update config".to_string(),
            ))?;
            let subscription_exists = subscription
                .exists(None)
                .await
                .map_err(|e| Error::BadGateway(e.message().to_string()))?;

            let push_config = match create_update_config.delivery_type {
                DeliveryType::Pull => {
                    create_update_config.delivery_config = None;
                    None
                }
                DeliveryType::Push => {
                    let push_config = prepare_push_config(
                        path,
                        base_endpoint
                            .as_ref()
                            .ok_or(Error::BadRequest("Missing base endpoint".to_string()))?
                            .to_owned(),
                        create_update_config
                            .delivery_config
                            .get_or_insert(SqlxJson(PushConfig::default())),
                        workspace_id,
                        &gcloud_service_account_email,
                        trigger_mode,
                        is_flow,
                    )?;

                    Some(push_config)
                }
            };

            match subscription_exists {
                true => {
                    subscription
                        .update(
                            SubscriptionConfigToUpdate {
                                push_config: push_config
                                    .or(Some(GooglePubSubPushConfig::default())),
                                ..SubscriptionConfigToUpdate::default()
                            },
                            None,
                        )
                        .await
                        .map_err(|e| Error::BadGateway(e.message().to_string()))?;
                }
                false => {
                    client
                        .create_subscription(
                            &subscription_id,
                            topic_id,
                            SubscriptionConfig { push_config, ..SubscriptionConfig::default() },
                            None,
                        )
                        .await
                        .map_err(|e| Error::BadGateway(e.message().to_string()))?;
                }
            }

            create_update_config
        }
    };

    Ok(google_pubsub_config)
}
pub async fn process_google_push_request(
    headers: HeaderMap,
    request: Request,
) -> Result<(String, HashMap<String, Box<RawValue>>)> {
    let bytes = Bytes::from_request(request, &())
        .await
        .map_err(|e| Error::BadRequest(e.to_string()))?
        .to_vec();

    #[derive(Debug, Deserialize)]
    #[allow(non_snake_case, unused)]
    struct GooglePushMessage {
        #[serde(default)]
        attributes: Option<HashMap<String, String>>,
        data: String,
        messageId: String,
        message_id: String,
        publishTime: String,
        publish_time: String,
    }
    #[derive(Debug, Deserialize)]
    struct Payload {
        message: GooglePushMessage,
        subscription: String,
    }

    let payload = serde_json::from_slice::<Payload>(&bytes);

    let mut gcp = HashMap::new();
    let payload = match payload {
        Ok(payload) => {
            let Payload { message, subscription } = payload;
            let GooglePushMessage { attributes, data, message_id, publish_time, .. } = message;

            gcp.extend([
                ("attributes".to_string(), to_raw_value(&attributes)),
                ("message_id".to_string(), to_raw_value(&message_id)),
                ("subscription".to_string(), to_raw_value(&subscription)),
                ("publish_time".to_string(), to_raw_value(&publish_time)),
            ]);

            data
        }
        Err(error) => {
            if error.is_data() {
                let subscription = headers
                    .get("x-goog-pubsub-subscription-name")
                    .and_then(|header_value| header_value.to_str().ok());
                let message_id = headers
                    .get("x-goog-pubsub-message-id")
                    .and_then(|header_value| header_value.to_str().ok());
                let publish_time = headers
                    .get("x-goog-pubsub-publish-time")
                    .and_then(|header_value| header_value.to_str().ok());
                let ordering_key = headers
                    .get("x-goog-pubsub-ordering-key")
                    .and_then(|header_value| header_value.to_str().ok());

                gcp.extend([
                    ("message_id".to_string(), to_raw_value(&message_id)),
                    ("subscription".to_string(), to_raw_value(&subscription)),
                    ("ordering_key".to_string(), to_raw_value(&ordering_key)),
                    ("publish_time".to_string(), to_raw_value(&publish_time)),
                ]);

                BASE64_STANDARD.encode(bytes)
            } else {
                return Err(Error::BadRequest(error.to_string()));
            }
        }
    };

    let headers: HashMap<String, String> = headers
        .iter()
        .filter_map(|(header_name, header_value)| {
            let header_name = header_name.to_string();

            match header_value.to_str() {
                Ok(header_value) => Some((header_name, header_value.to_string())),
                Err(_) => None,
            }
        })
        .collect();

    gcp.insert("delivery_type".to_string(), to_raw_value(&"push"));
    gcp.insert("headers".to_string(), to_raw_value(&headers));

    Ok((payload, gcp))
}

pub async fn validate_jwt_token(
    db: &DB,
    user_db: UserDB,
    authed: ApiAuthed,
    headers: &HeaderMap,
    gcp_resource_path: &str,
    workspace_id: &str,
    delivery_config: &PushConfig,
) -> Result<()> {
    let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());

    let authorization = auth_header
        .map(|h| {
            h.strip_prefix("Bearer ")
                .ok_or(Error::NotAuthorized("Invalid token".to_string()))
        })
        .transpose()?;

    if let Some(token) = authorization {
        let service_account = try_get_resource_from_db_as::<ServiceAccount>(
            &authed,
            Some(user_db.clone()),
            db,
            gcp_resource_path,
            workspace_id,
        )
        .await?;

        let header =
            jsonwebtoken::decode_header(token).map_err(|e| Error::NotAuthorized(e.to_string()))?;

        let kid = header
            .kid
            .ok_or(Error::NotAuthorized("Invalid token".to_string()))?;

        let get_certificate = || async {
            let certs = reqwest::get(&service_account.auth_provider_x509_cert_url)
                .await
                .map_err(|e| Error::BadGateway(e.to_string()))?
                .json::<Value>()
                .await
                .map_err(|e| Error::BadRequest(format!("Invalid json: {}", e.to_string())))?;

            let certificate = certs.get(&kid);

            let certificate = match certificate {
                Some(Value::String(certificate)) => certificate,
                _ => return Err(Error::NotFound("pem certificate not found".to_string())),
            };

            Ok(certificate.to_owned())
        };

        let certificate = GOOGLE_PUBLIC_KEY_CACHE
            .get_or_insert_async(&kid, get_certificate())
            .await?;

        let decoding_key = DecodingKey::from_rsa_pem(certificate.as_bytes());

        let decoding_key = match decoding_key {
            Ok(decoding_key) => decoding_key,
            Err(_) => {
                let certificate = get_certificate().await?;
                let decoding_key = DecodingKey::from_rsa_pem(certificate.as_bytes())
                    .map_err(|err| Error::NotAuthorized(err.to_string()));
                GOOGLE_PUBLIC_KEY_CACHE.insert(kid, certificate);
                decoding_key?
            }
        };

        let mut validation = Validation::new(header.alg);

        let audience = delivery_config.audience.as_deref();
        validation.set_audience(&[audience.unwrap_or("")]);
        validation.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);

        let claims = jsonwebtoken::decode::<Value>(token, &decoding_key, &validation)
            .map_err(|e| Error::NotAuthorized(e.to_string()))?;

        let claims = claims.claims;

        match (&claims["email"], &claims["email_verified"]) {
            (Value::String(email), Value::Bool(verified))
                if email == &service_account.client_email && *verified =>
            {
                tracing::debug!("Email is valid and verified");
            }
            _ => {
                return Err(Error::NotAuthorized(
                    "Invalid token: Email is not valid or/and not verified".to_string(),
                ))
            }
        }
    }

    Ok(())
}
