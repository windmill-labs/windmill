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

use windmill_api_auth::ApiAuthed;
pub mod handler;
pub mod sync;
pub mod workspace_integrations;

// Service modules - add new services here:
#[cfg(feature = "native_trigger")]
pub mod nextcloud;
// #[cfg(feature = "native_trigger")]
// pub mod newservice;

/// Enum of all supported native trigger services.
/// When adding a new service, add a variant here (e.g., `NewService`).
#[derive(EnumIter, sqlx::Type, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[sqlx(type_name = "native_trigger_service", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ServiceName {
    Nextcloud,
    // Add new services here:
    // NewService,
}

impl TryFrom<String> for ServiceName {
    type Error = Error;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        // Add new service match arms here:
        let service = match value.as_str() {
            "nextcloud" => ServiceName::Nextcloud,
            // "newservice" => ServiceName::NewService,
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
    /// Add new service match arms here.
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "nextcloud",
            // ServiceName::NewService => "newservice",
        }
    }

    /// Returns the corresponding TriggerKind for this service.
    /// Requires adding the variant to TriggerKind in windmill_common.
    pub fn as_trigger_kind(&self) -> TriggerKind {
        match self {
            ServiceName::Nextcloud => TriggerKind::Nextcloud,
            // ServiceName::NewService => TriggerKind::NewService,
        }
    }

    /// Returns the corresponding JobTriggerKind for this service.
    /// Requires adding the variant to JobTriggerKind in windmill_common.
    pub fn as_job_trigger_kind(&self) -> windmill_common::jobs::JobTriggerKind {
        match self {
            ServiceName::Nextcloud => windmill_common::jobs::JobTriggerKind::Nextcloud,
            // ServiceName::NewService => windmill_common::jobs::JobTriggerKind::NewService,
        }
    }

    /// Returns the OAuth token endpoint path for this service.
    /// Used for building OAuth clients dynamically.
    pub fn token_endpoint(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "/apps/oauth2/api/v1/token",
            // ServiceName::NewService => "/oauth/token",
        }
    }

    /// Returns the OAuth authorization endpoint path for this service.
    /// Used for building OAuth authorization URLs.
    pub fn auth_endpoint(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "/apps/oauth2/authorize",
            // ServiceName::NewService => "/oauth/authorize",
        }
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
        _script_path: &str,
        _is_flow: bool,
    ) -> Result<PushArgsOwned> {
        Ok(PushArgsOwned { extra: None, args: HashMap::new() })
    }

    fn external_id_and_metadata_from_response(
        &self,
        resp: &Self::CreateResponse,
    ) -> (String, Option<serde_json::Value>);

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
            Err(err)
                if err.status() == Some(StatusCode::UNAUTHORIZED)
                    || err.status() == Some(StatusCode::FORBIDDEN) =>
            {
                tracing::info!(
                    "HTTP auth error ({}), attempting token refresh",
                    err.status().unwrap()
                );

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
) -> std::result::Result<T, reqwest::Error> {
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

    let response = request.send().await?.error_for_status()?;

    let response_json = response.json().await?;

    Ok(response_json)
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

    // Build OAuth client for token refresh
    // Auth URL is not used for refresh, but required by the client constructor
    let auth_url = Url::parse(&format!("{}/oauth/authorize", oauth_config.base_url))
        .map_err(|e| Error::InternalErr(format!("Invalid auth URL: {}", e)))?;
    let token_url = Url::parse(&format!("{}{}", oauth_config.base_url, refresh_endpoint))
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
