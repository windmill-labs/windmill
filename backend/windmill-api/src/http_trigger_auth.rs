use std::{borrow::Cow, collections::HashMap};

use axum::response::{IntoResponse, Response};
use base64::{
    prelude::{BASE64_STANDARD, BASE64_URL_SAFE},
    Engine,
};
use hmac::{Hmac, Mac};
use http::{HeaderMap, HeaderValue, StatusCode};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha1::Sha1;
use sha2::{Sha256, Sha512};

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
            _: &AuthenticationMethod,
            _: &str,
        ) -> Result<Option<Response>, WebhookError> {
            Ok(None)
        }

        fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
            &self,
            headers: &'header HeaderMap,
            raw_payload: &'payload str,
        ) -> Result<SignatureAuthenticationData<'payload, 'header, 'prefix>, WebhookError> {
            let github_secret_header = headers.try_get_webhook_header("X-Hub-Signature-256")?;

            let authentication_data = SignatureAuthenticationData::new(
                Cow::Borrowed(raw_payload),
                github_secret_header,
                Some("sha256"),
                SignatureAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Hex),
            );

            Ok(authentication_data)
        }
    }
}

mod shopify {
    use super::*;

    pub struct Shopify;

    impl WebhookHandler for Shopify {
        fn handle_challenge_request<'header>(
            &self,
            _: &'header HeaderMap,
            _: &AuthenticationMethod,
            _: &str,
        ) -> Result<Option<Response>, WebhookError> {
            Ok(None)
        }

        fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
            &self,
            headers: &'header HeaderMap,
            raw_payload: &'payload str,
        ) -> Result<SignatureAuthenticationData<'payload, 'header, 'prefix>, WebhookError> {
            let shopify_secret_header = headers.try_get_webhook_header("X-Shopify-Hmac-Sha256")?;
            Ok(SignatureAuthenticationData::new(
                Cow::Borrowed(raw_payload),
                shopify_secret_header,
                None,
                SignatureAuthenticationDetails::new(HmacAlgorithm::Sha256, Encoding::Base64),
            ))
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
            _: &AuthenticationMethod,
            _: &str,
        ) -> Result<Option<Response>, WebhookError> {
            Ok(None)
        }

        fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
            &self,
            headers: &'header HeaderMap,
            raw_payload: &'payload str,
        ) -> Result<SignatureAuthenticationData<'payload, 'header, 'prefix>, WebhookError> {
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
            _: &AuthenticationMethod,
            _: &str,
        ) -> Result<Option<Response>, WebhookError> {
            Ok(None)
        }

        fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
            &self,
            headers: &'header HeaderMap,
            raw_payload: &'payload str,
        ) -> Result<SignatureAuthenticationData<'payload, 'header, 'prefix>, WebhookError> {
            let stripe_signature_header = headers.try_get_webhook_header("STRIPE-SIGNATURE")?;

            let stripe_signature = parse_signature(stripe_signature_header, (",", "="));

            let timestamp = *stripe_signature
                .get("t")
                .ok_or(WebhookError::InvalidTimestamp)?;
            let v1 = *stripe_signature
                .get("v1")
                .ok_or(WebhookError::InvalidSignature)?;

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
            _: &AuthenticationMethod,
            _: &str,
        ) -> Result<Option<Response>, WebhookError> {
            Ok(None)
        }

        fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
            &self,
            headers: &'header HeaderMap,
            raw_payload: &'payload str,
        ) -> Result<SignatureAuthenticationData<'payload, 'header, 'prefix>, WebhookError> {
            let tiktok_secret_signature = headers.try_get_webhook_header("TikTok-Signature")?;

            let stripe_signature = parse_signature(tiktok_secret_signature, (",", "="));

            let timestamp = *stripe_signature
                .get("t")
                .ok_or(WebhookError::InvalidTimestamp)?;
            let s = *stripe_signature
                .get("s")
                .ok_or(WebhookError::InvalidSignature)?;

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
        ) -> Result<SignatureAuthenticationData<'payload, 'header, 'prefix>, WebhookError> {
            let twitch_secret_signature =
                headers.try_get_webhook_header("Twitch-Eventsub-Message-Signature")?;
            let twitch_message_id_header =
                headers.try_get_webhook_header("Twitch-Eventsub-Message-Id header")?;
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
            authentication_method: &AuthenticationMethod,
            raw_payload: &str,
        ) -> Result<Option<Response>, WebhookError> {
            match &authentication_method {
                AuthenticationMethod::Signature(hmac) => {
                    let authentication_data =
                        self.get_hmac_authentication_data(headers, raw_payload)?;
                    verify_hmac_signature(
                        authentication_data,
                        &hmac.signature_config_data.secret_key,
                    )?;
                }
                _ => {
                    tracing::error!("Twitch webhook should only handle hmac authentication");
                    return Ok(None);
                }
            }

            let twitch_eventsub_message_type =
                headers.try_get_webhook_header("Twitch-Eventsub-Message-Type")?;

            if twitch_eventsub_message_type != "webhook_callback_verification" {
                return Ok(None);
            }
            let twitch_crc_body =
                serde_json::from_str::<TwitchCrcBody>(raw_payload).map_err(|e| {
                    WebhookError::InvalidChallengeResponse(format!("Twitch :{}", e.to_string()))
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
    #[serde(rename_all = "snake_case")]
    struct ZoomPayload {
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
            authentication_method: &AuthenticationMethod,
            raw_payload: &str,
        ) -> Result<Option<Response>, WebhookError> {
            let Ok(zoom_request_body) = serde_json::from_str::<ZoomChallengeResponse>(raw_payload)
            else {
                return Ok(None);
            };

            if zoom_request_body.event != "endpoint.url_validation" {
                return Ok(None);
            }

            match &authentication_method {
                AuthenticationMethod::Signature(hmac) => {
                    let hmac_signature = calculate_hmac_signature(
                        HmacAlgorithm::Sha256,
                        &hmac.signature_config_data.secret_key,
                        &zoom_request_body.payload.plain_token,
                    );

                    let encoded_hmac_signature =
                        encode_hmac_signature(Encoding::Hex, &hmac_signature);

                    let response = (
                        StatusCode::OK,
                        Json(json!({
                            "plainToken": zoom_request_body.payload.plain_token,
                            "encryptedToken": encoded_hmac_signature
                        })),
                    );

                    Ok(Some(response.into_response()))
                }
                _ => return Ok(None),
            }
        }

        fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
            &self,
            headers: &'header HeaderMap,
            raw_payload: &'payload str,
        ) -> Result<SignatureAuthenticationData<'payload, 'header, 'prefix>, WebhookError> {
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
use shopify::Shopify;
use slack::Slack;
use stripe::Stripe;
use tiktok::TikTok;
use twitch::Twitch;
use zoom::Zoom;

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
        autentication_method: &AuthenticationMethod,
        raw_payload: &str,
    ) -> Result<Option<Response>, WebhookError>;

    fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
        &self,
        headers: &'header HeaderMap,
        raw_payload: &'payload str,
    ) -> Result<SignatureAuthenticationData<'payload, 'header, 'prefix>, WebhookError>;
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum HmacAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Encoding {
    Base64,
    Base64Uri,
    Hex,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureAuthenticationMethod {
    algorithm: HmacAlgorithm,
    encoding: Encoding,
    signature_header_name: String,
    signature_prefix: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SignatureConfigData {
    secret_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureAuthentication {
    signature_providers: WebhookType,
    #[serde(flatten)]
    signature_config_data: SignatureConfigData,
    config: Option<SignatureAuthenticationMethod>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BasicAuthAuthentication {
    pub username: String,
    pub password: String,
    pub custom_header_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyAuthentication {
    api_key_header: String,
    api_key: String,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WebhookType {
    Github,
    Shopify,
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
            WebhookType::Shopify => &Shopify,
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
    ) -> Result<&'header str, WebhookError>;
}

impl TryGetWebhookHeader for HeaderMap<HeaderValue> {
    fn try_get_webhook_header<'header>(
        &'header self,
        header_name: &str,
    ) -> Result<&'header str, WebhookError> {
        let Some(signature_header) = self.get(header_name) else {
            return Err(WebhookError::MissingHeader(header_name.to_string()));
        };
        let Some(signature_header) = signature_header.to_str().ok() else {
            return Err(WebhookError::InvalidHeader(header_name.to_string()));
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
) -> Result<(), WebhookError> {
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
        return Err(WebhookError::InvalidSignature);
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AuthenticationMethod {
    Signature(SignatureAuthentication),
    BasicAuth(BasicAuthAuthentication),
    ApiKey(ApiKeyAuthentication),
}

impl AuthenticationMethod {
    pub fn authenticate_http_request(
        &self,
        headers: &HeaderMap,
        raw_payload: &str,
    ) -> Result<Option<Response>, WebhookError> {
        match self {
            AuthenticationMethod::Signature(SignatureAuthentication {
                signature_config_data,
                config,
                signature_providers,
            }) => {
                let handler = signature_providers.get_webhook_handler();

                let challenge_response = handler
                    .map(|handler| handler.handle_challenge_request(headers, self, raw_payload))
                    .transpose()?
                    .flatten();

                if let Some(challenge_response) = challenge_response {
                    return Ok(Some(challenge_response));
                }

                let authentication_data = match handler {
                    Some(handler) => handler.get_hmac_authentication_data(headers, raw_payload)?,
                    None => {
                        let config = config.as_ref().ok_or(WebhookError::InvalidCustomConfig)?;
                        let signature_header_value =
                            headers.try_get_webhook_header(&config.signature_header_name)?;
                        SignatureAuthenticationData::new(
                            Cow::Borrowed(raw_payload),
                            signature_header_value,
                            config.signature_prefix.as_deref(),
                            SignatureAuthenticationDetails::new(config.algorithm, config.encoding),
                        )
                    }
                };

                verify_hmac_signature(authentication_data, &signature_config_data.secret_key)?;
            }
            AuthenticationMethod::ApiKey(ApiKeyAuthentication { api_key_header, api_key }) => {
                let api_key_to_cmp = headers.try_get_webhook_header(&api_key_header)?;
                if api_key_to_cmp != api_key {
                    return Err(WebhookError::InvalidApiKey);
                }
            }
            AuthenticationMethod::BasicAuth(BasicAuthAuthentication {
                username,
                password,
                custom_header_name,
            }) => {
                let mut credentials_store = headers
                    .try_get_webhook_header(
                        custom_header_name.as_deref().unwrap_or("Authorization"),
                    )?
                    .split(' ');

                let _ = credentials_store
                    .next()
                    .filter(|r#type| *r#type == "Basic")
                    .ok_or(WebhookError::InvalidAuthHeader(
                        "missing `Basic` type".to_string(),
                    ))?;

                let credentials_as_base64 = credentials_store.next().ok_or(
                    WebhookError::InvalidHeader("missing credentials".to_string()),
                )?;

                let credentials_from_base64_as_bytes = BASE64_STANDARD
                    .decode(credentials_as_base64.as_bytes())
                    .map_err(|_| {
                        WebhookError::InvalidAuthHeader(
                            "invalid encoding for username:password expected base64 encoding"
                                .to_string(),
                        )
                    })?;

                let credentials_separated_with_colon =
                    String::from_utf8_lossy(&credentials_from_base64_as_bytes);

                let credentials = credentials_separated_with_colon.split(':').collect_vec();

                if credentials.len() != 2 {
                    return Err(WebhookError::InvalidAuthHeader("basic auth format expected: `Basic username:password` (with username:password encoded in base64)".to_string()));
                }

                if credentials.get(0).unwrap() != username
                    && credentials.get(1).unwrap() != password
                {
                    return Err(WebhookError::InvalidAuthHeader(
                        "wrong credentials".to_string(),
                    ));
                }
            }
        }

        Ok(None)
    }
}

#[derive(thiserror::Error, Debug)]
#[allow(unused)]
pub enum WebhookError {
    #[error("failed to parse timestamp")]
    InvalidTimestamp,

    #[error("invalid secret")]
    InvalidSecret(#[from] base64::DecodeError),

    #[error("invalid header {0}")]
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
}

impl IntoResponse for WebhookError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            WebhookError::InvalidTimestamp
            | WebhookError::InvalidPayload
            | WebhookError::InvalidHeader(_)
            | WebhookError::MissingHeader(_)
            | WebhookError::TimestampTooOldError
            | WebhookError::FutureTimestampError
            | WebhookError::InvalidCustomConfig
            | WebhookError::InvalidChallengeResponse(_) => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }

            WebhookError::InvalidSecret(_)
            | WebhookError::InvalidSignature
            | WebhookError::InvalidAuthHeader(_) => (StatusCode::UNAUTHORIZED, self.to_string()),

            WebhookError::InvalidApiKey => (StatusCode::FORBIDDEN, self.to_string()),
        };

        let body = json!({ "error": error_message });

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        (status, headers, body.to_string()).into_response()
    }
}
