use axum::{
    extract::Path,
    routing::{delete, get, post},
    Extension, Json, Router,
};

#[cfg(feature = "native_triggers")]
use serde_json::to_value;
use sqlx::prelude::FromRow;
use strum::IntoEnumIterator;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
#[cfg(feature = "native_triggers")]
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{to_anyhow, Error, JsonResult, Result},
    utils::require_admin,
    variables::{build_crypt, encrypt},
    DB,
};

#[cfg(feature = "native_triggers")]
use crate::{
    db::ApiAuthed,
    native_triggers::{delete_workspace_integration, store_workspace_integration, ServiceName},
};

use std::{collections::HashSet, sync::Mutex};

lazy_static::lazy_static! {
    static ref STATE_STORE: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

#[derive(Debug, Serialize)]
pub struct IntegrationStatusResponse {
    pub connected: bool,
    pub service_name: ServiceName,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_by: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListIntegrationsResponse {
    pub integrations: Vec<IntegrationStatusResponse>,
}

#[derive(Debug, Serialize)]
pub struct ConnectIntegrationResponse {
    pub auth_url: String,
}

#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub base_url: String,
    pub access_token: Option<String>
}

#[derive(Debug, Serialize)]
pub struct OAuthConfigResponse {
    pub configured: bool,
    pub base_url: Option<String>,
    pub redirect_uri: Option<String>,
}

#[cfg(feature = "native_triggers")]
async fn generate_connect_url(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((workspace_id, service_name)): Path<(String, ServiceName)>,
    Json(RedirectUri { redirect_uri }): Json<RedirectUri>,
) -> JsonResult<String> {
    require_admin(authed.is_admin, &workspace_id)?;

    let oauth_config =
        get_workspace_oauth_config_as_oauth_config(&db, &workspace_id, service_name).await?;
    let random_state = uuid::Uuid::new_v4().to_string();
    let auth_url = build_authorization_url(&oauth_config, &random_state, &redirect_uri)?;
    Ok(Json(auth_url))
}

#[cfg(feature = "native_triggers")]
async fn delete_integration(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name)): Path<(String, ServiceName)>,
) -> JsonResult<String> {
    require_admin(authed.is_admin, &workspace_id)?;

    let mut tx = user_db.begin(&authed).await?;

    let deleted = delete_workspace_integration(&mut *tx, &workspace_id, service_name).await?;

    if !deleted {
        return Err(Error::NotFound(format!(
            "{} integration not found for workspace",
            service_name
        )));
    }

    audit_log(
        &mut *tx,
        &authed,
        &format!("workspace_integrations.{}.disconnect", service_name),
        ActionKind::Delete,
        &workspace_id,
        Some(&format!("Disconnected {} integration", service_name)),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(Json(format!(
        "{} integration disconnected successfully",
        service_name
    )))
}

#[derive(FromRow, Debug, Deserialize, Serialize)]
struct WorkspaceIntegrations {
    service_name: ServiceName,
    oauth_data: Option<sqlx::types::Json<WorkspaceOAuthConfig>>,
}

#[cfg(feature = "native_triggers")]
async fn list_integrations(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(_user_db): Extension<UserDB>,
    Path(workspace_id): Path<String>,
) -> JsonResult<Vec<WorkspaceIntegrations>> {
    require_admin(authed.is_admin, &workspace_id)?;
    let mut tx = db.begin().await?;
    let integrations = sqlx::query_as!(
        WorkspaceIntegrations,
        r#"
        SELECT 
            oauth_data as "oauth_data!: sqlx::types::Json<WorkspaceOAuthConfig>",
            service_name as "service_name!: ServiceName"
        FROM 
            workspace_integrations 
        WHERE 
            workspace_id = $1
        "#,
        workspace_id
    )
    .fetch_all(&mut *tx)
    .await?;

    let key_value = integrations
        .into_iter()
        .map(|integration| (integration.service_name, integration.oauth_data))
        .collect::<std::collections::HashMap<_, _>>();

    let integrations = ServiceName::iter()
        .map(|service_name| WorkspaceIntegrations {
            service_name: service_name,
            oauth_data: key_value.get(&service_name).cloned().flatten(),
        })
        .collect::<Vec<_>>();

    tx.commit().await?;

    Ok(Json(integrations))
}

async fn integration_exist(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name)): Path<(String, ServiceName)>,
) -> JsonResult<bool> {
    let mut tx = user_db.begin(&authed).await?;
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM workspace_integrations
            WHERE workspace_id = $1
            AND service_name = $2 
            AND oauth_data IS NOT NULL
        )
        "#,
        workspace_id,
        service_name as ServiceName
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(false);

    Ok(Json(exists))
}

#[derive(Debug, Deserialize)]
struct RedirectUri {
    redirect_uri: String,
}

#[cfg(feature = "native_triggers")]
async fn oauth_callback(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name, code, state)): Path<(String, ServiceName, String, String)>,
    Json(RedirectUri { redirect_uri }): Json<RedirectUri>,
) -> JsonResult<String> {
    require_admin(authed.is_admin, &workspace_id)?;

    let state_was_present = STATE_STORE.lock().unwrap().remove(&state);

    if !state_was_present {
        return Err(Error::BadRequest("Unknown state parameter".to_string()));
    }

    let oauth_config =
        get_workspace_oauth_config::<WorkspaceOAuthConfig>(&db, &workspace_id, service_name)
            .await?;

    let token_response = exchange_code_for_token(&oauth_config, &code, &redirect_uri).await?;

    let mut tx = user_db.begin(&authed).await?;

    let mc = build_crypt(&db, &workspace_id).await?;
    let mut oauth_data = serde_json::to_value(oauth_config).unwrap();

    let encrypted_access_token = encrypt(&mc, &token_response.access_token);
    oauth_data["access_token"] = serde_json::Value::String(encrypted_access_token);

    if let Some(refresh_token) = token_response.refresh_token {
        let encrypted_refresh_token = encrypt(&mc, &refresh_token);
        oauth_data["refresh_token"] = serde_json::Value::String(encrypted_refresh_token);
    }
    if let Some(expires_at) = token_response.expires_at {
        oauth_data["token_expires_at"] = serde_json::Value::String(expires_at.to_rfc3339());
    }

    store_workspace_integration(&mut *tx, &authed, &workspace_id, service_name, oauth_data).await?;

    audit_log(
        &mut *tx,
        &authed,
        &format!("workspace_integrations.{}.connect", service_name),
        ActionKind::Create,
        &workspace_id,
        Some(&format!("Connected {} integration via OAuth", service_name)),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(Json(format!(
        "{} integration connected successfully via OAuth",
        service_name
    )))
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

async fn exchange_code_for_token(
    config: &WorkspaceOAuthConfig,
    code: &str,
    redirect_uri: &str,
) -> Result<TokenResponse> {
    let client = reqwest::Client::new();

    let params = [
        ("grant_type", "authorization_code"),
        ("client_id", &config.client_id),
        ("client_secret", &config.client_secret),
        ("code", code),
        ("redirect_uri", redirect_uri),
    ];

    let response = client
        .post(format!("{}/apps/oauth2/api/v1/token", config.base_url))
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

    let expires_at = if let Some(expires_in) = token_data.get("expires_in").and_then(|v| v.as_u64())
    {
        Some(chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64))
    } else {
        None
    };

    Ok(TokenResponse {
        access_token: token_data["access_token"]
            .as_str()
            .ok_or_else(|| Error::BadRequest("Missing access_token in response".to_string()))?
            .to_string(),
        refresh_token: token_data["refresh_token"].as_str().map(|s| s.to_string()),
        expires_in: token_data["expires_in"].as_u64(),
        expires_at,
    })
}

async fn get_workspace_oauth_config<T: DeserializeOwned>(
    db: &DB,
    workspace_id: &str,
    service_name: ServiceName,
) -> Result<T> {
    let oauth_configs = sqlx::query_scalar!(
        r#"
        SELECT 
            oauth_data 
        FROM 
            workspace_integrations 
        WHERE 
            workspace_id = $1 AND
            service_name = $2
        "#,
        workspace_id,
        service_name as ServiceName
    )
    .fetch_optional(db)
    .await?
    .ok_or(Error::NotFound(format!(
        "Integration for service {} not found",
        service_name.as_str()
    )))?;

    let config = serde_json::from_value::<T>(oauth_configs)
        .map_err(|e| Error::InternalErr(format!("Failed to parse OAuth config: {}", e)))?;

    Ok(config)
}

#[cfg(feature = "native_triggers")]
pub async fn create_workspace_integration(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name)): Path<(String, ServiceName)>,
    Json(oauth_data): Json<WorkspaceOAuthConfig>,
) -> Result<()> {
    require_admin(authed.is_admin, &workspace_id)?;

    let mut tx = user_db.begin(&authed).await?;

    store_workspace_integration(
        &mut tx,
        &authed,
        &workspace_id,
        service_name,
        to_value(oauth_data).unwrap(),
    )
    .await?;

    tx.commit().await?;

    Ok(())
}

#[inline]
async fn get_workspace_oauth_config_as_oauth_config(
    db: &DB,
    workspace_id: &str,
    service_name: ServiceName,
) -> Result<WorkspaceOAuthConfig> {
    get_workspace_oauth_config::<WorkspaceOAuthConfig>(db, workspace_id, service_name).await
}

fn build_authorization_url(
    config: &WorkspaceOAuthConfig,
    state: &str,
    redirect_uri: &str,
) -> Result<String> {
    let params = [
        ("response_type", "code"),
        ("client_id", &config.client_id),
        ("redirect_uri", redirect_uri),
        ("state", state),
        ("scope", "read write"),
    ];

    let query_string = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    {
        let mut store = STATE_STORE.lock().unwrap();

        store.insert(state.to_owned());
    }

    Ok(format!(
        "{}/apps/oauth2/authorize?{}",
        config.base_url, query_string
    ))
}

pub fn workspaced_service() -> Router {
    let router = Router::new()
        .route("/list", get(list_integrations))
        .route("/:service_name/exists", get(integration_exist))
        .route("/:service_name/create", post(create_workspace_integration))
        .route(
            "/:service_name/generate_connect_url",
            post(generate_connect_url),
        )
        .route("/:service_name/delete", delete(delete_integration))
        .route("/:service_name/callback/:code/:state", post(oauth_callback));

    Router::new().nest("/integrations", router)
}
