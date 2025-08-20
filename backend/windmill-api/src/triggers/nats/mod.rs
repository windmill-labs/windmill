use std::collections::HashMap;

use base64::{engine, Engine};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::{types::Json, FromRow};
use windmill_common::{db::UserDB, error::Result, triggers::TriggerKind, worker::to_raw_value, DB};

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
pub struct NatsTrigger;

impl TriggerJobArgs for NatsTrigger {
    type Payload = Vec<u8>;
    const TRIGGER_KIND: TriggerKind = TriggerKind::Nats;

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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "label", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NatsResourceAuth {
    #[serde(rename = "NO_AUTH")]
    NoAuth,
    #[serde(rename = "TOKEN")]
    Token { token: String },
    #[serde(rename = "USER_PASSWORD")]
    UserPass { user: String, password: String },
    #[serde(rename = "NKEY")]
    NKey { seed: String },
    #[serde(rename = "JWT")]
    JWT { jwt: String, seed: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum NatsTriggerConfigConnection {
    Resource { nats_resource_path: String },
    Static { servers: Vec<String>, auth: NatsResourceAuth, require_tls: bool },
}

#[derive(Deserialize)]
struct NatsResource {
    servers: Vec<String>,
    auth: NatsResourceAuth,
    require_tls: bool,
}

impl NatsTriggerConfigConnection {
    pub async fn into_servers_auth_and_tls(
        &self,
        authed: &ApiAuthed,
        db: &DB,
        w_id: &str,
    ) -> Result<(Vec<String>, NatsResourceAuth, bool)> {
        match self {
            NatsTriggerConfigConnection::Resource { nats_resource_path } => {
                let resource = try_get_resource_from_db_as::<NatsResource>(
                    authed,
                    Some(UserDB::new(db.clone())),
                    db,
                    w_id,
                    nats_resource_path,
                )
                .await?;

                Ok((resource.servers, resource.auth, resource.require_tls))
            }
            NatsTriggerConfigConnection::Static { servers, auth, require_tls } => {
                let auth = match auth {
                    NatsResourceAuth::Token { token } => NatsResourceAuth::Token {
                        token: interpolate(authed, db, w_id, token.clone()).await?,
                    },
                    NatsResourceAuth::UserPass { user, password } => NatsResourceAuth::UserPass {
                        user: interpolate(authed, db, w_id, user.clone()).await?,
                        password: interpolate(authed, db, w_id, password.clone()).await?,
                    },
                    NatsResourceAuth::NKey { seed } => NatsResourceAuth::NKey {
                        seed: interpolate(authed, db, w_id, seed.clone()).await?,
                    },
                    NatsResourceAuth::JWT { jwt, seed } => NatsResourceAuth::JWT {
                        jwt: interpolate(authed, db, w_id, jwt.clone()).await?,
                        seed: interpolate(authed, db, w_id, seed.clone()).await?,
                    },
                    auth => auth.clone(),
                };

                Ok((servers.clone(), auth, *require_tls))
            }
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NatsConfig {
    pub nats_resource_path: String,
    pub subjects: Vec<String>,
    pub stream_name: Option<String>,
    pub consumer_name: Option<String>,
    pub use_jetstream: bool,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub connection: Option<Json<NatsTriggerConfigConnection>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewNatsConfig {
    pub nats_resource_path: String,
    pub subjects: Vec<String>,
    pub stream_name: Option<String>,
    pub consumer_name: Option<String>,
    pub use_jetstream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditNatsConfig {
    pub nats_resource_path: String,
    pub subjects: Vec<String>,
    pub stream_name: Option<String>,
    pub consumer_name: Option<String>,
    pub use_jetstream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestNatsConfig {
    pub nats_resource_path: String,
}

// Utility functions
pub fn validate_nats_resource_path(path: &str) -> windmill_common::error::Result<()> {
    if path.trim().is_empty() {
        return Err(windmill_common::error::Error::BadRequest(
            "NATS resource path cannot be empty".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_subjects(subjects: &[String]) -> windmill_common::error::Result<()> {
    if subjects.is_empty() {
        return Err(windmill_common::error::Error::BadRequest(
            "Subjects cannot be empty".to_string(),
        ));
    }

    for subject in subjects {
        if subject.trim().is_empty() {
            return Err(windmill_common::error::Error::BadRequest(
                "Subject names cannot be empty".to_string(),
            ));
        }
    }
    Ok(())
}
