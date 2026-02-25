use std::borrow::Cow;

use axum::async_trait;
use itertools::Itertools;
use serde_json::value::RawValue;
use sqlx::{types::Json as SqlxJson, PgConnection};
use tokio_tungstenite::connect_async;
use windmill_api_auth::ApiAuthed;
use windmill_common::DB;
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    worker::to_raw_value,
};
use windmill_git_sync::DeployedObject;
use windmill_trigger::{Trigger, TriggerCrud, TriggerData};

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

    fn get_deployed_object(path: String, parent_path: Option<String>) -> DeployedObject {
        DeployedObject::WebsocketTrigger { path, parent_path }
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
        db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        w_id: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        let resolved_edited_by = trigger.base.resolve_edited_by(authed);
        let resolved_email = trigger.base.resolve_email(authed, db, w_id).await?;
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
                mode,
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
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, now(), $14, $15, $16
            )
            "#,
            w_id,
            trigger.base.path,
            trigger.config.url,
            trigger.base.script_path,
            trigger.base.is_flow,
            trigger.base.mode() as _,
            &filters as _,
            &initial_messages as _,
            trigger
                .config
                .url_runnable_args
                .map(|v| SqlxJson(serde_json::value::to_raw_value(&v).unwrap())) as _,
            &resolved_edited_by,
            trigger.config.can_return_message,
            trigger.config.can_return_error_result,
            resolved_email,
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
        db: &DB,
        tx: &mut PgConnection,
        authed: &ApiAuthed,
        w_id: &str,
        path: &str,
        trigger: TriggerData<Self::TriggerConfigRequest>,
    ) -> Result<()> {
        let resolved_edited_by = trigger.base.resolve_edited_by(authed);
        let resolved_email = trigger.base.resolve_email(authed, db, w_id).await?;
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
            filters = $5,
            initial_messages = $6,
            url_runnable_args = $7,
            edited_by = $8,
            email = $9,
            can_return_message = $10,
            can_return_error_result = $11,
            edited_at = now(),
            server_id = NULL,
            error = NULL,
            error_handler_path = $14,
            error_handler_args = $15,
            retry = $16
        WHERE
            workspace_id = $12 AND path = $13
    ",
            trigger.config.url,
            trigger.base.script_path,
            trigger.base.path,
            trigger.base.is_flow,
            filters.as_slice() as &[SqlxJson<Box<RawValue>>],
            initial_messages.as_slice() as &[SqlxJson<Box<RawValue>>],
            trigger
                .config
                .url_runnable_args
                .map(|v| SqlxJson(serde_json::value::to_raw_value(&v).unwrap()))
                as Option<SqlxJson<Box<RawValue>>>,
            &resolved_edited_by,
            resolved_email,
            trigger.config.can_return_message,
            trigger.config.can_return_error_result,
            w_id,
            path,
            trigger.error_handling.error_handler_path,
            trigger.error_handling.error_handler_args as _,
            trigger.error_handling.retry as _
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

        connect_async(&*connect_url).await.map_err(|err| {
            Error::BadConfig(format!(
                "Error connecting to WebSocket: {}",
                err.to_string()
            ))
        })?;

        Ok(())
    }
}
