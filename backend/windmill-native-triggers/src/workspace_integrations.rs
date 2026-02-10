use axum::Router;

#[cfg(feature = "native_trigger")]
use axum::{
    extract::Path,
    routing::{delete, get, post},
    Extension, Json,
};

#[cfg(feature = "native_trigger")]
use serde_json::to_value;

#[cfg(feature = "native_trigger")]
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[cfg(feature = "native_trigger")]
use sqlx::prelude::FromRow;

#[cfg(feature = "native_trigger")]
use windmill_audit::{audit_oss::audit_log, ActionKind};

#[cfg(feature = "native_trigger")]
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    utils::require_admin,
    variables::{build_crypt, encrypt},
    DB,
};

#[cfg(feature = "native_trigger")]
use windmill_api_auth::ApiAuthed;

#[cfg(feature = "native_trigger")]
use crate::ServiceName;

#[cfg(feature = "native_trigger")]
use crate::{delete_workspace_integration, store_workspace_integration};

#[cfg(feature = "native_trigger")]
use windmill_oauth::{OClient, Url, OAUTH_HTTP_CLIENT};

#[cfg(feature = "native_trigger")]
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
#[cfg(feature = "native_trigger")]
use hmac::{Hmac, Mac};
#[cfg(feature = "native_trigger")]
use sha2::Sha256;

#[cfg(feature = "native_trigger")]
type HmacSha256 = Hmac<Sha256>;

#[cfg(feature = "native_trigger")]
const STATE_EXPIRATION_SECONDS: i64 = 600; // 10 minutes

/// Generate a signed OAuth state that is cluster-safe.
/// The state contains: workspace_id, service_name, timestamp, and nonce.
/// It's signed with HMAC-SHA256 using the workspace key.
#[cfg(feature = "native_trigger")]
async fn generate_signed_state(
    db: &DB,
    workspace_id: &str,
    service_name: ServiceName,
) -> Result<String> {
    use windmill_common::variables::get_workspace_key;

    let nonce = uuid::Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().timestamp();
    let payload = format!(
        "{}:{}:{}:{}",
        workspace_id,
        service_name.as_str(),
        timestamp,
        nonce
    );

    // Get workspace key for signing
    let key = get_workspace_key(workspace_id, db).await?;
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .map_err(|e| Error::InternalErr(e.to_string()))?;
    mac.update(payload.as_bytes());
    let signature = mac.finalize().into_bytes();

    // Encode as: base64(payload):base64(signature)
    let encoded_payload = URL_SAFE_NO_PAD.encode(payload.as_bytes());
    let encoded_signature = URL_SAFE_NO_PAD.encode(signature);

    Ok(format!("{}:{}", encoded_payload, encoded_signature))
}

/// Validate a signed OAuth state.
/// Returns true if the state is valid (correct signature and not expired).
#[cfg(feature = "native_trigger")]
async fn validate_signed_state(db: &DB, state: &str, workspace_id: &str) -> Result<bool> {
    use windmill_common::variables::get_workspace_key;

    let parts: Vec<&str> = state.split(':').collect();
    if parts.len() != 2 {
        return Ok(false);
    }

    let encoded_payload = parts[0];
    let encoded_signature = parts[1];

    // Decode payload
    let payload_bytes = match URL_SAFE_NO_PAD.decode(encoded_payload) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(false),
    };
    let payload = match String::from_utf8(payload_bytes) {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };

    // Parse payload: workspace_id:service_name:timestamp:nonce
    let payload_parts: Vec<&str> = payload.split(':').collect();
    if payload_parts.len() != 4 {
        return Ok(false);
    }

    let state_workspace_id = payload_parts[0];
    let timestamp: i64 = match payload_parts[2].parse() {
        Ok(ts) => ts,
        Err(_) => return Ok(false),
    };

    // Verify workspace_id matches
    if state_workspace_id != workspace_id {
        return Ok(false);
    }

    // Check expiration
    let now = chrono::Utc::now().timestamp();
    if now - timestamp > STATE_EXPIRATION_SECONDS {
        return Ok(false);
    }

    // Verify signature
    let key = get_workspace_key(workspace_id, db).await?;
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .map_err(|e| Error::InternalErr(e.to_string()))?;
    mac.update(payload.as_bytes());

    let received_signature = match URL_SAFE_NO_PAD.decode(encoded_signature) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(false),
    };

    Ok(mac.verify_slice(&received_signature).is_ok())
}

#[cfg(feature = "native_trigger")]
#[derive(Debug, Serialize)]
pub struct IntegrationStatusResponse {
    pub connected: bool,
    pub service_name: ServiceName,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_by: Option<String>,
}

#[cfg(feature = "native_trigger")]
#[derive(Debug, Serialize)]
pub struct ListIntegrationsResponse {
    pub integrations: Vec<IntegrationStatusResponse>,
}

#[cfg(feature = "native_trigger")]
#[derive(Debug, Serialize)]
pub struct ConnectIntegrationResponse {
    pub auth_url: String,
}

#[cfg(feature = "native_trigger")]
#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub base_url: String,
    pub access_token: Option<String>,
}

#[cfg(feature = "native_trigger")]
#[derive(Debug, Serialize)]
pub struct OAuthConfigResponse {
    pub configured: bool,
    pub base_url: Option<String>,
    pub redirect_uri: Option<String>,
}

#[cfg(feature = "native_trigger")]
async fn generate_connect_url(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((workspace_id, service_name)): Path<(String, ServiceName)>,
    Json(RedirectUri { redirect_uri }): Json<RedirectUri>,
) -> JsonResult<String> {
    require_admin(authed.is_admin, &workspace_id)?;

    let oauth_config =
        get_workspace_oauth_config_as_oauth_config(&db, &workspace_id, service_name).await?;

    // Generate a signed state that is cluster-safe
    let state = generate_signed_state(&db, &workspace_id, service_name).await?;
    let auth_url = build_authorization_url(&oauth_config, &state, &redirect_uri);
    Ok(Json(auth_url))
}

#[cfg(feature = "native_trigger")]
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

#[cfg(feature = "native_trigger")]
#[derive(FromRow, Debug, Deserialize, Serialize)]
struct WorkspaceIntegrations {
    service_name: ServiceName,
    oauth_data: Option<sqlx::types::Json<WorkspaceOAuthConfig>>,
}

#[cfg(feature = "native_trigger")]
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

    use strum::IntoEnumIterator;
    let integrations = ServiceName::iter()
        .map(|service_name| WorkspaceIntegrations {
            service_name: service_name,
            oauth_data: key_value.get(&service_name).cloned().flatten(),
        })
        .collect::<Vec<_>>();

    tx.commit().await?;

    Ok(Json(integrations))
}

#[cfg(feature = "native_trigger")]
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

#[cfg(feature = "native_trigger")]
#[derive(Debug, Deserialize)]
struct RedirectUri {
    redirect_uri: String,
}

#[cfg(feature = "native_trigger")]
async fn oauth_callback(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name, code, state)): Path<(String, ServiceName, String, String)>,
    Json(RedirectUri { redirect_uri }): Json<RedirectUri>,
) -> JsonResult<String> {
    require_admin(authed.is_admin, &workspace_id)?;

    // Validate the signed state (cluster-safe, no DB storage needed)
    let state_was_valid = validate_signed_state(&db, &state, &workspace_id).await?;

    if !state_was_valid {
        return Err(Error::BadRequest(
            "Invalid or expired state parameter".to_string(),
        ));
    }

    let oauth_config =
        get_workspace_oauth_config::<WorkspaceOAuthConfig>(&db, &workspace_id, service_name)
            .await?;

    let token_response =
        exchange_code_for_token(&oauth_config, service_name, &code, &redirect_uri).await?;

    let mut tx = user_db.begin(&authed).await?;

    let mc = build_crypt(&db, &workspace_id).await?;
    let mut oauth_data = serde_json::to_value(oauth_config).unwrap();

    let encrypted_access_token = encrypt(&mc, &token_response.access_token);
    oauth_data["access_token"] = serde_json::Value::String(encrypted_access_token);

    if let Some(refresh_token) = token_response.refresh_token {
        let encrypted_refresh_token = encrypt(&mc, &refresh_token);
        oauth_data["refresh_token"] = serde_json::Value::String(encrypted_refresh_token);
    }
    if let Some(expires_in) = token_response.expires_in {
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64);
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

/// Token response from OAuth token exchange
#[cfg(feature = "native_trigger")]
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
}

/// Build an OAuth client for native trigger services using windmill-oauth.
#[cfg(feature = "native_trigger")]
fn build_native_oauth_client(
    config: &WorkspaceOAuthConfig,
    service_name: ServiceName,
    redirect_uri: &str,
) -> Result<OClient> {
    let auth_url = Url::parse(&format!(
        "{}{}",
        config.base_url,
        service_name.auth_endpoint()
    ))
    .map_err(|e| Error::InternalErr(format!("Invalid auth URL: {}", e)))?;
    let token_url = Url::parse(&format!(
        "{}{}",
        config.base_url,
        service_name.token_endpoint()
    ))
    .map_err(|e| Error::InternalErr(format!("Invalid token URL: {}", e)))?;
    let redirect = Url::parse(redirect_uri).map_err(|e| {
        Error::BadRequest(format!(
            "Invalid redirect URI '{}': {}. The redirect URI must be an absolute URL (e.g., https://example.com/callback)",
            redirect_uri, e
        ))
    })?;

    let mut client = OClient::new(config.client_id.clone(), auth_url, token_url);
    client.set_client_secret(config.client_secret.clone());
    client.set_redirect_url(redirect);

    Ok(client)
}

/// Exchange authorization code for tokens using windmill-oauth.
#[cfg(feature = "native_trigger")]
async fn exchange_code_for_token(
    config: &WorkspaceOAuthConfig,
    service_name: ServiceName,
    code: &str,
    redirect_uri: &str,
) -> Result<TokenResponse> {
    let client = build_native_oauth_client(config, service_name, redirect_uri)?;

    let token_response: TokenResponse = client
        .exchange_code(code.to_string())
        .with_client(&*OAUTH_HTTP_CLIENT)
        .execute()
        .await
        .map_err(|e| Error::InternalErr(format!("Failed to exchange code for token: {:?}", e)))?;

    Ok(token_response)
}

#[cfg(feature = "native_trigger")]
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

#[cfg(feature = "native_trigger")]
pub async fn create_workspace_integration(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name)): Path<(String, ServiceName)>,
    Json(oauth_data): Json<WorkspaceOAuthConfig>,
) -> Result<()> {
    require_admin(authed.is_admin, &workspace_id)?;

    let mut tx = user_db.begin(&authed).await?;

    crate::store_workspace_integration(
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

#[cfg(feature = "native_trigger")]
#[inline]
async fn get_workspace_oauth_config_as_oauth_config(
    db: &DB,
    workspace_id: &str,
    service_name: ServiceName,
) -> Result<WorkspaceOAuthConfig> {
    get_workspace_oauth_config::<WorkspaceOAuthConfig>(db, workspace_id, service_name).await
}

#[cfg(feature = "native_trigger")]
fn build_authorization_url(
    config: &WorkspaceOAuthConfig,
    state: &str,
    redirect_uri: &str,
) -> String {
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

    format!("{}/apps/oauth2/authorize?{}", config.base_url, query_string)
}

#[cfg(feature = "native_trigger")]
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

#[cfg(not(feature = "native_trigger"))]
pub fn workspaced_service() -> Router {
    Router::new()
}
