use std::collections::HashMap;

use base64::{engine, Engine};
use itertools::Itertools;
use rdkafka::{
    consumer::{Consumer, DefaultConsumerContext, StreamConsumer},
    metadata::Metadata,
    ClientConfig,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::{types::Json, FromRow};
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    triggers::TriggerKind,
    worker::to_raw_value,
    DB,
};

use crate::{
    db::ApiAuthed, ee_oss::interpolate, resources::try_get_resource_from_db_as,
    triggers::trigger_helpers::TriggerJobArgs,
};

#[cfg(feature = "private")]
mod handler_ee;
pub mod handler_oss;
#[cfg(feature = "private")]
mod listener_ee;
pub mod listener_oss;

#[derive(Copy, Clone)]
pub struct KafkaTrigger;

impl TriggerJobArgs for KafkaTrigger {
    type Payload = Vec<u8>;
    const TRIGGER_KIND: TriggerKind = TriggerKind::Kafka;
    fn v1_payload_fn(payload: &Self::Payload) -> HashMap<String, Box<RawValue>> {
        let s = String::from_utf8(payload.to_vec())
            .unwrap_or_else(|_| "Error: invalid utf8 payload".to_string());
        HashMap::from([("msg".to_string(), to_raw_value(&s))])
    }

    fn v2_payload_fn(payload: &Self::Payload) -> HashMap<String, Box<RawValue>> {
        let base64_payload = engine::general_purpose::STANDARD.encode(payload);
        HashMap::from([("payload".to_string(), to_raw_value(&base64_payload))])
    }
}

// Kafka Configuration structs for the trait
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct KafkaConfig {
    pub kafka_resource_path: String,
    pub group_id: String,
    pub topics: Vec<String>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub connection: Option<Json<KafkaTriggerConfigConnection>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewKafkaConfig {
    pub kafka_resource_path: String,
    pub group_id: String,
    pub topics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditKafkaConfig {
    pub kafka_resource_path: String,
    pub group_id: String,
    pub topics: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "label", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KafkaResourceSecurity {
    Plaintext,
    SaslPlaintext {
        mechanism: String,
        username: String,
        password: String,
    },
    Ssl {
        ca: Option<String>,
        certificate: Option<String>,
        key: Option<String>,
        key_password: Option<String>,
    },
    SaslSsl {
        mechanism: String,
        username: String,
        password: String,
        ca: Option<String>,
        certificate: Option<String>,
        key: Option<String>,
        key_password: Option<String>,
    },
}

async fn interpolate_option(
    authed: &ApiAuthed,
    db: &DB,
    w_id: &str,
    s: Option<String>,
) -> std::result::Result<Option<String>, anyhow::Error> {
    if let Some(s) = s {
        Ok(Some(interpolate(authed, db, w_id, s).await?))
    } else {
        Ok(None)
    }
}

#[derive(Deserialize)]
struct KafkaResource {
    brokers: Vec<String>,
    security: KafkaResourceSecurity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KafkaTriggerConfigConnection {
    Resource { kafka_resource_path: String },
    Static { brokers: Vec<String>, security: KafkaResourceSecurity },
}

impl KafkaTriggerConfigConnection {
    pub async fn into_brokers_and_security(
        &self,
        authed: &ApiAuthed,
        db: &DB,
        w_id: &str,
    ) -> Result<(Vec<String>, KafkaResourceSecurity)> {
        match self {
            KafkaTriggerConfigConnection::Resource { kafka_resource_path } => {
                let resource = try_get_resource_from_db_as::<KafkaResource>(
                    authed,
                    Some(UserDB::new(db.clone())),
                    db,
                    w_id,
                    kafka_resource_path,
                )
                .await?;

                Ok((resource.brokers, resource.security))
            }
            KafkaTriggerConfigConnection::Static { brokers, security } => {
                let security = match security {
                    KafkaResourceSecurity::SaslPlaintext { mechanism, username, password } => {
                        KafkaResourceSecurity::SaslPlaintext {
                            mechanism: interpolate(authed, db, w_id, mechanism.clone()).await?,
                            username: interpolate(authed, db, w_id, username.clone()).await?,
                            password: interpolate(authed, db, w_id, password.clone()).await?,
                        }
                    }
                    KafkaResourceSecurity::SaslSsl {
                        mechanism,
                        username,
                        password,
                        ca,
                        certificate,
                        key,
                        key_password,
                    } => KafkaResourceSecurity::SaslSsl {
                        mechanism: interpolate(authed, db, w_id, mechanism.clone()).await?,
                        username: interpolate(authed, db, w_id, username.clone()).await?,
                        password: interpolate(authed, db, w_id, password.clone()).await?,
                        ca: interpolate_option(authed, db, w_id, ca.clone()).await?,
                        certificate: interpolate_option(authed, db, w_id, certificate.clone())
                            .await?,
                        key: interpolate_option(authed, db, w_id, key.clone()).await?,
                        key_password: interpolate_option(authed, db, w_id, key_password.clone())
                            .await?,
                    },
                    KafkaResourceSecurity::Ssl { ca, certificate, key, key_password } => {
                        KafkaResourceSecurity::Ssl {
                            ca: interpolate_option(authed, db, w_id, ca.clone()).await?,
                            certificate: interpolate_option(authed, db, w_id, certificate.clone())
                                .await?,
                            key: interpolate_option(authed, db, w_id, key.clone()).await?,
                            key_password: interpolate_option(
                                authed,
                                db,
                                w_id,
                                key_password.clone(),
                            )
                            .await?,
                        }
                    }
                    sec => sec.clone(),
                };

                Ok((brokers.clone(), security))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestKafkaConfig {
    connection: KafkaTriggerConfigConnection,
}

async fn fetch_metadata(
    consumer: StreamConsumer<DefaultConsumerContext>,
    timeout: std::time::Duration,
) -> Result<(StreamConsumer<DefaultConsumerContext>, Metadata)> {
    // fetch_metadata is blocking, so we need to spawn a blocking task
    let result = tokio::task::spawn_blocking(move || {
        let metadata = consumer
            .fetch_metadata(None, timeout)
            .map_err(|e| Error::internal_err(format!("Failed to fetch kafka metadata: {}", e)))?;

        Ok::<_, Error>((consumer, metadata))
    })
    .await
    .map_err(|e| Error::internal_err(format!("Failed to start kafka metadata fetch: {}", e)))??;

    Ok(result)
}

fn create_consumer_config(
    brokers: &[String],
    security: KafkaResourceSecurity,
    group_id: Option<&str>,
) -> ClientConfig {
    let mut config = ClientConfig::new();
    if let Some(group_id) = group_id {
        config.set("group.id", group_id);
    }
    config
        .set("bootstrap.servers", brokers.iter().join(","))
        .set("enable.partition.eof", "false")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "latest");
    match security {
        KafkaResourceSecurity::Plaintext => {
            config.set("security.protocol", "plaintext");
        }
        KafkaResourceSecurity::SaslPlaintext { mechanism, username, password } => {
            config
                .set("security.protocol", "sasl_plaintext")
                .set("sasl.mechanism", mechanism)
                .set("sasl.username", username)
                .set("sasl.password", password);
        }
        KafkaResourceSecurity::Ssl { ca, certificate, key, key_password } => {
            config.set("security.protocol", "ssl");
            if let Some(ca) = ca.filter(|s| !s.is_empty()) {
                config.set("ssl.ca.pem", ca);
            }
            if let Some(certificate) = certificate.filter(|s| !s.is_empty()) {
                config.set("ssl.certificate.pem", certificate);
            }
            if let Some(key) = key.filter(|s| !s.is_empty()) {
                config.set("ssl.key.pem", key);
            }
            if let Some(key_password) = key_password.filter(|s| !s.is_empty()) {
                config.set("ssl.key.password", key_password);
            }
        }
        KafkaResourceSecurity::SaslSsl {
            username,
            password,
            mechanism,
            ca,
            certificate,
            key,
            key_password,
        } => {
            config
                .set("security.protocol", "sasl_ssl")
                .set("sasl.mechanism", mechanism)
                .set("sasl.username", username)
                .set("sasl.password", password);

            if let Some(ca) = ca.filter(|s| !s.is_empty()) {
                config.set("ssl.ca.pem", ca);
            }
            if let Some(certificate) = certificate.filter(|s| !s.is_empty()) {
                config.set("ssl.certificate.pem", certificate);
            }
            if let Some(key) = key.filter(|s| !s.is_empty()) {
                config.set("ssl.key.pem", key);
            }
            if let Some(key_password) = key_password.filter(|s| !s.is_empty()) {
                config.set("ssl.key.password", key_password);
            }
        }
    }

    config
}

pub fn validate_kafka_resource_path(path: &str) -> windmill_common::error::Result<()> {
    if path.trim().is_empty() {
        return Err(windmill_common::error::Error::BadRequest(
            "Kafka resource path cannot be empty".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_group_id(group_id: &str) -> windmill_common::error::Result<()> {
    if group_id.trim().is_empty() {
        return Err(windmill_common::error::Error::BadRequest(
            "Group ID cannot be empty".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_topics(topics: &[String]) -> windmill_common::error::Result<()> {
    if topics.is_empty() {
        return Err(windmill_common::error::Error::BadRequest(
            "At least one topic must be specified".to_string(),
        ));
    }

    for topic in topics {
        if topic.trim().is_empty() {
            return Err(windmill_common::error::Error::BadRequest(
                "Topic names cannot be empty".to_string(),
            ));
        }
    }
    Ok(())
}
