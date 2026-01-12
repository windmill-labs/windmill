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
//! - Add `pub mod newservice;` under the `#[cfg(feature = "native_triggers")]` block
//! - Add `NewService` variant to `ServiceName` enum
//! - Update `ServiceName::as_str()` - add match arm returning `"newservice"`
//! - Update `TryFrom<String> for ServiceName` - add match arm for `"newservice"`
//! - Update `ServiceName::as_trigger_kind()` - add match arm (requires TriggerKind::NewService in windmill_common)
//! - Update `ServiceName::fmt()` (Display impl) - add match arm
//! - Add match arm in `dispatch_webhook_args()` function
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

use crate::db::ApiAuthed;
pub mod handler;
pub mod sync;
pub mod workspace_integrations;

// Service modules - add new services here:
#[cfg(feature = "native_triggers")]
pub mod nextcloud;
// #[cfg(feature = "native_triggers")]
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
    pub config: serde_json::Value,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebhookRequestType {
    Async,
    Sync,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub request_type: WebhookRequestType,
    #[serde(default)]
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EventType {
    Webhook(WebhookConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeTriggerConfig {
    pub script_path: String,
    pub is_flow: bool,
    pub event_type: EventType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NativeTriggerData<P> {
    pub script_path: String,
    pub is_flow: bool,
    pub event_type: EventType,
    pub payload: P,
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
    type Payload: Debug + DeserializeOwned + Serialize + Send + Sync;
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
        data: &NativeTriggerData<Self::Payload>,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<Self::CreateResponse>;

    async fn update(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        data: &NativeTriggerData<Self::Payload>,
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

    async fn validate_data_config(&self, _data: &NativeTriggerData<Self::Payload>) -> Result<()> {
        Ok(())
    }

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

    /// Extracts the service-specific config from the payload (for storage).
    /// This should include fields like event, event_filter, user_id_filter, etc.
    fn extract_service_config_from_payload(&self, payload: &Self::Payload) -> Box<RawValue>;

    /// Extracts the service-specific config from trigger data (from external service).
    /// Used for comparison during sync to detect config drift.
    fn extract_service_config_from_trigger_data(&self, data: &Self::TriggerData) -> Box<RawValue>;

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

pub async fn refresh_oauth_tokens(
    oauth_config: &OAuthConfig,
    refresh_endpoint: &str,
) -> Result<OAuthConfig> {
    let refresh_token = oauth_config
        .refresh_token
        .as_ref()
        .ok_or_else(|| Error::InternalErr("No refresh token available".to_string()))?;

    let client = Client::new();

    let params = [
        ("grant_type", "refresh_token"),
        ("client_id", &oauth_config.client_id),
        ("client_secret", &oauth_config.client_secret),
        ("refresh_token", refresh_token),
    ];

    let response = client
        .post(format!("{}{}", oauth_config.base_url, refresh_endpoint))
        .form(&params)
        .send()
        .await
        .map_err(to_anyhow)?
        .error_for_status()
        .map_err(to_anyhow)?;

    let token_data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to parse token response: {}", e)))?;

    let new_access_token = token_data["access_token"]
        .as_str()
        .ok_or_else(|| Error::InternalErr("No access_token in refresh response".to_string()))?
        .to_string();

    let new_refresh_token = token_data
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| oauth_config.refresh_token.clone());

    Ok(OAuthConfig {
        base_url: oauth_config.base_url.clone(),
        access_token: new_access_token,
        refresh_token: new_refresh_token,
        client_id: oauth_config.client_id.clone(),
        client_secret: oauth_config.client_secret.clone(),
    })
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

pub async fn store_native_trigger<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    workspace_id: &str,
    service_name: ServiceName,
    external_id: &str,
    config: &NativeTriggerConfig,
    service_config: Option<&RawValue>,
) -> Result<()> {
    let event_type_str = serde_json::to_string(&config.event_type)
        .map_err(|e| Error::InternalErr(format!("Failed to serialize event_type: {}", e)))?;

    // Build the combined config JSON string, embedding the RawValue directly
    let service_config_str = service_config.map(|v| v.get()).unwrap_or("null");
    let config_str = format!(
        r#"{{"event_type": {}, "service_config": {}}}"#,
        event_type_str, service_config_str
    );
    let config_json: serde_json::Value = serde_json::from_str(&config_str)
        .map_err(|e| Error::InternalErr(format!("Failed to parse config JSON: {}", e)))?;

    sqlx::query!(
        r#"
        INSERT INTO native_triggers (
            external_id,
            workspace_id,
            service_name,
            script_path,
            is_flow,
            config
        ) VALUES (
            $1, $2, $3, $4, $5, $6
        )
        ON CONFLICT (external_id, workspace_id, service_name)
        DO UPDATE SET script_path = $4, is_flow = $5, config = $6, error = NULL
        "#,
        external_id,
        workspace_id,
        service_name as ServiceName,
        config.script_path,
        config.is_flow,
        config_json,
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
    let event_type_str = serde_json::to_string(&config.event_type)
        .map_err(|e| Error::InternalErr(format!("Failed to serialize event_type: {}", e)))?;

    let service_config_str = service_config.map(|v| v.get()).unwrap_or("null");
    let config_str = format!(
        r#"{{"event_type": {}, "service_config": {}}}"#,
        event_type_str, service_config_str
    );
    let config_json: serde_json::Value = serde_json::from_str(&config_str)
        .map_err(|e| Error::InternalErr(format!("Failed to parse config JSON: {}", e)))?;

    sqlx::query!(
        r#"
        UPDATE native_triggers
        SET script_path = $1, is_flow = $2, config = $3, error = NULL
        WHERE
            workspace_id = $4
            AND service_name = $5
            AND external_id = $6
        "#,
        config.script_path,
        config.is_flow,
        config_json,
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
        DELETE FROM native_triggers
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
            config,
            error
        FROM
            native_triggers
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
            config,
            error
        FROM
            native_triggers
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
) -> Result<Vec<NativeTrigger>> {
    let offset = (page.unwrap_or(0) * per_page.unwrap_or(100)) as i64;
    let limit = per_page.unwrap_or(100) as i64;

    let triggers = sqlx::query_as!(
        NativeTrigger,
        r#"
        SELECT
            external_id,
            workspace_id,
            service_name AS "service_name!: ServiceName",
            script_path,
            is_flow,
            config,
            error
        FROM
            native_triggers
        WHERE
            workspace_id = $1 AND
            service_name = $2
        LIMIT $3
        OFFSET $4
        "#,
        workspace_id,
        service_name as ServiceName,
        limit,
        offset
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
        UPDATE native_triggers
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

pub async fn update_native_trigger_service_config<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    workspace_id: &str,
    service_name: ServiceName,
    external_id: &str,
    service_config: &RawValue,
) -> Result<()> {
    // Parse the RawValue to Value for SQLx
    let service_config_value: serde_json::Value = serde_json::from_str(service_config.get())
        .map_err(|e| Error::InternalErr(format!("Failed to parse service_config: {}", e)))?;

    sqlx::query!(
        r#"
        UPDATE native_triggers
        SET config = jsonb_set(config, '{service_config}', $1::jsonb)
        WHERE
            workspace_id = $2
            AND service_name = $3
            AND external_id = $4
        "#,
        service_config_value,
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
    webhook_config: &WebhookConfig,
) -> String {
    let endpoint_base = match webhook_config.request_type {
        WebhookRequestType::Async => "run",
        WebhookRequestType::Sync => "run_wait_result",
    };

    let runnable_prefix = if is_flow { "f" } else { "p" };

    let mut url = format!(
        "{}/api/w/{}/jobs/{}/{}/{}?token={}&service_name={}",
        base_url,
        w_id,
        endpoint_base,
        runnable_prefix,
        script_path,
        &webhook_config.token,
        service_name.as_str()
    );

    // Add trigger_id if we have it (for updates, not initial creation)
    if let Some(id) = external_id {
        url.push_str(&format!("&trigger_id={}", id));
    }

    url
}

/// Macro to register all native trigger services.
/// This is the single place where new services need to be added.
/// When adding a new service:
/// 1. Add the variant to ServiceName enum
/// 2. Add the service module (like `pub mod newservice;`)
/// 3. Add the service to this macro invocation
#[cfg(feature = "native_triggers")]
#[macro_export]
macro_rules! for_each_native_service {
    ($macro_name:ident) => {
        $macro_name!(nextcloud, NextCloud, crate::native_triggers::nextcloud::NextCloud);
        // Add new services here:
        // $macro_name!(servicename, ServiceVariant, path::to::Handler);
    };
}

#[cfg(not(feature = "native_triggers"))]
#[macro_export]
macro_rules! for_each_native_service {
    ($macro_name:ident) => {
        // No services when feature is disabled
    };
}

/// Dispatches webhook argument building to the appropriate service handler.
/// This is the centralized place for webhook dispatch logic.
/// When adding a new service, add a new match arm here.
#[cfg(feature = "native_triggers")]
pub async fn dispatch_webhook_args(
    service_name: ServiceName,
    db: &DB,
    w_id: &str,
    script_path: &str,
    is_flow: bool,
    body: Box<serde_json::value::RawValue>,
    headers: HashMap<String, Box<serde_json::value::RawValue>>,
) -> Result<PushArgsOwned> {
    use crate::triggers::trigger_helpers::TriggerJobArgs;

    match service_name {
        ServiceName::Nextcloud => {
            use nextcloud::NextCloud;
            NextCloud::build_job_args(script_path, is_flow, w_id, db, body, headers).await
        }
        // Add new services here:
        // ServiceName::NewService => {
        //     use newservice::NewServiceHandler;
        //     NewServiceHandler::build_job_args(script_path, is_flow, w_id, db, body, headers).await
        // }
    }
}

#[cfg(not(feature = "native_triggers"))]
pub async fn dispatch_webhook_args(
    _service_name: ServiceName,
    _db: &DB,
    _w_id: &str,
    _script_path: &str,
    _is_flow: bool,
    _body: Box<serde_json::value::RawValue>,
    _headers: HashMap<String, Box<serde_json::value::RawValue>>,
) -> Result<PushArgsOwned> {
    Err(Error::BadRequest("Native triggers feature is not enabled".to_string()))
}
