use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::{types::Json as SqlxJson, FromRow};
use windmill_api_auth::ApiAuthed;
use windmill_common::{
    error::{Error, Result},
    jobs::JobTriggerKind,
    triggers::{TriggerKind, TriggerMetadata},
    worker::to_raw_value,
    DB,
};
use windmill_queue::PushArgsOwned;
use windmill_trigger::trigger_helpers::{
    trigger_runnable_and_wait_for_raw_result_with_error_ctx, TriggerJobArgs,
};

pub mod handler;
pub mod listener;
pub mod proxy;

#[derive(Copy, Clone)]
pub struct WebsocketTrigger;

impl TriggerJobArgs for WebsocketTrigger {
    type Payload = String;
    const TRIGGER_KIND: TriggerKind = TriggerKind::Websocket;
    fn v1_payload_fn(payload: &Self::Payload) -> HashMap<String, Box<RawValue>> {
        HashMap::from([("msg".to_string(), to_raw_value(&payload))])
    }
}

fn default_filter_logic() -> String {
    "and".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsocketHeartbeat {
    pub interval_secs: u64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_field: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WebsocketConfig {
    pub url: String,
    #[serde(default)]
    pub filters: Vec<SqlxJson<Box<RawValue>>>,
    #[serde(default = "default_filter_logic")]
    pub filter_logic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_messages: Option<Vec<SqlxJson<Box<RawValue>>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_runnable_args: Option<SqlxJson<Box<RawValue>>>,
    #[serde(default)]
    pub can_return_message: bool,
    #[serde(default)]
    pub can_return_error_result: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heartbeat: Option<SqlxJson<WebsocketHeartbeat>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsocketConfigRequest {
    url: String,
    filters: Vec<serde_json::Value>,
    #[serde(default = "default_filter_logic")]
    filter_logic: String,
    initial_messages: Option<Vec<serde_json::Value>>,
    url_runnable_args: Option<serde_json::Value>,
    can_return_message: bool,
    can_return_error_result: bool,
    pub heartbeat: Option<WebsocketHeartbeat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestWebsocketConfig {
    url: String,
    url_runnable_args: Option<serde_json::Value>,
}

pub fn value_to_args_hashmap(
    args: Option<&Box<RawValue>>,
) -> Result<HashMap<String, Box<RawValue>>> {
    let args = if let Some(args) = args {
        let args_map: Option<HashMap<String, serde_json::Value>> = serde_json::from_str(args.get())
            .map_err(|e| Error::BadRequest(format!("invalid json: {}", e)))?;

        args_map
            .unwrap_or_else(HashMap::new)
            .into_iter()
            .map(|(k, v)| {
                let raw_value = serde_json::value::to_raw_value(&v).map_err(|e| {
                    Error::BadRequest(format!("failed to convert to raw value: {}", e))
                })?;
                Ok((k, raw_value))
            })
            .collect::<Result<HashMap<String, Box<RawValue>>>>()
    } else {
        Ok(HashMap::new())
    }?;
    Ok(args)
}

/// Env var that opts a deployment out of SSRF validation for WebSocket trigger
/// URLs, permitting connections to private/internal addresses. Off by default.
pub const ALLOW_PRIVATE_WEBSOCKET_URLS_ENV: &str = "ALLOW_PRIVATE_WEBSOCKET_URLS";

/// Reject WebSocket URLs that target (or resolve to) a private/internal address,
/// blocking SSRF probes of the host's internal network and cloud metadata
/// endpoints.
///
/// `ws://`/`wss://` are mapped to `http`/`https` so the shared
/// `validate_url_for_ssrf` host + DNS-resolution checks apply. The
/// security-critical call sites are the outbound connects (the test handler and
/// every listener (re)connect): validating the *resolved* URL there means a
/// `$flow:`/`$script:` URL is checked on its returned value and re-checked on
/// each reconnect (DNS rebinding). `validate_config` also calls this at save
/// time to reject static URLs early.
///
/// Returns the resolved [`ValidatedTarget`]: the connect must pin these
/// addresses (see [`proxy::connect_async_with_proxy`]) so a rebinder cannot swap
/// in an internal IP between this check and the connect (TOCTOU). The returned
/// `addrs` carry the http(s) port, which equals the ws(s) port the connection
/// uses (ws→80, wss→443, or the explicit port preserved through the mapping).
pub async fn validate_websocket_url_for_ssrf(
    url: &str,
) -> Result<windmill_common::ssrf::ValidatedTarget> {
    // `ws`/`wss` aren't recognised by `validate_url_for_ssrf`'s scheme check, so
    // map them to the http(s) equivalent the same connection would tunnel over.
    // The prefixes are ASCII, so byte-slicing at their length stays on a char
    // boundary.
    let lower = url.to_ascii_lowercase();
    let http_url = if lower.starts_with("wss://") {
        format!("https://{}", &url["wss://".len()..])
    } else if lower.starts_with("ws://") {
        format!("http://{}", &url["ws://".len()..])
    } else {
        url.to_string()
    };

    if std::env::var(ALLOW_PRIVATE_WEBSOCKET_URLS_ENV)
        .ok()
        .is_some_and(|v| v == "true" || v == "1")
    {
        // Opted out of the SSRF guard: allow any host and pin nothing. Parse the
        // host for a uniform return; fall back to an empty target if unparseable
        // (the connect then resolves normally, matching the pre-guard behavior).
        let host = url::Url::parse(&http_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_default();
        return Ok(windmill_common::ssrf::ValidatedTarget { host, addrs: Vec::new() });
    }

    windmill_common::ssrf::validate_url_for_ssrf(&http_url)
        .await
        .map_err(|e| match e {
            // The env-var hint is only actionable for a well-formed URL blocked
            // for targeting a private address; a malformed URL or bad scheme
            // surfaces its real error so the user fixes the URL (see #9171).
            e @ windmill_common::ssrf::SsrfValidationError::Private { .. } => {
                Error::BadRequest(format!(
                    "{e}. If you need to connect to private/internal WebSocket endpoints, \
                     set the {ALLOW_PRIVATE_WEBSOCKET_URLS_ENV}=true environment variable"
                ))
            }
            e => Error::from(e),
        })
}

pub async fn get_url_from_runnable_value(
    path: &str,
    is_flow: bool,
    db: &DB,
    authed: ApiAuthed,
    args: Option<&Box<RawValue>>,
    workspace_id: &str,
) -> Result<String> {
    tracing::info!(
        "Running {} {} to get WebSocket URL",
        if is_flow { "flow" } else { "script" },
        path
    );

    let args = value_to_args_hashmap(args)?;

    let result = trigger_runnable_and_wait_for_raw_result_with_error_ctx(
        db,
        None,
        authed,
        workspace_id,
        path,
        is_flow,
        PushArgsOwned { args, extra: None },
        None,
        None,
        None,
        "".to_string(), // doesn't matter as no retry/error handler
        TriggerMetadata::new(Some(path.to_owned()), JobTriggerKind::Websocket),
    )
    .await?;

    serde_json::from_str::<String>(result.get()).map_err(|_| {
        Error::BadConfig(format!(
            "{} {} did not return a string",
            if is_flow { "Flow" } else { "Script" },
            path,
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ssrf_blocks_private_and_metadata_ws_urls() {
        // ws:// → http:// mapping must still reach the IP-literal block.
        let err = validate_websocket_url_for_ssrf("ws://127.0.0.1:6379/")
            .await
            .unwrap_err();
        assert!(matches!(err, Error::BadRequest(_)));
        // Private errors carry the opt-out hint so operators can allow internal
        // targets deliberately.
        assert!(err.to_string().contains(ALLOW_PRIVATE_WEBSOCKET_URLS_ENV));

        // wss:// → https:// mapping blocks the cloud metadata endpoint.
        assert!(
            validate_websocket_url_for_ssrf("wss://169.254.169.254/latest/meta-data")
                .await
                .is_err()
        );
        assert!(validate_websocket_url_for_ssrf("ws://10.0.0.5:6379/")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn ssrf_rejects_non_ws_scheme_without_private_hint() {
        // A non-ws scheme isn't mapped and fails the scheme check; it must not
        // get the "set ALLOW_PRIVATE_WEBSOCKET_URLS" hint (issue #9171).
        let err = validate_websocket_url_for_ssrf("file:///etc/passwd")
            .await
            .unwrap_err();
        assert!(!err.to_string().contains(ALLOW_PRIVATE_WEBSOCKET_URLS_ENV));
    }
}
