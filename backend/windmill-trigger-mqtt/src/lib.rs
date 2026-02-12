use base64::{engine, prelude::*};
use itertools::Itertools;
use rumqttc::{
    v5::{
        mqttbytes::{
            v5::{ConnectProperties, Filter},
            QoS as V5QoS,
        },
        AsyncClient as V5AsyncClient, EventLoop as V5EventLoop, MqttOptions as V5MqttOptions,
    },
    AsyncClient as V3AsyncClient, EventLoop as V3EventLoop, MqttOptions as V3MqttOptions,
    QoS as V3QoS, SubscribeFilter, TlsConfiguration, Transport,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::{types::Json as SqlxJson, FromRow, Type};
use std::{collections::HashMap, time::Duration};
use windmill_common::{
    error::{to_anyhow, Error},
    triggers::TriggerKind,
    worker::to_raw_value,
};

use crate::listener::EventLoop;
use windmill_trigger::trigger_helpers::TriggerJobArgs;

pub mod handler;
pub mod listener;

#[derive(Clone, Copy)]
pub struct MqttTrigger;

impl TriggerJobArgs for MqttTrigger {
    type Payload = Vec<u8>;
    const TRIGGER_KIND: TriggerKind = TriggerKind::Mqtt;

    fn v1_payload_fn(payload: &Self::Payload) -> HashMap<String, Box<RawValue>> {
        HashMap::from([("payload".to_string(), to_raw_value(&payload))])
    }

    fn v2_payload_fn(payload: &Self::Payload) -> HashMap<String, Box<RawValue>> {
        let base64_payload = engine::general_purpose::STANDARD.encode(payload);
        HashMap::from([("payload".to_string(), to_raw_value(&base64_payload))])
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum QualityOfService {
    Qos0,
    Qos1,
    Qos2,
}

impl From<QualityOfService> for V3QoS {
    fn from(value: QualityOfService) -> Self {
        match value {
            QualityOfService::Qos0 => V3QoS::AtMostOnce,
            QualityOfService::Qos1 => V3QoS::AtLeastOnce,
            QualityOfService::Qos2 => V3QoS::ExactlyOnce,
        }
    }
}

impl From<QualityOfService> for V5QoS {
    fn from(value: QualityOfService) -> Self {
        match value {
            QualityOfService::Qos0 => V5QoS::AtMostOnce,
            QualityOfService::Qos1 => V5QoS::AtLeastOnce,
            QualityOfService::Qos2 => V5QoS::ExactlyOnce,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MqttV3Config {
    clean_session: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MqttV5Config {
    clean_start: Option<bool>,
    session_expiry_interval: Option<u32>,
    topic_alias_maximum: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Type)]
#[sqlx(type_name = "MQTT_CLIENT_VERSION")]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MqttClientVersion {
    V3,
    V5,
}

#[derive(Debug, Deserialize)]
pub struct Tls {
    enabled: bool,
    ca_certificate: String,
    pkcs12_client_certificate: Option<String>,
    pkcs12_certificate_password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Credentials {
    username: Option<String>,
    password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MqttResource {
    broker: String,
    port: u16,
    credentials: Option<Credentials>,
    tls: Option<Tls>,
}

#[derive(Clone, Debug, FromRow, Serialize, Deserialize)]
pub struct SubscribeTopic {
    qos: QualityOfService,
    topic: String,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MqttConfig {
    pub mqtt_resource_path: String,
    pub subscribe_topics: Vec<SqlxJson<SubscribeTopic>>,
    pub v3_config: Option<SqlxJson<MqttV3Config>>,
    pub v5_config: Option<SqlxJson<MqttV5Config>>,
    pub client_id: Option<String>,
    pub client_version: Option<MqttClientVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfigRequest {
    pub mqtt_resource_path: String,
    pub subscribe_topics: Vec<SubscribeTopic>,
    pub v3_config: Option<MqttV3Config>,
    pub v5_config: Option<MqttV5Config>,
    pub client_id: Option<String>,
    pub client_version: Option<MqttClientVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMqttConfig {
    pub mqtt_resource_path: String,
    pub client_version: Option<MqttClientVersion>,
    pub v3_config: Option<MqttV3Config>,
    pub v5_config: Option<MqttV5Config>,
}

// Constants
pub const KEEP_ALIVE: u64 = 60;
pub const CLIENT_CONNECTION_TIMEOUT: u64 = 60;
pub const TOPIC_ALIAS_MAXIMUM: u16 = 65535;
pub const TIMEOUT_DURATION: u64 = 10;
pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(TIMEOUT_DURATION);

pub struct V3MqttHandler;
pub struct V5MqttHandler;

pub enum MqttClientResult {
    V3((V3MqttHandler, V3EventLoop)),
    V5((V5MqttHandler, V5EventLoop)),
}

#[derive(Debug, thiserror::Error)]
pub enum MqttError {
    #[error("{0}")]
    Common(#[from] Error),
    #[error("{0}")]
    V5RumqttClient(#[from] rumqttc::v5::ClientError),
    #[error("{0}")]
    V5ConnectionError(#[from] rumqttc::v5::ConnectionError),
    #[error("{0}")]
    V3RumqttClient(#[from] rumqttc::ClientError),
    #[error("{0}")]
    V3ConnectionError(#[from] rumqttc::ConnectionError),
    #[error("{0}")]
    Base64Decode(#[from] base64::DecodeError),
}

pub struct MqttClientBuilder<'client> {
    mqtt_resource: MqttResource,
    client_id: &'client str,
    subscribe_topics: Vec<SubscribeTopic>,
    v3_config: Option<&'client MqttV3Config>,
    v5_config: Option<&'client MqttV5Config>,
    mqtt_client_version: Option<&'client MqttClientVersion>,
}

impl<'client> MqttClientBuilder<'client> {
    pub fn new(
        mqtt_resource: MqttResource,
        client_id: Option<&'client str>,
        subscribe_topics: Vec<SubscribeTopic>,
        v3_config: Option<&'client MqttV3Config>,
        v5_config: Option<&'client MqttV5Config>,
        mqtt_client_version: Option<&'client MqttClientVersion>,
    ) -> Self {
        Self {
            mqtt_resource,
            client_id: client_id.unwrap_or(""),
            subscribe_topics,
            v3_config,
            v5_config,
            mqtt_client_version,
        }
    }

    pub async fn build_client(&self) -> Result<MqttClientResult, Error> {
        match self.mqtt_client_version {
            Some(MqttClientVersion::V5) | None => self.build_v5_client().await,
            Some(MqttClientVersion::V3) => self.build_v3_client().await,
        }
    }

    fn get_tls_configuration(&self) -> Result<Option<Transport>, Error> {
        let transport = match self.mqtt_resource.tls {
            Some(ref tls) if tls.enabled => {
                let transport = match tls.ca_certificate.trim().is_empty() {
                    true => rumqttc::Transport::Tls(TlsConfiguration::Native),
                    false => rumqttc::Transport::Tls(TlsConfiguration::SimpleNative {
                        ca: tls.ca_certificate.as_bytes().to_vec(),
                        client_auth: {
                            match tls.pkcs12_client_certificate.as_ref() {
                                Some(client_certificate)
                                    if !client_certificate.trim().is_empty() =>
                                {
                                    let client_certificate = BASE64_STANDARD
                                        .decode(client_certificate)
                                        .map_err(to_anyhow)?;
                                    let password = tls
                                        .pkcs12_certificate_password
                                        .clone()
                                        .unwrap_or("".to_string());
                                    Some((client_certificate, password))
                                }
                                _ => None,
                            }
                        },
                    }),
                };

                Some(transport)
            }
            _ => None,
        };

        Ok(transport)
    }

    async fn build_v5_client(&self) -> Result<MqttClientResult, Error> {
        let mut mqtt_options = V5MqttOptions::new(
            self.client_id,
            &self.mqtt_resource.broker,
            self.mqtt_resource.port,
        );

        if let Some(credentials) = &self.mqtt_resource.credentials {
            let username = credentials.username.as_deref().unwrap_or("");
            let password = credentials.password.as_deref().unwrap_or("");
            mqtt_options.set_credentials(username, password);
        }

        if let Some(transport) = self.get_tls_configuration()? {
            mqtt_options.set_transport(transport);
        }

        mqtt_options.set_connection_timeout(CLIENT_CONNECTION_TIMEOUT);

        mqtt_options.set_keep_alive(Duration::from_secs(KEEP_ALIVE));

        if let Some(v5_config) = self.v5_config {
            mqtt_options.set_clean_start(v5_config.clean_start.unwrap_or(true));
            mqtt_options.set_connect_properties(ConnectProperties {
                session_expiry_interval: v5_config.session_expiry_interval,
                receive_maximum: None,
                max_packet_size: None,
                topic_alias_max: v5_config.topic_alias_maximum.or(Some(TOPIC_ALIAS_MAXIMUM)),
                request_response_info: None,
                request_problem_info: None,
                user_properties: vec![],
                authentication_method: None,
                authentication_data: None,
            });
        }

        let (async_client, mut event_loop) =
            V5AsyncClient::new(mqtt_options, self.subscribe_topics.len());
        event_loop.verify_connection().await?;

        if !self.subscribe_topics.is_empty() {
            let subscribe_filters = self
                .subscribe_topics
                .iter()
                .map(|topic| Filter::new(topic.topic.clone(), topic.qos.clone().into()))
                .collect_vec();

            async_client
                .subscribe_many(subscribe_filters)
                .await
                .map_err(to_anyhow)?;
        }
        Ok(MqttClientResult::V5((V5MqttHandler, event_loop)))
    }

    async fn build_v3_client(&self) -> Result<MqttClientResult, Error> {
        let mut mqtt_options = V3MqttOptions::new(
            self.client_id,
            &self.mqtt_resource.broker,
            self.mqtt_resource.port,
        );

        if let Some(credentials) = &self.mqtt_resource.credentials {
            let username = credentials.username.as_deref().unwrap_or("");
            let password = credentials.password.as_deref().unwrap_or("");
            mqtt_options.set_credentials(username, password);
        }

        if let Some(transport) = self.get_tls_configuration()? {
            mqtt_options.set_transport(transport);
        }
        mqtt_options.set_keep_alive(Duration::from_secs(KEEP_ALIVE));
        if let Some(v3_config) = self.v3_config {
            mqtt_options.set_clean_session(v3_config.clean_session.unwrap_or(true));
        }

        let (async_client, mut event_loop) =
            V3AsyncClient::new(mqtt_options, self.subscribe_topics.len());
        event_loop.verify_connection().await?;

        if !self.subscribe_topics.is_empty() {
            let subscribe_filters = self
                .subscribe_topics
                .iter()
                .map(|topic| SubscribeFilter::new(topic.topic.clone(), topic.qos.clone().into()))
                .collect_vec();

            async_client
                .subscribe_many(subscribe_filters)
                .await
                .map_err(to_anyhow)?;
        }
        Ok(MqttClientResult::V3((V3MqttHandler, event_loop)))
    }
}
