use axum::Router;

#[cfg(feature = "native_trigger")]
use axum::{
    extract::{Path, Query},
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
use windmill_api_auth::{check_scopes, require_is_writer, ApiAuthed};

#[cfg(feature = "native_trigger")]
use crate::{
    decrypt_oauth_data, delete_token_by_hash, delete_workspace_integration,
    list_usable_connections, nextcloud::OcsResponse, require_native_integration_use,
    resolve_endpoint, Connection, ServiceName,
};

#[cfg(feature = "native_trigger")]
use std::collections::HashMap;

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
/// The state contains: workspace_id, service_name, timestamp, nonce, the path the connection
/// will be saved at, and the user it was issued to. The path makes the callback independent of the
/// page the provider redirects to; the user stops anyone else from finishing the flow, which would
/// save their account at a path the issuer chose.
/// It's signed with HMAC-SHA256 using the workspace key.
#[cfg(feature = "native_trigger")]
async fn generate_signed_state(
    db: &DB,
    workspace_id: &str,
    service_name: ServiceName,
    connection_path: &str,
    username: &str,
) -> Result<String> {
    use windmill_common::variables::get_workspace_key;

    let nonce = uuid::Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().timestamp();
    let payload = format!(
        "{}:{}:{}:{}:{}:{}",
        workspace_id,
        service_name.as_str(),
        timestamp,
        nonce,
        URL_SAFE_NO_PAD.encode(connection_path),
        URL_SAFE_NO_PAD.encode(username)
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

/// Validate a signed OAuth state (correct signature, workspace, service and user, not expired)
/// and return the connection path it was issued for.
#[cfg(feature = "native_trigger")]
async fn validate_signed_state(
    db: &DB,
    state: &str,
    workspace_id: &str,
    service_name: ServiceName,
    username: &str,
) -> Result<String> {
    use windmill_common::variables::get_workspace_key;

    let invalid = || Error::BadRequest("Invalid or expired state parameter".to_string());

    let (encoded_payload, encoded_signature) = state.split_once(':').ok_or_else(invalid)?;

    let payload = URL_SAFE_NO_PAD
        .decode(encoded_payload)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or_else(invalid)?;

    // Parse payload: workspace_id:service_name:timestamp:nonce:connection_path:username
    let [state_workspace_id, state_service, timestamp, _nonce, encoded_path, encoded_user] =
        payload
            .split(':')
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| invalid())?;

    if state_workspace_id != workspace_id
        || state_service != service_name.as_str()
        || encoded_user != URL_SAFE_NO_PAD.encode(username)
    {
        return Err(invalid());
    }

    let timestamp: i64 = timestamp.parse().map_err(|_| invalid())?;
    if chrono::Utc::now().timestamp() - timestamp > STATE_EXPIRATION_SECONDS {
        return Err(invalid());
    }

    let key = get_workspace_key(workspace_id, db).await?;
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .map_err(|e| Error::InternalErr(e.to_string()))?;
    mac.update(payload.as_bytes());
    let received_signature = URL_SAFE_NO_PAD
        .decode(encoded_signature)
        .map_err(|_| invalid())?;
    mac.verify_slice(&received_signature)
        .map_err(|_| invalid())?;

    URL_SAFE_NO_PAD
        .decode(encoded_path)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or_else(invalid)
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

/// The OAuth client a connection is made with: the workspace's own when an admin configured one,
/// otherwise the instance's when the instance admin shares it. The flag says which.
#[cfg(feature = "native_trigger")]
async fn resolve_oauth_client(
    db: &DB,
    workspace_id: &str,
    service_name: ServiceName,
) -> Result<(WorkspaceOAuthConfig, bool)> {
    let workspace_config =
        get_workspace_oauth_config::<WorkspaceOAuthConfig>(db, workspace_id, service_name)
            .await
            .ok()
            .filter(|c| !c.instance_shared && !c.client_id.is_empty());
    match workspace_config {
        Some(config) => Ok((config, false)),
        None => Ok((get_instance_oauth_config(db, service_name).await?, true)),
    }
}

#[cfg(feature = "native_trigger")]
fn default_connection_path(authed: &ApiAuthed, service_name: ServiceName) -> String {
    format!(
        "u/{}/native_{}",
        authed.username,
        service_name.resource_type()
    )
}

#[cfg(feature = "native_trigger")]
async fn generate_connect_url(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((workspace_id, service_name)): Path<(String, ServiceName)>,
    Json(body): Json<ConnectUrlBody>,
) -> JsonResult<String> {
    require_native_integration_use(&authed)?;

    let (oauth_config, is_instance_shared) =
        resolve_oauth_client(&db, &workspace_id, service_name).await?;

    let connection_path = body
        .resource_path
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| default_connection_path(&authed, service_name));

    // A GitHub OAuth app only redirects under its one registered callback, and the instance app's
    // is the resource connect callback, so its connections land on a page below that one.
    let redirect_uri = if is_instance_shared && service_name == ServiceName::Github {
        let origin = Url::parse(&body.redirect_uri)
            .map_err(|e| Error::BadRequest(format!("Invalid redirect URI: {e}")))?
            .origin()
            .ascii_serialization();
        format!("{origin}/oauth/callback/github/native_trigger")
    } else {
        body.redirect_uri
    };

    // Generate a signed state that is cluster-safe
    let state = generate_signed_state(
        &db,
        &workspace_id,
        service_name,
        &connection_path,
        &authed.username,
    )
    .await?;
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

#[cfg(feature = "native_trigger")]
async fn try_delete_github_webhook(
    db: &DB,
    workspace_id: &str,
    access_token: &str,
    external_id: &str,
) {
    // Fetch owner/repo from service_config
    let config = sqlx::query_scalar!(
        "SELECT service_config FROM native_trigger WHERE external_id = $1 AND service_name = $2 AND workspace_id = $3",
        external_id,
        ServiceName::Github as ServiceName,
        workspace_id
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .flatten();

    if let Some(config) = config {
        let owner = config.get("owner").and_then(|v| v.as_str()).unwrap_or("");
        let repo = config.get("repo").and_then(|v| v.as_str()).unwrap_or("");
        if !owner.is_empty() && !repo.is_empty() {
            let url = format!(
                "https://api.github.com/repos/{}/{}/hooks/{}",
                owner, repo, external_id
            );
            let _ = HTTP_CLIENT
                .delete(&url)
                .bearer_auth(access_token)
                .header("Accept", "application/json")
                .header("User-Agent", "Windmill")
                .send()
                .await;
        }
    }
}

#[cfg(feature = "native_trigger")]
async fn fetch_nextcloud_user_id(base_url: &str, access_token: &str) -> anyhow::Result<String> {
    let url = format!("{}/ocs/v2.php/cloud/user", base_url);
    let resp = HTTP_CLIENT
        .get(&url)
        .bearer_auth(access_token)
        .header("OCS-APIRequest", "true")
        .header("Accept", "application/json")
        .send()
        .await?
        .error_for_status()?;
    let ocs: OcsResponse<NextcloudUserData> = resp.json().await?;
    Ok(ocs.ocs.data.id)
}

/// Delete the native triggers of a workspace+service, including remote webhook cleanup. When `only`
/// gives `(external_ids, script_paths)`, just the triggers still at those paths: a trigger moved
/// since its path was checked is left alone.
/// This is best-effort: errors during remote cleanup or token deletion are logged but ignored.
#[cfg(feature = "native_trigger")]
async fn delete_triggers_for_service(
    db: &DB,
    workspace_id: &str,
    service_name: ServiceName,
    only: Option<(&[String], &[String])>,
) {
    let (external_ids, script_paths) = only.unzip();
    let triggers = sqlx::query!(
        "SELECT external_id, connection_path FROM native_trigger
         WHERE workspace_id = $1 AND service_name = $2 AND ($3::text[] IS NULL
              OR (external_id, script_path) IN (SELECT * FROM unnest($3::text[], $4::text[])))",
        workspace_id,
        service_name as ServiceName,
        external_ids,
        script_paths,
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

    // Nextcloud and GitHub: try to delete the webhooks remotely (best-effort).
    // Google: skip remote cleanup (watch channels expire naturally).
    if service_name != ServiceName::Google {
        let mut credentials: HashMap<Option<String>, Option<BasicOAuthData>> = HashMap::new();
        for trigger in &triggers {
            if !credentials.contains_key(&trigger.connection_path) {
                let oauth_data = decrypt_oauth_data::<BasicOAuthData>(
                    db,
                    workspace_id,
                    service_name,
                    trigger.connection_path.as_deref(),
                )
                .await
                .ok();
                credentials.insert(trigger.connection_path.clone(), oauth_data);
            }
            let Some(Some(oauth_data)) = credentials.get(&trigger.connection_path) else {
                continue;
            };
            if service_name == ServiceName::Nextcloud {
                try_delete_nextcloud_webhook(
                    &oauth_data.base_url,
                    &oauth_data.access_token,
                    &trigger.external_id,
                )
                .await;
            } else {
                try_delete_github_webhook(
                    db,
                    workspace_id,
                    &oauth_data.access_token,
                    &trigger.external_id,
                )
                .await;
            }
        }
    }

    // Revoke the tokens the deleted rows actually named, not the ones listed above: a
    // re-registration running concurrently mints a replacement, and leaving that alive would let a
    // disconnected integration keep starting jobs.
    let deleted_token_hashes = sqlx::query_scalar!(
        "DELETE FROM native_trigger
         WHERE workspace_id = $1 AND service_name = $2 AND ($3::text[] IS NULL
              OR (external_id, script_path) IN (SELECT * FROM unnest($3::text[], $4::text[])))
         RETURNING webhook_token_hash",
        workspace_id,
        service_name as ServiceName,
        external_ids,
        script_paths,
    )
    .fetch_all(db)
    .await
    .unwrap_or_else(|e| {
        tracing::error!("Failed to delete native triggers for service {service_name:?} in workspace {workspace_id}: {e}");
        Vec::new()
    });

    for webhook_token_hash in &deleted_token_hashes {
        if let Err(e) = delete_token_by_hash(db, webhook_token_hash).await {
            tracing::error!(
                "Failed to delete webhook token with hash {}: {e}",
                webhook_token_hash
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

    // Delete triggers first: removing their registrations needs the connections cleaned up below
    delete_triggers_for_service(&db, &workspace_id, service_name, None).await;

    let mut tx = user_db.begin(&authed).await?;

    cleanup_service_connections(&mut *tx, &workspace_id, service_name).await?;

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
            oauth_data as "oauth_data: sqlx::types::Json<WorkspaceOAuthConfig>",
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
        .map(|service_name| {
            let oauth_data = key_value.get(&service_name).cloned().flatten();
            WorkspaceIntegrations { service_name, oauth_data }
        })
        .collect::<Vec<_>>();

    tx.commit().await?;

    Ok(Json(integrations))
}

/// Whether members can connect an account for this service: the workspace has an OAuth client,
/// or the instance shares one.
#[cfg(feature = "native_trigger")]
async fn integration_exist(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name)): Path<(String, ServiceName)>,
) -> JsonResult<bool> {
    if is_instance_sharing_enabled(&db, service_name).await? {
        return Ok(Json(true));
    }
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
struct ConnectUrlBody {
    redirect_uri: String,
    /// Where to save the connection; defaults to `u/<caller>/native_<resource type>`.
    resource_path: Option<String>,
}

#[cfg(feature = "native_trigger")]
#[derive(Debug, Deserialize)]
struct OAuthCallbackBody {
    redirect_uri: String,
    code: String,
    state: String,
}

#[cfg(feature = "native_trigger")]
#[derive(Debug, Deserialize)]
struct NextcloudUserData {
    id: String,
}

#[cfg(feature = "native_trigger")]
async fn oauth_callback(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name)): Path<(String, ServiceName)>,
    Json(body): Json<OAuthCallbackBody>,
) -> JsonResult<String> {
    require_native_integration_use(&authed)?;

    let resource_path = validate_signed_state(
        &db,
        &body.state,
        &workspace_id,
        service_name,
        &authed.username,
    )
    .await?;

    let (oauth_config, is_instance_shared) =
        resolve_oauth_client(&db, &workspace_id, service_name).await?;

    let token_response =
        exchange_code_for_token(&oauth_config, service_name, &body.code, &body.redirect_uri)
            .await?;

    let expires_in = token_response.expires_in.unwrap_or(3600);

    // A user transaction, so the variable and resource policies decide whether the caller may
    // write a connection at this path.
    let mut tx = user_db.begin(&authed).await?;

    // Reconnecting at a path replaces the connection there; triggers using it keep working.
    let replaced_account = cleanup_connection(&mut *tx, &workspace_id, &resource_path).await?;

    // 1. Create account record for token refresh
    let account_id = sqlx::query_scalar!(
        "INSERT INTO account (workspace_id, client, expires_at, refresh_token, is_workspace_integration, created_by)
         VALUES ($1, $2, now() + ($3 || ' seconds')::interval, $4, true, $5)
         RETURNING id",
        workspace_id,
        service_name.as_str(),
        expires_in.to_string(),
        token_response.refresh_token.as_deref().unwrap_or(""),
        authed.username,
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
    let resource_value = if service_name == ServiceName::Nextcloud {
        let token_value = format!("$var:{}", resource_path);
        let base_url = &oauth_config.base_url;
        let user_id = fetch_nextcloud_user_id(base_url, &token_response.access_token).await;
        match user_id {
            Ok(user_id) => json!({
                "token": token_value,
                "baseUrl": base_url,
                "userId": user_id,
            }),
            Err(e) => {
                tracing::warn!("Failed to fetch Nextcloud user info: {e}");
                json!({
                    "token": token_value,
                    "baseUrl": base_url,
                })
            }
        }
    } else {
        json!({ "token": format!("$var:{}", resource_path) })
    };

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

    // 4. The first connection made with the instance's client records that in the workspace.
    //    Token refresh reads the client from this row, so it must exist for every connection.
    if is_instance_shared {
        sqlx::query!(
            "INSERT INTO workspace_integrations (workspace_id, service_name, oauth_data, created_by)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (workspace_id, service_name) DO NOTHING",
            workspace_id,
            service_name as ServiceName,
            json!({ "instance_shared": true, "base_url": "" }),
            authed.username,
        )
        .execute(&mut *tx)
        .await?;
    }

    audit_log(
        &mut *tx,
        &authed,
        &format!("workspace_integrations.{}.connect", service_name),
        ActionKind::Create,
        &workspace_id,
        Some(&resource_path),
        None,
    )
    .await?;

    tx.commit().await?;
    delete_unlinked_account(&db, &workspace_id, replaced_account).await;

    Ok(Json(resource_path))
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
    )
    .await?;

    tx.commit().await?;

    Ok(())
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

/// Remove the connection at `path`: its variable and its resource.
///
/// Returns the account the variable linked. The caller removes it with
/// [`delete_unlinked_account`] once `tx` is committed.
#[cfg(feature = "native_trigger")]
pub async fn cleanup_connection(
    tx: &mut sqlx::PgConnection,
    workspace_id: &str,
    path: &str,
) -> Result<Option<i32>> {
    let account_id = sqlx::query_scalar!(
        "DELETE FROM variable WHERE workspace_id = $1 AND path = $2 RETURNING account",
        workspace_id,
        path,
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();

    sqlx::query!(
        "DELETE FROM resource WHERE workspace_id = $1 AND path = $2",
        workspace_id,
        path,
    )
    .execute(&mut *tx)
    .await?;

    Ok(account_id)
}

/// Delete a connection's account once no variable links it any more. Any variable can link any
/// account, so another one may still hold it; the check needs `db`, since the caller's own
/// transaction does not see variables it cannot read.
#[cfg(feature = "native_trigger")]
pub async fn delete_unlinked_account(db: &DB, workspace_id: &str, account_id: Option<i32>) {
    let Some(account_id) = account_id else {
        return;
    };
    if let Err(e) = sqlx::query!(
        "DELETE FROM account WHERE workspace_id = $1 AND id = $2 AND is_workspace_integration = true
         AND NOT EXISTS (SELECT 1 FROM variable WHERE workspace_id = $1 AND account = $2)",
        workspace_id,
        account_id,
    )
    .execute(db)
    .await
    {
        tracing::error!("Failed to delete account {account_id} in {workspace_id}: {e}");
    }
}

/// Remove every connection of a service in the workspace.
#[cfg(feature = "native_trigger")]
pub async fn cleanup_service_connections(
    tx: &mut sqlx::PgConnection,
    workspace_id: &str,
    service_name: ServiceName,
) -> Result<()> {
    let account_ids = sqlx::query_scalar!(
        "DELETE FROM account WHERE workspace_id = $1 AND client = $2 AND is_workspace_integration = true RETURNING id",
        workspace_id,
        service_name.as_str(),
    )
    .fetch_all(&mut *tx)
    .await?;

    let paths = sqlx::query_scalar!(
        "DELETE FROM variable WHERE workspace_id = $1 AND account = ANY($2) RETURNING path",
        workspace_id,
        &account_ids,
    )
    .fetch_all(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM resource WHERE workspace_id = $1 AND path = ANY($2)",
        workspace_id,
        &paths,
    )
    .execute(&mut *tx)
    .await?;

    Ok(())
}

/// Check if the instance admin has enabled sharing of OAuth credentials for a given service.
#[cfg(feature = "native_trigger")]
async fn is_instance_sharing_enabled(db: &DB, service_name: ServiceName) -> Result<bool> {
    // Each Nextcloud client belongs to one Nextcloud server, which is a workspace choice.
    if service_name == ServiceName::Nextcloud {
        return Ok(false);
    }

    let oauths_value = match load_value_from_global_settings(db, OAUTH_SETTING).await? {
        Some(v) => v,
        None => return Ok(false),
    };

    let key = service_name.resource_type();
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
        return Err(Error::BadRequest(format!(
            "No {service_name} OAuth app is configured for this workspace. A workspace admin can \
             configure one in the workspace settings, under native triggers."
        )));
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
async fn list_connections(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name)): Path<(String, ServiceName)>,
) -> JsonResult<Vec<Connection>> {
    let mut tx = user_db.begin(&authed).await?;
    let connections = list_usable_connections(&mut tx, &workspace_id, service_name).await?;
    tx.commit().await?;
    Ok(Json(connections))
}

#[cfg(feature = "native_trigger")]
#[derive(Debug, Deserialize)]
struct ConnectionPathQuery {
    path: String,
}

/// Disconnect one account: its triggers are deleted with it, since nothing else can act on
/// their registrations.
#[cfg(feature = "native_trigger")]
async fn delete_connection(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((workspace_id, service_name)): Path<(String, ServiceName)>,
    Query(ConnectionPathQuery { path }): Query<ConnectionPathQuery>,
) -> JsonResult<String> {
    check_scopes(&authed, || format!("variables:write:{path}"))?;
    require_is_writer(
        &authed,
        &path,
        &workspace_id,
        db.clone(),
        "SELECT extra_perms FROM variable WHERE path = $1 AND workspace_id = $2",
        "variable",
    )
    .await?;

    let mut tx = user_db.clone().begin(&authed).await?;
    let usable = list_usable_connections(&mut tx, &workspace_id, service_name).await?;
    tx.commit().await?;
    if !usable.iter().any(|c| c.path == path) {
        return Err(Error::NotFound(format!(
            "No {service_name} connection at {path}"
        )));
    }

    // Disconnecting deletes these triggers, so a path-scoped token must cover each of them. Only
    // the rows checked here, at the paths checked, are deleted: one created or moved meanwhile is
    // never removed unchecked.
    let triggers = sqlx::query!(
        "SELECT external_id, script_path FROM native_trigger
         WHERE workspace_id = $1 AND service_name = $2 AND connection_path = $3",
        workspace_id,
        service_name as ServiceName,
        path,
    )
    .fetch_all(&db)
    .await?;
    for trigger in &triggers {
        check_scopes(&authed, || {
            format!("native_triggers:write:{}", trigger.script_path)
        })?;
    }
    let (external_ids, script_paths): (Vec<String>, Vec<String>) = triggers
        .into_iter()
        .map(|t| (t.external_id, t.script_path))
        .unzip();

    // Before the connection goes: removing the registrations needs its token.
    delete_triggers_for_service(
        &db,
        &workspace_id,
        service_name,
        Some((&external_ids, &script_paths)),
    )
    .await;

    let mut tx = user_db.begin(&authed).await?;
    let account = cleanup_connection(&mut *tx, &workspace_id, &path).await?;

    audit_log(
        &mut *tx,
        &authed,
        &format!("workspace_integrations.{}.disconnect", service_name),
        ActionKind::Delete,
        &workspace_id,
        Some(&path),
        None,
    )
    .await?;

    tx.commit().await?;
    delete_unlinked_account(&db, &workspace_id, account).await;

    Ok(Json(format!("Disconnected {path}")))
}

#[cfg(feature = "native_trigger")]
pub fn workspaced_service() -> Router {
    let router = Router::new()
        .route("/list", get(list_integrations))
        .route("/{service_name}/exists", get(integration_exist))
        .route("/{service_name}/create", post(create_workspace_integration))
        .route(
            "/{service_name}/generate_connect_url",
            post(generate_connect_url),
        )
        .route(
            "/{service_name}/instance_sharing_available",
            get(check_instance_sharing_available),
        )
        .route("/{service_name}/connections", get(list_connections))
        .route(
            "/{service_name}/connections/delete",
            delete(delete_connection),
        )
        .route("/{service_name}/delete", delete(delete_integration))
        .route("/{service_name}/callback", post(oauth_callback));

    Router::new().nest("/integrations", router)
}

#[cfg(not(feature = "native_trigger"))]
pub fn workspaced_service() -> Router {
    Router::new()
}
