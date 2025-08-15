use axum::{async_trait, Json};
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::value::RawValue;
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::{types::Json as SqlxJson, FromRow, PgConnection};
use std::{collections::HashMap, fmt::Debug};
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    jobs::JobTriggerKind,
    utils::{paginate, Pagination},
    DB,
};
use windmill_git_sync::DeployedObject;

use crate::db::ApiAuthed;

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

mod routes;

pub use routes::generate_trigger_routers;

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
    pub retry: Option<sqlx::types::Json<Box<RawValue>>>,
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

#[async_trait]
pub trait TriggerCrud: Send + Sync + 'static {
    type Trigger: Serialize
        + DeserializeOwned
        + for<'r> FromRow<'r, sqlx::postgres::PgRow>
        + Send
        + Sync
        + Unpin;

    type TriggerConfig: Debug + DeserializeOwned + Serialize;
    type EditTriggerConfig: Debug + DeserializeOwned + Serialize + Send + Sync;
    type NewTriggerConfig: Debug + DeserializeOwned + Serialize + Send + Sync;
    type TestConnectionConfig: Debug + DeserializeOwned + Serialize + Send + Sync;

    const TABLE_NAME: &'static str;
    const TRIGGER_TYPE: &'static str;
    const SCOPE_NAME: &'static str = Self::TRIGGER_TYPE;
    const TRIGGER_KIND: JobTriggerKind;
    const SUPPORTS_ENABLED: bool = true;
    const SUPPORTS_SERVER_STATE: bool = true;
    const SUPPORTS_TEST_CONNECTION: bool = false;
    const ROUTE_PREFIX: &'static str;
    const DEPLOYMENT_NAME: &'static str;

    fn get_deployed_object(path: String) -> DeployedObject;

    async fn validate_new(&self, _workspace_id: &str, _new: &Self::NewTriggerConfig) -> Result<()> {
        Ok(())
    }

    fn scope_domain_name() -> &'static str {
        &Self::ROUTE_PREFIX[1..]
    }

    async fn validate_edit(
        &self,
        _workspace_id: &str,
        _path: &str,
        _edit: &Self::EditTriggerConfig,
    ) -> Result<()> {
        Ok(())
    }

    fn additional_select_fields(&self) -> Vec<&'static str> {
        vec![]
    }

    async fn create_trigger(
        &self,
        db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        w_id: &str,
        trigger: CreateTrigger<Self::NewTriggerConfig>,
    ) -> Result<()>;

    async fn update_trigger(
        &self,
        db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        workspace_id: &str,
        path: &str,
        trigger: EditTrigger<Self::EditTriggerConfig>,
    ) -> Result<()>;

    async fn test_connection(
        &self,
        _db: &DB,
        _authed: &ApiAuthed,
        _user_db: &UserDB,
        _workspace_id: &str,
        _config: Self::TestConnectionConfig,
    ) -> Result<()> {
        Err(
            anyhow::anyhow!("Test connection not supported for this trigger type".to_string(),)
                .into(),
        )
    }

    fn additional_routes(&self) -> axum::Router {
        axum::Router::new()
    }

    async fn get_trigger_by_path(
        &self,
        tx: &mut PgConnection,
        workspace_id: &str,
        path: &str,
    ) -> Result<Self::Trigger> {
        let mut fields = vec![
            "workspace_id",
            "path",
            "script_path",
            "is_flow",
            "edited_by",
            "email",
            "edited_at",
            "extra_perms",
        ];

        if Self::SUPPORTS_SERVER_STATE {
            fields.extend_from_slice(&["enabled", "server_id", "last_server_ping", "error"]);
        }

        fields.extend_from_slice(&["error_handler_path", "error_handler_args", "retry"]);
        fields.extend(self.additional_select_fields());

        let sql = format!(
            "SELECT {} FROM {} WHERE workspace_id = $1 AND path = $2",
            fields.join(", "),
            Self::TABLE_NAME
        );

        sqlx::query_as(&sql)
            .bind(workspace_id)
            .bind(path)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Trigger not found at path: {}", path)))
    }

    async fn exists(&self, tx: &mut PgConnection, workspace_id: &str, path: &str) -> Result<bool> {
        let exists = sqlx::query_scalar(&format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE workspace_id = $1 AND path = $2)",
            Self::TABLE_NAME
        ))
        .bind(workspace_id)
        .bind(path)
        .fetch_one(&mut *tx)
        .await?;

        Ok(exists)
    }

    async fn delete_by_path(
        &self,
        tx: &mut PgConnection,
        workspace_id: &str,
        path: &str,
    ) -> Result<bool> {
        let deleted = sqlx::query(&format!(
            "DELETE FROM {} WHERE workspace_id = $1 AND path = $2",
            Self::TABLE_NAME
        ))
        .bind(workspace_id)
        .bind(path)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        Ok(deleted > 0)
    }

    async fn set_enabled(
        &self,
        tx: &mut PgConnection,
        workspace_id: &str,
        path: &str,
        enabled: bool,
    ) -> Result<bool> {
        if !Self::SUPPORTS_SERVER_STATE {
            return Err(anyhow::anyhow!(
                "Enable/disable not supported for this trigger type".to_string(),
            )
            .into());
        }

        let updated = sqlx::query(&format!(
            "UPDATE {} SET enabled = $3 WHERE workspace_id = $1 AND path = $2",
            Self::TABLE_NAME
        ))
        .bind(workspace_id)
        .bind(path)
        .bind(enabled)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        Ok(updated > 0)
    }

    async fn trigger_count(
        &self,
        tx: &mut PgConnection,
        workspace_id: &str,
        is_flow: bool,
        script_path: &str,
    ) -> i64 {
        let count = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {} WHERE workspace_id = $1 AND is_flow = $2 AND script_path = $3",
            Self::TABLE_NAME
        ))
        .bind(workspace_id)
        .bind(is_flow)
        .bind(script_path)
        .fetch_one(&mut *tx)
        .await
        .unwrap_or(0);

        count
    }

    async fn list_triggers(
        &self,
        tx: &mut PgConnection,
        workspace_id: &str,
        query: &StandardTriggerQuery,
    ) -> Result<Vec<Self::Trigger>> {
        let mut fields = vec![
            "workspace_id",
            "path",
            "script_path",
            "is_flow",
            "edited_by",
            "email",
            "edited_at",
            "extra_perms",
        ];

        if Self::SUPPORTS_SERVER_STATE {
            fields.extend_from_slice(&["enabled", "server_id", "last_server_ping", "error"]);
        }

        fields.extend_from_slice(&["error_handler_path", "error_handler_args", "retry"]);
        fields.extend(self.additional_select_fields());

        let mut sqlb = SqlBuilder::select_from(Self::TABLE_NAME);

        let (per_page, offset) =
            paginate(Pagination { per_page: query.per_page, page: query.page });

        sqlb.fields(&fields)
            .order_by("edited_at", true)
            .and_where("workspace_id = ?".bind(&workspace_id))
            .offset(offset)
            .limit(per_page);

        if let Some(path) = &query.path {
            sqlb.and_where_eq("script_path", "?".bind(path));
        }

        if let Some(is_flow) = query.is_flow {
            sqlb.and_where_eq("is_flow", "?".bind(&is_flow));
        }

        if let Some(path_start) = &query.path_start {
            sqlb.and_where_like_left("path", path_start);
        }

        let sql = sqlb
            .sql()
            .map_err(|e| Error::InternalErr(format!("SQL error: {}", e)))?;

        let triggers = sqlx::query_as(&sql).fetch_all(&mut *tx).await?;

        Ok(triggers)
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
        use crate::triggers::postgres::handler::PostgresTriggerHandler;
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
