use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{types::Json as SqlxJson, FromRow};
use std::{collections::HashMap, fmt::Debug};
use windmill_common::{error::JsonResult, DB};

#[derive(Debug, Clone)]
pub struct EnabledTriggers {
    pub http: bool,
    pub websocket: bool,
    pub kafka: bool,
    pub nats: bool,
    pub postgres: bool,
    pub mqtt: bool,
    pub sqs: bool,
    pub gcp: bool,
}

impl EnabledTriggers {
    pub fn detect() -> Self {
        Self {
            http: cfg!(feature = "http_trigger"),
            websocket: cfg!(feature = "websocket"),
            kafka: cfg!(all(feature = "kafka", feature = "enterprise", feature = "private")),
            nats: cfg!(all(feature = "nats", feature = "enterprise", feature = "private")),
            postgres: cfg!(feature = "postgres_trigger"),
            mqtt: cfg!(feature = "mqtt_trigger"),
            sqs: cfg!(all(feature = "sqs_trigger", feature = "enterprise", feature = "private")),
            gcp: cfg!(all(feature = "gcp_trigger", feature = "enterprise", feature = "private")),
        }
    }

    pub fn enabled_types(&self) -> Vec<&'static str> {
        let mut types = Vec::new();
        if self.http {
            types.push("http");
        }
        if self.websocket {
            types.push("websocket");
        }
        if self.kafka {
            types.push("kafka");
        }
        if self.nats {
            types.push("nats");
        }
        if self.postgres {
            types.push("postgres");
        }
        if self.mqtt {
            types.push("mqtt");
        }
        if self.sqs {
            types.push("sqs");
        }
        if self.gcp {
            types.push("gcp");
        }
        types
    }
}

lazy_static::lazy_static! {
    pub static ref ENABLED_TRIGGERS: EnabledTriggers = EnabledTriggers::detect();
}

#[cfg(all(feature = "gcp_trigger", feature = "enterprise", feature = "private"))]
pub mod gcp;
#[cfg(feature = "http_trigger")]
pub mod http;
#[cfg(all(feature = "kafka", feature = "enterprise", feature = "private"))]
pub mod kafka;
#[cfg(feature = "mqtt_trigger")]
pub mod mqtt;
#[cfg(all(feature = "nats", feature = "enterprise", feature = "private"))]
pub mod nats;
#[cfg(feature = "postgres_trigger")]
pub mod postgres;
#[cfg(all(feature = "sqs_trigger", feature = "enterprise", feature = "private"))]
pub mod sqs;
#[cfg(feature = "websocket")]
pub mod websocket;

mod handler;
pub mod trigger_helpers;

pub use handler::generate_trigger_routers;
#[allow(unused)]
pub(crate) use handler::TriggerCrud;

#[derive(Serialize, Deserialize, Debug)]
pub struct TriggerPrimarySchedule {
    schedule: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TriggersCount {
    primary_schedule: Option<TriggerPrimarySchedule>,
    schedule_count: i64,
    http_routes_count: i64,
    webhook_count: i64,
    email_count: i64,
    websocket_count: i64,
    kafka_count: i64,
    nats_count: i64,
    postgres_count: i64,
    mqtt_count: i64,
    sqs_count: i64,
    gcp_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardTriggerQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub path: Option<String>,
    pub is_flow: Option<bool>,
    pub path_start: Option<String>,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct BaseTrigger {
    pub workspace_id: String,
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    pub edited_by: String,
    pub email: String,
    pub edited_at: DateTime<Utc>,
    pub extra_perms: Option<serde_json::Value>,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct ServerState {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_server_ping: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct TriggerErrorHandling {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handler_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handler_args: Option<SqlxJson<HashMap<String, serde_json::Value>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<sqlx::types::Json<windmill_common::flows::Retry>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Trigger<T>
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow>,
{
    #[serde(flatten)]
    pub base: BaseTrigger,

    #[serde(flatten)]
    pub config: T,

    #[serde(flatten)]
    pub server_state: Option<ServerState>,

    #[serde(flatten)]
    pub error_handling: TriggerErrorHandling,
}

impl<T> FromRow<'_, sqlx::postgres::PgRow> for Trigger<T>
where
    T: for<'r> FromRow<'r, sqlx::postgres::PgRow>,
{
    fn from_row(row: &sqlx::postgres::PgRow) -> std::result::Result<Self, sqlx::Error> {
        Ok(Trigger {
            base: BaseTrigger::from_row(row)?,
            config: T::from_row(row)?,
            server_state: ServerState::from_row(row).ok(),
            error_handling: TriggerErrorHandling::from_row(row)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseTriggerData {
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerData<T: Debug> {
    #[serde(flatten)]
    pub base: BaseTriggerData,

    #[serde(flatten)]
    pub config: T,

    #[serde(flatten)]
    pub error_handling: TriggerErrorHandling,
}

impl StandardTriggerQuery {
    pub fn offset(&self) -> i64 {
        let page = self.page.unwrap_or(0);
        let per_page = self.per_page.unwrap_or(100);
        (page * per_page) as i64
    }

    pub fn limit(&self) -> i64 {
        self.per_page.unwrap_or(100) as i64
    }
}

impl Default for StandardTriggerQuery {
    fn default() -> Self {
        Self { page: Some(0), per_page: Some(100), path: None, path_start: None, is_flow: None }
    }
}

pub(crate) async fn get_triggers_count_internal(
    db: &DB,
    w_id: &str,
    path: &str,
    is_flow: bool,
) -> JsonResult<TriggersCount> {
    let primary_schedule = sqlx::query_scalar!(
        "SELECT schedule FROM schedule WHERE path = $1 AND script_path = $1 AND is_flow = $2 AND workspace_id = $3",
        path,
        is_flow,
        w_id
    )
    .fetch_optional(db)
    .await?;

    let schedule_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM schedule WHERE script_path = $1 AND is_flow = $2 AND workspace_id = $3",
        path,
        is_flow,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    let mut tx = db.begin().await?;

    let enabled = &*ENABLED_TRIGGERS;

    let http_routes_count = if enabled.http {
        use crate::triggers::http::handler::HttpTrigger;
        HttpTrigger
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await
    } else {
        0
    };

    let websocket_count = if enabled.websocket {
        use crate::triggers::websocket::WebsocketTrigger;
        WebsocketTrigger
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await
    } else {
        0
    };

    let kafka_count = if enabled.kafka {
        use crate::triggers::kafka::KafkaTrigger;
        KafkaTrigger
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await
    } else {
        0
    };

    let nats_count = if enabled.nats {
        use crate::triggers::nats::NatsTrigger;
        NatsTrigger
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await
    } else {
        0
    };

    let postgres_count = if enabled.postgres {
        use crate::triggers::postgres::PostgresTrigger;
        PostgresTrigger
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await
    } else {
        0
    };

    let mqtt_count = if enabled.mqtt {
        use crate::triggers::mqtt::MqttTrigger;
        MqttTrigger
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await
    } else {
        0
    };

    let sqs_count = if enabled.sqs {
        use crate::triggers::sqs::SqsTrigger;
        SqsTrigger.trigger_count(&mut tx, w_id, is_flow, path).await
    } else {
        0
    };

    let gcp_count = if enabled.gcp {
        use crate::triggers::gcp::GcpTrigger;
        GcpTrigger.trigger_count(&mut tx, w_id, is_flow, path).await
    } else {
        0
    };

    tx.commit().await?;

    let webhook_count = (if is_flow {
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM token WHERE label LIKE 'webhook-%' AND workspace_id = $1 AND scopes @> ARRAY['run:flow/' || $2]::text[]",
            w_id,
            path,
        )
    } else {
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM token WHERE label LIKE 'webhook-%' AND workspace_id = $1 AND scopes @> ARRAY['run:' || $2]::text[]",
            w_id,
            path,
        )
    }).fetch_one(db)
    .await?
    .unwrap_or(0);

    let email_count = (if is_flow {
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM token WHERE label LIKE 'email-%' AND workspace_id = $1 AND scopes @> ARRAY['run:flow/' || $2]::text[]",
            w_id,
            path,
        )
    } else {
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM token WHERE label LIKE 'email-%' AND workspace_id = $1 AND scopes @> ARRAY['run:script/' || $2]::text[]",
            w_id,
            path,
        )
    }).fetch_one(db)
    .await?
    .unwrap_or(0);

    Ok(Json(TriggersCount {
        primary_schedule: primary_schedule.map(|s| TriggerPrimarySchedule { schedule: s }),
        schedule_count,
        http_routes_count,
        webhook_count,
        email_count,
        websocket_count,
        kafka_count,
        nats_count,
        postgres_count,
        mqtt_count,
        gcp_count,
        sqs_count,
    }))
}
