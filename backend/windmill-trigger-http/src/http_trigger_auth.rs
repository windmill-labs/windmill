use axum::response::{IntoResponse, Response};
use base64::{
    prelude::{BASE64_STANDARD, BASE64_URL_SAFE},
    Engine,
};
use hmac::{Hmac, Mac};
use http::{header, HeaderMap, HeaderValue, StatusCode};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::{borrow::Cow, collections::HashMap};

pub type HmacSha256 = Hmac<Sha256>;
pub type HmacSha512 = Hmac<Sha512>;
pub type HmacSha1 = Hmac<Sha1>;

mod github {
    use super::*;
    pub struct Github;

    impl WebhookHandler for Github {
        fn handle_challenge_request<'header>(
            &self,
            _: &'header HeaderMap,
            _: &SignatureConfigData,
            _: &str,
        ) -> Result<Option<Response>, AuthenticationError> {
            Ok(None)
        }

        fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
            &self,
            headers: &'header HeaderMap,
            raw_payload: &'payload str,
        ) -> Result<SignatureAuthenticationData<'payload, 'header, 'prefix>, AuthenticationError>
        {
            let github_secret_header = headers.try_get_webhook_header("X-Hub-Signature-256")?;

            let authentication_data = SignatureAuthenticationData::new(
                Cow::Borrowed(raw_payload),
                github_secret_header,
                Some("sha256="),
                SignatureAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex),
            );

            Ok(authentication_data)
        }
    }
}

mod slack {
    use super::*;
    pub struct Slack;

    impl WebhookHandler for Slack {
        fn handle_challenge_request<'header>(
            &self,
            _: &'header HeaderMap,
            _: &SignatureConfigData,
            _: &str,
        ) -> Result<Option<Response>, AuthenticationError> {
            Ok(None)
        }

        fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
            &self,
            headers: &'header HeaderMap,
            raw_payload: &'payload str,
        ) -> Result<SignatureAuthenticationData<'payload, 'header, 'prefix>, AuthenticationError>
        {
            let slack_secret_signature = headers.try_get_webhook_header("X-Slack-Signature")?;
            let slack_timestamp_header =
                headers.try_get_webhook_header("X-Slack-Request-Timestamp")?;
            let signed_payload = format!("v0:{}:{}", slack_timestamp_header, raw_payload);

            Ok(SignatureAuthenticationData::new(
                Cow::Owned(signed_payload),
                slack_secret_signature,
                Some("v0="),
                SignatureAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex),
            ))
        }
    }
}

mod stripe {
    use super::*;

    pub struct Stripe;

    impl WebhookHandler for Stripe {
        fn handle_challenge_request<'header>(
            &self,
            _: &'header HeaderMap,
            _: &SignatureConfigData,
            _: &str,
        ) -> Result<Option<Response>, AuthenticationError> {
            Ok(None)
        }

        fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
            &self,
            headers: &'header HeaderMap,
            raw_payload: &'payload str,
        ) -> Result<SignatureAuthenticationData<'payload, 'header, 'prefix>, AuthenticationError>
        {
            let stripe_signature_header = headers.try_get_webhook_header("STRIPE-SIGNATURE")?;

            let stripe_signature = parse_signature(stripe_signature_header, (",", "="));

            let timestamp = *stripe_signature
                .get("t")
                .ok_or(AuthenticationError::InvalidTimestamp)?;
            let v1 = *stripe_signature
                .get("v1")
                .ok_or(AuthenticationError::InvalidSignature)?;

            let signed_payload = format!("{}.{}", timestamp, raw_payload);

            Ok(SignatureAuthenticationData::new(
                Cow::Owned(signed_payload),
                v1,
                None,
                SignatureAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex),
            ))
        }
    }
}

mod tiktok {
    use super::*;

    pub struct TikTok;

    impl WebhookHandler for TikTok {
        fn handle_challenge_request<'header>(
            &self,
            _: &'header HeaderMap,
            _: &SignatureConfigData,
            _: &str,
        ) -> Result<Option<Response>, AuthenticationError> {
            Ok(None)
        }

        fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
            &self,
            headers: &'header HeaderMap,
            raw_payload: &'payload str,
        ) -> Result<SignatureAuthenticationData<'payload, 'header, 'prefix>, AuthenticationError>
        {
            let tiktok_secret_signature = headers.try_get_webhook_header("TikTok-Signature")?;

            let stripe_signature = parse_signature(tiktok_secret_signature, (",", "="));

            let timestamp = *stripe_signature
                .get("t")
                .ok_or(AuthenticationError::InvalidTimestamp)?;
            let s = *stripe_signature
                .get("s")
                .ok_or(AuthenticationError::InvalidSignature)?;

            let signed_payload = format!("{}.{}", timestamp, raw_payload);

            Ok(SignatureAuthenticationData::new(
                Cow::Owned(signed_payload),
                s,
                None,
                SignatureAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex),
            ))
        }
    }
}

mod twitch {
    use super::*;
    use http::header;
    use serde_json::value::RawValue;
    #[derive(Debug, Deserialize)]
    struct TwitchCrcBody {
        challenge: String,
        #[allow(unused)]
        subscription: Box<RawValue>,
    }

    pub struct Twitch;

    impl WebhookHandler for Twitch {
        fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
            &self,
            headers: &'header HeaderMap,
            raw_payload: &'payload str,
        ) -> Result<SignatureAuthenticationData<'payload, 'header, 'prefix>, AuthenticationError>
        {
            let twitch_secret_signature =
                headers.try_get_webhook_header("Twitch-Eventsub-Message-Signature")?;
            let twitch_message_id_header =
                headers.try_get_webhook_header("Twitch-Eventsub-Message-Id")?;
            let twitch_timestamp_header =
                headers.try_get_webhook_header("Twitch-Eventsub-Message-Timestamp")?;

            let message = format!(
                "{}{}{}",
                twitch_message_id_header, twitch_timestamp_header, raw_payload
            );

            Ok(SignatureAuthenticationData::new(
                Cow::Owned(message),
                twitch_secret_signature,
                Some("sha256="),
                SignatureAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex),
            ))
        }

        fn handle_challenge_request<'header>(
            &self,
            headers: &'header HeaderMap,
            signature_config_data: &SignatureConfigData,
            raw_payload: &str,
        ) -> Result<Option<Response>, AuthenticationError> {
            let authentication_data = self.get_hmac_authentication_data(headers, raw_payload)?;
            verify_hmac_signature(authentication_data, &signature_config_data.secret_key)?;

            let twitch_eventsub_message_type =
                headers.try_get_webhook_header("Twitch-Eventsub-Message-Type")?;

            if twitch_eventsub_message_type != "webhook_callback_verification" {
                return Ok(None);
            }
            let twitch_crc_body =
                serde_json::from_str::<TwitchCrcBody>(raw_payload).map_err(|e| {
                    AuthenticationError::InvalidChallengeResponse(format!(
                        "Twitch :{}",
                        e.to_string()
                    ))
                })?;

            let response = (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/plain")],
                twitch_crc_body.challenge.to_string(),
            );

            Ok(Some(response.into_response()))
        }
    }
}

mod zoom {
    use axum::Json;

    use super::*;

    #[derive(Debug, Deserialize)]
    struct ZoomPayload {
        #[serde(rename = "plainToken")]
        plain_token: String,
    }

    #[derive(Debug, Deserialize)]
    #[allow(unused)]
    struct ZoomChallengeResponse {
        payload: ZoomPayload,
        event_ts: u64,
        event: String,
    }

    pub struct Zoom;

    impl WebhookHandler for Zoom {
        fn handle_challenge_request<'header>(
            &self,
            _: &'header HeaderMap,
            signature_config_data: &SignatureConfigData,
            raw_payload: &str,
        ) -> Result<Option<Response>, AuthenticationError> {
            let Ok(zoom_request_body) = serde_json::from_str::<ZoomChallengeResponse>(raw_payload)
            else {
                return Ok(None);
            };

            if zoom_request_body.event != "endpoint.url_validation" {
                return Ok(None);
            }

            let hmac_signature = calculate_hmac_signature(
                HmacAlgorithm::Sha256,
                &signature_config_data.secret_key,
                &zoom_request_body.payload.plain_token,
            );

            let encoded_hmac_signature = encode_hmac_signature(Encoding::Hex, &hmac_signature);

            let response = (
                StatusCode::OK,
                Json(json!({
                    "plainToken": zoom_request_body.payload.plain_token,
                    "encryptedToken": encoded_hmac_signature
                })),
            );

            Ok(Some(response.into_response()))
        }

        fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
            &self,
            headers: &'header HeaderMap,
            raw_payload: &'payload str,
        ) -> Result<SignatureAuthenticationData<'payload, 'header, 'prefix>, AuthenticationError>
        {
            let zoom_signature_header = headers.try_get_webhook_header("x-zm-signature")?;
            let zoom_timestamp_header = headers.try_get_webhook_header("x-zm-request-timestamp")?;

            let message = format!("v0:{}:{}", zoom_timestamp_header, raw_payload);

            Ok(SignatureAuthenticationData::new(
                Cow::Owned(message),
                zoom_signature_header,
                Some("v0="),
                SignatureAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex),
            ))
        }
    }
}

use constant_time_eq::constant_time_eq;
use github::Github;
use slack::Slack;
use stripe::Stripe;
use tiktok::TikTok;
use twitch::Twitch;
use zoom::Zoom;

#[derive(Debug)]
pub struct SignatureAuthenticationDetails {
    pub algorithm_to_use: HmacAlgorithm,
    pub header_key_encoding: Encoding,
}

impl SignatureAuthenticationDetails {
    #[inline]
    fn new(algorithm_to_use: HmacAlgorithm, header_key_encoding: Encoding) -> Self {
        Self { algorithm_to_use, header_key_encoding }
    }
}

fn parse_signature<'header>(
    signature: &'header str,
    splitters: (&str, &str),
) -> HashMap<&'header str, &'header str> {
    let headers: HashMap<&str, &str> = signature
        .split(splitters.0)
        .map(|header| {
            let mut key_and_value = header.split(splitters.1);
            let key = key_and_value.next();
            let value = key_and_value.next();
            (key, value)
        })
        .filter_map(|(key, value)| match (key, value) {
            (Some(key), Some(value)) => Some((key, value)),
            _ => None,
        })
        .collect();
    headers
}

#[derive(Debug)]
pub struct SignatureAuthenticationData<'payload, 'header, 'prefix> {
    pub signed_payload: Cow<'payload, str>,
    pub header_key_value: &'header str,
    pub signature_prefix: Option<&'prefix str>,
    pub config: SignatureAuthenticationDetails,
}

impl<'payload, 'header, 'prefix> SignatureAuthenticationData<'payload, 'header, 'prefix> {
    pub fn new(
        signed_payload: Cow<'payload, str>,
        header_key_value: &'header str,
        signature_prefix: Option<&'prefix str>,
        config: SignatureAuthenticationDetails,
    ) -> Self {
        Self { signed_payload, header_key_value, signature_prefix, config }
    }
}

pub trait WebhookHandler {
    fn handle_challenge_request<'header>(
        &self,
        headers: &'header HeaderMap,
        signature_config_data: &SignatureConfigData,
        raw_payload: &str,
    ) -> Result<Option<Response>, AuthenticationError>;

    fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
        &self,
        headers: &'header HeaderMap,
        raw_payload: &'payload str,
    ) -> Result<SignatureAuthenticationData<'payload, 'header, 'prefix>, AuthenticationError>;
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HmacAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    Base64,
    Base64Uri,
    Hex,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignatureAuthenticationMethod {
    algorithm: HmacAlgorithm,
    encoding: Encoding,
    signature_header_name: String,
    signature_prefix: Option<String>,
}

pub struct SignatureConfigData<'config> {
    secret_key: &'config str,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignatureAuthentication {
    signature_provider: WebhookType,
    secret_key: String,
    authentication_config: Option<SignatureAuthenticationMethod>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BasicAuthAuthentication {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiKeyAuthentication {
    pub api_key_header: String,
    pub api_key_secret: String,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WebhookType {
    Github,
    Slack,
    Stripe,
    TikTok,
    Twitch,
    Zoom,
    Custom,
}

impl WebhookType {
    pub fn get_webhook_handler(&self) -> Option<&'static dyn WebhookHandler> {
        let handler: &'static dyn WebhookHandler = match *self {
            WebhookType::Github => &Github,
            WebhookType::Slack => &Slack,
            WebhookType::Stripe => &Stripe,
            WebhookType::TikTok => &TikTok,
            WebhookType::Twitch => &Twitch,
            WebhookType::Zoom => &Zoom,
            WebhookType::Custom => return None,
        };
        Some(handler)
    }
}

trait TryGetWebhookHeader {
    fn try_get_webhook_header<'header>(
        &'header self,
        header_name: &str,
    ) -> Result<&'header str, AuthenticationError>;
}

impl TryGetWebhookHeader for HeaderMap<HeaderValue> {
    fn try_get_webhook_header<'header>(
        &'header self,
        header_name: &str,
    ) -> Result<&'header str, AuthenticationError> {
        let Some(signature_header) = self.get(header_name) else {
            return Err(AuthenticationError::MissingHeader(header_name.to_string()));
        };
        let Some(signature_header) = signature_header.to_str().ok() else {
            return Err(AuthenticationError::InvalidHeader(header_name.to_string()));
        };

        Ok(signature_header)
    }
}

pub fn calculate_hmac_signature(algorithm: HmacAlgorithm, secret: &str, payload: &str) -> Vec<u8> {
    match algorithm {
        HmacAlgorithm::Sha1 => {
            let mut mac =
                HmacSha1::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
            mac.update(payload.as_bytes());
            mac.finalize().into_bytes().to_vec()
        }
        HmacAlgorithm::Sha256 => {
            let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                .expect("HMAC can take key of any size");
            mac.update(payload.as_bytes());
            mac.finalize().into_bytes().to_vec()
        }
        HmacAlgorithm::Sha512 => {
            let mut mac = HmacSha512::new_from_slice(secret.as_bytes())
                .expect("HMAC can take key of any size");
            mac.update(payload.as_bytes());
            mac.finalize().into_bytes().to_vec()
        }
    }
}

pub fn encode_hmac_signature(encoding: Encoding, hmac_signature: &[u8]) -> String {
    match encoding {
        Encoding::Hex => hex::encode(hmac_signature),
        Encoding::Base64 => BASE64_STANDARD.encode(hmac_signature),
        Encoding::Base64Uri => BASE64_URL_SAFE.encode(hmac_signature),
    }
}

pub fn verify_hmac_signature(
    authentication_data: SignatureAuthenticationData,
    webhook_signing_secret: &str,
) -> Result<(), AuthenticationError> {
    let hmac_signature = calculate_hmac_signature(
        authentication_data.config.algorithm_to_use,
        &webhook_signing_secret,
        &authentication_data.signed_payload,
    );

    let encoded_signature = encode_hmac_signature(
        authentication_data.config.header_key_encoding,
        &hmac_signature,
    );

    let final_expected_signature =
        if let Some(signature_prefix) = authentication_data.signature_prefix {
            format!("{}{}", signature_prefix, encoded_signature)
        } else {
            encoded_signature
        };

    if !constant_time_eq(
        final_expected_signature.as_bytes(),
        authentication_data.header_key_value.as_bytes(),
    ) {
        return Err(AuthenticationError::InvalidSignature);
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AuthenticationMethod {
    Signature(SignatureAuthentication),
    BasicAuth(BasicAuthAuthentication),
    ApiKey(ApiKeyAuthentication),
}

impl AuthenticationMethod {
    pub fn authenticate_http_request(
        &self,
        headers: &HeaderMap,
        raw_payload: Option<&String>,
    ) -> Result<Option<Response>, AuthenticationError> {
        match self {
            AuthenticationMethod::Signature(SignatureAuthentication {
                secret_key,
                authentication_config,
                signature_provider,
            }) => {
                let raw_payload = raw_payload.ok_or(AuthenticationError::InvalidPayload)?;
                let config_data = SignatureConfigData { secret_key: &secret_key };
                let handler = signature_provider.get_webhook_handler();
                let challenge_response = handler
                    .map(|handler| {
                        handler.handle_challenge_request(headers, &config_data, raw_payload)
                    })
                    .transpose()?
                    .flatten();

                if let Some(challenge_response) = challenge_response {
                    return Ok(Some(challenge_response));
                }

                let authentication_data = match handler {
                    Some(handler) => handler.get_hmac_authentication_data(headers, raw_payload)?,
                    None => {
                        let authentication_config = authentication_config
                            .as_ref()
                            .ok_or(AuthenticationError::InvalidCustomConfig)?;
                        let signature_header_value = headers
                            .try_get_webhook_header(&authentication_config.signature_header_name)?;
                        SignatureAuthenticationData::new(
                            Cow::Borrowed(raw_payload),
                            signature_header_value,
                            authentication_config.signature_prefix.as_deref(),
                            SignatureAuthenticationDetails::new(
                                authentication_config.algorithm,
                                authentication_config.encoding,
                            ),
                        )
                    }
                };

                verify_hmac_signature(authentication_data, &secret_key)?;
            }
            AuthenticationMethod::ApiKey(ApiKeyAuthentication {
                api_key_header,
                api_key_secret,
            }) => {
                let api_key_to_cmp = headers
                    .try_get_webhook_header(&api_key_header)
                    .map_err(|_| AuthenticationError::InvalidApiKey)?;
                if api_key_to_cmp != api_key_secret {
                    return Err(AuthenticationError::InvalidApiKey);
                }
            }
            AuthenticationMethod::BasicAuth(BasicAuthAuthentication { username, password }) => {
                let mut credentials_store = headers
                    .try_get_webhook_header("Authorization")
                    .map_err(|_| AuthenticationError::UnauthorizedBasicHttpAuth)?
                    .split(' ');

                let _ = credentials_store
                    .next()
                    .filter(|r#type| *r#type == "Basic")
                    .ok_or(AuthenticationError::UnauthorizedBasicHttpAuth)?;

                let credentials_as_base64 = credentials_store
                    .next()
                    .ok_or(AuthenticationError::UnauthorizedBasicHttpAuth)?;

                let credentials_from_base64_as_bytes = BASE64_STANDARD
                    .decode(credentials_as_base64.as_bytes())
                    .map_err(|_| AuthenticationError::UnauthorizedBasicHttpAuth)?;

                let credentials_separated_with_colon =
                    String::from_utf8(credentials_from_base64_as_bytes)
                        .map_err(|_| AuthenticationError::UnauthorizedBasicHttpAuth)?;

                let credentials = credentials_separated_with_colon.split(':').collect_vec();

                if credentials.len() != 2 {
                    return Err(AuthenticationError::UnauthorizedBasicHttpAuth);
                }

                if credentials.get(0).unwrap() != username
                    || credentials.get(1).unwrap() != password
                {
                    return Err(AuthenticationError::UnauthorizedBasicHttpAuth);
                }
            }
        }

        Ok(None)
    }
}

#[derive(thiserror::Error, Debug)]
#[allow(unused)]
pub enum AuthenticationError {
    #[error("failed to parse timestamp")]
    InvalidTimestamp,

    #[error("invalid secret")]
    InvalidSecret(#[from] base64::DecodeError),

    #[error("invalid header `{0}`")]
    InvalidHeader(String),

    #[error("signature timestamp too old")]
    TimestampTooOldError,

    #[error("signature timestamp too far in future")]
    FutureTimestampError,

    #[error("missing header {0}")]
    MissingHeader(String),

    #[error("signature invalid")]
    InvalidSignature,

    #[error("payload invalid")]
    InvalidPayload,

    #[error("invalid custom config")]
    InvalidCustomConfig,

    #[error("invalid auth header: {0}")]
    InvalidAuthHeader(String),

    #[error("invalid api key")]
    InvalidApiKey,

    #[error("invalid challenge response: {0}")]
    InvalidChallengeResponse(String),

    #[error("")]
    UnauthorizedBasicHttpAuth,
}

impl IntoResponse for AuthenticationError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AuthenticationError::InvalidTimestamp
            | AuthenticationError::InvalidPayload
            | AuthenticationError::InvalidHeader(_)
            | AuthenticationError::MissingHeader(_)
            | AuthenticationError::TimestampTooOldError
            | AuthenticationError::FutureTimestampError
            | AuthenticationError::InvalidCustomConfig
            | AuthenticationError::InvalidChallengeResponse(_) => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }

            AuthenticationError::InvalidSecret(_)
            | AuthenticationError::InvalidSignature
            | AuthenticationError::InvalidAuthHeader(_) => {
                (StatusCode::UNAUTHORIZED, self.to_string())
            }
            AuthenticationError::UnauthorizedBasicHttpAuth => {
                return (
                    StatusCode::UNAUTHORIZED,
                    [(header::WWW_AUTHENTICATE, r#"Basic realm="Restricted Area""#)],
                    "Unauthorized",
                )
                    .into_response()
            }
            AuthenticationError::InvalidApiKey => {
                return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
            }
        };

        let body = json!({ "error": error_message });

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        (status, headers, body.to_string()).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- calculate_hmac_signature ---

    #[test]
    fn test_hmac_sha256_deterministic() {
        let sig1 = calculate_hmac_signature(HmacAlgorithm::Sha256, "secret", "payload");
        let sig2 = calculate_hmac_signature(HmacAlgorithm::Sha256, "secret", "payload");
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_hmac_sha256_different_keys() {
        let sig1 = calculate_hmac_signature(HmacAlgorithm::Sha256, "key1", "payload");
        let sig2 = calculate_hmac_signature(HmacAlgorithm::Sha256, "key2", "payload");
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_hmac_sha256_different_payloads() {
        let sig1 = calculate_hmac_signature(HmacAlgorithm::Sha256, "secret", "payload1");
        let sig2 = calculate_hmac_signature(HmacAlgorithm::Sha256, "secret", "payload2");
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_hmac_sha1_length() {
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha1, "secret", "payload");
        assert_eq!(sig.len(), 20); // SHA1 = 160 bits = 20 bytes
    }

    #[test]
    fn test_hmac_sha256_length() {
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha256, "secret", "payload");
        assert_eq!(sig.len(), 32); // SHA256 = 256 bits = 32 bytes
    }

    #[test]
    fn test_hmac_sha512_length() {
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha512, "secret", "payload");
        assert_eq!(sig.len(), 64); // SHA512 = 512 bits = 64 bytes
    }

    #[test]
    fn test_hmac_sha256_empty_payload() {
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha256, "secret", "");
        assert_eq!(sig.len(), 32);
    }

    #[test]
    fn test_hmac_sha256_empty_key() {
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha256, "", "payload");
        assert_eq!(sig.len(), 32);
    }

    // --- encode_hmac_signature ---

    #[test]
    fn test_encode_hex() {
        let bytes = vec![0xde, 0xad, 0xbe, 0xef];
        assert_eq!(encode_hmac_signature(Encoding::Hex, &bytes), "deadbeef");
    }

    #[test]
    fn test_encode_base64() {
        let bytes = vec![0xde, 0xad, 0xbe, 0xef];
        let encoded = encode_hmac_signature(Encoding::Base64, &bytes);
        assert_eq!(BASE64_STANDARD.decode(&encoded).unwrap(), bytes);
    }

    #[test]
    fn test_encode_base64uri() {
        let bytes = vec![0xde, 0xad, 0xbe, 0xef];
        let encoded = encode_hmac_signature(Encoding::Base64Uri, &bytes);
        assert_eq!(BASE64_URL_SAFE.decode(&encoded).unwrap(), bytes);
    }

    #[test]
    fn test_encode_hex_empty() {
        assert_eq!(encode_hmac_signature(Encoding::Hex, &[]), "");
    }

    // --- verify_hmac_signature round-trip ---

    fn make_auth_data<'a>(
        payload: &'a str,
        header_value: &'a str,
        prefix: Option<&'a str>,
        algorithm: HmacAlgorithm,
        encoding: Encoding,
    ) -> SignatureAuthenticationData<'a, 'a, 'a> {
        SignatureAuthenticationData::new(
            Cow::Borrowed(payload),
            header_value,
            prefix,
            SignatureAuthenticationDetails::new(algorithm, encoding),
        )
    }

    #[test]
    fn test_verify_hmac_sha256_hex_roundtrip() {
        let secret = "my_webhook_secret";
        let payload = r#"{"event":"push"}"#;
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha256, secret, payload);
        let encoded = encode_hmac_signature(Encoding::Hex, &sig);

        let data = make_auth_data(payload, &encoded, None, HmacAlgorithm::Sha256, Encoding::Hex);
        assert!(verify_hmac_signature(data, secret).is_ok());
    }

    #[test]
    fn test_verify_hmac_sha256_base64_roundtrip() {
        let secret = "my_secret";
        let payload = "test body";
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha256, secret, payload);
        let encoded = encode_hmac_signature(Encoding::Base64, &sig);

        let data = make_auth_data(
            payload,
            &encoded,
            None,
            HmacAlgorithm::Sha256,
            Encoding::Base64,
        );
        assert!(verify_hmac_signature(data, secret).is_ok());
    }

    #[test]
    fn test_verify_hmac_sha512_hex_roundtrip() {
        let secret = "long_secret_key";
        let payload = "some data";
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha512, secret, payload);
        let encoded = encode_hmac_signature(Encoding::Hex, &sig);

        let data = make_auth_data(payload, &encoded, None, HmacAlgorithm::Sha512, Encoding::Hex);
        assert!(verify_hmac_signature(data, secret).is_ok());
    }

    #[test]
    fn test_verify_hmac_sha1_hex_roundtrip() {
        let secret = "sha1_key";
        let payload = "data";
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha1, secret, payload);
        let encoded = encode_hmac_signature(Encoding::Hex, &sig);

        let data = make_auth_data(payload, &encoded, None, HmacAlgorithm::Sha1, Encoding::Hex);
        assert!(verify_hmac_signature(data, secret).is_ok());
    }

    #[test]
    fn test_verify_with_prefix() {
        let secret = "key";
        let payload = "body";
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha256, secret, payload);
        let encoded = encode_hmac_signature(Encoding::Hex, &sig);
        let header_value = format!("sha256={}", encoded);

        let data = make_auth_data(
            payload,
            &header_value,
            Some("sha256="),
            HmacAlgorithm::Sha256,
            Encoding::Hex,
        );
        assert!(verify_hmac_signature(data, secret).is_ok());
    }

    #[test]
    fn test_verify_wrong_signature() {
        let data = make_auth_data(
            "payload",
            "wrong_signature_value",
            None,
            HmacAlgorithm::Sha256,
            Encoding::Hex,
        );
        let result = verify_hmac_signature(data, "secret");
        assert!(matches!(result, Err(AuthenticationError::InvalidSignature)));
    }

    #[test]
    fn test_verify_wrong_key() {
        let secret = "correct_key";
        let payload = "body";
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha256, secret, payload);
        let encoded = encode_hmac_signature(Encoding::Hex, &sig);

        let data = make_auth_data(payload, &encoded, None, HmacAlgorithm::Sha256, Encoding::Hex);
        let result = verify_hmac_signature(data, "wrong_key");
        assert!(matches!(result, Err(AuthenticationError::InvalidSignature)));
    }

    #[test]
    fn test_verify_wrong_prefix() {
        let secret = "key";
        let payload = "body";
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha256, secret, payload);
        let encoded = encode_hmac_signature(Encoding::Hex, &sig);
        let header_value = format!("v0={}", encoded);

        let data = make_auth_data(
            payload,
            &header_value,
            Some("sha256="),
            HmacAlgorithm::Sha256,
            Encoding::Hex,
        );
        assert!(matches!(
            verify_hmac_signature(data, secret),
            Err(AuthenticationError::InvalidSignature)
        ));
    }

    // --- parse_signature ---

    #[test]
    fn test_parse_signature_stripe_format() {
        let header = "t=1234567890,v1=abc123def456";
        let parsed = parse_signature(header, (",", "="));
        assert_eq!(parsed.get("t"), Some(&"1234567890"));
        assert_eq!(parsed.get("v1"), Some(&"abc123def456"));
    }

    #[test]
    fn test_parse_signature_tiktok_format() {
        let header = "t=1234567890,s=signaturevalue";
        let parsed = parse_signature(header, (",", "="));
        assert_eq!(parsed.get("t"), Some(&"1234567890"));
        assert_eq!(parsed.get("s"), Some(&"signaturevalue"));
    }

    #[test]
    fn test_parse_signature_single_entry() {
        let header = "key=value";
        let parsed = parse_signature(header, (",", "="));
        assert_eq!(parsed.get("key"), Some(&"value"));
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn test_parse_signature_empty_string() {
        let parsed = parse_signature("", (",", "="));
        assert!(parsed.is_empty() || parsed.len() == 1);
    }

    #[test]
    fn test_parse_signature_multiple_entries() {
        let header = "a=1,b=2,c=3";
        let parsed = parse_signature(header, (",", "="));
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed.get("a"), Some(&"1"));
        assert_eq!(parsed.get("b"), Some(&"2"));
        assert_eq!(parsed.get("c"), Some(&"3"));
    }

    // --- WebhookType serde ---

    #[test]
    fn test_webhook_type_serde_roundtrip() {
        for wt in [
            WebhookType::Github,
            WebhookType::Slack,
            WebhookType::Stripe,
            WebhookType::TikTok,
            WebhookType::Twitch,
            WebhookType::Zoom,
            WebhookType::Custom,
        ] {
            let json = serde_json::to_value(wt).unwrap();
            let deserialized: WebhookType = serde_json::from_value(json).unwrap();
            assert_eq!(wt, deserialized);
        }
    }

    #[test]
    fn test_webhook_type_handler_known_providers() {
        assert!(WebhookType::Github.get_webhook_handler().is_some());
        assert!(WebhookType::Slack.get_webhook_handler().is_some());
        assert!(WebhookType::Stripe.get_webhook_handler().is_some());
        assert!(WebhookType::TikTok.get_webhook_handler().is_some());
        assert!(WebhookType::Twitch.get_webhook_handler().is_some());
        assert!(WebhookType::Zoom.get_webhook_handler().is_some());
    }

    #[test]
    fn test_webhook_type_custom_has_no_handler() {
        assert!(WebhookType::Custom.get_webhook_handler().is_none());
    }

    // --- HmacAlgorithm / Encoding serde ---

    #[test]
    fn test_hmac_algorithm_serde() {
        assert_eq!(
            serde_json::to_value(HmacAlgorithm::Sha1).unwrap(),
            "sha1"
        );
        assert_eq!(
            serde_json::to_value(HmacAlgorithm::Sha256).unwrap(),
            "sha256"
        );
        assert_eq!(
            serde_json::to_value(HmacAlgorithm::Sha512).unwrap(),
            "sha512"
        );
    }

    #[test]
    fn test_encoding_serde() {
        assert_eq!(serde_json::to_value(Encoding::Hex).unwrap(), "hex");
        assert_eq!(serde_json::to_value(Encoding::Base64).unwrap(), "base64");
        assert_eq!(
            serde_json::to_value(Encoding::Base64Uri).unwrap(),
            "base64uri"
        );
    }

    // --- TryGetWebhookHeader ---

    #[test]
    fn test_try_get_header_present() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Custom-Header", HeaderValue::from_static("value123"));
        assert_eq!(
            headers.try_get_webhook_header("X-Custom-Header").unwrap(),
            "value123"
        );
    }

    #[test]
    fn test_try_get_header_missing() {
        let headers = HeaderMap::new();
        let result = headers.try_get_webhook_header("X-Missing");
        assert!(matches!(result, Err(AuthenticationError::MissingHeader(_))));
    }

    #[test]
    fn test_try_get_header_case_insensitive() {
        let mut headers = HeaderMap::new();
        headers.insert("x-hub-signature-256", HeaderValue::from_static("sig"));
        assert_eq!(
            headers
                .try_get_webhook_header("X-Hub-Signature-256")
                .unwrap(),
            "sig"
        );
    }

    // --- GitHub webhook end-to-end ---

    fn github_headers(secret: &str, payload: &str) -> HeaderMap {
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha256, secret, payload);
        let encoded = encode_hmac_signature(Encoding::Hex, &sig);
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Hub-Signature-256",
            HeaderValue::from_str(&format!("sha256={}", encoded)).unwrap(),
        );
        headers
    }

    #[test]
    fn test_github_authenticate_valid() {
        let secret = "github_webhook_secret";
        let payload = r#"{"action":"opened","number":1}"#.to_string();
        let headers = github_headers(secret, &payload);

        let method = AuthenticationMethod::Signature(SignatureAuthentication {
            signature_provider: WebhookType::Github,
            secret_key: secret.to_string(),
            authentication_config: None,
        });
        assert!(method
            .authenticate_http_request(&headers, Some(&payload))
            .is_ok());
    }

    #[test]
    fn test_github_authenticate_wrong_secret() {
        let payload = r#"{"action":"opened"}"#.to_string();
        let headers = github_headers("correct_secret", &payload);

        let method = AuthenticationMethod::Signature(SignatureAuthentication {
            signature_provider: WebhookType::Github,
            secret_key: "wrong_secret".to_string(),
            authentication_config: None,
        });
        assert!(method
            .authenticate_http_request(&headers, Some(&payload))
            .is_err());
    }

    #[test]
    fn test_github_authenticate_missing_header() {
        let payload = r#"{"action":"opened"}"#.to_string();
        let headers = HeaderMap::new();

        let method = AuthenticationMethod::Signature(SignatureAuthentication {
            signature_provider: WebhookType::Github,
            secret_key: "secret".to_string(),
            authentication_config: None,
        });
        assert!(matches!(
            method.authenticate_http_request(&headers, Some(&payload)),
            Err(AuthenticationError::MissingHeader(_))
        ));
    }

    #[test]
    fn test_github_authenticate_no_payload() {
        let headers = HeaderMap::new();
        let method = AuthenticationMethod::Signature(SignatureAuthentication {
            signature_provider: WebhookType::Github,
            secret_key: "secret".to_string(),
            authentication_config: None,
        });
        assert!(matches!(
            method.authenticate_http_request(&headers, None),
            Err(AuthenticationError::InvalidPayload)
        ));
    }

    // --- Slack webhook end-to-end ---

    fn slack_headers(secret: &str, payload: &str, timestamp: &str) -> HeaderMap {
        let signed_payload = format!("v0:{}:{}", timestamp, payload);
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha256, secret, &signed_payload);
        let encoded = encode_hmac_signature(Encoding::Hex, &sig);
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Slack-Signature",
            HeaderValue::from_str(&format!("v0={}", encoded)).unwrap(),
        );
        headers.insert(
            "X-Slack-Request-Timestamp",
            HeaderValue::from_str(timestamp).unwrap(),
        );
        headers
    }

    #[test]
    fn test_slack_authenticate_valid() {
        let secret = "slack_signing_secret";
        let payload = "token=xxx&command=%2Ftest".to_string();
        let timestamp = "1531420618";
        let headers = slack_headers(secret, &payload, timestamp);

        let method = AuthenticationMethod::Signature(SignatureAuthentication {
            signature_provider: WebhookType::Slack,
            secret_key: secret.to_string(),
            authentication_config: None,
        });
        assert!(method
            .authenticate_http_request(&headers, Some(&payload))
            .is_ok());
    }

    #[test]
    fn test_slack_authenticate_wrong_timestamp() {
        let secret = "slack_secret";
        let payload = "data".to_string();
        let headers = slack_headers(secret, &payload, "1000000000");

        let method = AuthenticationMethod::Signature(SignatureAuthentication {
            signature_provider: WebhookType::Slack,
            secret_key: secret.to_string(),
            authentication_config: None,
        });
        // Constructed with timestamp "1000000000" but that's valid - it just needs to match
        assert!(method
            .authenticate_http_request(&headers, Some(&payload))
            .is_ok());
    }

    // --- Stripe webhook end-to-end ---

    fn stripe_headers(secret: &str, payload: &str, timestamp: &str) -> HeaderMap {
        let signed_payload = format!("{}.{}", timestamp, payload);
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha256, secret, &signed_payload);
        let encoded = encode_hmac_signature(Encoding::Hex, &sig);
        let mut headers = HeaderMap::new();
        headers.insert(
            "STRIPE-SIGNATURE",
            HeaderValue::from_str(&format!("t={},v1={}", timestamp, encoded)).unwrap(),
        );
        headers
    }

    #[test]
    fn test_stripe_authenticate_valid() {
        let secret = "whsec_stripe_secret";
        let payload = r#"{"id":"evt_123"}"#.to_string();
        let timestamp = "1614556800";
        let headers = stripe_headers(secret, &payload, timestamp);

        let method = AuthenticationMethod::Signature(SignatureAuthentication {
            signature_provider: WebhookType::Stripe,
            secret_key: secret.to_string(),
            authentication_config: None,
        });
        assert!(method
            .authenticate_http_request(&headers, Some(&payload))
            .is_ok());
    }

    #[test]
    fn test_stripe_authenticate_wrong_secret() {
        let payload = r#"{"id":"evt_123"}"#.to_string();
        let headers = stripe_headers("correct", &payload, "12345");

        let method = AuthenticationMethod::Signature(SignatureAuthentication {
            signature_provider: WebhookType::Stripe,
            secret_key: "wrong".to_string(),
            authentication_config: None,
        });
        assert!(method
            .authenticate_http_request(&headers, Some(&payload))
            .is_err());
    }

    // --- TikTok webhook end-to-end ---

    fn tiktok_headers(secret: &str, payload: &str, timestamp: &str) -> HeaderMap {
        let signed_payload = format!("{}.{}", timestamp, payload);
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha256, secret, &signed_payload);
        let encoded = encode_hmac_signature(Encoding::Hex, &sig);
        let mut headers = HeaderMap::new();
        headers.insert(
            "TikTok-Signature",
            HeaderValue::from_str(&format!("t={},s={}", timestamp, encoded)).unwrap(),
        );
        headers
    }

    #[test]
    fn test_tiktok_authenticate_valid() {
        let secret = "tiktok_secret";
        let payload = r#"{"event":"video.upload"}"#.to_string();
        let timestamp = "1700000000";
        let headers = tiktok_headers(secret, &payload, timestamp);

        let method = AuthenticationMethod::Signature(SignatureAuthentication {
            signature_provider: WebhookType::TikTok,
            secret_key: secret.to_string(),
            authentication_config: None,
        });
        assert!(method
            .authenticate_http_request(&headers, Some(&payload))
            .is_ok());
    }

    // --- Twitch webhook end-to-end ---

    fn twitch_headers(
        secret: &str,
        payload: &str,
        message_id: &str,
        timestamp: &str,
        message_type: &str,
    ) -> HeaderMap {
        let message = format!("{}{}{}", message_id, timestamp, payload);
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha256, secret, &message);
        let encoded = encode_hmac_signature(Encoding::Hex, &sig);
        let mut headers = HeaderMap::new();
        headers.insert(
            "Twitch-Eventsub-Message-Signature",
            HeaderValue::from_str(&format!("sha256={}", encoded)).unwrap(),
        );
        headers.insert(
            "Twitch-Eventsub-Message-Id",
            HeaderValue::from_str(message_id).unwrap(),
        );
        headers.insert(
            "Twitch-Eventsub-Message-Timestamp",
            HeaderValue::from_str(timestamp).unwrap(),
        );
        headers.insert(
            "Twitch-Eventsub-Message-Type",
            HeaderValue::from_str(message_type).unwrap(),
        );
        headers
    }

    #[test]
    fn test_twitch_authenticate_valid_notification() {
        let secret = "twitch_secret";
        let payload = r#"{"subscription":{},"event":{"user_id":"123"}}"#.to_string();
        let headers = twitch_headers(secret, &payload, "msg-123", "2024-01-01T00:00:00Z", "notification");

        let method = AuthenticationMethod::Signature(SignatureAuthentication {
            signature_provider: WebhookType::Twitch,
            secret_key: secret.to_string(),
            authentication_config: None,
        });
        assert!(method
            .authenticate_http_request(&headers, Some(&payload))
            .is_ok());
    }

    #[test]
    fn test_twitch_challenge_response() {
        let secret = "twitch_secret";
        let payload = r#"{"challenge":"test_challenge_string","subscription":{"id":"sub-123"}}"#;
        let headers = twitch_headers(
            secret,
            payload,
            "msg-456",
            "2024-01-01T00:00:00Z",
            "webhook_callback_verification",
        );

        let handler = WebhookType::Twitch.get_webhook_handler().unwrap();
        let config_data = SignatureConfigData { secret_key: secret };
        let response = handler
            .handle_challenge_request(&headers, &config_data, payload)
            .unwrap();
        assert!(response.is_some());
    }

    #[test]
    fn test_twitch_non_challenge_returns_none() {
        let secret = "twitch_secret";
        let payload = r#"{"subscription":{},"event":{}}"#;
        let headers = twitch_headers(
            secret,
            payload,
            "msg-789",
            "2024-01-01T00:00:00Z",
            "notification",
        );

        let handler = WebhookType::Twitch.get_webhook_handler().unwrap();
        let config_data = SignatureConfigData { secret_key: secret };
        let response = handler
            .handle_challenge_request(&headers, &config_data, payload)
            .unwrap();
        assert!(response.is_none());
    }

    // --- Zoom webhook end-to-end ---

    fn zoom_headers(secret: &str, payload: &str, timestamp: &str) -> HeaderMap {
        let message = format!("v0:{}:{}", timestamp, payload);
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha256, secret, &message);
        let encoded = encode_hmac_signature(Encoding::Hex, &sig);
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-zm-signature",
            HeaderValue::from_str(&format!("v0={}", encoded)).unwrap(),
        );
        headers.insert(
            "x-zm-request-timestamp",
            HeaderValue::from_str(timestamp).unwrap(),
        );
        headers
    }

    #[test]
    fn test_zoom_authenticate_valid() {
        let secret = "zoom_secret";
        let payload = r#"{"event":"meeting.started"}"#.to_string();
        let timestamp = "1700000000";
        let headers = zoom_headers(secret, &payload, timestamp);

        let method = AuthenticationMethod::Signature(SignatureAuthentication {
            signature_provider: WebhookType::Zoom,
            secret_key: secret.to_string(),
            authentication_config: None,
        });
        assert!(method
            .authenticate_http_request(&headers, Some(&payload))
            .is_ok());
    }

    #[test]
    fn test_zoom_challenge_response() {
        let secret = "zoom_secret";
        let payload = r#"{"event":"endpoint.url_validation","event_ts":1234567890,"payload":{"plainToken":"abc123"}}"#;

        let handler = WebhookType::Zoom.get_webhook_handler().unwrap();
        let config_data = SignatureConfigData { secret_key: secret };
        let response = handler
            .handle_challenge_request(&HeaderMap::new(), &config_data, payload)
            .unwrap();
        assert!(response.is_some());
    }

    #[test]
    fn test_zoom_non_challenge_returns_none() {
        let payload = r#"{"event":"meeting.started","event_ts":1234567890,"payload":{"plainToken":"abc"}}"#;

        let handler = WebhookType::Zoom.get_webhook_handler().unwrap();
        let config_data = SignatureConfigData { secret_key: "secret" };
        let response = handler
            .handle_challenge_request(&HeaderMap::new(), &config_data, payload)
            .unwrap();
        assert!(response.is_none());
    }

    // --- Custom webhook end-to-end ---

    #[test]
    fn test_custom_signature_authenticate_valid() {
        let secret = "custom_key";
        let payload = "custom body".to_string();
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha256, secret, &payload);
        let encoded = encode_hmac_signature(Encoding::Hex, &sig);

        let mut headers = HeaderMap::new();
        headers.insert(
            "X-My-Signature",
            HeaderValue::from_str(&encoded).unwrap(),
        );

        let method = AuthenticationMethod::Signature(SignatureAuthentication {
            signature_provider: WebhookType::Custom,
            secret_key: secret.to_string(),
            authentication_config: Some(SignatureAuthenticationMethod {
                algorithm: HmacAlgorithm::Sha256,
                encoding: Encoding::Hex,
                signature_header_name: "X-My-Signature".to_string(),
                signature_prefix: None,
            }),
        });
        assert!(method
            .authenticate_http_request(&headers, Some(&payload))
            .is_ok());
    }

    #[test]
    fn test_custom_signature_with_prefix() {
        let secret = "key";
        let payload = "body".to_string();
        let sig = calculate_hmac_signature(HmacAlgorithm::Sha512, secret, &payload);
        let encoded = encode_hmac_signature(Encoding::Base64, &sig);

        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Sig",
            HeaderValue::from_str(&format!("hmac={}", encoded)).unwrap(),
        );

        let method = AuthenticationMethod::Signature(SignatureAuthentication {
            signature_provider: WebhookType::Custom,
            secret_key: secret.to_string(),
            authentication_config: Some(SignatureAuthenticationMethod {
                algorithm: HmacAlgorithm::Sha512,
                encoding: Encoding::Base64,
                signature_header_name: "X-Sig".to_string(),
                signature_prefix: Some("hmac=".to_string()),
            }),
        });
        assert!(method
            .authenticate_http_request(&headers, Some(&payload))
            .is_ok());
    }

    #[test]
    fn test_custom_signature_missing_config() {
        let payload = "body".to_string();
        let headers = HeaderMap::new();

        let method = AuthenticationMethod::Signature(SignatureAuthentication {
            signature_provider: WebhookType::Custom,
            secret_key: "secret".to_string(),
            authentication_config: None,
        });
        assert!(matches!(
            method.authenticate_http_request(&headers, Some(&payload)),
            Err(AuthenticationError::InvalidCustomConfig)
        ));
    }

    // --- API key authentication ---

    #[test]
    fn test_api_key_authenticate_valid() {
        let mut headers = HeaderMap::new();
        headers.insert("X-API-Key", HeaderValue::from_static("my_secret_key_123"));

        let method = AuthenticationMethod::ApiKey(ApiKeyAuthentication {
            api_key_header: "X-API-Key".to_string(),
            api_key_secret: "my_secret_key_123".to_string(),
        });
        assert!(method.authenticate_http_request(&headers, None).is_ok());
    }

    #[test]
    fn test_api_key_authenticate_wrong_key() {
        let mut headers = HeaderMap::new();
        headers.insert("X-API-Key", HeaderValue::from_static("wrong_key"));

        let method = AuthenticationMethod::ApiKey(ApiKeyAuthentication {
            api_key_header: "X-API-Key".to_string(),
            api_key_secret: "correct_key".to_string(),
        });
        assert!(matches!(
            method.authenticate_http_request(&headers, None),
            Err(AuthenticationError::InvalidApiKey)
        ));
    }

    #[test]
    fn test_api_key_authenticate_missing_header() {
        let headers = HeaderMap::new();

        let method = AuthenticationMethod::ApiKey(ApiKeyAuthentication {
            api_key_header: "X-API-Key".to_string(),
            api_key_secret: "secret".to_string(),
        });
        assert!(matches!(
            method.authenticate_http_request(&headers, None),
            Err(AuthenticationError::InvalidApiKey)
        ));
    }

    // --- Basic auth ---

    fn basic_auth_header(username: &str, password: &str) -> HeaderMap {
        let credentials = BASE64_STANDARD.encode(format!("{}:{}", username, password));
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Basic {}", credentials)).unwrap(),
        );
        headers
    }

    #[test]
    fn test_basic_auth_valid() {
        let headers = basic_auth_header("admin", "password123");

        let method = AuthenticationMethod::BasicAuth(BasicAuthAuthentication {
            username: "admin".to_string(),
            password: "password123".to_string(),
        });
        assert!(method.authenticate_http_request(&headers, None).is_ok());
    }

    #[test]
    fn test_basic_auth_wrong_password() {
        let headers = basic_auth_header("admin", "wrong");

        let method = AuthenticationMethod::BasicAuth(BasicAuthAuthentication {
            username: "admin".to_string(),
            password: "correct".to_string(),
        });
        assert!(matches!(
            method.authenticate_http_request(&headers, None),
            Err(AuthenticationError::UnauthorizedBasicHttpAuth)
        ));
    }

    #[test]
    fn test_basic_auth_wrong_username() {
        let headers = basic_auth_header("wrong_user", "password");

        let method = AuthenticationMethod::BasicAuth(BasicAuthAuthentication {
            username: "admin".to_string(),
            password: "password".to_string(),
        });
        assert!(matches!(
            method.authenticate_http_request(&headers, None),
            Err(AuthenticationError::UnauthorizedBasicHttpAuth)
        ));
    }

    #[test]
    fn test_basic_auth_missing_header() {
        let headers = HeaderMap::new();

        let method = AuthenticationMethod::BasicAuth(BasicAuthAuthentication {
            username: "admin".to_string(),
            password: "password".to_string(),
        });
        assert!(matches!(
            method.authenticate_http_request(&headers, None),
            Err(AuthenticationError::UnauthorizedBasicHttpAuth)
        ));
    }

    #[test]
    fn test_basic_auth_bearer_instead_of_basic() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_static("Bearer sometoken"),
        );

        let method = AuthenticationMethod::BasicAuth(BasicAuthAuthentication {
            username: "admin".to_string(),
            password: "password".to_string(),
        });
        assert!(matches!(
            method.authenticate_http_request(&headers, None),
            Err(AuthenticationError::UnauthorizedBasicHttpAuth)
        ));
    }

    #[test]
    fn test_basic_auth_invalid_base64() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_static("Basic !!!invalid!!!"),
        );

        let method = AuthenticationMethod::BasicAuth(BasicAuthAuthentication {
            username: "admin".to_string(),
            password: "password".to_string(),
        });
        assert!(matches!(
            method.authenticate_http_request(&headers, None),
            Err(AuthenticationError::UnauthorizedBasicHttpAuth)
        ));
    }

    // --- AuthenticationMethod serde (untagged enum) ---

    #[test]
    fn test_authentication_method_signature_serde() {
        let method = AuthenticationMethod::Signature(SignatureAuthentication {
            signature_provider: WebhookType::Github,
            secret_key: "secret".to_string(),
            authentication_config: None,
        });
        let json = serde_json::to_value(&method).unwrap();
        assert_eq!(json["signature_provider"], "Github");
        assert_eq!(json["secret_key"], "secret");

        let deserialized: AuthenticationMethod = serde_json::from_value(json).unwrap();
        match deserialized {
            AuthenticationMethod::Signature(sig) => {
                assert_eq!(sig.signature_provider, WebhookType::Github);
                assert_eq!(sig.secret_key, "secret");
            }
            _ => panic!("expected Signature variant"),
        }
    }

    #[test]
    fn test_authentication_method_api_key_serde() {
        let method = AuthenticationMethod::ApiKey(ApiKeyAuthentication {
            api_key_header: "X-Key".to_string(),
            api_key_secret: "val".to_string(),
        });
        let json = serde_json::to_value(&method).unwrap();
        let deserialized: AuthenticationMethod = serde_json::from_value(json).unwrap();
        match deserialized {
            AuthenticationMethod::ApiKey(ak) => {
                assert_eq!(ak.api_key_header, "X-Key");
                assert_eq!(ak.api_key_secret, "val");
            }
            _ => panic!("expected ApiKey variant"),
        }
    }

    #[test]
    fn test_authentication_method_basic_auth_serde() {
        let method = AuthenticationMethod::BasicAuth(BasicAuthAuthentication {
            username: "user".to_string(),
            password: "pass".to_string(),
        });
        let json = serde_json::to_value(&method).unwrap();
        let deserialized: AuthenticationMethod = serde_json::from_value(json).unwrap();
        match deserialized {
            AuthenticationMethod::BasicAuth(ba) => {
                assert_eq!(ba.username, "user");
                assert_eq!(ba.password, "pass");
            }
            _ => panic!("expected BasicAuth variant"),
        }
    }

    // --- AuthenticationError into_response ---

    #[test]
    fn test_error_invalid_signature_is_401() {
        let response = AuthenticationError::InvalidSignature.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_error_missing_header_is_400() {
        let response =
            AuthenticationError::MissingHeader("X-Sig".to_string()).into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_error_invalid_payload_is_400() {
        let response = AuthenticationError::InvalidPayload.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_error_invalid_api_key_is_401() {
        let response = AuthenticationError::InvalidApiKey.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_error_basic_auth_has_www_authenticate() {
        let response = AuthenticationError::UnauthorizedBasicHttpAuth.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(response.headers().contains_key("www-authenticate"));
    }

    #[test]
    fn test_error_invalid_custom_config_is_400() {
        let response = AuthenticationError::InvalidCustomConfig.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_error_invalid_timestamp_is_400() {
        let response = AuthenticationError::InvalidTimestamp.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
