use base64::engine;
use base64::prelude::*;
use lapin::{
    options::{BasicConsumeOptions, BasicQosOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
    Channel, Connection, ConnectionProperties, Consumer,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::{types::Json as SqlxJson, FromRow};
use std::collections::HashMap;
use windmill_common::{error::Error, triggers::TriggerKind, worker::to_raw_value};

use windmill_trigger::trigger_helpers::TriggerJobArgs;

pub mod handler;
pub mod listener;

#[derive(Clone, Copy)]
pub struct AmqpTrigger;

impl TriggerJobArgs for AmqpTrigger {
    type Payload = Vec<u8>;
    const TRIGGER_KIND: TriggerKind = TriggerKind::Amqp;

    fn v1_payload_fn(payload: &Self::Payload) -> HashMap<String, Box<RawValue>> {
        HashMap::from([("payload".to_string(), to_raw_value(&payload))])
    }

    fn v2_payload_fn(payload: &Self::Payload) -> HashMap<String, Box<RawValue>> {
        let base64_payload = engine::general_purpose::STANDARD.encode(payload);
        HashMap::from([("payload".to_string(), to_raw_value(&base64_payload))])
    }
}

#[derive(Debug, Deserialize)]
pub struct AmqpResource {
    pub host: String,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub vhost: Option<String>,
    pub tls: Option<bool>,
}

/// Binding of the consumed queue to an exchange. When present, the queue is bound
/// to `exchange_name` for each routing key so messages published to the exchange
/// are routed to it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub exchange_name: String,
    #[serde(default)]
    pub routing_keys: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AmqpOptions {
    /// Declare the queue (durable) before consuming. When false the queue is
    /// declared passively, i.e. it must already exist on the broker.
    pub declare_queue: Option<bool>,
    /// Maximum number of unacknowledged messages the broker delivers at once.
    pub prefetch_count: Option<u16>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AmqpConfig {
    pub amqp_resource_path: String,
    pub queue_name: String,
    pub exchange: Option<SqlxJson<ExchangeConfig>>,
    pub options: Option<SqlxJson<AmqpOptions>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmqpConfigRequest {
    pub amqp_resource_path: String,
    pub queue_name: String,
    pub exchange: Option<ExchangeConfig>,
    pub options: Option<AmqpOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAmqpConfig {
    pub amqp_resource_path: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AmqpError {
    #[error("{0}")]
    Common(#[from] Error),
    #[error("{0}")]
    Lapin(#[from] lapin::Error),
}

/// Consumer bundle. The connection and channel are kept alive for as long as the
/// consumer stream is polled: dropping them would tear down the AMQP consumer.
pub struct AmqpConsumer {
    pub connection: Connection,
    pub channel: Channel,
    pub consumer: Consumer,
    pub queue_name: String,
}

pub const CONSUMER_TAG: &str = "windmill";

fn build_uri(resource: &AmqpResource) -> String {
    let tls = resource.tls.unwrap_or(false);
    let scheme = if tls { "amqps" } else { "amqp" };
    let port = resource.port.unwrap_or(if tls { 5671 } else { 5672 });

    let credentials = match (resource.username.as_deref(), resource.password.as_deref()) {
        (Some(user), password) if !user.is_empty() => format!(
            "{}:{}@",
            urlencoding::encode(user),
            urlencoding::encode(password.unwrap_or("")),
        ),
        _ => String::new(),
    };

    // The URI path is the virtual host and must be percent-encoded; the default
    // vhost "/" therefore becomes "%2F". An empty vhost also falls back to "/".
    let vhost = match resource.vhost.as_deref() {
        Some(v) if !v.is_empty() => v,
        _ => "/",
    };
    let vhost_encoded = urlencoding::encode(vhost);

    // Bracket an IPv6 literal host so `host:port` parses correctly.
    let host = if resource.host.contains(':') && !resource.host.starts_with('[') {
        format!("[{}]", resource.host)
    } else {
        resource.host.clone()
    };

    format!(
        "{}://{}{}:{}/{}",
        scheme, credentials, host, port, vhost_encoded
    )
}

fn connection_properties() -> ConnectionProperties {
    ConnectionProperties::default()
        .with_executor(tokio_executor_trait::Tokio::current())
        .with_reactor(tokio_reactor_trait::Tokio)
}

pub struct AmqpClientBuilder<'client> {
    resource: AmqpResource,
    queue_name: &'client str,
    exchange: Option<&'client ExchangeConfig>,
    options: Option<&'client AmqpOptions>,
}

impl<'client> AmqpClientBuilder<'client> {
    pub fn new(
        resource: AmqpResource,
        queue_name: &'client str,
        exchange: Option<&'client ExchangeConfig>,
        options: Option<&'client AmqpOptions>,
    ) -> Self {
        Self { resource, queue_name, exchange, options }
    }

    async fn connect(&self) -> Result<Connection, AmqpError> {
        let uri = build_uri(&self.resource);
        let connection = Connection::connect(&uri, connection_properties()).await?;
        Ok(connection)
    }

    /// Establish a connection and open a channel to verify the broker is reachable
    /// with the provided credentials.
    pub async fn test_connection(&self) -> Result<(), AmqpError> {
        let connection = self.connect().await?;
        connection.create_channel().await?;
        Ok(())
    }

    pub async fn build_consumer(&self) -> Result<AmqpConsumer, AmqpError> {
        let connection = self.connect().await?;
        let channel = connection.create_channel().await?;

        if let Some(prefetch_count) = self.options.and_then(|o| o.prefetch_count) {
            channel
                .basic_qos(prefetch_count, BasicQosOptions::default())
                .await?;
        }

        let declare_queue = self.options.and_then(|o| o.declare_queue).unwrap_or(true);

        let queue_declare_options = QueueDeclareOptions {
            passive: !declare_queue,
            durable: declare_queue,
            exclusive: false,
            auto_delete: false,
            nowait: false,
        };

        channel
            .queue_declare(
                self.queue_name,
                queue_declare_options,
                FieldTable::default(),
            )
            .await?;

        if let Some(exchange) = self.exchange {
            if !exchange.exchange_name.trim().is_empty() {
                // Bind the queue for every routing key; an empty list binds once
                // with an empty routing key (fanout exchanges ignore it anyway).
                let routing_keys = if exchange.routing_keys.is_empty() {
                    vec![String::new()]
                } else {
                    exchange.routing_keys.clone()
                };
                for routing_key in routing_keys {
                    channel
                        .queue_bind(
                            self.queue_name,
                            &exchange.exchange_name,
                            &routing_key,
                            QueueBindOptions::default(),
                            FieldTable::default(),
                        )
                        .await?;
                }
            }
        }

        let consumer = channel
            .basic_consume(
                self.queue_name,
                CONSUMER_TAG,
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok(AmqpConsumer { connection, channel, consumer, queue_name: self.queue_name.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resource(
        host: &str,
        port: Option<u16>,
        username: Option<&str>,
        password: Option<&str>,
        vhost: Option<&str>,
        tls: Option<bool>,
    ) -> AmqpResource {
        AmqpResource {
            host: host.to_string(),
            port,
            username: username.map(str::to_string),
            password: password.map(str::to_string),
            vhost: vhost.map(str::to_string),
            tls,
        }
    }

    #[test]
    fn build_uri_defaults_encode_root_vhost_and_pick_plaintext_port() {
        let uri = build_uri(&resource("broker", None, None, None, None, None));
        assert_eq!(uri, "amqp://broker:5672/%2F");
    }

    #[test]
    fn build_uri_tls_picks_amqps_and_5671() {
        let uri = build_uri(&resource("broker", None, None, None, None, Some(true)));
        assert_eq!(uri, "amqps://broker:5671/%2F");
    }

    #[test]
    fn build_uri_encodes_credentials_and_custom_vhost() {
        let uri = build_uri(&resource(
            "broker",
            Some(5673),
            Some("us er"),
            Some("p@ss/word"),
            Some("my/vhost"),
            None,
        ));
        assert_eq!(uri, "amqp://us%20er:p%40ss%2Fword@broker:5673/my%2Fvhost");
    }

    #[test]
    fn build_uri_omits_credentials_when_username_empty() {
        let uri = build_uri(&resource(
            "broker",
            None,
            Some(""),
            Some("secret"),
            None,
            None,
        ));
        assert_eq!(uri, "amqp://broker:5672/%2F");
    }

    #[test]
    fn build_uri_blank_vhost_falls_back_to_root() {
        let uri = build_uri(&resource("broker", None, None, None, Some(""), None));
        assert_eq!(uri, "amqp://broker:5672/%2F");
    }

    #[test]
    fn build_uri_brackets_ipv6_host() {
        let uri = build_uri(&resource("::1", Some(5672), None, None, None, None));
        assert_eq!(uri, "amqp://[::1]:5672/%2F");
    }
}
