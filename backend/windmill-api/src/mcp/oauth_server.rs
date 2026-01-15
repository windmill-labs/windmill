//! OAuth Authorization Server for MCP
//!
//! This module implements Windmill as an OAuth 2.0 Authorization Server,
//! allowing external MCP clients (like Claude Desktop) to authenticate
//! and obtain tokens to access Windmill's MCP tools.
//!
//! Implements:
//! - RFC 8414: OAuth 2.0 Authorization Server Metadata
//! - RFC 7591: OAuth 2.0 Dynamic Client Registration
//! - RFC 6749: OAuth 2.0 Authorization Framework
//! - RFC 7636: PKCE (Proof Key for Code Exchange)

use axum::{
    extract::{Extension, Query},
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
    workspace_id: Option<String>,
    scopes: Vec<String>,
    redirect_uri: String,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /.well-known/oauth-authorization-server
/// Returns OAuth 2.0 Authorization Server Metadata
pub async fn oauth_metadata() -> Json<AuthorizationMetadata> {
    let base_url = BASE_URL.read().await.clone();
    let base_url = if base_url.is_empty() {
        "http://localhost:8000".to_string()
    } else {
        base_url.trim_end_matches('/').to_string()
    };

    Json(AuthorizationMetadata {
        issuer: base_url.clone(),
        authorization_endpoint: format!("{}/api/mcp/oauth/server/authorize", base_url),
        token_endpoint: format!("{}/api/mcp/oauth/server/token", base_url),
        registration_endpoint: Some(format!("{}/api/mcp/oauth/server/register", base_url)),
        scopes_supported: Some(vec![
            "mcp:all".to_string(),
            "mcp:favorites".to_string(),
            "mcp:scripts:*".to_string(),
            "mcp:flows:*".to_string(),
            "mcp:endpoints:*".to_string(),
        ]),
        response_types_supported: Some(vec!["code".to_string()]),
        code_challenge_methods_supported: Some(vec!["S256".to_string(), "plain".to_string()]),
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

/// GET /api/mcp/oauth/server/authorize
/// Authorization Endpoint - validates params and redirects to frontend consent page
pub async fn oauth_authorize(
    Extension(db): Extension<DB>,
    Query(params): Query<AuthorizeQuery>,
) -> impl IntoResponse {
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

    // Validate client exists and redirect_uri matches
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
            return OAuthErrorRedirect::new(
                &params.redirect_uri,
                "invalid_client",
                Some("Unknown client_id"),
                params.state.as_deref(),
            )
            .into_response();
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return OAuthErrorRedirect::new(
                &params.redirect_uri,
                "server_error",
                Some("Database error"),
                params.state.as_deref(),
            )
            .into_response();
        }
    };

    // Validate redirect_uri
    if !client.redirect_uris.contains(&params.redirect_uri) {
        return OAuthErrorRedirect::new(
            &params.redirect_uri,
            "invalid_request",
            Some("redirect_uri does not match registered URIs"),
            params.state.as_deref(),
        )
        .into_response();
    }

    // Redirect to frontend consent page (frontend handles login redirect)
    let base_url = BASE_URL.read().await.clone();
    let frontend_url = format!(
        "{}/oauth/mcp_authorize?{}",
        base_url,
        serde_urlencoded::to_string(&[
            ("client_id", params.client_id.as_str()),
            ("client_name", client.client_name.as_str()),
            ("redirect_uri", params.redirect_uri.as_str()),
            ("scope", params.scope.as_deref().unwrap_or("mcp:all")),
            ("state", params.state.as_deref().unwrap_or("")),
            (
                "code_challenge",
                params.code_challenge.as_deref().unwrap_or("")
            ),
            (
                "code_challenge_method",
                params.code_challenge_method.as_deref().unwrap_or("")
            ),
        ])
        .unwrap_or_default()
    );

    Redirect::temporary(&frontend_url).into_response()
}

/// Approval Response (returned to frontend)
#[derive(Debug, Serialize)]
pub struct ApprovalResponse {
    pub code: String,
    pub state: Option<String>,
}

/// POST /api/mcp/oauth/server/approve
/// Handle user approval of authorization request (called by frontend)
pub async fn oauth_approve(
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(form): Json<ApprovalForm>,
) -> Result<Json<ApprovalResponse>> {
    // Generate authorization code
    let code = format!("mcp-code-{}", rd_string(32));

    // Parse scopes
    let scopes: Vec<String> = form
        .scope
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    // Store the authorization code
    sqlx::query!(
        "INSERT INTO mcp_oauth_server_code
         (code, client_id, user_email, scopes, redirect_uri, code_challenge, code_challenge_method)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        code,
        form.client_id,
        authed.email,
        &scopes,
        form.redirect_uri,
        if form.code_challenge.is_empty() {
            None
        } else {
            Some(&form.code_challenge)
        },
        if form.code_challenge_method.is_empty() {
            None
        } else {
            Some(&form.code_challenge_method)
        },
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

/// POST /api/mcp/oauth/server/token
/// Token endpoint - exchange authorization code for access token
pub async fn oauth_token(
    Extension(db): Extension<DB>,
    Form(req): Form<TokenRequest>,
) -> Result<Json<TokenResponse>> {
    // Validate grant_type
    if req.grant_type != "authorization_code" {
        return Err(Error::BadRequest(
            "Only authorization_code grant type is supported".to_string(),
        ));
    }

    // Fetch and validate the authorization code
    let auth_code = sqlx::query_as!(
        AuthorizationCode,
        "SELECT code, client_id, user_email, workspace_id, scopes, redirect_uri,
                code_challenge, code_challenge_method
         FROM mcp_oauth_server_code
         WHERE code = $1 AND expires_at > now()",
        req.code
    )
    .fetch_optional(&db)
    .await
    .map_err(|e| Error::InternalErr(format!("Database error: {}", e)))?
    .ok_or_else(|| Error::BadRequest("Invalid or expired authorization code".to_string()))?;

    // Validate client_id
    if auth_code.client_id != req.client_id {
        return Err(Error::BadRequest("client_id mismatch".to_string()));
    }

    // Validate redirect_uri
    if auth_code.redirect_uri != req.redirect_uri {
        return Err(Error::BadRequest("redirect_uri mismatch".to_string()));
    }

    // Validate PKCE if code_challenge was provided
    if let Some(challenge) = &auth_code.code_challenge {
        let verifier = req
            .code_verifier
            .as_ref()
            .ok_or_else(|| Error::BadRequest("code_verifier required for PKCE".to_string()))?;

        let method = auth_code
            .code_challenge_method
            .as_deref()
            .unwrap_or("plain");
        if !validate_pkce(verifier, challenge, method) {
            return Err(Error::BadRequest("Invalid code_verifier".to_string()));
        }
    }

    // Delete the authorization code (single-use)
    sqlx::query!(
        "DELETE FROM mcp_oauth_server_code WHERE code = $1",
        req.code
    )
    .execute(&db)
    .await
    .map_err(|e| Error::InternalErr(format!("Failed to delete code: {}", e)))?;

    // Create a Windmill token with MCP scopes
    let token = rd_string(32);
    let scopes = auth_code.scopes;
    let expires_in: u64 = 86400; // 24 hours

    sqlx::query!(
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
    .map_err(|e| Error::InternalErr(format!("Failed to create token: {}", e)))?;

    Ok(Json(TokenResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in,
        scope: Some(scopes.join(" ")),
    }))
}

// ============================================================================
// Helpers
// ============================================================================

/// PKCE validation
fn validate_pkce(verifier: &str, challenge: &str, method: &str) -> bool {
    match method {
        "S256" => {
            let mut hasher = Sha256::new();
            hasher.update(verifier.as_bytes());
            let computed = base64_url_encode(&hasher.finalize());
            constant_time_eq(computed.as_bytes(), challenge.as_bytes())
        }
        "plain" => verifier == challenge,
        _ => false,
    }
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

/// Create the OAuth server router
pub fn global_service() -> Router {
    Router::new()
        .route("/register", post(oauth_register))
        .route("/authorize", get(oauth_authorize))
        .route("/approve", post(oauth_approve))
        .route("/token", post(oauth_token))
}
