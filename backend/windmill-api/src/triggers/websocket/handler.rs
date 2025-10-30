use std::borrow::Cow;

use crate::{
    db::{ApiAuthed, DB},
    triggers::{Trigger, TriggerCrud, TriggerData},
};
use axum::async_trait;
use itertools::Itertools;
use serde_json::value::RawValue;
use sqlx::{types::Json as SqlxJson, PgConnection};
use tokio_tungstenite::connect_async;
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    worker::to_raw_value,
};
use windmill_git_sync::DeployedObject;

use super::{
    get_url_from_runnable_value, TestWebsocketConfig, WebsocketConfig, WebsocketConfigRequest,
    WebsocketTrigger,
};

#[async_trait]
impl TriggerCrud for WebsocketTrigger {
    type TriggerConfig = WebsocketConfig;
    type Trigger = Trigger<Self::TriggerConfig>;
    type TriggerConfigRequest = WebsocketConfigRequest;
    type TestConnectionConfig = TestWebsocketConfig;

    const TABLE_NAME: &'static str = "websocket_trigger";
    const TRIGGER_TYPE: &'static str = "websocket";
    const SUPPORTS_ENABLED: bool = true;
    const SUPPORTS_SERVER_STATE: bool = true;
    const SUPPORTS_TEST_CONNECTION: bool = true;
    const ROUTE_PREFIX: &'static str = "/websocket_triggers";
    const DEPLOYMENT_NAME: &'static str = "WebSocket trigger";
    const ADDITIONAL_SELECT_FIELDS: &[&'static str] = &[
        "url",
        "filters",
        "initial_messages",
        "url_runnable_args",
        "can_return_message",
        "can_return_error_result",
    ];
    const IS_ALLOWED_ON_CLOUD: bool = false;

    fn get_deployed_object(path: String) -> DeployedObject {
        DeployedObject::WebsocketTrigger { path }
    }

    async fn validate_config(
        &self,
        _db: &DB,
        config: &Self::TriggerConfigRequest,
        _workspace_id: &str,
    ) -> Result<()> {
        if config.url.trim().is_empty() {
            return Err(Error::BadRequest(
                "WebSocket URL cannot be empty".to_string(),
            ));
        }

        if let Some(args) = &config.url_runnable_args {
            if !args.is_object() {
                return Err(Error::BadRequest(
                    "url_runnable_args must be an object".to_string(),
                ));
            }
        }

        Ok(())
    }

    async fn create_trigger(
        &self,
        _db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        w_id: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        let filters = trigger
            .config
            .filters
            .into_iter()
            .map(|v| SqlxJson(serde_json::value::to_raw_value(&v).unwrap()))
            .collect_vec();
        let initial_messages = trigger
            .config
            .initial_messages
            .unwrap_or_default()
            .into_iter()
            .map(|v| SqlxJson(serde_json::value::to_raw_value(&v).unwrap()))
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
                action_to_take,
                filters,
                initial_messages,
                url_runnable_args,
                edited_by,
                can_return_message,
                can_return_error_result,
                email,
                edited_at,
                error_handler_path,
                error_handler_args,
                retry
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, now(), $15, $16, $17
            )
            "#,
            w_id,
            trigger.base.path,
            trigger.config.url,
            trigger.base.script_path,
            trigger.base.is_flow,
            trigger.base.enabled.unwrap_or(true),
            trigger.base.action_to_take as _,
            &filters as _,
            &initial_messages as _,
            trigger
                .config
                .url_runnable_args
                .map(|v| SqlxJson(serde_json::value::to_raw_value(&v).unwrap())) as _,
            authed.username,
            trigger.config.can_return_message,
            trigger.config.can_return_error_result,
            authed.email,
            trigger.error_handling.error_handler_path,
            trigger.error_handling.error_handler_args as _,
            trigger.error_handling.retry as _
        )
        .execute(&mut *tx)
        .await?;
        Ok(())
    }

    async fn update_trigger(
        &self,
        _db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        w_id: &str,
        path: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        let filters = trigger
            .config
            .filters
            .into_iter()
            .map(|v| SqlxJson(serde_json::value::to_raw_value(&v).unwrap()))
            .collect_vec();
        let initial_messages = trigger
            .config
            .initial_messages
            .unwrap_or_default()
            .into_iter()
            .map(|v| SqlxJson(serde_json::value::to_raw_value(&v).unwrap()))
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
            action_to_take = $5,
            filters = $6,
            initial_messages = $7,
            url_runnable_args = $8,
            edited_by = $9,
            email = $10,
            can_return_message = $11,
            can_return_error_result = $12,
            edited_at = now(),
            server_id = NULL,
            error = NULL,
            error_handler_path = $13,
            error_handler_args = $14,
            retry = $15
        WHERE
            workspace_id = $16 AND path = $17
    ",
            trigger.config.url,
            trigger.base.script_path,
            trigger.base.path,
            trigger.base.is_flow,
            trigger.base.action_to_take as _,
            filters.as_slice() as &[SqlxJson<Box<RawValue>>],
            initial_messages.as_slice() as &[SqlxJson<Box<RawValue>>],
            trigger
                .config
                .url_runnable_args
                .map(|v| SqlxJson(serde_json::value::to_raw_value(&v).unwrap()))
                as Option<SqlxJson<Box<RawValue>>>,
            &authed.username,
            &authed.email,
            trigger.config.can_return_message,
            trigger.config.can_return_error_result,
            trigger.error_handling.error_handler_path,
            trigger.error_handling.error_handler_args as _,
            trigger.error_handling.retry as _,
            w_id,
            path
        )
        .execute(&mut *tx)
        .await?;

        Ok(())
    }

    async fn test_connection(
        &self,
        db: &DB,
        authed: &ApiAuthed,
        _user_db: &UserDB,
        workspace_id: &str,
        config: Self::TestConnectionConfig,
    ) -> Result<()> {
        let url = &config.url;

        let connect_url: Cow<str> = if url.starts_with("$") {
            if url.starts_with("$flow:") || url.starts_with("$script:") {
                let path = url.splitn(2, ':').nth(1).unwrap();
                Cow::Owned(
                    get_url_from_runnable_value(
                        path,
                        url.starts_with("$flow:"),
                        &db,
                        authed.clone(),
                        config.url_runnable_args.as_ref().map(to_raw_value).as_ref(),
                        &workspace_id,
                    )
                    .await?,
                )
            } else {
                return Err(Error::BadConfig(format!(
                    "Invalid WebSocket runnable path: {}",
                    url
                )));
            }
        } else {
            Cow::Borrowed(&url)
        };

        connect_async(connect_url.as_ref()).await.map_err(|err| {
            Error::BadConfig(format!(
                "Error connecting to WebSocket: {}",
                err.to_string()
            ))
        })?;

        Ok(())
    }
}
