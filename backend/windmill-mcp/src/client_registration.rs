/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! MCP OAuth client registration and credential management.
//!
//! Handles dynamic client registration (DCR) and Client Identifier Metadata Document (CIMD)
//! flows for MCP OAuth, including caching registered credentials in the database.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use windmill_common::db::DB;
use windmill_common::error;
use windmill_common::variables::{build_crypt, decrypt, encrypt};

use crate::oauth::{no_redirect_http_client_pinned, AuthorizationManager};

/// MCP client credentials returned by [`get_or_refresh_mcp_client`].
pub struct McpClientCredentials {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub token_endpoint: String,
}

#[derive(FromRow)]
struct McpOAuthClient {
    #[allow(dead_code)]
    mcp_server_url: String,
    client_id: String,
    client_secret: Option<String>,
    client_secret_expires_at: Option<chrono::NaiveDateTime>,
    token_endpoint: String,
}

impl McpOAuthClient {
    fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.client_secret_expires_at {
            expires_at < chrono::Utc::now().naive_utc()
        } else {
            false
        }
    }
}

async fn encrypt_client_secret(db: &DB, client_secret: &str) -> error::Result<String> {
    let mc = build_crypt(db, "admins").await?;
    Ok(encrypt(&mc, client_secret))
}

async fn decrypt_client_secret(db: &DB, encrypted_secret: &str) -> error::Result<String> {
    let mc = build_crypt(db, "admins").await?;
    decrypt(&mc, encrypted_secret.to_string())
}

#[derive(Serialize)]
struct DcrRequest {
    client_name: String,
    redirect_uris: Vec<String>,
    grant_types: Vec<String>,
    response_types: Vec<String>,
    token_endpoint_auth_method: String,
}

#[derive(Deserialize)]
struct DcrResponse {
    client_id: String,
    client_secret: Option<String>,
    client_secret_expires_at: Option<i64>,
}

async fn register_client(
    registration_endpoint: &str,
    redirect_uri: &str,
    client_name: &str,
) -> Result<DcrResponse, error::Error> {
    let validated = windmill_common::ssrf::validate_mcp_server_url_for_bad_request(
        registration_endpoint,
        "MCP server registration endpoint URL",
    )
    .await?;

    // Pin to the validated address: DCR posts to an author-controlled endpoint,
    // so the connect must not rebind to an internal IP after the check.
    let client = no_redirect_http_client_pinned(&validated)
        .map_err(|e| error::Error::BadRequest(format!("Failed to build DCR client: {e}")))?;
    let request = DcrRequest {
        client_name: client_name.to_string(),
        redirect_uris: vec![redirect_uri.to_string()],
        grant_types: vec![
            "authorization_code".to_string(),
            "refresh_token".to_string(),
        ],
        response_types: vec!["code".to_string()],
        token_endpoint_auth_method: "none".to_string(),
    };

    let response = client
        .post(registration_endpoint)
        .json(&request)
        .send()
        .await
        .map_err(|e| error::Error::BadRequest(format!("DCR request failed: {e}")))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(error::Error::BadRequest(format!(
            "DCR failed: {error_text}"
        )));
    }

    response
        .json::<DcrResponse>()
        .await
        .map_err(|e| error::Error::BadRequest(format!("DCR response parse failed: {e}")))
}

/// Get MCP client credentials from cache, re-registering if expired.
///
/// Supports two modes:
/// - DCR (Dynamic Client Registration): If the server's metadata includes a registration endpoint
/// - CIMD (Client Identifier Metadata Document): Otherwise, uses our metadata URL as client_id
pub async fn get_or_refresh_mcp_client(
    db: &DB,
    mcp_server_url: &str,
) -> Result<McpClientCredentials, error::Error> {
    let base_url = (**windmill_common::BASE_URL.load()).clone();
    let redirect_uri = format!("{}/api/mcp/oauth/callback", base_url);

    let validated_server = windmill_common::ssrf::validate_mcp_server_url_for_bad_request(
        mcp_server_url,
        "MCP server URL",
    )
    .await?;

    let cached_client: Option<McpOAuthClient> =
        sqlx::query_as("SELECT mcp_server_url, client_id, client_secret, client_secret_expires_at, token_endpoint FROM mcp_oauth_client WHERE mcp_server_url = $1")
            .bind(mcp_server_url)
            .fetch_optional(db)
            .await
            .map_err(|e| error::Error::InternalErr(format!("Database error: {e}")))?;

    if let Some(client) = cached_client {
        if !client.is_expired() {
            tracing::debug!("Using cached MCP client for {}", mcp_server_url);
            windmill_common::ssrf::validate_mcp_server_url_for_bad_request(
                &client.token_endpoint,
                "MCP server token endpoint URL",
            )
            .await?;
            let decrypted_secret = if let Some(ref encrypted_secret) = client.client_secret {
                Some(decrypt_client_secret(db, encrypted_secret).await?)
            } else {
                None
            };
            return Ok(McpClientCredentials {
                client_id: client.client_id,
                client_secret: decrypted_secret,
                token_endpoint: client.token_endpoint,
            });
        }
        tracing::debug!("Cached MCP client expired, re-registering");
    }

    let mut manager = AuthorizationManager::new(mcp_server_url)
        .await
        .map_err(|e| error::Error::BadRequest(format!("Failed to create auth manager: {e}")))?;
    // Discovery hits the well-known endpoint on the MCP server host validated
    // above; pin to that address so it cannot rebind between check and connect.
    // Limitation: rmcp's discover_metadata may additionally follow server-supplied
    // metadata URLs (resource_metadata / authorization_servers) on other hosts,
    // which this per-host pin does not cover — a pre-existing gap in rmcp discovery
    // that a validating resolver would need to close, out of scope for this pin.
    let discovery_client = no_redirect_http_client_pinned(&validated_server).map_err(|e| {
        error::Error::BadRequest(format!("Failed to build MCP OAuth discovery client: {e}"))
    })?;
    manager
        .with_client(discovery_client)
        .map_err(|e| error::Error::BadRequest(format!("Failed to configure auth manager: {e}")))?;

    let metadata = manager
        .discover_metadata()
        .await
        .map_err(|e| error::Error::BadRequest(format!("OAuth discovery failed: {e}")))?;

    windmill_common::ssrf::validate_mcp_server_url_for_bad_request(
        &metadata.token_endpoint,
        "MCP server token endpoint URL",
    )
    .await?;

    let supports_dynamic_registration = metadata.registration_endpoint.is_some();

    let (client_id, client_secret, expires_at) = if supports_dynamic_registration {
        let reg_endpoint = metadata.registration_endpoint.as_ref().unwrap();
        tracing::debug!("Performing DCR at {}", reg_endpoint);
        let dcr = register_client(reg_endpoint, &redirect_uri, "Windmill").await?;

        let expires_at = dcr.client_secret_expires_at.and_then(|ts| {
            if ts == 0 {
                None
            } else {
                chrono::DateTime::from_timestamp(ts, 0).map(|dt| dt.naive_utc())
            }
        });

        (dcr.client_id, dcr.client_secret, expires_at)
    } else {
        let client_metadata_url = format!("{}/api/mcp/oauth/client-metadata.json", base_url);
        tracing::debug!(
            "Using CIMD with metadata URL as client_id: {}",
            client_metadata_url
        );

        (client_metadata_url, None, None)
    };

    let encrypted_secret = if let Some(ref secret) = client_secret {
        Some(encrypt_client_secret(db, secret).await?)
    } else {
        None
    };

    sqlx::query(
        "INSERT INTO mcp_oauth_client (mcp_server_url, client_id, client_secret, client_secret_expires_at, token_endpoint)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (mcp_server_url) DO UPDATE
         SET client_id = EXCLUDED.client_id,
             client_secret = EXCLUDED.client_secret,
             client_secret_expires_at = EXCLUDED.client_secret_expires_at,
             token_endpoint = EXCLUDED.token_endpoint",
    )
    .bind(mcp_server_url)
    .bind(&client_id)
    .bind(&encrypted_secret)
    .bind(expires_at)
    .bind(&metadata.token_endpoint)
    .execute(db)
    .await
    .map_err(|e| error::Error::InternalErr(format!("Database error: {e}")))?;

    Ok(McpClientCredentials { client_id, client_secret, token_endpoint: metadata.token_endpoint })
}
