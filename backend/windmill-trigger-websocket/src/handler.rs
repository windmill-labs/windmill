use std::borrow::Cow;

use async_trait::async_trait;
use itertools::Itertools;
use serde_json::value::RawValue;
use sqlx::{types::Json as SqlxJson, PgConnection};
use windmill_api_auth::{check_scopes, ApiAuthed};
use windmill_common::DB;
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    worker::to_raw_value,
};
use windmill_git_sync::DeployedObject;
use windmill_trigger::{Trigger, TriggerCrud, TriggerData};

use super::{
    get_url_from_runnable_value, listener::InitialMessage, proxy::connect_async_with_proxy,
    validate_websocket_url_for_ssrf, TestWebsocketConfig, WebsocketConfig, WebsocketConfigRequest,
    WebsocketTrigger,
};

/// A websocket_triggers:write token can configure secondary runnables that the
/// listener later executes under the trigger owner's identity: a `$flow:`/
/// `$script:` URL resolver and `initial_messages` of kind `runnable_result`.
/// That execution happens in a background task where the reconstructed authed is
/// scopeless (so its check_scopes is a no-op), so enforce run scope here, at
/// create/update time, against the API caller's token.
fn check_secondary_runnable_scopes(
    authed: &ApiAuthed,
    config: &WebsocketConfigRequest,
) -> Result<()> {
    if let Some(rest) = config.url.strip_prefix("$flow:") {
        check_scopes(authed, || format!("jobs:run:flows:{}", rest))?;
    } else if let Some(rest) = config.url.strip_prefix("$script:") {
        check_scopes(authed, || format!("jobs:run:scripts:{}", rest))?;
    }
    if let Some(messages) = config.initial_messages.as_ref() {
        for msg in messages {
            if let Ok(InitialMessage::RunnableResult { path, is_flow, .. }) =
                serde_json::from_value::<InitialMessage>(msg.clone())
            {
                let kind = if is_flow { "flows" } else { "scripts" };
                check_scopes(authed, || format!("jobs:run:{}:{}", kind, path))?;
            }
        }
    }
    Ok(())
}

#[async_trait]
impl TriggerCrud for WebsocketTrigger {
    type TriggerConfig = WebsocketConfig;
    type Trigger = Trigger<Self::TriggerConfig>;
    type TriggerConfigRequest = WebsocketConfigRequest;
    type TestConnectionConfig = TestWebsocketConfig;

    const TABLE_NAME: &'static str = "websocket_trigger";
    const TRIGGER_TYPE: &'static str = "websocket";
    const DRAFT_KIND: windmill_common::user_drafts::UserDraftItemKind =
        windmill_common::user_drafts::UserDraftItemKind::TriggerWebsocket;
    const SUPPORTS_SERVER_STATE: bool = true;
    const SUPPORTS_TEST_CONNECTION: bool = true;
    const ROUTE_PREFIX: &'static str = "/websocket_triggers";
    const DEPLOYMENT_NAME: &'static str = "WebSocket trigger";
    const ADDITIONAL_SELECT_FIELDS: &[&'static str] = &[
        "url",
        "filters",
        "filter_logic",
        "initial_messages",
        "url_runnable_args",
        "can_return_message",
        "can_return_error_result",
        "heartbeat",
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

        // Reject SSRF targets at save time for static URLs. A `$flow:`/`$script:`
        // URL is only known at runtime, so it is validated at connect time
        // instead (in the listener and test handler).
        if !config.url.starts_with('$') {
            validate_websocket_url_for_ssrf(&config.url).await?;
        }

        if let Some(args) = &config.url_runnable_args {
            if !args.is_object() {
                return Err(Error::BadRequest(
                    "url_runnable_args must be an object".to_string(),
                ));
            }
        }

        if let Some(ref hb) = config.heartbeat {
            if hb.interval_secs < 1 {
                return Err(Error::BadRequest(
                    "heartbeat interval_secs must be at least 1".to_string(),
                ));
            }
            if hb.message.is_empty() {
                return Err(Error::BadRequest(
                    "heartbeat message cannot be empty".to_string(),
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
        check_secondary_runnable_scopes(authed, &trigger.config)?;
        let resolved_edited_by = trigger.base.resolve_edited_by(authed);
        let resolved_permissioned_as = trigger.base.resolve_permissioned_as(authed);
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
                filter_logic,
                initial_messages,
                url_runnable_args,
                edited_by,
                can_return_message,
                can_return_error_result,
                permissioned_as,
                edited_at,
                error_handler_path,
                error_handler_args,
                retry,
                heartbeat
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, now(), $15, $16, $17, $18
            )
            "#,
            w_id,
            trigger.base.path,
            trigger.config.url,
            trigger.base.script_path,
            trigger.base.is_flow,
            trigger.base.mode() as _,
            &filters as _,
            trigger.config.filter_logic,
            &initial_messages as _,
            trigger
                .config
                .url_runnable_args
                .map(|v| SqlxJson(serde_json::value::to_raw_value(&v).unwrap())) as _,
            &resolved_edited_by,
            trigger.config.can_return_message,
            trigger.config.can_return_error_result,
            resolved_permissioned_as,
            trigger.error_handling.error_handler_path,
            trigger.error_handling.error_handler_args as _,
            trigger.error_handling.retry as _,
            trigger.config.heartbeat.map(SqlxJson) as _
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
        check_secondary_runnable_scopes(authed, &trigger.config)?;
        let resolved_edited_by = trigger.base.resolve_edited_by(authed);
        let resolved_permissioned_as = trigger.base.resolve_permissioned_as(authed);
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
            filter_logic = $6,
            initial_messages = $7,
            url_runnable_args = $8,
            edited_by = $9,
            permissioned_as = $10,
            can_return_message = $11,
            can_return_error_result = $12,
            edited_at = now(),
            server_id = NULL,
            error = NULL,
            error_handler_path = $15,
            error_handler_args = $16,
            retry = $17,
            heartbeat = $18
        WHERE
            workspace_id = $13 AND path = $14
    ",
            trigger.config.url,
            trigger.base.script_path,
            trigger.base.path,
            trigger.base.is_flow,
            filters.as_slice() as &[SqlxJson<Box<RawValue>>],
            trigger.config.filter_logic,
            initial_messages.as_slice() as &[SqlxJson<Box<RawValue>>],
            trigger
                .config
                .url_runnable_args
                .map(|v| SqlxJson(serde_json::value::to_raw_value(&v).unwrap()))
                as Option<SqlxJson<Box<RawValue>>>,
            &resolved_edited_by,
            resolved_permissioned_as,
            trigger.config.can_return_message,
            trigger.config.can_return_error_result,
            w_id,
            path,
            trigger.error_handling.error_handler_path,
            trigger.error_handling.error_handler_args as _,
            trigger.error_handling.retry as _,
            trigger.config.heartbeat.map(SqlxJson) as _
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

        validate_websocket_url_for_ssrf(&connect_url).await?;

        connect_async_with_proxy(&*connect_url)
            .await
            .map_err(|err| {
                Error::BadConfig(format!(
                    "Error connecting to WebSocket: {}",
                    err.to_string()
                ))
            })?;

        Ok(())
    }
}
