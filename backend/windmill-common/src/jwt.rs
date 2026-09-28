use crate::error::{self, to_anyhow, Error};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use serde::{de::DeserializeOwned, Serialize};
use sha2::Sha256;

lazy_static::lazy_static! {
    pub static ref JWT_SECRET: arc_swap::ArcSwap<String> = arc_swap::ArcSwap::from_pointee("".to_string());
}

pub async fn encode_with_internal_secret<T: Serialize>(claims: T) -> error::Result<String> {
    let jwt_secret = JWT_SECRET.load();

    if jwt_secret.is_empty() {
        return Err(Error::internal_err("JWT secret is not set".to_string()));
    }

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(to_anyhow)?;

    Ok(token)
}

pub async fn decode_with_internal_secret<T: DeserializeOwned>(token: &str) -> error::Result<T> {
    let jwt_secret = JWT_SECRET.load();

    if jwt_secret.is_empty() {
        return Err(Error::internal_err("JWT secret is not set".to_string()));
    }

    let result = jsonwebtoken::decode::<T>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(jwt_secret.as_bytes()),
        &jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256),
    )
    .map_err(to_anyhow)?;

    Ok(result.claims)
}

pub fn decode_without_verify<T: DeserializeOwned>(token: &str) -> anyhow::Result<T> {
    // No signature, expiry, nbf or required-claim checks: the caller wants the raw claims.
    let token_data = jsonwebtoken::dangerous::insecure_decode::<T>(token)?;

    Ok(token_data.claims)
}

// header_and_payload: `{header}.{payload}`
pub async fn generate_signature(header_and_payload: &str) -> anyhow::Result<String> {
    let header_and_payload = header_and_payload.trim_start_matches("jwt_ext_");
    let header_and_payload = header_and_payload.trim_start_matches("jwt_");
    let secret = JWT_SECRET.load();

    // Create HMAC-SHA256
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())?;
    mac.update(header_and_payload.as_bytes());

    // Finalize and encode
    let result = mac.finalize().into_bytes();
    Ok(URL_SAFE_NO_PAD.encode(result))
}

/// The signing algorithm a JWK names in its `alg`, or `None` when it names none (or one
/// that is not a signing algorithm, e.g. `RSA-OAEP`). The only place that reads the
/// field, so callers, including EE ones, stay agnostic of how `jsonwebtoken` models it.
pub fn jwk_algorithm(jwk: &jsonwebtoken::jwk::Jwk) -> Option<jsonwebtoken::Algorithm> {
    // `alg` is a `KeyAlgorithm` (signing and encryption algorithms alike); only the
    // signing subset maps onto `Algorithm`.
    jwk.common
        .key_algorithm
        .and_then(|alg| alg.to_string().parse::<jsonwebtoken::Algorithm>().ok())
}

#[cfg(test)]
mod tests {
    use super::jwk_algorithm;
    use jsonwebtoken::{jwk::Jwk, Algorithm};

    #[test]
    fn jwk_algorithm_reads_alg_when_present() {
        let with: Jwk =
            serde_json::from_value(serde_json::json!({"kty":"RSA","alg":"RS256","n":"aa","e":"AQAB"}))
                .unwrap();
        assert_eq!(jwk_algorithm(&with), Some(Algorithm::RS256));
        let without: Jwk =
            serde_json::from_value(serde_json::json!({"kty":"RSA","n":"aa","e":"AQAB"})).unwrap();
        assert_eq!(jwk_algorithm(&without), None);
        let encryption: Jwk = serde_json::from_value(
            serde_json::json!({"kty":"RSA","alg":"RSA-OAEP","n":"aa","e":"AQAB"}),
        )
        .unwrap();
        assert_eq!(jwk_algorithm(&encryption), None);
    }
}
