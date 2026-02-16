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
    global_settings::{load_value_from_global_settings, OAUTH_SETTING},
    utils::{require_admin, HTTP_CLIENT},
    variables::{build_crypt, encrypt},
    DB,
};

#[cfg(feature = "native_trigger")]
use windmill_api_auth::ApiAuthed;

#[cfg(feature = "native_trigger")]
use crate::{
    decrypt_oauth_data, delete_token_by_prefix, delete_workspace_integration, resolve_endpoint,
    store_workspace_integration, ServiceName,
};

#[cfg(feature = "native_trigger")]
use windmill_oauth::{OClient, Url, OAUTH_HTTP_CLIENT};

#[cfg(feature = "native_trigger")]
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
#[cfg(feature = "native_trigger")]
use hmac::{Hmac, Mac};
#[cfg(feature = "native_trigger")]
use serde_json::json;
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
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub instance_shared: bool,
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
    let auth_url = build_authorization_url(&oauth_config, service_name, &state, &redirect_uri);
    Ok(Json(auth_url))
}

#[cfg(feature = "native_trigger")]
#[derive(Debug, Deserialize)]
struct BasicOAuthData {
    base_url: String,
    access_token: String,
}

#[cfg(feature = "native_trigger")]
async fn try_delete_nextcloud_webhook(base_url: &str, access_token: &str, external_id: &str) {
    let url = format!(
        "{}/ocs/v2.php/apps/webhook_listeners/api/v1/webhooks/{}",
        base_url, external_id
    );
    let _ = HTTP_CLIENT
        .delete(&url)
        .bearer_auth(access_token)
        .header("OCS-APIRequest", "true")
        .send()
        .await;
}

/// Delete all native triggers for a workspace+service, including remote webhook cleanup.
/// This is best-effort: errors during remote cleanup or token deletion are logged but ignored.
#[cfg(feature = "native_trigger")]
async fn delete_triggers_for_service(db: &DB, workspace_id: &str, service_name: ServiceName) {
    let triggers = sqlx::query!(
        "SELECT external_id, webhook_token_prefix FROM native_trigger WHERE workspace_id = $1 AND service_name = $2",
        workspace_id,
        service_name as ServiceName
    )
    .fetch_all(db)
    .await;

    let triggers = match triggers {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to fetch native triggers for service {service_name:?} in workspace {workspace_id}: {e}");
            return;
        }
    };

    if triggers.is_empty() {
        return;
    }

    // For Nextcloud: try to delete webhooks on the remote instance (best-effort)
    if service_name == ServiceName::Nextcloud {
        if let Ok(oauth_data) =
            decrypt_oauth_data::<BasicOAuthData>(db, workspace_id, service_name).await
        {
            for trigger in &triggers {
                try_delete_nextcloud_webhook(
                    &oauth_data.base_url,
                    &oauth_data.access_token,
                    &trigger.external_id,
                )
                .await;
            }
        }
    }
    // For Google: skip remote cleanup (watch channels expire naturally)

    // Bulk delete all triggers
    if let Err(e) = sqlx::query!(
        "DELETE FROM native_trigger WHERE workspace_id = $1 AND service_name = $2",
        workspace_id,
        service_name as ServiceName
    )
    .execute(db)
    .await
    {
        tracing::error!("Failed to delete native triggers for service {service_name:?} in workspace {workspace_id}: {e}");
    }

    // Delete all associated webhook tokens
    for trigger in &triggers {
        if let Err(e) = delete_token_by_prefix(db, &trigger.webhook_token_prefix).await {
            tracing::error!(
                "Failed to delete webhook token with prefix {}: {e}",
                trigger.webhook_token_prefix
            );
        }
    }
}

#[cfg(feature = "native_trigger")]
async fn delete_integration(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name)): Path<(String, ServiceName)>,
) -> JsonResult<String> {
    require_admin(authed.is_admin, &workspace_id)?;

    // Delete triggers first (needs OAuth data that cleanup_oauth_resource will remove)
    delete_triggers_for_service(&db, &workspace_id, service_name).await;

    let mut tx = user_db.begin(&authed).await?;

    // Clean up account+variable+resource
    cleanup_oauth_resource(&mut *tx, &workspace_id, service_name).await;

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
    resource_path: Option<String>,
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
            oauth_data as "oauth_data: sqlx::types::Json<WorkspaceOAuthConfig>",
            service_name as "service_name!: ServiceName",
            resource_path
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
        .map(|integration| {
            (
                integration.service_name,
                (integration.oauth_data, integration.resource_path),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();

    use strum::IntoEnumIterator;
    let integrations = ServiceName::iter()
        .map(|service_name| {
            let (oauth_data, resource_path) = key_value
                .get(&service_name)
                .cloned()
                .map(|(od, rp)| (od, rp))
                .unwrap_or((None, None));
            WorkspaceIntegrations { service_name, oauth_data, resource_path }
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
            FROM workspace_integrations wi
            WHERE wi.workspace_id = $1
            AND wi.service_name = $2
            AND wi.oauth_data IS NOT NULL
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
#[derive(Debug, Deserialize)]
struct OAuthCallbackBody {
    redirect_uri: String,
    code: String,
    state: String,
    resource_path: Option<String>,
}

#[cfg(feature = "native_trigger")]
async fn oauth_callback(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name)): Path<(String, ServiceName)>,
    Json(body): Json<OAuthCallbackBody>,
) -> JsonResult<String> {
    require_admin(authed.is_admin, &workspace_id)?;

    let state_was_valid = validate_signed_state(&db, &body.state, &workspace_id).await?;

    if !state_was_valid {
        return Err(Error::BadRequest(
            "Invalid or expired state parameter".to_string(),
        ));
    }

    // Check if this integration uses instance-shared credentials
    let existing_oauth_data = sqlx::query_scalar!(
        r#"SELECT oauth_data FROM workspace_integrations
           WHERE workspace_id = $1 AND service_name = $2"#,
        workspace_id,
        service_name as ServiceName
    )
    .fetch_optional(&db)
    .await?
    .flatten();

    let is_instance_shared = existing_oauth_data
        .as_ref()
        .and_then(|v| v.get("instance_shared"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let oauth_config = if is_instance_shared {
        get_instance_oauth_config(&db, service_name).await?
    } else {
        get_workspace_oauth_config::<WorkspaceOAuthConfig>(&db, &workspace_id, service_name).await?
    };

    let token_response =
        exchange_code_for_token(&oauth_config, service_name, &body.code, &body.redirect_uri)
            .await?;

    let resource_path = body
        .resource_path
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| {
            format!(
                "u/{}/native_{}",
                authed.username,
                service_name.resource_type()
            )
        });

    let expires_in = token_response.expires_in.unwrap_or(3600);

    let mut tx = user_db.begin(&authed).await?;

    // Clean up any previous account+variable+resource for this integration
    cleanup_oauth_resource(&mut *tx, &workspace_id, service_name).await;

    // 1. Create account record for token refresh
    let account_id = sqlx::query_scalar!(
        "INSERT INTO account (workspace_id, client, expires_at, refresh_token, is_workspace_integration)
         VALUES ($1, $2, now() + ($3 || ' seconds')::interval, $4, true)
         RETURNING id",
        workspace_id,
        service_name.as_str(),
        expires_in.to_string(),
        token_response.refresh_token.as_deref().unwrap_or(""),
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::InternalErr(format!("Failed to create account: {}", e)))?;

    // 2. Create variable with encrypted access token
    let mc = build_crypt(&db, &workspace_id).await?;
    let encrypted_access_token = encrypt(&mc, &token_response.access_token);

    sqlx::query!(
        "INSERT INTO variable (workspace_id, path, value, is_secret, description, account, is_oauth)
         VALUES ($1, $2, $3, true, $4, $5, true)
         ON CONFLICT (workspace_id, path) DO UPDATE
         SET value = EXCLUDED.value, account = EXCLUDED.account",
        workspace_id,
        resource_path,
        encrypted_access_token,
        format!("OAuth token for {} workspace integration", service_name),
        account_id,
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::InternalErr(format!("Failed to create variable: {}", e)))?;

    // 3. Create resource pointing to the variable
    let resource_value = json!({ "token": format!("$var:{}", resource_path) });

    sqlx::query!(
        "INSERT INTO resource (workspace_id, path, value, resource_type, description, created_by)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (workspace_id, path) DO UPDATE
         SET value = EXCLUDED.value, resource_type = EXCLUDED.resource_type",
        workspace_id,
        resource_path,
        resource_value,
        service_name.resource_type(),
        format!("{} workspace integration", service_name),
        authed.username,
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::InternalErr(format!("Failed to create resource: {}", e)))?;

    // 4. Store config + resource_path in workspace_integrations (no tokens).
    //    For instance-shared integrations, store the flag instead of credentials.
    let stored_data = if is_instance_shared {
        json!({
            "instance_shared": true,
            "base_url": "",
        })
    } else {
        to_value(&oauth_config).unwrap()
    };
    store_workspace_integration(
        &mut *tx,
        &authed,
        &workspace_id,
        service_name,
        stored_data,
        Some(&resource_path),
    )
    .await?;

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
    let auth_url = Url::parse(&resolve_endpoint(
        &config.base_url,
        service_name.auth_endpoint(),
    ))
    .map_err(|e| Error::InternalErr(format!("Invalid auth URL: {}", e)))?;
    let token_url = Url::parse(&resolve_endpoint(
        &config.base_url,
        service_name.token_endpoint(),
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
    let oauth_data = sqlx::query_scalar!(
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
    .flatten()
    .ok_or(Error::NotFound(format!(
        "Integration for service {} not found",
        service_name.as_str()
    )))?;

    serde_json::from_value::<T>(oauth_data)
        .map_err(|e| Error::InternalErr(format!("Failed to parse OAuth config: {}", e)))
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
        None,
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
    service_name: ServiceName,
    state: &str,
    redirect_uri: &str,
) -> String {
    let base_auth_url = resolve_endpoint(&config.base_url, service_name.auth_endpoint());

    let mut params = vec![
        ("response_type", "code"),
        ("client_id", config.client_id.as_str()),
        ("redirect_uri", redirect_uri),
        ("state", state),
        ("scope", service_name.oauth_scopes()),
    ];

    for &(key, value) in service_name.extra_auth_params() {
        params.push((key, value));
    }

    let query_string = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    format!("{}?{}", base_auth_url, query_string)
}

#[cfg(feature = "native_trigger")]
pub async fn cleanup_oauth_resource(
    tx: &mut sqlx::PgConnection,
    workspace_id: &str,
    service_name: ServiceName,
) {
    // Look up the stored resource_path from workspace_integrations
    let stored_resource_path: Option<String> = sqlx::query_scalar!(
        r#"SELECT resource_path FROM workspace_integrations WHERE workspace_id = $1 AND service_name = $2"#,
        workspace_id,
        service_name as ServiceName,
    )
    .fetch_optional(&mut *tx)
    .await
    .ok()
    .flatten()
    .flatten();

    // Find and delete any existing account+variable+resource for this integration
    let account_ids: Vec<i32> = sqlx::query_scalar!(
        "DELETE FROM account WHERE workspace_id = $1 AND client = $2 AND is_workspace_integration = true RETURNING id",
        workspace_id,
        service_name.as_str(),
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap_or_default();

    if !account_ids.is_empty() {
        // Delete variables linked to these accounts
        let _ = sqlx::query!(
            "DELETE FROM variable WHERE workspace_id = $1 AND account = ANY($2)",
            workspace_id,
            &account_ids,
        )
        .execute(&mut *tx)
        .await;
    }

    // Delete resource by exact stored path, or fall back to legacy pattern
    let resource_type = service_name.resource_type();
    if let Some(ref path) = stored_resource_path {
        let _ = sqlx::query!(
            "DELETE FROM resource WHERE workspace_id = $1 AND path = $2",
            workspace_id,
            path,
        )
        .execute(&mut *tx)
        .await;
    } else {
        // Legacy fallback for integrations created before user-chosen paths
        let _ = sqlx::query!(
            "DELETE FROM resource WHERE workspace_id = $1 AND resource_type = $2 AND path LIKE 'u/%/native_%'",
            workspace_id,
            resource_type,
        )
        .execute(&mut *tx)
        .await;
    }
}

/// Check if the instance admin has enabled sharing of OAuth credentials for a given service.
/// Currently only supported for Google (gworkspace).
#[cfg(feature = "native_trigger")]
async fn is_instance_sharing_enabled(db: &DB, service_name: ServiceName) -> Result<bool> {
    // Only Google supports instance sharing for now
    if service_name != ServiceName::Google {
        return Ok(false);
    }

    let oauths_value = match load_value_from_global_settings(db, OAUTH_SETTING).await? {
        Some(v) => v,
        None => return Ok(false),
    };

    let key = service_name.resource_type(); // "gworkspace"
    let entry = match oauths_value.get(key) {
        Some(v) => v,
        None => return Ok(false),
    };

    let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let secret = entry.get("secret").and_then(|v| v.as_str()).unwrap_or("");
    let share = entry
        .get("share_with_workspaces")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(!id.is_empty() && !secret.is_empty() && share)
}

/// Read instance-level OAuth credentials for a service (when sharing is enabled).
#[cfg(feature = "native_trigger")]
async fn get_instance_oauth_config(
    db: &DB,
    service_name: ServiceName,
) -> Result<WorkspaceOAuthConfig> {
    if !is_instance_sharing_enabled(db, service_name).await? {
        return Err(Error::BadRequest(
            "Instance credential sharing is not enabled for this service".to_string(),
        ));
    }

    let (client_id, client_secret) =
        windmill_common::global_settings::get_instance_oauth_credentials(
            db,
            service_name.resource_type(),
        )
        .await?;

    Ok(WorkspaceOAuthConfig {
        client_id,
        client_secret,
        base_url: String::new(), // Google uses absolute URLs
        instance_shared: false,
    })
}

#[cfg(feature = "native_trigger")]
async fn check_instance_sharing_available(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((workspace_id, service_name)): Path<(String, ServiceName)>,
) -> JsonResult<bool> {
    require_admin(authed.is_admin, &workspace_id)?;
    let available = is_instance_sharing_enabled(&db, service_name).await?;
    Ok(Json(available))
}

#[cfg(feature = "native_trigger")]
async fn generate_instance_connect_url(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name)): Path<(String, ServiceName)>,
    Json(RedirectUri { redirect_uri }): Json<RedirectUri>,
) -> JsonResult<String> {
    require_admin(authed.is_admin, &workspace_id)?;

    let instance_config = get_instance_oauth_config(&db, service_name).await?;

    // Store a marker in workspace_integrations — NOT the actual credentials.
    // The callback and token refresh will read credentials from global settings
    // when they see instance_shared=true.
    let mut tx = user_db.begin(&authed).await?;
    crate::store_workspace_integration(
        &mut tx,
        &authed,
        &workspace_id,
        service_name,
        json!({ "instance_shared": true, "base_url": "" }),
        None,
    )
    .await?;
    tx.commit().await?;

    // Generate signed state and build authorization URL
    let state = generate_signed_state(&db, &workspace_id, service_name).await?;
    let auth_url = build_authorization_url(&instance_config, service_name, &state, &redirect_uri);
    Ok(Json(auth_url))
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
        .route(
            "/:service_name/instance_sharing_available",
            get(check_instance_sharing_available),
        )
        .route(
            "/:service_name/generate_instance_connect_url",
            post(generate_instance_connect_url),
        )
        .route("/:service_name/delete", delete(delete_integration))
        .route("/:service_name/callback", post(oauth_callback));

    Router::new().nest("/integrations", router)
}

#[cfg(not(feature = "native_trigger"))]
pub fn workspaced_service() -> Router {
    Router::new()
}
