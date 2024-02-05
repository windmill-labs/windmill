/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::process::Command;

use anyhow;

#[cfg(feature = "enterprise")]
use openidconnect::{
    core::{
        CoreClaimName, CoreJsonWebKeySet, CoreJwsSigningAlgorithm, CoreProviderMetadata,
        CoreResponseType, CoreRsaPrivateSigningKey, CoreSubjectIdentifierType,
    },
    AdditionalClaims, AuthUrl, EmptyAdditionalProviderMetadata, IssuerUrl, JsonWebKeyId,
    JsonWebKeySetUrl, ResponseTypes,
};

#[cfg(feature = "enterprise")]
impl AdditionalClaims for JobClaim {}

use crate::db::DB;
use axum::extract::Path;
use axum::routing::{get, post};
use axum::Extension;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

#[cfg(feature = "enterprise")]
pub fn global_service() -> Router {
    Router::new()
        .route(
            "/.well-known/openid-configuration",
            get(openid_configuration),
        )
        .route("/jwks", get(jwks))
}
#[cfg(not(feature = "enterprise"))]
pub fn global_service() -> Router {
    Router::new()
}

#[cfg(not(feature = "enterprise"))]
pub fn workspaced_service() -> Router {
    Router::new()
}

#[cfg(feature = "enterprise")]
pub fn workspaced_service() -> Router {
    Router::new().route("/token/:audience", post(gen_token))
}

#[cfg(feature = "enterprise")]
#[derive(Debug, Clone, serde::Serialize)]
struct Keys {
    private_key: String,
}

#[cfg(feature = "enterprise")]
async fn gen_pems(db: &DB) -> anyhow::Result<Keys> {
    let private_key_cmd = Command::new("openssl")
        .arg("genrsa")
        .arg("--traditional")
        .arg("2048")
        .output()
        .expect("failed to execute process");

    let private_key = String::from_utf8(private_key_cmd.stdout).unwrap();

    tracing::debug!("Generated private key: {}", private_key);
    let keys = Keys { private_key };

    sqlx::query!(
        "INSERT INTO global_settings (name, value) VALUES ('rsa_keys', $1)",
        serde_json::to_value(&keys).unwrap()
    )
    .execute(db)
    .await?;

    Ok(keys)
}

#[cfg(feature = "enterprise")]
async fn get_private_key(db: &DB) -> anyhow::Result<String> {
    let key = sqlx::query_scalar!(
        "SELECT value->>'private_key' FROM global_settings WHERE name = 'rsa_keys'",
    )
    .fetch_optional(db)
    .await?
    .flatten();

    if let Some(key) = key {
        return Ok(key);
    } else {
        let keys = gen_pems(db).await?;
        return Ok(keys.private_key);
    }
}

#[cfg(feature = "enterprise")]
pub async fn jwks(
    Extension(db): Extension<DB>,
) -> windmill_common::error::JsonResult<CoreJsonWebKeySet> {
    use openidconnect::PrivateSigningKey;

    let private_key = get_private_key(&db).await?;
    let jwks = CoreJsonWebKeySet::new(vec![CoreRsaPrivateSigningKey::from_pem(
        &private_key,
        Some(JsonWebKeyId::new("windmill".to_string())),
    )
    .map_err(|e| anyhow::anyhow!("Failed to parse PEM: {}", e))?
    .as_verification_key()]);

    Ok(Json(jwks))
}

#[cfg(feature = "enterprise")]
pub async fn openid_configuration() -> windmill_common::error::JsonResult<CoreProviderMetadata> {
    use windmill_common::BASE_URL;

    let base_url = BASE_URL.read().await.clone();
    return get_provider_metadata(base_url)
        .map(Json)
        .map_err(|e| e.into());
}

#[cfg(feature = "enterprise")]
pub fn get_provider_metadata(base_url: String) -> anyhow::Result<CoreProviderMetadata> {
    let provider_metadata = CoreProviderMetadata::new(
        IssuerUrl::new(format!("{base_url}/api/oidc/"))?,
        AuthUrl::new(format!("{base_url}/api/oidc/"))?,
        JsonWebKeySetUrl::new(format!("{base_url}/api/oidc/jwks"))?,
        vec![
            // Optional: support the implicit flow.
            ResponseTypes::new(vec![CoreResponseType::Token, CoreResponseType::IdToken]), // Other flows including hybrid flows may also be specified here.
        ],
        vec![CoreSubjectIdentifierType::Public],
        vec![CoreJwsSigningAlgorithm::RsaSsaPssSha256],
        EmptyAdditionalProviderMetadata {},
    )
    // Recommended: specify the supported ID token claims.
    .set_claims_supported(Some(vec![
        // Providers may also define an enum instead of using CoreClaimName.
        CoreClaimName::new("sub".to_string()),
        CoreClaimName::new("aud".to_string()),
        CoreClaimName::new("email".to_string()),
        CoreClaimName::new("email_verified".to_string()),
        CoreClaimName::new("exp".to_string()),
        CoreClaimName::new("iat".to_string()),
        CoreClaimName::new("iss".to_string()),
        CoreClaimName::new("job_id".to_string()),
        CoreClaimName::new("path".to_string()),
        CoreClaimName::new("flow_path".to_string()),
        CoreClaimName::new("groups".to_string()),
        CoreClaimName::new("username".to_string()),
        CoreClaimName::new("workspace".to_string()),
    ]));
    return Ok(provider_metadata);
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
struct JobClaim {
    job_id: String,
    path: Option<String>,
    flow_path: Option<String>,
    groups: Vec<String>,
    username: String,
    email: String,
    workspace: String,
}

use crate::db::ApiAuthed;
use crate::users::Tokened;

#[cfg(feature = "enterprise")]
pub async fn gen_token(
    authed: ApiAuthed,
    token: Tokened,
    Extension(db): Extension<DB>,
    Path((w_id, audience)): Path<(String, String)>,
) -> windmill_common::error::Result<String> {
    use chrono::{Duration, Utc};
    use openidconnect::{
        core::{CoreGenderClaim, CoreJsonWebKeyType, CoreJweContentEncryptionAlgorithm},
        Audience, EndUserEmail, IdToken, IdTokenClaims, StandardClaims, SubjectIdentifier,
    };
    use windmill_queue::get_queued_job;

    use crate::users::get_groups_for_user;

    let private_key = get_private_key(&db).await?;

    let username = authed.username;
    let email = authed.email;

    let job_id = {
        let job = sqlx::query_scalar!("SELECT job FROM token WHERE token = $1", token.token)
            .fetch_optional(&db)
            .await?
            .flatten();
        if job.is_none() {
            return Err(anyhow::anyhow!("Token not found").into());
        } else {
            job.unwrap()
        }
    };
    let job = get_queued_job(job_id, &w_id, &db).await?;

    let job = job.ok_or_else(|| anyhow::anyhow!("Queued job {} not found", job_id))?;
    let issue_url = format!("{}/api/oidc/", crate::BASE_URL.read().await.clone());
    let flow_path = if let Some(uuid) = job.parent_job {
        sqlx::query_scalar!("SELECT script_path FROM queue WHERE id = $1", uuid)
            .fetch_optional(&db)
            .await?
            .flatten()
    } else {
        None
    };

    let groups = get_groups_for_user(&w_id, &username, &email, &db)
        .await
        .ok()
        .unwrap_or_default();

    let id_token = IdToken::<
        JobClaim,
        CoreGenderClaim,
        CoreJweContentEncryptionAlgorithm,
        CoreJwsSigningAlgorithm,
        CoreJsonWebKeyType,
    >::new(
        IdTokenClaims::<JobClaim, CoreGenderClaim>::new(
            // Specify the issuer URL for the OpenID Connect Provider.
            IssuerUrl::new(issue_url)
                .map_err(|e| anyhow::anyhow!("Failed to generate IssueUrl: {}", e))?,
            // The audience is usually a single entry with the client ID of the client for whom
            // the ID token is intended. This is a required claim.
            vec![Audience::new(audience)],
            // The ID token expiration is usually much shorter than that of the access or refresh
            // tokens issued to clients.
            Utc::now() + Duration::hours(48),
            // The issue time is usually the current time.
            Utc::now(),
            // Set the standard claims defined by the OpenID Connect Core spec.
            StandardClaims::new(
                // Stable subject identifiers are recommended in place of e-mail addresses or other
                // potentially unstable identifiers. This is the only required claim.
                SubjectIdentifier::new(format!(
                    "{}::{}::{}::{}",
                    email,
                    job.script_path
                        .clone()
                        .unwrap_or_else(|| "no_path".to_string()),
                    flow_path.clone().unwrap_or_else(|| "no_flow".to_string()),
                    w_id
                )),
            )
            // Optional: specify the user's e-mail address. This should only be provided if the
            // client has been granted the 'profile' or 'email' scopes.
            .set_email(Some(EndUserEmail::new(job.email.clone())))
            // Optional: specify whether the provider has verified the user's e-mail address.
            .set_email_verified(Some(true)),
            // OpenID Connect Providers may supply custom claims by providing a struct that
            // implements the AdditionalClaims trait. This requires manually using the
            // generic IdTokenClaims struct rather than the CoreIdTokenClaims type alias,
            // however.
            JobClaim {
                job_id: job_id.to_string(),
                path: job.script_path,
                flow_path,
                username: job.created_by,
                email: job.email,
                workspace: job.workspace_id,
                groups,
            },
        ),
        // The private key used for signing the ID token. For confidential clients (those able
        // to maintain a client secret), a CoreHmacKey can also be used, in conjunction
        // with one of the CoreJwsSigningAlgorithm::HmacSha* signing algorithms. When using an
        // HMAC-based signing algorithm, the UTF-8 representation of the client secret should
        // be used as the HMAC key.
        &CoreRsaPrivateSigningKey::from_pem(
            &private_key,
            Some(JsonWebKeyId::new("windmill".to_string())),
        )
        .map_err(|e| anyhow::anyhow!("Invalid private key: {}", e))?,
        // Uses the RS256 signature algorithm. This crate supports any RS*, PS*, or HS*
        // signature algorithm.
        CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
        // When returning the ID token alongside an access token (e.g., in the Authorization Code
        // flow), it is recommended to pass the access token here to set the `at_hash` claim
        // automatically.
        None,
        // When returning the ID token alongside an authorization code (e.g., in the implicit
        // flow), it is recommended to pass the authorization code here to set the `c_hash` claim
        // automatically.
        None,
    )
    .map_err(|e| anyhow::anyhow!("Failed to generate token: {}", e))?;
    Ok(id_token.to_string())
}
