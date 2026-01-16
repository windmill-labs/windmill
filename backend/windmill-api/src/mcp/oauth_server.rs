//! OAuth Authorization Server for MCP
//!
//! This module implements Windmill as an OAuth 2.0 Authorization Server,
//! allowing external MCP clients (like Claude Desktop) to authenticate
//! and obtain tokens to access Windmill's MCP tools.
//!
//! Implements:
//! - RFC 8414: OAuth 2.0 Authorization Server Metadata
//! - RFC 9728: OAuth 2.0 Protected Resource Metadata
//! - RFC 7591: OAuth 2.0 Dynamic Client Registration
//! - RFC 6749: OAuth 2.0 Authorization Framework
//! - RFC 7636: PKCE (Proof Key for Code Exchange)

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

// ============================================================================
// Types
// ============================================================================

/// OAuth 2.0 Authorization Server Metadata (RFC 8414)
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
    pub code_challenge_methods_supported: Option<Vec<String>>,
}

/// OAuth 2.0 Protected Resource Metadata (RFC 9728)
/// Returned by /.well-known/oauth-protected-resource to help MCP clients
/// discover the authorization server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedResourceMetadata {
    /// The resource identifier (URI of the MCP server)
    pub resource: String,
    /// List of authorization servers that can issue tokens for this resource
    pub authorization_servers: Vec<String>,
    /// Scopes supported by this resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes_supported: Option<Vec<String>>,
    /// Bearer token methods supported
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer_methods_supported: Option<Vec<String>>,
}

/// OAuth error response for JSON errors (non-redirect cases)
/// Used when we cannot safely redirect (e.g., invalid client_id)
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

/// OAuth token endpoint error response (RFC 6749 Section 5.2)
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

/// Dynamic Client Registration Request (RFC 7591)
#[derive(Debug, Deserialize)]
pub struct ClientRegistrationRequest {
    pub client_name: String,
    pub redirect_uris: Vec<String>,
}

/// Dynamic Client Registration Response (RFC 7591)
#[derive(Debug, Serialize)]
pub struct ClientRegistrationResponse {
    pub client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
}

/// Authorization Request Query Parameters
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
}

/// Authorization Approval Form
#[derive(Debug, Deserialize)]
pub struct ApprovalForm {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

/// Token Request (application/x-www-form-urlencoded)
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
}

/// Token Response
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Database row for OAuth client
#[allow(dead_code)]
#[derive(Debug, FromRow)]
struct OAuthClient {
    client_id: String,
    client_name: String,
    redirect_uris: Vec<String>,
}

/// Database row for authorization code
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

// ============================================================================
// Handlers
// ============================================================================

/// Helper to get the normalized base URL
async fn get_base_url() -> String {
    let base_url = BASE_URL.read().await.clone();
    if base_url.is_empty() {
        "http://localhost:8000".to_string()
    } else {
        base_url.trim_end_matches('/').to_string()
    }
}

/// Supported MCP scopes
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
/// Returns OAuth 2.0 Authorization Server Metadata for workspace-scoped OAuth server (RFC 8414)
/// Per RFC 8414, when issuer is http://host/path, metadata is at http://host/.well-known/oauth-authorization-server/path
///
/// Note: registration_endpoint is GLOBAL (not workspace-scoped) because client registration
/// is a one-time operation that doesn't require workspace context. A single MCP client
/// (like Claude Desktop) registers once and can then authorize access to multiple workspaces.
pub async fn workspaced_oauth_metadata(
    Path(workspace_id): Path<String>,
) -> Json<AuthorizationMetadata> {
    let base_url = get_base_url().await;
    let issuer = format!("{}/api/w/{}/mcp/oauth/server", base_url, workspace_id);

    Json(AuthorizationMetadata {
        issuer,
        authorization_endpoint: format!("{}/api/w/{}/mcp/oauth/server/authorize", base_url, workspace_id),
        token_endpoint: format!("{}/api/w/{}/mcp/oauth/server/token", base_url, workspace_id),
        // Registration is global - client registers once and can authorize to multiple workspaces
        registration_endpoint: Some(format!("{}/api/mcp/oauth/server/register", base_url)),
        scopes_supported: Some(supported_scopes()),
        response_types_supported: Some(vec!["code".to_string()]),
        code_challenge_methods_supported: Some(vec!["S256".to_string()]),
    })
}

/// GET /.well-known/oauth-protected-resource/api/mcp/w/:workspace_id/sse
/// RFC 9728 path-based protected resource metadata discovery.
/// Per RFC 9728, for resource http://host/api/mcp/w/test/sse, metadata is at
/// http://host/.well-known/oauth-protected-resource/api/mcp/w/test/sse
pub async fn protected_resource_metadata_by_path(
    Path(workspace_id): Path<String>,
) -> Json<ProtectedResourceMetadata> {
    let base_url = get_base_url().await;
    let resource_url = format!("{}/api/mcp/w/{}/sse", base_url, workspace_id);
    // Return workspace-scoped authorization server URL
    let auth_server_url = format!("{}/api/w/{}/mcp/oauth/server", base_url, workspace_id);
    Json(ProtectedResourceMetadata {
        resource: resource_url,
        authorization_servers: vec![auth_server_url],
        scopes_supported: Some(supported_scopes()),
        bearer_methods_supported: Some(vec!["header".to_string()]),
    })
}

/// POST /api/mcp/oauth/server/register
/// Dynamic Client Registration (RFC 7591)
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
            client_secret: None, // Public client, PKCE required
            client_name: req.client_name,
            redirect_uris: req.redirect_uris,
        }),
    ))
}

/// Approval Response (returned to frontend)
#[derive(Debug, Serialize)]
pub struct ApprovalResponse {
    pub code: String,
    pub state: Option<String>,
}

/// POST /api/mcp/oauth/server/token
/// POST /api/w/:workspace_id/mcp/oauth/server/token
/// Token endpoint - exchange authorization code for access token
/// Returns RFC 6749 compliant error responses.
pub async fn oauth_token(
    Extension(db): Extension<DB>,
    Form(req): Form<TokenRequest>,
) -> std::result::Result<Json<TokenResponse>, OAuthTokenError> {
    // Validate grant_type
    if req.grant_type != "authorization_code" {
        return Err(OAuthTokenError::unsupported_grant_type(
            "Only authorization_code grant type is supported",
        ));
    }

    // Atomically fetch and delete the authorization code (single-use)
    // Using DELETE ... RETURNING prevents race conditions where the same code
    // could be replayed in concurrent requests
    let auth_code = match sqlx::query_as!(
        AuthorizationCode,
        "DELETE FROM mcp_oauth_server_code
         WHERE code = $1 AND expires_at > now()
         RETURNING code, client_id, user_email, workspace_id, scopes, redirect_uri,
                   code_challenge, code_challenge_method",
        req.code
    )
    .fetch_optional(&db)
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

    // Validate client_id
    if auth_code.client_id != req.client_id {
        return Err(OAuthTokenError::invalid_grant("client_id mismatch"));
    }

    // Validate redirect_uri
    if auth_code.redirect_uri != req.redirect_uri {
        return Err(OAuthTokenError::invalid_grant("redirect_uri mismatch"));
    }

    // PKCE is required - code_challenge must be present
    let challenge = auth_code.code_challenge.as_ref().ok_or_else(|| {
        OAuthTokenError::invalid_grant("Authorization code missing PKCE challenge")
    })?;

    // Validate PKCE (S256 only)
    let verifier = req
        .code_verifier
        .as_ref()
        .ok_or_else(|| OAuthTokenError::invalid_request("code_verifier is required"))?;

    // Verify the method is S256 (should always be, but defense in depth)
    let method = auth_code.code_challenge_method.as_deref().unwrap_or("S256");
    if method != "S256" {
        return Err(OAuthTokenError::invalid_grant(
            "Only S256 PKCE method is supported",
        ));
    }

    if !validate_pkce_s256(verifier, challenge) {
        return Err(OAuthTokenError::invalid_grant("Invalid code_verifier"));
    }

    // Create a Windmill token with MCP scopes
    let token = rd_string(32);
    let scopes = auth_code.scopes;
    let expires_in: u64 = 86400; // 24 hours

    if let Err(e) = sqlx::query!(
        "INSERT INTO token (token, email, label, expiration, scopes, workspace_id)
         VALUES ($1, $2, $3, now() + ($4 || ' seconds')::interval, $5, $6)",
        token,
        auth_code.user_email,
        format!("mcp-oauth-{}", auth_code.client_id),
        expires_in.to_string(),
        &scopes,
        auth_code.workspace_id,
    )
    .execute(&db)
    .await
    {
        tracing::error!("Failed to create token: {}", e);
        return Err(OAuthTokenError::server_error(
            "Failed to create access token",
        ));
    }

    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in,
        scope: Some(scopes.join(" ")),
    }))
}

// ============================================================================
// Workspace-Scoped Handlers
// ============================================================================

/// GET /api/w/:workspace_id/mcp/oauth/server/authorize
/// Workspace-scoped authorization endpoint - validates params and redirects to frontend consent page
pub async fn workspaced_oauth_authorize(
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
    Query(params): Query<AuthorizeQuery>,
) -> impl IntoResponse {
    // First, validate client exists - we cannot trust redirect_uri until we verify client
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
            // RFC 6749 4.1.2.1: If client_id is invalid, do NOT redirect
            return OAuthJsonError::new("invalid_client", Some("Unknown client_id"))
                .into_response();
        }
        Err(e) => {
            tracing::error!("Database error looking up client: {}", e);
            return OAuthJsonError::new("server_error", Some("Database error"))
                .into_response();
        }
    };

    // Validate redirect_uri BEFORE any redirects
    // RFC 6749 4.1.2.1: If redirect_uri is invalid, do NOT redirect
    if !client.redirect_uris.contains(&params.redirect_uri) {
        return OAuthJsonError::new(
            "invalid_request",
            Some("redirect_uri does not match registered URIs"),
        )
        .into_response();
    }

    // Now we have a validated redirect_uri, we can use OAuthErrorRedirect for errors

    // Validate response_type
    if params.response_type != "code" {
        return OAuthErrorRedirect::new(
            &params.redirect_uri,
            "unsupported_response_type",
            Some("Only 'code' response type is supported"),
            params.state.as_deref(),
        )
        .into_response();
    }

    // Require PKCE for public clients (this is a public client server)
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

    // Validate code_challenge_method (must be S256)
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

    // Redirect to frontend consent page with workspace_id included
    let base_url = get_base_url().await;
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
        ])
        .unwrap_or_default()
    );

    Redirect::temporary(&frontend_url).into_response()
}

/// POST /api/w/:workspace_id/mcp/oauth/server/approve
/// Handle user approval of authorization request (called by frontend)
/// Stores the workspace_id with the authorization code
pub async fn workspaced_oauth_approve(
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
    authed: ApiAuthed,
    Json(form): Json<ApprovalForm>,
) -> Result<Json<ApprovalResponse>> {
    // PKCE is required for public clients - validate code_challenge is present
    if form.code_challenge.is_empty() {
        return Err(Error::BadRequest(
            "PKCE required: code_challenge is mandatory".to_string(),
        ));
    }

    // Validate code_challenge_method
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

    // Generate authorization code
    let code = format!("mcp-code-{}", rd_string(32));

    // Parse scopes
    let scopes: Vec<String> = form
        .scope
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    // Store the authorization code with workspace_id
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

// ============================================================================
// Helpers
// ============================================================================

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

// ============================================================================
// Router
// ============================================================================

/// Global OAuth endpoints (unauthenticated) - only registration
/// Client registration is a one-time operation, not workspace-specific.
/// Token exchange is at workspace-scoped endpoint.
/// Mounted at /api/mcp/oauth/server
pub fn global_service() -> Router {
    Router::new().route("/register", post(oauth_register))
}

/// Workspace-scoped OAuth endpoints that don't require authentication
/// These are called by the MCP client during the OAuth flow, before the user is authenticated.
/// Mounted at /api/w/:workspace_id/mcp/oauth/server (outside authenticated section)
pub fn workspaced_unauthed_service() -> Router {
    Router::new()
        .route("/authorize", get(workspaced_oauth_authorize))
        .route("/token", post(oauth_token))
}

/// Workspace-scoped OAuth endpoints that require authentication
/// Called by the frontend when the logged-in user approves access.
/// Mounted at /api/w/:workspace_id/mcp/oauth/server (inside authenticated section)
pub fn workspaced_authed_service() -> Router {
    Router::new().route("/approve", post(workspaced_oauth_approve))
}
