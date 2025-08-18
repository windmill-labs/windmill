use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::{types::Json as SqlxJson, FromRow};
use std::{collections::HashMap, fmt::Debug};
use windmill_common::{error::JsonResult, DB};

#[cfg(all(feature = "gcp_trigger", feature = "enterprise"))]
pub mod gcp;
#[cfg(feature = "http_trigger")]
pub mod http;
#[cfg(all(feature = "kafka", feature = "enterprise"))]
pub mod kafka;
#[cfg(feature = "mqtt_trigger")]
pub mod mqtt;
#[cfg(all(feature = "nats", feature = "enterprise"))]
pub mod nats;
#[cfg(feature = "postgres_trigger")]
pub mod postgres;
#[cfg(all(feature = "sqs_trigger", feature = "enterprise"))]
pub mod sqs;
#[cfg(feature = "websocket")]
pub mod websocket;

mod handler;
mod listener;

pub use handler::generate_trigger_routers;
pub(crate) use handler::TriggerCrud;
pub use listener::start_all_listeners;
pub(crate) use listener::Listener;

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
    pub error_handler_args: Option<SqlxJson<HashMap<String, Box<RawValue>>>>,
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
pub struct BaseEditTrigger {
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditTrigger<T> {
    #[serde(flatten)]
    pub base: BaseEditTrigger,

    #[serde(flatten)]
    pub error_handling: TriggerErrorHandling,

    #[serde(flatten)]
    pub config: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseCreateTrigger {
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTrigger<T: Debug> {
    #[serde(flatten)]
    pub base: BaseCreateTrigger,

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

    #[cfg(feature = "http_trigger")]
    let http_routes_count = {
        use crate::triggers::http::handler::HttpTriggerHandler;
        let mut tx = db.begin().await?;
        let count = HttpTriggerHandler
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await;
        tx.rollback().await.ok();
        count
    };
    #[cfg(not(feature = "http_trigger"))]
    let http_routes_count = 0;

    #[cfg(feature = "websocket")]
    let websocket_count = {
        use crate::triggers::websocket::handler::WebsocketTriggerHandler;
        let mut tx = db.begin().await?;
        let count = WebsocketTriggerHandler
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await;
        tx.rollback().await.ok();
        count
    };
    #[cfg(not(feature = "websocket"))]
    let websocket_count = 0;

    #[cfg(all(feature = "kafka", feature = "enterprise"))]
    let kafka_count = {
        use crate::triggers::kafka::handler_oss::KafkaTriggerHandler;
        let mut tx = db.begin().await?;
        let count = KafkaTriggerHandler
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await;
        tx.rollback().await.ok();
        count
    };
    #[cfg(not(all(feature = "kafka", feature = "enterprise")))]
    let kafka_count = 0;

    #[cfg(all(feature = "nats", feature = "enterprise"))]
    let nats_count = {
        use crate::triggers::nats::handler_oss::NatsTriggerHandler;
        let mut tx = db.begin().await?;
        let count = NatsTriggerHandler
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await;
        tx.rollback().await.ok();
        count
    };
    #[cfg(not(all(feature = "nats", feature = "enterprise")))]
    let nats_count = 0;

    #[cfg(feature = "postgres_trigger")]
    let postgres_count = {
        use crate::triggers::postgres::PostgresTriggerHandler;
        let mut tx = db.begin().await?;
        let count = PostgresTriggerHandler
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await;
        tx.rollback().await.ok();
        count
    };
    #[cfg(not(feature = "postgres_trigger"))]
    let postgres_count = 0;

    #[cfg(feature = "mqtt_trigger")]
    let mqtt_count = {
        use crate::triggers::mqtt::handler::MqttTriggerHandler;
        let mut tx = db.begin().await?;
        let count = MqttTriggerHandler
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await;
        tx.rollback().await.ok();
        count
    };
    #[cfg(not(feature = "mqtt_trigger"))]
    let mqtt_count = 0;

    #[cfg(all(feature = "sqs_trigger", feature = "enterprise"))]
    let sqs_count = {
        use crate::triggers::sqs::handler_oss::SqsTriggerHandler;
        let mut tx = db.begin().await?;
        let count = SqsTriggerHandler
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await;
        tx.rollback().await.ok();
        count
    };
    #[cfg(not(all(feature = "sqs_trigger", feature = "enterprise")))]
    let sqs_count = 0;

    #[cfg(all(feature = "gcp_trigger", feature = "enterprise"))]
    let gcp_count = {
        use crate::triggers::gcp::handler_oss::GcpTriggerHandler;
        let mut tx = db.begin().await?;
        let count = GcpTriggerHandler
            .trigger_count(&mut tx, w_id, is_flow, path)
            .await;
        tx.rollback().await.ok();
        count
    };
    #[cfg(not(all(feature = "gcp_trigger", feature = "enterprise")))]
    let gcp_count = 0;

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
