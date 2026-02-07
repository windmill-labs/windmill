use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use bytes::Bytes;
use rumqttc::{
    v5::{
        mqttbytes::v5::PublishProperties, Event as V5Event, EventLoop as V5EventLoop,
        Incoming as V5Incoming,
    },
    Event as V3Event, EventLoop as V3EventLoop, Incoming as V3Incoming,
};
use std::time::Duration;
use tokio::sync::RwLock;
use windmill_common::{
    db::UserDB,
    error::{to_anyhow, Error, Result},
    jobs::JobTriggerKind,
    worker::to_raw_value,
    DB,
};

use windmill_store::resources::try_get_resource_from_db_as;
use windmill_trigger::listener::ListeningTrigger;
use windmill_trigger::trigger_helpers::TriggerJobArgs;
use windmill_trigger::Listener;

use super::{
    MqttClientBuilder, MqttClientResult, MqttConfig, MqttResource, MqttTrigger, V3MqttHandler,
    V5MqttHandler,
};

#[async_trait]
impl Listener for MqttTrigger {
    type Consumer = MqttClientResult;
    type Extra = ();
    type ExtraState = ();
    const JOB_TRIGGER_KIND: JobTriggerKind = JobTriggerKind::Mqtt;

    async fn get_consumer(
        &self,
        db: &DB,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        _err_message: Arc<RwLock<Option<String>>>,
        _killpill_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<Option<Self::Consumer>> {
        let ListeningTrigger::<Self::TriggerConfig> { workspace_id, trigger_config, .. } =
            listening_trigger;

        let MqttConfig {
            mqtt_resource_path,
            subscribe_topics,
            v3_config,
            v5_config,
            client_id,
            client_version,
            ..
        } = trigger_config;

        let authed = listening_trigger
            .authed(db, &Self::TRIGGER_KIND.to_string())
            .await?;

        let mqtt_resource = try_get_resource_from_db_as::<MqttResource>(
            &authed,
            Some(UserDB::new(db.clone())),
            &db,
            mqtt_resource_path,
            workspace_id,
        )
        .await?;

        let subscribe_topics = subscribe_topics
            .iter()
            .map(|topic| topic.0.clone())
            .collect();

        let client_builder = MqttClientBuilder::new(
            mqtt_resource,
            client_id.as_deref(),
            subscribe_topics,
            v3_config.as_ref().map(|c| &c.0),
            v5_config.as_ref().map(|c| &c.0),
            client_version.as_ref(),
        );

        let client_result = client_builder
            .build_client()
            .await
            .map_err(|e| Error::BadConfig(format!("Failed to build MQTT client: {}", e)))?;

        Ok(Some(client_result))
    }

    async fn consume(
        &self,
        db: &DB,
        consumer: Self::Consumer,
        listening_trigger: &ListeningTrigger<Self::TriggerConfig>,
        _err_message: Arc<RwLock<Option<String>>>,
        _killpill_rx: tokio::sync::broadcast::Receiver<()>,
        _extra_state: Option<&Self::ExtraState>,
    ) {
        tracing::info!(
            "Starting to listen for MQTT trigger {}",
            &listening_trigger.path
        );

        match consumer {
            MqttClientResult::V3((v3_handler, event_loop)) => {
                handle_event(&db, self, listening_trigger, v3_handler, event_loop).await
            }
            MqttClientResult::V5((v5_handler, event_loop)) => {
                handle_event(&db, self, listening_trigger, v5_handler, event_loop).await
            }
        }
    }
}

const TIMEOUT_DURATION: u64 = 10;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(TIMEOUT_DURATION);

fn convert_disconnect_packet_into_string(
    disconnect: rumqttc::v5::mqttbytes::v5::Disconnect,
) -> String {
    let err_message = disconnect
        .properties
        .map(|properties| properties.reason_string)
        .flatten();
    let reason_code = disconnect.reason_code as u8;
    format!(
        "Disconnected by the broker, reason code: {}, {}",
        reason_code,
        err_message
            .map(|err| format!("message: {}", err))
            .unwrap_or("".to_string())
    )
}

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
                    return Err(Error::BadConfig(convert_disconnect_packet_into_string(
                        disconnect,
                    )));
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
    type Error = rumqttc::ConnectionError;

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

async fn handle_event<T, E, H>(
    db: &DB,
    listener: &T,
    listening_trigger: &ListeningTrigger<T::TriggerConfig>,
    handler: H,
    mut event_loop: E,
) -> ()
where
    T: Listener,
    H: MqttEvent,
    E: EventLoop<Event = H::Event>,
    E::Error: ToString,
    <T as TriggerJobArgs>::Payload: From<Bytes>,
{
    loop {
        let event = event_loop.poll().await;

        match event {
            Ok(event) => {
                let publish_data = handler.handle_event(event);
                if let Ok(Some((payload, publish_data))) = publish_data {
                    let trigger_info = HashMap::from([
                        ("topic".to_string(), to_raw_value(&publish_data.topic)),
                        ("retain".to_string(), to_raw_value(&publish_data.retain)),
                        ("pkid".to_string(), to_raw_value(&publish_data.pkid)),
                        ("qos".to_string(), to_raw_value(&publish_data.qos)),
                        (
                            "v5".to_string(),
                            to_raw_value(&publish_data.v5.map(|properties| {
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
                    let _ = listener
                        .handle_event(db, listening_trigger, payload.into(), trigger_info, None)
                        .await;
                }
            }
            Err(err) => {
                let error = err.to_string();
                tracing::debug!("Error: {}", &err);
                listener
                    .disable_with_error(db, listening_trigger, error)
                    .await;
                return;
            }
        }
    }
}

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

trait MqttEvent {
    type IncomingPacket;
    type PublishPacket;
    type Event;

    fn handle_publish_packet(publish_packet: Self::PublishPacket) -> PublishData;
    fn handle_event(&self, event: Self::Event) -> Result<Option<(Bytes, PublishData)>>;
}

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
                    return Err(
                        anyhow::anyhow!(convert_disconnect_packet_into_string(disconnect)).into(),
                    );
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
