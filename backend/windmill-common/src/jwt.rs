use crate::error::{self, to_anyhow, Error};
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashSet, sync::Arc};
use tokio::sync::RwLock;

lazy_static::lazy_static! {
    pub static ref JWT_SECRET: Arc<RwLock<String>> = Arc::new(RwLock::new("".to_string()));
}

pub async fn encode_with_internal_secret<T: Serialize>(claims: T) -> error::Result<String> {
    let jwt_secret = JWT_SECRET.read().await;

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
    let jwt_secret = JWT_SECRET.read().await;

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
    // Create a validation that skips all checks
    let mut validation = jsonwebtoken::Validation::default();
    validation.insecure_disable_signature_validation();
    validation.validate_exp = false;
    validation.validate_nbf = false;
    validation.required_spec_claims = HashSet::new();

    // Use an empty key since we're not verifying
    let key = jsonwebtoken::DecodingKey::from_secret(&[]);

    // Decode the token
    let token_data = jsonwebtoken::decode::<T>(token, &key, &validation)?;

    Ok(token_data.claims)
}
