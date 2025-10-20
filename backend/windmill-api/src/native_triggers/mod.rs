use crate::db::ApiAuthed;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use http::StatusCode;
use itertools::Itertools;
use reqwest::{Client, Method};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection, Postgres};
use std::{collections::HashMap, fmt::Debug};
use strum::{EnumIter, IntoEnumIterator};
use tokio::task;
use windmill_common::{
    error::{to_anyhow, Error, Result},
    triggers::TriggerKind,
    utils::RunnableKind,
    variables::{build_crypt, decrypt, encrypt},
    DB,
};
use windmill_queue::PushArgsOwned;
pub mod handler;
pub mod sync;
pub mod workspace_integrations;

#[cfg(feature = "native_triggers")]
pub mod nextcloud;

#[derive(EnumIter, sqlx::Type, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[sqlx(type_name = "native_trigger_service", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]

pub enum ServiceName {
    Nextcloud,
}

impl TryFrom<String> for ServiceName {
    type Error = Error;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        let service = match value.as_str() {
            "nextcloud" => ServiceName::Nextcloud,
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
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "nextcloud",
        }
    }
    pub fn as_trigger_kind(&self) -> TriggerKind {
        match self {
            ServiceName::Nextcloud => TriggerKind::Nextcloud,
        }
    }
}

impl std::fmt::Display for ServiceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let service_name = match self {
            ServiceName::Nextcloud => "nextcloud",
        };

        write!(f, "{}", service_name)
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NativeTrigger {
    pub id: i64,
    pub service_name: ServiceName,
    pub external_id: String,
    pub workspace_id: String,
    pub runnable_path: String,
    pub runnable_kind: RunnableKind,
    pub event_type: sqlx::types::Json<EventType>,
    pub summary: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub edited_by: String,
    pub email: String,
    pub edited_at: DateTime<Utc>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct NativeTriggerData<P> {
    pub external_id: String,
    pub runnable_path: String,
    pub runnable_kind: RunnableKind,
    pub summary: Option<String>,
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
        internal_id: i64,
        oauth_data: &Self::OAuthData,
        data: &NativeTriggerData<Self::Payload>,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<Self::CreateResponse>;

    async fn update(
        &self,
        w_id: &str,
        internal_id: i64,
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
        _runnable_path: &str,
        _is_flow: bool,
    ) -> Result<PushArgsOwned> {
        Ok(PushArgsOwned { extra: None, args: HashMap::new() })
    }

    fn external_id_and_metadata_from_response(
        &self,
        resp: &Self::CreateResponse,
    ) -> (String, Option<serde_json::Value>);

    fn get_external_id_from_trigger_data(&self, data: &Self::TriggerData) -> String;

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

pub async fn store_native_trigger<'c, E: sqlx::Executor<'c, Database = Postgres>, P>(
    db: E,
    authed: &ApiAuthed,
    workspace_id: &str,
    service_name: ServiceName,
    native_trigger_data: &NativeTriggerData<P>,
) -> Result<i64> {
    let row = sqlx::query!(
        r#"
        INSERT INTO native_triggers (
            service_name,
            event_type,
            external_id,
            runnable_path,
            runnable_kind,
            workspace_id,
            summary,
            metadata,
            edited_by,
            email,
            edited_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now()
        )
        RETURNING id
        "#,
        service_name as ServiceName,
        serde_json::to_value(native_trigger_data.event_type.clone()).unwrap(),
        &native_trigger_data.external_id,
        &native_trigger_data.runnable_path,
        native_trigger_data.runnable_kind as RunnableKind,
        workspace_id,
        native_trigger_data.summary.as_ref(),
        Some(serde_json::Value::Null),
        authed.username,
        authed.email,
    )
    .fetch_one(db)
    .await?;

    Ok(row.id)
}

pub async fn update_native_trigger<'c, E: sqlx::Executor<'c, Database = Postgres>, P>(
    db: E,
    authed: &ApiAuthed,
    workspace_id: &str,
    id: i64,
    service_name: ServiceName,
    native_trigger_data: &NativeTriggerData<P>,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE 
            native_triggers
        SET
            runnable_path = $1,
            runnable_kind = $2,
            summary = $3,
            metadata = $4,
            edited_by = $5,
            email = $6,
            edited_at = now()
        WHERE
            workspace_id = $7
            AND id = $8
            AND service_name = $9
        "#,
        native_trigger_data.runnable_path,
        native_trigger_data.runnable_kind as RunnableKind,
        native_trigger_data.summary,
        Some(serde_json::Value::Null),
        authed.username,
        authed.email,
        workspace_id,
        id,
        service_name as ServiceName
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn delete_native_trigger<'c, E: sqlx::Executor<'c, Database = Postgres>>(
    db: E,
    workspace_id: &str,
    id: i64,
    service_name: ServiceName,
) -> Result<bool> {
    let deleted = sqlx::query!(
        r#"
        DELETE 
            FROM native_triggers
        WHERE
            workspace_id = $1
            AND id = $2
            AND service_name = $3
        "#,
        workspace_id,
        id,
        service_name as ServiceName
    )
    .execute(db)
    .await?
    .rows_affected();

    Ok(deleted > 0)
}
#[allow(unused)]
pub async fn get_native_trigger_by_external_id(
    tx: &mut PgConnection,
    workspace_id: &str,
    service_name: ServiceName,
    external_id: &str,
) -> Result<Option<NativeTrigger>> {
    let trigger = sqlx::query_as!(
        NativeTrigger,
        r#"
        SELECT
            id,
            event_type AS "event_type!: sqlx::types::Json<EventType>",
            runnable_path,
            runnable_kind AS "runnable_kind!: RunnableKind",
            service_name AS "service_name!: ServiceName",
            external_id,
            workspace_id,
            summary,
            metadata,
            edited_by,
            email,
            edited_at
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
    .fetch_optional(tx)
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
                id,
                runnable_path,
                event_type AS "event_type!: sqlx::types::Json<EventType>",
                runnable_kind AS "runnable_kind!: RunnableKind",
                service_name AS "service_name!: ServiceName",
                external_id,
                workspace_id,
                summary,
                metadata,
                edited_by,
                email,
                edited_at
            FROM
                native_triggers
            WHERE
                workspace_id = $1 AND 
                service_name = $2
            ORDER BY 
                edited_at DESC
            LIMIT 
                $3 
            OFFSET 
                $4
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

pub fn generate_webhook_service_url(
    base_url: &str,
    w_id: &str,
    runnable_path: &str,
    runnable_kind: RunnableKind,
    internal_id: &str,
    service_name: ServiceName,
    webhook_config: &WebhookConfig,
) -> String {
    let endpoint_base = match webhook_config.request_type {
        WebhookRequestType::Async => "run",
        WebhookRequestType::Sync => "run_wait_result",
    };

    let runnable_prefix = match runnable_kind {
        RunnableKind::Script => "p",
        RunnableKind::Flow => "f",
    };

    let url = format!(
        "{}/api/w/{}/jobs/{}/{}/{}?token={}&internal_id={}&service_name={}",
        base_url,
        w_id,
        endpoint_base,
        runnable_prefix,
        runnable_path,
        &webhook_config.token,
        internal_id,
        service_name.as_str()
    );

    url
}
