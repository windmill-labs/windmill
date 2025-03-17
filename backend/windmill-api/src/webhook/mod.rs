use std::borrow::Cow;

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

mod github;
mod shopify;
mod slack;
mod stripe;
mod tiktok;
mod twitch;
mod zoom;

use constant_time_eq::constant_time_eq;
use github::Github;
use shopify::Shopify;
use slack::Slack;
use stripe::Stripe;
use tiktok::TikTok;
use twitch::Twitch;
use zoom::Zoom;

struct HmacAuthenticationDetails {
    algorithm_to_use: HmacAlgorithm,
    header_key_encoding: Encoding,
}

impl HmacAuthenticationDetails {
    #[inline]
    fn new(algorithm_to_use: HmacAlgorithm, header_key_encoding: Encoding) -> Self {
        Self { algorithm_to_use, header_key_encoding }
    }
}

struct HmacAuthenticationData<'payload, 'header, 'prefix> {
    signed_payload: Cow<'payload, str>,
    header_key_value: &'header str,
    signature_prefix: Option<&'prefix str>,
    config: HmacAuthenticationDetails,
}

impl<'payload, 'header, 'prefix> HmacAuthenticationData<'payload, 'header, 'prefix> {
    pub fn new(
        signed_payload: Cow<'payload, str>,
        header_key_value: &'header str,
        signature_prefix: Option<&'prefix str>,
        config: HmacAuthenticationDetails,
    ) -> Self {
        Self { signed_payload, header_key_value, signature_prefix, config }
    }
}

trait WebhookHandler {
    fn handle_challenge_request<'header>(
        &self,
        headers: &'header HeaderMap,
        authentication_method: &WebhookAuthenticationMethod,
        raw_payload: &str,
    ) -> Result<Option<Response>, WebhookError>;

    fn get_hmac_authentication_data<'payload, 'header, 'prefix>(
        &self,
        headers: &'header HeaderMap,
        raw_payload: &'payload str,
    ) -> Result<HmacAuthenticationData<'payload, 'header, 'prefix>, WebhookError>;
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
pub struct HmacAuthenticationMethod {
    algorithm: HmacAlgorithm,
    encoding: Encoding,
    signature_header_key: String,
    signature_prefix: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HmacAuthentication {
    pub webhook_signing_secret: String,
    #[serde(flatten)]
    pub config: Option<HmacAuthenticationMethod>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BasicAuthAuthentication {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyAuthentication {
    api_key_header: String,
    api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WebhookAuthenticationMethod {
    HMAC(HmacAuthentication),
    BasicAuth(BasicAuthAuthentication),
    ApiKey(ApiKeyAuthentication),
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WebhookType {
    //Adyen,
    //Discord,
    //Ebay,
    //Facebook,
    Github,
    //GitLab,
    //LinkedIn,
    //Linear,
    //MailChimp,
    //Mailgun,
    //Persona,
    //Paypal,
    Shopify,
    Slack,
    Stripe,
    //Twillio,
    //Trello,
    //Treezor,
    TikTok,
    Twitch,
    //WhatsApp,
    //X,
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
    authentication_data: HmacAuthenticationData,
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

pub enum WebhookRequestType {
    Challenge(Response),
    Event,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Webhook {
    pub r#type: WebhookType,
    pub authentication_method: WebhookAuthenticationMethod,
}

impl Webhook {
    pub fn verify_signatures(
        &self,
        headers: &HeaderMap,
        raw_payload: &str,
    ) -> Result<WebhookRequestType, WebhookError> {
        let handler = self.r#type.get_webhook_handler();

        let challenge_response = handler
            .map(|handler| {
                handler.handle_challenge_request(headers, &self.authentication_method, raw_payload)
            })
            .transpose()?
            .flatten();

        if let Some(challenge_response) = challenge_response {
            return Ok(WebhookRequestType::Challenge(challenge_response));
        }

        match &self.authentication_method {
            WebhookAuthenticationMethod::HMAC(HmacAuthentication {
                webhook_signing_secret,
                config,
            }) => {
                let authentication_data = match handler {
                    Some(handler) => handler.get_hmac_authentication_data(headers, raw_payload)?,
                    None => {
                        let config = config.as_ref().ok_or(WebhookError::InvalidCustomConfig)?;
                        let signature_header_value =
                            headers.try_get_webhook_header(&config.signature_header_key)?;
                        HmacAuthenticationData::new(
                            Cow::Borrowed(raw_payload),
                            signature_header_value,
                            config.signature_prefix.as_deref(),
                            HmacAuthenticationDetails::new(config.algorithm, config.encoding),
                        )
                    }
                };

                verify_hmac_signature(authentication_data, webhook_signing_secret)?;
            }
            WebhookAuthenticationMethod::ApiKey(ApiKeyAuthentication {
                api_key_header,
                api_key,
            }) => {
                let api_key_to_cmp = headers.try_get_webhook_header(&api_key_header)?;
                if api_key_to_cmp != api_key {
                    return Err(WebhookError::InvalidApiKey);
                }
            }
            WebhookAuthenticationMethod::BasicAuth(BasicAuthAuthentication {
                username,
                password,
            }) => {
                let mut credentials_store =
                    headers.try_get_webhook_header("Authorization")?.split(' ');

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

        Ok(WebhookRequestType::Event)
    }
}

#[derive(thiserror::Error, Debug)]
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
