use crate::{
    db::{ApiAuthed, DB},
    triggers::{CreateTrigger, EditTrigger, Trigger, TriggerCrud},
};
use axum::{async_trait, routing::Router};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::{types::Json as SqlxJson, FromRow, PgExecutor};
use tokio_tungstenite::connect_async;
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    jobs::JobTriggerKind,
};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WebsocketConfig {
    pub url: String,
    pub filters: Vec<SqlxJson<Box<RawValue>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_messages: Option<Vec<SqlxJson<Box<RawValue>>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_runnable_args: Option<SqlxJson<Box<RawValue>>>,
    pub can_return_message: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWebsocketConfig {
    pub url: String,
    pub filters: Vec<Box<RawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_messages: Option<Vec<Box<RawValue>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_runnable_args: Option<Box<RawValue>>,
    #[serde(default)]
    pub can_return_message: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditWebsocketConfig {
    pub url: String,
    pub filters: Vec<Box<RawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_messages: Option<Vec<Box<RawValue>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_runnable_args: Option<Box<RawValue>>,
    pub can_return_message: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestWebsocketConfig {
    pub url: String,
}

pub struct WebsocketTriggerHandler;

#[async_trait]
impl TriggerCrud for WebsocketTriggerHandler {
    type Trigger = Trigger<WebsocketConfig>;
    type TriggerConfig = WebsocketConfig;
    type EditTriggerConfig = EditWebsocketConfig;
    type NewTriggerConfig = NewWebsocketConfig;
    type TestConnectionConfig = TestWebsocketConfig;

    const TABLE_NAME: &'static str = "websocket_trigger";
    const TRIGGER_TYPE: &'static str = "websocket";
    const TRIGGER_KIND: JobTriggerKind = JobTriggerKind::Websocket;
    const SUPPORTS_ENABLED: bool = true;
    const SUPPORTS_SERVER_STATE: bool = true;
    const SUPPORTS_TEST_CONNECTION: bool = true;
    const ROUTE_PREFIX: &'static str = "/websocket_triggers";
    const DEPLOYMENT_NAME: &'static str = "WebSocket trigger";

    fn additional_select_fields(&self) -> Vec<&'static str> {
        vec![
            "url",
            "filters",
            "initial_messages",
            "url_runnable_args",
            "can_return_message",
        ]
    }

    async fn validate_new(&self, _workspace_id: &str, new: &Self::NewTriggerConfig) -> Result<()> {
        if new.url.trim().is_empty() {
            return Err(Error::BadRequest(
                "WebSocket URL cannot be empty".to_string(),
            ));
        }

        // Basic URL validation
        if !new.url.starts_with("ws://") && !new.url.starts_with("wss://") {
            return Err(Error::BadRequest(
                "WebSocket URL must start with ws:// or wss://".to_string(),
            ));
        }

        Ok(())
    }

    async fn validate_edit(
        &self,
        _workspace_id: &str,
        _path: &str,
        edit: &Self::EditTriggerConfig,
    ) -> Result<()> {
        if edit.url.trim().is_empty() {
            return Err(Error::BadRequest(
                "WebSocket URL cannot be empty".to_string(),
            ));
        }

        // Basic URL validation
        if !edit.url.starts_with("ws://") && !edit.url.starts_with("wss://") {
            return Err(Error::BadRequest(
                "WebSocket URL must start with ws:// or wss://".to_string(),
            ));
        }

        Ok(())
    }

    async fn create_trigger<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
        authed: &ApiAuthed,
        w_id: &str,
        trigger: &CreateTrigger<Self::NewTriggerConfig>,
    ) -> Result<()> {
        let filters = trigger
            .config
            .filters
            .into_iter()
            .map(SqlxJson)
            .collect_vec();
        let initial_messages = trigger
            .config
            .initial_messages
            .unwrap_or_default()
            .into_iter()
            .map(SqlxJson)
            .collect_vec();
        sqlx::query!(
            r#"
            INSERT INTO websocket_trigger (
                workspace_id,
                path,
                url,
                script_path,
                is_flow,
                enabled,
                filters,
                initial_messages,
                url_runnable_args,
                edited_by,
                can_return_message,
                email,
                edited_at,
                error_handler_path,
                error_handler_args,
                retry
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now(), $13, $14, $15
            )
            "#,
            w_id,
            trigger.base.path,
            trigger.base.url,
            trigger.base.script_path,
            trigger.base.is_flow,
            trigger.base.enabled.unwrap_or(true),
            &filters as _,
            &initial_messages as _,
            trigger.config.url_runnable_args.map(SqlxJson) as _,
            authed.username,
            trigger.config.can_return_message,
            authed.email,
            trigger.error_handling.error_handler_path,
            trigger.error_handling.error_handler_args as _,
            trigger.error_handling.retry as _
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    async fn update_trigger<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
        authed: &ApiAuthed,
        w_id: &str,
        path: &str,
        trigger: &EditTrigger<Self::EditTriggerConfig>,
    ) -> Result<()> {
        let filters = trigger
            .config
            .filters
            .into_iter()
            .map(SqlxJson)
            .collect_vec();
        let initial_messages = trigger
            .config
            .initial_messages
            .unwrap_or_default()
            .into_iter()
            .map(SqlxJson)
            .collect_vec();

        // important to update server_id to NULL to stop current websocket listener
        sqlx::query!(
            "
        UPDATE 
            websocket_trigger
        SET
            url = $1,
            script_path = $2,
            path = $3,
            is_flow = $4,
            filters = $5,
            initial_messages = $6,
            url_runnable_args = $7,
            edited_by = $8,
            email = $9,
            can_return_message = $10,
            edited_at = now(),
            server_id = NULL,
            error = NULL,
            error_handler_path = $13,
            error_handler_args = $14,
            retry = $15
        WHERE
            workspace_id = $11 AND path = $12
    ",
            trigger.config.url,
            trigger.base.script_path,
            trigger.base.path,
            trigger.base.is_flow,
            filters.as_slice() as &[SqlxJson<Box<RawValue>>],
            initial_messages.as_slice() as &[SqlxJson<Box<RawValue>>],
            trigger.config.url_runnable_args.map(SqlxJson) as Option<SqlxJson<Box<RawValue>>>,
            &authed.username,
            &authed.email,
            trigger.config.can_return_message,
            w_id,
            path,
            trigger.error_handling.error_handler_path,
            trigger.error_handling.error_handler_args as _,
            trigger.error_handling.retry as _
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    async fn test_connection(
        &self,
        _db: &DB,
        _authed: &ApiAuthed,
        _user_db: &UserDB,
        _workspace_id: &str,
        config: &Self::TestConnectionConfig,
    ) -> Result<serde_json::Value> {
        // Test WebSocket connection
        let connect_result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            connect_async(&config.url),
        )
        .await;

        match connect_result {
            Ok(Ok((ws_stream, response))) => {
                // Connection successful, close it immediately
                drop(ws_stream);
                Ok(serde_json::json!({
                    "success": true,
                    "message": "Successfully connected to WebSocket",
                    "status": response.status().as_u16(),
                    "headers": response.headers().len()
                }))
            }
            Ok(Err(e)) => Err(Error::BadConfig(format!(
                "Failed to connect to WebSocket: {}",
                e
            ))),
            Err(_) => Err(Error::BadConfig(
                "WebSocket connection timeout after 10 seconds".to_string(),
            )),
        }
    }

    fn additional_routes(&self) -> Router {
        // WebSocket triggers don't need additional routes
        Router::new()
    }
}

// Helper function to create the handler instance
pub fn create_websocket_trigger_handler() -> WebsocketTriggerHandler {
    WebsocketTriggerHandler
}
