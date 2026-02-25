//! OAuth 2.0 Authorization Server for MCP (RFC 6749, 7591, 7636, 8414, 9728)

use axum::{
    extract::{Extension, Path, Query},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Form, Json, Router,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;
use windmill_common::{
    error::{Error, Result},
    utils::rd_string,
    BASE_URL, DB,
};

use crate::db::ApiAuthed;

/// Token expiration for MCP OAuth tokens (1 week in seconds)
const MCP_OAUTH_TOKEN_EXPIRATION_SECS: u64 = 7 * 24 * 60 * 60;

/// Refresh token expiration for MCP OAuth (30 days in seconds)
const MCP_OAUTH_REFRESH_TOKEN_EXPIRATION_SECS: u64 = 30 * 24 * 60 * 60;

/// RFC 8414
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes_supported: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_types_supported: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_types_supported: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_challenge_methods_supported: Option<Vec<String>>,
}

/// RFC 9728
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedResourceMetadata {
    pub resource: String,
    pub authorization_servers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes_supported: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer_methods_supported: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct OAuthJsonError {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
}

impl OAuthJsonError {
    fn new(error: &str, description: Option<&str>) -> Self {
        Self { error: error.to_string(), error_description: description.map(|s| s.to_string()) }
    }
}

impl IntoResponse for OAuthJsonError {
    fn into_response(self) -> axum::response::Response {
        (axum::http::StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}

#[derive(Debug, Serialize)]
pub struct OAuthTokenError {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
}

impl OAuthTokenError {
    fn invalid_request(description: &str) -> Self {
        Self {
            error: "invalid_request".to_string(),
            error_description: Some(description.to_string()),
        }
    }

    fn invalid_grant(description: &str) -> Self {
        Self {
            error: "invalid_grant".to_string(),
            error_description: Some(description.to_string()),
        }
    }

    fn unsupported_grant_type(description: &str) -> Self {
        Self {
            error: "unsupported_grant_type".to_string(),
            error_description: Some(description.to_string()),
        }
    }

    fn server_error(description: &str) -> Self {
        Self { error: "server_error".to_string(), error_description: Some(description.to_string()) }
    }
}

impl IntoResponse for OAuthTokenError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.error.as_str() {
            "server_error" => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            _ => axum::http::StatusCode::BAD_REQUEST,
        };
        (status, Json(self)).into_response()
    }
}

#[derive(Debug, Deserialize)]
pub struct ClientRegistrationRequest {
    pub client_name: String,
    pub redirect_uris: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ClientRegistrationResponse {
    pub client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeQuery {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub code_challenge: Option<String>,
    #[serde(default)]
    pub code_challenge_method: Option<String>,
    #[serde(default)]
    pub resource: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApprovalForm {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub redirect_uri: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub code_verifier: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, FromRow)]
struct OAuthClient {
    client_id: String,
    client_name: String,
    redirect_uris: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, FromRow)]
struct AuthorizationCode {
    code: String,
    client_id: String,
    user_email: String,
    workspace_id: String,
    scopes: Vec<String>,
    redirect_uri: String,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, FromRow)]
struct RefreshTokenRow {
    id: i64,
    refresh_token: String,
    access_token: String,
    client_id: String,
    user_email: String,
    workspace_id: String,
    scopes: Vec<String>,
    token_family: sqlx::types::Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: chrono::DateTime<chrono::Utc>,
    used_at: Option<chrono::DateTime<chrono::Utc>>,
    revoked: bool,
}

fn supported_scopes() -> Vec<String> {
    vec![
        "mcp:all".to_string(),
        "mcp:favorites".to_string(),
        "mcp:scripts:*".to_string(),
        "mcp:flows:*".to_string(),
        "mcp:endpoints:*".to_string(),
    ]
}

/// GET /.well-known/oauth-authorization-server/api/w/:workspace_id/mcp/oauth/server
pub async fn workspaced_oauth_metadata(
    Path(workspace_id): Path<String>,
) -> Json<AuthorizationMetadata> {
    let base_url = BASE_URL.read().await;
    let issuer = format!("{}/api/w/{}/mcp/oauth/server", base_url, workspace_id);

    Json(AuthorizationMetadata {
        issuer,
        authorization_endpoint: format!(
            "{}/api/w/{}/mcp/oauth/server/authorize",
            base_url, workspace_id
        ),
        token_endpoint: format!("{}/api/w/{}/mcp/oauth/server/token", base_url, workspace_id),
        registration_endpoint: Some(format!("{}/api/mcp/oauth/server/register", base_url)),
        scopes_supported: Some(supported_scopes()),
        response_types_supported: Some(vec!["code".to_string()]),
        grant_types_supported: Some(vec![
            "authorization_code".to_string(),
            "refresh_token".to_string(),
        ]),
        code_challenge_methods_supported: Some(vec!["S256".to_string()]),
    })
}

/// GET /.well-known/oauth-protected-resource/api/mcp/w/:workspace_id/mcp
pub async fn protected_resource_metadata_by_path(
    Path(workspace_id): Path<String>,
) -> Json<ProtectedResourceMetadata> {
    let base_url = BASE_URL.read().await;
    let resource_url = format!("{}/api/mcp/w/{}/mcp", base_url, workspace_id);
    let auth_server_url = format!("{}/api/w/{}/mcp/oauth/server", base_url, workspace_id);
    Json(ProtectedResourceMetadata {
        resource: resource_url,
        authorization_servers: vec![auth_server_url],
        scopes_supported: Some(supported_scopes()),
        bearer_methods_supported: Some(vec!["header".to_string()]),
    })
}

/// POST /api/mcp/oauth/server/register - dynamic client registration
pub async fn oauth_register(
    Extension(db): Extension<DB>,
    Json(req): Json<ClientRegistrationRequest>,
) -> Result<(axum::http::StatusCode, Json<ClientRegistrationResponse>)> {
    if req.redirect_uris.is_empty() {
        return Err(Error::BadRequest(
            "At least one redirect_uri is required".to_string(),
        ));
    }

    let client_id = format!("mcp-client-{}", rd_string(16));

    sqlx::query!(
        "INSERT INTO mcp_oauth_server_client (client_id, client_name, redirect_uris)
         VALUES ($1, $2, $3)",
        client_id,
        req.client_name,
        &req.redirect_uris,
    )
    .execute(&db)
    .await
    .map_err(|e| Error::InternalErr(format!("Failed to register client: {}", e)))?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(ClientRegistrationResponse {
            client_id,
            client_secret: None,
            client_name: req.client_name,
            redirect_uris: req.redirect_uris,
        }),
    ))
}

#[derive(Debug, Serialize)]
pub struct ApprovalResponse {
    pub code: String,
    pub state: Option<String>,
}

/// POST /api/w/:workspace_id/mcp/oauth/server/token - exchange code for token or refresh
pub async fn oauth_token(
    Extension(db): Extension<DB>,
    Form(req): Form<TokenRequest>,
) -> std::result::Result<Json<TokenResponse>, OAuthTokenError> {
    match req.grant_type.as_str() {
        "authorization_code" => handle_authorization_code_grant(&db, &req).await,
        "refresh_token" => handle_refresh_token_grant(&db, &req).await,
        _ => Err(OAuthTokenError::unsupported_grant_type(
            "Supported grant types: authorization_code, refresh_token",
        )),
    }
}

/// Handle authorization_code grant type
async fn handle_authorization_code_grant(
    db: &DB,
    req: &TokenRequest,
) -> std::result::Result<Json<TokenResponse>, OAuthTokenError> {
    let auth_code = match sqlx::query_as!(
        AuthorizationCode,
        "DELETE FROM mcp_oauth_server_code
         WHERE code = $1 AND expires_at > now()
         RETURNING code, client_id, user_email, workspace_id, scopes, redirect_uri,
                   code_challenge, code_challenge_method",
        req.code
    )
    .fetch_optional(db)
    .await
    {
        Ok(Some(code)) => code,
        Ok(None) => {
            return Err(OAuthTokenError::invalid_grant(
                "Invalid or expired authorization code",
            ));
        }
        Err(e) => {
            tracing::error!("Database error consuming auth code: {}", e);
            return Err(OAuthTokenError::server_error("Database error"));
        }
    };

    if auth_code.client_id != req.client_id {
        return Err(OAuthTokenError::invalid_grant("client_id mismatch"));
    }

    if auth_code.redirect_uri != req.redirect_uri {
        return Err(OAuthTokenError::invalid_grant("redirect_uri mismatch"));
    }

    let challenge = auth_code.code_challenge.as_ref().ok_or_else(|| {
        OAuthTokenError::invalid_grant("Authorization code missing PKCE challenge")
    })?;

    let verifier = req
        .code_verifier
        .as_ref()
        .ok_or_else(|| OAuthTokenError::invalid_request("code_verifier is required"))?;

    let method = auth_code.code_challenge_method.as_deref().unwrap_or("S256");
    if method != "S256" {
        return Err(OAuthTokenError::invalid_grant(
            "Only S256 PKCE method is supported",
        ));
    }

    if !validate_pkce_s256(verifier, challenge) {
        return Err(OAuthTokenError::invalid_grant("Invalid code_verifier"));
    }

    let access_token = rd_string(32);
    let refresh_token = rd_string(32);
    let token_family = sqlx::types::Uuid::new_v4();
    let scopes = auth_code.scopes;

    // Create access token (rejects archived workspaces inline)
    let rows = sqlx::query!(
        "INSERT INTO token (token, email, label, expiration, scopes, workspace_id)
         SELECT $1::varchar, $2::varchar, $3::varchar, now() + ($4 || ' seconds')::interval, $5::text[], $6::varchar
         WHERE NOT EXISTS(SELECT 1 FROM workspace WHERE id = $6 AND deleted = true)",
        access_token,
        auth_code.user_email,
        format!("mcp-oauth-{}", auth_code.client_id),
        MCP_OAUTH_TOKEN_EXPIRATION_SECS.to_string(),
        &scopes,
        auth_code.workspace_id,
    )
    .execute(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create access token: {}", e);
        OAuthTokenError::server_error("Failed to create access token")
    })?;
    if rows.rows_affected() == 0 {
        return Err(OAuthTokenError::invalid_grant(
            "Cannot create a token for an archived workspace",
        ));
    }

    // Create refresh token
    let refresh_token_result = sqlx::query!(
        "INSERT INTO mcp_oauth_refresh_token
         (refresh_token, access_token, client_id, user_email, workspace_id, scopes, token_family, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, now() + ($8 || ' seconds')::interval)",
        refresh_token,
        access_token,
        auth_code.client_id,
        auth_code.user_email,
        auth_code.workspace_id,
        &scopes,
        token_family,
        MCP_OAUTH_REFRESH_TOKEN_EXPIRATION_SECS.to_string(),
    )
    .execute(db)
    .await;

    let refresh_token_value = match refresh_token_result {
        Ok(_) => Some(refresh_token),
        Err(e) => {
            tracing::error!("Failed to create refresh token: {}", e);
            None // Don't include invalid refresh token
        }
    };

    Ok(Json(TokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: MCP_OAUTH_TOKEN_EXPIRATION_SECS,
        scope: Some(scopes.join(" ")),
        refresh_token: refresh_token_value,
    }))
}

/// Handle refresh_token grant type with token rotation and theft detection
async fn handle_refresh_token_grant(
    db: &DB,
    req: &TokenRequest,
) -> std::result::Result<Json<TokenResponse>, OAuthTokenError> {
    let refresh_token_value = req
        .refresh_token
        .as_ref()
        .ok_or_else(|| OAuthTokenError::invalid_request("refresh_token is required"))?;

    if req.client_id.is_empty() {
        return Err(OAuthTokenError::invalid_request("client_id is required"));
    }

    // Atomically claim the refresh token by setting used_at in a single UPDATE.
    let token_row = match sqlx::query_as!(
        RefreshTokenRow,
        "UPDATE mcp_oauth_refresh_token
         SET used_at = now()
         WHERE refresh_token = $1
           AND client_id = $2
           AND used_at IS NULL
           AND NOT revoked
           AND expires_at > now()
         RETURNING id, refresh_token, access_token, client_id, user_email, workspace_id,
                   scopes, token_family, created_at, expires_at, used_at, revoked",
        refresh_token_value,
        req.client_id
    )
    .fetch_optional(db)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            // Check for token reuse (theft detection) and revoke family if detected
            if let Ok(Some(family)) = sqlx::query_scalar!(
                "SELECT token_family FROM mcp_oauth_refresh_token
                 WHERE refresh_token = $1 AND used_at IS NOT NULL",
                refresh_token_value
            )
            .fetch_optional(db)
            .await
            {
                tracing::warn!("Refresh token reuse detected, revoking family {:?}", family);
                let _ = sqlx::query!(
                    "UPDATE mcp_oauth_refresh_token SET revoked = TRUE WHERE token_family = $1",
                    family
                )
                .execute(db)
                .await;
            }
            return Err(OAuthTokenError::invalid_grant("Invalid refresh token"));
        }
        Err(e) => {
            tracing::error!("Database error claiming refresh token: {}", e);
            return Err(OAuthTokenError::server_error("Database error"));
        }
    };

    // Delete old access token
    if let Err(e) = sqlx::query!("DELETE FROM token WHERE token = $1", token_row.access_token)
        .execute(db)
        .await
    {
        tracing::error!("Failed to delete old access token: {}", e);
        // Non-fatal, continue with token creation
    }

    // Generate new tokens
    let new_access_token = rd_string(32);
    let new_refresh_token = rd_string(32);
    let scopes = token_row.scopes;

    // Create new access token (rejects archived workspaces inline)
    let rows = sqlx::query!(
        "INSERT INTO token (token, email, label, expiration, scopes, workspace_id)
         SELECT $1::varchar, $2::varchar, $3::varchar, now() + ($4 || ' seconds')::interval, $5::text[], $6::varchar
         WHERE NOT EXISTS(SELECT 1 FROM workspace WHERE id = $6 AND deleted = true)",
        new_access_token,
        token_row.user_email,
        format!("mcp-oauth-{}", token_row.client_id),
        MCP_OAUTH_TOKEN_EXPIRATION_SECS.to_string(),
        &scopes,
        token_row.workspace_id,
    )
    .execute(db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create new access token: {}", e);
        OAuthTokenError::server_error("Failed to create access token")
    })?;
    if rows.rows_affected() == 0 {
        return Err(OAuthTokenError::invalid_grant(
            "Cannot create a token for an archived workspace",
        ));
    }

    // Create new refresh token (same token family for tracking)
    if let Err(e) = sqlx::query!(
        "INSERT INTO mcp_oauth_refresh_token
         (refresh_token, access_token, client_id, user_email, workspace_id, scopes, token_family, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, now() + ($8 || ' seconds')::interval)",
        new_refresh_token,
        new_access_token,
        token_row.client_id,
        token_row.user_email,
        token_row.workspace_id,
        &scopes,
        token_row.token_family,
        MCP_OAUTH_REFRESH_TOKEN_EXPIRATION_SECS.to_string(),
    )
    .execute(db)
    .await
    {
        tracing::error!("Failed to create new refresh token: {}", e);
        // Access token was created, return success without refresh token
    }

    Ok(Json(TokenResponse {
        access_token: new_access_token,
        token_type: "Bearer".to_string(),
        expires_in: MCP_OAUTH_TOKEN_EXPIRATION_SECS,
        scope: Some(scopes.join(" ")),
        refresh_token: Some(new_refresh_token),
    }))
}

/// GET /api/w/:workspace_id/mcp/oauth/server/authorize - redirects to consent page
pub async fn workspaced_oauth_authorize(
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
    Query(params): Query<AuthorizeQuery>,
) -> impl IntoResponse {
    let client = match sqlx::query_as!(
        OAuthClient,
        "SELECT client_id, client_name, redirect_uris FROM mcp_oauth_server_client WHERE client_id = $1",
        params.client_id
    )
    .fetch_optional(&db)
    .await
    {
        Ok(Some(client)) => client,
        Ok(None) => {
            return OAuthJsonError::new("invalid_client", Some("Unknown client_id"))
                .into_response();
        }
        Err(e) => {
            tracing::error!("Database error looking up client: {}", e);
            return OAuthJsonError::new("server_error", Some("Database error"))
                .into_response();
        }
    };

    if !client.redirect_uris.contains(&params.redirect_uri) {
        return OAuthJsonError::new(
            "invalid_request",
            Some("redirect_uri does not match registered URIs"),
        )
        .into_response();
    }

    if params.response_type != "code" {
        return OAuthErrorRedirect::new(
            &params.redirect_uri,
            "unsupported_response_type",
            Some("Only 'code' response type is supported"),
            params.state.as_deref(),
        )
        .into_response();
    }

    let code_challenge = match &params.code_challenge {
        Some(challenge) if !challenge.is_empty() => challenge.as_str(),
        _ => {
            return OAuthErrorRedirect::new(
                &params.redirect_uri,
                "invalid_request",
                Some("PKCE required: code_challenge parameter is mandatory"),
                params.state.as_deref(),
            )
            .into_response();
        }
    };

    let code_challenge_method = params.code_challenge_method.as_deref().unwrap_or("S256");

    if code_challenge_method != "S256" {
        return OAuthErrorRedirect::new(
            &params.redirect_uri,
            "invalid_request",
            Some("Invalid code_challenge_method: only 'S256' is supported"),
            params.state.as_deref(),
        )
        .into_response();
    }

    let resource = match &params.resource {
        Some(r) => r,
        None => {
            return OAuthErrorRedirect::new(
                &params.redirect_uri,
                "invalid_request",
                Some("Missing 'resource' parameter. Required for MCP audience binding."),
                params.state.as_deref(),
            )
            .into_response();
        }
    };

    let base_url = BASE_URL.read().await;
    let frontend_url = format!(
        "{}/oauth/mcp_authorize?{}",
        base_url,
        serde_urlencoded::to_string(&[
            ("workspace_id", workspace_id.as_str()),
            ("client_id", params.client_id.as_str()),
            ("client_name", client.client_name.as_str()),
            ("redirect_uri", params.redirect_uri.as_str()),
            ("scope", params.scope.as_deref().unwrap_or("mcp:all")),
            ("state", params.state.as_deref().unwrap_or("")),
            ("code_challenge", code_challenge),
            ("code_challenge_method", code_challenge_method),
            ("resource", resource),
        ])
        .unwrap_or_default()
    );

    Redirect::temporary(&frontend_url).into_response()
}

/// POST /api/w/:workspace_id/mcp/oauth/server/approve - user approval (frontend)
pub async fn workspaced_oauth_approve(
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
    authed: ApiAuthed,
    Json(form): Json<ApprovalForm>,
) -> Result<Json<ApprovalResponse>> {
    // Verify user is a member of the workspace
    let is_member = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = $1 AND email = $2 AND NOT disabled)",
        workspace_id,
        authed.email
    )
    .fetch_one(&db)
    .await
    .map_err(|e| Error::InternalErr(format!("Database error: {}", e)))?
    .unwrap_or(false);

    if !is_member {
        return Err(Error::NotAuthorized(
            "User is not a member of this workspace".to_string(),
        ));
    }

    // Verify client exists and redirect_uri is registered
    let client = sqlx::query_as!(
        OAuthClient,
        "SELECT client_id, client_name, redirect_uris FROM mcp_oauth_server_client WHERE client_id = $1",
        form.client_id
    )
    .fetch_optional(&db)
    .await
    .map_err(|e| Error::InternalErr(format!("Database error: {}", e)))?
    .ok_or_else(|| Error::BadRequest("Unknown client_id".to_string()))?;

    if !client.redirect_uris.contains(&form.redirect_uri) {
        return Err(Error::BadRequest(
            "Invalid redirect_uri for this client".to_string(),
        ));
    }

    if form.code_challenge.is_empty() {
        return Err(Error::BadRequest(
            "PKCE required: code_challenge is mandatory".to_string(),
        ));
    }

    if form.code_challenge_method.is_empty() {
        return Err(Error::BadRequest(
            "PKCE required: code_challenge_method is mandatory".to_string(),
        ));
    }

    if form.code_challenge_method != "S256" {
        return Err(Error::BadRequest(
            "Invalid code_challenge_method: only 'S256' is supported".to_string(),
        ));
    }

    let code = format!("mcp-code-{}", rd_string(32));

    let scopes: Vec<String> = form
        .scope
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    sqlx::query!(
        "INSERT INTO mcp_oauth_server_code
         (code, client_id, user_email, workspace_id, scopes, redirect_uri, code_challenge, code_challenge_method)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        code,
        form.client_id,
        authed.email,
        workspace_id,
        &scopes,
        form.redirect_uri,
        &form.code_challenge,
        &form.code_challenge_method,
    )
    .execute(&db)
    .await
    .map_err(|e| Error::InternalErr(format!("Failed to store authorization code: {}", e)))?;

    Ok(Json(ApprovalResponse {
        code,
        state: if form.state.is_empty() {
            None
        } else {
            Some(form.state)
        },
    }))
}

/// PKCE validation (S256 only)
fn validate_pkce_s256(verifier: &str, challenge: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let computed = base64_url_encode(&hasher.finalize());
    constant_time_eq(computed.as_bytes(), challenge.as_bytes())
}

/// Base64 URL encoding (no padding)
fn base64_url_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(data)
}

/// Constant-time comparison
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Helper for OAuth error redirects
struct OAuthErrorRedirect {
    redirect_uri: String,
    error: String,
    error_description: Option<String>,
    state: Option<String>,
}

impl OAuthErrorRedirect {
    fn new(
        redirect_uri: &str,
        error: &str,
        error_description: Option<&str>,
        state: Option<&str>,
    ) -> Self {
        Self {
            redirect_uri: redirect_uri.to_string(),
            error: error.to_string(),
            error_description: error_description.map(|s| s.to_string()),
            state: state.map(|s| s.to_string()),
        }
    }
}

impl IntoResponse for OAuthErrorRedirect {
    fn into_response(self) -> axum::response::Response {
        let mut url = format!("{}?error={}", self.redirect_uri, self.error);
        if let Some(desc) = &self.error_description {
            url.push_str(&format!("&error_description={}", urlencoding::encode(desc)));
        }
        if let Some(state) = &self.state {
            url.push_str(&format!("&state={}", state));
        }
        Redirect::temporary(&url).into_response()
    }
}

/// Mounted at /api/mcp/oauth/server
pub fn global_service() -> Router {
    Router::new().route("/register", post(oauth_register))
}

/// Workspace-scoped OAuth endpoints that don't require authentication
/// Mounted at /api/w/:workspace_id/mcp/oauth/server (outside authenticated section)
pub fn workspaced_unauthed_service() -> Router {
    Router::new()
        .route("/authorize", get(workspaced_oauth_authorize))
        .route("/token", post(oauth_token))
}

/// Workspace-scoped OAuth endpoints that require authentication
/// Mounted at /api/w/:workspace_id/mcp/oauth/server (inside authenticated section)
pub fn workspaced_authed_service() -> Router {
    Router::new().route("/approve", post(workspaced_oauth_approve))
}
