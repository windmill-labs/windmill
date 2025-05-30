/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
#[cfg(all(feature = "enterprise", feature = "openidconnect"))]
use {
    crate::db::DB,
    crate::{auth::IdToken as WindmillIdToken, error::Result},
    anyhow,
    openidconnect::{
        core::{CoreJwsSigningAlgorithm, CoreRsaPrivateSigningKey},
        IssuerUrl, JsonWebKeyId,
    },
    std::process::Command,
};

#[cfg(feature = "openidconnect")]
use openidconnect::AdditionalClaims;

#[cfg(feature = "openidconnect")]
impl AdditionalClaims for JobClaim {}

#[cfg(feature = "openidconnect")]
impl AdditionalClaims for WorkspaceClaim {}

#[cfg(feature = "openidconnect")]
impl AdditionalClaims for InstanceClaim {}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct WorkspaceClaim {
    pub workspace: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct InstanceClaim {}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct JobClaim {
    pub job_id: String,
    pub path: Option<String>,
    pub flow_path: Option<String>,
    pub groups: Vec<String>,
    pub username: String,
    pub email: String,
    pub workspace: String,
}

lazy_static::lazy_static! {
    static ref PRIVATE_KEY: RwLock<Option<String>> = RwLock::new(None);
}

pub async fn generate_id_token<T: AdditionalClaims>(
    db: Option<&DB>,
    claim: T,
    audience: &str,
    identifier: String,
    email: Option<String>,
) -> Result<WindmillIdToken> {
    use chrono::{Duration, Utc};
    use openidconnect::{
        core::{CoreGenderClaim, CoreJweContentEncryptionAlgorithm},
        Audience, EndUserEmail, IdToken, IdTokenClaims, StandardClaims, SubjectIdentifier,
    };

    let private_key = get_private_key(db).await?;

    let issue_url = format!("{}/api/oidc/", crate::BASE_URL.read().await.clone());
    let issue_time = Utc::now();
    let expiration = issue_time + Duration::try_hours(48).unwrap();
    let id_token = IdToken::<
        T,
        CoreGenderClaim,
        CoreJweContentEncryptionAlgorithm,
        CoreJwsSigningAlgorithm,
    >::new(
        IdTokenClaims::<T, CoreGenderClaim>::new(
            // Specify the issuer URL for the OpenID Connect Provider.
            IssuerUrl::new(issue_url)
                .map_err(|e| anyhow::anyhow!("Failed to generate IssueUrl: {}", e))?,
            // The audience is usually a single entry with the client ID of the client for whom
            // the ID token is intended. This is a required claim.
            vec![Audience::new(audience.to_string())],
            // The ID token expiration is usually much shorter than that of the access or refresh
            // tokens issued to clients.
            expiration,
            // The issue time is usually the current time.
            issue_time,
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

    Ok(WindmillIdToken::new(id_token.to_string(), expiration))
}

#[cfg(all(feature = "enterprise", feature = "openidconnect"))]
pub async fn get_private_key(db: Option<&DB>) -> anyhow::Result<String> {
    if let Some(key) = PRIVATE_KEY.read().await.clone() {
        return Ok(key);
    } else if let Some(db) = db {
        let key = sqlx::query_scalar!(
            "SELECT value->>'private_key' FROM global_settings WHERE name = 'rsa_keys'",
        )
        .fetch_optional(db)
        .await?
        .flatten();

        let key = key.filter(|s| !s.is_empty());

        if let Some(key) = key {
            return Ok(key);
        } else {
            let keys = gen_pems(db).await?;
            return Ok(keys.private_key);
        }
    } else {
        return Err(anyhow::anyhow!("Private key not found and no db provided"));
    }
}

#[cfg(all(feature = "enterprise", feature = "openidconnect"))]
#[derive(Debug, Clone, serde::Serialize)]
struct Keys {
    private_key: String,
}

#[cfg(all(feature = "enterprise", feature = "openidconnect"))]
async fn gen_pems(db: &DB) -> anyhow::Result<Keys> {
    use anyhow::anyhow;

    let private_key_cmd = Command::new("openssl")
        .arg("genrsa")
        .arg("--traditional")
        .arg("2048")
        .output()
        .expect("failed to execute process");

    let private_key = String::from_utf8(private_key_cmd.stdout)?;

    tracing::debug!("Generated private key: {}", private_key);

    if private_key.is_empty() {
        return Err(anyhow!("Failed to generate RSA key: key is empty"));
    }

    let keys = Keys { private_key };

    sqlx::query!(
        r#"INSERT INTO global_settings (name, value) VALUES ('rsa_keys', $1)"#,
        serde_json::to_value(&keys).unwrap()
    )
    .execute(db)
    .await?;

    Ok(keys)
}
