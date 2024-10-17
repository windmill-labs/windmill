/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(feature = "openidconnect")]
use anyhow;
#[cfg(feature = "openidconnect")]
use std::process::Command;

#[cfg(feature = "openidconnect")]
use openidconnect::{
    core::{CoreJwsSigningAlgorithm, CoreRsaPrivateSigningKey},
    IssuerUrl, JsonWebKeyId,
};

#[cfg(all(feature = "enterprise", feature = "openidconnect"))]
use openidconnect::{
    core::{
        CoreClaimName, CoreJsonWebKeySet, CoreProviderMetadata, CoreResponseType,
        CoreSubjectIdentifierType,
    },
    AuthUrl, EmptyAdditionalProviderMetadata, JsonWebKeySetUrl, ResponseTypes,
};

#[cfg(feature = "openidconnect")]
use openidconnect::AdditionalClaims;

#[cfg(feature = "openidconnect")]
use crate::db::DB;

#[cfg(all(feature = "enterprise", feature = "openidconnect"))]
use axum::extract::Path;
#[cfg(all(feature = "enterprise", feature = "openidconnect"))]
use axum::routing::{get, post};
#[cfg(all(feature = "enterprise", feature = "openidconnect"))]
use axum::Extension;
#[cfg(all(feature = "enterprise", feature = "openidconnect"))]
use axum::Json;

use axum::Router;
use serde::{Deserialize, Serialize};

#[cfg(all(feature = "enterprise", feature = "openidconnect"))]
pub async fn generate_id_token<T: AdditionalClaims>(
    db: &DB,
    claim: T,
    audience: String,
    identifier: String,
    email: Option<String>,
) -> crate::error::Result<String> {
    use chrono::{Duration, Utc};
    use openidconnect::{
        core::{CoreGenderClaim, CoreJsonWebKeyType, CoreJweContentEncryptionAlgorithm},
        Audience, EndUserEmail, IdToken, IdTokenClaims, StandardClaims, SubjectIdentifier,
    };

    let private_key = get_private_key(&db).await?;
    let issue_url = format!("{}/api/oidc/", crate::BASE_URL.read().await.clone());

    let id_token = IdToken::<
        T,
        CoreGenderClaim,
        CoreJweContentEncryptionAlgorithm,
        CoreJwsSigningAlgorithm,
        CoreJsonWebKeyType,
    >::new(
        IdTokenClaims::<T, CoreGenderClaim>::new(
            // Specify the issuer URL for the OpenID Connect Provider.
            IssuerUrl::new(issue_url)
                .map_err(|e| anyhow::anyhow!("Failed to generate IssueUrl: {}", e))?,
            // The audience is usually a single entry with the client ID of the client for whom
            // the ID token is intended. This is a required claim.
            vec![Audience::new(audience)],
            // The ID token expiration is usually much shorter than that of the access or refresh
            // tokens issued to clients.
            Utc::now() + Duration::try_hours(48).unwrap(),
            // The issue time is usually the current time.
            Utc::now(),
            // Set the standard claims defined by the OpenID Connect Core spec.
            StandardClaims::new(
                // Stable subject identifiers are recommended in place of e-mail addresses or other
                // potentially unstable identifiers. This is the only required claim.
                SubjectIdentifier::new(identifier),
            )
            // Optional: specify the user's e-mail address. This should only be provided if the
            // client has been granted the 'profile' or 'email' scopes.
            .set_email(email.map(|x| EndUserEmail::new(x)))
            // Optional: specify whether the provider has verified the user's e-mail address.
            .set_email_verified(Some(true)),
            // OpenID Connect Providers may supply custom claims by providing a struct that
            // implements the AdditionalClaims trait. This requires manually using the
            // generic IdTokenClaims struct rather than the CoreIdTokenClaims type alias,
            // however.
            claim,
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

#[cfg(feature = "openidconnect")]
pub async fn get_private_key(db: &DB) -> anyhow::Result<String> {
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

#[cfg(feature = "openidconnect")]
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

#[derive(Debug, Clone, serde::Serialize)]
struct Keys {
    private_key: String,
}
