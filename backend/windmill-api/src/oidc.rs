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

use axum::{Json, Router};

#[cfg(feature = "enterprise")]
pub fn global_service() -> Router {
    use axum::routing::get;

    Router::new().route(
        "/.well-known/openid-configuration",
        get(get_provider_metadata),
    )
}

#[cfg(feature = "enterprise")]
pub fn get_provider_metadata() -> windmill_common::error::JsonResult<CoreProviderMetadata> {
    let provider_metadata = CoreProviderMetadata::new(
        // Parameters required by the OpenID Connect Discovery spec.
        IssuerUrl::new("https://accounts.example.com".to_string())?,
        AuthUrl::new("https://accounts.example.com/authorize".to_string())?,
        // Use the JsonWebKeySet struct to serve the JWK Set at this URL.
        JsonWebKeySetUrl::new("https://accounts.example.com/jwk".to_string())?,
        // Supported response types (flows).
        vec![
            // Optional: support the implicit flow.
            ResponseTypes::new(vec![CoreResponseType::Token, CoreResponseType::IdToken]), // Other flows including hybrid flows may also be specified here.
        ],
        // For user privacy, the Pairwise subject identifier type is preferred. This prevents
        // distinct relying parties (clients) from knowing whether their users represent the same
        // real identities. This identifier type is only useful for relying parties that don't
        // receive the 'email', 'profile' or other personally-identifying scopes.
        // The Public subject identifier type is also supported.
        vec![CoreSubjectIdentifierType::Public],
        // Support the RS256 signature algorithm.
        vec![CoreJwsSigningAlgorithm::RsaSsaPssSha256],
        // OpenID Connect Providers may supply custom metadata by providing a struct that
        // implements the AdditionalProviderMetadata trait. This requires manually using the
        // generic ProviderMetadata struct rather than the CoreProviderMetadata type alias,
        // however.
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
    return Ok(Json(provider_metadata));
}
