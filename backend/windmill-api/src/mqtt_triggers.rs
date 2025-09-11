use crate::{
    capture::{insert_capture_payload, MqttTriggerConfig},
    db::{ApiAuthed, DB},
    resources::try_get_resource_from_db_as,
    trigger_helpers::{trigger_runnable, TriggerJobArgs},
    users::fetch_api_authed,
};

use axum::async_trait;
use base64::{engine, prelude::*};
use bytes::Bytes;
use itertools::Itertools;
use rumqttc::{
    v5::{
        mqttbytes::{
            v5::{ConnectProperties, Filter, PublishProperties},
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
use sqlx::{FromRow, Type};
use std::collections::HashMap;
use std::time::Duration;
use windmill_common::{
    db::UserDB,
    error::{to_anyhow, Error, Result},
    triggers::TriggerKind,
    utils::report_critical_error,
    worker::to_raw_value,
    INSTANCE_NAME,
};

use rand::seq::SliceRandom;
use serde_json::value::RawValue;
use sqlx::types::Json as SqlxJson;

async fn run_job(
    payload: &[u8],
    trigger_info: HashMap<String, Box<RawValue>>,
    db: &DB,
    trigger: &MqttTrigger,
) -> anyhow::Result<()> {
    let args = MqttTrigger::build_job_args(
        &trigger.script_path,
        trigger.is_flow,
        &trigger.workspace_id,
        db,
        payload,
        trigger_info,
    )
    .await?;

    let authed = fetch_api_authed(
        trigger.edited_by.clone(),
        trigger.email.clone(),
        &trigger.workspace_id,
        db,
        Some(format!("mqtt-{}", trigger.path)),
    )
    .await?;

    trigger_runnable(
        db,
        None,
        authed,
        &trigger.workspace_id,
        &trigger.script_path,
        trigger.is_flow,
        args,
        trigger.retry.as_ref(),
        trigger.error_handler_path.as_deref(),
        trigger.error_handler_args.as_ref(),
        format!("mqtt_trigger/{}", trigger.path),
    )
    .await?;

    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MqttV3Config {
    clean_session: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MqttV5Config {
    clean_start: Option<bool>,
    session_expiry_interval: Option<u32>,
    topic_alias_maximum: Option<u16>,
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

#[derive(Debug, Deserialize, Serialize, Type)]
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
    //encoded in base64
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

struct MqttClientBuilder<'client> {
    mqtt_resource: MqttResource,
    client_id: &'client str,
    subscribe_topics: Vec<SubscribeTopic>,
    v3_config: Option<&'client MqttV3Config>,
    v5_config: Option<&'client MqttV5Config>,
    mqtt_client_version: Option<&'client MqttClientVersion>,
}

impl<'client> MqttClientBuilder<'client> {
    fn new(
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

    async fn build_client(&self) -> Result<MqttClientResult> {
        match self.mqtt_client_version {
            Some(MqttClientVersion::V5) | None => self.build_v5_client().await,
            Some(MqttClientVersion::V3) => self.build_v3_client().await,
        }
    }

    fn get_tls_configuration(&self) -> Result<Option<Transport>> {
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

    async fn build_v5_client(&self) -> Result<MqttClientResult> {
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

    async fn build_v3_client(&self) -> Result<MqttClientResult> {
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

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct MqttTrigger {
    pub mqtt_resource_path: String,
    pub subscribe_topics: Vec<SqlxJson<SubscribeTopic>>,
    pub v3_config: Option<SqlxJson<MqttV3Config>>,
    pub v5_config: Option<SqlxJson<MqttV5Config>>,
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_version: Option<MqttClientVersion>,
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    pub workspace_id: String,
    pub edited_by: String,
    pub email: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    pub extra_perms: Option<serde_json::Value>,
    pub error: Option<String>,
    pub server_id: Option<String>,
    pub last_server_ping: Option<chrono::DateTime<chrono::Utc>>,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handler_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handler_args: Option<SqlxJson<HashMap<String, Box<RawValue>>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<SqlxJson<windmill_common::flows::Retry>>,
}

const KEEP_ALIVE: u64 = 60;
const CLIENT_CONNECTION_TIMEOUT: u64 = 60;
const TOPIC_ALIAS_MAXIMUM: u16 = 65535;

fn convert_disconnect_packet_into_err(disconnect: rumqttc::v5::mqttbytes::v5::Disconnect) -> Error {
    let err_message = disconnect
        .properties
        .map(|properties| properties.reason_string)
        .flatten();
    let reason_code = disconnect.reason_code as u8;
    anyhow::anyhow!(
        "Disconnected by the broker, reason code: {}, {}",
        reason_code,
        err_message
            .map(|err| format!("message: {}", err))
            .unwrap_or("".to_string())
    )
    .into()
}

async fn loop_ping(db: &DB, mqtt: &MqttConfig, error: Option<&str>) {
    loop {
        if mqtt.update_ping(db, error).await.is_none() {
            return;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

enum MqttClientResult {
    V3((V3MqttHandler, V3EventLoop)),
    V5((V5MqttHandler, V5EventLoop)),
}

trait MqttEvent {
    type IncomingPacket;
    type PublishPacket;
    type Event;

    fn handle_publish_packet(publish_packet: Self::PublishPacket) -> PublishData;
    fn handle_event(&self, event: Self::Event) -> Result<Option<(Bytes, PublishData)>>;
}

struct V5MqttHandler;

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

    fn handle_event(&self, event: Self::Event) -> Result<Option<(Bytes, PublishData)>> {
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
                    return Err(convert_disconnect_packet_into_err(disconnect));
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

struct V3MqttHandler;

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

    fn handle_event(&self, event: Self::Event) -> Result<Option<(Bytes, PublishData)>> {
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

const TIMEOUT_DURATION: u64 = 10;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(TIMEOUT_DURATION);

#[async_trait]
pub trait EventLoop {
    type Event;
    type Error;

    async fn poll(&mut self) -> Result<Self::Event>;
    async fn verify_connection(&mut self) -> Result<()>;
}

#[async_trait]
impl EventLoop for V5EventLoop {
    type Event = V5Event;
    type Error = rumqttc::v5::ConnectionError;
    async fn poll(&mut self) -> Result<Self::Event> {
        self.poll().await.map_err(|err| to_anyhow(err).into())
    }

    async fn verify_connection(&mut self) -> Result<()> {
        let start = std::time::Instant::now();

        while start.elapsed() < CONNECTION_TIMEOUT {
            match self.poll().await.map_err(to_anyhow)? {
                Self::Event::Incoming(V5Incoming::ConnAck(_)) => return Ok(()),
                Self::Event::Incoming(V5Incoming::Disconnect(disconnect)) => {
                    return Err(convert_disconnect_packet_into_err(disconnect));
                }
                _ => continue,
            }
        }

        Err(Error::BadConfig(format!(
            "Timeout occurred while trying to connect to mqtt broker after {} seconds",
            TIMEOUT_DURATION
        )))
    }
}

#[async_trait]
impl EventLoop for V3EventLoop {
    type Event = V3Event;
    type Error = Error;

    async fn poll(&mut self) -> Result<Self::Event> {
        self.poll().await.map_err(|err| to_anyhow(err).into())
    }

    async fn verify_connection(&mut self) -> Result<()> {
        let start = std::time::Instant::now();

        while start.elapsed() < CONNECTION_TIMEOUT {
            match self.poll().await.map_err(to_anyhow)? {
                Self::Event::Incoming(rumqttc::Packet::ConnAck(_)) => return Ok(()),
                _ => continue,
            }
        }

        Err(Error::BadConfig(format!(
            "Timeout occurred while trying to connect to mqtt broker after {} seconds",
            TIMEOUT_DURATION
        )))
    }
}

async fn handle_publish_packet(db: &DB, mqtt: &MqttConfig, payload: Bytes, publish: PublishData) {
    let trigger_info = HashMap::from([
        ("topic".to_string(), to_raw_value(&publish.topic)),
        ("retain".to_string(), to_raw_value(&publish.retain)),
        ("pkid".to_string(), to_raw_value(&publish.pkid)),
        ("qos".to_string(), to_raw_value(&publish.qos)),
        (
            "v5".to_string(),
            to_raw_value(&publish.v5.map(|properties| {
                serde_json::json!({
                    "payload_format_indicator": properties.payload_format_indicator,
                    "topic_alias": properties.topic_alias,
                    "response_topic": properties.response_topic,
                    "correlation_data": properties.correlation_data.as_deref(),
                    "user_properties": properties.user_properties,
                    "subscription_identifiers": properties.subscription_identifiers,
                    "content_type": properties.content_type,
                })
            })),
        ),
    ]);
    mqtt.handle(&db, payload.as_ref(), trigger_info).await;
}

async fn handle_event<E, H>(db: &DB, mqtt: &MqttConfig, handler: H, mut event_loop: E) -> ()
where
    H: MqttEvent,
    E: EventLoop<Event = H::Event>,
    E::Error: ToString,
{
    loop {
        let event = event_loop.poll().await;

        match event {
            Ok(event) => {
                let publish_data = handler.handle_event(event);
                if let Ok(Some((payload, publish_data))) = publish_data {
                    handle_publish_packet(db, mqtt, payload, publish_data).await;
                }
            }
            Err(err) => {
                let err = err.to_string();
                tracing::debug!("Error: {}", &err);
                mqtt.disable_with_error(&db, err).await;
                return;
            }
        }
    }
}

#[derive(Debug)]
enum MqttConfig {
    Trigger(MqttTrigger),
    Capture(CaptureConfigForMqttTrigger),
}

impl MqttConfig {
    async fn update_ping(&self, db: &DB, error: Option<&str>) -> Option<()> {
        match self {
            MqttConfig::Trigger(trigger) => trigger.update_ping(db, error).await,
            MqttConfig::Capture(capture) => capture.update_ping(db, error).await,
        }
    }

    async fn disable_with_error(&self, db: &DB, error: String) -> () {
        match self {
            MqttConfig::Trigger(trigger) => trigger.disable_with_error(&db, error).await,
            MqttConfig::Capture(capture) => capture.disable_with_error(db, error).await,
        }
    }

    async fn start_consuming_messages(
        &self,
        db: &DB,
    ) -> std::result::Result<MqttClientResult, Error> {
        let mqtt_resource_path;
        let subscribe_topics;
        let workspace_id;
        let authed;
        let client_version;
        let client_id;
        let v3_config;
        let v5_config;
        match self {
            MqttConfig::Capture(capture) => {
                mqtt_resource_path = &capture.trigger_config.0.mqtt_resource_path;
                subscribe_topics = capture.trigger_config.0.subscribe_topics.clone();
                workspace_id = &capture.workspace_id;
                authed = capture.fetch_authed(&db).await?;
                client_version = capture.trigger_config.0.client_version.as_ref();
                client_id = capture.trigger_config.0.client_id.as_deref();
                v3_config = capture.trigger_config.0.v3_config.as_ref();
                v5_config = capture.trigger_config.0.v5_config.as_ref();
            }
            MqttConfig::Trigger(trigger) => {
                mqtt_resource_path = &trigger.mqtt_resource_path;
                subscribe_topics = trigger
                    .subscribe_topics
                    .iter()
                    .map(|topic| topic.0.clone())
                    .collect_vec();
                workspace_id = &trigger.workspace_id;
                client_version = trigger.client_version.as_ref();
                authed = trigger.fetch_authed(&db).await?;
                client_id = trigger.client_id.as_deref();
                v3_config = trigger.v3_config.as_ref().map(|v3_config| &v3_config.0);
                v5_config = trigger.v5_config.as_ref().map(|v5_config| &v5_config.0);
            }
        }
        let mqtt_resource = try_get_resource_from_db_as::<MqttResource>(
            &authed,
            Some(UserDB::new(db.clone())),
            db,
            mqtt_resource_path,
            workspace_id,
        )
        .await?;
        let client_builder = MqttClientBuilder::new(
            mqtt_resource,
            client_id,
            subscribe_topics,
            v3_config,
            v5_config,
            client_version,
        );

        client_builder.build_client().await
    }

    async fn handle(
        &self,
        db: &DB,
        payload: &[u8],
        trigger_info: HashMap<String, Box<RawValue>>,
    ) -> () {
        match self {
            MqttConfig::Trigger(trigger) => trigger.handle(&db, payload, trigger_info).await,
            MqttConfig::Capture(capture) => capture.handle(&db, payload, trigger_info).await,
        }
    }
}

impl MqttTrigger {
    async fn try_to_listen_to_mqtt_messages(
        self,
        db: DB,
        killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> () {
        let mqtt_trigger = sqlx::query_scalar!(
            r#"
            UPDATE 
                mqtt_trigger 
            SET 
                server_id = $1, 
                last_server_ping = now(),
                error = 'Connecting...'
            WHERE 
                enabled IS TRUE 
                AND workspace_id = $2 
                AND path = $3 
                AND (last_server_ping IS NULL 
                    OR last_server_ping < now() - INTERVAL '15 seconds'
                ) 
            RETURNING true
            "#,
            *INSTANCE_NAME,
            self.workspace_id,
            self.path,
        )
        .fetch_optional(&db)
        .await;
        match mqtt_trigger {
            Ok(has_lock) => {
                if has_lock.flatten().unwrap_or(false) {
                    tracing::info!("Spawning new task to listen to mqtt notifications");
                    tokio::spawn(async move {
                        listen_to_messages(MqttConfig::Trigger(self), db.clone(), killpill_rx)
                            .await;
                    });
                } else {
                    tracing::info!("Mqtt trigger {} already being listened to", self.path);
                }
            }
            Err(err) => {
                tracing::error!(
                    "Error acquiring lock for mqtt trigger {}: {:?}",
                    self.path,
                    err
                );
            }
        };
    }

    async fn update_ping(&self, db: &DB, error: Option<&str>) -> Option<()> {
        let updated = sqlx::query_scalar!(
            r#"
            UPDATE 
                mqtt_trigger
            SET 
                last_server_ping = now(),
                error = $1
            WHERE
                workspace_id = $2
                AND path = $3
                AND server_id = $4 
                AND enabled IS TRUE
            RETURNING 1
            "#,
            error,
            &self.workspace_id,
            &self.path,
            *INSTANCE_NAME
        )
        .fetch_optional(db)
        .await;

        match updated {
            Ok(updated) => {
                if updated.flatten().is_none() {
                    // allow faster restart of mqtt trigger
                    sqlx::query!(
                        r#"
                    UPDATE 
                        mqtt_trigger 
                    SET
                        last_server_ping = NULL 
                    WHERE 
                        workspace_id = $1 
                        AND path = $2 
                        AND server_id IS NULL"#,
                        &self.workspace_id,
                        &self.path,
                    )
                    .execute(db)
                    .await
                    .ok();
                    tracing::info!(
                        "Mqtt trigger {} changed, disabled, or deleted, stopping...",
                        self.path
                    );
                    return None;
                }
            }
            Err(err) => {
                tracing::warn!(
                    "Error updating ping of mqtt trigger {}: {:?}",
                    self.path,
                    err
                );
            }
        };

        Some(())
    }

    async fn disable_with_error(&self, db: &DB, error: String) -> () {
        match sqlx::query!(
            r#"
                UPDATE 
                    mqtt_trigger 
                SET 
                    enabled = FALSE, 
                    error = $1, 
                    server_id = NULL, 
                    last_server_ping = NULL 
                WHERE 
                    workspace_id = $2 AND 
                    path = $3
            "#,
            error,
            self.workspace_id,
            self.path,
        )
        .execute(db)
        .await
        {
            Ok(_) => {
                report_critical_error(
                    format!(
                        "Disabling mqtt trigger {} because of error: {}",
                        self.path, error
                    ),
                    db.clone(),
                    Some(&self.workspace_id),
                    None,
                )
                .await;
            }
            Err(disable_err) => {
                report_critical_error(
                    format!("Could not disable mqtt trigger {} with err {}, disabling because of error {}", self.path, disable_err, error), 
                    db.clone(),
                    Some(&self.workspace_id),
                    None,
                ).await;
            }
        }
    }

    async fn fetch_authed(&self, db: &DB) -> Result<ApiAuthed> {
        fetch_api_authed(
            self.edited_by.clone(),
            self.email.clone(),
            &self.workspace_id,
            db,
            Some(format!("mqtt-{}", self.path)),
        )
        .await
    }

    async fn handle(
        &self,
        db: &DB,
        payload: &[u8],
        trigger_info: HashMap<String, Box<RawValue>>,
    ) -> () {
        if let Err(err) = run_job(payload, trigger_info, db, self).await {
            report_critical_error(
                format!("Failed to trigger job from mqtt {}: {:?}", self.path, err),
                db.clone(),
                Some(&self.workspace_id),
                None,
            )
            .await;
        };
    }
}

impl TriggerJobArgs<&[u8]> for MqttTrigger {
    fn v1_payload_fn(payload: &[u8]) -> HashMap<String, Box<RawValue>> {
        HashMap::from([("payload".to_string(), to_raw_value(&payload))])
    }

    fn v2_payload_fn(payload: &[u8]) -> HashMap<String, Box<RawValue>> {
        let base64_payload = engine::general_purpose::STANDARD.encode(payload);
        HashMap::from([("payload".to_string(), to_raw_value(&base64_payload))])
    }

    fn trigger_kind() -> TriggerKind {
        TriggerKind::Mqtt
    }
}

struct PublishData {
    topic: String,
    retain: bool,
    pkid: u16,
    v5: Option<PublishProperties>,
    qos: u8,
}

impl PublishData {
    fn new(
        topic: String,
        retain: bool,
        pkid: u16,
        v5: Option<PublishProperties>,
        qos: u8,
    ) -> PublishData {
        PublishData { topic, retain, pkid, v5, qos }
    }
}

async fn listen_to_messages(
    mqtt: MqttConfig,
    db: DB,
    mut killpill_rx: tokio::sync::broadcast::Receiver<()>,
) {
    tokio::select! {
        biased;

        _ = killpill_rx.recv() => {
            return;
        }

        _ = loop_ping(&db, &mqtt, Some("Connecting...")) => {
            return;
        }

        result = mqtt.start_consuming_messages(&db) => {
            tokio::select! {
                biased;

                _ = killpill_rx.recv() => {
                    return;
                }

                _ = loop_ping(&db, &mqtt, None) => {
                    return;
                }

                _ = async {
                    match result {
                        Ok(connection) => {
                            match connection {
                                MqttClientResult::V3((v3_handler, event_loop)) => handle_event(&db, &mqtt, v3_handler, event_loop).await,
                                MqttClientResult::V5((v5_handler, event_loop)) => handle_event(&db, &mqtt, v5_handler, event_loop).await,
                            }
                        }
                        Err(err) => {
                            tracing::error!(
                                "Mqtt trigger error while trying to start listening to notifications: {}",
                                &err
                            );
                            mqtt.disable_with_error(&db, err.to_string()).await
                        }
                    }
                } => {}
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct CaptureConfigForMqttTrigger {
    trigger_config: SqlxJson<MqttTriggerConfig>,
    path: String,
    is_flow: bool,
    workspace_id: String,
    owner: String,
    email: String,
}

impl CaptureConfigForMqttTrigger {
    async fn try_to_listen_to_mqtt_messages(
        self,
        db: DB,
        killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> () {
        match sqlx::query_scalar!(
            r#"
            UPDATE 
                capture_config 
            SET 
                server_id = $1,
                last_server_ping = now(), 
                error = 'Connecting...' 
            WHERE 
                last_client_ping > NOW() - INTERVAL '10 seconds' AND 
                workspace_id = $2 AND 
                path = $3 AND 
                is_flow = $4 AND 
                trigger_kind = 'mqtt' AND 
                (last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds') 
            RETURNING true
            "#,
            *INSTANCE_NAME,
            self.workspace_id,
            self.path,
            self.is_flow,
        )
        .fetch_optional(&db)
        .await
        {
            Ok(has_lock) => {
                if has_lock.flatten().unwrap_or(false) {
                    tokio::spawn(listen_to_messages(
                        MqttConfig::Capture(self),
                        db,
                        killpill_rx,
                    ));
                } else {
                    tracing::info!("Mqtt {} already being listened to", self.path);
                }
            }
            Err(err) => {
                tracing::error!(
                    "Error acquiring lock for capture mqtt {}: {:?}",
                    self.path,
                    err
                );
            }
        };
    }

    async fn update_ping(&self, db: &DB, error: Option<&str>) -> Option<()> {
        match sqlx::query_scalar!(
            r#"
            UPDATE 
                capture_config 
            SET 
                last_server_ping = now(), 
                error = $1 
            WHERE 
                workspace_id = $2 AND 
                path = $3 AND 
                is_flow = $4 AND 
                trigger_kind = 'mqtt' AND 
                server_id = $5 AND 
                last_client_ping > NOW() - INTERVAL '10 seconds' 
            RETURNING 1
        "#,
            error,
            self.workspace_id,
            self.path,
            self.is_flow,
            *INSTANCE_NAME
        )
        .fetch_optional(db)
        .await
        {
            Ok(updated) => {
                if updated.flatten().is_none() {
                    // allow faster restart of mqtt capture
                    sqlx::query!(
                        r#"UPDATE 
                        capture_config 
                    SET 
                        last_server_ping = NULL 
                    WHERE 
                        workspace_id = $1 AND 
                        path = $2 AND 
                        is_flow = $3 AND 
                        trigger_kind = 'mqtt' AND 
                        server_id IS NULL
                    "#,
                        self.workspace_id,
                        self.path,
                        self.is_flow,
                    )
                    .execute(db)
                    .await
                    .ok();
                    tracing::info!(
                        "Mqtt capture {} changed, disabled, or deleted, stopping...",
                        self.path
                    );
                    return None;
                }
            }
            Err(err) => {
                tracing::warn!(
                    "Error updating ping of capture mqtt {}: {:?}",
                    self.path,
                    err
                );
            }
        };

        Some(())
    }

    async fn fetch_authed(&self, db: &DB) -> Result<ApiAuthed> {
        fetch_api_authed(
            self.owner.clone(),
            self.email.clone(),
            &self.workspace_id,
            db,
            Some(format!("mqtt-{}", self.get_trigger_path())),
        )
        .await
    }

    fn get_trigger_path(&self) -> String {
        format!(
            "{}-{}",
            if self.is_flow { "flow" } else { "script" },
            self.path
        )
    }

    async fn disable_with_error(&self, db: &DB, error: String) -> () {
        if let Err(err) = sqlx::query!(
            r#"
                UPDATE 
                    capture_config 
                SET 
                    error = $1, 
                    server_id = NULL, 
                    last_server_ping = NULL 
                WHERE 
                    workspace_id = $2 AND 
                    path = $3 AND 
                    is_flow = $4 AND 
                    trigger_kind = 'mqtt'
            "#,
            error,
            self.workspace_id,
            self.path,
            self.is_flow,
        )
        .execute(db)
        .await
        {
            tracing::error!(
                "Could not disable mqtt capture {} ({}) with err {}, disabling because of error {}",
                self.path,
                self.workspace_id,
                err,
                error
            );
        }
    }

    async fn handle(
        &self,
        db: &DB,
        payload: &[u8],
        trigger_info: HashMap<String, Box<RawValue>>,
    ) -> () {
        let (main_args, preprocessor_args) =
            MqttTrigger::build_capture_payloads(payload, trigger_info);
        if let Err(err) = insert_capture_payload(
            db,
            &self.workspace_id,
            &self.path,
            self.is_flow,
            &TriggerKind::Mqtt,
            main_args,
            preprocessor_args,
            &self.owner,
        )
        .await
        {
            tracing::error!("Error inserting capture payload: {:?}", err);
        }
    }
}

async fn listen_to_unlistened_mqtt_events(
    db: &DB,
    killpill_rx: &tokio::sync::broadcast::Receiver<()>,
) {
    let mqtt_triggers = sqlx::query_as!(
        MqttTrigger,
        r#"
            SELECT
                mqtt_resource_path,
                subscribe_topics as "subscribe_topics!: Vec<SqlxJson<SubscribeTopic>>",
                v3_config as "v3_config!: Option<SqlxJson<MqttV3Config>>",
                v5_config as "v5_config!: Option<SqlxJson<MqttV5Config>>",
                client_version as "client_version: _",
                client_id,
                workspace_id,
                path,
                script_path,
                is_flow,
                edited_by,
                email,
                edited_at,
                server_id,
                last_server_ping,
                extra_perms,
                error,
                enabled,
                error_handler_path,
                error_handler_args as "error_handler_args: _",
                retry as "retry: _"
            FROM
                mqtt_trigger
            WHERE
                enabled IS TRUE
                AND (last_server_ping IS NULL OR
                    last_server_ping < now() - interval '15 seconds'
                )
            "#
    )
    .fetch_all(db)
    .await;

    match mqtt_triggers {
        Ok(mut triggers) => {
            triggers.shuffle(&mut rand::rng());
            for trigger in triggers {
                trigger
                    .try_to_listen_to_mqtt_messages(db.clone(), killpill_rx.resubscribe())
                    .await;
            }
        }
        Err(err) => {
            tracing::error!("Error fetching mqtt triggers: {:?}", err);
        }
    };

    let mqtt_triggers_capture = sqlx::query_as!(
        CaptureConfigForMqttTrigger,
        r#"
            SELECT
                path,
                is_flow,
                workspace_id,
                owner,
                email,
                trigger_config as "trigger_config!: _"
            FROM
                capture_config
            WHERE
                trigger_kind = 'mqtt' AND
                last_client_ping > NOW() - INTERVAL '10 seconds' AND
                trigger_config IS NOT NULL AND
                (last_server_ping IS NULL OR last_server_ping < now() - interval '15 seconds')
            "#
    )
    .fetch_all(db)
    .await;

    match mqtt_triggers_capture {
        Ok(mut captures) => {
            captures.shuffle(&mut rand::rng());
            for capture in captures {
                capture
                    .try_to_listen_to_mqtt_messages(db.clone(), killpill_rx.resubscribe())
                    .await;
            }
        }
        Err(err) => {
            tracing::error!("Error fetching captures mqtt triggers: {:?}", err);
        }
    };
}

pub fn start_mqtt_consumer(db: DB, mut killpill_rx: tokio::sync::broadcast::Receiver<()>) {
    tokio::spawn(async move {
        listen_to_unlistened_mqtt_events(&db, &killpill_rx).await;
        loop {
            tokio::select! {
                biased;
                _ = killpill_rx.recv() => {
                    return;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(15)) => {
                    listen_to_unlistened_mqtt_events(&db,  &killpill_rx).await
                }
            }
        }
    });
}
