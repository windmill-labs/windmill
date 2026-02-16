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
use windmill_oauth::{OClient, RefreshToken, Url, OAUTH_HTTP_CLIENT};

use windmill_api_auth::ApiAuthed;
pub mod handler;
pub mod sync;
pub mod workspace_integrations;

// Service modules - add new services here:
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
}

impl TryFrom<String> for ServiceName {
    type Error = Error;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        let service = match value.as_str() {
            "nextcloud" => ServiceName::Nextcloud,
            "google" => ServiceName::Google,
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
        }
    }

    /// Returns the corresponding TriggerKind for this service.
    pub fn as_trigger_kind(&self) -> TriggerKind {
        match self {
            ServiceName::Nextcloud => TriggerKind::Nextcloud,
            ServiceName::Google => TriggerKind::Google,
        }
    }

    /// Returns the corresponding JobTriggerKind for this service.
    pub fn as_job_trigger_kind(&self) -> windmill_common::jobs::JobTriggerKind {
        match self {
            ServiceName::Nextcloud => windmill_common::jobs::JobTriggerKind::Nextcloud,
            ServiceName::Google => windmill_common::jobs::JobTriggerKind::Google,
        }
    }

    /// Returns the OAuth token endpoint path for this service.
    pub fn token_endpoint(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "/apps/oauth2/api/v1/token",
            ServiceName::Google => "https://oauth2.googleapis.com/token",
        }
    }

    /// Returns the OAuth authorization endpoint path for this service.
    pub fn auth_endpoint(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "/apps/oauth2/authorize",
            ServiceName::Google => "https://accounts.google.com/o/oauth2/v2/auth",
        }
    }

    /// Returns the OAuth scopes for this service's authorization flow.
    pub fn oauth_scopes(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "read write",
            ServiceName::Google => "https://www.googleapis.com/auth/drive.readonly https://www.googleapis.com/auth/calendar.readonly https://www.googleapis.com/auth/calendar.events",
        }
    }

    /// Returns the resource type used for storing OAuth tokens.
    pub fn resource_type(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "nextcloud",
            ServiceName::Google => "gworkspace",
        }
    }

    /// Returns extra OAuth authorization parameters required by this service.
    pub fn extra_auth_params(&self) -> &[(&'static str, &'static str)] {
        match self {
            ServiceName::Google => &[("access_type", "offline"), ("prompt", "consent")],
            ServiceName::Nextcloud => &[],
        }
    }

    /// Returns the integration service name for workspace_integrations lookup.
    pub fn integration_service(&self) -> ServiceName {
        *self
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
    pub webhook_token_prefix: String,
    pub service_config: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
            Err(err)
                if err.status() == Some(StatusCode::UNAUTHORIZED)
                    || err.status() == Some(StatusCode::FORBIDDEN) =>
            {
                tracing::info!(
                    "HTTP auth error ({}), attempting token refresh",
                    err.status().unwrap()
                );

                let refreshed_oauth_config = refresh_oauth_tokens(
                    &oauth_config,
                    Self::REFRESH_ENDPOINT,
                    Self::AUTH_ENDPOINT,
                )
                .await?;

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

                let response = make_http_request(
                    url,
                    method,
                    headers,
                    body.as_ref(),
                    &refreshed_oauth_config.access_token,
                )
                .await
                .map_err(to_anyhow)?;
                Ok(response)
            }
            Err(e) => Err(to_anyhow(e).into()),
        }
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

    let response = request.send().await?.error_for_status()?;

    // Handle empty responses (e.g. 204 No Content from Google channels/stop)
    let bytes = response.bytes().await?;
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
}

impl std::fmt::Display for HttpRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpRequestError::Reqwest(e) => write!(f, "{}", e),
            HttpRequestError::Json(e) => write!(f, "JSON decode error: {}", e),
        }
    }
}

impl std::error::Error for HttpRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HttpRequestError::Reqwest(e) => Some(e),
            HttpRequestError::Json(e) => Some(e),
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
        }
    }
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

/// Refresh OAuth tokens using windmill-oauth.
#[cfg(feature = "native_trigger")]
pub async fn refresh_oauth_tokens(
    oauth_config: &OAuthConfig,
    refresh_endpoint: &str,
    auth_endpoint: &str,
) -> Result<OAuthConfig> {
    let refresh_token_str = oauth_config
        .refresh_token
        .as_ref()
        .ok_or_else(|| Error::InternalErr("No refresh token available".to_string()))?;

    // Build OAuth client for token refresh
    // Auth URL is not used for refresh, but required by the client constructor
    let auth_url = Url::parse(&resolve_endpoint(&oauth_config.base_url, auth_endpoint))
        .map_err(|e| Error::InternalErr(format!("Invalid auth URL: {}", e)))?;
    let token_url = Url::parse(&resolve_endpoint(&oauth_config.base_url, refresh_endpoint))
        .map_err(|e| Error::InternalErr(format!("Invalid token URL: {}", e)))?;

    let mut client = OClient::new(oauth_config.client_id.clone(), auth_url, token_url);
    client.set_client_secret(oauth_config.client_secret.clone());

    let token_response: RefreshTokenResponse = client
        .exchange_refresh_token(&RefreshToken::from(refresh_token_str.as_str()))
        .with_client(&*OAUTH_HTTP_CLIENT)
        .execute()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to refresh token: {:?}", e)))?;

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
) -> Result<OAuthConfig> {
    Err(Error::InternalErr(
        "Native triggers feature is not enabled".to_string(),
    ))
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

/// Look up the full token from the token table using its prefix
pub async fn get_token_by_prefix<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    token_prefix: &str,
) -> Result<Option<String>> {
    let token = sqlx::query_scalar!(
        r#"
        SELECT token
        FROM token
        WHERE token LIKE concat($1::text, '%')
        LIMIT 1
        "#,
        token_prefix
    )
    .fetch_optional(db)
    .await?;

    Ok(token)
}

/// Delete a token from the token table using its prefix
pub async fn delete_token_by_prefix<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    token_prefix: &str,
) -> Result<bool> {
    let deleted = sqlx::query!(
        r#"
        DELETE FROM token
        WHERE token LIKE concat($1::text, '%')
        "#,
        token_prefix
    )
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
) -> Result<()> {
    // Store only the first 10 characters of the webhook token as a prefix
    let webhook_token_prefix: String = config.webhook_token.chars().take(10).collect();

    sqlx::query!(
        r#"
        INSERT INTO native_trigger (
            external_id,
            workspace_id,
            service_name,
            script_path,
            is_flow,
            webhook_token_prefix,
            service_config
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7
        )
        ON CONFLICT (external_id, workspace_id, service_name)
        DO UPDATE SET script_path = $4, is_flow = $5, webhook_token_prefix = $6, service_config = $7, error = NULL, updated_at = NOW()
        "#,
        external_id,
        workspace_id,
        service_name as ServiceName,
        config.script_path,
        config.is_flow,
        webhook_token_prefix,
        sqlx::types::Json(service_config) as _,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn update_native_trigger<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    workspace_id: &str,
    service_name: ServiceName,
    external_id: &str,
    config: &NativeTriggerConfig,
    service_config: Option<&RawValue>,
) -> Result<()> {
    // Store only the first 10 characters of the webhook token as a prefix
    let webhook_token_prefix: String = config.webhook_token.chars().take(10).collect();

    sqlx::query!(
        r#"
        UPDATE native_trigger
        SET script_path = $1, is_flow = $2, webhook_token_prefix = $3, service_config = $4, error = NULL, updated_at = NOW()
        WHERE
            workspace_id = $5
            AND service_name = $6
            AND external_id = $7
        "#,
        config.script_path,
        config.is_flow,
        webhook_token_prefix,
        service_config.map(sqlx::types::Json) as _,
        workspace_id,
        service_name as ServiceName,
        external_id,
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
            webhook_token_prefix,
            service_config,
            error,
            created_at,
            updated_at
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
            webhook_token_prefix,
            service_config,
            error,
            created_at,
            updated_at
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
            nt.webhook_token_prefix,
            nt.service_config,
            nt.error,
            nt.created_at,
            nt.updated_at
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
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE native_trigger
        SET service_config = $1, updated_at = NOW()
        WHERE
            workspace_id = $2
            AND service_name = $3
            AND external_id = $4
        "#,
        service_config,
        workspace_id,
        service_name as ServiceName,
        external_id,
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
