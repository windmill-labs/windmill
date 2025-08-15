use base64::prelude::*;
use bytes::Bytes;
use itertools::Itertools;
use rumqttc::{
    v5::{
        mqttbytes::{
            v5::{ConnectProperties, PublishProperties},
            QoS as V5QoS,
        },
        AsyncClient as V5AsyncClient, Event as V5Event, EventLoop as V5EventLoop,
        Incoming as V5Incoming, MqttOptions as V5MqttOptions,
    },
    AsyncClient as V3AsyncClient, Event as V3Event, EventLoop as V3EventLoop,
    Incoming as V3Incoming, MqttOptions as V3MqttOptions, QoS as V3QoS, SubscribeFilter,
    TlsConfiguration, Transport,
};
use serde::{Deserialize, Serialize};
use sqlx::{types::Json as SqlxJson, FromRow, Type};
use std::time::Duration;
use windmill_common::error::Error;

pub mod handler;

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
pub struct NewMqttConfig {
    pub mqtt_resource_path: String,
    pub subscribe_topics: Vec<SubscribeTopic>,
    pub v3_config: Option<MqttV3Config>,
    pub v5_config: Option<MqttV5Config>,
    pub client_id: Option<String>,
    pub client_version: Option<MqttClientVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditMqttConfig {
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


#[derive(Clone)]
#[allow(unused)]
pub struct PublishData {
    topic: String,
    retain: bool,
    pkid: u16,
    v5: Option<PublishProperties>,
    qos: u8,
}

impl PublishData {
    pub fn new(
        topic: String,
        retain: bool,
        pkid: u16,
        v5: Option<PublishProperties>,
        qos: u8,
    ) -> PublishData {
        PublishData { topic, retain, pkid, v5, qos }
    }
}

pub trait MqttEvent {
    type IncomingPacket;
    type PublishPacket;
    type Event;

    fn handle_publish_packet(publish_packet: Self::PublishPacket) -> PublishData;
    fn handle_event(
        &self,
        event: Self::Event,
    ) -> std::result::Result<Option<(Bytes, PublishData)>, String>;
}

pub struct V5MqttHandler;

impl MqttEvent for V5MqttHandler {
    type IncomingPacket = V5Incoming;
    type PublishPacket = rumqttc::v5::mqttbytes::v5::Publish;
    type Event = V5Event;

    fn handle_publish_packet(publish_packet: Self::PublishPacket) -> PublishData {
        PublishData::new(
            String::from_utf8(publish_packet.topic.as_ref().to_vec()).unwrap_or("".to_string()),
            publish_packet.retain,
            publish_packet.pkid,
            publish_packet.properties,
            publish_packet.qos as u8,
        )
    }

    fn handle_event(
        &self,
        event: Self::Event,
    ) -> std::result::Result<Option<(Bytes, PublishData)>, String> {
        tracing::debug!("Inside V5 event");
        match event {
            Self::Event::Incoming(packet) => match packet {
                Self::IncomingPacket::Publish(publish_packet) => {
                    return Ok(Some((
                        publish_packet.payload.clone(),
                        Self::handle_publish_packet(publish_packet),
                    )))
                }
                Self::IncomingPacket::Disconnect(disconnect) => {
                    return Err(convert_disconnect_packet_into_string(disconnect));
                }
                packet => {
                    tracing::debug!("Received = {:#?}", packet);
                }
            },
            Self::Event::Outgoing(packet) => {
                tracing::debug!("Outgoing Received = {:#?}", packet);
            }
        }

        Ok(None)
    }
}

pub struct V3MqttHandler;

impl MqttEvent for V3MqttHandler {
    type IncomingPacket = V3Incoming;
    type PublishPacket = rumqttc::mqttbytes::v4::Publish;
    type Event = V3Event;

    fn handle_publish_packet(publish_packet: Self::PublishPacket) -> PublishData {
        PublishData::new(
            publish_packet.topic,
            publish_packet.retain,
            publish_packet.pkid,
            None,
            publish_packet.qos as u8,
        )
    }

    fn handle_event(
        &self,
        event: Self::Event,
    ) -> std::result::Result<Option<(Bytes, PublishData)>, String> {
        tracing::debug!("Inside V3 event");
        match event {
            Self::Event::Incoming(packet) => match packet {
                Self::IncomingPacket::Publish(publish_packet) => {
                    return Ok(Some((
                        publish_packet.payload.clone(),
                        Self::handle_publish_packet(publish_packet),
                    )))
                }
                packet => {
                    tracing::debug!("Received = {:?}", packet);
                }
            },
            Self::Event::Outgoing(packet) => {
                tracing::debug!("Outgoing Received = {:?}", packet);
            }
        }

        Ok(None)
    }
}

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

    pub async fn build_client(&self) -> std::result::Result<MqttClientResult, MqttError> {
        match self.mqtt_client_version {
            Some(MqttClientVersion::V5) | None => self.build_v5_client().await,
            Some(MqttClientVersion::V3) => self.build_v3_client().await,
        }
    }

    pub fn get_tls_configuration(&self) -> std::result::Result<Option<Transport>, MqttError> {
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
                                        .map_err(MqttError::Base64Decode)?;
                                    let password =
                                        tls.pkcs12_certificate_password.clone().unwrap_or_default();
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

    pub async fn build_v5_client(&self) -> std::result::Result<MqttClientResult, MqttError> {
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

        let (async_client, event_loop) =
            V5AsyncClient::new(mqtt_options, self.subscribe_topics.len());
        // Connection will be verified when the event loop starts

        if !self.subscribe_topics.is_empty() {
            let subscribe_filters = self
                .subscribe_topics
                .iter()
                .map(|topic| {
                    rumqttc::v5::mqttbytes::v5::Filter::new(
                        topic.topic.clone(),
                        topic.qos.clone().into(),
                    )
                })
                .collect_vec();

            async_client
                .subscribe_many(subscribe_filters)
                .await
                .map_err(MqttError::V5RumqttClient)?;
        }
        Ok(MqttClientResult::V5((V5MqttHandler, event_loop)))
    }

    pub async fn build_v3_client(&self) -> std::result::Result<MqttClientResult, MqttError> {
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

        let (async_client, event_loop) =
            V3AsyncClient::new(mqtt_options, self.subscribe_topics.len());
        // Connection will be verified when the event loop starts

        if !self.subscribe_topics.is_empty() {
            let subscribe_filters = self
                .subscribe_topics
                .iter()
                .map(|topic| SubscribeFilter::new(topic.topic.clone(), topic.qos.clone().into()))
                .collect_vec();

            async_client
                .subscribe_many(subscribe_filters)
                .await
                .map_err(MqttError::V3RumqttClient)?;
        }
        Ok(MqttClientResult::V3((V3MqttHandler, event_loop)))
    }

    pub async fn test_v5_connection(&self) -> std::result::Result<(), MqttError> {
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

        let (_async_client, mut event_loop) =
            V5AsyncClient::new(mqtt_options, self.subscribe_topics.len());

        let start = std::time::Instant::now();
        while start.elapsed() < CONNECTION_TIMEOUT {
            match event_loop.poll().await? {
                rumqttc::v5::Event::Incoming(rumqttc::v5::mqttbytes::v5::Packet::ConnAck(_)) => {
                    return Ok(())
                }
                rumqttc::v5::Event::Incoming(rumqttc::v5::mqttbytes::v5::Packet::Disconnect(
                    disconnect,
                )) => {
                    return Err(MqttError::Common(Error::BadConfig(format!(
                        "Disconnected by the broker, reason code: {}",
                        disconnect.reason_code as u8
                    ))));
                }
                _ => continue,
            }
        }

        Err(MqttError::Common(Error::BadConfig(format!(
            "Timeout occurred while trying to connect to mqtt broker after {} seconds",
            TIMEOUT_DURATION
        ))))
    }

    pub async fn test_v3_connection(&self) -> std::result::Result<(), MqttError> {
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

        let (_async_client, mut event_loop) =
            V3AsyncClient::new(mqtt_options, self.subscribe_topics.len());

        let start = std::time::Instant::now();
        while start.elapsed() < CONNECTION_TIMEOUT {
            match event_loop.poll().await? {
                rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_)) => return Ok(()),
                _ => continue,
            }
        }

        Err(MqttError::Common(Error::BadConfig(format!(
            "Timeout occurred while trying to connect to mqtt broker after {} seconds",
            TIMEOUT_DURATION
        ))))
    }

    pub async fn test_connection(&self) -> std::result::Result<(), MqttError> {
        match self.mqtt_client_version {
            Some(MqttClientVersion::V5) | None => self.test_v5_connection().await,
            Some(MqttClientVersion::V3) => self.test_v3_connection().await,
        }
    }
}

// Utility function for disconnect packet conversion
pub fn convert_disconnect_packet_into_string(
    disconnect: rumqttc::v5::mqttbytes::v5::Disconnect,
) -> String {
    format!(
        "Disconnected by the broker, reason code: {}",
        disconnect.reason_code as u8
    )
}
