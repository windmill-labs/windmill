use std::borrow::Cow;

use async_trait::async_trait;
use itertools::Itertools;
use serde_json::value::RawValue;
use sqlx::{types::Json as SqlxJson, PgConnection};
use windmill_api_auth::ApiAuthed;
use windmill_common::DB;
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    jobs::ChosenBy,
    runnable_ref::RunnableRef,
    worker::to_raw_value,
};
use windmill_git_sync::DeployedObject;
use windmill_trigger::{filter::CompiledFilters, Trigger, TriggerCrud, TriggerData};

use super::{
    get_url_from_runnable_value, listener::InitialMessage, proxy::connect_async_with_proxy,
    validate_websocket_url_for_ssrf, TestWebsocketConfig, WebsocketConfig, WebsocketConfigRequest,
    WebsocketTrigger,
};

/// Refuses a `$flow:`/`$script:` URL resolver or `runnable_result` initial message whose path
/// does not parse: `config_runnables` leaves those out, so the save's run-scope check never sees
/// them.
fn check_secondary_runnables_parse(config: &WebsocketConfigRequest) -> Result<()> {
    for runnable in secondary_runnables(&config.url, config.initial_messages.as_deref()) {
        runnable?;
    }
    Ok(())
}

fn secondary_runnables(
    url: &str,
    initial_messages: Option<&[serde_json::Value]>,
) -> Vec<Result<RunnableRef>> {
    let mut runnables: Vec<_> = RunnableRef::from_ws_url(url).into_iter().collect();
    for msg in initial_messages.unwrap_or_default() {
        if let Ok(InitialMessage::RunnableResult { path, is_flow, .. }) =
            serde_json::from_value::<InitialMessage>(msg.clone())
        {
            runnables.push(RunnableRef::from_path(&path, is_flow));
        }
    }
    runnables
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
        "run_on_filter_error",
        "initial_messages",
        "url_runnable_args",
        "can_return_message",
        "can_return_error_result",
        "heartbeat",
    ];
    const IS_ALLOWED_ON_CLOUD: bool = false;

    fn config_runnables(config: &serde_json::Value) -> Vec<RunnableRef> {
        let url = config
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let messages = config.get("initial_messages").and_then(|v| v.as_array());
        secondary_runnables(url, messages.map(Vec::as_slice))
            .into_iter()
            .flatten()
            .collect()
    }

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

        CompiledFilters::validate(&config.filters)?;

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
        check_secondary_runnables_parse(&trigger.config)?;
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
                heartbeat,
                run_on_filter_error
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, now(), $15, $16, $17, $18, $19
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
            trigger.config.heartbeat.map(SqlxJson) as _,
            trigger.config.run_on_filter_error
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
        check_secondary_runnables_parse(&trigger.config)?;
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
            heartbeat = $18,
            run_on_filter_error = $19
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
            trigger.config.heartbeat.map(SqlxJson) as _,
            trigger.config.run_on_filter_error
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
            let Some(runnable) = RunnableRef::from_ws_url(url) else {
                return Err(Error::BadConfig(format!(
                    "Invalid WebSocket runnable path: {}",
                    url
                )));
            };
            Cow::Owned(
                get_url_from_runnable_value(
                    &runnable?,
                    &db,
                    authed.clone(),
                    ChosenBy::Caller,
                    config.url_runnable_args.as_ref().map(to_raw_value).as_ref(),
                    &workspace_id,
                )
                .await?,
            )
        } else {
            Cow::Borrowed(&url)
        };

        let validated = validate_websocket_url_for_ssrf(&connect_url).await?;

        connect_async_with_proxy(&*connect_url, validated.pinned_addrs())
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
