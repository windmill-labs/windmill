use axum::{async_trait, Json};
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::value::RawValue;
use sql_builder::{bind::Bind, SqlBuilder};
use sqlx::{types::Json as SqlxJson, FromRow, PgExecutor};
use std::collections::HashMap;
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    jobs::JobTriggerKind,
    DB,
};

use crate::db::ApiAuthed;

pub mod routes;
#[cfg(feature = "websocket")]
pub mod websocket;

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

#[derive(FromRow, Serialize)]
pub struct TruncatedTokenWithEmail {
    pub label: Option<String>,
    pub token_prefix: Option<String>,
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: chrono::DateTime<chrono::Utc>,
    pub scopes: Option<Vec<String>>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardTriggerQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_desc: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_exact: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_archived: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_flow: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
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
    pub extra_perms: serde_json::Value,
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

#[derive(FromRow, Serialize, Deserialize, Clone)]
pub struct Trigger<T> {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub base: BaseTrigger,

    #[sqlx(flatten)]
    #[serde(flatten)]
    pub config: T,

    #[sqlx(flatten)]
    #[serde(flatten)]
    pub server_state: ServerState,

    #[sqlx(flatten)]
    #[serde(flatten)]
    pub error_handling: TriggerErrorHandling,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseEditTrigger {
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
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
pub struct CreateTrigger<T> {
    #[serde(flatten)]
    pub base: BaseCreateTrigger,

    #[serde(flatten)]
    pub error_handling: TriggerErrorHandling,

    #[serde(flatten)]
    pub config: T,
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

    pub fn order_field(&self) -> &str {
        self.order_by.as_deref().unwrap_or("edited_at")
    }
}

impl Default for StandardTriggerQuery {
    fn default() -> Self {
        Self {
            page: Some(0),
            per_page: Some(100),
            order_desc: Some(true),
            created_by: None,
            path: None,
            path_start: None,
            path_exact: None,
            first_kind: None,
            last_kind: None,
            starred_only: None,
            show_archived: None,
            order_by: Some("edited_at".to_string()),
            is_flow: None,
            enabled: None,
        }
    }
}

impl Default for TriggerErrorHandling {
    fn default() -> Self {
        Self { error_handler_path: None, error_handler_args: None, retry: None }
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self { enabled: true, server_id: None, last_server_ping: None, error: None }
    }
}

impl BaseTrigger {
    pub fn new(
        workspace_id: String,
        path: String,
        script_path: String,
        is_flow: bool,
        edited_by: String,
        email: String,
    ) -> Self {
        Self {
            workspace_id,
            path,
            script_path,
            is_flow,
            edited_by,
            email,
            edited_at: Utc::now(),
            extra_perms: serde_json::json!({}),
        }
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

    type TriggerConfig: DeserializeOwned + Serialize;
    type EditTriggerConfig: DeserializeOwned + Serialize + Send + Sync;
    type NewTriggerConfig: DeserializeOwned + Serialize + Send + Sync;
    type TestConnectionConfig: DeserializeOwned + Serialize + Send + Sync;

    const TABLE_NAME: &'static str;
    const TRIGGER_TYPE: &'static str;
    const SCOPE_NAME: &'static str = Self::TRIGGER_TYPE;
    const TRIGGER_KIND: JobTriggerKind;
    const SUPPORTS_ENABLED: bool = true;
    const SUPPORTS_SERVER_STATE: bool = true;
    const SUPPORTS_TEST_CONNECTION: bool = false;
    const ROUTE_PREFIX: &'static str;
    const DEPLOYMENT_NAME: &'static str;

    async fn validate_new(&self, _workspace_id: &str, _new: &Self::NewTriggerConfig) -> Result<()> {
        Ok(())
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

    fn get_scope(&self, operation: &str, path: &str) -> String {
        format!("triggers.{}.{}:{}", operation, Self::TRIGGER_TYPE, path)
    }

    async fn create_trigger<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
        authed: &ApiAuthed,
        w_id: &str,
        trigger: &CreateTrigger<Self::NewTriggerConfig>,
    ) -> Result<()>;

    async fn update_trigger<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
        authed: &ApiAuthed,
        workspace_id: &str,
        path: &str,
        trigger: &EditTrigger<Self::EditTriggerConfig>,
    ) -> Result<()>;

    async fn test_connection(
        &self,
        _db: &DB,
        _authed: &ApiAuthed,
        _user_db: &UserDB,
        _workspace_id: &str,
        _config: &Self::TestConnectionConfig,
    ) -> Result<serde_json::Value> {
        Err(
            anyhow::anyhow!("Test connection not supported for this trigger type".to_string(),)
                .into(),
        )
    }

    fn additional_routes(&self) -> axum::Router {
        axum::Router::new()
    }

    async fn get_trigger_by_path<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
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
            .fetch_optional(executor)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Trigger not found at path: {}", path)))
    }

    fn validate_trigger(_trigger: &Self::Trigger) -> Result<()> {
        Ok(())
    }

    async fn exists<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
        workspace_id: &str,
        path: &str,
    ) -> Result<bool> {
        let exists = sqlx::query_scalar(&format!(
            "SELECT EXISTS(SELECT 1 FROM {} WHERE workspace_id = $1 AND path = $2)",
            Self::TABLE_NAME
        ))
        .bind(workspace_id)
        .bind(path)
        .fetch_one(executor)
        .await?;

        Ok(exists)
    }

    async fn delete_by_path<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
        workspace_id: &str,
        path: &str,
    ) -> Result<bool> {
        let deleted = sqlx::query(&format!(
            "DELETE FROM {} WHERE workspace_id = $1 AND path = $2",
            Self::TABLE_NAME
        ))
        .bind(workspace_id)
        .bind(path)
        .execute(executor)
        .await?
        .rows_affected();

        Ok(deleted > 0)
    }

    async fn set_enabled<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
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
        .execute(executor)
        .await?
        .rows_affected();

        Ok(updated > 0)
    }

    async fn trigger_count<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
        workspace_id: &str,
        is_flow: bool,
        script_path: &str,
    ) -> Result<i64> {
        let count = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {} WHERE workspace_id = $1 AND is_flow = $2 AND script_path = $3",
            Self::TABLE_NAME
        ))
        .bind(workspace_id)
        .bind(is_flow)
        .bind(script_path)
        .fetch_one(executor)
        .await?;

        Ok(count)
    }

    async fn list_triggers<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
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

        sqlb.fields(&fields)
            .order_by(query.order_field(), query.order_desc.unwrap_or(true))
            .and_where("workspace_id = ?".bind(&workspace_id))
            .offset(query.offset())
            .limit(query.limit());

        if let Some(path) = &query.path {
            sqlb.and_where_eq("script_path", "?".bind(path));
        }

        if let Some(is_flow) = query.is_flow {
            sqlb.and_where_eq("is_flow", "?".bind(&is_flow));
        }

        if let Some(path_start) = &query.path_start {
            sqlb.and_where_like_left("path", path_start);
        }

        if let Some(enabled) = query.enabled {
            if Self::SUPPORTS_SERVER_STATE {
                sqlb.and_where_eq("enabled", "?".bind(&enabled));
            }
        }

        let sql = sqlb
            .sql()
            .map_err(|e| Error::InternalErr(format!("SQL error: {}", e)))?;

        let triggers = sqlx::query_as(&sql).fetch_all(executor).await?;

        Ok(triggers)
    }
}

#[macro_export]
macro_rules! trigger_sql {
    (select_fields, $trigger_type:expr, $supports_server_state:expr) => {{
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

        if $supports_server_state {
            fields.extend_from_slice(&["enabled", "server_id", "last_server_ping", "error"]);
        }

        fields.extend_from_slice(&["error_handler_path", "error_handler_args", "retry"]);
        fields
    }};
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

    let http_routes_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM http_trigger WHERE script_path = $1 AND is_flow = $2 AND workspace_id = $3",
        path,
        is_flow,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    let websocket_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM websocket_trigger WHERE script_path = $1 AND is_flow = $2 AND workspace_id = $3",
        path,
        is_flow,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    let kafka_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM kafka_trigger WHERE script_path = $1 AND is_flow = $2 AND workspace_id = $3",
        path,
        is_flow,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    let nats_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM nats_trigger WHERE script_path = $1 AND is_flow = $2 AND workspace_id = $3",
        path,
        is_flow,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    let postgres_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM postgres_trigger WHERE script_path = $1 AND is_flow = $2 AND workspace_id = $3",
        path,
        is_flow,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    let mqtt_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM mqtt_trigger WHERE script_path = $1 AND is_flow = $2 AND workspace_id = $3",
        path,
        is_flow,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    let sqs_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM sqs_trigger WHERE script_path = $1 AND is_flow = $2 AND workspace_id = $3",
        path,
        is_flow,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

    let gcp_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM gcp_trigger WHERE script_path = $1 AND is_flow = $2 AND workspace_id = $3",
        path,
        is_flow,
        w_id
    )
    .fetch_one(db)
    .await?
    .unwrap_or(0);

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

pub async fn list_tokens_internal(
    db: &DB,
    w_id: &str,
    path: &str,
    is_flow: bool,
) -> JsonResult<Vec<TruncatedTokenWithEmail>> {
    let tokens = if is_flow {
        sqlx::query_as!(
            TruncatedTokenWithEmail,
            r#"
        SELECT label,
               concat(substring(token for 10)) AS token_prefix,
               expiration,
               created_at,
               last_used_at,
               scopes,
               email
        FROM token
        WHERE workspace_id = $1
          AND (
               scopes @> ARRAY['jobs:run:flows:' || $2]::text[]
               OR scopes @> ARRAY['run:flows/' || $2]::text[]
              )
        "#,
            w_id,
            path
        )
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as!(
            TruncatedTokenWithEmail,
            r#"
        SELECT label,
               concat(substring(token for 10)) AS token_prefix,
               expiration,
               created_at,
               last_used_at,
               scopes,
               email
        FROM token
        WHERE workspace_id = $1
          AND (
               scopes @> ARRAY['jobs:run:scripts:' || $2]::text[]
               OR scopes @> ARRAY['run:scripts/' || $2]::text[]
              )
        "#,
            w_id,
            path
        )
        .fetch_all(db)
        .await?
    };

    Ok(Json(tokens))
}
