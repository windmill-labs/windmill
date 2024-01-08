/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2023
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

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

use axum::routing::get;
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

#[cfg(feature = "enterprise")]
pub async fn jwks() -> windmill_common::error::JsonResult<CoreJsonWebKeySet> {
    use openidconnect::PrivateSigningKey;

    let jwks = CoreJsonWebKeySet::new(vec![
        // RSA keys may also be constructed directly using CoreJsonWebKey::new_rsa(). Providers
        // aiming to support other key types may provide their own implementation of the
        // JsonWebKey trait or submit a PR to add the desired support to this crate.
        CoreRsaPrivateSigningKey::from_pem("foo", Some(JsonWebKeyId::new("key1".to_string())))
            .expect("Invalid RSA private key")
            .as_verification_key(),
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
        IssuerUrl::new(format!("{base_url}/oidc/"))?,
        AuthUrl::new(format!("{base_url}/oidc/"))?,
        JsonWebKeySetUrl::new(format!("{base_url}/oidc/jwks"))?,
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
