/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::process::Command;

use anyhow;
use openidconnect::core::{
    CoreClaimName, CoreJwsSigningAlgorithm, CoreProviderMetadata, CoreResponseType,
    CoreSubjectIdentifierType,
};
use openidconnect::{
    AuthUrl, EmptyAdditionalProviderMetadata, IssuerUrl, JsonWebKeySetUrl, ResponseTypes,
};

use openidconnect::{
    core::{CoreJsonWebKeySet, CoreRsaPrivateSigningKey},
    JsonWebKeyId,
};

use crate::db::DB;
use axum::routing::get;
use axum::Extension;
use axum::{Json, Router};

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

pub fn generate_pems() {
    let public_key = Command::new("openssl")
        .arg("genrsa")
        .arg("3072")
        .output()
        .expect("failed to execute process");

    let private_key = Command::new("openssl")
        .arg("rsa")
        .arg("-pubout")
        .output()
        .expect("failed to execute process");
    //     # generate a private key with the correct length
    // openssl genrsa -out private-key.pem 3072

    // # generate corresponding public key
    // openssl rsa -in private-key.pem -pubout -out public-key.pem
}
#[cfg(feature = "enterprise")]
pub async fn jwks(
    Extension(db): Extension<DB>,
) -> windmill_common::error::JsonResult<CoreJsonWebKeySet> {
    use openidconnect::{core::CoreJsonWebKey, PrivateSigningKey};
    use windmill_common::utils::get_uid;

    let uid = get_uid(&db).await?;

    let jwks = CoreJsonWebKeySet::new(vec![
        CoreJsonWebKey::new_rsa(vec![], vec![], None), // // RSA keys may also be constructed directly using CoreJsonWebKey::new_rsa(). Providers
                                                       // // aiming to support other key types may provide their own implementation of the
                                                       // // JsonWebKey trait or submit a PR to add the desired support to this crate.
                                                       // CoreRsaPrivateSigningKey::from_pem(
                                                       //     &pem.to_string(),
                                                       //     Some(JsonWebKeyId::new("key1".to_string())),
                                                       // )
                                                       // .map_err(|e| anyhow::anyhow!("Failed to parse PEM: {}", e))?
                                                       // .as_verification_key(),
    ]);

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
        CoreClaimName::new("name".to_string()),
        CoreClaimName::new("given_name".to_string()),
        CoreClaimName::new("family_name".to_string()),
        CoreClaimName::new("picture".to_string()),
        CoreClaimName::new("locale".to_string()),
    ]));
    return Ok(provider_metadata);
}

#[cfg(feature = "enterprise")]
pub fn gen_token() {
    use chrono::Utc;
    use openidconnect::{
        core::{CoreIdToken, CoreIdTokenClaims},
        Audience, EmptyAdditionalClaims, EndUserEmail, StandardClaims, SubjectIdentifier,
    };
    use time::Duration;

    let id_token = CoreIdToken::new(
        CoreIdTokenClaims::new(
            // Specify the issuer URL for the OpenID Connect Provider.
            IssuerUrl::new("https://accounts.example.com".to_string())?,
            // The audience is usually a single entry with the client ID of the client for whom
            // the ID token is intended. This is a required claim.
            vec![Audience::new("client-id-123".to_string())],
            // The ID token expiration is usually much shorter than that of the access or refresh
            // tokens issued to clients.
            Utc::now() + Duration::seconds(300),
            // The issue time is usually the current time.
            Utc::now(),
            // Set the standard claims defined by the OpenID Connect Core spec.
            StandardClaims::new(
                // Stable subject identifiers are recommended in place of e-mail addresses or other
                // potentially unstable identifiers. This is the only required claim.
                SubjectIdentifier::new("5f83e0ca-2b8e-4e8c-ba0a-f80fe9bc3632".to_string()),
            )
            // Optional: specify the user's e-mail address. This should only be provided if the
            // client has been granted the 'profile' or 'email' scopes.
            .set_email(Some(EndUserEmail::new("bob@example.com".to_string())))
            // Optional: specify whether the provider has verified the user's e-mail address.
            .set_email_verified(Some(true)),
            // OpenID Connect Providers may supply custom claims by providing a struct that
            // implements the AdditionalClaims trait. This requires manually using the
            // generic IdTokenClaims struct rather than the CoreIdTokenClaims type alias,
            // however.
            EmptyAdditionalClaims {},
        ),
        // The private key used for signing the ID token. For confidential clients (those able
        // to maintain a client secret), a CoreHmacKey can also be used, in conjunction
        // with one of the CoreJwsSigningAlgorithm::HmacSha* signing algorithms. When using an
        // HMAC-based signing algorithm, the UTF-8 representation of the client secret should
        // be used as the HMAC key.
        &CoreRsaPrivateSigningKey::from_pem(&rsa_pem, Some(JsonWebKeyId::new("key1".to_string())))
            .expect("Invalid RSA private key"),
        // Uses the RS256 signature algorithm. This crate supports any RS*, PS*, or HS*
        // signature algorithm.
        CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
        // When returning the ID token alongside an access token (e.g., in the Authorization Code
        // flow), it is recommended to pass the access token here to set the `at_hash` claim
        // automatically.
        Some(&access_token),
        // When returning the ID token alongside an authorization code (e.g., in the implicit
        // flow), it is recommended to pass the authorization code here to set the `c_hash` claim
        // automatically.
        None,
    )?;
}
