//! Native Triggers Module
//!
//! This module provides integration with external services (like Nextcloud) that can
//! trigger Windmill scripts/flows via webhooks.
//!
//! ## Adding a New Native Trigger Service
//!
//! When adding a new service (e.g., "NewService"), you need to update the following locations:
//!
//! ### 1. This file (mod.rs):
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
use itertools::Itertools;
use reqwest::{Client, Method};
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
    variables::{build_crypt, decrypt, encrypt},
    DB,
};
use windmill_queue::PushArgsOwned;

#[cfg(feature = "native_trigger")]
use windmill_oauth::{OClient, RefreshToken, Url, OAUTH_HTTP_CLIENT};

use crate::db::ApiAuthed;
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
/// Note: `Google` service handles both Drive and Calendar triggers via trigger_type.
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

    /// Returns the integration service name for workspace_integrations lookup.
    /// For most services, this is the service itself.
    pub fn integration_service(&self) -> ServiceName {
        *self
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

impl std::fmt::Display for ServiceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
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

    async fn create(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        webhook_token: &str,
        data: &NativeTriggerData<Self::ServiceConfig>,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<Self::CreateResponse>;

    async fn update(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        webhook_token: &str,
        data: &NativeTriggerData<Self::ServiceConfig>,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<()>;

    async fn get(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<Self::TriggerData>;

    async fn delete(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<()>;

    #[allow(unused)]
    async fn exists(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<bool>;

    async fn list_all(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<Vec<Self::TriggerData>>;

    async fn prepare_webhook(
        &self,
        _db: &DB,
        _w_id: &str,
        _header: HashMap<String, String>,
        _body: String,
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

    fn get_external_id_from_trigger_data(&self, data: &Self::TriggerData) -> String;

    /// Extracts the service-specific config from trigger data (from external service).
    /// Used for comparison during sync to detect config drift.
    /// Default implementation converts the trigger data to a JSON value
    /// If you need to exclude some fields, skip serializing attributes on the TriggerData struct or override this method.
    fn extract_service_config_from_trigger_data(
        &self,
        data: &Self::TriggerData,
    ) -> Result<serde_json::Value> {
        serde_json::to_value(data).map_err(|e| {
            Error::internal_err(format!("Failed to convert trigger data to JSON: {}", e))
        })
    }

    fn additional_routes(&self) -> axum::Router {
        axum::Router::new()
    }

    async fn http_client_request<T: DeserializeOwned + Send, B: Serialize + Send + Sync>(
        &self,
        url: &str,
        method: Method,
        workspace_id: &str,
        tx: &mut PgConnection,
        db: &DB,
        headers: Option<HashMap<String, String>>,
        body: Option<&B>,
    ) -> Result<T> {
        let oauth_config: OAuthConfig =
            decrypt_oauth_data(tx, db, workspace_id, Self::SERVICE_NAME).await?;

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
            Err(ref err) if is_unauthorized_error(err) => {
                tracing::info!("HTTP 401 for {}, attempting token refresh", url);

                let refreshed_oauth_config =
                    refresh_oauth_tokens(&oauth_config, Self::REFRESH_ENDPOINT).await?;

                task::spawn({
                    let db_clone = db.clone();
                    let workspace_id_clone = workspace_id.to_string();
                    let refreshed_json = oauth_config_to_json(&refreshed_oauth_config);
                    async move {
                        update_workspace_integration_tokens_helper(
                            db_clone,
                            workspace_id_clone,
                            Self::SERVICE_NAME,
                            refreshed_json,
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
            }
            other => other,
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

fn is_unauthorized_error(err: &Error) -> bool {
    match err {
        Error::InternalErr(msg) => msg.starts_with("HTTP 401 "),
        _ => false,
    }
}

pub async fn make_http_request<T: DeserializeOwned + Send, B: Serialize>(
    url: &str,
    method: Method,
    headers: Option<HashMap<String, String>>,
    body: Option<&B>,
    access_token: &str,
) -> Result<T> {
    let client = Client::new();
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

    let response = request.send().await.map_err(to_anyhow)?;
    let status = response.status();

    if !status.is_success() {
        let body_text = response.text().await.unwrap_or_default();
        let message = if let Ok(err_json) = serde_json::from_str::<serde_json::Value>(&body_text) {
            err_json
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or(&body_text)
                .to_string()
        } else {
            body_text
        };
        return Err(Error::InternalErr(format!(
            "HTTP {} for {}: {}",
            status.as_u16(),
            url,
            message
        )));
    }

    response
        .json()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to parse response from {}: {}", url, e)))
}

pub async fn decrypt_oauth_data<
    'c,
    E: sqlx::Executor<'c, Database = Postgres>,
    T: DeserializeOwned,
>(
    tx: E,
    db: &DB,
    workspace_id: &str,
    service_name: ServiceName,
) -> Result<T> {
    let integration = get_workspace_integration(tx, workspace_id, service_name).await?;

    let mc = build_crypt(db, workspace_id).await?;
    let mut oauth_data: serde_json::Value = integration.oauth_data;

    if let Some(encrypted_access_token) = oauth_data.get("access_token").and_then(|v| v.as_str()) {
        let decrypted_access_token = decrypt(&mc, encrypted_access_token.to_string())
            .map_err(|e| Error::InternalErr(format!("Failed to decrypt access token: {}", e)))?;
        oauth_data["access_token"] = serde_json::Value::String(decrypted_access_token);
    }

    if let Some(encrypted_refresh_token) = oauth_data.get("refresh_token").and_then(|v| v.as_str())
    {
        let decrypted_refresh_token = decrypt(&mc, encrypted_refresh_token.to_string())
            .map_err(|e| Error::InternalErr(format!("Failed to decrypt refresh token: {}", e)))?;
        oauth_data["refresh_token"] = serde_json::Value::String(decrypted_refresh_token);
    }

    serde_json::from_value(oauth_data)
        .map_err(|e| Error::InternalErr(format!("Failed to deserialize OAuth data: {}", e)))
}

#[allow(unused)]
pub fn oauth_data_to_config(oauth_data: &serde_json::Value) -> Result<OAuthConfig> {
    let base_url = oauth_data
        .get("base_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::InternalErr("No base_url in OAuth data".to_string()))?
        .to_string();

    let access_token = oauth_data
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::InternalErr("No access_token in OAuth data".to_string()))?
        .to_string();

    let refresh_token = oauth_data
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let client_id = oauth_data
        .get("client_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::InternalErr("No client_id in OAuth data".to_string()))?
        .to_string();

    let client_secret = oauth_data
        .get("client_secret")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::InternalErr("No client_secret in OAuth data".to_string()))?
        .to_string();

    Ok(OAuthConfig { base_url, access_token, refresh_token, client_id, client_secret })
}

#[inline]
pub fn oauth_config_to_json(config: &OAuthConfig) -> serde_json::Value {
    let mut json = json!({
        "base_url": config.base_url,
        "access_token": config.access_token,
        "client_id": config.client_id,
        "client_secret": config.client_secret,
    });

    if let Some(refresh_token) = &config.refresh_token {
        json["refresh_token"] = serde_json::Value::String(refresh_token.clone());
    }

    json
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
) -> Result<OAuthConfig> {
    let refresh_token_str = oauth_config
        .refresh_token
        .as_ref()
        .ok_or_else(|| Error::InternalErr("No refresh token available".to_string()))?;

    // Auth URL is not used for refresh, but required by the client constructor
    let auth_url_str = if oauth_config.base_url.is_empty() {
        "https://localhost/oauth/authorize".to_string()
    } else {
        format!("{}/oauth/authorize", oauth_config.base_url)
    };
    let auth_url = Url::parse(&auth_url_str)
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
) -> Result<OAuthConfig> {
    Err(Error::InternalErr(
        "Native triggers feature is not enabled".to_string(),
    ))
}

async fn update_workspace_integration_tokens_helper(
    db: DB,
    workspace_id: String,
    service_name: ServiceName,
    oauth_data: serde_json::Value,
) {
    let result = async {
        let mut tx = db.begin().await?;
        let mc = build_crypt(&db, &workspace_id).await?;
        let mut encrypted_oauth_data = oauth_data;

        if let Some(access_token) = encrypted_oauth_data
            .get("access_token")
            .and_then(|v| v.as_str())
        {
            let encrypted_access_token = encrypt(&mc, access_token);
            encrypted_oauth_data["access_token"] =
                serde_json::Value::String(encrypted_access_token);
        }

        if let Some(refresh_token) = encrypted_oauth_data
            .get("refresh_token")
            .and_then(|v| v.as_str())
        {
            let encrypted_refresh_token = encrypt(&mc, refresh_token);
            encrypted_oauth_data["refresh_token"] =
                serde_json::Value::String(encrypted_refresh_token);
        }

        sqlx::query!(
            r#"
            UPDATE workspace_integrations 
            SET oauth_data = $1, updated_at = now()
            WHERE workspace_id = $2 AND service_name = $3
            "#,
            encrypted_oauth_data,
            workspace_id,
            service_name as ServiceName,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok::<(), Error>(())
    }
    .await;

    if let Err(e) = result {
        tracing::error!("Critical error: Failed to update workspace integration tokens for {} in workspace {}: {}", 
            service_name, workspace_id, e);
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
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO workspace_integrations (
            workspace_id,
            service_name,
            oauth_data,
            created_by,
            created_at,
            updated_at
        ) VALUES (
            $1, $2, $3, $4, now(), now()
        )
        ON CONFLICT (workspace_id, service_name)
        DO UPDATE SET
            oauth_data = $3,
            updated_at = now()
        "#,
        workspace_id,
        service_name as ServiceName,
        oauth_data,
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
) -> Result<Option<windmill_queue::PushArgsOwned>> {
    let headers_map: HashMap<String, String> = headers
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
        .collect();

    match service_name {
        ServiceName::Google => {
            let handler = google::Google;
            let args = handler.prepare_webhook(db, w_id, headers_map, body).await?;
            Ok(Some(args))
        }
        _ => Ok(None),
    }
}
