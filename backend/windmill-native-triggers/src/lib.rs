//! Native Triggers Module
//!
//! This module provides integration with external services (like Nextcloud) that can
//! trigger Windmill scripts/flows via webhooks.
//!
//! ## Adding a New Native Trigger Service
//!
//! When adding a new service (e.g., "NewService"), you need to update the following locations:
//!
//! ### 1. This file (lib.rs):
//! - Add `pub mod newservice;` under the `#[cfg(feature = "native_trigger")]` block
//! - Add `NewService` variant to `ServiceName` enum
//! - Update `ServiceName::as_str()` - add match arm returning `"newservice"`
//! - Update `TryFrom<String> for ServiceName` - add match arm for `"newservice"`
//! - Update `ServiceName::as_trigger_kind()` - add match arm (requires TriggerKind::NewService in windmill_common)
//! - Update `ServiceName::as_job_trigger_kind()` - add match arm (requires JobTriggerKind::NewService in windmill_common)
//! - Update `ServiceName::fmt()` (Display impl) - add match arm
//!
//! ### 2. sync.rs:
//! - Add `sync_service!()` macro call in `sync_all_triggers()`
//!
//! ### 3. handler.rs:
//! - Add `.nest("/newservice", service_routes(NewServiceHandler))` in `generate_native_trigger_routers()`
//!
//! ### 4. Database migration:
//! - Add `'newservice'` to the `native_trigger_service` enum type
//!
//! ### 5. windmill_common (if needed):
//! - Add `NewService` variant to `TriggerKind` enum
//! - Add `'newservice'` to `job_trigger_kind` enum type in migration
//!
//! The generic code (trait definitions, route handlers, database operations) does NOT
//! need modification when adding new services.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use http::StatusCode;
use itertools::Itertools;
use reqwest::Method;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use serde_json::value::RawValue;
use sqlx::{FromRow, PgConnection, Postgres};
use std::{collections::HashMap, fmt::Debug};
use strum::{EnumIter, IntoEnumIterator};
use tokio::task;
use windmill_common::{
    error::{to_anyhow, Error, Result},
    triggers::TriggerKind,
    utils::HTTP_CLIENT,
    variables::{build_crypt, decrypt, encrypt},
    DB,
};
use windmill_queue::PushArgsOwned;

#[cfg(feature = "native_trigger")]
use windmill_oauth::{ErrorField, ExecuteError, OClient, RefreshToken, Url, OAUTH_HTTP_CLIENT};

use windmill_api_auth::ApiAuthed;
pub mod handler;
pub(crate) mod lock;
pub mod sync;
pub mod workspace_integrations;

#[cfg(feature = "native_trigger")]
pub mod rename;

// Service modules - add new services here:
#[cfg(feature = "native_trigger")]
pub mod github;
#[cfg(feature = "native_trigger")]
pub mod google;
#[cfg(feature = "native_trigger")]
pub mod nextcloud;

/// Enum of all supported native trigger services.
/// When adding a new service, add a variant here (e.g., `NewService`).
#[derive(EnumIter, sqlx::Type, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[sqlx(type_name = "native_trigger_service", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ServiceName {
    Nextcloud,
    Google,
    Github,
}

impl TryFrom<String> for ServiceName {
    type Error = Error;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        let service = match value.as_str() {
            "nextcloud" => ServiceName::Nextcloud,
            "google" => ServiceName::Google,
            "github" => ServiceName::Github,
            _ => {
                return Err(anyhow::anyhow!(
                    "Unknown service, currently supported services are: [{}]",
                    ServiceName::iter().join(",")
                )
                .into())
            }
        };

        Ok(service)
    }
}

impl ServiceName {
    /// Returns the lowercase string identifier for this service.
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "nextcloud",
            ServiceName::Google => "google",
            ServiceName::Github => "github",
        }
    }

    /// Returns the corresponding TriggerKind for this service.
    pub fn as_trigger_kind(&self) -> TriggerKind {
        match self {
            ServiceName::Nextcloud => TriggerKind::Nextcloud,
            ServiceName::Google => TriggerKind::Google,
            ServiceName::Github => TriggerKind::Github,
        }
    }

    /// Returns the corresponding JobTriggerKind for this service.
    pub fn as_job_trigger_kind(&self) -> windmill_common::jobs::JobTriggerKind {
        match self {
            ServiceName::Nextcloud => windmill_common::jobs::JobTriggerKind::Nextcloud,
            ServiceName::Google => windmill_common::jobs::JobTriggerKind::Google,
            ServiceName::Github => windmill_common::jobs::JobTriggerKind::Github,
        }
    }

    /// Returns the OAuth token endpoint path for this service.
    pub fn token_endpoint(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "/apps/oauth2/api/v1/token",
            ServiceName::Google => "https://oauth2.googleapis.com/token",
            ServiceName::Github => "https://github.com/login/oauth/access_token",
        }
    }

    /// Returns the OAuth authorization endpoint path for this service.
    pub fn auth_endpoint(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "/apps/oauth2/authorize",
            ServiceName::Google => "https://accounts.google.com/o/oauth2/v2/auth",
            ServiceName::Github => "https://github.com/login/oauth/authorize",
        }
    }

    /// Returns the OAuth scopes for this service's authorization flow.
    pub fn oauth_scopes(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "read write",
            ServiceName::Google => "https://www.googleapis.com/auth/drive.readonly https://www.googleapis.com/auth/calendar.readonly https://www.googleapis.com/auth/calendar.events",
            // `repo` is required to list private repositories via /user/repos and
            // /search/repositories. It also covers webhook management, so it
            // supersedes `admin:repo_hook`.
            ServiceName::Github => "repo read:user",
        }
    }

    /// Returns the resource type used for storing OAuth tokens.
    pub fn resource_type(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "nextcloud",
            ServiceName::Google => "gworkspace",
            ServiceName::Github => "github",
        }
    }

    /// Returns extra OAuth authorization parameters required by this service.
    pub fn extra_auth_params(&self) -> &[(&'static str, &'static str)] {
        match self {
            ServiceName::Google => &[("access_type", "offline"), ("prompt", "consent")],
            ServiceName::Nextcloud => &[],
            ServiceName::Github => &[],
        }
    }

    /// Returns the integration service name for workspace_integrations lookup.
    pub fn integration_service(&self) -> ServiceName {
        *self
    }

    /// How long webhook tokens for this service should remain valid. `None` = no expiry.
    /// Google channels turn over on a tight schedule (24h Drive, 7d Calendar) — a finite
    /// TTL lets `delete_expired_items` (`monitor.rs`) sweep orphaned tokens automatically.
    /// Persistent-webhook services (Nextcloud, GitHub) return `None`.
    pub fn webhook_token_expiration(&self) -> Option<chrono::Duration> {
        match self {
            ServiceName::Google => Some(chrono::Duration::days(14)),
            ServiceName::Nextcloud | ServiceName::Github => None,
        }
    }
}

impl std::fmt::Display for ServiceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Resolves an endpoint URL. If the endpoint is already an absolute URL (starts with http),
/// returns it as-is. Otherwise, prepends the base_url.
pub fn resolve_endpoint(base_url: &str, endpoint: &str) -> String {
    if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        endpoint.to_string()
    } else {
        format!("{}{}", base_url, endpoint)
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NativeTrigger {
    pub external_id: String,
    pub workspace_id: String,
    pub service_name: ServiceName,
    pub script_path: String,
    pub is_flow: bool,
    pub webhook_token_hash: String,
    pub service_config: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeTriggerConfig {
    pub script_path: String,
    pub is_flow: bool,
    pub webhook_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NativeTriggerData<C> {
    pub script_path: String,
    pub is_flow: bool,
    pub service_config: C,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WorkspaceIntegration {
    pub workspace_id: String,
    pub service_name: ServiceName,
    pub oauth_data: serde_json::Value,
    pub resource_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
}

#[async_trait]
pub trait External: Send + Sync + 'static {
    type ServiceConfig: Debug + DeserializeOwned + Serialize + Send + Sync;
    type TriggerData: Debug + Serialize + Send + Sync;
    type OAuthData: DeserializeOwned + Serialize + Clone + Send + Sync;
    type CreateResponse: DeserializeOwned + Send + Sync;

    const SUPPORT_WEBHOOK: bool;
    const SERVICE_NAME: ServiceName;
    const DISPLAY_NAME: &'static str;
    const TOKEN_ENDPOINT: &'static str;
    const REFRESH_ENDPOINT: &'static str;
    const AUTH_ENDPOINT: &'static str;

    async fn create(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        webhook_token: &str,
        data: &NativeTriggerData<Self::ServiceConfig>,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<Self::CreateResponse>;

    /// Update a trigger on the external service and return the resolved service_config to store.
    /// Each service is responsible for resolving the final config:
    /// - Services that re-create the resource (e.g. Google) build config from request data + response metadata.
    /// - Services that modify in-place (e.g. Nextcloud) fetch back the updated state and extract config.
    async fn update(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        webhook_token: &str,
        data: &NativeTriggerData<Self::ServiceConfig>,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<serde_json::Value>;

    /// Fetch the trigger's state from the external service.
    /// Returns `Ok(None)` (default) when the service has no "get" API (e.g. Google).
    /// Services that can fetch state (e.g. Nextcloud) override to return `Ok(Some(data))`.
    async fn get(
        &self,
        _w_id: &str,
        _oauth_data: &Self::OAuthData,
        _external_id: &str,
        _db: &DB,
        _tx: &mut PgConnection,
    ) -> Result<Option<Self::TriggerData>> {
        Ok(None)
    }

    async fn delete(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<()>;

    /// Periodic background maintenance for triggers in a workspace.
    /// Each service implements its own logic:
    /// - Nextcloud: lists external triggers and reconciles with DB state
    /// - Google: renews expiring watch channels
    async fn maintain_triggers(
        &self,
        db: &DB,
        workspace_id: &str,
        triggers: &[NativeTrigger],
        oauth_data: &Self::OAuthData,
        synced: &mut Vec<crate::sync::TriggerSyncInfo>,
        errors: &mut Vec<crate::sync::SyncError>,
    );

    async fn prepare_webhook(
        &self,
        _db: &DB,
        _w_id: &str,
        _header: HashMap<String, String>,
        _body: String,
        _script_path: &str,
        _is_flow: bool,
    ) -> Result<PushArgsOwned> {
        Ok(PushArgsOwned { extra: None, args: HashMap::new() })
    }

    fn external_id_and_metadata_from_response(
        &self,
        resp: &Self::CreateResponse,
    ) -> (String, Option<serde_json::Value>);

    /// Build the service_config directly from the create response and input data,
    /// skipping the update+get cycle after creation.
    /// Return `None` (default) to use the update+get pattern (e.g. Nextcloud needs to
    /// correct the webhook URL with the external_id assigned by the remote service).
    /// Return `Some(config)` to skip update+get entirely (e.g. Google already includes
    /// the channel_id in the webhook URL from the start).
    fn service_config_from_create_response(
        &self,
        _data: &NativeTriggerData<Self::ServiceConfig>,
        _resp: &Self::CreateResponse,
    ) -> Option<serde_json::Value> {
        None
    }

    fn additional_routes(&self) -> axum::Router {
        axum::Router::new()
    }

    /// Pull the human-readable message out of an error body, when the service wraps it in an
    /// envelope. Returning `None` (default) shows the body as-is.
    fn describe_error_body(&self, _body: &str) -> Option<String> {
        None
    }

    /// What the user has to do about a rejection, appended to the service's own message.
    fn error_hint(&self, _status: StatusCode) -> Option<&'static str> {
        None
    }

    /// Whether the service is telling us it cannot serve the request right now, when the
    /// status alone would read as a refusal. GitHub and Google both answer 403 to throttling.
    fn is_transient_response(&self, _status: StatusCode, _body: &str) -> bool {
        false
    }

    async fn http_client_request<T: DeserializeOwned + Send, B: Serialize + Send + Sync>(
        &self,
        url: &str,
        method: Method,
        workspace_id: &str,
        db: &DB,
        headers: Option<HashMap<String, String>>,
        body: Option<&B>,
    ) -> Result<T> {
        let oauth_config: OAuthConfig =
            decrypt_oauth_data(db, workspace_id, Self::SERVICE_NAME).await?;

        let result = make_http_request(
            url,
            method.clone(),
            headers.clone(),
            body.as_ref(),
            &oauth_config.access_token,
        )
        .await;

        match result {
            Ok(response) => Ok(response),
            // Only an expired or revoked token is worth a refresh. A 403 means the account
            // behind the token is authenticated and still not allowed, which minting a new
            // token for that same account cannot change; retrying would only burn a refresh
            // rotation per request and bury the service's own explanation.
            Err(err) if err.status() == Some(StatusCode::UNAUTHORIZED) => {
                tracing::info!(
                    "HTTP 401 from {}, attempting token refresh",
                    Self::DISPLAY_NAME
                );

                let refreshed_oauth_config = refresh_oauth_tokens(
                    &oauth_config,
                    Self::REFRESH_ENDPOINT,
                    Self::AUTH_ENDPOINT,
                )
                .await
                .map_err(|f| {
                    self.external_error(
                        f.rejected.then_some(StatusCode::UNAUTHORIZED),
                        format!(
                            "the stored credentials could not be refreshed: {}",
                            f.message
                        ),
                    )
                })?;

                task::spawn({
                    let db_clone = db.clone();
                    let workspace_id_clone = workspace_id.to_string();
                    let service_name = Self::SERVICE_NAME;
                    let new_access_token = refreshed_oauth_config.access_token.clone();
                    let new_refresh_token = refreshed_oauth_config.refresh_token.clone();
                    async move {
                        update_oauth_token_resource(
                            &db_clone,
                            &workspace_id_clone,
                            service_name,
                            &new_access_token,
                            new_refresh_token.as_deref(),
                        )
                        .await;
                    }
                });

                make_http_request(
                    url,
                    method,
                    headers,
                    body.as_ref(),
                    &refreshed_oauth_config.access_token,
                )
                .await
                .map_err(|e| self.external_api_error(e))
            }
            Err(e) => Err(self.external_api_error(e)),
        }
    }

    /// Wrap a failed provider call so the status stays inspectable by internal callers (a 404
    /// means the trigger is gone, not that the call broke) and the message stays readable by
    /// the time it reaches a user.
    fn external_api_error(&self, e: HttpRequestError) -> Error {
        let (detail, transient) = match &e {
            HttpRequestError::ApiError { status, body } => (
                self.describe_error_body(body)
                    .unwrap_or_else(|| body.to_string()),
                self.is_transient_response(*status, body),
            ),
            // A reqwest error names the request it was making, never why it failed: the reason
            // (refused, DNS, TLS) lives one link down the source chain.
            other => (error_source_chain(other), false),
        };
        self.external_error_inner(e.status(), detail, transient)
    }

    fn external_error(&self, status: Option<StatusCode>, detail: String) -> Error {
        self.external_error_inner(status, detail, false)
    }

    fn external_error_inner(
        &self,
        status: Option<StatusCode>,
        detail: String,
        transient: bool,
    ) -> Error {
        to_anyhow(ExternalApiError {
            service: Self::DISPLAY_NAME,
            status,
            detail: truncate_detail(&detail),
            // Guidance about permissions on a throttled request sends the reader after a
            // problem they do not have.
            hint: if transient {
                None
            } else {
                status.and_then(|s| self.error_hint(s))
            },
            transient,
        })
        .into()
    }
}

/// A native trigger provider refused or could not serve a request.
#[derive(Debug)]
pub struct ExternalApiError {
    pub service: &'static str,
    pub status: Option<StatusCode>,
    pub detail: String,
    pub hint: Option<&'static str>,
    /// The service could not serve the request now, whatever the status suggests: providers
    /// overload 403 for throttling, and a refusal and an outage want opposite reactions.
    pub transient: bool,
}

impl ExternalApiError {
    fn is_transient(&self) -> bool {
        self.transient || self.status.is_some_and(is_transient_status)
    }
}

impl std::fmt::Display for ExternalApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.status {
            Some(status) if self.is_transient() => write!(
                f,
                "{} failed to serve the request ({}): {}",
                self.service, status, self.detail
            )?,
            Some(status) => write!(
                f,
                "{} rejected the request ({}): {}",
                self.service, status, self.detail
            )?,
            // No status covers both never reaching the service and not being able to read what
            // it answered, so the wording may not commit to either.
            None => write!(f, "Request to {} failed: {}", self.service, self.detail)?,
        }
        if let Some(hint) = self.hint {
            write!(f, " — {}", hint)?;
        }
        Ok(())
    }
}

impl std::error::Error for ExternalApiError {}

fn error_source_chain(e: &dyn std::error::Error) -> String {
    let mut msg = e.to_string();
    let mut source = e.source();
    while let Some(cause) = source {
        let cause = cause.to_string();
        // reqwest repeats "error sending request for url (…)" at two levels of the chain.
        if !msg.contains(&cause) {
            msg.push_str(&format!(": {cause}"));
        }
        source = source.and_then(|c| c.source());
    }
    msg
}

/// Provider bodies are unbounded and end up in toasts and `native_trigger.error`.
fn truncate_detail(detail: &str) -> String {
    const MAX: usize = 400;
    let detail = detail.trim();
    match detail.char_indices().nth(MAX) {
        Some((idx, _)) => format!("{}…", &detail[..idx]),
        None => detail.to_string(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub base_url: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub client_id: String,
    pub client_secret: String,
}

pub async fn make_http_request<T: DeserializeOwned + Send, B: Serialize>(
    url: &str,
    method: Method,
    headers: Option<HashMap<String, String>>,
    body: Option<&B>,
    access_token: &str,
) -> std::result::Result<T, HttpRequestError> {
    let client = &*HTTP_CLIENT;
    let mut request = client.request(method, url);

    request = request
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", access_token));

    if body.is_some() {
        request = request.header("Content-Type", "application/json");
    }

    if let Some(custom_headers) = headers {
        for (key, value) in custom_headers {
            request = request.header(key, value);
        }
    }

    if let Some(body_content) = body {
        request = request.json(body_content);
    }

    let response = request.send().await?;
    let status = response.status();
    let bytes = response.bytes().await?;

    if !status.is_success() {
        let body = String::from_utf8_lossy(&bytes);
        return Err(HttpRequestError::ApiError { status, body: body.into_owned() });
    }

    // Handle empty responses (e.g. 204 No Content from Google channels/stop)
    if bytes.is_empty() {
        serde_json::from_str("null").map_err(HttpRequestError::Json)
    } else {
        serde_json::from_slice(&bytes).map_err(HttpRequestError::Json)
    }
}

#[derive(Debug)]
pub enum HttpRequestError {
    Reqwest(reqwest::Error),
    Json(serde_json::Error),
    ApiError { status: StatusCode, body: String },
}

impl std::fmt::Display for HttpRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpRequestError::Reqwest(e) => write!(f, "{}", e),
            HttpRequestError::Json(e) => write!(f, "JSON decode error: {}", e),
            HttpRequestError::ApiError { status, body } => {
                write!(f, "HTTP {} error: {}", status.as_u16(), body)
            }
        }
    }
}

impl std::error::Error for HttpRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HttpRequestError::Reqwest(e) => Some(e),
            HttpRequestError::Json(e) => Some(e),
            HttpRequestError::ApiError { .. } => None,
        }
    }
}

impl From<reqwest::Error> for HttpRequestError {
    fn from(e: reqwest::Error) -> Self {
        HttpRequestError::Reqwest(e)
    }
}

impl HttpRequestError {
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            HttpRequestError::Reqwest(e) => e.status(),
            HttpRequestError::Json(_) => None,
            HttpRequestError::ApiError { status, .. } => Some(*status),
        }
    }
}

/// `Some` only for a failure the service itself produced. Everything else — a query, a
/// decryption, a serialization — is Windmill's own and must not be reported as the service's.
pub fn as_external_error(e: &Error) -> Option<&ExternalApiError> {
    match e {
        Error::Anyhow { error, .. } => error.downcast_ref::<ExternalApiError>(),
        _ => None,
    }
}

/// Extract the HTTP status code from an error returned by `http_client_request`.
/// Returns `None` if the error didn't originate from an HTTP call.
pub fn http_error_status(e: &Error) -> Option<StatusCode> {
    as_external_error(e).and_then(|e| e.status)
}

/// The message to show a user for a failed provider call, without the internal decoration
/// `Error`'s own `Display` adds. Non-provider errors are rendered as-is.
pub fn external_error_message(e: &Error) -> String {
    as_external_error(e).map_or_else(|| e.to_string(), |ext| ext.to_string())
}

/// What a failed read of a trigger from its service means for the response.
#[derive(Debug)]
pub enum ExternalReadFailure {
    /// The service answered that the trigger is gone.
    Missing,
    /// The service could not be read, which says nothing about the stored trigger.
    Unreadable(String),
    /// Windmill's own failure, wearing no service's name.
    Internal(Error),
}

pub fn classify_read_failure(e: Error) -> ExternalReadFailure {
    match as_external_error(&e) {
        Some(ext) if ext.status == Some(StatusCode::NOT_FOUND) => ExternalReadFailure::Missing,
        Some(ext) => ExternalReadFailure::Unreadable(ext.to_string()),
        None => ExternalReadFailure::Internal(e),
    }
}

/// Turn a provider failure into something a client can read and act on.
///
/// Without this every rejection reaches the browser as a 500 carrying a raw upstream body.
/// The provider's status is deliberately not mirrored: a 401/403 answered by Windmill reads as
/// a Windmill permission problem, when the account that lacks rights is the one connected to
/// the *provider*. Errors that did not come from a provider call pass through untouched.
pub fn map_external_error(e: Error) -> Error {
    map_external_error_with(e, |message| message)
}

/// `map_external_error` with a chance to add what the failure means for Windmill's own state.
pub fn map_external_error_with(e: Error, decorate: impl FnOnce(String) -> String) -> Error {
    let Some((status, transient, message)) =
        as_external_error(&e).map(|ext| (ext.status, ext.is_transient(), ext.to_string()))
    else {
        return e;
    };
    let message = decorate(message);
    match status {
        // A refusal is the caller's to fix; being unable to serve the request now is not, and
        // the two want different reactions from whoever reads it.
        _ if transient => Error::BadGateway(message),
        Some(StatusCode::NOT_FOUND) => Error::NotFound(message),
        Some(_) => Error::BadRequest(message),
        // No status at all: the provider was unreachable or answered something undecodable.
        None => Error::BadGateway(message),
    }
}

fn is_transient_status(status: StatusCode) -> bool {
    status.is_server_error()
        || matches!(
            status,
            StatusCode::REQUEST_TIMEOUT | StatusCode::TOO_MANY_REQUESTS
        )
}

/// Read OAuth client_id and client_secret from instance-level global settings.
/// Used when a workspace integration has `instance_shared: true`.
async fn get_instance_oauth_credentials(
    db: &DB,
    service_name: ServiceName,
) -> Result<(String, String)> {
    windmill_common::global_settings::get_instance_oauth_credentials(
        db,
        service_name.resource_type(),
    )
    .await
}

pub async fn decrypt_oauth_data<T: DeserializeOwned>(
    db: &DB,
    workspace_id: &str,
    service_name: ServiceName,
) -> Result<T> {
    let integration = get_workspace_integration(db, workspace_id, service_name).await?;
    let oauth_data = integration.oauth_data;

    let resource_path = integration.resource_path.as_deref().ok_or_else(|| {
        Error::InternalErr(format!(
            "No resource_path in {} integration config. Please reconnect the integration.",
            service_name
        ))
    })?;

    let mc = build_crypt(db, workspace_id).await?;

    let var_row = sqlx::query!(
        "SELECT value, account FROM variable WHERE workspace_id = $1 AND path = $2",
        workspace_id,
        resource_path,
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        Error::InternalErr(format!(
            "Variable at {} not found for {} integration",
            resource_path, service_name
        ))
    })?;

    let access_token = decrypt(&mc, var_row.value)
        .map_err(|e| Error::InternalErr(format!("Failed to decrypt access token: {}", e)))?;

    let refresh_token = if let Some(account_id) = var_row.account {
        sqlx::query_scalar!(
            "SELECT refresh_token FROM account WHERE workspace_id = $1 AND id = $2",
            workspace_id,
            account_id,
        )
        .fetch_optional(db)
        .await?
    } else {
        None
    };

    let (client_id, client_secret) = if oauth_data
        .get("instance_shared")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        // Read credentials from instance-level global settings instead of workspace_integrations
        let (id, secret) = get_instance_oauth_credentials(db, service_name)
            .await
            .map_err(|e| {
                Error::InternalErr(format!(
                    "Failed to read instance OAuth credentials for {}: {}",
                    service_name, e
                ))
            })?;
        (id, secret)
    } else {
        (
            oauth_data["client_id"].as_str().unwrap_or("").to_string(),
            oauth_data["client_secret"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        )
    };

    let assembled = json!({
        "base_url": oauth_data["base_url"].as_str().unwrap_or(""),
        "access_token": access_token,
        "refresh_token": refresh_token,
        "client_id": client_id,
        "client_secret": client_secret,
    });

    serde_json::from_value(assembled)
        .map_err(|e| Error::InternalErr(format!("Failed to deserialize OAuth data: {}", e)))
}

/// Token refresh response
#[cfg(feature = "native_trigger")]
#[derive(Debug, Deserialize)]
struct RefreshTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

/// Why an OAuth token refresh did not produce a new token.
#[derive(Debug)]
pub struct RefreshFailure {
    /// The token endpoint refused the grant, so the stored credentials are what to fix.
    ///
    /// Nothing else about the answer is carried further. The status a caller reads off an
    /// `ExternalApiError` says what the call for the *trigger* answered, and 404 there means
    /// the trigger is gone — it records that on the row and lets a delete drop it. A wrong
    /// base URL 404s the token endpoint while the webhook is perfectly alive.
    pub rejected: bool,
    pub message: String,
}

#[cfg(feature = "native_trigger")]
impl RefreshFailure {
    fn rejected(message: String) -> Self {
        RefreshFailure { rejected: true, message }
    }

    fn outage(message: String) -> Self {
        RefreshFailure { rejected: false, message }
    }
}

/// Whether a token endpoint's answer means reconnecting the integration is the fix, rather
/// than the endpoint failing to serve a grant that may well be valid.
///
/// Used when the answer carried no OAuth error code to read. The body is checked because no
/// status reliably reports a refusal: GitHub answers `bad_refresh_token` with HTTP 200. The
/// status is the last resort — on a token endpoint these three most often mean the stored
/// grant is spent, and there is nothing better to go on.
pub fn grant_refused(status: Option<StatusCode>, body: &str) -> bool {
    body.contains("invalid_grant")
        || body.contains("bad_refresh_token")
        || matches!(
            status,
            Some(StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN)
        )
}

/// Whether an OAuth error code (RFC 6749 §5.2) means the credentials Windmill stored are what
/// needs replacing. The rest — a malformed request, an unsupported grant type, a scope the
/// server will not grant — are faults in how the integration asks, which reconnecting as the
/// same user will reproduce exactly.
#[cfg(feature = "native_trigger")]
fn code_means_reconnect(code: &ErrorField) -> bool {
    matches!(code, ErrorField::InvalidGrant | ErrorField::InvalidClient)
}

#[cfg(feature = "native_trigger")]
fn refresh_failure_from(e: ExecuteError) -> RefreshFailure {
    let rejected = match &e {
        // A code says exactly what was refused, so nothing else needs guessing at.
        ExecuteError::ErrorResponse { error, .. } => code_means_reconnect(&error.error),
        // A body that would not deserialize is where a refusal hides when the status says
        // nothing, and it is the only place the reason is written down.
        ExecuteError::BadResponse { status, body, .. } => {
            grant_refused(Some(*status), &String::from_utf8_lossy(body))
        }
        _ => grant_refused(e.status(), ""),
    };
    let message = match &e {
        ExecuteError::BadResponse { body, .. } => {
            format!("{e}: {}", String::from_utf8_lossy(body))
        }
        _ => error_source_chain(&e),
    };
    if rejected {
        RefreshFailure::rejected(message)
    } else {
        RefreshFailure::outage(message)
    }
}

/// Refresh OAuth tokens using windmill-oauth.
#[cfg(feature = "native_trigger")]
pub async fn refresh_oauth_tokens(
    oauth_config: &OAuthConfig,
    refresh_endpoint: &str,
    auth_endpoint: &str,
) -> std::result::Result<OAuthConfig, RefreshFailure> {
    let refresh_token_str = oauth_config
        .refresh_token
        .as_ref()
        .ok_or_else(|| RefreshFailure::rejected("no refresh token is stored".to_string()))?;

    // Build OAuth client for token refresh
    // Auth URL is not used for refresh, but required by the client constructor
    let auth_url = Url::parse(&resolve_endpoint(&oauth_config.base_url, auth_endpoint))
        .map_err(|e| RefreshFailure::outage(format!("invalid auth URL: {e}")))?;
    let token_url = Url::parse(&resolve_endpoint(&oauth_config.base_url, refresh_endpoint))
        .map_err(|e| RefreshFailure::outage(format!("invalid token URL: {e}")))?;

    let mut client = OClient::new(oauth_config.client_id.clone(), auth_url, token_url);
    client.set_client_secret(oauth_config.client_secret.clone());

    let token_response: RefreshTokenResponse = client
        .exchange_refresh_token(&RefreshToken::from(refresh_token_str.as_str()))
        .with_client(&*OAUTH_HTTP_CLIENT)
        .execute()
        .await
        .map_err(refresh_failure_from)?;

    Ok(OAuthConfig {
        base_url: oauth_config.base_url.clone(),
        access_token: token_response.access_token,
        refresh_token: token_response
            .refresh_token
            .or_else(|| oauth_config.refresh_token.clone()),
        client_id: oauth_config.client_id.clone(),
        client_secret: oauth_config.client_secret.clone(),
    })
}

/// Fallback refresh without native_triggers feature
#[cfg(not(feature = "native_trigger"))]
pub async fn refresh_oauth_tokens(
    _oauth_config: &OAuthConfig,
    _refresh_endpoint: &str,
    _auth_endpoint: &str,
) -> std::result::Result<OAuthConfig, RefreshFailure> {
    Err(RefreshFailure {
        rejected: false,
        message: "the native_trigger feature is not enabled".to_string(),
    })
}

async fn update_oauth_token_resource(
    db: &DB,
    workspace_id: &str,
    service_name: ServiceName,
    new_access_token: &str,
    new_refresh_token: Option<&str>,
) {
    let result = async {
        let integration = get_workspace_integration(db, workspace_id, service_name).await?;
        let resource_path = integration.resource_path.ok_or_else(|| {
            Error::InternalErr(format!(
                "No resource_path in {} integration config",
                service_name
            ))
        })?;

        let mc = build_crypt(db, workspace_id).await?;
        let encrypted_token = encrypt(&mc, new_access_token);

        sqlx::query!(
            "UPDATE variable SET value = $1 WHERE workspace_id = $2 AND path = $3",
            encrypted_token,
            workspace_id,
            resource_path,
        )
        .execute(db)
        .await?;

        if let Some(refresh_token) = new_refresh_token {
            sqlx::query!(
                "UPDATE account SET
                   refresh_token = $1,
                   expires_at = now() + interval '1 hour',
                   refresh_error = NULL
                 WHERE workspace_id = $2 AND client = $3 AND is_workspace_integration = true",
                refresh_token,
                workspace_id,
                service_name.as_str(),
            )
            .execute(db)
            .await?;
        } else {
            // Even without a new refresh token, update expires_at to prevent
            // the background refresh from re-refreshing immediately
            sqlx::query!(
                "UPDATE account SET
                   expires_at = now() + interval '1 hour',
                   refresh_error = NULL
                 WHERE workspace_id = $1 AND client = $2 AND is_workspace_integration = true",
                workspace_id,
                service_name.as_str(),
            )
            .execute(db)
            .await?;
        }

        Ok::<(), Error>(())
    }
    .await;

    if let Err(e) = result {
        tracing::error!(
            "Failed to update OAuth tokens for {} in workspace {}: {}",
            service_name,
            workspace_id,
            e
        );
    }
}

/// The scopes a webhook token must carry to run one runnable, and nothing else.
///
/// Always derived from the runnable the trigger points at *now*. A token that outlives a rename
/// carries the old path, and a webhook presenting it is refused however correct the URL is.
pub fn webhook_token_scopes(script_path: &str, is_flow: bool) -> Vec<String> {
    let kind = if is_flow { "flows" } else { "scripts" };
    vec![format!("jobs:run:{kind}:{script_path}")]
}

/// Create a new webhook token, minting a fresh `ephemeral-webhook-{service}-{rd5}`
/// label and the per-service expiration (see `ServiceName::webhook_token_expiration`).
/// The old token is **not** deleted — callers must call `delete_token_by_hash` on
/// `old_token_hash` after the trigger row has been successfully updated.
///
/// Returns `Ok(None)` if the old token no longer exists (e.g. manually deleted by user).
///
/// `scopes` is applied rather than carried over: the old token's scopes name whatever runnable it
/// was minted for, which a rename may since have moved. Copying them is how a re-save "succeeds"
/// while every callback it authorises is refused.
pub async fn rotate_webhook_token(
    db: &DB,
    old_token_hash: &str,
    service_name: ServiceName,
    scopes: Vec<String>,
) -> Result<Option<RotatedToken>> {
    use windmill_common::auth::{hash_token, TOKEN_PREFIX_LEN};
    use windmill_common::min_version::MIN_VERSION_SUPPORTS_TOKEN_HASH;
    use windmill_common::utils::rd_string;

    let old = match sqlx::query!(
        "SELECT email, workspace_id, super_admin, owner FROM token WHERE token_hash = $1",
        old_token_hash
    )
    .fetch_optional(db)
    .await?
    {
        Some(row) => row,
        None => {
            tracing::warn!(
                "Webhook token not found for hash {}, caller should create a fresh token",
                old_token_hash
            );
            return Ok(None);
        }
    };

    let new_token = rd_string(32);
    let new_hash = hash_token(&new_token);
    let new_prefix = new_token.get(..TOKEN_PREFIX_LEN).unwrap_or(&new_token);
    let plaintext: Option<&str> = if MIN_VERSION_SUPPORTS_TOKEN_HASH.met().await {
        None
    } else {
        Some(&new_token)
    };

    let new_label = webhook_token_label(service_name);
    let new_expiration = service_name
        .webhook_token_expiration()
        .map(|d| Utc::now() + d);

    sqlx::query!(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin, scopes, workspace_id, owner, expiration)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        new_hash,
        new_prefix,
        plaintext as Option<&str>,
        old.email,
        new_label,
        old.super_admin,
        Some(scopes.as_slice()),
        old.workspace_id,
        old.owner,
        new_expiration,
    )
    .execute(db)
    .await?;

    Ok(Some(RotatedToken {
        new_token,
        old_token_hash: old_token_hash.to_string(),
    }))
}

/// Mint the standard label used for native-trigger webhook tokens.
/// The `ephemeral-` prefix opts the token out of the user-token email/critical-alert
/// notification paths (`is_user_token` in `monitor.rs`, `register_token_expiry_notification`
/// in `windmill-api-auth/src/lib.rs`, `isUserToken` in the frontend).
pub fn webhook_token_label(service_name: ServiceName) -> String {
    use windmill_common::utils::rd_string;
    format!(
        "ephemeral-webhook-{}-{}",
        service_name.as_str(),
        rd_string(5)
    )
}

pub struct RotatedToken {
    pub new_token: String,
    /// Hash of the old token — callers should delete this after the
    /// trigger row has been successfully updated to point at the new token.
    pub old_token_hash: String,
}

/// Delete a token by hash. Returns `Ok(false)` when no row matched.
/// Some call sites legitimately race against expiry sweeps or concurrent deletes;
/// callers that consider 0-rows anomalous should log themselves at the appropriate
/// level rather than have this helper warn unconditionally.
pub async fn delete_token_by_hash<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    token_hash: &str,
) -> Result<bool> {
    let deleted = sqlx::query!("DELETE FROM token WHERE token_hash = $1", token_hash)
        .execute(db)
        .await?
        .rows_affected();

    Ok(deleted > 0)
}

pub async fn store_native_trigger<'c, E: sqlx::Executor<'c, Database = Postgres>, C: Serialize>(
    db: E,
    workspace_id: &str,
    service_name: ServiceName,
    external_id: &str,
    config: &NativeTriggerConfig,
    service_config: C,
    summary: Option<&str>,
) -> Result<()> {
    use windmill_common::auth::hash_token;

    let webhook_token_hash = hash_token(&config.webhook_token);

    sqlx::query!(
        r#"
        INSERT INTO native_trigger (
            external_id,
            workspace_id,
            service_name,
            script_path,
            is_flow,
            webhook_token_hash,
            service_config,
            summary
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8
        )
        ON CONFLICT (external_id, workspace_id, service_name)
        DO UPDATE SET script_path = $4, is_flow = $5, webhook_token_hash = $6, service_config = $7, summary = $8, error = NULL, updated_at = NOW()
        "#,
        external_id,
        workspace_id,
        service_name as ServiceName,
        config.script_path,
        config.is_flow,
        webhook_token_hash,
        sqlx::types::Json(service_config) as _,
        summary,
    )
    .execute(db)
    .await?;

    Ok(())
}

/// Record the outcome of re-registering a webhook: the token that now authenticates it and the
/// config the service resolved.
///
/// Deliberately not `store_native_trigger`: the runnable a trigger points at belongs to whoever
/// renamed or edited it, not to the re-registration, which only ever holds the path as it stood
/// before its network call. Writing that path back would undo a rename that landed in between.
///
/// Conditional on `updated_at` — the row version — for the same reason. `TriggerLock` does not
/// cover the rename's own `UPDATE`, because deploys must never block on a third party, so another
/// write can land *during* this registration's network call. Recording then would attach a token
/// for state that no longer holds and, worse, clear the `REREGISTRATION_PENDING` a newer rename
/// set to protect itself. Comparing the path alone would not catch it: a save at the same path, or
/// a rename away and back, leaves the path equal while the token has moved on.
///
/// Returns `false` when the row changed, leaving it untouched for whoever wrote it to finish.
pub(crate) async fn record_reregistration<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    workspace_id: &str,
    service_name: ServiceName,
    external_id: &str,
    webhook_token: &str,
    service_config: serde_json::Value,
    expected_updated_at: DateTime<Utc>,
) -> Result<bool> {
    use windmill_common::auth::hash_token;

    let applied = sqlx::query_scalar!(
        r#"
        UPDATE native_trigger
        SET webhook_token_hash = $1, service_config = $2, error = NULL, updated_at = NOW()
        WHERE
            workspace_id = $3
            AND service_name = $4
            AND external_id = $5
            AND updated_at = $6
        RETURNING 1 AS "applied!"
        "#,
        hash_token(webhook_token),
        sqlx::types::Json(service_config) as _,
        workspace_id,
        service_name as ServiceName,
        external_id,
        expected_updated_at,
    )
    .fetch_optional(db)
    .await?
    .is_some();

    Ok(applied)
}

/// Apply a trigger edit, unless the runnable moved under it in the meantime.
///
/// A rename writes `native_trigger.script_path` from inside the deploy transaction, which is not
/// under `TriggerLock` — so an edit holding the lock across its network call can still have the
/// ground shift beneath it. Writing its snapshot's path back would undo the rename and hide the
/// trigger from every listing, which is the bug this whole change exists to fix, so refuse instead.
///
/// Callers MUST have verified write access to `config.script_path`.
///
/// Returns `false` when the runnable moved; the edit is then stale and the caller should say so.
pub(crate) async fn update_native_trigger_if_runnable_unchanged<
    'c,
    E: sqlx::Executor<'c, Database = Postgres>,
>(
    db: E,
    workspace_id: &str,
    service_name: ServiceName,
    external_id: &str,
    config: &NativeTriggerConfig,
    service_config: serde_json::Value,
    summary: Option<&str>,
    expected_script_path: &str,
    expected_is_flow: bool,
) -> Result<bool> {
    use windmill_common::auth::hash_token;

    let applied = sqlx::query_scalar!(
        r#"
        UPDATE native_trigger
        SET script_path = $1, is_flow = $2, webhook_token_hash = $3, service_config = $4,
            summary = $5, error = NULL, updated_at = NOW()
        WHERE
            workspace_id = $6
            AND service_name = $7
            AND external_id = $8
            AND script_path = $9
            AND is_flow = $10
        RETURNING 1 AS "applied!"
        "#,
        config.script_path,
        config.is_flow,
        hash_token(&config.webhook_token),
        sqlx::types::Json(service_config) as _,
        summary,
        workspace_id,
        service_name as ServiceName,
        external_id,
        expected_script_path,
        expected_is_flow,
    )
    .fetch_optional(db)
    .await?
    .is_some();

    Ok(applied)
}

pub async fn update_native_trigger<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    workspace_id: &str,
    service_name: ServiceName,
    external_id: &str,
    config: &NativeTriggerConfig,
    service_config: Option<&RawValue>,
    summary: Option<&str>,
) -> Result<()> {
    use windmill_common::auth::hash_token;

    let webhook_token_hash = hash_token(&config.webhook_token);

    sqlx::query!(
        r#"
        UPDATE native_trigger
        SET script_path = $1, is_flow = $2, webhook_token_hash = $3, service_config = $4, summary = $8, error = NULL, updated_at = NOW()
        WHERE
            workspace_id = $5
            AND service_name = $6
            AND external_id = $7
        "#,
        config.script_path,
        config.is_flow,
        webhook_token_hash,
        service_config.map(sqlx::types::Json) as _,
        workspace_id,
        service_name as ServiceName,
        external_id,
        summary,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn delete_native_trigger<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    workspace_id: &str,
    service_name: ServiceName,
    external_id: &str,
) -> Result<bool> {
    let deleted = sqlx::query!(
        r#"
        DELETE FROM native_trigger
        WHERE
            workspace_id = $1
            AND service_name = $2
            AND external_id = $3
        "#,
        workspace_id,
        service_name as ServiceName,
        external_id,
    )
    .execute(db)
    .await?
    .rows_affected();

    Ok(deleted > 0)
}
pub async fn get_native_trigger<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    workspace_id: &str,
    service_name: ServiceName,
    external_id: &str,
) -> Result<Option<NativeTrigger>> {
    let trigger = sqlx::query_as!(
        NativeTrigger,
        r#"
        SELECT
            external_id,
            workspace_id,
            service_name AS "service_name!: ServiceName",
            script_path,
            is_flow,
            webhook_token_hash,
            service_config,
            error,
            created_at,
            updated_at,
            summary
        FROM
            native_trigger
        WHERE
            workspace_id = $1
            AND service_name = $2
            AND external_id = $3
        "#,
        workspace_id,
        service_name as ServiceName,
        external_id
    )
    .fetch_optional(db)
    .await?;

    Ok(trigger)
}

pub async fn get_native_trigger_by_script<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    workspace_id: &str,
    service_name: ServiceName,
    script_path: &str,
    is_flow: bool,
) -> Result<Option<NativeTrigger>> {
    let trigger = sqlx::query_as!(
        NativeTrigger,
        r#"
        SELECT
            external_id,
            workspace_id,
            service_name AS "service_name!: ServiceName",
            script_path,
            is_flow,
            webhook_token_hash,
            service_config,
            error,
            created_at,
            updated_at,
            summary
        FROM
            native_trigger
        WHERE
            workspace_id = $1
            AND service_name = $2
            AND script_path = $3
            AND is_flow = $4
        LIMIT 1
        "#,
        workspace_id,
        service_name as ServiceName,
        script_path,
        is_flow
    )
    .fetch_optional(db)
    .await?;

    Ok(trigger)
}

pub async fn list_native_triggers<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    workspace_id: &str,
    service_name: ServiceName,
    page: Option<usize>,
    per_page: Option<usize>,
    path: Option<&str>,
    is_flow: Option<bool>,
) -> Result<Vec<NativeTrigger>> {
    let offset = (page.unwrap_or(0) * per_page.unwrap_or(100)) as i64;
    let limit = per_page.unwrap_or(100) as i64;

    let triggers = sqlx::query_as!(
        NativeTrigger,
        r#"
        SELECT
            nt.external_id,
            nt.workspace_id,
            nt.service_name AS "service_name!: ServiceName",
            nt.script_path,
            nt.is_flow,
            nt.webhook_token_hash,
            nt.service_config,
            nt.error,
            nt.created_at,
            nt.updated_at,
            nt.summary
        FROM
            native_trigger nt
        WHERE
            nt.workspace_id = $1 AND
            nt.service_name = $2 AND
            ($5::text IS NULL OR nt.script_path = $5) AND
            ($6::bool IS NULL OR nt.is_flow = $6) AND
            (
                (nt.is_flow = false AND EXISTS (
                    SELECT 1 FROM script s
                    WHERE s.workspace_id = nt.workspace_id
                    AND s.path = nt.script_path
                ))
                OR
                (nt.is_flow = true AND EXISTS (
                    SELECT 1 FROM flow f
                    WHERE f.workspace_id = nt.workspace_id
                    AND f.path = nt.script_path
                ))
            )
        LIMIT $3
        OFFSET $4
        "#,
        workspace_id,
        service_name as ServiceName,
        limit,
        offset,
        path,
        is_flow
    )
    .fetch_all(db)
    .await?;

    Ok(triggers)
}

pub async fn update_native_trigger_error<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    workspace_id: &str,
    service_name: ServiceName,
    external_id: &str,
    error: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE native_trigger
        SET error = $1
        WHERE
            workspace_id = $2
            AND service_name = $3
            AND external_id = $4
        "#,
        error,
        workspace_id,
        service_name as ServiceName,
        external_id,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn update_native_trigger_service_config<
    'c,
    E: sqlx::Executor<'c, Database = Postgres>,
>(
    db: E,
    workspace_id: &str,
    service_name: ServiceName,
    external_id: &str,
    service_config: &serde_json::Value,
    new_webhook_token: Option<&str>,
) -> Result<()> {
    let new_hash = new_webhook_token.map(windmill_common::auth::hash_token);

    sqlx::query!(
        r#"
        UPDATE native_trigger
        SET service_config = $1,
            webhook_token_hash = COALESCE($5, webhook_token_hash),
            updated_at = NOW()
        WHERE
            workspace_id = $2
            AND service_name = $3
            AND external_id = $4
        "#,
        service_config,
        workspace_id,
        service_name as ServiceName,
        external_id,
        new_hash.as_deref(),
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn store_workspace_integration(
    tx: &mut PgConnection,
    authed: &ApiAuthed,
    workspace_id: &str,
    service_name: ServiceName,
    oauth_data: serde_json::Value,
    resource_path: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO workspace_integrations (
            workspace_id,
            service_name,
            oauth_data,
            resource_path,
            created_by,
            created_at,
            updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, now(), now()
        )
        ON CONFLICT (workspace_id, service_name)
        DO UPDATE SET
            oauth_data = $3,
            resource_path = $4,
            updated_at = now()
        "#,
        workspace_id,
        service_name as ServiceName,
        oauth_data,
        resource_path,
        authed.username,
    )
    .execute(&mut *tx)
    .await?;

    Ok(())
}

/// Authorization gate for the integration *use* routes (calendar/drive/repo/event
/// pickers). A workspace admin configures the integration, but any member who can
/// create a native trigger needs the pickers to configure one. Operators are
/// read-only and cannot create triggers, so they must not be able to drive the
/// admin-configured integration's upstream API and enumerate its data.
pub fn require_native_integration_use(authed: &ApiAuthed) -> Result<()> {
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "Operators cannot use workspace integrations".to_string(),
        ));
    }
    Ok(())
}

pub async fn get_workspace_integration<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    workspace_id: &str,
    service_name: ServiceName,
) -> Result<WorkspaceIntegration> {
    let integration = sqlx::query_as!(
        WorkspaceIntegration,
        r#"
        SELECT
            workspace_id,
            service_name AS "service_name!: ServiceName",
            oauth_data,
            resource_path,
            created_at,
            updated_at,
            created_by
        FROM
            workspace_integrations
        WHERE
            workspace_id = $1
            AND service_name = $2
        "#,
        workspace_id,
        service_name as ServiceName,
    )
    .fetch_one(db)
    .await?;

    Ok(integration)
}

pub async fn delete_workspace_integration(
    tx: &mut PgConnection,
    workspace_id: &str,
    service_name: ServiceName,
) -> Result<bool> {
    let deleted = sqlx::query!(
        r#"
        DELETE FROM workspace_integrations
        WHERE
            workspace_id = $1
            AND service_name = $2
        "#,
        workspace_id,
        service_name as ServiceName,
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    Ok(deleted > 0)
}

/// Generates the webhook URL that external services will call.
///
/// `external_id` is optional because during CREATE we don't have it yet
/// (it's returned by the external service). During UPDATE, we have it.
pub fn generate_webhook_service_url(
    base_url: &str,
    w_id: &str,
    script_path: &str,
    is_flow: bool,
    external_id: Option<&str>,
    service_name: ServiceName,
    webhook_token: &str,
) -> String {
    let runnable_prefix = if is_flow { "f" } else { "p" };

    let mut url = format!(
        "{}/api/w/{}/jobs/run/{}/{}?token={}&service_name={}",
        base_url,
        w_id,
        runnable_prefix,
        script_path,
        &webhook_token,
        service_name.as_str(),
    );

    if let Some(id) = external_id {
        url.push_str(&format!("&trigger_external_id={}", id));
    }

    url
}

/// Process incoming webhook request for a native trigger service.
/// Dispatches to the service-specific `prepare_webhook` to transform headers/body into args.
/// Returns `None` if the service doesn't need special processing (standard body parsing is used).
#[cfg(feature = "native_trigger")]
pub async fn prepare_native_trigger_args(
    service_name: ServiceName,
    db: &DB,
    w_id: &str,
    headers: &http::HeaderMap,
    body: String,
) -> Result<Option<PushArgsOwned>> {
    let headers_map: HashMap<String, String> = headers
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
        .collect();

    match service_name {
        ServiceName::Google => {
            let handler = google::Google;
            let args = handler
                .prepare_webhook(db, w_id, headers_map, body, "", false)
                .await?;
            Ok(Some(args))
        }
        ServiceName::Github => {
            let handler = github::GitHub;
            let args = handler
                .prepare_webhook(db, w_id, headers_map, body, "", false)
                .await?;
            Ok(Some(args))
        }
        ServiceName::Nextcloud => Ok(None),
    }
}

/// Fallback when native_trigger feature is disabled
#[cfg(not(feature = "native_trigger"))]
pub async fn prepare_native_trigger_args(
    _service_name: ServiceName,
    _db: &DB,
    _w_id: &str,
    _headers: &http::HeaderMap,
    _body: String,
) -> Result<Option<PushArgsOwned>> {
    Ok(None)
}
